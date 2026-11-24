//! The block inverses (inverse-encode contract Part 3; dd_encode §3.1), each with its forward mate in
//! `crate::decode`, on a state `C` has put on the canonical section:
//!
//! ```text
//! Mass      μₖ = log(mₖ/m₀),  z_μk = artanh(μₖ/μ_max), |μₖ| clamped to (1 − ε_μ)·μ_max      (the mass link's inverse)
//! Config    ρ̃ = √μ_ρ·(r₁ − r₀),  λ̃ = √μ_λ·(r₂ − (m₀r₀ + m₁r₁)/M₀₁)                           (dd_decoder §3.2)
//!           α = atan2(‖λ̃‖, ‖ρ̃‖),  β = atan2(λ̃_y, λ̃_x) reduced to [0, 2π)  (R-865, R-898)
//!           s_α = 2α/π, z_α = logit(s_α), s_α clamped to [ε_z, 1 − ε_z];  z_β = β − π/2    (the α and β links' inverses; R-1020)
//! Momentum  p_λ = p₂,  p_ρ = p₁ + (m₁/M₀₁)·p₂;  s = ½(q/q_max + 1), z = logit(s), s clamped to [ε_q, 1 − ε_q]
//! ```
//!
//! **The inverses are the registry's** (dd_encode §2, §4 seam 2/13): each `z` is the selection's link inverse
//! (`crate::generated::links`), never re-derived here, so swapping a link swaps encode's inverse with it. What is written
//! here is the decoder's own Jacobi map run backwards ([`jacobi`]: the forward mates are `config::reconstruct` and
//! `momentum::to_particles`), `α` and `β` ([`alpha`], [`beta`]), and the clamp tests ([`clamps`], [`SimplexClamp`]).
//! `M₀₁` is the mass block's one rounding, `fma(m₁, 1, m₀)` (`mass::from_masses`).
//!
//! **`β`** ([`beta_at`], [`beta`]; dd_encode §3.1, §3.2). `C` has no mirror step, so `λ̃_y` keeps
//! its sign and `β` is the signed angle of `λ̃` from `+x`, reduced to one period, `[0, 2π̂]`: a state above the `x`-axis
//! reads `β ∈ (0, π)`, one below it `β ∈ (π, 2π]`, `π̂` being the registry's image of π (`descriptor::pi_image`) and
//! `τ̂ = 2π̂`, exact.
//!
//! **The case is not read off the floats.** A collinear state's rotated `λ̃_y` is a rounding residue whose sign turns
//! on a backend's `fma` and on whether it flushes subnormals, and the two ends there, 0 and `τ̂`, are one point of the
//! circle but two values. So [`encode_at`](super::encode_at) hands [`beta_at`] the signs `C` decided on its input
//! (`crate::canon::signs_of`: the `Wide` `ρ × Λ` and `ρ · Λ`, the same on every backend), and each case is chosen by
//! those, selecting a value already computed:
//! - `λ̃_y` decided 0 and `λ̃_x` decided below 0, the `−x` half-axis: `π̂`, with no `atan2`;
//! - `λ̃_y` decided 0 elsewhere, the `+x` half-axis and the `α = 0` pole: `+0`, never `atan2(0, 0)`;
//! - `λ̃_y` decided below 0, the lower half: `τ̂ − a`, in `(π, 2π̂]` to rounding;
//! - `λ̃_y` decided above 0: `a` where it is above 0, `π̂` where it is above `π̂` (near the `−x` half-axis an f32
//!   `atan2` that rounds to nearest gives f32's nearest π, above `π̂`; dd_encode §3.1), and `+0` where it is not
//!   above 0.
//!
//! `a = atan2(|λ̃_y|, x′)` is the angle from `+x` on the decided side, in `[0, π]`: `x′` is `|λ̃_x|` with the decided
//! sign, and `+0` where `λ̃_x` is decided 0 or the float is 0 (never `−0`: Metal's `atan2` at its default math mode
//! gives `atan2(y, −0)` the opposite sign of `π/2` to IEEE's, pitfalls §11). So the operands agree with the decision
//! whatever the residues' own signs: a state decided below reads `[π, 2π̂]` even where its float `λ̃_y` is `+tiny`
//! or 0, one decided above reads `[0, π̂]`, and no result lies on the other side of its select. `atan2` is odd in its
//! first operand, bit for bit (`crate::arctan` applies the sign last; the pinned libm's is odd), so off the seam
//! `τ̂ − a` is `atan2(λ̃_y, λ̃_x) + τ̂`'s bits and `a` is `atan2(λ̃_y, λ̃_x)`'s.
//!
//! **A NaN `λ̃`** gives a NaN `β`, by a select on the operands' bits, so `z_β` is NaN and its clamp bit is set
//! ([`clamps`] reads a NaN as outside): a state that is not a number is never encoded to a clean angle.
//!
//! [`beta`] is the same selects on a given `λ̃` alone, its signs the operands' own compared with 0
//! (`LambdaSigns::of_floats`): exact on those operands, for a caller with no state `C` saw. [`invert_at`], given a
//! state and no decision, decides on that state as `C` would (`crate::canon::lambda_signs`).
//!
//! `α = atan2(‖λ̃‖, ‖ρ̃‖)` has no pole on the section, where `‖ρ̃‖² + ‖λ̃‖² = I = 1`; its operands are both 0 only
//! where `C` could not reach `I = 1`, all three bodies coinciding, which keeps `C`'s select (divisor 1, `λ = 1`,
//! `p = 0`, `lookup_clamped`, `φ = +0`).
//!
//! **The clamp tests** (`lookup_clamped`; inverse-encode Parts 3 and 6). A control's bit is set iff its
//! input lay outside what its link's inverse takes unclamped:
//! - **A bounded or symmetric control** (`α`, `β`, the four `q`; [`clamps`]): the input `v` lies outside the closed
//!   range the decode reaches from the clamped controls, `[f(f⁻¹(a)), f(f⁻¹(b))]`, `f` the link's forward, `f⁻¹` its
//!   inverse and `a`, `b` the codomain's ends at `R` ([`Ends`]), each past its clamp. Read off the registry's own
//!   functions, it holds for every link a selection can name; it agrees with the inverse's own clamp to the rounding of
//!   `f ∘ f⁻¹` at the clamp's end, a few ulps. It is a comparison of the input with a constant, not of the clamped `z`
//!   with the clamp's image: a shader compiler may fold the image's `log` at a constant differently from the run time's,
//!   and every clamped `z` sits exactly on the image. So `|q| > q_max` is clamped and flagged, as is
//!   `|q| > q_max·(1 − 2ε_q)`, past σ's clamp.
//! - **The mass** ([`SimplexClamp`]): the simplex's reachable region is two-dimensional, and no comparison of the input
//!   with constants read off the forward bounds it, so each simplex link states its own clamp's argument, as the
//!   registry's inverse forms it, and the test is that argument outside the clamp's interval: for `softmax_tanh`
//!   `log(mₖ/m₀)` against `±(1 − ε_μ)·μ_max` (Part 3), for `stick_breaking` the raw `2a − ε_μ·t` of each `N(a)` against
//!   `c·ε_μ·t`, `c = 1 − ε_μ`, the floor of the clamp dd_generation_root §3.9's form applies. That is the inverse's own
//!   arithmetic at the small side, which is where the edge is resolved. In exact arithmetic `N(v) + N(w) = 2c·t`, so one
//!   `N` passes its floor exactly as the other passes its top `c·(1 + c)·t`; the large `N`'s top alone can also trip a
//!   few ulps short of the edge, moving `z` only by rounding, and is no clamp to flag. The outer clamp of `N(v)/N(w)`
//!   likewise holds an exact corner, which [`guard`] keeps out, and on the open simplex moves only rounding.
//!
//! **The simplex corner** (the registry's note): the simplex inverses divide `0` by `0` at an exact
//! corner, which SPIR-V's `FMax` leaves undefined. So a mass that is not positive is selected as the smallest positive
//! normal before the inverse and its clamp test ([`guard`]): every ratio is then finite, and a mass on an edge or
//! corner is clamped and flagged. The section's masses are positive (inverse-encode Part 6, "Mass simplex").

