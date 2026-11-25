//! The pin's enforcement (R-991, REQ-CHART-212): every f64 transcendental the CPU chart evaluator reaches goes through
//! the pinned libm, by [`kernel::Real`]'s methods, so no `Float::` or inherent transcendental may stand in the
//! evaluator's source outside `src/real.rs`'s `Real` items (the trait and its two impls), `sqrt` excepted (IEEE 754
//! rounds it correctly on every platform). A source scan in the `discipline_lint` style, not a clippy
//! `disallowed-methods` entry, which would be lint configuration (R-197).
//!
//! - `transcendental_pin`: the evaluator's source holds no stray transcendental; its control plants a `Float::sin`.
//! - `platform_maths`: the production crates (engine, render, gui, prin, ledger) and the kernel's integrator layers
//!   hold no platform maths call (`f64::sin`, `x.exp2()`, `Float::ln`) outside `KEPT`, each kept one with why: the rest
//!   take the pinned libm, `engine::exact` or `kernel::Real`'s. Its control plants an `exp2` in the manifold view;
//!   `platform_maths_scan` holds the scan's cases.
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

// ── The production crates ────────────────────────────────────────────────────────────────────────────────────────

/// The production crates outside the kernel, scanned by [`platform_maths`]: their `src`, test code left out; with them
/// the kernel's integrator layers ([`OUTSIDE`]).
const PRODUCTION: [&str; 5] = ["engine", "render", "gui", "prin", "ledger"];

