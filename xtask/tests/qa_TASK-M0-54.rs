//! qa's tests for TASK-M0-54, written from REQ-TOOL-223 and R-809, not from `ci.yml`'s gate or `ci_changes.rs`'s model.
//!
//! REQ-TOOL-223: "The required check `ci` must gather `ci-release`: a `ci-release` shard that fails, or that skips on
//! a change that isn't docs or data, must fail the check, and a skip on a docs or data change must pass it (R-809)."
//! R-809: "A skip counts as a pass only on a docs or data change, where `ci-release` and its archive don't run. A
//! failed archive job skips `ci-release`, and that skip must fail the gate."
//!
//! - Retired with `ci-gate` by the owner's CI model (approved 9 Nov 2026), which takes the full tier, the release
//!   profile among it, off the pull request's path to each merge to `main`, where no required check gathers it:
//!   `qa_m0_54_the_gate_gathers_ci_release_and_reads_every_result`, `qa_m0_54_each_release_ending_on_each_head` and
//!   `qa_m0_54_the_quick_tier_is_untouched` (R-290's exception, applied per R-369; REQ-TOOL-223's gate is the model's
//!   to restate).
//! - `qa_m0_54_no_release_failure_is_swallowed`: neither job, and no step of theirs that runs or builds the tests,
//!   turns a failure into a pass by `continue-on-error` or `|| true`, so the per-merge run holds the failure.
//! - `qa_m0_54_a_runner_fault_upstream_of_ci_is_retried`, `qa_m0_54_a_real_release_failure_is_not_retried`: the seam
//!   R-809 opens with `retry.yml`: `ci_retry.py`, on the real `ci.yml`, must re-run a runner fault in a release shard
//!   or in the archive, and must not re-run a real failure.
//!
//! The gate's script is a workflow `run:` step: it runs here under a scratch HOME and CARGO_HOME. Each test has its
//! negative control (R-176).

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};
use validation::negative_control;
use validation::spawn::Spawn;

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the workspace root")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("qa_m0_54: {rel}: {e}"))
}

fn workflow() -> String {
    read(".github/workflows/ci.yml")
}

/// The checked-in workflow with `from` replaced once by `to`; `from` must be there, so a control cannot pass vacuously.
#[cfg(feature = "controls")]
fn edited(from: &str, to: &str) -> String {
    let yml = workflow();
    assert!(
        yml.contains(from),
        "the control's edit target is gone: {from}"
    );
    yml.replacen(from, to, 1)
}

/// `yml`'s jobs, through PyYAML.
fn jobs(yml: &str) -> Value {
    let dir = Scratch::new("qa_m0_54_yaml");
    std::fs::create_dir_all(&*dir).expect("scratch folder");
    let file = dir.join("ci.yml");
    std::fs::write(&file, yml).expect("workflow written");
    let out = Command::new("python3")
        .args([
            "-c",
            "import json, sys, yaml\nprint(json.dumps(yaml.safe_load(open(sys.argv[1]))['jobs']))",
        ])
        .arg(&file)
        .timed_output()
        .expect("qa_m0_54: python3 ran");
    assert!(
        out.status.success(),
        "qa_m0_54: the workflow is not YAML: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("qa_m0_54: PyYAML's JSON")
}

fn text(v: &Value) -> String {
    v.as_str().map(str::to_owned).unwrap_or_default()
}

// Retired with the gate (the owner's CI model, approved 9 Nov 2026):
// `qa_m0_54_the_gate_gathers_ci_release_and_reads_every_result`, `qa_m0_54_each_release_ending_on_each_head` and `qa_m0_54_the_quick_tier_is_untouched`, with their controls and the
// walk they shared. They held REQ-TOOL-223's "the required check `ci` must gather `ci-release`" (R-809) on `ci-gate`.
// The model moves the release profile, with the rest of the full tier, off the pull request's path to each merge to
// `main`, where no required check gathers anything: `ci` is the PR tier's product tests, a job of its own, and
// `ci-gate` and the quick tier are gone. What stays held is that the release jobs swallow no failure, so the per-merge
// run is red when they fail, and the retry seam on them (R-290's exception, applied per R-369 by the orchestrator).

// ---- The result the gate reads can hold the failure -----------------------------------------------------------------

fn check_nothing_swallowed(yml: &str) {
    let jobs = jobs(yml);
    for id in ["ci-release", "ci-release-archive"] {
        let job = &jobs[id];
        assert!(
            job.get("continue-on-error").is_none(),
            "qa_m0_54: `{id}` passes whatever it ends in"
        );
        let steps = job["steps"].as_array().expect("the job's steps");
        let tests: Vec<&Value> = steps
            .iter()
            .filter(|s| {
                let run = text(&s["run"]);
                run.contains("nextest") || run.contains("cargo test")
            })
            .collect();
        assert!(!tests.is_empty(), "qa_m0_54: `{id}` has no test step");
        for step in tests {
            assert!(
                step.get("continue-on-error").is_none() && !text(&step["run"]).contains("|| true"),
                "qa_m0_54: a test step of `{id}` passes whatever it ends in: {}",
                text(&step["run"])
            );
        }
    }
}

#[test]
fn qa_m0_54_no_release_failure_is_swallowed() {
    check_nothing_swallowed(&workflow());
}

negative_control!(
    qa_m0_54_no_release_failure_is_swallowed,
    "ci.yml with `ci-release` allowed to fail",
    expected = "`ci-release` passes whatever it ends in",
    check_nothing_swallowed(&edited(
        "    needs: ci-release-archive\n",
        "    needs: ci-release-archive\n    continue-on-error: true\n",
    ))
);

mod a_test_step_allowed_to_fail {
    use super::*;
    negative_control!(
        qa_m0_54_no_release_failure_is_swallowed,
        "ci.yml with `ci-release`'s doctest step allowed to fail",
        expected = "a test step of `ci-release` passes whatever it ends in",
        check_nothing_swallowed(&edited(
            "        run: cargo test --workspace --release --no-fail-fast --doc\n",
            "        continue-on-error: true\n        run: cargo test --workspace --release --no-fail-fast --doc\n",
        ))
    );
}

// ---- `ci_retry.py` on the real `ci.yml`, `ci` now red with `ci-release` ---------------------------------------------

/// What a runner's full disk leaves in a job's failure annotation.
const NO_SPACE: &str = "Unhandled exception. System.IO.IOException: No space left on device";
/// What a real test failure leaves.
const REAL: &str = "Process completed with exit code 100.";

/// A job of a run: its name, how it ended and its failure annotation.
type RunJob<'a> = (&'a str, &'a str, Option<&'a str>);

