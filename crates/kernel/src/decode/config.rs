//! `D_cfg`, the configuration block (dd_decoder §3.2; chart_reference §0.2; REQ-DEC-012, REQ-DEC-013, REQ-DEC-015):
//! the two controls `z[0:2]` through the selection's α and β links, the canonical-frame hyperspherical decode, then
//! unweighting and reconstruction. Under the default links, `α = (π/2)·σ(z_α)` (`α_min = 0`, R-21) and
//! `β = π·σ(z_β)`.
//!
//! **Canonical frame** (rotation and scale gauged by construction, so no latent coordinate is spent on them;
//! REQ-SYS-010): `ρ̃ = R̃ cos α·(1, 0)`, `λ̃ = R̃ sin α·(cos β, sin β)`, `R̃ = 1`, so `I = ‖ρ̃‖² + ‖λ̃‖² = 1`.
//! `‖ρ̃‖ = cos α`: **small α is a large inner-pair separation**, and `α → π/2` a tight inner pair with a distant third
//! body (dd_decoder §3.2's orientation gotcha; decoder test 9).
//!
//! **Unweight and reconstruct** (`M = 1`): `μ_ρ = m₀m₁/M₀₁`, `μ_λ = m₂M₀₁`, `ρ = ρ̃/√μ_ρ`, `λ = λ̃/√μ_λ`;
//! `r₀₁ = −m₂λ`, `r₂ = M₀₁λ`, `r₀ = r₀₁ − (m₁/M₀₁)ρ`, `r₁ = r₀₁ + (m₀/M₀₁)ρ`. Then `r₁ − r₀ = ρ`, the total CoM
//! `M₀₁r₀₁ + m₂r₂ = 0` identically, and `I = Σmᵢ‖rᵢ‖² = 1`. `M₀₁` is the mass block's, the same rounded sum its tag
//! tested.
//!
//! **What is a link here** (REQ-CHART-033): α and β come only from the selection's generated links. The block's own
//! `cos α`, `sin α`, `cos β`, `sin β` and the unweighting's `√μ_ρ`, `√μ_λ` are the decoder's formula (§3.2), not
//! registry links: they map no control onto a constrained parameter, and no link entry or inverse is re-derived by
//! them.
//!
//! **No guard** (REQ-DEC-028): finite `z` keeps `α` off the exact poles in exact arithmetic; nothing here clamps,
//! excludes or lands on them, and the poles are fenced downstream (the collision detector, the conditioning readout,
//! the saturation flags; R-21).

use spirv_std::num_traits::Float;

use crate::decode::mass::Masses;
use crate::generated::links::{Link, LinkReal, Selection};

/// The configuration block's output.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config<R> {
    /// The shape-sphere angle α, `‖ρ̃‖ = cos α`.
    pub alpha: R,
    /// The azimuth β, `λ̃`'s angle from `+x`.
    pub beta: R,
    /// The mass-weighted inner Jacobi vector `ρ̃`, on `+x`.
    pub rho_t: [R; 2],
    /// The mass-weighted outer Jacobi vector `λ̃`.
    pub lambda_t: [R; 2],
    /// The unweighted inner Jacobi vector `ρ = r₁ − r₀`.
    pub rho: [R; 2],
    /// The unweighted outer Jacobi vector `λ = r₂ − r₀₁`.
    pub lambda: [R; 2],
    /// The bodies' positions `r₀, r₁, r₂`, CoM at the origin.
    pub r: [[R; 2]; 3],
}

/// `(α, β)` from `(z_α, z_β)`, each through the selection's link.
#[inline]
pub fn angles<S: Selection, R: LinkReal>(z_cfg: [R; 2]) -> (R, R) {
    let [alpha] = <S::AlphaLink as Link<1, 1>>::forward([z_cfg[0]]);
    let [beta] = <S::BetaLink as Link<1, 1>>::forward([z_cfg[1]]);
    (alpha, beta)
}

/// The canonical-frame decode at `R̃ = 1`: `ρ̃ = cos α·(1, 0)` and `λ̃ = sin α·(cos β, sin β)`.
#[inline]
pub fn hyperspherical<R: LinkReal>(alpha: R, beta: R) -> ([R; 2], [R; 2]) {
    let sin_alpha = Float::sin(alpha);
    (
        [Float::cos(alpha), R::zero()],
        [sin_alpha * Float::cos(beta), sin_alpha * Float::sin(beta)],
    )
}

/// Unweights `ρ̃, λ̃` by the masses' reduced masses and reconstructs the positions: `(ρ, λ, r)`.
#[inline]
pub fn reconstruct<R: LinkReal>(
    masses: &Masses<R>,
    rho_t: [R; 2],
    lambda_t: [R; 2],
) -> ([R; 2], [R; 2], [[R; 2]; 3]) {
    let [m0, m1, m2] = masses.m;
    let m01 = masses.m01;
    let sqrt_mu_rho = Float::sqrt(m0 * m1 / m01);
    let sqrt_mu_lambda = Float::sqrt(m2 * m01);
    let rho = [rho_t[0] / sqrt_mu_rho, rho_t[1] / sqrt_mu_rho];
    let lambda = [lambda_t[0] / sqrt_mu_lambda, lambda_t[1] / sqrt_mu_lambda];
    let r01 = [-(m2 * lambda[0]), -(m2 * lambda[1])];
    let w0 = m1 / m01;
    let w1 = m0 / m01;
    let r = [
        [r01[0] - w0 * rho[0], r01[1] - w0 * rho[1]],
        [r01[0] + w1 * rho[0], r01[1] + w1 * rho[1]],
        [m01 * lambda[0], m01 * lambda[1]],
    ];
    (rho, lambda, r)
}

/// `D_cfg`: the config controls `z[0:2]`, with the mass block's output, to the canonical-frame configuration.
#[inline]
pub fn decode<S: Selection, R: LinkReal>(z_cfg: [R; 2], masses: &Masses<R>) -> Config<R> {
    let (alpha, beta) = angles::<S, R>(z_cfg);
    let (rho_t, lambda_t) = hyperspherical(alpha, beta);
    let (rho, lambda, r) = reconstruct(masses, rho_t, lambda_t);
    Config {
        alpha,
        beta,
        rho_t,
        lambda_t,
        rho,
        lambda,
        r,
    }
}
