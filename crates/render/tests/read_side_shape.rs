//! The read side's shape map is the translation's (REQ-SYS-085; R-116, R-547, RQ-365 (c)), and REQ-SYS-015's scan, at
//! its full reach, finds no second implementation of a dd_decoder §3 formula.
//!
//! - `read_side_shape_generated`: `crates/render/frag/generated/read_side.wgsl`'s one `fn shape(` lies inside
//!   `ledger::gen::read::SHAPE_WGSL`, which is `crates/ledger/wgsl/shape.wgsl` under the translation's header (written by
//!   `cargo xtask build-kernel`, which fails when it differs from the translation of `kernel::stage::fragment::shape`, a
//!   call of `kernel::shape::shape`); `crates/ledger/src/gen/read.rs` holds no `fn shape(` in any string; and the scan
//!   finds nothing. Controls: a hand-written `mu_rho` formula planted in a `.wgsl` file, and one planted in a string in
//!   `read.rs`, are each found. The twin test against `kernel::shape`, `kernel/tests/derived.rs`'s
//!   `read_side_parity_rust_and_wgsl_agree`, runs the same `shape()` on the GPU.
//!
//! **The scan's reach** (applied per R-369, TASK-M2-25): every `.wgsl` file under `crates/`, `xtask/` and `fixtures/`
//! (build directories skipped), whole, for [`WGSL_TOKENS`]; and every string literal of every Rust source under
//! `crates/*/src/` and `xtask/src/` but the shared source, `crates/kernel/src/`, for [`FORMULA_TOKENS`], since a WGSL
//! formula a Rust file emits is a string there. The Rust code itself is not scanned: the f64 paths outside the kernel
//! that name these quantities (`engine::lookup`'s error bounds, the chart authoring's previews) are CPU checks, not a
//! fragment's second decode. Test sources are not scanned: their strings are expectations and controls. The translation's
//! outputs carry no token: naga names its values, never the source's.

use std::path::{Path, PathBuf};

use validation::negative_control;

/// The mass, configuration and momentum formulae's names (dd_decoder §3.1–§3.3), as a hand transcription names them.
const FORMULA_TOKENS: [&str; 3] = ["mu_rho", "mu_lambda", "m01"];

