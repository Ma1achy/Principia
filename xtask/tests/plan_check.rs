//! `cargo xtask plan-check` (REQ-SYS-007, REQ-SYS-008): the checker fails, naming the fault, on a copy of `plan/` + the
//! corpus with a task dropped from `tasks.yaml`, a requirement source into `docs/archive/`, or a task reference to an
//! `ARCHIVE_` brief (R-184: "cites archived file", not "doesn't exist"). It also fails on a still-open item of
//! `open-questions.md` that names neither a requirement nor an open REVIEW_QUEUE entry, or names a missing or retired
//! requirement, or an RQ that isn't open (R-334). It fails too on an RQ heading that a union merge of `REVIEW_QUEUE.md`
//! left twice, or left open though it is archived, and on what else that merge leaves in the queue: two entries with
//! one question, two versions of a field or a text, an entry with no body or short of a required field, a heading out
//! of form, or a fragment. `--queue` checks a queue at any path, so a would-be merge's (`.gitattributes`;
//! `plan/OPERATIONS.md` § "Merging" step 2). That it passes on this tree is CI's `python3 plan/check_plan.py` step on
//! every pull request, and `cargo xtask plan-check` in `cargo xtask ci` on each merge (R-909 retired the test of it
//! here).

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
    union_merge_from(name, BASE_RQ, ours, theirs)
}

