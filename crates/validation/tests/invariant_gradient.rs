//! The `invariant-gradient` gate's measure and refusals, with no GPU (`validation::invariant_gradient`; TASK-M2-12,
//! REQ-VAL-015); `invariant_gradient_gpu.rs` runs it on the kernel:
//! - `invariant_gradient_measure`: a field equal to its target measures 0 on all three; a row tilted along `L_z`
//!   measures its spread, a row below the one before it its decrease, and a pixel off its target its distance, each in
//!   f32 `eps` scaled by `max(|E|, 1)`.
//! - `invariant_gradient_perturbed`: the perturbed construction (`a = √(2K*)`) fails the gate on its fixture's frozen
//!   configurations, at the gate's threshold, its row spread `L_max²/2` in its eps, far beyond it.
//! - `invariant_gradient_refusals`: a grid under two points per side, or no frozen configuration, is refused before any
//!   decode, and two points per side are judged; the gate is registered under its name and its `gate.json` reads.

use std::path::{Path, PathBuf};

use validation::gate::{self, Gate, Verdict};
use validation::invariant_gradient::{
    grid, measure, Field, Input, InvariantGradientGate, Measurement, Perturb, Which, NAME,
};
use validation::negative_control;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

const EPS: f64 = f32::EPSILON as f64;

/// A 3 × 3 field at targets `[−0.5, 0.25, 2.0]`, each pixel its row's target rounded to f32.
fn exact() -> Field {
    let target = vec![-0.5, 0.25, 2.0];
    Field {
        n: 3,
        e0: target.iter().flat_map(|&t| [t as f32; 3]).collect(),
        target,
    }
}

/// `measure` under check.
type Measure = fn(&[Field]) -> Measurement;

fn check_measure(measure: Measure) {
    let m = measure(&[exact()]);
    assert_eq!(
        (m.target, m.spread, m.monotone, m.samples),
        (0.0, 0.0, 0.0, 9),
        "an exact field does not measure 0"
    );
    // Row 2 (target 2.0, scale eps·2): one pixel 8 eps above 2.0, four of its ulps, so spread and target both 4.
    let mut tilted = exact();
    tilted.e0[7] = 2.0 + 4.0 * 2.0 * EPS as f32;
    let m = measure(&[tilted]);
    assert!(
        (m.spread - 4.0).abs() < 1e-9 && (m.target - 4.0).abs() < 1e-9 && m.monotone == 0.0,
        "a tilted row measures {m:?}, not a spread of 4 eps"
    );
    // Row 1 (target 0.25, scale 1) dropped below row 0's top by 6 eps: its decrease from row 0.
    let mut falling = exact();
    falling.e0[0] = 0.25 + 6.0 * EPS as f32;
    let m = measure(&[falling]);
    assert!(
        (m.monotone - 6.0).abs() < 1e-9,
        "a row above the next measures a decrease of {}, not 6 eps",
        m.monotone
    );
    let both = measure(&[exact(), exact()]);
    assert_eq!(both.samples, 18, "two fields' pixels");
    assert_eq!(grid(3).len(), 9, "the grid's points");
    assert_eq!(
        grid(3)[3..6],
        [[0.0, 0.5], [0.5, 0.5], [1.0, 0.5]],
        "the grid's second row"
    );
}

#[test]
fn invariant_gradient_measure() {
    check_measure(measure);
}

/// A measure blind to the rows' spread, reading only each row's first pixel.
#[cfg(feature = "controls")]
fn first_pixel(fields: &[Field]) -> Measurement {
    let flat: Vec<Field> = fields
        .iter()
        .map(|f| Field {
            n: f.n,
            e0: f.e0.chunks(f.n).flat_map(|r| vec![r[0]; f.n]).collect(),
            target: f.target.clone(),
        })
        .collect();
    measure(&flat)
}

negative_control!(
    invariant_gradient_measure,
    "a measure reading only each row's first pixel misses a tilted row",
    expected = "not a spread of 4 eps",
    check_measure(first_pixel)
);

/// The fixture's `perturbed.json`.
fn perturbed_input() -> Input {
    let text =
        std::fs::read_to_string(root().join("fixtures/gates/invariant-gradient/perturbed.json"))
            .expect("perturbed.json");
    let mut v: serde_json::Value = serde_json::from_str(&text).expect("JSON");
    v.as_object_mut().expect("an object").remove("expected");
    serde_json::from_value(v).expect("an invariant-gradient input")
}

