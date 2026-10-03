//! `cargo xtask gate-report` (REQ-SYS-067; R-177, R-186): on a fixture milestone file, the report names every
//! requirement of the milestone's gate block and every earlier one, with its pass/fail from the fixture results; it
//! fails when a requirement failed or has no result; a benchmark requirement is awaiting the human's run until its
//! `prin profile` file is supplied; a review-checklist requirement passes when its closing task's PR merged with its
//! reviewers' approvals, or, for a task closed by a ruling, when that ruling's PR merged, no approval asked of it, read
//! from a fixture in place of `gh` (decided per R-369, RQ-201). A unit-test, property-test, numerical-gate, golden or
//! screenshot requirement fails when a `cargo test` command its detail names matches no test of its build in the gate
//! run's own listing, and one naming none is marked (REQ-SYS-079; the human's instruction, 3 Oct 2026: "fix the gate
//! report to check tests exist"). And `cargo xtask bench`'s reading of `prin profile diff`'s exit code. Each test
//! registers the control that must make it fail (R-176).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use validation::negative_control;
use xtask::gate_report::{
    benchmark_ids, check_named_tests, closing_rulings, closing_tasks, gate_ids, named_tests,
    names_ruling, outcomes, render, review_checklist_ids, review_outcome, run, run_listed,
    test_builds, title_rulings, verifies, NamedTest, Outcome, PrSource, Recorded, TestList,
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

/// The review-checklist fixture workspace: tasks TASK-M0-90…97, their task files, and their PRs and its rulings' PRs
/// (`prs.json`).
fn review_root() -> PathBuf {
    fixture().join("review")
}

fn review_prs() -> Recorded {
    Recorded::from_json(&read("review/prs.json")).expect("fixture PRs")
}

/// `id`'s outcome on the review fixture, its PRs from `prs`.
fn review(id: &str, prs: &Recorded) -> Outcome {
    let tasks = read("review/plan/tasks.yaml");
    review_outcome(
        &review_root(),
        &closing_tasks(&tasks),
        &closing_rulings(&tasks),
        id,
        prs,
    )
    .expect("review outcome")
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

/// `id` passes on ruling `ruling`'s PR `pr`, merged.
fn check_ruled(id: &str, pr: u64, ruling: u32, prs: &Recorded) {
    assert_eq!(
        review(id, prs),
        Outcome::Ruled(pr, ruling),
        "the review-checklist requirement did not pass"
    );
}

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
/// requirements.yaml gives the nine review-checklist ids.
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
    assert_eq!(closers.len(), 9, "tasks.yaml's requirements were misread");
    let ids = review_checklist_ids(&read("review/plan/requirements.yaml"));
    assert_eq!(ids.len(), 9, "the review-checklist ids were misread");
    assert!(!ids.contains("REQ-TOOL-001"));
}

negative_control!(
    gate_report_reads_closing_tasks_and_checklist_ids,
    "the gate fixture's tasks are not the review fixture's",
    expected = "tasks.yaml's requirements were misread",
    assert_eq!(
        closing_tasks("- id: TASK-M0-90\n  requirements: [A, B]\n").len(),
        9,
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
        "plan/tasks/M0/TASK-M0-94.md",
        "plan/tasks/M0/TASK-M0-95.md",
        "plan/tasks/M0/TASK-M0-96.md",
        "plan/tasks/M0/TASK-M0-97.md",
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
    assert!(
        report.contains("REQ-VAL-906  pass: PR #101 merged, closing it by ruling R-901"),
        "the report does not pass REQ-VAL-906 on its ruling's merged PR:\n{report}"
    );
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

/// A task done and closed by a ruling (TASK-M0-94, R-901) passes on that ruling's merged PR, #101, titled
/// `R-900, R-901: …`, merged (its approvals are not read).
#[test]
fn gate_report_review_checklist_passes_on_a_closing_rulings_pr() {
    check_ruled("REQ-VAL-906", 101, 901, &review_prs());
}

negative_control!(
    gate_report_review_checklist_passes_on_a_closing_rulings_pr,
    "a ruling with no PR must not pass the task it closes",
    expected = "the review-checklist requirement did not pass",
    check_ruled("REQ-VAL-906", 101, 901, &Recorded::default())
);

#[test]
fn gate_report_review_checklist_fails_on_an_unmerged_rulings_pr() {
    check_unreviewed(
        "REQ-VAL-907",
        "TASK-M0-95's ruling R-902's PR #102 is not merged",
    );
}

negative_control!(
    gate_report_review_checklist_fails_on_an_unmerged_rulings_pr,
    "a ruling's merged, approved PR must not be called unmerged",
    expected = "the requirement did not fail saying",
    check_unreviewed("REQ-VAL-906", "is not merged")
);

/// R-904's PR, #103, is titled `R-903 to R-905: …` and merged with no reviews, though TASK-M0-96 names code and qa:
/// the ruling is the human's own decision, and so its approval.
#[test]
fn gate_report_review_checklist_passes_on_a_merged_rulings_pr_with_no_reviews() {
    check_ruled("REQ-VAL-908", 103, 904, &review_prs());
}

negative_control!(
    gate_report_review_checklist_passes_on_a_merged_rulings_pr_with_no_reviews,
    "a ruling's PR with no reviews must not pass the task it closes until it is merged",
    expected = "the review-checklist requirement did not pass",
    check_ruled(
        "REQ-VAL-908",
        103,
        904,
        &Recorded::from_json(
            r#"{"rulings": [{"merged": false, "number": 103, "title": "R-903 to R-905: open", "head": "h103"}]}"#
        )
        .expect("control PRs")
    )
);

/// A task whose status comment names a ruling but is not `done` (TASK-M0-97, R-906) reads its own PRs, and has none,
/// though R-906's PR merged with code's approval.
#[test]
fn gate_report_review_checklist_reads_a_ruling_only_for_a_done_task() {
    check_unreviewed("REQ-VAL-909", "TASK-M0-97 has no PR");
}

negative_control!(
    gate_report_review_checklist_reads_a_ruling_only_for_a_done_task,
    "a done task closed by a ruling reads that ruling's PR, not its own",
    expected = "the requirement did not fail saying",
    check_unreviewed("REQ-VAL-907", "TASK-M0-95 has no PR")
);

/// tasks.yaml's done tasks with `closed by R-<n>` map to their rulings; the not-done TASK-M0-97 does not.
#[test]
fn gate_report_reads_closing_rulings() {
    let rulings = closing_rulings(&read("review/plan/tasks.yaml"));
    let want: Vec<(String, u32)> = [
        ("TASK-M0-94", 901),
        ("TASK-M0-95", 902),
        ("TASK-M0-96", 904),
    ]
    .map(|(t, n)| (t.to_owned(), n))
    .to_vec();
    assert_eq!(
        rulings.into_iter().collect::<Vec<_>>(),
        want,
        "tasks.yaml's closing rulings were misread"
    );
}

negative_control!(
    gate_report_reads_closing_rulings,
    "a task not done names no closing ruling",
    expected = "tasks.yaml's closing rulings were misread",
    assert_eq!(
        closing_rulings("- id: TASK-M0-97\n  status: todo  # closed by R-906\n").len(),
        1,
        "tasks.yaml's closing rulings were misread"
    )
);

/// The title forms the repository's ruling PRs use name their rulings; a title whose prefix is anything else names
/// none.
fn check_title_rulings(title: &str, want: &[(u32, u32)]) {
    assert_eq!(
        title_rulings(title),
        want,
        "the rulings named by `{title}` were misread"
    );
}

#[test]
fn gate_report_reads_ruling_pr_titles() {
    check_title_rulings("R-185: crate map confirmed", &[(185, 185)]);
    check_title_rulings("R-366 to R-372: autonomy rule", &[(366, 372)]);
    check_title_rulings("R-348–R-352: mutants caps", &[(348, 352)]);
    check_title_rulings("R-268..R-277: the human's rulings", &[(268, 277)]);
    check_title_rulings(
        "R-353, R-354, R-355: veto items",
        &[(353, 353), (354, 354), (355, 355)],
    );
    check_title_rulings("R-209/R-210: split", &[(209, 209), (210, 210)]);
    check_title_rulings("R-271 follow-ups: d_min", &[]);
    check_title_rulings("TASK-M0-19: R-185", &[]);
    check_title_rulings("R-185 no colon", &[]);
    assert!(names_ruling("R-366 to R-372: x", 369));
    assert!(!names_ruling("R-366 to R-372: x", 373));
    assert!(!names_ruling("R-1850: x", 185));
}

negative_control!(
    gate_report_reads_ruling_pr_titles,
    "a follow-ups title names no ruling",
    expected = "were misread",
    check_title_rulings("R-271 follow-ups: d_min", &[(271, 271)])
);

/// The fixture source lists a ruling's PRs from any key, by title, each once.
#[test]
fn gate_report_recorded_lists_a_rulings_prs() {
    let numbers = |n: u32| -> Vec<u64> {
        review_prs()
            .ruling_prs(n)
            .expect("ruling PRs")
            .iter()
            .map(|p| p.pr.number)
            .collect()
    };
    assert_eq!(numbers(904), [103], "R-904's PRs were misread");
    assert_eq!(numbers(901), [101], "R-901's PRs were misread");
}

negative_control!(
    gate_report_recorded_lists_a_rulings_prs,
    "R-907 has no PR",
    expected = "R-907's PRs were misread",
    assert_eq!(
        review_prs()
            .ruling_prs(907)
            .expect("ruling PRs")
            .iter()
            .map(|p| p.pr.number)
            .collect::<Vec<_>>(),
        [103],
        "R-907's PRs were misread"
    )
);

/// A ruling's number is digits only: `u32`'s parser alone would take `R-+5` as R-5.
#[test]
fn gate_report_reads_ruling_numbers_as_digits_only() {
    check_title_rulings("R-+5: signed", &[]);
    check_title_rulings("R-: none", &[]);
}

negative_control!(
    gate_report_reads_ruling_numbers_as_digits_only,
    "R-5 names a ruling",
    expected = "were misread",
    check_title_rulings("R-5: unsigned", &[])
);

// --- The `gh` source, through a stand-in `gh` ----------------------------------------------------------------------

/// A stand-in `gh` in directory `case` answering, as GitHub does, from `prs` (in `prs.json`'s form): `gh pr list` lists
/// each PR's number, title and state; `gh api …/pulls/N` gives PR N's title and head, `…/pulls/N/reviews` its
/// reviews. With `fail`, every call fails.
#[cfg(unix)]
fn stand_in_gh(case: &str, prs: &serde_json::Value, fail: bool) -> PathBuf {
    use serde_json::json;
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("gate_report_gh_{case}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("stand-in dir made");
    let all: Vec<&serde_json::Value> = prs
        .as_object()
        .expect("PRs by key")
        .values()
        .flat_map(|list| list.as_array().expect("a list of PRs"))
        .collect();
    let list: Vec<serde_json::Value> = all
        .iter()
        .map(|p| {
            let state = if p["merged"] == true {
                "MERGED"
            } else {
                "CLOSED"
            };
            json!({"number": p["number"], "title": p["title"], "state": state})
        })
        .collect();
    let write = |name: String, v: serde_json::Value| {
        std::fs::write(dir.join(name), v.to_string()).expect("stand-in answer written")
    };
    write("list.json".to_owned(), serde_json::Value::Array(list));
    for p in &all {
        let n = &p["number"];
        write(
            format!("pull_{n}.json"),
            json!({"number": n, "title": p["title"], "head": {"sha": p["head"]}}),
        );
        write(format!("reviews_{n}.json"), p["reviews"].clone());
    }
    let gh = dir.join("gh");
    let script = if fail {
        "#!/bin/sh\necho 'gh: not logged in' >&2\nexit 1\n".to_owned()
    } else {
        format!(
            r#"#!/bin/sh
if [ "$1 $2" = "pr list" ]; then cat '{dir}/list.json'; exit; fi
case "$2" in
  */reviews) n=${{2%/reviews}}; cat "{dir}/reviews_${{n##*/}}.json" ;;
  */pulls/*) cat "{dir}/pull_${{2##*/}}.json" ;;
  *) echo "unexpected gh $*" >&2; exit 1 ;;
esac
"#,
            dir = dir.display()
        )
    };
    validation::spawn::write_executable(&gh, script).expect("stand-in gh written");
    gh
}