/// Each platform maths call the production crates may keep, by file and token, how many, and why: each feeds no decoded
/// state, no edit and no stored record, or is not maths. Every other call is the pinned libm's (`libm::`), the
/// engine's correctly rounded `engine::exact`, or `kernel::Real`'s.
const KEPT: &[(&str, &str, usize, &str)] = &[
    (
        "engine/src/contract/store.rs",
        "log",
        1,
        "the store's logging method, not maths",
    ),
    (
        "gui/src/side.rs",
        "log",
        1,
        "the store's logging method, not maths",
    ),
    (
        "gui/src/stain/controls.rs",
        "cos",
        1,
        "draws the angle control's dial hand",
    ),
    (
        "gui/src/stain/controls.rs",
        "sin",
        1,
        "draws the angle control's dial hand",
    ),
    (
        "gui/src/explore/compass.rs",
        "atan2",
        1,
        "orders the slice polygon's corners for drawing",
    ),
    (
        "gui/src/explore/compass.rs",
        "hypot",
        2,
        "places an axis label on screen",
    ),
    (
        "gui/src/explore/compass.rs",
        "sin",
        2,
        "the axes' angle in the dead-zone readout text",
    ),
    (
        "gui/src/explore/compass.rs",
        "acos",
        1,
        "the axes' angle in the dead-zone readout text",
    ),
    (
        "gui/src/explore/compass.rs",
        "powf",
        1,
        "the compass drawing's easing per frame",
    ),
    (
        "gui/src/explore/trajectory.rs",
        "exp2",
        6,
        "the trajectory plot's on-screen zoom",
    ),
    (
        "gui/src/explore/trajectory.rs",
        "sin",
        3,
        "the trajectory plot's 3-D projection, drawn",
    ),
    (
        "gui/src/explore/trajectory.rs",
        "cos",
        2,
        "the trajectory plot's 3-D projection, drawn",
    ),
    (
        "gui/src/explore/trajectory.rs",
        "sin_cos",
        2,
        "the trajectory plot's camera, drawn",
    ),
    (
        "gui/src/explore/axis_labels.rs",
        "log10",
        1,
        "an axis label's decimals",
    ),
    (
        "gui/src/windows/profiler.rs",
        "sin",
        1,
        "the profiler's pie, drawn",
    ),
    (
        "gui/src/windows/profiler.rs",
        "cos",
        1,
        "the profiler's pie, drawn",
    ),
    (
        "gui/src/windows/profiler.rs",
        "ln_1p",
        2,
        "the profiler's histogram bars, drawn",
    ),
    (
        "gui/src/windows/inspector.rs",
        "sin_cos",
        1,
        "the inspector's gauge-turned display",
    ),
    (
        "gui/src/windows/inspector.rs",
        "exp",
        2,
        "the inspector's mass readout",
    ),
    (
        "gui/src/windows/chart_builder.rs",
        "sin",
        2,
        "the chart builder's drawn shape and its hit test",
    ),
    ("gui/src/bench.rs", "ln", 1, "the benchmark's statistics"),
    ("gui/src/bench.rs", "exp", 1, "the benchmark's statistics"),
    (
        "render/src/presets/roundtrip.rs",
        "hypot",
        1,
        "a preset's pixel colour",
    ),
    (
        "render/src/presets/roundtrip.rs",
        "log2",
        1,
        "a preset's pixel colour",
    ),
    (
        "render/src/presets/agreement.rs",
        "log2",
        1,
        "a preset's pixel colour",
    ),
    (
        "render/src/colour/field_ramp.rs",
        "ln",
        2,
        "a pixel colour's compaction, the GPU ramp's CPU twin",
    ),
    (
        "render/src/colour/space.rs",
        "powf",
        3,
        "sRGB's transfer curve, a pixel colour",
    ),
    (
        "render/src/colour/space.rs",
        "cbrt",
        1,
        "OKLab's cube root, a pixel colour",
    ),
    (
        "render/src/colour/space.rs",
        "atan2",
        2,
        "OKLCh's hue, a pixel colour",
    ),
    (
        "render/src/colour/space.rs",
        "cos",
        2,
        "OKLCh's hue, a pixel colour",
    ),
    (
        "render/src/colour/space.rs",
        "sin",
        2,
        "OKLCh's hue, a pixel colour",
    ),
    (
        "render/src/present.rs",
        "ln",
        2,
        "a pixel colour's compaction",
    ),
    (
        "ledger/src/gen/numeric.rs",
        "ln",
        1,
        "the colour compaction's CPU twin, a pixel colour",
    ),
    (
        "ledger/src/links/admissible.rs",
        "exp",
        3,
        "`Fl`'s methods, the pinned libm's at each width",
    ),
    (
        "ledger/src/links/admissible.rs",
        "ln",
        3,
        "`Fl`'s methods, the pinned libm's at each width",
    ),
    (
        "ledger/src/links/admissible.rs",
        "ln_1p",
        1,
        "`Fl`'s methods, the pinned libm's at each width",
    ),
    (
        "ledger/src/links/admissible.rs",
        "exp_m1",
        1,
        "`Fl`'s methods, the pinned libm's at each width",
    ),
    (
        "ledger/src/links/admissible.rs",
        "tanh",
        1,
        "`Fl`'s methods, the pinned libm's at each width",
    ),
    (
        "ledger/src/links/admissible.rs",
        "atanh",
        1,
        "`Fl`'s methods, the pinned libm's at each width",
    ),
    (
        "ledger/src/links/admissible.rs",
        "cosh",
        1,
        "`Fl`'s methods, the pinned libm's at each width",
    ),
];

