//! `cargo xtask golden repro` (REQ-VAL-003, REQ-VAL-009; philosophy §4.3a, pitfalls §1.3 and §8): a pair of arms
//! differing in two fields is refused; a one-field pair reports, per arm, the RGB values along the case's line, and
//! each symptom in its own column, the one the change left alone reported unchanged rather than merged.
//!
//! The fixture `tests/fixtures/golden_repro/two_symptoms` draws two co-located symptoms with separate causes: a
//! magenta disc of radius `disc` at (32, 32) and a white column at x = `stripe`.

use std::path::Path;

use serde_json::json;
use validation::negative_control;
use xtask::golden::{self, Case, Config, Renderer, Repro};

fn case() -> Case {
    let dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/golden_repro/two_symptoms");
    Case::load(&dir, "golden_repro/two_symptoms").expect("fixture case loaded")
}

/// The case's configuration with each `(field, value)` set.
fn config(case: &Case, fields: &[(&str, f64)]) -> Config {
    let mut config = case.config.clone();
    for (field, value) in fields {
        config.set(field, json!(value)).unwrap();
    }
    config
}

/// A repro of `case` whose arm a sets `a` and arm b sets `b`.
fn repro(case: &Case, a: &[(&str, f64)], b: &[(&str, f64)]) -> Result<Repro, String> {
    let renderer = Renderer::new().unwrap_or_else(|e| panic!("{e}"));
    golden::repro(case, [config(case, a), config(case, b)], &renderer)
}

// --- A pair differing in two fields is refused ---------------------------------------------------------------------

fn check_two_field_refusal(result: Result<Repro, String>) {
    let message = result.expect_err("a pair differing in two fields was not refused");
    assert!(
        message.contains("differ in 2 field(s) (constants.disc, constants.stripe)"),
        "refused for another reason: {message}"
    );
}

#[test]
fn golden_repro_refuses_two_fields() {
    let case = case();
    check_two_field_refusal(repro(
        &case,
        &[],
        &[("constants.disc", 8.0), ("constants.stripe", 20.0)],
    ));
}

negative_control!(
    golden_repro_refuses_two_fields,
    "a pair differing in one field given to the refusal check",
    expected = "a pair differing in two fields was not refused",
    {
        let case = case();
        check_two_field_refusal(repro(&case, &[], &[("constants.disc", 8.0)]));
    }
);

#[test]
fn golden_repro_refuses_no_field() {
    let case = case();
    let message = repro(&case, &[], &[]).expect_err("an identical pair was not refused");
    assert!(message.contains("differ in 0 field(s)"), "{message}");
}

negative_control!(
    golden_repro_refuses_no_field,
    "a pair differing in one field given as the identical pair",
    expected = "an identical pair was not refused",
    {
        let case = case();
        let _ = repro(&case, &[], &[("constants.disc", 8.0)])
            .expect_err("an identical pair was not refused");
    }
);

// --- A one-field pair reports the RGB profile along the line, per arm ----------------------------------------------

/// `repro` names its one field, and holds per arm the RGB values of that arm's own image along `row_32`, which the
/// disc crosses in arm b only; the report prints each arm's profile.
fn check_profiles(repro: &Repro) {
    assert_eq!(repro.field, "constants.disc");
    let points = repro.lines[0].points();
    assert_eq!(points.len(), 64, "row_32 is not 64 pixels");
    for (tag, arm) in ["a", "b"].iter().zip(&repro.arms) {
        let from_image: Vec<[u8; 3]> = points.iter().map(|&(x, y)| arm.image.pixel(x, y)).collect();
        assert_eq!(
            arm.profiles[0], from_image,
            "the profile of arm {tag} is not its own image along the line"
        );
    }
    // Arm a's ramp at x = 32: R = 32 / 63, stored as round(32 * 255 / 63) = round(129.52) = 130.
    assert_eq!(
        repro.arms[0].profiles[0][32],
        [130, 0, 0],
        "arm a at (32, 32)"
    );
    assert_eq!(
        repro.arms[1].profiles[0][32],
        [255, 0, 255],
        "arm b at (32, 32)"
    );
    let report = repro.report();
    assert!(
        report.contains("differing field: constants.disc (a: 0, b: 8"),
        "{report}"
    );
    for tag in ["a", "b"] {
        assert!(
            report.contains(&format!("arm {tag} line row_32")),
            "no arm {tag} profile: {report}"
        );
    }
}

