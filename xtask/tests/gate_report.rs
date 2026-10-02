//! `cargo xtask gate-report` (REQ-SYS-067; R-177, R-186): on a fixture milestone file, the report names every
//! requirement of the milestone's gate block and every earlier one, with its pass/fail from the fixture results; it
//! fails when a requirement failed or has no result; a benchmark requirement is awaiting the human's run until its
//! `prin profile` file is supplied; a review-checklist requirement passes when its closing task's PR merged with its
//! reviewers' approvals, read from a fixture in place of `gh` (decided per R-369, RQ-201). And `cargo xtask bench`'s reading of `prin profile diff`'s exit code. Each test
//! registers the control that must make it fail (R-176).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use validation::negative_control;
use xtask::gate_report::{
    benchmark_ids, closing_tasks, gate_ids, outcomes, render, review_checklist_ids, review_outcome,
    run, Outcome, Recorded,
};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/gate_report")
}

fn read(path: &str) -> String {
    std::fs::read_to_string(fixture().join(path)).expect("fixture read")
}

fn results() -> BTreeMap<String, String> {
    serde_json::from_str(&read("results.json")).expect("fixture results")
}

/// M1's gate and M0's, in milestone order, ranges expanded.
const M1: [&str; 7] = [
    "REQ-TOOL-001",
    "REQ-TOOL-002",
    "REQ-TOOL-005",
    "REQ-SYS-067",
    "REQ-PAY-009",
    "REQ-PERF-004",
    "REQ-PERF-010",
];

/// The report for `milestone` names every id of `M1` once, each with its outcome from the fixture.
fn check_names_every_requirement(milestone: &str) {
    let ids = gate_ids(&read("plan/MILESTONES.md"), milestone).expect("gate ids");
    let benches = benchmark_ids(&read("plan/requirements.yaml"));
    let got = outcomes(&ids, &benches, &results(), None).expect("outcomes");
    let text = render(milestone, &got).unwrap_or_else(|text| text);
    for (id, want) in M1.iter().zip([
        "pass",
        "pass",
        "FAIL",
        "pass",
        "pass",
        "awaiting the human's run",
        "awaiting the human's run",
    ]) {
        assert!(
            text.lines()
                .any(|l| l.starts_with(&format!("{id}  {want}"))),
            "the report does not name {id} as {want}:\n{text}"
        );
    }
    assert_eq!(got.len(), M1.len(), "the report names extra requirements");
}

#[test]
fn gate_report_names_every_requirement() {
    check_names_every_requirement("M1");
}

negative_control!(
    gate_report_names_every_requirement,
    "the report of an earlier milestone lacks M1's requirements",
    expected = "the report does not name REQ-PAY-009",
    check_names_every_requirement("M0")
);

/// With `results`, M0's report fails, naming the missing or failed requirement.
fn check_fails(results: &BTreeMap<String, String>, named: &str) {
    let ids = gate_ids(&read("plan/MILESTONES.md"), "M0").expect("gate ids");
    let got = outcomes(&ids, &BTreeSet::new(), results, None).expect("outcomes");
    let text = render("M0", &got).expect_err("the report passed");
    assert!(
        text.contains(named),
        "the report does not say `{named}`:\n{text}"
    );
}

#[test]
fn gate_report_fails_on_a_missing_result() {
    let mut r = results();
    r.insert("REQ-TOOL-005".to_owned(), "pass".to_owned());
    r.remove("REQ-SYS-067");
    check_fails(&r, "REQ-SYS-067  MISSING");
}

negative_control!(
    gate_report_fails_on_a_missing_result,
    "a report with every result present and passing must pass",
    expected = "the report passed",
    {
        let mut r = results();
        r.insert("REQ-TOOL-005".to_owned(), "pass".to_owned());
        check_fails(&r, "MISSING")
    }
);

#[test]
fn gate_report_fails_on_a_failed_result() {
    check_fails(&results(), "REQ-TOOL-005  FAIL");
}

