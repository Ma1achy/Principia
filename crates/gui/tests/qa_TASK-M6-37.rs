//! QA tests for TASK-M6-37, written from REQ-GUI-203 and R-451 B11, not from the implementation:
//!
//! - B11's test: `s = 2√(g₁₁g₂₂ − g₁₂²) / (g₁₁ + g₂₂)` on the plane's own `q₁`, `q₂`, which under the decoder's map is
//!   `√(1 − sin²τ₁ sin²τ₂)` for every `γ` and `σ`, is unchanged by scaling both vectors together, and is `≈ 2r` for
//!   orthogonal vectors of norm ratio `r`.
//! - The guards, `in_plane` and `chart_coords`, keep their sine tests and also decline where `s < 10⁻⁶` (the guards' own
//!   cut, not the compass's 10⁻³): at the four corner points `τ₁ = ±τ₂ = ±90°`, every `γ` and `σ` in [0.01, 4], they
//!   return none and the mock's figure (mock/quads.rs, mock/sample.rs) places nothing; everywhere else they return
//!   what they returned before this task (the sine-test guards, re-stated below from the task's text).
//! - Locked, at a corner a basis edit sends no `SetField` on `z₀` and passes as it does free; inside the compass's
//!   0.057° disc a locked tilt drag turns about the pin, its chart point kept within `17ε(1/sin²θ + 1/sin²θ′)` (B11's
//!   error model). Both are checked on `about_the_pin` and, end to end, on a tilt drag on the compass of the running
//!   app on the mock engine (the seam: the compass's drag → the app's `about_the_pin` → the engine's SetFields).
//!
//! gui takes no dependency on validation (R-187), so each check runs its own control (R-176) through [`rejects`].
// The file name `qa_TASK-M6-37` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use eframe::egui::{self, Event, PointerButton, Pos2};
use engine::contract::log::Source;
use engine::contract::set_field::{Edit, SetField, SimField};
use engine::contract::sim_config::{Lock, Plane, SimConfig};
use gui::app::App;
use gui::capture::{FORMAT, PIXELS_PER_POINT, SIZE};
use gui::explore::compass;
use gui::explore::manifold_view::{
    about_the_pin, basis_edit, chart_coords, in_plane, lock_edit, plane_s, z0_edit, View,
    PLANE_S_CUT, PRESETS, SLICE_AXIS,
};
use gui::headless::Headless;
use gui::layout::Layout;
use gui::mock::{quads, sample, MockEngine};
use gui::side::MockSide;

type Z = [f64; 8];

/// The double-precision rounding unit, as REQ-GUI-203 states it.
const EPS: f64 = 2.22e-16;

/// The four corner points `(τ₁, τ₂)`, in degrees.
const CORNERS: [(f64, f64); 4] = [(90.0, 90.0), (90.0, -90.0), (-90.0, 90.0), (-90.0, -90.0)];

/// Asserts that `check` panics: the check can fail (R-176's control, run in the test).
fn rejects(what: &str, check: impl FnOnce()) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check));
    assert!(
        outcome.is_err(),
        "the check passed on {what}, which it must reject"
    );
}

// --- vectors, written here ------------------------------------------------------------------------------------------

fn dot(a: &Z, b: &Z) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn norm(a: &Z) -> f64 {
    a.iter().map(|x| x * x).sum::<f64>().sqrt()
}

fn add(a: &Z, k: f64, b: &Z) -> Z {
    std::array::from_fn(|i| a[i] + k * b[i])
}

fn e(i: usize, k: f64) -> Z {
    std::array::from_fn(|j| if j == i { k } else { 0.0 })
}

/// The latent point with components `(x, y, z)` on the first preset's frame `(e_a, e_b, e_d)`.
fn in_frame(x: f64, y: f64, z: f64) -> Z {
    let (a, b) = PRESETS[0];
    add(&add(&e(a, x), y, &e(b, 1.0)), z, &e(SLICE_AXIS, 1.0))
}

/// The sine of the angle between `q₁` and `q₂`.
fn sine(q1: &Z, q2: &Z) -> f64 {
    let c = dot(q1, q2) / (norm(q1) * norm(q2));
    (1.0 - c * c).max(0.0).sqrt()
}

/// A small deterministic generator in `[0, 1)` (no randomness in a test).
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

// --- the views ------------------------------------------------------------------------------------------------------

