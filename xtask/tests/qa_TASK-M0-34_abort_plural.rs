//! qa's tests for TASK-M0-34 (REQ-VAL-167, R-236, R-244): the finding of a control that did not make its test fail
//! carries that control's own output, also for a target run after one that died without libtest's closing report.
//!
//! `qa_TASK-M0-34_abort.rs` covers a target of one test (`running 1 test`) after the aborted one. Here the target
//! after the aborted one runs several tests (`running 3 tests`, libtest's plural form) and two of them fail, so
//! both of their outputs must be kept, each its own, and none may run on into libtest's closing report.
//! `STDOUT` has libtest's plain-text layout (as `cargo test --no-fail-fast` prints it): a target `b_abort` that
//! reports one `FAILED` and then aborts (no `failures:` list, no `test result:`), then `c_many` with three tests.
//! Each check has its negative control (R-176): the same check for the other test's marker.

use xtask::controls::parse_outputs;

const STDOUT: &str = "
running 2 tests
test aborted_negative_control ... FAILED

running 3 tests
test passes_fine ... ok
test first_negative_control ... FAILED
test second_negative_control ... FAILED

failures:

---- first_negative_control stdout ----

thread 'first_negative_control' panicked at tests/c_many.rs:1:39:
FIRST_MARKER
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

---- second_negative_control stdout ----

thread 'second_negative_control' panicked at tests/c_many.rs:2:39:
SECOND_MARKER


failures:
    first_negative_control
    second_negative_control

test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

";

/// The one output `parse_outputs` keeps for `test` holds `marker` and nothing of libtest's closing report.
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
        !kept.contains("test result:") && !kept.contains("    second_negative_control"),
        "the output kept for `{test}` runs past its target's list of failures:\n{kept}"
    );
}

/// The first failing test of a many-test target after the aborted one: its output is kept, and is its own.
#[test]
fn qa_m034_first_output_of_a_plural_target_after_an_abort_is_kept() {
    check("first_negative_control", "FIRST_MARKER");
}

/// The second failing test of that target: its output is kept, and stops at libtest's list of failures.
#[test]
fn qa_m034_second_output_of_a_plural_target_after_an_abort_is_kept() {
    check("second_negative_control", "SECOND_MARKER");
}

validation::negative_control!(
    qa_m034_first_output_of_a_plural_target_after_an_abort_is_kept,
    "the second test's marker, which the first test's output must not hold",
    expected = "the output kept for `first_negative_control` does not hold SECOND_MARKER",
    check("first_negative_control", "SECOND_MARKER")
);

validation::negative_control!(
    qa_m034_second_output_of_a_plural_target_after_an_abort_is_kept,
    "the first test's marker, which the second test's output must not hold",
    expected = "the output kept for `second_negative_control` does not hold FIRST_MARKER",
    check("second_negative_control", "FIRST_MARKER")
);
