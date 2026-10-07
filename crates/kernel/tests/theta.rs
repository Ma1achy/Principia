//! The live shape readout and the unwrapped phase `θ̃` (`kernel::shape`; dd_integrator §3.7; R-389, R-392), on
//! synthetic `n(t)` paths (REQ-INT-001; R-113: a real circulating orbit is REQ-INT-082's, M3):
//! - `theta_unwrap_circulating`: paths circulating the `w` axis, both ways, from starting longitudes on both sides of
//!   the ±π cut, accumulate with no 2π jump; `orbit_count` is the path's completed turns and `retrograde` its
//!   direction, by the read side's own derivations; the hold never fires on them.
//! - `theta_unwrap_starts_at_zero`: `θ̃(0) = 0` whatever `n(0)`'s longitude (R-389).
//! - `theta_unwrap_pole_passage`: a path through the pole disc adds nothing inside and exactly `wrap(exit − stored)` on
//!   exit, the stored longitude the last one outside; a run ending inside adds nothing for that passage (R-389).
//! - `theta_unwrap_exact_pi`: a per-step difference, and an exit-minus-stored difference, of exactly ±π each add +π;
//!   the wrap's edges either side of ±π (R-389).
//! - `theta_unwrap_pole_boundary`: a point at exactly `r_pole` is outside, the next float below it inside.
//! - `theta_unwrap_pole_disc_squared`: the disc's branch input, `fma(v, v, u·u)` against `r_pole²·I²` (R-34): exactly
//!   `r_pole²` is outside, the next float below it inside, at f32 and f64, normalised and unnormalised.
//! - `theta_unwrap_ic_inside`: an IC inside the disc leaves `θ̃` at 0 at its first exit and counts from the exit
//!   longitude; a later passage adds `wrap(exit − stored)` (R-392).
//! - `theta_unwrap_r_pole_evidence`: REQ-INT-086's evidence, the longitude's round-off against `ρ = √(n_u² + n_v²)` at
//!   f32 and f64, measured against an exact reference, and the proposed `r_pole`'s error.
//! - `shape_landmarks`: the shape map's landmarks (R-14; dd_integrator §3.7), normalised and unnormalised.
//!
//! Each runs at f64 and, where its path is exact at f32, at f32 too. `r_pole` is REQ-INT-086's calibration: the value
//! here is the proposal, unconfirmed until the M1 gate (R-71). Each check takes the step it tests as an argument, so
//! its control runs the same check on a faulty one.

use std::f64::consts::{FRAC_PI_2, PI, TAU};

use kernel::payload::{orbit_count, retrograde};
use kernel::shape::{in_pole_disc, longitude, shape, shape_unnormalised, theta_step, wrap, Theta};
use kernel::Real;
use spirv_std::num_traits::{Float, FloatConst};
use validation::negative_control;

/// The proposed pole radius (REQ-INT-086; proposed, R-71; unconfirmed until the M1 gate).
const R_POLE: f64 = 1e-2;

/// The step arc on the sphere of every synthetic path here stays below `π·r_pole`, about 0.031 rad, where it passes
/// near the disc: past that, a step's longitude change near the disc's edge can exceed π, and the principal value picks
/// the wrong branch.
const POLE_ARC: f64 = 2e-3;

/// One macro-step of `θ̃`: [`theta_step`], or a control's faulty one.
type Step<R> = fn(Theta<R>, [R; 3], [R; 3], R) -> Theta<R>;

/// `x` at the precision `R`.
fn real<R: Real + Float>(x: f64) -> R {
    <R as spirv_std::num_traits::NumCast>::from(x).expect("a finite f64")
}

/// The point at longitude `lon` and polar radius `rho` on the sphere, north (`w > 0`) or south.
fn point(lon: f64, rho: f64, north: bool) -> [f64; 3] {
    let w = (1.0 - rho * rho).sqrt();
    [rho * lon.cos(), rho * lon.sin(), if north { w } else { -w }]
}

/// `p` at the precision `R`.
fn at<R: Real + Float>(p: [f64; 3]) -> [R; 3] {
    p.map(real)
}

/// `θ̃` along `path` (`path[0]` the IC), stepped by `step` with pole radius `r_pole`: its value after each point,
/// `θ̃(0)` first.
fn run<R: Real + Float>(step: Step<R>, path: &[[R; 3]], r_pole: R) -> Vec<R> {
    let mut theta = Theta::start();
    let mut out = vec![theta.theta];
    for w in path.windows(2) {
        theta = step(theta, w[0], w[1], r_pole);
        out.push(theta.theta);
    }
    out
}

/// The principal value of `d` as R-389 defines it, written here independently of `kernel::shape::wrap`: `d − 2πk`
/// in (−π, π], so exactly ±π gives +π.
fn principal(d: f64) -> f64 {
    let k = ((d - PI) / TAU).ceil();
    let r = d - k * TAU;
    if r <= -PI {
        r + TAU
    } else {
        r
    }
}

// ── Circulating paths ───────────────────────────────────────────────────────────────────────────────────────────────

/// A latitude circle at polar radius `rho`, from longitude `lon0`, `turns` turns in direction `dir` (±1), in
/// `per_turn` steps a turn.
fn circle(lon0: f64, rho: f64, dir: f64, turns: f64, per_turn: usize) -> Vec<[f64; 3]> {
    let steps = (turns * per_turn as f64).round() as usize;
    (0..=steps)
        .map(|s| point(lon0 + dir * TAU * s as f64 / per_turn as f64, rho, true))
        .collect()
}

