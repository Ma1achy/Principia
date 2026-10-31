//! QA for tooling/rq-union, from the owner's words on #359: a check that fails on duplicate RQ ids or duplicated or
//! conflicting entries in `REVIEW_QUEUE.md`, tested by real union merges of two branches (`plan/OPERATIONS.md`
//! § "Merging" step 2). Each merge here starts from this tree's real queue and archive, not a toy queue, and each side
//! passes the check alone.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use validation::negative_control;
use validation::spawn::Spawn;
use xtask::plan_check::{command, repo_root};

/// Runs this tree's `--queue` form on `dir`'s queue and archive: (passed, its output).
fn check(dir: &Path) -> (bool, String) {
    let mut checker = command(&repo_root());
    checker
        .arg("--queue")
        .arg(dir.join("REVIEW_QUEUE.md"))
        .arg("--archive")
        .arg(dir.join("archive"));
    let out = checker.timed_output().expect("python3 ran the queue check");
    let text =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    (out.status.success(), text)
}

fn fails_naming(dir: &Path, fault: &str) {
    let (passed, out) = check(dir);
    assert!(!passed, "queue check passed:\n{out}");
    assert!(out.contains(fault), "fault {fault:?} not named:\n{out}");
}

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

/// A repo holding the real queue and archive on `main`, with the union attribute. The archive lives at `archive/`.
fn repo(name: &str) -> PathBuf {
    let (root, dir) = (
        repo_root(),
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("qa_rq_union_{name}")),
    );
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("archive")).expect("dir made");
    fs::copy(root.join("REVIEW_QUEUE.md"), dir.join("REVIEW_QUEUE.md")).expect("queue copied");
    for file in ["M0.md", "untangling.md"] {
        fs::copy(
            root.join("docs/archive/review_queue").join(file),
            dir.join("archive").join(file),
        )
        .expect("archive copied");
    }
    fs::copy(root.join(".gitattributes"), dir.join(".gitattributes")).expect("attrs copied");
    git(&dir, &["init", "-q", "-b", "main"]);
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-qm", "base"]);
    dir
}

fn queue(dir: &Path) -> String {
    fs::read_to_string(dir.join("REVIEW_QUEUE.md")).expect("queue read")
}

/// Branches `name` from `main`, applies `edit` to its files, commits it, and asserts it passes alone.
fn side(dir: &Path, name: &str, edit: impl FnOnce(&Path)) {
    git(dir, &["checkout", "-q", "-b", name, "main"]);
    edit(dir);
    git(dir, &["add", "."]);
    git(dir, &["commit", "-qm", name]);
    let (passed, out) = check(dir);
    assert!(passed, "branch {name} fails alone:\n{out}");
}

/// Merges `a` into `b` (the current branch), which must be clean: union never conflicts.
fn merge(dir: &Path) {
    git(dir, &["merge", "-q", "--no-edit", "a"]);
}

fn entry(id: u32, question: &str) -> String {
    format!(
        "## RQ-{id}: {question}\n\n- **File, section:** `{question}.md` § \"One\".\n- **Applied meanwhile:** the answer that {question} gives.\n"
    )
}

/// Appends a new entry to the real queue.
fn file_entry(dir: &Path, id: u32, question: &str) {
    let text = queue(dir).trim_end().to_owned() + "\n\n" + &entry(id, question);
    fs::write(dir.join("REVIEW_QUEUE.md"), text).expect("queue written");
}

/// The real queue's last entry's text, heading included.
fn last_entry(dir: &Path) -> String {
    let text = queue(dir);
    let at = text.rfind("\n## RQ-").expect("the queue has an entry") + 1;
    text[at..].to_owned()
}

/// Adds `note`, wrapped as given, at the end of the real queue's last entry.
fn add_note(dir: &Path, note: &str) {
    let text = queue(dir).trim_end().to_owned() + "\n" + note + "\n";
    fs::write(dir.join("REVIEW_QUEUE.md"), text).expect("queue written");
}

#[test]
fn two_branches_taking_one_id_on_the_real_queue_are_caught() {
    let dir = repo("collide");
    side(&dir, "a", |d| file_entry(d, 999_998, "Alpha things"));
    side(&dir, "b", |d| file_entry(d, 999_998, "Beta matters"));
    merge(&dir);
    fails_naming(&dir, "RQ-999998 is two entries");
}

negative_control!(
    two_branches_taking_one_id_on_the_real_queue_are_caught,
    "two branches taking ids of their own merge and pass, so the failure check must fail on them",
    expected = "queue check passed",
    {
        let dir = repo("ctl_collide");
        side(&dir, "a", |d| file_entry(d, 999_998, "Alpha things"));
        side(&dir, "b", |d| file_entry(d, 999_999, "Beta matters"));
        merge(&dir);
        fails_naming(&dir, "is two entries");
    }
);

