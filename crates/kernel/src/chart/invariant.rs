//! `InvariantLE` and `InvariantLK`, the invariant-momentum charts `(L_z, E)` and `(L_z, K)` (chart_reference §2, §2.1,
//! §5.1; REQ-CHART-017): they fix geometry and mass and spend both axes on conserved or constructed quantities, the
//! momenta built by the invariant-momentum construction ([`crate::decode::invariant`], chart_reference §2.2).
//!
//! **The frozen part** (the chart's parameters; [`Invariant`]): the configuration controls `(z_α, z_β)` and the mass
//! controls `(z_μ₁, z_μ₂)`, in the decoder's own terms, decoded by the shared mass and configuration blocks
//! (chart_reference §0.1, §0.2). The chart pins those four of the eight degrees of freedom and the construction's
//! deterministic seed pins the two left in the momentum block (chart_decoder_contract Part 5, "residual convention"),
//! so the chart is 2 swept + 6 frozen. `U` is the potential of the decoded configuration (§2.1), and `I = 1`, the
//! canonical-frame decode's (§0.2).
//!
//! **The warp** ([`warp`]; §2.1), with `(s, t) = (u, v) ∈ [0,1]²`, `K_max > 0`, `γ_K ≥ 1`:
//!
//! ```text
//! K(t)     = K_max·t^γ_K        E(t) = U + K(t)
//! L_max(t) = √(2·I·K(t))        L_z(s, t) = (2s − 1)·L_max(t)
//! ```
//!
//! It maps the unit square onto the feasible interior `|L_z| ≤ √(2I(E − U))`, so no pixel is infeasible by
//! construction. **It holds in floating point too** ([`l_max`]): a rounded `√(2K)` can lie above the exact root, so that
//! `L_max² > 2K` and the edge pixel `s ∈ {0, 1}` would fail the construction's exact test `2·I·K* < L_z²`. `L_max` is
//! therefore stepped down, one float at a time, while that exact test ([`infeasible`]) still fails it, four steps at
//! most, enough for a square root a few ulps above the exact one (a correctly rounded one needs one). Then
//! `|L_z| = |2s − 1|·L_max ≤ L_max` after rounding, as `|2s − 1| ≤ 1` is computed exactly or rounded within `[−1, 1]`
//! and rounding is monotone, so `L_z² ≤ L_max² ≤ 2K` exactly, and the construction's branch is feasible at every
//! `(s, t) ∈ [0,1]²`, the edges included, on a backend whose `sqrt` lies within four ulps of the exact root: proven
//! and tested on the CPU at f32 and f64 (correctly rounded). WGSL's nominal `sqrt` bound (inherited from
//! `1/inverseSqrt`) can exceed four ulps, so the GPU's edges are verified where its feasibility tags are compared
//! with the CPU's, TASK-M2-24's cross-backend check, whose fuzz covers the edge columns `s ∈ {0, 1}`.
//!
//! **`K* = K(t)` on both charts.** On `(L_z, K)` it is the coordinate itself (§2.1). On `(L_z, E)`, §2.2's
//! `K* = E − U` with `E = U + K(t)` is `K(t)` in exact arithmetic, so the decode takes `K(t)` directly rather than
//! forming `E` and subtracting `U` again, whose rounding could put `K*` below `K_min` at the parabola's edge. The two
//! charts decode alike; they differ in what the vertical axis reads ([`InvariantLE::energy`]), in their constraints
//! (inverse_encode_contract § "Chart-aware validation", layer 2) and in their names.
//!
//! **The decode** ([`decode()`]): the shared mass and configuration blocks, then the construction in place of the free
//! momentum block ([`InvariantBlocks`]), then the shared derived invariants ([`crate::decode::decode_with`]). The
//! configuration block hands the construction the `z_α` it decoded, so R-82's seed is decided on the very control the
//! configuration came from ([`crate::decode::invariant::construct`]). `Φ`'s [`ChartOut`] carries the frozen controls
//! (the momentum controls 0, unread); the targets `(L_z, K*)` are [`Invariant::targets`]. Both charts override
//! [`Chart::decode`] with it, the outcome dropped, so every entry runs the construction, never the free decode of `Φ`'s
//! controls; the exact-sign arithmetic it needs ([`WarpReal`]) every payload float type has
//! ([`crate::real::IeeeBits`]'s bound).
//!
//! **`η_E` is forbidden** (chart_reference §0.6; R-25): energy is a coordinate here, or is fixed by the construction,
//! so both charts set `forbids_energy_normalisation`, and the validation pass refuses any view combining them with an
//! `E*` (`engine::chart_validate`).
//!
//! **The infeasible label** (chart_reference §2.2): the construction's [`Outcome`] is returned beside the decode. Under
//! the warp it is never [`Outcome::Infeasible`] inside `[0,1]²` (the note above); past it the chart is bounded on every
//! side and hatched (§5.4), and [`validate`](Chart::validate) projects such a point back. `DEGENERATE`'s member for it,
//! `INVARIANT_INFEASIBLE`, `decode_failed`/3 ([`crate::decode::invariant::infeasible_label`]), joins the reason type with
//! its full set (TASK-M2-24); until then [`Decoded::degenerate`] carries the mass block's tag alone.
//!
//! **Validation** (R-26; chart_reference §5.1): inside `[0,1]²` every point is `Valid` (layer 2 by the warp; layer 3,
//! decode sanity, by the shared decoder's construction: positive masses, the CoM at the origin, `Σpᵢ = 0` by the
//! construction's projection). Past it: a non-finite coordinate is rejected; `t < 0` (`K < 0`, `E < U`, and no real
//! `t^γ_K` for a non-integer `γ_K`) is projected to `(clamp(s), 0)`, the parabola's apex, the rest start; `s ∉ [0, 1]`
//! (`|L_z| > L_max`) is projected to `(clamp(s), t)`, on the parabola at the same `K` and `E`. `t > 1` with `s ∈ [0, 1]`
//! is feasible (`K > K_max`) and `Valid`; the axis is still bounded there, which is the extension type's business, not
//! validation's (§2.1, §5.4).