/// `code`, `src` blanked, with each item under `#[cfg(test)]` or `#[cfg(feature = "controls")]` blanked too: from the attribute
/// to the item's closing brace, or its `;` where that comes first.
fn without_tests(src: &str, code: &mut [u8]) {
    for attr in ["#[cfg(test)]", "#[cfg(feature = \"controls\")]"] {
        let mut from = 0;
        while let Some(n) = src[from..].find(attr) {
            let start = from + n;
            // A match inside a comment or a string is blanked in `code`: not an attribute.
            if code[start] != b'#' {
                from = start + attr.len();
                continue;
            }
            let rest = &code[start + attr.len()..];
            let brace = rest.iter().position(|&c| c == b'{');
            let semi = rest.iter().position(|&c| c == b';');
            let end = match (brace, semi) {
                (Some(b), Some(s)) if s < b => start + attr.len() + s,
                (Some(b), _) => {
                    let open = start + attr.len() + b;
                    let mut depth = 0;
                    let mut end = open;
                    for (k, &c) in code[open..].iter().enumerate() {
                        depth += i32::from(c == b'{') - i32::from(c == b'}');
                        if depth == 0 {
                            end = open + k;
                            break;
                        }
                    }
                    end
                }
                (None, Some(s)) => start + attr.len() + s,
                (None, None) => code.len() - 1,
            };
            for c in &mut code[start..=end] {
                if *c != b'\n' {
                    *c = b' ';
                }
            }
            from = end + 1;
        }
    }
}

/// Each platform maths call in `src`, a production file, as `(line, token)`: a transcendental's name as a method call or
/// on a float's path (`f64::sin`, `Float::ln`, `<f32>::cos`); a `libm::` or `engine::exact::` path is pinned, and a
/// module's path (`crate::log`) is not a call.
fn platform_calls(src: &str) -> Vec<(usize, String)> {
    let mut code = blank(src, true);
    without_tests(src, &mut code);
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
        let float_path = [b"f32::".as_slice(), b"f64::", b"Float::", b">::"]
            .iter()
            .any(|p| start >= p.len() && &code[start - p.len()..start] == *p);
        let after_dot =
            start >= 1 && code[start - 1] == b'.' && !(start >= 2 && code[start - 2] == b'.');
        let called = code[i..].iter().find(|c| !c.is_ascii_whitespace()) == Some(&b'(');
        // `pow` is no float method (f64's are `powf` and `powi`): `.pow(` is an integer's, exact.
        let method = after_dot && called && word != "pow";
        if NAMES.contains(&word) && (float_path || method) {
            found.push((line(start), word.to_owned()));
        }
    }
    found
}

/// The production crates' source files, test files left out (a `tests` directory or a `tests.rs`), each labelled
/// `<crate>/src/<path>`.
fn production_sources() -> Vec<(String, String)> {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    for name in PRODUCTION {
        let root = crates.join(name).join("src");
        let mut stack = vec![root.clone()];
        while let Some(dir) = stack.pop() {
            for entry in fs::read_dir(&dir).expect("a source directory") {
                let path = entry.expect("a directory entry").path();
                let rel = path
                    .strip_prefix(&root)
                    .expect("below src")
                    .to_string_lossy()
                    .replace('\\', "/");
                let test_file = rel.split('/').any(|s| s == "tests" || s == "tests.rs");
                // The mock side's placeholder content, behind `--features mock`'s start, decodes no real state.
                let test_file = test_file || (name == "gui" && rel.starts_with("mock"));
                if path.is_dir() && !test_file {
                    stack.push(path);
                } else if rel.ends_with(".rs") && !test_file {
                    let text = fs::read_to_string(&path).expect("a source file");
                    files.push((format!("{name}/src/{rel}"), text));
                }
            }
        }
    }
    // The kernel's integrator layers, which `transcendental_pin` leaves out: they write the stored records.
    let kernel = crates.join("kernel").join("src");
    for layer in OUTSIDE {
        let mut stack = vec![kernel.join(layer)];
        while let Some(dir) = stack.pop() {
            for entry in fs::read_dir(&dir).expect("a source directory") {
                let path = entry.expect("a directory entry").path();
                let rel = path
                    .strip_prefix(&kernel)
                    .expect("below src")
                    .to_string_lossy()
                    .replace('\\', "/");
                if path.is_dir() {
                    stack.push(path);
                } else if rel.ends_with(".rs") {
                    files.push((
                        format!("kernel/src/{rel}"),
                        fs::read_to_string(&path).expect("a source file"),
                    ));
                }
            }
        }
    }
    files.sort();
    files
}

