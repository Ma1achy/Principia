//! Energy normalisation `η_E` (dd_decoder §3.7; chart_reference §0.6; REQ-DEC-024, REQ-DEC-041): the optional
//! post-momentum rescale that sets the decoded energy to a target `E*`,
//!
//! ```text
//! pᵢ ← η_E·pᵢ,    η_E = √((E* − U)/K₀),    feasible iff E* ≥ U
//! ```
//!
//! with `U = V₀` and `K₀` the decoded IC's (§3.6).
//!
//! **The off switch** (R-25): the target is an `Option`. `None` is off, and the momenta are untouched, bit for bit.
//! `Some(0)` is a real target, the energy set to zero, never "off". On a chart whose `forbids_energy_normalisation` is
//! set, the invariant-momentum charts, the rescale is never applied: chart-aware validation refuses such a view with
//! any `Some(E*)` before anything decodes (R-124; inverse_encode_contract's flag table), so the decoder reads the
//! target there as `None`. [`decode_chart`] reads the flag from the chart itself ([`Chart::forbids_energy_normalisation`]).
//!
//! **The outcomes** (dd_decoder §3.7, in its order; [`Outcome`]): the collision pole `U = −∞`, where a pair separation
//! is 0, and the rest start `K₀ = 0` at `E* > U`, are tagged `DEGENERATE(ENERGY_UNSCALABLE)`; `E* < U` is tagged
//! `DEGENERATE(ENERGY_INFEASIBLE)`; the rest start at `E* = U` is already at the target and untouched; otherwise
//! `η_E` is applied. A tagged decode keeps its momenta as decoded, and a decode the mass block has tagged keeps that
//! tag.
//!
//! **The scaled form.** `K₀ = 0` means every `pᵢ` is 0, tested as `s = 0` with `s` the largest `|p_{i,x}|, |p_{i,y}|`
//! (exact: no rounding). The rescale is formed on `p̂ᵢ = pᵢ/s`: `η_E·pᵢ = √((E* − U)/K̂)·p̂ᵢ`, `K̂ = K₀/s² ≥ ½`, since
//! one component of `p̂` is ±1 and every `mᵢ ≤ 1`. So `η_E` is finite wherever the rest start is excluded, even where
//! `(E* − U)/K₀` overflows or `K₀` rounds to 0 with a non-zero momentum.
//!
//! **The branch inputs** (dd_decoder §2's comparison-only rule; R-436): `E* < V₀`, `E* = V₀`, `s = 0` and
//! `r_min_pair_0 = 0`, comparisons of emitted values with no arithmetic at the decision. `V₀` and `r_min_pair_0` are
//! the descriptor's, so the tag reads the values the descriptor stores. The collision pole is tested on the
//! separation, and `V₀` is replaced by 0 there before any arithmetic or comparison reads it, so nothing here computes
//! with `∞`. Each choice is a `select` between values already computed.

use spirv_std::num_traits::Float;

use crate::chart::Chart;
use crate::decode::degenerate::Degenerate;
use crate::decode::descriptor::{describe, kinetic};
use crate::decode::{decode_with as decode_free_with, Blocks, Decoded, Free, Ic, Latent};
use crate::generated::links::{LinkReal, Selection};
use crate::payload::{ICDescriptorOf, PayloadReal};

/// What `η_E` did to a decode (dd_decoder §3.7). Its discriminant is a `u32`: the kernel's SPIR-V, where the decode
/// stage applies `η_E`, has no 8-bit integers (no `OpCapability Int8`), as [`Degenerate`]'s.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Outcome {
    /// The target is `None`, or the chart forbids the rescale: the momenta are untouched.
    Off,
    /// `η_E` applied: the energy is `E*` to rounding.
    Scaled,
    /// The rest start at `E* = U`: already at the target, the momenta untouched, no tag.
    AtTarget,
    /// `E* < U`: `DEGENERATE(ENERGY_INFEASIBLE)`, the momenta untouched.
    Infeasible,
    /// The collision pole, or the rest start at `E* > U`: `DEGENERATE(ENERGY_UNSCALABLE)`, the momenta untouched.
    Unscalable,
}

