//! QA tests for TASK-M0-23's sharding, written from REQ-VAL-148 and REQ-VAL-149 as reworded by R-302, not from the
//! implementation:
//!
//! - REQ-VAL-148: "Every pull request must run `cargo mutants --in-diff` on its changed code in CI, sharded across
//!   parallel CI jobs (`--shard k/n`), each shard within the per-shard time limit (REQ-VAL-149) ... The job must fail
//!   on any surviving mutant not in a checked-in list of equivalent mutants". verify (code): "`mutants.yml` runs `cargo
//!   mutants --in-diff` against the PR's base on pull_request events as a matrix of n shards, each `--shard k/n` with
//!   its own timeout, and a shard cut off by its timeout fails the job (R-302)".
//! - REQ-VAL-149: "each shard runs under that limit".
//! - cargo-mutants 27.1.0 numbers shards from 0: `--shard k/n` takes `0 <= k < n`.
//!
//! What these tests hold to: every shard k of 0..n runs, and no other; a shard cut off by its limit, or failed, or one
//! that tested nothing and did not say so with exit 0, fails and is named; and the aggregate verdict fails when any
//! shard was cut off or failed, and reads every shard's report, so a survivor in any shard is named.
//!
//! The workflow's shell steps are executed here in bash, with their `${{ … }}` expressions substituted and the
//! programs they call stood in for (a stand-in `cargo` for `cargo mutants`; the built xtask for `cargo xtask`), so the
//! assertions are on what the steps do, not on how they are spelt. Each test's control (R-176) feeds the same
//! assertion a workflow differing in the one respect the requirement turns on, and trips it by its message.
// The file name `qa_TASK-M0-23_shards` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::path::{Path, PathBuf};
use std::process::Command;

use validation::negative_control;
use validation::spawn::{write_executable, Spawn};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn mutants_yml() -> String {
    std::fs::read_to_string(root().join(".github/workflows/mutants.yml")).expect("mutants.yml")
}

/// A fresh directory for one call: tests and their controls run in parallel, so each call gets its own.
fn scratch(tag: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("qa23s_{}_{n}_{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

// ---------------------------------------------------------------------------------------------------------------
// Reading the workflow: a job's lines, its steps, a step's keys and its `run: |` script.

/// The lines of job `name`: from `  <name>:` to the next job key at the same indent.
fn job(yml: &str, name: &str) -> Vec<String> {
    let head = format!("  {name}:");
    let mut lines = yml.lines().skip_while(|l| l.trim_end() != head);
    let Some(first) = lines.next() else {
        panic!("qa23s: no `{name}` job in the workflow");
    };
    std::iter::once(first)
        .chain(lines.take_while(|l| {
            l.is_empty() || l.starts_with("   ") || l.trim_start().starts_with('#')
        }))
        .map(str::to_owned)
        .collect()
}

/// A job's steps, each its lines, the first being `      - …`.
fn steps(job: &[String]) -> Vec<Vec<String>> {
    let mut out: Vec<Vec<String>> = Vec::new();
    let mut inside = false;
    for line in job {
        if line.trim_end() == "    steps:" {
            inside = true;
            continue;
        }
        if !inside {
            continue;
        }
        if line.starts_with("      - ") {
            out.push(vec![line.clone()]);
        } else if let Some(step) = out.last_mut() {
            step.push(line.clone());
        }
    }
    out
}

/// A step's `key: value` (not comments), on its first line (`- key:`) or at the step's key indent.
fn key(step: &[String], key: &str) -> Option<String> {
    step.iter().find_map(|l| {
        let t = l.trim_start().trim_start_matches("- ");
        let indent = l.len() - l.trim_start().len();
        (indent <= 8 && !t.starts_with('#'))
            .then(|| t.strip_prefix(&format!("{key}:")))
            .flatten()
            .map(|v| v.trim().to_owned())
    })
}

/// A step's `run: |` block, dedented.
fn script(step: &[String]) -> String {
    let at = step
        .iter()
        .position(|l| l.trim() == "run: |")
        .expect("qa23s: step has no `run: |` block");
    let indent = step[at].len() - step[at].trim_start().len();
    step[at + 1..]
        .iter()
        .take_while(|l| l.trim().is_empty() || l.len() - l.trim_start().len() > indent)
        .map(|l| l.get(indent + 2..).unwrap_or("").to_owned() + "\n")
        .collect()
}

/// `text` with each `${{ expr }}` given in `values` substituted; any other expression left is refused.
fn expand(text: &str, values: &[(&str, &str)]) -> String {
    let mut out = text.to_owned();
    for (expr, value) in values {
        out = out.replace(&format!("${{{{ {expr} }}}}"), value);
    }
    assert!(
        !out.contains("${{"),
        "qa23s: unexpanded expression in {out}"
    );
    out
}

/// Runs `script` as GitHub's default `run` shell on ubuntu does (`bash --noprofile --norc -eo pipefail`), with `env`,
/// from the workspace root: whether it exited 0, and its stdout and stderr together.
fn bash(script: &str, env: &[(&str, &Path)], path_prefix: Option<&Path>) -> (bool, String) {
    let mut cmd = Command::new("bash");
    cmd.args(["--noprofile", "--norc", "-eo", "pipefail", "-c", script])
        .current_dir(root());
    for (k, v) in env {
        cmd.env(k, v);
    }
    if let Some(dir) = path_prefix {
        let path = std::env::var("PATH").unwrap_or_default();
        cmd.env("PATH", format!("{}:{path}", dir.display()));
    }
    let out = cmd.timed_output().expect("qa23s: bash ran");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr),
    )
}

