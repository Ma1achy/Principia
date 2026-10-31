//! `cargo xtask job` (R-658): the local job queue for heavy commands. Each live test runs the real binary on a queue of
//! its own, in a scratch folder, with dummy commands (`sh`, `sleep`, `true`), never cargo: jobs run by class, then
//! first in, first out; a timed-out job's whole process group is killed; a runner lock whose pid is dead or whose
//! heartbeat is stale expires; a new runner ends and requeues the jobs a dead runner left running; and heavy processes
//! outside the queue are logged, those inside it not. The pure parts (the order, the heavy-process test, the lock's
//! expiry, the config) are checked at their boundaries.
#![cfg(unix)]

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;

use std::collections::{BTreeMap, BTreeSet};
use std::os::unix::process::CommandExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

use scratch::Scratch;
use validation::negative_control;
use validation::spawn::Spawn;
use xtask::jobq::{
    archivable, heavy, leader_is_jobs, lock_holder, may_signal_group, next_job, parse_etime,
    parse_ps, parse_submit, pid_alive, run_order, runner_settings, snapshot_env, target_key,
    timed_out, utc, violations, Config, Job, Priority, Proc, Queue, RunnerLock, State,
    ARCHIVE_AFTER, MAX_ATTEMPTS, START_SLACK,
};

/// A job that waits for the file `go`, 40 s at most: its queue's drop can miss it, should the test die, and a gate with
/// no bound would go on forking `sleep` until the machine ends.
const GATE: &str = "i=0; while [ ! -e go ] && [ $i -lt 800 ]; do sleep 0.05; i=$((i+1)); done";

/// A queue of the test's own, in a scratch folder, with short timings.
struct Q {
    dir: Scratch,
}

impl Q {
    fn new(tag: &str, config: &str) -> Self {
        sweep_dead_tests();
        let dir = Scratch::new(tag);
        std::fs::create_dir_all(dir.join("work")).expect("scratch made");
        std::fs::write(
            dir.join("config"),
            format!("concurrency = 1\nidle = 1\ntick_ms = 50\nsample = 1\ngrace = 1\nstale = 3\n{config}"),
        )
        .expect("config written");
        Q { dir }
    }

    fn work(&self) -> PathBuf {
        self.dir.join("work")
    }

    fn queue(&self) -> Queue {
        Queue::at(self.dir.to_path_buf())
    }

    /// Runs `xtask job <args>` on this queue, bounded by `within`, under the process limits ([`Q::command`]), which
    /// the runner it may start and the runner's jobs inherit.
    fn run_within(&self, args: &[&str], within: Duration) -> std::io::Result<Output> {
        self.command(args).output_within(within)
    }

    /// `xtask job <args>` on this queue, under the process cap ([`process_cap`]), and 600 s of CPU and 1 GiB per file, so
    /// that a runner that spins (a mutant's) ends, and one that writes without end stops.
    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new("/bin/bash");
        command
            .args([
                "-c",
                "ulimit -u \"$0\" && ulimit -t 600 && ulimit -f 1048576 && exec \"$@\"",
            ])
            .arg(process_cap().to_string())
            .arg(env!("CARGO_BIN_EXE_xtask"))
            .arg("job")
            .args(args)
            .env("PRINCIPIA_JOBQ", &*self.dir);
        command
    }

    /// Submits `command` as [`Q::submit`] does, from a submitter whose `PATH` is `path`.
    fn submit_with_path(&self, path: &str, label: &str, extra: &[&str], command: &[&str]) -> u64 {
        let work = self.work();
        let mut args = vec![
            "submit",
            work.to_str().expect("utf-8 path"),
            "--priority",
            "peek",
            "--label",
            label,
            "--env",
            "CARGO_TARGET_DIR=/nonexistent/jobq-test-target",
        ];
        args.extend_from_slice(extra);
        args.push("--");
        args.extend_from_slice(command);
        let out = self
            .command(&args)
            .env("PATH", path)
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

    fn run(&self, args: &[&str]) -> Output {
        self.run_within(args, Duration::from_secs(45))
            .expect("xtask job ran")
    }

    /// Submits `command` at `priority`, returning its id.
    fn submit(&self, priority: &str, label: &str, extra: &[&str], command: &[&str]) -> u64 {
        let work = self.work();
        let mut args = vec![
            "submit",
            work.to_str().expect("utf-8 path"),
            "--priority",
            priority,
            "--label",
            label,
            "--env",
            "CARGO_TARGET_DIR=/nonexistent/jobq-test-target",
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

    /// `job wait <id>`: its exit code and its standard output.
    fn wait(&self, id: u64) -> (i32, String) {
        let out = self.run(&["wait", &id.to_string()]);
        (
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stdout).into_owned(),
        )
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
    /// Ends every process still running for this queue, by pid: one that names its state directory in its arguments, as
    /// a runner and each stand-in the tests start do, or whose working directory is in it, as a job's is. A test that
    /// passed leaves none; one that failed, or a mutant, may. Twice, since a runner may start a job as it is read.
    fn drop(&mut self) {
        let dir = self.dir.to_string_lossy().to_lowercase();
        for _ in 0..2 {
            for (pid, text) in process_texts() {
                if text.contains(&dir) && !text.starts_with("ps ") {
                    kill("-9", pid);
                }
            }
        }
    }
}

/// Every process's pid, with its command line and working directory as one text, in lower case: `lsof` gives a directory
/// in the case its volume stores, which on the Mac's case-insensitive volume need not be the case a test named it in.
fn process_texts() -> Vec<(i32, String)> {
    let Ok(out) = Command::new("ps")
        .args(["-axo", "pid=,args="])
        .output_within(Duration::from_secs(30))
    else {
        return Vec::new();
    };
    let cwds = working_directories();
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let (pid, args) = line.trim_start().split_once(' ')?;
            let pid: i32 = pid.parse().ok()?;
            let cwd = cwds.get(&pid).map_or("", String::as_str);
            Some((pid, format!("{args}\n{cwd}").to_lowercase()))
        })
        .collect()
}

/// This user's processes' working directories: from `/proc` on Linux, from `lsof` elsewhere.
fn working_directories() -> BTreeMap<i32, String> {
    let mut cwds = BTreeMap::new();
    if let Ok(entries) = std::fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let pid = entry.file_name().to_str().and_then(|n| n.parse().ok());
            if let (Some(pid), Ok(cwd)) = (pid, std::fs::read_link(entry.path().join("cwd"))) {
                cwds.insert(pid, cwd.display().to_string());
            }
        }
        return cwds;
    }
    let Ok(out) = Command::new("sh")
        .args(["-c", "lsof -a -d cwd -u \"$(id -u)\" -Fpn"])
        .output_within(Duration::from_secs(30))
    else {
        return cwds;
    };
    let mut pid = None;
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        if let Some(value) = line.strip_prefix('p') {
            pid = value.parse().ok();
        } else if let (Some(pid), Some(value)) = (pid, line.strip_prefix('n')) {
            cwds.insert(pid, value.to_owned());
        }
    }
    cwds
}

/// Ends, once per test process, every process left of an earlier test process that has gone (a test killed at a
/// timeout, as nextest and cargo-mutants kill one, never drops its queue).
fn sweep_dead_tests() {
    static SWEPT: std::sync::Once = std::sync::Once::new();
    SWEPT.call_once(sweep_dead_tests_now);
}

/// Ends every process that names, in its arguments or working directory, a scratch folder of these tests or of qa's
/// (`<CARGO_TARGET_TMPDIR>/[qa]jobq_<tag>-<pid>-<n>`) whose test process, the `<pid>`, has gone.
fn sweep_dead_tests_now() {
    let root = format!("{}/", env!("CARGO_TARGET_TMPDIR")).to_lowercase();
    for (pid, text) in process_texts() {
        let dead = text.split(&root).skip(1).any(|rest| {
            let name = rest.split(['/', ' ', '\n']).next().unwrap_or_default();
            let owner = name.rsplit('-').nth(1).and_then(|p| p.parse::<i32>().ok());
            name.contains("jobq_")
                && owner.is_some_and(|owner| owner != std::process::id() as i32 && !alive(owner))
        });
        if dead && !text.starts_with("ps ") {
            kill("-9", pid);
        }
    }
}

/// The most processes this user may have while an xtask call of these tests runs (`ulimit -u`): as many as it had
/// when the first test started, plus 300, so that code that starts processes without bound (a mutant's) fails to fork
/// long before it fills the machine's per-user limit.
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

/// Whether a process with this pid exists.
fn alive(pid: i32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .output_within(Duration::from_secs(30))
        .expect("kill ran")
        .status
        .success()
}

