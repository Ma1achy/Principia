//! The body plane (chart_reference §5.1): today's slice kept as a chart. One body's position
//! sweeps a square window, the other two stay at their held positions, every body at rest, the masses held.
//!
//! **`Φ`.** `Φ(u, v)` puts body `body` at `p(u, v) = ((c_x − h) + (2h)·u, (c_y − h) + (2h)·v)`, `(c_x, c_y)` the
//! window's centre and `h` its half-width, in that operation order: the order the slice has always used (the
//! re-registration gate's `ics`), so the chart reproduces today's pixels bit for bit. The map is affine in `(u, v)` on
//! the whole plane, so past `[0,1]²` it keeps going: both axes' extension type is affine on both sides
//! ([`BodyPlane::extension`]), and every `(u, v)` with a finite position is a system (a binary collision, where the
//! moved body lands on a held one, is a system with `V₀ = −∞`, never a forbidden pixel).
//!
//! **Into the shared decoder.** The system `(m, r(u, v), p = 0)` is put on the canonical section by `C`
//! ([`canon::canonicalise`]: CoM at the origin, `I = 1`, `ρ` on `+x`, no mirror), and its Jacobi vectors read off there go
//! through the rest-start blocks the Euclid plane uses ([`RestBlocks`]): the masses through the shared entry for a chart
//! that sets them, the configuration through the shared canonical-frame entry for an explicit `(ρ, λ)`, every momentum
//! 0. So the decoded state is the window's system in the canonical gauge: its shape and its dimensionless energy
//! `E·√I` are the window's, its length and time scale are the gauge's, not the window's units. `Φ` reproduces the
//! initial conditions of the reference's slice, not a field at a common physical time (each pixel's unit of time is
//! its own `√(I³)` once the gauge is applied).
//!
//! **The reference** (chart_reference §5.1). The Python reference (`docs/reference/prin-rs/reference/tb.py`'s
//! `burrau_grid`, masses `(3, 4, 5)` at `(1, 3)`, `(−2, −1)`, `(1, −1)`) places the body at `numpy.linspace`'s nodes,
//! `i·((b − a)/(n − 1)) + a` with the last node `b` exactly: a different operation order from `Φ`'s at `u = i/(n − 1)`,
//! so the two agree to a calibrated tolerance, not bitwise (`validation::bodyplane`, `cargo xtask golden bodyplane`).

use spirv_std::num_traits::Float;

use super::burrau::RestBlocks;
use super::energy::{ExtensionType, Sides};
use super::{Constraint, ValidationResult};
use crate::burrau::System;
use crate::canon;
use crate::decode::{self, Decoded, Ic, Latent};
use crate::generated::links::LinkReal;
use crate::payload::PayloadReal;

/// The chart's name, in every dump header.
pub const NAME: &str = "BodyPlane";

/// The body plane: body `body`'s position over the window centred on `centre`, half-width `half`; the other bodies at
/// `r`, the masses `m` (`Σmᵢ = 1`), all at rest.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyPlane<P> {
    /// The body that moves, `0`, `1` or `2`.
    pub body: u32,
    /// The window's centre `(c_x, c_y)`.
    pub centre: [P; 2],
    /// The window's half-width `h`.
    pub half: P,
    /// The masses, `Σmᵢ = 1`.
    pub m: [P; 3],
    /// The held positions; `r[body]` is replaced by `Φ`'s.
    pub r: [[P; 2]; 3],
}

impl<P: Copy> BodyPlane<P> {
    /// The moved body's position at `(u, v)`: `((c_x − h) + (2h)·u, (c_y − h) + (2h)·v)`, in that order (the module's
    /// note).
    #[inline]
    pub fn position<F: Float + From<P>>(&self, u: F, v: F) -> [F; 2] {
        let wide = <F as From<P>>::from;
        let h = wide(self.half);
        let two_h = h + h;
        [
            (wide(self.centre[0]) - h) + two_h * u,
            (wide(self.centre[1]) - h) + two_h * v,
        ]
    }

