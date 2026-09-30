//! qa's tests for TASK-M0-38, written from REQ-SYS-070 (R-267): "Tests that write an executable and then spawn it must
//! not fail with ETXTBSY ("Text file busy") when run in parallel on Linux: writing a stand-in executable and spawning
//! children must not overlap across the test binary's threads."
//!
//! The overlap is made observable on any Unix, without relying on Linux's race: the stand-in is written to a FIFO
//! larger than any pipe buffer, so the writer stays inside `write_executable`, with the file open for writing, until
//! this test drains it. Opening the FIFO's read end returns only once the writer has opened it for writing, so from
//! that moment the write is known to be under way. A child spawned through `Spawn` then must not start until the
//! write is done, and must run after it. The CI soak (50 parallel runs on ubuntu-latest) is the statistical proof on
//! Linux; this is the deterministic one of the property it rests on.
//!
//! The negative control (R-176) writes the same FIFO with a plain `std::fs::write`, which does not hold spawns back.

#![cfg(unix)]

use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Mutex, PoisonError};
use std::thread;
use std::time::Duration;

use validation::spawn::{write_executable, Spawn, SPAWN_TIMEOUT};

/// How long a spawn started during the write is watched for finishing. `true` spawned with nothing held back finishes
/// far within it.
const WATCH: Duration = Duration::from_secs(2);

/// The stand-in's size: above any pipe's buffer (64 KiB on Linux and macOS), so its write blocks until drained.
const SIZE: usize = 1 << 20;

/// A fresh FIFO under the target's tmp, one per call.
fn fifo() -> PathBuf {
    static CALL: AtomicUsize = AtomicUsize::new(0);
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "qa_m038_fifo_{}_{}",
        std::process::id(),
        CALL.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("stand-in");
    let made = Command::new("mkfifo")
        .arg(&path)
        .timed_output()
        .expect("mkfifo ran");
    assert!(made.status.success(), "mkfifo failed: {made:?}");
    path
}

/// Writes a stand-in with `write` into a FIFO, and spawns `true` through `Spawn` while the writer holds it open for
/// writing; checks that the spawn did not finish while the write lasted, and that it ran once the write was done.
fn check_spawn_held_back(write: fn(&Path, Vec<u8>) -> io::Result<()>) {
    // One check at a time: a check's write would hold back the other's spawns.
    static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
    let path = fifo();
    let writer = {
        let path = path.clone();
        thread::spawn(move || write(&path, vec![b'#'; SIZE]))
    };
    // Returns once the writer has opened the FIFO for writing: its write is under way from here until drained.
    let mut reader = File::open(&path).expect("the FIFO opened for reading");
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let ran = Command::new("true").timed_output();
        let _ = tx.send(
            ran.map(|output| output.status.success())
                .map_err(|e| e.to_string()),
        );
    });
    let during = rx.recv_timeout(WATCH);
    // Drained whichever way the check goes, so the writer finishes.
    let mut read = Vec::new();
    reader.read_to_end(&mut read).expect("the FIFO drained");
    let written = writer.join().expect("the writer thread did not panic");
    assert!(written.is_ok(), "the stand-in was not written: {written:?}");
    assert_eq!(read.len(), SIZE, "the stand-in was not written whole");
    assert!(
        matches!(during, Err(mpsc::RecvTimeoutError::Timeout)),
        "a child was spawned while the stand-in executable was open for writing: {during:?}"
    );
    let after = rx.recv_timeout(SPAWN_TIMEOUT);
    assert_eq!(
        after,
        Ok(Ok(true)),
        "the spawn held back by the write did not run after it"
    );
}

#[test]
fn qa_m038_no_spawn_while_a_stand_in_is_open_for_writing() {
    check_spawn_held_back(write_executable);
}

validation::negative_control!(
    qa_m038_no_spawn_while_a_stand_in_is_open_for_writing,
    "the same stand-in written by a plain `std::fs::write`, which holds no spawn back",
    expected = "a child was spawned while the stand-in executable was open for writing",
    check_spawn_held_back(|path, contents| std::fs::write(path, contents))
);
