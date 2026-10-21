//! Kind 4, the coupled curve: the Burrau family's encode (inverse_encode_contract Part 5 kind 4; dd_encode §3.1, §3.4;
//! R-24, R-27; REQ-ENC-011, REQ-ENC-017, REQ-ENC-020, REQ-ENC-022). The family's systems lie on the embed
//! `γ : ν ↦ (configuration, masses)` (`crate::chart::burrau::embed`), and encode reads `ν` off `C(x)` two ways.
//!
//! **(a) On the curve, the closed form** ([`closed_form`]). `C(x)`'s triangle has legs `a = ‖r₁ − r₀‖`,
//! `b = ‖r₂ − r₀‖` and the side `c = ‖r₂ − r₁‖`, the Burrau convention's sides, each opposite the body of the same
//! mass (`crate::burrau`): the acute angle is `θ = atan2(a, b)` (`tan θ = a/b`, the angle at `r₂`), and
//! `ν(θ) = sec θ − tan θ` ([`nu_of`], as the acute-angle charts evaluate it). The mass coupling is checked as
//! `μ₁ = log(b/c)`, `μ₂ = log(a/c)`, `μₖ = log(mₖ/m₀)` of `C(x)`'s masses: [`ClosedForm::coupling`] holds the two
//! differences. Whether `x` is on the curve is decided by the projection metric at `ν(θ)` (below): its mass term is 0
//! exactly where the coupling holds and the triangle is `γ(ν)`'s.
//!
//! **(b) Off the curve, the projection** ([`nearest`]; dd_encode §3.4; R-24). `ν* = argmin_ν d²(ν)` with
//!
//! ```text
//! d²(ν) = ‖n(α, β) − n(α_γ(ν), β_γ(ν))‖² + ‖m − m_γ(ν)‖²
//! ```
//!
//! the shape sphere's chordal distance ⊕ the mass simplex's Euclidean one, unit weights. `n(α, β)` is `C(x)`'s shape
//! point, `crate::shape::shape_vec` of its mass-weighted Jacobi vectors, `w` read as `|w|` as `β` reads `|λ̃_y|`
//! (R-438), so the point is on the upper hemisphere where the embed's are ([`state_point`]); `n(α_γ(ν), β_γ(ν))` is the
//! embed's, through the same Jacobi weighting and the same map ([`curve_point`]). The search runs over the chart's own
//! `ν` span ([`CurveChart::nu_span`]): a grid of [`GRID`] + 1 points, the span's edges included, then golden-section
//! steps ([`GOLDEN_STEPS`], a fixed count) inside the best grid point's two neighbouring cells, and `ν*` the better of
//! the two searches, so an edge minimum is the edge itself ([`Projection::edge`]).
//!
//! **The landing on the curve** ([`project`], kind 4 (a) and (b)): on the curve where `ν(θ)` lies in the span and
//! `d(ν(θ)) ≤ tol`, the argmin bypassed; else projected to `ν*` where `d(ν*) ≤ tol`; else latent `z`, `E(x)`'s, always
//! available (Part 6 case 3). `d` is reported in every case. `tol` is the caller's, REQ-ENC-026's calibration (R-71),
//! proposed with its evidence in `fixtures/gates/curve_projection/`.
//!
//! **Past the span's edge, and the fold** (dd_encode §3.1, §3.4; REQ-ENC-022). The curve axis of every Burrau chart,
//! `ν`, or `θ` on the acute-angle charts, is bounded (chart_reference §5.4): past its edge the chart shows no system.
//! (`K` continues past `t = 1`, R-508, and `L_z` and δm are affine: those axes are read unbounded.) So a `ν` past the
//! span is no chart point: the closed form does not land there, and the search does not leave the span. An input at
//! `θ > π/4` on a folded chart is the leg-swapped twin of a fold shape, bodies 1 and 2 exchanged, a distinct labelled
//! system (R-27): encode does not relabel it. Its closed-form `ν` is below `√2 − 1`, past the fold's end, and it falls
//! back to latent `z`, unless the fold's end is within `tol` of it; [`ClosedForm::past_fold`] flags it.
//!
//! **Into the active chart** ([`ByKind`] for the Burrau charts; dd_encode §3.1, "The composition"). Kind 4 supplies the
//! curve axis's coordinate, [`curve_axis`]: [`project`]'s landing at the caller's `tol`, the closed form's coordinate
//! on the curve (`θ` read directly on the acute-angle charts), `ν*`'s where projected (at an edge of the span, the
//! axis's end, 0 or 1, exactly), and [`Coord::Unplaced`] where the landing is latent. The other axis is read by its own
//! kind: `K` by the warp's inverse, `L_z` by the band's, δm by the blend's least squares, the Euclid plane's `m` an
//! annotation. Then `by_kind::encode_into_within`'s comparison, Part 6 step 4's `‖·‖_phys ≤ ε_phys`, decides chart
//! against latent, as for every chart: `d` does not see the momentum. A chart landing needs both. The curve encode,
//! `past_fold` among it, goes to the caller in `ChartEncoded::curve`; `by_kind::encode_into` is `tol = 0`, where
//! rounding alone puts `d > 0` on the curve and a curve chart places essentially no state, so the lookup calls the
//! `_within` forms at REQ-ENC-026's tolerance (inverse_encode_contract Part 6, step 4). The δm strip, whose masses
//! leave the curve's off its `v = 0` row, reads `θ` off the triangle alone; the projection does not apply there.
//!
//! **The branch inputs** (gpu_determinism_note, "The discipline"; the human's ruling on RQ-298, 21 Oct 2026: a branch
//! input that must agree across backends is built only from operations every IEEE backend rounds alike, with no
//! intermediate that can be subnormal, `IeeeBits::product`'s form the pattern). Every distance a comparison reads is a
//! squared distance or a squared length built by [`sq`]: a difference below [`floor`], `2^⌊e/4⌋` for the precision's
//! least normal exponent `e` (`2⁻³¹` at f32), is taken as 0, so its square's partial products are normal, then the
//! square is `IeeeBits::product`, and the squares are added in one fixed order. No `fma` is written. They are: the
//! search's `f < best` and the golden step's `f(c) < f(d)`; the landings' `d² ≤ tol²`, `tol² = product(tol, tol)`;
//! [`ClosedForm::past_fold`]'s `a² > b²`; and the δm strip's guard, `‖m_t − m_B‖²` against 0. The other comparisons
//! read a value directly, with no arithmetic between: the span test on `ν(θ)`, the grid's last point, the edge flag
//! (`ν* = lo` or `hi`, exact) and [`curve_axis`]'s `ν*` against the axis's ends, and `Φ_θ,L_z`'s band width against
//! 0, which is 0 exactly where `K` or `λ_L` is. The search's points are formed alike on every backend: the golden
//! ratio `g = (√5 − 1)/2` is `IeeeBits::INV_PHI`, a constant correctly rounded at each width, not a runtime `sqrt`;
//! the grid's `1/64` is `2⁻⁶` from its bits (`IeeeBits::pow2`), so `(hi − lo)·2⁻⁶` and `k·2⁻⁶` are exact (a power of
//! two scales without rounding where the result is normal, and no division, which WGSL does not round correctly, is
//! formed); each point is then a product in `IeeeBits::product`'s form and a lone add or subtraction. The values the
//! distances are formed from are the decode's own (the embed's divisions, the weighting's `sqrt`) and `C`'s, which a
//! backend rounds within its own bounds; two backends can differ on a comparison only where the two distances lie
//! within those roundings of each other.

