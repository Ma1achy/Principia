//! The convergence-under-refinement gate (REQ-VAL-004, REQ-VAL-168, REQ-VAL-169; R-171, R-258): r_k and the pass
//! rule on the pitfalls §3 record and a converging sequence, the refusal of a sequence not ordered coarse → fine, the
//! scatter of a two-region input computed by hand, and the runner on `fixtures/gates/convergence/`.

use std::fs;
use std::path::{Path, PathBuf};

use validation::convergence::{check_order, passes, relative_steps, ConvergenceGate, Input};
use validation::gate::{self, GateConfig, Verdict};

/// The pitfalls §3 record, ordered coarse → fine: strides 32, 4, 1, 0.
const PITFALL: [f64; 4] = [0.2153, 0.4423, 0.5494, 0.0947];
/// The converging fixture: strides 32, 4, 1, 0.
const CONVERGING: [f64; 4] = [0.3000, 0.3200, 0.3260, 0.3275];

/// `values` give r_k = `expected`, each within 5e-5 (the four places the task records).
fn check_steps(values: &[f64], expected: &[f64]) {
    let r = relative_steps(values).unwrap();
    assert_eq!(
        r.len(),
        expected.len(),
        "r has {} steps, not {}",
        r.len(),
        expected.len()
    );
    for (k, (got, want)) in r.iter().zip(expected).enumerate() {
        assert!((got - want).abs() < 5e-5, "r_{} = {got}, not {want}", k + 1);
    }
}

#[test]
fn convergence_steps_are_relative() {
    check_steps(&PITFALL, &[1.0543, 0.2421, 0.8276]);
    check_steps(&CONVERGING, &[0.0667, 0.0188, 0.0046]);
}

validation::negative_control!(
    convergence_steps_are_relative,
    "the absolute steps the ruling first quoted (0.227, 0.107, 0.455), in place of the relative ones",
    expected = "r_1 = ",
    check_steps(&PITFALL, &[0.227, 0.107, 0.455])
);

/// The pass rule gives `pass` on `values` against 0.1.
fn check_rule(values: &[f64], pass: bool) {
    let r = relative_steps(values).unwrap();
    assert_eq!(
        passes(&r, 0.1),
        pass,
        "the rule gave {} on {values:?} (r = {r:?})",
        !pass
    );
}

#[test]
fn convergence_pitfall_record_fails() {
    check_rule(&PITFALL, false);
}

validation::negative_control!(
    convergence_pitfall_record_fails,
    "the converging sequence, required to fail",
    expected = "the rule gave true",
    check_rule(&CONVERGING, false)
);

#[test]
fn convergence_converging_sequence_passes() {
    check_rule(&CONVERGING, true);
}

validation::negative_control!(
    convergence_converging_sequence_passes,
    "the pitfalls record, required to pass",
    expected = "the rule gave false",
    check_rule(&PITFALL, true)
);

#[test]
fn convergence_finest_step_above_threshold_fails() {
    // r = 0.5, 0.2: strictly decreasing, but the finest r is not below 0.1.
    check_rule(&[1.0, 1.5, 1.2], false);
}

validation::negative_control!(
    convergence_finest_step_above_threshold_fails,
    "a finest r of 0.05, below the threshold, required to fail",
    expected = "the rule gave true",
    check_rule(&[1.0, 1.5, 1.425], false)
);

/// `strides` are refused as not coarse → fine.
fn check_order_refused(strides: &[u32]) {
    let message = check_order(strides).expect_err("the strides were not refused");
    assert!(message.starts_with("refused"), "{message}");
}

#[test]
fn convergence_refuses_wrong_order() {
    check_order_refused(&[0, 32, 4, 1]);
    check_order_refused(&[32, 4, 4, 0]);
    check_order_refused(&[1, 4, 32, 0]);
    check_order_refused(&[32, 4, 1]);
    check_order_refused(&[1, 0]);
}

validation::negative_control!(
    convergence_refuses_wrong_order,
    "strides 32, 4, 1, 0, required to be refused",
    expected = "the strides were not refused",
    check_order_refused(&[32, 4, 1, 0])
);

/// The requirements the configs here are checked against.
const REQUIREMENTS: &str = "\
- id: REQ-VAL-135
  kind: calibration
- id: REQ-VAL-168
  kind: calibration
";

fn config() -> GateConfig {
    GateConfig::parse(
        r#"{"gate": "convergence", "inputs": ["two.json"],
            "threshold": {"value": 0.1, "requirement": "REQ-VAL-135", "status": "provisional", "ruling": "R-171"},
            "min_regions": {"value": null, "requirement": "REQ-VAL-168"}}"#,
        REQUIREMENTS,
    )
    .unwrap()
}

