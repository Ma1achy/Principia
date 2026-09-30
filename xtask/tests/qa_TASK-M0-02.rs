//! QA tests for TASK-M0-02, written from REQ-SYS-008 ("files marked archived — `principia_ARCHIVE_*` and the
//! Archived table — must not be implemented from and must never be cited"; verify: "requirement/source and
//! task-reference audit shows nothing sourced from docs/archive or an ARCHIVE_ brief"), from R-184 ("TASK-M0-02
//! includes the `check_plan.py` change that names archive faults") and from the task's Goal and Deliverables, not
//! from the implementation:
//!
//! - "a plan that loses a requirement, closes one twice, cites a section that doesn't exist — including any section
//!   of an archived file — or disagrees with its task files turns CI red";
//! - "`cargo xtask plan-check`: runs `python3 plan/check_plan.py` from the repo root, streams its output, exits with
//!   its status; a clear error if `python3` or PyYAML is missing";
//! - "`plan-check` registered in `cargo xtask ci`", with Python and PyYAML set up in CI.
//!
//! The archive cases cover the INDEX's Archived table beyond the implementer's two files: a `docs/archive/` file
//! without `ARCHIVE_` in its name, a nested `docs/archive/untangling/` file, and the kernel-build brief, each cited at
//! a heading that really exists in it, so the only fault is that the file is archived. Each test's control (R-176,
//! R-199) runs the same assertion on an input differing in the one respect the requirement turns on (a live
//! consolidated-doc citation in place of the archived one; an unedited plan; a working `python3`) and trips it.
// The file name `qa_TASK-M0-02` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use validation::negative_control;
use validation::spawn::Spawn;
use xtask::plan_check::repo_root;

// --- A copy of the plan and the corpus --------------------------------------------------------------------------------

/// Copies `from` into `to`: every file, or with `md_only` only `.md` files; `__pycache__` is skipped.
fn copy_tree(from: &Path, to: &Path, md_only: bool) {
    fs::create_dir_all(to).expect("qa02: dir created");
    for entry in fs::read_dir(from).expect("qa02: dir read") {
        let path = entry.expect("qa02: entry read").path();
        let name = path.file_name().expect("qa02: named");
        if path.is_dir() {
            if name != "__pycache__" {
                copy_tree(&path, &to.join(name), md_only);
            }
        } else if !md_only || path.extension().is_some_and(|e| e == "md") {
            fs::copy(&path, to.join(name)).expect("qa02: file copied");
        }
    }
}

/// A fresh copy of `plan/`, the markdown corpus (archive included) and the root files, under `name`.
fn copy(name: &str) -> PathBuf {
    let src = repo_root();
    let dst = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("qa02_{name}"));
    let _ = fs::remove_dir_all(&dst);
    copy_tree(&src.join("plan"), &dst.join("plan"), false);
    copy_tree(&src.join("docs"), &dst.join("docs"), true);
    for file in ["decisions.md", "open-questions.md", "REVIEW_QUEUE.md"] {
        fs::copy(src.join(file), dst.join(file)).expect("qa02: root file copied");
    }
    dst
}

/// In `root`'s `file`, replaces the first `from` found after `anchor` with `to`.
fn edit(root: &Path, file: &str, anchor: &str, from: &str, to: &str) {
    let path = root.join(file);
    let text = fs::read_to_string(&path).expect("qa02: file read");
    let at = text
        .find(anchor)
        .unwrap_or_else(|| panic!("qa02: anchor {anchor:?} in {file}"));
    let hit = at
        + text[at..]
            .find(from)
            .unwrap_or_else(|| panic!("qa02: {from:?} after {anchor:?} in {file}"));
    let out = format!("{}{to}{}", &text[..hit], &text[hit + from.len()..]);
    fs::write(&path, out).expect("qa02: file written");
}

/// Appends `line` to `root`'s `file`.
fn append(root: &Path, file: &str, line: &str) {
    let path = root.join(file);
    let mut text = fs::read_to_string(&path).expect("qa02: file read");
    text.push_str(line);
    fs::write(&path, text).expect("qa02: file written");
}

/// Runs `python3 plan/check_plan.py` in `root`: (passed, stdout + stderr).
fn check(root: &Path) -> (bool, String) {
    let out = Command::new("python3")
        .arg("plan/check_plan.py")
        .current_dir(root)
        .timed_output()
        .expect("qa02: python3 ran plan/check_plan.py");
    let text =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    (out.status.success(), text)
}

