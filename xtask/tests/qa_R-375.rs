//! qa's tests for R-375, written from the ruling and REQ-SYS-065, not from the workflows:
//! - R-375: "pin CI runners to ubuntu-24.04, not ubuntu-latest. Moving to Ubuntu 26 is a deliberate later step";
//!   applied: "Every job in `.github/workflows/` that ran on `ubuntu-latest` runs on `ubuntu-24.04` ... The `macos-15`
//!   jobs were already pinned and stay as they are."
//! - REQ-SYS-065: GPU CI runs on "macos-15 (Apple silicon, paravirtual Metal) for the Metal correctness suites and
//!   ubuntu-24.04 (pinned, R-375) with Mesa lavapipe (Vulkan)".
//!
//! So every `runs-on:` in every workflow names one of the two pinned images, never a moving label (`ubuntu-latest`,
//! `macos-latest`), another image, or an expression this check cannot read; and the lavapipe job of the per-commit
//! workflow is on `ubuntu-24.04`. Each test registers a negative control (R-176) that trips its assertion by message.
// The file name `qa_R-375` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::fs;
use std::path::{Path, PathBuf};

use validation::negative_control;

/// The two pinned runner images R-375 allows.
const PINNED: [&str; 2] = ["ubuntu-24.04", "macos-15"];

fn workflows_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../.github/workflows")
}

/// Every workflow, as (file name, text), sorted by name.
fn workflows() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = fs::read_dir(workflows_dir())
        .expect("qa_r375: .github/workflows is not readable")
        .map(|e| e.expect("qa_r375: workflow entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "yml" || x == "yaml"))
        .map(|p| {
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            let text = fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
            (name, text)
        })
        .collect();
    out.sort();
    out
}

/// `files` with `file`'s first `from` replaced by `to`; a control whose edit target is gone fails here.
#[cfg(feature = "controls")]
fn edited(files: &[(String, String)], file: &str, from: &str, to: &str) -> Vec<(String, String)> {
    files
        .iter()
        .map(|(f, t)| {
            if f != file {
                return (f.clone(), t.clone());
            }
            assert!(t.contains(from), "qa_r375: {file} has no {from:?} to edit");
            (f.clone(), t.replacen(from, to, 1))
        })
        .collect()
}

fn indent(l: &str) -> usize {
    l.len() - l.trim_start().len()
}

/// Each job of a workflow's `jobs:` block (indent 2), with the value of its own `runs-on:` (indent 4), if any.
fn jobs(text: &str) -> Vec<(String, Option<String>)> {
    let mut out: Vec<(String, Option<String>)> = Vec::new();
    let mut in_jobs = false;
    for l in text.lines() {
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
            out.push((t.trim_end_matches(':').to_owned(), None));
        } else if indent(l) == 4 {
            if let (Some(v), Some((_, runs_on))) = (t.strip_prefix("runs-on:"), out.last_mut()) {
                let v = v.split(" #").next().unwrap_or("").trim();
                *runs_on = Some(v.trim_matches(|c| c == '"' || c == '\'').to_owned());
            }
        }
    }
    out
}

/// R-375: every job of every workflow runs on a pinned image, `ubuntu-24.04` or `macos-15`; there is at least one
/// workflow, every workflow has a job, and the Linux image is in use (so the check reads something).
fn check_every_runner_pinned(files: &[(String, String)]) {
    assert!(!files.is_empty(), "qa_r375: no workflow was read");
    let mut linux = 0;
    for (file, text) in files {
        let jobs = jobs(text);
        assert!(
            !jobs.is_empty(),
            "qa_r375: {file} has no job the check read"
        );
        for (job, runs_on) in jobs {
            let runs_on =
                runs_on.unwrap_or_else(|| panic!("qa_r375: {file}: job `{job}` names no runs-on"));
            assert!(
                PINNED.contains(&runs_on.as_str()),
                "qa_r375: {file}: job `{job}` runs on `{runs_on}`, not a pinned image (R-375: {PINNED:?})"
            );
            if runs_on == "ubuntu-24.04" {
                linux += 1;
            }
        }
    }
    assert!(linux > 0, "qa_r375: no job runs on ubuntu-24.04");
}