/// The chart at `(τ₁, τ₂, γ)` in degrees and scale `σ`, in the first preset's frame.
fn view(t1: f64, t2: f64, gamma: f64, sigma: f64) -> View {
    let (a, b) = PRESETS[0];
    View {
        preset: Some(0),
        a,
        b,
        d: SLICE_AXIS,
        // Both targets the slice axis, the slice frame's defaults (R-457 E4): the one shared target this task tested.
        x: SLICE_AXIS,
        y: SLICE_AXIS,
        scale: sigma,
        tau1: t1.to_radians(),
        tau2: t2.to_radians(),
        gamma: gamma.to_radians(),
    }
}

fn plane(t1: f64, t2: f64, gamma: f64, sigma: f64, z0: Z) -> Plane {
    let (q1, q2) = view(t1, t2, gamma, sigma).basis();
    Plane { z0, q1, q2 }
}

/// σ over [0.01, 4], both ends included: 25 points, log-spaced.
fn sigma_sweep() -> Vec<f64> {
    (0..=24)
        .map(|i| 0.01 * 400f64.powf(f64::from(i) / 24.0))
        .collect()
}

// --- the guards before this task, from the task's text: their sine tests alone ---------------------------------------

/// `in_plane` before: `None` where `1 − (q̂₁·q̂₂)² ≤ 10⁻¹²` on the unit directions (or a zero vector).
fn in_plane_sine_only(p: &Plane, z: &Z) -> Option<(f64, f64)> {
    let (n1, n2) = (norm(&p.q1), norm(&p.q2));
    if n1 == 0.0 || n2 == 0.0 {
        return None;
    }
    let (h1, h2) = (p.q1.map(|x| x / n1), p.q2.map(|x| x / n2));
    let g = dot(&h1, &h2);
    let det = 1.0 - g * g;
    if det <= 1e-12 {
        return None;
    }
    let (c1, c2) = (dot(z, &h1), dot(z, &h2));
    Some(((c1 - g * c2) / det, (c2 - g * c1) / det))
}

/// `chart_coords` before: `None` where `g₁₁g₂₂ − g₁₂² ≤ 10⁻²⁴ g₁₁g₂₂`.
fn chart_coords_sine_only(p: &Plane, z: &Z) -> Option<(f64, f64)> {
    let r = add(z, -1.0, &p.z0);
    let (g11, g12, g22) = (dot(&p.q1, &p.q1), dot(&p.q1, &p.q2), dot(&p.q2, &p.q2));
    let det = g11 * g22 - g12 * g12;
    if det <= 1e-24 * (g11 * g22).max(f64::MIN_POSITIVE) {
        return None;
    }
    let (c1, c2) = (dot(&r, &p.q1), dot(&r, &p.q2));
    let (x, y) = ((g22 * c1 - g12 * c2) / det, (g11 * c2 - g12 * c1) / det);
    Some(((x + 1.0) / 2.0, (y + 1.0) / 2.0))
}

/// §G4's rule for a locked basis edit `(q₁, q₂) → (q₁', q₂')` with the pivot `(s, t)`: `z₀' = z₀ + (2s−1)(q₁ − q₁') +
/// (2t−1)(q₂ − q₂')` (chart_decoder_contract Part 4).
fn pivoted_z0(p: &Plane, (s, t): (f64, f64), (q1, q2): &(Z, Z)) -> Z {
    add(
        &add(&p.z0, 2.0 * s - 1.0, &add(&p.q1, -1.0, q1)),
        2.0 * t - 1.0,
        &add(&p.q2, -1.0, q2),
    )
}

/// The mock engine's starting `SimConfig`, read once through a running app's snapshot.
fn base_sim() -> SimConfig {
    static BASE: std::sync::OnceLock<SimConfig> = std::sync::OnceLock::new();
    BASE.get_or_init(|| Rig::new().app.snapshot().sim.clone())
        .clone()
}

fn locked_on(p: &Plane, anchor: Z) -> SimConfig {
    let mut sim = base_sim();
    sim.plane = p.clone();
    sim.lock = Lock {
        locked: true,
        z_locked: anchor,
    };
    sim
}

/// That the guards decline `p` at each of `points`.
fn declines(p: &Plane, points: &[Z], what: &str) {
    for z in points {
        assert_eq!(in_plane(p, z), None, "{what}: in_plane at {z:?}");
        assert_eq!(chart_coords(p, z), None, "{what}: chart_coords at {z:?}");
    }
}

