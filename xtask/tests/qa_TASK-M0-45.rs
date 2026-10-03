//! qa's tests for TASK-M0-45, written from REQ-SYS-077 ("the `ci` CI job's nextest runs must be sharded across
//! parallel jobs, the shards together running every test the unsharded run did, and the `xtask-ci` job [...] must run
//! as 4 parallel jobs of `cargo xtask ci --partition <k>/4`, the controls assigned to a shard by a stable hash of the
//! control name, every control run exactly once across the shards"), R-360, R-365 and R-366 ("nextest uses
//! `--partition hash:<k>/4`, not `slice:` (stable assignment)"), and REQ-VAL-166 ("CI runs every control through
//! `cargo xtask ci` in a job of its own [...] failing on any finding"), not from the implementation.
//!
//! - The workflow: each sharded step's job has a matrix of exactly the shards 1 to n its `--partition` names; the
//!   `ci` job's nextest step uses nextest's `hash:`; the doctests run in one shard of the matrix; the checks branch
//!   protection requires, `ci` and `xtask-ci`, gather every shard, cannot be skipped by a failed shard, and fail
//!   unless every shard succeeded.
//! - `cargo xtask controls --partition k/4`, run on a stand-in `cargo` (`CARGO`) that answers `metadata` and the
//!   listings from canned text and logs each control it is asked to run: the 4 shards together run each control
//!   exactly as often as the unpartitioned run does, each in one shard; a control's shard depends on its name alone,
//!   not on its crate, the listing's order or the other names; a control that leaves its test passing fails exactly
//!   one shard.
//! - nextest's `hash:` shards, on a fixture crate of its own: together they list every test once, and a test keeps
//!   its shard when another test is added.
//!
//! Each test registers a negative control (R-176, R-199).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use validation::negative_control;
use validation::spawn::Spawn;

// ---- The workflow ---------------------------------------------------------------------------------------------------

/// CI's per-push workflow, as checked in.
fn the_workflow() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../.github/workflows/ci.yml");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// A job of the workflow: its id (the key two spaces in under `jobs:`) and its lines, comments dropped.
struct Job {
    id: String,
    lines: Vec<String>,
}

impl Job {
    /// The value of the job's own key `key` (four spaces in), if it has one.
    fn own(&self, key: &str) -> Option<String> {
        self.lines.iter().find_map(|l| {
            let indent = l.len() - l.trim_start().len();
            (indent == 4)
                .then(|| l.trim().strip_prefix(&format!("{key}:")))
                .flatten()
                .map(|v| v.trim().to_owned())
        })
    }

    /// The job's steps, each as its lines (a step starts at a `- ` six spaces in).
    fn steps(&self) -> Vec<Vec<String>> {
        let mut steps: Vec<Vec<String>> = Vec::new();
        for l in &self.lines {
            if l.starts_with("      - ") {
                steps.push(vec![l.clone()]);
            } else if l.starts_with("       ") {
                if let Some(step) = steps.last_mut() {
                    step.push(l.clone());
                }
            }
        }
        steps
    }

    /// The `run:` commands of the job's steps, trimmed (one-line scripts).
    fn runs(&self) -> Vec<String> {
        self.steps()
            .iter()
            .filter_map(|s| step_key(s, "run"))
            .collect()
    }

    /// The values of the job's matrix `shard:` list, in order; `None` if it has none.
    fn shards(&self) -> Option<Vec<String>> {
        self.lines.iter().find_map(|l| {
            let list = l.trim().strip_prefix("shard:")?.trim();
            let list = list.strip_prefix('[')?.strip_suffix(']')?;
            Some(list.split(',').map(|v| v.trim().to_owned()).collect())
        })
    }
}

/// The value of `key` among a step's lines, trimmed.
fn step_key(step: &[String], key: &str) -> Option<String> {
    step.iter().find_map(|l| {
        l.trim()
            .trim_start_matches("- ")
            .strip_prefix(&format!("{key}:"))
            .map(|v| v.trim().to_owned())
    })
}

