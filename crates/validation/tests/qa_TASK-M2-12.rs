//! QA tests for TASK-M2-12, written from REQ-VAL-015 (render_contract § "Cross-check views (the seams)";
//! debug_tooling_plan §G) and REQ-CHART-017/044 (chart_reference §2, §2.1), not from the implementation.
//! - `qa_m212_gradient_view`: the invariant-chart gradient view, `E₀ = K₀ + V₀` of the decode stage's f32 output on an
//!   `(L_z, E)` chart at the registered `K_max`, `γ_K` (REQ-CHART-044's proposal), is a perfect axis-aligned gradient:
//!   constant along `L_z` (each row's spread), monotone along `E` (no row's largest above the next row's smallest), and
//!   at each pixel the target `E(t) = U + K(t)` (`U` and `K(t)` at f64), all within the gate's threshold in f32 `eps`
//!   scaled by `max(|E|, 1)`, read from `fixtures/gates/invariant-gradient/gate.json` (not picked here). It runs the
//!   kernel's decode-stage source natively at f32 on the CPU, over frozen configurations of its own and a 41×41 grid
//!   with the square's edges and corners; the GPU half is the gate itself (`cargo xtask gate invariant-gradient`). On
//!   both charts every pixel's `V₀` is the frozen configuration's, bit for bit (§2: the chart fixes geometry and mass),
//!   and the view spans `[U, U + K_max]` from the first row to the last.
//!
//! **Control** (pitfalls §3): a construction asked for `K* + 10⁻³·L_z²/2`, an error a thousandth of §2.2's
//! `a = √(2K*)` slip, gives an `E₀` that leans along `L_z` by at most `10⁻³·K_max`; the view must fail it at the
//! gate's threshold.
//!
//! **Domain** (pitfalls §10): the decode alone, its inputs held fixed; no trajectory.

use std::path::PathBuf;

use kernel::chart::invariant::Invariant;
use kernel::chart::{InvariantLE, InvariantLK};
use kernel::decode::descriptor;
use kernel::generated::links::Defaults;
use kernel::stage::decode::sample;
use validation::negative_control;

/// The gate's threshold in f32 `eps`, as the gate's own configuration states it.
fn threshold() -> f64 {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/gates/invariant-gradient/gate.json");
    let text = std::fs::read_to_string(&root).expect("the gate's gate.json");
    let v: serde_json::Value = serde_json::from_str(&text).expect("gate.json is JSON");
    v["threshold"]["value"]
        .as_f64()
        .expect("a numeric threshold")
}

/// An f32 decode's `E₀` at `(s, t)` on the chart.
type View = fn(&Invariant<f32>, f32, f32) -> f32;

fn view_le(c: &Invariant<f32>, s: f32, t: f32) -> f32 {
    descriptor::energy(&sample::<_, Defaults, f32>(&InvariantLE(*c), s, t).descriptor)
}

fn view_lk(c: &Invariant<f32>, s: f32, t: f32) -> f32 {
    descriptor::energy(&sample::<_, Defaults, f32>(&InvariantLK(*c), s, t).descriptor)
}

/// The frozen configuration's `U` at f64, from the chart's own f32 parameters widened.
fn u_pot(c: &Invariant<f32>) -> f64 {
    let wide = Invariant {
        z_cfg: c.z_cfg.map(f64::from),
        z_mass: c.z_mass.map(f64::from),
        k_max: f64::from(c.k_max),
        gamma_k: f64::from(c.gamma_k),
    };
    sample::<_, Defaults, f64>(&InvariantLE(wide), 0.5, 0.5)
        .descriptor
        .V_0
}

fn frozen() -> Vec<Invariant<f32>> {
    let k_max = validation::K_MAX.number() as f32;
    let gamma_k = validation::GAMMA_K.number() as f32;
    [
        ([0.0, 0.0], [0.0, 0.0]),
        ([0.9, -1.2], [0.4, 0.4]),
        ([-0.7, 0.2], [-1.3, 0.6]),
        ([2.2, 1.6], [1.0, -1.8]),
        ([-1.9, -0.6], [2.0, 2.0]),
    ]
    .into_iter()
    .map(|(z_cfg, z_mass)| Invariant {
        z_cfg,
        z_mass,
        k_max,
        gamma_k,
    })
    .collect()
}

const N: usize = 41;

