//! QA tests for TASK-M0-19's gate report and cadence workflows, written from REQ-SYS-067 ("CI must run at the cadence
//! R-110, R-177 and R-186 set: ci.yml runs `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`
//! and `cargo xtask ci` on every push; nightly.yml, scheduled, runs the CPU and lavapipe suites, not benchmarks;
//! gate.yml, dispatched with a milestone, runs every hosted suite and writes the gate report. Benchmarks and
//! performance gates run only on the human's own Mac"; verify: the gate-report fixture lists every requirement of the
//! milestone's and earlier gate blocks, fails on a missing one, and marks benchmark requirements awaiting the human's
//! run until their prin profile files are supplied; no workflow runs bench, nightly.yml is scheduled, gate.yml takes a
//! milestone input), from REQ-VAL-150 (a scheduled nightly runs the full `cargo mutants` over the workspace with R-196's
//! exclusions, the per-PR job's one list, and uploads its report of caught, missed, unviable and timed-out mutants) and
//! from REQ-VAL-179 (the nightly run's timeout and memory cap read from the one place the per-PR shards read them).
//! Each test has its negative control (R-176).
// The file name `qa_TASK-M0-19` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::path::{Path, PathBuf};

use validation::negative_control;
use xtask::gate_report::{self, Recorded};

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

// ---- The gate report, on a fixture of three milestones ----------------------------------------------------------------

const MILESTONES: &str = "\
# Fixture milestones

<!-- gate:M0 -->
**Exit gate — 3 requirements** (and every earlier gate still green):

- AAA (2): REQ-AAA-001…002
- PERF (1): REQ-PERF-001
<!-- /gate:M0 -->

<!-- gate:M1 -->
**Exit gate — 2 requirements** (and every earlier gate still green):

- BBB (2): REQ-BBB-001, REQ-BBB-003
<!-- /gate:M1 -->

<!-- gate:M2 -->
**Exit gate — 1 requirements** (and every earlier gate still green):

- CCC (1): REQ-CCC-001
<!-- /gate:M2 -->
";

fn requirement(id: &str, method: &str, milestone: &str) -> String {
    format!(
        "- id: {id}\n  area: X\n  statement: fixture\n  verify:\n    method: {method}\n    detail: fixture\n  milestone: {milestone}\n"
    )
}

/// A workspace root holding the fixture plan: REQ-PERF-001 a benchmark, REQ-BBB-003 a review checklist closed by
/// TASK-M1-01, the rest unit tests.
fn fixture() -> Scratch {
    let dir = Scratch::new("qa_m019_gate_report");
    std::fs::create_dir_all(dir.join("plan/tasks/M1")).unwrap();
    std::fs::write(dir.join("plan/MILESTONES.md"), MILESTONES).unwrap();
    let reqs = [
        requirement("REQ-AAA-001", "unit test", "M0"),
        requirement("REQ-AAA-002", "property test", "M0"),
        requirement("REQ-PERF-001", "benchmark", "M0"),
        requirement("REQ-BBB-001", "unit test", "M1"),
        requirement("REQ-BBB-003", "review checklist", "M1"),
        requirement("REQ-CCC-001", "unit test", "M2"),
    ]
    .concat();
    std::fs::write(dir.join("plan/requirements.yaml"), reqs).unwrap();
    std::fs::write(
        dir.join("plan/tasks.yaml"),
        "- id: TASK-M1-01\n  milestone: M1\n  title: \"fixture\"\n  requirements: [REQ-BBB-003]\n  reviewers: [code, qa]\n  status: todo\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("plan/tasks/M1/TASK-M1-01.md"),
        "# TASK-M1-01 — fixture\n\n- **Milestone:** M1\n- **Reviewers:** code, qa\n",
    )
    .unwrap();
    dir
}

/// TASK-M1-01's PR, merged with both reviewers' approvals on its head, or not merged.
fn prs(merged: bool) -> Recorded {
    Recorded::from_json(&format!(
        r#"{{"TASK-M1-01": [{{"merged": {merged}, "number": 7, "title": "TASK-M1-01: fixture", "head": "h7",
            "reviews": [
              {{"body": "VERDICT: APPROVE code", "commit_id": "h7", "submitted_at": "2026-10-01T10:00:00Z"}},
              {{"body": "VERDICT: APPROVE qa", "commit_id": "h7", "submitted_at": "2026-10-01T11:00:00Z"}}
            ]}}]}}"#
    ))
    .unwrap()
}

