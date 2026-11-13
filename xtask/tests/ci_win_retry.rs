//! `.github/scripts/win_retry.py` and `.github/workflows/win-retry.yml`: a job on the Windows PC that stops on a
//! runner-level fault is re-run once, automatically, by a hosted job; a hang, a run cancelled by a newer one, a job no
//! runner took and a failure of the job's own are logged and never re-run (the owner's goal of 12 Nov 2026, item 5,
//! R-988: "Unclear stop: one automatic re-run; hang: cancelled by time limit; both logged"; R-714's classifier, "a
//! script with tests against saved logs").
//!
//! The runs are real, trimmed, under `fixtures/win_retry/` (`fixtures/README.md`): a test failure on `main`
//! (43316207795), a job ended by its own time limit (43146846084), steps ended by theirs (43225140437), a job a newer run
//! cancelled before any runner took it (42751477363), and a nightly whose three jobs failed on their own, one cancelled
//! by hand (43023140554). No run on the PC has yet lost its runner or been shut down mid-job, so those cases edit the
//! saved test failure in the one respect they turn on: its annotation, GitHub's text for a runner gone, and no log.
//!
//! - `ci_win_retry_reruns_*`: a runner lost and a runner shut down are re-run once, that job alone;
//! - `ci_win_retry_keeps_*`: a test failure (one the runner's going follows in a later step too), a hang (the job's
//!   limit and a step's, and a step's in a job that passed on from it, which is logged too), a run cancelled by a newer one, a job no runner took, a second attempt, a fault once a newer
//!   run of it has started, and a run of a pull request or of a push to another branch are not;
//! - `ci_win_retry_a_limit_*`: a step's limit the job passed on from is a row of its own beside the job's verdict (a
//!   runner lost after it re-run once, a failure of its own after it `own`), and a limit that stopped the job a hang;
//! - `ci_win_retry_logs_every_time_limit_once`: every distinct limit in a job, stopping or passed on from, is a row of
//!   its own, once, in a hung job and a passing one;
//! - `ci_win_retry_mixed_runs`: one fault beside failures of their own is re-run alone; two faults alone are re-run
//!   together; two beside a failure of its own are not;
//! - `ci_win_retry_decide_*`: `decide`, under a stand-in `gh`, asks for the one re-run and writes every unclear stop and
//!   hang to the step summary, with the run, the job, the step, the signature and the re-run's id, and a re-run says it
//!   is one; a passing run's step that reached its limit is logged and not re-run;
//! - `ci_win_retry_workflow_*`: `win-retry.yml`, parsed: `workflow_run` on `win-gpu` and `runner-proof` alone, hosted,
//!   the least permissions, the repository guard, a passing run read, and `main`'s checkout.
//!
//! Each test has its negative control (R-176). Unix only: `decide`'s stand-in `gh` is a shell script.
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};
use validation::negative_control;
use validation::spawn::{write_executable, Spawn};
use yaml_rust2::{Yaml, YamlLoader};

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;

const SCRIPT: &str = ".github/scripts/win_retry.py";
const WORKFLOW: &str = ".github/workflows/win-retry.yml";
/// `main`, a push: `win-vulkan` failed its own tests.
const TEST_RUN: &str = "43316207795";
const TEST_JOB: &str = "131524774857";
/// A dispatch: `wsl-register` ended by its own limit of an hour.
const JOB_LIMIT_RUN: &str = "43146846084";
/// A dispatch's second attempt: `wsl-register`'s two downloads each ended by its own limit.
const STEP_LIMIT_RUN: &str = "43225140437";
const STEP_LIMIT_JOB: &str = "131274911991";
/// That run's first download's limit, GitHub's annotation for it: a step that `continue-on-error` lets the job pass on
/// from.
const FIRST_DOWNLOAD_LIMIT: &str = "The action 'This run's image' has timed out after 25 minutes.";
/// That run's second download.
const SECOND_DOWNLOAD: &str = "This run's image, again if the first download didn't finish whole";
/// The verdict for a step's time limit in a job that passed on from it.
const RECOVERED: &str = "hang, a step ended by its time limit, the job passing on: not re-run";
/// `main`, a push: `win-vulkan` cancelled, waiting, by a newer run.
const NEWER_RUN: &str = "42751477363";
const NEWER_JOB: &str = "129615866444";
/// The schedule: `win-vulkan` and `win-self-check` failed their own steps; `win-tests` was cancelled by hand.
const NIGHTLY_RUN: &str = "43023140554";
const NIGHTLY_VULKAN: &str = "130531059475";
const NIGHTLY_CHECK: &str = "130531059678";
/// GitHub's annotation for a self-hosted runner gone mid-job; no run on the PC has shown it yet.
const LOST: &str = "The self-hosted runner: malachys-pc-rtx2080s lost communication with the server. Verify the \
                    machine is running and has a healthy network connection. Anything in your workflow that \
                    terminates the runner process, starves it for CPU/Memory, or blocks its network access can cause \
                    this error.";
/// The runner's own text when its service stops mid-job (`actions/runner`, `src/Runner.Worker/JobRunner.cs`).
const SHUTDOWN: &str = "The runner has received a shutdown signal. This can happen when the runner service is stopped, \
                        or a manually started runner is canceled.";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the workspace root")
        .to_path_buf()
}

fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path))
        .unwrap_or_else(|e| panic!("ci_win_retry: {path}: {e}"))
}

fn script() -> String {
    read(SCRIPT)
}

/// `text` with `from` replaced by `to`, which must be in it: a control's one edit.
#[cfg(feature = "controls")]
fn edit(text: &str, from: &str, to: &str) -> String {
    assert!(
        text.contains(from),
        "ci_win_retry: a control's edit is gone"
    );
    text.replacen(from, to, 1)
}

/// A run as the classifier reads it.
#[derive(Clone)]
struct Run {
    run: Value,
    jobs: Vec<Value>,
    annotations: Vec<(String, String)>,
    logs: Vec<(String, String)>,
    /// The newer runs of the workflow, each `{"id", "jobs"}`; `None` when they could not be read.
    newer: Option<Value>,
}

impl Run {
    /// The saved run `id`, read whole, no newer run of it known to have started.
    fn saved(id: &str) -> Run {
        let dir = root().join("fixtures/win_retry").join(id);
        let json = |p: &Path| -> Value {
            let text = std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{p:?}: {e}"));
            serde_json::from_str(&text).unwrap_or_else(|e| panic!("{p:?}: {e}"))
        };
        let folder = |name: &str| -> Vec<(String, String)> {
            let Ok(entries) = std::fs::read_dir(dir.join(name)) else {
                return Vec::new();
            };
            let mut out: Vec<(String, String)> = entries
                .map(|e| e.expect("entry").path())
                .map(|p| {
                    let id = p.file_stem().expect("stem").to_string_lossy().into_owned();
                    (id, std::fs::read_to_string(&p).expect("a saved file"))
                })
                .collect();
            out.sort();
            out
        };
        Run {
            run: json(&dir.join("run.json")),
            jobs: json(&dir.join("jobs.json"))
                .as_array()
                .expect("jobs")
                .clone(),
            annotations: folder("annotations"),
            logs: folder("logs"),
            newer: Some(json!([])),
        }
    }

    fn job(&mut self, id: &str) -> &mut Value {
        self.jobs
            .iter_mut()
            .find(|j| j["id"].as_u64() == id.parse().ok())
            .unwrap_or_else(|| panic!("ci_win_retry: no job {id}"))
    }

    /// The job `id`'s only annotation, a failure saying `message`, and no log, as a runner that went leaves it.
    fn gone(self, id: &str, message: &str) -> Run {
        self.annotated(id, &[message])
    }

    /// The job `id`'s annotations, failures saying `messages`, and no log.
    fn annotated(mut self, id: &str, messages: &[&str]) -> Run {
        let listed: Vec<Value> = messages
            .iter()
            .map(|m| json!({"annotation_level": "failure", "message": m}))
            .collect();
        self.annotations.retain(|(k, _)| k != id);
        self.annotations
            .push((id.to_owned(), Value::from(listed).to_string()));
        self.logs.retain(|(k, _)| k != id);
        self
    }

