//! QA tests for `tooling/ci-one-build`, the owner's items (f) and (g) of 4 Nov, written from those items, REQ-SYS-077
//! and REQ-VAL-165, not from the code:
//!
//! - (f) "stop the second build in xtask-ci": no `xtask-ci` shard runs a control that lists the workspace in its
//!   default feature set, and each such control still runs on every head the shards run on, in `ci-workspace`, once, and
//!   in the gate and nightly workflows' unpartitioned `cargo xtask ci`.
//! - (g) "move qa_TASK-M0-24 from the `ci` profile to `ci-workspace` (still run, not dropped)": out of the `ci` shards,
//!   in one job's step that names it in every workflow that ran it, alone on the target (REQ-VAL-164).
//! - "Every PR: lowest free disk per job": what the job gains stays under the disk sampler.
//!
//! The workflows and `.config/nextest.toml` are read as text, and no test here runs a workflow's step or a cargo build:
//! this binary runs in a `ci` shard, the job (g) takes a build out of. The one child is xtask against a stand-in `cargo`.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use toml_edit::DocumentMut;
use validation::negative_control;
use validation::spawn::Spawn;
use xtask::controls::IN_CI_WORKSPACE;

const M0_24_FILTER: &str = "binary_id(=validation::qa_TASK-M0-24)";
const DOCS_ONLY_SKIP: &str = "needs.changes.outputs.docs_only != 'true'";
const SAMPLER: &str = "bash .github/scripts/disk_sampler.sh {0}";

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the workspace root")
}

fn read(path: &str) -> String {
    fs::read_to_string(root().join(path)).expect("readable")
}

fn workflow(name: &str) -> String {
    read(&format!(".github/workflows/{name}.yml"))
}

/// The lines of job `name` of `workflow`, comments out, from its header to the next job's or the next top-level key.
fn job_lines<'a>(workflow: &'a str, name: &str) -> Vec<&'a str> {
    let header = format!("  {name}:");
    let mut lines = Vec::new();
    let mut inside = false;
    for line in workflow.lines() {
        if line.trim_start().starts_with('#') || line.trim().is_empty() {
            continue;
        }
        if !line.starts_with(' ') || (line.starts_with("  ") && !line.starts_with("   ")) {
            inside = line.trim_end() == header;
        } else if inside {
            lines.push(line);
        }
    }
    assert!(!lines.is_empty(), "no job `{name}`");
    lines
}

/// One workflow step: what the fields this file reads hold.
#[derive(Debug, Default)]
struct Step {
    name: String,
    condition: String,
    run: String,
    shell: String,
    profile: String,
}

/// The steps of a job's lines.
fn steps(job: &[&str]) -> Vec<Step> {
    let mut steps: Vec<Step> = Vec::new();
    for line in job {
        let body = match line.strip_prefix("      - ") {
            Some(first) => {
                steps.push(Step::default());
                first
            }
            None => line.trim_start(),
        };
        let Some(step) = steps.last_mut() else {
            continue;
        };
        let value = |key: &str| body.strip_prefix(key).map(|v| v.trim().to_owned());
        if let Some(v) = value("name:") {
            step.name = v;
        } else if let Some(v) = value("if:") {
            step.condition = v;
        } else if let Some(v) = value("run:") {
            step.run = v;
        } else if let Some(v) = value("shell:") {
            step.shell = v;
        } else if let Some(v) = value("NEXTEST_PROFILE:") {
            step.profile = v;
        }
    }
    steps
}

/// The job-level `if:` of a job's lines.
fn job_condition<'a>(job: &[&'a str]) -> &'a str {
    job.iter()
        .find_map(|line| line.strip_prefix("    if: "))
        .map_or("", str::trim)
}

/// The job names of `workflow`.
fn job_names(workflow: &str) -> Vec<String> {
    let mut in_jobs = false;
    let mut names = Vec::new();
    for line in workflow.lines() {
        if !line.starts_with(' ') && !line.is_empty() && !line.starts_with('#') {
            in_jobs = line.trim_end() == "jobs:";
        } else if in_jobs && line.starts_with("  ") && !line[2..].starts_with([' ', '#']) {
            names.push(line.trim().trim_end_matches(':').to_owned());
        }
    }
    names
}

/// The `negative_control!` blocks of a test source: each control's name and its text.
fn control_blocks(source: &str) -> Vec<(String, String)> {
    source
        .split("validation::negative_control!(\n")
        .skip(1)
        .map(|rest| {
            let body = rest.split("\n);").next().unwrap_or(rest);
            let name = body
                .trim_start()
                .split(',')
                .next()
                .expect("a control names its test")
                .trim();
            (name.to_owned(), body.to_owned())
        })
        .collect()
}