/// The checker fails on `root`, and one of its `FAIL:` lines contains every string in `fault`.
fn fails_naming(root: &Path, fault: &[&str]) {
    let (passed, out) = check(root);
    assert!(!passed, "qa02: plan-check passed:\n{out}");
    assert!(
        out.lines()
            .any(|l| l.starts_with("FAIL:") && fault.iter().all(|f| l.contains(f))),
        "qa02: plan-check did not name the fault {fault:?}:\n{out}"
    );
}

// --- REQ-SYS-008: an archived file is never cited ----------------------------------------------------------------------

/// A live, citable section: the consolidated doc that supersedes the briefs (R-112).
const LIVE_FILE: &str = "docs/design/principia_deep_zoom.md";
const LIVE_SECTION: &str = "3. Three-layer quadtree";

/// From the INDEX's Archived table: in `docs/archive/`, without `ARCHIVE_` in its name.
const PARITY_NOTE: (&str, &str) = (
    "docs/archive/principia_parity_testing_note.md",
    "Three assertion tiers",
);
/// From the INDEX's Archived table: nested under `docs/archive/untangling/`.
const MOVES: (&str, &str) = ("docs/archive/untangling/MOVES.md", "Moves — step 2");
/// From the INDEX's Archived table: an `ARCHIVE_` brief outside `docs/archive/`, at its §5.
const KERNEL_BRIEF: (&str, &str) = (
    "docs/experiments/briefs/principia_ARCHIVE_brief_kernel_build.md",
    "5. Verification — do this before trusting any output",
);
/// From the INDEX's Archived table: in `docs/archive/` and named `ARCHIVE_`.
const CRITERION_V0: (&str, &str) = (
    "docs/archive/principia_ARCHIVE_dd_refinement_criterion_v0.md",
    "1. Summary",
);

/// REQ-SCHED-001's first source (deep_zoom §3) re-pointed at `file` § `section`.
fn source_at(root: &Path, file: &str, section: &str) {
    edit(
        root,
        "plan/requirements.yaml",
        "- id: REQ-SCHED-001\n",
        &format!("- file: {LIVE_FILE}\n    section: {LIVE_SECTION}\n"),
        &format!("- file: {file}\n    section: \"{section}\"\n"),
    );
}

/// TASK-M0-02's deep_zoom reference re-pointed at `file` § `section`.
fn reference_at(root: &Path, file: &str, section: &str) {
    edit(
        root,
        "plan/tasks/M0/TASK-M0-02.md",
        "## References\n",
        &format!("- `{LIVE_FILE}` § \"{LIVE_SECTION}\""),
        &format!("- `{file}` § \"{section}\""),
    );
}

/// A requirement source at `(file, section)` fails plan-check with "cites archived file <file>" (R-184).
fn source_refused(name: &str, (file, section): (&str, &str)) {
    let root = copy(name);
    source_at(&root, file, section);
    fails_naming(
        &root,
        &["REQ-SCHED-001", &format!("cites archived file {file}")],
    );
}

/// A task reference at `(file, section)` fails plan-check with "cites archived file <file>" (R-184).
fn reference_refused(name: &str, (file, section): (&str, &str)) {
    let root = copy(name);
    reference_at(&root, file, section);
    fails_naming(
        &root,
        &[
            "plan/tasks/M0/TASK-M0-02.md",
            &format!("cites archived file {file}"),
        ],
    );
}

#[test]
fn qa02_source_into_archive_without_archive_prefix_is_refused() {
    source_refused("src_parity", PARITY_NOTE);
}

#[test]
fn qa02_source_into_nested_archive_dir_is_refused() {
    source_refused("src_moves", MOVES);
}

#[test]
fn qa02_source_into_kernel_build_brief_is_refused() {
    source_refused("src_kernel", KERNEL_BRIEF);
}

#[test]
fn qa02_reference_into_archive_without_archive_prefix_is_refused() {
    reference_refused("ref_parity", PARITY_NOTE);
}

#[test]
fn qa02_reference_to_archived_criterion_is_refused() {
    reference_refused("ref_criterion", CRITERION_V0);
}

