//! qa's tests for TASK-M0-54, written from REQ-TOOL-223 and R-809, not from `ci.yml`'s gate or `ci_changes.rs`'s model.
//!
//! REQ-TOOL-223: "The required check `ci` must gather `ci-release`: a `ci-release` shard that fails, or that skips on
//! a change that isn't docs or data, must fail the check, and a skip on a docs or data change must pass it (R-809)."
//! R-809: "A skip counts as a pass only on a docs or data change, where `ci-release` and its archive don't run. A
//! failed archive job skips `ci-release`, and that skip must fail the gate."
//!
//! - `qa_m0_54_the_gate_gathers_ci_release_and_reads_every_result`: `ci-gate`'s `needs` is exactly its five jobs, and
//!   its step reads every one of their results, so `ci-release` in `needs` is not a name no script looks at.
//! - `qa_m0_54_each_release_ending_on_each_head`: `ci.yml`'s jobs walked as GitHub runs them, on verdicts from the real
//!   classifier (code, a code and docs mix, data, docs), the gate's own script run under bash: every way `ci-release`
//!   can end that isn't a pass or a docs or data skip fails `ci`, a shard at a time (GitHub's `needs` joins them), a
//!   cancelled shard, a failed or cancelled archive and a failed classification among them.
//! - `qa_m0_54_the_quick_tier_is_untouched`: on a quick run `ci-gate` still skips (TASK-M0-54's Notes).
//! - `qa_m0_54_no_release_failure_is_swallowed`: neither job, and no step of theirs that runs or builds the tests,
//!   turns a failure into a pass by `continue-on-error` or `|| true`, so the result the gate reads can hold the failure.
//! - `qa_m0_54_a_runner_fault_upstream_of_ci_is_retried`, `qa_m0_54_a_real_release_failure_is_not_retried`: the seam
//!   R-809 opens with `retry.yml`. `ci` now goes red with `ci-release`, so `ci_retry.py`, on the real `ci.yml`, must
//!   read that red as the gate of a runner fault, in a shard or in the archive, and must not re-run a real failure.
//!
//! The gate's script is a workflow `run:` step: it runs here under a scratch HOME and CARGO_HOME. Each test has its
//! negative control (R-176).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};
use validation::negative_control;
use validation::spawn::Spawn;

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;

/// What the gate reads when it reads every job it gathers.
const EVERY_RESULT: &str = "${{ join(needs.*.result, ' ') }}";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the workspace root")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("qa_m0_54: {rel}: {e}"))
}

fn workflow() -> String {
    read(".github/workflows/ci.yml")
}

/// The checked-in workflow with `from` replaced once by `to`; `from` must be there, so a control cannot pass vacuously.
#[cfg(feature = "controls")]
fn edited(from: &str, to: &str) -> String {
    let yml = workflow();
    assert!(
        yml.contains(from),
        "the control's edit target is gone: {from}"
    );
    yml.replacen(from, to, 1)
}