/// Signals `pid`, or the group `-pid`. Never 0, 1, -1 or their like: `kill -9 -1` is every process of this user.
fn kill(signal: &str, pid: i32) {
    if pid.abs() <= 1 {
        return;
    }
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

// --- Priority, then first in, first out ---------------------------------------------------------------------------

/// Submits a job that holds the runner until `go` exists, then `jobs` (class, label) in order, each appending its
/// label to `order`; lets the gate go and checks the labels ran in the order `expected`.
fn run_order_case(tag: &str, jobs: &[(&str, &str)], expected: &str) {
    let q = Q::new(tag, "");
    let gate = q.submit("peek", "gate", &[], &["sh", "-c", GATE]);
    let ids: Vec<u64> = jobs
        .iter()
        .map(|(class, label)| {
            q.submit(
                class,
                label,
                &[],
                &["sh", "-c", &format!("printf {label} >> order")],
            )
        })
        .collect();
    std::fs::write(q.work().join("go"), "").expect("gate opened");
    assert_eq!(q.wait(gate).0, 0, "the gate job failed");
    for id in ids {
        assert_eq!(q.wait(id).0, 0, "job {id} failed");
    }
    let order = std::fs::read_to_string(q.work().join("order")).expect("order written");
    q.finish();
    assert_eq!(order, expected, "run order wrong");
}

const MIXED: [(&str, &str); 7] = [
    ("peek", "a"),
    ("build", "b"),
    ("fix", "c"),
    ("merge", "d"),
    ("peek", "e"),
    ("merge", "f"),
    ("fix", "g"),
];

#[test]
fn jobq_runs_by_class_then_first_in_first_out() {
    run_order_case("jobq_order", &MIXED, "dfcgbae");
}

negative_control!(
    jobq_runs_by_class_then_first_in_first_out,
    "the queue reorders by class, so submission order is not the run order",
    expected = "run order wrong",
    run_order_case("jobq_order_control", &MIXED, "abcdefg")
);

fn job(id: u64, priority: Priority, state: State, target: Option<&str>) -> Job {
    let mut env = BTreeMap::new();
    if let Some(target) = target {
        env.insert("CARGO_TARGET_DIR".to_owned(), target.to_owned());
    }
    Job {
        id,
        label: format!("j{id}"),
        priority,
        worktree: PathBuf::from("/"),
        command: vec!["true".to_owned()],
        env,
        timeout_secs: 10,
        submitted: 0,
        state,
        attempts: 0,
        started: None,
        finished: None,
        pgid: None,
        leader_start: None,
        runner_pid: None,
        exit: None,
        cancel_requested: false,
        note: None,
    }
}

/// `next_job` on `jobs` picks `expected`; `run_order` lists the queued ones as `order`.
fn check_next(jobs: &[Job], expected: Option<u64>, order: &[u64]) {
    assert_eq!(next_job(jobs).map(|j| j.id), expected, "next job wrong");
    let listed: Vec<u64> = run_order(jobs).iter().map(|j| j.id).collect();
    assert_eq!(listed, order, "next job wrong: run order");
}

#[test]
fn jobq_next_job_by_class_id_and_free_target() {
    use Priority::*;
    use State::*;
    // Class first, then id; a running or ended job is never next.
    let jobs = [
        job(1, Merge, Running, Some("/t1")),
        job(2, Peek, Queued, Some("/t2")),
        job(3, Build, Queued, Some("/t3")),
        job(4, Merge, Exited, Some("/t4")),
        job(5, Build, Queued, Some("/t5")),
        job(6, Fix, Queued, Some("/t6")),
    ];
    check_next(&jobs, Some(6), &[6, 3, 5, 2]);
    // A queued job whose target directory a running job uses waits; the next one runs.
    let shared = [
        job(1, Peek, Running, Some("/t")),
        job(2, Merge, Queued, Some("/t")),
        job(3, Fix, Queued, Some("/u")),
        job(4, Merge, Queued, None),
    ];
    check_next(&shared, Some(4), &[2, 4, 3]);
    let only_shared = [
        job(1, Peek, Running, Some("/t")),
        job(2, Merge, Queued, Some("/t")),
    ];
    check_next(&only_shared, None, &[2]);
    // An ended job's target directory is free.
    let ended = [
        job(1, Peek, TimedOut, Some("/t")),
        job(2, Merge, Queued, Some("/t")),
    ];
    check_next(&ended, Some(2), &[2]);
    check_next(&[], None, &[]);
}

negative_control!(
    jobq_next_job_by_class_id_and_free_target,
    "with the later id expected first, FIFO within a class fails",
    expected = "next job wrong",
    check_next(
        &[
            job(1, Priority::Fix, State::Queued, None),
            job(2, Priority::Fix, State::Queued, None)
        ],
        Some(2),
        &[2, 1]
    )
);

/// `target_key` gives `a` and `b` one key exactly when `same`.
fn check_key(a: &str, b: &str, same: bool) {
    let (ka, kb) = (target_key(a), target_key(b));
    assert_eq!(
        ka == kb,
        same,
        "target keys wrong: {a:?} gives {ka:?}, {b:?} gives {kb:?}"
    );
}

#[test]
fn jobq_target_directories_compare_by_a_normalised_key() {
    let dir = Scratch::new("jobq_target_key");
    let real = dir.join("Real");
    std::fs::create_dir_all(real.join("sub")).expect("scratch made");
    std::os::unix::fs::symlink(&real, dir.join("link")).expect("link made");
    let path = |p: PathBuf| p.to_str().expect("utf-8 path").to_owned();
    let (real, link) = (path(real), path(dir.join("link")));
    // Paths that don't exist: compared as written, but for `/`, `.` and case.
    check_key("/t/a", "/t/a/", true);
    check_key("/t/a", "/T/A", true);
    check_key("/t/a", "/t//./a", true);
    check_key("/t/a", "/t/b", false);
    check_key("/t/a", "/t/ab", false);
    // Paths that exist, or whose parent does: made canonical, through the link.
    check_key(&real, &format!("{real}/"), true);
    check_key(&real, &link, true);
    check_key(&format!("{real}/sub"), &format!("{link}/SUB/"), true);
    check_key(&format!("{real}/new"), &format!("{link}/New"), true);
    check_key(&format!("{real}/new"), &format!("{real}/other"), false);
    // A queued job on another spelling of a running job's target directory waits.
    use Priority::*;
    use State::*;
    let jobs = [
        job(1, Peek, Running, Some("/t/A/")),
        job(2, Merge, Queued, Some("/T/a")),
        job(3, Build, Queued, Some("/t/b")),
    ];
    check_next(&jobs, Some(3), &[2, 3]);
}

negative_control!(
    jobq_target_directories_compare_by_a_normalised_key,
    "two directories are two keys",
    expected = "target keys wrong",
    check_key("/t/a", "/t/b", true)
);

// --- Timeouts kill the whole process group ------------------------------------------------------------------------

/// A job that starts a background `sleep 60`, then waits on it (or, with `exit_at_once`, exits): with `--timeout 1`
/// it must end as timed out, status 124, its background child killed with it.
fn timeout_case(tag: &str, exit_at_once: bool) {
    let q = Q::new(tag, "");
    let tail = if exit_at_once { "exit 0" } else { "wait" };
    let script = format!("sleep 60 & echo $! > child.pid; {tail}");
    let id = q.submit(
        "build",
        "timeout",
        &["--timeout", "1"],
        &["sh", "-c", &script],
    );
    let started = Instant::now();
    let (code, out) = q.wait(id);
    let child = read_pid(&q.work().join("child.pid"));
    let child_alive = !until(5, || !alive(child));
    if child_alive {
        kill("-9", child);
    }
    q.finish();
    assert!(
        code == 124 && out.contains("timed out") && !child_alive && started.elapsed() < Duration::from_secs(60),
        "job not ended by its timeout with its group: status {code}, {out:?}, child alive {child_alive}"
    );
}

#[test]
fn jobq_timeout_kills_the_whole_group() {
    timeout_case("jobq_timeout", false);
}

negative_control!(
    jobq_timeout_kills_the_whole_group,
    "a job that exits by itself ends with its own status, 0, not by its timeout",
    expected = "job not ended by its timeout",
    timeout_case("jobq_timeout_control", true)
);

/// A job that starts a background `sleep 60` (with `escape`, in a process group of its own) and exits 0 at once: it
/// must end with status 0, and its background child, left in its group, must be killed when the runner reaps it.
fn finished_group_case(tag: &str, escape: bool) {
    let q = Q::new(tag, "");
    let child = if escape {
        "perl -e 'setpgrp(0,0); exec @ARGV' sleep 60"
    } else {
        "sleep 60"
    };
    let script = format!("{child} & echo $! > child.pid; exit 0");
    let id = q.submit("build", "finished", &[], &["sh", "-c", &script]);
    let (code, out) = q.wait(id);
    let child = read_pid(&q.work().join("child.pid"));
    let child_alive = !until(5, || !alive(child));
    if child_alive {
        kill("-9", child);
    }
    q.finish();
    assert!(
        code == 0 && !child_alive,
        "a finished job's group outlived it: status {code}, {out:?}, child alive {child_alive}"
    );
}

#[test]
fn jobq_a_finished_jobs_group_is_killed() {
    finished_group_case("jobq_finished", false);
}

negative_control!(
    jobq_a_finished_jobs_group_is_killed,
    "a child that left the job's process group survives the group's kill, and the check sees it",
    expected = "a finished job's group outlived it",
    finished_group_case("jobq_finished_control", true)
);

/// A job `sleep 3` with `--timeout <timeout>` whose recorded start is set an hour back once it runs, as a clock
/// stepped forward would make it look: it must end with status `expected`, 0 unless its own timeout passes.
fn clock_step_case(tag: &str, timeout: &str, expected: i32) {
    let q = Q::new(tag, "");
    let id = q.submit("peek", "clock", &["--timeout", timeout], &["sleep", "3"]);
    let queue = q.queue();
    assert!(
        until(30, || queue
            .load(id)
            .is_ok_and(|j| j.state == State::Running)),
        "never ran"
    );
    let mut job = queue.load(id).expect("job file");
    job.started = job.started.map(|t| t - 3600);
    queue.save(&job).expect("job saved");
    let (code, out) = q.wait(id);
    q.finish();
    assert_eq!(code, expected, "timeout read from the wall clock: {out:?}");
}

#[test]
fn jobq_timeout_is_measured_on_a_monotonic_clock() {
    clock_step_case("jobq_clock", "60", 0);
    for (ran, timeout, past) in [(9_999, 10, false), (10_000, 10, true), (10_001, 10, true)] {
        assert_eq!(
            timed_out(Duration::from_millis(ran), timeout),
            past,
            "timeout read from the wall clock: {ran} ms of {timeout} s"
        );
    }
}

negative_control!(
    jobq_timeout_is_measured_on_a_monotonic_clock,
    "a job past its own timeout is timed out",
    expected = "timeout read from the wall clock",
    clock_step_case("jobq_clock_control", "1", 0)
);

// --- Stale runner locks expire ------------------------------------------------------------------------------------

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

/// Plants a runner lock of `pid` with a heartbeat `age` seconds old (the queue's `stale` is `stale`), submits `true`,
/// and checks a new runner took the lock over and ran it.
fn stale_lock_case(tag: &str, pid: i32, age: u64, stale: u64) {
    let q = Q::new(tag, &format!("stale = {stale}\n"));
    std::fs::create_dir_all(q.dir.join("jobs")).expect("jobs dir");
    let lock = format!("pid {pid}\nheartbeat {}\n", xtask::jobq::now() - age);
    std::fs::write(q.dir.join("runner.lock"), lock).expect("lock planted");
    let id = q.submit("peek", "after-stale", &[], &["true"]);
    let waited = q.run_within(&["wait", &id.to_string()], Duration::from_secs(4));
    let ran = waited.as_ref().is_ok_and(|out| out.status.success());
    let taken = q.queue().runner_lock().is_none_or(|lock| lock.pid != pid);
    if !ran {
        std::fs::remove_file(q.dir.join("runner.lock")).ok();
        q.run(&["cancel", &id.to_string()]);
    }
    q.finish();
    assert!(
        ran && taken,
        "job did not run under the expired lock: {waited:?}"
    );
}

#[test]
fn jobq_lock_of_a_dead_pid_expires() {
    stale_lock_case("jobq_dead_lock", dead_pid(), 0, 60);
}

#[test]
fn jobq_lock_with_a_stale_heartbeat_expires() {
    // The pid is alive (this test's own), but its heartbeat is a minute past `stale`.
    stale_lock_case("jobq_stale_lock", std::process::id() as i32, 120, 60);
}

negative_control!(
    jobq_lock_of_a_dead_pid_expires,
    "a lock of a live pid with a fresh heartbeat holds, so no runner starts",
    expected = "job did not run under the expired lock",
    stale_lock_case("jobq_dead_lock_control", std::process::id() as i32, 0, 60)
);

negative_control!(
    jobq_lock_with_a_stale_heartbeat_expires,
    "a heartbeat younger than `stale` holds the lock",
    expected = "job did not run under the expired lock",
    stale_lock_case("jobq_stale_lock_control", std::process::id() as i32, 50, 60)
);

/// `RunnerLock::is_live` at `now`: live only with the pid alive and the heartbeat younger than `stale`.
fn check_lock(lock: RunnerLock, now: u64, stale: u64, pid_alive: bool, live: bool) {
    assert_eq!(
        lock.is_live(now, stale, |_| pid_alive),
        live,
        "lock liveness wrong at {now}"
    );
}

#[test]
fn jobq_lock_expires_at_exactly_stale_seconds() {
    let lock = RunnerLock {
        pid: 7,
        heartbeat: 1000,
    };
    check_lock(lock, 1059, 60, true, true);
    check_lock(lock, 1060, 60, true, false);
    check_lock(lock, 1000, 60, false, false);
    // A heartbeat ahead of the clock (a clock set back) holds.
    check_lock(lock, 990, 60, true, true);
    assert_eq!(
        RunnerLock::parse("pid 7\nheartbeat 1000\n"),
        Some(lock),
        "lock text misread"
    );
    assert_eq!(
        RunnerLock::parse("heartbeat 1000\n"),
        None,
        "lock text misread"
    );
    assert_eq!(RunnerLock::parse("pid 7\n"), None, "lock text misread");
}

negative_control!(
    jobq_lock_expires_at_exactly_stale_seconds,
    "a heartbeat exactly `stale` seconds old has expired",
    expected = "lock liveness wrong",
    check_lock(
        RunnerLock {
            pid: 7,
            heartbeat: 1000
        },
        1060,
        60,
        true,
        true
    )
);

// --- A restarted runner recovers the jobs a dead one left running --------------------------------------------------

/// Starts a job that, on its first start, records its pid and sleeps; kills the runner (with `kill_runner`; else
/// cancels the job); then waits: a new runner must have ended the first start's process and run the job again.
fn restart_case(tag: &str, kill_runner: bool) {
    let q = Q::new(tag, "");
    let script =
        "if [ -e marker ]; then exit 0; fi; touch marker; echo $$ > first.pid; exec sleep 60";
    let id = q.submit("fix", "restart", &[], &["sh", "-c", script]);
    let first = read_pid(&q.work().join("first.pid"));
    let runner = q
        .queue()
        .runner_lock()
        .expect("a runner holds the lock")
        .pid;
    if kill_runner {
        kill("-9", runner);
    } else {
        q.run(&["cancel", &id.to_string()]);
    }
    let (code, out) = q.wait(id);
    let first_alive = alive(first);
    if first_alive {
        kill("-9", first);
    }
    let job = q.queue().load(id).expect("job file");
    q.finish();
    assert!(
        code == 0 && job.attempts == 2 && !first_alive,
        "job not started again after its runner died: status {code}, {out:?}, starts {}, first alive {first_alive}",
        job.attempts
    );
}

#[test]
fn jobq_restart_requeues_a_dead_runners_job() {
    restart_case("jobq_restart", true);
}

negative_control!(
    jobq_restart_requeues_a_dead_runners_job,
    "with the runner alive and the job cancelled, it is never started again",
    expected = "job not started again after its runner died",
    restart_case("jobq_restart_control", false)
);

/// A job left running by a dead runner after `attempts` starts: at [`MAX_ATTEMPTS`] it is lost (status 125), below it
/// it runs again (status 0).
fn lost_case(tag: &str, attempts: u32, expected: i32) {
    let q = Q::new(tag, "");
    std::fs::create_dir_all(q.dir.join("jobs")).expect("jobs dir");
    let mut planted = job(1, Priority::Peek, State::Running, Some("/t"));
    planted.worktree = q.work();
    planted.attempts = attempts;
    planted.runner_pid = Some(dead_pid());
    q.queue().save(&planted).expect("job planted");
    // `wait` starts a runner when none is alive (one started by this test binary would be the test binary).
    let (code, out) = q.wait(1);
    q.finish();
    assert_eq!(code, expected, "recovered job ended wrongly: {out:?}");
}

#[test]
fn jobq_restart_marks_a_job_lost_after_its_last_start() {
    lost_case("jobq_lost", MAX_ATTEMPTS, 125);
    lost_case("jobq_lost_below", MAX_ATTEMPTS - 1, 0);
}

negative_control!(
    jobq_restart_marks_a_job_lost_after_its_last_start,
    "one start short of the last, the job runs again",
    expected = "recovered job ended wrongly",
    lost_case("jobq_lost_control", MAX_ATTEMPTS - 1, 125)
);

/// Plants a job left `Running` by a runner that died before recording its leader's start time (`leader_start` none),
/// whose group leader is a live stand-in that started now, while the job's file says it started `ago` seconds ago;
/// a runner must end that group before it starts the job again, which the second start checks.
fn unrecorded_start_case(tag: &str, ago: u64) {
    let q = Q::new(tag, "");
    std::fs::create_dir_all(q.dir.join("jobs")).expect("jobs dir");
    // The stand-in names the queue in its arguments, so the queue's drop ends it should the test fail.
    let mut leader = Command::new("/bin/sh")
        .args(["-c", "sleep 60; :"])
        .arg(&*q.dir)
        .process_group(0)
        .spawn()
        .expect("stand-in started");
    let pid = leader.id() as i32;
    let reaper = std::thread::spawn(move || leader.wait());
    let mut planted = job(1, Priority::Peek, State::Running, Some("/t"));
    planted.worktree = q.work();
    planted.attempts = 1;
    planted.started = Some(xtask::jobq::now() - ago);
    planted.pgid = Some(pid);
    planted.runner_pid = Some(dead_pid());
    planted.command = [
        "/bin/sh",
        "-c",
        &format!("if kill -0 {pid} 2>/dev/null; then exit 3; fi"),
    ]
    .map(str::to_owned)
    .to_vec();
    q.queue().save(&planted).expect("job planted");
    let (code, out) = q.wait(1);
    let left = alive(pid);
    if left {
        // The group, so the stand-in's `sleep` goes with it.
        kill("-9", -pid);
    }
    let _ = reaper.join();
    q.finish();
    assert!(
        code == 0 && !left,
        "a dead runner's group left running beside the job's second start: status {code}, {out:?}, leader alive {left}"
    );
}

#[test]
fn jobq_restart_ends_a_group_whose_start_was_not_recorded() {
    unrecorded_start_case("jobq_unrecorded", 0);
}

negative_control!(
    jobq_restart_ends_a_group_whose_start_was_not_recorded,
    "a leader started an hour from the job's start is another process's, left alone",
    expected = "a dead runner's group left running",
    unrecorded_start_case("jobq_unrecorded_control", 3600)
);

/// `leader_is_jobs` for a job started at 1000 with `recorded` as its leader's start time, against a live leader whose
/// `ps` start is `start` and whose start in Unix seconds is `at`.
fn check_leader(
    recorded: Option<&str>,
    start: Option<&str>,
    at: Option<u64>,
    started: Option<u64>,
    is: bool,
) {
    let mut j = job(1, Priority::Peek, State::Running, Some("/t"));
    j.started = started;
    j.leader_start = recorded.map(str::to_owned);
    assert_eq!(
        leader_is_jobs(&j, start, at),
        is,
        "leader misjudged: recorded {recorded:?}, start {start:?}, at {at:?}, started {started:?}"
    );
}

#[test]
fn jobq_a_leader_is_the_jobs_by_its_start() {
    let s = Some(1000);
    // A recorded start time decides alone.
    check_leader(
        Some("Fri 30 Oct 18:00:00 2026"),
        Some("Fri 30 Oct 18:00:00 2026"),
        None,
        s,
        true,
    );
    check_leader(
        Some("Fri 30 Oct 18:00:00 2026"),
        Some("Fri 30 Oct 18:00:01 2026"),
        Some(1000),
        s,
        false,
    );
    check_leader(Some("Fri 30 Oct 18:00:00 2026"), None, Some(1000), s, false);
    // None recorded: within START_SLACK of the job's start, either side, and no further.
    check_leader(None, None, Some(1000), s, true);
    check_leader(None, None, Some(1000 + START_SLACK), s, true);
    check_leader(None, None, Some(1000 + START_SLACK + 1), s, false);
    check_leader(None, None, Some(1000 - START_SLACK), s, true);
    check_leader(None, None, Some(1000 - START_SLACK - 1), s, false);
    check_leader(None, None, None, s, false);
    check_leader(None, None, Some(1000), None, false);
}

negative_control!(
    jobq_a_leader_is_the_jobs_by_its_start,
    "a leader started one second past the slack is not the job's",
    expected = "leader misjudged",
    check_leader(None, None, Some(1000 + START_SLACK + 1), Some(1000), true)
);

fn check_etime(cases: &[(&str, Option<u64>)]) {
    for (text, secs) in cases {
        assert_eq!(parse_etime(text), *secs, "etime misread: {text:?}");
    }
}

#[test]
fn jobq_parses_ps_elapsed_times() {
    check_etime(&[
        ("00:05", Some(5)),
        ("   01:02\n", Some(62)),
        ("1:00:00", Some(3600)),
        ("2-03:04:05", Some(2 * 86_400 + 3 * 3600 + 4 * 60 + 5)),
        ("10-00:00", Some(864_000)),
        ("5", None),
        ("1:2:3:4", None),
        ("1::05", None),
        ("+1:00", None),
        ("x-01:00", None),
        ("", None),
    ]);
}

negative_control!(
    jobq_parses_ps_elapsed_times,
    "hours, minutes and seconds are not minutes and seconds",
    expected = "etime misread",
    check_etime(&[("1:00:00", Some(60))])
);

// --- Heavy processes outside the queue are logged -----------------------------------------------------------------

/// Runs a stand-in `rustc` (a link to `sleep`) outside the queue (with `outside`) and one inside it, and checks the
/// violations log names the outside one and not the inside one.
fn violation_case(tag: &str, outside: bool) {
    let q = Q::new(tag, "");
    let bin = q.dir.join("bin-standin");
    std::fs::create_dir_all(&bin).expect("bin dir");
    let rustc = bin.join("rustc");
    std::os::unix::fs::symlink("/bin/sleep", &rustc).expect("stand-in linked");
    let rustc = rustc.to_str().expect("utf-8 path").to_owned();
    let stray = outside.then(|| {
        let out = Command::new("sh")
            .args(["-c", "\"$0\" 30 >/dev/null 2>&1 & echo $!", &rustc])
            .output_within(Duration::from_secs(30))
            .expect("stand-in started");
        String::from_utf8_lossy(&out.stdout)
            .trim()
            .parse::<i32>()
            .expect("a pid")
    });
    let id = q.submit(
        "peek",
        "inside",
        &[],
        &["sh", "-c", "exec \"$0\" 30", &rustc],
    );
    let inside = {
        let queue = q.queue();
        assert!(
            until(30, || queue.load(id).is_ok_and(|j| j.pgid.is_some())),
            "job never started"
        );
        queue.load(id).expect("job file").pgid.expect("its group")
    };
    let log = q.dir.join("violations.log");
    let read = || std::fs::read_to_string(&log).unwrap_or_default();
    // Two samples at least, a second apart, with the job running.
    let named = |pid: i32| read().lines().any(|l| l.contains(&format!("pid={pid} ")));
    until(6, || stray.is_some_and(named));
    std::thread::sleep(Duration::from_millis(2500));
    let (stray_named, inside_named) = (stray.is_some_and(named), named(inside));
    q.run(&["cancel", &id.to_string()]);
    q.wait(id);
    if let Some(pid) = stray {
        kill("-9", pid);
    }
    q.finish();
    assert!(
        stray_named && !inside_named,
        "violations wrong: outside {stray:?} named {stray_named}, inside {inside} named {inside_named}:\n{}",
        read()
    );
}

#[test]
fn jobq_logs_heavy_processes_outside_the_queue_only() {
    violation_case("jobq_violations", true);
}

negative_control!(
    jobq_logs_heavy_processes_outside_the_queue_only,
    "with no heavy process outside the queue, nothing outside is named",
    expected = "violations wrong",
    violation_case("jobq_violations_control", false)
);

fn proc(pid: i32, ppid: i32, pgid: i32, exe: &str, args: &str) -> Proc {
    Proc {
        pid,
        ppid,
        pgid,
        exe: exe.to_owned(),
        args: args.to_owned(),
    }
}

/// `heavy` on each (exe, args) gives its expected verdict.
fn check_heavy(cases: &[(&str, &str, bool)]) {
    for (exe, args, expected) in cases {
        assert_eq!(
            heavy(&proc(10, 1, 10, exe, args)),
            *expected,
            "heavy wrong for {args:?}"
        );
    }
}

const TC: &str = "/Users/u/.rustup/toolchains/nightly/bin";

#[test]
fn jobq_heavy_processes_are_told_apart() {
    check_heavy(&[
        ("/opt/homebrew/bin/cargo", "cargo build -p kernel", true),
        (
            "/opt/homebrew/bin/cargo",
            "cargo +nightly test -p kernel",
            true,
        ),
        (
            "/opt/homebrew/bin/cargo",
            "cargo -Z unstable-options --config x=1 clippy",
            true,
        ),
        (
            "/opt/homebrew/bin/cargo",
            "cargo --color always check",
            true,
        ),
        ("/opt/homebrew/bin/cargo", "cargo -C /some/dir doc", true),
        ("/opt/homebrew/bin/cargo", "cargo mutants --in-diff d", true),
        ("/opt/homebrew/bin/cargo", "cargo nextest run", true),
        ("/opt/homebrew/bin/cargo", "cargo xtask ci", true),
        ("/opt/homebrew/bin/cargo", "cargo xtask lint vocab", true),
        ("/opt/homebrew/bin/cargo", "cargo xtask job wait 3", false),
        ("/opt/homebrew/bin/cargo", "cargo xtask", true),
        ("/opt/homebrew/bin/cargo", "cargo fmt --check", false),
        (
            "/opt/homebrew/bin/cargo",
            "cargo metadata --format-version 1",
            false,
        ),
        ("/opt/homebrew/bin/cargo", "cargo --version", false),
        ("/opt/homebrew/bin/cargo", "cargo", false),
        (
            &format!("{TC}/rustc"),
            &format!("{TC}/rustc --crate-name kernel src/lib.rs"),
            true,
        ),
        (&format!("{TC}/rustc"), &format!("{TC}/rustc -vV"), false),
        (&format!("{TC}/rustc"), &format!("{TC}/rustc -V"), false),
        (
            &format!("{TC}/rustc"),
            &format!("{TC}/rustc --version"),
            false,
        ),
        (
            &format!("{TC}/rustc"),
            &format!("{TC}/rustc - --crate-name ___ --print=file-names"),
            false,
        ),
        (
            &format!("{TC}/rustdoc"),
            "rustdoc --edition 2021 src/lib.rs",
            true,
        ),
        (
            &format!("{TC}/clippy-driver"),
            "clippy-driver rustc --crate-name x",
            true,
        ),
        (
            "/Users/u/.cargo/bin/cargo-mutants",
            "/Users/u/.cargo/bin/cargo-mutants mutants -j 2",
            true,
        ),
        (
            "/Users/u/.cargo/bin/cargo-nextest",
            "cargo-nextest nextest run",
            true,
        ),
        (
            "/Users/u/.cargo/bin/cargo-clippy",
            "cargo-clippy clippy",
            true,
        ),
        (
            "/Volumes/X10 Pro/t/debug/xtask",
            "/Volumes/X10 Pro/t/debug/xtask controls --list",
            true,
        ),
        (
            "/Volumes/X10 Pro/t/debug/xtask",
            "/Volumes/X10 Pro/t/debug/xtask gate --all",
            true,
        ),
        (
            "/Volumes/X10 Pro/t/debug/xtask",
            "/Volumes/X10 Pro/t/debug/xtask job runner",
            false,
        ),
        (
            "/Volumes/X10 Pro/t/debug/xtask",
            "/Volumes/X10 Pro/t/debug/xtask",
            false,
        ),
        (
            "/Volumes/X10 Pro/t/debug/deps/kernel-0123456789abcdef",
            "x --test-threads 4",
            true,
        ),
        (
            "/Volumes/X10 Pro/t/debug/deps/kernel-0123456789abcdeg",
            "x",
            false,
        ),
        (
            "/Volumes/X10 Pro/t/debug/deps/kernel-0123456789abcde",
            "x",
            false,
        ),
        (
            "/Volumes/X10 Pro/t/debug/build/kernel-0123456789abcdef",
            "x",
            false,
        ),
        ("/bin/sleep", "sleep 30", false),
    ]);
}

negative_control!(
    jobq_heavy_processes_are_told_apart,
    "cargo fmt compiles nothing",
    expected = "heavy wrong",
    check_heavy(&[("/opt/homebrew/bin/cargo", "cargo fmt --check", true)])
);

/// `violations` on `procs`, with the queue's groups `groups` and runner 900, names exactly `expected`.
fn check_violations(procs: &[Proc], groups: &[i32], expected: &[i32]) {
    let groups: BTreeSet<i32> = groups.iter().copied().collect();
    let found: Vec<i32> = violations(procs, &groups, 900)
        .iter()
        .map(|p| p.pid)
        .collect();
    assert_eq!(found, expected, "violations named wrong");
}

#[test]
fn jobq_violations_skip_the_queue_and_name_the_top_of_a_chain() {
    let cargo = "/opt/homebrew/bin/cargo";
    let rustc = "/tc/bin/rustc";
    let procs = [
        proc(100, 1, 100, "/bin/zsh", "zsh"),
        // Outside: a cargo under a shell, and its rustc (only the cargo is named).
        proc(101, 100, 100, cargo, "cargo test -p kernel"),
        proc(102, 101, 100, rustc, "rustc --crate-name kernel"),
        // Inside: a job's group, and a child that left it, by ancestry.
        proc(200, 900, 200, cargo, "cargo build"),
        proc(201, 200, 201, rustc, "rustc --crate-name a"),
        // The runner's own child, before its group is recorded.
        proc(300, 900, 300, cargo, "cargo check"),
        // sccache's server runs compilers it is handed.
        proc(400, 1, 400, "/opt/homebrew/bin/sccache", "sccache"),
        proc(401, 400, 400, rustc, "rustc --crate-name b"),
        // Outside, under a light parent, and an orphan.
        proc(500, 1, 500, cargo, "cargo fmt"),
        proc(501, 500, 500, rustc, "rustc --crate-name c"),
        proc(600, 1, 600, rustc, "rustc --crate-name d"),
        // A parent missing from the table.
        proc(700, 699, 700, cargo, "cargo clippy"),
    ];
    check_violations(&procs, &[200], &[101, 501, 600, 700]);
    // Without the job's group recorded, its cargo is still inside, as the runner's child, and so is its rustc.
    check_violations(&procs[..5], &[], &[101]);
}

negative_control!(
    jobq_violations_skip_the_queue_and_name_the_top_of_a_chain,
    "a job's group is inside the queue",
    expected = "violations named wrong",
    check_violations(
        &[proc(200, 2, 200, "/opt/homebrew/bin/cargo", "cargo build")],
        &[200],
        &[200]
    )
);

/// `parse_ps` joins the two readings, keeping spaces in paths and command lines.
fn check_parse_ps(ids: &str, args: &str, expected: &[Proc]) {
    assert_eq!(parse_ps(ids, args), expected, "ps misread");
}

#[test]
fn jobq_parses_ps_with_spaces_in_paths() {
    let ids =
        "  12     1    12 /Volumes/X10 Pro/t/debug/xtask\n   13   12   12 /bin/sh\nnot a line\n";
    let args = "   12 /Volumes/X10 Pro/t/debug/xtask ci --list\n   13 sh -c a b\n";
    check_parse_ps(
        ids,
        args,
        &[
            proc(
                12,
                1,
                12,
                "/Volumes/X10 Pro/t/debug/xtask",
                "/Volumes/X10 Pro/t/debug/xtask ci --list",
            ),
            proc(13, 12, 12, "/bin/sh", "sh -c a b"),
        ],
    );
    // A pid missing from the second reading keeps its executable as its command line.
    check_parse_ps(
        "  14 1 14 /bin/ls\n",
        "",
        &[proc(14, 1, 14, "/bin/ls", "/bin/ls")],
    );
}

negative_control!(
    jobq_parses_ps_with_spaces_in_paths,
    "a path with a space is one executable, not two",
    expected = "ps misread",
    check_parse_ps("  14 1 14 /a b\n", "", &[proc(14, 1, 14, "/a", "/a")])
);

// --- The rest: the config, submit's arguments, the environment, the statuses --------------------------------------

/// The config text parses to `expected`, or fails naming `error`.
fn check_config(text: &str, expected: Result<(usize, u64, u64), &str>) {
    let got = Config::parse(text);
    match (got, expected) {
        (Ok(config), Ok(values)) => assert_eq!(
            (config.concurrency, config.timeout, config.tick_ms),
            values,
            "config misread"
        ),
        (Err(message), Err(error)) => assert!(message.contains(error), "config misread: {message}"),
        (got, expected) => panic!("config misread: {got:?}, expected {expected:?}"),
    }
}

#[test]
fn jobq_config_reads_each_key() {
    check_config("", Ok((2, 3600, 500)));
    check_config(
        "concurrency = 3\ntimeout=60 # a comment\n\ntick_ms = 1\n",
        Ok((3, 60, 1)),
    );
    check_config("concurrency = 0", Err("at least 1"));
    check_config("concurrency = two", Err("not a whole number"));
    check_config("speed = 2", Err("unknown key"));
    check_config("concurrency", Err("not key = value"));
    check_config("env RUSTC_WRAPPER", Err("env takes KEY=VALUE"));
    let config = Config::parse(
        "stale = 5\nsample = 6\nidle = 7\ngrace = 8\ncores = 9\nlock_wait = 4\nenv RUSTC_WRAPPER = sccache\n",
    )
    .expect("config");
    assert_eq!(
        (
            config.stale,
            config.sample,
            config.idle,
            config.grace,
            config.cores,
            config.lock_wait
        ),
        (5, 6, 7, 8, 9, 4),
        "config misread"
    );
    let defaults = config.job_defaults();
    assert_eq!(
        (
            defaults.get("CARGO_BUILD_JOBS").map(String::as_str),
            defaults.get("RUST_TEST_THREADS").map(String::as_str),
            defaults.get("RUSTC_WRAPPER").map(String::as_str)
        ),
        (Some("9"), Some("9"), Some("sccache")),
        "config misread: job defaults"
    );
    assert_eq!(Config::default().lock_wait, 30, "config misread: lock_wait");
    let cores = std::thread::available_parallelism().map_or(1, |n| n.get());
    assert_eq!(
        Config::default().cores,
        cores,
        "config misread: the core count"
    );
}

negative_control!(
    jobq_config_reads_each_key,
    "a concurrency of 0 is refused",
    expected = "config misread",
    check_config("concurrency = 0", Ok((0, 3600, 500)))
);

/// A job's environment: the runner's defaults, the job's own over them, and its id.
fn check_env(job_env: &[(&str, &str)], key: &str, expected: &str) {
    let mut j = job(4, Priority::Peek, State::Queued, None);
    j.env = job_env
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect();
    let env = j.env_for(&Config::parse("cores = 11").expect("config").job_defaults());
    assert_eq!(
        env.get(key).map(String::as_str),
        Some(expected),
        "job env wrong for {key}"
    );
}

#[test]
fn jobq_job_env_has_the_core_count_unless_it_sets_its_own() {
    check_env(&[], "CARGO_BUILD_JOBS", "11");
    check_env(&[], "RUST_TEST_THREADS", "11");
    check_env(&[("RUST_TEST_THREADS", "1")], "RUST_TEST_THREADS", "1");
    check_env(&[], "PRINCIPIA_JOBQ_JOB", "4");
    let kept = snapshot_env(
        [
            ("PATH", "/bin"),
            ("HOME", "/h"),
            ("LC_ALL", "C"),
            ("CARGO_HOME", "/c"),
            ("CARGO_INCREMENTAL", "0"),
            ("RUSTUP_HOME", "/r"),
            ("RUSTUP_TOOLCHAIN", "stable"),
            ("RUSTFLAGS", "-D x"),
            ("SCCACHE_DIR", "/s"),
            ("CARGO", "/bin/cargo"),
            ("CARGO_MANIFEST_DIR", "/m"),
            ("CARGO_PKG_NAME", "xtask"),
            ("CARGO_BIN_EXE_xtask", "/x"),
            ("CARGO_TARGET_DIR", "/t"),
            ("CARGO_BUILD_JOBS", "4"),
            ("RUST_TEST_THREADS", "4"),
            ("GH_TOKEN", "secret"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned())),
    );
    let keys: Vec<&str> = kept.keys().map(String::as_str).collect();
    assert_eq!(
        keys,
        [
            "CARGO_HOME",
            "CARGO_INCREMENTAL",
            "HOME",
            "LC_ALL",
            "PATH",
            "RUSTFLAGS",
            "RUSTUP_HOME",
            "SCCACHE_DIR"
        ],
        "job env wrong: the submitter's snapshot"
    );
}

negative_control!(
    jobq_job_env_has_the_core_count_unless_it_sets_its_own,
    "a job's own RUST_TEST_THREADS wins over the core count",
    expected = "job env wrong",
    check_env(&[("RUST_TEST_THREADS", "1")], "RUST_TEST_THREADS", "11")
);

/// `parse_submit` on `args` fails naming `error`, or reads (priority, label, timeout, env, command).
fn check_submit(args: &[&str], error: Option<&str>) {
    match (parse_submit(args), error) {
        (Err(message), Some(error)) => {
            assert!(message.contains(error), "submit misread: {message}")
        }
        (Ok(_), None) => {}
        (got, _) => panic!("submit misread: {got:?}"),
    }
}

#[test]
fn jobq_submit_arguments() {
    let request = parse_submit(&[
        "/w",
        "--priority",
        "fix",
        "--label",
        "T-1",
        "--timeout",
        "5",
        "--env",
        "A=b=c",
        "--",
        "cargo",
        "--",
        "x",
    ])
    .expect("parsed");
    assert_eq!(
        (
            request.worktree,
            request.priority,
            request.label.as_str(),
            request.timeout,
            request.env.get("A").map(String::as_str),
            request.command
        ),
        (
            PathBuf::from("/w"),
            Priority::Fix,
            "T-1",
            Some(5),
            Some("b=c"),
            vec!["cargo".to_owned(), "--".to_owned(), "x".to_owned()]
        ),
        "submit misread"
    );
    check_submit(
        &["/w", "--priority", "peek", "--label", "l", "--", "true"],
        None,
    );
    check_submit(
        &["/w", "--priority", "peek", "--label", "l"],
        Some("the command follows --"),
    );
    check_submit(
        &["--priority", "peek", "--", "true"],
        Some("the worktree comes first"),
    );
    check_submit(
        &["/w", "--label", "l", "--", "true"],
        Some("--priority is required"),
    );
    check_submit(
        &["/w", "--priority", "peek", "--", "true"],
        Some("--label is required"),
    );
    check_submit(
        &["/w", "--priority", "urgent", "--label", "l", "--", "true"],
        Some("merge, fix, build or peek"),
    );
    check_submit(
        &[
            "/w",
            "--priority",
            "peek",
            "--label",
            "l",
            "--timeout",
            "0",
            "--",
            "true",
        ],
        Some("whole seconds"),
    );
    check_submit(
        &[
            "/w",
            "--priority",
            "peek",
            "--label",
            "l",
            "--timeout",
            "x",
            "--",
            "true",
        ],
        Some("whole seconds"),
    );
    check_submit(
        &[
            "/w",
            "--priority",
            "peek",
            "--label",
            "l",
            "--env",
            "=v",
            "--",
            "true",
        ],
        Some("KEY=VALUE"),
    );
    check_submit(
        &[
            "/w",
            "--priority",
            "peek",
            "--label",
            "l",
            "--env",
            "v",
            "--",
            "true",
        ],
        Some("KEY=VALUE"),
    );
    check_submit(
        &["/w", "--priority", "peek", "--label", "--", "true"],
        Some("--label takes a value"),
    );
    check_submit(
        &[
            "/w",
            "--priority",
            "peek",
            "--label",
            "l",
            "--bogus",
            "1",
            "--",
            "true",
        ],
        Some("unrecognised"),
    );
    check_submit(&["/w", "--priority"], Some("the command follows --"));
    check_submit(
        &["/w", "--priority", "--", "x"],
        Some("--priority takes a value"),
    );
    for (name, class) in [
        ("merge", Priority::Merge),
        ("fix", Priority::Fix),
        ("build", Priority::Build),
        ("peek", Priority::Peek),
    ] {
        assert_eq!(
            Priority::parse(name),
            Ok(class),
            "submit misread: class {name}"
        );
        assert_eq!(class.name(), name, "submit misread: class name {name}");
    }
}

negative_control!(
    jobq_submit_arguments,
    "a timeout of 0 is refused",
    expected = "submit misread",
    check_submit(
        &[
            "/w",
            "--priority",
            "peek",
            "--label",
            "l",
            "--timeout",
            "0",
            "--",
            "true"
        ],
        None
    )
);

/// The binary refuses `args`, naming `error`.
fn check_refused(q: &Q, args: &[&str], error: &str) {
    let out = q.run(args);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !out.status.success() && stderr.contains(error),
        "submit not refused: {stderr}"
    );
}

