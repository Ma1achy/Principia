//! `compass_latent_jacobian` (REQ-DEC-049; deep_zoom §2, "The latent Jacobian"; R-550, R-633): eight central-difference
//! columns in f64 at `J_D`'s step, divided by the step actually taken, read after `η_E`, in the view's coordinates; rank
//! 7 with `E*` set and 8 without; the stretch on the orthonormalised pair. Each check also runs on an input it must
//! reject (R-176, through `rejects`).

use std::cell::RefCell;

use engine::contract::decoded::{self, Sphere};
use engine::contract::sim_config::{Latent, Links, Plane, LATENT_DIM};

use super::compass_plucker::Rng;
use super::support::rejects;
use crate::explore::compass::physical::{
    apply, central_column, difference, gram, latent_jacobian, near_pole, refuses, smaller_singular,
    svd, Decoder, Inverse, Matrix, Set, CUT, JACOBI_NAMES, SHAPE_NAMES,
};
use crate::explore::compass::views::orthonormal;

/// A smooth fixture decoder, each output a known function of all eight inputs.
fn smooth(z: &Latent) -> Latent {
    std::array::from_fn(|r| {
        let k = (r + 3) % LATENT_DIM;
        z[r].sin() + 0.5 * z[k] * z[k] + 0.25 * z[r] * z[k]
    })
}

/// `smooth`'s analytic Jacobian, rows first.
fn smooth_jacobian(z: &Latent) -> Matrix {
    let mut j = [[0.0; LATENT_DIM]; LATENT_DIM];
    for (r, row) in j.iter_mut().enumerate() {
        let k = (r + 3) % LATENT_DIM;
        row[r] += z[r].cos() + 0.25 * z[k];
        row[k] += z[k] + 0.25 * z[r];
    }
    j
}

/// The plain difference.
fn plain(_: usize, a: f64, b: f64) -> f64 {
    b - a
}

/// The method's bound at `J_D`'s step on `smooth` near the unit box: truncation `h²|f‴|/6` plus rounding `ε|f|/h`.
const FIXTURE_BOUND: f64 = 1e-9;

/// Asserts `col` is column `k` of the analytic Jacobian at `z` within the bound.
fn check_column(col: &Latent, z: &Latent, k: usize) {
    let want = smooth_jacobian(z);
    for r in 0..LATENT_DIM {
        assert!(
            (col[r] - want[r][k]).abs() <= FIXTURE_BOUND,
            "row {r}, column {k}: {} against {}",
            col[r],
            want[r][k]
        );
    }
}

#[test]
fn compass_latent_jacobian_agrees_with_the_analytic_fixture() {
    let mut rng = Rng::new(11);
    for _ in 0..20 {
        let z = rng.latent();
        for k in 0..LATENT_DIM {
            check_column(&central_column(&smooth, &plain, &z, k), &z, k);
        }
    }
    // Control: a one-sided difference, at the same step, misses the bound.
    let z = [0.7; LATENT_DIM];
    let h = f64::EPSILON.cbrt();
    let mut up = z;
    up[0] += h;
    let (a, b) = (smooth(&z), smooth(&up));
    let one_sided: Latent = std::array::from_fn(|r| (b[r] - a[r]) / (up[0] - z[0]));
    rejects("a one-sided difference", || check_column(&one_sided, &z, 0));
}

/// The two points `central_column` evaluates along `k` at `z`, and the column.
fn evaluated(z: &Latent, k: usize) -> (Vec<Latent>, Latent) {
    let seen = RefCell::new(Vec::new());
    let f = |at: &Latent| {
        seen.borrow_mut().push(*at);
        *at
    };
    let col = central_column(&f, &plain, z, k);
    (seen.into_inner(), col)
}

