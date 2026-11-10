//! qa's tests for the per-merge verdict (`.github/scripts/ci_per_merge.py`, `ci.yml`'s `per-merge` job), written from
//! the rulings, not the script: R-907, "a correctness failure (physics, validation, bit-for-bit, golden) is red main
//! under R-681; any other failure files a top-priority task"; R-907 item 1, the product-tooling tests (`build_kernel`,
//! `lint_wgsl_*`) are product tests, not process ones; R-909, the golden runner over every suite
//! (`qa_TASK-M0-06_runner`, `_list`) and the screenshot runner's self-test (`qa_TASK-M0-20`) are product correctness,
//! run per merge; and `plan/OPERATIONS.md` § "Merging", what is red `main`: only the lints and build beside the tests,
//! `cargo xtask ci`, the web skeleton, the classification, a runner or setup step, or a process test fail as a task.
//!
//! - `qa_per_merge_every_test_step_on_a_push_is_read_as_one`: over the real `ci.yml`, each step of each job a push runs
//!   that runs tests (`cargo nextest run`, `cargo test`, `cargo xtask golden`, `gate` or `screenshot`), named as GitHub
//!   names it (a step without a `name:` is `Run <its first line>`), failing with a product test in its log, gives the
//!   verdict red, unless its job is one OPERATIONS puts beside the tests. A step renamed so the verdict no longer reads
//!   it as a test step would let a golden or GPU failure through as a task.
//! - `qa_per_merge_product_tooling_and_golden_failures_are_red`: against the real `.config/nextest.toml`, each
//!   product-tooling binary and R-909's named product correctness, failing alone in a `ci` shard, is red, and a process
//!   test (`plan_check`, R-907 item 1's list) failing alone is a task.
//!
//! - `qa_per_merge_each_failed_step_is_judged_by_its_own_log`: a `ci-workspace` run whose nextest step fails a process
//!   test and whose doctest step, later in the same log, fails a doctest (which names no nextest test) is red: the
//!   doctest is a product test, and the process test's line elsewhere in the job's log must not make it a task.
//! - `qa_per_merge_a_part_run_suite_is_red`: a `ci` shard whose nextest summary counts fewer tests run than it
//!   selected is red even when the only failure it names is a process test's: product tests went unrun (R-909).
//! - `qa_per_merge_no_skipped_product_step_reads_task`: over the real `ci.yml`, each test step of each job a push runs,
//!   failing with a process test alone, and each later step GitHub's `if:` rules would then skip, skipped: the verdict is
//!   never a task while a skipped step runs product tests (anything but `cargo nextest run -p <pkg> --test <process
//!   binary>…`); and in `ci-workspace` and `gpu-kernel`, whose suites R-909 has run whole, a process failure is a task.
//! - `qa_per_merge_product_profiles_run_on_after_a_failure`: the `ci`, `ci-workspace` and `gpu-kernel` nextest
//!   profiles run every selected test after a failure (`fail-fast` false, as set or inherited).
//!
//! Each has its registered negative control (R-176): the same check on a `ci.yml` whose golden steps are renamed; on
//! a configuration whose process profile names a `lint_wgsl` binary; on a script that gives every failed step the
//! whole job's log; on one that never reads the summary's unrun count; on one that takes every skipped step as process
//! tests alone; and on a configuration whose `ci-workspace` profile stops at the first failure. Unix only, as python3
//! runs the script.
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use validation::negative_control;

const SCRIPT: &str = ".github/scripts/ci_per_merge.py";
const CONFIG: &str = ".config/nextest.toml";
const CI: &str = ".github/workflows/ci.yml";

/// The jobs OPERATIONS § "Merging" puts outside correctness whatever fails in them: the classification (`changes`),
/// the lints and build beside the tests (`ci-checks`), `cargo xtask ci` (`xtask-ci`) and the web skeleton.
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
        verdict, _ = m.classify(jobs, {"7": log}, process)
        out.append([key, name, step_name, verdict, run.strip().splitlines()[0]])
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
        let want = if BESIDE.contains(&key) { "task" } else { "red" };
        assert!(
            verdict == want,
            "qa_ci_per_merge: a product test failing in step `{step}` of `{name}` reads {verdict}, not {want}"
        );
    }
}

#[test]
fn qa_per_merge_every_test_step_on_a_push_is_read_as_one() {
    check_steps(&read(CI));
}

negative_control!(
    qa_per_merge_every_test_step_on_a_push_is_read_as_one,
    "the golden runner's steps renamed so the verdict does not read them as test steps",
    expected = "qa_ci_per_merge: a product test failing in step `The golden runner over every suite` of `gpu-metal suites` reads task, not red",
    check_steps(&edit(
        &read(CI),
        "- name: cargo xtask golden --all\n",
        "- name: The golden runner over every suite\n"
    ))
);

// ---- Product tooling and golden failures ----------------------------------------------------------------------------

/// For each binary, one a line in the file `sys.argv[3]`: the verdict on a run whose `ci (1)` shard failed its nextest
/// step with that binary's test alone.
const ALONE: &str = r#"
process = m.process_binaries(open(sys.argv[2]).read())
out = {}
for binary in open(sys.argv[3]).read().split("\n"):
    jobs = [{"id": 9, "name": "ci (1)", "conclusion": "failure",
             "steps": [{"name": "cargo nextest run --archive-file … --partition hash:1/4", "conclusion": "failure"}]}]
    log = f"        FAIL [   1.250s] ( 12/5514) {binary} a_test\n"
    out[binary] = m.classify(jobs, {"9": log}, process)[0]
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