#[test]
fn qa02_reference_to_kernel_build_brief_is_refused() {
    reference_refused("ref_kernel", KERNEL_BRIEF);
}

/// A reviewer checklist item citing `(file, section)`, appended to `qa.md`.
fn checklist_cites(root: &Path, (file, section): (&str, &str)) {
    append(
        root,
        "plan/reviewers/qa.md",
        &format!("- [ ] A QA probe item. `{file}` § \"{section}\"\n"),
    );
}

/// "Never cited" reaches the reviewer checklists too: the check fails naming the archived file.
fn checklist_refused(name: &str, cited: (&str, &str)) {
    let root = copy(name);
    checklist_cites(&root, cited);
    fails_naming(&root, &["plan/reviewers/qa.md", cited.0]);
}

#[test]
fn qa02_reviewer_checklist_citing_archive_is_refused() {
    checklist_refused("chk_parity", PARITY_NOTE);
}

// Controls: the same edit with the consolidated doc's live section in place of the archived file passes the check, so
// the refusal assertion goes red on "plan-check passed".

negative_control!(
    qa02_source_into_archive_without_archive_prefix_is_refused,
    "the source pointed at a live consolidated-doc section passes, so the refusal must fail",
    expected = "qa02: plan-check passed",
    source_refused("ctl_src_parity", (LIVE_FILE, LIVE_SECTION))
);

negative_control!(
    qa02_source_into_nested_archive_dir_is_refused,
    "the source pointed at a live consolidated-doc section passes, so the refusal must fail",
    expected = "qa02: plan-check passed",
    source_refused("ctl_src_moves", (LIVE_FILE, LIVE_SECTION))
);

negative_control!(
    qa02_source_into_kernel_build_brief_is_refused,
    "a source at a missing non-archived file fails as \"doesn't exist\", not naming the archive",
    expected = "qa02: plan-check did not name the fault",
    source_refused(
        "ctl_src_kernel",
        (
            "docs/design/principia_qa_no_such_file.md",
            "5. Verification"
        )
    )
);

negative_control!(
    qa02_reference_into_archive_without_archive_prefix_is_refused,
    "the reference pointed at a live consolidated-doc section passes, so the refusal must fail",
    expected = "qa02: plan-check passed",
    reference_refused("ctl_ref_parity", (LIVE_FILE, LIVE_SECTION))
);

negative_control!(
    qa02_reference_to_archived_criterion_is_refused,
    "the reference pointed at a live consolidated-doc section passes, so the refusal must fail",
    expected = "qa02: plan-check passed",
    reference_refused("ctl_ref_criterion", (LIVE_FILE, LIVE_SECTION))
);

negative_control!(
    qa02_reference_to_kernel_build_brief_is_refused,
    "a reference to a missing non-archived file fails as \"doesn't exist\", not naming the archive",
    expected = "qa02: plan-check did not name the fault",
    reference_refused(
        "ctl_ref_kernel",
        (
            "docs/design/principia_qa_no_such_file.md",
            "5. Verification"
        )
    )
);

negative_control!(
    qa02_reviewer_checklist_citing_archive_is_refused,
    "a checklist item citing a live consolidated-doc section passes, so the refusal must fail",
    expected = "qa02: plan-check passed",
    checklist_refused("ctl_chk_parity", (LIVE_FILE, LIVE_SECTION))
);

// --- The Goal: lost, doubly closed, missing section, disagreeing task file -> red -------------------------------------

/// TASK-M0-02's manifest line of closed requirements.
const M0_02_CLOSES: &str = "requirements: [REQ-SYS-007, REQ-SYS-008, REQ-SCHED-001, REQ-TOOL-004]";

/// A requirement no task closes: REQ-SYS-007 dropped from TASK-M0-02, in `tasks.yaml` and its task file alike.
fn lose_requirement(root: &Path) {
    edit(
        root,
        "plan/tasks.yaml",
        "- id: TASK-M0-02\n",
        M0_02_CLOSES,
        "requirements: [REQ-SYS-008, REQ-SCHED-001, REQ-TOOL-004]",
    );
    edit(
        root,
        "plan/tasks/M0/TASK-M0-02.md",
        "- **Closes:**",
        "REQ-SYS-007, ",
        "",
    );
}

