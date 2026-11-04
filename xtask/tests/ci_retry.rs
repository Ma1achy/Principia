//! `.github/scripts/ci_retry.py` and `.github/workflows/retry.yml`: a failed run's failed jobs are re-run when, and only
//! when, the runner failed, not the code (the owner, 4 Nov 2026: "put the 'is this a runner fault?' classifier in a
//! script with tests against saved logs (today's 'No space left on device' run, a lost-communication sample, and a real
//! test failure that must NOT retry), so the PR proves the decision logic even though the trigger itself can only be
//! exercised after merge").
//!
//! The runs are real, trimmed, under `fixtures/ci_retry/` (`fixtures/README.md`): the classifier reads each as it reads
//! a run online, and a case that needs what no saved run shows edits one saved run in the one respect it turns on. The
//! timeouts have no saved run; their texts are real annotations of runs named here. The shutdown signal has no real
//! trace in this repository; its text rests on the source of `actions/runner`.
//!
//! - `ci_retry_retries_*`: each signature is re-run, the gate that gathers the job beside it;
//! - `ci_retry_keeps_*`: a real test failure, a mixed run, attempt 3 and on, a timeout, a run that did not fail, a
//!   gate alone, and a signature in the wrong place are not;
//! - `ci_retry_main_and_pull_requests_alike`: the event and the branch change nothing;
//! - `ci_retry_decide_*`: `decide`, under bash with a stand-in `gh`, asks for `rerun-failed-jobs` once, never a whole
//!   run, and writes the decision and its reasons to the step summary;
//! - `ci_retry_workflow_*`: `retry.yml`, parsed with PyYAML, and `ci.yml`'s disk probe.
//!
//! No panic here prints a verdict or a log. The classifier reads a failed step's text, so a test failing here must not
//! put a signature in it, or that failure would read as a runner fault.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};
use validation::negative_control;
use validation::spawn::{write_executable, Spawn};

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;

const SCRIPT: &str = ".github/scripts/ci_retry.py";
/// `main` e1df88fa: `ci (3)` died out of disk and `ci (4)` failed at 1910 MiB free.
const DISK_RUN: &str = "41744485022";
const CI_3: &str = "126298758271";
const CI_4: &str = "126298758113";
const CI_GATE: &str = "126311255990";
/// PR #389: `xtask-ci (1)` lost communication with the server.
const LOST_RUN: &str = "41707146179";
const LOST_JOB: &str = "126168286028";
/// `main`: real test failures.
const TEST_RUN: &str = "41431397009";
const RELEASE_1: &str = "125269262008";
const WORKSPACE: &str = "125245906416";
/// `ci-archive` out of disk on a pull request, the `ci` shards skipped, the gate `ci` red through the skip.
const ARCHIVE_RUN: &str = "41211489245";
/// No run here has hit it: no saved or real annotation, the text as `actions/runner` has it, in
/// `src/Runner.Worker/JobRunner.cs`.
const SHUTDOWN: &str = "The runner has received a shutdown signal. This can happen when the runner service is stopped, \
                        or a manually started runner is canceled.";
/// A real annotation: `mutants` run 41566550367, `mutants (4)`.
const STEP_TIMEOUT: &str =
    "The action 'cargo mutants --in-diff --shard 4/8 (REQ-VAL-149 per-shard limit, R-376)' has \
                            timed out after 300 minutes.";
/// A real annotation: `ci` run 40558523181, `ci (3)`, which ended `cancelled`.
const JOB_TIMEOUT: &str = "The job has exceeded the maximum execution time of 6h0m0s";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the workspace root")
        .to_path_buf()
}

fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path)).unwrap_or_else(|e| panic!("ci_retry: {path}: {e}"))
}

fn script() -> String {
    read(SCRIPT)
}

/// `text` with `from` replaced by `to`, which must be in it: a control's one edit.
#[cfg(feature = "controls")]
fn edit(text: &str, from: &str, to: &str) -> String {
    assert!(text.contains(from), "ci_retry: a control's edit is gone");
    text.replace(from, to)
}

/// A run as the classifier reads it.
#[derive(Clone)]
struct Run {
    run: Value,
    jobs: Vec<Value>,
    annotations: BTreeMap<String, String>,
    logs: BTreeMap<String, String>,
}

impl Run {
    /// The saved run `id`, read whole.
    fn saved(id: &str) -> Run {
        let dir = root().join("fixtures/ci_retry").join(id);
        let json = |p: &Path| -> Value {
            let text = std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{p:?}: {e}"));
            serde_json::from_str(&text).unwrap_or_else(|e| panic!("{p:?}: {e}"))
        };
        let folder = |name: &str| -> BTreeMap<String, String> {
            std::fs::read_dir(dir.join(name))
                .expect("ci_retry: a saved run's folder")
                .map(|e| e.expect("entry").path())
                .map(|p| {
                    let id = p.file_stem().expect("stem").to_string_lossy().into_owned();
                    (id, std::fs::read_to_string(&p).expect("a saved file"))
                })
                .collect()
        };
        Run {
            run: json(&dir.join("run.json")),
            jobs: json(&dir.join("jobs.json"))
                .as_array()
                .expect("jobs")
                .clone(),
            annotations: folder("annotations"),
            logs: folder("logs"),
        }
    }

    /// This run, with the job `id` passed, as if it had never failed.
    fn passing(mut self, id: &str) -> Run {
        let job = self
            .jobs
            .iter_mut()
            .find(|j| j["id"].as_u64() == id.parse().ok())
            .unwrap_or_else(|| panic!("ci_retry: no job {id}"));
        job["conclusion"] = json!("success");
        self
    }

    /// This run, with the job `id` of `other` in it as it failed there.
    fn with_job_of(mut self, other: &Run, id: &str) -> Run {
        let job = other
            .jobs
            .iter()
            .find(|j| j["id"].as_u64() == id.parse().ok())
            .unwrap_or_else(|| panic!("ci_retry: no job {id}"));
        self.jobs.push(job.clone());
        for (from, to) in [
            (&other.annotations, &mut self.annotations),
            (&other.logs, &mut self.logs),
        ] {
            if let Some(text) = from.get(id) {
                to.insert(id.to_owned(), text.clone());
            }
        }
        self
    }

    fn attempt(mut self, n: u32) -> Run {
        self.run["run_attempt"] = json!(n);
        self
    }

    fn conclusion(mut self, c: &str) -> Run {
        self.run["conclusion"] = json!(c);
        self
    }

    fn on(mut self, event: &str, branch: &str) -> Run {
        self.run["event"] = json!(event);
        self.run["head_branch"] = json!(branch);
        self
    }

