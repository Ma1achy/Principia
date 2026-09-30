//! QA tests for TASK-M0-37, written from REQ-SYS-068, R-260, R-261 and R-237, not from the implementation:
//!
//! - "`reviews-check` must count a role's APPROVE on an earlier commit when every later commit is qa's own
//!   `qa: tests for <TASK-id>` commit adding files only under `crates/*/tests/`, `xtask/tests/` or `fixtures/`
//!   (R-260), and must pass a PR whose title names no task, printing "no task, no named reviewers" (R-261)."
//! - verify: "an approval followed only by a qa test commit passes; followed by a qa-titled commit that modifies a
//!   file, or adds outside qa's paths, or by any other commit, fails naming the role; a title naming no task passes
//!   with "no task, no named reviewers"".
//!
//! Two surfaces: `verdict` on a hand-built PR (title, head, reviews, commits with their files), and the whole
//! `cargo xtask reviews-check --pr 62` with a stand-in `gh` on `PATH` answering the PR, reviews, commit-list and
//! per-commit endpoints the way the GitHub API does. The task file read is TASK-M0-37's own (`code, qa`).
//!
//! Each test's control (R-176, R-199) feeds the same assertion an input differing in the one respect the requirement
//! turns on, and trips the assertion by its message.
// The file name `qa_TASK-M0-37` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::path::Path;

use serde_json::{json, Value};
use validation::negative_control;
use xtask::reviews_check::{verdict, Pr};

const TITLE: &str = "TASK-M0-37: `reviews-check`: qa's test commits keep an approval";
/// The implementer's commit, which both roles approved.
const IMPL: &str = "1111111111111111111111111111111111111111";
/// Commits after it.
const C2: &str = "2222222222222222222222222222222222222222";
const C3: &str = "3333333333333333333333333333333333333333";

/// A commit: `(sha, message, [(filename, status)])`.
type C<'a> = (&'a str, &'a str, &'a [(&'a str, &'a str)]);

const QA_MSG: &str = "qa: tests for TASK-M0-37\n\nCo-Authored-By: someone";
const IMPL_COMMIT: C = (
    IMPL,
    "TASK-M0-37: the change",
    &[("xtask/src/reviews_check.rs", "modified")],
);
const QA_ADDS: &[(&str, &str)] = &[
    ("xtask/tests/qa_TASK-M0-37.rs", "added"),
    ("crates/engine/tests/qa_x.rs", "added"),
    ("fixtures/gates/qa_x.json", "added"),
];

fn review(body: &str, commit: &str, at: &str) -> Value {
    json!({"user": {"login": "Ma1achy"}, "body": body, "state": "COMMENTED", "commit_id": commit, "submitted_at": at})
}

/// Both roles approved on `IMPL`.
fn both_on_impl() -> Vec<Value> {
    vec![
        review("VERDICT: APPROVE code", IMPL, "2026-09-30T10:00:00Z"),
        review("VERDICT: APPROVE qa", IMPL, "2026-09-30T10:05:00Z"),
    ]
}

/// `code` approved on `IMPL`, `qa` on `head`.
fn code_on_impl(head: &str) -> Vec<Value> {
    vec![
        review("VERDICT: APPROVE code", IMPL, "2026-09-30T10:00:00Z"),
        review("VERDICT: APPROVE qa", head, "2026-09-30T10:05:00Z"),
    ]
}

fn commit_json(c: &C) -> Value {
    let files: Vec<Value> =
        c.2.iter()
            .map(|(f, s)| json!({"filename": f, "status": s}))
            .collect();
    json!({"sha": c.0, "commit": {"message": c.1}, "files": files})
}

/// `verdict` on a PR titled `title`, head the last of `commits`.
fn check(title: &str, reviews: Vec<Value>, commits: &[C]) -> Result<String, String> {
    let head = commits.last().map(|c| c.0).unwrap_or(IMPL);
    let pr = json!({
        "number": 62,
        "title": title,
        "head": head,
        "reviews": reviews,
        "commits": commits.iter().map(commit_json).collect::<Vec<_>>(),
    });
    let pr: Pr = serde_json::from_value(pr).expect("PR built");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    verdict(&root, &pr)
}