/// A great circle tilted `tilt` from the equator, through `(cos lon0, sin lon0, 0)`: it circulates the `w` axis, its
/// polar radius never below `cos tilt`.
fn tilted(lon0: f64, tilt: f64, dir: f64, turns: f64, per_turn: usize) -> Vec<[f64; 3]> {
    let steps = (turns * per_turn as f64).round() as usize;
    let e1 = [lon0.cos(), lon0.sin(), 0.0];
    let e2 = [
        -lon0.sin() * tilt.cos(),
        lon0.cos() * tilt.cos(),
        tilt.sin(),
    ];
    (0..=steps)
        .map(|s| {
            let a = dir * TAU * s as f64 / per_turn as f64;
            [0, 1, 2].map(|c| a.cos() * e1[c] + a.sin() * e2[c])
        })
        .collect()
}

/// `θ̃` along a path circulating the `w` axis `turns` times in direction `dir`: 0 at the start, no step's increment
/// past π/2 (no 2π jump), the total `dir · 2π · turns`, `orbit_count` the completed turns and `retrograde` the
/// direction; and the hold never fires, `θ̃` the same with `r_pole = 0`.
fn check_circulating<R: Real + Float + FloatConst>(
    step: Step<R>,
    path: &[[f64; 3]],
    dir: f64,
    turns: f64,
    tol: f64,
) {
    let path: Vec<[R; 3]> = path.iter().map(|&p| at(p)).collect();
    let theta = run(step, &path, real(R_POLE));
    assert_eq!(theta[0], R::zero(), "θ̃(0) is not 0");
    for (k, w) in theta.windows(2).enumerate() {
        let inc: f64 = (w[1] - w[0]).to_f64().unwrap();
        assert!(
            inc.abs() < FRAC_PI_2,
            "θ̃ jumps by {inc} at step {k}, a 2π jump"
        );
    }
    let last: f64 = theta.last().unwrap().to_f64().unwrap();
    assert!(
        (last - dir * TAU * turns).abs() < tol,
        "θ̃ is {last} after {turns} turns, not {}",
        dir * TAU * turns
    );
    let last32 = last as f32;
    assert_eq!(
        orbit_count(last32),
        turns.floor() as u32,
        "orbit_count is not the path's completed turns"
    );
    assert_eq!(
        retrograde(last32),
        dir < 0.0,
        "retrograde is not the path's direction"
    );
    let unheld = run(step, &path, R::zero());
    assert_eq!(theta, unheld, "the hold fired on a circulating path");
}

/// The circulating cases: latitude circles at polar radii 0.6 and 0.05 (step arc ≈ 0.059 and ≈ 0.0049 rad) and a
/// great circle tilted 60°, each way, from longitudes on both sides of the ±π cut.
fn circulating_cases() -> Vec<(Vec<[f64; 3]>, f64, f64)> {
    let mut cases = Vec::new();
    for lon0 in [0.0, 2.5, PI, -PI + 1e-3, -2.0] {
        for dir in [1.0, -1.0] {
            cases.push((circle(lon0, 0.6, dir, 3.25, 64), dir, 3.25));
            cases.push((circle(lon0, 0.05, dir, 2.5, 64), dir, 2.5));
            cases.push((tilted(lon0, PI / 3.0, dir, 4.75, 96), dir, 4.75));
        }
    }
    cases
}

#[test]
fn theta_unwrap_circulating() {
    for (path, dir, turns) in circulating_cases() {
        check_circulating::<f64>(theta_step, &path, dir, turns, 1e-9);
        check_circulating::<f32>(theta_step, &path, dir, turns, 1e-3);
    }
}

/// A step that adds the raw longitude difference, unwrapped: its first crossing of the ±π cut jumps by 2π.
#[cfg(feature = "controls")]
fn raw_step(theta: Theta<f64>, n_prev: [f64; 3], n: [f64; 3], _r_pole: f64) -> Theta<f64> {
    Theta {
        theta: theta.theta + longitude(n) - longitude(n_prev),
        ..theta
    }
}

negative_control!(
    theta_unwrap_circulating,
    "a step adding the raw longitude difference must jump by 2π at the cut",
    expected = "a 2π jump",
    check_circulating::<f64>(raw_step, &circle(2.5, 0.6, 1.0, 3.25, 64), 1.0, 3.25, 1e-9)
);

// ── The start ───────────────────────────────────────────────────────────────────────────────────────────────────────

/// `θ̃` from a start `start(n(0))`, one step of `π/8` east from each longitude `lon0`: `θ̃(0) = 0`, and the step adds
/// `π/8` whatever `lon0`.
fn check_start(start: fn([f64; 3]) -> Theta<f64>) {
    for k in -16..=16 {
        let lon0 = PI * k as f64 / 16.0;
        let (n0, n1) = (point(lon0, 0.5, true), point(lon0 + PI / 8.0, 0.5, true));
        let t0 = start(n0);
        assert_eq!(
            t0.theta, 0.0,
            "θ̃(0) is {} at n(0)'s longitude {lon0}",
            t0.theta
        );
        let t1 = theta_step(t0, n0, n1, R_POLE);
        assert!(
            (t1.theta - PI / 8.0).abs() < 1e-12,
            "θ̃(0) is {} at n(0)'s longitude {lon0}, after one step of π/8",
            t1.theta - PI / 8.0
        );
    }
}

