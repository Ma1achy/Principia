//! Writing a stand-in executable never overlaps a spawn (REQ-SYS-070, R-267): on Linux a child forked while another
//! thread has an executable open for writing inherits that descriptor until it execs, and an exec of that executable
//! in the meantime fails with ETXTBSY ("Text file busy"). `validation::spawn::write_executable` writes with no child
//! being spawned through `Spawn`, and no spawn starts until the write is done.

#![cfg(unix)]

use std::process::Command;
use std::sync::{mpsc, Mutex, PoisonError};
use std::thread;
use std::time::Duration;

use validation::spawn::{while_no_spawn, write_executable, Spawn, SPAWN_TIMEOUT};

#[path = "support/scratch.rs"]
mod scratch;
use scratch::Scratch;

/// How long a spawn started during a write is watched: it must not finish while the write lasts. A `true` spawned
/// with nothing held finishes far within it.
const WATCH: Duration = Duration::from_secs(2);

/// Spawns `true` on another thread from inside `hold`, which runs what it is given as a write would run, and checks
/// that the spawn does not finish until `hold` has returned, and then finishes.
fn check_spawn_waits(hold: fn(&mut dyn FnMut())) {
    // One check at a time: the test's write would otherwise hold back its control's spawn.
    static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
    let (tx, rx) = mpsc::channel();
    let mut during = None;
    hold(&mut || {
        let tx = tx.clone();
        thread::spawn(move || {
            let ran = Command::new("true").timed_output();
            let _ = tx.send(
                ran.map(|output| output.status.success())
                    .map_err(|e| e.to_string()),
            );
        });
        during = Some(rx.recv_timeout(WATCH));
    });
    assert!(
        matches!(during, Some(Err(mpsc::RecvTimeoutError::Timeout))),
        "a child was spawned while a stand-in executable was being written: {during:?}"
    );
    let after = rx
        .recv_timeout(SPAWN_TIMEOUT)
        .expect("the spawn never finished");
    assert_eq!(after, Ok(true), "the spawn after the write failed");
}

#[test]
fn stand_in_spawn_waits_for_the_write() {
    check_spawn_waits(|write| while_no_spawn(write));
}

validation::negative_control!(
    stand_in_spawn_waits_for_the_write,
    "the same spawn with the write not held, required to wait",
    expected = "a child was spawned while a stand-in executable was being written",
    check_spawn_waits(|write| write())
);

/// A fresh file path under the target's tmp; one per call, as a test and its control may run at once. Deleted when the
/// test passes, kept when it fails (R-342).
fn scratch(name: &str) -> Scratch {
    Scratch::new(&format!("stand_in_{name}"))
}

/// `write` puts a script printing `ran` at a fresh path, and the script runs.
fn check_runs(name: &str, write: fn(&std::path::Path, &str) -> std::io::Result<()>) {
    let path = scratch(name);
    write(&path, "#!/bin/sh\necho ran\n").expect("the stand-in was written");
    let output = Command::new(&*path).timed_output();
    assert!(
        matches!(&output, Ok(o) if o.status.success() && o.stdout == b"ran\n"),
        "the stand-in did not run: {output:?}"
    );
}

#[test]
fn stand_in_written_executable_runs() {
    check_runs("runs", |path, script| write_executable(path, script));
}

validation::negative_control!(
    stand_in_written_executable_runs,
    "the same script written as a plain file, without its executable mode, required to run",
    expected = "the stand-in did not run",
    check_runs("runs_control", |path, script| std::fs::write(path, script))
);
