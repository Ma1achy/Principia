//! qa's tests for R-658's bounds, written from `decisions.md` § "R-658" ("A new runner ends the process groups of the jobs
//! a dead one left running and queues them again, or marks one lost after its third start"; "A job left running by dead
//! runners three times is marked lost, so a job that kills its runner cannot loop"), not from the implementation:
//! - a job whose runners keep dying starts exactly three times, ends lost, and leaves no copy;
//! - a job queued with its three starts behind it never runs;
//! - a runner started in another time zone and locale still knows a dead runner's job for its own, and ends it;
//! - a job's file saying queued while its copy runs neither starts a second copy nor stalls the runner;
//! - the check in front of every group signal, as a pure function, over pids and groups that coincide.
//!
//! Safety, as in `xtask/tests/qa_R-658.rs`: a queue of each test's own in a scratch folder; every `xtask job` call under
//! `ulimit -u` and a time limit; the commands are `sh` and a stand-in for `sleep`, each ending by itself; every long-lived
//! process names the queue's folder, and the queue's guard ends, by pid, every process naming it when the test ends. No
//! test plants a group `may_signal_group` refuses: with that check mutated, the runner would really signal it. Only the
//! pure function sees those groups. The one signal these tests send themselves is SIGKILL or SIGSTOP to a runner's pid.
#![cfg(unix)]

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

use scratch::Scratch;
use validation::negative_control;
use validation::spawn::Spawn;
use xtask::jobq::{may_signal_group, now, Job, Priority, Queue, State};

/// The starts `decisions.md` § "R-658" allows a job: "marks one lost after its third start".
const THIRD_START: u32 = 3;

/// A job's time zone and locale that differ from the later runner's. POSIX `TZ` strings, which need no zone database.
const SUBMITTER: [(&str, &str); 2] = [("LC_ALL", "en_GB.UTF-8"), ("TZ", "JST-9")];
const LATER_RUNNER: [(&str, &str); 2] = [("LC_ALL", "C"), ("TZ", "EST5")];

struct Q {
    dir: Scratch,
    sleeper: PathBuf,
}

