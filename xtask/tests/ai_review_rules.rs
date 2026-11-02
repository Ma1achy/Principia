//! The advisory AI reviewers' brief cannot drift (decisions.md R-635): `GEMINI.md` and
//! `.github/copilot-instructions.md` each carry a copy of `AGENTS.md` § "Code Review Rules", and each copy must be
//! byte-identical to it. Codex reads `AGENTS.md`; the Gemini review's script reads the section from `AGENTS.md`; GitHub
//! Copilot's code review reads `.github/copilot-instructions.md`. One brief, three readers.
//!
//! One delimiting rule on every side: the section starts at the line that is exactly `## Code Review Rules` and runs
//! up to, not including, the next line that starts with `## ` (a level-2 heading, not `### `), or to the end of the
//! file. Each test has its registered negative controls (R-176): one byte of the copy changed, a line appended past
//! the section's end, and the copy cut short by a level-2 heading inside it.

use std::path::{Path, PathBuf};

use validation::negative_control;

const HEADING: &str = "## Code Review Rules";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path))
        .unwrap_or_else(|e| panic!("ai_review_rules: {path} not read: {e}"))
}

/// The section of `text` under `HEADING`, by the rule above, with the line endings it has in the file.
fn section<'a>(text: &'a str, file: &str) -> &'a str {
    let mut start = None;
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let bare = line.trim_end_matches(['\n', '\r']);
        match start {
            None if bare == HEADING => start = Some(offset),
            Some(s) if bare.starts_with("## ") => return &text[s..offset],
            _ => {}
        }
        offset += line.len();
    }
    match start {
        Some(s) => &text[s..],
        None => panic!("ai_review_rules: {file} has no `{HEADING}` section"),
    }
}

/// Fails unless `copy`'s section is byte-identical to `agents`'s, naming `file` and the first line that differs.
fn check_same(agents: &str, copy: &str, file: &str) {
    let a = section(agents, "AGENTS.md");
    let c = section(copy, file);
    if a != c {
        let line = a
            .lines()
            .zip(c.lines())
            .position(|(x, y)| x != y)
            .unwrap_or_else(|| a.lines().count().min(c.lines().count()));
        panic!(
            "ai_review_rules: {file}'s copy differs from AGENTS.md § \"Code Review Rules\", first at line {} of the \
             section; copy the section from AGENTS.md",
            line + 1
        );
    }
}

#[cfg(feature = "controls")]
/// `file`'s text with `from` replaced by `to` once, failing if `from` is gone, so a control cannot pass vacuously.
fn edited(file: &str, from: &str, to: &str) -> String {
    let text = read(file);
    assert!(
        text.contains(from),
        "ai_review_rules: the control's edit target is gone"
    );
    text.replacen(from, to, 1)
}

const GEMINI: &str = "GEMINI.md";
const COPILOT: &str = ".github/copilot-instructions.md";

#[cfg(feature = "controls")]
/// The controls' edits: one byte of a severity; a `### ` heading raised to `## `, which cuts the copy short; and a
/// line appended past the section's end.
const SEVERITY: (&str, &str) = (
    "| A correctness bug | P1 / HIGH |",
    "| A correctness bug | P2 / HIGH |",
);
#[cfg(feature = "controls")]
const HEADING_RAISED: (&str, &str) = ("\n### Severity\n", "\n## Severity\n");
#[cfg(feature = "controls")]
const APPENDED: &str = "- Also flag long functions.\n";

#[test]
fn ai_review_rules_gemini_md_copy_matches_agents_md() {
    check_same(&read("AGENTS.md"), &read(GEMINI), GEMINI);
}

negative_control!(
    ai_review_rules_gemini_md_copy_matches_agents_md,
    "one byte of GEMINI.md's copy changed: a severity moved from P1 to P2",
    expected = "ai_review_rules: GEMINI.md's copy differs from AGENTS.md",
    check_same(
        &read("AGENTS.md"),
        &edited(GEMINI, SEVERITY.0, SEVERITY.1),
        GEMINI
    )
);

#[test]
fn ai_review_rules_copilot_instructions_copy_matches_agents_md() {
    check_same(&read("AGENTS.md"), &read(COPILOT), COPILOT);
}

negative_control!(
    ai_review_rules_copilot_instructions_copy_matches_agents_md,
    "one byte of .github/copilot-instructions.md's copy changed: a severity moved from P1 to P2",
    expected = "ai_review_rules: .github/copilot-instructions.md's copy differs from AGENTS.md",
    check_same(
        &read("AGENTS.md"),
        &edited(COPILOT, SEVERITY.0, SEVERITY.1),
        COPILOT
    )
);

mod trailing {
    use super::*;
    negative_control!(
        ai_review_rules_gemini_md_copy_matches_agents_md,
        "a line appended to GEMINI.md's copy, past the end of AGENTS.md's section",
        expected = "ai_review_rules: GEMINI.md's copy differs from AGENTS.md",
        check_same(
            &read("AGENTS.md"),
            &format!("{}{APPENDED}", read(GEMINI)),
            GEMINI
        )
    );
    negative_control!(
        ai_review_rules_copilot_instructions_copy_matches_agents_md,
        "a line appended to .github/copilot-instructions.md's copy, past the end of AGENTS.md's section",
        expected = "ai_review_rules: .github/copilot-instructions.md's copy differs from AGENTS.md",
        check_same(
            &read("AGENTS.md"),
            &format!("{}{APPENDED}", read(COPILOT)),
            COPILOT
        )
    );
}

mod truncated {
    use super::*;
    negative_control!(
        ai_review_rules_gemini_md_copy_matches_agents_md,
        "GEMINI.md's copy cut short by a level-2 heading inside it",
        expected = "ai_review_rules: GEMINI.md's copy differs from AGENTS.md",
        check_same(
            &read("AGENTS.md"),
            &edited(GEMINI, HEADING_RAISED.0, HEADING_RAISED.1),
            GEMINI
        )
    );
    negative_control!(
        ai_review_rules_copilot_instructions_copy_matches_agents_md,
        ".github/copilot-instructions.md's copy cut short by a level-2 heading inside it",
        expected = "ai_review_rules: .github/copilot-instructions.md's copy differs from AGENTS.md",
        check_same(
            &read("AGENTS.md"),
            &edited(COPILOT, HEADING_RAISED.0, HEADING_RAISED.1),
            COPILOT
        )
    );
}