#[test]
fn jobq_submit_refuses_a_job_it_cannot_run() {
    let q = Q::new("jobq_refused", "");
    let work = q.work();
    let work = work.to_str().expect("utf-8 path");
    let missing = q.dir.join("missing");
    let missing = missing.to_str().expect("utf-8 path");
    let target = "CARGO_TARGET_DIR=/t";
    check_refused(
        &q,
        &[
            "submit",
            work,
            "--priority",
            "peek",
            "--label",
            "l",
            "--",
            "true",
        ],
        "CARGO_TARGET_DIR",
    );
    for (worktree, label, error) in [
        (missing, "l", "not a directory"),
        (work, " ", "--label takes"),
        (work, "a\nb", "--label takes"),
    ] {
        let args = [
            "submit",
            worktree,
            "--priority",
            "peek",
            "--label",
            label,
            "--env",
            target,
            "--",
            "true",
        ];
        check_refused(&q, &args, error);
    }
    check_refused(
        &q,
        &[
            "submit",
            work,
            "--priority",
            "peek",
            "--label",
            "l",
            "--env",
            target,
            "--",
        ],
        "no command",
    );
    check_refused(&q, &["wait", "x"], "a job id is a number");
    check_refused(&q, &["wait", "99"], "cannot read");
    check_refused(&q, &["cancel", "x"], "a job id is a number");
    check_refused(&q, &["frobnicate"], "unrecognised arguments");
    assert!(
        !q.dir.join("jobs").join("000001.json").exists(),
        "submit not refused: a job was written"
    );
}