negative_control!(
    gate_report_fails_on_a_failed_result,
    "a report whose failed result passes must fail the check",
    expected = "the report passed",
    {
        let mut r = results();
        r.insert("REQ-TOOL-005".to_owned(), "pass".to_owned());
        check_fails(&r, "FAIL")
    }
);

/// A benchmark requirement is awaiting until its file is in `dir`, then supplied; awaiting doesn't fail the report.
fn check_bench(dir: &Path) {
    let ids = vec!["REQ-PERF-004".to_owned(), "REQ-PERF-010".to_owned()];
    let benches = benchmark_ids(&read("plan/requirements.yaml"));
    let got = outcomes(&ids, &benches, &BTreeMap::new(), Some(dir)).expect("outcomes");
    assert_eq!(
        got[0].1,
        Outcome::Supplied(dir.join("REQ-PERF-004.jsonl")),
        "a supplied prin profile file is not read"
    );
    assert_eq!(got[1].1, Outcome::Awaiting);
    assert!(render("M1", &got).is_ok(), "awaiting fails the report");
}

#[test]
fn gate_report_bench_awaits_the_humans_run() {
    check_bench(&fixture().join("bench"));
}

negative_control!(
    gate_report_bench_awaits_the_humans_run,
    "a directory without the file must leave it awaiting",
    expected = "a supplied prin profile file is not read",
    check_bench(&fixture())
);

/// A supplied file that is not profiler schema v1 is refused.
#[test]
fn gate_report_refuses_a_bench_file_not_v1() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("gate_report_not_v1");
    std::fs::create_dir_all(&dir).expect("dir");
    std::fs::write(dir.join("REQ-PERF-004.jsonl"), "{\"schema\":\"other\"}\n").expect("write");
    let benches = benchmark_ids(&read("plan/requirements.yaml"));
    let got = outcomes(
        &["REQ-PERF-004".to_owned()],
        &benches,
        &BTreeMap::new(),
        Some(&dir),
    );
    assert!(got.is_err(), "a file that is not schema v1 was accepted");
}

negative_control!(
    gate_report_refuses_a_bench_file_not_v1,
    "the fixture's schema v1 file must be accepted",
    expected = "a file that is not schema v1 was accepted",
    {
        let benches = benchmark_ids(&read("plan/requirements.yaml"));
        let dir = fixture().join("bench");
        let got = outcomes(
            &["REQ-PERF-004".to_owned()],
            &benches,
            &BTreeMap::new(),
            Some(&dir),
        );
        assert!(got.is_err(), "a file that is not schema v1 was accepted");
    }
);

/// A gate block whose stated count disagrees with its ids, or a milestone with no block, is refused.
fn check_refuses(milestones: &str, milestone: &str) {
    assert!(
        gate_ids(milestones, milestone).is_err(),
        "a miscounted or absent gate block was read"
    );
}

#[test]
fn gate_report_refuses_a_miscounted_block() {
    let text = read("plan/MILESTONES.md");
    check_refuses(&text.replace("SYS (1)", "SYS (2)"), "M0");
    check_refuses(&text.replace("4 requirements", "5 requirements"), "M0");
    check_refuses(&text, "M3");
    check_refuses(&text, "X1");
}

negative_control!(
    gate_report_refuses_a_miscounted_block,
    "the fixture's own block must be read",
    expected = "a miscounted or absent gate block was read",
    check_refuses(&read("plan/MILESTONES.md"), "M2")
);

/// `run` on a copy of the fixture workspace writes the report, and fails as the report does.
fn check_run(name: &str, results: &str) {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::create_dir_all(root.join("plan")).expect("dir");
    for f in ["plan/MILESTONES.md", "plan/requirements.yaml"] {
        std::fs::copy(fixture().join(f), root.join(f)).expect("copy");
    }
    let file = root.join("results.json");
    std::fs::write(&file, results).expect("write");
    let got = run(&root, "M0", &file, None, &Recorded::default());
    let report =
        std::fs::read_to_string(root.join("target/gate-report/M0.txt")).expect("no report written");
    assert!(
        report.contains("REQ-SYS-067"),
        "the written report lacks a requirement"
    );
    assert!(got.is_err(), "run passed a failed requirement");
}

