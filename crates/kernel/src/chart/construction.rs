//! The six charts that hold a block by a physical construction, with that construction's physical parameters as their
//! hidden coordinates, tilted and sliced by value, a held `K` in warped mode tilted in its warp coordinate instead
//! (R-633, ruling RQ-603, amending R-473 J1; R-708, R-709, R-720; chart_decoder_contract § "Tilt (basis edit)", "Tilt on
//! every chart"; chart_reference §4.5, "Hidden coordinates under tilt and slice").
//!
//! **Which coordinates** (chart_reference §4.5's table). A hidden coordinate is named by an index, as on the latent
//! charts ([`super::tilt`]'s note): a momentum control by its latent index `2..6`, and a physical value by an index of
//! its own past the controls, which [`super::tilt::shift_controls`] leaves untouched: `ν₀` ([`NU0`]), the held `L_z`
//! ([`HELD_LZ`]) and the held `K` ([`HELD_K`]).
//! - **A rest start** ([`RestStart`]: the mass simplex, the Euclid plane, the δm strip) is `z_mom = 0`, and each of the
//!   four momentum controls is a hidden coordinate from 0: the momenta are `D_mom` of the controls under the pixel's
//!   masses ([`FromRest`]), the selection's links at the active `q_max`. Every registered momentum link maps 0 to 0, so
//!   the rest start is `D_mom(0)`; its one `−0`, `p₀ = −p_ρ − (m₀/M₀₁)·p_λ` at 0, is added to the rest start's `+0`,
//!   which leaves every other value as it is, so the controls at 0 give the untilted chart bit for bit.
//! - **The mass simplex** adds `ν₀`, the held Burrau-family shape, its own parameter: the triangle is the Burrau one at
//!   `ν₀ + d`, the masses the chart map's at `(a, b)`; at `d = 0` the untilted chart exactly. A cell whose `ν₀ + d`
//!   leaves `(0, 1)` has a zero or negative side, so it is refused ([`Constraint::BurrauNu`]).
//! - **The `(ν, K)` chart and `Φ_θ,K`, the strip `Φ_strip,K` the same map** ([`HeldLz`]) hold `L_z`, 0 untilted;
//!   **`Φ_θ,L_z`** holds `K`, its own parameter `k` ([`HeldK`]). The momenta are §2.2's construction at the tilted value,
//!   the chart's axes through its map at the slice centre exactly (R-473 T2), each chart by its energy axis's mapping
//!   ([`Mapping`], R-471's two modes; R-708), below.
//!
//! **Warped mode, the default** (R-708, R-720). Every pixel inside the chart's square is a system under every tilt.
//! - The held `L_z` moves by value in both modes, `L_z + d`, tilted and sliced alike, with no sign constraint. The `K`
//!   axis takes the pixel's own held `L_z` as its floor, `K = L_z²/2I + K_max·t^γ_K` ([`k_floor`], which holds the bottom
//!   row feasible through the rounding), past `K_max` at the top if the floor takes it there.
//! - The held `K` tilts in its warp coordinate, not by value: `K = K_max·t^γ_K`, `t = t₀ + d`,
//!   `t₀ = (K₀/K_max)^(1/γ_K)`, `d` in units of `t`, the band `L_z(b)` at the pixel's own `K`, so `L_z²/2I ≤ K` at every
//!   `K ≥ 0` and it takes no floor. At `d = 0` the held `K₀` is read itself, not through `t₀`, so the chart is its
//!   untilted self bit for bit. `K_max` and `γ_K` are [`HeldK`]'s inputs: `Φ_θ,L_z` has no `K` axis of its own to read
//!   them from.
//! - **`t₀ + d < 0` arises** wherever a tilt's `d` passes `−t₀`, and at a non-integer `γ_K` the warp has no real value
//!   there, so the coordinate clamps at `t = 0` (R-709): `K = 0`, `L_z = 0`, the rest start at the pixel's own shape, a
//!   real system. No mirror. The pixel is flagged ([`Tiltable::held_at_floor`]).
//!
//! **Linear mode, the option per axis** (R-471 E2). Rows keep fixed physical values, `K(b) = K_max·b` and `L_z + d` on
//! the first charts, and `L_z(b)`, the band of the centre's `K`, and `K + d` on `Φ_θ,L_z`, whose tilt is that of the bare
//! chart. A slice moves the centre, so there `L_z(b)`'s band follows the sliced `K`.
//!
//! **The refused cell** (R-633, read per cell by R-636; chart_reference §2.2's terminal case), the linear arm's. Where
//! the tilted value leaves no momentum construction, `K < L_z²/2I`, decided by the construction's exact test
//! ([`infeasible`], `I = 1`), the cell is no system: its decode carries `DEGENERATE(INVARIANT_INFEASIBLE)`, the tilted
//! view's `validate` refuses it ([`Constraint::KAboveLzBound`]) and its area weight is 0
//! ([`super::tilt::Tilted::weight`]); the chart's `system_image` stays its own. The warped path runs the same test, which
//! no pixel inside the square fails: past it `|2b − 1|·λ_L` can pass 1, and a `L_z` with no finite floor is refused
//! ([`k_floor`]).
//!
//! **The held values.** The registered charts' own parameters stay as they are; a rest start's momentum controls, the
//! held `L_z`, a chart's mode and `Φ_θ,L_z`'s warp are held beside the chart by the wrappers [`RestStart`], [`HeldLz`] and
//! [`HeldK`], which are the registered chart with its hidden coordinates, not charts of their own (their name, and every
//! entry at the held values' rest, is the chart's).

