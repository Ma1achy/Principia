//! QA tests for TASK-M0-33, written from REQ-VAL-165 and R-270: "one build directory per fixture type shared by its
//! copies, kept across runs"; and from R-231 / R-224 as R-270 reads them: a fixture's directory is never used by a
//! test and its control at once, and REQ-VAL-164: no cargo run replaces a binary another test is spawning. nextest
//! runs each test in a process of its own, so a type's directory must be held by one process at a time, not only by
//! one thread.
//!
//! Each test registers a negative control (R-176, R-212). Children are spawned only through the helper (R-214).
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use validation::negative_control;
use validation::spawn::Spawn;

#[path = "support/own_target.rs"]
mod own_target;
use own_target::{fixture_files, Lease, FIXTURES};

/// A fresh pool of this file's own, named `pool`, and the files of a one-file source to copy into its leases.
fn fresh_pool(pool: &str) -> Vec<(PathBuf, Vec<u8>)> {
    assert_ne!(pool, FIXTURES, "the test must not disturb the shared pool");
    let _ = std::fs::remove_dir_all(Path::new(env!("CARGO_TARGET_TMPDIR")).join(pool));
    let source = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("qa_m0_33_type")
        .join(format!("{pool}-source"));
    let _ = std::fs::remove_dir_all(&source);
    std::fs::create_dir_all(source.join("src")).unwrap();
    std::fs::write(source.join("src/lib.rs"), "pub fn g() {}\n").unwrap();
    fixture_files(&source)
}

// ---------------------------------------------------------------------------------------------------------------------
// R-270: one build directory per fixture type, shared by its copies.

/// Three copies of fixture type `x`, each made in the lease `take` gives for it, one after the other: all three are
/// in one directory of the pool, and each copy's files are there.
fn check_copies_share_one_directory(pool: &str, take: fn(&str, &str) -> Lease) {
    let files = fresh_pool(pool);
    let mut dirs: Vec<PathBuf> = Vec::new();
    for copy in ["a", "b", "c"] {
        let lease = take(pool, copy);
        let written = lease.copy(copy, &files);
        assert!(
            written.join("src/lib.rs").exists(),
            "copy {copy} was not written"
        );
        if !dirs.contains(&lease.dir().to_path_buf()) {
            dirs.push(lease.dir().to_path_buf());
        }
    }
    assert_eq!(
        dirs.len(),
        1,
        "the copies of fixture type x build in {} directories, not one: {dirs:?}",
        dirs.len()
    );
}

#[test]
fn qa_m0_33_the_copies_of_a_fixture_type_share_one_build_directory() {
    check_copies_share_one_directory("qa_m0_33_type-share", |pool, _copy| {
        Lease::take(pool, Some("x"))
    });
}

negative_control!(
    qa_m0_33_the_copies_of_a_fixture_type_share_one_build_directory,
    "a directory per copy, as before R-270, required to be one directory per fixture type",
    expected = "directories, not one",
    check_copies_share_one_directory("qa_m0_33_type-share-control", |pool, copy| {
        Lease::take(pool, Some(copy))
    })
);

// ---------------------------------------------------------------------------------------------------------------------
// R-224, REQ-VAL-164 under nextest: a type's directory is held by one process at a time.

/// Set in the child process: the directory the child and its parent signal through.
const CHILD_DIR: &str = "QA_M0_33_TYPE_CHILD_DIR";
/// Set in the child process: the pool the child leases type `x` of.
const CHILD_POOL: &str = "QA_M0_33_TYPE_CHILD_POOL";

/// The child's side: hold type `x` of the pool, say so in `held`, wait for `go`, write `released`, then let go.
fn child(signals: &Path, pool: &str) {
    let lease = Lease::take(pool, Some("x"));
    std::fs::write(signals.join("held"), lease.dir().to_str().unwrap()).unwrap();
    let started = Instant::now();
    while !signals.join("go").exists() {
        assert!(
            started.elapsed() < Duration::from_secs(60),
            "the parent never said go"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    std::fs::write(signals.join("released"), "").unwrap();
    drop(lease);
}

/// Another process (this test binary, running this test as the child) holds type `x` of a pool of this test's own
/// until told to go; meanwhile this process takes the lease `take` gives. That lease must be granted only once the
/// child has let go, and in `x`'s directory. The child is told to go once this process's lease is granted, or after
/// [`GIVE`], whichever comes first.
fn check_one_process_at_a_time(test: &str, pool: &str, take: fn(&str) -> Lease) {
    /// How long a lease that waits for the child is given before the child is told to go.
    const GIVE: Duration = Duration::from_secs(2);
    if let (Ok(signals), Ok(pool)) = (std::env::var(CHILD_DIR), std::env::var(CHILD_POOL)) {
        return child(Path::new(&signals), &pool);
    }
    fresh_pool(pool);
    let signals = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("qa_m0_33_type")
        .join(format!("{pool}-signals"));
    let _ = std::fs::remove_dir_all(&signals);
    std::fs::create_dir_all(&signals).unwrap();
    let exe = std::env::current_exe().unwrap();
    let (child_signals, child_pool, child_test) =
        (signals.clone(), pool.to_owned(), test.to_owned());
    let child_run = std::thread::spawn(move || {
        Command::new(exe)
            .args([&child_test, "--exact", "--nocapture", "--test-threads", "1"])
            .env(CHILD_DIR, &child_signals)
            .env(CHILD_POOL, &child_pool)
            .output_within(Duration::from_secs(120))
    });
    let started = Instant::now();
    while !signals.join("held").exists() {
        assert!(
            started.elapsed() < Duration::from_secs(60) && !child_run.is_finished(),
            "the child process never held type x"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    let child_dir = PathBuf::from(std::fs::read_to_string(signals.join("held")).unwrap());
    let (granted, grant) = mpsc::channel();
    let (taker_signals, taker_pool) = (signals.clone(), pool.to_owned());
    let taker = std::thread::spawn(move || {
        let lease = take(&taker_pool);
        let released = taker_signals.join("released").exists();
        let _ = granted.send(());
        (released, lease.dir().to_path_buf())
    });
    let _ = grant.recv_timeout(GIVE);
    std::fs::write(signals.join("go"), "").unwrap();
    let (released, dir) = taker.join().unwrap();
    let output = child_run
        .join()
        .unwrap()
        .expect("the child process ran within its timeout");
    assert!(
        output.status.success(),
        "the child process failed:\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        released,
        "two processes held fixture type x's directory at once"
    );
    assert_eq!(
        dir, child_dir,
        "the lease of type x waiting on another process came to another directory"
    );
}

#[test]
fn qa_m0_33_a_fixture_type_is_held_by_one_process_at_a_time() {
    check_one_process_at_a_time(
        "qa_m0_33_a_fixture_type_is_held_by_one_process_at_a_time",
        "qa_m0_33_type-process",
        |pool| Lease::take(pool, Some("x")),
    );
}

negative_control!(
    qa_m0_33_a_fixture_type_is_held_by_one_process_at_a_time,
    "this process's lease taken as a directory of no type, required to wait for the child holding x",
    expected = "two processes held fixture type x's directory at once",
    check_one_process_at_a_time(
        "qa_m0_33_a_fixture_type_is_held_by_one_process_at_a_time",
        "qa_m0_33_type-process-control",
        |pool| Lease::take(pool, None),
    )
);
