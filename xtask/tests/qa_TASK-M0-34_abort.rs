//! qa's tests for TASK-M0-34 (REQ-VAL-167, R-236, R-244): the finding of a control that did not make its test fail
//! carries that control's own output. `cargo xtask controls` runs `cargo test --no-fail-fast`, so one test target
//! can end without libtest's closing report (a test aborts the process: `std::process::abort`, a stack overflow)
//! and the next targets still run. The output of each control in those later targets must still be read, and be
//! that control's alone.
//!
//! `STDOUT` is the stdout of `cargo test --no-fail-fast` (rustc 1.9x, thread ids removed) on a scratch crate with
//! four test targets: `a_silent` (a failing test that prints nothing), `b_abort` (one test fails, then another
//! calls `std::process::abort()`), `c_after` and `d_later` (one failing test each, panicking with a marker).
//! Each check has its negative control (R-176): the same check for another test's marker.

use xtask::controls::parse_outputs;

const STDOUT: &str = "
running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 1 test
test silent_negative_control ... FAILED

failures:

failures:
    silent_negative_control

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 2 tests
test fails_negative_control ... FAILED

running 1 test
test after_negative_control ... FAILED

failures:

---- after_negative_control stdout ----

thread 'after_negative_control' panicked at tests/c_after.rs:1:39:
AFTER_MARKER
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    after_negative_control

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 1 test
test later_negative_control ... FAILED

failures:

---- later_negative_control stdout ----

thread 'later_negative_control' panicked at tests/d_later.rs:1:39:
LATER_MARKER
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    later_negative_control

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
";

/// The one output `parse_outputs` keeps for `test` holds `marker` and nothing of libtest's closing report or of
/// another target.
fn check(test: &str, marker: &str) {
    let outputs = parse_outputs(STDOUT);
    let Some(kept) = outputs.get(test) else {
        panic!("no output kept for `{test}`: {outputs:#?}");
    };
    assert_eq!(
        kept.len(),
        1,
        "`{test}` has {} outputs: {kept:#?}",
        kept.len()
    );
    let kept = &kept[0];
    assert!(
        kept.contains(marker),
        "the output kept for `{test}` does not hold {marker}:\n{kept}"
    );
    assert!(
        !kept.contains("test result:") && !kept.contains("running 1 test"),
        "the output kept for `{test}` runs past its target's list of failures:\n{kept}"
    );
}

/// The target right after the aborted one: its control's output is kept, and stops at its own list of failures.
#[test]
fn qa_m034_output_after_an_aborted_target_is_its_own() {
    check("after_negative_control", "AFTER_MARKER");
}

/// A later target: its control's output is still found.
#[test]
fn qa_m034_output_two_targets_after_an_aborted_target_is_kept() {
    check("later_negative_control", "LATER_MARKER");
}

validation::negative_control!(
    qa_m034_output_after_an_aborted_target_is_its_own,
    "the later target's marker, which the output of the target after the aborted one must not hold",
    expected = "the output kept for `after_negative_control` does not hold LATER_MARKER",
    check("after_negative_control", "LATER_MARKER")
);

validation::negative_control!(
    qa_m034_output_two_targets_after_an_aborted_target_is_kept,
    "the earlier target's marker, which the later target's output must not hold",
    expected = "the output kept for `later_negative_control` does not hold AFTER_MARKER",
    check("later_negative_control", "AFTER_MARKER")
);
