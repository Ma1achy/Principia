//! R-305: `mutants-check`, a required status check on `main`, reports on every pull request and passes when the PR's
//! diff has no mutant. A diff that names no `.rs` file (a docs-only or rulings PR) skips each shard's setup and run;
//! a diff naming Rust source runs `cargo mutants`, and a shard of it with nothing to mutate passes (REQ-VAL-148).
//!
//! These tests run the `mutants` and `mutants-check` jobs of `.github/workflows/mutants.yml` as a runner would, step by
//! step: each step's `if:` is evaluated (the subset of GitHub's expressions the jobs use), a `uses:` step that runs
//! is recorded but not executed, and each `run:` script runs in bash with its `${{ … }}` expressions substituted.
//! The programs the scripts call are stand-ins that log each call: `git diff` prints the test's diff, `cargo`
//! (for `cargo mutants`) writes no outcomes, as cargo-mutants does when it finds no mutant, and `sudo` and `python3`
//! do nothing. `cargo xtask` is the built xtask. Each test's control (R-176) runs the same test on a workflow with the
//! one step it turns on broken, and trips it by its message.
//!
//! A job-level `if:` does not stop a workflow run from reporting the job: skipped, which GitHub counts as passed for a
//! required check. So the gate is a workflow of its own that only `pull_request` runs, and no other workflow has a
//! `mutants-check` job, which a `push` run would report skipped (R-305).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use validation::negative_control;
use validation::spawn::{write_executable, Spawn};

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

/// The per-PR mutation gate's workflow, which only `pull_request` runs (R-305).
fn mutants_yml() -> String {
    std::fs::read_to_string(root().join(".github/workflows/mutants.yml")).expect("mutants.yml")
}

/// A fresh directory per call, since tests and their controls run in parallel; deleted when the test passes, kept
/// when it fails (R-342).
fn scratch(tag: &str) -> Scratch {
    let dir = Scratch::new(&format!("mutants_no_mutant_{tag}"));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

// ---------------------------------------------------------------------------------------------------------------
// Reading the workflow.

/// The lines of job `name`, from `  <name>:` to the next job.
fn job(yml: &str, name: &str) -> Vec<String> {
    let head = format!("  {name}:");
    let mut lines = yml.lines().skip_while(|l| l.trim_end() != head);
    let first = lines
        .next()
        .unwrap_or_else(|| panic!("no `{name}` job in mutants.yml"));
    std::iter::once(first)
        .chain(lines.take_while(|l| {
            l.is_empty() || l.starts_with("   ") || l.trim_start().starts_with('#')
        }))
        .map(str::to_owned)
        .collect()
}

/// A job's key at the job's own indent (4), such as its `if:`.
fn job_key(job: &[String], key: &str) -> Option<String> {
    job.iter().find_map(|l| {
        l.strip_prefix("    ")
            .filter(|t| !t.starts_with(' '))
            .and_then(|t| t.strip_prefix(&format!("{key}:")))
            .map(|v| v.trim().to_owned())
    })
}

/// A job's steps, each its lines, the first being `      - …`; comment lines are dropped.
fn steps(job: &[String]) -> Vec<Vec<String>> {
    let mut out: Vec<Vec<String>> = Vec::new();
    let mut inside = false;
    for line in job {
        if line.trim_end() == "    steps:" {
            inside = true;
        } else if inside && !line.trim_start().starts_with('#') {
            if line.starts_with("      - ") {
                out.push(vec![line.clone()]);
            } else if let Some(step) = out.last_mut() {
                step.push(line.clone());
            }
        }
    }
    out
}

/// A step's `key: value`, on its first line (`- key:`) or at the step's key indent (8).
fn key(step: &[String], key: &str) -> Option<String> {
    step.iter().find_map(|l| {
        let indent = l.len() - l.trim_start().len();
        let t = l.trim_start().trim_start_matches("- ");
        (indent <= 8)
            .then(|| t.strip_prefix(&format!("{key}:")))
            .flatten()
            .map(|v| v.trim().to_owned())
    })
}

/// A step's script: its `run: |` block, dedented, or its one-line `run:`.
fn script(step: &[String]) -> Option<String> {
    let run = key(step, "run")?;
    if run != "|" {
        return Some(run + "\n");
    }
    let at = step
        .iter()
        .position(|l| l.trim() == "run: |")
        .expect("run block");
    let indent = step[at].len() - step[at].trim_start().len();
    Some(
        step[at + 1..]
            .iter()
            .take_while(|l| l.trim().is_empty() || l.len() - l.trim_start().len() > indent)
            .map(|l| l.get(indent + 2..).unwrap_or("").to_owned() + "\n")
            .collect(),
    )
}

// ---------------------------------------------------------------------------------------------------------------
// Running a job as a runner would.

/// What a job's steps leave for the next step's `if:` to read.
#[derive(Default)]
struct State {
    failed: bool,
    outcome: HashMap<String, String>,
    outputs: HashMap<String, HashMap<String, String>>,
}

/// One operand of a comparison: a quoted literal or a context value.
fn operand(s: &str, state: &State) -> String {
    let s = s.trim();
    if let Some(lit) = s.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')) {
        return lit.to_owned();
    }
    if s == "github.event_name" {
        return "pull_request".to_owned();
    }
    // A pull request's run is on its merge ref, never `refs/heads/main`, so a cache save (R-326) does not run.
    if s == "github.ref" {
        return "refs/pull/1/merge".to_owned();
    }
    let parts: Vec<&str> = s.split('.').collect();
    match parts.as_slice() {
        ["steps", id, "outcome"] => state
            .outcome
            .get(*id)
            .cloned()
            .unwrap_or_else(|| panic!("`if:` reads step {id}'s outcome before it ran")),
        ["steps", id, "outputs", k] => state
            .outputs
            .get(*id)
            .and_then(|o| o.get(*k))
            .cloned()
            .unwrap_or_default(),
        _ => panic!("the test's `if:` evaluator does not know `{s}`"),
    }
}

