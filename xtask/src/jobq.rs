//! `cargo xtask job`: the local job queue for heavy commands (R-658). No agent runs a heavy command (anything that
//! compiles: cargo build, check, test, clippy, doc, mutants, nextest, benches, GPU tests, `cargo xtask` runners) on
//! the Mac directly; it submits it here, and one runner process runs the queue at a fixed concurrency.
//!
//! - `submit <worktree> --priority <p> --label <l> [--timeout <s>] [--env K=V]... -- <command>...` writes a job and
//!   prints its id; it starts the runner if none is alive. `CARGO_TARGET_DIR` must be one of the `--env`s, so no job
//!   builds in a target directory it doesn't name.
//! - `wait <id>` blocks until the job ends, prints its end and its log, and exits with its status
//!   ([`Job::status_code`]). It can be re-run at any time, from any process: the job doesn't belong to its submitter.
//! - `status` lists the runner, the running jobs, the queue in the order it will run, and the recent jobs.
//! - `cancel <id>` drops a queued job, or ends a running one with its process group.
//! - `install` copies this executable to `<root>/bin/xtask`, the stable client the agents call, so that calling the
//!   queue compiles nothing.
//! - `runner [<dir>]` is the runner itself, started by `submit` (and by `wait`, should the runner have died), with the
//!   state directory as its argument so that `ps` shows which queue it serves.
//!
//! Priorities, highest first: `merge` (merge-ready verification of a final head), `fix` (fix-round tests), `build`
//! (new-task builds), `peek` (exploratory runs); first in, first out within a class.
//!
//! The state lives under `<root>`, `$PRINCIPIA_JOBQ` or [`DEFAULT_ROOT`], outside the repository: `jobs/<id>.json`, one
//! file per job, moved to `archive/` a day after it ends; `logs/<id>.log`, each job's output; `runner.lock`, the
//! runner's pid, heartbeat, concurrency and cores per job; `state.lock`, an advisory file lock (`flock`) every change to
//! the state holds briefly, naming its holder, released by the kernel if its holder dies; `config`, optional settings
//! ([`Config`]); `runner.log`, the runner's own log; `violations.log`.
//!
//! A runner lock whose pid is dead, or whose heartbeat is older than `stale` seconds, has expired: the next `submit`
//! or `wait` starts a runner, which takes the lock over. A new runner ends the process group of every job still marked
//! running (a dead runner's), unless its leader is alive and not the job's ([`leader_is_jobs`]), then queues it again,
//! or marks it lost after [`MAX_ATTEMPTS`] starts. A job runs in a
//! process group of its own; past its timeout, or when cancelled, the runner sends the group SIGTERM, then SIGKILL
//! after `grace` seconds; and whenever the job's own process ends, the runner kills what is left of its group. A runner with nothing queued or running for `idle` seconds exits, so none is left behind.
//! Every group signal passes [`may_signal_group`], and a pass starts at most `concurrency` jobs.
//!
//! Two limits of that recovery. A runner killed between a job's spawn and its save leaves that start untracked: the
//! job's file still says queued, so the job runs again while that one copy has no timeout or cancel and may overlap the
//! second start; `runner.log` shows the start, `violations.log` a heavy copy, and no lock is orphaned. And when a job's
//! file holds no leader start time, a live leader that started more than [`START_SLACK`] seconds from the job's recorded
//! start is left alone, and the job queued again.
//!
//! A live runner whose save fails runs nothing its files don't record. After a spawn it kills and reaps that start's
//! group, and the job stays queued; at a job's end it keeps the job and saves it again on the next pass. A pass with a
//! failed save starts no job and counts as idle, so a runner that can't write exits.
//!
//! While it runs, the runner reads `ps` every `sample` seconds and logs to `violations.log` each heavy process
//! ([`heavy`]) that no queue job started, with its pid, cwd and command. It never kills one.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::Write as _;
use std::os::unix::process::{CommandExt as _, ExitStatusExt as _};
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, ExitCode, ExitStatus, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// The queue's state directory on the Mac, outside the repository (R-658).
pub const DEFAULT_ROOT: &str = "/Volumes/X10 Pro/Principia/orch/jobq";

/// The environment variable naming another state directory, as the queue's tests use.
pub const ROOT_VAR: &str = "PRINCIPIA_JOBQ";

/// How many times a job may be started: a job a dead runner left running is queued again until it has started this
/// many times, and then marked lost, so a job that kills its runner cannot loop.
pub const MAX_ATTEMPTS: u32 = 3;

/// How long after it ends a job's file moves from `jobs/` to `archive/`, in seconds, so the runner's passes read only
/// the recent ones.
pub const ARCHIVE_AFTER: u64 = 86_400;

/// The fewest seconds between two runner starts by `submit` or `wait`.
pub const SPAWN_GAP: u64 = 5;

/// How many seconds a live group leader's start may be from a job's recorded start, [`Job::started`], for a new runner
/// to take that leader as the job's when the job's file holds no [`Job::leader_start`] yet: a runner that died between
/// starting the job and recording its leader's start time ([`leader_is_jobs`]). The leader starts within moments of
/// `started`; a reused pid would need the system's pids to wrap round within these seconds.
pub const START_SLACK: u64 = 5;

/// The usage text `cargo xtask --help` shows for `job`.
pub const USAGE: &str = "\
  job submit <worktree> --priority (merge|fix|build|peek) --label <label> [--timeout <s>] [--env K=V]...
             -- <command>...
                                  queue a heavy command to run in <worktree> (R-658); prints the job id and starts
                                  the runner if none is alive. --env CARGO_TARGET_DIR=<dir> is required; the job gets
                                  CARGO_BUILD_JOBS and RUST_TEST_THREADS at the core count unless --env sets them
  job wait <id>                   block until the job ends, print its end and log path, exit with its status (its
                                  exit code; 124 timed out, 125 lost, 126 not started, 130 cancelled); re-runnable
  job status                      the runner, the running jobs, the queue in run order and the recent jobs
  job cancel <id>                 drop a queued job, or end a running one with its process group
  job install                     copy this executable to <root>/bin/xtask, the client agents call
  job runner [<dir>]              run the queue, or the one in <dir> (started by submit; not for direct use)";

/// A job's priority class (R-658), highest first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    /// Merge-ready verification of a final head.
    Merge,
    /// Fix-round tests.
    Fix,
    /// New-task builds.
    Build,
    /// Exploratory runs.
    Peek,
}

impl Priority {
    /// Reads a class by its name.
    pub fn parse(name: &str) -> Result<Self, String> {
        match name {
            "merge" => Ok(Priority::Merge),
            "fix" => Ok(Priority::Fix),
            "build" => Ok(Priority::Build),
            "peek" => Ok(Priority::Peek),
            other => Err(format!(
                "job: --priority takes merge, fix, build or peek, not `{other}`"
            )),
        }
    }

    /// The class's name.
    pub fn name(self) -> &'static str {
        match self {
            Priority::Merge => "merge",
            Priority::Fix => "fix",
            Priority::Build => "build",
            Priority::Peek => "peek",
        }
    }
}

/// Where a job is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum State {
    /// Waiting to run.
    Queued,
    /// Running under the runner named in [`Job::runner_pid`].
    Running,
    /// Ended by itself, with [`Job::exit`].
    Exited,
    /// Ended by the runner at its timeout.
    TimedOut,
    /// Cancelled, queued or running.
    Cancelled,
    /// Left running by dead runners [`MAX_ATTEMPTS`] times.
    Lost,
    /// Its command could not be started.
    NotStarted,
}

impl State {
    /// Whether the job has ended.
    pub fn is_final(self) -> bool {
        !matches!(self, State::Queued | State::Running)
    }

    fn name(self) -> &'static str {
        match self {
            State::Queued => "queued",
            State::Running => "running",
            State::Exited => "exited",
            State::TimedOut => "timed out",
            State::Cancelled => "cancelled",
            State::Lost => "lost",
            State::NotStarted => "not started",
        }
    }
}

/// One job: its file, `jobs/<id>.json`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Job {
    pub id: u64,
    pub label: String,
    pub priority: Priority,
    pub worktree: PathBuf,
    pub command: Vec<String>,
    /// The job's environment, applied over the runner's defaults ([`Job::env_for`]).
    pub env: BTreeMap<String, String>,
    pub timeout_secs: u64,
    /// When it was submitted, in Unix seconds.
    pub submitted: u64,
    pub state: State,
    /// How many times it has been started.
    pub attempts: u32,
    pub started: Option<u64>,
    pub finished: Option<u64>,
    /// Its process group, the id of its command's process.
    pub pgid: Option<i32>,
    /// When its command's process started, as `ps -o lstart=` gives it in the C locale and UTC, to tell that process from a later one with
    /// the same pid.
    pub leader_start: Option<String>,
    pub runner_pid: Option<i32>,
    /// Its exit code, or 128 plus the signal that ended it.
    pub exit: Option<i32>,
    pub cancel_requested: bool,
    /// Why it ended as it did, or was queued again.
    pub note: Option<String>,
}

