//! The acute-angle Burrau charts and the strips (chart_reference §4.5; REQ-CHART-024, REQ-CHART-027, REQ-CHART-025):
//! `Φ_θ,K` ([`BurrauThetaK`], which is the bifurcation strip `Φ_strip,K`), `Φ_θ,L_z` ([`BurrauThetaLz`]) and the δm
//! strip `Φ_strip,δm` ([`BurrauThetaDeltaM`]). Each is an ordinary chart map into the shared decoder, through the
//! `ν` axis's blocks (`super::burrau`): no Burrau-specific integrator or renderer.
//!
//! **The acute-angle axis** ([`nu_of`], [`theta_of`]; §4.5). `θ ∈ (0, π/4]` is horizontal,
//! `θ(u) = θ_min + (θ_max − θ_min)·u`, its range REQ-CHART-045's (proposed in the constants register), and the system is
//! the `ν` axis's embed at `ν(θ) = sec θ − tan θ`, evaluated as `cos θ/(1 + sin θ)`, the same function, which cancels
//! nowhere on `(0, π/4]`. It inverts `θ(ν) = arctan((1 − ν²)/(2ν))` ([`super::burrau::acute_angle`]) there, onto
//! `ν ∈ [√2 − 1, 1)`: the fold's end `θ = π/4` is the isosceles triangle, and the classical point `ν = 1/2` is
//! `θ = arctan(3/4)`. `θ` folds out the leg swap (`θ` and `π/2 − θ` are the same triangle with the legs swapped), so
//! each chart covers each shape once: [`SystemImage::Bijective`], the quotient "shape only" ([`ChartLabel`]).
//!
//! **`Φ_θ,K`** ([`BurrauThetaK`]). `K(v) = K_max·v^γ_K`, the invariant warp's ([`super::invariant::kinetic`]; its
//! `K_max` and `γ_K` REQ-CHART-044's, R-113), and the momenta §2.2's construction at `(L_z = 0, K(v))`, its seed R-82's
//! decided on `ν` ([`super::burrau::seed`]; on `ν ≥ √2 − 1 > 1/3` always `(0, λ)`), as on the `(ν, K)` chart.
//!
//! **`Φ_θ,L_z`** ([`BurrauThetaLz`]). `K` fixed and `L_z(v) = (2v − 1)·λ_L·L_max(K)`, `λ_L ∈ (0, 1]`, both
//! REQ-CHART-046's: a band symmetric about 0 inside the feasible `|L_z| ≤ √(2IK)`, `I = 1`. `L_max` is the warp's
//! ([`super::invariant::l_max`]), held to `L_max² ≤ 2K` exactly, and `|(2v − 1)·λ_L| ≤ 1` after rounding on `[0, 1]`,
//! so `L_z² ≤ 2K` and the construction is feasible at every `(u, v) ∈ [0,1]²`, the edges included.
//!
//! **The δm strip** ([`BurrauThetaDeltaM`], [`DeltaMBlocks`]). `m(u, v) = (1 − v)·m_Burrau(ν(θ(u))) + v·m_target`, the
//! target by default equal masses ([`BurrauThetaDeltaM::equal_masses`]), the momenta zero. The triangle is the Burrau
//! one at `ν(θ(u))`; only the masses are blended, and its Jacobi vectors are taken under the blended masses,
//! `ρ = r₁ − r₀ = (a/c, 0)` and `λ = r₂ − r₀₁ = (−(m₁/M₀₁)·a/c, b/c)` ([`jacobi`]). The `v = 0` row's masses are the
//! Burrau masses and the `v = 1` row's `m_target`, exactly: `1·x + 0·y = x` and `0·x + 1·y = y` in floating point.
//!
//! **Into the shared decoder**, as on the `ν` axis's charts: `Φ`'s [`ChartOut`] is the zero vector, unread. `Φ_θ,K`
//! and `Φ_θ,L_z` decode by [`decode_theta_k`] and [`decode_theta_lz`] (the construction's [`ExactSign`](crate::decode::invariant::ExactSign) is beyond
//! [`Chart::decode`]'s bound, as on the `(ν, K)` chart), which their [`ChartDecode`] calls; the δm strip overrides
//! [`Chart::decode`] with [`decode_delta_m`]. Energy is the construction's on `Φ_θ,K` and `Φ_θ,L_z`, which forbid
//! `η_E` (chart_reference §0.6; R-25); the δm strip is a rest start and allows it, as the Euclid plane does.
//!
//! **Validation** (R-26; chart_reference §5.1). Layer 2: `θ ∈ (0, π/4]` ([`Constraint::BurrauTheta`]), whose failure is
//! rejected: past `θ = 0` a mass is zero or negative, and past `π/4` the system is the other leg ordering, outside the
//! fold, so neither clamping nor projecting keeps the IC the one intended. Then each chart's vertical constraint:
//! `Φ_θ,K`'s `K ≥ 0`, `v < 0` projected to `v = 0`, the rest start ([`Constraint::KNonNegative`]); `Φ_θ,L_z`'s
//! `K ≥ L_z²/2I`, an infeasible `v` projected to the nearest feasible one at the same `θ` and `K`
//! ([`Constraint::KAboveLzBound`]); the δm strip's `mᵢ > 0` on the blended masses, whose failure, a zero or negative
//! mass, is rejected ([`Constraint::MassPositive`]). Layer 3 on the f64 decode: every decoded mass positive
//! ([`Constraint::DecodedMassPositive`]; `θ` so small that `ν` rounds to 1 gives `a = 0`), then the positions and
//! momenta finite, as on the `ν` axis's charts.
//!
//! **Past `[0,1]²`** each axis's extension type is chart_reference §5.4's (REQ-CHART-055): `θ` and `K` bounded, `L_z`
//! and δm affine; the types are declared in code by TASK-M8-05 (REQ-CHART-054), not here.