fn passes(reviews: Vec<Value>, commits: &[C]) {
    let found = check(TITLE, reviews, commits);
    assert!(
        found.is_ok(),
        "qa37: approval before qa's own test commit(s) did not count: {found:?}"
    );
}

/// Fails, naming `code` and not `qa` (qa approved on the head, or approved alongside code; `named` lists which
/// roles must be named).
fn fails_naming(reviews: Vec<Value>, commits: &[C], named: &[&str]) {
    let found = check(TITLE, reviews, commits);
    let ok = found.as_ref().is_err_and(|e| {
        ["code", "qa"]
            .iter()
            .all(|r| e.contains(&format!("`{r}`")) == named.contains(r))
    });
    assert!(
        ok,
        "qa37: check did not fail naming exactly {named:?}: {found:?}"
    );
}

// --- Passes: an approval followed only by qa's own test commit(s) ---------------------------------------------------

#[test]
fn qa37_reviews_check_qa_commit_keeps_both_approvals() {
    passes(both_on_impl(), &[IMPL_COMMIT, (C2, QA_MSG, QA_ADDS)]);
}

negative_control!(
    qa37_reviews_check_qa_commit_keeps_both_approvals,
    "the same commit, not titled qa's, drops the approvals",
    expected = "qa37: approval before qa's own test commit(s) did not count",
    passes(
        both_on_impl(),
        &[IMPL_COMMIT, (C2, "tests for TASK-M0-37", QA_ADDS)]
    )
);

/// "every later commit": two qa test commits in a row still carry the approval.
#[test]
fn qa37_reviews_check_two_qa_commits_keep_the_approval() {
    passes(
        code_on_impl(C3),
        &[
            IMPL_COMMIT,
            (C2, QA_MSG, &[("xtask/tests/qa_a.rs", "added")]),
            (C3, QA_MSG, &[("fixtures/qa_b.json", "added")]),
        ],
    );
}

negative_control!(
    qa37_reviews_check_two_qa_commits_keep_the_approval,
    "the second commit modifies a file",
    expected = "qa37: approval before qa's own test commit(s) did not count",
    passes(
        code_on_impl(C3),
        &[
            IMPL_COMMIT,
            (C2, QA_MSG, &[("xtask/tests/qa_a.rs", "added")]),
            (C3, QA_MSG, &[("fixtures/qa_b.json", "modified")]),
        ]
    )
);

// --- Fails naming the role: a qa-titled commit that modifies, removes or renames ---------------------------------------

#[test]
fn qa37_reviews_check_qa_commit_modifying_fails() {
    for status in ["modified", "removed", "renamed", "changed", "copied"] {
        fails_naming(
            code_on_impl(C2),
            &[
                IMPL_COMMIT,
                (
                    C2,
                    QA_MSG,
                    &[
                        ("xtask/tests/qa_new.rs", "added"),
                        ("xtask/tests/reviews_check.rs", status),
                    ],
                ),
            ],
            &["code"],
        );
    }
}

negative_control!(
    qa37_reviews_check_qa_commit_modifying_fails,
    "both files added: nothing to name",
    expected = "qa37: check did not fail naming exactly",
    fails_naming(
        code_on_impl(C2),
        &[
            IMPL_COMMIT,
            (
                C2,
                QA_MSG,
                &[
                    ("xtask/tests/qa_new.rs", "added"),
                    ("xtask/tests/reviews_check2.rs", "added")
                ]
            ),
        ],
        &["code"]
    )
);

// --- Fails naming the role: a qa-titled commit adding outside qa's paths ---------------------------------------------

/// Paths outside `crates/*/tests/`, `xtask/tests/`, `fixtures/`, including near-misses on each prefix.
const OUTSIDE: &[&str] = &[
    "xtask/src/reviews_check.rs",
    "crates/engine/src/tests.rs",
    "crates/engine/src/tests/x.rs",
    "crates/tests/x.rs",
    "crates/engine/tests_x/a.rs",
    "xtask/tests_extra/a.rs",
    "xtask/testsx.rs",
    "fixtures_extra/a.json",
    "fixturesx.json",
    "docs/fixtures/a.json",
    "sub/xtask/tests/a.rs",
    "plan/tasks/M0/TASK-M0-99.md",
    "decisions.md",
    ".github/workflows/ci.yml",
    "Cargo.toml",
];

