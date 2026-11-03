//! Physical-quantity axes: warped by default, linear as an option per axis, the warp evaluated per pixel (R-471 E1 to
//! E3, H1 to H7; R-506 W1 to W5; R-508 V1 to V5; chart_reference §2.1; chart_decoder_contract § "The four axis kinds — a
//! closed set", § "Past the unit square — each axis's extension type (R-407)").
//!
//! **The quantities and their warps** ([`Quantity`], [`written`]). The energy-type axes of kind 3, `E`, `L_z`, `K` and
//! `KE` (`K`'s quantity), take chart_reference §2.1's warp, each alone its own line of it: `K = K_max·t^γ_K`,
//! `E = U + K_max·t^γ_K`, `L_z = (2s − 1)·L_max` with `L_max = √(2IK)` at the kinetic energy the axis reads. The virial
//! ratio and the mass ratio take the warps §2.1 writes for them (REQ-CHART-070, R-72), built in `super::physical`. `PE`
//! has no warp: §2.1 writes that no chart offers it, so [`written`] refuses it, and the well-posedness validator with it
//! (`engine::chart_authoring::validate`).
//!
//! **The mapping** ([`Mapping`]; R-471 E2, H4, H7). `Warped` is the default and every existing chart's map, bit for bit.
//! `Linear` maps the axis affinely onto the warp's range at the slice centre: `K ∈ [0, K_max]`, `E ∈ [U₀, U₀ + K_max]`,
//! `L_z ∈ ±√(2·I₀·K_max)`, `U₀` and `I₀` the slice centre's (the canonical-frame decode's `I₀ = 1`). Its infeasible
//! pixels are not a system, refused by `validate`, which the chart classification draws blank and never integrates.
//! The mapping of each axis is a chart parameter, a sim key (`engine::contract::sim_config::Chart`).
//!
//! **Each axis's extension type** ([`extension`]; R-407, R-508, R-506 W4): a warped `t` axis (`K`, `KE`, `E`) is bounded
//! below `t = 0` and continues, affine, past `t = 1`; the warp's `s` axis (`L_z`) is bounded on both sides; the virial
//! ratio and the mass ratio, warped, are bounded on both sides; under the linear option every axis is affine, R-408's
//! B1 axis.
//!
//! **The invariant charts and the Burrau family's `K` axes under the linear option** ([`Mapped`]). The warped map is the
//! chart's own: [`Mapped`] with every axis warped decodes, maps and validates as the chart it wraps, bit for bit. Under
//! the linear option the targets `(L_z, K*)` come from [`Rule`] and the construction runs in [`RuleBlocks`], which reads
//! the pixel's own potential `U` where the rule needs it (`E`'s linear map, `K* = E − U`, E3).
//!
//! **Per pixel** (R-471 E3, H6; R-473 T5). Every target is formed from the pixel's own decoded masses and configuration,
//! after a tilt's hidden offsets: `I = 1` at every pixel (the canonical-frame decode), `K*` is `K(t)` itself on the warp
//! and `U` is read where an axis needs it. Untilted, those are the slice centre's at every pixel.

use core::fmt;
use core::marker::PhantomData;

use spirv_std::num_traits::Float;

use super::burrau::{self, embed, unread, BurrauBlocks};
use super::burrau_acute::BurrauThetaK;
use super::invariant::{self, Invariant, InvariantLE, InvariantLK, WarpReal};
use super::tilt::{shift_controls, Hidden, Shift, Tiltable};
use super::{Burrau, Chart, ChartDecode, ChartOut, Constraint, ValidationResult};
use crate::decode::config::{self, Config};
use crate::decode::invariant::{self as construction, infeasible, ExactSign, Outcome};
use crate::decode::mass::{self, Masses};
use crate::decode::momentum::Momentum;
use crate::decode::{self, descriptor, Blocks, Decoded, Degenerate, Ic, Latent};
use crate::generated::links::{LinkParams, LinkReal, Selection};
use crate::payload::{ICDescriptorOf, PayloadReal};

/// How an axis maps `[0, 1]` onto its quantity (R-471 E1, E2; R-506): warped by default, linear as an option. A `u32`
/// discriminant, a type the kernel's SPIR-V has.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum Mapping {
    /// chart_reference §2.1's warp, or the warp §2.1 writes for the quantity: every pixel of `[0,1]²` feasible.
    #[default]
    Warped,
    /// Affine onto the warp's range at the slice centre (a Chart-builder axis onto its own range): its infeasible pixels
    /// are not a system.
    Linear,
}

