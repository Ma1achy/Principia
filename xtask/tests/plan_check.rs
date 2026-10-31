//! `cargo xtask plan-check` (REQ-SYS-007, REQ-SYS-008): the checker passes on this tree, and fails, naming the fault,
//! on a copy of `plan/` + the corpus with a task dropped from `tasks.yaml`, a requirement source into `docs/archive/`,
//! or a task reference to an `ARCHIVE_` brief (R-184: "cites archived file", not "doesn't exist"). It also fails on a
//! still-open item of `open-questions.md` that names neither a requirement nor an open REVIEW_QUEUE entry, or names a
//! missing or retired requirement, or an RQ that isn't open (R-334). It fails too on an RQ heading that a union merge
//! of `REVIEW_QUEUE.md` left twice, or left open though it is archived, and on what else that merge leaves in the
//! queue: two entries with one question, two versions of a field or a text, an entry with no body or short of a required
//! field, a heading out of form, or a fragment. `--queue` checks a queue at any path, so a would-be merge's
//! (`.gitattributes`; `plan/OPERATIONS.md` § "Merging" step 2).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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

/// A well-formed entry, with its own text so no two are alike.
fn entry(id: &str, question: &str) -> String {
    format!(
        "## {id}: {question}\n\n- **File, section:** `{id}.md` § \"{question}\".\n- **Applied meanwhile:** none, for {id}.\n"
    )
}

/// Appends `heading` to `root`'s `REVIEW_QUEUE.md`, as a union merge would.
fn append_to_queue(root: &Path, heading: &str) {
    let path = root.join("REVIEW_QUEUE.md");
    let text = fs::read_to_string(&path).expect("queue read");
    fs::write(&path, format!("{text}{heading}\n")).expect("queue written");
}

#[test]
fn plan_check_fails_on_an_rq_heading_a_union_merge_left_twice() {
    let root = copy("plan_check_rq_twice");
    append_to_queue(&root, "## RQ-999999: one side's heading");
    append_to_queue(&root, "## RQ-999999: the other side's heading");
    fails_naming(&root, "RQ-999999 is two entries in REVIEW_QUEUE.md");
}

#[test]
fn plan_check_fails_on_an_archived_rq_a_union_merge_left_open() {
    let root = copy("plan_check_rq_reopened");
    append_to_queue(
        &root,
        &format!("## {ARCHIVED_RQ}: edited on a branch, archived on the other"),
    );
    fails_naming(
        &root,
        &format!(
            "{ARCHIVED_RQ} is an entry in both REVIEW_QUEUE.md and docs/archive/review_queue/"
        ),
    );
}

negative_control!(
    plan_check_fails_on_an_rq_heading_a_union_merge_left_twice,
    "a heading with an id of its own passes, so the failure check must fail on it",
    expected = "plan-check passed",
    {
        let root = copy("plan_check_ctl_rq_twice");
        append_to_queue(&root, &entry("RQ-999999", "A new entry"));
        fails_naming(&root, "is two entries in REVIEW_QUEUE.md");
    }
);

negative_control!(
    plan_check_fails_on_an_archived_rq_a_union_merge_left_open,
    "a heading with an id of its own passes, so the failure check must fail on it",
    expected = "plan-check passed",
    {
        let root = copy("plan_check_ctl_rq_reopened");
        append_to_queue(&root, &entry("RQ-999999", "A new entry"));
        fails_naming(&root, "is an entry in both REVIEW_QUEUE.md and");
    }
);

const SAME_QUESTION: &str = "the same question as the entry at line";

#[test]
fn plan_check_runs_the_queue_form_check() {
    let root = copy("plan_check_rq_question");
    append_to_queue(&root, &entry("RQ-999998", "One question"));
    append_to_queue(&root, &entry("RQ-999999", "One question"));
    fails_naming(&root, SAME_QUESTION);
}