/// Runs `spawn`, which spawns the stand-in `gh` from xtask's own code and so not through `validation::spawn::Spawn`,
/// while no other test writes a stand-in: a child forked as another thread holds an executable open for writing
/// inherits that descriptor until it execs, and an exec of that executable meanwhile fails with ETXTBSY (REQ-SYS-070).
#[cfg(unix)]
fn no_write<T>(spawn: impl FnOnce() -> T) -> T {
    validation::spawn::while_no_spawn(spawn)
}

/// Through `gh` (a stand-in serving `prs`), each review-checklist requirement of the review fixture gets the outcome
/// it gets from the recorded PRs: a task's merged, approved PR passes it; an unmerged one, or one lacking an approval,
/// fails it; a ruling's merged PR passes the task it closes, and its unmerged one fails it.
#[cfg(unix)]
fn check_gh_outcomes(case: &str, prs: &serde_json::Value) {
    let gh = xtask::gate_report::Gh::new(stand_in_gh(case, prs, false));
    let tasks = read("review/plan/tasks.yaml");
    let outcome = |id: &str| {
        no_write(|| {
            review_outcome(
                &review_root(),
                &closing_tasks(&tasks),
                &closing_rulings(&tasks),
                id,
                &gh,
            )
        })
        .expect("review outcome through gh")
    };
    assert_eq!(
        outcome("REQ-VAL-901"),
        Outcome::Reviewed(90),
        "through gh, the task's merged, approved PR did not pass it"
    );
    assert_eq!(outcome("REQ-VAL-906"), Outcome::Ruled(101, 901));
    for (id, why) in [
        ("REQ-VAL-902", "TASK-M0-91's PR #91 is not merged"),
        ("REQ-VAL-903", "role `qa` has not approved"),
        (
            "REQ-VAL-907",
            "TASK-M0-95's ruling R-902's PR #102 is not merged",
        ),
    ] {
        match outcome(id) {
            Outcome::Unreviewed(reason) if reason.contains(why) => {}
            other => panic!("{id} did not fail saying `{why}`: {other:?}"),
        }
    }
}

