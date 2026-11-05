//! The per-mutant timeout from one measured baseline (the owner's order of 5 Nov 2026, R-712, on the port; R-348,
//! REQ-VAL-180 as it amends them; R-202, R-302): `mutants-baseline` measures the unmutated baseline once, in cargo-mutants'
//! own run, on the shards' setup, and fails the gate if it fails; `cargo xtask mutants-timeout` applies
//! `.cargo/mutants.toml`'s multiplier and floor to it, and the eight shards, their baselines skipped, take the result; a
//! timed-out mutant stays no survivor, and every one is listed, across every shard, with a count, 0 included.
//!
//! The workflow's shell steps run here in bash, with a stand-in `cargo` for `cargo mutants` and the built xtask for
//! `cargo xtask`, as `qa_TASK-M0-23_shards.rs` runs them. `mutants-fixture` runs the same commands on the real
//! cargo-mutants.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};
use validation::negative_control;
use validation::spawn::{write_executable, Spawn};
use xtask::mutants_check::{baseline_seconds, per_mutant_timeout, timeout_rule, EQUIVALENT_LIST};

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn mutants_yml() -> String {
    read(".github/workflows/mutants.yml")
}

/// `yml` with `from`, which must be in it, replaced by `to`: a control's one edit.
#[cfg(feature = "controls")]
fn edited(yml: &str, from: &str, to: &str) -> String {
    assert!(
        yml.contains(from),
        "the control's edit target is gone: {from}"
    );
    yml.replace(from, to)
}

/// `yml` as JSON, read by PyYAML, which `xtask/tests` already need (`merge_ready.rs`).
fn parse(yml: &str) -> Value {
    let dir = Scratch::new("mutants_timeout_yaml");
    std::fs::create_dir_all(&dir).expect("yaml dir");
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
        .expect("python3 ran");
    assert!(
        out.status.success(),
        "the workflow is not YAML: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("JSON from python3")
}

fn text(value: &Value) -> &str {
    value.as_str().unwrap_or_default()
}

/// The step of `job` named `name`.
fn step<'a>(job: &'a Value, name: &str) -> &'a Value {
    job["steps"]
        .as_array()
        .expect("the job's steps")
        .iter()
        .find(|s| text(&s["name"]).starts_with(name))
        .unwrap_or_else(|| panic!("no step `{name}`"))
}

// ---------------------------------------------------------------------------------------------------------------
// The rule: cargo-mutants' own, the file's multiplier and floor.

type Rule = fn(f64, f64, f64) -> f64;

/// `rule` is cargo-mutants' rule at the floor's and the rounding's boundaries, and on PR #360's measured baseline.
fn timeout_rule_holds(rule: Rule) {
    for (baseline, want, why) in [
        (0.0, 60.0, "no baseline: the floor"),
        (19.9, 60.0, "under the floor"),
        (20.0, 60.0, "exactly on the floor"),
        (20.01, 61.0, "just over the floor, rounded up"),
        (21.0, 63.0, "over the floor, a whole number"),
        (5103.469501362, 15311.0, "shard 0's baseline on PR #360"),
    ] {
        assert_eq!(
            rule(baseline, 3.0, 60.0),
            want,
            "the per-mutant timeout is not 3 times {baseline} s, rounded up, floored at 60 s: {why}"
        );
    }
}

#[test]
fn mutants_timeout_is_cargo_mutants_rule() {
    timeout_rule_holds(per_mutant_timeout);
}

negative_control!(
    mutants_timeout_is_cargo_mutants_rule,
    "no floor: a fast baseline would time out its own mutants",
    expected = "the per-mutant timeout is not 3 times",
    timeout_rule_holds(|baseline, multiplier, _| (baseline * multiplier).ceil())
);

mod not_rounded_up {
    use super::*;
    negative_control!(
        mutants_timeout_is_cargo_mutants_rule,
        "not rounded up: cargo-mutants rounds up",
        expected = "the per-mutant timeout is not 3 times",
        timeout_rule_holds(|baseline, multiplier, floor| (baseline * multiplier).max(floor))
    );
}

/// The one place's values: a multiple of 3 of the measured baseline, with the floor the caps' fixture stands on.
fn rule_is_the_orders(toml: &str) {
    let rule = timeout_rule(toml).unwrap_or_else(|e| panic!("no rule: {e}"));
    assert_eq!(
        rule,
        (3.0, 60.0),
        "not the owner's multiplier of 3 and the floor of 60 s"
    );
}

#[test]
fn mutants_timeout_rule_is_read_from_the_config() {
    rule_is_the_orders(&read(".cargo/mutants.toml"));
    rule_is_the_orders("timeout_multiplier = 3\nminimum_test_timeout = 60.0\n");
}