    /// The job `id`'s only annotation, a failure saying `message`.
    fn annotated(mut self, id: &str, message: &str) -> Run {
        self.annotations.insert(
            id.to_owned(),
            json!([{"annotation_level": "failure", "message": message}]).to_string(),
        );
        self
    }

    /// The job `id`'s log, edited.
    fn log(mut self, id: &str, change: impl Fn(&str) -> String) -> Run {
        let new = change(self.logs.get(id).expect("ci_retry: a saved log"));
        self.logs.insert(id.to_owned(), new);
        self
    }

    fn no_log(mut self, id: &str) -> Run {
        self.logs.remove(id);
        self
    }

    /// The job `id`'s conclusion.
    fn concluded(mut self, id: &str, conclusion: &str) -> Run {
        let job = self
            .jobs
            .iter_mut()
            .find(|j| j["id"].as_u64() == id.parse().ok())
            .unwrap_or_else(|| panic!("ci_retry: no job {id}"));
        job["conclusion"] = json!(conclusion);
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
    }
}

/// `ci (3)` alone failed on the saved disk run, and the gate that gathers it.
fn no_space() -> Run {
    Run::saved(DISK_RUN).passing(CI_4)
}

/// `ci (4)` alone failed on the saved disk run, and the gate that gathers it.
fn low_disk() -> Run {
    Run::saved(DISK_RUN).passing(CI_3)
}