/// The jobs of `workflow`.
fn jobs(workflow: &str) -> Vec<Job> {
    let mut found: Vec<Job> = Vec::new();
    let mut in_jobs = false;
    for line in workflow.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if !line.starts_with(' ') {
            in_jobs = t == "jobs:";
            continue;
        }
        if !in_jobs {
            continue;
        }
        if line.starts_with("  ") && !line.starts_with("   ") && t.ends_with(':') {
            found.push(Job {
                id: t.trim_end_matches(':').to_owned(),
                lines: Vec::new(),
            });
        } else if let Some(job) = found.last_mut() {
            job.lines.push(line.to_owned());
        }
    }
    found
}

/// The matrix expression a sharded step names its shard by.
const SHARD: &str = "${{ matrix.shard }}";

/// The `ci` job's nextest step and the `xtask-ci` job's `cargo xtask ci` step, as R-366 and R-360 give them; since
/// R-372 the nextest step runs the workspace's tests from the archive `cargo nextest archive --workspace` builds.
const NEXTEST_STEP: &str = "cargo nextest run --archive-file $RUNNER_TEMP/nextest-ci.tar.zst --extract-to . --extract-overwrite --partition hash:${{ matrix.shard }}/4";
/// The nextest step's unsharded form from the same archive: every test of the workspace, in one job.
const NEXTEST_UNSHARDED: &str =
    "cargo nextest run --archive-file $RUNNER_TEMP/nextest-ci.tar.zst --extract-to . --extract-overwrite";
const XTASK_CI_STEP: &str = "cargo xtask ci --partition ${{ matrix.shard }}/4";

/// REQ-SYS-077 on `workflow`: one job runs the workspace's nextest step, with nextest's `hash:` partition, and one job
/// runs `cargo xtask ci --partition`; no job runs either unsharded beside them (a second run of every test or every
/// control); and each step's job has a matrix of exactly the shards 1 to n its `--partition <k>/n` names, each once,
/// so the shards together run every slice, and n is 4.
fn check_sharded_steps(workflow: &str) {
    let jobs = jobs(workflow);
    for (step, what) in [
        (NEXTEST_STEP, "nextest"),
        (XTASK_CI_STEP, "`cargo xtask ci`"),
    ] {
        let with: Vec<&Job> = jobs
            .iter()
            .filter(|j| j.runs().iter().any(|r| r == step))
            .collect();
        assert_eq!(
            with.len(),
            1,
            "no single job runs the sharded {what} step `{step}`"
        );
        let job = with[0];
        let n: usize = step.rsplit('/').next().unwrap().parse().unwrap();
        let shards = job
            .shards()
            .unwrap_or_else(|| panic!("job `{}` names {SHARD} but has no `shard:` matrix", job.id));
        let want: Vec<String> = (1..=n).map(|k| k.to_string()).collect();
        assert_eq!(
            shards, want,
            "job `{}`'s matrix shards are not 1 to {n}, each once, so some slice of `{step}` runs never or twice",
            job.id
        );
    }
    for job in &jobs {
        for run in job.runs() {
            assert!(
                run != "cargo nextest run --workspace"
                    && run != NEXTEST_UNSHARDED
                    && run != "cargo xtask ci",
                "job `{}` runs `{run}` unsharded, beside its shards",
                job.id
            );
        }
    }
}

/// R-366 on `workflow`: every workspace nextest step that is sharded is sharded by nextest's `hash:`, the stable
/// assignment, not by `count:` or `slice:`.
fn check_hash_partition(workflow: &str) {
    let steps: Vec<String> = jobs(workflow)
        .iter()
        .flat_map(|j| j.runs())
        .filter(|r| {
            (r.starts_with("cargo nextest run --workspace")
                || r.starts_with("cargo nextest run --archive-file"))
                && r.contains("--partition")
        })
        .collect();
    assert!(!steps.is_empty(), "no sharded workspace nextest step");
    for step in steps {
        assert!(
            step.contains(&format!("--partition hash:{SHARD}/")),
            "the workspace nextest step is not sharded by nextest's `hash:` (R-366): `{step}`"
        );
    }
}

