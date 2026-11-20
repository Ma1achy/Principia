//! QA tests for TASK-M0-27, written from REQ-VAL-157: "a check copied between qa's test files and the `*_controls.rs`
//! targets (`qa_TASK-M0-04*`, `qa_TASK-M0-21*`, prin's `qa_TASK-M0-01`) must live once, in a shared test-support
//! module that the test and its control both call (R-215)", verified as "no check body is copied between qa's test
//! files and the controls targets; each control calls the same check its test does".
//!
//! Both tests read the source files the requirement names. A small lexer blanks comments and string literals, so the
//! structure (macro calls, function bodies, parentheses) is found in code only:
//! - no assertion message in a controls target is a copy of one in qa's test files or the shared modules;
//! - every `negative_control!` in those targets calls a shared check (a support-module function holding an
//!   assertion) that the test it names also calls.
//!
//! Each test registers a negative control (R-176, R-199, R-212): the same check, run on the controls source with the
//! pre-TASK-M0-27 shape of a control (the check copied inline) appended, must fail.

use std::path::{Path, PathBuf};
use validation::negative_control;

/// A test file group REQ-VAL-157 names: qa's test files, their controls target, and the shared modules (paths from
/// the workspace root).
struct Group {
    tests: &'static [&'static str],
    controls: &'static str,
    supports: &'static [&'static str],
}

const GROUPS: &[Group] = &[
    Group {
        tests: &["crates/prin/tests/qa_TASK-M0-01.rs"],
        controls: "crates/prin/tests/qa_TASK-M0-01_controls.rs",
        supports: &["crates/prin/tests/support/qa_m0_01.rs"],
    },
    Group {
        tests: &[
            "crates/validation/tests/qa_TASK-M0-04.rs",
            "crates/validation/tests/qa_TASK-M0-04_r2.rs",
        ],
        controls: "crates/validation/tests/qa_TASK-M0-04_controls.rs",
        supports: &[
            "crates/validation/tests/support/qa_m0_04.rs",
            "crates/validation/tests/support/qa_m0_04_r2.rs",
        ],
    },
    Group {
        tests: &[
            "crates/validation/tests/qa_TASK-M0-21.rs",
            "crates/validation/tests/qa_TASK-M0-21_r2.rs",
        ],
        controls: "crates/validation/tests/qa_TASK-M0-21_controls.rs",
        supports: &[
            "crates/validation/tests/support/qa_m0_21.rs",
            "crates/validation/tests/support/qa_m0_21_r2.rs",
            "crates/validation/tests/support/qa_m0_21_fixture.rs",
        ],
    },
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// A lexed source file: `code` is the text with comments and string/char literals blanked (same byte offsets);
/// `literals` are each string literal's byte range and normalised contents.
struct Src {
    name: String,
    code: Vec<u8>,
    literals: Vec<(usize, usize, String)>,
}

/// The group's files, read from the checkout.
struct Loaded {
    tests: Vec<Src>,
    controls: Src,
    supports: Vec<Src>,
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("read {rel}: {e}"))
}

impl Group {
    fn load(&self) -> Loaded {
        Loaded {
            tests: self.tests.iter().map(|f| lex(f, &read(f))).collect(),
            controls: lex(self.controls, &read(self.controls)),
            supports: self.supports.iter().map(|f| lex(f, &read(f))).collect(),
        }
    }
}

