//! `.github/scripts/disk_sampler.sh`, the `shell:` of `cargo xtask ci` in `xtask-ci` (applied per R-369): a measurement
//! of the runner's disk that can't change the step's result. It exits with the step script's status, under the default
//! shell's `-e`; it writes its samples, bounded in number; it doesn't outlive the step; and the job wires it in and
//! uploads what it wrote, on a failure too.
#![cfg(unix)]

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Duration;

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

const SAMPLER: &str = ".github/scripts/disk_sampler.sh";

/// The shell `sampler` run on a step script `step`, in a scratch folder, `interval` seconds apart, killed at `limit`.
fn run(
    tag: &str,
    sampler: &str,
    step: &str,
    interval: u32,
    limit: u64,
) -> std::io::Result<(Output, Option<String>)> {
    let dir = Scratch::new(tag);
    std::fs::create_dir_all(&*dir).expect("scratch folder");
    std::fs::write(dir.join("sampler.sh"), sampler).expect("sampler written");
    std::fs::write(dir.join("step.sh"), step).expect("step written");
    let output = Command::new("bash")
        .args(["sampler.sh", "step.sh"])
        .current_dir(&*dir)
        .env("RUNNER_TEMP", &*dir)
        .env("RUNNER_DISK_SAMPLE_SECONDS", interval.to_string())
        .output_within(Duration::from_secs(limit))?;
    let samples = std::fs::read_to_string(dir.join("runner-disk.log")).ok();
    Ok((output, samples))
}

/// A long way past any run here: this bound is the one on a run that went wrong.
const LIMIT: u64 = 60;

fn check_status(sampler: &str) {
    // Both sides of a pass: each script's own exit, and a failing command in mid-script under `-e`.
    for (step, expected) in [
        ("exit 0\n", 0),
        ("exit 3\n", 3),
        ("false\nexit 0\n", 1),
        ("true\nexit 0\n", 0),
    ] {
        let (output, _) = run("ci_disk_status", sampler, step, 600, LIMIT).expect("sampler ran");
        assert_eq!(
            output.status.code(),
            Some(expected),
            "the step {step:?} exited {:?} under the sampler",
            output.status.code()
        );
    }
}

#[test]
fn ci_disk_sampler_exits_with_the_steps_status() {
    check_status(&read(SAMPLER));
}

negative_control!(
    ci_disk_sampler_exits_with_the_steps_status,
    "a sampler that hides the step's failure",
    expected = "the step \"exit 3\\n\" exited Some(0) under the sampler",
    check_status(&read(SAMPLER).replace("exit \"$status\"", "exit 0"))
);

mod no_dash_e {
    use super::*;
    negative_control!(
        ci_disk_sampler_exits_with_the_steps_status,
        "a sampler running the step without the default shell's `-e`",
        expected = "the step \"false\\nexit 0\\n\" exited Some(0) under the sampler",
        check_status(&read(SAMPLER).replace("bash -e \"$script\"", "bash \"$script\""))
    );
}

fn check_samples(sampler: &str) {
    let (output, samples) =
        run("ci_disk_samples", sampler, "sleep 3\n", 1, LIMIT).expect("sampler ran");
    let samples = samples.expect("the sampler wrote no file");
    let count = samples.matches("avail_kb=").count();
    assert!(count >= 2, "{count} samples in 3 s, 1 s apart:\n{samples}");
    assert!(
        samples.contains("/tmp"),
        "a sample has no `du` of /tmp:\n{samples}"
    );
    let log = String::from_utf8_lossy(&output.stdout);
    assert!(
        log.contains("== sample 1,"),
        "the step's log has no first sample:\n{log}"
    );
    assert!(
        log.contains("lowest free") && log.contains(&format!("over {count} samples")),
        "the step's log has no lowest free over the {count} samples:\n{log}"
    );
}

#[test]
fn ci_disk_sampler_writes_samples_and_the_lowest_free() {
    check_samples(&read(SAMPLER));
}

negative_control!(
    ci_disk_sampler_writes_samples_and_the_lowest_free,
    "a sampler that writes no file",
    expected = "the sampler wrote no file",
    check_samples(&read(SAMPLER).replace(">> \"$out\"", ">> /dev/null"))
);

/// Past the last sample, it stops: `max_samples` is what bounds the file.
fn check_bounded(sampler: &str) {
    let two = sampler.replace("max_samples=480", "max_samples=2");
    let (_, samples) = run("ci_disk_bound", &two, "sleep 4\n", 1, LIMIT).expect("sampler ran");
    let count = samples.expect("no samples").matches("avail_kb=").count();
    assert_eq!(count, 2, "{count} samples with a bound of 2");
}

#[test]
fn ci_disk_sampler_stops_at_its_bound() {
    check_bounded(&read(SAMPLER));
}

negative_control!(
    ci_disk_sampler_stops_at_its_bound,
    "a sampler that compares past its bound",
    expected = "3 samples with a bound of 2",
    check_bounded(&read(SAMPLER).replace("n <= max_samples", "n <= max_samples + 1"))
);

/// Well under the sampler's own 15 s wait, so its cleanup has to work; the next sample, at 600 s, is far past it.
fn check_ends(sampler: &str) {
    run("ci_disk_ends", sampler, "exit 0\n", 600, 10)
        .expect("the sampler outlived the step's script");
}

#[test]
fn ci_disk_sampler_does_not_outlive_the_step() {
    check_ends(&read(SAMPLER));
}

negative_control!(
    ci_disk_sampler_does_not_outlive_the_step,
    "a sampler left running when the step's script ends",
    expected = "the sampler outlived the step's script",
    check_ends(
        &read(SAMPLER)
            .replace("kill -TERM \"$sampler_pid\" 2> /dev/null", "")
            .replace("kill -KILL \"$sampler_pid\" 2> /dev/null", "")
    )
);

const STEP: &str = "      - name: cargo xtask ci --partition ${{ matrix.shard }}/4\n        \
     shell: bash .github/scripts/disk_sampler.sh {0}\n        \
     run: cargo xtask ci --partition ${{ matrix.shard }}/4\n";

const AFTER: &str = "      - name: Runner disk after cargo xtask ci\n        \
     if: always()\n        run: df -h /\n      - uses: actions/upload-artifact@v4\n        \
     if: always()\n        with:\n          name: runner-disk-xtask-ci-${{ matrix.shard }}\n          \
     path: ${{ runner.temp }}/runner-disk.log\n";

fn check_wired(workflow: &str) {
    assert!(
        workflow.contains(STEP),
        "`cargo xtask ci` is not run under the sampler"
    );
    assert!(
        workflow.contains(&format!("{STEP}{AFTER}")),
        "the samples are not uploaded straight after the step, whatever it ended in"
    );
}

#[test]
fn ci_disk_sampler_is_the_steps_shell_and_its_file_is_uploaded() {
    check_wired(&read(".github/workflows/ci.yml"));
}

negative_control!(
    ci_disk_sampler_is_the_steps_shell_and_its_file_is_uploaded,
    "the upload only on a pass, when a failure is what it's for",
    expected = "the samples are not uploaded straight after the step",
    check_wired(&read(".github/workflows/ci.yml").replace(
        "      - uses: actions/upload-artifact@v4\n        if: always()\n",
        "      - uses: actions/upload-artifact@v4\n",
    ))
);