#[cfg(unix)]
#[test]
fn gate_report_gh_reads_task_and_ruling_prs() {
    let prs: serde_json::Value = serde_json::from_str(&read("review/prs.json")).expect("fixture");
    check_gh_outcomes("reads", &prs);
}

#[cfg(unix)]
negative_control!(
    gate_report_gh_reads_task_and_ruling_prs,
    "gh lists no PR",
    expected = "through gh, the task's merged, approved PR did not pass it",
    check_gh_outcomes("reads_control", &serde_json::json!({}))
);

/// A `gh` that fails fails the requirement's lookup, naming `gh pr list`.
#[cfg(unix)]
fn check_gh_fails(case: &str, fail: bool) {
    let gh = xtask::gate_report::Gh::new(stand_in_gh(case, &serde_json::json!({}), fail));
    let tasks = read("review/plan/tasks.yaml");
    let got = no_write(|| {
        review_outcome(
            &review_root(),
            &closing_tasks(&tasks),
            &closing_rulings(&tasks),
            "REQ-VAL-901",
            &gh,
        )
    });
    assert!(
        got.as_ref()
            .is_err_and(|e| e.contains("`gh pr list` failed: gh: not logged in")),
        "a failing gh did not fail the lookup: {got:?}"
    );
}

#[cfg(unix)]
#[test]
fn gate_report_gh_failing_fails_the_lookup() {
    check_gh_fails("fails", true);
}

