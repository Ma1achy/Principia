//! QA tests for TASK-M1-11, written from the requirements it closes, not from the code:
//! - REQ-INT-001 (dd_integrator §3.7; R-389; R-392): `θ̃` is a running accumulator adding the principal-value delta
//!   of `atan2(n_v, n_u)` each step, `θ̃(0) = 0` whatever `n(0)`'s longitude; below the pole radius it holds with a
//!   frozen reference (the last longitude outside), adding `wrap(exit − stored)` into (−π, π] on exit; a difference of
//!   exactly ±π, per step or on exit, adds +π; an IC inside the disc adds nothing at its first exit and counts from the
//!   exit longitude; `orbit_count = ⌊|θ̃|/2π⌋` and `retrograde = θ̃ < 0` derived at read (payload §5).
//! - The shape map `n` (dd_integrator §3.7; chart_reference §0.2, §3.1; R-14), checked against chart_reference §0.2's
//!   canonical-frame decode, whose `n` is `(cos 2α, sin 2α cos β, sin 2α sin β)` by §3.1's forward map, and the
//!   landmarks §3.7 lists.
//! - REQ-TOOL-013 (R-41, R-75): the kernel's debug variants are exactly the bring-up variant, a baked variant selected
//!   by type that reads nothing but the sample's index; its output decodes through the normal unpack path to
//!   colour_composition Appendix A's pattern (REQ-TOOL-123), written here from Appendix A's table.
//!
//! Every test whose name holds `theta_unwrap` or `debug_variants` runs under the task's acceptance commands. `r_pole`
//! is REQ-INT-086's calibration, unconfirmed until the M1 gate (R-71): the rule's tests run at several radii, the
//! proposal among them, since R-389's rule holds whatever its value. Each test has its negative control (R-176).
// The file name `qa_TASK-M1-11` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::f64::consts::{FRAC_PI_2, PI, TAU};

use kernel::bringup::{pattern, write_words, BringUp, Variant, VARIANTS};
use kernel::payload::{
    canonical_nan, orbit_count, retrograde, sim_state_from_ftle, PayloadReal, ReadParams,
    SimStateFTLE, SimStateFTLEOf, FGW_UNBOUND,
};
use kernel::shape::{shape, theta_step, Theta};
use proptest::prelude::*;
use validation::negative_control;

// ----- θ̃: the reference written from the requirement -----

/// One macro-step of `θ̃` at f64: `kernel::shape::theta_step`, or a control's faulty one.
type Step = fn(Theta<f64>, [f64; 3], [f64; 3], f64) -> Theta<f64>;

/// REQ-INT-086's proposed `r_pole` (unconfirmed, R-71), and two radii either side of it: R-389's rule is stated for
/// any radius, so each passage test runs at each.
const RADII: [f64; 3] = [1e-3, 1e-2, 0.1];

/// R-389's wrap, written from its words: `d − 2πk` in (−π, π], closed at +π, open at −π.
fn principal(d: f64) -> f64 {
    let mut x = d;
    while x > PI {
        x -= TAU;
    }
    while x <= -PI {
        x += TAU;
    }
    x
}

/// The equatorial longitude `atan2(n_v, n_u)` (dd_integrator §3.7).
fn lon(n: [f64; 3]) -> f64 {
    n[1].atan2(n[0])
}

/// The polar radius `√(n_u² + n_v²)` (R-389).
fn rho(n: [f64; 3]) -> f64 {
    (n[0] * n[0] + n[1] * n[1]).sqrt()
}

/// The point on the sphere at longitude `l` and polar radius `r`, in the north (`w > 0`) or south hemisphere.
fn pt(l: f64, r: f64, north: bool) -> [f64; 3] {
    let w = (1.0 - r * r).max(0.0).sqrt();
    [r * l.cos(), r * l.sin(), if north { w } else { -w }]
}

/// `θ̃` after each point of `path` (`path[0]` the IC, `θ̃(0)` first), by `step` with pole radius `r_pole`.
fn run(step: Step, path: &[[f64; 3]], r_pole: f64) -> Vec<f64> {
    let mut t = Theta::start();
    let mut out = vec![t.theta];
    for w in path.windows(2) {
        t = step(t, w[0], w[1], r_pole);
        out.push(t.theta);
    }
    out
}

/// `θ̃` after each point of `path`, by R-389's and R-392's rule as the ruling words it: start at 0; a point inside
/// `ρ < r_pole` adds nothing, and on entering, the last longitude outside is stored; a point outside adds the wrapped
/// difference from the last trusted longitude (the previous point's, or the stored one on exit); with no trusted
/// longitude yet (an IC inside, at its first exit), nothing.
fn expected(path: &[[f64; 3]], r_pole: f64) -> Vec<f64> {
    let mut theta = 0.0;
    let mut trusted: Option<f64> = (rho(path[0]) >= r_pole).then(|| lon(path[0]));
    let mut out = vec![theta];
    for &n in &path[1..] {
        if rho(n) >= r_pole {
            if let Some(reference) = trusted {
                theta += principal(lon(n) - reference);
            }
            trusted = Some(lon(n));
        }
        out.push(theta);
    }
    out
}

/// Round-off bound on a `θ̃` of at most 10⁴ f64 steps of at most π each: 10⁴ · 4π · 2⁻⁵² ≈ 3·10⁻¹¹, rounded up; any 2π
/// jump, a missed or doubled delta, or a delta from the wrong reference is at least 10⁻³ on these paths.
const TOL64: f64 = 1e-9;

/// `θ̃` from `step` along `path` agrees with the rule, point by point, at `r_pole`.
fn check_path(step: Step, path: &[[f64; 3]], r_pole: f64, what: &str) {
    let got = run(step, path, r_pole);
    let want = expected(path, r_pole);
    for (k, (g, w)) in got.iter().zip(&want).enumerate() {
        assert!(
            (g - w).abs() <= TOL64,
            "{what}: θ̃ after point {k} is {g}, the rule's {w} (r_pole {r_pole})"
        );
    }
}

