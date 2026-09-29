//! QA tests for TASK-M0-36, written from REQ-SYS-066 and R-175 — "every reviewer role a task file names must record
//! its verdict as a PR review headed `VERDICT: APPROVE <role>` or `VERDICT: CHANGES <role>`, and the
//! `reviews-complete` CI check must pass only when every named role has approved on the PR's latest commit with no
//! later CHANGES verdict" — and from the task's Deliverables ("a role counts as approved when its latest review whose
//! body starts `VERDICT: APPROVE <role>` was submitted on the head commit and no later `VERDICT: CHANGES <role>`
//! exists; fails naming each missing role"), not from the implementation.
//!
//! Each case is a hand-written review list checked through two shared assertions, `passes` and `fails_naming`; each
//! test's control (R-176, R-199) feeds the same assertion an input that differs in the one respect the requirement
//! turns on, and trips the assertion by its message.
// The file name `qa_TASK-M0-36` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::path::Path;

use validation::negative_control;
use xtask::reviews_check::{check, parse_reviews, reviewers, task_file};

const HEAD: &str = "1111111111111111111111111111111111111111";
const OLD: &str = "2222222222222222222222222222222222222222";

/// One review as the GitHub API lists it: `(body, commit, submitted_at)`.
type R<'a> = (&'a str, &'a str, &'a str);

/// The problems reviews-check reports for `reviews` (in the order given, not necessarily submission order) against
/// `roles` on `HEAD`.
fn problems(reviews: &[R], roles: &[&str]) -> Vec<String> {
    let json = serde_json::Value::Array(
        reviews
            .iter()
            .map(|(body, commit, at)| {
                serde_json::json!({
                    "user": {"login": "Ma1achy"},
                    "body": body,
                    "state": "COMMENTED",
                    "commit_id": commit,
                    "submitted_at": at,
                })
            })
            .collect(),
    )
    .to_string();
    let reviews = parse_reviews(&json).expect("review list parsed");
    let roles: Vec<String> = roles.iter().map(|r| (*r).to_owned()).collect();
    check(&reviews, HEAD, &roles)
}

/// The check passes: no problem at all.
fn passes(reviews: &[R], roles: &[&str]) {
    let found = problems(reviews, roles);
    assert!(
        found.is_empty(),
        "qa36: check failed where every role approved on head: {found:?}"
    );
}

/// The check fails with exactly one problem per role in `failing`, each naming its role, and names no other role.
fn fails_naming(reviews: &[R], roles: &[&str], failing: &[&str]) {
    let found = problems(reviews, roles);
    let names = |p: &String, role: &str| p.contains(&format!("`{role}`"));
    let ok = found.len() == failing.len()
        && failing
            .iter()
            .all(|role| found.iter().any(|p| names(p, role)))
        && roles
            .iter()
            .filter(|role| !failing.contains(role))
            .all(|role| !found.iter().any(|p| names(p, role)));
    assert!(
        ok,
        "qa36: check did not fail naming exactly {failing:?}: {found:?}"
    );
}

// --- An earlier CHANGES is answered by a later APPROVE on head -------------------------------------------------------

const CHANGES_THEN_APPROVE: &[R] = &[
    (
        "VERDICT: CHANGES qa\n\n1. a test cannot fail",
        OLD,
        "2026-09-29T10:00:00Z",
    ),
    ("VERDICT: APPROVE qa", HEAD, "2026-09-29T11:00:00Z"),
];

#[test]
fn qa36_changes_then_approve_on_head_passes() {
    passes(CHANGES_THEN_APPROVE, &["qa"]);
}

negative_control!(
    qa36_changes_then_approve_on_head_passes,
    "the same APPROVE on the older commit is not an approval on head",
    expected = "qa36: check failed where every role approved on head",
    passes(
        &[
            (
                "VERDICT: CHANGES qa\n\n1. a test cannot fail",
                OLD,
                "2026-09-29T10:00:00Z"
            ),
            ("VERDICT: APPROVE qa", OLD, "2026-09-29T11:00:00Z"),
        ],
        &["qa"]
    )
);

