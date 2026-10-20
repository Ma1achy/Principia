//! Encode into the active chart, by axis kind (inverse_encode_contract Parts 5 and 6; dd_encode §3.1, §3.2 steps 4–6;
//! REQ-ENC-008, REQ-ENC-009, REQ-ENC-010, REQ-CHART-005): `E(x)` ([`super::encode`]) puts `x` on the canonical section
//! and gives its latent `z`, always available. [`encode_into`] then reads the active chart's point `c(x)` off `C(x)`,
//! axis by axis, and lands on the chart where the chart represents `x`, otherwise in latent `z`.
//!
//! **By kind** ([`ByKind`], one impl per chart type; chart_decoder_contract § "The four axis kinds"):
//! - **Kind 1, raw latent** ([`component`], [`plane`]): the axis's component of `E(x)`'s `z`, exact. On the latent chart
//!   the two directions `q₁`, `q₂` need not be orthogonal, so the components are the solution of the plane's 2×2 Gram
//!   system, which for `q = ê_k` is `z_k − z₀,k` itself.
//! - **Kind 2, derived in block** ([`derived`], [`Residual`]): the block inverse's own value (`α`, `β`, a Jacobi
//!   momentum component), plus the axis's residual convention. A mixed chart's derived axis has no residual: the
//!   block's other control is frozen; the ternary mass plot's two axes pin the mass block together, and the shape
//!   sphere's the configuration block (the chart map's inverse). An `m₀` axis must declare one ([`HoldRatio`], [`HoldEqual`]): the convention is
//!   one function, [`Residual::masses`], which the forward axis ([`Residual::controls`]) and encode both call, so
//!   encoding `m₀ = 0.4` under two conventions gives two `z`, and each round trip closes only under its own.
//! - **Kind 3, invariant** ([`invariants`], [`fibre`]): state → chart coordinates evaluates `L_z(x)`, `E(x)` and `K(x)`
//!   directly, no solve; chart coordinates → latent runs the chart's own forward construction at the frozen
//!   configuration, exactly as a pixel does (`crate::chart::invariant`'s `InvariantBlocks`), and free-momentum-inverts
//!   the constructed `p` through the registry's momentum links (`super::blocks::invert`). Encode reuses decode, so an
//!   entered `(L_z, E)` and the clicked pixel land on one `z`; smallest-latent-norm is not used, there being a
//!   construction.
//! - **Annotation** ([`Coord::Annotation`]): nothing to invert; the axis gives no coordinate and nothing is compared
//!   for it.
//! - **Kind 4, coupled curve**: the Burrau family's encode, its closed form and off-curve projection, is
//!   TASK-M2-18's; no impl here.
//!
//! **Lowered, never interpreted** (chart_decoder_contract design axiom 4; lowering Part 2): the kind is each chart
//! type's impl, monomorphised, as the decode is; the kernel holds no runtime axis-kind value. The authoring layer's
//! `AxisKind` (`engine::chart_authoring`) describes the same axes, CPU-side.
//!
//! **Where possible** (inverse_encode_contract Part 6, step 4's definition, R-72): `x` lands on the chart iff
//! `D(Φ(c(x)))`, the chart's decode at `c(x)` ([`crate::chart::ChartDecode::decode_at`]), reproduces `C(x)`: every
//! mass, position and momentum component within the caller's tolerance ([`residual`]), both states canonical, `M = I =
//! 1`. A coordinate that is not finite, or an axis that cannot place the state ([`Coord::Unplaced`]), lands it in
//! latent `z` with no comparison.
//!
//! **Feasibility first** ([`feasibility`]; Part 5 kind 3, Part 6 "Chart-aware validation"; REQ-ENC-010, REQ-ENC-030).
//! An entered pair is tested before the construction, by the construction's own exact test `2·I·K* < L_z²`
//! (`crate::decode::invariant::infeasible`, `I = 1`). An infeasible pair is projected by the contract's metric: `K*`
//! (so `E`) held and `L_z` moved onto the parabola on its own side, `L_z ← sign(L_z)·L_max(K*)`, `L_max` the chart's own
//! (`crate::chart::invariant::l_max`), held so that `L_z² ≤ 2K*` exactly; where `K* < 0`, to the apex `(0, 0)`, the rest
//! start. It is rejected where that changes the class of `sign(L_z)`: `L_z ≠ 0` with `K* ≤ 0`. The contract's other
//! class, `sign(E)`, is kept by every such projection (`E` is held, or raised from below `U` to `U < 0`), so it is not
//! tested. A non-finite entry is rejected, never carried as a NaN. The constraint named is the chart's
//! ([`InvariantAxis`]), as its `validate` names it. Layer 1, the hypercube, is the momentum links' clamp
//! (`lookup_clamped`, [`Lookup::Encoded`]'s `clamped`).
//!
//! **DEGENERATE** (Part 5 kind 3; R-82). Where no seed qualifies, the fibre returns [`Lookup::Degenerate`], the
//! construction's `Outcome::Degenerate`, the label the pixel path gives. On a configuration of the canonical-frame
//! decode the seed rule never degenerates (`crate::decode::invariant`'s note: the largest norm is at least ½), so an
//! invariant chart's frozen configuration never gives it; a configuration from elsewhere takes the rule on its computed
//! norms ([`SeedRule::ByNorms`], the decoder's `construct_by_norms`), where it can.

