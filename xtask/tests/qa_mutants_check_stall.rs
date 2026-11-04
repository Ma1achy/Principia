//! qa's tests for PR #389's fix round 3, written from the owner's words, not from `mutants.yml`'s code:
//!
//! - The owner, 3 Nov 2026 (R-678): "`mutants-check` must report on EVERY pull_request event and label change. [...]
//!   the real result once mutants ran on the merge-ready head; never left unreported."
//! - The fix round's task: a step before the report that stalls fails the job quickly and the report states it; "If
//!   the report step cannot run after a failed checkout because it needs the tree, restructure so it still concludes
//!   the job's check, and say how."
//!
//! - `qa_mcs_stall_report_needs_no_tree`: the step that reports a stall before the report is itself a step a failed
//!   checkout cannot stop. It names no action and no working directory, and its script, run by bash as GitHub runs a
//!   `run:` step (`bash --noprofile --norc -eo pipefail`), in an empty directory with no checkout and only the system's
//!   own tools on `PATH`, fails with the check run's error annotation and a summary, with and without a step summary
//!   file.
//! - `qa_mcs_every_job_before_the_report_is_bounded`: on the merge-ready run, `mutants-check` waits (`needs:`) for
//!   `mutants-verdict`, which reads the shards' reports, so a stall there leaves `mutants-check` unreported exactly as
//!   a stalled checkout of its own would. Each job `mutants-check` needs, but the shards, ends within `VERDICT_LIMIT`
//!   minutes once it starts: a `timeout-minutes` of the job's own no larger, or one on every step, adding up to no
//!   more. GitHub's default for a job with neither is 360 minutes.
//! - `qa_mcs_every_shard_step_is_bounded`: the shards (`mutants`), which `mutants-verdict` needs, are covered too (the
//!   orchestrator's round-4 reading of the owner's order on PR #389: "never left unreported" covers every job
//!   `mutants-check` waits on). A shard runs for up to REQ-VAL-149's 300 minutes by design, so no job-level bound
//!   tells a stall from a long run; a stall is bounded only by a limit on the step it happens in. Every step of the
//!   shard job has a `timeout-minutes` of its own, so a step that stalls (a download, an install, a build, an upload)
//!   fails the shard at its own limit, not at GitHub's 360 minutes for the job.
//!
//! Each has its negative controls (R-176), a workflow differing in the one respect the test turns on.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use validation::negative_control;

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;

/// The longest, in minutes, a job `mutants-check` waits for, the shards apart, may run once it starts: an hour, a
/// tenth of GitHub's 360-minute default, and far more than reading eight shard reports and building `xtask` takes.
const VERDICT_LIMIT: u64 = 60;

/// The step after `mutants-check`'s report that says a step before it did not finish.
const STALL_STEP: &str = "A step before the report did not finish";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn mutants_yml() -> String {
    std::fs::read_to_string(root().join(".github/workflows/mutants.yml"))
        .unwrap_or_else(|e| panic!("qa_mcs: mutants.yml: {e}"))
}

/// `mutants.yml` with `from` replaced once by `to`; `from` must be there, so a control never edits nothing.
#[cfg(feature = "controls")]
fn edited(from: &str, to: &str) -> String {
    let yml = mutants_yml();
    assert!(
        yml.contains(from),
        "qa_mcs: the control's edit target is gone: {from}"
    );
    yml.replacen(from, to, 1)
}

/// A workflow as JSON, parsed by PyYAML (YAML 1.1 reads the key `on` as `true`; it is put back).
fn parse(yml: &str) -> Value {
    let dir = Scratch::new("qa_mcs_yaml");
    std::fs::create_dir_all(&*dir).expect("scratch folder");
    let file = dir.join("workflow.yml");
    std::fs::write(&file, yml).expect("workflow written");
    let out = Command::new("python3")
        .args([
            "-I",
            "-c",
            "import json, sys, yaml\n\
             d = yaml.safe_load(open(sys.argv[1]))\n\
             if True in d: d['on'] = d.pop(True)\n\
             print(json.dumps(d))",
        ])
        .arg(&file)
        .output()
        .expect("qa_mcs: python3 runs");
    assert!(
        out.status.success(),
        "qa_mcs: mutants.yml does not parse: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("qa_mcs: PyYAML's JSON")
}

fn steps<'a>(wf: &'a Value, job: &str) -> &'a [Value] {
    wf["jobs"][job]["steps"]
        .as_array()
        .unwrap_or_else(|| panic!("qa_mcs: job `{job}` has no steps"))
}

