//! `cargo xtask screenshot selftest` (REQ-TOOL-134): the layout case writes its capture beside its reference; the
//! presence-only case passes, and fails naming the control when one control is removed (R-68, R-129). Each test runs
//! on a copy of `fixtures/screenshot/selftest/` under its own root, so the captures land in that root's `target/`.

use std::fs;
use std::path::{Path, PathBuf};

use validation::negative_control;
use xtask::screenshot::{self, CaseResult, Outcome};

const SUITE: &str = "selftest";

/// The repo's selftest suite directory.
fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(screenshot::SUITES)
        .join(SUITE)
}

/// A root holding a copy of the selftest suite, its surface's controls filtered by `keep` (by label).
fn root(name: &str, keep: impl Fn(&str) -> bool) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&root);
    let dir = root.join(screenshot::SUITES).join(SUITE);
    fs::create_dir_all(&dir).expect("suite dir created");
    for file in ["cases.json", "reference.png"] {
        fs::copy(fixture().join(file), dir.join(file)).expect("fixture copied");
    }
    let text = fs::read_to_string(fixture().join("surface.json")).expect("surface read");
    let mut surface: serde_json::Value = serde_json::from_str(&text).expect("surface parsed");
    surface["controls"]
        .as_array_mut()
        .expect("controls")
        .retain(|c| keep(c["label"].as_str().expect("label")));
    fs::write(dir.join("surface.json"), surface.to_string()).expect("surface written");
    root
}

/// The result of the case `case` of the selftest suite under `root`.
fn case(root: &Path, case: &str) -> CaseResult {
    screenshot::run_suite(root, SUITE)
        .expect("suite read")
        .into_iter()
        .find(|r| r.case == case)
        .unwrap_or_else(|| panic!("no case `{case}`"))
}

/// The layout case's capture under `root`, after checking it is of the surface's size and sits beside a
/// byte-for-byte copy of the reference it names.
fn capture(root: &Path) -> Vec<u8> {
    let result = case(root, "layout");
    let Ok(Outcome::Captured { capture, artboard }) = &result.result else {
        panic!("layout case did not capture: {result}");
    };
    assert_eq!(
        capture.parent(),
        artboard.parent(),
        "capture not beside its reference"
    );
    assert!(capture.starts_with(root.join(screenshot::OUT)));
    assert_eq!(
        fs::read(artboard).expect("artboard copy"),
        fs::read(fixture().join("reference.png")).expect("reference"),
        "the copy beside the capture is not the reference"
    );
    let (size, rgba) = screenshot::read_png(capture).expect("capture decoded");
    assert_eq!(size, [240, 80], "capture not the surface's size");
    rgba
}

/// The layout case under `root` wrote its capture beside its reference, and the capture shows the surface's controls:
/// it differs from the capture under `other`, whose surface lacks one of them. Two captures on one backend are
/// compared, so the check holds on every backend (an empty panel need not render uniform, as on llvmpipe).
fn check_capture(root: &Path, other: &Path) {
    assert!(
        capture(root) != capture(other),
        "capture is identical to a surface lacking a control: the surface's controls were not drawn"
    );
}

#[test]
fn screenshot_selftest_layout_writes_capture_beside_reference() {
    check_capture(
        &root("shot_layout", |_| true),
        &root("shot_layout_other", |l| l != "Selftest checkbox"),
    );
}

negative_control!(
    screenshot_selftest_layout_writes_capture_beside_reference,
    "two captures of the same surface are identical, which the capture check must reject",
    expected = "capture is identical",
    check_capture(
        &root("shot_layout_control", |_| true),
        &root("shot_layout_control_other", |_| true),
    )
);

/// The presence-only case under `root` passes, finding both controls.
fn check_presence_passes(root: &Path) {
    let result = case(root, "presence");
    assert_eq!(
        result.result,
        Ok(Outcome::Present {
            controls: vec!["Selftest button".into(), "Selftest checkbox".into()]
        }),
        "presence case failed: {result}"
    );
}

#[test]
fn screenshot_selftest_presence_passes() {
    check_presence_passes(&root("shot_presence", |_| true));
}

