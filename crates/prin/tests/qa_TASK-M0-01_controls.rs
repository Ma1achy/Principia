//! Negative controls for qa's tests in `qa_TASK-M0-01.rs` (REQ-VAL-153; R-199, R-209): each runs its test's check,
//! copied here, on the other argument, which the check must reject, so the test can fail (philosophy §4.4).
#![cfg(feature = "controls")]

use std::process::{Command, Output};
use validation::negative_control;
use validation::spawn::Spawn;

fn prin(arg: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_prin"))
        .arg(arg)
        .timed_output()
        .expect("run prin")
}

negative_control!(
    qa_prin_help_succeeds_and_prints_usage,
    "an unknown flag fails, so the --help check must fail on it",
    expected = "prin --help fails",
    assert!(
        prin("--qa-no-such-flag").status.success(),
        "prin --help fails"
    )
);

negative_control!(
    qa_prin_refuses_an_unknown_argument,
    "--help succeeds, so the refusal check must fail on it",
    expected = "prin accepts an unknown flag",
    assert!(
        !prin("--help").status.success(),
        "prin accepts an unknown flag"
    )
);
