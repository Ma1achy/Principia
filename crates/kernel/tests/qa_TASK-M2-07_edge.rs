//! QA tests for TASK-M2-07, round 4, written from the requirement, not from the implementation: `C`
//! (`kernel::canon::canonicalise`) takes no mirror and reads no `δ_λ` (REQ-DEC-017; dd_decoder §3.3, dd_encode §3.2),
//! on hierarchical states at the precision's whole exponent range, either side of the old deadband's edge, at f64
//! and f32. *Was (before TASK-M2-57 removed the decoder's mirror step):* "R-82's one mirror test holds … near the
//! deadband's edge".
//!
//! - `qa_m207_edge_range`: states whose inner pair is `2^j` of the outer distance (`j` down to −40 at f32, −500 at
//!   f64), built with `λ̃_y = −k·δ_λ` exactly in f64 and rounded to the precision, `k` either side of 1 by `10⁻³`, and
//!   at `0.5` and 2; each turned by the exact quarter turns, reflected (`y → −y`, `λ̃_y = +k·δ_λ`) and scaled by
//!   exact powers of two up to the precision's range, wherever every coordinate and pair difference stays 0 or normal
//!   (the domain `C`'s branch inputs are claimed for). `C` keeps `λ̃_y`'s sign at every `k`: its output is in the
//!   lower half, `λ̃_y < 0`, on the unreflected state and in the upper on the reflected one, whether `k` is inside or
//!   outside the old deadband (*was:* "`C` mirrors iff `λ̃_y < −δ_λ`: for `k > 1` on the unreflected state").
//!
//! The expected value is `k`'s, from the construction; the state's own `λ̃_y`, recomputed at f64 from its rounded
//! values at scale 1, is checked to land within `10⁻⁵·k` of `−k·δ_λ` (the rounding of the inputs, about `ε` relative),
//! so the margin of `10⁻³` is clear of it. Quarter turns, `y → −y` and power-of-two scalings are exact in floating
//! point, so every variant has the base state's `λ̃_y` (or its negative) exactly.
//!
//! **Domain** (pitfalls §10): `C` alone, on states held fixed; no trajectory.
//!
//! Its negative controls (R-176): `C` with the mirror step left in, which changes the sign for `k > 1`; and the half
//! read in plain floats off the input, the sign of `ρ × (M₀₁r₂ − m₀r₀ − m₁r₁)`, which underflows on the scaled states
//! at f32.

use kernel::canon::{canonicalise, Canonical};
use kernel::decode::Ic;
use kernel::generated::links::LinkReal;
use kernel::payload::PayloadReal;
use validation::decode_f32_eps::SplitMix;
use validation::negative_control;

/// δ_λ, the old mirror deadband's half-width (dd_decoder §3.3): no longer read by `C`; the scale the states are built
/// at, and the test of the control's mirror step.
const DELTA: f64 = 1e-12;

/// A precision the shared source runs at.
trait P: LinkReal + PayloadReal + Into<f64> + std::fmt::Debug {
    /// The inner pair's size over the outer distance, as powers of two.
    const RATIOS: &'static [i32];
    /// The powers of two the states are scaled by.
    const SCALES: &'static [i32];
}
impl P for f32 {
    const RATIOS: &'static [i32] = &[0, -10, -20, -30, -40];
    const SCALES: &'static [i32] = &[-110, -90, -70, -50, -30, -10, 0, 10, 30, 50, 70, 90, 110];
}
impl P for f64 {
    const RATIOS: &'static [i32] = &[0, -50, -200, -500];
    const SCALES: &'static [i32] = &[
        -1000, -900, -700, -500, -300, -100, 0, 100, 300, 500, 700, 900, 1000,
    ];
}

fn cast<R: P>(v: f64) -> R {
    <R as spirv_std::num_traits::NumCast>::from(v).expect("finite")
}

/// The `k` of `λ̃_y = −k·δ_λ`: inside the old deadband and outside it (where the removed step mirrored).
const KS: [f64; 4] = [0.5, 1.0 - 1e-3, 1.0 + 1e-3, 2.0];

/// How far the rounded state's own `λ̃_y/δ_λ` may sit from `−k`, relative: the inputs' rounding, about `ε_f32`, with
/// room; well inside the `10⁻³` margin of [`KS`].
const LANDING: f64 = 1e-5;

/// `λ̃_y/δ_λ` of `x` at f64, `x` with its inner pair on the x axis (`r₀ = 0`, `r₁ = (a, 0)`), so `λ_y = y₂` exactly:
/// `λ̃_y = √(m₂M₀₁)·y₂/√I_in`, `I_in` the CoM-frame moment of inertia (dd_encode §3.2 steps 1a and 0).
fn lambda_ratio(x: &Ic<f64>) -> f64 {
    let m = x.m;
    let com = [0, 1].map(|k| (0..3).map(|i| m[i] * x.r[i][k]).sum::<f64>());
    let i_in: f64 = (0..3)
        .map(|i| m[i] * ((x.r[i][0] - com[0]).powi(2) + (x.r[i][1] - com[1]).powi(2)))
        .sum();
    (m[2] * (m[0] + m[1])).sqrt() * x.r[2][1] / i_in.sqrt() / DELTA
}