use spirv_std::num_traits::{Float, FloatConst};

use super::{encode, Encoded, SimplexClamp};
use crate::chart::invariant::{l_max, InvariantChart, InvariantLE, InvariantLK, WarpReal};
use crate::chart::mixed::{Coordinate, Quantity};
use crate::chart::{ChartDecode, Constraint, Latent, MassSimplex, Mixed, ShapeMap, ShapeSphere};
use crate::decode::config::{self, Config};
use crate::decode::invariant::{construct, construct_by_norms, infeasible, Outcome};
use crate::decode::mass::{self, Masses};
use crate::decode::{descriptor, Ic};
use crate::generated::links::{Link, LinkReal, Selection};
use crate::payload::PayloadReal;
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

// ── The dispatch ───────────────────────────────────────────────────────────────────────────────────────────────────

/// One axis's chart coordinate, read off `C(x)` by its kind.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Coord<R> {
    /// The axis's chart coordinate, `u` or `v`.
    At(R),
    /// An annotation axis: no IC content, nothing to invert.
    Annotation,
    /// The axis cannot place the state: a mass coordinate as a derived quantity, or a latent plane whose directions are
    /// dependent.
    Unplaced,
}

/// Where encode lands (Part 6, step 4).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Landing<R> {
    /// On the active chart, at `(u, v)`; an annotation axis's coordinate is 0, unread.
    Chart { u: R, v: R },
    /// In latent `z`: the chart does not represent the state.
    Latent,
}

/// A chart whose axes encode inverts by kind (the module's note).
pub trait ByKind: ChartDecode {
    /// The chart coordinates of the encoded state `e`, each axis by its kind.
    fn coords<S, R>(&self, e: &Encoded<R>) -> [Coord<R>; 2]
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<Self::Param>;
}

/// Encode into the active chart: `E(x)`, the chart point read off it by kind, and where it lands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChartEncoded<R> {
    /// `E(x)`: the latent `z`, which represents every state, `C(x)` and the discard report.
    pub encoded: Encoded<R>,
    /// The chart coordinates, by kind.
    pub coords: [Coord<R>; 2],
    /// [`residual`] of the chart's decode at the coordinates against `C(x)`; `+∞` where no point was placed.
    pub residual: R,
    /// The chart point, or latent `z`.
    pub landing: Landing<R>,
}

/// The largest difference of any mass, position or momentum component of `a` and `b`; NaN where any is NaN.
#[inline]
pub fn residual<R: LinkReal>(a: &Ic<R>, b: &Ic<R>) -> R {
    let mut worst = R::zero();
    let mut take = |x: R, y: R| {
        let d = Float::abs(x - y);
        worst = select(Float::is_nan(worst) | (d <= worst), worst, d);
    };
    for i in 0..3 {
        take(a.m[i], b.m[i]);
        for k in 0..2 {
            take(a.r[i][k], b.r[i][k]);
            take(a.p[i][k], b.p[i][k]);
        }
    }
    worst
}

