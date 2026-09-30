//! `cargo xtask reviews-check`, the `reviews-complete` check (R-175; REQ-SYS-066): all roles approved on the head
//! passes; an approval on an older commit, an APPROVE superseded by a later CHANGES and a missing role each fail
//! naming the role. An approval followed only by qa's own test commit counts on the head (R-260); one followed by a
//! qa-titled commit that modifies a file, adds outside qa's paths, or by any other commit fails naming the role; a title
//! naming no task passes with "no task, no named reviewers" (R-261; REQ-SYS-068).

use std::path::Path;

use validation::negative_control;
use xtask::reviews_check::{check, parse_reviews, reviewers, task_file, verdict, Pr, NO_TASK};

const HEAD: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

/// The problems reviews-check finds in the review-list fixture `name`, for roles code, qa and physics.
fn problems(name: &str) -> Vec<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("tests/fixtures/{name}.json"));
    let reviews = parse_reviews(&std::fs::read_to_string(path).expect("fixture read"))
        .expect("fixture parsed");
    let roles = ["code", "qa", "physics"].map(str::to_owned);
    check(&reviews, HEAD, &roles)
}

/// Exactly one problem, naming `role` and containing `why`.
fn fails_naming(name: &str, role: &str, why: &str) {
    let found = problems(name);
    assert!(
        found.len() == 1 && found[0].contains(&format!("role `{role}`")) && found[0].contains(why),
        "reviews-check did not fail naming `{role}`: {found:?}"
    );
}

#[test]
fn reviews_check_all_approved_on_head_passes() {
    let found = problems("reviews_all_approved");
    assert!(found.is_empty(), "all-approved fixture failed: {found:?}");
}

#[test]
fn reviews_check_approval_on_older_commit_fails_naming_the_role() {
    fails_naming(
        "reviews_stale_commit",
        "qa",
        "not approved on the head commit",
    );
}

#[test]
fn reviews_check_approve_superseded_by_changes_fails_naming_the_role() {
    fails_naming(
        "reviews_superseded",
        "physics",
        "superseded by a later `VERDICT: CHANGES physics`",
    );
}

/// `VERDICT: APPROVE physics-reviewer` is not physics's verdict.
#[test]
fn reviews_check_missing_role_fails_naming_it() {
    fails_naming(
        "reviews_missing_role",
        "physics",
        "no review headed `VERDICT: APPROVE physics`",
    );
}

/// The title names this task's file, whose Reviewers field gives the roles; a title naming no task fails.
#[test]
fn reviews_check_reads_roles_from_the_task_named_by_the_title() {
    let (id, file) = task_file("TASK-M0-36: `cargo xtask reviews-check`").expect("title parsed");
    assert_eq!(
        (id.as_str(), file.as_str()),
        ("TASK-M0-36", "plan/tasks/M0/TASK-M0-36.md")
    );
    let task = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(file))
        .expect("task file read");
    assert_eq!(
        reviewers(&task).expect("roles read"),
        ["code", "qa"],
        "roles misread"
    );
    assert!(
        task_file("R-254: a ruling").is_err(),
        "a title naming no task was accepted"
    );
}

/// `gh api --paginate` prints one array per page, back to back.
#[test]
fn reviews_check_joins_paginated_pages() {
    let pages = format!(
        r#"[{{"body":"VERDICT: APPROVE code","commit_id":"{HEAD}","submitted_at":"2026-09-29T10:00:00Z"}}]
[{{"body":"VERDICT: CHANGES code","commit_id":"{HEAD}","submitted_at":"2026-09-29T11:00:00Z"}}]"#
    );
    let reviews = parse_reviews(&pages).expect("pages parsed");
    assert_eq!(reviews.len(), 2, "pages not joined");
    assert_eq!(
        check(&reviews, HEAD, &["code".to_owned()]).len(),
        1,
        "the second page's CHANGES was not read"
    );
}

negative_control!(
    reviews_check_all_approved_on_head_passes,
    "the stale-commit fixture must fail the all-approved check",
    expected = "all-approved fixture failed",
    {
        let found = problems("reviews_stale_commit");
        assert!(found.is_empty(), "all-approved fixture failed: {found:?}");
    }
);

negative_control!(
    reviews_check_approval_on_older_commit_fails_naming_the_role,
    "every role approved on the head gives no stale role to name",
    expected = "reviews-check did not fail naming",
    fails_naming(
        "reviews_all_approved",
        "qa",
        "not approved on the head commit"
    )
);

