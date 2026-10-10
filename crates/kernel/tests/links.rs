//! The generated link functions (`kernel::generated::links`; dd_generation_root §3.9, §5 test 8; REQ-CHART-034) and
//! chart constants (`kernel::generated::constants`; REQ-DEC-009):
//! - `link_properties_cover_every_entry`: the links under test here are every generated entry.
//! - `link_properties_a_constraint`: §5 test 8 (a), fuzzed at f32 and f64 across the domain, saturation included:
//!   simplex outputs positive and summing to 1, interval outputs in range, half-line outputs positive.
//! - `link_properties_b_round_trip`: (b), `f(f⁻¹(f(x)))` against `f(x)` in physical units, within the ε clamp's
//!   tolerance (inverse_encode Part 4), at f32 and f64.
//! - `link_properties_c_log_det`: (c), the analytic log-det against the numeric Jacobian's log volume factor, `log |J|`
//!   or, for the 3×2 simplex Jacobian, `log √det(JᵀJ)` (R-368), at f64.
//! - `link_properties_d_c1`: (d), the central-difference derivative continuous along a fine grid through saturation.
//! - `link_selection_defaults_resolve_by_name`: the baked defaults are the named entries (REQ-GEN-013).
//! - `chart_constants_values`: the generated constants equal REQ-DEC-009's values at f32 and f64.
//! - `chart_constants_source_scan`: no float literal of a chart constant's value in `crates/kernel/src/{decode,encode}`.
//!
//! Each property is a check over a link behind function pointers, so its control runs the same check on a link broken
//! for it. The step and tolerances of (c) and (d) are REQ-GEN-025's calibration: the values here are the proposal,
//! with the measured discrepancy per link as evidence, unconfirmed until the M2 gate (R-71).

use std::path::Path;

use kernel::generated::constants::ChartConstants;
use kernel::generated::links::*;
use proptest::prelude::*;
#[cfg(feature = "controls")]
use std::f64::consts::{FRAC_PI_2, PI};
use validation::{negative_control, prop};

// ── The links under test ────────────────────────────────────────────────────────────────────────────────────────

/// A link at f64 and f32, behind function pointers, with its codomain's constraint and bounds.
#[derive(Clone, Copy)]
struct Under {
    name: &'static str,
    codomain: &'static str,
    constraint: Constraint,
    lo: f64,
    hi: f64,
    /// The size of the values its computation rounds, so the scale of its absolute round-off: 1 for the simplex,
    /// the larger end's magnitude for an interval, and 0 for a line, whose round-off is relative to its value.
    scale: f64,
    controls: usize,
    forward64: fn(&[f64]) -> Vec<f64>,
    inverse64: fn(&[f64]) -> Vec<f64>,
    log_det64: fn(&[f64]) -> f64,
    forward32: fn(&[f32]) -> Vec<f32>,
    inverse32: fn(&[f32]) -> Vec<f32>,
}

fn under<const C: usize, const P: usize, L: Link<C, P>>() -> Under {
    Under {
        name: L::NAME,
        codomain: <L::Codomain as Codomain>::NAME,
        constraint: <L::Codomain as Codomain>::CONSTRAINT,
        lo: <L::Codomain as Codomain>::LO,
        hi: <L::Codomain as Codomain>::HI,
        scale: match <L::Codomain as Codomain>::CONSTRAINT {
            Constraint::Simplex => 1.0,
            Constraint::Bounded | Constraint::Symmetric => {
                let (lo, hi) = (<L::Codomain as Codomain>::LO, <L::Codomain as Codomain>::HI);
                lo.abs().max(hi.abs())
            }
            Constraint::Positive | Constraint::Unbounded => 0.0,
        },
        controls: C,
        forward64: |v| L::forward::<f64>(v.try_into().expect("the controls")).to_vec(),
        inverse64: |v| L::inverse::<f64>(v.try_into().expect("the components")).to_vec(),
        log_det64: |v| L::log_det::<f64>(v.try_into().expect("the controls")),
        forward32: |v| L::forward::<f32>(v.try_into().expect("the controls")).to_vec(),
        inverse32: |v| L::inverse::<f32>(v.try_into().expect("the components")).to_vec(),
    }
}

/// `v` at the float `R`, for the broken links of the controls.
#[cfg(feature = "controls")]
fn num<R: LinkReal>(v: f64) -> R {
    R::from(v).expect("a finite f64 converts")
}

