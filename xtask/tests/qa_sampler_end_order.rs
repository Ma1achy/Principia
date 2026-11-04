//! qa's tests for the end of `.github/scripts/disk_sampler.sh` once its processes have left the jobs table
//! (`tooling/ci-release-disk`), when no `wait` can hold the main shell any more. Written from the sampler's stated order
//! and bounds, not from how it polls: `watch` goes first, the sampler gets a TERM and up to 2 s to go at its trap, and
//! only one still there then gets a KILL; each of those waits is bounded, "well under 10 s" in all
//! (`qa_sampler_cleanup_bound.rs`), even for a process no signal stops, as one in D state on a stalled runner is.
//!
//! - `qa_sampler_end_is_bounded_with_nothing_going`: a `watch` and a sampler that outlive every signal hold up the step
//!   by the bounds alone, and leave the step's status.
//! - `qa_sampler_command_in_hand_gets_its_grace`: a command the sampler has in hand when the step ends is let finish
//!   within the grace, not cut off by a KILL at once.
//!
//! Only the sampler is run, on a scratch step, in a scratch folder with a scratch HOME and CARGO_HOME, and with no
//! tmpfs named, so nothing asks for sudo. Each test has its negative control (R-176).
#![cfg(unix)]

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use scratch::Scratch;
use validation::negative_control;
use validation::spawn::Spawn;

fn sampler() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(".github/scripts/disk_sampler.sh");
    std::fs::read_to_string(path).expect("the sampler is readable")
}

/// `from` in `text`, replaced; an edit that no longer applies fails loudly rather than leave the check unchanged.
fn edit(text: &str, from: &str, to: &str) -> String {
    assert!(text.contains(from), "qa: the sampler has no {from:?}");
    text.replace(from, to)
}

/// The sampler's whole end, with every wait at its bound: well under this.
const ENDS_WITHIN: Duration = Duration::from_secs(10);
/// How long the stand-ins for a process no signal stops last at the least: past `ENDS_WITHIN`, so that a wait for them
/// to go shows.
const LIVES_SECONDS: u32 = 12;
/// The harness's own limit, far past both, so that a control fails on this file's assertions and not on the harness.
const HARNESS_LIMIT: Duration = Duration::from_secs(60);

struct Run {
    dir: Scratch,
    status: Option<String>,
    took: Duration,
}

/// `script` as the sampler on the step `step`. Its streams go to files, so the time taken is the main shell's, and not
/// that of whatever is left holding a stream.
fn run(tag: &str, script: &str, step: &str, sample_seconds: u32) -> Run {
    let dir = Scratch::new(tag);
    for folder in ["temp", "home/cargo"] {
        std::fs::create_dir_all(dir.join(folder)).expect("scratch folder");
    }
    std::fs::write(dir.join("sampler.sh"), script).expect("sampler written");
    std::fs::write(dir.join("step.sh"), step).expect("step written");
    let start = Instant::now();
    Command::new("bash")
        .args([
            "-c",
            "bash sampler.sh step.sh > out.log 2> err.log; echo $? > status",
        ])
        .current_dir(&*dir)
        .env("BED", &*dir)
        .env("HOME", dir.join("home"))
        .env("CARGO_HOME", dir.join("home/cargo"))
        .env("RUNNER_TEMP", dir.join("temp"))
        .env("RUNNER_DISK_SAMPLE_SECONDS", sample_seconds.to_string())
        .env("RUNNER_PRESSURE_SAMPLE_SECONDS", "1")
        .env_remove("RUNNER_DISK_TMPFS")
        .output_within(HARNESS_LIMIT)
        .expect("qa: the sampler ran within the harness's limit");
    let took = start.elapsed();
    let status = std::fs::read_to_string(dir.join("status"))
        .ok()
        .map(|s| s.trim().to_owned());
    Run { dir, status, took }
}

/// The sampler with what a process no signal stops looks like from the main shell: its KILLs reach nothing and the
/// sampler ignores the TERM. Each stops by its own bound, and the main shell leaves the groups to end the test with.
fn nothing_goes(script: &str) -> String {
    let no_watch_kill = edit(script, "kill -KILL -- \"-$watch_pid\" 2> /dev/null\n", "");
    let no_kills = edit(
        &no_watch_kill,
        "kill -0 \"$sampler_pid\" 2> /dev/null && kill -KILL -- \"-$sampler_pid\" 2> /dev/null\n",
        "",
    );
    let no_term = edit(
        &no_kills,
        "trap 'kill \"${!:-}\"; exit 0' TERM",
        "trap '' TERM",
    );
    let one_nap = edit(&no_term, "max_samples=480", "max_samples=1");
    let bounded = edit(
        &one_nap,
        "max_watch_samples=4320",
        &format!("max_watch_samples={LIVES_SECONDS}"),
    );
    edit(
        &bounded,
        "exit \"$status\"",
        "echo \"$watch_pid $sampler_pid\" > \"$BED/groups\"\nexit \"$status\"",
    )
}

fn check_bounded(script: &str) {
    let run = run(
        "qa_sampler_nothing_goes",
        &nothing_goes(script),
        "exit 3\n",
        LIVES_SECONDS,
    );
    // Nothing of this test's goes on past it, whatever it finds.
    let groups = "read -r watch sampler < groups; kill -KILL -- \"-$watch\" \"-$sampler\"";
    let _ = Command::new("bash")
        .args(["-c", groups])
        .current_dir(&*run.dir)
        .output_within(Duration::from_secs(10));
    assert_eq!(
        run.status.as_deref(),
        Some("3"),
        "the step's status with nothing gone"
    );
    assert!(
        run.took < ENDS_WITHIN,
        "the step took {:?} to end with nothing going",
        run.took
    );
}

#[test]
fn qa_sampler_end_is_bounded_with_nothing_going() {
    check_bounded(&sampler());
}

negative_control!(
    qa_sampler_end_is_bounded_with_nothing_going,
    "a wait far past its bound",
    expected = "to end with nothing going",
    check_bounded(&edit(
        &sampler(),
        "local end=$((SECONDS + 2))",
        "local end=$((SECONDS + 15))"
    ))
);

/// The first `df` of the first sample has a short command in hand, and the step ends while it runs. A marker it writes
/// at its own end shows it was let finish.
fn check_grace(script: &str) {
    let in_hand = edit(
        script,
        "    avail=$(df -Pk / |",
        "    avail=$(touch \"$BED/in_hand\"; sleep 0.5; touch \"$BED/finished\"; df -Pk / |",
    );
    let step =
        "for _ in {1..300}; do\n  [ -e \"$BED/in_hand\" ] && exit 3\n  sleep 0.1\ndone\nexit 9\n";
    let run = run("qa_sampler_grace", &in_hand, step, 600);
    assert_eq!(run.status.as_deref(), Some("3"), "the step's status");
    assert!(
        run.dir.join("finished").exists(),
        "the sampler's command in hand was cut off, with no grace before the KILL"
    );
}

#[test]
fn qa_sampler_command_in_hand_gets_its_grace() {
    check_grace(&sampler());
}

negative_control!(
    qa_sampler_command_in_hand_gets_its_grace,
    "a KILL at once, with no TERM and no wait before it",
    expected = "with no grace before the KILL",
    check_grace(&edit(
        &sampler(),
        "kill -TERM \"$sampler_pid\" 2> /dev/null\nwait_gone \"$sampler_pid\"\n",
        ""
    ))
);
