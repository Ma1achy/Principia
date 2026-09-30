//! QA tests for TASK-M0-05, written from the requirements it closes and the rulings they cite:
//! - REQ-VAL-004 (R-171): samples ordered coarse → fine (strides strictly decreasing, 0 last); r_k = |x_k − x_{k−1}| /
//!   |x_{k−1}|; pass iff r_k is *strictly* decreasing and the finest r_k is *below* the threshold (provisional 0.1
//!   against REQ-VAL-135); "a quantity whose largest relative step is at the finest sampling must not be reported as
//!   converged"; a sequence in the wrong order is refused.
//! - REQ-VAL-002 (philosophy §4.6): the report has sections for scatter, region count and negative results; a report
//!   missing one is refused; a conclusion from fewer regions than declared is flagged.
//! - REQ-VAL-168 (R-258): `cargo xtask gate convergence` prints the region count it saw and "minimum not yet
//!   calibrated"; the fixtures record their count as "not recorded"; a conclusion from an unrecorded count is flagged
//!   like one from too few regions.
//! - REQ-VAL-169 (R-258): the scatter is the r_k sequence per region and the min–max of the final r_k across regions,
//!   checked on a two-region input computed by hand (a different one from the implementer's, with the minimum in the
//!   first region).
//! - The deliverable "the runner refuses a numeric threshold that names no requirement or calibration requirement".
//!
//! The threshold every rule test judges against is the one `fixtures/gates/convergence/gate.json` gives, read through
//! the runner's own parser against `plan/requirements.yaml`; none is picked here. The end-to-end tests run the `gate`
//! binary that `cargo xtask gate` runs, on the checked-in fixtures or on a sandbox copy of them under
//! `CARGO_TARGET_TMPDIR`, with reports written under a sandbox `CARGO_TARGET_DIR`. Each test registers a negative
//! control (R-176) that feeds its check the input it guards against and trips its assertion.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};
use validation::convergence::{ConvergenceGate, Input};
use validation::gate::{GateConfig, GateReport, Minimum, RegionCount, Regions, Verdict};
use validation::negative_control;

// ---------------------------------------------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------------------------------------------

/// The workspace root.
fn ws() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The checked-in convergence gate's configuration, parsed and checked against `plan/requirements.yaml`.
fn real_config() -> GateConfig {
    let text = fs::read_to_string(ws().join("fixtures/gates/convergence/gate.json")).unwrap();
    let reqs = fs::read_to_string(ws().join("plan/requirements.yaml")).unwrap();
    let config = GateConfig::parse(&text, &reqs).expect("the checked-in gate.json is refused");
    // The task and R-171 fix M0's threshold at 0.1, provisional against REQ-VAL-135; the boundary cases below are
    // built on that value.
    assert_eq!(
        config.threshold.value, 0.1,
        "the M0 threshold is not R-171's 0.1"
    );
    assert_eq!(config.threshold.requirement, "REQ-VAL-135");
    config
}

/// A single-series input at `strides` with `values`, its region count "not recorded".
fn input(strides: &[u32], values: &[f64]) -> Input {
    serde_json::from_value(json!({
        "description": "qa",
        "strides": strides,
        "series": [{ "label": "q", "values": values }],
        "region_count": "not recorded"
    }))
    .unwrap()
}

/// The gate, at the checked-in threshold, gives `want` on `values` at strides 32, 4, 1, 0.
fn check_verdict(values: &[f64], want: Verdict) {
    let report = ConvergenceGate::judge(&real_config(), "qa.json", &input(&[32, 4, 1, 0], values))
        .unwrap_or_else(|e| panic!("the gate refused {values:?}: {e}"));
    assert_eq!(
        report.verdict, want,
        "the gate gave {:?} on {values:?}, not {want:?}",
        report.verdict
    );
}