/// `union_merge`, from the queue `base` on `main`.
fn union_merge_from(name: &str, base: &str, ours: &str, theirs: &str) -> (PathBuf, PathBuf) {
    let (queue, archive) = queue_file(name, base);
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

#[test]
fn union_merge_of_a_ruling_number_edited_on_two_sides_is_caught() {
    let (queue, archive) = union_merge(
        "union_ruling",
        &base_with_note("closed by R-100, in the words of that ruling"),
        &base_with_note("closed by R-101, in the words of that ruling"),
    );
    queue_fails_naming(&queue, &archive, "are two versions of one text");
}

negative_control!(
    union_merge_of_a_ruling_number_edited_on_two_sides_is_caught,
    "two lines differing in a plain number are a list's siblings, and pass, so the failure check must fail on them",
    expected = "queue check passed",
    {
        let (queue, archive) = union_merge(
            "union_ctl_ruling",
            &base_with_note("N = 6: the same long words of a list, one item each"),
            &base_with_note("N = 12: the same long words of a list, one item each"),
        );
        queue_fails_naming(&queue, &archive, "are two versions of one text");
    }
);

/// `BASE_RQ` whose last field wraps to a short tail, as the real queue's last entry did after PR #482.
fn base_with_short_tail() -> String {
    BASE_RQ.replace(
        "- **Applied meanwhile:** none.\n",
        "- **Applied meanwhile:** none, until the long note of the base runs on to the end of\n  it.\n",
    )
}

/// The union merge of one dated note added after `base_with_short_tail`'s short tail, closed by `ours` on one side and
/// by `theirs` on the other.
fn note_after_a_short_tail(name: &str, ours: &str, theirs: &str) -> (PathBuf, PathBuf) {
    let base = base_with_short_tail();
    let note =
        |by: &str| format!("{base}- **31 Oct 2026:** closed by {by}, in the very words of it.\n");
    union_merge_from(name, &base, &note(ours), &note(theirs))
}

/// The queue at `queue` passes the check.
fn queue_passes(queue: &Path, archive: &Path) {
    let (passed, out) = check_queue(queue, archive);
    assert!(passed, "queue check failed:\n{out}");
}

#[test]
fn union_merge_of_siblings_after_a_short_tail_passes() {
    // Two lines differing only in a plain number are a list's siblings (OPERATIONS § "Merging" step 2), also when the
    // line above them is a paragraph's short tail, which a run of two lines would else carry past the ratio.
    let (queue, archive) = note_after_a_short_tail(
        "union_short_tail",
        "R-700 at N = 6 (one of a list)",
        "R-700 at N = 12 (one of a list)",
    );
    queue_passes(&queue, &archive);
}

negative_control!(
    union_merge_of_siblings_after_a_short_tail_passes,
    "the same note differing in a ruling after the short tail is two versions, so the passes check must fail on it",
    expected = "are two versions of one text",
    {
        let (queue, archive) =
            note_after_a_short_tail("union_ctl_short_tail", "R-700", "R-701");
        queue_passes(&queue, &archive);
    }
);

// ---- The generated files: not committed; CI regenerates and checks them (the owner, 8 Nov 2026) ----------------------

/// Runs `python3 <script> <args>` in `root`: (passed, its output).
fn python(root: &Path, script: &str, args: &[&str]) -> (bool, String) {
    let out = Command::new("python3")
        .arg(script)
        .args(args)
        .current_dir(root)
        .timed_output()
        .expect("python3 ran");
    let text =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    (out.status.success(), text)
}

/// Writes `root`'s generated files and makes `root` a git repo; with `track`, `plan/coverage.md` is added to its index,
/// past the `.gitignore` the main tree has.
fn generated_in_git(root: &Path, track: bool) {
    let (passed, out) = python(root, "plan/tools/regenerate.py", &[]);
    assert!(passed, "regenerate.py failed:\n{out}");
    git(root, &["init", "-q"]);
    if track {
        git(root, &["add", "-f", "plan/coverage.md"]);
    }
}

const COMMITTED: &str = "plan/coverage.md: generated, so not committed";

#[test]
fn plan_check_fails_on_a_committed_generated_file() {
    let root = copy("plan_check_committed");
    generated_in_git(&root, true);
    fails_naming(&root, COMMITTED);
}

negative_control!(
    plan_check_fails_on_a_committed_generated_file,
    "the same files written but left out of git pass, so the failure check must fail on them",
    expected = "plan-check passed",
    {
        let root = copy("plan_check_ctl_committed");
        generated_in_git(&root, false);
        fails_naming(&root, COMMITTED);
    }
);

/// Appends `text` to `root`'s perf checklist.
fn append_to_perf(root: &Path, text: &str) {
    let path = root.join("plan/reviewers/perf.md");
    let doc = fs::read_to_string(&path).expect("checklist read");
    fs::write(&path, format!("{doc}\n{text}")).expect("checklist written");
}

const LIST_BLOCK: &str = "plan/reviewers/perf.md holds a `<!-- list:… -->` block";

#[test]
fn plan_check_fails_on_a_list_block_committed_in_a_checklist() {
    let root = copy("plan_check_list_block");
    append_to_perf(
        &root,
        "<!-- list:benchmarks -->\n*48 requirements*\n<!-- /list:benchmarks -->\n",
    );
    fails_naming(&root, LIST_BLOCK);
}

negative_control!(
    plan_check_fails_on_a_list_block_committed_in_a_checklist,
    "a comment that is no list marker passes, so the failure check must fail on it",
    expected = "plan-check passed",
    {
        let root = copy("plan_check_ctl_list_block");
        append_to_perf(&root, "<!-- a note on the benchmarks -->\n");
        fails_naming(&root, LIST_BLOCK);
    }
);

const UNNAMED_LIST: &str =
    "plan/reviewers/perf.md doesn't name its list `plan/reviewers/lists/benchmarks.md`";

#[test]
fn plan_check_fails_on_a_checklist_not_naming_its_list() {
    let root = copy("plan_check_unnamed_list");
    edit(
        &root,
        "plan/reviewers/perf.md",
        "## 9. Benchmarks",
        "`plan/reviewers/lists/benchmarks.md`",
        "the benchmarks list",
    );
    fails_naming(&root, UNNAMED_LIST);
}

negative_control!(
    plan_check_fails_on_a_checklist_not_naming_its_list,
    "the pointer reworded around the path still names it, and passes, so the failure check must fail on it",
    expected = "plan-check passed",
    {
        let root = copy("plan_check_ctl_unnamed_list");
        edit(
            &root,
            "plan/reviewers/perf.md",
            "## 9. Benchmarks",
            "are listed in",
            "are in",
        );
        fails_naming(&root, UNNAMED_LIST);
    }
);

const NO_GROUP: &str = "decisions.md's entry R-99999 is in no group";

/// Appends a ruling, R-99999, to `root`'s `decisions.md`.
fn add_ruling(root: &Path) {
    let path = root.join("decisions.md");
    let text = fs::read_to_string(&path).expect("decisions.md read");
    fs::write(
        &path,
        format!("{text}\n## R-99999 — A ruling in no group\n\nText.\n"),
    )
    .expect("decisions.md written");
}

#[test]
fn plan_check_fails_on_rules_that_cannot_be_generated() {
    let root = copy("plan_check_no_group");
    add_ruling(&root);
    fails_naming(&root, NO_GROUP);
}

negative_control!(
    plan_check_fails_on_rules_that_cannot_be_generated,
    "a copy with no ruling added generates, and passes, so the failure check must fail on it",
    expected = "plan-check passed",
    fails_naming(&copy("plan_check_ctl_no_group"), NO_GROUP)
);

const STALE_GATE: &str = "plan/MILESTONES.md is out of date";

/// In `root`'s `plan/MILESTONES.md`, drops one requirement from M0's committed gate block.
fn stale_gate(root: &Path) {
    edit(
        root,
        "plan/MILESTONES.md",
        "<!-- gate:M0 -->",
        "REQ-SYS-063…079",
        "REQ-SYS-063…078",
    );
}

#[test]
fn plan_check_fails_on_a_gate_block_unlike_a_fresh_regeneration() {
    let root = copy("plan_check_stale_gate");
    stale_gate(&root);
    fails_naming(&root, STALE_GATE);
}

negative_control!(
    plan_check_fails_on_a_gate_block_unlike_a_fresh_regeneration,
    "an edit to the hand-written prose, outside every block, passes, so the failure check must fail on it",
    expected = "plan-check passed",
    {
        let root = copy("plan_check_ctl_stale_gate");
        edit(
            &root,
            "plan/MILESTONES.md",
            "# Milestones",
            "in dependency order",
            "in the order of their dependencies",
        );
        fails_naming(&root, STALE_GATE);
    }
);

/// Puts a merge conflict, as git writes one, after `anchor`'s line in `root`'s `plan/MILESTONES.md`.
fn conflict_after(root: &Path, anchor: &str) {
    edit(
        root,
        "plan/MILESTONES.md",
        anchor,
        &format!("{anchor}\n"),
        &format!("{anchor}\n<<<<<<< HEAD\n- one side\n=======\n- the other side\n>>>>>>> theirs\n"),
    );
}

/// `regenerate.py --all` resolves `root`'s conflicted `plan/MILESTONES.md` to the tree's own.
fn resolves(root: &Path) {
    let (passed, out) = python(root, "plan/tools/regenerate.py", &["--all"]);
    assert!(passed, "regenerate.py --all failed:\n{out}");
    let (now, tree) = (
        fs::read_to_string(root.join("plan/MILESTONES.md")).expect("resolved file read"),
        fs::read_to_string(repo_root().join("plan/MILESTONES.md")).expect("tree's file read"),
    );
    assert!(now == tree, "the resolved MILESTONES.md is not the tree's");
}

#[test]
fn regenerate_all_resolves_a_conflict_inside_a_gate_block() {
    let root = copy("regenerate_conflict_in_block");
    conflict_after(&root, "<!-- gate:M0 -->");
    fails_naming(&root, "a merge conflict inside a generated block");
    resolves(&root);
    let (passed, out) = check(&root);
    assert!(passed, "plan-check failed on the resolved copy:\n{out}");
}

negative_control!(
    regenerate_all_resolves_a_conflict_inside_a_gate_block,
    "a conflict in the hand-written prose is no generated block's, so regenerating must refuse it",
    expected = "regenerate.py --all failed",
    {
        let root = copy("regenerate_ctl_conflict_in_block");
        conflict_after(&root, "# Milestones — M0 to M9");
        resolves(&root);
    }
);

const OUTSIDE: &str = "a merge conflict outside the generated blocks";

/// `regenerate.py --all` refuses `root`, naming a conflict in the hand-written text.
fn refuses_outside(root: &Path) {
    let (passed, out) = python(root, "plan/tools/regenerate.py", &["--all"]);
    assert!(!passed, "regenerate.py --all passed:\n{out}");
    assert!(
        out.contains(OUTSIDE),
        "regenerate.py --all did not name the fault:\n{out}"
    );
}

#[test]
fn regenerate_all_refuses_a_conflict_in_the_hand_written_text() {
    let root = copy("regenerate_conflict_outside");
    conflict_after(&root, "# Milestones — M0 to M9");
    refuses_outside(&root);
}

negative_control!(
    regenerate_all_refuses_a_conflict_in_the_hand_written_text,
    "a conflict inside a gate block is the generator's to resolve, so the refusal check must fail on it",
    expected = "regenerate.py --all passed",
    {
        let root = copy("regenerate_ctl_conflict_outside");
        conflict_after(&root, "<!-- gate:M0 -->");
        refuses_outside(&root);
    }
);

/// Runs `regenerate.py` with `args` on a copy whose M0 gate block is stale; returns the copy.
fn regenerate_on_a_stale_gate(name: &str, args: &[&str]) -> PathBuf {
    let root = copy(name);
    for file in [
        "plan/CURRENT_RULES.md",
        "plan/coverage.md",
        "plan/reviewers/lists",
    ] {
        let path = root.join(file);
        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir_all(&path);
    }
    stale_gate(&root);
    let (passed, out) = python(&root, "plan/tools/regenerate.py", args);
    assert!(passed, "regenerate.py failed:\n{out}");
    root
}

/// `root`'s uncommitted files are written, and its committed MILESTONES.md is as it was.
fn wrote_only_the_uncommitted(root: &Path) {
    for file in [
        "plan/CURRENT_RULES.md",
        "plan/coverage.md",
        "plan/reviewers/lists/numerical-gates.md",
        "plan/reviewers/lists/m9-targets.md",
        "plan/reviewers/lists/benchmarks.md",
        "plan/reviewers/lists/gui-screenshots.md",
    ] {
        assert!(
            root.join(file).is_file(),
            "regenerate.py did not write {file}"
        );
    }
    let text = fs::read_to_string(root.join("plan/MILESTONES.md")).expect("MILESTONES.md read");
    assert!(
        text.contains("REQ-SYS-063…078"),
        "regenerate.py rewrote the committed MILESTONES.md"
    );
}

#[test]
fn regenerate_writes_the_uncommitted_files_and_leaves_committed_ones() {
    wrote_only_the_uncommitted(&regenerate_on_a_stale_gate("regenerate_default", &[]));
}

negative_control!(
    regenerate_writes_the_uncommitted_files_and_leaves_committed_ones,
    "--all rewrites the committed gate blocks too, so the leaves-them check must fail on it",
    expected = "regenerate.py rewrote the committed MILESTONES.md",
    wrote_only_the_uncommitted(&regenerate_on_a_stale_gate(
        "regenerate_ctl_default",
        &["--all"]
    ))
);

/// Every file under `dir`, relative to it, with its length and modified time.
fn snapshot(dir: &Path) -> Vec<(PathBuf, u64, std::time::SystemTime)> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(at) = stack.pop() {
        for entry in fs::read_dir(&at).expect("dir read") {
            let path = entry.expect("entry read").path();
            let meta = fs::metadata(&path).expect("metadata read");
            if meta.is_dir() {
                stack.push(path);
            } else {
                let rel = path.strip_prefix(dir).expect("under dir").to_path_buf();
                out.push((rel, meta.len(), meta.modified().expect("mtime read")));
            }
        }
    }
    out.sort();
    out
}