negative_control!(
    mutants_timeout_rule_is_read_from_the_config,
    "R-376's multiplier, superseded",
    expected = "not the owner's multiplier of 3",
    rule_is_the_orders(&edited(
        &read(".cargo/mutants.toml"),
        "timeout_multiplier = 3.0",
        "timeout_multiplier = 2.0"
    ))
);

fn refuses_rule(toml: &str, naming: &str) {
    let got = timeout_rule(toml);
    assert!(
        got.as_ref().is_err_and(|e| e.contains(naming)),
        "a rule without {naming} was read: {got:?}"
    );
}

#[test]
fn mutants_timeout_rule_refuses_a_config_without_it() {
    refuses_rule(
        "minimum_test_timeout = 60\n",
        "no number `timeout_multiplier`",
    );
    refuses_rule(
        "timeout_multiplier = 3.0\n",
        "no number `minimum_test_timeout`",
    );
    refuses_rule(
        "timeout_multiplier = \"3\"\nminimum_test_timeout = 60\n",
        "no number `timeout_multiplier`",
    );
    refuses_rule("timeout_multiplier = [", "is not TOML");
}

negative_control!(
    mutants_timeout_rule_refuses_a_config_without_it,
    "a config that has both",
    expected = "was read",
    refuses_rule(
        "timeout_multiplier = 3.0\nminimum_test_timeout = 60\n",
        "no number `timeout_multiplier`"
    )
);

// ---------------------------------------------------------------------------------------------------------------
// The measurement: the baseline's Test phase, from cargo-mutants' own report.

/// An `outcomes.json` in cargo-mutants' schema: a baseline of `summary` with these build and test times, then the
/// stopped mutants the baseline run records.
fn baseline_run(summary: &str, build: f64, test: f64) -> String {
    json!({"outcomes": [
        {"scenario": "Baseline", "summary": summary, "phase_results": [
            {"phase": "Build", "duration": build},
            {"phase": "Test", "duration": test},
        ]},
        {"scenario": {"Mutant": {"name": "a.rs:1:1: replace f"}}, "summary": "Timeout", "phase_results": [
            {"phase": "Build", "duration": 0.001},
        ]},
    ]})
    .to_string()
}

fn measures(json: &str, want: f64) {
    let got = baseline_seconds(json).unwrap_or_else(|e| panic!("not measured: {e}"));
    assert_eq!(got, want, "not the baseline's Test phase");
}

#[test]
fn mutants_timeout_measures_the_test_phase() {
    measures(&baseline_run("Success", 172.0, 5103.5), 5103.5);
}

negative_control!(
    mutants_timeout_measures_the_test_phase,
    "the build's time in the test's place",
    expected = "not the baseline's Test phase",
    measures(&baseline_run("Success", 5103.5, 172.0), 5103.5)
);

fn refuses_baseline(json: &str, naming: &str) {
    let got = baseline_seconds(json);
    assert!(
        got.as_ref().is_err_and(|e| e.contains(naming)),
        "a run that measured nothing was read: {got:?}"
    );
}

#[test]
fn mutants_timeout_refuses_a_run_that_measured_nothing() {
    refuses_baseline(&baseline_run("Failure", 1.0, 2.0), "did not pass (Failure)");
    refuses_baseline(
        &json!({"outcomes": [{"scenario": {"Mutant": {"name": "m"}}, "summary": "Timeout"}]})
            .to_string(),
        "no unmutated baseline",
    );
    refuses_baseline(
        &json!({"outcomes": [{"scenario": "Baseline", "summary": "Success", "phase_results": [
            {"phase": "Build", "duration": 1.0}]}]})
        .to_string(),
        "no timed Test phase",
    );
    refuses_baseline(
        &json!({"outcomes": [{"scenario": "Baseline", "summary": "Success"}]}).to_string(),
        "no timed Test phase",
    );
    refuses_baseline("{}", "no `outcomes` array");
    refuses_baseline("not json", "is not JSON");
}

negative_control!(
    mutants_timeout_refuses_a_run_that_measured_nothing,
    "a baseline that passed",
    expected = "a run that measured nothing was read",
    refuses_baseline(&baseline_run("Success", 1.0, 2.0), "did not pass")
);