use core::marker::PhantomData;

use spirv_std::num_traits::Float;

use super::{Chart, ChartOut, Constraint, ValidationResult};
use crate::decode::config::{self, Config};
use crate::decode::invariant::{self as construction, infeasible, ExactSign, Outcome};
use crate::decode::mass::{self, Masses};
use crate::decode::momentum::Momentum;
use crate::decode::{self, descriptor, Blocks, Decoded, Ic};
use crate::generated::links::{LinkReal, Selection};
use crate::payload::{ICDescriptorOf, PayloadReal};

/// `t` where `c` holds, else `f`: both operands are computed before the choice, so it is a select, not a branch.
#[inline]
fn select<T: Copy>(c: bool, t: T, f: T) -> T {
    if c {
        t
    } else {
        f
    }
}

/// The float below a non-negative float, for [`l_max`]'s steps down.
pub trait WarpReal: ExactSign {
    /// The next float toward 0 from a positive finite `self`; `self` itself at `±0`.
    fn below(self) -> Self;
}

impl WarpReal for f32 {
    #[inline]
    fn below(self) -> f32 {
        let bits = self.to_bits();
        f32::from_bits(bits - select(bits & 0x7FFF_FFFF == 0, 0, 1))
    }
}

impl WarpReal for f64 {
    #[inline]
    fn below(self) -> f64 {
        let bits = self.to_bits();
        f64::from_bits(bits - select(bits & 0x7FFF_FFFF_FFFF_FFFF == 0, 0, 1))
    }
}

/// One step of [`l_max`]: `l` one float lower where `L_z = l` is still infeasible at `K* = k`, `I = 1`.
#[inline]
fn lower<R: WarpReal>(k: R, l: R) -> R {
    select(infeasible(R::one(), k, l), l.below(), l)
}

