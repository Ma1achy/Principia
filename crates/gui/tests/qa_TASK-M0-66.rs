//! QA's tests for TASK-M0-66, written from REQ-TOOL-238: "a test must not pin a value only rounding residue sets". The
//! compass's corner test now reads the sine of the angle between `q₁` and `q₂`, and `s`, at `τ = (89.98°, 89.98°)`,
//! `γ = 45°`, in place of the corner `τ = (90°, 90°)`. A value set by the angles barely moves when its inputs move by
//! a few ulps; one set by rounding residue moves by its own size. So the check here nudges `τ₁` and `τ₂` by up to four
//! ulps each way and asks that the sine and `s` stay put, and that `s` is the closed form the angles give,
//! `s = cos τ · √(1 + sin² τ)` for `τ₁ = τ₂ = τ`, `γ = 45°`, at every scale (from `View::basis`, R-451 B11's `s`).
//! The negative control (R-176), run in the test: the same check at the corner itself, which only residue sets, fails.

use gui::explore::compass::{cube_s, frame_basis, P3};
use gui::explore::manifold_view::{View, PRESETS, SLICE_AXIS};

const SIGMAS: [f64; 6] = [0.01, 0.1, 0.5, 1.0, 2.0, 4.0];

/// The view `(τ₁, τ₂)` in radians, `γ = 45°`, at scale `σ`, in the first preset's frame.
fn view(tau1: f64, tau2: f64, sigma: f64) -> View {
    View {
        preset: Some(0),
        a: PRESETS[0].0,
        b: PRESETS[0].1,
        d: SLICE_AXIS,
        x: SLICE_AXIS,
        y: SLICE_AXIS,
        scale: sigma,
        tau1,
        tau2,
        gamma: 45f64.to_radians(),
        map: engine::contract::sim_config::TiltMap::Lean,
    }
}

fn cross(a: P3, b: P3) -> P3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn norm(a: P3) -> f64 {
    a.iter().map(|x| x * x).sum::<f64>().sqrt()
}

/// The sine of the angle between `q₁` and `q₂`, and `s`, of `view`.
fn sine_and_s(view: &View) -> (f64, f64) {
    let (q1, q2) = frame_basis(view);
    (norm(cross(q1, q2)) / (norm(q1) * norm(q2)), cube_s(q1, q2))
}

/// `x` moved by `k` ulps.
fn ulps(x: f64, k: i64) -> f64 {
    f64::from_bits((x.to_bits() as i64 + k) as u64)
}

/// At `τ₁ = τ₂ = tau` (radians), every scale: the sine and `s` move by less than 10⁻⁹ of themselves when `τ₁` and
/// `τ₂` move by up to four ulps, and `s` is the angles' closed form to 10⁻⁹.
fn check_set_by_the_angles(tau: f64) {
    for sigma in SIGMAS {
        let (sine, s) = sine_and_s(&view(tau, tau, sigma));
        for k1 in -4..=4 {
            for k2 in -4..=4 {
                let (nudged_sine, nudged_s) =
                    sine_and_s(&view(ulps(tau, k1), ulps(tau, k2), sigma));
                assert!(
                    (nudged_sine - sine).abs() <= 1e-9 * sine.abs()
                        && (nudged_s - s).abs() <= 1e-9 * s.abs(),
                    "rounding residue sets the value at τ = {tau}, σ {sigma}: sine {sine} → {nudged_sine}, \
                     s {s} → {nudged_s} for ({k1}, {k2}) ulps"
                );
            }
        }
        let closed = tau.cos() * (1.0 + tau.sin().powi(2)).sqrt();
        assert!(
            (s - closed).abs() <= 1e-9 * closed,
            "s at τ = {tau}, σ {sigma} reads {s}, the angles give {closed}"
        );
    }
}

#[test]
fn qa_the_near_corner_values_are_set_by_the_angles() {
    let tau = 89.98f64.to_radians();
    check_set_by_the_angles(tau);
    // And they are the values the compass test reads: the sine is 1 and s is under the draw cut (10⁻³).
    for sigma in SIGMAS {
        let (sine, s) = sine_and_s(&view(tau, tau, sigma));
        assert!(
            (sine - 1.0).abs() < 1e-9 && s < 1e-3 && s > 4e-4,
            "σ {sigma}: sine {sine}, s {s}"
        );
    }
    // The control: at the corner itself only residue sets them, and the check says so.
    let corner = std::panic::catch_unwind(|| check_set_by_the_angles(90f64.to_radians()));
    assert!(
        corner.is_err(),
        "the check passed at the corner τ = (90°, 90°), where only rounding residue sets the sine"
    );
}