/// Evaluates an `if:` (`&&` of `always()` and `==`/`!=` comparisons), with GitHub's implied `success()` when the
/// condition names no status function.
fn holds(cond: Option<&str>, state: &State) -> bool {
    let Some(cond) = cond else {
        return !state.failed;
    };
    let cond = cond
        .trim()
        .trim_start_matches("${{")
        .trim_end_matches("}}")
        .trim();
    assert!(
        !cond.contains("||") && !cond.contains("success()") && !cond.contains("failure()"),
        "the test's `if:` evaluator does not know `{cond}`"
    );
    let mut always = false;
    let mut value = true;
    for term in cond.split("&&").map(str::trim) {
        if term == "always()" {
            always = true;
        } else if let Some((l, r)) = term.split_once("!=") {
            value &= operand(l, state) != operand(r, state);
        } else if let Some((l, r)) = term.split_once("==") {
            value &= operand(l, state) == operand(r, state);
        } else {
            panic!("the test's `if:` evaluator does not know `{term}`");
        }
    }
    value && (always || !state.failed)
}

/// Stand-ins for the programs the scripts call; each logs its call to `$STUB_CALLS`.
fn stand_ins(dir: &Path) -> PathBuf {
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).expect("bin");
    let log = "printf '%s\\n' \"$(basename \"$0\") $*\" >> \"$STUB_CALLS\"";
    let git = format!("#!/bin/sh\n{log}\n[ \"$1\" = diff ] && cat \"$STUB_DIFF\"\nexit 0\n");
    write_executable(&bin.join("git"), git).expect("git");
    for name in ["cargo", "sudo", "python3"] {
        write_executable(&bin.join(name), format!("#!/bin/sh\n{log}\nexit 0\n")).expect(name);
    }
    bin
}

/// What running a job left: whether it passed, its log, the `uses:` steps that ran, and the stand-ins' calls.
struct Run {
    ok: bool,
    log: String,
    used: Vec<String>,
    calls: String,
    /// The run's scratch directory, kept until the test that made it ends.
    _scratch: Scratch,
}

