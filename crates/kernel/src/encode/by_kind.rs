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
//! - **Kind 4, coupled curve** (`super::curve`): the Burrau charts' impls are there, beside the closed form, the
//!   off-curve projection and the fold. The curve axis's coordinate is kind 4's landing at the curve tolerance `tol`
//!   ([`ByKind::coords_within`]; REQ-ENC-026): the closed form's on the curve, `ν*`'s within `tol`, and none beyond
//!   it; the chart's other axis is read by its own kind.
//!
//! **Lowered, never interpreted** (chart_decoder_contract design axiom 4; lowering Part 2): the kind is each chart
//! type's impl, monomorphised, as the decode is; the kernel holds no runtime axis-kind value. The authoring layer's
//! `AxisKind` (`engine::chart_authoring`) describes the same axes, CPU-side.
//!
//! **Where possible** (inverse_encode_contract Part 6, step 4's definition, R-72): `x` lands on the chart iff
//! `‖D(Φ(c(x))) − C(x)‖_phys ≤ ε_phys`, `D(Φ(c(x)))` the chart's decode at `c(x)`
//! ([`crate::chart::ChartDecode::decode_at`]), both states canonical: Part 4's norm (REQ-ENC-029) and tolerance
//! (REQ-ENC-024). [`encode_into_within`] takes both from its caller, the lookup, and fixes neither, and with them kind
//! 4's curve tolerance `tol` (REQ-ENC-026), which only a curve chart reads; [`encode_into`] is it at `tol = 0`, where a
//! curve chart represents only a state its curve holds exactly, and rounding alone puts `d > 0` on the curve, so it
//! places essentially no state: a caller on a curve chart calls [`encode_into_within`] with REQ-ENC-026's tolerance
//! (inverse_encode_contract Part 6, step 4). A coordinate that is not finite, or an axis that
//! cannot place the state ([`Coord::Unplaced`]), lands it in latent `z` with no comparison. A curve chart's own report,
//! `d`, `ν*` and the fold's flag, is [`ChartEncoded::curve`].
//!
//! **The shape sphere's section** (R-544; inverse_encode_contract Parts 5 and 6, "On the shape sphere chart";
//! REQ-ENC-034). The shape sphere chart decodes with `C`'s step 3 not taken (`crate::canon::canonicalise_signed`), so
//! its states keep their signed shape, and its point and its comparison are read off that section, not `C(x)`'s: `C(x)`
//! with step 3's mirror undone on the full state, every `rᵢ` and `pᵢ`, where `lookup_mirrored` is set
//! ([`ByKind::section`]). So `x` and `Π·x` land at `(θ, φ)` and `(θ, π − φ)`, and a lower-hemisphere `x` whose masses
//! and unmirrored body-frame momenta are the slice's lands on the chart and decodes back to itself. `E(x)` itself, its
//! `z` and T1 over the gauge group with its `Z₂` mirror, is unchanged, and so is every other chart's point and
//! comparison, against `C(x)`.
//!
//! **The branch inputs** (R-34; gpu_determinism_note, "The discipline", rules 1, 2, 4 and 6). The kernel builds this
//! module to f32 SPIR-V, so every float that decides a branch or a select is built in a fixed form, every sum of
//! products an explicit `fma`, and no `±∞` or NaN literal is written. Each divisor a select guards is itself selected
//! as 1 there, so no `x/0` is formed (R-436's producer rule):
//! - **The landing**, in [`encode_into`]: the caller's norm against the caller's `ε_phys`, `d ≤ ε_phys`, so a NaN `d`
//!   lands in latent `z`; `placed`'s finiteness test on each coordinate.
//! - **`plane`'s Gram determinant**, in [`plane`]: `det = g₁₁g₂₂ − g₁₂²` against 0, in Kahan's form, `w = g₁₂·g₁₂`,
//!   `det = fma(g₁₁, g₂₂, −w) + fma(−g₁₂, g₁₂, w)`, which is exactly 0 wherever the rounded Gram entries satisfy
//!   `g₁₁g₂₂ = g₁₂²`; each `g` and the right-hand side is `dot`, a chain of explicit `fma`s from the lone first product.
//! - **The ternary plot's edge `u = 1`**, in [`MassSimplex`]'s impl: `v`'s divisor `k − (m₁ − ε_m)` against 0, lone
//!   subtractions, no division.
//! - **The pole and `θ`'s sign**, in [`spherical_inverse`]: `n₀`, `n₁` against 0, `θ`'s sign decided on `n₁`, never on
//!   `atan2`'s output; the pole's `θ` is selected as 0, as `crate::canon`'s rotation angle selects `φ` (R-437).
//! - **The signed point and section**, in [`ShapeSphere`]'s impl: `C`'s own `lookup_mirrored`, decided by
//!   `crate::canon::mirror_decision` in its fixed form, no float of this module's.
//! - **The centre**, in [`exponential_inverse`]: `‖n⊥‖²`, a chain of explicit `fma`s, against 0, not its `sqrt`.
//! - **The warp at rest**, in [`warp_inverse`]: `K` against 0, where `L_max(K) = 0`, not `L_max` itself.
//! - **Feasibility**, in [`feasibility`]: the finiteness of the entries; the construction's own exact test
//!   (`crate::decode::invariant::infeasible`); `K*` against 0, and `L_z` against 0, for the constraint named and the
//!   class kept.
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
//!
//! **The active values** (chart_reference §0.1; chart_decoder_contract § "Tunable parameters": the encode's clamps and
//! its layer-1 range check read the active value; R-481 G4; REQ-DEC-048). Every path here has a form at the tunable
//! parameters `p` ([`encode_into_within_params`], [`Residual::controls_at`], [`fibre_at_params`], [`fibre_params`]):
//! `E(x)`, the chart's decode, the mass link's inverse and the momentum clamps, so layer 1's `lookup_clamped` bits, all
//! at the active `μ_max` and `q_max`. The forms without them are these at `LinkParams::DEFAULT`.