// --- The latest APPROVE decides the commit ----------------------------------------------------------------------------

/// A re-approval on head after an approval on an older commit counts.
#[test]
fn qa36_reapproval_on_head_passes() {
    passes(
        &[
            ("VERDICT: APPROVE code", OLD, "2026-09-29T10:00:00Z"),
            ("VERDICT: APPROVE code", HEAD, "2026-09-29T11:00:00Z"),
        ],
        &["code"],
    );
}

negative_control!(
    qa36_reapproval_on_head_passes,
    "with no approval on head, an approval on an older commit does not pass",
    expected = "qa36: check failed where every role approved on head",
    passes(
        &[("VERDICT: APPROVE code", OLD, "2026-09-29T10:00:00Z")],
        &["code"]
    )
);

/// The latest APPROVE is on an older commit: the role fails even though an earlier APPROVE was on head.
#[test]
fn qa36_latest_approve_on_older_commit_fails_naming_the_role() {
    fails_naming(
        &[
            ("VERDICT: APPROVE code", HEAD, "2026-09-29T10:00:00Z"),
            ("VERDICT: APPROVE code", OLD, "2026-09-29T11:00:00Z"),
        ],
        &["code"],
        &["code"],
    );
}

negative_control!(
    qa36_latest_approve_on_older_commit_fails_naming_the_role,
    "with the latest APPROVE on head there is no role to name",
    expected = "qa36: check did not fail naming exactly",
    fails_naming(
        &[
            ("VERDICT: APPROVE code", OLD, "2026-09-29T10:00:00Z"),
            ("VERDICT: APPROVE code", HEAD, "2026-09-29T11:00:00Z"),
        ],
        &["code"],
        &["code"]
    )
);

// --- A later CHANGES supersedes, whatever commit it is on ---------------------------------------------------------

/// "No later CHANGES verdict": a CHANGES submitted after the APPROVE supersedes it even if it names an older commit.
#[test]
fn qa36_later_changes_on_older_commit_still_supersedes() {
    fails_naming(
        &[
            ("VERDICT: APPROVE physics", HEAD, "2026-09-29T11:00:00Z"),
            (
                "VERDICT: CHANGES physics\n\n1. wrong space",
                OLD,
                "2026-09-29T12:00:00Z",
            ),
        ],
        &["physics"],
        &["physics"],
    );
}

negative_control!(
    qa36_later_changes_on_older_commit_still_supersedes,
    "a CHANGES earlier than the APPROVE supersedes nothing",
    expected = "qa36: check did not fail naming exactly",
    fails_naming(
        &[
            (
                "VERDICT: CHANGES physics\n\n1. wrong space",
                OLD,
                "2026-09-29T10:00:00Z"
            ),
            ("VERDICT: APPROVE physics", HEAD, "2026-09-29T11:00:00Z"),
        ],
        &["physics"],
        &["physics"]
    )
);

/// "Later" is submission time, not the order the list happens to arrive in.
#[test]
fn qa36_later_is_submission_time_not_list_order() {
    fails_naming(
        &[
            (
                "VERDICT: CHANGES physics\n\n1. wrong space",
                HEAD,
                "2026-09-29T12:00:00Z",
            ),
            ("VERDICT: APPROVE physics", HEAD, "2026-09-29T11:00:00Z"),
        ],
        &["physics"],
        &["physics"],
    );
}

negative_control!(
    qa36_later_is_submission_time_not_list_order,
    "the same list with the CHANGES submitted first is approved",
    expected = "qa36: check did not fail naming exactly",
    fails_naming(
        &[
            (
                "VERDICT: CHANGES physics\n\n1. wrong space",
                HEAD,
                "2026-09-29T10:00:00Z"
            ),
            ("VERDICT: APPROVE physics", HEAD, "2026-09-29T11:00:00Z"),
        ],
        &["physics"],
        &["physics"]
    )
);

// --- Verdicts are per role ------------------------------------------------------------------------------------------