/// `L_max = √(2·I·K)` at `I = 1`, held to `L_max² ≤ 2K` exactly where `sqrt` lies within four ulps of the exact root:
/// the rounded root, stepped down while the exact test still finds it infeasible, four steps at most (the module's
/// note: the CPU's is correctly rounded; a GPU's is checked by TASK-M2-24).
#[inline]
pub fn l_max<R: WarpReal>(k: R) -> R {
    let root = Float::sqrt(k + k);
    lower(k, lower(k, lower(k, lower(k, root))))
}

/// A chart point's targets under the warp: the kinetic energy `K* = K(t)`, `L_max(t)` and `L_z(s, t)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Targets<R> {
    /// `K* = K(t) = K_max·t^γ_K`.
    pub k: R,
    /// `L_max(t) = √(2·I·K(t))`, `L_max² ≤ 2K` exactly.
    pub l_max: R,
    /// `L_z(s, t) = (2s − 1)·L_max(t)`.
    pub lz: R,
}

/// §2.1's `K(t) = K_max·t^γ_K`, the warp's kinetic energy, which the Burrau `(ν, K)` chart's vertical axis takes too
/// (chart_reference §4.5; R-113).
#[inline]
pub fn kinetic<R: Float>(k_max: R, gamma_k: R, t: R) -> R {
    k_max * Float::powf(t, gamma_k)
}

/// §2.1's warp at `(s, t)` under `K_max = k_max` and `γ_K = gamma_k`, `I = 1` (the module's note).
#[inline]
pub fn warp<R: WarpReal>(k_max: R, gamma_k: R, s: R, t: R) -> Targets<R> {
    let k = kinetic(k_max, gamma_k, t);
    let l_max = l_max(k);
    Targets {
        k,
        l_max,
        lz: (s + s - R::one()) * l_max,
    }
}

/// What both invariant charts hold: the frozen configuration and mass controls and the warp's two parameters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Invariant<P> {
    /// The configuration controls `(z_α, z_β)`, decoded by the shared configuration block.
    pub z_cfg: [P; 2],
    /// The mass controls `(z_μ₁, z_μ₂)`, decoded by the shared mass block.
    pub z_mass: [P; 2],
    /// `K_max > 0`, the kinetic energy at `t = 1`.
    pub k_max: P,
    /// `γ_K ≥ 1`, the warp's exponent.
    pub gamma_k: P,
}

impl<P: Copy> Invariant<P> {
    /// The frozen controls as a latent vector, in the decoder's block layout: `z[0:2] = (z_α, z_β)`, `z[6:8] =
    /// (z_μ₁, z_μ₂)`, and the momentum controls `z[2:6]` 0, which the construction does not read.
    #[inline]
    pub fn frozen<F: Float + From<P>>(&self) -> decode::Latent<F> {
        let wide = <F as From<P>>::from;
        let zero = F::zero();
        decode::Latent::new([
            wide(self.z_cfg[0]),
            wide(self.z_cfg[1]),
            zero,
            zero,
            zero,
            zero,
            wide(self.z_mass[0]),
            wide(self.z_mass[1]),
        ])
    }

    /// The targets at `(u, v)`: §2.1's warp at `(s, t) = (u, v)`.
    #[inline]
    pub fn targets<F: WarpReal + From<P>>(&self, u: F, v: F) -> Targets<F> {
        let wide = <F as From<P>>::from;
        warp(wide(self.k_max), wide(self.gamma_k), u, v)
    }
}

/// The `(L_z, E)` chart.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InvariantLE<P>(pub Invariant<P>);

/// The `(L_z, K)` chart.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InvariantLK<P>(pub Invariant<P>);