/// A copy with no generated file in it, and an empty directory beside it, outside it: (copy, directory).
fn copy_and_out(name: &str) -> (PathBuf, PathBuf) {
    let root = copy(name);
    for file in [
        "plan/CURRENT_RULES.md",
        "plan/coverage.md",
        "plan/reviewers/lists",
    ] {
        let path = root.join(file);
        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir_all(&path);
    }
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}_out"));
    let _ = fs::remove_dir_all(&out);
    (root, out)
}

/// `regenerate.py` with `args` writes every generated file under `out` and leaves `root`'s tree as it was.
fn writes_only_under(root: &Path, out: &Path, args: &[&str]) {
    let before = snapshot(root);
    let (passed, text) = python(root, "plan/tools/regenerate.py", args);
    assert!(passed, "regenerate.py failed:\n{text}");
    let after = snapshot(root);
    let added: Vec<_> = after.iter().filter(|f| !before.contains(f)).collect();
    assert!(
        added.is_empty(),
        "regenerate.py wrote into the tree: {added:?}"
    );
    for file in [
        "plan/CURRENT_RULES.md",
        "plan/coverage.md",
        "plan/reviewers/lists/numerical-gates.md",
        "plan/reviewers/lists/m9-targets.md",
        "plan/reviewers/lists/benchmarks.md",
        "plan/reviewers/lists/gui-screenshots.md",
    ] {
        assert!(
            out.join(file).is_file(),
            "regenerate.py did not write {file} under the --out directory"
        );
    }
}

