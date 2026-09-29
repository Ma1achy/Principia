//! `cargo xtask controls` (REQ-VAL-147): every test in a crate that declares the `controls` feature has a negative
//! control, registered with `validation::negative_control!(test_name, "description", expected = "message", control)`,
//! and every control makes its test fail by panicking with its expected message (philosophy §4.4; pitfalls §9;
//! R-212). A crate without the feature is skipped and reported, not failed
//! (R-176). Tests and controls are paired by the name in the macro call (R-199), across all of a crate's test
//! targets, so a control in `tests/` pairs with a unit test in `src/` (R-201). It runs in `cargo xtask ci`, on every
//! push (R-177, R-198). With `--list` it lists each test's controls and runs none (R-226).
//!
//! Applied per R-204, pending a veto: within a crate, a control covers exactly one test, so tests sharing a name
//! need a control each; and a doctest, which `negative_control!` cannot name, counts as a test without a control.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::Path;
use std::process::{Command, Output};

use serde::Deserialize;

use crate::deps::cargo;

/// The feature under which `negative_control!` compiles its controls (R-176).
pub const FEATURE: &str = "controls";

/// The last path segment of the test `negative_control!` generates, under a module named for its test. The name is
/// reserved in a crate that declares the feature: a listing carries no mark of the macro, so a hand-written test
/// whose path ends in `::negative_control` is read as a control of its parent module's name.
pub const CONTROL_FN: &str = "negative_control";

/// A test the command fails on, by the test's full name as libtest lists it.
#[derive(Debug, PartialEq, Eq)]
pub enum Finding {
    /// No `negative_control!` names the test.
    NoControl(String),
    /// A control named for the test ran and did not make it fail.
    ControlPasses(String),
    /// A control named for the test panicked, but not with the message it expects (R-212), so it did not trip the
    /// test's check. `note` is libtest's: the panic message and the expected substring.
    WrongPanic {
        /// The test's full name.
        test: String,
        /// libtest's note on the panic.
        note: String,
    },
    /// A doctest, which `negative_control!` cannot name, so it has no control (applied per R-204).
    Doctest(String),
    /// Tests sharing the last path segment `name` are listed more times across the crate's targets than controls
    /// named `name` are: each test needs its own control (applied per R-204).
    Collision {
        /// The shared name, as `negative_control!` names it.
        name: String,
        /// The distinct full names of the tests that share it.
        tests: Vec<String>,
        /// How many times tests of that name are listed, counting each target's listing.
        listings: usize,
        /// How many controls named `name` are listed.
        controls: usize,
    },
}