use spirv_std::num_traits::Float;

use super::burrau::{self, embed, select, Burrau, EuclidPlane, RestBlocks};
use super::burrau_acute::{self, BurrauThetaDeltaM, BurrauThetaK, BurrauThetaLz, DeltaMBlocks};
use super::energy::Mapping;
use super::invariant::{self, k_floor, WarpReal};
use super::mass_simplex::{self, MassSimplex};
use super::tilt::{shift_controls, shifted, Hidden, Shift, Tiltable};
use super::{Chart, ChartDecode, ChartOut, Constraint, ValidationResult};
use crate::decode::config::Config;
use crate::decode::invariant::infeasible;
use crate::decode::mass::Masses;
use crate::decode::momentum::{self, Momentum};
use crate::decode::{self, Blocks, Decoded, Ic, Latent};
use crate::generated::links::{LinkParams, LinkReal, Selection};
use crate::payload::{ICDescriptorOf, PayloadReal};

use core::marker::PhantomData;

/// `ν₀`'s index, the mass simplex's held shape: the register's `hidden_nu0`.
pub const NU0: u32 = crate::generated::constants::HIDDEN_NU0;
/// The held `L_z`'s index, on the `(ν, K)` chart and `Φ_θ,K`: the register's `hidden_held_lz`.
pub const HELD_LZ: u32 = crate::generated::constants::HIDDEN_HELD_LZ;
/// The held `K`'s index, on `Φ_θ,L_z`: the register's `hidden_held_k`.
pub const HELD_K: u32 = crate::generated::constants::HIDDEN_HELD_K;

/// Whether `k` names a momentum control, `2..6`.
#[inline]
pub fn momentum_control(k: u32) -> bool {
    (2..6).contains(&k)
}

/// `rest`, the chart's check at `(u, v)`, then `refusal` at the point it accepts, or moves to: a point the check would
/// commit to and the tilted value refuses is refused.
fn refuse(
    u: f64,
    v: f64,
    rest: ValidationResult,
    refusal: impl Fn(f64, f64) -> Option<Constraint>,
) -> ValidationResult {
    let (a, b) = match rest {
        ValidationResult::Valid => (u, v),
        ValidationResult::Project { u, v, .. } | ValidationResult::Clamp { u, v, .. } => (u, v),
        ValidationResult::Reject { .. } => return rest,
    };
    match refusal(a, b) {
        Some(failed) => ValidationResult::Reject { failed },
        None => rest,
    }
}

// ── A rest start: the momentum controls from 0 ────────────────────────────────────────────────────────────────────

/// The rest start's momenta plus `D_mom`'s (the module's note): each value `+ 0`, which turns only a `−0` into `+0`.
#[inline]
fn plus_rest<R: Float>(m: Momentum<R>) -> Momentum<R> {
    let z = R::zero();
    let add = |v: [R; 2]| [v[0] + z, v[1] + z];
    Momentum {
        q: [m.q[0] + z, m.q[1] + z, m.q[2] + z, m.q[3] + z],
        p_rho: add(m.p_rho),
        p_lambda: add(m.p_lambda),
        p: [add(m.p[0]), add(m.p[1]), add(m.p[2])],
    }
}

