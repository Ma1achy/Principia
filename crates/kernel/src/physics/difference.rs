//! The force difference of the FTLE shadow's joint step (R-974, RQ-793 (a); dd_integrator §3.8; integrator_contract
//! Part 2): the change in the three bodies' accelerations when their positions `r` move by a displacement `δr`,
//! `a(r + δr) − a(r)`, formed from the pair separations, never from an absolute `r + δr`.
//!
//! Pair `k` (R-22: `(i, j) = (1, 2), (2, 0), (0, 1)`) has the separation `dₖ = rⱼ − rᵢ` of the sample's
//! [`PairGeom`] and the separation's displacement `eₖ = δrⱼ − δrᵢ`, and contributes the change of its term
//! `u(d) = d/‖d‖³` ([`super::force`]),
//!
//! ```text
//! Δuₖ = u(dₖ + eₖ) − u(dₖ)
//! ```
//!
//! which [`super::force::combine`] turns into the accelerations' change, the force being linear in the terms.
//! [`ForceDifference`] has two forms, R-974's sub-choice, which a measurement chooses (TASK-M3-72's criterion):
//!
//! - **(a1) [`Plain`]**: `d + e` formed at the working precision, its `u` formed as the force forms it, less `u(d)`.
//!   Where `‖e‖ ≪ ‖d‖`, `d + e` is rounded to `ε·‖d‖`, so the difference keeps about `ε·‖d‖/‖e‖` relative.
//!   Where `‖e‖ ≲ ε·‖d‖/2`, below the rounding step of the separation, `d + e` rounds to `d` and the difference is 0:
//!   a shadow that small on every pair drifts force-free, and its finite `λ_T` measures no exponent (dd_integrator
//!   §3.8, (a1)'s precision floor).
//! - **(a2) [`CancellationFree`]** (the Encke problem): with `n = ‖d‖`, `n' = ‖d + e‖`,
//!
//!   ```text
//!   q   = e·(2d + e)                    = n'² − n², formed from e, never as a difference of the two
//!   n'² = n² + q ;   n' = √n'² ;   n'³ = n'²·n'
//!   Δu  = e/n'³ + d·c ,   c = 1/n'³ − 1/n³ = −q·(n'² + n'·n + n²) / ((n' + n)·n³·n'³)
//!   ```
//!
//!   `c` is the difference of the two inverse cubes written as a product, `n'³ − n³ = (n' − n)(n'² + n'n + n²)` and
//!   `n' − n = q/(n' + n)`, so no two nearly equal numbers are subtracted, and every factor keeps its relative
//!   precision: the difference keeps about `ε` relative whatever `‖e‖/‖d‖`. Its range in `‖d‖` is narrower than the
//!   force's: the denominator `(n' + n)·n³·n'³` is seven powers of distance, so at f32 it leaves the normal range
//!   below `‖d‖ ≈ 4e-6` and rounds to zero below about `3e-7`, where (a2) reads infinite or NaN; (a1) reaches as far
//!   as the force itself. (a2) is the measured alternative, not the chosen form (TASK-M3-72's result: `Displacement`
//!   names (a1)); only that measurement and the tests form it.
//!
//! No branch, no loop, no runtime index (integrator_contract Part 2c): each pair is written out.

use super::force::combine;
use super::PairGeom;
use crate::Real;

/// How the joint step forms the force difference (R-974's (a1) or (a2)).
pub trait ForceDifference {
    /// The change of pair `k`'s term, `u(d + e) − u(d)`, for its separation `d` (with `‖d‖²`, `‖d‖` and `‖d‖³` as the
    /// sample's [`PairGeom`] formed them) and the separation's displacement `e`.
    fn term<R: Real>(d: [R; 2], d2: R, norm: R, cube: R, e: [R; 2]) -> [R; 2];

    /// `a(r + δr) − a(r)` for bodies of masses `m` whose positions `r` have the pairwise geometry `pair`, displaced by
    /// `dr`: each pair's [`ForceDifference::term`], combined as the force combines its terms.
    #[inline]
    fn accel<R: Real>(m: &[R; 3], pair: &PairGeom<R>, dr: &[[R; 2]; 3]) -> [[R; 2]; 3] {
        let (sep, d2, norm, cube) = (pair.sep(), pair.d2(), pair.norm(), pair.cube());
        let e = [
            [dr[2][0] - dr[1][0], dr[2][1] - dr[1][1]],
            [dr[0][0] - dr[2][0], dr[0][1] - dr[2][1]],
            [dr[1][0] - dr[0][0], dr[1][1] - dr[0][1]],
        ];
        let du = [
            Self::term(sep[0], d2[0], norm[0], cube[0], e[0]),
            Self::term(sep[1], d2[1], norm[1], cube[1], e[1]),
            Self::term(sep[2], d2[2], norm[2], cube[2], e[2]),
        ];
        combine(m, &du)
    }
}

/// (a1): the difference formed plainly at the working precision (the module's note).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Plain;

impl ForceDifference for Plain {
    #[inline]
    fn term<R: Real>(d: [R; 2], _d2: R, _norm: R, cube: R, e: [R; 2]) -> [R; 2] {
        let s = [d[0] + e[0], d[1] + e[1]];
        // `‖d + e‖²`, `‖d + e‖` and `‖d + e‖³` as `PairGeom::new` forms them for a separation.
        let s2 = s[1].fma(s[1], s[0] * s[0]);
        let s3 = s2 * s2.square_root();
        [s[0] / s3 - d[0] / cube, s[1] / s3 - d[1] / cube]
    }
}

/// (a2): the difference formed cancellation-free (the module's note).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CancellationFree;

impl ForceDifference for CancellationFree {
    #[inline]
    fn term<R: Real>(d: [R; 2], d2: R, norm: R, cube: R, e: [R; 2]) -> [R; 2] {
        // `q = e·(2d + e)`.
        let w = [(d[0] + d[0]) + e[0], (d[1] + d[1]) + e[1]];
        let q = e[1].fma(w[1], e[0] * w[0]);
        let s2 = d2 + q;
        let s = s2.square_root();
        let s3 = s2 * s;
        let c = -(q * ((s2 + s * norm) + d2)) / (((s + norm) * cube) * s3);
        [e[0] / s3 + d[0] * c, e[1] / s3 + d[1] * c]
    }
}