use spirv_std::num_traits::Float;

use super::burrau::{
    self, embed, lerp, sanity, select, system_config, system_masses, unread, BurrauBlocks,
};
use super::invariant::{self, WarpReal};
use super::{
    AxisMeta, BlockTouch, Chart, ChartDecode, ChartLabel, ChartOut, Constraint, SystemImage,
    ValidationResult,
};
use crate::burrau::{Convention, System, CONVENTION, CONVENTION_STATEMENT};
use crate::decode::config::Config;
use crate::decode::invariant::{infeasible, Outcome};
use crate::decode::mass::Masses;
use crate::decode::momentum::Momentum;
use crate::decode::{self, descriptor, Blocks, Decoded, Ic};
use crate::generated::links::{LinkReal, Selection};
use crate::payload::{ICDescriptorOf, PayloadReal};

/// `ν(θ) = sec θ − tan θ`, evaluated as `cos θ/(1 + sin θ)` (the module's note).
#[inline]
pub fn nu_of<R: Float>(theta: R) -> R {
    Float::cos(theta) / (R::one() + Float::sin(theta))
}

/// `θ(ν) = arctan((1 − ν²)/(2ν))`, the inverse of [`nu_of`] on `(0, π/4]` (§4.3; [`burrau::acute_angle`]).
#[inline]
pub fn theta_of<R: Float>(nu: R) -> R {
    burrau::acute_angle(nu)
}

/// `θ(u) = θ_min + (θ_max − θ_min)·u` on `theta_range`.
#[inline]
fn theta_at<F: Float + From<P>, P: Copy>(theta_range: [P; 2], u: F) -> F {
    let wide = <F as From<P>>::from;
    lerp(wide(theta_range[0]), wide(theta_range[1]), u)
}

/// Layer 2's `θ ∈ (0, π/4]`.
#[inline]
fn in_fold(theta: f64) -> bool {
    theta > 0.0 && theta <= core::f64::consts::FRAC_PI_4
}

/// The axis metadata of `θ`: it writes the configuration and the masses, through `ν`.
#[inline]
fn theta_axis() -> AxisMeta {
    AxisMeta {
        touches: BlockTouch {
            config: true,
            momentum: false,
            mass: true,
        },
        annotation: false,
    }
}