/// The gate, at `threshold`, judges `input` (no GPU: the perturbed construction decodes natively) and fails it.
fn check_perturbed(input: &Input, threshold: f64) {
    let mut config = gate::config(&root(), NAME).expect("gate.json");
    config.threshold.value = threshold;
    let report = InvariantGradientGate::judge(&config, "perturbed.json", input, "")
        .expect("the perturbed input is judged");
    assert_eq!(
        report.verdict,
        Verdict::Fail,
        "the perturbed construction passes the gate"
    );
    // E₀ = U + K(t) + L_z²/2: the top row spreads by L_max²/2 = K_max less the inner column's share, (1/(n − 1))², in
    // units of eps·max(|E|, 1), every target's |E| below 1.
    let n = f64::from(input.grid);
    let want = (1.0 - 1.0 / ((n - 1.0) * (n - 1.0))) / f64::from(f32::EPSILON);
    let line = report
        .scatter
        .iter()
        .find(|l| l.starts_with("E₀ against U + K(t)"))
        .expect("the measures' line");
    let spread: f64 = line
        .split("row spread along L_z: worst ")
        .nth(1)
        .and_then(|r| r.split(' ').next())
        .and_then(|x| x.parse().ok())
        .expect("the row spread");
    assert!(
        (spread / want - 1.0).abs() < 1e-3,
        "the perturbed construction's row spread is {spread} eps, not L_max²/2's {want}"
    );
}

#[test]
fn invariant_gradient_perturbed() {
    let input = perturbed_input();
    assert_eq!(input.perturb, Some(Perturb::KMinNotSubtracted));
    assert_eq!(input.chart, Which::LzE);
    let config = gate::config(&root(), NAME).expect("gate.json");
    check_perturbed(&input, config.threshold.value);
}

negative_control!(
    invariant_gradient_perturbed,
    "at a threshold of 2³⁰ eps the perturbed construction passes, so the check must fail",
    expected = "the perturbed construction passes the gate",
    check_perturbed(&perturbed_input(), f64::from(1u32 << 30))
);

/// Each refused input names why, before any decode.
fn check_refusals(judge: fn(&Input) -> Result<(), String>) {
    let base = perturbed_input();
    for (input, why) in [
        (
            Input {
                grid: 1,
                ..base.clone()
            },
            "a grid of 1 points",
        ),
        (
            Input {
                grid: 0,
                ..base.clone()
            },
            "a grid of 0 points",
        ),
        (
            Input {
                frozen: Vec::new(),
                ..base.clone()
            },
            "over 0 frozen configurations",
        ),
    ] {
        let err = judge(&input).expect_err("the input is not refused");
        assert!(
            err.starts_with("refused:") && err.contains(why),
            "the refusal: {err}"
        );
    }
}

fn real_judge(input: &Input) -> Result<(), String> {
    let config = gate::config(&root(), NAME).expect("gate.json");
    InvariantGradientGate::judge(&config, "x.json", input, "").map(|_| ())
}

#[test]
fn invariant_gradient_refusals() {
    check_refusals(real_judge);
    let two = Input {
        grid: 2,
        ..perturbed_input()
    };
    assert!(
        real_judge(&two).is_ok(),
        "a grid of 2 points per side is refused"
    );
    assert_eq!(InvariantGradientGate.name(), NAME);
    assert_eq!(NAME, "invariant-gradient");
    assert!(gate::gate(NAME).is_ok(), "the gate is not registered");
    let config = gate::config(&root(), NAME).expect("gate.json");
    assert_eq!(
        config.threshold.value, 256.0,
        "the threshold, REQ-DEC-043's factor"
    );
    assert_eq!(
        config.inputs,
        ["lz_e.json", "lz_k.json", "perturbed.json"],
        "the inputs"
    );
}

/// A judge that measures a one-point grid rather than refusing it.
#[cfg(feature = "controls")]
fn no_grid_check(input: &Input) -> Result<(), String> {
    if input.grid == 1 {
        return Ok(());
    }
    real_judge(input)
}

negative_control!(
    invariant_gradient_refusals,
    "a judge accepting a one-point grid is not refused",
    expected = "the input is not refused",
    check_refusals(no_grid_check)
);