/// Whether `ci_retry.py classify` re-runs a failed first attempt of `ci.yml` made of `failed`, every other job of the
/// gate's having passed.
fn retried(failed: &[RunJob]) -> bool {
    const PASSED: [&str; 8] = [
        "changes",
        "ci-archive",
        "ci (1)",
        "ci (2)",
        "ci (3)",
        "ci (4)",
        "ci-checks",
        "ci-workspace",
    ];
    let all: Vec<RunJob> = PASSED
        .iter()
        .map(|n| (*n, "success", None))
        .chain(failed.iter().copied())
        .collect();
    let dir = Scratch::new("qa_m0_54_run");
    std::fs::create_dir_all(dir.join("annotations")).expect("scratch folder");
    let listed: Vec<Value> = all
        .iter()
        .enumerate()
        .map(|(i, (name, conclusion, message))| {
            if let Some(message) = message {
                let annotation = json!([{"annotation_level": "failure", "message": message}]);
                std::fs::write(
                    dir.join("annotations").join(format!("{}.json", i + 1)),
                    annotation.to_string(),
                )
                .expect("annotation written");
            }
            json!({"id": i + 1, "name": name, "status": "completed", "conclusion": conclusion})
        })
        .collect();
    std::fs::write(
        dir.join("run.json"),
        json!({"conclusion": "failure", "run_attempt": 1}).to_string(),
    )
    .expect("run written");
    std::fs::write(dir.join("jobs.json"), Value::from(listed).to_string()).expect("jobs written");
    let out = Command::new("python3")
        .arg("-I")
        .arg(root().join(".github/scripts/ci_retry.py"))
        .arg("classify")
        .arg(&*dir)
        .arg(root().join(".github/workflows/ci.yml"))
        .timed_output()
        .expect("qa_m0_54: python3 ran");
    assert!(out.status.success(), "qa_m0_54: the classifier failed");
    let verdict = String::from_utf8_lossy(&out.stdout);
    match verdict.lines().next() {
        Some("retry=true") => true,
        Some("retry=false") => false,
        _ => panic!("qa_m0_54: no verdict in: {verdict}"),
    }
}

/// A release shard of the per-merge run ended by `message`, the other passing. Until the owner's CI model (9 Nov
/// 2026) `ci`'s gate went red with it, a gate of the fault; the release jobs run on `main` now, where no `ci` gate
/// gathers them, so the failed shard is the run's one failed job (R-290's exception, applied per R-369).
fn a_shard_failed(message: &str) -> bool {
    retried(&[
        ("ci-release-archive", "success", None),
        ("ci-release (1)", "failure", Some(message)),
        ("ci-release (2)", "success", None),
    ])
}

/// The archive ended by `message`, the shards skipped (no `ci` gate red through the skip since the owner's CI model).
fn the_archive_failed(message: &str) -> bool {
    retried(&[
        ("ci-release-archive", "failure", Some(message)),
        // A job with no `name:` skipped at job level reports its bare id.
        ("ci-release", "skipped", None),
    ])
}

fn check_fault_retried(message: &str) {
    assert!(
        a_shard_failed(message),
        "qa_m0_54: a runner fault in a release shard and `ci` red: not re-run"
    );
    assert!(
        the_archive_failed(message),
        "qa_m0_54: a runner fault in the release archive and `ci` red: not re-run"
    );
}

#[test]
fn qa_m0_54_a_runner_fault_upstream_of_ci_is_retried() {
    check_fault_retried(NO_SPACE);
}

negative_control!(
    qa_m0_54_a_runner_fault_upstream_of_ci_is_retried,
    "a real failure in place of the fault",
    expected = "a runner fault in a release shard and `ci` red: not re-run",
    check_fault_retried(REAL)
);

fn check_real_not_retried(message: &str) {
    assert!(
        !a_shard_failed(message),
        "qa_m0_54: a real failure in a release shard is re-run"
    );
    assert!(
        !the_archive_failed(message),
        "qa_m0_54: a real failure in the release archive is re-run"
    );
}

#[test]
fn qa_m0_54_a_real_release_failure_is_not_retried() {
    check_real_not_retried(REAL);
}

negative_control!(
    qa_m0_54_a_real_release_failure_is_not_retried,
    "a runner fault in place of the real failure",
    expected = "a real failure in a release shard is re-run",
    check_real_not_retried(NO_SPACE)
);