impl Job {
    /// The status `job wait` exits with: the job's own exit code (128 plus the signal for one a signal ended), 124
    /// timed out, 125 lost, 126 not started, 130 cancelled, as `timeout(1)` and shells use them.
    pub fn status_code(&self) -> i32 {
        match self.state {
            State::Exited => self.exit.unwrap_or(1),
            State::TimedOut => 124,
            State::Lost => 125,
            State::NotStarted => 126,
            State::Cancelled => 130,
            State::Queued | State::Running => 1,
        }
    }

    /// The environment the job runs with: `defaults` (the runner's: the core count for `CARGO_BUILD_JOBS` and
    /// `RUST_TEST_THREADS`, and the config's `env` lines), then the job's own, which win.
    pub fn env_for(&self, defaults: &BTreeMap<String, String>) -> BTreeMap<String, String> {
        let mut env = defaults.clone();
        env.extend(self.env.clone());
        env.insert("PRINCIPIA_JOBQ_JOB".to_owned(), self.id.to_string());
        env
    }

    /// The target directory the job builds in, as given.
    pub fn target_dir(&self) -> Option<&str> {
        self.env.get("CARGO_TARGET_DIR").map(String::as_str)
    }

    /// The key its target directory is compared by ([`target_key`]). The command runs in the job's worktree, so a
    /// relative directory is that worktree's, whatever the runner's own working directory is.
    pub fn target_key(&self) -> Option<String> {
        self.target_dir()
            .map(|dir| target_key(&self.worktree.join(dir).to_string_lossy()))
    }
}

/// The key two target directories are compared by (R-658): the path walked from its start, each prefix made canonical
/// where it exists and joined on as written where it doesn't, and a `..` taking off the last component, so that one
/// below a missing component resolves too; without `.` components or repeated or trailing `/`; then folded to lower
/// case, since the Mac's volume is case-insensitive. Two spellings of one directory give one key. (On a case-sensitive
/// volume two directories differing only in case share a key, and so run one after the other: never at once on one
/// directory, which is the point.)
pub fn target_key(dir: &str) -> String {
    let mut key = PathBuf::new();
    for component in Path::new(dir).components() {
        // What is on the key is canonical or missing, so taking off its last component is what `..` means. A relative
        // path's leading `..` has nothing to take off, and is joined on.
        if component == Component::ParentDir && !key.as_os_str().is_empty() {
            key.pop();
            continue;
        }
        let next = key.join(component);
        key = fs::canonicalize(&next).unwrap_or(next);
    }
    key.to_string_lossy().to_lowercase()
}

/// The settings in `<root>/config`, one `key = value` per line, `#` starting a comment; each key not given keeps its
/// default. `env KEY=VALUE` lines add to the environment every job starts with (as sccache's would).
#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    /// Jobs run at once.
    pub concurrency: usize,
    /// A job's timeout when its submit gives none, in seconds.
    pub timeout: u64,
    /// The age, in seconds, past which a runner lock's heartbeat has expired.
    pub stale: u64,
    /// Seconds between two readings of `ps` for violations.
    pub sample: u64,
    /// Seconds a runner with nothing queued or running waits before it exits.
    pub idle: u64,
    /// Seconds between SIGTERM and SIGKILL to a job's group.
    pub grace: u64,
    /// Milliseconds between two passes of the runner's loop.
    pub tick_ms: u64,
    /// Seconds a call waits for the state lock before it gives up, naming the lock's holder.
    pub lock_wait: u64,
    /// `CARGO_BUILD_JOBS` and `RUST_TEST_THREADS` for each job: the core count unless set.
    pub cores: usize,
    /// The `env` lines.
    pub env: BTreeMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            // Measured on 30 Oct 2026 (R-658): two jobs at a time verified three task heads in 97-101 s, against
            // 135 s one at a time.
            concurrency: 2,
            timeout: 3600,
            stale: 120,
            sample: 30,
            idle: 60,
            grace: 10,
            tick_ms: 500,
            lock_wait: 30,
            cores: std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get),
            env: BTreeMap::new(),
        }
    }
}

impl Config {
    /// Reads `text`, a config file's contents, over the defaults.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut config = Config::default();
        for (n, line) in text.lines().enumerate() {
            let line = line.split('#').next().unwrap_or_default().trim();
            if line.is_empty() {
                continue;
            }
            let bad = |what: &str| format!("job config line {}: {what}: `{line}`", n + 1);
            if let Some(pair) = line.strip_prefix("env ") {
                let (key, value) = pair
                    .trim()
                    .split_once('=')
                    .ok_or_else(|| bad("env takes KEY=VALUE"))?;
                config
                    .env
                    .insert(key.trim().to_owned(), value.trim().to_owned());
                continue;
            }
            let (key, value) = line.split_once('=').ok_or_else(|| bad("not key = value"))?;
            let number: u64 = value
                .trim()
                .parse()
                .map_err(|_| bad("not a whole number"))?;
            if number == 0 {
                return Err(bad("must be at least 1"));
            }
            match key.trim() {
                "concurrency" => config.concurrency = number as usize,
                "timeout" => config.timeout = number,
                "stale" => config.stale = number,
                "sample" => config.sample = number,
                "idle" => config.idle = number,
                "grace" => config.grace = number,
                "tick_ms" => config.tick_ms = number,
                "lock_wait" => config.lock_wait = number,
                "cores" => config.cores = number as usize,
                _ => return Err(bad("unknown key")),
            }
        }
        Ok(config)
    }

    /// The environment each job starts with, under its own.
    pub fn job_defaults(&self) -> BTreeMap<String, String> {
        let mut env = BTreeMap::new();
        env.insert("CARGO_BUILD_JOBS".to_owned(), self.cores.to_string());
        env.insert("RUST_TEST_THREADS".to_owned(), self.cores.to_string());
        env.extend(self.env.clone());
        env
    }
}

/// The next job to start: the first queued one by class, then by id (first in, first out), whose target directory
/// no running job uses, compared by [`target_key`], since two cargo runs on one target directory serialise on its lock.
pub fn next_job(jobs: &[Job]) -> Option<&Job> {
    let busy: BTreeSet<String> = jobs
        .iter()
        .filter(|job| job.state == State::Running)
        .filter_map(Job::target_key)
        .collect();
    jobs.iter()
        .filter(|job| job.state == State::Queued)
        .filter(|job| job.target_key().is_none_or(|key| !busy.contains(&key)))
        .min_by_key(|job| (job.priority, job.id))
}

/// Whether `job`'s file moves to the archive at `now`: it ended [`ARCHIVE_AFTER`] seconds ago or more.
pub fn archivable(job: &Job, now: u64) -> bool {
    job.state.is_final()
        && job
            .finished
            .is_some_and(|finished| now.saturating_sub(finished) >= ARCHIVE_AFTER)
}

/// The queued jobs in the order they will run, ignoring target directories.
pub fn run_order(jobs: &[Job]) -> Vec<&Job> {
    let mut queued: Vec<&Job> = jobs
        .iter()
        .filter(|job| job.state == State::Queued)
        .collect();
    queued.sort_by_key(|job| (job.priority, job.id));
    queued
}

/// The runner's lock: its pid and its last heartbeat, in Unix seconds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunnerLock {
    pub pid: i32,
    pub heartbeat: u64,
}

impl RunnerLock {
    /// Reads a lock file's text, `pid <n>` and `heartbeat <secs>` lines; `None` for any other.
    pub fn parse(text: &str) -> Option<Self> {
        let mut pid = None;
        let mut heartbeat = None;
        for line in text.lines() {
            match line.split_once(' ') {
                Some(("pid", value)) => pid = value.trim().parse().ok(),
                Some(("heartbeat", value)) => heartbeat = value.trim().parse().ok(),
                _ => {}
            }
        }
        Some(RunnerLock {
            pid: pid?,
            heartbeat: heartbeat?,
        })
    }

    fn text(self) -> String {
        format!("pid {}\nheartbeat {}\n", self.pid, self.heartbeat)
    }

    /// Whether the lock still holds at `now`: its pid is alive and its heartbeat is younger than `stale` seconds. A
    /// dead pid, or a stale heartbeat (a hung runner, or a reused pid), expires it.
    pub fn is_live(self, now: u64, stale: u64, alive: impl Fn(i32) -> bool) -> bool {
        alive(self.pid) && now.saturating_sub(self.heartbeat) < stale
    }
}