/// The chart coordinate an axis's [`Coord`] places: `Some` for a finite coordinate, and 0 for an annotation axis, which
/// the decode does not read; `None` where the axis places nothing.
#[inline]
fn placed<R: LinkReal>(c: Coord<R>) -> Option<R> {
    match c {
        Coord::At(t) if Float::is_finite(t) => Some(t),
        Coord::Annotation => Some(R::zero()),
        _ => None,
    }
}

/// Encode `ic` into `chart` under the selection `S` (the module's note): `E(ic)`, the chart point by kind, and the chart
/// as the landing where its decode there is within `tol` of `C(ic)` in every component ([`residual`]), else latent `z`.
#[inline]
pub fn encode_into<S, R, C>(chart: &C, ic: &Ic<R>, tol: R) -> ChartEncoded<R>
where
    S: Selection,
    S::MassLink: SimplexClamp,
    R: LinkReal + PayloadReal + From<C::Param>,
    C: ByKind,
{
    let encoded = encode::<S, R>(ic);
    let coords = chart.coords::<S, R>(&encoded);
    let (residual, landing) = match (placed(coords[0]), placed(coords[1])) {
        (Some(u), Some(v)) => {
            let d = residual(&chart.decode_at::<S, R>(u, v).ic, &encoded.canonical.ic);
            (
                d,
                select(d <= tol, Landing::Chart { u, v }, Landing::Latent),
            )
        }
        _ => (R::infinity(), Landing::Latent),
    };
    ChartEncoded {
        encoded,
        coords,
        residual,
        landing,
    }
}

// ── Kind 1: raw latent ─────────────────────────────────────────────────────────────────────────────────────────────

/// `z`'s component at the coordinate `k`, read by a literal index (gpu_determinism_note rule 5).
#[inline]
pub fn component<R: Copy>(z: &[R; 8], k: Coordinate) -> R {
    match k {
        Coordinate::ZAlpha => z[0],
        Coordinate::ZBeta => z[1],
        Coordinate::ZQ0 => z[2],
        Coordinate::ZQ1 => z[3],
        Coordinate::ZQ2 => z[4],
        Coordinate::ZQ3 => z[5],
        Coordinate::ZMu1 => z[6],
        Coordinate::ZMu2 => z[7],
    }
}

/// `⟨a, b⟩` over the eight components.
#[inline]
fn dot<R: LinkReal>(a: &[R; 8], b: &[R; 8]) -> R {
    let mut s = R::zero();
    for k in 0..8 {
        s = s + a[k] * b[k];
    }
    s
}

/// The components `(a, b)` of `d` in the plane of `q1`, `q2`: the least-squares solution of `d ≈ a·q₁ + b·q₂`, the
/// Gram system `[⟨q₁,q₁⟩ ⟨q₁,q₂⟩; ⟨q₁,q₂⟩ ⟨q₂,q₂⟩]·(a, b) = (⟨q₁,d⟩, ⟨q₂,d⟩)`, which is `d`'s own components for
/// `q = ê_i, ê_j`; `None` where the directions are dependent (a zero Gram determinant).
#[inline]
pub fn plane<R: LinkReal>(d: &[R; 8], q1: &[R; 8], q2: &[R; 8]) -> Option<(R, R)> {
    let (g11, g12, g22) = (dot(q1, q1), dot(q1, q2), dot(q2, q2));
    let (r1, r2) = (dot(q1, d), dot(q2, d));
    let det = g11 * g22 - g12 * g12;
    select(
        det != R::zero(),
        Some(((g22 * r1 - g12 * r2) / det, (g11 * r2 - g12 * r1) / det)),
        None,
    )
}

/// The latent chart: both axes kind 1, `(u, v) = ((a + 1)/2, (b + 1)/2)` from `z − z₀ = a·q₁ + b·q₂`.
impl<P: Copy> ByKind for Latent<P> {
    fn coords<S, R>(&self, e: &Encoded<R>) -> [Coord<R>; 2]
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<P>,
    {
        let wide = |q: [P; 8]| q.map(<R as From<P>>::from);
        let z0 = wide(self.z0);
        let z = e.z.z;
        let mut d = [R::zero(); 8];
        for k in 0..8 {
            d[k] = z[k] - z0[k];
        }
        let half = R::one() / (R::one() + R::one());
        match plane(&d, &wide(self.q1), &wide(self.q2)) {
            Some((a, b)) => [
                Coord::At((a + R::one()) * half),
                Coord::At((b + R::one()) * half),
            ],
            None => [Coord::Unplaced, Coord::Unplaced],
        }
    }
}

