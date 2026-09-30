//! The gate report (REQ-VAL-002, REQ-VAL-168; philosophy §4.6; R-258): a report missing its scatter, region-count or
//! negative-results section is refused; a conclusion from fewer regions than the gate declares is flagged, and so is
//! one from an unrecorded count. And `gate.json`'s threshold is refused unless it names a requirement (R-71).

use std::path::PathBuf;

use validation::gate::{
    GateConfig, GateReport, Minimum, RegionCount, Regions, Status, Threshold, Verdict,
};

/// A threshold of 0.1, provisional against REQ-VAL-135 under R-171.
fn threshold() -> Threshold {
    Threshold {
        value: 0.1,
        requirement: "REQ-VAL-135".to_owned(),
        status: Status::Provisional,
        ruling: Some("R-171".to_owned()),
    }
}

/// A complete report: `count` regions seen against a declared minimum of `minimum`.
fn report(count: RegionCount, minimum: Minimum) -> GateReport {
    GateReport {
        gate: "convergence".to_owned(),
        input: "complete.json".to_owned(),
        description: "a complete report".to_owned(),
        verdict: Verdict::Pass,
        threshold: threshold(),
        scatter: vec!["r_k, a: 0.0667, 0.0188, 0.0046".to_owned()],
        regions: Some(Regions { count, minimum }),
        negative_results: vec!["a: largest relative step r = 0.0667".to_owned()],
    }
}

fn declared(value: u32) -> Minimum {
    Minimum::Declared {
        value,
        requirement: "REQ-VAL-168".to_owned(),
    }
}

/// A complete report with `section` emptied or removed.
fn without(section: &str) -> GateReport {
    let mut r = report(RegionCount::Recorded(5), declared(5));
    match section {
        "scatter" => r.scatter.clear(),
        "region count" => r.regions = None,
        "negative results" => r.negative_results = vec!["  ".to_owned()],
        "none" => {}
        other => panic!("no section `{other}`"),
    }
    r
}

/// `report` is refused, by the check and by each writer, naming `section`.
fn check_refused(report: &GateReport, section: &str) {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("gate_report_refused");
    for (what, result) in [
        ("check", report.check().map(|_| ())),
        ("markdown", report.markdown().map(|_| ())),
        ("json", report.json().map(|_| ())),
        ("write", report.write(&dir).map(|_| ())),
    ] {
        let message = result.expect_err(&format!(
            "the report without {section} was not refused by {what}"
        ));
        assert!(
            message.contains(section),
            "{what}'s refusal does not name {section}: {message}"
        );
    }
}

#[test]
fn gate_report_refuses_missing_scatter() {
    check_refused(&without("scatter"), "scatter");
}

validation::negative_control!(
    gate_report_refuses_missing_scatter,
    "a complete report, required to be refused",
    expected = "was not refused",
    check_refused(&without("none"), "scatter")
);

#[test]
fn gate_report_refuses_missing_region_count() {
    check_refused(&without("region count"), "region count");
}

validation::negative_control!(
    gate_report_refuses_missing_region_count,
    "a complete report, required to be refused",
    expected = "was not refused",
    check_refused(&without("none"), "region count")
);

#[test]
fn gate_report_refuses_empty_negative_results() {
    check_refused(&without("negative results"), "negative results");
}

validation::negative_control!(
    gate_report_refuses_empty_negative_results,
    "a complete report, required to be refused",
    expected = "was not refused",
    check_refused(&without("none"), "negative results")
);

/// `report`'s conclusion is flagged with a flag containing `flag`, in its flags, its markdown and its JSON.
fn check_flagged(report: &GateReport, flag: &str) {
    let flags = report.flags().expect("a complete report");
    assert!(
        flags.iter().any(|f| f.contains(flag)),
        "the conclusion was not flagged `{flag}`: {flags:?}"
    );
    assert!(
        report.markdown().unwrap().contains(flag),
        "the markdown omits the flag `{flag}`"
    );
    assert!(
        report.json().unwrap().contains(flag),
        "the JSON omits the flag `{flag}`"
    );
}

#[test]
fn gate_report_flags_fewer_regions_than_declared() {
    check_flagged(
        &report(RegionCount::Recorded(2), declared(5)),
        "conclusion drawn from 2 region(s), fewer than the 5 the gate declares",
    );
}

validation::negative_control!(
    gate_report_flags_fewer_regions_than_declared,
    "exactly the declared number of regions, required to be flagged as too few",
    expected = "the conclusion was not flagged",
    check_flagged(
        &report(RegionCount::Recorded(5), declared(5)),
        "fewer than the"
    )
);

#[test]
fn gate_report_flags_an_unrecorded_count() {
    check_flagged(
        &report(RegionCount::NotRecorded { series: 1 }, declared(1)),
        "conclusion drawn from an unrecorded region count: flagged as one from too few regions",
    );
}

validation::negative_control!(
    gate_report_flags_an_unrecorded_count,
    "a recorded count meeting its minimum, required to be flagged as unrecorded",
    expected = "the conclusion was not flagged",
    check_flagged(
        &report(RegionCount::Recorded(1), declared(1)),
        "unrecorded region count"
    )
);

/// `report` has no flag at all.
fn check_not_flagged(report: &GateReport) {
    let flags = report.flags().expect("a complete report");
    assert!(
        flags.is_empty(),
        "a conclusion meeting its minimum was flagged: {flags:?}"
    );
}

