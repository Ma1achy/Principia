//! The helper and checks `qa_TASK-M0-01.rs` shares with its controls in `qa_TASK-M0-01_controls.rs`, so each control
//! runs the check it controls, not a copy of it (REQ-VAL-157; R-215). A `tests/*.rs` file is a crate of its own, so
//! each includes this file with `#[path]`; every item here is used by both.

use std::process::{Command, Output};
use validation::spawn::Spawn;

pub fn prin(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_prin"))
        .args(args)
        .timed_output()
        .expect("run prin")
}

/// `qa_prin_help_succeeds_and_prints_usage`'s check that `prin --help` succeeds.
pub fn check_help_succeeds(out: &Output) {
    assert!(out.status.success(), "prin --help fails");
}

/// `qa_prin_refuses_an_unknown_argument`'s check.
pub fn check_refused(out: &Output) {
    assert!(!out.status.success(), "prin accepts an unknown flag");
}