#[test]
fn qa_m0_45_ci_tests_run_in_4_shards() {
    check_sharded_steps(&the_workflow());
}

negative_control!(
    qa_m0_45_ci_tests_run_in_4_shards,
    "the checked-in workflow with the `ci` job's matrix cut to shards 1 to 3, so slice 4 of the tests never runs",
    expected = "matrix shards are not 1 to 4",
    check_sharded_steps(&the_workflow().replacen(
        "shard: [1, 2, 3, 4]",
        "shard: [1, 2, 3]",
        1
    ))
);

#[test]
fn qa_m0_45_xtask_ci_runs_in_4_shards() {
    check_sharded_steps(&the_workflow());
}

negative_control!(
    qa_m0_45_xtask_ci_runs_in_4_shards,
    "the checked-in workflow with `xtask-ci`'s matrix running shard 3 twice and shard 4 never",
    expected = "job `xtask-ci`'s matrix shards are not 1 to 4",
    {
        let workflow = the_workflow();
        let at = workflow.find("\n  xtask-ci:\n").expect("no xtask-ci job");
        let (head, tail) = workflow.split_at(at);
        check_sharded_steps(&format!(
            "{head}{}",
            tail.replacen("shard: [1, 2, 3, 4]", "shard: [1, 2, 3, 3]", 1)
        ))
    }
);

#[test]
fn qa_m0_45_ci_tests_shard_by_nextest_hash() {
    check_hash_partition(&the_workflow());
}

negative_control!(
    qa_m0_45_ci_tests_shard_by_nextest_hash,
    "the checked-in workflow sharding the tests by nextest's `count:`, which R-366 rules out",
    expected = "not sharded by nextest's `hash:`",
    check_hash_partition(&the_workflow().replace(
        "--partition hash:${{ matrix.shard }}/4",
        "--partition count:${{ matrix.shard }}/4"
    ))
);

/// R-231 and R-366: the workspace's doctest step runs in exactly one shard of its job's matrix, not in none and not in
/// each.
fn check_doctests_in_one_shard(workflow: &str) {
    let jobs = jobs(workflow);
    let (job, step) = jobs
        .iter()
        .find_map(|j| {
            j.steps()
                .into_iter()
                .find(|s| step_key(s, "run").as_deref() == Some("cargo test --workspace --doc"))
                .map(|s| (j, s))
        })
        .expect("no job runs `cargo test --workspace --doc`");
    let Some(shards) = job.shards() else {
        return; // An unsharded job runs it once.
    };
    let cond = step_key(&step, "if").unwrap_or_default();
    let shard = cond.strip_prefix("matrix.shard ==").map(str::trim);
    assert!(
        shard.is_some_and(|s| shards.iter().any(|v| v == s)),
        "job `{}` runs the doctests in {} of its shards {shards:?} (`if: {cond}`), not in exactly one",
        job.id,
        if shard.is_some() { "none" } else { "each" }
    );
}

#[test]
fn qa_m0_45_doctests_run_in_one_shard() {
    check_doctests_in_one_shard(&the_workflow());
}

negative_control!(
    qa_m0_45_doctests_run_in_one_shard,
    "the checked-in workflow with the doctest step conditioned on shard 5, which the matrix does not have",
    expected = "runs the doctests in none of its shards",
    check_doctests_in_one_shard(
        &the_workflow().replacen("if: matrix.shard == 1", "if: matrix.shard == 5", 1)
    )
);

/// The job whose `name:` is `check` (the status check branch protection requires), and its run script.
fn gate<'a>(jobs: &'a [Job], check: &str) -> (&'a Job, String) {
    let found: Vec<&Job> = jobs
        .iter()
        .filter(|j| match j.own("name") {
            Some(name) => name == check,
            // A matrix job reports one check per shard, `<id> (<k>)`, never `<id>`.
            None => j.id == check && j.shards().is_none(),
        })
        .collect();
    assert_eq!(found.len(), 1, "no single job reports the check `{check}`");
    let job = found[0];
    let runs = job.runs();
    assert_eq!(runs.len(), 1, "job `{}` has not one run step", job.id);
    (job, runs[0].clone())
}

