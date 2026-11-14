//! The pin's enforcement (R-991, REQ-CHART-212): every f64 transcendental the CPU chart evaluator reaches goes through
//! the pinned libm, by [`kernel::Real`]'s methods, so no `Float::` or inherent transcendental may stand in the
//! evaluator's source outside `src/real.rs`'s `Real` items (the trait and its two impls), `sqrt` excepted (IEEE 754
//! rounds it correctly on every platform). A source scan in the `discipline_lint` style, not a clippy
//! `disallowed-methods` entry, which would be lint configuration (R-197).
//!
//! - `transcendental_pin`: the evaluator's source holds no stray transcendental; its control plants a `Float::sin`.
//! - `transcendental_pin_scan`: the scan's own cases: what it flags (a `Float::`, `f64::`, `R::` or `libm::` path, a
//!   method call, a function named as a value) and what it passes (comments, strings, char literals, `sqrt`, the
//!   `Real` methods, a longer name, a field); its control is a scan that reads comments as code.
//!
//! **The evaluator's source** is `crates/kernel/src`, every file, but for the integrator's layers, which the chart
//! evaluator does not reach (R-926: the CPU evaluates charts, the integrator only integrates records): `driver/`,
//! `occupant/`, `physics/`, `regularisation/`, `detect/` and `payload/` (applied per R-369: the scan's scope). A file
//! added anywhere else is scanned. With it, the CPU evaluator's entry, `crates/engine/src/decode_dispatch.rs`
//! (`decode_only_with`), labelled `engine/decode_dispatch.rs`; the rest of the engine calls the evaluator and is not
//! reached by it.
//!
//! **Flagged**: a path ending in a transcendental's name (`Float::sin`, `f64::ln`, `<R as Float>::exp`, `R::powf`),
//! whether called or named as a value, a method call of one (`x.ln()`), and any `libm::` path, so the pinned library is
//! reached through `Real` alone. The names are `num_traits::Float`'s transcendentals and libm's spellings of them.

use std::fs;
use std::path::{Path, PathBuf};

use validation::negative_control;

/// The transcendentals' names: `num_traits::Float`'s and libm's (`log`, `expm1`, `log1p`, `pow`, `sincos`).
const NAMES: [&str; 31] = [
    "sin", "cos", "tan", "asin", "acos", "atan", "atan2", "sin_cos", "sincos", "exp", "exp2",
    "exp_m1", "expm1", "ln", "log", "log2", "log10", "ln_1p", "log1p", "powf", "pow", "hypot",
    "cbrt", "sinh", "cosh", "tanh", "asinh", "acosh", "atanh", "erf", "gamma",
];

/// The integrator's layers, outside the evaluator (the module's note).
const OUTSIDE: [&str; 6] = [
    "driver",
    "occupant",
    "physics",
    "regularisation",
    "detect",
    "payload",
];

/// `src` with every comment, string and char literal blanked to spaces, newlines kept, so a byte's line is unchanged.
/// With `comments` false, comments are kept as code (the scan's control).
fn blank(src: &str, comments: bool) -> Vec<u8> {
    let b = src.as_bytes();
    let mut out = b.to_vec();
    let mut i = 0;
    let wipe = |out: &mut Vec<u8>, from: usize, to: usize| {
        for c in &mut out[from..to] {
            if *c != b'\n' {
                *c = b' ';
            }
        }
    };
    while i < b.len() {
        if comments && b[i..].starts_with(b"//") {
            let end = b[i..]
                .iter()
                .position(|&c| c == b'\n')
                .map_or(b.len(), |n| i + n);
            wipe(&mut out, i, end);
            i = end;
        } else if comments && b[i..].starts_with(b"/*") {
            let (mut depth, mut j) = (1, i + 2);
            while j < b.len() && depth > 0 {
                if b[j..].starts_with(b"/*") {
                    depth += 1;
                    j += 2;
                } else if b[j..].starts_with(b"*/") {
                    depth -= 1;
                    j += 2;
                } else {
                    j += 1;
                }
            }
            wipe(&mut out, i, j);
            i = j;
        } else if b[i] == b'r'
            && (b.get(i + 1) == Some(&b'"') || b.get(i + 1) == Some(&b'#'))
            && !ident(b, i)
        {
            let hashes = b[i + 1..].iter().take_while(|&&c| c == b'#').count();
            if b.get(i + 1 + hashes) != Some(&b'"') {
                i += 1;
                continue;
            }
            let close: Vec<u8> = std::iter::once(b'"')
                .chain(std::iter::repeat_n(b'#', hashes))
                .collect();
            let body = i + 2 + hashes;
            let end = b[body..]
                .windows(close.len())
                .position(|w| w == close.as_slice())
                .map_or(b.len(), |n| body + n + close.len());
            wipe(&mut out, i, end);
            i = end;
        } else if b[i] == b'"' {
            let mut j = i + 1;
            while j < b.len() && b[j] != b'"' {
                j += if b[j] == b'\\' { 2 } else { 1 };
            }
            let end = (j + 1).min(b.len());
            wipe(&mut out, i, end);
            i = end;
        } else if b[i] == b'\'' {
            // A char literal: `'x'`, `'\n'`, `'\u{…}'` or a multi-byte char; otherwise a lifetime, left as it is.
            let close = if b.get(i + 1) == Some(&b'\\') {
                b[i + 3..]
                    .iter()
                    .position(|&c| c == b'\'')
                    .map(|n| i + 3 + n)
            } else {
                let width = src[i + 1..].chars().next().map_or(1, char::len_utf8);
                (b.get(i + 1 + width) == Some(&b'\'')).then_some(i + 1 + width)
            };
            match close {
                Some(end) => {
                    wipe(&mut out, i, end + 1);
                    i = end + 1;
                }
                None => i += 1,
            }
        } else {
            i += 1;
        }
    }
    out
}

