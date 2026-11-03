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
//! - `plan/requirements.yaml`, field by field (R-677, the owner, 2 Nov 2026: "Yeah do that then, make it good and work
//!   well"; "keep it simple and make ci fast without sacrificing testing on actual code"): on scratch copies, never the
//!   real file, a note, statement, source or comment edit is tier (b) with `plan_data`
//!   (`ci_changes_requirements_unread_field_edit_is_tier_b`); a change to a field of `scripts/ci_req_fields.py`'s
//!   CODE_FIELDS, a field on neither list or a reformatted line code reads is code
//!   (`ci_changes_requirements_code_field_edit_is_full_ci`); so are an added or removed requirement
//!   (`ci_changes_requirements_added_is_full_ci`), a copy that does not parse, holds no list of ids, or is missing
//!   (`ci_changes_requirements_malformed_is_full_ci`), and a tier (b) edit beside a `crates/` file
//!   (`ci_changes_requirements_beside_code_is_full_ci`); each prints one reason line naming the id and field. `detect`
//!   gives the field check the merge's base and head copies (`ci_changes_detect_checks_the_requirements_fields`).
//! - `ci_changes_code_fields_cover_every_loader`: every source naming `plan/requirements.yaml` (each crate's and
//!   xtask's `src/` and `tests/`, and the workflows) reads, as a string literal starting `<field>:` or a `get("…")`
//!   lookup shows, only fields in CODE_FIELDS, but the tests tier (b) runs on the plan's data and the classifier's own
//!   tests. Its limit: a reader that builds the field name some other way is not seen.
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
//!   workflow-level `paths:` or `paths-ignore:`; the required checks are each reported by exactly one job,
//!   `mutants-check` among them, a job of every run of `mutants.yml` (R-678); a job that can be skipped (a job-level
//!   `if:` or a `needs:`) is never named for a required check unless its `if:` is `always()`; and a matrix job never
//!   bears a required check's name without its shard.
//! - `ci_changes_every_required_check_reports_on_every_tier`: a change of each tier, classified by the script, is run
//!   through `ci.yml`'s jobs (their `if:`s and `needs:` evaluated as GitHub does, every job that runs succeeding) and
//!   each gate's script run under bash: `ci`, `xtask-ci` and the three GPU gates pass on every tier. `mutants.yml`'s
//!   `mutants-check`, its report step run with a stand-in `gh`, passes a push of each "not applicable", with its
//!   reason: no Rust under `crates/` on a data or docs change, not merge-ready on a code one (R-677, R-678);
//!   `pr-check` and `reviews-complete` read no classification.
//! - `ci_changes_every_required_check_reports_on_every_path`: the required checks are `plan/HUMAN_SETUP.md` §2's, and
//!   each reports on every path the owner named (3 Nov 2026, R-678): a push of code, plan-data and docs, a label put on
//!   and taken off, `merge-ready` and another. GitHub counts, of each workflow's runs on the head, only the latest run's
//!   check suite, so the workflow bearing a check either runs on the event, the job bearing it unskippable and, in
//!   `ci.yml` and `mutants.yml`, concluding as it should, or runs on every new head, its push run standing; a workflow
//!   runs on both label changes or neither, and filters no pull request.
//! - `ci_changes_mutants_fixture_runs_on_the_gates_own_files`: `mutants.yml`'s `mutants-fixture`, the gate's self-test on
//!   its fixtures, runs every step after its classification on a change to the gate's configuration
//!   (`.cargo/mutants.toml`, `fixtures/mutants/untested/`, `fixtures/mutants/caps/`), which has no Rust under `crates/`,
//!   and on a crate's source and tier (b)'s data, as before R-677; on a docs change it skips them all (R-677, as applied
//!   after PR #382's code review).
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

/// The checks branch protection requires on `main` (R-266, R-305, and `gpu-kernel` since TASK-M0-14), as
/// `plan/HUMAN_SETUP.md` §2 records them (`ci_changes_every_required_check_reports_on_every_path` checks it).
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
    classify_with(script, paths, &[]).0
}

/// [`classify`] with `env` set, as `(name, value)`: its verdict and what it printed on stderr.
fn classify_with(script: &str, paths: &[&str], env: &[(&str, &Path)]) -> (Verdict, String) {
    let dir = Scratch::new("ci_changes_classify");
    std::fs::create_dir_all(&*dir).expect("scratch folder");
    let file = dir.join("ci_changes.sh");
    let input = dir.join("paths.txt");
    std::fs::write(&file, script).expect("script written");
    std::fs::write(&input, paths.join("\n")).expect("paths written");
    let mut command = Command::new("bash");
    command
        .args(["-c", "bash \"$0\" classify < \"$1\""])
        .arg(&file)
        .arg(&input)
        .env_remove("CI_REQS_BASE")
        .env_remove("CI_REQS_HEAD")
        .env_remove("CI_REQ_FIELDS");
    for (name, value) in env {
        command.env(name, value);
    }
    let out = command.timed_output().expect("ci_changes: bash ran");
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
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
    let verdict = Verdict {
        gpu: flag("gpu"),
        plan_data: flag("plan_data"),
        gui_data: flag("gui_data"),
        mutants: flag("mutants"),
        tier,
    };
    (verdict, stderr)
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
    ("scripts/ci_req_fields.py", &["scripts/ci_req_fields.py"]),
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

// ---- plan/requirements.yaml, field by field ------------------------------------------------------------------------

/// A scratch `plan/requirements.yaml`, never the real one: two requirements, with every field the plan uses.
const REQS: &str = "# Principia build requirements (a fixture).\n\
- id: REQ-QA-001\n  area: QA\n  kind: calibration\n  statement: \"the first\"\n  source:\n  - file: docs/a.md\n    \
section: A\n  verify:\n    method: numerical gate\n    detail: \"the gate\"\n  rulings: [R-1]\n  milestone: M2\n  \
note: \"a note\"\n\
- id: REQ-QA-002\n  area: QA\n  statement: \"the second\"\n  source:\n  - file: docs/b.md\n    section: B\n  \
verify:\n    method: unit test\n    detail: \"a test\"\n  rulings: []\n  milestone: M3\n";

/// [`REQS`] with `from` replaced by `to` once, failing if `from` is gone.
fn reqs_edited(from: &str, to: &str) -> String {
    assert!(
        REQS.contains(from),
        "ci_changes: the fixture's edit target is gone: {from}"
    );
    REQS.replacen(from, to, 1)
}

/// `script` and `fields` (the texts of `scripts/ci_changes.sh` and `scripts/ci_req_fields.py`) run as `classify` on
/// `paths`, with scratch copies of `plan/requirements.yaml` at the base (`base`) and the head (`head`), or no base copy
/// at all when `base` is `None`: the verdict and the field check's reason line.
fn classify_reqs(
    script: &str,
    fields: &str,
    paths: &[&str],
    base: Option<&str>,
    head: &str,
) -> (Verdict, String) {
    let dir = Scratch::new("ci_changes_reqs");
    std::fs::create_dir_all(&*dir).expect("scratch folder");
    let fields_file = dir.join("ci_req_fields.py");
    let base_file = dir.join("base.yaml");
    let head_file = dir.join("head.yaml");
    std::fs::write(&fields_file, fields).expect("field check written");
    if let Some(base) = base {
        std::fs::write(&base_file, base).expect("base written");
    }
    std::fs::write(&head_file, head).expect("head written");
    classify_with(
        script,
        paths,
        &[
            ("CI_REQ_FIELDS", &fields_file),
            ("CI_REQS_BASE", &base_file),
            ("CI_REQS_HEAD", &head_file),
        ],
    )
}

/// A requirements change: what it is, its paths, the head's text (the base is [`REQS`], or none when `None`), the tier
/// and `plan_data` it gets, and a part of the reason line it prints.
struct ReqCase {
    what: &'static str,
    paths: &'static [&'static str],
    base: Option<&'static str>,
    head: fn() -> String,
    tier: &'static str,
    plan_data: bool,
    reason: &'static str,
}

const REQ_PATH: &[&str] = &["plan/requirements.yaml"];

fn check_reqs(script: &str, fields: &str, cases: &[ReqCase]) {
    for case in cases {
        let (got, stderr) = classify_reqs(script, fields, case.paths, case.base, &(case.head)());
        assert_eq!(
            (got.tier.as_str(), got.plan_data),
            (case.tier, case.plan_data),
            "ci_changes: {} classified wrongly (tier, plan_data); the field check said: {stderr}",
            case.what
        );
        if got.tier == "data" {
            assert!(
                !got.gpu && !got.mutants,
                "ci_changes: {} runs the GPU suites or mutants in tier (b)",
                case.what
            );
        }
        let reasons: Vec<&str> = stderr
            .lines()
            .filter(|l| l.starts_with("plan/requirements.yaml: "))
            .collect();
        assert!(
            reasons.len() == 1 && reasons[0].contains(case.reason),
            "ci_changes: {}: not one reason line naming `{}`: {stderr}",
            case.what,
            case.reason
        );
    }
}

