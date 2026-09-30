//! QA tests for TASK-M0-40, written from REQ-TOOL-136 and R-275, not from the implementation:
//!
//! - "The screenshot runner's presence check must count a control only if it is in egui's tree and its rect
//!   intersects the visible surface; a control clipped out of the visible surface isn't present (R-275)."
//! - verify: "cargo test -p xtask screenshot: a control laid out below a 120×20 surface fails presence naming it; the
//!   same control inside the surface passes".
//!
//! Surfaces are hand-built suites under `CARGO_TARGET_TMPDIR`, run through `screenshot::run_suite` and
//! `screenshot::run` (the entry `cargo xtask screenshot <suite>` calls). The 120×20 size is the requirement's; the
//! other sizes only place a control wholly inside, wholly outside, or touching an edge of the surface, on either
//! axis ("intersects" is a two-axis relation). PIT-3: a clipped control must still be reported (the check can fire),
//! and a control clipped out of view must not be confused with one missing from the tree.
//!
//! Each test's control (R-176) feeds the same assertion an input differing in the one respect the requirement turns
//! on (where the control sits relative to the surface), and trips the assertion by its message.
// The file name `qa_TASK-M0-40` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::json;
use validation::negative_control;
use xtask::screenshot::{self, Outcome, Which};

const SUITE: &str = "qa40";

