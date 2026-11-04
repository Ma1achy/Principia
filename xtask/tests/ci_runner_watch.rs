//! The sampler's `watch` (`.github/scripts/disk_sampler.sh`, applied per R-369), which names a stall of the whole
//! runner in `ci-workspace`: beside the step it logs the load, the kernel's pressure figures, Dirty, Writeback and the
//! device-mapper's state to a tmpfs, with builtins, so neither a stuck disk nor a stuck command stops it; it can't
//! change the step's status; and `ci-workspace` and `ci-release` upload what it wrote, with the kernel's log on a
//! failure.
//!
//! Every run here is in a scratch folder, with a fake /proc, HOME, CARGO_HOME and tmpfs there, and a fake `sudo` first
//! on the PATH: no test touches the real /dev/shm or asks for the real sudo. Each check has its negative control
//! (R-176).
#![cfg(unix)]

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

use scratch::Scratch;
use validation::negative_control;
use validation::spawn::Spawn;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the workspace root")
        .to_path_buf()
}

fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path)).expect("readable")
}

fn sampler() -> String {
    read(".github/scripts/disk_sampler.sh")
}

fn ci_yml() -> String {
    read(".github/workflows/ci.yml")
}

/// `from` in `text`, replaced: an edit that no longer applies must not quietly leave a sampler the check passes on.
fn edit(text: &str, from: &str, to: &str) -> String {
    assert!(text.contains(from), "the script has no {from:?}");
    text.replace(from, to)
}

fn executable(path: &Path, body: &str) {
    std::fs::write(path, body).expect("script written");
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).expect("mode");
}

/// Where a run keeps what a real runner has: a scratch /proc, tmpfs, temp folder and PATH.
struct Bed {
    dir: Scratch,
}

const SUDO_ANSWERS: &str =
    "#!/bin/sh\necho \"$*\" >> \"$BED/sudo-calls\"\necho 'cow-root: 0 100 snapshot 7/100 16'\n";

/// Only ever runs if the sampler's `timeout` is put back outside `sudo`: it hands on to the fake `sudo`, so that a Mac,
/// which has no `timeout`, reads that the same way as a runner. Under `sudo`, the fake `sudo` runs nothing it is given, so
/// the real `timeout`, the limit and root are in no test here: CI is the first real run.
const FAKE_TIMEOUT: &str = "#!/bin/sh\nshift\nexec \"$@\"\n";

impl Bed {
    fn new(tag: &str) -> Bed {
        let dir = Scratch::new(tag);
        for folder in ["proc/pressure", "shm", "temp", "bin", "home/cargo"] {
            std::fs::create_dir_all(dir.join(folder)).expect("scratch folder");
        }
        for (file, text) in [
            ("proc/loadavg", "1.25 2.50 3.75 2/345 6789\n"),
            ("proc/pressure/cpu", "some avg10=1.00 avg60=0 avg300=0 total=1\n"),
            (
                "proc/pressure/memory",
                "some avg10=2.00 avg60=0 avg300=0 total=2\nfull avg10=3.00 avg60=0 avg300=0 total=3\n",
            ),
            ("proc/pressure/io", "some avg10=4.00 avg60=0 avg300=0 total=4\n"),
            (
                "proc/meminfo",
                "MemFree: 11 kB\nDirty: 22 kB\nWriteback: 33 kB\nWritebackTmp: 44 kB\n",
            ),
        ] {
            std::fs::write(dir.join(file), text).expect("proc file");
        }
        let bed = Bed { dir };
        bed.fake("sudo", SUDO_ANSWERS);
        bed.fake("timeout", FAKE_TIMEOUT);
        bed
    }

    fn fake(&self, name: &str, body: &str) {
        executable(&self.dir.join("bin").join(name), body);
    }

    fn command(&self, path: String) -> Command {
        let mut command = Command::new("bash");
        command
            .current_dir(&*self.dir)
            .env("BED", &*self.dir)
            .env("HOME", self.dir.join("home"))
            .env("CARGO_HOME", self.dir.join("home/cargo"))
            .env("PATH", path)
            .env("RUNNER_TEMP", self.dir.join("temp"))
            .env("RUNNER_DISK_SAMPLE_SECONDS", "2")
            .env("RUNNER_PRESSURE_SAMPLE_SECONDS", "1")
            .env_remove("RUNNER_DISK_TMPFS");
        command
    }

