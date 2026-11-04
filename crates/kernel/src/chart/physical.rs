//! Mixed-axis charts with a physical-quantity axis (chart_reference §1.3, §2.1; R-471 E1 to E3, H1; R-506 W2, W3;
//! REQ-CHART-067, REQ-CHART-070): one axis any latent coordinate or derived quantity of a block the physical axis does
//! not write ([`super::mixed::Quantity`], horizontal), the other a physical quantity (vertical), the rest of `z₀`
//! frozen. Each is a mixed-axis chart, named `Mixed` as [`super::Mixed`] is.
//!
//! **An energy-type axis alone** ([`MixedEnergy`]; R-471 H1). The axis takes its own line of chart_reference §2.1's
//! warp and solves into the momentum block by §2.2's construction, its seed R-82's on the decoded `z_α`, the masses and
//! configuration read as fixed (kind 3: the other axis sits upstream, in the mass or configuration block):
//! - `K` and `KE` ([`AxisK`], [`AxisKE`]): `K* = L_z²/2I + K_max·t^γ_K`, at the chart's held `L_z`.
//! - `E` ([`AxisE`]): `E = U + K*`, that `K*`, `U` the pixel's own potential (E3).
//! - `L_z` ([`AxisLz`]): `L_z = (2t − 1)·L_max`, `L_max = √(2·I·K)` at the chart's held `K`, `I = 1`.
//! - The virial ratio ([`AxisVirial`]; chart_reference §2.1's paragraph) over the asked range `[Q_lo, Q_hi]`
//!   ([`MixedEnergy::virial`]; [`MixedEnergy::new`]'s is `[0, Q_max]`): `Q = Q₀ + (Q_hi − Q₀)·t^γ_K`,
//!   `Q₀ = max(Q_lo, 0)`, `K* = L_z²/2I + Q·|U|/2` on the pixel's own `U`, at the held `L_z`.
//!
//! Every pixel of `[0,1]²` is feasible on a warped `t` axis at every held `L_z` (the owner's ruling of 4 Nov 2026 on
//! RQ-683; chart_reference §2.1): the floor `L_z²/2I` ([`invariant::k_floor`], 0 at the held `L_z = 0`, `Φ_θ,K`'s) sits
//! under the line, so `K*` is at or above it, the bottom row `t = 0` exactly on it. `I = 1` on every pixel, so the floor
//! does not vary across the window. A held `L_z` whose `L_z²` has no finite float at the chart's own precision has no
//! floor, §2.1's one exception: `validate` refuses it there as the decode at that precision labels it. An `L_z` axis is
//! feasible at every held `K ≥ 0`.
//! Under the linear option (`super::energy::Mapping::Linear`) `K` and `KE` are `K_max·t`, `E` is `U₀ + K_max·t` with
//! `U₀` the slice centre's potential, so `K* = E − U` at the pixel, the virial ratio `Q_lo + (Q_hi − Q_lo)·t`, their
//! pixels with `K* < L_z²/2I` (or, virial, `Q < 0`) not a system, and `L_z`'s line is already linear at its held `K`
//! and `I = I₀ = 1`, so its two mappings decode alike and differ only in their extension types.
//!
//! **A mass-ratio axis** ([`MixedMassRatio`]; R-506 W3; chart_reference §2.1's paragraph). The ratio `q = m_b/m_a` of
//! two bodies, the third body's mass `m_c` held at its slice-centre value: `m_a = (1 − m_c)/(1 + q)`, `m_b = q·m_a`.
//! The axis moves the active mass link's inputs, `z_mass = L⁻¹(m)`, and the decode reads the masses through the link,
//! `L(z_mass)`, so every pixel's masses are positive and sum to 1. Warped, `q(t) = q₀·(q₁/q₀)^t` over `[q₀, q₁]`, the
//! part of the asked range the link realises at `m_c` at its active parameters ([`MixedMassRatio::new`]); linear,
//! `q(t) = q_lo + (q_hi − q_lo)·t` over the asked range, a pixel whose ratio the link does not realise, or at or below
//! 0, not a system. A ratio is realised where the link's inverse followed by its forward returns its masses
//! `(m_a, m_b, m_c)`, `m_c` at its held value, to within `ε_μ`, relative: the inverse saturates past what the link
//! reaches. `ε_μ` is the register's link clamp
//! (chart_decoder_contract § "Three hard requirements on any registered link", item 2; inverse_encode_contract Part 3),
//! reused here as the round trip's tolerance, not a new constant.
//!
//! **Validation** (R-26). The other axis's own check (`super::mixed`'s), then the physical axis's: past a bounded side
//! the point is clamped to the edge; then, on the decode under the chart's selection, the construction's exact test
//! (`K* < L_z²/2I` refused) or the ratio's realisation. `validate` takes no selection, so each chart carries its own,
//! `S`, the active links, which its decode reads too (`super::energy::Mapped`'s note).
//!
//! **Tilt** (R-473 J1). The hidden coordinates are the frozen controls the axes do not write: the other axis's
//! coordinate is not one, nor is the physical axis's block (the momentum block for an energy axis, the mass block for
//! the mass ratio). A tilt into a configuration control or a mass logit moves `U` and the masses across the window,
//! and the decode reads them per pixel (E3).