#[test]
fn theta_unwrap_starts_at_zero() {
    check_start(|_| Theta::start());
    let f32_start: Theta<f32> = Theta::start();
    assert_eq!(
        (f32_start.theta, f32_start.has_frozen),
        (0.0, false),
        "θ̃(0) is not 0 at f32"
    );
}

negative_control!(
    theta_unwrap_starts_at_zero,
    "a θ̃ starting at n(0)'s longitude must fail the start check",
    expected = "θ̃(0) is",
    check_start(|n0| Theta {
        theta: longitude(n0),
        ..Theta::start()
    })
);

// ── The pole passage ────────────────────────────────────────────────────────────────────────────────────────────────

/// A path along the meridian at `lon_in` from polar radius 0.3 into the disc, a wander inside it through longitudes
/// all round, and out along the meridian at `lon_out` to polar radius 0.3; every step's arc at most [`POLE_ARC`]. The
/// path and the index of its last point outside before the disc and of its first point outside after it.
fn passage(lon_in: f64, lon_out: f64, end_inside: bool) -> (Vec<[f64; 3]>, usize, usize) {
    let rho_step = POLE_ARC;
    let mut path = Vec::new();
    let mut rho = 0.3;
    while rho >= R_POLE {
        // Spiral in: the longitude drifts as it approaches, so the last point outside differs from the first.
        path.push(point(lon_in + (0.3 - rho), rho, true));
        rho -= rho_step;
    }
    let last_out = path.len() - 1;
    let stored = lon_in + (0.3 - (rho + rho_step));
    for k in 0..40 {
        path.push(point(stored + 0.35 + 0.7 * k as f64, R_POLE * 0.5, true));
    }
    if end_inside {
        return (path, last_out, usize::MAX);
    }
    let first_out = path.len();
    let mut rho = R_POLE;
    while rho <= 0.3 {
        path.push(point(lon_out, rho, true));
        rho += rho_step;
    }
    (path, last_out, first_out)
}

/// `θ̃` along a pole passage from `lon_in` to `lon_out`: unchanged at every point inside, and on exit exactly
/// `wrap(longitude(exit) − longitude(last point outside))`; for a run ending inside, unchanged from the entry on.
fn check_passage(step: Step<f64>, lon_in: f64, lon_out: f64, end_inside: bool) {
    let (path, last_out, first_out) = passage(lon_in, lon_out, end_inside);
    let theta = run(step, &path, R_POLE);
    let held = theta[last_out];
    let inside_end = if end_inside { path.len() } else { first_out };
    for (k, t) in theta.iter().enumerate().take(inside_end).skip(last_out + 1) {
        assert_eq!(*t, held, "θ̃ changed inside the disc, at point {k}");
    }
    if end_inside {
        return;
    }
    let want = principal(longitude(path[first_out]) - longitude(path[last_out]));
    assert_eq!(
        theta[first_out],
        held + want,
        "the exit adds {}, not wrap(exit − stored) = {want}",
        theta[first_out] - held
    );
}

#[test]
fn theta_unwrap_pole_passage() {
    for (lon_in, lon_out) in [(0.2, 1.4), (2.9, -2.9), (-1.0, 2.0), (0.5, 0.5 + PI - 0.1)] {
        check_passage(theta_step, lon_in, lon_out, false);
        check_passage(theta_step, lon_in, lon_out, true);
    }
}

/// A step with no hold: it differences every consecutive pair, the disc's untrusted longitudes included.
#[cfg(feature = "controls")]
fn unheld_step(theta: Theta<f64>, n_prev: [f64; 3], n: [f64; 3], _r_pole: f64) -> Theta<f64> {
    theta_step(theta, n_prev, n, 0.0)
}

/// A step that, on exit, differences against the first longitude inside the disc rather than the last outside.
#[cfg(feature = "controls")]
fn entry_inside_step(theta: Theta<f64>, n_prev: [f64; 3], n: [f64; 3], r_pole: f64) -> Theta<f64> {
    let next = theta_step(theta, n_prev, n, r_pole);
    if in_pole_disc(n, r_pole) && !in_pole_disc(n_prev, r_pole) {
        return Theta {
            frozen: longitude(n),
            ..next
        };
    }
    next
}

negative_control!(
    theta_unwrap_pole_passage,
    "a step with no hold must change θ̃ inside the disc",
    expected = "θ̃ changed inside the disc",
    check_passage(unheld_step, 0.2, 1.4, false)
);

#[cfg(feature = "controls")]
mod stored_control {
    use super::*;

    negative_control!(
        theta_unwrap_pole_passage,
        "a hold storing the first longitude inside, not the last outside, must fail the exit's delta",
        expected = "not wrap(exit − stored)",
        check_passage(entry_inside_step, 0.2, 1.4, false)
    );
}

// ── Exactly ±π ──────────────────────────────────────────────────────────────────────────────────────────────────────

/// A wrap: [`wrap`], or a control's faulty one.
type Wrap = fn(f64) -> f64;