/// `yml`'s jobs, through PyYAML.
fn jobs(yml: &str) -> Value {
    let dir = Scratch::new("qa_m0_54_yaml");
    std::fs::create_dir_all(&*dir).expect("scratch folder");
    let file = dir.join("ci.yml");
    std::fs::write(&file, yml).expect("workflow written");
    let out = Command::new("python3")
        .args([
            "-c",
            "import json, sys, yaml\nprint(json.dumps(yaml.safe_load(open(sys.argv[1]))['jobs']))",
        ])
        .arg(&file)
        .timed_output()
        .expect("qa_m0_54: python3 ran");
    assert!(
        out.status.success(),
        "qa_m0_54: the workflow is not YAML: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("qa_m0_54: PyYAML's JSON")
}

fn text(v: &Value) -> String {
    v.as_str().map(str::to_owned).unwrap_or_default()
}

fn needs_of(job: &Value) -> Vec<String> {
    match job.get("needs") {
        None => Vec::new(),
        Some(Value::Array(a)) => a.iter().map(text).collect(),
        Some(v) => vec![text(v)],
    }
}

/// The one step of `ci-gate`, which must read the result of every job it gathers.
fn gate_step(jobs: &Value) -> &Value {
    let step = &jobs["ci-gate"]["steps"][0];
    assert_eq!(
        step["env"]["RESULTS"].as_str(),
        Some(EVERY_RESULT),
        "qa_m0_54: `ci-gate` does not read every result it gathers"
    );
    step
}

fn check_gathers(yml: &str) {
    let jobs = jobs(yml);
    let mut needs = needs_of(&jobs["ci-gate"]);
    needs.sort();
    assert_eq!(
        needs,
        ["changes", "ci", "ci-checks", "ci-release", "ci-workspace"],
        "qa_m0_54: `ci-gate`'s `needs`"
    );
    gate_step(&jobs);
}

#[test]
fn qa_m0_54_the_gate_gathers_ci_release_and_reads_every_result() {
    check_gathers(&workflow());
}

negative_control!(
    qa_m0_54_the_gate_gathers_ci_release_and_reads_every_result,
    "ci.yml with `ci-release` out of `ci-gate`'s `needs`",
    expected = "`ci-gate`'s `needs`",
    check_gathers(&edited(
        "needs: [ci, ci-checks, ci-workspace, ci-release, changes]",
        "needs: [ci, ci-checks, ci-workspace, changes]",
    ))
);

mod a_result_left_unread {
    use super::*;
    negative_control!(
        qa_m0_54_the_gate_gathers_ci_release_and_reads_every_result,
        "ci.yml with `ci-release` in `needs` and the gate reading `ci`'s result alone",
        expected = "`ci-gate` does not read every result it gathers",
        check_gathers(&edited(
            concat!(
                "RESULTS: ${{ join(needs.*.result, ' ') }}\n",
                "          DOCS_ONLY: ${{ needs.ci-workspace.outputs.docs_only }}\n",
                "        run: for r in $RESULTS; do [ \"$r\" = success ] || { [ \"$r\" = skipped ] && ",
                "[ \"$DOCS_ONLY\" = true ]; } || { echo \"a job ci gathers",
            ),
            concat!(
                "RESULTS: ${{ needs.ci.result }}\n",
                "          DOCS_ONLY: ${{ needs.ci-workspace.outputs.docs_only }}\n",
                "        run: for r in $RESULTS; do [ \"$r\" = success ] || { [ \"$r\" = skipped ] && ",
                "[ \"$DOCS_ONLY\" = true ]; } || { echo \"a job ci gathers",
            ),
        ))
    );
}

// ---- A walk of the jobs `ci-gate` depends on ------------------------------------------------------------------------

/// How a job that ran ended, a shard at a time. A job not listed succeeds.
type Ending<'a> = (&'a str, &'a [&'a str]);

/// The one result GitHub's `needs` gives a job of `shards`: any failure, then any cancellation, else success.
fn joined(shards: &[&str]) -> String {
    ["failure", "cancelled"]
        .into_iter()
        .find(|bad| shards.contains(bad))
        .unwrap_or("success")
        .to_owned()
}

#[derive(Default)]
struct Walk {
    results: BTreeMap<String, String>,
    outputs: BTreeMap<String, BTreeMap<String, String>>,
}

impl Walk {
    /// One term of an expression: a quoted literal, `always()`, or an output of a job.
    fn term(&self, t: &str) -> String {
        let t = t.trim();
        if t == "always()" {
            return "true".to_owned();
        }
        if let Some(lit) = t.strip_prefix('\'').and_then(|l| l.strip_suffix('\'')) {
            return lit.to_owned();
        }
        let parts: Vec<&str> = t.split('.').collect();
        match parts.as_slice() {
            ["needs", job, "outputs", key] => self
                .outputs
                .get(*job)
                .and_then(|o| o.get(*key))
                .cloned()
                .unwrap_or_default(),
            _ => panic!("qa_m0_54: the walk does not know the term `{t}`"),
        }
    }