use spirv_std::num_traits::Float;

use super::by_kind::{invariants, warp_inverse, ByKind, Coord};
use super::{blocks, Encoded};
use crate::chart::burrau::{embed, Burrau, EuclidPlane};
use crate::chart::burrau_acute::{nu_of, BurrauThetaDeltaM, BurrauThetaK, BurrauThetaLz};
use crate::decode::Ic;
use crate::generated::constants;
use crate::generated::links::{LinkReal, Selection};
use crate::payload::PayloadReal;
use crate::real::IeeeBits;
use crate::shape::shape_vec;

/// `t` where `c` holds, else `f`: both operands are computed before the choice, so it is a select, not a branch.
#[inline]
fn select<T: Copy>(c: bool, t: T, f: T) -> T {
    if c {
        t
    } else {
        f
    }
}

/// The search's grid is `2^GRID_LOG2` cells, so its spacing is the span times a power of two, exact (the module's
/// note): the register's `curve_grid_log2`.
pub const GRID_LOG2: i32 = constants::CURVE_GRID_LOG2;

/// The search's grid: `GRID` cells, `GRID + 1` points, the span's edges included (the register's `curve_grid_log2`).
pub const GRID: u32 = constants::CURVE_GRID;

/// The golden-section steps inside the best grid point's two cells: each shrinks the bracket by the golden ratio, so
/// 80 take a width of `2/64` below `10⁻¹⁷`, past f64's resolution on `(0, 1)` (the register's `curve_golden_steps`).
pub const GOLDEN_STEPS: u32 = constants::CURVE_GOLDEN_STEPS;