/// The axis metadata of a vertical axis writing only `momentum` (`K`, `L_z`) or only the masses (δm).
#[inline]
fn vertical_axis(momentum: bool) -> AxisMeta {
    AxisMeta {
        touches: BlockTouch {
            config: false,
            momentum,
            mass: !momentum,
        },
        annotation: false,
    }
}

/// The acute-angle charts' label: `name`, bijective, "shape only" (the module's note).
#[inline]
fn fold_label(name: &str) -> ChartLabel<'_> {
    burrau::label(name, SystemImage::Bijective)
}

/// Layer 3 on the f64 decode `d` (the module's note): the decoded masses positive, then [`sanity`].
fn decoded_sanity(d: &Decoded<f64>) -> Option<Constraint> {
    if d.ic.m.iter().all(|&m| m > 0.0) {
        sanity(d)
    } else {
        Some(Constraint::DecodedMassPositive)
    }
}

/// `validate`'s first steps on every acute-angle chart: `Some` of the result where `u` or `v` is not finite (naming
/// `vertical`, the vertical axis' constraint, where only `v` is not), or where `θ(u)` is outside the fold.
fn reject_early(theta: f64, u: f64, v: f64, vertical: Constraint) -> Option<ValidationResult> {
    if !(u.is_finite() && v.is_finite()) {
        return Some(ValidationResult::Reject {
            failed: select(u.is_finite(), vertical, Constraint::BurrauTheta),
        });
    }
    (!in_fold(theta)).then_some(ValidationResult::Reject {
        failed: Constraint::BurrauTheta,
    })
}

// ── Φ_θ,K ───────────────────────────────────────────────────────────────────────────────────────────────────────────

/// `Φ_θ,K`, the bifurcation strip `Φ_strip,K` (chart_reference §4.5): `θ(u)` on `theta_range`, `K(v) = K_max·v^γ_K`,
/// `L_z = 0`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BurrauThetaK<P> {
    /// `[θ_min, θ_max]`, inside `(0, π/4]`.
    pub theta_range: [P; 2],
    /// `K_max > 0`, the kinetic energy at `v = 1`.
    pub k_max: P,
    /// `γ_K ≥ 1`, the warp's exponent.
    pub gamma_k: P,
}

impl<P: Copy> BurrauThetaK<P> {
    /// `θ(u) = θ_min + (θ_max − θ_min)·u`.
    #[inline]
    pub fn theta<F: Float + From<P>>(&self, u: F) -> F {
        theta_at(self.theta_range, u)
    }

    /// `ν(θ(u))`.
    #[inline]
    pub fn nu<F: Float + From<P>>(&self, u: F) -> F {
        nu_of(self.theta(u))
    }

    /// `K(v) = K_max·v^γ_K`.
    #[inline]
    pub fn kinetic<F: Float + From<P>>(&self, v: F) -> F {
        let wide = <F as From<P>>::from;
        invariant::kinetic(wide(self.k_max), wide(self.gamma_k), v)
    }

    /// The convention the chart's systems are stated in: the normalised one (chart_reference §4.2).
    #[inline]
    pub fn convention(&self) -> Convention {
        CONVENTION
    }

    /// [`BurrauThetaK::convention`] in words.
    #[inline]
    pub fn convention_statement(&self) -> &'static str {
        CONVENTION_STATEMENT
    }

    /// The axes' metadata: `θ` writes the configuration and the masses, `K` the momenta; the chart is 2-swept.
    #[inline]
    pub fn axes(&self) -> [AxisMeta; 2] {
        [theta_axis(), vertical_axis(true)]
    }

    /// The chart's `system_image`: bijective, each shape once (the module's note).
    #[inline]
    pub fn system_image(&self) -> SystemImage {
        SystemImage::Bijective
    }
}

impl<P: Copy> BurrauThetaK<P>
where
    f64: From<P>,
{
    /// The chart's label: its name, bijective, "shape only".
    #[inline]
    pub fn label(&self) -> ChartLabel<'_> {
        fold_label(self.name())
    }
}

