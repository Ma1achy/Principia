//! QA tests for TASK-M6-35's `engine::contract::decoded`, written from R-550, R-14 and REQ-GUI-199, against the
//! particles the decoder itself hands the integrator:
//!
//! - the state is the one after `η_E`: its Hamiltonian, summed here over the particles and in Jacobi form, is `E*`;
//! - the Jacobi momenta and `L_z` are the particles' own (chart_reference §0.3);
//! - `θ, φ` are R-14's and the rates are `d/dt` at `t = 0` of the mass-weighted `ρ̃, λ̃`, here by moving the particles
//!   at `ṙ = p/m` and differencing R-14's map, never through the code's formula.
//!
//! Each check runs its control (R-176) through [`rejects`], and each test has one that `cargo xtask controls` runs, by
//! `negative_control!` (REQ-VAL-147, R-199).
// The file name `qa_TASK-M6-35` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use engine::contract::decoded::{state, Sphere, State};
use engine::contract::sim_config::Links;
use kernel::decode::{energy_norm, Latent as KernelLatent};
use kernel::generated::links::Defaults;
use validation::negative_control;

type Z = [f64; 8];
type P = [[f64; 2]; 3];

fn rejects(what: &str, check: impl FnOnce()) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check));
    assert!(outcome.is_err(), "the check passed on {what}");
}

/// A fixed-seed splitmix64 over `[−2, 2]⁸`.
fn sample(seed: u64, n: usize) -> Vec<Z> {
    let mut s = seed;
    let mut next = move || {
        s = s.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut x = s;
        x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        x ^= x >> 31;
        (x >> 11) as f64 / (1u64 << 53) as f64
    };
    (0..n)
        .map(|_| std::array::from_fn(|_| 4.0 * next() - 2.0))
        .collect()
}

/// The particles the decoder hands the integrator.
fn particles(z: &Z, e_star: Option<f64>) -> ([f64; 3], P, P) {
    let params = Links::defaults().link_params();
    let (d, _) = energy_norm::decode_at::<Defaults, f64>(&KernelLatent::new(*z), e_star, &params);
    (d.ic.m, d.ic.r, d.ic.p)
}

/// `H` over the particles, `G = 1`.
fn energy(m: &[f64; 3], r: &P, p: &P) -> f64 {
    let t: f64 = (0..3)
        .map(|k| (p[k][0] * p[k][0] + p[k][1] * p[k][1]) / (2.0 * m[k]))
        .sum();
    let v: f64 = [(0, 1), (0, 2), (1, 2)]
        .iter()
        .map(|&(a, b)| -m[a] * m[b] / (r[a][0] - r[b][0]).hypot(r[a][1] - r[b][1]))
        .sum();
    t + v
}

/// `ρ̃, λ̃` of positions, chart_reference §0.1's weights.
fn weighted(m: &[f64; 3], r: &P) -> ([f64; 2], [f64; 2]) {
    let m01 = m[0] + m[1];
    let (mu_rho, mu_lambda) = (m[0] * m[1] / m01, m[2] * m01);
    let rho: [f64; 2] = std::array::from_fn(|k| r[1][k] - r[0][k]);
    let lambda: [f64; 2] =
        std::array::from_fn(|k| r[2][k] - (m[0] * r[0][k] + m[1] * r[1][k]) / m01);
    (
        rho.map(|x| x * mu_rho.sqrt()),
        lambda.map(|x| x * mu_lambda.sqrt()),
    )
}

/// `(θ, φ, R, sin φ)` by R-14: `u = |ρ̃|² − |λ̃|²`, `v = 2ρ̃·λ̃`, `w = 2ρ̃∧λ̃`, over `I`.
fn r14(rho: [f64; 2], lambda: [f64; 2]) -> (f64, f64, f64, f64) {
    let (a, b) = (
        rho[0] * rho[0] + rho[1] * rho[1],
        lambda[0] * lambda[0] + lambda[1] * lambda[1],
    );
    let i = a + b;
    let u = (a - b) / i;
    let v = 2.0 * (rho[0] * lambda[0] + rho[1] * lambda[1]) / i;
    let w = 2.0 * (rho[0] * lambda[1] - rho[1] * lambda[0]) / i;
    let sin = u.hypot(v);
    (v.atan2(u), sin.atan2(w), i.sqrt(), sin)
}

fn close(got: f64, want: f64, tol: f64, what: &str) {
    assert!((got - want).abs() <= tol, "{what}: {got} against {want}");
}