/// `run`, classified by `script` against the workflow `workflow`: what it printed.
fn classified_on(script: &str, run: &Run, workflow: &str) -> String {
    let dir = Scratch::new("ci_retry_classify");
    std::fs::create_dir_all(&*dir).expect("scratch folder");
    let file = dir.join("ci_retry.py");
    std::fs::write(&file, script).expect("script written");
    run.write(&dir);
    let out = Command::new("python3")
        .arg("-I")
        .arg(&file)
        .arg("classify")
        .arg(&*dir)
        .arg(root().join(".github/workflows").join(workflow))
        .timed_output()
        .expect("ci_retry: python3 ran");
    assert!(out.status.success(), "ci_retry: the classifier failed");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn retried_on(script: &str, run: &Run, workflow: &str) -> bool {
    let text = classified_on(script, run, workflow);
    let line = text.lines().next().expect("ci_retry: a verdict");
    match line {
        "retry=true" => true,
        "retry=false" => false,
        _ => panic!("ci_retry: the verdict's first line is not `retry=`"),
    }
}

fn retried(script: &str, run: &Run) -> bool {
    retried_on(script, run, "ci.yml")
}

/// Asserts `run` is re-run, and, the verdict naming `job`, that `says` is why.
fn check_retried(script: &str, case: &str, run: &Run, job: &str, says: &str) {
    assert!(retried(script, run), "{case}: not re-run");
    let text = classified_on(script, run, "ci.yml");
    assert!(
        text.lines()
            .any(|l| l.starts_with(&format!("job={job}: runner fault: {says}"))),
        "{case}: the verdict does not give the signature it was re-run for"
    );
}

fn check_kept(script: &str, case: &str, run: &Run) {
    assert!(!retried(script, run), "{case}: re-run");
}

// ---- Each signature is re-run ---------------------------------------------------------------------------------------

fn check_no_space(script: &str) {
    check_retried(
        script,
        "the saved no-space run",
        &no_space(),
        "ci (3)",
        "No space left on device (annotation)",
    );
    check_retried(
        script,
        "the saved run whole",
        &Run::saved(DISK_RUN),
        "ci (3)",
        "No space left on device (annotation)",
    );
}

#[test]
fn ci_retry_retries_no_space_left() {
    check_no_space(&script());
}

negative_control!(
    ci_retry_retries_no_space_left,
    "a classifier without the signature",
    expected = "the saved no-space run: not re-run",
    check_no_space(&edit(
        &script(),
        "re.compile(r\"No space left on device\")",
        "re.compile(r\"No room left on device\")"
    ))
);

/// The signature in the log of a failed step, as a compiler or test run writes it: no annotation carries it.
fn check_no_space_in_a_log(script: &str) {
    let run = at(40960)
        .log(CI_4, |l| {
            l.replace(
                "##[notice]",
                "error: failed to write: No space left on device (os error 28)\n##[notice]",
            )
        })
        .annotated(CI_4, "Process completed with exit code 100.");
    check_retried(
        script,
        "the signature in a failed step's log",
        &run,
        "ci (4)",
        "No space left on device (log)",
    );
}

#[test]
fn ci_retry_retries_no_space_in_a_failed_steps_log() {
    check_no_space_in_a_log(&script());
}

negative_control!(
    ci_retry_retries_no_space_in_a_failed_steps_log,
    "a classifier that reads the log for no signature but the sampler's",
    expected = "the signature in a failed step's log: not re-run",
    check_no_space_in_a_log(&edit(
        &script(),
        "            if pattern.search(step):\n",
        "            if False:\n"
    ))
);

fn check_lost(script: &str) {
    check_retried(
        script,
        "the saved lost-communication run",
        &Run::saved(LOST_RUN),
        "xtask-ci (1)",
        "The hosted runner lost communication with the server (annotation)",
    );
    // The form that names the runner, which GitHub also writes.
    let named = Run::saved(LOST_RUN).annotated(
        LOST_JOB,
        "The hosted runner: GitHub Actions 1000073521 lost communication with the server.",
    );
    check_retried(
        script,
        "the form naming the runner",
        &named,
        "xtask-ci (1)",
        "The hosted runner lost communication with the server (annotation)",
    );
}

#[test]
fn ci_retry_retries_lost_communication() {
    check_lost(&script());
}

negative_control!(
    ci_retry_retries_lost_communication,
    "a classifier without the signature",
    expected = "the saved lost-communication run: not re-run",
    check_lost(&edit(
        &script(),
        "re.compile(r\"The hosted runner(?:: [^\\n]*?)? lost communication with the server\")",
        "re.compile(r\"The hosted runner went away\")"
    ))
);

mod lost_exact {
    use super::*;
    negative_control!(
        ci_retry_retries_lost_communication,
        "a classifier that knows only the form without the runner's name",
        expected = "the form naming the runner: not re-run",
        check_lost(&edit(
            &script(),
            "The hosted runner(?:: [^\\n]*?)? lost",
            "The hosted runner lost"
        ))
    );
}

fn check_shutdown(script: &str) {
    let run = Run::saved(LOST_RUN).annotated(LOST_JOB, SHUTDOWN);
    check_retried(
        script,
        "a shutdown signal",
        &run,
        "xtask-ci (1)",
        "The runner has received a shutdown signal (annotation)",
    );
}

#[test]
fn ci_retry_retries_a_shutdown_signal() {
    check_shutdown(&script());
}

negative_control!(
    ci_retry_retries_a_shutdown_signal,
    "a classifier without the signature",
    expected = "a shutdown signal: not re-run",
    check_shutdown(&edit(
        &script(),
        "re.compile(r\"The runner has received a shutdown signal\")",
        "re.compile(r\"The runner went away\")"
    ))
);

/// The failed step's lowest sample at `kb`. Its notice still says 1910 MiB, as a low earlier step would leave it.
fn at_kb(kb: u32) -> Run {
    low_disk().log(CI_4, |l| {
        l.replace("avail_kb=1956588", &format!("avail_kb={kb}"))
    })
}

fn at(mib: u32) -> Run {
    at_kb(mib * 1024)
}

fn check_low_disk(script: &str) {
    check_retried(
        script,
        "the saved run at 1910 MiB",
        &low_disk(),
        "ci (4)",
        "lowest free 1910 MiB, under 2048 MiB, in the failed step (disk sampler)",
    );
    check_retried(
        script,
        "a KiB under 2 GiB",
        &at_kb(2 * 1024 * 1024 - 1),
        "ci (4)",
        "lowest free 2047 MiB",
    );
    check_kept(script, "2 GiB exactly", &at_kb(2 * 1024 * 1024));
    check_kept(script, "the same failure on 40 GiB", &at(40960));
}

#[test]
fn ci_retry_retries_under_2_gib_free_in_the_failed_step() {
    check_low_disk(&script());
}

negative_control!(
    ci_retry_retries_under_2_gib_free_in_the_failed_step,
    "a limit that takes 2 GiB in",
    expected = "2 GiB exactly: re-run",
    check_low_disk(&edit(
        &script(),
        "int(m.group(1)) < LOW_MIB * 1024",
        "int(m.group(1)) <= LOW_MIB * 1024"
    ))
);

mod limit_low {
    use super::*;
    negative_control!(
        ci_retry_retries_under_2_gib_free_in_the_failed_step,
        "a limit of 1 GiB",
        expected = "the saved run at 1910 MiB: not re-run",
        check_low_disk(&edit(&script(), "LOW_MIB = 2048", "LOW_MIB = 1024"))
    );
}

/// The saved `ci-archive` run: the shards skipped, the gate red through the skip, and nothing else wrong.
fn check_upstream(script: &str) {
    let run = Run::saved(ARCHIVE_RUN);
    check_retried(
        script,
        "a fault behind skipped shards",
        &run,
        "ci-archive",
        "No space left on device (annotation)",
    );
    assert!(
        classified_on(script, &run, "ci.yml")
            .lines()
            .any(|l| l == "job=ci: gate of a re-run job (ci-archive)"),
        "the verdict does not name the fault behind the gate"
    );
    // One shard listed as passed beside the skip: the walk is by job, and the job did not pass.
    let mut shard = run.clone();
    shard
        .jobs
        .push(json!({"id": 1, "name": "ci (1)", "conclusion": "success"}));
    assert!(
        retried(script, &shard),
        "a fault behind shards, one of them passed: not re-run"
    );
}

#[test]
fn ci_retry_retries_a_fault_behind_skipped_jobs() {
    check_upstream(&script());
}

negative_control!(
    ci_retry_retries_a_fault_behind_skipped_jobs,
    "a walk that stops at the jobs a gate gathers",
    expected = "a fault behind skipped shards: not re-run",
    check_upstream(&edit(
        &script(),
        "            stack.extend(needs.get(n, ()))\n",
        "            pass\n"
    ))
);

mod any_shard {
    use super::*;
    negative_control!(
        ci_retry_retries_a_fault_behind_skipped_jobs,
        "a job taken as passed when any of its shards did",
        expected = "a fault behind shards, one of them passed: not re-run",
        check_upstream(&edit(
            &script(),
            "if all(c in (\"success\", \"neutral\") for c in seen)",
            "if any(c in (\"success\", \"neutral\") for c in seen)"
        ))
    );
}

// ---- What is not re-run ---------------------------------------------------------------------------------------------

fn check_test_failure(script: &str) {
    check_kept(script, "the saved test failure", &Run::saved(TEST_RUN));
}

/// The negative control of every `ci_retry_retries_*`: the same classifier, on a run with no runner fault.
#[test]
fn ci_retry_keeps_a_real_test_failure() {
    check_test_failure(&script());
}

negative_control!(
    ci_retry_keeps_a_real_test_failure,
    "a classifier that takes any failed step for a runner fault",
    expected = "the saved test failure: re-run",
    check_test_failure(&edit(
        &script(),
        "re.compile(r\"The runner has received a shutdown signal\")",
        "re.compile(r\"Process completed with exit code\")"
    ))
);

fn check_mixed(script: &str) {
    let tests = Run::saved(TEST_RUN);
    check_kept(
        script,
        "a disk fault beside a test failure",
        &no_space().with_job_of(&tests, RELEASE_1),
    );
    check_kept(
        script,
        "a low disk beside a test failure",
        &low_disk().with_job_of(&tests, WORKSPACE),
    );
}

#[test]
fn ci_retry_keeps_a_mixed_run() {
    check_mixed(&script());
}

negative_control!(
    ci_retry_keeps_a_mixed_run,
    "one runner fault enough to re-run every failed job",
    expected = "a disk fault beside a test failure: re-run",
    check_mixed(&edit(
        &script(),
        "    if blocking:\n",
        "    if not faulted:\n"
    ))
);

fn check_attempts(script: &str) {
    for (attempt, expected) in [(1, true), (2, true), (3, false), (4, false)] {
        assert_eq!(
            retried(script, &no_space().attempt(attempt)),
            expected,
            "attempt {attempt}"
        );
    }
}

#[test]
fn ci_retry_keeps_attempt_3_and_on() {
    check_attempts(&script());
}

negative_control!(
    ci_retry_keeps_attempt_3_and_on,
    "a cap one higher",
    expected = "attempt 3",
    check_attempts(&edit(&script(), "MAX_ATTEMPTS = 3", "MAX_ATTEMPTS = 4"))
);

mod cap_low {
    use super::*;
    negative_control!(
        ci_retry_keeps_attempt_3_and_on,
        "a cap one lower",
        expected = "attempt 2",
        check_attempts(&edit(&script(), "MAX_ATTEMPTS = 3", "MAX_ATTEMPTS = 2"))
    );
}

fn check_conclusions(script: &str) {
    for conclusion in ["success", "cancelled", "timed_out", "skipped"] {
        check_kept(
            script,
            &format!("a run that concluded {conclusion}"),
            &no_space().conclusion(conclusion),
        );
    }
}

#[test]
fn ci_retry_keeps_a_run_that_did_not_fail() {
    check_conclusions(&script());
}

negative_control!(
    ci_retry_keeps_a_run_that_did_not_fail,
    "a classifier that does not read the conclusion",
    expected = "a run that concluded cancelled: re-run",
    check_conclusions(&edit(
        &script(),
        "    if conclusion != \"failure\":\n",
        "    if conclusion == \"success\":\n"
    ))
);

fn check_timeouts(script: &str) {
    // Beside a signature, each of them: the timeout is what keeps the run.
    let beside =
        |timeout: &str| no_space().annotated(CI_3, &format!("No space left on device\n{timeout}"));
    check_kept(
        script,
        "a step's timeout, in an annotation",
        &beside(STEP_TIMEOUT),
    );
    check_kept(
        script,
        "a job's timeout, in an annotation",
        &beside(JOB_TIMEOUT),
    );
    check_kept(
        script,
        "a timeout in the log, beside a signature",
        &low_disk().log(CI_4, |l| {
            format!("{l}2026-11-04T00:41:30.3574347Z ##[error]{STEP_TIMEOUT}\n")
        }),
    );
    check_kept(
        script,
        "a timed-out job",
        &low_disk().concluded(CI_4, "timed_out"),
    );
    check_kept(
        script,
        "a gate that timed out",
        &no_space().concluded(CI_GATE, "timed_out"),
    );
}

#[test]
fn ci_retry_keeps_a_timeout() {
    check_timeouts(&script());
}

negative_control!(
    ci_retry_keeps_a_timeout,
    "a classifier that reads no timeout",
    expected = "a step's timeout, in an annotation: re-run",
    check_timeouts(&edit(
        &script(),
        "        if timed_out(j, annotations.get(jid, []), logs.get(jid)):\n",
        "        if False:\n"
    ))
);

mod timed_out_state {
    use super::*;
    negative_control!(
        ci_retry_keeps_a_timeout,
        "a classifier that does not read the job's state",
        expected = "a timed-out job: re-run",
        check_timeouts(&edit(
            &script(),
            "    if job.get(\"conclusion\") == \"timed_out\":\n",
            "    if False:\n"
        ))
    );
}

mod timed_out_gate {
    use super::*;
    negative_control!(
        ci_retry_keeps_a_timeout,
        "a gate that timed out still let through",
        expected = "a gate that timed out: re-run",
        check_timeouts(&edit(
            &script(),
            "if n in faulted or n in timeouts or jid is None:",
            "if n in faulted or jid is None:"
        ))
    );
}

fn check_gates(script: &str) {
    // The gate alone: nothing it gathers failed here, so it failed for itself.
    check_kept(
        script,
        "a gate alone",
        &Run::saved(DISK_RUN).passing(CI_3).passing(CI_4),
    );
    // A gate of something else: `xtask-ci`, which gathers no `ci` shard.
    let beside = no_space().with_job_of(&Run::saved(LOST_RUN), "126203575523");
    check_kept(script, "a gate of no re-run job", &beside);
    // The gates of the `mutants` matrix, the second gathering the first.
    let mutants = |faulted: &str| -> Run {
        let mut run = no_space();
        for (id, name) in [
            ("1", "mutants (3)"),
            ("2", "mutants-verdict"),
            ("3", "mutants-check"),
        ] {
            run.jobs.push(json!({"id": id.parse::<u64>().expect("id"), "name": name, "conclusion": "failure"}));
        }
        run.jobs
            .retain(|j| !j["name"].as_str().expect("name").starts_with("ci"));
        run.annotated("1", faulted)
    };
    assert!(
        retried_on(script, &mutants("No space left on device"), "mutants.yml"),
        "a gate of a gate: not re-run"
    );
    assert!(
        !retried_on(
            script,
            &mutants("Process completed with exit code 1."),
            "mutants.yml"
        ),
        "gates of a real failure: re-run"
    );
    // The fault is no reason for a gate behind a job that passed: that gate failed for itself.
    let mut behind = mutants("No space left on device");
    for job in &mut behind.jobs {
        if job["name"] == "mutants-verdict" {
            job["conclusion"] = json!("success");
        }
    }
    assert!(
        !retried_on(script, &behind, "mutants.yml"),
        "a gate behind a job that passed: re-run"
    );
}

#[test]
fn ci_retry_keeps_a_gate_of_nothing_re_run() {
    check_gates(&script());
}

negative_control!(
    ci_retry_keeps_a_gate_of_nothing_re_run,
    "every job with `needs` taken for a gate of the re-run job",
    expected = "a gate of no re-run job: re-run",
    check_gates(&edit(
        &script(),
        "        if hit:\n            gates[n] = hit\n",
        "        if needs.get(jid):\n            gates[n] = hit\n"
    ))
);

mod one_level {
    use super::*;
    negative_control!(
        ci_retry_keeps_a_gate_of_nothing_re_run,
        "no gate of a gate",
        expected = "a gate of a gate: not re-run",
        check_gates(&edit(
            &script(),
            "            stack.extend(needs.get(n, ()))\n",
            "            pass\n"
        ))
    );
}

mod through_passed {
    use super::*;
    negative_control!(
        ci_retry_keeps_a_gate_of_nothing_re_run,
        "a walk that goes on through a job that passed",
        expected = "a gate behind a job that passed: re-run",
        check_gates(&edit(
            &script(),
            "        elif n not in passed_ids:\n",
            "        else:\n"
        ))
    );
}

mod no_matrix {
    use super::*;
    negative_control!(
        ci_retry_keeps_a_gate_of_nothing_re_run,
        "a matrix job without a name template not known by its shard's name",
        expected = "a gate of a gate: not re-run",
        check_gates(&edit(
            &script(),
            "            pattern += r\" \\(.+\\)\"\n",
            "            pass\n"
        ))
    );
}

/// A low sample in a step that passed, then a step that fails with no reading of its own. And a signature in a warning
/// or a step that passed, a sample in a test's own output, or a job with no trace at all. The real sampler, whose later
/// steps do read, is `ci_retry_keeps_a_failure_after_an_earlier_steps_dip`.
fn check_wrong_place(script: &str) {
    let earlier = low_disk().log(CI_4, |l| {
        l.replace(
            "##[error]Process completed with exit code 100.",
            "##[group]Run cargo test --workspace --doc\n##[error]Process completed with exit code 101.",
        )
    });
    check_kept(script, "a low sample in a step that passed", &earlier);
    let passed = at(40960).log(CI_4, |l| {
        l.replace(
            "##[group]Run cargo nextest run",
            "failed to write: No space left on device\n##[group]Run cargo nextest run",
        )
    });
    check_kept(script, "the signature in a step that passed", &passed);
    let mut warned = at(40960);
    warned.annotations.insert(
        CI_4.to_owned(),
        json!([{"annotation_level": "warning", "message": "No space left on device"}]).to_string(),
    );
    check_kept(script, "the signature in a warning", &warned);
    let output = at(40960).log(CI_4, |l| {
        l.replace(
            "##[notice]",
            "    == sample 1, +0 s, avail_kb=100, 00:00:00\n##[notice]",
        )
    });
    check_kept(script, "a sample in a test's output", &output);
    let silent = no_space().annotated(CI_3, "").no_log(CI_3);
    check_kept(script, "a job with no trace", &silent);
}

#[test]
fn ci_retry_keeps_a_signature_in_the_wrong_place() {
    check_wrong_place(&script());
}

negative_control!(
    ci_retry_keeps_a_signature_in_the_wrong_place,
    "a classifier that reads every step of the log",
    expected = "a low sample in a step that passed: re-run",
    check_wrong_place(&edit(
        &script(),
        "        if line.startswith(STEP):\n            if failed:\n",
        "        if line.startswith(STEP):\n            if True:\n"
    ))
);

mod warnings {
    use super::*;
    negative_control!(
        ci_retry_keeps_a_signature_in_the_wrong_place,
        "a classifier that reads every annotation",
        expected = "the signature in a warning: re-run",
        check_wrong_place(&edit(
            &script(),
            " if a.get(\"annotation_level\") == \"failure\"]",
            "]"
        ))
    );
}

mod indented {
    use super::*;
    negative_control!(
        ci_retry_keeps_a_signature_in_the_wrong_place,
        "a classifier that takes a test's output for a sample",
        expected = "a sample in a test's output: re-run",
        check_wrong_place(&edit(
            &script(),
            "SAMPLE.match(line)",
            "SAMPLE.match(line.strip())"
        ))
    );
}

/// What the real `.github/scripts/disk_sampler.sh` writes to a job's log, its notice as the runner writes it: a sampled
/// step for each `(free KiB, fails)`, one after the other, with `df` standing in for the disk.
fn sampled_log(tag: &str, steps: &[(u64, bool)]) -> String {
    let dir = Scratch::new(tag);
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).expect("bin");
    write_executable(
        &bin.join("df"),
        "#!/bin/sh\necho 'Filesystem 1024-blocks Used Available Capacity Mounted'\n\
         echo \"/dev/x 100 50 $STUB_AVAIL_KB 50% /\"\n",
    )
    .expect("df stand-in");
    let mut log = String::new();
    for (n, (kb, fails)) in steps.iter().enumerate() {
        // Ends once this step's first sample is written and the sampler has had a second to print it.
        let script = format!(
            "for _ in {{1..250}}; do\n  \
             [ \"$(grep -c avail_kb \"$RUNNER_TEMP/runner-disk.log\")\" -gt {n} ] && break\n  \
             sleep 0.1\ndone\nsleep 1\nexit {}\n",
            if *fails { 3 } else { 0 }
        );
        std::fs::write(dir.join("step.sh"), script).expect("step written");
        let out = Command::new("bash")
            .arg(root().join(".github/scripts/disk_sampler.sh"))
            .arg("step.sh")
            .current_dir(&*dir)
            .env("RUNNER_TEMP", &*dir)
            .env("RUNNER_DISK_SAMPLE_SECONDS", "600")
            .env("STUB_AVAIL_KB", kb.to_string())
            .env(
                "PATH",
                format!(
                    "{}:{}",
                    bin.display(),
                    std::env::var("PATH").unwrap_or_default()
                ),
            )
            .timed_output()
            .expect("the sampler ran");
        log.push_str("##[group]Run the step\n##[endgroup]\n");
        log.push_str(
            &String::from_utf8_lossy(&out.stdout)
                .replace("::notice title=Runner disk::", "##[notice]"),
        );
        if *fails {
            log.push_str("##[error]Process completed with exit code 3.\n");
        }
    }
    log
}