#[test]
fn compass_latent_jacobian_takes_j_ds_step_divided_by_the_step_taken() {
    let root = f64::EPSILON.cbrt();
    // Below, at and above |z_k| = 1, of either sign.
    for (zk, h) in [
        (0.0, root),
        (0.5, root),
        (-0.5, root),
        (1.0, root),
        (-1.0, root),
        (3.0, 3.0 * root),
        (-1e6, 1e6 * root),
    ] {
        let mut z = [0.25; LATENT_DIM];
        z[2] = zk;
        let (points, col) = evaluated(&z, 2);
        assert_eq!(points.len(), 2, "two evaluations per direction");
        assert_eq!(points[0][2], zk - h, "z − h at {zk}");
        assert_eq!(points[1][2], zk + h, "z + h at {zk}");
        for (k, x) in points[0].iter().enumerate() {
            if k != 2 {
                assert_eq!(*x, z[k], "only z_k moves");
            }
        }
        // Dividing by the step taken, the identity's own coordinate differentiates to exactly 1.
        assert_eq!(col[2], 1.0, "at {zk}");
        assert_eq!(col[3], 0.0);
    }
    // Control: dividing by 2h, not the step taken, misses 1 where the sum rounds.
    let zk = 1.0e6_f64 + 0.3;
    let h = root * zk.abs();
    let by_two_h = ((zk + h) - (zk - h)) / (2.0 * h);
    rejects("a difference divided by 2h", || assert_eq!(by_two_h, 1.0));
}

#[test]
fn compass_latent_jacobian_on_the_decoder_each_column_is_its_two_evaluations() {
    let links = Links::defaults();
    let q_max = links.link_params().q_max;
    let mut rng = Rng::new(21);
    for (set, e_star) in [
        (Set::Jacobi, None),
        (Set::Jacobi, Some(0.0)),
        (Set::Shape, None),
    ] {
        let z = rng.latent();
        let d = Decoder::Real { e_star };
        let j = latent_jacobian(&d, set, &links, &z);
        let ranges = set.ranges(q_max);
        for k in 0..LATENT_DIM {
            let h = f64::EPSILON.cbrt() * z[k].abs().max(1.0);
            let (mut up, mut down) = (z, z);
            up[k] += h;
            down[k] -= h;
            let (a, b) = (
                set.coordinates(&decoded::state(&down, &links, e_star)),
                set.coordinates(&decoded::state(&up, &links, e_star)),
            );
            for r in 0..LATENT_DIM {
                // θ's own row is checked across the seam below; away from it the two agree.
                let want = (b[r] - a[r]) / (up[k] - down[k]) / ranges[r];
                assert_eq!(j[r][k], want, "{set:?}, row {r}, column {k}");
            }
        }
    }
}

#[test]
fn compass_latent_jacobian_theta_differs_on_the_circle() {
    // A linear fixture cannot show it: the difference itself. Across the seam θ = ±π the short way round is small.
    let links = Links::defaults();
    let mut rng = Rng::new(31);
    let mut crossed = 0;
    for _ in 0..3000 {
        let mut z = rng.latent().map(|x| 3.0 * x);
        z[2] = 0.0;
        let j = latent_jacobian(&Decoder::default(), Set::Shape, &links, &z);
        let s0 = Sphere::of(&decoded::state(&z, &links, None));
        // Close to the seam, away from the poles, the row stays that of a smooth function.
        if s0.theta.abs() > std::f64::consts::PI - 2e-3 && s0.sin_phi > 0.1 {
            crossed += 1;
            for col in j[0] {
                assert!(col.abs() < 1e3, "θ's row across the seam: {col}");
            }
        }
    }
    assert!(crossed > 0, "no sample reached the seam");
    // Control: the raw difference across the seam is about 2π, over a step of about 1e-5.
    let raw = (std::f64::consts::PI - 1e-6 - (-std::f64::consts::PI + 1e-6))
        / 1.2e-5
        / std::f64::consts::TAU;
    rejects("θ's raw difference", || assert!(raw.abs() < 1e3));
}

#[test]
fn compass_latent_jacobian_has_rank_8_unset_and_7_with_e_star() {
    let links = Links::defaults();
    for set in [Set::Jacobi, Set::Shape] {
        let mut rng = Rng::new(41);
        for _ in 0..200 {
            let z = rng.latent().map(|x| 3.0 * x);
            let at = |e_star| {
                Inverse::of(
                    &latent_jacobian(&Decoder::Real { e_star }, set, &links, &z),
                    CUT,
                )
                .kept
            };
            assert_eq!(at(None), 8, "{set:?} at {z:?}");
            assert_eq!(at(Some(0.0)), 7, "{set:?} at {z:?}");
        }
    }
}

