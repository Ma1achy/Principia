//! TASK-M2-29, qa: the mass floors and the `wᵢⱼ` bound colour_composition §6 rests the conditioning term's finiteness
//! on, written from §6's text, not from the preset. Plain f64 arithmetic on the kernel's Burrau embed; no GPU, no tier.
//!
//! - `qa_m229_burrau_floors`: over the Euclid plane's default `ν ∈ [1/32, 31/32]` and the acute-angle axis from the
//!   `θ_min` of `ν = 31/32` to `π/4`, the embed's masses are §6's `(1 + ν²)/(2 + 2ν)`, `ν/(1 + ν)`, `(1 − ν)/2`; the first
//!   is at least `√2 − 1`; `ν(θ) = cos θ/(1 + sin θ)` lies on `[√2 − 1, 1)` and falls as `θ` grows; the least mass over
//!   either range is §6's closed form, `1/64`. Control: a claimed floor of `1/32` must fail.
//! - `qa_m229_separation_scale_bound`: with the centre of mass at 0 and `I = 1`, `wᵢⱼ = ‖rᵢ‖∞ + ‖rⱼ‖∞ ≥ √m_k/2` and
//!   `‖rᵢ‖ ≤ 1/√mᵢ`, on fuzzed systems and the bound's tight case (body `i` holding nearly all the mass, body `j` at the
//!   origin, the line at 45°), with the least mass down to `2⁻⁶²`, where `√m_k/2` is `agreement::FLOOR`, so `δ` is inert
//!   as §6 says. Control: the coefficient `3/4` for §6's `1/2` must fail on the tight case.

use kernel::chart::burrau::{acute_angle, embed};
use render::presets::agreement;
use std::f64::consts::{FRAC_PI_4, SQRT_2};
use validation::negative_control;

/// §6's least mass at the Euclid plane's default range and the acute-angle axis's proposed `θ_min`.
const FLOOR_1_64: f64 = 1.0 / 64.0;

/// `ν(θ)`, chart_reference §4.3's inverse of the acute angle.
fn nu_of_theta(theta: f64) -> f64 {
    theta.cos() / (1.0 + theta.sin())
}

/// §6's masses at `ν`, against the embed's, and their least.
fn checked_least(nu: f64) -> f64 {
    let m = embed(nu).m;
    let want = [
        (1.0 + nu * nu) / (2.0 + 2.0 * nu),
        nu / (1.0 + nu),
        (1.0 - nu) / 2.0,
    ];
    for i in 0..3 {
        assert!(
            (m[i] - want[i]).abs() <= 1e-15,
            "ν = {nu}: m[{i}] = {} against §6's {}",
            m[i],
            want[i]
        );
    }
    assert!(
        want[0] >= SQRT_2 - 1.0 - 1e-15,
        "ν = {nu}: the first mass {} is under √2 − 1",
        want[0]
    );
    m[0].min(m[1]).min(m[2])
}

fn check_floors(claimed: f64) {
    const N: usize = 4096;
    let (lo, hi) = (1.0f64 / 32.0, 31.0f64 / 32.0);
    let closed = (lo / (1.0 + lo)).min((1.0 - hi) / 2.0);
    assert!(
        (closed - FLOOR_1_64).abs() <= 1e-15,
        "the closed form {closed} is not 1/64"
    );

    let mut least = f64::INFINITY;
    for i in 0..=N {
        least = least.min(checked_least(lo + (hi - lo) * i as f64 / N as f64));
    }
    assert!(
        (least - closed).abs() <= 1e-12,
        "Euclid plane: the least mass {least} is not {closed}"
    );
    assert!(
        least >= claimed - 1e-12,
        "Euclid plane: the least mass {least} is under the floor {claimed}"
    );

    let theta_min = acute_angle(hi);
    assert!(
        (nu_of_theta(theta_min) - hi).abs() <= 1e-14,
        "ν(θ) does not invert the acute angle"
    );
    assert!(
        (nu_of_theta(FRAC_PI_4) - (SQRT_2 - 1.0)).abs() <= 1e-15,
        "ν(π/4) is not √2 − 1"
    );
    let (mut least, mut last) = (f64::INFINITY, f64::INFINITY);
    for i in 0..=N {
        let nu = nu_of_theta(theta_min + (FRAC_PI_4 - theta_min) * i as f64 / N as f64);
        assert!(
            nu < last && (SQRT_2 - 1.0 - 1e-15..1.0).contains(&nu),
            "ν(θ) = {nu}, after {last}"
        );
        last = nu;
        least = least.min(checked_least(nu));
    }
    let closed = (1.0 - nu_of_theta(theta_min)) / 2.0;
    assert!(
        (least - closed).abs() <= 1e-12,
        "acute angle: the least mass {least} is not {closed}"
    );
    assert!(
        least >= claimed - 1e-12,
        "acute angle: the least mass {least} is under the floor {claimed}"
    );
}

