//! qa's tests for R-790, written from the ruling and the owner's words, not from `scripts/ci_changes.sh`'s code.
//!
//! The owner, 4 Nov 2026: "R-677 amendment: .claude/ (commands, agents, settings) is docs tier." R-790: the whole
//! directory is tier (c), a change only to it runs only the cheap checks; a change beside code or data takes the
//! highest tier, as R-677 has it; it has no carve-out in `reads_code`; `.claude` itself and `.clauderc` run full CI.
//!
//! - `qa_r790_claude_is_docs_tier`: the classifier's whole verdict, every output, on a change only to `.claude/`, at
//!   any depth and of any file type; `xtask/tests/qa_R-662_ci_scope.rs` checks the tracked files, the required checks
//!   on the cheap path and `detect` on real merge commits.
//! - `qa_r790_a_change_takes_its_highest_tier`: `.claude/` beside each other tier, on either side of it.
//! - `qa_r790_only_the_directory_is_listed`: what only looks like it runs full CI, alone and beside `.claude/`.
//!
//! Each test has its negative control (R-176).

use std::path::{Path, PathBuf};
use std::process::Command;

use validation::negative_control;
use validation::spawn::Spawn;

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn script() -> String {
    std::fs::read_to_string(root().join("scripts/ci_changes.sh")).expect("qa_r790: the classifier")
}

/// `text` with `from` replaced once by `to`; `from` must be there, so a control cannot pass vacuously.
#[cfg(feature = "controls")]
fn swap(text: &str, from: &str, to: &str) -> String {
    assert!(
        text.contains(from),
        "qa_r790: the control's edit target is gone: {from}"
    );
    text.replacen(from, to, 1)
}

/// The classifier's six outputs on one change of `paths`, in the order it prints them.
fn verdict(script: &str, paths: &[&str]) -> Vec<String> {
    let dir = Scratch::new("qa_r790_classify");
    std::fs::create_dir_all(&*dir).expect("scratch");
    let file = dir.join("ci_changes.sh");
    let input = dir.join("paths");
    std::fs::write(&file, script).expect("script written");
    std::fs::write(&input, paths.join("\n")).expect("paths written");
    let out = Command::new("bash")
        .args(["-c", "bash \"$0\" classify < \"$1\""])
        .arg(&file)
        .arg(&input)
        .timed_output()
        .expect("qa_r790: bash ran");
    assert!(out.status.success(), "qa_r790: classify failed");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_owned)
        .collect()
}

