//! Navigation as chart construction (chart_decoder_contract Part 4; chart_reference §1.1; R-83; TASK-M2-22). There is
//! no view separate from the chart: the view state is `SimConfig`'s plane `(z₀, q₁, q₂)`, and every gesture here is a
//! CPU-side computation of one typed [`SetField`] edit of it, nothing else (Part 4, "What the GPU knows").
//!
//! | Gesture | Edit | Constraint (Part 4, "The operation table") |
//! |---|---|---|
//! | [`pan`] | `z₀` | `Δz₀ ∈ span(q₁, q₂)`: the same plane, another window |
//! | [`slice`] | `z₀` | `Δz₀ ⊥ span(q₁, q₂)`, `z₀ ← z₀ + t·d`: every pixel's IC moves by the same hidden step |
//! | [`zoom`] | `q₁, q₂` | one common factor, log-stepped: `2^−octaves` |
//! | [`ChartConstructor::tilt`] | `q₁` or `q₂` | `q′(τ) = normalise(cos τ·q + sin τ·d_target)`, `τ ∈ [−90°, +90°]` |
//!
//! **The scale lives in `q`** (R-83): a zoomed basis vector is `k·q̂`. A tilt rotates the direction and keeps each basis
//! vector's length, `q′ = ‖q‖·normalise(cos τ·q̂ + sin τ·d̂)`, so a tilt neither resets the zoom nor gives the two axes
//! different scales; for a unit `q` it is Part 4's formula exactly (applied per R-369: the mechanical consequence of R-83
//! for the tilt formula, written when `q` was unit).
//!
//! **Raw tilts replace rather than compose** (chart_reference §1.1): a [`ChartConstructor`] holds the committed plane,
//! and each raw tilt is of that plane, so a second tilt replaces the first; [`ChartConstructor::commit`] is the commit
//! mechanism that makes the current plane the one later tilts start from.
//!
//! **An oblique slice records its pair** (§1.1): [`oblique`] orthonormalises two vectors by Gram–Schmidt into the
//! `(q₁, q₂)` the plane holds, so `SimConfig`'s export carries the pair used.
//!
//! The directions offered to slice along and tilt toward, the six orthogonal to the plane and the named compound
//! directions evaluated at the centre, are [`directions`].

pub mod directions;

use crate::contract::set_field::{Edit, SetField, SimField};
use crate::contract::sim_config::{Latent, Plane};

/// Why a gesture or a direction gives no edit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavError {
    /// An input is not finite.
    NonFinite,
    /// A tilt angle outside `[−90°, +90°]` (Part 4, "Tilt").
    TauOutOfRange,
    /// A vector to be normalised has length 0.
    ZeroVector,
    /// `q₁` and `q₂` do not span a plane: one is 0, or they are parallel.
    DegeneratePlane,
    /// The morph's source: neither basis vector has a configuration component (Part 4, REQ-CHART-051).
    NoConfiguration,
    /// A named direction is unavailable at this centre ([`directions::Unavailable`]).
    Unavailable(directions::Unavailable),
}

/// One of the plane's two basis vectors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BasisVector {
    /// `q₁`, along `s`.
    Q1,
    /// `q₂`, along `t`.
    Q2,
}

/// `a · b`.
pub(crate) fn dot(a: &Latent, b: &Latent) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// `‖a‖`.
pub(crate) fn norm(a: &Latent) -> f64 {
    dot(a, a).sqrt()
}

/// `a + k·b`.
pub(crate) fn axpy(a: &Latent, k: f64, b: &Latent) -> Latent {
    std::array::from_fn(|i| a[i] + k * b[i])
}

/// `k·a`.
fn scaled(a: &Latent, k: f64) -> Latent {
    a.map(|x| k * x)
}

/// `a / ‖a‖`: [`NavError::NonFinite`] for a non-finite `a`, [`NavError::ZeroVector`] for `a = 0`.
pub(crate) fn unit(a: &Latent) -> Result<Latent, NavError> {
    unit_of(a, 0.0)
}