/// Edits to fields no code reads: tier (b), with the plan's readers' tests.
const TIER_B_REQS: &[ReqCase] = &[
    ReqCase {
        what: "a note edit",
        paths: REQ_PATH,
        base: Some(REQS),
        head: || reqs_edited("  note: \"a note\"", "  note: \"a longer note\""),
        tier: "data",
        plan_data: true,
        reason: "REQ-QA-001 note",
    },
    ReqCase {
        what: "a statement edit",
        paths: REQ_PATH,
        base: Some(REQS),
        head: || reqs_edited("\"the second\"", "\"the second, reworded\""),
        tier: "data",
        plan_data: true,
        reason: "REQ-QA-002 statement",
    },
    ReqCase {
        what: "a note added and a source moved",
        paths: REQ_PATH,
        base: Some(REQS),
        head: || {
            reqs_edited("  milestone: M3\n", "  milestone: M3\n  note: \"new\"\n").replacen(
                "docs/b.md",
                "docs/c.md",
                1,
            )
        },
        tier: "data",
        plan_data: true,
        reason: "REQ-QA-002 note and 1 more",
    },
    ReqCase {
        what: "a comment edit",
        paths: REQ_PATH,
        base: Some(REQS),
        head: || reqs_edited("(a fixture)", "(a fixture, reworded)"),
        tier: "data",
        plan_data: true,
        reason: "no requirement's fields changed",
    },
    ReqCase {
        what: "a statement edit beside a task file",
        paths: &["plan/requirements.yaml", "plan/tasks.yaml"],
        base: Some(REQS),
        head: || reqs_edited("\"the first\"", "\"the first, reworded\""),
        tier: "data",
        plan_data: true,
        reason: "REQ-QA-001 statement",
    },
];

#[test]
fn ci_changes_requirements_unread_field_edit_is_tier_b() {
    check_reqs(
        &read("scripts/ci_changes.sh"),
        &read("scripts/ci_req_fields.py"),
        TIER_B_REQS,
    );
}

negative_control!(
    ci_changes_requirements_unread_field_edit_is_tier_b,
    "the script never asking the field check, so plan/requirements.yaml is always code",
    expected = "a note edit classified wrongly",
    check_reqs(
        &edited(
            "scripts/ci_changes.sh",
            "  if requirements_data \"$1\"; then\n",
            "  if false; then\n"
        ),
        &read("scripts/ci_req_fields.py"),
        TIER_B_REQS,
    )
);

/// A change to a field code reads, or to the lines it reads: full CI.
const CODE_FIELD_REQS: &[ReqCase] = &[
    ReqCase {
        what: "a milestone change",
        paths: REQ_PATH,
        base: Some(REQS),
        head: || reqs_edited("  milestone: M2", "  milestone: M4"),
        tier: "code",
        plan_data: false,
        reason: "REQ-QA-001 milestone changed, which code reads",
    },
    ReqCase {
        what: "a kind removed",
        paths: REQ_PATH,
        base: Some(REQS),
        head: || reqs_edited("  kind: calibration\n", ""),
        tier: "code",
        plan_data: false,
        reason: "REQ-QA-001 kind changed",
    },
    ReqCase {
        what: "a requirement retired",
        paths: REQ_PATH,
        base: Some(REQS),
        head: || reqs_edited("  milestone: M3\n", "  milestone: M3\n  retired: \"R-2\"\n"),
        tier: "code",
        plan_data: false,
        reason: "REQ-QA-002 retired changed",
    },
    ReqCase {
        what: "a verify detail edit",
        paths: REQ_PATH,
        base: Some(REQS),
        head: || reqs_edited("\"a test\"", "\"another test\""),
        tier: "code",
        plan_data: false,
        reason: "REQ-QA-002 verify changed",
    },
    ReqCase {
        what: "a field on neither list",
        paths: REQ_PATH,
        base: Some(REQS),
        head: || reqs_edited("  milestone: M3\n", "  milestone: M3\n  owner: qa\n"),
        tier: "code",
        plan_data: false,
        reason: "REQ-QA-002 owner changed, a field on neither list",
    },
];

/// The lines code reads, changed with the parsed fields equal: full CI, as the loaders read lines.
const CODE_LINE_REQS: &[ReqCase] = &[ReqCase {
    what: "a kind requoted",
    paths: REQ_PATH,
    base: Some(REQS),
    head: || reqs_edited("  kind: calibration", "  kind: \"calibration\""),
    tier: "code",
    plan_data: false,
    reason: "a line code reads moved or changed: kind: \"calibration\"",
}];

#[test]
fn ci_changes_requirements_code_field_edit_is_full_ci() {
    let fields = read("scripts/ci_req_fields.py");
    check_reqs(&read("scripts/ci_changes.sh"), &fields, CODE_FIELD_REQS);
    check_reqs(&read("scripts/ci_changes.sh"), &fields, CODE_LINE_REQS);
}

negative_control!(
    ci_changes_requirements_code_field_edit_is_full_ci,
    "the field check with `milestone` moved from CODE_FIELDS to DATA_FIELDS",
    expected = "a milestone change classified wrongly",
    check_reqs(
        &read("scripts/ci_changes.sh"),
        &edited("scripts/ci_req_fields.py", "    \"milestone\",\n", "").replacen(
            "DATA_FIELDS = {\"area\",",
            "DATA_FIELDS = {\"milestone\", \"area\",",
            1
        ),
        CODE_FIELD_REQS,
    )
);

mod code_lines_unchecked {
    use super::*;
    negative_control!(
        ci_changes_requirements_code_field_edit_is_full_ci,
        "the field check comparing parsed fields only, not the lines code reads",
        expected = "a kind requoted classified wrongly",
        check_reqs(
            &read("scripts/ci_changes.sh"),
            &edited(
                "scripts/ci_req_fields.py",
                "    if old != new:\n",
                "    if False:\n"
            ),
            CODE_LINE_REQS,
        )
    );
}

/// A requirement added or removed: full CI.
const ADDED_REQS: &[ReqCase] = &[
    ReqCase {
        what: "an added requirement",
        paths: REQ_PATH,
        base: Some(REQS),
        head: || format!("{REQS}- id: REQ-QA-003\n  area: QA\n  statement: \"new\"\n"),
        tier: "code",
        plan_data: false,
        reason: "REQ-QA-003 added",
    },
    ReqCase {
        what: "a removed requirement",
        paths: REQ_PATH,
        base: Some(REQS),
        head: || REQS[..REQS.find("- id: REQ-QA-002").expect("the second")].to_owned(),
        tier: "code",
        plan_data: false,
        reason: "REQ-QA-002 removed",
    },
];

#[test]
fn ci_changes_requirements_added_is_full_ci() {
    check_reqs(
        &read("scripts/ci_changes.sh"),
        &read("scripts/ci_req_fields.py"),
        ADDED_REQS,
    );
}

negative_control!(
    ci_changes_requirements_added_is_full_ci,
    "the script taking tier (b) whatever the field check says",
    expected = "an added requirement classified wrongly",
    check_reqs(
        &edited(
            "scripts/ci_changes.sh",
            "    && python3 -I \"$fields\" \"$CI_REQS_BASE\" \"$CI_REQS_HEAD\" >&2; then\n",
            "    && { python3 -I \"$fields\" \"$CI_REQS_BASE\" \"$CI_REQS_HEAD\" >&2 || true; }; then\n"
        ),
        &read("scripts/ci_req_fields.py"),
        ADDED_REQS,
    )
);

/// A copy that does not parse, or is missing: full CI.
const MALFORMED_REQS: &[ReqCase] = &[
    ReqCase {
        what: "malformed YAML",
        paths: REQ_PATH,
        base: Some(REQS),
        head: || format!("{REQS}- id: [\n"),
        tier: "code",
        plan_data: false,
        reason: "does not parse",
    },
    ReqCase {
        what: "a mapping, not a list",
        paths: REQ_PATH,
        base: Some(REQS),
        head: || "id: REQ-QA-001\n".to_owned(),
        tier: "code",
        plan_data: false,
        reason: "does not parse",
    },
    ReqCase {
        what: "an entry without an id",
        paths: REQ_PATH,
        base: Some(REQS),
        head: || format!("{REQS}- area: QA\n"),
        tier: "code",
        plan_data: false,
        reason: "does not parse",
    },
    ReqCase {
        what: "an id twice",
        paths: REQ_PATH,
        base: Some(REQS),
        head: || format!("{REQS}- id: REQ-QA-001\n  area: QA\n"),
        tier: "code",
        plan_data: false,
        reason: "does not parse",
    },
    ReqCase {
        what: "no base copy",
        paths: REQ_PATH,
        base: None,
        head: || REQS.to_owned(),
        tier: "code",
        plan_data: false,
        reason: "does not parse",
    },
];

#[test]
fn ci_changes_requirements_malformed_is_full_ci() {
    check_reqs(
        &read("scripts/ci_changes.sh"),
        &read("scripts/ci_req_fields.py"),
        MALFORMED_REQS,
    );
}

negative_control!(
    ci_changes_requirements_malformed_is_full_ci,
    "the field check taking a file that does not parse as tier (b)",
    expected = "malformed YAML classified wrongly",
    check_reqs(
        &read("scripts/ci_changes.sh"),
        &edited(
            "scripts/ci_req_fields.py",
            "        return False, \"does not parse: \"",
            "        return True, \"does not parse: \""
        ),
        MALFORMED_REQS,
    )
);

