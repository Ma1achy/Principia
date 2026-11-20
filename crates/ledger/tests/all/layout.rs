//! ledger's tests are one binary, `tests/all/main.rs`, with a module per file (TASK-M0-80, the owner's item 3 of
//! 17 Nov 2026). A source under `tests/` that no `mod` declaration reaches from `main.rs` is compiled into no binary,
//! so its tests would stop running without a word; this test fails naming each such file.
//!
//! The declarations are read from the sources' tokens, comments and string literals stepped over, so `// mod x;` or a
//! `"mod x;"` in a fixture reaches nothing. A `mod x;` resolves as rustc resolves it: beside `main.rs`, a `mod.rs` or a
//! file loaded through `#[path]`, in the folder named for any other file, and a `#[path = "…"]` relative to the
//! declaring file's folder. A `mod x;` inside an inline `mod y { … }` is not modelled (ledger's tests have none): it
//! would be read as declared where `y` itself is declared, not in `y/`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// A token of a source, as far as module declarations need: an identifier or keyword, a string literal's text, or one
/// punctuation character.
#[derive(Debug, PartialEq)]
enum Token {
    Ident(String),
    Str(String),
    Punct(char),
}

/// `text`'s tokens, comments (nested block comments too), character literals and lifetimes left out.
fn tokens(text: &str) -> Vec<Token> {
    let s: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < s.len() {
        let c = s[i];
        let next = s.get(i + 1).copied();
        if c.is_whitespace() {
            i += 1;
        } else if c == '/' && next == Some('/') {
            while i < s.len() && s[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && next == Some('*') {
            let mut depth = 0;
            while i < s.len() {
                if s[i] == '/' && s.get(i + 1) == Some(&'*') {
                    depth += 1;
                    i += 2;
                } else if s[i] == '*' && s.get(i + 1) == Some(&'/') {
                    depth -= 1;
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    i += 1;
                }
            }
        } else if let Some((body, end)) = string_at(&s, i) {
            out.push(Token::Str(body));
            i = end;
        } else if c == '\'' {
            // A character literal ('x', '\n', '\''), or a lifetime ('a), which has no closing quote.
            if next == Some('\\') {
                i += 3;
                while i < s.len() && s[i] != '\'' {
                    i += 1;
                }
                i += 1;
            } else if s.get(i + 2) == Some(&'\'') {
                i += 3;
            } else {
                i += 1;
            }
        } else if c.is_alphanumeric() || c == '_' {
            let start = i;
            while i < s.len() && (s[i].is_alphanumeric() || s[i] == '_') {
                i += 1;
            }
            out.push(Token::Ident(s[start..i].iter().collect()));
        } else {
            out.push(Token::Punct(c));
            i += 1;
        }
    }
    out
}

/// The string literal starting at `i`, if one does (`"…"`, `b"…"`, `r#"…"#`, `br"…"`), as its text and the index past
/// its end. An escape is kept as written: no path a test declares holds one.
fn string_at(s: &[char], i: usize) -> Option<(String, usize)> {
    let mut j = i;
    if s.get(j) == Some(&'b') {
        j += 1;
    }
    if s.get(j) == Some(&'r') {
        j += 1;
        let mut hashes = 0;
        while s.get(j) == Some(&'#') {
            hashes += 1;
            j += 1;
        }
        if s.get(j) != Some(&'"') {
            return None;
        }
        let start = j + 1;
        let mut k = start;
        while k < s.len() {
            if s[k] == '"' && (1..=hashes).all(|h| s.get(k + h) == Some(&'#')) {
                return Some((s[start..k].iter().collect(), k + 1 + hashes));
            }
            k += 1;
        }
        return Some((s[start..].iter().collect(), s.len()));
    }
    if s.get(j) != Some(&'"') {
        return None;
    }
    let start = j + 1;
    let mut k = start;
    while k < s.len() && s[k] != '"' {
        k += if s[k] == '\\' { 2 } else { 1 };
    }
    Some((
        s[start..k.min(s.len())].iter().collect(),
        (k + 1).min(s.len()),
    ))
}

/// The files `file`'s out-of-line `mod` declarations name, resolved as rustc resolves them, each with whether it owns
/// its folder. `owns` is whether `file` does: the crate root, a `mod.rs` and a file loaded through `#[path]` declare
/// their modules beside themselves; any other `x.rs` declares them in `x/`.
fn declared(file: &Path, owns: bool) -> Vec<(PathBuf, bool)> {
    let text = std::fs::read_to_string(file)
        .unwrap_or_else(|e| panic!("layout: read {}: {e}", file.display()));
    let t = tokens(&text);
    let dir = file.parent().expect("a file has a folder");
    let own = if owns {
        dir.to_path_buf()
    } else {
        dir.join(file.file_stem().expect("a file stem"))
    };
    let mut out = Vec::new();
    for (k, tok) in t.iter().enumerate() {
        let (Token::Ident(kw), Some(Token::Ident(name)), Some(Token::Punct(';'))) =
            (tok, t.get(k + 1), t.get(k + 2))
        else {
            continue;
        };
        if kw != "mod" {
            continue;
        }
        out.push(match path_attribute(&t[..k]) {
            Some(p) => (dir.join(p), true),
            None => {
                let flat = own.join(format!("{name}.rs"));
                if flat.is_file() {
                    (flat, false)
                } else {
                    (own.join(name).join("mod.rs"), true)
                }
            }
        });
    }
    out
}

/// The `#[path = "…"]` among the attributes that end `before`, the tokens ahead of a `mod` (a `pub` or `pub(…)` in
/// between is stepped over).
fn path_attribute(before: &[Token]) -> Option<String> {
    let mut end = before.len();
    // Step back over a visibility: `pub` or `pub ( … )`.
    if end > 0 && before[end - 1] == Token::Punct(')') {
        while end > 0 && before[end - 1] != Token::Punct('(') {
            end -= 1;
        }
        end = end.saturating_sub(1);
    }
    if end > 0 && before[end - 1] == Token::Ident("pub".into()) {
        end -= 1;
    }
    // Each attribute `# [ … ]` ending at `end`, latest first.
    while end > 0 && before[end - 1] == Token::Punct(']') {
        let close = end - 1;
        let mut open = close;
        while open > 0 && before[open] != Token::Punct('[') {
            open -= 1;
        }
        if open == 0 || before[open - 1] != Token::Punct('#') {
            return None;
        }
        if let [Token::Ident(p), Token::Punct('='), Token::Str(path)] = &before[open + 1..close] {
            if p == "path" {
                return Some(path.clone());
            }
        }
        end = open - 1;
    }
    None
}

/// Every `.rs` file under `dir`, at any depth.
fn sources(dir: &Path, out: &mut BTreeSet<PathBuf>) {
    let entries =
        std::fs::read_dir(dir).unwrap_or_else(|e| panic!("layout: read {}: {e}", dir.display()));
    for entry in entries {
        let path = entry.expect("a folder entry").path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.insert(path);
        }
    }
}

/// Every file reached from `root` through out-of-line `mod` declarations, `root` included.
fn reached(root: &Path) -> BTreeSet<PathBuf> {
    let mut seen = BTreeSet::new();
    let mut todo = vec![(root.to_path_buf(), true)];
    while let Some((file, owns)) = todo.pop() {
        assert!(
            file.is_file(),
            "layout: {} is declared as a module but is no file",
            file.display()
        );
        if seen.insert(file.clone()) {
            todo.extend(declared(&file, owns));
        }
    }
    seen
}

/// Fails naming every `.rs` file under `tests` that `tests/all/main.rs` does not reach as a module.
fn check(tests: &Path) {
    let reached: BTreeSet<PathBuf> = reached(&tests.join("all").join("main.rs"))
        .into_iter()
        .map(|p| normal(&p))
        .collect();
    let mut on_disk = BTreeSet::new();
    sources(tests, &mut on_disk);
    let stray: Vec<String> = on_disk
        .iter()
        .filter(|p| !reached.contains(&normal(p)))
        .map(|p| p.strip_prefix(tests).unwrap_or(p).display().to_string())
        .collect();
    assert!(
        stray.is_empty(),
        "layout: not a module of tests/all/main.rs, so no test in it runs; declare it there: {}",
        stray.join(", ")
    );
}

/// `p` with its `.` and `..` components folded, so two spellings of one file compare equal.
fn normal(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            c => out.push(c),
        }
    }
    out
}