// ── Kind 2: derived in block ───────────────────────────────────────────────────────────────────────────────────────

/// The block inverse's own value of the quantity at the coordinate `k` (Part 5, kind 2): `α` or `β` for the
/// configuration controls, the Jacobi momentum component `q₀ … q₃` for the momentum controls; `None` for a mass
/// control, the mass block having no single-mass quantity of its own (a mixed chart's mass axis is a latent control,
/// `crate::chart::mixed`'s note; an `m₀` axis takes a [`Residual`]).
#[inline]
pub fn derived<R: Copy>(e: &Encoded<R>, k: Coordinate) -> Option<R> {
    let j = &e.blocks.jacobi;
    match k {
        Coordinate::ZAlpha => Some(e.blocks.alpha),
        Coordinate::ZBeta => Some(e.blocks.beta),
        Coordinate::ZQ0 => Some(j.p_rho[0]),
        Coordinate::ZQ1 => Some(j.p_rho[1]),
        Coordinate::ZQ2 => Some(j.p_lambda[0]),
        Coordinate::ZQ3 => Some(j.p_lambda[1]),
        Coordinate::ZMu1 | Coordinate::ZMu2 => None,
    }
}

/// A mixed chart's axis `Q` on `range` (`[lo, hi]` at `t = 0, 1`): kind 1 reads `z`'s component, kind 2 the block
/// inverse's value, and `t = (x − lo)/(hi − lo)` inverts the axis's affine warp.
#[inline]
fn mixed_axis<Q: Quantity, R: LinkReal>(e: &Encoded<R>, range: [R; 2]) -> Coord<R> {
    let x = select(
        Q::DERIVED,
        derived(e, Q::COORDINATE),
        Some(component(&e.z.z, Q::COORDINATE)),
    );
    match x {
        Some(x) => Coord::At((x - range[0]) / (range[1] - range[0])),
        None => Coord::Unplaced,
    }
}

/// A mixed chart: each axis by its own kind, the other six controls frozen (kind 2's residual: none).
impl<H: Quantity, V: Quantity, P: Copy> ByKind for Mixed<H, V, P>
where
    f64: From<P>,
{
    fn coords<S, R>(&self, e: &Encoded<R>) -> [Coord<R>; 2]
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<P>,
    {
        let wide = |r: [P; 2]| r.map(<R as From<P>>::from);
        [
            mixed_axis::<H, R>(e, wide(self.h_range)),
            mixed_axis::<V, R>(e, wide(self.v_range)),
        ]
    }
}

/// A kind-2 `m₀` axis's residual convention (chart_decoder_contract § "The four axis kinds", "Residual conventions"):
/// how the mass block's other DOF is pinned, declared once and used both ways.
pub trait Residual {
    /// The convention, in words: the axis's label.
    const STATEMENT: &'static str;

    /// The masses at `m₀ = m0`, the leftover DOF pinned from `held`, the slice's masses.
    fn masses<R: LinkReal>(m0: R, held: [R; 3]) -> [R; 3];

    /// The mass controls of [`Residual::masses`] under the selection `S`: the mass link's inverse, the forward axis's
    /// latent point and encode's alike. A mass that is not positive is guarded as the block inverse guards it
    /// (`super::blocks::guard`).
    #[inline]
    fn controls<S: Selection, R: LinkReal>(m0: R, held: [R; 3]) -> [R; 2] {
        let m = super::blocks::guard(Self::masses(m0, held));
        <S::MassLink as Link<2, 3>>::inverse(m)
    }
}

/// Hold the `m₁:m₂` ratio at the slice's: `m₁ = (1 − m₀)·m₁*/(m₁* + m₂*)`, `m₂ = (1 − m₀) − m₁`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HoldRatio;