/// A path circulating the `w` axis: `steps` steps from longitude `l0`, a total turning `total` (signed), its polar
/// radius wobbling about `r0` by `wobble` (never into the disc), in the north or south hemisphere.
fn circulate(
    l0: f64,
    total: f64,
    steps: usize,
    r0: f64,
    wobble: f64,
    north: bool,
) -> Vec<[f64; 3]> {
    (0..=steps)
        .map(|k| {
            let s = k as f64 / steps as f64;
            pt(l0 + total * s, r0 + wobble * (7.0 * TAU * s).sin(), north)
        })
        .collect()
}

/// On a circulating path: `θ̃` follows the true turning with no 2π jump; at the end it is the total turning, and the
/// read side's `orbit_count` and `retrograde` (payload §5), on `θ̃` stored as the f32 payload's `theta`, give the
/// path's completed turns and its direction; the hold never fires (the same `θ̃` with no disc at all).
fn check_circulating(step: Step, path: &[[f64; 3]], total: f64, r_pole: f64) {
    let got = run(step, path, r_pole);
    let n = path.len() - 1;
    for (k, g) in got.iter().enumerate() {
        let truth = total * k as f64 / n as f64;
        assert!(
            (g - truth).abs() <= TOL64,
            "θ̃ after point {k} is {g}, the path's turning {truth}: a 2π jump or a lost delta"
        );
    }
    let theta = *got.last().unwrap();
    let no_disc = run(step, path, 0.0);
    assert_eq!(
        got, no_disc,
        "the hold fired on a path circulating away from the poles (r_pole {r_pole})"
    );
    let turns = (total.abs() / TAU).floor() as u32;
    assert_eq!(
        orbit_count(theta as f32),
        turns,
        "orbit_count of θ̃ = {theta} is not the path's {turns} completed turns"
    );
    assert_eq!(
        retrograde(theta as f32),
        total < 0.0,
        "retrograde of θ̃ = {theta} is not the path's direction (turning {total})"
    );
}

#[test]
fn qa_theta_unwrap_circulating_paths() {
    // Fixed cases: both directions, starting on and either side of the ±π cut, north and south, whole and part turns.
    for &(l0, total, north) in &[
        (PI, 3.0 * TAU + 1.0, true),
        (-PI + 1e-9, -(3.0 * TAU + 1.0), true),
        (PI - 1e-9, 2.0 * TAU + 0.5, false),
        (0.0, -(5.0 * TAU + 3.0), false),
        (2.0, 0.75 * TAU, true),
        (-2.5, -0.4 * TAU, true),
    ] {
        for r_pole in RADII {
            let path = circulate(l0, total, 2000, 0.5, 0.3, north);
            check_circulating(theta_step, &path, total, r_pole);
        }
    }
}

#[test]
fn qa_theta_unwrap_circulating_property() {
    check_circulating_property(theta_step);
}

/// `check_circulating` over random circulating paths, stepped by `step`.
fn check_circulating_property(step: Step) {
    // Any start, direction, turn count (fraction kept away from a whole turn, where f32 rounding of θ̃ would decide
    // orbit_count), step count with every step's turning below π, and latitude away from the pole disc.
    let strategy = (
        -PI..=PI,
        prop_oneof![Just(1.0), Just(-1.0)],
        0u32..6,
        0.05f64..0.95,
        16usize..400,
        0.15f64..0.95,
        any::<bool>(),
    );
    validation::prop::run(&strategy, |(l0, dir, k, f, per_turn, r0, north)| {
        let total = dir * TAU * (f64::from(k) + f);
        let steps = ((f64::from(k) + f) * per_turn as f64).ceil() as usize + 1;
        let wobble = (r0 - 0.12).min(0.99 - r0).min(0.1);
        let path = circulate(l0, total, steps, r0, wobble, north);
        check_circulating(step, &path, total, 1e-2);
        Ok(())
    });
}

/// A step that adds the raw longitude difference, never wrapped: its first crossing of the ±π cut jumps by 2π.
#[cfg(feature = "controls")]
fn unwrapped_step(t: Theta<f64>, a: [f64; 3], b: [f64; 3], _r: f64) -> Theta<f64> {
    Theta {
        theta: t.theta + (lon(b) - lon(a)),
        ..t
    }
}

negative_control!(
    qa_theta_unwrap_circulating_paths,
    "a step that does not wrap the longitude difference must fail the circulating path",
    expected = "a 2π jump or a lost delta",
    check_circulating(
        unwrapped_step,
        &circulate(PI, 3.0 * TAU + 1.0, 2000, 0.5, 0.3, true),
        3.0 * TAU + 1.0,
        1e-2
    )
);

/// A step whose disc swallows the whole sphere's equatorial band below polar radius 0.6: on a path at polar radius
/// 0.2–0.8 it holds where it must not.
#[cfg(feature = "controls")]
fn wide_disc_step(t: Theta<f64>, a: [f64; 3], b: [f64; 3], r: f64) -> Theta<f64> {
    theta_step(t, a, b, if r > 0.0 { 0.6 } else { r })
}

#[cfg(feature = "controls")]
mod circulating_hold_control {
    use super::*;

    mod property {
        use super::*;

        negative_control!(
            qa_theta_unwrap_circulating_property,
            "a step that does not wrap the longitude difference must fail the random circulating paths",
            expected = "a 2π jump or a lost delta",
            check_circulating_property(unwrapped_step)
        );
    }

    negative_control!(
        qa_theta_unwrap_circulating_paths,
        "a hold that fires away from the poles must fail the circulating path",
        expected = "θ̃ after point",
        check_circulating(
            wide_disc_step,
            &circulate(0.0, 2.0 * TAU + 0.5, 2000, 0.5, 0.3, true),
            2.0 * TAU + 0.5,
            1e-2
        )
    );
}

// ----- θ̃(0) = 0 (R-389) -----