#[cfg(unix)]
negative_control!(
    gate_report_gh_failing_fails_the_lookup,
    "a gh that answers",
    expected = "a failing gh did not fail the lookup",
    check_gh_fails("fails_control", false)
);

// --- Named tests (REQ-SYS-079; TASK-M0-53) -------------------------------------------------------------------------

/// The named-test fixture's listing `name` (`tests.txt`, or `tests_controls.txt`, which adds the controls build and
/// the tests `tests.txt` lacks).
fn listing(name: &str) -> TestList {
    TestList::parse(&read(&format!("named/{name}")))
}

/// `id`'s outcome from the named-test fixture, its suites having passed (or failed, `pass` false), checked against
/// `list`.
fn named_outcome(id: &str, pass: bool, list: &TestList) -> Outcome {
    let verifies = verifies(&read("named/plan/requirements.yaml"));
    let suite = if pass { Outcome::Pass } else { Outcome::Fail };
    let mut got = vec![(id.to_owned(), suite)];
    check_named_tests(&mut got, &verifies, list);
    got.remove(0).1
}

/// `id`'s outcome is a [`Outcome::NoTest`] whose reasons each say "no test matches" and together name every one of
/// `named` (crate and filter).
fn check_no_test(id: &str, list: &TestList, named: &[(&str, &str)]) {
    match named_outcome(id, true, list) {
        Outcome::NoTest(why) => {
            assert_eq!(
                why.len(),
                named.len(),
                "{id}: not one reason per missing command: {why:?}"
            );
            for (reason, (krate, filter)) in why.iter().zip(named) {
                for part in [
                    "no test matches",
                    &format!("crate `{krate}`"),
                    &format!("filter `{filter}`"),
                ] {
                    assert!(
                        reason.contains(part),
                        "{id}: the reason does not say {part}: {reason}"
                    );
                }
            }
        }
        other => panic!("{id} was not failed for a missing named test: {other:?}"),
    }
}