/// Hold `m₁ = m₂`: both `(1 − m₀)/2`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HoldEqual;

impl Residual for HoldRatio {
    const STATEMENT: &'static str = "hold the m₁:m₂ ratio";

    #[inline]
    fn masses<R: LinkReal>(m0: R, held: [R; 3]) -> [R; 3] {
        let rest = R::one() - m0;
        let m1 = rest * (held[1] / (held[1] + held[2]));
        [m0, m1, rest - m1]
    }
}

impl Residual for HoldEqual {
    const STATEMENT: &'static str = "hold m₁ = m₂";

    #[inline]
    fn masses<R: LinkReal>(m0: R, _held: [R; 3]) -> [R; 3] {
        let half = (R::one() - m0) / (R::one() + R::one());
        [m0, half, half]
    }
}

/// An `m₀` axis's value read off the encoded state: `C(x)`'s `m₀`, the mass block's own quantity. Whether `x` lies on
/// the axis's convention is [`encode_into`]'s comparison of the decode at it.
#[inline]
pub fn m0<R: Copy>(e: &Encoded<R>) -> R {
    e.canonical.ic.m[0]
}

/// The ternary mass plot: both axes kind 2 in the mass block, together pinning it, so no residual. The buffer is undone,
/// `m′ = (m − ε_m)/(1 − 3ε_m)`, and the shear inverted, `u = m′₁`, `v = m′₂/(1 − u)`, as `crate::chart::mass_simplex`
/// forms both; `v = 0` on the edge `u = 1`, where the decode does not read it. The fixed triangle and the rest start are
/// [`encode_into`]'s comparison.
impl<P: Copy> ByKind for MassSimplex<P>
where
    f64: From<P>,
{
    fn coords<S, R>(&self, e: &Encoded<R>) -> [Coord<R>; 2]
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<P>,
    {
        let eps = <R as From<P>>::from(self.eps_m);
        let k = R::one() - (eps + eps + eps);
        let m = e.canonical.ic.m;
        let u = (m[1] - eps) / k;
        let rest = R::one() - u;
        let v = select(rest == R::zero(), R::zero(), ((m[2] - eps) / k) / rest);
        [Coord::At(u), Coord::At(v)]
    }
}

