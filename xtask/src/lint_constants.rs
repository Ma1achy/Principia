//! `cargo xtask lint constants` — no numeric constant in the physics and engine crates except those read from the
//! constants register (dd_generation_root §3.8; REQ-SYS-001, REQ-SYS-005). It reads each `const` and `static` item in
//! `crates/{kernel,ledger,engine}`'s `src/` and `build.rs`, and fails on one whose initializer holds a numeric literal,
//! naming the file and line. The register itself is where the numbers are declared, and the generated files are
//! excluded: their numbers are emitted from the ledger.

use std::fs;
use std::path::{Path, PathBuf};

/// The crates the lint reads (the physics and engine crates).
pub const CRATES: [&str; 3] = ["kernel", "ledger", "engine"];

/// The register, relative to the workspace root.
pub const REGISTER: &str = "crates/ledger/src/constants.rs";

/// A numeric constant not read from the register: its file, relative to the root, its line and its name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub path: PathBuf,
    pub line: usize,
    pub name: String,
}

impl std::fmt::Display for Finding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}: numeric constant `{}` is not read from the constants register ({REGISTER})",
            self.path.display(),
            self.line,
            self.name
        )
    }
}

/// Every finding under `root`, skipping the files in `excluded` (relative to `root`).
pub fn check(root: &Path, excluded: &[PathBuf]) -> Result<Vec<Finding>, String> {
    let mut files = Vec::new();
    for krate in CRATES {
        let dir = Path::new("crates").join(krate);
        collect(root, &dir.join("src"), &mut files)?;
        if root.join(dir.join("build.rs")).is_file() {
            files.push(dir.join("build.rs"));
        }
    }
    files.sort();
    let mut found = Vec::new();
    for path in files.into_iter().filter(|p| !excluded.contains(p)) {
        let source =
            fs::read_to_string(root.join(&path)).map_err(|e| format!("{}: {e}", path.display()))?;
        found.extend(scan(&source).into_iter().map(|(line, name)| Finding {
            path: path.clone(),
            line,
            name,
        }));
    }
    Ok(found)
}

/// The `.rs` files under `root/dir`, relative to `root`, into `files`; a missing directory holds none.
fn collect(root: &Path, dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let Ok(entries) = fs::read_dir(root.join(dir)) else {
        return Ok(());
    };
    for entry in entries {
        let entry = entry.map_err(|e| format!("{}: {e}", dir.display()))?;
        let path = dir.join(entry.file_name());
        if entry.path().is_dir() {
            collect(root, &path, files)?;
        } else if path.extension().is_some_and(|x| x == "rs") {
            files.push(path);
        }
    }
    Ok(())
}

/// The line and name of each `const` or `static` item in `source` whose initializer holds a numeric literal.
pub fn scan(source: &str) -> Vec<(usize, String)> {
    let code = strip(source);
    let b = code.as_bytes();
    let ident = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    let mut found = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if !ident(b[i]) || (i > 0 && (ident(b[i - 1]) || b[i - 1] == b'\'' || b[i - 1] == b'*')) {
            i += 1;
            continue;
        }
        let start = i;
        while i < b.len() && ident(b[i]) {
            i += 1;
        }
        let word = &code[start..i];
        if word != "const" && word != "static" {
            continue;
        }
        let mut words = code[i..].split_whitespace();
        let mut name = words.next().unwrap_or("");
        if name == "mut" {
            name = words.next().unwrap_or("");
        }
        let name: String = name
            .bytes()
            .take_while(|&c| ident(c))
            .map(char::from)
            .collect();
        if name.is_empty() || ["fn", "unsafe", "extern", "async"].contains(&name.as_str()) {
            continue;
        }
        if let Some(init) = initializer(&code[i..]) {
            if has_number(init) {
                found.push((code[..start].matches('\n').count() + 1, name));
            }
        }
    }
    found
}

/// The initializer of the item whose text after its keyword is `rest`: the text between its `=` and its `;`, or
/// `None` if it has none (a const generic parameter, a `const` block).
fn initializer(rest: &str) -> Option<&str> {
    let b = rest.as_bytes();
    let (mut depth, mut eq) = (0i32, None);
    for (i, &c) in b.iter().enumerate() {
        match c {
            b'(' | b'[' | b'{' => depth += 1,
            b'<' if eq.is_none() => depth += 1,
            b'>' if eq.is_some() || b[..i].ends_with(b"-") => {}
            b')' | b']' | b'}' | b'>' => depth -= 1,
            b'=' if depth == 0 && eq.is_none() => eq = Some(i + 1),
            b';' if depth == 0 => return eq.map(|e| &rest[e..i]),
            b',' if depth == 0 && eq.is_none() => return None,
            _ => {}
        }
        if depth < 0 || (c == b'{' && eq.is_none()) {
            return None;
        }
    }
    None
}

