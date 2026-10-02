//! qa's tests for R-335 in TASK-M0-14, written from REQ-VAL-166 ("CI runs every control through `cargo xtask ci` in a
//! job of its own, in parallel with the tests, failing on any finding (R-235)") as R-335 reads it: the controls job
//! "still cannot be skipped or pass with a finding", where only the job's own `if:` (indent 4) can skip it; a step's
//! `if:`, as R-326's conditional cache save in that job, does not.
//!
//! `qa_TASK-M0-22_r235.rs`'s `check_controls_job_beside_the_tests` is narrowed to the job's `if:` (R-335); R-335 keeps
//! the rest of that file as it is, so the controls showing that a job-level `if:` or `continue-on-error:` still fails
//! are here. Each test registers a negative control (R-176).

use std::fs;
use std::path::Path;

use validation::negative_control;

/// CI's per-push workflow, as checked in.
fn the_workflow() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../.github/workflows/ci.yml");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

fn indent(l: &str) -> usize {
    l.len() - l.trim_start().len()
}

/// The lines of the job (two spaces in under `jobs:`) whose steps run bare `cargo xtask ci`, as (job id, lines).
fn controls_job(workflow: &str) -> (String, Vec<String>) {
    let mut jobs: Vec<(String, Vec<String>)> = Vec::new();
    let mut in_jobs = false;
    for l in workflow.lines() {
        let t = l.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if indent(l) == 0 {
            in_jobs = t == "jobs:";
            continue;
        }
        if !in_jobs {
            continue;
        }
        if indent(l) == 2 && t.ends_with(':') {
            jobs.push((t.trim_end_matches(':').to_owned(), Vec::new()));
        } else if let Some((_, lines)) = jobs.last_mut() {
            lines.push(l.to_owned());
        }
    }
    let found: Vec<(String, Vec<String>)> = jobs
        .into_iter()
        .filter(|(_, lines)| {
            lines.iter().any(|l| {
                l.trim().trim_start_matches("- ").trim()
                    == "run: cargo xtask ci --partition ${{ matrix.shard }}/4"
            })
        })
        .collect();
    assert_eq!(
        found.len(),
        1,
        "no single job runs bare `cargo xtask ci` (every control)"
    );
    found.into_iter().next().unwrap()
}

/// The controls job cannot be skipped (no `if:` among its own keys, indent 4) or pass with a finding (no
/// `continue-on-error:` at any indent, as in `qa_TASK-M0-22_r235.rs`); a step-level `if:` is allowed.
fn check_controls_job_runs_and_fails(workflow: &str) {
    let (id, lines) = controls_job(workflow);
    for l in &lines {
        let t = l.trim().trim_start_matches("- ");
        assert!(
            !(indent(l) == 4 && t.starts_with("if:")),
            "the controls job `{id}` can be skipped: its own `{t}`"
        );
        assert!(
            !t.starts_with("continue-on-error:"),
            "the controls job `{id}` can pass with a finding: `{t}`"
        );
    }
}

/// The checked-in controls job has a step-level `if:` (R-326's save), so the check above reads it and lets it pass.
fn check_a_step_if_is_read(workflow: &str) {
    let (id, lines) = controls_job(workflow);
    assert!(
        lines
            .iter()
            .any(|l| indent(l) > 4 && l.trim().trim_start_matches("- ").starts_with("if:")),
        "the controls job `{id}` has no step-level `if:`: R-326's conditional save is gone or unread"
    );
    check_controls_job_runs_and_fails(workflow);
}

#[test]
fn qa_r335_the_controls_job_cannot_be_skipped() {
    check_controls_job_runs_and_fails(&the_workflow());
}

negative_control!(
    qa_r335_the_controls_job_cannot_be_skipped,
    "the checked-in workflow with a job-level `if:` on the controls job",
    expected = "can be skipped: its own `if: github.event_name == 'push'`",
    check_controls_job_runs_and_fails(&the_workflow().replacen(
        "  xtask-ci:\n    runs-on: ubuntu-latest\n",
        "  xtask-ci:\n    if: github.event_name == 'push'\n    runs-on: ubuntu-latest\n",
        1
    ))
);

#[test]
fn qa_r335_the_controls_job_cannot_pass_with_a_finding() {
    check_controls_job_runs_and_fails(&the_workflow());
}

negative_control!(
    qa_r335_the_controls_job_cannot_pass_with_a_finding,
    "the checked-in workflow with `continue-on-error: true` on the `cargo xtask ci` step",
    expected = "can pass with a finding",
    check_controls_job_runs_and_fails(&the_workflow().replacen(
        "        run: cargo xtask ci --partition ${{ matrix.shard }}/4\n",
        "        continue-on-error: true\n        run: cargo xtask ci --partition ${{ matrix.shard }}/4\n",
        1
    ))
);

#[test]
fn qa_r335_a_step_level_if_is_allowed() {
    check_a_step_if_is_read(&the_workflow());
}

negative_control!(
    qa_r335_a_step_level_if_is_allowed,
    "the checked-in workflow with every step-level `if:` in the controls job removed",
    expected = "has no step-level `if:`",
    check_a_step_if_is_read(&the_workflow().replace(
        "        if: github.event_name == 'push' && github.ref == 'refs/heads/main'\n",
        ""
    ))
);
