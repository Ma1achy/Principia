//! R-656, the owner's words of 30 Oct 2026: "Constraint for 2 and 3: required checks must still report. Do the GPU and
//! mutants skips inside the job (changed-files detection, then skip as success under the same check name), not with
//! workflow-level `paths:`, or required checks will hang pending on docs-only PRs."
//!
//! R-662, the owner's words of 31 Oct 2026: "Yeah and if it's like python don't check? Only the actual Principia CODE
//! matters in ci? If you get me. So GUI and Physics etc for Principia, not random doc stuff".
//!
//! R-677, the owner's words of 2 Nov 2026: "a three-tier classifier in scripts/ci_changes.sh. (a) CODE … → full CI as
//! now. (b) PLAN-DATA: the reads_code files … → only the checks that read them … (c) DOCS: as now, cheap checks only.
//! Mutants run only when crates/ source changes, on the merge-ready head."
//!
//! - `ci_changes_classifies_conservatively`: `scripts/ci_changes.sh classify` gives each change its tier, the highest
//!   of its paths': docs only when every path is on its allowlist, the corpus, the plan's prose and Python, the scripts
//!   and the workbench; data when the plan's task data or the GUI's artboards and references are among them; code
//!   otherwise. It skips the GPU suites only when no path is a GPU path, and runs mutants only when a `.rs` file under
//!   `crates/` changes. An empty change, a workflow, an unknown top-level path and a path that merely starts like an
//!   allowed one run everything, and so does a mixed change; `docs_only` is true exactly on a data or docs change.
//! - `ci_changes_files_code_reads_run_everything`: each pattern of the script's `reads_code` list, the data only full
//!   CI reads, the classifier itself (R-662) and the containment, is touched through real files it names, each
//!   classifying as code: not docs-only, GPU suites run.
//! - `ci_changes_data_lands_in_tier_b`: each pattern of the script's `plan_data` and `gui_data` lists (R-677), touched
//!   through real files it names, classifies as data, with its own flag, no GPU suite and no mutants.
//! - `ci_changes_a_script_runs_go_through_is_code`: a script named as a workflow step's `shell:` or as nextest's wrapper
//!   runs that step or test inside it, so a wrong edit to it must run them: each classifies as code.
//! - `ci_changes_no_file_code_reads_is_docs_only`: a static scan, best effort, of the string literals in every crate's
//!   `src/`, `tests/` and `build.rs`, and in `xtask/`, for a data file they name, and of their `src/` for the plan's
//!   markdown; none may classify as docs, so a new reader cannot silently leave its data in tier (c). Prose a test
//!   checks, and a test of a script, are not data (R-662). A `/`-led literal, the tail of
//!   `concat!(env!("CARGO_MANIFEST_DIR"), "/../../…")`, is read from its package directory. Its limit: a reader
//!   walking a directory, or building a path from more than one literal, is not seen; the script lists those by hand.
//! - `ci_changes_data_tier_runs_every_reader`: on a data change `ci-workspace` runs the tests of every reader of what
//!   changed, as that scan finds them and as the walkers listed here by hand read them: each test file reading tier
//!   (b)'s data, and each xtask module's own tests, is among the `--test`s of the step its flag runs (R-677).
//! - `ci_changes_gates_pass_only_on_the_verdict`: the gates `ci`, `xtask-ci`, `gpu-kernel`, `gpu-metal` and
//!   `gpu-lavapipe` run whatever their jobs ended in, read every one's result and a flag set from the classification,
//!   and (their one-line scripts, run under bash) pass a job skipped on that verdict, and fail when the
//!   classification failed, when a job its verdict runs was skipped, and on any failed or cancelled job.
//! - `ci_changes_jobs_skip_on_the_verdict`: the `changes` job runs the classifier and cannot be skipped; the build,
//!   test and GPU jobs wait on it alone and skip on its verdict (data or docs, or no GPU-path file); `ci-workspace`
//!   runs its tests or, on a data or docs pull request, the cheap checks, and on a data one its readers' tests, built
//!   one test binary at a time, and passes the verdict to the gate `ci`.
//! - `ci_changes_no_skipped_job_reports_a_required_name`: every workflow that runs on pull requests has no
//!   workflow-level `paths:` or `paths-ignore:`; the required checks are each reported by exactly one job, but
//!   `mutants-check`, which `mutants.yml` posts and no job bears (R-656, the merge-ready run); a job that
//!   can be skipped (a job-level `if:` or a `needs:`) is never named for a required check unless its `if:` is
//!   `always()`; and a matrix job never bears a required check's name without its shard.
//!
//! Each test has its registered negative control (R-176), which feeds the same check a script or workflow differing in
//! the one respect it turns on. The workflows are parsed with PyYAML, as `plan/check_plan.py` and the qa tests that read
//! the workflows parse them.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use validation::negative_control;
use validation::spawn::Spawn;

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;

/// The checks branch protection requires on `main` (R-266, R-305, and `gpu-kernel` since TASK-M0-14).
const REQUIRED: [&str; 8] = [
    "ci",
    "gpu-metal",
    "gpu-lavapipe",
    "pr-check",
    "reviews-complete",
    "xtask-ci",
    "mutants-check",
    "gpu-kernel",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("ci_changes: {rel}: {e}"))
}

#[cfg(feature = "controls")]
/// `rel`'s text with `from` replaced by `to` once, failing if `from` is gone, so a control cannot pass vacuously.
fn edited(rel: &str, from: &str, to: &str) -> String {
    let text = read(rel);
    assert!(
        text.contains(from),
        "ci_changes: the control's edit target is gone from {rel}: {from}"
    );
    text.replacen(from, to, 1)
}

// ---- The classifier ----------------------------------------------------------------------------------------------

/// A change's verdict, as `classify` prints it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Verdict {
    /// `code`, `data` or `docs`.
    tier: String,
    gpu: bool,
    plan_data: bool,
    gui_data: bool,
    mutants: bool,
}

/// `script` (the text of `scripts/ci_changes.sh`) run as `classify` on `paths`: its verdict. Its `docs_only` must be
/// true exactly when the tier is data or docs.
fn classify(script: &str, paths: &[&str]) -> Verdict {
    let dir = Scratch::new("ci_changes_classify");
    std::fs::create_dir_all(&*dir).expect("scratch folder");
    let file = dir.join("ci_changes.sh");
    let input = dir.join("paths.txt");
    std::fs::write(&file, script).expect("script written");
    std::fs::write(&input, paths.join("\n")).expect("paths written");
    let out = Command::new("bash")
        .args(["-c", "bash \"$0\" classify < \"$1\""])
        .arg(&file)
        .arg(&input)
        .timed_output()
        .expect("ci_changes: bash ran");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "ci_changes: classify failed: {stdout}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let value = |key: &str| -> String {
        let lines: Vec<&str> = stdout
            .lines()
            .filter_map(|l| l.strip_prefix(&format!("{key}=")))
            .collect();
        assert_eq!(
            lines.len(),
            1,
            "ci_changes: classify printed not one `{key}=`: {stdout}"
        );
        lines[0].to_owned()
    };
    let flag = |key: &str| -> bool {
        match value(key).as_str() {
            "true" => true,
            "false" => false,
            other => panic!("ci_changes: `{key}={other}` is neither true nor false"),
        }
    };
    let tier = value("tier");
    assert!(
        ["code", "data", "docs"].contains(&tier.as_str()),
        "ci_changes: `tier={tier}` is none of code, data and docs"
    );
    assert_eq!(
        flag("docs_only"),
        tier != "code",
        "ci_changes: {paths:?}: `docs_only` is not true exactly on a data or docs change (tier {tier})"
    );
    Verdict {
        gpu: flag("gpu"),
        plan_data: flag("plan_data"),
        gui_data: flag("gui_data"),
        mutants: flag("mutants"),
        tier,
    }
}