#[test]
fn qa02_a_lost_requirement_turns_the_check_red() {
    let root = copy("lost");
    lose_requirement(&root);
    fails_naming(&root, &["REQ-SYS-007", "no task closes it"]);
}

negative_control!(
    qa02_a_lost_requirement_turns_the_check_red,
    "an unedited copy passes, so the failure assertion must fail",
    expected = "qa02: plan-check passed",
    fails_naming(&copy("ctl_lost"), &["REQ-SYS-007", "no task closes it"])
);

/// REQ-SYS-004, closed by TASK-M0-01, also closed by TASK-M0-02 (in `tasks.yaml` and its task file alike).
fn close_twice(root: &Path) {
    edit(
        root,
        "plan/tasks.yaml",
        "- id: TASK-M0-02\n",
        M0_02_CLOSES,
        "requirements: [REQ-SYS-007, REQ-SYS-008, REQ-SCHED-001, REQ-TOOL-004, REQ-SYS-004]",
    );
    edit(
        root,
        "plan/tasks/M0/TASK-M0-02.md",
        "- **Closes:**",
        "REQ-TOOL-004",
        "REQ-TOOL-004, REQ-SYS-004",
    );
}

#[test]
fn qa02_a_requirement_closed_twice_turns_the_check_red() {
    let root = copy("twice");
    close_twice(&root);
    fails_naming(&root, &["REQ-SYS-004", "more than one task"]);
}

negative_control!(
    qa02_a_requirement_closed_twice_turns_the_check_red,
    "an unedited copy passes, so the failure assertion must fail",
    expected = "qa02: plan-check passed",
    fails_naming(&copy("ctl_twice"), &["REQ-SYS-004", "more than one task"])
);

#[test]
fn qa02_a_missing_section_turns_the_check_red() {
    let root = copy("section");
    reference_at(&root, LIVE_FILE, "3. A heading deep_zoom does not have");
    fails_naming(
        &root,
        &[
            "plan/tasks/M0/TASK-M0-02.md",
            "A heading deep_zoom does not have",
        ],
    );
}

negative_control!(
    qa02_a_missing_section_turns_the_check_red,
    "the reference at deep_zoom's real §3 heading passes, so the failure assertion must fail",
    expected = "qa02: plan-check passed",
    {
        let root = copy("ctl_section");
        reference_at(&root, LIVE_FILE, LIVE_SECTION);
        fails_naming(&root, &["plan/tasks/M0/TASK-M0-02.md", LIVE_SECTION]);
    }
);

#[test]
fn qa02_a_task_file_disagreeing_with_tasks_yaml_turns_the_check_red() {
    let root = copy("disagree");
    edit(
        &root,
        "plan/tasks/M0/TASK-M0-02.md",
        "- **Reviewers:**",
        "code, qa",
        "code, qa, perf",
    );
    fails_naming(&root, &["plan/tasks/M0/TASK-M0-02.md", "Reviewers"]);
}

negative_control!(
    qa02_a_task_file_disagreeing_with_tasks_yaml_turns_the_check_red,
    "an unedited copy passes, so the failure assertion must fail",
    expected = "qa02: plan-check passed",
    fails_naming(
        &copy("ctl_disagree"),
        &["plan/tasks/M0/TASK-M0-02.md", "Reviewers"]
    )
);

// --- `cargo xtask plan-check`: runs the checker, streams it, exits with its status; clear errors ----------------------

/// The `xtask` binary run as `xtask plan-check`, with `PATH` replaced by `path` when given.
fn xtask_plan_check(path: Option<&Path>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_xtask"));
    command.arg("plan-check");
    if let Some(path) = path {
        command.env("PATH", path);
    }
    command.timed_output().expect("qa02: xtask ran")
}

/// A directory holding only a fake `python3` whose `-c` probe exits `probe` (printing a ModuleNotFoundError when
/// non-zero) and whose script run prints a marker and exits `code`.
fn shim(name: &str, probe: i32, code: i32) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("qa02_shim_{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("qa02: shim dir");
    let script = format!(
        "#!/bin/sh\nif [ \"$1\" = \"-c\" ]; then\n  if [ {probe} -ne 0 ]; then echo \"ModuleNotFoundError: No module \
         named 'yaml'\" >&2; fi\n  exit {probe}\nfi\necho \"QA02-SHIM-RAN $*\"\nexit {code}\n"
    );
    let file = dir.join("python3");
    validation::spawn::write_executable(&file, script).expect("qa02: shim written");
    dir
}

