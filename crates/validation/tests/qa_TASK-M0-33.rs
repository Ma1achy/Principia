//! QA tests for TASK-M0-33, written from REQ-VAL-165 and R-231:
//! - REQ-VAL-165: "fixture workspaces must be written only when their content changes, each with its own target
//!   directory kept across runs; and the spawn helper's grace and timeout must be injectable so its own tests need not
//!   wait the calibrated values"; verify: "a warm second run of a fixture test rebuilds nothing; no spawn-helper test
//!   waits the calibrated 5 s grace or 300 s timeout".
//! - R-231: "The spawn helper's calibrated values (REQ-VAL-156) are unchanged": a caller that injects nothing still gets
//!   R-217's grace after SIGTERM.
//!
//! None of these tests waits the calibrated grace or timeout. Unix only: the grace is R-217's, between SIGTERM and
//! SIGKILL to a process group. Each test registers a negative control (R-176, R-212). Children are spawned only
//! through the helper (R-214). The shared fixture pool's leases are tested in `qa_TASK-M0-33_lease.rs`, since
//! `support/own_target.rs` includes `support/fixture_tree.rs` as a module of its own.
#![cfg(unix)]

use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant, SystemTime};

use validation::negative_control;
use validation::spawn::{Spawn, GRACE, TIMEOUT};

#[path = "support/fixture_tree.rs"]
mod fixture_tree;

/// A slack for process start-up, signalling and reaping on a loaded machine; less than the calibrated grace, so a
/// bound of `timeout + injected grace + SLACK` excludes having waited [`GRACE`].
const SLACK: Duration = Duration::from_millis(2500);