negative_control!(
    reviews_check_approve_superseded_by_changes_fails_naming_the_role,
    "with no later CHANGES there is no superseded role to name",
    expected = "reviews-check did not fail naming",
    fails_naming("reviews_all_approved", "physics", "superseded by a later")
);

negative_control!(
    reviews_check_missing_role_fails_naming_it,
    "with physics approved there is no missing role to name",
    expected = "reviews-check did not fail naming",
    fails_naming("reviews_all_approved", "physics", "no review headed")
);

negative_control!(
    reviews_check_reads_roles_from_the_task_named_by_the_title,
    "a task file naming other roles must fail the roles check",
    expected = "roles misread",
    assert_eq!(
        reviewers("- **Reviewers:** code, qa, gui\n").expect("roles read"),
        ["code", "qa"],
        "roles misread"
    )
);

negative_control!(
    reviews_check_joins_paginated_pages,
    "a single page must fail the joined-pages check",
    expected = "pages not joined",
    {
        let reviews = parse_reviews(&format!(
            r#"[{{"body":"VERDICT: APPROVE code","commit_id":"{HEAD}"}}]"#
        ))
        .expect("page parsed");
        assert_eq!(reviews.len(), 2, "pages not joined");
    }
);

/// reviews-check's verdict on the PR fixture `name`, against the task files of this workspace.
fn pr_verdict(name: &str) -> Result<String, String> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = manifest.join(format!("tests/fixtures/{name}.json"));
    let pr: Pr = serde_json::from_str(&std::fs::read_to_string(path).expect("fixture read"))
        .expect("fixture parsed");
    verdict(&manifest.join(".."), &pr)
}

/// The PR fixture passes.
fn pr_passes(name: &str) {
    let found = pr_verdict(name);
    assert!(found.is_ok(), "PR fixture `{name}` failed: {found:?}");
}

/// The PR fixture fails naming role `code`, whose APPROVE is on the commit before the head, and not `qa`.
fn pr_fails_naming_code(name: &str) {
    let found = pr_verdict(name);
    assert!(
        found
            .as_ref()
            .is_err_and(|e| e.contains("role `code`") && !e.contains("role `qa`")),
        "reviews-check did not fail naming `code`: {found:?}"
    );
}

#[test]
fn reviews_check_approval_before_qa_test_commit_counts_on_head() {
    pr_passes("reviews_qa_commit");
}

#[test]
fn reviews_check_qa_titled_commit_modifying_a_file_fails_naming_the_role() {
    pr_fails_naming_code("reviews_qa_modifies");
}

#[test]
fn reviews_check_qa_titled_commit_adding_outside_qa_paths_fails_naming_the_role() {
    pr_fails_naming_code("reviews_qa_outside");
}

#[test]
fn reviews_check_other_commit_after_approval_fails_naming_the_role() {
    pr_fails_naming_code("reviews_other_commit");
}

#[test]
fn reviews_check_title_naming_no_task_passes() {
    let found = pr_verdict("reviews_no_task");
    assert!(
        found.as_ref().is_ok_and(|line| line.contains(NO_TASK)),
        "no-task title did not pass with `{NO_TASK}`: {found:?}"
    );
    assert_eq!(NO_TASK, "no task, no named reviewers");
}

negative_control!(
    reviews_check_approval_before_qa_test_commit_counts_on_head,
    "an approval followed by a non-qa commit must fail the qa-commit check",
    expected = "PR fixture `reviews_other_commit` failed",
    pr_passes("reviews_other_commit")
);

negative_control!(
    reviews_check_qa_titled_commit_modifying_a_file_fails_naming_the_role,
    "a qa commit adding only under qa's paths gives no role to name",
    expected = "reviews-check did not fail naming",
    pr_fails_naming_code("reviews_qa_commit")
);

negative_control!(
    reviews_check_qa_titled_commit_adding_outside_qa_paths_fails_naming_the_role,
    "a qa commit adding only under qa's paths gives no role to name",
    expected = "reviews-check did not fail naming",
    pr_fails_naming_code("reviews_qa_commit")
);

negative_control!(
    reviews_check_other_commit_after_approval_fails_naming_the_role,
    "a qa commit adding only under qa's paths gives no role to name",
    expected = "reviews-check did not fail naming",
    pr_fails_naming_code("reviews_qa_commit")
);

negative_control!(
    reviews_check_title_naming_no_task_passes,
    "a title naming a task prints the roles, not the no-task line",
    expected = "no-task title did not pass",
    {
        let found = pr_verdict("reviews_qa_commit");
        assert!(
            found.as_ref().is_ok_and(|line| line.contains(NO_TASK)),
            "no-task title did not pass with `{NO_TASK}`: {found:?}"
        );
    }
);