/// Stdout and stderr of `out`.
fn texts(out: &Output) -> (String, String) {
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// `xtask plan-check` under `path` exits 0 and streams the checker's own output, which names `plan/check_plan.py`'s
/// summary (the real checker) or the shim's marker.
fn passes_streaming(path: Option<&Path>, marker: &str) {
    let out = xtask_plan_check(path);
    let (stdout, stderr) = texts(&out);
    assert!(
        out.status.success(),
        "qa02: xtask plan-check exited {:?}:\n{stdout}\n{stderr}",
        out.status.code()
    );
    assert!(
        stdout.contains(marker),
        "qa02: xtask plan-check did not stream the checker's output {marker:?}:\n{stdout}"
    );
}

#[test]
fn qa02_xtask_plan_check_passes_on_the_tree_and_streams() {
    passes_streaming(None, "live requirements, 0 failures");
}

negative_control!(
    qa02_xtask_plan_check_passes_on_the_tree_and_streams,
    "a python3 whose checker run exits 1 must fail the passing assertion",
    expected = "qa02: xtask plan-check exited",
    passes_streaming(Some(&shim("ctl_tree", 0, 1)), "QA02-SHIM-RAN")
);

/// `xtask plan-check` exits with exactly the checker's status `code`, having run `plan/check_plan.py`.
fn exits_with(path: &Path, code: i32) {
    let out = xtask_plan_check(Some(path));
    let (stdout, stderr) = texts(&out);
    assert_eq!(
        out.status.code(),
        Some(code),
        "qa02: xtask plan-check did not exit with the checker's status:\n{stdout}\n{stderr}"
    );
    assert!(
        stdout.contains("QA02-SHIM-RAN plan/check_plan.py"),
        "qa02: xtask plan-check did not run plan/check_plan.py:\n{stdout}"
    );
}

#[test]
fn qa02_xtask_plan_check_exits_with_the_checker_status() {
    exits_with(&shim("status3", 0, 3), 3);
    exits_with(&shim("status1", 0, 1), 1);
}

negative_control!(
    qa02_xtask_plan_check_exits_with_the_checker_status,
    "a checker exiting 0 does not give status 3",
    expected = "qa02: xtask plan-check did not exit with the checker's status",
    exits_with(&shim("ctl_status", 0, 0), 3)
);

/// `xtask plan-check` under `path` fails without running the checker, its stderr naming every string in `names`.
fn fails_clearly(path: &Path, names: &[&str]) {
    let out = xtask_plan_check(Some(path));
    let (stdout, stderr) = texts(&out);
    assert!(
        !out.status.success(),
        "qa02: xtask plan-check succeeded:\n{stdout}\n{stderr}"
    );
    assert!(
        !stdout.contains("QA02-SHIM-RAN"),
        "qa02: xtask plan-check ran the checker anyway:\n{stdout}"
    );
    assert!(
        names.iter().all(|n| stderr.contains(n)),
        "qa02: xtask plan-check's error does not name {names:?}:\n{stderr}"
    );
}

/// An empty directory: `PATH` with no `python3` on it.
fn no_python() -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("qa02_no_python");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("qa02: empty dir");
    dir
}

/// `xtask plan-check` under `path` fails, its error saying that `python3` is missing, not merely failing to run it.
fn python3_reported_missing(path: &Path) {
    fails_clearly(path, &["python3"]);
    let (_, stderr) = texts(&xtask_plan_check(Some(path)));
    let lower = stderr.to_lowercase();
    assert!(
        lower.contains("not found") || lower.contains("missing"),
        "qa02: xtask plan-check's error does not say python3 is missing:\n{stderr}"
    );
}

#[test]
fn qa02_missing_python3_is_a_clear_error() {
    python3_reported_missing(&no_python());
}

negative_control!(
    qa02_missing_python3_is_a_clear_error,
    "with a working python3 on PATH the check succeeds, so the error assertion must fail",
    expected = "qa02: xtask plan-check succeeded",
    python3_reported_missing(&shim("ctl_python", 0, 0))
);

#[test]
fn qa02_missing_pyyaml_is_a_clear_error() {
    fails_clearly(&shim("no_yaml", 1, 0), &["PyYAML"]);
}

