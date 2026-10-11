//! QA tests for PR #182, from `cargo xtask lint constants`' specification (dd_generation_root §3.8, "Reading a
//! constant"; the lint's module doc): the lint fails on each numeric `const` or `static` *item* in the physics and engine
//! crates, by line and name, and on nothing else. These pin, independently of the implementation:
//! - a `let` binding is not an item, even when its name is the raw identifier `r#const` or `r#static`;
//! - an item's initializer is read after a byte or C raw string (`br"…"`, `cr"…"`, with or without `#`s), whose
//!   contents are blanked like any string's, a backslash included;
//! - an item a `macro_rules!` body declares under a metavariable (`const $name: u32 = 64;`) is a numeric constant all
//!   the same, and one whose value is itself a metavariable or read from the register is not.
//!
//! Each test has a registered negative control (R-176).

use validation::negative_control;
use xtask::lint_constants::scan;

/// Asserts the lint's findings in `source` are exactly `want`, as (line, name), in source order.
fn expect(source: &str, want: &[(usize, &str)]) {
    let want: Vec<(usize, String)> = want.iter().map(|&(l, n)| (l, n.to_owned())).collect();
    assert_eq!(
        scan(source),
        want,
        "qa: unexpected lint findings in:\n{source}"
    );
}

/// `let` bindings named by raw identifiers that spell the keywords; a `const` block and a `const fn` hold numbers but
/// are not items with an initializer. Only `REAL` is a numeric item.
const RAW_IDENTS: &str = "fn f() {
    let r#const = 5;
    let r#static = 6;
    let k = const { 7 };
}
const fn g() -> u8 { 8 }
const REAL: u8 = 9;
";

#[test]
fn qa_lint_constants_a_let_named_by_a_raw_keyword_is_no_item() {
    expect(RAW_IDENTS, &[(7, "REAL")]);
}

negative_control!(
    qa_lint_constants_a_let_named_by_a_raw_keyword_is_no_item,
    "with `let r#const` turned into the item `const C: u8`, line 2 is a numeric item the check must report",
    expected = "qa: unexpected lint findings",
    expect(
        &RAW_IDENTS.replace("let r#const", "const C: u8"),
        &[(7, "REAL")]
    )
);

/// Byte and C raw strings, with and without `#`s, ending in a backslash or holding a quote or an item's text: each is
/// blanked to its own close, and the item after it is read.
const RAW_BYTES: &str = r####"const A: &[u8] = br"\"; const AFTER_A: u8 = 1;
const B: &core::ffi::CStr = cr"\"; const AFTER_B: u8 = 2;
const C: &[u8] = br#"say "hi" \"#; const AFTER_C: u8 = 3;
const D: &core::ffi::CStr = cr##"const IN_D: u8 = 4; "# \"##; const AFTER_D: u8 = 5;
const E: &[u8] = br"const IN_E: u8 = 6;";
const F: u8 = 7;
"####;

#[test]
fn qa_lint_constants_reads_past_byte_and_c_raw_strings() {
    expect(
        RAW_BYTES,
        &[
            (1, "AFTER_A"),
            (2, "AFTER_B"),
            (3, "AFTER_C"),
            (4, "AFTER_D"),
            (6, "F"),
        ],
    );
}

negative_control!(
    qa_lint_constants_reads_past_byte_and_c_raw_strings,
    "with the byte raw string on line 5 made plain code, IN_E is a numeric item the check must report",
    expected = "qa: unexpected lint findings",
    expect(
        &RAW_BYTES.replace(r#"br"const IN_E: u8 = 6;";"#, "0; const IN_E: u8 = 6;"),
        &[
            (1, "AFTER_A"),
            (2, "AFTER_B"),
            (3, "AFTER_C"),
            (4, "AFTER_D"),
            (6, "F"),
        ]
    )
);

/// A `macro_rules!` body's items named by metavariables: numeric ones are findings named by the metavariable, a
/// `static mut` one included; one whose value is a metavariable or read from the register is not; nor is the
/// matcher's `const` pattern.
const MACRO: &str = "macro_rules! limits {
    (const $name:ident = $val:expr) => {
        const $name: u32 = $val;
        pub const $cap: u32 = 64;
        static mut $count: u32 = 0;
        static $scale: f32 = 0.5;
        const $read: u32 = ledger::constants::FGW_CAPACITY;
    };
}
";

#[test]
fn qa_lint_constants_reads_a_metavariable_named_macro_item() {
    expect(MACRO, &[(4, "$cap"), (5, "$count"), (6, "$scale")]);
}

negative_control!(
    qa_lint_constants_reads_a_metavariable_named_macro_item,
    "with $read's value a bare number rather than the register's, line 7 is a finding the check must report",
    expected = "qa: unexpected lint findings",
    expect(
        &MACRO.replace("ledger::constants::FGW_CAPACITY", "4096"),
        &[(4, "$cap"), (5, "$count"), (6, "$scale")]
    )
);