/// `a / ‖a‖` for an `a` left by cancelling terms of size `size`: [`NavError::ZeroVector`] where `‖a‖ ≤ √ε·size`, `ε`
/// the f64 machine epsilon, below which the direction would keep fewer than half its digits, rounding rather than
/// geometry (applied per R-369: the conditioning guard of a Gram–Schmidt step and of the tilt's sum).
pub(crate) fn unit_of(a: &Latent, size: f64) -> Result<Latent, NavError> {
    if !a.iter().all(|x| x.is_finite()) {
        return Err(NavError::NonFinite);
    }
    let n = norm(a);
    if n <= f64::EPSILON.sqrt() * size {
        return Err(NavError::ZeroVector);
    }
    Ok(scaled(a, 1.0 / n))
}

/// [`NavError::NonFinite`] unless every value is finite.
fn finite(values: &[f64]) -> Result<(), NavError> {
    match values.iter().all(|x| x.is_finite()) {
        true => Ok(()),
        false => Err(NavError::NonFinite),
    }
}

/// A `SimConfig` edit, entering the undo history (R-69).
fn edit(field: SimField) -> SetField {
    SetField {
        edit: Edit::Sim(field),
        no_history: false,
    }
}

/// The plane's orthonormal frame `(û₁, û₂)`: Gram–Schmidt of `q₁`, then `q₂`. [`NavError::DegeneratePlane`] where they
/// span no plane: `q₁ = 0`, or `q₂`'s part orthogonal to `q₁` at most `√ε·‖q₂‖` ([`unit_of`]).
pub fn frame(plane: &Plane) -> Result<[Latent; 2], NavError> {
    let degenerate = |e| match e {
        NavError::ZeroVector => NavError::DegeneratePlane,
        e => e,
    };
    let u1 = unit(&plane.q1).map_err(degenerate)?;
    let u2 = unit_of(&axpy(&plane.q2, -dot(&plane.q2, &u1), &u1), norm(&plane.q2))
        .map_err(degenerate)?;
    Ok([u1, u2])
}

/// `v` with its component in `span(q₁, q₂)` removed: the part of `v` orthogonal to the plane.
pub fn off_plane(plane: &Plane, v: &Latent) -> Result<Latent, NavError> {
    let [u1, u2] = frame(plane)?;
    let v = axpy(v, -dot(v, &u1), &u1);
    Ok(axpy(&v, -dot(&v, &u2), &u2))
}

/// Pan by `(ds, dt)` of the view's unsigned `[0,1]²` addresses (Part 4, "Pan"): `Δz₀ = 2ds·q₁ + 2dt·q₂ ∈ span(q₁, q₂)`,
/// so the pixel at `(½ + ds, ½ + dt)` becomes the centre and the plane is the same.
pub fn pan(plane: &Plane, ds: f64, dt: f64) -> Result<SetField, NavError> {
    finite(&[ds, dt])?;
    let z0 = axpy(&axpy(&plane.z0, 2.0 * ds, &plane.q1), 2.0 * dt, &plane.q2);
    Ok(edit(SimField::Z0(z0)))
}

/// Slice by `t` along the hidden direction `d` (Part 4, "Slice"): `z₀ ← z₀ + t·d⊥`, `d⊥` the part of `d` orthogonal to
/// the plane, so `Δz₀ ⊥ span(q₁, q₂)` and the plane translates parallel to itself. A direction orthogonal to the plane,
/// as the six of [`directions::hidden`] are, is `d⊥ = d`; a slider's direction in general mixes a pan with a slice, and
/// its slice is the orthogonal component (Part 4, "Pan and slice are the same operation").
pub fn slice(plane: &Plane, d: &Latent, t: f64) -> Result<SetField, NavError> {
    finite(&[t])?;
    finite(d)?;
    let step = off_plane(plane, d)?;
    Ok(edit(SimField::Z0(axpy(&plane.z0, t, &step))))
}

/// Zoom by `octaves` about the centre (Part 4, "Zoom"; R-83): `q₁` and `q₂` scaled by the one common factor
/// `2^−octaves`, log-stepped, `z₀` kept; a positive `octaves` narrows the window.
pub fn zoom(plane: &Plane, octaves: f64) -> Result<SetField, NavError> {
    finite(&[octaves])?;
    let k = (-octaves).exp2();
    if k == 0.0 || !k.is_finite() {
        return Err(NavError::DegeneratePlane);
    }
    Ok(edit(SimField::Basis {
        q1: scaled(&plane.q1, k),
        q2: scaled(&plane.q2, k),
    }))
}

