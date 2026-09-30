//! QA tests for TASK-M0-06, `golden --list` (the golden runner's listing form in `cargo xtask ci --list`, R-235): it
//! renders nothing, but it still loads every case of every suite with the run's refusals. Written from R-110 ("no
//! re-baselining without a gate decision") and the Deliverables' BASELINES.md rule ("the golden runner refuses a case
//! whose reference changed without an entry naming that decision"): a listing that passed such a case would let
//! `cargo xtask ci --list` report a refused baseline as fine. Each test has a registered negative control (R-176).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};
use validation::negative_control;
use validation::spawn::Spawn;
use xtask::golden;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("qa_m006_list_{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("scratch created");
    dir
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("dir created");
    for entry in fs::read_dir(from).expect("dir read") {
        let entry = entry.expect("entry");
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).expect("copied");
        }
    }
}

fn sha(path: &Path) -> String {
    format!("{:x}", Sha256::digest(fs::read(path).expect("read")))
}

/// A scratch root holding the repo's self-test suite under each name in `suites`, decisions.md, and a BASELINES.md
/// with one row per case naming R-186 and the hash of the reference then on disk.
fn root_with(name: &str, suites: &[&str]) -> PathBuf {
    let root = scratch(name);
    for suite in suites {
        copy_tree(
            &repo_root().join("fixtures/golden/selftest"),
            &root.join("fixtures/golden").join(suite),
        );
    }
    fs::copy(repo_root().join("decisions.md"), root.join("decisions.md")).expect("decisions");
    let mut table = String::from("| case | sha256 | decision |\n|---|---|---|\n");
    for suite in suites {
        for case in ["gradient", "gradient_shifted"] {
            let reference = root.join(format!("fixtures/golden/{suite}/{case}/reference.png"));
            table.push_str(&format!(
                "| `{suite}/{case}` | `{}` | R-186 (qa scratch) |\n",
                sha(&reference)
            ));
        }
    }
    fs::write(root.join("fixtures/golden/BASELINES.md"), table).expect("baselines");
    root
}

/// Replaces `<suite>/gradient`'s reference with `<suite>/gradient_shifted`'s, after BASELINES.md was written: a
/// baseline changed with no row naming a decision for the new image.
fn rebaseline_without_row(root: &Path, suite: &str) {
    let dir = root.join("fixtures/golden").join(suite);
    fs::copy(
        dir.join("gradient_shifted/reference.png"),
        dir.join("gradient/reference.png"),
    )
    .expect("reference replaced");
}

/// `golden --list` on `root` is refused, naming `suite`.
fn check_list_refused(root: &Path, suite: &str) {
    let err = golden::cli(root, &["--list"])
        .expect_err("golden --list passed a baseline changed without its BASELINES.md row");
    assert!(
        err.contains(suite),
        "refused, but not naming {suite}: {err}"
    );
}

// --- A reference changed without its BASELINES.md row fails the listing, as it fails the run -----------------------

#[test]
fn qa_m006_list_refuses_a_rebaseline_without_row() {
    let root = root_with("rebaseline", &["selftest"]);
    rebaseline_without_row(&root, "selftest");
    check_list_refused(&root, "selftest");
}

negative_control!(
    qa_m006_list_refuses_a_rebaseline_without_row,
    "the reference left as its row records it",
    expected = "golden --list passed a baseline changed without its BASELINES.md row",
    check_list_refused(&root_with("rebaseline_ctl", &["selftest"]), "selftest")
);

// --- The listing covers every suite, not only the first ------------------------------------------------------------

#[test]
fn qa_m006_list_refuses_a_rebaseline_in_a_later_suite() {
    let root = root_with("later", &["aaa", "zzz"]);
    rebaseline_without_row(&root, "zzz");
    check_list_refused(&root, "zzz");
}

negative_control!(
    qa_m006_list_refuses_a_rebaseline_in_a_later_suite,
    "both suites left as their rows record them",
    expected = "golden --list passed a baseline changed without its BASELINES.md row",
    check_list_refused(&root_with("later_ctl", &["aaa", "zzz"]), "zzz")
);

// --- The run and the listing agree on a refused baseline ------------------------------------------------------------

/// The full run on `root` is refused too: the listing refuses what the run refuses, no more.
fn check_run_refused(root: &Path, suite: &str) {
    let err = golden::load_suite(root, suite)
        .expect_err("the run loaded a baseline changed without its BASELINES.md row");
    assert!(err.contains("gradient"), "refused for another case: {err}");
}

#[test]
fn qa_m006_list_and_run_agree_on_a_refused_baseline() {
    let root = root_with("agree", &["selftest"]);
    rebaseline_without_row(&root, "selftest");
    check_run_refused(&root, "selftest");
    check_list_refused(&root, "selftest");
}

negative_control!(
    qa_m006_list_and_run_agree_on_a_refused_baseline,
    "the reference left as its row records it",
    expected = "the run loaded a baseline changed without its BASELINES.md row",
    {
        let root = root_with("agree_ctl", &["selftest"]);
        check_run_refused(&root, "selftest");
    }
);

// --- `cargo xtask ci --list` runs golden's listing form, which lists the cases without a device (R-235) -------------

/// The `xtask` binary run with `args` on this repo, with `PRIN_GPU_BACKEND` set to a value that is no backend, so that
/// opening a device fails: whether it passed, and its stdout.
fn xtask_without_device(args: &[&str]) -> (bool, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(args)
        .env(golden::BACKEND_VAR, "none")
        .timed_output()
        .expect("run xtask");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
    )
}

/// The run passed and listed each self-test case: the ci listing form reached the golden cases, rather than passing
/// without loading them.
fn check_lists_golden_cases((passed, stdout): (bool, String)) {
    assert!(passed, "the listing failed without a device: {stdout}");
    for case in ["selftest/gradient:", "selftest/gradient_shifted:"] {
        assert!(
            stdout.contains(case),
            "the listing did not list the golden case {case}: {stdout}"
        );
    }
}

#[test]
fn qa_m006_ci_list_lists_golden_cases_without_a_device() {
    check_lists_golden_cases(xtask_without_device(&["ci", "--list"]));
}

negative_control!(
    qa_m006_ci_list_lists_golden_cases_without_a_device,
    "a listing that passes without reaching the golden cases (`gate --list`)",
    expected = "the listing did not list the golden case",
    check_lists_golden_cases(xtask_without_device(&["gate", "--list"]))
);