/// A two-region input, region b ending at `b_last`.
fn two_regions(b_last: f64) -> Input {
    serde_json::from_value(serde_json::json!({
        "description": "two regions",
        "strides": [32, 4, 1, 0],
        "series": [
            { "label": "a", "values": CONVERGING },
            { "label": "b", "values": [0.5, 0.55, 0.56, b_last] }
        ],
        "region_count": 2
    }))
    .unwrap()
}

/// The scatter of `input` is, by hand:
/// a: 0.02/0.3 = 0.0667, 0.006/0.32 = 0.0188, 0.0015/0.326 = 0.0046;
/// b: 0.05/0.5 = 0.1000, 0.01/0.55 = 0.0182, 0.001/0.56 = 0.0018;
/// final r across the two: min 0.0018 (b), max 0.0046 (a), spread 0.0028.
fn check_two_region_scatter(input: &Input) {
    let report = ConvergenceGate::judge(&config(), "two.json", input).unwrap();
    assert_eq!(
        report.scatter,
        [
            "r_k, a: 0.0667, 0.0188, 0.0046",
            "r_k, b: 0.1000, 0.0182, 0.0018",
            "final r_k across 2 region(s): min 0.0018, max 0.0046 (spread 0.0028)",
        ],
        "the scatter is not the hand-computed one"
    );
    assert_eq!(report.verdict, Verdict::Pass, "both regions converge");
    let md = report.markdown().unwrap();
    assert!(
        md.contains("## Scatter\n\n- r_k, a:"),
        "the markdown's scatter section:\n{md}"
    );
    assert!(
        md.contains("region count: 2"),
        "the markdown's region count:\n{md}"
    );
}

#[test]
fn convergence_scatter_two_regions_by_hand() {
    check_two_region_scatter(&two_regions(0.561));
}

validation::negative_control!(
    convergence_scatter_two_regions_by_hand,
    "region b ending at 0.562 (final r 0.0036), in place of 0.561",
    expected = "the scatter is not the hand-computed one",
    check_two_region_scatter(&two_regions(0.562))
);

/// This workspace's root.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A copy of this workspace's convergence gate under a directory of its own named `case`, with the pitfall input
/// expected to give `pitfall_expected`; returns the copy's root.
fn copy_gate(case: &str, pitfall_expected: &str) -> PathBuf {
    let copy = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("convergence_{case}"));
    let _ = fs::remove_dir_all(&copy);
    let from = root().join("fixtures/gates/convergence");
    let to = copy.join("fixtures/gates/convergence");
    fs::create_dir_all(&to).unwrap();
    fs::create_dir_all(copy.join("plan")).unwrap();
    fs::copy(
        root().join("plan/requirements.yaml"),
        copy.join("plan/requirements.yaml"),
    )
    .unwrap();
    for entry in fs::read_dir(&from).unwrap() {
        let path = entry.unwrap().path();
        fs::copy(&path, to.join(path.file_name().unwrap())).unwrap();
    }
    let pitfall = to.join("pitfall_escape_fraction.json");
    let text = fs::read_to_string(&pitfall).unwrap().replace(
        r#""expected": "fail""#,
        &format!(r#""expected": "{pitfall_expected}""#),
    );
    fs::write(&pitfall, text).unwrap();
    copy
}

/// The runner on the gate at `root` passes, and writes each report with the threshold's source and the region count.
fn check_runner(root: &Path) {
    let out = root.join("out");
    gate::run(root, "convergence", &out).unwrap_or_else(|e| panic!("the gate run failed: {e}"));
    for stem in ["pitfall_escape_fraction", "converging"] {
        let md = fs::read_to_string(out.join(format!("convergence/{stem}.md"))).unwrap();
        for text in [
            "0.1, provisional (REQ-VAL-135, R-171)",
            "region count: not recorded",
            "minimum not yet calibrated (REQ-VAL-168)",
            "conclusion drawn from an unrecorded region count",
        ] {
            assert!(md.contains(text), "{stem}.md lacks `{text}`:\n{md}");
        }
        let json: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(out.join(format!("convergence/{stem}.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(json["threshold"]["requirement"], "REQ-VAL-135");
    }
    assert!(
        !out.join("convergence/wrong_order.md").exists(),
        "a refused input has a report"
    );
}

#[test]
fn convergence_runner_gives_each_fixture_its_outcome() {
    check_runner(&copy_gate("fixtures", "fail"));
}

validation::negative_control!(
    convergence_runner_gives_each_fixture_its_outcome,
    "the pitfalls record expected to pass",
    expected = "the gate run failed",
    check_runner(&copy_gate("ctl_fixtures", "pass"))
);