/// Runs `script` under bash, as Actions runs a step, with `RESULTS` set to `results`; whether it passed.
fn passes(script: &str, results: &str) -> bool {
    Command::new("bash")
        .args(["-e", "-c", script])
        .env("RESULTS", results)
        .timed_output()
        .expect("run bash")
        .status
        .success()
}

/// REQ-VAL-166 and R-266 on `workflow`: the required check `check` is reported by a job that gathers every job in
/// `gathers` (`needs:`), runs whatever they ended in (`if: always()`, so a failed shard turns it red rather than
/// skipping it, which branch protection counts as passing), reads their results, and passes only when every one
/// succeeded.
fn check_gate(workflow: &str, check: &str, gathers: &[&str]) {
    let jobs = jobs(workflow);
    let (job, script) = gate(&jobs, check);
    let needs = job.own("needs").unwrap_or_default();
    let needs: BTreeSet<&str> = needs
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    for g in gathers {
        assert!(
            needs.contains(g),
            "the check `{check}` (job `{}`) does not gather job `{g}`: needs {needs:?}",
            job.id
        );
    }
    assert_eq!(
        job.own("if").as_deref(),
        Some("always()"),
        "the check `{check}` (job `{}`) can be skipped when a job it gathers fails",
        job.id
    );
    assert!(
        job.lines.iter().any(|l| l.contains("needs.*.result")),
        "the check `{check}` does not read the results of the jobs it gathers"
    );
    assert!(
        passes(&script, "success success success success success"),
        "the check `{check}` fails when every job it gathers succeeded: `{script}`"
    );
    for bad in ["failure", "cancelled", "skipped"] {
        assert!(
            !passes(&script, &format!("success success {bad} success success")),
            "the check `{check}` passes with a job it gathers {bad}: `{script}`"
        );
    }
}

#[test]
fn qa_m0_45_the_xtask_ci_check_fails_unless_every_shard_passed() {
    check_gate(&the_workflow(), "xtask-ci", &["xtask-ci"]);
}