/// A directory of this file's own under the target's tmp dir, emptied.
fn scratch(tag: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("qa_m0_33")
        .join(tag);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The pid the helper's timeout error names.
fn pid_named(err: &io::Error) -> String {
    let message = err.to_string();
    message
        .split("(pid ")
        .nth(1)
        .and_then(|rest| rest.split(')').next())
        .unwrap_or_else(|| panic!("the error names no pid: {message}"))
        .to_owned()
}

/// Whether `kill -0 -- -<pgid>` finds a live member of the group.
fn group_alive(pgid: &str) -> bool {
    Command::new("kill")
        .args(["-0", "--", &format!("-{pgid}")])
        .timed_output()
        .expect("kill ran")
        .status
        .success()
}

// ---------------------------------------------------------------------------------------------------------------------
// R-231 / REQ-VAL-156: a caller that injects nothing keeps a grace after SIGTERM.

/// A child timed out after 500 ms by `run`, whose SIGTERM handler takes 0.5 s and then records that it ran: the
/// handler ran to its end, so the group got SIGTERM and a grace before SIGKILL; and the helper returned once the group
/// was gone, without waiting the whole calibrated grace.
fn check_default_grace(tag: &str, run: impl FnOnce(&mut Command) -> io::Result<Output>) {
    let dir = scratch(tag);
    let timeout = Duration::from_millis(500);
    let started = Instant::now();
    let result = run(Command::new("sh")
        .args([
            "-c",
            "trap 'sleep 0.5; echo done > \"$1/done\"; exit 0' TERM; sleep 37 & wait",
            "qa_m0_33_term",
        ])
        .arg(&dir));
    let took = started.elapsed();
    let done = std::fs::read_to_string(dir.join("done")).unwrap_or_default();
    let err = result.expect_err("the child was not timed out");
    assert_eq!(err.kind(), ErrorKind::TimedOut, "{err}");
    assert!(
        !group_alive(&pid_named(&err)),
        "the timed-out child's group outlived the helper"
    );
    assert_eq!(
        done.trim(),
        "done",
        "the SIGTERM handler was not let finish: no grace after SIGTERM"
    );
    assert!(
        took < timeout + GRACE,
        "the helper waited {took:?}, the whole calibrated grace, for a group already gone"
    );
}

#[test]
fn qa_m0_33_output_within_keeps_a_grace_after_sigterm() {
    check_default_grace("default-grace", |c| {
        c.output_within(Duration::from_millis(500))
    });
}

negative_control!(
    qa_m0_33_output_within_keeps_a_grace_after_sigterm,
    "a zero grace injected, which SIGKILLs the handler at once, required to let it finish",
    expected = "the SIGTERM handler was not let finish: no grace after SIGTERM",
    check_default_grace("default-grace-control", |c| c
        .output_within_grace(Duration::from_millis(500), Duration::ZERO))
);

/// The calibrated values stand (REQ-VAL-156, R-231): 300 s timeout, 5 s grace.
fn check_calibrated(timeout: Duration, grace: Duration) {
    assert_eq!(
        (TIMEOUT, GRACE),
        (timeout, grace),
        "the calibrated timeout and grace changed"
    );
}

#[test]
fn qa_m0_33_the_calibrated_values_are_unchanged() {
    check_calibrated(Duration::from_secs(300), Duration::from_secs(5));
}

negative_control!(
    qa_m0_33_the_calibrated_values_are_unchanged,
    "a 2 s grace, required to be the calibrated one",
    expected = "the calibrated timeout and grace changed",
    check_calibrated(Duration::from_secs(300), Duration::from_secs(2))
);

// ---------------------------------------------------------------------------------------------------------------------
// REQ-VAL-165: the grace is injectable, so a test need not wait the calibrated one.

/// A group that ignores SIGTERM, timed out after 300 ms with `injected` grace: the helper ends it no sooner than the
/// timeout plus `expected`, and well before the timeout plus the calibrated grace; nothing of the group is left.
fn check_injected_grace(tag: &str, injected: Duration, expected: Duration) {
    let dir = scratch(tag);
    let timeout = Duration::from_millis(300);
    let started = Instant::now();
    let err = Command::new("sh")
        .args(["-c", "trap '' TERM; sleep 37 & wait", "qa_m0_33_deaf"])
        .arg(&dir)
        .output_within_grace(timeout, injected)
        .expect_err("the child was not timed out");
    let took = started.elapsed();
    assert_eq!(err.kind(), ErrorKind::TimedOut, "{err}");
    assert!(
        !group_alive(&pid_named(&err)),
        "the group ignoring SIGTERM outlived the helper"
    );
    assert!(
        took >= timeout + expected,
        "the group was ended after {took:?}, before the injected grace ran out"
    );
    assert!(
        took < timeout + expected + SLACK && took < timeout + GRACE,
        "the helper waited {took:?}, past the injected grace, towards the calibrated {GRACE:?}"
    );
}

#[test]
fn qa_m0_33_an_injected_grace_is_the_one_waited() {
    check_injected_grace(
        "injected",
        Duration::from_millis(700),
        Duration::from_millis(700),
    );
}

negative_control!(
    qa_m0_33_an_injected_grace_is_the_one_waited,
    "a zero grace injected, required to last 700 ms",
    expected = "before the injected grace ran out",
    check_injected_grace(
        "injected-control",
        Duration::ZERO,
        Duration::from_millis(700)
    )
);

// ---------------------------------------------------------------------------------------------------------------------
// REQ-VAL-165: fixtures are written only when their content changes.

/// Writes `files` under `root` unconditionally, as the fixture writers did before R-231.
#[cfg(feature = "controls")]
fn write_always(root: &Path, files: &[(&str, String)]) {
    for (rel, text) in files {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
}

fn mtime(path: &Path) -> SystemTime {
    std::fs::metadata(path).unwrap().modified().unwrap()
}

/// A tree written by `write_tree`, with a build's `target/` and `Cargo.lock` beside it, then written again by `rewrite`
/// with one file unchanged, one changed and one dropped: the unchanged file keeps its mtime, the changed one holds its
/// new text, the dropped one is gone, and `target/` and `Cargo.lock` are kept.
fn check_rewrite(tag: &str, rewrite: fn(&Path, &[(&str, String)])) {
    let root = scratch(tag);
    fixture_tree::write_tree(
        &root,
        &[
            ("same.rs", "same\n".to_owned()),
            ("sub/changed.rs", "before\n".to_owned()),
            ("dropped.rs", "dropped\n".to_owned()),
        ],
    );
    std::fs::create_dir_all(root.join("target/debug")).unwrap();
    std::fs::write(root.join("target/debug/built"), "built\n").unwrap();
    std::fs::write(root.join("Cargo.lock"), "lock\n").unwrap();
    let before = mtime(&root.join("same.rs"));
    std::thread::sleep(Duration::from_millis(200));
    rewrite(
        &root,
        &[
            ("same.rs", "same\n".to_owned()),
            ("sub/changed.rs", "after\n".to_owned()),
        ],
    );
    assert_eq!(
        mtime(&root.join("same.rs")),
        before,
        "a fixture file whose content did not change was rewritten"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("sub/changed.rs")).unwrap(),
        "after\n",
        "a changed fixture file was not rewritten"
    );
    assert!(
        !root.join("dropped.rs").exists(),
        "a file no longer in the fixture was left in it"
    );
    assert!(
        root.join("target/debug/built").exists() && root.join("Cargo.lock").exists(),
        "the fixture's target directory or lockfile was not kept"
    );
}

#[test]
fn qa_m0_33_a_fixture_file_is_written_only_when_it_changes() {
    check_rewrite("rewrite", |root, files| {
        fixture_tree::write_tree(root, files)
    });
}

negative_control!(
    qa_m0_33_a_fixture_file_is_written_only_when_it_changes,
    "the pre-R-231 writer, which writes every file on every run, required to leave unchanged files alone",
    expected = "a fixture file whose content did not change was rewritten",
    check_rewrite("rewrite-control", write_always)
);

/// A one-file crate at a stable path under the tmp dir, written by `write_tree` and built into its own target
/// directory, then written again by `rewrite` with the same content and built again: the warm build compiles nothing.
fn check_warm_build(tag: &str, rewrite: fn(&Path, &[(&str, String)])) {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("qa_m0_33")
        .join(tag);
    let files = [
        (
            "Cargo.toml",
            "[package]\nname = \"qa_m0_33_fresh\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[workspace]\n"
                .to_owned(),
        ),
        ("src/lib.rs", "pub fn f() -> u32 { 33 }\n".to_owned()),
    ];
    let build = || {
        let output = Command::new(env!("CARGO"))
            // Plain text whatever the caller's `CARGO_TERM_COLOR` (CI sets `always`), so "Compiling <crate>" is
            // matched as written.
            .args(["build", "--color", "never", "--manifest-path"])
            .arg(root.join("Cargo.toml"))
            .env("CARGO_TARGET_DIR", root.join("target"))
            .timed_output()
            .expect("run cargo build on the fixture");
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        assert!(
            output.status.success(),
            "the fixture did not build:\n{stderr}"
        );
        stderr
    };
    fixture_tree::write_tree(&root, &files);
    build();
    std::thread::sleep(Duration::from_millis(200));
    rewrite(&root, &files);
    let warm = build();
    assert!(
        !warm.contains("Compiling qa_m0_33_fresh"),
        "a warm build of an unchanged fixture rebuilt it:\n{warm}"
    );
}

#[test]
fn qa_m0_33_a_warm_build_of_an_unchanged_fixture_rebuilds_nothing() {
    check_warm_build("warm", |root, files| fixture_tree::write_tree(root, files));
}

negative_control!(
    qa_m0_33_a_warm_build_of_an_unchanged_fixture_rebuilds_nothing,
    "the pre-R-231 writer, which writes every file on every run, required to leave the build fresh",
    expected = "a warm build of an unchanged fixture rebuilt it",
    check_warm_build("warm-control", write_always)
);
