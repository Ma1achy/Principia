//! qa's tests for TASK-M0-22, written from REQ-VAL-163 and REQ-VAL-007:
//!
//! - "the CI step `cargo xtask controls` fails on a missing or non-tripping control" (REQ-VAL-163): `xtask ci`, the
//!   CI step, is run against a stand-in `cargo` (`CARGO`, which xtask spawns) that reports one crate with the
//!   `controls` feature, and fails when a listed test has no control, or when its control's run passes (leaves the
//!   test passing); it passes when every control trips.
//! - "neither test runs the workspace's controls" (REQ-VAL-163): the listing-only mode, `xtask controls --list`,
//!   which both tests now go through, starts no `cargo test` other than listings, even when a control would leak.
//!
//! The stand-in cargo answers `metadata`, `test ... -- --list` and the controls' run from canned text and logs each
//! call, so nothing is built and these tests stay cheap. Each case has its own directory under the target's tmp.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use validation::spawn::Spawn;

/// One crate, `qa_m022_fake`, declaring the `controls` feature, with no doctest target.
const METADATA: &str = r#"{"packages":[{"name":"qa_m022_fake","features":{"controls":[]},"targets":[{"doctest":false}]}]}"#;

/// A libtest listing of one test and its control.
const LISTED: &str = "qa_m022_t::pairs: test\nqa_m022_t::pairs::negative_control: test\n";

/// A libtest listing of one test with a control and one without.
const LISTED_MISSING: &str =
    "qa_m022_t::pairs: test\nqa_m022_t::pairs::negative_control: test\nqa_m022_t::alone: test\n";

/// The control's run: it panicked with its expected message, so libtest reports the `should_panic` test ok.
const TRIPPED: &str =
    "running 1 test\ntest qa_m022_t::pairs::negative_control - should panic ... ok\n";

/// The control's run: it did not panic, so it leaves its test passing.
const LEAKED: &str =
    "running 1 test\ntest qa_m022_t::pairs::negative_control - should panic ... FAILED\n";

struct Outcome {
    ok: bool,
    out: String,
    /// Every argument list the stand-in cargo was called with, one per line.
    calls: Vec<String>,
}