/// A root holding suite `qa40`: one surface of `size` holding the buttons `labels` in order (plus one checkbox
/// `checkbox`, if given, last), and one presence case `presence` listing `listed`.
fn root(
    name: &str,
    size: [u32; 2],
    labels: &[&str],
    checkbox: Option<&str>,
    listed: &[&str],
) -> PathBuf {
    // Unique per call: a test and its control run in parallel with the same `name`.
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("qa_m0_40_{name}_{}_{n}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let dir = root.join(screenshot::SUITES).join(SUITE);
    fs::create_dir_all(&dir).expect("suite dir created");
    let mut controls: Vec<_> = labels
        .iter()
        .map(|l| json!({ "kind": "button", "label": l }))
        .collect();
    if let Some(c) = checkbox {
        controls.push(json!({ "kind": "checkbox", "label": c }));
    }
    fs::write(
        dir.join("surface.json"),
        json!({ "size": size, "controls": controls }).to_string(),
    )
    .expect("surface written");
    fs::write(
        dir.join("cases.json"),
        json!({ "cases": [{ "name": "presence", "surface": "surface.json", "controls": listed }] })
            .to_string(),
    )
    .expect("cases written");
    root
}

/// The presence case's result under `root`.
fn presence(root: &Path) -> Result<Outcome, String> {
    let results = screenshot::run_suite(root, SUITE).expect("suite read");
    assert_eq!(results.len(), 1, "one case expected");
    results.into_iter().next().expect("one case").result
}

/// Twelve fillers: at any font size egui uses, twelve buttons stacked top-down pass the bottom of a 20-high surface.
const FILLERS: [&str; 12] = [
    "QA fill 01",
    "QA fill 02",
    "QA fill 03",
    "QA fill 04",
    "QA fill 05",
    "QA fill 06",
    "QA fill 07",
    "QA fill 08",
    "QA fill 09",
    "QA fill 10",
    "QA fill 11",
    "QA fill 12",
];

fn with_target_last(target: &str) -> Vec<&str> {
    let mut v = FILLERS.to_vec();
    v.push(target);
    v
}

fn with_target_first(target: &str) -> Vec<&str> {
    let mut v = vec![target];
    v.extend(FILLERS);
    v
}

/// The presence case fails, and its message names `control` (in backticks, as the runner names controls).
fn check_fails_naming(root: &Path, control: &str) -> String {
    let message = match presence(root) {
        Ok(o) => {
            panic!("presence passed though `{control}` lies outside the visible surface: {o:?}")
        }
        Err(m) => m,
    };
    assert!(
        message.contains(&format!("`{control}`")),
        "failure does not name `{control}`: {message}"
    );
    message
}

// --- REQ-TOOL-136's verify line: below a 120×20 surface fails naming it; inside passes. ---

/// A button far below a 120×20 surface fails presence, named, and named as clipped rather than absent: it is in the
/// tree (R-275 distinguishes the two).
fn check_below_120x20_fails(labels: &[&str]) {
    let r = root("below_120x20", [120, 20], labels, None, &["QA target"]);
    let message = check_fails_naming(&r, "QA target");
    assert!(
        message.contains("clipped"),
        "failure does not say the control is clipped: {message}"
    );
}

#[test]
fn screenshot_qa_control_below_120x20_surface_fails_named() {
    check_below_120x20_fails(&with_target_last("QA target"));
}

negative_control!(
    screenshot_qa_control_below_120x20_surface_fails_named,
    "the same control laid out first, at the top of the surface, is present, so the clipped check must fail",
    expected = "presence passed though",
    check_below_120x20_fails(&with_target_first("QA target"))
);

/// The same control, laid out first, is inside the 120×20 surface and passes.
fn check_inside_120x20_passes(labels: &[&str]) {
    let r = root("inside_120x20", [120, 20], labels, None, &["QA target"]);
    assert_eq!(
        presence(&r),
        Ok(Outcome::Present {
            controls: vec!["QA target".into()]
        }),
        "the control inside the surface is not present"
    );
}

#[test]
fn screenshot_qa_control_inside_120x20_surface_passes() {
    check_inside_120x20_passes(&with_target_first("QA target"));
}

negative_control!(
    screenshot_qa_control_inside_120x20_surface_passes,
    "the same control laid out last, below the surface, is clipped, so the inside check must fail",
    expected = "the control inside the surface is not present",
    check_inside_120x20_passes(&with_target_last("QA target"))
);

// --- The same control passes when the surface grows to show it: "visible" follows the surface, not the tree. ---

/// On a surface of `size`, the last of the fillers-plus-target stack is present.
fn check_last_present_at(size: [u32; 2]) {
    let r = root(
        "tall",
        size,
        &with_target_last("QA target"),
        None,
        &["QA target"],
    );
    assert!(
        presence(&r).is_ok(),
        "the last control on a {size:?} surface is not present: {:?}",
        presence(&r)
    );
}

#[test]
fn screenshot_qa_same_stack_passes_on_a_tall_surface() {
    // 13 buttons stacked top-down fit inside 120×2000 at any default egui font size.
    check_last_present_at([120, 2000]);
}

negative_control!(
    screenshot_qa_same_stack_passes_on_a_tall_surface,
    "the same stack on the 120×20 surface clips its last control, so the tall-surface check must fail",
    expected = "is not present",
    check_last_present_at([120, 20])
);

// --- Intersection is two-axis: a control right of a narrow surface is clipped too. ---

/// A 1-point-wide surface shows nothing of a button laid out after the panel's left margin; the button is clipped.
fn check_right_of_narrow_fails(size: [u32; 2]) {
    let r = root("narrow", size, &["QA target"], None, &["QA target"]);
    let message = check_fails_naming(&r, "QA target");
    assert!(
        message.contains("clipped"),
        "failure does not say the control is clipped: {message}"
    );
}

#[test]
fn screenshot_qa_control_right_of_narrow_surface_fails_named() {
    check_right_of_narrow_fails([1, 200]);
}

negative_control!(
    screenshot_qa_control_right_of_narrow_surface_fails_named,
    "the same button on a 120-wide surface is visible, so the narrow check must fail",
    expected = "presence passed though",
    check_right_of_narrow_fails([120, 200])
);

// --- A checkbox follows the same rule as a button. ---

fn check_checkbox_below_fails(labels: &[&str], checkbox: &str) {
    let r = root("checkbox", [120, 20], labels, Some(checkbox), &["QA box"]);
    check_fails_naming(&r, "QA box");
}

#[test]
fn screenshot_qa_checkbox_below_surface_fails_named() {
    check_checkbox_below_fails(&FILLERS, "QA box");
}

negative_control!(
    screenshot_qa_checkbox_below_surface_fails_named,
    "the checkbox alone on the surface is visible, so the clipped check must fail",
    expected = "presence passed though",
    check_checkbox_below_fails(&[], "QA box")
);

// --- Clipped and absent are reported apart; the visible controls are not named. ---

/// A case listing a visible control, a clipped one and one not on the surface fails naming the clipped one and the
/// absent one, each in its own part of the message, and not the visible one.
fn check_mixed(absent: &str) {
    let r = root(
        "mixed",
        [120, 20],
        &with_target_last("QA target"),
        None,
        &["QA fill 01", "QA target", absent],
    );
    let message = check_fails_naming(&r, "QA target");
    assert!(
        !message.contains("`QA fill 01`"),
        "the visible control is named as failing: {message}"
    );
    let clipped_at = message.find("clipped").expect("a clipped part");
    let absent_at = message
        .find("absent")
        .unwrap_or_else(|| panic!("no absent part: {message}"));
    let target_at = message.find("`QA target`").expect("target named");
    let ghost_at = message
        .find(&format!("`{absent}`"))
        .unwrap_or_else(|| panic!("the control not on the surface is not named: {message}"));
    // Each name sits after its own part's heading and not after the other's, whichever part comes first.
    let in_part = |at: usize, own: usize, other: usize| at > own && (other < own || at < other);
    assert!(
        in_part(target_at, clipped_at, absent_at),
        "`QA target` is not in the clipped part: {message}"
    );
    assert!(
        in_part(ghost_at, absent_at, clipped_at),
        "`{absent}` is not in the absent part: {message}"
    );
}

#[test]
fn screenshot_qa_clipped_and_absent_reported_apart() {
    check_mixed("QA ghost");
}

negative_control!(
    screenshot_qa_clipped_and_absent_reported_apart,
    "listing a filler that is on the surface but clipped, as the 'absent' one, puts it in the clipped part",
    expected = "no absent part",
    check_mixed("QA fill 12")
);

// --- Through `screenshot::run` (the command's entry): a clipped control fails the command. ---

fn check_run_fails_naming(labels: &[&str]) {
    let r = root("run", [120, 20], labels, None, &["QA target"]);
    let err = match screenshot::run(&r, Which::One(SUITE)) {
        Ok(()) => panic!("screenshot run passed with the control clipped"),
        Err(e) => e,
    };
    assert!(
        err.contains("1 of 1"),
        "run's error does not count the failed case: {err}"
    );
}

#[test]
fn screenshot_qa_run_fails_on_clipped_control() {
    check_run_fails_naming(&with_target_last("QA target"));
}

negative_control!(
    screenshot_qa_run_fails_on_clipped_control,
    "with the control visible the run passes, so the run check must fail",
    expected = "screenshot run passed",
    check_run_fails_naming(&with_target_first("QA target"))
);

// --- A rect that only touches the surface's edge shows no pixel of it: not present. ---
//
// The surface renders at one pixel per point (`screenshot::Surface`), so a control whose rect meets the surface only
// along an edge puts no pixel on it: it is clipped out of the visible surface (R-275). The control's rect is measured
// by laying the surface out directly in egui, as the surface description defines it (a central panel of the given
// size holding the controls top-down), not read from the runner.

/// The rect of `target` in a surface of `size` holding the buttons `labels`.
fn rect_of(size: [u32; 2], labels: &[&str], target: &str) -> egui::Rect {
    let ctx = egui::Context::default();
    let screen =
        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(size[0] as f32, size[1] as f32));
    let mut found = None;
    for _ in 0..2 {
        let input = egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                for label in labels {
                    let response = ui.button(*label);
                    if *label == target {
                        found = Some(response.rect);
                    }
                }
            });
        });
        // Textures are not drawn here; egui asserts an unapplied delta is cleared, not dropped.
        output.textures_delta.clear();
    }
    found.expect("target laid out")
}

