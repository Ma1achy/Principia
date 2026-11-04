//! qa's tests for `ci-release`'s cleanup and the sampler's `watch` (`tooling/ci-release-disk`), written from the task's
//! words and the `ci-workspace` diagnosis, not from `.github/scripts/disk_sampler.sh`'s code: `ci-release` gets the
//! `ci` shards' cleanup and a sampler; the job's samples, a load and pressure log on a tmpfs and the kernel's log on a
//! failure are always uploaded, and read by no gate.
//!
//! - The workflow's steps are parsed, and **never run**: the cleanup holds `sudo rm -rf`. It is read token by token.
//! - The sampler alone is run, in a scratch folder with a scratch HOME and CARGO_HOME, a fake `sudo` first on the PATH
//!   and a scratch tmpfs. No test here touches the real /dev/shm or the real sudo.
//!
//! - `qa_ci_release_cleanup_is_the_shards_and_comes_first`: the cleanup's list is the `ci` shards', and comes before
//!   anything is installed or downloaded.
//! - `qa_ci_release_cleanup_leaves_the_runner_its_tools`: each path it removes is an absolute preinstall, named outright.
//! - `qa_ci_release_tests_run_under_the_sampler`: no step of the job runs tests outside the sampler.
//! - `qa_samples_reach_the_upload_whatever_the_job_ended_in`: the files the job's steps write are the files it uploads,
//!   after those steps, whatever they ended in, under a name no other job uses.
//! - `qa_sampler_step_status_and_streams_are_the_steps`: a step's status and its streams, with `watch` beside it, a
//!   step killed outright and a log that can't be written among the cases.
//! - `qa_second_step_keeps_the_first_steps_samples`: a job's sampled steps share the log.
//! - `qa_watch_leaves_only_its_log`: nothing but the log stays on the tmpfs.
//! - `qa_watch_reads_the_real_proc` (Linux): the sampler as shipped, on the real /proc.
#![cfg(unix)]

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;

use std::collections::BTreeSet;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Duration;

use scratch::Scratch;
use serde_json::Value;
use validation::negative_control;
use validation::spawn::Spawn;

const SAMPLER_SHELL: &str = "bash .github/scripts/disk_sampler.sh {0}";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the workspace root")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).expect("readable")
}

fn ci_yml() -> String {
    read(".github/workflows/ci.yml")
}

fn sampler() -> String {
    read(".github/scripts/disk_sampler.sh")
}

/// `from` in `text` replaced at its every place: an edit that no longer applies fails loudly.
#[cfg(feature = "controls")]
fn swap(text: &str, from: &str, to: &str) -> String {
    assert!(text.contains(from), "qa: no {from:?} to edit");
    text.replace(from, to)
}

/// `from` replaced at its first place after `after`.
#[cfg(feature = "controls")]
fn swap_after(text: &str, after: &str, from: &str, to: &str) -> String {
    let at = text.find(after).expect("qa: the place to edit");
    let (head, tail) = text.split_at(at);
    assert!(tail.contains(from), "qa: no {from:?} after {after:?}");
    format!("{head}{}", tail.replacen(from, to, 1))
}

fn parse(yml: &str) -> Value {
    let dir = Scratch::new("qa_ci_release_disk_yaml");
    std::fs::create_dir_all(&*dir).expect("scratch");
    let file = dir.join("wf.yml");
    std::fs::write(&file, yml).expect("written");
    let out = Command::new("python3")
        .args([
            "-c",
            "import json, sys, yaml\n\
             d = yaml.safe_load(open(sys.argv[1]))\n\
             if True in d: d['on'] = d.pop(True)\n\
             print(json.dumps(d))",
        ])
        .arg(&file)
        .timed_output()
        .expect("qa: python3 ran");
    assert!(
        out.status.success(),
        "qa: not YAML: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("qa: python3 wrote JSON")
}

fn steps<'a>(wf: &'a Value, job: &str) -> &'a [Value] {
    wf["jobs"][job]["steps"]
        .as_array()
        .unwrap_or_else(|| panic!("qa: no job `{job}`"))
}

fn script(step: &Value) -> &str {
    step["run"].as_str().unwrap_or("")
}

fn action(step: &Value) -> &str {
    step["uses"].as_str().unwrap_or("")
}