    /// `a && b && …` of `x == 'v'`, `x != 'v'` or `x`, inside `${{ }}` or not. One clause gives its value.
    fn eval(&self, expr: &str) -> String {
        let e = expr
            .trim()
            .trim_start_matches("${{")
            .trim_end_matches("}}")
            .trim();
        assert!(
            !e.contains("||"),
            "qa_m0_54: the walk does not know the expression `{e}`"
        );
        let values: Vec<String> = e
            .split(" && ")
            .map(|c| {
                if let Some((a, b)) = c.split_once(" == ") {
                    (self.term(a) == self.term(b)).to_string()
                } else if let Some((a, b)) = c.split_once(" != ") {
                    (self.term(a) != self.term(b)).to_string()
                } else {
                    self.term(c)
                }
            })
            .collect();
        match values.as_slice() {
            [one] => one.clone(),
            all => all.iter().all(|v| v == "true").to_string(),
        }
    }

    /// `ci-gate`'s own script under bash, as a `run:` step runs: `failure` or `success`.
    fn gate(&self, jobs: &Value) -> String {
        let step = gate_step(jobs);
        let results: Vec<&str> = needs_of(&jobs["ci-gate"])
            .iter()
            .map(|n| self.results[n].as_str())
            .collect();
        let home = Scratch::new("qa_m0_54_home");
        std::fs::create_dir_all(&*home).expect("scratch folder");
        let mut cmd = Command::new("bash");
        cmd.args(["-e", "-c", &text(&step["run"])])
            .env("HOME", &*home)
            .env("CARGO_HOME", &*home)
            .env("RESULTS", results.join(" "));
        for (k, v) in step["env"].as_object().expect("the gate's env") {
            if k != "RESULTS" {
                cmd.env(k, self.eval(&text(v)));
            }
        }
        let passed = cmd
            .timed_output()
            .expect("qa_m0_54: bash ran")
            .status
            .success();
        if passed { "success" } else { "failure" }.to_owned()
    }
}

/// The jobs `ci-gate` depends on, walked as GitHub runs them, `changes` giving `verdict`: a job whose `needs` didn't
/// all succeed skips unless its `if:` holds `always()`.
fn walk(jobs: &Value, verdict: &BTreeMap<String, String>, endings: &[Ending]) -> Walk {
    let mut wanted = vec!["ci-gate".to_owned()];
    let mut at = 0;
    while at < wanted.len() {
        for n in needs_of(&jobs[&wanted[at]]) {
            if !wanted.contains(&n) {
                wanted.push(n);
            }
        }
        at += 1;
    }
    let mut w = Walk::default();
    while w.results.len() < wanted.len() {
        let before = w.results.len();
        for id in &wanted {
            let job = &jobs[id];
            let needs = needs_of(job);
            if w.results.contains_key(id) || needs.iter().any(|n| !w.results.contains_key(n)) {
                continue;
            }
            let cond = text(&job["if"]);
            let runs = if cond.contains("always()") {
                w.eval(&cond) == "true"
            } else {
                needs.iter().all(|n| w.results[n] == "success")
                    && (cond.is_empty() || w.eval(&cond) == "true")
            };
            let result = if !runs {
                "skipped".to_owned()
            } else if id == "ci-gate" {
                w.gate(jobs)
            } else {
                endings
                    .iter()
                    .find(|(job, _)| job == id)
                    .map_or_else(|| "success".to_owned(), |(_, shards)| joined(shards))
            };
            let mut outs = BTreeMap::new();
            if id == "changes" {
                if result == "success" {
                    outs = verdict.clone();
                }
            } else if result != "skipped" {
                if let Some(declared) = job["outputs"].as_object() {
                    for (k, e) in declared.iter().filter(|(_, e)| text(e).contains("needs.")) {
                        outs.insert(k.clone(), w.eval(&text(e)));
                    }
                }
            }
            w.outputs.insert(id.clone(), outs);
            w.results.insert(id.clone(), result);
        }
        assert!(
            w.results.len() > before,
            "qa_m0_54: the jobs' `needs` never resolve"
        );
    }
    w
}