#[test]
fn qa37_reviews_check_qa_commit_adding_outside_fails() {
    for path in OUTSIDE {
        fails_naming(
            code_on_impl(C2),
            &[
                IMPL_COMMIT,
                (
                    C2,
                    QA_MSG,
                    &[("xtask/tests/qa_new.rs", "added"), (*path, "added")],
                ),
            ],
            &["code"],
        );
    }
}

negative_control!(
    qa37_reviews_check_qa_commit_adding_outside_fails,
    "the extra file inside qa's paths: nothing to name",
    expected = "qa37: check did not fail naming exactly",
    fails_naming(
        code_on_impl(C2),
        &[
            IMPL_COMMIT,
            (
                C2,
                QA_MSG,
                &[
                    ("xtask/tests/qa_new.rs", "added"),
                    ("crates/kernel/tests/qa_new.rs", "added")
                ]
            ),
        ],
        &["code"]
    )
);

// --- Fails naming the role: any other commit after the approval -------------------------------------------------------

/// Other commits, each adding only under qa's paths: only the title is wrong.
const OTHER_TITLES: &[&str] = &[
    "TASK-M0-37: fix the finding",
    "Merge branch 'main' into task/TASK-M0-37",
    "qa: tests for TASK-M0-36",
    "qa: tests for TASK-M0-37 and more",
    "QA: tests for TASK-M0-37",
    "tests for TASK-M0-37",
];

#[test]
fn qa37_reviews_check_other_commit_after_approval_fails() {
    for title in OTHER_TITLES {
        fails_naming(
            code_on_impl(C2),
            &[IMPL_COMMIT, (C2, *title, QA_ADDS)],
            &["code"],
        );
    }
}

negative_control!(
    qa37_reviews_check_other_commit_after_approval_fails,
    "the commit titled qa's: nothing to name",
    expected = "qa37: check did not fail naming exactly",
    fails_naming(
        code_on_impl(C2),
        &[IMPL_COMMIT, (C2, "qa: tests for TASK-M0-37", QA_ADDS)],
        &["code"]
    )
);

/// A qa commit then another commit, or another commit then a qa commit: not every later commit is qa's.
#[test]
fn qa37_reviews_check_other_commit_among_qa_commits_fails() {
    let other: C = (C2, "TASK-M0-37: fix", &[("xtask/tests/a.rs", "added")]);
    let qa2: C = (C2, QA_MSG, &[("xtask/tests/a.rs", "added")]);
    let qa3: C = (C3, QA_MSG, &[("xtask/tests/b.rs", "added")]);
    let other3: C = (C3, "TASK-M0-37: fix", &[("xtask/tests/b.rs", "added")]);
    fails_naming(code_on_impl(C3), &[IMPL_COMMIT, other, qa3], &["code"]);
    fails_naming(code_on_impl(C3), &[IMPL_COMMIT, qa2, other3], &["code"]);
}

negative_control!(
    qa37_reviews_check_other_commit_among_qa_commits_fails,
    "both later commits qa's: nothing to name",
    expected = "qa37: check did not fail naming exactly",
    fails_naming(
        code_on_impl(C3),
        &[
            IMPL_COMMIT,
            (C2, QA_MSG, &[("xtask/tests/a.rs", "added")]),
            (C3, QA_MSG, &[("xtask/tests/b.rs", "added")])
        ],
        &["code"]
    )
);

/// Both roles approved before a non-qa commit: both are named.
#[test]
fn qa37_reviews_check_other_commit_names_every_role() {
    fails_naming(
        both_on_impl(),
        &[IMPL_COMMIT, (C2, "TASK-M0-37: fix", QA_ADDS)],
        &["code", "qa"],
    );
}

negative_control!(
    qa37_reviews_check_other_commit_names_every_role,
    "the later commit is qa's: nothing to name",
    expected = "qa37: check did not fail naming exactly",
    fails_naming(
        both_on_impl(),
        &[IMPL_COMMIT, (C2, QA_MSG, QA_ADDS)],
        &["code", "qa"]
    )
);