/// Where the cleanup of `job` is, and the paths it removes, read as words and never run.
fn cleanup(wf: &Value, job: &str) -> (usize, BTreeSet<String>) {
    let all = steps(wf, job);
    let at = all
        .iter()
        .position(|s| script(s).contains("sudo rm -rf"))
        .unwrap_or_else(|| panic!("qa: `{job}` frees no disk"));
    let joined = script(&all[at]).replace("\\\n", " ");
    let line = joined
        .lines()
        .find(|l| l.trim_start().starts_with("sudo rm -rf"))
        .expect("qa: the `rm` line");
    let paths = line.split_whitespace().skip(3).map(str::to_owned).collect();
    (at, paths)
}

/// `ci-release`'s list is the `ci` shards', and nothing has been put on the disk before it.
fn check_cleanup_first(yml: &str) {
    let wf = parse(yml);
    let (at, paths) = cleanup(&wf, "ci-release");
    let (_, shards) = cleanup(&wf, "ci");
    assert_eq!(paths, shards, "`ci-release` frees other paths than `ci`");
    for (n, step) in steps(&wf, "ci-release").iter().enumerate() {
        let fills = ["rustup", "apt-get", "pip", "cargo "]
            .iter()
            .any(|word| script(step).contains(word))
            || action(step).starts_with("actions/download-artifact")
            || action(step).starts_with("taiki-e/")
            || action(step).starts_with("actions/setup-python");
        assert!(
            !fills || n > at,
            "`ci-release` fills the disk before it frees it: step {n}"
        );
    }
}

#[test]
fn qa_ci_release_cleanup_is_the_shards_and_comes_first() {
    check_cleanup_first(&ci_yml());
}

negative_control!(
    qa_ci_release_cleanup_is_the_shards_and_comes_first,
    "`ci-release` keeps the JVMs the shards remove",
    expected = "`ci-release` frees other paths than `ci`",
    check_cleanup_first(&swap_after(
        &ci_yml(),
        "\n  ci-release:\n",
        "/usr/lib/jvm ",
        ""
    ))
);

mod late {
    use super::*;
    negative_control!(
        qa_ci_release_cleanup_is_the_shards_and_comes_first,
        "`ci-release` installs a tool before it frees the disk",
        expected = "`ci-release` fills the disk before it frees it",
        check_cleanup_first(&swap_after(
            &ci_yml(),
            "\n  ci-release:\n",
            "      - name: Free runner disk\n",
            "      - run: rustup toolchain install\n      - name: Free runner disk\n"
        ))
    );
}

/// What a test step needs from the runner, none of which a cleanup may reach.
const KEPT: [&str; 9] = [
    "/home/runner",
    "/usr/bin",
    "/usr/local/bin",
    "/usr/lib/x86_64-linux-gnu",
    "/etc",
    "/tmp",
    "/dev/shm",
    "/opt/hostedtoolcache/Python",
    "/mnt",
];

/// Every word is an absolute path two folders deep at least, with no variable or relative part, and none holds or is
/// what the job goes on using.
fn check_cleanup_paths(yml: &str) {
    let wf = parse(yml);
    let (_, paths) = cleanup(&wf, "ci-release");
    assert!(paths.len() > 3, "qa: too few paths read: {paths:?}");
    for path in &paths {
        let folders = path.split('/').filter(|p| !p.is_empty()).count();
        assert!(
            path.starts_with('/') && folders >= 2 && !path.contains('$') && !path.contains(".."),
            "the cleanup removes `{path}`, not a preinstall named outright"
        );
        let fixed = path.trim_end_matches('*').trim_end_matches('/');
        for kept in KEPT {
            assert!(
                !kept.starts_with(fixed) && !fixed.starts_with(kept),
                "the cleanup reaches `{kept}` through `{path}`"
            );
        }
    }
}

#[test]
fn qa_ci_release_cleanup_leaves_the_runner_its_tools() {
    check_cleanup_paths(&ci_yml());
}

negative_control!(
    qa_ci_release_cleanup_leaves_the_runner_its_tools,
    "a variable among the paths",
    expected = "not a preinstall named outright",
    check_cleanup_paths(&swap_after(
        &ci_yml(),
        "\n  ci-release:\n",
        "/usr/share/dotnet ",
        "/usr/share/dotnet $HOME/.cargo "
    ))
);

mod too_wide {
    use super::*;
    negative_control!(
        qa_ci_release_cleanup_leaves_the_runner_its_tools,
        "the whole tool cache, which holds the Python the job goes on to set up",
        expected = "the cleanup reaches `/opt/hostedtoolcache/Python`",
        check_cleanup_paths(&swap_after(
            &ci_yml(),
            "\n  ci-release:\n",
            "/usr/share/dotnet ",
            "/usr/share/dotnet /opt/hostedtoolcache "
        ))
    );
}

fn runs_tests(step: &Value) -> bool {
    let run = script(step);
    run.contains("cargo nextest run") || run.contains("cargo test")
}

