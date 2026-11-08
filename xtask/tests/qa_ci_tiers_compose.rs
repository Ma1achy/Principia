//! QA's tests for the CI tooling change "the full tier adds to the quick tier" (no task id; the owner, 7 Nov 2026:
//! "Yeah if adding the label cancels the quick tier that needs to be changed lol if not don't worry / The full tier
//! should compose / add on to the quick tier"), as the owner's order of 8 Nov 2026 amends its waiting clause ("ci-full:
//! on merge-ready, cancel the in-progress quick run and start the full tier, instead of waiting for it"; PR #450),
//! written from those orders and the rulings they must keep, not from the implementation:
//!
//! - `qa_compose_label_mid_quick_cancels_it_at_once`: `merge-ready` going on while the head's quick attempt is in
//!   progress cancels it at once, on the first look at the run, and starts the full attempt (`/rerun`, R-276's
//!   pattern kept by R-678) once the cancelled attempt has been seen ended (the owner, 8 Nov 2026: "ci-full: on
//!   merge-ready, cancel the in-progress quick run and start the full tier, instead of waiting for it"); a quick attempt
//!   no cancel ends is never re-run over, the job failing. An attempt whose `changes` job has not decided its tier is
//!   read again until it decides (the orchestrator's decision on code review 5819591377, R-369): decided quick, it is
//!   then cancelled at once; never deciding, the job fails, named, cancelling nothing.
//! - `qa_compose_label_acts_only_on_a_shown_tier`: by the same decision, an attempt whose `changes` decides full after
//!   some reads is waited for, never cancelled; the reads spent waiting for `changes` come out of the full attempt's
//!   own bounded wait; an attempt whose jobs cannot be read is read again, a bounded number of times (30), then the job
//!   fails, named, cancelling nothing.
//! - `qa_compose_label_mid_full_waits_never_cancels`: a full attempt still going (a retry's, or a first attempt opened
//!   with the label) is the full tier already: it is waited for, never cancelled, and re-run only once ended.
//! - `qa_compose_a_push_still_takes_the_label_off`: a push still takes `merge-ready` off (`mutants.yml`), the pushed
//!   head's first attempt is quick whatever the label says, a superseded head's quick attempt or one whose label is
//!   already off is never cancelled, and a push or the label coming off during a wait starts no full attempt on the old
//!   head.
//! - `qa_compose_label_never_cancels_a_run_on_main`: neither the label going on nor going off cancels a run on `main`
//!   (R-656).
//! - `qa_compose_evidence_only_from_an_earlier_passed_quick_attempt`: what `ci_tier.sh decide` carries into the full
//!   attempt is shown only by an earlier attempt of the same run whose quick job concluded `success`; it covers the `ci`
//!   shards only when that quick tier tested every crate; nothing unread counts. The stand-in `gh` answers the jobs API
//!   in its own JSON shape and applies the script's own `--jq` filter with `jq`, so the filter is under test too.
//! - `qa_compose_gate_ci_passes_a_skip_only_on_proof`: the gate that reports the required check `ci` is evaluated from
//!   `ci.yml` itself (its `${{ }}` expressions by a small evaluator here, its `run:` under bash): it passes the shards'
//!   skip only on `quick_covers_ci`, never a skip without it, never a second skip, never a failure (R-678, R-684: no
//!   required check passes on a skip).
//! - `qa_compose_every_required_full_only_check_still_reports`: on the composed full attempt every full-only required
//!   check's gate still runs (its `if:` evaluates true), the shards skip only on `quick_covers_ci` (not on
//!   `quick_passed` alone), and what quick never runs (cargo doc, the crate graph, the controls listing) still runs.
//!
//! Each test has its registered negative control (R-176), which runs the same check on a script or workflow edited in
//! the one respect it turns on. Unix only: the stand-ins are executable scripts found through `PATH`. Needs `jq` and
//! PyYAML, as the CI runners have.
#![cfg(unix)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use validation::negative_control;
use validation::spawn::{write_executable, Spawn};

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;

const TIER: &str = ".github/scripts/ci_tier.sh";
const CRATES: &str = ".github/scripts/ci_quick_crates.py";
const CI: &str = ".github/workflows/ci.yml";
const MUTANTS: &str = ".github/workflows/mutants.yml";
/// The quick tier's job, as the jobs API names a called workflow's job: `<caller job> / <called job>`.
const QUICK_JOB: &str = "ci-quick / ci-quick";
/// The required checks only the full tier reports (R-684; branch protection's list, `plan/OPERATIONS.md`).
const FULL_ONLY: [&str; 5] = ["ci", "xtask-ci", "gpu-kernel", "gpu-metal", "gpu-lavapipe"];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("qa_compose: {rel}: {e}"))
}

#[cfg(feature = "controls")]
/// `rel`'s text with `from` replaced by `to` once, failing if `from` is gone, so a control cannot pass vacuously.
fn edited(rel: &str, from: &str, to: &str) -> String {
    let text = read(rel);
    assert!(
        text.contains(from),
        "qa_compose: the control's edit target is gone from {rel}: {from}"
    );
    text.replacen(from, to, 1)
}

// ---- A stand-in `gh` over a small world ----------------------------------------------------------------------------

/// The stand-in `gh`. The world advances one line of `$QA_DIR/world` with each call for the runs of `ci.yml` (a poll),
/// the last line repeating; a line is `<run status|none> <run attempt> <pull request head> <labels, comma-joined, or ->`.
/// With `QA_CANCEL_ENDS=<n>`, the run (5) reads `completed` once `<n>` cancels or force-cancels of it have been asked
/// for. With `QA_MAIN_RUN` set, a query of the runs not narrowed to `event=pull_request` also lists, first, a push run
/// on `main` (7, a second attempt, in progress), which no cancel ends.
/// The pull request's head and labels answer from the line of the latest poll (the first before any). The jobs of
/// attempt `<n>` answer `$QA_DIR/attempt_<n>.json`, failing when it is absent; its first `QA_UNREAD` reads fail (jobs
/// unreadable), and, where `$QA_DIR/attempt_<n>.before.json` is present, the `QA_DECIDE_AFTER` reads after those answer
/// it instead (an attempt whose `changes` has not decided yet, deciding at the next read); a run's latest jobs answer
/// `$QA_DIR/latest.json`. Every answer is JSON in the API's shape, put through the caller's `--jq` filter by `jq -r`, as
/// `gh --jq` does. `/rerun` is refused `QA_REFUSE` times.
const GH: &str = r#"#!/bin/bash
echo "$*" >> "$QA_DIR/log"
filter=""
prev=""
for a in "$@"; do
  if [ "$prev" = --jq ]; then filter=$a; fi
  prev=$a