/// Whether `b[i - 1]` is part of an identifier.
fn ident(b: &[u8], i: usize) -> bool {
    i > 0 && (b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_')
}

/// The byte ranges of `real.rs`'s `Real` items in the blanked `code`: `trait Real` and each `impl Real for`, from the
/// header to its closing brace.
fn exempt(code: &[u8]) -> Vec<(usize, usize)> {
    let text = String::from_utf8_lossy(code);
    let mut spans = Vec::new();
    for header in [
        "pub trait Real:",
        "impl Real for f32 {",
        "impl Real for f64 {",
    ] {
        let mut from = 0;
        while let Some(n) = text[from..].find(header) {
            let start = from + n;
            let open = start + text[start..].find('{').expect("an item's brace");
            let mut depth = 0;
            let mut end = open;
            for (k, &c) in code[open..].iter().enumerate() {
                depth += i32::from(c == b'{') - i32::from(c == b'}');
                if depth == 0 {
                    end = open + k;
                    break;
                }
            }
            spans.push((start, end));
            from = end;
        }
    }
    spans
}

/// Each stray transcendental in `src`, the file at `label`, as `label:line: token`; `real.rs`'s `Real` items are
/// exempt. `comments` as [`blank`] reads it.
fn strays(label: &str, src: &str, comments: bool) -> Vec<String> {
    let code = blank(src, comments);
    let spans = if label == "real.rs" {
        exempt(&code)
    } else {
        Vec::new()
    };
    let line = |at: usize| 1 + code[..at].iter().filter(|&&c| c == b'\n').count();
    let mut found = Vec::new();
    let mut i = 0;
    while i < code.len() {
        let start = i;
        if !(code[i].is_ascii_alphabetic() || code[i] == b'_') || ident(&code, i) {
            i += 1;
            continue;
        }
        while i < code.len() && (code[i].is_ascii_alphanumeric() || code[i] == b'_') {
            i += 1;
        }
        let word = std::str::from_utf8(&code[start..i]).expect("an ASCII identifier");
        let after_path = start >= 2 && &code[start - 2..start] == b"::";
        let after_dot =
            start >= 1 && code[start - 1] == b'.' && !(start >= 2 && code[start - 2] == b'.');
        let called = code[i..].iter().find(|c| !c.is_ascii_whitespace()) == Some(&b'(');
        let libm = word == "libm" && code[i..].starts_with(b"::");
        let name = NAMES.contains(&word) && (after_path || (after_dot && called));
        if (libm || name) && !spans.iter().any(|&(a, b)| (a..=b).contains(&start)) {
            let token = if libm { "libm::" } else { word };
            found.push(format!("{label}:{}: {token}", line(start)));
        }
    }
    found
}

/// The evaluator's source files, each with its path below `src/` (the module's note).
fn sources() -> Vec<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut stack: Vec<PathBuf> = vec![root.clone()];
    let mut files = Vec::new();
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).expect("a source directory") {
            let path = entry.expect("a directory entry").path();
            let rel = path
                .strip_prefix(&root)
                .expect("below src")
                .to_string_lossy()
                .replace('\\', "/");
            if path.is_dir() {
                if !OUTSIDE.contains(&rel.as_str()) {
                    stack.push(path);
                }
            } else if rel.ends_with(".rs") {
                files.push((rel, fs::read_to_string(&path).expect("a source file")));
            }
        }
    }
    let entry = Path::new(env!("CARGO_MANIFEST_DIR")).join("../engine/src/decode_dispatch.rs");
    files.push((
        "engine/decode_dispatch.rs".to_owned(),
        fs::read_to_string(&entry).expect("the CPU evaluator's entry"),
    ));
    files.sort();
    files
}

