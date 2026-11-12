//! REQ-TOOL-228: the per-merge verdict (`.github/scripts/ci_per_merge.py`) covers every job of `ci.yml`'s per-merge
//! run. R-982 (12 Nov 2026, the owner's veto of D-2 of that day's report): "Any red job on main is red; make the
//! per-merge verdict cover every job so the two can't disagree." R-979 (RQ-789 (a)): an install step's network or
//! archive-download failure reads "no result", never green, in any job; beside any other failure, red.
//!
//! - `ci_per_merge_verdict_a_failed_job_of_any_kind_is_red`: a failed process test, the lints, `cargo xtask ci`, the
//!   web skeleton and the classification each read red.
//! - `ci_per_merge_verdict_an_install_outage_is_no_result_in_any_job`: the Mesa install step's network or archive
//!   download failing, in a release shard, a `ci` shard, `xtask-ci`, the GPU suites and `ci-workspace`, reads
//!   "no result", the step and its log's evidence named; `decide` fails on it under its own summary line.
//! - `ci_per_merge_verdict_an_outage_beside_another_failure_is_red`: the outage beside a product test's failure, beside
//!   `xtask-ci`'s, and beside another failed step in its own job, each red.
//! - `ci_per_merge_verdict_an_install_outage_is_narrow`: the install step failing on a package error, with no network
//!   error, without `bounded`'s last try failing, its evidence in another step's part, another install step, a job
//!   cancelled, and a job without its log, each red.
//! - `ci_per_merge_verdict_reads_the_saved_runs`: the three per-merge runs of 11 Nov 2026 (RQ-789's evidence), `main`
//!   at 12 Nov 2026 04:00 (D-2's run) and the next run on `main`, saved under `fixtures/ci_per_merge/`: red, red, no
//!   result, red and green.
//!
//! Each test has its registered negative control (R-176), which feeds the same check a script differing in the one
//! respect it turns on. Unix only: `decide`'s stand-in `gh` is an executable script.
#![cfg(unix)]

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
const INSTALL: &str = "Install Mesa (lavapipe)";
const CI_STEP: &str = "cargo nextest run --archive-file … --partition hash:2/4";
/// The network evidence the outage of 11 Nov 2026 shows (run 43029168415).
const IGN: &str = "Ign:2 http://azure.archive.ubuntu.com/ubuntu noble InRelease";
/// `bounded`'s last try of `apt-get update` failing (run 43029168415).
const LAST_TRY: &str = "attempt 3 of 3 failed or ran past 90 s, apt-get update";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the workspace root")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel))
        .unwrap_or_else(|e| panic!("ci_per_merge_verdict: {rel}: {e}"))
}

fn script() -> String {
    read(SCRIPT)
}

/// `text` with its first `from` replaced by `to`, which must be in it: a control's one edit.
#[cfg(feature = "controls")]
fn edit(text: &str, from: &str, to: &str) -> String {
    assert!(
        text.contains(from),
        "ci_per_merge_verdict: a control's edit target is gone: {from}"
    );
    text.replacen(from, to, 1)
}

/// A log line written at `time` (`HH:MM:SS.f` on 11 Nov 2026).
fn at(time: &str, text: &str) -> String {
    format!("2026-11-11T{time}Z {text}")
}

/// A run as the classifier reads it: its jobs and the failed ones' logs, by job id.
#[derive(Clone, Default)]
struct Run {
    jobs: Vec<Value>,
    logs: Vec<(u64, String)>,
}

impl Run {
    /// A run whose jobs `names` all passed.
    fn passing(names: &[&str]) -> Run {
        let jobs = names
            .iter()
            .enumerate()
            .map(|(i, n)| json!({"id": i as u64 + 1, "name": n, "status": "completed", "conclusion": "success"}))
            .collect();
        Run {
            jobs,
            logs: Vec::new(),
        }
    }