impl Outcome {
    /// The `DEGENERATE` reason the outcome raises, if any.
    #[inline]
    pub fn tag(self) -> Option<Degenerate> {
        match self {
            Outcome::Off | Outcome::Scaled | Outcome::AtTarget => None,
            Outcome::Infeasible => Some(Degenerate::EnergyInfeasible),
            Outcome::Unscalable => Some(Degenerate::EnergyUnscalable),
        }
    }
}

/// The rescaled momenta and the outcome.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Normalised<R> {
    /// The particle momenta after `η_E`: `η_E·pᵢ` where [`Outcome::Scaled`], the decoded `pᵢ` otherwise.
    pub p: [[R; 2]; 3],
    /// What `η_E` did.
    pub outcome: Outcome,
}

/// `t` where `c` holds, else `f`: both operands are computed before the choice, so it is a select, not a branch.
#[inline]
fn select<T: Copy>(c: bool, t: T, f: T) -> T {
    if c {
        t
    } else {
        f
    }
}

/// `s`, the largest `|p_{i,x}|, |p_{i,y}|`: 0 iff every `pᵢ` is 0, the rest start.
#[inline]
pub fn largest<R: LinkReal>(p: &[[R; 2]; 3]) -> R {
    let a = |x: R| Float::abs(x);
    let m0 = Float::max(a(p[0][0]), a(p[0][1]));
    let m1 = Float::max(a(p[1][0]), a(p[1][1]));
    let m2 = Float::max(a(p[2][0]), a(p[2][1]));
    Float::max(Float::max(m0, m1), m2)
}

/// `η_E` on the decoded IC `ic` toward the finite target `e_star` (dd_decoder §3.7), reading the descriptor's `V₀`
/// (`v0`) and `r_min_pair_0` (`r_min`) of the same IC.
#[inline]
pub fn normalise<R: LinkReal>(ic: &Ic<R>, v0: R, r_min: R, e_star: R) -> Normalised<R> {
    let zero = R::zero();
    let one = R::one();
    let collision = r_min == zero;
    let u = select(collision, zero, v0);
    let s = largest(&ic.p);
    let at_rest = s == zero;
    let d = select(at_rest, one, s);
    let p = ic.p;
    let unit = [
        [p[0][0] / d, p[0][1] / d],
        [p[1][0] / d, p[1][1] / d],
        [p[2][0] / d, p[2][1] / d],
    ];
    let k_hat = kinetic(&Ic {
        m: ic.m,
        r: ic.r,
        p: unit,
    });
    let k_hat = select(at_rest, one, k_hat);
    let eta = Float::sqrt(Float::max(e_star - u, zero) / k_hat);
    let scaled = [
        [eta * unit[0][0], eta * unit[0][1]],
        [eta * unit[1][0], eta * unit[1][1]],
        [eta * unit[2][0], eta * unit[2][1]],
    ];
    let rest = select(e_star == u, Outcome::AtTarget, Outcome::Unscalable);
    let moving = select(at_rest, rest, Outcome::Scaled);
    let feasible = select(e_star < u, Outcome::Infeasible, moving);
    let outcome = select(collision, Outcome::Unscalable, feasible);
    Normalised {
        p: select(outcome == Outcome::Scaled, scaled, p),
        outcome,
    }
}

/// `η_E` on a decode, under the target `e_star` and the chart's `forbids_energy_normalisation`: the IC's momenta
/// rescaled where [`Outcome::Scaled`], the descriptor rewritten from the result through `blocks`, and the outcome's tag
/// added where the decode has none. The decode's `momentum` stays the momentum block's output, before `η_E`.
#[inline]
pub fn apply<R: LinkReal + PayloadReal, B: Blocks<R>>(
    blocks: &mut B,
    decoded: Decoded<R>,
    e_star: Option<R>,
    forbids_energy_normalisation: bool,
) -> (Decoded<R>, Outcome) {
    let target = match e_star {
        Some(e) if !forbids_energy_normalisation => e,
        _ => return (decoded, Outcome::Off),
    };
    let (ic, n) = rescaled(&decoded, target);
    let descriptor = blocks.invariants(&ic);
    joined(decoded, ic, descriptor, n)
}

