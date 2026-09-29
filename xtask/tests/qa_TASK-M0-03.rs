//! QA tests for TASK-M0-03, written from the requirements it closes, not from the implementation:
//!
//! - REQ-SYS-006: a design PR answers the four drift questions of philosophy §6 (read here from the philosophy
//!   document itself, so the template and the check are held to the source's wording).
//! - REQ-VAL-001 / REQ-VAL-005 (R-180): in a validation record every `- meter: <name> — <statement>` and
//!   `- discriminator: <name> — <statement>` line states its quantity or its termination dependency; each item that
//!   does not is named.
//! - REQ-VAL-008: an investigation PR gives the pitfalls entry or the reason none applies.
//! - The task's deliverables: labels select the mandatory sections; the check reads the event JSON (`--event` or
//!   `$GITHUB_EVENT_PATH`) and re-runs on the six `pull_request` event types.
//!
//! Each test has its negative control (R-176, R-199), tripping the test's own assertion by its message (R-212).
// The file name `qa_TASK-M0-03` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use validation::negative_control;
use validation::spawn::Spawn;
use xtask::pr_check::{check, PullRequest};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn labels(names: &[&str]) -> Vec<String> {
    names.iter().map(|s| (*s).to_owned()).collect()
}

/// The four drift questions, as philosophy §6 words them (the bold text of its numbered list).
fn drift_questions() -> Vec<String> {
    let text = std::fs::read_to_string(root().join("docs/read_first/principia_00_philosophy.md"))
        .expect("philosophy readable");
    let start = text
        .find("## 6. How to tell if a decision is drifting")
        .expect("philosophy §6 present");
    let section = &text[start..];
    let end = section[3..].find("\n## ").map_or(section.len(), |e| e + 3);
    let questions: Vec<String> = section[..end]
        .lines()
        .filter_map(|line| {
            let rest = line.trim_start().split_once(". **")?.1;
            Some(rest.split_once("**")?.0.to_owned())
        })
        .collect();
    assert_eq!(
        questions.len(),
        4,
        "philosophy §6 has four questions: {questions:?}"
    );
    questions
}

/// A design body answering every drift question except the one at `skip`.
fn design_body(skip: Option<usize>, newline: &str) -> String {
    let mut body = format!(
        "## Task and Closes{newline}TASK-M0-99{newline}{newline}## Design questions{newline}"
    );
    for (i, q) in drift_questions().iter().enumerate() {
        body.push_str(&format!("### {q}{newline}"));
        if Some(i) != skip {
            body.push_str(&format!("Answer {i}: stated.{newline}"));
        }
        body.push_str(newline);
    }
    body
}

fn assert_passes(problems: &[String]) {
    assert!(
        problems.is_empty(),
        "qa: pr-check rejected a complete body: {problems:?}"
    );
}

/// Exactly `needles.len()` problems, each needle in its own one.
fn assert_names(problems: &[String], needles: &[&str]) {
    assert!(
        problems.len() == needles.len()
            && needles
                .iter()
                .all(|n| problems.iter().any(|p| p.contains(n))),
        "qa: pr-check did not name {needles:?}: {problems:?}"
    );
}

// --- REQ-SYS-006: the four drift questions ---------------------------------------------------------------

#[test]
fn qa_m003_design_every_question_answered_passes() {
    assert_passes(&check(&design_body(None, "\n"), &labels(&["design"])));
}

negative_control!(
    qa_m003_design_every_question_answered_passes,
    "a body leaving the last question unanswered must not pass",
    expected = "qa: pr-check rejected a complete body",
    assert_passes(&check(&design_body(Some(3), "\n"), &labels(&["design"])))
);

/// Each question, left unanswered alone, is the one problem and is named.
#[test]
fn qa_m003_design_each_unanswered_question_is_named() {
    let questions = drift_questions();
    for (i, q) in questions.iter().enumerate() {
        let found = check(&design_body(Some(i), "\n"), &labels(&["design"]));
        assert_names(&found, &[q]);
    }
}

negative_control!(
    qa_m003_design_each_unanswered_question_is_named,
    "a body answering every question gives no problem naming the first",
    expected = "qa: pr-check did not name",
    {
        let q = drift_questions().remove(0);
        assert_names(
            &check(&design_body(None, "\n"), &labels(&["design"])),
            &[&q],
        );
    }
);