negative_control!(
    jobq_submit_refuses_a_job_it_cannot_run,
    "a job with its target directory is accepted",
    expected = "submit not refused",
    {
        let q = Q::new("jobq_refused_control", "");
        let work = q.work();
        let work = work.to_str().expect("utf-8 path");
        let args = [
            "submit",
            work,
            "--priority",
            "peek",
            "--label",
            "l",
            "--env",
            "CARGO_TARGET_DIR=/t",
            "--",
            "true",
        ];
        let refused = std::panic::catch_unwind(|| check_refused(&q, &args, "CARGO_TARGET_DIR"));
        q.run(&["wait", "1"]);
        q.finish();
        if let Err(panic) = refused {
            std::panic::resume_unwind(panic);
        }
    }
);

/// `job wait`'s status for each end, and the status, cancel and install commands, on a live queue.
fn check_statuses(q: &Q, cases: &[(&[&str], i32)]) {
    for (command, expected) in cases {
        let id = q.submit("peek", "status", &[], command);
        assert_eq!(q.wait(id).0, *expected, "wait status wrong for {command:?}");
    }
}

#[test]
fn jobq_wait_status_cancel_and_install() {
    let q = Q::new("jobq_status", "");
    // A job file that can't be read is skipped, not fatal.
    std::fs::create_dir_all(q.dir.join("jobs")).expect("jobs dir");
    std::fs::write(q.dir.join("jobs").join("000999.json"), "not json").expect("bad job file");
    check_statuses(
        &q,
        &[
            (&["true"], 0),
            (&["sh", "-c", "exit 7"], 7),
            (&["sh", "-c", "kill -9 $$"], 137),
            (&["/nonexistent/command"], 126),
        ],
    );
    // `wait` says how the job ended and how long it ran; the runner's log has its start and end.
    let id = q.submit("peek", "seven", &[], &["sh", "-c", "exit 7"]);
    let (_, out) = q.wait(id);
    let runner_log = std::fs::read_to_string(q.dir.join("runner.log")).unwrap_or_default();
    assert!(
        out.starts_with(&format!("job {id} (seven): exited 7 after "))
            && runner_log.contains(&format!("job {id}: started"))
            && runner_log.contains(&format!("job {id}: exited 7 after ")),
        "wait status wrong: wait printed {out:?}, the runner logged {runner_log:?}"
    );
    // A queued job cancelled behind a running one; then the running one.
    let running = q.submit("peek", "hold", &[], &["sleep", "60"]);
    let queued = q.submit("peek", "behind", &[], &["true"]);
    assert!(
        until(30, || q
            .queue()
            .load(running)
            .is_ok_and(|j| j.state == State::Running)),
        "never ran"
    );
    let status = String::from_utf8_lossy(&q.run(&["status"]).stdout).into_owned();
    assert!(
        status.contains("runner: pid") && status.contains("sleep 60") && status.contains("behind"),
        "wait status wrong: status lists {status}"
    );
    let out = q.run(&["cancel", &queued.to_string()]);
    assert!(out.status.success(), "wait status wrong: cancel failed");
    assert_eq!(
        q.wait(queued).0,
        130,
        "wait status wrong: a cancelled queued job"
    );
    let out = q.run(&["cancel", &queued.to_string()]);
    assert!(
        !out.status.success(),
        "wait status wrong: a second cancel succeeded"
    );
    q.run(&["cancel", &running.to_string()]);
    let (code, out) = q.wait(running);
    assert!(
        code == 130 && out.contains("log: "),
        "wait status wrong: a cancelled running job: {code} {out}"
    );
    let status = String::from_utf8_lossy(&q.run(&["status"]).stdout).into_owned();
    assert!(
        status.contains("cancelled"),
        "wait status wrong: status lists {status}"
    );
    let out = q.run(&["install"]);
    assert!(
        out.status.success() && q.dir.join("bin").join("xtask").exists(),
        "wait status wrong: install"
    );
    q.finish();
    let status = String::from_utf8_lossy(&q.run(&["status"]).stdout).into_owned();
    assert!(
        status.starts_with("runner: none"),
        "wait status wrong: after idle, status lists {status}"
    );
    assert_eq!(
        utc(1_793_367_985),
        "2026-10-30T13:46:25Z",
        "wait status wrong: the log's time"
    );
    assert_eq!(
        utc(951_782_400),
        "2000-02-29T00:00:00Z",
        "wait status wrong: the log's time"
    );
}

