//! The block inverses (inverse-encode contract Part 3; dd_encode §3.1; REQ-ENC-007), each with its forward mate in
//! `crate::decode`, on a state `C` has put on the canonical section:
//!
//! ```text
//! Mass      μₖ = log(mₖ/m₀),  z_μk = artanh(μₖ/μ_max), |μₖ| clamped to (1 − ε_μ)·μ_max      (the mass link's inverse)
//! Config    ρ̃ = √μ_ρ·(r₁ − r₀),  λ̃ = √μ_λ·(r₂ − (m₀r₀ + m₁r₁)/M₀₁)                           (dd_decoder §3.2)
//!           α = atan2(‖λ̃‖, ‖ρ̃‖),  β = atan2(|λ̃_y|, λ̃_x) ∈ [0, π]  (R-438)
//!           s_α = 2α/π, s_β = β/π, z = logit(s), s clamped to [ε_z, 1 − ε_z]                  (the α and β links' inverses)
//! Momentum  p_λ = p₂,  p_ρ = p₁ + (m₁/M₀₁)·p₂;  s = ½(q/q_max + 1), z = logit(s), s clamped to [ε_q, 1 − ε_q]
//! ```
//!
//! **The inverses are the registry's** (dd_encode §2, §4 seam 2/13): each `z` is the selection's link inverse
//! (`crate::generated::links`), never re-derived here, so swapping a link swaps encode's inverse with it. What is written
//! here is the decoder's own Jacobi map run backwards ([`jacobi`]: the forward mates are `config::reconstruct` and
//! `momentum::to_particles`), `α` and `β` ([`alpha`], [`beta`]), and the clamp tests ([`clamps`], [`SimplexClamp`]).
//! `M₀₁` is the mass block's one rounding, `fma(m₁, 1, m₀)` (`mass::from_masses`).
//!
//! **`β`** ([`beta`]; R-437, R-438; dd_encode §3.1). `β` reads `|λ̃_y|`, by `abs`, never `max` (`FMax` returns `−0` for
//! `max(−0, +0)`): `C`'s deadband leaves `λ̃_y ∈ [−δ_λ, 0]` unmirrored (R-82), and `β ∈ [0, π]` holds because `β` reads
//! `|λ̃_y|`. Each case is chosen by a comparison with a constant (0, or `π̂` for the third), selecting a value already
//! computed:
//! - `λ̃_y` compares equal to 0 (true for `±0`) and `λ̃_x < 0`, the `−x` half-axis: `π̂`, R-417's image of π
//!   (`descriptor::pi_image`), since a GPU may ignore a zero's sign through `atan2` and give `−π` there;
//! - `λ̃_x = 0` and `λ̃_y = 0`, the `α = 0` pole: 0, never `atan2(0, 0)`, which GLSL.std.450 leaves undefined;
//! - `atan2(|λ̃_y|, λ̃_x)` above `π̂`: `π̂`. Near the `−x` half-axis the true `β` is just below π, and an f32 `atan2`
//!   that rounds to nearest (glibc's `atan2f`; Apple's returns `π̂` there) gives f32's nearest π, above `π̂` (C8/Q5;
//!   dd_encode §3.1);
//! - elsewhere `atan2(|λ̃_y|, λ̃_x)`, a zero `λ̃_x` taken as `+0` by a select on it against 0: Metal's `atan2` at its
//!   default math mode gives `atan2(y, −0)` the opposite sign of `π/2` to IEEE's (pitfalls §11). Where `λ̃_y ≠ 0`
//!   IEEE's `atan2(|λ̃_y|, ±0)` is one value, and at the pole the pole's select discards it, so the CPU's `β` is
//!   unchanged.
//!
//! The two zero-selected cases are disjoint (`λ̃_x < 0` against `λ̃_x = 0`). `α = atan2(‖λ̃‖, ‖ρ̃‖)` has no pole on the
//! section, where `‖ρ̃‖² + ‖λ̃‖² = I = 1`; its operands are both 0 only where `C` could not reach `I = 1`, all three
//! bodies coinciding, whose output RQ-296 holds.
//!
//! **The clamp tests** (`lookup_clamped`, REQ-ENC-014; inverse-encode Parts 3 and 6). A control's bit is set iff its
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
//!   `log(mₖ/m₀)` against `±(1 − ε_μ)·μ_max` (Part 3), for `stick_breaking` `2(m₁ + m₂) − 1` and `2m₂/(m₁ + m₂) − 1`
//!   against `±(1 − ε_μ)²`. The argument is the inverse's own arithmetic, so the test is the clamp's, bit for bit.
//!
//! **The simplex corner** (registry note, TASK-M2-01 qa (c)): the simplex inverses divide `0` by `0` at an exact
//! corner, which SPIR-V's `FMax` leaves undefined. So a mass that is not positive is selected as the smallest positive
//! normal before the inverse and its clamp test ([`guard`]): every ratio is then finite, and a mass on an edge or
//! corner is clamped and flagged. The section's masses are positive (inverse-encode Part 6, "Mass simplex").

use spirv_std::num_traits::Float;