/// What a tier runs: `(tier, docs_only, gpu, plan_data, gui_data, mutants)`.
type Run = (&'static str, bool, bool, bool, bool, bool);

/// That, as the classifier prints it.
fn lines(want: Run) -> Vec<String> {
    let (tier, docs_only, gpu, plan, gui, mutants) = want;
    vec![
        format!("tier={tier}"),
        format!("docs_only={docs_only}"),
        format!("gpu={gpu}"),
        format!("plan_data={plan}"),
        format!("gui_data={gui}"),
        format!("mutants={mutants}"),
    ]
}

/// R-790's "commands, agents, settings" and "the whole directory": what is under it, whatever it is.
const CLAUDE: &[&str] = &[
    ".claude/commands/qa.md",
    ".claude/agents/qa.md",
    ".claude/settings.json",
    ".claude/settings.local.json",
    ".claude/commands/deep/er/qa.md",
    ".claude/hooks/qa.sh",
    // Rust that isn't under `crates/` has no mutants (R-677), and this folder holds no code (R-790).
    ".claude/qa/a.rs",
    ".claude/q a.md",
];

fn check_docs(script: &str) {
    let docs = lines(("docs", true, false, false, false, false));
    for path in CLAUDE {
        assert_eq!(
            verdict(script, &[path]),
            docs,
            "qa_r790: {path} alone is not the docs tier"
        );
    }
    assert_eq!(
        verdict(script, CLAUDE),
        docs,
        "qa_r790: all of `.claude/` together is not the docs tier"
    );
    assert_eq!(
        verdict(
            script,
            &[".claude/agents/qa.md", "docs/qa_note.md", "CLAUDE.md"]
        ),
        docs,
        "qa_r790: `.claude/` beside the docs is not the docs tier"
    );
}

#[test]
fn qa_r790_claude_is_docs_tier() {
    check_docs(&script());
}

negative_control!(
    qa_r790_claude_is_docs_tier,
    "the script with `.claude/` off its allowlist",
    expected = "qa_r790: .claude/commands/qa.md alone is not the docs tier",
    check_docs(&swap(
        &script(),
        " | .claude/*) return 0 ;;",
        ") return 0 ;;"
    ))
);

mod settings_carved_out {
    use super::*;
    negative_control!(
        qa_r790_claude_is_docs_tier,
        "the script with a `reads_code` carve-out for `.claude/settings.json`, which R-790 rules out",
        expected = "qa_r790: .claude/settings.json alone is not the docs tier",
        check_docs(&swap(
            &script(),
            "    plan/requirements.yaml) return 0 ;;\n",
            "    plan/requirements.yaml | .claude/settings.json) return 0 ;;\n"
        ))
    );
}

/// (the other path, `.claude/` beside it runs as this).
const MIXED: &[(&str, Run)] = &[
    // Tier (b).
    (
        "plan/tasks/M9/TASK-M9-98.md",
        ("data", true, false, true, false, false),
    ),
    (
        "docs/gui/design/qa.png",
        ("data", true, false, false, true, false),
    ),
    // Tier (a), a crate the GPU jobs read, one they don't, and the rest of the code.
    (
        "crates/kernel/src/qa.rs",
        ("code", false, true, false, false, true),
    ),
    (
        "crates/gui/src/qa.rs",
        ("code", false, false, false, false, true),
    ),
    (
        ".github/workflows/ci.yml",
        ("code", false, true, false, false, false),
    ),
    (
        "scripts/ci_changes.sh",
        ("code", false, true, false, false, false),
    ),
    ("newtop/a.md", ("code", false, true, false, false, false)),
];

fn check_mixed(script: &str) {
    for (other, want) in MIXED {
        let want = lines(*want);
        assert_eq!(
            verdict(script, &[".claude/agents/qa.md", other]),
            want,
            "qa_r790: `.claude/` then {other} does not take the highest tier"
        );
        assert_eq!(
            verdict(script, &[other, ".claude/agents/qa.md"]),
            want,
            "qa_r790: {other} then `.claude/` does not take the highest tier"
        );
    }
}

#[test]
fn qa_r790_a_change_takes_its_highest_tier() {
    check_mixed(&script());
}

negative_control!(
    qa_r790_a_change_takes_its_highest_tier,
    "the script letting a later path lower the tier",
    expected = "qa_r790: plan/tasks/M9/TASK-M9-98.md then `.claude/`",
    check_mixed(&swap(
        &script(),
        "      data) if [ \"$tier\" = docs ]; then tier=data; fi ;;\n    esac\n",
        "      data) if [ \"$tier\" = docs ]; then tier=data; fi ;;\n      docs) tier=docs ;;\n    esac\n"
    ))
);

/// What only looks like the directory, or holds its name below another path.
const NEIGHBOURS: &[&str] = &[
    ".claude",
    ".clauderc",
    ".claude.json",
    ".claude2/a.md",
    ".claude-old/a.md",
    ".CLAUDE/a.md",
    "claude/a.md",
    "a/.claude/b.md",
    "crates/.claude/a.md",
    "xtask/.claude/a.md",
    "fixtures/.claude/a.md",
    "./.claude/a.md",
];

fn check_neighbours(script: &str) {
    for path in NEIGHBOURS {
        for paths in [
            [*path].as_slice(),
            [".claude/agents/qa.md", path].as_slice(),
        ] {
            let got = verdict(script, paths);
            assert!(
                got.iter().any(|l| l == "tier=code") && got.iter().any(|l| l == "docs_only=false"),
                "qa_r790: {paths:?} does not run full CI: {got:?}"
            );
        }
    }
}

#[test]
fn qa_r790_only_the_directory_is_listed() {
    check_neighbours(&script());
}

negative_control!(
    qa_r790_only_the_directory_is_listed,
    "the script's allowlist taking whatever starts like `.claude`",
    expected = "qa_r790: [\".claude\"] does not run full CI",
    check_neighbours(&swap(&script(), " | .claude/*)", " | .claude*)"))
);

mod below_another_path {
    use super::*;
    negative_control!(
        qa_r790_only_the_directory_is_listed,
        "the script's allowlist taking `.claude/` at any depth",
        expected = "qa_r790: [\"a/.claude/b.md\"] does not run full CI",
        check_neighbours(&swap(
            &script(),
            " | .claude/*)",
            " | .claude/* | */.claude/*)"
        ))
    );
}
