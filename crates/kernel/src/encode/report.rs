//! The discard report (inverse-encode contract Parts 1, 2, 3 and 6; dd_encode §2, §4 seam 8): everything encode
//! discards is reported, `lookup_rescaled` and `lookup_clamped`, each raised iff its operation did.
//! There is no `lookup_mirrored`: `C` has no mirror step, so nothing is reflected and the user's `L_z` sign never
//! changes frame.
//!
//! - **`lookup_clamped`** ([`Report::lookup_clamped`]) is raised iff `C`'s step 2 met `ρ = 0` exactly, the coincident
//!   inner pair that cannot be represented ([`Report::coincident`]), or a block inverse clamped one of the eight
//!   controls ([`Report::clamped_controls`], bit `k` for `z[k]` in the latent block layout). It is the encode clamp flag
//!   meant by the "saturation flags" fencing the `α`-poles (dd_decoder §3.2), not the payload's `saturated` bit, and it
//!   fires at either exact `α`-pole, where `s_α` is 0 or 1 and the `α` link's inverse clamps it. Which controls moved
//!   is kept, as Part 6's "show which coordinates moved" asks.
//! - **`lookup_rescaled`** ([`Report::rescaled`]) is raised iff `C`'s step 0 ran, `|I_in − 1| > τ_I` with the bodies
//!   apart, and carries `λ = I_in^{−1/2}` ([`Report::lambda`]); where step 0 was skipped or the bodies
//!   coincide, `λ = 1` and the notice is not raised (`crate::canon`'s note). A canonical input raises none.

/// What encode discarded, with the value `lookup_rescaled` carries.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Report<R> {
    /// `λ = I_in^{−1/2}` where step 0 ran, else 1: `lookup_rescaled`'s value.
    pub lambda: R,
    /// `lookup_rescaled`: `C`'s step 0 ran.
    pub rescaled: bool,
    /// `ρ = 0` exactly at `C`'s step 2: the inner pair coincides and cannot be represented.
    pub coincident: bool,
    /// Bit `k` set iff the inverse of `z[k]`'s link clamped its argument, `z` in the latent block layout
    /// (`z_α, z_β | z_q0 … z_q3 | z_μ1, z_μ2`).
    pub clamped_controls: u32,
}

impl<R> Report<R> {
    /// `lookup_clamped`: the coincident inner pair, or any control clamped.
    #[inline]
    pub fn lookup_clamped(&self) -> bool {
        self.coincident | (self.clamped_controls != 0)
    }

    /// `lookup_rescaled`.
    #[inline]
    pub fn lookup_rescaled(&self) -> bool {
        self.rescaled
    }
}
