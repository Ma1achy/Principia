//! QA tests for TASK-M0-39, written from the requirement it closes:
//! - REQ-VAL-175: "qa_TASK-M0-26_r217's out-of-group grandchild must be put in its own process group at spawn, not by
//!   perl's setpgrp, so its two controls trip under CI load (R-276)."
//! - R-276: "the out-of-group grandchild leaves the process group only when perl runs `setpgrp`, and the 1 s timeout can
//!   fire first. The grandchild is put in its own process group at spawn, not by perl."
//!
//! "At spawn" is checked at the earliest moment the grandchild's pid is known outside its spawner: the pid file the
//! spawner writes. By then the grandchild must already lead a process group of its own, whatever happens (or does not
//! yet happen) inside it afterwards. A grandchild that leaves the group later (perl's `setpgrp` after a delay, which is
//! what load does to it) is still in its spawner's group at that moment, and fails the check.
//!
//! The two controls tripping under load is REQ-VAL-175's CI soak; this file checks the property that makes them trip.
//! Each test registers a negative control (R-176, R-199, R-212). Children are spawned only through the helper. Unix only:
//! elsewhere there are no process groups.
#![cfg(unix)]

use std::path::Path;
use std::process::Command;

use validation::negative_control;
use validation::spawn::Spawn;

#[path = "support/scratch.rs"]
mod scratch;
use scratch::Scratch;

/// A fresh scratch directory under the target's tmp dir: deleted when the test passes, kept with its path printed when
/// it fails (R-342).
fn scratch() -> Scratch {
    let dir = Scratch::new("qa_m039");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The pid written to `file`.
fn pid_in(file: &Path) -> String {
    let pid = std::fs::read_to_string(file)
        .unwrap_or_else(|e| panic!("no pid in {}: {e}", file.display()));
    let pid = pid.trim().to_owned();
    assert!(!pid.is_empty(), "an empty pid in {}", file.display());
    pid
}

/// The process group of the live process `pid` (`ps -o pgid= -p <pid>`).
fn group_of(pid: &str) -> String {
    let o = Command::new("ps")
        .args(["-o", "pgid=", "-p", pid])
        .timed_output()
        .expect("ps ran");
    let group = String::from_utf8_lossy(&o.stdout).trim().to_owned();
    assert!(
        o.status.success() && !group.is_empty(),
        "the grandchild {pid} is not running"
    );
    group
}

/// Runs `sh -c <script> qa_m039_spawner <dir>` through the helper; the script writes its own pid to `$1/spawner` and the
/// grandchild's to `$1/grandchild`, and sends the grandchild's output to /dev/null, so the helper returns as soon as the
/// script ends. Returns the grandchild's and the spawner's pids, the grandchild's group read at once afterwards, and the
/// scratch directory, for the caller to hold until its test ends.
fn spawn_grandchild(script: &str) -> (String, String, String, Scratch) {
    let dir = scratch();
    let o = Command::new("sh")
        .args([
            "-c",
            &format!("echo $$ > \"$1/spawner\"; {script}"),
            "qa_m039_spawner",
        ])
        .arg(&*dir)
        .timed_output()
        .expect("the spawner ran");
    assert!(
        o.status.success(),
        "the spawner failed: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    let grandchild = pid_in(&dir.join("grandchild"));
    let spawner = pid_in(&dir.join("spawner"));
    let group = group_of(&grandchild);
    (grandchild, spawner, group, dir)
}

/// The grandchild `script` starts is in a process group of its own (it leads it) and not in its spawner's group, as
/// soon as its pid is known. The grandchild is killed before the check, so a failing run leaks nothing.
fn check_own_group_at_spawn(script: &str) {
    let (grandchild, spawner, group, _dir) = spawn_grandchild(script);
    let _ = Command::new("kill")
        .args(["-9", &grandchild])
        .timed_output();
    assert!(
        group == grandchild && group != spawner,
        "the grandchild {grandchild} was not in a process group of its own at spawn: its group is {group}, its \
         spawner's is {spawner}"
    );
}

/// qa_TASK-M0-26_r217's spawner (`qa_child`'s `r217_out_of_group`), starting the same `sleep 37`.
const AT_SPAWN: &str = concat!(
    "\"",
    env!("CARGO_BIN_EXE_qa_child"),
    "\" r217_out_of_group \"$1/grandchild\" sleep 37 > /dev/null 2>&1"
);

/// The old grandchild, leaving the group by perl's `setpgrp`, here 2 s late: what CI load did to it (R-276).
#[cfg(feature = "controls")]
const BY_PERL_LATE: &str =
    "perl -e 'select(undef, undef, undef, 2); setpgrp(0, 0); exec @ARGV' sleep 37 \
     > /dev/null 2>&1 & echo $! > \"$1/grandchild\"";

#[test]
fn qa_m039_the_r217_grandchild_is_in_its_own_process_group_at_spawn() {
    check_own_group_at_spawn(AT_SPAWN);
}

negative_control!(
    qa_m039_the_r217_grandchild_is_in_its_own_process_group_at_spawn,
    "a grandchild that leaves its spawner's group by perl's setpgrp, 2 s late, required to be out of it at spawn",
    expected = "was not in a process group of its own at spawn",
    check_own_group_at_spawn(BY_PERL_LATE)
);
