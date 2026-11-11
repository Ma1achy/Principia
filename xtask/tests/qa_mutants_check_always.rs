//! qa's tests for R-678, written from the owner's words and the ruling, not from `mutants.yml`'s code:
//!
//! - The owner, 3 Nov 2026 (R-678, the later message, which governs): "`mutants-check` must report on EVERY
//!   pull_request event and label change. Success with a "not applicable" summary when no crates/ code changed or the
//!   PR isn't merge-ready; the real result once mutants ran on the merge-ready head; never left unreported. Add a check
//!   that every required check name reports in all paths (label added, removed, docs-only, plan-data, code)."
//! - R-678: `mutants-check` is a job of every run of `mutants.yml`, never a check run posted through the Checks API;
//!   branch protection counts, of each workflow's runs on a head, only the latest run's check suite; another label put
//!   on or taken off a merge-ready head reports that head's merge-ready verdict, and with none on the head, a failure.
//! - `plan/HUMAN_SETUP.md` §2 lists the checks branch protection requires on `main`, `mutants-check` among them (the
//!   owner will add it back once this is verified).
//!
//! One test per path the owner named, each in two parts:
//! - every required check reports: of the workflows a pull request runs, exactly one job bears each name; its workflow
//!   filters no pull request by path or branch, runs on the event or, where it does not, on every new head (so its
//!   push run's suite stands), and runs on a label taken off wherever it runs on one put on (so a label's run never
//!   stands for a label since removed); and the job runs whatever happened before it: no `if:` and no `needs:`, or
//!   `if: always()`;
//! - `mutants.yml` run as GitHub runs it on the path's events, against a stand-in GitHub (the pull request's head and
//!   labels, and the workflow's runs with their jobs, in a file a stand-in `gh` serves): its `if:`s evaluated, the
//!   real classifier (`scripts/ci_changes.sh classify`) reading the path's change, the label and report steps run
//!   under bash, and the mutants work ending in the verdict given. Branch protection's reading of `mutants-check` is
//!   then the job in the head's latest run, and it must be what the owner said.
//!
//! The paths: `qa_mca_label_added`, `qa_mca_label_removed`, `qa_mca_docs_only`, `qa_mca_plan_data`, `qa_mca_code`.
//! Each has its negative controls (R-176), a workflow differing in the one respect the path turns on.
//!
//! The owner's CI model (approved 9 Nov 2026) takes per-PR mutation testing off the pull request's path, and
//! `mutants-check` off the required checks, which become the PR tier's nine (`plan/HUMAN_SETUP.md` §2). So the second
//! part of each test, `mutants.yml` run against a stand-in GitHub, is retired with it, and the stand-ins with it; the
//! first part, the owner's "check that every required check name reports in all paths (label added, removed,
//! docs-only, plan-data, code)", stands, on the new list: each of the nine is borne by one job of the workflows a pull
//! request runs, unfiltered, on every new head, and skipped by none (R-290's exception, applied per R-369).

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::json;
use validation::negative_control;
use validation::spawn::Spawn;

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("qa_mca: {rel}: {e}"))
}

/// `text` with `from` replaced once by `to`; `from` must be there, so a control never edits nothing.
#[cfg(feature = "controls")]
fn swap(text: &str, from: &str, to: &str) -> String {
    assert!(
        text.contains(from),
        "qa_mca: the control's edit target is gone: {from}"
    );
    text.replacen(from, to, 1)
}

/// Every workflow under `.github/workflows/`, by file name.
fn workflows() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = std::fs::read_dir(root().join(".github/workflows"))
        .expect("qa_mca: workflows")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".yml") || n.ends_with(".yaml"))
        .map(|n| {
            let t = read(&format!(".github/workflows/{n}"));
            (n, t)
        })
        .collect();
    out.sort();
    out
}

/// The workflows with `file`'s text replaced by `text`.
#[cfg(feature = "controls")]
fn with(file: &str, text: &str) -> Vec<(String, String)> {
    workflows()
        .into_iter()
        .map(|(n, t)| {
            if n == file {
                (n, text.to_owned())
            } else {
                (n, t)
            }
        })
        .collect()
}

// ---- The report check ----------------------------------------------------------------------------------------------

