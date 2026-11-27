//! Saturate: the coordinate squashed toward a true end it never reaches (chart_reference §5.4, "Saturate on a
//! zero-mass end").
//!
//! With `p` measured outward (`σ` times the axis coordinate), `p_s` where the squash starts and `p₀` where it would
//! end, `Δ = p₀ − p_s > 0` and `d` the pixel's outward distance past `p_s`, the pixel shows the formula at
//! `p = p_s + Δ·d/(Δ + d)`: `C¹` at `p_s`, strictly monotone, approaching `p₀` as `1/d`. Evaluate masses from the
//! remaining distance `δ = Δ²/(Δ + d)`, never from the rounded `p`, which reaches `p₀` at a finite `d`: each mass is
//! its value at `p₀` plus its increment `−δ·rate`, so no difference of nearly equal values is formed and the binding
//! mass is its floor plus a product, never a rounded zero.

use super::{Count, Kind, Resolved, Sheet};

/// The mass block's `ε_M01` fence, from the register.
pub const EPS_M01: f64 = <f64 as kernel::generated::constants::ChartConstants>::EPS_M01;

/// One edge's squash, in outward coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Squash {
    /// `+1` past the axis's maximum, `−1` past its minimum.
    pub sigma: f64,
    /// Where it starts, outward.
    pub p_s: f64,
    /// Where it would end, outward, past `p_s`.
    pub p0: f64,
}

/// A squashed pixel: the coordinate to decode and the distance `δ` it still has to `p₀`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Squashed {
    /// The axis coordinate `σ·p`, rounded.
    pub coord: f64,
    /// `δ = p₀ − p`, the remaining distance, evaluated without the rounded `p`.
    pub remaining: f64,
}

impl Squash {
    /// `Δ = p₀ − p_s`.
    pub fn width(&self) -> f64 {
        self.p0 - self.p_s
    }

    /// The pixel at axis coordinate `x`: `None` before the squash starts (the formula is affine there).
    pub fn at(&self, x: f64) -> Option<Squashed> {
        let d = self.sigma * x - self.p_s;
        if d <= 0.0 {
            return None;
        }
        let w = self.width();
        let p = self.p_s + w * d / (w + d);
        Some(Squashed {
            coord: self.sigma * p,
            remaining: w * w / (w + d),
        })
    }
}

/// The pixel at `at` with its coordinate `axis` squashed by `sq` under `kind`; the squashed pixel is counted where
/// its system is not shown inside the square, which the edge declares (`counted`).
pub fn resolve(
    at: [f64; 2],
    axis: usize,
    sq: &Squash,
    kind: Kind,
    counted: bool,
) -> (Resolved, Option<Squashed>) {
    match (kind, sq.at(at[axis])) {
        (Kind::Blank, Some(_)) => (Resolved::blank(at), None),
        (Kind::Saturate, Some(s)) => {
            let mut p = at;
            p[axis] = s.coord;
            (
                Resolved {
                    at: p,
                    sheet: Sheet::One,
                    applied: Some(Kind::Saturate),
                    count: if counted {
                        Count::Saturated
                    } else {
                        Count::Repeat
                    },
                },
                Some(s),
            )
        }
        _ => (Resolved::own(at), None),
    }
}

/// How the squash is bounded where a mass is already at or below `ε_M01` at the edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Band {
    /// The first floor any mass crosses: `ε_M01` for a mass above it at `p_s`, 0 for one at or below it, `M₀₁`'s
    /// `ε_M01`.
    First,
    /// Every mass taken to `ε_M01`, whatever it is at `p_s`: on a mass below `ε_M01` at `p_s` this gives `p₀` before
    /// `p_s` and so a negative `Δ`, which folds the pixel inward. Kept as the control's variant, never used.
    EpsEverywhere,
}

