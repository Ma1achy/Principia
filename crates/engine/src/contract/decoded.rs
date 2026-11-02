//! The state the integrator starts from, as the compass's physical views read it (R-550; deep_zoom §2, "The latent
//! Jacobian"; dd_decoder §3.2): the decoder's CPU f64 path at a latent point, in the canonical frame and the scale gauge
//! it decodes to, read after `η_E`. A pure function of its arguments, crossing nothing: the GUI differentiates it through
//! the contract, the only part of the engine it may read (R-185, R-390).

use kernel::decode::{energy_norm, Latent as KernelLatent};
use kernel::encode::blocks::jacobi;
use kernel::generated::links::Defaults;
use kernel::physics::energy::angular_momentum;
use kernel::shape::shape_vec;

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
    // chart_reference §0.3's map inverted on the state after η_E, by the encode's own inverse.
    let momenta = jacobi(&decoded.ic);
    State {
        m: decoded.ic.m,
        alpha: decoded.config.alpha,
        beta: decoded.config.beta,
        rho_t: decoded.config.rho_t,
        lambda_t: decoded.config.lambda_t,
        p_rho: momenta.p_rho,
        p_lambda: momenta.p_lambda,
        l_z: angular_momentum(&decoded.ic.r, &decoded.ic.p),
    }
}

/// The 2-D dot product.
fn dot2(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

/// The 2-D cross product, `a ∧ b` (R-14).
fn cross2(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}

/// The state on the shape sphere (chart_reference §3.1; R-14) with its rates at `t = 0`, from `ρ̃̇ = p_ρ/√μ_ρ` and
/// `λ̃̇ = p_λ/√μ_λ` (RQ-343 (b)). At a pole the azimuth and the angles' rates, which the sphere does not define there, are
/// 0: the poles are the chart's singularities, the steer refusing there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sphere {
    /// The azimuth `θ = atan2(v, u)`.
    pub theta: f64,
    /// The polar angle `φ` from `+w`.
    pub phi: f64,
    /// `θ̇`.
    pub theta_dot: f64,
    /// `φ̇`.
    pub phi_dot: f64,
    /// `Ṙ`, `R = √I`.
    pub r_dot: f64,
    /// `sin φ`, the distance from the pole's axis.
    pub sin_phi: f64,
}

impl Sphere {
    /// `s` on the sphere, `n` the kernel's `shape_vec`; its rates are this map's time derivative, which the kernel has
    /// no use for and so does not define.
    pub fn of(s: &State) -> Self {
        let (rho, lambda) = (s.rho_t, s.lambda_t);
        let m01 = s.m[0] + s.m[1];
        let (root_rho, root_lambda) = ((s.m[0] * s.m[1] / m01).sqrt(), (s.m[2] * m01).sqrt());
        let rho_dot = [s.p_rho[0] / root_rho, s.p_rho[1] / root_rho];
        let lambda_dot = [s.p_lambda[0] / root_lambda, s.p_lambda[1] / root_lambda];
        let i = dot2(rho, rho) + dot2(lambda, lambda);
        let half_i_dot = dot2(rho, rho_dot) + dot2(lambda, lambda_dot);
        let i_dot = 2.0 * half_i_dot;
        let n = shape_vec(rho, lambda);
        let raw_dot = [
            2.0 * (dot2(rho, rho_dot) - dot2(lambda, lambda_dot)),
            2.0 * (dot2(rho_dot, lambda) + dot2(rho, lambda_dot)),
            2.0 * (cross2(rho_dot, lambda) + cross2(rho, lambda_dot)),
        ];
        let n_dot: [f64; 3] = std::array::from_fn(|k| raw_dot[k] / i - n[k] * i_dot / i);
        let sq = n[0] * n[0] + n[1] * n[1];
        let sin_phi = sq.sqrt();
        let at_pole = sq == 0.0;
        Self {
            theta: if at_pole { 0.0 } else { n[1].atan2(n[0]) },
            phi: sin_phi.atan2(n[2]),
            theta_dot: if at_pole {
                0.0
            } else {
                (n[0] * n_dot[1] - n[1] * n_dot[0]) / sq
            },
            phi_dot: if at_pole { 0.0 } else { -n_dot[2] / sin_phi },
            r_dot: half_i_dot / i.sqrt(),
            sin_phi,
        }
    }
}