/// Every generated entry, in registry order.
fn registry() -> Vec<Under> {
    vec![
        under::<2, 3, SoftmaxTanh>(),
        under::<2, 3, StickBreaking>(),
        under::<1, 1, SigmoidAlpha>(),
        under::<1, 1, SigmoidBeta>(),
        under::<1, 1, TanhAlpha>(),
        under::<1, 1, TanhBeta>(),
        under::<1, 1, SoftsignAlpha>(),
        under::<1, 1, SoftsignBeta>(),
        under::<1, 1, SigmoidQ>(),
        under::<1, 1, TanhQ>(),
        under::<1, 1, SoftsignQ>(),
        under::<1, 1, Softplus>(),
        under::<1, 1, Exp>(),
        under::<1, 1, Identity>(),
    ]
}

/// The links under test are `names`, each once, in order.
fn check_covers(under: &[Under], names: &[&str]) {
    let tested: Vec<&str> = under.iter().map(|u| u.name).collect();
    assert_eq!(
        tested, names,
        "the links under test are not every generated entry"
    );
}

#[test]
fn link_properties_cover_every_entry() {
    check_covers(&registry(), &NAMES);
}

negative_control!(
    link_properties_cover_every_entry,
    "a list missing an entry must fail the cover check",
    expected = "the links under test are not every generated entry",
    check_covers(&registry()[1..], &NAMES)
);

// ── The domain ──────────────────────────────────────────────────────────────────────────────────────────────────

/// The fuzzed control range at f64 and f32: `|x| ≤ ⌊ln MAX⌋`, the widest over which `exp` stays finite, so every link
/// is saturated well inside it (σ by `|x| ≈ 17` at f32, 37 at f64).
const X64: f64 = 709.0;
const X32: f64 = 88.0;

/// A control drawn across the domain: from all of it, from the transition into saturation, or from the centre.
fn control() -> impl Strategy<Value = (usize, f64)> {
    (0usize..3, -1.0f64..=1.0)
}

/// The control `(k, u)` draws at the range `x_max`.
fn at((k, u): (usize, f64), x_max: f64) -> f64 {
    u * [x_max, 40.0, 4.0][k]
}

/// Fixed controls besides the fuzzed ones: the domain's ends, saturation and the centre.
fn fixed(x_max: f64) -> Vec<f64> {
    vec![
        -x_max, -40.0, -20.0, -10.0, -1.0, 0.0, 1.0, 10.0, 20.0, 40.0, x_max,
    ]
}

/// Every pair of fixed controls, or each fixed control, for a link of `controls` controls.
fn fixed_points(controls: usize, x_max: f64) -> Vec<Vec<f64>> {
    let xs = fixed(x_max);
    if controls == 1 {
        xs.iter().map(|&x| vec![x]).collect()
    } else {
        xs.iter()
            .flat_map(|&a| xs.iter().map(move |&b| vec![a, b]))
            .collect()
    }
}

/// Runs `check` on `u` at each fixed point and fuzzed point of its controls, at f64's domain and f32's.
fn over_domain(u: &Under, check: impl Fn(&Under, &[f64], f64)) {
    for x_max in [X64, X32] {
        for x in fixed_points(u.controls, x_max) {
            check(u, &x, x_max);
        }
    }
    let draws = proptest::collection::vec(control(), u.controls);
    prop::run(&draws, |d| {
        for x_max in [X64, X32] {
            let x: Vec<f64> = d.iter().map(|&c| at(c, x_max)).collect();
            check(u, &x, x_max);
        }
        Ok(())
    });
}

// ── (a) Constraint preservation ─────────────────────────────────────────────────────────────────────────────────

/// `y` satisfies `u`'s constraint at a float of machine epsilon `eps` whose bounds are `lo` and `hi`: simplex
/// outputs finite, positive and summing to 1 within the sum's round-off, `4·eps`; interval outputs finite and in
/// `[lo, hi]`, which a saturated control reaches exactly, as `σ(x)` rounds to 1; half-line outputs finite and
/// positive; real-line outputs finite.
fn check_constraint(u: &Under, x: &[f64], y: &[f64], eps: f64, (lo, hi): (f64, f64)) {
    let at = |what: &str| {
        format!(
            "{} at {x:?} gives {y:?}, {what} (constraint not preserved)",
            u.name
        )
    };
    assert!(y.iter().all(|v| v.is_finite()), "{}", at("not finite"));
    match u.constraint {
        Constraint::Simplex => {
            assert!(y.iter().all(|&m| m > 0.0), "{}", at("a mass not positive"));
            let sum: f64 = y.iter().sum();
            assert!(
                (sum - 1.0).abs() <= 4.0 * eps,
                "{}",
                at("masses not summing to 1")
            );
        }
        Constraint::Bounded | Constraint::Symmetric => {
            assert!(
                y.iter().all(|&v| lo <= v && v <= hi),
                "{}",
                at("out of range")
            );
        }
        Constraint::Positive => assert!(y.iter().all(|&v| v > 0.0), "{}", at("not positive")),
        Constraint::Unbounded => {}
    }
}

