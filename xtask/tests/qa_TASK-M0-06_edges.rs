//! QA tests for TASK-M0-06, re-check round: the edges of a one-field repro that changes the target's size. REQ-VAL-003
//! asks that "a one-field pair reports the RGB profile along the line for each arm" (the task's acceptance line); a
//! `width` or `height` arm is a one-field pair, so a line lying exactly on an arm's last row or column is inside that
//! arm's image and must be reported, and a line one pixel past it is outside and must be refused, naming the line and
//! the arm, not read out of bounds. Known answers come from the self-test shader's formulas
//! (`fixtures/golden/selftest/gradient/gradient.wgsl`: R = min(x + step, 255), G = y, B = floor((x + y) / 2), in 8-bit
//! steps). The case's `row_128` runs from (0, 128) to (255, 128). Each test has a registered negative control (R-176).

use std::path::{Path, PathBuf};

use serde_json::json;
use validation::negative_control;
use xtask::golden::{self, Case, Config, Renderer};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn selftest_case() -> Case {
    Case::load(
        &repo_root().join("fixtures/golden/selftest/gradient"),
        "selftest/gradient",
    )
    .expect("selftest/gradient loads")
}

fn with(case: &Case, field: &str, value: u32) -> Config {
    let mut c = case.config.clone();
    c.set(field, json!(value)).expect("field set");
    c
}

// --- A line on an arm's last row is inside it: the pair is reported, with the analytic profile ---------------------

/// Arm b sets `height` to `height`. For 129, row 128 is the image's last row, so the repro runs and each arm's
/// `row_128` profile is the analytic row y = 128 at step 0.
fn check_last_row_reported(height: u32) {
    let case = selftest_case();
    let arms = [case.config.clone(), with(&case, "height", height)];
    let renderer = Renderer::new().unwrap_or_else(|e| panic!("{e}"));
    let repro = golden::repro(&case, arms, &renderer)
        .unwrap_or_else(|e| panic!("a line on the arm's last row was refused: {e}"));
    assert_eq!(repro.field, "height");
    let row: Vec<[u8; 3]> = (0..256u32)
        .map(|x| [x as u8, 128, ((x + 128) / 2) as u8])
        .collect();
    for arm in &repro.arms {
        assert!(
            arm.profiles[0] == row,
            "row_128 is not the analytic row y = 128"
        );
    }
}

#[test]
fn qa_m006_repro_line_on_last_row_reported() {
    check_last_row_reported(129);
}

negative_control!(
    qa_m006_repro_line_on_last_row_reported,
    "arm b's height set to 128, one row short of row_128",
    expected = "a line on the arm's last row was refused",
    check_last_row_reported(128)
);

// --- A line one pixel past an arm's edge is refused, naming the line and the arm -----------------------------------

/// Arm b sets `field` to `value`: height 128 leaves row 128 one row past the image, width 255 leaves the line's end
/// (x = 255) one column past it. Refused before rendering, naming `row_128` and arm b.
fn check_one_past_refused(field: &str, value: u32) {
    let case = selftest_case();
    let arms = [case.config.clone(), with(&case, field, value)];
    let message = golden::check_arms(&case, &arms)
        .expect_err("a line one pixel past the arm's image was not refused");
    assert!(
        message.contains("row_128") && message.contains("arm b"),
        "refused without naming the line and the arm: {message}"
    );
}

#[test]
fn qa_m006_repro_line_one_past_edge_refused() {
    check_one_past_refused("height", 128);
    check_one_past_refused("width", 255);
}

negative_control!(
    qa_m006_repro_line_one_past_edge_refused,
    "arm b's height set to 129, which holds row_128",
    expected = "a line one pixel past the arm's image was not refused",
    check_one_past_refused("height", 129)
);

// --- Numbers are compared by value, exactly: two different integers are two values --------------------------------

/// Arm a sets `constants.step` to `a`, arm b to `b`. Numbers are compared by value (REQ-VAL-003's "exactly one
/// variable changed": `1` against `1.0` is no change), so two different integers are one changed field, including
/// integers past f64's exact range (above i64::MAX, below -2^53), where a comparison through f64 would merge them.
fn check_integers_differ(a: serde_json::Value, b: serde_json::Value) {
    let case = selftest_case();
    let mut arm_a = case.config.clone();
    arm_a.set("constants.step", a.clone()).expect("field set");
    let mut arm_b = case.config.clone();
    arm_b.set("constants.step", b.clone()).expect("field set");
    let field = golden::single_differing_field(&arm_a, &arm_b).unwrap_or_else(|e| {
        panic!("two different integers were taken as one value ({a} and {b}): {e}")
    });
    assert_eq!(field, "constants.step");
}