/// `wrap` at and either side of ±π and ±2π: exactly ±π gives +π; just past +π wraps to just past −π; just inside −π
/// stays; ±2π gives 0.
fn check_wrap(wrap: Wrap) {
    let above_pi = f64::from_bits(PI.to_bits() + 1);
    let above_minus_pi = f64::from_bits((-PI).to_bits() - 1);
    let cases = [
        (PI, PI),
        (-PI, PI),
        (above_pi, above_pi - TAU),
        (above_minus_pi, above_minus_pi),
        (TAU, 0.0),
        (-TAU, 0.0),
        (0.25, 0.25),
        (-0.25, -0.25),
        (3.0 * FRAC_PI_2, -FRAC_PI_2),
        (-3.0 * FRAC_PI_2, FRAC_PI_2),
    ];
    for (d, want) in cases {
        assert_eq!(wrap(d), want, "wrap({d}) adds {}, not {want}", wrap(d));
    }
}

/// Two points at polar radius `scale`, a power of two, whose longitudes differ by exactly π at the precision `R`:
/// `west` on the negative `u` axis, at the longitude the platform's `atan2` gives it (π itself at f64; at f32 the
/// native `atan2` gives the float one below π), and `east` just off the positive `u` axis at that longitude minus π,
/// a difference exact by Sterbenz's lemma, which `atan2` returns unchanged as `y/x` is tiny. `(east, west)`; panics
/// unless their longitudes differ by exactly π.
fn antipodes<R: Real + Float + FloatConst>(scale: R, w: R) -> ([R; 3], [R; 3]) {
    let west = [-scale, R::zero(), w];
    let y = longitude(west) - R::PI();
    let east = [scale, scale * y, w];
    assert_eq!(
        longitude(west) - longitude(east),
        R::PI(),
        "the test's antipodes do not differ by exactly π"
    );
    (east, west)
}

/// A step's, and a passage's, exact ±π differences each add +π, at the precision `R`.
fn check_exact_pi<R: Real + Float + FloatConst>(step: Step<R>) {
    let pi = R::PI();
    let r_pole: R = real(R_POLE);
    // Per step: east → west is +π, west → east −π.
    let (east, west) = antipodes(R::one(), R::zero());
    for (from, to) in [(east, west), (west, east)] {
        let t = step(Theta::start(), from, to, r_pole);
        assert_eq!(
            t.theta, pi,
            "a step of exactly ±π adds {:?}, not +π",
            t.theta
        );
    }
    // A pole passage, entering and leaving at polar radius 1/32, outside the disc, inside at r_pole/2.
    let scale: R = real(1.0 / 32.0);
    let w = Float::sqrt(R::one() - scale * scale);
    let (east, west) = antipodes(scale, w);
    let inside = [R::zero(), r_pole / (R::one() + R::one()), w];
    for (enter, exit) in [(east, west), (west, east)] {
        let t = step(Theta::start(), enter, inside, r_pole);
        let t = step(t, inside, inside, r_pole);
        let t = step(t, inside, exit, r_pole);
        assert_eq!(
            t.theta, pi,
            "a passage of exactly ±π adds {:?}, not +π",
            t.theta
        );
    }
}

#[test]
fn theta_unwrap_exact_pi() {
    check_wrap(wrap::<f64>);
    check_exact_pi::<f64>(theta_step);
    check_exact_pi::<f32>(theta_step);
    let pi32 = std::f32::consts::PI;
    assert_eq!(wrap(pi32), pi32, "wrap(π) at f32 is not +π");
    assert_eq!(wrap(-pi32), pi32, "wrap(−π) at f32 is not +π");
}

/// A wrap into [−π, π): exactly +π gives −π.
#[cfg(feature = "controls")]
fn half_open_low(d: f64) -> f64 {
    if d >= PI {
        d - TAU
    } else if d < -PI {
        d + TAU
    } else {
        d
    }
}

/// A step wrapping into [−π, π).
#[cfg(feature = "controls")]
fn low_step(theta: Theta<f64>, n_prev: [f64; 3], n: [f64; 3], r_pole: f64) -> Theta<f64> {
    let next = theta_step(theta, n_prev, n, r_pole);
    if next.theta - theta.theta == PI {
        return Theta {
            theta: theta.theta - PI,
            ..next
        };
    }
    next
}

negative_control!(
    theta_unwrap_exact_pi,
    "a wrap into [−π, π) must fail at exactly +π",
    expected = "adds",
    check_wrap(half_open_low)
);

#[cfg(feature = "controls")]
mod exact_pi_step_control {
    use super::*;

    negative_control!(
        theta_unwrap_exact_pi,
        "a step adding −π for an exact ±π difference must fail",
        expected = "a step of exactly ±π adds",
        check_exact_pi::<f64>(low_step)
    );
}

// ── The disc's edge ─────────────────────────────────────────────────────────────────────────────────────────────────

/// A float's neighbours, for the disc's edge.
trait Ulp: Real + Float + FloatConst + std::fmt::Debug {
    /// The next float below `self`, `self` positive.
    fn below(self) -> Self;
}

impl Ulp for f64 {
    fn below(self) -> Self {
        f64::from_bits(self.to_bits() - 1)
    }
}

impl Ulp for f32 {
    fn below(self) -> Self {
        f32::from_bits(self.to_bits() - 1)
    }
}

