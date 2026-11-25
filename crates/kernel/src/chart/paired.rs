//! Pair maps (design `principia_chart_composition.md` § 5.1): `(a, b) = M(u, v)` sits before a chart's axes, so a
//! chart `C` seen through `M` is `Φ_C(M(u, v))`. [`Paired<C, M>`] is the wrapper, as [`super::Tilted`] wraps a chart
//! under its tilt; `M` is the chart's type, its parameters values.
//!
//! | Map | `(a, b)` | `u`'s extension | `v`'s extension |
//! |---|---|---|---|
//! | [`Affine`] | `(u, v)` | affine | affine |
//! | [`Polar`] about `c` | `c + r·(cos ϕ, sin ϕ)`, `r = lerp(radius, u)`, `ϕ = lerp(angle, v)` | pole-crossing, or bounded at `r = 0` | periodic, `2π` |
//! | [`LogPolar`] about `c` | `c + r·(cos ϕ, sin ϕ)`, `r = r₀·e^τ`, `τ = lerp(log_radius, u)` | bounded to `[0, 1]` | periodic, `2π` |
//!
//! `(a, b)` are the chart's own parameters, so polar and log-polar apply to any pair: a polar view of `(ξ, η)` about a
//! body is a ring picture of region D. The affine map hands `(u, v)` on with no arithmetic, so [`Paired`] over it is
//! the chart's own bit for bit.
//!
//! **Pole-crossing** ([`Pole::Crossing`]): `r` runs on through 0, and `(r, ϕ)` and `(−r, ϕ + π)` are one point, each
//! the other's partner ([`Polar::partner`]). The inverse takes the `r ≥ 0` branch, a pure function of the point.
//! **Bounded** ([`Pole::Bounded`]): validation clamps `r` at 0.
//!
//! **The inverse's `atan2`** takes its second operand through a select that maps `−0` to `+0`, so a GPU whose `atan2`
//! reads that zero's sign agrees with IEEE's (pitfalls § 11); `atan2(0, 0)` is the pole, where any `ϕ` is the point.
//!
//! **`map` and `decode`.** `M` is evaluated with [`Real`]'s functions, the pinned libm's at f64. [`Chart::map`] is bound
//! by `Float` alone, so it evaluates `M` at f64 and narrows `(a, b)` to its own float; only `decode` is a `D ∘ Φ` entry.
//! Don't call `Float`'s `sin`, `cos` or `exp` there instead: they are the platform's, not the pinned ones.

use spirv_std::num_traits::{Float, FloatConst};

use super::mixed::lerp;
use super::{Chart, ChartDecode, ChartOut, Constraint, ValidationResult};
use crate::chart::invariant::WarpReal;
use crate::decode::Decoded;
use crate::encode::by_kind::{ByKind, Coord};
use crate::encode::Encoded;
use crate::generated::links::{LinkParams, LinkReal, Selection};
use crate::payload::PayloadReal;
use crate::real::Real;

/// What a pair map's coordinate does past the unit square.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PairExtension {
    /// The formula continues.
    Affine,
    /// The coordinate wraps with the map's period.
    Periodic,
    /// The radius runs through the pole; the partner is `ϕ + π`.
    PoleCrossing,
    /// Clamped at its edge.
    Bounded,
}

/// A pair map `(a, b) = M(u, v)` (the module's note).
pub trait PairMap: Copy {
    /// The float type its parameters are held at.
    type Param: Copy;

    /// The map's name, for the chart's label.
    fn name(&self) -> &'static str;

    /// `u`'s and `v`'s extensions.
    fn extensions(&self) -> [PairExtension; 2];

    /// `(a, b)` at `(u, v)`, through [`Real`]'s functions.
    fn forward<R: Real + Float + From<Self::Param>>(&self, u: R, v: R) -> [R; 2];

    /// `(u, v)` at `(a, b)`, the map's inverse, on the branch the module's note names.
    fn inverse<R: Real + Float + From<Self::Param>>(&self, a: R, b: R) -> [R; 2];

    /// `|det ∂(a, b)/∂(u, v)|` at `(u, v)`: the map's area factor.
    fn area<R: Real + Float + From<Self::Param>>(&self, u: R, v: R) -> R;

    /// `(u, v)` brought inside the map's own bounds (its bounded extensions); unchanged where it is inside.
    fn bound(&self, u: f64, v: f64) -> [f64; 2];
}

