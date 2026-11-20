//! QA tests for TASK-M1-11, second round: `θ̃`'s pole hold stepped from the configuration, the form the kernel steps
//! by (`theta_step_config`), written from the requirements, not from the code:
//! - REQ-INT-001 (dd_integrator §3.7; R-389; R-392): `θ̃` starts at 0, adds the principal-value delta of
//!   `atan2(n_v, n_u)` each step, and inside the pole disc `√(n_u² + n_v²) < r_pole` holds with the last longitude
//!   outside frozen, adding `wrap(exit − stored)` on exit; an IC inside adds nothing at its first exit. The disc is
//!   strict: a point at exactly `r_pole` is outside, and one a hair inside is held.
//! - REQ-INT-086 (R-389): the hold fires on a pole passage and never on a path circulating the `w` axis away from the
//!   poles, at the shared kernel's two precisions, f32 and f64 (R-265).
//!
//! The configurations are chart_reference §0.2's canonical-frame decode of a chosen shape point, rotated, scaled and
//! translated, so the rule is evaluated on the chosen point `n` itself, never on the code's `shape`. Masses sum to 1
//! (`M = 1`, chart_reference §0.2). Every test name holds `theta_unwrap`, so `cargo test -p kernel theta_unwrap` runs
//! it. Each test has its negative control (R-176).
// The file name `qa_TASK-M1-11_config` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::f64::consts::{FRAC_PI_2, PI, TAU};

#[cfg(feature = "controls")]
use kernel::shape::{disc_input, in_disc, theta_step_held};
use kernel::shape::{shape, theta_step_config, Theta};
use validation::negative_control;

// ----- The rule, written from R-389 and R-392 -----

/// R-389's wrap: `d − 2πk` in (−π, π].
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

/// The longitude `atan2(n_v, n_u)`.
fn lon(n: [f64; 3]) -> f64 {
    n[1].atan2(n[0])
}

/// The polar radius `√(n_u² + n_v²)`.
fn rho(n: [f64; 3]) -> f64 {
    (n[0] * n[0] + n[1] * n[1]).sqrt()
}

/// The shape point at longitude `l` and polar radius `r`, north (`w > 0`) or south.
fn pt(l: f64, r: f64, north: bool) -> [f64; 3] {
    let w = (1.0 - r * r).max(0.0).sqrt();
    [r * l.cos(), r * l.sin(), if north { w } else { -w }]
}

/// `θ̃` after each point of a path of shape points, by R-389's and R-392's words.
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

// ----- Configurations from shape points (chart_reference §0.2, §3.1) -----

