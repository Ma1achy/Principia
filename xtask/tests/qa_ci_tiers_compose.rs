//! QA's tests for the CI tooling change "the full tier adds to the quick tier" (no task id; the owner, 7 Nov 2026:
//! "Yeah if adding the label cancels the quick tier that needs to be changed lol if not don't worry / The full tier
//! should compose / add on to the quick tier"), written from that order and the rulings it must keep, not from the
//! implementation.
//!
//! The owner's CI model (approved 9 Nov 2026) replaces the quick and full tiers with one PR tier, and the full tier
//! with the per-merge run on `main`: no label starts, re-runs or cancels anything on a pull request, and the PR tier's
//! `ci` tests the changed crates and their reverse dependencies. So the five tests that followed the label's full
//! attempt through `ci_tier.sh`'s waits and re-runs are retired with that machinery (R-290's exception, applied per
//! R-369):
//! - `qa_compose_label_mid_quick_never_cancels_it` and `qa_compose_a_push_still_takes_the_label_off`: there is no
//!   label-started attempt to wait for; what the order wanted, that a label never cancels the tier a head is running,
//!   `qa_compose_a_label_never_starts_or_cancels_the_pr_tier` keeps, below.
//! - `qa_compose_evidence_only_from_an_earlier_passed_quick_attempt`: no attempt carries evidence into another; what
//!   it guarded, that the tests' coverage is read only from what is shown and fails towards every crate,
//!   `qa_compose_the_pr_tier_reads_its_crates_from_the_change_alone` keeps, below.
//! - `qa_compose_gate_ci_passes_a_skip_only_on_proof` and `qa_compose_every_required_full_only_check_still_reports`:
//!   the gate jobs and the full-only checks are gone; that every required check reports on every pull request,
//!   skipped by none, `qa_tooling_pr_tier` and `qa_mutants_check_always` pin, and that what the PR tier's tests never
//!   run (cargo doc, the crate graph, the controls listing) still runs, `qa_tooling_pr_tier`'s
//!   `qa_pr_tier_build_runs_the_product_rules`.
//!
//! The tests:
//! - `qa_compose_a_label_never_starts_or_cancels_the_pr_tier`: `ci.yml`, which bears the PR tier, runs on no label
//!   change, its concurrency group reads no label nor event action (so no label's run shares a head's group), and no
//!   workflow step calls GitHub's `/cancel`, `/force-cancel` or `/rerun`.
//! - `qa_compose_the_pr_tier_reads_its_crates_from_the_change_alone`: `ci_tier.sh decide` run under bash with a
//!   stand-in classifier and a stand-in `gh` that records every call: on a pull request it names the changed crate,
//!   not every crate; on a push, or with the change unread, every crate (`--workspace`), for the tests and the
//!   doctests; and it never calls the GitHub API.
//!
//! Each test has its registered negative controls (R-176), the same check on a workflow or script edited in the one
//! respect it turns on. Unix only: the stand-ins are executable scripts found through `PATH`.
#![cfg(unix)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use validation::negative_control;
use validation::spawn::{write_executable, Spawn};

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;

const TIER: &str = ".github/scripts/ci_tier.sh";
const CRATES: &str = ".github/scripts/ci_quick_crates.py";
const CI: &str = ".github/workflows/ci.yml";

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
fn edited(rel: &str, from: &str, to: &str) -> String {
    let text = read(rel);
    assert!(
        text.contains(from),
        "qa_compose: the control's edit target is gone from {rel}: {from}"
    );
    text.replacen(from, to, 1)
}

// ---- 1. A label never starts or cancels the PR tier -----------------------------------------------------------------

/// The lines of `yml`'s top-level `key:` block, the key's own line included.
fn block<'a>(yml: &'a str, key: &str) -> Vec<&'a str> {
    let head = format!("{key}:");
    yml.lines()
        .skip_while(|l| !l.starts_with(&head))
        .enumerate()
        .take_while(|(i, l)| *i == 0 || l.is_empty() || l.starts_with(' ') || l.starts_with('#'))
        .map(|(_, l)| l)
        .filter(|l| !l.trim_start().starts_with('#'))
        .collect()
}