/// The `mutants` job's step that runs `cargo mutants --shard`.
fn shard_step(job: &[String]) -> Vec<String> {
    steps(job)
        .into_iter()
        .find(|s| {
            s.iter()
                .any(|l| l.contains("cargo mutants") && l.contains("--shard"))
        })
        .expect("qa23s: no step runs `cargo mutants --shard` in the mutants job")
}

// ---------------------------------------------------------------------------------------------------------------
// Every shard k of 0..n runs, and no other.

/// The shard count n in `--shard ${{ matrix.<var> }}/n`, and the matrix variable's values.
fn shards(yml: &str) -> (u32, Vec<u32>) {
    let lines = job(yml, "mutants");
    let step = shard_step(&lines);
    let text = step.join("\n");
    let at = text
        .find("--shard ${{ matrix.")
        .expect("qa23s: `--shard` does not take k from the matrix");
    let rest = &text[at + "--shard ${{ matrix.".len()..];
    let (var, rest) = rest
        .split_once(" }}/")
        .expect("qa23s: `--shard k/n` malformed");
    let n: u32 = rest
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .expect("qa23s: n in `--shard k/n` is not a number");
    let list = lines
        .iter()
        .find_map(|l| {
            l.trim()
                .strip_prefix(&format!("{var}:"))
                .map(|v| v.trim().to_owned())
        })
        .unwrap_or_else(|| panic!("qa23s: the matrix has no `{var}`"));
    let values = list
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(|v| v.trim().parse::<u32>().expect("qa23s: shard index"))
        .collect();
    (n, values)
}

/// R-302: the matrix runs shard k for every k of 0..n (n >= 2: parallel), each once; a failing shard does not
/// cancel the others (their survivors would go unnamed); the sharded command is the in-diff one.
fn every_shard_runs(yml: &str) {
    let (n, mut ks) = shards(yml);
    ks.sort_unstable();
    assert!(
        n >= 2 && ks == (0..n).collect::<Vec<_>>(),
        "qa23s: the matrix does not run each shard of 0..{n} once: {ks:?}"
    );
    let lines = job(yml, "mutants");
    assert!(
        lines.iter().any(|l| l.trim() == "fail-fast: false"),
        "qa23s: one failing shard cancels the others (no `fail-fast: false`)"
    );
    let text = shard_step(&lines).join("\n");
    assert!(
        text.contains("--in-diff"),
        "qa23s: the sharded cargo mutants is not the in-diff run"
    );
}

#[test]
fn qa23s_every_shard_of_n_runs_once() {
    every_shard_runs(&mutants_yml());
}

negative_control!(
    qa23s_every_shard_of_n_runs_once,
    "the last shard left out of the matrix",
    expected = "qa23s: the matrix does not run each shard of 0..8 once",
    every_shard_runs(&mutants_yml().replace(
        "shard: [0, 1, 2, 3, 4, 5, 6, 7]",
        "shard: [0, 1, 2, 3, 4, 5, 6]"
    ))
);

