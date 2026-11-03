//! Mutants on the merge-ready head, and one `reviews` run per burst (R-656, R-676; the owner's words of 2 Nov 2026,
//! items 1 and 4: "The FULL suite plus mutants runs once on the merge-ready head (all required approvals present, or a
//! ready-to-merge label your merge watcher applies), and again only if that head changes", "required check names must
//! report correctly in both tiers (skip-as-success only where the tier logic allows it)", "cancel superseded `reviews`
//! runs per PR").
//!
//! - `merge_ready_mutants_gate_waits_for_the_label`: `mutants.yml` runs on a pull request's new heads and on every label
//!   change, put on or taken off, with no filter; its shards, fixture and verdict run only on the `merge-ready` label's
//!   run, the verdict whatever the shards ended in; one job, `mutants-check`, `if: always()`, reports `mutants-check` in
//!   every run (R-678: GitHub counts only the latest run's check suite, so a check posted into another run's suite is
//!   lost). Its report step, run under bash with stand-in `gh` and `sleep`, concludes on each event: the merge-ready
//!   run's verdict, success only when it passed; "not applicable", passed, on a change with no `.rs` file under
//!   `crates/` (R-677; `mutants=false`, an empty classification not among them) and on a head that is not merge-ready, a
//!   push's included; and on another label put on or taken off a merge-ready head, the verdict of its latest merge-ready
//!   run, waited for while it runs, failing with none, or on a head a push has superseded. `merge-ready-off` runs on a
//!   push alone and takes the label off, a 404 (the label already off) passing and any other error failing. A run on
//!   `main`, a run of another label, put on or taken off, and a reopened pull request's run have groups of their own,
//!   and only a pull request's runs are cancelled.
//! - `merge_ready_reviews_runs_cancel_per_pr_and_event`: `reviews.yml` keeps one cancelling group per pull request and
//!   event, never cancelling on `main`; `reviews-complete` still runs on every run; and the re-run of the stale
//!   `pull_request` run (R-276), run under bash with a stand-in `gh`, re-runs it on the live head and leaves a
//!   superseded head's alone, the head read again after any wait, so a push during the wait is seen.
//!
//! Each test has its registered negative controls (R-176), each feeding the same check a workflow differing in the one
//! respect it turns on. The workflows are parsed with PyYAML, as `xtask/tests/ci_changes.rs` parses them.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
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

fn workflow(name: &str) -> String {
    std::fs::read_to_string(root().join(".github/workflows").join(name))
        .unwrap_or_else(|e| panic!("merge_ready: {name}: {e}"))
}

/// `workflow(name)` with `from` replaced by `to`, which must be in it: a control's one edit.
#[cfg(feature = "controls")]
fn edited(name: &str, from: &str, to: &str) -> String {
    let yml = workflow(name);
    assert!(
        yml.contains(from),
        "merge_ready: the control's text is not in {name}: {from}"
    );
    yml.replace(from, to)
}