/// The runner's own settings, from its lock's text: `concurrency <n>, <m> cores per job`, a `?` for each it doesn't
/// give (a runner from before they were written).
pub fn runner_settings(text: &str) -> String {
    let field = |key: &str| {
        text.lines()
            .find_map(|line| line.strip_prefix(key)?.trim().parse::<usize>().ok())
            .map_or_else(|| "?".to_owned(), |n| n.to_string())
    };
    format!(
        "concurrency {}, {} cores per job",
        field("concurrency "),
        field("cores ")
    )
}

/// Whether a process with this pid exists (ours or not). A pid of zero or below, as a damaged lock file could give,
/// names no process.
pub fn pid_alive(pid: i32) -> bool {
    use rustix::io::Errno;
    if pid <= 0 {
        return false;
    }
    match rustix::process::Pid::from_raw(pid) {
        Some(pid) => !matches!(rustix::process::test_kill_process(pid), Err(Errno::SRCH)),
        None => false,
    }
}

/// Whether process group `pgid` may be signalled by the process `own_pid` in group `own_group`: the one check in front
/// of every group signal (`signal_group`). `kill(-1, ...)`, which is what signalling group 1 comes to, reaches every
/// process this user may signal, and group 0 is the caller's own; ids below those are not groups at all; and neither
/// the runner's own pid nor group can be a job's, since each job leads a group of its own.
///
/// Its tests call only this function. None may ever plant a group this refuses and let the runner signal it: with this
/// check mutated, that would be a real `kill(-1, SIGKILL)`.
pub fn may_signal_group(pgid: i32, own_pid: i32, own_group: i32) -> bool {
    pgid > 1 && pgid != own_pid && pgid != own_group
}

/// Now, in Unix seconds.
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Whether a runner started at `last` is too recent for another at `now`: under [`SPAWN_GAP`] seconds ago, or, with the
/// clock stepped back, in the future.
pub fn spawn_too_soon(now: u64, last: u64) -> bool {
    now.saturating_sub(last) < SPAWN_GAP
}

/// Records `start`, what `ps` read as the start time of leader `pid`, into `job`, if `job` is still running under that
/// leader with no start recorded: the state lock was let go between the job's start and this, so it may have ended,
/// or changed hands. Returns whether `job` changed, and so wants saving.
pub fn record_start(job: &mut Job, pid: i32, start: Option<String>) -> bool {
    let ours = job.state == State::Running && job.pgid == Some(pid) && job.leader_start.is_none();
    if ours {
        job.leader_start = start;
    }
    ours
}

/// The queue's state directory.
#[derive(Clone, Debug)]
pub struct Queue {
    root: PathBuf,
}

/// The state lock, held while it lives. Its holder's pid and the time it took it are in the file meanwhile, and
/// cleared as it is let go.
struct Held {
    file: File,
}

impl Drop for Held {
    fn drop(&mut self) {
        let _ = self.file.set_len(0);
    }
}

/// Who holds the state lock, from the lock file's text (`pid <n> since <secs>`) at `now`: `pid <n>, for <age> s`, or,
/// when the text names none, that the holder is unknown.
pub fn lock_holder(text: &str, now: u64) -> String {
    let field = |key: &str| {
        text.lines()
            .find_map(|line| line.strip_prefix(key)?.trim().parse::<u64>().ok())
    };
    match (field("pid "), field("since ")) {
        (Some(pid), Some(since)) => format!("pid {pid}, for {} s", now.saturating_sub(since)),
        (Some(pid), None) => format!("pid {pid}"),
        _ => "a process that does not say which".to_owned(),
    }
}

impl Queue {
    /// The queue at `$PRINCIPIA_JOBQ`, or at [`DEFAULT_ROOT`].
    pub fn from_env() -> Self {
        let root =
            std::env::var_os(ROOT_VAR).map_or_else(|| PathBuf::from(DEFAULT_ROOT), PathBuf::from);
        Queue { root }
    }

