//! The per-merge tier (the owner's CI model, R-909 item 2, approved 9 Nov 2026: "Each merge to main (per merge commit;
//! advisory; never blocks a merge) ... Correctness failure (physics, validation, bit-for-bit, golden) = red main (R-681);
//! other failures file a top-priority task"; R-908: "A failing process test never makes main red; it files a task"):
//! `ci.yml`'s jobs on a push and their verdict, `per-merge`, which `.github/scripts/ci_per_merge.py` gives.
//!
//! - `ci_per_merge_verdict_*`: the classifier, `classify` on a run written here: no failure is green; a product test's
//!   failure is red, a process test's alone a task, both together red; a failure outside the test steps, or the build
//!   failing inside one, a task; a test step that names no failed test, or whose log is gone, red; the jobs that are
//!   never correctness a task, the release profile's always red, and a job it does not know red; nextest's lines read
//!   whatever their status, counter and colours, a retried attempt no failure of its own; each failed test step read in
//!   its own part of the job's log, a part opening in its step's second its own and never a skipped step's; two failed
//!   test steps whose parts cannot be told apart red; a test step skipped after a failed one red, one skipped before it
//!   not; nextest's summary counting fewer tests run than selected red.
//! - `ci_per_merge_process_class_is_the_process_profiles`: the classifier's process tests are the `process` profile's
//!   binaries, and none of the product correctness the owner named (R-909).
//! - `ci_per_merge_decide_*`: `decide`, with a stand-in `gh`: on `main`, one issue labelled `per-merge` for the other
//!   failures, or a comment on the open one of that title, and none off `main`; red exits 1; it never reads its own job;
//!   jobs it cannot read are red; a task it cannot file is an error annotation, not red.
//! - `ci_per_merge_tier_*`: `ci.yml`, parsed with PyYAML: it runs on each push to `main`, each merge commit in a run of
//!   its own and never cancelled; `per-merge` runs on a push alone, last, after every job a push runs, whatever each
//!   ended in, with the permissions it uses and nothing self-hosted; the classifier knows each of those jobs by name;
//!   and the tier runs the owner's list: the `ci`, `ci-workspace` and `gpu-kernel` profiles between them select every
//!   test (the `ci` profile leaving out exactly the other two's), `cargo xtask golden --all` and `cargo xtask gate
//!   decode-agreement` on lavapipe and on Metal, and the release profile; and the jobs of those three profiles run
//!   every test after a failure.
//!
//! Each test has its registered negative control (R-176), which feeds the same check a script, run, workflow or
//! configuration differing in the one respect it turns on. Unix only: the stand-in `gh` is an executable script.
#![cfg(unix)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};
use validation::negative_control;
use validation::spawn::{write_executable, Spawn};

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;

const SCRIPT: &str = ".github/scripts/ci_per_merge.py";
const CONFIG: &str = ".config/nextest.toml";
const CI: &str = ".github/workflows/ci.yml";

/// Real nextest lines, with their timestamps and colours: `ci`'s job 129521436250 of run 42722794689 (9 Nov 2026), a
/// product test's failure, as its status line printed it.
const PRODUCT_FAIL: &str = "2026-11-09T23:57:32.9720251Z \u{1b}[31;1m        FAIL\u{1b}[0m [   0.006s] ( 319/5514) \
                            \u{1b}[35;1mengine::branch_read_gpu\u{1b}[0m \
                            \u{1b}[34;1mbranch_read_gpu_canon_stage_runs_from_the_production_module\u{1b}[0m";
const SUMMARY: &str = "2026-11-09T23:57:41.8462447Z \u{1b}[31;1m     Summary\u{1b}[0m [  71.045s] \u{1b}[1m322\u{1b}[0m/\
                       \u{1b}[1m5514\u{1b}[0m tests run: \u{1b}[1m320\u{1b}[0m \u{1b}[32;1mpassed\u{1b}[0m, \
                       \u{1b}[1m2\u{1b}[0m \u{1b}[31;1mfailed\u{1b}[0m";
const ERROR: &str = "2026-11-09T23:57:42.1794288Z ##[error]Process completed with exit code 100.";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the workspace root")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("ci_per_merge: {rel}: {e}"))
}

/// `text` with its first `from` replaced by `to`, which must be in it: a control's one edit.
#[cfg(feature = "controls")]
fn edit(text: &str, from: &str, to: &str) -> String {
    assert!(
        text.contains(from),
        "ci_per_merge: a control's edit target is gone: {from}"
    );
    text.replacen(from, to, 1)
}

/// A nextest status line as a job's log has it, for `binary` and `test`, ended with `status`.
fn line(status: &str, binary: &str, test: &str) -> String {
    format!("2026-11-10T01:00:00.0000000Z {status:>12} [   1.234s] (  7/900) {binary} {test}")
}

/// A test step's log: its start, `lines`, and the error GitHub writes when it fails.
fn log(step: &str, lines: &[String]) -> String {
    let mut out = vec![
        "\u{feff}2026-11-10T00:59:00.0000000Z Current runner version: '2.338.2'".to_owned(),
        format!("2026-11-10T00:59:01.0000000Z ##[group]Run {step}"),
    ];
    out.extend(lines.iter().cloned());
    out.push(ERROR.to_owned());
    out.join("\n") + "\n"
}

/// A job of the jobs API: its `id`, `name` and `conclusion`, and its steps, `failed` among them ended `failure`.
fn job(id: u64, name: &str, conclusion: &str, failed: &str) -> Value {
    let mut steps = vec![json!({"name": "Set up job", "conclusion": "success"})];
    if !failed.is_empty() {
        steps.push(json!({"name": failed, "conclusion": "failure"}));
        steps.push(json!({"name": "Post Run actions/checkout", "conclusion": "skipped"}));
    }
    json!({"id": id, "name": name, "status": "completed", "conclusion": conclusion, "steps": steps})
}

/// A run as the classifier reads it: its jobs and the logs of those that failed, by job id.
#[derive(Clone)]
struct Run {
    jobs: Vec<Value>,
    logs: BTreeMap<u64, String>,
}

impl Run {
    /// The per-merge tier's jobs, every one passed or skipped as on a green push.
    fn green() -> Run {
        let mut jobs = vec![job(1, "changes", "success", "")];
        for (id, name) in [
            (2, "ci-archive"),
            (3, "ci (1)"),
            (4, "ci (2)"),
            (5, "ci (3)"),
            (6, "ci (4)"),
            (7, "ci-workspace"),
            (8, "ci-checks"),
            (9, "web-skeleton"),
            (10, "ci-release-archive"),
            (11, "ci-release (1)"),
            (12, "ci-release (2)"),
            (13, "gpu-kernel suites"),
            (14, "xtask-ci (1)"),
            (15, "gpu-metal suites"),
            (16, "gpu-lavapipe suites"),
            (17, "gpu-lavapipe"),
            (18, "gpu-metal"),
        ] {
            jobs.push(job(id, name, "success", ""));
        }
        for (id, name) in [
            (19, "fmt"),
            (20, "clippy"),
            (21, "build"),
            (22, "ci"),
            (23, "process"),
        ] {
            jobs.push(job(id, name, "skipped", ""));
        }
        Run {
            jobs,
            logs: BTreeMap::new(),
        }
    }

    /// This run with the job named `name` ended `conclusion` at the step `step`, its log `log` (none if `None`).
    fn failing(mut self, name: &str, conclusion: &str, step: &str, log: Option<String>) -> Run {
        let at = self
            .jobs
            .iter()
            .position(|j| j["name"] == name && j["conclusion"] != "skipped")
            .unwrap_or_else(|| {
                self.jobs
                    .push(job(100 + self.jobs.len() as u64, name, "success", ""));
                self.jobs.len() - 1
            });
        let id = self.jobs[at]["id"].as_u64().expect("an id");
        self.jobs[at] = job(id, name, conclusion, step);
        match log {
            Some(text) => self.logs.insert(id, text),
            None => self.logs.remove(&id),
        };
        self
    }

    /// This run with `name` failed at the test step `step`, its log naming `lines`.
    fn test_failed(self, name: &str, step: &str, lines: &[String]) -> Run {
        let text = log(step, lines);
        self.failing(name, "failure", step, Some(text))
    }

    fn write(&self, dir: &Path) {
        std::fs::create_dir_all(dir.join("logs")).expect("logs");
        std::fs::write(
            dir.join("jobs.json"),
            Value::from(self.jobs.clone()).to_string(),
        )
        .expect("jobs");
        for (id, text) in &self.logs {
            std::fs::write(dir.join("logs").join(format!("{id}.log")), text).expect("a log");
        }
    }
}