#[test]
fn compass_latent_jacobian_reads_after_eta_e() {
    let links = Links::defaults();
    let z = [0.2, 0.4, 0.3, -0.2, 0.5, 0.1, 0.1, -0.2];
    let after = latent_jacobian(
        &Decoder::Real { e_star: Some(0.0) },
        Set::Jacobi,
        &links,
        &z,
    );
    let before = latent_jacobian(&Decoder::Real { e_star: None }, Set::Jacobi, &links, &z);
    // η_E moves the momenta alone: their rows differ, and the state's energy is the target.
    let s = decoded::state(&z, &links, Some(0.0));
    let m01 = s.m[0] + s.m[1];
    let (mu_rho, mu_lambda) = (s.m[0] * s.m[1] / m01, s.m[2] * m01);
    let kinetic = (s.p_rho[0] * s.p_rho[0] + s.p_rho[1] * s.p_rho[1]) / (2.0 * mu_rho)
        + (s.p_lambda[0] * s.p_lambda[0] + s.p_lambda[1] * s.p_lambda[1]) / (2.0 * mu_lambda);
    let unset = decoded::state(&z, &links, None);
    assert!(kinetic > 0.0);
    assert!(
        (kinetic - s_potential(&s)).abs() <= 1e-12 * kinetic,
        "E* = 0: K = −U: {kinetic}"
    );
    assert_ne!(s.p_rho, unset.p_rho);
    assert_eq!(s.alpha, unset.alpha, "η_E moves only the momenta");
    let differs = |a: &Matrix, b: &Matrix| {
        for row in 2..6 {
            for k in 0..LATENT_DIM {
                if (a[row][k] - b[row][k]).abs() > 1e-6 {
                    return;
                }
            }
        }
        panic!("the momenta's rows agree");
    };
    differs(&after, &before);
    // Control: the Jacobian before η_E against itself.
    rejects("a Jacobian taken before η_E", || differs(&before, &before));
}

/// `−U = Σ_{i<j} m_i m_j / r_ij` of `s`, from its mass-weighted Jacobi vectors (`G = 1`).
fn s_potential(s: &decoded::State) -> f64 {
    let m01 = s.m[0] + s.m[1];
    let root_rho = (s.m[0] * s.m[1] / m01).sqrt();
    let root_lambda = (s.m[2] * m01).sqrt();
    let rho = [s.rho_t[0] / root_rho, s.rho_t[1] / root_rho];
    let lambda = [s.lambda_t[0] / root_lambda, s.lambda_t[1] / root_lambda];
    let r01 = rho[0].hypot(rho[1]);
    let r02 = (lambda[0] + s.m[1] / m01 * rho[0]).hypot(lambda[1] + s.m[1] / m01 * rho[1]);
    let r12 = (lambda[0] - s.m[0] / m01 * rho[0]).hypot(lambda[1] - s.m[0] / m01 * rho[1]);
    s.m[0] * s.m[1] / r01 + s.m[0] * s.m[2] / r02 + s.m[1] * s.m[2] / r12
}

/// The plane at `z` spanned by `q̂₁`, `q̂₂` stretched by `a`, `b`, with `shear` of `q̂₁` added to `q₂`.
fn sheared(z: Latent, e1: &Latent, e2: &Latent, a: f64, b: f64, shear: f64) -> Plane {
    Plane {
        z0: z,
        q1: e1.map(|x| x * a),
        q2: std::array::from_fn(|k| b * e2[k] + shear * e1[k]),
    }
}

#[test]
fn compass_latent_jacobian_the_stretch_is_on_the_orthonormalised_pair() {
    let links = Links::defaults();
    let mut rng = Rng::new(51);
    let z = rng.latent();
    let j = latent_jacobian(&Decoder::default(), Set::Jacobi, &links, &z);
    let base = sheared(z, &rng.latent(), &rng.latent(), 1.0, 1.0, 0.0);
    let (e1, e2) = orthonormal(&base).expect("a plane");
    let stretch = |p: &Plane| {
        let (a, b) = orthonormal(p).expect("a plane");
        gram(&apply(&j, &a), &apply(&j, &b))
    };
    let want = stretch(&sheared(z, &e1, &e2, 1.0, 1.0, 0.0));
    // The Gram factor by its definition.
    let (u1, u2) = (apply(&j, &e1), apply(&j, &e2));
    let dot = |a: &Latent, b: &Latent| -> f64 { a.iter().zip(b).map(|(x, y)| x * y).sum() };
    let defined = (dot(&u1, &u1) * dot(&u2, &u2) - dot(&u1, &u2) * dot(&u1, &u2)).sqrt();
    assert!((want - defined).abs() <= 1e-12 * defined);
    for (a, b, shear) in [(2.0, 2.0, 0.0), (0.5, 3.0, 0.0), (1.0, 1.0, 0.7)] {
        let s = stretch(&sheared(z, &e1, &e2, a, b, shear));
        assert!(
            (s - want).abs() <= 1e-12 * want,
            "the stretch moved: {s} against {want}"
        );
    }
    // Control: a stretch on the un-orthonormalised q moves with σ.
    let doubled = sheared(z, &e1, &e2, 2.0, 2.0, 0.0);
    let raw = gram(&apply(&j, &doubled.q1), &apply(&j, &doubled.q2));
    rejects("a stretch on the un-orthonormalised q", || {
        assert!((raw - want).abs() <= 1e-12 * want)
    });
}