use spirv_std::num_traits::Float;

use crate::Real;

use crate::canon::{lambda_signs, lambda_t_axis, LambdaSigns};
use crate::decode::descriptor::pi_image;
use crate::decode::mass::from_masses;
use crate::decode::Ic;
use crate::generated::links::{
    Alpha, Beta, Link, LinkParams, LinkReal, Momentum, Selection, SoftmaxTanh, StickBreaking,
};
use crate::real::IeeeBits;

/// `t` where `c` holds, else `f`: both operands are computed before the choice, so it is a select, not a branch.
#[inline]
fn select<T: Copy>(c: bool, t: T, f: T) -> T {
    if c {
        t
    } else {
        f
    }
}

/// `‖v‖ = √fma(v_x, v_x, v_y·v_y)`.
#[inline]
fn norm<R: LinkReal>(v: [R; 2]) -> R {
    Float::sqrt(Float::mul_add(v[0], v[0], v[1] * v[1]))
}

/// The mass-weighted Jacobi vectors and the Jacobi momenta of a canonical state: the decoder's forward maps
/// (`config::reconstruct`, `momentum::to_particles`) run backwards.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Jacobi<R> {
    /// `ρ̃ = √μ_ρ·(r₁ − r₀)`, `μ_ρ = m₀m₁/M₀₁`.
    pub rho_t: [R; 2],
    /// `λ̃ = √μ_λ·(r₂ − (m₀r₀ + m₁r₁)/M₀₁)`, `μ_λ = m₂M₀₁`.
    pub lambda_t: [R; 2],
    /// `p_ρ = p₁ + (m₁/M₀₁)·p₂`.
    pub p_rho: [R; 2],
    /// `p_λ = p₂`.
    pub p_lambda: [R; 2],
}