fn is_ident(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Drops `\`-newline continuations and their indentation, so a message split over lines compares whole.
fn normalise(inner: &str) -> String {
    let mut out = String::new();
    let mut chars = inner.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && chars.peek() == Some(&'\n') {
            chars.next();
            while chars.peek().is_some_and(|c| c.is_whitespace()) {
                chars.next();
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn lex(name: &str, text: &str) -> Src {
    let s = text.as_bytes();
    let mut code = s.to_vec();
    let mut literals = Vec::new();
    let blank = |code: &mut Vec<u8>, a: usize, b: usize| {
        for c in &mut code[a..b] {
            if *c != b'\n' {
                *c = b' ';
            }
        }
    };
    let mut i = 0;
    while i < s.len() {
        let prev_ident = i > 0 && is_ident(s[i - 1]);
        if s[i..].starts_with(b"//") {
            let end = s[i..]
                .iter()
                .position(|&c| c == b'\n')
                .map_or(s.len(), |p| i + p);
            blank(&mut code, i, end);
            i = end;
        } else if s[i..].starts_with(b"/*") {
            let (mut depth, mut j) = (1, i + 2);
            while j < s.len() && depth > 0 {
                if s[j..].starts_with(b"/*") {
                    depth += 1;
                    j += 2;
                } else if s[j..].starts_with(b"*/") {
                    depth -= 1;
                    j += 2;
                } else {
                    j += 1;
                }
            }
            blank(&mut code, i, j);
            i = j;
        } else if !prev_ident
            && (s[i] == b'r' || (s[i] == b'b' && s.get(i + 1) == Some(&b'r')))
            && {
                let k = if s[i] == b'b' { i + 2 } else { i + 1 };
                let hashes = s[k..].iter().take_while(|&&c| c == b'#').count();
                s.get(k + hashes) == Some(&b'"')
            }
        {
            let k = if s[i] == b'b' { i + 2 } else { i + 1 };
            let hashes = s[k..].iter().take_while(|&&c| c == b'#').count();
            let open = k + hashes + 1;
            let mut close = vec![b'"'];
            close.extend(std::iter::repeat_n(b'#', hashes));
            let at = s[open..]
                .windows(close.len())
                .position(|w| w == close.as_slice())
                .map(|p| open + p)
                .unwrap_or_else(|| panic!("{name}: unterminated raw string at byte {i}"));
            let end = at + close.len();
            literals.push((i, end, text[open..at].to_owned()));
            blank(&mut code, i, end);
            i = end;
        } else if s[i] == b'"' || (!prev_ident && s[i] == b'b' && s.get(i + 1) == Some(&b'"')) {
            let open = if s[i] == b'b' { i + 2 } else { i + 1 };
            let mut j = open;
            while s[j] != b'"' {
                j += if s[j] == b'\\' { 2 } else { 1 };
            }
            literals.push((i, j + 1, normalise(&text[open..j])));
            blank(&mut code, i, j + 1);
            i = j + 1;
        } else if s[i] == b'\'' {
            // A char literal ('x', '\n', '\''), else a lifetime or label.
            if s.get(i + 1) == Some(&b'\\') {
                let end = i + 2 + s[i + 2..].iter().position(|&c| c == b'\'').unwrap() + 1;
                let end = if end == i + 3 { i + 4 } else { end }; // '\''
                blank(&mut code, i, end);
                i = end;
            } else if s.get(i + 2) == Some(&b'\'') {
                blank(&mut code, i, i + 3);
                i += 3;
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    Src {
        name: name.to_owned(),
        code,
        literals,
    }
}

/// Byte offsets where `token` starts in code, not preceded by an identifier character.
fn find_all(code: &[u8], token: &str) -> Vec<usize> {
    let t = token.as_bytes();
    (0..code.len().saturating_sub(t.len() - 1))
        .filter(|&i| &code[i..i + t.len()] == t && (i == 0 || !is_ident(code[i - 1])))
        .collect()
}

/// The index just past the bracket matching the one at `open`.
fn matching(code: &[u8], open: usize) -> usize {
    let (o, c) = (code[open], if code[open] == b'(' { b')' } else { b'}' });
    let mut depth = 0;
    for (k, &b) in code.iter().enumerate().skip(open) {
        if b == o {
            depth += 1;
        } else if b == c {
            depth -= 1;
            if depth == 0 {
                return k + 1;
            }
        }
    }
    panic!("unbalanced bracket at byte {open}")
}

const ASSERTS: &[&str] = &[
    "assert!(",
    "assert_eq!(",
    "assert_ne!(",
    "panic!(",
    "unreachable!(",
];

/// The argument ranges of every assertion macro in `range`.
fn assertions(src: &Src, range: (usize, usize)) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for m in ASSERTS {
        for at in find_all(&src.code[..range.1], m) {
            if at >= range.0 {
                let open = at + m.len() - 1;
                out.push((open, matching(&src.code, open)));
            }
        }
    }
    out
}

/// The messages of every assertion in the file (string literals inside an assertion macro's arguments).
fn messages(src: &Src) -> Vec<String> {
    let calls = assertions(src, (0, src.code.len()));
    src.literals
        .iter()
        .filter(|(a, _, _)| calls.iter().any(|&(o, c)| *a > o && *a < c))
        .map(|(_, _, t)| t.clone())
        .collect()
}

/// A message's fixed words: the text before its first placeholder, trailing `:`, newlines and spaces removed.
fn stem(message: &str) -> String {
    let head = message.split('{').next().unwrap_or_default();
    head.trim_end_matches([':', ' ', '\n'])
        .trim_end_matches("\\n")
        .trim_end_matches([':', ' '])
        .to_owned()
}

/// The shortest stem compared: shorter ones ("child failed") are too generic to name a copied check.
const MIN_STEM: usize = 12;

/// REQ-VAL-157's "no check body is copied between qa's test files and the controls targets": no assertion in the
/// controls target carries a message whose fixed words are those of an assertion in qa's test files or the shared
/// modules (a copied check keeps its message, as every pre-TASK-M0-27 copy did, reworded or not).
fn check_no_copied_message(g: &Loaded) {
    let theirs: Vec<String> = g
        .tests
        .iter()
        .chain(&g.supports)
        .flat_map(messages)
        .map(|m| stem(&m))
        .filter(|s| s.len() >= MIN_STEM)
        .collect();
    for mine in messages(&g.controls) {
        let s = stem(&mine);
        if s.len() < MIN_STEM {
            continue;
        }
        if let Some(t) = theirs.iter().find(|t| t.contains(s.as_str())) {
            panic!(
                "{} copies the check message {mine:?} (qa's check: {t:?}) instead of calling the shared module",
                g.controls.name
            );
        }
    }
}

/// The body range of `fn name(` in `src`, if defined there.
fn body(src: &Src, name: &str) -> Option<(usize, usize)> {
    let at = *find_all(&src.code, &format!("fn {name}(")).first()?;
    let open = at + src.code[at..].iter().position(|&c| c == b'{')?;
    Some((open, matching(&src.code, open)))
}

/// Whether `range` of `src` calls `name(` (not its definition).
fn calls(src: &Src, range: (usize, usize), name: &str) -> bool {
    find_all(&src.code[..range.1], &format!("{name}("))
        .into_iter()
        .any(|at| at >= range.0 && !src.code[..at].ends_with(b"fn "))
}

/// The shared checks: `pub fn`s of the support modules whose own body holds an assertion.
fn shared_checks(g: &Loaded) -> Vec<String> {
    let mut out = Vec::new();
    for src in &g.supports {
        for at in find_all(&src.code, "pub fn ") {
            let rest = &src.code[at + 7..];
            let len = rest.iter().take_while(|&&c| is_ident(c)).count();
            let name = String::from_utf8_lossy(&rest[..len]).into_owned();
            if let Some(b) = body(src, &name) {
                if !assertions(src, b).is_empty() {
                    out.push(name);
                }
            }
        }
    }
    out
}

/// REQ-VAL-157's "each control calls the same check its test does": every `negative_control!` in the controls target
/// names a test in qa's files, and its body calls a shared check that the test's body calls too.
fn check_controls_call_their_tests_check(g: &Loaded) {
    let shared = shared_checks(g);
    let c = &g.controls;
    let sites = find_all(&c.code, "negative_control!(");
    assert!(!sites.is_empty(), "{}: no negative_control! found", c.name);
    for at in sites {
        let open = at + "negative_control!".len();
        let end = matching(&c.code, open);
        let head = &c.code[open + 1..end];
        let skip = head.iter().take_while(|b| b.is_ascii_whitespace()).count();
        let len = head[skip..].iter().take_while(|&&b| is_ident(b)).count();
        let test = String::from_utf8_lossy(&head[skip..skip + len]).into_owned();
        let (file, range) = g
            .tests
            .iter()
            .find_map(|t| body(t, &test).map(|r| (t, r)))
            .unwrap_or_else(|| panic!("{}: control `{test}` names no test in qa's files", c.name));
        let common: Vec<&String> = shared
            .iter()
            .filter(|f| calls(c, (open, end), f) && calls(file, range, f))
            .collect();
        assert!(
            !common.is_empty(),
            "{}: control `{test}` calls no shared check its test calls (shared checks: {shared:?})",
            c.name
        );
    }
}

#[test]
fn qa_m0_27_no_check_message_is_copied_into_a_controls_target() {
    for g in GROUPS {
        check_no_copied_message(&g.load());
    }
}

#[test]
fn qa_m0_27_each_control_calls_the_shared_check_its_test_calls() {
    for g in GROUPS {
        let loaded = g.load();
        assert!(
            !shared_checks(&loaded).is_empty(),
            "{}: no shared check found in {:?}",
            g.controls,
            g.supports
        );
        check_controls_call_their_tests_check(&loaded);
    }
}

/// The pre-TASK-M0-27 shape of a control (from `qa_TASK-M0-21_controls.rs` before this task): the test's check
/// copied inline, its message reworded to `{all}`, calling no shared check.
#[cfg(feature = "controls")]
const COPIED_CONTROL: &str = r#"
negative_control!(
    qa_control_under_another_name_does_not_pair,
    "with the misnamed control removed, the rightly named one pairs, so the must-fail check must fail",
    expected = "a test whose only control names another test passed",
    {
        let (ok, all) = controls("misnamed", "tests/misnamed.rs");
        assert!(!ok, "a test whose only control names another test passed:\n{all}");
    }
);
"#;

/// The M0-21 group with `COPIED_CONTROL` appended to its controls target.
#[cfg(feature = "controls")]
fn with_copied_control() -> Loaded {
    let g = &GROUPS[2];
    let text = read(g.controls) + COPIED_CONTROL;
    Loaded {
        controls: lex(g.controls, &text),
        ..g.load()
    }
}

negative_control!(
    qa_m0_27_no_check_message_is_copied_into_a_controls_target,
    "a control with its test's check copied inline must fail the no-copy check",
    expected = "copies the check message",
    check_no_copied_message(&with_copied_control())
);

negative_control!(
    qa_m0_27_each_control_calls_the_shared_check_its_test_calls,
    "a control with its test's check copied inline calls no shared check, so the both-call check must fail",
    expected = "control `qa_control_under_another_name_does_not_pair` calls no shared check its test calls",
    check_controls_call_their_tests_check(&with_copied_control())
);
