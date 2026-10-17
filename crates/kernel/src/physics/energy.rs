//! The monitored invariants, integrator_contract Part 1 ("COM projection and invariant monitoring"), with `G = 1`:
//!
//! ```text
//! E   = Σᵢ ‖pᵢ‖²/2mᵢ − Σ_{i<j} mᵢmⱼ/‖rᵢ − rⱼ‖
//! L_z = Σᵢ (xᵢ p_{y,i} − yᵢ p_{x,i})
//! ```
//!
//! The potential term reads the force evaluation's [`PairGeom`], so it forms no distance of its own (REQ-PERF-001).
//! Each sum is written out over its three terms, so no fold is seeded.

use super::PairGeom;
use crate::Real;

/// The kinetic energy `Σᵢ ‖pᵢ‖²/2mᵢ` of momenta `p` and masses `m`; `2mᵢ` is formed as `mᵢ + mᵢ`, which is exact.
#[inline]
pub fn kinetic<R: Real>(m: &[R; 3], p: &[[R; 2]; 3]) -> R {
    (p[0][0] * p[0][0] + p[0][1] * p[0][1]) / (m[0] + m[0])
        + (p[1][0] * p[1][0] + p[1][1] * p[1][1]) / (m[1] + m[1])
        + (p[2][0] * p[2][0] + p[2][1] * p[2][1]) / (m[2] + m[2])
}

/// The potential energy `−Σ_{i<j} mᵢmⱼ/‖rᵢ − rⱼ‖` of masses `m`, from the pairwise geometry `g`.
#[inline]
pub fn potential<R: Real>(m: &[R; 3], g: &PairGeom<R>) -> R {
    let norm = g.norm();
    -(m[0] * m[1] / norm[0] + m[1] * m[2] / norm[1] + m[2] * m[0] / norm[2])
}

/// The energy `E`, [`kinetic`] plus [`potential`].
#[inline]
pub fn energy<R: Real>(m: &[R; 3], p: &[[R; 2]; 3], g: &PairGeom<R>) -> R {
    kinetic(m, p) + potential(m, g)
}

/// The angular momentum `L_z = Σᵢ (xᵢ p_{y,i} − yᵢ p_{x,i})` of positions `r` and momenta `p`.
#[inline]
pub fn angular_momentum<R: Real>(r: &[[R; 2]; 3], p: &[[R; 2]; 3]) -> R {
    (r[0][0] * p[0][1] - r[0][1] * p[0][0])
        + (r[1][0] * p[1][1] - r[1][1] * p[1][0])
        + (r[2][0] * p[2][1] - r[2][1] * p[2][0])
}
