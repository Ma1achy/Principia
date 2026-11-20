//! QA tests for TASK-M0-57 in xtask, written from REQ-SYS-097's statement and R-862 as R-711 item 2 reads it: "no
//! required or hosted CI job builds the feature or sets `PRIN_GPU_BACKEND=dx12`", and "without the feature `dx12` is
//! refused as an error naming the variable and the feature".
//!
//! The workflow check is this file's own, apart from `ci_self_hosted.rs`'s: every job not on a self-hosted runner, and
//! each workflow's own `env:`, is read as YAML, and no string in it may name `dx12` in any case or enable every
//! feature (`--all-features`). Each test registers a negative control (R-176).

use std::path::Path;
use validation::negative_control;
use yaml_rust2::{Yaml, YamlLoader};

const BACKEND_VAR: &str = "PRIN_GPU_BACKEND";

/// Whether `job` runs on a self-hosted runner: its `runs-on` is a list naming `self-hosted`. A GitHub-hosted label, an
/// expression or a reusable workflow's call all count as hosted.
fn self_hosted(job: &Yaml) -> bool {
    job["runs-on"]
        .as_vec()
        .is_some_and(|labels| labels.iter().any(|l| l.as_str() == Some("self-hosted")))
}

/// Every string in `value`, keys and values, depth first.
fn strings<'a>(value: &'a Yaml, out: &mut Vec<&'a str>) {
    match value {
        Yaml::String(s) => out.push(s),
        Yaml::Array(items) => items.iter().for_each(|v| strings(v, out)),
        Yaml::Hash(h) => h.iter().for_each(|(k, v)| {
            strings(k, out);
            strings(v, out)
        }),
        _ => {}
    }
}

/// The job (or workflow env) `place` of `file` names DX12 or enables every feature in `text`, if it does.
fn offence(text: &str) -> Option<&'static str> {
    if text.to_lowercase().contains("dx12") {
        Some("names dx12")
    } else if text.contains("--all-features") {
        Some("enables every feature, dx12 among them")
    } else {
        None
    }
}

/// No hosted job of the workflow `file` (its text `text`), nor the workflow's own `env:`, names `dx12` or builds
/// `--all-features`. Returns the number of hosted jobs read.
fn check_workflow(file: &str, text: &str) -> usize {
    let docs = YamlLoader::load_from_str(text).unwrap_or_else(|e| panic!("{file}: {e}"));
    let doc = &docs[0];
    let mut read = 0;
    let mut places: Vec<(String, &Yaml)> = vec![("the workflow's env".to_owned(), &doc["env"])];
    if let Some(jobs) = doc["jobs"].as_hash() {
        for (id, job) in jobs {
            if !self_hosted(job) {
                read += 1;
                places.push((format!("hosted job {}", id.as_str().unwrap_or("?")), job));
            }
        }
    }
    for (place, value) in places {
        let mut found = Vec::new();
        strings(value, &mut found);
        for s in found {
            if let Some(what) = offence(s) {
                panic!("{file}, {place}: {what} (R-862, R-711 item 2): {s}");
            }
        }
    }
    read
}

/// R-862: no required or hosted CI job builds the `dx12` feature or sets `PRIN_GPU_BACKEND=dx12`. Every required job is
/// a hosted one, so reading every hosted job covers both.
#[test]
fn qa_m0_57_no_hosted_job_names_dx12() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../.github/workflows");
    let mut jobs = 0;
    for entry in std::fs::read_dir(&dir).expect("the workflows") {
        let path = entry.expect("an entry").path();
        if path.extension().is_some_and(|e| e == "yml" || e == "yaml") {
            let text = std::fs::read_to_string(&path).expect("a workflow");
            jobs += check_workflow(&path.display().to_string(), &text);
        }
    }
    assert!(jobs > 0, "read no hosted job");
}

/// A hosted job, with `body` its last step's command and `env` its own `env:` block.
fn hosted_workflow(env: &str, body: &str) -> String {
    format!(
        "name: t\non: push\njobs:\n  test:\n    runs-on: ubuntu-24.04\n    env:\n      {env}\n    steps:\n      - run: {body}\n"
    )
}

