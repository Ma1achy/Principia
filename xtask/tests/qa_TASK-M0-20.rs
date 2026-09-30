//! QA tests for TASK-M0-20, written from REQ-TOOL-134, R-68, R-129, R-110 and R-177, not from the implementation:
//!
//! - "The GUI screenshot runner (`cargo xtask screenshot <suite>`) must exist before the first GUI screenshot
//!   requirement: it captures a GUI surface headless beside the artboard it names for layout comparison only (R-68),
//!   or asserts a presence-only case's named controls where no artboard exists (R-129)."
//! - verify: "cargo xtask screenshot selftest: the layout case writes its capture; the presence-only case passes, and
//!   fails naming the control when one is removed".
//!
//! Two surfaces: the whole `cargo xtask screenshot` binary on the checked-in suite, and `screenshot::run_suite` /
//! `screenshot::run` on hand-built suites under `CARGO_TARGET_TMPDIR`. PIT-3 ("check the measurement can fire"): a
//! presence case naming nothing cannot fail, and the capture must follow the surface it claims to show.
//!
//! Each test's control (R-176) feeds the same assertion an input differing in the one respect the requirement turns
//! on, and trips the assertion by its message.
// The file name `qa_TASK-M0-20` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};
use validation::negative_control;
use xtask::screenshot::{self, Outcome, Which};

const SUITE: &str = "qa";

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// A root under `CARGO_TARGET_TMPDIR` holding suite `qa` (and any `extra` suites), with the given `cases.json` and
/// one surface `surface.json` per `(file, surface)`, and a copy of the selftest reference at `art/reference.png`.
/// A suite: `(name, cases.json, [(surface file, surface)])`.
type Suite<'a> = (&'a str, Value, &'a [(&'a str, Value)]);

fn root(name: &str, suites: &[Suite]) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("qa_m0_20_{name}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("art")).expect("art dir");
    fs::copy(
        repo().join("fixtures/screenshot/selftest/reference.png"),
        root.join("art/reference.png"),
    )
    .expect("reference copied");
    for (suite, cases, surfaces) in suites {
        let dir = root.join(screenshot::SUITES).join(suite);
        fs::create_dir_all(&dir).expect("suite dir");
        fs::write(dir.join("cases.json"), cases.to_string()).expect("cases written");
        for (file, surface) in *surfaces {
            fs::write(dir.join(file), surface.to_string()).expect("surface written");
        }
    }
    root
}

fn two_controls() -> Value {
    json!({"size": [240, 80], "controls": [
        {"kind": "button", "label": "Selftest button"},
        {"kind": "checkbox", "label": "Selftest checkbox"}
    ]})
}

fn surface_with(labels: &[&str]) -> Value {
    let controls: Vec<Value> = labels
        .iter()
        .map(|l| json!({"kind": "button", "label": l}))
        .collect();
    json!({"size": [240, 80], "controls": controls})
}

fn presence_suite(name: &str, listed: &[&str], surface: Value) -> PathBuf {
    root(
        name,
        &[(
            SUITE,
            json!({"cases": [{"name": "presence", "surface": "s.json", "controls": listed}]}),
            &[("s.json", surface)],
        )],
    )
}

fn only_result(root: &Path) -> Result<Outcome, String> {
    let mut results = screenshot::run_suite(root, SUITE).expect("suite read");
    assert_eq!(results.len(), 1, "one case expected");
    results.remove(0).result
}

// ---------------------------------------------------------------------------------------------------------------
// The binary, on the checked-in selftest suite.

fn xtask(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(args)
        .output()
        .expect("xtask ran")
}

/// `cargo xtask screenshot <suite>` exits 0 and the layout case's capture sits beside the reference it names.
fn check_cli_passes(suite: &str) {
    let out = xtask(&["screenshot", suite]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "cli screenshot failed: {suite}\n{stdout}\n{stderr}"
    );
    let dir = repo().join(screenshot::OUT).join(suite).join("layout");
    assert!(
        dir.join(screenshot::CAPTURE).is_file(),
        "no capture in {}",
        dir.display()
    );
    assert!(
        dir.join("reference.png").is_file(),
        "no reference beside the capture"
    );
    assert!(stdout.contains("Selftest button") && stdout.contains("Selftest checkbox"));
}

