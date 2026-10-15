//! The shared decoder `D`, the Matter rung (dd_decoder §1, §3): `z → mass × config × momentum → (m, r, p)`, one
//! source for the f32 kernel and the f64 CPU path (R-265), generic over the kernel's `Real` and over the link
//! [`Selection`], so each selection is a monomorphised variant (REQ-GEN-013; lowering Part 2).
//!
//! The blocks, in the decode order mass → config → momentum → derived invariants (chart_decoder_contract Part 2):
//! - [`latent`]: the 8-component control vector and its fixed block layout.
//! - [`mass`]: `D_mass`, the simplex point and the one tag the block may raise, `DEGENERATE(M01_TINY)`.
//! - [`config`]: `D_cfg`, the canonical-frame hyperspherical decode, unweighted and reconstructed.
//! - [`momentum`]: `D_mom`, the free Jacobi momenta and the Jacobi-to-particle map.
//! - [`descriptor`]: the derived invariants, the `ICDescriptor` filled from the decoded `(m, r, p)`.
//! - [`degenerate`]: the `DEGENERATE` reason type.
//! - [`units`]: the dimensionless system `G = M = I = 1`.
//!
//! **The ordered entry** ([`decode`], through [`decode_with`]; REQ-DEC-001, REQ-DEC-004): `D = D_mass × D_cfg × D_mom`
//! over the blocks `z[6:8]`, `z[0:2]` and `z[2:6]`, evaluated mass → config → momentum → derived invariants, each
//! block reading its own `z` slice and the blocks before it, never a later one ([`Blocks`]' signatures). The free
//! momentum block reads no configuration; the configuration is passed to it so that an invariant-momentum block
//! (dd_decoder §3.4) reads the already-decoded masses and configuration through the same entry. The derived
//! invariants read only the joined `(m, r, p)`. The output is the canonical `(m, r, p)` ([`Ic`], seam 3), the block
//! outputs, the descriptor and the `DEGENERATE` tag: a tagged decode is kept, never dropped.
//!
//! **No guard** (REQ-DEC-028; validation ground-truth note, "Degenerate configs"): the decoder accepts any finite `z`
//! (R-407), and nothing in it clamps, lands on or excludes an endpoint. In exact arithmetic degenerate configurations
//! are approached as limits; in floating point the range is closed (R-417, dd_decoder §3.2), the lower poles reached
//! only by the links' own rounding where their output underflows. The only tag is the narrow, enumerated
//! `DEGENERATE` cause.

pub mod config;
pub mod degenerate;
pub mod descriptor;
pub mod latent;
pub mod mass;
pub mod momentum;
pub mod units;

use core::marker::PhantomData;

pub use degenerate::Degenerate;
pub use latent::Latent;

use crate::generated::links::{LinkReal, Selection};
use crate::payload::{ICDescriptorOf, PayloadReal};
use config::Config;
use mass::Masses;
use momentum::Momentum;

/// The canonical `(m, r, p)`, the decoder's output and the integrator's one input type (dd_decoder §2; seam 3): CoM at
/// the origin, `ρ̃` on `+x`, `λ̃_y ≥ 0`, `G = M = I = 1`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ic<R> {
    /// The masses `m₀, m₁, m₂`, `Σmᵢ = 1`.
    pub m: [R; 3],
    /// The positions `r₀, r₁, r₂`.
    pub r: [[R; 2]; 3],
    /// The momenta `p₀, p₁, p₂`, `Σpᵢ = 0`.
    pub p: [[R; 2]; 3],
}

/// The four stages of the decode, in the order [`decode_with`] calls them; each takes its `z` slice and the outputs
/// of the stages before it, and nothing later.
pub trait Blocks<R: PayloadReal> {
    /// `D_mass`, from `z[6:8]`.
    fn mass(&mut self, z_mass: [R; 2]) -> Masses<R>;
    /// `D_cfg`, from `z[0:2]` and the masses.
    fn config(&mut self, z_cfg: [R; 2], masses: &Masses<R>) -> Config<R>;
    /// `D_mom`, from `z[2:6]`, the masses and the configuration.
    fn momentum(&mut self, z_mom: [R; 4], masses: &Masses<R>, config: &Config<R>) -> Momentum<R>;
    /// The derived invariants, the `ICDescriptor`, from the joined `(m, r, p)`.
    fn invariants(&mut self, ic: &Ic<R>) -> ICDescriptorOf<R>;
}

/// The free-momentum decoder under the selection `S`: the blocks of [`mass`], [`config`] and [`momentum`], and
/// [`descriptor::describe`].
pub struct Free<S>(PhantomData<S>);

impl<S> Free<S> {
    /// The decoder.
    #[inline]
    pub fn new() -> Self {
        Free(PhantomData)
    }
}

impl<S> Default for Free<S> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Selection, R: LinkReal + PayloadReal> Blocks<R> for Free<S> {
    #[inline]
    fn mass(&mut self, z_mass: [R; 2]) -> Masses<R> {
        mass::decode::<S, R>(z_mass)
    }

    #[inline]
    fn config(&mut self, z_cfg: [R; 2], masses: &Masses<R>) -> Config<R> {
        config::decode::<S, R>(z_cfg, masses)
    }

    #[inline]
    fn momentum(&mut self, z_mom: [R; 4], masses: &Masses<R>, _config: &Config<R>) -> Momentum<R> {
        momentum::decode::<S, R>(z_mom, masses)
    }

    #[inline]
    fn invariants(&mut self, ic: &Ic<R>) -> ICDescriptorOf<R> {
        descriptor::describe(ic)
    }
}

/// A decoded `z`: the canonical `(m, r, p)`, each block's output, the `ICDescriptor` and the `DEGENERATE` tag.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Decoded<R: PayloadReal> {
    /// The canonical `(m, r, p)`, the integrator's input.
    pub ic: Ic<R>,
    /// The mass block's output.
    pub masses: Masses<R>,
    /// The configuration block's output.
    pub config: Config<R>,
    /// The momentum block's output.
    pub momentum: Momentum<R>,
    /// The `ICDescriptor`, written post-decode and pre-integration.
    pub descriptor: ICDescriptorOf<R>,
    /// `Some(reason)` for a `DEGENERATE` decode, which is kept, never dropped.
    pub degenerate: Option<Degenerate>,
}

/// Decodes `z` through `blocks`, in the order mass → config → momentum → derived invariants.
#[inline]
pub fn decode_with<R: PayloadReal, B: Blocks<R>>(blocks: &mut B, z: &Latent<R>) -> Decoded<R> {
    let masses = blocks.mass(z.mass());
    let config = blocks.config(z.config(), &masses);
    let momentum = blocks.momentum(z.momentum(), &masses, &config);
    let ic = Ic {
        m: masses.m,
        r: config.r,
        p: momentum.p,
    };
    let descriptor = blocks.invariants(&ic);
    Decoded {
        ic,
        masses,
        config,
        momentum,
        descriptor,
        degenerate: masses.degenerate,
    }
}

/// Decodes `z` under the selection `S` with free momentum ([`Free`]).
#[inline]
pub fn decode<S: Selection, R: LinkReal + PayloadReal>(z: &Latent<R>) -> Decoded<R> {
    decode_with(&mut Free::<S>::new(), z)
}