/// (a) at f64 and, where `x` is within f32's domain, at f32.
fn check_a(u: &Under, x: &[f64], x_max: f64) {
    if x_max == X64 {
        check_constraint(u, x, &(u.forward64)(x), f64::EPSILON, (u.lo, u.hi));
    } else {
        let x32: Vec<f32> = x.iter().map(|&v| v as f32).collect();
        let y: Vec<f64> = (u.forward32)(&x32).iter().map(|&v| v as f64).collect();
        let bounds = (u.lo as f32 as f64, u.hi as f32 as f64);
        check_constraint(u, x, &y, f32::EPSILON as f64, bounds);
    }
}

#[test]
fn link_properties_a_constraint() {
    for u in registry() {
        over_domain(&u, check_a);
    }
}

/// A bounded link onto `α` that overshoots its range by 1%: `y = 1.01·(π/2)·σ(x)`.
#[cfg(feature = "controls")]
struct Overshoot;

#[cfg(feature = "controls")]
impl Link<1, 1> for Overshoot {
    const NAME: &'static str = "overshoot";
    type Codomain = Alpha;
    fn forward<R: LinkReal>(v: [R; 1]) -> [R; 1] {
        let [y] = SigmoidAlpha::forward(v);
        [y * num::<R>(1.01)]
    }
    fn inverse<R: LinkReal>(v: [R; 1]) -> [R; 1] {
        SigmoidAlpha::inverse(v)
    }
    fn log_det<R: LinkReal>(v: [R; 1]) -> R {
        SigmoidAlpha::log_det(v)
    }
}

/// Softmax of unsaturated logits `5·z`, which underflows a mass to 0.
#[cfg(feature = "controls")]
struct BareSoftmax;

#[cfg(feature = "controls")]
impl Link<2, 3> for BareSoftmax {
    const NAME: &'static str = "bare_softmax";
    type Codomain = Mass;
    fn forward<R: LinkReal>(v: [R; 2]) -> [R; 3] {
        let e = |z: R| (z * R::MU_MAX).exp();
        let total = R::one() + e(v[0]) + e(v[1]);
        [R::one() / total, e(v[0]) / total, e(v[1]) / total]
    }
    fn inverse<R: LinkReal>(v: [R; 3]) -> [R; 2] {
        SoftmaxTanh::inverse(v)
    }
    fn log_det<R: LinkReal>(v: [R; 2]) -> R {
        SoftmaxTanh::log_det(v)
    }
}

negative_control!(
    link_properties_a_constraint,
    "a bounded link overshooting its range must fail the constraint check",
    expected = "out of range",
    over_domain(&under::<1, 1, Overshoot>(), check_a)
);

#[cfg(feature = "controls")]
mod simplex_constraint_control {
    use super::*;

    negative_control!(
        link_properties_a_constraint,
        "unsaturated softmax underflows a mass to 0 and must fail the positivity check",
        expected = "a mass not positive",
        over_domain(&under::<2, 3, BareSoftmax>(), check_a)
    );
}

// ── (b) Round trip in physical units ────────────────────────────────────────────────────────────────────────────

/// The clamp's tolerance in `u`'s physical units: how far the ε clamp can move a value that saturated past it
/// (inverse_encode Part 4). The mass inverse clamps the logits at `(1 − ε_μ)·μ_max`, moving each by at most
/// `ε_μ·μ_max`, and a mass by less; an interval's inverse clamps `s` or `u` within `ε` of its end, moving `y` by at
/// most `ε·(b − a)`; the lines have no clamp, so nothing.
fn clamp_tolerance(u: &Under) -> f64 {
    match u.codomain {
        "mass" => f64::EPS_MU * f64::MU_MAX,
        "alpha" | "beta" => f64::EPS_Z * (u.hi - u.lo),
        "momentum" => f64::EPS_Q * (u.hi - u.lo),
        _ => 0.0,
    }
}

