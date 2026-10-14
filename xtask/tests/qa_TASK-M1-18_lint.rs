//! QA tests for TASK-M1-18's change to `cargo xtask lint constants`, written from REQ-SYS-082 ("No number in
//! crates/{kernel,ledger,engine} that is a register constant may be wrapped … to get past `cargo xtask lint constants`";
//! "cargo xtask lint constants passes") and dd_generation_root §3.8, not from the implementation.
//!
//! The lint now skips test code. REQ-SYS-082's "the lint passes" means something only if that skip reaches no code the
//! crate ships: a numeric `const` the crate compiles outside `cfg(test)` is still a finding when
//! - `#[cfg(test)]` appears only in a comment or a string before it;
//! - it follows a test-only item that ends with `;` (`use`), or whose signature holds `;` or nested braces;
//! - it sits under `#[cfg(not(test))]`, which ships;
//! - it sits in a module file whose name matches a test module declared elsewhere (a `#[cfg(test)] mod tests;` in
//!   `src/a.rs` is `src/a/tests.rs`, never the crate root's `src/tests.rs`).
//!
//! Each test has a registered negative control (R-176).

use std::fs;
use std::path::{Path, PathBuf};

use validation::negative_control;
use xtask::lint_constants::{check, scan};

/// Source whose numeric constants `SHIP_*` are compiled into the crate and `TEST_*` only under `cfg(test)`.
const SOURCE: &str = r##"// #[cfg(test)]
const SHIP_AFTER_COMMENT: u32 = 1;
/* #[cfg(test)] */
const SHIP_AFTER_BLOCK: u32 = 2;
const LABEL: &str = "#[cfg(test)]";
const SHIP_AFTER_STRING: u32 = 3;
#[cfg(test)]
use std::fmt;
const SHIP_AFTER_USE: u32 = 4;
#[cfg(test)]
fn helper() -> [u8; 4] {
    const TEST_IN_FN: u8 = 7;
    if true { [TEST_IN_FN; 4] } else { [0; 4] }
}
const SHIP_AFTER_FN: u32 = 5;
#[cfg(test)]
#[derive(Debug)]
struct Probe {
    a: [u8; 2],
}
const SHIP_AFTER_STRUCT: u32 = 6;
#[cfg(not(test))]
const SHIP_NOT_TEST: u32 = 7;
#[cfg(test)]
const TEST_CONST: u32 = 8;
#[cfg(test)]
mod tests {
    const TEST_IN_MOD: f64 = 0.5;
    mod inner {
        static TEST_STATIC: u8 = 1;
    }
}
pub const SHIP_LAST: f64 = 2.5;
"##;

/// The constants of [`SOURCE`] the crate ships, in order.
const SHIPPED: [&str; 8] = [
    "SHIP_AFTER_COMMENT",
    "SHIP_AFTER_BLOCK",
    "SHIP_AFTER_STRING",
    "SHIP_AFTER_USE",
    "SHIP_AFTER_FN",
    "SHIP_AFTER_STRUCT",
    "SHIP_NOT_TEST",
    "SHIP_LAST",
];

/// The lint's findings in `source` are exactly `want`, by name.
fn check_scan(source: &str, want: &[&str]) {
    let found: Vec<String> = scan(source).into_iter().map(|(_, name)| name).collect();
    assert_eq!(
        found, want,
        "the lint's findings are not the shipped constants"
    );
}

#[test]
fn qa_lint_constants_test_code_skip_reaches_no_shipped_constant() {
    check_scan(SOURCE, &SHIPPED);
}

negative_control!(
    qa_lint_constants_test_code_skip_reaches_no_shipped_constant,
    "a test-only constant is no finding, so expecting it among the shipped ones must fail",
    expected = "the lint's findings are not the shipped constants",
    check_scan(
        "#[cfg(test)]\nconst TEST_CONST: u32 = 8;\nconst SHIP: u32 = 1;\n",
        &["TEST_CONST", "SHIP"]
    )
);

/// A kernel tree: `src/lib.rs` declares `a` and `tests`; `src/a.rs` declares its test module `tests` under
/// `#[cfg(test)]`. `src/tests.rs` is the crate root's module `tests`, declared without `cfg(test)`, so it ships;
/// `src/a/tests.rs` is `a`'s test module.
fn tree(name: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let src = root.join("crates/kernel/src");
    fs::create_dir_all(src.join("a")).expect("tree created");
    for (file, text) in [
        ("lib.rs", "pub mod a;\npub mod tests;\n"),
        ("a.rs", "#[cfg(test)]\nmod tests;\n"),
        ("tests.rs", "pub const SHIP_ROOT_TESTS: u32 = 1;\n"),
        ("a/tests.rs", "const TEST_A: u32 = 2;\n"),
    ] {
        fs::write(src.join(file), text).expect("file written");
    }
    root
}

/// The lint over `root` finds exactly `want`, as `file: name`.
fn check_tree(root: &Path, want: &[&str]) {
    let found: Vec<String> = check(root, &[])
        .expect("lint ran")
        .iter()
        .map(|f| format!("{}: {}", f.path.display(), f.name))
        .collect();
    assert_eq!(
        found, want,
        "the lint's findings are not the shipped module's"
    );
}

#[test]
fn qa_lint_constants_test_module_is_resolved_from_its_declaring_file() {
    check_tree(
        &tree("qa_m1_18_lint_tree"),
        &["crates/kernel/src/tests.rs: SHIP_ROOT_TESTS"],
    );
}

negative_control!(
    qa_lint_constants_test_module_is_resolved_from_its_declaring_file,
    "a's test module is skipped, so expecting its constant as a finding must fail",
    expected = "the lint's findings are not the shipped module's",
    check_tree(
        &tree("qa_m1_18_lint_tree_control"),
        &[
            "crates/kernel/src/a/tests.rs: TEST_A",
            "crates/kernel/src/tests.rs: SHIP_ROOT_TESTS",
        ],
    )
);