    fn path(&self) -> String {
        format!(
            "{}:{}",
            self.dir.join("bin").display(),
            std::env::var("PATH").unwrap_or_default()
        )
    }

    /// `script` run as the sampler on the step `step`, its /proc this bed's, with `tmpfs` as RUNNER_DISK_TMPFS.
    fn run(
        &self,
        script: &str,
        step: &str,
        tmpfs: Option<&Path>,
        path: String,
        limit: u64,
    ) -> Output {
        let script = edit(
            script,
            "proc=/proc\n",
            &format!("proc='{}'\n", self.dir.join("proc").display()),
        );
        std::fs::write(self.dir.join("sampler.sh"), script).expect("sampler written");
        std::fs::write(self.dir.join("step.sh"), step).expect("step written");
        let mut command = self.command(path);
        command.args(["sampler.sh", "step.sh"]);
        if let Some(tmpfs) = tmpfs {
            command.env("RUNNER_DISK_TMPFS", tmpfs);
        }
        command
            .output_within(Duration::from_secs(limit))
            .expect("the sampler ran")
    }

    fn shm(&self) -> PathBuf {
        self.dir.join("shm")
    }

    fn watch_log(&self, folder: &str) -> Option<String> {
        std::fs::read_to_string(self.dir.join(folder).join("runner-pressure.log")).ok()
    }
}

/// A long way past any run here: this bound is the one on a run that went wrong.
const LIMIT: u64 = 60;

/// Ends once `watch` has written two samples, wherever it writes.
const STEP_TO_TWO_SAMPLES: &str = "for _ in {1..300}; do\n  \
     [ \"$(cat \"$BED\"/*/runner-pressure.log 2> /dev/null | grep -c '^== watch')\" -ge 2 ] && break\n  \
     sleep 0.1\ndone\nexit 0\n";

fn samples(log: &str) -> usize {
    log.lines().filter(|l| l.starts_with("== watch")).count()
}

/// Everything asked for reaches the tmpfs, and nothing beside it does.
fn check_reads(script: &str) {
    let bed = Bed::new("ci_watch_reads");
    bed.run(
        script,
        STEP_TO_TWO_SAMPLES,
        Some(&bed.shm()),
        bed.path(),
        LIMIT,
    );
    let log = bed.watch_log("shm").expect("no `watch` log on the tmpfs");
    for line in [
        "loadavg 1.25 2.50 3.75 2/345 6789",
        "cpu some avg10=1.00",
        "memory some avg10=2.00",
        "memory full avg10=3.00",
        "io some avg10=4.00",
        "Dirty: 22 kB",
        "Writeback: 33 kB",
    ] {
        assert!(log.contains(line), "no `{line}` in:\n{log}");
    }
    for line in ["WritebackTmp", "MemFree"] {
        assert!(!log.contains(line), "`{line}` in:\n{log}");
    }
    assert!(
        bed.watch_log("temp").is_none(),
        "`watch` wrote on the disk with a tmpfs to hand"
    );
}

#[test]
fn ci_watch_logs_the_load_to_the_tmpfs() {
    check_reads(&sampler());
}

negative_control!(
    ci_watch_logs_the_load_to_the_tmpfs,
    "`watch` writing on the disk it is there to watch",
    expected = "no `watch` log on the tmpfs",
    check_reads(&edit(&sampler(), "watch_dir=$tmpfs\n", "watch_dir=\n"))
);

mod no_memory {
    use super::*;
    negative_control!(
        ci_watch_logs_the_load_to_the_tmpfs,
        "no memory pressure, the hypothesis the log is there to test",
        expected = "no `memory some avg10=2.00`",
        check_reads(&edit(
            &sampler(),
            "for kind in cpu memory io",
            "for kind in cpu io"
        ))
    );
}

