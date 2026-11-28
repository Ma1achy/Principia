//! The FTLE field's chart independence: a pixel's `λ_T` is a function of the state it decodes to, not of the chart.
//!
//! Three charts decode one latent point `Z` at the pixel `(½, ½)`:
//! - A, linear, on the axes `(z_q1, z_μ1)`;
//! - B, the same axes warped by a power of 2, ranges chosen so every warped value is exact at `t = ½`
//!   (`lo + (hi−lo)·¼`, dyadic): the warp rescales the tangent and keeps its direction;
//! - C, linear, on the axes `(z_q2, z_μ1)`: a different tangent through the same state, a tilted pair.
//!
//! Criteria, written before the run:
//! 1. Precondition: the three decodes are one bit pattern, none degenerate; A's and B's `∂z/∂u` are parallel and C's is
//!    not, so the arms below can tell a warp from a turn.
//! 2. `λ_T` is finite and its f32 bits are equal across A, B and C, and differ from the `λ_T` of a nearby state (the
//!    exponent is not constant).
//! 3. Control: with `δ₀` along the chart's tangent (`Seed::ChartTangent`) the three `λ_T` bits are not equal, and the
//!    same equality check fails.

use engine::first_slice::{uniforms, MarchKey, Quad};
use engine::ftle::{lambda_at, Seed};
use kernel::chart::mixed::{Mixed, ZMu1, ZQ1, ZQ2};
use kernel::chart::warp::AxisWarp;
use kernel::chart::Latent;
use kernel::decode::Ic;
use kernel::driver::benettin::BenettinParams;
use kernel::generated::links::Defaults;
use validation::negative_control;

type A = engine::first_slice::HeggieRk4;
const UV: [f64; 2] = [0.5, 0.5];

/// The latent point, every component dyadic: `[z_α, z_β, z_q0, z_q1, z_q2, z_q3, z_μ1, z_μ2]`.
const Z: [f64; 8] = [0.25, 0.5, 0.25, -0.5, 0.75, 0.125, 0.5, -0.25];

fn with(z: [f64; 8], q0: f64) -> [f64; 8] {
    let mut z = z;
    z[2] = q0;
    z
}

fn key() -> engine::contract::sim_config::SimConfig {
    Quad {
        chart: Latent {
            z0: [0.0f32; 8],
            q1: [0.0; 8],
            q2: [0.0; 8],
        },
        window: engine::contract::sim_config::Window::SQUARE,
        size: [1, 1],
    }
    .sim_key()
}

fn march() -> MarchKey {
    uniforms(1.0).expect("a valid horizon")
}

fn params() -> BenettinParams<f32> {
    BenettinParams {
        delta_0: 1e-5,
        n_renorm: 10,
    }
}

/// The three charts at `z`, as closures from the seed to `λ_T`, with their decode.
type Arm = (&'static str, Box<dyn Fn(Seed) -> f32>, Ic<f64>, [f64; 8]);

fn arms(z: [f64; 8]) -> Vec<Arm> {
    let (sim, m) = (key(), march());
    let lin = |h: [f64; 2], v: [f64; 2]| Mixed::<ZQ1, ZMu1, f64>::new(z, h, v);
    let a = lin([-1.5, 0.5], [0.0, 1.0]);
    let b = lin([-1.5, 2.5], [0.0, 2.0])
        .with_warps(AxisWarp::power(2.0).unwrap(), AxisWarp::power(2.0).unwrap())
        .expect("power fits");
    let c = Mixed::<ZQ2, ZMu1, f64>::new(z, [0.25, 1.25], [0.0, 1.0]);
    let mk = |name: &'static str, tangent: [f64; 8], f: Box<dyn Fn(Seed) -> f32>, ic| {
        (name, f, ic, tangent)
    };
    macro_rules! arm {
        ($name:literal, $chart:expr, $tangent:expr) => {{
            let chart = $chart;
            let ic = engine::decode_dispatch::decode_only::<_, Defaults>(&chart, UV, &sim)
                .unwrap()
                .ic;
            let sim = sim.clone();
            mk(
                $name,
                $tangent,
                Box::new(move |seed| {
                    lambda_at::<_, Defaults, A>(&chart, UV, &sim, &m, params(), seed).unwrap()
                }),
                ic,
            )
        }};
    }
    // ∂z/∂u at the pixel: A's `(h_hi − h_lo)` along z_q1, B's `γ·(hi − lo)·t^(γ−1)` along z_q1, C's along z_q2.
    let along = |k: usize, x: f64| {
        let mut t = [0.0; 8];
        t[k] = x;
        t
    };
    vec![
        arm!("linear", a, along(3, 2.0)),
        arm!("power warp", b, along(3, 4.0)),
        arm!("tilted pair", c, along(4, 1.0)),
    ]
}

fn bits(ic: &Ic<f64>) -> Vec<u64> {
    ic.m.iter()
        .chain(ic.r.iter().flatten())
        .chain(ic.p.iter().flatten())
        .map(|x| x.to_bits())
        .collect()
}

/// Criterion 2 under `seed`: finite, and one bit pattern across the arms.
fn check_independent(seed: Seed) {
    let arms = arms(Z);
    let lambdas: Vec<f32> = arms.iter().map(|a| a.1(seed)).collect();
    for ((name, ..), l) in arms.iter().zip(&lambdas) {
        println!("lambda_T, {name}: {l:e} (bits {:#010x})", l.to_bits());
        assert!(l.is_finite(), "{name}: lambda_T is not finite");
    }
    assert!(
        lambdas.iter().all(|l| l.to_bits() == lambdas[0].to_bits()),
        "lambda_T differ across charts: {lambdas:?}"
    );
}

fn check_decode_is_one_state() {
    let arms = arms(Z);
    for (name, _, ic, _) in &arms {
        assert_eq!(bits(ic), bits(&arms[0].2), "{name} decodes another state");
    }
    // A's and B's tangents are parallel, C's is not.
    let cross = |a: &[f64; 8], b: &[f64; 8]| {
        let (na, nb) = (
            a.iter().map(|x| x * x).sum::<f64>(),
            b.iter().map(|x| x * x).sum::<f64>(),
        );
        let ab: f64 = a.iter().zip(b).map(|(x, y)| x * y).sum();
        1.0 - ab * ab / (na * nb)
    };
    assert_eq!(cross(&arms[0].3, &arms[1].3), 0.0);
    assert!(cross(&arms[0].3, &arms[2].3) > 0.5);
}

#[test]
fn ftle_chart_independent() {
    check_decode_is_one_state();
    check_independent(Seed::State);
    check_depends_on_state();
}

/// The exponent is a function of the state: another state gives another one, so equal bits above are not a constant.
fn check_depends_on_state() {
    let here = arms(Z)[0].1(Seed::State);
    let there = arms(with(Z, 0.375))[0].1(Seed::State);
    assert!(
        there.is_finite() && here.to_bits() != there.to_bits(),
        "{here} {there}"
    );
}

negative_control!(
    ftle_chart_independent,
    "delta_0 taken along the chart's tangent",
    expected = "lambda_T differ across charts",
    check_independent(Seed::ChartTangent { step: 1.0 / 64.0 })
);