/// That the mock's figure on `p` holds nothing: no quad, no bound, no figure point and no hover path.
fn figure_empty(p: &Plane, what: &str) {
    assert!(quads::quads(p).is_empty(), "{what}: quads");
    assert!(
        quads::quads_over(p, [-2.0, -2.0, 3.0, 3.0]).is_empty(),
        "{what}: quads_over"
    );
    assert!(quads::bounds(p).is_empty(), "{what}: bounds");
    assert!(
        quads::bounds_over(p, [-2.0, -2.0, 3.0, 3.0]).is_empty(),
        "{what}: bounds_over"
    );
    for uv in [(0.0, 0.0), (0.3, -0.2), (1.0, 1.0)] {
        assert_eq!(quads::on_figure(p, uv), None, "{what}: on_figure {uv:?}");
    }
    for at in [[0.5, 0.5], [0.1, 0.9]] {
        assert!(
            sample::hover_path(p, at).is_empty(),
            "{what}: hover_path at {at:?}"
        );
    }
}

// --- B11's s --------------------------------------------------------------------------------------------------------

#[test]
fn qa_s_is_b11s_closed_form_under_the_decoders_map() {
    // B11: under the decoder's map s = |u₁ × u₂| = √(1 − sin²τ₁ sin²τ₂) for every γ and σ. Off the corners, over a
    // grid, s agrees with it to rounding: B11's 17ε on the computed s², and as much again for the closed form and the
    // basis's own rounding.
    for i in 0..=12 {
        for j in 0..=12 {
            let (t1, t2) = (-90.0 + 15.0 * f64::from(i), -90.0 + 15.0 * f64::from(j));
            for gamma in [-170.0, -45.0, 0.0, 30.0, 45.0, 90.0, 135.0] {
                for sigma in [0.01, 0.3, 1.0, 4.0] {
                    let (q1, q2) = view(t1, t2, gamma, sigma).basis();
                    let s = plane_s(&q1, &q2);
                    let (a, b) = (t1.to_radians().sin(), t2.to_radians().sin());
                    let closed = 1.0 - a * a * b * b;
                    assert!(
                        (s * s - closed).abs() <= 2.0 * 17.0 * EPS,
                        "({t1}°, {t2}°, γ = {gamma}°), σ = {sigma}: s = {s}, closed form {}",
                        closed.max(0.0).sqrt()
                    );
                }
            }
        }
    }
    // The values the task states, to the digits it gives them.
    for (t1, t2, gamma, stated) in [
        (90.0, 89.9, 0.0, 1.75e-3),
        (90.0, 89.97, 45.0, 5.2e-4),
        (90.0, 89.999, 45.0, 1.75e-5),
        (90.0, 89.97, 0.0, 5.2e-4),
        (90.0, 89.999, 30.0, 1.75e-5),
    ] {
        let (q1, q2) = view(t1, t2, gamma, 1.0).basis();
        let s = plane_s(&q1, &q2);
        assert!(
            (s / stated - 1.0).abs() < 0.01,
            "({t1}°, {t2}°, γ = {gamma}°): s = {s}, the task's {stated}"
        );
    }
    // In the frame, s is 2|q₁ × q₂| / (|q₁|² + |q₂|²) by the cross product (B11's first form).
    let (a, b) = PRESETS[0];
    for (t1, t2, gamma) in [(30.0, -60.0, 20.0), (90.0, 89.97, 45.0), (10.0, 0.0, 0.0)] {
        let (q1, q2) = view(t1, t2, gamma, 0.7).basis();
        let (x, y) = (
            [q1[a], q1[b], q1[SLICE_AXIS]],
            [q2[a], q2[b], q2[SLICE_AXIS]],
        );
        let cross = [
            x[1] * y[2] - x[2] * y[1],
            x[2] * y[0] - x[0] * y[2],
            x[0] * y[1] - x[1] * y[0],
        ];
        let by_cross =
            2.0 * cross.iter().map(|c| c * c).sum::<f64>().sqrt() / (dot(&q1, &q1) + dot(&q2, &q2));
        let s = plane_s(&q1, &q2);
        assert!(
            (s - by_cross).abs() <= 1e-12 * by_cross.max(1e-3),
            "({t1}°, {t2}°, γ = {gamma}°): s = {s}, by the cross product {by_cross}"
        );
    }
    // The control: B11's sine is not s. At a corner with γ = 45° one vector is rounding alone and the sine between them
    // is between about 0.27 and 1, so a sine test reads a plane there; s does not.
    let (q1, q2) = view(90.0, 90.0, 45.0, 1.0).basis();
    assert!(plane_s(&q1, &q2) < PLANE_S_CUT);
    rejects("the sine at a corner with γ = 45°", || {
        assert!(sine(&q1, &q2) < PLANE_S_CUT, "sine {}", sine(&q1, &q2))
    });
    // And a closed form with the tilts swapped for their cosines fails the same check.
    rejects("a wrong closed form", || {
        let (q1, q2) = view(20.0, 50.0, 0.0, 1.0).basis();
        let (a, b) = (20f64.to_radians().cos(), 50f64.to_radians().cos());
        assert!((plane_s(&q1, &q2).powi(2) - (1.0 - a * a * b * b)).abs() <= 34.0 * EPS);
    });
}