/// A base state at `R`: random masses, the inner pair `2^j·(1…2)` long on `+x` from the origin, the third body at
/// `x₂ ∈ [0.5, 1.5]` and `y₂` set so that `λ̃_y = −k·δ_λ`, momenta random.
fn base<R: P>(rng: &mut SplitMix, j: i32, k: f64) -> Ic<R> {
    let mut u = |lo: f32, hi: f32| f64::from(rng.uniform(lo, hi));
    let w = [u(0.2, 1.2), u(0.2, 1.2), u(0.2, 1.2)];
    let total: f64 = w.iter().sum();
    let m = w.map(|v| cast::<R>(v / total).into());
    let a: f64 = cast::<R>(2f64.powi(j) * u(1.0, 2.0)).into();
    let x2: f64 = cast::<R>(u(0.5, 1.5)).into();
    let m01 = m[0] + m[1];
    let (mu_rho, mu_lambda) = (m[0] * m[1] / m01, m[2] * m01);
    let lx = x2 - m[1] * a / m01;
    let i = mu_rho * a * a + mu_lambda * lx * lx;
    let y2 = -k * DELTA * (i / mu_lambda).sqrt();
    let p = [0; 3].map(|_| [u(-0.5, 0.5), u(-0.5, 0.5)]);
    let x = Ic {
        m,
        r: [[0.0, 0.0], [a, 0.0], [x2, y2]],
        p,
    };
    let x = Ic {
        m: x.m.map(cast::<R>),
        r: x.r.map(|v| v.map(cast::<R>)),
        p: x.p.map(|v| v.map(cast::<R>)),
    };
    let back = Ic {
        m: x.m.map(Into::into),
        r: x.r.map(|v| v.map(Into::into)),
        p: x.p.map(|v| v.map(Into::into)),
    };
    let landed = lambda_ratio(&back);
    assert!(
        (landed + k).abs() <= LANDING * k,
        "{}: the construction missed λ̃_y = −{k}·δ_λ: {landed}·δ_λ at 2^{j}",
        R::NAME
    );
    x
}

/// `x` turned by `q` quarter turns, reflected where `flip`, and scaled by `2^e`: each exact where no value leaves the
/// normal range, which the caller checks.
fn variant<R: P>(x: &Ic<R>, q: usize, flip: bool, e: i32) -> Ic<R> {
    let s = cast::<R>(2f64.powi(e));
    let f = |v: [R; 2]| {
        let v = if flip { [v[0], -v[1]] } else { v };
        let v = (0..q).fold(v, |w, _| [-w[1], w[0]]);
        [v[0] * s, v[1] * s]
    };
    Ic {
        m: x.m,
        r: x.r.map(f),
        p: x.p,
    }
}

/// Every coordinate and pair difference of `x`, the scaling of `unscaled` by a power of two, is a finite normal, or 0
/// where `unscaled`'s is 0: the scaling was exact, and `x` is in the decision's claimed domain.
fn exact<R: P>(x: &Ic<R>, unscaled: &Ic<R>) -> bool {
    let ok = |q: R, u: R| {
        (q == R::zero() && u == R::zero()) || (q.is_finite() && q.abs() >= R::min_positive_value())
    };
    let values = |r: [[R; 2]; 3]| {
        let mut out: Vec<R> = r.iter().flatten().copied().collect();
        for (i, j) in [(0, 1), (0, 2), (1, 2)] {
            out.extend([r[j][0] - r[i][0], r[j][1] - r[i][1]]);
        }
        out
    };
    values(x.r)
        .into_iter()
        .zip(values(unscaled.r))
        .all(|(q, u)| ok(q, u))
}

/// Whether a state's canonical form is in the lower half, `λ̃_y < 0`.
type Decide<R> = fn(&Ic<R>) -> bool;

/// The half of `C`'s output: `λ̃_y < 0`, read off the output state at f64 (`√μ_λ·(y₂ − (m₀y₀ + m₁y₁)/M₀₁)`), which
/// `C`'s own record, [`Canonical::lambda_t_y`], must agree with.
fn lower_half<R: P>(c: &Canonical<R>) -> bool {
    let m: [f64; 3] = c.ic.m.map(Into::into);
    let y: [f64; 3] = c.ic.r.map(|v| v[1].into());
    let m01 = m[0] + m[1];
    let lt = (m[2] * m01).sqrt() * (y[2] - (m[0] * y[0] + m[1] * y[1]) / m01);
    let recorded: f64 = c.lambda_t_y.into();
    assert_eq!(
        lt < 0.0,
        recorded < 0.0,
        "{}: C's recorded λ̃_y = {recorded:e} is not in its output's half, λ̃_y = {lt:e}",
        R::NAME
    );
    lt < 0.0
}

