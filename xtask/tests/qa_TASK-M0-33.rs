//! QA tests for TASK-M0-33, written from REQ-VAL-165 and R-231: "The test suite must run through `cargo nextest run`
//! in CI and in the documented local run (doctests through `cargo test --doc`)"; verify: "the CI test steps use nextest
//! plus `cargo test --doc`" (both feature sets). R-231: "No test is dropped: doctests still run through
//! `cargo test --doc`".
//!
//! The workspace has no doctest today, so a check that compares test lists cannot see a doctest step go missing: this
//! file checks the steps themselves. Each nextest step has, in its job, a `cargo test --doc` step over the same
//! packages, features and filter; no step runs tests through bare `cargo test`; the workspace suite runs through
//! nextest. Each test registers a negative control (R-176, R-212).
//!
//! R-677: a step selecting its test binaries with `--test` (tier (b)'s readers) has no doctest step beside it, since
//! `cargo test --doc` refuses `--test` and a `--test` selection runs no doctest; so such a step pairs with no doctest
//! step of its own, and instead each package it names must have its doctests run by some `cargo test --doc` step of the
//! workflow (that package's, or the workspace's), so none is dropped (R-231). That is the one change here, R-677's
//! exception to R-290.
//!
//! `--lib`, as `--test`: R-677 (decisions.md, "Tier (b)'s steps have no doctest step"): "Each selects its test
//! binaries with `--test`, which `cargo test --doc` refuses beside it (\"can't mix --doc with other target selecting
//! options\"); a `--test` selection runs no doctest under `cargo test` either, so none is dropped (R-231), and every
//! doctest runs on `main` and on every code head." `--lib` is a target selection of the same kind: cargo refuses
//! `cargo test -p gui --lib --doc` with the same words, and a `--lib` selection runs no doctest. The smoke sets'
//! `cargo nextest run -p gui --lib` (the lattice tests REQ-GUI-412 makes block on lavapipe and Metal) therefore pairs
//! with no doctest step of its own (applied per R-369 on PR #457, R-290's exception: R-909 and REQ-GUI-412 force it).
//! What stands in for the pairing is held per tier, not workflow-wide: on every tier the selecting step's job runs on
//! (the pull-request tier, the push tier, or both, read from the job's `if:` and, where that does not say `always()`,
//! the jobs it needs), some job of that tier runs a whole `cargo test --doc` step (no test-name filter) covering each
//! package the step names: `--workspace`, `-p <package>`, or the changed crates' `$PACKAGES` list, which covers a
//! workspace package with a library (`cargo test $PACKAGES --doc` over the changed crates and their reverse
//! dependencies). A `--lib` or `--test` step whose package has no such step on one of its tiers fails.

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

/// Each job of `workflow` with the tiers it runs on, `(pull_request, push)`: a job-level `if:` naming
/// `github.event_name == 'pull_request'` leaves out the push tier and one naming `!= 'pull_request'` the pull-request
/// tier; a job whose `if:` does not say `always()` runs only where every job it `needs:` runs.
fn job_tiers(workflow: &str) -> Vec<(String, bool, bool)> {
    let mut jobs: Vec<(String, Option<String>, Vec<String>)> = Vec::new();
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
            jobs.push((body.trim_end_matches(':').to_owned(), None, Vec::new()));
        } else if indent == 4 {
            let Some((_, cond, needs)) = jobs.last_mut() else {
                continue;
            };
            if let Some(c) = body.strip_prefix("if:") {
                *cond = Some(c.trim().to_owned());
            } else if let Some(n) = body.strip_prefix("needs:") {
                *needs = n
                    .trim()
                    .trim_start_matches('[')
                    .trim_end_matches(']')
                    .split(',')
                    .map(|w| w.trim().to_owned())
                    .filter(|w| !w.is_empty())
                    .collect();
            }
        }
    }
    let mut out: Vec<(String, bool, bool)> = Vec::new();
    for (name, cond, needs) in jobs {
        let cond = cond.unwrap_or_default();
        let (mut pr, mut push) = (true, true);
        if cond.contains("github.event_name == 'pull_request'") {
            push = false;
        }
        if cond.contains("github.event_name != 'pull_request'") {
            pr = false;
        }
        if !cond.contains("always()") {
            for need in &needs {
                if let Some((_, p, q)) = out.iter().find(|(n, _, _)| n == need) {
                    pr &= *p;
                    push &= *q;
                }
            }
        }
        out.push((name, pr, push));
    }
    out
}

/// Whether workspace package `pkg` has a library, so has doctests a `cargo test --doc` over it can run.
fn has_library(pkg: &str) -> bool {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let dir = if pkg == "xtask" {
        root.join("xtask")
    } else {
        root.join("crates").join(pkg)
    };
    dir.join("src/lib.rs").is_file()
}