use core::fmt;
use core::marker::PhantomData;

use spirv_std::num_traits::Float;

use super::energy::{centre_potential, decode_rule, sides, Mapping, Quantity, Rule, Sides};
use super::invariant::{self, WarpReal};
use super::mixed::{axis_check, lerp, place, Quantity as Coordinate};
use super::tilt::{shift_controls, Hidden, Shift, Tiltable};
use super::{Chart, ChartDecode, ChartOut, Constraint, ValidationResult};
use crate::decode::invariant::{infeasible, Outcome};
use crate::decode::{self, mass, Decoded, Latent};
use crate::generated::constants::ChartConstants;
use crate::generated::links::{Link, LinkParams, LinkReal, Selection};
use crate::payload::PayloadReal;

/// `t` where `c` holds, else `f`: both operands are computed before the choice, so it is a select, not a branch.
#[inline]
fn select<T: Copy>(c: bool, t: T, f: T) -> T {
    if c {
        t
    } else {
        f
    }
}

/// The eight controls at the float type `F`.
#[inline]
fn wide8<F: Float + From<P>, P: Copy>(z: [P; 8]) -> [F; 8] {
    let w = <F as From<P>>::from;
    [
        w(z[0]),
        w(z[1]),
        w(z[2]),
        w(z[3]),
        w(z[4]),
        w(z[5]),
        w(z[6]),
        w(z[7]),
    ]
}

/// Whether the latent index `k` is in the momentum block, `z[2:6]`.
#[inline]
const fn momentum_index(k: u32) -> bool {
    k >= 2 && k <= 5
}

/// Whether the latent index `k` is in the mass block, `z[6:8]`.
#[inline]
const fn mass_index(k: u32) -> bool {
    k == 6 || k == 7
}

/// The marker types a chart names but holds no value of: `fn() -> T`, so the chart is `Send`, `Sync` and `Copy` whatever
/// they are.
type Markers<T> = PhantomData<fn() -> T>;

// ── An energy-type axis ──────────────────────────────────────────────────────────────────────────────────────────────

/// A lone energy-type axis (the module's note): its quantity and its line of the warp.
pub trait EnergyAxis {
    /// The quantity.
    const QUANTITY: Quantity;

    /// The targets at the axis's value `x` (`K(t)` or `Q(t)` warped, `top·t` linear; `2t − 1` on `L_z`'s line), the
    /// chart's held value `held` and, for a linear `E`, the slice centre's potential `u_ref`.
    fn rule<R: WarpReal>(x: R, held: R, linear: bool, u_ref: R) -> Rule<R>;
}

/// The rule of a `t` axis whose `K*` reads no potential: `K* = x` at the held `L_z`.
#[inline]
fn kinetic_rule<R: WarpReal>(k: R, held_lz: R) -> Rule<R> {
    Rule {
        k0: k,
        weight: R::zero(),
        u_ref: R::zero(),
        s2: R::zero(),
        lz: held_lz,
        lz_on_warp: false,
    }
}

/// The axis `K`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AxisK;

/// The axis `KE`, `K`'s quantity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AxisKE;

/// The axis `E`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AxisE;

/// The axis `L_z`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AxisLz;

/// The axis of the virial ratio `Q = 2K/|U|`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AxisVirial;

impl EnergyAxis for AxisK {
    const QUANTITY: Quantity = Quantity::K;
    #[inline]
    fn rule<R: WarpReal>(x: R, held: R, _linear: bool, _u_ref: R) -> Rule<R> {
        kinetic_rule(x, held)
    }
}

impl EnergyAxis for AxisKE {
    const QUANTITY: Quantity = Quantity::KE;
    #[inline]
    fn rule<R: WarpReal>(x: R, held: R, _linear: bool, _u_ref: R) -> Rule<R> {
        kinetic_rule(x, held)
    }
}

impl EnergyAxis for AxisE {
    const QUANTITY: Quantity = Quantity::E;
    /// Warped, `K* = K(t)`, `E = U + K(t)` at the pixel's own `U`; linear, `E = U₀ + K_max·t` and `K* = E − U`.
    #[inline]
    fn rule<R: WarpReal>(x: R, held: R, linear: bool, u_ref: R) -> Rule<R> {
        Rule {
            weight: select(linear, R::one(), R::zero()),
            u_ref,
            ..kinetic_rule(x, held)
        }
    }
}