/// The report runner under test, or a control's broken one: the run's result, and the report file's text.
type Report = fn(&Path, &str, &Path, Option<&Path>, &Recorded) -> (Result<(), String>, String);

fn report(
    root: &Path,
    milestone: &str,
    results: &Path,
    bench: Option<&Path>,
    prs: &Recorded,
) -> (Result<(), String>, String) {
    let result = gate_report::run(root, milestone, results, bench, prs);
    let file = root
        .join("target/gate-report")
        .join(format!("{milestone}.txt"));
    let text = std::fs::read_to_string(&file)
        .unwrap_or_else(|e| panic!("the report was not written to {}: {e}", file.display()));
    (result, text)
}

/// Writes `results` (id, outcome) as the results file in `root`, and returns its path.
fn results(root: &Path, results: &[(&str, &str)]) -> PathBuf {
    let map: serde_json::Map<String, serde_json::Value> = results
        .iter()
        .map(|(k, v)| ((*k).to_owned(), serde_json::Value::from(*v)))
        .collect();
    let path = root.join("results.json");
    std::fs::write(&path, serde_json::Value::Object(map).to_string()).unwrap();
    path
}

/// The report's line for `id`, the line that starts with it.
fn line<'a>(text: &'a str, id: &str) -> Option<&'a str> {
    text.lines()
        .find(|l| l.split_whitespace().next() == Some(id))
}

const ALL_PASS: [(&str, &str); 3] = [
    ("REQ-AAA-001", "pass"),
    ("REQ-AAA-002", "pass"),
    ("REQ-BBB-001", "pass"),
];

/// M1's report names every requirement of M1's block and of the earlier M0's, and none of the later M2's; all given
/// passing (the review-checklist one by its merged, approved PR), it passes, the benchmark awaiting the human's run.
fn check_lists_every(run: Report) {
    let root = fixture();
    let res = results(&root, &ALL_PASS);
    let (result, text) = run(&root, "M1", &res, None, &prs(true));
    for id in [
        "REQ-AAA-001",
        "REQ-AAA-002",
        "REQ-PERF-001",
        "REQ-BBB-001",
        "REQ-BBB-003",
    ] {
        assert!(
            line(&text, id).is_some(),
            "the M1 report does not name {id}:\n{text}"
        );
    }
    assert!(
        line(&text, "REQ-CCC-001").is_none(),
        "the M1 report names M2's REQ-CCC-001:\n{text}"
    );
    for id in ["REQ-AAA-001", "REQ-BBB-001", "REQ-BBB-003"] {
        let l = line(&text, id).unwrap().to_lowercase();
        assert!(
            l.contains("pass") && !l.contains("fail"),
            "{id} is not reported passing: {l}"
        );
    }
    assert!(
        result.is_ok(),
        "a report with every requirement passing or awaiting fails: {result:?}\n{text}"
    );
}

#[test]
fn qa_m019_gate_report_lists_the_milestone_and_every_earlier_gate() {
    check_lists_every(report);
}

negative_control!(
    qa_m019_gate_report_lists_the_milestone_and_every_earlier_gate,
    "a report that drops an earlier gate's requirement must fail",
    expected = "the M1 report does not name REQ-AAA-002",
    check_lists_every(|r, m, res, b, p| {
        let (result, text) = report(r, m, res, b, p);
        let kept: Vec<&str> = text
            .lines()
            .filter(|l| !l.starts_with("REQ-AAA-002"))
            .collect();
        (result, kept.join("\n"))
    })
);

/// A requirement with no result fails the report, and its line says so; so does a requirement whose run failed.
fn check_missing_fails(run: Report) {
    let root = fixture();
    let res = results(&root, &[("REQ-AAA-001", "pass"), ("REQ-BBB-001", "pass")]);
    let (result, text) = run(&root, "M1", &res, None, &prs(true));
    assert!(
        result.is_err(),
        "a report missing REQ-AAA-002's result passes:\n{text}"
    );
    let l = line(&text, "REQ-AAA-002").expect("the missing requirement is not listed");
    assert!(
        !l.to_lowercase().contains("pass"),
        "the missing requirement is reported as a pass: {l}"
    );
    let res = results(
        &root,
        &[
            ("REQ-AAA-001", "pass"),
            ("REQ-AAA-002", "fail"),
            ("REQ-BBB-001", "pass"),
        ],
    );
    let (result, text) = run(&root, "M1", &res, None, &prs(true));
    assert!(
        result.is_err(),
        "a report with REQ-AAA-002 failed passes:\n{text}"
    );
}

