//! QA tests for tooling/sampler-cleanup-bound: `.github/scripts/disk_sampler.sh` ends within its own bound whatever the
//! sampler has in hand, and leaves nothing of the sampler running, read against the task's "The change" item 1 ("Make
//! the main shell's cleanup bounded on its own, well under 10 s, independent of the sampler's trap ... KILL the group so
//! the in-hand df/du and the nap die with it") and CI's failure (job 125622051950).
//!
//! The slow command here sits inside the command substitution, where CI's `df` and `du` run: its output goes to the
//! sampler's own pipe, not the step's, so ending the step on time is not enough to show it was ended. It touches a
//! marker once its sleep is over; a marker there after the run shows it outlived the cleanup. The step exits 3, so the
//! status the step ends with is checked on the KILL path too.
//!
//! Discriminates on the Mac's bash 3.2 as well as CI's bash 5 (checked by hand on 3.2, 3 Nov 2026: the old 15 s cleanup
//! ended at the substitution's end and left the marker; a KILL of the sampler alone ended in 2 s and left the marker).
//!
//! Each test has its negative control (R-176).
#![cfg(unix)]

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;

use std::path::{Path, PathBuf};
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
    assert!(text.contains(from), "the sampler has no {from:?}");
    text.replace(from, to)
}

/// How long the slow command in hand sleeps: past the step's 10 s limit, as CI's slow `df` or `du` was.
const SLOW: u64 = 12;
/// The step's end, from its start, with the cleanup bound at about 2 s: well short of `SLOW`.
const ENDS_WITHIN: Duration = Duration::from_secs(6);
/// The harness's own limit, far past both, so a control fails on this file's assertions and not on the harness.
const HARNESS_LIMIT: u64 = 40;

/// The sampler with a slow command first in its first sample's `df` substitution, which marks `marker` once done.
fn with_a_slow_substitution(sampler: &str, marker: &Path) -> String {
    edit(
        sampler,
        "    avail=$(df -Pk / |",
        &format!(
            "    avail=$(sleep {SLOW}; touch '{}'; df -Pk / |",
            marker.display()
        ),
    )
}

fn check_bounded_and_clean(sampler: &str) {
    let dir = Scratch::new("qa_sampler_cleanup");
    std::fs::create_dir_all(&*dir).expect("scratch folder");
    let marker: PathBuf = dir.join("slow_command_finished");
    std::fs::write(
        dir.join("sampler.sh"),
        with_a_slow_substitution(sampler, &marker),
    )
    .expect("sampler written");
    std::fs::write(dir.join("step.sh"), "exit 3\n").expect("step written");

    let start = Instant::now();
    let output = Command::new("bash")
        .args(["sampler.sh", "step.sh"])
        .current_dir(&*dir)
        .env("RUNNER_TEMP", &*dir)
        .env("RUNNER_DISK_SAMPLE_SECONDS", "600")
        .output_within(Duration::from_secs(HARNESS_LIMIT))
        .expect("the sampler ran within the harness's limit");
    let elapsed = start.elapsed();

    assert_eq!(
        output.status.code(),
        Some(3),
        "the step's status was lost on the cleanup's KILL path"
    );
    assert!(
        elapsed < ENDS_WITHIN,
        "the step took {elapsed:?} to end with a slow command in the sampler's hand"
    );
    // Past the slow command's own end, with room to spare: had it survived the cleanup, it has marked by now.
    std::thread::sleep(Duration::from_secs(SLOW + 3).saturating_sub(start.elapsed()));
    assert!(
        !marker.exists(),
        "the sampler's command in hand outlived the cleanup"
    );
}

#[test]
fn qa_sampler_cleanup_bounded_and_ends_the_command_in_hand() {
    check_bounded_and_clean(&sampler());
}

negative_control!(
    qa_sampler_cleanup_bounded_and_ends_the_command_in_hand,
    "the old cleanup's 15 s wait for the sampler's trap",
    expected = "to end with a slow command in the sampler's hand",
    check_bounded_and_clean(&edit(
        &sampler(),
        "end=$((SECONDS + 2))",
        "end=$((SECONDS + 15))"
    ))
);

mod sampler_only {
    use super::*;
    negative_control!(
        qa_sampler_cleanup_bounded_and_ends_the_command_in_hand,
        "a KILL of the sampler alone, leaving its command substitution running",
        expected = "the sampler's command in hand outlived the cleanup",
        check_bounded_and_clean(&edit(
            &sampler(),
            "kill -KILL -- \"-$sampler_pid\"",
            "kill -KILL \"$sampler_pid\""
        ))
    );
}

mod status_lost {
    use super::*;
    negative_control!(
        qa_sampler_cleanup_bounded_and_ends_the_command_in_hand,
        "a sampler that exits with the cleanup's last status, not the step's",
        expected = "the step's status was lost on the cleanup's KILL path",
        check_bounded_and_clean(&edit(&sampler(), "exit \"$status\"", "exit"))
    );
}