/// A whole number of points, as an edge must be to coincide with an integer surface size.
fn whole(v: f32, what: &str) -> u32 {
    assert!(
        v.fract() == 0.0 && v > 0.0,
        "{what} is {v}, not a whole positive number of points: the touching-edge surface cannot be built"
    );
    v as u32
}

/// On a 120-wide surface whose height is `top + extra`, where `top` is the second button's top edge, the second
/// button is present exactly when `extra` > 0 (at `extra` = 0 it only touches the bottom edge).
fn check_bottom_edge(extra: u32) {
    let labels = ["QA edge fill", "QA edge target"];
    let top = whole(
        rect_of([120, 400], &labels, "QA edge target").min.y,
        "the target's top",
    );
    let size = [120, top + extra];
    assert_eq!(
        rect_of(size, &labels, "QA edge target").min.y,
        top as f32,
        "the target moved with the surface's height"
    );
    let r = root("bottom_edge", size, &labels, None, &["QA edge target"]);
    let present = presence(&r).is_ok();
    assert_eq!(
        present,
        extra > 0,
        "touching-edge rule broken on {size:?} (target top {top}): present = {present}: {:?}",
        presence(&r)
    );
}

#[test]
fn screenshot_qa_control_touching_bottom_edge_is_clipped() {
    check_bottom_edge(0);
}

