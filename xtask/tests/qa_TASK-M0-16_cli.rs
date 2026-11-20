//! qa's test for TASK-M0-16's acceptance command itself: `cargo xtask lint vocab` (REQ-SYS-002, REQ-SYS-003) is a
//! command of the xtask binary, run on this workspace, which exits 0 and reports the lint's clean result. The `ci`
//! registry calls the lint's function directly, so only this test runs the binary's `lint vocab` dispatch.

use std::process::{Command, Output};

use validation::negative_control;
use validation::spawn::Spawn;

/// The xtask binary run with `args` on this workspace.
fn xtask(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(args)
        .timed_output()
        .expect("qa16: xtask ran")
}

/// The binary run with `args` exits 0 and prints the vocabulary lint's clean report on this (clean) workspace.
fn check_lint_vocab_command(args: &[&str]) {
    let out = xtask(args);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success() && stdout.contains("xtask lint vocab: no retired term"),
        "qa16: `xtask {}` did not run the vocabulary lint to a clean pass:\nstdout:\n{stdout}\nstderr:\n{stderr}",
        args.join(" ")
    );
}

#[test]
fn qa_cargo_xtask_lint_vocab_runs_the_lint() {
    check_lint_vocab_command(&["lint", "vocab"]);
}

negative_control!(
    qa_cargo_xtask_lint_vocab_runs_the_lint,
    "a command that is not `lint vocab` required to run the vocabulary lint",
    expected = "did not run the vocabulary lint to a clean pass",
    check_lint_vocab_command(&["lint", "vocabulary"])
);
