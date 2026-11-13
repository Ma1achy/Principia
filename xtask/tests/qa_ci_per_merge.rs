//! qa's tests for the per-merge verdict (`.github/scripts/ci_per_merge.py`, `ci.yml`'s `per-merge` job), written from
//! the rulings, not the script. R-982 (12 Nov 2026, the owner's veto of D-2): "Any red job on main is red; make the
//! per-merge verdict cover every job so the two can't disagree"; R-979 (RQ-789 (a)): only the install step's network and
//! archive-download failures count as infrastructure, their verdict "no result", never green; REQ-TOOL-228, which
//! states both. Earlier rulings still shape what a failed job's line says (`plan/OPERATIONS.md` § "Merging": each failed
//! job's kind, `correctness` or `other`, both red): R-907 item 1, the product-tooling tests (`build_kernel`,
//! `lint_wgsl_*`) are product tests, not process ones; R-909, the golden runner over every suite (`qa_TASK-M0-06_runner`,
//! `_list`) and the screenshot runner's self-test (`qa_TASK-M0-20`) are product correctness, run per merge.
//!
//! - `qa_per_merge_every_test_step_on_a_push_is_read_as_one`: over the real `ci.yml`, each step of each job a push runs
//!   that runs tests (`cargo nextest run`, `cargo test`, `cargo xtask golden`, `gate` or `screenshot`), named as GitHub
//!   names it (a step without a `name:` is `Run <its first line>`), failing with a product test in its log, gives the
//!   verdict red and the job the kind `correctness`. A step renamed so the verdict no longer reads it as a test step
//!   would call a golden or GPU failure something else.
//! - `qa_per_merge_product_tooling_and_golden_failures_are_red`: against the real `.config/nextest.toml`, each
//!   product-tooling binary and R-909's named product correctness, failing alone in a `ci` shard, is red and
//!   `correctness`, and a process test (`plan_check`, R-907 item 1's list) failing alone is red too (R-982), `other`.
//!
//! - `qa_per_merge_each_failed_step_is_judged_by_its_own_log`: a `ci-workspace` run whose nextest step fails a process
//!   test and whose doctest step, later in the same log, fails a doctest (which names no nextest test) is red and
//!   `correctness`: the doctest is a product test, and the process test's line elsewhere in the job's log must not make
//!   it `other`.
//! - `qa_per_merge_a_part_run_suite_is_red`: a `ci` shard whose nextest summary counts fewer tests run than it
//!   selected is red and `correctness` even when the only failure it names is a process test's: product tests went
//!   unrun (R-909).
//! - `qa_per_merge_no_skipped_product_step_reads_task`: over the real `ci.yml`, each test step of each job a push runs,
//!   failing with a process test alone, and each later step GitHub's `if:` rules would then skip, skipped: the verdict is
//!   red (R-982), and the job `correctness` while a skipped step runs product tests (anything but `cargo nextest run -p
//!   <pkg> --test <process binary>…`); in `ci-workspace` and `gpu-kernel`, whose suites R-909 has run whole, a process
//!   failure is `other`.
//! - `qa_per_merge_product_profiles_run_on_after_a_failure`: the `ci`, `ci-workspace` and `gpu-kernel` nextest
//!   profiles run every selected test after a failure (`fail-fast` false, as set or inherited).
//! - `qa_per_merge_every_job_of_the_push_run_failing_is_red` (R-982): `per-merge` waits for every job `ci.yml` runs on a
//!   push, whatever each ended in, and fails with the verdict; and every one of those jobs, failing at any of its own
//!   steps (with its log and without), or ending cancelled, timed out or in any other failed state, reads red, while all
//!   of them passing reads green.
//! - `qa_per_merge_the_install_steps_outage_is_no_result` (R-979, with R-982): the real Mesa install step of `ci.yml`,
//!   run under bash with stand-in `sudo`, `timeout` and `apt-get`, its archive hanging, unresolvable or refusing a
//!   package's download: the job failing so, in every job a push runs that has the step, and all of them at once, reads
//!   "no result", its line naming the step; the same step failing on a package or `dpkg` error, a network error beside
//!   it or not, reads red; the outage beside the lints' failure, or a process test's in another job, reads red.
//! - `qa_per_merge_decide_fails_unless_green_and_files_nothing`: `decide`, its whole path against a stand-in `gh`, fails
//!   the job on a run whose one failure is a process test, on an install outage and on a run it cannot read, passes a
//!   green run (its own job, still running, not counted), and asks `gh` for nothing but reads: no issue, no label.
//!
//! Each has its registered negative control (R-176): the same check on a `ci.yml` whose golden steps are renamed; on
//! a configuration whose process profile names a `lint_wgsl` binary; on a script that gives every failed step the
//! whole job's log; on one that never reads the summary's unrun count; on one that takes every skipped step as process
//! tests alone; on a configuration whose `ci-workspace` profile stops at the first failure; on a verdict that reads
//! a failure outside correctness as green (the vetoed D-2); on a `per-merge` that does not wait for `xtask-ci`; on a
//! verdict that reads an install outage as green, and on one that lets an outage hide another failure. Unix only, as
//! python3 runs the script and bash the install step.
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use validation::negative_control;

const SCRIPT: &str = ".github/scripts/ci_per_merge.py";
const CONFIG: &str = ".config/nextest.toml";
const CI: &str = ".github/workflows/ci.yml";

/// The jobs OPERATIONS § "Merging" calls `other` whatever fails in them, red all the same (R-982): the classification
/// (`changes`), the lints and build beside the tests (`ci-checks`), `cargo xtask ci` (`xtask-ci`) and the web skeleton.
const BESIDE: [&str; 4] = ["changes", "ci-checks", "xtask-ci", "web-skeleton"];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the workspace root")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel))
        .unwrap_or_else(|e| panic!("qa_ci_per_merge: {rel}: {e}"))
}

/// `text` with every `from` replaced by `to`, which must be in it: a control's edit.
#[cfg(feature = "controls")]
fn edit(text: &str, from: &str, to: &str) -> String {
    assert!(
        text.contains(from),
        "qa_ci_per_merge: a control's edit target is gone: {from}"
    );
    text.replace(from, to)
}

/// `program` run by python3 with the script loaded as `m`, `sys.argv[1]` the script and the further arguments after:
/// its standard output parsed as JSON.
fn python(program: &str, files: &[(&str, &str)]) -> Value {
    python_with("", program, files)
}

