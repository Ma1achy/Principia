//! QA tests for TASK-M0-08, written from REQ-SYS-001 (every settled constant carries a citation), REQ-SYS-005 (every
//! constant records its admissibility class, philosophy §4.2), REQ-VAL-006 (a threshold on a quantity spanning
//! decades is relative, set from the observed distribution or a measured gap; pitfalls §3; the regressions closure
//! 2e-3 and `tau_display` at the 0.4th percentile fail) and REQ-SYS-063 (the register as dd_generation_root §3.8
//! defines it), not from the implementation. Each test has a registered negative control (R-176).

use ledger::constants::{
    gate, Admissibility, Citation, ConstantBuilder, Population, RelativeBasis, Value, REGISTER,
};
use validation::negative_control;

const PITFALLS: Citation = Citation::Corpus {
    file: "docs/read_first/principia_01_pitfalls.md",
    section: "3. Standing rules earned in this sequence",
};

fn complete(name: &'static str) -> ConstantBuilder {
    ConstantBuilder {
        name,
        value: Some(Value::Exact(1.0)),
        class: Some(Admissibility::CanonicalUnits),
        citation: Some(PITFALLS),
        relative_basis: None,
    }
}

/// `register` is refused by the generation gate, and the refusal names each of `names`.
fn refused_naming(register: &[ConstantBuilder], names: &[&str]) {
    let message = match gate(register) {
        Ok(_) => panic!("the gate admitted the register"),
        Err(e) => e.to_string(),
    };
    for name in names {
        assert!(
            message.contains(name),
            "refused, but the refusal does not name `{name}`: {message}"
        );
    }
}

// ---- REQ-SYS-001 / REQ-SYS-005: a missing citation or class is refused, naming each offending constant ----

fn uncited_and_unclassed(
    uncited: Option<Citation>,
    unclassed: Option<Admissibility>,
) -> Vec<ConstantBuilder> {
    vec![
        complete("qa_fine"),
        ConstantBuilder {
            citation: uncited,
            ..complete("qa_uncited")
        },
        ConstantBuilder {
            class: unclassed,
            ..complete("qa_unclassed")
        },
    ]
}

#[test]
fn qa_constants_gate_names_every_incomplete_constant() {
    refused_naming(
        &uncited_and_unclassed(None, None),
        &["qa_uncited", "qa_unclassed"],
    );
}

negative_control!(
    qa_constants_gate_names_every_incomplete_constant,
    "with both entries complete the gate admits the register, so the refusal check must fail",
    expected = "the gate admitted the register",
    refused_naming(
        &uncited_and_unclassed(Some(PITFALLS), Some(Admissibility::ConservationLaw)),
        &["qa_uncited"]
    )
);

#[test]
fn qa_constants_gate_refuses_a_missing_value() {
    refused_naming(
        &[ConstantBuilder {
            value: None,
            ..complete("qa_valueless")
        }],
        &["qa_valueless"],
    );
}

negative_control!(
    qa_constants_gate_refuses_a_missing_value,
    "an entry with a value is complete, so the refusal check must fail",
    expected = "the gate admitted the register",
    refused_naming(&[complete("qa_valueless")], &["qa_valueless"])
);

// ---- R-71 as §3.8 defines it: a value is pending exactly when its citation is its calibration requirement ----

#[test]
fn qa_constants_gate_pending_value_must_cite_its_calibration_requirement() {
    // A pending value cited to a corpus section, and a settled value cited to a calibration requirement.
    refused_naming(
        &[ConstantBuilder {
            value: Some(Value::Calibration),
            ..complete("qa_pending_uncalibrated")
        }],
        &["qa_pending_uncalibrated"],
    );
    refused_naming(
        &[ConstantBuilder {
            citation: Some(Citation::Calibration("REQ-QA-000")),
            ..complete("qa_settled_but_calibration")
        }],
        &["qa_settled_but_calibration"],
    );
}

negative_control!(
    qa_constants_gate_pending_value_must_cite_its_calibration_requirement,
    "a pending value citing its calibration requirement is admissible (R-71), so the refusal check must fail",
    expected = "the gate admitted the register",
    refused_naming(
        &[ConstantBuilder {
            value: Some(Value::Calibration),
            citation: Some(Citation::Calibration("REQ-QA-000")),
            ..complete("qa_pending")
        }],
        &["qa_pending"]
    )
);

// ---- REQ-VAL-006: thresholds ----

const fn pop(name: &'static str, lo: f64, hi: f64) -> Population {
    Population { name, lo, hi }
}

fn threshold(name: &'static str, v: f64, basis: Option<RelativeBasis>) -> ConstantBuilder {
    ConstantBuilder {
        value: Some(Value::Threshold(v)),
        relative_basis: basis,
        ..complete(name)
    }
}

fn distribution(percentile: f64, populations: &'static [Population]) -> Option<RelativeBasis> {
    Some(RelativeBasis::Distribution {
        source: PITFALLS,
        percentile,
        populations,
    })
}

/// The closure regression under a distribution basis: 2e-3 inside the bound population's range (pitfalls §3), with
/// the escape population above it. The ranges' ends are this fixture's; the recorded fact is that 2e-3 is inside.
const CLOSURE: &[Population] = &[pop("bound", 1e-7, 1e-1), pop("escape", 1.0, 10.0)];