fn check_label(ci: &str, workflows: &[(String, String)]) {
    let on = block(ci, "on");
    assert!(
        on.iter().any(|l| l.trim() == "pull_request:"),
        "qa_compose: ci.yml does not run on pull requests"
    );
    assert!(
        !on.iter().any(|l| l.contains("labeled")),
        "qa_compose: ci.yml's PR tier runs on a label change: {on:?}"
    );
    let group = block(ci, "concurrency").join("\n");
    assert!(
        group.contains("group:"),
        "qa_compose: ci.yml has no concurrency group"
    );
    for word in ["github.event.label", "github.event.action", "labels"] {
        assert!(
            !group.contains(word),
            "qa_compose: ci.yml's concurrency group reads the label or the event's action (`{word}`)"
        );
    }
    for (name, text) in workflows {
        for line in text.lines().filter(|l| !l.trim_start().starts_with('#')) {
            for call in ["/cancel", "/force-cancel", "/rerun"] {
                assert!(
                    !line.contains(call),
                    "qa_compose: {name} calls `{call}`: {line}"
                );
            }
        }
    }
}

/// Every workflow under `.github/workflows/`, `ci.yml` as `ci` gives it.
fn workflows(ci: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = std::fs::read_dir(root().join(".github/workflows"))
        .expect("qa_compose: workflows")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".yml") || n.ends_with(".yaml"))
        .map(|n| {
            let t = if n == "ci.yml" {
                ci.to_owned()
            } else {
                read(&format!(".github/workflows/{n}"))
            };
            (n, t)
        })
        .collect();
    out.sort();
    out
}

fn label(ci: &str) {
    check_label(ci, &workflows(ci));
}

#[test]
fn qa_compose_a_label_never_starts_or_cancels_the_pr_tier() {
    label(&read(CI));
}

negative_control!(
    qa_compose_a_label_never_starts_or_cancels_the_pr_tier,
    "ci.yml run on a label put on or taken off",
    expected = "ci.yml's PR tier runs on a label change",
    label(&edited(
        CI,
        "  pull_request:\n",
        "  pull_request:\n    types: [opened, synchronize, reopened, labeled, unlabeled]\n"
    ))
);

mod group_reads_the_label {
    use super::*;
    negative_control!(
        qa_compose_a_label_never_starts_or_cancels_the_pr_tier,
        "ci.yml's concurrency group keyed on the label too",
        expected = "ci.yml's concurrency group reads the label",
        label(&edited(
            CI,
            "  group: ci-${{ github.event_name }}-",
            "  group: ci-${{ github.event_name }}-${{ github.event.label.name }}-"
        ))
    );
}

mod a_step_cancels {
    use super::*;
    negative_control!(
        qa_compose_a_label_never_starts_or_cancels_the_pr_tier,
        "a ci.yml step cancelling the run before it",
        expected = "ci.yml calls `/cancel`",
        label(&edited(
            CI,
            "        run: .github/scripts/ci_tier.sh decide\n",
            "        run: gh api -X POST repos/o/r/actions/runs/1/cancel; .github/scripts/ci_tier.sh decide\n"
        ))
    );
}

// ---- 2. The PR tier reads its crates from the change alone ----------------------------------------------------------

/// A stand-in `gh` that records each call and fails.
const GH: &str = "#!/bin/bash\necho \"gh $*\" >> \"$QA_DIR/gh_calls\"\nexit 1\n";