    /// The system at `(u, v)`: the masses and the positions, the moved body at [`BodyPlane::position`].
    #[inline]
    pub fn system<F: Float + From<P>>(&self, u: F, v: F) -> ([F; 3], [[F; 2]; 3]) {
        self.system_at(self.position(u, v))
    }

    /// The system with the moved body at `at`: the masses and the positions, the held bodies at `r`. Placement only, so
    /// a position the reference formed itself gives the reference's own state bit for bit.
    #[inline]
    pub fn system_at<F: Float + From<P>>(&self, at: [F; 2]) -> ([F; 3], [[F; 2]; 3]) {
        let wide = <F as From<P>>::from;
        let pick = |k: u32, held: [P; 2]| {
            if self.body == k {
                at
            } else {
                [wide(held[0]), wide(held[1])]
            }
        };
        (
            [wide(self.m[0]), wide(self.m[1]), wide(self.m[2])],
            [pick(0, self.r[0]), pick(1, self.r[1]), pick(2, self.r[2])],
        )
    }

    /// Both axes' extension type: affine on both sides, the formula continuing (the module's note).
    #[inline]
    pub fn extension(&self) -> [Sides; 2] {
        [Sides::both(ExtensionType::Affine); 2]
    }

    /// `D ∘ Φ` of the system `(m, r)` at rest (the module's note): `C`, then the rest-start blocks on its Jacobi vectors.
    #[inline]
    pub fn decode_system<R>(m: [R; 3], r: [[R; 2]; 3]) -> Decoded<R>
    where
        R: LinkReal + PayloadReal,
    {
        let zero = R::zero();
        let c = canon::canonicalise(&Ic {
            m,
            r,
            p: [[zero; 2]; 3],
        })
        .ic;
        let [r0, r1, r2] = c.r;
        let m01 = m[0] + m[1];
        let rho = [r1[0] - r0[0], r1[1] - r0[1]];
        let r01 = [
            (m[0] * r0[0] + m[1] * r1[0]) / m01,
            (m[0] * r0[1] + m[1] * r1[1]) / m01,
        ];
        let lambda = [r2[0] - r01[0], r2[1] - r01[1]];
        let mut blocks = RestBlocks::new(System {
            m,
            r: c.r,
            rho,
            lambda,
        });
        decode::decode_with(&mut blocks, &Latent::new([zero; 8]))
    }
}

/// `Φ` only, outside the [`Chart`](super::Chart) trait: the plan is unclear here; chose to keep the body plane out of
/// the trait because every `Chart` impl must be in [`super::REGISTRY`], and the registry carries tilt, lock, save, render
/// and descriptor support the cross-check's anchor does not need. Registering it is a follow-up; until then these are
/// the trait's three entries, under the same names, and the gallery sweep runs them.
impl<P: Copy> BodyPlane<P>
where
    f64: From<P>,
{
    /// The chart's name, in every dump header.
    pub fn name(&self) -> &str {
        NAME
    }

    /// `D ∘ Φ` at `(u, v)`: `C` and the rest-start blocks on the system there; no link is read.
    #[inline]
    pub fn decode<R>(&self, u: R, v: R) -> Decoded<R>
    where
        R: LinkReal + PayloadReal + From<P>,
    {
        let (m, r) = self.system(u, v);
        Self::decode_system(m, r)
    }

    /// Any finite position is a system, so layer 2 has no constraint; layer 3 on the f64 decode: a non-finite position
    /// (a window so far out that `Φ` overflows) is rejected, naming the CoM, the first quantity it spoils.
    pub fn validate(&self, u: f64, v: f64) -> ValidationResult {
        let d = self.decode::<f64>(u, v);
        let finite = |x: &[[f64; 2]; 3]| x.iter().flatten().all(|c| c.is_finite());
        if !(u.is_finite() && v.is_finite() && finite(&d.ic.r)) {
            ValidationResult::Reject {
                failed: Constraint::ComAtOrigin,
            }
        } else if !finite(&d.ic.p) {
            ValidationResult::Reject {
                failed: Constraint::MomentumZero,
            }
        } else {
            ValidationResult::Valid
        }
    }
}