/// A physical quantity an axis may show (chart_decoder_contract Part 3, kind 3's list; render_gui_spec §G7's physical
/// quantities, R-506).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quantity {
    /// The energy `E = K + U`.
    E,
    /// The angular momentum `L_z`.
    Lz,
    /// The kinetic energy `K`.
    K,
    /// The kinetic energy `KE`, `K`'s quantity.
    KE,
    /// The potential energy `PE`.
    PE,
    /// The virial ratio `Q = 2K/|U|`.
    VirialRatio,
    /// A mass ratio `m_b/m_a`.
    MassRatio,
}

impl Quantity {
    /// Every quantity, in the declaration order.
    pub const ALL: [Quantity; 7] = [
        Quantity::E,
        Quantity::Lz,
        Quantity::K,
        Quantity::KE,
        Quantity::PE,
        Quantity::VirialRatio,
        Quantity::MassRatio,
    ];

    /// Whether the quantity is energy-type, an axis of kind 3 (chart_decoder_contract Part 3).
    #[inline]
    pub fn energy_type(self) -> bool {
        matches!(
            self,
            Quantity::E | Quantity::Lz | Quantity::K | Quantity::KE | Quantity::PE
        )
    }
}

/// Why a physical-quantity axis is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refused {
    /// chart_reference §2.1 writes no warp for the quantity: no chart offers it (REQ-CHART-070, R-72; R-471 H2).
    NoWarpWritten(Quantity),
}

/// Whether chart_reference §2.1 writes a warp for `quantity`, so that a chart may offer an axis of it: every quantity
/// but `PE`, which §2.1's paragraph says no chart offers (REQ-CHART-070).
#[inline]
pub fn written(quantity: Quantity) -> Result<(), Refused> {
    match quantity {
        Quantity::PE => Err(Refused::NoWarpWritten(quantity)),
        _ => Ok(()),
    }
}

/// An axis's extension type past its edge (chart_decoder_contract Part 3, R-407's four).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtensionType {
    /// The formula continues.
    Affine,
    /// The axis wraps.
    Periodic,
    /// The axis continues over the pole, its partner shifted by half a period.
    PoleCrossing,
    /// Not a system past the edge: the domain's end.
    Bounded,
}

/// An axis's extension type on each side of its `[0, 1]`: `below` past `0`, `above` past `1`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sides {
    /// Past `0`.
    pub below: ExtensionType,
    /// Past `1`.
    pub above: ExtensionType,
}

impl Sides {
    /// One type on both sides.
    #[inline]
    pub const fn both(t: ExtensionType) -> Self {
        Sides { below: t, above: t }
    }
}

/// The extension type of an axis of `quantity` under `mapping` (the module's note): a warped `K`, `KE` or `E` axis
/// bounded below `t = 0` and affine past `t = 1` (R-508); a warped `L_z`, the warp's `s` axis, bounded on both sides; a
/// warped virial or mass ratio bounded on both sides (R-506 W4); every linear axis affine (B1); `PE` refused.
pub fn extension(quantity: Quantity, mapping: Mapping) -> Result<Sides, Refused> {
    written(quantity)?;
    Ok(sides(quantity, mapping))
}

/// [`extension`]'s types for a quantity a chart offers.
#[inline]
pub(super) fn sides(quantity: Quantity, mapping: Mapping) -> Sides {
    match (mapping, quantity) {
        (Mapping::Linear, _) => Sides::both(ExtensionType::Affine),
        (Mapping::Warped, Quantity::E | Quantity::K | Quantity::KE) => Sides {
            below: ExtensionType::Bounded,
            above: ExtensionType::Affine,
        },
        (Mapping::Warped, _) => Sides::both(ExtensionType::Bounded),
    }
}

/// How a pixel of a chart's window counts in R-408's area statistics, through its axes' types
/// (chart_decoder_contract Part 3, "Area statistics past `[0,1]²`").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Count {
    /// Past a bounded edge: the domain's end, out of the count and the total.
    DomainEnd,
    /// Outside a periodic or pole-crossing axis's primary range, its chart's `[0, 1]` span: a repeat, not counted.
    NotCounted,
    /// Inside every primary range, but not a system by `validate`: counted, forbidden.
    Forbidden,
    /// A counted system.
    System,
}

