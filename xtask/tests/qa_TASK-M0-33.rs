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

/// The words of `command` after `prefix`, without the capture flags and without nextest's `--partition <shard>`, sorted;
/// `None` if it does not start so. A shard (R-336, R-366) selects which of the step's tests run, not its packages,
/// features or filter: the doctest step beside a sharded nextest step covers the same packages, unsharded.
fn args_after(command: &str, prefix: &str) -> Option<Vec<String>> {
    let mut words = words_after(command, prefix)?;
    words.sort();
    Some(words)
}

/// `args_after`'s words in the command's own order, so each option stays beside its value.
fn words_after(command: &str, prefix: &str) -> Option<Vec<String>> {
    let rest = command.strip_prefix(prefix)?;
    // `${{ matrix.shard }}` is one word of the command, not three.
    let rest = rest.replace("${{ matrix.shard }}", "${{matrix.shard}}");
    let mut words: Vec<String> = Vec::new();
    let mut partition = false;
    for w in rest.split_whitespace() {
        if std::mem::take(&mut partition) {
            continue;
        }
        if w == "--partition" {
            partition = true;
        } else if !matches!(w, "--no-capture" | "--nocapture" | "--") {
            words.push(w.to_owned());
        }
    }
    Some(words)
}

/// The options of `cargo nextest run` that run a reused build (an archive), not choose its tests, each with its value;
/// and the reuse flags, which take none (R-372).
const REUSE_OPTIONS: &[&str] = &[
    "--archive-file",
    "--archive-format",
    "--extract-to",
    "--cargo-metadata",
    "--workspace-remap",
    "--binaries-metadata",
    "--target-dir-remap",
    "--build-dir-remap",
];
const REUSE_FLAGS: &[&str] = &["--extract-overwrite", "--persist-extract-tempdir"];

/// `words` without the options of `options` (each with its value) and the flags of `flags`, and the value its
/// `--archive-file` had, if any.
fn strip(words: &[String], options: &[&str], flags: &[&str]) -> (Vec<String>, Option<String>) {
    let (mut kept, mut archive) = (Vec::new(), None);
    let mut it = words.iter();
    while let Some(w) = it.next() {
        if options.contains(&w.as_str()) {
            let value = it.next().cloned();
            if w == "--archive-file" {
                archive = value;
            }
        } else if !flags.contains(&w.as_str()) {
            kept.push(w.clone());
        }
    }
    (kept, archive)
}

/// The `cargo nextest archive` steps of `jobs`: each archive file, with the sorted arguments it is built with (without
/// those naming the archive).
fn archives(jobs: &[(String, Vec<String>)]) -> Vec<(String, Vec<String>)> {
    let mut found = Vec::new();
    for (job, runs) in jobs {
        for run in runs {
            let Some(words) = words_after(run, "cargo nextest archive") else {
                continue;
            };
            let (mut args, path) = strip(
                &words,
                &["--archive-file", "--archive-format", "--zstd-level"],
                &[],
            );
            let path =
                path.unwrap_or_else(|| panic!("job {job} runs `{run}` with no --archive-file"));
            args.sort();
            found.push((path, args));
        }
    }
    found
}

/// In `workflow`: the workspace suite runs through nextest, its whole build archived by
/// `cargo nextest archive --workspace` and its shards run from that archive (R-372); every `cargo test` step is a
/// doctest step; and each `cargo nextest run <args>` step has, in its own job, `cargo test <args> --doc`, where a step
/// running from an archive has for `<args>` those the archive was built with.
fn check_nextest_and_doctest_steps(workflow: &str) {
    let jobs = job_runs(workflow);
    let archives = archives(&jobs);
    let mut nextest_steps = 0;
    for (job, runs) in &jobs {
        for run in runs {
            if let Some(args) = args_after(run, "cargo test") {
                assert!(
                    args.iter().any(|a| a == "--doc"),
                    "job {job} runs tests through `cargo test`, not nextest: `{run}`"
                );
            }
            let Some(words) = words_after(run, "cargo nextest run") else {
                continue;
            };
            let (mut args, archive) = strip(&words, REUSE_OPTIONS, REUSE_FLAGS);
            args.sort();
            if let Some(archive) = archive {
                let built: Vec<&Vec<String>> = archives
                    .iter()
                    .filter(|(path, _)| *path == archive)
                    .map(|(_, a)| a)
                    .collect();
                assert_eq!(
                    built.len(),
                    1,
                    "job {job} runs `{run}` from `{archive}`, which {} steps build",
                    built.len()
                );
                args.extend(built[0].iter().cloned());
                args.sort();
            }
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
    assert!(
        jobs.iter().any(|(_, runs)| runs.iter().any(|r| r
            == "cargo nextest run --archive-file $RUNNER_TEMP/nextest-ci.tar.zst --extract-to . --extract-overwrite \
                --partition hash:${{ matrix.shard }}/4"))
            && archives.iter().any(|(path, args)| path == "$RUNNER_TEMP/nextest-ci.tar.zst"
                && args == &["--workspace".to_owned()]),
        "no CI job runs the workspace suite through nextest, from an archive of `cargo nextest archive --workspace`"
    );
}

#[test]
fn qa_m0_33_each_nextest_step_has_its_doctest_step() {
    check_nextest_and_doctest_steps(&ci_workflow());
}

/// The archive pairing (R-372): a nextest step that runs from an archive no step builds has no arguments to pair.
#[test]
fn qa_m0_33_each_archive_run_has_its_archive_step() {
    check_nextest_and_doctest_steps(&ci_workflow());
}

negative_control!(
    qa_m0_33_each_archive_run_has_its_archive_step,
    "CI's workflow with the shards running from an archive no step builds, required to pair each archive run with the \
     step building its archive",
    expected = "which 0 steps build",
    check_nextest_and_doctest_steps(&ci_workflow().replace(
        "run: cargo nextest run --archive-file $RUNNER_TEMP/nextest-ci.tar.zst",
        "run: cargo nextest run --archive-file $RUNNER_TEMP/elsewhere.tar.zst"
    ))
);

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