/// The check: `report` lists, for a pull request event, each required check that would not report on it. (`run`, which
/// ran `mutants.yml` on a stand-in GitHub, is retired with `mutants-check` from the pull request's path.)
const RUNNER: &str = r#"
import json, os, sys
import yaml

def load(path):
    d = yaml.safe_load(open(path))
    if True in d:
        d["on"] = d.pop(True)
    return d

def types_of(wf):
    pr = (wf["on"] or {}).get("pull_request") if isinstance(wf["on"], dict) else None
    return (pr or {}).get("types") or ["opened", "synchronize", "reopened"]

def needs_of(job):
    n = job.get("needs") or []
    return [n] if isinstance(n, str) else list(n)

def runs_on_prs(wf):
    on = wf["on"]
    return (isinstance(on, dict) and "pull_request" in on) or on == "pull_request" or \
        (isinstance(on, list) and "pull_request" in on)

QUICK_GATE = "always() && needs.changes.outputs.quick != 'true'"
# The owner's CI model (9 Nov 2026): the PR tier's required jobs run on every pull request and skip inside the job.
# `!cancelled()` since 10 Nov 2026, where `always()` kept a superseded run's jobs running: it too runs the job whatever
# its `needs:` ended in.
PR_TIER_GATE = "${{ !cancelled() && github.event_name == 'pull_request' }}"
NOT_CANCELLED = "${{ !cancelled() }}"

def bears(job, jid):
    name, only = str(job.get("name", jid)), ((job.get("strategy") or {}).get("matrix") or {}).get("check")
    return str(only[0]) if name == "${{ matrix.check }}" and isinstance(only, list) and len(only) == 1 else name

def report(args):
    files, names, action = json.loads(args[0]), json.loads(args[1]), args[2]
    wfs = [(f, load(f)) for f in files]
    wfs = [(f, wf) for f, wf in wfs if runs_on_prs(wf)]
    problems = []
    for name in names:
        bearers = [(f, wf, jid, job) for f, wf in wfs for jid, job in (wf.get("jobs") or {}).items()
                   if bears(job, jid) == name]
        if len(bearers) != 1:
            problems.append("`%s` is borne by %d jobs of the pull-request workflows, not one" % (name, len(bearers)))
            continue
        f, wf, jid, job = bearers[0]
        f = os.path.basename(f)
        pr = wf["on"].get("pull_request") if isinstance(wf["on"], dict) else None
        for key in ("paths", "paths-ignore", "branches", "branches-ignore"):
            if pr and key in pr:
                problems.append("`%s`: %s filters its pull requests with `%s:`" % (name, f, key))
        types = types_of(wf)
        if ("labeled" in types) != ("unlabeled" in types):
            problems.append("`%s`: %s runs on one label change and not the other" % (name, f))
        if action not in types:
            if not all(t in types for t in ("opened", "synchronize", "reopened")):
                problems.append("`%s` on `%s`: %s runs neither on it nor on every new head" % (name, action, f))
            continue
        cond = job.get("if")
        # R-684: `ci.yml`'s gates skip on a quick run alone, under their raw `name:`, and report on any other.
        if not ((cond is None and not needs_of(job)) or str(cond).strip() in ("always()", NOT_CANCELLED, QUICK_GATE, PR_TIER_GATE)):
            problems.append("`%s` on `%s`: %s's job `%s` can be skipped (if: %s)" % (name, action, f, jid, cond))
    print(json.dumps(problems))

ROOT = os.environ["QA_ROOT"]
{"report": report}[sys.argv[1]](sys.argv[2:])
"#;

// ---- The required checks, as plan/HUMAN_SETUP.md §2 records them --------------------------------------------