// ── The metric ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// A point of the metric's space: a shape-sphere point `n` and the masses `m`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point<R> {
    /// The shape point, on the upper hemisphere.
    pub n: [R; 3],
    /// The masses.
    pub m: [R; 3],
}

/// The metric's point of the state `ic` (the module's note): its shape point by the encode's Jacobi weighting
/// (`super::blocks::jacobi`) and `crate::shape::shape_vec`, `w` as `|w|`, and its masses.
#[inline]
pub fn state_point<R: LinkReal>(ic: &Ic<R>) -> Point<R> {
    let j = blocks::jacobi(ic);
    let n = shape_vec(j.rho_t, j.lambda_t);
    Point {
        n: [n[0], n[1], Float::abs(n[2])],
        m: ic.m,
    }
}

/// The metric's point of `γ(ν)`: [`state_point`] of the embed's system at rest.
#[inline]
pub fn curve_point<R: LinkReal>(nu: R) -> Point<R> {
    let s = embed(nu);
    state_point(&Ic {
        m: s.m,
        r: s.r,
        p: [[R::zero(); 2]; 3],
    })
}

/// The least difference [`sq`] squares: `2^⌊e/4⌋`, `e` the precision's least normal exponent (`2⁻³¹` at f32, `2⁻²⁵⁵`
/// at f64), formed from its bits.
#[inline]
pub fn floor<R: IeeeBits + Float>() -> R {
    R::pow2(IeeeBits::exponent(R::min_positive_value()) / 4)
}

/// `x²` in the branch inputs' form (the module's note): 0 where `|x| <` [`floor`], else `IeeeBits::product(x, x)`.
#[inline]
pub fn sq<R: IeeeBits + Float>(x: R) -> R {
    let x = select(Float::abs(x) < floor(), R::zero(), x);
    x.product(x)
}

/// `d²` between `x` and `y` (R-24): `Σ (nₖ − n′ₖ)² + Σ (mᵢ − m′ᵢ)²`, unit weights, each square [`sq`], added in that
/// order.
#[inline]
pub fn distance2<R: IeeeBits + Float>(x: &Point<R>, y: &Point<R>) -> R {
    let shape = sq(x.n[0] - y.n[0]) + sq(x.n[1] - y.n[1]) + sq(x.n[2] - y.n[2]);
    shape + sq(x.m[0] - y.m[0]) + sq(x.m[1] - y.m[1]) + sq(x.m[2] - y.m[2])
}

/// `d²(ν)` of `x` from the curve: [`distance2`] to [`curve_point`]`(ν)`.
#[inline]
pub fn curve_distance2<R: LinkReal + IeeeBits>(x: &Point<R>, nu: R) -> R {
    distance2(x, &curve_point(nu))
}

