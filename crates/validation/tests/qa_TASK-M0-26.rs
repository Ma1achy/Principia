//! QA tests for TASK-M0-26, written from the requirements it closes:
//! - REQ-VAL-155: "Every test that spawns a child process must do so through one shared helper in `crates/validation`
//!   that waits at most the calibrated timeout (REQ-VAL-156) and, on timeout, kills the child and fails naming it
//!   (R-214); a subprocess body is not a `#[test]` (R-210), which includes
//!   `qa_r206_harness_opens_the_selected_backend`'s child path (R-213)." Verify: "a child that sleeps past a short test
//!   timeout is killed and the helper's error names it, and the child process no longer exists afterwards; a child
//!   that exits in time returns its output; [...] `cargo test -- --list` lists no child path of `qa_R-206.rs`".
//! - REQ-VAL-156: "300 s provisional (R-217)".
//! - REQ-VAL-154: "every `negative_control!` call in the workspace carries an expected message (the macro requires
//!   it)", checked on the fixture `fixtures/qa_m0_26/old_form`, a control in R-199's form without the message.
//!
//! Each test registers a negative control (R-176, R-199, R-212). Children are spawned only through the helper.

use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use validation::negative_control;
use validation::spawn::{Spawn, GRACE, TIMEOUT};

/// A slack for process start-up and reaping on a loaded machine, on top of the timeout the helper is given.
const SLACK: Duration = Duration::from_secs(5);

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// The workspace's target directory, where `validation` is already built.
fn target_dir() -> PathBuf {
    Path::new(env!("CARGO_TARGET_TMPDIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

/// A fresh scratch directory under the target's tmp dir.
fn scratch(tag: &str) -> PathBuf {
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "qa_m0_26-{tag}-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Whether `pid` names a live process (`kill -0`).
fn alive(pid: &str) -> bool {
    Command::new("kill")
        .args(["-0", pid])
        .timed_output()
        .expect("kill ran")
        .status
        .success()
}

/// The pid a child wrote to `file`, waiting briefly for the write.
fn read_pid(file: &Path) -> String {
    let started = Instant::now();
    loop {
        if let Ok(pid) = std::fs::read_to_string(file) {
            let pid = pid.trim().to_owned();
            if !pid.is_empty() {
                return pid;
            }
        }
        assert!(
            started.elapsed() < SLACK,
            "the child never wrote its pid to {file:?}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// REQ-VAL-156: the helper's timeout is 300 s, provisional (R-217).

fn check_timeout_is(want: Duration) {
    assert_eq!(
        TIMEOUT, want,
        "the helper's timeout is not the provisional REQ-VAL-156 value"
    );
}

#[test]
fn qa_m0_26_the_timeout_is_the_provisional_300_s() {
    check_timeout_is(Duration::from_secs(300));
}

negative_control!(
    qa_m0_26_the_timeout_is_the_provisional_300_s,
    "a timeout of 299 s, required to be the helper's",
    expected = "the helper's timeout is not the provisional REQ-VAL-156 value",
    check_timeout_is(Duration::from_secs(299))
);

// ---------------------------------------------------------------------------------------------------------------------
// REQ-VAL-155: a child past the timeout is killed, the error names it, the child no longer exists.

/// Runs `sh -c <script> qa_m0_26_sleeper` under `timeout`; the script writes its own pid (the `exec`'d process) to a
/// file first. Returns the helper's result, the time it took, and the child's pid.
fn run_sleeper(timeout: Duration, rest: &str) -> (std::io::Result<Output>, Duration, String) {
    let dir = scratch("sleeper");
    let pid_file = dir.join("pid");
    let script = format!("echo $$ > '{}'; exec {rest}", pid_file.display());
    let started = Instant::now();
    let result = Command::new("sh")
        .args(["-c", &script, "qa_m0_26_sleeper"])
        .output_within(timeout);
    let took = started.elapsed();
    let pid = read_pid(&pid_file);
    let _ = std::fs::remove_dir_all(&dir);
    (result, took, pid)
}

/// The child running `rest` under `timeout` is killed: the error is a timeout that names the child, the helper
/// returned within the timeout and R-217's grace after SIGTERM (plus start-up slack), and the child's pid no longer
/// names a process.
fn check_killed_named_and_gone(timeout: Duration, rest: &str) {
    let (result, took, pid) = run_sleeper(timeout, rest);
    let err = result.expect_err("the child was not killed at the timeout");
    assert_eq!(err.kind(), ErrorKind::TimedOut, "not a timeout: {err}");
    let message = err.to_string();
    assert!(
        message.contains("qa_m0_26_sleeper"),
        "the error does not name the child: {message}"
    );
    assert!(
        took < timeout + GRACE + SLACK,
        "the helper waited {took:?}, past the {timeout:?} timeout and the {GRACE:?} grace"
    );
    assert!(!alive(&pid), "the timed-out child {pid} still exists");
}

#[test]
fn qa_m0_26_a_child_past_the_timeout_is_killed_named_and_gone() {
    check_killed_named_and_gone(Duration::from_secs(1), "sleep 37");
}

negative_control!(
    qa_m0_26_a_child_past_the_timeout_is_killed_named_and_gone,
    "a child that exits well within the timeout, required to be killed",
    expected = "the child was not killed at the timeout",
    check_killed_named_and_gone(Duration::from_secs(10), "true")
);

/// A child that ignores SIGTERM is still gone after the timeout.
#[test]
fn qa_m0_26_a_child_ignoring_sigterm_is_still_killed() {
    check_killed_named_and_gone(
        Duration::from_secs(1),
        "sh -c 'trap \"\" TERM; sleep 37 & wait'",
    );
}

negative_control!(
    qa_m0_26_a_child_ignoring_sigterm_is_still_killed,
    "the same SIGTERM-ignoring child, exiting in time, required to be killed",
    expected = "the child was not killed at the timeout",
    check_killed_named_and_gone(Duration::from_secs(10), "sh -c 'trap \"\" TERM; true'")
);

// ---------------------------------------------------------------------------------------------------------------------
// REQ-VAL-155: a child that exits in time returns its output — all of it, from both streams, and its status.

/// 1 MiB, more than a pipe buffer holds, on each stream: stderr written first, so a helper that read stdout to its
/// end before stderr would deadlock with the child.
const BIG: usize = 1 << 20;

/// A child writing `err_bytes` to stderr, then `out_bytes` to stdout, then exiting 7, returns all of it within the
/// timeout.
fn check_returns_all_output(timeout: Duration, out_bytes: usize, err_bytes: usize) {
    let script = format!("yes e | head -c {err_bytes} >&2; yes o | head -c {out_bytes}; exit 7");
    let output = Command::new("sh")
        .args(["-c", &script])
        .output_within(timeout)
        .unwrap_or_else(|e| panic!("the child in time did not return its output: {e}"));
    assert_eq!(
        (output.stdout.len(), output.stderr.len()),
        (BIG, BIG),
        "the child's output was not returned whole"
    );
    assert!(
        output.stdout.chunks(2).all(|c| c[0] == b'o'),
        "stdout mixed"
    );
    assert!(
        output.stderr.chunks(2).all(|c| c[0] == b'e'),
        "stderr mixed"
    );
    assert_eq!(output.status.code(), Some(7), "the exit status was lost");
}

#[test]
fn qa_m0_26_a_child_in_time_returns_all_its_output() {
    check_returns_all_output(Duration::from_secs(60), BIG, BIG);
}

negative_control!(
    qa_m0_26_a_child_in_time_returns_all_its_output,
    "a child that writes half the stdout, required to return 1 MiB of it",
    expected = "the child's output was not returned whole",
    check_returns_all_output(Duration::from_secs(60), BIG / 2, BIG)
);

// ---------------------------------------------------------------------------------------------------------------------
// REQ-VAL-155: "waits at most the timeout", also when the child exits but a grandchild keeps its output open.

/// `sh` backgrounds a sleeping grandchild that inherits stdout, then exits at once. The helper, given `timeout`,
/// returns within `bound`; the grandchild is killed afterwards.
fn check_bounded_by_the_timeout(timeout: Duration, bound: Duration) {
    let dir = scratch("grandchild");
    let pid_file = dir.join("pid");
    let script = format!("sleep 37 & echo $! > '{}'; echo x", pid_file.display());
    let started = Instant::now();
    let result = Command::new("sh")
        .args(["-c", &script, "qa_m0_26_parent"])
        .output_within(timeout);
    let took = started.elapsed();
    let grandchild = read_pid(&pid_file);
    let _ = Command::new("kill").arg(&grandchild).timed_output();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        took < bound,
        "the helper waited {took:?} on a grandchild holding the output, past the bound {bound:?} (timeout {timeout:?})"
    );
    if let Err(e) = result {
        assert!(
            e.to_string().contains("qa_m0_26_parent"),
            "the error does not name the child: {e}"
        );
    }
}

#[test]
fn qa_m0_26_a_grandchild_holding_the_output_does_not_outlast_the_timeout() {
    check_bounded_by_the_timeout(
        Duration::from_millis(500),
        Duration::from_millis(500) + SLACK,
    );
}

negative_control!(
    qa_m0_26_a_grandchild_holding_the_output_does_not_outlast_the_timeout,
    "a 3 s timeout, which the helper waits out on the open output, required to return within 1 s",
    expected = "on a grandchild holding the output, past the bound",
    check_bounded_by_the_timeout(Duration::from_secs(3), Duration::from_secs(1))
);

// ---------------------------------------------------------------------------------------------------------------------
// REQ-VAL-155, R-213: `qa_r206_harness_opens_the_selected_backend` is a test only; its child path is the `qa_child`
// bin's.

fn cargo() -> Command {
    let mut cmd = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned()));
    cmd.current_dir(root())
        .env("CARGO_TARGET_DIR", target_dir());
    cmd
}

/// The names `cargo test -p validation --test qa_R-206 -- --list` lists, with or without `controls`.
fn r206_listing(controls: bool) -> Vec<String> {
    let mut cmd = cargo();
    cmd.args(["test", "-q", "-p", "validation", "--test", "qa_R-206"]);
    if controls {
        cmd.args(["--features", "controls"]);
    }
    let o = cmd
        .args(["--", "--list", "--format", "terse"])
        .timed_output()
        .expect("run cargo test --list");
    let out = String::from_utf8_lossy(&o.stdout).into_owned();
    assert!(
        o.status.success(),
        "cargo test --list failed:\n{out}\n{}",
        String::from_utf8_lossy(&o.stderr)
    );
    let mut names: Vec<String> = out
        .lines()
        .filter_map(|l| l.strip_suffix(": test"))
        .map(str::to_owned)
        .collect();
    names.sort();
    names
}

/// `qa_R-206.rs`'s five tests, from TASK-M0-25's list (none of them a child body).
const R206_TESTS: [&str; 5] = [
    "qa_r206_harness_explicit_value_overrides_the_default",
    "qa_r206_harness_logs_the_backend_it_ran_on",
    "qa_r206_harness_opens_the_selected_backend",
    "qa_r206_unknown_value_is_still_an_error",
    "qa_r206_unset_defaults_by_platform_and_explicit_overrides",
];

fn check_lists_only_the_tests(names: &[String]) {
    assert_eq!(
        names, R206_TESTS,
        "qa_R-206.rs lists more than its five tests"
    );
}

#[test]
fn qa_m0_26_r206_lists_its_five_tests_and_no_child_path() {
    check_lists_only_the_tests(&r206_listing(false));
}

negative_control!(
    qa_m0_26_r206_lists_its_five_tests_and_no_child_path,
    "the listing under `controls`, which adds a control per test, required to be the five tests alone",
    expected = "qa_R-206.rs lists more than its five tests",
    check_lists_only_the_tests(&r206_listing(true))
);

/// The marker the child path prints.
const OPENED: &str = "QA_R206_OPENED backend=";

/// The child path is gone from the test: run as the old parent did (the test, filtered exactly, with the old
/// `QA_R206_CHILD` switch and `--nocapture`), the test binary prints no marker.
fn test_binary_as_child() -> String {
    let o = cargo()
        .args([
            "test",
            "-q",
            "-p",
            "validation",
            "--test",
            "qa_R-206",
            "--",
            "qa_r206_harness_opens_the_selected_backend",
            "--exact",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("QA_R206_CHILD", "1")
        .timed_output()
        .expect("run the qa_R-206 test binary");
    let out = String::from_utf8_lossy(&o.stdout).into_owned();
    assert!(
        o.status.success(),
        "the test did not open the harness and pass:\n{out}\n{}",
        String::from_utf8_lossy(&o.stderr)
    );
    out
}

/// The `qa_child` body R-213 moved it to.
fn qa_child_r206() -> String {
    let o = Command::new(env!("CARGO_BIN_EXE_qa_child"))
        .arg("r206_opened")
        .timed_output()
        .expect("run qa_child r206_opened");
    let out = String::from_utf8_lossy(&o.stdout).into_owned();
    assert!(
        o.status.success(),
        "qa_child r206_opened failed:\n{out}\n{}",
        String::from_utf8_lossy(&o.stderr)
    );
    out
}

fn check_no_child_marker(stdout: &str) {
    assert!(
        !stdout.contains(OPENED),
        "the run printed the child path's marker:\n{stdout}"
    );
}

#[test]
fn qa_m0_26_r206_test_has_no_child_path_and_qa_child_has_it() {
    check_no_child_marker(&test_binary_as_child());
    assert!(
        qa_child_r206().contains(OPENED),
        "qa_child r206_opened printed no marker"
    );
}

negative_control!(
    qa_m0_26_r206_test_has_no_child_path_and_qa_child_has_it,
    "the qa_child body's own output, required to carry no child marker",
    expected = "the run printed the child path's marker",
    check_no_child_marker(&qa_child_r206())
);

// ---------------------------------------------------------------------------------------------------------------------
// REQ-VAL-154: the macro requires the expected message.

/// Copies `fixtures/qa_m0_26/old_form` to a scratch dir, making its validation path absolute; `add_expected` adds
/// R-212's `expected = …` to the control. Returns `cargo test --no-run --features controls` on the copy.
fn build_old_form(add_expected: bool) -> Output {
    let fixture = root().join("fixtures/qa_m0_26/old_form");
    let copy = scratch("old_form");
    for file in ["src/lib.rs", "tests/double.rs"] {
        std::fs::create_dir_all(copy.join(file).parent().unwrap()).unwrap();
        std::fs::copy(fixture.join(file), copy.join(file)).unwrap();
    }
    let validation = root().join("crates/validation");
    let manifest = std::fs::read_to_string(fixture.join("Cargo.toml"))
        .unwrap()
        .replace("../../../crates/validation", validation.to_str().unwrap());
    std::fs::write(copy.join("Cargo.toml"), manifest).unwrap();
    std::fs::copy(root().join("Cargo.lock"), copy.join("Cargo.lock")).unwrap();
    if add_expected {
        let test = copy.join("tests/double.rs");
        let body = std::fs::read_to_string(&test).unwrap();
        let description = "\"a doubling that adds one must fail the check\",";
        assert!(body.contains(description), "the fixture changed");
        let body = body.replace(
            description,
            &format!("{description}\n    expected = \"not the double of 3\","),
        );
        std::fs::write(&test, body).unwrap();
    }
    let o = cargo()
        .args([
            "test",
            "--no-run",
            "--features",
            "controls",
            "--manifest-path",
        ])
        .arg(copy.join("Cargo.toml"))
        .timed_output()
        .expect("run cargo test --no-run on the fixture");
    let _ = std::fs::remove_dir_all(&copy);
    o
}

/// The build failed, at the `negative_control!` call: rustc's macro-matching error, in the fixture's test file.
fn check_refused_by_the_macro(o: &Output) {
    let stderr = String::from_utf8_lossy(&o.stderr);
    assert!(
        !o.status.success(),
        "a control without an expected message compiled:\n{stderr}"
    );
    assert!(
        stderr.contains("no rules expected") && stderr.contains("tests/double.rs"),
        "the build failed, but not at negative_control!:\n{stderr}"
    );
}

#[test]
fn qa_m0_26_a_control_without_an_expected_message_does_not_compile() {
    check_refused_by_the_macro(&build_old_form(false));
}

negative_control!(
    qa_m0_26_a_control_without_an_expected_message_does_not_compile,
    "the same fixture with the expected message added, which compiles, required to be refused",
    expected = "a control without an expected message compiled",
    check_refused_by_the_macro(&build_old_form(true))
);