negative_control!(
    jobq_wait_status_cancel_and_install,
    "a job exiting 7 is not a success",
    expected = "wait status wrong",
    {
        let q = Q::new("jobq_status_control", "");
        let checked =
            std::panic::catch_unwind(|| check_statuses(&q, &[(&["sh", "-c", "exit 7"], 0)]));
        q.finish();
        if let Err(panic) = checked {
            std::panic::resume_unwind(panic);
        }
    }
);

// --- The state lock is waited for within a bound, naming its holder ------------------------------------------------

/// Takes the state lock in this test process, as a hung holder would keep it, with `pid 4242` and a time 100 s back
/// written in it (with `hold`; else leaves it free).
fn take_state_lock(q: &Q, hold: bool) -> Option<std::fs::File> {
    hold.then(|| {
        let path = q.dir.join("state.lock");
        let file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&path)
            .expect("state lock opened");
        file.lock().expect("state lock taken");
        let text = format!("pid 4242\nsince {}\n", xtask::jobq::now() - 100);
        std::fs::write(&path, text).expect("holder written");
        file
    })
}

/// With the state lock held (with `hold`) and `lock_wait = 1`, `cancel` of a queued job must give up within a few
/// seconds, naming the holder's pid and how long it has held the lock, and leave the job queued.
fn held_lock_case(tag: &str, hold: bool) {
    let q = Q::new(tag, "lock_wait = 1\n");
    std::fs::create_dir_all(q.dir.join("jobs")).expect("jobs dir");
    let mut queued = job(1, Priority::Peek, State::Queued, Some("/t"));
    queued.worktree = q.work();
    q.queue().save(&queued).expect("job planted");
    let held = take_state_lock(&q, hold);
    let started = Instant::now();
    let out = q.run_within(&["cancel", "1"], Duration::from_secs(20));
    let took = started.elapsed();
    drop(held);
    let stderr = out
        .as_ref()
        .map(|out| String::from_utf8_lossy(&out.stderr).into_owned())
        .unwrap_or_default();
    let state = q.queue().load(1).expect("job file").state;
    assert!(
        out.as_ref().is_ok_and(|out| !out.status.success())
            && stderr.contains("held by pid 4242, for 10")
            && took < Duration::from_secs(10)
            && state == State::Queued,
        "state lock wait unbounded or its holder unnamed: {out:?}, after {took:?}, job {state:?}"
    );
}

#[test]
fn jobq_a_held_state_lock_is_waited_for_within_a_bound() {
    held_lock_case("jobq_held_lock", true);
    let now = 1_000;
    let cases = [
        ("pid 7\nsince 900\n", "pid 7, for 100 s"),
        ("pid 7\nsince 1000\n", "pid 7, for 0 s"),
        ("pid 7\nsince 1001\n", "pid 7, for 0 s"),
        ("pid 7\n", "pid 7"),
        ("since 900\n", "a process that does not say which"),
        ("", "a process that does not say which"),
    ];
    for (text, expected) in cases {
        assert_eq!(
            lock_holder(text, now),
            expected,
            "state lock wait unbounded or its holder unnamed: {text:?}"
        );
    }
}

negative_control!(
    jobq_a_held_state_lock_is_waited_for_within_a_bound,
    "with the lock free, the cancel succeeds",
    expected = "state lock wait unbounded or its holder unnamed",
    held_lock_case("jobq_held_lock_control", false)
);

/// A runner whose state lock is held (with `hold`) for 3 s by a hung holder, with `lock_wait = 1`, must log that it
/// is waiting, naming the holder, and run the queued job once the lock is free.
fn runner_waits_case(tag: &str, hold: bool) {
    let q = Q::new(tag, "lock_wait = 1\nidle = 3\n");
    std::fs::create_dir_all(q.dir.join("jobs")).expect("jobs dir");
    let log = std::fs::File::create(q.dir.join("runner.log")).expect("runner log");
    let mut runner = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["job", "runner"])
        .arg(&*q.dir)
        .env("PRINCIPIA_JOBQ", &*q.dir)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(log)
        .spawn()
        .expect("runner started");
    let queue = q.queue();
    assert!(
        until(30, || queue.runner_lock().is_some()),
        "the runner never took its lock"
    );
    let held = take_state_lock(&q, hold);
    let mut planted = job(1, Priority::Peek, State::Queued, Some("/t"));
    planted.worktree = q.work();
    queue.save(&planted).expect("job planted");
    std::thread::sleep(Duration::from_secs(3));
    let waiting = queue.load(1).expect("job file").state == State::Queued;
    drop(held);
    let ran = until(20, || {
        queue
            .load(1)
            .is_ok_and(|j| j.state == State::Exited && j.exit == Some(0))
    });
    let exited = until(20, || runner.try_wait().is_ok_and(|s| s.is_some()));
    if !exited {
        let _ = runner.kill();
        let _ = runner.wait();
    }
    let text = std::fs::read_to_string(q.dir.join("runner.log")).unwrap_or_default();
    assert!(
        waiting && ran && exited && text.contains("held by pid 4242, for 10"),
        "runner did not wait out a held state lock: queued while held {waiting}, ran {ran}, exited {exited}, \
         log {text:?}"
    );
}

#[test]
fn jobq_a_runner_waits_out_a_held_state_lock() {
    runner_waits_case("jobq_runner_waits", true);
}

negative_control!(
    jobq_a_runner_waits_out_a_held_state_lock,
    "with the lock free, the job runs at once and nothing is logged",
    expected = "runner did not wait out a held state lock",
    runner_waits_case("jobq_runner_waits_control", false)
);

// --- Status shows the runner's own settings ----------------------------------------------------------------------------