#[test]
fn one_branch_archiving_an_entry_the_other_edits_is_caught() {
    let dir = repo("archive");
    side(&dir, "a", |d| {
        let last = last_entry(d);
        let kept = queue(d).replace(&last, "");
        fs::write(d.join("REVIEW_QUEUE.md"), kept).expect("queue written");
        let archived = fs::read_to_string(d.join("archive/M0.md")).expect("archive read");
        fs::write(d.join("archive/M0.md"), archived + "\n" + &last).expect("archive written");
    });
    side(&dir, "b", |d| {
        add_note(d, "- **31 Oct 2026:** a dated note from the other branch.");
    });
    merge(&dir);
    fails_naming(&dir, "is an entry in both");
}

negative_control!(
    one_branch_archiving_an_entry_the_other_edits_is_caught,
    "a branch that files a new entry beside the archiving one leaves no reopened id, so the failure check must fail",
    expected = "queue check passed",
    {
        let dir = repo("ctl_archive");
        side(&dir, "a", |d| {
            let last = last_entry(d);
            let kept = queue(d).replace(&last, "");
            fs::write(d.join("REVIEW_QUEUE.md"), kept).expect("queue written");
            let archived = fs::read_to_string(d.join("archive/M0.md")).expect("archive read");
            fs::write(d.join("archive/M0.md"), archived + "\n" + &last).expect("archive written");
        });
        // Base's first entry is untouched by `a`: a new one before it can't reopen the archived one.
        side(&dir, "b", |d| {
            let new = entry(999_999, "Beta matters") + "\n";
            let text = queue(d).replacen("\n## RQ-", &format!("\n{new}## RQ-"), 1);
            fs::write(d.join("REVIEW_QUEUE.md"), text).expect("queue written");
        });
        merge(&dir);
        fails_naming(&dir, "is an entry in both");
    }
);

#[test]
fn an_id_twice_across_the_archive_is_caught() {
    // OPERATIONS names this one ("twice across the archive"); only queue-against-archive is tested in plan_check.rs.
    let dir = repo("archive_twice");
    let (one, two) = (dir.join("archive/M0.md"), dir.join("archive/M1.md"));
    let more = fs::read_to_string(&one).expect("read") + "\n" + &entry(999_998, "Alpha things");
    fs::write(&one, more).expect("written");
    fs::write(&two, entry(999_998, "Beta matters")).expect("written");
    fails_naming(&dir, "RQ-999998 is an entry in both");
}

negative_control!(
    an_id_twice_across_the_archive_is_caught,
    "ids of their own across two archive files pass, so the failure check must fail on them",
    expected = "queue check passed",
    {
        let dir = repo("ctl_archive_twice");
        let two = dir.join("archive/M1.md");
        fs::write(&two, entry(999_998, "Alpha things")).expect("written");
        fails_naming(&dir, "is an entry in both");
    }
);

#[test]
fn one_note_edited_on_two_sides_and_rewrapped_is_caught() {
    // This queue wraps at 120, so a reworded paragraph often changes its line count. OPERATIONS § "Merging" step 2:
    // "a run of up to eight lines next to one that reads the same"; the union leaves three lines next to four.
    let dir = repo("rewrap");
    side(&dir, "a", |d| {
        add_note(d, "- **31 Oct 2026:** the wrapped text of one note that one branch has written over\n  three separate lines of the queue so that it runs on and on\n  until it stops here, for R-1.");
    });
    side(&dir, "b", |d| {
        add_note(d, "- **31 Oct 2026:** the wrapped text of one note that one\n  branch has written over three separate lines\n  of the queue so that it runs\n  on and on until it stops there, for R-1.");
    });
    merge(&dir);
    fails_naming(&dir, "are two versions of one text");
}

negative_control!(
    one_note_edited_on_two_sides_and_rewrapped_is_caught,
    "two notes in different words at different wraps pass, so the failure check must fail on them",
    expected = "queue check passed",
    {
        let dir = repo("ctl_rewrap");
        side(&dir, "a", |d| {
            add_note(d, "- **31 Oct 2026:** keep the first option as it is applied now, because nothing\n  built so far depends on the other one, and no task waits.");
        });
        side(&dir, "b", |d| {
            add_note(d, "- **31 Oct 2026 (later):** the owner moved this to\n  another milestone, whose gate\n  reads the census first and\n  rules on it there.");
        });
        merge(&dir);
        fails_naming(&dir, "are two versions of one text");
    }
);