/// The Jacobi coordinates of `ic` (dd_decoder §3.2, §3.4; inverse-encode Part 3): `ρ = r₁ − r₀`,
/// `λ = r₂ − (m₀r₀ + m₁r₁)/M₀₁`, each weighted by its reduced mass's root, and `p_λ = p₂`, `p_ρ = p₁ + (m₁/M₀₁)p₂`, with
/// no mass unweighting of the momenta. `Σpᵢ = 0` is consumed, not assumed: `p₀` is not read. `λ̃` is
/// [`lambda_t_axis`] on each axis, the arithmetic `C`'s recorded `λ̃_y` reads.
#[inline]
pub fn jacobi<R: LinkReal + IeeeBits>(ic: &Ic<R>) -> Jacobi<R> {
    let [m0, m1, _] = ic.m;
    let m01 = from_masses(ic.m).m01;
    let [r0, r1, r2] = ic.r;
    let rho = [r1[0] - r0[0], r1[1] - r0[1]];
    let sqrt_mu_rho = Float::sqrt(m0 * m1 / m01);
    let w1 = m1 / m01;
    let [_, p1, p2] = ic.p;
    Jacobi {
        rho_t: [sqrt_mu_rho * rho[0], sqrt_mu_rho * rho[1]],
        lambda_t: [
            lambda_t_axis(ic.m, [r0[0], r1[0], r2[0]]),
            lambda_t_axis(ic.m, [r0[1], r1[1], r2[1]]),
        ],
        p_rho: [p1[0] + w1 * p2[0], p1[1] + w1 * p2[1]],
        p_lambda: p2,
    }
}

/// `α = atan2(‖λ̃‖, ‖ρ̃‖)`, on `[0, π/2]` (no pole on the section, the module's note). `atan2` is
/// [`crate::arctan`]'s: at f32 the kernel's own, so the fragment's `α` is the CPU's, bit for bit.
#[inline]
pub fn alpha<R: LinkReal>(rho_t: [R; 2], lambda_t: [R; 2]) -> R {
    Real::arctan2(norm(lambda_t), norm(rho_t))
}