/// Whether `code` holds a numeric literal: a digit that starts a token, not a tuple index.
fn has_number(code: &str) -> bool {
    let b = code.as_bytes();
    (0..b.len()).any(|i| {
        b[i].is_ascii_digit()
            && (i == 0
                || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_' || b[i - 1] == b'.'))
    })
}

/// `source` with its comments, string literals and character literals blanked, lines kept.
pub(crate) fn strip(source: &str) -> String {
    let c: Vec<char> = source.chars().collect();
    let mut out = String::with_capacity(source.len());
    let blank = |out: &mut String, s: &[char]| {
        s.iter()
            .for_each(|&x| out.push(if x == '\n' { '\n' } else { ' ' }))
    };
    let mut i = 0;
    while i < c.len() {
        let end = if c[i] == '/' && c.get(i + 1) == Some(&'/') {
            (i..c.len()).find(|&j| c[j] == '\n').unwrap_or(c.len())
        } else if c[i] == '/' && c.get(i + 1) == Some(&'*') {
            let mut depth = 0;
            let mut j = i;
            loop {
                if j + 1 >= c.len() {
                    break c.len();
                }
                match (c[j], c[j + 1]) {
                    ('/', '*') => (depth, j) = (depth + 1, j + 2),
                    ('*', '/') if depth == 1 => break j + 2,
                    ('*', '/') => (depth, j) = (depth - 1, j + 2),
                    _ => j += 1,
                }
            }
        } else if let Some(hashes) = raw_string(&c, i) {
            let close: Vec<char> = std::iter::once('"')
                .chain(std::iter::repeat_n('#', hashes))
                .collect();
            let open = i + 1 + hashes + 1;
            (open..c.len())
                .find(|&j| c[j..].starts_with(&close))
                .map_or(c.len(), |j| j + close.len())
        } else if c[i] == '"' {
            let mut j = i + 1;
            while j < c.len() && c[j] != '"' {
                j += if c[j] == '\\' { 2 } else { 1 };
            }
            (j + 1).min(c.len())
        } else if c[i] == '\'' && c.get(i + 1) == Some(&'\\') {
            (i + 2..c.len())
                .find(|&j| c[j] == '\'')
                .map_or(c.len(), |j| j + 1)
        } else if c[i] == '\'' && c.get(i + 2) == Some(&'\'') {
            i + 3
        } else {
            out.push(c[i]);
            i += 1;
            continue;
        };
        blank(&mut out, &c[i..end]);
        i = end;
    }
    out
}

/// The number of `#`s of the raw string literal starting at `c[i]`, if one does (`r"…"`, `r#"…"#`, never `r#ident`).
fn raw_string(c: &[char], i: usize) -> Option<usize> {
    if c[i] != 'r' || (i > 0 && (c[i - 1].is_alphanumeric() || c[i - 1] == '_')) {
        return None;
    }
    let hashes = c[i + 1..].iter().take_while(|&&x| x == '#').count();
    (c.get(i + 1 + hashes) == Some(&'"')).then_some(hashes)
}

/// The files the lint skips, relative to the workspace root: the register, and the files the generator emits, whose
/// numbers are emitted from the ledger (dd_generation_root §3.8).
pub fn exempt() -> Result<Vec<PathBuf>, String> {
    let mut excluded = ledger::gen::generate(&ledger::layout(), ledger::gen::EMITTERS)
        .map_err(|e| format!("lint constants: {e}"))?
        .into_iter()
        .map(|g| g.path)
        .collect::<Vec<_>>();
    excluded.push(PathBuf::from(REGISTER));
    Ok(excluded)
}

/// Runs the lint over the workspace whose `Cargo.toml` is `manifest`, excluding the files [`exempt`] names.
pub fn run(manifest: &Path) -> Result<(), String> {
    let root = manifest
        .parent()
        .ok_or_else(|| format!("{}: no parent directory", manifest.display()))?;
    let found = check(root, &exempt()?)?;
    for finding in &found {
        eprintln!("xtask lint constants: {finding}");
    }
    if found.is_empty() {
        println!(
            "xtask lint constants: no numeric constant outside the register in crates/{{{}}}",
            CRATES.join(",")
        );
        Ok(())
    } else {
        Err(format!(
            "{} numeric constant(s) not read from the constants register (REQ-SYS-001, REQ-SYS-005)",
            found.len()
        ))
    }
}