/// Runs job `name` of `yml` on pull request whose diff is `diff`, `needs` giving the result of the job it needs,
/// and `exprs` the other `${{ … }}` values.
fn run_job(
    yml: &str,
    name: &str,
    tag: &str,
    diff: &str,
    needs: &str,
    exprs: &[(&str, &str)],
) -> Run {
    let scratch = scratch(tag);
    let dir = scratch.to_path_buf();
    let bin = stand_ins(&dir);
    let temp = dir.join("runner_temp");
    std::fs::create_dir_all(&temp).expect("runner temp");
    let diff_file = dir.join("pr.diff");
    std::fs::write(&diff_file, diff).expect("diff");
    let calls = dir.join("calls");
    std::fs::write(&calls, "").expect("calls");
    let xtask = format!("\"{}\"", env!("CARGO_BIN_EXE_xtask"));
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );

    let lines = job(yml, name);
    let mut state = State {
        failed: needs != "success",
        ..State::default()
    };
    let mut run = Run {
        ok: false,
        log: String::new(),
        used: Vec::new(),
        calls: String::new(),
        _scratch: scratch,
    };
    if !holds(job_key(&lines, "if").as_deref(), &state) {
        run.log = format!("job `{name}` did not run (needs: {needs})");
        return run;
    }
    state.failed = false;
    for (i, step) in steps(&lines).iter().enumerate() {
        let id = key(step, "id").unwrap_or_else(|| format!("step{i}"));
        if !holds(key(step, "if").as_deref(), &state) {
            state.outcome.insert(id, "skipped".to_owned());
            continue;
        }
        if let Some(uses) = key(step, "uses") {
            run.used.push(uses);
            state.outcome.insert(id, "success".to_owned());
            continue;
        }
        let mut text = script(step).expect("a step with neither uses nor run");
        for (expr, value) in exprs.iter().chain([("needs.mutants.result", needs)].iter()) {
            text = text.replace(&format!("${{{{ {expr} }}}}"), value);
        }
        assert!(!text.contains("${{"), "unsubstituted expression in {text}");
        let text = text.replace("cargo xtask", &xtask);
        let output = dir.join(format!("github_output_{i}"));
        std::fs::write(&output, "").expect("output");
        let out = Command::new("bash")
            .args(["--noprofile", "--norc", "-eo", "pipefail", "-c", &text])
            .current_dir(root())
            .env("PATH", &path)
            .env("RUNNER_TEMP", &temp)
            .env("GITHUB_OUTPUT", &output)
            .env("STUB_CALLS", &calls)
            .env("STUB_DIFF", &diff_file)
            .timed_output()
            .expect("bash ran");
        run.log += &String::from_utf8_lossy(&out.stdout);
        run.log += &String::from_utf8_lossy(&out.stderr);
        let outputs = std::fs::read_to_string(&output).unwrap_or_default();
        state.outputs.insert(
            id.clone(),
            outputs
                .lines()
                .filter_map(|l| l.split_once('='))
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
                .collect(),
        );
        let outcome = if out.status.success() {
            "success"
        } else {
            "failure"
        };
        state.failed |= !out.status.success();
        state.outcome.insert(id, outcome.to_owned());
    }
    run.ok = !state.failed;
    run.calls = std::fs::read_to_string(&calls).unwrap_or_default();
    run
}

// ---------------------------------------------------------------------------------------------------------------
// The diffs.

/// A rulings PR's diff: docs and plan only.
const DOCS_DIFF: &str = "diff --git a/decisions.md b/decisions.md
--- a/decisions.md
+++ b/decisions.md
@@ -1,1 +1,2 @@
 ## R-1
+A ruling.
diff --git a/docs/README.rst b/docs/README.rst
new file mode 100644
--- /dev/null
+++ b/docs/README.rst
@@ -0,0 +1 @@
+Not Rust.
";

/// A diff that changes Rust source with nothing to mutate: a comment.
const COMMENT_DIFF: &str = "diff --git a/crates/kernel/src/lib.rs b/crates/kernel/src/lib.rs
--- a/crates/kernel/src/lib.rs
+++ b/crates/kernel/src/lib.rs
@@ -1,1 +1,2 @@
 //! The kernel.
+// A comment.
";

/// The setup a shard with no mutant must not pay: every `uses:` but the checkout and the report's upload.
fn setup(used: &[String]) -> Vec<&String> {
    used.iter()
        .filter(|u| {
            !u.starts_with("actions/checkout@") && !u.starts_with("actions/upload-artifact@")
        })
        .collect()
}

fn shard(yml: &str, tag: &str, diff: &str) -> Run {
    run_job(
        yml,
        "mutants",
        tag,
        diff,
        "success",
        &[("matrix.shard", "3"), ("github.base_ref", "main")],
    )
}