mod every_writeback {
    use super::*;
    negative_control!(
        ci_watch_logs_the_load_to_the_tmpfs,
        "a match on the prefix alone, which takes WritebackTmp too",
        expected = "`WritebackTmp` in:",
        check_reads(&edit(&sampler(), "Writeback:*)", "Writeback*)"))
    );
}

/// A reading that needs a command fails on a PATH of only what `watch`'s start needs: every reading still arrives.
fn check_builtins(script: &str) {
    let bed = Bed::new("ci_watch_builtins");
    let only = bed.dir.join("only");
    std::fs::create_dir_all(&only).expect("scratch folder");
    for tool in ["bash", "mkfifo", "rm", "awk"] {
        let which = Command::new("sh")
            .args(["-c", &format!("command -v {tool}")])
            .output_within(Duration::from_secs(10))
            .expect("sh ran");
        let found = String::from_utf8_lossy(&which.stdout);
        std::os::unix::fs::symlink(found.trim(), only.join(tool)).expect("linked");
    }
    executable(&only.join("nproc"), "#!/bin/sh\necho 4\n");
    // Six seconds, with no command on the PATH to wait with.
    let step =
        "n=0\nwhile ((n++ < 6)); do read -r -t 1 x < <(/bin/sleep 2) || true; done\nexit 0\n";
    bed.run(
        script,
        step,
        Some(&bed.shm()),
        only.display().to_string(),
        LIMIT,
    );
    let log = bed.watch_log("shm").expect("no `watch` log");
    let reads: Vec<_> = log
        .lines()
        .filter(|l| l.starts_with("loadavg 1.25"))
        .collect();
    assert!(
        reads.len() >= 2 && log.contains("Dirty: 22 kB") && log.contains("nproc 4"),
        "too little read with no commands on the PATH:\n{log}"
    );
}

#[test]
fn ci_watch_reads_with_builtins_alone() {
    check_builtins(&sampler());
}

negative_control!(
    ci_watch_reads_with_builtins_alone,
    "a reading through `cat`, which needs a fork and a binary off the disk",
    expected = "too little read with no commands on the PATH",
    check_builtins(&edit(
        &sampler(),
        "read -r line < \"$proc/loadavg\"",
        "line=$(cat \"$proc/loadavg\")"
    ))
);

/// The sampler's `df` stuck, as on a stalled disk: `watch` goes on, and the step's end ends both.
fn check_beside_a_stuck_sampler(script: &str) {
    let bed = Bed::new("ci_watch_stuck_df");
    let stuck = edit(
        script,
        "    avail=$(df -Pk / |",
        "    sleep 30\n    avail=$(df -Pk / |",
    );
    let output = bed.run(
        &stuck,
        STEP_TO_TWO_SAMPLES,
        Some(&bed.shm()),
        bed.path(),
        45,
    );
    assert_eq!(output.status.code(), Some(0), "the step's status was lost");
    let log = bed.watch_log("shm").unwrap_or_default();
    assert!(
        samples(&log) >= 2,
        "`watch` stopped with the sampler:\n{log}"
    );
}

#[test]
fn ci_watch_goes_on_beside_a_stuck_sampler() {
    check_beside_a_stuck_sampler(&sampler());
}

negative_control!(
    ci_watch_goes_on_beside_a_stuck_sampler,
    "the readings taken only in the sampler's own loop",
    expected = "`watch` stopped with the sampler",
    check_beside_a_stuck_sampler(&edit(
        &sampler(),
        "set -m\nwatch &\nwatch_pid=$!\nset +m\n",
        "watch_pid=none\n"
    ))
);

/// `dmsetup status` is asked for once a tmpfs is named, without a password prompt, under a `timeout` that runs as root,
/// and logged.
fn check_device(script: &str) {
    let bed = Bed::new("ci_watch_device");
    bed.run(
        script,
        STEP_TO_TWO_SAMPLES,
        Some(&bed.shm()),
        bed.path(),
        LIMIT,
    );
    let log = bed.watch_log("shm").expect("no `watch` log");
    let calls = std::fs::read_to_string(bed.dir.join("sudo-calls")).unwrap_or_default();
    assert_eq!(
        calls.lines().next(),
        Some("-n timeout 10 dmsetup status"),
        "sudo was asked: {calls}"
    );
    assert!(
        log.contains("snapshot 7/100"),
        "no device status in:\n{log}"
    );
}