    /// The queue at `root`.
    pub fn at(root: impl Into<PathBuf>) -> Self {
        Queue { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn jobs_dir(&self) -> PathBuf {
        self.root.join("jobs")
    }

    /// A job's file.
    pub fn job_path(&self, id: u64) -> PathBuf {
        self.jobs_dir().join(format!("{id:06}.json"))
    }

    /// An archived job's file.
    pub fn archive_path(&self, id: u64) -> PathBuf {
        self.root.join("archive").join(format!("{id:06}.json"))
    }

    /// A job's log.
    pub fn log_path(&self, id: u64) -> PathBuf {
        self.root.join("logs").join(format!("{id:06}.log"))
    }

    pub fn lock_path(&self) -> PathBuf {
        self.root.join("runner.lock")
    }

    pub fn violations_path(&self) -> PathBuf {
        self.root.join("violations.log")
    }

    /// Makes the state directory's folders.
    fn ensure(&self) -> Result<(), String> {
        for dir in [
            self.jobs_dir(),
            self.root.join("archive"),
            self.root.join("logs"),
            self.root.join("bin"),
        ] {
            fs::create_dir_all(&dir)
                .map_err(|e| format!("job: cannot make {}: {e}", dir.display()))?;
        }
        Ok(())
    }

    /// Takes the state lock, waiting up to the config's `lock_wait` for it; past that, an error naming its holder. It
    /// makes no folder: a runner whose state directory has gone exits.
    fn hold(&self, config: &Config) -> Result<Held, String> {
        self.try_hold(Duration::from_secs(config.lock_wait))?
    }

    /// Takes the state lock, waiting up to `wait` for it: the outer error is a lock that can't be opened or taken at
    /// all, the inner one a lock another process held all the while, naming it. Once taken, the lock file names this
    /// process and the time.
    fn try_hold(&self, wait: Duration) -> Result<Result<Held, String>, String> {
        use std::os::unix::fs::FileExt as _;
        let path = self.root.join("state.lock");
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|e| format!("job: cannot open {}: {e}", path.display()))?;
        let from = Instant::now();
        loop {
            match file.try_lock() {
                Ok(()) => {
                    let me = format!("pid {}\nsince {}\n", std::process::id(), now());
                    let _ = file
                        .set_len(0)
                        .and_then(|()| file.write_all_at(me.as_bytes(), 0));
                    return Ok(Ok(Held { file }));
                }
                Err(std::fs::TryLockError::WouldBlock) => {}
                Err(std::fs::TryLockError::Error(e)) => {
                    return Err(format!("job: cannot lock {}: {e}", path.display()));
                }
            }
            if from.elapsed() >= wait {
                let text = fs::read_to_string(&path).unwrap_or_default();
                return Ok(Err(format!(
                    "job: the state lock {} is held by {}; gave up after {} s",
                    path.display(),
                    lock_holder(&text, now()),
                    wait.as_secs()
                )));
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// The settings: `<root>/config` over the defaults.
    pub fn config(&self) -> Result<Config, String> {
        match fs::read_to_string(self.root.join("config")) {
            Ok(text) => Config::parse(&text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
            Err(e) => Err(format!("job: cannot read the config: {e}")),
        }
    }

    /// Writes `text` to `path` whole: to a temporary file beside it, then renamed over it.
    fn write_atomic(path: &Path, text: &str) -> Result<(), String> {
        let tmp = path.with_extension(format!("tmp{}", std::process::id()));
        fs::write(&tmp, text)
            .and_then(|()| fs::rename(&tmp, path))
            .map_err(|e| format!("job: cannot write {}: {e}", path.display()))
    }

    /// Writes a job's file.
    pub fn save(&self, job: &Job) -> Result<(), String> {
        let text = serde_json::to_string_pretty(job).map_err(|e| format!("job: {e}"))?;
        Self::write_atomic(&self.job_path(job.id), &text)
    }

    /// Reads a job's file.
    pub fn load(&self, id: u64) -> Result<Job, String> {
        let path = Some(self.job_path(id))
            .filter(|path| path.exists())
            .unwrap_or_else(|| self.archive_path(id));
        let text = fs::read_to_string(&path)
            .map_err(|e| format!("job {id}: cannot read {}: {e}", path.display()))?;
        serde_json::from_str(&text).map_err(|e| format!("job {id}: {}: {e}", path.display()))
    }

    /// Every job whose file can be read, by id.
    pub fn jobs(&self) -> Result<Vec<Job>, String> {
        let mut jobs = Vec::new();
        let entries = match fs::read_dir(self.jobs_dir()) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(jobs),
            Err(e) => return Err(format!("job: cannot list the jobs: {e}")),
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(id) = name
                .to_str()
                .and_then(|n| n.strip_suffix(".json"))
                .and_then(|n| n.parse().ok())
            else {
                continue;
            };
            // A job file that can't be read is skipped, and said so, rather than stopping the queue.
            match self.load(id) {
                Ok(job) => jobs.push(job),
                Err(message) => eprintln!("{} {message}; skipped", utc(now())),
            }
        }
        jobs.sort_by_key(|job| job.id);
        Ok(jobs)
    }

    /// The runner lock, if there is one to read.
    pub fn runner_lock(&self) -> Option<RunnerLock> {
        fs::read_to_string(self.lock_path())
            .ok()
            .as_deref()
            .and_then(RunnerLock::parse)
    }

    /// Whether a live runner holds the lock now.
    fn runner_live(&self, config: &Config) -> bool {
        self.runner_lock()
            .is_some_and(|lock| lock.is_live(now(), config.stale, pid_alive))
    }

    /// Queues `command` to run in `worktree`, returning its id; starts the runner if none is alive.
    pub fn submit(&self, request: Submit) -> Result<u64, String> {
        if request.command.is_empty() {
            return Err("job submit: no command after --".to_owned());
        }
        if request.label.trim().is_empty() || request.label.contains('\n') {
            return Err("job submit: --label takes a one-line, non-empty label".to_owned());
        }
        if !request.env.contains_key("CARGO_TARGET_DIR") {
            return Err(
                "job submit: --env CARGO_TARGET_DIR=<dir> is required: each job builds in its own target directory"
                    .to_owned(),
            );
        }
        // Absolute, but as given: not canonicalised, which on a case-insensitive volume may change the spelling, and
        // with it every path a build hashes.
        let worktree = std::path::absolute(&request.worktree)
            .ok()
            .filter(|path| path.is_dir())
            .ok_or_else(|| {
                format!(
                    "job submit: {} is not a directory",
                    request.worktree.display()
                )
            })?;
        let config = self.config()?;
        let mut env = snapshot_env(std::env::vars());
        env.extend(request.env);
        self.ensure()?;
        let _held = self.hold(&config)?;
        let id = self.next_id()?;
        let job = Job {
            id,
            label: request.label,
            priority: request.priority,
            worktree,
            command: request.command,
            env,
            timeout_secs: request.timeout.unwrap_or(config.timeout),
            submitted: now(),
            state: State::Queued,
            attempts: 0,
            started: None,
            finished: None,
            pgid: None,
            leader_start: None,
            runner_pid: None,
            exit: None,
            cancel_requested: false,
            note: None,
        };
        self.save(&job)?;
        self.ensure_runner_held(&config)?;
        Ok(id)
    }

    /// The next job id, from `<root>/next_id`; the state lock is held.
    fn next_id(&self) -> Result<u64, String> {
        let path = self.root.join("next_id");
        let current: u64 = match fs::read_to_string(&path) {
            Ok(text) => text
                .trim()
                .parse()
                .map_err(|_| format!("job: {} is not a number", path.display()))?,
            Err(_) => self.jobs()?.last().map_or(1, |job| job.id + 1),
        };
        Self::write_atomic(&path, &format!("{}\n", current + 1))?;
        Ok(current)
    }

    /// Starts a runner unless a live one holds the lock.
    pub fn ensure_runner(&self) -> Result<(), String> {
        let config = self.config()?;
        let _held = self.hold(&config)?;
        self.ensure_runner_held(&config)
    }

    /// [`Queue::ensure_runner`], the state lock held: the runner starts by taking it, so it starts after this returns.
    ///
    /// It starts at most one runner per [`SPAWN_GAP`] seconds, whatever the lock says, by the time of the last start
    /// it records in `runner.spawned`: a runner that cannot take the lock, or a lock that never reads as live, can't
    /// make every call start another. The runner is reaped by a thread of this process, or, once this process exits, by
    /// launchd, so none is left a zombie.
    fn ensure_runner_held(&self, config: &Config) -> Result<(), String> {
        if self.runner_live(config) {
            return Ok(());
        }
        self.ensure()?;
        let spawned = self.root.join("runner.spawned");
        let last: u64 = fs::read_to_string(&spawned)
            .ok()
            .and_then(|text| text.trim().parse().ok())
            .unwrap_or(0);
        if spawn_too_soon(now(), last) {
            return Ok(());
        }
        Self::write_atomic(&spawned, &format!("{}\n", now()))?;
        let exe = std::env::current_exe()
            .map_err(|e| format!("job: cannot find this executable: {e}"))?;
        let log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.root.join("runner.log"))
            .map_err(|e| format!("job: cannot open the runner's log: {e}"))?;
        let err = log
            .try_clone()
            .map_err(|e| format!("job: cannot open the runner's log: {e}"))?;
        // The state directory is the runner's argument as well as its environment's, so `ps` shows which queue a
        // runner serves.
        let mut child = Command::new(exe)
            .arg("job")
            .arg("runner")
            .arg(&self.root)
            .env(ROOT_VAR, &self.root)
            .stdin(Stdio::null())
            .stdout(log)
            .stderr(err)
            .process_group(0)
            .spawn()
            .map_err(|e| format!("job: cannot start the runner: {e}"))?;
        std::thread::spawn(move || child.wait());
        Ok(())
    }

    /// Waits for job `id` to end, restarting the runner should it die meanwhile.
    pub fn wait(&self, id: u64) -> Result<Job, String> {
        // Each poll checks the runner too; `ensure_runner`'s gap keeps that to one start per [`SPAWN_GAP`].
        loop {
            let job = self.load(id)?;
            if job.state.is_final() {
                return Ok(job);
            }
            self.ensure_runner()?;
            std::thread::sleep(Duration::from_millis(500));
        }
    }

    /// Cancels job `id`: a queued job at once; a running one is ended, with its group, by the runner.
    pub fn cancel(&self, id: u64) -> Result<String, String> {
        let _held = self.hold(&self.config()?)?;
        let mut job = self.load(id)?;
        match job.state {
            State::Queued => {
                job.state = State::Cancelled;
                job.finished = Some(now());
                job.note = Some("cancelled while queued".to_owned());
                self.save(&job)?;
                Ok(format!("job {id}: cancelled"))
            }
            State::Running => {
                job.cancel_requested = true;
                self.save(&job)?;
                Ok(format!(
                    "job {id}: the runner ends it and its process group"
                ))
            }
            state => Err(format!("job {id} has already ended: {}", state.name())),
        }
    }

    /// The queue's state, for `job status`.
    pub fn status(&self) -> Result<String, String> {
        let config = self.config()?;
        let jobs = self.jobs()?;
        let now = now();
        let mut out = String::new();
        match self.runner_lock() {
            Some(lock) if lock.is_live(now, config.stale, pid_alive) => out.push_str(&format!(
                "runner: pid {}, heartbeat {} s ago, {}\n",
                lock.pid,
                now.saturating_sub(lock.heartbeat),
                runner_settings(&fs::read_to_string(self.lock_path()).unwrap_or_default())
            )),
            Some(lock) => out.push_str(&format!(
                "runner: none alive (lock of pid {} expired)\n",
                lock.pid
            )),
            None => out.push_str("runner: none\n"),
        }
        let line = |job: &Job, what: String| {
            format!(
                "  {:>5}  {:<5}  {:<14}  {:<22}  {}  [{}]\n",
                job.id,
                job.priority.name(),
                job.label,
                what,
                job.command.join(" "),
                job.worktree.display()
            )
        };
        out.push_str("running:\n");
        for job in jobs.iter().filter(|job| job.state == State::Running) {
            let ran = now.saturating_sub(job.started.unwrap_or(now));
            out.push_str(&line(job, format!("{ran} s of {} s", job.timeout_secs)));
        }
        out.push_str("queued, in run order:\n");
        for job in run_order(&jobs) {
            let waited = now.saturating_sub(job.submitted);
            out.push_str(&line(job, format!("waiting {waited} s")));
        }
        out.push_str("recent:\n");
        let mut ended: Vec<&Job> = jobs.iter().filter(|job| job.state.is_final()).collect();
        ended.sort_by_key(|job| (job.finished, job.id));
        for job in ended.iter().rev().take(10) {
            out.push_str(&line(job, end_text(job)));
        }
        Ok(out)
    }

    /// Copies `exe` to `<root>/bin/xtask`, the client agents call (R-658).
    pub fn install(&self, exe: &Path) -> Result<PathBuf, String> {
        self.ensure()?;
        let dest = self.root.join("bin").join("xtask");
        let tmp = dest.with_extension("tmp");
        fs::copy(exe, &tmp)
            .and_then(|_| fs::rename(&tmp, &dest))
            .map_err(|e| {
                format!(
                    "job install: cannot copy {} to {}: {e}",
                    exe.display(),
                    dest.display()
                )
            })?;
        Ok(dest)
    }

    /// Runs the queue until it has been idle for the config's `idle` seconds, or another runner has taken the lock
    /// over. Returns at once if a live runner holds the lock.
    pub fn run_runner(&self) -> Result<(), String> {
        let me = std::process::id() as i32;
        let mut runner = Runner {
            queue: self.clone(),
            me,
            children: HashMap::new(),
            logged: HashSet::new(),
            unrecorded: Vec::new(),
        };
        let config = self.config()?;
        let left = {
            let _held = self.hold(&config)?;
            if self.runner_live(&config) {
                return Ok(());
            }
            self.ensure()?;
            self.write_lock(me, &config)?;
            log(&format!("runner {me}: started"));
            self.jobs()?
                .into_iter()
                .filter(|job| job.state == State::Running)
                .collect::<Vec<_>>()
        };
        // Each job a dead runner left running is ended outside the state lock, then recorded under it.
        for job in left {
            if !runner.recover(&job, &config)? {
                return Ok(());
            }
        }
        let mut idle_since = Instant::now();
        let mut sampled: Option<Instant> = None;
        loop {
            let config = match self.config() {
                Ok(config) => config,
                Err(message) => {
                    log(&format!("runner {me}: {message}; using the defaults"));
                    Config::default()
                }
            };
            {
                // A lock held past `lock_wait` by a hung process is logged, and taken again on the next pass.
                let _held = match self.try_hold(Duration::from_secs(config.lock_wait))? {
                    Ok(held) => held,
                    Err(message) => {
                        log(&format!("runner {me}: {message}; trying again"));
                        std::thread::sleep(Duration::from_millis(config.tick_ms));
                        continue;
                    }
                };
                match self.runner_lock() {
                    Some(lock) if lock.pid == me => {}
                    _ => {
                        log(&format!(
                            "runner {me}: another runner holds the lock; exiting"
                        ));
                        return Ok(());
                    }
                }
                self.write_lock(me, &config)?;
                // A pass that fails (the state directory unreadable) is logged and tried again, and counts as idle, so a
                // runner that can do nothing exits.
                let busy = runner.pass(&config).unwrap_or_else(|message| {
                    log(&format!("runner {me}: {message}"));
                    false
                });
                if busy {
                    idle_since = Instant::now();
                } else if idle_since.elapsed() >= Duration::from_secs(config.idle) {
                    fs::remove_file(self.lock_path())
                        .map_err(|e| format!("job: cannot remove the lock: {e}"))?;
                    log(&format!("runner {me}: idle for {} s; exiting", config.idle));
                    return Ok(());
                }
            }
            runner.record_starts(&config);
            if sampled.is_none_or(|at| at.elapsed() >= Duration::from_secs(config.sample)) {
                sampled = Some(Instant::now());
                if let Err(message) = runner.sample_violations() {
                    log(&format!("runner {me}: violation sample failed: {message}"));
                }
            }
            std::thread::sleep(Duration::from_millis(config.tick_ms));
        }
    }

    /// Writes the runner lock: `pid`, the heartbeat, and the runner's own concurrency and cores per job, which
    /// `status` shows (a client built with other defaults would otherwise show its own).
    fn write_lock(&self, pid: i32, config: &Config) -> Result<(), String> {
        let lock = RunnerLock {
            pid,
            heartbeat: now(),
        };
        let text = format!(
            "{}concurrency {}\ncores {}\n",
            lock.text(),
            config.concurrency,
            config.cores
        );
        Self::write_atomic(&self.lock_path(), &text)
    }
}

/// How a job ended, in words.
fn end_text(job: &Job) -> String {
    let ran = match (job.started, job.finished) {
        (Some(started), Some(finished)) => format!(" after {} s", finished.saturating_sub(started)),
        _ => String::new(),
    };
    match job.state {
        State::Exited => format!("exited {}{ran}", job.exit.unwrap_or(1)),
        state => format!("{}{ran}", state.name()),
    }
}

/// A line in the runner's log (its standard error, `runner.log`).
fn log(line: &str) {
    eprintln!("{} {line}", utc(now()));
}

/// Unix seconds as a UTC time, `2026-10-30T13:46:25Z`.
pub fn utc(secs: u64) -> String {
    // Days since 1970-01-01 to a civil date (Howard Hinnant's `civil_from_days`).
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

/// What `submit` is given.
#[derive(Clone, Debug)]
pub struct Submit {
    pub worktree: PathBuf,
    pub priority: Priority,
    pub label: String,
    pub timeout: Option<u64>,
    pub env: BTreeMap<String, String>,
    pub command: Vec<String>,
}

/// The submitter's environment a job keeps: the paths and locale a build needs, and cargo's, rustup's and rustc's
/// settings, but not what `cargo run` sets for the submitting process itself, nor the target directory and the job
/// counts, which the job gets from `--env` and the runner.
pub fn snapshot_env(vars: impl Iterator<Item = (String, String)>) -> BTreeMap<String, String> {
    const KEEP: [&str; 8] = [
        "PATH", "HOME", "USER", "LOGNAME", "SHELL", "TMPDIR", "TERM", "LANG",
    ];
    const DROP: [&str; 12] = [
        "CARGO",
        "CARGO_MANIFEST_DIR",
        "CARGO_MANIFEST_PATH",
        "CARGO_CRATE_NAME",
        "CARGO_BIN_NAME",
        "CARGO_PRIMARY_PACKAGE",
        "CARGO_RUSTC_CURRENT_DIR",
        "CARGO_MAKEFLAGS",
        "CARGO_TARGET_DIR",
        "CARGO_TARGET_TMPDIR",
        "CARGO_BUILD_JOBS",
        "RUST_TEST_THREADS",
    ];
    vars.filter(|(key, _)| {
        let kept = KEEP.contains(&key.as_str())
            || ["LC_", "CARGO_", "RUSTUP_", "RUST", "SCCACHE_"]
                .iter()
                .any(|prefix| key.starts_with(prefix));
        kept && !DROP.contains(&key.as_str())
            && key != "RUSTUP_TOOLCHAIN"
            && !key.starts_with("CARGO_PKG_")
            && !key.starts_with("CARGO_BIN_EXE_")
    })
    .collect()
}

/// Whether a job that has run for `ran`, on the runner's monotonic clock, is past its timeout of `timeout_secs`.
pub fn timed_out(ran: Duration, timeout_secs: u64) -> bool {
    ran >= Duration::from_secs(timeout_secs)
}

/// A job the runner started: its child, when it started on the runner's monotonic clock, and how it is being ended, if
/// it is.
struct Running {
    child: Child,
    /// When the runner started it. Its timeout is measured from here, not from the wall clock's [`Job::started`], so a
    /// clock step or a sleep and wake can't time it out; a job a dead runner left running is started again, with a
    /// fresh timeout, or marked lost ([`Runner::recover`]).
    started: Instant,
    ending: Option<(State, Instant)>,
    killed: bool,
    /// How and when its process ended, once reaped: kept until the job's file says so, so a failed save is tried again
    /// with the true end, and its group, whose id may now be reused, is never signalled again.
    reaped: Option<(ExitStatus, u64)>,
}

/// The runner's state between passes.
struct Runner {
    queue: Queue,
    me: i32,
    children: HashMap<u64, Running>,
    /// The violations logged, by pid and command, so each is logged once.
    logged: HashSet<(i32, String)>,
    /// The jobs started (id, pid) whose leader's start time is not yet in their file: read with `ps` after the pass
    /// that started them, outside the state lock ([`Runner::record_starts`]).
    unrecorded: Vec<(u64, i32)>,
}

impl Runner {
    /// Ends the group of `left`, a job a dead runner left running, outside the state lock (it reads `ps` and waits up
    /// to `grace`); then, the lock held, queues it again, or marks it lost or cancelled, unless it has changed hands
    /// meanwhile. Returns false if another runner has taken the lock over, so this one exits.
    fn recover(&mut self, left: &Job, config: &Config) -> Result<bool, String> {
        let ended = end_stale_group(left, Duration::from_secs(config.grace));
        let _held = self.queue.hold(config)?;
        if self
            .queue
            .runner_lock()
            .is_none_or(|lock| lock.pid != self.me)
        {
            log(&format!(
                "runner {}: another runner holds the lock; exiting",
                self.me
            ));
            return Ok(false);
        }
        self.queue.write_lock(self.me, config)?;
        let mut job = self.queue.load(left.id)?;
        if job.state != State::Running || job.runner_pid != left.runner_pid {
            return Ok(true);
        }
        let from = job
            .runner_pid
            .map_or_else(|| "?".to_owned(), |pid| pid.to_string());
        job.pgid = None;
        job.leader_start = None;
        if job.cancel_requested {
            job.state = State::Cancelled;
            job.finished = Some(now());
            job.note = Some(format!(
                "cancelled; runner {from} ended while it ran; {ended}"
            ));
        } else if job.attempts >= MAX_ATTEMPTS {
            job.state = State::Lost;
            job.finished = Some(now());
            job.note = Some(format!(
                "lost: runner {from} ended while it ran, {} times; {ended}",
                job.attempts
            ));
        } else {
            job.state = State::Queued;
            job.note = Some(format!(
                "queued again: runner {from} ended while it ran; {ended}"
            ));
        }
        log(&format!(
            "runner {}: job {}: {}",
            self.me,
            job.id,
            job.note.as_deref().unwrap_or("")
        ));
        self.queue.save(&job)?;
        Ok(true)
    }

    /// Records the start time of each job started since the last call, read with `ps` outside the state lock, then
    /// written, the lock held, into each job still running under that leader. A lock it can't take is tried again on
    /// the next call.
    fn record_starts(&mut self, config: &Config) {
        if self.unrecorded.is_empty() {
            return;
        }
        let read: Vec<(u64, i32, Option<String>)> = self
            .unrecorded
            .iter()
            .map(|&(id, pid)| (id, pid, start_time(pid)))
            .collect();
        let _held = match self.queue.try_hold(Duration::from_secs(config.lock_wait)) {
            Ok(Ok(held)) => held,
            Ok(Err(message)) | Err(message) => {
                log(&format!("runner {}: {message}; trying again", self.me));
                return;
            }
        };
        self.unrecorded.clear();
        for (id, pid, start) in read {
            match self.queue.load(id) {
                Ok(mut job) => {
                    if record_start(&mut job, pid, start) {
                        if let Err(message) = self.queue.save(&job) {
                            log(&format!("runner {}: {message}", self.me));
                        }
                    }
                }
                Err(message) => log(&format!("runner {}: {message}", self.me)),
            }
        }
    }

    /// One pass, the state lock held: reaps ended jobs, ends timed-out and cancelled ones, starts queued ones up to the
    /// concurrency. Returns whether anything is queued or running. A failed save of one running job doesn't keep the
    /// others from being tended, but the pass then starts nothing and fails.
    fn pass(&mut self, config: &Config) -> Result<bool, String> {
        let mut jobs = self.queue.jobs()?;
        let at = now();
        for job in jobs.iter().filter(|job| archivable(job, at)) {
            let (from, to) = (self.queue.job_path(job.id), self.queue.archive_path(job.id));
            fs::rename(&from, &to).map_err(|e| format!("job {}: cannot archive: {e}", job.id))?;
        }
        jobs.retain(|job| !archivable(job, at));
        let grace = Duration::from_secs(config.grace);
        let mut failed = false;
        for job in jobs.iter_mut().filter(|job| job.state == State::Running) {
            let Some(running) = self.children.get_mut(&job.id) else {
                continue;
            };
            let pgid = job.pgid.unwrap_or(0);
            // The leader is seen to have exited before it is reaped: while it is a zombie its pid, and so its group's id,
            // can't be reused, so the kill below reaches only what the job left of its group.
            if running.reaped.is_none() && leader_exited(running.child.id()) {
                // What the job left of its group goes with it, however it ended, so nothing it started outlives it.
                signal_group(pgid, rustix::process::Signal::KILL);
                running.reaped = running
                    .child
                    .try_wait()
                    .map_err(|e| format!("job {}: cannot wait: {e}", job.id))?
                    .map(|status| (status, now()));
            }
            if let Some((status, finished)) = running.reaped {
                job.exit = Some(
                    status
                        .code()
                        .unwrap_or_else(|| 128 + status.signal().unwrap_or(0)),
                );
                job.state = running.ending.map_or(State::Exited, |(state, _)| state);
                job.finished = Some(finished);
                // The child stays until the file records the end, so a failed save is tried again on the next pass
                // and no job is left marked running with nothing to finish it.
                if let Err(message) = self.queue.save(job) {
                    log(&format!("runner {}: {message}", self.me));
                    failed = true;
                    continue;
                }
                self.children.remove(&job.id);
                log(&format!(
                    "runner {}: job {}: {}",
                    self.me,
                    job.id,
                    end_text(job)
                ));
                continue;
            }
            if running.ending.is_none() {
                let ending = if job.cancel_requested {
                    Some((State::Cancelled, "cancelled".to_owned()))
                } else if timed_out(running.started.elapsed(), job.timeout_secs) {
                    Some((
                        State::TimedOut,
                        format!("timed out after {} s", job.timeout_secs),
                    ))
                } else {
                    None
                };
                if let Some((state, note)) = ending {
                    signal_group(pgid, rustix::process::Signal::TERM);
                    running.ending = Some((state, Instant::now()));
                    job.note = Some(note);
                    if let Err(message) = self.queue.save(job) {
                        log(&format!("runner {}: {message}", self.me));
                        failed = true;
                    }
                }
            } else if let Some((_, since)) = running.ending {
                if !running.killed && since.elapsed() >= grace {
                    signal_group(pgid, rustix::process::Signal::KILL);
                    running.killed = true;
                }
            }
        }
        if failed {
            return Err(format!(
                "runner {}: a save failed; starting nothing this pass",
                self.me
            ));
        }
        // At most `concurrency` starts a pass, whatever the choice of job does: a loop on the slots alone would spin, or
        // start copies without end, on a job whose start doesn't fill one.
        for _ in 0..config.concurrency {
            if self.children.len() >= config.concurrency {
                break;
            }
            let Some(next) = next_job(&jobs).map(|job| job.id) else {
                break;
            };
            let Some(job) = jobs.iter_mut().find(|job| job.id == next) else {
                break;
            };
            self.start(job, config)?;
        }
        Ok(jobs
            .iter()
            .any(|job| matches!(job.state, State::Queued | State::Running)))
    }

    /// Starts `job` in a process group of its own, its output to its log. A job whose log can't be opened, or whose
    /// command can't be started, ends as not started. If the file can't record a start, the start is undone: its group
    /// is killed and reaped and the error returned, so nothing runs that the file doesn't record; otherwise the job,
    /// still queued, would start again on every pass, each copy with no timeout or cancel.
    ///
    /// Two limits hold whatever the file says. A job this runner already tracks is not started again, which would drop
    /// the running copy's child and so its timeout and cancel. And no job starts more than [`MAX_ATTEMPTS`] times: one
    /// queued with that many starts behind it ends as lost.
    fn start(&mut self, job: &mut Job, config: &Config) -> Result<(), String> {
        if self.children.contains_key(&job.id) {
            log(&format!(
                "runner {}: job {}: already running here; not started again",
                self.me, job.id
            ));
            return Ok(());
        }
        if job.attempts >= MAX_ATTEMPTS {
            job.state = State::Lost;
            job.finished = Some(now());
            job.note = Some(format!("lost: queued after {} starts", job.attempts));
            return self.queue.save(job);
        }
        job.attempts += 1;
        job.started = Some(now());
        job.runner_pid = Some(self.me);
        let mut child = match self.spawn(job, config) {
            Ok(child) => child,
            Err(note) => {
                job.state = State::NotStarted;
                job.finished = Some(now());
                log(&format!("runner {}: job {}: {note}", self.me, job.id));
                job.note = Some(note);
                return self.queue.save(job);
            }
        };
        let started = Instant::now();
        let pid = child.id() as i32;
        log(&format!(
            "runner {}: job {}: started, pid {pid}",
            self.me, job.id
        ));
        job.state = State::Running;
        job.pgid = Some(pid);
        // Read after the pass, outside the state lock ([`Runner::record_starts`]). Should this runner die first, the
        // next one tells the leader by its start against `started` ([`leader_is_jobs`]).
        job.leader_start = None;
        job.note = None;
        if let Err(message) = self.queue.save(job) {
            // The leader is not yet reaped, so its pid, and so its group's id, can't have been reused. SIGKILL can't be
            // ignored, so the wait is brief.
            signal_group(pid, rustix::process::Signal::KILL);
            let reaped = child.wait();
            log(&format!(
                "runner {}: job {}: start undone, group {pid} killed ({reaped:?}): its file can't be saved",
                self.me, job.id
            ));
            return Err(message);
        }
        self.unrecorded.push((job.id, pid));
        self.children.insert(
            job.id,
            Running {
                child,
                started,
                ending: None,
                killed: false,
                reaped: None,
            },
        );
        Ok(())
    }

    /// Opens `job`'s log, writes its header, and spawns its command.
    fn spawn(&self, job: &Job, config: &Config) -> Result<Child, String> {
        let path = self.queue.log_path(job.id);
        let cannot = |e: std::io::Error| format!("cannot open its log {}: {e}", path.display());
        let mut out = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(cannot)?;
        writeln!(
            out,
            "== job {} ({}, {}), start {} of at most {MAX_ATTEMPTS}, by runner {}, in {}\n== {}",
            job.id,
            job.label,
            job.priority.name(),
            job.attempts,
            self.me,
            job.worktree.display(),
            job.command.join(" ")
        )
        .map_err(cannot)?;
        let err = out.try_clone().map_err(cannot)?;
        // The command is looked up on the job's PATH, the submitter's unless its `--env PATH=…` gave one, not the
        // runner's: the environment is cleared and replaced before the lookup.
        Command::new(&job.command[0])
            .args(&job.command[1..])
            .current_dir(&job.worktree)
            .env_clear()
            .envs(job.env_for(&config.job_defaults()))
            .stdin(Stdio::null())
            .stdout(out)
            .stderr(err)
            .process_group(0)
            .spawn()
            .map_err(|e| format!("cannot start `{}`: {e}", job.command[0]))
    }

    /// Reads `ps` and logs each heavy process no queue job started that isn't logged yet.
    fn sample_violations(&mut self) -> Result<(), String> {
        let procs = processes()?;
        let jobs = self.queue.jobs()?;
        let groups: BTreeSet<i32> = jobs
            .iter()
            .filter(|job| job.state == State::Running)
            .filter_map(|job| job.pgid)
            .collect();
        let mut lines = String::new();
        for proc in violations(&procs, &groups, self.me) {
            if !self.logged.insert((proc.pid, proc.args.clone())) {
                continue;
            }
            let cwd = cwd_of(proc.pid).unwrap_or_else(|| "?".to_owned());
            lines.push_str(&format!(
                "{} pid={} ppid={} cwd={} command={}\n",
                utc(now()),
                proc.pid,
                proc.ppid,
                cwd,
                proc.args
            ));
        }
        if lines.is_empty() {
            return Ok(());
        }
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.queue.violations_path())
            .and_then(|mut file| file.write_all(lines.as_bytes()))
            .map_err(|e| format!("cannot write the violations log: {e}"))
    }
}

/// Sends `signal` to process group `pgid`, if [`may_signal_group`] allows it, and logs it if not; a group already gone
/// is not an error.
fn signal_group(pgid: i32, signal: rustix::process::Signal) {
    let own_pid = std::process::id() as i32;
    let own_group = rustix::process::getpgrp().as_raw_pid();
    if !may_signal_group(pgid, own_pid, own_group) {
        log(&format!(
            "runner {own_pid}: refused to signal process group {pgid} ({signal:?})"
        ));
        return;
    }
    if let Some(group) = rustix::process::Pid::from_raw(pgid) {
        let _ = rustix::process::kill_process_group(group, signal);
    }
}

/// Whether child `pid` has exited, without reaping it (`waitid` with `WNOWAIT`). An error (no such child) counts as
/// exited, so the caller's `try_wait` reports it.
fn leader_exited(pid: u32) -> bool {
    use rustix::process::{waitid, Pid, WaitId, WaitIdOptions};
    let Some(pid) = i32::try_from(pid).ok().and_then(Pid::from_raw) else {
        return true;
    };
    let options = WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT;
    !matches!(waitid(WaitId::Pid(pid), options), Ok(None))
}

/// When process `pid` started, as `ps -o lstart=` gives it in the C locale and UTC: `ps` prints it by the caller's `TZ`
/// and locale, and the runner that compares it may not have the environment of the runner that recorded it.
fn start_time(pid: i32) -> Option<String> {
    let out = Command::new("ps")
        .args(["-o", "lstart=", "-p", &pid.to_string()])
        .env("LC_ALL", "C")
        .env("TZ", "UTC")
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    (!text.is_empty()).then_some(text)
}

/// The members of process group `pgid`.
fn group_members(pgid: i32) -> Vec<i32> {
    processes()
        .unwrap_or_default()
        .into_iter()
        .filter(|proc| proc.pgid == pgid)
        .map(|proc| proc.pid)
        .collect()
}

/// How long process `pid` has run, in seconds, from `ps -o etime=`, whose form doesn't depend on the locale or zone.
fn running_for(pid: i32) -> Option<u64> {
    let out = Command::new("ps")
        .args(["-o", "etime=", "-p", &pid.to_string()])
        .env("LC_ALL", "C")
        .output()
        .ok()?;
    parse_etime(&String::from_utf8_lossy(&out.stdout))
}

/// `ps -o etime=`, `[[dd-]hh:]mm:ss`, in seconds; anything else is `None`.
pub fn parse_etime(text: &str) -> Option<u64> {
    let text = text.trim();
    let (days, clock) = match text.split_once('-') {
        Some((days, clock)) => (days.parse::<u64>().ok()?, clock),
        None => (0, text),
    };
    let parts: Vec<&str> = clock.split(':').collect();
    if !(2..=3).contains(&parts.len()) {
        return None;
    }
    let mut secs = 0u64;
    for part in parts {
        if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        secs = secs * 60 + part.parse::<u64>().ok()?;
    }
    Some(days * 86_400 + secs)
}

/// Whether the live leader of `job`'s group is still the job's: when the job's file holds its start time, if `start`
/// (`ps -o lstart=` now) is that time; when it doesn't, a runner having died before recording it, if `started_at` (the
/// leader's start in Unix seconds now) is within [`START_SLACK`] of [`Job::started`]. A missing start time is unknown,
/// not different: read as different, it left a job running untracked beside its second start.
pub fn leader_is_jobs(job: &Job, start: Option<&str>, started_at: Option<u64>) -> bool {
    match (&job.leader_start, job.started, started_at) {
        (Some(recorded), _, _) => start == Some(recorded.as_str()),
        (None, Some(started), Some(at)) => started.abs_diff(at) <= START_SLACK,
        (None, _, _) => false,
    }
}

/// Ends what is left of a job's process group after its runner died: SIGTERM, then SIGKILL after `grace`. A group
/// whose leader is alive but is not the job's ([`leader_is_jobs`]) is another process's, and is left alone.
fn end_stale_group(job: &Job, grace: Duration) -> String {
    let Some(pgid) = job.pgid else {
        return "it had no process group".to_owned();
    };
    let members = group_members(pgid);
    if members.is_empty() {
        return format!("its group {pgid} was already gone");
    }
    if members.contains(&pgid) {
        let (start, started_at) = match job.leader_start {
            Some(_) => (start_time(pgid), None),
            None => (
                None,
                running_for(pgid).and_then(|ran| now().checked_sub(ran)),
            ),
        };
        if !leader_is_jobs(job, start.as_deref(), started_at) {
            return format!("group {pgid} is another process's now, left alone");
        }
    }
    signal_group(pgid, rustix::process::Signal::TERM);
    let ends = Instant::now() + grace;
    while Instant::now() < ends && !group_members(pgid).is_empty() {
        std::thread::sleep(Duration::from_millis(100));
    }
    if group_members(pgid).is_empty() {
        return format!("its group {pgid} ended on SIGTERM");
    }
    signal_group(pgid, rustix::process::Signal::KILL);
    format!("its group {pgid} was killed")
}

/// One process, as `ps` lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Proc {
    pub pid: i32,
    pub ppid: i32,
    pub pgid: i32,
    /// Its executable's path (`ps -o comm=`).
    pub exe: String,
    /// Its command line (`ps -o args=`).
    pub args: String,
}

/// Every process, from two readings of `ps`, joined by pid: one for the ids and executable, one for the command line
/// (each may hold spaces, so each is the last column of its own reading).
pub fn processes() -> Result<Vec<Proc>, String> {
    let read = |columns: &str| -> Result<String, String> {
        let out = Command::new("ps")
            .args(["-axo", columns])
            .output()
            .map_err(|e| format!("cannot run ps: {e}"))?;
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    };
    Ok(parse_ps(
        &read("pid=,ppid=,pgid=,comm=")?,
        &read("pid=,args=")?,
    ))
}

/// Joins `ps -axo pid=,ppid=,pgid=,comm=` and `ps -axo pid=,args=` by pid.
pub fn parse_ps(ids: &str, args: &str) -> Vec<Proc> {
    let args: HashMap<i32, &str> = args
        .lines()
        .filter_map(|line| {
            let line = line.trim_start();
            let (pid, rest) = line.split_once(' ').unwrap_or((line, ""));
            Some((pid.parse().ok()?, rest.trim_start()))
        })
        .collect();
    ids.lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let pid: i32 = fields.next()?.parse().ok()?;
            let ppid = fields.next()?.parse().ok()?;
            let pgid = fields.next()?.parse().ok()?;
            let exe = fields.collect::<Vec<_>>().join(" ");
            Some(Proc {
                pid,
                ppid,
                pgid,
                args: args
                    .get(&pid)
                    .map_or_else(|| exe.clone(), |a| (*a).to_owned()),
                exe,
            })
        })
        .collect()
}

