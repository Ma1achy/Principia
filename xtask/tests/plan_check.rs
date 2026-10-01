//! `cargo xtask plan-check` (REQ-SYS-007, REQ-SYS-008): the checker passes on this tree, and fails, naming the fault,
//! on a copy of `plan/` + the corpus with a task dropped from `tasks.yaml`, a requirement source into `docs/archive/`,
//! or a task reference to an `ARCHIVE_` brief (R-184: "cites archived file", not "doesn't exist"). It also fails on a
//! still-open item of `open-questions.md` that names neither a requirement nor an open REVIEW_QUEUE entry, or names a
//! missing or retired requirement, or an RQ that isn't open (R-334).

use std::fs;
use std::path::{Path, PathBuf};

use validation::negative_control;
use validation::spawn::Spawn;
use xtask::plan_check::{command, repo_root};

/// Copies the `.md` files under `from` into `to`, skipping `docs/archive`, `docs/experiments` and `docs/reference`.
/// Of those the checker reads only `docs/archive/review_queue/`, which `copy` adds.
fn copy_md(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("dir created");
    for entry in fs::read_dir(from).expect("dir read") {
        let path = entry.expect("entry read").path();
        let name = path.file_name().expect("named");
        if path.is_dir() {
            if !["archive", "experiments", "reference"].contains(&name.to_str().unwrap_or("")) {
                copy_md(&path, &to.join(name));
            }
        } else if path.extension().is_some_and(|e| e == "md") {
            fs::copy(&path, to.join(name)).expect("file copied");
        }
    }
}

/// Copies every file under `from` into `to`, `__pycache__` excepted.
fn copy_all(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("dir created");
    for entry in fs::read_dir(from).expect("dir read") {
        let path = entry.expect("entry read").path();
        let name = path.file_name().expect("named");
        if path.is_dir() {
            if name != "__pycache__" {
                copy_all(&path, &to.join(name));
            }
        } else {
            fs::copy(&path, to.join(name)).expect("file copied");
        }
    }
}

/// A fresh copy of `plan/`, the corpus, the archived review queue (the checker resolves RQ-n references against it)
/// and the root files the checker reads.
fn copy(name: &str) -> PathBuf {
    let (src, dst) = (
        repo_root(),
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(name),
    );
    let _ = fs::remove_dir_all(&dst);
    copy_all(&src.join("plan"), &dst.join("plan"));
    copy_md(&src.join("docs"), &dst.join("docs"));
    copy_md(
        &src.join("docs/archive/review_queue"),
        &dst.join("docs/archive/review_queue"),
    );
    for file in ["decisions.md", "open-questions.md", "REVIEW_QUEUE.md"] {
        fs::copy(src.join(file), dst.join(file)).expect("file copied");
    }
    dst
}

/// In `root`'s `file`, replaces the first `from` after `after` with `to`.
fn edit(root: &Path, file: &str, after: &str, from: &str, to: &str) {
    let path = root.join(file);
    let text = fs::read_to_string(&path).expect("file read");
    let at = text.find(after).expect("anchor present");
    let hit = at + text[at..].find(from).expect("text present");
    fs::write(
        &path,
        format!("{}{to}{}", &text[..hit], &text[hit + from.len()..]),
    )
    .expect("file written");
}

/// Runs the checker in `root`: (passed, its output).
fn check(root: &Path) -> (bool, String) {
    let out = command(root)
        .timed_output()
        .expect("python3 ran plan/check_plan.py");
    let text =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    (out.status.success(), text)
}

/// The checker fails on `root`, naming `fault`.
fn fails_naming(root: &Path, fault: &str) {
    let (passed, out) = check(root);
    assert!(!passed, "plan-check passed:\n{out}");
    assert!(
        out.contains(fault),
        "plan-check did not name the fault {fault:?}:\n{out}"
    );
}

#[test]
fn plan_check_passes_on_the_tree() {
    let (passed, out) = check(&repo_root());
    assert!(passed, "plan-check failed on the tree:\n{out}");
}

/// Drops `TASK-M0-35`'s entry from `tasks.yaml`.
fn drop_task(root: &Path) {
    let path = root.join("plan/tasks.yaml");
    let text = fs::read_to_string(&path).expect("tasks.yaml read");
    let start = text.find("\n- id: TASK-M0-35\n").expect("entry present") + 1;
    let end = text[start..]
        .find("\n- id:")
        .map_or(text.len(), |i| start + i + 1);
    fs::write(&path, format!("{}{}", &text[..start], &text[end..])).expect("tasks.yaml written");
}

const DROPPED: &str = "plan/tasks/M0/TASK-M0-35.md: not in plan/tasks.yaml";

#[test]
fn plan_check_fails_on_a_task_missing_from_tasks_yaml() {
    let root = copy("plan_check_drop");
    drop_task(&root);
    fails_naming(&root, DROPPED);
}

const ARCHIVED_SOURCE: &str = "docs/archive/principia_ARCHIVE_dd_refinement_criterion_v0.md";

/// Points REQ-SYS-008's first source at `file`.
fn source_into(root: &Path, file: &str) {
    edit(
        root,
        "plan/requirements.yaml",
        "- id: REQ-SYS-008\n",
        "file: docs/read_first/principia_INDEX.md",
        &format!("file: {file}"),
    );
}

#[test]
fn plan_check_fails_on_a_requirement_source_into_the_archive() {
    let root = copy("plan_check_source");
    source_into(&root, ARCHIVED_SOURCE);
    fails_naming(
        &root,
        &format!("REQ-SYS-008: source cites archived file {ARCHIVED_SOURCE}"),
    );
}