// ── (a) The closed form ────────────────────────────────────────────────────────────────────────────────────────────

/// The closed form read off a canonical state (the module's note).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClosedForm<R> {
    /// The sides `(a, b, c)`: `‖r₁ − r₀‖`, `‖r₂ − r₀‖`, `‖r₂ − r₁‖`.
    pub sides: [R; 3],
    /// The acute angle `θ = atan2(a, b)`.
    pub theta: R,
    /// `ν(θ) = sec θ − tan θ`.
    pub nu: R,
    /// The coupling's differences, `(log(m₁/m₀) − log(b/c), log(m₂/m₀) − log(a/c))`: both 0 on the curve.
    pub coupling: [R; 2],
    /// `θ > π/4`, decided as `a² > b²`: past the fold, the leg-swapped labelling (REQ-ENC-022).
    pub past_fold: bool,
}

/// `‖v‖²` in the branch inputs' form.
#[inline]
fn length2<R: IeeeBits + Float>(v: [R; 2]) -> R {
    sq(v[0]) + sq(v[1])
}

/// The closed form of the canonical state `ic` (the module's note).
#[inline]
pub fn closed_form<R: LinkReal + IeeeBits>(ic: &Ic<R>) -> ClosedForm<R> {
    let [r0, r1, r2] = ic.r;
    let d = |p: [R; 2], q: [R; 2]| [p[0] - q[0], p[1] - q[1]];
    let (a2, b2, c2) = (length2(d(r1, r0)), length2(d(r2, r0)), length2(d(r2, r1)));
    let (a, b, c) = (Float::sqrt(a2), Float::sqrt(b2), Float::sqrt(c2));
    let theta = Float::atan2(a, b);
    let [m0, m1, m2] = ic.m;
    ClosedForm {
        sides: [a, b, c],
        theta,
        nu: nu_of(theta),
        coupling: [
            Float::ln(m1 / m0) - Float::ln(b / c),
            Float::ln(m2 / m0) - Float::ln(a / c),
        ],
        past_fold: a2 > b2,
    }
}

// ── (b) The projection ─────────────────────────────────────────────────────────────────────────────────────────────

/// The curve's nearest point (the module's note).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Projection<R> {
    /// `ν*`.
    pub nu: R,
    /// `d²(ν*)`.
    pub d2: R,
    /// Whether `ν*` is an edge of the span.
    pub edge: bool,
}

impl<R: Float> Projection<R> {
    /// `d(ν*)`.
    #[inline]
    pub fn distance(&self) -> R {
        Float::sqrt(self.d2)
    }
}

/// `(ν, f)` where `f` is less than `best`'s, or `best`'s is NaN; else `best`: ties keep `best`.
#[inline]
fn better<R: Float>(best: (R, R), candidate: (R, R)) -> (R, R) {
    let less = select(candidate.1 < best.1, candidate, best);
    select(Float::is_nan(best.1), candidate, less)
}