/// The last component of a path.
fn base(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// cargo's subcommands that compile (or run what compiles).
const CARGO_HEAVY: [&str; 18] = [
    "build", "b", "check", "c", "test", "t", "clippy", "doc", "d", "bench", "run", "r", "rustc",
    "rustdoc", "mutants", "nextest", "install", "fix",
];

/// xtask's runners that compile or run GPU work.
const XTASK_HEAVY: [&str; 8] = [
    "ci",
    "controls",
    "gate",
    "bench",
    "build-kernel",
    "golden",
    "screenshot",
    "deps",
];

/// Whether `proc` is a heavy process (R-658): cargo running a subcommand that compiles (`cargo xtask` but `job`
/// counts, since `cargo xtask` builds xtask), rustc, rustdoc and clippy-driver (but for the version queries cargo makes
/// of rustc), cargo-clippy, cargo-mutants, cargo-nextest, an xtask runner that compiles or runs GPU work, or a test
/// binary (`…/deps/<name>-<16 hex digits>`).
pub fn heavy(proc: &Proc) -> bool {
    let program = base(&proc.exe);
    // The arguments after the executable: the command line past its first word, or past the executable's path.
    let rest = proc
        .args
        .strip_prefix(proc.exe.as_str())
        .unwrap_or_else(|| proc.args.split_once(' ').map_or("", |(_, rest)| rest));
    let mut words = rest.split_whitespace();
    match program {
        "rustc" | "rustdoc" | "clippy-driver" => !rest
            .split_whitespace()
            .any(|w| matches!(w, "-vV" | "-V" | "--version" | "-") || w.starts_with("--print")),
        "cargo-clippy" | "cargo-mutants" | "cargo-nextest" => true,
        "cargo" => {
            // The subcommand: the first word that is no flag and no `+toolchain`, past the values of `-Z`,
            // `--config` and `-C`.
            let mut subcommand = None;
            while let Some(word) = words.next() {
                if matches!(word, "-Z" | "--config" | "-C" | "--color") {
                    words.next();
                } else if !word.starts_with('-') && !word.starts_with('+') {
                    subcommand = Some(word);
                    break;
                }
            }
            match subcommand {
                Some("xtask") => words.next() != Some("job"),
                Some(sub) => CARGO_HEAVY.contains(&sub),
                None => false,
            }
        }
        "xtask" => words.next().is_some_and(|sub| XTASK_HEAVY.contains(&sub)),
        name => {
            proc.exe.contains("/deps/")
                && name.rsplit_once('-').is_some_and(|(_, hash)| {
                    hash.len() == 16 && hash.bytes().all(|b| b.is_ascii_hexdigit())
                })
        }
    }
}

/// The heavy processes no queue job started (R-658): each heavy process none of whose ancestors, itself included, is
/// in one of `groups` (the running jobs' process groups), is the runner `runner`, or is sccache's server (which runs
/// the compilers it is handed). Of a chain of such processes only the topmost is given.
pub fn violations<'a>(procs: &'a [Proc], groups: &BTreeSet<i32>, runner: i32) -> Vec<&'a Proc> {
    let by_pid: HashMap<i32, &Proc> = procs.iter().map(|proc| (proc.pid, proc)).collect();
    let queued = |proc: &Proc| {
        let mut at = Some(proc);
        for _ in 0..256 {
            let Some(p) = at else {
                return false;
            };
            if groups.contains(&p.pgid)
                || p.pid == runner
                || p.ppid == runner
                || base(&p.exe) == "sccache"
            {
                return true;
            }
            if p.ppid == p.pid || p.ppid <= 1 {
                return false;
            }
            at = by_pid.get(&p.ppid).copied();
        }
        false
    };
    let outside = |proc: &Proc| heavy(proc) && !queued(proc);
    procs
        .iter()
        .filter(|proc| outside(proc))
        .filter(|proc| !by_pid.get(&proc.ppid).is_some_and(|parent| outside(parent)))
        .collect()
}

