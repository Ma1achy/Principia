//! QA tests for TASK-M0-21, round 2, written from REQ-VAL-147's statement: "run every control under the `controls`
//! feature of each crate that declares it", "pairing by name across all of a crate's test targets" (R-201). Each
//! runs `cargo xtask controls` on a copy of a fixture in `fixtures/controls_qa_m0_21/` and carries an inline negative
//! control (R-176; the registry's own controls for these tests are TASK-M0-22's, R-198).
//!
//! - `bin_only`: a controls crate with a binary target and no library (so nothing for `cargo test --doc` to list).
//!   Its unit test in `src/main.rs` has a discriminating control in `tests/`: the crate is checked and passes.

#[path = "support/qa_m0_21_fixture.rs"]
mod qa_m0_21_fixture;
#[path = "support/qa_m0_21_r2.rs"]
mod qa_m0_21_r2;

use qa_m0_21_fixture::*;
use qa_m0_21_r2::*;

fn has(text: &str, needle: &str) {
    assert!(text.contains(needle), "{needle:?} not in:\n{text}");
}

/// REQ-VAL-147: every crate declaring the feature has its controls run, and a unit test in `src/` pairs with its
/// control in `tests/` (R-201). A crate with only a binary target is such a crate: the command must check it and
/// pass, not stop on it for want of a library (looking for doctests where there can be none).
#[test]
fn qa_binary_only_crate_is_checked_and_passes() {
    let v = controls("bin_only", &[]);
    check_bin_only_passes(&v);
    has(
        &v.stdout,
        "qa_controls_bin_only: 1 test(s), each failed by its control",
    );
    // Control: without its control in `tests/`, the same crate fails naming the unit test, so it was checked, not
    // skipped, and the pass above is the control's.
    let bare = controls("bin_only", &["tests/controls.rs"]);
    assert!(
        !bare.ok,
        "control: the binary's unit test passed with no control:\n{}",
        bare.all()
    );
    has(&bare.stderr, "test `tests::doubles` has no control");
}
