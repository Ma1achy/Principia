//! QA tests for TASK-M0-06, re-check round: the golden runner is reachable where the task's Deliverables put it.
//! `cargo xtask golden <suite>` renders each case of the suite and writes a summary under `target/golden/` (the
//! Deliverables' diff output); `cargo xtask golden --all` is registered in `cargo xtask ci`, run on every commit
//! (R-110: native golden suites on every commit), and its listing form renders nothing (R-235). Each test has a
//! registered negative control (R-176).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::SystemTime;

use validation::negative_control;
use xtask::golden;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

// --- `cargo xtask golden <suite>`, through the binary ---------------------------------------------------------------

/// Runs the xtask binary with `args` and `CARGO_TARGET_DIR` set to a scratch directory: it must pass, and the self-test
/// gradient's summary must be written under `<target>/golden/selftest/gradient/`.
fn check_binary_renders_selftest(name: &str, args: &[&str]) {
    let target = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("qa_m006_bin_{name}"));
    let _ = fs::remove_dir_all(&target);
    fs::create_dir_all(&target).expect("scratch created");
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(args)
        .current_dir(repo_root())
        .env("CARGO_TARGET_DIR", &target)
        .output()
        .expect("xtask ran");
    assert!(
        output.status.success()
            && target
                .join("golden/selftest/gradient/summary.txt")
                .is_file(),
        "xtask {} did not render the self-test suite: {}{}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn qa_m006_binary_golden_selftest_runs() {
    check_binary_renders_selftest("selftest", &["golden", "selftest"]);
}

negative_control!(
    qa_m006_binary_golden_selftest_runs,
    "a suite that does not exist",
    expected = "did not render the self-test suite",
    check_binary_renders_selftest("ctl", &["golden", "no_such_suite"])
);

// --- `cargo xtask ci` holds a golden runner that renders every suite ------------------------------------------------

/// Runs `run` and checks that it passed and (re)wrote the self-test gradient's summary under the golden output
/// directory after it started: the runner rendered the suites, rather than passing without rendering. Checks are
/// serialised, so a test and its control, run side by side, never see each other's summary.
/// Serialises the checks that watch the golden output directory.
static SERIAL: Mutex<()> = Mutex::new(());

fn check_runner_renders(run: fn() -> Result<(), String>) {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let summary = golden::output_dir(&repo_root()).join("selftest/gradient/summary.txt");
    let start = SystemTime::now();
    run().unwrap_or_else(|e| panic!("the golden runner failed: {e}"));
    let written = fs::metadata(&summary)
        .and_then(|m| m.modified())
        .is_ok_and(|t| t >= start);
    assert!(
        written,
        "the ci golden runner did not render the suites: {} not written",
        summary.display()
    );
}

/// The registry's runner named `golden`: its full form and its listing form.
fn golden_runner() -> &'static xtask::ci::Runner {
    xtask::ci::RUNNERS
        .iter()
        .find(|runner| runner.name == "golden")
        .expect("the ci registry holds no `golden` runner")
}

#[test]
fn qa_m006_ci_golden_runner_renders() {
    check_runner_renders(golden_runner().run);
}

negative_control!(
    qa_m006_ci_golden_runner_renders,
    "a runner that passes without rendering",
    expected = "the ci golden runner did not render the suites",
    check_runner_renders(|| Ok(()))
);

// --- Its listing form renders nothing (R-235) ----------------------------------------------------------------------

/// Runs `list` and checks that it passed and wrote no self-test summary after it started: the listing form of the
/// runner (`cargo xtask ci --list`) loads and checks the cases but opens no device and renders nothing.
fn check_runner_lists_without_rendering(list: fn() -> Result<(), String>) {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let summary = golden::output_dir(&repo_root()).join("selftest/gradient/summary.txt");
    let start = SystemTime::now();
    list().unwrap_or_else(|e| panic!("the golden listing failed: {e}"));
    let written = fs::metadata(&summary)
        .and_then(|m| m.modified())
        .is_ok_and(|t| t >= start);
    assert!(
        !written,
        "the ci golden listing rendered the suites: {} written",
        summary.display()
    );
}

#[test]
fn qa_m006_ci_golden_listing_renders_nothing() {
    check_runner_lists_without_rendering(golden_runner().list);
}

negative_control!(
    qa_m006_ci_golden_listing_renders_nothing,
    "the runner's full form in place of its listing form",
    expected = "the ci golden listing rendered the suites",
    check_runner_lists_without_rendering(golden_runner().run)
);
