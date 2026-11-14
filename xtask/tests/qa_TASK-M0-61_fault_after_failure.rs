//! QA's tests for TASK-M0-61's second round, written from REQ-TOOL-231 (R-988, R-714): `win-retry` "must never re-run
//! a job that failed a test or a step on its own output", and must re-run once a job that "stopped on a runner-level
//! fault". A job on the runner can do both: `win-vulkan` runs its suites on past a failure, so the runner may go in a
//! later step after an earlier step failed on its own output. Its own failure stands, and it is not re-run; a job whose
//! only lines are the runner's going and the cancel that going leaves is still re-run once.
//!
//! The runs are built here from GitHub's own texts and fed to `win_retry.py classify`; nothing is read from how the
//! classifier is written. Each test has its negative control (R-176): the check pointed at a run that must decide the
//! other way. Unix only, as the classifier runs under `python3`.

#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};
use validation::negative_control;
use validation::spawn::Spawn;

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;

const SCRIPT: &str = ".github/scripts/win_retry.py";
const PC: &str = "malachys-pc-rtx2080s";
const RUN: u64 = 77_100_001;
const A: u64 = 9_100_001;
const B: u64 = 9_100_002;

// GitHub's texts: a self-hosted runner gone; the runner service's shutdown; the cancel a step cut short leaves; a test
// step's exit.
const LOST: &str = "The self-hosted runner: malachys-pc-rtx2080s lost communication with the server. Verify the \
                    machine is running and has a healthy network connection. Anything in your workflow that \
                    terminates the runner process, starves it for CPU/Memory, or blocks its network access can cause \
                    this error.";
const SHUTDOWN: &str =
    "The runner has received a shutdown signal. This can happen when the runner service is \
                        stopped, or a manually started runner is canceled.";
const CANCELED: &str = "The operation was canceled.";
const TEST_FAILED: &str = "Process completed with exit code 100.";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask sits in the workspace")
        .to_path_buf()
}

/// One job on the runner: its name, its failure annotations and its `##[error]` log lines.
struct Job {
    id: u64,
    name: &'static str,
    annotations: Vec<&'static str>,
    errors: Vec<&'static str>,
}

fn job(id: u64, name: &'static str, annotations: &[&'static str], errors: &[&'static str]) -> Job {
    Job {
        id,
        name,
        annotations: annotations.to_vec(),
        errors: errors.to_vec(),
    }
}