/// `(cos τ, sin τ)` of `tau_deg` degrees, [`NavError::TauOutOfRange`] outside `[−90°, +90°]`.
fn angle(tau_deg: f64) -> Result<(f64, f64), NavError> {
    finite(&[tau_deg])?;
    if !(-90.0..=90.0).contains(&tau_deg) {
        return Err(NavError::TauOutOfRange);
    }
    let (sin, cos) = tau_deg.to_radians().sin_cos();
    Ok((cos, sin))
}

/// A curve axis's tilt at the centre (Part 4, "Directions are axis kinds"): `q_tilt(τ) = length·normalise(cos τ·γ̂′ +
/// sin τ·d̂)`, `γ′ = tangent`, the curve's tangent at the centre's `ν₀`, and `d = target`, each taken as its direction.
/// `τ = 0` is the tangent's direction, `τ = ±90°` is `±d̂`; the sign matters.
pub fn tilt_tangent(
    tangent: &Latent,
    length: f64,
    tau_deg: f64,
    target: &Latent,
) -> Result<Latent, NavError> {
    finite(&[length])?;
    let (cos, sin) = angle(tau_deg)?;
    let along = unit(tangent)?;
    let toward = unit(target)?;
    let turned = unit_of(&axpy(&scaled(&along, cos), sin, &toward), 1.0)?;
    Ok(scaled(&turned, length))
}

/// A raw basis vector's tilt (Part 4, "Tilt"): `q′(τ) = ‖q‖·normalise(cos τ·q̂ + sin τ·d̂)`, the length kept (the
/// module's note on R-83).
pub fn tilt_vector(q: &Latent, tau_deg: f64, target: &Latent) -> Result<Latent, NavError> {
    tilt_tangent(q, norm(q), tau_deg, target)
}

/// The chart constructor (chart_reference §1.1): the committed plane, which every raw tilt starts from, so raw tilts
/// replace rather than compose; [`commit`](Self::commit) makes a plane the new start.
#[derive(Clone, Debug, PartialEq)]
pub struct ChartConstructor {
    committed: Plane,
}

impl ChartConstructor {
    /// A constructor whose committed plane is `plane`.
    pub fn new(plane: Plane) -> Self {
        Self { committed: plane }
    }

    /// The committed plane.
    pub fn committed(&self) -> &Plane {
        &self.committed
    }

    /// Commits `plane`: later raw tilts start from it.
    pub fn commit(&mut self, plane: Plane) {
        self.committed = plane;
    }

    /// The raw tilt of `which` by `tau_deg` toward `target`, as one basis edit: the committed plane's direction of
    /// `which` turned by [`tilt_tangent`], the other basis vector the committed one's direction, each at the length it
    /// has in `current`, so the zoom since the commit is kept and an earlier raw tilt is replaced, not composed.
    /// [`NavError::DegeneratePlane`] where the tilted pair spans no plane.
    pub fn tilt(
        &self,
        current: &Plane,
        which: BasisVector,
        tau_deg: f64,
        target: &Latent,
    ) -> Result<SetField, NavError> {
        let base = &self.committed;
        let (tilted, kept, kept_length) = match which {
            BasisVector::Q1 => (&base.q1, &base.q2, norm(&current.q2)),
            BasisVector::Q2 => (&base.q2, &base.q1, norm(&current.q1)),
        };
        let length = norm(match which {
            BasisVector::Q1 => &current.q1,
            BasisVector::Q2 => &current.q2,
        });
        let turned = tilt_tangent(tilted, length, tau_deg, target)?;
        let kept = scaled(&unit(kept)?, kept_length);
        let (q1, q2) = match which {
            BasisVector::Q1 => (turned, kept),
            BasisVector::Q2 => (kept, turned),
        };
        frame(&Plane {
            z0: current.z0,
            q1,
            q2,
        })?;
        Ok(edit(SimField::Basis { q1, q2 }))
    }
}

/// An oblique slice through `z0` (chart_reference §1.1, "Arbitrary / oblique"): `q₁, q₂` the orthonormal pair
/// Gram–Schmidt makes of `a` then `b` (two Gaussian draws, say). The plane holds the pair, so the exported `SimConfig`
/// records it.
pub fn oblique(z0: Latent, a: &Latent, b: &Latent) -> Result<Plane, NavError> {
    finite(&z0)?;
    let [q1, q2] = frame(&Plane { z0, q1: *a, q2: *b })?;
    Ok(Plane { z0, q1, q2 })
}
