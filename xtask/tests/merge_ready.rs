//! Mutants on the merge-ready head, and one `reviews` run per burst (R-656, R-676; the owner's words of 2 Nov 2026,
//! items 1 and 4: "The FULL suite plus mutants runs once on the merge-ready head (all required approvals present, or a
//! ready-to-merge label your merge watcher applies), and again only if that head changes", "required check names must
//! report correctly in both tiers (skip-as-success only where the tier logic allows it)", "cancel superseded `reviews`
//! runs per PR").
//!
//! - `merge_ready_mutants_gate_waits_for_the_label`: `mutants.yml` runs on a pull request's new heads and on its
//!   labels, with no filter; its shards, fixture and verdict run only on the `merge-ready` label's run, the verdict
//!   whatever the shards ended in; no job bears the name `mutants-check`, so no skipped job reports it;
//!   `mutants-pending` runs on every other run that is not a label's. Its scripts, run under bash with a stand-in `gh`,
//!   post a new head's `mutants-check` queued, or passed only on a docs-only head, and leave one already there alone; a
//!   push takes the `merge-ready` label off, a 404 (the label already off) passing and any other error failing; the
//!   verdict completes every `mutants-check` on the head, success only when the verdict step passed, and posts one if
//!   there is none. A run on `main` and a run of another label have groups of their own, and only a pull request's runs
//!   are cancelled.
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
/// and answers: a check-run count `$STUB_COUNT`, check-run ids `$STUB_IDS`, a pull request's head `$STUB_LIVE` (from
/// its second read on, `$STUB_LIVE_LATER` if set: a push between the reads), and, for `gh run list`, run 42, completed,
/// concluded `$STUB_CONCLUSION`; a DELETE fails with `gh: $STUB_DELETE_ERR` if that is set.
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
           *total_count*) echo \"$STUB_COUNT\" ;;\n\
           *'check_runs[].id'*) [ -z \"$STUB_IDS\" ] || printf '%s\\n' $STUB_IDS ;;\n\
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