negative_control!(
    qa_m0_45_the_xtask_ci_check_fails_unless_every_shard_passed,
    "the checked-in workflow with the xtask-ci check failing only on `failure`, so a cancelled shard passes it",
    expected = "passes with a job it gathers cancelled",
    check_gate(
        &the_workflow().replace(r#"[ "$r" = success ]"#, r#"[ "$r" != failure ]"#),
        "xtask-ci",
        &["xtask-ci"]
    )
);

#[test]
fn qa_m0_45_the_ci_check_gathers_every_test_shard_and_the_checks() {
    check_gate(&the_workflow(), "ci", &["ci", "ci-checks", "ci-workspace"]);
}

negative_control!(
    qa_m0_45_the_ci_check_gathers_every_test_shard_and_the_checks,
    "the checked-in workflow with the ci check gathering the test shards alone, not `ci-checks`",
    expected = "does not gather job `ci-checks`",
    check_gate(
        &the_workflow().replace(
            "needs: [ci, ci-checks, ci-workspace]",
            "needs: [ci, ci-workspace]"
        ),
        "ci",
        &["ci", "ci-checks", "ci-workspace"]
    )
);

/// R-372: the xtask tests that left the shards for `ci-workspace` still count toward the required check `ci`.
#[test]
fn qa_m0_45_the_ci_check_gathers_the_workspace_tests() {
    check_gate(&the_workflow(), "ci", &["ci", "ci-checks", "ci-workspace"]);
}

negative_control!(
    qa_m0_45_the_ci_check_gathers_the_workspace_tests,
    "the checked-in workflow with the ci check gathering the shards and the checks, not `ci-workspace`",
    expected = "does not gather job `ci-workspace`",
    check_gate(
        &the_workflow().replace("needs: [ci, ci-checks, ci-workspace]", "needs: [ci, ci-checks]"),
        "ci",
        &["ci", "ci-checks", "ci-workspace"]
    )
);

#[test]
fn qa_m0_45_the_required_checks_cannot_be_skipped() {
    let workflow = the_workflow();
    check_gate(&workflow, "ci", &["ci", "ci-checks", "ci-workspace"]);
    check_gate(&workflow, "xtask-ci", &["xtask-ci"]);
}

negative_control!(
    qa_m0_45_the_required_checks_cannot_be_skipped,
    "the checked-in workflow with `if: always()` dropped from the xtask-ci check, which a failed shard then skips",
    expected = "can be skipped when a job it gathers fails",
    {
        let workflow = the_workflow();
        let at = workflow.find("\n  xtask-ci-gate:\n").expect("no xtask-ci-gate job");
        let (head, tail) = workflow.split_at(at);
        check_gate(
            &format!("{head}{}", tail.replacen("    if: always()\n", "", 1)),
            "xtask-ci",
            &["xtask-ci"],
        )
    }
);

// ---- `cargo xtask controls --partition` on a stand-in cargo --------------------------------------------------------

/// A canned workspace: each crate's `cargo test --tests -- --list` lines.
type Listings = Vec<(&'static str, String)>;

/// Two crates, each with 24 tests and their controls; alpha lists `alpha_0::t00` and its control twice (two test
/// targets with the same path), and `t03` in two modules; `shared::s` and its control are in both crates.
fn workspace() -> Listings {
    let crate_listing = |krate: &str, n: usize| -> String {
        let mut out = String::new();
        for i in 0..n {
            let module = format!("{krate}_{}", i % 3);
            out.push_str(&format!(
                "{module}::t{i:02}: test\n{module}::t{i:02}::negative_control: test\n"
            ));
        }
        out
    };
    let mut alpha = crate_listing("alpha", 24);
    alpha.push_str("second::t03::negative_control: test\n");
    alpha.push_str("second::t03: test\n");
    alpha.push_str("alpha_0::t00: test\nalpha_0::t00::negative_control: test\n");
    let mut beta = crate_listing("beta", 24);
    beta.push_str("shared::s: test\nshared::s::negative_control: test\n");
    alpha.push_str("shared::s: test\nshared::s::negative_control: test\n");
    vec![("alpha", alpha), ("beta", beta)]
}

/// Writes the stand-in `cargo` for `listings` into a fresh directory named `case`, with `leaky` the controls it
/// reports as leaving their test passing. Returns the directory.
fn stand_in(case: &str, listings: &Listings, leaky: &[&str]) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("qa_m0_45_{case}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let packages: Vec<String> = listings
        .iter()
        .map(|(name, _)| {
            format!(r#"{{"name":"{name}","features":{{"controls":[]}},"targets":[{{"doctest":false}}]}}"#)
        })
        .collect();
    fs::write(
        dir.join("metadata.json"),
        format!(r#"{{"packages":[{}]}}"#, packages.join(",")),
    )
    .unwrap();
    for (name, listed) in listings {
        fs::write(dir.join(format!("listed_{name}.txt")), listed).unwrap();
    }
    fs::write(dir.join("leaky.txt"), leaky.join("\n") + "\n").unwrap();
    // A run of `cargo test -p <crate> ... -- <harness>`: `--list` prints the crate's listing; otherwise each listed
    // control the harness selects (by `--exact` names, or else by substring filters) is logged as `<crate> <name>`
    // and reported passing (it made its test fail), or failing if it is in leaky.txt.
    let script = r#"#!/bin/sh
D='@DIR@'
if [ "$1" = metadata ]; then cat "$D/metadata.json"; exit 0; fi
pkg=; prev=; after=; exact=; names=
for a in "$@"; do
  if [ -n "$after" ]; then
    case "$a" in
      --list) cat "$D/listed_$pkg.txt"; exit 0;;
      --exact) exact=1;;
      *) names="$names $a";;
    esac
  else
    [ "$prev" = -p ] && pkg="$a"
    [ "$a" = -- ] && after=1
  fi
  prev="$a"
done
status=0
while read -r line; do
  name="${line%: test}"
  case "$name" in *::negative_control) ;; *) continue;; esac
  run=
  for n in $names; do
    if [ -n "$exact" ]; then [ "$n" = "$name" ] && run=1
    else case "$name" in *"$n"*) run=1;; esac
    fi
  done
  [ -n "$run" ] || continue
  echo "$pkg $name" >> "$D/ran.log"
  if grep -qxF "$name" "$D/leaky.txt"; then
    echo "test $name - should panic ... FAILED"; status=101
  else
    echo "test $name - should panic ... ok"
  fi