/// The real classifier's verdict on `paths`, on a run of the full tier unless `quick`.
fn verdict_on(paths: &[&str], quick: bool) -> BTreeMap<String, String> {
    let dir = Scratch::new("qa_m0_54_classify");
    std::fs::create_dir_all(&*dir).expect("scratch folder");
    let input = dir.join("paths");
    std::fs::write(&input, paths.join("\n")).expect("paths written");
    let out = Command::new("bash")
        .args(["-c", "bash \"$0\" classify < \"$1\""])
        .arg(root().join("scripts/ci_changes.sh"))
        .arg(&input)
        .env_remove("CI_REQS_BASE")
        .env_remove("CI_REQS_HEAD")
        .timed_output()
        .expect("qa_m0_54: bash ran");
    assert!(out.status.success(), "qa_m0_54: classify failed");
    let mut v: BTreeMap<String, String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
    v.insert("quick".to_owned(), quick.to_string());
    if quick {
        // `ci_tier.sh` on a quick run: `docs_only` true, so every full-only job skips (`ci.yml`'s header, R-684).
        v.insert("docs_only".to_owned(), "true".to_owned());
    }
    v
}

const CODE_HEADS: [&[&str]; 3] = [
    &["crates/kernel/src/lib.rs"],
    &["docs/design/principia_dd_replication.md", "xtask/src/ci.rs"],
    &[".github/workflows/ci.yml"],
];
const DOCS_OR_DATA_HEADS: [&[&str]; 3] = [
    &["plan/tasks.yaml"],
    &["docs/gui/reference/principia_colour_explorer.html"],
    &["docs/design/principia_dd_replication.md"],
];

/// The result of `ci-release` and of the gate `ci`, `want`, on `head` with `what`.
fn assert_ends(
    jobs: &Value,
    head: &BTreeMap<String, String>,
    what: &str,
    endings: &[Ending],
    want: (&str, &str),
) {
    let w = walk(jobs, head, endings);
    assert_eq!(
        (
            w.results["ci-release"].as_str(),
            w.results["ci-gate"].as_str()
        ),
        want,
        "qa_m0_54: (`ci-release`, `ci`) on {:?} with {what}",
        head.get("tier")
    );
}

fn check_endings(yml: &str) {
    let jobs = jobs(yml);
    let shards = jobs["ci-release"]["strategy"]["matrix"]["shard"]
        .as_array()
        .map_or(0, Vec::len);
    assert_eq!(shards, 2, "qa_m0_54: `ci-release`'s shards");
    for paths in CODE_HEADS {
        let head = verdict_on(paths, false);
        assert_eq!(head["docs_only"], "false", "qa_m0_54: {paths:?} is code");
        assert_ends(
            &jobs,
            &head,
            "every job passing",
            &[],
            ("success", "success"),
        );
        for (shard, ends) in [(1, ["failure", "success"]), (2, ["success", "failure"])] {
            let what = format!("`ci-release` shard {shard} failed");
            let endings: &[Ending] = &[("ci-release", &ends)];
            assert_ends(&jobs, &head, &what, endings, ("failure", "failure"));
        }
        for (shard, ends) in [(1, ["cancelled", "success"]), (2, ["success", "cancelled"])] {
            let what = format!("`ci-release` shard {shard} cancelled");
            let endings: &[Ending] = &[("ci-release", &ends)];
            assert_ends(&jobs, &head, &what, endings, ("cancelled", "failure"));
        }
        for bad in ["failure", "cancelled"] {
            let what = format!("`ci-release-archive` ended {bad}");
            let endings: &[Ending] = &[("ci-release-archive", &[bad])];
            assert_ends(&jobs, &head, &what, endings, ("skipped", "failure"));
        }
    }
    for paths in DOCS_OR_DATA_HEADS {
        let head = verdict_on(paths, false);
        assert_eq!(
            head["docs_only"], "true",
            "qa_m0_54: {paths:?} is data or docs"
        );
        assert_ends(
            &jobs,
            &head,
            "a data or docs head",
            &[],
            ("skipped", "success"),
        );
    }
    let any = verdict_on(CODE_HEADS[0], false);
    assert_ends(
        &jobs,
        &any,
        "the classification failed",
        &[("changes", &["failure"])],
        ("skipped", "failure"),
    );
}