negative_control!(
    qa_m0_57_no_hosted_job_names_dx12,
    "a hosted job setting PRIN_GPU_BACKEND=dx12 in its env",
    expected = "names dx12",
    {
        check_workflow(
            "synthetic",
            &hosted_workflow(
                "PRIN_GPU_BACKEND: dx12",
                "cargo test -p validation gpu_harness",
            ),
        );
    }
);

/// The check's other reaches each fail: the feature by `--features`, by `-F`, inline in a command, every feature, the
/// workflow's own env; and a self-hosted job doing the same passes.
#[test]
fn qa_m0_57_workflow_check_sees_each_way_in() {
    let fails = |text: &str, what: &str| {
        let outcome = std::panic::catch_unwind(|| check_workflow("synthetic", text))
            .expect_err(&format!("passed: {what}"));
        let said = outcome
            .downcast_ref::<String>()
            .cloned()
            .unwrap_or_default();
        assert!(said.contains("(R-862"), "{what}: {said}");
    };
    let quiet = "PRIN_GPU_BACKEND: vulkan";
    fails(
        &hosted_workflow(quiet, "cargo test -p validation --features dx12"),
        "--features dx12",
    );
    fails(
        &hosted_workflow(quiet, "cargo build -p gui -F \"mock,gui/dx12\""),
        "-F gui/dx12",
    );
    fails(
        &hosted_workflow(quiet, "PRIN_GPU_BACKEND=DX12 cargo test -p validation"),
        "inline",
    );
    fails(
        &hosted_workflow(quiet, "cargo clippy --workspace --all-features"),
        "--all-features",
    );
    fails(
        &format!(
            "env:\n  {BACKEND_VAR}: dx12\n{}",
            hosted_workflow(quiet, "cargo test")
        ),
        "the workflow's env",
    );
    let runner = "name: t\non: push\njobs:\n  dx12:\n    runs-on: [self-hosted, windows]\n    steps:\n      \
                  - run: cargo test -p validation --features dx12\n        env:\n          PRIN_GPU_BACKEND: dx12\n";
    assert_eq!(
        check_workflow("runner", runner),
        0,
        "a self-hosted job was read as hosted"
    );
}

negative_control!(
    qa_m0_57_workflow_check_sees_each_way_in,
    "a hosted job with nothing of DX12, which the check passes",
    expected = "passed: nothing of DX12",
    {
        let text = hosted_workflow("PRIN_GPU_BACKEND: vulkan", "cargo test -p validation");
        std::panic::catch_unwind(|| check_workflow("synthetic", &text))
            .expect_err("passed: nothing of DX12");
    }
);

fn check_refusal(runner: &str, outcome: Result<wgpu::Backends, String>) {
    match outcome {
        Ok(b) => {
            panic!("{runner}: {BACKEND_VAR}=dx12 was taken as {b:?} in a build without the feature")
        }
        Err(err) => assert!(
            err.contains(BACKEND_VAR),
            "{runner}: the refusal of dx12 names no {BACKEND_VAR}: {err}"
        ),
    }
}

/// "Without the feature `dx12` is refused as an error" (REQ-SYS-097): xtask's golden and screenshot runners read
/// `PRIN_GPU_BACKEND` too and build wgpu without DX12, so `dx12` there is refused naming the variable, never quietly
/// another backend. (Whether their refusal must also name a feature xtask lacks is left to the review, not guessed.)
#[test]
fn qa_m0_57_xtask_runners_refuse_dx12() {
    check_refusal(
        "golden",
        xtask::golden::backend_from(Some("dx12")).map(|(_, b)| b),
    );
    check_refusal("screenshot", xtask::screenshot::backend(Some("dx12")));
}

negative_control!(
    qa_m0_57_xtask_runners_refuse_dx12,
    "dx12 quietly read as Vulkan",
    expected = "was taken as",
    check_refusal("golden", Ok(wgpu::Backends::VULKAN))
);