/// A first-attempt push to `main` of win-gpu, these jobs on the runner failed, no newer run started; the verdict.
fn verdict(jobs: &[Job]) -> String {
    let dir = Scratch::new("qa_m061_fault_after_failure");
    let data = dir.join("run");
    write(&data, jobs);
    let out = Command::new("python3")
        .arg("-I")
        .arg(root().join(SCRIPT))
        .arg("classify")
        .arg(&data)
        .timed_output()
        .expect("qa: python3 ran");
    assert!(
        out.status.success(),
        "qa: the classifier failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn write(dir: &Path, jobs: &[Job]) {
    std::fs::create_dir_all(dir.join("annotations")).unwrap();
    std::fs::create_dir_all(dir.join("logs")).unwrap();
    let run = json!({"id": RUN, "name": "win-gpu", "event": "push", "head_branch": "main", "run_attempt": 1,
                     "status": "completed", "conclusion": "failure"});
    std::fs::write(dir.join("run.json"), run.to_string()).unwrap();
    let listed: Vec<Value> = jobs
        .iter()
        .map(|j| {
            // Its suites run on past a failure: an early test step failed on its output, the last one cut short.
            let early = if j.errors.contains(&TEST_FAILED) {
                "failure"
            } else {
                "success"
            };
            json!({"id": j.id, "name": j.name, "status": "completed", "conclusion": "failure", "runner_name": PC,
                   "labels": ["self-hosted", "windows", "nvidia"], "steps": [
                {"number": 1, "name": "Set up job", "status": "completed", "conclusion": "success"},
                {"number": 2, "name": "cargo nextest run -p engine", "status": "completed",
                 "conclusion": early},
                {"number": 3, "name": "cargo nextest run -p validation", "status": "completed",
                 "conclusion": "cancelled"}]})
        })
        .collect();
    std::fs::write(dir.join("jobs.json"), Value::from(listed).to_string()).unwrap();
    for j in jobs {
        let annotations: Vec<Value> = j
            .annotations
            .iter()
            .enumerate()
            .map(|(i, m)| {
                json!({"path": ".github", "start_line": i + 1, "end_line": i + 1, "annotation_level": "failure",
                       "title": "", "message": m, "raw_details": ""})
            })
            .collect();
        std::fs::write(
            dir.join(format!("annotations/{}.json", j.id)),
            Value::from(annotations).to_string(),
        )
        .unwrap();
        let mut log = String::from("\u{feff}2026-11-13T01:00:00.0000000Z ##[group]Run cargo\n");
        for (i, e) in j.errors.iter().enumerate() {
            log.push_str(&format!(
                "2026-11-13T01:{:02}:00.0000000Z test result: FAILED\n2026-11-13T01:{:02}:01.0000000Z ##[error]{e}\n",
                10 + i,
                10 + i
            ));
        }
        std::fs::write(dir.join(format!("logs/{}.log", j.id)), log).unwrap();
    }
    std::fs::write(dir.join("newer.json"), "[]").unwrap();
}

fn check_kept(what: &str, jobs: &[Job]) {
    let text = verdict(jobs);
    assert!(
        text.starts_with("rerun=none\n"),
        "qa: {what}: re-run, though the job failed on its own output first: {text}"
    );
    for j in jobs {
        assert!(
            text.lines()
                .any(|l| !l.starts_with("rerun=") && l.contains(j.name)),
            "qa: {what}: the stop of {} is not logged: {text}",
            j.name
        );
    }
}

fn check_rerun(what: &str, jobs: &[Job], id: u64) {
    let text = verdict(jobs);
    assert_eq!(
        text.lines().next().unwrap_or(""),
        format!("rerun=job {id}"),
        "qa: {what}: the runner-level fault is not re-run once, alone: {text}"
    );
}

/// A test failure, then the runner's going in a later step, in each of the places GitHub may put either.
fn failed_then_gone() -> Vec<(&'static str, Job)> {
    vec![
        (
            "a test failure, then a shutdown",
            job(
                A,
                "win-vulkan",
                &[TEST_FAILED, SHUTDOWN, CANCELED],
                &[TEST_FAILED, CANCELED],
            ),
        ),
        (
            "a test failure in the log alone, then a lost runner",
            job(A, "win-vulkan", &[LOST], &[TEST_FAILED]),
        ),
        (
            "a test failure, then a shutdown in the log",
            job(A, "win-vulkan", &[TEST_FAILED], &[TEST_FAILED, SHUTDOWN]),
        ),
    ]
}

/// The runner's going alone, with the cancel line a step cut short leaves, in the annotations and the log.
fn gone_alone() -> Vec<(&'static str, Job)> {
    vec![
        (
            "a shutdown and its cancel",
            job(A, "win-vulkan", &[SHUTDOWN, CANCELED], &[CANCELED]),
        ),
        (
            "a lost runner and its cancel",
            job(A, "win-vulkan", &[LOST, CANCELED], &[CANCELED]),
        ),
    ]
}

fn check_failure_wins() {
    for (what, j) in failed_then_gone() {
        check_kept(what, &[j]);
    }
}

fn check_fault_alone_reruns() {
    for (what, j) in gone_alone() {
        check_rerun(what, &[j], A);
    }
    // Beside it, a job that failed on its own and then lost its runner is not re-run with it.
    check_rerun(
        "a fault beside a failure-then-fault",
        &[
            job(
                A,
                "win-vulkan",
                &[TEST_FAILED, SHUTDOWN],
                &[TEST_FAILED, CANCELED],
            ),
            job(B, "win-gpu-idle", &[SHUTDOWN, CANCELED], &[CANCELED]),
        ],
        B,
    );
}

#[test]
fn qa_win_retry_keeps_a_failure_the_runner_going_follows() {
    check_failure_wins();
}

negative_control!(
    qa_win_retry_keeps_a_failure_the_runner_going_follows,
    "the kept check pointed at a shutdown alone must fail",
    expected =
        "qa: a shutdown and its cancel: re-run, though the job failed on its own output first",
    check_kept(
        "a shutdown and its cancel",
        &[job(A, "win-vulkan", &[SHUTDOWN, CANCELED], &[CANCELED])]
    )
);

#[test]
fn qa_win_retry_reruns_the_runner_going_and_its_cancel_alone() {
    check_fault_alone_reruns();
}

negative_control!(
    qa_win_retry_reruns_the_runner_going_and_its_cancel_alone,
    "the re-run check pointed at a test failure then a shutdown must fail",
    expected =
        "qa: a test failure, then a shutdown: the runner-level fault is not re-run once, alone",
    check_rerun(
        "a test failure, then a shutdown",
        &[job(
            A,
            "win-vulkan",
            &[TEST_FAILED, SHUTDOWN, CANCELED],
            &[TEST_FAILED, CANCELED]
        )],
        A
    )
);