negative_control!(
    screenshot_selftest_presence_passes,
    "a surface lacking a listed control must fail the presence case",
    expected = "presence case failed",
    check_presence_passes(&root("shot_presence_control", |l| l != "Selftest button"))
);

/// The presence-only case under `root` fails, naming `Selftest checkbox` and no other control.
fn check_presence_names_removed(root: &Path) {
    let result = case(root, "presence");
    let Err(message) = &result.result else {
        panic!("presence case did not fail: {result}");
    };
    assert!(
        message.contains("`Selftest checkbox`") && !message.contains("`Selftest button`"),
        "failure does not name exactly the removed control: {message}"
    );
}

#[test]
fn screenshot_presence_fails_naming_removed_control() {
    check_presence_names_removed(&root("shot_removed", |l| l != "Selftest checkbox"));
}

negative_control!(
    screenshot_presence_fails_naming_removed_control,
    "with both controls present, the presence case passes, so the removal check must fail",
    expected = "presence case did not fail",
    check_presence_names_removed(&root("shot_removed_control", |_| true))
);

/// `screenshot::run` over `root` fails, and so the CLI exits non-zero, when a case fails.
fn check_run_fails(root: &Path) {
    let outcome = screenshot::run(root, screenshot::Which::One(SUITE));
    assert!(
        outcome.is_err(),
        "run passed with a failing case: {outcome:?}"
    );
}

#[test]
fn screenshot_run_fails_on_a_failing_case() {
    check_run_fails(&root("shot_run", |l| l != "Selftest button"));
}

negative_control!(
    screenshot_run_fails_on_a_failing_case,
    "with every case passing, run must pass, so the failure check must fail",
    expected = "run passed with a failing case",
    check_run_fails(&root("shot_run_control", |_| true))
);

/// `--all` finds the selftest suite in the repo.
fn check_listed(root: &Path) {
    let suites = screenshot::list_suites(root).expect("suites listed");
    assert!(
        suites.iter().any(|s| s == SUITE),
        "selftest not among the suites: {suites:?}"
    );
}

#[test]
fn screenshot_all_lists_selftest() {
    check_listed(&Path::new(env!("CARGO_MANIFEST_DIR")).join(".."));
}

negative_control!(
    screenshot_all_lists_selftest,
    "a root whose suite has no cases.json lists no selftest",
    expected = "selftest not among the suites",
    {
        let root = root("shot_list_control", |_| true);
        fs::remove_file(root.join(screenshot::SUITES).join(SUITE).join("cases.json"))
            .expect("cases.json removed");
        check_listed(&root);
    }
);

/// `screenshot --all` over `root` fails, naming the missing suites, when it finds no suite: a run of nothing must not
/// pass (PIT-3).
fn check_all_with_no_suite_fails(root: &Path) {
    let outcome = screenshot::run(root, screenshot::Which::All);
    assert!(
        outcome.as_ref().is_err_and(|e| e.contains("no suite")),
        "--all found no suite to run and did not fail: {outcome:?}"
    );
}

#[test]
fn screenshot_all_with_no_suite_fails() {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("shot_all_empty");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join(screenshot::SUITES)).expect("empty suites dir created");
    check_all_with_no_suite_fails(&root);
}

negative_control!(
    screenshot_all_with_no_suite_fails,
    "with a passing suite present, --all passes, so the no-suite check must fail",
    expected = "did not fail",
    check_all_with_no_suite_fails(&root("shot_all_empty_control", |_| true))
);

/// `cargo xtask screenshot <arg>` exits 2, the usage error, so a flag is never taken for a suite name.
fn check_flag_is_usage_error(arg: &str) {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["screenshot", arg])
        .output()
        .expect("xtask ran");
    assert_eq!(
        out.status.code(),
        Some(2),
        "`screenshot {arg}` is not a usage error: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn screenshot_flag_is_not_a_suite() {
    check_flag_is_usage_error("--bogus");
}

negative_control!(
    screenshot_flag_is_not_a_suite,
    "a suite name that is not a flag runs as a suite, and an unknown one exits 1, not 2",
    expected = "is not a usage error",
    check_flag_is_usage_error("no_such_suite")
);