/// `β` of `λ̃` at the decided `signs` of its components (the module's note): `π̂` on the `−x` half-axis and `+0` on the
/// `+x` half-axis and at the `α = 0` pole, with no `atan2`; elsewhere `a = atan2(|λ̃_y|, x′)`, `x′` being `|λ̃_x|` with
/// the decided sign, read as `τ̂ − a` below the axis and as `a`, held to `[+0, π̂]`, above it. A NaN operand gives NaN.
/// `atan2` is [`crate::arctan`]'s, as [`alpha`]'s.
#[inline]
pub fn beta_at<R: LinkReal + IeeeBits>(lambda_t: [R; 2], signs: LambdaSigns) -> R {
    let zero = R::zero();
    let pi = pi_image::<R>();
    let [x, y] = lambda_t;
    let (ax, ay) = (Float::abs(x), Float::abs(y));
    let x_signed = select(signs.x_negative, -ax, ax);
    let x_operand = select(signs.x_zero | (ax == zero), zero, x_signed);
    let a = Real::arctan2(ay, x_operand);
    let upper = select(a > zero, select(a > pi, pi, a), zero);
    let lower = (pi + pi) - a;
    let off_axis = select(signs.y_negative, lower, upper);
    let on_axis = select(signs.x_negative, pi, zero);
    let decided = select(signs.y_zero, on_axis, off_axis);
    // A NaN operand is handed on itself, its bits selected, so no arithmetic has to carry it.
    let nan = select(x.nan_bits(), x, y);
    select(x.nan_bits() | y.nan_bits(), nan, decided)
}

/// `β` of a given `λ̃` alone: [`beta_at`] at the operands' own signs, each compared with 0, which `±0` pass alike. In
/// `[0, 2π̂]`: `atan2(λ̃_y, λ̃_x)`, `+ τ̂` below the `x`-axis.
#[inline]
pub fn beta<R: LinkReal + IeeeBits>(lambda_t: [R; 2]) -> R {
    beta_at(lambda_t, LambdaSigns::of_floats(lambda_t))
}

/// A codomain's two ends at `R`: values at or past its closed range's ends, which the inverse's clamp moves. Any such
/// value gives the same [`clamps`] range, the clamp's image being one value.
pub trait Ends {
    /// `(a, b)`, the lower and the upper end.
    fn ends<R: LinkReal>() -> (R, R);

    /// [`Ends::ends`] at the tunable parameters `p`: a codomain whose ends read none is its own.
    #[inline]
    fn ends_at<R: LinkReal>(_p: &LinkParams<R>) -> (R, R) {
        Self::ends()
    }
}

/// `α`'s `[α_min, π/2 − α_min]` (dd_decoder §3.2), read as `α_min` and the registry's literal `π/2`
/// (`LIT_6`, rounded toward zero at f32), which lies at or past the upper end for every `α_min ≥ 0`.
impl Ends for Alpha {
    #[inline]
    fn ends<R: LinkReal>() -> (R, R) {
        (R::ALPHA_MIN, R::LIT_6)
    }
}

/// `β`'s one period, `[0, τ̂]`, `τ̂ = 2π̂`: the circle has no clamp, and the periodic link, the offset onto it,
/// has no edge, so every `β` [`beta`] reads lies in the range and `z_β`'s clamp bit is never set.
impl Ends for Beta {
    #[inline]
    fn ends<R: LinkReal>() -> (R, R) {
        let pi = pi_image::<R>();
        (R::zero(), pi + pi)
    }
}

/// The momenta's `[−q_max, q_max]`, at the active `q_max`.
impl Ends for Momentum {
    #[inline]
    fn ends<R: LinkReal>() -> (R, R) {
        Self::ends_at(&LinkParams::DEFAULT)
    }

    #[inline]
    fn ends_at<R: LinkReal>(p: &LinkParams<R>) -> (R, R) {
        (-p.q_max, p.q_max)
    }
}

/// Whether the inverse of the 1-to-1 link `L` clamps `v`: `v` outside `[f(f⁻¹(a)), f(f⁻¹(b))]`, the range the decode
/// reaches, `a` and `b` the codomain's [`Ends`] (the module's note). A NaN `v` is outside.
#[inline]
pub fn clamps<L, R>(v: R) -> bool
where
    L: Link<1, 1>,
    L::Codomain: Ends,
    R: LinkReal,
{
    clamps_at::<L, R>(v, &LinkParams::DEFAULT)
}