/// `ci (4)` with the log the sampler writes for `steps`.
fn sampled(tag: &str, steps: &[(u64, bool)]) -> Run {
    let log = sampled_log(tag, steps);
    low_disk()
        .log(CI_4, |_| log.clone())
        .annotated(CI_4, "Process completed with exit code 3.")
}

/// 100 MiB, and 40 GiB, free.
const LOW: u64 = 100 * 1024;
const PLENTY: u64 = 40 * 1024 * 1024;

/// The sampler's notice covers the job so far, so a step after a low one repeats it: only the step's own samples say
/// what that step had.
fn check_dip(script: &str) {
    let dip = sampled("ci_retry_dip", &[(LOW, false), (PLENTY, true)]);
    assert!(
        dip.logs[CI_4].contains("##[notice]lowest free 100 MiB over 2 samples"),
        "the sampler's notice is not the job's: this test no longer tests that"
    );
    check_kept(script, "a failure after an earlier step's dip", &dip);
    let own = sampled("ci_retry_own", &[(PLENTY, false), (LOW, true)]);
    assert!(
        retried(script, &own),
        "a failure on the step's own low disk: not re-run"
    );
}

#[test]
fn ci_retry_keeps_a_failure_after_an_earlier_steps_dip() {
    check_dip(&script());
}