#[test]
fn regenerate_out_writes_under_its_directory_and_nothing_into_the_tree() {
    let (root, out) = copy_and_out("regenerate_out");
    let dir = out.to_str().expect("utf-8 path");
    writes_only_under(&root, &out, &["--out", dir]);
}

negative_control!(
    regenerate_out_writes_under_its_directory_and_nothing_into_the_tree,
    "with no --out the files are written into the tree, so the nothing-into-the-tree check must fail on it",
    expected = "regenerate.py wrote into the tree",
    {
        let (root, out) = copy_and_out("regenerate_ctl_out");
        writes_only_under(&root, &out, &[]);
    }
);

const OUT_INSIDE: &str = "is inside the repo tree";

/// `regenerate.py --out <dir>` refuses `dir`, naming it inside the tree, and writes nothing there.
fn refuses_out(root: &Path, dir: &Path) {
    refuses_out_by(root, "plan/tools/regenerate.py", dir);
}

/// `<script> --out <dir>` refuses `dir`, naming it inside the tree, and writes nothing there. Run with `-B`, as
/// `regenerate.py` runs the generators, so Python's bytecode cache for their imports isn't counted as a write.
fn refuses_out_by(root: &Path, script: &str, dir: &Path) {
    refuses_out_naming(root, script, dir, OUT_INSIDE);
}