impl EnergyAxis for AxisLz {
    const QUANTITY: Quantity = Quantity::Lz;
    /// `L_z = (2t − 1)·L_max(K)` at the held `K`, under both mappings (the module's note); `x` is `2t − 1`.
    #[inline]
    fn rule<R: WarpReal>(x: R, held: R, _linear: bool, _u_ref: R) -> Rule<R> {
        Rule {
            k0: held,
            weight: R::zero(),
            u_ref: R::zero(),
            s2: x,
            lz: R::zero(),
            lz_on_warp: true,
        }
    }
}

impl EnergyAxis for AxisVirial {
    const QUANTITY: Quantity = Quantity::VirialRatio;
    /// `K* = Q·|U|/2`, as `0 + (Q/2)·(0 − U)`, `U < 0`, at the held `L_z`.
    #[inline]
    fn rule<R: WarpReal>(x: R, held: R, _linear: bool, _u_ref: R) -> Rule<R> {
        Rule {
            weight: x / (R::one() + R::one()),
            ..kinetic_rule(R::zero(), held)
        }
    }
}

/// A mixed-axis chart with a lone energy-type axis (the module's note): the horizontal axis `A` on `a_range`, the
/// vertical axis `Q`, the rest of `z₀` frozen, built for the link selection `S`.
pub struct MixedEnergy<A, Q, S, P> {
    /// The slice centre `z₀`, in the decoder's block layout: `A`'s coordinate is the centre's, read only for `U₀`, and the
    /// momentum controls are not read.
    pub frozen: [P; 8],
    /// `[lo, hi]`, the horizontal axis's value at `u = 0` and `u = 1`.
    pub a_range: [P; 2],
    /// The vertical axis's value scale at `t = 1`: `K_max` on `K`, `KE` and `E`, `Q_hi`, the top of the asked range,
    /// on the virial ratio; not read on `L_z`.
    pub top: P,
    /// `Q_lo`, the bottom of the virial ratio's asked range ([`MixedEnergy::virial`]); 0 from [`MixedEnergy::new`], and
    /// read only on the virial ratio.
    pub bottom: P,
    /// `γ_K ≥ 1`, the warp's exponent.
    pub gamma_k: P,
    /// The held value: `L_z` on a `K`, `KE`, `E` or virial axis, `K` on an `L_z` axis.
    pub held: P,
    /// The vertical axis's mapping.
    pub mapping: Mapping,
    axes: Markers<(A, Q, S)>,
}

impl<A, Q, S, P: Copy> Clone for MixedEnergy<A, Q, S, P> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, Q, S, P: Copy> Copy for MixedEnergy<A, Q, S, P> {}

impl<A, Q, S, P: PartialEq> PartialEq for MixedEnergy<A, Q, S, P> {
    fn eq(&self, other: &Self) -> bool {
        self.frozen == other.frozen
            && self.a_range == other.a_range
            && self.top == other.top
            && self.bottom == other.bottom
            && self.gamma_k == other.gamma_k
            && self.held == other.held
            && self.mapping == other.mapping
    }
}

impl<A: Coordinate, Q: EnergyAxis, S, P: fmt::Debug> fmt::Debug for MixedEnergy<A, Q, S, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MixedEnergy")
            .field("a", &A::NAME)
            .field("q", &Q::QUANTITY)
            .field("frozen", &self.frozen)
            .field("a_range", &self.a_range)
            .field("top", &self.top)
            .field("bottom", &self.bottom)
            .field("gamma_k", &self.gamma_k)
            .field("held", &self.held)
            .field("mapping", &self.mapping)
            .finish()
    }
}

impl<A: Coordinate, Q: EnergyAxis, S: Selection, P: Float> MixedEnergy<A, Q, S, P> {
    /// The chart (the module's note); on the virial ratio, over the asked range `[0, top]`.
    #[inline]
    pub fn new(
        frozen: [P; 8],
        a_range: [P; 2],
        top: P,
        gamma_k: P,
        held: P,
        mapping: Mapping,
    ) -> Self {
        let () = Self::UPSTREAM;
        MixedEnergy {
            frozen,
            a_range,
            top,
            bottom: P::zero(),
            gamma_k,
            held,
            mapping,
            axes: PhantomData,
        }
    }
}

impl<A: Coordinate, S: Selection, P: Float> MixedEnergy<A, AxisVirial, S, P> {
    /// The virial-ratio chart over the asked range `asked = [Q_lo, Q_hi]`, a Chart-builder axis's own range (R-471 H4;
    /// chart_reference §2.1's paragraph): warped over its feasible part `[max(Q_lo, 0), Q_hi]`, linear over `asked`.
    /// `None` where `asked` is not an increasing finite range or, warped, its feasible part is empty or one point
    /// (`Q_hi ≤ 0`).
    #[inline]
    pub fn virial(
        frozen: [P; 8],
        a_range: [P; 2],
        asked: [P; 2],
        gamma_k: P,
        held: P,
        mapping: Mapping,
    ) -> Option<Self> {
        let [lo, hi] = asked;
        let ordered = lo.is_finite() && hi.is_finite() && lo < hi;
        let feasible = mapping == Mapping::Linear || hi > P::zero();
        (ordered && feasible).then(|| MixedEnergy {
            bottom: lo,
            ..MixedEnergy::new(frozen, a_range, hi, gamma_k, held, mapping)
        })
    }
}

