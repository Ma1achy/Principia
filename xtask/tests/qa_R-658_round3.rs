//! qa's tests for R-658's failed saves, written from `plan/OPERATIONS.md` § "Resources", "A failed save in a live
//! runner", not from the implementation:
//! - "A pass with a failed save starts no job";
//! - "and counts as idle, so a runner that can't write exits after `idle` seconds, and the next runner ends and queues
//!   again each job still marked running";
//! - a failed save "starts nothing and runs nothing the file doesn't record", the job staying queued, with the runner's
//!   exit leaving no copy alive;
//! - a running job past its timeout is still ended, SIGKILL included, while its saves fail.
//!
//! The failing save is a folder planted at the temporary name beside the job's file, the only name the runner's pid
//! blocks, so clients still write.
//!
//! Safety, as in `xtask/tests/qa_R-658.rs`: a queue of each test's own in a scratch folder; every `xtask job` call under
//! `ulimit -u` and 45 s at most; the commands are `sh` and a stand-in for `sleep`; every long-lived process names the
//! queue's folder, and the queue's guard ends, by pid, every process naming it when the test ends.
#![cfg(unix)]

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

use scratch::Scratch;
use validation::negative_control;
use validation::spawn::Spawn;
use xtask::jobq::{Queue, State};

/// Loops until `go` exists in the job's folder, 40 s at most.
const GATE: &str = "i=0; while [ ! -e go ] && [ $i -lt 800 ]; do sleep 0.05; i=$((i+1)); done";

struct Q {
    dir: Scratch,
    sleeper: PathBuf,
}

impl Q {
    fn new(tag: &str, config: &str) -> Self {
        let dir = Scratch::new(&format!("qajobq_r3_{tag}"));
        std::fs::create_dir_all(dir.join("work")).expect("scratch made");
        std::fs::write(
            dir.join("config"),
            format!("tick_ms = 50\nsample = 1\ngrace = 1\nstale = 3\n{config}"),
        )
        .expect("config written");
        let sleeper = dir.join("qa-sleep");
        std::os::unix::fs::symlink("/bin/sleep", &sleeper).expect("stand-in linked");
        Q { dir, sleeper }
    }

    fn work(&self) -> PathBuf {
        self.dir.join("work")
    }

    fn queue(&self) -> Queue {
        Queue::at(self.dir.to_path_buf())
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new("/bin/bash");
        command
            .args(["-c", "ulimit -u \"$0\" && exec \"$@\""])
            .arg(process_cap().to_string())
            .arg(env!("CARGO_BIN_EXE_xtask"))
            .arg("job")
            .args(args)
            .env("PRINCIPIA_JOBQ", &*self.dir);
        command
    }