#[test]
fn ci_watch_logs_the_device_status() {
    check_device(&sampler());
}

negative_control!(
    ci_watch_logs_the_device_status,
    "a sudo that could wait for a password",
    expected = "sudo was asked: timeout 10 dmsetup status",
    check_device(&edit(&sampler(), "sudo -n timeout", "sudo timeout"))
);

mod device_timeout_outside_sudo {
    use super::*;
    negative_control!(
        ci_watch_logs_the_device_status,
        "`timeout` back outside `sudo`, where this user can't signal a root `dmsetup`",
        expected = "sudo was asked: -n dmsetup status",
        check_device(&edit(
            &sampler(),
            "sudo -n timeout 10 dmsetup",
            "timeout 10 sudo -n dmsetup"
        ))
    );
}

mod device_timeout_dropped {
    use super::*;
    negative_control!(
        ci_watch_logs_the_device_status,
        "a device read with no limit",
        expected = "sudo was asked: -n dmsetup status",
        check_device(&edit(
            &sampler(),
            "sudo -n timeout 10 dmsetup",
            "sudo -n dmsetup"
        ))
    );
}

/// With no tmpfs named, as in the tests a step runs, nothing asks for sudo.
fn check_no_sudo_unasked(script: &str) {
    let bed = Bed::new("ci_watch_no_sudo");
    bed.run(script, STEP_TO_TWO_SAMPLES, None, bed.path(), LIMIT);
    assert!(
        !bed.dir.join("sudo-calls").exists(),
        "sudo was asked with no tmpfs named"
    );
    assert!(
        bed.watch_log("temp").is_some_and(|l| samples(&l) >= 2),
        "no `watch` log beside the sampler's"
    );
}

#[test]
fn ci_watch_leaves_sudo_alone_without_a_tmpfs() {
    check_no_sudo_unasked(&sampler());
}

negative_control!(
    ci_watch_leaves_sudo_alone_without_a_tmpfs,
    "the device status asked for on any run",
    expected = "sudo was asked with no tmpfs named",
    check_no_sudo_unasked(&edit(
        &sampler(),
        "[ -z \"$tmpfs\" ] || device_watch &",
        "device_watch &"
    ))
);

/// A device read that the step's KILL doesn't reach, as it doesn't reach a root process on a runner: the read is a job
/// of its own, in a group outside the one the KILL takes, and takes longer than any step here.
const STUCK_SUDO: &str = "#!/usr/bin/env bash\nset -m\n(sleep 12; touch \"$BED/sudo-finished\") &\nset +m\nwait \"$!\"\n";