    /// This run with the job `name` failed, `conclusion`, its steps each `(name, conclusion, HH:MM:SS start)`, and its
    /// log `lines`, none if `None`.
    fn with(
        mut self,
        name: &str,
        conclusion: &str,
        steps: &[(&str, &str, &str)],
        lines: Option<&[String]>,
    ) -> Run {
        let id = 100 + self.jobs.len() as u64;
        let steps: Vec<Value> = steps
            .iter()
            .map(|(s, c, t)| json!({"name": s, "conclusion": c, "started_at": format!("2026-11-11T{t}Z")}))
            .collect();
        self.jobs.push(
            json!({"id": id, "name": name, "status": "completed", "conclusion": conclusion, "steps": steps}),
        );
        if let Some(lines) = lines {
            self.logs.push((id, lines.join("\n") + "\n"));
        }
        self
    }

    /// This run with the job `name` failed in its install step, `step`, whose part of the log holds `lines`.
    fn install_failed(self, name: &str, step: &str, lines: &[&str]) -> Run {
        let steps = [
            ("Set up job", "success", "13:30:00"),
            ("Restore the Mesa packages", "success", "13:30:10"),
            (step, "failure", "13:31:00"),
            (CI_STEP, "skipped", "13:36:00"),
        ];
        let log = install_log(step, lines);
        self.with(name, "failure", &steps, Some(&log[..]))
    }

    /// This run with the job `name` failed in its install step by the archive's outage, as on 11 Nov 2026.
    fn outage(self, name: &str) -> Run {
        self.install_failed(name, INSTALL, &outage_lines())
    }