impl<P: Copy> Chart for BurrauThetaK<P>
where
    f64: From<P>,
{
    type Param = P;

    /// The zero vector, unread (the module's note).
    #[inline]
    fn map<F: Float + From<P>>(&self, _u: F, _v: F) -> ChartOut<F> {
        unread()
    }

    /// `true`: the construction fixes the energy (chart_reference §0.6).
    fn forbids_energy_normalisation(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "BurrauThetaK"
    }

    /// Layer 2's `θ ∈ (0, π/4]` and `K ≥ 0`, then layer 3 on the f64 decode (the module's note).
    fn validate(&self, u: f64, v: f64) -> ValidationResult {
        if let Some(r) = reject_early(self.theta(u), u, v, Constraint::KNonNegative) {
            return r;
        }
        let t = v.max(0.0);
        match decoded_sanity(&decode_theta_k(self, u, t).0) {
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

/// `D ∘ Φ_θ,K` at `(u, v)`: the embed at `ν(θ(u))`, §2's construction at `L_z = 0`, `K* = K(v)`, its seed
/// [`burrau::seed`]`(ν)`; and the construction's outcome.
#[inline]
pub fn decode_theta_k<R, P>(chart: &BurrauThetaK<P>, u: R, v: R) -> (Decoded<R>, Outcome)
where
    R: LinkReal + PayloadReal + WarpReal + From<P>,
    P: Copy,
{
    let nu = chart.nu(u);
    let mut blocks = BurrauBlocks::new(embed(nu), burrau::seed(nu), R::zero(), chart.kinetic(v));
    let decoded = decode::decode_with(&mut blocks, &unread().z);
    (decoded, blocks.outcome())
}

impl<P: Copy> ChartDecode for BurrauThetaK<P>
where
    f64: From<P>,
{
    /// [`decode_theta_k`]: the construction's outcome is not part of [`Decoded`], as for the `(ν, K)` chart.
    #[inline]
    fn decode_at<S, R>(&self, u: R, v: R) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + WarpReal + From<P>,
    {
        decode_theta_k(self, u, v).0
    }
}

// ── Φ_θ,L_z ─────────────────────────────────────────────────────────────────────────────────────────────────────────

/// `Φ_θ,L_z` (chart_reference §4.5): `θ(u)` on `theta_range`, `K` fixed, `L_z(v) = (2v − 1)·λ_L·L_max(K)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BurrauThetaLz<P> {
    /// `[θ_min, θ_max]`, inside `(0, π/4]`.
    pub theta_range: [P; 2],
    /// The fixed kinetic energy `K > 0`.
    pub k: P,
    /// `λ_L ∈ (0, 1]`: the `L_z` range is `[−λ_L·√(2IK), λ_L·√(2IK)]`.
    pub lz_fraction: P,
}

impl<P: Copy> BurrauThetaLz<P> {
    /// `θ(u) = θ_min + (θ_max − θ_min)·u`.
    #[inline]
    pub fn theta<F: Float + From<P>>(&self, u: F) -> F {
        theta_at(self.theta_range, u)
    }

    /// `ν(θ(u))`.
    #[inline]
    pub fn nu<F: Float + From<P>>(&self, u: F) -> F {
        nu_of(self.theta(u))
    }

    /// The fixed `K`.
    #[inline]
    pub fn kinetic<F: Float + From<P>>(&self) -> F {
        <F as From<P>>::from(self.k)
    }

    /// The band's half-width `λ_L·L_max(K)`, `L_max(K) = √(2IK)` held to `L_max² ≤ 2K` ([`invariant::l_max`]).
    #[inline]
    pub fn lz_max<F: WarpReal + From<P>>(&self) -> F {
        <F as From<P>>::from(self.lz_fraction) * invariant::l_max(self.kinetic::<F>())
    }

    /// `L_z(v) = (2v − 1)·λ_L·L_max(K)`.
    #[inline]
    pub fn lz<F: WarpReal + From<P>>(&self, v: F) -> F {
        (v + v - F::one()) * self.lz_max::<F>()
    }

    /// The convention the chart's systems are stated in: the normalised one (chart_reference §4.2).
    #[inline]
    pub fn convention(&self) -> Convention {
        CONVENTION
    }

    /// [`BurrauThetaLz::convention`] in words.
    #[inline]
    pub fn convention_statement(&self) -> &'static str {
        CONVENTION_STATEMENT
    }

    /// The axes' metadata: `θ` writes the configuration and the masses, `L_z` the momenta; the chart is 2-swept.
    #[inline]
    pub fn axes(&self) -> [AxisMeta; 2] {
        [theta_axis(), vertical_axis(true)]
    }

    /// The chart's `system_image`: bijective, each shape once (the module's note).
    #[inline]
    pub fn system_image(&self) -> SystemImage {
        SystemImage::Bijective
    }
}

impl<P: Copy> BurrauThetaLz<P>
where
    f64: From<P>,
{
    /// The chart's label: its name, bijective, "shape only".
    #[inline]
    pub fn label(&self) -> ChartLabel<'_> {
        fold_label(self.name())
    }

    /// Whether `L_z(v)` is infeasible at the fixed `K`, by the construction's exact test, at f64.
    #[inline]
    fn lz_infeasible(&self, v: f64) -> bool {
        infeasible(1.0, self.kinetic::<f64>(), self.lz(v))
    }

    /// The feasible `v` nearest an infeasible `v` (the module's note): the band's edge on `v`'s side,
    /// `½ ± ½/λ_L`, stepped toward `½` while the exact test still finds it infeasible, eight steps at most; the
    /// clamped `v`, inside `[0, 1]` and so feasible, where those do not suffice. With `λ_L ≤ 1` the upper edge is at
    /// least 1 and the lower at most 0, so each step toward `½` is a step toward 0 ([`WarpReal::below`]).
    fn nearest_feasible(&self, v: f64) -> f64 {
        let half = 0.5;
        let reach = half / <f64 as From<P>>::from(self.lz_fraction);
        let mut edge = select(v > half, half + reach, half - reach);
        for _ in 0..8 {
            if !self.lz_infeasible(edge) {
                return edge;
            }
            edge = edge.below();
        }
        v.clamp(0.0, 1.0)
    }
}