#[test]
fn qa_s_is_scale_free_and_two_r_for_unequal_norms() {
    let mut rng = Lcg(37);
    let mut bases: Vec<(Z, Z)> = (0..40)
        .map(|_| {
            let mut v = || std::array::from_fn(|_| 2.0 * rng.next() - 1.0);
            (v(), v())
        })
        .collect();
    bases.push(view(90.0, 89.97, 45.0, 1.0).basis());
    bases.push(view(90.0, 89.999, 30.0, 1.0).basis());
    bases.push((e(0, 1.0), e(1, 1e-4)));
    // Scaled together by k over the zoom's range and past it, powers of two and not, s stays to its rounding (B11's
    // 17ε on each computed s²).
    let ks: Vec<f64> = (0..=60)
        .map(|i| 2f64.powf(-50.0 + 0.913 * f64::from(i)))
        .collect();
    for (n, (q1, q2)) in bases.iter().enumerate() {
        let s = plane_s(q1, q2);
        for &k in &ks {
            let sk = plane_s(&q1.map(|x| k * x), &q2.map(|x| k * x));
            assert!(
                (sk * sk - s * s).abs() <= 2.0 * 17.0 * EPS,
                "basis {n}, k = {k}: s = {sk} against {s}"
            );
        }
    }
    // Orthogonal vectors of norm ratio r: s = 2r/(1 + r²) ≈ 2r (B11).
    for r in [0.5, 1e-2, 1e-4, 1e-6, 1e-8] {
        for (i, j) in [(0, 1), (3, 7), (5, 2)] {
            let s = plane_s(&e(i, 1.0), &e(j, r));
            assert!(
                (s / (2.0 * r / (1.0 + r * r)) - 1.0).abs() < 1e-14,
                "r = {r}, axes ({i}, {j}): s = {s}"
            );
        }
    }
    // The control: one vector scaled alone changes s.
    rejects("one vector scaled alone", || {
        let (q1, q2) = &bases[0];
        let sk = plane_s(&q1.map(|x| 3.0 * x), q2);
        assert!((sk * sk - plane_s(q1, q2).powi(2)).abs() <= 2.0 * 17.0 * EPS);
    });
}

// --- the guards ----------------------------------------------------------------------------------------------------

#[test]
fn qa_the_guards_cut_is_ten_to_minus_six() {
    assert_eq!(PLANE_S_CUT, 1e-6, "the guards' cut is B11's 10⁻⁶");
    // A typed orthogonal basis q₁ = e_x, q₂ = r e_y has no sine to speak of (its vectors are at 90°), so only the cut
    // can decline it: just under the cut (s = 2r/(1 + r²) < 10⁻⁶) both guards decline; just over it both return.
    let mut z = [0.0; 8];
    for (r, declined) in [
        (1e-7, true),
        (4.9e-7, true),
        (4.99e-7, true),
        (5.01e-7, false),
        (5.1e-7, false),
        (1e-6, false),
        (1e-4, false),
    ] {
        let p = Plane {
            z0: [0.0; 8],
            q1: e(0, 1.0),
            q2: e(1, r),
        };
        (z[0], z[1]) = (0.5, 0.2 * r);
        let what = format!("q₂ = {r}·e_y, s = {}", plane_s(&p.q1, &p.q2));
        // Before this task, the sine tests read a plane at every one of these.
        assert!(in_plane_sine_only(&p, &z).is_some(), "{what}");
        assert!(chart_coords_sine_only(&p, &z).is_some(), "{what}");
        if declined {
            declines(&p, &[z, p.z0], &what);
            figure_empty(&p, &what);
        } else {
            assert_eq!(in_plane(&p, &z), in_plane_sine_only(&p, &z), "{what}");
            assert_eq!(
                chart_coords(&p, &z),
                chart_coords_sine_only(&p, &z),
                "{what}"
            );
            assert!(
                in_plane(&p, &z).is_some() && chart_coords(&p, &z).is_some(),
                "{what}"
            );
        }
    }
    // The task's typed basis: q₁ = e_x, q₂ = 10⁻⁴ e_y, z₀ = 0, z = (0.5, 2·10⁻⁵, 0, …).
    let p = Plane {
        z0: [0.0; 8],
        q1: e(0, 1.0),
        q2: e(1, 1e-4),
    };
    (z[0], z[1]) = (0.5, 2e-5);
    assert_eq!(chart_coords(&p, &z), Some((0.75, 0.6)), "chart_coords");
    assert_eq!(in_plane(&p, &z), Some((0.5, 2e-5)), "in_plane");
    // Its out-of-plane part dropped: the same chart point with z off the plane, to B11's 17ε (sin θ = 1 here).
    let mut off = z;
    (off[3], off[6]) = (0.4, -0.9);
    let (s, t) = chart_coords(&p, &off).expect("a plane");
    assert!(
        (s - 0.75).abs() <= 17.0 * EPS && (t - 0.6).abs() <= 17.0 * EPS,
        "({s}, {t})"
    );
    // The control: the compass's 10⁻³ drawing cut, applied to the guards, would refuse this basis.
    rejects("the compass's cut on the typed basis", || {
        assert!(plane_s(&p.q1, &p.q2) >= 1e-3)
    });
    // And a cut a decade lower, 10⁻⁷, would have kept q₂ = 4.9·10⁻⁷ e_y, which the guards decline.
    rejects("a 10⁻⁷ cut", || {
        let thin = Plane {
            q2: e(1, 4.9e-7),
            ..p.clone()
        };
        assert!(chart_coords(&thin, &z).is_some())
    });
}