/// [`clamps`] at the tunable parameters `p`: the ends, the forward and the inverse at the active values, so a momentum
/// past the active `q_max` is clamped and flagged, encode's layer 1 (inverse-encode § "Chart-aware validation").
#[inline]
pub fn clamps_at<L, R>(v: R, p: &LinkParams<R>) -> bool
where
    L: Link<1, 1>,
    L::Codomain: Ends,
    R: LinkReal,
{
    let (a, b) = <L::Codomain as Ends>::ends_at::<R>(p);
    let [lo] = L::forward_at(L::inverse_at([a], p), p);
    let [hi] = L::forward_at(L::inverse_at([b], p), p);
    !((v >= lo) & (v <= hi))
}

/// `c` outside `[−cap, cap]`, a NaN outside.
#[inline]
fn outside<R: LinkReal>(c: R, cap: R) -> bool {
    !((c >= -cap) & (c <= cap))
}

/// A simplex link's clamp test: bit `k` set iff its inverse clamps the argument of its `k`-th control at `m` (the
/// module's note). `m` is positive, as [`guard`] leaves it.
pub trait SimplexClamp: Link<2, 3> {
    /// The two controls' clamp bits.
    fn clamps<R: LinkReal>(m: [R; 3]) -> u32;

    /// [`SimplexClamp::clamps`] at the tunable parameters `p`: a link whose clamp reads none is its own.
    #[inline]
    fn clamps_at<R: LinkReal>(m: [R; 3], _p: &LinkParams<R>) -> u32 {
        Self::clamps(m)
    }
}

/// `μₖ = log(mₖ/m₀)` against `±(1 − ε_μ)·μ_max` (inverse-encode Part 3), as the registry's inverse forms both, at the
/// active `μ_max`.
impl SimplexClamp for SoftmaxTanh {
    #[inline]
    fn clamps<R: LinkReal>(m: [R; 3]) -> u32 {
        Self::clamps_at(m, &LinkParams::DEFAULT)
    }

    #[inline]
    fn clamps_at<R: LinkReal>(m: [R; 3], p: &LinkParams<R>) -> u32 {
        let cap = (R::one() - R::EPS_MU) * p.mu_max;
        let mu1 = Real::logarithm(m[1] / m[0]);
        let mu2 = Real::logarithm(m[2] / m[0]);
        select(outside(mu1, cap), 1, 0) | select(outside(mu2, cap), 2, 0)
    }
}

/// Whether `stick_breaking`'s inverse clamps `(v, w)`: `2a − ε_μ·t` of `v` or of `w` below `c·ε_μ·t`, `t = v + w`,
/// `c = 1 − ε_μ`, each operation in the order the registry's tree does it (dd_generation_root §3.9's list of forms; the
/// module's note says why the floor is the edge).
#[inline]
fn stick_clamps<R: LinkReal>(v: R, w: R) -> bool {
    let two = R::one() + R::one();
    let c = R::one() - R::EPS_MU;
    let t = v + w;
    let shift = R::EPS_MU * t;
    let floor = c * R::EPS_MU * t;
    !(((two * v - shift) >= floor) & ((two * w - shift) >= floor))
}

/// Each of the two clamps of the registry's stick-breaking entry, as its inverse forms them: `(v, w) = (m₁ + m₂, m₀)`,
/// then `(m₂, m₁)`.
impl SimplexClamp for StickBreaking {
    #[inline]
    fn clamps<R: LinkReal>(m: [R; 3]) -> u32 {
        let first = stick_clamps(m[1] + m[2], m[0]);
        let second = stick_clamps(m[2], m[1]);
        select(first, 1, 0) | select(second, 2, 0)
    }
}

/// `m` with each mass that is not positive, a NaN included, selected as the smallest positive normal, so the simplex
/// inverse never meets `0/0` (the module's note).
#[inline]
pub fn guard<R: LinkReal>(m: [R; 3]) -> [R; 3] {
    let tiny = R::min_positive_value();
    [
        select(m[0] > R::zero(), m[0], tiny),
        select(m[1] > R::zero(), m[1], tiny),
        select(m[2] > R::zero(), m[2], tiny),
    ]
}