/// Whether `run` is a whole `cargo test --doc` step (no test-name filter) over `pkg`'s doctests: by `--workspace`,
/// `-p <pkg>`, or the changed crates' `$PACKAGES` when `pkg` has a library.
fn doc_step_covers(run: &str, pkg: &str) -> bool {
    let Some(words) = words_after(run, "cargo test") else {
        return false;
    };
    if !words.iter().any(|a| a == "--doc") {
        return false;
    }
    let (mut covers, mut it) = (false, words.iter());
    while let Some(w) = it.next() {
        match w.as_str() {
            "-p" | "--package" => covers |= it.next().is_some_and(|p| p == pkg),
            "--features" | "-F" => {
                it.next();
            }
            "--workspace" => covers = true,
            "$PACKAGES" => covers |= has_library(pkg),
            w if w.starts_with('-') => {}
            // A test-name filter: the step runs some of the package's doctests, not all.
            _ => return false,
        }
    }
    covers
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
    let tiers = job_tiers(workflow);
    let tier_of = |job: &str| -> (bool, bool) {
        tiers
            .iter()
            .find(|(n, _, _)| n == job)
            .map_or((true, true), |(_, p, q)| (*p, *q))
    };
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
            if args.iter().any(|a| a == "--test" || a == "--lib") {
                // R-677: a `--test` (or `--lib`) selection runs no doctest; its packages' doctests run in a whole
                // `--doc` step elsewhere, on each tier its job runs on.
                let packages: Vec<&String> = words
                    .windows(2)
                    .filter(|w| w[0] == "-p" || w[0] == "--package")
                    .map(|w| &w[1])
                    .collect();
                assert!(
                    !packages.is_empty(),
                    "job {job} runs `{run}`, a `--test` or `--lib` selection naming no package"
                );
                let (pr, push) = tier_of(job);
                for (tier, runs_on) in [("pull_request", pr), ("push", push)] {
                    if !runs_on {
                        continue;
                    }
                    for pkg in &packages {
                        let covered = jobs.iter().any(|(j, rs)| {
                            let (p, q) = tier_of(j);
                            (if tier == "push" { q } else { p })
                                && rs.iter().any(|r| doc_step_covers(r, pkg))
                        });
                        assert!(
                            covered,
                            "job {job} runs `{run}`, whose package {pkg}'s doctests no `cargo test --doc` step runs \
                             on the {tier} tier"
                        );
                    }
                }
                continue;
            }
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

mod test_selection_doctests {
    use super::*;
    negative_control!(
        qa_m0_33_each_nextest_step_has_its_doctest_step,
        "CI's workflow with the workspace's doctest steps gone and, first among its jobs, a `--test` selection (R-677) \
         in a package no other `cargo test --doc` step covers",
        expected = "whose package qa_nodoc's doctests no `cargo test --doc` step runs",
        check_nextest_and_doctest_steps(&{
            let ci = ci_workflow();
            assert!(ci.contains("\njobs:\n"));
            ci.replacen(
                "\njobs:\n",
                "\njobs:\n  qa_first:\n    steps:\n      - run: cargo nextest run -p qa_nodoc --test qa\n",
                1,
            )
            .lines()
            .filter(|l| !(l.trim_start().starts_with("run: cargo test --workspace") && l.contains("--doc")))
            .collect::<Vec<_>>()
            .join("\n")
        })
    );
}

/// A `--lib` selection (the smoke sets' gui lattice step, R-677 read for `--lib`) in a package no `cargo test --doc`
/// step covers, on a job of both tiers: the changed crates' `$PACKAGES` covers only a workspace package with a library.
mod lib_selection_doctests {
    use super::*;
    negative_control!(
        qa_m0_33_each_nextest_step_has_its_doctest_step,
        "CI's workflow with, first among its jobs, a `--lib` selection in a package no `cargo test --doc` step covers",
        expected = "whose package qa_nodoc's doctests no `cargo test --doc` step runs on the pull_request tier",
        check_nextest_and_doctest_steps(&{
            let ci = ci_workflow();
            assert!(ci.contains("\njobs:\n"));
            ci.replacen(
                "\njobs:\n",
                "\njobs:\n  qa_first:\n    steps:\n      - run: cargo nextest run -p qa_nodoc --lib --no-capture\n",
                1,
            )
        })
    );
}

/// The tiers kept apart: a `--lib` selection on both tiers whose package's one whole doctest step is in a job that
/// needs the push-only `ci-archive`, so runs on the push tier only (no `always()`), has none on the pull-request tier.
mod pull_request_tier_doctests {
    use super::*;
    negative_control!(
        qa_m0_33_each_nextest_step_has_its_doctest_step,
        "CI's workflow with a `--lib` selection on both tiers whose doctests run only in a job needing `ci-archive`",
        expected = "whose package qa_nodoc's doctests no `cargo test --doc` step runs on the pull_request tier",
        check_nextest_and_doctest_steps(&{
            let mut ci = ci_workflow();
            assert!(ci.contains("\n  ci-archive:\n") && ci.ends_with('\n'));
            ci.push_str(
                "  qa_doc:\n    needs: ci-archive\n    steps:\n      - run: cargo test -p qa_nodoc --doc\n  \
                 qa_sel:\n    needs: changes\n    if: always()\n    steps:\n      \
                 - run: cargo nextest run -p qa_nodoc --lib --no-capture\n",
            );
            ci
        })
    );
}

/// The push tier's: without the workspace's `cargo test --workspace … --doc` steps, gui's `--lib` lattice step has none
/// there; `cargo test -p gui lattice_ --doc` runs only the doctests its filter names, so it does not count.
mod push_tier_doctests {
    use super::*;
    negative_control!(
        qa_m0_33_each_nextest_step_has_its_doctest_step,
        "CI's workflow without its `cargo test --workspace … --doc` steps, gui's left only a filtered one",
        expected = "whose package gui's doctests no `cargo test --doc` step runs on the push tier",
        check_nextest_and_doctest_steps(
            &ci_workflow()
                .lines()
                .filter(|l| !(l.trim_start().starts_with("run: cargo test --workspace") && l.contains("--doc")))
                .collect::<Vec<_>>()
                .join("\n")
        )
    );
}

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