/// A named test that exists passes its requirement.
fn check_exists_passes(list: &TestList) {
    assert_eq!(
        named_outcome("REQ-NAM-001", true, list),
        Outcome::Pass,
        "a requirement whose named test exists did not pass"
    );
    assert_eq!(named_outcome("REQ-NAM-001", false, list), Outcome::Fail);
}

#[test]
fn gate_report_named_test_exists_passes() {
    check_exists_passes(&listing("tests.txt"));
}

negative_control!(
    gate_report_named_test_exists_passes,
    "a listing that lacks the named test",
    expected = "a requirement whose named test exists did not pass",
    check_exists_passes(&TestList::parse("== -p xtask\ngate::tests::first: test\n"))
);

/// A filter that matches nothing fails its requirement with the new outcome, naming the crate, the filter and "no
/// test matches", and the report fails, the line saying so.
fn check_matching_nothing(list: &TestList) {
    check_no_test("REQ-NAM-002", list, &[("xtask", "no_such_test")]);
    match named_outcome("REQ-NAM-002", true, list) {
        Outcome::NoTest(why) => assert!(
            why[0].contains("(`cargo test -p xtask no_such_test`;"),
            "the reason does not give the command: {why:?}"
        ),
        other => panic!("REQ-NAM-002 was not failed for a missing named test: {other:?}"),
    }
    let got = vec![(
        "REQ-NAM-002".to_owned(),
        named_outcome("REQ-NAM-002", true, list),
    )];
    let text = render("M0", &got).expect_err("a missing named test passed the report");
    assert!(
        text.contains("REQ-NAM-002  FAIL: no test matches: crate `xtask`, filter `no_such_test`"),
        "the report's line does not name the missing test:\n{text}"
    );
}

#[test]
fn gate_report_named_test_matching_nothing_fails() {
    check_matching_nothing(&listing("tests.txt"));
}

negative_control!(
    gate_report_named_test_matching_nothing_fails,
    "a listing in which the filter matches a test",
    expected = "REQ-NAM-002 was not failed for a missing named test",
    check_matching_nothing(&listing("tests_controls.txt"))
);