#[test]
fn qa_m006_repro_distinct_large_integers_differ() {
    check_integers_differ(
        json!(9_223_372_036_854_775_808u64),
        json!(9_223_372_036_854_775_809u64),
    );
    check_integers_differ(
        json!(-9_007_199_254_740_993i64),
        json!(-9_007_199_254_740_992i64),
    );
}

negative_control!(
    qa_m006_repro_distinct_large_integers_differ,
    "the same integer in both arms",
    expected = "two different integers were taken as one value",
    check_integers_differ(
        json!(9_223_372_036_854_775_808u64),
        json!(9_223_372_036_854_775_808u64)
    )
);

// --- REQ-VAL-003: the profile along a line away from the image's edges ---------------------------------------------

/// The analytic pixel of the self-test at `(x, y)` with `constants.step = step`.
fn analytic(x: u32, y: u32, step: u32) -> [u8; 3] {
    [(x + step).min(255) as u8, y as u8, ((x + y) / 2) as u8]
}

/// Lines that touch neither x = 0 nor y = 0: a row, a column, a 45-degree diagonal and a row walked right to left. At
/// one pixel per step of the longer axis each is exactly the pixels between its ends, so each arm's profile is the
/// analytic gradient there at that arm's own step.
fn check_interior_profiles(steps: [u32; 2]) {
    let mut case = selftest_case();
    let interior = [
        ("row_mid", [100, 40], [140, 40]),
        ("col_mid", [30, 50], [30, 90]),
        ("diag_mid", [20, 10], [60, 50]),
        ("row_mid_reversed", [140, 40], [100, 40]),
    ];
    case.lines = interior
        .iter()
        .map(|&(name, from, to)| golden::Line {
            name: name.into(),
            from,
            to,
        })
        .collect();
    let arms = [
        with(&case, "constants.step", 0),
        with(&case, "constants.step", 1),
    ];
    let renderer = Renderer::new().unwrap_or_else(|e| panic!("{e}"));
    let repro = golden::repro(&case, arms, &renderer).expect("repro ran");
    for (arm, step) in repro.arms.iter().zip(steps) {
        let expected: Vec<Vec<[u8; 3]>> = vec![
            (100..=140).map(|x| analytic(x, 40, step)).collect(),
            (50..=90).map(|y| analytic(30, y, step)).collect(),
            (0..=40).map(|k| analytic(20 + k, 10 + k, step)).collect(),
            (100..=140).rev().map(|x| analytic(x, 40, step)).collect(),
        ];
        assert!(
            arm.profiles == expected,
            "an interior line's profile is not the analytic gradient at step {step}"
        );
    }
}

#[test]
fn qa_m006_repro_interior_line_profiles() {
    check_interior_profiles([0, 1]);
}

negative_control!(
    qa_m006_repro_interior_line_profiles,
    "the arms' steps swapped in the expectation",
    expected = "an interior line's profile is not the analytic gradient",
    check_interior_profiles([1, 0])
);

// --- The repro report gives each arm's configuration ---------------------------------------------------------------

/// The Deliverables' repro report gives "the arms' configurations with the single differing field": a line per arm
/// naming it and its render input (the shader, the fragment entry point, the size and the constant).
fn check_report_configurations(report: &str) {
    for arm in ["arm a", "arm b"] {
        assert!(
            report.lines().any(|l| l.starts_with(arm)
                && l.contains("gradient.wgsl")
                && l.contains("fs_main")
                && l.contains("256")
                && l.contains("constants.step")),
            "the report does not give {arm}'s configuration: {report}"
        );
    }
}

fn selftest_report() -> String {
    let case = selftest_case();
    let arms = [
        with(&case, "constants.step", 0),
        with(&case, "constants.step", 1),
    ];
    let renderer = Renderer::new().unwrap_or_else(|e| panic!("{e}"));
    golden::repro(&case, arms, &renderer)
        .expect("repro ran")
        .report()
}

#[test]
fn qa_m006_repro_report_gives_each_arm_configuration() {
    check_report_configurations(&selftest_report());
}

negative_control!(
    qa_m006_repro_report_gives_each_arm_configuration,
    "the report with arm b's configuration line removed",
    expected = "the report does not give arm b's configuration",
    check_report_configurations(
        &selftest_report()
            .lines()
            .filter(|l| !(l.starts_with("arm b") && l.contains("gradient.wgsl")))
            .collect::<Vec<_>>()
            .join("\n")
    )
);

