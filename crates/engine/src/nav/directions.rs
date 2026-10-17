//! The direction library (chart_decoder_contract Part 4, "Tilt" and "Directions are axis kinds"; REQ-GUI-003,
//! REQ-CHART-009, REQ-CHART-051): the directions a slice steps along and a tilt rotates toward.
//!
//! - **The six orthogonal to the plane** ([`hidden`]): the raw latent directions `ê_k` with the plane projected out,
//!   orthonormalised. Constant vectors, needing no anchor.
//! - **The named compound directions**, each a unit latent vector evaluated at the chart centre `z₀` and recomputed
//!   whenever it changes, as Part 4's "Their latent vectors" defines them (R-72): [`mass_away_from_burrau`],
//!   [`energy_at_fixed_lz`] and the Burrau-to-unconstrained [`morph`]. [`library`] evaluates them all at one plane.
//!
//! None of it reaches the kernel (Part 4, "What the GPU knows"): the directions are read from the shared CPU decode
//! (`kernel::decode`) and the selected links, and a gesture along one is an edit of `z₀` or the basis.

use kernel::decode::latent::Latent as Controls;
use kernel::decode::{self, invariant};
use kernel::generated::constants::ChartConstants;
use kernel::generated::links::{Link, Selection};

use super::{axpy, dot, frame, norm, off_plane, unit, unit_of, BasisVector, NavError};
use crate::contract::sim_config::{Latent, Plane, LATENT_DIM};

/// Why a named direction is unavailable at a centre (Part 4, "Their latent vectors").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unavailable {
    /// The mass direction: the centre's mass controls are equal masses' already.
    EqualMasses,
    /// The energy direction: a momentum link's derivative underflows to 0 at the centre (a saturated control).
    SaturatedMomentum,
    /// The morph's target: its direction lies in the plane.
    InPlane,
    /// The decode at the centre, or the direction, is not finite.
    NonFinite,
}

/// One of the six directions orthogonal to the plane: the raw latent direction `ê_source` with the plane and the
/// directions before it projected out, normalised.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hidden {
    /// The latent index `k` of the `ê_k` it is made from.
    pub source: usize,
    /// The unit direction.
    pub direction: Latent,
}

/// The six directions orthogonal to the plane (Part 4, "Tilt": "the 8D has 6 directions orthogonal to the current
/// plane"), an orthonormal basis of its complement: pivoted Gram–Schmidt of `ê₀ … ê₇` against the plane's frame, each
/// step taking the `ê_k` with the largest remaining component (the lowest `k` on a tie), so each is well conditioned.
/// On an axis-aligned plane `(ê_i, ê_j)` they are the six other `ê_k` exactly, in index order.
pub fn hidden(plane: &Plane) -> Result<[Hidden; 6], NavError> {
    let [u1, u2] = frame(plane)?;
    let mut basis = vec![u1, u2];
    let mut used = [false; LATENT_DIM];
    let mut out = Vec::with_capacity(6);
    for _ in 0..6 {
        let mut best: Option<(usize, Latent, f64)> = None;
        for (k, done) in used.iter().enumerate() {
            if *done {
                continue;
            }
            let mut r: Latent = [0.0; LATENT_DIM];
            r[k] = 1.0;
            for b in &basis {
                r = axpy(&r, -dot(&r, b), b);
            }
            let n = norm(&r);
            if best.as_ref().is_none_or(|(_, _, held)| n > *held) {
                best = Some((k, r, n));
            }
        }
        let (k, r, _) = best.ok_or(NavError::DegeneratePlane)?;
        let direction = unit(&r)?;
        used[k] = true;
        basis.push(direction);
        out.push(Hidden {
            source: k,
            direction,
        });
    }
    out.try_into().map_err(|_| NavError::DegeneratePlane)
}

/// `v / ‖v‖` for a named direction left by cancelling terms of size `size` ([`unit_of`]): `zero` where it has no
/// direction, [`Unavailable::NonFinite`] where `v` is not finite.
fn named_unit(v: &Latent, size: f64, zero: Unavailable) -> Result<Latent, Unavailable> {
    unit_of(v, size).map_err(|e| match e {
        NavError::ZeroVector => zero,
        _ => Unavailable::NonFinite,
    })
}

/// The mass perturbation away from Burrau at the centre `z0`, under the link selection `S` (Part 4, "Their latent
/// vectors"): in the mass block, `d_m ∝ z_eq − z₀[6:8]`, `z_eq` the mass link's inverse at equal masses, the straight
/// latent line from the centre's mass controls (the Burrau logits at a Burrau centre) to equal masses. Unavailable
/// where the centre's mass controls are `z_eq`.
pub fn mass_away_from_burrau<S: Selection>(z0: &Latent) -> Result<Latent, Unavailable> {
    let third = 1.0 / 3.0;
    let z_eq = <S::MassLink as Link<2, 3>>::inverse([third; 3]);
    let mut d = [0.0; LATENT_DIM];
    d[6] = z_eq[0] - z0[6];
    d[7] = z_eq[1] - z0[7];
    named_unit(&d, 0.0, Unavailable::EqualMasses)
}