/// A rest-start chart's decoder with its momentum controls through `D_mom`: the masses, the configuration and the
/// invariants `rest`'s, the momenta `D_mom` of the controls `decode_with` hands it, under the selection `S` at the
/// tunable parameters `params`. Neither raises a tag of its own: a rest start's blocks tag nothing past the mass
/// block's `M01_TINY`, which the masses carry, and `D_mom` is total.
pub struct FromRest<B, S, R> {
    /// The chart's own blocks, at rest.
    rest: B,
    /// The tunable parameters the momentum links read.
    params: LinkParams<R>,
    links: PhantomData<S>,
}

impl<B, S, R> FromRest<B, S, R> {
    /// `rest` with its momenta from the controls at `params`.
    #[inline]
    pub fn new(rest: B, params: LinkParams<R>) -> Self {
        FromRest {
            rest,
            params,
            links: PhantomData,
        }
    }
}

impl<B: Blocks<R>, S: Selection, R: LinkReal + PayloadReal> Blocks<R> for FromRest<B, S, R> {
    #[inline]
    fn mass(&mut self, z_mass: [R; 2]) -> Masses<R> {
        self.rest.mass(z_mass)
    }

    #[inline]
    fn config(&mut self, z_cfg: [R; 2], masses: &Masses<R>) -> Config<R> {
        self.rest.config(z_cfg, masses)
    }

    /// `D_mom` of the controls under the masses, added to the rest start's momenta.
    #[inline]
    fn momentum(&mut self, z_mom: [R; 4], masses: &Masses<R>, _config: &Config<R>) -> Momentum<R> {
        plus_rest(momentum::decode_at::<S, R>(z_mom, masses, &self.params))
    }

    #[inline]
    fn invariants(&mut self, ic: &Ic<R>) -> ICDescriptorOf<R> {
        self.rest.invariants(ic)
    }
}

/// `blocks` decoded with the momentum controls `z_mom` from the rest start.
#[inline]
fn from_rest<S, R, B>(blocks: B, z_mom: [R; 4], p: &LinkParams<R>) -> Decoded<R>
where
    S: Selection,
    R: LinkReal + PayloadReal,
    B: Blocks<R>,
{
    let zero = R::zero();
    let z = Latent::new([
        zero, zero, z_mom[0], z_mom[1], z_mom[2], z_mom[3], zero, zero,
    ]);
    decode::decode_with(&mut FromRest::<B, S, R>::new(blocks, *p), &z)
}

/// A chart that starts at rest: its masses and configuration its own, its momenta from the momentum controls.
pub trait RestChart: Chart + Copy {
    /// Whether `k` names one of the chart's own physical hidden coordinates: `ν₀` on the mass simplex.
    fn physical(&self, _k: u32) -> bool {
        false
    }

    /// The chart with its physical hidden coordinate `k` set to `value`; `None` where it has none.
    fn with_physical(&self, _k: u32, _value: Self::Param) -> Option<Self> {
        None
    }

    /// The constraint the cell at `(a, b)` fails with `shift` on its physical hidden coordinates; none by default.
    fn refusal(&self, _a: f64, _b: f64, _shift: &Shift<f64>) -> Option<Constraint> {
        None
    }

    /// `D ∘ Φ` at `(u, v)` with the momentum controls `z_mom` from the rest start and `shift` on the chart's physical
    /// hidden coordinates, under `S` at `p`.
    fn decode_moving<S, R>(
        &self,
        u: R,
        v: R,
        z_mom: [R; 4],
        shift: &Shift<R>,
        p: &LinkParams<R>,
    ) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<Self::Param>;
}

impl<P: Copy> RestChart for MassSimplex<P>
where
    f64: From<P>,
{
    fn physical(&self, k: u32) -> bool {
        k == NU0
    }

    fn with_physical(&self, k: u32, value: P) -> Option<Self> {
        (k == NU0).then_some(MassSimplex {
            nu0: value,
            ..*self
        })
    }

    /// `ν₀ + d` outside `(0, 1)`: a side zero or negative.
    fn refusal(&self, _a: f64, _b: f64, shift: &Shift<f64>) -> Option<Constraint> {
        let nu = shifted(f64::from(self.nu0), NU0, shift);
        (!(nu > 0.0 && nu < 1.0)).then_some(Constraint::BurrauNu)
    }

    /// The triangle at `ν₀ + d`, the masses the map's at `(u, v)`.
    #[inline]
    fn decode_moving<S, R>(
        &self,
        u: R,
        v: R,
        z_mom: [R; 4],
        shift: &Shift<R>,
        p: &LinkParams<R>,
    ) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<P>,
    {
        let nu = shifted(<R as From<P>>::from(self.nu0), NU0, shift);
        let system = mass_simplex::system(nu, self.masses(u, v));
        from_rest::<S, R, _>(RestBlocks::new(system), z_mom, p)
    }
}