/// Runs `xtask <args>` with `CARGO` set to a stand-in that prints `listed` for `--list` and `run` for any other
/// `cargo test`, in a directory of its own named `case`.
fn run_with_fake_cargo(case: &str, args: &[&str], listed: &str, run: &str) -> Outcome {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("qa_m022_{case}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("metadata.json"), METADATA).unwrap();
    fs::write(dir.join("listed.txt"), listed).unwrap();
    fs::write(dir.join("run.txt"), run).unwrap();
    let log = dir.join("calls.log");
    let script = format!(
        "#!/bin/sh\nD='{d}'\necho \"$*\" >> \"$D/calls.log\"\ncase \"$1\" in\n  metadata) cat \"$D/metadata.json\"; exit 0;;\nesac\nfor a in \"$@\"; do\n  if [ \"$a\" = \"--list\" ]; then cat \"$D/listed.txt\"; exit 0; fi\ndone\ncat \"$D/run.txt\"\nif grep -q FAILED \"$D/run.txt\"; then exit 101; fi\nexit 0\n",
        d = dir.display()
    );
    let cargo = dir.join("cargo");
    validation::spawn::write_executable(&cargo, script).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(args)
        .env("CARGO", &cargo)
        .timed_output()
        .expect("run xtask");
    let calls = fs::read_to_string(&log)
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect();
    Outcome {
        ok: output.status.success(),
        out: format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
        calls,
    }
}

/// The CI step failed, naming the controls runner and `what` of the test `test`.
fn check_ci_fails_naming(o: &Outcome, test: &str, what: &str) {
    assert!(!o.ok, "xtask ci passed with {test} {what}:\n{}", o.out);
    assert!(
        o.out.contains("controls FAILED"),
        "xtask ci failed, but not in its controls runner:\n{}",
        o.out
    );
    assert!(
        o.out.contains(&format!("test `{test}`")) && o.out.contains(what),
        "xtask ci does not name `{test}` as `{what}`:\n{}",
        o.out
    );
}

/// The CI step passed, having run the controls (a cargo call filtered on `negative_control`).
fn check_ci_passes_running_controls(o: &Outcome) {
    assert!(
        o.ok,
        "xtask ci failed with every control tripping:\n{}",
        o.out
    );
    assert!(
        o.calls
            .iter()
            .any(|call| call.split(' ').any(|a| a == "negative_control")),
        "xtask ci passed without running the controls: {:?}",
        o.calls
    );
}

/// No call of the stand-in cargo other than `metadata` and listings: no control was run.
fn check_no_control_run(o: &Outcome) {
    assert!(
        !o.calls.is_empty(),
        "the stand-in cargo was never called, so the check shows nothing:\n{}",
        o.out
    );
    let runs: Vec<&String> = o
        .calls
        .iter()
        .filter(|call| !call.starts_with("metadata") && !call.split(' ').any(|a| a == "--list"))
        .collect();
    assert!(
        runs.is_empty(),
        "xtask controls --list ran cargo test beyond the listings: {runs:?}"
    );
}

#[test]
fn qa_m022_ci_fails_on_a_control_that_leaves_its_test_passing() {
    let o = run_with_fake_cargo("ci_leaked", &["ci"], LISTED, LEAKED);
    check_ci_fails_naming(&o, "qa_m022_t::pairs", "its control did not make it fail");
}

#[test]
fn qa_m022_ci_fails_on_a_test_without_control() {
    let o = run_with_fake_cargo("ci_missing", &["ci"], LISTED_MISSING, TRIPPED);
    check_ci_fails_naming(&o, "qa_m022_t::alone", "has no control");
}

#[test]
fn qa_m022_ci_passes_when_every_control_trips() {
    check_ci_passes_running_controls(&run_with_fake_cargo("ci_tripped", &["ci"], LISTED, TRIPPED));
}

#[test]
fn qa_m022_list_runs_no_control() {
    let o = run_with_fake_cargo("list_leaked", &["controls", "--list"], LISTED, LEAKED);
    assert!(
        o.ok,
        "xtask controls --list failed on a leak it cannot see:\n{}",
        o.out
    );
    check_no_control_run(&o);
}

validation::negative_control!(
    qa_m022_ci_fails_on_a_control_that_leaves_its_test_passing,
    "every control tripping, where the check requires ci to fail",
    expected = "xtask ci passed with",
    check_ci_fails_naming(
        &run_with_fake_cargo("ctl_ci_leaked", &["ci"], LISTED, TRIPPED),
        "qa_m022_t::pairs",
        "its control did not make it fail"
    )
);

validation::negative_control!(
    qa_m022_ci_fails_on_a_test_without_control,
    "every listed test with its control, where the check requires ci to fail",
    expected = "xtask ci passed with",
    check_ci_fails_naming(
        &run_with_fake_cargo("ctl_ci_missing", &["ci"], LISTED, TRIPPED),
        "qa_m022_t::alone",
        "has no control"
    )
);

validation::negative_control!(
    qa_m022_ci_passes_when_every_control_trips,
    "a leaking control, where the check requires ci to pass",
    expected = "xtask ci failed with every control tripping",
    check_ci_passes_running_controls(&run_with_fake_cargo(
        "ctl_ci_tripped",
        &["ci"],
        LISTED,
        LEAKED
    ))
);

validation::negative_control!(
    qa_m022_list_runs_no_control,
    "bare `xtask controls`, which runs the controls, given to the no-run check",
    expected = "xtask controls --list ran cargo test beyond the listings",
    check_no_control_run(&run_with_fake_cargo(
        "ctl_list",
        &["controls"],
        LISTED,
        TRIPPED
    ))
);