/// `script` (`ci_tier.sh`'s text) run as `decide` on `event`, with the change `paths` (`None`: the classifier fails):
/// its outputs, and the `gh` calls it made.
fn decide(script: &str, event: &str, paths: Option<&[&str]>) -> (BTreeMap<String, String>, String) {
    let dir = Scratch::new("qa_compose");
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).expect("scratch");
    write_executable(&bin.join("gh"), GH).expect("gh stand-in");
    let file = dir.join("ci_tier.sh");
    write_executable(&file, script).expect("script");
    std::fs::write(dir.join("ci_quick_crates.py"), read(CRATES)).expect("crates script");
    let changes = dir.join("changes.sh");
    let body = match paths {
        Some(p) => format!(
            "#!/bin/bash\n[ \"$1\" = paths ] || exit 3\nprintf '%s\\n' {}\n",
            p.join(" ")
        ),
        None => "#!/bin/bash\nexit 1\n".to_owned(),
    };
    write_executable(&changes, body).expect("changes stand-in");
    let output = dir.join("output");
    let path = std::env::var("PATH").expect("PATH");
    let out = Command::new("bash")
        .arg(&file)
        .arg("decide")
        .env_clear()
        .env("PATH", format!("{}:{path}", bin.display()))
        .env("HOME", &*dir)
        .env("QA_DIR", &*dir)
        .env("GITHUB_EVENT_NAME", event)
        .env("GITHUB_OUTPUT", &output)
        .env("CI_CHANGES", &changes)
        .env("CI_ROOT", root())
        .timed_output()
        .expect("qa_compose: bash ran");
    assert!(
        out.status.success(),
        "qa_compose: ci_tier.sh decide failed on {event}: {}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let outputs = std::fs::read_to_string(&output)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
    let calls = std::fs::read_to_string(dir.join("gh_calls")).unwrap_or_default();
    (outputs, calls)
}

fn check_crates(script: &str) {
    const GUI: &[&str] = &["crates/gui/src/lib.rs"];
    let (pr, calls) = decide(script, "pull_request", Some(GUI));
    let packages = pr.get("packages").cloned().unwrap_or_default();
    assert!(
        packages.split_whitespace().any(|w| w == "gui") && !packages.contains("--workspace"),
        "qa_compose: on a pull request changing crates/gui, the tests cover `{packages}`, not the changed crate"
    );
    assert!(
        calls.is_empty(),
        "qa_compose: ci_tier.sh calls the GitHub API: {calls}"
    );
    for (event, paths, why) in [
        ("push", Some(GUI), "on `push`"),
        ("pull_request", None, "with the change unread"),
    ] {
        let (got, calls) = decide(script, event, paths);
        for key in ["packages", "doc_packages"] {
            assert_eq!(
                got.get(key).map(String::as_str),
                Some("--workspace"),
                "qa_compose: {why}, the tests cover `{key}` = {:?}, not every crate",
                got.get(key)
            );
        }
        assert!(
            calls.is_empty(),
            "qa_compose: ci_tier.sh calls the GitHub API: {calls}"
        );
    }
}

#[test]
fn qa_compose_the_pr_tier_reads_its_crates_from_the_change_alone() {
    check_crates(&read(TIER));
}

negative_control!(
    qa_compose_the_pr_tier_reads_its_crates_from_the_change_alone,
    "the changed crates chosen on every event",
    expected = "on `push`, the tests cover `packages`",
    check_crates(&edited(
        TIER,
        "  if [ \"${GITHUB_EVENT_NAME:-}\" = pull_request ]; then\n",
        "  if true; then\n"
    ))
);

mod unread_change {
    use super::*;
    negative_control!(
        qa_compose_the_pr_tier_reads_its_crates_from_the_change_alone,
        "a crate guessed when the change cannot be read",
        expected = "with the change unread, the tests cover `packages`",
        check_crates(&edited(
            TIER,
            "    || crates=\"\"\n",
            "    || crates=\"gui\"\n"
        ))
    );
}

mod calls_the_api {
    use super::*;
    negative_control!(
        qa_compose_the_pr_tier_reads_its_crates_from_the_change_alone,
        "ci_tier.sh asking GitHub for the run's attempts",
        expected = "ci_tier.sh calls the GitHub API",
        check_crates(&edited(
            TIER,
            "decide() {\n",
            "decide() {\n  gh api repos/o/r/actions/runs/1/attempts/1/jobs > /dev/null || true\n"
        ))
    );
}

mod every_crate_on_a_pr {
    use super::*;
    negative_control!(
        qa_compose_the_pr_tier_reads_its_crates_from_the_change_alone,
        "every crate tested on a pull request whatever it changes",
        expected = "not the changed crate",
        check_crates(&edited(TIER, "    changed_packages\n", "    :\n"))
    );
}
