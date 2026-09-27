//! QA tests for TASK-M0-26 under R-217 (amends R-214), written from the ruling and REQ-VAL-155/156 as it rewords them:
//! - R-217: "Spawn every child in its own process group (process_group(0) on Unix). On timeout, kill the whole group:
//!   SIGTERM, then after a 5 s grace SIGKILL, and reap it. Nothing a child started may survive its timeout."
//! - R-217: "Add a test: a child that starts a grandchild and then hangs. After the timeout, both are gone, and no
//!   process from the group remains."
//! - REQ-VAL-155 verify: "a child that starts a grandchild and then hangs: after the timeout both are gone and no process
//!   from its group remains (R-217)".
//!
//! "Gone" for a grandchild is "no live process": once the group is killed, a grandchild is a zombie only until init
//! reaps it, which the helper does not control. The child itself is the helper's to reap, so for it "gone" is "no
//! process at all" (`kill -0` succeeds on a zombie). Unix only: elsewhere there are no process groups.
//!
//! Each test registers a negative control (R-176, R-199, R-212). Children are spawned only through the helper.
#![cfg(unix)]

use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use validation::negative_control;
use validation::spawn::{Spawn, GRACE};

/// A slack for process start-up, signalling and reaping on a loaded machine.
const SLACK: Duration = Duration::from_secs(5);