const ARCHIVED_BRIEF: &str =
    "docs/experiments/briefs/principia_ARCHIVE_brief_structure_criterion.md";

/// Points TASK-M0-02's first reference at `file`.
fn reference_to(root: &Path, file: &str) {
    edit(
        root,
        "plan/tasks/M0/TASK-M0-02.md",
        "## References\n",
        "`docs/design/principia_deep_zoom.md` § \"3. Three-layer quadtree\"",
        &format!("`{file}` § \"4. The slippy map\""),
    );
}

#[test]
fn plan_check_fails_on_a_task_reference_to_an_archive_brief() {
    let root = copy("plan_check_reference");
    reference_to(&root, ARCHIVED_BRIEF);
    fails_naming(
        &root,
        &format!("plan/tasks/M0/TASK-M0-02.md: reference cites archived file {ARCHIVED_BRIEF}"),
    );
}

negative_control!(
    plan_check_passes_on_the_tree,
    "a copy with a task dropped from tasks.yaml must fail the passing check",
    expected = "plan-check failed on the tree",
    {
        let root = copy("plan_check_ctl_tree");
        drop_task(&root);
        let (passed, out) = check(&root);
        assert!(passed, "plan-check failed on the tree:\n{out}");
    }
);

negative_control!(
    plan_check_fails_on_a_task_missing_from_tasks_yaml,
    "an unedited copy passes, so the failure check must fail on it",
    expected = "plan-check passed",
    fails_naming(&copy("plan_check_ctl_drop"), DROPPED)
);

negative_control!(
    plan_check_fails_on_a_requirement_source_into_the_archive,
    "a source into a missing, non-archived file fails as \"doesn't exist\", not naming the archive",
    expected = "plan-check did not name the fault",
    {
        let root = copy("plan_check_ctl_source");
        source_into(&root, "docs/design/principia_no_such_file.md");
        fails_naming(&root, "source cites archived file");
    }
);

negative_control!(
    plan_check_fails_on_a_task_reference_to_an_archive_brief,
    "a reference to a missing, non-archived brief fails as \"doesn't exist\", not naming the archive",
    expected = "plan-check did not name the fault",
    {
        let root = copy("plan_check_ctl_reference");
        reference_to(&root, "docs/experiments/briefs/principia_brief_no_such_file.md");
        fails_naming(&root, "reference cites archived file");
    }
);

/// Points the "Carried by" note of `open-questions.md`'s change-10 item (REQ-VAL-036) at `to`.
fn carrier(root: &Path, to: &str) {
    edit(
        root,
        "open-questions.md",
        "\n10. Folded",
        "Carried by: REQ-VAL-036.",
        &format!("Carried by: {to}."),
    );
}

const NEITHER: &str = "still-open item names neither a requirement nor an open REVIEW_QUEUE entry";

#[test]
fn plan_check_fails_on_an_open_item_naming_neither() {
    let root = copy("plan_check_open_neither");
    carrier(&root, "nothing yet");
    fails_naming(&root, NEITHER);
}

const RETIRED: &str = "REQ-CHART-042";

#[test]
fn plan_check_fails_on_an_open_item_naming_a_retired_requirement() {
    let root = copy("plan_check_open_retired");
    carrier(&root, RETIRED);
    fails_naming(
        &root,
        &format!("still-open item names retired requirement {RETIRED}"),
    );
}

#[test]
fn plan_check_fails_on_an_open_item_naming_a_missing_requirement() {
    let root = copy("plan_check_open_missing");
    carrier(&root, "REQ-VAL-999");
    fails_naming(
        &root,
        "still-open item names REQ-VAL-999, which is not a requirement",
    );
}

/// RQ-173 has a ruling (R-332), so it is in `docs/archive/review_queue/M0.md`, not open.
const ARCHIVED_RQ: &str = "RQ-173";

#[test]
fn plan_check_fails_on_an_open_item_naming_an_rq_that_is_not_open() {
    let root = copy("plan_check_open_rq");
    carrier(&root, ARCHIVED_RQ);
    fails_naming(
        &root,
        &format!(
            "still-open item names {ARCHIVED_RQ}, which is not an open entry in REVIEW_QUEUE.md"
        ),
    );
}

negative_control!(
    plan_check_fails_on_an_open_item_naming_neither,
    "an item that names another live requirement passes, so the failure check must fail on it",
    expected = "plan-check passed",
    {
        let root = copy("plan_check_ctl_open_neither");
        carrier(&root, "REQ-VAL-040");
        fails_naming(&root, NEITHER);
    }
);

negative_control!(
    plan_check_fails_on_an_open_item_naming_a_retired_requirement,
    "an item that names a live requirement passes, so the failure check must fail on it",
    expected = "plan-check passed",
    {
        let root = copy("plan_check_ctl_open_retired");
        carrier(&root, "REQ-VAL-040");
        fails_naming(
            &root,
            &format!("still-open item names retired requirement {RETIRED}"),
        );
    }
);

negative_control!(
    plan_check_fails_on_an_open_item_naming_a_missing_requirement,
    "an item that names a retired requirement fails as \"retired\", not as missing",
    expected = "plan-check did not name the fault",
    {
        let root = copy("plan_check_ctl_open_missing");
        carrier(&root, RETIRED);
        fails_naming(&root, "which is not a requirement");
    }
);

negative_control!(
    plan_check_fails_on_an_open_item_naming_an_rq_that_is_not_open,
    "an item that names a live requirement in place of the RQ passes, so the failure check must fail on it",
    expected = "plan-check passed",
    {
        let root = copy("plan_check_ctl_open_rq");
        carrier(&root, "REQ-VAL-040");
        fails_naming(&root, "which is not an open entry in REVIEW_QUEUE.md");
    }
);
