//! `contract::decoded` (R-550; dd_decoder §3.2, §3.4): the state the compass's physical views read is the decoder's own,
//! its Jacobi momenta conjugate to its bodies' at unequal masses and read after `η_E`.

use kernel::decode::{energy_norm, Latent as KernelLatent};
use kernel::generated::links::Defaults;

use crate::contract::decoded::{state, Sphere, State};
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

/// The state a time `t` along its rates: `ρ̃ + t ρ̃̇`, `λ̃ + t λ̃̇`.
fn advanced(s: &State, t: f64) -> State {
    let m01 = s.m[0] + s.m[1];
    let (a, b) = ((s.m[0] * s.m[1] / m01).sqrt(), (s.m[2] * m01).sqrt());
    State {
        rho_t: std::array::from_fn(|k| s.rho_t[k] + t * s.p_rho[k] / a),
        lambda_t: std::array::from_fn(|k| s.lambda_t[k] + t * s.p_lambda[k] / b),
        ..*s
    }
}

/// A state off the scale gauge, `I = 4`, where the rates' divisions by `I` and `√I` matter.
fn off_gauge() -> State {
    State {
        m: [0.2, 0.3, 0.5],
        alpha: 0.0,
        beta: 0.0,
        rho_t: [1.2, 0.3],
        lambda_t: [0.4, 1.5],
        p_rho: [0.11, -0.12],
        p_lambda: [0.13, 0.14],
        l_z: 0.0,
    }
}

/// Asserts `sphere`'s rates are `d/dt` of `s` by central difference, against `shape_vec`'s angles.
fn check_rates(s: &State, sphere: &Sphere) {
    let h = 1e-6;
    let (up, down) = (Sphere::of(&advanced(s, h)), Sphere::of(&advanced(s, -h)));
    let near = |rate: f64, want: f64, what: &str| {
        assert!((rate - want).abs() <= 1e-6, "{what}: {rate} against {want}");
    };
    near((up.theta - down.theta) / (2.0 * h), sphere.theta_dot, "θ̇");
    near((up.phi - down.phi) / (2.0 * h), sphere.phi_dot, "φ̇");
    let root_i = |x: &State| {
        let n2 = |v: [f64; 2]| v[0] * v[0] + v[1] * v[1];
        (n2(x.rho_t) + n2(x.lambda_t)).sqrt()
    };
    near(
        (root_i(&advanced(s, h)) - root_i(&advanced(s, -h))) / (2.0 * h),
        sphere.r_dot,
        "Ṙ",
    );
}

#[test]
fn decoded_sphere_is_shape_vecs_with_its_rates() {
    let s = off_gauge();
    let sphere = Sphere::of(&s);
    let n = kernel::shape::shape_vec(s.rho_t, s.lambda_t);
    assert_eq!(sphere.sin_phi, (n[0] * n[0] + n[1] * n[1]).sqrt());
    assert_eq!(sphere.theta, n[1].atan2(n[0]));
    assert_eq!(sphere.phi, sphere.sin_phi.atan2(n[2]), "φ from +w");
    check_rates(&s, &sphere);
    // At the exact pole the azimuth and the angles' rates are defined as 0, never NaN; Ṙ is still the state's.
    let at = State {
        rho_t: [1.0, 0.0],
        lambda_t: [0.0, 1.0],
        p_rho: [0.1, 0.2],
        p_lambda: [0.3, 0.4],
        ..s
    };
    let pole = Sphere::of(&at);
    assert_eq!(
        (pole.theta, pole.theta_dot, pole.phi_dot, pole.sin_phi),
        (0.0, 0.0, 0.0, 0.0)
    );
    assert_eq!(pole.phi, 0.0, "the +w pole");
    assert!(pole.r_dot != 0.0 && pole.r_dot.is_finite());
    // Beside the pole, its rates are not the pole's zeros.
    let beside = State {
        rho_t: [1.0, 1e-3],
        ..at
    };
    let near = Sphere::of(&beside);
    assert!(near.theta != 0.0 && near.theta_dot != 0.0 && near.phi_dot != 0.0);
}

/// The check on rates read at twice the state's speed.
#[cfg(feature = "controls")]
fn check_doubled_rates() {
    let s = off_gauge();
    let mut sphere = Sphere::of(&s);
    sphere.r_dot *= 2.0;
    check_rates(&s, &sphere);
}

validation::negative_control!(
    decoded_sphere_is_shape_vecs_with_its_rates,
    "a rate off the state's d/dt must be rejected",
    expected = "Ṙ",
    check_doubled_rates()
);

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