impl<A: Coordinate, Q: EnergyAxis, S: Selection, P: WarpReal> MixedEnergy<A, Q, S, P> {
    /// The horizontal axis sits upstream of the momentum block the energy axis solves into (kind 3), checked where the
    /// chart is built.
    const UPSTREAM: () = assert!(
        !momentum_index(A::COORDINATE as u32),
        "an energy-type axis solves into the momentum block, which the other axis writes"
    );

    /// The vertical axis's extension type ([`super::energy::extension`]).
    #[inline]
    pub fn extension(&self) -> Sides {
        sides(Q::QUANTITY, self.mapping)
    }

    /// The vertical axis's value scale at `t`: on `K`, `KE` and `E` warped `top·t^γ_K`, linear `top·t`; on the virial
    /// ratio warped `Q₀ + (top − Q₀)·t^γ_K`, `Q₀ = max(bottom, 0)`, linear `bottom + (top − bottom)·t`; on `L_z`,
    /// `2t − 1`.
    #[inline]
    pub fn scale<R: WarpReal + From<P>>(&self, t: R) -> R {
        let w = <R as From<P>>::from;
        let linear = self.mapping == Mapping::Linear;
        let (top, gamma) = (w(self.top), w(self.gamma_k));
        let x = select(linear, top * t, invariant::kinetic(top, gamma, t));
        let q0 = Float::max(w(self.bottom), R::zero());
        let q = select(
            linear,
            lerp([w(self.bottom), top], t),
            q0 + invariant::kinetic(top - q0, gamma, t),
        );
        let x = select(Q::QUANTITY == Quantity::VirialRatio, q, x);
        select(Q::QUANTITY == Quantity::Lz, t + t - R::one(), x)
    }

    /// The decode at `(u, v)` from the frozen controls `z`, the chart's own or a tilt's, and the construction's outcome:
    /// `A`'s control placed, then the rule's targets on the pixel's own masses and configuration.
    #[inline]
    pub fn decode_from<R>(&self, z: [R; 8], u: R, v: R, p: &LinkParams<R>) -> (Decoded<R>, Outcome)
    where
        R: LinkReal + PayloadReal + From<P>,
    {
        let w = <R as From<P>>::from;
        let range = [w(self.a_range[0]), w(self.a_range[1])];
        let z = place(z, A::COORDINATE, A::control_at::<S, R>(lerp(range, u), p));
        let linear = self.mapping == Mapping::Linear;
        let u_ref = if linear & (Q::QUANTITY == Quantity::E) {
            centre_potential::<S, R>(&wide8(self.frozen), p)
        } else {
            R::zero()
        };
        let held = w(self.held);
        let rule = Q::rule(self.scale(v), held, linear, u_ref);
        let floored = !linear & (Q::QUANTITY != Quantity::Lz);
        let rule = Rule {
            k0: select(floored, invariant::k_floor(held) + rule.k0, rule.k0),
            ..rule
        };
        decode_rule::<S, R>(rule, &Latent::new(z), p)
    }

    /// The vertical axis's bounded sides, where its point is clamped: `t < 0` on a warped `t` axis, past either edge
    /// on a warped `L_z` or virial axis; none linear.
    #[inline]
    fn clamp_vertical(&self, v: f64) -> f64 {
        let s = self.extension();
        let low = select(
            s.below == super::energy::ExtensionType::Bounded,
            v.max(0.0),
            v,
        );
        select(
            s.above == super::energy::ExtensionType::Bounded,
            low.min(1.0),
            low,
        )
    }

    /// Whether the chart's own precision `P` holds the floor (the module's note): `check` decodes at f64, and an f32
    /// chart's held `L_z` can have a floor at f64 and none at f32.
    #[inline]
    fn floor_held(&self) -> bool {
        self.mapping == Mapping::Linear
            || Q::QUANTITY == Quantity::Lz
            || !infeasible(P::one(), invariant::k_floor(self.held), self.held)
    }