/// Another role's CHANGES does not supersede this role's APPROVE; the check names only the role that has not approved.
#[test]
fn qa36_another_roles_changes_names_only_that_role() {
    fails_naming(
        &[
            ("VERDICT: APPROVE qa", HEAD, "2026-09-29T11:00:00Z"),
            (
                "VERDICT: CHANGES code\n\n1. clippy",
                HEAD,
                "2026-09-29T12:00:00Z",
            ),
        ],
        &["code", "qa"],
        &["code"],
    );
}

negative_control!(
    qa36_another_roles_changes_names_only_that_role,
    "with code also approved there is no role to name",
    expected = "qa36: check did not fail naming exactly",
    fails_naming(
        &[
            ("VERDICT: APPROVE qa", HEAD, "2026-09-29T11:00:00Z"),
            ("VERDICT: APPROVE code", HEAD, "2026-09-29T12:00:00Z"),
        ],
        &["code", "qa"],
        &["code"]
    )
);

/// Each missing role is named, not just the first.
#[test]
fn qa36_every_missing_role_is_named() {
    fails_naming(
        &[("VERDICT: APPROVE code", HEAD, "2026-09-29T11:00:00Z")],
        &["code", "qa", "physics", "gui", "perf"],
        &["qa", "physics", "gui", "perf"],
    );
}

negative_control!(
    qa36_every_missing_role_is_named,
    "with only qa missing, the other roles are not named",
    expected = "qa36: check did not fail naming exactly",
    fails_naming(
        &[
            ("VERDICT: APPROVE code", HEAD, "2026-09-29T11:00:00Z"),
            ("VERDICT: APPROVE physics", HEAD, "2026-09-29T11:01:00Z"),
            ("VERDICT: APPROVE gui", HEAD, "2026-09-29T11:02:00Z"),
            ("VERDICT: APPROVE perf", HEAD, "2026-09-29T11:03:00Z"),
        ],
        &["code", "qa", "physics", "gui", "perf"],
        &["qa", "physics", "gui", "perf"]
    )
);

// --- The verdict heads the review --------------------------------------------------------------------------------

/// A body that mentions the verdict after other text is not headed by it: the role has not approved.
#[test]
fn qa36_verdict_not_at_the_head_does_not_count() {
    fails_naming(
        &[(
            "Looks fine to me.\n\nVERDICT: APPROVE qa",
            HEAD,
            "2026-09-29T11:00:00Z",
        )],
        &["qa"],
        &["qa"],
    );
}

negative_control!(
    qa36_verdict_not_at_the_head_does_not_count,
    "the same verdict heading the body counts",
    expected = "qa36: check did not fail naming exactly",
    fails_naming(
        &[(
            "VERDICT: APPROVE qa\n\nLooks fine to me.",
            HEAD,
            "2026-09-29T11:00:00Z"
        )],
        &["qa"],
        &["qa"]
    )
);

/// A CHANGES verdict not at the head of a later review does not supersede.
#[test]
fn qa36_changes_not_at_the_head_does_not_supersede() {
    passes(
        &[
            ("VERDICT: APPROVE qa", HEAD, "2026-09-29T11:00:00Z"),
            (
                "Note: an earlier round said VERDICT: CHANGES qa",
                HEAD,
                "2026-09-29T12:00:00Z",
            ),
        ],
        &["qa"],
    );
}

negative_control!(
    qa36_changes_not_at_the_head_does_not_supersede,
    "a later review headed by CHANGES supersedes",
    expected = "qa36: check failed where every role approved on head",
    passes(
        &[
            ("VERDICT: APPROVE qa", HEAD, "2026-09-29T11:00:00Z"),
            (
                "VERDICT: CHANGES qa\n\n1. flaky",
                HEAD,
                "2026-09-29T12:00:00Z"
            ),
        ],
        &["qa"]
    )
);

// --- The roles come from the task file the title names ----------------------------------------------------------

/// WORKFLOW § "Task files": `code` and `qa` review every task; the others are physics, gui and perf.
fn roles_are_valid(file: &str, roles: &[String]) {
    let known = ["code", "qa", "physics", "gui", "perf"];
    assert!(
        roles.iter().any(|r| r == "code")
            && roles.iter().any(|r| r == "qa")
            && roles.iter().all(|r| known.contains(&r.as_str())),
        "qa36: {file}: roles read wrongly: {roles:?}"
    );
}