/// The identity, the default: each axis written alone.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Affine<P>(pub core::marker::PhantomData<P>);

impl<P: Copy> PairMap for Affine<P> {
    type Param = P;

    fn name(&self) -> &'static str {
        "affine"
    }

    fn extensions(&self) -> [PairExtension; 2] {
        [PairExtension::Affine, PairExtension::Affine]
    }

    #[inline]
    fn forward<R: Real + Float + From<P>>(&self, u: R, v: R) -> [R; 2] {
        [u, v]
    }

    #[inline]
    fn inverse<R: Real + Float + From<P>>(&self, a: R, b: R) -> [R; 2] {
        [a, b]
    }

    #[inline]
    fn area<R: Real + Float + From<P>>(&self, _u: R, _v: R) -> R {
        R::one()
    }

    fn bound(&self, u: f64, v: f64) -> [f64; 2] {
        [u, v]
    }
}

/// What polar's radius does at the pole (the module's note).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pole {
    /// `r` runs through 0; `(−r, ϕ + π)` is `(r, ϕ)`'s partner.
    Crossing,
    /// `r` is clamped at 0.
    Bounded,
}

/// `c + r·(cos ϕ, sin ϕ)`, from `(sin ϕ, cos ϕ)`.
#[inline]
fn ring<F: Float>(c: [F; 2], r: F, (s, k): (F, F)) -> [F; 2] {
    [c[0] + r * k, c[1] + r * s]
}

/// `(r, ϕ)` about `c` at `(a, b)`: `r ≥ 0`, `ϕ` in `[lo, lo + 2π)`, `atan2`'s second operand a `+0` where it is a zero.
#[inline]
fn polar_of<R: Real + Float>(c: [R; 2], a: R, b: R, lo: R, period: R) -> (R, R) {
    let (x, y) = (a - c[0], b - c[1]);
    let x = if x == R::zero() { R::zero() } else { x };
    let r = x.hypotenuse(y);
    let phi = y.arctan2(x);
    let turns = ((phi - lo) / period).floor();
    (r, phi - turns * period)
}

/// `2π` at the parameters' type, widened.
#[inline]
fn period<P: Float + FloatConst, R: From<P>>() -> R {
    <R as From<P>>::from(P::TAU())
}

/// Polar about `centre` (the module's note): `r` on `radius` along `u`, `ϕ` on `angle` along `v`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Polar<P> {
    /// The centre `c` in the chart's own parameters.
    pub centre: [P; 2],
    /// `r` at `u = 0` and `u = 1`.
    pub radius: [P; 2],
    /// `ϕ` at `v = 0` and `v = 1`, radians.
    pub angle: [P; 2],
    /// What `r` does at the pole.
    pub pole: Pole,
}

impl<P: Copy + Float + FloatConst> Polar<P> {
    /// `(r, ϕ)` at `(u, v)`.
    #[inline]
    fn coords<F: Float + From<P>>(&self, u: F, v: F) -> (F, F) {
        let w = |x: [P; 2]| x.map(<F as From<P>>::from);
        (lerp(w(self.radius), u), lerp(w(self.angle), v))
    }

    /// The pole-crossing partner of `(u, v)`: the chart point at `(−r, ϕ + π)`, the same `(a, b)`.
    pub fn partner(&self, u: f64, v: f64) -> [f64; 2]
    where
        f64: From<P>,
    {
        let [r0, r1] = self.radius.map(f64::from);
        let [p0, p1] = self.angle.map(f64::from);
        let r = lerp([r0, r1], u);
        [(-r - r0) / (r1 - r0), v + f64::from(P::PI()) / (p1 - p0)]
    }

    /// `v`'s period: the step in `v` that turns `ϕ` by `2π`.
    pub fn v_period(&self) -> f64
    where
        f64: From<P>,
    {
        let [p0, p1] = self.angle.map(f64::from);
        f64::from(P::TAU()) / (p1 - p0)
    }
}