#[test]
fn qa_the_guards_decline_at_every_corner() {
    let points = [
        in_frame(0.1, 0.05, 0.2),
        [0.0; 8],
        in_frame(-0.6, 0.3, -0.4),
        [0.2, -0.1, 0.3, 0.05, -0.7, 0.4, 0.15, -0.25],
    ];
    let mut sine_would_pass = 0;
    for (t1, t2) in CORNERS {
        // γ = 0 and 45° as the task names, and B11's "every γ" besides.
        for gamma in (-12..=12).map(|k| 15.0 * f64::from(k)) {
            for sigma in sigma_sweep() {
                let what = format!("({t1}°, {t2}°, γ = {gamma}°), σ = {sigma}");
                for z0 in [[0.0; 8], in_frame(0.3, -0.2, 0.1)] {
                    let p = plane(t1, t2, gamma, sigma, z0);
                    // B11: a basis with no plane reads s at most √(17ε) ≈ 6.1·10⁻⁸.
                    let s = plane_s(&p.q1, &p.q2);
                    assert!(s <= (17.0 * EPS).sqrt(), "{what}: s = {s}");
                    declines(&p, &points, &what);
                    figure_empty(&p, &what);
                    if chart_coords_sine_only(&p, &points[0]).is_some() {
                        sine_would_pass += 1;
                    }
                }
            }
        }
    }
    // The failing control: at the corners with γ = 45° today's guards do not fire.
    for (t1, t2) in CORNERS {
        for sigma in sigma_sweep() {
            let p = plane(t1, t2, 45.0, sigma, [0.0; 8]);
            let what = format!("({t1}°, {t2}°, 45°), σ = {sigma}");
            rejects(&what, || declines_sine_only(&p, &points[0]));
        }
    }
    assert!(sine_would_pass > 0);
    // A plane off the corners is accepted, and the figure places its quads.
    let sound = plane(89.0, 90.0, 45.0, 1.0, [0.0; 8]);
    rejects("a sound plane", || declines(&sound, &points, "sound"));
    rejects("a sound plane's figure", || figure_empty(&sound, "sound"));
}

fn declines_sine_only(p: &Plane, z: &Z) {
    assert_eq!(in_plane_sine_only(p, z), None);
    assert_eq!(chart_coords_sine_only(p, z), None);
}