/// For every task file in the plan, the title `<TASK-id>: …` names that file, and its Reviewers field reads to the
/// roles the workflow allows, `code` and `qa` among them.
#[test]
fn qa36_every_task_title_names_its_file_and_roles() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut seen = 0;
    for milestone in std::fs::read_dir(root.join("plan/tasks")).expect("plan/tasks read") {
        let milestone = milestone.expect("dir entry").path();
        if !milestone.is_dir() {
            continue;
        }
        for entry in std::fs::read_dir(&milestone).expect("milestone dir read") {
            let path = entry.expect("dir entry").path();
            let Some(id) = path
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.strip_suffix(".md"))
                .filter(|n| n.starts_with("TASK-"))
            else {
                continue;
            };
            let (got_id, file) =
                task_file(&format!("{id}: a title")).unwrap_or_else(|e| panic!("{id}: {e}"));
            assert_eq!(got_id, id);
            assert_eq!(
                root.join(&file),
                path,
                "qa36: title {id} names the wrong file"
            );
            let text = std::fs::read_to_string(&path).expect("task file read");
            let roles = reviewers(&text).unwrap_or_else(|e| panic!("{file}: {e}"));
            roles_are_valid(&file, &roles);
            seen += 1;
        }
    }
    assert!(seen > 100, "qa36: only {seen} task files found");
}

negative_control!(
    qa36_every_task_title_names_its_file_and_roles,
    "a Reviewers field without qa is not a valid role list",
    expected = "qa36: plan/tasks/M0/TASK-M0-99.md: roles read wrongly",
    roles_are_valid(
        "plan/tasks/M0/TASK-M0-99.md",
        &reviewers("- **Milestone:** M0\n- **Reviewers:** code, physics\n").expect("roles read")
    )
);

// --- Only a submitted review records a verdict ---------------------------------------------------------------------

/// The problems for one review headed `VERDICT: APPROVE qa` on head, submitted at `at` (`null`: still pending, as
/// the reviews API lists a pending review to its author, which is the one account every agent posts from).
fn one_approve(at: &str) -> Vec<String> {
    let json = format!(
        r#"[{{"body":"VERDICT: APPROVE qa","state":"{}","commit_id":"{HEAD}","submitted_at":{at}}}]"#,
        if at == "null" { "PENDING" } else { "COMMENTED" }
    );
    check(
        &parse_reviews(&json).expect("review list parsed"),
        HEAD,
        &["qa".to_owned()],
    )
}

/// The role is named as not approved.
fn not_approved(found: &[String]) {
    assert!(
        found.len() == 1 && found[0].contains("`qa`"),
        "qa36: an unrecorded verdict counted as qa's approval: {found:?}"
    );
}

/// A pending review is not yet a recorded verdict: it does not approve.
#[test]
fn qa36_pending_approve_does_not_count() {
    not_approved(&one_approve("null"));
}

negative_control!(
    qa36_pending_approve_does_not_count,
    "the same review, submitted, approves",
    expected = "qa36: an unrecorded verdict counted as qa's approval",
    not_approved(&one_approve("\"2026-09-29T11:00:00Z\""))
);

// --- The whole check, end to end: title → task file → roles → reviews on the PR's head -------------------------------

