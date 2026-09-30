//! REQ-VAL-165 (R-231): CI's test steps run through `cargo nextest run`, pinned, with doctests, which nextest does not
//! run, through `cargo test --doc`, and no test is dropped: in each feature set, the nextest and `--doc` steps
//! together list every test that the `cargo test` steps they replaced list (`cargo test <args> -- --list`). The
//! documented local run (README) installs the same pinned version.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use validation::spawn::Spawn;

/// The arguments of the `cargo test` steps CI ran before nextest (TASK-M0-33's base): the whole workspace in the
/// default feature set, and the GPU jobs' suites in the `controls` one.
const REPLACED: &[&str] = &[
    "--workspace",
    "-p validation --features controls metal_hosted_probe",
    "-p validation --features controls gpu_harness",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

/// The text of every workflow under `.github/workflows/`.
fn workflows() -> Vec<String> {
    let dir = root().join(".github/workflows");
    let mut texts: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|e| e == "yml" || e == "yaml"))
        .map(|path| std::fs::read_to_string(path).unwrap())
        .collect();
    texts.sort();
    texts
}

fn readme() -> String {
    std::fs::read_to_string(root().join("README.md")).unwrap()
}

/// The jobs of `workflow`: each job's lines, from its name (two spaces in, under `jobs:`) to the next job's.
fn jobs(workflow: &str) -> Vec<Vec<&str>> {
    let mut jobs: Vec<Vec<&str>> = Vec::new();
    let mut in_jobs = false;
    for line in workflow.lines() {
        if !line.starts_with(' ') && !line.is_empty() {
            in_jobs = line.trim_end() == "jobs:";
        } else if in_jobs && line.starts_with("  ") && !line[2..].starts_with([' ', '#']) {
            jobs.push(vec![line]);
        } else if let Some(job) = jobs.last_mut().filter(|_| in_jobs) {
            job.push(line);
        }
    }
    jobs
}

/// The commands of a job's `run:` steps.
fn runs<'a>(job: &[&'a str]) -> Vec<&'a str> {
    job.iter()
        .filter_map(|line| {
            let line = line.trim_start();
            line.strip_prefix("- run: ")
                .or_else(|| line.strip_prefix("run: "))
        })
        .map(str::trim)
        .collect()
}

/// A CI test step: `cargo nextest run <args>` (`doc` false) or `cargo test <args>` with `--doc` (`doc` true), its
/// arguments without `--no-capture` and `--doc`.
struct Step {
    doc: bool,
    args: Vec<String>,
}

/// The value of `--features` in `args`, or "" for the default feature set.
fn features(args: &[String]) -> &str {
    args.iter()
        .position(|a| a == "--features")
        .and_then(|i| args.get(i + 1))
        .map_or("", String::as_str)
}

/// The test steps of `workflows`, and the pinned cargo-nextest version they install. Every job with a nextest step
/// installs cargo-nextest at one exact version, the same in every job; no step runs `cargo test` but for doctests.
fn test_steps(workflows: &[String]) -> (Vec<Step>, String) {
    let mut steps = Vec::new();
    let mut pins: Vec<String> = Vec::new();
    for workflow in workflows {
        for job in jobs(workflow) {
            let mut nextest = false;
            for run in runs(&job) {
                let words = |rest: &str| -> Vec<String> {
                    rest.split_whitespace()
                        .filter(|w| !matches!(*w, "--no-capture" | "--nocapture" | "--doc"))
                        .map(str::to_owned)
                        .collect()
                };
                if let Some(rest) = run.strip_prefix("cargo nextest run") {
                    nextest = true;
                    steps.push(Step {
                        doc: false,
                        args: words(rest),
                    });
                } else if let Some(rest) = run.strip_prefix("cargo test") {
                    assert!(
                        rest.split_whitespace().any(|w| w == "--doc"),
                        "a CI step runs tests through `cargo test`, not nextest: `{run}` (job {})",
                        job[0].trim()
                    );
                    steps.push(Step {
                        doc: true,
                        args: words(rest),
                    });
                }
            }
            let pin = job.iter().find_map(|line| {
                let version = line.trim().strip_prefix("tool: cargo-nextest@")?;
                let exact = version.split('.').count() == 3
                    && version.split('.').all(|n| n.parse::<u32>().is_ok());
                exact.then(|| version.to_owned())
            });
            if nextest {
                let pin = pin.unwrap_or_else(|| {
                    panic!(
                        "job {} runs nextest without installing cargo-nextest at an exact version",
                        job[0].trim()
                    )
                });
                pins.push(pin);
            }
        }
    }
    pins.dedup();
    assert_eq!(
        pins.len(),
        1,
        "CI installs cargo-nextest at more than one version, or none: {pins:?}"
    );
    (steps, pins.remove(0))
}

