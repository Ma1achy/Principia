//! QA for fix-rq-union-control, from the rule in `plan/OPERATIONS.md` § "Merging" step 2: the queue check fails on "a
//! run of up to eight lines next to one that reads the same, of that length or a line more or less, an edit in an id
//! (`R-n`, `RQ-n`, `TASK-…`, `REQ-…`, `PR #n`) or a `line N` counting as a difference", and lines that differ only in
//! plain numbers are a list's siblings, which pass. Each case sits at the end of this tree's real queue, checked with
//! its real archive; the two-versions case is a real union merge of two branches.

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

fn passes(dir: &Path) {
    let (passed, out) = check(dir);
    assert!(passed, "queue check failed:\n{out}");
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

/// A scratch directory holding the real queue and archive (at `archive/`) and the union attribute.
fn copy(name: &str) -> PathBuf {
    let (root, dir) = (
        repo_root(),
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("qa_fix_rq_union_{name}")),
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
    dir
}

/// The real queue, with `lines` appended to its last entry.
fn queue_with(dir: &Path, lines: &[String]) -> String {
    let text = fs::read_to_string(dir.join("REVIEW_QUEUE.md")).expect("queue read");
    queue_with_text(&text, lines)
}

/// `text`, with `lines` appended to its last entry.
fn queue_with_text(text: &str, lines: &[String]) -> String {
    text.trim_end().to_owned() + "\n" + &lines.join("\n") + "\n"
}

/// A copy of the real queue whose last entry ends with `lines`, as a union merge or one branch leaves them.
fn queue_ending(name: &str, lines: &[String]) -> PathBuf {
    let dir = copy(name);
    let text = queue_with(&dir, lines);
    fs::write(dir.join("REVIEW_QUEUE.md"), text).expect("queue written");
    dir
}

// ---- A real two-versions case whose two runs meet at a pair of siblings ---------------------------------------------

/// A three-line paragraph whose first and last lines are a list's siblings (N = 6, N = 12) around a middle line.
fn paragraph(runs: &str, middle: &str) -> Vec<String> {
    vec![
        format!("- at N = 6 the run holds in {runs} runs, as the long test of it shows."),
        format!("  The fix is {middle}, applied in the long note written here."),
        format!("- at N = 12 the run holds in {runs} runs, as the long test of it shows."),
    ]
}

/// The real queue, its last entry ending with `paragraph("1", "R-699")` on `main`; branch `a` rewrites it to
/// `paragraph("2", ours)` and branch `b` to `paragraph("3", theirs)`, every line changed on both sides, so the union
/// keeps all three lines of each, one version after the other: the first's last line (N = 12) meets the second's
/// first (N = 6), siblings.
fn paragraph_edited_on_two_sides(name: &str, ours: &str, theirs: &str) -> PathBuf {
    let dir = copy(name);
    let write = |runs: &str, middle: &str| {
        let text = fs::read_to_string(dir.join("REVIEW_QUEUE.md")).expect("queue read");
        let base = text
            .split("\n- at N = 6 the run holds in ")
            .next()
            .expect("split")
            .to_owned();
        let edited = queue_with_text(&base, &paragraph(runs, middle));
        fs::write(dir.join("REVIEW_QUEUE.md"), edited).expect("queue written");
    };
    git(&dir, &["init", "-q", "-b", "main"]);
    write("1", "R-699");
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-qm", "base"]);
    for (branch, runs, middle) in [("a", "2", ours), ("b", "3", theirs)] {
        git(&dir, &["checkout", "-q", "-b", branch, "main"]);
        write(runs, middle);
        git(&dir, &["commit", "-qam", branch]);
        let (passed, out) = check(&dir);
        assert!(passed, "branch {branch} fails alone:\n{out}");
    }
    git(&dir, &["merge", "-q", "--no-edit", "a"]);
    let merged = fs::read_to_string(dir.join("REVIEW_QUEUE.md")).expect("queue read");
    let (a, b) = (paragraph("2", ours), paragraph("3", theirs));
    let (ab, ba) = (
        [&a[..], &b[..]].concat().join("\n"),
        [&b[..], &a[..]].concat().join("\n"),
    );
    assert!(
        merged.contains(&ab) || merged.contains(&ba),
        "the union did not keep both versions whole, one after the other:\n{merged}"
    );
    dir
}

#[test]
fn versions_differing_in_a_ruling_that_meet_at_siblings_are_caught() {
    // Two three-line versions, the second's middle line naming another ruling: "a run of up to eight lines next to
    // one that reads the same", the id edit "counting as a difference". That the runs' ends are siblings (N = 12 then
    // N = 6) does not make the versions siblings.
    let dir = paragraph_edited_on_two_sides("join_versions", "R-700", "R-701");
    fails_naming(&dir, "are two versions of one text");
}

negative_control!(
    versions_differing_in_a_ruling_that_meet_at_siblings_are_caught,
    "the same two versions differing only in plain numbers under one ruling are siblings and pass",
    expected = "queue check passed",
    {
        let dir = paragraph_edited_on_two_sides("ctl_join_versions", "R-700 in 2", "R-700 in 3");
        fails_naming(&dir, "are two versions of one text");
    }
);

// ---- Siblings at other joins: a list's siblings with short lines about them pass ------------------------------------

/// One line of a list, at `n`, naming `id`.
fn sibling(id: &str, n: u32) -> String {
    format!("- **31 Oct 2026:** closed by {id} at N = {n}, in the very words of the long note of")
}

/// The tail of a paragraph that wraps to one short line.
fn long_then_short() -> Vec<String> {
    vec![
        "- **Note:** the long note of the base runs on and on, until it comes to the end of"
            .to_owned(),
        "  it.".to_owned(),
    ]
}

/// Three siblings after a paragraph's short tail; `last` names the third's id.
fn three_after_a_tail(last: &str) -> Vec<String> {
    [
        long_then_short(),
        vec![sibling("R-700", 6), sibling("R-700", 12), sibling(last, 12)],
    ]
    .concat()
}

#[test]
fn three_siblings_after_a_short_tail_pass() {
    let mut lines = three_after_a_tail("R-700");
    lines[4] = sibling("R-700", 24);
    passes(&queue_ending("three_after_tail", &lines));
}

negative_control!(
    three_siblings_after_a_short_tail_pass,
    "the third line naming another ruling is a version of the second, so the passes check must fail on it",
    expected = "queue check failed",
    {
        passes(&queue_ending("ctl_three_after_tail", &three_after_a_tail("R-701")));
    }
);

/// Two siblings between a paragraph's short tail and a short line after them; `second` names the second's id and `n`
/// its number.
fn between_two_short(second: &str, n: u32) -> Vec<String> {
    [
        long_then_short(),
        vec![sibling("R-700", 6), sibling(second, n), "  it.".to_owned()],
    ]
    .concat()
}

#[test]
fn siblings_between_two_short_lines_pass() {
    passes(&queue_ending(
        "between_short",
        &between_two_short("R-700", 12),
    ));
}

negative_control!(
    siblings_between_two_short_lines_pass,
    "the second line naming another ruling is a version of the first, so the passes check must fail on it",
    expected = "queue check failed",
    {
        passes(&queue_ending("ctl_between_short", &between_two_short("R-701", 6)));
    }
);

/// Two siblings after a paragraph that wraps to two short lines; `second` names the second's id and `n` its number.
fn after_two_short(second: &str, n: u32) -> Vec<String> {
    vec![
        "- **Note:** the long note of the base runs on and on, until it comes to".to_owned(),
        "  the end".to_owned(),
        "  of it.".to_owned(),
        sibling("R-700", 12),
        sibling(second, n),
    ]
}

#[test]
fn siblings_after_two_short_lines_pass() {
    passes(&queue_ending(
        "after_two_short",
        &after_two_short("R-700", 6),
    ));
}

negative_control!(
    siblings_after_two_short_lines_pass,
    "the second line naming another ruling is a version of the first, so the passes check must fail on it",
    expected = "queue check failed",
    {
        passes(&queue_ending("ctl_after_two_short", &after_two_short("R-701", 12)));
    }
);

/// Two siblings, each wrapped to a short tail of its own, as this queue's 120-column wrap leaves a list's items;
/// `second` names the second's id and `n` its number.
fn each_wrapped(second: &str, n: u32) -> Vec<String> {
    vec![
        sibling("R-700", 6),
        "  it.".to_owned(),
        sibling(second, n),
        "  it.".to_owned(),
    ]
}

#[test]
fn siblings_each_wrapped_to_a_short_tail_pass() {
    // Two items of a list, differing only in a plain number, each run to a short second line: siblings, which pass,
    // however the wrap falls about them.
    passes(&queue_ending("each_wrapped", &each_wrapped("R-700", 12)));
}

negative_control!(
    siblings_each_wrapped_to_a_short_tail_pass,
    "the second item naming another ruling is a version of the first, so the passes check must fail on it",
    expected = "queue check failed",
    {
        passes(&queue_ending("ctl_each_wrapped", &each_wrapped("R-701", 6)));
    }
);