/// As `python`, with `prelude` run first, after the script is loaded: a control's edit of the script's functions.
fn python_with(prelude: &str, program: &str, files: &[(&str, &str)]) -> Value {
    let dir = std::env::temp_dir().join(format!(
        "qa_ci_per_merge_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("qa_ci_per_merge: scratch");
    let mut args = vec![root().join(SCRIPT).to_string_lossy().into_owned()];
    for (name, text) in files {
        let path = dir.join(name);
        std::fs::write(&path, text).expect("qa_ci_per_merge: a file");
        args.push(path.to_string_lossy().into_owned());
    }
    let full = format!(
        "import importlib.util, json, sys\n\
         spec = importlib.util.spec_from_file_location('m', sys.argv[1])\n\
         m = importlib.util.module_from_spec(spec)\n\
         spec.loader.exec_module(m)\n{prelude}\n{program}"
    );
    let out = Command::new("python3")
        .arg("-I")
        .arg("-c")
        .arg(&full)
        .args(&args)
        .output()
        .expect("qa_ci_per_merge: python3 runs");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        out.status.success(),
        "qa_ci_per_merge: python3 failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("qa_ci_per_merge: JSON")
}

// ---- Every test step a push runs ------------------------------------------------------------------------------------

/// For each step of each job a push runs whose command runs tests: [job key, job name, step name, verdict, the command's
/// first line], the step failing with a product test's nextest line in the job's log.
const STEPS: &str = r#"
import re, yaml
ci = yaml.safe_load(open(sys.argv[2]))
process = m.process_binaries(open(sys.argv[3]).read())
runs = re.compile(r"cargo (nextest run|test)\b|cargo xtask (golden|gate|screenshot)\b")
log = "2026-11-10T00:00:00.0000000Z         FAIL [   0.010s] engine::canon_gpu canonicaliser_seam_sweep\n"
def shown(text):
    return re.sub(r"\$\{\{\s*matrix\.[a-z_]+\s*\}\}", "1", text)
out = []
for key, job in ci["jobs"].items():
    if "== 'pull_request'" in str(job.get("if", "")) or key == "per-merge":
        continue
    if job.get("name"):
        name = shown(job["name"])
    elif job.get("strategy"):
        name = f"{key} (1)"
    else:
        name = key
    for step in job.get("steps") or []:
        run = step.get("run") or ""
        if not runs.search(run):
            continue
        step_name = shown(step.get("name") or "Run " + run.strip().splitlines()[0])
        jobs = [{"id": 7, "name": name, "conclusion": "failure",
                 "steps": [{"name": "Set up job", "conclusion": "success"},
                           {"name": step_name, "conclusion": "failure"}]}]
        verdict, judged = m.classify(jobs, {"7": log}, process)
        out.append([key, name, step_name, verdict, run.strip().splitlines()[0], judged[0][1] if judged else None])
print(json.dumps(out))
"#;

fn check_steps(ci: &str) {
    let got = python(STEPS, &[("ci.yml", ci), ("nextest.toml", &read(CONFIG))]);
    let rows = got.as_array().expect("qa_ci_per_merge: a list");
    let commands: Vec<&str> = rows.iter().filter_map(|r| r[4].as_str()).collect();
    // Not vacuous: the owner's per-merge list is there to be read (R-909): the golden runner on both GPUs, the
    // decode-agreement gate, the `ci` shards' archive run.
    for want in [
        "cargo xtask golden --all",
        "cargo xtask gate decode-agreement",
    ] {
        assert!(
            commands.iter().filter(|c| c.starts_with(want)).count() >= 2,
            "qa_ci_per_merge: {want} is not run on lavapipe and on Metal"
        );
    }
    assert!(
        rows.len() >= 30,
        "qa_ci_per_merge: only {} test steps on a push",
        rows.len()
    );
    for row in rows {
        let key = row[0].as_str().expect("a key");
        let name = row[1].as_str().expect("a name");
        let step = row[2].as_str().expect("a step");
        let verdict = row[3].as_str().expect("a verdict");
        let kind = row[5].as_str().unwrap_or("nothing");
        // R-982: any failed job is red, whatever its kind.
        assert!(
            verdict == "red",
            "qa_ci_per_merge: a product test failing in step `{step}` of `{name}` reads {verdict}, not red"
        );
        if !BESIDE.contains(&key) {
            assert!(
                kind == "correctness",
                "qa_ci_per_merge: a product test failing in step `{step}` of `{name}` is judged {kind}, not correctness"
            );
        }
    }
}

#[test]
fn qa_per_merge_every_test_step_on_a_push_is_read_as_one() {
    check_steps(&read(CI));
}

negative_control!(
    qa_per_merge_every_test_step_on_a_push_is_read_as_one,
    "the golden runner's steps renamed so the verdict does not read them as test steps",
    expected = "qa_ci_per_merge: a product test failing in step `The golden runner over every suite` of `gpu-metal suites` is judged other, not correctness",
    check_steps(&edit(
        &read(CI),
        "- name: cargo xtask golden --all\n",
        "- name: The golden runner over every suite\n"
    ))
);

// ---- Product tooling and golden failures ----------------------------------------------------------------------------

/// For each binary, one a line in the file `sys.argv[3]`: the verdict on a run whose `ci (1)` shard failed its nextest
/// step with that binary's test alone, and the shard's kind.
const ALONE: &str = r#"
process = m.process_binaries(open(sys.argv[2]).read())
out = {}
for binary in open(sys.argv[3]).read().split("\n"):
    jobs = [{"id": 9, "name": "ci (1)", "conclusion": "failure",
             "steps": [{"name": "cargo nextest run --archive-file … --partition hash:1/4", "conclusion": "failure"}]}]
    log = f"        FAIL [   1.250s] ( 12/5514) {binary} a_test\n"
    verdict, judged = m.classify(jobs, {"9": log}, process)
    out[binary] = [verdict, judged[0][1] if judged else None]
print(json.dumps(out))
"#;

/// R-907 item 1's product tooling (`build_kernel`, every `lint_wgsl_*` test file in xtask) and R-909's product
/// correctness: the binaries whose failure on `main` is red.
fn product_binaries() -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(root().join("xtask/tests"))
        .expect("qa_ci_per_merge: xtask/tests")
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().to_str().map(str::to_owned))
        .filter_map(|f| f.strip_suffix(".rs").map(str::to_owned))
        .filter(|s| s.starts_with("lint_wgsl") || s == "build_kernel")
        .map(|s| format!("xtask::{s}"))
        .collect();
    out.sort();
    assert!(
        out.len() >= 10 && out.contains(&"xtask::build_kernel".to_owned()),
        "qa_ci_per_merge: the product tooling's test files are not found: {out:?}"
    );
    for named in [
        "xtask::qa_TASK-M0-06_runner",
        "xtask::qa_TASK-M0-06_list",
        "xtask::qa_TASK-M0-20",
    ] {
        out.push(named.to_owned());
    }
    out
}

