//! QA tests for TASK-M0-26's R-217 fix, written from the ruling and REQ-VAL-155, with no slack after the helper returns:
//! - R-217: "On timeout, kill the whole group: SIGTERM, then after a 5 s grace SIGKILL, and reap it. Nothing a child
//!   started may survive its timeout."
//! - REQ-VAL-155 verify: "a child that starts a grandchild and then hangs: after the timeout both are gone and no process
//!   from its group remains (R-217)".
//!
//! "After the timeout" is read as "when the helper returns its timeout error": the caller is then free to go on, so a
//! member still dying, or still unreaped, at that moment has outlived the timeout. The test is the one the kernel gives
//! a group: `kill(-pgid, 0)`, in process, at once. ESRCH (no member) or EPERM (macOS reports a group holding only
//! zombies so) is gone; success is a member that can still be signalled, which on Linux includes a zombie.
//!
//! Every path to the timeout is covered: a child that hangs past it, a child that ignores SIGTERM too (so SIGKILL ends
//! both), and a child that exits in time while a grandchild keeps its output open. Each runs [`RUNS`] times at once, so a
//! race between SIGKILL and the helper's return has room to show. Unix only: elsewhere there are no process groups.
//!
//! The negative control feeds the same check a group whose grandchild is deliberately left alive (R-176, R-199, R-212).
//! Children are spawned only through the helper.
#![cfg(unix)]

use std::io::{self, ErrorKind};
use std::process::{Command, Output};
use std::thread;
use std::time::Duration;

use rustix::io::Errno;
use rustix::process::{kill_process_group, test_kill_process_group, Pid, Signal};
use validation::negative_control;
use validation::spawn::Spawn;

/// How many helper calls each case runs at once.
const RUNS: usize = 16;

/// The timeout and grace the cases inject, in place of the calibrated ones (R-231).
const SHORT_WAIT: Duration = Duration::from_millis(300);
const SHORT_GRACE: Duration = Duration::from_millis(200);

/// The child hangs, its grandchild ignoring SIGTERM, so SIGKILL is what ends the grandchild.
const HANGS: &str = "echo $$; (trap '' TERM; sleep 37) & wait";
/// The child and its grandchild both ignore SIGTERM, so SIGKILL ends both.
const BOTH_DEAF: &str = "trap '' TERM; echo $$; sleep 37 & wait";
/// The child exits at once; its grandchild, ignoring SIGTERM, keeps the child's output open past the timeout.
const OUTPUT_HELD: &str = "echo $$; (trap '' TERM; sleep 37) &";
/// The child exits at once, its grandchild in its group with the output closed, so the helper returns in time and the
/// grandchild lives on: the group is not gone, whatever the helper does on a timeout.
#[cfg(feature = "controls")]
const LEFT_ALIVE: &str = "echo $$; (trap '' TERM; sleep 37) </dev/null >/dev/null 2>&1 &";

/// The process group the run's child led: named in a timeout's error as its pid, or printed first by the child.
fn group_of(result: &io::Result<Output>) -> Pid {
    let pid = match result {
        Err(err) => {
            let message = err.to_string();
            message
                .split("(pid ")
                .nth(1)
                .and_then(|rest| rest.split(')').next())
                .unwrap_or_else(|| panic!("the error names no pid: {message}"))
                .to_owned()
        }
        Ok(output) => String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .expect("the child printed its pid")
            .trim()
            .to_owned(),
    };
    Pid::from_raw(pid.parse().expect("a pid")).expect("a nonzero pid")
}

/// Runs `script` [`RUNS`] times at once through the helper. Each run's result is checked the moment the helper returns:
/// a timeout error of kind `TimedOut` that tells of no failure to end the group, and the child's group gone. Returns the
/// number of runs whose group was still there, after ending any survivor so a failing run leaks nothing.
fn runs_left(script: &str) -> usize {
    thread::scope(|scope| {
        let runs: Vec<_> = (0..RUNS)
            .map(|_| {
                scope.spawn(move || {
                    let result = Command::new("sh")
                        .args(["-c", script])
                        .output_within_grace(SHORT_WAIT, SHORT_GRACE);
                    // Checked first, before anything else can give the group time.
                    let group = group_of(&result);
                    let left = !matches!(
                        test_kill_process_group(group),
                        Err(Errno::SRCH | Errno::PERM)
                    );
                    let _ = kill_process_group(group, Signal::KILL);
                    if let Err(err) = &result {
                        assert_eq!(err.kind(), ErrorKind::TimedOut, "{err}");
                        assert!(
                            !err.to_string().contains("failed"),
                            "the helper did not end the group: {err}"
                        );
                    }
                    left
                })
            })
            .collect();
        runs.into_iter()
            .map(|run| run.join().expect("a run panicked"))
            .filter(|&left| left)
            .count()
    })
}

/// No run of `script` left anything of its child's group as the helper returned.
fn check_gone_at_return(script: &str) {
    let left = runs_left(script);
    assert_eq!(
        left, 0,
        "a member of the child's process group could still be signalled as the helper returned, in {left} of {RUNS} runs"
    );
}

#[test]
fn qa_m0_26_gone_at_return_when_the_child_hangs() {
    check_gone_at_return(HANGS);
}

negative_control!(
    qa_m0_26_gone_at_return_when_the_child_hangs,
    "a grandchild left alive in the group, required to be gone as the helper returns",
    expected =
        "a member of the child's process group could still be signalled as the helper returned",
    check_gone_at_return(LEFT_ALIVE)
);

#[test]
fn qa_m0_26_gone_at_return_when_child_and_grandchild_ignore_sigterm() {
    check_gone_at_return(BOTH_DEAF);
}

negative_control!(
    qa_m0_26_gone_at_return_when_child_and_grandchild_ignore_sigterm,
    "a grandchild left alive in the group, required to be gone as the helper returns",
    expected =
        "a member of the child's process group could still be signalled as the helper returned",
    check_gone_at_return(LEFT_ALIVE)
);

#[test]
fn qa_m0_26_gone_at_return_when_a_grandchild_holds_the_output() {
    check_gone_at_return(OUTPUT_HELD);
}

negative_control!(
    qa_m0_26_gone_at_return_when_a_grandchild_holds_the_output,
    "a grandchild left alive in the group, required to be gone as the helper returns",
    expected =
        "a member of the child's process group could still be signalled as the helper returned",
    check_gone_at_return(LEFT_ALIVE)
);