/// A sandbox copy of the workspace's convergence gate and `plan/requirements.yaml` under `CARGO_TARGET_TMPDIR`,
/// named `case`; `edit` may rewrite each fixture file (by file name) before it is written.
fn sandbox(case: &str, edit: impl Fn(&str, &mut Value)) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("qa_m0_05")
        .join(case);
    let _ = fs::remove_dir_all(&root);
    let to = root.join("fixtures/gates/convergence");
    fs::create_dir_all(&to).unwrap();
    fs::create_dir_all(root.join("plan")).unwrap();
    fs::copy(
        ws().join("plan/requirements.yaml"),
        root.join("plan/requirements.yaml"),
    )
    .unwrap();
    for entry in fs::read_dir(ws().join("fixtures/gates/convergence")).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let mut value: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        edit(&name, &mut value);
        fs::write(
            to.join(&name),
            serde_json::to_string_pretty(&value).unwrap(),
        )
        .unwrap();
    }
    root
}

/// Runs the `gate` binary (what `cargo xtask gate` runs) with `--root <root> <what>`, reports going under
/// `<out>/gates`; returns whether it succeeded, and its stdout and stderr together.
fn run_gate(root: &Path, what: &str, out: &Path) -> (bool, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_gate"))
        .arg("--root")
        .arg(root)
        .arg(what)
        .env("CARGO_TARGET_DIR", out)
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    (o.status.success(), text)
}

/// A fresh output directory for `case`.
fn out_dir(case: &str) -> PathBuf {
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("qa_m0_05_out")
        .join(case);
    let _ = fs::remove_dir_all(&out);
    out
}

/// The JSON report the binary wrote for input `stem`.
fn json_report(out: &Path, stem: &str) -> Value {
    let path = out.join("gates/convergence").join(format!("{stem}.json"));
    let text =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("no report {}: {e}", path.display()));
    serde_json::from_str(&text).unwrap()
}

/// The markdown report the binary wrote for input `stem`.
fn md_report(out: &Path, stem: &str) -> String {
    let path = out.join("gates/convergence").join(format!("{stem}.md"));
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("no report {}: {e}", path.display()))
}

// ---------------------------------------------------------------------------------------------------------------
// REQ-VAL-004 — the R-171 pass rule, at its edges
// ---------------------------------------------------------------------------------------------------------------

/// r = 1/32, 1/32, 1/64 exactly (binary-exact arithmetic): r does not *strictly* decrease, though the finest r is
/// far below 0.1. R-171: "pass iff r_k is strictly decreasing".
const EQUAL_STEPS: [f64; 4] = [64.0, 66.0, 68.0625, 69.1259765625];
/// r = 1/32, 1/64, 1/128 exactly: strictly decreasing, finest below 0.1.
const HALVING_STEPS: [f64; 4] = [64.0, 66.0, 67.03125, 67.554931640625];

#[test]
fn qa_m0_05_equal_steps_are_not_strictly_decreasing() {
    check_verdict(&EQUAL_STEPS, Verdict::Fail);
}

negative_control!(
    qa_m0_05_equal_steps_are_not_strictly_decreasing,
    "a strictly halving r (1/32, 1/64, 1/128), required to fail",
    expected = "the gate gave Pass",
    check_verdict(&HALVING_STEPS, Verdict::Fail)
);

#[test]
fn qa_m0_05_strictly_halving_steps_pass() {
    check_verdict(&HALVING_STEPS, Verdict::Pass);
}

negative_control!(
    qa_m0_05_strictly_halving_steps_pass,
    "equal steps (1/32, 1/32, 1/64), required to pass",
    expected = "the gate gave Fail",
    check_verdict(&EQUAL_STEPS, Verdict::Pass)
);

/// r = 1, 0.25, then exactly 1/10 == 0.1 in f64: the finest r equals the threshold. R-171: "the finest r_k below the
/// threshold" — equal is not below.
const FINEST_AT_THRESHOLD: [f64; 4] = [4.0, 8.0, 10.0, 11.0];
/// The same, finest r = 0.099: below.
const FINEST_UNDER_THRESHOLD: [f64; 4] = [4.0, 8.0, 10.0, 10.99];

#[test]
fn qa_m0_05_finest_step_equal_to_threshold_fails() {
    assert_eq!(
        (11.0f64 - 10.0) / 10.0,
        0.1,
        "the boundary case is not exactly 0.1"
    );
    check_verdict(&FINEST_AT_THRESHOLD, Verdict::Fail);
}

