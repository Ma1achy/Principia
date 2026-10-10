//! `cargo xtask lint constants`' reading of a source file (dd_generation_root §3.8, "Reading a constant"; REQ-SYS-001,
//! REQ-SYS-005): each `const` and `static` item whose initializer holds a numeric literal is a finding, by its line and
//! name, and nothing else is. Each test pins a part of that reading the lint's doc comments define: what an item is
//! (not a lifetime, a raw pointer or a const generic parameter), where its initializer starts and ends, what a numeric
//! literal is (a digit starting a token, not a tuple index), and what is blanked first (comments, nested or not,
//! string, raw string and character literals). Written to kill the mutants `xtask/src/lint_constants.rs` carried from
//! before the per-PR mutants gate. Each test has a registered negative control (R-176).

use validation::negative_control;
use xtask::lint_constants::scan;

/// Asserts the lint's findings in `source` are exactly `want`, as (line, name), in source order.
fn check_scan(source: &str, want: &[(usize, &str)]) {
    let want: Vec<(usize, String)> = want.iter().map(|&(l, n)| (l, n.to_owned())).collect();
    assert_eq!(
        scan(source),
        want,
        "the lint's findings are not the expected ones in:\n{source}"
    );
}

/// A `static mut` item is named after `mut`; `'static` is a lifetime and `*const` a raw pointer, not items; the file
/// ends in an identifier, with no newline.
const ITEMS: &str = "static mut COUNT: u32 = 0;
fn f() {
    let r: &'static Grid = &Grid(4);
    let p: *const Grid = GRIDS.as_ptr().wrapping_add(2);
}
pub use grid::Grid";

#[test]
fn lint_constants_reads_items_not_lifetimes_or_raw_pointers() {
    check_scan(ITEMS, &[(1, "COUNT")]);
}

negative_control!(
    lint_constants_reads_items_not_lifetimes_or_raw_pointers,
    "with `static` in place of the lifetime `'static`, line 3 declares a static item holding 4, a finding the check \
     must report",
    expected = "the lint's findings are not the expected ones",
    check_scan(&ITEMS.replace("&'static", "&static"), &[(1, "COUNT")])
);

/// A `macro_rules!` body declares its item under a metavariable: a numeric constant all the same.
const MACRO: &str = "macro_rules! limit {
    ($name:ident) => {
        pub const $name: u32 = 64;
    };
}
";

#[test]
fn lint_constants_reads_a_macro_item_named_by_a_metavariable() {
    check_scan(MACRO, &[(3, "$name")]);
}

negative_control!(
    lint_constants_reads_a_macro_item_named_by_a_metavariable,
    "the same macro item read from the register is no finding, so the check must fail",
    expected = "the lint's findings are not the expected ones",
    check_scan(
        &MACRO.replace("64", "ledger::constants::FGW_CAPACITY.number() as u32"),
        &[(3, "$name")]
    )
);

/// Initializers run from the item's `=` to its `;`: across generic arguments, tuples, a comparison, an arrow, an
/// equality and an associated-type binding; a const generic parameter has none, defaulted type parameter or not; an
/// item inside an `impl` with a const generic parameter is read.
const INITIALIZERS: &str = "const MAP: Map<u8, u8> = Map::new(3);
const PAIR: (u8, u8) = (1, 2);
const BIG: bool = LIMIT > 3;
const PICK: fn() -> u8 = PICKS[2];
const SAME: bool = 2 == LIMIT;
const ITER: Option<&dyn Iterator<Item = u8>> = pick(4);
const NAME: u32= LIMIT;
pub struct Grid<const N: usize, T = [u8; 4]>(T);
impl<const N: usize> Grid<N> {
    const ONE: u8 = 1;
}
";

#[test]
fn lint_constants_reads_each_initializer_from_its_eq_to_its_semicolon() {
    check_scan(
        INITIALIZERS,
        &[
            (1, "MAP"),
            (2, "PAIR"),
            (3, "BIG"),
            (4, "PICK"),
            (5, "SAME"),
            (6, "ITER"),
            (10, "ONE"),
        ],
    );
}

negative_control!(
    lint_constants_reads_each_initializer_from_its_eq_to_its_semicolon,
    "with BIG's literal named instead, line 3 is no finding, so the check must fail",
    expected = "the lint's findings are not the expected ones",
    check_scan(
        &INITIALIZERS.replace("LIMIT > 3", "LIMIT > THREE"),
        &[
            (1, "MAP"),
            (2, "PAIR"),
            (3, "BIG"),
            (4, "PICK"),
            (5, "SAME"),
            (6, "ITER"),
            (10, "ONE"),
        ]
    )
);

