//! QA tests for TASK-M0-25, written from REQ-VAL-153 ("every qa test in `crates/validation` and `crates/prin` merged
//! before TASK-M0-22 must have a registered negative control [...]; a subprocess body a test spawns must not be a
//! `#[test]`"), R-210 ("the three qa child-mode helpers [...] move there") and the task's review line ("the moved
//! child bodies keep what the parent tests read"). Each test registers a negative control (R-176, R-199).
//!
//! R-909 retired the two inventories that paired each merged qa test of validation and prin with its control and found
//! no child body among the tests: the CI step `cargo xtask controls --list` fails on any test without a control, a
//! child body made a `#[test]` again, with no control of its own, among them. What stays is the moved child body's
//! check.

use std::process::Command;
use validation::negative_control;
use validation::spawn::Spawn;

/// The first failing draw the `failing_property` body prints, run with `PROPTEST_RNG_SEED=seed`.
fn first_failing_draw(seed: &str) -> u32 {
    let o = Command::new(env!("CARGO_BIN_EXE_qa_child"))
        .arg("failing_property")
        .env("PROPTEST_RNG_SEED", seed)
        .timed_output()
        .expect("run qa_child");
    let t = format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    assert!(!o.status.success(), "the failing property passed: {t}");
    let at = t
        .find("QA_FIRST_DRAW Some(")
        .unwrap_or_else(|| panic!("no failing draw printed: {t}"));
    t[at + "QA_FIRST_DRAW Some(".len()..]
        .split(')')
        .next()
        .and_then(|d| d.parse().ok())
        .unwrap_or_else(|| panic!("the failing draw is not a u32: {t}"))
}

/// The property moved from `qa_TASK-M0-04.rs` fails exactly on draws with `x % 7 == 3`.
fn check_fails_as_moved(draw: u32) {
    assert_eq!(
        draw % 7,
        3,
        "the reported failing draw {draw} is not one the moved property (x % 7 == 3) fails on"
    );
}

/// R-210 and the task's review line: the moved `failing_property` body keeps what its parent test reads. The first
/// failing draw it prints is one the property it was moved from fails on (`x % 7 == 3`, one draw in seven), so the
/// parent's seed test reproduces a case reached after passing draws, not merely the first draw.
#[test]
fn qa_m0_25_moved_failing_property_reports_a_draw_the_property_fails_on() {
    for seed in ["1", "2", "3", "12345", "18446744073709551615"] {
        check_fails_as_moved(first_failing_draw(seed));
    }
}

negative_control!(
    qa_m0_25_moved_failing_property_reports_a_draw_the_property_fails_on,
    "the draw after the reported one is not 3 mod 7, so the check must fail on it",
    expected = "is not one the moved property (x % 7 == 3) fails on",
    check_fails_as_moved(first_failing_draw("1").wrapping_add(1))
);