/// A test that exists only under `--features controls` fails, while the listing lacks that build: named with the
/// feature (the listing has no such build) and named without it (the default build lacks the test).
fn check_only_under_controls(list: &TestList) {
    check_no_test(
        "REQ-NAM-003",
        list,
        &[("validation", "scratch_guard_keeps_a_wrong_message")],
    );
    check_no_test(
        "REQ-NAM-004",
        list,
        &[("validation", "scratch_guard_keeps_a_wrong_message")],
    );
    match named_outcome("REQ-NAM-003", true, list) {
        Outcome::NoTest(why) => assert!(
            why[0].contains("the listing has no build `-p validation --features controls`"),
            "the reason does not say the controls build is not listed: {why:?}"
        ),
        other => panic!("REQ-NAM-003 passed: {other:?}"),
    }
}

#[test]
fn gate_report_named_test_only_under_controls_fails() {
    check_only_under_controls(&listing("tests.txt"));
}

negative_control!(
    gate_report_named_test_only_under_controls_fails,
    "a listing that holds the controls build",
    expected = "REQ-NAM-003 was not failed for a missing named test",
    check_only_under_controls(&listing("tests_controls.txt"))
);

/// A detail naming several commands passes only when every one matches: both matching passes; the second matching
/// nothing fails, naming only the second.
fn check_every_command(list: &TestList) {
    assert_eq!(
        named_outcome("REQ-NAM-005", true, list),
        Outcome::Pass,
        "a detail whose every named test exists did not pass"
    );
    check_no_test("REQ-NAM-006", list, &[("xtask", "lint_wgsl_finite_max")]);
}

#[test]
fn gate_report_named_test_every_command_must_match() {
    check_every_command(&listing("tests.txt"));
}

negative_control!(
    gate_report_named_test_every_command_must_match,
    "a listing in which the second command matches too",
    expected = "REQ-NAM-006 was not failed for a missing named test",
    check_every_command(&listing("tests_controls.txt"))
);

/// A hosted requirement whose detail names no command keeps its suites' result, marked "no named test to check", a
/// failed one still failing the report; a review-checklist requirement is not checked at all.
fn check_unnamed(id: &str) {
    let list = listing("tests.txt");
    assert_eq!(
        named_outcome(id, true, &list),
        Outcome::PassUnnamed,
        "a requirement naming no test is not marked"
    );
    let got = vec![(id.to_owned(), named_outcome(id, true, &list))];
    let text = render("M0", &got).expect("an unnamed passing requirement fails the report");
    assert!(
        text.contains(&format!("{id}  pass (no named test to check)")),
        "the report does not mark it:\n{text}"
    );
    let got = vec![(id.to_owned(), named_outcome(id, false, &list))];
    assert_eq!(got[0].1, Outcome::FailUnnamed);
    assert!(
        render("M0", &got).is_err(),
        "an unnamed failed requirement passes the report"
    );
    assert_eq!(
        named_outcome("REQ-NAM-008", true, &list),
        Outcome::Pass,
        "a review-checklist requirement is checked for named tests"
    );
}

#[test]
fn gate_report_named_test_unnamed_is_marked() {
    check_unnamed("REQ-NAM-007");
}

negative_control!(
    gate_report_named_test_unnamed_is_marked,
    "a requirement that names a test",
    expected = "a requirement naming no test is not marked",
    check_unnamed("REQ-NAM-001")
);

fn named(krate: &str, flags: &[&str], filter: &str, exact: bool) -> NamedTest {
    NamedTest {
        krate: krate.to_owned(),
        flags: flags.iter().map(|f| (*f).to_owned()).collect(),
        filter: filter.to_owned(),
        exact,
    }
}

/// A detail's commands are read as cargo reads them: package, flags, filter, `-- --exact`; a backticked command ends
/// at its backtick, a prose one at its filter or at prose punctuation; one with no package or no filter names none.
fn check_parses(detail: &str, want: &[NamedTest]) {
    assert_eq!(
        named_tests(detail),
        want,
        "the detail was parsed wrongly: {detail}"
    );
}