// --- REQ-VAL-009: each symptom metric on a hand-filled image with known values -------------------------------------

/// A 2x2 image, white, white, magenta, black: magenta counts 1, white is a fraction 0.5 and magenta 0.25 of it, and
/// the red channel's mean is (255 + 255 + 255 + 0) / 4 = 191.25.
fn check_symptoms_known(rgb: Vec<u8>) {
    let image = golden::Image {
        width: 2,
        height: 2,
        rgb,
    };
    let measure = |kind| {
        golden::Symptom {
            name: "s".into(),
            kind,
        }
        .measure(&image)
    };
    let got = [
        measure(golden::SymptomKind::Count([255, 0, 255])),
        measure(golden::SymptomKind::Fraction([255, 255, 255])),
        measure(golden::SymptomKind::Fraction([255, 0, 255])),
        measure(golden::SymptomKind::Mean(0)),
    ];
    assert_eq!(
        got,
        [1.0, 0.5, 0.25, 191.25],
        "a symptom metric is not its known value"
    );
}

#[test]
fn qa_m006_symptom_metrics_known_values() {
    check_symptoms_known(vec![255, 255, 255, 255, 255, 255, 255, 0, 255, 0, 0, 0]);
}

negative_control!(
    qa_m006_symptom_metrics_known_values,
    "an all-white image in place of the known one",
    expected = "a symptom metric is not its known value",
    check_symptoms_known(vec![255; 12])
);

// --- The case format: a line is two pixels [x, y] inside the image, or the case is refused -------------------------

/// A copy of `selftest/gradient` (256x256) whose one line runs from `from` to `to`, loaded.
fn load_with_line(
    name: &str,
    from: serde_json::Value,
    to: serde_json::Value,
) -> Result<Case, String> {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("qa_m006_line_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch created");
    let source = repo_root().join("fixtures/golden/selftest/gradient");
    for file in ["gradient.wgsl", "reference.png"] {
        std::fs::copy(source.join(file), dir.join(file)).expect("copied");
    }
    let mut json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(source.join("case.json")).expect("read"))
            .expect("case.json parses");
    json["lines"] = json!({ "probe": { "from": from, "to": to } });
    std::fs::write(dir.join("case.json"), json.to_string()).expect("written");
    Case::load(&dir, "selftest/probe")
}

/// Each (from, to) is refused at load: an end on x = 256 or y = 256 (one past a 256-pixel image), or a point that is
/// not exactly two coordinates, which would otherwise be read as some other pixel.
fn check_bad_lines_refused(lines: &[(serde_json::Value, serde_json::Value)]) {
    for (i, (from, to)) in lines.iter().enumerate() {
        if let Ok(case) = load_with_line(&format!("bad{i}"), from.clone(), to.clone()) {
            panic!(
                "a line that is not two pixels inside the image was loaded: {from} to {to} as {:?} to {:?}",
                case.lines[0].from, case.lines[0].to
            );
        }
    }
}

#[test]
fn qa_m006_case_line_outside_or_malformed_refused() {
    let edge = load_with_line("edge", json!([255, 255]), json!([0, 255]))
        .expect("a line on the last row loads");
    assert_eq!(
        (edge.lines[0].from, edge.lines[0].to),
        ([255, 255], [0, 255])
    );
    check_bad_lines_refused(&[
        (json!([0, 256]), json!([255, 256])),
        (json!([256, 0]), json!([256, 10])),
        (json!([0, 128, 5]), json!([255, 128])),
        (json!([-1, 128, 7]), json!([255, 128])),
        (json!([0.5, 128, 7]), json!([255, 128])),
    ]);
}

negative_control!(
    qa_m006_case_line_outside_or_malformed_refused,
    "a line on the image's last row given to the refusal check",
    expected = "a line that is not two pixels inside the image was loaded",
    check_bad_lines_refused(&[(json!([255, 255]), json!([0, 255]))])
);

// --- The diff output on hand-filled images with known values -------------------------------------------------------