/// `cargo xtask <args>`: whether it passed, its stdout and its stderr.
fn xtask(args: &[&Path]) -> (bool, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(args)
        .timed_output()
        .expect("xtask ran");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// `cargo xtask mutants-timeout` on a run holding `outcomes`.
fn timeout_of(outcomes: &str) -> (bool, String, String) {
    let dir = Scratch::new("mutants_timeout_run");
    std::fs::create_dir_all(&dir).expect("run dir");
    std::fs::write(dir.join("outcomes.json"), outcomes).expect("outcomes written");
    xtask(&[Path::new("mutants-timeout"), &dir])
}

fn prints_key_values(outcomes: &str, want: &str) {
    let (ok, out, err) = timeout_of(outcomes);
    assert!(ok && out == want, "mutants-timeout printed {out:?}: {err}");
}

#[test]
fn mutants_timeout_binary_prints_the_outputs() {
    prints_key_values(
        &baseline_run("Success", 172.0, 5103.4695),
        "baseline-seconds=5103.5\ntimeout=15311\n",
    );
    prints_key_values(
        &baseline_run("Success", 0.9, 0.1),
        "baseline-seconds=0.1\ntimeout=60\n",
    );
}

negative_control!(
    mutants_timeout_binary_prints_the_outputs,
    "a run whose baseline took a different time",
    expected = "mutants-timeout printed",
    prints_key_values(
        &baseline_run("Success", 172.0, 5000.0),
        "baseline-seconds=5103.5\ntimeout=15311\n"
    )
);

fn no_mutant_is_the_floor(out: &str) {
    assert_eq!(
        out, "baseline-seconds=0.0\ntimeout=60\n",
        "a diff with no mutant does not get the floor"
    );
}

#[test]
fn mutants_timeout_binary_gives_a_diff_with_no_mutant_the_floor() {
    let (ok, out, err) = xtask(&[Path::new("mutants-timeout"), Path::new("--no-mutant")]);
    assert!(ok, "--no-mutant failed: {err}");
    no_mutant_is_the_floor(&out);
}

negative_control!(
    mutants_timeout_binary_gives_a_diff_with_no_mutant_the_floor,
    "a measured run's output",
    expected = "a diff with no mutant does not get the floor",
    no_mutant_is_the_floor("baseline-seconds=30.0\ntimeout=90\n")
);

fn publishes_no_timeout(outcomes: &str, naming: &str) {
    let (ok, out, err) = timeout_of(outcomes);
    assert!(
        !ok && !out.contains("timeout=") && err.contains(naming),
        "a failed measurement published a timeout: {out:?} {err}"
    );
}

#[test]
fn mutants_timeout_binary_fails_a_failed_baseline() {
    publishes_no_timeout(
        &baseline_run("Failure", 1.0, 2.0),
        "the unmutated baseline did not pass",
    );
}

negative_control!(
    mutants_timeout_binary_fails_a_failed_baseline,
    "a baseline that passed",
    expected = "a failed measurement published a timeout",
    publishes_no_timeout(
        &baseline_run("Success", 1.0, 2.0),
        "the unmutated baseline did not pass"
    )
);

// ---------------------------------------------------------------------------------------------------------------
// The wiring: one baseline before the shards, no new required check.

const MERGE_READY: &str =
    "github.event_name == 'pull_request' && github.event.action == 'labeled' && github.event.label.name == 'merge-ready'";

fn baseline_is_wired(yml: &str) {
    let wf = parse(yml);
    let jobs = &wf["jobs"];
    let baseline = &jobs["mutants-baseline"];
    let shards = &jobs["mutants"];
    assert_eq!(
        text(&baseline["if"]),
        MERGE_READY,
        "the baseline does not run on exactly the merge-ready run"
    );
    assert_eq!(
        text(&shards["needs"]),
        "mutants-baseline",
        "the shards do not wait for the baseline"
    );
    assert!(
        baseline.get("needs").is_none(),
        "the baseline waits for another job"
    );
    assert_eq!(
        shards["strategy"]["matrix"]["shard"],
        json!([0, 1, 2, 3, 4, 5, 6, 7]),
        "not 8 shards"
    );
    // `mutants-check` stays the one job gathering the run; its chain reaches the baseline through the shards, so the
    // shards' `skipped` fails it.
    assert_eq!(
        text(&jobs["mutants-verdict"]["needs"]),
        "mutants",
        "the verdict no longer reads the shards' result"
    );
    assert_eq!(
        text(&jobs["mutants-check"]["needs"]),
        "mutants-verdict",
        "mutants-check no longer reports the verdict"
    );
    // The outputs the shards read come from the step that computed them.
    let outputs = &baseline["outputs"];
    assert_eq!(
        text(&outputs["timeout"]),
        "${{ steps.timeout.outputs.timeout }}",
        "the baseline does not publish the timeout"
    );
    assert_eq!(
        text(&outputs["baseline-seconds"]),
        "${{ steps.timeout.outputs.baseline-seconds }}",
        "the baseline does not publish the measured seconds"
    );
    let run = step(shards, "cargo mutants --in-diff --shard");
    assert_eq!(
        text(&run["env"]["MUTANT_TIMEOUT"]),
        "${{ needs.mutants-baseline.outputs.timeout }}",
        "a shard does not read the measured timeout"
    );
    let script = text(&run["run"]);
    assert!(
        script.contains("--baseline=skip --timeout \"$MUTANT_TIMEOUT\""),
        "a shard runs a baseline of its own, or not the measured timeout: {script}"
    );
    assert_eq!(
        run["timeout-minutes"],
        json!(300),
        "a shard's limit is not REQ-VAL-149's 300 minutes"
    );
}

#[test]
fn mutants_baseline_runs_once_before_the_shards() {
    baseline_is_wired(&mutants_yml());
}

negative_control!(
    mutants_baseline_runs_once_before_the_shards,
    "the shards not waiting for it, so the timeout reads empty",
    expected = "the shards do not wait for the baseline",
    baseline_is_wired(&edited(&mutants_yml(), "    needs: mutants-baseline\n", ""))
);

mod own_baseline {
    use super::*;
    negative_control!(
        mutants_baseline_runs_once_before_the_shards,
        "a shard with its baseline back",
        expected = "a shard runs a baseline of its own",
        baseline_is_wired(&edited(
            &mutants_yml(),
            "--baseline=skip --timeout \"$MUTANT_TIMEOUT\" \\\n            --shard",
            "--timeout \"$MUTANT_TIMEOUT\" \\\n            --shard"
        ))
    );
}

mod every_label {
    use super::*;
    negative_control!(
        mutants_baseline_runs_once_before_the_shards,
        "the baseline on every label",
        expected = "the baseline does not run on exactly the merge-ready run",
        baseline_is_wired(&edited(
            &mutants_yml(),
            "  mutants-baseline:\n    if: github.event_name == 'pull_request' && github.event.action == 'labeled' && github.event.label.name == 'merge-ready'\n",
            "  mutants-baseline:\n    if: github.event_name == 'pull_request' && github.event.action == 'labeled'\n"
        ))
    );
}

/// What the measured run is run on: every step up to it is the shard's own, and it has the shard's environment and
/// containment, and the workflow's package selection, cargo-mutants' `--in-diff`.
fn baseline_matches_a_shard(yml: &str) {
    let wf = parse(yml);
    let (baseline, shards) = (&wf["jobs"]["mutants-baseline"], &wf["jobs"]["mutants"]);
    assert_eq!(
        baseline["env"], shards["env"],
        "the baseline's job environment is not the shards'"
    );
    let steps = baseline["steps"].as_array().expect("the baseline's steps");
    let at = steps
        .iter()
        .position(|s| text(&s["id"]) == "baseline")
        .expect("the measured step");
    assert_eq!(
        steps[..at],
        shards["steps"].as_array().expect("the shards' steps")[..at],
        "the baseline's setup is not the shards'"
    );
    let measured = &steps[at];
    let run = step(shards, "cargo mutants --in-diff --shard");
    for key in ["shell", "if"] {
        assert_eq!(
            measured[key], run[key],
            "the measured step's `{key}` is not a shard's"
        );
    }
    for var in ["RUSTFLAGS", "PRIN_CONTAIN", "HEARTBEAT_OUT"] {
        assert_eq!(
            measured["env"][var], run["env"][var],
            "the measured step's {var} is not a shard's"
        );
    }
    let script = text(&measured["run"]);
    assert!(
        script.contains(
            "cargo mutants --in-place --in-diff \"$RUNNER_TEMP/pr.diff\" --build-timeout 0 "
        ),
        "the measured step is not cargo-mutants' own baseline on the diff: {script}"
    );
    for flag in ["--shard", "--baseline", "--no-config", "--config"] {
        assert!(
            !script.contains(flag),
            "the measured step narrows or changes the run: {flag}"
        );
    }
}

#[test]
fn mutants_baseline_is_measured_as_a_shard_runs() {
    baseline_matches_a_shard(&mutants_yml());
}

negative_control!(
    mutants_baseline_is_measured_as_a_shard_runs,
    "a shard's share of the packages, not the diff's",
    expected = "the measured step narrows or changes the run: --shard",
    baseline_matches_a_shard(&edited(
        &mutants_yml(),
        "--build-timeout 0 -o \"$RUNNER_TEMP\"\n          code=$?\n          echo \"cargo mutants baseline",
        "--build-timeout 0 --shard 0/8 -o \"$RUNNER_TEMP\"\n          code=$?\n          echo \"cargo mutants baseline"
    ))
);

mod other_setup {
    use super::*;
    negative_control!(
        mutants_baseline_is_measured_as_a_shard_runs,
        "the baseline's job without the GPU backend the shards test on",
        expected = "the baseline's job environment is not the shards'",
        baseline_matches_a_shard(&mutants_yml().replacen(
            "    env:\n      PRIN_GPU_BACKEND: vulkan\n",
            "    env:\n      PRIN_GPU_BACKEND: none\n",
            1
        ))
    );
}

mod uncontained {
    use super::*;
    negative_control!(
        mutants_baseline_is_measured_as_a_shard_runs,
        "the measured step outside the containment",
        expected = "the measured step's `shell` is not a shard's",
        baseline_matches_a_shard(&mutants_yml().replacen(
            "        shell: bash scripts/contain.sh run bash --noprofile --norc -eo pipefail {0}\n        timeout-minutes: 300\n",
            "        timeout-minutes: 300\n",
            1
        ))
    );
}

// ---------------------------------------------------------------------------------------------------------------
// The measured step's exits, and the published value, on a stand-in `cargo`.

/// A scratch directory, its `temp`, a `bin` holding a stand-in `cargo` that exits `code`, and `$GITHUB_OUTPUT`.
fn stand_in(tag: &str, code: i32) -> (Scratch, PathBuf, PathBuf) {
    let dir = Scratch::new(&format!("mutants_timeout_{tag}"));
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).expect("bin");
    write_executable(&bin.join("cargo"), format!("#!/bin/sh\nexit {code}\n")).expect("cargo");
    let temp = dir.join("temp");
    std::fs::create_dir_all(&temp).expect("temp");
    let output = dir.join("github_output");
    std::fs::write(&output, "").expect("output");
    (dir, bin, output)
}