// ---- qa_mcs_stall_report_needs_no_tree ---------------------------------------------------------------------------

struct Ran {
    ok: bool,
    log: String,
    summary: String,
}

/// The stall step's script run as GitHub runs a `run:` step, in an empty directory: no checkout, no `scripts/`, and
/// only `/usr/bin:/bin` on `PATH`, so nothing of the tree or the toolchain is there to lean on.
fn run_stall(tag: &str, script: &str, checkout: &str, classify: &str, with_summary: bool) -> Ran {
    let dir = Scratch::new(&format!("qa_mcs_stall_{tag}"));
    let work = dir.join("empty");
    std::fs::create_dir_all(&work).expect("empty work directory");
    let summary = dir.join("summary.md");
    let mut cmd = Command::new("bash");
    cmd.args(["--noprofile", "--norc", "-eo", "pipefail", "-c", script])
        .current_dir(&work)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HEAD_SHA", "0123456789abcdef0123456789abcdef01234567")
        .env("CHECKOUT", checkout)
        .env("CLASSIFY", classify);
    if with_summary {
        cmd.env("GITHUB_STEP_SUMMARY", &summary);
    }
    let out = cmd.output().expect("qa_mcs: bash runs");
    Ran {
        ok: out.status.success(),
        log: format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
        summary: std::fs::read_to_string(&summary).unwrap_or_default(),
    }
}

fn check_stall_report(yml: &str) {
    let wf = parse(yml);
    let all = steps(&wf, "mutants-check");
    let stall = all
        .iter()
        .find(|s| s["name"].as_str() == Some(STALL_STEP))
        .unwrap_or_else(|| panic!("qa_mcs: mutants-check has no step `{STALL_STEP}`"));
    for key in ["uses", "working-directory", "shell"] {
        assert!(
            stall.get(key).is_none(),
            "qa_mcs: the stall step sets `{key}` ({}), so a failed checkout or setup can stop it",
            stall[key]
        );
    }
    let script = stall["run"]
        .as_str()
        .expect("qa_mcs: the stall step has a `run:` script");
    for (i, (checkout, classify, with_summary)) in [
        ("failure", "skipped", true),
        ("cancelled", "skipped", true),
        ("success", "failure", true),
        ("failure", "skipped", false),
    ]
    .into_iter()
    .enumerate()
    {
        let got = run_stall(&i.to_string(), script, checkout, classify, with_summary);
        assert!(
            !got.ok,
            "qa_mcs: the stall step passed with the checkout {checkout} and the classification {classify}, so \
             mutants-check could pass on nothing: {}",
            got.log
        );
        assert!(
            got.log.contains("::error title=mutants-check::"),
            "qa_mcs: the stall step, with no tree and the checkout {checkout}, did not reach mutants-check's error \
             annotation: {}",
            got.log
        );
        if with_summary {
            assert!(
                got.summary.contains("mutants-check") && got.summary.contains(checkout),
                "qa_mcs: the stall step's summary does not name mutants-check and the checkout's outcome \
                 ({checkout}): `{}`",
                got.summary
            );
        }
    }
}

#[test]
fn qa_mcs_stall_report_needs_no_tree() {
    check_stall_report(&mutants_yml());
}

negative_control!(
    qa_mcs_stall_report_needs_no_tree,
    "the stall step's script calling a script of the tree first, so after a failed checkout it dies unannotated",
    expected = "did not reach mutants-check's error annotation",
    check_stall_report(&edited(
        "          set -uo pipefail\n          title=\"a step before the report did not finish\"\n",
        "          set -uo pipefail\n          scripts/ci_changes.sh detect || exit 1\n          \
         title=\"a step before the report did not finish\"\n"
    ))
);

