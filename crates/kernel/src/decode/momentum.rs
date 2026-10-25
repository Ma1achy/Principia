//! `D_mom`, the free momentum block (dd_decoder §3.4; chart_reference §0.3; REQ-DEC-019, REQ-DEC-030): the four
//! controls `z[2:6]` through the selection's momentum links onto the physical Jacobi momenta, then the
//! Jacobi-to-particle map. Under the default links, `qₖ = q_max·(2σ(z_qk) − 1)`, `k = 0..3`; `p_ρ = (q₀, q₁)` and
//! `p_λ = (q₂, q₃)`.
//!
//! **Jacobi-to-particle** (`Σpᵢ = 0` identically):
//!
//! ```text
//! p₀ = −p_ρ − (m₀/M₀₁)·p_λ
//! p₁ = +p_ρ − (m₁/M₀₁)·p_λ
//! p₂ =  p_λ
//! ```
//!
//! The `m₀` and `m₁` factors are **crossed** against the position reconstruction's (`r₀` takes `−m₁/M₀₁`, `p₀` takes
//! `−m₀/M₀₁`; chart_reference §0.3's transcription hazard): the momenta conjugate to `ρ = r₁ − r₀` and
//! `λ = r₂ − (m₀r₀ + m₁r₁)/M₀₁` (PIT-9: `Σpᵢ = 0` catches a swap only at unequal masses).
//!
//! **No mass unweighting** (R-28): `p_ρ` and `p_λ` are the physical Jacobi momenta, used as decoded. The configuration
//! block's `√μ_ρ` and `√μ_λ` act on the positions only; nothing here divides or multiplies the momenta by them.
//!
//! **What is a link here** (REQ-CHART-033): each `qₖ` comes only from the selection's generated link. The map to the
//! particles is the decoder's formula (§3.4), not a registry link. `M₀₁` is the mass block's, the one rounded sum its
//! tag tested. No guard: `|qₖ| ≤ q_max` is the link's codomain, reached at saturation (R-417).

use crate::decode::mass::Masses;
use crate::generated::links::{Link, LinkParams, LinkReal, Selection};

/// The momentum block's output.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Momentum<R> {
    /// The decoded controls `q₀ … q₃`, each in `[−q_max, q_max]`.
    pub q: [R; 4],
    /// The inner pair's Jacobi momentum `p_ρ = (q₀, q₁)`.
    pub p_rho: [R; 2],
    /// The outer Jacobi momentum `p_λ = (q₂, q₃)`.
    pub p_lambda: [R; 2],
    /// The particle momenta `p₀, p₁, p₂`, `Σpᵢ = 0`.
    pub p: [[R; 2]; 3],
}

/// `(q₀, q₁, q₂, q₃)` from `z[2:6]`, each through the selection's link for its control.
#[inline]
pub fn controls<S: Selection, R: LinkReal>(z_mom: [R; 4]) -> [R; 4] {
    controls_at::<S, R>(z_mom, &LinkParams::DEFAULT)
}

/// [`controls`] at the tunable parameters `p`: each link reads the active `q_max`, so `|qₖ| ≤ q_max` at its active
/// value (R-481 A2, G4; REQ-DEC-048).
#[inline]
pub fn controls_at<S: Selection, R: LinkReal>(z_mom: [R; 4], p: &LinkParams<R>) -> [R; 4] {
    let [q0] = <S::Q0Link as Link<1, 1>>::forward_at([z_mom[0]], p);
    let [q1] = <S::Q1Link as Link<1, 1>>::forward_at([z_mom[1]], p);
    let [q2] = <S::Q2Link as Link<1, 1>>::forward_at([z_mom[2]], p);
    let [q3] = <S::Q3Link as Link<1, 1>>::forward_at([z_mom[3]], p);
    [q0, q1, q2, q3]
}

/// The Jacobi-to-particle map: `p₀ = −p_ρ − (m₀/M₀₁)p_λ`, `p₁ = p_ρ − (m₁/M₀₁)p_λ`, `p₂ = p_λ`, the factors crossed
/// against the positions' (the module's note), with no mass unweighting (R-28).
#[inline]
pub fn to_particles<R: LinkReal>(
    masses: &Masses<R>,
    p_rho: [R; 2],
    p_lambda: [R; 2],
) -> [[R; 2]; 3] {
    let [m0, m1, _] = masses.m;
    let w0 = m0 / masses.m01;
    let w1 = m1 / masses.m01;
    [
        [-p_rho[0] - w0 * p_lambda[0], -p_rho[1] - w0 * p_lambda[1]],
        [p_rho[0] - w1 * p_lambda[0], p_rho[1] - w1 * p_lambda[1]],
        p_lambda,
    ]
}

/// `D_mom`, free: the momentum controls `z[2:6]`, with the mass block's output, to the particle momenta.
#[inline]
pub fn decode<S: Selection, R: LinkReal>(z_mom: [R; 4], masses: &Masses<R>) -> Momentum<R> {
    decode_at::<S, R>(z_mom, masses, &LinkParams::DEFAULT)
}

/// [`decode`] at the tunable parameters `p` ([`controls_at`]).
#[inline]
pub fn decode_at<S: Selection, R: LinkReal>(
    z_mom: [R; 4],
    masses: &Masses<R>,
    p: &LinkParams<R>,
) -> Momentum<R> {
    let q = controls_at::<S, R>(z_mom, p);
    let p_rho = [q[0], q[1]];
    let p_lambda = [q[2], q[3]];
    Momentum {
        q,
        p_rho,
        p_lambda,
        p: to_particles(masses, p_rho, p_lambda),
    }
}