impl<P: Copy> InvariantLE<P> {
    /// The vertical axis' value, `E(t) = U + K(t)`, given the frozen configuration's potential `u_pot`.
    #[inline]
    pub fn energy<F: WarpReal + From<P>>(&self, u_pot: F, v: F) -> F {
        u_pot + self.0.targets(F::zero(), v).k
    }
}

/// An invariant chart: the frozen part both charts hold.
pub trait InvariantChart: Chart {
    /// The frozen controls and the warp.
    fn invariant(&self) -> &Invariant<Self::Param>;
}

impl<P: Copy> InvariantChart for InvariantLE<P> {
    #[inline]
    fn invariant(&self) -> &Invariant<P> {
        &self.0
    }
}

impl<P: Copy> InvariantChart for InvariantLK<P> {
    #[inline]
    fn invariant(&self) -> &Invariant<P> {
        &self.0
    }
}

/// `validate` for both charts (the module's note): `low` is the constraint `t < 0` fails, `wide` the one `s ∉ [0, 1]`
/// fails.
fn check(u: f64, v: f64, low: Constraint, wide: Constraint) -> ValidationResult {
    let s = u.clamp(0.0, 1.0);
    if !(u.is_finite() && v.is_finite()) {
        ValidationResult::Reject {
            failed: select(v.is_finite(), wide, low),
        }
    } else if v < 0.0 {
        ValidationResult::Project {
            u: s,
            v: 0.0,
            failed: low,
        }
    } else if s != u {
        ValidationResult::Project {
            u: s,
            v,
            failed: wide,
        }
    } else {
        ValidationResult::Valid
    }
}

impl<P: Copy> Chart for InvariantLE<P> {
    type Param = P;

    /// The frozen controls ([`Invariant::frozen`]); the targets are [`Invariant::targets`].
    #[inline]
    fn map<F: Float + From<P>>(&self, _u: F, _v: F) -> ChartOut<F> {
        ChartOut { z: self.0.frozen() }
    }

    /// `true`: energy is the vertical coordinate (chart_reference §0.6).
    fn forbids_energy_normalisation(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "InvariantLE"
    }

    /// `false`: the masses are held, `D_mass` at the chart's `z_mass`.
    fn requires_per_pixel_mass(&self) -> bool {
        false
    }

    /// [`decode()`] at `(u, v)` under `S`, the construction's outcome dropped, as [`super::ChartDecode::decode_at`]
    /// decodes it: the construction's momenta, not the free decode of the frozen controls.
    #[inline]
    fn decode<S, R>(&self, u: R, v: R) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<P>,
    {
        decode::<S, R, Self>(self, u, v).0
    }

    /// Layer 2's parabola `|L_z| ≤ √(2I(E − U))` (the module's note).
    fn validate(&self, u: f64, v: f64) -> ValidationResult {
        check(u, v, Constraint::LzEParabola, Constraint::LzEParabola)
    }
}

impl<P: Copy> Chart for InvariantLK<P> {
    type Param = P;

    /// The frozen controls ([`Invariant::frozen`]); the targets are [`Invariant::targets`].
    #[inline]
    fn map<F: Float + From<P>>(&self, _u: F, _v: F) -> ChartOut<F> {
        ChartOut { z: self.0.frozen() }
    }

    /// `true`: the construction fixes the energy (chart_reference §0.6).
    fn forbids_energy_normalisation(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "InvariantLK"
    }

    /// `false`: the masses are held, `D_mass` at the chart's `z_mass`.
    fn requires_per_pixel_mass(&self) -> bool {
        false
    }

    /// [`decode()`] at `(u, v)` under `S`, the construction's outcome dropped, as [`super::ChartDecode::decode_at`]
    /// decodes it: the construction's momenta, not the free decode of the frozen controls.
    #[inline]
    fn decode<S, R>(&self, u: R, v: R) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<P>,
    {
        decode::<S, R, Self>(self, u, v).0
    }

