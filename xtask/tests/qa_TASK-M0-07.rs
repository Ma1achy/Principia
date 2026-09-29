//! qa's test for TASK-M0-07's `cargo xtask codegen` (TASK-M0-07 Deliverables; R-241: "`cargo xtask codegen`
//! (regenerates the checked-in generated files), depending on `ledger` directly"). The live layout table is empty
//! until §3 is transcribed (R-242), so `codegen` on this workspace must succeed and change no file of it; the
//! refusal of an incomplete entry is `ledger`'s tests' (`crates/ledger/tests/qa_TASK-M0-07.rs`).

use std::path::Path;
use std::process::Command;

use validation::negative_control;
use validation::spawn::Spawn;

fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

/// `git status --porcelain` of the workspace.
fn git_status() -> String {
    let out = Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=all"])
        .current_dir(workspace_root())
        .timed_output()
        .expect("qa: run git status");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// `xtask <args>` from the workspace root succeeds and leaves the workspace's files as they were.
fn check_codegen_runs_clean(args: &[&str]) {
    let before = git_status();
    let out = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(args)
        .current_dir(workspace_root())
        .timed_output()
        .expect("qa: run xtask");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.status.success(),
        "qa: `xtask {}` failed: {text}",
        args.join(" ")
    );
    assert!(
        text.contains("codegen"),
        "qa: `xtask {}` says nothing of codegen: {text}",
        args.join(" ")
    );
    assert_eq!(
        before,
        git_status(),
        "qa: `xtask {}` changed the workspace although the layout table is empty",
        args.join(" ")
    );
}

#[test]
fn qa_m007_xtask_codegen_runs_on_the_empty_layout() {
    check_codegen_runs_clean(&["codegen"]);
}

negative_control!(
    qa_m007_xtask_codegen_runs_on_the_empty_layout,
    "an unrecognised codegen argument makes xtask fail, so the check must fail on it",
    expected = "failed",
    check_codegen_runs_clean(&["codegen", "--qa-no-such-flag"])
);