#[test]
fn qa_cli_screenshot_selftest_passes_and_writes_capture() {
    check_cli_passes("selftest");
}

negative_control!(
    qa_cli_screenshot_selftest_passes_and_writes_capture,
    "a suite that does not exist must fail the command",
    expected = "cli screenshot failed",
    check_cli_passes("qa_no_such_suite")
);

/// An unknown suite fails, naming the suite.
fn check_cli_unknown_suite_fails(suite: &str) {
    let out = xtask(&["screenshot", suite]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !out.status.success() && stderr.contains(suite),
        "unknown suite not refused by name: {stderr}"
    );
}

#[test]
fn qa_cli_unknown_suite_fails_naming_it() {
    check_cli_unknown_suite_fails("qa_no_such_suite");
}

negative_control!(
    qa_cli_unknown_suite_fails_naming_it,
    "the real suite passes, so the refusal check must fail",
    expected = "unknown suite not refused by name",
    check_cli_unknown_suite_fails("selftest")
);

// ---------------------------------------------------------------------------------------------------------------
// Presence-only cases (R-129).

/// Removing *either* control fails the case naming exactly that control (the implementer's test removes only the
/// checkbox).
fn check_names_removed(root: &Path, removed: &str, kept: &str) {
    let result = only_result(root);
    let Err(message) = &result else {
        panic!("presence case did not fail: {result:?}");
    };
    assert!(
        message.contains(removed) && !message.contains(&format!("`{kept}`")),
        "failure does not name the removed control: {message}"
    );
}

#[test]
fn qa_presence_fails_naming_the_removed_button() {
    let root = presence_suite(
        "removed_button",
        &["Selftest button", "Selftest checkbox"],
        json!({"size": [240, 80], "controls": [{"kind": "checkbox", "label": "Selftest checkbox"}]}),
    );
    check_names_removed(&root, "Selftest button", "Selftest checkbox");
}

negative_control!(
    qa_presence_fails_naming_the_removed_button,
    "with both controls present the case passes",
    expected = "presence case did not fail",
    check_names_removed(
        &presence_suite(
            "removed_button_control",
            &["Selftest button", "Selftest checkbox"],
            two_controls()
        ),
        "Selftest button",
        "Selftest checkbox"
    )
);

/// Both removed: both named.
fn check_names_both(root: &Path) {
    let result = only_result(root);
    let message = result.err().unwrap_or_default();
    assert!(
        message.contains("Selftest button") && message.contains("Selftest checkbox"),
        "not every absent control is named: {message:?}"
    );
}

#[test]
fn qa_presence_names_every_absent_control() {
    check_names_both(&presence_suite(
        "both_removed",
        &["Selftest button", "Selftest checkbox"],
        surface_with(&["Something else"]),
    ));
}

negative_control!(
    qa_presence_names_every_absent_control,
    "with only the checkbox removed the button is not named",
    expected = "not every absent control is named",
    check_names_both(&presence_suite(
        "both_removed_control",
        &["Selftest button", "Selftest checkbox"],
        surface_with(&["Selftest button"]),
    ))
);

/// A named control is matched by its name, not by a prefix of another's.
fn check_prefix_is_absent(root: &Path) {
    assert!(
        only_result(root).is_err(),
        "a control present only as part of another's name was counted present"
    );
}

#[test]
fn qa_presence_matches_whole_names() {
    check_prefix_is_absent(&presence_suite("prefix", &["Selftest"], two_controls()));
}

negative_control!(
    qa_presence_matches_whole_names,
    "a control with exactly that name is present",
    expected = "counted present",
    check_prefix_is_absent(&presence_suite(
        "prefix_control",
        &["Selftest"],
        surface_with(&["Selftest"]),
    ))
);

/// PIT-3: a presence-only case that names no control asserts nothing and cannot fail; the runner must refuse it.
fn check_empty_presence_refused(root: &Path) {
    assert!(
        only_result(root).is_err(),
        "a presence-only case naming no control passed: it can never fail"
    );
}

#[test]
fn qa_presence_case_naming_nothing_is_refused() {
    check_empty_presence_refused(&presence_suite("empty", &[], two_controls()));
}