/// Each change, and the verdict R-656, R-662 and R-677 give it: (paths, tier, GPU suites run, mutants run).
const CASES: &[(&[&str], &str, bool, bool)] = &[
    // The papers and their data, the reference sources, the workbench.
    (&["docs/incoming/a/b.pdf"], "docs", false, false),
    (&["docs/reference/prin-rs/src/a.rs"], "docs", false, false),
    (&["docs/experiments/results/a.json"], "docs", false, false),
    (
        &["docs/archive/a.png", "docs/incoming/a/b.csv"],
        "docs",
        false,
        false,
    ),
    (&["workbench/a/b.html"], "docs", false, false),
    (&["workbench/a/b.rs"], "docs", false, false),
    // The corpus's prose, which some tests read: not Principia's code (R-662).
    (
        &["docs/contracts/principia_parity_contract.md"],
        "docs",
        false,
        false,
    ),
    (
        &["docs/gui/principia_render_gui_spec.md"],
        "docs",
        false,
        false,
    ),
    (
        &["docs/archive/a.md", "docs/archive/a.html"],
        "docs",
        false,
        false,
    ),
    // The plan's prose, its other data and its Python.
    (
        &[
            "plan/OPERATIONS.md",
            "plan/WORKFLOW.md",
            "plan/CURRENT_RULES.md",
            "plan/coverage.md",
            "plan/rule_groups.yaml",
            "plan/section_notes.yaml",
            "plan/reviewers/qa.md",
        ],
        "docs",
        false,
        false,
    ),
    (
        &["plan/check_plan.py", "plan/tools/rulings.py"],
        "docs",
        false,
        false,
    ),
    // The other scripts.
    (&["scripts/progress_banner.py"], "docs", false, false),
    (&["scripts/check_no_atlas_params.sh"], "docs", false, false),
    (&["scripts/readme_gifs/figkit.py"], "docs", false, false),
    // The root's markdown.
    (
        &[
            "decisions.md",
            "REVIEW_QUEUE.md",
            "open-questions.md",
            "README.md",
            "CLAUDE.md",
            "AGENTS.md",
            "GEMINI.md",
        ],
        "docs",
        false,
        false,
    ),
    // Tier (b), the data only the cheap checks and its readers' tests read (R-677): no GPU suite, no mutants.
    (&["docs/gui/design/a.png"], "data", false, false),
    (&["docs/gui/reference/a/b.json"], "data", false, false),
    (&["plan/tasks.yaml"], "data", false, false),
    (&["plan/MILESTONES.md"], "data", false, false),
    (&["plan/tasks/M2/TASK-M2-01.md"], "data", false, false),
    // With docs beside it, a data change stays data: the highest tier.
    (
        &["plan/tasks.yaml", "docs/gui/design/a.png", "decisions.md"],
        "data",
        false,
        false,
    ),
    // The data only full CI reads, and the classifier, are code.
    (&["plan/requirements.yaml"], "code", true, false),
    (&["scripts/ci_changes.sh"], "code", true, false),
    (&["scripts/contain.sh"], "code", true, false),
    // Principia's code, and what builds it. Only Rust under `crates/` has mutants the gate runs (R-677).
    (&["crates/kernel/tests/qa.rs"], "code", true, true),
    (&["crates/kernel/src/lib.rs"], "code", true, true),
    (&["crates/kernel/build.rs"], "code", true, true),
    (
        &["crates/render/shaders/wgsl/lib/coords.wgsl"],
        "code",
        true,
        false,
    ),
    (&["crates/ledger/wgsl/shape.wgsl"], "code", true, false),
    (&["crates/kernel/Cargo.toml"], "code", true, false),
    (&["xtask/src/main.rs"], "code", true, false),
    (&["xtask/tests/ci_changes.rs"], "code", true, false),
    (&["fixtures/golden/a.png"], "code", true, false),
    (
        &["fixtures/mutants/untested/src/lib.rs"],
        "code",
        true,
        false,
    ),
    (&["Cargo.toml"], "code", true, false),
    (&["Cargo.lock"], "code", true, false),
    (&["rust-toolchain.toml"], "code", true, false),
    (&[".github/workflows/ci.yml"], "code", true, false),
    (&[".github/copilot-instructions.md"], "code", true, false),
    // A path nothing names fails towards full CI: an unknown top-level file or folder, or another dot-folder.
    (&["newfolder/a.md"], "code", true, false),
    (&["newfolder/crates/a.rs"], "code", true, false),
    (&["notes.md"], "code", true, false),
    (&[".cargo/config.toml"], "code", true, false),
    (&[".config/nextest.toml"], "code", true, false),
    (&[".claude/agents/implementer.md"], "code", true, false),
    (&[".gitignore"], "code", true, false),
    // One code path among docs or data makes the change code.
    (
        &["docs/a.md", "crates/kernel/src/lib.rs"],
        "code",
        true,
        true,
    ),
    (&["decisions.md", "Cargo.lock"], "code", true, false),
    (
        &["plan/WORKFLOW.md", "plan/requirements.yaml"],
        "code",
        true,
        false,
    ),
    (
        &["plan/requirements.yaml", "plan/tasks.yaml"],
        "code",
        true,
        false,
    ),
    (
        &["docs/gui/design/a.png", "xtask/src/a.rs"],
        "code",
        true,
        false,
    ),
    (
        &["scripts/progress_banner.py", "newfolder/a"],
        "code",
        true,
        false,
    ),
    // A path that only starts like an allowed one is not on the list.
    (&["docsx/a.md"], "code", true, false),
    (&["planx/a.md"], "code", true, false),
    (&["scriptsx/a.sh"], "code", true, false),
    (&["workbenchx/a"], "code", true, false),
    (&["decisions.md.bak"], "code", true, false),
    (&["plan"], "code", true, false),
    (&["docs"], "code", true, false),
    (&["cratesx/a.rs"], "code", true, false),
    // Nor does a listed datum's near neighbour count as it.
    (&["plan/requirements.yaml.bak"], "docs", false, false),
    (&["plan/tasks.yaml.bak"], "docs", false, false),
    (&["plan/MILESTONES.md.bak"], "docs", false, false),
    (&["plan/tasksx/a.md"], "docs", false, false),
    (&["docs/gui/designs/a.png"], "docs", false, false),
    (&["docs/gui/references/a.html"], "docs", false, false),
    (&["scripts/ci_changes.sh.bak"], "docs", false, false),
    (&["scripts/contain.sh.bak"], "docs", false, false),
    // The crates no GPU job builds skip the GPU suites, and nothing else.
    (&["crates/gui/src/app.rs"], "code", false, true),
    (
        &["crates/prin/src/main.rs", "docs/incoming/a.pdf"],
        "code",
        false,
        true,
    ),
    (&["crates/gui/src/app.rs", "docs/a.md"], "code", false, true),
    (
        &["crates/gui/src/app.rs", "docs/gui/design/a.png"],
        "code",
        false,
        true,
    ),
    (
        &["crates/gui/src/app.rs", "plan/requirements.yaml"],
        "code",
        true,
        true,
    ),
    (&["crates/guix/src/a.rs"], "code", true, true),
    (
        &[
            "crates/gui/src/a.rs",
            "crates/render/shaders/wgsl/lib/coords.wgsl",
        ],
        "code",
        true,
        true,
    ),
    // No path at all runs everything, its mutants among it.
    (&[], "code", true, true),
];

/// Which of tier (b)'s data each change names: (paths, plan_data, gui_data). A code change keeps its flags, which no
/// job then reads.
const DATA_CASES: &[(&[&str], bool, bool)] = &[
    (&["plan/tasks/M0/TASK-M0-01.md"], true, false),
    (&["plan/tasks.yaml", "decisions.md"], true, false),
    (&["plan/MILESTONES.md"], true, false),
    (&["docs/gui/design/01_main.png"], false, true),
    (&["docs/gui/reference/a.html", "docs/a.md"], false, true),
    (
        &["plan/tasks.yaml", "docs/gui/reference/a.html"],
        true,
        true,
    ),
    (&["docs/a.md", "plan/WORKFLOW.md"], false, false),
    (&["plan/requirements.yaml"], false, false),
    (
        &["crates/kernel/src/lib.rs", "plan/tasks.yaml"],
        true,
        false,
    ),
    (&[], false, false),
];