/// Panics naming every stray transcendental in `files`.
fn check(files: &[(String, String)]) {
    assert!(
        files.iter().any(|(rel, _)| rel == "real.rs"),
        "the scan reads src/real.rs"
    );
    assert!(
        files.iter().any(|(rel, _)| rel.starts_with("chart/")),
        "the scan reads src/chart/"
    );
    assert!(
        files
            .iter()
            .any(|(rel, _)| rel == "engine/decode_dispatch.rs"),
        "the scan reads the evaluator's entry, engine's decode_dispatch.rs"
    );
    let found: Vec<String> = files
        .iter()
        .flat_map(|(rel, src)| strays(rel, src, true))
        .collect();
    assert!(
        found.is_empty(),
        "stray transcendental outside Real's pinned methods (R-991): {}",
        found.join(", ")
    );
}

#[test]
fn transcendental_pin() {
    check(&sources());
}

negative_control!(
    transcendental_pin,
    "a planted `Float::sin` in a chart file",
    expected = "stray transcendental",
    {
        let mut files = sources();
        let chart = files
            .iter_mut()
            .find(|(rel, _)| rel == "chart/tilt.rs")
            .expect("chart/tilt.rs");
        chart
            .1
            .push_str("\nfn planted(x: f64) -> f64 {\n    Float::sin(x)\n}\n");
        check(&files)
    }
);

/// The scan's cases: a source line, a file label, and the tokens the scan must flag in it.
const CASES: &[(&str, &str, &[&str])] = &[
    ("let y = Float::sin(x);", "chart/a.rs", &["sin"]),
    ("let y = f64::ln(x);", "chart/a.rs", &["ln"]),
    ("let y = <R as Float>::exp(x);", "chart/a.rs", &["exp"]),
    ("let y = R::powf(x, a);", "chart/a.rs", &["powf"]),
    ("let y = x.ln();", "chart/a.rs", &["ln"]),
    ("let y = x.exp_m1 ();", "chart/a.rs", &["exp_m1"]),
    ("let (s, c) = t.sin_cos();", "chart/a.rs", &["sin_cos"]),
    ("let f = xs.map(f64::cbrt);", "chart/a.rs", &["cbrt"]),
    ("let y = libm::pow(x, a);", "chart/a.rs", &["libm::", "pow"]),
    (
        "let y = Float::hypot(a, b) + Float::atanh(c);",
        "chart/a.rs",
        &["hypot", "atanh"],
    ),
    ("let y = Float::sqrt(x) + x.sqrt();", "chart/a.rs", &[]),
    (
        "let y = Real::sine(x) + x.logarithm() + R::arctan2(a, b);",
        "chart/a.rs",
        &[],
    ),
    ("// Float::sin(x) in a comment", "chart/a.rs", &[]),
    (
        "/* Float::sin(x) /* nested */ Float::cos(x) */ let y = 1;",
        "chart/a.rs",
        &[],
    ),
    ("let s = \"Float::sin(x) \\\" x.ln()\";", "chart/a.rs", &[]),
    ("let s = r#\"Float::sin(x)\"#;", "chart/a.rs", &[]),
    (
        "let c = '('; let q = '\\''; let t: &'static str = x; let y = x.exp();",
        "chart/a.rs",
        &["exp"],
    ),
    ("let t = self.sinh_table + x.exponent;", "chart/a.rs", &[]),
    ("let e = x.exp; let r = 0..ln;", "chart/a.rs", &[]),
    ("let y = my_cos(x) + Float::cos(x);", "chart/a.rs", &["cos"]),
    (
        "impl Real for f64 {\n    fn sine(self) -> f64 {\n        libm::sin(self)\n    }\n}",
        "real.rs",
        &[],
    ),
    (
        "impl Real for f64 {\n    fn sine(self) -> f64 {\n        libm::sin(self)\n    }\n}",
        "chart/a.rs",
        &["libm::", "sin"],
    ),
];

/// Panics where the scan, comments read as `comments` says, flags other tokens than a case lists.
fn check_scan(comments: bool) {
    for &(src, label, want) in CASES {
        let got: Vec<String> = strays(label, src, comments)
            .into_iter()
            .map(|s| s.rsplit(": ").next().expect("a token").to_string())
            .collect();
        assert_eq!(got, want, "scan of `{src}` in {label}");
    }
    let lines = strays(
        "chart/a.rs",
        "fn a() {}\n\n// x\nlet y = x.tan();\n",
        comments,
    );
    assert_eq!(lines, ["chart/a.rs:4: tan"], "the line a token is on");
}

#[test]
fn transcendental_pin_scan() {
    check_scan(true);
}

negative_control!(
    transcendental_pin_scan,
    "a scan that reads comments as code",
    expected = "scan of",
    check_scan(false)
);