/// A stuck device read holds up no reading, and one that outlives the step doesn't hold the step open by its streams.
fn check_stuck_device(script: &str) {
    let bed = Bed::new("ci_watch_stuck_device");
    bed.fake("sudo", STUCK_SUDO);
    bed.run(
        script,
        STEP_TO_TWO_SAMPLES,
        Some(&bed.shm()),
        bed.path(),
        45,
    );
    let finished = bed.dir.join("sudo-finished");
    assert!(!finished.exists(), "the device read held the step open");
    let log = bed.watch_log("shm").unwrap_or_default();
    assert!(
        samples(&log) >= 2,
        "a stuck `dmsetup` stopped `watch`:\n{log}"
    );
    // The stand-in leaves nothing running behind the test.
    let end = Instant::now() + Duration::from_secs(20);
    while !finished.exists() && Instant::now() < end {
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[test]
fn ci_watch_goes_on_beside_a_stuck_device_read() {
    check_stuck_device(&sampler());
}

negative_control!(
    ci_watch_goes_on_beside_a_stuck_device_read,
    "the device read in `watch`'s own loop",
    expected = "a stuck `dmsetup` stopped `watch`",
    check_stuck_device(&edit(
        &sampler(),
        "[ -z \"$tmpfs\" ] || device_watch &",
        "device_watch"
    ))
);

mod device_read_on_the_steps_streams {
    use super::*;
    negative_control!(
        ci_watch_goes_on_beside_a_stuck_device_read,
        "a device read that keeps the step's streams",
        expected = "the device read held the step open",
        check_stuck_device(&edit(
            &edit(&sampler(), "  exec > /dev/null 2>&1 < /dev/null\n", ""),
            "text=$(sudo -n timeout 10 dmsetup status 2>&1)",
            "text=; sudo -n timeout 10 dmsetup status"
        ))
    );
}

/// The step's end stops `device_watch`'s loop, which runs as this user: no read starts after the step.
fn check_device_stops(script: &str) {
    let bed = Bed::new("ci_watch_device_stops");
    bed.run(
        script,
        STEP_TO_TWO_SAMPLES,
        Some(&bed.shm()),
        bed.path(),
        LIMIT,
    );
    let calls = || std::fs::read_to_string(bed.dir.join("sudo-calls")).unwrap_or_default();
    let at_the_end = calls();
    std::thread::sleep(Duration::from_secs(5));
    assert_eq!(calls(), at_the_end, "a device read started after the step");
}

#[test]
fn ci_watch_starts_no_device_read_after_the_step() {
    check_device_stops(&sampler());
}

negative_control!(
    ci_watch_starts_no_device_read_after_the_step,
    "a KILL of `watch` alone, which leaves `device_watch` running",
    expected = "a device read started after the step",
    check_device_stops(&edit(
        &edit(
            &sampler(),
            "kill -KILL -- \"-$watch_pid\"",
            "kill -KILL \"$watch_pid\""
        ),
        "max_samples=480",
        "max_samples=4"
    ))
);

/// No sample is written after the step.
fn check_ends(script: &str) {
    let bed = Bed::new("ci_watch_ends");
    bed.run(script, STEP_TO_TWO_SAMPLES, None, bed.path(), LIMIT);
    let at_the_end = samples(&bed.watch_log("temp").expect("no `watch` log"));
    std::thread::sleep(Duration::from_secs(4));
    let later = samples(&bed.watch_log("temp").expect("no `watch` log"));
    assert_eq!(at_the_end, later, "`watch` went on after the step");
}

#[test]
fn ci_watch_ends_with_the_step() {
    check_ends(&sampler());
}

negative_control!(
    ci_watch_ends_with_the_step,
    "a `watch` left running, for six seconds here",
    expected = "`watch` went on after the step",
    check_ends(&edit(
        &edit(
            &sampler(),
            "kill -KILL -- \"-$watch_pid\" 2> /dev/null\nwait \"$watch_pid\" 2> /dev/null\n",
            ""
        ),
        "max_watch_samples=4320",
        "max_watch_samples=6"
    ))
);

/// Left running, `watch` holds none of the step's streams, which in Actions can hold the step open.
fn check_streams(script: &str) {
    let bed = Bed::new("ci_watch_streams");
    let left = edit(
        &edit(
            script,
            "kill -KILL -- \"-$watch_pid\" 2> /dev/null\nwait \"$watch_pid\" 2> /dev/null\n",
            "",
        ),
        "max_watch_samples=4320",
        "max_watch_samples=15",
    );
    bed.run(&left, "exit 0\n", None, bed.path(), 10);
}

#[test]
fn ci_watch_holds_none_of_the_steps_streams() {
    check_streams(&sampler());
}

negative_control!(
    ci_watch_holds_none_of_the_steps_streams,
    "`watch` on the step's streams",
    expected = "exited, but its output stayed open",
    check_streams(&edit(
        &sampler(),
        "  exec > /dev/null 2>&1 < /dev/null\n  [ -z",
        "  [ -z"
    ))
);

/// What can go wrong with the measurement leaves the step's status alone, and `watch` still writes where it can.
fn check_faults(script: &str) {
    let bed = Bed::new("ci_watch_faults");
    let not_a_folder = bed.dir.join("a_file");
    std::fs::write(&not_a_folder, "").expect("written");
    let locked = bed.dir.join("locked");
    std::fs::create_dir_all(&locked).expect("scratch folder");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o555)).expect("mode");
    let missing = bed.dir.join("missing");
    for tmpfs in [&not_a_folder, &locked, &missing] {
        for status in [0, 3] {
            let step = STEP_TO_TWO_SAMPLES.replace("exit 0", &format!("exit {status}"));
            let output = bed.run(script, &step, Some(tmpfs), bed.path(), LIMIT);
            assert_eq!(
                output.status.code(),
                Some(status),
                "the step's status, with {} as the tmpfs",
                tmpfs.display()
            );
        }
        let log = bed
            .watch_log("temp")
            .expect("no `watch` log beside the sampler's");
        assert!(samples(&log) >= 2, "no fallback from {}", tmpfs.display());
        std::fs::remove_file(bed.dir.join("temp/runner-pressure.log")).expect("removed");
    }
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).expect("mode");
    // No /proc, as on a kernel without pressure figures; `sudo` that fails.
    bed.fake("sudo", "#!/bin/sh\nexit 1\n");
    std::fs::remove_dir_all(bed.dir.join("proc")).expect("removed");
    let output = bed.run(script, "exit 3\n", Some(&bed.shm()), bed.path(), LIMIT);
    assert_eq!(
        output.status.code(),
        Some(3),
        "the step's status with no /proc"
    );
}