impl Finding {
    /// How many tests the finding leaves without a control that makes them fail.
    pub fn tests(&self) -> usize {
        match self {
            Finding::Collision {
                listings, controls, ..
            } => listings - controls,
            _ => 1,
        }
    }
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
                "test `{test}`: its control did not make it fail, so the test may not be able to (philosophy §4.4)"
            ),
            Finding::WrongPanic { test, note } => write!(
                f,
                "test `{test}`: its control did not make it fail (philosophy §4.4): it panicked without its \
                 expected message (R-212): {note}"
            ),
            Finding::Doctest(test) => write!(
                f,
                "doctest `{test}` has no control: `negative_control!` cannot name a doctest (REQ-VAL-147)"
            ),
            Finding::Collision {
                name,
                tests,
                listings,
                controls,
            } => write!(
                f,
                "tests `{}` share the name `{name}`: listed {listings} time(s), but {controls} control(s) name \
                 `{name}`, and each test needs its own (REQ-VAL-147)",
                tests.join("`, `")
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

/// The controls libtest reports in `stdout` as panicking without their expected message (R-212), each with libtest's
/// note: the panic message and the expected substring, on one line.
pub fn parse_wrong_panics(stdout: &str) -> BTreeMap<String, String> {
    let mut found = BTreeMap::new();
    let mut current = None;
    let mut lines = stdout.lines();
    while let Some(line) = lines.next() {
        if let Some(name) = line
            .strip_prefix("---- ")
            .and_then(|rest| rest.strip_suffix(" stdout ----"))
        {
            current = Some(name);
        } else if line == "note: panic did not contain expected string" {
            if let Some(name) = current {
                let note = lines.by_ref().take(2).map(str::trim).collect::<Vec<_>>();
                found.insert(name.to_owned(), note.join(" "));
            }
        }
    }
    found
}

/// `found` with each `ControlPasses` whose test has a control in `wrong` (by [`parse_wrong_panics`]) reported as
/// `WrongPanic` with that control's note.
pub fn name_wrong_panics(found: Vec<Finding>, wrong: &BTreeMap<String, String>) -> Vec<Finding> {
    found
        .into_iter()
        .map(|finding| {
            let Finding::ControlPasses(test) = finding else {
                return finding;
            };
            let short = test.rsplit("::").next().unwrap_or(&test);
            match wrong
                .iter()
                .find(|(control, _)| control_of(control) == Some(short))
            {
                Some((_, note)) => Finding::WrongPanic {
                    test,
                    note: note.clone(),
                },
                None => Finding::ControlPasses(test),
            }
        })
        .collect()
}

/// Each test libtest reports failing in `stdout`, with its own output: the lines under its `---- <name> stdout ----`
/// header, up to the next header or libtest's closing list of failures. A name failing in several targets keeps
/// each output.
///
/// A test's output can itself hold libtest's lines (a control that embeds a child's `cargo test` report), so a
/// boundary is taken only where it is libtest's own, for the target being read: a header names a test that target
/// reported `FAILED` and not yet given a header, and the closing list is a `failures:` line followed by exactly the
/// names that target reported `FAILED`, sorted, then a blank line and `test result:`.
pub fn parse_outputs(stdout: &str) -> BTreeMap<String, Vec<String>> {
    let lines: Vec<&str> = stdout.lines().collect();
    let mut blocks: Vec<(&str, String)> = Vec::new();
    // The tests the current target reported failing, and those of them still without a header.
    let mut failed: Vec<&str> = Vec::new();
    let mut pending: Vec<&str> = Vec::new();
    let mut in_outputs = false;
    let mut open = false;
    for (at, line) in lines.iter().enumerate() {
        if !in_outputs {
            if let Some((name, "FAILED")) = line
                .strip_prefix("test ")
                .and_then(|rest| rest.rsplit_once(" ... "))
            {
                failed.push(name.strip_suffix(" - should panic").unwrap_or(name));
            } else if *line == "failures:" && !failed.is_empty() {
                failed.sort_unstable();
                pending = failed.clone();
                in_outputs = true;
                open = false;
            } else if line.starts_with("test result:") {
                failed.clear();
            }
            continue;
        }
        if let Some(i) = line
            .strip_prefix("---- ")
            .and_then(|rest| rest.strip_suffix(" stdout ----"))
            .and_then(|name| pending.iter().position(|p| *p == name))
        {
            blocks.push((pending.remove(i), String::new()));
            open = true;
        } else if *line == "failures:" && closes(&lines[at + 1..], &failed) {
            in_outputs = false;
            open = false;
            failed.clear();
        } else if let (true, Some((_, output))) = (open, blocks.last_mut()) {
            output.push_str(line);
            output.push('\n');
        }
    }
    let mut found: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (name, output) in blocks {
        let output = output.trim().to_owned();
        found.entry(name.to_owned()).or_default().push(output);
    }
    found
}

/// Whether `rest`, the lines after a `failures:` line, is libtest's closing list for a target whose failing tests are
/// `failed` (sorted): one `    <name>` line for each, then a blank line and the `test result:` line.
fn closes(rest: &[&str], failed: &[&str]) -> bool {
    let n = failed.len();
    rest.len() > n + 1
        && rest[..n]
            .iter()
            .zip(failed)
            .all(|(line, name)| line.strip_prefix("    ") == Some(*name))
        && rest[n].is_empty()
        && rest[n + 1].starts_with("test result:")
}

/// The message of `finding`; for a control that did not make its test fail (`ControlPasses`, `WrongPanic`), followed
/// by the output of each control named for the test that libtest reports failing, by [`parse_outputs`] (R-236,
/// R-244), so the cause is kept.
pub fn describe(finding: &Finding, outputs: &BTreeMap<String, Vec<String>>) -> String {
    let (Finding::ControlPasses(test) | Finding::WrongPanic { test, .. }) = finding else {
        return finding.to_string();
    };
    let short = test.rsplit("::").next().unwrap_or(test);
    let mut message = finding.to_string();
    let mut shown = false;
    for (control, runs) in outputs.iter().filter(|(c, _)| control_of(c) == Some(short)) {
        for output in runs {
            message.push_str(&format!("\n--- output of control `{control}`:\n{output}"));
            shown = true;
        }
    }
    if !shown {
        message.push_str("\n--- libtest reported no output for its control");
    }
    message
}

/// Pairs each listed test with the controls named for it, by the test's last path segment (R-199, R-201), and
/// returns the tests with no control, with a control that `results` shows passing, or sharing their name with more
/// tests than there are controls of it (each listing of a test needs a listing of its own control). Every run of
/// every paired control is judged on its own: one leaky control fails its test, whatever a control of the same name
/// in another target does. A name listed in several targets is reported once. `Err` when a paired control has fewer
/// results than it was listed.
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
    // The tests by last path segment, in the order first listed: the distinct full names, and the listings.
    let mut groups: Vec<(&str, Vec<&String>, usize)> = Vec::new();
    for name in listed.iter().filter(|name| control_of(name).is_none()) {
        let short = name.rsplit("::").next().unwrap_or(name);
        match groups.iter_mut().find(|(of, _, _)| *of == short) {
            Some((_, names, listings)) => {
                *listings += 1;
                if !names.contains(&name) {
                    names.push(name);
                }
            }
            None => groups.push((short, vec![name], 1)),
        }
    }
    let mut found = Vec::new();
    for (short, names, listings) in groups {
        let paired: Vec<_> = controls.iter().filter(|(of, _, _)| *of == short).collect();
        let registered: usize = paired.iter().map(|(_, _, count)| count).sum();
        if registered == 0 {
            found.extend(names.iter().map(|name| Finding::NoControl((*name).clone())));
        } else if listings > registered {
            found.push(Finding::Collision {
                name: short.to_owned(),
                tests: names.iter().map(|name| (*name).clone()).collect(),
                listings,
                controls: registered,
            });
        }
        let mut leaks = false;
        for (_, control, count) in paired {
            let runs = results.get(*control).map_or(&[][..], Vec::as_slice);
            if runs.len() < *count {
                let tests = names.iter().map(|name| name.as_str()).collect::<Vec<_>>();
                return Err(format!(
                    "the control `{control}` of test `{}` did not run",
                    tests.join("`, `")
                ));
            }
            leaks |= runs.iter().any(|passed| !passed);
        }
        if leaks {
            found.extend(
                names
                    .iter()
                    .map(|name| Finding::ControlPasses((*name).clone())),
            );
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

/// What [`run`] does with each crate's controls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Runs every control and judges its run.
    Run,
    /// Lists each test with the controls named for it, and fails only on what the listing shows (a test with no
    /// control, a shared name, a doctest); runs no control (`--list`, R-226).
    List,
}

/// Each listed test with each control named for it (R-199, R-201), once per pair, sorted.
pub fn pairs(listed: &[String]) -> BTreeSet<(&str, &str)> {
    let mut found = BTreeSet::new();
    for test in listed.iter().filter(|name| control_of(name).is_none()) {
        let short = test.rsplit("::").next().unwrap_or(test);
        for control in listed.iter().filter(|name| control_of(name) == Some(short)) {
            found.insert((test.as_str(), control.as_str()));
        }
    }
    found
}

/// Results in which every listed control made its test fail on each listing, so that [`findings`] reports only what
/// the listing shows (`--list`).
pub fn as_tripped(listed: &[String]) -> BTreeMap<String, Vec<bool>> {
    let mut results: BTreeMap<String, Vec<bool>> = BTreeMap::new();
    for control in listed.iter().filter(|name| control_of(name).is_some()) {
        results.entry(control.clone()).or_default().push(true);
    }
    results
}

/// Runs the check in `mode` on every member of the workspace of `manifest`; `Err` if any test fails it.
pub fn run(manifest: &Path, mode: Mode) -> Result<(), String> {
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
        let mut outputs = BTreeMap::new();
        let mut found = match mode {
            Mode::Run => {
                // Runs the tests whose names contain `negative_control`: every control, and none when there are none.
                let output = cargo_test(manifest, name, "--tests", &[CONTROL_FN])?;
                let stdout = String::from_utf8_lossy(&output.stdout);
                let found = findings(&listed, &parse_results(&stdout))
                    .map_err(|e| format!("{name}: {e}"))?;
                outputs = parse_outputs(&stdout);
                name_wrong_panics(found, &parse_wrong_panics(&stdout))
            }
            Mode::List => {
                for (test, control) in pairs(&listed) {
                    println!("xtask controls: {name}: test `{test}`: control `{control}`");
                }
                findings(&listed, &as_tripped(&listed)).map_err(|e| format!("{name}: {e}"))?
            }
        };
        found.extend(doctests.into_iter().map(Finding::Doctest));
        for finding in &found {
            eprintln!("xtask controls: {name}: {}", describe(finding, &outputs));
        }
        if found.is_empty() {
            match mode {
                Mode::Run => {
                    println!("xtask controls: {name}: {tests} test(s), each failed by its control")
                }
                Mode::List => println!(
                    "xtask controls: {name}: {tests} test(s), each with a control; none run (--list)"
                ),
            }
        }
        failed += found.iter().map(Finding::tests).sum::<usize>();
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The output of the control of `tests::t`: a child's libtest report, with its own list of failures (as a control
    /// embedding a child's `cargo test` report prints), then its panic without its expected message.
    const OUTPUT: &str = "running 1 test\ntest c::negative_control ... FAILED\n\nfailures:\n\n\
                          ---- c::negative_control stdout ----\nchild output\n\nfailures:\n    \
                          c::negative_control\n\ntest result: FAILED. 0 passed; 1 failed\n\n\
                          thread 't::negative_control' panicked at tests/t.rs:6:36:\nsetup failed\n\
                          note: panic did not contain expected string\n      panic message: \"setup failed\"\n \
                          expected substring: \"the check\"";

    /// libtest's report of a run in which the control of `tests::t` printed [`OUTPUT`].
    fn run() -> String {
        format!(
            "running 2 tests\ntest tests::t ... ok\ntest t::negative_control - should panic ... FAILED\n\n\
             failures:\n\n---- t::negative_control stdout ----\n{OUTPUT}\n\nfailures:\n    \
             t::negative_control\n\ntest result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered \
             out; finished in 0.01s\n"
        )
    }

    /// R-236, R-244: each finding of a control that did not make its test fail carries that control's output, all of
    /// it up to libtest's own list of failures, and no longer says it "leaves it passing".
    fn check_output_kept(stdout: &str) {
        let output = format!("--- output of control `t::negative_control`:\n{OUTPUT}");
        let test = "tests::t".to_owned();
        let note = "the note".to_owned();
        for finding in [
            Finding::ControlPasses(test.clone()),
            Finding::WrongPanic { test, note },
        ] {
            let message = describe(&finding, &parse_outputs(stdout));
            assert!(
                message.ends_with(&output),
                "the control's output is not in the message:\n{message}"
            );
            assert!(!message.contains("leaves it passing"), "{message}");
        }
    }

    #[test]
    fn controls_finding_keeps_the_control_output() {
        check_output_kept(&run());
    }

    validation::negative_control!(
        controls_finding_keeps_the_control_output,
        "the same run with the control's header renamed, so no output is the control's",
        expected = "the control's output is not in the message",
        check_output_kept(&run().replace("---- t::", "---- u::"))
    );
}