negative_control!(
    qa_m0_05_finest_step_equal_to_threshold_fails,
    "the finest r at 0.099, under the threshold, required to fail",
    expected = "the gate gave Pass",
    check_verdict(&FINEST_UNDER_THRESHOLD, Verdict::Fail)
);

#[test]
fn qa_m0_05_finest_step_under_threshold_passes() {
    check_verdict(&FINEST_UNDER_THRESHOLD, Verdict::Pass);
}

negative_control!(
    qa_m0_05_finest_step_under_threshold_passes,
    "the finest r at exactly the threshold, required to pass",
    expected = "the gate gave Fail",
    check_verdict(&FINEST_AT_THRESHOLD, Verdict::Pass)
);

/// REQ-VAL-004's statement: "a quantity whose largest relative step is at the finest sampling must not be reported as
/// converged". r = 0.05, 0.02, 0.06: every r is below 0.1, the largest is at the finest step.
const LARGEST_AT_FINEST: [f64; 4] = [1.0, 1.05, 1.071, 1.13526];
/// r = 0.05, 0.02, 0.01.
const SHRINKING_SMALL: [f64; 4] = [1.0, 1.05, 1.071, 1.08171];

#[test]
fn qa_m0_05_largest_step_at_finest_is_not_converged() {
    check_verdict(&LARGEST_AT_FINEST, Verdict::Fail);
    check_verdict(&SHRINKING_SMALL, Verdict::Pass);
}

negative_control!(
    qa_m0_05_largest_step_at_finest_is_not_converged,
    "a sequence whose finest step is its smallest, required to fail",
    expected = "the gate gave Pass",
    check_verdict(&SHRINKING_SMALL, Verdict::Fail)
);

/// r_k divides by |x_{k−1}| (R-171): a negative aggregate converging in magnitude has the same r as its mirror and
/// passes. Without the absolute value every r would be negative, and r would be increasing.
const NEGATIVE_CONVERGING: [f64; 4] = [-0.3000, -0.3200, -0.3260, -0.3275];
/// The pitfalls §3 record, negated.
const NEGATIVE_PITFALL: [f64; 4] = [-0.2153, -0.4423, -0.5494, -0.0947];

#[test]
fn qa_m0_05_negative_values_use_the_magnitude() {
    check_verdict(&NEGATIVE_CONVERGING, Verdict::Pass);
    check_verdict(&NEGATIVE_PITFALL, Verdict::Fail);
}

negative_control!(
    qa_m0_05_negative_values_use_the_magnitude,
    "the negated pitfalls record, required to pass",
    expected = "the gate gave Fail",
    check_verdict(&NEGATIVE_PITFALL, Verdict::Pass)
);

/// The rule is on r, not on x: x may overshoot and come back while r strictly shrinks (r = 0.5, 0.2, 0.025).
const OSCILLATING_CONVERGING: [f64; 4] = [1.0, 1.5, 1.2, 1.23];
/// x overshooting with r = 0.5, 0.2, 0.25.
const OSCILLATING_NOT: [f64; 4] = [1.0, 1.5, 1.2, 1.5];

#[test]
fn qa_m0_05_rule_is_on_r_not_on_x() {
    check_verdict(&OSCILLATING_CONVERGING, Verdict::Pass);
    check_verdict(&OSCILLATING_NOT, Verdict::Fail);
}

negative_control!(
    qa_m0_05_rule_is_on_r_not_on_x,
    "an oscillation whose finest r (0.25) exceeds the one before, required to pass",
    expected = "the gate gave Fail",
    check_verdict(&OSCILLATING_NOT, Verdict::Pass)
);

/// Each of `cases` (strides) is refused by the gate, not judged.
fn check_refused_orders(cases: &[&[u32]]) {
    for strides in cases {
        let values: Vec<f64> = (0..strides.len())
            .map(|k| 1.0 + 0.1 / (k as f64 + 1.0))
            .collect();
        let got = ConvergenceGate::judge(&real_config(), "qa.json", &input(strides, &values));
        assert!(got.is_err(), "strides {strides:?} were not refused");
    }
}