/// Whether a control's text lists the workspace in its default feature set: it runs the no-test-dropped check on
/// `REPLACED`, or on its first entry, and not only on the two that carry `--features controls`.
fn lists_default_features(control: &str) -> bool {
    control.contains("check_no_test_dropped")
        && control.match_indices("REPLACED").any(|(at, _)| {
            !control[at..].starts_with("REPLACED[1]") && !control[at..].starts_with("REPLACED[2]")
        })
}

/// The second build's cause: `source`'s controls that list the default feature set are exactly those the shards leave.
fn check_shards_leave_every_default_listing(source: &str) {
    let listing: BTreeSet<String> = control_blocks(source)
        .into_iter()
        .filter(|(_, body)| lists_default_features(body))
        .map(|(name, _)| format!("{name}::negative_control"))
        .collect();
    let left: BTreeSet<String> = IN_CI_WORKSPACE.iter().map(|c| (*c).to_owned()).collect();
    assert!(!listing.is_empty(), "no control lists the workspace");
    let stay: Vec<&String> = listing.difference(&left).collect();
    assert!(
        stay.is_empty(),
        "a shard still runs a control that builds the default feature set: {stay:?}"
    );
    let extra: Vec<&String> = left.difference(&listing).collect();
    assert!(
        extra.is_empty(),
        "the shards leave a control that lists no default feature set: {extra:?}"
    );
}

#[test]
fn qa_one_build_the_shards_leave_every_default_feature_listing() {
    check_shards_leave_every_default_listing(&read("xtask/tests/nextest.rs"));
}

negative_control!(
    qa_one_build_the_shards_leave_every_default_feature_listing,
    "a fourth control that lists the default feature set, in a shard",
    expected = "a shard still runs a control that builds the default feature set",
    check_shards_leave_every_default_listing(&format!(
        "{}\nvalidation::negative_control!(\n    a_fourth,\n    \"…\",\n    check_no_test_dropped(&x, &[REPLACED[0]])\n);\n",
        read("xtask/tests/nextest.rs")
    ))
);

/// Each control the shards leave is in one step of `ci`, in `ci-workspace`, built with the feature it compiles under,
/// under the profile that selects its binary, and runs on exactly the heads the shards run on.
fn check_left_controls_run_once(ci: &str) {
    let shards = job_lines(ci, "xtask-ci");
    assert_eq!(
        job_condition(&shards),
        DOCS_ONLY_SKIP,
        "the shards' own condition changed"
    );
    for control in IN_CI_WORKSPACE {
        let mut found = Vec::new();
        for name in job_names(ci) {
            let job = job_lines(ci, &name);
            for step in steps(&job) {
                if step.run.split_whitespace().any(|word| word == control) {
                    found.push((name.clone(), job_condition(&job).to_owned(), step));
                }
            }
        }
        assert_eq!(found.len(), 1, "`{control}` is not in exactly one step");
        let (job, condition, step) = &found[0];
        assert_eq!(job, "ci-workspace", "`{control}` runs in `{job}`");
        assert!(
            condition.is_empty() && step.condition == DOCS_ONLY_SKIP,
            "`{control}` runs on other heads than the shards that left it: `{}`",
            step.condition
        );
        let words: Vec<&str> = step.run.split_whitespace().collect();
        for want in [
            ["-p", "xtask"],
            ["--features", "controls"],
            ["--test", "nextest"],
        ] {
            assert!(
                words.windows(2).any(|pair| pair == want),
                "`{}` lacks {want:?}",
                step.run
            );
        }
        assert_eq!(
            step.profile, "ci-workspace",
            "the step is not under the `ci-workspace` profile"
        );
    }
}

#[test]
fn qa_one_build_left_controls_run_once_per_push() {
    check_left_controls_run_once(&workflow("ci"));
}

negative_control!(
    qa_one_build_left_controls_run_once_per_push,
    "the controls' step skipped on a head the shards run on",
    expected = "runs on other heads than the shards that left it",
    check_left_controls_run_once(&workflow("ci").replace(
        "      - name: cargo nextest run --features controls (the default-feature listing's controls)\n        if: needs.changes.outputs.docs_only != 'true'",
        "      - name: cargo nextest run --features controls (the default-feature listing's controls)\n        if: needs.changes.outputs.gpu == 'true'",
    ))
);