/// The `w ≥ 0` for which the disc's `I² = fma(w, w, rho2)` is exactly 1, near `√(1 − rho2)`: so `r_pole²·I²` is
/// `r_pole²` exactly, and a point's side of the edge is its `ρ²_I`'s.
fn unit_w<R: Ulp>(rho2: R) -> R {
    let mut w = Float::sqrt(R::one() - rho2);
    for _ in 0..16 {
        let i2 = Float::mul_add(w, w, rho2);
        if i2 == R::one() {
            return w;
        }
        w = if i2 > R::one() {
            w.below()
        } else {
            w + (w - w.below())
        };
    }
    panic!("no w puts I² at exactly 1 for ρ²_I = {rho2:?}")
}

/// A shape point `(u, v, w)` whose disc input `fma(v, v, u·u)` is exactly `rho2` and whose `I²` is exactly 1, `rho2`
/// near `r_pole²`: `u` at or just below `√rho2`, `v` the small remainder, tuned to the float.
fn on_rho2<R: Ulp>(rho2: R) -> [R; 3] {
    let mut u = Float::sqrt(rho2);
    for _ in 0..64 {
        if u * u <= rho2 {
            let mut v = Float::sqrt(rho2 - u * u);
            for _ in 0..64 {
                let got = Float::mul_add(v, v, u * u);
                if got == rho2 {
                    return [u, v, unit_w(rho2)];
                }
                if got > rho2 {
                    break;
                }
                v = v + (v - v.below()).max(R::min_positive_value());
            }
        }
        u = u.below();
    }
    panic!("no point has ρ²_I = {rho2:?} exactly")
}

/// The disc's squared edge (R-34): a point with `ρ²_I` exactly `r_pole²` (`I² = 1`) is outside, and one with `ρ²_I`
/// the next float below it is inside; both the same scaled by 16, unnormalised, as [`shape_unnormalised`]'s point is.
fn check_squared_edge<R: Ulp>(disc: fn([R; 3], R) -> bool) {
    let r_pole: R = real(R_POLE);
    let r2 = r_pole * r_pole;
    let sixteen: R = real(16.0);
    for (rho2, inside, side) in [
        (r2, false, "exactly r_pole²"),
        (r2.below(), true, "just below r_pole²"),
    ] {
        let n = on_rho2(rho2);
        assert_eq!(
            disc(n, r_pole),
            inside,
            "a point with ρ²_I {side} is {}",
            if inside { "outside" } else { "inside" }
        );
        assert_eq!(
            disc(n.map(|x| x * sixteen), r_pole),
            inside,
            "the unnormalised point with ρ²_I {side} is on the other side"
        );
    }
}

#[test]
fn theta_unwrap_pole_disc_squared() {
    check_squared_edge::<f64>(in_pole_disc);
    check_squared_edge::<f32>(in_pole_disc);
}

/// A closed disc, `ρ²_I ≤ r_pole²·I²`.
#[cfg(feature = "controls")]
fn closed_disc(n: [f64; 3], r_pole: f64) -> bool {
    let rho2 = n[1].mul_add(n[1], n[0] * n[0]);
    rho2 <= r_pole * r_pole * n[2].mul_add(n[2], rho2)
}

negative_control!(
    theta_unwrap_pole_disc_squared,
    "a closed disc must hold a point at exactly r_pole²",
    expected = "with ρ²_I exactly r_pole² is inside",
    check_squared_edge::<f64>(closed_disc)
);

#[cfg(feature = "controls")]
mod squared_edge_control {
    use super::*;

    /// The disc of the next float below `r_pole`.
    fn narrow_disc(n: [f32; 3], r_pole: f32) -> bool {
        in_pole_disc(n, r_pole.below())
    }

    negative_control!(
        theta_unwrap_pole_disc_squared,
        "a disc one float narrower must leave a point just below r_pole² outside",
        expected = "with ρ²_I just below r_pole² is outside",
        check_squared_edge::<f32>(narrow_disc)
    );
}

/// A point at exactly `r_pole` is outside, its step's delta added; the next float below `r_pole` is inside, held. Each
/// point's `I²` is exactly 1.
fn check_edge<R: Ulp>(step: Step<R>) {
    let r_pole: R = real(R_POLE);
    let w = unit_w(r_pole * r_pole);
    let from = [real::<R>(0.5), R::zero(), w];
    let on_edge = [R::zero(), r_pole, w];
    let t = step(Theta::start(), from, on_edge, r_pole);
    assert_eq!(
        t.theta,
        R::FRAC_PI_2(),
        "a point at exactly r_pole is held, not outside"
    );
    let below = r_pole.below();
    let inside = [R::zero(), below, unit_w(below * below)];
    let t = step(Theta::start(), from, inside, r_pole);
    assert_eq!(t.theta, R::zero(), "a point just inside r_pole is not held");
    // Back out, onto the edge on the u axis: the passage adds the wrapped exit-minus-stored, 0.
    let t = step(t, inside, [r_pole, R::zero(), w], r_pole);
    assert_eq!(
        t.theta,
        R::zero(),
        "the edge exit adds {:?}, not 0",
        t.theta
    );
}

#[test]
fn theta_unwrap_pole_boundary() {
    check_edge::<f64>(theta_step);
    check_edge::<f32>(theta_step);
}

/// A step whose disc is closed, `ρ ≤ r_pole`.
#[cfg(feature = "controls")]
fn closed_disc_step(theta: Theta<f64>, n_prev: [f64; 3], n: [f64; 3], r_pole: f64) -> Theta<f64> {
    theta_step(theta, n_prev, n, f64::from_bits(r_pole.to_bits() + 1))
}