    /// This run with a product test failing in `name`'s test step.
    fn product_failed(self, name: &str) -> Run {
        let steps = [
            ("Set up job", "success", "13:30:00"),
            (CI_STEP, "failure", "13:31:00"),
        ];
        let lines = [
            at("13:30:00.1", "Current runner version: '2.338.2'"),
            at("13:31:00.1", &format!("##[group]Run {CI_STEP}")),
            at(
                "13:33:00.0",
                "        FAIL [   0.006s] ( 319/5514) engine::branch_read_gpu branch_read_gpu_canon_echoes_its_operands",
            ),
            at("13:33:01.0", "##[error]Process completed with exit code 100."),
        ];
        self.with(name, "failure", &steps, Some(&lines[..]))
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

/// The install step's lines of an outage: apt's ignored fetches and `bounded`'s three tries failing.
fn outage_lines() -> Vec<&'static str> {
    vec![
        IGN,
        "attempt 1 of 3 failed or ran past 90 s, apt-get update",
        IGN,
        "attempt 2 of 3 failed or ran past 90 s, apt-get update",
        IGN,
        LAST_TRY,
    ]
}

/// A job's log: the runner's first line, then the install step `step`'s part holding `lines`, and its error.
fn install_log(step: &str, lines: &[&str]) -> Vec<String> {
    let mut out = vec![
        "\u{feff}2026-11-11T13:30:00.1000000Z Current runner version: '2.338.2'".to_owned(),
        at("13:30:10.1", "##[group]Run ./.github/actions/mesa-debs"),
        at("13:30:11.0", "mesa-debs: installed from the cache: false"),
        at(
            "13:31:00.1",
            &format!("##[group]Run bounded() {{ … }}; bounded 90 apt-get update # {step}"),
        ),
    ];
    for (i, line) in lines.iter().enumerate() {
        out.push(at(&format!("13:3{}:00.0", 2 + i.min(3)), line));
    }
    out.push(at(
        "13:35:59.0",
        "##[error]Process completed with exit code 1.",
    ));
    out
}

/// `script` run by python3 over `run` with `args` (`classify run nextest.toml`, or `decide` with a stand-in `gh`):
/// its exit, stdout, and the step summary it wrote.
fn python(script: &str, run: &Run, decide: bool) -> (bool, String, String) {
    let dir = Scratch::new("ci_per_merge_verdict");
    std::fs::create_dir_all(dir.join("bin")).expect("scratch folder");
    let file = dir.join("ci_per_merge.py");
    std::fs::write(&file, script).expect("script written");
    run.write(&dir.join("run"));
    std::fs::write(dir.join("nextest.toml"), read(CONFIG)).expect("config");
    let summary = dir.join("summary");
    let mut cmd = Command::new("python3");
    cmd.arg("-I").arg(&file).current_dir(&*dir);
    if decide {
        std::fs::create_dir_all(dir.join(".config")).expect(".config");
        std::fs::write(dir.join(".config/nextest.toml"), read(CONFIG)).expect("config");
        let half = run.jobs.len() / 2;
        let pages = format!(
            "{}\n{}\n",
            json!({"jobs": run.jobs[..half]}),
            json!({"jobs": run.jobs[half..]})
        );
        std::fs::write(dir.join("run/pages"), pages).expect("pages");
        write_executable(
            &dir.join("bin/gh"),
            "#!/bin/sh\n\
             args=\"$*\"\n\
             case \"$args\" in\n\
               *'/attempts/'*'/jobs'*) cat \"$STUB_DATA/pages\" ;;\n\
               *'/logs') id=${args##*jobs/}; id=${id%/logs}\n\
                 cat \"$STUB_DATA/logs/$id.log\" 2> /dev/null || { echo 'gh: HTTP 404' >&2; exit 1; } ;;\n\
               *) echo \"unexpected: $args\" >&2; exit 1 ;;\n\
             esac\n",
        )
        .expect("gh stand-in");
        let path = format!(
            "{}:{}",
            dir.join("bin").display(),
            std::env::var("PATH").unwrap_or_default()
        );
        cmd.arg("decide")
            .env("PATH", path)
            .env("STUB_DATA", dir.join("run"))
            .env("GH_REPO", "o/r")
            .env("RUN_ID", "42")
            .env("RUN_ATTEMPT", "1")
            .env("GITHUB_STEP_SUMMARY", &summary);
    } else {
        cmd.args(["classify", "run", "nextest.toml"]);
    }
    let out = cmd
        .timed_output()
        .expect("ci_per_merge_verdict: python3 ran");
    assert!(
        decide || out.status.success(),
        "ci_per_merge_verdict: the classifier failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        std::fs::read_to_string(&summary).unwrap_or_default(),
    )
}

/// Asserts the verdict on `run` under `script` is `want`, and each `(job, kind, says)` is judged `kind` saying `says`,
/// and no other job is judged.
fn check(script: &str, case: &str, run: &Run, want: &str, jobs: &[(&str, &str, &str)]) {
    let (_, text, _) = python(script, run, false);
    assert_eq!(
        text.lines().next().unwrap_or_default(),
        format!("verdict={want}"),
        "ci_per_merge_verdict: {case}: the verdict is not {want}"
    );
    for (name, kind, says) in jobs {
        assert!(
            text.lines()
                .any(|l| l.starts_with(&format!("job={name}: {kind}: ")) && l.contains(says)),
            "ci_per_merge_verdict: {case}: {name} is not judged {kind} ({says})"
        );
    }
    assert_eq!(
        text.lines().filter(|l| l.starts_with("job=")).count(),
        jobs.len(),
        "ci_per_merge_verdict: {case}: a job is judged that did not fail, or one that failed is not"
    );
}

const PASSING: [&str; 4] = [
    "ci-archive",
    "gpu-metal suites",
    "gpu-metal",
    "ci-release-archive",
];

// ---- Any failed job is red --------------------------------------------------------------------------------------

fn check_any_kind(script: &str) {
    let process = Run::passing(&PASSING).with(
        "ci (3)",
        "failure",
        &[
            ("Set up job", "success", "13:30:00"),
            (CI_STEP, "failure", "13:31:00"),
        ],
        Some(
            &[
                at("13:31:00.1", &format!("##[group]Run {CI_STEP}")),
                at(
                    "13:32:00.0",
                    "        FAIL [   1.234s] (  7/900) xtask::ci ci_list_runs_no_control",
                ),
                at(
                    "13:32:01.0",
                    "##[error]Process completed with exit code 100.",
                ),
            ][..],
        ),
    );
    check(
        script,
        "a process test failing in `ci (3)`",
        &process,
        "red",
        &[(
            "ci (3)",
            "other",
            "process tests failed: xtask::ci ci_list_runs_no_control",
        )],
    );
    for (name, step) in [
        (
            "ci-checks",
            "cargo clippy --workspace --all-targets -- -D warnings",
        ),
        ("xtask-ci (2)", "cargo xtask ci --partition 2/4"),
        (
            "web-skeleton",
            "cargo rustc -p gui (wasm32, the web feature)",
        ),
        ("changes", "scripts/ci_changes.sh detect"),
    ] {
        let run = Run::passing(&PASSING).with(
            name,
            "failure",
            &[
                ("Set up job", "success", "13:30:00"),
                (step, "failure", "13:31:00"),
            ],
            None,
        );
        check(
            script,
            &format!("`{name}` failing"),
            &run,
            "red",
            &[(name, "other", "not a correctness job")],
        );
    }
}

#[test]
fn ci_per_merge_verdict_a_failed_job_of_any_kind_is_red() {
    check_any_kind(&script());
}

negative_control!(
    ci_per_merge_verdict_a_failed_job_of_any_kind_is_red,
    "a verdict that reads a failure outside correctness as green, the vetoed D-2",
    expected = "ci_per_merge_verdict: a process test failing in `ci (3)`: the verdict is not red",
    check_any_kind(&edit(
        &script(),
        "RED = (\"correctness\", \"other\")",
        "RED = (\"correctness\",)"
    ))
);

// ---- An install outage is no result -----------------------------------------------------------------------------

const ANY_JOB: [&str; 6] = [
    "ci-release (1)",
    "ci (2)",
    "xtask-ci (3)",
    "gpu-kernel suites",
    "ci-workspace",
    "gpu-lavapipe",
];

fn check_outage(script: &str) {
    let mut run = Run::passing(&PASSING);
    for name in ANY_JOB {
        run = run.outage(name);
    }
    let said: Vec<(&str, &str, &str)> = ANY_JOB
        .iter()
        .map(|n| (*n, "no-result", "`Install Mesa (lavapipe)`: its network or archive download failed (R-979): `Ign:2 http://azure.archive.ubuntu.com/ubuntu noble InRelease`; `attempt 3 of 3 failed or ran past 90 s, apt-get update`"))
        .collect();
    check(
        script,
        "the archive's outage in every kind of job",
        &run,
        "no-result",
        &said,
    );
    let fetch = Run::passing(&PASSING).install_failed(
        "ci-release (2)",
        INSTALL,
        &[
            "Err:7 http://azure.archive.ubuntu.com/ubuntu noble-updates/main amd64 mesa-vulkan-drivers amd64 25.0.7",
            "  Could not connect to azure.archive.ubuntu.com:80 (52.252.75.106), connection timed out",
            "E: Failed to fetch http://azure.archive.ubuntu.com/ubuntu/pool/main/m/mesa/mesa-vulkan-drivers.deb",
            "E: Unable to fetch some archives, maybe run apt-get update or try with --fix-missing?",
            "attempt 3 of 3 failed or ran past 420 s, apt-get install -y mesa-vulkan-drivers -oDPkg::Lock::Timeout=60",
        ],
    );
    check(
        script,
        "a package's download failing",
        &fetch,
        "no-result",
        &[(
            "ci-release (2)",
            "no-result",
            "`Err:7 http://azure.archive.ubuntu.com",
        )],
    );
}

#[test]
fn ci_per_merge_verdict_an_install_outage_is_no_result_in_any_job() {
    check_outage(&script());
}

negative_control!(
    ci_per_merge_verdict_an_install_outage_is_no_result_in_any_job,
    "a verdict that reads an install outage as green",
    expected = "ci_per_merge_verdict: the archive's outage in every kind of job: the verdict is not no-result",
    check_outage(&edit(
        &script(),
        "        verdict = NO_RESULT\n",
        "        verdict = \"green\"\n"
    ))
);

fn check_decide_no_result(script: &str) {
    let run = Run::passing(&PASSING).outage("ci-release (1)");
    let (ok, out, summary) = python(script, &run, true);
    assert!(
        !ok,
        "ci_per_merge_verdict: decide passed a run with no result"
    );
    assert!(
        out.starts_with("verdict=no-result\n") && summary.contains("**No result, not green:**"),
        "ci_per_merge_verdict: decide does not say the run has no result"
    );
    let (ok, out, summary) = python(script, &Run::passing(&PASSING), true);
    assert!(
        ok && out == "verdict=green\n" && summary.contains("**Green:** no job failed."),
        "ci_per_merge_verdict: decide does not pass a green run"
    );
}

#[test]
fn ci_per_merge_verdict_decide_fails_on_no_result() {
    check_decide_no_result(&script());
}

negative_control!(
    ci_per_merge_verdict_decide_fails_on_no_result,
    "a decide that fails on red alone",
    expected = "ci_per_merge_verdict: decide passed a run with no result",
    check_decide_no_result(&edit(
        &script(),
        "return finish(summary, 0 if verdict == \"green\" else 1)",
        "return finish(summary, 1 if verdict == \"red\" else 0)"
    ))
);

mod decide_green {
    use super::*;
    negative_control!(
        ci_per_merge_verdict_decide_fails_on_no_result,
        "a decide that fails on every verdict",
        expected = "ci_per_merge_verdict: decide does not pass a green run",
        check_decide_no_result(&edit(
            &script(),
            "return finish(summary, 0 if verdict == \"green\" else 1)",
            "return finish(summary, 1)"
        ))
    );
}

// ---- An outage beside another failure is red --------------------------------------------------------------------

fn check_beside(script: &str) {
    let product = Run::passing(&PASSING)
        .outage("ci-release (1)")
        .product_failed("ci (2)");
    check(
        script,
        "the outage beside a product test's failure",
        &product,
        "red",
        &[
            ("ci-release (1)", "no-result", INSTALL),
            ("ci (2)", "correctness", "product tests failed"),
        ],
    );
    let controls = Run::passing(&PASSING).outage("ci (1)").with(
        "xtask-ci (2)",
        "failure",
        &[
            ("Set up job", "success", "13:30:00"),
            ("cargo xtask ci --partition 2/4", "failure", "13:31:00"),
        ],
        None,
    );
    check(
        script,
        "the outage beside `cargo xtask ci`'s failure",
        &controls,
        "red",
        &[
            ("ci (1)", "no-result", INSTALL),
            ("xtask-ci (2)", "other", "not a correctness job"),
        ],
    );
}

#[test]
fn ci_per_merge_verdict_an_outage_beside_another_failure_is_red() {
    check_beside(&script());
}

negative_control!(
    ci_per_merge_verdict_an_outage_beside_another_failure_is_red,
    "a verdict that lets an install outage hide a test failure, reading the run as no result",
    expected =
        "ci_per_merge_verdict: the outage beside a product test's failure: the verdict is not red",
    check_beside(&edit(
        &script(),
        "    if kinds & set(RED):\n",
        "    if kinds & set(RED) and NO_RESULT not in kinds:\n"
    ))
);

/// One job whose install step failed by the outage and whose `!cancelled()` step after it failed too.
fn check_beside_in_its_job(script: &str) {
    let mut log = install_log(INSTALL, &outage_lines());
    log.push(at("13:36:00.1", "##[group]Run df -h /"));
    log.push(at(
        "13:36:01.0",
        "##[error]Process completed with exit code 1.",
    ));
    let run = Run::passing(&PASSING).with(
        "gpu-kernel suites",
        "failure",
        &[
            ("Set up job", "success", "13:30:00"),
            (INSTALL, "failure", "13:31:00"),
            ("Runner disk after the tests", "failure", "13:36:00"),
        ],
        Some(&log[..]),
    );
    check(
        script,
        "the outage beside another failed step of its job",
        &run,
        "red",
        &[("gpu-kernel suites", "other", "no test step failed")],
    );
}

#[test]
fn ci_per_merge_verdict_an_outage_beside_a_failed_step_of_its_job_is_red() {
    check_beside_in_its_job(&script());
}

negative_control!(
    ci_per_merge_verdict_an_outage_beside_a_failed_step_of_its_job_is_red,
    "a verdict that reads a job as an outage when its first failed step is the install step",
    expected = "ci_per_merge_verdict: the outage beside another failed step of its job: the verdict is not red",
    check_beside_in_its_job(&edit(&script(), "len(failed) != 1", "not failed"))
);

// ---- The install outage is narrow -------------------------------------------------------------------------------

/// Each `(case, run)` reads red, its one failed job `job` judged `other`.
fn check_red_each(script: &str, cases: &[(&str, Run, &str)]) {
    for (case, run, job) in cases {
        check(script, case, run, "red", &[(*job, "other", "")]);
    }
}

fn package_error() -> Run {
    let mut lines = outage_lines();
    lines.insert(0, "E: Unable to locate package mesa-vulkan-drivers");
    Run::passing(&PASSING).install_failed("ci (2)", INSTALL, &lines)
}

fn no_network() -> Run {
    Run::passing(&PASSING).install_failed(
        "ci (2)",
        INSTALL,
        &["Reading package lists...", LAST_TRY],
    )
}

fn not_the_last_try() -> Run {
    Run::passing(&PASSING).install_failed(
        "ci (2)",
        INSTALL,
        &[
            IGN,
            "attempt 2 of 3 failed or ran past 90 s, apt-get update",
            "##[error]The action 'Install Mesa (lavapipe)' has timed out after 8 minutes.",
        ],
    )
}

fn another_install_step() -> Run {
    Run::passing(&PASSING).install_failed(
        "ci (2)",
        "rustup toolchain install (rust-toolchain.toml)",
        &outage_lines(),
    )
}

/// The network evidence is in the restore step's part, before the install step's own.
fn evidence_elsewhere() -> Run {
    let lines = [
        "\u{feff}2026-11-11T13:30:00.1000000Z Current runner version: '2.338.2'".to_owned(),
        at("13:30:10.1", "##[group]Run ./.github/actions/mesa-debs"),
        at("13:30:11.0", IGN),
        at("13:31:00.1", "##[group]Run bounded() { … }"),
        at("13:35:00.0", LAST_TRY),
        at("13:35:59.0", "##[error]Process completed with exit code 1."),
    ];
    Run::passing(&PASSING).with(
        "ci (2)",
        "failure",
        &[
            ("Set up job", "success", "13:30:00"),
            ("Restore the Mesa packages", "success", "13:30:10"),
            (INSTALL, "failure", "13:31:00"),
        ],
        Some(&lines[..]),
    )
}

fn cancelled() -> Run {
    let log = install_log(INSTALL, &outage_lines());
    Run::passing(&PASSING).with(
        "ci (2)",
        "cancelled",
        &[
            ("Set up job", "success", "13:30:00"),
            (INSTALL, "cancelled", "13:31:00"),
        ],
        Some(&log[..]),
    )
}

fn no_log() -> Run {
    Run::passing(&PASSING).with(
        "ci (2)",
        "failure",
        &[
            ("Set up job", "success", "13:30:00"),
            (INSTALL, "failure", "13:31:00"),
        ],
        None,
    )
}

fn narrow_cases() -> Vec<(&'static str, Run, &'static str)> {
    vec![
        ("a package error", package_error(), "ci (2)"),
        ("no network error", no_network(), "ci (2)"),
        ("not the last try failing", not_the_last_try(), "ci (2)"),
        ("another install step", another_install_step(), "ci (2)"),
        (
            "the evidence in another step's part",
            evidence_elsewhere(),
            "ci (2)",
        ),
        ("a job cancelled", cancelled(), "ci (2)"),
        ("a job without its log", no_log(), "ci (2)"),
    ]
}

#[test]
fn ci_per_merge_verdict_an_install_outage_is_narrow() {
    check_red_each(&script(), &narrow_cases());
}

negative_control!(
    ci_per_merge_verdict_an_install_outage_is_narrow,
    "a verdict that reads no package error",
    expected = "ci_per_merge_verdict: a package error: the verdict is not red",
    check_red_each(
        &edit(
            &script(),
            " or any(PACKAGE.search(line) for line in lines)",
            ""
        ),
        &narrow_cases()
    )
);

mod no_network_error {
    use super::*;
    negative_control!(
        ci_per_merge_verdict_an_install_outage_is_narrow,
        "a verdict that asks for no network error",
        expected = "ci_per_merge_verdict: no network error: the verdict is not red",
        check_red_each(
            &edit(
                &script(),
                "exhausted is None or network is None",
                "exhausted is None"
            ),
            &narrow_cases()
        )
    );
}

mod any_try {
    use super::*;
    negative_control!(
        ci_per_merge_verdict_an_install_outage_is_narrow,
        "a verdict that takes any try failing for the last",
        expected = "ci_per_merge_verdict: not the last try failing: the verdict is not red",
        check_red_each(
            &edit(
                &script(),
                "^attempt (\\d+) of \\1 failed",
                "^attempt (\\d+) of \\d+ failed"
            ),
            &narrow_cases()
        )
    );
}

mod any_step {
    use super::*;
    negative_control!(
        ci_per_merge_verdict_an_install_outage_is_narrow,
        "a verdict that reads any step's failure as the install step's",
        expected = "ci_per_merge_verdict: another install step: the verdict is not red",
        check_red_each(
            &edit(
                &script(),
                "    if step not in INSTALL_STEPS:\n",
                "    if not step:\n"
            ),
            &narrow_cases()
        )
    );
}

mod the_whole_log {
    use super::*;
    negative_control!(
        ci_per_merge_verdict_an_install_outage_is_narrow,
        "a verdict that reads the evidence from the whole log",
        expected =
            "ci_per_merge_verdict: the evidence in another step's part: the verdict is not red",
        check_red_each(
            &edit(
                &script(),
                "for line in own.get(failed[0], [])]",
                "for part in own.values() for line in part]"
            ),
            &narrow_cases()
        )
    );
}

mod any_conclusion {
    use super::*;
    negative_control!(
        ci_per_merge_verdict_an_install_outage_is_narrow,
        "a verdict that reads a cancelled job as an outage",
        expected = "ci_per_merge_verdict: a job cancelled: the verdict is not red",
        check_red_each(
            &edit(&script(), "job.get(\"conclusion\") != \"failure\" or ", ""),
            &narrow_cases()
        )
    );
}

// ---- The saved runs ---------------------------------------------------------------------------------------------

/// A saved run under `fixtures/ci_per_merge/`, read whole.
fn saved(id: &str) -> Run {
    let dir = root().join("fixtures/ci_per_merge").join(id);
    let jobs: Vec<Value> = serde_json::from_str(
        &std::fs::read_to_string(dir.join("jobs.json")).expect("ci_per_merge_verdict: a saved run"),
    )
    .expect("ci_per_merge_verdict: its jobs");
    let mut logs = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir.join("logs")) {
        for entry in entries {
            let path = entry.expect("an entry").path();
            let id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .and_then(|s| s.parse().ok())
                .expect("a job id");
            logs.push((id, std::fs::read_to_string(&path).expect("a saved log")));
        }
    }
    Run { jobs, logs }
}