/// The job's lowest free disk stays measured (the owner's "lowest free disk per job"): every nextest step of
/// `ci-workspace` on a head with the full run, the heads the shards run on, is under the disk sampler, and ends before
/// the job's last `df`.
fn check_ci_workspace_is_measured(ci: &str) {
    let job = job_lines(ci, "ci-workspace");
    let steps = steps(&job);
    let last_df = steps
        .iter()
        .position(|step| step.name == "Runner disk after the tests")
        .expect("the job's last df");
    let nextest: Vec<usize> = (0..steps.len())
        .filter(|&at| {
            steps[at].run.starts_with("cargo nextest run") && steps[at].condition == DOCS_ONLY_SKIP
        })
        .collect();
    assert!(
        nextest.len() >= 3,
        "fewer nextest steps than (f) and (g) need"
    );
    for at in nextest {
        assert_eq!(
            steps[at].shell, SAMPLER,
            "`{}` runs outside the disk sampler",
            steps[at].name
        );
        assert!(at < last_df, "`{}` runs after the last df", steps[at].name);
    }
}

#[test]
fn qa_one_build_ci_workspace_stays_measured() {
    check_ci_workspace_is_measured(&workflow("ci"));
}

negative_control!(
    qa_one_build_ci_workspace_stays_measured,
    "qa_TASK-M0-24's step on the default shell",
    expected = "runs outside the disk sampler",
    check_ci_workspace_is_measured(&workflow("ci").replace(
        "run: cargo nextest run -p validation --test qa_TASK-M0-24\n        shell: bash .github/scripts/disk_sampler.sh {0}",
        "run: cargo nextest run -p validation --test qa_TASK-M0-24",
    ))
);

/// The gate and nightly `cargo xtask ci` run the left controls too: each is unpartitioned.
fn check_gate_and_nightly_unpartitioned(gate: &str, nightly: &str) {
    for text in [gate, nightly] {
        assert!(
            text.lines().any(|line| line.trim() == "run: cargo xtask ci"),
            "a workflow's `cargo xtask ci` is partitioned or gone, so nothing there runs the left controls"
        );
        assert!(!text.contains("--partition"), "a workflow partitions");
    }
}

#[test]
fn qa_one_build_gate_and_nightly_run_every_control() {
    check_gate_and_nightly_unpartitioned(&workflow("gate"), &workflow("nightly"));
}

negative_control!(
    qa_one_build_gate_and_nightly_run_every_control,
    "a gate whose `cargo xtask ci` runs one shard",
    expected = "a workflow's `cargo xtask ci` is partitioned or gone",
    check_gate_and_nightly_unpartitioned(
        &workflow("gate").replace(
            "run: cargo xtask ci\n",
            "run: cargo xtask ci --partition 1/4\n"
        ),
        &workflow("nightly")
    )
);

/// The `default-filter` of nextest profile `profile`.
fn default_filter(config: &DocumentMut, profile: &str) -> String {
    config["profile"][profile]["default-filter"]
        .as_str()
        .unwrap_or_else(|| panic!("no default-filter in profile {profile}"))
        .to_owned()
}

/// `qa_TASK-M0-24` is in exactly one of the three CI profiles, `ci-workspace`, and alone on the target there: nextest
/// inherits the default profile's overrides, one of which gives it every test thread (REQ-VAL-164).
fn check_m0_24_profile(text: &str) {
    let config: DocumentMut = text.parse().expect("nextest.toml parses");
    let ci = default_filter(&config, "ci");
    let inside_not = ci
        .trim_start()
        .strip_prefix("not (")
        .is_some_and(|rest| rest.contains(M0_24_FILTER));
    assert!(inside_not, "the `ci` shards still select qa_TASK-M0-24");
    assert!(
        default_filter(&config, "ci-workspace").contains(M0_24_FILTER),
        "`ci-workspace` does not select qa_TASK-M0-24"
    );
    assert!(
        !default_filter(&config, "gpu-kernel").contains("qa_TASK-M0-24"),
        "`gpu-kernel` selects qa_TASK-M0-24 too"
    );
    let overrides = config["profile"]["default"]["overrides"]
        .as_array_of_tables()
        .expect("the default profile's overrides");
    let alone = overrides.iter().any(|table| {
        table.get("threads-required").and_then(|v| v.as_str()) == Some("num-test-threads")
            && table
                .get("filter")
                .and_then(|v| v.as_str())
                .is_some_and(|filter| filter.contains("validation::qa_TASK-M0-24"))
    });
    assert!(alone, "qa_TASK-M0-24 does not hold every test thread");
}