/// Runs `script` as a workflow's `bash` does, on `bin` first in `PATH`, from the workspace root.
fn bash(
    script: &str,
    temp: &Path,
    output: &Path,
    bin: &Path,
    env: &[(&str, &str)],
) -> (bool, String) {
    let path = std::env::var("PATH").unwrap_or_default();
    let mut cmd = Command::new("bash");
    cmd.args(["--noprofile", "--norc", "-eo", "pipefail", "-c", script])
        .current_dir(root())
        .env("RUNNER_TEMP", temp)
        .env("GITHUB_OUTPUT", output)
        .env("PATH", format!("{}:{path}", bin.display()));
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.timed_output().expect("bash ran");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr),
    )
}

fn measured_step(yml: &str) -> String {
    let wf = parse(yml);
    text(
        &step(
            &wf["jobs"]["mutants-baseline"],
            "cargo mutants --in-diff, the unmutated",
        )["run"],
    )
    .to_owned()
}

/// The step on cargo-mutants' exit `code`: passed, marked measured, and its log.
fn measured_on(script: &str, code: i32) -> (bool, bool, String) {
    let (dir, bin, output) = stand_in(&format!("exit_{code}"), code);
    let (ok, log) = bash(script, &dir.join("temp"), &output, &bin, &[]);
    let marked = std::fs::read_to_string(&output)
        .expect("output")
        .lines()
        .any(|l| l == "measured=true");
    (ok, marked, log)
}