#[test]
fn gate_report_does_not_flag_enough_regions() {
    check_not_flagged(&report(RegionCount::Recorded(6), declared(5)));
}

validation::negative_control!(
    gate_report_does_not_flag_enough_regions,
    "fewer regions than declared, required to pass unflagged",
    expected = "a conclusion meeting its minimum was flagged",
    check_not_flagged(&report(RegionCount::Recorded(4), declared(5)))
);

/// The region section, with a minimum not yet calibrated, prints the count seen and "minimum not yet calibrated".
fn check_uncalibrated(report: &GateReport) {
    let md = report.markdown().expect("a complete report");
    assert!(
        md.contains("region count: 3"),
        "the count seen is not printed:\n{md}"
    );
    assert!(
        md.contains("minimum not yet calibrated (REQ-VAL-168)"),
        "the minimum is not marked not yet calibrated:\n{md}"
    );
}

#[test]
fn gate_report_marks_the_minimum_not_yet_calibrated() {
    check_uncalibrated(&report(
        RegionCount::Recorded(3),
        Minimum::NotCalibrated {
            requirement: "REQ-VAL-168".to_owned(),
        },
    ));
}

validation::negative_control!(
    gate_report_marks_the_minimum_not_yet_calibrated,
    "a declared minimum, required to read as not yet calibrated",
    expected = "the minimum is not marked not yet calibrated",
    check_uncalibrated(&report(RegionCount::Recorded(3), declared(3)))
);

/// The requirements a `gate.json` is checked against: a calibration, a plain requirement and a retired one.
const REQUIREMENTS: &str = "\
- id: REQ-VAL-135
  area: VAL
  kind: calibration
  statement: \"threshold\"
- id: REQ-VAL-004
  area: VAL
  statement: \"plain\"
- id: REQ-VAL-900
  area: VAL
  kind: calibration
  retired: \"R-0: gone\"
- id: REQ-VAL-168
  area: VAL
  kind: calibration
";

/// A `gate.json` whose threshold names `requirement`, with `status`.
fn gate_json(requirement: &str, status: &str) -> String {
    format!(
        r#"{{"gate": "convergence", "inputs": ["a.json"],
            "threshold": {{"value": 0.1, "requirement": "{requirement}", "status": "{status}", "ruling": "R-171"}},
            "min_regions": {{"value": null, "requirement": "REQ-VAL-168"}}}}"#
    )
}

/// `text` is refused, the refusal containing `why`.
fn check_config_refused(text: &str, why: &str) {
    let message = GateConfig::parse(text, REQUIREMENTS)
        .map(|_| ())
        .expect_err("the gate.json was not refused");
    assert!(
        message.contains(why),
        "the refusal does not say `{why}`: {message}"
    );
}

#[test]
fn gate_config_refuses_a_threshold_naming_no_requirement() {
    check_config_refused(
        &gate_json("", "provisional"),
        "names no requirement or calibration requirement",
    );
    let bare = r#"{"gate": "convergence", "inputs": ["a.json"], "threshold": {"value": 0.1, "status": "confirmed"},
                   "min_regions": {"value": null, "requirement": "REQ-VAL-168"}}"#;
    check_config_refused(bare, "names no requirement or calibration requirement");
}

validation::negative_control!(
    gate_config_refuses_a_threshold_naming_no_requirement,
    "a threshold naming its calibration requirement, required to be refused",
    expected = "the gate.json was not refused",
    check_config_refused(
        &gate_json("REQ-VAL-135", "provisional"),
        "names no requirement"
    )
);

#[test]
fn gate_config_refuses_an_unknown_or_retired_requirement() {
    check_config_refused(
        &gate_json("REQ-VAL-999", "confirmed"),
        "not in plan/requirements.yaml",
    );
    check_config_refused(&gate_json("REQ-VAL-900", "confirmed"), "retired");
}

validation::negative_control!(
    gate_config_refuses_an_unknown_or_retired_requirement,
    "a live requirement, required to be refused",
    expected = "the gate.json was not refused",
    check_config_refused(&gate_json("REQ-VAL-004", "confirmed"), "retired")
);

#[test]
fn gate_config_refuses_a_provisional_threshold_on_a_plain_requirement() {
    check_config_refused(
        &gate_json("REQ-VAL-004", "provisional"),
        "not a calibration requirement",
    );
}

validation::negative_control!(
    gate_config_refuses_a_provisional_threshold_on_a_plain_requirement,
    "a provisional threshold on a calibration requirement, required to be refused",
    expected = "the gate.json was not refused",
    check_config_refused(
        &gate_json("REQ-VAL-135", "provisional"),
        "not a calibration requirement"
    )
);

/// `text` parses, its threshold reading "0.1, provisional (REQ-VAL-135, R-171)".
fn check_config_accepted(text: &str) {
    let config =
        GateConfig::parse(text, REQUIREMENTS).unwrap_or_else(|e| panic!("gate.json refused: {e}"));
    assert_eq!(
        config.threshold.describe(),
        "0.1, provisional (REQ-VAL-135, R-171)"
    );
}

#[test]
fn gate_config_accepts_a_provisional_threshold_on_its_calibration() {
    check_config_accepted(&gate_json("REQ-VAL-135", "provisional"));
}

validation::negative_control!(
    gate_config_accepts_a_provisional_threshold_on_its_calibration,
    "a threshold naming no requirement, required to be accepted",
    expected = "gate.json refused",
    check_config_accepted(&gate_json("", "provisional"))
);