#[test]
fn compass_latent_jacobian_the_sets_and_ranges_are_r_550s() {
    use std::f64::consts::{FRAC_PI_2, PI, TAU};
    assert_eq!(
        JACOBI_NAMES,
        ["α", "β", "p_ρx", "p_ρy", "p_λx", "p_λy", "m₀/M", "m₁/M"]
    );
    assert_eq!(
        SHAPE_NAMES,
        ["θ", "φ", "dθ/dt", "dφ/dt", "L_z", "dR/dt", "m₀/M", "m₁/M"]
    );
    assert_eq!(Set::Jacobi.names(), JACOBI_NAMES);
    assert_eq!(Set::Shape.names(), SHAPE_NAMES);
    assert_eq!(
        Set::Jacobi.ranges(2.0),
        [FRAC_PI_2, PI, 2.0, 2.0, 2.0, 2.0, 1.0, 1.0]
    );
    let r = 21.0_f64.sqrt() * 3.0;
    assert_eq!(
        Set::Shape.ranges(3.0),
        [TAU, PI, 2.0 * r, 2.0 * r, r, r, 1.0, 1.0]
    );
    // The coordinates, each from its field.
    let s = decoded::State {
        m: [0.2, 0.3, 0.5],
        alpha: 0.6,
        beta: 1.1,
        rho_t: [0.6_f64.cos(), 0.0],
        lambda_t: [0.6_f64.sin() * 1.1_f64.cos(), 0.6_f64.sin() * 1.1_f64.sin()],
        p_rho: [0.11, 0.12],
        p_lambda: [0.13, 0.14],
        l_z: 0.15,
    };
    assert_eq!(
        Set::Jacobi.coordinates(&s),
        [0.6, 1.1, 0.11, 0.12, 0.13, 0.14, 0.2, 0.3]
    );
    let sphere = Sphere::of(&s);
    assert_eq!(
        Set::Shape.coordinates(&s),
        [
            sphere.theta,
            sphere.phi,
            sphere.theta_dot,
            sphere.phi_dot,
            0.15,
            sphere.r_dot,
            0.2,
            0.3
        ]
    );
    // The bound behind √21: at the simplex's centre, with every momentum component at q_max, |ρ̃̇|² + |λ̃̇|².
    let (mu_rho, mu_lambda) = (1.0 / 6.0, 2.0 / 9.0);
    let q: f64 = 3.0;
    let bound = (2.0 * q * q / mu_rho + 2.0 * q * q / mu_lambda).sqrt();
    assert!((bound - r).abs() <= 1e-12 * r);
    // Control: a range recomputed from the view moves when the slice moves; these do not read the state at all.
    rejects("a range that moves with q_max alone being wrong", || {
        assert_eq!(Set::Shape.ranges(2.0), Set::Shape.ranges(3.0))
    });
}

/// The state at time `t` along its rates: `ρ̃ + t ρ̃̇`, `λ̃ + t λ̃̇`.
fn advanced(s: &decoded::State, t: f64) -> decoded::State {
    let m01 = s.m[0] + s.m[1];
    let (a, b) = ((s.m[0] * s.m[1] / m01).sqrt(), (s.m[2] * m01).sqrt());
    decoded::State {
        rho_t: [
            s.rho_t[0] + t * s.p_rho[0] / a,
            s.rho_t[1] + t * s.p_rho[1] / a,
        ],
        lambda_t: [
            s.lambda_t[0] + t * s.p_lambda[0] / b,
            s.lambda_t[1] + t * s.p_lambda[1] / b,
        ],
        ..*s
    }
}