/// `ν*`, the curve's nearest point to `x` over `span = [lo, hi]`, `lo ≤ hi` (the module's note): the grid, then the
/// golden section inside the best grid point's neighbouring cells, and the better of the two. The one source of the
/// curve axis's nearest point: every consumer calls it (R-542).
pub fn nearest<R: LinkReal + IeeeBits>(x: &Point<R>, span: [R; 2]) -> Projection<R> {
    let [lo, hi] = span;
    let one = R::one();
    let mut cells = R::zero();
    for _ in 0..GRID {
        cells = cells + one;
    }
    let step = R::pow2(-GRID_LOG2);
    let width = hi - lo;
    let h = width.product(step);
    let at = |k: R| select(k == cells, hi, lo + width.product(k.product(step)));
    let mut best = (lo, curve_distance2(x, lo));
    let mut k = R::zero();
    for _ in 0..GRID {
        k = k + one;
        let nu = at(k);
        best = better(best, (nu, curve_distance2(x, nu)));
    }
    let grid = best;
    let mut a = Float::max(lo, grid.0 - h);
    let mut b = Float::min(hi, grid.0 + h);
    let g = R::INV_PHI;
    let mut c = b - g.product(b - a);
    let mut d = a + g.product(b - a);
    let mut fc = curve_distance2(x, c);
    let mut fd = curve_distance2(x, d);
    for _ in 0..GOLDEN_STEPS {
        let left = fc < fd;
        let (na, nb) = (select(left, a, c), select(left, d, b));
        let kept = select(left, (c, fc), (d, fd));
        let fresh = select(left, nb - g.product(nb - na), na + g.product(nb - na));
        let f = curve_distance2(x, fresh);
        (c, fc) = select(left, (fresh, f), kept);
        (d, fd) = select(left, kept, (fresh, f));
        (a, b) = (na, nb);
    }
    let golden = better((d, fd), (c, fc));
    let (nu, d2) = better(grid, golden);
    Projection {
        nu,
        d2,
        edge: (nu == lo) | (nu == hi),
    }
}

/// Where a curve encode lands (the module's note).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CurveLanding<R> {
    /// On the curve at the closed form's `ν`, the argmin bypassed.
    OnCurve { nu: R },
    /// Projected to `ν*`, within the tolerance.
    Projected { nu: R },
    /// In latent `z`, `E(x)`'s: the curve chart cannot represent the point.
    Latent,
}

/// A curve encode: the closed form, the projection where it ran, `d` and the landing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CurveEncoded<R> {
    /// The closed form, its flags among it.
    pub closed: ClosedForm<R>,
    /// `d²` at the closed form's `ν`.
    pub closed_d2: R,
    /// The projection; `None` where the closed form was on the curve and the argmin was bypassed.
    pub projection: Option<Projection<R>>,
    /// `d` at the landing's `ν`, or at `ν*` for latent `z`: reported in every case.
    pub d: R,
    /// The landing.
    pub landing: CurveLanding<R>,
}

/// The curve encode of `e` over `span = [lo, hi]` at the tolerance `tol` (the module's note): on the curve where the
/// closed form's `ν` is in the span and `d ≤ tol` there; else projected where `d(ν*) ≤ tol`; else latent `z`.
pub fn project<R: LinkReal + IeeeBits>(e: &Encoded<R>, span: [R; 2], tol: R) -> CurveEncoded<R> {
    let x = state_point(&e.canonical.ic);
    let closed = closed_form(&e.canonical.ic);
    let tol2 = tol.product(tol);
    let closed_d2 = curve_distance2(&x, closed.nu);
    let in_span = (closed.nu >= span[0]) & (closed.nu <= span[1]);
    if in_span & (closed_d2 <= tol2) {
        return CurveEncoded {
            closed,
            closed_d2,
            projection: None,
            d: Float::sqrt(closed_d2),
            landing: CurveLanding::OnCurve { nu: closed.nu },
        };
    }
    let p = nearest(&x, span);
    CurveEncoded {
        closed,
        closed_d2,
        projection: Some(p),
        d: p.distance(),
        landing: select(
            p.d2 <= tol2,
            CurveLanding::Projected { nu: p.nu },
            CurveLanding::Latent,
        ),
    }
}

// ── Into the active chart ──────────────────────────────────────────────────────────────────────────────────────────

/// A Burrau chart whose systems lie on the curve: the `ν` its curve axis spans and the axis's coordinate at a `ν`.
/// The δm strip is not one: off its `v = 0` row its masses leave the curve's.
pub trait CurveChart: ByKind {
    /// `[ν(0), ν(1)]`, the `ν` the chart's decode gives at its curve axis's two ends, `t = 0` and `t = 1`.
    fn nu_ends<R: LinkReal + From<Self::Param>>(&self) -> [R; 2];