impl<P: Copy> RestChart for EuclidPlane<P>
where
    f64: From<P>,
{
    /// The Burrau system at `ν(v)`.
    #[inline]
    fn decode_moving<S, R>(
        &self,
        _u: R,
        v: R,
        z_mom: [R; 4],
        _shift: &Shift<R>,
        p: &LinkParams<R>,
    ) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<P>,
    {
        from_rest::<S, R, _>(RestBlocks::new(embed(self.nu(v))), z_mom, p)
    }
}

impl<P: Copy> RestChart for BurrauThetaDeltaM<P>
where
    f64: From<P>,
{
    /// The triangle at `ν(θ(u))` with the blended masses.
    #[inline]
    fn decode_moving<S, R>(
        &self,
        u: R,
        v: R,
        z_mom: [R; 4],
        _shift: &Shift<R>,
        p: &LinkParams<R>,
    ) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<P>,
    {
        let system = embed(self.nu(u));
        let blocks = DeltaMBlocks::new(system, self.blend(system.m, v));
        from_rest::<S, R, _>(blocks, z_mom, p)
    }
}

/// A rest-start chart with its four momentum controls held at the slice centre, `0` at rest (the module's note).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RestStart<C, P> {
    /// The registered chart.
    pub chart: C,
    /// The momentum controls `z[2:6]` at the slice centre.
    pub z_mom: [P; 4],
}

impl<C, P: Float> RestStart<C, P> {
    /// `chart` at rest, `z_mom = 0`.
    #[inline]
    pub fn new(chart: C) -> Self {
        RestStart {
            chart,
            z_mom: [P::zero(); 4],
        }
    }
}

impl<C: RestChart<Param = P>, P: Copy> RestStart<C, P> {
    /// The held momentum controls, widened.
    #[inline]
    fn controls<R: Float + From<P>>(&self) -> [R; 4] {
        let w = <R as From<P>>::from;
        [
            w(self.z_mom[0]),
            w(self.z_mom[1]),
            w(self.z_mom[2]),
            w(self.z_mom[3]),
        ]
    }
}

impl<C: RestChart<Param = P>, P: Copy> Chart for RestStart<C, P> {
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

    fn validate(&self, u: f64, v: f64) -> ValidationResult {
        self.chart.validate(u, v)
    }

    fn validate_params(&self, u: f64, v: f64, p: &LinkParams<f64>) -> ValidationResult {
        self.chart.validate_params(u, v, p)
    }

    #[inline]
    fn decode<S, R>(&self, u: R, v: R) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<P>,
    {
        self.decode_params::<S, R>(u, v, &LinkParams::DEFAULT)
    }

    /// The chart's decode with the held momentum controls through `D_mom`.
    #[inline]
    fn decode_params<S, R>(&self, u: R, v: R, p: &LinkParams<R>) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<P>,
    {
        self.chart
            .decode_moving::<S, R>(u, v, self.controls(), &Shift::none(), p)
    }
}

impl<C: RestChart<Param = P>, P: Copy> ChartDecode for RestStart<C, P> {
    #[inline]
    fn decode_at<S, R>(&self, u: R, v: R) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + WarpReal + From<P>,
    {
        Chart::decode::<S, R>(self, u, v)
    }

    #[inline]
    fn decode_at_params<S, R>(&self, u: R, v: R, p: &LinkParams<R>) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + WarpReal + From<P>,
    {
        Chart::decode_params::<S, R>(self, u, v, p)
    }
}

impl<C: RestChart<Param = P>, P: Copy> Hidden for RestStart<C, P> {
    /// The four momentum controls, and the chart's own physical ones.
    fn hidden(&self, k: u32) -> bool {
        momentum_control(k) || self.chart.physical(k)
    }