use spirv_std::num_traits::{Float, FloatConst};

use super::curve::CurveEncoded;
use super::{encode_at, Encoded, SimplexClamp};
use crate::canon;
use crate::chart::invariant::{l_max, InvariantChart, InvariantLE, InvariantLK, WarpReal};
use crate::chart::mixed::{Coordinate, Quantity};
use crate::chart::{ChartDecode, Constraint, Latent, MassSimplex, Mixed, ShapeMap, ShapeSphere};
use crate::decode::config::{self, Config};
use crate::decode::invariant::{construct, construct_by_norms, infeasible, Outcome};
use crate::decode::mass::{self, Masses};
use crate::decode::{descriptor, Ic};
use crate::generated::links::{Link, LinkParams, LinkReal, Selection};
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

    /// The chart coordinates at kind 4's curve tolerance `tol` (Part 5 kind 4; REQ-ENC-026), and a curve chart's curve
    /// encode (`super::curve`). A chart with no curve axis ignores `tol`: its [`ByKind::coords`], and no curve encode.
    #[inline]
    fn coords_within<S, R>(
        &self,
        e: &Encoded<R>,
        _tol: R,
    ) -> ([Coord<R>; 2], Option<CurveEncoded<R>>)
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<Self::Param>,
    {
        (self.coords::<S, R>(e), None)
    }

    /// The state the chart's decode at `c(x)` is compared with, on the chart's own section (Part 6, step 4): `C(x)`,
    /// `e`'s canonical state, on every chart but the shape sphere ([`ShapeSphere`]'s impl, the module's note).
    #[inline]
    fn section<R: LinkReal>(&self, e: &Encoded<R>) -> Ic<R> {
        e.canonical.ic
    }
}