    /// `[lo, hi]`, [`CurveChart::nu_ends`] ordered.
    #[inline]
    fn nu_span<R: LinkReal + From<Self::Param>>(&self) -> [R; 2] {
        let [x, y] = self.nu_ends::<R>();
        [Float::min(x, y), Float::max(x, y)]
    }

    /// The curve axis's chart coordinate at `ν`.
    fn at_nu<R: LinkReal + From<Self::Param>>(&self, nu: R) -> R;

    /// The curve axis's chart coordinate at the closed form `c`, on the curve: [`CurveChart::at_nu`] at its `ν`.
    #[inline]
    fn at_closed<R: LinkReal + From<Self::Param>>(&self, c: &ClosedForm<R>) -> R {
        self.at_nu(c.nu)
    }
}

/// `t` as a coordinate of a bounded axis: [`Coord::At`] on `[0, 1]`, else [`Coord::Unplaced`] (the module's note).
#[inline]
pub fn bounded<R: Float>(t: R) -> Coord<R> {
    select(
        (t >= R::zero()) & (t <= R::one()),
        Coord::At(t),
        Coord::Unplaced,
    )
}

/// Kind 4's coordinate of `e` on `chart`'s curve axis at the tolerance `tol` (the module's note; Part 5 kind 4), and
/// the curve encode it came from: [`project`] over the chart's span; on the curve, [`CurveChart::at_closed`]; projected
/// to `ν*`, [`CurveChart::at_nu`] there, or the axis's end, 0 or 1, where `ν*` is the end's `ν`; latent,
/// [`Coord::Unplaced`]. Each read [`bounded`].
#[inline]
pub fn curve_axis<C, R>(chart: &C, e: &Encoded<R>, tol: R) -> (Coord<R>, CurveEncoded<R>)
where
    C: CurveChart,
    R: LinkReal + IeeeBits + From<C::Param>,
{
    let c = project(e, chart.nu_span::<R>(), tol);
    let [start, end] = chart.nu_ends::<R>();
    let coord = match c.landing {
        CurveLanding::OnCurve { .. } => bounded(chart.at_closed(&c.closed)),
        CurveLanding::Projected { nu } => bounded(select(
            nu == start,
            R::zero(),
            select(nu == end, R::one(), chart.at_nu(nu)),
        )),
        CurveLanding::Latent => Coord::Unplaced,
    };
    (coord, c)
}

/// `(x − lo)/(hi − lo)`, the linear axis `lerp(lo, hi, t)` inverted.
#[inline]
fn unlerp<R: Float>(lo: R, hi: R, x: R) -> R {
    (x - lo) / (hi - lo)
}

/// `(ν, K)`: `ν` by [`curve_axis`], `K` by the warp's inverse (`by_kind::warp_inverse`); `L_z = 0` is
/// `by_kind::encode_into_within`'s comparison. [`ByKind::coords`] is `tol = 0`.
impl<P: Copy> ByKind for Burrau<P>
where
    f64: From<P>,
{
    /// `tol = 0`: rounding alone puts `d > 0` on the curve, so this places essentially no state. A caller on this chart
    /// uses [`ByKind::coords_within`] at REQ-ENC-026's tolerance (inverse_encode_contract Part 6, step 4).
    fn coords<S, R>(&self, e: &Encoded<R>) -> [Coord<R>; 2]
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<P>,
    {
        self.coords_within::<S, R>(e, R::zero()).0
    }

    fn coords_within<S, R>(
        &self,
        e: &Encoded<R>,
        tol: R,
    ) -> ([Coord<R>; 2], Option<CurveEncoded<R>>)
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<P>,
    {
        let wide = <R as From<P>>::from;
        let (_, t) = warp_inverse(
            wide(self.k_max),
            wide(self.gamma_k),
            &invariants(&e.canonical.ic),
        );
        let (nu, curve) = curve_axis(self, e, tol);
        ([nu, Coord::At(t)], Some(curve))
    }
}