/// `⟨a, b⟩` in three dimensions.
#[inline]
fn dot3<R: LinkReal>(a: [R; 3], b: [R; 3]) -> R {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// The spherical map inverted at the unit point `n`, `pi` the float type's π: `θ = atan2(n₁, n₀)` taken into
/// `[0, 2π)`, `φ = atan2(√(n₀² + n₁²), n₂)`, `(s, t) = (θ/2π, 1 − φ/π)` (chart_reference §3.3). At the poles, `n₀ = n₁
/// = 0`, `θ` is 0, selected, never `atan2(0, 0)` (R-437).
#[inline]
pub fn spherical_inverse<R: LinkReal>(n: [R; 3], pi: R) -> (R, R) {
    let zero = R::zero();
    let turn = pi + pi;
    let pole = (n[0] == zero) & (n[1] == zero);
    let theta = select(pole, zero, Float::atan2(n[1], n[0]));
    let theta = select(theta < zero, theta + turn, theta);
    let phi = Float::atan2(Float::sqrt(n[0] * n[0] + n[1] * n[1]), n[2]);
    (theta / turn, R::one() - phi / pi)
}

/// The exponential map inverted at the unit point `n` about the frame `(n0, e1, e2)` with extent `scale`: the log map,
/// `r = atan2(‖n⊥‖, n·n₀)`, `n⊥ = n − (n·n₀)·n₀`, `d = (r/‖n⊥‖)·n⊥`, then `u = (d·e₁/s + 1)/2`, `v = (d·e₂/s + 1)/2`
/// (chart_reference §3.3). At `n⊥ = 0`, `d = 0`, selected: `n₀` itself, and at the antipode, which the map reaches only
/// past its extent, the centre too, which [`encode_into`]'s comparison then refuses.
#[inline]
pub fn exponential_inverse<R: LinkReal>(
    n: [R; 3],
    (n0, e1, e2): ([R; 3], [R; 3], [R; 3]),
    scale: R,
) -> (R, R) {
    let c = dot3(n, n0);
    let perp = [n[0] - c * n0[0], n[1] - c * n0[1], n[2] - c * n0[2]];
    let sin_r = Float::sqrt(dot3(perp, perp));
    let k = select(
        sin_r == R::zero(),
        R::zero(),
        Float::atan2(sin_r, c) / sin_r,
    );
    let d = [k * perp[0], k * perp[1], k * perp[2]];
    let half = R::one() / (R::one() + R::one());
    (
        (dot3(d, e1) / scale + R::one()) * half,
        (dot3(d, e2) / scale + R::one()) * half,
    )
}

/// The shape sphere: both axes kind 2 in the configuration block, together pinning its two DOF, so no residual. The
/// point is `n = shape_vec(ρ̃, λ̃)` of `C(x)` (`crate::shape`), on the upper hemisphere, `C` having mirrored the lower
/// onto it (R-141), through the chart map's inverse. The held masses and momenta are [`encode_into`]'s comparison.
impl<P: Copy + FloatConst> ByKind for ShapeSphere<P> {
    fn coords<S, R>(&self, e: &Encoded<R>) -> [Coord<R>; 2]
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<P>,
    {
        let wide = <R as From<P>>::from;
        let j = &e.blocks.jacobi;
        let n = shape_vec(j.rho_t, j.lambda_t);
        let (u, v) = match self.map {
            ShapeMap::Spherical => spherical_inverse(n, wide(P::PI())),
            ShapeMap::Exponential => exponential_inverse(
                n,
                (self.n0.map(wide), self.e1.map(wide), self.e2.map(wide)),
                wide(self.scale),
            ),
        };
        [Coord::At(u), Coord::At(v)]
    }
}

// ── Kind 3: invariant ──────────────────────────────────────────────────────────────────────────────────────────────

/// The invariants of a state, evaluated directly (Part 5, kind 3 (a)).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Invariants<R> {
    /// `L_z = Σ rᵢ × pᵢ`.
    pub lz: R,
    /// `K = Σ ‖pᵢ‖²/2mᵢ`.
    pub k: R,
    /// `U`, the potential.
    pub u: R,
    /// `E = K + U`.
    pub e: R,
}

/// `L_z(x)`, `K(x)`, `U(x)` and `E(x)` of `ic`: no solve.
#[inline]
pub fn invariants<R: LinkReal + PayloadReal>(ic: &Ic<R>) -> Invariants<R> {
    let cross = |r: [R; 2], p: [R; 2]| r[0] * p[1] - r[1] * p[0];
    let lz = cross(ic.r[0], ic.p[0]) + cross(ic.r[1], ic.p[1]) + cross(ic.r[2], ic.p[2]);
    let k = descriptor::kinetic(ic);
    let u = descriptor::potential(ic);
    Invariants { lz, k, u, e: k + u }
}

/// An invariant chart's vertical axis (inverse_encode_contract § "Chart-aware validation", layer 2): the kinetic
/// energy its entered value gives at the frozen potential, and the constraints its `validate` names.
pub trait InvariantAxis: InvariantChart {
    /// The constraint `K* < 0` fails.
    const LOW: Constraint;
    /// The constraint `K* < L_z²/2I` fails.
    const WIDE: Constraint;
    /// `K*` of the vertical value `y` at the frozen configuration's potential `u_pot`.
    fn kinetic<R: LinkReal>(y: R, u_pot: R) -> R;
}

/// `(L_z, E)`: `K* = E − U`; one constraint, the parabola.
impl<P: Copy> InvariantAxis for InvariantLE<P> {
    const LOW: Constraint = Constraint::LzEParabola;
    const WIDE: Constraint = Constraint::LzEParabola;

    #[inline]
    fn kinetic<R: LinkReal>(y: R, u_pot: R) -> R {
        y - u_pot
    }
}

/// `(L_z, K)`: `K* = K`; `K ≥ 0` and `K ≥ L_z²/2I`.
impl<P: Copy> InvariantAxis for InvariantLK<P> {
    const LOW: Constraint = Constraint::KNonNegative;
    const WIDE: Constraint = Constraint::KAboveLzBound;