    /// The check at `(u, v)` on the frozen controls `z` (the module's note).
    fn check(&self, z: [f64; 8], u: f64, v: f64, p: &LinkParams<f64>) -> ValidationResult
    where
        f64: From<P>,
    {
        let vertical = select(
            Q::QUANTITY == Quantity::Lz,
            Constraint::KAboveLzBound,
            Constraint::KNonNegative,
        );
        if !(u.is_finite() && v.is_finite()) {
            return ValidationResult::Reject {
                failed: select(u.is_finite(), vertical, Constraint::AxisDomain),
            };
        }
        let range = [f64::from(self.a_range[0]), f64::from(self.a_range[1])];
        let Ok(cu) = axis_check(A::domain_at(p), range, u) else {
            return ValidationResult::Reject {
                failed: Constraint::AxisDomain,
            };
        };
        let cv = self.clamp_vertical(v);
        let (decoded, outcome) = self.decode_from(z, cu, cv, p);
        let finite = decoded.ic.p.iter().flatten().all(|x| x.is_finite());
        if outcome == Outcome::Infeasible || !finite || !self.floor_held() {
            return ValidationResult::Reject {
                failed: Constraint::KAboveLzBound,
            };
        }
        if cu == u && cv == v {
            ValidationResult::Valid
        } else {
            ValidationResult::Clamp {
                u: cu,
                v: cv,
                moved: [cu != u, cv != v],
                failed: select(cu != u, Constraint::AxisDomain, vertical),
            }
        }
    }
}

impl<A: Coordinate, Q: EnergyAxis, S: Selection, P: WarpReal> Chart for MixedEnergy<A, Q, S, P>
where
    f64: From<P>,
{
    type Param = P;

    /// `z₀` with a latent horizontal axis's coordinate placed; a derived axis's is placed by the decode, through its
    /// link (`super::mixed`'s note).
    #[inline]
    fn map<F: Float + From<P>>(&self, u: F, _v: F) -> ChartOut<F> {
        let z = wide8(self.frozen);
        let w = <F as From<P>>::from;
        let z = if A::DERIVED {
            z
        } else {
            place(
                z,
                A::COORDINATE,
                lerp([w(self.a_range[0]), w(self.a_range[1])], u),
            )
        };
        ChartOut { z: Latent::new(z) }
    }

    /// `true`: the energy is a coordinate, or the construction fixes it (chart_reference §0.6).
    fn forbids_energy_normalisation(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "Mixed"
    }

    /// `true` where the horizontal axis is a mass axis.
    fn requires_per_pixel_mass(&self) -> bool {
        A::COORDINATE.touches().mass
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

    /// [`MixedEnergy::decode_from`] on `z₀` under the chart's selection, which the call's does not replace.
    #[inline]
    fn decode_params<S2, R>(&self, u: R, v: R, p: &LinkParams<R>) -> Decoded<R>
    where
        S2: Selection,
        R: LinkReal + PayloadReal + From<P>,
    {
        self.decode_from(wide8(self.frozen), u, v, p).0
    }

    /// [`Chart::validate_params`] at the defaults.
    fn validate(&self, u: f64, v: f64) -> ValidationResult {
        self.validate_params(u, v, &LinkParams::DEFAULT)
    }

    /// The check on `z₀` (the module's note).
    fn validate_params(&self, u: f64, v: f64, p: &LinkParams<f64>) -> ValidationResult {
        self.check(wide8(self.frozen), u, v, p)
    }
}

impl<A: Coordinate, Q: EnergyAxis, S: Selection, P: WarpReal> ChartDecode
    for MixedEnergy<A, Q, S, P>
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

impl<A: Coordinate, Q: EnergyAxis, S: Selection, P: WarpReal> Hidden for MixedEnergy<A, Q, S, P>
where
    f64: From<P>,
{
    /// The frozen controls neither axis writes: not `A`'s coordinate, nor the momentum block.
    fn hidden(&self, k: u32) -> bool {
        k < 8 && k != A::COORDINATE as u32 && !momentum_index(k)
    }

    /// The frozen control `k` set.
    fn sliced(&self, k: u32, value: P) -> Option<Self> {
        self.hidden(k).then(|| {
            let mut chart = *self;
            chart.frozen[k as usize] = value;
            chart
        })
    }
}

impl<A: Coordinate, Q: EnergyAxis, S: Selection, P: WarpReal> Tiltable for MixedEnergy<A, Q, S, P>
where
    f64: From<P>,
{
    /// [`MixedEnergy::decode_from`] on `z₀` shifted.
    #[inline]
    fn decode_shifted<S2, R>(&self, u: R, v: R, shift: &Shift<R>, p: &LinkParams<R>) -> Decoded<R>
    where
        S2: Selection,
        R: LinkReal + PayloadReal + From<P>,
    {
        self.decode_from(shift_controls(wide8(self.frozen), shift), u, v, p)
            .0
    }

    /// The check on `z₀` shifted: the pixel's own `U`, `I` and masses (E3).
    fn validate_shifted(
        &self,
        a: f64,
        b: f64,
        shift: &Shift<f64>,
        p: &LinkParams<f64>,
    ) -> ValidationResult {
        self.check(shift_controls(wide8(self.frozen), shift), a, b, p)
    }
}

// ── A mass-ratio axis ────────────────────────────────────────────────────────────────────────────────────────────────

/// A mixed-axis chart with a mass-ratio axis (the module's note): the horizontal axis `A` on `a_range`, the vertical
/// the ratio `q = m_b/m_a`, the rest of `z₀` frozen, built for the link selection `S`.
pub struct MixedMassRatio<A, S, P> {
    /// The slice centre `z₀`, in the decoder's block layout; the mass controls give the held `m_c` and the centre's
    /// ratio, and the axes write them.
    pub frozen: [P; 8],
    /// `[lo, hi]`, the horizontal axis's value at `u = 0` and `u = 1`.
    pub a_range: [P; 2],
    /// `[q₀, q₁]`, the ratio at `t = 0` and `t = 1`: warped, the part of the asked range the link realises; linear,
    /// the asked range.
    pub ratio: [P; 2],
    /// The bodies `(a, b)` of `q = m_b/m_a`.
    pub bodies: [u32; 2],
    /// The third body's mass `m_c`, held at its slice-centre value.
    pub held: P,
    /// The axis's mapping.
    pub mapping: Mapping,
    axes: Markers<(A, S)>,
}

impl<A, S, P: Copy> Clone for MixedMassRatio<A, S, P> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, S, P: Copy> Copy for MixedMassRatio<A, S, P> {}

impl<A, S, P: PartialEq> PartialEq for MixedMassRatio<A, S, P> {
    fn eq(&self, other: &Self) -> bool {
        self.frozen == other.frozen
            && self.a_range == other.a_range
            && self.ratio == other.ratio
            && self.bodies == other.bodies
            && self.held == other.held
            && self.mapping == other.mapping
    }
}

impl<A: Coordinate, S, P: fmt::Debug> fmt::Debug for MixedMassRatio<A, S, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MixedMassRatio")
            .field("a", &A::NAME)
            .field("frozen", &self.frozen)
            .field("a_range", &self.a_range)
            .field("ratio", &self.ratio)
            .field("bodies", &self.bodies)
            .field("held", &self.held)
            .field("mapping", &self.mapping)
            .finish()
    }
}