/// ledger's `tests/`, found at run time.
fn tests_dir() -> PathBuf {
    Path::new(crate::manifest_dir()).join("tests")
}

#[test]
fn every_test_source_is_a_module_of_all() {
    check(&tests_dir());
}

/// A scratch copy of ledger's `tests/` sources, removed when dropped, a panic's unwinding included; only the negative
/// control uses it.
#[cfg(feature = "controls")]
struct Scratch(PathBuf);

#[cfg(feature = "controls")]
impl Scratch {
    /// A copy of every `.rs` file under `from`, at the same relative paths.
    fn copy_of(from: &Path) -> Scratch {
        let to = std::env::temp_dir().join(format!(
            "ledger-layout-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("after the epoch")
                .as_nanos()
        ));
        let mut files = BTreeSet::new();
        sources(from, &mut files);
        for f in files {
            let dest = to.join(f.strip_prefix(from).expect("under tests"));
            std::fs::create_dir_all(dest.parent().expect("a folder"))
                .expect("create the scratch folder");
            std::fs::copy(&f, &dest).expect("copy a source");
        }
        Scratch(to)
    }
}

#[cfg(feature = "controls")]
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

validation::negative_control!(
    every_test_source_is_a_module_of_all,
    "a test file that no module declaration reaches must fail the check",
    expected =
        "not a module of tests/all/main.rs, so no test in it runs; declare it there: stray.rs",
    {
        let scratch = Scratch::copy_of(&tests_dir());
        std::fs::write(scratch.0.join("stray.rs"), "#[test]\nfn never_runs() {}\n")
            .expect("write the stray");
        check(&scratch.0);
    }
);