fn check_alone(prelude: &str, config: &str) {
    let product = product_binaries();
    let process = "xtask::plan_check";
    let names: Vec<String> = product
        .iter()
        .cloned()
        .chain([process.to_owned()])
        .collect();
    let got = python_with(
        prelude,
        ALONE,
        &[("nextest.toml", config), ("binaries", &names.join("\n"))],
    );
    for binary in &product {
        let verdict = got[binary.as_str()][0].as_str().expect("a verdict");
        let kind = got[binary.as_str()][1].as_str().unwrap_or("nothing");
        assert!(
            verdict == "red",
            "qa_ci_per_merge: {binary} failing reads {verdict}, not red"
        );
        assert!(
            kind == "correctness",
            "qa_ci_per_merge: {binary} failing is judged {kind}, not correctness"
        );
    }
    // R-982: a process test failing alone is red too, its kind `other`.
    let verdict = got[process][0].as_str().expect("a verdict");
    let kind = got[process][1].as_str().unwrap_or("nothing");
    assert!(
        verdict == "red",
        "qa_ci_per_merge: a process test failing alone reads {verdict}, not red"
    );
    assert!(
        kind == "other",
        "qa_ci_per_merge: a process test failing alone is judged {kind}, not other"
    );
}

#[test]
fn qa_per_merge_product_tooling_and_golden_failures_are_red() {
    check_alone("", &read(CONFIG));
}

negative_control!(
    qa_per_merge_product_tooling_and_golden_failures_are_red,
    "a process profile that names a lint_wgsl binary",
    expected = "qa_ci_per_merge: xtask::lint_wgsl_float failing is judged other, not correctness",
    check_alone(
        "",
        &edit(
            &read(CONFIG),
            "        | binary_id(=xtask::plan_check)\n",
            "        | binary_id(=xtask::plan_check)\n        | binary_id(=xtask::lint_wgsl_float)\n"
        )
    )
);

mod process_alone_green {
    use super::*;
    negative_control!(
        qa_per_merge_product_tooling_and_golden_failures_are_red,
        "a verdict that reads a failure outside correctness as green, the vetoed D-2",
        expected = "qa_ci_per_merge: a process test failing alone reads green, not red",
        check_alone("m.RED = ('correctness',)", &read(CONFIG))
    );
}

// ---- Each failed step judged by its own part of the log --------------------------------------------------------------

/// A `ci-workspace` run: its nextest step fails `xtask::plan_check`, a process test, and its doctest step, minutes later
/// in the same log, ends as `DOC` says (`success` or `failure`). The verdict and the job's kind.
const OWN_LOG: &str = r#"
doc = DOC
steps = [
    {"name": "Set up job", "conclusion": "success", "started_at": "2026-11-10T09:00:00Z"},
    {"name": "cargo nextest run -p xtask", "conclusion": "failure", "started_at": "2026-11-10T09:00:10Z"},
    {"name": "cargo nextest run -p validation --test qa_TASK-M0-24", "conclusion": "skipped", "started_at": None},
    {"name": "cargo nextest run --features controls (the default-feature listing's controls)",
     "conclusion": "skipped", "started_at": None},
    {"name": "Runner disk after the tests", "conclusion": "success", "started_at": "2026-11-10T09:02:00Z"},
    {"name": "cargo test -p xtask --doc", "conclusion": doc, "started_at": "2026-11-10T09:02:01Z"},
]
lines = [
    "2026-11-10T09:00:00.1000000Z Current runner version: '2.329.0'",
    "2026-11-10T09:00:10.2000000Z ##[group]Run cargo nextest run -p xtask",
    "2026-11-10T09:00:10.2000000Z cargo nextest run -p xtask",
    "2026-11-10T09:00:10.2000000Z ##[endgroup]",
    "2026-11-10T09:01:50.0000000Z         FAIL [   0.120s] ( 40/ 900) xtask::plan_check plan_check_passes",
    "2026-11-10T09:01:59.9000000Z      Summary [ 109.700s] 900 tests run: 899 passed, 1 failed",
    "2026-11-10T09:01:59.9000000Z error: test run failed",
    "2026-11-10T09:01:59.9000000Z ##[error]Process completed with exit code 100.",
    "2026-11-10T09:02:00.1000000Z ##[group]Run df -h /",
    "2026-11-10T09:02:00.1000000Z Filesystem      Size  Used Avail Use% Mounted on",
    "2026-11-10T09:02:01.2000000Z ##[group]Run cargo test -p xtask --doc",
    "2026-11-10T09:02:01.2000000Z cargo test -p xtask --doc",
    "2026-11-10T09:02:01.2000000Z ##[endgroup]",
]
if doc == "failure":
    lines += [
        "2026-11-10T09:02:30.0000000Z test xtask/src/lib.rs - plan (line 12) ... FAILED",
        "2026-11-10T09:02:30.0000000Z error: doctest failed, to rerun pass `--doc`",
        "2026-11-10T09:02:30.1000000Z ##[error]Process completed with exit code 101.",
    ]
else:
    lines += ["2026-11-10T09:02:30.0000000Z test result: ok. 3 passed; 0 failed; 0 ignored"]
jobs = [{"id": 11, "name": "ci-workspace", "conclusion": "failure", "steps": steps}]
verdict, judged = m.classify(jobs, {"11": "\n".join(lines) + "\n"}, m.process_binaries(open(sys.argv[2]).read()))
print(json.dumps([verdict, judged[0][1] if judged else None]))
"#;

fn check_own_log(prelude: &str) {
    let config = read(CONFIG);
    let run = |doc: &str| -> (String, String) {
        let program = OWN_LOG.replacen("DOC", &format!("{doc:?}"), 1);
        let got = python_with(prelude, &program, &[("nextest.toml", &config)]);
        (
            got[0].as_str().expect("a verdict").to_owned(),
            got[1].as_str().unwrap_or("nothing").to_owned(),
        )
    };
    // Not vacuous: with the doctests passing, the process test's failure alone is red (R-982) and `other`
    // (OPERATIONS § "Merging").
    let (verdict, kind) = run("success");
    assert!(
        verdict == "red" && kind == "other",
        "qa_ci_per_merge: a process test failing alone in ci-workspace reads {verdict}, judged {kind}, not red and other"
    );
    let (verdict, kind) = run("failure");
    assert!(
        verdict == "red",
        "qa_ci_per_merge: a doctest failing after a process test in ci-workspace reads {verdict}, not red"
    );
    assert!(
        kind == "correctness",
        "qa_ci_per_merge: a doctest failing after a process test in ci-workspace is judged {kind}, not correctness"
    );
}

#[test]
fn qa_per_merge_each_failed_step_is_judged_by_its_own_log() {
    check_own_log("");
}