/// `f(f⁻¹(y))` against `y = f(x)`, in physical units, within the clamp's tolerance and the round-off: the
/// computation's own, `4·eps` of its scale, and that of representing the control, which `exp` and `softplus` turn
/// from one ulp of `x` into `|x|·eps` relative in `y`, so `4·eps·(1 + |x|)·|y|`; the smallest normal is a floor for
/// subnormal `y`.
fn check_round_trip(u: &Under, x: &[f64], y: &[f64], again: &[f64], eps: f64, tiny: f64) {
    let span = 1.0 + x.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    for (a, b) in y.iter().zip(again) {
        let tol = clamp_tolerance(u) + 4.0 * eps * (u.scale + span * a.abs()) + tiny;
        assert!(
            (a - b).abs() <= tol,
            "{} at {x:?}: f(x) = {y:?}, f(f⁻¹(f(x))) = {again:?}: off by {} > {tol} (round trip)",
            u.name,
            (a - b).abs()
        );
    }
}

/// (b) at f64 and f32.
fn check_b(u: &Under, x: &[f64], x_max: f64) {
    if x_max == X64 {
        let y = (u.forward64)(x);
        let again = (u.forward64)(&(u.inverse64)(&y));
        check_round_trip(u, x, &y, &again, f64::EPSILON, f64::MIN_POSITIVE);
    } else {
        let x32: Vec<f32> = x.iter().map(|&v| v as f32).collect();
        let y = (u.forward32)(&x32);
        let again = (u.forward32)(&(u.inverse32)(&y));
        let widen = |v: &[f32]| v.iter().map(|&a| a as f64).collect::<Vec<_>>();
        let (eps, tiny) = (f32::EPSILON as f64, f32::MIN_POSITIVE as f64);
        check_round_trip(u, x, &widen(&y), &widen(&again), eps, tiny);
    }
}

#[test]
fn link_properties_b_round_trip() {
    for u in registry() {
        over_domain(&u, check_b);
    }
}

/// σ onto `β` whose inverse clamps `s` at `10⁻³`, not `ε_z`.
#[cfg(feature = "controls")]
struct WideClamp;

#[cfg(feature = "controls")]
impl Link<1, 1> for WideClamp {
    const NAME: &'static str = "wide_clamp";
    type Codomain = Beta;
    fn forward<R: LinkReal>(v: [R; 1]) -> [R; 1] {
        SigmoidBeta::forward(v)
    }
    fn inverse<R: LinkReal>(v: [R; 1]) -> [R; 1] {
        let eps = num::<R>(1e-3);
        let s = (v[0] / num::<R>(PI)).max(eps).min(R::one() - eps);
        [(s / (R::one() - s)).ln()]
    }
    fn log_det<R: LinkReal>(v: [R; 1]) -> R {
        SigmoidBeta::log_det(v)
    }
}

negative_control!(
    link_properties_b_round_trip,
    "an inverse clamping at 10⁻³ moves a saturated value past the ε clamp's tolerance",
    expected = "(round trip)",
    over_domain(&under::<1, 1, WideClamp>(), check_b)
);

// ── (c) The log-det against a numeric Jacobian ──────────────────────────────────────────────────────────────────

/// REQ-GEN-025's proposed (c) (R-71, unconfirmed until the M2 gate): the central-difference step, the control range
/// the comparison spans, and the tolerance on `|analytic − numeric|` of the log volume factor, beside the numeric
/// Jacobian's own round-off ([`c_allowance`]).
const C_STEP: f64 = 1e-5;
const C_RANGE: f64 = 20.0;
const C_TOL: f64 = 1e-5;

/// The numeric Jacobian of `u`'s forward at `x`, by central differences of step [`C_STEP`], column `k` the derivative
/// along control `k`, with each column's round-off bound: each value it differences is off by at most 16 ulps of
/// `max(|f|, scale)`, so each entry by at most `16·eps·max(|f|, scale)/h`.
fn jacobian(u: &Under, x: &[f64]) -> Vec<(Vec<f64>, f64)> {
    (0..u.controls)
        .map(|k| {
            let at = |d: f64| {
                let mut v = x.to_vec();
                v[k] += d;
                (u.forward64)(&v)
            };
            let (plus, minus) = (at(C_STEP), at(-C_STEP));
            let size = plus
                .iter()
                .chain(&minus)
                .fold(u.scale, |m, v| m.max(v.abs()));
            let col = plus
                .iter()
                .zip(&minus)
                .map(|(p, m)| (p - m) / (2.0 * C_STEP))
                .collect();
            (col, 16.0 * f64::EPSILON * size / C_STEP)
        })
        .collect()
}

