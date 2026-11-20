//! QA's tests for TASK-M0-61's fourth round, written from REQ-TOOL-231 and REQ-TOOL-232 (R-988, R-714). A job on the
//! runner may pass on from a step that reached its time limit (a `continue-on-error` download, then a second try that
//! passes) and then stop later on a runner-level fault. REQ-TOOL-231 asks that a job that "stopped on a runner-level
//! fault" be re-run once, and that the summary "log every unclear stop and every hang it sees"; REQ-TOOL-232 asks that
//! a job or step "reaching its limit is cancelled, logged by `win-retry` and not re-run". The step's limit is a hang the
//! job went past, not what stopped it: the job is still re-run once, and the summary logs both the fault and the hang.
//! Where the limit did stop the step (the step not passing), the job is a hang and nothing is re-run.
//!
//! The run is built here from GitHub's own texts and fed to `win_retry.py decide` under a stand-in `gh`, which records
//! every re-run request; nothing is read from how the classifier is written. Each test has its negative control
//! (R-176): the check pointed at a run that must decide the other way. Unix only, as the classifier runs under
//! `python3` and the stand-in `gh` is a shell script.

#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};
use validation::negative_control;
use validation::spawn::{write_executable, Spawn};

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;

const SCRIPT: &str = ".github/scripts/win_retry.py";
const PC: &str = "malachys-pc-rtx2080s";
const RUN: u64 = 77_300_001;
const JOB: u64 = 9_300_001;
const FIRST: &str = "Fetch the image (first try)";
const SECOND: &str = "Fetch the image (second try)";
const PROOFS: &str = "cargo xtask wsl-proofs";

// GitHub's texts: a step's own time limit; a self-hosted runner gone; the runner service's shutdown.
const STEP_LIMIT: &str = "The action 'Fetch the image (first try)' has timed out after 7 minutes.";
const LOST: &str = "The self-hosted runner: malachys-pc-rtx2080s lost communication with the server. Verify the \
                    machine is running and has a healthy network connection. Anything in your workflow that \
                    terminates the runner process, starves it for CPU/Memory, or blocks its network access can cause \
                    this error.";
const SHUTDOWN: &str =
    "The runner has received a shutdown signal. This can happen when the runner service is \
                        stopped, or a manually started runner is canceled.";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask sits in the workspace")
        .to_path_buf()
}

/// A first-attempt push to `main` of runner-proof, no newer run started. `wsl-proofs` on the runner failed: its first
/// fetch reached its limit and concluded `first`, its second passed, its proofs step concluded `failure`; its failure
/// annotations are `stop` after the step's limit.
struct Case {
    first: &'static str,
    stop: &'static str,
}

const fn case(first: &'static str, stop: &'static str) -> Case {
    Case { first, stop }
}

fn write(dir: &Path, c: &Case) {
    std::fs::create_dir_all(dir.join("annotations")).unwrap();
    let jobs = json!([
        {"id": JOB, "name": "wsl-proofs", "status": "completed", "conclusion": "failure", "runner_name": PC,
         "labels": ["self-hosted", "windows", "nvidia"], "steps": [
            {"number": 1, "name": "Set up job", "status": "completed", "conclusion": "success"},
            {"number": 2, "name": FIRST, "status": "completed", "conclusion": c.first},
            {"number": 3, "name": SECOND, "status": "completed", "conclusion": "success"},
            {"number": 4, "name": PROOFS, "status": "completed", "conclusion": "failure"}]},
        {"id": JOB + 1, "name": "wsl-image", "status": "completed", "conclusion": "success",
         "runner_name": "GitHub Actions 3", "labels": ["ubuntu-24.04"], "steps": []}
    ]);
    std::fs::write(dir.join("jobs.json"), jobs.to_string()).unwrap();
    let annotations: Vec<Value> = [STEP_LIMIT, c.stop]
        .iter()
        .map(|m| {
            json!({"path": ".github", "start_line": 1, "end_line": 1, "annotation_level": "failure",
                   "title": "", "message": m, "raw_details": ""})
        })
        .collect();
    std::fs::write(
        dir.join(format!("annotations/{JOB}.json")),
        Value::from(annotations).to_string(),
    )
    .unwrap();
}

const FAKE_GH: &str = r#"#!/bin/sh
d="$QA_GH_DIR"
args="$*"
case "$args" in
  *"--method POST"*) echo "$args" >> "$d/posts"; echo '{}'; exit 0 ;;
  *"/attempts/"*"/jobs"*) printf '{"total_count":2,"jobs":%s}' "$(cat "$d/run/jobs.json")"; exit 0 ;;
  *"/check-runs/"*"/annotations"*)
    id=$(echo "$args" | sed 's#.*check-runs/\([0-9]*\)/annotations.*#\1#')
    if [ -f "$d/run/annotations/$id.json" ]; then cat "$d/run/annotations/$id.json"; else echo '[]'; fi; exit 0 ;;
  *"/jobs/"*"/logs"*) echo 'not found' >&2; exit 1 ;;
  *"/workflows/"*"/runs"*) echo '{"total_count":0,"workflow_runs":[]}'; exit 0 ;;
  *"/runs/"*"/jobs"*) echo '{"total_count":0,"jobs":[]}'; exit 0 ;;
