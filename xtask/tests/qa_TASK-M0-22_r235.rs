//! qa's tests for TASK-M0-22's R-235 round, written from REQ-VAL-166: "`qa_cargo_xtask_alias_runs_deps` must run
//! `cargo xtask ci` in a listing-only form that runs no control, while CI runs every control through `cargo xtask ci`
//! in a job of its own, in parallel with the tests, failing on any finding (R-235)."
//!
//! - CI's workflow (`.github/workflows/ci.yml`) runs bare `cargo xtask ci` (every control, not `--list`) in a job
//!   other than the one running `cargo nextest run --workspace` (R-231); neither job waits on the other (`needs:`), and the controls
//!   job is neither skipped (`if:`) nor allowed to fail (`continue-on-error`). Its GPU controls run where the GPU
//!   tests do: the same `PRIN_GPU_BACKEND` and the lavapipe install (TASK-M0-22 § Notes, "A GPU test's control runs
//!   where the test does").
//! - The listing-only form, `xtask ci --list`, still fails on what a listing shows (a test with no control), so the
//!   alias test's child keeps its guard while running no control. It runs against a stand-in `cargo` (`CARGO`) that
//!   answers `metadata` and listings from canned text and logs each call.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use validation::spawn::Spawn;

/// CI's per-push workflow, as checked in.
fn the_workflow() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../.github/workflows/ci.yml");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// The jobs of `workflow`, each as (name, its lines), from the two-space-indented keys under `jobs:`.
fn jobs(workflow: &str) -> Vec<(String, Vec<String>)> {
    let mut found: Vec<(String, Vec<String>)> = Vec::new();
    let mut in_jobs = false;
    for line in workflow.lines() {
        if !line.starts_with(' ') && !line.trim().is_empty() && !line.starts_with('#') {
            in_jobs = line.trim_end() == "jobs:";
            continue;
        }
        if !in_jobs {
            continue;
        }
        let is_key = line.starts_with("  ")
            && !line.starts_with("   ")
            && !line.trim_start().starts_with('#')
            && line.trim_end().ends_with(':');
        if is_key {
            found.push((line.trim().trim_end_matches(':').to_owned(), Vec::new()));
        } else if let Some((_, lines)) = found.last_mut() {
            lines.push(line.to_owned());
        }
    }
    found
}

/// The `run:` commands of a job's lines, trimmed.
fn runs(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .map(|l| l.trim().trim_start_matches("- ").trim())
        .filter_map(|l| l.strip_prefix("run:"))
        .map(|c| c.trim().to_owned())
        .collect()
}

/// Whether one of `lines` (a job's), trimmed, starts with `key`.
fn has_key(lines: &[String], key: &str) -> bool {
    lines
        .iter()
        .any(|l| l.trim().trim_start_matches("- ").starts_with(key))
}

/// REQ-VAL-166's CI half, on `workflow`'s text.
fn check_controls_job_beside_the_tests(workflow: &str) {
    let jobs = jobs(workflow);
    assert!(
        workflow
            .lines()
            .any(|l| l.trim() == "push:" || l.trim().starts_with("on: [push")),
        "the workflow does not run on every push"
    );
    let tests: Vec<&(String, Vec<String>)> = jobs
        .iter()
        .filter(|(_, lines)| {
            runs(lines)
                .iter()
                .any(|r| r == "cargo nextest run --workspace")
        })
        .collect();
    assert_eq!(
        tests.len(),
        1,
        "no single job runs `cargo nextest run --workspace`"
    );
    let (test_job, test_lines) = tests[0];
    let controls: Vec<&(String, Vec<String>)> = jobs
        .iter()
        .filter(|(_, lines)| runs(lines).iter().any(|r| r == "cargo xtask ci"))
        .collect();
    assert_eq!(
        controls.len(),
        1,
        "no single job runs bare `cargo xtask ci` (every control)"
    );
    let (controls_job, controls_lines) = controls[0];
    assert_ne!(
        controls_job, test_job,
        "`cargo xtask ci` runs in the tests' job, not in a job of its own"
    );
    assert!(
        !jobs.iter().any(|(_, lines)| runs(lines)
            .iter()
            .any(|r| r.starts_with("cargo xtask ci --list"))),
        "CI runs the listing-only `cargo xtask ci --list`, which runs no control"
    );
    assert!(
        !has_key(controls_lines, "needs:") && !has_key(test_lines, "needs:"),
        "the controls job and the tests' job wait on one another, not beside each other"
    );
    assert!(
        !has_key(controls_lines, "if:") && !has_key(controls_lines, "continue-on-error:"),
        "the controls job can be skipped or pass with a finding"
    );
    let backend = |lines: &[String]| -> Vec<String> {
        lines
            .iter()
            .map(|l| l.trim().to_owned())
            .filter(|l| l.starts_with("PRIN_GPU_BACKEND:"))
            .collect()
    };
    assert!(
        !backend(controls_lines).is_empty() && backend(controls_lines) == backend(test_lines),
        "the controls job does not set the tests' PRIN_GPU_BACKEND"
    );
    assert!(
        runs(controls_lines)
            .iter()
            .any(|r| r.contains("mesa-vulkan-drivers")),
        "the controls job does not install lavapipe, where the GPU tests run"
    );
}

#[test]
fn qa_r235_ci_runs_every_control_in_a_job_beside_the_tests() {
    check_controls_job_beside_the_tests(&the_workflow());
}

