//! `cargo xtask controls` (REQ-VAL-147): every test in a crate that declares the `controls` feature has a negative
//! control, registered with `validation::negative_control!(test_name, "description", control)`, and every control
//! makes its test fail (philosophy §4.4; pitfalls §9). A crate without the feature is skipped and reported, not failed
//! (R-176). Tests and controls are paired by the name in the macro call (R-199), across all of a crate's test
//! targets, so a control in `tests/` pairs with a unit test in `src/` (R-201). Not yet in `cargo xtask ci` (R-198).
//!
//! Applied per R-204, pending a veto: a doctest, which `negative_control!` cannot name, counts as a test without a
//! control.

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;
use std::process::{Command, Output};

use serde::Deserialize;

use crate::deps::cargo;

/// The feature under which `negative_control!` compiles its controls (R-176).
pub const FEATURE: &str = "controls";

/// The last path segment of the test `negative_control!` generates, under a module named for its test.
pub const CONTROL_FN: &str = "negative_control";

/// A test the command fails on, by the test's full name as libtest lists it.
#[derive(Debug, PartialEq, Eq)]
pub enum Finding {
    /// No `negative_control!` names the test.
    NoControl(String),
    /// A control named for the test ran and left it passing.
    ControlPasses(String),
    /// A doctest, which `negative_control!` cannot name, so it has no control (applied per R-204).
    Doctest(String),
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Finding::NoControl(test) => write!(
                f,
                "test `{test}` has no control: no `negative_control!` names it (REQ-VAL-147, R-199)"
            ),
            Finding::ControlPasses(test) => write!(
                f,
                "test `{test}`: its control leaves it passing, so it cannot fail (philosophy §4.4)"
            ),
            Finding::Doctest(test) => write!(
                f,
                "doctest `{test}` has no control: `negative_control!` cannot name a doctest (REQ-VAL-147)"
            ),
        }
    }
}

/// The test a listed name is the control of, when it is one: `<path>::<test>::negative_control` gives `<test>`.
pub fn control_of(name: &str) -> Option<&str> {
    let module = name.strip_suffix(CONTROL_FN)?.strip_suffix("::")?;
    module.rsplit("::").next()
}

/// The test names in the output of libtest's `--list`, across every target (`<name>: test` lines).
pub fn parse_list(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .filter_map(|line| line.strip_suffix(": test"))
        .map(str::to_owned)
        .collect()
}

/// Each test libtest reports in `stdout`, and whether it passed, once per run of it. A crate's test targets are
/// separate binaries, so one name can run in several of them (two targets each registering
/// `negative_control!(doubles, ...)`); each run keeps its own outcome, in the order reported. A control is
/// `#[should_panic]`, so it passes when it makes its test's check fail.
pub fn parse_results(stdout: &str) -> BTreeMap<String, Vec<bool>> {
    let mut results: BTreeMap<String, Vec<bool>> = BTreeMap::new();
    for line in stdout.lines() {
        let Some((name, outcome)) = line
            .strip_prefix("test ")
            .and_then(|rest| rest.rsplit_once(" ... "))
        else {
            continue;
        };
        let name = name.strip_suffix(" - should panic").unwrap_or(name);
        results
            .entry(name.to_owned())
            .or_default()
            .push(outcome == "ok");
    }
    results
}

/// Pairs each listed test with the controls named for it, by the test's last path segment (R-199, R-201), and
/// returns the tests with no control or with a control that `results` shows passing. Every run of every paired
/// control is judged on its own: one leaky control fails its test, whatever a control of the same name in another
/// target does. A name listed in several targets is reported once. `Err` when a paired control has fewer results
/// than it was listed.
pub fn findings(
    listed: &[String],
    results: &BTreeMap<String, Vec<bool>>,
) -> Result<Vec<Finding>, String> {
    let mut controls: Vec<(&str, &String, usize)> = Vec::new();
    for name in listed {
        let Some(of) = control_of(name) else { continue };
        match controls.iter_mut().find(|(_, control, _)| *control == name) {
            Some((_, _, count)) => *count += 1,
            None => controls.push((of, name, 1)),
        }
    }
    let mut tests: Vec<&String> = Vec::new();
    for name in listed.iter().filter(|name| control_of(name).is_none()) {
        if !tests.contains(&name) {
            tests.push(name);
        }
    }
    let mut found = Vec::new();
    for test in tests {
        let short = test.rsplit("::").next().unwrap_or(test);
        let mut paired = controls.iter().filter(|(of, _, _)| *of == short).peekable();
        if paired.peek().is_none() {
            found.push(Finding::NoControl(test.clone()));
        }
        let mut leaks = false;
        for (_, control, count) in paired {
            let runs = results.get(*control).map_or(&[][..], Vec::as_slice);
            if runs.len() < *count {
                return Err(format!(
                    "the control `{control}` of test `{test}` did not run"
                ));
            }
            leaks |= runs.iter().any(|passed| !passed);
        }
        if leaks {
            found.push(Finding::ControlPasses(test.clone()));
        }
    }
    Ok(found)
}

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
}