#[test]
fn compass_latent_jacobian_the_shape_rates_are_d_dt_of_the_state() {
    let links = Links::defaults();
    let mut rng = Rng::new(61);
    for _ in 0..50 {
        let z = rng.latent();
        let s = decoded::state(&z, &links, None);
        let sphere = Sphere::of(&s);
        // R-14's convention, from the mass-weighted vectors.
        let (a, b) = (
            s.rho_t[0] * s.rho_t[0] + s.rho_t[1] * s.rho_t[1],
            s.lambda_t[0] * s.lambda_t[0] + s.lambda_t[1] * s.lambda_t[1],
        );
        let i = a + b;
        assert!((i - 1.0).abs() <= 1e-12, "the scale gauge: I = {i}");
        let (u, v, w) = (
            (a - b) / i,
            2.0 * (s.rho_t[0] * s.lambda_t[0] + s.rho_t[1] * s.lambda_t[1]) / i,
            2.0 * (s.rho_t[0] * s.lambda_t[1] - s.rho_t[1] * s.lambda_t[0]) / i,
        );
        assert!((sphere.phi.cos() - w).abs() <= 1e-12, "φ from +w");
        assert!((sphere.theta - v.atan2(u)).abs() <= 1e-12, "θ");
        let h = 1e-6;
        let (up, down) = (Sphere::of(&advanced(&s, h)), Sphere::of(&advanced(&s, -h)));
        let tol = 1e-5 / sphere.sin_phi.max(0.01).powi(2);
        let theta_rate = {
            let d = up.theta - down.theta;
            let wrapped = d - std::f64::consts::TAU * (d / std::f64::consts::TAU).round();
            wrapped / (2.0 * h)
        };
        assert!(
            (theta_rate - sphere.theta_dot).abs() <= tol,
            "θ̇: {theta_rate} against {}",
            sphere.theta_dot
        );
        let phi_rate = (up.phi - down.phi) / (2.0 * h);
        assert!(
            (phi_rate - sphere.phi_dot).abs() <= tol,
            "φ̇: {phi_rate} against {}",
            sphere.phi_dot
        );
        // Ṙ, with R = √I.
        let root_i = |x: &decoded::State| {
            (x.rho_t[0] * x.rho_t[0]
                + x.rho_t[1] * x.rho_t[1]
                + x.lambda_t[0] * x.lambda_t[0]
                + x.lambda_t[1] * x.lambda_t[1])
                .sqrt()
        };
        let r_rate = (root_i(&advanced(&s, h)) - root_i(&advanced(&s, -h))) / (2.0 * h);
        assert!((r_rate - sphere.r_dot).abs() <= 1e-6, "Ṙ");
        // L_z in the mass-weighted vectors, and the decoder's own conjugacy.
        let (ra, rb) = (
            (s.m[0] * s.m[1] / (s.m[0] + s.m[1])).sqrt(),
            (s.m[2] * (s.m[0] + s.m[1])).sqrt(),
        );
        let l = s.rho_t[0] * s.p_rho[1] / ra - s.rho_t[1] * s.p_rho[0] / ra
            + s.lambda_t[0] * s.p_lambda[1] / rb
            - s.lambda_t[1] * s.p_lambda[0] / rb;
        assert!((l - s.l_z).abs() <= 1e-12, "L_z: {l} against {}", s.l_z);
    }
    // At the exact pole the angles' rates are defined as 0, never NaN.
    let pole = decoded::State {
        m: [0.25, 0.25, 0.5],
        alpha: 0.0,
        beta: 0.0,
        rho_t: [0.0, 0.0],
        lambda_t: [0.0, 0.0],
        p_rho: [0.1, 0.2],
        p_lambda: [0.3, 0.4],
        l_z: 0.0,
    };
    // A state with u = v = 0 by hand: ρ̃ ⟂ λ̃ at equal lengths.
    let at = decoded::State {
        rho_t: [1.0, 0.0],
        lambda_t: [0.0, 1.0],
        ..pole
    };
    let sphere = Sphere::of(&at);
    assert_eq!(
        (sphere.theta, sphere.theta_dot, sphere.phi_dot),
        (0.0, 0.0, 0.0)
    );
    assert_eq!(sphere.sin_phi, 0.0);
    assert_eq!(sphere.phi, 0.0, "the +w pole");
    // Control: the sphere's rate formula unguarded at the pole is NaN.
    let zero = sphere.sin_phi;
    rejects("an unguarded rate", || assert!((zero / zero).is_finite()));
}