/// `<script> --out <dir>` fails, naming `fault`, and writes nothing into the tree `root`.
fn refuses_out_naming(root: &Path, script: &str, dir: &Path, fault: &str) {
    let before = snapshot(root);
    let (passed, out) = python(
        root,
        "-B",
        &[script, "--out", dir.to_str().expect("utf-8 path")],
    );
    assert!(!passed, "{script} --out passed:\n{out}");
    assert!(
        out.contains(fault),
        "{script} --out did not name the fault:\n{out}"
    );
    assert!(
        snapshot(root) == before,
        "{script} --out wrote into the tree"
    );
}

#[test]
fn regenerate_out_refuses_the_tree_and_a_directory_in_it() {
    let (root, _) = copy_and_out("regenerate_out_inside");
    refuses_out(&root, &root);
    refuses_out(&root, &root.join("plan/generated"));
}

negative_control!(
    regenerate_out_refuses_the_tree_and_a_directory_in_it,
    "a sibling whose name starts with the tree's is outside it, and passes, so the refusal check must fail on it",
    expected = "regenerate.py --out passed",
    {
        let (root, _) = copy_and_out("regenerate_ctl_out_inside");
        let sibling = PathBuf::from(format!("{}-sibling", root.display()));
        let _ = fs::remove_dir_all(&sibling);
        refuses_out(&root, &sibling);
    }
);

/// Each script that takes `--out`: `regenerate.py` and the generators it runs, which share its guard.
const OUT_SCRIPTS: [&str; 4] = [
    "plan/tools/regenerate.py",
    "plan/tools/current_rules.py",
    "plan/tools/coverage.py",
    "plan/tools/reviewer_lists.py",
];