negative_control!(
    qa_per_merge_each_failed_step_is_judged_by_its_own_log,
    "a script that gives every failed step the whole job's log",
    expected = "qa_ci_per_merge: a doctest failing after a process test in ci-workspace is judged other, not correctness",
    check_own_log("m.parts = lambda steps, log: {i: m.clean(log) for i in range(len(steps))}")
);

// ---- A part-run suite ------------------------------------------------------------------------------------------------

/// A `ci (2)` shard failing its nextest step with `xtask::plan_check` alone, its summary line `SUMMARY`. The verdict
/// and the shard's kind.
const PART_RUN: &str = r#"
steps = [
    {"name": "Set up job", "conclusion": "success", "started_at": "2026-11-10T09:00:00Z"},
    {"name": "cargo test --workspace --doc", "conclusion": "skipped", "started_at": None},
    {"name": "cargo nextest run --archive-file … --partition hash:2/4", "conclusion": "failure",
     "started_at": "2026-11-10T09:00:10Z"},
]
log = "\n".join([
    "2026-11-10T09:00:00.1000000Z Current runner version: '2.329.0'",
    "2026-11-10T09:00:10.2000000Z ##[group]Run cargo nextest run --archive-file $RUNNER_TEMP/nextest-ci.tar.zst",
    "2026-11-10T09:01:00.0000000Z         FAIL [   0.120s] ( 321/5514) xtask::plan_check plan_check_passes",
    "2026-11-10T09:01:21.0000000Z      SUMMARY",
    "2026-11-10T09:01:21.0000000Z error: test run failed",
]) + "\n"
jobs = [{"id": 12, "name": "ci (2)", "conclusion": "failure", "steps": steps}]
verdict, judged = m.classify(jobs, {"12": log}, m.process_binaries(open(sys.argv[2]).read()))
print(json.dumps([verdict, judged[0][1] if judged else None]))
"#;

fn check_part_run(prelude: &str) {
    let config = read(CONFIG);
    let run = |summary: &str| -> (String, String) {
        let got = python_with(
            prelude,
            &PART_RUN.replacen("SUMMARY", summary, 1),
            &[("nextest.toml", &config)],
        );
        (
            got[0].as_str().expect("a verdict").to_owned(),
            got[1].as_str().unwrap_or("nothing").to_owned(),
        )
    };
    // Not vacuous: every selected test run, the process test's failure alone is red (R-982) and `other`.
    let (verdict, kind) = run("Summary [  71.045s] 5514 tests run: 5513 passed, 1 failed");
    assert!(
        verdict == "red" && kind == "other",
        "qa_ci_per_merge: a whole run failing a process test reads {verdict}, judged {kind}, not red and other"
    );
    let (verdict, kind) = run("Summary [  71.045s] 322/5514 tests run: 321 passed, 1 failed");
    assert!(
        verdict == "red",
        "qa_ci_per_merge: a run that left 5192 tests unrun reads {verdict}, not red"
    );
    assert!(
        kind == "correctness",
        "qa_ci_per_merge: a run that left 5192 tests unrun is judged {kind}, not correctness"
    );
}

#[test]
fn qa_per_merge_a_part_run_suite_is_red() {
    check_part_run("");
}

negative_control!(
    qa_per_merge_a_part_run_suite_is_red,
    "a script that never reads the summary's count of tests unrun",
    expected = "qa_ci_per_merge: a run that left 5192 tests unrun is judged other, not correctness",
    check_part_run("m.unrun = lambda lines: None")
);

// ---- Skipped product steps -------------------------------------------------------------------------------------------

/// For each test step of each job a push runs: that step failing with a process test alone, the steps before it passed,
/// each after it skipped unless its `if:` runs it after a failure (`always()`, `!cancelled()`, `failure()`), as GitHub
/// does. Rows of [job key, job name, failed step, verdict, skipped steps running product tests, every skipped test step,
/// the job's kind].
const SKIPS: &str = r#"
import re, yaml
ci = yaml.safe_load(open(sys.argv[2]))
process = m.process_binaries(open(sys.argv[3]).read())
runs = re.compile(r"cargo (nextest run|test)\b|cargo xtask (golden|gate|screenshot)\b")
def shown(text):
    return re.sub(r"\$\{\{\s*matrix\.[a-z_]+\s*\}\}", "1", text)
def after_failure(cond):
    return any(f in str(cond or "") for f in ("always()", "!cancelled()", "failure()"))
def binaries(run):
    words = run.split()
    if words[:3] != ["cargo", "nextest", "run"] or "--lib" in words:
        return None
    pk = [words[i + 1] for i, w in enumerate(words[:-1]) if w == "-p"]
    tests = [words[i + 1] for i, w in enumerate(words[:-1]) if w == "--test"]
    if len(pk) != 1 or not tests:
        return None
    return [f"{pk[0]}::{t}" for t in tests]
log = "2026-11-10T00:00:00.0000000Z         FAIL [   0.010s] xtask::plan_check a_test\n"
out = []
for key, job in ci["jobs"].items():
    if "== 'pull_request'" in str(job.get("if", "")) or key == "per-merge":
        continue
    name = shown(job["name"]) if job.get("name") else (f"{key} (1)" if job.get("strategy") else key)
    steps = job.get("steps") or []
    for i, step in enumerate(steps):
        if not runs.search(step.get("run") or ""):
            continue
        listed, skipped = [], []
        for j, s in enumerate(steps):
            first = (s.get("run") or s.get("uses") or "").strip().splitlines()[0]
            sname = shown(s.get("name") or "Run " + first)
            if j < i:
                c = "success"
            elif j == i:
                c = "failure"
            elif after_failure(s.get("if")):
                c = "success"
            else:
                c = "skipped"
                if runs.search(s.get("run") or ""):
                    skipped.append([sname, binaries(first)])
            listed.append({"name": sname, "conclusion": c})
        jobs = [{"id": 7, "name": name, "conclusion": "failure", "steps": listed}]
        verdict, judged = m.classify(jobs, {"7": log}, process)
        hidden = [n for n, b in skipped if b is None or any(x not in process for x in b)]
        out.append([key, name, listed[i]["name"], verdict, hidden, [n for n, _ in skipped],
                    judged[0][1] if judged else None])
print(json.dumps(out))
"#;