#[test]
fn compass_latent_jacobian_svd_reconstructs_in_order() {
    let mut rng = Rng::new(71);
    for _ in 0..20 {
        let j: Matrix = std::array::from_fn(|_| rng.latent());
        let d = svd(&j);
        for k in 1..LATENT_DIM {
            assert!(d.sigma[k - 1] >= d.sigma[k], "descending");
        }
        for (r, row) in j.iter().enumerate() {
            for (c, x) in row.iter().enumerate() {
                let rebuilt: f64 = (0..LATENT_DIM)
                    .map(|k| d.u[k][r] * d.sigma[k] * d.v[k][c])
                    .sum();
                assert!((rebuilt - x).abs() <= 1e-12, "J at ({r}, {c})");
            }
        }
        for a in 0..LATENT_DIM {
            for b in 0..LATENT_DIM {
                let want = if a == b { 1.0 } else { 0.0 };
                let (uu, vv): (f64, f64) = (
                    d.u[a].iter().zip(&d.u[b]).map(|(x, y)| x * y).sum(),
                    d.v[a].iter().zip(&d.v[b]).map(|(x, y)| x * y).sum(),
                );
                assert!((uu - want).abs() <= 1e-12, "UᵀU");
                assert!((vv - want).abs() <= 1e-12, "VᵀV");
            }
        }
    }
    // A zero column: its singular value is exactly 0, and its left vector zero, never NaN.
    let mut zero: Matrix = std::array::from_fn(|r| std::array::from_fn(|c| f64::from(r == c)));
    for row in &mut zero {
        row[3] = 0.0;
    }
    let d = svd(&zero);
    assert_eq!(d.sigma[7], 0.0);
    assert_eq!(d.u[7], [0.0; LATENT_DIM]);
    // Control: a matrix's Vᵀ check on a matrix not orthonormal.
    rejects("a Σ out of order", || {
        let mut s = d.sigma;
        s.swap(0, 7);
        assert!(s[0] >= s[7]);
    });
}

#[test]
fn compass_latent_jacobian_the_smaller_singular_value_is_the_cells_light() {
    // [x, y] with known singular values: orthogonal lengths 3 and 0.5.
    let mut x = [0.0; LATENT_DIM];
    let mut y = [0.0; LATENT_DIM];
    x[0] = 3.0;
    y[1] = 0.5;
    assert_eq!(smaller_singular(&x, &y), 0.5);
    assert_eq!(smaller_singular(&y, &x), 0.5);
    assert_eq!(smaller_singular(&x, &x), 0.0, "parallel");
    assert_eq!(smaller_singular(&[0.0; LATENT_DIM], &y), 0.0, "one zero");
    // The projector's sign: on an identity J, P e_i = e_i, and J⁺ e_i = e_i.
    let identity: Matrix = std::array::from_fn(|r| std::array::from_fn(|c| f64::from(r == c)));
    let inverse = Inverse::of(&identity, CUT);
    let e2: Latent = std::array::from_fn(|k| f64::from(k == 2));
    assert_eq!(inverse.projected(2), e2);
    assert_eq!(inverse.preimage(2), e2);
    // Control: the larger is not the smaller.
    rejects("the larger singular value", || {
        assert_eq!(smaller_singular(&x, &y), 3.0)
    });
}

#[test]
fn compass_latent_jacobian_the_boundaries_hold() {
    use crate::explore::compass::physical::{POLE_SIN, TOLERANCE};
    use std::f64::consts::PI;
    // θ's row differs on the circle, both ways round; no other row, nor Jacobi's first, does.
    let (a, b) = (PI - 0.01, -PI + 0.01);
    assert!((difference(Set::Shape, 0, a, b) - 0.02).abs() <= 1e-12);
    assert!((difference(Set::Shape, 0, b, a) + 0.02).abs() <= 1e-12);
    assert_eq!(difference(Set::Shape, 0, 0.1, 0.4), 0.4 - 0.1);
    assert_eq!(difference(Set::Shape, 1, a, b), b - a);
    assert_eq!(difference(Set::Jacobi, 0, a, b), b - a);
    assert!(!refuses(TOLERANCE), "at the tolerance: accepted");
    assert!(refuses(TOLERANCE * (1.0 - f64::EPSILON)));
    assert!(!near_pole(POLE_SIN), "at the radius: not at the pole");
    assert!(near_pole(POLE_SIN * (1.0 - f64::EPSILON)));
    rejects("a θ difference off the circle", || {
        assert!((difference(Set::Jacobi, 0, a, b) - 0.02).abs() <= 1e-12)
    });
}
