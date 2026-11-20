//! QA's tests for TASK-M0-61's fifth round, written from REQ-TOOL-231 and REQ-TOOL-232 (R-988, R-714). REQ-TOOL-231
//! asks that the run summary "log every unclear stop and every hang it sees, with the run, the job, the step where it
//! applies, the signature and the re-run's id", and REQ-TOOL-232 that "a job or step reaching its limit is cancelled,
//! logged by `win-retry` and not re-run". A job on the runner may reach more than one limit: a build step's, then a
//! test step run `if: always()`, then the job's own. Each is a hang it sees, so each is logged, at its own step, once
//! though both its annotation and its log line carry it, and none is re-run. A runner-level fault in another job of the
//! same run is still re-run once, alone: the extra hangs change no decision.
//!
//! The run is built here from GitHub's own texts and fed to `win_retry.py decide` under a stand-in `gh`, which serves
//! each job's annotations and log and records every re-run request; nothing is read from how the classifier is
//! written. Each test has its negative control (R-176): the check pointed at a run that must log otherwise. Unix only,
//! as the classifier runs under `python3` and the stand-in `gh` is a shell script.

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
const RUN: u64 = 77_500_001;
const HUNG: u64 = 9_500_001;
const FAULTED: u64 = 9_500_002;
const BUILD: &str = "Build the GPU tests";
const TESTS: &str = "Run the GPU tests";

// GitHub's texts: two steps' limits, the job's own limit, and a self-hosted runner gone.
const BUILD_LIMIT: &str = "The action 'Build the GPU tests' has timed out after 40 minutes.";
const TESTS_LIMIT: &str = "The action 'Run the GPU tests' has timed out after 30 minutes.";
const JOB_LIMIT: &str =
    "The job running on runner malachys-pc-rtx2080s has exceeded the maximum execution time of 90 minutes.";
const LOST: &str = "The self-hosted runner: malachys-pc-rtx2080s lost communication with the server. Verify the \
                    machine is running and has a healthy network connection. Anything in your workflow that \
                    terminates the runner process, starves it for CPU/Memory, or blocks its network access can cause \
                    this error.";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask sits in the workspace")
        .to_path_buf()
}

fn annotated(messages: &[&str]) -> String {
    let all: Vec<Value> = messages
        .iter()
        .map(|m| {
            json!({"path": ".github", "start_line": 1, "end_line": 1, "annotation_level": "failure",
                   "title": "", "message": m, "raw_details": ""})
        })
        .collect();
    Value::from(all).to_string()
}

/// A first-attempt scheduled run of win-gpu, no newer run started. `win-vulkan` on the runner reached each of `hung`'s
/// limits, carried by its annotations and again by its log's `##[error]` lines; `win-metal` on it lost its runner.
fn write(dir: &Path, hung: &[&str]) {
    std::fs::create_dir_all(dir.join("annotations")).unwrap();
    std::fs::create_dir_all(dir.join("logs")).unwrap();
    let jobs = json!([
        {"id": HUNG, "name": "win-vulkan", "status": "completed", "conclusion": "cancelled", "runner_name": PC,
         "labels": ["self-hosted", "windows", "nvidia"], "steps": [
            {"number": 1, "name": "Set up job", "status": "completed", "conclusion": "success"},
            {"number": 2, "name": BUILD, "status": "completed", "conclusion": "cancelled"},
            {"number": 3, "name": TESTS, "status": "completed", "conclusion": "cancelled"}]},
        {"id": FAULTED, "name": "win-metal", "status": "completed", "conclusion": "failure", "runner_name": PC,
         "labels": ["self-hosted", "windows", "nvidia"], "steps": [
            {"number": 1, "name": "Set up job", "status": "completed", "conclusion": "success"},
            {"number": 2, "name": TESTS, "status": "completed", "conclusion": "failure"}]},
        {"id": HUNG + 10, "name": "wsl-image", "status": "completed", "conclusion": "success",
         "runner_name": "GitHub Actions 3", "labels": ["ubuntu-24.04"], "steps": []}
    ]);
    std::fs::write(dir.join("jobs.json"), jobs.to_string()).unwrap();
    std::fs::write(
        dir.join(format!("annotations/{HUNG}.json")),
        annotated(hung),
    )
    .unwrap();
    std::fs::write(
        dir.join(format!("annotations/{FAULTED}.json")),
        annotated(&[LOST]),
    )
    .unwrap();
    let log: String = hung
        .iter()
        .enumerate()
        .map(|(i, m)| format!("2026-11-13T02:{:02}:00.0000000Z ##[error]{m}\n", 10 + i))
        .collect();
    std::fs::write(dir.join(format!("logs/{HUNG}.log")), log).unwrap();
}