/// The log of the volume factor of the Jacobian whose columns are `cols`: `log |J|` for one column of one row, and
/// `log √det(JᵀJ)` otherwise (R-368).
fn log_volume(cols: &[(Vec<f64>, f64)]) -> f64 {
    let dot = |a: &[f64], b: &[f64]| a.iter().zip(b).map(|(p, q)| p * q).sum::<f64>();
    match cols {
        [(c, _)] if c.len() == 1 => c[0].abs().ln(),
        [(a, _), (b, _)] => 0.5 * (dot(a, a) * dot(b, b) - dot(a, b) * dot(a, b)).ln(),
        _ => unreachable!("one control onto one value, or two onto the simplex"),
    }
}

/// The allowance on the log-det's gap: [`C_TOL`], and the numeric log volume's round-off, each column's entry bound
/// over its length, twice, `√P` entries to a column.
fn c_allowance(cols: &[(Vec<f64>, f64)]) -> f64 {
    let rounding: f64 = cols
        .iter()
        .map(|(c, noise)| {
            let len = c.iter().map(|v| v * v).sum::<f64>().sqrt();
            2.0 * (c.len() as f64).sqrt() * noise / len
        })
        .sum();
    C_TOL + rounding
}

/// `|analytic − numeric|` of `u`'s log-det at `x`, and its allowance.
fn log_det_gap(u: &Under, x: &[f64]) -> (f64, f64) {
    let cols = jacobian(u, x);
    let gap = ((u.log_det64)(x) - log_volume(&cols)).abs();
    (gap, c_allowance(&cols))
}

/// `u`'s analytic log-det matches the numeric Jacobian's within the allowance at `x`.
fn check_c(u: &Under, x: &[f64]) {
    let (gap, allowance) = log_det_gap(u, x);
    assert!(
        gap <= allowance,
        "{} at {x:?}: log-det {} against the numeric {}: off by {gap} > {allowance} (log-det)",
        u.name,
        (u.log_det64)(x),
        log_volume(&jacobian(u, x))
    );
}

/// The grid of `n + 1` controls per axis across `±C_RANGE`.
fn c_grid(controls: usize, n: usize) -> Vec<Vec<f64>> {
    let xs: Vec<f64> = (0..=n)
        .map(|i| -C_RANGE + 2.0 * C_RANGE * i as f64 / n as f64)
        .collect();
    if controls == 1 {
        xs.iter().map(|&x| vec![x]).collect()
    } else {
        xs.iter()
            .flat_map(|&a| xs.iter().map(move |&b| vec![a, b]))
            .collect()
    }
}

/// (c) over a grid and fuzzed controls across `±C_RANGE`.
fn check_c_over(u: &Under) {
    for x in c_grid(u.controls, 64) {
        check_c(u, &x);
    }
    let draws = proptest::collection::vec(-C_RANGE..=C_RANGE, u.controls);
    prop::run(&draws, |x| {
        check_c(u, &x);
        Ok(())
    });
}

#[test]
fn link_properties_c_log_det() {
    for u in registry() {
        check_c_over(&u);
    }
}

/// σ onto `α` whose log-det leaves out the width `π/2`: the log of `σ'`, not of the volume factor.
#[cfg(feature = "controls")]
struct NoWidth;

#[cfg(feature = "controls")]
impl Link<1, 1> for NoWidth {
    const NAME: &'static str = "no_width";
    type Codomain = Alpha;
    fn forward<R: LinkReal>(v: [R; 1]) -> [R; 1] {
        SigmoidAlpha::forward(v)
    }
    fn inverse<R: LinkReal>(v: [R; 1]) -> [R; 1] {
        SigmoidAlpha::inverse(v)
    }
    fn log_det<R: LinkReal>(v: [R; 1]) -> R {
        SigmoidAlpha::log_det(v) - num::<R>(FRAC_PI_2).ln()
    }
}

/// The simplex link whose log-det is `log |det|` of its first two rows, without the area element's `√3` (R-368).
#[cfg(feature = "controls")]
struct NoAreaElement;

#[cfg(feature = "controls")]
impl Link<2, 3> for NoAreaElement {
    const NAME: &'static str = "no_area_element";
    type Codomain = Mass;
    fn forward<R: LinkReal>(v: [R; 2]) -> [R; 3] {
        SoftmaxTanh::forward(v)
    }
    fn inverse<R: LinkReal>(v: [R; 3]) -> [R; 2] {
        SoftmaxTanh::inverse(v)
    }
    fn log_det<R: LinkReal>(v: [R; 2]) -> R {
        SoftmaxTanh::log_det(v) - num::<R>(3.0).sqrt().ln()
    }
}

negative_control!(
    link_properties_c_log_det,
    "a bounded log-det without the width must fail the numeric comparison",
    expected = "(log-det)",
    check_c_over(&under::<1, 1, NoWidth>())
);