/// The masses at the ratio `q = m_b/m_a`, `m_c` held, bodies `(a, b)`, each component read by a literal index.
#[inline]
pub fn ratio_masses<R: Float>(bodies: [u32; 2], held: R, q: R) -> [R; 3] {
    let m_a = (R::one() - held) / (R::one() + q);
    let m_b = q * m_a;
    let at = |i: u32| select(i == bodies[0], m_a, select(i == bodies[1], m_b, held));
    [at(0), at(1), at(2)]
}

/// `m_b/m_a` of the masses `m`, bodies `(a, b)`, each component read by a literal index.
#[inline]
pub fn ratio_of<R: Float>(bodies: [u32; 2], m: [R; 3]) -> R {
    let pick = |k: u32| select(k == 0, m[0], select(k == 1, m[1], m[2]));
    pick(bodies[1]) / pick(bodies[0])
}

impl<A: Coordinate, S: Selection, P: Copy> MixedMassRatio<A, S, P> {
    /// The horizontal axis writes no mass control, which the mass-ratio axis writes, checked where the chart is built.
    const APART: () = assert!(
        !mass_index(A::COORDINATE as u32),
        "a mass-ratio axis writes the mass block, which the other axis writes"
    );

    /// The vertical axis's extension type ([`super::energy::extension`]).
    #[inline]
    pub fn extension(&self) -> Sides {
        sides(Quantity::MassRatio, self.mapping)
    }

    /// The ratio at `t`: warped `q₀·(q₁/q₀)^t`, linear `q₀ + (q₁ − q₀)·t`.
    #[inline]
    pub fn ratio_at<R: Float + From<P>>(&self, t: R) -> R {
        let w = <R as From<P>>::from;
        let (q0, q1) = (w(self.ratio[0]), w(self.ratio[1]));
        let geometric = q0 * Float::powf(q1 / q0, t);
        select(
            self.mapping == Mapping::Linear,
            lerp([q0, q1], t),
            geometric,
        )
    }

    /// The chart with its parameters at f64, for the CPU's checks.
    #[inline]
    pub fn at_f64(&self) -> MixedMassRatio<A, S, f64>
    where
        f64: From<P>,
    {
        self.with_params(<f64 as From<P>>::from)
    }

    /// The chart with each parameter `x` replaced by `w(x)`: the chart [`MixedMassRatio::new`] builds at f64, narrowed
    /// for a decode at another precision.
    #[inline]
    pub fn with_params<P2, W: Fn(P) -> P2>(&self, w: W) -> MixedMassRatio<A, S, P2> {
        let f = self.frozen;
        MixedMassRatio {
            frozen: [
                w(f[0]),
                w(f[1]),
                w(f[2]),
                w(f[3]),
                w(f[4]),
                w(f[5]),
                w(f[6]),
                w(f[7]),
            ],
            a_range: [w(self.a_range[0]), w(self.a_range[1])],
            ratio: [w(self.ratio[0]), w(self.ratio[1])],
            bodies: self.bodies,
            held: w(self.held),
            mapping: self.mapping,
            axes: PhantomData,
        }
    }