fn exits_mean_what_they_say(script: &str) {
    let (ok, marked, log) = measured_on(script, 3);
    assert!(
        ok && marked,
        "a measured baseline (exit 3) is not published: {log}"
    );
    let (ok, marked, log) = measured_on(script, 0);
    assert!(
        ok && !marked,
        "a diff with no mutant (exit 0) failed, or counted as measured: {log}"
    );
    let (ok, marked, log) = measured_on(script, 4);
    assert!(
        !ok && !marked && log.contains("::error::the unmutated baseline failed its tests"),
        "a failing baseline (exit 4) does not fail the gate, named: {log}"
    );
    for code in [1, 2, 5, 70] {
        let (ok, marked, log) = measured_on(script, code);
        assert!(
            !ok && !marked && log.contains(&format!("::error::cargo mutants exited {code}")),
            "exit {code} is not a failed measurement: {log}"
        );
    }
}

#[test]
fn mutants_baseline_step_fails_on_a_failing_baseline() {
    exits_mean_what_they_say(&measured_step(&mutants_yml()));
}

negative_control!(
    mutants_baseline_step_fails_on_a_failing_baseline,
    "a failing baseline let through",
    expected = "a failing baseline (exit 4) does not fail the gate",
    exits_mean_what_they_say(&measured_step(&mutants_yml()).replace("exit 4\n", "exit 0\n"))
);

mod survivors_exit {
    use super::*;
    negative_control!(
        mutants_baseline_step_fails_on_a_failing_baseline,
        "any other exit read as measured",
        expected = "exit 2 is not a failed measurement",
        exits_mean_what_they_say(
            &measured_step(&mutants_yml()).replace("3) echo \"measured", "2|3) echo \"measured")
        )
    );
}

/// A cut-off baseline fails its job, named (R-302), and the published timeout is computed only from a measurement.
fn cut_baseline_fails(yml: &str) {
    let wf = parse(yml);
    let job = &wf["jobs"]["mutants-baseline"];
    let named = step(job, "Baseline cut off or failed");
    let cond = text(&named["if"]);
    assert!(
        cond.contains("always()") && cond.contains("steps.baseline.outcome != 'success'"),
        "a cut-off baseline is not named: {cond}"
    );
    assert!(
        text(&named["run"]).trim_end().ends_with("exit 1"),
        "a cut-off baseline does not fail the job"
    );
    assert_eq!(
        step(job, "cargo mutants --in-diff, the unmutated")["timeout-minutes"],
        json!(300),
        "the baseline has no limit of its own"
    );
}

#[test]
fn mutants_baseline_cut_off_fails_named() {
    cut_baseline_fails(&mutants_yml());
}