fn check_skips(prelude: &str, ci: &str) {
    let got = python_with(
        prelude,
        SKIPS,
        &[("ci.yml", ci), ("nextest.toml", &read(CONFIG))],
    );
    let rows = got.as_array().expect("qa_ci_per_merge: a list");
    let mut hiding = 0;
    for row in rows {
        let key = row[0].as_str().expect("a key");
        let name = row[1].as_str().expect("a name");
        let step = row[2].as_str().expect("a step");
        let verdict = row[3].as_str().expect("a verdict");
        let hidden = row[4].as_array().expect("hidden");
        let skipped = row[5].as_array().expect("skipped");
        let kind = row[6].as_str().unwrap_or("nothing");
        // R-982: a process test failing is red, whatever else ran.
        assert!(
            verdict == "red",
            "qa_ci_per_merge: `{step}` of `{name}` failing with a process test reads {verdict}, not red"
        );
        if !hidden.is_empty() {
            hiding += 1;
            assert!(
                kind == "correctness",
                "qa_ci_per_merge: `{step}` of `{name}` failing with a process test leaves {} unrun, and is judged {kind}, not correctness",
                hidden[0]
            );
        }
        // R-909's two suites run whole after a process test fails: `other`, though process-only steps are skipped.
        if (key == "ci-workspace" || key == "gpu-kernel")
            && step.starts_with("cargo nextest run -p ")
        {
            assert!(
                kind == "other" && hidden.is_empty(),
                "qa_ci_per_merge: `{step}` of `{name}` failing with a process test is judged {kind}, skipping {skipped:?}"
            );
        }
    }
    // Not vacuous: the GPU jobs' later steps are skipped after a failure, and the suites R-909 runs whole are read.
    assert!(
        hiding >= 10
            && rows.iter().any(|r| r[0] == "ci-workspace")
            && rows.iter().any(|r| r[0] == "gpu-kernel"),
        "qa_ci_per_merge: the simulation reads too little: {hiding} rows hide a step"
    );
}

#[test]
fn qa_per_merge_no_skipped_product_step_reads_task() {
    check_skips("", &read(CI));
}

negative_control!(
    qa_per_merge_no_skipped_product_step_reads_task,
    "a script that takes every skipped test step as running process tests alone",
    expected = "failing with a process test leaves",
    check_skips(
        "class _All:\n    def __contains__(self, x):\n        return True\nm.PROCESS_STEPS = _All()",
        &read(CI)
    )
);

// ---- Profiles that run on after a failure ----------------------------------------------------------------------------