/// An answer given only inside an HTML comment is the template's guidance, not an answer.
#[test]
fn qa_m003_design_answer_inside_comment_is_unanswered() {
    let questions = drift_questions();
    let body = design_body(Some(2), "\n").replace(
        &format!("### {}\n", questions[2]),
        &format!("### {}\n<!-- yes, it could have failed -->\n", questions[2]),
    );
    assert_names(&check(&body, &labels(&["design"])), &[&questions[2]]);
}

negative_control!(
    qa_m003_design_answer_inside_comment_is_unanswered,
    "the same answer outside the comment is an answer, leaving nothing to name",
    expected = "qa: pr-check did not name",
    {
        let questions = drift_questions();
        let body = design_body(Some(2), "\n").replace(
            &format!("### {}\n", questions[2]),
            &format!("### {}\nyes, it could have failed\n", questions[2]),
        );
        assert_names(&check(&body, &labels(&["design"])), &[&questions[2]]);
    }
);

/// GitHub stores descriptions typed in the browser with CRLF line ends.
#[test]
fn qa_m003_design_crlf_body_is_read_as_lines() {
    assert_passes(&check(&design_body(None, "\r\n"), &labels(&["design"])));
    let q = drift_questions().remove(1);
    assert_names(
        &check(&design_body(Some(1), "\r\n"), &labels(&["design"])),
        &[&q],
    );
}

negative_control!(
    qa_m003_design_crlf_body_is_read_as_lines,
    "a CRLF body with an unanswered question must not pass",
    expected = "qa: pr-check rejected a complete body",
    assert_passes(&check(&design_body(Some(0), "\r\n"), &labels(&["design"])))
);

// --- REQ-VAL-001 / REQ-VAL-005 (R-180): item-level validation record -------------------------------------

fn validation_body(items: &[&str]) -> String {
    let mut body = String::from("## Task and Closes\nTASK-M0-99\n\n## Validation record\n");
    for item in items {
        body.push_str(item);
        body.push('\n');
    }
    body.push_str("\n## Removed lines\nnone\n");
    body
}

const STATED: [&str; 4] = [
    "- meter: energy error — |E(t) − E(0)| of the relative-coordinate state the occupant integrates",
    "- meter: L_z error — the angular momentum of the advanced relative state",
    "- discriminator: d_min — depends on termination; recomputed on fully integrated runs",
    "- discriminator: outcome class — independent of termination",
];

#[test]
fn qa_m003_validation_every_item_stated_passes() {
    assert_passes(&check(&validation_body(&STATED), &labels(&["validation"])));
}

negative_control!(
    qa_m003_validation_every_item_stated_passes,
    "a record with a meter stating nothing must not pass",
    expected = "qa: pr-check rejected a complete body",
    assert_passes(&check(
        &validation_body(&[STATED[0], "- meter: COM drift —", STATED[2]]),
        &labels(&["validation"])
    ))
);

/// Every unstated item is its own problem, named: a meter with nothing after the dash, a meter with no dash, a
/// discriminator with only whitespace after the dash, a discriminator with no name or statement at all.
#[test]
fn qa_m003_validation_each_unstated_item_is_named() {
    let body = validation_body(&[
        STATED[0],
        "- meter: COM drift —",
        "- meter: momentum residual",
        STATED[2],
        "- discriminator: t_end histogram —   ",
        STATED[3],
    ]);
    let found = check(&body, &labels(&["validation"]));
    assert_names(
        &found,
        &[
            "meter `COM drift`",
            "meter `momentum residual`",
            "discriminator `t_end histogram`",
        ],
    );
    assert!(
        found.iter().all(|p| if p.contains("discriminator `") {
            p.contains("termination")
        } else {
            p.contains("integrates")
        }),
        "qa: pr-check did not name what each item must state: {found:?}"
    );
}

negative_control!(
    qa_m003_validation_each_unstated_item_is_named,
    "a record whose items all state their quantity gives nothing to name",
    expected = "qa: pr-check did not name",
    assert_names(
        &check(&validation_body(&STATED), &labels(&["validation"])),
        &[
            "meter `COM drift`",
            "meter `momentum residual`",
            "discriminator `t_end histogram`"
        ]
    )
);

/// A validation-labelled PR with no Validation record, or an empty one, fails naming the section.
#[test]
fn qa_m003_validation_section_missing_or_empty_is_named() {
    let missing = check("## Task and Closes\nTASK-M0-99\n", &labels(&["validation"]));
    assert_names(&missing, &["Validation record"]);
    let empty = check(
        "## Validation record\n<!-- guidance only -->\n\n## Removed lines\nnone\n",
        &labels(&["validation"]),
    );
    assert_names(&empty, &["Validation record"]);
}