/// Process `pid`'s working directory: from `/proc` on Linux, from `lsof` elsewhere.
fn cwd_of(pid: i32) -> Option<String> {
    if let Ok(path) = fs::read_link(format!("/proc/{pid}/cwd")) {
        return Some(path.display().to_string());
    }
    let out = Command::new("lsof")
        .args(["-a", "-p", &pid.to_string(), "-d", "cwd", "-Fn"])
        .output()
        .ok()?;
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find_map(|line| line.strip_prefix('n').map(str::to_owned))
}

/// `cargo xtask job <args>`.
pub fn cli(args: &[&str]) -> ExitCode {
    let queue = Queue::from_env();
    let result = match args {
        ["submit", rest @ ..] => parse_submit(rest)
            .and_then(|request| queue.submit(request))
            .map(|id| {
                println!("{id}");
                0
            }),
        ["wait", id] => parse_id(id).and_then(|id| queue.wait(id)).map(|job| {
            println!(
                "job {} ({}): {}{}",
                job.id,
                job.label,
                end_text(&job),
                job.note
                    .as_deref()
                    .map(|n| format!(" ({n})"))
                    .unwrap_or_default()
            );
            println!("log: {}", queue.log_path(job.id).display());
            job.status_code()
        }),
        ["status"] => queue.status().map(|text| {
            print!("{text}");
            0
        }),
        ["cancel", id] => parse_id(id).and_then(|id| queue.cancel(id)).map(|text| {
            println!("{text}");
            0
        }),
        ["install"] => std::env::current_exe()
            .map_err(|e| format!("job install: {e}"))
            .and_then(|exe| queue.install(&exe))
            .map(|dest| {
                println!("installed {}", dest.display());
                0
            }),
        ["runner"] => queue.run_runner().map(|()| 0),
        ["runner", root] => Queue::at(root).run_runner().map(|()| 0),
        _ => Err(format!(
            "job: unrecognised arguments: {}\n\nUsage:\n{USAGE}",
            args.join(" ")
        )),
    };
    match result {
        Ok(code) => ExitCode::from(code.clamp(0, 255) as u8),
        Err(message) => {
            eprintln!("xtask {message}");
            ExitCode::FAILURE
        }
    }
}