esac
echo "qa stand-in gh: unexpected $args" >&2
exit 1
"#;

struct Decided {
    stdout: String,
    summary: String,
    posts: String,
}

fn decide(c: &Case) -> Decided {
    let dir = Scratch::new("qa_m061_fault_after_recovered");
    std::fs::create_dir_all(dir.join("bin")).unwrap();
    write(&dir.join("run"), c);
    write_executable(&dir.join("bin/gh"), FAKE_GH).unwrap();
    let summary = dir.join("summary.md");
    let path = format!(
        "{}:{}",
        dir.join("bin").display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = Command::new("python3")
        .arg("-I")
        .arg(root().join(SCRIPT))
        .arg("decide")
        .env("PATH", path)
        .env("QA_GH_DIR", &*dir)
        .env("GH_REPO", "Ma1achy/Principia")
        .env("RUN_ID", RUN.to_string())
        .env("RUN_ATTEMPT", "1")
        .env("RUN_EVENT", "push")
        .env("HEAD_BRANCH", "main")
        .env("RUN_CONCLUSION", "failure")
        .env("WORKFLOW_ID", "4244")
        .env("GITHUB_STEP_SUMMARY", &summary)
        .timed_output()
        .expect("qa: python3 ran");
    assert!(
        out.status.success(),
        "qa: decide failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    Decided {
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        summary: std::fs::read_to_string(&summary).unwrap_or_default(),
        posts: std::fs::read_to_string(dir.join("posts")).unwrap_or_default(),
    }
}

/// The job is re-run once, alone, and the summary logs both the fault (with the re-run) and the step's hang.
fn check_rerun_once(c: &Case, what: &str) {
    let d = decide(c);
    let posts: Vec<&str> = d.posts.lines().collect();
    assert_eq!(
        posts.len(),
        1,
        "qa: {what}: a fault after a step limit passed on from is not re-run exactly once: {:?}\n{}",
        d.posts,
        d.stdout
    );
    assert!(
        posts[0].contains(&format!("actions/jobs/{JOB}/rerun")),
        "qa: {what}: the re-run is not of the faulted job alone: {}",
        posts[0]
    );
    let rows: Vec<&str> = d
        .summary
        .lines()
        .filter(|l| l.starts_with("| wsl-proofs |"))
        .collect();
    assert!(
        rows.iter()
            .any(|r| r.contains("runner-level fault")
                && r.contains(&format!("run {RUN}, attempt 2"))),
        "qa: {what}: the summary does not log the fault with its re-run: {}",
        d.summary
    );
    assert!(
        rows.iter().any(|r| r.contains(FIRST)
            && r.contains("hang")
            && r.contains("has timed out after 7 minutes")
            && !r.contains("attempt 2")),
        "qa: {what}: the summary does not log the step's limit as a hang, not re-run: {}",
        d.summary
    );
}

#[test]
fn qa_win_retry_reruns_a_fault_after_a_step_limit_passed_on_from() {
    check_rerun_once(&case("success", LOST), "a lost runner");
    check_rerun_once(&case("success", SHUTDOWN), "a shutdown");
}

negative_control!(
    qa_win_retry_reruns_a_fault_after_a_step_limit_passed_on_from,
    "the re-run check pointed at a job its step's limit stopped must fail",
    expected =
        "qa: a stopped step: a fault after a step limit passed on from is not re-run exactly once",
    check_rerun_once(&case("cancelled", LOST), "a stopped step")
);

#[cfg(feature = "controls")]
mod qa_own_failure_control {
    use super::*;
    // GitHub's text for a step's exit.
    const OWN_FAILURE: &str = "Process completed with exit code 1.";
    negative_control!(
        qa_win_retry_reruns_a_fault_after_a_step_limit_passed_on_from,
        "the re-run check pointed at a failure of the job's own after the limit must fail",
        expected =
            "qa: own failure: a fault after a step limit passed on from is not re-run exactly once",
        check_rerun_once(&case("success", OWN_FAILURE), "own failure")
    );
}

/// A step's limit that stopped the step (it did not pass) stopped the job: a hang, never re-run, though the runner's
/// going follows it.
fn check_hang_kept(c: &Case, what: &str) {
    let d = decide(c);
    assert!(
        d.posts.is_empty(),
        "qa: {what}: a job its step's limit stopped is re-run: {:?}",
        d.posts
    );
    assert!(
        d.summary
            .lines()
            .any(|l| l.starts_with("| wsl-proofs |") && l.contains(FIRST) && l.contains("hang")),
        "qa: {what}: the stopping limit is not logged as a hang: {}",
        d.summary
    );
}

#[test]
fn qa_win_retry_keeps_a_job_its_step_limit_stopped() {
    check_hang_kept(&case("cancelled", LOST), "a cancelled step");
    check_hang_kept(&case("failure", LOST), "a failed step");
}

negative_control!(
    qa_win_retry_keeps_a_job_its_step_limit_stopped,
    "the hang check pointed at a step passed on from, the runner lost after it, must fail",
    expected = "qa: a passed-on step: a job its step's limit stopped is re-run",
    check_hang_kept(&case("success", LOST), "a passed-on step")
);