#[test]
fn qa_the_guards_return_todays_values_off_the_corners() {
    let mut rng = Lcg(2037);
    let mut tilts: Vec<(f64, f64, f64)> = vec![
        (90.0, 89.9, 0.0),
        (90.0, 89.97, 45.0),
        (90.0, 89.999, 45.0),
        (90.0, 89.999, 30.0),
        (-89.97, -90.0, 45.0),
        (89.999, -90.0, 0.0),
        (-90.0, 89.97, 120.0),
        (90.0, 0.0, 45.0),
        (0.0, 0.0, 0.0),
    ];
    for _ in 0..300 {
        tilts.push((
            180.0 * rng.next() - 90.0,
            180.0 * rng.next() - 90.0,
            360.0 * rng.next() - 180.0,
        ));
    }
    let points = [
        in_frame(0.1, 0.05, 0.2),
        in_frame(-0.6, 0.3, -0.4),
        [0.2, -0.1, 0.3, 0.05, -0.7, 0.4, 0.15, -0.25],
    ];
    let mut checked = 0;
    for (t1, t2, gamma) in tilts {
        for sigma in [0.01, 0.37, 1.0, 4.0, 2f64.powi(-48)] {
            let p = plane(t1, t2, gamma, sigma, in_frame(0.02, -0.03, 0.05));
            let s = plane_s(&p.q1, &p.q2);
            if s < PLANE_S_CUT {
                // Only the corner points themselves, which a random tilt does not land on.
                panic!("({t1}°, {t2}°, γ = {gamma}°): s = {s} off the corners");
            }
            let what = format!("({t1}°, {t2}°, γ = {gamma}°), σ = {sigma}, s = {s}");
            for z in points.iter().chain([&p.z0]) {
                let before = chart_coords_sine_only(&p, z);
                assert!(before.is_some(), "{what}: today's chart_coords declines");
                assert_eq!(chart_coords(&p, z), before, "{what}: chart_coords");
                assert_eq!(
                    in_plane(&p, z),
                    in_plane_sine_only(&p, z),
                    "{what}: in_plane"
                );
                assert!(in_plane(&p, z).is_some(), "{what}: in_plane declines");
                checked += 1;
            }
        }
    }
    assert!(checked > 1000);
    // The control: at a corner, the values are not today's.
    let corner = plane(-90.0, 90.0, 45.0, 1.0, [0.0; 8]);
    rejects("a corner", || {
        assert_eq!(
            chart_coords(&corner, &points[0]),
            chart_coords_sine_only(&corner, &points[0])
        )
    });
}

// --- locked ---------------------------------------------------------------------------------------------------------

fn z0_edits(edits: &[SetField]) -> usize {
    edits
        .iter()
        .filter(|e| matches!(e.edit, Edit::Sim(SimField::Z0(_))))
        .count()
}