#[test]
fn qa_m019_gate_report_fails_on_a_missing_requirement() {
    check_missing_fails(report);
}

negative_control!(
    qa_m019_gate_report_fails_on_a_missing_requirement,
    "a report that passes whatever is missing must fail",
    expected = "a report missing REQ-AAA-002's result passes",
    check_missing_fails(|r, m, res, b, p| {
        let (_, text) = report(r, m, res, b, p);
        (Ok(()), text)
    })
);

/// A benchmark requirement is awaiting the human's run until its prin profile file is supplied, and awaiting doesn't
/// fail the report; supplied, it is no longer awaiting; a supplied file that is not schema v1 is refused.
fn check_awaiting(run: Report) {
    let root = fixture();
    let res = results(&root, &ALL_PASS);
    let (result, text) = run(&root, "M0", &res, None, &prs(true));
    let l = line(&text, "REQ-PERF-001").expect("the benchmark requirement is not listed");
    assert!(
        l.to_lowercase().contains("awaiting"),
        "the benchmark requirement is not awaiting the human's run: {l}"
    );
    assert!(
        result.is_ok(),
        "an awaiting benchmark fails the report:\n{text}"
    );
    let empty = root.join("bench-empty");
    std::fs::create_dir_all(&empty).unwrap();
    let (_, text) = run(&root, "M0", &res, Some(&empty), &prs(true));
    let l = line(&text, "REQ-PERF-001").unwrap();
    assert!(
        l.to_lowercase().contains("awaiting"),
        "a benchmark whose file is absent from the directory is not awaiting: {l}"
    );
    let bench = root.join("bench");
    std::fs::create_dir_all(&bench).unwrap();
    let header = std::fs::read_to_string(root_of_workspace_baseline()).unwrap();
    std::fs::write(bench.join("REQ-PERF-001.jsonl"), header).unwrap();
    let (result, text) = run(&root, "M0", &res, Some(&bench), &prs(true));
    let l = line(&text, "REQ-PERF-001").unwrap();
    assert!(
        !l.to_lowercase().contains("awaiting"),
        "a benchmark whose prin profile file is supplied is still awaiting: {l}"
    );
    assert!(
        result.is_ok(),
        "a supplied benchmark fails the report:\n{text}"
    );
    std::fs::write(
        bench.join("REQ-PERF-001.jsonl"),
        "{\"not\": \"a profile\"}\n",
    )
    .unwrap();
    assert!(
        gate_report::run(&root, "M0", &res, Some(&bench), &prs(true)).is_err(),
        "a supplied file that is not schema v1 is accepted"
    );
}

fn root_of_workspace_baseline() -> PathBuf {
    root().join("fixtures/bench/trivial-kernel/baseline.json")
}

#[test]
fn qa_m019_gate_report_benchmark_awaits_the_humans_run() {
    check_awaiting(report);
}

negative_control!(
    qa_m019_gate_report_benchmark_awaits_the_humans_run,
    "a report that never marks a benchmark awaiting must fail",
    expected = "the benchmark requirement is not awaiting the human's run",
    check_awaiting(|r, m, res, b, p| {
        let (result, text) = report(r, m, res, b, p);
        (result, text.replace("awaiting", "done"))
    })
);

/// A review-checklist requirement whose closing task's PR has not merged fails the report.
fn check_unmerged(run: Report) {
    let root = fixture();
    let res = results(&root, &ALL_PASS);
    let (result, text) = run(&root, "M1", &res, None, &prs(false));
    assert!(
        result.is_err(),
        "a review-checklist requirement whose task never merged passes:\n{text}"
    );
    let l = line(&text, "REQ-BBB-003").unwrap().to_lowercase();
    assert!(!l.contains("pass"), "the unmerged one reads as a pass: {l}");
}

#[test]
fn qa_m019_gate_report_unmerged_review_fails() {
    check_unmerged(report);
}

negative_control!(
    qa_m019_gate_report_unmerged_review_fails,
    "a report that reads any PR as merged must fail",
    expected = "a review-checklist requirement whose task never merged passes",
    check_unmerged(|r, m, res, b, _| report(r, m, res, b, &prs(true)))
);