/// An edit to fields no code reads beside a code path: full CI.
const BESIDE_CODE_REQS: &[ReqCase] = &[ReqCase {
    what: "a note edit beside a crates/ file",
    paths: &["crates/kernel/src/lib.rs", "plan/requirements.yaml"],
    base: Some(REQS),
    head: || reqs_edited("  note: \"a note\"", "  note: \"a longer note\""),
    tier: "code",
    plan_data: true,
    reason: "REQ-QA-001 note",
}];

#[test]
fn ci_changes_requirements_beside_code_is_full_ci() {
    check_reqs(
        &read("scripts/ci_changes.sh"),
        &read("scripts/ci_req_fields.py"),
        BESIDE_CODE_REQS,
    );
}

negative_control!(
    ci_changes_requirements_beside_code_is_full_ci,
    "the script letting a data path lower a code change to data",
    expected = "a note edit beside a crates/ file classified wrongly",
    check_reqs(
        &edited(
            "scripts/ci_changes.sh",
            "      data) if [ \"$tier\" = docs ]; then tier=data; fi ;;\n",
            "      data) tier=data ;;\n"
        ),
        &read("scripts/ci_req_fields.py"),
        BESIDE_CODE_REQS,
    )
);

/// `git` in `repo`, with no user or system configuration; its trimmed stdout.
fn git(repo: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(["-c", "user.name=qa", "-c", "user.email=qa@example.invalid"])
        .args(args)
        .current_dir(repo)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .timed_output()
        .expect("ci_changes: git ran");
    assert!(
        out.status.success(),
        "ci_changes: git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

/// `script` run as `detect` on a pull request's merge commit whose branch edits a note in [`REQS`], the base having
/// moved on with code: its tier.
fn detect_reqs(script: &str) -> String {
    let repo = Scratch::new("ci_changes_detect_reqs");
    std::fs::create_dir_all(repo.join("plan")).expect("repo");
    git(&repo, &["init", "-q", "-b", "main"]);
    std::fs::write(repo.join("plan/requirements.yaml"), REQS).expect("written");
    std::fs::write(repo.join("lib.rs"), "// base\n").expect("written");
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-q", "-m", "base"]);
    git(&repo, &["checkout", "-q", "-b", "pr"]);
    std::fs::write(
        repo.join("plan/requirements.yaml"),
        reqs_edited("  note: \"a note\"", "  note: \"a longer note\""),
    )
    .expect("written");
    git(&repo, &["commit", "-q", "-am", "pr"]);
    git(&repo, &["checkout", "-q", "main"]);
    std::fs::write(repo.join("lib.rs"), "// main moved on\n").expect("written");
    git(&repo, &["commit", "-q", "-am", "main"]);
    git(&repo, &["merge", "-q", "--no-ff", "-m", "merge", "pr"]);
    let sha = git(&repo, &["rev-parse", "HEAD"]);
    let file = repo.join("ci_changes_under_test.sh");
    let output = repo.join("github_output");
    std::fs::write(&file, script).expect("script written");
    let out = Command::new("bash")
        .arg(&file)
        .arg("detect")
        .current_dir(&repo)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GITHUB_EVENT_NAME", "pull_request")
        .env("GITHUB_SHA", &sha)
        .env("GITHUB_OUTPUT", &output)
        .env("CI_REQ_FIELDS", root().join("scripts/ci_req_fields.py"))
        .env_remove("CI_REQS_BASE")
        .env_remove("CI_REQS_HEAD")
        .timed_output()
        .expect("ci_changes: bash ran");
    assert!(
        out.status.success(),
        "ci_changes: detect failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let written =
        std::fs::read_to_string(&output).expect("ci_changes: detect wrote no GITHUB_OUTPUT");
    written
        .lines()
        .find_map(|l| l.strip_prefix("tier="))
        .unwrap_or_else(|| panic!("ci_changes: detect wrote no tier: {written}"))
        .to_owned()
}

fn check_detect_reqs(script: &str) {
    assert_eq!(
        detect_reqs(script),
        "data",
        "ci_changes: detect on a pull request editing a note in plan/requirements.yaml is not tier (b)"
    );
}

#[test]
fn ci_changes_detect_checks_the_requirements_fields() {
    check_detect_reqs(&read("scripts/ci_changes.sh"));
}

negative_control!(
    ci_changes_detect_checks_the_requirements_fields,
    "detect never writing the base's and the head's copies",
    expected = "editing a note in plan/requirements.yaml is not tier (b)",
    check_detect_reqs(&edited(
        "scripts/ci_changes.sh",
        "      requirements_copies\n",
        "      true\n"
    ))
);

/// The field names `scripts/ci_req_fields.py` lists in its set `name` (`CODE_FIELDS`).
fn field_set(fields: &str, name: &str) -> Vec<String> {
    let start = fields
        .find(&format!("\n{name} = {{\n"))
        .unwrap_or_else(|| panic!("ci_changes: the field check has no `{name}`"));
    let body = &fields[start..];
    let end = body
        .find("\n}\n")
        .unwrap_or_else(|| panic!("ci_changes: `{name}` does not end"));
    body[..end]
        .lines()
        .map(str::trim)
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| l.strip_prefix('"')?.strip_suffix("\","))
        .map(str::to_owned)
        .collect()
}

/// The field names of the real `plan/requirements.yaml`: each key of a requirement and of its `source` and `verify`.
fn requirement_fields() -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in read("plan/requirements.yaml").lines() {
        let indent = line.len() - line.trim_start_matches(' ').len();
        if line.starts_with('#') || indent > 4 {
            continue;
        }
        let key = line.trim_start_matches(' ');
        let key = key.strip_prefix("- ").unwrap_or(key);
        let Some((name, _)) = key.split_once(':') else {
            continue;
        };
        if !name.is_empty() && name.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
            out.push(name.to_owned());
        }
    }
    out.sort();
    out.dedup();
    out
}

