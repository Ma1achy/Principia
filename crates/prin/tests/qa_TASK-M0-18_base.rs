//! QA tests for TASK-M0-18, written from REQ-TOOL-119's definition in render_gui_spec § "Profiler", "A file with no
//! frames (R-323; BASE applied per R-204)": a BASE trace with no frame records has no p95 to compare, so
//! `prin profile diff` exits 2, as it does for a file it cannot read, and says that BASE has no frame records. Both a
//! complete BASE (header and summary) and an incomplete one (header alone, or its only frame line cut off). Each test
//! has its negative control (R-176).
// The file name `qa_TASK-M0-18_base` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

use validation::spawn::Spawn;

fn prin(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_prin"))
        .args(args)
        .timed_output()
        .expect("cannot run prin")
}

/// A fresh path in this test target's scratch directory.
fn scratch(name: &str) -> PathBuf {
    static N: AtomicU32 = AtomicU32::new(0);
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("qa_TASK-M0-18_base");
    std::fs::create_dir_all(&dir).expect("cannot create the scratch directory");
    let n = N.fetch_add(1, Ordering::SeqCst);
    let path = dir.join(format!("{}-{n}-{name}", std::process::id()));
    let _ = std::fs::remove_file(&path);
    path
}

fn path_str(path: &std::path::Path) -> &str {
    path.to_str().expect("a UTF-8 path")
}

/// A real trace from `prin profile --scenario synthetic_frames --frames 20`: header, 20 frame lines, summary.
fn synthetic() -> String {
    let path = scratch("synthetic.jsonl");
    let out = prin(&[
        "profile",
        "--scenario",
        "synthetic_frames",
        "--frames",
        "20",
        "--json",
        path_str(&path),
    ]);
    assert!(
        out.status.success(),
        "prin profile failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    std::fs::read_to_string(&path).expect("prin profile wrote no file")
}

/// The header line, frame lines and summary line of a complete trace.
fn split(text: &str) -> (String, Vec<String>, String) {
    let lines: Vec<&str> = text.lines().collect();
    assert!(lines.len() >= 3, "a trace too short to split: {text}");
    let header = lines[0].to_string();
    let summary = lines[lines.len() - 1].to_string();
    assert!(
        summary.contains("leak_flags"),
        "the last line is not the summary: {summary}"
    );
    let frames = lines[1..lines.len() - 1]
        .iter()
        .map(|l| l.to_string())
        .collect();
    (header, frames, summary)
}

/// `prin profile diff BASE NEW --threshold 5%` on BASE's text and a full synthetic NEW: exits 2 and names BASE as the
/// file with no frame records.
fn check_base_refused(base: &str, what: &str) {
    let new_text = synthetic();
    let (b, n) = (scratch("base.jsonl"), scratch("new.jsonl"));
    std::fs::write(&b, base).expect("cannot write BASE");
    std::fs::write(&n, &new_text).expect("cannot write NEW");
    let out = prin(&[
        "profile",
        "diff",
        path_str(&b),
        path_str(&n),
        "--threshold",
        "5%",
    ]);
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        out.status.code(),
        Some(2),
        "{what} gives exit {:?}, not 2: {text}",
        out.status.code()
    );
    // The phrase, not the word "base" alone: the BASE path itself contains "base".
    let lower = text.to_lowercase();
    assert!(
        lower.contains("base has no frame"),
        "{what}: the diff does not say BASE has no frame records: {text}"
    );
    // The NEW of this diff has frames; the refusal must not blame NEW.
    assert!(
        !lower.contains("new has no frame"),
        "{what}: the diff blames NEW, which has frames: {text}"
    );
}

#[test]
fn qa_profile_diff_base_without_frames_exits_2() {
    let (header, frames, summary) = split(&synthetic());
    check_base_refused(
        &format!("{header}\n{summary}\n"),
        "a complete BASE with no frame records",
    );
    check_base_refused(
        &format!("{header}\n"),
        "an incomplete BASE, its header line alone",
    );
    check_base_refused(
        &format!("{header}\n{}", &frames[0][..frames[0].len() / 2]),
        "an incomplete BASE whose only frame line is cut off",
    );
}

validation::negative_control!(
    qa_profile_diff_base_without_frames_exits_2,
    "a BASE with one frame record must not exit 2",
    expected = "a BASE with one frame record gives exit Some(",
    {
        let (header, frames, summary) = split(&synthetic());
        check_base_refused(
            &format!("{header}\n{}\n{summary}\n", frames[19]),
            "a BASE with one frame record",
        );
    }
);