/// With a live runner lock (this test's pid, a fresh heartbeat) that gives `concurrency 7` and `cores 9` (with
/// `settings`), and a config that says otherwise, `status` must show the lock's settings, not the config's.
fn status_settings_case(tag: &str, settings: bool) {
    let q = Q::new(tag, "concurrency = 1\ncores = 2\n");
    std::fs::create_dir_all(q.dir.join("jobs")).expect("jobs dir");
    let extra = if settings {
        "concurrency 7\ncores 9\n"
    } else {
        ""
    };
    let lock = format!(
        "pid {}\nheartbeat {}\n{extra}",
        std::process::id(),
        xtask::jobq::now()
    );
    std::fs::write(q.dir.join("runner.lock"), lock).expect("lock planted");
    let status = String::from_utf8_lossy(&q.run(&["status"]).stdout).into_owned();
    std::fs::remove_file(q.dir.join("runner.lock")).expect("lock removed");
    assert!(
        status.contains("concurrency 7, 9 cores per job"),
        "status shows the client's settings: {status}"
    );
}

#[test]
fn jobq_status_shows_the_runners_own_settings() {
    status_settings_case("jobq_status_settings", true);
    assert_eq!(
        runner_settings("pid 1\nheartbeat 2\n"),
        "concurrency ?, ? cores per job",
        "status shows the client's settings: an older runner's lock"
    );
}

negative_control!(
    jobq_status_shows_the_runners_own_settings,
    "a lock that gives no settings shows none",
    expected = "status shows the client's settings",
    status_settings_case("jobq_status_settings_control", false)
);

// --- Ended jobs move to the archive a day later ----------------------------------------------------------------------

/// Plants an ended job that finished `age` seconds ago, runs a job through the runner, and checks the old job's file
/// is in the archive exactly when `archived`, and that `wait` still reads it.
fn archive_case(tag: &str, age: u64, archived: bool) {
    let q = Q::new(tag, "");
    std::fs::create_dir_all(q.dir.join("jobs")).expect("jobs dir");
    let mut old = job(1, Priority::Peek, State::Exited, Some("/t"));
    old.exit = Some(3);
    old.finished = Some(xtask::jobq::now() - age);
    q.queue().save(&old).expect("job planted");
    // With no `next_id` file, the next id follows the highest job's.
    let id = q.submit("peek", "now", &[], &["true"]);
    assert_eq!(id, 2, "archive wrong: the next id");
    q.wait(id);
    let (code, _) = q.wait(1);
    let moved = q.queue().archive_path(1).exists() && !q.queue().job_path(1).exists();
    q.finish();
    assert!(
        moved == archived && code == 3,
        "archive wrong: moved {moved}, wait status {code}"
    );
}

#[test]
fn jobq_archives_a_job_a_day_after_it_ends() {
    archive_case("jobq_archive", ARCHIVE_AFTER + 60, true);
    archive_case("jobq_archive_recent", ARCHIVE_AFTER - 600, false);
    let mut ended = job(1, Priority::Peek, State::Cancelled, None);
    ended.finished = Some(1_000);
    let at = |now| archivable(&ended, now);
    assert!(
        at(1_000 + ARCHIVE_AFTER) && !at(999 + ARCHIVE_AFTER),
        "archive wrong at the boundary"
    );
    let mut running = job(2, Priority::Peek, State::Running, None);
    running.finished = Some(1_000);
    assert!(
        !archivable(&running, 1_000 + ARCHIVE_AFTER),
        "archive wrong: a running job"
    );
    assert!(
        !archivable(&job(3, Priority::Peek, State::Exited, None), u64::MAX),
        "archive wrong: no end time"
    );
}

negative_control!(
    jobq_archives_a_job_a_day_after_it_ends,
    "a job that ended within the day stays in jobs/",
    expected = "archive wrong",
    archive_case("jobq_archive_control", ARCHIVE_AFTER - 600, true)
);

/// `Queue::config` with no config file gives the defaults; with one it can't read, an error naming the config.
fn check_config_file(tag: &str, make_unreadable: bool) {
    let dir = Scratch::new(tag);
    std::fs::create_dir_all(&*dir).expect("scratch made");
    if make_unreadable {
        std::fs::create_dir_all(dir.join("config")).expect("config made a folder");
    }
    match (Queue::at(dir.to_path_buf()).config(), make_unreadable) {
        (Ok(config), false) => assert_eq!(
            config,
            Config::default(),
            "config file misread: not the defaults"
        ),
        (Err(message), true) => assert!(
            message.contains("cannot read the config"),
            "config file misread: {message}"
        ),
        (got, _) => panic!("config file misread: {got:?}"),
    }
}

#[test]
fn jobq_config_file_missing_or_unreadable() {
    check_config_file("jobq_config_missing", false);
    check_config_file("jobq_config_unreadable", true);
}

negative_control!(
    jobq_config_file_missing_or_unreadable,
    "a config that is a folder is no config file",
    expected = "config file misread",
    {
        let dir = Scratch::new("jobq_config_control");
        std::fs::create_dir_all(dir.join("config")).expect("config made a folder");
        assert!(
            Queue::at(dir.to_path_buf()).config().is_ok(),
            "config file misread: refused"
        );
    }
);

/// `Queue::jobs` with no `jobs/` folder lists none; with a `jobs` it can't list, an error.
fn check_jobs_dir(tag: &str, make_unlistable: bool) {
    let dir = Scratch::new(tag);
    std::fs::create_dir_all(&*dir).expect("scratch made");
    if make_unlistable {
        std::fs::write(dir.join("jobs"), "a file").expect("jobs made a file");
    }
    match (Queue::at(dir.to_path_buf()).jobs(), make_unlistable) {
        (Ok(jobs), false) => assert!(jobs.is_empty(), "jobs folder misread: {jobs:?}"),
        (Err(message), true) => assert!(
            message.contains("cannot list the jobs"),
            "jobs folder misread: {message}"
        ),
        (got, _) => panic!("jobs folder misread: {got:?}"),
    }
}

#[test]
fn jobq_jobs_folder_missing_or_unlistable() {
    check_jobs_dir("jobq_jobs_missing", false);
    check_jobs_dir("jobq_jobs_unlistable", true);
}

negative_control!(
    jobq_jobs_folder_missing_or_unlistable,
    "a jobs file is no jobs folder",
    expected = "jobs folder misread",
    {
        let dir = Scratch::new("jobq_jobs_control");
        std::fs::create_dir_all(&*dir).expect("scratch made");
        std::fs::write(dir.join("jobs"), "a file").expect("jobs made a file");
        assert!(
            Queue::at(dir.to_path_buf()).jobs().is_ok(),
            "jobs folder misread: refused"
        );
    }
);

// --- A second runner leaves a live one alone -------------------------------------------------------------------------

/// With a job running under a live runner, `job runner` started again (with `planted`, after the live lock is replaced
/// by one of a dead pid) must exit at once, the lock and the job untouched.
fn second_runner_case(tag: &str, expire_first: bool) {
    let q = Q::new(tag, "stale = 60\n");
    let id = q.submit("peek", "hold", &[], &["sleep", "60"]);
    assert!(
        until(30, || q
            .queue()
            .load(id)
            .is_ok_and(|j| j.state == State::Running)),
        "never ran"
    );
    let first = q.queue().runner_lock().expect("a live runner").pid;
    if expire_first {
        let lock = format!("pid {}\nheartbeat {}\n", dead_pid(), xtask::jobq::now());
        std::fs::write(q.dir.join("runner.lock"), lock).expect("lock replaced");
    }
    let dir = q.dir.to_str().expect("utf-8 path").to_owned();
    let second = q.run_within(&["runner", &dir], Duration::from_secs(5));
    let left = second.as_ref().is_ok_and(|out| out.status.success());
    let job = q.queue().load(id).expect("job file");
    q.run(&["cancel", &id.to_string()]);
    q.wait(id);
    q.finish();
    assert!(
        left && job.attempts == 1 && job.runner_pid == Some(first),
        "second runner did not leave the live one alone: {second:?}, starts {}, runner {:?}",
        job.attempts,
        job.runner_pid
    );
}

#[test]
fn jobq_a_second_runner_leaves_a_live_one_alone() {
    second_runner_case("jobq_second_runner", false);
}

negative_control!(
    jobq_a_second_runner_leaves_a_live_one_alone,
    "under an expired lock, the second runner takes the queue over",
    expected = "second runner did not leave the live one alone",
    second_runner_case("jobq_second_runner_control", true)
);

// --- A runner whose lock another holds exits; status tells an expired lock ------------------------------------------

/// With a job running, the runner lock is handed to a dead pid (as a takeover would leave it, with `handed`): the
/// first runner must exit, leaving the lock to its new holder, and `status` must call that lock expired.
fn superseded_case(tag: &str, handed: bool) {
    let q = Q::new(tag, "stale = 60\n");
    let id = q.submit("peek", "hold", &[], &["sleep", "60"]);
    assert!(
        until(30, || q
            .queue()
            .load(id)
            .is_ok_and(|j| j.state == State::Running)),
        "never ran"
    );
    let first = q.queue().runner_lock().expect("a live runner").pid;
    let dead = dead_pid();
    if handed {
        let lock = format!("pid {dead}\nheartbeat {}\n", xtask::jobq::now());
        std::fs::write(q.dir.join("runner.lock"), lock).expect("lock handed over");
    }
    let exited = until(5, || !alive(first));
    let holder = q.queue().runner_lock().map(|lock| lock.pid);
    let status = String::from_utf8_lossy(&q.run(&["status"]).stdout).into_owned();
    q.run(&["cancel", &id.to_string()]);
    q.wait(id);
    q.finish();
    assert!(
        exited
            && holder == Some(dead)
            && status.contains(&format!("none alive (lock of pid {dead} expired)")),
        "superseded runner wrong: exited {exited}, lock holder {holder:?}, status {status}"
    );
}

#[test]
fn jobq_a_superseded_runner_exits() {
    superseded_case("jobq_superseded", true);
}

negative_control!(
    jobq_a_superseded_runner_exits,
    "a runner that still holds its lock keeps running",
    expected = "superseded runner wrong",
    superseded_case("jobq_superseded_control", false)
);

// --- The runner reads ps every `sample` seconds, not every pass -----------------------------------------------------

/// Starts a stand-in `rustc` outside the queue a moment after the runner's first sample, with `sample` seconds between
/// samples, and checks it is named within 3 s exactly when `named`.
fn sample_case(tag: &str, sample: u64, named: bool) {
    let q = Q::new(tag, &format!("sample = {sample}\n"));
    let id = q.submit("peek", "hold", &[], &["sleep", "60"]);
    assert!(
        until(30, || q
            .queue()
            .load(id)
            .is_ok_and(|j| j.state == State::Running)),
        "never ran"
    );
    std::thread::sleep(Duration::from_millis(1500));
    let bin = q.dir.join("bin-standin");
    std::fs::create_dir_all(&bin).expect("bin dir");
    let rustc = bin.join("rustc");
    std::os::unix::fs::symlink("/bin/sleep", &rustc).expect("stand-in linked");
    let out = Command::new("sh")
        .args([
            "-c",
            "\"$0\" 30 >/dev/null 2>&1 & echo $!",
            rustc.to_str().expect("utf-8 path"),
        ])
        .output_within(Duration::from_secs(30))
        .expect("stand-in started");
    let stray: i32 = String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse()
        .expect("a pid");
    let log = q.dir.join("violations.log");
    let seen = until(3, || {
        std::fs::read_to_string(&log).is_ok_and(|text| text.contains(&format!("pid={stray} ")))
    });
    kill("-9", stray);
    q.run(&["cancel", &id.to_string()]);
    q.wait(id);
    q.finish();
    assert_eq!(seen, named, "sampled at the wrong rate: every {sample} s");
}

#[test]
fn jobq_samples_ps_every_sample_seconds() {
    sample_case("jobq_sample_slow", 3600, false);
    sample_case("jobq_sample_fast", 1, true);
}

negative_control!(
    jobq_samples_ps_every_sample_seconds,
    "a sample every second sees the stand-in",
    expected = "sampled at the wrong rate",
    sample_case("jobq_sample_control", 1, false)
);

/// `utc` gives each date, at the leap-year and century boundaries of the civil calendar.
fn check_utc(cases: &[(u64, &str)]) {
    for (secs, expected) in cases {
        assert_eq!(utc(*secs), *expected, "utc wrong for {secs}");
    }
}