#[derive(Deserialize)]
struct Package {
    name: String,
    features: BTreeMap<String, Vec<String>>,
    targets: Vec<Target>,
}

#[derive(Deserialize)]
struct Target {
    doctest: bool,
}

/// Runs the check on every member of the workspace of `manifest`; `Err` if any test fails it.
pub fn run(manifest: &Path) -> Result<(), String> {
    let output = Command::new(cargo())
        .args([
            "metadata",
            "--no-deps",
            "--format-version",
            "1",
            "--manifest-path",
        ])
        .arg(manifest)
        .output()
        .map_err(|e| format!("cannot run cargo metadata: {e}"))?;
    let stdout = succeeded("cargo metadata", output)?;
    let metadata: Metadata =
        serde_json::from_str(&stdout).map_err(|e| format!("cannot parse cargo metadata: {e}"))?;
    let mut failed = 0;
    for package in &metadata.packages {
        let name = &package.name;
        if !package.features.contains_key(FEATURE) {
            println!("xtask controls: {name}: skipped: it declares no `{FEATURE}` feature (R-176)");
            continue;
        }
        let listed = parse_list(&succeeded(
            "cargo test -- --list",
            cargo_test(manifest, name, "--tests", &["--list"])?,
        )?);
        // Doctests are tests of the crate that no `negative_control!` can name (applied per R-204).
        let doctests = if package.targets.iter().any(|target| target.doctest) {
            parse_list(&succeeded(
                "cargo test --doc -- --list",
                cargo_test(manifest, name, "--doc", &["--list"])?,
            )?)
        } else {
            Vec::new()
        };
        let tests = listed.iter().filter(|t| control_of(t).is_none()).count();
        // Runs the tests whose names contain `negative_control`: every control, and none when there are none.
        let output = cargo_test(manifest, name, "--tests", &[CONTROL_FN])?;
        let results = parse_results(&String::from_utf8_lossy(&output.stdout));
        let mut found = findings(&listed, &results).map_err(|e| format!("{name}: {e}"))?;
        found.extend(doctests.into_iter().map(Finding::Doctest));
        for finding in &found {
            eprintln!("xtask controls: {name}: {finding}");
        }
        if found.is_empty() {
            println!("xtask controls: {name}: {tests} test(s), each failed by its control");
        }
        failed += found.len();
    }
    if failed == 0 {
        return Ok(());
    }
    Err(format!(
        "{failed} test(s) without a control that makes them fail (REQ-VAL-147)"
    ))
}

/// `cargo test --features controls <targets> --no-fail-fast` on `package`, with `harness` passed to libtest.
fn cargo_test(
    manifest: &Path,
    package: &str,
    targets: &str,
    harness: &[&str],
) -> Result<Output, String> {
    Command::new(cargo())
        .arg("test")
        .arg("--manifest-path")
        .arg(manifest)
        .args([
            "-p",
            package,
            "--features",
            FEATURE,
            targets,
            "--no-fail-fast",
            "--",
        ])
        .args(harness)
        .output()
        .map_err(|e| format!("cannot run cargo test: {e}"))
}

/// The stdout of a command that must succeed, or `Err` with its stderr.
fn succeeded(what: &str, output: Output) -> Result<String, String> {
    if !output.status.success() {
        return Err(format!(
            "{what} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
