//! The state the integrator starts from, as the compass's physical views read it (R-550; deep_zoom §2, "The latent
//! Jacobian"; dd_decoder §3.2): the decoder's CPU f64 path at a latent point, in the canonical frame and the scale gauge
//! it decodes to, read after `η_E`. A pure function of its arguments, crossing nothing: the GUI differentiates it through
//! the contract, the only part of the engine it may read (R-185, R-390).

use kernel::decode::{energy_norm, Latent as KernelLatent};
use kernel::generated::links::Defaults;

use super::sim_config::{Latent, Links};

/// The decoded state at a latent point, after canonicalisation, the scale gauge and `η_E`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    /// The masses `m₀, m₁, m₂`, `M = 1`.
    pub m: [f64; 3],
    /// The configuration angle `α`, as the decoder writes it.
    pub alpha: f64,
    /// The configuration angle `β`, as the decoder writes it.
    pub beta: f64,
    /// The mass-weighted inner Jacobi vector `ρ̃`.
    pub rho_t: [f64; 2],
    /// The mass-weighted outer Jacobi vector `λ̃`.
    pub lambda_t: [f64; 2],
    /// The Jacobi momentum `p_ρ`, after `η_E`.
    pub p_rho: [f64; 2],
    /// The Jacobi momentum `p_λ`, after `η_E`.
    pub p_lambda: [f64; 2],
    /// The angular momentum `L_z = Σ rᵢ × pᵢ`, after `η_E`.
    pub l_z: f64,
}

/// The decoded state at `z` under `links`' tunable parameters with the energy target `e_star` (`None` is off, R-25).
/// The link ids are the defaults': a selection is a compile-time type (`kernel::generated::links::Selection`), and no
/// surface of the dev GUI chooses another yet.
pub fn state(z: &Latent, links: &Links, e_star: Option<f64>) -> State {
    let params = links.link_params();
    let (decoded, _) =
        energy_norm::decode_at::<Defaults, f64>(&KernelLatent::new(*z), e_star, &params);
    let (m, r, p) = (decoded.ic.m, decoded.ic.r, decoded.ic.p);
    let m01 = m[0] + m[1];
    // chart_reference §0.3's map inverted on the state after η_E: p₁·m₀ − p₀·m₁ = M₀₁·p_ρ, and p₂ = p_λ.
    let p_rho = [
        (m[0] * p[1][0] - m[1] * p[0][0]) / m01,
        (m[0] * p[1][1] - m[1] * p[0][1]) / m01,
    ];
    let l_z = (0..3).map(|i| r[i][0] * p[i][1] - r[i][1] * p[i][0]).sum();
    State {
        m,
        alpha: decoded.config.alpha,
        beta: decoded.config.beta,
        rho_t: decoded.config.rho_t,
        lambda_t: decoded.config.lambda_t,
        p_rho,
        p_lambda: p[2],
        l_z,
    }
}