fn check_sampled(yml: &str) {
    let wf = parse(yml);
    let tests: Vec<&Value> = steps(&wf, "ci-release")
        .iter()
        .filter(|s| runs_tests(s))
        .collect();
    assert!(tests.len() >= 2, "qa: `ci-release` runs no tests");
    for step in tests {
        assert_eq!(
            step["shell"].as_str(),
            Some(SAMPLER_SHELL),
            "`{}` runs outside the sampler",
            script(step)
        );
    }
}

#[test]
fn qa_ci_release_tests_run_under_the_sampler() {
    check_sampled(&ci_yml());
}

negative_control!(
    qa_ci_release_tests_run_under_the_sampler,
    "the nextest shard on the default shell",
    expected = "runs outside the sampler",
    check_sampled(&swap_after(
        &ci_yml(),
        "\n  ci-release:\n",
        "/2\n        shell: bash .github/scripts/disk_sampler.sh {0}\n",
        "/2\n"
    ))
);

/// The upload of `job` that carries `file`, with where it stands among the steps.
fn upload_of<'a>(wf: &'a Value, job: &str, file: &str) -> (usize, &'a Value) {
    steps(wf, job)
        .iter()
        .enumerate()
        .find(|(_, s)| {
            action(s).starts_with("actions/upload-artifact")
                && s["with"]["path"].as_str().is_some_and(|p| p.contains(file))
        })
        .unwrap_or_else(|| panic!("`{job}` uploads no `{file}`"))
}

fn check_uploads(yml: &str) {
    let wf = parse(yml);
    for job in ["ci-workspace", "ci-release"] {
        let tmpfs = wf["jobs"][job]["env"]["RUNNER_DISK_TMPFS"]
            .as_str()
            .unwrap_or_else(|| panic!("`{job}` names no tmpfs"));
        let (at, upload) = upload_of(&wf, job, "runner-disk.log");
        let paths = upload["with"]["path"].as_str().expect("checked");
        for path in [
            "${{ runner.temp }}/runner-disk.log".to_owned(),
            format!("{tmpfs}/runner-pressure.log"),
            format!("{tmpfs}/runner-dmesg.log"),
        ] {
            assert!(
                paths.lines().any(|l| l.trim() == path),
                "`{job}`'s upload has no `{path}`"
            );
        }
        let cond = upload["if"].as_str().unwrap_or("");
        assert!(
            cond.contains("always()"),
            "`{job}`'s upload depends on how the job went: `{cond}`"
        );
        assert_eq!(
            upload["continue-on-error"],
            Value::Bool(true),
            "`{job}`'s upload can fail the job"
        );
        let all = steps(&wf, job);
        let last_sampled = all
            .iter()
            .rposition(|s| s["shell"].as_str() == Some(SAMPLER_SHELL))
            .expect("qa: no sampled step");
        let dmesg = all
            .iter()
            .position(|s| script(s).contains("dmesg"))
            .unwrap_or_else(|| panic!("`{job}` takes no kernel log"));
        assert_eq!(
            all[dmesg]["if"].as_str(),
            Some("failure()"),
            "`{job}`'s kernel log is not for a failure"
        );
        assert_eq!(
            all[dmesg]["continue-on-error"],
            Value::Bool(true),
            "`{job}`'s kernel log can fail the job"
        );
        assert!(
            script(&all[dmesg]).contains("$RUNNER_DISK_TMPFS/runner-dmesg.log"),
            "`{job}`'s kernel log goes elsewhere than its upload"
        );
        assert!(
            last_sampled < dmesg && dmesg < at,
            "`{job}` uploads before its steps have written"
        );
    }
    // `overwrite: true` would let two jobs' artifacts replace each other, silently.
    let mut names = BTreeSet::new();
    for (id, job) in wf["jobs"].as_object().expect("qa: jobs") {
        for step in job["steps"].as_array().map_or(&[][..], Vec::as_slice) {
            if action(step).starts_with("actions/upload-artifact") {
                let name = step["with"]["name"].as_str().unwrap_or("").to_owned();
                assert!(
                    names.insert(name.clone()),
                    "two uploads are named `{name}`: `{id}`'s and another's"
                );
            }
        }
    }
}

#[test]
fn qa_samples_reach_the_upload_whatever_the_job_ended_in() {
    check_uploads(&ci_yml());
}