/// R-305: on a diff naming no `.rs` file, or on no diff at all, a shard passes saying it has no mutant, having set
/// up nothing and called neither cargo nor the installers; on a diff naming Rust source it sets up and runs
/// `cargo mutants --in-diff` on the diff, and passes when cargo-mutants finds nothing to mutate.
fn shard_passes_with_no_mutant(yml: &str) {
    for (tag, diff) in [("docs", DOCS_DIFF), ("empty", "")] {
        let run = shard(yml, tag, diff);
        println!("mutants shard 3/8, {tag} diff:\n{}", run.log);
        assert!(
            run.ok && run.log.contains("no mutant") && setup(&run.used).is_empty()
                && !run.calls.lines().any(|c| !c.starts_with("git diff")),
            "a shard of a diff naming no Rust source set up or ran more than the diff, or failed ({tag}): \
             setup {:?}, calls {:?}, log {}",
            setup(&run.used),
            run.calls,
            run.log
        );
    }
    let run = shard(yml, "comment", COMMENT_DIFF);
    println!(
        "mutants shard 3/8, Rust comment diff:\n{}calls:\n{}",
        run.log, run.calls
    );
    assert!(
        run.calls.lines().any(|c| c.starts_with("cargo mutants")
            && c.contains("--in-diff")
            && c.contains("--shard 3/8"))
            && !setup(&run.used).is_empty(),
        "a shard of a diff naming Rust source skipped cargo mutants: calls {:?}, log {}",
        run.calls,
        run.log
    );
    assert!(
        run.ok && run.log.contains("no mutant in this shard"),
        "a shard of a Rust diff with nothing to mutate failed: {}",
        run.log
    );
}

#[test]
fn mutants_shard_passes_a_diff_with_no_mutant() {
    shard_passes_with_no_mutant(&mutants_yml());
}

// Read only by the controls.
#[cfg(feature = "controls")]
const GATE: &str =
    "if grep -Eq '^\\+\\+\\+ .*\\.rs\"?[[:space:]]*$' \"$RUNNER_TEMP/pr.diff\"; then";

negative_control!(
    mutants_shard_passes_a_diff_with_no_mutant,
    "a shard that takes every diff for Rust source",
    expected = "a shard of a diff naming no Rust source set up or ran more than the diff",
    {
        let yml = mutants_yml();
        assert!(
            yml.contains(GATE),
            "the control's gate text is not in mutants.yml"
        );
        shard_passes_with_no_mutant(&yml.replace(GATE, "if true; then"))
    }
);

mod never_rust {
    use super::*;
    negative_control!(
        mutants_shard_passes_a_diff_with_no_mutant,
        "a shard that takes no diff for Rust source",
        expected = "a shard of a diff naming Rust source skipped cargo mutants",
        {
            let yml = mutants_yml();
            assert!(
                yml.contains(GATE),
                "the control's gate text is not in mutants.yml"
            );
            shard_passes_with_no_mutant(&yml.replace(GATE, "if false; then"))
        }
    );
}

// ---------------------------------------------------------------------------------------------------------------
// The aggregate, `mutants-check`.

/// R-305: `mutants-check` runs whatever the shards' result, so it always reports; with every shard passed and no
/// shard report (no shard had a mutant, so none uploaded one) it passes, running no check, and with the shards failed,
/// cancelled or skipped it fails.
fn aggregate_reports_with_no_mutant(yml: &str) {
    for needs in ["success", "failure", "cancelled", "skipped"] {
        let run = run_job(
            yml,
            "mutants-check",
            &format!("agg_{needs}"),
            DOCS_DIFF,
            needs,
            &[],
        );
        println!("mutants-check, shards {needs}:\n{}", run.log);
        assert!(
            !run.log.contains("did not run"),
            "mutants-check did not report when the shards' result was {needs}: {}",
            run.log
        );
        if needs == "success" {
            assert!(
                run.ok && run.log.contains("no shard tested a mutant"),
                "mutants-check failed a pull request with no mutant: {}",
                run.log
            );
        } else {
            assert!(
                !run.ok && run.log.contains("::error::"),
                "mutants-check passed with the shards {needs}: {}",
                run.log
            );
        }
    }
}

#[test]
fn mutants_check_reports_and_passes_a_pr_with_no_mutant() {
    aggregate_reports_with_no_mutant(&mutants_yml());
}

// Read only by the controls.
#[cfg(feature = "controls")]
const ANY_REPORT: &str = "if [ \"${#shards[@]}\" -gt 0 ]; then";