/// [`apply`] with the shared derived invariants, [`describe`], as every decoder's blocks write them: the decode stage's
/// form (`crate::stage::decode::sample_with`). It takes no branch on a decode: the rescale is computed whatever the
/// target, and the switch, a uniform's (`None`, or a chart that forbids it), selects between the rescaled momenta and
/// the decoded ones, so the kernel's SPIR-V carries no decode as a value across a branch. With the switch off the
/// momenta are the decode's, and the descriptor `describe` rewrites from them is the decode's, bit for bit, as every
/// decoder writes its descriptor by `describe` of its `(m, r, p)`.
#[inline]
pub fn apply_shared<R: LinkReal + PayloadReal>(
    decoded: Decoded<R>,
    e_star: Option<R>,
    forbids_energy_normalisation: bool,
) -> (Decoded<R>, Outcome) {
    let (on, target) = match e_star {
        Some(e) => (!forbids_energy_normalisation, e),
        None => (false, R::zero()),
    };
    let (ic, n) = rescaled(&decoded, target);
    let ic = Ic {
        p: select(on, ic.p, decoded.ic.p),
        ..ic
    };
    let n = Normalised {
        p: ic.p,
        outcome: select(on, n.outcome, Outcome::Off),
    };
    let descriptor = describe(&ic);
    joined(decoded, ic, descriptor, n)
}

/// [`normalise`] on `decoded` toward `target`, with the descriptor's `V₀` and `r_min_pair_0`, and the IC with its
/// momenta replaced by the result.
#[inline]
fn rescaled<R: LinkReal + PayloadReal>(decoded: &Decoded<R>, target: R) -> (Ic<R>, Normalised<R>) {
    let d: &ICDescriptorOf<R> = &decoded.descriptor;
    let n = normalise(&decoded.ic, d.V_0, d.r_min_pair_0, target);
    (
        Ic {
            p: n.p,
            ..decoded.ic
        },
        n,
    )
}

/// `decoded` with `ic` and its `descriptor`, rewritten from the result (unchanged momenta give it back bit for bit, as
/// `describe` reads only the IC), and the outcome's tag added where the decode has none.
#[inline]
fn joined<R: LinkReal + PayloadReal>(
    decoded: Decoded<R>,
    ic: Ic<R>,
    descriptor: ICDescriptorOf<R>,
    n: Normalised<R>,
) -> (Decoded<R>, Outcome) {
    (
        Decoded {
            ic,
            descriptor,
            degenerate: decoded.degenerate.or(n.outcome.tag()),
            ..decoded
        },
        n.outcome,
    )
}

/// Decodes `z` through `blocks`, mass → config → momentum → `η_E` → derived invariants: [`crate::decode::decode_with`],
/// then [`apply`].
#[inline]
pub fn decode_with<R: LinkReal + PayloadReal, B: Blocks<R>>(
    blocks: &mut B,
    z: &Latent<R>,
    e_star: Option<R>,
    forbids_energy_normalisation: bool,
) -> (Decoded<R>, Outcome) {
    let decoded = decode_free_with(blocks, z);
    apply(blocks, decoded, e_star, forbids_energy_normalisation)
}

/// Decodes `z` under the selection `S` with free momentum, a chart that allows `η_E`, and the target `e_star`.
#[inline]
pub fn decode<S: Selection, R: LinkReal + PayloadReal>(
    z: &Latent<R>,
    e_star: Option<R>,
) -> (Decoded<R>, Outcome) {
    decode_with(&mut Free::<S>::new(), z, e_star, false)
}

/// `D ∘ Φ` at `(u, v)` under the selection `S` with `η_E` toward `e_star`, applied only where the chart allows it: its
/// own [`Chart::forbids_energy_normalisation`] (chart_reference §0.6). The decode is the chart's own
/// [`Chart::decode`], so a chart that sets its configuration explicitly (the shape sphere) is decoded as it does it,
/// never from `map`'s latent alone; `η_E` follows on its `ic`, the descriptor rewritten by the shared `describe`
/// ([`Free`]'s invariants).
#[inline]
pub fn decode_chart<S, R, C>(chart: &C, u: R, v: R, e_star: Option<R>) -> (Decoded<R>, Outcome)
where
    S: Selection,
    R: LinkReal + PayloadReal + From<C::Param>,
    C: Chart,
{
    apply(
        &mut Free::<S>::new(),
        chart.decode::<S, R>(u, v),
        e_star,
        chart.forbids_energy_normalisation(),
    )
}