/// The squash of masses affine in the outward distance `x` from the edge, `mᵢ(x) = mᵢ + bᵢ·x`, with `M₀₁ = m₀ + m₁`:
/// `p_s` where the smallest decreasing mass reaches `ε_m` (the edge itself if one is already below), `p₀` the first
/// floor crossed past it ([`Band`]), `eps_m` the ternary plot's interior buffer `ε_m` (the register's
/// `mass_simplex_eps_m`). `None` where no mass decreases.
pub fn affine_band(m: [f64; 3], b: [f64; 3], eps_m: f64, band: Band) -> Option<(f64, f64)> {
    let at = |i: usize, x: f64| m[i] + b[i] * x;
    let m01 = |x: f64| at(0, x) + at(1, x);
    let hits =
        |from: f64, level: f64, i: usize| (b[i] < 0.0).then(|| from + (level - at(i, from)) / b[i]);
    let mut p_s = f64::INFINITY;
    for i in 0..3 {
        if b[i] < 0.0 {
            let x = if m[i] <= eps_m {
                0.0
            } else {
                (eps_m - m[i]) / b[i]
            };
            p_s = p_s.min(x);
        }
    }
    if !p_s.is_finite() {
        return None;
    }
    let mut p0 = f64::INFINITY;
    for i in 0..3 {
        let floor = match band {
            Band::EpsEverywhere => EPS_M01,
            Band::First if at(i, p_s) > EPS_M01 => EPS_M01,
            Band::First => 0.0,
        };
        if let Some(x) = hits(p_s, floor, i) {
            p0 = p0.min(x);
        }
    }
    if m01(p_s) > EPS_M01 && b[0] + b[1] < 0.0 {
        p0 = p0.min(p_s + (EPS_M01 - m01(p_s)) / (b[0] + b[1]));
    }
    p0.is_finite().then_some((p_s, p0))
}

/// The masses of an affine edge at the squashed pixel: each is its value at `p₀` plus its increment `−bᵢ·δ`, the
/// binding mass (the first floor crossed) exactly its floor plus that product.
pub fn affine_masses(m: [f64; 3], b: [f64; 3], p0: f64, remaining: f64) -> [f64; 3] {
    [0, 1, 2].map(|i| (m[i] + b[i] * p0) - b[i] * remaining)
}

/// `ν`'s masses (Burrau, §4.2) at `ν = ν₀ + dν`: `m₁ = ν/(1+ν)`, `m₂ = (1−ν)/2`, `m₀ = (1+ν²)/(2(1+ν))`, each its
/// value at `ν₀` (`at0`, the binding mass given exactly) plus the closed-form increment.
pub fn nu_masses(nu0: f64, at0: [f64; 3], d_nu: f64) -> [f64; 3] {
    let nu = nu0 + d_nu;
    let k = d_nu / ((1.0 + nu0) * (1.0 + nu));
    [
        at0[0] + k * (nu0 * nu + nu0 + nu - 1.0) / 2.0,
        at0[1] + k,
        at0[2] - d_nu / 2.0,
    ]
}

/// The acute-angle axis's masses at `θ = θ₀ + dθ`, `t = tan(θ/2)`: `m₂ = t/(1+t)`, `m₁ = (1−t)/2`,
/// `m₀ = (1+t²)/(2(1+t))`, each its value at `θ₀` plus the increment through `t − t₀ = sin(dθ/2)/(cos(θ/2)cos(θ₀/2))`.
pub fn theta_masses(theta0: f64, at0: [f64; 3], d_theta: f64) -> [f64; 3] {
    let t0 = libm::tan(theta0 / 2.0);
    let dt =
        libm::sin(d_theta / 2.0) / (libm::cos((theta0 + d_theta) / 2.0) * libm::cos(theta0 / 2.0));
    let t = t0 + dt;
    let k = dt / ((1.0 + t0) * (1.0 + t));
    [
        at0[0] + k * (t0 * t + t0 + t - 1.0) / 2.0,
        at0[1] - dt / 2.0,
        at0[2] + k,
    ]
}