/// From each starting longitude, `θ̃(0) = 0`, and one step east by 0.25 rad gives `θ̃ = 0.25`: the start's longitude
/// is never counted. `start` builds the state from `n(0)`.
fn check_start(start: fn([f64; 3]) -> Theta<f64>) {
    for k in 0..64 {
        let l0 = -PI + TAU * (k as f64 + 0.5) / 64.0;
        let n0 = pt(l0, 0.7, k % 2 == 0);
        let t0 = start(n0);
        assert!(
            t0.theta == 0.0,
            "θ̃(0) is {} at starting longitude {l0}, not 0",
            t0.theta
        );
        let t1 = theta_step(t0, n0, pt(l0 + 0.25, 0.7, k % 2 == 0), 1e-2);
        assert!(
            (t1.theta - 0.25).abs() <= TOL64,
            "θ̃(0) is {} at starting longitude {l0}, not 0: a step of 0.25 gives {}",
            t0.theta,
            t1.theta
        );
    }
}

#[test]
fn qa_theta_unwrap_starts_at_zero() {
    check_start(|_| Theta::start());
    assert_eq!(Theta::<f32>::start().theta, 0.0, "θ̃(0) at f32 is not 0");
}

negative_control!(
    qa_theta_unwrap_starts_at_zero,
    "a start at n(0)'s longitude must fail θ̃(0) = 0",
    expected = "θ̃(0) is",
    check_start(|n| Theta {
        theta: lon(n),
        ..Theta::start()
    })
);

// ----- The pole passage (R-389) -----

/// A pole passage: a spiral in from polar radius 0.3 at `l_in` (the longitude turning as it closes, so the last point
/// outside is not at the spiral's earlier longitudes), first point inside at another longitude, a wander inside
/// through every longitude at radii below `r_pole`, out at `l_out` (first point outside at exactly that longitude),
/// and a spiral out. The path, the last point outside before, and the first point outside after.
fn passage(l_in: f64, l_out: f64, r_pole: f64, north: bool) -> (Vec<[f64; 3]>, usize, usize) {
    let mut path = Vec::new();
    for k in 0..40 {
        let s = k as f64 / 40.0;
        let r = 0.3 + (1.5 * r_pole - 0.3) * s;
        path.push(pt(l_in - 0.6 * (1.0 - s), r, north));
    }
    path.push(pt(l_in, 1.01 * r_pole, north));
    let last_out = path.len() - 1;
    for k in 0..30 {
        path.push(pt(
            l_in + 1.7 + 0.9 * k as f64,
            r_pole * (0.9 - 0.85 * (k as f64 / 30.0).sin()),
            north,
        ));
    }
    path.push(pt(l_out, r_pole * 1.02, north));
    let first_out = path.len() - 1;
    for k in 1..40 {
        let s = k as f64 / 40.0;
        let r = 1.02 * r_pole + (0.3 - 1.02 * r_pole) * s;
        path.push(pt(l_out + 0.5 * s, r, north));
    }
    (path, last_out, first_out)
}

/// Along a passage from `l_in` to `l_out`: `θ̃` is held, bit for bit, from the last point outside through every point
/// inside, adds exactly `wrap(longitude(first out) − longitude(last out))` on exit, and then follows the rule.
fn check_passage(step: Step, l_in: f64, l_out: f64, r_pole: f64, north: bool) {
    let (path, last_out, first_out) = passage(l_in, l_out, r_pole, north);
    let got = run(step, &path, r_pole);
    for k in last_out + 1..first_out {
        assert!(
            got[k].to_bits() == got[last_out].to_bits(),
            "inside the disc θ̃ moved: {} at point {k}, {} on entry (l_in {l_in}, l_out {l_out}, r_pole {r_pole})",
            got[k],
            got[last_out]
        );
    }
    let want = principal(lon(path[first_out]) - lon(path[last_out]));
    let added = got[first_out] - got[last_out];
    assert!(
        (added - want).abs() <= TOL64,
        "the exit added {added}, not wrap(exit − stored) = {want} (l_in {l_in}, l_out {l_out}, r_pole {r_pole})"
    );
    check_path(step, &path, r_pole, "pole passage");
}

#[test]
fn qa_theta_unwrap_pole_passage() {
    // Exits on either side of the stored longitude, across the ±π cut, and with raw differences past ±π, where only
    // the wrap gives (−π, π]: 3 → −3 wraps −6 to 2π − 6; 2 → −1.5 wraps −3.5 to 2π − 3.5; −2.9 → 2.9 wraps 5.8.
    let cases = [
        (0.5, 2.5),
        (2.5, 0.5),
        (3.0, -3.0),
        (-3.0, 3.0),
        (2.0, -1.5),
        (-2.9, 2.9),
        (1.0, 1.0),
        (-0.3, 2.6),
    ];
    for r_pole in RADII {
        for (l_in, l_out) in cases {
            for north in [true, false] {
                check_passage(theta_step, l_in, l_out, r_pole, north);
            }
        }
    }
}

#[test]
fn qa_theta_unwrap_pole_passage_property() {
    check_passage_property(theta_step);
}

/// `check_passage` over random entry and exit longitudes, radii and hemispheres, stepped by `step`.
fn check_passage_property(step: Step) {
    let strategy = (-PI..PI, -PI..PI, 1e-4f64..0.2, any::<bool>());
    validation::prop::run(&strategy, |(l_in, l_out, r_pole, north)| {
        check_passage(step, l_in, l_out, r_pole, north);
        Ok(())
    });
}

#[test]
fn qa_theta_unwrap_run_ending_inside() {
    check_ending_inside(theta_step);
}

/// A run that terminates inside the disc adds nothing for that passage (dd_integrator §3.7): θ̃ stays at its value on
/// entry.
fn check_ending_inside(step: Step) {
    for r_pole in RADII {
        let (path, last_out, first_out) = passage(2.0, -2.0, r_pole, true);
        let got = run(step, &path[..first_out], r_pole);
        assert_eq!(
            got.last().unwrap().to_bits(),
            got[last_out].to_bits(),
            "a run ending inside the disc moved θ̃"
        );
    }
}

