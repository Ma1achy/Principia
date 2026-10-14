//! The `ICDescriptor`, what the IC *is* (dd_decoder §3.6; dd_generation_root §3.6; REQ-PAY-034): filled at the decode
//! stage, post-decode and pre-integration, from the decoded `(m, r, p)` alone, so every chart's IC is described alike
//! (dd_decoder §2: the canonical `(m, r, p)` is the one seam). Its twelve fields, in the generated struct's order:
//!
//! ```text
//! m0 m1 m2       the masses
//! q_mass         min(m₀, m₁)/max(m₀, m₁)
//! rho_mag        ‖ρ̃‖ = √μ_ρ·‖ρ‖,  ρ = r₁ − r₀,                       μ_ρ = m₀m₁/M₀₁
//! lambda_mag     ‖λ̃‖ = √μ_λ·‖λ‖,  λ = r₂ − (m₀r₀ + m₁r₁)/M₀₁,        μ_λ = m₂M₀₁
//! rho_ratio      ‖ρ̃‖/‖λ̃‖
//! rho_angle      atan2(ρ × λ, ρ · λ), radians on (−π, π]
//! K_0            Σᵢ ‖pᵢ‖²/2mᵢ
//! V_0            −Σ_{i<j} mᵢmⱼ/‖rᵢ − rⱼ‖                 (G = 1)
//! virial_ratio   2K₀/|V₀|
//! r_min_pair_0   min_{i<j} ‖rᵢ − rⱼ‖
//! ```
//!
//! The six fields after the masses are REQ-PAY-088's definitions, written into dd_decoder §3.6 by this task (R-72).
//! `E₀` is derived at read, `K_0 + V_0` ([`energy`]), never stored (R-86). The struct follows `Real` (R-313): the
//! f32 instantiation is the GPU's 64 B, the f64 the CPU's 112 B; `_pad` is declared padding, written zero.
//!
//! `2mᵢ` and `2K₀` are formed as the sums `mᵢ + mᵢ` and `K₀ + K₀`, which are exact at any `Real`.

use spirv_std::num_traits::Float;

use crate::decode::Ic;
use crate::generated::links::LinkReal;
use crate::payload::{ICDescriptorOf, PayloadReal};

/// `a − b`.
#[inline]
fn sub<R: LinkReal>(a: [R; 2], b: [R; 2]) -> [R; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

/// `‖a‖ = √(a_x² + a_y²)`.
#[inline]
fn norm<R: LinkReal>(a: [R; 2]) -> R {
    Float::sqrt(a[0] * a[0] + a[1] * a[1])
}

/// The kinetic energy `K₀ = Σᵢ ‖pᵢ‖²/2mᵢ`.
#[inline]
pub fn kinetic<R: LinkReal>(ic: &Ic<R>) -> R {
    let term = |p: [R; 2], m: R| (p[0] * p[0] + p[1] * p[1]) / (m + m);
    term(ic.p[0], ic.m[0]) + term(ic.p[1], ic.m[1]) + term(ic.p[2], ic.m[2])
}

/// The potential energy `V₀ = −Σ_{i<j} mᵢmⱼ/‖rᵢ − rⱼ‖`, `G = 1`.
#[inline]
pub fn potential<R: LinkReal>(ic: &Ic<R>) -> R {
    let [r0, r1, r2] = ic.r;
    let [m0, m1, m2] = ic.m;
    let pair = |a: [R; 2], b: [R; 2], ma: R, mb: R| ma * mb / norm(sub(a, b));
    -(pair(r0, r1, m0, m1) + pair(r0, r2, m0, m2) + pair(r1, r2, m1, m2))
}

/// The derived `E₀ = K_0 + V_0`, formed at `Real` width from the stored fields (R-86; dd_generation_root §3.6).
#[inline]
pub fn energy<R: LinkReal + PayloadReal>(d: &ICDescriptorOf<R>) -> R {
    d.K_0 + d.V_0
}

/// The `ICDescriptor` of the decoded IC `ic` (dd_decoder §3.6).
#[inline]
pub fn describe<R: LinkReal + PayloadReal>(ic: &Ic<R>) -> ICDescriptorOf<R> {
    let [m0, m1, m2] = ic.m;
    let [r0, r1, r2] = ic.r;
    let m01 = m0 + m1;
    let rho = sub(r1, r0);
    let inner = [
        (m0 * r0[0] + m1 * r1[0]) / m01,
        (m0 * r0[1] + m1 * r1[1]) / m01,
    ];
    let lambda = sub(r2, inner);
    let rho_mag = Float::sqrt(m0 * m1 / m01) * norm(rho);
    let lambda_mag = Float::sqrt(m2 * m01) * norm(lambda);
    let cross = rho[0] * lambda[1] - rho[1] * lambda[0];
    let dot = rho[0] * lambda[0] + rho[1] * lambda[1];
    let k = kinetic(ic);
    let v = potential(ic);
    let r_min = Float::min(
        Float::min(norm(sub(r1, r2)), norm(sub(r2, r0))),
        norm(sub(r0, r1)),
    );
    ICDescriptorOf {
        m0,
        m1,
        m2,
        q_mass: Float::min(m0, m1) / Float::max(m0, m1),
        rho_mag,
        lambda_mag,
        rho_ratio: rho_mag / lambda_mag,
        rho_angle: Float::atan2(cross, dot),
        K_0: k,
        V_0: v,
        virial_ratio: (k + k) / Float::abs(v),
        r_min_pair_0: r_min,
        _pad: Default::default(),
        _tail: Default::default(),
    }
}
