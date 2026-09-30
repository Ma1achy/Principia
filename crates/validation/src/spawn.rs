//! The one way a test spawns a child process (R-214): `Command::output` bounded by a timeout, so a child that hangs
//! fails the test naming the child instead of stalling the suite. Every test that spawns a child uses it (REQ-VAL-155):
//!
//! ```text
//! use validation::spawn::Spawn;
//!
//! let output = Command::new("cargo").arg("--version").timed_output().expect("cargo ran");
//! ```
//!
//! On Unix every child runs in a process group of its own, and a timeout ends the whole group: SIGTERM, then after
//! [`GRACE`] SIGKILL, and the child is reaped, so nothing the child started survives its timeout (R-217). Elsewhere
//! there are no process groups, and only the child itself is killed.

use std::io::{self, Read};
use std::process::{Child, Command, Output, Stdio};
use std::sync::{mpsc, PoisonError, RwLock};
use std::thread;
use std::time::{Duration, Instant};

/// How long a child may run before it is killed: the calibration value of REQ-VAL-156 (R-214, R-71). 300 s is the
/// provisional value, covering cold builds with margin (R-217); the human confirms or changes it at the M0 gate, from
/// cold and warm measurements on CI and on their Mac (R-182, R-217).
pub const SPAWN_TIMEOUT: Duration = Duration::from_secs(300);

/// How long a timed-out child's process group has, after SIGTERM, to exit before it is sent SIGKILL (R-217).
pub const GRACE: Duration = Duration::from_secs(5);

/// `Command::output`, bounded by a timeout (R-214).
pub trait Spawn {
    /// Runs the command as `Command::output` does (stdin closed, stdout and stderr captured) but waits at most
    /// `timeout` for it to exit and close its output. A child still running then is ended with everything it started
    /// (see the module docs) and reaped, and the error, of kind `TimedOut`, names it and its pid.
    fn output_within(&mut self, timeout: Duration) -> io::Result<Output>;

    /// [`Spawn::output_within`] the provisional [`SPAWN_TIMEOUT`].
    fn timed_output(&mut self) -> io::Result<Output> {
        self.output_within(SPAWN_TIMEOUT)
    }
}

impl Spawn for Command {
    fn output_within(&mut self, timeout: Duration) -> io::Result<Output> {
        let deadline = Instant::now() + timeout;
        // The child leads a new process group, which everything it starts joins unless it leaves it (R-217).
        #[cfg(unix)]
        std::os::unix::process::CommandExt::process_group(self, 0);
        // No stand-in executable is being written while the child is forked and exec'd (REQ-SYS-070).
        let spawning = WRITING.read().unwrap_or_else(PoisonError::into_inner);
        let mut child = self
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        drop(spawning);
        // Both pipes are drained while the child runs, so a child filling one never blocks on it.
        let (tx, rx) = mpsc::channel();
        drain(child.stdout.take(), 0, tx.clone());
        drain(child.stderr.take(), 1, tx);
        // Whatever went wrong in ending the child is told in the timeout's error, never in place of it (REQ-VAL-155).
        let timed_out = |pid: u32, what: &str, ended: io::Result<()>| {
            let mut message = format!(
                "child `{}` (pid {pid}) {what} the {} s timeout (R-214; the {} s default is provisional, \
                 REQ-VAL-156)",
                name(self),
                timeout.as_secs_f64(),
                SPAWN_TIMEOUT.as_secs_f64()
            );
            if let Err(e) = ended {
                message.push_str(&format!("; {ENDING}: {e}"));
            }
            io::Error::new(io::ErrorKind::TimedOut, message)
        };
        let status = loop {
            if let Some(status) = child.try_wait()? {
                break status;
            }
            if Instant::now() >= deadline {
                let ended = end(&mut child);
                return Err(timed_out(child.id(), "was killed: it outlived", ended));
            }
            thread::sleep(Duration::from_millis(10));
        };
        // The child has exited; a grandchild may still hold its pipes open, so reading them is bounded too.
        let mut streams = [Vec::new(), Vec::new()];
        for _ in 0..2 {
            let wait = deadline.saturating_duration_since(Instant::now());
            let Ok((stream, bytes)) = rx.recv_timeout(wait) else {
                // What holds the output open is something the child started: it is ended too (R-217).
                let ended = end(&mut child);
                return Err(timed_out(
                    child.id(),
                    "exited, but its output stayed open past",
                    ended,
                ));
            };
            streams[stream] = bytes?;
        }
        let [stdout, stderr] = streams;
        Ok(Output {
            status,
            stdout,
            stderr,
        })
    }
}

