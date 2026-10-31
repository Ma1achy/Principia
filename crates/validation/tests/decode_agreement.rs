//! The `decode-agreement` gate's judgement off the GPU (TASK-M2-25, TASK-M2-29; REQ-COL-006, REQ-COL-060): a sample's
//! tally at the tolerance's boundary in either form, the hatch's placement, the measurement and its report lines, the
//! collision windows' runs, the refusals, and the operands the hatch is judged on (the payload's `K₀` and `V₀` and the
//! fragment decode's own `K`, `V`, `m` and `r`, never the decode stage's). `cargo xtask gate decode-agreement` runs it
//! on the GPU.

use render::presets::agreement::FACTOR;
use render::presets::DecodeChart;
use validation::decode_agreement::{
    chart_named, check, operands_finite, quantile, runs, store_exact, DecodeAgreementGate, Form,
    Input, Run, Sample, Stored, Tally, Window,
};
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
        depth: Some(depth),
        windows: vec![],
        form: Form::Conditioned,
        stored: Stored::Stage,
        shift,
        poison,
    }
}

/// An input of `windows`, `side` samples each.
fn windowed(side: [u32; 2], windows: Vec<Window>) -> Input {
    Input {
        depth: None,
        windows,
        ..input(side, 0, 0, vec![])
    }
}

