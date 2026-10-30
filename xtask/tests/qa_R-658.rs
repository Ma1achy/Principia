//! qa's tests for R-658, `cargo xtask job`, written from the owner's items 2, 3 and 5 (`decisions.md` § "R-658"), not
//! from the implementation:
//! - priority across classes, highest first, and first in, first out within one, past ten ids (item 3);
//! - a timeout ends the job's whole process group, a grandchild that ignores SIGTERM included (item 2);
//! - a hung runner's lock goes stale by itself and a new runner takes the queue over; the old one, resumed, steps
//!   aside (item 2, "stale locks expire");
//! - a new runner ends a dead runner's job with all its group, leaves alone a group whose pid was reused, and ends a
//!   job whose cancel was asked as cancelled (item 2, "robust to agent restarts");
//! - a job outlives a waiter killed with its whole process group, and `wait` re-attaches (item 2);
//! - the runner logs a heavy process outside the queue once, with its pid, cwd and command, and never kills it, nor a
//!   light one (item 5);
//! - at concurrency 2, two jobs on one target directory never run at once, while one on another does (item 2).
//!
//! Safety (the branch once fork-bombed the Mac): every live test has a queue of its own in a scratch folder; every
//! `xtask job` call runs under `ulimit -u` (this user's processes at the start, plus 300) and 45 s at most, as the
//! implementer's `xtask/tests/jobq.rs` bounds them; the commands are `sh`, `sleep`, `tail`, `true` and `perl`, never
//! cargo; every long-lived process is a stand-in linked inside the queue's folder, or names it as an argument, and the
//! queue's guard ends, by pid, every process naming its folder when the test ends.
#![cfg(unix)]

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;

use std::collections::BTreeMap;
use std::os::unix::process::CommandExt as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use scratch::Scratch;
use validation::negative_control;
use validation::spawn::Spawn;
use xtask::jobq::{Job, Priority, Queue, State};

/// The prefix of every scratch folder of these tests, so a sweep finds a dead test process's leftovers.
const PREFIX: &str = "qajobq_";

/// A queue of the test's own, in a scratch folder, with short timings.
struct Q {
    dir: Scratch,
}

impl Q {
    fn new(tag: &str, config: &str) -> Self {
        sweep_dead_tests();
        let dir = Scratch::new(&format!("{PREFIX}{tag}"));
        std::fs::create_dir_all(dir.join("work")).expect("scratch made");
        std::fs::write(
            dir.join("config"),
            format!(
                "concurrency = 1\nidle = 1\ntick_ms = 50\nsample = 1\ngrace = 1\nstale = 3\n{config}"
            ),
        )
        .expect("config written");
        Q { dir }
    }

    fn work(&self) -> PathBuf {
        self.dir.join("work")
    }

    fn path(&self) -> String {
        self.dir.to_str().expect("utf-8 path").to_owned()
    }

    fn queue(&self) -> Queue {
        Queue::at(self.dir.to_path_buf())
    }

