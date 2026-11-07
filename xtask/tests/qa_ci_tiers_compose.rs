//! QA's tests for the CI tooling change "the full tier adds to the quick tier" (no task id; the owner, 7 Nov 2026:
//! "Yeah if adding the label cancels the quick tier that needs to be changed lol if not don't worry / The full tier
//! should compose / add on to the quick tier"), written from that order and the rulings it must keep, not from the
//! implementation:
//!
//! - `qa_compose_label_mid_quick_never_cancels_it`: `merge-ready` going on while the head's quick run is queued or in
//!   progress never cancels it (no `/cancel` nor `/force-cancel` call), and the full attempt (`/rerun`, R-276's pattern
//!   kept by R-678) starts only once the run has been seen ended.
//! - `qa_compose_a_push_still_takes_the_label_off`: a push still takes `merge-ready` off (`mutants.yml`), the pushed
//!   head's first attempt is quick whatever the label says, and a push or the label coming off during the wait
//!   starts no full attempt on the old head.
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
/// The pull request's head and labels answer from the line of the latest poll (the first before any). The jobs of
/// attempt `<n>` answer `$QA_DIR/attempt_<n>.json`, failing when it is absent; a run's latest jobs answer
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
    if [ "$status" = none ]; then
      echo '{"total_count":0,"workflow_runs":[]}' | answer
    else
      printf '{"total_count":1,"workflow_runs":[{"id":5,"status":"%s","run_attempt":%s}]}\n' "$status" "$attempt" | answer
    fi ;;
  *"/attempts/"*"/jobs"*)
    args="$*"
    n=${args#*/attempts/}
    n=${n%%/*}
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

/// `merge-ready` put on, in a world of `lines`.
fn label_on(script: &str, lines: &[&str], env: &[(&str, &str)]) -> Ran {
    let mut all = vec![("ACTION", "labeled")];
    all.extend_from_slice(env);
    tier(
        script,
        "merge-ready",
        &World {
            lines,
            attempts: &[],
            latest: None,
            paths: None,
            env: &all,
        },
    )
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

// ---- 1. The label added mid-quick never cancels the quick run -------------------------------------------------------

fn check_never_cancels(script: &str) {
    let cases: &[(&str, &[&str])] = &[
        (
            "in progress, then ended",
            &[
                "in_progress 1 aaa111 merge-ready",
                "in_progress 1 aaa111 merge-ready",
                "in_progress 1 aaa111 merge-ready",
                "completed 1 aaa111 merge-ready",
            ],
        ),
        (
            "queued, then in progress, then ended",
            &[
                "queued 1 aaa111 merge-ready",
                "in_progress 1 aaa111 merge-ready",
                "completed 1 aaa111 merge-ready",
            ],
        ),
        ("already ended", &["completed 1 aaa111 merge-ready"]),
    ];
    for (what, lines) in cases {
        let got = label_on(script, lines, &[]);
        assert_eq!(
            got.count("cancel"),
            0,
            "qa_compose: the label cancelled the quick run ({what}): {:?}",
            got.calls
        );
        assert!(
            got.ok() && got.count("/rerun") == 1,
            "qa_compose: the label did not start the full attempt once ({what}): {:?} {}",
            got.calls,
            got.out
        );
        let seen = seen_at_rerun(&got, lines).unwrap_or("");
        assert!(
            seen.starts_with("completed "),
            "qa_compose: the full attempt was asked for before the quick run ended ({what}; last seen `{seen}`): {:?}",
            got.calls
        );
    }
    // A quick run that outlasts the bounded wait is neither cancelled nor re-run; the job fails, so the full-only checks
    // stay pending, never passed.
    let got = label_on(script, &["in_progress 1 aaa111 merge-ready"], &[]);
    assert!(
        !got.ok() && got.count("cancel") == 0 && got.count("/rerun") == 0,
        "qa_compose: a quick run outlasting the wait was cancelled or re-run, or the job passed: {:?} {}",
        got.calls,
        got.out
    );
}

#[test]
fn qa_compose_label_mid_quick_never_cancels_it() {
    check_never_cancels(&read(TIER));
}

/// The fault the owner's order removes: the label cancelling the quick run, then re-running it.
mod cancels_first {
    use super::*;
    negative_control!(
        qa_compose_label_mid_quick_never_cancels_it,
        "the wait cancelling the run it waits for",
        expected = "qa_compose: the label cancelled the quick run",
        check_never_cancels(&edited(
            TIER,
            "wait_for_end() {\n",
            "wait_for_end() {\n  gh api --method POST \"repos/$GH_REPO/actions/runs/5/cancel\" > /dev/null || true\n"
        ))
    );
}

negative_control!(
    qa_compose_label_mid_quick_never_cancels_it,
    "the wait returning at once, the run still going",
    expected = "qa_compose: the full attempt was asked for before the quick run ended",
    check_never_cancels(&edited(
        TIER,
        "wait_for_end() {\n",
        "wait_for_end() {\n  latest_run\n  return 0\n"
    ))
);

// ---- 2. A push still takes merge-ready off ---------------------------------------------------------------------------

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
    // A push during the wait: the old head's run ends (cancelled by the push's run), and nothing is re-run on it, even
    // before the push's `merge-ready-off` has taken the label off.
    let pushed = [
        "in_progress 1 aaa111 merge-ready",
        "completed 1 bbb222 merge-ready",
    ];
    let got = label_on(script, &pushed, &[]);
    assert!(
        got.ok() && got.count("/rerun") == 0 && got.count("cancel") == 0,
        "qa_compose: a push during the wait still started the full attempt on the old head: {:?} {}",
        got.calls,
        got.out
    );
    // The label taken off during the wait, the head unchanged: nothing is re-run.
    let off = ["in_progress 1 aaa111 merge-ready", "completed 1 aaa111 -"];
    let got = label_on(script, &off, &[]);
    assert!(
        got.ok() && got.count("/rerun") == 0 && got.count("cancel") == 0,
        "qa_compose: the label off by the quick run's end still started the full attempt: {:?} {}",
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
        "the wait's end re-running the head it was given, not the live one",
        expected = "qa_compose: a push during the wait",
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
    "the wait's end not reading the live label",
    expected = "qa_compose: the label off by the quick run's end",
    check_push(
        &edited(
            TIER,
            "    if ! grep -qxF merge-ready <<< \"$labels\"; then\n",
            "    if false; then\n"
        ),
        &read(MUTANTS)
    )
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