    /// The controls at `(u, v)` from the frozen controls `z`: `A`'s placed, and the mass controls the link's inverse
    /// at the ratio's masses.
    #[inline]
    pub fn controls_from<R>(&self, z: [R; 8], u: R, v: R, p: &LinkParams<R>) -> [R; 8]
    where
        R: LinkReal + From<P>,
    {
        let w = <R as From<P>>::from;
        let range = [w(self.a_range[0]), w(self.a_range[1])];
        let z = place(z, A::COORDINATE, A::control_at::<S, R>(lerp(range, u), p));
        let m = ratio_masses(self.bodies, w(self.held), self.ratio_at(v));
        let [z6, z7] = <S::MassLink as Link<2, 3>>::inverse_at(m, p);
        [z[0], z[1], z[2], z[3], z[4], z[5], z6, z7]
    }
}

impl<A: Coordinate, S: Selection> MixedMassRatio<A, S, f64> {
    /// The chart over the asked range `asked` of `q = m_b/m_a`, `bodies = (a, b)`, under `mapping` at the link's tunable
    /// parameters `p` (the module's note): `m_c` the third body's mass at `z₀`'s mass controls; warped, the range the
    /// part of `asked` the link realises at `m_c`, found from the slice centre's own ratio outward. `None` where the
    /// bodies are not two distinct bodies, `asked` is not an increasing finite range, or, warped, the link realises no
    /// part of it.
    pub fn new(
        frozen: [f64; 8],
        a_range: [f64; 2],
        asked: [f64; 2],
        bodies: [u32; 2],
        mapping: Mapping,
        p: &LinkParams<f64>,
    ) -> Option<Self> {
        let () = Self::APART;
        let [a, b] = bodies;
        let ordered = asked[0].is_finite() && asked[1].is_finite() && asked[0] < asked[1];
        if !(a < 3 && b < 3 && a != b && ordered) {
            return None;
        }
        let m = mass::decode_at::<S, f64>([frozen[6], frozen[7]], p).m;
        let held = m[(3 - a - b) as usize];
        let chart = MixedMassRatio {
            frozen,
            a_range,
            ratio: asked,
            bodies,
            held,
            mapping,
            axes: PhantomData,
        };
        if mapping == Mapping::Linear {
            return Some(chart);
        }
        let centre = ratio_of(bodies, m);
        if !chart.realised_within(centre, 0.5 * <f64 as ChartConstants>::EPS_MU, p) {
            return None;
        }
        let lo = chart.edge(centre, asked[0], false, p)?;
        let hi = chart.edge(centre, asked[1], true, p)?;
        (lo < hi).then_some(MixedMassRatio {
            ratio: [lo, hi],
            ..chart
        })
    }

    /// Whether the link realises the ratio `q` at the held `m_c`: `q` positive and finite, and the link's inverse then
    /// forward returning the masses [`ratio_masses`] writes, positive, summing to 1, with ratio `q` and the third body's
    /// mass at the held `m_c`, each to within `ε_μ`, relative. The last test is not implied by the others: where the
    /// logit that sets `m_c` saturates before the ratio's does, the round trip keeps the ratio and the sum and moves
    /// `m_c`, a system off the slice.
    pub fn realised(&self, q: f64, p: &LinkParams<f64>) -> bool {
        self.realised_within(q, <f64 as ChartConstants>::EPS_MU, p)
    }

    /// [`MixedMassRatio::realised`] to within `eps`, relative.
    fn realised_within(&self, q: f64, eps: f64, p: &LinkParams<f64>) -> bool {
        if !(q.is_finite() && q > 0.0) {
            return false;
        }
        let m = ratio_masses(self.bodies, self.held, q);
        let link = <S::MassLink as Link<2, 3>>::inverse_at(m, p);
        let back = <S::MassLink as Link<2, 3>>::forward_at(link, p);
        let positive = back.iter().all(|&x| x > 0.0);
        let sum = back[0] + back[1] + back[2];
        let [a, b] = self.bodies;
        let m_c = back[(3 - a - b) as usize];
        positive
            && (sum - 1.0).abs() <= eps
            && (ratio_of(self.bodies, back) - q).abs() <= eps * q
            && (m_c - self.held).abs() <= eps * self.held
    }

    /// The end of the realised range from the realised ratio `from` toward the asked end `toward`, the range's upper
    /// end where `upper`: `toward` itself where realised; `None` where it is not and lies on `from`'s inner side (an
    /// upper end below `from`, a lower end above it), the asked range then outside the interval the link realises;
    /// else the last realised ratio on the way, by bisection in `ln q`, sixty-four halvings, an end at or below 0 taken
    /// at the least positive double. Each test here is to within `ε_μ/2`, so that the warp's rounding at the range's
    /// ends (`q₀·(q₁/q₀)^t` at `t = 1`) stays inside [`MixedMassRatio::realised`]'s `ε_μ`.
    fn edge(&self, from: f64, toward: f64, upper: bool, p: &LinkParams<f64>) -> Option<f64> {
        let eps = 0.5 * <f64 as ChartConstants>::EPS_MU;
        if self.realised_within(toward, eps, p) {
            return Some(toward);
        }
        if upper != (toward > from) {
            return None;
        }
        let (mut ok, mut bad) = (from.ln(), toward.max(f64::MIN_POSITIVE).ln());
        let mut last = from;
        for _ in 0..64 {
            let mid = 0.5 * (ok + bad);
            let q = mid.exp();
            if self.realised_within(q, eps, p) {
                ok = mid;
                last = q;
            } else {
                bad = mid;
            }
        }
        Some(last)
    }