#[test]
fn qa_r375_every_ci_job_runs_on_a_pinned_image() {
    check_every_runner_pinned(&workflows());
}

negative_control!(
    qa_r375_every_ci_job_runs_on_a_pinned_image,
    "ci.yml's `ci` shard job back on ubuntu-latest",
    expected = "job `ci` runs on `ubuntu-latest`, not a pinned image",
    check_every_runner_pinned(&edited(
        &workflows(),
        "ci.yml",
        "  ci:\n    needs: ci-archive\n    runs-on: ubuntu-24.04\n",
        "  ci:\n    needs: ci-archive\n    runs-on: ubuntu-latest\n"
    ))
);

mod other_image {
    use super::*;
    negative_control!(
        qa_r375_every_ci_job_runs_on_a_pinned_image,
        "nightly.yml's mutants job on ubuntu-22.04, an image R-375 does not name",
        expected = "job `mutants` runs on `ubuntu-22.04`, not a pinned image",
        check_every_runner_pinned(&edited(
            &workflows(),
            "nightly.yml",
            "  mutants:\n    runs-on: ubuntu-24.04\n",
            "  mutants:\n    runs-on: ubuntu-22.04\n"
        ))
    );
}

mod quoted_latest {
    use super::*;
    negative_control!(
        qa_r375_every_ci_job_runs_on_a_pinned_image,
        "reviews.yml's job on a quoted \"ubuntu-latest\" with a trailing comment",
        expected = "job `reviews-complete` runs on `ubuntu-latest`, not a pinned image",
        check_every_runner_pinned(&edited(
            &workflows(),
            "reviews.yml",
            "    runs-on: ubuntu-24.04\n",
            "    runs-on: \"ubuntu-latest\" # moves with GitHub\n"
        ))
    );
}

mod macos_latest {
    use super::*;
    negative_control!(
        qa_r375_every_ci_job_runs_on_a_pinned_image,
        "ci.yml's Metal job on macos-latest",
        expected = "runs on `macos-latest`, not a pinned image",
        check_every_runner_pinned(&edited(
            &workflows(),
            "ci.yml",
            "    runs-on: macos-15\n",
            "    runs-on: macos-latest\n"
        ))
    );
}

/// REQ-SYS-065 / R-375: the per-commit lavapipe job, `ci.yml`'s `gpu-lavapipe`, runs on `ubuntu-24.04`.
fn check_lavapipe_pinned(files: &[(String, String)]) {
    let (_, ci) = files
        .iter()
        .find(|(f, _)| f == "ci.yml")
        .expect("qa_r375: no ci.yml");
    let runs_on = jobs(ci)
        .into_iter()
        .find(|(j, _)| j == "gpu-lavapipe")
        .expect("qa_r375: ci.yml has no gpu-lavapipe job")
        .1;
    assert_eq!(
        runs_on.as_deref(),
        Some("ubuntu-24.04"),
        "qa_r375: ci.yml's lavapipe job is not on ubuntu-24.04 (REQ-SYS-065, R-375)"
    );
}

#[test]
fn qa_r375_the_lavapipe_job_runs_on_ubuntu_24_04() {
    check_lavapipe_pinned(&workflows());
}

negative_control!(
    qa_r375_the_lavapipe_job_runs_on_ubuntu_24_04,
    "ci.yml's gpu-lavapipe job on ubuntu-latest",
    expected = "qa_r375: ci.yml's lavapipe job is not on ubuntu-24.04",
    check_lavapipe_pinned(&edited(
        &workflows(),
        "ci.yml",
        "  gpu-lavapipe:\n    runs-on: ubuntu-24.04\n",
        "  gpu-lavapipe:\n    runs-on: ubuntu-latest\n"
    ))
);
