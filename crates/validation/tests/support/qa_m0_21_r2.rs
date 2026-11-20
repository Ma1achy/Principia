//! The check `qa_TASK-M0-21_r2.rs` shares with its control in `qa_TASK-M0-21_controls.rs`, so the control runs the
//! check it controls, not a copy of it (REQ-VAL-157; R-215). A `tests/*.rs` file is a crate of its own, so each
//! includes this file with `#[path]` beside `qa_m0_21_fixture.rs`; every item here is used by both.

use super::qa_m0_21_fixture::Verdict;

/// `qa_binary_only_crate_is_checked_and_passes`'s check.
pub fn check_bin_only_passes(v: &Verdict) {
    assert!(
        v.ok,
        "a binary-only crate whose one test has a discriminating control failed the command:\n{}",
        v.all()
    );
}