#[test]
fn jobq_utc_across_leap_years_and_centuries() {
    check_utc(&[
        (0, "1970-01-01T00:00:00Z"),
        (951_782_400, "2000-02-29T00:00:00Z"),
        (983_404_800, "2001-03-01T00:00:00Z"),
        (1_793_367_985, "2026-10-30T13:46:25Z"),
        (4_007_750_400, "2096-12-31T00:00:00Z"),
        (4_012_848_000, "2097-02-28T00:00:00Z"),
        (4_107_542_399, "2100-02-28T23:59:59Z"),
        (4_107_542_400, "2100-03-01T00:00:00Z"),
        (4_233_708_428, "2104-02-29T06:07:08Z"),
        (13_569_422_400, "2399-12-31T12:00:00Z"),
        (13_574_563_201, "2400-02-29T00:00:01Z"),
        (13_574_649_600, "2400-03-01T00:00:00Z"),
    ]);
}

negative_control!(
    jobq_utc_across_leap_years_and_centuries,
    "2100 is no leap year",
    expected = "utc wrong",
    check_utc(&[(4_107_542_400, "2100-02-29T00:00:00Z")])
);

// --- A job runs with its submitter's PATH ----------------------------------------------------------------------------

/// A runner started from a submitter whose PATH is `/usr/bin:/bin` holds a gate job, while a job running the bare
/// command `jobq-path-probe` (in a folder of its own, exiting 7) is submitted from a PATH with that folder first (with
/// `on_path`; else without it), and another from that PATH with `--env PATH=/usr/bin:/bin`. The first must find it
/// (7), and the second must not, so it doesn't start (126).
fn path_case(tag: &str, on_path: bool) {
    let q = Q::new(tag, "");
    let probe = q.dir.join("probe");
    std::fs::create_dir_all(&probe).expect("probe dir");
    let script = probe.join("jobq-path-probe");
    std::fs::write(&script, "#!/bin/sh\nexit 7\n").expect("probe written");
    std::process::Command::new("chmod")
        .args(["755", script.to_str().expect("utf-8 path")])
        .output_within(Duration::from_secs(30))
        .expect("chmod ran");
    let base = "/usr/bin:/bin";
    let with = format!("{}:{base}", probe.display());
    let gate = q.submit_with_path(base, "gate", &[], &["/bin/sh", "-c", GATE]);
    let submitter = if on_path { with.as_str() } else { base };
    let found = q.submit_with_path(submitter, "found", &[], &["jobq-path-probe"]);
    let env_path = format!("PATH={base}");
    let overridden = q.submit_with_path(
        &with,
        "overridden",
        &["--env", &env_path],
        &["jobq-path-probe"],
    );
    std::fs::write(q.work().join("go"), "").expect("gate opened");
    let codes = (q.wait(gate).0, q.wait(found).0, q.wait(overridden).0);
    let recorded = q
        .queue()
        .load(found)
        .expect("job file")
        .env
        .get("PATH")
        .cloned();
    q.finish();
    assert!(
        codes == (0, 7, 126) && recorded.as_deref() == Some(with.as_str()),
        "job did not run with its submitter's PATH: statuses {codes:?}, recorded {recorded:?}"
    );
}

#[test]
fn jobq_a_job_runs_with_its_submitters_path() {
    path_case("jobq_path", true);
}

negative_control!(
    jobq_a_job_runs_with_its_submitters_path,
    "a submitter whose PATH lacks the command's folder can't run it, whatever the runner's",
    expected = "job did not run with its submitter's PATH",
    path_case("jobq_path_control", false)
);

// --- A save that fails in a live runner -------------------------------------------------------------------------------

/// Plants a folder where the runner writes `id`'s file before renaming it, so that file's every save fails.
fn plant_failing_save(q: &Q, id: u64) -> PathBuf {
    let runner = q.queue().runner_lock().expect("a live runner").pid;
    let planted = q
        .queue()
        .job_path(id)
        .with_extension(format!("tmp{runner}"));
    std::fs::create_dir_all(&planted).expect("planted");
    planted
}

/// How many times the runner has logged that it couldn't write a file.
fn failed_saves(q: &Q) -> usize {
    std::fs::read_to_string(q.dir.join("runner.log"))
        .unwrap_or_default()
        .matches("cannot write")
        .count()
}

/// How many of the pids in `path`, one a line, are alive.
fn live_copies(path: &Path) -> Vec<i32> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| line.trim().parse().ok())
        .filter(|&pid| alive(pid))
        .collect()
}

/// A gate job holds the target directory while a long job waits for it, at a concurrency that leaves it a slot; with
/// `fail`, the long job's every save fails when the gate opens. While it fails, at most one copy may be alive and the
/// file must not move. Once saves work again, the job must run as its first recorded start, one copy, and be
/// cancellable.
fn failed_start_save_case(tag: &str, fail: bool) {
    let q = Q::new(tag, "concurrency = 2\nidle = 5\ntick_ms = 100\n");
    let gate = q.submit("peek", "gate", &["--timeout", "30"], &["sh", "-c", GATE]);
    let copies = q.work().join("copies");
    let id = q.submit(
        "peek",
        "long",
        &["--timeout", "30"],
        &["sh", "-c", "echo $$ >> copies; exec sleep 20"],
    );
    let planted = fail.then(|| plant_failing_save(&q, id));
    std::fs::write(q.work().join("go"), "").expect("gate opened");
    assert_eq!(q.wait(gate).0, 0, "the gate job failed");
    // Long enough for the unfixed runner to reach several copies, a copy every tick, and shorter than `idle`.
    let (mut most, ends) = (0, Instant::now() + Duration::from_millis(1500));
    while Instant::now() < ends {
        most = most.max(live_copies(&copies).len());
        std::thread::sleep(Duration::from_millis(50));
    }
    let during = q.queue().load(id).expect("job file");
    let undone = std::fs::read_to_string(q.dir.join("runner.log"))
        .unwrap_or_default()
        .contains("start undone");
    if let Some(planted) = &planted {
        std::fs::remove_dir(planted).expect("plant removed");
    }
    let runs = until(5, || {
        q.queue()
            .load(id)
            .is_ok_and(|job| job.state == State::Running)
            && live_copies(&copies).len() == 1
    });
    let after = q.queue().load(id).expect("job file");
    let live = live_copies(&copies);
    let cancelled = q.run(&["cancel", &id.to_string()]).status.success();
    let code = q.wait(id).0;
    let left = live_copies(&copies);
    for &pid in &left {
        kill("-9", pid);
    }
    q.finish();
    assert!(
        undone
            && most <= 1
            && during.state == State::Queued
            && during.attempts == 0
            && runs
            && after.attempts == 1
            && live.len() == 1
            && cancelled
            && code == 130
            && left.is_empty(),
        "a failed save after a start left what its file doesn't record: undone {undone}, most copies {most}, \
         during {:?} attempts {}, runs {runs}, after attempts {}, live {live:?}, cancel {cancelled}, status {code}, \
         left {left:?}",
        during.state,
        during.attempts,
        after.attempts
    );
}

#[test]
fn jobq_a_failed_save_after_a_start_undoes_it() {
    failed_start_save_case("jobq_start_save", true);
}

negative_control!(
    jobq_a_failed_save_after_a_start_undoes_it,
    "with no failing save the job simply starts: nothing is undone, and the check sees that",
    expected = "a failed save after a start left what its file",
    failed_start_save_case("jobq_start_save_control", false)
);

/// A job waits for a gate, then exits 7, while a second job, on its own target directory, has a timeout of 2 s. With
/// `fail`, the first job's every save fails when the gate opens. While it fails, the file must still say running, and
/// the second job must still be timed out. Once saves work again, the first must end with its true status and end time,
/// from its one start, and the runner go idle.
fn failed_end_save_case(tag: &str, fail: bool) {
    let q = Q::new(tag, "concurrency = 2\nidle = 8\n");
    let id = q.submit(
        "peek",
        "ends",
        &["--timeout", "30"],
        &["sh", "-c", &format!("{GATE}; exit 7")],
    );
    assert!(
        until(10, || q
            .queue()
            .load(id)
            .is_ok_and(|job| job.leader_start.is_some())),
        "the job's start was never recorded"
    );
    let planted = fail.then(|| plant_failing_save(&q, id));
    let other = q.submit(
        "peek",
        "other",
        &[
            "--timeout",
            "2",
            "--env",
            "CARGO_TARGET_DIR=/nonexistent/jobq-test-other",
        ],
        &["sleep", "20"],
    );
    // A pass with a failed save starts nothing, so the second job must be running before the first one's end.
    assert!(
        until(10, || q
            .queue()
            .load(other)
            .is_ok_and(|job| job.state == State::Running)),
        "the second job never started"
    );
    std::fs::write(q.work().join("go"), "").expect("gate opened");
    let tried_again = until(3, || failed_saves(&q) >= 3);
    let other_ended = until(5, || {
        q.queue()
            .load(other)
            .is_ok_and(|job| job.state == State::TimedOut)
    });
    let during = q.queue().load(id).expect("job file");
    // The clock moves on, so a save that recomputed the end would record a later one.
    std::thread::sleep(Duration::from_millis(2100));
    let removed_at = xtask::jobq::now();
    if let Some(planted) = &planted {
        std::fs::remove_dir(planted).expect("plant removed");
    }
    let ended = until(5, || {
        q.queue()
            .load(id)
            .is_ok_and(|job| job.state == State::Exited)
    });
    let after = q.queue().load(id).expect("job file");
    let code = if ended { q.wait(id).0 } else { -1 };
    let idled = until(10, || !q.dir.join("runner.lock").exists());
    q.finish();
    assert!(
        tried_again
            && other_ended
            && during.state == State::Running
            && ended
            && code == 7
            && after.exit == Some(7)
            && after.attempts == 1
            && after.finished.is_some_and(|at| at < removed_at)
            && idled,
        "a failed save at a job's end lost the job: tried again {tried_again}, other ended {other_ended}, during \
         {:?}, ended {ended}, status {code}, exit {:?}, attempts {}, finished {:?} against {removed_at}, idled {idled}",
        during.state,
        after.exit,
        after.attempts,
        after.finished
    );
}

#[test]
fn jobq_a_failed_save_at_a_jobs_end_is_tried_again() {
    failed_end_save_case("jobq_end_save", true);
}

negative_control!(
    jobq_a_failed_save_at_a_jobs_end_is_tried_again,
    "with no failing save nothing is tried again, and the check sees that",
    expected = "a failed save at a job's end lost the job",
    failed_end_save_case("jobq_end_save_control", false)
);

// --- Concurrency -----------------------------------------------------------------------------------------------------

/// Two jobs that each wait for the other to start, at `concurrency`, the first on target directory `a` with
/// `--timeout <timeout>`, the second on `b`: run at once, both end 0; run one after the other, the first times out
/// (124) and the second, started only then, ends 0. Their statuses must be `expected`.
fn concurrency_case(
    tag: &str,
    concurrency: usize,
    (a, b): (&str, &str),
    timeout: u64,
    expected: (i32, i32),
) {
    let q = Q::new(tag, &format!("concurrency = {concurrency}\n"));
    let script = |me: &str, other: &str| {
        format!("touch {me}; i=0; while [ ! -e {other} ] && [ $i -lt 800 ]; do sleep 0.05; i=$((i+1)); done")
    };
    let (a_env, b_env, timeout) = (
        format!("CARGO_TARGET_DIR={a}"),
        format!("CARGO_TARGET_DIR={b}"),
        timeout.to_string(),
    );
    let first = q.submit(
        "peek",
        "a",
        &["--timeout", &timeout, "--env", &a_env],
        &["sh", "-c", &script("a", "b")],
    );
    let second = q.submit(
        "peek",
        "b",
        &["--timeout", "10", "--env", &b_env],
        &["sh", "-c", &script("b", "a")],
    );
    let codes = (q.wait(first).0, q.wait(second).0);
    q.finish();
    assert_eq!(
        codes, expected,
        "jobs not run {concurrency} at a time on {a} and {b}"
    );
}

#[test]
fn jobq_runs_concurrency_jobs_at_once() {
    concurrency_case("jobq_concurrency", 2, ("/t/a", "/t/b"), 10, (0, 0));
}

negative_control!(
    jobq_runs_concurrency_jobs_at_once,
    "one at a time, the first job waits for the second until its timeout",
    expected = "jobs not run 1 at a time",
    concurrency_case("jobq_concurrency_control", 1, ("/t/a", "/t/b"), 10, (0, 0))
);

