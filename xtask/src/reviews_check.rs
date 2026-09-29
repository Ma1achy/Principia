//! `cargo xtask reviews-check [--pr N]`, the `reviews-complete` check (R-175; REQ-SYS-066): the reviewers are agents
//! posting from one account, so each posts a PR review headed `VERDICT: APPROVE <role>` or `VERDICT: CHANGES <role>`.
//! The check reads the task id from the PR title, the Reviewers field from `plan/tasks/<Mn>/<TASK-id>.md` and the
//! PR's reviews, and passes only when every role the task file names has approved on the latest commit: a role counts
//! as approved when its latest `VERDICT: APPROVE <role>` review was submitted on the head commit and no later
//! `VERDICT: CHANGES <role>` exists. It fails naming each role that has not.

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
    let mut reviews = Vec::new();
    for page in serde_json::Deserializer::from_str(json).into_iter::<Vec<Review>>() {
        reviews.extend(page.map_err(|e| format!("review list is not JSON: {e}"))?);
    }
    Ok(reviews)
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

/// One problem for each role in `roles` not approved on `head`, naming the role; empty when every role has approved.
pub fn check(reviews: &[Review], head: &str, roles: &[String]) -> Vec<String> {
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
                if commit != head {
                    problems.push(format!(
                        "role `{role}` has not approved on the head commit {head}: its latest `VERDICT: APPROVE \
                         {role}` is on {commit}"
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

/// `cargo xtask reviews-check`: checks PR `pr` (or the event's) of the repository `gh` resolves from the checkout,
/// against the task file under `root`.
pub fn run(root: &Path, pr: Option<u64>) -> Result<(), String> {
    let n = pr_number(pr)?;
    let pull: serde_json::Value = serde_json::from_str(&gh_api(
        &format!("repos/{{owner}}/{{repo}}/pulls/{n}"),
        false,
    )?)
    .map_err(|e| format!("PR {n} is not JSON: {e}"))?;
    let title = pull["title"].as_str().unwrap_or_default();
    let head = pull["head"]["sha"]
        .as_str()
        .ok_or(format!("PR {n} has no head sha"))?;
    let (id, file) = task_file(title)?;
    let task = std::fs::read_to_string(root.join(&file))
        .map_err(|e| format!("cannot read {file}: {e}"))?;
    let roles = reviewers(&task).map_err(|e| format!("{file}: {e}"))?;
    let reviews = parse_reviews(&gh_api(
        &format!("repos/{{owner}}/{{repo}}/pulls/{n}/reviews"),
        true,
    )?)?;
    let problems = check(&reviews, head, &roles);
    for problem in &problems {
        eprintln!("xtask reviews-check: {problem}");
    }
    if problems.is_empty() {
        println!(
            "xtask reviews-check: PR #{n} ({id}): {} approved on {head} (R-175)",
            roles.join(", ")
        );
        Ok(())
    } else {
        Err(format!(
            "PR #{n} ({id}): {} role(s) not approved on the head commit (R-175)",
            problems.len()
        ))
    }
}
