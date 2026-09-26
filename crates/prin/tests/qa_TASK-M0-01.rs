//! QA tests for TASK-M0-01: `crates/prin/` is a binary stub answering `prin --help` (task deliverables).
#![allow(non_snake_case)]

use std::process::Command;

fn prin(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_prin")).args(args).output().expect("run prin")
}

#[test]
fn qa_prin_help_succeeds_and_prints_usage() {
    let out = prin(&["--help"]);
    assert!(out.status.success(), "prin --help fails");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.to_lowercase().contains("usage"), "prin --help prints no usage:\n{text}");
}

#[test]
fn qa_prin_refuses_an_unknown_argument() {
    // Control: the binary distinguishes --help from anything else, so the test above can fail.
    let out = prin(&["--qa-no-such-flag"]);
    assert!(!out.status.success(), "prin accepts an unknown flag");
}
