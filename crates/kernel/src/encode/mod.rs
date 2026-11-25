//! Encode `E`, physical state → `z` (inverse-encode contract Parts 1, 3 and 6; dd_encode §1–§3): the quotient map onto
//! the decode's canonical section, in the same shared source as the decode, inverted (dd_decoder §1):
//!
//! ```text
//! E  =  (block inverses) ∘ C
//! ```
//!
//! [`encode`] runs `C` (`crate::canon`: 1a CoM → 0 scale → 1b boost → 2 rotate; no mirror step), then the block
//! inverses ([`blocks`]: config, momentum and mass, each through the selection's registry link inverse), and reports
//! what it discarded ([`report`]).
//! [`by_kind`] then encodes into the active chart, axis by axis by kind, and lands in latent `z` where the chart does
//! not represent the state (Parts 5 and 6, step 4); [`curve`] is kind 4's, the Burrau family's closed form, its
//! off-curve projection and the fold.
//! [`ingest`] is the ground-truth import's (Part 7): `z`, the physical residual `‖D(z) − C(x)‖_phys` and `κ(x)`.
//!
//! **T1, constant on the continuous gauge orbits and equivariant under the mirror** (Part 1; dd_encode §3.3):
//! translation and boost are removed by `C`'s 1a and 1b, rotation by step 2, scale by step 0. The mirror `Π` is no gauge
//! direction: `E(Π·x)` is the mirrored `z`, `β → 2π − β` (reduced to `[0, 2π)`, so `β = 0` stays 0; on the control, `z_β → π − z_β`), the momentum
//! controls `z_q1` and `z_q3` (the `y` components of `p_ρ` and `p_λ`) negated by the odd momentum links, `L_z`
//! negated.
//!
//! **One source**: the block inverses are the registry's (`crate::generated::links`), the Jacobi
//! map is the decoder's run backwards, and `C` is the decode's. The fragment WGSL encode is never written by hand: it is
//! generated from this source by the rust-gpu → SPIR-V → WGSL translation, as `crate::stage::encode`'s compute entry
//! point already is.
//!
//! **At the exact `α`-poles** (dd_decoder §3.2): at `‖λ̃‖ = 0`, `α = 0` and `β`'s select gives 0; at
//! `‖ρ̃‖ = 0`, `α = π/2` and `C`'s `φ` is 0 by its own select. Each gives a deterministic `z`, no NaN, and raises
//! `lookup_clamped`: `s_α` is 0 or 1, and the `α` link's inverse clamps it; at `ρ = 0` `C` raises it too.

pub mod blocks;
pub mod by_kind;
pub mod curve;
pub mod held;
// The host's ingestion (`κ` via `+∞` and a swept Jacobi loop): not taken by the SPIR-V build.
#[cfg(not(target_arch = "spirv"))]
pub mod ingest;
pub mod report;

pub use blocks::{Inverted, Jacobi, SimplexClamp};
pub use report::Report;

use crate::canon::{canonicalise, Canonical};
use crate::decode::{Ic, Latent};
use crate::generated::links::{LinkParams, LinkReal, Selection};
use crate::real::IeeeBits;

/// An encoded state: `z`, `C`'s canonical state, the block inverses' intermediates and the discard report.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Encoded<R> {
    /// The eight controls, in the latent block layout.
    pub z: Latent<R>,
    /// `C`'s output: the canonical `(m, r, p)`, `I_in`, `λ`, `φ`, `λ̃_y`, its notices and the decided signs of `λ̃`.
    pub canonical: Canonical<R>,
    /// The block inverses' output: `z`, `α`, `β`, the Jacobi coordinates and the clamp bits.
    pub blocks: Inverted<R>,
    /// What encode discarded.
    pub report: Report<R>,
}

/// `E(ic) = (block inverses) ∘ C (ic)` under the selection `S`. `ic.m` is positive with `Σmᵢ = 1`, as `C` requires; the
/// one exception is [`ingest::weighted_encode_jacobian`]'s mass columns, which differentiate these formulas off the
/// simplex, the total read as given.
#[inline]
pub fn encode<S, R>(ic: &Ic<R>) -> Encoded<R>
where
    S: Selection,
    S::MassLink: SimplexClamp,
    R: LinkReal + IeeeBits,
{
    encode_at::<S, R>(ic, &LinkParams::DEFAULT)
}

/// [`encode`] at the tunable parameters `p`: the block inverses and their clamp tests, encode's layer 1, at the active
/// `μ_max` and `q_max`.
#[inline]
pub fn encode_at<S, R>(ic: &Ic<R>, p: &LinkParams<R>) -> Encoded<R>
where
    S: Selection,
    S::MassLink: SimplexClamp,
    R: LinkReal + IeeeBits,
{
    let canonical = canonicalise(ic);
    // β's case is the one `C` decided on `ic`, not one read off the rotated `canonical.ic` (`blocks`' note).
    let blocks = blocks::invert_decided_at::<S, R>(&canonical.ic, canonical.signs, p);
    Encoded {
        z: Latent::new(blocks.z),
        canonical,
        blocks,
        report: Report {
            lambda: canonical.lambda,
            rescaled: canonical.rescaled,
            coincident: canonical.clamped,
            clamped_controls: blocks.clamped,
        },
    }
}