fn check_gate(yml: &str) {
    let wf = parse(yml);
    let pr = &wf["on"]["pull_request"];
    let types: Vec<String> = pr["types"]
        .as_array()
        .map(|a| a.iter().map(text).collect())
        .unwrap_or_default();
    for t in ["opened", "synchronize", "reopened", "labeled"] {
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

    let jobs = wf["jobs"].as_object().expect("merge_ready: jobs");
    for (id, job) in jobs {
        let name = job.get("name").map(text).unwrap_or_else(|| id.clone());
        assert!(
            name != "mutants-check",
            "merge_ready: a job reports mutants-check ({id}), which a run it skips reports passed"
        );
    }
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
    let pending = text(&jobs["mutants-pending"]["if"]);
    assert!(
        pending.contains("github.event.action != 'labeled'") && !pending.contains("||"),
        "merge_ready: mutants-pending does not run on exactly the runs that are not a label's: `{pending}`"
    );

    let group = text(&wf["concurrency"]["group"]);
    assert!(
        group.contains("github.ref == 'refs/heads/main'")
            && group.contains("github.event.label.name != 'merge-ready'")
            && group.contains("&& github.run_id || github.ref"),
        "merge_ready: mutants.yml's group does not keep `main`'s runs and other labels' runs apart: `{group}`"
    );
    assert_eq!(
        text(&wf["concurrency"]["cancel-in-progress"]),
        "${{ github.ref != 'refs/heads/main' }}",
        "merge_ready: mutants.yml cancels runs on `main`, or no pull request's"
    );

    // The new head's `mutants-check`.
    let post = step(&wf, "mutants-pending", "Post mutants-check on the head")["run"]
        .as_str()
        .expect("run");
    for (docs, count, want) in [
        ("false", "0", "queued"),
        ("true", "0", "success"),
        ("false", "1", "none"),
        ("true", "1", "none"),
    ] {
        let ran = run(
            &format!("pending_{docs}_{count}"),
            post,
            &[("DOCS_ONLY", docs), ("STUB_COUNT", count)],
        );
        let posts: Vec<&str> = ran
            .calls
            .lines()
            .filter(|c| c.contains("--method POST"))
            .collect();
        let got = match posts.as_slice() {
            [] => "none",
            [one] if one.contains("conclusion=success") => "success",
            [one] if one.contains("status=queued") && !one.contains("conclusion") => "queued",
            _ => "other",
        };
        assert!(
            ran.ok && got == want,
            "merge_ready: posted mutants-check {got}, not {want}, on a head docs-only {docs} with {count} already \
             there: {} {}",
            ran.calls,
            ran.log
        );
        assert!(
            posts
                .iter()
                .all(|p| p.contains("name=mutants-check") && p.contains("head_sha=abc")),
            "merge_ready: posted a check other than the head's mutants-check: {posts:?}"
        );
    }

    // A push takes the label off.
    let drop = step(
        &wf,
        "mutants-pending",
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
    // off, so the step passes and the head's check is still posted), or any other error, which fails the step.
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

    // The verdict, on every `mutants-check` of the head.
    let verdict = step(
        &wf,
        "mutants-verdict",
        "Post mutants-check's verdict on the head",
    );
    assert_eq!(
        text(&verdict["if"]),
        "always()",
        "merge_ready: the verdict is not posted whatever the run ended in"
    );
    let verdict = verdict["run"].as_str().expect("run");
    for (outcome, ids, want) in [
        ("success", "11 12", "success"),
        ("failure", "11 12", "failure"),
        ("skipped", "11 12", "failure"),
        ("cancelled", "11", "failure"),
        ("success", "", "success"),
        ("failure", "", "failure"),
    ] {
        let ran = run(
            &format!("verdict_{outcome}"),
            verdict,
            &[
                ("VERDICT", outcome),
                ("STUB_IDS", ids),
                ("RUN_URL", "https://example/run"),
            ],
        );
        let writes: Vec<&str> = ran
            .calls
            .lines()
            .filter(|c| c.contains("--method"))
            .collect();
        let expect: Vec<String> = if ids.is_empty() {
            vec!["--method POST repos/o/r/check-runs".to_owned()]
        } else {
            ids.split(' ')
                .map(|id| format!("--method PATCH repos/o/r/check-runs/{id}"))
                .collect()
        };
        assert!(
            ran.ok
                && writes.len() == expect.len()
                && writes.iter().zip(&expect).all(|(w, e)| w.contains(e.as_str())
                    && w.contains("status=completed")
                    && w.contains(&format!("conclusion={want}"))),
            "merge_ready: the verdict step, its step {outcome}, did not complete each mutants-check {want}: {} {}",
            ran.calls,
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
    "the verdict job named mutants-check again, so a run that skips it reports it passed",
    expected = "a job reports mutants-check (mutants-check)",
    check_gate(&edited(
        "mutants.yml",
        "  mutants-verdict:\n",
        "  mutants-check:\n"
    ))
);

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

mod passed_unrun {
    use super::*;
    negative_control!(
        merge_ready_mutants_gate_waits_for_the_label,
        "a new head's mutants-check posted passed before any mutant ran",
        expected = "posted mutants-check success, not queued",
        check_gate(&edited(
            "mutants.yml",
            "              -f status=queued\n",
            "              -f status=completed -f conclusion=success\n"
        ))
    );
}

mod verdict_always_passes {
    use super::*;
    negative_control!(
        merge_ready_mutants_gate_waits_for_the_label,
        "a verdict posted passed whatever its step ended in",
        expected = "its step failure, did not complete each mutants-check failure",
        check_gate(&edited(
            "mutants.yml",
            "          if [ \"$VERDICT\" = success ]; then\n",
            "          if true; then\n"
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
        "a label step that fails on a 404, so a second quick push's head gets no mutants-check",
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
        "every label's run in the pull request's group, so a `design` label cancels the merge-ready run",
        expected = "group does not keep `main`'s runs and other labels' runs apart",
        check_gate(&edited(
            "mutants.yml",
            "\n    || (github.event.action == 'labeled' && github.event.label.name != 'merge-ready'))\n",
            ")\n"
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