negative_control!(
    qa_m003_validation_section_missing_or_empty_is_named,
    "a record with a stated meter gives no section problem",
    expected = "qa: pr-check did not name",
    assert_names(
        &check(&validation_body(&STATED[..1]), &labels(&["validation"])),
        &["Validation record"]
    )
);

// --- REQ-VAL-008: pitfalls entry or the reason none applies -------------------------------------------------

fn investigation_body(line: &str) -> String {
    format!(
        "## Task and Closes\nTASK-M0-99\n\n## Investigation\n{line}\n\n## Removed lines\nnone\n"
    )
}

#[test]
fn qa_m003_investigation_blank_entry_or_reason_fails() {
    for line in [
        "- pitfalls entry:",
        "- none applies:   ",
        "the mechanism is interesting",
    ] {
        let found = check(&investigation_body(line), &labels(&["investigation"]));
        assert_names(&found, &["investigation"]);
    }
}

negative_control!(
    qa_m003_investigation_blank_entry_or_reason_fails,
    "an entry with its statement gives nothing to name",
    expected = "qa: pr-check did not name",
    assert_names(
        &check(
            &investigation_body(
                "- pitfalls entry: §4.6 added — observed / assumed / actual / measurement"
            ),
            &labels(&["investigation"])
        ),
        &["investigation"]
    )
);

#[test]
fn qa_m003_investigation_entry_or_reason_passes() {
    for line in [
        "- pitfalls entry: §4.6 added — observed / assumed / actual / measurement",
        "- none applies: the investigation found no mechanism, only a mis-set fixture",
    ] {
        assert_passes(&check(
            &investigation_body(line),
            &labels(&["investigation"]),
        ));
    }
}

negative_control!(
    qa_m003_investigation_entry_or_reason_passes,
    "a reason line with nothing after the colon must not pass",
    expected = "qa: pr-check rejected a complete body",
    assert_passes(&check(
        &investigation_body("- none applies:"),
        &labels(&["investigation"])
    ))
);

// --- Labels select the mandatory sections --------------------------------------------------------------

/// Only the labelled sections are checked: a body lacking the design and validation sections passes under
/// `investigation`, and fails naming only them when every label is set.
#[test]
fn qa_m003_labels_select_only_their_sections() {
    let body = investigation_body("- none applies: a fixture typo");
    assert_passes(&check(&body, &labels(&["investigation"])));
    assert_passes(&check(&body, &labels(&["bug", "documentation"])));
    let all = check(&body, &labels(&["design", "investigation", "validation"]));
    assert_names(&all, &["Design questions", "Validation record"]);
}

negative_control!(
    qa_m003_labels_select_only_their_sections,
    "under the design label the body lacks the design section, so it must not pass",
    expected = "qa: pr-check rejected a complete body",
    assert_passes(&check(
        &investigation_body("- none applies: a fixture typo"),
        &labels(&["design"])
    ))
);

// --- The event, the binary and the workflow -----------------------------------------------------------------

fn write_event(tag: &str, body: serde_json::Value, label_names: &[&str]) -> PathBuf {
    let event = serde_json::json!({
        "action": "edited",
        "pull_request": {
            "number": 1,
            "body": body,
            "labels": label_names.iter().map(|n| serde_json::json!({ "name": n })).collect::<Vec<_>>(),
        }
    });
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("qa_m003_{tag}.json"));
    std::fs::write(&path, serde_json::to_vec_pretty(&event).unwrap()).unwrap();
    path
}

/// A PR opened with no description has a `null` body: under a label, the mandatory section is missing.
#[test]
fn qa_m003_null_body_under_a_label_fails() {
    let pr = PullRequest::from_event(&write_event("null", serde_json::Value::Null, &["design"]))
        .expect("event read");
    assert_names(&check(&pr.body, &pr.labels), &["Design questions"]);
}

negative_control!(
    qa_m003_null_body_under_a_label_fails,
    "a complete design body gives no missing section to name",
    expected = "qa: pr-check did not name",
    {
        let path = write_event("null_control", design_body(None, "\n").into(), &["design"]);
        let pr = PullRequest::from_event(&path).expect("event read");
        assert_names(&check(&pr.body, &pr.labels), &["Design questions"]);
    }
);

fn xtask(args: &[&str], event_env: Option<&Path>) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_xtask"));
    cmd.args(args).env_remove("GITHUB_EVENT_PATH");
    if let Some(path) = event_env {
        cmd.env("GITHUB_EVENT_PATH", path);
    }
    cmd.timed_output().expect("xtask ran")
}