/// A numeric literal is a digit that starts a token: not one inside an identifier, after `_`, or a tuple index; one
/// straight after the `=` is.
const NUMBERS: &str = "const FIELD: u8 = PAIR.0;
const WORD: u8 = X2;
const UNDER: u8 = MAX_2;
const TIGHT: u8 =7;
const FRACTION: f64 = 0.25;
";

#[test]
fn lint_constants_a_numeric_literal_starts_a_token() {
    check_scan(NUMBERS, &[(4, "TIGHT"), (5, "FRACTION")]);
}

negative_control!(
    lint_constants_a_numeric_literal_starts_a_token,
    "with FIELD adding 0 rather than reading a tuple index, line 1 is a finding the check must report",
    expected = "the lint's findings are not the expected ones",
    check_scan(
        &NUMBERS.replace("PAIR.0", "PAIR + 0"),
        &[(4, "TIGHT"), (5, "FRACTION")]
    )
);

/// Comments (nested block comments too), strings with escaped quotes, raw strings (byte and C ones too) and character
/// literals (escaped ones too) are blanked: an item inside one is no finding, a digit inside one is no literal, and the
/// items after each are read, on their own lines.
const BLANKED: &str = r###"/* outer /* inner */ const HIDDEN: u8 = 1; */ const SEEN: u8 = 2;
const SPLIT: u8 = LIMIT /* 5 */;
const DOC: &str = r#"# heading const IN_RAW: u8 = 3; "#; const AFTER_RAW: u8 = 4;
const BYTES: &[u8] = br"\"; const AFTER_BYTES: u8 = 5;
const C: &core::ffi::CStr = cr"\"; const AFTER_C: u8 = 6;
const EMPTY: &str = ""; const AFTER_EMPTY: u8 = 7;
const QUOTE: &str = "\""; const AFTER_QUOTE: u8 = 8;
const TABS: &[char] = &['\t','1'];
const ONE: char = '1';
const DQ: char = '"'; const AFTER_DQ: u8 = 9;
fn r#match() {} const AFTER_IDENT: u8 = 10;
"###;

#[test]
fn lint_constants_blanks_comments_strings_and_characters() {
    check_scan(
        BLANKED,
        &[
            (1, "SEEN"),
            (3, "AFTER_RAW"),
            (4, "AFTER_BYTES"),
            (5, "AFTER_C"),
            (6, "AFTER_EMPTY"),
            (7, "AFTER_QUOTE"),
            (10, "AFTER_DQ"),
            (11, "AFTER_IDENT"),
        ],
    );
}

negative_control!(
    lint_constants_blanks_comments_strings_and_characters,
    "with SPLIT's comment turned into code, line 2 adds 5, a finding the check must report",
    expected = "the lint's findings are not the expected ones",
    check_scan(
        &BLANKED.replace("/* 5 */", "+ 5"),
        &[
            (1, "SEEN"),
            (3, "AFTER_RAW"),
            (4, "AFTER_BYTES"),
            (5, "AFTER_C"),
            (6, "AFTER_EMPTY"),
            (7, "AFTER_QUOTE"),
            (10, "AFTER_DQ"),
            (11, "AFTER_IDENT"),
        ]
    )
);

/// An escaped character literal is blanked to its closing quote: the digit in `'\u{7}'` is no literal, and the item
/// after it is read.
const ESCAPED: &str = r"const BELL: char = '\u{7}'; const N: u8 = 3;";

#[test]
fn lint_constants_blanks_an_escaped_character() {
    check_scan(ESCAPED, &[(1, "N")]);
}

negative_control!(
    lint_constants_blanks_an_escaped_character,
    "with BELL's code point written as a number, line 1 holds two findings, so the check must fail",
    expected = "the lint's findings are not the expected ones",
    check_scan(&ESCAPED.replace(r"'\u{7}'", "7 as char"), &[(1, "N")])
);

/// A block comment or a string left open runs to the end of the file: what follows it is no finding.
const OPEN_COMMENT: &str = "const A: u8 = 1;\n/* const TAIL: u8 = 2;";
const OPEN_STRING: &str = "const A: u8 = 1;\nconst S: &str = \"const TAIL: u8 = 2;";

#[test]
fn lint_constants_an_open_comment_or_string_runs_to_the_end() {
    check_scan(OPEN_COMMENT, &[(1, "A")]);
    check_scan(OPEN_STRING, &[(1, "A")]);
}

negative_control!(
    lint_constants_an_open_comment_or_string_runs_to_the_end,
    "with the comment's opening removed, line 2 is an item holding 2, a finding the check must report",
    expected = "the lint's findings are not the expected ones",
    check_scan(&OPEN_COMMENT.replace("/* ", ""), &[(1, "A")])
);
