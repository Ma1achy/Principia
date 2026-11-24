//! The retired `β` links and the conversion into the periodic `β` (dd_decoder §3.2).
//!
//! Latent `β` is a periodic angle with one link, `periodic_beta`, the offset onto the angle: `β = z_β + π/2`
//! read modulo `2π` by the decode's `cos β` and `sin β`. The three bounded links under which older records stored
//! `z_β` (`sigmoid_beta`, `tanh_beta` and `softsign_beta`, onto `β ∈ [0, π]`) are no block's link: their
//! codomain, `RetiredBeta`, is no slot's, so no [`Selection`](crate::generated::links::Selection) names one. Their
//! forward maps stay, as conversion sources only:
//! - **Conversion** ([`convert`], [`converted`]): a stored `z_β` made under the retired link `L` becomes the periodic
//!   link's inverse of the `β` it decoded to, `z_β′ = L(z_β) − π/2` (dd_decoder § 3.2, "How a stored value
//!   converts"), one period, so stored reduced and unreduced alike: the old golden, `z_β = 0` under `sigmoid_beta`,
//!   converts to 0. The subtraction rounds once and the decode adds `π/2` back, so a converted value need not decode
//!   bit for bit to the old state, at f64 or at f32; it is held to the old system within `ε_phys`
//!   (`tests/latent_conversion.rs`).
//! - **Recreation** ([`decode_retired`]): a record made under `L` decodes through `L` itself, bitwise, at the
//!   decode's own precision: `L`'s forward of `z_β` at `R` is the angle `β`, which [`Retired`]'s configuration block
//!   takes as it is, in place of the periodic link's; every other block is the selection's. Nothing is converted on
//!   recreation.
//!
//! [`RetiredLink`] names the three at run time, for a reader that learns the link from a file; [`ASSUMED`] is
//! the link an unpinned old saved chart is read under, with a notice.

use core::marker::PhantomData;

use super::config::{self, Config};
use super::mass::Masses;
use super::momentum::Momentum;
use super::{decode_with, Blocks, Decoded, Free, Ic, Latent};
use crate::generated::links::{Link, LinkReal, PeriodicBeta, RetiredBeta, Selection};
#[cfg(not(target_arch = "spirv"))]
use crate::generated::links::{SigmoidBeta, SoftsignBeta, TanhBeta};
use crate::payload::ICDescriptorOf;
use crate::payload::PayloadReal;

/// `z_β` made under the retired link `L`, converted to the periodic link's control at `R`: the periodic link's inverse
/// of `L`'s forward, `L(z_β) − π/2`.
#[inline]
pub fn convert<L: Link<1, 1, Codomain = RetiredBeta>, R: LinkReal>(z_beta: R) -> R {
    let [beta] = L::forward([z_beta]);
    let [z] = PeriodicBeta::inverse([beta]);
    z
}

/// A `z_β` that was stored as an f32 under the retired link `L`, converted: [`convert`] at f64 of the stored f32
/// itself, widened, then rounded once to the nearest f32. The stored number is the f32, not the decimal it was typed
/// as (`-0.3_f32` is −0.30000001192…), and the two can round to different f32 after conversion; a stored f32 control
/// is converted through this, never by hand from its decimal.
#[cfg(not(target_arch = "spirv"))]
#[inline]
pub fn convert_stored_f32<L: Link<1, 1, Codomain = RetiredBeta>>(z_beta: f32) -> f32 {
    convert::<L, f64>(f64::from(z_beta)) as f32
}

/// `z` with its `z_β` converted from the retired link `L` ([`convert`]), the other seven controls as they are.
#[inline]
pub fn converted<L: Link<1, 1, Codomain = RetiredBeta>, R: LinkReal>(z: &Latent<R>) -> Latent<R> {
    let mut out = *z;
    out.z[1] = convert::<L, R>(z.z[1]);
    out
}