/// The count of the pixel at the chart point `at`, `[s, t]`, whose axes have the types `sides` and whose `validate`
/// accepts it where `valid` (chart_decoder_contract Part 3's order): past a bounded side the domain's end; past a
/// periodic or pole-crossing one not counted; an affine side's primary range is unbounded; then forbidden where not
/// valid, else a system. So a linear axis's unreachable pixels are forbidden, in the count and the total (B1), and a
/// warped axis's pixels past a bounded edge leave both (R-471 H3; R-506 W4).
pub fn count(sides: [Sides; 2], at: [f64; 2], valid: bool) -> Count {
    let side = |s: Sides, x: f64| {
        if x < 0.0 {
            Some(s.below)
        } else if x > 1.0 {
            Some(s.above)
        } else {
            None
        }
    };
    let types = [side(sides[0], at[0]), side(sides[1], at[1])];
    if types.contains(&Some(ExtensionType::Bounded)) {
        Count::DomainEnd
    } else if types.contains(&Some(ExtensionType::Periodic))
        || types.contains(&Some(ExtensionType::PoleCrossing))
    {
        Count::NotCounted
    } else if valid {
        Count::System
    } else {
        Count::Forbidden
    }
}

/// Whether a pixel inside `[0,1]²` that is not a system logs R-467's decoder-bug warning on a chart whose
/// physical-quantity axes have the mappings `mappings` (R-506 W1; R-471 H5): only where every one is warped, the chart
/// then valid there by construction; a linear axis's in-square failures are by design, and warn nothing.
#[inline]
pub fn decoder_bug_warning(mappings: &[Mapping]) -> bool {
    mappings.iter().all(|m| *m == Mapping::Warped)
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

// ── The targets rule and its blocks ──────────────────────────────────────────────────────────────────────────────────

/// A pixel's targets for §2.2's construction, read against its own potential `U` (the module's note):
/// `K* = k0 + weight·(u_ref − U)` where `weight ≠ 0`, else `k0` itself; `L_z = s2·L_max(K*)` on the warp's line
/// (`lz_on_warp`), else the fixed `lz`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rule<R> {
    /// `K*`'s part that reads no potential.
    pub k0: R,
    /// The potential's weight: 0 where `K*` reads none.
    pub weight: R,
    /// The reference potential: `U₀` for a linear `E`, 0 for the virial ratio.
    pub u_ref: R,
    /// `2s − 1`, the factor on `L_max(K*)` where `L_z` is on the warp's line.
    pub s2: R,
    /// The fixed `L_z` where it is not.
    pub lz: R,
    /// Whether `L_z = s2·L_max(K*)`.
    pub lz_on_warp: bool,
}

impl<R: WarpReal> Rule<R> {
    /// `K*` at the pixel's potential `u_pot`.
    #[inline]
    pub fn k_star(&self, u_pot: R) -> R {
        let read = self.k0 + self.weight * (self.u_ref - u_pot);
        select(self.weight == R::zero(), self.k0, read)
    }

    /// `L_z` at `K* = k`: on the warp's line `(2s − 1)·L_max(K*)`, `L_max` of a negative `K*` taken at 0, where the
    /// construction finds the pixel infeasible.
    #[inline]
    pub fn lz_at(&self, k: R) -> R {
        let on_warp = self.s2 * invariant::l_max(Float::max(k, R::zero()));
        select(self.lz_on_warp, on_warp, self.lz)
    }
}

/// The decoder of a [`Rule`]'s targets under the selection `S`: the shared mass and configuration blocks at the tunable
/// parameters, the construction (chart_reference §2.2) at the targets the rule gives on the pixel's own potential, its
/// seed R-82's decided on the decoded `z_α`, and the shared descriptor.
pub struct RuleBlocks<S, R> {
    rule: Rule<R>,
    z_alpha: R,
    outcome: Outcome,
    params: LinkParams<R>,
    links: PhantomData<S>,
}

impl<S, R: Float> RuleBlocks<S, R> {
    /// The decoder of `rule` at the tunable parameters `params`.
    #[inline]
    pub fn new(rule: Rule<R>, params: LinkParams<R>) -> Self {
        RuleBlocks {
            rule,
            z_alpha: R::zero(),
            outcome: Outcome::Feasible,
            params,
            links: PhantomData,
        }
    }

