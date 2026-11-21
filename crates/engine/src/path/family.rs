//! Each named family's formula (`docs/design/principia_chart_path_sketch.md` § 1): the very function today's operation
//! ran, moved here from `nav` and `lock` unchanged, so a family's frame is the direct operation's bits. Nothing outside
//! [`super`] edits a view through these: a view edit is a [`super::Segment`], built by its constructor.
//!
//! | Family | Formula | Edit |
//! |---|---|---|
//! | `Pan` | [`pan`] | `z₀ += 2ds·q₁ + 2dt·q₂` (chart_decoder_contract Part 4, "Pan") |
//! | `Zoom` | [`zoom`] | `q₁, q₂ ×= 2^−octaves`, log-stepped (Part 4, "Zoom"; R-83) |
//! | `Slice` | [`slice`] | `z₀ += t·off_plane(d)` (Part 4, "Slice") |
//! | `Control` | [`control`] | `z₀[k] ← value`, no projection (Part 4, "Sliders are re-based") |
//! | `RawTilt` | [`Constructor::tilt`], [`Constructor::tilt_pair`] | `q′ = ‖q‖·normalise(cos τ·q̂ + sin τ·d̂)` from the committed plane (Part 4, "Tilt"; chart_reference §1.1) |
//!
//! **The scale lives in `q`** (R-83): a zoomed basis vector is `k·q̂`, and a tilt keeps each basis vector's length, so a
//! tilt neither resets the zoom nor gives the two axes different scales. **Raw tilts replace rather than compose**
//! (chart_reference §1.1): the [`Constructor`] holds the committed plane each raw tilt starts from.

use crate::contract::set_field::{Edit, SetField, SimField};
use crate::contract::sim_config::{Latent, Plane, SimConfig};
use crate::lock::LockError;
use crate::nav::{
    axpy, finite, frame, norm, off_plane, scaled, unit, unit_of, BasisVector, NavError,
};

/// A `SimConfig` edit, entering the undo history (R-69).
pub(crate) fn edit(field: SimField) -> SetField {
    SetField {
        edit: Edit::Sim(field),
        no_history: false,
    }
}

/// Pan by `(ds, dt)` of the view's unsigned `[0,1]²` addresses: `Δz₀ = 2ds·q₁ + 2dt·q₂ ∈ span(q₁, q₂)`, so the pixel at
/// `(½ + ds, ½ + dt)` becomes the centre and the plane is the same.
pub(crate) fn pan(plane: &Plane, ds: f64, dt: f64) -> Result<SetField, NavError> {
    finite(&[ds, dt])?;
    let z0 = axpy(&axpy(&plane.z0, 2.0 * ds, &plane.q1), 2.0 * dt, &plane.q2);
    Ok(edit(SimField::Z0(z0)))
}

/// Slice by `t` along the hidden direction `d`: `z₀ ← z₀ + t·d⊥`, `d⊥` the part of `d` orthogonal to the plane, so
/// `Δz₀ ⊥ span(q₁, q₂)` and the plane translates parallel to itself.
pub(crate) fn slice(plane: &Plane, d: &Latent, t: f64) -> Result<SetField, NavError> {
    finite(&[t])?;
    finite(d)?;
    let step = off_plane(plane, d)?;
    Ok(edit(SimField::Z0(axpy(&plane.z0, t, &step))))
}

/// Zoom by `octaves` about the centre (R-83): `q₁` and `q₂` scaled by the one common factor `2^−octaves`, log-stepped,
/// `z₀` kept; a positive `octaves` narrows the window.
pub(crate) fn zoom(plane: &Plane, octaves: f64) -> Result<SetField, NavError> {
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

/// A slider's move of the control `k` to `value`: the `SetField` on `z₀` with that control set, the others kept. While
/// locked it is an excursion along `ê_k` from the anchor, which it keeps.
pub(crate) fn control(sim: &SimConfig, k: usize, value: f64) -> Result<SetField, LockError> {
    if !value.is_finite() {
        return Err(LockError::NonFinite);
    }
    let mut z0 = sim.plane.z0;
    *z0.get_mut(k).ok_or(LockError::NoSuchControl(k))? = value;
    Ok(edit(SimField::Z0(z0)))
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

/// The tilt formula (Part 4, "Directions are axis kinds"): `length·normalise(cos τ·γ̂′ + sin τ·d̂)`, `γ′ = tangent` and
/// `d = target`, each taken as its direction. `τ = 0` is the tangent's direction, `τ = ±90°` is `±d̂`; the sign matters.
/// A vector, not a view edit: the raw tilt and a curve axis's tangent tilt both read it.
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

/// A raw basis vector's tilt (Part 4, "Tilt"): `q′(τ) = ‖q‖·normalise(cos τ·q̂ + sin τ·d̂)`, the length kept.
pub fn tilt_vector(q: &Latent, tau_deg: f64, target: &Latent) -> Result<Latent, NavError> {
    tilt_tangent(q, norm(q), tau_deg, target)
}

/// The chart constructor (chart_reference §1.1): the committed plane every raw tilt starts from, so raw tilts replace
/// rather than compose; [`commit`](Self::commit) makes a plane the new start. Its tilts are segments
/// ([`super::Segment::raw_tilt`], [`super::Segment::raw_tilt_pair`]).
#[derive(Clone, Debug, PartialEq)]
pub struct Constructor {
    committed: Plane,
}

impl Constructor {
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
    /// has in `current`. [`NavError::DegeneratePlane`] where the tilted pair spans no plane.
    pub(crate) fn tilt(
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

    /// The raw tilt of both basis vectors at once, as one basis edit: `q₁` the committed `q₁` turned by `τ₁ = q1.0`
    /// toward `q1.1`, `q₂` the committed `q₂` turned by `τ₂ = q2.0` toward `q2.1`, each at the length it has in
    /// `current`. [`NavError::DegeneratePlane`] where the tilted pair spans no plane.
    pub(crate) fn tilt_pair(
        &self,
        current: &Plane,
        q1: (f64, &Latent),
        q2: (f64, &Latent),
    ) -> Result<SetField, NavError> {
        let base = &self.committed;
        let q1 = tilt_tangent(&base.q1, norm(&current.q1), q1.0, q1.1)?;
        let q2 = tilt_tangent(&base.q2, norm(&current.q2), q2.0, q2.1)?;
        frame(&Plane {
            z0: current.z0,
            q1,
            q2,
        })?;
        Ok(edit(SimField::Basis { q1, q2 }))
    }
}

/// An oblique slice's plane through `z0` (chart_reference §1.1, "Arbitrary / oblique"): `q₁, q₂` the orthonormal pair
/// Gram–Schmidt makes of `a` then `b`. A constructor of a view's window, not an edit: as an edit of an existing view it
/// is a cut ([`super::Segment::cut`]).
pub fn oblique(z0: Latent, a: &Latent, b: &Latent) -> Result<Plane, NavError> {
    finite(&z0)?;
    let [q1, q2] = frame(&Plane { z0, q1: *a, q2: *b })?;
    Ok(Plane { z0, q1, q2 })
}
