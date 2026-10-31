//! `contract::decoded` (R-550; dd_decoder §3.2, §3.4): the state the compass's physical views read is the decoder's own,
//! its Jacobi momenta conjugate to its bodies' at unequal masses and read after `η_E`.

use kernel::decode::{energy_norm, Latent as KernelLatent};
use kernel::generated::links::Defaults;

use crate::contract::decoded::{state, State};
use crate::contract::sim_config::{Latent, Links};

/// A latent point at unequal masses, moving.
const Z: Latent = [0.2, 0.4, 0.3, -0.2, 0.5, 0.1, 0.1, -0.2];

/// Asserts `s` is the decoder's at `z` with `e_star` and active `q_max`: its masses, angles, weighted vectors and bodies'
/// momenta from `p_ρ`, `p_λ` by chart_reference §0.3's map, and `L_z = Σ rᵢ × pᵢ`.
fn check(s: &State, z: &Latent, links: &Links, e_star: Option<f64>) {
    let params = links.link_params();
    let (d, _) = energy_norm::decode_at::<Defaults, f64>(&KernelLatent::new(*z), e_star, &params);
    assert_eq!(s.m, d.ic.m, "the masses");
    assert_eq!((s.alpha, s.beta), (d.config.alpha, d.config.beta), "α, β");
    assert_eq!((s.rho_t, s.lambda_t), (d.config.rho_t, d.config.lambda_t));
    let m01 = s.m[0] + s.m[1];
    for k in 0..2 {
        let p0 = -s.p_rho[k] - s.m[0] / m01 * s.p_lambda[k];
        let p1 = s.p_rho[k] - s.m[1] / m01 * s.p_lambda[k];
        assert!((p0 - d.ic.p[0][k]).abs() <= 1e-13, "p₀ at {k}");
        assert!((p1 - d.ic.p[1][k]).abs() <= 1e-13, "p₁ at {k}");
        assert_eq!(s.p_lambda[k], d.ic.p[2][k], "p₂ = p_λ");
    }
    let l: f64 = (0..3)
        .map(|i| d.ic.r[i][0] * d.ic.p[i][1] - d.ic.r[i][1] * d.ic.p[i][0])
        .sum();
    assert_eq!(s.l_z, l, "L_z");
}

#[test]
fn decoded_state_is_the_decoders() {
    let links = Links::defaults();
    check(&state(&Z, &links, None), &Z, &links, None);
    check(&state(&Z, &links, Some(0.0)), &Z, &links, Some(0.0));
    // With η_E off the Jacobi momenta are the momentum block's own.
    let params = links.link_params();
    let (d, _) = energy_norm::decode_at::<Defaults, f64>(&KernelLatent::new(Z), None, &params);
    let s = state(&Z, &links, None);
    for k in 0..2 {
        assert!((s.p_rho[k] - d.momentum.p_rho[k]).abs() <= 1e-13, "p_ρ");
        assert!(
            (s.p_lambda[k] - d.momentum.p_lambda[k]).abs() <= 1e-13,
            "p_λ"
        );
    }
    // The tunable q_max is read: doubled, the momenta double at these controls.
    let mut wide = Links::defaults();
    let q_max = ledger::constants::Q_MAX.name;
    let was = wide.link_params().q_max;
    wide.momenta.params.insert(q_max.to_owned(), 2.0 * was);
    let scaled = state(&Z, &wide, None);
    assert!((scaled.p_lambda[0] - 2.0 * s.p_lambda[0]).abs() <= 1e-12);
    // η_E moves only the momenta.
    let on = state(&Z, &links, Some(0.0));
    assert_eq!((on.m, on.alpha, on.beta), (s.m, s.alpha, s.beta));
    assert_ne!(on.p_rho, s.p_rho);
}

/// The check on a state read before `η_E`, against the target.
#[cfg(feature = "controls")]
fn check_before_eta() {
    let links = Links::defaults();
    check(&state(&Z, &links, None), &Z, &links, Some(0.0));
}

validation::negative_control!(
    decoded_state_is_the_decoders,
    "a state read before η_E on an E* chart must be rejected",
    expected = "p₀ at",
    check_before_eta()
);