/// `decide` on every variant of 24 base states per ratio and `k`: the lower half exactly on the unreflected states,
/// at every `k`; the numbers checked and found in the lower half.
fn check_edge_range<R: P>(decide: Decide<R>) -> (usize, usize) {
    let mut rng = SplitMix::new(2207);
    let (mut n, mut lower) = (0, 0);
    for &j in R::RATIOS {
        for k in KS {
            for _ in 0..24 {
                let x = base::<R>(&mut rng, j, k);
                for &e in R::SCALES {
                    for q in 0..4 {
                        for flip in [false, true] {
                            let v = variant(&x, q, flip, e);
                            if !exact(&v, &variant(&x, q, flip, 0)) {
                                continue;
                            }
                            let want = !flip;
                            let got = decide(&v);
                            assert_eq!(
                                got,
                                want,
                                "{}: λ̃_y = {}{k}·δ_λ, inner pair 2^{j} of the outer distance, scaled by 2^{e}, {q} \
                                 quarter turns: the lower half must be {want} (C keeps λ̃_y's sign); state {v:?}",
                                R::NAME,
                                if flip { "+" } else { "−" }
                            );
                            n += 1;
                            lower += usize::from(got);
                        }
                    }
                }
            }
        }
    }
    (n, lower)
}

#[test]
fn qa_m207_edge_range() {
    let c = |x: &Ic<f32>| lower_half(&canonicalise(x));
    let (n32, m32) = check_edge_range::<f32>(c);
    let c = |x: &Ic<f64>| lower_half(&canonicalise(x));
    let (n64, m64) = check_edge_range::<f64>(c);
    // Every ratio meets scales on both sides of 1 where it stays normal, so each precision checks thousands.
    assert!(
        n32 > 5000 && n64 > 5000,
        "too few states in range: {n32} (f32), {n64} (f64)"
    );
    // The reflected and unreflected variants are in range together, so half of the states are in the lower half.
    assert!(
        2 * m32 == n32 && 2 * m64 == n64,
        "the lower half is not half the states: {m32} of {n32} (f32), {m64} of {n64} (f64)"
    );
    eprintln!(
        "qa_m207_edge_range: {n32} f32 states ({m32} lower) and {n64} f64 states ({m64} lower), λ̃_y = ∓k·δ_λ \
         with k = {KS:?}, inner pairs 2^{:?} (f32) and 2^{:?} (f64), scaled 2^{:?} (f32) and 2^{:?} (f64): C keeps \
         λ̃_y's sign at every k",
        <f32 as P>::RATIOS,
        <f64 as P>::RATIOS,
        <f32 as P>::SCALES,
        <f64 as P>::SCALES
    );
}

/// `C` with the removed mirror step left in: the half read after reflecting the output where `λ̃_y < −δ_λ`.
#[cfg(feature = "controls")]
fn mirror_left_in(x: &Ic<f32>) -> bool {
    let mut c = canonicalise(x);
    if f64::from(c.lambda_t_y) < -DELTA {
        c.ic.r = c.ic.r.map(|v| [v[0], -v[1]]);
        c.ic.p = c.ic.p.map(|v| [v[0], -v[1]]);
        c.lambda_t_y = -c.lambda_t_y;
    }
    lower_half(&c)
}

negative_control!(
    qa_m207_edge_range,
    "C with its mirror step left in puts a state past the old deadband in the upper half and must fail",
    // The first state past the edge, unreflected: the message names its k.
    expected = "f32: λ̃_y = −1.001·δ_λ",
    {
        check_edge_range::<f32>(mirror_left_in);
    }
);

/// The half in plain floats, read off the input: `c < 0`, `c = ρ × (M₀₁r₂ − m₀r₀ − m₁r₁)`, whose sign is `λ̃_y`'s.
/// *Was:* R-82's whole test in plain floats, this with `m₂c² > δ_λ²·M₀₁·‖ρ‖²·I_in`.
#[cfg(feature = "controls")]
fn plain(x: &Ic<f32>) -> bool {
    let (m, r) = (x.m, x.r);
    let rho = [r[1][0] - r[0][0], r[1][1] - r[0][1]];
    let m01 = m[0] + m[1];
    let big = |k: usize| m01 * r[2][k] - m[0] * r[0][k] - m[1] * r[1][k];
    let c = rho[0] * big(1) - rho[1] * big(0);
    c < 0.0
}

/// A second control: the plain-float half.
#[cfg(feature = "controls")]
mod edge_plain_floats {
    use super::*;

    negative_control!(
        qa_m207_edge_range,
        "the half read in plain f32 floats underflows on scaled states and must fail",
        expected = "f32: λ̃_y = −",
        {
            check_edge_range::<f32>(plain);
        }
    );
}