negative_control!(
    mutants_baseline_cut_off_fails_named,
    "the named failure no longer running on a cut",
    expected = "a cut-off baseline is not named",
    cut_baseline_fails(&edited(
        &mutants_yml(),
        "if: always() && steps.baseline.outcome != 'success' && steps.baseline.outcome != 'skipped'",
        "if: steps.baseline.outcome == 'never'"
    ))
);

/// The timeout step's script on a run holding `outcomes`, or none: whether it passed, and `$GITHUB_OUTPUT`.
fn published(script: &str, tag: &str, outcomes: Option<&str>) -> (bool, String, String) {
    let (dir, bin, output) = stand_in(&format!("published_{tag}"), 0);
    let temp = dir.join("temp");
    let mut measured = "";
    if let Some(outcomes) = outcomes {
        std::fs::create_dir_all(temp.join("mutants.out")).expect("out");
        std::fs::write(temp.join("mutants.out/outcomes.json"), outcomes).expect("outcomes");
        measured = "true";
    }
    let xtask = format!("\"{}\"", env!("CARGO_BIN_EXE_xtask"));
    let script = script.replace("cargo xtask", &xtask);
    let (ok, log) = bash(&script, &temp, &output, &bin, &[("MEASURED", measured)]);
    (ok, std::fs::read_to_string(&output).expect("output"), log)
}

fn publishes_the_outputs(yml: &str) {
    let wf = parse(yml);
    let job = &wf["jobs"]["mutants-baseline"];
    let script = text(&step(job, "cargo xtask mutants-timeout")["run"]);
    let (ok, out, log) = published(
        script,
        "measured",
        Some(&baseline_run("Success", 172.0, 5103.5)),
    );
    assert!(
        ok && out == "baseline-seconds=5103.5\ntimeout=15311\n",
        "the measured seconds and the timeout are not published: {out:?} {log}"
    );
    assert!(
        log.contains("baseline-seconds=5103.5") && log.contains("timeout=15311"),
        "the measured seconds are not in the log: {log}"
    );
    let (ok, out, log) = published(script, "none", None);
    assert!(
        ok && out == "baseline-seconds=0.0\ntimeout=60\n",
        "a diff with no mutant publishes no floor: {out:?} {log}"
    );
}

#[test]
fn mutants_baseline_publishes_the_seconds_and_the_timeout() {
    publishes_the_outputs(&mutants_yml());
}

negative_control!(
    mutants_baseline_publishes_the_seconds_and_the_timeout,
    "the outputs not written",
    expected = "the measured seconds and the timeout are not published",
    publishes_the_outputs(&edited(
        &mutants_yml(),
        "          cat \"$out\" >> \"$GITHUB_OUTPUT\"\n",
        ""
    ))
);

/// A shard that lost the baseline's output fails: cargo-mutants 27.1.0 refuses `--timeout ""` as a usage error (`invalid
/// value ''`), exit 1, which the shard's step fails on. That needs the variable quoted, so the empty value stays one.
fn empty_timeout_stays_a_value(yml: &str) {
    let wf = parse(yml);
    let script = text(&step(&wf["jobs"]["mutants"], "cargo mutants --in-diff --shard")["run"]);
    assert!(
        script.contains("--timeout \"$MUTANT_TIMEOUT\""),
        "the timeout is not quoted: an empty one would take the next option as its value"
    );
}

#[test]
fn mutants_shard_refuses_a_lost_timeout() {
    empty_timeout_stays_a_value(&mutants_yml());
}

negative_control!(
    mutants_shard_refuses_a_lost_timeout,
    "the variable unquoted",
    expected = "the timeout is not quoted",
    empty_timeout_stays_a_value(&edited(
        &mutants_yml(),
        "--timeout \"$MUTANT_TIMEOUT\"",
        "--timeout $MUTANT_TIMEOUT"
    ))
);

// ---------------------------------------------------------------------------------------------------------------
// The listing: every timed-out mutant, by name, across every shard, with a count.

/// A shard report under `temp/shards`, with these timed-out and surviving mutants.
fn shard_report(temp: &Path, k: u32, timeouts: &[&str], missed: &[&str]) -> PathBuf {
    let dir = temp.join(format!("shards/mutants-report-shard-{k}"));
    std::fs::create_dir_all(&dir).expect("shard dir");
    let outcome =
        |name: &&str, summary| json!({"scenario": {"Mutant": {"name": name}}, "summary": summary});
    let mut outcomes: Vec<Value> = timeouts.iter().map(|m| outcome(m, "Timeout")).collect();
    outcomes.extend(missed.iter().map(|m| outcome(m, "MissedMutant")));
    outcomes.push(outcome(&"a.rs:1:1: caught", "CaughtMutant"));
    std::fs::write(
        dir.join("outcomes.json"),
        json!({ "outcomes": outcomes }).to_string(),
    )
    .expect("outcomes written");
    dir
}

