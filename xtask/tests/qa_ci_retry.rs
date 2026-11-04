//! QA tests for PR #392, `.github/scripts/ci_retry.py` and `.github/workflows/retry.yml`, written from the owner's words
//! of 4 Nov 2026 and the task's item 2, not from the script: a failed run's failed jobs are re-run "when every failed
//! job shows a runner-level signature", "never on assertion failures, timeouts or flakes", and the classifier is tested
//! "against saved logs".
//!
//! The classifier is driven only through its command line, `ci_retry.py classify DIR WORKFLOW`, against the repository's
//! real workflow files, on hand-built runs whose job names are those `ci.yml` gives. Every control here changes the
//! run, never the script, so that each holds whatever the script's text becomes.
//!
//! A gate is a job that fails only because a job it gathers did not pass: `ci.yml` says so of `ci-gate` ("a failed or
//! cancelled job turns it red"; "the skips turn it red"). A runner fault upstream of the jobs a gate gathers turns the
//! gate red through a skip, so a gate's red is the fault's trace there too.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};
use validation::negative_control;
use validation::spawn::Spawn;

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;

/// What the job's failure annotation says, verbatim from the saved run `41744485022`'s `ci (3)`, shortened at its end.
const NO_SPACE: &str = "Unhandled exception. System.IO.IOException: No space left on device : \
                        '/home/runner/actions-runner/cached/2.338.2/_diag/Worker_20261104-001916-utc.log'";
/// The saved run `41707146179`'s `xtask-ci (1)`, verbatim.
const LOST: &str = "The hosted runner lost communication with the server. Anything in your workflow that terminates \
                    the runner process, starves it for CPU/Memory, or blocks its network access can cause this error.";
/// A failed step's annotation, as every real test failure in the saved runs has it.
const REAL: &str = "Process completed with exit code 101.";
/// A job over its limit: GitHub's text. The job ends `cancelled`, so its state alone does not say so.
const JOB_TIMEOUT: &str =
    "The job running on runner GitHub Actions 3 has exceeded the maximum execution time of 360 minutes.";
/// The same as a real run has it: this repository's run `40558523181`, `ci (3)`, which ended `cancelled`.
const JOB_TIMEOUT_NOW: &str = "The job has exceeded the maximum execution time of 6h0m0s";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the workspace root")
        .to_path_buf()
}

/// A job of a run: its name, how it ended (`None`, a state GitHub did not give), and its failure annotation, if any.
type Job<'a> = (&'a str, Option<&'a str>, Option<&'a str>);

fn failed<'a>(name: &'a str, message: &'a str) -> Job<'a> {
    (name, Some("failure"), Some(message))
}

fn skipped(name: &str) -> Job<'_> {
    (name, Some("skipped"), None)
}

