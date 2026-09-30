//! `cargo xtask reviews-check [--pr N]`, the `reviews-complete` check (R-175; REQ-SYS-066): the reviewers are agents
//! posting from one account, so each posts a PR review headed `VERDICT: APPROVE <role>` or `VERDICT: CHANGES <role>`.
//! The check reads the task id from the PR title, the Reviewers field from `plan/tasks/<Mn>/<TASK-id>.md` and the
//! PR's reviews, and passes only when every role the task file names has approved on the latest commit: a role counts
//! as approved when its latest `VERDICT: APPROVE <role>` review was submitted on the head commit and no later
//! `VERDICT: CHANGES <role>` exists. It fails naming each role that has not.
//!
//! Two amendments. An APPROVE on an earlier commit still counts on the head when every later commit is qa's own
//! `qa: tests for <TASK-id>` commit adding files only under qa's paths (R-260, R-237). A PR whose title names no task
//! passes, printing "no task, no named reviewers" (R-261).

use std::path::Path;
use std::process::Command;

use serde::Deserialize;

/// One PR review, as `gh api repos/{owner}/{repo}/pulls/N/reviews` lists it.
#[derive(Debug, Deserialize)]
pub struct Review {
    /// The review's text; its first line carries the verdict.
    #[serde(default)]
    pub body: Option<String>,
    /// The commit the review was submitted on.
    #[serde(default)]
    pub commit_id: Option<String>,
    /// When it was submitted (ISO 8601, UTC); absent while the review is pending.
    #[serde(default)]
    pub submitted_at: Option<String>,
}

/// Parses a review list. `gh api --paginate` prints one JSON array per page, back to back; their reviews are joined
/// in order.
pub fn parse_reviews(json: &str) -> Result<Vec<Review>, String> {
    parse_pages(json, "review list")
}