impl<P: Copy> CurveChart for Burrau<P>
where
    f64: From<P>,
{
    fn nu_ends<R: LinkReal + From<P>>(&self) -> [R; 2] {
        [self.nu(R::zero()), self.nu(R::one())]
    }

    fn at_nu<R: LinkReal + From<P>>(&self, nu: R) -> R {
        let wide = <R as From<P>>::from;
        unlerp(wide(self.nu_range[0]), wide(self.nu_range[1]), nu)
    }
}

/// The Euclid plane: `m` an annotation, `ν` by [`curve_axis`]; the rest start is the comparison's. [`ByKind::coords`]
/// is `tol = 0`.
impl<P: Copy> ByKind for EuclidPlane<P>
where
    f64: From<P>,
{
    /// `tol = 0`: rounding alone puts `d > 0` on the curve, so this places essentially no state. A caller on this chart
    /// uses [`ByKind::coords_within`] at REQ-ENC-026's tolerance (inverse_encode_contract Part 6, step 4).
    fn coords<S, R>(&self, e: &Encoded<R>) -> [Coord<R>; 2]
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<P>,
    {
        self.coords_within::<S, R>(e, R::zero()).0
    }

    fn coords_within<S, R>(
        &self,
        e: &Encoded<R>,
        tol: R,
    ) -> ([Coord<R>; 2], Option<CurveEncoded<R>>)
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<P>,
    {
        let (nu, curve) = curve_axis(self, e, tol);
        ([Coord::Annotation, nu], Some(curve))
    }
}

impl<P: Copy> CurveChart for EuclidPlane<P>
where
    f64: From<P>,
{
    fn nu_ends<R: LinkReal + From<P>>(&self) -> [R; 2] {
        [self.nu(R::zero()), self.nu(R::one())]
    }

    fn at_nu<R: LinkReal + From<P>>(&self, nu: R) -> R {
        let wide = <R as From<P>>::from;
        unlerp(wide(self.nu_range[0]), wide(self.nu_range[1]), nu)
    }
}

/// `θ`'s coordinate on `theta_range` at `θ`.
#[inline]
fn theta_coordinate<R: Float + From<P>, P: Copy>(theta_range: [P; 2], theta: R) -> R {
    let wide = <R as From<P>>::from;
    unlerp(wide(theta_range[0]), wide(theta_range[1]), theta)
}

/// `Φ_θ,K`: `θ` by [`curve_axis`], on the curve the closed form's `θ` read directly, not through `ν`; `K` by the
/// warp's inverse; `L_z = 0` is the comparison's. [`ByKind::coords`] is `tol = 0`.
impl<P: Copy> ByKind for BurrauThetaK<P>
where
    f64: From<P>,
{
    /// `tol = 0`: rounding alone puts `d > 0` on the curve, so this places essentially no state. A caller on this chart
    /// uses [`ByKind::coords_within`] at REQ-ENC-026's tolerance (inverse_encode_contract Part 6, step 4).
    fn coords<S, R>(&self, e: &Encoded<R>) -> [Coord<R>; 2]
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<P>,
    {
        self.coords_within::<S, R>(e, R::zero()).0
    }

    fn coords_within<S, R>(
        &self,
        e: &Encoded<R>,
        tol: R,
    ) -> ([Coord<R>; 2], Option<CurveEncoded<R>>)
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<P>,
    {
        let wide = <R as From<P>>::from;
        let (_, t) = warp_inverse(
            wide(self.k_max),
            wide(self.gamma_k),
            &invariants(&e.canonical.ic),
        );
        let (theta, curve) = curve_axis(self, e, tol);
        ([theta, Coord::At(t)], Some(curve))
    }
}

impl<P: Copy> CurveChart for BurrauThetaK<P>
where
    f64: From<P>,
{
    fn nu_ends<R: LinkReal + From<P>>(&self) -> [R; 2] {
        [self.nu(R::zero()), self.nu(R::one())]
    }

    fn at_nu<R: LinkReal + From<P>>(&self, nu: R) -> R {
        theta_coordinate(self.theta_range, crate::chart::burrau_acute::theta_of(nu))
    }

    fn at_closed<R: LinkReal + From<P>>(&self, c: &ClosedForm<R>) -> R {
        theta_coordinate(self.theta_range, c.theta)
    }
}