#[test]
fn qa_locked_at_a_corner_a_basis_edit_sends_no_z0_edit() {
    let anchor = in_frame(0.1, 0.05, 0.2);
    // The review's case.
    let corner = plane(90.0, 90.0, 45.0, 1.0, [0.0; 8]);
    let to = view(89.0, 90.0, 45.0, 1.0).basis();
    let edit = basis_edit(to);
    let out = about_the_pin(vec![edit.clone()], &locked_on(&corner, anchor));
    assert_eq!(out, vec![edit.clone()], "the review's case");
    // It passes as it does free.
    let mut free = locked_on(&corner, anchor);
    free.lock.locked = false;
    assert_eq!(about_the_pin(vec![edit.clone()], &free), out);

    // The failing control: today's guards give the pivot t ≈ −2.887·10¹⁴ and a z₀ edit of about
    // (−7.13·10¹², 0, 6.22·10¹⁰) in the frame.
    let (s, t) = chart_coords_sine_only(&corner, &anchor).expect("today's guard reads a plane");
    assert!((t / -2.887e14 - 1.0).abs() < 5e-3, "t = {t}");
    let z0 = pivoted_z0(&corner, (s, t), &to);
    let (a, b) = PRESETS[0];
    assert!((z0[a] / -7.13e12 - 1.0).abs() < 5e-3, "{z0:?}");
    assert!(z0[b].abs() < 1e-3 * z0[a].abs(), "{z0:?}");
    assert!((z0[SLICE_AXIS] / 6.22e10 - 1.0).abs() < 5e-3, "{z0:?}");
    let today = vec![z0_edit(z0), edit.clone()];
    rejects("today's guards", || assert_eq!(z0_edits(&today), 0));

    // At each corner, γ = 0 and 45° and more, over the σ sweep, a tilt one degree back off each ±90° and off both, and
    // a turn of γ: no z₀ edit, the basis edit as it was.
    for (t1, t2) in CORNERS {
        for gamma in [0.0, 45.0, -30.0, 120.0] {
            for sigma in sigma_sweep() {
                let p = plane(t1, t2, gamma, sigma, in_frame(0.0, 0.0, 0.0));
                for to in [
                    (t1 - t1.signum(), t2, gamma),
                    (t1, t2 - t2.signum(), gamma),
                    (t1 - 3.0 * t1.signum(), t2 - 2.0 * t2.signum(), gamma),
                    (t1, t2, gamma + 10.0),
                ] {
                    let edit = basis_edit(view(to.0, to.1, to.2, sigma).basis());
                    let out = about_the_pin(vec![edit.clone()], &locked_on(&p, anchor));
                    assert_eq!(
                        out,
                        vec![edit],
                        "({t1}°, {t2}°, γ = {gamma}°), σ = {sigma}, to {to:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn qa_locked_inside_the_disc_a_drag_turns_about_the_pin() {
    for (t2, gamma, stated) in [
        (89.97, 45.0, 7.6e-15),
        (89.97, 0.0, 1.4e-8),
        (89.999, 30.0, 3.1e-6),
    ] {
        let what = format!("(90°, {t2}°, γ = {gamma}°)");
        let p = plane(90.0, t2, gamma, 1.0, [0.0; 8]);
        // Inside the compass's disc: under its 10⁻³ drawing cut, over the guards' 10⁻⁶.
        let s = plane_s(&p.q1, &p.q2);
        assert!((PLANE_S_CUT..1e-3).contains(&s), "{what}: s = {s}");
        let to = view(89.0, t2, gamma, 1.0).basis();
        let (sin_a, sin_b) = (sine(&p.q1, &p.q2), sine(&to.0, &to.1));
        let tolerance = 17.0 * EPS * (1.0 / (sin_a * sin_a) + 1.0 / (sin_b * sin_b));
        assert!(
            (tolerance / stated - 1.0).abs() < 0.1,
            "{what}: B11's tolerance {tolerance}, the task's {stated}"
        );
        let anchor = add(&add(&p.z0, 0.3, &p.q1), 0.2, &p.q2);
        let edit = basis_edit(to);
        let out = about_the_pin(vec![edit.clone()], &locked_on(&p, anchor));
        assert_eq!(
            out.len(),
            2,
            "{what}: a z₀ edit and the basis edit: {out:?}"
        );
        assert_eq!(out[1], edit, "{what}: the basis edit as it was");
        let Edit::Sim(SimField::Z0(z0)) = out[0].edit else {
            panic!("{what}: the first edit is not on z₀: {out:?}")
        };
        let held = |z0: Z| {
            let after = Plane {
                z0,
                q1: to.0,
                q2: to.1,
            };
            let (s, t) = chart_coords(&after, &anchor).expect("a plane after the drag");
            let off = (s - 0.65).hypot(t - 0.60);
            assert!(
                off <= tolerance,
                "{what}: the pin moved {off}, over {tolerance}"
            );
        };
        held(z0);
        // The control: the drag with no z₀ edit moves the pin off its chart point.
        rejects(&what, || held(p.z0));
    }
}

// --- end to end: a tilt drag on the compass of the running app ------------------------------------------------------

struct Rig {
    app: App<MockSide>,
    h: Headless,
}

impl Rig {
    fn new() -> Self {
        let mut rig = Self {
            app: App::new(MockSide::new(MockEngine::frozen(), None), FORMAT),
            h: Headless::new(SIZE, PIXELS_PER_POINT),
        };
        rig.idle(2);
        rig
    }

    fn idle(&mut self, n: usize) {
        for _ in 0..n {
            let _ = self.h.frame(&mut self.app, Vec::new());
        }
    }

    /// The SetFields the engine has applied, as its `contract` log entries, less the hover's.
    fn applied(&self) -> Vec<String> {
        self.app
            .console()
            .iter()
            .filter(|e| {
                e.source == Source::Contract && !e.message.starts_with("SetField Hover.at ")
            })
            .map(|e| e.message.clone())
            .collect()
    }

    /// The plane set to `p`, locked at `anchor`, straight on the engine.
    fn set(&mut self, p: &Plane, anchor: Z) {
        self.app.set_field(z0_edit(p.z0));
        self.app.set_field(basis_edit((p.q1, p.q2)));
        self.app.set_field(lock_edit(Lock {
            locked: true,
            z_locked: anchor,
        }));
        self.idle(3);
        let sim = &self.app.snapshot().sim;
        assert_eq!(sim.plane, *p, "the plane as set");
        assert!(
            sim.lock.locked && sim.lock.z_locked == anchor,
            "the lock as set"
        );
    }

    /// A horizontal drag of `dx` points on the compass's cube, in tilt mode (the default).
    fn tilt_drag(&mut self, dx: f32) {
        let square = compass::square(self.layout().compass);
        let from = square.center();
        let to = from + egui::vec2(dx, 0.0);
        let button = |pos: Pos2, pressed| Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        let _ = self.h.frame(&mut self.app, vec![Event::PointerMoved(from)]);
        let _ = self.h.frame(&mut self.app, vec![button(from, true)]);
        for i in 1..=8 {
            let at = from + (to - from) * (i as f32 / 8.0);
            let _ = self.h.frame(&mut self.app, vec![Event::PointerMoved(at)]);
        }
        let _ = self.h.frame(&mut self.app, vec![button(to, false)]);
        self.idle(3);
    }

    fn layout(&self) -> Layout {
        Layout::new(self.h.screen(), PIXELS_PER_POINT)
    }
}

#[test]
fn qa_app_locked_tilt_drag_at_a_corner_sends_no_z0_edit() {
    let anchor = in_frame(0.1, 0.05, 0.2);
    for (t1, t2) in CORNERS {
        for gamma in [0.0, 45.0] {
            for sigma in [0.01, 1.0, 4.0] {
                let what = format!("({t1}°, {t2}°, γ = {gamma}°), σ = {sigma}");
                let p = plane(t1, t2, gamma, sigma, [0.0; 8]);
                let mut rig = Rig::new();
                rig.set(&p, anchor);
                let before = rig.applied().len();
                // Off the corner: τ₁ back towards 0.
                rig.tilt_drag(-12.0 * t1.signum() as f32);
                let new: Vec<String> = rig.applied()[before..].to_vec();
                assert_eq!(new.len(), 1, "{what}: one SetField: {new:?}");
                assert!(
                    new[0].starts_with("SetField Plane.q1 Plane.q2 "),
                    "{what}: the basis edit: {new:?}"
                );
                let sim = &rig.app.snapshot().sim;
                assert_eq!(sim.plane.z0, p.z0, "{what}: z₀ moved");
                assert_ne!(
                    (sim.plane.q1, sim.plane.q2),
                    (p.q1, p.q2),
                    "{what}: no tilt"
                );
                // The control: the same check on a sound plane's locked drag, which does turn about the pin.
                if (t1, t2, gamma, sigma) == (90.0, 90.0, 45.0, 1.0) {
                    let sound = plane(60.0, 30.0, 45.0, 1.0, [0.0; 8]);
                    let mut rig = Rig::new();
                    rig.set(&sound, anchor);
                    let before = rig.applied().len();
                    rig.tilt_drag(-12.0);
                    let new: Vec<String> = rig.applied()[before..].to_vec();
                    rejects("a sound plane's locked drag", || {
                        assert_eq!(new.len(), 1, "{new:?}")
                    });
                }
            }
        }
    }
}

#[test]
fn qa_app_locked_tilt_drag_inside_the_disc_turns_about_the_pin() {
    for (t2, gamma) in [(89.97, 45.0), (89.97, 0.0), (89.999, 30.0)] {
        let what = format!("(90°, {t2}°, γ = {gamma}°)");
        let p = plane(90.0, t2, gamma, 1.0, [0.0; 8]);
        let anchor = add(&add(&p.z0, 0.3, &p.q1), 0.2, &p.q2);
        let mut rig = Rig::new();
        rig.set(&p, anchor);
        let before = rig.applied().len();
        rig.tilt_drag(-2.0);
        let new: Vec<String> = rig.applied()[before..].to_vec();
        assert_eq!(
            new.len(),
            2,
            "{what}: a z₀ edit and the basis edit: {new:?}"
        );
        assert!(new[0].starts_with("SetField Plane.z0 "), "{what}: {new:?}");
        assert!(
            new[1].starts_with("SetField Plane.q1 Plane.q2 "),
            "{what}: {new:?}"
        );
        let after = rig.app.snapshot().sim.plane.clone();
        let (sin_a, sin_b) = (sine(&p.q1, &p.q2), sine(&after.q1, &after.q2));
        let tolerance = 17.0 * EPS * (1.0 / (sin_a * sin_a) + 1.0 / (sin_b * sin_b));
        let (s, t) = chart_coords(&after, &anchor).expect("a plane after the drag");
        let off = (s - 0.65).hypot(t - 0.60);
        assert!(
            off <= tolerance,
            "{what}: the pin moved {off}, over {tolerance}"
        );
        // The control: the drag without its z₀ edit moves the pin.
        rejects(&what, || {
            let unpinned = Plane {
                z0: p.z0,
                ..after.clone()
            };
            let (s, t) = chart_coords(&unpinned, &anchor).expect("a plane");
            assert!((s - 0.65).hypot(t - 0.60) <= tolerance);
        });
    }
}