/// Runs `cargo xtask reviews-check --pr 57` with a stand-in `gh` on `PATH` that answers the PR endpoint with `title`
/// and head `HEAD`, and the reviews endpoint with `reviews`. Returns whether it passed, and its stderr.
#[cfg(unix)]
fn run_check(tag: &str, title: &str, reviews: &[R]) -> (bool, String) {
    use std::os::unix::fs::PermissionsExt;
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("qa36_gh_{tag}"));
    std::fs::create_dir_all(&dir).expect("stub dir made");
    let pull = serde_json::json!({"number": 57, "title": title, "head": {"sha": HEAD}}).to_string();
    let list = serde_json::Value::Array(
        reviews
            .iter()
            .map(|(body, commit, at)| {
                serde_json::json!({"body": body, "state": "COMMENTED", "commit_id": commit, "submitted_at": at})
            })
            .collect(),
    )
    .to_string();
    std::fs::write(dir.join("pull.json"), pull).expect("pull written");
    std::fs::write(dir.join("reviews.json"), list).expect("reviews written");
    let script = format!(
        "#!/bin/sh\ncase \"$2\" in\n  */pulls/57) cat '{0}/pull.json' ;;\n  */pulls/57/reviews) cat '{0}/reviews.json' ;;\n  *) echo \"unexpected gh $*\" >&2; exit 1 ;;\nesac\n",
        dir.display()
    );
    let gh = dir.join("gh");
    std::fs::write(&gh, script).expect("stub gh written");
    std::fs::set_permissions(&gh, std::fs::Permissions::from_mode(0o755))
        .expect("stub gh executable");
    let path = format!(
        "{}:{}",
        dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["reviews-check", "--pr", "57"])
        .env("PATH", path)
        .env_remove("GITHUB_EVENT_PATH")
        .output()
        .expect("xtask ran");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// TASK-M0-36's own task file names `code, qa`; both approved on head passes the whole check.
#[cfg(unix)]
fn end_to_end_passes(tag: &str, reviews: &[R]) {
    let (ok, stderr) = run_check(tag, "TASK-M0-36: `cargo xtask reviews-check`", reviews);
    assert!(
        ok,
        "qa36: end-to-end check failed with every role approved on head: {stderr}"
    );
}

#[cfg(unix)]
#[test]
fn qa36_end_to_end_task_roles_approved_on_head_pass() {
    end_to_end_passes(
        "pass",
        &[
            ("VERDICT: APPROVE code", HEAD, "2026-09-29T11:00:00Z"),
            ("VERDICT: APPROVE qa", HEAD, "2026-09-29T11:05:00Z"),
        ],
    );
}

#[cfg(unix)]
negative_control!(
    qa36_end_to_end_task_roles_approved_on_head_pass,
    "qa's approval on an older commit fails the end-to-end check",
    expected = "qa36: end-to-end check failed",
    end_to_end_passes(
        "pass_control",
        &[
            ("VERDICT: APPROVE code", HEAD, "2026-09-29T11:00:00Z"),
            ("VERDICT: APPROVE qa", OLD, "2026-09-29T11:05:00Z"),
        ]
    )
);

/// The end-to-end check fails, and its stderr names `role` and no other of the task's roles.
#[cfg(unix)]
fn end_to_end_fails_naming(tag: &str, reviews: &[R], role: &str, other: &str) {
    let (ok, stderr) = run_check(tag, "TASK-M0-36: `cargo xtask reviews-check`", reviews);
    assert!(
        !ok && stderr.contains(&format!("`{role}`")) && !stderr.contains(&format!("`{other}`")),
        "qa36: end-to-end check did not fail naming {role}: ok={ok}, {stderr}"
    );
}

/// A role the task file names but that never posted a verdict fails the check, named; the roles come from the task
/// file, not from the reviews present.
#[cfg(unix)]
#[test]
fn qa36_end_to_end_role_named_by_task_file_but_absent_fails() {
    end_to_end_fails_naming(
        "missing",
        &[("VERDICT: APPROVE code", HEAD, "2026-09-29T11:00:00Z")],
        "qa",
        "code",
    );
}

#[cfg(unix)]
negative_control!(
    qa36_end_to_end_role_named_by_task_file_but_absent_fails,
    "with qa approved on head there is no role to name",
    expected = "qa36: end-to-end check did not fail naming qa",
    end_to_end_fails_naming(
        "missing_control",
        &[
            ("VERDICT: APPROVE code", HEAD, "2026-09-29T11:00:00Z"),
            ("VERDICT: APPROVE qa", HEAD, "2026-09-29T11:05:00Z"),
        ],
        "qa",
        "code"
    )
);
