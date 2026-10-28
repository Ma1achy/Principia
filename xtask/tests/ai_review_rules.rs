//! The advisory AI reviewers' brief cannot drift (decisions.md R-635): `.gemini/styleguide.md` carries a copy of
//! `AGENTS.md` § "Code Review Rules", and the two sections must be byte-identical. Codex reads `AGENTS.md`; Gemini Code
//! Assist reads the style guide.
//!
//! One delimiting rule on both sides: the section starts at the line that is exactly `## Code Review Rules` and runs
//! up to, not including, the next line that starts with `## ` (a level-2 heading, not `### `), or to the end of the
//! file. Each test has a registered negative control (R-176).

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

/// Fails unless the two files' sections are byte-identical, naming the first line that differs.
fn check_same(agents: &str, styleguide: &str) {
    let a = section(agents, "AGENTS.md");
    let g = section(styleguide, ".gemini/styleguide.md");
    if a != g {
        let line = a
            .lines()
            .zip(g.lines())
            .position(|(x, y)| x != y)
            .unwrap_or_else(|| a.lines().count().min(g.lines().count()));
        panic!(
            "ai_review_rules: .gemini/styleguide.md's copy differs from AGENTS.md § \"Code Review Rules\", first at \
             line {} of the section; copy the section from AGENTS.md",
            line + 1
        );
    }
}

#[test]
fn ai_review_rules_styleguide_copy_matches_agents_md() {
    check_same(&read("AGENTS.md"), &read(".gemini/styleguide.md"));
}

negative_control!(
    ai_review_rules_styleguide_copy_matches_agents_md,
    "one byte of the style guide's copy changed: a severity moved from P1 to P2",
    expected = "ai_review_rules: .gemini/styleguide.md's copy differs from AGENTS.md",
    check_same(&read("AGENTS.md"), &{
        let guide = read(".gemini/styleguide.md");
        assert!(
            guide.contains("- P1 / HIGH: any correctness bug"),
            "ai_review_rules: the control's edit target is gone"
        );
        guide.replacen(
            "- P1 / HIGH: any correctness bug",
            "- P2 / HIGH: any correctness bug",
            1,
        )
    })
);

mod trailing {
    use super::*;
    negative_control!(
        ai_review_rules_styleguide_copy_matches_agents_md,
        "a line appended to the style guide's copy, past the end of AGENTS.md's section",
        expected = "ai_review_rules: .gemini/styleguide.md's copy differs from AGENTS.md",
        check_same(
            &read("AGENTS.md"),
            &format!(
                "{}- Also flag long functions.\n",
                read(".gemini/styleguide.md")
            )
        )
    );
}

mod truncated {
    use super::*;
    negative_control!(
        ai_review_rules_styleguide_copy_matches_agents_md,
        "the style guide's copy cut short by a level-2 heading inside it",
        expected = "ai_review_rules: .gemini/styleguide.md's copy differs from AGENTS.md",
        check_same(&read("AGENTS.md"), &{
            let guide = read(".gemini/styleguide.md");
            assert!(
                guide.contains("\n### Severity\n"),
                "ai_review_rules: the control's edit target is gone"
            );
            guide.replacen("\n### Severity\n", "\n## Severity\n", 1)
        })
    );
}