#[test]
fn ci_watch_faults_leave_the_step_status() {
    check_faults(&sampler());
}

negative_control!(
    ci_watch_faults_leave_the_step_status,
    "the step's status read from the end of `watch`",
    expected = "the step's status, with",
    check_faults(&edit(
        &sampler(),
        "wait \"$watch_pid\" 2> /dev/null\n",
        "wait \"$watch_pid\" 2> /dev/null\nstatus=$?\n"
    ))
);

mod no_fallback {
    use super::*;
    negative_control!(
        ci_watch_faults_leave_the_step_status,
        "no check of the tmpfs folder, so nothing is written",
        expected = "no `watch` log beside the sampler's",
        check_faults(&edit(
            &sampler(),
            "watch_dir=\"${RUNNER_TEMP:-/tmp}\"\n",
            ":\n"
        ))
    );
}

/// The step, and so any test it runs, never sees the real tmpfs named.
fn check_step_env(script: &str) {
    let bed = Bed::new("ci_watch_step_env");
    bed.run(
        script,
        "echo \"${RUNNER_DISK_TMPFS:-unset}\" > \"$BED/seen\"\n",
        Some(&bed.shm()),
        bed.path(),
        LIMIT,
    );
    let seen = std::fs::read_to_string(bed.dir.join("seen")).expect("the step ran");
    assert_eq!(seen.trim(), "unset", "the step sees RUNNER_DISK_TMPFS");
}

#[test]
fn ci_watch_keeps_the_tmpfs_from_the_step() {
    check_step_env(&sampler());
}

negative_control!(
    ci_watch_keeps_the_tmpfs_from_the_step,
    "the variable left in the step's environment",
    expected = "the step sees RUNNER_DISK_TMPFS",
    check_step_env(&edit(&sampler(), "unset RUNNER_DISK_TMPFS\n", ""))
);

/// Past the last sample, `watch` stops: `max_watch_samples` is what bounds the tmpfs file.
fn check_bound(script: &str) {
    let bed = Bed::new("ci_watch_bound");
    let two = edit(script, "max_watch_samples=4320", "max_watch_samples=2");
    bed.run(&two, "sleep 7\n", None, bed.path(), LIMIT);
    let count = samples(&bed.watch_log("temp").expect("no `watch` log"));
    assert_eq!(count, 2, "{count} samples with a bound of 2");
}

#[test]
fn ci_watch_stops_at_its_bound() {
    check_bound(&sampler());
}

negative_control!(
    ci_watch_stops_at_its_bound,
    "a `watch` that compares past its bound",
    expected = "3 samples with a bound of 2",
    check_bound(&edit(
        &sampler(),
        "n <= max_watch_samples",
        "n <= max_watch_samples + 1"
    ))
);