#[test]
fn qa_constants_threshold_closure_inside_bound_range_fails_under_a_distribution_basis() {
    refused_naming(
        &[threshold(
            "closure_cutoff",
            2e-3,
            distribution(50.0, CLOSURE),
        )],
        &["closure_cutoff", "bound"],
    );
}

negative_control!(
    qa_constants_threshold_closure_inside_bound_range_fails_under_a_distribution_basis,
    "a cutoff between the bound and escape populations separates them, so the refusal check must fail",
    expected = "the gate admitted the register",
    refused_naming(
        &[threshold("closure_cutoff", 0.5, distribution(50.0, CLOSURE))],
        &["closure_cutoff"]
    )
);

/// Two populations the fixture threshold 0.5 lies between.
const SPLIT: &[Population] = &[pop("smooth", 1e-3, 1e-1), pop("structured", 1.0, 1e2)];

/// REQ-VAL-006 verify: "a threshold ... at an extreme percentile (tau_display at the 0.4th percentile) fails review".
/// philosophy §4.2: at the 0.4th percentile the split predicate is true for 99.6% of quads. The requirement fails
/// the threshold for its percentile, whatever populations its basis names.
fn tau_display_at(percentile: f64) {
    refused_naming(
        &[threshold(
            "tau_display",
            0.5,
            distribution(percentile, SPLIT),
        )],
        &["tau_display"],
    );
}

#[test]
fn qa_constants_threshold_tau_display_at_the_0_4th_percentile_fails_whatever_its_populations() {
    tau_display_at(0.4);
}

negative_control!(
    qa_constants_threshold_tau_display_at_the_0_4th_percentile_fails_whatever_its_populations,
    "the same threshold at the median of its distribution is not at an extreme percentile, so the refusal must fail",
    expected = "the gate admitted the register",
    tau_display_at(50.0)
);

/// §3.8 (the definition this task writes): "a threshold separates populations". One below every population, or
/// above every one, separates nothing.
fn outside_every_population(v: f64) {
    refused_naming(
        &[threshold("qa_outside", v, distribution(50.0, SPLIT))],
        &["qa_outside"],
    );
}

#[test]
fn qa_constants_threshold_beyond_every_population_separates_nothing() {
    outside_every_population(1e-5);
    outside_every_population(1e5);
}

negative_control!(
    qa_constants_threshold_beyond_every_population_separates_nothing,
    "a threshold between the two populations separates them, so the refusal check must fail",
    expected = "the gate admitted the register",
    outside_every_population(0.5)
);

/// A NaN threshold compares false with every range end, so it is "outside" every population; it is set from no
/// distribution and no gap, and must be refused.
fn nan_threshold(v: f64) {
    refused_naming(
        &[threshold("qa_nan", v, distribution(50.0, SPLIT))],
        &["qa_nan"],
    );
}

#[test]
fn qa_constants_threshold_nan_is_refused() {
    nan_threshold(f64::NAN);
}

negative_control!(
    qa_constants_threshold_nan_is_refused,
    "a finite threshold between the populations is admissible, so the refusal check must fail",
    expected = "the gate admitted the register",
    nan_threshold(0.5)
);

// ---- The register's M0 entries (task Deliverables; payload §1, §3; R-86; R-245) ----

/// The values the task names, each derived here from its definition rather than copied.
fn m0_values() -> Vec<f64> {
    let horizon = f64::from(u16::MAX); // R-86: the greatest count the exact u16 `times` holds
    let capacity = 1.0 + ((121.0 - 2.0) / 3f64.log2()).floor(); // payload §3
    let sentinel = f64::from((1u32 << 7) - 1); // payload §3: the 7-bit length field's greatest value
    let f16_max = (2.0 - 2f64.powi(-10)) * 2f64.powi(15); // binary16's greatest finite value
    vec![horizon, capacity, sentinel, f16_max]
}

/// Every value the task names is in `register`, settled and passing the gate; no entry is the dropped diffusion
/// sentinel −1.0 (R-245).
fn check_m0_register(register: &[ConstantBuilder]) {
    let built = gate(register).unwrap_or_else(|e| panic!("the register is refused: {e}"));
    let settled: Vec<f64> = built
        .iter()
        .filter_map(|c| match c.value {
            Value::Exact(v) | Value::Threshold(v) => Some(v),
            Value::Calibration => None,
        })
        .collect();
    for v in m0_values() {
        assert!(
            settled.contains(&v),
            "the register lacks the M0 value {v}: {settled:?}"
        );
    }
    assert!(
        !settled.contains(&-1.0),
        "the register holds a diffusion sentinel −1.0, dropped by R-245"
    );
    assert_eq!(
        ledger::check::F16_MAX,
        m0_values()[3],
        "the layout check's f16 bound is not binary16's greatest finite value"
    );
}

#[test]
fn qa_constants_register_holds_the_m0_values() {
    assert_eq!(m0_values(), [65535.0, 76.0, 127.0, 65504.0]);
    check_m0_register(REGISTER);
}

negative_control!(
    qa_constants_register_holds_the_m0_values,
    "a register without the word capacity must fail the check",
    expected = "the register lacks the M0 value 76",
    check_m0_register(&[
        ledger::constants::HORIZON_STEPS_MAX,
        ledger::constants::FGW_LENGTH_SENTINEL,
        ledger::constants::F16_FINITE_MAX,
    ])
);