    /// The job `id`'s saved annotations and log kept, and one more failure annotation saying `message` after them.
    fn appended(mut self, id: &str, message: &str) -> Run {
        let at = self.annotations.iter().position(|(k, _)| k == id);
        let mut listed: Vec<Value> = at
            .map(|i| serde_json::from_str(&self.annotations[i].1).expect("saved annotations"))
            .unwrap_or_default();
        listed.push(json!({"annotation_level": "failure", "message": message}));
        let text = Value::from(listed).to_string();
        match at {
            Some(i) => self.annotations[i].1 = text,
            None => self.annotations.push((id.to_owned(), text)),
        }
        self
    }

    /// The job `id` with no annotation and no log.
    fn bare(mut self, id: &str) -> Run {
        self.annotations.retain(|(k, _)| k != id);
        self.logs.retain(|(k, _)| k != id);
        self
    }

    /// The job `id`'s steps named `name` concluding `conclusion`.
    fn step(mut self, id: &str, name: &str, conclusion: &str) -> Run {
        let steps = self.job(id)["steps"].as_array_mut().expect("steps");
        let mut found = false;
        for step in steps.iter_mut().filter(|s| s["name"] == name) {
            step["conclusion"] = json!(conclusion);
            found = true;
        }
        assert!(found, "ci_win_retry: no step {name}");
        self
    }

    fn attempt(mut self, n: u32) -> Run {
        self.run["run_attempt"] = json!(n);
        self
    }

    fn on(mut self, event: &str, branch: &str) -> Run {
        self.run["event"] = json!(event);
        self.run["head_branch"] = json!(branch);
        self
    }

    /// A newer run of the workflow, `id`, whose job `name` ended `conclusion` (`null` while it waits or runs).
    fn newer_run(mut self, id: u64, name: &str, conclusion: Value) -> Run {
        let mut runs = self.newer.take().unwrap_or_else(|| json!([]));
        runs.as_array_mut()
            .expect("runs")
            .push(json!({"id": id, "jobs": [{"name": name, "conclusion": conclusion}]}));
        self.newer = Some(runs);
        self
    }

    fn newer_unread(mut self) -> Run {
        self.newer = None;
        self
    }

    fn write(&self, dir: &Path) {
        std::fs::create_dir_all(dir.join("annotations")).expect("annotations");
        std::fs::create_dir_all(dir.join("logs")).expect("logs");
        std::fs::write(dir.join("run.json"), self.run.to_string()).expect("run");
        std::fs::write(
            dir.join("jobs.json"),
            Value::from(self.jobs.clone()).to_string(),
        )
        .expect("jobs");
        for (id, text) in &self.annotations {
            std::fs::write(dir.join("annotations").join(format!("{id}.json")), text)
                .expect("annotations");
        }
        for (id, text) in &self.logs {
            std::fs::write(dir.join("logs").join(format!("{id}.log")), text).expect("log");
        }
        if let Some(newer) = &self.newer {
            std::fs::write(dir.join("newer.json"), newer.to_string()).expect("newer");
        }
    }
}

/// The saved test failure, its runner lost mid-job.
fn lost() -> Run {
    Run::saved(TEST_RUN).gone(TEST_JOB, LOST)
}

/// The saved step limits' run as it would read had the second download finished: the first download ended by its limit,
/// its annotation kept, every step, the job and the run passing, at the first attempt.
fn recovered() -> Run {
    let mut run = Run::saved(STEP_LIMIT_RUN)
        .attempt(1)
        .annotated(STEP_LIMIT_JOB, &[FIRST_DOWNLOAD_LIMIT]);
    let job = run.job(STEP_LIMIT_JOB);
    job["conclusion"] = json!("success");
    for step in job["steps"].as_array_mut().expect("steps") {
        if step["conclusion"] == "failure" {
            step["conclusion"] = json!("success");
        }
    }
    run.run["conclusion"] = json!("success");
    run
}