negative_control!(
    theta_unwrap_pole_boundary,
    "a closed disc must hold a point at exactly r_pole",
    expected = "a point at exactly r_pole is held",
    check_edge::<f64>(closed_disc_step)
);

// ── An IC inside the disc (R-392) ───────────────────────────────────────────────────────────────────────────────────

/// An IC at longitude 2.0 inside the disc, a wander inside, a first exit along the meridian at `lon_exit`, a turn and
/// a half east at polar radius 0.3, and a second pole passage. `θ̃` is 0 throughout and at the first exit, then
/// counts from the exit longitude, and the second passage adds `wrap(exit − stored)`.
fn check_ic_inside(step: Step<f64>, lon_exit: f64) {
    let mut path: Vec<[f64; 3]> = (0..20)
        .map(|k| point(2.0 + 0.9 * k as f64, R_POLE * 0.4, true))
        .collect();
    let first_exit = path.len();
    let mut rho = R_POLE;
    while rho <= 0.3 {
        path.push(point(lon_exit, rho, true));
        rho += POLE_ARC;
    }
    let circled = path.len();
    path.extend(circle(lon_exit, 0.3, 1.0, 1.5, 128).into_iter().skip(1));
    let (passage, last_out, first_out) = passage(lon_exit + PI + 0.3, lon_exit + 0.4, false);
    let offset = path.len();
    path.extend(passage);
    let theta = run(step, &path, R_POLE);
    for (k, t) in theta.iter().enumerate().take(first_exit + 1) {
        assert_eq!(
            *t, 0.0,
            "θ̃ is {t} at point {k}, inside or at the first exit"
        );
    }
    let turned = theta[circled + 191] - theta[circled - 1];
    assert!(
        (turned - 1.5 * TAU).abs() < 1e-9,
        "θ̃ turned {turned} in a turn and a half from the exit"
    );
    let (last_out, first_out) = (offset + last_out, offset + first_out);
    let want = principal(longitude(path[first_out]) - longitude(path[last_out]));
    assert_eq!(
        theta[first_out],
        theta[last_out] + want,
        "the later passage adds {}, not wrap(exit − stored) = {want}",
        theta[first_out] - theta[last_out]
    );
}

#[test]
fn theta_unwrap_ic_inside() {
    for lon_exit in [-2.5, 0.0, 1.0, PI] {
        check_ic_inside(theta_step, lon_exit);
    }
}

/// A step that, at an IC-inside first exit, differences against the IC's longitude (RQ-225's option 2, which R-392
/// rejects).
#[cfg(feature = "controls")]
fn from_ic_step(theta: Theta<f64>, n_prev: [f64; 3], n: [f64; 3], r_pole: f64) -> Theta<f64> {
    let theta = if theta.has_frozen || theta.theta != 0.0 {
        theta
    } else {
        Theta {
            frozen: 2.0,
            has_frozen: true,
            ..theta
        }
    };
    theta_step(theta, n_prev, n, r_pole)
}

negative_control!(
    theta_unwrap_ic_inside,
    "a first exit differencing against the IC's longitude must fail R-392",
    expected = "inside or at the first exit",
    check_ic_inside(from_ic_step, 0.0)
);

// ── REQ-INT-086's evidence: the longitude's round-off against ρ ─────────────────────────────────────────────────────

/// A small deterministic generator (splitmix64), so the evidence is the same every run.
struct Mix(u64);