#[test]
fn qa_one_build_m0_24_is_in_ci_workspace_alone() {
    check_m0_24_profile(&read(".config/nextest.toml"));
}

negative_control!(
    qa_one_build_m0_24_is_in_ci_workspace_alone,
    "a config that only names it in `ci-workspace`, and leaves it in the `ci` shards",
    expected = "the `ci` shards still select qa_TASK-M0-24",
    check_m0_24_profile(&read(".config/nextest.toml").replacen(
        &format!("        | {M0_24_FILTER}\n"),
        "",
        1
    ))
);

/// In each workflow that runs the `ci` profile, a job that runs it, or that job's twin, has a `ci-workspace` step that
/// can select `qa_TASK-M0-24`: one naming the package with no `--test`, or `--test qa_TASK-M0-24` itself.
fn check_m0_24_step(name: &str, text: &str) {
    let mut selecting = 0;
    for job in job_names(text) {
        for step in steps(&job_lines(text, &job)) {
            let words: Vec<&str> = step.run.split_whitespace().collect();
            let package = words.windows(2).any(|pair| pair == ["-p", "validation"]);
            let targets: Vec<&[&str]> = words
                .windows(2)
                .filter(|pair| pair[0] == "--test")
                .collect();
            let names_it = targets.is_empty() || targets.iter().any(|p| p[1] == "qa_TASK-M0-24");
            if step.profile == "ci-workspace"
                && step.run.starts_with("cargo nextest run")
                && package
                && names_it
            {
                selecting += 1;
            }
        }
    }
    assert_eq!(selecting, 1, "`{name}` does not run qa_TASK-M0-24 once");
}

#[test]
fn qa_one_build_m0_24_still_runs_in_each_workflow() {
    for name in ["ci", "gate", "nightly"] {
        check_m0_24_step(name, &workflow(name));
    }
}

negative_control!(
    qa_one_build_m0_24_still_runs_in_each_workflow,
    "a nightly step that names another of validation's test targets",
    expected = "does not run qa_TASK-M0-24 once",
    check_m0_24_step(
        "nightly",
        &workflow("nightly").replace("--test qa_TASK-M0-24", "--test bringup")
    )
);

/// `xtask controls --list --partition <slice>` against a stand-in `cargo` listing `listed`, in `case`'s directory: its
/// output, which the run must have exited well to give.
fn listing_shard(case: &str, slice: &str, listed: &str) -> String {
    let dir: PathBuf = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("qa_one_build_{case}_{}", slice.replace('/', "of")));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("listed.txt"), listed).unwrap();
    let cargo = dir.join("cargo");
    validation::spawn::write_executable(
        &cargo,
        format!(
            r#"#!/bin/sh
if [ "$1" = metadata ]; then
  echo '{{"packages":[{{"name":"fake","features":{{"controls":[]}},"targets":[{{"doctest":false}}]}}]}}'
  exit 0
fi
cat '{}/listed.txt'
"#,
            dir.display()
        ),
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["controls", "--list", "--partition", slice])
        .env("CARGO", &cargo)
        .timed_output()
        .expect("run xtask");
    let out = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(output.status.success(), "shard {slice} failed:\n{out}");
    out
}

/// What each shard of 4 lists as its controls leaves out `left`, and the unpartitioned listing shows all of `left`.
fn check_listing_leaves(case: &str, left: &[&str]) {
    let listed: String = left
        .iter()
        .map(|control| {
            let test = control.strip_suffix("::negative_control").unwrap();
            format!("{test}: test\n{control}: test\n")
        })
        .collect();
    for k in 1..=4 {
        let slice = format!("{k}/4");
        let out = listing_shard(case, &slice, &listed);
        for control in left {
            assert!(
                !out.contains(&format!("control `{control}`")),
                "shard {slice} lists `{control}` as its own"
            );
        }
    }
    let all = listing_shard(case, "1/1", &listed);
    for control in left {
        assert!(
            all.contains(&format!("control `{control}`")),
            "the unpartitioned listing leaves out `{control}`"
        );
    }
}

#[test]
fn qa_one_build_a_shard_lists_no_left_control() {
    check_listing_leaves("left", &IN_CI_WORKSPACE);
}

negative_control!(
    qa_one_build_a_shard_lists_no_left_control,
    "a control no job leaves: some shard lists it",
    expected = "lists `plain_control::negative_control` as its own",
    check_listing_leaves("kept", &["plain_control::negative_control"])
);