/// What a timeout's error says before a failure to end the child: a signal its group refused, or a failure to reap it.
const ENDING: &str = "ending it and its process group failed";

/// Ends a timed-out child and everything in its process group (R-217): SIGTERM to the group, then, unless the child
/// has been reaped and nothing in the group can still be signalled within [`GRACE`], SIGKILL to the group. The child
/// itself is then killed, in case it has left its group, and reaped, whatever the signals to the group met. A signal
/// the group refused is returned as the error, once the child is reaped.
#[cfg(unix)]
fn end(child: &mut Child) -> io::Result<()> {
    use rustix::io::Errno;
    use rustix::process::{kill_process_group, test_kill_process_group, Pid, Signal};
    // The group's id is the child's pid, and stays taken while the child is unreaped or any member lives.
    let group = Pid::from_child(child);
    // A group with no member left has nothing to signal (ESRCH). Any other failure is kept, to be reported. It is
    // EPERM when no member may be signalled: one not ours, or, on macOS, a group of only zombies, which is refused.
    let refused = |signal| match kill_process_group(group, signal) {
        Err(e) if e != Errno::SRCH => Some(e),
        _ => None,
    };
    let mut refusal = refused(Signal::TERM);
    // After a refused SIGTERM no member can be signalled, and SIGKILL would be refused too.
    if refusal.is_none() {
        refusal = 'grace: {
            let grace_ends = Instant::now() + GRACE;
            while Instant::now() < grace_ends {
                // Nothing is left for SIGKILL once the child is reaped and the group is empty or refuses signals.
                if child.try_wait()?.is_some()
                    && matches!(
                        test_kill_process_group(group),
                        Err(Errno::SRCH | Errno::PERM)
                    )
                {
                    break 'grace None;
                }
                thread::sleep(Duration::from_millis(10));
            }
            refused(Signal::KILL)
        };
    }
    // The child is ours to kill (a no-op once it has exited) and to reap, so the wait is bounded.
    child.kill()?;
    child.wait()?;
    refusal.map_or(Ok(()), |e| Err(io::Error::from(e)))
}

/// Ends a timed-out child. Without process groups, only the child itself is killed and reaped.
#[cfg(not(unix))]
fn end(child: &mut Child) -> io::Result<()> {
    child.kill()?;
    child.wait()?;
    Ok(())
}

/// Reads `pipe` to its end on a thread of its own, and sends the bytes as stream `index`.
fn drain<R: Read + Send + 'static>(
    pipe: Option<R>,
    index: usize,
    tx: mpsc::Sender<(usize, io::Result<Vec<u8>>)>,
) {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let read = pipe.map_or(Ok(0), |mut p| p.read_to_end(&mut bytes));
        let _ = tx.send((index, read.map(|_| bytes)));
    });
}