/// A workflow's text as JSON, parsed by PyYAML. YAML 1.1 reads the key `on` as `true`; it is put back as `on`.
fn parse(yml: &str) -> Value {
    let dir = Scratch::new("merge_ready_yaml");
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
        .expect("merge_ready: python3 ran");
    assert!(
        out.status.success(),
        "merge_ready: the workflow is not YAML: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("merge_ready: PyYAML's JSON")
}

fn text(v: &Value) -> String {
    match v {
        Value::String(s) => s.split_whitespace().collect::<Vec<_>>().join(" "),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// The step of job `job` whose `name:` is `name`.
fn step<'a>(wf: &'a Value, job: &str, name: &str) -> &'a Value {
    wf["jobs"][job]["steps"]
        .as_array()
        .unwrap_or_else(|| panic!("merge_ready: job `{job}` has no steps"))
        .iter()
        .find(|s| s["name"].as_str() == Some(name))
        .unwrap_or_else(|| panic!("merge_ready: job `{job}` has no step `{name}`"))
}

/// What a script did: whether it passed, and the stand-in `gh`'s calls, one per line.
struct Ran {
    ok: bool,
    calls: String,
    log: String,
}

/// Runs `script` under bash as a workflow step runs, with `env`, and a stand-in `gh` on the `PATH` that logs each call
/// and answers: a pull request's head `$STUB_LIVE` (from its second read on, `$STUB_LIVE_LATER` if set: a push between
/// the reads), and, for `gh run list`, run 42, completed, concluded `$STUB_CONCLUSION`; a DELETE fails with
/// `gh: $STUB_DELETE_ERR` if that is set.
fn run(tag: &str, script: &str, env: &[(&str, &str)]) -> Ran {
    let dir = Scratch::new(&format!("merge_ready_{tag}"));
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).expect("bin");
    let calls = dir.join("calls");
    std::fs::write(&calls, "").expect("calls");
    write_executable(
        &bin.join("gh"),
        "#!/bin/sh\n\
         printf '%s\\n' \"gh $*\" >> \"$STUB_CALLS\"\n\
         case \"$*\" in\n\
           *'--method DELETE'*) [ -z \"$STUB_DELETE_ERR\" ] || { echo \"gh: $STUB_DELETE_ERR\" >&2; exit 1; } ;;\n\
           *'/pulls/'*)\n\
             if [ \"$(grep -c /pulls/ \"$STUB_CALLS\")\" -gt 1 ] && [ -n \"$STUB_LIVE_LATER\" ]; then\n\
               echo \"$STUB_LIVE_LATER\"\n\
             else\n\
               echo \"$STUB_LIVE\"\n\
             fi ;;\n\
           'run list'*'| .databaseId') echo 42 ;;\n\
           'run list'*'| .status') echo completed ;;\n\
           'run list'*'| .conclusion') echo \"$STUB_CONCLUSION\" ;;\n\
         esac\n\
         exit 0\n",
    )
    .expect("gh stand-in");
    let mut cmd = Command::new("bash");
    cmd.args(["--noprofile", "--norc", "-e", "-c", script])
        .env(
            "PATH",
            format!(
                "{}:{}",
                bin.display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .env("STUB_CALLS", &calls)
        .env("GH_REPO", "o/r")
        .env("HEAD_SHA", "abc")
        .env("PR", "7");
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.timed_output().expect("merge_ready: bash ran");
    Ran {
        ok: out.status.success(),
        calls: std::fs::read_to_string(&calls).unwrap_or_default(),
        log: format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    }
}

// ---- mutants.yml ---------------------------------------------------------------------------------------------------

/// The merge-ready run: the `merge-ready` label's, and only that.
const MERGE_READY: [&str; 2] = [
    "github.event.action == 'labeled'",
    "github.event.label.name == 'merge-ready'",
];

/// What `mutants-check`'s report step did on one event: whether it passed, its job summary and its log.
struct Reported {
    ok: bool,
    summary: String,
    log: String,
    calls: String,
}

/// Runs `mutants-check`'s report step under bash with `env`, as the runner does, and stand-ins: `gh` answers the pull
/// request's head `$STUB_LIVE`, this workflow's runs on it `$STUB_RUNS` (newest first), and run n's `mutants-verdict`
/// `$STUB_JOB_<n>`, or, once the step has waited, `$STUB_JOB_<n>_LATER` if set; `sleep` logs the wait and fails on
/// the fourth, so a wait that never ends fails the step.
fn report(tag: &str, script: &str, env: &[(&str, &str)]) -> Reported {
    let dir = Scratch::new(&format!("merge_ready_report_{tag}"));
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).expect("bin");
    let calls = dir.join("calls");
    std::fs::write(&calls, "").expect("calls");
    write_executable(
        &bin.join("gh"),
        "#!/usr/bin/env bash\n\
         printf '%s\\n' \"gh $*\" >> \"$STUB_CALLS\"\n\
         case \"$*\" in\n\
           *'/pulls/'*) echo \"$STUB_LIVE\" ;;\n\
           *'/actions/runs/'*'/jobs'*)\n\
             all=\"$*\"; id=${all#*actions/runs/}; id=${id%%/*}\n\
             var=STUB_JOB_$id; later=STUB_JOB_${id}_LATER\n\
             if grep -q '^sleep' \"$STUB_CALLS\" && [ -n \"${!later:-}\" ]; then echo \"${!later}\"; else echo \"${!var:-}\"; fi ;;\n\
           *'/actions/workflows/mutants.yml/runs'*) [ -z \"$STUB_RUNS\" ] || printf '%s\\n' $STUB_RUNS ;;\n\
           *) echo \"gh stand-in: unexpected call: $*\" >&2; exit 1 ;;\n\
         esac\n",
    )
    .expect("gh stand-in");
    write_executable(
        &bin.join("sleep"),
        "#!/bin/sh\n\
         printf '%s\\n' \"sleep $*\" >> \"$STUB_CALLS\"\n\
         [ \"$(grep -c '^sleep' \"$STUB_CALLS\")\" -lt 4 ]\n",
    )
    .expect("sleep stand-in");
    let summary = dir.join("summary.md");
    let mut cmd = Command::new("bash");
    cmd.args(["--noprofile", "--norc", "-e", "-c", script])
        .env(
            "PATH",
            format!(
                "{}:{}",
                bin.display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .env("STUB_CALLS", &calls)
        .env("GITHUB_STEP_SUMMARY", &summary)
        .env("GH_REPO", "o/r")
        .env("HEAD_SHA", "abc")
        .env("STUB_LIVE", "abc")
        .env("STUB_RUNS", "")
        .env("PR", "7")
        .env("RUN_ID", "9")
        .env("LABEL", "")
        .env("MUTANTS", "true")
        .env("VERDICT", "skipped");
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.timed_output().expect("merge_ready: bash ran");
    Reported {
        ok: out.status.success(),
        summary: std::fs::read_to_string(&summary).unwrap_or_default(),
        log: format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
        calls: std::fs::read_to_string(&calls).unwrap_or_default(),
    }
}

/// An event's context and the stand-ins' answers, as `(variable, value)`.
type Env = &'static [(&'static str, &'static str)];

/// `mutants-check` on each event: (what, its env, whether it passes, what its summary says).
const REPORTS: &[(&str, Env, bool, &str)] = &[
    (
        "the merge-ready run, its verdict passed",
        &[("ACTION", "labeled"), ("LABEL", "merge-ready"), ("READY", "true"), ("MUTANTS", ""), ("VERDICT", "success")],
        true,
        "no surviving mutant",
    ),
    (
        "the merge-ready run, its verdict failed",
        &[("ACTION", "labeled"), ("LABEL", "merge-ready"), ("READY", "true"), ("MUTANTS", ""), ("VERDICT", "failure")],
        false,
        "a survivor, or a shard that did not finish (failure)",
    ),
    (
        "the merge-ready run, its verdict skipped",
        &[("ACTION", "labeled"), ("LABEL", "merge-ready"), ("READY", "true"), ("MUTANTS", "false"), ("VERDICT", "skipped")],
        false,
        "(skipped)",
    ),
    (
        "the merge-ready run, cancelled",
        &[("ACTION", "labeled"), ("LABEL", "merge-ready"), ("READY", "true"), ("VERDICT", "cancelled")],
        false,
        "(cancelled)",
    ),
    (
        "a push with no Rust under crates/",
        &[("ACTION", "synchronize"), ("READY", "false"), ("MUTANTS", "false")],
        true,
        "touches no Rust source under crates/",
    ),
    (
        "a push of code",
        &[("ACTION", "synchronize"), ("READY", "false")],
        true,
        "The head is not merge-ready",
    ),
    (
        "a push of code to a merge-ready head, which takes the label off",
        &[("ACTION", "synchronize"), ("READY", "true"), ("STUB_RUNS", "5"), ("STUB_JOB_5", "completed failure")],
        true,
        "The head is not merge-ready",
    ),
    (
        "a push the classifier said nothing of",
        &[("ACTION", "synchronize"), ("READY", "false"), ("MUTANTS", "")],
        true,
        "The head is not merge-ready",
    ),
    (
        "code, opened",
        &[("ACTION", "opened"), ("READY", "false")],
        true,
        "The head is not merge-ready",
    ),
    (
        "merge-ready taken off",
        &[("ACTION", "unlabeled"), ("LABEL", "merge-ready"), ("READY", "false"), ("VERDICT", "skipped")],
        true,
        "The head is not merge-ready",
    ),
    (
        "another label on a code head that is not merge-ready",
        &[("ACTION", "labeled"), ("LABEL", "design"), ("READY", "false")],
        true,
        "The head is not merge-ready",
    ),
    (
        "another label on a merge-ready head with no Rust under crates/",
        &[("ACTION", "labeled"), ("LABEL", "design"), ("READY", "true"), ("MUTANTS", "false")],
        true,
        "touches no Rust source under crates/",
    ),
    (
        "another label on a merge-ready head, its merge-ready run passed",
        &[
            ("ACTION", "labeled"), ("LABEL", "design"), ("READY", "true"),
            ("STUB_RUNS", "9 8 7 6"), ("STUB_JOB_8", "completed skipped"), ("STUB_JOB_7", "completed success"),
            ("STUB_JOB_6", "completed failure"),
        ],
        true,
        "no surviving mutant",
    ),
    (
        "another label taken off a merge-ready head, its merge-ready run failed",
        &[
            ("ACTION", "unlabeled"), ("LABEL", "design"), ("READY", "true"),
            ("STUB_RUNS", "9 8 7 6"), ("STUB_JOB_8", "completed skipped"), ("STUB_JOB_7", "completed failure"),
            ("STUB_JOB_6", "completed success"),
        ],
        false,
        "a survivor, or a shard that did not finish (failure)",
    ),
    (
        "another label on a merge-ready head the classifier said nothing of, its merge-ready run failed",
        &[
            ("ACTION", "labeled"), ("LABEL", "design"), ("READY", "true"), ("MUTANTS", ""),
            ("STUB_RUNS", "7"), ("STUB_JOB_7", "completed failure"),
        ],
        false,
        "(failure)",
    ),
    (
        "another label on a merge-ready head, its merge-ready run cancelled",
        &[
            ("ACTION", "labeled"), ("LABEL", "design"), ("READY", "true"),
            ("STUB_RUNS", "7"), ("STUB_JOB_7", "completed cancelled"),
        ],
        false,
        "(cancelled)",
    ),
    (
        "another label on a merge-ready head, its merge-ready run still going, then passed",
        &[
            ("ACTION", "labeled"), ("LABEL", "design"), ("READY", "true"),
            ("STUB_RUNS", "7"), ("STUB_JOB_7", "in_progress null"), ("STUB_JOB_7_LATER", "completed success"),
        ],
        true,
        "no surviving mutant",
    ),
    (
        "another label on a merge-ready head, a newer run not started yet, then skipped",
        &[
            ("ACTION", "labeled"), ("LABEL", "design"), ("READY", "true"),
            ("STUB_RUNS", "10 7"), ("STUB_JOB_10", ""), ("STUB_JOB_10_LATER", "completed skipped"),
            ("STUB_JOB_7", "completed success"),
        ],
        true,
        "no surviving mutant",
    ),
    (
        "another label on a merge-ready head, its merge-ready run never ending",
        &[
            ("ACTION", "labeled"), ("LABEL", "design"), ("READY", "true"),
            ("STUB_RUNS", "7"), ("STUB_JOB_7", "queued null"),
        ],
        false,
        "",
    ),
    (
        "another label on a merge-ready head with no merge-ready run",
        &[
            ("ACTION", "labeled"), ("LABEL", "design"), ("READY", "true"),
            ("STUB_RUNS", "9 8"), ("STUB_JOB_8", "completed skipped"),
        ],
        false,
        "no merge-ready run",
    ),
    (
        "another label on a merge-ready head a push has superseded",
        &[
            ("ACTION", "labeled"), ("LABEL", "design"), ("READY", "true"), ("STUB_LIVE", "def"),
            ("STUB_RUNS", "7"), ("STUB_JOB_7", "completed success"),
        ],
        false,
        "superseded",
    ),
];

fn check_gate(yml: &str) {
    let wf = parse(yml);
    let pr = &wf["on"]["pull_request"];
    let types: Vec<String> = pr["types"]
        .as_array()
        .map(|a| a.iter().map(text).collect())
        .unwrap_or_default();
    for t in ["opened", "synchronize", "reopened", "labeled", "unlabeled"] {
        assert!(
            types.iter().any(|x| x == t),
            "merge_ready: mutants.yml does not run on `{t}`: {types:?}"
        );
    }
    for key in ["paths", "paths-ignore", "branches", "branches-ignore"] {
        assert!(
            pr.get(key).is_none(),
            "merge_ready: mutants.yml filters its pull requests with `{key}:`"
        );
    }

    // `mutants-check` is a job of every run, never skipped (R-678): GitHub counts only the latest run's check suite.
    let jobs = wf["jobs"].as_object().expect("merge_ready: jobs");
    let bearing: Vec<&String> = jobs
        .iter()
        .filter(|(id, job)| {
            job.get("name").map(text).unwrap_or_else(|| (*id).clone()) == "mutants-check"
        })
        .map(|(id, _)| id)
        .collect();
    assert!(
        bearing == ["mutants-check"],
        "merge_ready: not one job `mutants-check` reports mutants-check: {bearing:?}"
    );
    let check = &jobs["mutants-check"];
    assert!(
        text(&check["if"]) == "always()" && text(&check["needs"]) == "mutants-verdict",
        "merge_ready: mutants-check can be skipped, or does not wait for the verdict: if `{}`, needs `{}`",
        text(&check["if"]),
        text(&check["needs"])
    );
    for id in ["mutants", "mutants-fixture", "mutants-verdict"] {
        let cond = text(&jobs[id]["if"]);
        assert!(
            MERGE_READY.iter().all(|t| cond.contains(t)) && !cond.contains("||"),
            "merge_ready: job `{id}` runs without the merge-ready label: `{cond}`"
        );
    }
    assert!(
        text(&jobs["mutants-verdict"]["if"]).starts_with("always()"),
        "merge_ready: the verdict does not run whatever the shards ended in"
    );
    assert_eq!(
        text(&jobs["merge-ready-off"]["if"]),
        "github.event_name == 'pull_request' && github.event.action == 'synchronize'",
        "merge_ready: merge-ready-off does not run on exactly a push's run"
    );

    let group = text(&wf["concurrency"]["group"]);
    assert!(
        group.contains("github.ref == 'refs/heads/main'")
            && group.contains(
                "((github.event.action == 'labeled' || github.event.action == 'unlabeled') && \
                 github.event.label.name != 'merge-ready'))"
            )
            && group.contains("&& github.run_id || github.ref"),
        "merge_ready: mutants.yml's group does not keep `main`'s runs and other labels' runs apart: `{group}`"
    );
    // A reopened pull request's run changes no head: in the ref's group it would cancel the merge-ready run.
    assert!(
        group.contains("(github.ref == 'refs/heads/main' || github.event.action == 'reopened' || ("),
        "merge_ready: a reopened pull request's run shares the merge-ready run's group, so it cancels it: `{group}`"
    );
    assert_eq!(
        text(&wf["concurrency"]["cancel-in-progress"]),
        "${{ github.ref != 'refs/heads/main' }}",
        "merge_ready: mutants.yml cancels runs on `main`, or no pull request's"
    );

    // What `mutants-check` concludes on each event.
    let report_step = step(&wf, "mutants-check", "Report mutants-check");
    for (key, want) in [
        ("ACTION", "${{ github.event.action }}"),
        ("LABEL", "${{ github.event.label.name }}"),
        (
            "READY",
            "${{ contains(github.event.pull_request.labels.*.name, 'merge-ready') }}",
        ),
        ("MUTANTS", "${{ steps.changes.outputs.mutants }}"),
        ("VERDICT", "${{ needs.mutants-verdict.result }}"),
        ("HEAD_SHA", "${{ github.event.pull_request.head.sha }}"),
        ("RUN_ID", "${{ github.run_id }}"),
    ] {
        assert_eq!(
            text(&report_step["env"][key]),
            want,
            "merge_ready: mutants-check's report does not read `{key}` from the event"
        );
    }
    let script = report_step["run"].as_str().expect("run");
    for (i, (what, env, ok, says)) in REPORTS.iter().enumerate() {
        let got = report(&format!("{i}"), script, env);
        assert!(
            got.ok == *ok && got.summary.contains(says),
            "merge_ready: mutants-check {} on {what}, summary `{}`, not {} with `{says}`: {} {}",
            if got.ok { "passed" } else { "failed" },
            got.summary.trim(),
            if *ok { "passed" } else { "failed" },
            got.log,
            got.calls
        );
        assert!(
            !got.summary.is_empty() || what.contains("never ending"),
            "merge_ready: mutants-check on {what} wrote no summary: {}",
            got.log
        );
    }

    // A push takes the label off.
    let drop = step(
        &wf,
        "merge-ready-off",
        "Take the merge-ready label off the pushed pull request",
    );
    let cond = text(&drop["if"]);
    assert!(
        cond.contains("github.event.action == 'synchronize'")
            && cond.contains("contains(github.event.pull_request.labels.*.name, 'merge-ready')")
            && !cond.contains("||"),
        "merge_ready: the label is not taken off on a push: `{cond}`"
    );
    // GitHub's answer to the DELETE: none (the label came off), a 404 (an earlier push's run took it off: the label is
    // off, so the step passes), or any other error, which fails the step.
    let drop = drop["run"].as_str().expect("run");
    for (err, ok, case) in [
        ("", true, "the label taken off"),
        (
            "Label does not exist (HTTP 404)",
            true,
            "a 404, the label already off",
        ),
        (
            "Resource not accessible by integration (HTTP 403)",
            false,
            "a 403",
        ),
        ("Server Error (HTTP 500)", false, "a 500"),
    ] {
        let ran = run(
            &format!("drop_{}", err.len()),
            drop,
            &[("STUB_DELETE_ERR", err)],
        );
        assert!(
            ran.calls
                .contains("gh api --method DELETE repos/o/r/issues/7/labels/merge-ready"),
            "merge_ready: the push's run did not take the label off: {}",
            ran.calls
        );
        assert!(
            ran.ok == ok,
            "merge_ready: the label step {} on {case}: {}",
            if ran.ok { "passed" } else { "failed" },
            ran.log
        );
    }
}

#[test]
fn merge_ready_mutants_gate_waits_for_the_label() {
    check_gate(&workflow("mutants.yml"));
}

negative_control!(
    merge_ready_mutants_gate_waits_for_the_label,
    "mutants-check posted through the Checks API again, no job bearing it, which a later run's suite supersedes (#384)",
    expected = "not one job `mutants-check` reports mutants-check",
    check_gate(&edited(
        "mutants.yml",
        "  mutants-check:\n",
        "  mutants-report:\n"
    ))
);

mod check_skippable {
    use super::*;
    negative_control!(
        merge_ready_mutants_gate_waits_for_the_label,
        "mutants-check run only on the merge-ready run, so every other run reports it skipped",
        expected = "mutants-check can be skipped, or does not wait for the verdict",
        check_gate(&edited(
            "mutants.yml",
            "  mutants-check:\n    if: always()\n",
            "  mutants-check:\n    if: always() && github.event.label.name == 'merge-ready'\n"
        ))
    );
}

mod no_unlabeled {
    use super::*;
    negative_control!(
        merge_ready_mutants_gate_waits_for_the_label,
        "a workflow not run when a label is taken off",
        expected = "mutants.yml does not run on `unlabeled`",
        check_gate(&edited(
            "mutants.yml",
            "    types: [opened, synchronize, reopened, labeled, unlabeled]\n",
            "    types: [opened, synchronize, reopened, labeled]\n"
        ))
    );
}

mod shards_every_push {
    use super::*;
    negative_control!(
        merge_ready_mutants_gate_waits_for_the_label,
        "shards that run on every head, not only the merge-ready one",
        expected = "job `mutants` runs without the merge-ready label",
        check_gate(&edited(
            "mutants.yml",
            "  mutants:\n    if: github.event_name == 'pull_request' && github.event.action == 'labeled' && \
             github.event.label.name == 'merge-ready'\n",
            "  mutants:\n    if: github.event_name == 'pull_request'\n"
        ))
    );
}

mod verdict_always_passes {
    use super::*;
    negative_control!(
        merge_ready_mutants_gate_waits_for_the_label,
        "a mutants-check passed whatever the verdict",
        expected = "mutants-check passed on the merge-ready run, its verdict failed",
        check_gate(&edited(
            "mutants.yml",
            "            if [ \"$1\" = success ]; then\n",
            "            if true; then\n"
        ))
    );
}

mod verdict_unread {
    use super::*;
    negative_control!(
        merge_ready_mutants_gate_waits_for_the_label,
        "another label's run passing a merge-ready head whatever its merge-ready run's verdict",
        expected = "mutants-check passed on another label taken off a merge-ready head, its merge-ready run failed",
        check_gate(&edited(
            "mutants.yml",
            "                \"completed \"*) verdict \"${state#completed }\" \"run $id\" ;;\n",
            "                \"completed \"*) verdict success \"run $id\" ;;\n"
        ))
    );
}

mod no_wait {
    use super::*;
    negative_control!(
        merge_ready_mutants_gate_waits_for_the_label,
        "another label's run that passes over a merge-ready run still going, as if it had none",
        expected = "mutants-check failed on another label on a merge-ready head, its merge-ready run still going",
        check_gate(&edited(
            "mutants.yml",
            "                *) wait=\"run $id: ${state:-no jobs yet}\"; break ;;\n",
            "                *) continue ;;\n"
        ))
    );
}

mod superseded_unseen {
    use super::*;
    negative_control!(
        merge_ready_mutants_gate_waits_for_the_label,
        "another label's run that reports a merge-ready verdict on a head a push has superseded",
        expected =
            "mutants-check passed on another label on a merge-ready head a push has superseded",
        check_gate(&edited(
            "mutants.yml",
            "            if [ \"$live\" != \"$HEAD_SHA\" ]; then\n",
            "            if false; then\n"
        ))
    );
}

mod push_follows {
    use super::*;
    negative_control!(
        merge_ready_mutants_gate_waits_for_the_label,
        "a push's run that takes the head for merge-ready because the push's payload still lists the label",
        expected = "mutants-check failed on a push of code to a merge-ready head, which takes the label off",
        check_gate(&edited(
            "mutants.yml",
            "          if [ \"$ACTION\" = synchronize ] || [ \"$READY\" != true ]; then\n",
            "          if [ \"$READY\" != true ]; then\n"
        ))
    );
}

mod empty_classification {
    use super::*;
    negative_control!(
        merge_ready_mutants_gate_waits_for_the_label,
        "a classification that said nothing taken for one with no Rust under crates/",
        expected = "mutants-check passed on a push the classifier said nothing of",
        check_gate(&edited(
            "mutants.yml",
            "          if [ \"$MUTANTS\" = false ]; then\n",
            "          if [ \"$MUTANTS\" != true ]; then\n"
        ))
    );
}

mod label_kept {
    use super::*;
    negative_control!(
        merge_ready_mutants_gate_waits_for_the_label,
        "a push that leaves the merge-ready label on",
        expected = "the push's run did not take the label off",
        check_gate(&edited(
            "mutants.yml",
            "          if err=$(gh api --method DELETE \"repos/$GH_REPO/issues/$PR/labels/merge-ready\" 2>&1 > \
             /dev/null); then\n",
            "          if true; then\n"
        ))
    );
}

mod label_404_fails {
    use super::*;
    negative_control!(
        merge_ready_mutants_gate_waits_for_the_label,
        "a label step that fails on a 404, so a second quick push's run fails",
        expected = "the label step failed on a 404, the label already off",
        check_gate(&edited(
            "mutants.yml",
            "          elif grep -q 'HTTP 404' <<< \"$err\"; then\n",
            "          elif false; then\n"
        ))
    );
}

mod label_errors_pass {
    use super::*;
    negative_control!(
        merge_ready_mutants_gate_waits_for_the_label,
        "a label step that passes on any error",
        expected = "the label step passed on a 403",
        check_gate(&edited(
            "mutants.yml",
            "          elif grep -q 'HTTP 404' <<< \"$err\"; then\n",
            "          elif true; then\n"
        ))
    );
}

mod other_labels_cancel {
    use super::*;
    negative_control!(
        merge_ready_mutants_gate_waits_for_the_label,
        "another label taken off in the pull request's group, so it cancels the merge-ready run",
        expected = "group does not keep `main`'s runs and other labels' runs apart",
        check_gate(&edited(
            "mutants.yml",
            "(github.event.action == 'labeled' || github.event.action == 'unlabeled')",
            "github.event.action == 'labeled'"
        ))
    );
}

mod reopened_cancels {
    use super::*;
    negative_control!(
        merge_ready_mutants_gate_waits_for_the_label,
        "a reopened pull request's run in the ref's group, so reopening mid-run cancels the merge-ready run",
        expected = "a reopened pull request's run shares the merge-ready run's group",
        check_gate(&edited(
            "mutants.yml",
            "    || github.event.action == 'reopened'\n",
            ""
        ))
    );
}

// ---- reviews.yml ---------------------------------------------------------------------------------------------------

fn check_reviews(yml: &str) {
    let wf = parse(yml);
    let group = text(&wf["concurrency"]["group"]);
    assert!(
        group.contains("${{ github.event_name }}"),
        "merge_ready: reviews.yml's group is one for both events, so a review's re-run cancels its own run: `{group}`"
    );
    assert!(
        group.contains("github.ref == 'refs/heads/main' && github.run_id || github.ref"),
        "merge_ready: reviews.yml's group is not the pull request's, or `main`'s runs share one: `{group}`"
    );
    assert_eq!(
        text(&wf["concurrency"]["cancel-in-progress"]),
        "${{ github.ref != 'refs/heads/main' }}",
        "merge_ready: reviews.yml's superseded review runs are not cancelled, or `main`'s are"
    );
    let job = &wf["jobs"]["reviews-complete"];
    assert!(
        job.get("if").is_none() && job.get("needs").is_none() && job.get("name").is_none(),
        "merge_ready: reviews-complete can be skipped or renamed"
    );

    let rerun = step(
        &wf,
        "reviews-complete",
        "re-run the stale pull_request run for this head",
    )["run"]
        .as_str()
        .expect("run");
    for (live, later, conclusion, reruns, case) in [
        ("abc", "", "failure", true, "the live head's failed run"),
        (
            "abc",
            "abc",
            "failure",
            true,
            "the live head's failed run, read twice",
        ),
        ("abc", "", "success", false, "the live head's passed run"),
        ("def", "", "failure", false, "a superseded head's run"),
        (
            "abc",
            "def",
            "cancelled",
            false,
            "a head superseded during the wait",
        ),
    ] {
        let ran = run(
            &format!("rerun_{live}_{later}_{conclusion}"),
            rerun,
            &[
                ("STUB_LIVE", live),
                ("STUB_LIVE_LATER", later),
                ("STUB_CONCLUSION", conclusion),
            ],
        );
        let did = ran.calls.contains("gh run rerun 42");
        assert!(
            ran.ok && did == reruns,
            "merge_ready: the review's run {} {case}: {} {}",
            if did { "re-ran" } else { "did not re-run" },
            ran.calls,
            ran.log
        );
    }
}

#[test]
fn merge_ready_reviews_runs_cancel_per_pr_and_event() {
    check_reviews(&workflow("reviews.yml"));
}

negative_control!(
    merge_ready_reviews_runs_cancel_per_pr_and_event,
    "one group for both events",
    expected = "group is one for both events",
    check_reviews(&edited(
        "reviews.yml",
        "  group: reviews-${{ github.event_name }}-",
        "  group: reviews-"
    ))
);

mod no_cancel {
    use super::*;
    negative_control!(
        merge_ready_reviews_runs_cancel_per_pr_and_event,
        "a group that queues review runs rather than cancelling them",
        expected = "superseded review runs are not cancelled",
        check_reviews(&edited(
            "reviews.yml",
            "  cancel-in-progress: ${{ github.ref != 'refs/heads/main' }}\n",
            "  cancel-in-progress: false\n"
        ))
    );
}

mod superseded_rerun {
    use super::*;
    negative_control!(
        merge_ready_reviews_runs_cancel_per_pr_and_event,
        "a review's run that re-runs its head's run after a push superseded that head",
        expected = "re-ran a superseded head's run",
        check_reviews(&edited(
            "reviews.yml",
            "          if [ \"$live\" != \"$HEAD_SHA\" ]; then\n",
            "          if false; then\n"
        ))
    );
}

mod superseded_during_wait {
    use super::*;
    negative_control!(
        merge_ready_reviews_runs_cancel_per_pr_and_event,
        "a review's run that reads the head only before the wait, so a push during it goes unseen",
        expected = "re-ran a head superseded during the wait",
        check_reviews(&edited(
            "reviews.yml",
            "          # The head again, now, after the wait.\n          live=$(live_head)\n",
            "          # The head again, now, after the wait.\n"
        ))
    );
}