#[test]
fn gate_report_named_test_parses_detail() {
    check_parses(
        "cargo test -p xtask scratch_guard, cargo test -p prin scratch_guard and cargo test -p validation \
         --features controls scratch_guard_keeps: each",
        &[
            named("xtask", &[], "scratch_guard", false),
            named("prin", &[], "scratch_guard", false),
            named("validation", &["--features", "controls"], "scratch_guard_keeps", false),
        ],
    );
    check_parses(
        "`cargo test -p xtask` passes; (or `cargo test --features controls`); `cargo nextest run -p xtask \
         controls_partition`; `cargo test -p engine session::tests::header -- --exact` too",
        &[
            named("xtask", &[], "controls_partition", false),
            named("engine", &[], "session::tests::header", true),
        ],
    );
    check_parses(
        "cargo testing -p xtask golden; cargo test -p xtask, then",
        &[],
    );
    check_parses(
        "`cargo test -p xtask golden-run` and `cargo test -p xtask golden.`, then",
        &[named("xtask", &[], "golden", false)],
    );
    check_parses(
        "`cargo test -p engine -- --exact first second`",
        &[named("engine", &[], "first", true)],
    );
}

negative_control!(
    gate_report_named_test_parses_detail,
    "a detail whose command names a package and a filter",
    expected = "the detail was parsed wrongly",
    check_parses("`cargo test -p xtask golden` passes", &[])
);

/// The builds a gate's hosted requirements name, each once: the review checklist's are not among them.
fn check_builds(ids: &[String]) {
    let verifies = verifies(&read("named/plan/requirements.yaml"));
    assert_eq!(
        test_builds(ids, &verifies),
        [
            "-p validation",
            "-p validation --features controls",
            "-p xtask"
        ],
        "the gate's builds are not every hosted requirement's, once each"
    );
}

#[test]
fn gate_report_named_test_builds() {
    check_builds(&gate_ids(&read("named/plan/MILESTONES.md"), "M0").expect("gate ids"));
}

negative_control!(
    gate_report_named_test_builds,
    "a gate without the controls requirement",
    expected = "the gate's builds are not every hosted requirement's",
    check_builds(&[
        "REQ-NAM-001".to_owned(),
        "REQ-NAM-004".to_owned(),
        "REQ-NAM-008".to_owned()
    ])
);

/// A scratch copy of the named-test fixture's workspace, under `name`.
fn named_root(name: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("plan")).expect("dir");
    for f in ["plan/MILESTONES.md", "plan/requirements.yaml"] {
        std::fs::copy(fixture().join("named").join(f), root.join(f)).expect("copy");
    }
    root
}

/// `run_listed` with the listing `list` (none when `None`) writes a report that fails, naming the missing test, passes
/// the existing one and marks the unnamed one.
fn check_run_listed(name: &str, list: Option<&str>) {
    let root = named_root(name);
    let list = list.map(|l| fixture().join("named").join(l));
    let results = fixture().join("named/results.json");
    let got = run_listed(
        &root,
        "M0",
        &results,
        None,
        list.as_deref(),
        &Recorded::default(),
    );
    let report =
        std::fs::read_to_string(root.join("target/gate-report/M0.txt")).expect("no report written");
    assert!(
        report.contains("REQ-NAM-002  FAIL: no test matches: crate `xtask`, filter `no_such_test`"),
        "the report does not fail REQ-NAM-002 for its missing test:\n{report}"
    );
    assert!(
        got.is_err(),
        "run passed a requirement whose named test is missing"
    );
    assert!(
        report.contains("REQ-NAM-007  pass (no named test to check)"),
        "the report does not mark the unnamed requirement:\n{report}"
    );
    if list.is_some() {
        assert!(
            report.contains("REQ-NAM-001  pass\n"),
            "the report does not pass the requirement whose test exists:\n{report}"
        );
    } else {
        assert!(
            report.contains("REQ-NAM-001  FAIL: no test matches")
                && report.contains("no test listing was given (--test-list)"),
            "with no listing, a named test is not missing:\n{report}"
        );
    }
}

#[test]
fn gate_report_named_test_run_fails_the_report() {
    check_run_listed("gate_report_named_run", Some("tests.txt"));
    check_run_listed("gate_report_named_run_unlisted", None);
}

negative_control!(
    gate_report_named_test_run_fails_the_report,
    "a listing that holds REQ-NAM-002's test",
    expected = "the report does not fail REQ-NAM-002 for its missing test",
    check_run_listed("gate_report_named_run_control", Some("tests_controls.txt"))
);