#[test]
fn qa_m0_54_each_release_ending_on_each_head() {
    check_endings(&workflow());
}

/// The gate's script up to the end of its skip rule, each control's edit target.
#[cfg(feature = "controls")]
const SKIP_RULE: &str = "[ \"$DOCS_ONLY\" = true ]; } || { echo \"a job ci gathers";

negative_control!(
    qa_m0_54_each_release_ending_on_each_head,
    "ci.yml with the gate `ci` passing any skip",
    expected = "with `ci-release-archive` ended failure",
    check_endings(&edited(SKIP_RULE, "true; } || { echo \"a job ci gathers"))
);

mod a_failure_passed {
    use super::*;
    negative_control!(
        qa_m0_54_each_release_ending_on_each_head,
        "ci.yml with the gate `ci` passing a failed job",
        expected = "with `ci-release` shard 1 failed",
        check_endings(&edited(
            SKIP_RULE,
            "[ \"$DOCS_ONLY\" = true ]; } || [ \"$r\" = failure ] || { echo \"a job ci gathers",
        ))
    );
}

mod a_cancelled_shard_passed {
    use super::*;
    negative_control!(
        qa_m0_54_each_release_ending_on_each_head,
        "ci.yml with the gate `ci` passing a cancelled job",
        expected = "with `ci-release` shard 1 cancelled",
        check_endings(&edited(
            SKIP_RULE,
            "[ \"$DOCS_ONLY\" = true ]; } || [ \"$r\" = cancelled ] || { echo \"a job ci gathers",
        ))
    );
}

mod a_shard_run_without_its_archive {
    use super::*;
    negative_control!(
        qa_m0_54_each_release_ending_on_each_head,
        "ci.yml with `ci-release` running whatever its archive ended in",
        expected = "with `ci-release-archive` ended failure",
        check_endings(&edited(
            "    needs: ci-release-archive\n",
            "    needs: ci-release-archive\n    if: always()\n",
        ))
    );
}

mod a_docs_skip_refused {
    use super::*;
    negative_control!(
        qa_m0_54_each_release_ending_on_each_head,
        "ci.yml with the gate `ci` passing no skip",
        expected = "with a data or docs head",
        check_endings(&edited(
            "|| { [ \"$r\" = skipped ] && [ \"$DOCS_ONLY\" = true ]; } || { echo \"a job ci gathers",
            "|| { echo \"a job ci gathers",
        ))
    );
}

mod a_failed_classification_passed {
    use super::*;
    negative_control!(
        qa_m0_54_each_release_ending_on_each_head,
        "ci.yml with `changes` ungathered and the gate `ci` passing a skip unless the head is known to be code: the \
         failed classification's own result, which fails the gate as well, is out of the way",
        expected = "with the classification failed",
        check_endings(
            &edited(
                SKIP_RULE,
                "[ \"$DOCS_ONLY\" != false ]; } || { echo \"a job ci gathers",
            )
            .replacen(
                "needs: [ci, ci-checks, ci-workspace, ci-release, changes]",
                "needs: [ci, ci-checks, ci-workspace, ci-release]",
                1,
            )
        )
    );
}