negative_control!(
    plan_check_runs_the_queue_form_check,
    "two entries with questions of their own pass, so the failure check must fail on them",
    expected = "plan-check passed",
    {
        let root = copy("plan_check_ctl_rq_question");
        append_to_queue(&root, &entry("RQ-999998", "One question"));
        append_to_queue(&root, &entry("RQ-999999", "Another question"));
        fails_naming(&root, SAME_QUESTION);
    }
);

/// A fresh directory under the test target.
fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("dir created");
    dir
}

/// Runs this tree's checker on the queue at `queue`, with ids against `archive`: (passed, its output).
fn check_queue(queue: &Path, archive: &Path) -> (bool, String) {
    let mut checker = command(&repo_root());
    checker
        .arg("--queue")
        .arg(queue)
        .arg("--archive")
        .arg(archive);
    let out = checker.timed_output().expect("python3 ran the queue check");
    let text =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    (out.status.success(), text)
}

/// The queue at `queue` fails, naming `fault`.
fn queue_fails_naming(queue: &Path, archive: &Path, fault: &str) {
    let (passed, out) = check_queue(queue, archive);
    assert!(!passed, "queue check passed:\n{out}");
    assert!(
        out.contains(fault),
        "queue check did not name the fault {fault:?}:\n{out}"
    );
}

/// Writes `text` as a queue in a fresh directory: (the queue's path, an empty archive beside it).
fn queue_file(name: &str, text: &str) -> (PathBuf, PathBuf) {
    let dir = scratch(name);
    let (queue, archive) = (dir.join("REVIEW_QUEUE.md"), dir.join("archive"));
    fs::write(&queue, text).expect("queue written");
    fs::create_dir_all(&archive).expect("archive made");
    (queue, archive)
}

#[test]
fn queue_check_names_an_id_collision_at_any_path() {
    let text = entry("RQ-7", "One question") + "\n" + &entry("RQ-7", "Another question");
    let (queue, archive) = queue_file("queue_collision", &text);
    queue_fails_naming(&queue, &archive, "RQ-7 is two entries in");
    queue_fails_naming(&queue, &archive, "(lines 1 and 6)");
}

negative_control!(
    queue_check_names_an_id_collision_at_any_path,
    "two ids of their own pass at any path, so the failure check must fail on them",
    expected = "queue check passed",
    {
        let text = entry("RQ-7", "One question") + "\n" + &entry("RQ-8", "Another question");
        let (queue, archive) = queue_file("queue_ctl_collision", &text);
        queue_fails_naming(&queue, &archive, "is two entries in");
    }
);

#[test]
fn queue_check_reads_the_archive_it_is_given() {
    let (queue, archive) = queue_file("queue_archive", &entry("RQ-7", "One question"));
    fs::write(archive.join("M0.md"), entry("RQ-7", "One question")).expect("archive written");
    queue_fails_naming(&queue, &archive, "RQ-7 is an entry in both");
}

negative_control!(
    queue_check_reads_the_archive_it_is_given,
    "an archive that holds other ids passes, so the failure check must fail on it",
    expected = "queue check passed",
    {
        let (queue, archive) = queue_file("queue_ctl_archive", &entry("RQ-7", "One question"));
        fs::write(archive.join("M0.md"), entry("RQ-8", "Another question"))
            .expect("archive written");
        queue_fails_naming(&queue, &archive, "is an entry in both");
    }
);

#[test]
fn queue_check_fails_on_a_missing_queue_or_archive() {
    let (queue, archive) = queue_file("queue_missing", &entry("RQ-7", "One question"));
    let gone = archive.join("gone");
    queue_fails_naming(&queue, &gone, "no such archive directory");
    queue_fails_naming(&gone, &archive, "gone: missing");
}

negative_control!(
    queue_check_fails_on_a_missing_queue_or_archive,
    "a queue and an archive that exist pass, so the failure check must fail on them",
    expected = "queue check passed",
    {
        let (queue, archive) = queue_file("queue_ctl_missing", &entry("RQ-7", "One question"));
        queue_fails_naming(&queue, &archive, "no such archive directory");
    }
);