    /// What the construction did, once the momentum block has run.
    #[inline]
    pub fn outcome(&self) -> Outcome {
        self.outcome
    }
}

/// The potential `U` of the masses and configuration, `G = 1` (the descriptor's `V_0`).
#[inline]
pub fn potential<R: LinkReal + PayloadReal>(masses: &Masses<R>, config: &Config<R>) -> R {
    let zero = [R::zero(); 2];
    descriptor::potential(&Ic {
        m: masses.m,
        r: config.r,
        p: [zero; 3],
    })
}

impl<S: Selection, R: LinkReal + PayloadReal + ExactSign> Blocks<R> for RuleBlocks<S, R> {
    #[inline]
    fn mass(&mut self, z_mass: [R; 2]) -> Masses<R> {
        mass::decode_at::<S, R>(z_mass, &self.params)
    }

    #[inline]
    fn config(&mut self, z_cfg: [R; 2], masses: &Masses<R>) -> Config<R> {
        self.z_alpha = z_cfg[0];
        config::decode::<S, R>(z_cfg, masses)
    }

    /// The construction at the rule's targets on this pixel's potential; the momentum controls are not read.
    #[inline]
    fn momentum(&mut self, _z_mom: [R; 4], masses: &Masses<R>, config: &Config<R>) -> Momentum<R> {
        let k = self.rule.k_star(potential(masses, config));
        let lz = self.rule.lz_at(k);
        let c = construction::construct(masses, config, self.z_alpha, lz, k);
        self.outcome = c.outcome;
        invariant::jacobi(masses, c.p)
    }

    #[inline]
    fn invariants(&mut self, ic: &Ic<R>) -> ICDescriptorOf<R> {
        descriptor::describe(ic)
    }

    #[inline]
    fn tag(&self) -> Option<Degenerate> {
        self.outcome.tag()
    }
}

/// `D ∘ Φ` of the controls `z` under `rule` at the tunable parameters `p`: the decode and the construction's outcome.
#[inline]
pub fn decode_rule<S, R>(rule: Rule<R>, z: &Latent<R>, p: &LinkParams<R>) -> (Decoded<R>, Outcome)
where
    S: Selection,
    R: LinkReal + PayloadReal,
{
    let mut blocks = RuleBlocks::<S, R>::new(rule, *p);
    let decoded = decode::decode_with(&mut blocks, z);
    (decoded, blocks.outcome())
}

/// The potential `U₀` of the controls `z` at the tunable parameters `p`: the shared mass and configuration blocks'
/// decode, then [`potential`]. On a chart's frozen controls it is the slice centre's.
#[inline]
pub fn centre_potential<S, R>(z: &[R; 8], p: &LinkParams<R>) -> R
where
    S: Selection,
    R: LinkReal + PayloadReal,
{
    let masses = mass::decode_at::<S, R>([z[6], z[7]], p);
    let config = config::decode::<S, R>([z[0], z[1]], &masses);
    potential(&masses, &config)
}

// ── The mapped charts ────────────────────────────────────────────────────────────────────────────────────────────────

/// A chart with each physical-quantity axis's mapping, built for the link selection `S` (the module's note):
/// `mapping[0]` the horizontal axis's, `mapping[1]` the vertical's; on the Burrau family's `K` charts only the vertical
/// axis is a physical quantity, and `mapping[0]` is not read. Every axis warped, it is the chart it wraps, bit for bit.
///
/// **Its selection.** `validate` reads the pixel's decode where a linear axis's feasibility depends on it (a tilted
/// linear `E`, `K* = E − U`), and `validate` takes no selection, so the chart carries its own: `S`, the active links,
/// which the dispatch builds it for. Its decode reads `S` too, and the selection a call names is not read, as on the
/// Burrau charts, which read none.
pub struct Mapped<C, S> {
    /// The chart.
    pub chart: C,
    /// Each axis's mapping, `[horizontal, vertical]`.
    pub mapping: [Mapping; 2],
    links: PhantomData<fn() -> S>,
}

impl<C, S> Mapped<C, S> {
    /// `chart` under `mapping`.
    #[inline]
    pub fn new(chart: C, mapping: [Mapping; 2]) -> Self {
        Mapped {
            chart,
            mapping,
            links: PhantomData,
        }
    }

    /// Whether every physical-quantity axis is warped, so the chart is its own.
    #[inline]
    fn own(&self) -> bool {
        self.mapping[0] == Mapping::Warped && self.mapping[1] == Mapping::Warped
    }
}