/// `run`, classified by `script`: what it printed.
fn classified(script: &str, run: &Run) -> String {
    let dir = Scratch::new("ci_win_retry_classify");
    std::fs::create_dir_all(&*dir).expect("scratch folder");
    let file = dir.join("win_retry.py");
    std::fs::write(&file, script).expect("script written");
    let data = dir.join("run");
    run.write(&data);
    let out = Command::new("python3")
        .arg("-I")
        .arg(&file)
        .arg("classify")
        .arg(&data)
        .timed_output()
        .expect("ci_win_retry: python3 ran");
    assert!(out.status.success(), "ci_win_retry: the classifier failed");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// What `script` asks of `run`: its first line, `rerun=…`.
fn rerun(script: &str, run: &Run) -> String {
    classified(script, run)
        .lines()
        .next()
        .expect("ci_win_retry: a verdict")
        .to_owned()
}

/// Asserts `script` re-runs nothing of `run`, and that its line for `job` gives `kind`.
fn check_kept(script: &str, case: &str, run: &Run, job: &str, kind: &str) {
    let text = classified(script, run);
    assert!(text.starts_with("rerun=none\n"), "{case}: re-run");
    assert!(
        text.lines()
            .any(|l| l.starts_with(&format!("job={job}: {kind}"))),
        "{case}: not logged as {kind}"
    );
}

// ---- A runner-level fault is re-run once ------------------------------------------------------------------------------

fn check_lost(script: &str) {
    assert_eq!(
        rerun(script, &lost()),
        format!("rerun=job {TEST_JOB}"),
        "a runner lost mid-job: not re-run"
    );
    assert_eq!(
        rerun(script, &lost().on("schedule", "main")),
        format!("rerun=job {TEST_JOB}"),
        "a runner lost mid-job on a nightly: not re-run"
    );
    assert_eq!(
        rerun(script, &lost().on("workflow_dispatch", "task/x")),
        format!("rerun=job {TEST_JOB}"),
        "a runner lost mid-job on a dispatch: not re-run"
    );
    let text = classified(script, &lost());
    assert!(
        text.lines()
            .any(|l| l == format!("evidence=win-vulkan: {LOST}")),
        "a runner lost mid-job: the line it rests on is not logged"
    );
}

#[test]
fn ci_win_retry_reruns_a_runner_lost_mid_job() {
    check_lost(&script());
}

negative_control!(
    ci_win_retry_reruns_a_runner_lost_mid_job,
    "a classifier without the lost runner's signature",
    expected = "a runner lost mid-job: not re-run",
    check_lost(&edit(
        &script(),
        "lost communication with the server\")",
        "lost touch with the server\")"
    ))
);

fn check_shutdown(script: &str) {
    assert_eq!(
        rerun(script, &Run::saved(TEST_RUN).gone(TEST_JOB, SHUTDOWN)),
        format!("rerun=job {TEST_JOB}"),
        "a runner shut down mid-job: not re-run"
    );
}

#[test]
fn ci_win_retry_reruns_a_runner_shut_down_mid_job() {
    check_shutdown(&script());
}

negative_control!(
    ci_win_retry_reruns_a_runner_shut_down_mid_job,
    "a classifier without the shutdown signal",
    expected = "a runner shut down mid-job: not re-run",
    check_shutdown(&edit(
        &script(),
        "re.compile(r\"The runner has received a shutdown signal\")",
        "re.compile(r\"The runner has received a stop signal\")"
    ))
);

// ---- What is never re-run -------------------------------------------------------------------------------------------

fn check_test_failure(script: &str) {
    check_kept(
        script,
        "a test failure",
        &Run::saved(TEST_RUN),
        "win-vulkan",
        "own",
    );
}

#[test]
fn ci_win_retry_keeps_a_test_failure() {
    check_test_failure(&script());
}

negative_control!(
    ci_win_retry_keeps_a_test_failure,
    "a classifier that reads a step's exit as a runner fault",
    expected = "a test failure: re-run",
    check_test_failure(&edit(
        &script(),
        "    (\"the runner received a shutdown signal\"",
        "    (\"a step's exit\", re.compile(r\"Process completed with exit code\")),\n    \
         (\"the runner received a shutdown signal\""
    ))
);

fn check_failure_then_fault(script: &str) {
    // The saved test failure (four steps failed on their own output, the job running on past each), then the runner
    // shut down or lost in a later step: its failures stand, and it is not re-run (R-714).
    check_kept(
        script,
        "a test failure, then a shutdown",
        &Run::saved(TEST_RUN).appended(TEST_JOB, SHUTDOWN),
        "win-vulkan",
        "own at step 'cargo nextest run -p validation -p xtask -p engine'",
    );
    check_kept(
        script,
        "a test failure, then a lost runner",
        &Run::saved(TEST_RUN).appended(TEST_JOB, LOST),
        "win-vulkan",
        "own",
    );
    // The failure in the log alone, the annotations the runner's only.
    let mut logged = Run::saved(TEST_RUN);
    logged.annotations.retain(|(k, _)| k != TEST_JOB);
    let logged = logged.appended(TEST_JOB, SHUTDOWN);
    check_kept(
        script,
        "a test failure in the log, then a shutdown",
        &logged,
        "win-vulkan",
        "own",
    );
    // A cancel's line or an empty one beside the runner's going is no failure of the job's own: still re-run.
    let cut = Run::saved(TEST_RUN).annotated(TEST_JOB, &["The operation was canceled.", SHUTDOWN]);
    assert_eq!(
        rerun(script, &cut),
        format!("rerun=job {TEST_JOB}"),
        "a shutdown beside a cancel's line: not re-run"
    );
    let blank = Run::saved(TEST_RUN).annotated(TEST_JOB, &["", LOST]);
    assert_eq!(
        rerun(script, &blank),
        format!("rerun=job {TEST_JOB}"),
        "a lost runner beside an empty line: not re-run"
    );
}

#[test]
fn ci_win_retry_keeps_a_test_failure_before_a_runner_fault() {
    check_failure_then_fault(&script());
}

negative_control!(
    ci_win_retry_keeps_a_test_failure_before_a_runner_fault,
    "a classifier that lets the runner's going outrank a failure before it",
    expected = "a test failure, then a shutdown: re-run",
    check_failure_then_fault(&edit(
        &script(),
        "            if own:\n",
        "            if False:\n"
    ))
);

mod failure_then_fault_cancel {
    use super::*;
    negative_control!(
        ci_win_retry_keeps_a_test_failure_before_a_runner_fault,
        "a classifier that reads a cancel's line as a failure of the job's own",
        expected = "a shutdown beside a cancel's line: not re-run",
        check_failure_then_fault(&edit(&script(), " and t.strip() != CANCELED]", "]"))
    );
}

mod failure_then_fault_blank {
    use super::*;
    negative_control!(
        ci_win_retry_keeps_a_test_failure_before_a_runner_fault,
        "a classifier that reads an empty line as a failure of the job's own",
        expected = "a lost runner beside an empty line: not re-run",
        check_failure_then_fault(&edit(
            &script(),
            "own = [t for t in texts if t.strip() and not signed(t)",
            "own = [t for t in texts if not signed(t)"
        ))
    );
}

fn check_hangs(script: &str) {
    check_kept(
        script,
        "a job at its limit",
        &Run::saved(JOB_LIMIT_RUN),
        "wsl-register",
        "hang at step 'This run's image'",
    );
    // The saved step limits' run, its second download stopping the job at its limit rather than passing on.
    check_kept(
        script,
        "steps at their limits",
        &Run::saved(STEP_LIMIT_RUN)
            .attempt(1)
            .step(STEP_LIMIT_JOB, SECOND_DOWNLOAD, "cancelled"),
        "wsl-register",
        &format!("hang at step '{SECOND_DOWNLOAD}': hang, ended by its time limit"),
    );
    // A hang beside a runner's signature is still a hang.
    let both = Run::saved(TEST_RUN).annotated(
        TEST_JOB,
        &[
            "The job has exceeded the maximum execution time of 2h36m0s",
            LOST,
        ],
    );
    check_kept(
        script,
        "a hang beside a lost runner",
        &both,
        "win-vulkan",
        "hang",
    );
    // A job whose state says it timed out.
    let mut timed = Run::saved(TEST_RUN).bare(TEST_JOB);
    timed.job(TEST_JOB)["conclusion"] = json!("timed_out");
    check_kept(script, "a job that timed out", &timed, "win-vulkan", "hang");
}

#[test]
fn ci_win_retry_keeps_a_hang() {
    check_hangs(&script());
}

negative_control!(
    ci_win_retry_keeps_a_hang,
    "a classifier that reads a lost runner before a time limit",
    expected = "a hang beside a lost runner: re-run",
    check_hangs(&edit(
        &script(),
        "    for text in texts:\n        m = TIME_LIMIT.search(text)",
        "    for text in texts:\n        if any(p.search(text) for _, p in SIGNATURES):\n            \
         return \"fault\", None, text\n    for text in texts:\n        m = TIME_LIMIT.search(text)"
    ))
);

mod hang_by_quote {
    use super::*;
    negative_control!(
        ci_win_retry_keeps_a_hang,
        "a step's limit unread when its name holds a quote",
        expected = "steps at their limits: not logged as hang",
        check_hangs(&edit(
            &script(),
            "TIME_LIMIT = re.compile(r\"The action '(.*)' has timed out",
            "TIME_LIMIT = re.compile(r\"The action '([^']*)' has timed out"
        ))
    );
}

fn check_recovered(script: &str) {
    let text = classified(script, &recovered());
    assert!(
        text.starts_with("rerun=none\n"),
        "a recovered step limit: re-run"
    );
    assert!(
        text.lines().any(|l| l
            == format!("job=wsl-register: hang at step 'This run's image': {RECOVERED}")),
        "a recovered step limit: not logged as a hang"
    );
    assert!(
        text.lines()
            .any(|l| l == format!("evidence=wsl-register: {FIRST_DOWNLOAD_LIMIT}")),
        "a recovered step limit: the line it rests on is not logged"
    );
    // A passing job with no limit reached logs nothing.
    let clean = classified(script, &recovered().bare(STEP_LIMIT_JOB));
    assert!(
        !clean.lines().any(|l| l.starts_with("job=")),
        "a passing job with no limit reached: logged"
    );
    // A recovered limit beside a fault leaves the fault to re-run alone, and is logged beside it.
    let mut beside = lost();
    beside.jobs.push(json!({
        "id": 1, "name": "wsl-proofs", "status": "completed", "conclusion": "success",
        "runner_name": "malachys-pc-rtx2080s", "labels": ["self-hosted", "windows", "nvidia"], "steps": []
    }));
    let beside = beside.appended("1", FIRST_DOWNLOAD_LIMIT);
    let text = classified(script, &beside);
    assert!(
        text.starts_with(&format!("rerun=job {TEST_JOB}\n")),
        "a fault beside a recovered step limit: not re-run alone"
    );
    assert!(
        text.lines()
            .any(|l| l.starts_with("job=wsl-proofs: hang at step 'This run's image'")),
        "a recovered step limit beside a fault: not logged"
    );
}

#[test]
fn ci_win_retry_keeps_a_step_limit_the_job_passed_on_from() {
    check_recovered(&script());
}

negative_control!(
    ci_win_retry_keeps_a_step_limit_the_job_passed_on_from,
    "a passing job's step limit unread",
    expected = "a recovered step limit: not logged as a hang",
    check_recovered(&edit(
        &script(),
        "        for step, evidence in recovered:\n",
        "        for step, evidence in []:\n"
    ))
);

mod recovered_as_fault {
    use super::*;
    negative_control!(
        ci_win_retry_keeps_a_step_limit_the_job_passed_on_from,
        "a passing job's step limit read as a fault",
        expected = "a recovered step limit: re-run",
        check_recovered(&edit(
            &script(),
            "rows.append([job[\"name\"], jid, \"hang\", step, evidence, RECOVERED])",
            "rows.append([job[\"name\"], jid, \"fault\", step, evidence, RECOVERED])"
        ))
    );
}

// ---- A step's limit the job passed on from is never the job's verdict -------------------------------------------------

/// The `job=wsl-register: …` lines of `text`.
fn register_rows(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|l| l.starts_with("job=wsl-register: "))
        .collect()
}