    /// Layer 2's `K ≥ 0` and `K ≥ L_z²/2I` (the module's note).
    fn validate(&self, u: f64, v: f64) -> ValidationResult {
        check(u, v, Constraint::KNonNegative, Constraint::KAboveLzBound)
    }
}

/// The invariant charts' decoder under the selection `S`: the shared mass and configuration blocks, the construction
/// in place of the free momentum block, and the shared derived invariants. The configuration block keeps the `z_α` it
/// decoded, which the construction's seed rule reads.
pub struct InvariantBlocks<S, R> {
    /// The target `L_z`.
    lz: R,
    /// The target `K*`.
    k_star: R,
    /// The `z_α` the configuration block decoded.
    z_alpha: R,
    /// What the construction did.
    outcome: Outcome,
    links: PhantomData<S>,
}

impl<S, R: Float> InvariantBlocks<S, R> {
    /// The decoder of the targets `lz`, `k_star`.
    #[inline]
    pub fn new(lz: R, k_star: R) -> Self {
        InvariantBlocks {
            lz,
            k_star,
            z_alpha: R::zero(),
            outcome: Outcome::Feasible,
            links: PhantomData,
        }
    }

    /// What the construction did, once the momentum block has run.
    #[inline]
    pub fn outcome(&self) -> Outcome {
        self.outcome
    }
}

/// The Jacobi momenta of the particle momenta `p` (the inverse of §0.3's map): `p_λ = p₂`, `p_ρ = p₁ + (m₁/M₀₁)·p₂`.
#[inline]
pub(crate) fn jacobi<R: LinkReal>(masses: &Masses<R>, p: [[R; 2]; 3]) -> Momentum<R> {
    let w1 = masses.m[1] / masses.m01;
    let p_lambda = p[2];
    let p_rho = [p[1][0] + w1 * p[2][0], p[1][1] + w1 * p[2][1]];
    Momentum {
        q: [p_rho[0], p_rho[1], p_lambda[0], p_lambda[1]],
        p_rho,
        p_lambda,
        p,
    }
}

impl<S: Selection, R: LinkReal + PayloadReal + ExactSign> Blocks<R> for InvariantBlocks<S, R> {
    #[inline]
    fn mass(&mut self, z_mass: [R; 2]) -> Masses<R> {
        mass::decode::<S, R>(z_mass)
    }

    #[inline]
    fn config(&mut self, z_cfg: [R; 2], masses: &Masses<R>) -> Config<R> {
        self.z_alpha = z_cfg[0];
        config::decode::<S, R>(z_cfg, masses)
    }

    /// The construction (chart_reference §2.2), its seed decided on the decoded `z_α`; the momentum controls are not
    /// read. [`Momentum::q`] holds the construction's Jacobi momenta `(p_ρ, p_λ)`, not decoded controls.
    #[inline]
    fn momentum(&mut self, _z_mom: [R; 4], masses: &Masses<R>, config: &Config<R>) -> Momentum<R> {
        let c = construction::construct(masses, config, self.z_alpha, self.lz, self.k_star);
        self.outcome = c.outcome;
        jacobi(masses, c.p)
    }

    #[inline]
    fn invariants(&mut self, ic: &Ic<R>) -> ICDescriptorOf<R> {
        descriptor::describe(ic)
    }
}

/// `D ∘ Φ` at `(u, v)` on an invariant chart under the selection `S`: the decode and the construction's outcome (the
/// module's note).
#[inline]
pub fn decode<S, R, C>(chart: &C, u: R, v: R) -> (Decoded<R>, Outcome)
where
    S: Selection,
    R: LinkReal + PayloadReal + WarpReal + From<C::Param>,
    C: InvariantChart,
{
    let inv = chart.invariant();
    let t = inv.targets(u, v);
    let mut blocks = InvariantBlocks::<S, R>::new(t.lz, t.k);
    let decoded = decode::decode_with(&mut blocks, &chart.map(u, v).z);
    (decoded, blocks.outcome())
}
