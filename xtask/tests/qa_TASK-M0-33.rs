//! QA tests for TASK-M0-33, written from REQ-VAL-165 and R-231: "The test suite must run through `cargo nextest run`
//! in CI and in the documented local run (doctests through `cargo test --doc`)"; verify: "the CI test steps use nextest
//! plus `cargo test --doc`" (both feature sets). R-231: "No test is dropped: doctests still run through
//! `cargo test --doc`".
//!
//! The workspace has no doctest today, so a check that compares test lists cannot see a doctest step go missing: this
//! file checks the steps themselves. Each nextest step has, in its job, a `cargo test --doc` step over the same
//! packages, features and filter; no step runs tests through bare `cargo test`; the workspace suite runs through
//! nextest. Each test registers a negative control (R-176, R-212).

use std::path::Path;

use validation::negative_control;

fn ci_workflow() -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join(".github/workflows/ci.yml"),
    )
    .unwrap()
}

/// The `run:` commands of each job of `workflow`, by job name (a key two spaces in under `jobs:`).
fn job_runs(workflow: &str) -> Vec<(String, Vec<String>)> {
    let mut jobs: Vec<(String, Vec<String>)> = Vec::new();
    let mut in_jobs = false;
    for line in workflow.lines() {
        if !line.is_empty() && !line.starts_with(' ') && !line.starts_with('#') {
            in_jobs = line.trim_end() == "jobs:";
            continue;
        }
        if !in_jobs {
            continue;
        }
        let body = line.trim_start();
        let indent = line.len() - body.len();
        if indent == 2 && body.ends_with(':') && !body.starts_with('#') {
            jobs.push((body.trim_end_matches(':').to_owned(), Vec::new()));
        } else if let Some(run) = body
            .strip_prefix("run: ")
            .or_else(|| body.strip_prefix("- run: "))
        {
            if let Some((_, runs)) = jobs.last_mut() {
                runs.push(run.trim().to_owned());
            }
        }
    }
    jobs
}

/// The words of `command` after `prefix`, without the capture flags, sorted; `None` if it does not start so.
fn args_after(command: &str, prefix: &str) -> Option<Vec<String>> {
    let rest = command.strip_prefix(prefix)?;
    let mut words: Vec<String> = rest
        .split_whitespace()
        .filter(|w| !matches!(*w, "--no-capture" | "--nocapture" | "--"))
        .map(str::to_owned)
        .collect();
    words.sort();
    Some(words)
}

/// In `workflow`: the workspace suite runs through `cargo nextest run --workspace`; every `cargo test` step is a
/// doctest step; and each `cargo nextest run <args>` step has, in its own job, `cargo test <args> --doc`.
fn check_nextest_and_doctest_steps(workflow: &str) {
    let jobs = job_runs(workflow);
    assert!(
        jobs.iter()
            .any(|(_, runs)| runs.iter().any(|r| r == "cargo nextest run --workspace")),
        "no CI job runs the workspace suite through `cargo nextest run --workspace`"
    );
    let mut nextest_steps = 0;
    for (job, runs) in &jobs {
        for run in runs {
            if let Some(args) = args_after(run, "cargo test") {
                assert!(
                    args.iter().any(|a| a == "--doc"),
                    "job {job} runs tests through `cargo test`, not nextest: `{run}`"
                );
            }
            let Some(args) = args_after(run, "cargo nextest run") else {
                continue;
            };
            nextest_steps += 1;
            let mut doc = args.clone();
            doc.push("--doc".to_owned());
            doc.sort();
            assert!(
                runs.iter()
                    .any(|r| args_after(r, "cargo test").as_ref() == Some(&doc)),
                "job {job} runs `{run}` with no `cargo test --doc` over the same packages and features"
            );
        }
    }
    assert!(nextest_steps >= 3, "CI has {nextest_steps} nextest steps");
}

#[test]
fn qa_m0_33_each_nextest_step_has_its_doctest_step() {
    check_nextest_and_doctest_steps(&ci_workflow());
}

negative_control!(
    qa_m0_33_each_nextest_step_has_its_doctest_step,
    "CI's workflow with its `cargo test --doc` steps removed, required to run doctests beside nextest",
    expected = "with no `cargo test --doc` over the same packages and features",
    check_nextest_and_doctest_steps(
        &ci_workflow()
            .lines()
            .filter(|l| !(l.trim_start().starts_with("run: cargo test") && l.contains("--doc")))
            .collect::<Vec<_>>()
            .join("\n")
    )
);

/// The control of the bare-`cargo test` assertion: the doctest steps kept, and a workspace `cargo test` step added.
#[cfg(feature = "controls")]
fn with_bare_cargo_test(workflow: &str) -> String {
    workflow.replace(
        "        run: cargo test --workspace --doc\n",
        "        run: cargo test --workspace --doc\n      - run: cargo test --workspace\n",
    )
}

#[test]
fn qa_m0_33_no_ci_step_runs_tests_through_bare_cargo_test() {
    let workflow = ci_workflow();
    check_nextest_and_doctest_steps(&workflow);
    assert!(
        workflow.contains("        run: cargo test --workspace --doc\n"),
        "the workspace doctest step moved"
    );
}

negative_control!(
    qa_m0_33_no_ci_step_runs_tests_through_bare_cargo_test,
    "CI's workflow with a `cargo test --workspace` step added, required to run tests only through nextest",
    expected = "runs tests through `cargo test`, not nextest",
    check_nextest_and_doctest_steps(&with_bare_cargo_test(&ci_workflow()))
);