/// The names in §2's "require status checks to pass: …" (before its parenthesis) and "require `…` too".
fn required(setup: &str) -> Vec<String> {
    let section: String = setup
        .lines()
        .skip_while(|l| !l.starts_with("## 2."))
        .skip(1)
        .take_while(|l| !l.starts_with("## "))
        .map(|l| format!("{l}\n"))
        .collect();
    let mut names = Vec::new();
    for bullet in section.split("\n- ").map(|b| b.replace('\n', " ")) {
        let ticks = |t: &str| -> Vec<String> {
            t.split('`').skip(1).step_by(2).map(str::to_owned).collect()
        };
        if let Some(rest) = bullet
            .trim_start_matches("- ")
            .strip_prefix("require status checks to pass:")
        {
            names.extend(ticks(rest.split('(').next().unwrap_or_default()));
        } else if let Some(rest) = bullet.trim_start_matches("- ").strip_prefix("require `") {
            if let Some((name, after)) = rest.split_once('`') {
                if after.trim_start().starts_with("too") {
                    names.push(name.to_owned());
                }
            }
        }
    }
    names
}

/// Every required check `plan/HUMAN_SETUP.md` §2 (`setup`) lists reports on a pull request's `action`, as the module
/// docs say; the assertion names each that does not.
fn check_reports(workflows: &[(String, String)], setup: &str, action: &str) {
    let names = required(setup);
    // The PR tier's nine since the owner's CI model (9 Nov 2026), `mutants-check` no longer among them.
    assert!(
        names.len() >= 9 && names.iter().any(|n| n == "fmt"),
        "qa_mca: plan/HUMAN_SETUP.md §2 names {names:?}, not the nine required checks"
    );
    let dir = Scratch::new("qa_mca_report");
    std::fs::create_dir_all(&*dir).expect("scratch");
    std::fs::write(dir.join("runner.py"), RUNNER).expect("runner");
    let files: Vec<String> = workflows
        .iter()
        .map(|(n, t)| {
            let f = dir.join(n);
            std::fs::write(&f, t).expect("workflow");
            f.display().to_string()
        })
        .collect();
    let out = Command::new("python3")
        .arg(dir.join("runner.py"))
        .args([
            "report",
            &json!(files).to_string(),
            &json!(names).to_string(),
            action,
        ])
        .env("QA_ROOT", root())
        .timed_output()
        .expect("qa_mca: runner ran");
    assert!(
        out.status.success(),
        "qa_mca: the report check failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let problems: Vec<String> = serde_json::from_slice(&out.stdout).expect("qa_mca: report JSON");
    assert!(
        problems.is_empty(),
        "qa_mca: a required check does not report on `{action}`: {}",
        problems.join("; ")
    );
}

/// The owner's CI model's required checks (9 Nov 2026), as `plan/HUMAN_SETUP.md` §2 must list them; R-909's PR tier
/// names "check_plan.py, reviews-complete, pr-check, no-atlas-params".
const NINE: [&str; 9] = [
    "fmt",
    "clippy",
    "build",
    "ci",
    "gpu-lavapipe",
    "gpu-metal",
    "pr-check",
    "reviews-complete",
    "no-atlas-params",
];

/// §2 lists exactly the nine, and every one reports on each of `actions`.
fn reports_on(workflows: &[(String, String)], setup: &str, actions: &[&str]) {
    let mut names = required(setup);
    names.sort();
    let mut want: Vec<String> = NINE.iter().map(|n| (*n).to_owned()).collect();
    want.sort();
    assert_eq!(
        names, want,
        "qa_mca: plan/HUMAN_SETUP.md §2 names other than the PR tier's nine"
    );
    for action in actions {
        check_reports(workflows, setup, action);
    }
}

/// `file`'s text in the workflows edited once, `from` to `to`.
#[cfg(feature = "controls")]
fn edited(file: &str, from: &str, to: &str) -> Vec<(String, String)> {
    with(
        file,
        &swap(&read(&format!(".github/workflows/{file}")), from, to),
    )
}

// The second part of each path, `mutants.yml` on a stand-in GitHub (`Pr`, `FAKE_GH`, `FAKE_SLEEP` and the runner's
// `run`), is retired with `mutants-check` from the pull request's path; `RUNNER`'s `report` stays.

// ---- Label added --------------------------------------------------------------------------------------------

/// A label put on: no workflow bearing a required check runs on one change of a label and not the other, and each
/// either runs on it, unskippable, or runs on every new head, so its head's last run stands.
fn label_added(workflows: &[(String, String)]) {
    reports_on(workflows, &read("plan/HUMAN_SETUP.md"), &["labeled"]);
}

#[test]
fn qa_mca_label_added() {
    label_added(&workflows());
}

negative_control!(
    qa_mca_label_added,
    "reviews.yml run on a label put on but not on one taken off",
    expected = "`reviews-complete`: reviews.yml runs on one label change and not the other",
    label_added(&edited(
        "reviews.yml",
        "  pull_request:\n  pull_request_review:\n",
        "  pull_request:\n    types: [opened, synchronize, reopened, labeled]\n  pull_request_review:\n",
    ))
);

// ---- Label removed ------------------------------------------------------------------------------------------

fn label_removed(workflows: &[(String, String)]) {
    reports_on(workflows, &read("plan/HUMAN_SETUP.md"), &["unlabeled"]);
}

#[test]
fn qa_mca_label_removed() {
    label_removed(&workflows());
}

negative_control!(
    qa_mca_label_removed,
    "ci.yml run on no label change and on no new head",
    expected = "`fmt` on `unlabeled`: ci.yml runs neither on it nor on every new head",
    label_removed(&edited(
        "ci.yml",
        "  pull_request:\n",
        "  pull_request:\n    types: [opened, reopened]\n",
    ))
);

// ---- Docs-only, plan-data and code --------------------------------------------------------------------------

/// A head's push, opened, a new head and reopened: every required check reports, whatever the head changes, since no
/// workflow bearing one filters its pull requests and no job bearing one skips at job level.
fn pushes(workflows: &[(String, String)]) {
    reports_on(
        workflows,
        &read("plan/HUMAN_SETUP.md"),
        &["opened", "synchronize", "reopened"],
    );
}

#[test]
fn qa_mca_docs_only() {
    pushes(&workflows());
}

negative_control!(
    qa_mca_docs_only,
    "ci.yml skipped on a docs-only pull request by a path filter",
    expected = "`fmt`: ci.yml filters its pull requests with `paths-ignore:`",
    pushes(&edited(
        "ci.yml",
        "  pull_request:\n",
        "  pull_request:\n    paths-ignore: ['docs/**']\n",
    ))
);

#[test]
fn qa_mca_plan_data() {
    pushes(&workflows());
}

negative_control!(
    qa_mca_plan_data,
    "`build` skipped at job level on a plan-data head",
    expected = "`build` on `opened`: ci.yml's job `build` can be skipped",
    pushes(&edited(
        "ci.yml",
        "  build:\n    needs: changes\n    if: ${{ !cancelled() && github.event_name == 'pull_request' }}\n",
        "  build:\n    needs: changes\n    if: needs.changes.outputs.docs_only != 'true'\n",
    ))
);

#[test]
fn qa_mca_code() {
    pushes(&workflows());
}

negative_control!(
    qa_mca_code,
    "the Metal smoke set run only when the classification passed",
    expected = "`gpu-metal` on `opened`: ci.yml's job `gpu-metal-smoke` can be skipped",
    pushes(&edited(
        "ci.yml",
        "    name: gpu-metal\n    needs: changes\n    if: ${{ !cancelled() }}\n",
        "    name: gpu-metal\n    needs: changes\n    if: success()\n",
    ))
);

mod human_setup_list {
    use super::*;
    negative_control!(
        qa_mca_code,
        "plan/HUMAN_SETUP.md §2 requiring a check no workflow bears",
        expected = "`mutants-check` is borne by 0 jobs of the pull-request workflows, not one",
        check_reports(
            &workflows(),
            &swap(
                &read("plan/HUMAN_SETUP.md"),
                "`reviews-complete` and `no-atlas-params` (",
                "`reviews-complete`, `mutants-check` and `no-atlas-params` ("
            ),
            "opened"
        )
    );
}

mod human_setup_list_short {
    use super::*;
    negative_control!(
        qa_mca_code,
        "plan/HUMAN_SETUP.md §2 without `clippy`",
        expected = "plan/HUMAN_SETUP.md §2 names other than the PR tier's nine",
        reports_on(
            &workflows(),
            &swap(
                &read("plan/HUMAN_SETUP.md"),
                "`fmt`, `clippy`, `build`,",
                "`fmt`, `build`,"
            ),
            &["opened"]
        )
    );
}