/// The saved runs: each verdict, and each failed job's kind.
fn check_saved(script: &str) {
    let outage = |names: &[&'static str]| -> Vec<(&'static str, &'static str, &'static str)> {
        names.iter().map(|n| (*n, "no-result", IGN)).collect()
    };
    let mut first = vec![("xtask-ci (2)", "other", "not a correctness job")];
    first.extend(outage(&["ci-release (1)"]));
    let mut second = vec![("xtask-ci (2)", "other", "not a correctness job")];
    second.extend(outage(&[
        "ci-release (2)",
        "ci-release (1)",
        "gpu-lavapipe",
        "ci (1)",
        "ci (2)",
        "ci (3)",
        "ci (4)",
    ]));
    let third = outage(&[
        "gpu-lavapipe suites",
        "gpu-kernel suites",
        "xtask-ci (1)",
        "xtask-ci (2)",
        "xtask-ci (3)",
        "xtask-ci (4)",
        "ci-workspace",
        "gpu-lavapipe",
        "ci-release (1)",
        "ci-release (2)",
        "ci (1)",
        "ci (2)",
        "ci (3)",
        "ci (4)",
    ]);
    let d2 = vec![("xtask-ci (2)", "other", "not a correctness job")];
    for (id, want, jobs) in [
        ("43029168415", "no-result", third),
        ("43026202826", "red", first),
        ("43026967872", "red", second),
        ("43133837255", "red", d2),
        ("43155593554", "green", vec![]),
    ] {
        check(script, &format!("run {id}"), &saved(id), want, &jobs);
    }
}

#[test]
fn ci_per_merge_verdict_reads_the_saved_runs() {
    check_saved(&script());
}

negative_control!(
    ci_per_merge_verdict_reads_the_saved_runs,
    "a verdict that takes apt's `Err:` lines alone for a failed fetch, not the `Ign:` lines the outage printed",
    expected = "ci_per_merge_verdict: run 43029168415: the verdict is not no-result",
    check_saved(&edit(&script(), "r\"^(?:Ign|Err):\\d+ https?://\"", "r\"^Err:\\d+ https?://\""))
);