#[test]
fn gate_report_run_writes_and_fails() {
    check_run("gate_report_run", &read("results.json"));
}

negative_control!(
    gate_report_run_writes_and_fails,
    "a passing gate must not fail",
    expected = "run passed a failed requirement",
    check_run(
        "gate_report_run_control",
        r#"{"REQ-TOOL-001":"pass","REQ-TOOL-002":"pass","REQ-TOOL-005":"pass","REQ-SYS-067":"pass"}"#
    )
);

/// `cargo xtask bench` reads `prin profile diff`'s exit: 0 and 1 (a rise, reported) run; 2 or a kill fails.
fn check_diff_outcome(outcome: fn(Option<i32>) -> Result<&'static str, String>) {
    assert!(
        outcome(Some(0)).is_ok() && outcome(Some(1)).is_ok(),
        "a diff that ran fails the bench"
    );
    assert!(
        outcome(Some(2)).is_err() && outcome(None).is_err(),
        "a diff that failed passes the bench"
    );
}

#[test]
fn bench_reads_the_diff_exit() {
    check_diff_outcome(xtask::bench::diff_outcome);
}

negative_control!(
    bench_reads_the_diff_exit,
    "a reader that passes every exit must fail",
    expected = "a diff that failed passes the bench",
    check_diff_outcome(|_| Ok("ran"))
);

/// The bench's trace and baseline paths: `target/bench/<bench>.jsonl`, `fixtures/bench/<bench>/baseline.json`.
#[test]
fn bench_paths() {
    let root = Path::new("/w");
    assert_eq!(
        xtask::bench::result_path(root, "b"),
        Path::new("/w/target/bench/b.jsonl")
    );
    assert_eq!(
        xtask::bench::baseline_path(root, "b"),
        Path::new("/w/fixtures/bench/b/baseline.json")
    );
}

negative_control!(
    bench_paths,
    "another bench's path must differ",
    expected = "assertion",
    assert_eq!(
        xtask::bench::result_path(Path::new("/w"), "c"),
        Path::new("/w/target/bench/b.jsonl")
    )
);

/// The review-checklist fixture workspace: tasks TASK-M0-90…93, their task files, and their PRs (`prs.json`).
fn review_root() -> PathBuf {
    fixture().join("review")
}

fn review_prs() -> Recorded {
    Recorded::from_json(&read("review/prs.json")).expect("fixture PRs")
}

/// `id`'s outcome on the review fixture, its PRs from `prs`.
fn review(id: &str, prs: &Recorded) -> Outcome {
    let closers = closing_tasks(&read("review/plan/tasks.yaml"));
    review_outcome(&review_root(), &closers, id, prs).expect("review outcome")
}

/// `id` passes, on PR `pr`, merged with its reviewers' approvals.
fn check_reviewed(id: &str, pr: u64, prs: &Recorded) {
    assert_eq!(
        review(id, prs),
        Outcome::Reviewed(pr),
        "the review-checklist requirement did not pass"
    );
}

#[test]
fn gate_report_review_checklist_passes_on_a_merged_approved_pr() {
    check_reviewed("REQ-VAL-901", 90, &review_prs());
}

negative_control!(
    gate_report_review_checklist_passes_on_a_merged_approved_pr,
    "a task with no PR must not pass",
    expected = "the review-checklist requirement did not pass",
    check_reviewed("REQ-VAL-901", 90, &Recorded::default())
);

/// `id` fails, the reason saying `why`.
fn check_unreviewed(id: &str, why: &str) {
    match review(id, &review_prs()) {
        Outcome::Unreviewed(reason) if reason.contains(why) => {}
        other => panic!("the requirement did not fail saying `{why}`: {other:?}"),
    }
}

#[test]
fn gate_report_review_checklist_fails_on_an_unmerged_pr() {
    check_unreviewed("REQ-VAL-902", "TASK-M0-91's PR #91 is not merged");
}

negative_control!(
    gate_report_review_checklist_fails_on_an_unmerged_pr,
    "a merged, approved PR must not be called unmerged",
    expected = "the requirement did not fail saying",
    check_unreviewed("REQ-VAL-901", "is not merged")
);