fn check_classifier(script: &str) {
    for (paths, tier, gpu, mutants) in CASES {
        let got = classify(script, paths);
        assert_eq!(
            (got.tier.as_str(), got.gpu, got.mutants),
            (*tier, *gpu, *mutants),
            "ci_changes: {paths:?} classified wrongly (tier, gpu, mutants)"
        );
    }
    for (paths, plan, gui) in DATA_CASES {
        let got = classify(script, paths);
        assert_eq!(
            (got.plan_data, got.gui_data),
            (*plan, *gui),
            "ci_changes: {paths:?} names the wrong data (plan_data, gui_data)"
        );
    }
}

#[test]
fn ci_changes_classifies_conservatively() {
    check_classifier(&read("scripts/ci_changes.sh"));
}

negative_control!(
    ci_changes_classifies_conservatively,
    "the script with `.github/` on its docs-only allowlist",
    expected = "\".github/workflows/ci.yml\"] classified wrongly",
    check_classifier(&edited(
        "scripts/ci_changes.sh",
        "    docs/* | plan/* | scripts/* | workbench/*) return 0 ;;",
        "    docs/* | plan/* | scripts/* | workbench/* | .github/*) return 0 ;;"
    ))
);

mod data_below_docs {
    use super::*;
    negative_control!(
        ci_changes_classifies_conservatively,
        "the script letting a later docs path lower a data change to docs, not taking the highest tier",
        expected = "\"plan/tasks.yaml\", \"docs/gui/design/a.png\", \"decisions.md\"] classified wrongly",
        check_classifier(&edited(
            "scripts/ci_changes.sh",
            "      data) if [ \"$tier\" = docs ]; then tier=data; fi ;;\n",
            "      data) if [ \"$tier\" = docs ]; then tier=data; fi ;;\n      docs) if [ \"$tier\" = data ]; then tier=docs; fi ;;\n"
        ))
    );
}

mod data_over_code {
    use super::*;
    negative_control!(
        ci_changes_classifies_conservatively,
        "the script letting a data path lower a code change to data",
        expected = "\"plan/requirements.yaml\", \"plan/tasks.yaml\"] classified wrongly",
        check_classifier(&edited(
            "scripts/ci_changes.sh",
            "      data) if [ \"$tier\" = docs ]; then tier=data; fi ;;\n",
            "      data) tier=data ;;\n"
        ))
    );
}

mod mutants_anywhere {
    use super::*;
    negative_control!(
        ci_changes_classifies_conservatively,
        "the script running mutants on Rust source outside crates/",
        expected = "\"docs/reference/prin-rs/src/a.rs\"] classified wrongly",
        check_classifier(&edited(
            "scripts/ci_changes.sh",
            "    crates/*.rs) return 0 ;;\n",
            "    *.rs) return 0 ;;\n"
        ))
    );
}

mod no_mutants_on_no_diff {
    use super::*;
    negative_control!(
        ci_changes_classifies_conservatively,
        "the script running no mutants when the diff can't be computed",
        expected = "[] classified wrongly",
        check_classifier(&edited(
            "scripts/ci_changes.sh",
            "    gpu=true\n    mutants=true\n",
            "    gpu=true\n"
        ))
    );
}

mod gpu_on_data {
    use super::*;
    negative_control!(
        ci_changes_classifies_conservatively,
        "the script running the GPU suites on tier (b)'s data, as before R-677",
        expected = "\"docs/gui/design/a.png\"] classified wrongly",
        check_classifier(&edited(
            "scripts/ci_changes.sh",
            "  if [ \"$(tier_of \"$1\")\" != code ]; then\n",
            "  if [ \"$(tier_of \"$1\")\" = docs ]; then\n"
        ))
    );
}

mod plan_flag_lost {
    use super::*;
    negative_control!(
        ci_changes_classifies_conservatively,
        "the script never setting plan_data",
        expected = "\"plan/tasks/M0/TASK-M0-01.md\"] names the wrong data",
        check_classifier(&edited(
            "scripts/ci_changes.sh",
            "    if plan_data \"$path\"; then plan=true; fi\n",
            ""
        ))
    );
}

// ---- The files code reads -----------------------------------------------------------------------------------------

/// The patterns of the function `name` in `script`, in order: each `<pattern> | …) return 0 ;;` line's.
fn patterns(script: &str, name: &str) -> Vec<String> {
    let start = script
        .find(&format!("\n{name}() {{\n"))
        .unwrap_or_else(|| panic!("ci_changes: the script has no `{name}` function"));
    let body = &script[start..];
    let end = body
        .find("\n}\n")
        .unwrap_or_else(|| panic!("ci_changes: `{name}` does not end"));
    body[..end]
        .lines()
        .filter_map(|l| l.trim().strip_suffix(") return 0 ;;"))
        .flat_map(|l| l.split(" | ").map(str::to_owned).collect::<Vec<_>>())
        .collect()
}

/// Each pattern of `scripts/ci_changes.sh`'s `reads_code` list, in its order, and real files it names: touching any one
/// of them must run the build, test and GPU jobs (R-662, R-677).
const CODE_READ: &[(&str, &[&str])] = &[
    ("plan/requirements.yaml", &["plan/requirements.yaml"]),
    ("scripts/ci_changes.sh", &["scripts/ci_changes.sh"]),
    ("scripts/contain.sh", &["scripts/contain.sh"]),
];

fn check_code_read(script: &str) {
    let tested: Vec<&str> = CODE_READ.iter().map(|(p, _)| *p).collect();
    assert_eq!(
        patterns(script, "reads_code"),
        tested,
        "ci_changes: the script's list of files only full CI reads is not the one this test touches"
    );
    for (pattern, paths) in CODE_READ {
        for path in *paths {
            assert!(
                root().join(path).is_file(),
                "ci_changes: {path}, {pattern}'s representative, is not a file in the repository"
            );
            let got = classify(script, &[path]);
            assert_eq!(
                (got.tier.as_str(), got.gpu),
                ("code", true),
                "ci_changes: touching {path} ({pattern}, a file only full CI reads) does not run the build, test \
                 and GPU jobs (tier, gpu)"
            );
        }
    }
}

#[test]
fn ci_changes_files_code_reads_run_everything() {
    check_code_read(&read("scripts/ci_changes.sh"));
}

negative_control!(
    ci_changes_files_code_reads_run_everything,
    "the script with its list of files only full CI reads emptied",
    expected = "touching plan/requirements.yaml (plan/requirements.yaml, a file only full CI reads) does not run",
    check_code_read(&edited(
        "scripts/ci_changes.sh",
        "reads_code() {\n  case \"$1\" in\n",
        "reads_code() {\n  return 1\n  case \"$1\" in\n"
    ))
);

/// Each pattern of `scripts/ci_changes.sh`'s `plan_data` and `gui_data` lists, the function's name, and real files it
/// names: touching any one of them is tier (b) (R-677), with that function's flag.
const TIER_B: &[(&str, &str, &[&str])] = &[
    (
        "plan_data",
        "plan/tasks/*",
        &["plan/tasks/M0/TASK-M0-01.md"],
    ),
    ("plan_data", "plan/tasks.yaml", &["plan/tasks.yaml"]),
    ("plan_data", "plan/MILESTONES.md", &["plan/MILESTONES.md"]),
    (
        "gui_data",
        "docs/gui/design/*",
        &["docs/gui/design/01_main.png"],
    ),
    (
        "gui_data",
        "docs/gui/reference/*",
        &[
            "docs/gui/reference/print_graph/catalogue/presets.json",
            "docs/gui/reference/ic_inspector.html",
        ],
    ),
];