fn parse_id(id: &str) -> Result<u64, String> {
    id.parse()
        .map_err(|_| format!("job: a job id is a number, not `{id}`"))
}

/// Reads `submit`'s arguments.
pub fn parse_submit(args: &[&str]) -> Result<Submit, String> {
    let split = args
        .iter()
        .position(|arg| *arg == "--")
        .ok_or("job submit: the command follows --")?;
    let (flags, command) = (&args[..split], &args[split + 1..]);
    let mut flags = flags.iter();
    let worktree = flags
        .next()
        .filter(|w| !w.starts_with('-'))
        .ok_or("job submit: the worktree comes first")?;
    let (mut priority, mut label, mut timeout, mut env) = (None, None, None, BTreeMap::new());
    while let Some(flag) = flags.next() {
        let value = flags
            .next()
            .ok_or_else(|| format!("job submit: {flag} takes a value"))?;
        match *flag {
            "--priority" => priority = Some(Priority::parse(value)?),
            "--label" => label = Some((*value).to_owned()),
            "--timeout" => {
                let secs: u64 = value.parse().ok().filter(|s| *s > 0).ok_or_else(|| {
                    format!("job submit: --timeout takes whole seconds, not `{value}`")
                })?;
                timeout = Some(secs);
            }
            "--env" => {
                let (key, val) = value
                    .split_once('=')
                    .filter(|(key, _)| !key.is_empty())
                    .ok_or_else(|| format!("job submit: --env takes KEY=VALUE, not `{value}`"))?;
                env.insert(key.to_owned(), val.to_owned());
            }
            other => return Err(format!("job submit: unrecognised flag `{other}`")),
        }
    }
    Ok(Submit {
        worktree: PathBuf::from(worktree),
        priority: priority.ok_or("job submit: --priority is required")?,
        label: label.ok_or("job submit: --label is required")?,
        timeout,
        env,
        command: command.iter().map(|arg| (*arg).to_owned()).collect(),
    })
}