/// A symlink named after `name` beside the copy `root`, outside it, to `to`.
#[cfg(unix)]
fn link_beside(root: &Path, name: &str, to: &Path) -> PathBuf {
    let link = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}_link"));
    let _ = fs::remove_file(&link);
    std::os::unix::fs::symlink(to, &link).expect("symlink made");
    assert!(!link.starts_with(root), "the link is outside the copy");
    link
}

#[cfg(unix)]
#[test]
fn regenerate_out_refuses_a_symlink_to_a_directory_in_the_tree() {
    let (root, _) = copy_and_out("regenerate_out_symlink");
    let link = link_beside(&root, "regenerate_out_symlink", &root.join("plan"));
    for script in OUT_SCRIPTS {
        refuses_out_by(&root, script, &link.join("z"));
    }
    assert!(!root.join("plan/z").exists(), "a generator wrote plan/z");
}

#[cfg(unix)]
negative_control!(
    regenerate_out_refuses_a_symlink_to_a_directory_in_the_tree,
    "a symlink to a directory outside the tree is outside it, and passes, so the refusal check must fail on it",
    expected = "--out passed",
    {
        let (root, out) = copy_and_out("regenerate_ctl_out_symlink");
        fs::create_dir_all(&out).expect("out made");
        let link = link_beside(&root, "regenerate_ctl_out_symlink", &out);
        refuses_out_by(&root, "plan/tools/regenerate.py", &link.join("z"));
    }
);

/// An empty `--out` directory for `root`, outside it, holding a symlink `plan` to `to` (code review C3).
#[cfg(unix)]
fn out_with_plan_link(root: &Path, name: &str, to: &Path) -> PathBuf {
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}_out"));
    let _ = fs::remove_dir_all(&out);
    fs::create_dir_all(&out).expect("out made");
    std::os::unix::fs::symlink(to, out.join("plan")).expect("symlink made");
    assert!(
        !out.starts_with(root),
        "the --out directory is outside the copy"
    );
    out
}

#[cfg(unix)]
#[test]
fn regenerate_out_refuses_a_symlink_under_it_into_the_tree() {
    let (root, _) = copy_and_out("regenerate_out_link_under");
    let out = out_with_plan_link(&root, "regenerate_out_link_under", &root.join("plan"));
    for script in OUT_SCRIPTS {
        refuses_out_by(&root, script, &out);
    }
}

#[cfg(unix)]
negative_control!(
    regenerate_out_refuses_a_symlink_under_it_into_the_tree,
    "a symlink under --out to a directory outside the tree carries the writes outside, and passes, so the refusal \
     check must fail on it",
    expected = "--out passed",
    {
        let (root, elsewhere) = copy_and_out("regenerate_ctl_out_link_under");
        fs::create_dir_all(&elsewhere).expect("elsewhere made");
        let out = out_with_plan_link(&root, "regenerate_ctl_out_link_under", &elsewhere);
        refuses_out_by(&root, "plan/tools/regenerate.py", &out);
    }
);

/// Every file the generators write, relative to the repo root.
const GENERATED_FILES: [&str; 6] = [
    "plan/CURRENT_RULES.md",
    "plan/coverage.md",
    "plan/reviewers/lists/numerical-gates.md",
    "plan/reviewers/lists/m9-targets.md",
    "plan/reviewers/lists/benchmarks.md",
    "plan/reviewers/lists/gui-screenshots.md",
];

/// Puts at each generated file's place under `out` a symlink to `to(file)`.
#[cfg(unix)]
fn link_each_target(out: &Path, to: impl Fn(&str) -> PathBuf) {
    let _ = fs::remove_dir_all(out);
    for file in GENERATED_FILES {
        let at = out.join(file);
        fs::create_dir_all(at.parent().expect("has a parent")).expect("dir made");
        std::os::unix::fs::symlink(to(file), &at).expect("symlink made");
    }
}