#[test]
fn qa_m0_05_orders_other_than_coarse_to_fine_are_refused() {
    check_refused_orders(&[
        &[0, 1, 4, 32],    // fine → coarse
        &[4, 32, 1, 0],    // one pair swapped
        &[32, 4, 0, 1],    // 0 not last
        &[32, 4, 1],       // no unstrided sample (0 last, deliverables)
        &[32, 4, 4, 0],    // a repeated stride is not strictly decreasing
        &[32, 4, 1, 0, 0], // a repeated 0
    ]);
}

negative_control!(
    qa_m0_05_orders_other_than_coarse_to_fine_are_refused,
    "R-171's own order, strides 32, 4, 1, 0, required to be refused",
    expected = "were not refused",
    check_refused_orders(&[&[32, 4, 1, 0]])
);

/// Strides strictly decreasing with 0 last are coarse → fine whatever the strides are (deliverables: "strides
/// strictly decreasing, 0 last").
fn check_accepted_order(strides: &[u32]) {
    let values = [0.3000, 0.3200, 0.3260, 0.3275];
    let got = ConvergenceGate::judge(&real_config(), "qa.json", &input(strides, &values));
    assert!(
        got.is_ok(),
        "strides {strides:?} were refused: {:?}",
        got.err()
    );
}

#[test]
fn qa_m0_05_any_strictly_decreasing_order_ending_at_zero_is_judged() {
    check_accepted_order(&[16, 8, 2, 0]);
    check_accepted_order(&[32, 4, 1, 0]);
}

