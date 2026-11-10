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
//! Each has its registered negative control (R-176): the same check on a `ci.yml` whose golden steps are renamed, and on
//! a configuration whose process profile names a `lint_wgsl` binary. Unix only, as python3 runs the script.
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
         spec.loader.exec_module(m)\n{program}"
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