    /// The `xtask job <args>` command on this queue, under the process cap.
    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new("bash");
        command
            .args(["-c", "ulimit -u \"$0\" && exec \"$@\""])
            .arg(process_cap().to_string())
            .arg(env!("CARGO_BIN_EXE_xtask"))
            .arg("job")
            .args(args)
            .env("PRINCIPIA_JOBQ", &*self.dir);
        command
    }

    fn run_within(&self, args: &[&str], within: Duration) -> std::io::Result<Output> {
        self.command(args).output_within(within)
    }

    fn run(&self, args: &[&str]) -> Output {
        self.run_within(args, Duration::from_secs(45))
            .expect("xtask job ran")
    }

    /// Submits `command` at `priority` with target directory `target`, returning its id.
    fn submit(
        &self,
        priority: &str,
        label: &str,
        target: &str,
        extra: &[&str],
        command: &[&str],
    ) -> u64 {
        let work = self.work();
        let env = format!("CARGO_TARGET_DIR={target}");
        let mut args = vec![
            "submit",
            work.to_str().expect("utf-8 path"),
            "--priority",
            priority,
            "--label",
            label,
            "--env",
            &env,
        ];
        args.extend_from_slice(extra);
        args.push("--");
        args.extend_from_slice(command);
        let out = self.run(&args);
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

    /// `job wait <id>`: its exit code and standard output.
    fn wait(&self, id: u64) -> (i32, String) {
        let out = self.run(&["wait", &id.to_string()]);
        (
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stdout).into_owned(),
        )
    }

    /// A stand-in executable inside the queue's folder, `bin-standin/<name>`, linked to `target`: every process it
    /// runs names the queue's folder, so the guard ends it.
    fn standin(&self, name: &str, target: &str) -> String {
        let bin = self.dir.join("bin-standin");
        std::fs::create_dir_all(&bin).expect("bin dir");
        let path = bin.join(name);
        if !path.exists() {
            std::os::unix::fs::symlink(target, &path).expect("stand-in linked");
        }
        path.to_str().expect("utf-8 path").to_owned()
    }

    /// Waits for the runner to exit (it does once idle), so nothing writes into the scratch as it is removed.
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
        for (pid, args) in ps_args() {
            if args.contains(&dir) && !args.starts_with("ps ") {
                kill("-9", pid);
            }
        }
    }
}

/// Every process's pid and command line.
fn ps_args() -> Vec<(i32, String)> {
    let Ok(out) = Command::new("ps")
        .args(["-axo", "pid=,args="])
        .output_within(Duration::from_secs(30))
    else {
        return Vec::new();
    };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let (pid, args) = line.trim_start().split_once(' ')?;
            Some((pid.parse().ok()?, args.to_owned()))
        })
        .collect()
}

/// Ends, once per test process, every process left naming the queue of an earlier, dead process of these tests.
fn sweep_dead_tests() {
    static SWEPT: std::sync::Once = std::sync::Once::new();
    SWEPT.call_once(|| {
        let prefix = format!("{}/{PREFIX}", env!("CARGO_TARGET_TMPDIR"));
        for (pid, args) in ps_args() {
            let Some(at) = args.find(&prefix) else {
                continue;
            };
            let name = args[at + prefix.len()..]
                .split('/')
                .next()
                .unwrap_or_default();
            let owner = name.rsplit('-').nth(1).and_then(|p| p.parse::<i32>().ok());
            if let Some(owner) = owner {
                if owner != std::process::id() as i32 && !alive(owner) && !args.starts_with("ps ") {
                    kill("-9", pid);
                }
            }
        }
    });
}