/// R-260 carries an approval over; it does not undo R-175's "no later CHANGES verdict".
#[test]
fn qa37_reviews_check_later_changes_still_fails_across_qa_commit() {
    let mut reviews = code_on_impl(C2);
    reviews.push(review(
        "VERDICT: CHANGES code\n\n1. a finding",
        C2,
        "2026-09-30T11:00:00Z",
    ));
    fails_naming(reviews, &[IMPL_COMMIT, (C2, QA_MSG, QA_ADDS)], &["code"]);
}

negative_control!(
    qa37_reviews_check_later_changes_still_fails_across_qa_commit,
    "without the CHANGES verdict: nothing to name",
    expected = "qa37: check did not fail naming exactly",
    fails_naming(
        code_on_impl(C2),
        &[IMPL_COMMIT, (C2, QA_MSG, QA_ADDS)],
        &["code"]
    )
);

// --- R-261: a title naming no task ----------------------------------------------------------------------------------

const NO_TASK_TITLES: &[&str] = &[
    "R-264: size is the orchestrator's call, not a question for the human",
    "Rulings batch 12",
    "plan: port the colour section",
];

fn no_task_passes(title: &str) {
    // No reviews at all: there is no task file, so no role to wait for.
    let found = check(title, Vec::new(), &[IMPL_COMMIT]);
    assert!(
        found
            .as_ref()
            .is_ok_and(|line| line.contains("no task, no named reviewers")),
        "qa37: title `{title}` did not pass with \"no task, no named reviewers\": {found:?}"
    );
}

#[test]
fn qa37_reviews_check_no_task_title_passes() {
    for title in NO_TASK_TITLES {
        no_task_passes(title);
    }
}

negative_control!(
    qa37_reviews_check_no_task_title_passes,
    "a title naming a task with no reviews does not pass",
    expected = "did not pass with \"no task, no named reviewers\"",
    no_task_passes(TITLE)
);

/// A title that names a task still needs its roles: R-261 is not a way around R-175.
#[test]
fn qa37_reviews_check_task_title_without_reviews_fails() {
    fails_naming(Vec::new(), &[IMPL_COMMIT], &["code", "qa"]);
}

negative_control!(
    qa37_reviews_check_task_title_without_reviews_fails,
    "both roles approved on head: nothing to name",
    expected = "qa37: check did not fail naming exactly",
    fails_naming(
        vec![
            review("VERDICT: APPROVE code", IMPL, "2026-09-30T10:00:00Z"),
            review("VERDICT: APPROVE qa", IMPL, "2026-09-30T10:05:00Z"),
        ],
        &[IMPL_COMMIT],
        &["code", "qa"]
    )
);

// --- The whole check, end to end, through a stand-in `gh` -------------------------------------------------------------