fn lists(out: &str, count: &str, names: &[&str]) {
    assert!(
        out.contains(&format!("mutants-check: {count}\n")),
        "mutants-check does not give the count `{count}`: {out}"
    );
    for name in names {
        assert!(
            out.contains(&format!(
                "mutants-check: timed out (not a survivor): {name}\n"
            )),
            "mutants-check does not list {name}: {out}"
        );
    }
    assert_eq!(
        out.matches("timed out (not a survivor)").count(),
        names.len(),
        "mutants-check lists another timeout: {out}"
    );
}

const COUNTS_AS_CAUGHT: &str = "over 2 shard(s): each counts as caught, not as a survivor";

/// `cargo xtask mutants-check` over two shards, the given mutants in the first and second.
fn checked(tag: &str, first: &[&str], second: &[&str], missed: &[&str]) -> (bool, String, String) {
    let dir = Scratch::new(&format!("mutants_timeout_check_{tag}"));
    let a = shard_report(&dir, 0, first, missed);
    let b = shard_report(&dir, 1, second, &[]);
    let list = root().join(EQUIVALENT_LIST);
    xtask(&[
        Path::new("mutants-check"),
        &a,
        &b,
        Path::new("--equivalent"),
        &list,
    ])
}

#[test]
fn mutants_check_lists_every_timeout_across_the_shards() {
    let (ok, out, err) = checked("two", &["a.rs:2:2: hang"], &["b.rs:3:3: hang"], &[]);
    assert!(ok, "a timeout failed the check: {err}");
    lists(
        &out,
        &format!("2 timed out {COUNTS_AS_CAUGHT}"),
        &["a.rs:2:2: hang", "b.rs:3:3: hang"],
    );
}

negative_control!(
    mutants_check_lists_every_timeout_across_the_shards,
    "one timeout missing from the list",
    expected = "mutants-check does not list b.rs:3:3: hang",
    lists(
        "mutants-check: 2 timed out over 2 shard(s): each counts as caught, not as a survivor\n\
         mutants-check: timed out (not a survivor): a.rs:2:2: hang\n",
        &format!("2 timed out {COUNTS_AS_CAUGHT}"),
        &["a.rs:2:2: hang", "b.rs:3:3: hang"]
    )
);

#[test]
fn mutants_check_says_none_timed_out() {
    let (ok, out, err) = checked("none", &[], &[], &[]);
    assert!(ok, "the check failed: {err}");
    lists(&out, &format!("0 timed out {COUNTS_AS_CAUGHT}"), &[]);
}

negative_control!(
    mutants_check_says_none_timed_out,
    "a run that gives no count",
    expected = "mutants-check does not give the count",
    lists("mutants-check: 0 surviving mutant(s)\n", "0 timed out", &[])
);

#[test]
fn mutants_check_lists_the_timeouts_on_a_failing_verdict() {
    let (ok, out, err) = checked("failing", &["a.rs:2:2: hang"], &[], &["c.rs:4:4: survives"]);
    assert!(
        !ok && err.contains("c.rs:4:4: survives"),
        "a survivor passed: {err}"
    );
    lists(
        &out,
        &format!("1 timed out {COUNTS_AS_CAUGHT}"),
        &["a.rs:2:2: hang"],
    );
}

negative_control!(
    mutants_check_lists_the_timeouts_on_a_failing_verdict,
    "a listing with the wrong count",
    expected = "mutants-check does not give the count",
    lists(
        "mutants-check: 2 timed out over 2 shard(s): each counts as caught, not as a survivor\n",
        &format!("1 timed out {COUNTS_AS_CAUGHT}"),
        &[]
    )
);

/// The verdict step over `reports` (given to [`shard_report`]) with the shards' result `result`: passed, its log and
/// its job summary.
fn verdict(
    yml: &str,
    tag: &str,
    result: &str,
    reports: &[(&[&str], &[&str])],
) -> (bool, String, String) {
    let wf = parse(yml);
    let script = text(
        &step(
            &wf["jobs"]["mutants-verdict"],
            "cargo xtask mutants-check over every shard",
        )["run"],
    )
    .replace("${{ needs.mutants.result }}", result)
    .replace(
        "cargo xtask",
        &format!("\"{}\"", env!("CARGO_BIN_EXE_xtask")),
    );
    let (dir, bin, output) = stand_in(&format!("verdict_{tag}"), 0);
    let temp = dir.join("temp");
    for (k, (timeouts, missed)) in (0u32..).zip(reports) {
        shard_report(&temp, k, timeouts, missed);
    }
    let summary = dir.join("summary");
    let path = summary.to_string_lossy().into_owned();
    let (ok, log) = bash(
        &script,
        &temp,
        &output,
        &bin,
        &[("GITHUB_STEP_SUMMARY", &path)],
    );
    (
        ok,
        log,
        std::fs::read_to_string(&summary).unwrap_or_default(),
    )
}

