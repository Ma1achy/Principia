//! The `decode-agreement` gate's judgement off the GPU (TASK-M2-25; REQ-COL-006): a sample's tally at the tolerance's
//! boundary, the hatch's placement, and the refusals. `cargo xtask gate decode-agreement` runs it on the GPU.

use render::presets::DecodeChart;
use validation::decode_agreement::{check, Input, Sample, Tally};
use validation::gate::GateConfig;
use validation::negative_control;

fn s(ratio: f32, valid: bool, finite: bool) -> Sample {
    Sample {
        ratio,
        valid,
        finite,
    }
}

fn tally(samples: &[Sample]) -> Tally {
    Tally::of(DecodeChart::Latent, samples)
}

/// The tally at the boundary: a ratio of exactly 1 agrees, the next f32 above it does not, NaN does not.
fn check_boundary(bound: f32) {
    let at = tally(&[s(bound, true, true), s(0.0, true, true)]);
    assert_eq!(
        (at.disagree, at.valid, at.misplaced),
        (0, 2, 0),
        "a ratio of {bound} is over the tolerance"
    );
    assert!(at.pass(), "a ratio of {bound} fails the chart");
    assert_eq!(at.worst, f64::from(bound));
    let over = tally(&[s(f32::from_bits(1.0f32.to_bits() + 1), true, true)]);
    assert_eq!(over.disagree, 1, "a ratio past 1 agrees");
    assert!(!over.pass());
    let nan = tally(&[s(f32::NAN, true, true)]);
    assert_eq!(nan.disagree, 1, "a NaN ratio agrees");
    assert_eq!(nan.worst, f64::INFINITY);
}

#[test]
fn decode_agreement_tally_boundary() {
    check_boundary(1.0);
}

negative_control!(
    decode_agreement_tally_boundary,
    "a tally that let a ratio past 1 agree must be caught",
    expected = "is over the tolerance",
    check_boundary(f32::from_bits(1.0f32.to_bits() + 1))
);

/// The hatch's placement: an invalid sample over non-finite operands is excluded and fine; one over finite operands,
/// or a valid one over a non-finite operand, is misplaced; a chart with nothing compared fails.
fn check_hatch(valid_over_finite: bool) {
    let excluded = tally(&[s(9.0, false, false), s(0.5, true, true)]);
    assert_eq!(
        (excluded.valid, excluded.disagree, excluded.misplaced),
        (1, 0, 0)
    );
    assert!(
        excluded.pass(),
        "an excluded sample's ratio counted against the chart"
    );
    let hatched = tally(&[s(0.5, false, valid_over_finite), s(0.5, true, true)]);
    assert_eq!(
        hatched.misplaced, 1,
        "a hatch over finite operands is not counted misplaced"
    );
    assert!(!hatched.pass());
    let compared = tally(&[s(0.5, true, false), s(0.5, true, true)]);
    assert_eq!(compared.misplaced, 1);
    assert!(!compared.pass());
    let none = tally(&[s(0.0, false, false)]);
    assert_eq!((none.valid, none.misplaced, none.worst), (0, 0, 0.0));
    assert!(!none.pass(), "a chart with no compared sample passes");
    assert!(!tally(&[]).pass());
}

#[test]
fn decode_agreement_tally_hatch() {
    check_hatch(true);
}

negative_control!(
    decode_agreement_tally_hatch,
    "a hatch over non-finite operands taken as misplaced must be caught",
    expected = "a hatch over finite operands is not counted misplaced",
    check_hatch(false)
);

const REQUIREMENTS: &str = "- id: REQ-DEC-043\n  kind: calibration\n";

fn config(threshold: &str) -> GateConfig {
    GateConfig::parse(
        &format!(
            r#"{{"gate": "decode-agreement", "inputs": ["healthy.json"],
            "threshold": {{"value": {threshold}, "requirement": "REQ-DEC-043", "status": "provisional", "ruling": "R-71"}},
            "min_regions": {{"value": null, "requirement": "REQ-DEC-043"}}}}"#
        ),
        REQUIREMENTS,
    )
    .unwrap()
}

fn input(side: [u32; 2], depth: u32, shift: u32, poison: Vec<u32>) -> Input {
    Input {
        description: String::new(),
        side,
        depth,
        shift,
        poison,
    }
}

/// The refusals, each at its edge; `factor` is the threshold the healthy configuration gives.
fn check_refusals(factor: &str) {
    let c = config(factor);
    let ok = |i: Input| check(&c, &i);
    ok(input([16, 16], 4, 0, vec![])).expect("the healthy input is refused");
    ok(input([16, 16], 4, 255, vec![255])).expect("the last shift and sample are refused");
    ok(input([1, 1], 0, 0, vec![])).expect("one sample at depth 0 is refused");
    ok(input([2, 1], 30, 0, vec![])).expect("depth 30 is refused");
    let refused = |i: Input, why: &str| {
        let e = ok(i).expect_err(why);
        assert!(e.starts_with("refused: "), "{e}");
    };
    refused(input([16, 16], 4, 256, vec![]), "a shift of every sample");
    refused(
        input([16, 16], 4, 0, vec![256]),
        "a poisoned sample past the last",
    );
    refused(input([0, 16], 4, 0, vec![]), "an empty lattice");
    refused(input([16, 0], 4, 0, vec![]), "an empty lattice");
    refused(input([17, 1], 4, 0, vec![]), "more samples than cells on x");
    refused(input([1, 17], 4, 0, vec![]), "more samples than cells on y");
    refused(input([1, 1], 31, 0, vec![]), "depth 31");
    for t in ["255", "257", "256.0001"] {
        let e = check(&config(t), &input([16, 16], 4, 0, vec![]))
            .expect_err("a threshold other than the preset's factor");
        assert!(e.contains("compiled factor"), "{e}");
    }
}

#[test]
fn decode_agreement_refusals() {
    check_refusals("256");
}

negative_control!(
    decode_agreement_refusals,
    "a configuration whose threshold is not the preset's factor must be refused",
    expected = "the healthy input is refused",
    check_refusals("255")
);