impl<P: Copy> Chart for BurrauThetaLz<P>
where
    f64: From<P>,
{
    type Param = P;

    /// The zero vector, unread (the module's note).
    #[inline]
    fn map<F: Float + From<P>>(&self, _u: F, _v: F) -> ChartOut<F> {
        unread()
    }

    /// `true`: the construction fixes the energy (chart_reference §0.6).
    fn forbids_energy_normalisation(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "BurrauThetaLz"
    }

    /// Layer 2's `θ ∈ (0, π/4]` and `K ≥ L_z²/2I`, then layer 3 on the f64 decode (the module's note).
    fn validate(&self, u: f64, v: f64) -> ValidationResult {
        if let Some(r) = reject_early(self.theta(u), u, v, Constraint::KAboveLzBound) {
            return r;
        }
        let t = select(self.lz_infeasible(v), self.nearest_feasible(v), v);
        match decoded_sanity(&decode_theta_lz(self, u, t).0) {
            Some(failed) => ValidationResult::Reject { failed },
            None if t != v => ValidationResult::Project {
                u,
                v: t,
                failed: Constraint::KAboveLzBound,
            },
            None => ValidationResult::Valid,
        }
    }
}

/// `D ∘ Φ_θ,L_z` at `(u, v)`: the embed at `ν(θ(u))`, §2's construction at `L_z(v)`, `K* = K`, its seed
/// [`burrau::seed`]`(ν)`; and the construction's outcome.
#[inline]
pub fn decode_theta_lz<R, P>(chart: &BurrauThetaLz<P>, u: R, v: R) -> (Decoded<R>, Outcome)
where
    R: LinkReal + PayloadReal + WarpReal + From<P>,
    P: Copy,
{
    let nu = chart.nu(u);
    let mut blocks = BurrauBlocks::new(embed(nu), burrau::seed(nu), chart.lz(v), chart.kinetic());
    let decoded = decode::decode_with(&mut blocks, &unread().z);
    (decoded, blocks.outcome())
}

impl<P: Copy> ChartDecode for BurrauThetaLz<P>
where
    f64: From<P>,
{
    /// [`decode_theta_lz`]: the construction's outcome is not part of [`Decoded`], as for the `(ν, K)` chart.
    #[inline]
    fn decode_at<S, R>(&self, u: R, v: R) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + WarpReal + From<P>,
    {
        decode_theta_lz(self, u, v).0
    }
}