/// The saved step limits' run at its first attempt, its job's lines `messages` alone, no log.
fn limits_then(messages: &[&str]) -> Run {
    Run::saved(STEP_LIMIT_RUN)
        .attempt(1)
        .annotated(STEP_LIMIT_JOB, messages)
}

fn check_passed_on(script: &str) {
    let first_row = format!("job=wsl-register: hang at step 'This run's image': {RECOVERED}");
    // The saved run as it was: both downloads at their limits, passed on from, then the job failing its own check of
    // the image. Its verdict is its own failure, and each limit is a row of its own, once, though both the annotations
    // and the log carry it.
    let text = classified(script, &Run::saved(STEP_LIMIT_RUN).attempt(1));
    assert!(
        text.starts_with("rerun=none\n"),
        "a failure of its own after recovered limits: re-run"
    );
    let rows = register_rows(&text);
    assert_eq!(
        rows.first().copied(),
        Some(
            "job=wsl-register: own at step 'This run's image, never downloaded whole': a failure of the job's own: \
             not re-run"
        ),
        "a failure of its own after recovered limits: not its verdict"
    );
    assert_eq!(
        rows.len(),
        3,
        "a failure of its own after recovered limits: a limit logged more than once, or not at all"
    );
    assert!(
        rows.contains(&first_row.as_str())
            && rows.contains(
                &format!("job=wsl-register: hang at step '{SECOND_DOWNLOAD}': {RECOVERED}")
                    .as_str()
            ),
        "a failure of its own after recovered limits: a recovered limit not logged"
    );
    assert!(
        text.lines()
            .any(|l| l == "evidence=wsl-register: Process completed with exit code 1."),
        "a failure of its own after recovered limits: its own line not its evidence"
    );
    // A recovered limit, then the runner lost: a fault, re-run once, the limit logged beside it.
    let text = classified(script, &limits_then(&[FIRST_DOWNLOAD_LIMIT, LOST]));
    assert!(
        text.starts_with(&format!("rerun=job {STEP_LIMIT_JOB}\n")),
        "a lost runner after a recovered limit: not re-run"
    );
    let rows = register_rows(&text);
    assert_eq!(
        rows.first().copied(),
        Some(
            "job=wsl-register: fault at step 'This run's image, never downloaded whole': unclear stop, a \
             runner-level fault: re-run once"
        ),
        "a lost runner after a recovered limit: not its verdict"
    );
    assert_eq!(
        rows.get(1..),
        Some(&[first_row.as_str()][..]),
        "a lost runner after a recovered limit: the limit not logged beside it"
    );
    // A shutdown alike.
    assert_eq!(
        rerun(script, &limits_then(&[FIRST_DOWNLOAD_LIMIT, SHUTDOWN])),
        format!("rerun=job {STEP_LIMIT_JOB}"),
        "a shutdown after a recovered limit: not re-run"
    );
}

#[test]
fn ci_win_retry_a_limit_passed_on_from_is_never_the_verdict() {
    check_passed_on(&script());
}

negative_control!(
    ci_win_retry_a_limit_passed_on_from_is_never_the_verdict,
    "a failing job's every step limit read as its stop",
    expected = "a failure of its own after recovered limits: not its verdict",
    check_passed_on(&edit(
        &script(),
        "if m and (passed or passed_on(job, m.group(1))):",
        "if m and passed:"
    ))
);

mod passed_on_unlogged {
    use super::*;
    negative_control!(
        ci_win_retry_a_limit_passed_on_from_is_never_the_verdict,
        "a failing job's recovered limits left out of the log",
        expected = "a failure of its own after recovered limits: a limit logged more than once, or not at all",
        check_passed_on(&edit(
            &script(),
            "        for step, evidence in recovered:\n",
            "        for step, evidence in (recovered if job.get(\"conclusion\") in PASSED else []):\n"
        ))
    );
}

mod passed_on_twice {
    use super::*;
    negative_control!(
        ci_win_retry_a_limit_passed_on_from_is_never_the_verdict,
        "a limit logged once per line that carries it",
        expected = "a failure of its own after recovered limits: a limit logged more than once, or not at all",
        check_passed_on(&edit(
            &script(),
            "            if limit not in found:\n",
            "            if True:\n"
        ))
    );
}

mod passed_on_in_the_verdict {
    use super::*;
    negative_control!(
        ci_win_retry_a_limit_passed_on_from_is_never_the_verdict,
        "a failing job's verdict read from every line, recovered limits among them",
        expected = "a failure of its own after recovered limits: not its verdict",
        check_passed_on(&edit(
            &script(),
            "kind, step, evidence = kind_of(job, texts)",
            "kind, step, evidence = kind_of(job, lines(annotations.get(jid, []), logs.get(jid)))"
        ))
    );
}

fn check_stopping_limit(script: &str) {
    // The first download stopping the job at its limit, then the runner lost: a hang, never re-run.
    let stopped = limits_then(&[FIRST_DOWNLOAD_LIMIT, LOST]).step(
        STEP_LIMIT_JOB,
        "This run's image",
        "cancelled",
    );
    check_kept(
        script,
        "a stopping step limit",
        &stopped,
        "wsl-register",
        "hang at step 'This run's image': hang, ended by its time limit: not re-run",
    );
    assert_eq!(
        register_rows(&classified(script, &stopped)).len(),
        1,
        "a stopping step limit: logged as passed on from too"
    );
    // A limit naming no step of the job is read as its stop.
    let unnamed = limits_then(&[
        "The action 'A step this job lacks' has timed out after 25 minutes.",
        LOST,
    ]);
    check_kept(
        script,
        "a limit on a step the job doesn't name",
        &unnamed,
        "wsl-register",
        "hang at step 'A step this job lacks'",
    );
    // Two steps of the name, one passed on from and one stopped: a stop.
    let mut twice = limits_then(&[FIRST_DOWNLOAD_LIMIT, LOST]);
    let steps = twice.job(STEP_LIMIT_JOB)["steps"]
        .as_array_mut()
        .expect("steps");
    steps.push(json!({"number": 99, "name": "This run's image", "status": "completed", "conclusion": "cancelled"}));
    check_kept(
        script,
        "a limit on a step of a repeated name, one stopped",
        &twice,
        "wsl-register",
        "hang at step 'This run's image'",
    );
    // A recovered limit, then the job ended at its own: a hang.
    let mut timed = limits_then(&[FIRST_DOWNLOAD_LIMIT, LOST]);
    timed.job(STEP_LIMIT_JOB)["conclusion"] = json!("timed_out");
    check_kept(
        script,
        "a job that timed out after a recovered limit",
        &timed,
        "wsl-register",
        "hang",
    );
}

#[test]
fn ci_win_retry_a_limit_that_stops_the_job_is_a_hang() {
    check_stopping_limit(&script());
}

negative_control!(
    ci_win_retry_a_limit_that_stops_the_job_is_a_hang,
    "a limit read as passed on from whatever its step's conclusion",
    expected = "a stopping step limit: re-run",
    check_stopping_limit(&edit(
        &script(),
        "all(s.get(\"conclusion\") == \"success\" for s in named)",
        "True"
    ))
);

mod stopping_unnamed {
    use super::*;
    negative_control!(
        ci_win_retry_a_limit_that_stops_the_job_is_a_hang,
        "a limit on no step of the job read as passed on from",
        expected = "a limit on a step the job doesn't name: re-run",
        check_stopping_limit(&edit(
            &script(),
            "return bool(named) and all(",
            "return all("
        ))
    );
}