/// A stand-in `cargo` in `root` answering `test <build> --no-fail-fast -- --list` as libtest lists, from `answers`
/// (build, listing); any other build fails as cargo does.
#[cfg(unix)]
fn stand_in_cargo(root: &Path, answers: &[(&str, &str)]) -> PathBuf {
    let mut script = "#!/bin/sh\ncase \"$*\" in\n".to_owned();
    for (build, list) in answers {
        script.push_str(&format!(
            "  \"test {build} --no-fail-fast -- --list\") printf '{list}' ;;\n"
        ));
    }
    script.push_str("  *) echo \"error: cargo $*\" >&2; exit 101 ;;\nesac\n");
    let cargo = root.join("cargo");
    validation::spawn::write_executable(&cargo, script).expect("stand-in cargo written");
    cargo
}

/// `write_test_list` writes each build's `-- --list` output under its build, recording a build whose listing failed,
/// and the report reads it back: the test listed is found, the failed build's test is missing, saying it failed.
#[cfg(unix)]
fn check_writes_listing(name: &str, xtask_list: &str) {
    let root = named_root(name);
    let cargo = stand_in_cargo(
        &root,
        &[
            ("-p xtask", xtask_list),
            // No newline at the end: the next build's header must still start its own line.
            ("-p validation", "scratch_guard_deletes_on_pass: test"),
        ],
    );
    let out = root.join("out/tests.txt");
    no_write(|| xtask::gate_report::write_test_list(&root, "M0", cargo.as_os_str(), &out))
        .expect("the listing is written");
    let list = TestList::parse(&std::fs::read_to_string(&out).expect("listing read"));
    assert_eq!(
        named_outcome("REQ-NAM-001", true, &list),
        Outcome::Pass,
        "the written listing lacks cargo's listed test"
    );
    match named_outcome("REQ-NAM-003", true, &list) {
        Outcome::NoTest(why) => assert!(
            why[0].contains("the listing of `-p validation --features controls` failed"),
            "a failed build's listing is not recorded as failed: {why:?}"
        ),
        other => panic!("a test of a failed build passed: {other:?}"),
    }
}

#[cfg(unix)]
#[test]
fn gate_report_named_test_list_written_from_cargo() {
    check_writes_listing(
        "gate_report_named_write",
        "gate::tests::first: test\\nlint_wgsl_unset_isinf_fails: test\\n\\n2 tests, 0 benchmarks\\n",
    );
}

#[cfg(unix)]
negative_control!(
    gate_report_named_test_list_written_from_cargo,
    "a cargo that lists no test of xtask",
    expected = "the written listing lacks cargo's listed test",
    check_writes_listing(
        "gate_report_named_write_control",
        "\\n0 tests, 0 benchmarks\\n"
    )
);

/// A command's filter matches a test as libtest matches it: a substring of the path name, or with `-- --exact` the
/// whole of it; and the command is given back as cargo runs it.
fn check_matches(exact: bool) {
    let test = named(
        "engine",
        &["--features", "controls"],
        "session::tests::header",
        exact,
    );
    assert!(
        test.matches("session::tests::header"),
        "the filter does not match its own name"
    );
    assert_eq!(
        test.matches("session::tests::header_fields"),
        !exact,
        "an exact filter matched a longer name, or a substring filter did not"
    );
    assert!(
        !test.matches("session::tests::head"),
        "a shorter name matched"
    );
    let tail = if exact { " -- --exact" } else { "" };
    assert_eq!(
        test.command(),
        format!("cargo test -p engine --features controls session::tests::header{tail}"),
        "the command is not given back as cargo runs it"
    );
}

#[test]
fn gate_report_named_test_matches_by_cargos_rule() {
    check_matches(true);
    check_matches(false);
}

negative_control!(
    gate_report_named_test_matches_by_cargos_rule,
    "an exact filter checked as a substring one",
    expected = "an exact filter matched a longer name, or a substring filter did not",
    {
        let test = named("engine", &[], "session::tests::header", false);
        assert!(
            !test.matches("session::tests::header_fields"),
            "an exact filter matched a longer name, or a substring filter did not"
        );
    }
);