fn check_alone(config: &str) {
    let product = product_binaries();
    let process = "xtask::plan_check";
    let names: Vec<String> = product
        .iter()
        .cloned()
        .chain([process.to_owned()])
        .collect();
    let got = python(
        ALONE,
        &[("nextest.toml", config), ("binaries", &names.join("\n"))],
    );
    for binary in &product {
        let verdict = got[binary.as_str()].as_str().expect("a verdict");
        assert!(
            verdict == "red",
            "qa_ci_per_merge: {binary} failing reads {verdict}, not red"
        );
    }
    assert_eq!(
        got[process].as_str(),
        Some("task"),
        "qa_ci_per_merge: a process test failing alone is not a task"
    );
}

#[test]
fn qa_per_merge_product_tooling_and_golden_failures_are_red() {
    check_alone(&read(CONFIG));
}

negative_control!(
    qa_per_merge_product_tooling_and_golden_failures_are_red,
    "a process profile that names a lint_wgsl binary",
    expected = "qa_ci_per_merge: xtask::lint_wgsl_float failing reads task, not red",
    check_alone(&edit(
        &read(CONFIG),
        "        | binary_id(=xtask::plan_check)\n",
        "        | binary_id(=xtask::plan_check)\n        | binary_id(=xtask::lint_wgsl_float)\n"
    ))
);

// ---- Each failed step judged by its own part of the log --------------------------------------------------------------

/// A `ci-workspace` run: its nextest step fails `xtask::plan_check`, a process test, and its doctest step, minutes later
/// in the same log, ends as `DOC` says (`success` or `failure`). The verdict.
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
print(json.dumps(m.classify(jobs, {"11": "\n".join(lines) + "\n"}, m.process_binaries(open(sys.argv[2]).read()))[0]))
"#;

fn check_own_log(prelude: &str) {
    let config = read(CONFIG);
    let run = |doc: &str| -> String {
        let program = OWN_LOG.replacen("DOC", &format!("{doc:?}"), 1);
        python_with(prelude, &program, &[("nextest.toml", &config)])
            .as_str()
            .expect("a verdict")
            .to_owned()
    };
    // Not vacuous: with the doctests passing, the process test's failure alone is a task (OPERATIONS § "Merging").
    let passed = run("success");
    assert!(
        passed == "task",
        "qa_ci_per_merge: a process test failing alone in ci-workspace reads {passed}, not task"
    );
    let failed = run("failure");
    assert!(
        failed == "red",
        "qa_ci_per_merge: a doctest failing after a process test in ci-workspace reads {failed}, not red"
    );
}

#[test]
fn qa_per_merge_each_failed_step_is_judged_by_its_own_log() {
    check_own_log("");
}

negative_control!(
    qa_per_merge_each_failed_step_is_judged_by_its_own_log,
    "a script that gives every failed step the whole job's log",
    expected = "qa_ci_per_merge: a doctest failing after a process test in ci-workspace reads task, not red",
    check_own_log("m.parts = lambda steps, log: {i: m.clean(log) for i in range(len(steps))}")
);

// ---- A part-run suite ------------------------------------------------------------------------------------------------

/// A `ci (2)` shard failing its nextest step with `xtask::plan_check` alone, its summary line `SUMMARY`. The verdict.
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
print(json.dumps(m.classify(jobs, {"12": log}, m.process_binaries(open(sys.argv[2]).read()))[0]))
"#;

fn check_part_run(prelude: &str) {
    let config = read(CONFIG);
    let run = |summary: &str| -> String {
        python_with(
            prelude,
            &PART_RUN.replacen("SUMMARY", summary, 1),
            &[("nextest.toml", &config)],
        )
        .as_str()
        .expect("a verdict")
        .to_owned()
    };
    // Not vacuous: every selected test run, the process test's failure alone is a task.
    let whole = run("Summary [  71.045s] 5514 tests run: 5513 passed, 1 failed");
    assert!(
        whole == "task",
        "qa_ci_per_merge: a whole run failing a process test reads {whole}, not task"
    );
    let part = run("Summary [  71.045s] 322/5514 tests run: 321 passed, 1 failed");
    assert!(
        part == "red",
        "qa_ci_per_merge: a run that left 5192 tests unrun reads {part}, not red"
    );
}

#[test]
fn qa_per_merge_a_part_run_suite_is_red() {
    check_part_run("");
}

negative_control!(
    qa_per_merge_a_part_run_suite_is_red,
    "a script that never reads the summary's count of tests unrun",
    expected = "qa_ci_per_merge: a run that left 5192 tests unrun reads task, not red",
    check_part_run("m.unrun = lambda lines: None")
);

// ---- Skipped product steps -------------------------------------------------------------------------------------------

/// For each test step of each job a push runs: that step failing with a process test alone, the steps before it passed,
/// each after it skipped unless its `if:` runs it after a failure (`always()`, `!cancelled()`, `failure()`), as GitHub
/// does. Rows of [job key, job name, failed step, verdict, skipped steps running product tests, every skipped test step].
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
        verdict = m.classify(jobs, {"7": log}, process)[0]
        hidden = [n for n, b in skipped if b is None or any(x not in process for x in b)]
        out.append([key, name, listed[i]["name"], verdict, hidden, [n for n, _ in skipped]])
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
        if !hidden.is_empty() {
            hiding += 1;
            assert!(
                verdict == "red",
                "qa_ci_per_merge: `{step}` of `{name}` failing with a process test leaves {} unrun, and reads {verdict}, not red",
                hidden[0]
            );
        }
        // R-909's two suites run whole after a process test fails: a task, though process-only steps are skipped.
        if (key == "ci-workspace" || key == "gpu-kernel")
            && step.starts_with("cargo nextest run -p ")
        {
            assert!(
                verdict == "task" && hidden.is_empty(),
                "qa_ci_per_merge: `{step}` of `{name}` failing with a process test reads {verdict}, skipping {skipped:?}"
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