#[cfg(feature = "controls")]
mod area_element_control {
    use super::*;

    negative_control!(
        link_properties_c_log_det,
        "a simplex log-det without the area element's √3 must fail the numeric comparison",
        expected = "(log-det)",
        check_c_over(&under::<2, 3, NoAreaElement>())
    );
}

// ── (d) C¹ ──────────────────────────────────────────────────────────────────────────────────────────────────────

/// REQ-GEN-025's proposed (d) (R-71, unconfirmed until the M2 gate): the central-difference step, the grid's spacing
/// and range, and the tolerance `τ` on the change of the derivative between neighbouring grid points, relative to
/// its size: `|D(x + δ) − D(x)| ≤ τ·max(|D(x)|, |D(x + δ)|)`, beside each difference's own round-off,
/// `16·eps·max(|f|, scale)/h`, as in (c).
const D_STEP: f64 = 1e-5;
const D_SPACING: f64 = 1e-3;
const D_RANGE: f64 = 40.0;
const D_TOL: f64 = 1e-2;

/// The second controls a simplex link's other control is swept at.
const D_ACROSS: [f64; 7] = [-30.0, -3.0, -0.5, 0.0, 0.7, 3.0, 30.0];

/// The derivative of `u`'s forward along control `k` at `x`, by central differences, and the round-off bound of
/// each component.
fn derivative(u: &Under, x: &[f64], k: usize) -> (Vec<f64>, Vec<f64>) {
    let at = |d: f64| {
        let mut v = x.to_vec();
        v[k] += d;
        (u.forward64)(&v)
    };
    let (plus, minus) = (at(D_STEP), at(-D_STEP));
    let d = plus
        .iter()
        .zip(&minus)
        .map(|(p, m)| (p - m) / (2.0 * D_STEP))
        .collect();
    let noise = plus
        .iter()
        .zip(&minus)
        .map(|(p, m)| 16.0 * f64::EPSILON * p.abs().max(m.abs()).max(u.scale) / D_STEP)
        .collect();
    (d, noise)
}

/// The largest change of `u`'s derivative along control `k` between neighbouring points of the line through `base`,
/// as a share of `τ`'s allowance: above 1 is a kink. The line is swept across `±D_RANGE` at spacing `D_SPACING`.
fn worst_kink(u: &Under, base: &[f64], k: usize) -> (f64, f64) {
    let n = (2.0 * D_RANGE / D_SPACING).round() as usize;
    let point = |i: usize| {
        let mut v = base.to_vec();
        v[k] = -D_RANGE + D_SPACING * i as f64;
        v
    };
    let mut worst = (0.0, 0.0);
    let mut prev = derivative(u, &point(0), k);
    for i in 1..=n {
        let next = derivative(u, &point(i), k);
        for c in 0..prev.0.len() {
            let (a, b) = (prev.0[c], next.0[c]);
            let allowance = D_TOL * a.abs().max(b.abs()) + prev.1[c] + next.1[c];
            let share = if allowance > 0.0 {
                (a - b).abs() / allowance
            } else {
                0.0
            };
            if share > worst.0 {
                worst = (share, point(i)[k]);
            }
        }
        prev = next;
    }
    worst
}

/// Each line `(base, k)` (d) sweeps for `u`.
fn d_lines(u: &Under) -> Vec<(Vec<f64>, usize)> {
    if u.controls == 1 {
        return vec![(vec![0.0], 0)];
    }
    D_ACROSS
        .iter()
        .flat_map(|&other| [(vec![0.0, other], 0), (vec![other, 0.0], 1)])
        .collect()
}

/// `u`'s derivative changes by no more than `τ`'s allowance between neighbouring grid points of every line.
fn check_d(u: &Under) {
    for (base, k) in d_lines(u) {
        let (share, at) = worst_kink(u, &base, k);
        assert!(
            share <= 1.0,
            "{}: along control {k} through {base:?} the derivative jumps at {at} by {share} × the allowance (C¹)",
            u.name
        );
    }
}

#[test]
fn link_properties_d_c1() {
    for u in registry() {
        check_d(&u);
    }
}

/// The hard sigmoid onto `α`, `(π/2)·clamp(½ + x/4, 0, 1)`: continuous, with σ's centre slope, but kinked at ±2.
#[cfg(feature = "controls")]
struct HardSigmoid;