/// The requirement fields `text` reads, best effort: a string literal that starts (after an optional `\n`, spaces and
/// `- `) with `<field>:`, as the line-reading loaders match `"  kind: "`, `"\n  milestone: M3"` or
/// `format!("method: {method}")`, or a literal `"<field>"` right after `get(` or `[`, as gate.yml's Python does. A
/// literal is any `"` the text holds, so a fixture whose first line names a field counts too: a false reading fails
/// towards full CI.
fn fields_read(text: &str, fields: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for (at, _) in text.match_indices('"') {
        let before = &text[..at];
        let mut rest = &text[at + 1..];
        let lookup = before.ends_with("get(") || before.ends_with('[');
        for field in fields {
            if lookup && rest.starts_with(&format!("{field}\"")) {
                out.push(field.clone());
            }
        }
        rest = rest.strip_prefix("\\n").unwrap_or(rest);
        rest = rest.trim_start_matches(' ');
        rest = rest.strip_prefix("- ").unwrap_or(rest);
        for field in fields {
            if rest.starts_with(&format!("{field}:")) {
                out.push(field.clone());
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Every source that loads `plan/requirements.yaml`: each crate's and xtask's `src/`, `tests/` and `build.rs`, and the
/// workflows, naming the file.
fn requirement_loaders() -> Vec<(String, String)> {
    let mut files = Vec::new();
    let crates = std::fs::read_dir(root().join("crates")).expect("ci_changes: crates/");
    let mut packages: Vec<PathBuf> = crates
        .map(|e| Path::new("crates").join(e.expect("entry").file_name()))
        .collect();
    packages.push(PathBuf::from("xtask"));
    for package in &packages {
        sources(&package.join("src"), ".rs", &mut files);
        sources(&package.join("tests"), ".rs", &mut files);
        files.push(package.join("build.rs"));
    }
    sources(Path::new(".github/workflows"), ".yml", &mut files);
    files.sort();
    files
        .into_iter()
        .filter_map(|file| {
            let text = std::fs::read_to_string(root().join(&file)).ok()?;
            text.contains("requirements.yaml")
                .then(|| (file.to_string_lossy().into_owned(), text))
        })
        .collect()
}

/// Each loader of `plan/requirements.yaml` reads only fields of `fields`'s CODE_FIELDS, but a test tier (b) runs on a
/// plan-data change (ci.yml's plan-data steps, from `wf`), which then checks the field, and the classifier's tests
/// ([`runs_classifier`]), which name fields to classify them.
fn check_code_fields(fields: &str, wf: &str, loaders: &[(String, String)]) {
    let code = field_set(fields, "CODE_FIELDS");
    let classifier = classifier_tests(loaders);
    let known = requirement_fields();
    let plan = data_tests(&parse(wf), "plan_data");
    // Not vacuous: the gates' check of a threshold's requirement, and gate.yml's results by method, are found.
    for (want, field) in [
        ("crates/validation/src/gate/mod.rs", "retired"),
        (".github/workflows/gate.yml", "verify"),
    ] {
        let (_, text) = loaders.iter().find(|(f, _)| f == want).unwrap_or_else(|| {
            panic!("ci_changes: {want} is not found loading plan/requirements.yaml")
        });
        assert!(
            fields_read(text, &known).contains(&field.to_owned()),
            "ci_changes: {want} is not found reading `{field}`"
        );
    }
    let mut wrong = Vec::new();
    for (file, text) in loaders {
        if classifier.contains(file) {
            continue;
        }
        let path = Path::new(file);
        let tier_b_test = path.parent().is_some_and(|p| p.ends_with("tests"))
            && reader_test(file).is_some_and(|t| plan.contains(&t));
        if tier_b_test {
            continue;
        }
        for field in fields_read(text, &known) {
            if !code.contains(&field) {
                wrong.push(format!("{file} reads `{field}`"));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "ci_changes: a loader of plan/requirements.yaml outside tier (b)'s tests reads a field not in \
         scripts/ci_req_fields.py's CODE_FIELDS; add it there, naming its reader:\n  {}",
        wrong.join("\n  ")
    );
}

#[test]
fn ci_changes_code_fields_cover_every_loader() {
    check_code_fields(
        &read("scripts/ci_req_fields.py"),
        &read(".github/workflows/ci.yml"),
        &requirement_loaders(),
    );
}

negative_control!(
    ci_changes_code_fields_cover_every_loader,
    "CODE_FIELDS without `retired`, which the gates read",
    expected = "crates/validation/src/gate/mod.rs reads `retired`",
    check_code_fields(
        &edited("scripts/ci_req_fields.py", "    \"retired\",\n", ""),
        &read(".github/workflows/ci.yml"),
        &requirement_loaders(),
    )
);

mod not_a_classifier_test_reads_a_field {
    use super::*;
    negative_control!(
        ci_changes_code_fields_cover_every_loader,
        "a test naming a requirement field and the classifier, but not running it",
        expected = "xtask/tests/qa_not_a_classifier_test.rs reads `statement`",
        {
            let (file, text) = NOT_A_CLASSIFIER_TEST;
            let mut loaders = requirement_loaders();
            loaders.push((file.to_owned(), text.to_owned()));
            check_code_fields(
                &read("scripts/ci_req_fields.py"),
                &read(".github/workflows/ci.yml"),
                &loaders,
            )
        }
    );
}

mod statement_reader_dropped {
    use super::*;
    negative_control!(
        ci_changes_code_fields_cover_every_loader,
        "ci.yml's plan-data step without the print-graph coverage, which reads the statements",
        expected = "xtask/tests/qa_R-485_print_graph_coverage.rs reads `statement`",
        check_code_fields(
            &read("scripts/ci_req_fields.py"),
            &edited(
                ".github/workflows/ci.yml",
                " --test qa_R-485_print_graph_coverage --test qa_TASK-M0-02 ",
                " --test qa_TASK-M0-02 "
            ),
            &requirement_loaders(),
        )
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

/// Whether `text`, a test file's source, runs the classifier: outside whole-line comments, one string literal is
/// `scripts/ci_changes.sh` and one has `classify` or `detect`, the script's subcommands, as a whole word (`.arg("detect")`,
/// `"bash \"$0\" classify < \"$1\""`). Such a test names plan paths and field lines to classify them, no reading: the
/// classifier is code, so it runs with full CI whenever the classifier changes, and a new one needs no list kept here.
/// Its limit: a test that also reads the plan's data for itself is taken for a classifier test.
fn runs_classifier(text: &str) -> bool {
    let (mut names, mut runs) = (false, false);
    for line in text.lines() {
        if line.trim_start().starts_with("//") {
            continue;
        }
        for lit in literals(line) {
            names |= lit == "scripts/ci_changes.sh";
            runs |= lit
                .split(|c: char| c.is_whitespace() || c == '"')
                .any(|w| w == "classify" || w == "detect");
        }
    }
    names && runs
}

/// The test files (a `tests/` folder's) among `sources`, as (file, text), that run the classifier ([`runs_classifier`]).
/// Not vacuous: this file, which runs it, is among them.
fn classifier_tests(sources: &[(String, String)]) -> Vec<String> {
    let out: Vec<String> = sources
        .iter()
        .filter(|(file, text)| {
            Path::new(file)
                .parent()
                .is_some_and(|p| p.ends_with("tests"))
                && runs_classifier(text)
        })
        .map(|(file, _)| file.clone())
        .collect();
    assert!(
        out.iter().any(|f| f == "xtask/tests/ci_changes.rs"),
        "ci_changes: the classifier's own tests are not recognised as running it"
    );
    out
}

/// Every test file, each crate's and xtask's `tests/`, as (file, text).
fn test_sources() -> Vec<(String, String)> {
    let mut files = Vec::new();
    let crates = std::fs::read_dir(root().join("crates")).expect("ci_changes: crates/");
    for e in crates {
        let package = Path::new("crates").join(e.expect("entry").file_name());
        sources(&package.join("tests"), ".rs", &mut files);
    }
    sources(Path::new("xtask/tests"), ".rs", &mut files);
    files.sort();
    files
        .into_iter()
        .map(|f| {
            let text = std::fs::read_to_string(root().join(&f)).expect("ci_changes: a test source");
            (f.to_string_lossy().into_owned(), text)
        })
        .collect()
}

#[cfg(feature = "controls")]
/// A test that names plan paths and a requirement field, and names the classifier, but does not run it: a comment and a
/// literal speak of classifying, and no literal runs a subcommand. The guards must still flag it.
const NOT_A_CLASSIFIER_TEST: (&str, &str) = (
    "xtask/tests/qa_not_a_classifier_test.rs",
    "// Runs `scripts/ci_changes.sh classify`, its comment says, but it does not.\n\
     const SCRIPT: &str = \"scripts/ci_changes.sh\";\n\
     const NOTE: &str = \"reclassify-later\";\n\
     const REQS: &str = \"plan/requirements.yaml\";\n\
     const TASKS: &str = \"plan/tasks.yaml\";\n\
     fn statement(line: &str) -> bool { line.starts_with(\"  statement: \") }\n",
);

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

/// Each reader in `named` (reader, file), and each walker listed by hand, of tier (b)'s data has its test run by
/// `ci-workspace`'s step for that data, but the tests among `tests` (file, text) that run the classifier.
fn check_data_readers(
    script: &str,
    wf: &str,
    named: &[(String, String)],
    tests: &[(String, String)],
) {
    let classifier = classifier_tests(tests);
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
        if classifier.contains(reader) {
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
        &test_sources(),
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
        &test_sources(),
    )
);

mod not_a_classifier_test_reads_plan_data {
    use super::*;
    negative_control!(
        ci_changes_data_tier_runs_every_reader,
        "a test naming the plan's task data and the classifier, but not running it",
        expected = "xtask/tests/qa_not_a_classifier_test.rs reads plan/tasks.yaml",
        {
            let (file, text) = NOT_A_CLASSIFIER_TEST;
            let mut named = named_files();
            named.push((file.to_owned(), "plan/tasks.yaml".to_owned()));
            let mut tests = test_sources();
            tests.push((file.to_owned(), text.to_owned()));
            check_data_readers(
                &read("scripts/ci_changes.sh"),
                &read(".github/workflows/ci.yml"),
                &named,
                &tests,
            )
        }
    );
}

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
            &test_sources(),
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
            &test_sources(),
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
            &test_sources(),
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

fn check_required(workflows: &[(String, String)]) {
    let mut found: Vec<(String, String)> = Vec::new();
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

mod posted_not_a_job {
    use super::*;
    negative_control!(
        ci_changes_no_skipped_job_reports_a_required_name,
        "mutants.yml with no job bearing mutants-check, as when it was a check run posted through the Checks API, which \
         a later run's check suite supersedes (#384, R-678)",
        expected = "the required check `mutants-check` is reported by 0 jobs",
        {
            let mut workflows = pull_request_workflows();
            let at = workflows
                .iter()
                .position(|(f, _)| f == "mutants.yml")
                .expect("ci_changes: no mutants.yml");
            workflows[at].1 = edited(
                ".github/workflows/mutants.yml",
                "  mutants-check:\n",
                "  mutants-report:\n",
            );
            check_required(&workflows)
        }
    );
}

mod skippable_mutants_check {
    use super::*;
    negative_control!(
        ci_changes_no_skipped_job_reports_a_required_name,
        "mutants.yml's mutants-check job run only on the merge-ready label's run, so every other run reports it skipped",
        expected = "the required check `mutants-check`, can be skipped",
        {
            let mut workflows = pull_request_workflows();
            let at = workflows
                .iter()
                .position(|(f, _)| f == "mutants.yml")
                .expect("ci_changes: no mutants.yml");
            workflows[at].1 = edited(
                ".github/workflows/mutants.yml",
                "  mutants-check:\n    if: always()\n",
                "  mutants-check:\n    if: github.event.label.name == 'merge-ready'\n",
            );
            check_required(&workflows)
        }
    );
}

// ---- Every required check reports on every tier ------------------------------------------------------------------

/// A change of each tier, as a pull request makes it: what it is and its paths.
const TIER_CHANGES: [(&str, &[&str]); 4] = [
    ("a code change", &["crates/kernel/src/lib.rs"]),
    ("a data change (the plan's task data)", &["plan/tasks.yaml"]),
    (
        "a data change (the GUI's references)",
        &["docs/gui/reference/principia_colour_explorer.html"],
    ),
    (
        "a docs change",
        &["docs/design/principia_dd_replication.md"],
    ),
];

/// What a job-level `if:` or an env expression of a pull request's run reads: the event, each job's result and
/// outputs. Only the forms the workflows use are understood; any other fails the test, so a new form is looked at.
struct Ctx<'a> {
    action: &'a str,
    results: &'a std::collections::BTreeMap<String, String>,
    outputs: &'a std::collections::BTreeMap<String, std::collections::BTreeMap<String, String>>,
    needs: &'a [String],
}

fn operand(term: &str, ctx: &Ctx) -> String {
    let term = term.trim();
    if let Some(lit) = term.strip_prefix('\'').and_then(|t| t.strip_suffix('\'')) {
        return lit.to_owned();
    }
    match term {
        "always()" => return "true".to_owned(),
        "github.event_name" => return "pull_request".to_owned(),
        "github.event.action" => return ctx.action.to_owned(),
        "join(needs.*.result, ' ')" => {
            return ctx
                .needs
                .iter()
                .map(|n| ctx.results[n].clone())
                .collect::<Vec<_>>()
                .join(" ")
        }
        _ => {}
    }
    let parts: Vec<&str> = term.split('.').collect();
    match parts.as_slice() {
        ["needs", job, "result"] => ctx.results[*job].clone(),
        ["needs", job, "outputs", key] => {
            // A job that did not succeed passes no outputs on.
            if ctx.results.get(*job).map(String::as_str) == Some("success") {
                ctx.outputs
                    .get(*job)
                    .and_then(|o| o.get(*key))
                    .cloned()
                    .unwrap_or_default()
            } else {
                String::new()
            }
        }
        _ => panic!("ci_changes: an expression this test does not read: `{term}`"),
    }
}

/// An expression's value: `a && b && …` of `x == 'v'`, `x != 'v'` or `x`, inside `${{ }}` or not.
fn evaluate(expr: &str, ctx: &Ctx) -> String {
    let expr = expr.trim();
    let expr = expr
        .strip_prefix("${{")
        .and_then(|e| e.strip_suffix("}}"))
        .unwrap_or(expr);
    let terms: Vec<&str> = expr.split(" && ").collect();
    let value = |term: &str| -> String {
        if let Some((a, b)) = term.split_once(" == ") {
            (operand(a, ctx) == operand(b, ctx)).to_string()
        } else if let Some((a, b)) = term.split_once(" != ") {
            (operand(a, ctx) != operand(b, ctx)).to_string()
        } else {
            operand(term, ctx)
        }
    };
    if terms.len() == 1 {
        return value(terms[0]);
    }
    terms.iter().all(|t| value(t) == "true").to_string()
}

fn needs_of(job: &Value) -> Vec<String> {
    match job.get("needs") {
        None => Vec::new(),
        Some(Value::Array(a)) => a.iter().map(text).collect(),
        Some(v) => vec![text(v)],
    }
}

/// `ci.yml`'s run on a pull request whose classification is `verdict`: every job's result, each job that runs
/// succeeding, and each required check it reports with whether its gate's script passes.
fn ci_run(yml: &str, verdict: &[(&str, String)]) -> Vec<(String, bool)> {
    use std::collections::BTreeMap;
    let wf = parse(yml);
    let all = jobs(&wf);
    let mut results: BTreeMap<String, String> = BTreeMap::new();
    let mut outputs: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    outputs.insert(
        "changes".to_owned(),
        verdict
            .iter()
            .map(|(k, v)| ((*k).to_owned(), v.clone()))
            .collect(),
    );
    while results.len() < all.len() {
        let before = results.len();
        for (id, job) in &all {
            let needs = needs_of(job);
            if results.contains_key(id) || !needs.iter().all(|n| results.contains_key(n)) {
                continue;
            }
            let ctx = Ctx {
                action: "opened",
                results: &results,
                outputs: &outputs,
                needs: &needs,
            };
            let cond = job.get("if").map(text).unwrap_or_default();
            let ran = if cond.starts_with("always()") {
                evaluate(&cond, &ctx) == "true"
            } else {
                needs.iter().all(|n| results[n] == "success")
                    && (cond.is_empty() || evaluate(&cond, &ctx) == "true")
            };
            if id != "changes" && ran {
                let outs: BTreeMap<String, String> = job
                    .get("outputs")
                    .and_then(Value::as_object)
                    .map(|o| {
                        o.iter()
                            .filter(|(_, e)| text(e).contains("needs."))
                            .map(|(k, e)| (k.clone(), evaluate(&text(e), &ctx)))
                            .collect()
                    })
                    .unwrap_or_default();
                outputs.insert(id.clone(), outs);
            }
            let result = if ran { "success" } else { "skipped" };
            results.insert(id.clone(), result.to_owned());
        }
        assert!(
            results.len() > before,
            "ci_changes: ci.yml's jobs need one another in a cycle"
        );
    }
    let mut reported = Vec::new();
    for (id, job) in &all {
        let name = reports(id, job);
        if !REQUIRED.contains(&name.as_str()) {
            continue;
        }
        let needs = needs_of(job);
        let ctx = Ctx {
            action: "opened",
            results: &results,
            outputs: &outputs,
            needs: &needs,
        };
        let steps = job["steps"]
            .as_array()
            .unwrap_or_else(|| panic!("ci_changes: the required check `{name}` has no steps"));
        let pass = results[id] == "success"
            && steps.iter().all(|s| {
                let env: Vec<(String, String)> = s
                    .get("env")
                    .and_then(Value::as_object)
                    .map(|e| {
                        e.iter()
                            .map(|(k, v)| (k.clone(), evaluate(&text(v), &ctx)))
                            .collect()
                    })
                    .unwrap_or_default();
                let env: Vec<(&str, &str)> =
                    env.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
                passes(&text(&s["run"]), &env)
            });
        reported.push((name, pass));
    }
    reported
}

/// What `mutants.yml`'s `mutants-check` job reported: whether its report step passed, its log and its job summary.
struct Reported {
    ok: bool,
    log: String,
    summary: String,
}

/// `mutants.yml`'s `mutants-check` job, run as a runner runs it on an event whose context is `env` (`ACTION`, `LABEL`,
/// `READY`, `MUTANTS`, `VERDICT`): its classification step reads the classifier, and its report step runs under bash
/// with a stand-in `gh` that answers the pull request's head `0123abcd`, one earlier run of the workflow on it, run 5,
/// and that run's `mutants-verdict` as `$STUB_JOB`; a stand-in `sleep` fails, so a wait ends the step at once.
fn mutants_report(yml: &str, env: &[(&str, &str)]) -> Reported {
    let wf = parse(yml);
    let job = &wf["jobs"]["mutants-check"];
    let steps = job["steps"]
        .as_array()
        .expect("ci_changes: mutants-check's steps");
    assert!(
        steps.iter().any(|s| text(&s["id"]) == "changes"
            && text(&s["run"]).trim() == "scripts/ci_changes.sh detect"),
        "ci_changes: mutants-check does not run the classifier"
    );
    let step = steps
        .iter()
        .find(|s| text(&s["name"]) == "Report mutants-check")
        .expect("ci_changes: mutants-check has no report step");
    assert_eq!(
        text(&step["env"]["MUTANTS"]),
        "${{ steps.changes.outputs.mutants }}",
        "ci_changes: mutants-check's report does not read the classification's `mutants`"
    );
    let dir = Scratch::new("ci_changes_mutants_report");
    std::fs::create_dir_all(&*dir).expect("scratch folder");
    for (name, body) in [
        (
            "gh",
            "#!/usr/bin/env bash\n\
             case \"$*\" in\n\
               *'/pulls/'*) echo 0123abcd ;;\n\
               *'/jobs'*) echo \"$STUB_JOB\" ;;\n\
               *'/runs?'*) echo 5 ;;\n\
             esac\n",
        ),
        ("sleep", "#!/bin/sh\nexit 1\n"),
    ] {
        let file = dir.join(name);
        std::fs::write(&file, body).expect("stand-in written");
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755)).expect("mode");
    }
    let summary = dir.join("summary.md");
    let mut cmd = Command::new("bash");
    cmd.args(["-e", "-c", &text(&step["run"])])
        .env(
            "PATH",
            format!(
                "{}:{}",
                dir.display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .env("GITHUB_STEP_SUMMARY", &summary)
        .env("GH_TOKEN", "t")
        .env("GH_REPO", "o/r")
        .env("PR", "7")
        .env("HEAD_SHA", "0123abcd")
        .env("RUN_ID", "9")
        .env("STUB_JOB", "completed success");
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.timed_output().expect("ci_changes: bash ran");
    Reported {
        ok: out.status.success(),
        log: format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
        summary: std::fs::read_to_string(&summary).unwrap_or_default(),
    }
}

/// The reason `mutants-check` gives for "not applicable" on a change with no `.rs` file under `crates/`, and on a head
/// that is not merge-ready.
const NO_CRATES: &str = "touches no Rust source under crates/";
const NOT_READY: &str = "The head is not merge-ready";

fn check_every_tier_reports(ci: &str, mutants: &str) {
    let script = read("scripts/ci_changes.sh");
    for (what, paths) in TIER_CHANGES {
        let v = classify(&script, paths);
        let verdict = [
            ("tier", v.tier.clone()),
            ("docs_only", (v.tier != "code").to_string()),
            ("gpu", v.gpu.to_string()),
            ("plan_data", v.plan_data.to_string()),
            ("gui_data", v.gui_data.to_string()),
            ("mutants", v.mutants.to_string()),
        ];
        let reported = ci_run(ci, &verdict);
        for check in ["ci", "xtask-ci", "gpu-kernel", "gpu-metal", "gpu-lavapipe"] {
            let by: Vec<bool> = reported
                .iter()
                .filter(|(n, _)| n == check)
                .map(|(_, p)| *p)
                .collect();
            assert_eq!(
                by,
                [true],
                "ci_changes: the required check `{check}` does not pass on {what} (tier {})",
                v.tier
            );
        }
        // A push of the change, its head not merge-ready: `mutants-check` passes, not applicable, for the reason the
        // tier gives (R-677, R-678).
        let mutants_flag = v.mutants.to_string();
        let got = mutants_report(
            mutants,
            &[
                ("ACTION", "synchronize"),
                ("LABEL", ""),
                ("READY", "false"),
                ("MUTANTS", mutants_flag.as_str()),
                ("VERDICT", "skipped"),
            ],
        );
        let why = if v.tier == "code" {
            NOT_READY
        } else {
            NO_CRATES
        };
        assert!(
            got.ok && got.summary.contains("not applicable") && got.summary.contains(why),
            "ci_changes: `mutants-check` on {what} does not report not applicable with its reason ({why}): {} {}",
            got.summary,
            got.log
        );
    }
    // `pr-check` and `reviews-complete` read no classification: each runs on every pull request, whatever its tier.
    for (file, check) in [
        ("pr-check.yml", "pr-check"),
        ("reviews.yml", "reviews-complete"),
    ] {
        let yml = read(&format!(".github/workflows/{file}"));
        let wf = parse(&yml);
        let (id, job) = jobs(&wf)
            .into_iter()
            .find(|(id, j)| reports(id, j) == check)
            .unwrap_or_else(|| panic!("ci_changes: {file} reports no `{check}`"));
        assert!(
            job.get("if").is_none() && job.get("needs").is_none(),
            "ci_changes: {file}'s `{id}` can be skipped"
        );
        assert!(
            !yml.contains("ci_changes.sh") && !yml.contains("docs_only"),
            "ci_changes: {file} reads the classification"
        );
    }
}

#[test]
fn ci_changes_every_required_check_reports_on_every_tier() {
    check_every_tier_reports(
        &read(".github/workflows/ci.yml"),
        &read(".github/workflows/mutants.yml"),
    );
}

mod workspace_skipped_on_docs {
    use super::*;
    negative_control!(
        ci_changes_every_required_check_reports_on_every_tier,
        "ci.yml whose ci-workspace skips on a data or docs change, so it passes no verdict on to `ci`",
        expected = "the required check `ci` does not pass on a data change (the plan's task data)",
        check_every_tier_reports(
            &edited(
                ".github/workflows/ci.yml",
                "  ci-workspace:\n    runs-on: ubuntu-24.04\n",
                "  ci-workspace:\n    if: needs.changes.outputs.docs_only != 'true'\n    runs-on: ubuntu-24.04\n",
            ),
            &read(".github/workflows/mutants.yml"),
        )
    );
}

mod xtask_ci_docs_tier_only {
    use super::*;
    negative_control!(
        ci_changes_every_required_check_reports_on_every_tier,
        "ci.yml whose xtask-ci gate passes a skip on the docs tier alone",
        expected = "the required check `xtask-ci` does not pass on a data change (the plan's task data)",
        check_every_tier_reports(
            &edited(
                ".github/workflows/ci.yml",
                "          DOCS_ONLY: ${{ needs.changes.result == 'success' && needs.changes.outputs.docs_only == 'true' }}\n",
                "          DOCS_ONLY: ${{ needs.changes.result == 'success' && needs.changes.outputs.tier == 'docs' }}\n",
            ),
            &read(".github/workflows/mutants.yml"),
        )
    );
}

mod mutants_pending_on_data {
    use super::*;
    negative_control!(
        ci_changes_every_required_check_reports_on_every_tier,
        "mutants.yml's mutants-check reading no classification, so a data change's reports not merge-ready",
        expected = "`mutants-check` on a data change (the plan's task data) does not report not applicable with its reason",
        check_every_tier_reports(
            &read(".github/workflows/ci.yml"),
            &edited(
                ".github/workflows/mutants.yml",
                "          if [ \"$MUTANTS\" = false ]; then\n",
                "          if [ \"$MUTANTS\" = never ]; then\n",
            ),
        )
    );
}

mod gpu_gate_on_docs_only {
    use super::*;
    negative_control!(
        ci_changes_every_required_check_reports_on_every_tier,
        "ci.yml whose gpu-kernel suites run on a docs change alone, so a code change skips them and the gate fails",
        expected = "the required check `gpu-kernel` does not pass on a code change",
        check_every_tier_reports(
            &edited(
                ".github/workflows/ci.yml",
                "    name: gpu-kernel suites\n    needs: changes\n    if: needs.changes.outputs.gpu == 'true'\n",
                "    name: gpu-kernel suites\n    needs: changes\n    if: needs.changes.outputs.tier == 'docs'\n",
            ),
            &read(".github/workflows/mutants.yml"),
        )
    );
}

// ---- Every required check reports on every path ----------------------------------------------------------------

/// The checks `plan/HUMAN_SETUP.md` §2 records branch protection as requiring on `main`: the names in its bullet
/// "require status checks to pass: …" before its first parenthesis, and the one in "require `…` too".
fn recorded_required(setup: &str) -> Vec<String> {
    let section: Vec<&str> = setup
        .lines()
        .skip_while(|l| !l.starts_with("## 2."))
        .skip(1)
        .take_while(|l| !l.starts_with("## "))
        .collect();
    let mut bullets: Vec<String> = Vec::new();
    for line in section {
        if let Some(b) = line.strip_prefix("- ") {
            bullets.push(b.to_owned());
        } else if let (Some(last), true) = (bullets.last_mut(), line.starts_with("  ")) {
            last.push(' ');
            last.push_str(line.trim());
        }
    }
    let ticked =
        |t: &str| -> Vec<String> { t.split('`').skip(1).step_by(2).map(str::to_owned).collect() };
    let mut names = Vec::new();
    for b in &bullets {
        if let Some(rest) = b.strip_prefix("require status checks to pass:") {
            names.extend(ticked(rest.split('(').next().unwrap_or("")));
        } else if b.starts_with("require `") {
            let first = ticked(b).into_iter().next().unwrap_or_default();
            if b.contains(&format!("`{first}` too")) {
                names.push(first);
            }
        }
    }
    names
}

/// A pull request's event on its head, as GitHub starts runs on it: what it is, the event's action, the label it
/// names, whether the head carries `merge-ready` (the event's labels), the change's paths, and what `mutants-check`
/// reports in `mutants.yml`'s run of it, the merge-ready run's verdict and that head's earlier merge-ready run passing.
struct EventPath {
    what: &'static str,
    action: &'static str,
    label: &'static str,
    ready: bool,
    paths: &'static [&'static str],
    mutants_check: &'static str,
}

const CODE_PATHS: &[&str] = &["crates/kernel/src/lib.rs"];
const PLAN_PATHS: &[&str] = &["plan/tasks.yaml"];
const DOCS_PATHS: &[&str] = &["docs/design/principia_dd_replication.md"];

/// The paths the owner named (3 Nov 2026, R-678): label added, label removed, docs-only, plan-data and code.
const EVENT_PATHS: &[EventPath] = &[
    EventPath {
        what: "code, a push",
        action: "synchronize",
        label: "",
        ready: false,
        paths: CODE_PATHS,
        mutants_check: NOT_READY,
    },
    EventPath {
        what: "code, a push to a merge-ready head",
        action: "synchronize",
        label: "",
        ready: true,
        paths: CODE_PATHS,
        mutants_check: NOT_READY,
    },
    EventPath {
        what: "code, opened",
        action: "opened",
        label: "",
        ready: false,
        paths: CODE_PATHS,
        mutants_check: NOT_READY,
    },
    EventPath {
        what: "plan-data, a push",
        action: "synchronize",
        label: "",
        ready: false,
        paths: PLAN_PATHS,
        mutants_check: NO_CRATES,
    },
    EventPath {
        what: "docs-only, a push",
        action: "synchronize",
        label: "",
        ready: false,
        paths: DOCS_PATHS,
        mutants_check: NO_CRATES,
    },
    EventPath {
        what: "label added: merge-ready, code",
        action: "labeled",
        label: "merge-ready",
        ready: true,
        paths: CODE_PATHS,
        mutants_check: "no surviving mutant",
    },
    EventPath {
        what: "label added: merge-ready, docs-only",
        action: "labeled",
        label: "merge-ready",
        ready: true,
        paths: DOCS_PATHS,
        mutants_check: "no surviving mutant",
    },
    EventPath {
        what: "label added: another, code",
        action: "labeled",
        label: "design",
        ready: false,
        paths: CODE_PATHS,
        mutants_check: NOT_READY,
    },
    EventPath {
        what: "label added: another, on a merge-ready code head",
        action: "labeled",
        label: "design",
        ready: true,
        paths: CODE_PATHS,
        mutants_check: "no surviving mutant",
    },
    EventPath {
        what: "label added: another, plan-data",
        action: "labeled",
        label: "design",
        ready: true,
        paths: PLAN_PATHS,
        mutants_check: NO_CRATES,
    },
    EventPath {
        what: "label removed: merge-ready, code",
        action: "unlabeled",
        label: "merge-ready",
        ready: false,
        paths: CODE_PATHS,
        mutants_check: NOT_READY,
    },
    EventPath {
        what: "label removed: another, on a merge-ready code head",
        action: "unlabeled",
        label: "design",
        ready: true,
        paths: CODE_PATHS,
        mutants_check: "no surviving mutant",
    },
    EventPath {
        what: "label removed: another, docs-only",
        action: "unlabeled",
        label: "design",
        ready: false,
        paths: DOCS_PATHS,
        mutants_check: NO_CRATES,
    },
];

/// The `pull_request` activity types a workflow runs on: its `types:`, or GitHub's defaults.
fn pr_types(wf: &Value) -> Vec<String> {
    wf["on"]["pull_request"]["types"]
        .as_array()
        .map(|a| a.iter().map(text).collect())
        .unwrap_or_else(|| {
            ["opened", "synchronize", "reopened"]
                .map(str::to_owned)
                .to_vec()
        })
}

/// R-678: every check branch protection requires (`plan/HUMAN_SETUP.md` §2, which `REQUIRED` must match) reports on
/// every path, as GitHub counts checks: of each workflow's runs on the head, only the latest run's check suite. On each
/// path, the workflow bearing the check either runs, and then the job bearing it cannot be skipped (no `if:` and no
/// `needs:`, or `if: always()`) and, in `ci.yml` and `mutants.yml`, concludes as it should; or it does not run on the
/// event, and then its run on the head's push stands, which it must have. A workflow that runs on one label change runs
/// on both, so a label taken off never leaves the run of its putting on standing; no workflow bearing a check filters
/// its pull requests.
fn check_every_path_reports(workflows: &[(String, String)], setup: &str) {
    let mut recorded = recorded_required(setup);
    recorded.sort();
    let mut required: Vec<String> = REQUIRED.iter().map(|s| (*s).to_owned()).collect();
    required.sort();
    assert_eq!(
        recorded, required,
        "ci_changes: plan/HUMAN_SETUP.md §2's required checks are not this test's"
    );
    let script = read("scripts/ci_changes.sh");
    // The workflows a pull request runs: the rest (`gate.yml`'s `gpu-metal` among them) report nothing on one.
    let parsed: Vec<(String, Value)> = workflows
        .iter()
        .map(|(f, yml)| (f.clone(), parse(yml)))
        .filter(|(_, wf)| {
            let on = &wf["on"];
            on.get("pull_request").is_some()
                || on
                    .as_array()
                    .is_some_and(|a| a.iter().any(|t| t == "pull_request"))
                || on.as_str() == Some("pull_request")
        })
        .collect();
    for path in EVENT_PATHS {
        let v = classify(&script, path.paths);
        let verdict = [
            ("tier", v.tier.clone()),
            ("docs_only", (v.tier != "code").to_string()),
            ("gpu", v.gpu.to_string()),
            ("plan_data", v.plan_data.to_string()),
            ("gui_data", v.gui_data.to_string()),
            ("mutants", v.mutants.to_string()),
        ];
        let mut ci_reported: Option<Vec<(String, bool)>> = None;
        for check in REQUIRED {
            let owners: Vec<(&String, &Value, String)> = parsed
                .iter()
                .flat_map(|(f, wf)| {
                    jobs(wf)
                        .into_iter()
                        .filter(|(id, j)| reports(id, j) == check)
                        .map(move |(id, _)| (f, wf, id))
                })
                .collect();
            assert_eq!(
                owners.len(),
                1,
                "ci_changes: `{check}` is reported by no job, or by more than one, on {}",
                path.what
            );
            let (file, wf, id) = &owners[0];
            let job = &wf["jobs"][id.as_str()];
            let pr = &wf["on"]["pull_request"];
            for key in ["paths", "paths-ignore", "branches", "branches-ignore"] {
                assert!(
                    pr.get(key).is_none(),
                    "ci_changes: `{check}` is not reported on {}: {file} filters its pull requests with `{key}:`",
                    path.what
                );
            }
            let types = pr_types(wf);
            let on = |t: &str| types.iter().any(|x| x == t);
            assert!(
                on("labeled") == on("unlabeled"),
                "ci_changes: `{check}` is not reported on {}: {file} runs on one label change and not the other",
                path.what
            );
            if !on(path.action) {
                // The head's push run's suite stands: the workflow must run on every new head.
                assert!(
                    on("opened") && on("synchronize") && on("reopened"),
                    "ci_changes: `{check}` is not reported on {}: {file} runs neither on it nor on every new head",
                    path.what
                );
                continue;
            }
            // The job runs on this event: no `if:` and no `needs:`, or an `if:` led by `always()` that holds on it.
            let cond = job.get("if").map(text).unwrap_or_default();
            let results = std::collections::BTreeMap::new();
            let outputs = std::collections::BTreeMap::new();
            let ctx = Ctx {
                action: path.action,
                results: &results,
                outputs: &outputs,
                needs: &[],
            };
            let runs = (cond.is_empty() && job.get("needs").is_none())
                || (cond.trim_start().starts_with("always()") && evaluate(&cond, &ctx) == "true");
            assert!(
                runs,
                "ci_changes: `{check}` is not reported on {}: {file}'s `{id}` can be skipped (if: `{cond}`)",
                path.what
            );
            if file.as_str() == "ci.yml" {
                let reported = ci_reported.get_or_insert_with(|| {
                    let yml = &workflows
                        .iter()
                        .find(|(f, _)| f == "ci.yml")
                        .expect("ci.yml")
                        .1;
                    ci_run(yml, &verdict)
                });
                assert!(
                    reported.iter().any(|(n, pass)| n == check && *pass),
                    "ci_changes: `{check}` does not pass on {} (tier {})",
                    path.what,
                    v.tier
                );
            }
            if check == "mutants-check" {
                let merge_ready = path.action == "labeled" && path.label == "merge-ready";
                // The merge-ready run does not classify; its verdict passed. Any other run's verdict job is skipped.
                let mutants = if merge_ready {
                    String::new()
                } else {
                    v.mutants.to_string()
                };
                let ready = path.ready.to_string();
                let yml = &workflows
                    .iter()
                    .find(|(f, _)| f == file.as_str())
                    .expect("workflow")
                    .1;
                let got = mutants_report(
                    yml,
                    &[
                        ("ACTION", path.action),
                        ("LABEL", path.label),
                        ("READY", ready.as_str()),
                        ("MUTANTS", mutants.as_str()),
                        ("VERDICT", if merge_ready { "success" } else { "skipped" }),
                    ],
                );
                assert!(
                    got.ok && got.summary.contains(path.mutants_check),
                    "ci_changes: `mutants-check` on {} does not pass with \"{}\": {} {}",
                    path.what,
                    path.mutants_check,
                    got.summary,
                    got.log
                );
            }
        }
    }
}

#[test]
fn ci_changes_every_required_check_reports_on_every_path() {
    check_every_path_reports(&pull_request_workflows(), &read("plan/HUMAN_SETUP.md"));
}

/// The pull-request workflows with `file`'s text edited once, `from` to `to`.
#[cfg(feature = "controls")]
fn workflows_edited(file: &str, from: &str, to: &str) -> Vec<(String, String)> {
    let mut workflows = pull_request_workflows();
    let at = workflows
        .iter()
        .position(|(f, _)| f == file)
        .unwrap_or_else(|| panic!("ci_changes: no {file}"));
    workflows[at].1 = edited(&format!(".github/workflows/{file}"), from, to);
    workflows
}

negative_control!(
    ci_changes_every_required_check_reports_on_every_path,
    "mutants.yml's mutants-check skipped on a label taken off",
    expected = "`mutants-check` is not reported on label removed: merge-ready, code: mutants.yml's `mutants-check` can be skipped",
    check_every_path_reports(
        &workflows_edited(
            "mutants.yml",
            "  mutants-check:\n    if: always()\n",
            "  mutants-check:\n    if: always() && github.event.action != 'unlabeled'\n",
        ),
        &read("plan/HUMAN_SETUP.md"),
    )
);

mod path_no_unlabeled {
    use super::*;
    negative_control!(
        ci_changes_every_required_check_reports_on_every_path,
        "mutants.yml not run on a label taken off, so the merge-ready run's check stands once the label is off",
        expected = "`mutants-check` is not reported on code, a push: mutants.yml runs on one label change and not the other",
        check_every_path_reports(
            &workflows_edited(
                "mutants.yml",
                "    types: [opened, synchronize, reopened, labeled, unlabeled]\n",
                "    types: [opened, synchronize, reopened, labeled]\n",
            ),
            &read("plan/HUMAN_SETUP.md"),
        )
    );
}

mod path_posted {
    use super::*;
    negative_control!(
        ci_changes_every_required_check_reports_on_every_path,
        "mutants-check posted through the Checks API, no job bearing it, as #379 had it (#384)",
        expected = "`mutants-check` is reported by no job, or by more than one, on code, a push",
        check_every_path_reports(
            &workflows_edited("mutants.yml", "  mutants-check:\n", "  mutants-report:\n"),
            &read("plan/HUMAN_SETUP.md"),
        )
    );
}

mod path_filtered {
    use super::*;
    negative_control!(
        ci_changes_every_required_check_reports_on_every_path,
        "pr-check.yml run only on code paths, so a docs-only or plan-data head never reports pr-check",
        expected = "`pr-check` is not reported on code, a push: pr-check.yml filters its pull requests with `paths:`",
        check_every_path_reports(
            &workflows_edited(
                "pr-check.yml",
                "    types: [opened, edited, synchronize, reopened, labeled, unlabeled]\n",
                "    types: [opened, edited, synchronize, reopened, labeled, unlabeled]\n    paths: ['crates/**']\n",
            ),
            &read("plan/HUMAN_SETUP.md"),
        )
    );
}

mod path_plan_data {
    use super::*;
    negative_control!(
        ci_changes_every_required_check_reports_on_every_path,
        "ci.yml whose ci-workspace skips on a data or docs change, so `ci` never passes a plan-data head",
        expected = "`ci` does not pass on plan-data, a push (tier data)",
        check_every_path_reports(
            &workflows_edited(
                "ci.yml",
                "  ci-workspace:\n    runs-on: ubuntu-24.04\n",
                "  ci-workspace:\n    if: needs.changes.outputs.docs_only != 'true'\n    runs-on: ubuntu-24.04\n",
            ),
            &read("plan/HUMAN_SETUP.md"),
        )
    );
}

mod path_docs_only {
    use super::*;
    negative_control!(
        ci_changes_every_required_check_reports_on_every_path,
        "ci.yml whose xtask-ci gate passes a skip on the data tier alone, so a docs-only head fails it",
        expected = "`xtask-ci` does not pass on docs-only, a push (tier docs)",
        check_every_path_reports(
            &workflows_edited(
                "ci.yml",
                "          DOCS_ONLY: ${{ needs.changes.result == 'success' && needs.changes.outputs.docs_only == 'true' }}\n",
                "          DOCS_ONLY: ${{ needs.changes.result == 'success' && needs.changes.outputs.tier == 'data' }}\n",
            ),
            &read("plan/HUMAN_SETUP.md"),
        )
    );
}

mod path_label_added {
    use super::*;
    negative_control!(
        ci_changes_every_required_check_reports_on_every_path,
        "mutants.yml's mutants-check passing a merge-ready head not applicable when another label goes on, without its \
         merge-ready run's verdict",
        expected = "`mutants-check` on label added: another, on a merge-ready code head does not pass with \"no surviving mutant\"",
        check_every_path_reports(
            &workflows_edited(
                "mutants.yml",
                "          if [ \"$ACTION\" = synchronize ] || [ \"$READY\" != true ]; then\n",
                "          if true; then\n",
            ),
            &read("plan/HUMAN_SETUP.md"),
        )
    );
}

mod path_recorded {
    use super::*;
    negative_control!(
        ci_changes_every_required_check_reports_on_every_path,
        "plan/HUMAN_SETUP.md §2 with gpu-kernel missing from its required checks",
        expected = "plan/HUMAN_SETUP.md §2's required checks are not this test's",
        check_every_path_reports(
            &pull_request_workflows(),
            &edited(
                "plan/HUMAN_SETUP.md",
                "`xtask-ci`, `gpu-kernel`, ",
                "`xtask-ci`, "
            ),
        )
    );
}

// ---- The mutants gate's self-test ----------------------------------------------------------------------------------

/// Changes `mutants-fixture` must run on, each with what it is: the gate's own configuration (no Rust under `crates/`),
/// a crate's source and tier (b)'s data.
const FIXTURE_RUNS: [(&str, &[&str]); 5] = [
    ("a `.cargo/mutants.toml` change", &[".cargo/mutants.toml"]),
    (
        "a `fixtures/mutants/` change",
        &["fixtures/mutants/untested/src/lib.rs"],
    ),
    (
        "a change to the caps' fixture",
        &["fixtures/mutants/caps/src/lib.rs"],
    ),
    ("a crate's source change", &["crates/kernel/src/lib.rs"]),
    ("a data change (the plan's task data)", &["plan/tasks.yaml"]),
];

/// The change `mutants-fixture` must skip on: docs only.
const FIXTURE_SKIPS: (&str, &[&str]) = (
    "a docs change",
    &["docs/design/principia_dd_replication.md"],
);

/// Which of `mutants.yml`'s `mutants-fixture` steps after its classification run on the verdict `script` gives
/// `paths`: each step's name and whether its `if:` holds. Steps whose `if:` is `always()` (the report upload, which
/// costs nothing) are left out.
fn fixture_steps(yml: &str, script: &str, paths: &[&str]) -> Vec<(String, bool)> {
    use std::collections::BTreeMap;
    let v = classify(script, paths);
    let verdict = [
        ("tier", v.tier.clone()),
        ("docs_only", (v.tier != "code").to_string()),
        ("gpu", v.gpu.to_string()),
        ("plan_data", v.plan_data.to_string()),
        ("gui_data", v.gui_data.to_string()),
        ("mutants", v.mutants.to_string()),
    ];
    let wf = parse(yml);
    let steps = wf["jobs"]["mutants-fixture"]["steps"]
        .as_array()
        .expect("ci_changes: mutants-fixture's steps")
        .clone();
    let after = steps
        .iter()
        .position(|s| {
            text(&s["id"]) == "changes" && text(&s["run"]).trim() == "scripts/ci_changes.sh detect"
        })
        .expect("ci_changes: mutants-fixture does not run the classifier");
    let results = BTreeMap::new();
    let outputs = BTreeMap::new();
    let ctx = Ctx {
        action: "labeled",
        results: &results,
        outputs: &outputs,
        needs: &[],
    };
    let mut out = Vec::new();
    for s in &steps[after + 1..] {
        let mut cond = text(&s["if"]);
        if cond.trim().starts_with("always()") {
            continue;
        }
        for (k, val) in &verdict {
            cond = cond.replace(&format!("steps.changes.outputs.{k}"), &format!("'{val}'"));
        }
        let runs = cond.trim().is_empty() || evaluate(&cond, &ctx) == "true";
        let name = format!("{}{}", text(&s["name"]), text(&s["uses"]));
        out.push((name, runs));
    }
    out
}

fn check_fixture_gate(yml: &str) {
    let script = read("scripts/ci_changes.sh");
    for (what, paths) in FIXTURE_RUNS {
        let steps = fixture_steps(yml, &script, paths);
        assert!(
            steps.len() >= 2,
            "ci_changes: mutants-fixture has no work after its classification"
        );
        for (name, runs) in &steps {
            assert!(
                runs,
                "ci_changes: mutants-fixture skips `{name}` on {what}, a change that configures or reaches the gate"
            );
        }
    }
    let (what, paths) = FIXTURE_SKIPS;
    for (name, runs) in fixture_steps(yml, &script, paths) {
        assert!(
            !runs,
            "ci_changes: mutants-fixture runs `{name}` on {what}, which reaches no gate"
        );
    }
}

#[test]
fn ci_changes_mutants_fixture_runs_on_the_gates_own_files() {
    check_fixture_gate(&read(".github/workflows/mutants.yml"));
}

#[cfg(feature = "controls")]
/// `mutants.yml` with every `mutants-fixture` step gated on `from` replaced by `to`.
fn fixture_gated(to: &str) -> String {
    let text = read(".github/workflows/mutants.yml");
    let from = "        if: steps.changes.outputs.tier != 'docs'\n";
    assert!(
        text.matches(from).count() >= 2,
        "ci_changes: the control's edit target is gone from mutants.yml: {from}"
    );
    text.replace(from, to)
}

mod fixture_on_crates_rust_only {
    use super::*;
    negative_control!(
        ci_changes_mutants_fixture_runs_on_the_gates_own_files,
        "mutants.yml whose fixture steps run only when Rust under crates/ changed",
        expected =
            "on a `.cargo/mutants.toml` change, a change that configures or reaches the gate",
        check_fixture_gate(&fixture_gated(
            "        if: steps.changes.outputs.mutants != 'false'\n"
        ))
    );
}

mod fixture_on_code_tier_only {
    use super::*;
    negative_control!(
        ci_changes_mutants_fixture_runs_on_the_gates_own_files,
        "mutants.yml whose fixture steps run only on tier (a)",
        expected =
            "on a data change (the plan's task data), a change that configures or reaches the gate",
        check_fixture_gate(&fixture_gated(
            "        if: steps.changes.outputs.docs_only != 'true'\n"
        ))
    );
}

mod fixture_on_docs_too {
    use super::*;
    negative_control!(
        ci_changes_mutants_fixture_runs_on_the_gates_own_files,
        "mutants.yml whose fixture steps run on every change",
        expected = "on a docs change, which reaches no gate",
        check_fixture_gate(&fixture_gated(
            "        if: steps.changes.outputs.tier != ''\n"
        ))
    );
}