/// The free-momentum decoder under the selection `S` with `β` through the retired link `L`: [`Free`]'s blocks,
/// the configuration block's `β` `L`'s forward of `z_β`, taken as the angle: the decode an old record was made with.
pub struct Retired<S, L>(Free<S>, PhantomData<L>);

impl<S, L> Retired<S, L> {
    /// The decoder.
    #[inline]
    pub fn new() -> Self {
        Retired(Free::new(), PhantomData)
    }
}

impl<S, L> Default for Retired<S, L> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<S, L, R> Blocks<R> for Retired<S, L>
where
    S: Selection,
    L: Link<1, 1, Codomain = RetiredBeta>,
    R: LinkReal + PayloadReal,
{
    #[inline]
    fn mass(&mut self, z_mass: [R; 2]) -> Masses<R> {
        self.0.mass(z_mass)
    }

    #[inline]
    fn config(&mut self, z_cfg: [R; 2], masses: &Masses<R>) -> Config<R> {
        let [alpha] = <S::AlphaLink as Link<1, 1>>::forward([z_cfg[0]]);
        let [beta] = L::forward([z_cfg[1]]);
        config::from_angles(alpha, beta, masses)
    }

    #[inline]
    fn momentum(&mut self, z_mom: [R; 4], masses: &Masses<R>, c: &Config<R>) -> Momentum<R> {
        self.0.momentum(z_mom, masses, c)
    }

    #[inline]
    fn invariants(&mut self, ic: &Ic<R>) -> ICDescriptorOf<R> {
        <Free<S> as Blocks<R>>::invariants(&mut self.0, ic)
    }
}

/// The decode under the selection `S` of `z` made under the retired `β` link `L`: [`Retired`]'s, `L`'s forward
/// of `z_β` at the decode's precision taken as `β`, then the one decode's other steps: the decode an old record was
/// made with under `L`, bit for bit at f64 and at f32.
#[inline]
pub fn decode_retired<S, L, R>(z: &Latent<R>) -> Decoded<R>
where
    S: Selection,
    L: Link<1, 1, Codomain = RetiredBeta>,
    R: LinkReal + PayloadReal,
{
    decode_with(&mut Retired::<S, L>::new(), z)
}

/// A retired `β` link, by name, for a reader that learns it from what it reads.
#[cfg(not(target_arch = "spirv"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetiredLink {
    /// `sigmoid_beta`, the old default `β` link.
    Sigmoid,
    /// `tanh_beta`.
    Tanh,
    /// `softsign_beta`.
    Softsign,
}

/// The link an old saved chart that pins none was made under, the old default: its conversion is
/// reported, naming it, for the notice on load.
#[cfg(not(target_arch = "spirv"))]
pub const ASSUMED: RetiredLink = RetiredLink::Sigmoid;

#[cfg(not(target_arch = "spirv"))]
impl RetiredLink {
    /// The three, in the registry's order.
    pub const ALL: [RetiredLink; 3] = [
        RetiredLink::Sigmoid,
        RetiredLink::Tanh,
        RetiredLink::Softsign,
    ];

    /// Its registry name.
    pub fn name(self) -> &'static str {
        match self {
            RetiredLink::Sigmoid => SigmoidBeta::NAME,
            RetiredLink::Tanh => TanhBeta::NAME,
            RetiredLink::Softsign => SoftsignBeta::NAME,
        }
    }

    /// The link named `name`; `None` for any other name.
    pub fn named(name: &str) -> Option<RetiredLink> {
        RetiredLink::ALL.into_iter().find(|l| l.name() == name)
    }

    /// [`convert`] under this link.
    #[inline]
    pub fn convert<R: LinkReal>(self, z_beta: R) -> R {
        match self {
            RetiredLink::Sigmoid => convert::<SigmoidBeta, R>(z_beta),
            RetiredLink::Tanh => convert::<TanhBeta, R>(z_beta),
            RetiredLink::Softsign => convert::<SoftsignBeta, R>(z_beta),
        }
    }
}