#[cfg(feature = "controls")]
impl Link<1, 1> for HardSigmoid {
    const NAME: &'static str = "hard_sigmoid";
    type Codomain = Alpha;
    fn forward<R: LinkReal>(v: [R; 1]) -> [R; 1] {
        let s = (num::<R>(0.5) + v[0] / num::<R>(4.0))
            .max(R::zero())
            .min(R::one());
        [num::<R>(FRAC_PI_2) * s]
    }
    fn inverse<R: LinkReal>(v: [R; 1]) -> [R; 1] {
        SigmoidAlpha::inverse(v)
    }
    fn log_det<R: LinkReal>(v: [R; 1]) -> R {
        SigmoidAlpha::log_det(v)
    }
}

negative_control!(
    link_properties_d_c1,
    "a link kinked at ±2 must fail the C¹ check",
    expected = "(C¹)",
    check_d(&under::<1, 1, HardSigmoid>())
);

// ── The evidence for REQ-GEN-025 ────────────────────────────────────────────────────────────────────────────────

/// The largest (c) gap and (d) share of each of `links`, printed for the calibration's evidence (run with
/// `--nocapture`), each within its proposed tolerance.
fn check_evidence(links: &[Under]) {
    println!(
        "REQ-GEN-025 evidence (proposed, R-71): (c) h = {C_STEP}, |z| <= {C_RANGE}, tol {C_TOL}; \
         (d) h = {D_STEP}, spacing {D_SPACING}, |z| <= {D_RANGE}, tau = {D_TOL}"
    );
    for u in links {
        let gaps: Vec<(f64, f64)> = c_grid(u.controls, 64)
            .iter()
            .map(|x| log_det_gap(u, x))
            .collect();
        // The gap where the numeric Jacobian resolves it: its round-off under a tenth of the tolerance.
        let gap = gaps
            .iter()
            .filter(|g| g.1 - C_TOL <= 0.1 * C_TOL)
            .map(|g| g.0)
            .fold(0.0f64, f64::max);
        let c_share = gaps.iter().map(|g| g.0 / g.1).fold(0.0f64, f64::max);
        let share = d_lines(u)
            .iter()
            .map(|(b, k)| worst_kink(u, b, *k).0)
            .fold(0.0f64, f64::max);
        println!(
            "  {:<16} (c) max resolved gap = {gap:.3e}, max gap/allowance = {c_share:.3}   \
             (d) max change/allowance = {share:.3}",
            u.name
        );
        assert!(
            c_share <= 1.0 && share <= 1.0,
            "{} exceeds a proposed tolerance (evidence)",
            u.name
        );
    }
}

#[test]
fn link_properties_evidence() {
    check_evidence(&registry());
}

negative_control!(
    link_properties_evidence,
    "a link breaking (c) exceeds the proposed tolerance in the evidence",
    expected = "exceeds a proposed tolerance (evidence)",
    check_evidence(&[under::<1, 1, NoWidth>()])
);

// ── Selection ───────────────────────────────────────────────────────────────────────────────────────────────────

/// `ids` are dd_decoder §3's defaults, in `Selection`'s order: mass = softmax ∘ `μ_max·tanh`, config = sigmoid, free
/// momentum = sigmoid.
fn check_defaults(ids: [&str; 7]) {
    assert_eq!(
        ids,
        [
            "softmax_tanh",
            "sigmoid_alpha",
            "sigmoid_beta",
            "sigmoid_q",
            "sigmoid_q",
            "sigmoid_q",
            "sigmoid_q"
        ],
        "the baked defaults are not dd_decoder §3's"
    );
}

#[test]
fn link_selection_defaults_resolve_by_name() {
    check_defaults(<Defaults as Selection>::LINK_IDS);
}

negative_control!(
    link_selection_defaults_resolve_by_name,
    "a selection with tanh for α is not the defaults",
    expected = "the baked defaults are not dd_decoder §3's",
    check_defaults(
        <Select<SoftmaxTanh, TanhAlpha, SigmoidBeta, SigmoidQ, SigmoidQ, SigmoidQ, SigmoidQ> as Selection>::LINK_IDS
    )
);

// ── The chart constants ─────────────────────────────────────────────────────────────────────────────────────────

/// `got`, the chart constants in `ChartConstants`' order at some float, read as f64, are REQ-DEC-009's values at
/// that float: `μ_max = 5`, `q_max = 2`, `α_min = 0`, `ε_μ = ε_z = ε_q = 10⁻⁶`, `δ_λ = 10⁻¹²`, `ε_w = 10⁻¹⁰`, each
/// rounded once to the float by `round`.
fn check_chart_constants(got: [f64; 8], round: fn(f64) -> f64) {
    let want = [5.0, 2.0, 0.0, 1e-6, 1e-6, 1e-6, 1e-12, 1e-10].map(round);
    assert_eq!(got, want, "the chart constants are not REQ-DEC-009's");
}