/// The real plan: M0's gate block holds this task's requirements, M1's report holds every M0 one too, and the only
/// verification methods M0's requirements use are those gate.yml gives a result for (or the review and benchmark
/// paths), so no M0 requirement can go missing for want of a suite.
fn check_real_plan(milestones: &str) {
    let m0 = gate_report::gate_ids(milestones, "M0").expect("M0's gate block does not parse");
    for id in ["REQ-SYS-067", "REQ-TOOL-001", "REQ-TOOL-121", "REQ-VAL-150"] {
        assert!(m0.iter().any(|i| i == id), "M0's gate does not list {id}");
    }
    let m1 = gate_report::gate_ids(milestones, "M1").expect("M1's gate block does not parse");
    assert!(
        m0.iter().all(|id| m1.contains(id)) && m1.len() > m0.len(),
        "M1's report does not hold every M0 requirement as well as its own"
    );
    let reqs = std::fs::read_to_string(root().join("plan/requirements.yaml")).unwrap();
    assert!(
        gate_report::benchmark_ids(&reqs).contains("REQ-PERF-004"),
        "REQ-PERF-004, M3's first benchmark requirement, is not read as a benchmark"
    );
}

#[test]
fn qa_m019_gate_report_reads_the_real_plan() {
    check_real_plan(&std::fs::read_to_string(root().join("plan/MILESTONES.md")).unwrap());
}

negative_control!(
    qa_m019_gate_report_reads_the_real_plan,
    "a plan whose M0 gate omits REQ-SYS-067 must fail",
    expected = "M0's gate does not list REQ-SYS-067",
    check_real_plan(
        &std::fs::read_to_string(root().join("plan/MILESTONES.md"))
            .unwrap()
            .replace("REQ-SYS-063…078", "REQ-SYS-063…066, REQ-SYS-068…079")
    )
);

// ---- The workflows: cadence (R-177, R-186) ---------------------------------------------------------------------------

/// Every workflow file, as (name, text).
fn workflows() -> Vec<(String, String)> {
    let mut files: Vec<(String, String)> = std::fs::read_dir(root().join(".github/workflows"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| matches!(p.extension().and_then(|e| e.to_str()), Some("yml" | "yaml")))
        .map(|p| {
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read_to_string(&p).unwrap(),
            )
        })
        .collect();
    files.sort();
    files
}

fn workflow(files: &[(String, String)], name: &str) -> String {
    files
        .iter()
        .find(|(f, _)| f == name)
        .unwrap_or_else(|| panic!("there is no .github/workflows/{name}"))
        .1
        .clone()
}

