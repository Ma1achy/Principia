//! `cargo xtask pr-check [--event PATH]` (REQ-SYS-006, REQ-VAL-001, REQ-VAL-005, REQ-VAL-008): reads the
//! `pull_request` event JSON (`$GITHUB_EVENT_PATH` in CI) and fails naming every section the PR's labels make
//! mandatory that is missing or empty. The check verifies the answers are present; the physics reviewer judges them.
//!
//! - `design`: `## Design questions`, one `###` answer per drift question (philosophy §6).
//! - `investigation`: `## Investigation`, a `- pitfalls entry:` or a `- none applies:` line with a statement
//!   (pitfalls §4.5).
//! - `validation`: `## Validation record`, one line per item (R-180): `- meter: <name> — <the quantity the occupant
//!   integrates>` (philosophy §4.5) and `- discriminator: <name> — <its dependency on termination>` (pitfalls §3).
//!
//! HTML comments, where the template keeps its guidance, are not answers.

use std::path::Path;

/// The four drift questions (philosophy §6), each a `###` heading under `## Design questions`.
pub const DRIFT_QUESTIONS: [&str; 4] = [
    "Which path is this for?",
    "Where does the constant come from?",
    "Could this measurement have failed?",
    "Does the instrument still say where it stops knowing?",
];

/// The `design` label's section.
pub const DESIGN: &str = "Design questions";
/// The `investigation` label's section.
pub const INVESTIGATION: &str = "Investigation";
/// The `validation` label's section.
pub const VALIDATION: &str = "Validation record";

/// A pull request as its event describes it.
#[derive(Debug)]
pub struct PullRequest {
    /// The description; an absent body reads as empty.
    pub body: String,
    /// The label names.
    pub labels: Vec<String>,
}

impl PullRequest {
    /// Reads the `pull_request` object of the event JSON at `path`.
    pub fn from_event(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read event {}: {e}", path.display()))?;
        let event: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| format!("event {} is not JSON: {e}", path.display()))?;
        let pr = event
            .get("pull_request")
            .ok_or_else(|| format!("event {} has no `pull_request`", path.display()))?;
        let body = pr["body"].as_str().unwrap_or_default().to_owned();
        let labels = pr["labels"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|label| label["name"].as_str().map(str::to_owned))
            .collect();
        Ok(Self { body, labels })
    }
}

/// Every problem with `body` for a PR labelled `labels`, each naming the section, question or item; empty when it
/// passes.
pub fn check(body: &str, labels: &[String]) -> Vec<String> {
    let body = strip_comments(&body.replace("\r\n", "\n"));
    let sections = split(&body, "## ");
    let mut problems = Vec::new();
    let mut mandatory = |name: &str, label: &str| {
        if labels.iter().any(|l| l == label) {
            section(&sections, name, label, &mut problems)
        } else {
            None
        }
    };
    let design = mandatory(DESIGN, "design");
    let investigation = mandatory(INVESTIGATION, "investigation");
    let validation = mandatory(VALIDATION, "validation");
    if let Some(text) = design {
        let answers = split(text, "### ");
        for question in DRIFT_QUESTIONS {
            let answered = answers
                .iter()
                .any(|(heading, answer)| heading == question && !answer.trim().is_empty());
            if !answered {
                problems.push(format!(
                    "design question `{question}` is unanswered (philosophy §6)"
                ));
            }
        }
    }
    if let Some(text) = investigation {
        let stated = ["pitfalls entry", "none applies"]
            .iter()
            .any(|key| items(text, key).any(|rest| !rest.trim().is_empty()));
        if !stated {
            problems.push(
                "investigation: neither the principia_01_pitfalls.md entry added or updated (`- pitfalls entry:`) \
                 nor the reason none applies (`- none applies:`) is given (pitfalls §4.5)"
                    .to_owned(),
            );
        }
    }
    if let Some(text) = validation {
        let kinds = [
            (
                "meter",
                "the quantity the occupant integrates (philosophy §4.5)",
            ),
            (
                "discriminator",
                "its dependency on termination (pitfalls §3)",
            ),
        ];
        for (kind, statement) in kinds {
            for rest in items(text, kind) {
                let (name, said) = rest.split_once('—').unwrap_or((rest, ""));
                if said.trim().is_empty() {
                    problems.push(format!(
                        "validation {kind} `{}` does not state {statement}: expected `- {kind}: <name> — \
                         <statement>` (R-180)",
                        name.trim()
                    ));
                }
            }
        }
    }
    problems
}

/// `cargo xtask pr-check`: checks the PR in the event at `event`, printing each problem.
pub fn run(event: &Path) -> Result<(), String> {
    let pr = PullRequest::from_event(event)?;
    let problems = check(&pr.body, &pr.labels);
    for problem in &problems {
        eprintln!("xtask pr-check: {problem}");
    }
    if problems.is_empty() {
        println!(
            "xtask pr-check: every section labels [{}] make mandatory is answered",
            pr.labels.join(", ")
        );
        Ok(())
    } else {
        Err(format!(
            "{} problem(s) in the PR description (REQ-SYS-006, REQ-VAL-001, REQ-VAL-005, REQ-VAL-008)",
            problems.len()
        ))
    }
}

/// The text of section `name`, the `label` label making it mandatory; `None`, with a problem, when it is missing or
/// empty.
fn section<'a>(
    sections: &'a [(String, String)],
    name: &str,
    label: &str,
    problems: &mut Vec<String>,
) -> Option<&'a str> {
    match sections.iter().find(|(heading, _)| heading == name) {
        None => problems.push(format!("section `{name}` is missing (label `{label}`)")),
        Some((_, text)) if text.trim().is_empty() => {
            problems.push(format!("section `{name}` is empty (label `{label}`)"))
        }
        Some((_, text)) => return Some(text),
    }
    None
}

/// `text` without its HTML comments; an unclosed comment runs to the end.
fn strip_comments(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        rest = match rest[start..].find("-->") {
            Some(end) => &rest[start + end + 3..],
            None => "",
        };
    }
    out.push_str(rest);
    out
}

/// The parts of `text` headed by lines starting with `marker` (`"## "` or `"### "`): each heading's text, trimmed, and
/// the lines under it up to the next such heading.
fn split(text: &str, marker: &str) -> Vec<(String, String)> {
    let mut parts: Vec<(String, String)> = Vec::new();
    for line in text.lines() {
        if let Some(heading) = line.strip_prefix(marker) {
            parts.push((heading.trim().to_owned(), String::new()));
        } else if let Some((_, under)) = parts.last_mut() {
            under.push_str(line);
            under.push('\n');
        }
    }
    parts
}

/// What follows `- <key>:` on each line of `text` that starts so.
fn items<'a>(text: &'a str, key: &'a str) -> impl Iterator<Item = &'a str> {
    text.lines().filter_map(move |line| {
        line.trim_start()
            .strip_prefix("- ")?
            .trim_start()
            .strip_prefix(key)?
            .strip_prefix(':')
    })
}