negative_control!(
    qa_samples_reach_the_upload_whatever_the_job_ended_in,
    "the upload on a pass alone, when a failure is what it is for",
    expected = "`ci-release`'s upload depends on how the job went",
    check_uploads(&swap_after(
        &ci_yml(),
        "\n  ci-release:\n",
        "        if: always()\n        continue-on-error: true\n",
        "        continue-on-error: true\n"
    ))
);

mod other_tmpfs {
    use super::*;
    negative_control!(
        qa_samples_reach_the_upload_whatever_the_job_ended_in,
        "a job whose sampler writes to one folder and whose upload reads another",
        expected = "`ci-workspace`'s upload has no `/run/shm/runner-pressure.log`",
        check_uploads(&swap(
            &ci_yml(),
            "      RUNNER_DISK_TMPFS: /dev/shm\n",
            "      RUNNER_DISK_TMPFS: /run/shm\n"
        ))
    );
}

mod same_name {
    use super::*;
    negative_control!(
        qa_samples_reach_the_upload_whatever_the_job_ended_in,
        "the release shards' samples under one name, each replacing the last job's",
        expected = "two uploads are named `runner-disk-ci-workspace`",
        check_uploads(&swap(
            &ci_yml(),
            "name: runner-disk-ci-release-${{ matrix.shard }}\n",
            "name: runner-disk-ci-workspace\n"
        ))
    );
}

/// A step's place: a scratch temp folder, tmpfs, HOME and CARGO_HOME, and a fake `sudo` and `timeout` first on the PATH.
struct Bed {
    dir: Scratch,
}

impl Bed {
    fn new(tag: &str) -> Bed {
        let dir = Scratch::new(tag);
        for folder in ["temp", "shm", "bin", "home/cargo"] {
            std::fs::create_dir_all(dir.join(folder)).expect("scratch folder");
        }
        let bed = Bed { dir };
        for (name, body) in [
            ("sudo", "#!/bin/sh\necho fake device\n"),
            ("timeout", "#!/bin/sh\nshift\nexec \"$@\"\n"),
        ] {
            let path = bed.dir.join("bin").join(name);
            std::fs::write(&path, body).expect("written");
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("mode");
        }
        bed
    }

