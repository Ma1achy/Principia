//! The three-body force, dd_integrator §3.1, with `G = 1` (REQ-INT-036):
//!
//! ```text
//! aᵢ = Σ_{j≠i} mⱼ (rⱼ − rᵢ) / ‖rⱼ − rᵢ‖³        (3 pairs; ‖·‖³ shared with r_min)
//! ```
//!
//! read from a [`PairGeom`]. Pair `k = (i, j)` contributes `uₖ = dₖ/‖dₖ‖³` once: `+mⱼuₖ` to `aᵢ` and `−mᵢuₖ` to `aⱼ`,
//! since `rᵢ − rⱼ = −dₖ`.

use super::PairGeom;
use crate::Real;

/// The accelerations `aᵢ` of the three bodies of masses `m`, from their pairwise geometry `g`.
#[inline]
pub fn force<R: Real>(m: &[R; 3], g: &PairGeom<R>) -> [[R; 2]; 3] {
    let (sep, cube) = (g.sep(), g.cube());
    let u = [
        [sep[0][0] / cube[0], sep[0][1] / cube[0]],
        [sep[1][0] / cube[1], sep[1][1] / cube[1]],
        [sep[2][0] / cube[2], sep[2][1] / cube[2]],
    ];
    [
        [
            m[1] * u[0][0] - m[2] * u[2][0],
            m[1] * u[0][1] - m[2] * u[2][1],
        ],
        [
            m[2] * u[1][0] - m[0] * u[0][0],
            m[2] * u[1][1] - m[0] * u[0][1],
        ],
        [
            m[0] * u[2][0] - m[1] * u[1][0],
            m[0] * u[2][1] - m[1] * u[1][1],
        ],
    ]
}