mod counted_from_one {
    use super::*;
    negative_control!(
        qa23s_every_shard_of_n_runs_once,
        "shards counted 1..=n, so shard 0 never runs",
        expected = "qa23s: the matrix does not run each shard of 0..8 once",
        every_shard_runs(&mutants_yml().replace(
            "shard: [0, 1, 2, 3, 4, 5, 6, 7]",
            "shard: [1, 2, 3, 4, 5, 6, 7, 8]"
        ))
    );
}

// ---------------------------------------------------------------------------------------------------------------
// A shard cut off by its limit fails, visibly.

/// REQ-VAL-149 / R-302: the step running the shard has its own positive limit; nothing in the job lets a failed or
/// cut-off step pass (`continue-on-error`); and a later step that runs whatever happened (`always()`), on that step's
/// outcome not being success, fails with an `::error::` annotation naming the shard.
fn cut_off_fails(yml: &str) {
    let lines = job(yml, "mutants");
    let step = shard_step(&lines);
    let limit = key(&step, "timeout-minutes").and_then(|v| v.parse::<u32>().ok());
    assert!(
        limit.is_some_and(|m| m > 0),
        "qa23s: the shard's cargo mutants step has no limit of its own"
    );
    assert!(
        !lines
            .iter()
            .any(|l| !l.trim_start().starts_with('#') && l.contains("continue-on-error")),
        "qa23s: a failed or cut-off shard can pass (continue-on-error)"
    );
    let id = key(&step, "id").expect("qa23s: the shard step has no id to test its outcome by");
    let all = steps(&lines);
    let at = all.iter().position(|s| *s == step).expect("step");
    let reporter = all[at + 1..].iter().find(|s| {
        key(s, "if").is_some_and(|c| {
            c.contains("always()") && c.contains(&format!("steps.{id}.outcome != 'success'"))
        })
    });
    let Some(reporter) = reporter else {
        panic!("qa23s: no step reports a cut-off shard (always(), on steps.{id}.outcome)");
    };
    let (ok, log) = bash(
        &expand(&script(reporter), &[("matrix.shard", "5")]),
        &[],
        None,
    );
    assert!(
        !ok && log.contains("::error::") && log.contains("shard 5"),
        "qa23s: a cut-off shard is not failed with an error naming it: {log}"
    );
}

#[test]
fn qa23s_a_cut_off_shard_fails_visibly() {
    cut_off_fails(&mutants_yml());
}

negative_control!(
    qa23s_a_cut_off_shard_fails_visibly,
    "the shard step allowed to fail",
    expected = "qa23s: a failed or cut-off shard can pass",
    cut_off_fails(&mutants_yml().replacen(
        "        timeout-minutes: 120\n",
        "        timeout-minutes: 120\n        continue-on-error: true\n",
        1
    ))
);

mod no_limit {
    use super::*;
    negative_control!(
        qa23s_a_cut_off_shard_fails_visibly,
        "the shard step without its limit",
        expected = "qa23s: the shard's cargo mutants step has no limit of its own",
        cut_off_fails(&mutants_yml().replacen("        timeout-minutes: 120\n", "", 1))
    );
}

// ---------------------------------------------------------------------------------------------------------------
// A shard that tested nothing passes only on cargo-mutants' "nothing to test" (exit 0).

/// The shard step's script run with a stand-in `cargo` that exits `code`, having written an `outcomes.json` or not:
/// whether the step passed, and whether it marked the shard as tested for the check.
fn run_shard(script: &str, tag: &str, code: i32, writes: bool) -> (bool, bool, String) {
    let dir = scratch(&format!("shard_{tag}"));
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).expect("bin");
    let write = if writes {
        "mkdir -p \"$RUNNER_TEMP/mutants.out\"; echo '{\"outcomes\":[]}' > \"$RUNNER_TEMP/mutants.out/outcomes.json\""
    } else {
        ":"
    };
    write_executable(
        &bin.join("cargo"),
        format!("#!/bin/sh\n{write}\nexit {code}\n"),
    )
    .expect("stand-in cargo");
    let temp = dir.join("runner_temp");
    std::fs::create_dir_all(&temp).expect("temp");
    let output = dir.join("github_output");
    std::fs::write(&output, "").expect("output");
    let (ok, log) = bash(
        &expand(script, &[("matrix.shard", "3")]),
        &[("RUNNER_TEMP", &temp), ("GITHUB_OUTPUT", &output)],
        Some(&bin),
    );
    let tested = std::fs::read_to_string(&output)
        .unwrap_or_default()
        .lines()
        .any(|l| l.trim() == "tested=true");
    (ok, tested, log)
}