    fn sliced(&self, k: u32, value: P) -> Option<Self> {
        if momentum_control(k) {
            let mut z_mom = self.z_mom;
            z_mom[(k - 2) as usize] = value;
            Some(RestStart { z_mom, ..*self })
        } else {
            let chart = self.chart.with_physical(k, value)?;
            Some(RestStart { chart, ..*self })
        }
    }
}

impl<C: RestChart<Param = P>, P: Copy> Tiltable for RestStart<C, P> {
    /// The held momentum controls with `shift` added, through `D_mom`, and `shift` on the physical ones.
    #[inline]
    fn decode_shifted<S, R>(&self, u: R, v: R, shift: &Shift<R>, p: &LinkParams<R>) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<P>,
    {
        let [m0, m1, m2, m3] = self.controls::<R>();
        let zero = R::zero();
        let z = shift_controls([zero, zero, m0, m1, m2, m3, zero, zero], shift);
        self.chart
            .decode_moving::<S, R>(u, v, [z[2], z[3], z[4], z[5]], shift, p)
    }

    fn validate_shifted(
        &self,
        a: f64,
        b: f64,
        shift: &Shift<f64>,
        p: &LinkParams<f64>,
    ) -> ValidationResult {
        refuse(a, b, self.chart.validate_params(a, b, p), |a, b| {
            self.chart.refusal(a, b, shift)
        })
    }

    fn refusal(&self, a: f64, b: f64, shift: &Shift<f64>) -> Option<Constraint> {
        self.chart.refusal(a, b, shift)
    }
}

impl<C, P: Copy> RestStart<C, P> {
    /// The value of the hidden coordinate `k` at the slice centre: a momentum control's own, or `ν₀`; `None` where
    /// `k` is neither.
    pub fn held(&self, k: u32) -> Option<P>
    where
        C: Held<P>,
    {
        if momentum_control(k) {
            Some(self.z_mom[(k - 2) as usize])
        } else {
            self.chart.held(k)
        }
    }
}

/// A chart's own physical hidden coordinate at the slice centre: `ν₀` on the mass simplex, the held `K` on `Φ_θ,L_z`.
pub trait Held<P> {
    /// The value of `k` at the slice centre, `None` where `k` names none of the chart's own.
    fn held(&self, k: u32) -> Option<P>;
}

impl<P: Copy> Held<P> for MassSimplex<P> {
    fn held(&self, k: u32) -> Option<P> {
        (k == NU0).then_some(self.nu0)
    }
}

impl<P: Copy> Held<P> for EuclidPlane<P> {
    fn held(&self, _k: u32) -> Option<P> {
        None
    }
}

impl<P: Copy> Held<P> for BurrauThetaDeltaM<P> {
    fn held(&self, _k: u32) -> Option<P> {
        None
    }
}

impl<P: Copy> Held<P> for BurrauThetaLz<P> {
    fn held(&self, k: u32) -> Option<P> {
        (k == HELD_K).then_some(self.k)
    }
}

// ── The held L_z: the (ν, K) chart and Φ_θ,K ──────────────────────────────────────────────────────────────────────

/// The `(ν, K)` chart or `Φ_θ,K` with its `L_z` held at the slice centre, `0` untilted, and its `K` axis's mapping (the
/// module's note).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeldLz<C, P> {
    /// The registered chart.
    pub chart: C,
    /// The held `L_z`.
    pub lz: P,
    /// The `K` axis's mapping.
    pub mapping: Mapping,
}

impl<C, P: Float> HeldLz<C, P> {
    /// `chart` at `L_z = 0`, the chart itself, warped, the default.
    #[inline]
    pub fn new(chart: C) -> Self {
        HeldLz {
            chart,
            lz: P::zero(),
            mapping: Mapping::Warped,
        }
    }
}

impl<C, P: Copy> HeldLz<C, P> {
    /// The value of the hidden coordinate `k` at the slice centre: the held `L_z`; `None` for any other `k`.
    pub fn held(&self, k: u32) -> Option<P> {
        (k == HELD_LZ).then_some(self.lz)
    }
}

/// A chart whose momenta are §2.2's construction at a held `L_z` and a `K` its vertical axis reads.
pub trait LzChart: Chart + Copy {
    /// The warp's line `K_max·v^γ_K`.
    fn kinetic_at<F: Float + From<Self::Param>>(&self, v: F) -> F;

