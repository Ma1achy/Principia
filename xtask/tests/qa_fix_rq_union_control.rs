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

/// A long line naming no id, set between the real queue's tail and a two-sided case's lines, so that whatever the real
/// queue ends with (a short line naming an id, say) never reads as part of either version.
const SEPARATOR: &str =
    "- **Note:** the long note of the base runs on and on, until it comes to the end of the long base line here.";

/// `text` with `SEPARATOR` and then `lines` appended to its last entry.
fn queue_with_separated(text: &str, lines: &[String]) -> String {
    queue_with_text(text, &[&[SEPARATOR.to_owned()][..], lines].concat())
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

/// The real queue, its last entry ending with `SEPARATOR` and `paragraph("1", "R-699")` on `main`; branch `a` rewrites
/// the paragraph to `paragraph("2", ours)` and branch `b` to `paragraph("3", theirs)`, every line changed on both
/// sides, so the union keeps all three lines of each, one version after the other: the first's last line (N = 12)
/// meets the second's first (N = 6), siblings.
fn paragraph_edited_on_two_sides(name: &str, ours: &str, theirs: &str) -> PathBuf {
    let dir = copy(name);
    let write = |runs: &str, middle: &str| {
        let text = fs::read_to_string(dir.join("REVIEW_QUEUE.md")).expect("queue read");
        let base = text
            .split(&format!("\n{SEPARATOR}"))
            .next()
            .expect("split")
            .to_owned();
        let edited = queue_with_separated(&base, &paragraph(runs, middle));
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

// ---- A wrapped item edited on two sides: its short tails differ in words ---------------------------------------------

/// One item of a list at `n`, wrapped to a short tail `tail`, as this queue's 120-column wrap leaves it.
fn wrapped_item(n: u32, tail: &str) -> Vec<String> {
    vec![
        format!("- at N = {n} the run holds in every long test of it, and is kept"),
        format!("  {tail}."),
    ]
}

/// The real queue, its last entry ending with `SEPARATOR` and `wrapped_item(6, "as it was")` on `main`; branch `a`
/// rewrites both item lines to `wrapped_item(8, ours)` and branch `b` to `wrapped_item(10, theirs)`, so the union keeps
/// both versions whole.
fn item_edited_on_two_sides(name: &str, ours: &str, theirs: &str) -> PathBuf {
    let dir = copy(name);
    let real = fs::read_to_string(dir.join("REVIEW_QUEUE.md")).expect("queue read");
    let write = |lines: Vec<String>| {
        fs::write(
            dir.join("REVIEW_QUEUE.md"),
            queue_with_separated(&real, &lines),
        )
        .expect("queue written");
    };
    git(&dir, &["init", "-q", "-b", "main"]);
    write(wrapped_item(6, "as it was"));
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-qm", "base"]);
    for (branch, n, tail) in [("a", 8, ours), ("b", 10, theirs)] {
        git(&dir, &["checkout", "-q", "-b", branch, "main"]);
        write(wrapped_item(n, tail));
        git(&dir, &["commit", "-qam", branch]);
        let (passed, out) = check(&dir);
        assert!(passed, "branch {branch} fails alone:\n{out}");
    }
    git(&dir, &["merge", "-q", "--no-edit", "a"]);
    let merged = fs::read_to_string(dir.join("REVIEW_QUEUE.md")).expect("queue read");
    let (a, b) = (
        wrapped_item(8, ours).join("\n"),
        wrapped_item(10, theirs).join("\n"),
    );
    assert!(
        merged.contains(&format!("{a}\n{b}")) || merged.contains(&format!("{b}\n{a}")),
        "the union did not keep both versions whole, one after the other:\n{merged}"
    );
    dir
}

#[test]
fn a_wrapped_item_whose_two_versions_differ_in_words_is_caught() {
    // One side says the item is kept "in full", the other "in part": a difference in words, not only in a plain
    // number, so the two runs are not a list's siblings but two versions of one text, as they are on one line
    // ("- at N = 8 … kept in full." next to "- at N = 10 … kept in part."). Where the wrap falls must not hide it.
    let dir = item_edited_on_two_sides("wrapped_versions", "in full", "in part");
    fails_naming(&dir, "are two versions of one text");
}

negative_control!(
    a_wrapped_item_whose_two_versions_differ_in_words_is_caught,
    "the same two items with tails differing only in a plain number are siblings, which pass",
    expected = "queue check passed",
    {
        let dir = item_edited_on_two_sides("ctl_wrapped_versions", "in 2 runs", "in 3 runs");
        fails_naming(&dir, "are two versions of one text");
    }
);

// ---- A `line N` that a short line at the outer edge starts: leaving the line out drops the id ----------------------

/// A paragraph whose short line `edge` ends with the word before the number that starts each of the two lines after
/// it: what a union merge leaves when the base's third line (at 9) is edited to 6 on one side and to 12 on the other,
/// the short line above them kept once.
fn edge_starting_a_number(edge: &str) -> Vec<String> {
    let line = |n: u32| {
        format!("  {n}, which the long dated note of the owner names, as the record has it.")
    };
    vec![
        "- **Note:** the long note of the base runs on and on, as the record has it, until it ends at"
            .to_owned(),
        format!("  {edge}"),
        line(6),
        line(12),
    ]
}

#[test]
fn versions_differing_in_a_line_n_started_by_a_short_line_are_caught() {
    // "line" closing the short line and "6" or "12" opening the next make a `line N`, which step 2 counts as a
    // difference: the two runs ("line 6, …" and "12, …", a line more or less) are two versions, not siblings, though
    // the short line alone names no id.
    fails_naming(
        &queue_ending("edge_line_n", &edge_starting_a_number("line")),
        "are two versions of one text",
    );
}

negative_control!(
    versions_differing_in_a_line_n_started_by_a_short_line_are_caught,
    "with no `line` before them the numbers are plain, so the two lines are siblings and pass",
    expected = "queue check passed",
    {
        fails_naming(
            &queue_ending("ctl_edge_line_n", &edge_starting_a_number("the end, N")),
            "are two versions of one text",
        );
    }
);