/// Encode into the active chart: `E(x)`, the chart point read off it by kind, and where it lands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChartEncoded<R> {
    /// `E(x)`: the latent `z`, which represents every state, `C(x)` and the discard report.
    pub encoded: Encoded<R>,
    /// The chart coordinates, by kind.
    pub coords: [Coord<R>; 2],
    /// The caller's norm of the chart's decode at the coordinates against `C(x)`; `None` where no point was placed.
    pub residual: Option<R>,
    /// The chart point, or latent `z`.
    pub landing: Landing<R>,
    /// A curve chart's curve encode at the caller's `tol`: the closed form, its `past_fold` flag among it, `d`, `ν*`
    /// where the projection ran, and the landing on the curve (`super::curve`; REQ-ENC-022, REQ-ENC-026). `None` on
    /// every other chart.
    pub curve: Option<CurveEncoded<R>>,
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

/// [`encode_into_within`] at `tol = 0` (the module's note): a curve chart represents only a state on its curve exactly,
/// and rounding alone puts `d > 0` on the curve, so there it places essentially no state. On a curve chart, call
/// [`encode_into_within`] with REQ-ENC-026's tolerance (inverse_encode_contract Part 6, step 4).
#[inline]
pub fn encode_into<S, R, C, N>(chart: &C, ic: &Ic<R>, norm: N, eps_phys: R) -> ChartEncoded<R>
where
    S: Selection,
    S::MassLink: SimplexClamp,
    R: LinkReal + PayloadReal + From<C::Param>,
    C: ByKind,
    N: Fn(&Ic<R>, &Ic<R>) -> R,
{
    encode_into_within::<S, R, C, N>(chart, ic, norm, eps_phys, R::zero())
}

/// Encode `ic` into `chart` under the selection `S` (the module's note): `E(ic)`, the chart point by kind at the curve
/// tolerance `tol`, and the chart as the landing where `norm(C(ic), D(Φ(c(ic))))`, its decode there measured from
/// `C(ic)` (on the shape sphere, from [`ByKind::section`]'s `C(ic)` with the mirror undone), is at most `eps_phys`, else
/// latent `z`. `norm` is inverse_encode_contract Part 4's `‖·‖_phys` (REQ-ENC-029) and `eps_phys` its `ε_phys`
/// (REQ-ENC-024), both the caller's (Part 6, step 4's "where possible"), and `tol` Part 5 kind 4's (REQ-ENC-026), the
/// caller's too.
#[inline]
pub fn encode_into_within<S, R, C, N>(
    chart: &C,
    ic: &Ic<R>,
    norm: N,
    eps_phys: R,
    tol: R,
) -> ChartEncoded<R>
where
    S: Selection,
    S::MassLink: SimplexClamp,
    R: LinkReal + PayloadReal + From<C::Param>,
    C: ByKind,
    N: Fn(&Ic<R>, &Ic<R>) -> R,
{
    encode_into_within_params::<S, R, C, N>(chart, ic, norm, eps_phys, tol, &LinkParams::DEFAULT)
}