/// A step with no hold: it differences every pair of consecutive points, the disc's untrusted ones included.
#[cfg(feature = "controls")]
fn unheld(t: Theta<f64>, a: [f64; 3], b: [f64; 3], _r: f64) -> Theta<f64> {
    theta_step(t, a, b, 0.0)
}

/// A step that stores the first longitude inside the disc on entering, not the last one outside.
#[cfg(feature = "controls")]
fn stores_first_inside(t: Theta<f64>, a: [f64; 3], b: [f64; 3], r: f64) -> Theta<f64> {
    let next = theta_step(t, a, b, r);
    if rho(a) >= r && rho(b) < r {
        return Theta {
            frozen: lon(b),
            has_frozen: true,
            ..next
        };
    }
    next
}

/// A step whose exit difference is not wrapped.
#[cfg(feature = "controls")]
fn unwrapped_exit(t: Theta<f64>, a: [f64; 3], b: [f64; 3], r: f64) -> Theta<f64> {
    if rho(a) < r && rho(b) >= r && t.has_frozen {
        return Theta {
            theta: t.theta + (lon(b) - t.frozen),
            ..t
        };
    }
    theta_step(t, a, b, r)
}

negative_control!(
    qa_theta_unwrap_pole_passage,
    "a step that does not hold inside the disc must fail the pole passage",
    expected = "inside the disc θ̃ moved",
    check_passage(unheld, 0.5, 2.5, 1e-2, true)
);

#[cfg(feature = "controls")]
mod passage_controls {
    use super::*;

    mod property {
        use super::*;

        negative_control!(
            qa_theta_unwrap_pole_passage_property,
            "a step that does not hold inside the disc must fail the random passages",
            expected = "inside the disc θ̃ moved",
            check_passage_property(unheld)
        );
    }

    mod ending_inside {
        use super::*;

        negative_control!(
            qa_theta_unwrap_run_ending_inside,
            "a step that does not hold inside the disc must fail a run ending there",
            expected = "a run ending inside the disc moved θ̃",
            check_ending_inside(unheld)
        );
    }

    negative_control!(
        qa_theta_unwrap_pole_passage,
        "storing the first longitude inside, not the last outside, must fail the exit",
        expected = "not wrap(exit − stored)",
        check_passage(stores_first_inside, 0.5, 2.5, 1e-2, true)
    );

    mod unwrapped {
        use super::*;

        negative_control!(
            qa_theta_unwrap_pole_passage,
            "an exit difference not wrapped into (−π, π] must fail",
            expected = "not wrap(exit − stored)",
            check_passage(unwrapped_exit, 3.0, -3.0, 1e-2, true)
        );
    }
}

// ----- The exact-π case (R-389) -----

/// Each step, and each pole passage, whose longitude difference is exactly ±π at f64 adds exactly +π.
fn check_exact_pi(step: Step) {
    // Per step. atan2(0, −1) = π and atan2(−0, −1) = −π exactly; atan2(±1, 0) = ±π/2, whose difference is π exactly.
    let east = [1.0, 0.0, 0.0];
    let west_up = [-1.0, 0.0, 0.0];
    let west_down = [-1.0, -0.0, 0.0];
    let north_v = [0.0, 1.0, 0.0];
    let south_v = [0.0, -1.0, 0.0];
    let pairs = [
        (east, west_up),
        (west_up, east),
        (east, west_down),
        (west_down, east),
        (south_v, north_v),
        (north_v, south_v),
    ];
    for (a, b) in pairs {
        assert_eq!(
            (lon(b) - lon(a)).abs(),
            PI,
            "the test's pair is not exactly π apart"
        );
        let t = step(Theta::start(), a, b, 1e-2);
        assert!(
            t.theta == PI,
            "a step from {a:?} to {b:?}, longitude difference {}, adds {}, not +π",
            lon(b) - lon(a),
            t.theta
        );
    }
    // A pole passage: stored and exit longitudes exactly π apart, at polar radius 1/4 (exact), outside a disc of
    // radius 1/8; inside it, the pole itself and a point at another longitude.
    let r: f64 = 0.25;
    let r_pole = 0.125;
    let w = (1.0 - r * r).sqrt();
    let e = [r, 0.0, w];
    let wu = [-r, 0.0, w];
    let wd = [-r, -0.0, w];
    let pole = [0.0, 0.0, 1.0];
    for (enter, exit) in [(e, wu), (wu, e), (e, wd), (wd, e)] {
        let inside = [0.05, -0.03, w];
        assert!(
            rho(inside) < r_pole && rho(enter) >= r_pole,
            "the test's passage is not through the disc"
        );
        let t = step(Theta::start(), enter, pole, r_pole);
        let t = step(t, pole, inside, r_pole);
        let t = step(t, inside, exit, r_pole);
        assert!(
            t.theta == PI,
            "a passage stored at {} and leaving at {} adds {}, not +π",
            lon(enter),
            lon(exit),
            t.theta
        );
    }
}

#[test]
fn qa_theta_unwrap_exact_pi() {
    check_exact_pi(theta_step);
    // At f32 the step's own arithmetic: ±π/2 are exact at f32 too, and their difference is f32's π, which adds +π.
    let s = theta_step::<f32>(Theta::start(), [0.0, 1.0, 0.0], [0.0, -1.0, 0.0], 1e-2);
    assert_eq!(
        s.theta,
        std::f32::consts::PI,
        "f32: a step of −π adds {}",
        s.theta
    );
    let s = theta_step::<f32>(Theta::start(), [0.0, -1.0, 0.0], [0.0, 1.0, 0.0], 1e-2);
    assert_eq!(
        s.theta,
        std::f32::consts::PI,
        "f32: a step of +π adds {}",
        s.theta
    );
    // The f32 longitude of the negative u axis, for the review (atan2f's rounding of π).
    eprintln!(
        "f32 atan2(+0, −1) = {:e}, f32 π = {:e}",
        0.0f32.atan2(-1.0),
        std::f32::consts::PI
    );
}

