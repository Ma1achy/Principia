//! QA's tests for TASK-M0-61's third round, written from REQ-TOOL-231 ("its run summary must log every unclear stop and
//! every hang it sees") and REQ-TOOL-232 ("a job or step reaching its limit is cancelled, logged by `win-retry` and not
//! re-run"), R-988. A step on the runner may reach its limit inside a job that goes on from it (a `continue-on-error`
//! step, then a second try that passes), so the job and the run read `success` while GitHub keeps the step's limit as a
//! failure annotation. That hang is still one `win-retry` sees: its workflow must read a passing run, its classifier
//! must log the hang with its job, step and signature whatever the run's event or attempt, and nothing re-runs it.
//!
//! The runs are built here from GitHub's own texts and fed to `win_retry.py` (`classify`, and `decide` under a stand-in
//! `gh`); the workflow's guard is evaluated for a passing run. Each test has its negative control (R-176): the check
//! pointed at a passing job with no limit reached, or at the guard as it stood before. Unix only, as the classifier
//! runs under `python3` and the stand-in `gh` is a shell script.

#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};
use validation::negative_control;
use validation::spawn::{write_executable, Spawn};
use yaml_rust2::YamlLoader;

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;

const SCRIPT: &str = ".github/scripts/win_retry.py";
const WIN_RETRY: &str = ".github/workflows/win-retry.yml";
const PC: &str = "malachys-pc-rtx2080s";
const RUN: u64 = 77_200_001;
const JOB: u64 = 9_200_001;
const STEP: &str = "Download the image (first try)";
// GitHub's texts: a step's own time limit; a step failing on its own output.
const STEP_LIMIT: &str =
    "The action 'Download the image (first try)' has timed out after 5 minutes.";
const OWN_FAILURE: &str = "Process completed with exit code 1.";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask sits in the workspace")
        .to_path_buf()
}

/// A completed run of runner-proof, every job passing: `wsl-proofs` on the runner, its first download carrying
/// `annotation` (a failure annotation, or none), its second passing; a hosted job beside it.
struct Case {
    event: &'static str,
    branch: &'static str,
    attempt: u64,
    annotation: Option<&'static str>,
}

fn case(annotation: Option<&'static str>) -> Case {
    Case {
        event: "push",
        branch: "main",
        attempt: 1,
        annotation,
    }
}

fn write(dir: &Path, c: &Case) {
    std::fs::create_dir_all(dir.join("annotations")).unwrap();
    std::fs::create_dir_all(dir.join("logs")).unwrap();
    let run = json!({"id": RUN, "name": "runner-proof", "event": c.event, "head_branch": c.branch,
                     "run_attempt": c.attempt, "status": "completed", "conclusion": "success"});
    std::fs::write(dir.join("run.json"), run.to_string()).unwrap();
    let jobs = json!([
        {"id": JOB, "name": "wsl-proofs", "status": "completed", "conclusion": "success", "runner_name": PC,
         "labels": ["self-hosted", "windows", "nvidia"], "steps": [
            {"number": 1, "name": "Set up job", "status": "completed", "conclusion": "success"},
            {"number": 2, "name": STEP, "status": "completed", "conclusion": "success"},
            {"number": 3, "name": "Download the image (second try)", "status": "completed", "conclusion": "success"},
            {"number": 4, "name": "cargo xtask wsl-proofs", "status": "completed", "conclusion": "success"}]},
        {"id": JOB + 1, "name": "wsl-image", "status": "completed", "conclusion": "success",
         "runner_name": "GitHub Actions 9", "labels": ["ubuntu-24.04"], "steps": []}
    ]);
    std::fs::write(dir.join("jobs.json"), jobs.to_string()).unwrap();
    let annotations: Vec<Value> = c
        .annotation
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
    std::fs::write(dir.join(format!("annotations/{}.json", JOB + 1)), "[]").unwrap();
    std::fs::write(dir.join("newer.json"), "[]").unwrap();
}