/// The text's lines with comments removed (a `#` at a line's start, or after whitespace), and each step's or
/// workflow's `name:` blanked, so that only what runs is read, never a label.
fn code(text: &str) -> String {
    text.lines()
        .map(|l| {
            let t = l.trim_start();
            if t.starts_with("- name:") || t.starts_with("name:") {
                return "";
            }
            let cut = l
                .char_indices()
                .find(|&(i, c)| c == '#' && (i == 0 || l[..i].ends_with(char::is_whitespace)))
                .map_or(l.len(), |(i, _)| i);
            &l[..cut]
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The workflow's `on:` block (its triggers), up to the next top-level key.
fn triggers(text: &str) -> String {
    let code = code(text);
    let mut lines = code.lines().skip_while(|l| !l.starts_with("on:"));
    let first = lines.next().unwrap_or_default().to_owned();
    let rest: Vec<&str> = lines
        .take_while(|l| l.is_empty() || l.starts_with(' '))
        .collect();
    format!("{first}\n{}", rest.join("\n"))
}

/// The jobs of a workflow, as (id, code text).
fn jobs(text: &str) -> Vec<(String, String)> {
    let code = code(text);
    let mut out: Vec<(String, String)> = Vec::new();
    let mut in_jobs = false;
    for l in code.lines() {
        if l.starts_with("jobs:") {
            in_jobs = true;
            continue;
        }
        if !in_jobs {
            continue;
        }
        if !l.is_empty() && !l.starts_with(' ') {
            break;
        }
        if let Some(id) = l.strip_prefix("  ").and_then(|r| r.strip_suffix(':')) {
            if !id.starts_with(' ') {
                out.push((id.to_owned(), String::new()));
                continue;
            }
        }
        if let Some((_, body)) = out.last_mut() {
            body.push_str(l);
            body.push('\n');
        }
    }
    out
}

/// Whether a code text runs a benchmark: `xtask bench`, the bench binary, `cargo bench`, or `prin profile` (where the
/// human's benchmarks run).
fn runs_bench(code: &str) -> Option<String> {
    code.lines()
        .map(str::trim)
        .find(|l| {
            let w: Vec<&str> = l.split_whitespace().collect();
            w.windows(2).any(|p| {
                p == ["xtask", "bench"] || p == ["cargo", "bench"] || p == ["--bin", "bench"]
            }) || w.windows(2).any(|p| p == ["prin", "profile"])
        })
        .map(str::to_owned)
}

fn check_no_bench(files: &[(String, String)]) {
    for (name, text) in files {
        if let Some(l) = runs_bench(&code(text)) {
            panic!("{name} runs a benchmark on a hosted runner (R-186): `{l}`");
        }
    }
}

#[test]
fn qa_m019_no_workflow_runs_a_benchmark() {
    check_no_bench(&workflows());
}

negative_control!(
    qa_m019_no_workflow_runs_a_benchmark,
    "a nightly that runs the benches must fail",
    expected = "nightly.yml runs a benchmark on a hosted runner",
    check_no_bench(&edited(
        "nightly.yml",
        "      - name: cargo xtask ci\n        run: cargo xtask ci\n",
        "      - name: cargo xtask ci\n        run: cargo xtask ci\n      - run: cargo xtask bench --all\n"
    ))
);

/// The workflows with `from` replaced by `to` in the named file.
#[cfg(feature = "controls")]
fn edited(file: &str, from: &str, to: &str) -> Vec<(String, String)> {
    let mut files = workflows();
    let (_, text) = files.iter_mut().find(|(f, _)| f == file).unwrap();
    assert!(text.contains(from), "{file} has no {from:?} to edit");
    *text = text.replacen(from, to, 1);
    files
}

/// ci.yml runs on every push, with formatting, clippy at `-D warnings` over every target, and `cargo xtask ci`.
fn check_ci(files: &[(String, String)]) {
    let ci = workflow(files, "ci.yml");
    assert!(
        triggers(&ci).lines().any(|l| l.trim().starts_with("push")),
        "ci.yml does not run on every push"
    );
    let c = code(&ci);
    let runs = |words: &[&str]| {
        c.lines().any(|l| {
            let w: Vec<&str> = l.split_whitespace().collect();
            words.iter().all(|x| w.contains(x))
        })
    };
    assert!(
        runs(&["cargo", "fmt", "--check"]),
        "ci.yml does not run `cargo fmt --check`"
    );
    assert!(
        runs(&[
            "cargo",
            "clippy",
            "--workspace",
            "--all-targets",
            "-D",
            "warnings"
        ]),
        "ci.yml does not run clippy over the workspace's targets at -D warnings"
    );
    assert!(
        jobs(&ci)
            .iter()
            .any(|(_, j)| runs_every_slice_of_xtask_ci(j)),
        "ci.yml does not run `cargo xtask ci`, whole or as every one of its slices"
    );
}

/// Whether a job runs all of `cargo xtask ci` on each push: unpartitioned, or sharded as R-360 has it, `cargo xtask ci
/// --partition ${{ matrix.shard }}/<n>` in a job whose `shard` matrix is exactly 1..=n, so that the shards together run
/// every slice (REQ-SYS-077).
fn runs_every_slice_of_xtask_ci(job: &str) -> bool {
    let commands: Vec<&str> = job
        .lines()
        .map(|l| {
            let t = l.trim();
            let t = t.strip_prefix("- ").unwrap_or(t);
            t.strip_prefix("run:").unwrap_or(t).trim()
        })
        .collect();
    if commands.contains(&"cargo xtask ci") {
        return true;
    }
    let Some(n) = commands.iter().find_map(|c| {
        c.strip_prefix("cargo xtask ci --partition ${{ matrix.shard }}/")
            .and_then(|n| n.parse::<u32>().ok())
    }) else {
        return false;
    };
    let shards: Option<Vec<u32>> = commands.iter().find_map(|c| {
        let list = c.strip_prefix("shard:")?.trim();
        let list = list.strip_prefix('[')?.strip_suffix(']')?;
        list.split(',').map(|x| x.trim().parse().ok()).collect()
    });
    n > 0 && shards == Some((1..=n).collect())
}

#[test]
fn qa_m019_ci_runs_fmt_clippy_and_xtask_ci_on_push() {
    check_ci(&workflows());
}

negative_control!(
    qa_m019_ci_runs_fmt_clippy_and_xtask_ci_on_push,
    "a ci.yml whose clippy allows warnings must fail",
    expected = "ci.yml does not run clippy",
    check_ci(&edited(
        "ci.yml",
        "        run: cargo clippy --workspace --all-targets -- -D warnings",
        "        run: cargo clippy --workspace --all-targets"
    ))
);

/// The sharded `cargo xtask ci` with one slice dropped from the matrix no longer runs all of it.
#[test]
fn qa_m019_ci_runs_every_slice_of_xtask_ci() {
    check_ci(&workflows());
}

negative_control!(
    qa_m019_ci_runs_every_slice_of_xtask_ci,
    "a ci.yml whose xtask-ci matrix drops slice 4 of 4 must fail",
    expected = "ci.yml does not run `cargo xtask ci`",
    check_ci(&edited(
        "ci.yml",
        "  xtask-ci:\n    runs-on: ubuntu-latest\n    env:\n      PRIN_GPU_BACKEND: vulkan\n    strategy:\n      fail-fast: false\n      matrix:\n        shard: [1, 2, 3, 4]\n",
        "  xtask-ci:\n    runs-on: ubuntu-latest\n    env:\n      PRIN_GPU_BACKEND: vulkan\n    strategy:\n      fail-fast: false\n      matrix:\n        shard: [1, 2, 3]\n"
    ))
);

/// ci.yml runs `cargo xtask ci` on each push in some form.
#[test]
fn qa_m019_ci_runs_xtask_ci() {
    check_ci(&workflows());
}

negative_control!(
    qa_m019_ci_runs_xtask_ci,
    "a ci.yml with no `cargo xtask ci` at all must fail",
    expected = "ci.yml does not run `cargo xtask ci`",
    check_ci(&edited(
        "ci.yml",
        "        run: cargo xtask ci --partition ${{ matrix.shard }}/4\n",
        "        run: cargo xtask controls --partition ${{ matrix.shard }}/4\n"
    ))
);

/// nightly.yml is scheduled (a cron) and dispatchable, never per-push or per-PR; a job runs `cargo xtask ci`, and a
/// job runs the GPU suites on lavapipe (Mesa's Vulkan driver, the backend set to vulkan).
fn check_nightly(files: &[(String, String)]) {
    let nightly = workflow(files, "nightly.yml");
    let on = triggers(&nightly);
    assert!(
        on.contains("schedule:") && on.contains("cron:"),
        "nightly.yml is not scheduled: {on}"
    );
    assert!(
        !on.contains("push") && !on.contains("pull_request"),
        "nightly.yml runs per push or per PR: {on}"
    );
    let jobs = jobs(&nightly);
    assert!(
        jobs.iter()
            .any(|(_, j)| j.lines().any(|l| l.trim().ends_with("cargo xtask ci"))),
        "nightly.yml runs no CPU suites (`cargo xtask ci`)"
    );
    assert!(
        jobs.iter().any(|(_, j)| j.contains("mesa-vulkan-drivers")
            && j.contains("PRIN_GPU_BACKEND: vulkan")
            && (j.contains("gpu_harness") || j.contains("golden --all"))),
        "nightly.yml runs no lavapipe GPU suite"
    );
}

#[test]
fn qa_m019_nightly_is_scheduled_with_cpu_and_lavapipe_suites() {
    check_nightly(&workflows());
}

negative_control!(
    qa_m019_nightly_is_scheduled_with_cpu_and_lavapipe_suites,
    "a nightly with no schedule must fail",
    expected = "nightly.yml is not scheduled",
    check_nightly(&edited(
        "nightly.yml",
        "  schedule:\n    - cron: \"0 3 * * *\"\n",
        ""
    ))
);

/// gate.yml is dispatched by hand with a required `milestone` input, runs every hosted suite (the CPU suites, Metal on
/// macos-15, lavapipe, and every screenshot), then the gate report for that milestone, and uploads the report.
fn check_gate(files: &[(String, String)]) {
    let gate = workflow(files, "gate.yml");
    let on = triggers(&gate);
    assert!(
        on.contains("workflow_dispatch:"),
        "gate.yml is not dispatched by hand: {on}"
    );
    let input = on
        .split_once("milestone:")
        .unwrap_or_else(|| panic!("gate.yml takes no `milestone` input: {on}"))
        .1;
    assert!(
        input.contains("required: true"),
        "gate.yml's milestone input is not required"
    );
    assert!(
        !on.contains("push") && !on.contains("schedule"),
        "gate.yml runs on its own, not on a push or a schedule: {on}"
    );
    let jobs = jobs(&gate);
    let any = |p: &dyn Fn(&str) -> bool| jobs.iter().any(|(_, j)| p(j));
    assert!(
        any(&|j| j.lines().any(|l| l.trim().ends_with("cargo xtask ci"))),
        "gate.yml runs no `cargo xtask ci`"
    );
    assert!(
        any(&|j| j.contains("runs-on: macos-15")),
        "gate.yml runs no Metal suite on macos-15"
    );
    assert!(
        any(&|j| j.contains("mesa-vulkan-drivers") && j.contains("golden --all")),
        "gate.yml runs no lavapipe suite"
    );
    assert!(
        any(&|j| j.contains("cargo xtask screenshot --all")),
        "gate.yml runs no screenshot suite"
    );
    let (_, report) = jobs
        .iter()
        .find(|(_, j)| j.contains("gate-report --milestone"))
        .expect("gate.yml writes no gate report");
    assert!(
        report.contains("inputs.milestone"),
        "the gate report is not for the dispatched milestone"
    );
    assert!(
        report.contains("actions/upload-artifact") && report.contains("gate-report"),
        "the gate report is not uploaded"
    );
    assert!(
        report.contains("needs:"),
        "the gate report does not wait for the suites"
    );
}

#[test]
fn qa_m019_gate_takes_a_milestone_and_reports_it() {
    check_gate(&workflows());
}

negative_control!(
    qa_m019_gate_takes_a_milestone_and_reports_it,
    "a gate.yml without its milestone input must fail",
    expected = "gate.yml takes no `milestone` input",
    check_gate(&edited("gate.yml", "      milestone:\n", "      stage:\n"))
);

/// nightly.yml's full mutants run: `cargo mutants` over the whole workspace (no `--in-diff`, no `--file` or
/// `--package` narrowing), with the exclusions and caps of the one `.cargo/mutants.toml` the per-PR job reads (no
/// `--config` elsewhere, no `--exclude` or `--timeout` of its own, no `ulimit` or `prlimit` of its own), on Linux,
/// where the memory cap binds; its report lists caught, missed, unviable and timed-out mutants and is uploaded.
fn check_mutants(files: &[(String, String)], config: &str) {
    let nightly = workflow(files, "nightly.yml");
    let jobs = jobs(&nightly);
    let runs: Vec<&str> = jobs
        .iter()
        .flat_map(|(_, j)| j.lines())
        .filter(|l| {
            l.split_whitespace()
                .collect::<Vec<_>>()
                .windows(2)
                .any(|w| w == ["cargo", "mutants"])
        })
        .collect();
    assert!(!runs.is_empty(), "nightly.yml runs no `cargo mutants`");
    for l in &runs {
        for narrowing in [
            "--in-diff",
            "--file",
            "-f ",
            "--package",
            "-p ",
            "--exclude",
            "-e ",
            "--re",
            "--examine-re",
        ] {
            assert!(
                !l.contains(narrowing),
                "the nightly mutants run is narrowed by `{narrowing}`: {l}"
            );
        }
        for own in [
            "--config",
            "--no-config",
            "--timeout",
            "--minimum-test-timeout",
            "--timeout-multiplier",
        ] {
            assert!(
                !l.contains(own),
                "the nightly mutants run sets `{own}` of its own, not the one list's: {l}"
            );
        }
    }
    let (_, job) = jobs
        .iter()
        .find(|(_, j)| j.lines().any(|l| runs.contains(&l)))
        .unwrap();
    assert!(
        job.contains("runs-on: ubuntu"),
        "the nightly mutants run is not on Linux, where the memory cap binds"
    );
    assert!(
        !job.contains("ulimit") && !job.contains("prlimit"),
        "the nightly mutants run sets a memory cap of its own, not the one list's"
    );
    for key in [
        "exclude_globs",
        "timeout_multiplier",
        "minimum_test_timeout",
        "prlimit",
    ] {
        assert!(
            code(config).contains(key),
            ".cargo/mutants.toml, the one list, has no `{key}`"
        );
    }
    let uploads = jobs
        .iter()
        .filter(|(_, j)| j.contains("actions/upload-artifact"))
        .map(|(_, j)| j.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        uploads.contains("mutants"),
        "nightly.yml uploads no mutants report"
    );
    let reporter = jobs
        .iter()
        .map(|(_, j)| j.as_str())
        .find(|j| j.contains("actions/upload-artifact") && ["caught", "missed", "unviable", "timeout"].iter().all(|k| j.contains(k)))
        .unwrap_or_else(|| panic!("nightly.yml uploads no report listing caught, missed, unviable and timed-out mutants"));
    assert!(
        reporter.contains("mutants"),
        "the uploaded report is not the mutants run's"
    );
}

fn mutants_toml() -> String {
    std::fs::read_to_string(root().join(".cargo/mutants.toml")).unwrap()
}

#[test]
fn qa_m019_nightly_runs_the_full_mutants_under_the_one_list() {
    check_mutants(&workflows(), &mutants_toml());
}

negative_control!(
    qa_m019_nightly_runs_the_full_mutants_under_the_one_list,
    "a nightly that runs only a diff's mutants must fail",
    expected = "narrowed by `--in-diff`",
    check_mutants(
        &edited(
            "nightly.yml",
            "cargo mutants --in-place",
            "cargo mutants --in-diff x.diff --in-place"
        ),
        &mutants_toml()
    )
);

negative_control!(
    qa_m019_nightly_runs_the_full_mutants_under_the_one_list_timeout,
    "a nightly with a timeout of its own must fail",
    expected = "sets `--timeout` of its own",
    check_mutants(
        &edited(
            "nightly.yml",
            "cargo mutants --in-place",
            "cargo mutants --timeout 600 --in-place"
        ),
        &mutants_toml()
    )
);

negative_control!(
    qa_m019_nightly_runs_the_full_mutants_under_the_one_list_report,
    "a nightly whose report drops the timed-out mutants must fail",
    expected = "uploads no report listing caught, missed, unviable and timed-out",
    check_mutants(
        &workflows()
            .into_iter()
            .map(|(f, t)| {
                let t = if f == "nightly.yml" {
                    t.replace("timeout", "")
                } else {
                    t
                };
                (f, t)
            })
            .collect::<Vec<_>>(),
        &mutants_toml()
    )
);

/// The report's summary counts what its lines say: on the fixture's M1, five requirements, four passing (three by
/// their suites, one by its merged, approved PR) and one awaiting the human's run.
fn check_summary(run: Report) {
    let root = fixture();
    let res = results(&root, &ALL_PASS);
    let (_, text) = run(&root, "M1", &res, None, &prs(true));
    let summary = text
        .lines()
        .find(|l| l.contains("requirements:"))
        .unwrap_or_else(|| panic!("the report has no summary line:\n{text}"));
    for count in [
        "5 requirements",
        "4 pass",
        "0 fail",
        "0 missing",
        "1 awaiting",
    ] {
        assert!(
            summary.contains(count),
            "the summary does not count `{count}`: {summary}"
        );
    }
}

#[test]
fn qa_m019_gate_report_summary_counts_its_lines() {
    check_summary(report);
}

negative_control!(
    qa_m019_gate_report_summary_counts_its_lines,
    "a summary that counts no pass must fail",
    expected = "the summary does not count `4 pass`",
    check_summary(|r, m, res, b, p| {
        let (result, text) = report(r, m, res, b, p);
        (result, text.replace(" 4 pass", " 0 pass"))
    })
);

/// A one-id range (`REQ-X-007…007`) is that id: the gate blocks write ranges, and one whose ends meet is still a range.
fn check_one_id_range(ids: fn(&str, &str) -> Result<Vec<String>, String>) {
    let block = "<!-- gate:M0 -->\n**Exit gate — 3 requirements** (and every earlier gate still green):\n\n\
                 - X (3): REQ-X-007…007, REQ-X-010…011\n<!-- /gate:M0 -->\n";
    let got = ids(block, "M0").unwrap_or_else(|e| panic!("a one-id range is refused: {e}"));
    assert_eq!(
        got,
        ["REQ-X-007", "REQ-X-010", "REQ-X-011"],
        "the ranges expand wrongly"
    );
}

#[test]
fn qa_m019_gate_report_reads_a_one_id_range() {
    check_one_id_range(gate_report::gate_ids);
}

negative_control!(
    qa_m019_gate_report_reads_a_one_id_range,
    "a reader that refuses a range whose ends meet must fail",
    expected = "a one-id range is refused",
    check_one_id_range(|text, m| {
        if text.contains("007…007") {
            return Err("`REQ-X-007…007` is not a range of requirement ids".to_owned());
        }
        gate_report::gate_ids(text, m)
    })
);