negative_control!(
    qa_m0_05_any_strictly_decreasing_order_ending_at_zero_is_judged,
    "strides 16, 8, 2, 1 (no 0), required to be judged",
    expected = "were refused",
    check_accepted_order(&[16, 8, 2, 1])
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-VAL-004 / REQ-VAL-168 / PIT-3 — the runner end to end, on the checked-in fixtures
// ---------------------------------------------------------------------------------------------------------------

/// The binary on `root`: the pitfalls record fails, the converging fixture passes, the wrong-order fixture is refused
/// and writes no report; each report names the threshold as "provisional (REQ-VAL-135, R-171)"; the output prints
/// the region count seen and "minimum not yet calibrated" (REQ-VAL-168).
fn check_end_to_end(root: &Path, out: &Path) {
    let (ok, text) = run_gate(root, "convergence", out);
    assert!(ok, "the gate run failed:\n{text}");
    assert_eq!(
        json_report(out, "pitfall_escape_fraction")["verdict"],
        "fail",
        "the pitfalls §3 record is not reported as a fail:\n{text}"
    );
    assert_eq!(
        json_report(out, "converging")["verdict"],
        "pass",
        "converging.json did not pass:\n{text}"
    );
    assert!(
        !out.join("gates/convergence/wrong_order.json").exists(),
        "the wrong-order fixture was judged, not refused"
    );
    let refused_line = text
        .lines()
        .find(|l| l.contains("wrong_order.json"))
        .unwrap_or_else(|| panic!("wrong_order.json is not named:\n{text}"));
    assert!(
        refused_line.to_lowercase().contains("refused"),
        "{refused_line}"
    );
    assert!(
        text.contains("provisional (REQ-VAL-135, R-171)"),
        "no provisional threshold:\n{text}"
    );
    assert!(
        text.contains("minimum not yet calibrated"),
        "no uncalibrated minimum:\n{text}"
    );
    for stem in ["pitfall_escape_fraction", "converging"] {
        let md = md_report(out, stem);
        assert!(
            md.contains("provisional (REQ-VAL-135, R-171)"),
            "{stem}.md:\n{md}"
        );
        assert!(
            md.contains("not recorded"),
            "{stem}.md does not show the count as not recorded:\n{md}"
        );
        assert!(
            md.contains("minimum not yet calibrated"),
            "{stem}.md:\n{md}"
        );
    }
}

#[test]
fn qa_m0_05_binary_on_the_checked_in_fixtures() {
    check_end_to_end(&ws(), &out_dir("checked_in"));
}

negative_control!(
    qa_m0_05_binary_on_the_checked_in_fixtures,
    "the pitfalls fixture's values replaced by a converging sequence (and expected to pass)",
    expected = "the pitfalls §3 record is not reported as a fail",
    check_end_to_end(
        &sandbox("ctl_end_to_end", |name, v| {
            if name == "pitfall_escape_fraction.json" {
                v["series"][0]["values"] = json!([0.3000, 0.3200, 0.3260, 0.3275]);
                v["expected"] = json!("pass");
            }
        }),
        &out_dir("ctl_end_to_end")
    )
);

/// The checked-in convergence fixtures record their region count as "not recorded" (R-258 (2)), and `gate.json`
/// declares its region minimum against the calibration requirement REQ-VAL-168 with no value yet (R-258 (1)).
fn check_fixture_counts(dir: &Path) {
    for name in ["pitfall_escape_fraction.json", "converging.json"] {
        let v: Value = serde_json::from_str(&fs::read_to_string(dir.join(name)).unwrap()).unwrap();
        assert_eq!(
            v["region_count"], "not recorded",
            "{name} does not record its count as \"not recorded\""
        );
    }
    let gate: Value =
        serde_json::from_str(&fs::read_to_string(dir.join("gate.json")).unwrap()).unwrap();
    assert_eq!(gate["min_regions"]["requirement"], "REQ-VAL-168");
    assert!(
        gate["min_regions"]["value"].is_null(),
        "a region minimum is declared before calibration"
    );
}

#[test]
fn qa_m0_05_fixtures_record_their_count_as_not_recorded() {
    check_fixture_counts(&ws().join("fixtures/gates/convergence"));
}

negative_control!(
    qa_m0_05_fixtures_record_their_count_as_not_recorded,
    "a sandbox copy whose converging fixture records a count of 1",
    expected = "does not record its count",
    check_fixture_counts(
        &sandbox("ctl_counts", |name, v| if name == "converging.json" {
            v["region_count"] = json!(1);
        })
        .join("fixtures/gates/convergence")
    )
);

/// With a declared minimum of 1 (sandbox): the converging fixture, recorded as 1 region, is not flagged; the pitfalls
/// fixture, one series but its count "not recorded", is flagged all the same — "a conclusion drawn from an unrecorded
/// count is flagged like one from too few regions" (R-258 (2)), even when the series seen would meet the minimum.
fn check_unrecorded_flagged(root: &Path, out: &Path) {
    let (_, text) = run_gate(root, "convergence", out);
    let flags = |stem: &str| {
        json_report(out, stem)["flags"]
            .as_array()
            .cloned()
            .unwrap_or_default()
    };
    assert!(
        flags("converging").is_empty(),
        "1 region against a minimum of 1 is flagged:\n{text}"
    );
    assert!(
        !flags("pitfall_escape_fraction").is_empty(),
        "a conclusion from an unrecorded count is not flagged:\n{text}"
    );
    let md = md_report(out, "pitfall_escape_fraction");
    assert!(md.contains("not recorded"), "{md}");
}

/// A sandbox with a declared minimum of 1, the converging fixture recorded as 1 region, and the pitfalls fixture's
/// count `pitfall_count`.
fn min_one_sandbox(case: &str, pitfall_count: Value) -> PathBuf {
    sandbox(case, move |name, v| match name {
        "gate.json" => v["min_regions"]["value"] = json!(1),
        "converging.json" => v["region_count"] = json!(1),
        "pitfall_escape_fraction.json" => v["region_count"] = pitfall_count.clone(),
        _ => {}
    })
}

#[test]
fn qa_m0_05_unrecorded_count_is_flagged_like_too_few() {
    check_unrecorded_flagged(
        &min_one_sandbox("unrecorded", json!("not recorded")),
        &out_dir("unrecorded"),
    );
}

negative_control!(
    qa_m0_05_unrecorded_count_is_flagged_like_too_few,
    "the pitfalls fixture recording a count of 1, required to be flagged",
    expected = "a conclusion from an unrecorded count is not flagged",
    check_unrecorded_flagged(
        &min_one_sandbox("ctl_unrecorded", json!(1)),
        &out_dir("ctl_unrecorded")
    )
);

/// Each edit of `gate.json`'s threshold in `edits` is refused by the binary (nonzero exit, no report written):
/// "the runner refuses a numeric threshold that names no requirement or calibration requirement".
fn check_threshold_refused(edits: &[(&str, Value)]) {
    for (case, threshold) in edits {
        let t = threshold.clone();
        let root = sandbox(case, move |name, v| {
            if name == "gate.json" {
                v["threshold"] = t.clone();
            }
        });
        let out = out_dir(case);
        let (ok, text) = run_gate(&root, "convergence", &out);
        assert!(
            !ok,
            "{case}: the threshold {threshold} was not refused:\n{text}"
        );
        assert!(
            !out.join("gates/convergence/converging.json").exists(),
            "{case}: a report was written"
        );
    }
}

#[test]
fn qa_m0_05_threshold_naming_no_requirement_is_refused() {
    check_threshold_refused(&[
        (
            "t_none",
            json!({ "value": 0.1, "status": "provisional", "ruling": "R-171" }),
        ),
        (
            "t_blank",
            json!({ "value": 0.1, "requirement": "", "status": "provisional", "ruling": "R-171" }),
        ),
        (
            "t_unknown",
            json!({ "value": 0.1, "requirement": "REQ-VAL-9999", "status": "provisional", "ruling": "R-171" }),
        ),
        (
            "t_retired",
            json!({ "value": 0.1, "requirement": "REQ-CHART-042", "status": "confirmed" }),
        ),
        // A provisional value must stand under a calibration requirement (R-71); REQ-VAL-004 is not one.
        (
            "t_not_calibration",
            json!({ "value": 0.1, "requirement": "REQ-VAL-004", "status": "provisional", "ruling": "R-171" }),
        ),
    ]);
}

negative_control!(
    qa_m0_05_threshold_naming_no_requirement_is_refused,
    "the checked-in threshold (0.1 against REQ-VAL-135), required to be refused",
    expected = "was not refused",
    check_threshold_refused(&[(
        "ctl_t_ok",
        json!({ "value": 0.1, "requirement": "REQ-VAL-135", "status": "provisional", "ruling": "R-171" })
    )])
);

/// An unknown gate name is refused; `--list` names the convergence gate and writes no report.
fn check_names(name: &str) {
    let out = out_dir(&format!("names_{name}"));
    let (ok, text) = run_gate(&ws(), name, &out);
    assert!(!ok, "the unknown gate `{name}` was not refused:\n{text}");
    let out = out_dir(&format!("names_list_{name}"));
    let (ok, text) = run_gate(&ws(), "--list", &out);
    assert!(ok && text.contains("convergence"), "--list:\n{text}");
    assert!(!out.join("gates").exists(), "--list wrote a report");
}

#[test]
fn qa_m0_05_unknown_gate_is_refused() {
    check_names("no_such_gate");
}

negative_control!(
    qa_m0_05_unknown_gate_is_refused,
    "the registered gate `convergence`, required to be refused",
    expected = "was not refused",
    check_names("convergence")
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-VAL-002 — the report's sections and the region flag
// ---------------------------------------------------------------------------------------------------------------

/// A complete report; `f` may strip it.
fn report(f: impl FnOnce(&mut GateReport)) -> GateReport {
    let mut r = GateReport {
        gate: "convergence".into(),
        input: "qa.json".into(),
        description: "qa".into(),
        verdict: Verdict::Pass,
        threshold: real_config().threshold,
        scatter: vec!["r_k, q: 0.0667, 0.0188, 0.0046".into()],
        regions: Some(Regions {
            count: RegionCount::Recorded(4),
            minimum: Minimum::Declared {
                value: 4,
                requirement: "REQ-VAL-168".into(),
            },
        }),
        negative_results: vec!["q: largest relative step at stride 32 → 4".into()],
    };
    f(&mut r);
    r
}

/// `r` is refused by the writer, and nothing is written.
fn check_writer_refuses(r: &GateReport, case: &str) {
    let dir = out_dir(&format!("writer_{case}"));
    let got = r.write(&dir);
    assert!(got.is_err(), "{case}: the writer accepted the report");
    assert!(
        r.markdown().is_err() && r.json().is_err(),
        "{case}: a rendering accepted the report"
    );
    assert!(
        !dir.join("convergence").exists(),
        "{case}: a refused report left files"
    );
}

#[test]
fn qa_m0_05_writer_refuses_each_missing_or_blank_section() {
    check_writer_refuses(&report(|r| r.scatter.clear()), "no_scatter");
    check_writer_refuses(&report(|r| r.scatter = vec!["   ".into()]), "blank_scatter");
    check_writer_refuses(&report(|r| r.regions = None), "no_regions");
    check_writer_refuses(&report(|r| r.negative_results.clear()), "no_negative");
    check_writer_refuses(
        &report(|r| r.negative_results = vec!["".into(), "\t".into()]),
        "blank_negative",
    );
}

negative_control!(
    qa_m0_05_writer_refuses_each_missing_or_blank_section,
    "a complete report, required to be refused",
    expected = "the writer accepted the report",
    check_writer_refuses(&report(|_| {}), "ctl_complete")
);

/// A complete report writes markdown with a section for each of scatter, region count and negative results, and JSON
/// carrying them, with the threshold's source id.
fn check_written_sections(r: &GateReport, case: &str) {
    let dir = out_dir(&format!("sections_{case}"));
    let (md, js) = r
        .write(&dir)
        .unwrap_or_else(|e| panic!("{case}: refused: {e}"));
    let md = fs::read_to_string(md).unwrap();
    let headings: Vec<String> = md
        .lines()
        .filter(|l| l.starts_with('#'))
        .map(str::to_lowercase)
        .collect();
    for want in ["scatter", "region", "negative"] {
        assert!(
            headings.iter().any(|h| h.contains(want)),
            "{case}: no `{want}` section in:\n{md}"
        );
    }
    assert!(
        md.contains(&r.scatter[0]) && md.contains(&r.negative_results[0]),
        "{case}: content missing:\n{md}"
    );
    let v: Value = serde_json::from_str(&fs::read_to_string(js).unwrap()).unwrap();
    assert_eq!(
        v["threshold"]["requirement"], "REQ-VAL-135",
        "{case}: no threshold source id"
    );
}

#[test]
fn qa_m0_05_written_report_has_the_three_sections() {
    check_written_sections(&report(|_| {}), "complete");
}

negative_control!(
    qa_m0_05_written_report_has_the_three_sections,
    "a report without its region-count section",
    expected = "refused",
    check_written_sections(&report(|r| r.regions = None), "ctl_sections")
);

/// The flags on a conclusion from `count` against a declared minimum `min` (REQ-VAL-002: "conclusions from few
/// regions are flagged"; R-258: an unrecorded count flagged like too few). `flagged` is what is required.
fn check_flag(count: RegionCount, min: u32, flagged: bool) {
    let regions = Regions {
        count: count.clone(),
        minimum: Minimum::Declared {
            value: min,
            requirement: "REQ-VAL-168".into(),
        },
    };
    let got = !regions.flags().is_empty();
    assert_eq!(
        got, flagged,
        "a conclusion from {count:?} against a minimum of {min}: flagged = {got}"
    );
}

#[test]
fn qa_m0_05_region_flag_at_its_boundary() {
    check_flag(RegionCount::Recorded(4), 5, true); // fewer than declared
    check_flag(RegionCount::Recorded(0), 1, true);
    check_flag(RegionCount::Recorded(5), 5, false); // exactly the minimum is not "fewer"
    check_flag(RegionCount::Recorded(6), 5, false);
    check_flag(RegionCount::NotRecorded { series: 9 }, 5, true); // unrecorded: like too few, whatever was seen
    check_flag(RegionCount::NotRecorded { series: 1 }, 1, true);
}

negative_control!(
    qa_m0_05_region_flag_at_its_boundary,
    "exactly the declared minimum, required to be flagged",
    expected = "flagged = false",
    check_flag(RegionCount::Recorded(5), 5, true)
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-VAL-169 — scatter on a two-region input computed by hand
// ---------------------------------------------------------------------------------------------------------------

/// Two regions, strides 32, 4, 1, 0, the smaller final r in the FIRST region. By hand:
/// north: 0.8, 0.9, 0.91, 0.9105 → r = 0.1/0.8 = 0.125, 0.01/0.9 = 0.011111, 0.0005/0.91 = 0.000549;
/// south: 2.0, 2.5, 2.6, `south_last` (2.61) → r = 0.5/2 = 0.25, 0.1/2.5 = 0.04, 0.01/2.6 = 0.003846;
/// final r across the two: min 0.000549 (north), max 0.003846 (south).
fn two_regions(south_last: f64) -> Input {
    serde_json::from_value(json!({
        "description": "qa two regions",
        "strides": [32, 4, 1, 0],
        "series": [
            { "label": "north", "values": [0.8, 0.9, 0.91, 0.9105] },
            { "label": "south", "values": [2.0, 2.5, 2.6, south_last] }
        ],
        "region_count": 2
    }))
    .unwrap()
}

const NORTH_R: [f64; 3] = [0.125, 0.011_111_111, 0.000_549_450_5];
const SOUTH_R: [f64; 3] = [0.25, 0.04, 0.003_846_153_8];

/// Every number in `s` (digits, '.', leading '-').
fn numbers(s: &str) -> Vec<f64> {
    s.split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
        .filter_map(|t| {
            t.trim_matches('-')
                .parse::<f64>()
                .ok()
                .filter(|_| t.chars().any(|c| c.is_ascii_digit()))
        })
        .collect()
}

/// `got` matches `want` to the report's printing: within 5e-5 (four decimal places) or 1e-3 relative.
fn close(got: f64, want: f64) -> bool {
    (got - want).abs() <= 5e-5 || (got - want).abs() <= 1e-3 * want.abs()
}

/// The first number after `word` in `line`.
fn after(line: &str, word: &str) -> Option<f64> {
    line.split_once(word)
        .and_then(|(_, rest)| numbers(rest).into_iter().next())
}

fn check_scatter_by_hand(input: &Input) {
    let report = ConvergenceGate::judge(&real_config(), "qa_two.json", input).unwrap();
    let s = &report.scatter;
    for (label, want) in [("north", NORTH_R), ("south", SOUTH_R)] {
        let line = s
            .iter()
            .find(|l| l.contains(label))
            .unwrap_or_else(|| panic!("no scatter line for region {label}: {s:?}"));
        let got = numbers(line.split_once(label).unwrap().1);
        assert!(
            got.len() == 3 && got.iter().zip(want).all(|(g, w)| close(*g, w)),
            "region {label}'s r_k are {got:?}, not the hand-computed {want:?}"
        );
    }
    let spread = s
        .iter()
        .find(|l| {
            !l.contains("north") && !l.contains("south") && l.contains("min") && l.contains("max")
        })
        .unwrap_or_else(|| panic!("no min–max line in the scatter: {s:?}"));
    let (min, max) = (after(spread, "min").unwrap(), after(spread, "max").unwrap());
    assert!(
        close(min, NORTH_R[2]) && close(max, SOUTH_R[2]),
        "the final-r min–max is {min}–{max}, not the hand-computed {}–{}",
        NORTH_R[2],
        SOUTH_R[2]
    );
    let md = report.markdown().unwrap();
    let section = md
        .split("\n#")
        .find(|sec| {
            sec.to_lowercase()
                .trim_start_matches('#')
                .trim_start()
                .starts_with("scatter")
        })
        .unwrap_or_else(|| panic!("no scatter section:\n{md}"));
    assert!(
        section.contains("north") && section.contains("south"),
        "the scatter section:\n{section}"
    );
}

#[test]
fn qa_m0_05_scatter_two_regions_by_hand() {
    check_scatter_by_hand(&two_regions(2.61));
}

negative_control!(
    qa_m0_05_scatter_two_regions_by_hand,
    "south ending at 2.62 (final r 0.0077), in place of 2.61",
    expected = "not the hand-computed",
    check_scatter_by_hand(&two_regions(2.62))
);