#[test]
fn qa_m229_burrau_floors() {
    check_floors(FLOOR_1_64);
}

negative_control!(
    qa_m229_burrau_floors,
    "a claimed floor of 1/32 is above the ranges' least mass",
    expected = "is under the floor",
    check_floors(1.0 / 32.0)
);

/// A system moved to the centre of mass and scaled to `I = 1`.
fn normalised(m: [f64; 3], mut r: [[f64; 2]; 3]) -> [[f64; 2]; 3] {
    for axis in 0..2 {
        let c: f64 = (0..3).map(|i| m[i] * r[i][axis]).sum();
        for p in &mut r {
            p[axis] -= c;
        }
    }
    let inertia: f64 = (0..3)
        .map(|i| m[i] * (r[i][0].powi(2) + r[i][1].powi(2)))
        .sum();
    r.map(|p| p.map(|x| x / inertia.sqrt()))
}

fn norm_inf(p: [f64; 2]) -> f64 {
    p[0].abs().max(p[1].abs())
}

fn check_system(m: [f64; 3], r: [[f64; 2]; 3], coefficient: f64) {
    for k in 0..3 {
        let (i, j) = ((k + 1) % 3, (k + 2) % 3);
        let w = norm_inf(r[i]) + norm_inf(r[j]);
        let bound = coefficient * m[k].sqrt();
        // The one rounding allowance: the systems are f64, at relative eps.
        assert!(
            w >= bound * (1.0 - 1e-12),
            "m = {m:?}, r = {r:?}: w = {w} under the bound {bound} at k = {k}"
        );
        let euclid = (r[k][0].powi(2) + r[k][1].powi(2)).sqrt();
        assert!(
            euclid <= (1.0 + 1e-12) / m[k].sqrt(),
            "m = {m:?}: ‖r[{k}]‖ = {euclid} over 1/√m"
        );
    }
}

/// The bound's tight case: body 0 holds nearly all the mass, body 1 lies at the origin, and bodies 0 and 2 on the 45°
/// line, so `w₀₁ = ‖r₀‖∞` is close to `√(m₂/2)`.
fn tight(e: f64) -> ([f64; 3], [[f64; 2]; 3]) {
    let m = [1.0 - 2.0 * e, e, e];
    let along = -m[0] / m[2];
    let scale = 1.0 / (m[0] + m[2] * along * along).sqrt() / SQRT_2;
    (
        m,
        [[scale, scale], [0.0, 0.0], [along * scale, along * scale]],
    )
}

fn check_bound(coefficient: f64) {
    for e in [2f64.powi(-62), 2.5e-13, 2e-9, 1e-4, 1.0 / 64.0] {
        let (m, r) = tight(e);
        check_system(m, r, coefficient);
    }
    let mut x = 0x9E37_79B9_7F4A_7C15u64;
    let mut unit = move || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        (x >> 11) as f64 / (1u64 << 53) as f64
    };
    for _ in 0..20_000 {
        // Log-uniform masses from 2⁻⁶² up, summing to 1.
        let a = 2f64.powf(-62.0 * unit());
        let b = 2f64.powf(-62.0 * unit());
        let total = 1.0 + a + b;
        let m = [1.0 / total, a / total, b / total];
        let mut c = || 2.0 * unit() - 1.0;
        let r = [[c(), c()], [c(), c()], [c(), c()]];
        // Every labelling, so the small masses fall on each body in turn.
        check_system(m, normalised(m, r), coefficient);
        let m = [m[1], m[2], m[0]];
        check_system(m, normalised(m, r), coefficient);
    }
}

#[test]
fn qa_m229_separation_scale_bound() {
    check_bound(0.5);
    // At the least mass §6 names, √m_k/2 is δ: no such system's floor reaches δ.
    assert_eq!(
        2f64.powi(-62).sqrt() / 2.0,
        f64::from(agreement::FLOOR),
        "√(2⁻⁶²)/2 is not agreement::FLOOR"
    );
}

negative_control!(
    qa_m229_separation_scale_bound,
    "the coefficient 3/4 is over the tight case's 1/√2",
    expected = "under the bound",
    check_bound(0.75)
);