use crate::canon::lambda_t_axis;
use crate::decode::descriptor::pi_image;
use crate::decode::mass::from_masses;
use crate::decode::Ic;
use crate::generated::links::{
    Alpha, Beta, Link, LinkReal, Momentum, Selection, SoftmaxTanh, StickBreaking,
};

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
/// no mass unweighting of the momenta (R-28). `Σpᵢ = 0` is consumed, not assumed: `p₀` is not read. `λ̃` is
/// [`lambda_t_axis`] on each axis, the arithmetic `C`'s mirror decision reads.
#[inline]
pub fn jacobi<R: LinkReal>(ic: &Ic<R>) -> Jacobi<R> {
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

/// `α = atan2(‖λ̃‖, ‖ρ̃‖)`, on `[0, π/2]` (no pole on the section, the module's note).
#[inline]
pub fn alpha<R: LinkReal>(rho_t: [R; 2], lambda_t: [R; 2]) -> R {
    Float::atan2(norm(lambda_t), norm(rho_t))
}

/// `β = atan2(|λ̃_y|, λ̃_x)` (R-438), with `π̂` on the `−x` half-axis and 0 at the `α = 0` pole, each selected by a
/// comparison with 0 that `±0` pass alike (R-437), and `π̂` wherever `atan2` returns more, so `β ∈ [0, π̂]` (the
/// module's note).
#[inline]
pub fn beta<R: LinkReal>(lambda_t: [R; 2]) -> R {
    let zero = R::zero();
    let pi = pi_image::<R>();
    let [x, y] = lambda_t;
    let x0 = select(x == zero, zero, x);
    let raw = Float::atan2(Float::abs(y), x0);
    let off_axis = select(raw > pi, pi, raw);
    let half_axis = (y == zero) & (x < zero);
    let pole = (x == zero) & (y == zero);
    select(half_axis, pi, select(pole, zero, off_axis))
}

/// A codomain's two ends at `R`: values at or past its closed range's ends, which the inverse's clamp moves. Any such
/// value gives the same [`clamps`] range, the clamp's image being one value.
pub trait Ends {
    /// `(a, b)`, the lower and the upper end.
    fn ends<R: LinkReal>() -> (R, R);
}

/// `α`'s `[α_min, π/2 − α_min]` (dd_decoder §3.2, R-21), read as `α_min` and the registry's literal `π/2`
/// (`LIT_6`, R-417's image, rounded toward zero at f32), which lies at or past the upper end for every `α_min ≥ 0`.
impl Ends for Alpha {
    #[inline]
    fn ends<R: LinkReal>() -> (R, R) {
        (R::ALPHA_MIN, R::LIT_6)
    }
}

/// `β`'s `[0, π̂]`.
impl Ends for Beta {
    #[inline]
    fn ends<R: LinkReal>() -> (R, R) {
        (R::zero(), pi_image::<R>())
    }
}

/// The momenta's `[−q_max, q_max]` (R-10).
impl Ends for Momentum {
    #[inline]
    fn ends<R: LinkReal>() -> (R, R) {
        (-R::Q_MAX, R::Q_MAX)
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
    let (a, b) = <L::Codomain as Ends>::ends::<R>();
    let [lo] = L::forward(L::inverse([a]));
    let [hi] = L::forward(L::inverse([b]));
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
}

/// `μₖ = log(mₖ/m₀)` against `±(1 − ε_μ)·μ_max` (inverse-encode Part 3), as the registry's inverse forms both.
impl SimplexClamp for SoftmaxTanh {
    #[inline]
    fn clamps<R: LinkReal>(m: [R; 3]) -> u32 {
        let cap = (R::one() - R::EPS_MU) * R::MU_MAX;
        let mu1 = Float::ln(m[1] / m[0]);
        let mu2 = Float::ln(m[2] / m[0]);
        select(outside(mu1, cap), 1, 0) | select(outside(mu2, cap), 2, 0)
    }
}

/// `2(m₁ + m₂) − 1` and `2m₂/(m₁ + m₂) − 1` against `±(1 − ε_μ)²` (the registry's R-72 entry), as its inverse forms
/// them.
impl SimplexClamp for StickBreaking {
    #[inline]
    fn clamps<R: LinkReal>(m: [R; 3]) -> u32 {
        let two = R::one() + R::one();
        let keep = R::one() - R::EPS_MU;
        let cap = keep * keep;
        let stick = m[1] + m[2];
        let s = two * stick - R::one();
        let u = two * (m[2] / stick) - R::one();
        select(outside(s, cap), 1, 0) | select(outside(u, cap), 2, 0)
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
    R: LinkReal,
{
    let j = jacobi(ic);
    let a = alpha(j.rho_t, j.lambda_t);
    let b = beta(j.lambda_t);
    let [z_alpha] = <S::AlphaLink as Link<1, 1>>::inverse([a]);
    let [z_beta] = <S::BetaLink as Link<1, 1>>::inverse([b]);
    let q = [j.p_rho[0], j.p_rho[1], j.p_lambda[0], j.p_lambda[1]];
    let [z_q0] = <S::Q0Link as Link<1, 1>>::inverse([q[0]]);
    let [z_q1] = <S::Q1Link as Link<1, 1>>::inverse([q[1]]);
    let [z_q2] = <S::Q2Link as Link<1, 1>>::inverse([q[2]]);
    let [z_q3] = <S::Q3Link as Link<1, 1>>::inverse([q[3]]);
    let m = guard(ic.m);
    let [z_mu1, z_mu2] = <S::MassLink as Link<2, 3>>::inverse(m);
    let bit = |c: bool, k: u32| select(c, 1u32 << k, 0);
    let clamped = bit(clamps::<S::AlphaLink, R>(a), 0)
        | bit(clamps::<S::BetaLink, R>(b), 1)
        | bit(clamps::<S::Q0Link, R>(q[0]), 2)
        | bit(clamps::<S::Q1Link, R>(q[1]), 3)
        | bit(clamps::<S::Q2Link, R>(q[2]), 4)
        | bit(clamps::<S::Q3Link, R>(q[3]), 5)
        | (<S::MassLink as SimplexClamp>::clamps(m) << 6);
    Inverted {
        z: [z_alpha, z_beta, z_q0, z_q1, z_q2, z_q3, z_mu1, z_mu2],
        alpha: a,
        beta: b,
        jacobi: j,
        clamped,
    }
}