done < "$D/listed_$pkg.txt"
exit $status
"#
    .replace("@DIR@", &dir.display().to_string());
    validation::spawn::write_executable(&dir.join("cargo"), script).unwrap();
    dir
}

/// One run of `xtask controls [--partition <slice>]` on the stand-in in `dir`: whether it passed, its output, and the
/// controls it ran, as `<crate> <name>`, once per run.
fn controls_run(dir: &Path, slice: Option<&str>) -> (bool, String, Vec<String>) {
    let _ = fs::remove_file(dir.join("ran.log"));
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_xtask"));
    cmd.arg("controls");
    if let Some(slice) = slice {
        cmd.args(["--partition", slice]);
    }
    let output = cmd
        .env("CARGO", dir.join("cargo"))
        .timed_output()
        .expect("run xtask controls");
    let ran = fs::read_to_string(dir.join("ran.log"))
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect();
    let out = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (output.status.success(), out, ran)
}

/// Counts of each item of `ran`.
fn counted(ran: &[String]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for r in ran {
        *counts.entry(r.clone()).or_insert(0) += 1;
    }
    counts
}

/// REQ-SYS-077 and R-360: the runs of `slices` together run each control exactly as often as the unpartitioned
/// `cargo xtask controls` does, and every control's runs fall in one shard.
fn check_every_control_once(case: &str, slices: &[&str]) {
    let dir = stand_in(case, &workspace(), &[]);
    let (ok, out, whole) = controls_run(&dir, None);
    assert!(ok, "the unpartitioned run failed:\n{out}");
    assert!(
        whole.len() > 50,
        "the unpartitioned run ran {} controls",
        whole.len()
    );
    let mut together = Vec::new();
    let mut shard_of: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for slice in slices {
        let (ok, out, ran) = controls_run(&dir, Some(slice));
        assert!(ok, "shard {slice} failed:\n{out}");
        for r in &ran {
            shard_of
                .entry(r.clone())
                .or_default()
                .insert((*slice).to_owned());
        }
        together.extend(ran);
    }
    assert_eq!(
        counted(&together),
        counted(&whole),
        "the shards together do not run each control exactly as the unpartitioned run does"
    );
    for (control, shards) in shard_of {
        assert_eq!(
            shards.len(),
            1,
            "control `{control}` ran in shards {shards:?}"
        );
    }
}

#[test]
fn qa_m0_45_every_control_runs_once_across_the_4_shards() {
    check_every_control_once("once", &["1/4", "2/4", "3/4", "4/4"]);
}

negative_control!(
    qa_m0_45_every_control_runs_once_across_the_4_shards,
    "shards 1 to 3 of 4 alone, so slice 4's controls never run",
    expected = "the shards together do not run each control exactly as the unpartitioned run does",
    check_every_control_once("once_ctl", &["1/4", "2/4", "3/4"])
);

/// Each control's shard of 4, as `<name>` → shard, from running every shard on `listings`; a name run in several
/// crates or targets must be in one shard.
fn assignment(case: &str, listings: &Listings) -> BTreeMap<String, u32> {
    let dir = stand_in(case, listings, &[]);
    let mut found: BTreeMap<String, u32> = BTreeMap::new();
    for k in 1..=4u32 {
        let (ok, out, ran) = controls_run(&dir, Some(&format!("{k}/4")));
        assert!(ok, "shard {k}/4 failed:\n{out}");
        for r in ran {
            let name = r.split_once(' ').unwrap().1.to_owned();
            if let Some(was) = found.insert(name.clone(), k) {
                assert_eq!(was, k, "control `{name}` ran in shards {was} and {k}");
            }
        }
    }
    found
}