/// The sampler's own file, and the step's log, carry the same readings at its coarser interval: a job that uploads
/// nothing from the tmpfs, or a runner that dies, still has them.
fn check_in_samples(script: &str) {
    let bed = Bed::new("ci_watch_in_samples");
    let step = "for _ in {1..300}; do\n  \
         grep -q avail_kb \"$RUNNER_TEMP/runner-disk.log\" 2> /dev/null && break\n  sleep 0.1\ndone\n";
    let output = bed.run(script, step, None, bed.path(), LIMIT);
    let samples =
        std::fs::read_to_string(bed.dir.join("temp/runner-disk.log")).expect("no samples");
    let log = String::from_utf8_lossy(&output.stdout);
    for (name, text) in [("samples", samples.as_str()), ("step's log", &log)] {
        assert!(
            text.contains("loadavg 1.25") && text.contains("io some avg10=4.00"),
            "no load in the {name}:\n{text}"
        );
    }
}

#[test]
fn ci_disk_sampler_logs_the_load_in_each_sample() {
    check_in_samples(&sampler());
}

negative_control!(
    ci_disk_sampler_logs_the_load_in_each_sample,
    "samples of the disk alone",
    expected = "no load in the samples",
    check_in_samples(&edit(&sampler(), "$proc_lines\n$(du", "$(du"))
);

/// The text of `job`, up to the next job: a two-space key.
fn job(ci: &str, job: &str) -> String {
    let header = format!("  {job}:");
    let mut lines = ci.lines().skip_while(|l| *l != header);
    assert!(lines.next().is_some(), "ci.yml has no job `{job}`");
    lines
        .take_while(|l| {
            let key = l.starts_with("  ") && !l.starts_with("   ") && l.ends_with(':');
            !key || l.trim_start().starts_with('#')
        })
        .map(|l| format!("{l}\n"))
        .collect()
}

const DMESG: &str = "      - name: Kernel log on a failure\n        if: failure()\n        \
     continue-on-error: true\n        timeout-minutes: 1\n        run: ";

const JOBS: [(&str, &str); 2] = [
    ("ci-workspace", "runner-disk-ci-workspace\n"),
    ("ci-release", "runner-disk-ci-release-${{ matrix.shard }}\n"),
];

/// Each job names the tmpfs, and uploads from there and from the temp folder, whatever it ended in, with nothing able
/// to fail the job.
fn check_wired(ci: &str) {
    for (name, artifact) in JOBS {
        let job = job(ci, name);
        assert!(
            job.contains(
                "    env:\n      PRIN_GPU_BACKEND: vulkan\n      RUNNER_DISK_TMPFS: /dev/shm\n"
            ),
            "`{name}` names no tmpfs"
        );
        let upload = job
            .find("actions/upload-artifact@v4")
            .map(|at| &job[at..])
            .unwrap_or_else(|| panic!("`{name}` uploads nothing"));
        assert!(
            upload.starts_with("actions/upload-artifact@v4\n        if: always()")
                && upload.contains("\n        continue-on-error: true\n"),
            "`{name}`'s upload is not unconditional and harmless"
        );
        for path in [
            "            ${{ runner.temp }}/runner-disk.log\n",
            "            /dev/shm/runner-pressure.log\n",
            "            /dev/shm/runner-dmesg.log\n",
            "          overwrite: true\n",
        ] {
            assert!(
                upload.contains(path),
                "`{name}`'s upload has no `{}`",
                path.trim()
            );
        }
        assert!(
            upload.contains(&format!("          name: {artifact}")),
            "`{name}`'s artifact is not named for it"
        );
        let dmesg = job
            .find(DMESG)
            .unwrap_or_else(|| panic!("`{name}` has no kernel log"));
        assert!(
            job.rfind("disk_sampler.sh {0}")
                .is_some_and(|last| last < dmesg)
                && dmesg < job.find("actions/upload-artifact@v4").unwrap_or(0),
            "`{name}`'s kernel log is not between the sampled steps and the upload"
        );
    }
}