mod stall_uses_tree_dir {
    use super::*;
    negative_control!(
        qa_mcs_stall_report_needs_no_tree,
        "the stall step run in a directory of the checkout, which a failed checkout never made",
        expected = "the stall step sets `working-directory`",
        check_stall_report(&edited(
            "        if: always() && steps.report.outcome == 'skipped'\n",
            "        if: always() && steps.report.outcome == 'skipped'\n        working-directory: scripts\n"
        ))
    );
}

mod stall_passes {
    use super::*;
    negative_control!(
        qa_mcs_stall_report_needs_no_tree,
        "the stall step ending in success, so a stalled checkout leaves mutants-check passed",
        expected = "the stall step passed",
        check_stall_report(&edited(
            "          echo \"::error title=mutants-check::$title. $why\"\n          exit 1\n",
            "          echo \"::error title=mutants-check::$title. $why\"\n          exit 0\n"
        ))
    );
}

// ---- qa_mcs_every_job_before_the_report_is_bounded ---------------------------------------------------------------

/// The jobs `mutants-check` waits for, directly or through another, by `needs:`.
fn awaited(wf: &Value, job: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut todo = vec![job.to_owned()];
    while let Some(j) = todo.pop() {
        let needs = &wf["jobs"][&j]["needs"];
        let names: Vec<String> = match needs {
            Value::String(s) => vec![s.clone()],
            Value::Array(a) => a
                .iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect(),
            _ => Vec::new(),
        };
        for n in names {
            if !out.contains(&n) {
                out.push(n.clone());
                todo.push(n);
            }
        }
    }
    out
}

/// The most a job may run once it starts, in minutes: its own `timeout-minutes`, or, with none, the sum of its steps'
/// limits when every step has one; `None` when neither bounds it (GitHub's 360-minute default then applies).
fn bound(job: &Value) -> Option<u64> {
    if let Some(m) = job["timeout-minutes"].as_u64() {
        return Some(m);
    }
    let all = job["steps"].as_array()?;
    all.iter()
        .map(|s| s["timeout-minutes"].as_u64())
        .sum::<Option<u64>>()
}

fn check_awaited_bounded(yml: &str) {
    let wf = parse(yml);
    let jobs = awaited(&wf, "mutants-check");
    assert!(
        jobs.iter().any(|j| j == "mutants-verdict"),
        "qa_mcs: mutants-check does not wait for mutants-verdict ({jobs:?}), so it cannot report the verdict"
    );
    // The shards apart: `qa_mcs_every_shard_step_is_bounded` covers them, since their run is long by design.
    for j in jobs.iter().filter(|j| j.as_str() != "mutants") {
        let job = &wf["jobs"][j];
        let unbounded: Vec<String> = job["steps"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|s| s["timeout-minutes"].as_u64().is_none())
            .map(|s| {
                s["name"]
                    .as_str()
                    .or(s["uses"].as_str())
                    .unwrap_or("?")
                    .to_owned()
            })
            .collect();
        match bound(job) {
            Some(m) => assert!(
                m <= VERDICT_LIMIT,
                "qa_mcs: {j}, which mutants-check waits for, may run {m} minutes, more than {VERDICT_LIMIT}, before \
                 mutants-check reports"
            ),
            None => panic!(
                "qa_mcs: {j}, which mutants-check waits for, has no limit of its own and steps with none \
                 ({unbounded:?}): a stall there leaves mutants-check unreported for GitHub's 360 minutes"
            ),
        }
    }
}

#[test]
fn qa_mcs_every_job_before_the_report_is_bounded() {
    check_awaited_bounded(&mutants_yml());
}

negative_control!(
    qa_mcs_every_job_before_the_report_is_bounded,
    "mutants-verdict limited to an hour and a minute, past the bound",
    expected = "may run 61 minutes",
    // The job's own limit edited in place: a second `timeout-minutes` key would lose to the job's later one, which
    // PyYAML, like GitHub, keeps.
    check_awaited_bounded(&edited(
        "    needs: mutants\n    runs-on: ubuntu-24.04\n    timeout-minutes: 30\n",
        "    needs: mutants\n    runs-on: ubuntu-24.04\n    timeout-minutes: 61\n"
    ))
);