/// The chart constants at `R`, as f64.
fn chart_constants<R: ChartConstants + Into<f64>>() -> [f64; 8] {
    [
        R::MU_MAX,
        R::Q_MAX,
        R::ALPHA_MIN,
        R::EPS_MU,
        R::EPS_Z,
        R::EPS_Q,
        R::DELTA_LAMBDA,
        R::EPS_W,
    ]
    .map(Into::into)
}

#[test]
fn chart_constants_values() {
    check_chart_constants(chart_constants::<f64>(), |v| v);
    check_chart_constants(chart_constants::<f32>(), |v| v as f32 as f64);
}

negative_control!(
    chart_constants_values,
    "μ_max = 4, the IC Inspector's value before R-10, must fail the check",
    expected = "the chart constants are not REQ-DEC-009's",
    {
        let mut got = chart_constants::<f64>();
        got[0] = 4.0;
        check_chart_constants(got, |v| v)
    }
);

/// The float literals of `source`, by line: tokens with a decimal point, an exponent or a float suffix, after its
/// comments and strings are blanked; an integer, such as an array length or index, is not one.
fn float_literals(source: &str) -> Vec<(usize, String)> {
    let mut code = String::new();
    let mut chars = source.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '/' if chars.peek() == Some(&'/') => {
                for d in chars.by_ref() {
                    if d == '\n' {
                        code.push('\n');
                        break;
                    }
                }
            }
            '"' => {
                while let Some(d) = chars.next() {
                    match d {
                        '\\' => {
                            chars.next();
                        }
                        '"' => break,
                        '\n' => code.push('\n'),
                        _ => {}
                    }
                }
            }
            _ => code.push(c),
        }
    }
    let mut found = Vec::new();
    for (n, line) in code.lines().enumerate() {
        let b = line.as_bytes();
        let mut i = 0;
        while i < b.len() {
            let starts = b[i].is_ascii_digit()
                && (i == 0
                    || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_' || b[i - 1] == b'.'));
            if !starts {
                i += 1;
                continue;
            }
            let mut j = i;
            while j < b.len()
                && (b[j].is_ascii_alphanumeric()
                    || b[j] == b'_'
                    || (b[j] == b'.' && b.get(j + 1).is_some_and(u8::is_ascii_digit))
                    || ((b[j] == b'-' || b[j] == b'+') && matches!(b[j - 1], b'e' | b'E')))
            {
                j += 1;
            }
            let token = &line[i..j];
            if token.contains('.')
                || token.contains(['e', 'E'])
                || token.ends_with("f32")
                || token.ends_with("f64")
            {
                found.push((n + 1, token.to_owned()));
            }
            i = j;
        }
    }
    found
}

/// The chart constants' values.
const CHART_VALUES: [f64; 8] = [5.0, 2.0, 0.0, 1e-6, 1e-6, 1e-6, 1e-12, 1e-10];

/// No float literal in `files` (path and source) equals a chart constant's value: the formulae name the constants
/// (REQ-DEC-009).
fn check_no_constant_literals(files: &[(String, String)]) {
    for (path, source) in files {
        for (line, token) in float_literals(source) {
            let digits = token
                .trim_end_matches("f32")
                .trim_end_matches("f64")
                .replace('_', "");
            if let Ok(v) = digits.parse::<f64>() {
                assert!(
                    !CHART_VALUES.contains(&v),
                    "{path}:{line}: the literal `{token}` is a chart constant's value; name the constant (REQ-DEC-009)"
                );
            }
        }
    }
}

/// Every `.rs` file under `dir`, recursively, with its path; none if `dir` does not exist.
fn sources(dir: &Path) -> Vec<(String, String)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.extend(sources(&path));
        } else if path.extension().is_some_and(|x| x == "rs") {
            let source = std::fs::read_to_string(&path).expect("a source reads");
            files.push((path.display().to_string(), source));
        }
    }
    files
}

#[test]
fn chart_constants_source_scan() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = sources(&src.join("decode"));
    files.extend(sources(&src.join("encode")));
    check_no_constant_literals(&files);
}

negative_control!(
    chart_constants_source_scan,
    "a decode formula with μ_max written as 5.0 must fail the scan",
    expected = "is a chart constant's value",
    check_no_constant_literals(&[(
        "decode/mass.rs".to_owned(),
        "// mu = 5.0 * tanh(z)\nlet mu = R::from(5.0).unwrap() * z.tanh(); let n = [z; 2];\n"
            .to_owned()
    )])
);