negative_control!(
    qa_presence_case_naming_nothing_is_refused,
    "a case naming a control its surface lacks fails, as a refused case does",
    expected = "can never fail",
    check_empty_presence_refused(&presence_suite(
        "empty_control",
        &["Selftest button"],
        two_controls()
    ))
);

// ---------------------------------------------------------------------------------------------------------------
// Layout cases (R-68): a capture beside the artboard, no pixel comparison.

fn layout_suite(name: &str, artboard: &str, surface: Value) -> PathBuf {
    root(
        name,
        &[(
            SUITE,
            json!({"cases": [{"name": "layout", "surface": "s.json", "artboard": artboard}]}),
            &[("s.json", surface)],
        )],
    )
}

fn capture_of(root: &Path) -> ([u32; 2], Vec<u8>) {
    match only_result(root) {
        Ok(Outcome::Captured { capture, .. }) => {
            screenshot::read_png(&capture).expect("capture decoded")
        }
        other => panic!("layout case did not capture: {other:?}"),
    }
}

/// R-68: the artboard is illustrative, so a capture that looks nothing like it still passes. The artboard here is a
/// different size and different content from the surface (the selftest reference is 240x80 of two controls; the
/// surface is 400x150 with one control).
fn check_artboard_not_compared(root: &Path) {
    let (size, _) = capture_of(root);
    assert_eq!(size, [400, 150], "capture is not the surface's size");
}

#[test]
fn qa_layout_compares_no_pixels() {
    let surface = json!({"size": [400, 150], "controls": [{"kind": "button", "label": "Other"}]});
    check_artboard_not_compared(&layout_suite("no_pixels", "art/reference.png", surface));
}

negative_control!(
    qa_layout_compares_no_pixels,
    "an artboard that does not exist fails the case, so the capture check trips",
    expected = "layout case did not capture",
    {
        let surface =
            json!({"size": [400, 150], "controls": [{"kind": "button", "label": "Other"}]});
        check_artboard_not_compared(&layout_suite(
            "no_pixels_control",
            "art/missing.png",
            surface,
        ));
    }
);

/// A layout case names an artboard; one that does not exist fails the case, naming it.
fn check_missing_artboard_named(root: &Path) {
    let result = only_result(root);
    let message = result.as_ref().err().cloned().unwrap_or_default();
    assert!(
        message.contains("missing.png"),
        "a missing artboard is not refused by name: {result:?}"
    );
}

#[test]
fn qa_layout_missing_artboard_fails_naming_it() {
    check_missing_artboard_named(&layout_suite("missing", "art/missing.png", two_controls()));
}

negative_control!(
    qa_layout_missing_artboard_fails_naming_it,
    "an artboard that exists passes",
    expected = "not refused by name",
    check_missing_artboard_named(&layout_suite(
        "missing_control",
        "art/reference.png",
        two_controls()
    ))
);

/// The capture shows the surface: a surface with a control removed renders a different image.
fn check_capture_follows_surface(a: &Path, b: &Path) {
    let (sa, pa) = capture_of(a);
    let (sb, pb) = capture_of(b);
    assert_eq!(sa, sb);
    assert!(pa != pb, "captures of different surfaces are identical");
}

#[test]
fn qa_layout_capture_follows_its_surface() {
    check_capture_follows_surface(
        &layout_suite("follow_a", "art/reference.png", two_controls()),
        &layout_suite(
            "follow_b",
            "art/reference.png",
            surface_with(&["Selftest button"]),
        ),
    );
}

negative_control!(
    qa_layout_capture_follows_its_surface,
    "two captures of the same surface are identical",
    expected = "are identical",
    check_capture_follows_surface(
        &layout_suite("follow_ctl_a", "art/reference.png", two_controls()),
        &layout_suite("follow_ctl_b", "art/reference.png", two_controls()),
    )
);

// ---------------------------------------------------------------------------------------------------------------
// Case shape, --all, and the backend.

/// A case is exactly one of layout and presence-only.
fn check_case_shape_refused(name: &str, case: Value) {
    let root = root(
        name,
        &[(
            SUITE,
            json!({"cases": [case]}),
            &[("s.json", two_controls())],
        )],
    );
    assert!(only_result(&root).is_err(), "an ill-formed case passed");
}