#[test]
fn screenshot_qa_control_one_point_over_bottom_edge_is_present() {
    check_bottom_edge(1);
}

negative_control!(
    screenshot_qa_control_touching_bottom_edge_is_clipped,
    "asserting the touching control present must fail, as it is clipped",
    expected = "touching control not present",
    {
        let labels = ["QA edge fill", "QA edge target"];
        let top = whole(
            rect_of([120, 400], &labels, "QA edge target").min.y,
            "the target's top",
        );
        let r = root(
            "bottom_edge_inv",
            [120, top],
            &labels,
            None,
            &["QA edge target"],
        );
        assert!(presence(&r).is_ok(), "touching control not present");
    }
);

negative_control!(
    screenshot_qa_control_one_point_over_bottom_edge_is_present,
    "a surface one point shorter leaves the target touching the edge, so the present check must fail",
    expected = "touching-edge rule broken",
    {
        let labels = ["QA edge fill", "QA edge target"];
        let top = whole(rect_of([120, 400], &labels, "QA edge target").min.y, "the target's top");
        let r = root("bottom_edge_short", [120, top], &labels, None, &["QA edge target"]);
        let present = presence(&r).is_ok();
        assert!(present, "touching-edge rule broken: a touching control is absent from the present check");
    }
);

/// On a surface of height 200 whose width is the button's left edge plus `extra`, the button is present exactly when
/// `extra` > 0 (at 0 it only touches the right edge).
fn check_right_edge(extra: u32) {
    let labels = ["QA edge target"];
    let left = whole(
        rect_of([400, 200], &labels, "QA edge target").min.x,
        "the target's left",
    );
    let size = [left + extra, 200];
    assert_eq!(
        rect_of(size, &labels, "QA edge target").min.x,
        left as f32,
        "the target moved with the surface's width"
    );
    let r = root("right_edge", size, &labels, None, &["QA edge target"]);
    let present = presence(&r).is_ok();
    assert_eq!(
        present,
        extra > 0,
        "touching-edge rule broken on {size:?} (target left {left}): present = {present}: {:?}",
        presence(&r)
    );
}

#[test]
fn screenshot_qa_control_touching_right_edge_is_clipped() {
    check_right_edge(0);
}

#[test]
fn screenshot_qa_control_one_point_over_right_edge_is_present() {
    check_right_edge(1);
}

negative_control!(
    screenshot_qa_control_touching_right_edge_is_clipped,
    "asserting the touching control present must fail, as it is clipped",
    expected = "touching control not present",
    {
        let labels = ["QA edge target"];
        let left = whole(
            rect_of([400, 200], &labels, "QA edge target").min.x,
            "the target's left",
        );
        let r = root(
            "right_edge_inv",
            [left, 200],
            &labels,
            None,
            &["QA edge target"],
        );
        assert!(presence(&r).is_ok(), "touching control not present");
    }
);

negative_control!(
    screenshot_qa_control_one_point_over_right_edge_is_present,
    "a surface one point narrower leaves the target touching the edge, so the present check must fail",
    expected = "touching-edge rule broken",
    {
        let labels = ["QA edge target"];
        let left = whole(rect_of([400, 200], &labels, "QA edge target").min.x, "the target's left");
        let r = root("right_edge_short", [left, 200], &labels, None, &["QA edge target"]);
        assert!(
            presence(&r).is_ok(),
            "touching-edge rule broken: a touching control is absent from the present check"
        );
    }
);