/// R-302 / R-202: a shard passes its run step only having tested its mutants (outcomes written, then checked), or on
/// exit 0 with nothing to test; any other exit, or a survivor exit with no outcomes, fails it.
fn untested_shard_fails(script: &str) {
    let (ok, tested, log) = run_shard(script, "shard_none", 0, false);
    assert!(
        ok && !tested,
        "qa23s: a shard with nothing to test failed: {log}"
    );
    for (code, tag) in [(2, "survivors"), (3, "timeouts"), (0, "tested_clean")] {
        let (ok, tested, log) = run_shard(script, tag, code, true);
        assert!(
            ok && tested,
            "qa23s: a shard that tested its mutants (exit {code}) is not passed to the check: {log}"
        );
    }
    for (code, tag) in [
        (2, "no_outcomes_2"),
        (3, "no_outcomes_3"),
        (1, "usage"),
        (4, "baseline"),
    ] {
        let (ok, _, log) = run_shard(script, tag, code, false);
        assert!(
            !ok,
            "qa23s: a shard that tested nothing passed (exit {code}): {log}"
        );
    }
    let (ok, _, log) = run_shard(script, "baseline_written", 4, true);
    assert!(!ok, "qa23s: a shard whose baseline failed passed: {log}");
}

fn shard_script() -> String {
    script(&shard_step(&job(&mutants_yml(), "mutants")))
}

#[test]
fn qa23s_a_shard_that_tested_nothing_fails() {
    untested_shard_fails(&shard_script());
}

negative_control!(
    qa23s_a_shard_that_tested_nothing_fails,
    "a script taking any exit without outcomes as nothing to test",
    expected = "qa23s: a shard that tested nothing passed (exit 2)",
    untested_shard_fails(
        &shard_script().replace("elif [ \"$code\" -eq 0 ]; then", "elif true; then")
    )
);

// ---------------------------------------------------------------------------------------------------------------
// The aggregate verdict: after every shard, over every shard's report.

const CAUGHT: &str = "src/a.rs:1:1: replace f -> u32 with 0";

/// One shard's uploaded report, as `mutants-check` downloads it: `shards/mutants-report-shard-<k>/outcomes.json`.
fn report(temp: &Path, k: u32, missed: &[&str]) {
    let dir = temp.join(format!("shards/mutants-report-shard-{k}"));
    std::fs::create_dir_all(&dir).expect("shard dir");
    let mut outcomes = vec![serde_json::json!({"scenario": "Baseline", "summary": "Success"})];
    outcomes.push(
        serde_json::json!({"scenario": {"Mutant": {"name": format!("{CAUGHT} (shard {k})")}},
                                     "summary": "CaughtMutant"}),
    );
    outcomes.extend(missed.iter().map(
        |m| serde_json::json!({"scenario": {"Mutant": {"name": m}}, "summary": "MissedMutant"}),
    ));
    std::fs::write(
        dir.join("outcomes.json"),
        serde_json::json!({ "outcomes": outcomes }).to_string(),
    )
    .expect("outcomes");
}

/// The aggregate job's script run with the shards' job result `result`, over n shard reports, `survivors` giving
/// each shard's unlisted survivors; `cargo xtask` is the built xtask, reading the checked-in list.
fn aggregate(
    script: &str,
    tag: &str,
    n: u32,
    result: &str,
    survivors: &[(u32, &str)],
) -> (bool, String) {
    let temp = scratch(&format!("agg_{tag}"));
    for k in 0..n {
        let missed: Vec<&str> = survivors
            .iter()
            .filter(|(s, _)| *s == k)
            .map(|(_, m)| *m)
            .collect();
        report(&temp, k, &missed);
    }
    let xtask = format!("\"{}\"", env!("CARGO_BIN_EXE_xtask"));
    let script = expand(script, &[("needs.mutants.result", result)]).replace("cargo xtask", &xtask);
    bash(&script, &[("RUNNER_TEMP", &temp)], None)
}

