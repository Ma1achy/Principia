//! QA tests for TASK-M0-25, written from REQ-VAL-153 ("every qa test in `crates/validation` and `crates/prin` merged
//! before TASK-M0-22 must have a registered negative control [...]; a subprocess body a test spawns must not be a
//! `#[test]`"), R-210 ("the three qa child-mode helpers [...] move there") and the task's review line ("the moved
//! child bodies keep what the parent tests read"). Each test registers a negative control (R-176, R-199).

use std::path::{Path, PathBuf};
use std::process::Command;
use validation::negative_control;
use validation::spawn::Spawn;

/// The qa tests merged before TASK-M0-22, per crate, from the files the task names (22 at R-209): prin's, since R-909
/// retired the test that listed validation's.
const QA_TESTS: &[(&str, &[&str])] = &[
    (
        "prin",
        &[
            "qa_prin_help_succeeds_and_prints_usage",
            "qa_prin_refuses_an_unknown_argument",
        ],
    ),
];

/// The three child bodies R-210 names; none may be a `#[test]`.
const CHILD_BODIES: &[&str] = &[
    "qa_child_open_harness",
    "qa_child_failing_property",
    "qa_child_count_cases",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// The listing's own target directory (R-224). A cargo run into the workspace's target directory replaces
/// `target/debug/qa_child` (removes it, then copies the build back) even when nothing is rebuilt, and the moved-property
/// test spawns that file meanwhile: it found no file (`NotFound`) under load. Here no listing writes that file.
fn target_dir() -> PathBuf {
    Path::new(env!("CARGO_TARGET_TMPDIR")).join("qa_m0_25-listing")
}

/// The names libtest lists for every test target of `krate` (`cargo test --tests -- --list`), with or without the
/// `controls` feature.
fn listed(krate: &str, controls: bool) -> Vec<String> {
    let mut cmd = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned()));
    cmd.current_dir(root())
        .args(["test", "-q", "-p", krate, "--tests"])
        .env("CARGO_TARGET_DIR", target_dir());
    if controls {
        cmd.args(["--features", "controls"]);
    }
    let o = cmd
        .args(["--", "--list", "--format", "terse"])
        .timed_output()
        .expect("run cargo test --list");
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(
        o.status.success(),
        "cargo test -p {krate} --list failed:\n{out}\n{}",
        String::from_utf8_lossy(&o.stderr)
    );
    out.lines()
        .filter_map(|l| l.strip_suffix(": test"))
        .map(str::to_owned)
        .collect()
}

/// REQ-VAL-153 on one crate's listing: each qa test is listed, a control named for it is listed, and no child body
/// is listed.
fn check_paired(krate: &str, names: &[String], qa: &[&str]) {
    for body in CHILD_BODIES {
        assert!(
            !names.iter().any(|n| n.ends_with(body)),
            "{krate}: the child body {body} is still a #[test]"
        );
    }
    for test in qa {
        assert!(
            names.iter().any(|n| n == test),
            "{krate}: the qa test {test} is not listed"
        );
        let control = format!("{test}::negative_control");
        assert!(
            names.contains(&control),
            "{krate}: the qa test {test} has no registered control"
        );
    }
}

/// The qa tests of `krate` in [`QA_TESTS`].
fn qa_tests(krate: &str) -> &'static [&'static str] {
    QA_TESTS
        .iter()
        .find(|(k, _)| *k == krate)
        .unwrap_or_else(|| panic!("QA_TESTS has no crate {krate}"))
        .1
}

/// REQ-VAL-153 on prin: under the `controls` feature, every qa test merged before TASK-M0-22 in prin has a control
/// registered under its name, and none of R-210's three child bodies is a test. Split from validation's (R-336), which
/// R-909 retired.
#[test]
fn qa_m0_25_prin_every_merged_qa_test_has_a_control_and_no_child_body_is_a_test() {
    check_paired("prin", &listed("prin", true), qa_tests("prin"));
}

negative_control!(
    qa_m0_25_prin_every_merged_qa_test_has_a_control_and_no_child_body_is_a_test,
    "prin's listing without the feature, where no control compiles, so the pairing check must fail on it",
    expected = "prin: the qa test qa_prin_help_succeeds_and_prints_usage has no registered control",
    check_paired("prin", &listed("prin", false), qa_tests("prin"))
);

/// The first failing draw the `failing_property` body prints, run with `PROPTEST_RNG_SEED=seed`.
fn first_failing_draw(seed: &str) -> u32 {
    let o = Command::new(env!("CARGO_BIN_EXE_qa_child"))
        .arg("failing_property")
        .env("PROPTEST_RNG_SEED", seed)
        .timed_output()
        .expect("run qa_child");
    let t = format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    assert!(!o.status.success(), "the failing property passed: {t}");
    let at = t
        .find("QA_FIRST_DRAW Some(")
        .unwrap_or_else(|| panic!("no failing draw printed: {t}"));
    t[at + "QA_FIRST_DRAW Some(".len()..]
        .split(')')
        .next()
        .and_then(|d| d.parse().ok())
        .unwrap_or_else(|| panic!("the failing draw is not a u32: {t}"))
}

/// The property moved from `qa_TASK-M0-04.rs` fails exactly on draws with `x % 7 == 3`.
fn check_fails_as_moved(draw: u32) {
    assert_eq!(
        draw % 7,
        3,
        "the reported failing draw {draw} is not one the moved property (x % 7 == 3) fails on"
    );
}

/// R-210 and the task's review line: the moved `failing_property` body keeps what its parent test reads. The first
/// failing draw it prints is one the property it was moved from fails on (`x % 7 == 3`, one draw in seven), so the
/// parent's seed test reproduces a case reached after passing draws, not merely the first draw.
#[test]
fn qa_m0_25_moved_failing_property_reports_a_draw_the_property_fails_on() {
    for seed in ["1", "2", "3", "12345", "18446744073709551615"] {
        check_fails_as_moved(first_failing_draw(seed));
    }
}

negative_control!(
    qa_m0_25_moved_failing_property_reports_a_draw_the_property_fails_on,
    "the draw after the reported one is not 3 mod 7, so the check must fail on it",
    expected = "is not one the moved property (x % 7 == 3) fails on",
    check_fails_as_moved(first_failing_draw("1").wrapping_add(1))
);