// ── The δm strip ────────────────────────────────────────────────────────────────────────────────────────────────────

/// The δm bifurcation strip `Φ_strip,δm` (chart_reference §4.5): `θ(u)` on `theta_range`, the masses
/// `(1 − v)·m_Burrau(ν(θ(u))) + v·m_target`, at rest.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BurrauThetaDeltaM<P> {
    /// `[θ_min, θ_max]`, inside `(0, π/4]`.
    pub theta_range: [P; 2],
    /// `m_target`, the masses at `v = 1`, summing to 1; equal masses by default ([`BurrauThetaDeltaM::equal_masses`]).
    pub m_target: [P; 3],
}

impl<P: Float> BurrauThetaDeltaM<P> {
    /// The default target, equal masses `(1, 1, 1)/3`, at the parameter type.
    #[inline]
    pub fn equal_masses() -> [P; 3] {
        let one = P::one();
        [one / (one + one + one); 3]
    }
}

impl<P: Copy> BurrauThetaDeltaM<P> {
    /// `θ(u) = θ_min + (θ_max − θ_min)·u`.
    #[inline]
    pub fn theta<F: Float + From<P>>(&self, u: F) -> F {
        theta_at(self.theta_range, u)
    }

    /// `ν(θ(u))`.
    #[inline]
    pub fn nu<F: Float + From<P>>(&self, u: F) -> F {
        nu_of(self.theta(u))
    }

    /// `m(u, v) = (1 − v)·m_Burrau(ν(θ(u))) + v·m_target`, given the Burrau masses `burrau` of `ν(θ(u))`.
    #[inline]
    pub fn blend<F: Float + From<P>>(&self, burrau: [F; 3], v: F) -> [F; 3] {
        let wide = <F as From<P>>::from;
        let keep = F::one() - v;
        let t = self.m_target;
        [
            keep * burrau[0] + v * wide(t[0]),
            keep * burrau[1] + v * wide(t[1]),
            keep * burrau[2] + v * wide(t[2]),
        ]
    }

    /// The masses at `(u, v)`: [`BurrauThetaDeltaM::blend`] of the embed's at `ν(θ(u))`.
    #[inline]
    pub fn masses<F: Float + From<P>>(&self, u: F, v: F) -> [F; 3] {
        self.blend(embed(self.nu(u)).m, v)
    }

    /// The convention the chart's Burrau masses are stated in: the normalised one (chart_reference §4.2).
    #[inline]
    pub fn convention(&self) -> Convention {
        CONVENTION
    }

    /// [`BurrauThetaDeltaM::convention`] in words.
    #[inline]
    pub fn convention_statement(&self) -> &'static str {
        CONVENTION_STATEMENT
    }

    /// The axes' metadata: `θ` writes the configuration and the masses, δm the masses; the momenta are the rest
    /// convention's, so the chart is 2-swept.
    #[inline]
    pub fn axes(&self) -> [AxisMeta; 2] {
        [theta_axis(), vertical_axis(false)]
    }

    /// The chart's `system_image`: bijective, each shape once (the module's note).
    #[inline]
    pub fn system_image(&self) -> SystemImage {
        SystemImage::Bijective
    }
}

impl<P: Copy> BurrauThetaDeltaM<P>
where
    f64: From<P>,
{
    /// The chart's label: its name, bijective, "shape only".
    #[inline]
    pub fn label(&self) -> ChartLabel<'_> {
        fold_label(self.name())
    }
}

/// The Jacobi vectors of `system`'s triangle under the masses `masses` (the module's note): `ρ = r₁ − r₀ = (a/c, 0)`
/// and `λ = r₂ − r₀₁ = (−(m₁/M₀₁)·a/c, b/c)`.
#[inline]
pub fn jacobi<R: Float>(system: &System<R>, masses: &Masses<R>) -> ([R; 2], [R; 2]) {
    let w = masses.m[1] / masses.m01;
    let rho = system.rho;
    (rho, [-(w * rho[0]), system.lambda[1]])
}

