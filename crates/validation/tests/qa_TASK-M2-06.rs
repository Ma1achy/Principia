//! QA tests for TASK-M2-06's gate, `decode-f32-eps` (REQ-DEC-043; dd_decoder §5 test 12), written from the
//! requirement, not from the implementation; no GPU (the GPU side is `engine`'s `qa_TASK-M2-06_gpu`).
//! - The measure is f32 eps scaled by `max(|x₆₄|, 1)`: known differences measure as their known multiples, at, below
//!   and above magnitude 1 (`qa_m206_measure_known_answers`).
//! - A `DEGENERATE` tag is a branch decision, compared exactly, never within the factor: a pair whose tags differ is
//!   refused, whatever their values (`qa_m206_measure_tags_exact`); and a pair the factor would pass on every value
//!   but one fails on that one (the measure takes the worst, not a mean).
//! - The gate refuses inputs that measure nothing (`qa_m206_gate_refuses_empty_inputs`), and its checked-in
//!   configuration is REQ-DEC-043's provisional proposal, with the three inputs the acceptance line names
//!   (`qa_m206_gate_config`).
//!
//! **Domain** (pitfalls §10): the decode alone, its inputs fixed; no trajectory.
//!
//! Each test has a registered negative control (R-176).

use std::path::Path;

use kernel::chart::Latent;
use kernel::decode::Degenerate;
use kernel::generated::links::Defaults;
use kernel::stage::decode::{sample, DecodeOut};
use validation::decode_f32_eps::{error, measure, DecodeF32EpsGate, Fuzz, Input, Perturb, Points};
use validation::gate::Status;
use validation::negative_control;

const EPS: f64 = f32::EPSILON as f64;

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

// ── The measure ──────────────────────────────────────────────────────────────────────────────────────────────

/// `(x32, x64, eps)`: the difference in f32 eps, relative above magnitude 1, absolute below.
fn known() -> Vec<(f64, f64, f64)> {
    vec![
        (1.0 + 3.0 * EPS, 1.0, 3.0),
        (0.5 + 2.0 * EPS, 0.5, 2.0),
        (1e-6 + 5.0 * EPS, 1e-6, 5.0),
        (1000.0 * (1.0 + 7.0 * EPS), 1000.0, 7.0),
        (-4.0 * (1.0 + 256.0 * EPS), -4.0, 256.0),
        (0.0, 0.0, 0.0),
    ]
}

fn check_measure(f: fn(f64, f64) -> f64) {
    for (a, b, want) in known() {
        let got = f(a, b);
        assert!(
            (got - want).abs() <= 1e-6 * want.max(1.0),
            "the measure of {a} against {b} is {got} eps, not {want}"
        );
    }
    // A non-finite value agrees only with the same bits: a pole's +∞ stored by both, never +∞ against −∞ or a
    // finite value.
    assert_eq!(f(f64::INFINITY, f64::INFINITY), 0.0);
    assert_eq!(f(f64::NEG_INFINITY, f64::NEG_INFINITY), 0.0);
    assert_eq!(f(f64::INFINITY, f64::NEG_INFINITY), f64::INFINITY);
    assert_eq!(f(f64::INFINITY, 1e30), f64::INFINITY);
    assert_eq!(f(1e30, f64::INFINITY), f64::INFINITY);
    assert_eq!(f(f64::NAN, 1.0), f64::INFINITY);
}

#[test]
fn qa_m206_measure_known_answers() {
    check_measure(error);
}

negative_control!(
    qa_m206_measure_known_answers,
    "a measure that is always absolute must fail at magnitude 1000",
    expected = "eps, not 7",
    check_measure(|a, b| {
        if a.is_finite() && b.is_finite() {
            (a - b).abs() / EPS
        } else {
            error(a, b)
        }
    })
);

/// The f32 and f64 decodes of a few interior points.
fn pairs() -> (Vec<DecodeOut<f32>>, Vec<DecodeOut<f64>>) {
    let chart = Latent {
        z0: [0.2, -0.3, 0.1, 0.4, -0.1, 0.0, 0.3, -0.2],
        q1: [0.25; 8],
        q2: [-0.25; 8],
    };
    let uv = [[0.5f32, 0.5], [0.1, 0.9], [0.8, 0.3]];
    let f32s = uv
        .iter()
        .map(|p| sample::<_, Defaults, f32>(&chart, p[0], p[1]))
        .collect();
    let f64s = uv
        .iter()
        .map(|p| sample::<_, Defaults, f64>(&chart, f64::from(p[0]), f64::from(p[1])))
        .collect();
    (f32s, f64s)
}