impl<C: Clone, S> Clone for Mapped<C, S> {
    fn clone(&self) -> Self {
        Mapped::new(self.chart.clone(), self.mapping)
    }
}

impl<C: Copy, S> Copy for Mapped<C, S> {}

impl<C: PartialEq, S> PartialEq for Mapped<C, S> {
    fn eq(&self, other: &Self) -> bool {
        self.chart == other.chart && self.mapping == other.mapping
    }
}

impl<C: fmt::Debug, S> fmt::Debug for Mapped<C, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Mapped")
            .field("chart", &self.chart)
            .field("mapping", &self.mapping)
            .finish()
    }
}

/// An invariant chart's targets at `(s, t)` under `mapping` (the module's note): the vertical `K(t)` warped or
/// `K_max·t` linear, `E`'s chart (`energy`) reading the pixel's potential against `U₀ = u_ref` where linear; the
/// horizontal `L_z` on the warp's line, or `(2s − 1)·L_max(K_max)` linear, `I₀ = 1`.
#[inline]
pub fn invariant_rule<R, P>(
    inv: &Invariant<P>,
    mapping: [Mapping; 2],
    energy: bool,
    u_ref: R,
    s: R,
    t: R,
) -> Rule<R>
where
    R: WarpReal + From<P>,
    P: Copy,
{
    let wide = <R as From<P>>::from;
    let (k_max, gamma) = (wide(inv.k_max), wide(inv.gamma_k));
    let linear_t = mapping[1] == Mapping::Linear;
    let s2 = s + s - R::one();
    Rule {
        k0: select(linear_t, k_max * t, invariant::kinetic(k_max, gamma, t)),
        weight: select(linear_t & energy, R::one(), R::zero()),
        u_ref,
        s2,
        lz: s2 * invariant::l_max(k_max),
        lz_on_warp: mapping[0] == Mapping::Warped,
    }
}

/// A mapped invariant chart's decode at `(s, t)` from the frozen controls `z`, the chart's own or a tilt's with its
/// hidden offsets added: `U₀` is the slice centre's, the chart's own frozen controls', read only by a linear `E`.
#[inline]
pub fn invariant_decode<S, R, P>(
    inv: &Invariant<P>,
    mapping: [Mapping; 2],
    energy: bool,
    s: R,
    t: R,
    z: &Latent<R>,
    p: &LinkParams<R>,
) -> (Decoded<R>, Outcome)
where
    S: Selection,
    R: LinkReal + PayloadReal + From<P>,
    P: Copy,
{
    let reads_u = energy & (mapping[1] == Mapping::Linear);
    let u_ref = if reads_u {
        centre_potential::<S, R>(&inv.controls(), p)
    } else {
        R::zero()
    };
    decode_rule::<S, R>(invariant_rule(inv, mapping, energy, u_ref, s, t), z, p)
}

/// The `s` nearest `u` at which a linear `L_z` is feasible at `K* = k`, at the same `t`: the parabola's edge on `u`'s
/// side, `½ ± ½·L_max(K*)/L_max(K_max)`, its offset from `½` shrunk by four ulps at a time while the construction's
/// exact test still finds it infeasible, eight steps at most, then `½` itself, where `L_z = 0`. Each step moves `L_z` by a
/// few of its own ulps wherever `s` lies (an ulp of `s` near 0 is far finer than one of `L_z`).
fn nearest_linear_s(k_max: f64, k: f64, u: f64) -> f64 {
    let lin = invariant::l_max(k_max);
    let reach = 0.5 * invariant::l_max(k) / lin;
    let mut offset = select(u > 0.5, reach, -reach);
    for _ in 0..8 {
        let s = 0.5 + offset;
        if !infeasible(1.0, k, (s + s - 1.0) * lin) {
            return s;
        }
        offset *= 1.0 - 4.0 * f64::EPSILON;
    }
    0.5
}

/// What a mapped invariant chart's `validate` names: whether its vertical axis is `E`, the constraint below `t = 0`
/// (`low`) and the one past the parabola (`wide`), as the chart's own `validate` names them.
#[derive(Clone, Copy)]
struct Checks {
    energy: bool,
    low: Constraint,
    wide: Constraint,
}