/// Runs `cargo xtask reviews-check --pr 62` with a stand-in `gh` that answers as the GitHub API does: the PR, its
/// reviews, its commit list (without files) and each commit (with its files). Returns (passed, stdout, stderr).
#[cfg(unix)]
fn run_check(tag: &str, title: &str, reviews: Vec<Value>, commits: &[C]) -> (bool, String, String) {
    use validation::spawn::Spawn;
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("qa37_gh_{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("stub dir made");
    let head = commits.last().map(|c| c.0).unwrap_or(IMPL);
    let write = |name: &str, v: &Value| {
        std::fs::write(dir.join(name), v.to_string()).expect("stub file written")
    };
    write(
        "pull.json",
        &json!({"number": 62, "title": title, "head": {"sha": head}}),
    );
    write("reviews.json", &Value::Array(reviews));
    let list: Vec<Value> = commits
        .iter()
        .map(|c| json!({"sha": c.0, "commit": {"message": c.1}, "parents": []}))
        .collect();
    write("commits.json", &Value::Array(list));
    for c in commits {
        write(&format!("commit_{}.json", c.0), &commit_json(c));
    }
    let script = format!(
        "#!/bin/sh\ncase \"$2\" in\n  */pulls/62) cat '{0}/pull.json' ;;\n  */pulls/62/reviews) cat '{0}/reviews.json' ;;\n  */pulls/62/commits) cat '{0}/commits.json' ;;\n  */commits/*) f='{0}/commit_'\"${{2##*/}}\"'.json'; [ -f \"$f\" ] && cat \"$f\" || {{ echo \"no commit $2\" >&2; exit 1; }} ;;\n  *) echo \"unexpected gh $*\" >&2; exit 1 ;;\nesac\n",
        dir.display()
    );
    let gh = dir.join("gh");
    validation::spawn::write_executable(&gh, script).expect("stub gh written");
    let path = format!(
        "{}:{}",
        dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["reviews-check", "--pr", "62"])
        .env("PATH", path)
        .env_remove("GITHUB_EVENT_PATH")
        .timed_output()
        .expect("xtask ran");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[cfg(unix)]
fn end_to_end_passes(tag: &str, reviews: Vec<Value>, commits: &[C]) {
    let (ok, out, err) = run_check(tag, TITLE, reviews, commits);
    assert!(
        ok,
        "qa37: end-to-end check failed over qa's own test commit: {out} {err}"
    );
}

#[cfg(unix)]
#[test]
fn qa37_reviews_check_end_to_end_qa_commit_passes() {
    end_to_end_passes(
        "pass",
        both_on_impl(),
        &[IMPL_COMMIT, (C2, QA_MSG, QA_ADDS)],
    );
}

#[cfg(unix)]
negative_control!(
    qa37_reviews_check_end_to_end_qa_commit_passes,
    "the qa-titled commit also modifies a file: the approvals drop",
    expected = "qa37: end-to-end check failed",
    end_to_end_passes(
        "pass_control",
        both_on_impl(),
        &[
            IMPL_COMMIT,
            (
                C2,
                QA_MSG,
                &[
                    ("xtask/tests/qa_x.rs", "added"),
                    ("xtask/src/reviews_check.rs", "modified")
                ]
            )
        ]
    )
);

#[cfg(unix)]
fn end_to_end_fails_naming_code(tag: &str, commits: &[C]) {
    let (ok, out, err) = run_check(tag, TITLE, code_on_impl(commits.last().unwrap().0), commits);
    assert!(
        !ok && err.contains("`code`") && !err.contains("`qa`"),
        "qa37: end-to-end check did not fail naming code: ok={ok}, {out} {err}"
    );
}

#[cfg(unix)]
#[test]
fn qa37_reviews_check_end_to_end_outside_and_modifying_fail() {
    end_to_end_fails_naming_code(
        "outside",
        &[IMPL_COMMIT, (C2, QA_MSG, &[("xtask/src/new.rs", "added")])],
    );
    end_to_end_fails_naming_code(
        "modifies",
        &[
            IMPL_COMMIT,
            (C2, QA_MSG, &[("fixtures/gates/x.json", "modified")]),
        ],
    );
    end_to_end_fails_naming_code("other", &[IMPL_COMMIT, (C2, "TASK-M0-37: fix", QA_ADDS)]);
}

#[cfg(unix)]
negative_control!(
    qa37_reviews_check_end_to_end_outside_and_modifying_fail,
    "qa's commit adds only under qa's paths: nothing to name",
    expected = "qa37: end-to-end check did not fail naming code",
    end_to_end_fails_naming_code("fail_control", &[IMPL_COMMIT, (C2, QA_MSG, QA_ADDS)])
);

#[cfg(unix)]
fn end_to_end_no_task(tag: &str, title: &str) {
    let (ok, out, err) = run_check(tag, title, Vec::new(), &[IMPL_COMMIT]);
    assert!(
        ok && format!("{out}{err}").contains("no task, no named reviewers"),
        "qa37: end-to-end no-task title did not pass printing the line: ok={ok}, {out} {err}"
    );
}

#[cfg(unix)]
#[test]
fn qa37_reviews_check_end_to_end_no_task_passes() {
    end_to_end_no_task("no_task", NO_TASK_TITLES[0]);
}

#[cfg(unix)]
negative_control!(
    qa37_reviews_check_end_to_end_no_task_passes,
    "a title naming a task, with no reviews, fails",
    expected = "qa37: end-to-end no-task title did not pass",
    end_to_end_no_task("no_task_control", TITLE)
);

// --- Edges of "adding files only under qa's paths" and "every later commit" -------------------------------------------

/// A qa-titled commit that adds no file at all is not a commit "adding files" (R-260): the approval does not carry.
#[test]
fn qa37_reviews_check_qa_commit_adding_nothing_fails() {
    fails_naming(
        code_on_impl(C2),
        &[IMPL_COMMIT, (C2, QA_MSG, &[])],
        &["code"],
    );
}

negative_control!(
    qa37_reviews_check_qa_commit_adding_nothing_fails,
    "the commit adds one file under qa's paths: nothing to name",
    expected = "qa37: check did not fail naming exactly",
    fails_naming(
        code_on_impl(C2),
        &[
            IMPL_COMMIT,
            (C2, QA_MSG, &[("xtask/tests/qa_x.rs", "added")])
        ],
        &["code"]
    )
);

/// A path that climbs out of qa's directories is not under them.
#[test]
fn qa37_reviews_check_qa_commit_path_escaping_fails() {
    for path in [
        "xtask/tests/../src/reviews_check.rs",
        "crates/engine/tests/../src/lib.rs",
        "fixtures/../decisions.md",
    ] {
        fails_naming(
            code_on_impl(C2),
            &[IMPL_COMMIT, (C2, QA_MSG, &[(path, "added")])],
            &["code"],
        );
    }
}

negative_control!(
    qa37_reviews_check_qa_commit_path_escaping_fails,
    "the same paths without the climb: nothing to name",
    expected = "qa37: check did not fail naming exactly",
    fails_naming(
        code_on_impl(C2),
        &[
            IMPL_COMMIT,
            (C2, QA_MSG, &[("xtask/tests/src/reviews_check.rs", "added")])
        ],
        &["code"]
    )
);

/// An approval on a commit that is not in the PR's history (dropped by a force-push) has no "later commits" to carry
/// it: it counts only if it is on the head (R-175).
#[test]
fn qa37_reviews_check_approval_on_commit_not_in_pr_fails() {
    const GONE: &str = "9999999999999999999999999999999999999999";
    fails_naming(
        vec![
            review("VERDICT: APPROVE code", GONE, "2026-09-30T10:00:00Z"),
            review("VERDICT: APPROVE qa", C2, "2026-09-30T10:05:00Z"),
        ],
        &[IMPL_COMMIT, (C2, QA_MSG, QA_ADDS)],
        &["code"],
    );
}

negative_control!(
    qa37_reviews_check_approval_on_commit_not_in_pr_fails,
    "the approval on a commit in the PR's history: nothing to name",
    expected = "qa37: check did not fail naming exactly",
    fails_naming(
        code_on_impl(C2),
        &[IMPL_COMMIT, (C2, QA_MSG, QA_ADDS)],
        &["code"]
    )
);

/// `verdict` with the head given apart from the commit list, as when GitHub's list stops short of the head (it lists
/// at most 250 commits): the commits between the listed ones and the head are unknown, so "every later commit is
/// qa's" is not shown.
fn fails_with_head(head: &str, commits: &[C]) {
    let pr = json!({
        "number": 62,
        "title": TITLE,
        "head": head,
        "reviews": code_on_impl(head),
        "commits": commits.iter().map(commit_json).collect::<Vec<_>>(),
    });
    let pr: Pr = serde_json::from_value(pr).expect("PR built");
    let found = verdict(&Path::new(env!("CARGO_MANIFEST_DIR")).join(".."), &pr);
    assert!(
        found
            .as_ref()
            .is_err_and(|e| e.contains("`code`") && !e.contains("`qa`")),
        "qa37: approval carried to a head the commit list does not reach: {found:?}"
    );
}

#[test]
fn qa37_reviews_check_commit_list_short_of_head_fails() {
    fails_with_head(C3, &[IMPL_COMMIT, (C2, QA_MSG, QA_ADDS)]);
}

negative_control!(
    qa37_reviews_check_commit_list_short_of_head_fails,
    "the list reaches the head: nothing to name",
    expected = "qa37: approval carried to a head the commit list does not reach",
    fails_with_head(C2, &[IMPL_COMMIT, (C2, QA_MSG, QA_ADDS)])
);