/// A step that wraps into [−π, π): an exact ±π adds −π.
#[cfg(feature = "controls")]
fn low_wrap(t: Theta<f64>, a: [f64; 3], b: [f64; 3], r: f64) -> Theta<f64> {
    let next = theta_step(t, a, b, r);
    if next.theta - t.theta == PI {
        return Theta {
            theta: t.theta - PI,
            ..next
        };
    }
    next
}

/// A step that adds `d` itself for an exact ±π, so −π stays −π.
#[cfg(feature = "controls")]
fn signed_pi(t: Theta<f64>, a: [f64; 3], b: [f64; 3], r: f64) -> Theta<f64> {
    let next = theta_step(t, a, b, r);
    if rho(b) >= r && rho(a) >= r && lon(b) - lon(a) == -PI {
        return Theta {
            theta: t.theta - PI,
            ..next
        };
    }
    next
}

negative_control!(
    qa_theta_unwrap_exact_pi,
    "a wrap into [−π, π) must fail the exact-π case",
    expected = "not +π",
    check_exact_pi(low_wrap)
);

#[cfg(feature = "controls")]
mod exact_pi_control {
    use super::*;

    negative_control!(
        qa_theta_unwrap_exact_pi,
        "a step keeping −π as −π must fail the exact-π case",
        expected = "not +π",
        check_exact_pi(signed_pi)
    );
}

// ----- The disc's edge -----

/// `ρ = r_pole` exactly is outside (the disc is `ρ < r_pole`, R-389): a step to it adds its delta.
fn check_edge(step: Step) {
    let r_pole = 0.5;
    let w = (0.75f64).sqrt();
    let a = pt(0.3, 0.6, true);
    let on_edge = [0.5, 0.0, w];
    assert_eq!(rho(on_edge), r_pole, "the test's point is not on the edge");
    let t = step(Theta::start(), a, on_edge, r_pole);
    assert!(
        (t.theta - (-0.3)).abs() <= TOL64,
        "a step to ρ = r_pole adds {}, not its delta −0.3: the edge is outside",
        t.theta
    );
}

#[test]
fn qa_theta_unwrap_disc_edge() {
    check_edge(theta_step);
}

negative_control!(
    qa_theta_unwrap_disc_edge,
    "a closed disc, ρ ≤ r_pole, must fail the edge",
    expected = "the edge is outside",
    check_edge(|t, a, b, r| theta_step(t, a, b, r * (1.0 + 1e-12)))
);

// ----- An IC inside the disc (R-392) -----

/// An IC inside the disc at longitude `l_ic` (or on the pole itself), a wander inside, a first exit at `l_exit`, one
/// and a quarter turns east outside, and a second passage: `θ̃` is 0 through the first exit, then follows the rule
/// (counting from the exit longitude), the second passage adding `wrap(exit − stored)`.
fn check_ic_inside(step: Step, l_ic: f64, l_exit: f64, on_pole: bool, r_pole: f64) {
    let mut path = vec![if on_pole {
        [0.0, 0.0, 1.0]
    } else {
        pt(l_ic, 0.5 * r_pole, true)
    }];
    for k in 0..20 {
        path.push(pt(l_ic + 2.1 * k as f64, r_pole * 0.6, true));
    }
    path.push(pt(l_exit, 1.05 * r_pole, true));
    let first_exit = path.len() - 1;
    for k in 1..=500 {
        let s = k as f64 / 500.0;
        path.push(pt(
            l_exit + 1.25 * TAU * s,
            1.05 * r_pole + (0.4 - 1.05 * r_pole) * (PI * s).sin(),
            true,
        ));
    }
    let tail = passage(lon(*path.last().unwrap()), l_exit - 2.0, r_pole, true).0;
    path.extend(tail);
    let got = run(step, &path, r_pole);
    for (k, g) in got.iter().enumerate().take(first_exit + 1) {
        assert!(
            *g == 0.0,
            "an IC inside the disc: θ̃ is {g} at point {k}, at or before its first exit, not 0"
        );
    }
    let after = got[first_exit + 500];
    assert!(
        (after - 1.25 * TAU).abs() <= TOL64,
        "an IC inside the disc: θ̃ is {after} after 1.25 turns from the exit, not counted from the exit longitude"
    );
    check_path(step, &path, r_pole, "IC inside the disc");
}

#[test]
fn qa_theta_unwrap_ic_inside() {
    for r_pole in RADII {
        for (l_ic, l_exit) in [(2.0, -2.0), (-1.0, 1.0), (3.1, -3.1), (0.0, PI)] {
            for on_pole in [false, true] {
                check_ic_inside(theta_step, l_ic, l_exit, on_pole, r_pole);
            }
        }
    }
}

/// A step that, at an IC-inside first exit, adds the wrapped difference from the IC's longitude (RQ-225's option 2,
/// which R-392 rejects).
#[cfg(feature = "controls")]
fn from_ic(t: Theta<f64>, a: [f64; 3], b: [f64; 3], r: f64) -> Theta<f64> {
    if !t.has_frozen && rho(a) < r && rho(b) >= r && t.theta == 0.0 {
        return Theta {
            theta: principal(lon(b) - 2.0),
            ..t
        };
    }
    theta_step(t, a, b, r)
}

negative_control!(
    qa_theta_unwrap_ic_inside,
    "a first exit that differences against the IC's longitude must fail R-392",
    expected = "not 0",
    check_ic_inside(from_ic, 2.0, -2.0, false, 1e-2)
);

// ----- Derived at read (payload §5) -----