#[test]
fn golden_repro_reports_line_profile_per_arm() {
    let case = case();
    check_profiles(&repro(&case, &[], &[("constants.disc", 8.0)]).expect("repro ran"));
}

negative_control!(
    golden_repro_reports_line_profile_per_arm,
    "a report whose arms' profiles are swapped",
    expected = "is not its own image along the line",
    {
        let case = case();
        let mut repro = repro(&case, &[], &[("constants.disc", 8.0)]).expect("repro ran");
        let [a, b] = &mut repro.arms;
        std::mem::swap(&mut a.profiles, &mut b.profiles);
        check_profiles(&repro);
    }
);

// --- Each symptom in its own column; the unchanged column is reported unchanged ------------------------------------

/// The disc's change moves the magenta column and leaves the white one numerically identical, and the report shows
/// them as two columns, `moved` and `unchanged` (pitfalls §8).
fn check_columns(repro: &Repro) {
    let columns = repro.columns();
    let names: Vec<&str> = columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["magenta", "white"], "not one column per symptom");
    assert!(columns[0].moved, "the magenta column did not move");
    assert_eq!(
        (columns[1].moved, columns[1].values),
        (false, [64.0 / 4096.0; 2]),
        "the white column was not reported unchanged"
    );
    let report = repro.report();
    let row = |label: &str| {
        report
            .lines()
            .find(|l| !l.contains(':') && l.split_whitespace().next() == Some(label))
            .unwrap_or_else(|| panic!("no {label} row: {report}"))
            .split_whitespace()
            .collect::<Vec<_>>()
    };
    assert_eq!(row("arm"), ["arm", "magenta", "white"], "{report}");
    assert_eq!(row("change"), ["change", "moved", "unchanged"], "{report}");
    assert_eq!(row("a")[2], row("b")[2], "{report}");
}

#[test]
fn golden_repro_columns_unchanged_column_reported() {
    let case = case();
    check_columns(&repro(&case, &[], &[("constants.disc", 8.0)]).expect("repro ran"));
}

negative_control!(
    golden_repro_columns_unchanged_column_reported,
    "the white column moved through the disc (stripe = 32), so the change moves both symptoms",
    expected = "the white column was not reported unchanged",
    {
        let case = case();
        let through = [("constants.stripe", 32.0)];
        check_columns(
            &repro(
                &case,
                &through,
                &[("constants.stripe", 32.0), ("constants.disc", 8.0)],
            )
            .expect("repro ran"),
        );
    }
);

// --- A number spelled two ways is one value: the pair differs in no field ------------------------------------------

/// Arm a keeps the fixture's integer `disc: 0`; arm b sets `disc` to `b`, a float. `0.0` is the same number, so the
/// pair differs in no field and is refused.
fn check_same_number_refused(b: f64) {
    let case = case();
    let message =
        golden::single_differing_field(&case.config, &config(&case, &[("constants.disc", b)]))
            .expect_err("a pair differing only in a number's spelling was not refused");
    assert!(message.contains("differ in 0 field(s)"), "{message}");
}

#[test]
fn golden_repro_refuses_same_number_two_spellings() {
    check_same_number_refused(0.0);
}

negative_control!(
    golden_repro_refuses_same_number_two_spellings,
    "arm b given a different number (1.0) instead of 0 spelled 0.0",
    expected = "a pair differing only in a number's spelling was not refused",
    check_same_number_refused(1.0)
);