/// The δm strip's decoder: the blended masses through the shared entry for a chart that sets masses, the triangle's
/// configuration under them through the shared canonical-frame entry, every momentum 0, and the shared descriptor. No
/// block reads its `z` slice.
pub struct DeltaMBlocks<R> {
    /// The system the embed gave: its triangle.
    system: System<R>,
    /// The blended masses.
    m: [R; 3],
}

impl<R> DeltaMBlocks<R> {
    /// The decoder of `system`'s triangle with the masses `m`, at rest.
    #[inline]
    pub fn new(system: System<R>, m: [R; 3]) -> Self {
        DeltaMBlocks { system, m }
    }
}

impl<R: LinkReal + PayloadReal> Blocks<R> for DeltaMBlocks<R> {
    #[inline]
    fn mass(&mut self, _z_mass: [R; 2]) -> Masses<R> {
        system_masses(&System {
            m: self.m,
            ..self.system
        })
    }

    #[inline]
    fn config(&mut self, _z_cfg: [R; 2], masses: &Masses<R>) -> Config<R> {
        let (rho, lambda) = jacobi(&self.system, masses);
        system_config(
            &System {
                rho,
                lambda,
                ..self.system
            },
            masses,
        )
    }

    /// The rest start: every momentum 0.
    #[inline]
    fn momentum(
        &mut self,
        _z_mom: [R; 4],
        _masses: &Masses<R>,
        _config: &Config<R>,
    ) -> Momentum<R> {
        let zero = R::zero();
        Momentum {
            q: [zero; 4],
            p_rho: [zero; 2],
            p_lambda: [zero; 2],
            p: [[zero; 2]; 3],
        }
    }

    #[inline]
    fn invariants(&mut self, ic: &Ic<R>) -> ICDescriptorOf<R> {
        descriptor::describe(ic)
    }
}

impl<P: Copy> Chart for BurrauThetaDeltaM<P>
where
    f64: From<P>,
{
    type Param = P;

    /// The zero vector, unread (the module's note).
    #[inline]
    fn map<F: Float + From<P>>(&self, _u: F, _v: F) -> ChartOut<F> {
        unread()
    }

    fn name(&self) -> &str {
        "BurrauThetaDeltaM"
    }

    /// [`decode_delta_m`].
    #[inline]
    fn decode<S, R>(&self, u: R, v: R) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<P>,
    {
        decode_delta_m(self, u, v)
    }

    /// Layer 2's `θ ∈ (0, π/4]` and the blended masses' `mᵢ > 0`, then layer 3 on the f64 decode (the module's note).
    fn validate(&self, u: f64, v: f64) -> ValidationResult {
        if let Some(r) = reject_early(self.theta(u), u, v, Constraint::MassPositive) {
            return r;
        }
        if !self.masses(u, v).iter().all(|&m| m > 0.0) {
            return ValidationResult::Reject {
                failed: Constraint::MassPositive,
            };
        }
        match decoded_sanity(&decode_delta_m(self, u, v)) {
            Some(failed) => ValidationResult::Reject { failed },
            None => ValidationResult::Valid,
        }
    }
}

/// `D ∘ Φ_strip,δm` at `(u, v)`: the triangle at `ν(θ(u))` with the masses [`BurrauThetaDeltaM::masses`], at rest.
#[inline]
pub fn decode_delta_m<R, P>(chart: &BurrauThetaDeltaM<P>, u: R, v: R) -> Decoded<R>
where
    R: LinkReal + PayloadReal + From<P>,
    P: Copy,
{
    let system = embed(chart.nu(u));
    let mut blocks = DeltaMBlocks::new(system, chart.blend(system.m, v));
    decode::decode_with(&mut blocks, &unread().z)
}

impl<P: Copy> ChartDecode for BurrauThetaDeltaM<P>
where
    f64: From<P>,
{
    /// The chart's own [`Chart::decode`], [`decode_delta_m`].
    #[inline]
    fn decode_at<S, R>(&self, u: R, v: R) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + WarpReal + From<P>,
    {
        Chart::decode::<S, R>(self, u, v)
    }
}