/// A 3x2 render against a 3x2 reference: pixel (0, 1) differs by 1 in red and pixel (2, 1) by 5 in green. The diff
/// image is those per-channel differences; 2 of 6 pixels differ; the largest step is 5; the mean per-channel step is
/// 6 / 18; the first differing pixel, in row order, is (0, 1).
fn check_diff_known(render_rgb: Vec<u8>) {
    let reference = golden::Image {
        width: 3,
        height: 2,
        rgb: vec![10; 18],
    };
    let render = golden::Image {
        width: 3,
        height: 2,
        rgb: render_rgb,
    };
    let d = golden::diff(&render, &reference).expect("same size");
    let mut expected_image = vec![0u8; 18];
    expected_image[9] = 1;
    expected_image[16] = 5;
    assert!(
        d.image.rgb == expected_image
            && (d.image.width, d.image.height) == (3, 2)
            && d.max_step == 5
            && d.differing == 2
            && d.pixels == 6
            && d.mean_step == 6.0 / 18.0
            && d.first == Some((0, 1)),
        "the diff is not its known value: {d:?}"
    );
}

#[test]
fn qa_m006_diff_known_values() {
    let mut render = vec![10u8; 18];
    render[9] = 11; // (0, 1) red, +1
    render[16] = 5; // (2, 1) green, -5
    check_diff_known(render);
}

negative_control!(
    qa_m006_diff_known_values,
    "a render identical to its reference",
    expected = "the diff is not its known value",
    check_diff_known(vec![10; 18])
);

// --- The case format: symptoms -------------------------------------------------------------------------------------

/// A copy of `selftest/gradient` with `symptoms` replaced, loaded.
fn load_with_symptoms(name: &str, symptoms: serde_json::Value) -> Result<Case, String> {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("qa_m006_symptoms_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch created");
    let source = repo_root().join("fixtures/golden/selftest/gradient");
    for file in ["gradient.wgsl", "reference.png"] {
        std::fs::copy(source.join(file), dir.join(file)).expect("copied");
    }
    let mut json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(source.join("case.json")).expect("read"))
            .expect("case.json parses");
    json["symptoms"] = symptoms;
    std::fs::write(dir.join("case.json"), json.to_string()).expect("written");
    Case::load(&dir, "selftest/symptoms")
}

/// A mean over each of the three channels, r, g and b, loads, and on a 1x1 image (10, 20, 30) measures 10, 20, 30.
fn check_channel_means(pixel: [u8; 3]) {
    let case = load_with_symptoms(
        "means",
        json!([
            { "name": "r", "kind": "mean", "channel": "r" },
            { "name": "g", "kind": "mean", "channel": "g" },
            { "name": "b", "kind": "mean", "channel": "b" }
        ]),
    )
    .unwrap_or_else(|e| panic!("a mean over r, g or b was refused: {e}"));
    let image = golden::Image {
        width: 1,
        height: 1,
        rgb: pixel.to_vec(),
    };
    let got: Vec<f64> = case.symptoms.iter().map(|s| s.measure(&image)).collect();
    assert_eq!(
        got,
        [10.0, 20.0, 30.0],
        "a channel mean is not that channel's value"
    );
}

#[test]
fn qa_m006_symptom_each_channel_mean() {
    check_channel_means([10, 20, 30]);
}

negative_control!(
    qa_m006_symptom_each_channel_mean,
    "the pixel's channels reversed",
    expected = "a channel mean is not that channel's value",
    check_channel_means([30, 20, 10])
);

/// Each `rgb` is refused at load: a colour is exactly three channels in 0..=255, and one that is not would otherwise
/// be counted as some other colour.
fn check_bad_rgb_refused(colours: &[serde_json::Value]) {
    for (i, rgb) in colours.iter().enumerate() {
        let symptoms = json!([{ "name": "c", "kind": "count", "rgb": rgb }]);
        if let Ok(case) = load_with_symptoms(&format!("bad{i}"), symptoms) {
            panic!(
                "a colour that is not three channels in 0..=255 was loaded: {rgb} as {:?}",
                case.symptoms[0].kind
            );
        }
    }
}

#[test]
fn qa_m006_case_symptom_colour_malformed_refused() {
    load_with_symptoms(
        "ok",
        json!([{ "name": "c", "kind": "count", "rgb": [255, 0, 255] }]),
    )
    .expect("a three-channel colour loads");
    check_bad_rgb_refused(&[
        json!([255, 0]),
        json!([300, 255, 0, 255]),
        json!([-1, 255, 0, 255]),
        json!([0.5, 255, 0, 255]),
    ]);
}

negative_control!(
    qa_m006_case_symptom_colour_malformed_refused,
    "a three-channel colour given to the refusal check",
    expected = "a colour that is not three channels in 0..=255 was loaded",
    check_bad_rgb_refused(&[json!([255, 0, 255])])
);
