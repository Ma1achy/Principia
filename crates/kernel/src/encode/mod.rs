//! Encode `E`, physical state → `z` (inverse-encode contract Parts 1, 3 and 6; dd_encode §1–§3): the quotient map onto
//! the decode's canonical section, in the same shared source as the decode, inverted (dd_decoder §1; R-116):
//!
//! ```text
//! E  =  (block inverses) ∘ C
//! ```
//!
//! [`encode`] runs `C` (`crate::canon`: 1a CoM → 0 scale → 1b boost → 2 rotate → 3 mirror, R-23), then the block
//! inverses ([`blocks`]: config, momentum and mass, each through the selection's registry link inverse), and reports
//! what it discarded ([`report`]).
//! [`by_kind`] then encodes into the active chart, axis by axis by kind, and lands in latent `z` where the chart does
//! not represent the state (Parts 5 and 6, step 4).
//!
//! **T1, constant on gauge orbits** (Part 1; dd_encode §3.3): translation and boost are removed by `C`'s 1a and 1b,
//! rotation by step 2, scale by step 0, and the mirror by step 3, with R-82's deterministic tie: `|λ̃_y| ≤ δ_λ` is no
//! mirror, the same branch on every backend (`crate::canon`'s note). `β` reads `|λ̃_y|` (R-438), so the deadband's two
//! sides give the same `β`. On that band, exactly collinear states included, `x` and `Π·x` are both left unmirrored,
//! so `E(Π·x)` and `E(x)` differ in the momenta's `y` components: RQ-360 holds T1 there for the mirror.
//!
//! **One source** (REQ-SYS-015; R-116): the block inverses are the registry's (`crate::generated::links`), the Jacobi
//! map is the decoder's run backwards, and `C` is the decode's. The fragment WGSL encode is never written by hand: it is
//! generated from this source by the rust-gpu → SPIR-V → WGSL translation, as `crate::stage::encode`'s compute entry
//! point already is.
//!
//! **At the exact `α`-poles** (REQ-DEC-014; R-21, R-133, R-437): at `‖λ̃‖ = 0`, `α = 0` and `β`'s select gives 0; at
//! `‖ρ̃‖ = 0`, `α = π/2` and `C`'s `φ` is 0 by its own select. Each gives a deterministic `z`, no NaN, and raises
//! `lookup_clamped`: `s_α` is 0 or 1, and the `α` link's inverse clamps it; at `ρ = 0` `C` raises it too (R-13).

pub mod blocks;
pub mod by_kind;
pub mod report;

pub use blocks::{Inverted, Jacobi, SimplexClamp};
pub use report::Report;

use crate::canon::{canonicalise, Canonical};
use crate::decode::{Ic, Latent};
use crate::generated::links::{LinkReal, Selection};
use crate::real::IeeeBits;

/// An encoded state: `z`, `C`'s canonical state, the block inverses' intermediates and the discard report.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Encoded<R> {
    /// The eight controls, in the latent block layout.
    pub z: Latent<R>,
    /// `C`'s output: the canonical `(m, r, p)`, `I_in`, `λ`, `φ`, `λ̃_y` and its notices.
    pub canonical: Canonical<R>,
    /// The block inverses' output: `z`, `α`, `β`, the Jacobi coordinates and the clamp bits.
    pub blocks: Inverted<R>,
    /// What encode discarded.
    pub report: Report<R>,
}

/// `E(ic) = (block inverses) ∘ C (ic)` under the selection `S`. `ic.m` is positive with `Σmᵢ = 1`, as `C` requires.
#[inline]
pub fn encode<S, R>(ic: &Ic<R>) -> Encoded<R>
where
    S: Selection,
    S::MassLink: SimplexClamp,
    R: LinkReal + IeeeBits,
{
    let canonical = canonicalise(ic);
    let blocks = blocks::invert::<S, R>(&canonical.ic);
    Encoded {
        z: Latent::new(blocks.z),
        canonical,
        blocks,
        report: Report {
            lambda: canonical.lambda,
            coincident: canonical.clamped,
            clamped_controls: blocks.clamped,
            mirrored: canonical.mirrored,
        },
    }
}