/// Tags differing on one pair are refused, naming it; a single value moved past the factor shows as the worst.
fn check_tags(mark: fn(&mut DecodeOut<f32>)) {
    let (mut g, c) = (pairs().0, pairs().1);
    let base = measure(&g, &c).expect("untagged pairs measure");
    assert!(
        base.max().0 < 256.0,
        "the interior pairs are {} eps apart",
        base.max().0
    );
    mark(&mut g[1]);
    let r = measure(&g, &c);
    assert!(
        r.as_ref().is_err_and(|e| e.contains("sample 1")),
        "a pair whose tags differ was measured, not refused: {:?}",
        r.map(|m| m.max())
    );
    // One value of one sample moved by 10³ eps: the worst is that value, at that sample.
    let (mut g, c) = pairs();
    g[2].descriptor.K_0 += (1000.0 * EPS * f64::from(g[2].descriptor.K_0).abs().max(1.0)) as f32;
    let m = measure(&g, &c).expect("measured");
    let (w, k) = m.max();
    assert!(
        w > 900.0 && m.worst[k].at == 2,
        "the moved K_0 is not the worst: {w} at {}",
        m.worst[k].at
    );
}

#[test]
fn qa_m206_measure_tags_exact() {
    check_tags(|o| o.degenerate = Some(Degenerate::M01Tiny));
}

negative_control!(
    qa_m206_measure_tags_exact,
    "a 'mark' that leaves the tag as it was must fail",
    expected = "was measured, not refused",
    check_tags(|_| {})
);

// ── The gate's inputs and configuration ──────────────────────────────────────────────────────────────────────

fn input(points: Points, perturb: Option<Perturb>) -> Input {
    Input {
        description: "qa".to_owned(),
        points,
        perturb,
        recorded: None,
    }
}

/// Each input that measures nothing, or perturbs nothing real, is refused before any GPU work (`wgsl` empty).
fn check_refusals(inputs: Vec<(&str, Input)>) {
    let config = validation::gate::config(root(), "decode-f32-eps").expect("gate.json");
    for (what, i) in inputs {
        let got = DecodeF32EpsGate::judge(&config, "qa.json", &i, "");
        assert!(
            got.as_ref().is_err_and(|e| e.starts_with("refused")),
            "{what} was not refused: {:?}",
            got.map(|r| r.verdict)
        );
    }
}

fn empty_inputs() -> Vec<(&'static str, Input)> {
    let fuzz = |charts, per_chart| {
        Points::Interior(Fuzz {
            seed: 1,
            charts,
            per_chart,
        })
    };
    let p = |component, by| Some(Perturb { component, by });
    vec![
        ("no charts", input(fuzz(0, 8), None)),
        ("no points", input(fuzz(8, 0), None)),
        ("component 8", input(Points::Golden, p(8, 1e-4))),
        ("a zero perturbation", input(Points::Golden, p(0, 0.0))),
        ("a NaN perturbation", input(Points::Golden, p(0, f64::NAN))),
        (
            "an infinite perturbation",
            input(Points::Golden, p(3, f64::INFINITY)),
        ),
    ]
}

#[test]
fn qa_m206_gate_refuses_empty_inputs() {
    check_refusals(empty_inputs());
}

negative_control!(
    qa_m206_gate_refuses_empty_inputs,
    "a valid perturbation is not refused, so the check must fail on it",
    expected = "was not refused",
    check_refusals(vec![(
        "a perturbation of component 7 by 10⁻⁴",
        input(
            Points::Golden,
            Some(Perturb {
                component: 7,
                by: 1e-4
            })
        )
    )])
);

/// `gate.json` proposes `factor` provisionally under REQ-DEC-043, R-71, and runs the golden IC, the interior and the
/// perturbed input.
fn check_config(factor: f64) {
    let c = validation::gate::config(root(), "decode-f32-eps").expect("gate.json");
    assert_eq!(c.threshold.value, factor, "gate.json's factor");
    assert_eq!(c.threshold.requirement, "REQ-DEC-043");
    assert_eq!(
        c.threshold.status,
        Status::Provisional,
        "REQ-DEC-043 is not confirmed until the M2 gate"
    );
    assert_eq!(c.inputs, ["golden.json", "interior.json", "perturbed.json"]);
}

#[test]
fn qa_m206_gate_config() {
    // The proposal the PR makes: 256 f32 eps (task acceptance line; REQ-DEC-043).
    check_config(256.0);
}

negative_control!(
    qa_m206_gate_config,
    "another factor must fail",
    expected = "gate.json's factor",
    check_config(128.0)
);