/// `git` in `dir`, which must succeed; identity and signing are the test's own, whatever the machine's.
fn git(dir: &Path, args: &[&str]) {
    let mut cmd = Command::new("git");
    cmd.args(["-c", "user.name=t", "-c", "user.email=t@example.invalid"])
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(dir);
    let out = cmd.timed_output().expect("git ran");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// The base queue of a union-merge repo: one entry, with a field the sides can edit.
const BASE_RQ: &str = "## RQ-1: The base question\n\n- **File, section:** `base.md` § \"One\".\n- **Options seen:** (a) the one that the base has.\n- **Applied meanwhile:** none.\n";

/// Commits `text` as the queue on a new branch `name`, from `main`, in the repo `dir`.
fn side(dir: &Path, name: &str, text: &str) {
    git(dir, &["checkout", "-q", "-b", name, "main"]);
    fs::write(dir.join("REVIEW_QUEUE.md"), text).expect("queue written");
    git(dir, &["commit", "-qam", name, "--allow-empty"]);
}

/// A repo whose `REVIEW_QUEUE.md` merges by union: branches `a` and `b` hold `ours` and `theirs`, each passing the
/// queue check alone. Merges them and returns the merged queue and an empty archive; the merge itself must be clean.
fn union_merge(name: &str, ours: &str, theirs: &str) -> (PathBuf, PathBuf) {
    let (queue, archive) = queue_file(name, BASE_RQ);
    let dir = queue.parent().expect("queue has a directory");
    fs::write(dir.join(".gitattributes"), "REVIEW_QUEUE.md merge=union\n").expect("attr written");
    git(dir, &["init", "-q", "-b", "main"]);
    git(dir, &["add", ".gitattributes", "REVIEW_QUEUE.md"]);
    git(dir, &["commit", "-qm", "base"]);
    side(dir, "a", ours);
    let (a_passed, a_out) = check_queue(&queue, &archive);
    assert!(a_passed, "branch a fails alone:\n{a_out}");
    side(dir, "b", theirs);
    let (b_passed, b_out) = check_queue(&queue, &archive);
    assert!(b_passed, "branch b fails alone:\n{b_out}");
    git(dir, &["merge", "-q", "--no-edit", "a"]);
    (queue, archive)
}

/// `BASE_RQ` with `added` after it.
fn base_and(added: &str) -> String {
    format!("{BASE_RQ}\n{added}")
}

#[test]
fn union_merge_of_one_id_with_two_headings_is_caught() {
    let (queue, archive) = union_merge(
        "union_ids",
        &base_and(&entry("RQ-2", "A question from one side")),
        &base_and(&entry("RQ-2", "A question from the other side")),
    );
    queue_fails_naming(&queue, &archive, "RQ-2 is two entries in");
}

negative_control!(
    union_merge_of_one_id_with_two_headings_is_caught,
    "two branches taking different ids merge and pass, so the failure check must fail on them",
    expected = "queue check passed",
    {
        let (queue, archive) = union_merge(
            "union_ctl_ids",
            &base_and(&entry("RQ-2", "A question from one side")),
            &base_and(&entry("RQ-3", "A question from the other side")),
        );
        queue_fails_naming(&queue, &archive, "is two entries in");
    }
);

#[test]
fn union_merge_of_one_heading_with_two_bodies_is_caught() {
    let head = "## RQ-2: One question\n\n";
    let (queue, archive) = union_merge(
        "union_bodies",
        &base_and(&format!(
            "{head}- **File, section:** `this.md` § \"Here\".\n- **Applied meanwhile:** the first answer.\n"
        )),
        &base_and(&format!(
            "{head}- **File, section:** `that.md` § \"Elsewhere\".\n- **Applied meanwhile:** another entirely.\n"
        )),
    );
    queue_fails_naming(&queue, &archive, "two \"File, section\" fields");
}

negative_control!(
    union_merge_of_one_heading_with_two_bodies_is_caught,
    "a heading with one body twice merges to one body and passes, so the failure check must fail on it",
    expected = "queue check passed",
    {
        let both = base_and(&entry("RQ-2", "One question"));
        let (queue, archive) = union_merge("union_ctl_bodies", &both, &both);
        queue_fails_naming(&queue, &archive, "fields");
    }
);

#[test]
fn union_merge_of_a_field_edited_on_two_sides_is_caught() {
    let (queue, archive) = union_merge(
        "union_field",
        &BASE_RQ.replace("- **Options seen:** (a)", "- **Options seen:** (x)"),
        &BASE_RQ.replace("- **Options seen:** (a)", "- **Options seen:** (y)"),
    );
    queue_fails_naming(&queue, &archive, "two \"Options seen\" fields");
}

negative_control!(
    union_merge_of_a_field_edited_on_two_sides_is_caught,
    "an edit on one side leaves one field and passes, so the failure check must fail on it",
    expected = "queue check passed",
    {
        let (queue, archive) = union_merge(
            "union_ctl_field",
            &BASE_RQ.replace("- **Options seen:** (a)", "- **Options seen:** (x)"),
            BASE_RQ,
        );
        queue_fails_naming(&queue, &archive, "fields");
    }
);

/// A note line in `BASE_RQ` the sides can edit, not a one-per-entry field.
fn base_with_note(note: &str) -> String {
    BASE_RQ.replace(
        "- **Options seen:**",
        &format!("  {note}\n- **Options seen:**"),
    )
}

#[test]
fn union_merge_of_a_line_edited_on_two_sides_is_caught() {
    let (queue, archive) = union_merge(
        "union_line",
        &base_with_note("a wrapped line of the quotation, on this side"),
        &base_with_note("a wrapped line of the quotation, on that side"),
    );
    queue_fails_naming(&queue, &archive, "are two versions of one text");
}

negative_control!(
    union_merge_of_a_line_edited_on_two_sides_is_caught,
    "two new lines in different words merge and pass, so the failure check must fail on them",
    expected = "queue check passed",
    {
        let (queue, archive) = union_merge(
            "union_ctl_line",
            &base_with_note("a wrapped line of the quotation, on this side"),
            &base_with_note("nothing in common with what the other adds here"),
        );
        queue_fails_naming(&queue, &archive, "are two versions of one text");
    }
);

/// Two new entries sharing a tail of more than three lines: a union merge moves the tail to the second alone.
fn shared_tail(id: &str, unique: &str, tail: &str) -> String {
    base_and(&format!(
        "## {id}: A question of {id}\n\n- **File, section:** `{id}.md` § \"{unique}\".\n{tail}"
    ))
}

const SHARED_TAIL: &str = "- **The question:** one the two new entries both ask.\n- **Options seen:** the same for each.\n- **Applied meanwhile:** none yet, for either.\n- **Waits:** the same task, for both.\n";

#[test]
fn union_merge_interleaving_two_entries_is_caught() {
    let (queue, archive) = union_merge(
        "union_tail",
        &shared_tail("RQ-2", "This", SHARED_TAIL),
        &shared_tail("RQ-3", "That", SHARED_TAIL),
    );
    queue_fails_naming(&queue, &archive, "no \"- **Applied meanwhile:**\" field");
}

negative_control!(
    union_merge_interleaving_two_entries_is_caught,
    "two new entries with tails of their own stay whole and pass, so the failure check must fail on them",
    expected = "queue check passed",
    {
        let (queue, archive) = union_merge(
            "union_ctl_tail",
            &shared_tail("RQ-2", "This", "- **Applied meanwhile:** none for the first.\n"),
            &shared_tail("RQ-3", "That", "- **Applied meanwhile:** the other answer.\n"),
        );
        queue_fails_naming(&queue, &archive, "no \"- **Applied meanwhile:**\" field");
    }
);
