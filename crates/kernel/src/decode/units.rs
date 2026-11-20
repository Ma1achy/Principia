//! The dimensionless system `G = M = I = 1` (canonical_spec §3; dd_decoder §2; REQ-INT-002): the decoder emits
//! `(m, r, p)` with total mass `M = Σmᵢ = 1` (the mass block's simplex) and moment of inertia `I = Σmᵢ‖rᵢ‖² = 1`
//! (the configuration block's scale gauge, `R̃ = 1`), and the physics reads Newton's constant as `G = 1`. Scale is
//! gauged, so a single global horizon `T` is legitimate.
//!
//! `G` is a unit convention, not a chart constant: no chart coordinate decodes through it, so it is not in the link
//! registry's hashed constants (R-340, R-344). It lives here, with the decoder's other unit, as a function of the
//! float type, since the kernel declares no numeric `const` outside the constants register (`cargo xtask lint
//! constants`).

use spirv_std::num_traits::Float;

/// Newton's constant in the dimensionless system: `G = 1`.
#[inline]
pub fn g<R: Float>() -> R {
    R::one()
}

/// The total mass in the dimensionless system: `M = 1`.
#[inline]
pub fn total_mass<R: Float>() -> R {
    R::one()
}

/// The moment of inertia about the CoM in the dimensionless system: `I = 1`.
#[inline]
pub fn inertia<R: Float>() -> R {
    R::one()
}