    /// `K_max`, the top of the linear option's range.
    fn k_max_at<F: Float + From<Self::Param>>(&self) -> F;

    /// `D ∘ Φ` at the horizontal `u` with the construction at `(L_z, K*) = (lz, k)`.
    fn decode_lz<R>(&self, u: R, lz: R, k: R) -> Decoded<R>
    where
        R: LinkReal + PayloadReal + From<Self::Param>;
}

impl<P: Copy> LzChart for Burrau<P>
where
    f64: From<P>,
{
    #[inline]
    fn kinetic_at<F: Float + From<P>>(&self, v: F) -> F {
        self.kinetic(v)
    }

    #[inline]
    fn k_max_at<F: Float + From<P>>(&self) -> F {
        <F as From<P>>::from(self.k_max)
    }

    #[inline]
    fn decode_lz<R>(&self, u: R, lz: R, k: R) -> Decoded<R>
    where
        R: LinkReal + PayloadReal + From<P>,
    {
        burrau::decode_lz(self, u, lz, k).0
    }
}

impl<P: Copy> LzChart for BurrauThetaK<P>
where
    f64: From<P>,
{
    #[inline]
    fn kinetic_at<F: Float + From<P>>(&self, v: F) -> F {
        self.kinetic(v)
    }

    #[inline]
    fn k_max_at<F: Float + From<P>>(&self) -> F {
        <F as From<P>>::from(self.k_max)
    }

    #[inline]
    fn decode_lz<R>(&self, u: R, lz: R, k: R) -> Decoded<R>
    where
        R: LinkReal + PayloadReal + From<P>,
    {
        burrau_acute::decode_theta_k_lz(self, u, lz, k).0
    }
}

impl<C: LzChart<Param = P>, P: Copy> HeldLz<C, P>
where
    f64: From<P>,
{
    /// The row `b`'s `K*` at the pixel's own held `L_z = lz`: warped, the floor plus the warp's line,
    /// `L_z²/2I + K_max·b^γ_K`, 0 plus the line, bit for bit, at `L_z = 0`; linear, the fixed `K_max·b`.
    #[inline]
    fn row<R: WarpReal + From<P>>(&self, b: R, lz: R) -> R {
        if self.mapping == Mapping::Warped {
            k_floor(lz) + self.chart.kinetic_at(b)
        } else {
            self.chart.k_max_at::<R>() * b
        }
    }

    /// No construction at `(L_z, K*)`, `K*` the row's: `K* < L_z²/2I`, by the construction's exact test.
    fn refused_at(&self, b: f64, lz: f64) -> Option<Constraint> {
        infeasible(1.0, self.row(b, lz), lz).then_some(Constraint::KAboveLzBound)
    }

    /// `D ∘ Φ` at `(u, v)` with the held `L_z = lz`, the row's `K*` at it.
    #[inline]
    fn decode_held<R>(&self, u: R, v: R, lz: R) -> Decoded<R>
    where
        R: LinkReal + PayloadReal + From<P>,
    {
        self.chart.decode_lz(u, lz, self.row(v, lz))
    }
}

impl<C: LzChart<Param = P>, P: Copy> Chart for HeldLz<C, P>
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

    /// The chart's check, then the construction at the held `L_z`.
    fn validate(&self, u: f64, v: f64) -> ValidationResult {
        let lz = f64::from(self.lz);
        refuse(u, v, self.chart.validate(u, v), |_, b| {
            self.refused_at(b, lz)
        })
    }

    /// The construction at the held `L_z` and the row's `K*` there.
    #[inline]
    fn decode<S, R>(&self, u: R, v: R) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<P>,
    {
        self.decode_held(u, v, <R as From<P>>::from(self.lz))
    }
}

impl<C: LzChart<Param = P>, P: Copy> ChartDecode for HeldLz<C, P>
where
    f64: From<P>,
{
    #[inline]
    fn decode_at<S, R>(&self, u: R, v: R) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + WarpReal + From<P>,
    {
        Chart::decode::<S, R>(self, u, v)
    }
}

impl<C: LzChart<Param = P>, P: Copy> Hidden for HeldLz<C, P>
where
    f64: From<P>,
{
    /// The held `L_z`.
    fn hidden(&self, k: u32) -> bool {
        k == HELD_LZ
    }

    fn sliced(&self, k: u32, value: P) -> Option<Self> {
        (k == HELD_LZ).then_some(HeldLz { lz: value, ..*self })
    }
}