/// `orbit_count = ⌊|θ̃|/2π⌋` and `retrograde = θ̃ < 0`, through the generated unpack of a `SimStateFTLE` holding `θ̃`.
fn check_derived(read: fn(f32) -> (u32, bool)) {
    let cases: [(f32, u32, bool); 8] = [
        (0.0, 0, false),
        (1.0, 0, false),
        (-1.0, 0, true),
        (6.2, 0, false),
        (6.3, 1, false),
        (-6.3, 1, true),
        ((3.0 * TAU + 0.5) as f32, 3, false),
        (-(7.0 * TAU + 2.0) as f32, 7, true),
    ];
    for (theta, count, retro) in cases {
        assert_eq!(
            read(theta),
            (count, retro),
            "θ̃ = {theta}: (orbit_count, retrograde) is not ({count}, {retro})"
        );
    }
}

/// `θ̃` stored in a fresh `SimStateFTLE` and read through `sim_state_from_ftle`.
fn read_through_unpack(theta: f32) -> (u32, bool) {
    let s = SimStateFTLE {
        theta,
        ..SimStateFTLE::default()
    };
    let params = ReadParams {
        dt_macro: 1.0,
        delta_0: 1.0,
        n_renorm: 0,
        horizon_steps: 1,
    };
    let read = sim_state_from_ftle(
        &s,
        FGW_UNBOUND,
        false,
        canonical_nan(),
        false,
        [1.0 / 3.0; 3],
        &params,
    );
    (read.orbit_count, read.retrograde)
}

#[test]
fn qa_theta_unwrap_derived_at_read() {
    check_derived(read_through_unpack);
    // End to end: 2.6 turns clockwise, θ̃ accumulated at f64, stored at f32, read: 2 turns, retrograde.
    let path = circulate(1.0, -2.6 * TAU, 1200, 0.6, 0.2, true);
    let theta = *run(theta_step, &path, 1e-2).last().unwrap();
    assert_eq!(
        read_through_unpack(theta as f32),
        (2, true),
        "2.6 turns clockwise read wrongly"
    );
}

negative_control!(
    qa_theta_unwrap_derived_at_read,
    "a retrograde read as θ̃ > 0 must fail",
    expected = "(orbit_count, retrograde) is not",
    check_derived(|t| {
        let (c, r) = read_through_unpack(t);
        (c, !r)
    })
);

// ----- The shape map (dd_integrator §3.7; chart_reference §0.2, §3.1; R-14) -----

/// chart_reference §0.2's canonical-frame decode at `(α, β)` with masses `m` (summing to 1), then rotated by `g`,
/// scaled by `sc` and translated by `t`: the bodies' positions.
fn decode(alpha: f64, beta: f64, m: [f64; 3], g: f64, sc: f64, t: [f64; 2]) -> [[f64; 2]; 3] {
    let m01 = m[0] + m[1];
    let mu_rho = m[0] * m[1] / m01;
    let mu_lambda = m[2] * m01;
    let rt = [alpha.cos(), 0.0];
    let lt = [alpha.sin() * beta.cos(), alpha.sin() * beta.sin()];
    let rho_v = [rt[0] / mu_rho.sqrt(), rt[1] / mu_rho.sqrt()];
    let lam = [lt[0] / mu_lambda.sqrt(), lt[1] / mu_lambda.sqrt()];
    let r01 = [-m[2] * lam[0], -m[2] * lam[1]];
    let r2 = [m01 * lam[0], m01 * lam[1]];
    let r0 = [
        r01[0] - m[1] / m01 * rho_v[0],
        r01[1] - m[1] / m01 * rho_v[1],
    ];
    let r1 = [
        r01[0] + m[0] / m01 * rho_v[0],
        r01[1] + m[0] / m01 * rho_v[1],
    ];
    let place = |p: [f64; 2]| {
        [
            sc * (g.cos() * p[0] - g.sin() * p[1]) + t[0],
            sc * (g.sin() * p[0] + g.cos() * p[1]) + t[1],
        ]
    };
    [place(r0), place(r1), place(r2)]
}

/// `‖a − b‖∞ ≤ tol`, else a panic naming `what`.
fn close(a: [f64; 3], b: [f64; 3], tol: f64, what: &str) {
    let d = (0..3).map(|k| (a[k] - b[k]).abs()).fold(0.0, f64::max);
    assert!(d <= tol, "{what}: n = {a:?}, expected {b:?} (off by {d:e})");
}