negative_control!(
    ci_retry_keeps_a_failure_after_an_earlier_steps_dip,
    "a classifier that reads the sampler's notice, the job's lowest so far",
    expected = "a failure after an earlier step's dip: re-run",
    check_dip(&edit(
        &script(),
        "re.compile(r\"^== sample \\d+, \\+\\d+ s, avail_kb=(\\d+),\")",
        "re.compile(r\"^##\\[notice\\]lowest free (\\d+) MiB\")"
    ))
);

// ---- Main and pull requests alike -----------------------------------------------------------------------------------

fn check_alike(script: &str) {
    let events = [("push", "main"), ("pull_request", "task/TASK-M0-01")];
    for (event, branch) in events {
        assert!(
            retried(script, &no_space().on(event, branch)),
            "a runner fault on {event}: not re-run"
        );
        assert!(
            retried(script, &Run::saved(LOST_RUN).on(event, branch)),
            "lost communication on {event}: not re-run"
        );
        assert!(
            !retried(script, &Run::saved(TEST_RUN).on(event, branch)),
            "a test failure on {event}: re-run"
        );
    }
}

#[test]
fn ci_retry_main_and_pull_requests_alike() {
    check_alike(&script());
}

negative_control!(
    ci_retry_main_and_pull_requests_alike,
    "a classifier that re-runs only on `main`",
    expected = "a runner fault on pull_request: not re-run",
    check_alike(&edit(
        &script(),
        "    conclusion = run.get(\"conclusion\")\n",
        "    conclusion = run.get(\"conclusion\") if run.get(\"event\") == \"push\" else None\n"
    ))
);