fn verdict_lists_in_its_summary(yml: &str) {
    let (ok, log, summary) = verdict(
        yml,
        "two",
        "success",
        &[(&["a.rs:2:2: hang"], &[]), (&["b.rs:3:3: hang"], &[])],
    );
    assert!(ok, "the verdict failed on timeouts: {log}");
    assert!(
        summary.contains("2 timed out over 2 shard(s)")
            && summary.contains("- a.rs:2:2: hang\n")
            && summary.contains("- b.rs:3:3: hang\n"),
        "the job summary does not list the timeouts: {summary:?}"
    );
    assert!(
        log.contains("mutants-check: timed out (not a survivor): b.rs:3:3: hang"),
        "the verdict's log does not list the timeouts: {log}"
    );
}

#[test]
fn mutants_verdict_lists_the_timeouts_in_its_summary() {
    verdict_lists_in_its_summary(&mutants_yml());
}

negative_control!(
    mutants_verdict_lists_the_timeouts_in_its_summary,
    "nothing written to the summary",
    expected = "the job summary does not list the timeouts",
    verdict_lists_in_its_summary(&edited(
        &mutants_yml(),
        "} >> \"${GITHUB_STEP_SUMMARY:-/dev/null}\"\n          exit \"$fail\"",
        "} > /dev/null\n          exit \"$fail\""
    ))
);

fn verdict_says_none(yml: &str) {
    let (ok, log, summary) = verdict(yml, "zero", "success", &[]);
    assert!(ok, "a diff with no mutant failed: {log}");
    assert!(
        summary.contains("0 timed out") && log.contains("mutants-check: 0 timed out"),
        "no count when no shard tested a mutant: {summary:?} {log}"
    );
    let (ok, _, summary) = verdict(yml, "zero_shards", "success", &[(&[], &[]), (&[], &[])]);
    assert!(
        ok && summary.contains("0 timed out over 2 shard(s)"),
        "no count when none timed out: {summary:?}"
    );
}

#[test]
fn mutants_verdict_says_none_timed_out() {
    verdict_says_none(&mutants_yml());
}

negative_control!(
    mutants_verdict_says_none_timed_out,
    "no count when no shard reported",
    expected = "no count when no shard tested a mutant",
    verdict_says_none(&edited(
        &mutants_yml(),
        "            echo \"mutants-check: 0 timed out over 0 shard(s)\" >> \"$report\"\n",
        ""
    ))
);

/// R-302 stands: a cut shard (a timeout list beside it or not) fails the verdict, and its reports are still listed; a
/// shard skipped because the baseline failed fails it too.
fn cut_shard_stays_a_failure(yml: &str) {
    for result in ["failure", "cancelled", "skipped"] {
        let (ok, log, summary) = verdict(yml, result, result, &[(&["a.rs:2:2: hang"], &[])]);
        assert!(
            !ok && log.contains("::error::a mutants shard did not pass"),
            "a run with shards `{result}` passed the verdict: {log}"
        );
        assert!(
            summary.contains("- a.rs:2:2: hang\n"),
            "a cut run's timeouts are not listed: {summary:?}"
        );
    }
}

#[test]
fn mutants_verdict_fails_a_cut_shard_and_still_lists() {
    cut_shard_stays_a_failure(&mutants_yml());
}

negative_control!(
    mutants_verdict_fails_a_cut_shard_and_still_lists,
    "the shards' result ignored",
    expected = "passed the verdict",
    cut_shard_stays_a_failure(&edited(
        &mutants_yml(),
        "if [ \"${{ needs.mutants.result }}\" != success ]; then",
        "if [ \"${{ needs.mutants.result }}\" = never ]; then"
    ))
);

/// The baseline's report is no shard's: the verdict's download pattern does not take it.
fn baseline_report_is_no_shard(yml: &str) {
    let wf = parse(yml);
    let upload = wf["jobs"]["mutants-baseline"]["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .find(|s| text(&s["uses"]).starts_with("actions/upload-artifact"))
        .expect("the baseline's upload");
    let name = text(&upload["with"]["name"]);
    assert_eq!(
        upload["if"],
        json!("always()"),
        "a failing baseline's log is lost"
    );
    assert!(
        !name.starts_with("mutants-report-shard-"),
        "the verdict would read the baseline's report as a shard's: {name}"
    );
}

#[test]
fn mutants_baseline_report_is_no_shard() {
    baseline_report_is_no_shard(&mutants_yml());
}

negative_control!(
    mutants_baseline_report_is_no_shard,
    "the baseline's report under a shard's name",
    expected = "the verdict would read the baseline's report as a shard's",
    baseline_report_is_no_shard(&edited(
        &mutants_yml(),
        "name: mutants-report-baseline",
        "name: mutants-report-shard-baseline"
    ))
);