#[test]
fn qa_case_with_both_or_neither_kind_is_refused() {
    check_case_shape_refused(
        "shape_both",
        json!({"name": "both", "surface": "s.json",
        "artboard": "art/reference.png", "controls": ["Selftest button"]}),
    );
    check_case_shape_refused(
        "shape_neither",
        json!({"name": "neither", "surface": "s.json"}),
    );
}

negative_control!(
    qa_case_with_both_or_neither_kind_is_refused,
    "a well-formed presence case passes",
    expected = "an ill-formed case passed",
    check_case_shape_refused(
        "shape_control",
        json!({"name": "ok", "surface": "s.json", "controls": ["Selftest button"]})
    )
);

/// `--all` runs every suite: a failing case in a second suite fails the whole run.
fn check_all_fails(root: &Path) {
    assert!(
        screenshot::run(root, Which::All).is_err(),
        "--all passed with a failing suite"
    );
}

fn all_root(name: &str, second_listed: &str) -> PathBuf {
    let presence =
        |l: &str| json!({"cases": [{"name": "p", "surface": "s.json", "controls": [l]}]});
    root(
        name,
        &[
            (
                "a_good",
                presence("Selftest button"),
                &[("s.json", two_controls())],
            ),
            (
                "z_second",
                presence(second_listed),
                &[("s.json", two_controls())],
            ),
        ],
    )
}

#[test]
fn qa_all_runs_every_suite() {
    check_all_fails(&all_root("all", "Absent control"));
}

negative_control!(
    qa_all_runs_every_suite,
    "with every suite passing, --all passes",
    expected = "--all passed with a failing suite",
    check_all_fails(&all_root("all_control", "Selftest checkbox"))
);

/// `PRIN_GPU_BACKEND` names metal or vulkan; anything else is refused naming the variable.
fn check_backend_refused(value: &str) {
    let err = screenshot::backend(Some(value)).err().unwrap_or_default();
    assert!(
        err.contains(screenshot::BACKEND_VAR),
        "backend {value:?} not refused"
    );
}

#[test]
fn qa_unknown_backend_is_refused() {
    check_backend_refused("cuda");
    assert!(screenshot::backend(Some("metal")).is_ok());
    assert!(screenshot::backend(Some("vulkan")).is_ok());
}

negative_control!(
    qa_unknown_backend_is_refused,
    "vulkan is a backend",
    expected = "not refused",
    check_backend_refused("vulkan")
);

// ---------------------------------------------------------------------------------------------------------------
// Where it runs (R-110, R-177): GUI PRs, not the per-commit ci.

fn check_workflow(text: &str) {
    for needle in [
        "pull_request",
        "\"crates/gui/**\"",
        "\"docs/gui/**\"",
        "cargo xtask screenshot --all",
    ] {
        assert!(text.contains(needle), "screenshot.yml lacks {needle}");
    }
    assert!(
        !text.contains("push:"),
        "screenshot.yml runs on every push, not on GUI PRs"
    );
}

#[test]
fn qa_screenshot_runs_on_gui_prs_not_in_ci() {
    let text =
        fs::read_to_string(repo().join(".github/workflows/screenshot.yml")).expect("workflow");
    check_workflow(&text);
    assert!(
        xtask::ci::RUNNERS
            .iter()
            .all(|r| !r.name.contains("screenshot")),
        "screenshot is registered in the per-commit ci"
    );
}

negative_control!(
    qa_screenshot_runs_on_gui_prs_not_in_ci,
    "a workflow on every push, without the GUI paths, fails the check",
    expected = "screenshot.yml lacks",
    check_workflow(
        "on:\n  push:\njobs:\n  x:\n    steps:\n      - run: cargo xtask screenshot --all\n"
    )
);

/// `cargo xtask screenshot --all`, the command the GUI-PR workflow and the gates run, runs the checked-in suites.
fn check_cli_all(args: &[&str]) {
    let out = xtask(args);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success()
            && stdout.contains("selftest/layout")
            && stdout.contains("selftest/presence"),
        "cli --all did not run the selftest suite: {stdout}{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn qa_cli_screenshot_all_runs_selftest() {
    check_cli_all(&["screenshot", "--all"]);
}

negative_control!(
    qa_cli_screenshot_all_runs_selftest,
    "a flag the runner does not know is refused, running nothing",
    expected = "cli --all did not run the selftest suite",
    check_cli_all(&["screenshot", "--every"])
);