/// A mapped invariant chart's `validate` at `(u, v)`, untilted, not every axis warped. Below `t = 0` the point is
/// projected to `t = 0`, the warp's bounded side and the linear axis's `K < 0`; a warped `L_z` past `s`'s edges is
/// projected onto the edge, on the parabola; a linear `L_z` past the parabola at the point's `K*` is projected onto it
/// at the same `t` ([`nearest_linear_s`]). Untilted, every pixel's potential is the slice centre's (§2's fixed
/// geometry and masses), so `K*` is the rule's `k0` and no decode is read.
fn invariant_validate<P: Copy>(
    inv: &Invariant<P>,
    mapping: [Mapping; 2],
    checks: Checks,
    u: f64,
    v: f64,
) -> ValidationResult
where
    f64: From<P>,
{
    if !(u.is_finite() && v.is_finite()) {
        return ValidationResult::Reject {
            failed: select(v.is_finite(), checks.wide, checks.low),
        };
    }
    let t = v.max(0.0);
    let linear_s = mapping[0] == Mapping::Linear;
    let s = select(linear_s, u, u.clamp(0.0, 1.0));
    let rule = invariant_rule::<f64, P>(inv, mapping, checks.energy, 0.0, s, t);
    let k = rule.k0;
    let s = if linear_s && infeasible(1.0, k, rule.lz) {
        nearest_linear_s(f64::from(inv.k_max), k, s)
    } else {
        s
    };
    if s == u && t == v {
        ValidationResult::Valid
    } else {
        ValidationResult::Project {
            u: s,
            v: t,
            failed: select(t != v, checks.low, checks.wide),
        }
    }
}

/// A mapped invariant chart's `validate` at a tilt's `(a, b)` with its hidden offsets `shift`, not every axis warped:
/// the warp's bounded sides refused (below `t = 0`, past `s`'s edges), then the construction's exact test on the
/// pixel's own decode under `S`, its `U` the pixel's (E3). [`super::tilt::Tilted::validate`] refuses every failure.
fn invariant_validate_shifted<S: Selection, P: Copy>(
    inv: &Invariant<P>,
    mapping: [Mapping; 2],
    checks: Checks,
    a: f64,
    b: f64,
    shift: &Shift<f64>,
    p: &LinkParams<f64>,
) -> ValidationResult
where
    f64: From<P>,
{
    let refuse = |failed| ValidationResult::Reject { failed };
    if !(a.is_finite() && b.is_finite()) {
        return refuse(select(b.is_finite(), checks.wide, checks.low));
    }
    if mapping[1] == Mapping::Warped && b < 0.0 {
        return refuse(checks.low);
    }
    if mapping[0] == Mapping::Warped && !(0.0..=1.0).contains(&a) {
        return refuse(checks.wide);
    }
    let z = Latent::new(shift_controls(inv.controls(), shift));
    match invariant_decode::<S, f64, P>(inv, mapping, checks.energy, a, b, &z, p).1 {
        Outcome::Infeasible => refuse(checks.wide),
        Outcome::Feasible | Outcome::Degenerate => ValidationResult::Valid,
    }
}