mod pull_requests_only {
    use super::*;
    negative_control!(
        ci_retry_main_and_pull_requests_alike,
        "a classifier that re-runs only on a pull request",
        expected = "a runner fault on push: not re-run",
        check_alike(&edit(
            &script(),
            "    conclusion = run.get(\"conclusion\")\n",
            "    conclusion = run.get(\"conclusion\") if run.get(\"event\") != \"push\" else None\n"
        ))
    );
}

// ---- `decide` -------------------------------------------------------------------------------------------------------

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
            .filter(|c| c.contains("--method POST") || c.contains("-X POST"))
            .collect()
    }
}

/// `decide` under `script`, on `run`, with a stand-in `gh` that serves `run` as GitHub does: the jobs in two pages, the
/// run's workflow file `ci.yml`, a 404 for a job with no log, the branch's head `$STUB_HEAD`, and
/// refusing what `env` asks of it.
fn decide(script: &str, run: &Run, env: &[(&str, &str)]) -> Decided {
    let dir = Scratch::new("ci_retry_decide");
    let data = dir.join("run");
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).expect("bin");
    run.write(&data);
    let half = run.jobs.len() / 2;
    let pages = format!(
        "{}\n{}\n",
        json!({"jobs": run.jobs[..half]}),
        json!({"jobs": run.jobs[half..]})
    );
    std::fs::write(data.join("pages"), pages).expect("pages");
    let file = dir.join("ci_retry.py");
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
           *--allow-escape-sequences*)\n\
             if [ -n \"$STUB_OLD_GH\" ]; then echo 'unknown flag: --allow-escape-sequences' >&2; exit 1; fi ;;\n\
         esac\n\
         case \"$args\" in\n\
           *'--method POST'*)\n\
             if [ -n \"$STUB_POST_ERR\" ]; then echo \"$STUB_POST_ERR\" >&2; exit 1; fi ;;\n\
           *'/attempts/'*'/jobs'*)\n\
             if [ -n \"$STUB_JOBS_ERR\" ]; then echo \"$STUB_JOBS_ERR\" >&2; exit 1; fi\n\
             cat \"$STUB_DATA/pages\" ;;\n\
           *'/annotations'*)\n\
             id=${args##*check-runs/}; id=${id%%/*}\n\
             cat \"$STUB_DATA/annotations/$id.json\" 2> /dev/null || echo '[]' ;;\n\
           *'/logs')\n\
             id=${args##*jobs/}; id=${id%/logs}\n\
             cat \"$STUB_DATA/logs/$id.log\" 2> /dev/null || { echo 'gh: HTTP 404' >&2; exit 1; } ;;\n\
           *'/contents/'*) cat \"$STUB_WORKFLOW\" ;;\n\
           *'/git/ref/heads/'*)\n\
             if [ -n \"$STUB_REF_ERR\" ]; then echo \"$STUB_REF_ERR\" >&2; exit 1; fi\n\
             if [ -n \"$STUB_REF_BODY\" ]; then echo \"$STUB_REF_BODY\"\n\
             else echo \"{\\\"object\\\":{\\\"sha\\\":\\\"$STUB_HEAD\\\"}}\"; fi ;;\n\
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
        .env("STUB_WORKFLOW", root().join(".github/workflows/ci.yml"))
        .env("GH_REPO", "o/r")
        .env("RUN_ID", "42")
        .env(
            "RUN_ATTEMPT",
            run.run["run_attempt"]
                .as_u64()
                .expect("attempt")
                .to_string(),
        )
        .env(
            "RUN_CONCLUSION",
            run.run["conclusion"].as_str().expect("conclusion"),
        )
        .env("WORKFLOW_PATH", ".github/workflows/ci.yml")
        .env("HEAD_SHA", "abc")
        .env("HEAD_BRANCH", "main")
        .env("STUB_HEAD", "abc")
        .env("GITHUB_STEP_SUMMARY", &summary);
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.timed_output().expect("ci_retry: decide ran");
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

const RERUN: &str = "gh api --method POST repos/o/r/actions/runs/42/rerun-failed-jobs";

fn check_decide_retry(script: &str, env: &[(&str, &str)]) {
    let done = decide(script, &Run::saved(DISK_RUN), env);
    assert!(done.ok, "decide failed");
    let posts = done.posts();
    assert!(
        posts.iter().all(|c| c.as_str() == RERUN),
        "decide asked for something other than the failed jobs: the whole run"
    );
    assert_eq!(
        posts.len(),
        1,
        "{} requests for the failed jobs",
        posts.len()
    );
    assert!(
        done.summary.contains("Decision: re-run the failed jobs"),
        "the summary gives no decision"
    );
    for row in [
        "| ci (3) | runner fault: No space left on device (annotation) |",
        "| ci (4) | runner fault: lowest free 1910 MiB",
        "| ci | gate of a re-run job (ci) |",
    ] {
        assert!(
            done.summary.contains(row),
            "the summary gives no reason for a job"
        );
    }
}

#[test]
fn ci_retry_decide_reruns_the_failed_jobs_and_says_why() {
    check_decide_retry(&script(), &[]);
}

negative_control!(
    ci_retry_decide_reruns_the_failed_jobs_and_says_why,
    "a re-run of the whole run",
    expected = "the whole run",
    check_decide_retry(
        &edit(
            &script(),
            "/actions/runs/{run_id}/rerun-failed-jobs",
            "/actions/runs/{run_id}/rerun"
        ),
        &[]
    )
);

mod no_summary {
    use super::*;
    negative_control!(
        ci_retry_decide_reruns_the_failed_jobs_and_says_why,
        "a decision written nowhere",
        expected = "the summary gives no decision",
        check_decide_retry(&edit(&script(), "    if target:\n", "    if False:\n"), &[])
    );
}

mod one_page {
    use super::*;
    negative_control!(
        ci_retry_decide_reruns_the_failed_jobs_and_says_why,
        "only the first page of jobs read, which `ci (3)`, on the second, is not on",
        expected = "0 requests for the failed jobs",
        check_decide_retry(
            &edit(
                &script(),
                "    while at < len(text):\n        value, end",
                "    if at < len(text):\n        value, end"
            ),
            &[]
        )
    );
}

/// `ci (4)`'s signature is only in its log, and an older `gh` refuses the newer one's flag.
fn check_old_gh(script: &str) {
    let done = decide(script, &low_disk(), &[("STUB_OLD_GH", "1")]);
    assert!(
        done.summary.contains("Decision: re-run the failed jobs"),
        "the summary gives no decision"
    );
}

#[test]
fn ci_retry_decide_reads_logs_on_an_older_gh() {
    check_old_gh(&script());
}

negative_control!(
    ci_retry_decide_reads_logs_on_an_older_gh,
    "no fallback from the flag an older `gh` lacks",
    expected = "the summary gives no decision",
    check_old_gh(&edit(
        &script(),
        "    if out.returncode != 0 and b\"unknown flag\" in out.stderr:\n",
        "    if False:\n"
    ))
);

fn check_decide_keeps(script: &str) {
    for (case, run) in [
        ("a test failure", Run::saved(TEST_RUN)),
        ("attempt 3", no_space().attempt(3)),
        ("a success", no_space().conclusion("success")),
    ] {
        let done = decide(script, &run, &[]);
        assert!(done.ok, "{case}: decide failed");
        assert!(done.posts().is_empty(), "{case}: asked for a re-run");
        assert!(
            done.summary.contains("Decision: no re-run"),
            "{case}: the summary gives no decision"
        );
    }
    let unread = decide(script, &no_space(), &[("STUB_JOBS_ERR", "gh: HTTP 500")]);
    assert!(
        unread.posts().is_empty(),
        "jobs that could not be read: asked for a re-run"
    );
}

#[test]
fn ci_retry_decide_keeps_what_is_no_runner_fault() {
    check_decide_keeps(&script());
}

negative_control!(
    ci_retry_decide_keeps_what_is_no_runner_fault,
    "`decide` asking whatever the verdict",
    expected = "a test failure: asked for a re-run",
    check_decide_keeps(&edit(
        &script(),
        "    if retry:\n        if gh(",
        "    if True:\n        if gh("
    ))
);

/// A re-run GitHub refuses is a failed step, not a quiet one.
fn check_post_refused(script: &str) {
    let done = decide(script, &no_space(), &[("STUB_POST_ERR", "gh: HTTP 403")]);
    assert!(!done.ok, "a refused re-run passed the step");
    assert!(
        done.summary.contains("the re-run request failed"),
        "the summary does not say the re-run failed"
    );
}

#[test]
fn ci_retry_decide_fails_on_a_refused_re_run() {
    check_post_refused(&script());
}

negative_control!(
    ci_retry_decide_fails_on_a_refused_re_run,
    "a refused re-run passed over",
    expected = "a refused re-run passed the step",
    check_post_refused(&edit(
        &script(),
        "            status = 1\n",
        "            status = 0\n"
    ))
);

/// A re-run shares its pull request's concurrency group, so one of a superseded head would cancel the current head's run.
fn check_head(script: &str) {
    let run = Run::saved(DISK_RUN);
    let moved: [(&str, &[(&str, &str)]); 6] = [
        ("a head its branch moved on from", &[("STUB_HEAD", "def")]),
        (
            "a branch that cannot be read",
            &[("STUB_REF_ERR", "gh: HTTP 404")],
        ),
        ("an answer with no head", &[("STUB_REF_BODY", "{}")]),
        ("an answer that is not JSON", &[("STUB_REF_BODY", "oops")]),
        ("a run with no head", &[("HEAD_SHA", "")]),
        ("a run with no branch", &[("HEAD_BRANCH", "")]),
    ];
    for (case, env) in moved {
        let done = decide(script, &run, env);
        assert!(done.ok, "{case}: decide failed");
        assert!(done.posts().is_empty(), "{case}: asked for a re-run");
        assert!(
            done.summary.contains("Decision: no re-run"),
            "{case}: the summary gives no decision"
        );
    }
    // Live, on a branch with a slash in it, as a task's is: asked once, after the head is read.
    let done = decide(script, &run, &[("HEAD_BRANCH", "task/TASK-M0-01")]);
    assert_eq!(done.posts().len(), 1, "the live head: not re-run");
    let read = done
        .calls
        .iter()
        .position(|c| c == "gh api repos/o/r/git/ref/heads/task/TASK-M0-01");
    let post = done.calls.iter().position(|c| c == RERUN);
    assert!(read.is_some() && read < post, "the head is not read first");
}

#[test]
fn ci_retry_decide_keeps_a_superseded_head() {
    check_head(&script());
}

negative_control!(
    ci_retry_decide_keeps_a_superseded_head,
    "`decide` not looking at the head",
    expected = "a head its branch moved on from: asked for a re-run",
    check_head(&edit(
        &script(),
        "    if retry:\n        gone = ",
        "    if False:\n        gone = "
    ))
);

mod keeps_the_live_head {
    use super::*;
    negative_control!(
        ci_retry_decide_keeps_a_superseded_head,
        "every head taken for moved",
        expected = "the live head: not re-run",
        check_head(&edit(
            &script(),
            "return None if live == sha else",
            "return None if False else"
        ))
    );
}

// ---- The workflow ---------------------------------------------------------------------------------------------------

/// A workflow's text as JSON, parsed by PyYAML. YAML 1.1 reads the key `on` as `true`; it is put back as `on`.
fn parse(yml: &str) -> Value {
    let dir = Scratch::new("ci_retry_yaml");
    std::fs::create_dir_all(&*dir).expect("scratch folder");
    let file = dir.join("workflow.yml");
    std::fs::write(&file, yml).expect("workflow written");
    let out = Command::new("python3")
        .args([
            "-c",
            "import json, sys, yaml\n\
             d = yaml.safe_load(open(sys.argv[1]))\n\
             if True in d: d['on'] = d.pop(True)\n\
             print(json.dumps(d))",
        ])
        .arg(&file)
        .timed_output()
        .expect("ci_retry: python3 ran");
    assert!(out.status.success(), "ci_retry: the workflow is not YAML");
    serde_json::from_slice(&out.stdout).expect("ci_retry: PyYAML's JSON")
}

fn retry_yml() -> String {
    read(".github/workflows/retry.yml")
}

/// The `name:` of each workflow file.
fn workflow_names() -> Vec<String> {
    std::fs::read_dir(root().join(".github/workflows"))
        .expect("ci_retry: workflows")
        .map(|e| e.expect("entry").path())
        .map(|p| {
            let text = std::fs::read_to_string(&p).expect("workflow");
            parse(&text)["name"].as_str().expect("a name").to_owned()
        })
        .collect()
}

fn check_trigger(yml: &str) {
    let wf = parse(yml);
    let trigger = &wf["on"]["workflow_run"];
    let mut names: Vec<&str> = trigger["workflows"]
        .as_array()
        .expect("retry.yml: no workflow_run workflows")
        .iter()
        .map(|n| n.as_str().expect("a name"))
        .collect();
    names.sort_unstable();
    assert_eq!(names, ["ci", "gate", "mutants"], "the wrong workflows");
    let known = workflow_names();
    assert!(
        names.iter().all(|n| known.iter().any(|k| k == n)),
        "a workflow_run names no workflow"
    );
    assert_eq!(trigger["types"], json!(["completed"]), "the wrong event");
    assert!(
        trigger.get("branches").is_none() && trigger.get("branches-ignore").is_none(),
        "main and pull requests are not alike"
    );
}

#[test]
fn ci_retry_workflow_runs_on_each_completed_run() {
    check_trigger(&retry_yml());
}

negative_control!(
    ci_retry_workflow_runs_on_each_completed_run,
    "only `ci`'s runs",
    expected = "the wrong workflows",
    check_trigger(&edit(&retry_yml(), "[ci, mutants, gate]", "[ci]"))
);

mod branches {
    use super::*;
    negative_control!(
        ci_retry_workflow_runs_on_each_completed_run,
        "only `main`'s runs",
        expected = "main and pull requests are not alike",
        check_trigger(&edit(
            &retry_yml(),
            "    types: [completed]\n",
            "    types: [completed]\n    branches: [main]\n"
        ))
    );
}

fn check_job(yml: &str) {
    let wf = parse(yml);
    assert_eq!(wf["permissions"]["actions"], "write", "no `actions: write`");
    assert_eq!(wf["permissions"]["checks"], "read", "no `checks: read`");
    let job = &wf["jobs"]["retry"];
    let condition = job["if"].as_str().expect("retry.yml: the job has no `if:`");
    assert!(
        condition.contains("github.event.workflow_run.conclusion == 'failure'"),
        "the job does not ask that the run failed"
    );
    assert!(
        condition.contains("github.event.workflow_run.run_attempt < 3"),
        "the job does not stop at attempt 3"
    );
    let steps = job["steps"].as_array().expect("retry.yml: no steps");
    let last = steps.last().expect("a step");
    assert_eq!(
        last["run"].as_str(),
        Some("python3 -I .github/scripts/ci_retry.py decide"),
        "the job does not run the classifier"
    );
    for key in [
        "GH_TOKEN",
        "GH_REPO",
        "RUN_ID",
        "RUN_ATTEMPT",
        "RUN_CONCLUSION",
        "WORKFLOW_PATH",
        "HEAD_SHA",
        "HEAD_BRANCH",
    ] {
        assert!(last["env"].get(key).is_some(), "the step has no {key}");
    }
    assert!(
        !yml.lines().any(|l| {
            !l.trim_start().starts_with('#')
                && (l.contains("/rerun\"") || l.contains("gh run rerun"))
        }),
        "the workflow re-runs a whole run"
    );
}

#[test]
fn ci_retry_workflow_gates_on_the_conclusion_and_the_attempt() {
    check_job(&retry_yml());
}

negative_control!(
    ci_retry_workflow_gates_on_the_conclusion_and_the_attempt,
    "a cap one higher",
    expected = "the job does not stop at attempt 3",
    check_job(&edit(&retry_yml(), "run_attempt < 3", "run_attempt < 4"))
);

mod no_write {
    use super::*;
    negative_control!(
        ci_retry_workflow_gates_on_the_conclusion_and_the_attempt,
        "no permission to re-run",
        expected = "no `actions: write`",
        check_job(&edit(
            &retry_yml(),
            "  actions: write\n",
            "  actions: read\n"
        ))
    );
}

mod any_conclusion {
    use super::*;
    negative_control!(
        ci_retry_workflow_gates_on_the_conclusion_and_the_attempt,
        "every completed run",
        expected = "the job does not ask that the run failed",
        check_job(&edit(
            &retry_yml(),
            "github.event.workflow_run.conclusion == 'failure' && ",
            ""
        ))
    );
}

/// The jobs that hold the disk probe: `ci.yml`'s heavy Linux jobs. `gpu-metal`, on macOS, has no `/mnt`.
const PROBED: [&str; 5] = ["ci", "ci-release", "xtask-ci", "gpu-kernel", "gpu-lavapipe"];

fn check_probe(yml: &str) {
    let wf = parse(yml);
    for job in PROBED {
        let steps = wf["jobs"][job]["steps"]
            .as_array()
            .unwrap_or_else(|| panic!("ci.yml: no job `{job}`"));
        let probe = steps
            .iter()
            .position(|s| s["name"] == "Runner disks")
            .unwrap_or_else(|| panic!("job `{job}` has no disk probe"));
        let run = steps[probe]["run"].as_str().expect("the probe's script");
        assert!(
            run.contains("df -h / /mnt || true\n") && run.contains("lsblk || true\n"),
            "job `{job}`'s probe can fail the job, or measures too little"
        );
        let toolchain = steps
            .iter()
            .position(|s| {
                s["run"]
                    .as_str()
                    .is_some_and(|r| r.contains("rustup toolchain install"))
            })
            .expect("a toolchain step");
        assert!(
            probe < toolchain,
            "job `{job}` measures the disks only after the toolchain fills them"
        );
    }
}

#[test]
fn ci_retry_workflow_probes_both_disks() {
    check_probe(&read(".github/workflows/ci.yml"));
}

negative_control!(
    ci_retry_workflow_probes_both_disks,
    "a probe that can fail the job",
    expected = "probe can fail the job",
    check_probe(&edit(
        &read(".github/workflows/ci.yml"),
        "          df -h / /mnt || true\n",
        "          df -h / /mnt\n"
    ))
);

mod no_probe {
    use super::*;
    negative_control!(
        ci_retry_workflow_probes_both_disks,
        "`ci-release` with no probe",
        expected = "job `ci-release` has no disk probe",
        check_probe(&{
            let text = read(".github/workflows/ci.yml");
            let at = text.find("\n  ci-release:\n").expect("the job");
            let (head, tail) = text.split_at(at);
            format!(
                "{head}{}",
                tail.replacen("      - name: Runner disks\n", "      - name: Other\n", 1)
            )
        })
    );
}