/// R-302 / R-202: the aggregate passes only when every shard succeeded and no shard's report holds an unlisted
/// survivor; a shard cut off (the matrix job's result `cancelled` or `failure`) fails it with an error, and survivors
/// in any shards, the last included, are each named.
fn aggregate_covers_every_shard(script: &str, n: u32) {
    let (ok, log) = aggregate(script, "clean", n, "success", &[]);
    assert!(ok, "qa23s: the aggregate failed a clean sharded run: {log}");
    for result in ["failure", "cancelled"] {
        let (ok, log) = aggregate(script, &format!("cut_{result}"), n, result, &[]);
        assert!(
            !ok && log.contains("::error::"),
            "qa23s: the aggregate passed though a shard was cut off or failed ({result}): {log}"
        );
    }
    let first = "src/b.rs:2:3: replace == with != in g";
    let last = "src/c.rs:4:5: delete ! in h";
    let (ok, log) = aggregate(
        script,
        "survivors",
        n,
        "success",
        &[(1, first), (n - 1, last)],
    );
    assert!(
        !ok && log.contains(first) && log.contains(last),
        "qa23s: the aggregate missed a survivor in some shard: {log}"
    );
}

fn aggregate_script() -> String {
    let lines = job(&mutants_yml(), "mutants-check");
    let step = steps(&lines)
        .into_iter()
        .find(|s| s.iter().any(|l| l.contains("cargo xtask mutants-check")))
        .expect("qa23s: the mutants-check job runs no mutants-check");
    script(&step)
}

#[test]
fn qa23s_the_aggregate_covers_every_shard() {
    aggregate_covers_every_shard(&aggregate_script(), shards(&mutants_yml()).0);
}

negative_control!(
    qa23s_the_aggregate_covers_every_shard,
    "an aggregate that ignores the shards' job result",
    expected = "qa23s: the aggregate passed though a shard was cut off or failed",
    aggregate_covers_every_shard(
        &aggregate_script().replace(
            "if [ \"${{ needs.mutants.result }}\" != success ]; then",
            "if [ \"${{ needs.mutants.result }}\" = never ]; then"
        ),
        shards(&mutants_yml()).0
    )
);

mod first_shard_only {
    use super::*;
    negative_control!(
        qa23s_the_aggregate_covers_every_shard,
        "an aggregate that reads only shard 0's report",
        expected = "qa23s: the aggregate missed a survivor in some shard",
        aggregate_covers_every_shard(
            &aggregate_script().replace("mutants-report-shard-*; do", "mutants-report-shard-0; do"),
            shards(&mutants_yml()).0
        )
    );
}

/// R-302: the aggregate job waits for the shards (`needs: mutants`) and runs whatever they did (`always()`), on pull
/// requests, and downloads every shard's report.
fn aggregate_runs_after_every_shard(yml: &str) {
    let lines = job(yml, "mutants-check");
    let has = |s: &str| {
        lines
            .iter()
            .any(|l| !l.trim_start().starts_with('#') && l.contains(s))
    };
    assert!(
        has("needs: mutants"),
        "qa23s: the aggregate does not wait for the shards"
    );
    assert!(
        lines.iter().any(|l| {
            let t = l.trim();
            t.starts_with("if:") && t.contains("always()") && t.contains("pull_request")
        }),
        "qa23s: the aggregate is skipped when a shard fails (no always())"
    );
    assert!(
        has("pattern: mutants-report-shard-*"),
        "qa23s: the aggregate does not download every shard's report"
    );
}

#[test]
fn qa23s_the_aggregate_runs_after_every_shard() {
    aggregate_runs_after_every_shard(&mutants_yml());
}

negative_control!(
    qa23s_the_aggregate_runs_after_every_shard,
    "an aggregate skipped once a shard has failed",
    expected = "qa23s: the aggregate is skipped when a shard fails",
    aggregate_runs_after_every_shard(&mutants_yml().replace(
        "if: always() && github.event_name == 'pull_request'",
        "if: github.event_name == 'pull_request'"
    ))
);