const FAKE_GH: &str = r#"#!/bin/sh
d="$QA_GH_DIR"
args="$*"
case "$args" in
  *"--method POST"*) echo "$args" >> "$d/posts"; echo '{}'; exit 0 ;;
  *"/attempts/"*"/jobs"*) printf '{"total_count":3,"jobs":%s}' "$(cat "$d/run/jobs.json")"; exit 0 ;;
  *"/check-runs/"*"/annotations"*)
    id=$(echo "$args" | sed 's#.*check-runs/\([0-9]*\)/annotations.*#\1#')
    if [ -f "$d/run/annotations/$id.json" ]; then cat "$d/run/annotations/$id.json"; else echo '[]'; fi; exit 0 ;;
  *"/jobs/"*"/logs"*)
    id=$(echo "$args" | sed 's#.*jobs/\([0-9]*\)/logs.*#\1#')
    if [ -f "$d/run/logs/$id.log" ]; then cat "$d/run/logs/$id.log"; exit 0; fi
    echo 'not found' >&2; exit 1 ;;
  *"/workflows/"*"/runs"*) echo '{"total_count":0,"workflow_runs":[]}'; exit 0 ;;
  *"/runs/"*"/jobs"*) echo '{"total_count":0,"jobs":[]}'; exit 0 ;;
esac
echo "qa stand-in gh: unexpected $args" >&2
exit 1
"#;

struct Decided {
    summary: String,
    posts: String,
}

fn decide(hung: &[&str]) -> Decided {
    let dir = Scratch::new("qa_m061_every_hang");
    std::fs::create_dir_all(dir.join("bin")).unwrap();
    write(&dir.join("run"), hung);
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
        .env("RUN_EVENT", "schedule")
        .env("HEAD_BRANCH", "main")
        .env("RUN_CONCLUSION", "failure")
        .env("WORKFLOW_ID", "4245")
        .env("GITHUB_STEP_SUMMARY", &summary)
        .timed_output()
        .expect("qa: python3 ran");
    assert!(
        out.status.success(),
        "qa: decide failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    Decided {
        summary: std::fs::read_to_string(&summary).unwrap_or_default(),
        posts: std::fs::read_to_string(dir.join("posts")).unwrap_or_default(),
    }
}

/// Each of `win-vulkan`'s three limits is a hang row of the summary, once, at its step (the job's own at the step it
/// stopped in), never re-run; `win-metal` is re-run once, alone, its row naming the re-run.
fn check_every_hang(hung: &[&str], what: &str) {
    let d = decide(hung);
    let posts: Vec<&str> = d.posts.lines().collect();
    assert!(
        posts.len() == 1 && posts[0].contains(&format!("actions/jobs/{FAULTED}/rerun")),
        "qa: {what}: the lost runner's job is not re-run once, alone: {:?}",
        d.posts
    );
    let rows: Vec<&str> = d
        .summary
        .lines()
        .filter(|l| l.starts_with("| win-vulkan |"))
        .collect();
    for (step, line) in [
        (BUILD, BUILD_LIMIT),
        (TESTS, TESTS_LIMIT),
        (BUILD, JOB_LIMIT),
    ] {
        let found = rows
            .iter()
            .filter(|r| r.contains(line) && r.contains(&format!("| {step} |")))
            .collect::<Vec<_>>();
        assert!(
            found.len() == 1,
            "qa: {what}: the limit `{line}` is not logged once at step '{step}': {}",
            d.summary
        );
        assert!(
            found[0].contains("hang") && found[0].ends_with("| — |"),
            "qa: {what}: the limit `{line}` is not logged as a hang, never re-run: {}",
            found[0]
        );
    }
    assert_eq!(
        rows.len(),
        3,
        "qa: {what}: a hung job's limits logged other than once each: {}",
        d.summary
    );
    assert!(
        d.summary.lines().any(|l| l.starts_with("| win-metal |")
            && l.contains("runner-level fault")
            && l.contains(&format!("run {RUN}, attempt 2"))),
        "qa: {what}: the fault is not logged with its re-run: {}",
        d.summary
    );
}

#[test]
fn qa_win_retry_logs_each_hang_of_a_job_once() {
    check_every_hang(&[BUILD_LIMIT, TESTS_LIMIT, JOB_LIMIT], "build, tests, job");
    // The order GitHub gives them in changes nothing.
    check_every_hang(&[JOB_LIMIT, TESTS_LIMIT, BUILD_LIMIT], "job, tests, build");
}

negative_control!(
    qa_win_retry_logs_each_hang_of_a_job_once,
    "the check pointed at a job that reached only its build step's limit must fail",
    expected = "qa: build alone: the limit `The action 'Run the GPU tests' has timed out after 30 minutes.` is not \
                logged once at step 'Run the GPU tests'",
    check_every_hang(&[BUILD_LIMIT], "build alone")
);
