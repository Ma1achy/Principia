//! qa's tests for R-658's second fix round, written from `plan/OPERATIONS.md` § "Resources", "The job queue" ("The
//! job's command is looked up on that PATH, not the runner's") and `decisions.md` § "R-658" ("A new runner ends the
//! process groups of the jobs a dead one left running"), not from the implementation:
//! - a command only the runner's PATH has doesn't start: the job's PATH alone is searched;
//! - a dead runner's group whose start time was never recorded ends whole, a member ignoring SIGTERM included.
//!
//! Safety, as in `xtask/tests/qa_R-658.rs`: a queue of each test's own in a scratch folder; every `xtask job` call under
//! `ulimit -u` and 45 s at most; the commands are `sh`, `sleep` and a script; every long-lived process names the
//! queue's folder, and the queue's guard ends, by pid, every process naming it when the test ends.
#![cfg(unix)]

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt as _;
use std::os::unix::process::CommandExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

use scratch::Scratch;
use validation::negative_control;
use validation::spawn::Spawn;
use xtask::jobq::{now, Job, Priority, Queue, State};

/// Loops until `go` exists in the job's folder, 40 s at most.
const GATE: &str = "i=0; while [ ! -e go ] && [ $i -lt 800 ]; do sleep 0.05; i=$((i+1)); done";

struct Q {
    dir: Scratch,
}

impl Q {
    fn new(tag: &str) -> Self {
        let dir = Scratch::new(&format!("qajobq_r2_{tag}"));
        std::fs::create_dir_all(dir.join("work")).expect("scratch made");
        std::fs::write(
            dir.join("config"),
            "concurrency = 1\nidle = 1\ntick_ms = 50\nsample = 1\ngrace = 1\nstale = 3\n",
        )
        .expect("config written");
        Q { dir }
    }

    fn work(&self) -> PathBuf {
        self.dir.join("work")
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

    /// Submits `command` from a submitter whose PATH is `path`, returning its id.
    fn submit(&self, path: &str, target: &str, command: &[&str]) -> u64 {
        let work = self.work();
        let env = format!("CARGO_TARGET_DIR={target}");
        let mut args = vec![
            "submit",
            work.to_str().expect("utf-8 path"),
            "--priority",
            "peek",
            "--label",
            "qa",
            "--env",
            &env,
            "--",
        ];
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

    fn wait(&self, id: u64) -> (i32, Output) {
        let out = self
            .command(&["wait", &id.to_string()])
            .output_within(Duration::from_secs(45))
            .expect("xtask job ran");
        (out.status.code().unwrap_or(-1), out)
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
                kill("-9", pid);
            }
        }
    }
}