negative_control!(
    qa02_missing_pyyaml_is_a_clear_error,
    "with PyYAML importable the check succeeds, so the error assertion must fail",
    expected = "qa02: xtask plan-check succeeded",
    fails_clearly(&shim("ctl_yaml", 0, 0), &["PyYAML"])
);

// --- Registered in `cargo xtask ci`, with Python and PyYAML set up in its CI job ----------------------------------------

/// `names` holds `plan-check`.
fn registers_plan_check(names: &[&str]) {
    assert!(
        names.contains(&"plan-check"),
        "qa02: plan-check is not a ci runner: {names:?}"
    );
}

#[test]
fn qa02_plan_check_is_a_ci_runner() {
    let names: Vec<&str> = xtask::ci::RUNNERS.iter().map(|r| r.name).collect();
    registers_plan_check(&names);
}

negative_control!(
    qa02_plan_check_is_a_ci_runner,
    "a registry without plan-check",
    expected = "qa02: plan-check is not a ci runner",
    registers_plan_check(&["controls", "lint constants"])
);

/// The lines of job `name` in `.github/workflows/ci.yml`, up to the next job (the next two-space-indented key).
fn job(name: &str) -> String {
    let text =
        fs::read_to_string(repo_root().join(".github/workflows/ci.yml")).expect("qa02: ci.yml");
    let header = format!("  {name}:");
    let lines = text.lines().skip_while(|l| *l != header).skip(1);
    let mut out = String::new();
    for line in lines {
        let key = line.starts_with("  ") && !line.starts_with("   ") && !line.starts_with("  #");
        if key {
            break;
        }
        out.push_str(line);
        out.push('\n');
    }
    assert!(!out.is_empty(), "qa02: job {name} in ci.yml");
    out
}

/// Job `name` runs on push, sets up Python and installs PyYAML, both before it runs `cargo xtask ci`.
fn runs_plan_check_with_python(name: &str) {
    let text =
        fs::read_to_string(repo_root().join(".github/workflows/ci.yml")).expect("qa02: ci.yml");
    assert!(
        text.contains("\n  push:"),
        "qa02: ci.yml does not run on push"
    );
    let steps = job(name);
    let ci = steps.find("run: cargo xtask ci");
    let python = steps.find("setup-python");
    let yaml = steps.to_lowercase().find("pip install pyyaml");
    assert!(
        matches!((ci, python, yaml), (Some(c), Some(p), Some(y)) if p < c && y < c),
        "qa02: job {name} does not set up Python and PyYAML before `cargo xtask ci`:\n{steps}"
    );
}

#[test]
fn qa02_ci_job_sets_up_python_before_xtask_ci() {
    runs_plan_check_with_python("xtask-ci");
}

negative_control!(
    qa02_ci_job_sets_up_python_before_xtask_ci,
    "the gpu-lavapipe job sets up no Python and runs no `cargo xtask ci`",
    expected = "qa02: job gpu-lavapipe does not set up Python and PyYAML",
    runs_plan_check_with_python("gpu-lavapipe")
);

/// `xtask ci --list` (each runner's listing-only form; plan-check's is itself, R-235) with `shim` first on `PATH`
/// fails, and names plan-check as the failed runner: a plan the checker rejects turns `cargo xtask ci` red.
fn ci_fails_on_plan_check(shim: &Path) {
    let path = std::env::join_paths(std::iter::once(shim.to_path_buf()).chain(
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()),
    ))
    .expect("qa02: PATH joined");
    let out = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["ci", "--list"])
        .env("PATH", path)
        .timed_output()
        .expect("qa02: xtask ci --list ran");
    let (stdout, stderr) = texts(&out);
    assert!(
        !out.status.success() && stdout.contains("plan-check FAILED"),
        "qa02: xtask ci did not fail on plan-check:\n{stdout}\n{stderr}"
    );
}

#[test]
fn qa02_a_failing_plan_check_turns_xtask_ci_red() {
    ci_fails_on_plan_check(&shim("ci_red", 0, 1));
}

negative_control!(
    qa02_a_failing_plan_check_turns_xtask_ci_red,
    "a checker exiting 0 leaves plan-check ok in xtask ci",
    expected = "qa02: xtask ci did not fail on plan-check",
    ci_fails_on_plan_check(&shim("ctl_ci_red", 0, 0))
);