fn check_tier_b(script: &str) {
    for function in ["plan_data", "gui_data"] {
        let tested: Vec<&str> = TIER_B
            .iter()
            .filter(|(f, _, _)| *f == function)
            .map(|(_, p, _)| *p)
            .collect();
        assert_eq!(
            patterns(script, function),
            tested,
            "ci_changes: the script's `{function}` list is not the one this test touches"
        );
    }
    for (function, pattern, paths) in TIER_B {
        for path in *paths {
            assert!(
                root().join(path).is_file(),
                "ci_changes: {path}, {pattern}'s representative, is not a file in the repository"
            );
            let got = classify(script, &[path]);
            let want = Verdict {
                tier: "data".to_owned(),
                gpu: false,
                plan_data: *function == "plan_data",
                gui_data: *function == "gui_data",
                mutants: false,
            };
            assert_eq!(
                got, want,
                "ci_changes: touching {path} ({pattern}, tier (b)'s {function}) is not tier (b)"
            );
        }
    }
}

#[test]
fn ci_changes_data_lands_in_tier_b() {
    check_tier_b(&read("scripts/ci_changes.sh"));
}

negative_control!(
    ci_changes_data_lands_in_tier_b,
    "the script with the task files moved back to the files only full CI reads",
    expected =
        "touching plan/tasks/M0/TASK-M0-01.md (plan/tasks/*, tier (b)'s plan_data) is not tier (b)",
    check_tier_b(&edited(
        "scripts/ci_changes.sh",
        "    plan/requirements.yaml) return 0 ;;\n",
        "    plan/requirements.yaml | plan/tasks/*) return 0 ;;\n"
    ))
);

mod gui_as_plan {
    use super::*;
    negative_control!(
        ci_changes_data_lands_in_tier_b,
        "the script setting the plan's flag on the GUI's artboards",
        expected = "touching docs/gui/design/01_main.png (docs/gui/design/*, tier (b)'s gui_data) is not tier (b)",
        check_tier_b(&edited(
            "scripts/ci_changes.sh",
            "    if plan_data \"$path\"; then plan=true; fi\n",
            "    if plan_data \"$path\" || gui_data \"$path\"; then plan=true; fi\n"
        ))
    );
}

/// The string literals in `line`: Rust's `"…"`. Best effort: an escaped quote is stepped over, a raw string's hashes are
/// not read, and a literal spanning lines is missed.
fn literals(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        if c != '"' {
            continue;
        }
        let mut lit = String::new();
        let mut escaped = false;
        for d in chars.by_ref() {
            if escaped {
                lit.push(d);
                escaped = false;
            } else if d == '\\' {
                escaped = true;
            } else if d == c {
                break;
            } else {
                lit.push(d);
            }
        }
        out.push(lit);
    }
    out
}

/// `path` with its `.` and `..` components resolved lexically, or `None` if it climbs out of its first component.
fn normalised(path: &Path) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !out.pop() {
                    return None;
                }
            }
            other => out.push(other),
        }
    }
    Some(out)
}

/// The files under `dir`, relative to the root, whose names end with `ext`, `target/` skipped.
fn sources(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root().join(dir)) else {
        return;
    };
    for entry in entries {
        let name = entry.expect("entry").file_name();
        let path = dir.join(&name);
        if name == "target" {
            continue;
        }
        if root().join(&path).is_dir() {
            sources(&path, ext, out);
        } else if name.to_string_lossy().ends_with(ext) {
            out.push(path);
        }
    }
}