#[test]
fn gate_report_review_checklist_fails_on_a_missing_approval() {
    check_unreviewed("REQ-VAL-903", "role `qa` has not approved");
}

negative_control!(
    gate_report_review_checklist_fails_on_a_missing_approval,
    "a PR every named reviewer approved must not lack an approval",
    expected = "the requirement did not fail saying",
    check_unreviewed("REQ-VAL-901", "has not approved")
);

#[test]
fn gate_report_review_checklist_fails_with_no_pr_or_no_task() {
    check_unreviewed("REQ-VAL-904", "TASK-M0-93 has no PR");
    check_unreviewed("REQ-VAL-905", "no task in plan/tasks.yaml closes it");
}

negative_control!(
    gate_report_review_checklist_fails_with_no_pr_or_no_task,
    "a requirement whose task merged with its approvals has a PR and a task",
    expected = "the requirement did not fail saying",
    check_unreviewed("REQ-VAL-901", "has no PR")
);

/// The fixture's tasks.yaml reads each listed requirement to its task, a trailing comment ignored; its
/// requirements.yaml gives the five review-checklist ids.
#[test]
fn gate_report_reads_closing_tasks_and_checklist_ids() {
    let closers = closing_tasks(&read("review/plan/tasks.yaml"));
    assert_eq!(
        closers.get("REQ-VAL-902").map(String::as_str),
        Some("TASK-M0-91")
    );
    assert_eq!(
        closers.get("REQ-TOOL-001").map(String::as_str),
        Some("TASK-M0-92")
    );
    assert_eq!(closers.len(), 5, "tasks.yaml's requirements were misread");
    let ids = review_checklist_ids(&read("review/plan/requirements.yaml"));
    assert_eq!(ids.len(), 5, "the review-checklist ids were misread");
    assert!(!ids.contains("REQ-TOOL-001"));
}

negative_control!(
    gate_report_reads_closing_tasks_and_checklist_ids,
    "the gate fixture's tasks are not the review fixture's",
    expected = "tasks.yaml's requirements were misread",
    assert_eq!(
        closing_tasks("- id: TASK-M0-90\n  requirements: [A, B]\n").len(),
        5,
        "tasks.yaml's requirements were misread"
    )
);

/// `run` on a copy of the review fixture, PRs from `prs`: the report gives REQ-VAL-901 its PR, fails REQ-VAL-902, and
/// the run fails.
fn check_run_reviewed(name: &str, prs: &Recorded) {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::create_dir_all(root.join("plan/tasks/M0")).expect("dir");
    for f in [
        "plan/MILESTONES.md",
        "plan/requirements.yaml",
        "plan/tasks.yaml",
        "plan/tasks/M0/TASK-M0-90.md",
        "plan/tasks/M0/TASK-M0-91.md",
        "plan/tasks/M0/TASK-M0-92.md",
        "plan/tasks/M0/TASK-M0-93.md",
    ] {
        std::fs::copy(review_root().join(f), root.join(f)).expect("copy");
    }
    let file = root.join("results.json");
    std::fs::write(&file, r#"{"REQ-TOOL-001":"pass"}"#).expect("write");
    let got = run(&root, "M0", &file, None, prs);
    let report =
        std::fs::read_to_string(root.join("target/gate-report/M0.txt")).expect("no report written");
    assert!(
        report.contains("REQ-VAL-901  pass: PR #90 merged"),
        "the report does not pass REQ-VAL-901 on its merged PR:\n{report}"
    );
    assert!(report.contains("REQ-VAL-902  FAIL"), "{report}");
    assert!(got.is_err(), "run passed an unreviewed requirement");
}

#[test]
fn gate_report_run_reads_review_checklist_prs() {
    check_run_reviewed("gate_report_run_review", &review_prs());
}

negative_control!(
    gate_report_run_reads_review_checklist_prs,
    "with no PRs, no review-checklist requirement passes",
    expected = "the report does not pass REQ-VAL-901",
    check_run_reviewed("gate_report_run_review_control", &Recorded::default())
);