/// The energy increase at fixed `L_z` at the centre `z0`, under the link selection `S` (Part 4, "Their latent
/// vectors"): in the momentum block, the velocity with its rigid rotation projected out, `δv = v − (L_z/I)·J r`
/// (chart_reference §2.2's projection), or §2.2's seed direction `w` where `‖δv‖²_m ≤ ε_w·‖v‖²_m`; as Jacobi momenta,
/// pulled back through each momentum link's derivative `exp(log_det)`, normalised. Unavailable where a derivative
/// underflows to 0.
pub fn energy_at_fixed_lz<S: Selection>(z0: &Latent) -> Result<Latent, Unavailable> {
    let decoded = decode::decode::<S, f64>(&Controls::new(*z0));
    let (masses, config) = (&decoded.masses, &decoded.config);
    let (m, r, p) = (masses.m, config.r, decoded.momentum.p);
    let v: [[f64; 2]; 3] = std::array::from_fn(|i| [p[i][0] / m[i], p[i][1] / m[i]]);
    let off_rotation = invariant::project(m, &r, &v);
    let rigid = invariant::mass_norm2(m, &off_rotation)
        <= <f64 as ChartConstants>::EPS_W * invariant::mass_norm2(m, &v);
    let dv = match rigid {
        true => invariant::construct(masses, config, z0[0], 0.0, 0.0).w,
        false => off_rotation,
    };
    let dp: [[f64; 2]; 3] = std::array::from_fn(|i| [m[i] * dv[i][0], m[i] * dv[i][1]]);
    let jacobi = |c: usize| (m[0] * dp[1][c] - m[1] * dp[0][c]) / masses.m01;
    let dq = [jacobi(0), jacobi(1), dp[2][0], dp[2][1]];
    let slopes = [
        <S::Q0Link as Link<1, 1>>::log_det([z0[2]]).exp(),
        <S::Q1Link as Link<1, 1>>::log_det([z0[3]]).exp(),
        <S::Q2Link as Link<1, 1>>::log_det([z0[4]]).exp(),
        <S::Q3Link as Link<1, 1>>::log_det([z0[5]]).exp(),
    ];
    if slopes.contains(&0.0) {
        return Err(Unavailable::SaturatedMomentum);
    }
    let mut d = [0.0; LATENT_DIM];
    for ((out, q), slope) in d[2..6].iter_mut().zip(dq).zip(slopes) {
        *out = q / slope;
    }
    named_unit(&d, 0.0, Unavailable::NonFinite)
}

/// What the morph rotates its source into (Part 4: "into a mass or momentum direction").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MorphInto {
    /// The mass perturbation away from Burrau.
    Mass,
    /// The energy increase at fixed `L_z`.
    Momentum,
}

/// The Burrau-to-unconstrained morph: the tilt of `source` toward `target`, which lies off the plane.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Morph {
    /// The basis vector tilted: the plane's configuration direction.
    pub source: BasisVector,
    /// The unit direction rotated into, orthogonal to the plane.
    pub target: Latent,
}

/// The share of `q`'s squared length in the configuration block, `‖q[0:2]‖²/‖q‖²`, 0 for `q = 0`.
fn configuration_share(q: &Latent) -> f64 {
    let whole = dot(q, q);
    match whole > 0.0 {
        true => (q[0] * q[0] + q[1] * q[1]) / whole,
        false => 0.0,
    }
}

/// The morph's source (Part 4, "Their latent vectors"): the basis vector with the larger share of its length in the
/// configuration block, `q₁` on a tie; [`NavError::NoConfiguration`] where neither has a configuration component.
pub fn morph_source(plane: &Plane) -> Result<BasisVector, NavError> {
    let (s1, s2) = (
        configuration_share(&plane.q1),
        configuration_share(&plane.q2),
    );
    if s1 == 0.0 && s2 == 0.0 {
        return Err(NavError::NoConfiguration);
    }
    Ok(match s2 > s1 {
        true => BasisVector::Q2,
        false => BasisVector::Q1,
    })
}

/// The Burrau-to-unconstrained morph at `plane`'s centre, under the link selection `S` (Part 4, "Their latent
/// vectors"; chart_reference §4.6): its source [`morph_source`], its target the named direction `into` names with its
/// component in the plane projected out, normalised, so a tilt along it leaves the plane and keeps `z(½,½) = z₀`.
pub fn morph<S: Selection>(plane: &Plane, into: MorphInto) -> Result<Morph, NavError> {
    let source = morph_source(plane)?;
    let direction = match into {
        MorphInto::Mass => mass_away_from_burrau::<S>(&plane.z0),
        MorphInto::Momentum => energy_at_fixed_lz::<S>(&plane.z0),
    }
    .map_err(NavError::Unavailable)?;
    let off = off_plane(plane, &direction)?;
    let target = named_unit(&off, 1.0, Unavailable::InPlane).map_err(NavError::Unavailable)?;
    Ok(Morph { source, target })
}

/// The library at one plane, each direction evaluated at its centre: what the tilt selector offers (Part 4, "Tilt").
#[derive(Clone, Debug, PartialEq)]
pub struct Library {
    /// The six directions orthogonal to the plane.
    pub hidden: Result<[Hidden; 6], NavError>,
    /// The mass perturbation away from Burrau.
    pub mass: Result<Latent, Unavailable>,
    /// The energy increase at fixed `L_z`.
    pub energy: Result<Latent, Unavailable>,
    /// The Burrau-to-unconstrained morph into mass.
    pub morph_mass: Result<Morph, NavError>,
    /// The Burrau-to-unconstrained morph into momentum.
    pub morph_momentum: Result<Morph, NavError>,
}

/// The library at `plane`'s centre under the link selection `S`; call it again when the centre moves, as the named
/// directions are recomputed with it.
pub fn library<S: Selection>(plane: &Plane) -> Library {
    Library {
        hidden: hidden(plane),
        mass: mass_away_from_burrau::<S>(&plane.z0),
        energy: energy_at_fixed_lz::<S>(&plane.z0),
        morph_mass: morph::<S>(plane, MorphInto::Mass),
        morph_momentum: morph::<S>(plane, MorphInto::Momentum),
    }
}