fn check_quick(yml: &str) {
    let jobs = jobs(yml);
    let w = walk(&jobs, &verdict_on(CODE_HEADS[0], true), &[]);
    assert_eq!(
        (
            w.results["ci-release"].as_str(),
            w.results["ci-gate"].as_str()
        ),
        ("skipped", "skipped"),
        "qa_m0_54: (`ci-release`, `ci`) on a quick run"
    );
}

#[test]
fn qa_m0_54_the_quick_tier_is_untouched() {
    check_quick(&workflow());
}

negative_control!(
    qa_m0_54_the_quick_tier_is_untouched,
    "ci.yml with `ci-gate` running on a quick run",
    expected = "on a quick run",
    check_quick(&edited(
        "ci-release, changes]\n    if: always() && needs.changes.outputs.quick != 'true'\n",
        "ci-release, changes]\n    if: always()\n",
    ))
);

// ---- The result the gate reads can hold the failure -----------------------------------------------------------------

fn check_nothing_swallowed(yml: &str) {
    let jobs = jobs(yml);
    for id in ["ci-release", "ci-release-archive"] {
        let job = &jobs[id];
        assert!(
            job.get("continue-on-error").is_none(),
            "qa_m0_54: `{id}` passes whatever it ends in"
        );
        let steps = job["steps"].as_array().expect("the job's steps");
        let tests: Vec<&Value> = steps
            .iter()
            .filter(|s| {
                let run = text(&s["run"]);
                run.contains("nextest") || run.contains("cargo test")
            })
            .collect();
        assert!(!tests.is_empty(), "qa_m0_54: `{id}` has no test step");
        for step in tests {
            assert!(
                step.get("continue-on-error").is_none() && !text(&step["run"]).contains("|| true"),
                "qa_m0_54: a test step of `{id}` passes whatever it ends in: {}",
                text(&step["run"])
            );
        }
    }
}

#[test]
fn qa_m0_54_no_release_failure_is_swallowed() {
    check_nothing_swallowed(&workflow());
}

negative_control!(
    qa_m0_54_no_release_failure_is_swallowed,
    "ci.yml with `ci-release` allowed to fail",
    expected = "`ci-release` passes whatever it ends in",
    check_nothing_swallowed(&edited(
        "    needs: ci-release-archive\n",
        "    needs: ci-release-archive\n    continue-on-error: true\n",
    ))
);

mod a_test_step_allowed_to_fail {
    use super::*;
    negative_control!(
        qa_m0_54_no_release_failure_is_swallowed,
        "ci.yml with `ci-release`'s doctest step allowed to fail",
        expected = "a test step of `ci-release` passes whatever it ends in",
        check_nothing_swallowed(&edited(
            "        run: cargo test --workspace --release --no-fail-fast --doc\n",
            "        continue-on-error: true\n        run: cargo test --workspace --release --no-fail-fast --doc\n",
        ))
    );
}

// ---- `ci_retry.py` on the real `ci.yml`, `ci` now red with `ci-release` ---------------------------------------------

/// What a runner's full disk leaves in a job's failure annotation.
const NO_SPACE: &str = "Unhandled exception. System.IO.IOException: No space left on device";
/// What a real test failure leaves.
const REAL: &str = "Process completed with exit code 100.";

/// A job of a run: its name, how it ended and its failure annotation.
type RunJob<'a> = (&'a str, &'a str, Option<&'a str>);