fn assert_exit_fails_naming(out: &Output, needles: &[&str]) {
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !out.status.success() && needles.iter().all(|n| stderr.contains(n)),
        "qa: xtask pr-check did not fail naming {needles:?}: {stderr}"
    );
}

/// In CI the event comes from `$GITHUB_EVENT_PATH`; the binary exits non-zero naming every problem, and zero on a
/// complete body. With neither `--event` nor the variable it fails rather than passing.
#[test]
fn qa_m003_binary_reads_github_event_path() {
    let body = validation_body(&["- meter: COM drift —", "- discriminator: d_min"]);
    let bad = write_event("env_bad", body.into(), &["validation"]);
    assert_exit_fails_naming(
        &xtask(&["pr-check"], Some(&bad)),
        &["meter `COM drift`", "discriminator `d_min`"],
    );
    let good = write_event("env_good", validation_body(&STATED).into(), &["validation"]);
    let out = xtask(&["pr-check"], Some(&good));
    assert!(
        out.status.success(),
        "qa: complete body failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_exit_fails_naming(&xtask(&["pr-check"], None), &["GITHUB_EVENT_PATH"]);
}

negative_control!(
    qa_m003_binary_reads_github_event_path,
    "the binary on a complete body exits zero, so it cannot be failing naming the items",
    expected = "qa: xtask pr-check did not fail naming",
    {
        let good = write_event(
            "env_control",
            validation_body(&STATED).into(),
            &["validation"],
        );
        assert_exit_fails_naming(&xtask(&["pr-check"], Some(&good)), &["meter"]);
    }
);

const EVENT_TYPES: [&str; 6] = [
    "opened",
    "edited",
    "synchronize",
    "reopened",
    "labeled",
    "unlabeled",
];

/// The workflow runs `cargo xtask pr-check` on `pull_request` of all six types, so a relabel re-runs it.
fn check_workflow(text: &str) {
    let types = text
        .lines()
        .find_map(|l| l.trim().strip_prefix("types:"))
        .unwrap_or("");
    let listed: Vec<&str> = types
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(str::trim)
        .collect();
    assert!(
        text.contains("pull_request:")
            && text.contains("cargo xtask pr-check")
            && EVENT_TYPES.iter().all(|t| listed.contains(t)),
        "qa: workflow does not run pr-check on every event type: {listed:?}"
    );
}

#[test]
fn qa_m003_workflow_reruns_on_every_event_type() {
    let text =
        std::fs::read_to_string(root().join(".github/workflows/pr-check.yml")).expect("workflow");
    check_workflow(&text);
}

negative_control!(
    qa_m003_workflow_reruns_on_every_event_type,
    "a workflow that does not re-run on unlabeled misses a reviewer's relabel",
    expected = "qa: workflow does not run pr-check on every event type",
    {
        let text = std::fs::read_to_string(root().join(".github/workflows/pr-check.yml"))
            .expect("workflow");
        check_workflow(&text.replace(", unlabeled", ""));
    }
);

// --- The template -------------------------------------------------------------------------------------------

/// The template, answered under each of its headings and given its item lines, passes under every label; so the
/// template asks what the check requires, in philosophy §6's words.
fn fill(template: &str) -> String {
    let mut out = String::new();
    for line in template.lines() {
        out.push_str(line);
        out.push('\n');
        if line.starts_with("### ") {
            out.push_str("An answer.\n");
        } else if line == "## Investigation" {
            out.push_str("- none applies: no mechanism found\n");
        } else if line == "## Validation record" {
            out.push_str(&STATED.join("\n"));
            out.push('\n');
        }
    }
    out
}

fn template() -> String {
    std::fs::read_to_string(root().join(".github/pull_request_template.md")).expect("template")
}

#[test]
fn qa_m003_filled_template_passes_every_label() {
    let t = template();
    for q in drift_questions() {
        assert!(
            t.contains(&format!("### {q}\n")),
            "qa: template lacks philosophy §6's `{q}`"
        );
    }
    for s in [
        "## Task and Closes",
        "## Removed lines",
        "- meter: <name> — ",
        "- discriminator: <name> — ",
    ] {
        assert!(t.contains(s), "qa: template lacks `{s}`");
    }
    assert_passes(&check(
        &fill(&t),
        &labels(&["design", "investigation", "validation"]),
    ));
}

negative_control!(
    qa_m003_filled_template_passes_every_label,
    "the unfilled template answers nothing, so it must not pass",
    expected = "qa: pr-check rejected a complete body",
    assert_passes(&check(
        &template(),
        &labels(&["design", "investigation", "validation"])
    ))
);