/// The workspace re-listed: each crate's lines reversed, alpha's controls moved to beta and beta's to alpha, and
/// twenty new tests with their controls added at the head of each.
fn relisted() -> Listings {
    let ws = workspace();
    let rev = |s: &str| -> String {
        let mut lines: Vec<&str> = s.lines().collect();
        lines.reverse();
        lines.iter().map(|l| format!("{l}\n")).collect()
    };
    let extra = |krate: &str| -> String {
        (0..20)
            .map(|i| {
                format!(
                    "{krate}_new::n{i:02}: test\n{krate}_new::n{i:02}::negative_control: test\n"
                )
            })
            .collect()
    };
    vec![
        ("alpha", extra("alpha") + &rev(&ws[1].1)),
        ("beta", extra("beta") + &rev(&ws[0].1)),
    ]
}

/// R-360: a control's shard is a stable function of its name alone: `second` (from a re-listed workspace, its
/// controls in the other crate, in reverse order, among new ones) gives every control of `first` the same shard; and
/// the controls spread over every shard, so the comparison has something to compare.
fn check_shard_by_name_alone(first: &BTreeMap<String, u32>, second: &BTreeMap<String, u32>) {
    assert_eq!(
        first.values().collect::<BTreeSet<_>>().len(),
        4,
        "the controls do not spread over the 4 shards: {first:?}"
    );
    for (name, k) in first {
        let again = second
            .get(name)
            .unwrap_or_else(|| panic!("control `{name}` did not run in the re-listed workspace"));
        assert_eq!(
            again, k,
            "control `{name}` moved shard, from {k} to {again}, when the listing changed around it"
        );
    }
}

#[test]
fn qa_m0_45_a_controls_shard_depends_on_its_name_alone() {
    check_shard_by_name_alone(
        &assignment("name_first", &workspace()),
        &assignment("name_second", &relisted()),
    );
}

negative_control!(
    qa_m0_45_a_controls_shard_depends_on_its_name_alone,
    "a second assignment dealt round-robin in listing order, as a slice by position would be",
    expected = "moved shard",
    {
        let mut dealt = BTreeMap::new();
        for (_, listed) in relisted() {
            for name in listed.lines().filter_map(|l| l.strip_suffix(": test")) {
                if name.ends_with("::negative_control") && !dealt.contains_key(name) {
                    let k = dealt.len() as u32 % 4 + 1;
                    dealt.insert(name.to_owned(), k);
                }
            }
        }
        check_shard_by_name_alone(&assignment("name_ctl", &workspace()), &dealt)
    }
);

/// REQ-VAL-147 under R-360: a control that leaves its test passing, `leaky`, fails exactly one of the 4 shards, which
/// names its test, as it fails the unpartitioned run.
fn check_leak_fails_one_shard(case: &str, leaky: &[&str]) {
    let dir = stand_in(case, &workspace(), leaky);
    let (ok, out, _) = controls_run(&dir, None);
    assert!(
        !ok,
        "the unpartitioned run passed with a leaky control:\n{out}"
    );
    let failed: Vec<(u32, String)> = (1..=4u32)
        .filter_map(|k| {
            let (ok, out, _) = controls_run(&dir, Some(&format!("{k}/4")));
            (!ok).then_some((k, out))
        })
        .collect();
    assert_eq!(
        failed.len(),
        1,
        "{} of the 4 shards failed on one leaky control, not exactly one: {:?}",
        failed.len(),
        failed.iter().map(|(k, _)| k).collect::<Vec<_>>()
    );
    assert!(
        failed[0].1.contains("t07"),
        "shard {} failed without naming the test its control leaves passing:\n{}",
        failed[0].0,
        failed[0].1
    );
}

#[test]
fn qa_m0_45_a_leaky_control_fails_exactly_one_shard() {
    check_leak_fails_one_shard("leak", &["beta_1::t07::negative_control"]);
}

