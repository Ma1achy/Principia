//! Navigation's geometry (chart_decoder_contract Part 4; chart_reference §1.1): the plane's frame, the part of a vector
//! off the plane, the vector arithmetic, and the direction library ([`directions`]) a slice steps along and a tilt
//! turns toward. Navigation itself, pan, slice, zoom, the raw tilts and the oblique plane, is the chart-path object's
//! (`crate::path`): each gesture is a segment of a named family, and its formula lives there.

pub mod directions;

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
    /// The morph's source: neither basis vector has a configuration component (Part 4).
    NoConfiguration,
    /// A named direction is unavailable at this centre ([`directions::Unavailable`]).
    Unavailable(directions::Unavailable),
}

/// One of the plane's two basis vectors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
pub(crate) fn scaled(a: &Latent, k: f64) -> Latent {
    a.map(|x| k * x)
}

/// `a / ‖a‖`: [`NavError::NonFinite`] for a non-finite `a`, [`NavError::ZeroVector`] for `a = 0`.
pub(crate) fn unit(a: &Latent) -> Result<Latent, NavError> {
    unit_of(a, 0.0)
}

/// `a / ‖a‖` for an `a` left by cancelling terms of size `size`: [`NavError::ZeroVector`] where `‖a‖ ≤ √ε·size`, `ε`
/// the f64 machine epsilon, below which the direction would keep fewer than half its digits, rounding rather than
/// geometry (the conditioning guard of a Gram–Schmidt step and of the tilt's sum).
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
pub(crate) fn finite(values: &[f64]) -> Result<(), NavError> {
    match values.iter().all(|x| x.is_finite()) {
        true => Ok(()),
        false => Err(NavError::NonFinite),
    }
}

/// The plane's orthonormal frame `(û₁, û₂)`: Gram–Schmidt of `q₁`, then `q₂`. [`NavError::DegeneratePlane`] where they
/// span no plane: `q₁ = 0`, or `q₂`'s part orthogonal to `q₁` at most `√ε·‖q₂‖` (the `unit_of` tolerance).
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