#[test]
fn ci_watch_files_upload_from_both_jobs() {
    check_wired(&ci_yml());
}

negative_control!(
    ci_watch_files_upload_from_both_jobs,
    "the upload only on a pass, when a failure is what it's for",
    expected = "upload is not unconditional and harmless",
    check_wired(&edit(
        &ci_yml(),
        "        if: always() && needs.changes.outputs.docs_only != 'true'\n",
        "        if: needs.changes.outputs.docs_only != 'true'\n"
    ))
);

mod no_tmpfs_named {
    use super::*;
    negative_control!(
        ci_watch_files_upload_from_both_jobs,
        "a job that leaves the sampler on the disk",
        expected = "`ci-release` names no tmpfs",
        check_wired(&edit(
            &ci_yml(),
            "    env:\n      PRIN_GPU_BACKEND: vulkan\n      RUNNER_DISK_TMPFS: /dev/shm\n    strategy:",
            "    env:\n      PRIN_GPU_BACKEND: vulkan\n    strategy:"
        ))
    );
}

mod kernel_log_last {
    use super::*;
    negative_control!(
        ci_watch_files_upload_from_both_jobs,
        "the kernel log taken after the upload, too late to go with it",
        expected = "`ci-workspace`'s kernel log is not between",
        {
            let ci = ci_yml();
            let at = ci.find(DMESG).expect("the step");
            let upload = at
                + ci[at..]
                    .find("      - uses: actions/upload-artifact")
                    .expect("upload");
            let end = upload + ci[upload..].find("overwrite: true\n").expect("end") + 16;
            check_wired(&format!(
                "{}{}{}{}",
                &ci[..at],
                &ci[upload..end],
                &ci[at..upload],
                &ci[end..]
            ))
        }
    );
}

/// The step's script, as the runner runs it, with `sudo` as `sudo_body`: in a scratch HOME and CARGO_HOME, on the job's
/// tmpfs variable pointed at scratch, under the stricter `-eo pipefail` of a `bash` shell.
fn run_dmesg(ci: &str, name: &str, sudo_body: &str) -> (Bed, Output) {
    let bed = Bed::new("ci_watch_dmesg");
    bed.fake("sudo", sudo_body);
    let job = job(ci, name);
    let script = job
        .split(DMESG)
        .nth(1)
        .and_then(|rest| rest.lines().next())
        .expect("the kernel log's script");
    let mut command = bed.command(bed.path());
    command
        .args(["--noprofile", "--norc", "-eo", "pipefail", "-c", script])
        .env("RUNNER_DISK_TMPFS", bed.shm());
    let output = command
        .output_within(Duration::from_secs(LIMIT))
        .expect("the step ran");
    (bed, output)
}

const KERNEL: &str =
    "#!/bin/sh\nn=0\nwhile [ $n -lt 300 ]; do n=$((n + 1)); echo \"kernel line $n\"; done\n";

fn check_dmesg(ci: &str) {
    for (name, _) in JOBS {
        let (bed, output) = run_dmesg(ci, name, KERNEL);
        assert!(output.status.success(), "the kernel log's step failed");
        let log = std::fs::read_to_string(bed.shm().join("runner-dmesg.log")).expect("no file");
        assert_eq!(log.lines().count(), 200, "not the log's tail");
        assert_eq!(
            log.lines().last(),
            Some("kernel line 300"),
            "not the log's tail"
        );
        let (_, refused) = run_dmesg(ci, name, "#!/bin/sh\necho refused >&2\nexit 1\n");
        assert!(
            refused.status.success(),
            "`{name}`'s step fails when sudo refuses"
        );
    }
}

#[test]
fn ci_watch_kernel_log_never_fails_its_step() {
    check_dmesg(&ci_yml());
}

negative_control!(
    ci_watch_kernel_log_never_fails_its_step,
    "the step's status left to sudo",
    expected = "`ci-workspace`'s step fails when sudo refuses",
    check_dmesg(&edit(
        &ci_yml(),
        "runner-dmesg.log\" || true\n",
        "runner-dmesg.log\"\n"
    ))
);