impl Mix {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        ((z ^ (z >> 31)) >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// The masses the evidence uses, `(1/4, 1/4, 1/2)`: exact in binary, and with them the exact longitude of a
/// configuration of dyadic positions is `atan2(√8 ρ·λ, |ρ|² − 2|λ|²)` (μ_ρ = 1/8, μ_λ = 1/4), whose two arguments are
/// exact integers in units of 2⁻⁴⁸.
const MASSES: [f64; 3] = [0.25, 0.25, 0.5];

/// The grids positions are on, as the units' reciprocals: for f32, integers in units of 2⁻²³; for f64, in units of
/// 2⁻⁵¹. Positions stay below 2 in magnitude, so the integers stay below 2²⁴ and 2⁵², exact at their precision, and
/// the f64 ones use its whole significand: on the f32 grid the f64 map would be exact, and measure nothing.
const GRID32: f64 = (1u64 << 23) as f64;
const GRID64: f64 = (1u64 << 51) as f64;

/// A configuration near the shape `point(lon, rho, north)`, rotated by `gamma` and scaled by `scale` (at most 0.8, so
/// every position is below 2 in magnitude), its positions rounded to `grid`: the integer positions.
fn configuration(
    lon: f64,
    rho: f64,
    north: bool,
    gamma: f64,
    scale: f64,
    grid: f64,
) -> [[i64; 2]; 3] {
    let n = point(lon, rho, north);
    // The canonical frame (chart_reference §0.2): n = (cos 2α, sin 2α cos β, sin 2α sin β).
    let alpha = n[0].clamp(-1.0, 1.0).acos() / 2.0;
    let beta = n[2].atan2(n[1]);
    let (c, s) = (gamma.cos(), gamma.sin());
    let rot = |v: [f64; 2]| [scale * (c * v[0] - s * v[1]), scale * (s * v[0] + c * v[1])];
    let (mu_rho, mu_lambda) = (1.0 / 8.0, 1.0 / 4.0);
    let rho_v = rot([alpha.cos() / mu_rho.sqrt(), 0.0]);
    let lam_v = rot([
        alpha.sin() * beta.cos() / mu_lambda.sqrt(),
        alpha.sin() * beta.sin() / mu_lambda.sqrt(),
    ]);
    // Positions (chart_reference §0.2, M = 1): r01 = −m2 λ, r2 = M01 λ, r0 = r01 − (m1/M01) ρ, r1 = r01 + (m0/M01) ρ.
    let r01 = lam_v.map(|x| -MASSES[2] * x);
    let r2 = lam_v.map(|x| (MASSES[0] + MASSES[1]) * x);
    let r0 = [r01[0] - 0.5 * rho_v[0], r01[1] - 0.5 * rho_v[1]];
    let r1 = [r01[0] + 0.5 * rho_v[0], r01[1] + 0.5 * rho_v[1]];
    [r0, r1, r2].map(|r| r.map(|x| (x * grid).round() as i64))
}

/// The configuration's exact longitude, to about 2 ulp, and its exact polar radius `ρ`.
fn exact(n: [[i64; 2]; 3]) -> (f64, f64) {
    let [n0, n1, n2] = n.map(|v| v.map(i128::from));
    let p = [2 * (n1[0] - n0[0]), 2 * (n1[1] - n0[1])];
    let l = [2 * n2[0] - n0[0] - n1[0], 2 * n2[1] - n0[1] - n1[1]];
    let pp = p[0] * p[0] + p[1] * p[1];
    let ll = l[0] * l[0] + l[1] * l[1];
    let x = (pp - 2 * ll) as f64;
    let y = 8f64.sqrt() * (p[0] * l[0] + p[1] * l[1]) as f64;
    (y.atan2(x), x.hypot(y) / (pp + 2 * ll) as f64)
}

/// The longitude `map` gives the configuration on `grid` at the precision `R`.
fn measured<R: Real + Float>(
    n: [[i64; 2]; 3],
    grid: f64,
    map: fn([[R; 2]; 3], [R; 3]) -> [R; 3],
) -> f64 {
    let r = n.map(|v| v.map(|x| real::<R>(x as f64 / grid)));
    longitude(map(r, MASSES.map(real))).to_f64().unwrap()
}

/// One decade of ρ: its bounds, the samples in it, and at each precision the largest longitude error and the largest
/// error × ρ / ε, the error's constant if it grows as ε/ρ.
#[derive(Debug, Default, Clone, Copy)]
struct Row {
    lo: f64,
    count32: usize,
    count64: usize,
    max32: f64,
    c32: f64,
    max64: f64,
    c64: f64,
}

/// The measurement: 4000 configurations per target ρ, at ρ from 10⁻¹ down to 10⁻⁶, binned by their exact ρ into
/// decades, the longitude `map` gives at f32 and f64 against the exact one.
fn measure(
    map32: fn([[f32; 2]; 3], [f32; 3]) -> [f32; 3],
    map64: fn([[f64; 2]; 3], [f64; 3]) -> [f64; 3],
) -> Vec<Row> {
    let mut rows: Vec<Row> = (1..=7)
        .map(|d| Row {
            lo: 10f64.powi(-d),
            ..Row::default()
        })
        .collect();
    let mut mix = Mix(0x5eed_0086);
    for target in [
        1e-1, 3e-2, 1e-2, 3e-3, 1e-3, 3e-4, 1e-4, 3e-5, 1e-5, 3e-6, 1e-6,
    ] {
        for _ in 0..4000 {
            let (lon, gamma) = (TAU * mix.next() - PI, TAU * mix.next());
            let (north, scale) = (mix.next() < 0.5, 0.4 + 0.4 * mix.next());
            for (grid, f32_row) in [(GRID32, true), (GRID64, false)] {
                let n = configuration(lon, target, north, gamma, scale, grid);
                let (lon_exact, rho) = exact(n);
                let Some(row) = rows.iter_mut().find(|r| rho >= r.lo) else {
                    continue;
                };
                if f32_row {
                    let e = principal(measured::<f32>(n, grid, map32) - lon_exact).abs();
                    row.count32 += 1;
                    row.max32 = row.max32.max(e);
                    row.c32 = row.c32.max(e * rho / f64::from(f32::EPSILON));
                } else {
                    let e = principal(measured::<f64>(n, grid, map64) - lon_exact).abs();
                    row.count64 += 1;
                    row.max64 = row.max64.max(e);
                    row.c64 = row.c64.max(e * rho / f64::EPSILON);
                }
            }
        }
    }
    rows
}

/// The evidence's claims, at both precisions: in every decade of ρ holding samples, the error is at most `4·ε/ρ`; and
/// in every decade below 10⁻², at least `ε/(4ρ)`, so the measurement sees the round-off it bounds and grows as 1/ρ
/// (pitfalls §3, "check the measurement can fire"). The table is the evidence REQ-INT-086 asks for.
fn check_evidence(rows: &[Row]) {
    eprintln!("ρ decade        n f32   max err f32   C f32     n f64   max err f64   C f64   (err ≤ C·ε/ρ)");
    for r in rows {
        eprintln!(
            "[{:.0e}, {:.0e})  {:>7}   {:>11.3e}   {:>5.2}   {:>7}   {:>11.3e}   {:>5.2}",
            r.lo,
            r.lo * 10.0,
            r.count32,
            r.max32,
            r.c32,
            r.count64,
            r.max64,
            r.c64
        );
    }
    for r in rows {
        for (name, count, c) in [("f32", r.count32, r.c32), ("f64", r.count64, r.c64)] {
            if count == 0 {
                continue;
            }
            assert!(
                c <= 4.0,
                "the {name} longitude's error in ρ ∈ [{:.0e}, {:.0e}) is past 4·ε/ρ: C = {c}",
                r.lo,
                r.lo * 10.0
            );
            assert!(
                r.lo >= 1e-2 || c >= 0.25,
                "the {name} longitude's error in ρ ∈ [{:.0e}, {:.0e}) is below ε/(4ρ), C = {c}: the measurement does \
                 not see the round-off",
                r.lo,
                r.lo * 10.0
            );
        }
    }
    for (name, decades) in [
        ("f32", rows.iter().filter(|r| r.count32 > 0).count()),
        ("f64", rows.iter().filter(|r| r.count64 > 0).count()),
    ] {
        assert!(
            decades >= 6,
            "the {name} measurement covers {decades} decades of ρ, not six"
        );
    }
}

#[test]
fn theta_unwrap_r_pole_evidence() {
    check_evidence(&measure(shape::<f32>, shape::<f64>));
}

/// The shape map weighted with the wrong masses, equal thirds, whose `μ_λ/μ_ρ` is 4/3, not 2.
#[cfg(feature = "controls")]
fn unweighted<R: Real + Float>(r: [[R; 2]; 3], _m: [R; 3]) -> [R; 3] {
    let one = R::one();
    let third = one / (one + one + one);
    shape(r, [third, third, third])
}

negative_control!(
    theta_unwrap_r_pole_evidence,
    "a shape map with the wrong masses must fail the round-off bound",
    expected = "is past 4·ε/ρ",
    check_evidence(&measure(unweighted::<f32>, unweighted::<f64>))
);

// ── The shape map's landmarks (R-14) ────────────────────────────────────────────────────────────────────────────────

/// The landmarks for equal masses: the collision of 0 and 1 at (−1, 0, 0), of 1 and 2 at (1/2, √3/2, 0), of 2 and 0
/// at (1/2, −√3/2, 0); `L⁺`, 0 → 1 → 2 anticlockwise, at (0, 0, 1), `L⁻` at (0, 0, −1); a rotated, translated and
/// scaled configuration at the same `n`.
fn check_landmarks(map: fn([[f64; 2]; 3], [f64; 3]) -> [f64; 3]) {
    let m = [1.0 / 3.0; 3];
    let h = 3f64.sqrt() / 2.0;
    let cases = [
        (
            "BC01",
            [[0.3, 0.2], [0.3, 0.2], [1.0, -0.4]],
            [-1.0, 0.0, 0.0],
        ),
        ("BC12", [[0.0, 0.0], [1.0, 0.0], [1.0, 0.0]], [0.5, h, 0.0]),
        ("BC20", [[0.0, 0.0], [1.0, 0.0], [0.0, 0.0]], [0.5, -h, 0.0]),
        ("L+", [[0.0, 0.0], [1.0, 0.0], [0.5, h]], [0.0, 0.0, 1.0]),
        ("L-", [[0.0, 0.0], [1.0, 0.0], [0.5, -h]], [0.0, 0.0, -1.0]),
    ];
    for (name, r, want) in cases {
        let n = map(r, m);
        for c in 0..3 {
            assert!(
                (n[c] - want[c]).abs() < 1e-12,
                "{name} is at {n:?}, not {want:?}"
            );
        }
    }
    let r = [[0.1, -0.7], [0.9, 0.2], [-0.3, 0.5]];
    let m = [0.2, 0.5, 0.3];
    let n = map(r, m);
    let norm = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    assert!((norm - 1.0).abs() < 1e-12, "|n| is {norm}, not 1");
    let (c, s, k) = (0.6f64.cos(), 0.6f64.sin(), 2.5);
    let moved = r.map(|p| {
        [
            k * (c * p[0] - s * p[1]) + 3.0,
            k * (s * p[0] + c * p[1]) - 1.0,
        ]
    });
    let n2 = map(moved, m);
    for c in 0..3 {
        assert!(
            (n[c] - n2[c]).abs() < 1e-12,
            "a rotated, translated, scaled configuration moves n from {n:?} to {n2:?}"
        );
    }
}

#[test]
fn shape_landmarks() {
    check_landmarks(shape::<f64>);
    // The unnormalised point is n scaled by I: by its own norm, it is the same landmarks (with shape's |n| = 1, its
    // norm is I).
    check_landmarks(|r, m| {
        let p = shape_unnormalised(r, m);
        let norm = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
        p.map(|x| x / norm)
    });
}

/// The shape map with `w` negated: the earlier convention, which put `L⁺` at the opposite pole (R-14).
#[cfg(feature = "controls")]
fn negated_cross(r: [[f64; 2]; 3], m: [f64; 3]) -> [f64; 3] {
    let n = shape(r, m);
    [n[0], n[1], -n[2]]
}

negative_control!(
    shape_landmarks,
    "the shape map with the cross negated must put L+ at the south pole",
    expected = "L+ is at",
    check_landmarks(negated_cross)
);