/// The mapped invariant charts (the module's note): `$energy` whether the vertical axis is `E`, `$low` and `$wide`
/// the constraints the chart's own `validate` names.
macro_rules! mapped_invariant {
    ($($chart:ident, $energy:expr, $low:expr, $wide:expr;)*) => {$(
        impl<P: Copy, S: Selection> Mapped<$chart<P>, S> {
            /// What `validate` names.
            #[inline]
            fn checks(&self) -> Checks {
                Checks {
                    energy: $energy,
                    low: $low,
                    wide: $wide,
                }
            }

            /// The decode at `(s, t)` from the frozen controls `z`, and the construction's outcome.
            #[inline]
            pub fn decode_from<R>(&self, s: R, t: R, z: &Latent<R>, p: &LinkParams<R>) -> (Decoded<R>, Outcome)
            where
                R: LinkReal + PayloadReal + From<P>,
            {
                invariant_decode::<S, R, P>(&self.chart.0, self.mapping, $energy, s, t, z, p)
            }

            /// Each axis's extension type, `[L_z, the vertical]` ([`extension`]).
            pub fn extension(&self) -> [Sides; 2] {
                let vertical = if $energy { Quantity::E } else { Quantity::K };
                [
                    sides(Quantity::Lz, self.mapping[0]),
                    sides(vertical, self.mapping[1]),
                ]
            }
        }

        impl<P: Copy, S: Selection> Chart for Mapped<$chart<P>, S>
        where
            f64: From<P>,
        {
            type Param = P;

            /// The chart's own `Φ`: the frozen controls.
            #[inline]
            fn map<F: Float + From<P>>(&self, u: F, v: F) -> ChartOut<F> {
                self.chart.map(u, v)
            }

            fn forbids_energy_normalisation(&self) -> bool {
                self.chart.forbids_energy_normalisation()
            }

            fn name(&self) -> &str {
                self.chart.name()
            }

            fn requires_per_pixel_mass(&self) -> bool {
                self.chart.requires_per_pixel_mass()
            }

            /// [`Chart::decode_params`] at the defaults.
            #[inline]
            fn decode<S2, R>(&self, u: R, v: R) -> Decoded<R>
            where
                S2: Selection,
                R: LinkReal + PayloadReal + From<P>,
            {
                self.decode_params::<S2, R>(u, v, &LinkParams::DEFAULT)
            }

            /// The chart's own decode under `S` where every axis is warped; else the rule's (the module's note).
            #[inline]
            fn decode_params<S2, R>(&self, u: R, v: R, p: &LinkParams<R>) -> Decoded<R>
            where
                S2: Selection,
                R: LinkReal + PayloadReal + From<P>,
            {
                if self.own() {
                    self.chart.decode_params::<S, R>(u, v, p)
                } else {
                    self.decode_from(u, v, &self.chart.0.frozen(), p).0
                }
            }

            /// [`Chart::validate_params`] at the defaults.
            fn validate(&self, u: f64, v: f64) -> ValidationResult {
                self.validate_params(u, v, &LinkParams::DEFAULT)
            }

            /// The chart's own check where every axis is warped; else `invariant_validate`.
            fn validate_params(&self, u: f64, v: f64, p: &LinkParams<f64>) -> ValidationResult {
                if self.own() {
                    self.chart.validate_params(u, v, p)
                } else {
                    invariant_validate(&self.chart.0, self.mapping, self.checks(), u, v)
                }
            }
        }

        impl<P: Copy, S: Selection> ChartDecode for Mapped<$chart<P>, S>
        where
            f64: From<P>,
        {
            #[inline]
            fn decode_at<S2, R>(&self, u: R, v: R) -> Decoded<R>
            where
                S2: Selection,
                R: LinkReal + PayloadReal + WarpReal + From<P>,
            {
                Chart::decode::<S2, R>(self, u, v)
            }

            #[inline]
            fn decode_at_params<S2, R>(&self, u: R, v: R, p: &LinkParams<R>) -> Decoded<R>
            where
                S2: Selection,
                R: LinkReal + PayloadReal + WarpReal + From<P>,
            {
                Chart::decode_params::<S2, R>(self, u, v, p)
            }
        }

        impl<P: Copy, S: Selection> Hidden for Mapped<$chart<P>, S>
        where
            f64: From<P>,
        {
            /// The chart's own hidden coordinates.
            fn hidden(&self, k: u32) -> bool {
                self.chart.hidden(k)
            }

            /// The chart's own slice, the mapping kept.
            fn sliced(&self, k: u32, value: P) -> Option<Self> {
                self.chart.sliced(k, value).map(|c| Mapped::new(c, self.mapping))
            }
        }

        impl<P: Copy, S: Selection> Tiltable for Mapped<$chart<P>, S>
        where
            f64: From<P>,
        {
            /// The chart's own where every axis is warped; else the rule's on the shifted controls.
            #[inline]
            fn decode_shifted<S2, R>(&self, u: R, v: R, shift: &Shift<R>, p: &LinkParams<R>) -> Decoded<R>
            where
                S2: Selection,
                R: LinkReal + PayloadReal + From<P>,
            {
                if self.own() {
                    self.chart.decode_shifted::<S, R>(u, v, shift, p)
                } else {
                    let z = Latent::new(shift_controls(self.chart.0.controls(), shift));
                    self.decode_from(u, v, &z, p).0
                }
            }

            /// The chart's own where every axis is warped; else `invariant_validate_shifted`.
            fn validate_shifted(&self, a: f64, b: f64, shift: &Shift<f64>, p: &LinkParams<f64>) -> ValidationResult {
                if self.own() {
                    self.chart.validate_shifted(a, b, shift, p)
                } else {
                    invariant_validate_shifted::<S, P>(&self.chart.0, self.mapping, self.checks(), a, b, shift, p)
                }
            }
        }
    )*};
}