/// The documented local run in `readme` installs cargo-nextest at `pin` and runs the suite and the doctests.
fn check_documented_run(readme: &str, pin: &str) {
    for line in [
        format!("cargo install cargo-nextest --version {pin} --locked"),
        "cargo nextest run --workspace".to_owned(),
        "cargo test --workspace --doc".to_owned(),
    ] {
        assert!(
            readme.lines().any(|l| l.trim() == line),
            "the README's local run lacks `{line}`"
        );
    }
}

#[test]
fn nextest_ci_and_the_documented_run_install_the_pinned_nextest() {
    let (_, pin) = test_steps(&workflows());
    check_documented_run(&readme(), &pin);
}

validation::negative_control!(
    nextest_ci_and_the_documented_run_install_the_pinned_nextest,
    "a README installing another cargo-nextest version than CI's, required to document the pinned one",
    expected = "the README's local run lacks `cargo install cargo-nextest --version",
    {
        let (_, pin) = test_steps(&workflows());
        check_documented_run(
            &readme().replace(&format!("--version {pin} "), "--version 0.0.1 "),
            &pin,
        )
    }
);

/// Runs `cargo <args>` on this workspace; its stdout, which it must have exited well to give.
fn cargo(args: &[&str]) -> String {
    let output = Command::new(env!("CARGO"))
        .args(args)
        .current_dir(root())
        .timed_output()
        .expect("run cargo");
    assert!(
        output.status.success(),
        "cargo {args:?} failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// The names `cargo test <args> -- --list` lists: every test, ignored or not, doctests included.
fn libtest_list(args: &[String]) -> Vec<String> {
    let mut command: Vec<&str> = vec!["test"];
    command.extend(args.iter().map(String::as_str));
    command.extend(["--", "--list"]);
    cargo(&command)
        .lines()
        .filter_map(|line| line.strip_suffix(": test"))
        .map(str::to_owned)
        .collect()
}

/// The names `cargo nextest list <args>` lists, ignored tests included.
fn nextest_list(args: &[String]) -> Vec<String> {
    let mut command: Vec<&str> = vec!["nextest", "list"];
    command.extend(args.iter().map(String::as_str));
    command.extend([
        "--run-ignored",
        "all",
        "--message-format",
        "oneline",
        "--color",
        "never",
    ]);
    cargo(&command)
        .lines()
        .filter_map(|line| line.split_once(' '))
        .map(|(_, name)| name.to_owned())
        .collect()
}

/// For each of `REPLACED`, the test steps of `workflows` in its feature set together list every test it lists, as
/// many times.
fn check_no_test_dropped(workflows: &[String]) {
    let (steps, _) = test_steps(workflows);
    for replaced in REPLACED {
        let replaced: Vec<String> = replaced.split_whitespace().map(str::to_owned).collect();
        let mut covered: BTreeMap<String, usize> = BTreeMap::new();
        for step in steps
            .iter()
            .filter(|s| features(&s.args) == features(&replaced))
        {
            let listed = if step.doc {
                let mut args = step.args.clone();
                args.push("--doc".to_owned());
                libtest_list(&args)
            } else {
                nextest_list(&step.args)
            };
            for name in listed {
                *covered.entry(name).or_default() += 1;
            }
        }
        let mut wanted: BTreeMap<String, usize> = BTreeMap::new();
        for name in libtest_list(&replaced) {
            *wanted.entry(name).or_default() += 1;
        }
        let dropped: Vec<&String> = wanted
            .iter()
            .filter(|(name, n)| covered.get(*name).copied().unwrap_or(0) < **n)
            .map(|(name, _)| name)
            .collect();
        assert!(
            !wanted.is_empty() && dropped.is_empty(),
            "the CI test steps drop tests `cargo test {}` lists: {dropped:?}",
            replaced.join(" ")
        );
    }
}

#[test]
fn nextest_ci_steps_list_every_test_cargo_test_listed() {
    check_no_test_dropped(&workflows());
}

validation::negative_control!(
    nextest_ci_steps_list_every_test_cargo_test_listed,
    "a CI whose workspace nextest step runs only kernel's tests, required to list every test",
    expected = "the CI test steps drop tests `cargo test --workspace` lists",
    check_no_test_dropped(
        &workflows()
            .iter()
            .map(|w| w.replace(
                "run: cargo nextest run --workspace",
                "run: cargo nextest run -p kernel"
            ))
            .collect::<Vec<_>>()
    )
);
