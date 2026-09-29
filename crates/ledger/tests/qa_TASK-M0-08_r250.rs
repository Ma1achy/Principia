//! QA tests for TASK-M0-08 (re-check), written from REQ-VAL-006 as R-250 reads it: a threshold must be finite and sit
//! between populations of its own distribution, with at least one population wholly below and one wholly above it;
//! its percentile is derived from the populations' counts; a threshold inside any population fails, at whatever
//! percentile; there is no numeric bound (so one between populations passes at whatever percentile). The derivation
//! `100 · (count wholly below) / (total count)` is dd_generation_root §3.8's. Each test has a registered negative
//! control (R-176). The ranges and counts are this file's fixtures; only "the 0.4th percentile" is the corpus's.

use ledger::constants::{
    gate, Admissibility, Citation, ConstantBuilder, Population, RelativeBasis, Value,
};
use validation::negative_control;

const PITFALLS: Citation = Citation::Corpus {
    file: "docs/read_first/principia_01_pitfalls.md",
    section: "3. Standing rules earned in this sequence",
};

const fn pop(name: &'static str, lo: f64, hi: f64, count: u64) -> Population {
    Population {
        name,
        lo,
        hi,
        count,
    }
}

fn threshold(
    name: &'static str,
    v: f64,
    percentile: f64,
    populations: &'static [Population],
) -> ConstantBuilder {
    ConstantBuilder {
        name,
        value: Some(Value::Threshold(v)),
        class: Some(Admissibility::CanonicalUnits),
        citation: Some(PITFALLS),
        relative_basis: Some(RelativeBasis::Distribution {
            source: PITFALLS,
            percentile,
            populations,
        }),
    }
}

/// `entry` is refused by the generation gate, and the refusal names each of `names`.
fn refused_naming(entry: ConstantBuilder, names: &[&str]) {
    let message = match gate(&[entry]) {
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

/// `entry` is admitted by the generation gate.
fn admitted(entry: ConstantBuilder) {
    if let Err(e) = gate(&[entry]) {
        panic!("the gate refused an admissible threshold: {e}");
    }
}

// ---- A threshold inside a population fails at whatever percentile, even with neighbours on both sides ----

/// A thin population `mid` holding the threshold 5e-3, with `low` (4 of 1000 observations) wholly below and `high`
/// wholly above. The recorded 0.4th percentile is the one the counts give (4 of 1000 wholly below), so nothing but
/// the threshold's lying inside `mid` makes it inadmissible.
const MID_HOLDS_IT: &[Population] = &[
    pop("low", 1e-4, 1e-3, 4),
    pop("mid", 4e-3, 6e-3, 1),
    pop("high", 1e-2, 1e2, 995),
];

#[test]
fn qa_r250_threshold_inside_a_population_fails_with_populations_below_and_above() {
    refused_naming(
        threshold("tau_display", 5e-3, 0.4, MID_HOLDS_IT),
        &["tau_display", "mid"],
    );
}

negative_control!(
    qa_r250_threshold_inside_a_population_fails_with_populations_below_and_above,
    "with `mid` moved wholly above the threshold, the same 0.4th percentile sits between populations and is \
     admissible (R-250), so the refusal must fail",
    expected = "the gate admitted the register",
    {
        const MID_ABOVE: &[Population] = &[
            pop("low", 1e-4, 1e-3, 4),
            pop("mid", 6e-3, 8e-3, 1),
            pop("high", 1e-2, 1e2, 995),
        ];
        refused_naming(threshold("tau_display", 5e-3, 0.4, MID_ABOVE), &["tau_display"])
    }
);

// ---- One between populations passes at whatever percentile, its percentile derived from the counts ----

const RARE_LOW: &[Population] = &[pop("low", 1e-4, 1e-3, 1), pop("high", 1e-2, 1e2, 999)];
const EVEN: &[Population] = &[pop("low", 1e-4, 1e-3, 500), pop("high", 1e-2, 1e2, 500)];
const RARE_HIGH: &[Population] = &[pop("low", 1e-4, 1e-3, 999), pop("high", 1e-2, 1e2, 1)];
/// Two populations wholly below: the percentile counts both (3 + 1 of 1000), not the nearest alone.
const TWO_BELOW: &[Population] = &[
    pop("lower", 1e-5, 1e-4, 3),
    pop("low", 2e-4, 1e-3, 1),
    pop("high", 1e-2, 1e2, 996),
];

/// The threshold 5e-3 between `low` and `high` of each fixture, recording the percentile its counts give; `v` for
/// the control.
fn between_at_whatever_percentile(v: f64) {
    admitted(threshold("qa_between", v, 0.1, RARE_LOW));
    admitted(threshold("qa_between", v, 50.0, EVEN));
    admitted(threshold("qa_between", v, 99.9, RARE_HIGH));
    admitted(threshold("qa_between", v, 0.4, TWO_BELOW));
}

#[test]
fn qa_r250_threshold_between_populations_passes_at_whatever_percentile() {
    between_at_whatever_percentile(5e-3);
}

negative_control!(
    qa_r250_threshold_between_populations_passes_at_whatever_percentile,
    "the same thresholds moved inside the `low` population are inadmissible, so the admission check must fail",
    expected = "the gate refused an admissible threshold",
    between_at_whatever_percentile(5e-4)
);