fn classified(c: &Case) -> String {
    let dir = Scratch::new("qa_m061_recovered_classify");
    let data = dir.join("run");
    write(&data, c);
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

/// The step's limit in a passing job is logged as a hang, with its job, step and signature, and not re-run, on every
/// event and attempt.
fn check_logged(c: &Case) {
    let what = format!("a {} run, attempt {}", c.event, c.attempt);
    let text = classified(c);
    assert!(
        text.starts_with("rerun=none\n"),
        "qa: {what}: a step's limit in a passing job is re-run: {text}"
    );
    let row = text.lines().find(|l| {
        !l.starts_with("rerun=") && !l.starts_with("evidence=") && l.contains("wsl-proofs")
    });
    assert!(
        row.is_some_and(|l| l.contains("hang") && l.contains(STEP)),
        "qa: {what}: a step's limit in a passing job is not logged as a hang with its step: {text}"
    );
    assert!(
        text.contains("has timed out after 5 minutes"),
        "qa: {what}: the limit's signature is not logged: {text}"
    );
}

fn check_every_event() {
    check_logged(&case(Some(STEP_LIMIT)));
    check_logged(&Case {
        event: "pull_request",
        branch: "task/x",
        ..case(Some(STEP_LIMIT))
    });
    check_logged(&Case {
        event: "workflow_dispatch",
        branch: "main",
        attempt: 2,
        ..case(Some(STEP_LIMIT))
    });
}

/// A passing job whose step failed on its own output, the job going on, is no hang.
fn check_no_false_hang() {
    let text = classified(&case(Some(OWN_FAILURE)));
    assert!(
        text.starts_with("rerun=none\n") && !text.lines().any(|l| l.contains("hang")),
        "qa: a passing job's own step failure is logged as a hang or re-run: {text}"
    );
}

#[test]
fn qa_win_retry_logs_a_step_limit_a_passing_job_went_on_from() {
    check_every_event();
    check_no_false_hang();
}

negative_control!(
    qa_win_retry_logs_a_step_limit_a_passing_job_went_on_from,
    "the logged check pointed at a passing job with no limit reached must fail",
    expected = "qa: a push run, attempt 1: a step's limit in a passing job is not logged as a hang with its step",
    check_logged(&case(None))
);

#[cfg(feature = "controls")]
mod qa_false_hang_control {
    use super::*;
    negative_control!(
        qa_win_retry_logs_a_step_limit_a_passing_job_went_on_from,
        "the no-false-hang check pointed at a step's limit must fail",
        expected = "qa: a passing job's own step failure is logged as a hang or re-run",
        {
            let text = classified(&case(Some(STEP_LIMIT)));
            assert!(
                text.starts_with("rerun=none\n") && !text.lines().any(|l| l.contains("hang")),
                "qa: a passing job's own step failure is logged as a hang or re-run: {text}"
            );
        }
    );
}

// ---- decide: the run summary, under a stand-in `gh` -------------------------------------------------------------------

const FAKE_GH: &str = r#"#!/bin/sh
d="$QA_GH_DIR"
args="$*"
echo "$args" >> "$d/calls"
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
    summary: String,
    posts: String,
}

fn decide(c: &Case) -> Decided {
    let dir = Scratch::new("qa_m061_recovered_decide");
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
        .env("RUN_ATTEMPT", c.attempt.to_string())
        .env("RUN_EVENT", c.event)
        .env("HEAD_BRANCH", c.branch)
        .env("RUN_CONCLUSION", "success")
        .env("WORKFLOW_ID", "4243")
        .env("GITHUB_STEP_SUMMARY", &summary)
        .timed_output()
        .expect("qa: python3 ran");
    assert!(
        out.status.success(),
        "qa: decide failed on a passing run: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    Decided {
        summary: std::fs::read_to_string(&summary).unwrap_or_default(),
        posts: std::fs::read_to_string(dir.join("posts")).unwrap_or_default(),
    }
}