/// The program and its arguments, as the error names the child.
fn name(command: &Command) -> String {
    std::iter::once(command.get_program())
        .chain(command.get_args())
        .map(|part| part.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Held to write while a stand-in executable is written, and to read by each spawn, from its fork to its exec
/// (REQ-SYS-070). On Linux a child forked while another thread has an executable open for writing inherits that
/// descriptor until it execs, and an exec of that executable in the meantime fails with ETXTBSY ("Text file busy").
/// Rust opens every file close-on-exec, so once `spawn` returns the child holds none of them.
static WRITING: RwLock<()> = RwLock::new(());

/// Runs `write` while no child is being spawned through [`Spawn`], and none starts until it returns (REQ-SYS-070).
/// `write` itself must not spawn through [`Spawn`], which would wait for it forever.
pub fn while_no_spawn<T>(write: impl FnOnce() -> T) -> T {
    let _writing = WRITING.write().unwrap_or_else(PoisonError::into_inner);
    write()
}

/// Writes `contents` to `path` as an executable (mode 0755 on Unix), with no child spawned while the file is open
/// for writing (REQ-SYS-070). Every test that writes a stand-in executable writes it with this.
pub fn write_executable(path: &std::path::Path, contents: impl AsRef<[u8]>) -> io::Result<()> {
    while_no_spawn(|| {
        std::fs::write(path, contents)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A child running `sleep 30` under `timeout` is killed: the error names it and its pid, it returns long before
    /// the child would have exited, and the pid no longer names a process.
    fn check_killed(timeout: Duration, seconds: &str) {
        let started = Instant::now();
        let err = Command::new("sleep")
            .arg(seconds)
            .output_within(timeout)
            .expect_err("the child was not timed out");
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "the helper waited for the child"
        );
        assert_eq!(err.kind(), io::ErrorKind::TimedOut, "{err}");
        let message = err.to_string();
        assert!(
            message.contains(&format!("child `sleep {seconds}` (pid ")),
            "{message}"
        );
        assert!(!message.contains(ENDING), "{message}");
        let pid = message
            .split("(pid ")
            .nth(1)
            .and_then(|rest| rest.split(')').next())
            .expect("the error names the pid");
        let alive = Command::new("kill")
            .args(["-0", pid])
            .timed_output()
            .expect("kill ran");
        assert!(
            !alive.status.success(),
            "the killed child {pid} still exists"
        );
    }

    #[test]
    fn spawn_kills_a_child_that_outlives_the_timeout() {
        check_killed(Duration::from_millis(200), "30");
    }

    crate::negative_control!(
        spawn_kills_a_child_that_outlives_the_timeout,
        "a child that exits well within the timeout, required to be killed",
        expected = "the child was not timed out",
        check_killed(Duration::from_secs(5), "0")
    );

    /// A child that exits within `timeout` returns its stdout, its stderr and its exit status.
    fn check_in_time(timeout: Duration, script: &str) {
        let output = Command::new("sh")
            .args(["-c", script])
            .output_within(timeout)
            .unwrap_or_else(|e| panic!("the child did not return its output: {e}"));
        assert_eq!(output.stdout, b"out\n");
        assert_eq!(output.stderr, b"err\n");
        assert_eq!(output.status.code(), Some(3));
    }

    #[test]
    fn spawn_returns_the_output_of_a_child_in_time() {
        check_in_time(Duration::from_secs(10), "echo out; echo err >&2; exit 3");
    }

    /// Whether `kill -0 <target>` finds a process: a pid, or `-<pgid>` for any member of a process group.
    fn exists(target: &str) -> bool {
        Command::new("kill")
            .args(["-0", "--", target])
            .timed_output()
            .expect("kill ran")
            .status
            .success()
    }

    /// A fresh scratch directory; one per call, as a test and its control may run at once.
    fn scratch() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static CALLS: AtomicUsize = AtomicUsize::new(0);
        let call = CALLS.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("spawn-{}-{call}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    /// The pid in a timed-out child's error.
    fn pid_in(err: &io::Error) -> String {
        err.to_string()
            .split("(pid ")
            .nth(1)
            .and_then(|rest| rest.split(')').next())
            .expect("the error names the pid")
            .to_owned()
    }

    /// A child that starts a grandchild (by running `start`, which backgrounds it and writes its pid to `$1`) and then
    /// hangs is timed out. After the timeout the child and the grandchild are gone, and no process of the child's group
    /// remains (R-217).
    fn check_group_ended(start: &str) {
        let dir = scratch();
        let pid_file = dir.join("grandchild");
        let err = Command::new("sh")
            .args(["-c", &format!("{start}; sleep 30"), "sh"])
            .arg(&pid_file)
            .output_within(Duration::from_millis(500))
            .expect_err("the child was not timed out");
        let child = pid_in(&err);
        let grandchild = std::fs::read_to_string(&pid_file).expect("the grandchild's pid");
        let grandchild = grandchild.trim();
        let (child_left, grandchild_left, group_left) = (
            exists(&child),
            exists(grandchild),
            exists(&format!("-{child}")),
        );
        // A survivor is ended here, so a failing run leaks nothing.
        let _ = Command::new("kill").args(["-9", grandchild]).timed_output();
        let _ = std::fs::remove_dir_all(&dir);
        assert!(!child_left, "the timed-out child {child} still exists");
        assert!(
            !grandchild_left,
            "the grandchild {grandchild} of the timed-out child survived its timeout"
        );
        assert!(!group_left, "a process of the group {child} remains");
    }

    #[test]
    fn spawn_ends_the_group_of_a_child_that_outlives_the_timeout() {
        check_group_ended("sleep 30 & echo $! > \"$1\"");
    }

    crate::negative_control!(
        spawn_ends_the_group_of_a_child_that_outlives_the_timeout,
        "a grandchild that leaves the child's process group, required to be gone after the timeout",
        expected = "of the timed-out child survived its timeout",
        check_group_ended("perl -e 'setpgrp(0, 0); exec @ARGV' sleep 30 & echo $! > \"$1\"")
    );

    /// A grandchild that ignores SIGTERM, while the child dies of it, is killed after the grace.
    #[test]
    fn spawn_ends_a_grandchild_that_ignores_sigterm() {
        check_group_ended("(trap '' TERM; sleep 30) & echo $! > \"$1\"");
    }

    crate::negative_control!(
        spawn_ends_a_grandchild_that_ignores_sigterm,
        "the same grandchild, leaving the child's process group, required to be gone after the timeout",
        expected = "of the timed-out child survived its timeout",
        check_group_ended(
            "(trap '' TERM; exec perl -e 'setpgrp(0, 0); exec @ARGV' sleep 30) & echo $! > \"$1\""
        )
    );

    /// A timed-out child whose SIGTERM handler takes `cleanup` seconds, then writes a file and exits, is let finish it
    /// within the grace before SIGKILL.
    fn check_grace(cleanup: &str) {
        let dir = scratch();
        let done = dir.join("done");
        let script =
            format!("trap 'sleep {cleanup}; echo done > \"$1\"; exit 0' TERM; sleep 30 & wait");
        let err = Command::new("sh")
            .args(["-c", &script, "sh"])
            .arg(&done)
            .output_within(Duration::from_millis(500))
            .expect_err("the child was not timed out");
        let finished = std::fs::read_to_string(&done).unwrap_or_default();
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(err.kind(), io::ErrorKind::TimedOut, "{err}");
        assert_eq!(
            finished, "done\n",
            "the child's SIGTERM handler was not let finish within the grace"
        );
    }

    #[test]
    fn spawn_gives_a_timed_out_child_its_grace_after_sigterm() {
        check_grace("1");
    }

    crate::negative_control!(
        spawn_gives_a_timed_out_child_its_grace_after_sigterm,
        "a SIGTERM handler that takes 7 s, longer than the grace, required to finish",
        expected = "the child's SIGTERM handler was not let finish within the grace",
        check_grace("7")
    );

    /// `program` runs `sh -c`'s arguments; the child exits at once while a process outside its group, whose pid it
    /// writes to `$1`, holds its output open. The helper still fails with a timeout, although the child's group is
    /// empty when it is signalled.
    fn check_timed_out(program: &str) {
        let dir = scratch();
        let pid_file = dir.join("holder");
        let script = "perl -e 'setpgrp(0, 0); exec @ARGV' sleep 30 & echo $! > \"$1\"; echo x";
        let result = Command::new(program)
            .args(["-c", script, "sh"])
            .arg(&pid_file)
            .output_within(Duration::from_millis(500));
        if let Ok(holder) = std::fs::read_to_string(&pid_file) {
            let _ = Command::new("kill")
                .args(["-9", holder.trim()])
                .timed_output();
        }
        let _ = std::fs::remove_dir_all(&dir);
        let err = result.expect_err("the helper returned the output");
        assert_eq!(
            err.kind(),
            io::ErrorKind::TimedOut,
            "the helper's error is not a timeout: {err}"
        );
        assert!(
            !err.to_string().contains(ENDING),
            "an empty group was reported as refusing its signal: {err}"
        );
    }

    #[test]
    fn spawn_times_out_a_child_whose_output_is_held_outside_its_group() {
        check_timed_out("sh");
    }

    crate::negative_control!(
        spawn_times_out_a_child_whose_output_is_held_outside_its_group,
        "a program that does not exist, which fails to spawn, required to time out",
        expected = "the helper's error is not a timeout",
        check_timed_out("/nonexistent/spawn-control")
    );

    /// A holder that forks a process which exits at once and is never reaped, then leaves the child's process group
    /// and holds the child's output open, writing its pid to `$1`. The zombie stays in the group while the holder lives.
    #[cfg(target_os = "macos")]
    const ZOMBIE_HOLDER: &str = "perl -e 'exit 0 if fork == 0; setpgrp(0, 0); \
        open my $f, \">\", $ARGV[0] or die; print $f $$; close $f; sleep 30' \"$1\" &";
    /// The same holder, without the zombie.
    #[cfg(all(target_os = "macos", feature = "controls"))]
    const HOLDER: &str = "perl -e 'setpgrp(0, 0); \
        open my $f, \">\", $ARGV[0] or die; print $f $$; close $f; sleep 30' \"$1\" &";

    /// Runs `sh -c <script>` under a 500 ms timeout, with `$1` a file the script writes a holder's pid to, which is
    /// killed afterwards. Returns the helper's error and how long the helper took.
    #[cfg(target_os = "macos")]
    fn time_out(script: &str) -> (io::Error, Duration) {
        let dir = scratch();
        let pid_file = dir.join("holder");
        let started = Instant::now();
        let result = Command::new("sh")
            .args(["-c", script, "sh"])
            .arg(&pid_file)
            .output_within(Duration::from_millis(500));
        let took = started.elapsed();
        if let Ok(holder) = std::fs::read_to_string(&pid_file) {
            let _ = Command::new("kill")
                .args(["-9", holder.trim()])
                .timed_output();
        }
        let _ = std::fs::remove_dir_all(&dir);
        (result.expect_err("the helper returned the output"), took)
    }

    /// macOS refuses a signal to a group whose only members are zombies (EPERM). A timed-out child whose group is only
    /// a zombie when it is signalled still fails with the timeout naming the child, reaped, and the refusal is told.
    #[cfg(target_os = "macos")]
    fn check_refusal_told(holder: &str) {
        let (err, _) = time_out(&format!("{holder} echo x"));
        assert_eq!(err.kind(), io::ErrorKind::TimedOut, "{err}");
        let message = err.to_string();
        assert!(message.contains("child `sh -c "), "{message}");
        assert!(
            !exists(&pid_in(&err)),
            "the child was not reaped: {message}"
        );
        assert!(
            message.contains(&format!("{ENDING}: Operation not permitted")),
            "the group's refusal of the signal was not told: {message}"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn spawn_tells_a_timed_out_group_of_zombies_refused_the_signal() {
        check_refusal_told(ZOMBIE_HOLDER);
    }

    #[cfg(target_os = "macos")]
    crate::negative_control!(
        spawn_tells_a_timed_out_group_of_zombies_refused_the_signal,
        "the same child without the zombie, whose group is empty and refuses nothing, required to tell a refusal",
        expected = "the group's refusal of the signal was not told",
        check_refusal_told(HOLDER)
    );

    /// A hanging child whose group, once SIGTERM has ended the child, holds only a zombie is not kept for the grace:
    /// nothing is left that SIGKILL could reach (macOS refuses it), and nothing is reported as refused.
    #[cfg(target_os = "macos")]
    fn check_no_grace_for_zombies(script: &str) {
        let (err, took) = time_out(script);
        assert_eq!(err.kind(), io::ErrorKind::TimedOut, "{err}");
        assert!(!err.to_string().contains(ENDING), "{err}");
        assert!(
            took < Duration::from_millis(500) + GRACE,
            "the helper waited out the grace for a group of zombies: {took:?}"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn spawn_does_not_keep_a_group_of_zombies_for_the_grace() {
        check_no_grace_for_zombies(&format!("{ZOMBIE_HOLDER} sleep 30"));
    }

    #[cfg(target_os = "macos")]
    crate::negative_control!(
        spawn_does_not_keep_a_group_of_zombies_for_the_grace,
        "the same group with a member that ignores SIGTERM, required to end before the grace",
        expected = "the helper waited out the grace for a group of zombies",
        check_no_grace_for_zombies(&format!("{ZOMBIE_HOLDER} (trap '' TERM; sleep 30) & wait"))
    );

    crate::negative_control!(
        spawn_returns_the_output_of_a_child_in_time,
        "the same child sleeping past a short timeout first, required to return its output",
        expected = "the child did not return its output",
        check_in_time(
            Duration::from_millis(200),
            "sleep 30; echo out; echo err >&2; exit 3"
        )
    );
}