impl<C: LzChart<Param = P>, P: Copy> Tiltable for HeldLz<C, P>
where
    f64: From<P>,
{
    /// The construction at `L_z + d`, by value in both modes (R-720), and the row's `K*` at it.
    #[inline]
    fn decode_shifted<S, R>(&self, u: R, v: R, shift: &Shift<R>, _p: &LinkParams<R>) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<P>,
    {
        self.decode_held(u, v, shifted(<R as From<P>>::from(self.lz), HELD_LZ, shift))
    }

    fn validate_shifted(
        &self,
        a: f64,
        b: f64,
        shift: &Shift<f64>,
        p: &LinkParams<f64>,
    ) -> ValidationResult {
        refuse(a, b, self.chart.validate_params(a, b, p), |a, b| {
            self.refusal(a, b, shift)
        })
    }

    fn refusal(&self, _a: f64, b: f64, shift: &Shift<f64>) -> Option<Constraint> {
        self.refused_at(b, shifted(f64::from(self.lz), HELD_LZ, shift))
    }
}

// ── The held K: Φ_θ,L_z ───────────────────────────────────────────────────────────────────────────────────────────

impl<P: Copy> Hidden for BurrauThetaLz<P>
where
    f64: From<P>,
{
    /// The held `K`.
    fn hidden(&self, k: u32) -> bool {
        k == HELD_K
    }

    /// The chart at `K = value`, its `L_z` band that `K`'s.
    fn sliced(&self, k: u32, value: P) -> Option<Self> {
        (k == HELD_K).then_some(BurrauThetaLz { k: value, ..*self })
    }
}

impl<P: Copy> Tiltable for BurrauThetaLz<P>
where
    f64: From<P>,
{
    /// The linear arm (the module's note): the construction at `(L_z(b), K + d)`, `L_z(b)` the centre's band. The bare
    /// chart holds no warp to tilt in, [`HeldK`] does.
    #[inline]
    fn decode_shifted<S, R>(&self, u: R, v: R, shift: &Shift<R>, _p: &LinkParams<R>) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<P>,
    {
        let k = shifted(self.kinetic::<R>(), HELD_K, shift);
        burrau_acute::decode_theta_lz_at(self, u, self.lz(v), k).0
    }

    fn validate_shifted(
        &self,
        a: f64,
        b: f64,
        shift: &Shift<f64>,
        p: &LinkParams<f64>,
    ) -> ValidationResult {
        refuse(a, b, self.validate_params(a, b, p), |a, b| {
            self.refusal(a, b, shift)
        })
    }

    /// No construction at `(L_z(b), K + d)`: `K + d < L_z(b)²/2I`, by the construction's exact test.
    fn refusal(&self, _a: f64, b: f64, shift: &Shift<f64>) -> Option<Constraint> {
        let k = shifted(self.kinetic::<f64>(), HELD_K, shift);
        infeasible(1.0, k, self.lz::<f64>(b)).then_some(Constraint::KAboveLzBound)
    }
}

/// `Φ_θ,L_z` with its held `K`'s mode and its warp (the module's note): `K_max` and `γ_K` are the view's, as the slider
/// span is, the chart having no `K` axis to read them from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeldK<P> {
    /// The registered chart, its `k` the held `K₀`.
    pub chart: BurrauThetaLz<P>,
    /// `K_max > 0`, the warp's kinetic energy at `t = 1`.
    pub k_max: P,
    /// `γ_K ≥ 1`, the warp's exponent.
    pub gamma_k: P,
    /// The `L_z` axis's mapping, the chart's mode (R-708).
    pub mapping: Mapping,
}

impl<P: Copy> HeldK<P> {
    /// `chart` under the warp `(k_max, gamma_k)`, warped, the default.
    #[inline]
    pub fn new(chart: BurrauThetaLz<P>, k_max: P, gamma_k: P) -> Self {
        HeldK {
            chart,
            k_max,
            gamma_k,
            mapping: Mapping::Warped,
        }
    }

    /// The value of the hidden coordinate `k` at the slice centre: the held `K`; `None` for any other `k`.
    pub fn held(&self, k: u32) -> Option<P> {
        self.chart.held(k)
    }
}