mod stopping_repeated {
    use super::*;
    negative_control!(
        ci_win_retry_a_limit_that_stops_the_job_is_a_hang,
        "one passing step of the name enough to pass on",
        expected = "a limit on a step of a repeated name, one stopped: re-run",
        check_stopping_limit(&edit(
            &script(),
            "all(s.get(\"conclusion\") == \"success\" for s in named)",
            "any(s.get(\"conclusion\") == \"success\" for s in named)"
        ))
    );
}

mod stopping_timed_out {
    use super::*;
    negative_control!(
        ci_win_retry_a_limit_that_stops_the_job_is_a_hang,
        "a job's timed_out conclusion unread",
        expected = "a job that timed out after a recovered limit: re-run",
        check_stopping_limit(&edit(
            &script(),
            "    if job.get(\"conclusion\") == \"timed_out\":",
            "    if False:"
        ))
    );
}

/// The verdict for a time limit that stopped the job.
const STOPPED: &str = "hang, ended by its time limit: not re-run";

fn check_every_limit(script: &str) {
    let second_limit = format!("The action '{SECOND_DOWNLOAD}' has timed out after 25 minutes.");
    let first_hang = format!("job=wsl-register: hang at step 'This run's image': {STOPPED}");
    let second_hang = format!("job=wsl-register: hang at step '{SECOND_DOWNLOAD}': {STOPPED}");
    // The saved run, both downloads stopping the job at their limits, each carried by an annotation and a log line:
    // a hang, never re-run, each limit a row of its own, once.
    let both = Run::saved(STEP_LIMIT_RUN)
        .attempt(1)
        .step(STEP_LIMIT_JOB, "This run's image", "cancelled")
        .step(STEP_LIMIT_JOB, SECOND_DOWNLOAD, "cancelled");
    let text = classified(script, &both);
    assert!(
        text.starts_with("rerun=none\n"),
        "two stopping step limits: re-run"
    );
    let rows = register_rows(&text);
    assert!(
        rows.contains(&first_hang.as_str()) && rows.contains(&second_hang.as_str()),
        "two stopping step limits: a limit not logged"
    );
    assert_eq!(
        rows.len(),
        2,
        "two stopping step limits: a limit logged more than once"
    );
    assert!(
        text.lines()
            .any(|l| l == format!("evidence=wsl-register: {FIRST_DOWNLOAD_LIMIT}"))
            && text
                .lines()
                .any(|l| l == format!("evidence=wsl-register: {second_limit}")),
        "two stopping step limits: a line it rests on not logged"
    );
    // The saved run, the first download passed on from and the second stopping the job: the stop the verdict, the
    // recovered limit a row beside it.
    let text = classified(
        script,
        &Run::saved(STEP_LIMIT_RUN)
            .attempt(1)
            .step(STEP_LIMIT_JOB, SECOND_DOWNLOAD, "cancelled"),
    );
    assert_eq!(
        register_rows(&text),
        vec![
            second_hang.as_str(),
            format!("job=wsl-register: hang at step 'This run's image': {RECOVERED}").as_str(),
        ],
        "a stopping limit after a recovered one: a limit not logged, or logged twice"
    );
    // A step's limit stopping the job, then the job's own: two hangs.
    let job_limit = "The job has exceeded the maximum execution time of 1h0m0s";
    let text = classified(
        script,
        &limits_then(&[FIRST_DOWNLOAD_LIMIT, job_limit]).step(
            STEP_LIMIT_JOB,
            "This run's image",
            "cancelled",
        ),
    );
    assert!(
        text.starts_with("rerun=none\n"),
        "a step's limit, then the job's: re-run"
    );
    let rows = register_rows(&text);
    assert_eq!(
        rows.len(),
        2,
        "a step's limit, then the job's: the job's limit not logged"
    );
    assert!(
        text.lines()
            .any(|l| l == format!("evidence=wsl-register: {job_limit}")),
        "a step's limit, then the job's: the job's line not logged"
    );
    // A passing job with two limits it passed on from: each a row, once.
    let text = classified(
        script,
        &recovered().annotated(STEP_LIMIT_JOB, &[FIRST_DOWNLOAD_LIMIT, &second_limit]),
    );
    assert!(
        text.starts_with("rerun=none\n"),
        "two recovered limits in a passing job: re-run"
    );
    assert_eq!(
        register_rows(&text),
        vec![
            format!("job=wsl-register: hang at step 'This run's image': {RECOVERED}").as_str(),
            format!("job=wsl-register: hang at step '{SECOND_DOWNLOAD}': {RECOVERED}").as_str(),
        ],
        "two recovered limits in a passing job: a limit not logged"
    );
}

#[test]
fn ci_win_retry_logs_every_time_limit_once() {
    check_every_limit(&script());
}

negative_control!(
    ci_win_retry_logs_every_time_limit_once,
    "a hung job's first limit logged alone",
    expected = "two stopping step limits: a limit not logged",
    check_every_limit(&edit(
        &script(),
        "for other in limits(job, texts):",
        "for other in []:"
    ))
);

mod every_limit_twice {
    use super::*;
    negative_control!(
        ci_win_retry_logs_every_time_limit_once,
        "a stopping limit logged once per line that carries it",
        expected = "two stopping step limits: a limit logged more than once",
        check_every_limit(&edit(
            &script(),
            "            if limit not in seen:\n",
            "            if True:\n"
        ))
    );
}

mod every_limit_verdict_twice {
    use super::*;
    negative_control!(
        ci_win_retry_logs_every_time_limit_once,
        "the verdict's own limit logged again beside it",
        expected = "two stopping step limits: a limit logged more than once",
        check_every_limit(&edit(
            &script(),
            "if other != (step, evidence):",
            "if True:"
        ))
    );
}

mod every_limit_recovered_beside_a_hang {
    use super::*;
    negative_control!(
        ci_win_retry_logs_every_time_limit_once,
        "a recovered limit unlogged beside a hung job's stop",
        expected = "a stopping limit after a recovered one: a limit not logged, or logged twice",
        check_every_limit(&edit(
            &script(),
            "        for step, evidence in recovered:\n",
            "        for step, evidence in (recovered if job.get(\"conclusion\") in PASSED else []):\n"
        ))
    );
}

mod every_limit_job_own {
    use super::*;
    negative_control!(
        ci_win_retry_logs_every_time_limit_once,
        "the job's own limit unread",
        expected = "a step's limit, then the job's: the job's limit not logged",
        check_every_limit(&edit(
            &script(),
            "|has exceeded the maximum execution time\")",
            "\")"
        ))
    );
}

mod every_limit_recovered_first_alone {
    use super::*;
    negative_control!(
        ci_win_retry_logs_every_time_limit_once,
        "a passing job's first recovered limit logged alone",
        expected = "two recovered limits in a passing job: a limit not logged",
        check_every_limit(&edit(
            &script(),
            "            if limit not in found:\n",
            "            if not found:\n"
        ))
    );
}

fn check_newer(script: &str) {
    check_kept(
        script,
        "a waiting run cancelled by a newer one",
        &Run::saved(NEWER_RUN),
        "win-vulkan",
        "newer",
    );
    // A running one, on the runner, cancelled by a newer one (R-986), with the runner's own line beside it.
    let mut running = Run::saved(TEST_RUN).annotated(
        TEST_JOB,
        &[
            "Canceling since a higher priority waiting request for win-vulkan exists",
            "The operation was canceled.",
            LOST,
        ],
    );
    running.job(TEST_JOB)["conclusion"] = json!("cancelled");
    check_kept(
        script,
        "a running job cancelled by a newer one",
        &running,
        "win-vulkan",
        "newer",
    );
}

#[test]
fn ci_win_retry_keeps_a_run_cancelled_by_a_newer_one() {
    check_newer(&script());
}