/// `Φ_θ,L_z`: `θ` by [`curve_axis`], on the curve the closed form's `θ` read directly; `L_z` by the band's inverse
/// `v = (L_z/(λ_L·L_max(K)) + 1)/2`, `v = ½` where the band's half-width is 0; the fixed `K` is the comparison's.
/// [`ByKind::coords`] is `tol = 0`.
impl<P: Copy> ByKind for BurrauThetaLz<P>
where
    f64: From<P>,
{
    /// `tol = 0`: rounding alone puts `d > 0` on the curve, so this places essentially no state. A caller on this chart
    /// uses [`ByKind::coords_within`] at REQ-ENC-026's tolerance (inverse_encode_contract Part 6, step 4).
    fn coords<S, R>(&self, e: &Encoded<R>) -> [Coord<R>; 2]
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<P>,
    {
        self.coords_within::<S, R>(e, R::zero()).0
    }

    fn coords_within<S, R>(
        &self,
        e: &Encoded<R>,
        tol: R,
    ) -> ([Coord<R>; 2], Option<CurveEncoded<R>>)
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<P>,
    {
        let half = R::one() / (R::one() + R::one());
        let width = self.lz_max::<R>();
        let flat = width == R::zero();
        let lz = invariants(&e.canonical.ic).lz;
        let ratio = select(flat, R::zero(), lz / select(flat, R::one(), width));
        let (theta, curve) = curve_axis(self, e, tol);
        ([theta, Coord::At((ratio + R::one()) * half)], Some(curve))
    }
}

impl<P: Copy> CurveChart for BurrauThetaLz<P>
where
    f64: From<P>,
{
    fn nu_ends<R: LinkReal + From<P>>(&self) -> [R; 2] {
        [self.nu(R::zero()), self.nu(R::one())]
    }

    fn at_nu<R: LinkReal + From<P>>(&self, nu: R) -> R {
        theta_coordinate(self.theta_range, crate::chart::burrau_acute::theta_of(nu))
    }

    fn at_closed<R: LinkReal + From<P>>(&self, c: &ClosedForm<R>) -> R {
        theta_coordinate(self.theta_range, c.theta)
    }
}

/// The δm strip: `θ` by the closed form, which reads the triangle alone; `v` the blend's least squares,
/// `v = (m − m_B)·(m_t − m_B)/‖m_t − m_B‖²`, `m_B` the Burrau masses at the closed form's `ν`, `v = 0` where the target
/// is the Burrau masses (`‖m_t − m_B‖² = 0`). The rest start is the comparison's.
impl<P: Copy> ByKind for BurrauThetaDeltaM<P>
where
    f64: From<P>,
{
    fn coords<S, R>(&self, e: &Encoded<R>) -> [Coord<R>; 2]
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<P>,
    {
        let wide = <R as From<P>>::from;
        let cf = closed_form(&e.canonical.ic);
        let mb = embed(cf.nu).m;
        let m = e.canonical.ic.m;
        let t = self.m_target.map(wide);
        let dt = [t[0] - mb[0], t[1] - mb[1], t[2] - mb[2]];
        let dm = [m[0] - mb[0], m[1] - mb[1], m[2] - mb[2]];
        let dot =
            |x: [R; 3], y: [R; 3]| x[0].product(y[0]) + x[1].product(y[1]) + x[2].product(y[2]);
        let span2 = sq(dt[0]) + sq(dt[1]) + sq(dt[2]);
        let same = span2 == R::zero();
        let v = select(same, R::zero(), dot(dm, dt) / select(same, R::one(), span2));
        [
            bounded(theta_coordinate(self.theta_range, cf.theta)),
            Coord::At(v),
        ]
    }
}