/// `ulimit -u` for every xtask call: this user's processes when the first test started, plus 300.
fn process_cap() -> usize {
    static CAP: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *CAP.get_or_init(|| {
        let out = Command::new("sh")
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

/// Whether a process with this pid exists and is no zombie.
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

fn kill(signal: &str, pid: i32) {
    let _ = Command::new("kill")
        .args([signal, &pid.to_string()])
        .output_within(Duration::from_secs(30));
}

/// Polls `ready` for up to `secs` seconds.
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

fn read_pid(path: &Path) -> i32 {
    assert!(
        until(30, || std::fs::read_to_string(path)
            .is_ok_and(|t| !t.trim().is_empty())),
        "no pid in {path:?}"
    );
    std::fs::read_to_string(path)
        .expect("pid file")
        .trim()
        .parse()
        .expect("a pid")
}

/// A pid that has exited.
fn dead_pid() -> i32 {
    let out = Command::new("sh")
        .args(["-c", "echo $$"])
        .output_within(Duration::from_secs(30))
        .expect("sh ran");
    String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse()
        .expect("a pid")
}

/// Starts `program args...` detached from this test, in `cwd`, returning its pid; with `own_group`, in a process
/// group of its own (perl's `setpgrp`, then exec).
fn start_stray(cwd: &Path, own_group: bool, program: &str, args: &[&str]) -> i32 {
    let script = if own_group {
        "perl -e 'setpgrp(0,0); exec @ARGV' \"$@\" >/dev/null 2>&1 & echo $!"
    } else {
        "\"$@\" >/dev/null 2>&1 & echo $!"
    };
    let out = Command::new("sh")
        .arg("-c")
        .arg(script)
        .arg("sh")
        .arg(program)
        .args(args)
        .current_dir(cwd)
        .output_within(Duration::from_secs(30))
        .expect("stray started");
    String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse()
        .expect("a pid")
}

/// A job file to plant, running or queued, in `worktree`.
fn planted(id: u64, worktree: PathBuf, state: State, command: &[&str]) -> Job {
    let mut env = BTreeMap::new();
    env.insert("CARGO_TARGET_DIR".to_owned(), format!("/t/{id}"));
    env.insert("PATH".to_owned(), "/usr/bin:/bin".to_owned());
    Job {
        id,
        label: format!("planted{id}"),
        priority: Priority::Fix,
        worktree,
        command: command.iter().map(|s| (*s).to_owned()).collect(),
        env,
        timeout_secs: 40,
        submitted: xtask::jobq::now(),
        state,
        attempts: u32::from(state == State::Running),
        started: (state == State::Running).then(xtask::jobq::now),
        finished: None,
        pgid: None,
        leader_start: None,
        runner_pid: (state == State::Running).then(dead_pid),
        exit: None,
        cancel_requested: false,
        note: None,
    }
}

/// A gate: loops until `go` exists in the job's folder, 40 s at most; its `$0` names the queue's folder.
const GATE: &str = "i=0; while [ ! -e go ] && [ $i -lt 800 ]; do sleep 0.05; i=$((i+1)); done";

// --- Item 3: priority across classes, then first in, first out ------------------------------------------------------

/// Twelve jobs, three per class, submitted interleaved behind a gate job holding the one slot (so their ids run past
/// ten), each appending its label to `order`: they must run as `expected`.
fn priority_case(tag: &str, expected: &str) {
    let q = Q::new(tag, "");
    let dir = q.path();
    let gate = q.submit("peek", "gate", "/t/gate", &[], &["sh", "-c", GATE, &dir]);
    let jobs = [
        ("build", "a"),
        ("peek", "b"),
        ("merge", "c"),
        ("fix", "d"),
        ("peek", "e"),
        ("build", "f"),
        ("merge", "g"),
        ("fix", "h"),
        ("merge", "i"),
        ("peek", "j"),
        ("fix", "k"),
        ("build", "l"),
    ];
    let ids: Vec<u64> = jobs
        .iter()
        .map(|(class, label)| {
            let script = format!("printf {label} >> order");
            q.submit(
                class,
                label,
                &format!("/t/{label}"),
                &[],
                &["sh", "-c", &script, &dir],
            )
        })
        .collect();
    std::fs::write(q.work().join("go"), "").expect("gate opened");
    assert_eq!(q.wait(gate).0, 0, "the gate job failed");
    for id in &ids {
        assert_eq!(q.wait(*id).0, 0, "job {id} failed");
    }
    let order = std::fs::read_to_string(q.work().join("order")).expect("order written");
    q.finish();
    assert!(
        ids.last() == Some(&13),
        "priority order wrong: ids {ids:?} do not pass ten"
    );
    assert_eq!(order, expected, "priority order wrong");
}

#[test]
fn qa_r658_priority_merge_fix_build_peek_then_fifo() {
    // merge c, g, i; fix d, h, k; build a, f, l; peek b, e, j.
    priority_case("priority", "cgidhkaflbej");
}

negative_control!(
    qa_r658_priority_merge_fix_build_peek_then_fifo,
    "within a class the first submitted runs first, so last-in-first-out is wrong",
    expected = "priority order wrong",
    priority_case("priority_control", "igckhdlfajeb")
);

// --- Item 2: a timeout ends the whole process group ------------------------------------------------------------------

/// A job whose leader and grandchild ignore SIGTERM, with `--timeout 1`: it must end as timed out (124), the
/// grandchild killed with it after the grace. With `escape`, the grandchild leaves the group (the control).
fn timeout_group_case(tag: &str, escape: bool) {
    let q = Q::new(tag, "");
    let sleeper = q.standin("qa-sleep", "/bin/sleep");
    let inner = if escape {
        "perl -e 'setpgrp(0,0); exec @ARGV' \"$0\" 60 & echo $! > gc.pid; wait"
    } else {
        "\"$0\" 60 & echo $! > gc.pid; wait"
    };
    // The leader ignores TERM; its child shell and that shell's sleep inherit the ignored TERM.
    let quoted = inner.replace('\'', "'\\''");
    let script = format!("trap '' TERM; sh -c '{quoted}' \"$0\" & echo $! > mid.pid; wait");
    let id = q.submit(
        "build",
        "timeout",
        "/t/timeout",
        &["--timeout", "1"],
        &["sh", "-c", &script, &sleeper],
    );
    let started = Instant::now();
    let (code, out) = q.wait(id);
    let (mid, grandchild) = (
        read_pid(&q.work().join("mid.pid")),
        read_pid(&q.work().join("gc.pid")),
    );
    let mid_alive = !until(5, || !alive(mid));
    let gc_alive = !until(5, || !alive(grandchild));
    for pid in [mid, grandchild] {
        kill("-9", pid);
    }
    q.finish();
    assert!(
        code == 124
            && out.contains("timed out")
            && !mid_alive
            && !gc_alive
            && started.elapsed() < Duration::from_secs(30),
        "job's group not ended at its timeout: status {code}, {out:?}, child alive {mid_alive}, \
         grandchild alive {gc_alive}"
    );
}

#[test]
fn qa_r658_timeout_kills_term_ignoring_grandchildren() {
    timeout_group_case("timeout_group", false);
}

negative_control!(
    qa_r658_timeout_kills_term_ignoring_grandchildren,
    "a grandchild that left the job's process group survives the group's kill, and the check sees it",
    expected = "job's group not ended at its timeout",
    timeout_group_case("timeout_group_control", true)
);

// --- Item 2: a hung runner's lock goes stale, and a new runner takes over ------------------------------------------

/// Stops the runner (SIGSTOP) at a moment it doesn't hold the state lock; returns whether it managed to.
fn stop_outside_the_lock(q: &Q, runner: i32) -> bool {
    for _ in 0..100 {
        kill("-STOP", runner);
        let file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(q.dir.join("state.lock"))
            .expect("state lock opened");
        if file.try_lock().is_ok() {
            return true;
        }
        kill("-CONT", runner);
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

/// With a job running, its runner hangs (SIGSTOP), so its heartbeat ages past `stale`: `wait` must start a new runner
/// that ends the job's first start and runs it again; the old runner, resumed, must exit without touching the job.
fn hung_runner_case(tag: &str, stale: u64) {
    let q = Q::new(tag, &format!("stale = {stale}\n"));
    let sleeper = q.standin("qa-sleep", "/bin/sleep");
    let script =
        "if [ -e marker ]; then exit 0; fi; touch marker; echo $$ > first.pid; exec \"$0\" 60";
    let id = q.submit(
        "fix",
        "hung",
        "/t/hung",
        &[],
        &["sh", "-c", script, &sleeper],
    );
    let first = read_pid(&q.work().join("first.pid"));
    let old = q
        .queue()
        .runner_lock()
        .expect("a runner holds the lock")
        .pid;
    let stopped = stop_outside_the_lock(&q, old);
    let waited = q.run_within(&["wait", &id.to_string()], Duration::from_secs(15));
    let code = waited
        .as_ref()
        .map_or(-1, |out| out.status.code().unwrap_or(-1));
    let new = q.queue().runner_lock().map(|lock| lock.pid);
    let after_takeover = q.queue().load(id).expect("job file");
    kill("-CONT", old);
    let old_exited = until(5, || !alive(old));
    let first_alive = !until(5, || !alive(first));
    let job = q.queue().load(id).expect("job file");
    if !job.state.is_final() {
        q.run(&["cancel", &id.to_string()]);
        q.wait(id);
    }
    if !old_exited {
        kill("-9", old);
        std::fs::remove_file(q.dir.join("runner.lock")).ok();
    }
    kill("-9", first);
    q.finish();
    assert!(
        stopped
            && code == 0
            && new != Some(old)
            && after_takeover.attempts == 2
            && old_exited
            && !first_alive
            && job == after_takeover,
        "hung runner's lock did not expire: stopped {stopped}, wait {waited:?}, lock {new:?} (old {old}), \
         starts {}, old exited {old_exited}, first start alive {first_alive}, job after resume {job:?}",
        after_takeover.attempts
    );
}

#[test]
fn qa_r658_a_hung_runners_lock_expires_and_it_steps_aside() {
    hung_runner_case("hung", 3);
}

negative_control!(
    qa_r658_a_hung_runners_lock_expires_and_it_steps_aside,
    "a heartbeat younger than `stale` holds the lock, so no runner takes over within the wait",
    expected = "hung runner's lock did not expire",
    hung_runner_case("hung_control", 60)
);

// --- Item 2: a new runner recovers a dead runner's jobs ---------------------------------------------------------------

/// A job that on its first start leaves a background child (outside its group with `escape`, the control) and
/// sleeps; its runner is killed; `wait` must run it again, its first start's whole group, the child included, ended.
fn recover_group_case(tag: &str, escape: bool) {
    let q = Q::new(tag, "");
    let sleeper = q.standin("qa-sleep", "/bin/sleep");
    let child = if escape {
        "perl -e 'setpgrp(0,0); exec @ARGV' \"$0\" 60 &"
    } else {
        "\"$0\" 60 &"
    };
    let script = format!(
        "if [ -e marker ]; then exit 0; fi; touch marker; {child} echo $! > bg.pid; echo $$ > first.pid; exec \"$0\" 60"
    );
    let id = q.submit(
        "fix",
        "recover",
        "/t/recover",
        &[],
        &["sh", "-c", &script, &sleeper],
    );
    let first = read_pid(&q.work().join("first.pid"));
    let background = read_pid(&q.work().join("bg.pid"));
    let runner = q
        .queue()
        .runner_lock()
        .expect("a runner holds the lock")
        .pid;
    kill("-9", runner);
    let (code, out) = q.wait(id);
    let first_alive = !until(5, || !alive(first));
    let background_alive = !until(5, || !alive(background));
    for pid in [first, background] {
        kill("-9", pid);
    }
    let job = q.queue().load(id).expect("job file");
    q.finish();
    assert!(
        code == 0 && job.attempts == 2 && !first_alive && !background_alive,
        "dead runner's job not recovered with its whole group: status {code}, {out:?}, starts {}, \
         first alive {first_alive}, background child alive {background_alive}",
        job.attempts
    );
}

#[test]
fn qa_r658_restart_ends_a_dead_runners_whole_group_and_reruns() {
    recover_group_case("recover", false);
}

negative_control!(
    qa_r658_restart_ends_a_dead_runners_whole_group_and_reruns,
    "a child that left the job's group survives recovery, and the check sees it",
    expected = "dead runner's job not recovered with its whole group",
    recover_group_case("recover_control", true)
);

/// A job left running by a dead runner, whose recorded group id is now another process's (a process in a group of
/// its own, whose start time is not the recorded one): recovery must leave that process alone and run the job again.
/// With `matching`, the recorded start time is that process's own, so it is the job's (the control).
fn reused_pgid_case(tag: &str, matching: bool) {
    let q = Q::new(tag, "");
    let sleeper = q.standin("qa-sleep", "/bin/sleep");
    let other = start_stray(&q.work(), true, &sleeper, &["60"]);
    assert!(until(10, || alive(other)), "stand-in never started");
    let lstart = Command::new("ps")
        .args(["-o", "lstart=", "-p", &other.to_string()])
        .output_within(Duration::from_secs(30))
        .expect("ps ran");
    let lstart = String::from_utf8_lossy(&lstart.stdout).trim().to_owned();
    std::fs::create_dir_all(q.dir.join("jobs")).expect("jobs dir");
    std::fs::write(q.dir.join("next_id"), "2\n").expect("next id");
    let mut job = planted(1, q.work(), State::Running, &["true"]);
    job.pgid = Some(other);
    job.leader_start = Some(if matching {
        lstart
    } else {
        "Thu Jan  1 00:00:00 1970".to_owned()
    });
    q.queue().save(&job).expect("job planted");
    let (code, out) = q.wait(1);
    let survived = alive(other);
    kill("-9", other);
    let job = q.queue().load(1).expect("job file");
    q.finish();
    assert!(
        code == 0 && job.attempts == 2 && survived,
        "recovery touched another process's group: status {code}, {out:?}, starts {}, other alive {survived}",
        job.attempts
    );
}

#[test]
fn qa_r658_restart_leaves_a_reused_group_alone() {
    reused_pgid_case("reused", false);
}

negative_control!(
    qa_r658_restart_leaves_a_reused_group_alone,
    "a group whose leader started when the job's did is the job's, and is ended",
    expected = "recovery touched another process's group",
    reused_pgid_case("reused_control", true)
);

/// A job left running by a dead runner after its cancel was asked must end as cancelled (130), not run again.
fn cancelled_recovery_case(tag: &str, cancel_requested: bool) {
    let q = Q::new(tag, "");
    std::fs::create_dir_all(q.dir.join("jobs")).expect("jobs dir");
    std::fs::write(q.dir.join("next_id"), "2\n").expect("next id");
    let mut job = planted(1, q.work(), State::Running, &["touch", "ran"]);
    job.cancel_requested = cancel_requested;
    q.queue().save(&job).expect("job planted");
    let (code, out) = q.wait(1);
    let ran = q.work().join("ran").exists();
    q.finish();
    assert!(
        code == 130 && !ran,
        "a cancelled job was run again after its runner died: status {code}, {out:?}, ran {ran}"
    );
}

#[test]
fn qa_r658_restart_ends_a_cancelled_job_as_cancelled() {
    cancelled_recovery_case("cancelled", true);
}

negative_control!(
    qa_r658_restart_ends_a_cancelled_job_as_cancelled,
    "a job whose cancel was never asked runs again",
    expected = "a cancelled job was run again",
    cancelled_recovery_case("cancelled_control", false)
);

// --- Item 2: `wait` re-attaches after its waiter dies ----------------------------------------------------------------

/// A `job wait` in a process group of its own, ended with its group when dropped.
struct Waiter(Child);

impl Drop for Waiter {
    fn drop(&mut self) {
        let group = format!("-{}", self.0.id());
        let _ = Command::new("kill")
            .args(["-9", "--", &group])
            .output_within(Duration::from_secs(30));
        let _ = self.0.wait();
    }
}

/// A queued job, no runner alive: a waiter starts the runner and is then killed with its whole process group (as an
/// agent's shell is). The runner and the job must live on, and a fresh `wait`, twice, must give the job's status
/// (7) and its log. With `runner_dies_too`, the runner is killed with the waiter (the control).
fn reattach_case(tag: &str, runner_dies_too: bool) {
    let q = Q::new(tag, "");
    let dir = q.path();
    std::fs::create_dir_all(q.dir.join("jobs")).expect("jobs dir");
    std::fs::write(q.dir.join("next_id"), "2\n").expect("next id");
    let script = format!("{GATE}; exit 7");
    q.queue()
        .save(&planted(
            1,
            q.work(),
            State::Queued,
            &["sh", "-c", &script, &dir],
        ))
        .expect("job planted");
    let waiter = Waiter(
        q.command(&["wait", "1"])
            .process_group(0)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("waiter started"),
    );
    let queue = q.queue();
    let running = until(20, || {
        queue.load(1).is_ok_and(|job| job.state == State::Running)
    });
    let runner = queue.runner_lock().map(|lock| lock.pid);
    drop(waiter);
    if runner_dies_too {
        if let Some(pid) = runner {
            kill("-9", pid);
        }
    }
    std::thread::sleep(Duration::from_millis(1000));
    let runner_alive = runner.is_some_and(alive);
    let still_running = queue.load(1).is_ok_and(|job| job.state == State::Running);
    std::fs::write(q.work().join("go"), "").expect("gate opened");
    let (code, out) = q.wait(1);
    let (again, _) = q.wait(1);
    let logged = out
        .lines()
        .find_map(|line| line.strip_prefix("log: "))
        .is_some_and(|path| Path::new(path).exists());
    let job = queue.load(1).expect("job file");
    q.finish();
    assert!(
        running
            && runner_alive
            && still_running
            && code == 7
            && again == 7
            && logged
            && job.attempts == 1,
        "job did not outlive its waiter: ran {running}, runner {runner:?} alive {runner_alive}, still running \
         {still_running}, re-attached status {code} then {again}, {out:?}, starts {}",
        job.attempts
    );
}

#[test]
fn qa_r658_wait_reattaches_after_its_waiter_dies() {
    reattach_case("reattach", false);
}

negative_control!(
    qa_r658_wait_reattaches_after_its_waiter_dies,
    "a runner that dies with the waiter is seen to",
    expected = "job did not outlive its waiter",
    reattach_case("reattach_control", true)
);

// --- Item 5: heavy processes outside the queue are logged, never killed ------------------------------------------

/// With a job running, starts outside the queue a stand-in `rustc` (sleep), a stand-in `cargo build` and a stand-in
/// `cargo fmt` (both `tail -f`): each heavy one must be logged once, with its pid, its cwd and its command, over
/// several samples, and every one must still be alive; the light one never logged. With `kill_named`, the test
/// itself kills the `rustc` once it is logged, as a detector that killed would (the control).
fn never_kills_case(tag: &str, kill_named: bool) {
    let q = Q::new(tag, "");
    let work = q.work();
    let sleeper = q.standin("qa-sleep", "/bin/sleep");
    let rustc = q.standin("rustc", "/bin/sleep");
    let cargo = q.standin("cargo", "/usr/bin/tail");
    for file in ["build", "fmt"] {
        std::fs::write(work.join(file), "").expect("tail's file");
    }
    let id = q.submit(
        "peek",
        "hold",
        "/t/hold",
        &["--timeout", "40"],
        &[&sleeper, "40"],
    );
    assert!(
        until(30, || q
            .queue()
            .load(id)
            .is_ok_and(|j| j.state == State::Running)),
        "job never ran"
    );
    let heavy_rustc = start_stray(&work, false, &rustc, &["40"]);
    let heavy_cargo = start_stray(&work, false, &cargo, &["-f", "build"]);
    let light_cargo = start_stray(&work, false, &cargo, &["-f", "fmt"]);
    let log = q.dir.join("violations.log");
    let read = || std::fs::read_to_string(&log).unwrap_or_default();
    let lines_of = |pid: i32| -> Vec<String> {
        read()
            .lines()
            .filter(|l| l.contains(&format!(" pid={pid} ")))
            .map(str::to_owned)
            .collect()
    };
    let named = until(10, || {
        !lines_of(heavy_rustc).is_empty() && !lines_of(heavy_cargo).is_empty()
    });
    if kill_named {
        kill("-9", heavy_rustc);
    }
    // Three more samples, a second apart.
    std::thread::sleep(Duration::from_millis(3500));
    let alive_all = [heavy_rustc, heavy_cargo, light_cargo].map(alive);
    let (rustc_lines, cargo_lines, light_lines) = (
        lines_of(heavy_rustc),
        lines_of(heavy_cargo),
        lines_of(light_cargo),
    );
    let work_lower = work.to_string_lossy().to_lowercase();
    let full = |lines: &[String], exe: &str| {
        lines.len() == 1
            && lines[0]
                .to_lowercase()
                .contains(&format!("cwd={work_lower} "))
            && lines[0].contains(&format!("command={exe}"))
            && lines[0].contains(" ppid=")
    };
    let (rustc_ok, cargo_ok) = (full(&rustc_lines, &rustc), full(&cargo_lines, &cargo));
    for pid in [heavy_rustc, heavy_cargo, light_cargo] {
        kill("-9", pid);
    }
    q.run(&["cancel", &id.to_string()]);
    q.wait(id);
    q.finish();
    assert!(
        named && alive_all == [true; 3] && rustc_ok && cargo_ok && light_lines.is_empty(),
        "violation detector wrong: named {named}, alive {alive_all:?}, rustc {rustc_lines:?}, cargo \
         {cargo_lines:?}, light {light_lines:?}:\n{}",
        read()
    );
}

#[test]
fn qa_r658_violations_are_logged_once_and_never_killed() {
    never_kills_case("never_kills", false);
}

negative_control!(
    qa_r658_violations_are_logged_once_and_never_killed,
    "a logged process that is then killed is seen dead",
    expected = "violation detector wrong",
    never_kills_case("never_kills_control", true)
);

// --- Item 2: concurrency 2 never runs two jobs on one target directory --------------------------------------------

/// At concurrency 2: job A holds target directory `/t/shared` until `go`; job B, submitted next on `b_target`, records
/// whether A was running when it started; job C, last, on its own directory. C must run beside A, B not until A has
/// ended. With B on another directory (the control), B runs beside A.
fn shared_target_case(tag: &str, b_target: &str) {
    let q = Q::new(tag, "concurrency = 2\n");
    let dir = q.path();
    let a_script = format!("touch A.run; {GATE}; rm A.run");
    let a = q.submit(
        "build",
        "a",
        "/t/shared",
        &[],
        &["sh", "-c", &a_script, &dir],
    );
    assert!(
        until(20, || q.work().join("A.run").exists()),
        "job A never ran"
    );
    let b = q.submit(
        "build",
        "b",
        b_target,
        &[],
        &[
            "sh",
            "-c",
            "if [ -e A.run ]; then touch overlap; fi; touch B.ran",
            &dir,
        ],
    );
    let c = q.submit(
        "build",
        "c",
        "/t/c",
        &[],
        &["sh", "-c", "touch C.ran", &dir],
    );
    let c_beside_a = until(20, || q.work().join("C.ran").exists());
    std::thread::sleep(Duration::from_millis(1000));
    let b_early = q.work().join("B.ran").exists();
    std::fs::write(q.work().join("go"), "").expect("gate opened");
    let codes = [q.wait(a).0, q.wait(b).0, q.wait(c).0];
    let overlap = q.work().join("overlap").exists();
    q.finish();
    assert!(
        c_beside_a && !b_early && !overlap && codes == [0, 0, 0],
        "two jobs ran on one target directory: C beside A {c_beside_a}, B before A ended {b_early}, overlap \
         {overlap}, statuses {codes:?}"
    );
}

#[test]
fn qa_r658_concurrency_two_never_shares_a_target_directory() {
    shared_target_case("shared", "/t/shared");
}

negative_control!(
    qa_r658_concurrency_two_never_shares_a_target_directory,
    "jobs on two directories run at once, and the check sees the overlap",
    expected = "two jobs ran on one target directory",
    shared_target_case("shared_control", "/t/other")
);