    /// The sampler, as `script`, on the step `step`. No test here runs a `run:` step of the workflow.
    fn run(&self, script: &str, step: &str, tmpfs: bool) -> Output {
        std::fs::write(self.dir.join("sampler.sh"), script).expect("written");
        std::fs::write(self.dir.join("step.sh"), step).expect("written");
        let path = format!(
            "{}:{}",
            self.dir.join("bin").display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let mut command = Command::new("bash");
        command
            .args(["sampler.sh", "step.sh"])
            .current_dir(&*self.dir)
            .env("BED", &*self.dir)
            .env("HOME", self.dir.join("home"))
            .env("CARGO_HOME", self.dir.join("home/cargo"))
            .env("PATH", path)
            .env("RUNNER_TEMP", self.dir.join("temp"))
            .env("RUNNER_DISK_SAMPLE_SECONDS", "2")
            .env("RUNNER_PRESSURE_SAMPLE_SECONDS", "1")
            .env_remove("RUNNER_DISK_TMPFS");
        if tmpfs {
            command.env("RUNNER_DISK_TMPFS", self.dir.join("shm"));
        }
        command
            .output_within(Duration::from_secs(60))
            .expect("qa: the sampler ran")
    }

    fn pressure(&self) -> String {
        std::fs::read_to_string(self.dir.join("shm/runner-pressure.log")).unwrap_or_default()
    }
}

/// Ends once `watch`'s log, wherever the bed has it, holds `n` samples.
fn until_samples(n: usize) -> String {
    format!(
        "for _ in {{1..300}}; do\n  \
         [ \"$(cat \"$BED\"/shm/runner-pressure.log 2> /dev/null | grep -c '^== watch')\" -ge {n} ] && break\n  \
         sleep 0.1\ndone\nexit 0\n"
    )
}

fn check_status_and_streams(script: &str) {
    let long_enough = "sleep 1.5\n";
    for (step, want, case) in [
        (format!("{long_enough}exit 7\n"), Some(7), "an exit"),
        (
            format!("{long_enough}false\necho not here\n"),
            Some(1),
            "`bash -e`",
        ),
        (format!("{long_enough}kill -KILL $$\n"), Some(137), "a kill"),
    ] {
        let bed = Bed::new("qa_watch_status");
        let got = bed.run(script, &step, true).status.code();
        assert_eq!(got, want, "the step's status after {case}");
    }

    // The step alone writes the job's stderr, and all of it comes through, the sampler's end among it.
    let bed = Bed::new("qa_watch_streams");
    let output = bed.run(
        script,
        "echo step out\necho step err >&2\nsleep 1.5\n",
        true,
    );
    let (out, err) = (
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    assert_eq!(err.trim(), "step err", "more than the step on stderr");
    assert!(
        out.contains("step out"),
        "the step's stdout was lost: {out}"
    );
    assert!(!out.contains("== watch"), "`watch` wrote to the step's log");

    // A log it can't write: the watch's path is a folder.
    let bed = Bed::new("qa_watch_unwritable");
    std::fs::create_dir_all(bed.dir.join("shm/runner-pressure.log")).expect("scratch folder");
    let output = bed.run(script, "sleep 1.5\nexit 5\n", true);
    assert_eq!(output.status.code(), Some(5), "the step's status, no log");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "",
        "more than the step on stderr, with no log"
    );
}

#[test]
fn qa_sampler_step_status_and_streams_are_the_steps() {
    check_status_and_streams(&sampler());
}

negative_control!(
    qa_sampler_step_status_and_streams_are_the_steps,
    "a sampler that passes every step",
    expected = "the step's status after an exit",
    check_status_and_streams(&swap(&sampler(), "exit \"$status\"", "exit 0"))
);

mod noisy_watch {
    use super::*;
    negative_control!(
        qa_sampler_step_status_and_streams_are_the_steps,
        "`watch` with the step's stderr",
        expected = "more than the step on stderr",
        check_status_and_streams(&swap(
            &sampler(),
            "  exec > /dev/null 2>&1 < /dev/null\n  [ -z",
            "  [ -z"
        ))
    );
}

/// Both steps' samples are in the job's one log, each marked by its time.
fn check_two_steps(script: &str) {
    let bed = Bed::new("qa_watch_two_steps");
    bed.run(script, &until_samples(2), true);
    let first = bed.pressure().matches("== watch").count();
    bed.run(script, &until_samples(first + 2), true);
    let both = bed.pressure().matches("== watch").count();
    assert!(
        first >= 2 && both >= first + 2,
        "the second step lost the first's samples: {first} then {both}"
    );
}

#[test]
fn qa_second_step_keeps_the_first_steps_samples() {
    check_two_steps(&sampler());
}

negative_control!(
    qa_second_step_keeps_the_first_steps_samples,
    "a log begun anew by each sample",
    expected = "the second step lost the first's samples",
    check_two_steps(&swap(
        &sampler(),
        "\"$proc_lines\" >> \"$watch_out\"",
        "\"$proc_lines\" > \"$watch_out\""
    ))
);

/// The upload names its files one by one, and a tmpfs is memory: nothing else stays there.
fn check_only_the_log(script: &str) {
    let bed = Bed::new("qa_watch_only_the_log");
    bed.run(script, &until_samples(1), true);
    let mut left: Vec<String> = std::fs::read_dir(bed.dir.join("shm"))
        .expect("the tmpfs folder")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    left.sort();
    assert_eq!(left, ["runner-pressure.log"], "left on the tmpfs");
}

#[test]
fn qa_watch_leaves_only_its_log() {
    check_only_the_log(&sampler());
}

negative_control!(
    qa_watch_leaves_only_its_log,
    "a nap's pipe left where it was made",
    expected = "left on the tmpfs",
    check_only_the_log(&swap(&sampler(), "  rm -f \"$fifo\"\n", ""))
);

/// The real /proc, which every other test here and in `ci_runner_watch.rs` either replaces or lacks.
#[cfg(target_os = "linux")]
mod on_linux {
    use super::*;

    fn check_real_proc(script: &str) {
        let bed = Bed::new("qa_watch_real_proc");
        bed.run(script, &until_samples(1), true);
        let log = bed.pressure();
        let load = log
            .lines()
            .find_map(|l| l.strip_prefix("loadavg "))
            .unwrap_or("none");
        let cpus = log
            .lines()
            .find_map(|l| l.strip_prefix("nproc "))
            .unwrap_or("none");
        assert!(
            load.starts_with(|c: char| c.is_ascii_digit())
                && cpus.starts_with(|c: char| c.is_ascii_digit())
                && log.contains("Dirty:")
                && log.contains("Writeback:"),
            "no load read from the real /proc:\n{log}"
        );
    }

    #[test]
    fn qa_watch_reads_the_real_proc() {
        check_real_proc(&sampler());
    }

    negative_control!(
        qa_watch_reads_the_real_proc,
        "a sampler that reads a folder that isn't there",
        expected = "no load read from the real /proc",
        check_real_proc(&swap(&sampler(), "proc=/proc\n", "proc=/no/such/proc\n"))
    );
}