negative_control!(
    mutants_check_reports_and_passes_a_pr_with_no_mutant,
    "an aggregate that runs the check over no report",
    expected = "mutants-check failed a pull request with no mutant",
    {
        let yml = mutants_yml();
        assert!(
            yml.contains(ANY_REPORT),
            "the control's text is not in mutants.yml"
        );
        aggregate_reports_with_no_mutant(&yml.replace(ANY_REPORT, "if true; then"))
    }
);

mod not_always {
    use super::*;
    // Read only by the controls.
    #[cfg(feature = "controls")]
    const ALWAYS: &str =
        "  mutants-check:\n    if: always() && github.event_name == 'pull_request'\n";
    negative_control!(
        mutants_check_reports_and_passes_a_pr_with_no_mutant,
        "an aggregate that runs only when every shard passed",
        expected = "mutants-check did not report when the shards' result was failure",
        {
            let yml = mutants_yml();
            assert!(
                yml.contains(ALWAYS),
                "the control's text is not in mutants.yml"
            );
            aggregate_reports_with_no_mutant(&yml.replace(
                ALWAYS,
                "  mutants-check:\n    if: github.event_name == 'pull_request'\n",
            ))
        }
    );
}

// ---------------------------------------------------------------------------------------------------------------
// No run but the pull request's reports `mutants-check`.

/// The events under a workflow's top-level `on:`.
fn events(yml: &str) -> Vec<String> {
    yml.lines()
        .skip_while(|l| l.trim_end() != "on:")
        .skip(1)
        .take_while(|l| l.is_empty() || l.starts_with(' ') || l.starts_with('#'))
        .filter_map(|l| l.strip_prefix("  ").filter(|t| !t.starts_with([' ', '#'])))
        .filter_map(|t| t.split(':').next().map(str::to_owned))
        .collect()
}

/// R-305: `mutants.yml` runs on `pull_request` alone, and no other workflow has a `mutants-check` job, so the only
/// `mutants-check` check run on a commit is the pull request's real one, never a skipped one from a `push` run.
fn only_pull_requests_report(workflows: &[(String, String)]) {
    let (_, mutants) = workflows
        .iter()
        .find(|(name, _)| name == "mutants.yml")
        .expect("mutants.yml");
    let on = events(mutants);
    assert!(
        on == ["pull_request"],
        "mutants.yml runs on events other than pull_request: {on:?}"
    );
    for (name, yml) in workflows {
        assert!(
            name == "mutants.yml" || !yml.lines().any(|l| l.trim_end() == "  mutants-check:"),
            "another workflow, {name}, has a mutants-check job"
        );
    }
}

fn workflows() -> Vec<(String, String)> {
    let dir = root().join(".github/workflows");
    let mut out: Vec<(String, String)> = std::fs::read_dir(&dir)
        .expect("workflows")
        .map(|e| e.expect("entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "yml" || x == "yaml"))
        .map(|p| {
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            (name, std::fs::read_to_string(&p).expect("workflow"))
        })
        .collect();
    out.sort();
    out
}

#[test]
fn mutants_check_is_reported_by_pull_request_runs_only() {
    only_pull_requests_report(&workflows());
}

negative_control!(
    mutants_check_is_reported_by_pull_request_runs_only,
    "mutants.yml also run on push",
    expected = "mutants.yml runs on events other than pull_request",
    only_pull_requests_report(
        &workflows()
            .into_iter()
            .map(|(n, y)| {
                let y = if n == "mutants.yml" {
                    y.replacen("on:\n", "on:\n  push:\n", 1)
                } else {
                    y
                };
                (n, y)
            })
            .collect::<Vec<_>>()
    )
);

mod in_ci {
    use super::*;
    negative_control!(
        mutants_check_is_reported_by_pull_request_runs_only,
        "ci.yml, which push runs, with a mutants-check job",
        expected = "another workflow, ci.yml, has a mutants-check job",
        only_pull_requests_report(
            &workflows()
                .into_iter()
                .map(|(n, y)| {
                    let y = if n == "ci.yml" {
                        y + "\n  mutants-check:\n    if: github.event_name == 'pull_request'\n"
                    } else {
                        y
                    };
                    (n, y)
                })
                .collect::<Vec<_>>()
        )
    );
}