    #[inline]
    fn kinetic<R: LinkReal>(y: R, _u_pot: R) -> R {
        y
    }
}

/// The chart point of the invariants `inv` on an invariant chart: `t = (K/K_max)^{1/γ_K}`, the warp's `K(t)` inverted,
/// and `s = (L_z/L_max + 1)/2`, `L_max = L_max(K)`, `s = ½` where `L_max = 0` (chart_reference §2.1's warp).
#[inline]
pub fn warp_inverse<R: WarpReal>(k_max: R, gamma_k: R, inv: &Invariants<R>) -> (R, R) {
    let half = R::one() / (R::one() + R::one());
    let l = l_max(inv.k);
    let ratio = select(l == R::zero(), R::zero(), inv.lz / l);
    let s = (ratio + R::one()) * half;
    let t = Float::powf(inv.k / k_max, R::one() / gamma_k);
    (s, t)
}

/// Both invariant charts: the horizontal `L_z`, the vertical `E` or `K`, kind 3 (a), through the warp's inverse.
macro_rules! invariant_by_kind {
    ($chart:ident) => {
        impl<P: Copy> ByKind for $chart<P> {
            fn coords<S, R>(&self, e: &Encoded<R>) -> [Coord<R>; 2]
            where
                S: Selection,
                R: LinkReal + PayloadReal + From<P>,
            {
                let frozen = self.invariant();
                let wide = <R as From<P>>::from;
                let inv = invariants(&e.canonical.ic);
                let (s, t) = warp_inverse(wide(frozen.k_max), wide(frozen.gamma_k), &inv);
                [Coord::At(s), Coord::At(t)]
            }
        }
    };
}

invariant_by_kind!(InvariantLE);
invariant_by_kind!(InvariantLK);

/// What the feasibility test did with an entered pair (the module's note).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Feasibility<R> {
    /// Feasible as entered.
    Valid,
    /// Infeasible; `(lz, k)` is the nearest valid point, and `failed` the constraint the entry failed.
    Projected { lz: R, k: R, failed: Constraint },
    /// Refused: the projection would change `L_z`'s class, or an entry is not finite.
    Rejected { failed: Constraint },
}

/// The feasibility test and its remedy for the pair `(lz, k)`, `K* = k`, `I = 1` (the module's note): `low` names
/// `K* < 0`, `wide` names `K* < L_z²/2`.
#[inline]
pub fn feasibility<R: WarpReal>(lz: R, k: R, low: Constraint, wide: Constraint) -> Feasibility<R> {
    let zero = R::zero();
    let finite = Float::is_finite(lz) & Float::is_finite(k);
    let short = finite && infeasible(R::one(), k, lz);
    let k_c = select(k > zero, k, zero);
    let l = l_max(k_c);
    let lz_c = select(lz < zero, -l, l);
    // `l > 0` exactly where `K* > 0`, so the projection keeps `L_z`'s class iff `L_z = 0` or `K* > 0`.
    let keeps = (lz == zero) | (k > zero);
    let failed = select(k < zero, low, wide);
    match (finite, short, keeps) {
        (false, _, _) => Feasibility::Rejected {
            failed: select(Float::is_finite(k), wide, low),
        },
        (true, false, _) => Feasibility::Valid,
        (true, true, true) => Feasibility::Projected {
            lz: lz_c,
            k: k_c,
            failed,
        },
        (true, true, false) => Feasibility::Rejected { failed },
    }
}

/// How the fibre's construction takes its seed (R-82; `crate::decode::invariant`'s note).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SeedRule<R> {
    /// A configuration of the canonical-frame decode: the seed decided on its control `z_α`, as the pixel's.
    Decoded { z_alpha: R },
    /// A configuration from elsewhere: the rule on the computed norms, `DEGENERATE` where none qualifies.
    ByNorms,
}