fn kill(signal: &str, pid: &str) {
    let _ = Command::new("kill")
        .args([signal, pid])
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

fn read_pid(path: &Path) -> i32 {
    assert!(
        until(30, || std::fs::read_to_string(path)
            .is_ok_and(|t| t.ends_with('\n'))),
        "no pid in {path:?}"
    );
    std::fs::read_to_string(path)
        .expect("pid file")
        .trim()
        .parse()
        .expect("a pid")
}

// --- The job's PATH, not the runner's -----------------------------------------------------------------------------

/// A runner started from a submitter whose PATH has a folder with `qa-only-runner` in it (exiting 7) holds a gate job;
/// a job running that bare command comes from a submitter whose PATH lacks it (or, `job_has_it`, the control, has it).
/// The job's PATH alone is searched, so it must not start: status 126.
fn runners_path_case(tag: &str, job_has_it: bool) {
    let q = Q::new(tag);
    let probe = q.dir.join("probe");
    std::fs::create_dir_all(&probe).expect("probe dir");
    let script = probe.join("qa-only-runner");
    std::fs::write(&script, "#!/bin/sh\nexit 7\n").expect("probe written");
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    let base = "/usr/bin:/bin";
    let with = format!("{}:{base}", probe.display());
    let gate = q.submit(&with, "/nonexistent/qa-a", &["/bin/sh", "-c", GATE]);
    let submitter = if job_has_it { with.as_str() } else { base };
    let id = q.submit(submitter, "/nonexistent/qa-b", &["qa-only-runner"]);
    std::fs::write(q.work().join("go"), "").expect("gate opened");
    let gate_code = q.wait(gate).0;
    let (code, out) = q.wait(id);
    q.finish();
    assert!(
        gate_code == 0 && code == 126,
        "a job ran a command only the runner's PATH has: gate {gate_code}, status {code}, {out:?}"
    );
}

#[test]
fn qa_r658_a_command_only_on_the_runners_path_does_not_start() {
    runners_path_case("runner_path", false);
}

negative_control!(
    qa_r658_a_command_only_on_the_runners_path_does_not_start,
    "a job whose own PATH has the command runs it",
    expected = "a job ran a command only the runner's PATH has",
    runners_path_case("runner_path_control", true)
);

// --- A dead runner's group whose start time was never recorded --------------------------------------------------------

/// A job left running by a dead runner that never recorded its leader's start time, whose group has a live leader and a
/// member, both ignoring SIGTERM. The job's file says it started `ago` seconds before the leader did. A new runner must
/// end the group, member included, and run the job again. With `ago` an hour (the control) the group isn't the job's.
fn unrecorded_group_case(tag: &str, ago: u64) {
    let q = Q::new(tag);
    let sleeper = q.dir.join("qa-sleep");
    std::os::unix::fs::symlink("/bin/sleep", &sleeper).expect("stand-in linked");
    let began = now();
    let script = "trap '' TERM; \"$0\" 60 & echo $! > member.pid; wait";
    let mut leader = Command::new("/bin/sh")
        .args(["-c", script])
        .arg(&sleeper)
        .current_dir(q.work())
        .process_group(0)
        .spawn()
        .expect("leader started");
    let pid = leader.id() as i32;
    let reaper = std::thread::spawn(move || leader.wait());
    let member = read_pid(&q.work().join("member.pid"));

    std::fs::create_dir_all(q.dir.join("jobs")).expect("jobs dir");
    std::fs::write(q.dir.join("next_id"), "2\n").expect("next id");
    let mut env = BTreeMap::new();
    env.insert("CARGO_TARGET_DIR".to_owned(), "/t/1".to_owned());
    env.insert("PATH".to_owned(), "/usr/bin:/bin".to_owned());
    let dead = Command::new("/bin/sh")
        .args(["-c", "echo $$"])
        .output_within(Duration::from_secs(30))
        .expect("sh ran");
    let job = Job {
        id: 1,
        label: "unrecorded".to_owned(),
        priority: Priority::Fix,
        worktree: q.work(),
        command: vec!["/usr/bin/true".to_owned()],
        env,
        timeout_secs: 40,
        submitted: began - ago,
        state: State::Running,
        attempts: 1,
        started: Some(began - ago),
        finished: None,
        pgid: Some(pid),
        leader_start: None,
        runner_pid: String::from_utf8_lossy(&dead.stdout).trim().parse().ok(),
        exit: None,
        cancel_requested: false,
        note: None,
    };
    let queue = Queue::at(q.dir.to_path_buf());
    queue.save(&job).expect("job planted");

    let (code, out) = q.wait(1);
    let leader_left = !until(5, || !alive(pid));
    let member_left = !until(5, || !alive(member));
    kill("-9", &format!("-{pid}"));
    let _ = reaper.join();
    let starts = queue.load(1).expect("job file").attempts;
    q.finish();
    assert!(
        code == 0 && starts == 2 && !leader_left && !member_left,
        "an unrecorded group not ended whole before the second start: status {code}, {out:?}, starts {starts}, \
         leader alive {leader_left}, member alive {member_left}"
    );
}

#[test]
fn qa_r658_restart_ends_an_unrecorded_group_whole() {
    unrecorded_group_case("unrecorded", 0);
}

negative_control!(
    qa_r658_restart_ends_an_unrecorded_group_whole,
    "a live group whose leader started an hour from the job's start is left alone, and the check sees it",
    expected = "an unrecorded group not ended whole",
    unrecorded_group_case("unrecorded_control", 3600)
);