done
answer() { if [ -n "$filter" ]; then jq -r "$filter"; else cat; fi; }
tick() {
  local n total
  n=$(grep -c '/workflows/ci.yml/runs' "$QA_DIR/log")
  total=$(wc -l < "$QA_DIR/world")
  [ "$n" -ge 1 ] || n=1
  [ "$n" -le "$total" ] || n=$total
  sed -n "${n}p" "$QA_DIR/world"
}
case "$*" in
  *"--method POST"*/rerun*)
    n=$(grep -c '/rerun' "$QA_DIR/log")
    [ "$n" -gt "${QA_REFUSE:-0}" ]; exit $? ;;
  *"--method POST"*/statuses/*) exit 0 ;;
  *"--method POST"*) exit 0 ;;
  *"/workflows/ci.yml/runs"*)
    read -r status attempt _ <<< "$(tick)"
    if [ -n "${QA_CANCEL_ENDS:-}" ] && [ "$status" != none ] \
      && [ "$(grep -c -e '/runs/5/cancel' -e '/runs/5/force-cancel' "$QA_DIR/log")" -ge "$QA_CANCEL_ENDS" ]; then
      status=completed
    fi
    main=""
    if [ -n "${QA_MAIN_RUN:-}" ] && [[ "$*" != *event=pull_request* ]]; then
      main='{"id":7,"event":"push","head_branch":"main","status":"in_progress","run_attempt":2}'
    fi
    if [ "$status" = none ]; then
      printf '{"workflow_runs":[%s]}\n' "$main" | answer
    else
      printf '{"workflow_runs":[%s%s{"id":5,"event":"pull_request","status":"%s","run_attempt":%s}]}\n' \
        "$main" "${main:+,}" "$status" "$attempt" | answer
    fi ;;
  *"/attempts/"*"/jobs"*)
    args="$*"
    n=${args#*/attempts/}
    n=${n%%/*}
    reads=$(grep -c "/attempts/$n/jobs" "$QA_DIR/log")
    [ "$reads" -gt "${QA_UNREAD:-0}" ] || exit 1
    if [ -f "$QA_DIR/attempt_$n.before.json" ] \
      && [ "$reads" -le "$((${QA_UNREAD:-0} + ${QA_DECIDE_AFTER:-0}))" ]; then
      answer < "$QA_DIR/attempt_$n.before.json"
      exit
    fi
    [ -f "$QA_DIR/attempt_$n.json" ] || exit 1
    answer < "$QA_DIR/attempt_$n.json" ;;
  *"/runs/"*"/jobs"*)
    if [ -f "$QA_DIR/latest.json" ]; then answer < "$QA_DIR/latest.json"; else echo '{"jobs":[]}' | answer; fi ;;
  *"/pulls/"*)
    read -r _ _ head labels <<< "$(tick)"
    json=""
    if [ "$labels" != - ]; then
      IFS=, read -r -a names <<< "$labels"
      for l in "${names[@]}"; do json="$json${json:+,}{\"name\":\"$l\"}"; done
    fi
    printf '{"head":{"sha":"%s"},"labels":[%s]}\n' "$head" "$json" | answer ;;
  *) echo "gh stand-in: unexpected: $*" >&2; exit 9 ;;
esac
"#;

/// A run of the script: its exit, output, `$GITHUB_OUTPUT` and the stand-in's call log.
struct Ran {
    code: Option<i32>,
    out: String,
    outputs: BTreeMap<String, String>,
    calls: Vec<String>,
}

impl Ran {
    fn ok(&self) -> bool {
        self.code == Some(0)
    }
    fn output(&self, key: &str) -> &str {
        self.outputs.get(key).map_or("", String::as_str)
    }
    fn count(&self, what: &str) -> usize {
        self.calls.iter().filter(|c| c.contains(what)).count()
    }
}

/// The jobs API's answer for jobs `(name, conclusion)`.
fn jobs(list: &[(&str, &str)]) -> String {
    let jobs: Vec<Value> = list
        .iter()
        .map(|(n, c)| serde_json::json!({"name": n, "status": "completed", "conclusion": c}))
        .collect();
    serde_json::json!({"total_count": jobs.len(), "jobs": jobs}).to_string()
}

struct World<'a> {
    /// The world's lines, one a poll.
    lines: &'a [&'a str],
    /// `(attempt, jobs JSON)`.
    attempts: &'a [(u32, String)],
    /// `(attempt, jobs JSON)` its first `QA_DECIDE_AFTER` reads answer, before its `changes` decides.
    before: &'a [(u32, String)],
    latest: Option<String>,
    /// What `scripts/ci_changes.sh paths` prints; `None`, it fails.
    paths: Option<&'a [&'a str]>,
    env: &'a [(&'a str, &'a str)],
}

fn need_jq() {
    let ok = Command::new("jq")
        .arg("--version")
        .timed_output()
        .is_ok_and(|o| o.status.success());
    assert!(
        ok,
        "qa_compose: these tests need jq, as `gh --jq` filters with it"
    );
}

/// `script` run as `subcommand` in a scratch folder, beside the real crates script, in `world`.
fn tier(script: &str, subcommand: &str, world: &World) -> Ran {
    need_jq();
    let dir = Scratch::new("qa_compose");
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).expect("scratch");
    write_executable(&bin.join("gh"), GH).expect("gh stand-in");
    let file = dir.join("ci_tier.sh");
    write_executable(&file, script).expect("script");
    std::fs::write(dir.join("ci_quick_crates.py"), read(CRATES)).expect("crates script");
    let changes = dir.join("changes.sh");
    let body = match world.paths {
        Some(p) => format!(
            "#!/bin/bash\n[ \"$1\" = paths ] || exit 3\nprintf '%s\\n' {}\n",
            p.join(" ")
        ),
        None => "#!/bin/bash\nexit 1\n".to_owned(),
    };
    write_executable(&changes, body).expect("changes stand-in");
    std::fs::write(dir.join("log"), "").expect("log");
    std::fs::write(dir.join("world"), format!("{}\n", world.lines.join("\n"))).expect("world");
    for (n, json) in world.attempts {
        std::fs::write(dir.join(format!("attempt_{n}.json")), json).expect("attempt");
    }
    for (n, json) in world.before {
        std::fs::write(dir.join(format!("attempt_{n}.before.json")), json).expect("attempt before");
    }
    if let Some(latest) = &world.latest {
        std::fs::write(dir.join("latest.json"), latest).expect("latest");
    }
    let output = dir.join("output");
    let path = std::env::var("PATH").expect("PATH");
    let mut cmd = Command::new("bash");
    cmd.arg(&file)
        .arg(subcommand)
        .env_clear()
        .env("PATH", format!("{}:{path}", bin.display()))
        .env("HOME", &*dir)
        .env("QA_DIR", &*dir)
        .env("GH_REPO", "owner/repo")
        .env("HEAD_SHA", "aaa111")
        .env("PR", "42")
        .env("GITHUB_OUTPUT", &output)
        .env("GITHUB_RUN_ID", "5")
        .env("CI_CHANGES", &changes)
        .env("CI_ROOT", root())
        .env("CI_TIER_POLL_SECONDS", "0")
        .env("CI_TIER_WAIT_SECONDS", "0")
        .env("CI_TIER_POLLS", "3")
        .env("CI_TIER_WAIT_POLLS", "6");
    for (k, v) in world.env {
        cmd.env(k, v);
    }
    let out = cmd.timed_output().expect("qa_compose: bash ran");
    let outputs = std::fs::read_to_string(&output)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
    Ran {
        code: out.status.code(),
        out: format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
        outputs,
        calls: std::fs::read_to_string(dir.join("log"))
            .expect("log")
            .lines()
            .map(str::to_owned)
            .collect(),
    }
}

/// `merge-ready` put on, in a world of `lines` whose attempts' jobs answer `attempts`.
fn label_on_in(
    script: &str,
    lines: &[&str],
    attempts: &[(u32, String)],
    env: &[(&str, &str)],
) -> Ran {
    let mut all = vec![("ACTION", "labeled")];
    all.extend_from_slice(env);
    tier(
        script,
        "merge-ready",
        &World {
            lines,
            attempts,
            before: &[],
            latest: None,
            paths: None,
            env: &all,
        },
    )
}

/// `merge-ready` put on, as `label_on_in`, with attempts whose first `QA_DECIDE_AFTER` reads answer `before`.
fn label_on_staged(
    script: &str,
    lines: &[&str],
    attempts: &[(u32, String)],
    before: &[(u32, String)],
    env: &[(&str, &str)],
) -> Ran {
    let mut all = vec![("ACTION", "labeled")];
    all.extend_from_slice(env);
    tier(
        script,
        "merge-ready",
        &World {
            lines,
            attempts,
            before,
            latest: None,
            paths: None,
            env: &all,
        },
    )
}

/// An attempt whose `changes` job is still queued: its tier not yet decided.
fn undecided() -> String {
    serde_json::json!({"total_count": 1, "jobs": [
        {"name": "changes", "status": "queued", "conclusion": null}
    ]})
    .to_string()
}

/// The positions in the call log of the reads of attempt `n`'s jobs.
fn attempt_reads(got: &Ran, n: u32) -> Vec<usize> {
    let what = format!("/attempts/{n}/jobs");
    got.calls
        .iter()
        .enumerate()
        .filter(|(_, c)| c.contains(&what))
        .map(|(i, _)| i)
        .collect()
}

/// The polls of the head's runs among calls `from..to`.
fn polls_between(got: &Ran, from: usize, to: usize) -> usize {
    got.calls[from..to]
        .iter()
        .filter(|c| c.contains("/workflows/ci.yml/runs"))
        .count()
}

/// The position of the first cancel of run 5, if any.
fn first_cancel(got: &Ran) -> Option<usize> {
    got.calls
        .iter()
        .position(|c| c.contains("--method POST") && c.contains(CANCEL_5) && c.contains("cancel"))
}

/// The jobs API's answer for an attempt still going: jobs `(name, conclusion)`, `None` for a job not yet ended.
fn going(list: &[(&str, Option<&str>)]) -> String {
    let jobs: Vec<Value> = list
        .iter()
        .map(|(n, c)| match c {
            Some(c) => serde_json::json!({"name": n, "status": "completed", "conclusion": c}),
            None => serde_json::json!({"name": n, "status": "in_progress", "conclusion": null}),
        })
        .collect();
    serde_json::json!({"total_count": jobs.len(), "jobs": jobs}).to_string()
}

/// A quick attempt still going: `changes` said quick, so the quick job runs.
fn quick_going() -> String {
    going(&[("changes", Some("success")), (QUICK_JOB, None)])
}

/// A full attempt still going: `changes` said full, so the quick job skipped.
fn full_going() -> String {
    going(&[
        ("changes", Some("success")),
        (QUICK_JOB, Some("skipped")),
        ("ci-checks", None),
    ])
}

/// The world line the latest poll before the first `/rerun` call saw, if the script re-ran the run.
fn seen_at_rerun<'a>(got: &Ran, lines: &[&'a str]) -> Option<&'a str> {
    let at = got.calls.iter().position(|c| c.contains("/rerun"))?;
    let polls = got.calls[..at]
        .iter()
        .filter(|c| c.contains("/workflows/ci.yml/runs"))
        .count();
    Some(lines[polls.clamp(1, lines.len()) - 1])
}

/// How many polls of the head's runs came before the first call containing `what`, if there is one.
fn polls_before(got: &Ran, what: &str) -> Option<usize> {
    let at = got.calls.iter().position(|c| c.contains(what))?;
    Some(
        got.calls[..at]
            .iter()
            .filter(|c| c.contains("/workflows/ci.yml/runs"))
            .count(),
    )
}

/// A cancel of the head's run, `5` (`/cancel` or `/force-cancel`).
const CANCEL_5: &str = "/runs/5/";

fn cancels_of(got: &Ran, run: &str) -> usize {
    got.calls
        .iter()
        .filter(|c| c.contains("--method POST") && c.contains(run) && c.contains("cancel"))
        .count()
}

/// A label case: what it is, its world's lines, and its attempts' jobs.
type Case = (&'static str, Vec<&'static str>, Vec<(u32, String)>);

// ---- 1. The label added mid-quick cancels the quick attempt and starts the full one at once --------------------------

fn check_cancels_quick(script: &str) {
    // The stand-in run ends only when cancelled (`QA_CANCEL_ENDS`): a script that waits for it, as on 7 Nov, never
    // sees it end.
    let ends = [("QA_CANCEL_ENDS", "1")];
    let cases: Vec<Case> = vec![
        (
            "a first attempt, quick, in progress",
            vec!["in_progress 1 aaa111 merge-ready"],
            vec![(1, quick_going())],
        ),
        (
            "a retry's quick attempt in progress",
            vec!["in_progress 2 aaa111 merge-ready"],
            vec![
                (1, jobs(&[("changes", "success"), (QUICK_JOB, "failure")])),
                (2, quick_going()),
            ],
        ),
    ];
    for (what, lines, attempts) in &cases {
        let got = label_on_in(script, lines, attempts, &ends);
        assert!(
            cancels_of(&got, CANCEL_5) >= 1,
            "qa_compose: the label did not cancel the quick attempt ({what}): {:?} {}",
            got.calls,
            got.out
        );
        let before = polls_before(&got, "cancel").unwrap_or(usize::MAX);
        assert!(
            before <= 1,
            "qa_compose: the label waited before cancelling the quick attempt ({what}; {before} polls first): {:?}",
            got.calls
        );
        assert!(
            got.ok() && got.count("/rerun") == 1,
            "qa_compose: the label did not start the full attempt once after cancelling ({what}): {:?} {}",
            got.calls,
            got.out
        );
        // Asked for only once the cancelled attempt has been seen ended: a poll after the cancel, before the re-run.
        let at_cancel = polls_before(&got, "cancel").unwrap_or(0);
        let at_rerun = polls_before(&got, "/rerun").unwrap_or(0);
        assert!(
            at_rerun > at_cancel,
            "qa_compose: the full attempt was asked for before the cancelled quick attempt was seen ended ({what}): {:?}",
            got.calls
        );
    }
    // A quick attempt already ended: nothing to cancel; the full attempt starts.
    let got = label_on_in(
        script,
        &["completed 1 aaa111 merge-ready"],
        &[(1, jobs(&[("changes", "success"), (QUICK_JOB, "success")]))],
        &ends,
    );
    assert!(
        got.ok() && cancels_of(&got, CANCEL_5) == 0 && got.count("/rerun") == 1,
        "qa_compose: the label on an ended quick attempt cancelled it or did not start the full attempt: {:?} {}",
        got.calls,
        got.out
    );
    // A quick attempt no cancel ends is never re-run over: the job fails, bounded, so the full-only checks stay
    // pending, never passed.
    let got = label_on_in(
        script,
        &["in_progress 1 aaa111 merge-ready"],
        &[(1, quick_going())],
        &[],
    );
    assert!(
        !got.ok() && got.count("/rerun") == 0,
        "qa_compose: a quick attempt no cancel ended was re-run, or the job passed: {:?} {}",
        got.calls,
        got.out
    );
    // A first attempt, queued, its `changes` not yet decided: read again until `changes` decides (here at the fourth
    // read), quick; then cancelled at once, and the full attempt started once the cancel has ended it.
    let decide_after = 3;
    let got = label_on_staged(
        script,
        &["queued 1 aaa111 merge-ready"],
        &[(1, quick_going())],
        &[(1, undecided())],
        &[("QA_CANCEL_ENDS", "1"), ("QA_DECIDE_AFTER", "3")],
    );
    let reads = attempt_reads(&got, 1);
    let cancel = first_cancel(&got);
    assert!(
        cancel.is_some(),
        "qa_compose: an attempt that decided quick after some reads was not cancelled: {:?} {}",
        got.calls,
        got.out
    );
    let cancel = cancel.unwrap_or(0);
    assert!(
        reads.iter().filter(|&&r| r < cancel).count() > decide_after,
        "qa_compose: an attempt was cancelled before its changes job had decided its tier: {:?}",
        got.calls
    );
    let decided_at = reads.get(decide_after).copied().unwrap_or(0);
    assert!(
        polls_between(&got, decided_at, cancel) <= 1,
        "qa_compose: the label waited before cancelling an attempt once it was shown quick: {:?}",
        got.calls
    );
    assert!(
        got.ok()
            && got.count("/rerun") == 1
            && polls_before(&got, "/rerun").unwrap_or(0) > polls_before(&got, "cancel").unwrap_or(0),
        "qa_compose: the full attempt was not started once, after the cancelled attempt ended (decided late): {:?} {}",
        got.calls,
        got.out
    );
    // One whose `changes` never decides: the job fails, named, bounded, cancelling and re-running nothing.
    let got = label_on_staged(
        script,
        &["queued 1 aaa111 merge-ready"],
        &[(1, quick_going())],
        &[(1, undecided())],
        &[("QA_CANCEL_ENDS", "1"), ("QA_DECIDE_AFTER", "1000")],
    );
    assert!(
        cancels_of(&got, CANCEL_5) == 0 && got.count("/rerun") == 0,
        "qa_compose: an attempt whose changes never decided was cancelled or re-run over: {:?} {}",
        got.calls,
        got.out
    );
    assert!(
        !got.ok() && got.out.contains("::error::") && got.out.contains("aaa111"),
        "qa_compose: an attempt whose changes never decided did not fail the job, named: {}",
        got.out
    );
    let n = attempt_reads(&got, 1).len();
    assert!(
        (2..=6).contains(&n),
        "qa_compose: an undecided attempt was not read again, or read past the wait's 6 polls ({n} reads): {:?}",
        got.calls
    );
}

#[test]
fn qa_compose_label_mid_quick_cancels_it_at_once() {
    check_cancels_quick(&read(TIER));
}

/// The rule the owner's order of 8 Nov 2026 removes: the quick attempt waited for, never cancelled.
mod waits_as_on_7_nov {
    use super::*;
    negative_control!(
        qa_compose_label_mid_quick_cancels_it_at_once,
        "the label waiting for a quick attempt to end, cancelling nothing",
        expected = "qa_compose: the label did not cancel the quick attempt",
        check_cancels_quick(&edited(
            TIER,
            "run=$(cancel_quick \"$id\") || return 1",
            "run=$(wait_full) || return 1"
        ))
    );
}

mod polls_first {
    use super::*;
    negative_control!(
        qa_compose_label_mid_quick_cancels_it_at_once,
        "the label polling the run again before cancelling it",
        expected = "qa_compose: the label waited before cancelling the quick attempt",
        check_cancels_quick(&edited(
            TIER,
            "cancel_quick() {\n",
            "cancel_quick() {\n  latest_run > /dev/null\n"
        ))
    );
}

mod reruns_unseen {
    use super::*;
    negative_control!(
        qa_compose_label_mid_quick_cancels_it_at_once,
        "the re-run asked for straight after the cancel, the run not seen ended",
        expected = "qa_compose: the full attempt was asked for before the cancelled quick attempt was seen ended",
        check_cancels_quick(&edited(
            TIER,
            "cancel_quick() {\n",
            "cancel_quick() {\n  gh api --method POST \"repos/$GH_REPO/actions/runs/$1/cancel\" > /dev/null\n  echo \"$1 completed 1\"\n  return 0\n"
        ))
    );
}

/// The defect code review 5819591377 found: an attempt whose tier is not shown taken for a quick one.
mod undecided_read_as_quick {
    use super::*;
    negative_control!(
        qa_compose_label_mid_quick_cancels_it_at_once,
        "an attempt whose changes has not decided read as quick, and cancelled",
        expected =
            "qa_compose: an attempt was cancelled before its changes job had decided its tier",
        check_cancels_quick(&edited(TIER, "then return 2; fi", "then return 1; fi"))
    );
}

mod undecided_unbounded {
    use super::*;
    negative_control!(
        qa_compose_label_mid_quick_cancels_it_at_once,
        "an undecided attempt read again past the wait's bound",
        expected =
            "qa_compose: an undecided attempt was not read again, or read past the wait's 6 polls",
        check_cancels_quick(&edited(
            TIER,
            "if [ \"$waited\" -ge \"$wait_polls\" ]; then",
            "if [ \"$waited\" -ge 9 ]; then"
        ))
    );
}

negative_control!(
    qa_compose_label_mid_quick_cancels_it_at_once,
    "the cancel's wait passing on a run it has not ended",
    expected = "qa_compose: a quick attempt no cancel ended was re-run",
    check_cancels_quick(&edited(
        TIER,
        "  wait_for_end \"$polls\" \"$seconds\"\n}",
        "  latest_run\n}"
    ))
);

// ---- 1b. A full attempt still going is waited for, never cancelled ---------------------------------------------------

fn check_full_waited(script: &str) {
    // A cancel would end the stand-in run at once, so a script that cancels it is seen re-running it early.
    let ends = [("QA_CANCEL_ENDS", "1")];
    let cases: Vec<Case> = vec![
        (
            "a retry's full attempt in progress, then ended",
            vec![
                "in_progress 2 aaa111 merge-ready",
                "in_progress 2 aaa111 merge-ready",
                "in_progress 2 aaa111 merge-ready",
                "completed 2 aaa111 merge-ready",
            ],
            vec![
                (1, jobs(&[("changes", "success"), (QUICK_JOB, "success")])),
                (2, full_going()),
            ],
        ),
        (
            "a first attempt, opened with the label, full, in progress, then ended",
            vec![
                "in_progress 1 aaa111 merge-ready",
                "in_progress 1 aaa111 merge-ready",
                "completed 1 aaa111 merge-ready",
            ],
            vec![(1, full_going())],
        ),
    ];
    for (what, lines, attempts) in &cases {
        let got = label_on_in(script, lines, attempts, &ends);
        assert_eq!(
            cancels_of(&got, CANCEL_5),
            0,
            "qa_compose: the label cancelled a full attempt ({what}): {:?}",
            got.calls
        );
        assert!(
            got.ok() && got.count("/rerun") == 1,
            "qa_compose: the label did not re-run once after the full attempt ({what}): {:?} {}",
            got.calls,
            got.out
        );
        let seen = seen_at_rerun(&got, lines).unwrap_or("");
        assert!(
            seen.starts_with("completed "),
            "qa_compose: the full attempt was re-run before it ended ({what}; last seen `{seen}`): {:?}",
            got.calls
        );
    }
    // One that outlasts the bounded wait is neither cancelled nor re-run; the job fails.
    let got = label_on_in(
        script,
        &["in_progress 2 aaa111 merge-ready"],
        &[(2, full_going())],
        &[],
    );
    assert!(
        !got.ok() && cancels_of(&got, CANCEL_5) == 0 && got.count("/rerun") == 0,
        "qa_compose: a full attempt outlasting the wait was cancelled or re-run, or the job passed: {:?} {}",
        got.calls,
        got.out
    );
}

#[test]
fn qa_compose_label_mid_full_waits_never_cancels() {
    check_full_waited(&read(TIER));
}

mod full_read_as_quick {
    use super::*;
    negative_control!(
        qa_compose_label_mid_full_waits_never_cancels,
        "every attempt still going read as quick, a full one cancelled",
        expected = "qa_compose: the label cancelled a full attempt",
        check_full_waited(&edited(
            TIER,
            "0 | 1) return \"$shown\" ;;",
            "0 | 1) return 1 ;;"
        ))
    );
}

negative_control!(
    qa_compose_label_mid_full_waits_never_cancels,
    "the full attempt not waited for, re-run while still going",
    expected = "qa_compose: the full attempt was re-run before it ended",
    check_full_waited(&edited(
        TIER,
        "run=$(wait_full) || return 1",
        "run=$(latest_run)"
    ))
);

// ---- 1c. Only a shown tier is acted on --------------------------------------------------------------------------------

fn check_shown_tier(script: &str) {
    // A retry's attempt whose `changes` decides full at the fourth read: waited for, never cancelled, re-run once ended.
    let lines = [
        "in_progress 2 aaa111 merge-ready",
        "in_progress 2 aaa111 merge-ready",
        "completed 2 aaa111 merge-ready",
    ];
    let got = label_on_staged(
        script,
        &lines,
        &[
            (1, jobs(&[("changes", "success"), (QUICK_JOB, "success")])),
            (2, full_going()),
        ],
        &[(2, undecided())],
        &[("QA_CANCEL_ENDS", "1"), ("QA_DECIDE_AFTER", "3")],
    );
    assert_eq!(
        cancels_of(&got, CANCEL_5),
        0,
        "qa_compose: the label cancelled an attempt that decided full after some reads: {:?}",
        got.calls
    );
    assert!(
        got.ok()
            && got.count("/rerun") == 1
            && seen_at_rerun(&got, &lines).is_some_and(|l| l.starts_with("completed ")),
        "qa_compose: an attempt that decided full late was not waited for, then re-run once: {:?} {}",
        got.calls,
        got.out
    );
    // The reads spent waiting for `changes` come out of the same bounded wait as the full attempt's (6 polls here):
    // decided full at the fifth read, never ending, the job fails having waited at most 6 polls in all.
    let decide_after = 4;
    let got = label_on_staged(
        script,
        &["in_progress 2 aaa111 merge-ready"],
        &[(2, full_going())],
        &[(2, undecided())],
        &[("QA_CANCEL_ENDS", "1"), ("QA_DECIDE_AFTER", "4")],
    );
    assert!(
        !got.ok() && cancels_of(&got, CANCEL_5) == 0 && got.count("/rerun") == 0,
        "qa_compose: a late-decided full attempt outlasting the wait was cancelled or re-run, or the job passed: {:?} {}",
        got.calls,
        got.out
    );
    let reads = attempt_reads(&got, 2);
    let decided_at = reads.get(decide_after).copied().unwrap_or(0);
    let waited = decide_after + polls_between(&got, decided_at, got.calls.len());
    assert!(
        waited <= 6,
        "qa_compose: the wait for changes did not share the full attempt's bound ({waited} polls of 6): {:?}",
        got.calls
    );
    // Jobs unread twice, then shown quick: cancelled, and the full attempt started.
    let got = label_on_in(
        script,
        &["in_progress 1 aaa111 merge-ready"],
        &[(1, quick_going())],
        &[("QA_CANCEL_ENDS", "1"), ("QA_UNREAD", "2")],
    );
    assert!(
        got.ok() && cancels_of(&got, CANCEL_5) >= 1 && got.count("/rerun") == 1,
        "qa_compose: a quick attempt whose jobs were read on the third try was not cancelled and re-run: {:?} {}",
        got.calls,
        got.out
    );
    // Jobs never readable: read again, at most 30 times, then the job fails, named, cancelling nothing.
    let got = label_on_in(
        script,
        &["in_progress 1 aaa111 merge-ready"],
        &[(1, quick_going())],
        &[("QA_CANCEL_ENDS", "1"), ("QA_UNREAD", "1000")],
    );
    assert!(
        cancels_of(&got, CANCEL_5) == 0 && got.count("/rerun") == 0,
        "qa_compose: the label cancelled an attempt whose jobs were unread, or re-ran over it: {:?} {}",
        got.calls,
        got.out
    );
    assert!(
        !got.ok() && got.out.contains("::error::") && got.out.contains("aaa111"),
        "qa_compose: an attempt whose jobs were unread did not fail the job, named: {}",
        got.out
    );
    let n = attempt_reads(&got, 1).len();
    assert!(
        (2..=30).contains(&n),
        "qa_compose: unread jobs were not read again, or read more than 30 times ({n} reads): {:?}",
        got.calls
    );
}

#[test]
fn qa_compose_label_acts_only_on_a_shown_tier() {
    check_shown_tier(&read(TIER));
}

mod late_full_read_as_quick {
    use super::*;
    negative_control!(
        qa_compose_label_acts_only_on_a_shown_tier,
        "an attempt whose changes has not decided read as quick, and cancelled",
        expected = "qa_compose: the label cancelled an attempt that decided full after some reads",
        check_shown_tier(&edited(TIER, "then return 2; fi", "then return 1; fi"))
    );
}

mod own_wait_bound {
    use super::*;
    negative_control!(
        qa_compose_label_acts_only_on_a_shown_tier,
        "the full attempt's wait not reduced by the polls spent waiting for changes",
        expected = "qa_compose: the wait for changes did not share the full attempt's bound",
        check_shown_tier(&edited(TIER, " - waited))", "))"))
    );
}

mod unread_read_as_quick {
    use super::*;
    negative_control!(
        qa_compose_label_acts_only_on_a_shown_tier,
        "an attempt whose jobs are unread read as quick, and cancelled",
        expected = "qa_compose: the label cancelled an attempt whose jobs were unread",
        check_shown_tier(&edited(TIER, "\"') || return 3", "\"') || return 1"))
    );
}

mod unread_unbounded {
    use super::*;
    negative_control!(
        qa_compose_label_acts_only_on_a_shown_tier,
        "unread jobs read again past 30 reads",
        expected = "qa_compose: unread jobs were not read again, or read more than 30 times",
        check_shown_tier(&edited(
            TIER,
            "if [ \"$unread\" -ge \"$read_polls\" ]; then",
            "if [ \"$unread\" -ge 40 ]; then"
        ))
    );
}

negative_control!(
    qa_compose_label_acts_only_on_a_shown_tier,
    "unread jobs never read again",
    expected = "qa_compose: a quick attempt whose jobs were read on the third try was not cancelled and re-run",
    check_shown_tier(&edited(
        TIER,
        "read_polls=\"${CI_TIER_DECIDE_POLLS:-30}\"",
        "read_polls=1"
    ))
);

// ---- 2. A push still takes merge-ready off; a superseded head or a label off is never cancelled ----------------------

/// A minimal evaluation of the `mutants.yml` job that answers a push: whether, on a `synchronize` whose payload lists
/// `merge-ready`, some step it runs deletes the label.
fn push_takes_label_off(mutants: &str) -> bool {
    let wf = parse(mutants);
    let ctx = Ctx {
        github: [
            ("github.event_name", "pull_request"),
            ("github.event.action", "synchronize"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect(),
        labels: vec!["merge-ready".to_owned()],
        ..Ctx::default()
    };
    // Only the jobs with a step that deletes the label are evaluated: the others' conditions read terms not modelled.
    let deletes = |s: &Value| {
        let text = s.get("run").map_or("", str_of);
        text.contains("--method DELETE") && text.contains("/labels/merge-ready")
    };
    wf["jobs"].as_object().expect("jobs").values().any(|job| {
        let steps = job["steps"].as_array().map_or(&[][..], Vec::as_slice);
        steps.iter().any(deletes)
            && job
                .get("if")
                .is_none_or(|c| truthy(&eval_if(str_of(c), &ctx)))
            && steps.iter().any(|s| {
                deletes(s)
                    && s.get("if")
                        .is_none_or(|c| truthy(&eval_if(str_of(c), &ctx)))
            })
    })
}

fn check_push(script: &str, mutants: &str) {
    assert!(
        push_takes_label_off(mutants),
        "qa_compose: a push no longer takes merge-ready off"
    );
    // The pushed head's first attempt is quick even if the live label is still on (it comes off in another workflow).
    let got = tier(
        script,
        "decide",
        &World {
            lines: &["completed 1 aaa111 merge-ready"],
            attempts: &[],
            before: &[],
            latest: None,
            paths: Some(&["crates/kernel/src/lib.rs"]),
            env: &[
                ("TIER", "code"),
                ("GITHUB_EVENT_NAME", "pull_request"),
                ("ACTION", "synchronize"),
                ("GITHUB_RUN_ATTEMPT", "1"),
                ("DOCS_ONLY", "false"),
                ("GPU", "true"),
            ],
        },
    );
    assert!(
        got.ok() && got.output("quick") == "true",
        "qa_compose: a pushed head's first attempt is not quick with the label still on: {:?} {}",
        got.outputs,
        got.out
    );
    let ends = [("QA_CANCEL_ENDS", "1")];
    // A superseded head: the pull request's head has moved on (bbb222) while the old head's quick attempt still runs,
    // the push's `merge-ready-off` not yet having taken the label off. The old run is never cancelled nor re-run: the
    // push's run has superseded it.
    let got = label_on_in(
        script,
        &["in_progress 1 bbb222 merge-ready"],
        &[(1, quick_going())],
        &ends,
    );
    assert!(
        got.ok() && cancels_of(&got, CANCEL_5) == 0 && got.count("/rerun") == 0,
        "qa_compose: a superseded head's quick attempt was cancelled or re-run: {:?} {}",
        got.calls,
        got.out
    );
    // The label already off when this job reads it, the head unchanged: the quick attempt is never cancelled.
    let got = label_on_in(
        script,
        &["in_progress 1 aaa111 -"],
        &[(1, quick_going())],
        &ends,
    );
    assert!(
        got.ok() && cancels_of(&got, CANCEL_5) == 0 && got.count("/rerun") == 0,
        "qa_compose: the label already off, the quick attempt was still cancelled or re-run: {:?} {}",
        got.calls,
        got.out
    );
    // A push during the wait for a full attempt: the old head's run ends (cancelled by the push's run), and nothing is
    // re-run on it, even before the push's `merge-ready-off` has taken the label off.
    let got = label_on_in(
        script,
        &[
            "in_progress 2 aaa111 merge-ready",
            "completed 2 bbb222 merge-ready",
        ],
        &[(2, full_going())],
        &ends,
    );
    assert!(
        got.ok() && got.count("/rerun") == 0 && cancels_of(&got, CANCEL_5) == 0,
        "qa_compose: a push during the wait still started the full attempt on the old head: {:?} {}",
        got.calls,
        got.out
    );
    // The label taken off during the wait for a full attempt, the head unchanged: nothing is re-run.
    let got = label_on_in(
        script,
        &["in_progress 2 aaa111 merge-ready", "completed 2 aaa111 -"],
        &[(2, full_going())],
        &ends,
    );
    assert!(
        got.ok() && got.count("/rerun") == 0 && cancels_of(&got, CANCEL_5) == 0,
        "qa_compose: the label off by the full attempt's end still started the full attempt: {:?} {}",
        got.calls,
        got.out
    );
}

#[test]
fn qa_compose_a_push_still_takes_the_label_off() {
    check_push(&read(TIER), &read(MUTANTS));
}

mod push_keeps_the_label {
    use super::*;
    negative_control!(
        qa_compose_a_push_still_takes_the_label_off,
        "mutants.yml's label removal running on no push",
        expected = "qa_compose: a push no longer takes merge-ready off",
        check_push(
            &read(TIER),
            &read(MUTANTS).replace(
                "github.event.action == 'synchronize'",
                "github.event.action == 'opened'"
            )
        )
    );
}

mod push_reads_the_payload {
    use super::*;
    negative_control!(
        qa_compose_a_push_still_takes_the_label_off,
        "a pushed head's first attempt reading the live label",
        expected = "qa_compose: a pushed head's first attempt is not quick",
        check_push(
            &edited(
                TIER,
                "[ \"${ACTION:-}\" = synchronize ] && [ \"${GITHUB_RUN_ATTEMPT:-1}\" = 1 ]",
                "false"
            ),
            &read(MUTANTS)
        )
    );
}

mod old_head_rerun {
    use super::*;
    negative_control!(
        qa_compose_a_push_still_takes_the_label_off,
        "the script reading the head it was given, not the live one",
        expected = "qa_compose: a superseded head's quick attempt was cancelled or re-run",
        check_push(
            &edited(
                TIER,
                "live=$(gh api \"repos/$GH_REPO/pulls/$PR\" --jq .head.sha)",
                "live=$HEAD_SHA"
            ),
            &read(MUTANTS)
        )
    );
}

negative_control!(
    qa_compose_a_push_still_takes_the_label_off,
    "the script not reading the live label",
    expected = "qa_compose: the label already off, the quick attempt was still cancelled",
    check_push(
        &edited(
            TIER,
            "  if ! grep -qxF merge-ready <<< \"$labels\"; then\n",
            "  if false; then\n"
        ),
        &read(MUTANTS)
    )
);

// ---- 2b. A run on main is never cancelled (R-656) --------------------------------------------------------------------

fn check_main_spared(script: &str) {
    // The stand-in lists, to any query of the head's runs not narrowed to `pull_request` runs, a push run on `main`
    // (run 7, a second attempt, in progress) ahead of the pull request's run.
    let main = [("QA_MAIN_RUN", "1"), ("QA_CANCEL_ENDS", "1")];
    // The stand-in answers jobs by attempt alone: attempt 2 (main's run's, read only by a script that took main's run
    // for the head's) is shown quick, so such a script would cancel it.
    let got = label_on_in(
        script,
        &["completed 1 aaa111 merge-ready"],
        &[
            (1, jobs(&[("changes", "success"), (QUICK_JOB, "success")])),
            (2, quick_going()),
        ],
        &main,
    );
    assert_eq!(
        cancels_of(&got, "/runs/7/"),
        0,
        "qa_compose: a run on main was cancelled (label on): {:?}",
        got.calls
    );
    assert!(
        got.ok() && got.count("/runs/5/rerun") == 1,
        "qa_compose: the label on did not start the pull request's full attempt beside a run on main: {:?} {}",
        got.calls,
        got.out
    );
    let mut env = vec![("ACTION", "unlabeled")];
    env.extend_from_slice(&main);
    let got = tier(
        script,
        "merge-ready",
        &World {
            lines: &["completed 2 aaa111 -"],
            attempts: &[],
            before: &[],
            latest: None,
            paths: None,
            env: &env,
        },
    );
    assert_eq!(
        cancels_of(&got, "/runs/7/"),
        0,
        "qa_compose: a run on main was cancelled (label off): {:?}",
        got.calls
    );
}

#[test]
fn qa_compose_label_never_cancels_a_run_on_main() {
    check_main_spared(&read(TIER));
}

negative_control!(
    qa_compose_label_never_cancels_a_run_on_main,
    "the head's runs read without narrowing them to pull_request runs",
    expected = "qa_compose: a run on main was cancelled",
    check_main_spared(&edited(TIER, "&event=pull_request", ""))
);

// ---- 3. The evidence the full attempt carries ------------------------------------------------------------------------

struct Evidence {
    what: &'static str,
    attempt: &'static str,
    attempts: Vec<(u32, String)>,
    paths: Option<&'static [&'static str]>,
    passed: bool,
    covers: bool,
}

const KERNEL: Option<&[&str]> = Some(&["crates/kernel/src/lib.rs"]);
const GUI: Option<&[&str]> = Some(&["crates/gui/src/lib.rs"]);

fn evidence_cases() -> Vec<Evidence> {
    let quick = |c: &str| jobs(&[("changes", "success"), (QUICK_JOB, c)]);
    vec![
        Evidence {
            what: "quick passed over every crate",
            attempt: "2",
            attempts: vec![(1, quick("success"))],
            paths: KERNEL,
            passed: true,
            covers: true,
        },
        Evidence {
            what: "quick passed over some crates",
            attempt: "2",
            attempts: vec![(1, quick("success"))],
            paths: GUI,
            passed: true,
            covers: false,
        },
        Evidence {
            what: "quick passed, paths unread",
            attempt: "2",
            attempts: vec![(1, quick("success"))],
            paths: None,
            passed: true,
            covers: false,
        },
        Evidence {
            what: "quick failed",
            attempt: "2",
            attempts: vec![(1, quick("failure"))],
            paths: KERNEL,
            passed: false,
            covers: false,
        },
        Evidence {
            what: "quick cancelled",
            attempt: "2",
            attempts: vec![(1, quick("cancelled"))],
            paths: KERNEL,
            passed: false,
            covers: false,
        },
        // The label's cancel (the owner, 8 Nov 2026) caught the quick job before it ended: a cancelled quick attempt
        // never counts as passed, whatever conclusion its job was left with.
        Evidence {
            what: "quick cancelled by the label, its job never concluded",
            attempt: "2",
            attempts: vec![(1, going(&[("changes", Some("success")), (QUICK_JOB, None)]))],
            paths: KERNEL,
            passed: false,
            covers: false,
        },
        Evidence {
            what: "quick skipped",
            attempt: "2",
            attempts: vec![(1, quick("skipped"))],
            paths: KERNEL,
            passed: false,
            covers: false,
        },
        Evidence {
            what: "a job named like quick's, not quick's",
            attempt: "2",
            attempts: vec![(
                1,
                jobs(&[
                    ("ci-quick", "success"),
                    ("ci-quick / ci-quick (x)", "success"),
                ]),
            )],
            paths: KERNEL,
            passed: false,
            covers: false,
        },
        Evidence {
            what: "the earlier attempt's jobs unread",
            attempt: "2",
            attempts: vec![],
            paths: KERNEL,
            passed: false,
            covers: false,
        },
        Evidence {
            what: "no earlier attempt",
            attempt: "1",
            attempts: vec![(1, quick("success"))],
            paths: KERNEL,
            passed: false,
            covers: false,
        },
        Evidence {
            what: "only this attempt's own jobs show quick",
            attempt: "2",
            attempts: vec![(1, jobs(&[("changes", "success")])), (2, quick("success"))],
            paths: KERNEL,
            passed: false,
            covers: false,
        },
        Evidence {
            what: "quick passed on a retry, two attempts back",
            attempt: "3",
            attempts: vec![(1, quick("failure")), (2, quick("success"))],
            paths: KERNEL,
            passed: true,
            covers: true,
        },
    ]
}

fn decide_full(script: &str, case: &Evidence) -> Ran {
    tier(
        script,
        "decide",
        &World {
            lines: &["completed 1 aaa111 merge-ready"],
            attempts: &case.attempts,
            before: &[],
            latest: None,
            paths: case.paths,
            env: &[
                ("TIER", "code"),
                ("GITHUB_EVENT_NAME", "pull_request"),
                // A first attempt is full only when opened with the label on; a push's first attempt is quick.
                (
                    "ACTION",
                    if case.attempt == "1" {
                        "opened"
                    } else {
                        "synchronize"
                    },
                ),
                ("GITHUB_RUN_ATTEMPT", case.attempt),
                ("DOCS_ONLY", "false"),
                ("GPU", "true"),
            ],
        },
    )
}

fn check_evidence(script: &str) {
    for case in evidence_cases() {
        let got = decide_full(script, &case);
        assert!(
            got.ok() && got.output("quick") == "false",
            "qa_compose: decide failed or ran quick on a full attempt ({}): {:?} {}",
            case.what,
            got.outputs,
            got.out
        );
        let want = |b: bool| if b { "true" } else { "false" };
        assert_eq!(
            got.output("quick_passed"),
            want(case.passed),
            "qa_compose: quick_passed wrong ({}): {:?} {}",
            case.what,
            got.outputs,
            got.out
        );
        assert_eq!(
            got.output("quick_covers_ci"),
            want(case.covers),
            "qa_compose: quick_covers_ci wrong ({}): {:?} {}",
            case.what,
            got.outputs,
            got.out
        );
    }
    // Nowhere but a pull request's full attempt: a push to main, a docs head and a quick run carry nothing.
    let quick_ok = vec![(1, jobs(&[(QUICK_JOB, "success")]))];
    let others: &[(&str, &[(&str, &str)])] = &[
        (
            "a push to main",
            &[
                ("TIER", "code"),
                ("GITHUB_EVENT_NAME", "push"),
                ("ACTION", ""),
                ("GITHUB_RUN_ATTEMPT", "2"),
            ],
        ),
        (
            "a docs head",
            &[
                ("TIER", "docs"),
                ("GITHUB_EVENT_NAME", "pull_request"),
                ("ACTION", "synchronize"),
                ("GITHUB_RUN_ATTEMPT", "2"),
            ],
        ),
        (
            "a quick retry, no label",
            &[
                ("TIER", "code"),
                ("GITHUB_EVENT_NAME", "pull_request"),
                ("ACTION", "synchronize"),
                ("GITHUB_RUN_ATTEMPT", "2"),
            ],
        ),
    ];
    for (what, env) in others {
        let mut env = env.to_vec();
        env.extend_from_slice(&[("DOCS_ONLY", "false"), ("GPU", "true")]);
        let line = if *what == "a quick retry, no label" {
            "completed 1 aaa111 -"
        } else {
            "completed 1 aaa111 merge-ready"
        };
        let got = tier(
            script,
            "decide",
            &World {
                lines: &[line],
                attempts: &quick_ok,
                before: &[],
                latest: None,
                paths: KERNEL,
                env: &env,
            },
        );
        assert!(
            got.ok()
                && got.output("quick_passed") != "true"
                && got.output("quick_covers_ci") != "true",
            "qa_compose: evidence carried outside a pull request's full attempt ({what}): {:?} {}",
            got.outputs,
            got.out
        );
    }
}

#[test]
fn qa_compose_evidence_only_from_an_earlier_passed_quick_attempt() {
    check_evidence(&read(TIER));
}

mod any_conclusion {
    use super::*;
    negative_control!(
        qa_compose_evidence_only_from_an_earlier_passed_quick_attempt,
        "the evidence reading any conclusion of the quick job as a pass",
        expected = "qa_compose: quick_passed wrong (quick failed)",
        check_evidence(&edited(
            TIER,
            "--jq '.jobs[] | select(.conclusion == \"success\") | .name') || continue",
            "--jq '.jobs[] | .name') || continue"
        ))
    );
}

mod its_own_attempt {
    use super::*;
    negative_control!(
        qa_compose_evidence_only_from_an_earlier_passed_quick_attempt,
        "the evidence reading this attempt's own jobs",
        expected = "qa_compose: quick_passed wrong (no earlier attempt)",
        check_evidence(&edited(TIER, "n < attempt", "n <= attempt"))
    );
}

mod some_crates_cover_all {
    use super::*;
    negative_control!(
        qa_compose_evidence_only_from_an_earlier_passed_quick_attempt,
        "a quick tier of some crates read as covering the shards",
        expected = "qa_compose: quick_covers_ci wrong (quick passed over some crates)",
        check_evidence(&edited(
            TIER,
            "if [ -n \"$crates\" ] && [ \"$crates\" = \"$every\" ]; then",
            "if [ -n \"$crates\" ]; then"
        ))
    );
}

negative_control!(
    qa_compose_evidence_only_from_an_earlier_passed_quick_attempt,
    "a job whose name only contains quick's read as quick's",
    expected = "qa_compose: quick_passed wrong (a job named like quick's",
    check_evidence(&edited(
        TIER,
        "grep -qxF \"$QUICK_JOB\" <<< \"$names\"",
        "grep -qF ci-quick <<< \"$names\""
    ))
);

// ---- Workflow expressions --------------------------------------------------------------------------------------------

fn str_of(v: &Value) -> &str {
    v.as_str()
        .unwrap_or_else(|| panic!("qa_compose: not a string: {v}"))
}

/// A workflow's text parsed by PyYAML, its `on` key restored.
fn parse(text: &str) -> Value {
    let dir = Scratch::new("qa_compose_yaml");
    std::fs::create_dir_all(&*dir).expect("scratch");
    let file = dir.join("wf.yml");
    std::fs::write(&file, text).expect("workflow");
    let out = Command::new("python3")
        .args([
            "-c",
            "import json, sys, yaml\nd = yaml.safe_load(open(sys.argv[1]))\nif True in d: d['on'] = d.pop(True)\nprint(json.dumps(d))",
        ])
        .arg(&file)
        .timed_output()
        .expect("python ran");
    assert!(
        out.status.success(),
        "qa_compose: yaml: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("json")
}

/// What an expression sees: the `needs` of the job, in its order, with results and outputs; `github.*`; the payload's
/// labels.
#[derive(Default)]
struct Ctx {
    needs: Vec<String>,
    results: BTreeMap<String, String>,
    /// `<job>.<output>`.
    outputs: BTreeMap<String, String>,
    github: BTreeMap<String, String>,
    labels: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
enum V {
    S(String),
    B(bool),
    L(Vec<String>),
}

fn truthy(v: &V) -> bool {
    match v {
        V::S(s) => !s.is_empty(),
        V::B(b) => *b,
        V::L(_) => true,
    }
}

fn render(v: &V) -> String {
    match v {
        V::S(s) => s.clone(),
        V::B(b) => b.to_string(),
        V::L(l) => panic!("qa_compose: a list rendered: {l:?}"),
    }
}

fn tokens(src: &str) -> Vec<String> {
    let c: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < c.len() {
        let ch = c[i];
        if ch.is_whitespace() {
            i += 1;
        } else if ch == '\'' {
            let mut j = i + 1;
            while c[j] != '\'' {
                j += 1;
            }
            out.push(c[i..=j].iter().collect());
            i = j + 1;
        } else if i + 1 < c.len()
            && ["==", "!=", "&&", "||"].contains(&&*format!("{ch}{}", c[i + 1]))
        {
            out.push(format!("{ch}{}", c[i + 1]));
            i += 2;
        } else if "()!,".contains(ch) {
            out.push(ch.to_string());
            i += 1;
        } else if ch.is_alphanumeric() || "_.-*".contains(ch) {
            let mut j = i;
            while j < c.len() && (c[j].is_alphanumeric() || "_.-*".contains(c[j])) {
                j += 1;
            }
            out.push(c[i..j].iter().collect());
            i = j;
        } else {
            panic!("qa_compose: expression: unexpected `{ch}` in {src}");
        }
    }
    out
}

struct P<'a> {
    t: Vec<String>,
    i: usize,
    ctx: &'a Ctx,
}

impl P<'_> {
    fn peek(&self) -> Option<&str> {
        self.t.get(self.i).map(String::as_str)
    }
    fn next(&mut self) -> String {
        self.i += 1;
        self.t[self.i - 1].clone()
    }
    fn or(&mut self) -> V {
        let mut l = self.and();
        while self.peek() == Some("||") {
            self.next();
            let r = self.and();
            l = if truthy(&l) { l } else { r };
        }
        l
    }
    fn and(&mut self) -> V {
        let mut l = self.unary();
        while self.peek() == Some("&&") {
            self.next();
            let r = self.unary();
            l = if truthy(&l) { r } else { l };
        }
        l
    }
    fn unary(&mut self) -> V {
        if self.peek() == Some("!") {
            self.next();
            return V::B(!truthy(&self.unary()));
        }
        let l = self.primary();
        match self.peek() {
            Some(op @ ("==" | "!=")) => {
                let eq = op == "==";
                self.next();
                let r = self.primary();
                let same = match (&l, &r) {
                    (V::S(a), V::S(b)) => a.eq_ignore_ascii_case(b),
                    (V::B(a), V::B(b)) => a == b,
                    _ => panic!("qa_compose: expression: comparing {l:?} with {r:?}"),
                };
                V::B(same == eq)
            }
            _ => l,
        }
    }
    fn primary(&mut self) -> V {
        let tok = self.next();
        if tok == "(" {
            let v = self.or();
            assert_eq!(self.next(), ")", "qa_compose: expression: unbalanced");
            return v;
        }
        if let Some(s) = tok.strip_prefix('\'') {
            return V::S(s.trim_end_matches('\'').to_owned());
        }
        if self.peek() == Some("(") {
            self.next();
            let mut args = Vec::new();
            while self.peek() != Some(")") {
                if tok == "join" && args.is_empty() {
                    let what = self.next();
                    assert_eq!(what, "needs.*.result", "qa_compose: join of {what}");
                    args.push(V::S(
                        self.ctx
                            .needs
                            .iter()
                            .map(|n| self.term(&format!("needs.{n}.result")))
                            .map(|v| render(&v))
                            .collect::<Vec<_>>()
                            .join("\u{0}"),
                    ));
                } else {
                    args.push(self.or());
                }
                if self.peek() == Some(",") {
                    self.next();
                }
            }
            self.next();
            return match (tok.as_str(), args.as_slice()) {
                ("always", []) => V::B(true),
                ("join", [V::S(items), V::S(sep)]) => V::S(items.replace('\u{0}', sep)),
                ("contains", [V::L(l), V::S(s)]) => {
                    V::B(l.iter().any(|x| x.eq_ignore_ascii_case(s)))
                }
                _ => panic!("qa_compose: expression: the function {tok} on {args:?}"),
            };
        }
        match tok.as_str() {
            "true" => V::B(true),
            "false" => V::B(false),
            _ => self.term(&tok),
        }
    }
    fn term(&self, t: &str) -> V {
        let parts: Vec<&str> = t.split('.').collect();
        match parts.as_slice() {
            ["needs", job, rest @ ..] => {
                assert!(
                    self.ctx.needs.iter().any(|n| n == job),
                    "qa_compose: expression reads needs.{job}, not among the job's needs"
                );
                match rest {
                    ["result"] => V::S(
                        self.ctx
                            .results
                            .get(*job)
                            .cloned()
                            .unwrap_or_else(|| "success".to_owned()),
                    ),
                    ["outputs", key] => V::S(
                        self.ctx
                            .outputs
                            .get(&format!("{job}.{key}"))
                            .cloned()
                            .unwrap_or_default(),
                    ),
                    _ => panic!("qa_compose: expression: the term {t}"),
                }
            }
            ["github", "event", "pull_request", "labels", "*", "name"] => {
                V::L(self.ctx.labels.clone())
            }
            ["github", ..] => V::S(
                self.ctx
                    .github
                    .get(t)
                    .cloned()
                    .unwrap_or_else(|| panic!("qa_compose: expression: the term {t}")),
            ),
            _ => panic!("qa_compose: expression: the term {t}"),
        }
    }
}

fn eval(expr: &str, ctx: &Ctx) -> V {
    let mut p = P {
        t: tokens(expr),
        i: 0,
        ctx,
    };
    let v = p.or();
    assert!(
        p.peek().is_none(),
        "qa_compose: expression: trailing tokens in {expr}"
    );
    v
}

/// An `if:`, with or without `${{ }}`.
fn eval_if(cond: &str, ctx: &Ctx) -> V {
    let c = cond.trim();
    let c = c
        .strip_prefix("${{")
        .and_then(|c| c.strip_suffix("}}"))
        .unwrap_or(c);
    eval(c, ctx)
}

/// A string with each `${{ }}` replaced by its value.
fn interpolate(text: &str, ctx: &Ctx) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(at) = rest.find("${{") {
        out.push_str(&rest[..at]);
        let end = rest[at..].find("}}").expect("closing }}") + at;
        out.push_str(&render(&eval(&rest[at + 3..end], ctx)));
        rest = &rest[end + 2..];
    }
    out.push_str(rest);
    out
}

fn needs_of(job: &Value) -> Vec<String> {
    match &job["needs"] {
        Value::String(s) => vec![s.clone()],
        Value::Array(a) => a.iter().map(|v| str_of(v).to_owned()).collect(),
        _ => vec![],
    }
}

/// The jobs of `ci.yml` named by a one-value `matrix.check`, by that check.
fn gates(wf: &Value) -> BTreeMap<String, (String, Value)> {
    wf["jobs"]
        .as_object()
        .expect("jobs")
        .iter()
        .filter_map(|(id, job)| {
            let check = job.pointer("/strategy/matrix/check")?.as_array()?;
            assert_eq!(
                check.len(),
                1,
                "qa_compose: gate {id}: a matrix of one check"
            );
            Some((str_of(&check[0]).to_owned(), (id.clone(), job.clone())))
        })
        .collect()
}

/// The composed full attempt's `changes` outputs: a code pull request's full attempt.
fn composed(passed: bool, covers: bool) -> BTreeMap<String, String> {
    [
        ("changes.quick", "false"),
        ("changes.docs_only", "false"),
        ("changes.gpu", "true"),
        ("changes.tier", "code"),
        ("changes.report", "true"),
        (
            "changes.quick_passed",
            if passed { "true" } else { "false" },
        ),
        (
            "changes.quick_covers_ci",
            if covers { "true" } else { "false" },
        ),
        ("ci-workspace.docs_only", "false"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v.to_owned()))
    .collect()
}

// ---- 4. The gate `ci` passes a skip only on proof ---------------------------------------------------------------------

/// Whether the gate reporting `ci` passes, its gathered jobs ending as `results` (the rest `success`), with the
/// `changes` outputs `outputs`.
fn gate_ci_passes(
    wf: &Value,
    results: &[(&str, &str)],
    outputs: &BTreeMap<String, String>,
) -> bool {
    let (id, job) = gates(wf)
        .remove("ci")
        .expect("qa_compose: no gate reports the required check ci");
    let ctx = Ctx {
        needs: needs_of(&job),
        results: results
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect(),
        outputs: outputs.clone(),
        ..Ctx::default()
    };
    if let Some(c) = job.get("if") {
        assert!(
            truthy(&eval_if(str_of(c), &ctx)),
            "qa_compose: the gate {id} does not run on a full attempt"
        );
    }
    let steps = job["steps"].as_array().expect("steps");
    steps.iter().all(|step| {
        if let Some(c) = step.get("if") {
            if !truthy(&eval_if(str_of(c), &ctx)) {
                return true;
            }
        }
        let Some(run) = step.get("run") else {
            return true;
        };
        let mut cmd = Command::new("bash");
        cmd.args(["-c", &interpolate(str_of(run), &ctx)])
            .env_clear()
            .env("PATH", std::env::var("PATH").expect("PATH"));
        if let Some(env) = step.get("env").and_then(Value::as_object) {
            for (k, v) in env {
                let text = match v {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                cmd.env(k, interpolate(&text, &ctx));
            }
        }
        cmd.timed_output().expect("bash ran").status.success()
    })
}

/// A case of the gate `ci`: what, the gathered jobs' results (the rest `success`), `quick_passed`, `quick_covers_ci`,
/// and whether the gate passes.
type GateCase = (
    &'static str,
    &'static [(&'static str, &'static str)],
    bool,
    bool,
    bool,
);

fn check_gate_ci(ci: &str) {
    let wf = parse(ci);
    let cases: &[GateCase] = &[
        // (what, results, quick_passed, quick_covers_ci, passes)
        ("every job passed", &[], false, false, true),
        ("every job passed, composed", &[], true, true, true),
        (
            "the shards skipped on the proof",
            &[("ci", "skipped")],
            true,
            true,
            true,
        ),
        (
            "the shards skipped without the proof",
            &[("ci", "skipped")],
            false,
            false,
            false,
        ),
        (
            "the shards skipped on quick_passed alone",
            &[("ci", "skipped")],
            true,
            false,
            false,
        ),
        (
            "the shards failed, the proof given",
            &[("ci", "failure")],
            true,
            true,
            false,
        ),
        (
            "the shards cancelled, the proof given",
            &[("ci", "cancelled")],
            true,
            true,
            false,
        ),
        (
            "ci-checks skipped beside the proved shards",
            &[("ci", "skipped"), ("ci-checks", "skipped")],
            true,
            true,
            false,
        ),
        (
            "ci-workspace skipped beside the proved shards",
            &[("ci", "skipped"), ("ci-workspace", "skipped")],
            true,
            true,
            false,
        ),
        (
            "ci-release skipped beside the proved shards",
            &[("ci", "skipped"), ("ci-release", "skipped")],
            true,
            true,
            false,
        ),
        (
            "ci-release failed beside the proved shards",
            &[("ci", "skipped"), ("ci-release", "failure")],
            true,
            true,
            false,
        ),
        (
            "ci-checks skipped, the shards run, the proof given",
            &[("ci-checks", "skipped")],
            true,
            true,
            false,
        ),
        (
            "ci-workspace skipped, the shards run, the proof given",
            &[("ci-workspace", "skipped")],
            true,
            true,
            false,
        ),
    ];
    for (what, results, passed, covers, passes) in cases {
        let got = gate_ci_passes(&wf, results, &composed(*passed, *covers));
        assert_eq!(
            got,
            *passes,
            "qa_compose: the gate ci {} ({what})",
            if *passes {
                "failed"
            } else {
                "passed on a skip or a failure"
            }
        );
    }
}

#[test]
fn qa_compose_gate_ci_passes_a_skip_only_on_proof() {
    check_gate_ci(&read(CI));
}

mod skip_without_proof {
    use super::*;
    negative_control!(
        qa_compose_gate_ci_passes_a_skip_only_on_proof,
        "the gate passing the shards' skip without quick's proof",
        expected = "qa_compose: the gate ci passed on a skip or a failure (the shards skipped without the proof)",
        check_gate_ci(&edited(
            CI,
            "needs.ci.result == 'skipped' && needs.changes.outputs.quick_covers_ci == 'true'",
            "needs.ci.result == 'skipped'"
        ))
    );
}

mod skip_any_job {
    use super::*;
    negative_control!(
        qa_compose_gate_ci_passes_a_skip_only_on_proof,
        "the gate passing a skip of any job on the proof",
        expected = "qa_compose: the gate ci passed on a skip or a failure (ci-checks skipped, the shards run",
        check_gate_ci(&edited(
            CI,
            "needs.ci.result == 'skipped' && needs.changes.outputs.quick_covers_ci == 'true'",
            "needs.changes.outputs.quick_covers_ci == 'true'"
        ))
    );
}

negative_control!(
    qa_compose_gate_ci_passes_a_skip_only_on_proof,
    "the gate's proof passing every skip, not one",
    expected = "qa_compose: the gate ci passed on a skip or a failure (ci-checks skipped beside the proved shards)",
    check_gate_ci(&edited(CI, " && COVERED=spent", ""))
);

// ---- 5. Every full-only required check still reports on the composed attempt ----------------------------------------

/// Whether `job`'s `if:` holds with `outputs`, every gathered job having passed.
fn runs(job: &Value, outputs: &BTreeMap<String, String>) -> bool {
    let ctx = Ctx {
        needs: needs_of(job),
        outputs: outputs.clone(),
        github: [
            ("github.event_name", "pull_request"),
            ("github.ref", "refs/pull/42/merge"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect(),
        ..Ctx::default()
    };
    job.get("if")
        .is_none_or(|c| truthy(&eval_if(str_of(c), &ctx)))
}

/// Whether the step of `job` whose `run:` is `command` runs with `outputs`.
fn step_runs(job: &Value, command: &str, outputs: &BTreeMap<String, String>) -> bool {
    let ctx = Ctx {
        needs: needs_of(job),
        outputs: outputs.clone(),
        github: [
            ("github.event_name", "pull_request"),
            ("github.ref", "refs/pull/42/merge"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect(),
        ..Ctx::default()
    };
    let step = job["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .find(|s| s.get("run").is_some_and(|r| str_of(r).trim() == command))
        .unwrap_or_else(|| panic!("qa_compose: no step runs `{command}`"));
    step.get("if")
        .is_none_or(|c| truthy(&eval_if(str_of(c), &ctx)))
}

fn check_reports(ci: &str) {
    let wf = parse(ci);
    let gates = gates(&wf);
    let names: Vec<&str> = gates.keys().map(String::as_str).collect();
    for check in FULL_ONLY {
        assert!(
            names.contains(&check),
            "qa_compose: no gate reports the required check {check}: {names:?}"
        );
    }
    for (passed, covers) in [(false, false), (true, false), (true, true)] {
        let outputs = composed(passed, covers);
        for (check, (id, job)) in &gates {
            assert!(
                runs(job, &outputs),
                "qa_compose: the gate {id} ({check}) does not run on a composed full attempt (passed {passed}, covers {covers})"
            );
        }
        let jobs = &wf["jobs"];
        // The shards skip only on quick's proof over every crate.
        assert_eq!(
            runs(&jobs["ci-archive"], &outputs),
            !covers,
            "qa_compose: the shards' archive runs or skips on the wrong proof (passed {passed}, covers {covers})"
        );
        // What quick never runs still runs.
        for (job, command) in [
            ("ci-checks", "cargo doc --workspace --no-deps"),
            ("ci-checks", "cargo xtask deps"),
        ] {
            assert!(
                runs(&jobs[job], &outputs) && step_runs(&jobs[job], command, &outputs),
                "qa_compose: `{command}`, which quick never runs, skipped on the composed attempt (passed {passed}, covers {covers})"
            );
        }
        for job in [
            "ci-checks",
            "ci-workspace",
            "ci-release",
            "xtask-ci",
            "gpu-kernel",
            "gpu-metal",
            "gpu-lavapipe",
        ] {
            assert!(
                runs(&jobs[job], &outputs),
                "qa_compose: the job {job}, which quick does not cover, skipped on the composed attempt (passed {passed}, covers {covers})"
            );
        }
        // fmt, clippy and the build skip only on quick's pass.
        for command in [
            "cargo fmt --all --check",
            "cargo clippy --workspace --all-targets -- -D warnings",
            "cargo build --workspace",
        ] {
            assert_eq!(
                step_runs(&jobs["ci-checks"], command, &outputs),
                !passed,
                "qa_compose: `{command}` runs or skips on the wrong proof (passed {passed}, covers {covers})"
            );
        }
    }
    // The quick run itself: no full-only gate runs, so none can pass on a skip; each stays pending.
    let mut quick = composed(false, false);
    quick.insert("changes.quick".to_owned(), "true".to_owned());
    quick.insert("changes.docs_only".to_owned(), "true".to_owned());
    for (check, (id, job)) in &gates {
        assert!(
            !runs(job, &quick),
            "qa_compose: the gate {id} ({check}) runs on a quick run"
        );
    }
}

#[test]
fn qa_compose_every_required_full_only_check_still_reports() {
    check_reports(&read(CI));
}

mod gate_skips_on_proof {
    use super::*;
    negative_control!(
        qa_compose_every_required_full_only_check_still_reports,
        "a gate skipped on the composed attempt",
        expected =
            "qa_compose: the gate xtask-ci-gate (xtask-ci) does not run on a composed full attempt",
        check_reports(&{
            let text = read(CI);
            let at = text.find("  xtask-ci-gate:").expect("xtask-ci-gate");
            let from = "    if: always() && needs.changes.outputs.quick != 'true'\n";
            let rel = text[at..].find(from).expect("its if") + at;
            format!(
                "{}    if: always() && needs.changes.outputs.quick != 'true' && needs.changes.outputs.quick_passed != 'true'\n{}",
                &text[..rel],
                &text[rel + from.len()..]
            )
        })
    );
}

mod shards_on_some_crates {
    use super::*;
    negative_control!(
        qa_compose_every_required_full_only_check_still_reports,
        "the shards skipped on quick_passed alone",
        expected = "qa_compose: the shards' archive runs or skips on the wrong proof (passed true, covers false)",
        check_reports(&edited(
            CI,
            "needs.changes.outputs.docs_only != 'true' && needs.changes.outputs.quick_covers_ci != 'true'\n    runs-on",
            "needs.changes.outputs.docs_only != 'true' && needs.changes.outputs.quick_passed != 'true'\n    runs-on"
        ))
    );
}

negative_control!(
    qa_compose_every_required_full_only_check_still_reports,
    "cargo doc skipped on quick's pass, though quick never runs it",
    expected = "qa_compose: `cargo doc --workspace --no-deps`, which quick never runs, skipped",
    check_reports(&edited(
        CI,
        "        run: cargo doc --workspace --no-deps",
        "        if: needs.changes.outputs.quick_passed != 'true'\n        run: cargo doc --workspace --no-deps"
    ))
);
