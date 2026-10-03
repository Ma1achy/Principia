//! QA tests for R-374 as TASK-M0-19's PR builds it: "Every CI job sets `CARGO_PROFILE_DEV_DEBUG=line-tables-only` and
//! `CARGO_PROFILE_TEST_DEBUG=line-tables-only`, not only the mutants and nightly jobs" (decisions.md § "R-374 — R-233
//! governs local builds only; every CI job uses line-tables-only debug info"). Read from the workflow files: each sets
//! both at the workflow's top-level `env:`, so every job in it inherits them, and no job, step or command sets either
//! to anything else. Each test has its negative control (R-176).
// The file name `qa_TASK-M0-19_r374` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::path::{Path, PathBuf};

use validation::negative_control;

const KEYS: [&str; 2] = ["CARGO_PROFILE_DEV_DEBUG", "CARGO_PROFILE_TEST_DEBUG"];
const VALUE: &str = "line-tables-only";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// Every workflow file, as (name, text).
fn workflows() -> Vec<(String, String)> {
    let mut files: Vec<(String, String)> = std::fs::read_dir(root().join(".github/workflows"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| matches!(p.extension().and_then(|e| e.to_str()), Some("yml" | "yaml")))
        .map(|p| {
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read_to_string(&p).unwrap(),
            )
        })
        .collect();
    files.sort();
    files
}

/// A line with its comment removed (a `#` at the line's start or after whitespace).
fn uncommented(l: &str) -> &str {
    let cut = l
        .char_indices()
        .find(|&(i, c)| c == '#' && (i == 0 || l[..i].ends_with(char::is_whitespace)))
        .map_or(l.len(), |(i, _)| i);
    l[..cut].trim_end()
}

/// The `key: value` pairs of the workflow's top-level `env:` block (its lines indented by two spaces).
fn top_env(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut inside = false;
    for l in text.lines().map(uncommented) {
        if l.is_empty() {
            continue;
        }
        if !l.starts_with(' ') {
            inside = l == "env:";
            continue;
        }
        if inside && l.starts_with("  ") && !l.starts_with("   ") {
            if let Some((k, v)) = l.trim().split_once(':') {
                out.push((
                    k.trim().to_owned(),
                    v.trim().trim_matches(['"', '\'']).to_owned(),
                ));
            }
        }
    }
    out
}

/// Each workflow sets both keys to line tables only at its top level, and nothing else in it (a job's or step's `env:`,
/// a command's assignment or `--config`) names either key or sets the profiles' debug info.
fn check(files: &[(String, String)]) {
    assert!(
        files.len() >= 8,
        "expected every workflow of .github/workflows, found {}",
        files.len()
    );
    for (name, text) in files {
        let env = top_env(text);
        for key in KEYS {
            let set: Vec<&String> = env
                .iter()
                .filter(|(k, _)| k == key)
                .map(|(_, v)| v)
                .collect();
            assert_eq!(
                set,
                [VALUE],
                "{name}'s top-level env does not set {key} to {VALUE} once"
            );
        }
        let mut inside = false;
        for l in text.lines().map(uncommented) {
            if !l.is_empty() && !l.starts_with(' ') {
                inside = l == "env:";
                continue;
            }
            if inside && l.starts_with("  ") && !l.starts_with("   ") {
                continue;
            }
            assert!(
                !KEYS.iter().any(|k| l.contains(k)) && !l.contains("debug="),
                "{name} sets the debug info again below its top-level env: `{}`",
                l.trim()
            );
        }
    }
}

#[test]
fn qa_r374_every_workflow_builds_with_line_tables_only() {
    check(&workflows());
}

/// The workflows with `from` replaced by `to` in `file`.
#[cfg(feature = "controls")]
fn edited(file: &str, from: &str, to: &str) -> Vec<(String, String)> {
    let mut files = workflows();
    let (_, text) = files
        .iter_mut()
        .find(|(f, _)| f == file)
        .unwrap_or_else(|| panic!("there is no .github/workflows/{file}"));
    assert!(text.contains(from), "{file} has no `{from}` to edit");
    *text = text.replacen(from, to, 1);
    files
}

negative_control!(
    qa_r374_every_workflow_builds_with_line_tables_only,
    "a workflow whose top-level env leaves the test profile's debug info at the default",
    expected = "pr-check.yml's top-level env does not set CARGO_PROFILE_TEST_DEBUG",
    check(&edited(
        "pr-check.yml",
        "  CARGO_PROFILE_TEST_DEBUG: line-tables-only\n",
        ""
    ))
);

#[cfg(feature = "controls")]
mod full_debug {
    use super::*;

    negative_control!(
        qa_r374_every_workflow_builds_with_line_tables_only,
        "a workflow that builds the dev profile with full debug info",
        expected = "screenshot.yml's top-level env does not set CARGO_PROFILE_DEV_DEBUG",
        check(&edited(
            "screenshot.yml",
            "CARGO_PROFILE_DEV_DEBUG: line-tables-only",
            "CARGO_PROFILE_DEV_DEBUG: full"
        ))
    );
}

#[cfg(feature = "controls")]
mod job_override {
    use super::*;

    negative_control!(
        qa_r374_every_workflow_builds_with_line_tables_only,
        "a job that sets the debug info back in its own env",
        expected = "ci.yml sets the debug info again below its top-level env",
        check(&edited(
            "ci.yml",
            "\njobs:\n",
            "\njobs:\n  extra:\n    env:\n      CARGO_PROFILE_DEV_DEBUG: true\n"
        ))
    );
}