negative_control!(
    ci_win_retry_keeps_a_run_cancelled_by_a_newer_one,
    "a classifier that reads the runner's signature before a newer run's cancel",
    expected = "a running job cancelled by a newer one: re-run",
    check_newer(&edit(
        &script(),
        "    for text in texts:\n        if NEWER.search(text):",
        "    for text in texts:\n        if any(p.search(text) for _, p in SIGNATURES):\n            \
         return \"fault\", None, text\n    for text in texts:\n        if NEWER.search(text):"
    ))
);

fn check_untaken(script: &str) {
    check_kept(
        script,
        "a job no runner took",
        &Run::saved(NEWER_RUN).bare(NEWER_JOB),
        "win-vulkan",
        "untaken",
    );
}

#[test]
fn ci_win_retry_keeps_a_job_no_runner_took() {
    check_untaken(&script());
}

negative_control!(
    ci_win_retry_keeps_a_job_no_runner_took,
    "a classifier reading a job no runner took as a fault",
    expected = "a job no runner took: re-run",
    check_untaken(&edit(
        &script(),
        "return \"untaken\", None, \"no runner took the job\"",
        "return \"fault\", None, \"no runner took the job\""
    ))
);

fn check_second_attempt(script: &str) {
    let text = classified(script, &lost().attempt(2));
    assert!(text.starts_with("rerun=none\n"), "a second attempt: re-run");
    assert!(
        text.contains("attempt 2 is a re-run: nothing re-runs it again"),
        "a second attempt: the reason is not given"
    );
}

#[test]
fn ci_win_retry_keeps_a_second_attempt() {
    check_second_attempt(&script());
}

negative_control!(
    ci_win_retry_keeps_a_second_attempt,
    "re-runs up to the third attempt",
    expected = "a second attempt: re-run",
    check_second_attempt(&edit(
        &script(),
        "    if attempt != FIRST:",
        "    if attempt > 2:"
    ))
);

fn check_newer_started(script: &str) {
    for (case, run) in [
        (
            "a newer run waiting",
            lost().newer_run(43316207796, "win-vulkan", Value::Null),
        ),
        (
            "a newer run done",
            lost().newer_run(43316207796, "win-vulkan", json!("failure")),
        ),
        ("newer runs unread", lost().newer_unread()),
    ] {
        assert_eq!(rerun(script, &run), "rerun=none", "{case}: re-run");
    }
    // A newer run that skips the job, or holds only other jobs, leaves it to re-run.
    for (case, run) in [
        (
            "a newer run skipping it",
            lost().newer_run(43316207796, "win-vulkan", json!("skipped")),
        ),
        (
            "a newer run of another job",
            lost().newer_run(43316207796, "win-tests", json!("failure")),
        ),
    ] {
        assert_eq!(
            rerun(script, &run),
            format!("rerun=job {TEST_JOB}"),
            "{case}: not re-run"
        );
    }
}

#[test]
fn ci_win_retry_keeps_a_fault_once_a_newer_run_started() {
    check_newer_started(&script());
}

negative_control!(
    ci_win_retry_keeps_a_fault_once_a_newer_run_started,
    "a re-run while a newer run of the job waits",
    expected = "a newer run waiting: re-run",
    check_newer_started(&edit(
        &script(),
        "if job.get(\"name\") == name and job.get(\"conclusion\") != \"skipped\":",
        "if job.get(\"name\") == name and job.get(\"conclusion\") not in (\"skipped\", None):"
    ))
);

fn check_events(script: &str) {
    for (case, run) in [
        ("a pull request's run", lost().on("pull_request", "task/x")),
        ("a push to a branch", lost().on("push", "task/x")),
        ("a workflow_run", lost().on("workflow_run", "main")),
    ] {
        assert_eq!(rerun(script, &run), "rerun=none", "{case}: re-run");
    }
}

#[test]
fn ci_win_retry_keeps_other_events() {
    check_events(&script());
}

negative_control!(
    ci_win_retry_keeps_other_events,
    "a push to any branch re-run",
    expected = "a push to a branch: re-run",
    check_events(&edit(
        &script(),
        "    elif event == \"push\" and branch != BRANCH:",
        "    elif False:"
    ))
);

// ---- Several jobs -----------------------------------------------------------------------------------------------------

fn check_mixed(script: &str) {
    let one = Run::saved(NIGHTLY_RUN).gone(NIGHTLY_VULKAN, LOST);
    assert_eq!(
        rerun(script, &one),
        format!("rerun=job {NIGHTLY_VULKAN}"),
        "one fault beside failures of their own: not re-run alone"
    );
    let two = one.clone().gone(NIGHTLY_CHECK, SHUTDOWN);
    assert_eq!(
        rerun(script, &two),
        "rerun=none",
        "two faults beside a failure of its own: re-run"
    );
    let mut alone = two;
    alone.jobs.retain(|j| j["name"] != "win-tests");
    assert_eq!(
        rerun(script, &alone),
        "rerun=failed-jobs",
        "two faults alone: not re-run together"
    );
}

#[test]
fn ci_win_retry_mixed_runs() {
    check_mixed(&script());
}

negative_control!(
    ci_win_retry_mixed_runs,
    "every failed job re-run beside a failure of its own",
    expected = "two faults beside a failure of its own: re-run",
    check_mixed(&edit(
        &script(),
        "elif len(ready) > 1 and len(ready) == len(faults) and not others:",
        "elif len(ready) > 1 and len(ready) == len(faults):"
    ))
);

// ---- decide -----------------------------------------------------------------------------------------------------------

/// What `decide` did.
struct Decided {
    ok: bool,
    calls: Vec<String>,
    summary: String,
}

impl Decided {
    fn posts(&self) -> Vec<&String> {
        self.calls
            .iter()
            .filter(|c| c.contains("--method POST"))
            .collect()
    }
}