fn window(chart: &str, centre: [f64; 2], depths: Vec<u32>) -> Window {
    Window {
        chart: chart.to_owned(),
        centre,
        depths,
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
    for name in [
        "survey.json",
        "collision.json",
        "collision_per_operand.json",
    ] {
        assert!(
            config.inputs.iter().any(|i| i == name),
            "the measurement's {name} is not an input"
        );
    }
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

/// A sample's measurement: `|E − E₀|` at f64 from the words, the per-operand scale from the stored `K₀`, `V₀`, each
/// `max` at its edge, and the conditioned factor the preset's ratio times `FACTOR`; `k_0` is the stored `K₀`, 1 in the
/// honest run, where `max(|K₀|, 1)` is 1 from either side.
fn check_measure(k_0: f32) {
    let one = Sample::from_words(words(1.5, 1.0), [k_0, -1.0], true);
    assert_eq!(one.diff, 0.5);
    assert_eq!(one.scale, 2.0 * EPS, "the scale at |K₀| = |V₀| = 1");
    assert_eq!(one.per_operand_factor(), 0.25 / EPS);
    assert_eq!(
        one.factor(),
        0.5 * f64::from(FACTOR),
        "the conditioned factor is not the preset's ratio times FACTOR"
    );
    let below = Sample::from_words(
        words(1.0, 1.0),
        [1.0f32.next_down(), (-1.0f32).next_up()],
        true,
    );
    assert_eq!(below.scale, 2.0 * EPS, "max(|x|, 1) under 1 is not 1");
    assert_eq!((below.diff, below.per_operand_factor()), (0.0, 0.0));
    let above = Sample::from_words(words(-3.0, -2.0), [1.0f32.next_up(), -4.0], true);
    assert_eq!(
        above.scale,
        EPS * (f64::from(1.0f32.next_up()) + 4.0),
        "max(|x|, 1) over 1 is not |x|"
    );
    assert_eq!(above.diff, 1.0, "|E − E₀| is not the absolute difference");
    let nan = Sample::from_words(words(f32::NAN, 1.0), [0.0, 0.0], true);
    assert_eq!(
        nan.per_operand_factor(),
        f64::INFINITY,
        "a NaN difference is not +∞"
    );
    assert!(nan.diff.is_nan());
    let nan_ratio = Sample::from_words([f32::NAN.to_bits(), 1, 0, 0], [0.0, 0.0], true);
    assert_eq!(nan_ratio.factor(), f64::INFINITY, "a NaN ratio is not +∞");
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

/// The tally's measurement: the worst factor of each scale and the worst difference over the compared samples only,
/// NaN read as `+∞`; `count_hatched` also feeds a hatched sample's in, the control.
fn check_tally_measure(count_hatched: bool) {
    let f = f64::from(FACTOR);
    let m = |ratio: f32, diff: f64, scale: f64, valid: bool| Sample {
        ratio,
        valid,
        finite: valid,
        diff,
        scale,
    };
    let mut samples = vec![
        m(0.25, 3.0, 1.0, true),
        m(0.5, 1.0, 0.125, true),
        m(0.125, 0.5, 2.0, true),
    ];
    let hatched = m(0.75, 100.0, 1.0, false);
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
        (t.worst_factor, t.worst_per_operand, t.worst_diff),
        (0.5 * f, 8.0, 3.0),
        "the measurement is not the compared samples' worst"
    );
    assert_eq!(
        (t.median_factor, t.p99_factor),
        (0.25 * f, 0.5 * f),
        "the distribution is not the compared samples'"
    );
    let nan = tally(&[m(f32::NAN, f64::NAN, 1.0, true), m(0.125, 1.0, 1.0, true)]);
    assert_eq!(
        (nan.worst_factor, nan.worst_per_operand, nan.worst_diff),
        (f64::INFINITY, f64::INFINITY, f64::INFINITY)
    );
    assert_eq!(
        (nan.median_factor, nan.p99_factor),
        (0.125 * f, f64::INFINITY)
    );
    let none = tally(&[m(0.5, 5.0, 1.0, false)]);
    assert_eq!(
        (none.worst_factor, none.worst_per_operand, none.worst_diff),
        (0.0, 0.0, 0.0)
    );
    assert_eq!((none.median_factor, none.p99_factor), (0.0, 0.0));
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
    let m = |factor: f64, valid: bool| Sample {
        ratio: (factor / f64::from(FACTOR)) as f32,
        valid,
        finite: valid,
        diff: factor,
        scale: 0.5,
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
            && latent.contains("measured worst |E − E₀| 3.000 eps of the conditioned scale")
            && latent.contains("3.000e0 absolute")
            && latent.contains("the per-operand scale's worst 6.000 eps"),
        "{latent}"
    );
    let last = r.scatter.last().unwrap();
    assert!(
        last.starts_with("every run: measured worst |E − E₀| 7.250 eps"),
        "the summary is not the worst over every chart: {last}"
    );
    assert!(
        last.contains(&format!(
            "the tolerance's {} eps (REQ-COL-060)",
            f64::from(FACTOR)
        )),
        "{last}"
    );
    assert!(
        !r.negative_results.iter().any(|l| l.contains("per-operand")),
        "a conditioned input is reported as judged by the per-operand scale"
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

/// The nearest-rank quantile at its edges: the `⌈q·n⌉`-th value, where `q·n` is whole and where it is just past a
/// whole number; `shift` moves the expected rank, 0 in the honest run.
fn check_quantile(shift: usize) {
    let v: Vec<f64> = (1..=200).map(f64::from).collect();
    let at = |n: usize, q: f64| quantile(&v[..n], q);
    let want = |rank: usize| v[rank - 1 + shift];
    assert_eq!(
        at(100, 0.99),
        want(99),
        "the 99th percentile of 100 is not the 99th value"
    );
    assert_eq!(
        at(101, 0.99),
        want(100),
        "q·n just under a whole number does not round up"
    );
    assert_eq!(at(4, 0.5), want(2));
    assert_eq!(at(3, 0.5), want(2));
    assert_eq!(at(1, 0.99), 1.0);
    assert_eq!(at(1, 0.5), 1.0);
    assert_eq!(at(5, 1.0), 5.0);
    assert_eq!(at(5, 1e-9), 1.0, "a tiny q is not the first value");
    assert_eq!(quantile(&[], 0.5), 0.0);
}

#[test]
fn decode_agreement_quantile() {
    check_quantile(0);
}

negative_control!(
    decode_agreement_quantile,
    "a quantile one rank off must be caught",
    expected = "the 99th percentile of 100 is not the 99th value",
    check_quantile(1)
);

/// The two forms at the boundary: the conditioned form judges by the preset's ratio, the per-operand form by
/// `per_operand_factor / FACTOR`, a factor of exactly `FACTOR` agreeing and the next one up not; `form` is the form the
/// per-operand samples are judged by (the control judges them by the preset's ratio, which agrees).
fn check_forms(form: Form) {
    let f = f64::from(FACTOR);
    // diff / scale = FACTOR exactly, and the next f64 above it; each with a preset ratio well within 1.
    let at = Sample {
        ratio: 0.25,
        valid: true,
        finite: true,
        diff: f,
        scale: 1.0,
    };
    let past = Sample {
        diff: f.next_up(),
        ..at
    };
    assert_eq!(at.ratio_in(Form::PerOperand), 1.0);
    assert_eq!(at.ratio_in(Form::Conditioned), 0.25);
    let t = Tally::judged(DecodeChart::ShapeSphere, &[at, past], form);
    assert_eq!(
        t.disagree, 1,
        "the per-operand form does not fail a factor past FACTOR, or fails one of exactly FACTOR"
    );
    assert_eq!(t.worst, f.next_up() / f);
    assert!(!t.pass());
    assert!(Tally::judged(DecodeChart::ShapeSphere, &[at], Form::PerOperand).pass());
    let conditioned = Tally::judged(DecodeChart::ShapeSphere, &[at, past], Form::Conditioned);
    assert_eq!((conditioned.disagree, conditioned.worst), (0, 0.25));
    let nan = Sample {
        diff: f64::NAN,
        ..at
    };
    assert_eq!(nan.ratio_in(Form::PerOperand), f64::INFINITY);
    let nan_ratio = Sample {
        ratio: f32::NAN,
        ..at
    };
    assert_eq!(nan_ratio.ratio_in(Form::Conditioned), f64::INFINITY);
    assert_eq!(Form::default(), Form::Conditioned);
}

#[test]
fn decode_agreement_forms() {
    check_forms(Form::PerOperand);
}

negative_control!(
    decode_agreement_forms,
    "per-operand samples judged by the preset's ratio must not fail",
    expected = "the per-operand form does not fail a factor past FACTOR",
    check_forms(Form::Conditioned)
);

/// The windows' runs: each window at each depth, its block's first cell `⌊c·2^d⌋ − side/2` on each axis, refused where
/// the block runs off the chart, at each edge; `half` is the half-side the expected first cell is formed with (the
/// control gives another).
fn check_windows(half: u64) {
    let c = config(&format!("{:?}", f64::from(FACTOR)));
    let w = windowed(
        [4, 2],
        vec![
            window("shape_sphere", [0.5, 0.25], vec![3, 5]),
            window("latent", [0.75, 0.5], vec![4]),
        ],
    );
    check(&c, &w).expect("the windows are refused");
    let r = runs(&w).unwrap();
    assert_eq!(r.len(), 3);
    assert_eq!(
        r[0],
        Run {
            chart: DecodeChart::ShapeSphere,
            depth: 3,
            window: Some((0, [0.5, 0.25], [4 - half, 2 - 1])),
        },
        "the block's first cell is not ⌊c·2^d⌋ − side/2"
    );
    assert_eq!(r[1].window, Some((0, [0.5, 0.25], [16 - half, 8 - 1])));
    assert_eq!(r[1].depth, 5);
    assert_eq!(
        (r[2].chart, r[2].depth, r[2].window),
        (
            DecodeChart::Latent,
            4,
            Some((1, [0.75, 0.5], [12 - half, 7]))
        )
    );
    assert_eq!(r[0].label(), "shape_sphere at (0.5, 0.25) depth 3");
    // The edges: the block from cell 0, and to the last cell, are drawn; one cell further either way is refused.
    let ok = |centre: [f64; 2], depth: u32| {
        runs(&windowed(
            [4, 4],
            vec![window("latent", centre, vec![depth])],
        ))
    };
    let first = ok([2.0 / 16.0, 0.5], 4).expect("a block from cell 0 is refused");
    assert_eq!(first[0].window.unwrap().2, [0, 6]);
    let last = ok([14.0 / 16.0, 0.5], 4).expect("a block to the last cell is refused");
    assert_eq!(last[0].window.unwrap().2, [12, 6]);
    for (centre, why) in [
        ([1.9 / 16.0, 0.5], "a block from cell −1"),
        ([15.0 / 16.0, 0.5], "a block past the last cell"),
        ([0.5, 1.9 / 16.0], "a block from cell −1 on y"),
        ([0.5, 15.0 / 16.0], "a block past the last cell on y"),
    ] {
        let e = ok(centre, 4).expect_err(why);
        assert!(e.contains("runs off the chart"), "{why}: {e}");
    }
    ok([0.5, 0.5], 30).expect("depth 30 is refused");
    let refused = |i: Input, why: &str| {
        let e = check(&c, &i).expect_err(why);
        assert!(e.starts_with("refused: "), "{e}");
    };
    refused(
        windowed([4, 4], vec![window("latent", [0.5, 0.5], vec![31])]),
        "depth 31",
    );
    refused(
        windowed([4, 4], vec![window("latent", [0.5, 0.5], vec![])]),
        "no depth",
    );
    refused(
        windowed([4, 4], vec![window("torus", [0.5, 0.5], vec![4])]),
        "no such chart",
    );
    refused(
        windowed([4, 4], vec![window("latent", [-1e-9, 0.5], vec![4])]),
        "a centre under 0",
    );
    refused(
        windowed([4, 4], vec![window("latent", [0.5, 1.0 + 1e-9], vec![4])]),
        "a centre past 1",
    );
    ok([0.0, 1.0], 4).expect_err("a centre at 0 runs off");
    refused(
        Input {
            depth: Some(4),
            ..windowed([4, 4], vec![window("latent", [0.5, 0.5], vec![4])])
        },
        "both a depth and windows",
    );
    refused(windowed([4, 4], vec![]), "neither a depth nor windows");
    refused(
        Input {
            shift: 16,
            ..windowed([4, 4], vec![window("latent", [0.5, 0.5], vec![4])])
        },
        "a shift of every window sample",
    );
    for chart in DecodeChart::ALL {
        assert_eq!(chart_named(chart.section()), Some(chart));
    }
    assert_eq!(chart_named("Latent"), None);
}

#[test]
fn decode_agreement_windows() {
    check_windows(2);
}

negative_control!(
    decode_agreement_windows,
    "a block whose first cell is off by one must be found",
    expected = "the block's first cell is not",
    check_windows(1)
);

/// A window run's scene: `side` adjacent cells from the block's first, row by row, at the run's depth; `step` is the
/// cell spacing expected (the control gives 2).
fn check_window_scene(step: f32) {
    let run = Run {
        chart: DecodeChart::Latent,
        depth: 6,
        window: Some((0, [0.5, 0.5], [30, 31])),
    };
    let scene = run.scene([3, 2]).unwrap();
    let cell = 1.0 / 64.0;
    for (q, (i, j)) in [
        (0u32, (0, 0)),
        (1, (1, 0)),
        (2, (2, 0)),
        (3, (0, 1)),
        (5, (2, 1)),
    ] {
        let want = [
            (30.0 + step * i as f32 + 0.5) * cell,
            (31.0 + step * j as f32 + 0.5) * cell,
        ];
        assert_eq!(
            scene.point(q),
            want,
            "sample {q} is not at its block's cell"
        );
    }
    let lattice = Run {
        chart: DecodeChart::Latent,
        depth: 2,
        window: None,
    };
    assert_eq!(lattice.label(), "latent");
    assert_eq!(lattice.scene([2, 2]).unwrap().point(0), [0.375, 0.375]);
}

#[test]
fn decode_agreement_window_scene() {
    check_window_scene(1.0);
}

negative_control!(
    decode_agreement_window_scene,
    "cells spaced two apart must be found",
    expected = "is not at its block's cell",
    check_window_scene(2.0)
);

/// The report of a windows input: a run line per window and depth, a line per window giving its worst factors by depth
/// in order, and the per-operand form's negative line; `windows` is how many windows the line count expects.
fn check_window_report(windows: usize) {
    let c = config(&format!("{:?}", f64::from(FACTOR)));
    let mut input = windowed(
        [2, 2],
        vec![
            window("shape_sphere", [0.5, 0.5], vec![3, 5]),
            window("latent", [0.25, 0.5], vec![4]),
        ],
    );
    input.form = Form::PerOperand;
    let f = f64::from(FACTOR);
    let s = |factor: f64, per_operand: f64| Sample {
        ratio: (factor / f) as f32,
        valid: true,
        finite: true,
        diff: per_operand,
        scale: 1.0,
    };
    let rs = runs(&input).unwrap();
    let tallies: Vec<Tally> = rs
        .iter()
        .zip([(2.0, 40.0), (3.0, 900.0), (1.0, 2.0)])
        .map(|(run, (a, b))| Tally::of_run(run, &[s(a, b)], Form::PerOperand))
        .collect();
    assert_eq!(tallies[1].window, Some((0, 5)));
    assert_eq!(tallies[1].label, "shape_sphere at (0.5, 0.5) depth 5");
    let r = DecodeAgreementGate::report(
        &c,
        "collision_per_operand.json",
        &input,
        "adapter",
        &tallies,
    );
    let lines: Vec<&String> = r
        .scatter
        .iter()
        .filter(|l| l.starts_with("window "))
        .collect();
    assert_eq!(
        lines.len(),
        windows,
        "not one refinement line per window: {lines:?}"
    );
    assert!(
        lines[0].starts_with("window 0, shape_sphere at (0.5, 0.5)")
            && lines[0].ends_with("by depth: 3: 2.000 (40.000), 5: 3.000 (900.000)"),
        "{}",
        lines[0]
    );
    assert!(
        lines[1].ends_with("by depth: 4: 1.000 (2.000)"),
        "{}",
        lines[1]
    );
    assert!(r
        .scatter
        .iter()
        .any(|l| l.starts_with("shape_sphere at (0.5, 0.5) depth 3: 1 samples")));
    assert!(r
        .negative_results
        .iter()
        .any(|l| l.contains("judged by the per-operand scale alone")));
    assert_eq!(
        r.verdict,
        validation::gate::report::Verdict::Fail,
        "900 eps of the per-operand scale passed"
    );
}

#[test]
fn decode_agreement_window_report() {
    check_window_report(2);
}

negative_control!(
    decode_agreement_window_report,
    "a refinement line missing for a window must be found",
    expected = "not one refinement line per window",
    check_window_report(1)
);

/// The f64 store: every sample's payload `K₀` and `V₀` the f64 `decodeOnly()`'s at its own point, rounded to f32, and
/// nothing else of the descriptor touched; `skip` leaves that many samples as they were (the control).
fn check_store_exact(skip: u32) {
    let run = Run {
        chart: DecodeChart::ShapeSphere,
        depth: 17,
        window: Some((0, [0.844518, 0.5], [110_689, 65_534])),
    };
    let mut scene = run.scene([3, 4]).unwrap();
    let before: Vec<_> = (0..12u32).map(|q| *scene.set.ic(q)).collect();
    let mut exact_scene = run.scene([3, 4]).unwrap();
    store_exact(&mut exact_scene).unwrap();
    for q in skip..12 {
        *scene.set.ic(q) = *exact_scene.set.ic(q);
    }
    for q in 0..12u32 {
        let want = validation::decode_preset::reference(
            DecodeChart::ShapeSphere,
            &scene.uniforms,
            scene.point(q),
        )
        .unwrap()
        .out
        .descriptor;
        let got = *scene.set.ic(q);
        assert_eq!(
            (got.K_0.to_bits(), got.V_0.to_bits()),
            ((want.K_0 as f32).to_bits(), (want.V_0 as f32).to_bits()),
            "sample {q}'s stored K₀, V₀ are not the f64 decode's, rounded"
        );
        assert_eq!(
            (got.m0, got.rho_mag, got.r_min_pair_0),
            (
                before[q as usize].m0,
                before[q as usize].rho_mag,
                before[q as usize].r_min_pair_0
            ),
            "sample {q}: a field other than K₀ and V₀ changed"
        );
    }
}

#[test]
fn decode_agreement_store_exact() {
    check_store_exact(0);
}

negative_control!(
    decode_agreement_store_exact,
    "a sample left with its old K₀ and V₀ must be found",
    expected = "are not the f64 decode's, rounded",
    check_store_exact(1)
);

/// The inputs' fields as the JSON gives them: `form` and `stored` by name, each defaulting, an unknown one refused; and
/// the report's line for an f64 store; `f64_name` is the name `stored`'s f64 is read under (the control gives another).
fn check_fields(f64_name: &str) {
    let parse = |extra: &str| {
        serde_json::from_str::<Input>(&format!(
            r#"{{"description": "d", "side": [2, 2], "depth": 1{extra}}}"#
        ))
    };
    let plain = parse("").unwrap();
    assert_eq!(
        (plain.form, plain.stored),
        (Form::Conditioned, Stored::Stage)
    );
    let both = parse(&format!(
        r#", "form": "per_operand", "stored": "{f64_name}""#
    ))
    .expect("the f64 store is not read by its name");
    assert_eq!((both.form, both.stored), (Form::PerOperand, Stored::F64));
    assert!(parse(r#", "stored": "f32""#).is_err());
    assert!(parse(r#", "form": "loose""#).is_err());
    let c = config(&format!("{:?}", f64::from(FACTOR)));
    let r = DecodeAgreementGate::report(&c, "f.json", &both, "adapter", &[]);
    assert!(r
        .negative_results
        .iter()
        .any(|l| l.contains("the f64 decodeOnly()'s at each sample's point")));
    let r = DecodeAgreementGate::report(&c, "p.json", &plain, "adapter", &[]);
    assert!(!r.negative_results.iter().any(|l| l.contains("f64")));
}

#[test]
fn decode_agreement_fields() {
    check_fields("f64");
}

negative_control!(
    decode_agreement_fields,
    "a store read under another name must be refused",
    expected = "the f64 store is not read by its name",
    check_fields("F64")
);

/// The hatch's operands ([`operands_finite`]): the payload's `K₀` and `V₀` and every word of the fragment decode's `K`,
/// `V`, `m` and `r`. Each made `+∞`, `−∞` or NaN in turn reads non-finite; each at a finite edge (the largest finite
/// value either sign, the least normal, the least subnormal, `−0`) reads finite. `poison` is the value written as the
/// non-finite one (the control writes a finite one).
fn check_operands(poison: [f32; 3]) {
    let stored = [0.25f32, -0.5];
    let fragment: [u32; 12] = [
        0.25f32, -0.5, 0.5, 0.3, 0.2, 0.9, 0.1, -0.4, 0.7, -0.5, -0.8, 0.0,
    ]
    .map(f32::to_bits);
    assert!(
        operands_finite(stored, &fragment),
        "finite operands read non-finite"
    );
    for bad in poison {
        for k in 0..2 {
            let mut s = stored;
            s[k] = bad;
            assert!(
                !operands_finite(s, &fragment),
                "a stored operand {k} of {bad} counted finite"
            );
        }
        for k in 0..12 {
            let mut f = fragment;
            f[k] = bad.to_bits();
            assert!(
                !operands_finite(stored, &f),
                "the fragment's word {k} of {bad} counted finite"
            );
        }
    }
    for edge in [
        f32::MAX,
        -f32::MAX,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
        -0.0,
    ] {
        for k in 0..2 {
            let mut s = stored;
            s[k] = edge;
            assert!(
                operands_finite(s, &fragment),
                "a stored {edge} read non-finite"
            );
        }
        for k in 0..12 {
            let mut f = fragment;
            f[k] = edge.to_bits();
            assert!(
                operands_finite(stored, &f),
                "the fragment's word {k} of {edge} read non-finite"
            );
        }
    }
}

#[test]
fn decode_agreement_operands_finite() {
    check_operands([f32::INFINITY, f32::NEG_INFINITY, f32::NAN]);
}

negative_control!(
    decode_agreement_operands_finite,
    "a non-finite operand counted finite must be caught",
    expected = "counted finite",
    check_operands([f32::MAX, f32::INFINITY, f32::NAN])
);