impl<P: Copy> HeldK<P>
where
    f64: From<P>,
{
    /// `t₀ = (K₀/K_max)^(1/γ_K)`, a `K₀` below 0 taken at 0: a centre `validate` refuses, its decode still finite.
    #[inline]
    fn t0<R: Float + From<P>>(&self) -> R {
        let wide = <R as From<P>>::from;
        let k0 = Float::max(self.chart.kinetic::<R>(), R::zero());
        Float::powf(k0 / wide(self.k_max), wide(self.gamma_k).recip())
    }

    /// The pixel's own `K` at the offset `d`: `K₀` itself at `d = 0`, else `K_max·t^γ_K` at `t = t₀ + d`, clamped at
    /// `t = 0` (R-709), never mirrored.
    #[inline]
    fn warped_k<R: WarpReal + From<P>>(&self, d: R) -> R {
        let wide = <R as From<P>>::from;
        let t = Float::max(self.t0::<R>() + d, R::zero());
        let moved = invariant::kinetic(wide(self.k_max), wide(self.gamma_k), t);
        select(d == R::zero(), self.chart.kinetic::<R>(), moved)
    }

    /// The warped targets `(L_z, K*)` at row `b`: the pixel's `K`, and the band at it.
    #[inline]
    fn warped<R: WarpReal + From<P>>(&self, b: R, shift: &Shift<R>) -> (R, R) {
        let k = self.warped_k(shifted(R::zero(), HELD_K, shift));
        (self.chart.lz_at(b, k), k)
    }
}

impl<P: Copy> Chart for HeldK<P>
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

    fn validate(&self, u: f64, v: f64) -> ValidationResult {
        self.chart.validate(u, v)
    }

    /// The chart's own: untilted, the held `K` is `K₀` in either mode.
    #[inline]
    fn decode<S, R>(&self, u: R, v: R) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<P>,
    {
        self.chart.decode::<S, R>(u, v)
    }
}

impl<P: Copy> ChartDecode for HeldK<P>
where
    f64: From<P>,
{
    #[inline]
    fn decode_at<S, R>(&self, u: R, v: R) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + WarpReal + From<P>,
    {
        Chart::decode::<S, R>(self, u, v)
    }
}

impl<P: Copy> Hidden for HeldK<P>
where
    f64: From<P>,
{
    /// The held `K`.
    fn hidden(&self, k: u32) -> bool {
        self.chart.hidden(k)
    }

    /// The chart at `K = value`, by value in both modes; the tilt's `t₀` follows it.
    fn sliced(&self, k: u32, value: P) -> Option<Self> {
        let chart = self.chart.sliced(k, value)?;
        Some(HeldK { chart, ..*self })
    }
}

impl<P: Copy> Tiltable for HeldK<P>
where
    f64: From<P>,
{
    /// Warped, the construction at the pixel's own `K` and the band at it; linear, the bare chart's.
    #[inline]
    fn decode_shifted<S, R>(&self, u: R, v: R, shift: &Shift<R>, p: &LinkParams<R>) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<P>,
    {
        if self.mapping == Mapping::Warped {
            let (lz, k) = self.warped(v, shift);
            burrau_acute::decode_theta_lz_at(&self.chart, u, lz, k).0
        } else {
            self.chart.decode_shifted::<S, R>(u, v, shift, p)
        }
    }

    fn validate_shifted(
        &self,
        a: f64,
        b: f64,
        shift: &Shift<f64>,
        p: &LinkParams<f64>,
    ) -> ValidationResult {
        refuse(a, b, self.chart.validate_params(a, b, p), |a, b| {
            self.refusal(a, b, shift)
        })
    }

    /// No construction at the pixel's targets, by the construction's exact test.
    fn refusal(&self, a: f64, b: f64, shift: &Shift<f64>) -> Option<Constraint> {
        if self.mapping == Mapping::Warped {
            let (lz, k) = self.warped(b, shift);
            infeasible(1.0, k, lz).then_some(Constraint::KAboveLzBound)
        } else {
            self.chart.refusal(a, b, shift)
        }
    }

    /// Warped, where `t₀ + d < 0`: the coordinate clamped at `t = 0` (R-709).
    fn held_at_floor(&self, shift: &Shift<f64>) -> bool {
        self.mapping == Mapping::Warped && self.t0::<f64>() + shifted(0.0, HELD_K, shift) < 0.0
    }
}