fn check_gradient(view: View, label: &str) {
    let limit = threshold();
    let eps = f64::from(f32::EPSILON);
    for c in frozen() {
        let u = u_pot(&c);
        let ctx = format!(
            "{label}: z_cfg = {:?}, z_mass = {:?}, U = {u}",
            c.z_cfg, c.z_mass
        );
        let mut previous_hi = f64::NEG_INFINITY;
        for j in 0..N {
            let t = j as f32 / (N - 1) as f32;
            let k = f64::from(c.k_max) * f64::from(t).powf(f64::from(c.gamma_k));
            let target = u + k;
            let scale = eps * target.abs().max(1.0);
            let row: Vec<f64> = (0..N)
                .map(|i| f64::from(view(&c, i as f32 / (N - 1) as f32, t)))
                .collect();
            let lo = row.iter().copied().fold(f64::INFINITY, f64::min);
            let hi = row.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            for (i, e) in row.iter().enumerate() {
                assert!(
                    (e - target).abs() <= limit * scale,
                    "E₀ is not U + K(t): {e} against {target} ({:.1} eps) at (i, j) = ({i}, {j}); {ctx}",
                    (e - target).abs() / scale
                );
            }
            assert!(
                hi - lo <= limit * scale,
                "E₀ is not constant along L_z: row {j} spreads {:.1} eps; {ctx}",
                (hi - lo) / scale
            );
            assert!(
                previous_hi - lo <= limit * scale,
                "E₀ is not monotone along E: row {j}'s smallest is {:.1} eps below the row before's largest; {ctx}",
                (previous_hi - lo) / scale
            );
            previous_hi = hi;
        }
    }
}

#[test]
fn qa_m212_gradient_view() {
    check_gradient(view_le, "(L_z, E)");
    check_gradient(view_lk, "(L_z, K)");
}

/// The construction asked for `K* + 10⁻³·L_z²/2` (the module's note).
#[cfg(feature = "controls")]
fn leaning(c: &Invariant<f32>, s: f32, t: f32) -> f32 {
    let w = c.targets(s, t);
    let mut blocks = kernel::chart::invariant::InvariantBlocks::<Defaults, f32>::new(
        w.lz,
        w.k + 1e-3 * w.lz * w.lz / 2.0,
    );
    let d = kernel::decode::decode_with(
        &mut blocks,
        &kernel::chart::Chart::map(&InvariantLE(*c), s, t).z,
    );
    descriptor::energy(&d.descriptor)
}

negative_control!(
    qa_m212_gradient_view,
    "A construction whose E₀ leans along L_z by 10⁻³·L_z²/2 fails the gradient view",
    expected = "E₀ is not",
    check_gradient(leaning, "leaning")
);

fn check_fixed(view: fn(&Invariant<f32>, f32, f32) -> (f32, f32)) {
    for c in frozen() {
        let (v_centre, _) = view(&c, 0.5, 0.5);
        for j in 0..N {
            for i in 0..N {
                let (s, t) = (i as f32 / (N - 1) as f32, j as f32 / (N - 1) as f32);
                let (v0, _) = view(&c, s, t);
                assert!(
                    v0.to_bits() == v_centre.to_bits(),
                    "V₀ moves with the pixel: {v0} at ({s}, {t}) against {v_centre} at the centre; z = {:?}, {:?}",
                    c.z_cfg,
                    c.z_mass
                );
            }
        }
        let u = u_pot(&c);
        let (_, k0) = view(&c, 0.5, 0.0);
        let (_, k1) = view(&c, 0.5, 1.0);
        let k_max = f64::from(c.k_max);
        assert!(
            f64::from(k0) == 0.0,
            "the bottom row is not the rest start: K₀ = {k0}; U = {u}"
        );
        assert!(
            (f64::from(k1) - k_max).abs() <= threshold() * f64::from(f32::EPSILON) * k_max.max(1.0),
            "the top row is not K_max: K₀ = {k1} against {k_max}; U = {u}"
        );
    }
}

fn v_k_le(c: &Invariant<f32>, s: f32, t: f32) -> (f32, f32) {
    let d = sample::<_, Defaults, f32>(&InvariantLE(*c), s, t).descriptor;
    (d.V_0, d.K_0)
}

fn v_k_lk(c: &Invariant<f32>, s: f32, t: f32) -> (f32, f32) {
    let d = sample::<_, Defaults, f32>(&InvariantLK(*c), s, t).descriptor;
    (d.V_0, d.K_0)
}

#[test]
fn qa_m212_view_fixes_geometry_and_spans() {
    check_fixed(v_k_le);
    check_fixed(v_k_lk);
}

/// A chart whose configuration moves with `s`.
#[cfg(feature = "controls")]
fn moving(c: &Invariant<f32>, s: f32, t: f32) -> (f32, f32) {
    let moved = Invariant {
        z_cfg: [c.z_cfg[0] + s, c.z_cfg[1]],
        ..*c
    };
    v_k_le(&moved, s, t)
}

negative_control!(
    qa_m212_view_fixes_geometry_and_spans,
    "A chart whose configuration moves with the pixel does not fix geometry",
    expected = "V₀ moves with the pixel",
    check_fixed(moving)
);