    /// The check at `(u, v)` (the module's note).
    fn check(&self, u: f64, v: f64, p: &LinkParams<f64>) -> ValidationResult {
        let failed = Constraint::AxisDomain;
        if !(u.is_finite() && v.is_finite()) {
            return ValidationResult::Reject { failed };
        }
        let Ok(cu) = axis_check(A::domain_at(p), self.a_range, u) else {
            return ValidationResult::Reject { failed };
        };
        let cv = select(self.mapping == Mapping::Warped, v.clamp(0.0, 1.0), v);
        if !self.realised(self.ratio_at(cv), p) {
            return ValidationResult::Reject { failed };
        }
        if cu == u && cv == v {
            ValidationResult::Valid
        } else {
            ValidationResult::Clamp {
                u: cu,
                v: cv,
                moved: [cu != u, cv != v],
                failed,
            }
        }
    }
}

impl<A: Coordinate, S: Selection, P: Copy> Chart for MixedMassRatio<A, S, P>
where
    f64: From<P>,
{
    type Param = P;

    /// `z₀` with a latent horizontal axis's coordinate placed; the mass controls are placed by the decode, through the
    /// link's inverse.
    #[inline]
    fn map<F: Float + From<P>>(&self, u: F, _v: F) -> ChartOut<F> {
        let z = wide8(self.frozen);
        let w = <F as From<P>>::from;
        let z = if A::DERIVED {
            z
        } else {
            place(
                z,
                A::COORDINATE,
                lerp([w(self.a_range[0]), w(self.a_range[1])], u),
            )
        };
        ChartOut { z: Latent::new(z) }
    }

    fn name(&self) -> &str {
        "Mixed"
    }

    /// `true`: the masses follow the ratio.
    fn requires_per_pixel_mass(&self) -> bool {
        true
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

    /// The shared decoder on [`MixedMassRatio::controls_from`]'s controls under the chart's selection, which the call's
    /// does not replace.
    #[inline]
    fn decode_params<S2, R>(&self, u: R, v: R, p: &LinkParams<R>) -> Decoded<R>
    where
        S2: Selection,
        R: LinkReal + PayloadReal + From<P>,
    {
        let z = self.controls_from(wide8(self.frozen), u, v, p);
        decode::decode_at::<S, R>(&Latent::new(z), p)
    }

    /// [`Chart::validate_params`] at the defaults.
    fn validate(&self, u: f64, v: f64) -> ValidationResult {
        self.validate_params(u, v, &LinkParams::DEFAULT)
    }

    /// The check (the module's note).
    fn validate_params(&self, u: f64, v: f64, p: &LinkParams<f64>) -> ValidationResult {
        self.at_f64().check(u, v, p)
    }
}

impl<A: Coordinate, S: Selection, P: Copy> ChartDecode for MixedMassRatio<A, S, P>
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

impl<A: Coordinate, S: Selection, P: Copy> Hidden for MixedMassRatio<A, S, P>
where
    f64: From<P>,
{
    /// The frozen controls neither axis writes: not `A`'s coordinate, nor the mass block.
    fn hidden(&self, k: u32) -> bool {
        k < 8 && k != A::COORDINATE as u32 && !mass_index(k)
    }

    /// The frozen control `k` set; the held `m_c` and the range, the mass block's, are unchanged.
    fn sliced(&self, k: u32, value: P) -> Option<Self> {
        self.hidden(k).then(|| {
            let mut chart = *self;
            chart.frozen[k as usize] = value;
            chart
        })
    }
}

impl<A: Coordinate, S: Selection, P: Copy> Tiltable for MixedMassRatio<A, S, P>
where
    f64: From<P>,
{
    /// The shared decoder on [`MixedMassRatio::controls_from`]'s controls from `z₀` shifted; the shift moves no mass
    /// control, so the check is the untilted one's at `(a, b)`.
    #[inline]
    fn decode_shifted<S2, R>(&self, u: R, v: R, shift: &Shift<R>, p: &LinkParams<R>) -> Decoded<R>
    where
        S2: Selection,
        R: LinkReal + PayloadReal + From<P>,
    {
        let z = self.controls_from(shift_controls(wide8(self.frozen), shift), u, v, p);
        decode::decode_at::<S, R>(&Latent::new(z), p)
    }
}