mod verdict_not_awaited {
    use super::*;
    negative_control!(
        qa_mcs_every_job_before_the_report_is_bounded,
        "mutants-check waiting for nothing, so it reports before the verdict",
        expected = "does not wait for mutants-verdict",
        check_awaited_bounded(&edited(
            "    needs: mutants-verdict\n",
            "    needs: merge-ready-off\n"
        ))
    );
}

// ---- qa_mcs_every_shard_step_is_bounded --------------------------------------------------------------------------

/// The shard job's steps without a positive `timeout-minutes` of their own, by name (or action), in order.
fn unlimited_shard_steps(wf: &Value) -> Vec<String> {
    steps(wf, "mutants")
        .iter()
        .filter(|s| s["timeout-minutes"].as_u64().is_none_or(|m| m == 0))
        .map(|s| {
            s["name"]
                .as_str()
                .or(s["uses"].as_str())
                .unwrap_or("?")
                .to_owned()
        })
        .collect()
}

fn check_shard_steps_bounded(yml: &str) {
    let wf = parse(yml);
    assert!(
        awaited(&wf, "mutants-check").iter().any(|j| j == "mutants"),
        "qa_mcs: mutants-check does not wait, through mutants-verdict, for the shards (`mutants`)"
    );
    let unlimited = unlimited_shard_steps(&wf);
    assert!(
        unlimited.is_empty(),
        "qa_mcs: shard steps with no limit of their own, so a stall in one holds mutants-check unreported for \
         GitHub's 360 minutes: {unlimited:?}"
    );
}

/// A shard job shaped as `mutants.yml`'s, every step limited: the check passes on it, so its controls trip on the one
/// edit each makes.
const SHARDS: &str = "on:\n  pull_request:\njobs:\n  mutants:\n    runs-on: ubuntu-24.04\n    steps:\n      \
                      - uses: actions/checkout@v4\n        timeout-minutes: 10\n      \
                      - name: rustup toolchain install\n        timeout-minutes: 5\n        \
                      run: rustup toolchain install\n      \
                      - name: cargo mutants\n        timeout-minutes: 300\n        run: cargo mutants\n      \
                      - uses: actions/upload-artifact@v4\n        if: always()\n        timeout-minutes: 5\n  \
                      mutants-verdict:\n    needs: mutants\n    timeout-minutes: 30\n    steps:\n      - run: true\n  \
                      mutants-check:\n    needs: mutants-verdict\n    steps:\n      - run: true\n";

#[test]
fn qa_mcs_every_shard_step_is_bounded() {
    check_shard_steps_bounded(SHARDS);
    check_shard_steps_bounded(&mutants_yml());
}

negative_control!(
    qa_mcs_every_shard_step_is_bounded,
    "a shard whose toolchain install has no limit, the checkout's and cargo mutants' kept",
    expected =
        "shard steps with no limit of their own, so a stall in one holds mutants-check unreported",
    check_shard_steps_bounded(&SHARDS.replacen(
        "        timeout-minutes: 5\n        run: rustup toolchain install\n",
        "        run: rustup toolchain install\n",
        1
    ))
);

mod shard_upload_unlimited {
    use super::*;
    negative_control!(
        qa_mcs_every_shard_step_is_bounded,
        "a shard whose report upload, after cargo mutants, has no limit",
        expected = "shard steps with no limit of their own, so a stall in one holds mutants-check unreported",
        check_shard_steps_bounded(&SHARDS.replacen(
            "        if: always()\n        timeout-minutes: 5\n",
            "        if: always()\n",
            1
        ))
    );
}

mod shards_not_awaited {
    use super::*;
    negative_control!(
        qa_mcs_every_shard_step_is_bounded,
        "mutants-verdict waiting for nothing, so the shards' bound is not what mutants-check waits on",
        expected = "does not wait, through mutants-verdict, for the shards",
        check_shard_steps_bounded(&SHARDS.replacen("    needs: mutants\n", "", 1))
    );
}