fn check_profiles(config: &str) {
    let doc: toml_edit::DocumentMut = config
        .parse()
        .expect("qa_ci_per_merge: nextest.toml parses");
    let profile = |name: &str| doc.get("profile").and_then(|p| p.get(name));
    let default = profile("default")
        .and_then(|p| p.get("fail-fast"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    for name in ["ci", "ci-workspace", "gpu-kernel"] {
        let table = profile(name).unwrap_or_else(|| panic!("qa_ci_per_merge: no profile {name}"));
        let fail_fast = table
            .get("fail-fast")
            .and_then(|v| v.as_bool())
            .unwrap_or(default);
        assert!(
            !fail_fast,
            "qa_ci_per_merge: the {name} profile stops at its first failure"
        );
    }
}

#[test]
fn qa_per_merge_product_profiles_run_on_after_a_failure() {
    check_profiles(&read(CONFIG));
}

negative_control!(
    qa_per_merge_product_profiles_run_on_after_a_failure,
    "a configuration whose ci-workspace profile stops at its first failure",
    expected = "qa_ci_per_merge: the ci-workspace profile stops at its first failure",
    check_profiles(&edit(
        &read(CONFIG),
        "[profile.ci-workspace]\nfail-fast = false\n",
        "[profile.ci-workspace]\n"
    ))
);

// ---- R-982: every job of the push run ------------------------------------------------------------------------------

/// Python shared by the tests below, after the script is loaded as `m`: `ci.yml` (`sys.argv[2]`) read, the jobs a push
/// runs (`pushed`, `per-merge` aside) named as GitHub names them, and `build`, which makes one of them fail at its own
/// step `k` with `lines` in that step's part of the log, the steps before it passed, each after it run (and passing) if
/// its `if:` runs it after a failure and skipped if not, as GitHub does.
const JOBS_PY: &str = r##"
import re, yaml
ci = yaml.safe_load(open(sys.argv[2]))
process = m.process_binaries(open(sys.argv[3]).read())
def shown(text):
    return re.sub(r"\$\{\{\s*matrix\.[a-z_]+\s*\}\}", "1", str(text))
def on_push(job):
    return "github.event_name == 'pull_request'" not in str(job.get("if", ""))
def first_line(s):
    return (s.get("run") or s.get("uses") or "").strip().splitlines()[0]
def step_name(s):
    return shown(s.get("name") or "Run " + first_line(s))
def after_failure(cond):
    return any(f in str(cond or "") for f in ("always()", "!cancelled()", "failure()"))
def stamp(sec):
    return f"2026-11-12T04:{sec // 60:02d}:{sec % 60:02d}"
def name_of(key, job):
    if job.get("name"):
        return shown(job["name"])
    return f"{key} (1)" if job.get("strategy") else key
pushed = {k: j for k, j in ci["jobs"].items() if on_push(j) and k != "per-merge"}
ids = {k: i + 1 for i, k in enumerate(pushed)}
def build(key, k, lines, conclusion="failure"):
    real = pushed[key].get("steps") or []
    steps = [{"name": "Set up job", "conclusion": "success", "started_at": stamp(0) + "Z"}]
    log = [stamp(0) + ".1000000Z Current runner version: '2.329.0'"]
    for j, s in enumerate(real):
        sec = 10 * (j + 1)
        if j < k or (j > k and after_failure(s.get("if"))):
            c = "success"
        elif j == k:
            c = "failure"
        else:
            c = "skipped"
        steps.append({"name": step_name(s), "conclusion": c, "started_at": None if c == "skipped" else stamp(sec) + "Z"})
        if c == "skipped":
            continue
        log.append(f"{stamp(sec)}.1000000Z ##[group]Run {first_line(s)}")
        if j == k:
            for t, line in enumerate(lines):
                log.append(f"{stamp(sec + 1 + min(t, 7))}.0000000Z {line}")
            log.append(f"{stamp(sec + 9)}.0000000Z ##[error]Process completed with exit code 1.")
    steps.append({"name": "Complete job", "conclusion": "success", "started_at": stamp(10 * (len(real) + 1)) + "Z"})
    job = {"id": ids[key], "name": name_of(key, pushed[key]), "status": "completed", "conclusion": conclusion,
           "steps": steps}
    return job, "\n".join(log) + "\n"
def passing():
    return [{"id": ids[k], "name": name_of(k, j), "status": "completed", "conclusion": "success", "steps": []}
            for k, j in pushed.items()]
def run_with(failed):
    """`failed`: key -> (job, log or None). Every other pushed job passed."""
    jobs = [failed[k][0] if k in failed else j for k, j in zip(pushed, passing())]
    logs = {str(failed[k][0]["id"]): failed[k][1] for k in failed if failed[k][1] is not None}
    return jobs, logs
PROCESS_FAIL = "        FAIL [   0.010s] (  1/  9) xtask::plan_check a_test"
"##;

/// Every pushed job failing at each of its steps (a process test's line in that step's part of the log, and again with
/// no log), and ending in each failed state GitHub has, with no step: rows of [job, case, verdict]; and `per-merge`'s
/// own `needs`, `if`, `continue-on-error` and command, the pushed jobs' keys, and the verdict when every job passed.
const EVERY_JOB: &str = r##"
rows = []
for key, job in pushed.items():
    name = name_of(key, job)
    for k, s in enumerate(job.get("steps") or []):
        failed, log = build(key, k, [PROCESS_FAIL])
        for with_log in (True, False):
            jobs, logs = run_with({key: (failed, log if with_log else None)})
            rows.append([name, f"`{step_name(s)}`" + ("" if with_log else ", no log"), m.classify(jobs, logs, process)[0]])
    for c in ("failure", "cancelled", "timed_out", "startup_failure", "action_required", "stale"):
        bare = {"id": ids[key], "name": name, "status": "completed", "conclusion": c, "steps": []}
        jobs, logs = run_with({key: (bare, None)})
        rows.append([name, f"its conclusion {c}", m.classify(jobs, logs, process)[0]])
pm = ci["jobs"]["per-merge"]
needs = pm.get("needs") or []
needs = [needs] if isinstance(needs, str) else needs
print(json.dumps({
    "rows": rows,
    "keys": sorted(pushed),
    "needs": sorted(needs),
    "if": str(pm.get("if", "")),
    "continue": bool(pm.get("continue-on-error")) or any(s.get("continue-on-error") for s in pm.get("steps") or []),
    "runs": " ".join(s.get("run") or "" for s in pm.get("steps") or []),
    "green": m.classify(passing(), {}, process)[0],
}))
"##;

fn check_every_job(prelude: &str, ci: &str) {
    let got = python_with(
        prelude,
        &format!("{JOBS_PY}\n{EVERY_JOB}"),
        &[("ci.yml", ci), ("nextest.toml", &read(CONFIG))],
    );
    let list = |v: &Value| -> Vec<String> {
        v.as_array()
            .expect("qa_ci_per_merge: a list")
            .iter()
            .map(|x| x.as_str().expect("a string").to_owned())
            .collect()
    };
    let keys = list(&got["keys"]);
    let needs = list(&got["needs"]);
    // Not vacuous: the jobs R-982 names are among those a push runs.
    for want in [
        "changes",
        "ci",
        "ci-checks",
        "ci-release",
        "web-skeleton",
        "xtask-ci",
    ] {
        assert!(
            keys.iter().any(|k| k == want),
            "qa_ci_per_merge: `{want}` is not among the jobs a push runs: {keys:?}"
        );
    }
    // `per-merge` reads every job of the run only if it runs after each of them, whatever each ended in, and fails with
    // the verdict.
    let missing: Vec<&String> = keys.iter().filter(|k| !needs.contains(k)).collect();
    assert!(
        missing.is_empty(),
        "qa_ci_per_merge: per-merge does not wait for {missing:?}, so the verdict can be given before they end"
    );
    let cond = got["if"].as_str().unwrap_or("");
    assert!(
        cond.contains("always()"),
        "qa_ci_per_merge: per-merge runs only when every job it needs passed: `{cond}`"
    );
    assert!(
        got["continue"] == Value::Bool(false),
        "qa_ci_per_merge: per-merge carries continue-on-error, so a red verdict passes the run"
    );
    assert!(
        got["runs"]
            .as_str()
            .unwrap_or("")
            .contains("ci_per_merge.py decide"),
        "qa_ci_per_merge: per-merge does not run the verdict's decide"
    );
    assert_eq!(
        got["green"].as_str(),
        Some("green"),
        "qa_ci_per_merge: a run whose every job passed is not green"
    );
    let rows = got["rows"].as_array().expect("qa_ci_per_merge: rows");
    assert!(
        rows.len() >= 300,
        "qa_ci_per_merge: only {} failing cases read",
        rows.len()
    );
    for row in rows {
        let job = row[0].as_str().expect("a job");
        let case = row[1].as_str().expect("a case");
        let verdict = row[2].as_str().expect("a verdict");
        assert!(
            verdict == "red",
            "qa_ci_per_merge: `{job}` failing at {case} reads {verdict}, not red"
        );
    }
}

#[test]
fn qa_per_merge_every_job_of_the_push_run_failing_is_red() {
    check_every_job("", &read(CI));
}

negative_control!(
    qa_per_merge_every_job_of_the_push_run_failing_is_red,
    "a verdict that reads a failure outside correctness as green, the vetoed D-2",
    expected = "reads green, not red",
    check_every_job("m.RED = ('correctness',)", &read(CI))
);

mod per_merge_needs {
    use super::*;
    negative_control!(
        qa_per_merge_every_job_of_the_push_run_failing_is_red,
        "a per-merge job that does not wait for xtask-ci",
        expected = "qa_ci_per_merge: per-merge does not wait for [\"xtask-ci\"]",
        check_every_job(
            "",
            &edit(
                &read(CI),
                "      - gpu-kernel\n      - xtask-ci\n      - gpu-metal\n",
                "      - gpu-kernel\n      - gpu-metal\n"
            )
        )
    );
}

// ---- R-979: the install step's outage is no result ------------------------------------------------------------------

/// The real install step (the step of `ci.yml` whose command calls `apt-get`) run under bash, `sudo` and `timeout`
/// stand-ins passing the command through, and `apt-get` a stand-in whose answer `APT_MODE` sets: the archive hanging
/// (`hang`, as on 11 Nov 2026: apt's `Ign:` line, then cut off), its name not resolving (`resolve`), a package's
/// download refused (`fetch`), a package not found (`package`), `dpkg` failing after a fetch was ignored (`dpkg`), and a
/// network error beside a package not found (`both`). Then the cases: [label, want, verdict, judged lines].
const OUTAGE: &str = r##"
import os, subprocess, tempfile
APT = """#!/bin/bash
case "$APT_MODE:$1" in
  hang:update) echo "Ign:1 http://azure.archive.ubuntu.com/ubuntu noble InRelease"; echo "0% [Connecting to azure.archive.ubuntu.com (52.252.75.106)]"; exit 124;;
  resolve:update) echo "Err:1 http://azure.archive.ubuntu.com/ubuntu noble InRelease"; echo "  Temporary failure resolving 'azure.archive.ubuntu.com'"; exit 100;;
  *:update) echo "Hit:1 http://azure.archive.ubuntu.com/ubuntu noble InRelease"; echo "Reading package lists..."; exit 0;;
  fetch:install) echo "Err:1 http://azure.archive.ubuntu.com/ubuntu noble-updates/main amd64 mesa-vulkan-drivers amd64 25.0.7-0ubuntu0.24.04.2"; echo "  Could not connect to azure.archive.ubuntu.com:80 (52.252.75.106), connection timed out"; echo "E: Failed to fetch http://azure.archive.ubuntu.com/ubuntu/pool/main/m/mesa/mesa-vulkan-drivers_25.0.7_amd64.deb  Could not connect to azure.archive.ubuntu.com:80 (52.252.75.106), connection timed out"; echo "E: Unable to fetch some archives, maybe run apt-get update or try with --fix-missing?"; exit 100;;
  package:install) echo "Reading package lists..."; echo "E: Unable to locate package mesa-vulkan-drivers"; exit 100;;
  dpkg:install) echo "Ign:2 http://azure.archive.ubuntu.com/ubuntu noble-updates/main amd64 libdrm2 amd64 2.4.122"; echo "dpkg: error processing package mesa-vulkan-drivers (--configure):"; echo "E: Sub-process /usr/bin/dpkg returned an error code (1)"; exit 100;;
  both:install) echo "Err:1 http://azure.archive.ubuntu.com/ubuntu noble-updates/main amd64 libllvm19 amd64 1:19.1.1"; echo "  Could not connect to azure.archive.ubuntu.com:80 (52.252.75.106), connection timed out"; echo "E: Unable to locate package mesa-vulkan-drivers"; exit 100;;
  *) exit 0;;
esac
"""
bin_dir = tempfile.mkdtemp(prefix="qa_ci_per_merge_apt_")
for name, text in (("apt-get", APT), ("sudo", "#!/bin/bash\nexec \"$@\"\n"),
                   ("timeout", "#!/bin/bash\nshift 3\nexec \"$@\"\n")):
    path = os.path.join(bin_dir, name)
    open(path, "w").write(text)
    os.chmod(path, 0o755)
def install_at(key):
    return next((k for k, s in enumerate(pushed[key].get("steps") or []) if "apt-get" in (s.get("run") or "")), None)
installs = [k for k in pushed if install_at(k) is not None]
script = pushed[installs[0]]["steps"][install_at(installs[0])]["run"]
outputs = {}
for mode in ("hang", "resolve", "fetch", "package", "dpkg", "both"):
    env = dict(os.environ, PATH=bin_dir + ":" + os.environ.get("PATH", ""), APT_MODE=mode)
    done = subprocess.run(["bash", "-c", script], env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    outputs[mode] = [done.returncode, done.stdout.splitlines()]
cases = []
def case(label, want, failed):
    jobs, logs = run_with(failed)
    verdict, judged = m.classify(jobs, logs, process)
    cases.append([label, want, verdict, [list(j) for j in judged]])
for key in installs:
    for mode, want in (("hang", "no-result"), ("resolve", "no-result"), ("fetch", "no-result"),
                       ("package", "red"), ("dpkg", "red"), ("both", "red")):
        case(f"`{name_of(key, pushed[key])}` failing at its install step ({mode})", want,
             {key: build(key, install_at(key), outputs[mode][1])})
case("every job with the install step failing at it (hang)", "no-result",
     {key: build(key, install_at(key), outputs["hang"][1]) for key in installs})
lints = next(k for k, s in enumerate(pushed["ci-checks"]["steps"]) if "clippy" in (s.get("run") or ""))
for key in installs:
    case(f"`{name_of(key, pushed[key])}`'s outage beside the lints failing", "red",
         {key: build(key, install_at(key), outputs["hang"][1]), "ci-checks": (build("ci-checks", lints, [])[0], None)})
    other = "ci-workspace" if key != "ci-workspace" else "ci"
    test = next(k for k, s in enumerate(pushed[other]["steps"]) if (s.get("run") or "").startswith("cargo nextest run"))
    case(f"`{name_of(key, pushed[key])}`'s outage beside a process test failing in `{other}`", "red",
         {key: build(key, install_at(key), outputs["hang"][1]), other: build(other, test, [PROCESS_FAIL])})
print(json.dumps({"installs": installs, "outputs": outputs, "cases": cases}))
"##;

fn check_outage(prelude: &str) {
    let got = python_with(
        prelude,
        &format!("{JOBS_PY}\n{OUTAGE}"),
        &[("ci.yml", &read(CI)), ("nextest.toml", &read(CONFIG))],
    );
    let installs = got["installs"]
        .as_array()
        .expect("qa_ci_per_merge: installs");
    // Not vacuous: the release shards, the `ci` shards, `xtask-ci`, the GPU suites and `ci-workspace` install Mesa.
    assert!(
        installs.len() >= 6,
        "qa_ci_per_merge: only {installs:?} install Mesa"
    );
    // The step really failed, on `bounded`'s last try, in every mode.
    for (mode, out) in got["outputs"]
        .as_object()
        .expect("qa_ci_per_merge: outputs")
    {
        let lines: Vec<&str> = out[1]
            .as_array()
            .expect("lines")
            .iter()
            .filter_map(Value::as_str)
            .collect();
        assert!(
            out[0].as_i64() != Some(0) && lines.iter().any(|l| l.starts_with("attempt 3 of 3 failed")),
            "qa_ci_per_merge: the install step under `{mode}` did not fail on its last try: {lines:?}"
        );
    }
    for case in got["cases"].as_array().expect("qa_ci_per_merge: cases") {
        let label = case[0].as_str().expect("a label");
        let want = case[1].as_str().expect("a want");
        let verdict = case[2].as_str().expect("a verdict");
        assert!(
            verdict == want,
            "qa_ci_per_merge: {label} reads {verdict}, not {want}: {}",
            case[3]
        );
        if want == "no-result" {
            for judged in case[3].as_array().expect("judged") {
                let why = judged[2].as_str().unwrap_or("");
                assert!(
                    why.contains("Install Mesa (lavapipe)"),
                    "qa_ci_per_merge: {label}: the line does not name the step: {why}"
                );
            }
        }
    }
}

#[test]
fn qa_per_merge_the_install_steps_outage_is_no_result() {
    check_outage("");
}

negative_control!(
    qa_per_merge_the_install_steps_outage_is_no_result,
    "a verdict that reads an install outage as green",
    expected = "reads green, not no-result",
    check_outage("m.NO_RESULT = 'green'")
);

mod outage_hides_a_failure {
    use super::*;
    negative_control!(
        qa_per_merge_the_install_steps_outage_is_no_result,
        "a verdict that lets an install outage hide another failure, reading the run as no result",
        expected = "reads no-result, not red",
        check_outage(
            "_classify = m.classify\n\
             def _hiding(jobs, logs, process):\n    \
                 verdict, judged = _classify(jobs, logs, process)\n    \
                 if verdict == 'red' and any(k == 'no-result' for _, k, _ in judged):\n        \
                     verdict = 'no-result'\n    \
                 return verdict, judged\n\
             m.classify = _hiding"
        )
    );
}

// ---- `decide`: fails unless green, files nothing ---------------------------------------------------------------------

/// `decide` run whole against a stand-in `gh` that serves the run's jobs (in two pages) and each job's log, and records
/// every call: for each case, its exit status, what it printed, its step summary, and the calls.
const DECIDE: &str = r##"
import contextlib, io, os, tempfile
GH = """#!/bin/bash
echo "$*" >> "$STUB/calls"
args="$*"
case "$args" in
  *"/attempts/"*"/jobs"*) [ -f "$STUB/pages" ] && cat "$STUB/pages" && exit 0;;
  *"/logs") id="${args##*jobs/}"; id="${id%/logs}"; [ -f "$STUB/logs/$id.log" ] && cat "$STUB/logs/$id.log" && exit 0;;
esac
echo "gh: Not Found (HTTP 404)" >&2
exit 1
"""
own = {"id": 999, "name": "per-merge", "status": "in_progress", "conclusion": None, "steps": []}
def decide(jobs, logs, listed=True):
    d = tempfile.mkdtemp(prefix="qa_ci_per_merge_decide_")
    os.makedirs(os.path.join(d, "bin"))
    os.makedirs(os.path.join(d, "logs"))
    open(os.path.join(d, "bin", "gh"), "w").write(GH)
    os.chmod(os.path.join(d, "bin", "gh"), 0o755)
    if listed:
        half = len(jobs) // 2
        open(os.path.join(d, "pages"), "w").write(json.dumps({"jobs": jobs[:half]}) + "\n" + json.dumps({"jobs": jobs[half:]}) + "\n")
    for jid, text in logs.items():
        open(os.path.join(d, "logs", f"{jid}.log"), "w").write(text)
    os.environ.update(PATH=os.path.join(d, "bin") + ":" + os.environ.get("PATH", ""), STUB=d, GH_REPO="o/r",
                      RUN_ID="42", RUN_ATTEMPT="1", GITHUB_STEP_SUMMARY=os.path.join(d, "summary"), CONFIG=sys.argv[3])
    os.environ.pop("SELF", None)
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        status = m.main(["ci_per_merge.py", "decide"])
    read = lambda f: open(os.path.join(d, f)).read() if os.path.exists(os.path.join(d, f)) else ""
    return {"status": status, "out": buf.getvalue(), "summary": read("summary"), "calls": read("calls").splitlines()}
test = next(k for k, s in enumerate(pushed["ci"]["steps"]) if (s.get("run") or "").startswith("cargo nextest run"))
failed, log = build("ci", test, [PROCESS_FAIL])
jobs, logs = run_with({"ci": (failed, log)})
out = {"process": decide(jobs + [own], logs)}
lints = next(k for k, s in enumerate(pushed["ci-checks"]["steps"]) if "clippy" in (s.get("run") or ""))
jobs, logs = run_with({"ci-checks": (build("ci-checks", lints, [])[0], None)})
out["no log"] = decide(jobs + [own], logs)
outage = ["Ign:1 http://azure.archive.ubuntu.com/ubuntu noble InRelease",
          "attempt 1 of 3 failed or ran past 90 s, apt-get update",
          "Ign:1 http://azure.archive.ubuntu.com/ubuntu noble InRelease",
          "attempt 2 of 3 failed or ran past 90 s, apt-get update",
          "Ign:1 http://azure.archive.ubuntu.com/ubuntu noble InRelease",
          "attempt 3 of 3 failed or ran past 90 s, apt-get update"]
at = next(k for k, s in enumerate(pushed["ci-release"]["steps"]) if "apt-get" in (s.get("run") or ""))
jobs, logs = run_with({"ci-release": build("ci-release", at, outage)})
out["outage"] = decide(jobs + [own], logs)
out["green"] = decide(passing() + [own], {})
out["unread"] = decide(passing(), {}, listed=False)
print(json.dumps(out))
"##;

fn check_decide(prelude: &str) {
    let got = python_with(
        prelude,
        &format!("{JOBS_PY}\n{DECIDE}"),
        &[("ci.yml", &read(CI)), ("nextest.toml", &read(CONFIG))],
    );
    let status = |case: &str| got[case]["status"].as_i64().unwrap_or(-1);
    let first = |case: &str| -> String {
        got[case]["out"]
            .as_str()
            .unwrap_or("")
            .lines()
            .next()
            .unwrap_or("")
            .to_owned()
    };
    assert!(
        status("process") != 0,
        "qa_ci_per_merge: decide passed a run whose process test failed: {}",
        got["process"]
    );
    assert_eq!(
        first("process"),
        "verdict=red",
        "qa_ci_per_merge: decide on a run whose process test failed"
    );
    assert!(
        status("no log") != 0 && first("no log") == "verdict=red",
        "qa_ci_per_merge: decide passed the lints failing with no log to read: {}",
        got["no log"]
    );
    assert!(
        status("outage") != 0 && first("outage") == "verdict=no-result",
        "qa_ci_per_merge: decide on an install outage is not a failing no result: {}",
        got["outage"]
    );
    assert!(
        got["outage"]["summary"]
            .as_str()
            .unwrap_or("")
            .to_lowercase()
            .contains("no result"),
        "qa_ci_per_merge: decide's summary does not say the run has no result: {}",
        got["outage"]["summary"]
    );
    assert!(
        status("unread") != 0,
        "qa_ci_per_merge: decide passed a run whose jobs it could not read"
    );
    // Not vacuous: a green run, its own job still running, passes.
    assert!(
        status("green") == 0 && first("green") == "verdict=green",
        "qa_ci_per_merge: decide does not pass a run whose every job passed, its own job aside: {}",
        got["green"]
    );
    for case in ["process", "no log", "outage", "green", "unread"] {
        for call in got[case]["calls"].as_array().expect("calls") {
            let call = call.as_str().unwrap_or("");
            assert!(
                !call.contains("--method")
                    && !call.contains("/issues")
                    && !call.contains("/labels")
                    && !call.contains(" -f "),
                "qa_ci_per_merge: decide ({case}) asks gh for more than a read: {call}"
            );
        }
    }
}

#[test]
fn qa_per_merge_decide_fails_unless_green_and_files_nothing() {
    check_decide("");
}

negative_control!(
    qa_per_merge_decide_fails_unless_green_and_files_nothing,
    "a verdict that reads a failure outside correctness as green, the vetoed D-2",
    expected = "qa_ci_per_merge: decide passed a run whose process test failed",
    check_decide("m.RED = ('correctness',)")
);