/// Nothing re-run; the summary names the run, the job, the step and the limit's line.
fn check_decide_logged(c: &Case) {
    let d = decide(c);
    assert!(
        d.posts.is_empty(),
        "qa: decide re-ran a step's limit in a passing job: {:?}",
        d.posts
    );
    for (what, needle) in [
        ("the run", RUN.to_string()),
        ("the job", "wsl-proofs".to_owned()),
        ("the step", STEP.to_owned()),
        ("the signature", "has timed out after 5 minutes".to_owned()),
    ] {
        assert!(
            d.summary.contains(&needle),
            "qa: the summary does not log the passing job's hang's {what}: {}",
            d.summary
        );
    }
}

#[test]
fn qa_win_retry_decide_logs_a_step_limit_of_a_passing_run() {
    check_decide_logged(&case(Some(STEP_LIMIT)));
    check_decide_logged(&Case {
        event: "schedule",
        attempt: 2,
        ..case(Some(STEP_LIMIT))
    });
}

negative_control!(
    qa_win_retry_decide_logs_a_step_limit_of_a_passing_run,
    "the summary check pointed at a passing job with no limit reached must fail",
    expected = "qa: the summary does not log the passing job's hang's the job",
    check_decide_logged(&case(None))
);

// ---- the workflow's guard: a passing run is read --------------------------------------------------------------------

/// The `win-retry` job's `if:`, evaluated by Python for this repository, a completed run of the given attempt and
/// conclusion (the expression's `&&`, `||`, `==`, `!=`, `>`, quoted strings and the four contexts it may name).
fn guard_admits(workflow: &str, attempt: u64, conclusion: &str) -> bool {
    let doc = YamlLoader::load_from_str(workflow)
        .expect("qa: win-retry.yml parses")
        .remove(0);
    let cond = doc["jobs"]["win-retry"]["if"]
        .as_str()
        .expect("qa: win-retry has a guard")
        .replace("${{", "")
        .replace("}}", "")
        .replace("&&", " and ")
        .replace("||", " or ")
        .replace("github.event.workflow_run.run_attempt", "attempt")
        .replace("github.event.workflow_run.conclusion", "conclusion")
        .replace("github.repository", "repo");
    let out = Command::new("python3")
        .arg("-I")
        .arg("-c")
        .arg("import sys; print(eval(sys.argv[1], {}, {'repo': 'Ma1achy/Principia', 'attempt': int(sys.argv[2]), 'conclusion': sys.argv[3]}))")
        .arg(&cond)
        .arg(attempt.to_string())
        .arg(conclusion)
        .timed_output()
        .expect("qa: python3 ran");
    assert!(
        out.status.success(),
        "qa: win-retry's guard does not evaluate ({cond}): {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim() == "True"
}

fn check_guard(workflow: &str) {
    assert!(
        guard_admits(workflow, 1, "failure"),
        "qa: win-retry passes over a failed first attempt"
    );
    assert!(
        guard_admits(workflow, 1, "success"),
        "qa: win-retry passes over a passing first attempt, its step limits unlogged"
    );
    assert!(
        guard_admits(workflow, 2, "success"),
        "qa: win-retry passes over a passing re-run, its step limits unlogged"
    );
}

#[test]
fn qa_win_retry_reads_a_passing_run() {
    check_guard(&std::fs::read_to_string(root().join(WIN_RETRY)).expect("qa: read win-retry.yml"));
}

negative_control!(
    qa_win_retry_reads_a_passing_run,
    "the guard that read failed first attempts alone must fail",
    expected = "qa: win-retry passes over a passing first attempt, its step limits unlogged",
    {
        let text = std::fs::read_to_string(root().join(WIN_RETRY)).expect("qa: read win-retry.yml");
        let from = "github.event.workflow_run.conclusion != 'skipped'";
        assert!(
            text.contains(from),
            "qa control: {from:?} is not in win-retry.yml"
        );
        check_guard(&text.replacen(
            from,
            "(github.event.workflow_run.conclusion != 'success' && github.event.workflow_run.conclusion != 'skipped')",
            1,
        ));
    }
);
