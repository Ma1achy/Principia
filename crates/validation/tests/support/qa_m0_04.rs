//! The helpers and checks `qa_TASK-M0-04.rs` shares with its controls in `qa_TASK-M0-04_controls.rs`, so each control
//! runs the check it controls, not a copy of it (REQ-VAL-157; R-215). A `tests/*.rs` file is a crate of its own, so
//! each includes this file with `#[path]`; every item here is used by both.

use std::process::{Command, Output};
use validation::gpu::{AdapterInfo, GpuHarness, BACKEND_VAR};
use validation::spawn::Spawn;

pub fn harness() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

/// Index of the first differing word, compared as full 32-bit words (no mask; pitfalls §9), lengths included.
pub fn diff_at(a: &[u32], b: &[u32]) -> Option<usize> {
    if a.len() != b.len() {
        return Some(a.len().min(b.len()));
    }
    (0..a.len()).find(|&i| a[i] != b[i])
}

/// Runs the `qa_child` body `test` in a child process with the given environment changes (R-210).
pub fn child(test: &str, env: &[(&str, Option<&str>)]) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_qa_child"));
    cmd.arg(test.strip_prefix("qa_child_").expect("a qa_child body"));
    for (k, v) in env {
        match v {
            Some(v) => cmd.env(k, v),
            None => cmd.env_remove(k),
        };
    }
    cmd.timed_output().expect("qa_child ran")
}

/// The child's marker line, from `tag` to the end of its line (the child may print before it).
pub fn marker(t: &str, tag: &str) -> String {
    let at = t
        .find(tag)
        .unwrap_or_else(|| panic!("child printed no {tag}: {t}"));
    t[at..].lines().next().unwrap_or_default().to_string()
}

pub fn text(o: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

pub fn open_with(value: Option<&str>) -> String {
    let o = child("qa_child_open_harness", &[(BACKEND_VAR, value)]);
    let t = text(&o);
    assert!(o.status.success(), "child crashed: {t}");
    marker(&t, "QA_OPEN_")
}

/// The backend an unset `PRIN_GPU_BACKEND` selects on this platform, as the variable names it (R-206).
pub fn platform_default() -> &'static str {
    if cfg!(target_os = "macos") {
        "metal"
    } else {
        "vulkan"
    }
}

pub fn first_draw(t: &str) -> String {
    marker(t, "QA_FIRST_DRAW")
}

/// `qa_gpu_harness_identity_round_trips_2_16_words_bit_exact`'s check.
pub fn check_bit_exact(input: &[u32], out: &[u32], on: &AdapterInfo) {
    assert_eq!(
        diff_at(input, out),
        None,
        "identity is not bit-exact on {on}"
    );
}

/// `qa_gpu_harness_round_trips_lengths_off_the_workgroup_size`'s check.
pub fn check_length(len: usize, input: &[u32], out: &[u32]) {
    assert_eq!(diff_at(input, out), None, "identity failed at length {len}");
}

/// `qa_gpu_harness_binds_several_inputs_in_order`'s check.
pub fn check_sub(want: &[u32], got: &[u32]) {
    assert_eq!(
        diff_at(want, got),
        None,
        "a - b is wrong: inputs bound out of order or lost"
    );
}

/// `qa_gpu_harness_sees_extractbits_sign_extension_at_every_width`'s check at a width below 32, word `i`.
pub fn check_overloads_differ(s: u32, u: u32, w: u32, i: usize) {
    assert_ne!(
        s, u,
        "i32 and u32 overloads agree at width {w} on a bit-31 word {i}"
    );
}

/// `qa_gpu_backend_env_governs_harness_new`'s check that `value` is refused.
pub fn check_refused(value: Option<&str>, line: &str) {
    assert!(
        line.starts_with("QA_OPEN_ERR"),
        "{BACKEND_VAR}={value:?} opened a device: {line}"
    );
}

/// `qa_prop_seed_printed_and_rerun_through_the_environment`'s check.
pub fn check_same_draw(first: &str, again: &str) {
    assert_eq!(
        first, again,
        "the printed seed did not reproduce the failing case"
    );
}

/// `qa_prop_shared_config_runs_256_cases_marked_provisional`'s check on the counted cases.
pub fn check_ran_256(n: u32) {
    assert_eq!(n, 256, "prop::run did not run R-203's 256 cases");
}
