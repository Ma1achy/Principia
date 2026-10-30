//! The `decode-agreement` gate's judgement off the GPU (TASK-M2-25, TASK-M2-29; REQ-COL-006, REQ-COL-060): a sample's
//! tally at the tolerance's boundary, the hatch's placement, the measurement and its report lines, and the refusals.
//! `cargo xtask gate decode-agreement` runs it on the GPU.

use render::presets::agreement::FACTOR;
use render::presets::DecodeChart;
use validation::decode_agreement::{check, DecodeAgreementGate, Input, Sample, Tally};
use validation::gate::GateConfig;
use validation::negative_control;

const EPS: f64 = f32::EPSILON as f64;

fn s(ratio: f32, valid: bool, finite: bool) -> Sample {
    Sample {
        ratio,
        valid,
        finite,
        diff: 0.0,
        scale: 1.0,
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

const REQUIREMENTS: &str =
    "- id: REQ-DEC-043\n  kind: calibration\n- id: REQ-COL-060\n  kind: calibration\n";

fn config(threshold: &str) -> GateConfig {
    GateConfig::parse(
        &format!(
            r#"{{"gate": "decode-agreement", "inputs": ["healthy.json"],
            "threshold": {{"value": {threshold}, "requirement": "REQ-COL-060", "status": "provisional", "ruling": "R-71"}},
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
    let f = f64::from(FACTOR);
    for t in [
        f - 1.0,
        f + 1.0,
        f * (1.0 + 1e-6),
        f.next_down(),
        f.next_up(),
    ] {
        let e = check(&config(&format!("{t:?}")), &input([16, 16], 4, 0, vec![]))
            .expect_err("a threshold other than the preset's factor");
        assert!(e.contains("compiled factor"), "{e}");
        assert!(e.contains("REQ-COL-060"), "{e}");
    }
}

#[test]
fn decode_agreement_refusals() {
    check_refusals(&format!("{:?}", f64::from(FACTOR)));
}

negative_control!(
    decode_agreement_refusals,
    "a configuration whose threshold is not the preset's factor must be refused",
    expected = "the healthy input is refused",
    check_refusals(&format!("{:?}", f64::from(FACTOR) - 1.0))
);

/// The gate's configuration reads REQ-COL-060's provisional factor, the preset's compiled one.
fn check_gate_json(threshold: f64) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let config = validation::gate::config(&root, "decode-agreement").unwrap();
    assert_eq!(
        config.threshold.requirement, "REQ-COL-060",
        "the agreement gate's threshold is not REQ-COL-060's"
    );
    assert_eq!(
        config.threshold.status,
        validation::gate::Status::Provisional
    );
    assert_eq!(
        config.threshold.value, threshold,
        "the gate's threshold is not the preset's compiled factor"
    );
    assert!(
        config.inputs.iter().any(|i| i == "survey.json"),
        "the measurement's survey is not an input"
    );
}

#[test]
fn decode_agreement_gate_json() {
    check_gate_json(f64::from(FACTOR));
}

negative_control!(
    decode_agreement_gate_json,
    "a gate.json threshold other than the compiled factor must be caught",
    expected = "the gate's threshold is not the preset's compiled factor",
    check_gate_json(f64::from(FACTOR) + 1.0)
);

/// The words `(ratio, valid, E, E₀)` of a compared sample.
fn words(e: f32, e_0: f32) -> [u32; 4] {
    [0.5f32.to_bits(), 1, e.to_bits(), e_0.to_bits()]
}

/// A sample's measurement: `|E − E₀|` at f64 from the words, and the scale from the stored `K₀`, `V₀`, each `max` at its
/// edge; `k_0` is the stored `K₀`, 1 in the honest run, where `max(|K₀|, 1)` is 1 from either side.
fn check_measure(k_0: f32) {
    let one = Sample::from_words(words(1.5, 1.0), [k_0, -1.0], true);
    assert_eq!(one.diff, 0.5);
    assert_eq!(one.scale, 2.0 * EPS, "the scale at |K₀| = |V₀| = 1");
    assert_eq!(one.factor(), 0.25 / EPS);
    let below = Sample::from_words(
        words(1.0, 1.0),
        [1.0f32.next_down(), (-1.0f32).next_up()],
        true,
    );
    assert_eq!(below.scale, 2.0 * EPS, "max(|x|, 1) under 1 is not 1");
    assert_eq!((below.diff, below.factor()), (0.0, 0.0));
    let above = Sample::from_words(words(-3.0, -2.0), [1.0f32.next_up(), -4.0], true);
    assert_eq!(
        above.scale,
        EPS * (f64::from(1.0f32.next_up()) + 4.0),
        "max(|x|, 1) over 1 is not |x|"
    );
    assert_eq!(above.diff, 1.0, "|E − E₀| is not the absolute difference");
    let nan = Sample::from_words(words(f32::NAN, 1.0), [0.0, 0.0], true);
    assert_eq!(nan.factor(), f64::INFINITY, "a NaN difference is not +∞");
    assert!(nan.diff.is_nan());
    let s = Sample::from_words([7, 0, 0, 0], [0.0, 0.0], false);
    assert_eq!((s.ratio.to_bits(), s.valid, s.finite), (7, false, false));
}

#[test]
fn decode_agreement_measure() {
    check_measure(1.0);
}

negative_control!(
    decode_agreement_measure,
    "a scale that mis-reads the stored K₀ must be caught",
    expected = "the scale at |K₀| = |V₀| = 1",
    check_measure(2.0)
);

/// The tally's measurement: the worst factor and difference over the compared samples only, NaN read as `+∞`;
/// `count_hatched` also feeds a hatched sample's difference in, the control.
fn check_tally_measure(count_hatched: bool) {
    let m = |diff: f64, scale: f64, valid: bool| Sample {
        ratio: 0.5,
        valid,
        finite: valid,
        diff,
        scale,
    };
    let mut samples = vec![m(3.0, 1.0, true), m(1.0, 0.125, true), m(0.5, 2.0, true)];
    let hatched = m(100.0, 1.0, false);
    samples.push(if count_hatched {
        Sample {
            valid: true,
            ..hatched
        }
    } else {
        hatched
    });
    let t = tally(&samples);
    assert_eq!(
        (t.worst_factor, t.worst_diff),
        (8.0, 3.0),
        "the measurement is not the compared samples' worst"
    );
    let nan = tally(&[m(f64::NAN, 1.0, true), m(1.0, 1.0, true)]);
    assert_eq!(
        (nan.worst_factor, nan.worst_diff),
        (f64::INFINITY, f64::INFINITY)
    );
    let none = tally(&[m(5.0, 1.0, false)]);
    assert_eq!((none.worst_factor, none.worst_diff), (0.0, 0.0));
}

#[test]
fn decode_agreement_tally_measure() {
    check_tally_measure(false);
}

negative_control!(
    decode_agreement_tally_measure,
    "a hatched sample's difference counted in the measurement must be caught",
    expected = "the measurement is not the compared samples' worst",
    check_tally_measure(true)
);

/// The report's measurement lines: each chart's worst factor and difference and its excluded count, and the worst
/// over every chart beside the threshold; `charts` is how many of the two tallies the report is given.
fn check_report(charts: usize) {
    let c = config(&format!("{:?}", f64::from(FACTOR)));
    let m = |diff: f64, valid: bool| Sample {
        ratio: 0.5,
        valid,
        finite: valid,
        diff,
        scale: 1.0,
    };
    let tallies = [
        Tally::of(
            DecodeChart::Latent,
            &[m(3.0, true), m(9.0, false), m(1.0, true)],
        ),
        Tally::of(DecodeChart::MassSimplex, &[m(7.25, true)]),
    ];
    let r = DecodeAgreementGate::report(
        &c,
        "survey.json",
        &input([2, 2], 1, 0, vec![]),
        "adapter",
        &tallies[..charts],
    );
    let latent = r
        .scatter
        .iter()
        .find(|l| l.starts_with("latent: "))
        .expect("no latent line");
    assert!(
        latent.contains("3 samples, 2 compared, 1 excluded")
            && latent.contains("measured worst |E − E₀| 3.000 eps")
            && latent.contains("3.000e0 absolute"),
        "{latent}"
    );
    let last = r.scatter.last().unwrap();
    assert!(
        last.starts_with("every chart: measured worst |E − E₀| 7.250 eps"),
        "the summary is not the worst over every chart: {last}"
    );
    assert!(
        last.contains(&format!(
            "the tolerance's {} eps (REQ-COL-060)",
            f64::from(FACTOR)
        )),
        "{last}"
    );
}

#[test]
fn decode_agreement_report_measure() {
    check_report(2);
}

negative_control!(
    decode_agreement_report_measure,
    "a summary that misses a chart's worst must be caught",
    expected = "the summary is not the worst over every chart",
    check_report(1)
);