/// The state's fields against the particles of the same decode.
fn check_state(s: &State, m: &[f64; 3], r: &P, p: &P) {
    let (rho, lambda) = weighted(m, r);
    for k in 0..2 {
        close(s.rho_t[k], rho[k], 1e-12, "ρ̃");
        close(s.lambda_t[k], lambda[k], 1e-12, "λ̃");
    }
    let m01 = m[0] + m[1];
    let (mu_rho, mu_lambda) = (m[0] * m[1] / m01, m[2] * m01);
    // Jacobi momenta are μ times the Jacobi velocities, from ṙ = p/m.
    let velocity = |b: usize, k: usize| p[b][k] / m[b];
    for k in 0..2 {
        let (v0, v1, v2) = (velocity(0, k), velocity(1, k), velocity(2, k));
        close(s.p_rho[k], mu_rho * (v1 - v0), 1e-10, "p_ρ");
        let com = (m[0] * v0 + m[1] * v1) / m01;
        close(s.p_lambda[k], mu_lambda * (v2 - com), 1e-10, "p_λ");
    }
    let l: f64 = (0..3).map(|b| r[b][0] * p[b][1] - r[b][1] * p[b][0]).sum();
    close(s.l_z, l, 1e-10, "L_z");
}

#[test]
fn qa_decoded_state_is_the_particles_after_eta_e() {
    let links = Links::defaults();
    let mut hit = 0;
    for z in sample(0x3535, 200) {
        for e_star in [None, Some(0.0), Some(-0.1)] {
            let (m, r, p) = particles(&z, e_star);
            check_state(&state(&z, &links, e_star), &m, &r, &p);
            if let Some(e) = e_star {
                let v_only = energy(&m, &r, &[[0.0; 2]; 3]);
                if e > v_only {
                    close(energy(&m, &r, &p), e, 1e-9 * (1.0 + v_only.abs()), "H = E*");
                    hit += 1;
                }
            }
        }
    }
    assert!(hit > 200, "the shell was reached {hit} times");
    // Control: the state before η_E is not the particles after it.
    let z = sample(0x3535, 1)[0];
    let (m, r, p) = particles(&z, Some(0.0));
    let before = state(&z, &links, None);
    rejects("the state before η_E", || check_state(&before, &m, &r, &p));
}

negative_control!(
    qa_decoded_state_is_the_particles_after_eta_e,
    "the state read before η_E must not pass as the particles after it",
    expected = "p_ρ:",
    {
        let z = sample(0x3535, 1)[0];
        let (m, r, p) = particles(&z, Some(0.0));
        check_state(&state(&z, &Links::defaults(), None), &m, &r, &p)
    }
);

/// `(θ̇, φ̇, Ṙ)` by a central difference in `t` of R-14's map over particles moving at `ṙ = p/m`.
fn rates_by_motion(m: &[f64; 3], r: &P, p: &P) -> (f64, f64, f64) {
    let dt = 1e-6;
    let at = |t: f64| -> (f64, f64, f64, f64) {
        let moved: P =
            std::array::from_fn(|b| [r[b][0] + t * p[b][0] / m[b], r[b][1] + t * p[b][1] / m[b]]);
        let (rho, lambda) = weighted(m, &moved);
        r14(rho, lambda)
    };
    let (a, b) = (at(-dt), at(dt));
    let mut dtheta = b.0 - a.0;
    let tau = std::f64::consts::TAU;
    dtheta -= tau * ((dtheta + std::f64::consts::PI) / tau).floor();
    (
        dtheta / (2.0 * dt),
        (b.1 - a.1) / (2.0 * dt),
        (b.2 - a.2) / (2.0 * dt),
    )
}

fn check_sphere(sh: &Sphere, m: &[f64; 3], r: &P, p: &P) {
    let (rho, lambda) = weighted(m, r);
    let (theta, phi, _, sin) = r14(rho, lambda);
    close(sh.theta, theta, 1e-12, "θ");
    close(sh.phi, phi, 1e-12, "φ");
    close(sh.sin_phi, sin, 1e-12, "sin φ");
    let (dtheta, dphi, dr) = rates_by_motion(m, r, p);
    let scale = 1.0 + dtheta.abs() + dphi.abs();
    close(sh.theta_dot, dtheta, 1e-4 * scale, "θ̇");
    close(sh.phi_dot, dphi, 1e-4 * scale, "φ̇");
    close(sh.r_dot, dr, 1e-4 * (1.0 + dr.abs()), "Ṙ");
}