/// [`encode_into_within`] at the tunable parameters `p`, the active values (the module's note): `E(ic)` at them, its
/// clamp bits layer 1's at the active `μ_max` and `q_max`, and the chart's decode at them
/// ([`ChartDecode::decode_at_params`]).
#[inline]
pub fn encode_into_within_params<S, R, C, N>(
    chart: &C,
    ic: &Ic<R>,
    norm: N,
    eps_phys: R,
    tol: R,
    p: &LinkParams<R>,
) -> ChartEncoded<R>
where
    S: Selection,
    S::MassLink: SimplexClamp,
    R: LinkReal + PayloadReal + From<C::Param>,
    C: ByKind,
    N: Fn(&Ic<R>, &Ic<R>) -> R,
{
    let encoded = encode_at::<S, R>(ic, p);
    let (coords, curve) = chart.coords_within::<S, R>(&encoded, tol);
    let (residual, landing) = match (placed(coords[0]), placed(coords[1])) {
        (Some(u), Some(v)) => {
            let d = norm(
                &chart.section(&encoded),
                &chart.decode_at_params::<S, R>(u, v, p).ic,
            );
            (
                Some(d),
                select(d <= eps_phys, Landing::Chart { u, v }, Landing::Latent),
            )
        }
        _ => (None, Landing::Latent),
    };
    ChartEncoded {
        encoded,
        coords,
        residual,
        landing,
        curve,
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

/// `⟨a, b⟩` over the eight components: the first product, then a chain of explicit `fma`s, each by a literal index
/// (gpu_determinism_note rules 5 and 6).
#[inline]
fn dot<R: LinkReal>(a: &[R; 8], b: &[R; 8]) -> R {
    let fma = Float::mul_add;
    let s = fma(a[1], b[1], a[0] * b[0]);
    let s = fma(a[2], b[2], s);
    let s = fma(a[3], b[3], s);
    let s = fma(a[4], b[4], s);
    let s = fma(a[5], b[5], s);
    let s = fma(a[6], b[6], s);
    fma(a[7], b[7], s)
}

/// The components `(a, b)` of `d` in the plane of `q1`, `q2`: the least-squares solution of `d ≈ a·q₁ + b·q₂`, the
/// Gram system `[⟨q₁,q₁⟩ ⟨q₁,q₂⟩; ⟨q₁,q₂⟩ ⟨q₂,q₂⟩]·(a, b) = (⟨q₁,d⟩, ⟨q₂,d⟩)`, which is `d`'s own components for
/// `q = ê_i, ê_j`; `None` where the directions are dependent (a zero Gram determinant, in the module's fixed form).
#[inline]
pub fn plane<R: LinkReal>(d: &[R; 8], q1: &[R; 8], q2: &[R; 8]) -> Option<(R, R)> {
    let (g11, g12, g22) = (dot(q1, q1), dot(q1, q2), dot(q2, q2));
    let (r1, r2) = (dot(q1, d), dot(q2, d));
    let w = g12 * g12;
    let det = Float::mul_add(g11, g22, -w) + Float::mul_add(-g12, g12, w);
    let independent = det != R::zero();
    let div = select(independent, det, R::one());
    select(
        independent,
        Some(((g22 * r1 - g12 * r2) / div, (g11 * r2 - g12 * r1) / div)),
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
        let d = [
            z[0] - z0[0],
            z[1] - z0[1],
            z[2] - z0[2],
            z[3] - z0[3],
            z[4] - z0[4],
            z[5] - z0[5],
            z[6] - z0[6],
            z[7] - z0[7],
        ];
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
    /// (`super::blocks::guard`). [`Residual::controls_at`] at the defaults.
    #[inline]
    fn controls<S: Selection, R: LinkReal>(m0: R, held: [R; 3]) -> [R; 2] {
        Self::controls_at::<S, R>(m0, held, &LinkParams::DEFAULT)
    }

    /// [`Residual::controls`] at the tunable parameters `p`: the mass link's inverse at the active `μ_max`, so the
    /// decode at the same `p` gives the masses back (the module's note).
    #[inline]
    fn controls_at<S: Selection, R: LinkReal>(m0: R, held: [R; 3], p: &LinkParams<R>) -> [R; 2] {
        let m = super::blocks::guard(Self::masses(m0, held));
        <S::MassLink as Link<2, 3>>::inverse_at(m, p)
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
/// forms both, `v` written `(m₂ − ε_m)/(k − (m₁ − ε_m))`, `k = 1 − 3ε_m`; `v = 0` on the edge `u = 1`, where that
/// divisor is 0 and the decode does not read `v`. The fixed triangle and the rest start are [`encode_into`]'s
/// comparison.
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
        let rest = k - (m[1] - eps);
        let edge = rest == R::zero();
        let v = select(edge, R::zero(), (m[2] - eps) / select(edge, R::one(), rest));
        [Coord::At(u), Coord::At(v)]
    }
}

/// `⟨a, b⟩` in three dimensions: the first product, then explicit `fma`s.
#[inline]
fn dot3<R: LinkReal>(a: [R; 3], b: [R; 3]) -> R {
    Float::mul_add(a[2], b[2], Float::mul_add(a[1], b[1], a[0] * b[0]))
}

/// The spherical map inverted at the unit point `n`, `pi` the float type's π: `θ = atan2(n₁, n₀)` taken into
/// `[0, 2π)`, `φ = atan2(√(n₀² + n₁²), n₂)`, `(s, t) = (θ/2π, 1 − φ/π)` (chart_reference §3.3). At the poles, `n₀ = n₁
/// = 0`, `θ` is 0, selected, never `atan2(0, 0)` (R-437). `θ` is taken into `[0, 2π)` on `n₁`'s sign, a `−0` as `+0`,
/// so the select reads no transcendental (the module's list).
#[inline]
pub fn spherical_inverse<R: LinkReal>(n: [R; 3], pi: R) -> (R, R) {
    let zero = R::zero();
    let turn = pi + pi;
    let y = select(n[1] == zero, zero, n[1]);
    let pole = (n[0] == zero) & (y == zero);
    let theta = select(pole, zero, Float::atan2(y, n[0]));
    let theta = select(y < zero, theta + turn, theta);
    let phi = Float::atan2(Float::sqrt(Float::mul_add(n[1], n[1], n[0] * n[0])), n[2]);
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
    let perp = [
        Float::mul_add(-c, n0[0], n[0]),
        Float::mul_add(-c, n0[1], n[1]),
        Float::mul_add(-c, n0[2], n[2]),
    ];
    let sin2 = dot3(perp, perp);
    let centre = sin2 == R::zero();
    let sin_r = select(centre, R::one(), Float::sqrt(sin2));
    let k = select(centre, R::zero(), Float::atan2(sin_r, c) / sin_r);
    let d = [k * perp[0], k * perp[1], k * perp[2]];
    let half = R::one() / (R::one() + R::one());
    (
        (dot3(d, e1) / scale + R::one()) * half,
        (dot3(d, e2) / scale + R::one()) * half,
    )
}

/// The shape sphere: both axes kind 2 in the configuration block, together pinning its two DOF, so no residual. The
/// point is the state's signed `n̂` (R-544; REQ-ENC-034): `n = shape_vec(ρ̃, λ̃)` of `C(x)` with step 3's mirror undone
/// where `lookup_mirrored` is set, `λ̃_y` and `ρ̃_y` negated back, so a lower-hemisphere state is placed below the
/// equator, through the chart map's inverse. The held masses and momenta are [`encode_into`]'s comparison, against
/// [`ByKind::section`], the same state undone in full.
impl<P: Copy + FloatConst> ByKind for ShapeSphere<P> {
    fn coords<S, R>(&self, e: &Encoded<R>) -> [Coord<R>; 2]
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<P>,
    {
        let wide = <R as From<P>>::from;
        let j = &e.blocks.jacobi;
        let flip = e.canonical.mirrored;
        let undo = |v: [R; 2]| [v[0], select(flip, -v[1], v[1])];
        let n = shape_vec(undo(j.rho_t), undo(j.lambda_t));
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

    /// `C(x)` with step 3's mirror undone on the full state, every `rᵢ` and every `pᵢ`, where `lookup_mirrored` is set:
    /// the decode's own section, `C` with its step 3 not taken (the module's note).
    #[inline]
    fn section<R: LinkReal>(&self, e: &Encoded<R>) -> Ic<R> {
        canon::reflect(&e.canonical.ic, e.canonical.mirrored)
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
/// and `s = (L_z/L_max + 1)/2`, `L_max = L_max(K)`, `s = ½` where `K = 0`, which is where `L_max = 0`
/// (chart_reference §2.1's warp; the module's list).
#[inline]
pub fn warp_inverse<R: WarpReal>(k_max: R, gamma_k: R, inv: &Invariants<R>) -> (R, R) {
    let half = R::one() / (R::one() + R::one());
    let rest = inv.k == R::zero();
    let l = select(rest, R::one(), l_max(inv.k));
    let ratio = select(rest, R::zero(), inv.lz / l);
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
    let k_c = Float::max(k, zero);
    let lz_c = Float::copysign(l_max(k_c), lz);
    // The contract's criterion itself: `lz_c` takes `L_z`'s sign, so the class changes only where one is 0 and the other
    // not, which is `L_z ≠ 0` with `K* ≤ 0`. `lz_c` is 0 exactly where `k_c` is (`L_max(K) = 0` iff `K = 0`), so the
    // test reads `k_c`, not the `sqrt` (the module's list).
    let keeps = (k_c == zero) == (lz == zero);
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
/// inverse of its `p`, in `z[2:6]`. [`fibre_at_params`] at the defaults.
#[inline]
pub fn fibre_at<S, R>(
    masses: &Masses<R>,
    config: &Config<R>,
    rule: SeedRule<R>,
    targets: (R, R),
    constraints: (Constraint, Constraint),
) -> Lookup<R>
where
    S: Selection,
    S::MassLink: SimplexClamp,
    R: LinkReal + PayloadReal,
{
    fibre_at_params::<S, R>(
        masses,
        config,
        rule,
        targets,
        constraints,
        &LinkParams::DEFAULT,
    )
}

/// [`fibre_at`] at the tunable parameters `params`: the free-momentum inverse and its clamp bits, layer 1, at the active
/// `q_max`, so a constructed momentum past it is clamped and flagged (the module's note).
#[inline]
pub fn fibre_at_params<S, R>(
    masses: &Masses<R>,
    config: &Config<R>,
    rule: SeedRule<R>,
    (lz, k): (R, R),
    (low, wide): (Constraint, Constraint),
    params: &LinkParams<R>,
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
            let inverted = super::blocks::invert_at::<S, R>(&ic, params);
            let mut z = [R::zero(); 8];
            // The momentum block by literal indices (gpu_determinism_note rule 5).
            z[2] = inverted.z[2];
            z[3] = inverted.z[3];
            z[4] = inverted.z[4];
            z[5] = inverted.z[5];
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
/// the pixel's seed, decided on the frozen `z_α`; the frozen controls fill the rest of `z`. [`fibre_params`] at the
/// defaults.
#[inline]
pub fn fibre<S, R, C>(chart: &C, lz: R, y: R) -> Lookup<R>
where
    S: Selection,
    S::MassLink: SimplexClamp,
    R: LinkReal + PayloadReal + From<C::Param>,
    C: InvariantAxis,
{
    fibre_params::<S, R, C>(chart, lz, y, &LinkParams::DEFAULT)
}

/// [`fibre`] at the tunable parameters `p`: the frozen masses at the active `μ_max`, as the pixel's decode at `p` gives
/// them, and [`fibre_at_params`] at `p`.
#[inline]
pub fn fibre_params<S, R, C>(chart: &C, lz: R, y: R, p: &LinkParams<R>) -> Lookup<R>
where
    S: Selection,
    S::MassLink: SimplexClamp,
    R: LinkReal + PayloadReal + From<C::Param>,
    C: InvariantAxis,
{
    let frozen = chart.invariant().frozen::<R>().z;
    let masses = mass::decode_at::<S, R>([frozen[6], frozen[7]], p);
    let z_cfg = [frozen[0], frozen[1]];
    let config = config::decode::<S, R>(z_cfg, &masses);
    let at_rest = Ic {
        m: masses.m,
        r: config.r,
        p: [[R::zero(); 2]; 3],
    };
    let k = C::kinetic(y, descriptor::potential(&at_rest));
    let rule = SeedRule::Decoded { z_alpha: z_cfg[0] };
    match fibre_at_params::<S, R>(&masses, &config, rule, (lz, k), (C::LOW, C::WIDE), p) {
        Lookup::Encoded {
            z,
            lz,
            k,
            projected,
            clamped,
        } => {
            let mut placed = frozen;
            placed[2] = z[2];
            placed[3] = z[3];
            placed[4] = z[4];
            placed[5] = z[5];
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