/// The shape map against the decode's `n = (cos 2α, sin 2α cos β, sin 2α sin β)` (§3.1 on §0.2's `ρ̃`, `λ̃`), for any
/// masses, rotation, scale and translation; and the landmarks of dd_integrator §3.7 at equal masses.
fn check_shape(map: fn([[f64; 2]; 3], [f64; 3]) -> [f64; 3]) {
    let third = 1.0 / 3.0;
    let eq = [third; 3];
    let s3 = 3f64.sqrt();
    // Landmarks: collisions 01, 12, 20; Euler 2-in-the-middle (−b₀₁); L⁺ (0 → 1 → 2 anticlockwise) and L⁻.
    type Landmark = ([[f64; 2]; 3], [f64; 3], &'static str);
    let landmarks: [Landmark; 6] = [
        (
            [[0.3, 0.2], [0.3, 0.2], [1.1, -0.4]],
            [-1.0, 0.0, 0.0],
            "collision 01",
        ),
        (
            [[0.0, 0.0], [1.0, 0.5], [1.0, 0.5]],
            [0.5, s3 / 2.0, 0.0],
            "collision 12",
        ),
        (
            [[0.0, 0.0], [1.0, 0.5], [0.0, 0.0]],
            [0.5, -s3 / 2.0, 0.0],
            "collision 20",
        ),
        (
            [[-1.0, 0.0], [1.0, 0.0], [0.0, 0.0]],
            [1.0, 0.0, 0.0],
            "Euler e01 = −b01",
        ),
        (
            [[0.0, 0.0], [1.0, 0.0], [0.5, s3 / 2.0]],
            [0.0, 0.0, 1.0],
            "L+",
        ),
        (
            [[0.0, 0.0], [1.0, 0.0], [0.5, -s3 / 2.0]],
            [0.0, 0.0, -1.0],
            "L−",
        ),
    ];
    for (r, want, what) in landmarks {
        close(map(r, eq), want, 1e-12, what);
    }
    let strategy = (
        0.02f64..1.0,
        0.02f64..1.0,
        0.02f64..1.0,
        0.01f64..(FRAC_PI_2 - 0.01),
        -PI..PI,
        -PI..PI,
        0.1f64..10.0,
        (-5.0f64..5.0, -5.0f64..5.0),
    );
    validation::prop::run(&strategy, |(a, b, c, alpha, beta, g, sc, (tx, ty))| {
        let total = a + b + c;
        let m = [a / total, b / total, c / total];
        let r = decode(alpha, beta, m, g, sc, [tx, ty]);
        let want = [
            (2.0 * alpha).cos(),
            (2.0 * alpha).sin() * beta.cos(),
            (2.0 * alpha).sin() * beta.sin(),
        ];
        // The positions are O(10) and the map is a ratio of quadratics; f64 round-off is far below 10⁻⁹.
        close(map(r, m), want, 1e-9, "the decode's n");
        Ok(())
    });
}

#[test]
fn qa_theta_unwrap_shape_map() {
    check_shape(shape::<f64>);
}

negative_control!(
    qa_theta_unwrap_shape_map,
    "a shape map with the cross negated (chart_reference §3.1's superseded version) must fail L+",
    expected = "L+",
    check_shape(|r, m| {
        let n = shape::<f64>(r, m);
        [n[0], n[1], -n[2]]
    })
);

#[cfg(feature = "controls")]
mod shape_weight_control {
    use super::*;

    negative_control!(
        qa_theta_unwrap_shape_map,
        "a shape map weighting every configuration as equal masses must fail at unequal masses",
        expected = "the decode's n",
        check_shape(|r, _m| shape::<f64>(r, [1.0 / 3.0; 3]))
    );
}

// ----- The bring-up variant (REQ-TOOL-013; Appendix A; REQ-TOOL-123) -----

/// Appendix A's pattern for sample `i`, from its table: the 31 real slots `32·i + k` in the ledger's member order, and
/// `packed_a`, `packed_b`, `times`, `total_substeps`, `closure_step`, `_reserved` as words. `packed_a`'s bits by the
/// payload's §2 layout: `state` 0–2, `detail` 3–4, `saturated` 5, `dmin_pair` 6–7, `last_symbol` 8–9, reserved 10–15
/// zero, `d_min` 16–31 unset, f16 +∞ (`0x7c00`, R-271).
struct Expected {
    reals: [f64; 31],
    packed_a: u32,
    packed_b: u32,
    times: u32,
    total_substeps: u32,
    closure_step: u16,
    reserved: u16,
}

fn appendix_a(i: u32) -> Expected {
    let j = i % 65536;
    let mut reals = [0.0; 31];
    for (k, r) in reals.iter_mut().enumerate() {
        *r = 32.0 * f64::from(i) + k as f64;
    }
    Expected {
        reals,
        packed_a: (i % 6)
            | ((i / 2 % 4) << 3)
            | ((i / 8 % 2) << 5)
            | ((i / 16 % 4) << 6)
            | ((i / 64 % 4) << 8)
            | (0x7c00 << 16),
        packed_b: 0,
        times: j | ((65535 - j) << 16),
        total_substeps: i,
        closure_step: j as u16,
        reserved: 0,
    }
}

/// The real slots of `s` in the ledger's member order, `r[b][c]` at `2b + c`.
fn slots<R: PayloadReal + Copy + Into<f64>>(s: &SimStateFTLEOf<R>) -> [f64; 31] {
    let mut out = [0.0; 31];
    let vecs = [s.r, s.p, s.r_sh, s.p_sh];
    for (v, block) in vecs.iter().enumerate() {
        for b in 0..3 {
            for c in 0..2 {
                out[6 * v + 2 * b + c] = block[b][c].into();
            }
        }
    }
    let scalars = [s.S, s.theta, s.mean_y, s.C_ty, s.E_0, s.Lz_0, s.closure_min];
    for (k, x) in scalars.iter().enumerate() {
        out[24 + k] = (*x).into();
    }
    out
}

/// `s`, sample `i`'s stored state, is Appendix A's pattern: every real slot and every word, and read through the
/// generated unpack (lowering Part 3a) each decoded field Appendix A's.
fn check_sample<R: PayloadReal + Copy + Into<f64>>(
    i: u32,
    s: &SimStateFTLEOf<R>,
    narrow: &SimStateFTLE,
) {
    let e = appendix_a(i);
    let got = slots(s);
    for (k, (g, w)) in got.iter().zip(&e.reals).enumerate() {
        assert!(
            g == w,
            "sample {i}: real slot {k} holds {g}, not Appendix A's {w}"
        );
    }
    let words = [
        ("packed_a", s.packed_a, e.packed_a),
        ("packed_b", s.packed_b, e.packed_b),
        ("times", s.times, e.times),
        ("total_substeps", s.total_substeps, e.total_substeps),
        (
            "closure_step",
            u32::from(s.closure_step),
            u32::from(e.closure_step),
        ),
        ("_reserved", u32::from(s._reserved), u32::from(e.reserved)),
    ];
    for (name, g, w) in words {
        assert!(
            g == w,
            "sample {i}: `{name}` is {g:#x}, not Appendix A's {w:#x}"
        );
    }
    let params = ReadParams {
        dt_macro: 1.0,
        delta_0: 1.0,
        n_renorm: 0,
        horizon_steps: 65535,
    };
    let read = sim_state_from_ftle(
        narrow,
        FGW_UNBOUND,
        false,
        canonical_nan(),
        false,
        [1.0 / 3.0; 3],
        &params,
    );
    let j = i % 65536;
    let decoded = [
        ("state", read.state, i % 6),
        ("detail", read.detail, i / 2 % 4),
        ("saturated", u32::from(read.saturated), i / 8 % 2),
        ("dmin_pair", read.dmin_pair, i / 16 % 4),
        ("last_symbol", read.last_symbol, i / 64 % 4),
        ("t_end_step", read.t_end_step, j),
        ("t_dmin_step", read.t_dmin_step, 65535 - j),
        ("total_substeps", read.total_substeps, i),
        ("closure_step", read.closure_step, j),
    ];
    for (name, g, w) in decoded {
        assert!(
            g == w,
            "sample {i}: decoded `{name}` is {g}, not Appendix A's {w}"
        );
    }
    assert!(
        read.d_min == f32::INFINITY,
        "sample {i}: decoded d_min is {}, not unset (+∞)",
        read.d_min
    );
    assert!(
        read.dE_max.to_bits() == 0 && read.dLz_max.to_bits() == 0,
        "sample {i}: decoded drift maxima are {} and {}, not +0",
        read.dE_max,
        read.dLz_max
    );
    let decoded_reals = [
        (read.r[0][0], 0),
        (read.r[2][1], 5),
        (read.p[0][0], 6),
        (read.p[2][1], 11),
        (read.S, 24),
        (read.theta, 25),
        (read.mean_y, 26),
        (read.C_ty, 27),
        (read.E_0, 28),
        (read.Lz_0, 29),
        (read.closure_min, 30),
    ];
    for (g, k) in decoded_reals {
        assert!(
            f64::from(g) == e.reals[k],
            "sample {i}: decoded real slot {k} is {g}, not Appendix A's {}",
            e.reals[k]
        );
    }
}

/// The samples checked: every packed field's full cycle (0–255), `j`'s wrap at 2¹⁶, and the last sample the pattern
/// is exact for at f32, 2¹⁹ − 1.
fn indices() -> Vec<u32> {
    let mut v: Vec<u32> = (0..256).collect();
    v.extend([65534, 65535, 65536, 65537, 300_001, (1 << 19) - 1]);
    v
}

/// The f64 native instantiation, narrowed (exactly: every value is an integer below 2²⁴) for the unpack.
fn narrow(s: &SimStateFTLEOf<f64>) -> SimStateFTLE {
    let v = |a: [[f64; 2]; 3]| a.map(|p| p.map(|x| x as f32));
    SimStateFTLE {
        r: v(s.r),
        p: v(s.p),
        r_sh: v(s.r_sh),
        p_sh: v(s.p_sh),
        S: s.S as f32,
        theta: s.theta as f32,
        mean_y: s.mean_y as f32,
        C_ty: s.C_ty as f32,
        E_0: s.E_0 as f32,
        Lz_0: s.Lz_0 as f32,
        packed_a: s.packed_a,
        packed_b: s.packed_b,
        times: s.times,
        total_substeps: s.total_substeps,
        closure_min: s.closure_min as f32,
        closure_step: s.closure_step,
        _reserved: s._reserved,
        ..SimStateFTLE::default()
    }
}

/// Both instantiations of the bring-up writer, f64 and f32, are Appendix A's pattern at every checked sample.
fn check_variant(f64_writer: fn(u32) -> SimStateFTLEOf<f64>, f32_writer: fn(u32) -> SimStateFTLE) {
    for i in indices() {
        let s64 = f64_writer(i);
        check_sample(i, &s64, &narrow(&s64));
        let s32 = f32_writer(i);
        check_sample(i, &s32, &s32);
    }
}

#[test]
fn qa_debug_variants_exactly_the_bring_up_mode() {
    // R-75: one debug mode; UV, DECODE and ROUNDTRIP are not kernel variants.
    assert_eq!(
        VARIANTS.len(),
        1,
        "the kernel has {} variants listed, not the bring-up mode alone",
        VARIANTS.len()
    );
    let debug: Vec<&str> = VARIANTS.iter().filter(|v| v.1).map(|v| v.0).collect();
    assert_eq!(
        debug,
        vec![BringUp::NAME],
        "the kernel's debug variants are {debug:?}"
    );
    assert_eq!(
        (BringUp::NAME, <BringUp as Variant>::DEBUG),
        ("bring_up", true),
        "the bring-up variant is not a debug mode by its type"
    );
    for name in ["uv", "decode", "roundtrip", "round_trip"] {
        assert!(
            !VARIANTS.iter().any(|v| v.0.to_lowercase().contains(name)),
            "`{name}` is a kernel variant (R-75)"
        );
    }
    // R-41: selected by its type, baked; the writer's inputs are the sample's index alone (and the output buffer), so
    // no flags word, `DEBUG_MODE` bit or uniform can reach it. A signature change fails this at compile time.
    let _native: fn(u32) -> SimStateFTLEOf<f64> = pattern::<f64>;
    let _words: fn(u32, &mut [u32]) = write_words;
    check_variant(pattern::<f64>, pattern::<f32>);
}

negative_control!(
    qa_debug_variants_exactly_the_bring_up_mode,
    "a writer one slot off must fail Appendix A's pattern (pitfalls §9)",
    expected = "not Appendix A's",
    check_variant(
        |i| {
            let mut s = pattern::<f64>(i);
            s.theta = s.S;
            s
        },
        pattern::<f32>
    )
);

#[cfg(feature = "controls")]
mod variant_controls {
    use super::*;

    negative_control!(
        qa_debug_variants_exactly_the_bring_up_mode,
        "a reserved bit of packed_a set must fail, though the unpack masks it",
        expected = "`packed_a` is",
        check_variant(pattern::<f64>, |i| {
            let mut s = pattern::<f32>(i);
            s.packed_a |= 1 << 12;
            s
        })
    );

    mod times_control {
        use super::*;

        negative_control!(
            qa_debug_variants_exactly_the_bring_up_mode,
            "times with its halves swapped must fail",
            expected = "not Appendix A's",
            check_variant(
                |i| {
                    let mut s = pattern::<f64>(i);
                    s.times = s.times.rotate_left(16);
                    s
                },
                pattern::<f32>
            )
        );
    }
}