validation::negative_control!(
    qa_r235_ci_runs_every_control_in_a_job_beside_the_tests,
    "the pre-R-235 workflow, with the `cargo xtask ci` step in the tests' job",
    expected = "`cargo xtask ci` runs in the tests' job",
    check_controls_job_beside_the_tests(
        "on:\n  push:\njobs:\n  ci:\n    runs-on: ubuntu-latest\n    steps:\n      - name: Install Mesa (lavapipe)\n        run: sudo apt-get install -y mesa-vulkan-drivers\n      - name: cargo nextest run --workspace\n        env:\n          PRIN_GPU_BACKEND: vulkan\n        run: cargo nextest run --workspace\n      - name: cargo xtask ci\n        env:\n          PRIN_GPU_BACKEND: vulkan\n        run: cargo xtask ci\n"
    )
);

#[test]
fn qa_r235_controls_job_does_not_wait_for_the_tests() {
    check_controls_job_beside_the_tests(&the_workflow());
}

validation::negative_control!(
    qa_r235_controls_job_does_not_wait_for_the_tests,
    "the checked-in workflow with `needs: ci` added to the job running `cargo xtask ci`",
    expected = "wait on one another",
    check_controls_job_beside_the_tests(&the_workflow().replacen(
        "  xtask-ci:\n",
        "  xtask-ci:\n    needs: ci\n",
        1
    ))
);

#[test]
fn qa_r235_ci_does_not_run_the_listing_only_form() {
    check_controls_job_beside_the_tests(&the_workflow());
}

validation::negative_control!(
    qa_r235_ci_does_not_run_the_listing_only_form,
    "the checked-in workflow with a further step running `cargo xtask ci --list`",
    expected = "CI runs the listing-only `cargo xtask ci --list`",
    check_controls_job_beside_the_tests(&the_workflow().replacen(
        "        run: cargo xtask ci\n",
        "        run: cargo xtask ci\n      - name: listing\n        run: cargo xtask ci --list\n",
        1
    ))
);

#[test]
fn qa_r235_controls_job_has_the_tests_gpu_backend() {
    check_controls_job_beside_the_tests(&the_workflow());
}

validation::negative_control!(
    qa_r235_controls_job_has_the_tests_gpu_backend,
    "the checked-in workflow with the controls job's PRIN_GPU_BACKEND removed",
    expected = "the controls job does not set the tests' PRIN_GPU_BACKEND",
    check_controls_job_beside_the_tests(&the_workflow().replacen(
        "  xtask-ci:\n    runs-on: ubuntu-latest\n    env:\n      PRIN_GPU_BACKEND: vulkan\n",
        "  xtask-ci:\n    runs-on: ubuntu-latest\n",
        1
    ))
);

/// Runs `xtask <args>` with a stand-in `CARGO` in a directory of its own named `case`, the stand-in listing `listed`
/// for every `-- --list` call and answering any other test run as a control that made its test fail. Returns whether
/// xtask passed, its output, and each argument list the stand-in was called with.
fn run_listing(case: &str, args: &[&str], listed: &str) -> (bool, String, Vec<String>) {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("qa_r235_{case}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("listed.txt"), listed).unwrap();
    let cargo = dir.join("cargo");
    validation::spawn::write_executable(
        &cargo,
        format!(
            r#"#!/bin/sh
echo "$*" >> '{dir}/calls.log'
if [ "$1" = metadata ]; then
  echo '{{"packages":[{{"name":"qa_r235_fake","features":{{"controls":[]}},"targets":[{{"doctest":false}}]}}]}}'
  exit 0
fi
for a in "$@"; do
  if [ "$a" = --list ]; then cat '{dir}/listed.txt'; exit 0; fi
done
echo 'test qa_r235_t::paired::negative_control - should panic ... ok'
"#,
            dir = dir.display()
        ),
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(args)
        .env("CARGO", &cargo)
        .timed_output()
        .expect("run xtask");
    let calls = fs::read_to_string(dir.join("calls.log"))
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect();
    let out = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (output.status.success(), out, calls)
}

/// A listing of one test with its control, and one test (`alone`) with none.
const LISTED_MISSING: &str =
    "qa_r235_t::paired: test\nqa_r235_t::paired::negative_control: test\nqa_r235_t::alone: test\n";

/// `xtask ci --list` on a listing with a test lacking its control: it fails naming that test, and ran no control.
fn check_list_fails_on_a_missing_control((ok, out, calls): (bool, String, Vec<String>)) {
    let ran: Vec<&String> = calls
        .iter()
        .filter(|c| !c.starts_with("metadata") && !c.split(' ').any(|a| a == "--list"))
        .collect();
    assert!(
        ran.is_empty(),
        "xtask ci --list ran a control: {ran:?}\n{out}"
    );
    assert!(
        !ok,
        "xtask ci --list passed with a test lacking its control\n{out}"
    );
    assert!(
        out.contains("alone"),
        "xtask ci --list did not name the test lacking its control\n{out}"
    );
}

#[test]
fn qa_r235_ci_list_fails_on_a_test_without_control() {
    check_list_fails_on_a_missing_control(run_listing(
        "missing",
        &["ci", "--list"],
        LISTED_MISSING,
    ));
}

validation::negative_control!(
    qa_r235_ci_list_fails_on_a_test_without_control,
    "every listed test with its control, where the check requires ci --list to fail",
    expected = "xtask ci --list passed with a test lacking its control",
    check_list_fails_on_a_missing_control(run_listing(
        "ctl_missing",
        &["ci", "--list"],
        "qa_r235_t::paired: test\nqa_r235_t::paired::negative_control: test\n"
    ))
);