mapped_invariant! {
    InvariantLE, true, Constraint::LzEParabola, Constraint::LzEParabola;
    InvariantLK, false, Constraint::KNonNegative, Constraint::KAboveLzBound;
}

/// The Burrau family's `K` charts under the vertical axis's mapping (the module's note): warped, the chart's own;
/// linear, `K = K_max·v`, the construction at `L_z = 0` as on the warp. The horizontal axis, `ν` or `θ`, is not a
/// physical quantity and is bounded (chart_reference §5.4).
macro_rules! mapped_k {
    ($($chart:ident;)*) => {$(
        impl<P: Copy, S> Mapped<$chart<P>, S>
        where
            f64: From<P>,
        {
            /// The linear decode at `(u, v)`: the embed at the chart's `ν(u)`, the construction at `L_z = 0`,
            /// `K* = K_max·v`.
            #[inline]
            pub fn decode_linear<R>(&self, u: R, v: R) -> (Decoded<R>, Outcome)
            where
                R: LinkReal + PayloadReal + From<P>,
            {
                let nu = self.chart.nu(u);
                let k = <R as From<P>>::from(self.chart.k_max) * v;
                let mut blocks = BurrauBlocks::new(embed(nu), burrau::seed(nu), R::zero(), k);
                let decoded = decode::decode_with(&mut blocks, &unread::<R>().z);
                (decoded, blocks.outcome())
            }

            /// Each axis's extension type: the horizontal bounded, the vertical `K`'s under its mapping.
            pub fn extension(&self) -> [Sides; 2] {
                [
                    Sides::both(ExtensionType::Bounded),
                    sides(Quantity::K, self.mapping[1]),
                ]
            }
        }

        impl<P: Copy, S> Chart for Mapped<$chart<P>, S>
        where
            f64: From<P>,
        {
            type Param = P;

            #[inline]
            fn map<F: Float + From<P>>(&self, u: F, v: F) -> ChartOut<F> {
                self.chart.map(u, v)
            }

            fn forbids_energy_normalisation(&self) -> bool {
                self.chart.forbids_energy_normalisation()
            }

            fn name(&self) -> &str {
                self.chart.name()
            }

            fn requires_per_pixel_mass(&self) -> bool {
                self.chart.requires_per_pixel_mass()
            }

            /// The chart's own decode where the vertical axis is warped; else [`Mapped::decode_linear`]. The
            /// selection is not read, as on the chart.
            #[inline]
            fn decode<S2, R>(&self, u: R, v: R) -> Decoded<R>
            where
                S2: Selection,
                R: LinkReal + PayloadReal + From<P>,
            {
                if self.mapping[1] == Mapping::Warped {
                    self.chart.decode::<S2, R>(u, v)
                } else {
                    self.decode_linear(u, v).0
                }
            }

            /// The chart's own check where the vertical axis is warped. Linear: the chart's own check of `u` (at
            /// `v = 0`, the rest start, the same under both mappings), then `v < 0` (`K < 0`) projected to `v = 0`,
            /// and layer 3 on the linear decode.
            fn validate(&self, u: f64, v: f64) -> ValidationResult {
                if self.mapping[1] == Mapping::Warped || !v.is_finite() {
                    return self.chart.validate(u, v);
                }
                match self.chart.validate(u, 0.0) {
                    ValidationResult::Valid => {}
                    other => return other,
                }
                let t = v.max(0.0);
                match burrau::sanity(&self.decode_linear(u, t).0) {
                    Some(failed) => ValidationResult::Reject { failed },
                    None if t != v => ValidationResult::Project {
                        u,
                        v: t,
                        failed: Constraint::KNonNegative,
                    },
                    None => ValidationResult::Valid,
                }
            }
        }

        impl<P: Copy, S> ChartDecode for Mapped<$chart<P>, S>
        where
            f64: From<P>,
        {
            #[inline]
            fn decode_at<S2, R>(&self, u: R, v: R) -> Decoded<R>
            where
                S2: Selection,
                R: LinkReal + PayloadReal + WarpReal + From<P>,
            {
                Chart::decode::<S2, R>(self, u, v)
            }
        }
    )*};
}

mapped_k! {
    Burrau;
    BurrauThetaK;
}
