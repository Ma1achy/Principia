//! The one way a test spawns a child process (R-214): `Command::output` bounded by a timeout, so a child that hangs
//! fails the test naming the child instead of stalling the suite. Every test that spawns a child uses it (REQ-VAL-155):
//!
//! ```text
//! use validation::spawn::Spawn;
//!
//! let output = Command::new("cargo").arg("--version").timed_output().expect("cargo ran");
//! ```

use std::io::{self, Read};
use std::process::{Command, Output, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

/// How long a child may run before it is killed: the calibration value of REQ-VAL-156 (R-214, R-71). 120 s is the
/// proposed value; it is provisional until the human confirms or changes it at the M0 gate (R-182).
pub const TIMEOUT: Duration = Duration::from_secs(120);

/// `Command::output`, bounded by a timeout (R-214).
pub trait Spawn {
    /// Runs the command as `Command::output` does (stdin closed, stdout and stderr captured) but waits at most
    /// `timeout` for it to exit and close its output. A child still running then is killed and reaped, and the error,
    /// of kind `TimedOut`, names it and its pid.
    fn output_within(&mut self, timeout: Duration) -> io::Result<Output>;

    /// [`Spawn::output_within`] the provisional [`TIMEOUT`].
    fn timed_output(&mut self) -> io::Result<Output> {
        self.output_within(TIMEOUT)
    }
}

impl Spawn for Command {
    fn output_within(&mut self, timeout: Duration) -> io::Result<Output> {
        let deadline = Instant::now() + timeout;
        let mut child = self
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        // Both pipes are drained while the child runs, so a child filling one never blocks on it.
        let (tx, rx) = mpsc::channel();
        drain(child.stdout.take(), 0, tx.clone());
        drain(child.stderr.take(), 1, tx);
        let timed_out = |pid: u32, what: &str| {
            io::Error::new(
                io::ErrorKind::TimedOut,
                format!(
                    "child `{}` (pid {pid}) {what} the {} s timeout (R-214; the 120 s default is provisional, \
                     REQ-VAL-156)",
                    name(self),
                    timeout.as_secs_f64()
                ),
            )
        };
        let status = loop {
            if let Some(status) = child.try_wait()? {
                break status;
            }
            if Instant::now() >= deadline {
                child.kill()?;
                child.wait()?;
                return Err(timed_out(child.id(), "was killed: it outlived"));
            }
            thread::sleep(Duration::from_millis(10));
        };
        // The child has exited; a grandchild may still hold its pipes open, so reading them is bounded too.
        let mut streams = [Vec::new(), Vec::new()];
        for _ in 0..2 {
            let wait = deadline.saturating_duration_since(Instant::now());
            let (stream, bytes) = rx
                .recv_timeout(wait)
                .map_err(|_| timed_out(child.id(), "exited, but its output stayed open past"))?;
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