impl<P: Copy + Float + FloatConst> PairMap for Polar<P>
where
    f64: From<P>,
{
    type Param = P;

    fn name(&self) -> &'static str {
        "polar"
    }

    fn extensions(&self) -> [PairExtension; 2] {
        let r = match self.pole {
            Pole::Crossing => PairExtension::PoleCrossing,
            Pole::Bounded => PairExtension::Bounded,
        };
        [r, PairExtension::Periodic]
    }

    #[inline]
    fn forward<R: Real + Float + From<P>>(&self, u: R, v: R) -> [R; 2] {
        let (r, phi) = self.coords(u, v);
        ring(self.centre.map(<R as From<P>>::from), r, phi.sine_cosine())
    }

    #[inline]
    fn inverse<R: Real + Float + From<P>>(&self, a: R, b: R) -> [R; 2] {
        let [r0, r1] = self.radius.map(<R as From<P>>::from);
        let [p0, p1] = self.angle.map(<R as From<P>>::from);
        let (r, phi) = polar_of(
            self.centre.map(<R as From<P>>::from),
            a,
            b,
            p0,
            period::<P, R>(),
        );
        [(r - r0) / (r1 - r0), (phi - p0) / (p1 - p0)]
    }

    #[inline]
    fn area<R: Real + Float + From<P>>(&self, u: R, _v: R) -> R {
        let [r0, r1] = self.radius.map(<R as From<P>>::from);
        let [p0, p1] = self.angle.map(<R as From<P>>::from);
        (lerp([r0, r1], u) * (r1 - r0) * (p1 - p0)).abs()
    }

    fn bound(&self, u: f64, v: f64) -> [f64; 2] {
        let [r0, r1] = self.radius.map(f64::from);
        match self.pole {
            Pole::Bounded if lerp([r0, r1], u) < 0.0 => [-r0 / (r1 - r0), v],
            _ => [u, v],
        }
    }
}

/// Log-polar about `centre` (the module's note): `r = r₀·e^τ`, `τ` on `log_radius` along `u`, `ϕ` on `angle` along `v`;
/// no pole in view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LogPolar<P> {
    /// The centre `c` in the chart's own parameters.
    pub centre: [P; 2],
    /// `r₀`, positive.
    pub r0: P,
    /// `τ` at `u = 0` and `u = 1`.
    pub log_radius: [P; 2],
    /// `ϕ` at `v = 0` and `v = 1`, radians.
    pub angle: [P; 2],
}

impl<P: Copy + Float + FloatConst> PairMap for LogPolar<P>
where
    f64: From<P>,
{
    type Param = P;

    fn name(&self) -> &'static str {
        "log-polar"
    }

    fn extensions(&self) -> [PairExtension; 2] {
        [PairExtension::Bounded, PairExtension::Periodic]
    }

    #[inline]
    fn forward<R: Real + Float + From<P>>(&self, u: R, v: R) -> [R; 2] {
        let w = |x: [P; 2]| x.map(<R as From<P>>::from);
        let r = <R as From<P>>::from(self.r0) * lerp(w(self.log_radius), u).exponential();
        ring(w(self.centre), r, lerp(w(self.angle), v).sine_cosine())
    }

    #[inline]
    fn inverse<R: Real + Float + From<P>>(&self, a: R, b: R) -> [R; 2] {
        let [t0, t1] = self.log_radius.map(<R as From<P>>::from);
        let [p0, p1] = self.angle.map(<R as From<P>>::from);
        let (r, phi) = polar_of(
            self.centre.map(<R as From<P>>::from),
            a,
            b,
            p0,
            period::<P, R>(),
        );
        let tau = (r / <R as From<P>>::from(self.r0)).logarithm();
        [(tau - t0) / (t1 - t0), (phi - p0) / (p1 - p0)]
    }

    #[inline]
    fn area<R: Real + Float + From<P>>(&self, u: R, _v: R) -> R {
        let [t0, t1] = self.log_radius.map(<R as From<P>>::from);
        let [p0, p1] = self.angle.map(<R as From<P>>::from);
        let r = <R as From<P>>::from(self.r0) * lerp([t0, t1], u).exponential();
        (r * r * (t1 - t0) * (p1 - p0)).abs()
    }

    fn bound(&self, u: f64, v: f64) -> [f64; 2] {
        [u.clamp(0.0, 1.0), v]
    }
}

/// A chart `C` seen through the pair map `M` (the module's note).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Paired<C, M> {
    /// The chart.
    pub chart: C,
    /// The pair map before its axes.
    pub pair: M,
}

impl<C: Chart, M: PairMap<Param = C::Param>> Paired<C, M> {
    /// `chart` through `pair`.
    pub fn new(chart: C, pair: M) -> Self {
        Paired { chart, pair }
    }
}