/// The task id a PR title starts with (`TASK-M0-03: ...` gives `TASK-M0-03`), and the path of its task file under
/// the workspace root.
pub fn task_file(title: &str) -> Result<(String, String), String> {
    let id = title.split(':').next().unwrap_or_default().trim();
    let parts: Vec<&str> = id.split('-').collect();
    let valid = matches!(parts.as_slice(), ["TASK", milestone, n]
        if milestone.len() > 1 && milestone.starts_with('M')
            && milestone[1..].chars().all(|c| c.is_ascii_digit())
            && !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
    if !valid {
        return Err(format!(
            "PR title `{title}` does not start `TASK-Mn-nn: ` (plan/WORKFLOW.md), so it names no task file"
        ));
    }
    Ok((id.to_owned(), format!("plan/tasks/{}/{id}.md", parts[1])))
}

/// The roles a task file's `- **Reviewers:**` field names, in order.
pub fn reviewers(task: &str) -> Result<Vec<String>, String> {
    let field = task
        .lines()
        .find_map(|line| line.trim_start().strip_prefix("- **Reviewers:**"))
        .ok_or("the task file has no `- **Reviewers:**` field")?;
    let roles: Vec<String> = field
        .split(',')
        .map(|role| role.trim().to_owned())
        .filter(|role| !role.is_empty())
        .collect();
    if roles.is_empty() {
        return Err("the task file's Reviewers field names no role".to_owned());
    }
    Ok(roles)
}

/// Whether `body` starts `VERDICT: <verdict> <role>`, the role ending the word.
fn is_verdict(body: &str, verdict: &str, role: &str) -> bool {
    let Some(rest) = body
        .trim_start()
        .strip_prefix("VERDICT: ")
        .and_then(|rest| rest.strip_prefix(verdict))
        .and_then(|rest| rest.strip_prefix(' '))
        .and_then(|rest| rest.strip_prefix(role))
    else {
        return false;
    };
    !rest.starts_with(|c: char| c.is_alphanumeric() || c == '-' || c == '_')
}

/// A PR commit, as `gh api repos/{owner}/{repo}/pulls/N/commits` lists it (oldest first), with its files as
/// `gh api repos/{owner}/{repo}/commits/SHA` gives them.
#[derive(Debug, Deserialize)]
pub struct Commit {
    pub sha: String,
    pub commit: CommitMessage,
    /// The files it changes; read only for a commit titled `qa: tests for <TASK-id>`.
    #[serde(default)]
    pub files: Vec<CommitFile>,
}

/// The `commit` object of a listed commit.
#[derive(Debug, Deserialize)]
pub struct CommitMessage {
    pub message: String,
}

/// One file a commit changes, and how (`added`, `modified`, `removed`, `renamed`, ...).
#[derive(Debug, Deserialize)]
pub struct CommitFile {
    pub filename: String,
    pub status: String,
}

/// Whether `commit` is qa's own test commit for task `id` (R-260): its subject line is `qa: tests for <id>`, it
/// changes at least one file, and every file it changes is added under `crates/*/tests/`, `xtask/tests/` or
/// `fixtures/` (R-237).
pub fn is_qa_test_commit(commit: &Commit, id: &str) -> bool {
    let subject = commit
        .commit
        .message
        .lines()
        .next()
        .unwrap_or_default()
        .trim();
    subject == format!("qa: tests for {id}")
        && !commit.files.is_empty()
        && commit
            .files
            .iter()
            .all(|f| f.status == "added" && is_qa_path(&f.filename))
}

/// Whether `path` lies under `crates/*/tests/`, `xtask/tests/` or `fixtures/` (R-237).
fn is_qa_path(path: &str) -> bool {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.iter().any(|p| p.is_empty() || *p == "..") {
        return false;
    }
    matches!(
        parts.as_slice(),
        ["crates", _, "tests", _, ..] | ["xtask", "tests", _, ..] | ["fixtures", _, ..]
    )
}

/// Whether an APPROVE on `commit` counts on `head`: `commit` is the head, or every PR commit after it is qa's own test
/// commit for task `id` (R-260). `commits` is the PR's commit list, oldest first.
fn counts_on_head(commit: &str, head: &str, id: &str, commits: &[Commit]) -> bool {
    if commit == head {
        return true;
    }
    let Some(at) = commits.iter().position(|c| c.sha == commit) else {
        return false;
    };
    let later = &commits[at + 1..];
    later.last().is_some_and(|c| c.sha == head) && later.iter().all(|c| is_qa_test_commit(c, id))
}

/// One problem for each role in `roles` not approved on `head`, naming the role; empty when every role has approved.
/// An approval counts only on the head commit itself (R-175); [`check_with_commits`] adds R-260.
pub fn check(reviews: &[Review], head: &str, roles: &[String]) -> Vec<String> {
    check_with_commits(reviews, head, roles, "", &[])
}

/// As [`check`], but an APPROVE on an earlier commit also counts when every later commit in `commits` (the PR's, oldest
/// first) is qa's own `qa: tests for <id>` commit adding files only under qa's paths (R-260).
pub fn check_with_commits(
    reviews: &[Review],
    head: &str,
    roles: &[String],
    id: &str,
    commits: &[Commit],
) -> Vec<String> {
    // Submitted reviews in submission order; a pending review has no verdict yet.
    let mut submitted: Vec<&Review> = reviews
        .iter()
        .filter(|r| r.submitted_at.is_some())
        .collect();
    submitted.sort_by(|a, b| a.submitted_at.cmp(&b.submitted_at));
    let mut problems = Vec::new();
    for role in roles {
        let latest = |verdict: &str| {
            submitted
                .iter()
                .rposition(|r| is_verdict(r.body.as_deref().unwrap_or_default(), verdict, role))
        };
        match (latest("APPROVE"), latest("CHANGES")) {
            (None, _) => problems.push(format!(
                "role `{role}` has not approved: no review headed `VERDICT: APPROVE {role}`"
            )),
            (Some(approve), Some(changes)) if changes > approve => problems.push(format!(
                "role `{role}` has not approved: its `VERDICT: APPROVE {role}` is superseded by a later \
                 `VERDICT: CHANGES {role}`"
            )),
            (Some(approve), _) => {
                let commit = submitted[approve].commit_id.as_deref().unwrap_or_default();
                if !counts_on_head(commit, head, id, commits) {
                    problems.push(format!(
                        "role `{role}` has not approved on the head commit {head}: its latest `VERDICT: APPROVE \
                         {role}` is on {commit}, and not every later commit is `qa: tests for {id}` adding files \
                         only under qa's paths (R-260)"
                    ));
                }
            }
        }
    }
    problems
}

/// Runs `gh api <endpoint>` (with `--paginate` when `paginate`), returning its stdout.
fn gh_api(endpoint: &str, paginate: bool) -> Result<String, String> {
    let mut command = Command::new("gh");
    command.arg("api").arg(endpoint);
    if paginate {
        command.arg("--paginate");
    }
    let output = command
        .output()
        .map_err(|e| format!("cannot run `gh api {endpoint}`: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "`gh api {endpoint}` failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout)
        .map_err(|e| format!("`gh api {endpoint}` printed non-UTF-8: {e}"))
}

/// The PR number: `pr`, or else the `pull_request` of the event at `$GITHUB_EVENT_PATH`.
fn pr_number(pr: Option<u64>) -> Result<u64, String> {
    if let Some(n) = pr {
        return Ok(n);
    }
    let path = std::env::var("GITHUB_EVENT_PATH")
        .map_err(|_| "no --pr N, and $GITHUB_EVENT_PATH is not set".to_owned())?;
    let text =
        std::fs::read_to_string(&path).map_err(|e| format!("cannot read event {path}: {e}"))?;
    let event: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("event {path} is not JSON: {e}"))?;
    event["pull_request"]["number"]
        .as_u64()
        .ok_or_else(|| format!("event {path} has no `pull_request.number`"))
}

/// What `reviews-complete` prints for a PR whose title names no task (R-261).
pub const NO_TASK: &str = "no task, no named reviewers";

/// Whether a PR title names a task: it starts `TASK-`, the form `TASK-Mn-nn: <title>` (plan/WORKFLOW.md) takes. A
/// title starting `TASK-` that is not well formed names a task that [`task_file`] then rejects.
pub fn names_task(title: &str) -> bool {
    title.trim_start().starts_with("TASK-")
}

/// What reviews-check reads of one PR: its number, title, head commit, reviews and commits (oldest first).
#[derive(Debug, Deserialize)]
pub struct Pr {
    pub number: u64,
    pub title: String,
    pub head: String,
    #[serde(default)]
    pub reviews: Vec<Review>,
    #[serde(default)]
    pub commits: Vec<Commit>,
}

/// The check's verdict on `pr`, reading the task file under `root`: the line to print when it passes, or else each
/// summary and then each problem on its own line.
pub fn verdict(root: &Path, pr: &Pr) -> Result<String, String> {
    let n = pr.number;
    if !names_task(&pr.title) {
        return Ok(format!("xtask reviews-check: PR #{n}: {NO_TASK} (R-261)"));
    }
    let (id, file) = task_file(&pr.title)?;
    let task = std::fs::read_to_string(root.join(&file))
        .map_err(|e| format!("cannot read {file}: {e}"))?;
    let roles = reviewers(&task).map_err(|e| format!("{file}: {e}"))?;
    let head = &pr.head;
    let problems = check_with_commits(&pr.reviews, head, &roles, &id, &pr.commits);
    if problems.is_empty() {
        return Ok(format!(
            "xtask reviews-check: PR #{n} ({id}): {} approved on {head} (R-175, R-260)",
            roles.join(", ")
        ));
    }
    let mut report = vec![format!(
        "PR #{n} ({id}): {} role(s) not approved on the head commit (R-175)",
        problems.len()
    )];
    report.extend(problems.iter().map(|p| format!("  {p}")));
    Err(report.join("\n"))
}

/// Parses `gh api --paginate` output, one JSON value per page back to back, joining each page's items.
fn parse_pages<T: serde::de::DeserializeOwned>(json: &str, what: &str) -> Result<Vec<T>, String> {
    let mut items = Vec::new();
    for page in serde_json::Deserializer::from_str(json).into_iter::<Vec<T>>() {
        items.extend(page.map_err(|e| format!("{what} is not JSON: {e}"))?);
    }
    Ok(items)
}

/// `cargo xtask reviews-check`: checks PR `pr` (or the event's) of the repository `gh` resolves from the checkout,
/// against the task file under `root`.
pub fn run(root: &Path, pr: Option<u64>) -> Result<(), String> {
    let n = pr_number(pr)?;
    let pull: serde_json::Value = serde_json::from_str(&gh_api(
        &format!("repos/{{owner}}/{{repo}}/pulls/{n}"),
        false,
    )?)
    .map_err(|e| format!("PR {n} is not JSON: {e}"))?;
    let title = pull["title"].as_str().unwrap_or_default().to_owned();
    let head = pull["head"]["sha"]
        .as_str()
        .ok_or(format!("PR {n} has no head sha"))?
        .to_owned();
    let mut pr = Pr {
        number: n,
        title,
        head,
        reviews: Vec::new(),
        commits: Vec::new(),
    };
    if names_task(&pr.title) {
        pr.reviews = parse_reviews(&gh_api(
            &format!("repos/{{owner}}/{{repo}}/pulls/{n}/reviews"),
            true,
        )?)?;
        // The commits matter only when some APPROVE is on an earlier commit, which R-260 may carry over to the head.
        let off_head = pr.reviews.iter().any(|r| {
            r.body
                .as_deref()
                .unwrap_or_default()
                .trim_start()
                .starts_with("VERDICT: APPROVE ")
                && r.commit_id.as_deref() != Some(pr.head.as_str())
        });
        if off_head {
            pr.commits = parse_pages(
                &gh_api(&format!("repos/{{owner}}/{{repo}}/pulls/{n}/commits"), true)?,
                "commit list",
            )?;
            // Only a commit titled `qa: tests for ...` can carry an approval over (R-260), so only its files are read.
            for commit in &mut pr.commits {
                if commit.commit.message.starts_with("qa: tests for ") {
                    let pages = gh_api(
                        &format!("repos/{{owner}}/{{repo}}/commits/{}", commit.sha),
                        true,
                    )?;
                    for page in serde_json::Deserializer::from_str(&pages).into_iter::<Commit>() {
                        let page =
                            page.map_err(|e| format!("commit {} is not JSON: {e}", commit.sha))?;
                        commit.files.extend(page.files);
                    }
                }
            }
        }
    }
    println!("{}", verdict(root, &pr)?);
    Ok(())
}