/// Whether `ci_retry.py classify` re-runs a failed first attempt of `ci.yml` made of `failed`, every other job of the
/// gate's having passed.
fn retried(failed: &[RunJob]) -> bool {
    const PASSED: [&str; 8] = [
        "changes",
        "ci-archive",
        "ci (1)",
        "ci (2)",
        "ci (3)",
        "ci (4)",
        "ci-checks",
        "ci-workspace",
    ];
    let all: Vec<RunJob> = PASSED
        .iter()
        .map(|n| (*n, "success", None))
        .chain(failed.iter().copied())
        .collect();
    let dir = Scratch::new("qa_m0_54_run");
    std::fs::create_dir_all(dir.join("annotations")).expect("scratch folder");
    let listed: Vec<Value> = all
        .iter()
        .enumerate()
        .map(|(i, (name, conclusion, message))| {
            if let Some(message) = message {
                let annotation = json!([{"annotation_level": "failure", "message": message}]);
                std::fs::write(
                    dir.join("annotations").join(format!("{}.json", i + 1)),
                    annotation.to_string(),
                )
                .expect("annotation written");
            }
            json!({"id": i + 1, "name": name, "status": "completed", "conclusion": conclusion})
        })
        .collect();
    std::fs::write(
        dir.join("run.json"),
        json!({"conclusion": "failure", "run_attempt": 1}).to_string(),
    )
    .expect("run written");
    std::fs::write(dir.join("jobs.json"), Value::from(listed).to_string()).expect("jobs written");
    let out = Command::new("python3")
        .arg("-I")
        .arg(root().join(".github/scripts/ci_retry.py"))
        .arg("classify")
        .arg(&*dir)
        .arg(root().join(".github/workflows/ci.yml"))
        .timed_output()
        .expect("qa_m0_54: python3 ran");
    assert!(out.status.success(), "qa_m0_54: the classifier failed");
    let verdict = String::from_utf8_lossy(&out.stdout);
    match verdict.lines().next() {
        Some("retry=true") => true,
        Some("retry=false") => false,
        _ => panic!("qa_m0_54: no verdict in: {verdict}"),
    }
}

/// A release shard ended by `message`, the other passing, and `ci` red with it.
fn a_shard_failed(message: &str) -> bool {
    retried(&[
        ("ci-release-archive", "success", None),
        ("ci-release (1)", "failure", Some(message)),
        ("ci-release (2)", "success", None),
        ("ci", "failure", Some(REAL)),
    ])
}

/// The archive ended by `message`, the shards skipped, and `ci` red through the skip.
fn the_archive_failed(message: &str) -> bool {
    retried(&[
        ("ci-release-archive", "failure", Some(message)),
        // A job with no `name:` skipped at job level reports its bare id: `ci-release` on this PR's quick run.
        ("ci-release", "skipped", None),
        ("ci", "failure", Some(REAL)),
    ])
}

fn check_fault_retried(message: &str) {
    assert!(
        a_shard_failed(message),
        "qa_m0_54: a runner fault in a release shard and `ci` red: not re-run"
    );
    assert!(
        the_archive_failed(message),
        "qa_m0_54: a runner fault in the release archive and `ci` red: not re-run"
    );
}

#[test]
fn qa_m0_54_a_runner_fault_upstream_of_ci_is_retried() {
    check_fault_retried(NO_SPACE);
}

negative_control!(
    qa_m0_54_a_runner_fault_upstream_of_ci_is_retried,
    "a real failure in place of the fault",
    expected = "a runner fault in a release shard and `ci` red: not re-run",
    check_fault_retried(REAL)
);

fn check_real_not_retried(message: &str) {
    assert!(
        !a_shard_failed(message),
        "qa_m0_54: a real failure in a release shard is re-run"
    );
    assert!(
        !the_archive_failed(message),
        "qa_m0_54: a real failure in the release archive is re-run"
    );
}

#[test]
fn qa_m0_54_a_real_release_failure_is_not_retried() {
    check_real_not_retried(REAL);
}

negative_control!(
    qa_m0_54_a_real_release_failure_is_not_retried,
    "a runner fault in place of the real failure",
    expected = "a real failure in a release shard is re-run",
    check_real_not_retried(NO_SPACE)
);