/// `script` run by python3 in a scratch folder with `args`, after `dir` is filled by `fill`: its exit, stdout, stderr.
fn python(script: &str, args: &[&str], fill: impl FnOnce(&Path)) -> (bool, String, String) {
    let dir = Scratch::new("ci_per_merge");
    std::fs::create_dir_all(&*dir).expect("scratch folder");
    let file = dir.join("ci_per_merge.py");
    std::fs::write(&file, script).expect("script written");
    fill(&dir);
    let out = Command::new("python3")
        .arg("-I")
        .arg(&file)
        .args(args)
        .current_dir(&*dir)
        .timed_output()
        .expect("ci_per_merge: python3 ran");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// `run` classified by `script` against the configuration `config`: what it printed.
fn classified_with(script: &str, run: &Run, config: &str) -> String {
    let (ok, out, err) = python(script, &["classify", "run", "nextest.toml"], |dir| {
        run.write(&dir.join("run"));
        std::fs::write(dir.join("nextest.toml"), config).expect("config");
    });
    assert!(ok, "ci_per_merge: the classifier failed: {err}");
    out
}

fn classified(script: &str, run: &Run) -> String {
    classified_with(script, run, &read(CONFIG))
}

/// Asserts the verdict on `run` is `want`, and, for each `(job, kind, says)`, that the job is judged `kind` saying `says`.
fn check_verdict(script: &str, case: &str, run: &Run, want: &str, jobs: &[(&str, &str, &str)]) {
    let text = classified(script, run);
    let verdict = text.lines().next().unwrap_or_default();
    assert_eq!(
        verdict,
        format!("verdict={want}"),
        "ci_per_merge: {case}: the verdict is not {want}"
    );
    for (name, kind, says) in jobs {
        assert!(
            text.lines()
                .any(|l| l.starts_with(&format!("job={name}: {kind}: ")) && l.contains(says)),
            "ci_per_merge: {case}: {name} is not judged {kind} ({says})"
        );
    }
    let judged = text.lines().filter(|l| l.starts_with("job=")).count();
    assert_eq!(
        judged,
        jobs.len(),
        "ci_per_merge: {case}: a job is judged that did not fail, or one that failed is not"
    );
}

fn script() -> String {
    read(SCRIPT)
}

const CI_STEP: &str = "cargo nextest run --archive-file … --partition hash:2/4";

// ---- The classifier -------------------------------------------------------------------------------------------------

fn check_green(script: &str) {
    check_verdict(script, "a green run", &Run::green(), "green", &[]);
}

#[test]
fn ci_per_merge_verdict_green_when_no_job_failed() {
    check_green(&script());
}

negative_control!(
    ci_per_merge_verdict_green_when_no_job_failed,
    "a classifier that reads a skipped job as failed",
    expected = "ci_per_merge: a green run: the verdict is not green",
    check_green(&edit(
        &script(),
        "PASSED = (\"success\", \"skipped\", \"neutral\")",
        "PASSED = (\"success\", \"neutral\")"
    ))
);

fn check_product(script: &str) {
    let run = Run::green().test_failed(
        "ci (2)",
        CI_STEP,
        &[PRODUCT_FAIL.to_owned(), SUMMARY.to_owned()],
    );
    check_verdict(
        script,
        "a product test failing in a `ci` shard",
        &run,
        "red",
        &[(
            "ci (2)",
            "correctness",
            "product tests failed: engine::branch_read_gpu branch_read_gpu_canon_stage_runs_from_the_production_module",
        )],
    );
}

#[test]
fn ci_per_merge_verdict_product_test_failure_is_red() {
    check_product(&script());
}

negative_control!(
    ci_per_merge_verdict_product_test_failure_is_red,
    "a classifier that reads every test as a process test (red still, by the summary's tests left unrun, but not for the \
     product test)",
    expected = "ci_per_merge: a product test failing in a `ci` shard: ci (2) is not judged correctness (product tests failed",
    check_product(&edit(
        &script(),
        "if failed and any(b not in process for b, _ in failed):",
        "if failed and any(b in process for b, _ in failed):"
    ))
);

/// A process test, `xtask::ci`'s, failing in a `ci` shard, checked against `config`.
fn check_process(config: &str) {
    let run = Run::green().test_failed(
        "ci (3)",
        CI_STEP,
        &[line("FAIL", "xtask::ci", "ci_list_runs_no_control")],
    );
    let text = classified_with(&script(), &run, config);
    assert!(
        text.starts_with("verdict=task\n")
            && text.contains("job=ci (3): other: `cargo nextest run --archive-file … --partition hash:2/4`: process tests failed: xtask::ci ci_list_runs_no_control"),
        "ci_per_merge: a process test's failure is not a task"
    );
}

#[test]
fn ci_per_merge_verdict_process_test_failure_is_a_task() {
    check_process(&read(CONFIG));
}

negative_control!(
    ci_per_merge_verdict_process_test_failure_is_a_task,
    "a configuration whose `process` profile is gone, so no test reads as a process test",
    expected = "ci_per_merge: a process test's failure is not a task",
    check_process(&edit(
        &read(CONFIG),
        "[profile.process]\n",
        "[profile.process-gone]\n"
    ))
);

fn check_mixed(script: &str) {
    let run = Run::green().test_failed(
        "ci-workspace",
        "cargo nextest run -p xtask",
        &[
            line(
                "FAIL",
                "xtask::controls",
                "controls_on_this_workspace_skips_gui",
            ),
            line(
                "FAIL",
                "xtask::qa_TASK-M0-06_list",
                "qa_m006_ci_list_lists_golden_cases_without_a_device",
            ),
        ],
    );
    check_verdict(
        script,
        "a process test and the golden listing's failing together",
        &run,
        "red",
        &[("ci-workspace", "correctness", "xtask::qa_TASK-M0-06_list")],
    );
}

#[test]
fn ci_per_merge_verdict_a_product_failure_beside_a_process_one_is_red() {
    check_mixed(&script());
}

negative_control!(
    ci_per_merge_verdict_a_product_failure_beside_a_process_one_is_red,
    "a classifier that calls a failure red only when every failed test is a product test",
    expected = "ci_per_merge: a process test and the golden listing's failing together: the verdict is not red",
    check_mixed(&edit(
        &script(),
        "if failed and any(b not in process for b, _ in failed):",
        "if failed and all(b not in process for b, _ in failed):"
    ))
);

/// A job that runs the product suite failing outside its test steps: the runner's disk, and the archive's build.
fn check_outside(script: &str, step: &str) {
    let run = Run::green()
        .failing("gpu-kernel suites", "failure", step, Some(log(step, &[])))
        .failing(
            "ci-archive",
            "failure",
            "cargo nextest archive --workspace",
            Some(log("cargo nextest archive --workspace", &[])),
        );
    check_verdict(
        script,
        "a failure outside the test steps",
        &run,
        "task",
        &[
            ("gpu-kernel suites", "other", "no test step failed"),
            ("ci-archive", "other", "no test step failed"),
        ],
    );
}

#[test]
fn ci_per_merge_verdict_a_failure_outside_the_tests_is_a_task() {
    check_outside(&script(), "Free runner disk");
}

negative_control!(
    ci_per_merge_verdict_a_failure_outside_the_tests_is_a_task,
    "the same job failing in its test step, its log naming no test",
    expected = "ci_per_merge: a failure outside the test steps: the verdict is not task",
    check_outside(
        &script(),
        "cargo nextest run -p validation -p xtask -p engine"
    )
);

const KERNEL_STEP: &str = "cargo nextest run -p validation -p xtask -p engine";

/// A test step whose build fails, with `lines` in its log.
fn check_build(lines: &[String]) {
    let run = Run::green().test_failed("gpu-kernel suites", KERNEL_STEP, lines);
    check_verdict(
        &script(),
        "a test step whose build failed",
        &run,
        "task",
        &[("gpu-kernel suites", "other", "the build failed")],
    );
}

const COMPILE_ERROR: &str = "2026-11-10T01:00:00.0000000Z \u{1b}[1m\u{1b}[91merror\u{1b}[0m\u{1b}[1m: could not \
                             compile `engine` (test \"canon_gpu\") due to 1 previous error\u{1b}[0m";

#[test]
fn ci_per_merge_verdict_a_build_failing_in_a_test_step_is_a_task() {
    check_build(&[
        "2026-11-10T01:00:00.0000000Z error[E0425]: cannot find value `x` in this scope".to_owned(),
        COMPILE_ERROR.to_owned(),
    ]);
}

negative_control!(
    ci_per_merge_verdict_a_build_failing_in_a_test_step_is_a_task,
    "the same step's log without cargo's could-not-compile line",
    expected = "ci_per_merge: a test step whose build failed: the verdict is not task",
    check_build(&[
        "2026-11-10T01:00:00.0000000Z error[E0425]: cannot find value `x` in this scope".to_owned()
    ])
);

/// Test steps that name no failed test, and one whose log is gone, are each red.
fn check_unnamed(script: &str) {
    for (case, name, step, lines) in [
        (
            "a golden run",
            "gpu-lavapipe suites",
            "cargo xtask golden --all",
            vec![],
        ),
        (
            "a gate",
            "gpu-metal suites",
            "cargo xtask gate decode-agreement",
            vec![],
        ),
        (
            "a doctest",
            "ci (1)",
            "cargo test --workspace --doc",
            vec![
                "2026-11-10T01:00:00.0000000Z test result: FAILED. 41 passed; 1 failed".to_owned(),
            ],
        ),
    ] {
        let run = Run::green().test_failed(name, step, &lines);
        check_verdict(
            script,
            case,
            &run,
            "red",
            &[(name, "correctness", "naming no failed test")],
        );
    }
    let gone = Run::green().failing("gpu-kernel suites", "failure", KERNEL_STEP, None);
    check_verdict(
        script,
        "a test step without its log",
        &gone,
        "red",
        &[(
            "gpu-kernel suites",
            "correctness",
            "its log could not be read",
        )],
    );
}

#[test]
fn ci_per_merge_verdict_a_test_step_naming_no_test_is_red() {
    check_unnamed(&script());
}

negative_control!(
    ci_per_merge_verdict_a_test_step_naming_no_test_is_red,
    "a classifier that calls a test step naming no failed test a task",
    expected = "ci_per_merge: a golden run: the verdict is not red",
    check_unnamed(&edit(
        &script(),
        "return \"correctness\", f\"{step} failed, its log naming no failed test\"",
        "return \"other\", f\"{step} failed, its log naming no failed test\""
    ))
);

mod a_lost_log {
    use super::*;
    negative_control!(
        ci_per_merge_verdict_a_test_step_naming_no_test_is_red,
        "a classifier that calls a test step whose log is gone a task",
        expected = "ci_per_merge: a test step without its log: the verdict is not red",
        check_unnamed(&edit(
            &script(),
            "return \"correctness\", f\"{step} failed and its log could not be read\"",
            "return \"other\", f\"{step} failed and its log could not be read\""
        ))
    );
}

/// The jobs that are never correctness, each failing in a step that would be a test step elsewhere, or none.
fn check_other(run: &Run, jobs: &[(&str, &str, &str)]) {
    check_verdict(&script(), "jobs outside correctness", run, "task", jobs);
}

fn other_run(unknown: bool) -> Run {
    let run = Run::green()
        .test_failed("xtask-ci (1)", "cargo xtask ci --partition 1/4", &[])
        .failing(
            "ci-checks",
            "failure",
            "cargo clippy --workspace --all-targets -- -D warnings",
            None,
        )
        .failing(
            "web-skeleton",
            "failure",
            "cargo rustc -p gui (wasm32, the web feature)",
            None,
        )
        .failing("changes", "failure", "scripts/ci_changes.sh detect", None);
    if unknown {
        run.test_failed(
            "ci-unknown",
            "cargo nextest run -p kernel",
            &[line("FAIL", "xtask::ci", "x")],
        )
    } else {
        run
    }
}

const OTHER_JUDGED: [(&str, &str, &str); 4] = [
    ("xtask-ci (1)", "other", "not a correctness job"),
    ("ci-checks", "other", "not a correctness job"),
    ("web-skeleton", "other", "not a correctness job"),
    ("changes", "other", "not a correctness job"),
];

#[test]
fn ci_per_merge_verdict_jobs_outside_correctness_are_a_task() {
    check_other(&other_run(false), &OTHER_JUDGED);
}

negative_control!(
    ci_per_merge_verdict_jobs_outside_correctness_are_a_task,
    "a run with a job the classifier does not know failing beside them, its failed test a process test's",
    expected = "ci_per_merge: jobs outside correctness: the verdict is not task",
    check_other(&other_run(true), &OTHER_JUDGED)
);

/// The release profile's jobs: a process test failing at `--release` and the archive's build, each red (REQ-TOOL-223).
fn check_release(shard: &str) {
    let run = Run::green()
        .test_failed(
            shard,
            "cargo nextest run --archive-file … --partition hash:1/2",
            &[line(
                "SIGKILL",
                "xtask::ci_disk_sampler",
                "ci_disk_sampler_does_not_outlive_the_step",
            )],
        )
        .failing(
            "ci-release-archive",
            "failure",
            "cargo nextest archive --workspace --release",
            Some(log(
                "cargo nextest archive --workspace --release",
                &[COMPILE_ERROR.to_owned()],
            )),
        );
    check_verdict(
        &script(),
        "the release profile's failures",
        &run,
        "red",
        &[
            (shard, "correctness", "REQ-TOOL-223"),
            ("ci-release-archive", "correctness", "REQ-TOOL-223"),
        ],
    );
}

#[test]
fn ci_per_merge_verdict_every_release_failure_is_red() {
    check_release("ci-release (1)");
}

negative_control!(
    ci_per_merge_verdict_every_release_failure_is_red,
    "the same process test failing in a `ci` shard, not a release shard",
    expected = "ci_per_merge: the release profile's failures: ci (1) is not judged correctness (REQ-TOOL-223)",
    check_release("ci (1)")
);

/// nextest's failed statuses, with and without a counter, and a retried attempt that failed before its test passed:
/// only the last failure counts. nextest's time-limit status is not written here: its name is a retired term of the
/// vocabulary lint (canonical_spec §8), which reads this file and not the script; the script's pattern takes it beside
/// these.
fn check_statuses(retried: &str) {
    let lines = [
        line("ABORT", "xtask::jobq", "jobq_a"),
        line("SIGSEGV", "xtask::contain", "contain_b"),
        line("LEAK-FAIL", "xtask::ci_retry", "retry_c"),
        "2026-11-10T01:00:00.0000000Z         FAIL [  12.000s] xtask::deps deps_d".to_owned(),
        line(retried, "engine::canon_gpu", "canonicaliser_seam_sweep"),
        line("FLAKY 2/3", "engine::canon_gpu", "canonicaliser_seam_sweep"),
    ];
    let run = Run::green().test_failed("ci (4)", CI_STEP, &lines);
    check_verdict(
        &script(),
        "nextest's statuses",
        &run,
        "task",
        &[(
            "ci (4)",
            "other",
            "process tests failed: xtask::jobq jobq_a, xtask::contain contain_b, xtask::ci_retry retry_c, xtask::deps deps_d",
        )],
    );
}

#[test]
fn ci_per_merge_verdict_reads_each_nextest_status() {
    check_statuses("TRY 1 FAIL");
}

negative_control!(
    ci_per_merge_verdict_reads_each_nextest_status,
    "the product test's attempt read as its last failure",
    expected = "ci_per_merge: nextest's statuses: the verdict is not task",
    check_statuses("FAIL")
);

// ---- Every failed test step, and the suite run whole ----------------------------------------------------------------

/// A job of the jobs API that failed, its steps each `(name, conclusion, start)`, the start `HH:MM:SS` on 10 Nov 2026
/// as the API gives it, to the second, or none if empty.
fn timed_job(id: u64, name: &str, steps: &[(&str, &str, &str)]) -> Value {
    let steps: Vec<Value> = steps
        .iter()
        .map(|(step, conclusion, at)| {
            let mut s = json!({"name": step, "conclusion": conclusion});
            if !at.is_empty() {
                s["started_at"] = json!(format!("2026-11-10T{at}Z"));
            }
            s
        })
        .collect();
    json!({"id": id, "name": name, "status": "completed", "conclusion": "failure", "steps": steps})
}

/// A log line written at `at` (`HH:MM:SS.fffffff` on 10 Nov 2026).
fn at(time: &str, text: &str) -> String {
    format!("2026-11-10T{time}Z {text}")
}

/// A nextest status line written at `at`.
fn status_at(time: &str, status: &str, binary: &str, test: &str) -> String {
    format!("2026-11-10T{time}Z {status:>12} [   1.234s] (  7/900) {binary} {test}")
}

impl Run {
    /// This run with its job named as `job` is replaced by `job`, its log `lines`.
    fn with(mut self, job: Value, lines: &[String]) -> Run {
        let at = self
            .jobs
            .iter()
            .position(|j| j["name"] == job["name"])
            .expect("a job of the run");
        let id = job["id"].as_u64().expect("an id");
        self.jobs[at] = job;
        self.logs.insert(id, lines.join("\n") + "\n");
        self
    }
}

const WORKSPACE_TESTS: &str = "cargo nextest run -p xtask";
const WORKSPACE_M0_24: &str = "cargo nextest run -p validation --test qa_TASK-M0-24";
const WORKSPACE_CONTROLS: &str =
    "cargo nextest run --features controls (the default-feature listing's controls)";
const WORKSPACE_DOC: &str = "cargo test -p xtask --doc";

/// `ci-workspace` failing its first test step with a process test, whose closing line is written in the second the
/// next step starts, and its doctest step after it naming no failed test: each step read in its own part of the log.
fn check_each_step(script: &str) {
    let job = timed_job(
        7,
        "ci-workspace",
        &[
            ("Set up job", "success", "01:00:00"),
            (WORKSPACE_TESTS, "failure", "01:00:02"),
            (WORKSPACE_M0_24, "success", "01:00:09"),
            (WORKSPACE_CONTROLS, "success", "01:00:11"),
            ("Runner disk after the tests", "skipped", "01:00:12"),
            (WORKSPACE_DOC, "failure", "01:00:12"),
            ("Complete job", "success", "01:00:20"),
        ],
    );
    let lines = [
        "\u{feff}2026-11-10T01:00:00.1000000Z Current runner version: '2.338.2'".to_owned(),
        at(
            "01:00:02.1000000",
            &format!("##[group]Run {WORKSPACE_TESTS}"),
        ),
        status_at(
            "01:00:05.0000000",
            "FAIL",
            "xtask::ci",
            "ci_list_runs_no_control",
        ),
        status_at(
            "01:00:09.6000000",
            "FAIL",
            "xtask::ci",
            "ci_list_runs_no_control",
        ),
        at(
            "01:00:09.7000000",
            "##[error]Process completed with exit code 100.",
        ),
        at(
            "01:00:09.8000000",
            &format!("##[group]Run {WORKSPACE_M0_24}"),
        ),
        at(
            "01:00:11.0000000",
            "##[group]Run cargo nextest run -p xtask --features controls --test nextest",
        ),
        at("01:00:12.2000000", &format!("##[group]Run {WORKSPACE_DOC}")),
        at(
            "01:00:13.0000000",
            "test result: FAILED. 41 passed; 1 failed",
        ),
        at(
            "01:00:13.1000000",
            "##[error]Process completed with exit code 101.",
        ),
    ];
    let run = Run::green().with(job, &lines);
    check_verdict(
        script,
        "a doctest failing after a process test",
        &run,
        "red",
        &[(
            "ci-workspace",
            "correctness",
            "`cargo nextest run -p xtask`: process tests failed: xtask::ci ci_list_runs_no_control; \
             `cargo test -p xtask --doc` failed, its log naming no failed test",
        )],
    );
}

#[test]
fn ci_per_merge_verdict_reads_each_failed_test_step() {
    check_each_step(&script());
}

negative_control!(
    ci_per_merge_verdict_reads_each_failed_test_step,
    "a classifier that reads the job's whole log for each failed step, so the process test's lines answer for the doctest",
    expected = "ci_per_merge: a doctest failing after a process test: the verdict is not red",
    check_each_step(&edit(
        &script(),
        "    own = parts(every, log)\n",
        "    own = {at: clean(log) for at in tests}\n"
    ))
);

/// `ci-workspace` failing two test steps, each with a process test alone: the second step's part opens in the second
/// it started, beside a skipped step started in that second too, and is read as its own.
fn check_two_process_steps(script: &str) {
    let job = timed_job(
        7,
        "ci-workspace",
        &[
            ("Set up job", "success", "01:00:00"),
            (WORKSPACE_TESTS, "failure", "01:00:02"),
            (WORKSPACE_M0_24, "success", "01:00:09"),
            (WORKSPACE_CONTROLS, "failure", "01:00:11"),
            ("Runner disk after the tests", "skipped", "01:00:11"),
            (WORKSPACE_DOC, "success", "01:00:30"),
        ],
    );
    let lines = [
        at("01:00:00.1000000", "Current runner version: '2.338.2'"),
        at(
            "01:00:02.1000000",
            &format!("##[group]Run {WORKSPACE_TESTS}"),
        ),
        status_at(
            "01:00:09.6000000",
            "FAIL",
            "xtask::ci",
            "ci_list_runs_no_control",
        ),
        at(
            "01:00:09.7000000",
            "##[error]Process completed with exit code 100.",
        ),
        at(
            "01:00:09.8000000",
            &format!("##[group]Run {WORKSPACE_M0_24}"),
        ),
        at(
            "01:00:11.0000000",
            "##[group]Run cargo nextest run -p xtask --features controls --test nextest",
        ),
        status_at(
            "01:00:11.4000000",
            "FAIL",
            "xtask::nextest",
            "nextest_ci_shards_together_list_every_test",
        ),
        at(
            "01:00:11.5000000",
            "##[error]Process completed with exit code 100.",
        ),
        at("01:00:30.0000000", &format!("##[group]Run {WORKSPACE_DOC}")),
        at("01:00:31.0000000", "test result: ok. 42 passed; 0 failed"),
    ];
    let run = Run::green().with(job, &lines);
    check_verdict(
        script,
        "two test steps failing with process tests",
        &run,
        "task",
        &[(
            "ci-workspace",
            "other",
            "`cargo nextest run -p xtask`: process tests failed: xtask::ci ci_list_runs_no_control; \
             `cargo nextest run --features controls (the default-feature listing's controls)`: process tests failed: \
             xtask::nextest nextest_ci_shards_together_list_every_test",
        )],
    );
}

#[test]
fn ci_per_merge_verdict_a_part_opening_in_its_steps_second_is_its_own() {
    check_two_process_steps(&script());
}

negative_control!(
    ci_per_merge_verdict_a_part_opening_in_its_steps_second_is_its_own,
    "a classifier that gives a part only to a step started before its first line's second",
    expected = "ci_per_merge: two test steps failing with process tests: the verdict is not task",
    check_two_process_steps(&edit(
        &script(),
        "if second <= m.group(1)]",
        "if second < m.group(1)]"
    ))
);

mod a_skipped_step_owns_no_part {
    use super::*;
    negative_control!(
        ci_per_merge_verdict_a_part_opening_in_its_steps_second_is_its_own,
        "a classifier that lets a skipped step, started in the same second, take the failed step's part",
        expected = "ci_per_merge: two test steps failing with process tests: the verdict is not task",
        check_two_process_steps(&edit(
            &script(),
            "if m and step.get(\"conclusion\") != \"skipped\":",
            "if m:"
        ))
    );
}

/// Steps without their start times: one failed test step reads the whole log; two are red, their parts not told apart.
/// And steps with times over a log without them reads the whole log too.
fn check_untimed(script: &str) {
    let mut job = job(7, "ci-workspace", "failure", WORKSPACE_TESTS);
    job["steps"]
        .as_array_mut()
        .expect("steps")
        .push(json!({"name": WORKSPACE_DOC, "conclusion": "failure"}));
    let lines = [
        line("FAIL", "xtask::ci", "ci_list_runs_no_control"),
        "2026-11-10T01:00:13.0000000Z test result: FAILED. 41 passed; 1 failed".to_owned(),
    ];
    let run = Run::green().with(job, &lines);
    check_verdict(
        script,
        "two failed test steps without their times",
        &run,
        "red",
        &[(
            "ci-workspace",
            "correctness",
            "`cargo nextest run -p xtask`, `cargo test -p xtask --doc` failed, and the log's parts for each cannot be \
             told apart",
        )],
    );
    let timed = timed_job(
        7,
        "ci-workspace",
        &[
            ("Set up job", "success", "01:00:00"),
            (WORKSPACE_TESTS, "failure", "01:00:02"),
        ],
    );
    let bare = Run::green().with(
        timed,
        &["        FAIL [   1.250s] ( 12/5514) xtask::ci ci_list_runs_no_control".to_owned()],
    );
    check_verdict(
        script,
        "a log without times",
        &bare,
        "task",
        &[("ci-workspace", "other", "process tests failed: xtask::ci")],
    );
}

#[test]
fn ci_per_merge_verdict_steps_not_told_apart_are_red() {
    check_untimed(&script());
}

negative_control!(
    ci_per_merge_verdict_steps_not_told_apart_are_red,
    "a classifier that reads the first of two failed test steps in the whole log when it cannot cut it",
    expected = "ci_per_merge: two failed test steps without their times: ci-workspace is not judged correctness",
    check_untimed(&edit(
        &script(),
        "        if len(tests) > 1:\n",
        "        if len(tests) > 2:\n"
    ))
);

const KERNEL_DOC: &str = "cargo test -p validation -p xtask -p engine --doc";

/// A test step skipped after a failed one is red: part of the suite did not run. A test step skipped before it (a
/// shard's doctest step, which runs in shard 1 alone, before the shard's nextest run) is not.
fn check_skipped(script: &str) {
    let mut kernel = job(13, "gpu-kernel suites", "failure", KERNEL_STEP);
    kernel["steps"]
        .as_array_mut()
        .expect("steps")
        .insert(2, json!({"name": KERNEL_DOC, "conclusion": "skipped"}));
    let lines = [line("FAIL", "xtask::ci", "ci_list_runs_no_control")];
    let run = Run::green().with(kernel, &lines);
    check_verdict(
        script,
        "a doctest step skipped after a failed test step",
        &run,
        "red",
        &[(
            "gpu-kernel suites",
            "correctness",
            "`cargo test -p validation -p xtask -p engine --doc` was skipped after \
             `cargo nextest run -p validation -p xtask -p engine` failed, the suite left unrun",
        )],
    );
    let shard = json!({"id": 4, "name": "ci (2)", "status": "completed", "conclusion": "failure", "steps": [
        {"name": "Set up job", "conclusion": "success"},
        {"name": "cargo test --workspace --doc", "conclusion": "skipped"},
        {"name": CI_STEP, "conclusion": "failure"},
    ]});
    let run = Run::green().with(shard, &lines);
    check_verdict(
        script,
        "a doctest step skipped before a failed test step",
        &run,
        "task",
        &[("ci (2)", "other", "process tests failed: xtask::ci")],
    );
    let mut workspace = job(7, "ci-workspace", "failure", WORKSPACE_TESTS);
    for (at, name, conclusion) in [
        (2, WORKSPACE_M0_24, "skipped"),
        (3, WORKSPACE_CONTROLS, "skipped"),
        (4, WORKSPACE_DOC, "success"),
    ] {
        workspace["steps"]
            .as_array_mut()
            .expect("steps")
            .insert(at, json!({"name": name, "conclusion": conclusion}));
    }
    let run = Run::green().with(workspace, &lines);
    check_verdict(
        script,
        "the process tests' steps skipped after a failed test step",
        &run,
        "task",
        &[("ci-workspace", "other", "process tests failed: xtask::ci")],
    );
}

#[test]
fn ci_per_merge_verdict_a_test_step_skipped_after_a_failure_is_red() {
    check_skipped(&script());
}

negative_control!(
    ci_per_merge_verdict_a_test_step_skipped_after_a_failure_is_red,
    "a classifier that reads no skipped test step",
    expected =
        "ci_per_merge: a doctest step skipped after a failed test step: the verdict is not red",
    check_skipped(&edit(&script(), "    if skipped:\n", "    if False:\n"))
);

mod skipped_process_steps {
    use super::*;
    negative_control!(
        ci_per_merge_verdict_a_test_step_skipped_after_a_failure_is_red,
        "a classifier that reads a step of process tests alone, skipped after a failure, as product tests left unrun",
        expected = "ci_per_merge: the process tests' steps skipped after a failed test step: the verdict is not task",
        check_skipped(&edit(
            &script(),
            " and s.get(\"name\") not in PROCESS_STEPS",
            ""
        ))
    );
}

mod skipped_before {
    use super::*;
    negative_control!(
        ci_per_merge_verdict_a_test_step_skipped_after_a_failure_is_red,
        "a classifier that reads a test step skipped before the failed one as left unrun",
        expected = "ci_per_merge: a doctest step skipped before a failed test step: the verdict is not task",
        check_skipped(&edit(
            &script(),
            "for s in every[tests[0] + 1:]",
            "for s in every[tests[0] - 1:]"
        ))
    );
}

/// A process test failing in a `ci` shard, nextest's summary saying how many of the tests it selected it ran: fewer is
/// red, every one (as `N/N` or as `N`) a task.
fn check_unrun(script: &str) {
    let process = line("FAIL", "xtask::ci", "ci_list_runs_no_control");
    let cases = [
        (SUMMARY.to_owned(), "red", "correctness", "nextest ran 322 of the 5514 tests it selected, the rest unrun"),
        (
            "2026-11-10T01:00:01.0000000Z      Summary [  71.045s] 5514/5514 tests run: 5513 passed, 1 failed".to_owned(),
            "task",
            "other",
            "process tests failed: xtask::ci ci_list_runs_no_control",
        ),
        (
            "2026-11-10T01:00:01.0000000Z      Summary [ 300.000s] 5514 tests run: 5513 passed, 1 failed".to_owned(),
            "task",
            "other",
            "process tests failed: xtask::ci ci_list_runs_no_control",
        ),
    ];
    for (summary, want, kind, says) in cases {
        let run = Run::green().test_failed("ci (3)", CI_STEP, &[process.clone(), summary.clone()]);
        check_verdict(
            script,
            &format!("a process test's failure, then `{}`", summary.trim()),
            &run,
            want,
            &[("ci (3)", kind, says)],
        );
    }
}

#[test]
fn ci_per_merge_verdict_tests_left_unrun_are_red() {
    check_unrun(&script());
}

negative_control!(
    ci_per_merge_verdict_tests_left_unrun_are_red,
    "a classifier that does not read nextest's count of the tests it ran",
    expected = "ci_per_merge: a process test's failure, then",
    check_unrun(&edit(&script(), "    if left:\n", "    if False:\n"))
);

mod every_test_run {
    use super::*;
    negative_control!(
        ci_per_merge_verdict_tests_left_unrun_are_red,
        "a classifier that reads a run of every selected test, `N/N`, as one that left tests unrun",
        expected = "5514/5514 tests run: 5513 passed, 1 failed`: the verdict is not task",
        check_unrun(&edit(
            &script(),
            "int(m.group(1)) < int(m.group(2))",
            "int(m.group(1)) <= int(m.group(2))"
        ))
    );
}

// ---- The process class ----------------------------------------------------------------------------------------------

/// `expr` evaluated by python3 with the script loaded as `m` and `sys.argv[2:]` the further arguments: its JSON.
fn evaluated(expr: &str, args: &[&str]) -> Value {
    let program = format!(
        "import importlib.util, json, sys\n\
         spec = importlib.util.spec_from_file_location('m', sys.argv[1])\n\
         m = importlib.util.module_from_spec(spec)\n\
         spec.loader.exec_module(m)\n\
         print(json.dumps({expr}))\n"
    );
    let out = Command::new("python3")
        .arg("-I")
        .arg("-c")
        .arg(program)
        .arg(root().join(SCRIPT))
        .args(args)
        .timed_output()
        .expect("ci_per_merge: python3 ran");
    assert!(
        out.status.success(),
        "ci_per_merge: python3 failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("ci_per_merge: JSON")
}

/// The bare `binary_id(=…)`s of the `process` profile's filter before its `& not (`, read here line by line.
fn process_profile(config: &str) -> BTreeSet<String> {
    let at = config
        .find("[profile.process]\n")
        .unwrap_or_else(|| panic!("ci_per_merge: no [profile.process]"));
    let rest = &config[at..];
    let head = &rest[..rest
        .find("& not (")
        .expect("ci_per_merge: the process profile's slow tier")];
    head.lines()
        .filter_map(|l| {
            l.trim()
                .trim_start_matches("| ")
                .strip_prefix("binary_id(=")
        })
        .filter_map(|b| b.strip_suffix(')'))
        .map(str::to_owned)
        .collect()
}

/// The classifier's process class, read from the configuration at `config`.
fn class_of(config: &Path) -> BTreeSet<String> {
    let got = evaluated(
        "sorted(m.process_binaries(open(sys.argv[2]).read()))",
        &[config.to_str().expect("a path")],
    );
    got.as_array()
        .expect("ci_per_merge: a list")
        .iter()
        .map(|v| v.as_str().expect("a name").to_owned())
        .collect()
}

/// The owner's product correctness, run per merge and never retired (R-909), and product binaries beside them.
const PRODUCT: [&str; 6] = [
    "xtask::qa_TASK-M0-06_runner",
    "xtask::qa_TASK-M0-06_list",
    "xtask::qa_TASK-M0-20",
    "xtask::screenshot",
    "engine::canon_gpu",
    "validation::agreement_preset_gpu",
];

fn check_class(config: &str) {
    let dir = Scratch::new("ci_per_merge_class");
    std::fs::create_dir_all(&*dir).expect("scratch");
    let file = dir.join("nextest.toml");
    std::fs::write(&file, config).expect("config");
    let class = class_of(&file);
    assert!(
        !class.is_empty() && class == process_profile(config),
        "ci_per_merge: the classifier's process class is not the process profile's binaries"
    );
    for binary in [
        "xtask::ci",
        "xtask::ci_per_merge",
        "validation::qa_TASK-M0-24",
        "xtask::nextest",
    ] {
        assert!(
            class.contains(binary),
            "ci_per_merge: {binary} is not a process test"
        );
    }
    for binary in PRODUCT {
        assert!(
            !class.contains(binary),
            "ci_per_merge: {binary} reads as a process test"
        );
    }
}

#[test]
fn ci_per_merge_process_class_is_the_process_profiles() {
    check_class(&read(CONFIG));
}

negative_control!(
    ci_per_merge_process_class_is_the_process_profiles,
    "a process profile that names the golden runner",
    expected = "ci_per_merge: xtask::qa_TASK-M0-06_runner reads as a process test",
    check_class(&edit(
        &read(CONFIG),
        "    (\n        binary_id(=validation::qa_TASK-M0-04_controls)\n",
        "    (\n        binary_id(=xtask::qa_TASK-M0-06_runner)\n        | binary_id(=validation::qa_TASK-M0-04_controls)\n"
    ))
);

// ---- `decide` -------------------------------------------------------------------------------------------------------

/// What `decide` did: its exit, its output, each `gh` call's arguments joined by a space, and its step summary.
struct Decided {
    ok: bool,
    out: String,
    calls: Vec<String>,
    summary: String,
}

impl Decided {
    /// The calls that POST to `path` exactly.
    fn posts_to(&self, path: &str) -> usize {
        self.calls
            .iter()
            .filter(|c| c.starts_with(&format!("api --method POST {path} ")))
            .count()
    }
}

/// `decide` under `script`, on `run`, with a stand-in `gh` serving the run's jobs in two pages, each failed job's log
/// (a 404 for one with none), the open issues `issues` (JSON), and failing what `env` asks of it.
fn decide(script: &str, run: &Run, issues: &str, env: &[(&str, &str)]) -> Decided {
    let dir = Scratch::new("ci_per_merge_decide");
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
    std::fs::write(data.join("issues"), issues).expect("issues");
    std::fs::create_dir_all(dir.join(".config")).expect(".config");
    std::fs::write(dir.join(".config/nextest.toml"), read(CONFIG)).expect("config");
    let file = dir.join("ci_per_merge.py");
    std::fs::write(&file, script).expect("script written");
    let calls = dir.join("calls");
    std::fs::write(&calls, "").expect("calls");
    let summary = dir.join("summary");
    write_executable(
        &bin.join("gh"),
        "#!/bin/sh\n\
         { printf '\\036'; printf '%s ' \"$@\"; } >> \"$STUB_CALLS\"\n\
         args=\"$*\"\n\
         case \"$args\" in\n\
           'api --method POST repos/o/r/labels '*) [ -z \"$STUB_LABEL_ERR\" ] || { echo \"$STUB_LABEL_ERR\" >&2; exit 1; } ;;\n\
           'api --method POST '*) [ -z \"$STUB_POST_ERR\" ] || { echo \"$STUB_POST_ERR\" >&2; exit 1; }; echo '{}' ;;\n\
           *'/attempts/'*'/jobs'*)\n\
             [ -z \"$STUB_JOBS_ERR\" ] || { echo \"$STUB_JOBS_ERR\" >&2; exit 1; }\n\
             cat \"$STUB_DATA/pages\" ;;\n\
           *'/logs')\n\
             id=${args##*jobs/}; id=${id%/logs}\n\
             cat \"$STUB_DATA/logs/$id.log\" 2> /dev/null || { echo 'gh: HTTP 404' >&2; exit 1; } ;;\n\
           *'/issues?labels='*) cat \"$STUB_DATA/issues\" ;;\n\
           *) echo \"unexpected: $args\" >&2; exit 1 ;;\n\
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
        .env("RUN_ATTEMPT", "1")
        .env("HEAD_SHA", "abc")
        .env("FILE_TASKS", "true")
        .env("GITHUB_STEP_SUMMARY", &summary);
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.timed_output().expect("ci_per_merge: decide ran");
    Decided {
        ok: out.status.success(),
        out: String::from_utf8_lossy(&out.stdout).into_owned(),
        calls: std::fs::read_to_string(&calls)
            .unwrap_or_default()
            .split('\u{1e}')
            .filter(|c| !c.is_empty())
            .map(str::to_owned)
            .collect(),
        summary: std::fs::read_to_string(&summary).unwrap_or_default(),
    }
}

/// A process test failing in a `ci` shard, a task.
fn process_run() -> Run {
    Run::green().test_failed(
        "ci (3)",
        CI_STEP,
        &[line("FAIL", "xtask::ci", "ci_list_runs_no_control")],
    )
}

const TITLE: &str = "per-merge: ci failed on main (not a correctness failure)";

fn check_files(env: &[(&str, &str)]) {
    let done = decide(&script(), &process_run(), "[]", env);
    assert!(done.ok, "ci_per_merge: decide failed on a task");
    assert_eq!(
        done.posts_to("repos/o/r/issues"),
        1,
        "ci_per_merge: decide did not open the task"
    );
    let opened = done
        .calls
        .iter()
        .find(|c| c.starts_with("api --method POST repos/o/r/issues "))
        .expect("the issue");
    assert!(
        opened.contains(&format!("-f title={TITLE} "))
            && opened.contains("-f labels[]=per-merge ")
            && opened.contains("https://github.com/o/r/actions/runs/42")
            && opened.contains("xtask::ci ci_list_runs_no_control"),
        "ci_per_merge: the task does not name its run, its failure and its label"
    );
    assert!(
        done.posts_to("repos/o/r/labels") == 1 && done.summary.contains(&format!("Filed: {TITLE}")),
        "ci_per_merge: decide did not make its label or say it filed the task"
    );
}

#[test]
fn ci_per_merge_decide_files_one_task_on_main() {
    check_files(&[]);
}

negative_control!(
    ci_per_merge_decide_files_one_task_on_main,
    "decide off `main`",
    expected = "ci_per_merge: decide did not open the task",
    check_files(&[("FILE_TASKS", "false")])
);

#[test]
fn ci_per_merge_decide_files_the_task_when_its_label_exists() {
    check_files(&[("STUB_LABEL_ERR", "gh: Validation Failed (HTTP 422)")]);
}

negative_control!(
    ci_per_merge_decide_files_the_task_when_its_label_exists,
    "decide off `main`, the label existing",
    expected = "ci_per_merge: decide did not open the task",
    check_files(&[
        ("STUB_LABEL_ERR", "gh: Validation Failed (HTTP 422)"),
        ("FILE_TASKS", "false")
    ])
);

fn check_comment(open_title: &str) {
    let issues = json!([{"number": 7, "title": open_title}]).to_string();
    let done = decide(&script(), &process_run(), &issues, &[]);
    assert!(done.ok, "ci_per_merge: decide failed on a task");
    assert_eq!(
        done.posts_to("repos/o/r/issues"),
        0,
        "ci_per_merge: decide opened a second issue"
    );
    assert_eq!(
        done.posts_to("repos/o/r/issues/7/comments"),
        1,
        "ci_per_merge: decide did not comment on the open task"
    );
}

#[test]
fn ci_per_merge_decide_comments_on_the_open_task() {
    check_comment(TITLE);
}

negative_control!(
    ci_per_merge_decide_comments_on_the_open_task,
    "an open issue of another title",
    expected = "ci_per_merge: decide opened a second issue",
    check_comment("per-merge: xtask-ci failed on main (not a correctness failure)")
);

fn check_red(run: &Run) {
    let done = decide(&script(), run, "[]", &[]);
    assert!(!done.ok, "ci_per_merge: decide passed a red run");
    assert!(
        done.out.starts_with("verdict=red\n") && done.summary.contains("**Red `main`:**"),
        "ci_per_merge: decide does not say the run is red"
    );
    assert_eq!(
        done.posts_to("repos/o/r/issues"),
        0,
        "ci_per_merge: decide filed a correctness failure as a task"
    );
}

#[test]
fn ci_per_merge_decide_red_fails_and_files_nothing() {
    check_red(&Run::green().test_failed("ci (2)", CI_STEP, &[PRODUCT_FAIL.to_owned()]));
}

negative_control!(
    ci_per_merge_decide_red_fails_and_files_nothing,
    "a process test's failure",
    expected = "ci_per_merge: decide passed a red run",
    check_red(&process_run())
);

/// The verdict's own job, still running as it reads the run.
fn with_itself(run: Run) -> Run {
    let mut run = run;
    run.jobs.push(json!({"id": 99, "name": "per-merge", "status": "in_progress", "conclusion": null, "steps": []}));
    run
}

fn check_itself(env: &[(&str, &str)]) {
    let done = decide(&script(), &with_itself(Run::green()), "[]", env);
    assert!(
        done.ok && done.out == "verdict=green\n",
        "ci_per_merge: decide read its own running job"
    );
    assert!(
        !done.calls.iter().any(|c| c.contains("jobs/99/logs")),
        "ci_per_merge: decide fetched its own log"
    );
}

#[test]
fn ci_per_merge_decide_never_reads_its_own_job() {
    check_itself(&[]);
}

negative_control!(
    ci_per_merge_decide_never_reads_its_own_job,
    "decide told its job has another name",
    expected = "ci_per_merge: decide read its own running job",
    check_itself(&[("SELF", "verdict")])
);

fn check_unread(env: &[(&str, &str)]) {
    let done = decide(&script(), &Run::green(), "[]", env);
    assert!(
        !done.ok && done.out.starts_with("verdict=red\n"),
        "ci_per_merge: a run whose jobs could not be read is not red"
    );
}

#[test]
fn ci_per_merge_decide_unread_jobs_are_red() {
    check_unread(&[("STUB_JOBS_ERR", "gh: HTTP 502")]);
}

negative_control!(
    ci_per_merge_decide_unread_jobs_are_red,
    "jobs that could be read",
    expected = "ci_per_merge: a run whose jobs could not be read is not red",
    check_unread(&[])
);

fn check_unfiled(script: &str) {
    let done = decide(
        script,
        &process_run(),
        "[]",
        &[("STUB_POST_ERR", "gh: HTTP 403")],
    );
    assert!(
        done.ok,
        "ci_per_merge: a task not filed made the verdict red"
    );
    assert!(
        done.out
            .contains("::error::the per-merge task was not filed: the issue could not be opened")
            && done.summary.contains("The task **was not filed**"),
        "ci_per_merge: a task not filed is not seen"
    );
}

#[test]
fn ci_per_merge_decide_a_task_not_filed_is_seen_not_red() {
    check_unfiled(&script());
}

negative_control!(
    ci_per_merge_decide_a_task_not_filed_is_seen_not_red,
    "a decide that says nothing when the task is not filed",
    expected = "ci_per_merge: a task not filed is not seen",
    check_unfiled(&edit(
        &script(),
        "print(f\"::error::the per-merge task was not filed: {why}\")",
        "pass"
    ))
);

// ---- `ci.yml`'s per-merge tier --------------------------------------------------------------------------------------

/// `text`, a workflow, parsed by PyYAML, as JSON, YAML's `on:` read back as the key `on`.
fn parse(text: &str) -> Value {
    let dir = Scratch::new("ci_per_merge_yaml");
    std::fs::create_dir_all(&*dir).expect("scratch");
    let file = dir.join("ci.yml");
    std::fs::write(&file, text).expect("workflow");
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
        .expect("python ran");
    assert!(
        out.status.success(),
        "ci_per_merge: ci.yml: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("json")
}

fn text(v: &Value) -> &str {
    v.as_str().unwrap_or_default()
}

fn check_each_merge(ci: &str) {
    let ci = parse(ci);
    let branches = &ci["on"]["push"]["branches"];
    assert!(
        branches
            .as_array()
            .is_some_and(|b| b.iter().any(|x| x == "main")),
        "ci_per_merge: ci.yml does not run on a push to main"
    );
    assert_eq!(
        text(&ci["concurrency"]["group"]),
        "ci-${{ github.event_name }}-${{ github.ref == 'refs/heads/main' && github.run_id || github.ref }}",
        "ci_per_merge: ci.yml does not give each merge commit a run of its own"
    );
    assert_eq!(
        text(&ci["concurrency"]["cancel-in-progress"]),
        "${{ github.ref != 'refs/heads/main' }}",
        "ci_per_merge: ci.yml cancels a run on main"
    );
}

#[test]
fn ci_per_merge_tier_runs_on_each_merge_commit() {
    check_each_merge(&read(CI));
}

negative_control!(
    ci_per_merge_tier_runs_on_each_merge_commit,
    "a ci.yml whose runs on main share one group, a later merge queuing behind an earlier",
    expected = "ci_per_merge: ci.yml does not give each merge commit a run of its own",
    check_each_merge(&edit(
        &read(CI),
        "github.ref == 'refs/heads/main' && github.run_id || github.ref }}",
        "github.ref }}"
    ))
);

mod cancelled_on_main {
    use super::*;
    negative_control!(
        ci_per_merge_tier_runs_on_each_merge_commit,
        "a ci.yml that cancels a superseded run on main",
        expected = "ci_per_merge: ci.yml cancels a run on main",
        check_each_merge(&edit(
            &read(CI),
            "  cancel-in-progress: ${{ github.ref != 'refs/heads/main' }}\n",
            "  cancel-in-progress: true\n"
        ))
    );
}

/// The jobs of `ci` a push runs: every job whose condition does not ask for a pull request, but `per-merge`.
fn push_jobs(ci: &Value) -> BTreeMap<String, Value> {
    ci["jobs"]
        .as_object()
        .expect("ci_per_merge: jobs")
        .iter()
        .filter(|(id, job)| {
            id.as_str() != "per-merge"
                && !text(&job["if"]).contains("github.event_name == 'pull_request'")
        })
        .map(|(id, job)| (id.clone(), job.clone()))
        .collect()
}

fn check_verdict_job(ci: &str) {
    let ci = parse(ci);
    let job = &ci["jobs"]["per-merge"];
    assert!(job.is_object(), "ci_per_merge: ci.yml has no per-merge job");
    let needs: BTreeSet<&str> = job["needs"]
        .as_array()
        .map(|n| n.iter().map(text).collect())
        .unwrap_or_default();
    for id in push_jobs(&ci).keys() {
        assert!(
            needs.contains(id.as_str()),
            "ci_per_merge: per-merge does not read {id}"
        );
    }
    assert_eq!(
        needs.len(),
        push_jobs(&ci).len(),
        "ci_per_merge: per-merge waits on a job a push does not run"
    );
    assert_eq!(
        text(&job["if"]),
        "always() && github.event_name != 'pull_request'",
        "ci_per_merge: per-merge does not run on every push whatever the jobs ended in"
    );
    assert_eq!(
        text(&job["runs-on"]),
        "ubuntu-24.04",
        "ci_per_merge: per-merge does not run on a GitHub-hosted runner"
    );
    assert_eq!(
        job["permissions"],
        json!({"actions": "read", "contents": "read", "issues": "write"}),
        "ci_per_merge: per-merge's permissions are not what it uses"
    );
    assert!(
        job.get("continue-on-error").is_none(),
        "ci_per_merge: per-merge passes whatever it finds"
    );
    let steps = job["steps"].as_array().expect("ci_per_merge: steps");
    let step = steps
        .iter()
        .find(|s| text(&s["run"]) == "python3 -I .github/scripts/ci_per_merge.py decide")
        .unwrap_or_else(|| panic!("ci_per_merge: per-merge does not run the verdict"));
    assert!(
        step.get("continue-on-error").is_none() && step.get("if").is_none(),
        "ci_per_merge: the verdict's step can be passed over"
    );
    for (key, want) in [
        ("FILE_TASKS", "${{ github.ref == 'refs/heads/main' }}"),
        ("RUN_ID", "${{ github.run_id }}"),
        ("RUN_ATTEMPT", "${{ github.run_attempt }}"),
        ("GH_TOKEN", "${{ github.token }}"),
        ("GH_REPO", "${{ github.repository }}"),
        ("HEAD_SHA", "${{ github.sha }}"),
    ] {
        assert_eq!(
            text(&step["env"][key]),
            want,
            "ci_per_merge: the verdict's {key} is not {want}"
        );
    }
}

#[test]
fn ci_per_merge_tier_verdict_runs_last_on_a_push() {
    check_verdict_job(&read(CI));
}

negative_control!(
    ci_per_merge_tier_verdict_runs_last_on_a_push,
    "a per-merge job that does not wait for xtask-ci",
    expected = "ci_per_merge: per-merge does not read xtask-ci",
    check_verdict_job(&edit(
        &read(CI),
        "      - xtask-ci\n      - gpu-metal\n",
        "      - gpu-metal\n"
    ))
);

mod skipped_after_a_failure {
    use super::*;
    negative_control!(
        ci_per_merge_tier_verdict_runs_last_on_a_push,
        "a per-merge job skipped when a job it needs failed",
        expected = "ci_per_merge: per-merge does not run on every push whatever the jobs ended in",
        check_verdict_job(&edit(
            &read(CI),
            "    if: always() && github.event_name != 'pull_request'\n    runs-on: ubuntu-24.04\n    timeout-minutes: 15\n",
            "    if: github.event_name != 'pull_request'\n    runs-on: ubuntu-24.04\n    timeout-minutes: 15\n"
        ))
    );
}

mod filing_off_main {
    use super::*;
    negative_control!(
        ci_per_merge_tier_verdict_runs_last_on_a_push,
        "a per-merge job that files tasks from a measure/ branch",
        expected = "ci_per_merge: the verdict's FILE_TASKS is not",
        check_verdict_job(&edit(
            &read(CI),
            "          FILE_TASKS: ${{ github.ref == 'refs/heads/main' }}\n",
            "          FILE_TASKS: true\n"
        ))
    );
}

/// The name each job of `jobs` reports under, less a matrix's ` (<k>)`, as the classifier reads it.
fn base_names(jobs: &BTreeMap<String, Value>) -> BTreeSet<String> {
    jobs.iter()
        .map(|(id, job)| {
            let name = job["name"].as_str().unwrap_or(id);
            name.split(" (${{").next().unwrap_or(name).to_owned()
        })
        .collect()
}

/// The classifier's three lists, by its own names for them.
fn lists(script: &str) -> BTreeMap<String, BTreeSet<String>> {
    let dir = Scratch::new("ci_per_merge_lists");
    std::fs::create_dir_all(&*dir).expect("scratch");
    let file = dir.join("ci_per_merge.py");
    std::fs::write(&file, script).expect("script");
    let out = Command::new("python3")
        .arg("-I")
        .arg("-c")
        .arg(
            "import importlib.util, json, sys\n\
             spec = importlib.util.spec_from_file_location('m', sys.argv[1])\n\
             m = importlib.util.module_from_spec(spec)\n\
             spec.loader.exec_module(m)\n\
             print(json.dumps({k: list(getattr(m, k)) for k in ('OTHER_JOBS', 'RELEASE_JOBS', 'TEST_JOBS')}))\n",
        )
        .arg(&file)
        .timed_output()
        .expect("python3 ran");
    assert!(
        out.status.success(),
        "ci_per_merge: the lists could not be read"
    );
    serde_json::from_slice::<BTreeMap<String, BTreeSet<String>>>(&out.stdout).expect("json")
}

fn check_known(ci: &str, script: &str) {
    let lists = lists(script);
    let mut known = BTreeSet::new();
    for names in lists.values() {
        for name in names {
            assert!(
                known.insert(name.clone()),
                "ci_per_merge: {name} is in two of the classifier's lists"
            );
        }
    }
    assert_eq!(
        base_names(&push_jobs(&parse(ci))),
        known,
        "ci_per_merge: the classifier does not know ci.yml's push jobs"
    );
}

#[test]
fn ci_per_merge_tier_verdict_knows_every_job() {
    check_known(&read(CI), &script());
}

negative_control!(
    ci_per_merge_tier_verdict_knows_every_job,
    "a ci.yml whose web-skeleton reports under another name",
    expected = "ci_per_merge: the classifier does not know ci.yml's push jobs",
    check_known(
        &edit(
            &read(CI),
            "  web-skeleton:\n    needs: changes\n",
            "  web-skeleton:\n    name: wasm\n    needs: changes\n"
        ),
        &script()
    )
);

mod in_two_lists {
    use super::*;
    negative_control!(
        ci_per_merge_tier_verdict_knows_every_job,
        "a classifier that lists ci-release among its test jobs too",
        expected = "ci_per_merge: ci-release is in two of the classifier's lists",
        check_known(
            &read(CI),
            &edit(
                &script(),
                "    \"ci-archive\",\n",
                "    \"ci-archive\",\n    \"ci-release\",\n"
            )
        )
    );
}

/// The terms of `profile`'s filter in `config`, one per line, without the `not (` around them.
fn terms(config: &str, profile: &str) -> BTreeSet<String> {
    let header = format!("[profile.{profile}]\n");
    let at = config
        .find(&header)
        .unwrap_or_else(|| panic!("ci_per_merge: no {header}"));
    let rest = &config[at..];
    let start =
        rest.find("default-filter = \"\"\"").expect("a filter") + "default-filter = \"\"\"".len();
    let body = &rest[start..];
    body[..body.find("\"\"\"").expect("its end")]
        .lines()
        .map(|l| l.trim().trim_start_matches("| ").to_owned())
        .filter(|l| !l.is_empty() && l != "not (" && l != ")")
        .collect()
}

/// Every step of the push jobs on a runner whose label starts `os`: each run command and its NEXTEST_PROFILE.
fn steps_on(ci: &Value, os: &str) -> Vec<(String, String)> {
    push_jobs(ci)
        .values()
        .filter(|job| text(&job["runs-on"]).starts_with(os))
        .flat_map(|job| job["steps"].as_array().cloned().unwrap_or_default())
        .map(|s| {
            (
                text(&s["run"]).to_owned(),
                text(&s["env"]["NEXTEST_PROFILE"]).to_owned(),
            )
        })
        .collect()
}

fn check_owners_list(ci: &str, config: &str) {
    let ci = parse(ci);
    for (os, backend) in [("ubuntu-", "lavapipe"), ("macos-", "Metal")] {
        let steps = steps_on(&ci, os);
        for command in [
            "cargo xtask golden --all",
            "cargo xtask gate decode-agreement",
        ] {
            assert!(
                steps.iter().any(|(run, _)| run == command),
                "ci_per_merge: no {backend} job of the per-merge tier runs {command}"
            );
        }
    }
    let linux = steps_on(&ci, "ubuntu-");
    for profile in ["ci", "ci-workspace", "gpu-kernel"] {
        assert!(
            linux
                .iter()
                .any(|(run, p)| p == profile && run.starts_with("cargo nextest run")),
            "ci_per_merge: the per-merge tier does not run the {profile} profile"
        );
    }
    assert!(
        linux
            .iter()
            .any(|(run, _)| run.starts_with("cargo nextest archive --workspace --release")),
        "ci_per_merge: the per-merge tier does not build the release profile's tests"
    );
    let mut others = terms(config, "gpu-kernel");
    others.extend(terms(config, "ci-workspace"));
    assert_eq!(
        terms(config, "ci"),
        others,
        "ci_per_merge: the ci, ci-workspace and gpu-kernel profiles do not split every test between them"
    );
}

#[test]
fn ci_per_merge_tier_runs_the_owners_list() {
    check_owners_list(&read(CI), &read(CONFIG));
}

negative_control!(
    ci_per_merge_tier_runs_the_owners_list,
    "a ci.yml whose Metal suites no longer run every golden suite",
    expected = "ci_per_merge: no Metal job of the per-merge tier runs cargo xtask golden --all",
    check_owners_list(
        &edit(
            &read(CI),
            "        run: cargo xtask golden --all\n",
            "        run: cargo xtask golden m1-outcome\n"
        ),
        &read(CONFIG)
    )
);

mod a_test_in_no_profile {
    use super::*;
    negative_control!(
        ci_per_merge_tier_runs_the_owners_list,
        "a gpu-kernel profile that no longer selects the lock's GPU test, which the ci profile leaves out",
        expected = "ci_per_merge: the ci, ci-workspace and gpu-kernel profiles do not split every test between them",
        check_owners_list(
            &read(CI),
            &edit(
                &read(CONFIG),
                // The gpu-kernel filter's one line for the lock's binary: its terms sit four spaces in, the ci and
                // pr profiles' eight, so this is the first match and the only edit.
                "\n    | binary_id(=engine::lock_gpu)\n",
                "\n"
            )
        )
    );
}

/// The steps of a job that run tests, by the start of their names, as the classifier reads them.
const TEST_STEPS: [&str; 4] = [
    "cargo nextest run",
    "cargo test",
    "cargo xtask golden",
    "cargo xtask gate",
];

/// The classifier's PROCESS_STEPS, from `script`.
fn process_steps(script: &str) -> BTreeSet<String> {
    let dir = Scratch::new("ci_per_merge_process_steps");
    std::fs::create_dir_all(&*dir).expect("scratch");
    let file = dir.join("ci_per_merge.py");
    std::fs::write(&file, script).expect("script");
    let got = evaluated_from(&file, "list(m.PROCESS_STEPS)");
    got.as_array()
        .expect("ci_per_merge: a list")
        .iter()
        .map(|v| v.as_str().expect("a name").to_owned())
        .collect()
}

/// `expr` evaluated by python3 with the script at `file` loaded as `m`: its JSON.
fn evaluated_from(file: &Path, expr: &str) -> Value {
    let program = format!(
        "import importlib.util, json, sys\n\
         spec = importlib.util.spec_from_file_location('m', sys.argv[1])\n\
         m = importlib.util.module_from_spec(spec)\n\
         spec.loader.exec_module(m)\n\
         print(json.dumps({expr}))\n"
    );
    let out = Command::new("python3")
        .arg("-I")
        .arg("-c")
        .arg(program)
        .arg(file)
        .timed_output()
        .expect("ci_per_merge: python3 ran");
    assert!(
        out.status.success(),
        "ci_per_merge: python3 failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("ci_per_merge: JSON")
}

/// The binaries a `cargo nextest run` command selects by `-p` and `--test`, each `<package>::<test>`; empty unless it
/// names one package and at least one `--test`, and nothing else selects more (`--workspace`, `--lib`, `--all-targets`).
fn selected_binaries(run: &str) -> BTreeSet<String> {
    let words: Vec<&str> = run.split_whitespace().collect();
    let packages: Vec<&str> = words
        .windows(2)
        .filter(|w| w[0] == "-p")
        .map(|w| w[1])
        .collect();
    let tests: Vec<&str> = words
        .windows(2)
        .filter(|w| w[0] == "--test")
        .map(|w| w[1])
        .collect();
    let wider = ["--workspace", "--lib", "--bins", "--all-targets", "--doc"];
    if packages.len() != 1 || tests.is_empty() || words.iter().any(|w| wider.contains(w)) {
        return BTreeSet::new();
    }
    tests
        .iter()
        .map(|t| format!("{}::{t}", packages[0]))
        .collect()
}

/// The jobs that run the `ci`, `ci-workspace` and `gpu-kernel` profiles, R-909's product suite, run every product test
/// after a failure: each profile with `fail-fast = false`; each test step after a job's first runs whatever an earlier
/// one ended in (`!cancelled()`), or runs process tests alone (the classifier's PROCESS_STEPS, each selecting process
/// binaries only), or follows doctest steps alone (whose failure is red); each `steps.<id>` a condition reads is an
/// earlier step's. The other test jobs' later steps do not run after a failure; the verdict reads a product test step
/// skipped there as red.
fn check_run_whole(ci: &str, config: &str, script: &str) {
    let ci = parse(ci);
    let dir = Scratch::new("ci_per_merge_run_whole");
    std::fs::create_dir_all(&*dir).expect("scratch");
    let file = dir.join("nextest.toml");
    std::fs::write(&file, config).expect("config");
    let process = class_of(&file);
    let alone = process_steps(script);
    let config: toml_edit::DocumentMut = config.parse().expect("ci_per_merge: nextest.toml");
    let mut seen = BTreeSet::new();
    for key in ["ci", "ci-workspace", "gpu-kernel"] {
        let steps = ci["jobs"][key]["steps"]
            .as_array()
            .unwrap_or_else(|| panic!("ci_per_merge: no job {key}"));
        let mut ids = BTreeSet::new();
        let mut earlier: Vec<String> = Vec::new();
        for step in steps {
            let name = text(&step["name"]);
            let cond = text(&step["if"]);
            let run = text(&step["run"]);
            let mut rest = cond;
            while let Some(at) = rest.find("steps.") {
                rest = &rest[at + "steps.".len()..];
                let id = rest.split('.').next().unwrap_or_default();
                assert!(
                    ids.contains(id),
                    "ci_per_merge: `{name}` of {key} reads `steps.{id}`, no earlier step's id"
                );
            }
            let profile = text(&step["env"]["NEXTEST_PROFILE"]);
            if !profile.is_empty() {
                assert_eq!(
                    config
                        .get("profile")
                        .and_then(|p| p.get(profile))
                        .and_then(|p| p.get("fail-fast"))
                        .and_then(|v| v.as_bool()),
                    Some(false),
                    "ci_per_merge: the {profile} profile stops at a failure"
                );
            }
            if alone.contains(name) {
                let binaries = selected_binaries(run);
                assert!(
                    !binaries.is_empty() && binaries.iter().all(|b| process.contains(b)),
                    "ci_per_merge: `{name}` of {key} is a step of process tests alone to the verdict, but runs `{run}`"
                );
                seen.insert(name.to_owned());
            }
            if TEST_STEPS.iter().any(|t| name.starts_with(t)) {
                let after_doctests = earlier.iter().all(|r| r.contains("--doc"));
                assert!(
                    earlier.is_empty()
                        || cond.contains("!cancelled()")
                        || alone.contains(name)
                        || after_doctests,
                    "ci_per_merge: `{name}` of {key} does not run after a failed test step"
                );
                earlier.push(run.to_owned());
            }
            if let Some(id) = step["id"].as_str() {
                ids.insert(id.to_owned());
            }
        }
        assert!(!earlier.is_empty(), "ci_per_merge: {key} runs no test step");
    }
    assert_eq!(
        seen, alone,
        "ci_per_merge: a step the verdict reads as process tests alone is not in ci.yml"
    );
}

#[test]
fn ci_per_merge_tier_runs_every_test_after_a_failure() {
    check_run_whole(&read(CI), &read(CONFIG), &script());
}

negative_control!(
    ci_per_merge_tier_runs_every_test_after_a_failure,
    "a gpu-kernel profile that stops at the first failure",
    expected = "ci_per_merge: the gpu-kernel profile stops at a failure",
    check_run_whole(
        &read(CI),
        &edit(
            &read(CONFIG),
            "[profile.gpu-kernel]\nfail-fast = false\n",
            "[profile.gpu-kernel]\n"
        ),
        &script()
    )
);

mod a_doctest_skipped_after_a_failure {
    use super::*;
    negative_control!(
        ci_per_merge_tier_runs_every_test_after_a_failure,
        "a ci-workspace whose doctest step is skipped after a failed test step",
        expected = "ci_per_merge: `cargo test -p xtask --doc` of ci-workspace does not run after a failed test step",
        check_run_whole(
            &edit(
                &read(CI),
                "      - name: cargo test -p xtask --doc\n        if: ${{ !cancelled() && steps.workspace-tests.outcome != 'skipped' }}\n",
                "      - name: cargo test -p xtask --doc\n"
            ),
            &read(CONFIG),
            &script()
        )
    );
}

mod a_step_id_gone {
    use super::*;
    negative_control!(
        ci_per_merge_tier_runs_every_test_after_a_failure,
        "a gpu-kernel whose doctest step reads the nextest step under an id it does not have",
        expected = "ci_per_merge: `cargo test -p validation -p xtask -p engine --doc` of gpu-kernel reads `steps.kernel-test`, no earlier step's id",
        check_run_whole(
            &edit(
                &read(CI),
                "steps.kernel-tests.outcome",
                "steps.kernel-test.outcome"
            ),
            &read(CONFIG),
            &script()
        )
    );
}

mod a_product_step_read_as_process {
    use super::*;
    negative_control!(
        ci_per_merge_tier_runs_every_test_after_a_failure,
        "a classifier that reads ci-workspace's doctest step as process tests alone",
        expected = "ci_per_merge: `cargo test -p xtask --doc` of ci-workspace is a step of process tests alone to the verdict",
        check_run_whole(
            &read(CI),
            &read(CONFIG),
            &edit(
                &script(),
                "PROCESS_STEPS = (\n",
                "PROCESS_STEPS = (\n    \"cargo test -p xtask --doc\",\n"
            )
        )
    );
}
