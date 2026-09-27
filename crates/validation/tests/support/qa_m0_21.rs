//! The checks `qa_TASK-M0-21.rs` shares with its controls in `qa_TASK-M0-21_controls.rs`, so each control runs the
//! check it controls, not a copy of it (REQ-VAL-157; R-215). A `tests/*.rs` file is a crate of its own, so each
//! includes this file with `#[path]` beside `qa_m0_21_fixture.rs`; every item here is used by both.

use super::qa_m0_21_fixture::{cargo, target_dir, Copy, Verdict};
use std::process::Command;
use validation::spawn::Spawn;

/// `cargo test --tests --no-fail-fast` on `copy`, with the extra arguments `features`.
pub fn cargo_test(copy: &Copy, features: &[&str]) -> Verdict {
    Verdict::from(
        Command::new(cargo())
            .args(["test", "--tests", "--no-fail-fast", "--manifest-path"])
            .arg(copy.manifest())
            .args(features)
            .env("CARGO_TARGET_DIR", target_dir())
            .timed_output()
            .expect("run cargo test"),
    )
}

pub fn lacks(text: &str, needle: &str) {
    assert!(!text.contains(needle), "{needle:?} in:\n{text}");
}

/// `qa_leaky_control_beside_a_sound_one_of_the_same_name_fails_naming_the_test`'s check.
pub fn check_leaky_fails(v: &Verdict) {
    assert!(
        !v.ok,
        "a control that leaves `doubles` passing passed the command, because a control of the same name in \
         another target discriminates:\n{}",
        v.all()
    );
}

/// `qa_control_under_another_name_does_not_pair`'s check.
pub fn check_misnamed_fails(v: &Verdict) {
    assert!(
        !v.ok,
        "a test whose only control names another test passed:\n{}",
        v.all()
    );
}

/// `qa_unit_test_in_a_binary_target_pairs_with_its_control_in_tests`'s check.
pub fn check_bin_target_pairs(v: &Verdict) {
    assert!(
        v.ok,
        "the binary's unit test did not pair with its control:\n{}",
        v.all()
    );
}

/// `qa_workspace_pairs_within_each_crate_and_skips_the_featureless_one`'s check.
pub fn check_workspace_fails(v: &Verdict) {
    assert!(
        !v.ok,
        "a test paired with a control in another crate:\n{}",
        v.all()
    );
}