impl Q {
    fn new(tag: &str, config: &str) -> Self {
        let dir = Scratch::new(&format!("qajobq_r4_{tag}"));
        std::fs::create_dir_all(dir.join("work")).expect("scratch made");
        std::fs::write(
            dir.join("config"),
            format!("concurrency = 1\nidle = 1\ntick_ms = 50\nsample = 1\ngrace = 1\n{config}"),
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

    fn job(&self, id: u64) -> Job {
        self.queue().load(id).expect("job file")
    }

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

    /// Submits a shell script, given the stand-in for `sleep` as `$0`, from a submitter with `env` besides its own.
    fn submit(&self, env: &[(&str, &str)], script: &str) -> u64 {
        let work = self.work();
        let args = [
            "submit",
            work.to_str().expect("utf-8 path"),
            "--priority",
            "peek",
            "--label",
            "qa",
            "--timeout",
            "40",
            "--env",
            "CARGO_TARGET_DIR=/nonexistent/qa-r4",
            "--",
            "/bin/sh",
            "-c",
            script,
            self.sleeper.to_str().expect("utf-8 path"),
        ];
        let out = self
            .command(&args)
            .envs(env.iter().copied())
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

    /// `job wait`, from a client with `env` besides its own, which restarts a dead runner. A new runner may start only
    /// every 5 s, so the limit is more than the others'.
    fn wait(&self, env: &[(&str, &str)], id: u64) -> (i32, Output) {
        let out = self
            .command(&["wait", &id.to_string()])
            .envs(env.iter().copied())
            .output_within(Duration::from_secs(90))
            .expect("xtask job ran");
        (out.status.code().unwrap_or(-1), out)
    }

    fn cancel(&self, id: u64) -> bool {
        self.command(&["cancel", &id.to_string()])
            .output_within(Duration::from_secs(45))
            .expect("xtask job ran")
            .status
            .success()
    }

    /// Waits until `id` runs its `start`th start, whose leader's start time is recorded; returns its runner's pid.
    fn running_start(&self, id: u64, start: u32) -> i32 {
        assert!(
            until(60, || self.queue().load(id).is_ok_and(|job| {
                job.state == State::Running && job.attempts == start && job.leader_start.is_some()
            })),
            "job {id} never ran its start {start}: {:?}",
            self.queue().load(id)
        );
        self.job(id).runner_pid.expect("a runner")
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
                if let Ok(pid) = pid.parse() {
                    signal("-9", pid);
                }
            }
        }
    }
}

/// Sends `signal` to one process, never to a group: a pid of 1 or below, as a damaged file could give, is refused, since
/// `kill -9 -1` and `kill -9 0` signal groups.
fn signal(signal: &str, pid: i32) {
    if pid > 1 {
        let _ = Command::new("kill")
            .args([signal, &pid.to_string()])
            .output_within(Duration::from_secs(30));
    }
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

// --- The check in front of every group signal ---------------------------------------------------------------------------

/// `may_signal_group` over each pid and group of the signaller, leaders and non-leaders, in turn: `refused` must be
/// refused, as must the signaller's own pid and group; `allowed` must be allowed where it is neither.
fn signal_check_case(refused: &[i32], allowed: &[i32]) {
    let selves = [
        (500, 500),
        (500, 400),
        (400, 500),
        (2, 2),
        (500, 1),
        (i32::MAX, i32::MAX),
    ];
    for (own_pid, own_group) in selves {
        let own = [own_pid, own_group];
        for &pgid in refused.iter().chain(&own) {
            assert!(
                !may_signal_group(pgid, own_pid, own_group),
                "group wrongly judged: {pgid} may be signalled by pid {own_pid} in group {own_group}"
            );
        }
        for &pgid in allowed.iter().filter(|pgid| !own.contains(pgid)) {
            assert!(
                may_signal_group(pgid, own_pid, own_group),
                "group wrongly judged: {pgid} refused to pid {own_pid} in group {own_group}"
            );
        }
    }
}

#[test]
fn qa_r658_no_signal_reaches_the_system_or_the_signaller() {
    signal_check_case(
        &[i32::MIN, -500, -2, -1, 0, 1],
        &[2, 3, 399, 401, 501, 99_999, i32::MAX],
    );
}

negative_control!(
    qa_r658_no_signal_reaches_the_system_or_the_signaller,
    "a job's group, which is no one else's, is to be signalled, so refusing it is wrong",
    expected = "group wrongly judged",
    signal_check_case(&[], &[1])
);

// --- A job whose runners keep dying -------------------------------------------------------------------------------------

/// A job is running, its start recorded, when its runner is killed, `kills` times in all, with a `wait` restarting a
/// runner each time. With three kills, it must end lost (125), having started three times, no copy left. With two (the
/// control), the third start runs on, and is cancelled.
fn dying_runners_case(tag: &str, kills: u32) {
    let q = Q::new(tag, "stale = 60\n");
    let id = q.submit(&[], "echo $$ >> copies; exec \"$0\" 30");
    let mut waiter = q.command(&["wait", &id.to_string()]);
    let waiter = std::thread::spawn(move || waiter.output_within(Duration::from_secs(150)).ok());
    for start in 1..=kills {
        signal("-9", q.running_start(id, start));
    }
    if kills < THIRD_START {
        q.running_start(id, kills + 1);
        assert!(q.cancel(id), "cancel failed");
    }
    let code = waiter
        .join()
        .expect("waiter thread")
        .and_then(|out| out.status.code())
        .unwrap_or(-1);
    let job = q.job(id);
    let copies = pids(&q.work().join("copies"));
    let left: Vec<i32> = copies.iter().copied().filter(|&pid| alive(pid)).collect();
    q.finish();
    assert!(
        code == 125
            && job.state == State::Lost
            && job.attempts == THIRD_START
            && copies.len() == THIRD_START as usize
            && left.is_empty(),
        "a job whose runners die was not lost at its third start: status {code}, {:?}, starts {}, copies {copies:?}, \
         left {left:?}",
        job.state,
        job.attempts
    );
}

#[test]
fn qa_r658_a_job_whose_runners_die_ends_lost_at_its_third_start() {
    dying_runners_case("dying", THIRD_START);
}

negative_control!(
    qa_r658_a_job_whose_runners_die_ends_lost_at_its_third_start,
    "with two runners killed the third start runs, and the check sees it",
    expected = "a job whose runners die was not lost",
    dying_runners_case("dying_control", THIRD_START - 1)
);

// --- A job queued with its starts behind it ----------------------------------------------------------------------------

/// A job file planted queued, with `attempts` starts behind it, the command touching `ran`: at three or more it must end
/// lost without running. Two (the control) is one short, and it runs.
fn queued_spent_case(tag: &str, attempts: u32) {
    let q = Q::new(tag, "stale = 3\n");
    std::fs::create_dir_all(q.dir.join("jobs")).expect("jobs dir");
    std::fs::write(q.dir.join("next_id"), "2\n").expect("next id");
    let mut env = BTreeMap::new();
    env.insert("CARGO_TARGET_DIR".to_owned(), "/t/1".to_owned());
    env.insert("PATH".to_owned(), "/usr/bin:/bin".to_owned());
    let job = Job {
        id: 1,
        label: "spent".to_owned(),
        priority: Priority::Fix,
        worktree: q.work(),
        command: ["/usr/bin/touch", "ran"].map(str::to_owned).to_vec(),
        env,
        timeout_secs: 40,
        submitted: now(),
        state: State::Queued,
        attempts,
        started: None,
        finished: None,
        pgid: None,
        leader_start: None,
        runner_pid: None,
        exit: None,
        cancel_requested: false,
        note: None,
    };
    q.queue().save(&job).expect("job planted");
    let (code, out) = q.wait(&[], 1);
    let ran = q.work().join("ran").exists();
    let after = q.job(1);
    q.finish();
    assert!(
        code == 125 && !ran && after.state == State::Lost && after.attempts == attempts,
        "a job with its starts spent was run: status {code}, {out:?}, ran {ran}, {:?}, starts {}",
        after.state,
        after.attempts
    );
}

#[test]
fn qa_r658_a_job_with_its_starts_spent_never_runs() {
    queued_spent_case("spent", THIRD_START);
    // No overflow past the limit, however the file came to say so.
    queued_spent_case("spent_most", u32::MAX);
}

negative_control!(
    qa_r658_a_job_with_its_starts_spent_never_runs,
    "one start short of the limit, the job runs",
    expected = "a job with its starts spent was run",
    queued_spent_case("spent_control", THIRD_START - 1)
);

// --- A runner in another time zone and locale ---------------------------------------------------------------------------

/// A runner in one time zone and locale records a job's start and is killed; a `wait` in another starts the next runner,
/// which must know the group for the job's, end it, and run the job again. With `pre_fix`, the control, the file holds
/// the start as `ps` gives it in the first environment, which is not the second's reading: the job's first copy is left
/// running, and the check sees it.
fn other_environment_case(tag: &str, pre_fix: bool) {
    let q = Q::new(tag, "stale = 60\n");
    let id = q.submit(
        &SUBMITTER,
        "if [ -e marker ]; then exit 0; fi; touch marker; echo $$ > first.pid; exec \"$0\" 60",
    );
    let runner = q.running_start(id, 1);
    let list = q.work().join("first.pid");
    assert!(until(10, || !pids(&list).is_empty()), "no pid from the job");
    let first = pids(&list)[0];
    signal("-9", runner);
    assert!(until(10, || !alive(runner)), "the runner outlived its kill");
    if pre_fix {
        let read = Command::new("ps")
            .args(["-o", "lstart=", "-p", &first.to_string()])
            .envs(SUBMITTER)
            .output_within(Duration::from_secs(30))
            .expect("ps ran");
        let mut job = q.job(id);
        job.leader_start = Some(String::from_utf8_lossy(&read.stdout).trim().to_owned());
        q.queue().save(&job).expect("job rewritten");
    }
    let (code, out) = q.wait(&LATER_RUNNER, id);
    let first_left = !until(5, || !alive(first));
    let job = q.job(id);
    signal("-9", first);
    q.finish();
    assert!(
        code == 0 && job.attempts == 2 && !first_left,
        "a runner in another environment did not know a dead runner's job: status {code}, {out:?}, starts {}, \
         first copy alive {first_left}",
        job.attempts
    );
}

#[test]
fn qa_r658_a_runner_in_another_environment_ends_a_dead_runners_job() {
    other_environment_case("other_env", false);
}

negative_control!(
    qa_r658_a_runner_in_another_environment_ends_a_dead_runners_job,
    "a start time written in the first environment's terms does not match the second's reading",
    expected = "a runner in another environment did not know",
    other_environment_case("other_env_control", true)
);

// --- A start that fills no slot -----------------------------------------------------------------------------------------

/// A running job's file is set back to queued, with a slot free, so each pass chooses a job whose start fills no slot.
/// Over three seconds the runner must start no second copy and go on passing: its heartbeat moves, and a client gets the
/// state lock at once. With `hang`, the control, the runner is stopped meanwhile, and the check sees the heartbeat stand.
fn no_slot_filled_case(tag: &str, hang: bool) {
    let q = Q::new(tag, "concurrency = 2\nidle = 5\nstale = 60\n");
    let id = q.submit(&[], "echo $$ >> copies; exec \"$0\" 30");
    let runner = q.running_start(id, 1);
    let mut back = q.job(id);
    back.state = State::Queued;
    q.queue().save(&back).expect("job set back");
    if hang {
        signal("-STOP", runner);
    }
    let before = q.queue().runner_lock().expect("a lock").heartbeat;
    std::thread::sleep(Duration::from_secs(3));
    let after = q.queue().runner_lock().expect("a lock").heartbeat;
    if hang {
        signal("-CONT", runner);
    }
    back.state = State::Running;
    q.queue().save(&back).expect("job restored");
    let answered = Instant::now();
    let answer = q
        .command(&["status"])
        .output_within(Duration::from_secs(45))
        .expect("xtask job ran");
    let answered = answered.elapsed();
    let copies = pids(&q.work().join("copies"));
    assert!(q.cancel(id), "cancel failed");
    let status = q.wait(&[], id).0;
    q.finish();
    assert!(
        after >= before + 2
            && answer.status.success()
            && answered < Duration::from_secs(20)
            && copies.len() == 1
            && status == 130,
        "a start that fills no slot stalled the runner or started a copy: heartbeat {before} to {after}, status took \
         {answered:?}, copies {copies:?}, cancelled with {status}"
    );
}

#[test]
fn qa_r658_a_start_that_fills_no_slot_neither_stalls_nor_copies() {
    no_slot_filled_case("no_slot", false);
}

negative_control!(
    qa_r658_a_start_that_fills_no_slot_neither_stalls_nor_copies,
    "a runner that has stopped passing leaves its heartbeat standing",
    expected = "a start that fills no slot stalled the runner",
    no_slot_filled_case("no_slot_control", true)
);