/// A fresh scratch directory under the target's tmp dir.
fn scratch(tag: &str) -> PathBuf {
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "qa_r217-{tag}-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The pid written to `file`, if any.
fn pid_in(file: &Path) -> Option<String> {
    let pid = std::fs::read_to_string(file).ok()?.trim().to_owned();
    (!pid.is_empty()).then_some(pid)
}

/// `ps -A -o pid=,pgid=,stat=`: every process, as (pid, pgid, state).
fn processes() -> Vec<(String, String, String)> {
    let o = Command::new("ps")
        .args(["-A", "-o", "pid=,pgid=,stat="])
        .timed_output()
        .expect("ps ran");
    assert!(o.status.success(), "ps failed");
    String::from_utf8_lossy(&o.stdout)
        .lines()
        .filter_map(|l| {
            let mut f = l.split_whitespace();
            Some((
                f.next()?.to_owned(),
                f.next()?.to_owned(),
                f.next()?.to_owned(),
            ))
        })
        .collect()
}

/// Whether `pid` is a live process (present and not a zombie).
fn live(pid: &str) -> bool {
    processes()
        .iter()
        .any(|(p, _, stat)| p == pid && !stat.starts_with('Z'))
}

/// Whether `pid` names any process at all, a zombie included (`kill -0`).
fn exists(pid: &str) -> bool {
    Command::new("kill")
        .args(["-0", pid])
        .timed_output()
        .expect("kill ran")
        .status
        .success()
}

/// The live processes of the process group `pgid`.
fn live_in_group(pgid: &str) -> Vec<String> {
    processes()
        .into_iter()
        .filter(|(_, g, stat)| g == pgid && !stat.starts_with('Z'))
        .map(|(p, _, _)| p)
        .collect()
}

/// Waits up to `SLACK` for `pids` to stop being live (a killed process may take a moment to be torn down); returns the
/// ones still live.
fn still_live(pids: &[&str]) -> Vec<String> {
    let started = Instant::now();
    loop {
        let left: Vec<String> = pids
            .iter()
            .filter(|p| live(p))
            .map(|p| p.to_string())
            .collect();
        if left.is_empty() || started.elapsed() > SLACK {
            return left;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Kills whatever `pids` are left, so a failing run leaks nothing.
fn reap_leftovers(pids: &[&str]) {
    for pid in pids {
        let _ = Command::new("kill").args(["-9", pid]).timed_output();
    }
}

/// Runs `sh -c <script> qa_r217_child <dir>` under `timeout`. The script writes its own pid to `$1/child` and may write
/// others to `$1`. Returns the helper's result, the time it took, and the scratch dir.
fn run(script: &str, timeout: Duration) -> (std::io::Result<Output>, Duration, PathBuf) {
    let dir = scratch("run");
    let started = Instant::now();
    let result = Command::new("sh")
        .args([
            "-c",
            &format!("echo $$ > \"$1/child\"; {script}"),
            "qa_r217_child",
        ])
        .arg(&dir)
        .output_within(timeout);
    (result, started.elapsed(), dir)
}

/// A grandchild started by `start` (which writes its pid to `$1/grandchild`), after which the child hangs.
const IN_GROUP: &str = "sleep 37 & echo $! > \"$1/grandchild\"";
/// The same grandchild, leaving the child's process group first.
#[cfg(feature = "controls")]
const OUT_OF_GROUP: &str =
    "perl -e 'setpgrp(0, 0); exec @ARGV' sleep 37 & echo $! > \"$1/grandchild\"";

// ---------------------------------------------------------------------------------------------------------------------
// R-217: a child that starts a grandchild and then hangs. After the timeout, both are gone.

fn check_child_and_grandchild_gone(start: &str) {
    let (result, _, dir) = run(&format!("{start}; sleep 37"), Duration::from_secs(1));
    let child = pid_in(&dir.join("child")).expect("the child's pid");
    let grandchild = pid_in(&dir.join("grandchild")).expect("the grandchild's pid");
    let _ = std::fs::remove_dir_all(&dir);
    let err = result.expect_err("the hanging child was not timed out");
    let left = still_live(&[&grandchild]);
    let child_exists = exists(&child);
    reap_leftovers(&[&grandchild]);
    assert_eq!(err.kind(), ErrorKind::TimedOut, "not a timeout: {err}");
    assert!(
        !child_exists,
        "the timed-out child {child} still exists (unreaped or alive)"
    );
    assert!(
        left.is_empty(),
        "the grandchild {grandchild} survived the timeout"
    );
}

#[test]
fn qa_r217_a_hanging_child_and_its_grandchild_are_gone_after_the_timeout() {
    check_child_and_grandchild_gone(IN_GROUP);
}

negative_control!(
    qa_r217_a_hanging_child_and_its_grandchild_are_gone_after_the_timeout,
    "a grandchild that leaves the child's process group, beyond the helper's reach, required to be gone",
    expected = "survived the timeout",
    check_child_and_grandchild_gone(OUT_OF_GROUP)
);

// ---------------------------------------------------------------------------------------------------------------------
// R-217: "... and no process from the group remains."

/// `left`, the live processes found in the group `pgid`, is empty.
fn check_group_empty(pgid: &str, left: &[String]) {
    assert!(
        left.is_empty(),
        "processes {left:?} of the group {pgid} remain"
    );
}

/// Times out a child that starts a grandchild in its group and hangs; returns the child's group id (its pid, as the
/// child leads its own group).
fn timed_out_group() -> String {
    let (result, _, dir) = run(&format!("{IN_GROUP}; sleep 37"), Duration::from_secs(1));
    let child = pid_in(&dir.join("child")).expect("the child's pid");
    let grandchild = pid_in(&dir.join("grandchild")).expect("the grandchild's pid");
    let _ = std::fs::remove_dir_all(&dir);
    result.expect_err("the hanging child was not timed out");
    let _ = still_live(&[&grandchild]);
    child
}

#[test]
fn qa_r217_no_process_of_the_group_remains_after_the_timeout() {
    let pgid = timed_out_group();
    let left = live_in_group(&pgid);
    // Survivors are killed before the check, so a failing run leaks nothing.
    reap_leftovers(&left.iter().map(String::as_str).collect::<Vec<_>>());
    check_group_empty(&pgid, &left);
}

/// The test process's own group, which is live.
#[cfg(feature = "controls")]
fn own_group() -> String {
    processes()
        .into_iter()
        .find(|(p, _, _)| *p == std::process::id().to_string())
        .map(|(_, g, _)| g)
        .expect("this process is listed")
}

negative_control!(
    qa_r217_no_process_of_the_group_remains_after_the_timeout,
    "this test process's own group, which has a live member, required to be empty",
    expected = "of the group",
    {
        let own = own_group();
        check_group_empty(&own, &live_in_group(&own))
    }
);

// ---------------------------------------------------------------------------------------------------------------------
// R-217: SIGTERM first; SIGKILL only after a 5 s grace.

/// The grace is R-217's 5 s.
fn check_grace_is(want: Duration) {
    assert_eq!(GRACE, want, "the helper's grace is not R-217's 5 s");
}

#[test]
fn qa_r217_the_grace_is_5_s() {
    check_grace_is(Duration::from_secs(5));
}

negative_control!(
    qa_r217_the_grace_is_5_s,
    "a grace of 4 s, required to be the helper's",
    expected = "the helper's grace is not R-217's 5 s",
    check_grace_is(Duration::from_secs(4))
);

/// A timed-out child whose SIGTERM handler waits `cleanup` s, then records it ran and exits: the handler ran, so the
/// group got SIGTERM and was let finish within the grace before any SIGKILL.
fn check_sigterm_then_grace(cleanup: &str) {
    let script =
        format!("trap 'sleep {cleanup}; echo term > \"$1/term\"; exit 0' TERM; sleep 37 & wait");
    let (result, _, dir) = run(&script, Duration::from_millis(500));
    let ran = std::fs::read_to_string(dir.join("term")).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        result.expect_err("the child was not timed out").kind(),
        ErrorKind::TimedOut
    );
    assert_eq!(
        ran, "term\n",
        "the child's SIGTERM handler did not run to its end before SIGKILL"
    );
}

#[test]
fn qa_r217_a_timed_out_child_gets_sigterm_and_the_grace() {
    check_sigterm_then_grace("2");
}

negative_control!(
    qa_r217_a_timed_out_child_gets_sigterm_and_the_grace,
    "a SIGTERM handler taking 8 s, past the grace, required to run to its end",
    expected = "the child's SIGTERM handler did not run to its end before SIGKILL",
    check_sigterm_then_grace("8")
);

/// A child and grandchild running `script` under a 1 s timeout: the helper returns no sooner than the timeout plus the
/// grace (no SIGKILL before the grace ran out) and no later than that plus slack, and both are gone.
fn check_killed_after_the_grace(script: &str) {
    let timeout = Duration::from_secs(1);
    let (result, took, dir) = run(script, timeout);
    let child = pid_in(&dir.join("child")).expect("the child's pid");
    let grandchild = pid_in(&dir.join("grandchild")).expect("the grandchild's pid");
    let _ = std::fs::remove_dir_all(&dir);
    let left = still_live(&[&grandchild]);
    reap_leftovers(&[&grandchild]);
    assert_eq!(
        result.expect_err("the child was not timed out").kind(),
        ErrorKind::TimedOut
    );
    assert!(
        took >= timeout + GRACE,
        "the group was ended after {took:?}, before the {timeout:?} timeout and the {GRACE:?} grace ran out"
    );
    assert!(
        took < timeout + GRACE + SLACK,
        "the helper waited {took:?}, past the timeout and the grace"
    );
    assert!(!exists(&child), "the timed-out child {child} still exists");
    assert!(
        left.is_empty(),
        "the grandchild {grandchild} survived SIGKILL"
    );
}

#[test]
fn qa_r217_a_group_ignoring_sigterm_is_killed_when_the_grace_runs_out() {
    check_killed_after_the_grace(&format!("trap '' TERM; {IN_GROUP}; wait"));
}

negative_control!(
    qa_r217_a_group_ignoring_sigterm_is_killed_when_the_grace_runs_out,
    "the same group without the trap, which SIGTERM ends at once, required to last the whole grace",
    expected = "before the 1s timeout and the 5s grace ran out",
    check_killed_after_the_grace(&format!("{IN_GROUP}; wait"))
);

// ---------------------------------------------------------------------------------------------------------------------
// R-217 "Nothing a child started may survive its timeout", also when the child exits and what it started holds its
// output open past the timeout.

fn check_holder_gone(start: &str) {
    let (result, _, dir) = run(&format!("{start}; echo x"), Duration::from_millis(500));
    let grandchild = pid_in(&dir.join("grandchild")).expect("the grandchild's pid");
    let _ = std::fs::remove_dir_all(&dir);
    let left = still_live(&[&grandchild]);
    reap_leftovers(&[&grandchild]);
    assert_eq!(
        result
            .expect_err("the held output was not timed out")
            .kind(),
        ErrorKind::TimedOut
    );
    assert!(
        left.is_empty(),
        "the grandchild {grandchild} holding the output survived the timeout"
    );
}

#[test]
fn qa_r217_a_grandchild_holding_the_output_is_gone_after_the_timeout() {
    check_holder_gone(IN_GROUP);
}

negative_control!(
    qa_r217_a_grandchild_holding_the_output_is_gone_after_the_timeout,
    "a holder that leaves the child's process group, beyond the helper's reach, required to be gone",
    expected = "holding the output survived the timeout",
    check_holder_gone(OUT_OF_GROUP)
);

// ---------------------------------------------------------------------------------------------------------------------
// R-214/R-217: on timeout the helper "fails naming it", whatever state the group's members are in. Here the only
// member left in the child's group when it is signalled is a zombie: the holder of the output forks a process that
// exits at once (and is never reaped), then leaves the group, keeping the output open past the timeout. (macOS refuses
// a signal to a group of zombies with EPERM; Linux delivers it.)

/// A holder that forks an unreaped zombie into the child's group, leaves the group and holds the output.
const ZOMBIE_IN_GROUP: &str = "perl -e 'my $z = fork; exit 0 if $z == 0; setpgrp(0, 0); \
     open my $f, \">\", $ARGV[0] or die; print $f $$; close $f; sleep 37' \"$1/holder\" & echo x";
/// A child that exits in time and closes its output.
#[cfg(feature = "controls")]
const IN_TIME: &str = "echo x";

fn check_fails_naming_the_child(script: &str) {
    let (result, _, dir) = run(script, Duration::from_millis(500));
    if let Some(holder) = pid_in(&dir.join("holder")) {
        reap_leftovers(&[&holder]);
    }
    let _ = std::fs::remove_dir_all(&dir);
    let named = matches!(
        &result,
        Err(e) if e.kind() == ErrorKind::TimedOut && e.to_string().contains("qa_r217_child")
    );
    assert!(
        named,
        "the helper did not fail with a timeout naming the child: {:?}",
        result.map(|o| o.status)
    );
}

#[test]
fn qa_r217_a_zombie_left_in_the_group_still_fails_naming_the_child() {
    check_fails_naming_the_child(ZOMBIE_IN_GROUP);
}

negative_control!(
    qa_r217_a_zombie_left_in_the_group_still_fails_naming_the_child,
    "a child that exits in time and closes its output, required to fail with a timeout",
    expected = "the helper did not fail with a timeout naming the child",
    check_fails_naming_the_child(IN_TIME)
);