negative_control!(
    qa_m0_45_a_leaky_control_fails_exactly_one_shard,
    "every control of t00 to t07, in both crates, leaky: they fall in more than one shard, which the check must see",
    expected = "not exactly one",
    {
        let leaky: Vec<String> = workspace()
            .iter()
            .flat_map(|(_, listed)| listed.lines().map(str::to_owned).collect::<Vec<_>>())
            .filter_map(|l| l.strip_suffix(": test").map(str::to_owned))
            .filter(|n| n.ends_with("::negative_control") && (0..8).any(|i| n.contains(&format!("::t{i:02}::"))))
            .collect();
        check_leak_fails_one_shard("leak_ctl", &leaky.iter().map(String::as_str).collect::<Vec<_>>())
    }
);

// ---- nextest's `hash:` shards ----------------------------------------------------------------------------------------

/// Writes a fixture crate of its own with `tests` unit tests into `dir`, written only when its content changes.
fn nextest_fixture(dir: &Path, tests: &[String]) {
    fs::create_dir_all(dir.join("src")).unwrap();
    let write = |path: PathBuf, text: String| {
        if fs::read_to_string(&path).ok().as_deref() != Some(text.as_str()) {
            fs::write(path, text).unwrap();
        }
    };
    write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"qa_m0_45_shards\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[workspace]\n"
            .to_owned(),
    );
    write(
        dir.join("src/lib.rs"),
        tests
            .iter()
            .map(|t| format!("#[test]\nfn {t}() {{}}\n"))
            .collect(),
    );
}

/// The tests `cargo nextest list [--partition <spec>]` lists for the fixture in `dir`.
fn nextest_list(dir: &Path, partition: Option<&str>) -> Vec<String> {
    let mut cmd = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned()));
    cmd.current_dir(dir)
        .args(["nextest", "list", "--message-format", "oneline"])
        .env("CARGO_TARGET_DIR", dir.join("target"))
        .env_remove("NEXTEST_PROFILE");
    if let Some(spec) = partition {
        cmd.args(["--partition", spec]);
    }
    let o = cmd.timed_output().expect("run cargo nextest list");
    assert!(
        o.status.success(),
        "cargo nextest list failed:\n{}",
        String::from_utf8_lossy(&o.stderr)
    );
    String::from_utf8_lossy(&o.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(1))
        .map(str::to_owned)
        .collect()
}

/// R-366 on nextest's partition `scheme`: the 4 shards together list every test of the fixture once; and with a test
/// added, every test keeps its shard (a stable assignment), so a test cannot move into a shard that has already run.
fn check_nextest_shards(case: &str, scheme: &str) {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("qa_m0_45_nextest_{case}"));
    let tests: Vec<String> = (0..32).map(|i| format!("t{i:02}")).collect();
    let shards = |tests: &[String]| -> BTreeMap<String, u32> {
        nextest_fixture(&dir, tests);
        let all: BTreeSet<String> = nextest_list(&dir, None).into_iter().collect();
        assert_eq!(all.len(), tests.len(), "the fixture lists {all:?}");
        let mut found = BTreeMap::new();
        for k in 1..=4u32 {
            for t in nextest_list(&dir, Some(&format!("{scheme}:{k}/4"))) {
                assert!(
                    found.insert(t.clone(), k).is_none(),
                    "test `{t}` is in two shards"
                );
            }
        }
        assert_eq!(
            found.keys().cloned().collect::<BTreeSet<_>>(),
            all,
            "the 4 shards together do not list every test"
        );
        found
    };
    let before = shards(&tests);
    let mut more = vec!["a_added".to_owned()];
    more.extend(tests.iter().cloned());
    let after = shards(&more);
    for (t, k) in &before {
        assert_eq!(
            after[t], *k,
            "test `{t}` moved shard, from {k} to {}, when a test was added",
            after[t]
        );
    }
}

#[test]
fn qa_m0_45_nextest_hash_shards_list_every_test_once_and_stay_put() {
    check_nextest_shards("hash", "hash");
}

negative_control!(
    qa_m0_45_nextest_hash_shards_list_every_test_once_and_stay_put,
    "nextest's `count:` partition, which deals the tests round-robin by position",
    expected = "moved shard",
    check_nextest_shards("count", "count")
);