/// `decide` under `script`, on `run`, with a stand-in `gh` serving it as GitHub does: the attempt's jobs, each job's
/// annotations and log (a 404 for none), the workflow's runs (the newer ones first, then this one) and each newer run's
/// jobs, and refusing a re-run when `STUB_POST_ERR` is set.
fn decide(script: &str, run: &Run, env: &[(&str, &str)]) -> Decided {
    let dir = Scratch::new("ci_win_retry_decide");
    let data = dir.join("run");
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).expect("bin");
    run.write(&data);
    std::fs::write(
        data.join("jobs-page"),
        json!({"jobs": run.jobs}).to_string(),
    )
    .expect("jobs page");
    let newer = run.newer.clone().unwrap_or_else(|| json!([]));
    let mut listed: Vec<Value> = newer
        .as_array()
        .expect("runs")
        .iter()
        .map(|r| json!({"id": r["id"]}))
        .collect();
    listed.push(json!({"id": 42}));
    std::fs::write(
        data.join("runs"),
        json!({"workflow_runs": listed}).to_string(),
    )
    .expect("runs");
    for r in newer.as_array().expect("runs") {
        std::fs::write(
            data.join(format!("jobs-of-{}", r["id"])),
            json!({"jobs": r["jobs"]}).to_string(),
        )
        .expect("a newer run's jobs");
    }
    let file = dir.join("win_retry.py");
    std::fs::write(&file, script).expect("script written");
    let calls = dir.join("calls");
    std::fs::write(&calls, "").expect("calls");
    let summary = dir.join("summary");
    write_executable(
        &bin.join("gh"),
        "#!/bin/sh\n\
         printf '%s\\n' \"gh $*\" >> \"$STUB_CALLS\"\n\
         args=\"$*\"\n\
         case \"$args\" in\n\
           *'--method POST'*)\n\
             if [ -n \"$STUB_POST_ERR\" ]; then echo \"$STUB_POST_ERR\" >&2; exit 1; fi ;;\n\
           *'/attempts/'*'/jobs'*) cat \"$STUB_DATA/jobs-page\" ;;\n\
           *'/annotations'*)\n\
             id=${args##*check-runs/}; id=${id%%/*}\n\
             cat \"$STUB_DATA/annotations/$id.json\" 2> /dev/null || echo '[]' ;;\n\
           *'/logs')\n\
             id=${args##*jobs/}; id=${id%/logs}\n\
             cat \"$STUB_DATA/logs/$id.log\" 2> /dev/null || { echo 'gh: HTTP 404' >&2; exit 1; } ;;\n\
           *'/workflows/'*'/runs'*)\n\
             if [ -n \"$STUB_RUNS_ERR\" ]; then echo \"$STUB_RUNS_ERR\" >&2; exit 1; fi\n\
             cat \"$STUB_DATA/runs\" ;;\n\
           *'/runs/'*'/jobs'*)\n\
             id=${args##*runs/}; id=${id%%/*}\n\
             cat \"$STUB_DATA/jobs-of-$id\" ;;\n\
         esac\n\
         exit 0\n",
    )
    .expect("gh stand-in");
    let mut cmd = Command::new("python3");
    cmd.arg("-I")
        .arg(&file)
        .arg("decide")
        .current_dir(&*dir)
        .env(
            "PATH",
            format!(
                "{}:{}",
                bin.display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .env("STUB_CALLS", &calls)
        .env("STUB_DATA", &data)
        .env("GH_REPO", "o/r")
        .env("RUN_ID", "42")
        .env(
            "RUN_ATTEMPT",
            run.run["run_attempt"]
                .as_u64()
                .expect("attempt")
                .to_string(),
        )
        .env("RUN_EVENT", run.run["event"].as_str().expect("event"))
        .env(
            "HEAD_BRANCH",
            run.run["head_branch"].as_str().expect("branch"),
        )
        .env(
            "RUN_CONCLUSION",
            run.run["conclusion"].as_str().expect("conclusion"),
        )
        .env("WORKFLOW_ID", "7")
        .env("GITHUB_STEP_SUMMARY", &summary);
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.timed_output().expect("ci_win_retry: decide ran");
    Decided {
        ok: out.status.success(),
        calls: std::fs::read_to_string(&calls)
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect(),
        summary: std::fs::read_to_string(&summary).unwrap_or_default(),
    }
}

fn check_decide_rerun(script: &str) {
    let done = decide(script, &lost(), &[]);
    assert!(done.ok, "decide failed");
    assert_eq!(
        done.posts(),
        [&format!(
            "gh api --method POST repos/o/r/actions/jobs/{TEST_JOB}/rerun"
        )],
        "decide asked for something other than the one job's re-run"
    );
    assert!(
        done.summary
            .contains(&format!("**Decision: re-run job {TEST_JOB} once.**")),
        "the summary gives no decision"
    );
    let row = done
        .summary
        .lines()
        .find(|l| l.starts_with("| win-vulkan |"))
        .expect("the summary has no row for the job");
    for part in [
        "| cargo nextest run -p validation -p xtask -p engine |",
        "unclear stop, a runner-level fault: re-run once",
        "lost communication with the server",
        "| run 42, attempt 2 |",
    ] {
        assert!(
            row.contains(part),
            "the summary's row lacks the run, step, signature or re-run"
        );
    }
    // Asked of the newer runs only for a fault, and read before the re-run.
    let listed = done
        .calls
        .iter()
        .position(|c| c.contains("/workflows/7/runs"));
    let posted = done.calls.iter().position(|c| c.contains("--method POST"));
    assert!(
        listed.is_some_and(|l| posted.is_some_and(|p| l < p)),
        "the newer runs were not read before the re-run"
    );
}

#[test]
fn ci_win_retry_decide_reruns_the_one_job_and_logs_it() {
    check_decide_rerun(&script());
}

negative_control!(
    ci_win_retry_decide_reruns_the_one_job_and_logs_it,
    "the whole run re-run for one job",
    expected = "decide asked for something other than the one job's re-run",
    check_decide_rerun(&edit(
        &script(),
        "f\"repos/{repo}/actions/jobs/{rerun[1]}/rerun\" if rerun[0] == \"job\"",
        "f\"repos/{repo}/actions/runs/{run_id}/rerun\" if rerun[0] == \"job\""
    ))
);

mod no_signature_logged {
    use super::*;
    negative_control!(
        ci_win_retry_decide_reruns_the_one_job_and_logs_it,
        "a summary without the line the decision rests on",
        expected = "the summary's row lacks the run, step, signature or re-run",
        check_decide_rerun(&edit(
            &script(),
            "            cell = evidence.replace(",
            "            cell = \"\".replace("
        ))
    );
}

fn check_decide_logs(script: &str) {
    // A hang: logged, no re-run, and the newer runs not asked for.
    let done = decide(script, &Run::saved(JOB_LIMIT_RUN), &[]);
    assert!(done.ok, "decide failed on a hang");
    assert!(done.posts().is_empty(), "a hang: re-run");
    assert!(
        done.summary.contains(
            "| wsl-register | This run's image | hang, ended by its time limit: not re-run | \
             `The job has exceeded the maximum execution time of 1h0m0s` | — |"
        ),
        "a hang: not logged with its step and signature"
    );
    assert!(
        !done.calls.iter().any(|c| c.contains("/workflows/")),
        "a hang: the newer runs read"
    );
    // A re-run says it is one, whatever its result.
    let mut passed = Run::saved(TEST_RUN).attempt(2);
    passed.job(TEST_JOB)["conclusion"] = json!("success");
    passed.run["conclusion"] = json!("success");
    let done = decide(script, &passed, &[]);
    assert!(done.ok, "decide failed on a re-run");
    assert!(
        done.summary
            .contains("This is a re-run, attempt 2; the first attempt's record stays in run 42."),
        "a re-run doesn't say it is one"
    );
    // A refused re-run fails the job and says so.
    let done = decide(script, &lost(), &[("STUB_POST_ERR", "HTTP 403")]);
    assert!(!done.ok, "a refused re-run passed");
    assert!(
        done.summary.contains("the re-run request failed")
            && done.summary.contains("| requested, refused |"),
        "a refused re-run: not logged"
    );
    // Newer runs that can't be read re-run nothing.
    let done = decide(script, &lost(), &[("STUB_RUNS_ERR", "HTTP 502")]);
    assert!(done.posts().is_empty(), "newer runs unread: re-run");
}

#[test]
fn ci_win_retry_decide_logs_every_unclear_stop_and_hang() {
    check_decide_logs(&script());
}

negative_control!(
    ci_win_retry_decide_logs_every_unclear_stop_and_hang,
    "a re-run whose summary doesn't say it is one",
    expected = "a re-run doesn't say it is one",
    check_decide_logs(&edit(
        &script(),
        "    if attempt > FIRST:\n",
        "    if attempt > 9:\n"
    ))
);

fn check_decide_recovered(script: &str) {
    let done = decide(script, &recovered(), &[]);
    assert!(done.ok, "decide failed on a passing run");
    assert!(done.posts().is_empty(), "a recovered step limit: re-run");
    assert!(
        done.summary.contains(&format!(
            "| wsl-register | This run's image | {RECOVERED} | `{FIRST_DOWNLOAD_LIMIT}` | — |"
        )),
        "a recovered step limit: not logged with its step and signature"
    );
    assert!(
        !done.calls.iter().any(|c| c.contains("/workflows/")),
        "a recovered step limit: the newer runs read"
    );
}

#[test]
fn ci_win_retry_decide_logs_a_step_limit_the_job_passed_on_from() {
    check_decide_recovered(&script());
}

negative_control!(
    ci_win_retry_decide_logs_a_step_limit_the_job_passed_on_from,
    "a passing job's annotations unread",
    expected = "a recovered step limit: not logged with its step and signature",
    check_decide_recovered(&edit(
        &script(),
        "        if not on_runner(j) or j.get(\"conclusion\") == \"skipped\":",
        "        if not on_runner(j) or j.get(\"conclusion\") in PASSED:"
    ))
);

// ---- The workflow -----------------------------------------------------------------------------------------------------

fn parse(text: &str) -> Yaml {
    YamlLoader::load_from_str(text)
        .expect("ci_win_retry: win-retry.yml parses")
        .remove(0)
}

fn names(value: &Yaml) -> Vec<&str> {
    match value {
        Yaml::String(s) => vec![s.as_str()],
        Yaml::Array(a) => a.iter().filter_map(Yaml::as_str).collect(),
        Yaml::Hash(h) => h.keys().filter_map(Yaml::as_str).collect(),
        _ => Vec::new(),
    }
}

/// Every string of the document, keys among them.
fn strings(value: &Yaml, out: &mut Vec<String>) {
    match value {
        Yaml::String(s) => out.push(s.clone()),
        Yaml::Array(a) => a.iter().for_each(|v| strings(v, out)),
        Yaml::Hash(h) => h.iter().for_each(|(k, v)| {
            strings(k, out);
            strings(v, out);
        }),
        _ => {}
    }
}

fn check_workflow(text: &str) {
    let doc = parse(text);
    assert_eq!(
        names(&doc["on"]),
        ["workflow_run"],
        "win-retry.yml's triggers aren't workflow_run alone"
    );
    assert_eq!(
        names(&doc["on"]["workflow_run"]["workflows"]),
        ["win-gpu", "runner-proof"],
        "win-retry.yml doesn't follow the runner's two workflows alone"
    );
    assert_eq!(
        names(&doc["on"]["workflow_run"]["types"]),
        ["completed"],
        "win-retry.yml runs on other than a completed run"
    );
    let mut all = Vec::new();
    strings(&doc, &mut all);
    assert!(
        !all.iter().any(|s| s.contains("pull_request")),
        "win-retry.yml names a pull_request trigger"
    );
    assert!(
        !all.iter()
            .any(|s| s.contains("head_sha") || s.contains("secrets.")),
        "win-retry.yml reaches the run's code or a secret"
    );
    let permissions: Vec<(String, String)> = doc["permissions"]
        .as_hash()
        .expect("win-retry.yml's permissions")
        .iter()
        .map(|(k, v)| {
            (
                k.as_str().unwrap_or("").to_owned(),
                v.as_str().unwrap_or("").to_owned(),
            )
        })
        .collect();
    assert_eq!(
        permissions,
        [
            ("actions".to_owned(), "write".to_owned()),
            ("checks".to_owned(), "read".to_owned()),
            ("contents".to_owned(), "read".to_owned())
        ],
        "win-retry.yml's permissions aren't the least it needs"
    );
    let jobs = doc["jobs"].as_hash().expect("win-retry.yml's jobs");
    assert_eq!(jobs.len(), 1, "win-retry.yml holds more than its one job");
    let job = &doc["jobs"]["win-retry"];
    assert!(
        job["runs-on"]
            .as_str()
            .is_some_and(|r| r.starts_with("ubuntu-")),
        "win-retry isn't on a hosted runner"
    );
    assert!(
        job["permissions"].is_badvalue(),
        "win-retry widens its permissions"
    );
    assert!(
        job["if"]
            .as_str()
            .is_some_and(|c| c.starts_with("github.repository == 'Ma1achy/Principia' && (")),
        "win-retry lacks the repository guard"
    );
    // Past the repository guard, the one test: a re-run, or any conclusion but `skipped`, `success` among them.
    assert_eq!(
        job["if"]
            .as_str()
            .and_then(|c| c.strip_prefix("github.repository == 'Ma1achy/Principia' && ")),
        Some(
            "(github.event.workflow_run.run_attempt > 1 || github.event.workflow_run.conclusion != 'skipped')"
        ),
        "win-retry passes over a passing run, its recovered step limits unlogged"
    );
    let steps = job["steps"].as_vec().expect("win-retry's steps");
    let checkout = &steps[0];
    assert!(
        checkout["uses"]
            .as_str()
            .is_some_and(|u| u.starts_with("actions/checkout@"))
            && checkout["with"]["ref"].as_str() == Some("main")
            && checkout["with"]["persist-credentials"].as_bool() == Some(false),
        "win-retry doesn't check out main alone, without credentials"
    );
    assert_eq!(
        steps.last().and_then(|s| s["run"].as_str()),
        Some("python3 -I .github/scripts/win_retry.py decide"),
        "win-retry doesn't run the classifier"
    );
}

#[test]
fn ci_win_retry_workflow_is_hosted_and_follows_the_runner_alone() {
    check_workflow(&read(WORKFLOW));
}

negative_control!(
    ci_win_retry_workflow_is_hosted_and_follows_the_runner_alone,
    "a pull_request trigger added",
    expected = "win-retry.yml's triggers aren't workflow_run alone",
    check_workflow(&edit(
        &read(WORKFLOW),
        "on:\n  workflow_run:\n",
        "on:\n  pull_request:\n  workflow_run:\n"
    ))
);

mod third_workflow {
    use super::*;
    negative_control!(
        ci_win_retry_workflow_is_hosted_and_follows_the_runner_alone,
        "a third workflow followed",
        expected = "win-retry.yml doesn't follow the runner's two workflows alone",
        check_workflow(&edit(
            &read(WORKFLOW),
            "[win-gpu, runner-proof]",
            "[win-gpu, runner-proof, ci]"
        ))
    );
}

mod on_the_pc {
    use super::*;
    negative_control!(
        ci_win_retry_workflow_is_hosted_and_follows_the_runner_alone,
        "the job on the PC",
        expected = "win-retry isn't on a hosted runner",
        check_workflow(&edit(
            &read(WORKFLOW),
            "runs-on: ubuntu-24.04",
            "runs-on: [self-hosted, windows, nvidia]"
        ))
    );
}

mod contents_write {
    use super::*;
    negative_control!(
        ci_win_retry_workflow_is_hosted_and_follows_the_runner_alone,
        "write access to the repository's contents",
        expected = "win-retry.yml's permissions aren't the least it needs",
        check_workflow(&edit(
            &read(WORKFLOW),
            "  contents: read\n",
            "  contents: write\n"
        ))
    );
}

mod run_code {
    use super::*;
    negative_control!(
        ci_win_retry_workflow_is_hosted_and_follows_the_runner_alone,
        "the checkout of the run's own head",
        expected = "win-retry.yml reaches the run's code or a secret",
        check_workflow(&edit(
            &read(WORKFLOW),
            "          ref: main\n",
            "          ref: ${{ github.event.workflow_run.head_sha }}\n"
        ))
    );
}

mod unguarded {
    use super::*;
    negative_control!(
        ci_win_retry_workflow_is_hosted_and_follows_the_runner_alone,
        "the repository guard gone",
        expected = "win-retry lacks the repository guard",
        check_workflow(&edit(
            &read(WORKFLOW),
            "github.repository == 'Ma1achy/Principia' && ",
            ""
        ))
    );
}

mod passing_passed_over {
    use super::*;
    negative_control!(
        ci_win_retry_workflow_is_hosted_and_follows_the_runner_alone,
        "a passing run passed over",
        expected = "win-retry passes over a passing run, its recovered step limits unlogged",
        check_workflow(&edit(
            &read(WORKFLOW),
            "github.event.workflow_run.conclusion != 'skipped'",
            "(github.event.workflow_run.conclusion != 'success' && \
             github.event.workflow_run.conclusion != 'skipped')"
        ))
    );
}

mod failures_alone {
    use super::*;
    negative_control!(
        ci_win_retry_workflow_is_hosted_and_follows_the_runner_alone,
        "a guard narrowed to failed runs",
        expected = "win-retry passes over a passing run, its recovered step limits unlogged",
        check_workflow(&edit(
            &read(WORKFLOW),
            "github.event.workflow_run.conclusion != 'skipped'",
            "github.event.workflow_run.conclusion == 'failure'"
        ))
    );
}

mod first_attempts_alone {
    use super::*;
    negative_control!(
        ci_win_retry_workflow_is_hosted_and_follows_the_runner_alone,
        "a guard that drops a re-run",
        expected = "win-retry passes over a passing run, its recovered step limits unlogged",
        check_workflow(&edit(
            &read(WORKFLOW),
            "github.event.workflow_run.run_attempt > 1 || ",
            ""
        ))
    );
}