/// An entered invariant pair, encoded (Part 5, kind 3 (b)).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Lookup<R> {
    /// The latent point: the frozen controls with the free-momentum inverse of the constructed `p`.
    Encoded {
        /// `z`, in the latent block layout. From [`fibre_at`], only the momentum block `z[2:6]` is set, the rest 0.
        z: [R; 8],
        /// The targets the construction ran at, after any projection.
        lz: R,
        /// `K*`.
        k: R,
        /// `Some(failed)` where the entry was projected, naming the constraint it failed.
        projected: Option<Constraint>,
        /// `lookup_clamped`'s bits: bit `k` set iff `z[k]`'s momentum link clamped (`k = 2 … 5`).
        clamped: u32,
    },
    /// No seed qualifies: the pixel path's `DEGENERATE` (R-82).
    Degenerate,
    /// Refused, naming the failed constraint.
    Rejected { failed: Constraint },
}

/// The fibre point of `(lz, k)`, `K* = k`, at `masses` and `config`, under `S` (the module's note): feasibility first,
/// with `low` and `wide` the constraints named; then the construction, its seed by `rule`; then the free-momentum
/// inverse of its `p`, in `z[2:6]`.
#[inline]
pub fn fibre_at<S, R>(
    masses: &Masses<R>,
    config: &Config<R>,
    rule: SeedRule<R>,
    (lz, k): (R, R),
    (low, wide): (Constraint, Constraint),
) -> Lookup<R>
where
    S: Selection,
    S::MassLink: SimplexClamp,
    R: LinkReal + PayloadReal,
{
    let (lz, k, projected) = match feasibility(lz, k, low, wide) {
        Feasibility::Valid => (lz, k, None),
        Feasibility::Projected { lz, k, failed } => (lz, k, Some(failed)),
        Feasibility::Rejected { failed } => return Lookup::Rejected { failed },
    };
    let c = match rule {
        SeedRule::Decoded { z_alpha } => construct(masses, config, z_alpha, lz, k),
        SeedRule::ByNorms => construct_by_norms(masses, config, lz, k),
    };
    match c.outcome {
        Outcome::Feasible => {
            let ic = Ic {
                m: masses.m,
                r: config.r,
                p: c.p,
            };
            let inverted = super::blocks::invert::<S, R>(&ic);
            let mut z = [R::zero(); 8];
            z[2..6].copy_from_slice(&inverted.z[2..6]);
            Lookup::Encoded {
                z,
                lz,
                k,
                projected,
                // The four momentum controls' bits, `z[2:6]`.
                clamped: inverted.clamped & (0b1111 << 2),
            }
        }
        Outcome::Degenerate => Lookup::Degenerate,
        // The feasibility test above is the construction's own, so a pair that passed it is feasible here.
        Outcome::Infeasible => Lookup::Rejected { failed: wide },
    }
}

/// An entered `(L_z, y)` on the invariant chart `chart`, `y` its vertical value (`E` or `K`), under `S`: the chart's
/// frozen masses and configuration decoded as the pixel's are, `K*` from `y` at their potential, then [`fibre_at`] with
/// the pixel's seed, decided on the frozen `z_α`; the frozen controls fill the rest of `z`.
#[inline]
pub fn fibre<S, R, C>(chart: &C, lz: R, y: R) -> Lookup<R>
where
    S: Selection,
    S::MassLink: SimplexClamp,
    R: LinkReal + PayloadReal + From<C::Param>,
    C: InvariantAxis,
{
    let frozen = chart.invariant().frozen::<R>().z;
    let masses = mass::decode::<S, R>([frozen[6], frozen[7]]);
    let z_cfg = [frozen[0], frozen[1]];
    let config = config::decode::<S, R>(z_cfg, &masses);
    let at_rest = Ic {
        m: masses.m,
        r: config.r,
        p: [[R::zero(); 2]; 3],
    };
    let k = C::kinetic(y, descriptor::potential(&at_rest));
    let rule = SeedRule::Decoded { z_alpha: z_cfg[0] };
    match fibre_at::<S, R>(&masses, &config, rule, (lz, k), (C::LOW, C::WIDE)) {
        Lookup::Encoded {
            z,
            lz,
            k,
            projected,
            clamped,
        } => {
            let mut placed = frozen;
            placed[2..6].copy_from_slice(&z[2..6]);
            Lookup::Encoded {
                z: placed,
                lz,
                k,
                projected,
                clamped,
            }
        }
        other => other,
    }
}