/// chart_reference §0.2's canonical-frame decode at `(α, β)`, masses `m` summing to 1, rotated by `g`, scaled by `sc`
/// and translated by `t`. Its shape point is `(cos 2α, sin 2α cos β, sin 2α sin β)` (§3.1).
fn decode(alpha: f64, beta: f64, m: [f64; 3], g: f64, sc: f64, t: [f64; 2]) -> [[f64; 2]; 3] {
    let m01 = m[0] + m[1];
    let mu_rho = m[0] * m[1] / m01;
    let mu_lambda = m[2] * m01;
    let rho_v = [alpha.cos() / mu_rho.sqrt(), 0.0];
    let lam = [
        alpha.sin() * beta.cos() / mu_lambda.sqrt(),
        alpha.sin() * beta.sin() / mu_lambda.sqrt(),
    ];
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

/// A frame: masses (summing to 1), rotation, scale, translation.
type Frame = ([f64; 3], f64, f64, [f64; 2]);

/// Unequal and equal masses, in frames that rotate, scale and translate the triangle.
const FRAMES: [Frame; 4] = [
    ([1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0], 0.0, 1.0, [0.0, 0.0]),
    ([0.2, 0.3, 0.5], 0.7, 2.0, [0.4, -0.3]),
    ([0.6, 0.25, 0.15], -2.1, 0.5, [-0.8, 0.6]),
    ([0.1, 0.45, 0.45], 2.9, 1.3, [0.2, 0.9]),
];

/// The configuration, in `frame`, whose shape point is the unit point `n`.
fn config(n: [f64; 3], frame: Frame) -> [[f64; 2]; 3] {
    let (m, g, sc, t) = frame;
    let alpha = n[0].clamp(-1.0, 1.0).acos() / 2.0;
    let beta = n[2].atan2(n[1]);
    decode(alpha, beta, m, g, sc, t)
}

/// `θ̃` after each configuration of `path`, by `theta_step_config` at f64.
fn run64(path: &[[[f64; 2]; 3]], m: [f64; 3], r_pole: f64, step: Step64) -> Vec<f64> {
    let mut t = Theta::start();
    let mut out = vec![t.theta];
    for w in path.windows(2) {
        t = step(t, w[0], w[1], m, r_pole);
        out.push(t.theta);
    }
    out
}

/// `θ̃` after each configuration of `path`, by `theta_step_config` at f32 (positions and masses rounded to f32).
fn run32(path: &[[[f64; 2]; 3]], m: [f64; 3], r_pole: f64) -> Vec<f64> {
    let to32 = |c: [[f64; 2]; 3]| c.map(|p| p.map(|x| x as f32));
    let m32 = m.map(|x| x as f32);
    let mut t = Theta::<f32>::start();
    let mut out = vec![t.theta as f64];
    for w in path.windows(2) {
        t = theta_step_config(t, to32(w[0]), to32(w[1]), m32, r_pole as f32);
        out.push(t.theta as f64);
    }
    out
}

/// `θ̃` along a configuration path at f32: [`run32`], or a control's faulty one.
type Run32 = fn(&[Config], [f64; 3], f64) -> Vec<f64>;

/// The bodies' positions.
type Config = [[f64; 2]; 3];

/// One f64 macro-step from configurations: `theta_step_config`, or a control's faulty one.
type Step64 = fn(Theta<f64>, [[f64; 2]; 3], [[f64; 2]; 3], [f64; 3], f64) -> Theta<f64>;

/// f64 tolerance: the configurations carry the decode's round-off, `O(10⁻¹⁵)` in `n`, so a longitude at polar radius
/// at least `1.01·10⁻³` (the smallest radius outside on these paths) is within 10⁻¹² of the chosen one; `θ̃` telescopes,
/// so its error is two longitudes' plus the sum's round-off, 150 steps · 20 · 2⁻⁵². A missed or doubled delta, or one
/// from the wrong reference, is at least 10⁻² on these paths.
const TOL64: f64 = 1e-9;

/// f32 tolerance at `r_pole`: two trusted longitudes' error, `64·2⁻²⁴/ρ` each at the smallest radius outside
/// (`1.01·r_pole`; 64 the conditioning of the frames, whose scale and translation are at most 2 and 1), plus the
/// sum's round-off, 150 steps · |θ̃| ≤ 20 · 2⁻²⁴.
fn tol32(r_pole: f64) -> f64 {
    let eps = 2f64.powi(-24);
    2.0 * 64.0 * eps / (1.01 * r_pole) + 150.0 * 20.0 * eps
}

/// The radii the f64 paths run at: REQ-INT-086's proposal is among them, and R-389's rule holds at any radius.
const RADII: [f64; 3] = [1e-3, 1e-2, 0.1];

/// At f32 the longitude near `r_pole = 10⁻³` carries `~10⁻⁴` of round-off, which is the calibration's evidence, not
/// this rule's; the f32 paths run at the larger two.
const RADII32: [f64; 2] = [1e-2, 0.1];

// ----- Paths -----

/// A circulating path away from the poles: polar radius 0.4 ± 0.2, `total` turning.
fn circulating(l0: f64, total: f64, north: bool) -> Vec<[f64; 3]> {
    let steps = 120;
    (0..=steps)
        .map(|k| {
            let s = k as f64 / steps as f64;
            pt(l0 + total * s, 0.4 + 0.2 * (5.0 * TAU * s).sin(), north)
        })
        .collect()
}

/// A pole passage: in from polar radius 0.3 to `1.01·r_pole` at `l_in`, a wander inside at radii below `r_pole`
/// through every longitude, out at `1.02·r_pole` at `l_out`, and back to 0.3. The path, the last point outside before
/// and the first one after.
fn passage(l_in: f64, l_out: f64, r_pole: f64, north: bool) -> (Vec<[f64; 3]>, usize, usize) {
    let mut path = Vec::new();
    for k in 0..30 {
        let s = k as f64 / 30.0;
        path.push(pt(
            l_in - 0.5 * (1.0 - s),
            0.3 + (1.5 * r_pole - 0.3) * s,
            north,
        ));
    }
    path.push(pt(l_in, 1.01 * r_pole, north));
    let last_out = path.len() - 1;
    for k in 0..20 {
        path.push(pt(
            l_in + 1.3 + 1.1 * k as f64,
            r_pole * (0.9 - 0.8 * (k as f64 / 20.0).sin()),
            north,
        ));
    }
    path.push(pt(l_out, 1.02 * r_pole, north));
    let first_out = path.len() - 1;
    for k in 1..30 {
        let s = k as f64 / 30.0;
        path.push(pt(
            l_out + 0.4 * s,
            1.02 * r_pole + (0.3 - 1.02 * r_pole) * s,
            north,
        ));
    }
    (path, last_out, first_out)
}

/// An IC inside the disc (R-392): inside at `l_ic`, a wander inside, out at `l_exit`, then on.
fn ic_inside(l_ic: f64, l_exit: f64, r_pole: f64, north: bool) -> (Vec<[f64; 3]>, usize) {
    let mut path = vec![pt(l_ic, 0.5 * r_pole, north)];
    for k in 1..10 {
        path.push(pt(
            l_ic + 0.8 * k as f64,
            r_pole * (0.5 + 0.04 * k as f64),
            north,
        ));
    }
    path.push(pt(l_exit, 1.02 * r_pole, north));
    let first_out = path.len() - 1;
    for k in 1..30 {
        let s = k as f64 / 30.0;
        path.push(pt(
            l_exit - 0.6 * s,
            1.02 * r_pole + (0.3 - 1.02 * r_pole) * s,
            north,
        ));
    }
    (path, first_out)
}

/// The paths each configuration test runs: (shape-point path, what it is).
fn paths(r_pole: f64) -> Vec<(Vec<[f64; 3]>, String)> {
    let mut out = Vec::new();
    for (l0, total, north) in [
        (0.3, 3.0 * TAU + 1.0, true),
        (-2.0, -2.0 * TAU - 0.5, false),
    ] {
        out.push((
            circulating(l0, total, north),
            format!("circulating {total}"),
        ));
    }
    // No exit-minus-stored difference of exactly ±π here: from a configuration its longitudes carry round-off, so
    // which side of the tie it lands on is not the rule's; the tie is the point paths' (`qa_TASK-M1-11.rs`).
    for (l_in, l_out, north) in [(0.2, 2.9, true), (2.5, -2.6, false), (-1.0, 2.0, true)] {
        let (p, _, _) = passage(l_in, l_out, r_pole, north);
        out.push((p, format!("passage {l_in} -> {l_out}")));
    }
    for (l_ic, l_exit, north) in [(1.0, -2.0, true), (-0.5, 2.2, false)] {
        let (p, _) = ic_inside(l_ic, l_exit, r_pole, north);
        out.push((p, format!("IC inside, exit at {l_exit}")));
    }
    out
}

// ----- The configuration path follows the rule -----

/// On every path, in every frame and at every radius, `θ̃` stepped from the configurations agrees point by point with
/// the rule on the chosen shape points: circulation with no 2π jump and no hold, the hold through a pole passage with
/// `wrap(exit − stored)` on exit, and an IC inside adding nothing at its first exit.
fn check_config_paths(step: Step64) {
    for r_pole in RADII {
        for frame in FRAMES {
            for (path, what) in paths(r_pole) {
                let configs: Vec<_> = path.iter().map(|&n| config(n, frame)).collect();
                let got = run64(&configs, frame.0, r_pole, step);
                let want = expected(&path, r_pole);
                for (k, (g, w)) in got.iter().zip(&want).enumerate() {
                    assert!(
                        (g - w).abs() <= TOL64,
                        "configuration path: {what}, masses {:?}, r_pole {r_pole}: θ̃ after point {k} is {g}, \
                         the rule's {w}",
                        frame.0
                    );
                }
            }
        }
    }
}

#[test]
fn qa_theta_unwrap_config_paths() {
    check_config_paths(theta_step_config);
}

negative_control!(
    qa_theta_unwrap_config_paths,
    "a configuration step that never holds must fail the pole passage",
    expected = "configuration path",
    check_config_paths(|t, a, b, m, _r| theta_step_held(t, shape(a, m), shape(b, m), false, false))
);

#[cfg(feature = "controls")]
mod config_paths_unnormalised {
    use super::*;

    negative_control!(
        qa_theta_unwrap_config_paths,
        "a disc decided on the scaled ρ²_I alone, not against r_pole²·I², must fail in scaled frames",
        expected = "configuration path",
        check_config_paths(|t, a, b, m, r| {
            let inside = |c| disc_input(c, m).rho2 < r * r;
            theta_step_held(t, shape(a, m), shape(b, m), inside(a), inside(b))
        })
    );
}

#[cfg(feature = "controls")]
mod config_paths_equal_masses {
    use super::*;

    negative_control!(
        qa_theta_unwrap_config_paths,
        "a disc decided as if the masses were equal must fail at unequal masses",
        expected = "configuration path",
        check_config_paths(|t, a, b, m, r| {
            let eq = [1.0 / 3.0; 3];
            theta_step_held(
                t,
                shape(a, m),
                shape(b, m),
                in_disc(disc_input(a, eq), r),
                in_disc(disc_input(b, eq), r),
            )
        })
    );
}

// ----- The same at f32, the GPU's precision (R-265) -----

/// The f32 configuration step on the same paths, against the rule within [`tol32`]: the hold fires on the passages and
/// never on the circulating paths at f32 too.
fn check_config_paths_f32(run: Run32) {
    for r_pole in RADII32 {
        let tol = tol32(r_pole);
        for frame in FRAMES {
            for (path, what) in paths(r_pole) {
                let configs: Vec<_> = path.iter().map(|&n| config(n, frame)).collect();
                let got = run(&configs, frame.0, r_pole);
                let want = expected(&path, r_pole);
                for (k, (g, w)) in got.iter().zip(&want).enumerate() {
                    assert!(
                        (g - w).abs() <= tol,
                        "f32 configuration path: {what}, masses {:?}, r_pole {r_pole}: θ̃ after point {k} is {g}, \
                         the rule's {w} (tolerance {tol:e})",
                        frame.0
                    );
                }
            }
        }
    }
}

#[test]
fn qa_theta_unwrap_config_paths_f32() {
    check_config_paths_f32(run32);
}

negative_control!(
    qa_theta_unwrap_config_paths_f32,
    "an f32 step whose disc is ten times too wide must hold where the rule does not",
    expected = "f32 configuration path",
    check_config_paths_f32(|p, m, r| run32(p, m, 10.0 * r))
);

// ----- Inside iff ρ < r_pole, over the configuration space -----

/// For any configuration, in any frame and at any radius: a step from a point well outside (polar radius 0.95, so
/// outside for every `r_pole < 0.9`) to the configuration holds `θ̃` at 0 exactly when the configuration's shape point
/// has `√(n_u² + n_v²) < r_pole`, and otherwise adds the wrapped longitude difference. Cases within 10⁻⁹ (relative)
/// of the edge, inside f64 round-off of the decode, are left to the exact edge test below.
fn check_disc_property(step: Step64) {
    let strategy = (
        (0.02f64..1.0, 0.02f64..1.0, 0.02f64..1.0),
        0.0f64..FRAC_PI_2,
        -PI..PI,
        (-PI..PI, 0.1f64..10.0, -5.0f64..5.0, -5.0f64..5.0),
        1e-4f64..0.9,
    );
    validation::prop::run(
        &strategy,
        |((a, b, c), alpha, beta, (g, sc, tx, ty), r_pole)| {
            let total = a + b + c;
            let m = [a / total, b / total, c / total];
            let n = [
                (2.0 * alpha).cos(),
                (2.0 * alpha).sin() * beta.cos(),
                (2.0 * alpha).sin() * beta.sin(),
            ];
            if (rho(n) - r_pole).abs() <= 1e-9 * r_pole {
                return Ok(());
            }
            let from_n = pt(lon(n) + 1.0, 0.95, n[2] >= 0.0);
            let frame = (m, g, sc, [tx, ty]);
            let from = config(from_n, frame);
            let to = decode(alpha, beta, m, g, sc, [tx, ty]);
            let t = step(Theta::start(), from, to, m, r_pole);
            let want = expected(&[from_n, n], r_pole)[1];
            assert!(
            (t.theta - want).abs() <= 1e-6,
            "the disc decision: ρ = {}, r_pole {r_pole}, masses {m:?}: θ̃ is {}, the rule's {want} (held when ρ < r_pole)",
            rho(n),
            t.theta
        );
            Ok(())
        },
    );
}

#[test]
fn qa_theta_unwrap_config_disc_property() {
    check_disc_property(theta_step_config);
}

negative_control!(
    qa_theta_unwrap_config_disc_property,
    "a disc decided on ρ²_I < r_pole² without I² must fail where the configuration is not unit-scaled",
    expected = "the disc decision",
    check_disc_property(|t, a, b, m, r| {
        let inside = |c| disc_input(c, m).rho2 < r * r;
        theta_step_held(t, shape(a, m), shape(b, m), inside(a), inside(b))
    })
);

#[cfg(feature = "controls")]
mod disc_property_radius {
    use super::*;

    negative_control!(
        qa_theta_unwrap_config_disc_property,
        "a disc of radius r_pole² rather than r_pole must fail",
        expected = "the disc decision",
        check_disc_property(|t, a, b, m, r| theta_step_config(t, a, b, m, r * r))
    );
}

// ----- The exact edge: at r_pole is outside, a hair inside is held -----

// A configuration whose shape point lies exactly on the circle of polar radius 1/2, every quantity of R-389's
// squared test exact at f32 and f64. Masses (3/4, 1/8, 1/8), so M₀₁ = 7/8 and μ_ρμ_λ = m₀m₁m₂ = 3/256 (M = 1). Bodies
// at (0, 0), (14, 0) and (2, 8): ρ = r₁ − r₀ = (14, 0) and M₀₁λ = M₀₁r₂ − (m₀r₀ + m₁r₁) = (7/4, 7) − (7/4, 0) = (0, 7),
// so ρ ⟂ λ (n_v = 0). Scaled by M₀₁: u = m₀m₁‖ρ‖² − m₂‖M₀₁λ‖² = (3/32)·196 − 49/8 = 49/4 and w = 2√(3/256)·(14·7),
// so u² = 2401/16, w² = (3/64)·98² = 7203/16 = 3u² and I² = u² + w² = 4u²: √(n_u² + n_v²) = u/I = 1/2 exactly, and
// n = (1/2, 0, √3/2), the triangle 0 → 1 → 2 anticlockwise. Every product above is a small dyadic, exact in f32.
const EDGE_M: [f64; 3] = [0.75, 0.125, 0.125];
const EDGE: [[f64; 2]; 3] = [[0.0, 0.0], [14.0, 0.0], [2.0, 8.0]];

/// The edge test at one precision, through a step function on that precision's configurations; `next_up` is the
/// smallest float above 1/2 at that precision.
macro_rules! edge_check {
    ($name:ident, $t:ty, $tol:expr) => {
        fn $name(step: fn(Theta<$t>, [[$t; 2]; 3], [[$t; 2]; 3], [$t; 3], $t) -> Theta<$t>) {
            let m = EDGE_M.map(|x| x as $t);
            let edge = EDGE.map(|p| p.map(|x| x as $t));
            let half: $t = 0.5;
            let next_up = <$t>::from_bits(half.to_bits() + 1);
            // A point outside at longitude 0.4, and one at longitude 1.0, polar radius 0.6, in the edge's masses.
            let frame = (EDGE_M, 0.0, 1.0, [0.0, 0.0]);
            let cast = |c: [[f64; 2]; 3]| c.map(|p| p.map(|x| x as $t));
            let a = cast(config(pt(0.4, 0.6, true), frame));
            let b = cast(config(pt(1.0, 0.6, true), frame));
            // At r_pole = 1/2 the edge point is outside: the step adds its delta, 0 − 0.4.
            let t = step(Theta::start(), a, edge, m, half);
            assert!(
                ((t.theta as f64) - (-0.4)).abs() <= $tol,
                "a step to polar radius exactly r_pole added {}, not its delta −0.4: the edge is outside",
                t.theta
            );
            // At the next float above 1/2 it is inside: held, and the exit adds wrap(1.0 − 0.4).
            let t = step(Theta::start(), a, edge, m, next_up);
            assert!(
                t.theta == 0.0,
                "a step to a point just inside the disc (r_pole one ulp above its radius) added {}: just inside is held",
                t.theta
            );
            let t = step(t, edge, b, m, next_up);
            assert!(
                ((t.theta as f64) - 0.6).abs() <= $tol,
                "the exit from just inside added {}, not wrap(1.0 − 0.4) = 0.6: just inside is held",
                t.theta
            );
        }
    };
}

// f64: the points a and b carry the decode's round-off, 10⁻¹⁵. f32: rounding a and b's positions, 2⁻²⁴ relative,
// moves their longitudes by at most 16·2⁻²⁴ at polar radius 0.6.
edge_check!(edge64, f64, 1e-12);
edge_check!(edge32, f32, 1e-5);

#[test]
fn qa_theta_unwrap_config_disc_edge() {
    // The fixture is on the edge: the shape point the code maps it to is (1/2, 0, √3/2).
    let n = shape(EDGE, EDGE_M);
    let want = [0.5, 0.0, 3f64.sqrt() / 2.0];
    for k in 0..3 {
        assert!(
            (n[k] - want[k]).abs() <= 1e-15,
            "the edge fixture maps to {n:?}, not {want:?}"
        );
    }
    edge64(theta_step_config);
    edge32(theta_step_config);
}

negative_control!(
    qa_theta_unwrap_config_disc_edge,
    "a closed disc, ρ²_I ≤ r_pole²·I², must fail the edge",
    expected = "the edge is outside",
    edge32(|t, a, b, m, r| {
        let inside = |c| {
            let d = disc_input(c, m);
            d.rho2 <= r * r * d.i2
        };
        theta_step_held(t, shape(a, m), shape(b, m), inside(a), inside(b))
    })
);

#[cfg(feature = "controls")]
mod edge_closed_f64 {
    use super::*;

    negative_control!(
        qa_theta_unwrap_config_disc_edge,
        "a closed disc at f64 must fail the edge",
        expected = "the edge is outside",
        edge64(|t, a, b, m, r| {
            let inside = |c| {
                let d = disc_input(c, m);
                d.rho2 <= r * r * d.i2
            };
            theta_step_held(t, shape(a, m), shape(b, m), inside(a), inside(b))
        })
    );
}

#[cfg(feature = "controls")]
mod edge_loose {
    use super::*;

    negative_control!(
        qa_theta_unwrap_config_disc_edge,
        "a disc a millionth narrower than r_pole must fail to hold just inside",
        expected = "just inside is held",
        edge32(|t, a, b, m, r| theta_step_config(t, a, b, m, r * (1.0 - 1e-6)))
    );
}