/// `ci_retry.py classify` on the directory `dir`, against `workflow` in `.github/workflows/`: its output.
fn classify(dir: &Path, workflow: &str) -> String {
    let out = Command::new("python3")
        .arg("-I")
        .arg(root().join(".github/scripts/ci_retry.py"))
        .arg("classify")
        .arg(dir)
        .arg(root().join(".github/workflows").join(workflow))
        .timed_output()
        .expect("python3 ran");
    assert!(out.status.success(), "the classifier failed");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn verdict(text: &str) -> bool {
    match text.lines().next() {
        Some("retry=true") => true,
        Some("retry=false") => false,
        _ => panic!("the classifier's first line is no verdict"),
    }
}

/// Whether a failed run at attempt 1 of `ci.yml`, made of `jobs`, is re-run. No job has a log: a dead runner leaves none.
fn retried(jobs: &[Job]) -> bool {
    let dir = Scratch::new("qa_ci_retry");
    std::fs::create_dir_all(dir.join("annotations")).expect("scratch folder");
    let listed: Vec<Value> = jobs
        .iter()
        .enumerate()
        .map(|(i, (name, conclusion, _))| {
            json!({"id": i + 1, "name": name, "status": "completed", "conclusion": conclusion})
        })
        .collect();
    for (i, (_, _, message)) in jobs.iter().enumerate() {
        if let Some(message) = message {
            let annotations = json!([{"annotation_level": "failure", "message": message}]);
            std::fs::write(
                dir.join("annotations").join(format!("{}.json", i + 1)),
                annotations.to_string(),
            )
            .expect("annotation written");
        }
    }
    std::fs::write(
        dir.join("run.json"),
        json!({"conclusion": "failure", "run_attempt": 1}).to_string(),
    )
    .expect("run written");
    std::fs::write(dir.join("jobs.json"), Value::from(listed).to_string()).expect("jobs written");
    verdict(&classify(&dir, "ci.yml"))
}

// ---- The owner's three saved runs, read as they are checked in --------------------------------------------------------

fn check_saved(cases: &[(&str, bool)]) {
    for (run, expected) in cases {
        let text = classify(&root().join("fixtures/ci_retry").join(run), "ci.yml");
        assert_eq!(verdict(&text), *expected, "the saved run {run}");
    }
}

/// "today's 'No space left on device' run, a lost-communication sample, and a real test failure that must NOT retry".
#[test]
fn qa_ci_retry_saved_runs_as_checked_in() {
    check_saved(&[
        ("41744485022", true),
        ("41707146179", true),
        ("41431397009", false),
    ]);
}

negative_control!(
    qa_ci_retry_saved_runs_as_checked_in,
    "the test failure taken for a run that retries",
    expected = "the saved run 41431397009",
    check_saved(&[("41431397009", true)])
);

// ---- A fault in each job of `ci.yml`, with the gate it turns red -------------------------------------------------------

/// Each job with a required gate that gathers it directly, then that gate.
const GATHERED: [(&str, &str); 7] = [
    ("ci (2)", "ci"),
    ("ci-checks", "ci"),
    ("ci-workspace", "ci"),
    ("xtask-ci (3)", "xtask-ci"),
    ("gpu-kernel suites", "gpu-kernel"),
    ("gpu-metal suites", "gpu-metal"),
    ("gpu-lavapipe suites", "gpu-lavapipe"),
];

fn check_gathered(message: &str) {
    for (job, gate) in GATHERED {
        assert!(
            retried(&[failed(job, message), failed(gate, REAL)]),
            "a fault in {job} and its gate: not re-run"
        );
    }
    assert!(
        retried(&[
            failed("ci-release-archive", message),
            skipped("ci-release (${{ matrix.shard }})"),
        ]),
        "a fault in ci-release-archive: not re-run"
    );
}

#[test]
fn qa_ci_retry_a_fault_in_each_job_and_its_gate() {
    check_gathered(LOST);
}

negative_control!(
    qa_ci_retry_a_fault_in_each_job_and_its_gate,
    "a real failure in place of the fault",
    expected = "a fault in ci (2) and its gate: not re-run",
    check_gathered(REAL)
);

/// `ci-archive` builds what the `ci` shards run. Out of disk there, the shards are skipped, and the skips turn `ci` red
/// (`ci.yml` § `ci-gate`): the one failed job besides the fault is that gate, and nothing but the fault is wrong.
fn check_archive(message: &str) {
    assert!(
        retried(&[
            failed("ci-archive", message),
            skipped("ci (${{ matrix.shard }})"),
            failed("ci", REAL),
        ]),
        "a fault in ci-archive, the shards skipped and ci red: not re-run"
    );
}

#[test]
fn qa_ci_retry_a_fault_in_the_archive_turns_ci_red_through_its_skipped_shards() {
    check_archive(NO_SPACE);
}

negative_control!(
    qa_ci_retry_a_fault_in_the_archive_turns_ci_red_through_its_skipped_shards,
    "a real failure in the archive job",
    expected = "a fault in ci-archive, the shards skipped and ci red: not re-run",
    check_archive(REAL)
);

/// `changes` feeds every job. Lost there, all of them are skipped and all five required gates go red.
fn check_changes(message: &str) {
    assert!(
        retried(&[
            failed("changes", message),
            skipped("ci-archive"),
            skipped("ci (${{ matrix.shard }})"),
            skipped("ci-checks"),
            skipped("ci-workspace"),
            skipped("xtask-ci (${{ matrix.shard }})"),
            failed("ci", REAL),
            failed("xtask-ci", REAL),
            failed("gpu-kernel", REAL),
            failed("gpu-metal", REAL),
            failed("gpu-lavapipe", REAL),
        ]),
        "a fault in changes and every gate red: not re-run"
    );
}

#[test]
fn qa_ci_retry_a_fault_in_changes_turns_every_gate_red() {
    check_changes(LOST);
}

negative_control!(
    qa_ci_retry_a_fault_in_changes_turns_every_gate_red,
    "a real failure in `changes`",
    expected = "a fault in changes and every gate red: not re-run",
    check_changes(REAL)
);

// ---- What is not re-run ------------------------------------------------------------------------------------------------

/// With `real` the failure of the jobs that did not fault.
fn check_kept(real: &str) {
    let cases: [(&str, Vec<Job>); 8] = [
        (
            "a failure beside a fault",
            vec![
                failed("ci (3)", NO_SPACE),
                failed("ci (4)", real),
                failed("ci", REAL),
            ],
        ),
        (
            "a failure in the archive",
            vec![
                failed("ci-archive", real),
                skipped("ci (${{ matrix.shard }})"),
                failed("ci", REAL),
            ],
        ),
        (
            "a failure in a job the gate gathers, beside a fault upstream of the gate's others",
            vec![
                failed("ci-archive", NO_SPACE),
                skipped("ci (${{ matrix.shard }})"),
                failed("ci-checks", real),
                failed("ci", REAL),
            ],
        ),
        (
            "a failure in one job, the signature in a job that passed",
            vec![
                ("ci (2)", Some("success"), Some(NO_SPACE)),
                failed("ci (3)", real),
                failed("ci", REAL),
            ],
        ),
        (
            "a job cancelled, no signature on it, beside a fault",
            vec![
                failed("ci (3)", NO_SPACE),
                ("ci (4)", Some("cancelled"), None),
                failed("ci", REAL),
            ],
        ),
        (
            "a job with no state, beside a fault",
            vec![
                failed("ci (3)", NO_SPACE),
                ("ci (4)", None, None),
                failed("ci", REAL),
            ],
        ),
        (
            "a job over its limit, which ends cancelled, beside a fault",
            vec![
                failed("ci (3)", NO_SPACE),
                ("ci (4)", Some("cancelled"), Some(JOB_TIMEOUT)),
                failed("ci", REAL),
            ],
        ),
        (
            "a job over its limit, in GitHub's other wording, beside a fault",
            vec![
                failed("ci (3)", NO_SPACE),
                ("ci (4)", Some("cancelled"), Some(JOB_TIMEOUT_NOW)),
                failed("ci", REAL),
            ],
        ),
    ];
    for (case, jobs) in cases {
        assert!(!retried(&jobs), "{case}: re-run");
    }
}

#[test]
fn qa_ci_retry_keeps_every_run_with_a_failure_of_its_own() {
    check_kept(REAL);
}

negative_control!(
    qa_ci_retry_keeps_every_run_with_a_failure_of_its_own,
    "the failure in the first case a fault after all",
    expected = "a failure beside a fault: re-run",
    check_kept(LOST)
);

// ---- The workflow ------------------------------------------------------------------------------------------------------

/// The workflow's text as JSON, parsed by PyYAML, `on` under that key.
fn parse(yml: &str) -> Value {
    let dir = Scratch::new("qa_ci_retry_yaml");
    std::fs::create_dir_all(&*dir).expect("scratch folder");
    let file = dir.join("workflow.yml");
    std::fs::write(&file, yml).expect("workflow written");
    let out = Command::new("python3")
        .args([
            "-c",
            "import json, sys, yaml\n\
             d = yaml.safe_load(open(sys.argv[1]))\n\
             d['on'] = d.pop(True)\n\
             print(json.dumps(d))",
        ])
        .arg(&file)
        .timed_output()
        .expect("python3 ran");
    assert!(out.status.success(), "the workflow is not YAML");
    serde_json::from_slice(&out.stdout).expect("PyYAML's JSON")
}

/// `workflow_run` holds `actions: write` whoever opened the pull request it reads, so nothing of that run may execute:
/// the job checks out only `main`'s tree, keeps no token in it, and no `run:` line expands what the run supplies.
fn check_runs_no_code(yml: &str) {
    let wf = parse(yml);
    let on = wf["on"].as_object().expect("`on` is a table");
    assert_eq!(
        on.keys().collect::<Vec<_>>(),
        ["workflow_run"],
        "the workflow starts on something else"
    );
    assert_eq!(
        wf["permissions"],
        json!({"actions": "write", "checks": "read", "contents": "read"}),
        "the workflow holds other permissions"
    );
    let jobs = wf["jobs"].as_object().expect("jobs");
    for job in jobs.values() {
        for step in job["steps"].as_array().expect("steps") {
            let uses = step["uses"].as_str().unwrap_or_default();
            if uses.starts_with("actions/checkout@") {
                assert!(
                    step["with"].get("ref").is_none() && step["with"].get("repository").is_none(),
                    "the job checks out the run's code"
                );
                assert_eq!(
                    step["with"]["persist-credentials"],
                    json!(false),
                    "the checkout keeps the token on disk"
                );
            }
            assert!(
                !step["run"].as_str().unwrap_or_default().contains("${{"),
                "a `run:` line expands an expression"
            );
        }
    }
}

fn retry_yml() -> String {
    std::fs::read_to_string(root().join(".github/workflows/retry.yml")).expect("retry.yml")
}

#[test]
fn qa_ci_retry_workflow_runs_no_code_from_the_run_it_reads() {
    check_runs_no_code(&retry_yml());
}

negative_control!(
    qa_ci_retry_workflow_runs_no_code_from_the_run_it_reads,
    "a checkout of the failed run's head",
    expected = "the job checks out the run's code",
    check_runs_no_code(&retry_yml().replace(
        "          persist-credentials: false\n",
        "          persist-credentials: false\n          ref: ${{ github.event.workflow_run.head_sha }}\n",
    ))
);