#[cfg(unix)]
#[test]
fn regenerate_out_refuses_a_target_that_is_a_symlink_into_the_tree() {
    let (root, out) = copy_and_out("regenerate_out_target_link");
    // CURRENT_RULES.md's link dangles at a file a write would create in the tree; the others name a source file.
    link_each_target(&out, |file| match file {
        "plan/CURRENT_RULES.md" => root.join(file),
        _ => root.join("plan/requirements.yaml"),
    });
    for script in OUT_SCRIPTS {
        refuses_out_by(&root, script, &out);
    }
}

#[cfg(unix)]
negative_control!(
    regenerate_out_refuses_a_target_that_is_a_symlink_into_the_tree,
    "a target linked to a file outside the tree is written there, and passes, so the refusal check must fail on it",
    expected = "--out passed",
    {
        let (root, out) = copy_and_out("regenerate_ctl_out_target_link");
        let elsewhere = Path::new(env!("CARGO_TARGET_TMPDIR")).join("regenerate_ctl_out_target_link_elsewhere");
        let _ = fs::remove_dir_all(&elsewhere);
        fs::create_dir_all(&elsewhere).expect("elsewhere made");
        link_each_target(&out, |file| elsewhere.join(file.replace('/', "_")));
        refuses_out_by(&root, "plan/tools/regenerate.py", &out);
    }
);

const HARD_LINK: &str = "has another hard link";

/// `out` after a clean `regenerate.py --out` run, and the run over it again, which passes: a file with one link is
/// the last run's, and is rewritten.
fn regenerated_twice(root: &Path, out: &Path) {
    let dir = out.to_str().expect("utf-8 path");
    for _ in 0..2 {
        let (passed, text) = python(root, "plan/tools/regenerate.py", &["--out", dir]);
        assert!(passed, "regenerate.py --out failed:\n{text}");
    }
}

#[test]
fn regenerate_out_refuses_a_target_with_a_second_hard_link() {
    let (root, out) = copy_and_out("regenerate_out_hard_link");
    regenerated_twice(&root, &out);
    for file in GENERATED_FILES {
        fs::remove_file(out.join(file)).expect("file removed");
        fs::hard_link(root.join("plan/requirements.yaml"), out.join(file)).expect("hard link made");
    }
    for script in OUT_SCRIPTS {
        refuses_out_naming(&root, script, &out, HARD_LINK);
    }
}

negative_control!(
    regenerate_out_refuses_a_target_with_a_second_hard_link,
    "a target the last run wrote has one link, and passes, so the refusal check must fail on it",
    expected = "--out passed",
    {
        let (root, out) = copy_and_out("regenerate_ctl_out_hard_link");
        regenerated_twice(&root, &out);
        refuses_out_naming(&root, "plan/tools/regenerate.py", &out, HARD_LINK);
    }
);

/// A line of `regenerate.py`'s usage, which it prints on arguments it doesn't take.
const USAGE: &str = "regenerate.py --out <dir>  write them under <dir>";

/// `regenerate.py` with `args` fails and prints `fault`.
fn refuses_args(root: &Path, args: &[&str], fault: &str) {
    let (passed, out) = python(root, "plan/tools/regenerate.py", args);
    assert!(!passed, "regenerate.py {args:?} passed:\n{out}");
    assert!(
        out.contains(fault),
        "regenerate.py {args:?} did not name the fault:\n{out}"
    );
}

#[test]
fn regenerate_out_needs_a_directory_and_goes_alone() {
    let (root, out) = copy_and_out("regenerate_out_args");
    let dir = out.to_str().expect("utf-8 path");
    refuses_args(&root, &["--out"], "--out needs a directory");
    refuses_args(&root, &["--out", "--all"], "--out needs a directory");
    refuses_args(&root, &["--out", dir, "--all"], USAGE);
    refuses_args(&root, &["--check", "--out", dir], USAGE);
}

negative_control!(
    regenerate_out_needs_a_directory_and_goes_alone,
    "--out with its directory and nothing else passes, so the refusal check must fail on it",
    expected = "passed",
    {
        let (root, out) = copy_and_out("regenerate_ctl_out_args");
        refuses_args(&root, &["--out", out.to_str().expect("utf-8 path")], USAGE);
    }
);