/// At concurrency 2, two jobs on one target directory, under each of `spellings`, never run at once: the first times
/// out waiting for the second, which starts only after it.
fn shared_target_case(tag: &str, spellings: &[(&str, &str)]) {
    for (n, pair) in spellings.iter().enumerate() {
        concurrency_case(&format!("{tag}_{n}"), 2, *pair, 3, (124, 0));
    }
}

#[test]
fn jobq_never_runs_two_jobs_on_one_target_directory() {
    shared_target_case(
        "jobq_shared_target",
        &[("/t/a", "/t/a"), ("/t/a", "/T/A/"), ("/t/a", "/t/./a")],
    );
}

negative_control!(
    jobq_never_runs_two_jobs_on_one_target_directory,
    "jobs on two target directories run at once",
    expected = "jobs not run 2 at a time",
    shared_target_case("jobq_shared_target_control", &[("/t/a", "/t/b")])
);

// --- What keeps a run of these tests, mutated or not, from harming its machine ----------------------------------------

/// `may_signal_group` answers `expected` for each group, for a process 500 in group 400.
fn check_may_signal(cases: &[(i32, bool)]) {
    for &(pgid, expected) in cases {
        assert_eq!(
            may_signal_group(pgid, 500, 400),
            expected,
            "group {pgid} wrongly judged"
        );
    }
}

// Only the pure check is tested. No test may plant a group it refuses for the runner to signal: with the check mutated,
// that would be a real `kill(-1, SIGKILL)` on the machine running the mutant.
#[test]
fn jobq_refuses_to_signal_system_and_own_groups() {
    check_may_signal(&[
        (i32::MIN, false),
        (-1, false),
        (0, false),
        (1, false),
        (2, true),
        (399, true),
        (400, false),
        (401, true),
        (499, true),
        (500, false),
        (501, true),
    ]);
}

negative_control!(
    jobq_refuses_to_signal_system_and_own_groups,
    "group 1, every process of this user, is not to be signalled",
    expected = "group 1 wrongly judged",
    check_may_signal(&[(1, true)])
);

/// `pid_alive` answers `expected` for each pid.
fn check_pid_alive(cases: &[(i32, bool)]) {
    for &(pid, expected) in cases {
        assert_eq!(pid_alive(pid), expected, "pid {pid} wrongly judged");
    }
}

#[test]
fn jobq_a_pid_of_zero_or_below_is_not_alive() {
    check_pid_alive(&[
        (-1, false),
        (0, false),
        (1, true),
        (std::process::id() as i32, true),
        (dead_pid(), false),
    ]);
}

negative_control!(
    jobq_a_pid_of_zero_or_below_is_not_alive,
    "pid 0 names no process",
    expected = "pid 0 wrongly judged",
    check_pid_alive(&[(0, true)])
);

/// A stand-in naming nothing in its arguments, running in `cwd`, in a group of its own; and the thread reaping it.
fn start_in(
    cwd: &Path,
) -> (
    i32,
    std::thread::JoinHandle<std::io::Result<std::process::ExitStatus>>,
) {
    let mut child = Command::new("sleep")
        .arg("60")
        .current_dir(cwd)
        .process_group(0)
        .spawn()
        .expect("stand-in started");
    let pid = child.id() as i32;
    (pid, std::thread::spawn(move || child.wait()))
}

/// A stand-in running in `cwd` must be ended by `ending`, exactly when `ended`.
fn check_ended(what: &str, cwd: &Path, ended: bool, ending: impl FnOnce()) {
    let (pid, reaper) = start_in(cwd);
    // Until `lsof` or `/proc` shows the stand-in in its folder, `ending` can't find it.
    assert!(
        until(10, || working_directories().contains_key(&pid)),
        "the stand-in never showed a working directory"
    );
    ending();
    let gone = until(5, || !alive(pid));
    kill("-9", pid);
    let _ = reaper.join();
    assert_eq!(gone, ended, "{what}");
}

/// A process working in the queue's folder, or, without `in_queue`, in another, as a job does though it doesn't name the
/// folder in its arguments, is ended by the queue's drop.
fn drop_case(tag: &str, in_queue: bool) {
    let q = Q::new(tag, "");
    let other = Scratch::new("jobq_drop_other");
    std::fs::create_dir_all(&*other).expect("other folder");
    let cwd = if in_queue {
        q.work()
    } else {
        other.to_path_buf()
    };
    check_ended("a job outlived its queue's drop", &cwd, true, move || {
        drop(q)
    });
}

#[test]
fn jobq_a_queues_drop_ends_its_jobs() {
    drop_case("jobq_drop", true);
}

negative_control!(
    jobq_a_queues_drop_ends_its_jobs,
    "a process in another folder is no job of this queue",
    expected = "a job outlived its queue's drop",
    drop_case("jobq_drop_control", false)
);

/// A process working in the scratch folder of a test process, gone or (without `owner_dead`) alive, as a job left by a
/// killed test does, is ended by the sweep only if that test process has gone.
fn sweep_case(tag: &str, name: &str, owner_dead: bool) {
    let owner = if owner_dead {
        dead_pid()
    } else {
        std::process::id() as i32
    };
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}_{tag}-{owner}-0"));
    std::fs::create_dir_all(&dir).expect("folder made");
    check_ended(
        "the sweep left a dead test's job running",
        &dir,
        true,
        sweep_dead_tests_now,
    );
    std::fs::remove_dir_all(&dir).expect("folder removed");
}

#[test]
fn jobq_the_sweep_ends_a_dead_tests_jobs_by_their_folder() {
    sweep_case("sweep", "jobq", true);
    sweep_case("sweep", "qajobq", true);
}

negative_control!(
    jobq_the_sweep_ends_a_dead_tests_jobs_by_their_folder,
    "a live test's job is not swept",
    expected = "the sweep left a dead test's job running",
    sweep_case("sweep_control", "jobq", false)
);

// --- The runner's own bounds --------------------------------------------------------------------------------------

/// A job queued with `attempts` starts behind it ends with status `expected`.
fn start_limit_case(tag: &str, attempts: u32, expected: i32) {
    let q = Q::new(tag, "");
    std::fs::create_dir_all(q.dir.join("jobs")).expect("jobs dir");
    std::fs::write(q.dir.join("next_id"), "2\n").expect("next id");
    let mut planted = job(1, Priority::Peek, State::Queued, Some("/t"));
    planted.worktree = q.work();
    planted.attempts = attempts;
    planted.command = ["sh", "-c", "exit 7"].map(str::to_owned).to_vec();
    q.queue().save(&planted).expect("job planted");
    let code = q.wait(1).0;
    q.finish();
    assert_eq!(code, expected, "a job's starts are not limited");
}

#[test]
fn jobq_no_job_starts_more_than_max_attempts_times() {
    start_limit_case("jobq_start_limit", MAX_ATTEMPTS, 125);
    start_limit_case("jobq_start_limit_last", MAX_ATTEMPTS - 1, 7);
}

negative_control!(
    jobq_no_job_starts_more_than_max_attempts_times,
    "one start short of the limit, the job runs",
    expected = "a job's starts are not limited",
    start_limit_case("jobq_start_limit_control", MAX_ATTEMPTS - 1, 125)
);

/// A running job's file is set back to queued, with a slot free: the runner must not start a second copy, which would
/// lose the first's timeout and cancel. It must have started `expected` copies.
fn tracked_case(tag: &str, expected: usize) {
    let q = Q::new(tag, "concurrency = 2\nidle = 5\n");
    let id = q.submit(
        "peek",
        "long",
        &["--timeout", "30"],
        &["sh", "-c", "echo $$ >> copies; exec sleep 20"],
    );
    assert!(
        until(10, || q
            .queue()
            .load(id)
            .is_ok_and(|job| job.leader_start.is_some())),
        "the job's start was never recorded"
    );
    let mut back = q.queue().load(id).expect("job file");
    back.state = State::Queued;
    q.queue().save(&back).expect("job set back");
    // Twenty passes of the runner.
    std::thread::sleep(Duration::from_secs(1));
    back.state = State::Running;
    q.queue().save(&back).expect("job restored");
    let copies = std::fs::read_to_string(q.work().join("copies")).expect("copies written");
    let copies: Vec<&str> = copies.lines().collect();
    assert!(
        q.run(&["cancel", &id.to_string()]).status.success(),
        "cancel failed"
    );
    assert_eq!(q.wait(id).0, 130, "the job's one copy was not cancelled");
    q.finish();
    for pid in &copies {
        kill("-9", pid.parse().expect("a pid"));
    }
    assert_eq!(
        copies.len(),
        expected,
        "a job already running started again"
    );
}

#[test]
fn jobq_a_job_already_running_is_not_started_again() {
    tracked_case("jobq_tracked", 1);
}

negative_control!(
    jobq_a_job_already_running_is_not_started_again,
    "a job that started once did not start twice",
    expected = "a job already running started again",
    tracked_case("jobq_tracked_control", 2)
);

/// `ps -o lstart=` for `pid` under `tz` and the C locale.
fn lstart(pid: i32, tz: &str) -> String {
    let out = Command::new("ps")
        .args(["-o", "lstart=", "-p", &pid.to_string()])
        .env("LC_ALL", "C")
        .env("TZ", tz)
        .output_within(Duration::from_secs(30))
        .expect("ps ran");
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

/// A runner started under another time zone and locale records its job's start as `ps` gives it under `compare_tz`
/// and the C locale; so a later runner, in any environment, reads the same text.
fn start_time_case(tag: &str, compare_tz: &str) {
    let q = Q::new(tag, "");
    let work = q.work();
    let out = q
        .command(&[
            "submit",
            work.to_str().expect("utf-8 path"),
            "--priority",
            "peek",
            "--label",
            "long",
            "--env",
            "CARGO_TARGET_DIR=/nonexistent/jobq-test-target",
            "--",
            "sleep",
            "20",
        ])
        .env("TZ", "Asia/Tokyo")
        .env("LC_ALL", "POSIX")
        .output_within(Duration::from_secs(45))
        .expect("xtask job ran");
    let id: u64 = String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse()
        .expect("submit printed an id");
    assert!(
        until(10, || q
            .queue()
            .load(id)
            .is_ok_and(|job| job.leader_start.is_some())),
        "the job's start was never recorded"
    );
    let job = q.queue().load(id).expect("job file");
    let pid = job.pgid.expect("its group");
    let expected = lstart(pid, compare_tz);
    assert!(
        q.run(&["cancel", &id.to_string()]).status.success(),
        "cancel failed"
    );
    assert_eq!(q.wait(id).0, 130, "the job was not cancelled");
    q.finish();
    assert_eq!(
        job.leader_start.as_deref(),
        Some(expected.as_str()),
        "a job's start depends on its runner's environment"
    );
}

#[test]
fn jobq_a_jobs_start_time_is_the_same_in_any_environment() {
    start_time_case("jobq_start_time", "UTC");
}

negative_control!(
    jobq_a_jobs_start_time_is_the_same_in_any_environment,
    "the same start reads differently in another zone, which is why it is recorded in UTC",
    expected = "a job's start depends on its runner's environment",
    start_time_case("jobq_start_time_control", "America/New_York")
);

/// For each (worktree, target, worktree, target, same): the two jobs have one target key exactly when `same`.
fn check_relative_target(pairs: &[(&str, &str, &str, &str, bool)]) {
    for &(worktree_a, a, worktree_b, b, same) in pairs {
        let mut first = job(1, Priority::Peek, State::Queued, Some(a));
        first.worktree = PathBuf::from(worktree_a);
        let mut second = job(2, Priority::Peek, State::Queued, Some(b));
        second.worktree = PathBuf::from(worktree_b);
        assert_eq!(
            first.target_key() == second.target_key(),
            same,
            "relative target wrong: {a} in {worktree_a}, {b} in {worktree_b}"
        );
    }
}

#[test]
fn jobq_a_relative_target_directory_is_its_worktrees() {
    check_relative_target(&[
        (
            "/nonexistent/w",
            "target",
            "/other",
            "/nonexistent/w/target",
            true,
        ),
        (
            "/nonexistent/w",
            "./target",
            "/nonexistent/w",
            "target",
            true,
        ),
        (
            "/nonexistent/w",
            "target",
            "/nonexistent/x",
            "target",
            false,
        ),
        (
            "/nonexistent/w",
            "/nonexistent/t",
            "/nonexistent/x",
            "/nonexistent/t",
            true,
        ),
    ]);
}

negative_control!(
    jobq_a_relative_target_directory_is_its_worktrees,
    "two worktrees' own `target` are two directories",
    expected = "relative target wrong",
    check_relative_target(&[("/nonexistent/w", "target", "/nonexistent/x", "target", true)])
);