/// The repository files the tests, build scripts and xtask name, best effort: every string literal in a crate's `src/`,
/// `tests/` or `build.rs`, and in `xtask/`, outside a whole-line comment,
/// that names an existing file, read from the repository's root, the source's package directory (as a path built on
/// `CARGO_MANIFEST_DIR` is) or the source's own directory (as `include_str!` reads). A literal starting with `/` (the
/// tail of `concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/…")`) is read from the package directory alone, its `/`
/// stripped. Each as (reader, file), both relative to the root. The limit: a reader that walks a directory (`cargo
/// xtask lint vocab`, the tests that copy `docs/`), builds a path with `format!`, `join` or
/// `concat!` in more than one literal (`concat!(env!("CARGO_MANIFEST_DIR"), "/../..")` then `.join(rel)`), or reads a
/// path from a data file, is not seen here; those are listed by hand in the script, and
/// `ci_changes_files_code_reads_run_everything` checks them.
fn named_files() -> Vec<(String, String)> {
    let mut files = Vec::new();
    let crates = std::fs::read_dir(root().join("crates")).expect("ci_changes: crates/");
    let mut packages: Vec<PathBuf> = crates
        .map(|e| Path::new("crates").join(e.expect("entry").file_name()))
        .collect();
    packages.push(PathBuf::from("xtask"));
    packages.sort();
    for package in &packages {
        sources(&package.join("src"), ".rs", &mut files);
        sources(&package.join("tests"), ".rs", &mut files);
        if root().join(package).join("build.rs").is_file() {
            files.push(package.join("build.rs"));
        }
    }
    files.sort();
    let mut out = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(root().join(&file)).expect("ci_changes: a source");
        let package = packages
            .iter()
            .find(|p| file.starts_with(p))
            .cloned()
            .unwrap_or_default();
        let dir = file.parent().expect("a source's folder").to_path_buf();
        for line in text.lines() {
            let t = line.trim_start();
            if t.starts_with("//") {
                continue;
            }
            for lit in literals(line) {
                if lit.is_empty() || lit.contains(['{', '*', '\n', '<', '$']) {
                    continue;
                }
                // A literal starting with `/` is the tail of `concat!(env!("CARGO_MANIFEST_DIR"), "/…")`: joined
                // as written it would be absolute, so its `/` is stripped and the rest read from the package.
                let (rest, bases) = match lit.strip_prefix('/') {
                    Some(rest) => (rest, vec![package.as_path()]),
                    None => (lit.as_str(), vec![Path::new(""), &package, &dir]),
                };
                for base in bases {
                    let Some(path) = normalised(&base.join(rest)) else {
                        continue;
                    };
                    if !path.as_os_str().is_empty() && root().join(&path).is_file() {
                        out.push((
                            file.to_string_lossy().into_owned(),
                            path.to_string_lossy().into_owned(),
                        ));
                    }
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// The `scripts/` files runs go through: named as a workflow step's `shell: bash scripts/…` or as nextest's
/// `command-line = "scripts/…`, with what names each.
fn wrapping_scripts() -> Vec<(String, String)> {
    let mut sources = vec![(
        ".config/nextest.toml".to_owned(),
        read(".config/nextest.toml"),
    )];
    for entry in std::fs::read_dir(root().join(".github/workflows")).expect("ci_changes: workflows")
    {
        let name = entry.expect("ci_changes: a workflow").file_name();
        let rel = format!(".github/workflows/{}", name.to_string_lossy());
        sources.push((rel.clone(), read(&rel)));
    }
    let mut out = Vec::new();
    for (source, text) in &sources {
        for line in text.lines() {
            let line = line.trim_start();
            if line.starts_with('#') {
                continue;
            }
            for lead in ["shell: bash scripts/", "command-line = \"scripts/"] {
                let Some(at) = line.find(lead) else { continue };
                let tail = &line[at + lead.len()..];
                let end = tail.find([' ', '"']).unwrap_or(tail.len());
                out.push((source.clone(), format!("scripts/{}", &tail[..end])));
            }
        }
    }
    out
}

fn check_wrapping(script: &str, wrapping: &[(String, String)]) {
    // Not vacuous: the three places that run the containment are found.
    for source in [
        ".config/nextest.toml",
        ".github/workflows/mutants.yml",
        ".github/workflows/nightly.yml",
    ] {
        assert!(
            wrapping
                .iter()
                .any(|(s, f)| s == source && f == "scripts/contain.sh"),
            "ci_changes: the scan did not find {source} running scripts/contain.sh"
        );
    }
    for (source, file) in wrapping {
        let got = classify(script, &[file.as_str()]);
        assert_eq!(
            (got.tier.as_str(), got.gpu),
            ("code", true),
            "ci_changes: {source} runs through {file}, which does not run the build, test and GPU jobs (tier, gpu)"
        );
    }
}

#[test]
fn ci_changes_a_script_runs_go_through_is_code() {
    check_wrapping(&read("scripts/ci_changes.sh"), &wrapping_scripts());
}

negative_control!(
    ci_changes_a_script_runs_go_through_is_code,
    "the script without the containment on its list",
    expected = "runs through scripts/contain.sh, which does not run the build, test and GPU jobs",
    check_wrapping(
        &edited(
            "scripts/ci_changes.sh",
            "    scripts/contain.sh) return 0 ;;\n",
            ""
        ),
        &wrapping_scripts()
    )
);

/// Whether `reader` naming `file` is reading data for behaviour (R-662): anything but prose and scripts, or the plan's
/// markdown named from a package's `src/`, which a test's mention of it is not. Prose a test checks, a citation in the
/// ledger's register and a test of a script stay on the docs' side.
fn loads_data(reader: &str, file: &str) -> bool {
    let prose_or_script = [".md", ".py", ".sh"].iter().any(|x| file.ends_with(x));
    !prose_or_script
        || (file.starts_with("plan/") && file.ends_with(".md") && reader.contains("/src/"))
}

/// The data code names that `script` classifies as docs-only: none may be.
fn check_readers(script: &str, named: &[(String, String)]) {
    // The scan is not vacuous: it finds a literal in a test, a package's `src/` reading the plan's data and its
    // markdown, and a reference a golden compares with.
    for want in [
        ("xtask/tests/screenshot.rs", "docs/gui/design/01_main.png"),
        (
            "crates/validation/src/gate/mod.rs",
            "plan/requirements.yaml",
        ),
        ("xtask/src/gate_report.rs", "plan/tasks.yaml"),
        ("xtask/src/gate_report.rs", "plan/MILESTONES.md"),
        (
            "crates/kernel/tests/qa_TASK-M2-08.rs",
            "docs/gui/reference/ic_inspector.html",
        ),
    ] {
        assert!(
            named.iter().any(|(r, f)| (r.as_str(), f.as_str()) == want),
            "ci_changes: the reader scan did not find {} reading {}",
            want.0,
            want.1
        );
        assert!(
            loads_data(want.0, want.1),
            "ci_changes: {} reading {} is not counted as loading data",
            want.0,
            want.1
        );
    }
    let mut verdicts: std::collections::BTreeMap<&str, bool> = std::collections::BTreeMap::new();
    let mut wrong = Vec::new();
    for (reader, file) in named {
        // This test names files to classify them, which is no reading.
        if !loads_data(reader, file) || reader == "xtask/tests/ci_changes.rs" {
            continue;
        }
        let docs = *verdicts
            .entry(file.as_str())
            .or_insert_with(|| classify(script, &[file.as_str()]).tier == "docs");
        if docs {
            wrong.push(format!("{reader} reads {file}"));
        }
    }
    assert!(
        wrong.is_empty(),
        "ci_changes: data code loads classifies as docs; list it in the script's `reads_code`, `plan_data` or \
         `gui_data`:\n  {}",
        wrong.join("\n  ")
    );
}

#[test]
fn ci_changes_no_file_code_reads_is_docs_only() {
    check_readers(&read("scripts/ci_changes.sh"), &named_files());
}

negative_control!(
    ci_changes_no_file_code_reads_is_docs_only,
    "the script without the artboards on its tier (b) list",
    expected = "xtask/tests/screenshot.rs reads docs/gui/design/01_main.png",
    check_readers(
        &edited(
            "scripts/ci_changes.sh",
            "    docs/gui/design/*) return 0 ;;\n",
            ""
        ),
        &named_files()
    )
);

// ---- Tier (b) runs its readers' tests ----------------------------------------------------------------------------

/// The readers the scan cannot see, each walking a folder or copying a tree of tier (b)'s data, listed by hand: (the
/// reader, a literal in it that shows the walk, a file of that data it reads).
const WALKERS: &[(&str, &str, &str)] = &[
    (
        "xtask/tests/qa_TASK-M0-36.rs",
        "\"plan/tasks\"",
        "plan/tasks/M0/TASK-M0-01.md",
    ),
    (
        "xtask/tests/qa_R-485_print_graph_coverage.rs",
        "\"plan/tasks\"",
        "plan/tasks/M0/TASK-M0-01.md",
    ),
    (
        "xtask/tests/qa_R-485_print_graph_coverage.rs",
        "\"docs/gui/reference/print_graph/catalogue/\"",
        "docs/gui/reference/print_graph/catalogue/presets.json",
    ),
    (
        "crates/validation/tests/part6_catalogue.rs",
        "join(\"tasks\")",
        "plan/tasks/M0/TASK-M0-01.md",
    ),
    (
        "xtask/tests/plan_check.rs",
        "join(\"plan\")",
        "plan/tasks.yaml",
    ),
    (
        "xtask/tests/qa_TASK-M0-02.rs",
        "join(\"plan\")",
        "plan/tasks.yaml",
    ),
];

/// The tests of the classifier, which name the data to classify it, no reading: the classifier is code, so they run
/// with full CI whenever it changes.
const CLASSIFIER_TESTS: &[&str] = &[
    "xtask/tests/ci_changes.rs",
    "xtask/tests/qa_R-656_ci_changes.rs",
    "xtask/tests/qa_R-662_ci_scope.rs",
];

/// The step condition `ci-workspace` runs a tier (b) flag's readers' tests on.
fn data_condition(flag: &str) -> String {
    format!("needs.changes.outputs.tier == 'data' && needs.changes.outputs.{flag} == 'true'")
}

/// The tests `ci-workspace` runs on a data change that sets `flag` (`plan_data` or `gui_data`): each step on
/// [`data_condition`] runs `cargo nextest run -p <package> --test <test>…`, and nothing else, as (package, test).
fn data_tests(wf: &Value, flag: &str) -> Vec<(String, String)> {
    let steps = wf["jobs"]["ci-workspace"]["steps"]
        .as_array()
        .expect("ci_changes: ci-workspace's steps");
    let mut out = Vec::new();
    for step in steps
        .iter()
        .filter(|s| text(&s["if"]) == data_condition(flag))
    {
        let run = text(&step["run"]);
        let words: Vec<&str> = run.split_whitespace().collect();
        let (package, tests) = match words.as_slice() {
            ["cargo", "nextest", "run", "-p", package, tests @ ..] => (*package, tests),
            _ => panic!("ci_changes: a {flag} step runs `{run}`, not `cargo nextest run -p <package> --test …`"),
        };
        assert!(
            !tests.is_empty() && tests.len() % 2 == 0,
            "ci_changes: the {flag} step `{run}` names no test, or an argument that is no `--test <name>`"
        );
        for pair in tests.chunks(2) {
            assert_eq!(
                pair[0], "--test",
                "ci_changes: the {flag} step `{run}` passes `{}`, which is no `--test <name>`: it would build more \
                 than its readers' tests",
                pair[0]
            );
            out.push((package.to_owned(), pair[1].to_owned()));
        }
    }
    out
}

/// The test a reader of tier (b)'s data runs as: a test file is its own (package, test); an xtask module is its own
/// tests', `xtask/tests/<module>.rs`. Anything else has no test to name.
fn reader_test(reader: &str) -> Option<(String, String)> {
    let path = Path::new(reader);
    let stem = path.file_stem()?.to_string_lossy().into_owned();
    let parent = path.parent()?.to_string_lossy().into_owned();
    if parent == "xtask/tests" {
        return Some(("xtask".to_owned(), stem));
    }
    if parent == "xtask/src" && root().join(format!("xtask/tests/{stem}.rs")).is_file() {
        return Some(("xtask".to_owned(), stem));
    }
    let package = parent.strip_prefix("crates/")?.strip_suffix("/tests")?;
    (!package.contains('/')).then(|| (package.to_owned(), stem))
}

fn check_data_readers(script: &str, wf: &str, named: &[(String, String)]) {
    let wf = parse(wf);
    let plan = data_tests(&wf, "plan_data");
    let gui = data_tests(&wf, "gui_data");
    // Not vacuous: the scan finds gate-report's module and tests reading the plan's data, and a screenshot test and a
    // reference test reading the GUI's.
    for want in [
        ("xtask/src/gate_report.rs", "plan/MILESTONES.md"),
        ("xtask/tests/gate_report.rs", "plan/MILESTONES.md"),
        ("xtask/tests/screenshot.rs", "docs/gui/design/01_main.png"),
        (
            "crates/kernel/tests/qa_TASK-M2-08.rs",
            "docs/gui/reference/ic_inspector.html",
        ),
    ] {
        assert!(
            named.iter().any(|(r, f)| (r.as_str(), f.as_str()) == want),
            "ci_changes: the reader scan did not find {} reading {}",
            want.0,
            want.1
        );
    }
    let mut readers: Vec<(String, String)> = named.to_vec();
    for (reader, marker, file) in WALKERS {
        assert!(
            read(reader).contains(marker),
            "ci_changes: {reader} no longer shows its walk ({marker}); the list of walkers is stale"
        );
        readers.push(((*reader).to_owned(), (*file).to_owned()));
    }
    let mut wrong = Vec::new();
    for (reader, file) in &readers {
        if CLASSIFIER_TESTS.contains(&reader.as_str()) {
            continue;
        }
        let verdict = classify(script, &[file.as_str()]);
        if verdict.tier != "data" {
            continue;
        }
        let ran = match reader_test(reader) {
            Some(test) => {
                (verdict.plan_data && plan.contains(&test))
                    || (verdict.gui_data && gui.contains(&test))
            }
            None => false,
        };
        if !ran {
            wrong.push(format!("{reader} reads {file}"));
        }
    }
    assert!(
        wrong.is_empty(),
        "ci_changes: a reader of tier (b)'s data whose tests `ci-workspace` does not run on that data; name its test \
         in the step, or move the file to `reads_code`:\n  {}",
        wrong.join("\n  ")
    );
}

#[test]
fn ci_changes_data_tier_runs_every_reader() {
    check_data_readers(
        &read("scripts/ci_changes.sh"),
        &read(".github/workflows/ci.yml"),
        &named_files(),
    );
}

negative_control!(
    ci_changes_data_tier_runs_every_reader,
    "ci.yml's plan-data step without gate-report's tests",
    expected = "xtask/src/gate_report.rs reads plan/MILESTONES.md",
    check_data_readers(
        &read("scripts/ci_changes.sh"),
        &edited(".github/workflows/ci.yml", " --test gate_report ", " "),
        &named_files(),
    )
);

mod walker_dropped {
    use super::*;
    negative_control!(
        ci_changes_data_tier_runs_every_reader,
        "ci.yml's plan-data step without the test that walks every task file",
        expected = "xtask/tests/qa_TASK-M0-36.rs reads plan/tasks/M0/TASK-M0-01.md",
        check_data_readers(
            &read("scripts/ci_changes.sh"),
            &edited(".github/workflows/ci.yml", " --test qa_TASK-M0-36 ", " "),
            &named_files(),
        )
    );
}

mod gui_reader_dropped {
    use super::*;
    negative_control!(
        ci_changes_data_tier_runs_every_reader,
        "ci.yml's GUI-data steps without render's colour explorer test",
        expected = "crates/render/tests/qa_TASK-M1-03.rs reads docs/gui/reference/principia_colour_explorer.html",
        check_data_readers(
            &read("scripts/ci_changes.sh"),
            &edited(
                ".github/workflows/ci.yml",
                "        run: cargo nextest run -p render --test qa_TASK-M1-03\n",
                "        run: cargo nextest run -p render --test qa_TASK-M7-02\n"
            ),
            &named_files(),
        )
    );
}

mod workspace_build {
    use super::*;
    negative_control!(
        ci_changes_data_tier_runs_every_reader,
        "ci.yml's plan-data step building the whole workspace's tests",
        expected = "is no `--test <name>`",
        check_data_readers(
            &read("scripts/ci_changes.sh"),
            &edited(
                ".github/workflows/ci.yml",
                "        run: cargo nextest run -p validation --test part6_catalogue\n",
                "        run: cargo nextest run -p validation --workspace --test part6_catalogue\n"
            ),
            &named_files(),
        )
    );
}

// ---- The workflows, parsed ---------------------------------------------------------------------------------------

/// A workflow's text as JSON, parsed by PyYAML. YAML 1.1 reads the key `on` as `true`; it is put back as `on`.
fn parse(yml: &str) -> Value {
    let dir = Scratch::new("ci_changes_yaml");
    std::fs::create_dir_all(&*dir).expect("scratch folder");
    let file = dir.join("workflow.yml");
    std::fs::write(&file, yml).expect("workflow written");
    let out = Command::new("python3")
        .args([
            "-c",
            "import json, sys, yaml\n\
             d = yaml.safe_load(open(sys.argv[1]))\n\
             if True in d: d['on'] = d.pop(True)\n\
             print(json.dumps(d))",
        ])
        .arg(&file)
        .timed_output()
        .expect("ci_changes: python3 ran");
    assert!(
        out.status.success(),
        "ci_changes: the workflow is not YAML: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("ci_changes: PyYAML's JSON")
}

fn text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn jobs(wf: &Value) -> Vec<(String, Value)> {
    wf["jobs"]
        .as_object()
        .expect("ci_changes: the workflow has jobs")
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

// ---- The gates ---------------------------------------------------------------------------------------------------

/// The job named `check` in `wf`: its id and its body.
fn named(wf: &Value, check: &str) -> (String, Value) {
    let mut by: Vec<(String, Value)> = jobs(wf)
        .into_iter()
        .filter(|(_, j)| j.get("name").map(text).as_deref() == Some(check))
        .collect();
    assert_eq!(by.len(), 1, "ci_changes: not one job named `{check}`");
    by.remove(0)
}

/// Whether `script` passes under bash with the environment `env`, as a gate's step runs on the runner.
fn passes(script: &str, env: &[(&str, &str)]) -> bool {
    let mut cmd = Command::new("bash");
    cmd.args(["-e", "-c", script]);
    for (k, v) in env {
        cmd.env(k, v);
    }
    cmd.timed_output()
        .expect("ci_changes: bash ran")
        .status
        .success()
}

/// A run a gate must pass or fail: RESULTS, its gathered jobs' results in its `needs:` order (as
/// `join(needs.*.result, ' ')` gives them, a matrix job's shards as one); its flag's value, as its env expression
/// computes it on that run; and whether the gate passes.
struct Run {
    what: &'static str,
    results: &'static str,
    flag: &'static str,
    pass: bool,
}

/// A required check's gate in `ci.yml`: its name, its `needs:`, the env variable that lets a skip pass and the
/// expression it is set from, and its runs.
struct Gate {
    check: &'static str,
    needs: &'static [&'static str],
    flag: &'static str,
    from: &'static str,
    runs: &'static [Run],
}

const fn run(what: &'static str, results: &'static str, flag: &'static str, pass: bool) -> Run {
    Run {
        what,
        results,
        flag,
        pass,
    }
}

/// The GPU suites' gates: `needs: [changes, <suites>]`; NO_GPU is true only when `changes` succeeded and found no
/// GPU-path file (a failed `changes` gives `false`).
const GPU_RUNS: &[Run] = &[
    run(
        "a change with no GPU-path file",
        "success skipped",
        "true",
        true,
    ),
    run("a GPU-path change", "success success", "false", true),
    run(
        "the classification failed",
        "failure skipped",
        "false",
        false,
    ),
    run(
        "a GPU-path change whose suites were skipped",
        "success skipped",
        "false",
        false,
    ),
    run(
        "a GPU-path change whose suites failed",
        "success failure",
        "false",
        false,
    ),
    run("a cancelled run", "success cancelled", "true", false),
];

const GPU_FLAG: &str =
    "${{ needs.changes.result == 'success' && needs.changes.outputs.gpu == 'false' }}";

const GATES: &[Gate] = &[
    // `needs: [ci, ci-checks, ci-workspace]`; DOCS_ONLY is `ci-workspace`'s output, empty unless it ran.
    Gate {
        check: "ci",
        needs: &["ci", "ci-checks", "ci-workspace"],
        flag: "DOCS_ONLY",
        from: "${{ needs.ci-workspace.outputs.docs_only }}",
        runs: &[
            run(
                "a docs-only pull request",
                "skipped skipped success",
                "true",
                true,
            ),
            run("a code change", "success success success", "false", true),
            run(
                "the classification failed",
                "skipped skipped skipped",
                "",
                false,
            ),
            run(
                "a code change whose ci-checks was skipped",
                "success skipped success",
                "false",
                false,
            ),
            run(
                "a docs-only pull request whose cheap checks failed",
                "skipped skipped failure",
                "true",
                false,
            ),
            run(
                "a cancelled shard",
                "cancelled success success",
                "false",
                false,
            ),
        ],
    },
    // `needs: [xtask-ci, changes]`; DOCS_ONLY is false when `changes` failed.
    Gate {
        check: "xtask-ci",
        needs: &["xtask-ci", "changes"],
        flag: "DOCS_ONLY",
        from:
            "${{ needs.changes.result == 'success' && needs.changes.outputs.docs_only == 'true' }}",
        runs: &[
            run("a docs-only pull request", "skipped success", "true", true),
            run("a code change", "success success", "false", true),
            run(
                "the classification failed",
                "skipped failure",
                "false",
                false,
            ),
            run(
                "a code change whose shards were skipped",
                "skipped success",
                "false",
                false,
            ),
            run(
                "a docs-only pull request with a failed shard",
                "failure success",
                "true",
                false,
            ),
        ],
    },
    Gate {
        check: "gpu-kernel",
        needs: &["changes", "gpu-kernel"],
        flag: "NO_GPU",
        from: GPU_FLAG,
        runs: GPU_RUNS,
    },
    Gate {
        check: "gpu-metal",
        needs: &["changes", "gpu-metal"],
        flag: "NO_GPU",
        from: GPU_FLAG,
        runs: GPU_RUNS,
    },
    Gate {
        check: "gpu-lavapipe",
        needs: &["changes", "gpu-lavapipe"],
        flag: "NO_GPU",
        from: GPU_FLAG,
        runs: GPU_RUNS,
    },
];

fn check_gates(yml: &str) {
    let wf = parse(yml);
    for gate in GATES {
        let check = gate.check;
        let (id, job) = named(&wf, check);
        assert_eq!(
            text(&job["if"]),
            "always()",
            "ci_changes: the gate `{check}` ({id}) is not `if: always()`"
        );
        let needs: Vec<String> = job["needs"]
            .as_array()
            .unwrap_or_else(|| panic!("ci_changes: the gate `{check}` has no `needs:` list"))
            .iter()
            .map(text)
            .collect();
        assert_eq!(
            needs, gate.needs,
            "ci_changes: the gate `{check}` gathers other jobs"
        );
        let steps = job["steps"]
            .as_array()
            .expect("ci_changes: the gate has steps");
        assert_eq!(
            steps.len(),
            1,
            "ci_changes: the gate `{check}` has not one step"
        );
        assert_eq!(
            text(&steps[0]["env"]["RESULTS"]),
            "${{ join(needs.*.result, ' ') }}",
            "ci_changes: the gate `{check}` does not read every gathered job's result"
        );
        assert_eq!(
            text(&steps[0]["env"][gate.flag]),
            gate.from,
            "ci_changes: the gate `{check}`'s {} is not set from the classification",
            gate.flag
        );
        let script = text(&steps[0]["run"]);
        for run in gate.runs {
            assert_eq!(
                passes(&script, &[("RESULTS", run.results), (gate.flag, run.flag)]),
                run.pass,
                "ci_changes: the gate `{check}` on {}: passed is not {}",
                run.what,
                run.pass
            );
        }
    }
}

#[test]
fn ci_changes_gates_pass_only_on_the_verdict() {
    check_gates(&read(".github/workflows/ci.yml"));
}

negative_control!(
    ci_changes_gates_pass_only_on_the_verdict,
    "ci.yml with the `gpu-metal` gate passing any skip",
    expected =
        "the gate `gpu-metal` on a GPU-path change whose suites were skipped: passed is not false",
    check_gates(&edited(
        ".github/workflows/ci.yml",
        "[ \"$NO_GPU\" = true ]; } || { echo \"a job gpu-metal gathers",
        "true; } || { echo \"a job gpu-metal gathers"
    ))
);

// ---- The jobs that skip --------------------------------------------------------------------------------------

const DOCS_SKIP: &str = "needs.changes.outputs.docs_only != 'true'";
const GPU_SKIP: &str = "needs.changes.outputs.gpu == 'true'";

/// Each job in `ci.yml` that skips on the classification, and the condition it runs on.
const SKIPS: [(&str, &str); 7] = [
    ("ci-archive", DOCS_SKIP),
    ("ci-checks", DOCS_SKIP),
    ("ci-release-archive", DOCS_SKIP),
    ("xtask-ci", DOCS_SKIP),
    ("gpu-kernel", GPU_SKIP),
    ("gpu-metal", GPU_SKIP),
    ("gpu-lavapipe", GPU_SKIP),
];

fn check_skips(yml: &str) {
    let wf = parse(yml);
    let changes = &wf["jobs"]["changes"];
    let steps = changes["steps"]
        .as_array()
        .expect("ci_changes: no `changes` job");
    assert!(
        steps
            .iter()
            .any(|s| text(&s["run"]).trim() == "scripts/ci_changes.sh detect"
                && text(&s["id"]) == "changes"),
        "ci_changes: the `changes` job does not run the classifier"
    );
    assert!(
        changes.get("if").is_none() && changes.get("needs").is_none(),
        "ci_changes: the `changes` job can be skipped"
    );
    // Each part of the verdict the jobs read, passed on as the classifier wrote it (R-677).
    for key in ["tier", "docs_only", "gpu", "plan_data", "gui_data"] {
        assert_eq!(
            text(&changes["outputs"][key]),
            format!("${{{{ steps.changes.outputs.{key} }}}}"),
            "ci_changes: the `changes` job does not pass on `{key}`"
        );
    }
    for (id, cond) in SKIPS {
        let job = &wf["jobs"][id];
        assert_eq!(
            text(&job["needs"]),
            "changes",
            "ci_changes: `{id}` does not wait on the classification alone"
        );
        assert_eq!(
            text(&job["if"]),
            cond,
            "ci_changes: `{id}` skips on `{}`",
            text(&job["if"])
        );
    }
    // `ci-workspace` runs either its tests or the cheap checks, and passes the verdict to the gate `ci`.
    let ws = &wf["jobs"]["ci-workspace"];
    assert_eq!(
        text(&ws["needs"]),
        "changes",
        "ci_changes: `ci-workspace` does not wait on the classification"
    );
    assert!(
        ws.get("if").is_none(),
        "ci_changes: `ci-workspace` can be skipped"
    );
    assert_eq!(
        text(&ws["outputs"]["docs_only"]),
        "${{ needs.changes.outputs.docs_only }}",
        "ci_changes: `ci-workspace` does not pass the verdict on"
    );
    let runs = |cond: &str| -> Vec<String> {
        ws["steps"]
            .as_array()
            .expect("ci_changes: ci-workspace's steps")
            .iter()
            .filter(|s| text(&s["if"]) == cond)
            .map(|s| text(&s["run"]))
            .collect()
    };
    let tests = runs(DOCS_SKIP);
    for want in ["cargo nextest run -p xtask", "cargo test -p xtask --doc"] {
        assert!(
            tests.iter().any(|r| r.trim() == want),
            "ci_changes: `ci-workspace`'s `{want}` does not skip on a docs-only pull request"
        );
    }
    let docs = runs("needs.changes.outputs.docs_only == 'true'").join("\n");
    for want in [
        "python3 plan/check_plan.py",
        "python3 plan/tools/current_rules.py --check",
        "cargo xtask lint vocab",
    ] {
        assert!(
            docs.contains(want),
            "ci_changes: `ci-workspace` does not run `{want}` on a docs-only pull request"
        );
    }
    // Only the cheap checks (R-662): no test run, and no build but the lint's.
    assert!(
        !docs.contains("nextest") && !docs.contains("cargo test") && !docs.contains("cargo build"),
        "ci_changes: `ci-workspace` runs more than the cheap checks on a docs-only pull request: {docs}"
    );
    // A data change's readers' tests (R-677) run on exactly a data verdict and its own flag; no other step reads the
    // tier.
    for step in ws["steps"]
        .as_array()
        .expect("ci_changes: ci-workspace's steps")
    {
        let cond = text(&step["if"]);
        if cond.contains("outputs.tier") {
            assert!(
                cond == data_condition("plan_data") || cond == data_condition("gui_data"),
                "ci_changes: a `ci-workspace` step runs on `{cond}`, not on a data verdict and its flag"
            );
        }
    }
    for flag in ["plan_data", "gui_data"] {
        assert!(
            !runs(&data_condition(flag)).is_empty(),
            "ci_changes: `ci-workspace` runs nothing on a data change that sets `{flag}`"
        );
    }
}

#[test]
fn ci_changes_jobs_skip_on_the_verdict() {
    check_skips(&read(".github/workflows/ci.yml"));
}

mod tier_not_passed_on {
    use super::*;
    negative_control!(
        ci_changes_jobs_skip_on_the_verdict,
        "ci.yml whose `changes` job does not pass the tier on",
        expected = "the `changes` job does not pass on `tier`",
        check_skips(&edited(
            ".github/workflows/ci.yml",
            "      tier: ${{ steps.changes.outputs.tier }}\n",
            ""
        ))
    );
}

mod data_steps_on_code {
    use super::*;
    negative_control!(
        ci_changes_jobs_skip_on_the_verdict,
        "ci.yml running the GUI data's readers on any tier but docs",
        expected = "runs on `needs.changes.outputs.tier != 'docs'",
        check_skips(&edited(
            ".github/workflows/ci.yml",
            "        if: needs.changes.outputs.tier == 'data' && needs.changes.outputs.gui_data == 'true'\n        run: cargo nextest run -p kernel",
            "        if: needs.changes.outputs.tier != 'docs' && needs.changes.outputs.gui_data == 'true'\n        run: cargo nextest run -p kernel"
        ))
    );
}

negative_control!(
    ci_changes_jobs_skip_on_the_verdict,
    "ci.yml with gpu-metal's suites skipped on the docs-only verdict",
    expected = "`gpu-metal` skips on `needs.changes.outputs.docs_only != 'true'`",
    check_skips(&edited(
        ".github/workflows/ci.yml",
        "    name: gpu-metal suites\n    needs: changes\n    if: needs.changes.outputs.gpu == 'true'\n",
        "    name: gpu-metal suites\n    needs: changes\n    if: needs.changes.outputs.docs_only != 'true'\n"
    ))
);

// ---- No skipped job reports a required name ----------------------------------------------------------------------

/// The workflows a pull request runs, by file name, and their texts.
fn pull_request_workflows() -> Vec<(String, String)> {
    let dir = root().join(".github/workflows");
    let mut out: Vec<(String, String)> = std::fs::read_dir(&dir)
        .expect("ci_changes: .github/workflows")
        .map(|e| e.expect("entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "yml"))
        .map(|p| {
            let name = p.file_name().expect("name").to_string_lossy().into_owned();
            (name, std::fs::read_to_string(&p).expect("workflow"))
        })
        .collect();
    out.sort();
    out
}

/// The check name a job reports: its `name:`, or its id. A matrix job's expands per shard; skipped, it is reported
/// unexpanded.
fn reports(id: &str, job: &Value) -> String {
    job.get("name").map(text).unwrap_or_else(|| id.to_owned())
}

/// The required checks a workflow posts through the Checks API rather than as a job, so that no run that skips the job
/// reports it passed: `mutants-check`, pending until the merge-ready run posts its verdict (R-656, R-676).
/// Each with the workflow that posts it.
const POSTED: [(&str, &str); 1] = [("mutants-check", "mutants.yml")];

fn check_required(workflows: &[(String, String)]) {
    let mut found: Vec<(String, String)> = Vec::new();
    for (check, poster) in POSTED {
        let posts = workflows
            .iter()
            .any(|(f, yml)| f == poster && yml.contains(&format!("-f name={check} ")));
        assert!(
            posts,
            "ci_changes: {poster} does not post the required check `{check}`"
        );
        found.push((check.to_owned(), format!("{poster}: posted")));
    }
    for (file, yml) in workflows {
        let wf = parse(yml);
        let on = &wf["on"];
        let runs_on_prs = on.get("pull_request").is_some()
            || on
                .as_array()
                .is_some_and(|a| a.iter().any(|t| t == "pull_request"))
            || on.as_str() == Some("pull_request");
        let required_here = jobs(&wf)
            .iter()
            .any(|(id, j)| REQUIRED.contains(&reports(id, j).as_str()));
        if runs_on_prs && required_here {
            let pr = &on["pull_request"];
            for key in ["paths", "paths-ignore", "branches", "branches-ignore"] {
                assert!(
                    pr.get(key).is_none(),
                    "ci_changes: {file} filters its pull_request trigger with `{key}:`, so a required check can hang \
                     pending"
                );
            }
        }
        for (id, job) in jobs(&wf) {
            let name = reports(&id, &job);
            if job.get("strategy").and_then(|s| s.get("matrix")).is_some() {
                assert!(
                    name.contains("${{ matrix.") || !REQUIRED.contains(&name.as_str()),
                    "ci_changes: {file}'s matrix job `{id}` is named `{name}`, without its shard; skipped, it would \
                     report the required name `{name}`"
                );
                continue;
            }
            if POSTED.iter().any(|(c, _)| *c == name) {
                found.push((name.clone(), format!("{file}: {id}")));
                continue;
            }
            if !REQUIRED.contains(&name.as_str()) || !runs_on_prs {
                continue;
            }
            found.push((name.clone(), format!("{file}: {id}")));
            let cond = job.get("if").map(text).unwrap_or_default();
            let skippable = job.get("needs").is_some() || !cond.is_empty();
            assert!(
                !skippable || cond.starts_with("always()"),
                "ci_changes: {file}'s job `{id}`, the required check `{name}`, can be skipped (if: `{cond}`)"
            );
        }
    }
    for check in REQUIRED {
        let by: Vec<&String> = found
            .iter()
            .filter(|(n, _)| n == check)
            .map(|(_, w)| w)
            .collect();
        assert_eq!(
            by.len(),
            1,
            "ci_changes: the required check `{check}` is reported by {} jobs of the pull-request workflows: {by:?}",
            by.len()
        );
    }
}

#[test]
fn ci_changes_no_skipped_job_reports_a_required_name() {
    check_required(&pull_request_workflows());
}

negative_control!(
    ci_changes_no_skipped_job_reports_a_required_name,
    "ci.yml with the xtask-ci shards named for their job alone",
    expected = "matrix job `xtask-ci` is named `xtask-ci`, without its shard",
    {
        let mut workflows = pull_request_workflows();
        let at = workflows
            .iter()
            .position(|(f, _)| f == "ci.yml")
            .expect("ci_changes: no ci.yml");
        workflows[at].1 = edited(
            ".github/workflows/ci.yml",
            "    name: xtask-ci (${{ matrix.shard }})\n",
            "    name: xtask-ci\n",
        );
        check_required(&workflows)
    }
);

mod posted_by_a_job {
    use super::*;
    negative_control!(
        ci_changes_no_skipped_job_reports_a_required_name,
        "mutants.yml's verdict job named mutants-check, which the run of a label other than merge-ready skips",
        expected = "the required check `mutants-check` is reported by 2 jobs",
        {
            let mut workflows = pull_request_workflows();
            let at = workflows
                .iter()
                .position(|(f, _)| f == "mutants.yml")
                .expect("ci_changes: no mutants.yml");
            workflows[at].1 = edited(
                ".github/workflows/mutants.yml",
                "  mutants-verdict:\n",
                "  mutants-check:\n",
            );
            check_required(&workflows)
        }
    );
}