impl<C: Chart, M: PairMap<Param = C::Param>> Chart for Paired<C, M>
where
    f64: From<C::Param>,
{
    type Param = C::Param;

    #[inline]
    fn map<F: Float + From<C::Param>>(&self, u: F, v: F) -> ChartOut<F> {
        let wide = |x: F| x.to_f64().unwrap_or(f64::NAN);
        let narrow = |x: f64| <F as spirv_std::num_traits::NumCast>::from(x).unwrap_or_else(F::nan);
        let [a, b] = self.pair.forward(wide(u), wide(v));
        self.chart.map(narrow(a), narrow(b))
    }

    fn forbids_energy_normalisation(&self) -> bool {
        self.chart.forbids_energy_normalisation()
    }

    fn lz_hold(&self) -> crate::chart::LzHold {
        self.chart.lz_hold()
    }

    fn name(&self) -> &str {
        self.chart.name()
    }

    fn requires_per_pixel_mass(&self) -> bool {
        self.chart.requires_per_pixel_mass()
    }

    fn phi_finite(&self, u: f64, v: f64) -> bool {
        let [a, b] = self.pair.forward(u, v);
        a.is_finite() && b.is_finite() && self.chart.phi_finite(a, b)
    }

    fn validate(&self, u: f64, v: f64) -> ValidationResult {
        self.validate_params(u, v, &LinkParams::DEFAULT)
    }

    /// The map's own bound, then the chart's check at `(a, b)`.
    // The plan is unclear here; chose to refuse any failure of the inner chart's check, as `Tilted` does, because a
    // point it clamps in `(a, b)` can have its nearest `(u, v)` across a periodic seam.
    fn validate_params(&self, u: f64, v: f64, p: &LinkParams<f64>) -> ValidationResult {
        let [bu, bv] = self.pair.bound(u, v);
        let [a, b] = self.pair.forward(bu, bv);
        match self.chart.validate_params(a, b, p) {
            ValidationResult::Valid if bu == u && bv == v => ValidationResult::Valid,
            ValidationResult::Valid => ValidationResult::Clamp {
                u: bu,
                v: bv,
                moved: [bu != u, bv != v],
                failed: Constraint::AxisDomain,
            },
            ValidationResult::Project { failed, .. }
            | ValidationResult::Clamp { failed, .. }
            | ValidationResult::Reject { failed } => ValidationResult::Reject { failed },
        }
    }

    #[inline]
    fn decode<S, R>(&self, u: R, v: R) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<C::Param>,
    {
        let [a, b] = self.pair.forward(u, v);
        self.chart.decode::<S, R>(a, b)
    }

    #[inline]
    fn decode_params<S, R>(&self, u: R, v: R, p: &LinkParams<R>) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<C::Param>,
    {
        let [a, b] = self.pair.forward(u, v);
        self.chart.decode_params::<S, R>(a, b, p)
    }
}

impl<C: ChartDecode, M: PairMap<Param = C::Param>> ChartDecode for Paired<C, M>
where
    f64: From<C::Param>,
{
    #[inline]
    fn decode_at<S, R>(&self, u: R, v: R) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + WarpReal + From<C::Param>,
    {
        let [a, b] = self.pair.forward(u, v);
        self.chart.decode_at::<S, R>(a, b)
    }

    #[inline]
    fn decode_at_params<S, R>(&self, u: R, v: R, p: &LinkParams<R>) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + WarpReal + From<C::Param>,
    {
        let [a, b] = self.pair.forward(u, v);
        self.chart.decode_at_params::<S, R>(a, b, p)
    }
}

/// The chart's coordinates of the state, through the map's inverse; an axis that cannot place the state leaves both
/// unplaced, since `M` mixes them.
impl<C: ByKind, M: PairMap<Param = C::Param>> ByKind for Paired<C, M>
where
    f64: From<C::Param>,
{
    fn coords<S, R>(&self, e: &Encoded<R>) -> [Coord<R>; 2]
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<C::Param>,
    {
        match self.chart.coords::<S, R>(e) {
            [Coord::At(a), Coord::At(b)] => self.pair.inverse(a, b).map(Coord::At),
            _ => [Coord::Unplaced, Coord::Unplaced],
        }
    }
}