/// [`FORMULA_TOKENS`] and the link formulae's names, for `.wgsl` files, where the names are never a CPU path's.
const WGSL_TOKENS: [&str; 9] = [
    "mu_rho",
    "mu_lambda",
    "m01",
    "softmax",
    "logit",
    "jacobi",
    "mu_max",
    "eps_mu",
    "q_max",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Whether `text` holds `token` as a word, case-insensitively: not inside a longer identifier.
fn has_word(text: &str, token: &str) -> bool {
    let lower = text.to_lowercase();
    let ident = |c: char| c.is_alphanumeric() || c == '_';
    lower.match_indices(token).any(|(i, _)| {
        let before = lower[..i].chars().next_back();
        let after = lower[i + token.len()..].chars().next();
        !before.is_some_and(ident) && !after.is_some_and(ident)
    })
}

/// The tokens of `tokens` that `text` holds.
fn hits(text: &str, tokens: &[&'static str]) -> Vec<&'static str> {
    tokens
        .iter()
        .copied()
        .filter(|t| has_word(text, t))
        .collect()
}

/// The contents of every string literal of the Rust source `src`: `"…"` with its escapes, raw strings `r#"…"#` and
/// their byte forms; comments and character literals skipped.
fn strings(src: &str) -> Vec<String> {
    let b = src.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i..].starts_with(b"//") {
            i = src[i..].find('\n').map_or(b.len(), |j| i + j);
            continue;
        }
        if b[i..].starts_with(b"/*") {
            i = src[i + 2..].find("*/").map_or(b.len(), |j| i + 2 + j + 2);
            continue;
        }
        let word_before = i > 0 && (b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_');
        let raw_at = if b[i..].starts_with(b"br") {
            Some(i + 2)
        } else if b[i] == b'r' {
            Some(i + 1)
        } else {
            None
        };
        if let (Some(mut j), false) = (raw_at, word_before) {
            let hashes = b[j..].iter().take_while(|&&c| c == b'#').count();
            j += hashes;
            if b.get(j) == Some(&b'"') {
                let close = format!("\"{}", "#".repeat(hashes));
                let start = j + 1;
                let end = src[start..].find(&close).map_or(b.len(), |k| start + k);
                out.push(src[start..end].to_owned());
                i = (end + close.len()).min(b.len());
                continue;
            }
        }
        if b[i] == b'"' {
            let mut j = i + 1;
            let mut s = Vec::new();
            while j < b.len() && b[j] != b'"' {
                if b[j] == b'\\' {
                    // `\"` and `\\` stand for their character; any other escape for a space, a word's edge.
                    let c = b.get(j + 1).copied().unwrap_or(b' ');
                    s.push(if c == b'"' || c == b'\\' { c } else { b' ' });
                    j += 2;
                    continue;
                }
                s.push(b[j]);
                j += 1;
            }
            out.push(String::from_utf8_lossy(&s).into_owned());
            i = j + 1;
            continue;
        }
        if b[i] == b'\'' {
            // A character literal, `'"'` or `'\''` among them; a lifetime's `'` is passed by.
            if b.get(i + 1) == Some(&b'\\') {
                if let Some(k) = src[i + 2..].find('\'') {
                    i += 2 + k + 1;
                    continue;
                }
            } else if let Some(c) = src[i + 1..].chars().next() {
                if src[i + 1 + c.len_utf8()..].starts_with('\'') {
                    i += 1 + c.len_utf8() + 1;
                    continue;
                }
            }
        }
        i += 1;
    }
    out
}

/// Every file under `dir` with extension `ext`, build directories and `skip` (a directory's name) passed by.
fn files(dir: &Path, ext: &str, skip: &[&str], out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name == "target" || name == ".git" || skip.contains(&name.as_ref()) {
            continue;
        }
        if path.is_dir() {
            files(&path, ext, skip, out);
        } else if path.extension().is_some_and(|e| e == ext) {
            out.push(path);
        }
    }
}

/// The scan's sources: every `.wgsl` file, whole, and every Rust source's strings, as `(path, text, is_wgsl)`.
fn sources() -> Vec<(String, String, bool)> {
    let root = root();
    let mut wgsl = Vec::new();
    for top in ["crates", "xtask", "fixtures"] {
        files(&root.join(top), "wgsl", &[], &mut wgsl);
    }
    let mut rust = Vec::new();
    for top in ["crates", "xtask"] {
        files(
            &root.join(top),
            "rs",
            &["tests", "benches", "examples"],
            &mut rust,
        );
    }
    let shared = root.join("crates/kernel/src");
    rust.retain(|p| !p.starts_with(&shared) && p.components().any(|c| c.as_os_str() == "src"));
    let read =
        |p: &PathBuf| std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
    let mut out: Vec<(String, String, bool)> = wgsl
        .iter()
        .map(|p| (p.display().to_string(), read(p), true))
        .collect();
    out.extend(
        rust.iter()
            .map(|p| (p.display().to_string(), read(p), false)),
    );
    out
}

/// The scan over `sources`: each hit, named.
fn scan(sources: &[(String, String, bool)]) -> Vec<String> {
    let mut found = Vec::new();
    for (path, text, is_wgsl) in sources {
        if *is_wgsl {
            let h = hits(text, &WGSL_TOKENS);
            if !h.is_empty() {
                found.push(format!("{path}: {h:?}"));
            }
        } else {
            for s in strings(text) {
                let h = hits(&s, &FORMULA_TOKENS);
                if !h.is_empty() {
                    found.push(format!("{path}: a string holds {h:?}"));
                }
            }
        }
    }
    found
}

/// A hand transcription of the configuration block's reduced masses, as WGSL.
const PLANTED: &str = "fn hand_shape(m: vec3<f32>) -> f32 {\n    let m01 = m.x + m.y;\n    let mu_rho = m.x * m.y / m01;\n    return mu_rho;\n}\n";

/// The checks; `plant` adds the controls' hand-written formula to the sources: `"wgsl"` as a `.wgsl` file, `"read.rs"`
/// as a string in `crates/ledger/src/gen/read.rs`.
fn check(plant: Option<&str>) {
    let root = root();
    let read_side =
        std::fs::read_to_string(root.join("crates/render/frag/generated/read_side.wgsl")).unwrap();
    let shape = ledger::gen::read::SHAPE_WGSL;
    let file = std::fs::read_to_string(root.join("crates/ledger/wgsl/shape.wgsl")).unwrap();
    assert_eq!(
        shape, file,
        "SHAPE_WGSL is not crates/ledger/wgsl/shape.wgsl"
    );
    assert!(
        shape.starts_with("// GENERATED by `cargo xtask build-kernel`"),
        "shape.wgsl does not carry the translation's header"
    );
    let at = read_side
        .find(shape)
        .expect("read_side.wgsl does not hold the translation's shape.wgsl");
    let defs: Vec<usize> = read_side
        .match_indices("fn shape(")
        .map(|(i, _)| i)
        .collect();
    assert_eq!(
        defs.len(),
        1,
        "read_side.wgsl defines shape() {} times",
        defs.len()
    );
    assert!(
        (at..at + shape.len()).contains(&defs[0]),
        "read_side.wgsl's shape() is not the translation's"
    );
    let read_rs_path = root.join("crates/ledger/src/gen/read.rs");
    let mut read_rs = std::fs::read_to_string(&read_rs_path).unwrap();
    let mut sources = sources();
    assert!(
        sources.iter().filter(|s| s.2).count() > 50
            && sources.iter().filter(|s| !s.2).count() > 100,
        "the scan reads too few files to be reading the tree"
    );
    match plant {
        Some("wgsl") => sources.push((
            "planted/hand_shape.wgsl".to_owned(),
            PLANTED.to_owned(),
            true,
        )),
        Some("read.rs") => {
            read_rs.push_str(&format!("\nconst PLANTED: &str = {PLANTED:?};\n"));
            let p = read_rs_path.display().to_string();
            let entry = sources
                .iter_mut()
                .find(|s| s.0 == p)
                .expect("the scan does not read read.rs");
            entry.1 = read_rs.clone();
        }
        _ => {}
    }
    assert!(
        !strings(&read_rs).iter().any(|s| s.contains("fn shape(")),
        "crates/ledger/src/gen/read.rs holds a shape() in a string"
    );
    let found = scan(&sources);
    assert!(
        found.is_empty(),
        "the scan finds a second implementation of a §3 formula: {found:?}"
    );
}

#[test]
fn read_side_shape_generated() {
    check(None);
}

negative_control!(
    read_side_shape_generated,
    "a hand-written mu_rho formula planted in a .wgsl file must be found by the scan",
    expected = "planted/hand_shape.wgsl",
    check(Some("wgsl"))
);

mod in_read_rs {
    use super::*;

    negative_control!(
        read_side_shape_generated,
        "a hand-written mu_rho formula planted in a string in read.rs must be found by the scan",
        expected = "read.rs: a string holds",
        check(Some("read.rs"))
    );
}

/// The lexer's edges: escapes, raw strings, comments and character literals; `extra` is appended to the source.
fn check_lexer(extra: &str) {
    let src = format!(
        "{}{extra}",
        r####"let a = "x\"mu_rho\nm01"; // "m01" in a comment
/* "mu_lambda" */ let b = r#"fn shape("#; let c = '"'; let d = 'a'; let e = "after";
fn f<'a>(x: &'a str) {} let g = br"raw"; let h = '\''; let i = "end";"####
    );
    assert_eq!(
        strings(&src),
        ["x\"mu_rho m01", "fn shape(", "after", "raw", "end"],
        "the lexer's strings differ"
    );
    assert_eq!(hits(&strings(&src)[0], &FORMULA_TOKENS), ["mu_rho", "m01"]);
    assert!(has_word("let mu_rho = 1", "mu_rho"));
    assert!(!has_word("let mu_rhox = 1", "mu_rho"));
    assert!(!has_word("xmu_rho", "mu_rho"));
    assert!(has_word("MU_RHO", "mu_rho"));
}

#[test]
fn read_side_shape_strings_lexed() {
    check_lexer("");
}

negative_control!(
    read_side_shape_strings_lexed,
    "a string the lexer misses must show",
    expected = "the lexer's strings differ",
    check_lexer(" let j = \"mu_rho\";")
);