/// Panics naming every platform maths call in `files` past [`KEPT`]'s count for its file and token, and every kept
/// entry no longer there in full.
fn check_production(files: &[(String, String)]) {
    use std::collections::BTreeMap;
    assert!(
        files
            .iter()
            .any(|(l, _)| l == "gui/src/explore/manifold_view/mod.rs"),
        "the scan reads the GUI's manifold view"
    );
    assert!(
        files
            .iter()
            .any(|(l, _)| l == "kernel/src/driver/benettin.rs"),
        "the scan reads the kernel's integrator layers"
    );
    let mut counts: BTreeMap<(String, String), Vec<usize>> = BTreeMap::new();
    for (label, src) in files {
        for (line, token) in platform_calls(src) {
            counts.entry((label.clone(), token)).or_default().push(line);
        }
    }
    let mut new = Vec::new();
    for ((label, token), lines) in &counts {
        let kept = KEPT
            .iter()
            .find(|k| k.0 == label && k.1 == token)
            .map_or(0, |k| k.2);
        if lines.len() > kept {
            new.push(format!("{label}:{lines:?}: {token} ({} kept)", kept));
        }
    }
    assert!(
        new.is_empty(),
        "a platform maths call in production code: {}; take `libm::`, `engine::exact` or `kernel::Real`'s, or keep it \
         in KEPT with why",
        new.join(", ")
    );
    let gone: Vec<String> = KEPT
        .iter()
        .filter(|k| {
            counts
                .get(&(k.0.to_owned(), k.1.to_owned()))
                .map_or(0, Vec::len)
                < k.2
        })
        .map(|k| format!("{} {}", k.0, k.1))
        .collect();
    assert!(
        gone.is_empty(),
        "KEPT lists calls no longer there: {}",
        gone.join(", ")
    );
}

#[test]
fn platform_maths() {
    check_production(&production_sources());
}

negative_control!(
    platform_maths,
    "a planted `f64::exp2` in the manifold view",
    expected = "a platform maths call in production code",
    {
        let mut files = production_sources();
        let view = files
            .iter_mut()
            .find(|(l, _)| l == "gui/src/explore/manifold_view/mod.rs")
            .expect("the manifold view");
        view.1
            .push_str("\nfn planted(z: f64) -> f64 {\n    z.exp2()\n}\n");
        check_production(&files)
    }
);

/// The production scan's own cases: what it flags and what it passes, by `scan`.
fn check_production_scan(scan: fn(&str) -> Vec<(usize, String)>) {
    let cases: &[(&str, &[&str])] = &[
        ("let y = x.exp2();", &["exp2"]),
        ("let y = f64::hypot(a, b);", &["hypot"]),
        (
            "let y = libm::hypot(a, b) + engine::exact::atan2(a, b);",
            &[],
        ),
        ("let (s, c) = exact::sin_cos(t);", &[]),
        (
            "#[cfg(test)]\nmod tests {\n    fn f(x: f64) -> f64 { x.sin() }\n}\nlet y = x.cos();",
            &["cos"],
        ),
        (
            "#[cfg(feature = \"controls\")]\nfn f(x: f64) -> f64 { x.sin() }",
            &[],
        ),
        ("#[cfg(test)]\nmod tests;\nlet y = x.tanh();", &["tanh"]),
    ];
    for &(src, want) in cases {
        let got: Vec<String> = scan(src).into_iter().map(|(_, t)| t).collect();
        assert_eq!(got, want, "production scan of `{src}`");
    }
}

#[test]
fn platform_maths_scan() {
    check_production_scan(platform_calls);
}

negative_control!(
    platform_maths_scan,
    "a scan that reads test code as production code",
    expected = "production scan of",
    check_production_scan(|src| {
        let mut out = platform_calls(src);
        if src.contains("#[cfg(") {
            out.insert(0, (1, "sin".to_owned()));
        }
        out
    })
);