#[test]
fn qa_decoded_sphere_is_r14s_and_the_rates_are_d_dt_of_the_motion() {
    let links = Links::defaults();
    let mut checked = 0;
    for z in sample(0x1414, 200) {
        for e_star in [None, Some(0.0)] {
            let (m, r, p) = particles(&z, e_star);
            let (rho, lambda) = weighted(&m, &r);
            // Clear of the pole, where the rates are the chart's singularity.
            if r14(rho, lambda).3 < 0.05 {
                continue;
            }
            check_sphere(&Sphere::of(&state(&z, &links, e_star)), &m, &r, &p);
            checked += 1;
        }
    }
    assert!(checked > 300, "only {checked} states");
    // Control: a sphere with twice the true rates, and one read before η_E on an E* chart.
    let z = sample(0x1414, 1)[0];
    let (m, r, p) = particles(&z, None);
    let mut doubled = Sphere::of(&state(&z, &links, None));
    doubled.theta_dot *= 2.0;
    doubled.phi_dot *= 2.0;
    rejects("doubled rates", || check_sphere(&doubled, &m, &r, &p));
    let (m, r, p) = particles(&z, Some(0.0));
    let before = Sphere::of(&state(&z, &links, None));
    rejects("the rates before η_E", || {
        check_sphere(&before, &m, &r, &p)
    });
}

negative_control!(
    qa_decoded_sphere_is_r14s_and_the_rates_are_d_dt_of_the_motion,
    "a sphere with twice the true rates must not pass as d/dt of the motion",
    expected = "θ̇:",
    {
        let links = Links::defaults();
        let z = sample(0x1414, 1)[0];
        let (m, r, p) = particles(&z, None);
        let mut doubled = Sphere::of(&state(&z, &links, None));
        doubled.theta_dot *= 2.0;
        doubled.phi_dot *= 2.0;
        check_sphere(&doubled, &m, &r, &p)
    }
);

/// A state of equal masses, at rest, with the given `ρ̃, λ̃`.
fn fill(rho_t: [f64; 2], lambda_t: [f64; 2]) -> State {
    State {
        m: [1.0 / 3.0; 3],
        alpha: 0.0,
        beta: 0.0,
        rho_t,
        lambda_t,
        p_rho: [0.0; 2],
        p_lambda: [0.0; 2],
        l_z: 0.0,
    }
}

/// R-14's landmarks on hand-filled states, through `sphere`: `L⁺` at `w = +1`, the 0–1 collision at
/// `n = (−1, 0, 0)`, `φ` from `+w`.
fn check_landmarks(sphere: impl Fn(&State) -> Sphere) {
    let root = std::f64::consts::FRAC_1_SQRT_2;
    let up = sphere(&fill([root, 0.0], [0.0, root]));
    assert!(up.phi.abs() < 1e-12 && up.sin_phi < 1e-12, "L⁺: {up:?}");
    let down = sphere(&fill([root, 0.0], [0.0, -root]));
    assert!(
        (down.phi - std::f64::consts::PI).abs() < 1e-12,
        "L⁻: {down:?}"
    );
    assert!(down.theta.is_finite() && down.theta_dot == 0.0, "{down:?}");
    let collision = sphere(&fill([0.0, 0.0], [1.0, 0.0]));
    assert!(
        (collision.theta.abs() - std::f64::consts::PI).abs() < 1e-12,
        "n = (−1, 0, 0): {collision:?}"
    );
    assert!((collision.phi - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
    let far = sphere(&fill([1.0, 0.0], [0.0, 0.0]));
    assert!(far.theta.abs() < 1e-12 && (far.phi - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
}

#[test]
fn qa_decoded_sphere_landmarks_are_r14s() {
    check_landmarks(Sphere::of);
    let root = std::f64::consts::FRAC_1_SQRT_2;
    // Control: the flipped cross puts L⁺ at the other pole.
    rejects("the negated cross", || {
        let flipped = Sphere::of(&fill([root, 0.0], [0.0, -root]));
        assert!(flipped.phi.abs() < 1e-12)
    });
}

negative_control!(
    qa_decoded_sphere_landmarks_are_r14s,
    "the negated cross puts L⁺ at the other pole, so must fail the landmarks",
    expected = "L⁺:",
    check_landmarks(|s| {
        let mut negated = *s;
        negated.lambda_t[1] = -negated.lambda_t[1];
        Sphere::of(&negated)
    })
);