    /// Submits a shell script, given the stand-in for `sleep` as `$0`, on its own target directory.
    fn submit(&self, target: &str, timeout: u64, script: &str) -> u64 {
        let work = self.work();
        let env = format!("CARGO_TARGET_DIR=/nonexistent/qa-r3-{target}");
        let timeout = timeout.to_string();
        let args = [
            "submit",
            work.to_str().expect("utf-8 path"),
            "--priority",
            "peek",
            "--label",
            "qa",
            "--timeout",
            &timeout,
            "--env",
            &env,
            "--",
            "/bin/sh",
            "-c",
            script,
            self.sleeper.to_str().expect("utf-8 path"),
        ];
        let out = self
            .command(&args)
            .output_within(Duration::from_secs(45))
            .expect("xtask job ran");
        assert!(
            out.status.success(),
            "submit failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout)
            .trim()
            .parse()
            .expect("submit printed an id")
    }

    fn wait(&self, id: u64) -> (i32, Output) {
        let out = self
            .command(&["wait", &id.to_string()])
            .output_within(Duration::from_secs(45))
            .expect("xtask job ran");
        (out.status.code().unwrap_or(-1), out)
    }

    fn state(&self, id: u64) -> State {
        self.queue().load(id).expect("job file").state
    }

    fn attempts(&self, id: u64) -> u32 {
        self.queue().load(id).expect("job file").attempts
    }

    /// Waits until `id`'s leader start is recorded, the runner's last step in starting it.
    fn started(&self, id: u64) {
        assert!(
            until(20, || self
                .queue()
                .load(id)
                .is_ok_and(|job| job.leader_start.is_some())),
            "job {id} never started"
        );
    }

    /// Makes every save of `id` by the current runner fail, returning the folder to remove to stop that.
    fn plant(&self, id: u64) -> PathBuf {
        let runner = self.queue().runner_lock().expect("a live runner").pid;
        let planted = self
            .queue()
            .job_path(id)
            .with_extension(format!("tmp{runner}"));
        std::fs::create_dir_all(&planted).expect("planted");
        planted
    }

    fn open_gate(&self) {
        std::fs::write(self.work().join("go"), "").expect("gate opened");
    }

    /// Waits for the runner to exit, so nothing writes into the scratch as it is removed.
    fn finish(&self) {
        let ends = Instant::now() + Duration::from_secs(30);
        while self.dir.join("runner.lock").exists() && Instant::now() < ends {
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}

impl Drop for Q {
    /// Ends, by pid, every process still naming this queue's folder.
    fn drop(&mut self) {
        let dir = self.dir.to_string_lossy().into_owned();
        let Ok(out) = Command::new("ps")
            .args(["-axo", "pid=,args="])
            .output_within(Duration::from_secs(30))
        else {
            return;
        };
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            let Some((pid, args)) = line.trim_start().split_once(' ') else {
                continue;
            };
            if args.contains(&dir) && !args.starts_with("ps ") {
                kill(pid);
            }
        }
    }
}

fn kill(pid: &str) {
    let _ = Command::new("kill")
        .args(["-9", pid])
        .output_within(Duration::from_secs(30));
}

/// `ulimit -u` for every xtask call: this user's processes when the first test started, plus 300.
fn process_cap() -> usize {
    static CAP: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *CAP.get_or_init(|| {
        let out = Command::new("/bin/sh")
            .args(["-c", "ps -U \"$(id -u)\" -o pid= | wc -l"])
            .output_within(Duration::from_secs(30))
            .expect("ps ran");
        String::from_utf8_lossy(&out.stdout)
            .trim()
            .parse::<usize>()
            .expect("a count")
            + 300
    })
}

fn alive(pid: i32) -> bool {
    let Ok(out) = Command::new("ps")
        .args(["-o", "stat=", "-p", &pid.to_string()])
        .output_within(Duration::from_secs(30))
    else {
        return false;
    };
    let stat = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    !stat.is_empty() && !stat.starts_with('Z')
}

fn until(secs: u64, mut ready: impl FnMut() -> bool) -> bool {
    let ends = Instant::now() + Duration::from_secs(secs);
    while Instant::now() < ends {
        if ready() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    ready()
}

/// The pids in `path`, one a line.
fn pids(path: &Path) -> Vec<i32> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| line.trim().parse().ok())
        .collect()
}

fn alive_of(path: &Path) -> Vec<i32> {
    pids(path).into_iter().filter(|&pid| alive(pid)).collect()
}

// --- A pass with a failed save starts no job --------------------------------------------------------------------------

/// A job's end can't be saved, and a slot is free; a second job, submitted then, must not start until the save works.
/// With `fail` off, the control, it starts at once, and the check sees that.
fn starts_no_job_case(tag: &str, fail: bool) {
    let q = Q::new(tag, "concurrency = 3\nidle = 10\n");
    let ends = q.submit("a", 30, &format!("{GATE}; exit 7"));
    q.started(ends);
    let planted = fail.then(|| q.plant(ends));
    q.open_gate();
    // The end has been seen, and its save tried, before the second job arrives.
    std::thread::sleep(Duration::from_millis(1000));
    let second = q.submit("b", 30, "echo ran > second; exit 0");
    let ran = q.work().join("second");
    let started_during = until(3, || ran.exists());
    let state_during = q.state(second);
    if let Some(planted) = &planted {
        std::fs::remove_dir(planted).expect("plant removed");
    }
    let (first_code, second_code) = (q.wait(ends).0, q.wait(second).0);
    q.finish();
    assert!(
        !started_during
            && state_during == State::Queued
            && first_code == 7
            && second_code == 0
            && ran.exists(),
        "a pass with a failed save started a job: started {started_during}, state {state_during:?}, statuses \
         {first_code} and {second_code}"
    );
}

#[test]
fn qa_r658_a_pass_with_a_failed_save_starts_no_job() {
    starts_no_job_case("starts_none", true);
}

negative_control!(
    qa_r658_a_pass_with_a_failed_save_starts_no_job,
    "with saves working the second job starts at once, and the check sees it",
    expected = "a pass with a failed save started a job",
    starts_no_job_case("starts_none_control", false)
);

// --- A runner that can't write exits, and the next one recovers ---------------------------------------------------------

/// One job's end can't be saved, and a healthy job runs beside it. The runner must exit after `idle` seconds, however
/// busy, and the next runner must end the healthy job's group and queue it again, and run the first again, both from
/// what their files say. With `fail` off, the control, the runner stays and neither job starts twice.
fn cant_write_case(tag: &str, fail: bool) {
    let q = Q::new(tag, "concurrency = 2\nidle = 2\n");
    let ends = q.submit("a", 40, &format!("{GATE}; exit 7"));
    let beside = q.submit("b", 40, "echo $$ >> beside; exec \"$0\" 35");
    q.started(ends);
    q.started(beside);
    let list = q.work().join("beside");
    assert!(until(10, || !pids(&list).is_empty()), "no pid from the job");
    let first_copy = pids(&list)[0];
    let runner = q.queue().runner_lock().expect("a live runner").pid;
    let planted = fail.then(|| q.plant(ends));
    q.open_gate();
    let runner_exited = until(8, || !alive(runner));
    let marked_running = q.state(ends) == State::Running;
    if let Some(planted) = &planted {
        std::fs::remove_dir(planted).expect("plant removed");
    }
    // `wait` starts the next runner where none is alive.
    let code = q.wait(ends).0;
    let copy_ended = until(10, || !alive(first_copy));
    let again = until(10, || {
        q.state(beside) == State::Running && alive_of(&list).len() == 1
    });
    let (first, second) = (q.attempts(ends), q.attempts(beside));
    let cancelled = q.run_cancel(beside);
    let status = q.wait(beside).0;
    let left = alive_of(&list);
    q.finish();
    assert!(
        runner_exited
            && marked_running
            && code == 7
            && copy_ended
            && again
            && first == 2
            && second == 2
            && cancelled
            && status == 130
            && left.is_empty(),
        "a runner that can't write was not replaced as stated: exited {runner_exited}, marked running \
         {marked_running}, status {code}, first copy ended {copy_ended}, running again {again}, starts {first} and \
         {second}, cancelled {cancelled} with {status}, left {left:?}"
    );
}

impl Q {
    fn run_cancel(&self, id: u64) -> bool {
        self.command(&["cancel", &id.to_string()])
            .output_within(Duration::from_secs(45))
            .expect("xtask job ran")
            .status
            .success()
    }
}

#[test]
fn qa_r658_a_runner_that_cannot_write_exits_and_the_next_recovers() {
    cant_write_case("cant_write", true);
}

negative_control!(
    qa_r658_a_runner_that_cannot_write_exits_and_the_next_recovers,
    "with saves working the runner stays and neither job starts twice, and the check sees it",
    expected = "a runner that can't write was not replaced as stated",
    cant_write_case("cant_write_control", false)
);

// --- A start that can't be saved ----------------------------------------------------------------------------------------

/// A job's every start can't be saved, for as long as the runner lives. The runner must exit, leaving no copy alive and
/// the job queued with no start counted, and the next runner must run it once. With `fail` off, the control, the first
/// runner runs the job and counts the start.
fn start_never_saved_case(tag: &str, fail: bool) {
    let q = Q::new(tag, "concurrency = 2\nidle = 2\n");
    let gate = q.submit("a", 30, GATE);
    q.started(gate);
    let runner = q.queue().runner_lock().expect("a live runner").pid;
    // On the gate's target directory, so it waits for the gate with a slot free.
    let id = q.submit("a", 30, "echo $$ >> copies; exec \"$0\" 30");
    let planted = fail.then(|| q.plant(id));
    q.open_gate();
    let list = q.work().join("copies");
    let (mut most, stop) = (0, Instant::now() + Duration::from_secs(1));
    while Instant::now() < stop {
        most = most.max(alive_of(&list).len());
        std::thread::sleep(Duration::from_millis(20));
    }
    let runner_exited = until(8, || !alive(runner));
    let left_after_exit = alive_of(&list);
    let during = q.queue().load(id).expect("job file");
    if let Some(planted) = &planted {
        std::fs::remove_dir(planted).expect("plant removed");
    }
    // Waiting on another job starts the next runner, once `submit` and `wait`'s bound of one runner per 5 s allows.
    let nudge = q.submit("c", 30, "exit 0");
    let nudge_status = q.wait(nudge).0;
    let runs = until(10, || {
        q.state(id) == State::Running && alive_of(&list).len() == 1
    });
    let starts = q.attempts(id);
    let cancelled = q.run_cancel(id);
    let status = q.wait(id).0;
    let left = alive_of(&list);
    q.finish();
    assert!(
        most <= 1
            && runner_exited
            && left_after_exit.is_empty()
            && during.state == State::Queued
            && during.attempts == 0
            && runs
            && starts == 1
            && cancelled
            && status == 130
            && nudge_status == 0
            && left.is_empty(),
        "a start that can't be saved left what its file doesn't record: most copies {most}, runner exited \
         {runner_exited}, left after it {left_after_exit:?}, during {:?} with {} starts, runs {runs}, starts {starts}, \
         cancelled {cancelled} with {status}, left {left:?}",
        during.state,
        during.attempts
    );
}

#[test]
fn qa_r658_a_start_never_saved_leaves_nothing_and_runs_once_later() {
    start_never_saved_case("never_saved", true);
}

negative_control!(
    qa_r658_a_start_never_saved_leaves_nothing_and_runs_once_later,
    "with saves working the first runner runs the job and counts the start, and the check sees it",
    expected = "a start that can't be saved left what its file",
    start_never_saved_case("never_saved_control", false)
);

// --- A timeout while saves fail ---------------------------------------------------------------------------------------

/// A job ignoring SIGTERM passes its timeout while its saves fail. It must still be killed within the grace, with the
/// saves still failing, and end as timed out once they work. With a long timeout, the control, it is never killed.
fn timeout_while_failing_case(tag: &str, timeout: u64) {
    let q = Q::new(tag, "concurrency = 1\nidle = 10\n");
    let id = q.submit(
        "a",
        timeout,
        "trap '' TERM; echo $$ > leader; while :; do \"$0\" 0.1; done",
    );
    q.started(id);
    let leader = q.work().join("leader");
    assert!(
        until(10, || !pids(&leader).is_empty()),
        "no pid from the job"
    );
    let pid = pids(&leader)[0];
    let planted = q.plant(id);
    let ended_during = until(7, || !alive(pid));
    let during = q.state(id);
    std::fs::remove_dir(&planted).expect("plant removed");
    if !ended_during {
        kill(&pid.to_string());
    }
    let status = q.wait(id).0;
    let state = q.state(id);
    q.finish();
    assert!(
        ended_during && during == State::Running && status == 124 && state == State::TimedOut,
        "a job past its timeout was not ended while its saves failed: ended {ended_during}, during {during:?}, \
         status {status}, state {state:?}"
    );
}

#[test]
fn qa_r658_a_timeout_is_enforced_while_saves_fail() {
    timeout_while_failing_case("timeout", 2);
}

negative_control!(
    qa_r658_a_timeout_is_enforced_while_saves_fail,
    "a job with a long timeout is not killed in the window, and the check sees it",
    expected = "a job past its timeout was not ended",
    timeout_while_failing_case("timeout_control", 30)
);