/// The block inverses' output: the eight controls, the intermediates they were read from, and the clamp bits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Inverted<R> {
    /// `z`, in the latent block layout `z_α, z_β | z_q0 … z_q3 | z_μ1, z_μ2`.
    pub z: [R; 8],
    /// `α`.
    pub alpha: R,
    /// `β`.
    pub beta: R,
    /// The Jacobi coordinates the config and momentum inverses read.
    pub jacobi: Jacobi<R>,
    /// Bit `k` set iff `z[k]`'s inverse clamped.
    pub clamped: u32,
}

/// The block inverses of the canonical `ic` under the selection `S`, block by block: config `z[0:2]`, momentum
/// `z[2:6]`, mass `z[6:8]`, each through its link's inverse.
#[inline]
pub fn invert<S, R>(ic: &Ic<R>) -> Inverted<R>
where
    S: Selection,
    S::MassLink: SimplexClamp,
    R: LinkReal + IeeeBits,
{
    invert_at::<S, R>(ic, &LinkParams::DEFAULT)
}

/// [`invert`] at the tunable parameters `p`: each inverse and each clamp test at the active `μ_max` and `q_max`.
/// `β`'s case is decided on `ic` itself, as `C` decides it on its input
/// (`crate::canon::lambda_signs`); [`invert_decided_at`] takes a decision already made.
#[inline]
pub fn invert_at<S, R>(ic: &Ic<R>, p: &LinkParams<R>) -> Inverted<R>
where
    S: Selection,
    S::MassLink: SimplexClamp,
    R: LinkReal + IeeeBits,
{
    invert_decided_at::<S, R>(ic, lambda_signs(ic.m, ic.r), p)
}

/// [`invert_at`] with `β`'s case given: `signs` are the signs of `λ̃` that `C` decided on the state it canonicalised
/// to `ic` (`crate::canon::Canonical::signs`), the same on every backend, where `ic`'s own rotated `λ̃` is not.
#[inline]
pub fn invert_decided_at<S, R>(ic: &Ic<R>, signs: LambdaSigns, p: &LinkParams<R>) -> Inverted<R>
where
    S: Selection,
    S::MassLink: SimplexClamp,
    R: LinkReal + IeeeBits,
{
    let j = jacobi(ic);
    let a = alpha(j.rho_t, j.lambda_t);
    let b = beta_at(j.lambda_t, signs);
    let [z_alpha] = <S::AlphaLink as Link<1, 1>>::inverse_at([a], p);
    let [z_beta] = <S::BetaLink as Link<1, 1>>::inverse_at([b], p);
    let q = [j.p_rho[0], j.p_rho[1], j.p_lambda[0], j.p_lambda[1]];
    let [z_q0] = <S::Q0Link as Link<1, 1>>::inverse_at([q[0]], p);
    let [z_q1] = <S::Q1Link as Link<1, 1>>::inverse_at([q[1]], p);
    let [z_q2] = <S::Q2Link as Link<1, 1>>::inverse_at([q[2]], p);
    let [z_q3] = <S::Q3Link as Link<1, 1>>::inverse_at([q[3]], p);
    let m = guard(ic.m);
    let [z_mu1, z_mu2] = <S::MassLink as Link<2, 3>>::inverse_at(m, p);
    let bit = |c: bool, k: u32| select(c, 1u32 << k, 0);
    let clamped = bit(clamps_at::<S::AlphaLink, R>(a, p), 0)
        | bit(clamps_at::<S::BetaLink, R>(b, p), 1)
        | bit(clamps_at::<S::Q0Link, R>(q[0], p), 2)
        | bit(clamps_at::<S::Q1Link, R>(q[1], p), 3)
        | bit(clamps_at::<S::Q2Link, R>(q[2], p), 4)
        | bit(clamps_at::<S::Q3Link, R>(q[3], p), 5)
        | (<S::MassLink as SimplexClamp>::clamps_at(m, p) << 6);
    Inverted {
        z: [z_alpha, z_beta, z_q0, z_q1, z_q2, z_q3, z_mu1, z_mu2],
        alpha: a,
        beta: b,
        jacobi: j,
        clamped,
    }
}
