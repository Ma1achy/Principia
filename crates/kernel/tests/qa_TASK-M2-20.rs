//! QA tests for TASK-M2-20 (REQ-ENC-025, REQ-VAL-014), written from inverse-encode contract Part 4, "The conditioning
//! number `κ`", not from `kernel::encode::ingest`'s code.
//!
//! - **The section identity.** On the section `E ∘ D` is the identity, so `J_E(D(z))·J_D(z) = I` (Part 4, "The
//!   section's tangent is a lower bound, not the bound"), and with the weight `W` at `x = D(z)`,
//!   `(J_E·W⁻¹)·(W·J_D) = I₈`. `weighted_encode_jacobian` (the encode side, read at `x`) and `weighted_jacobian` (the
//!   decode side, read at `z`) are formed by different differences of different maps, so their product is an oracle
//!   for each: a wrong weight on either side, a wrong step, or a column read off the wrong component leaves it away
//!   from `I₈`.
//! - **The lower bound.** Hence `κ(D(z)) ≥ 1/σ_min(W·J_D(z))`: checked on interior states of the section, the
//!   canonical representatives of the Lagrange, Burrau and light starts and on hand-picked `z`.
//!
//! **Tolerances** (applied per R-369, the test's decisions): both Jacobians are Part 4's central differences at
//! `hₖ = ε^{1/3}·max(1, |·ₖ|)`, whose truncation and rounding errors are each `O(ε^{2/3})` relative to the entries
//! they difference. The checks allow `ε^{1/3}` (≈ 6·10⁻⁶), the step itself, relative to the two factors' norms: the
//! identity's worst entry was measured at 10⁻¹¹ to 6·10⁻¹⁰ of that product at the interior points and 7·10⁻⁸ at the
//! clamp-ward `z` (`κ` ≈ 2·10³), and the controls' faults move it by 10⁻¹ or more.

use kernel::decode::{self, Ic, Latent};
use kernel::encode::encode;
use kernel::encode::ingest::{
    condition, kappa, sigma_max, weighted_encode_jacobian, weighted_jacobian, Phys,
};
use kernel::generated::links::Defaults;
use validation::negative_control;

/// Part 4's relative step, the checks' slack.
fn slack() -> f64 {
    f64::EPSILON.cbrt()
}

/// The equilateral Lagrange start, masses ⅓, in a foreign frame (shifted, rotated, boosted).
fn lagrange() -> Ic<f64> {
    let (c, s) = (0.6_f64.cos(), 0.6_f64.sin());
    let rot = |v: [f64; 2]| [c * v[0] - s * v[1], s * v[0] + c * v[1]];
    let r0 = [
        [1.0, 0.0],
        [-0.5, 0.75_f64.sqrt()],
        [-0.5, -(0.75_f64.sqrt())],
    ];
    let w = 0.7;
    let third = 1.0 / 3.0;
    Ic {
        m: [third; 3],
        r: r0.map(|v| {
            let q = rot(v);
            [q[0] + 0.4, q[1] - 1.2]
        }),
        p: r0.map(|v| {
            let q = rot([-w * v[1], w * v[0]]);
            [third * q[0] + 0.01, third * q[1] - 0.02]
        }),
    }
}

/// Burrau's 3-4-5 rest start, masses `(3, 4, 5)/12` at `(1, 3)`, `(−2, −1)`, `(1, −1)`, at rest.
fn burrau() -> Ic<f64> {
    Ic {
        m: [3.0 / 12.0, 4.0 / 12.0, 5.0 / 12.0],
        r: [[1.0, 3.0], [-2.0, -1.0], [1.0, -1.0]],
        p: [[0.0; 2]; 3],
    }
}

/// Unequal masses with slow momenta (the physics review's light start of PR #294, P1).
fn light() -> Ic<f64> {
    Ic {
        m: [0.05, 0.15, 0.8],
        r: [[0.3, 0.1], [-0.9, 0.4], [0.05, -0.08]],
        p: [[0.001, 0.002], [-0.003, 0.001], [0.002, -0.003]],
    }
}

/// The interior points of the section the checks run on, each as its `z`.
fn points() -> Vec<(&'static str, Latent<f64>)> {
    let mut v: Vec<(&'static str, Latent<f64>)> = [
        ("Lagrange", lagrange()),
        ("Burrau", burrau()),
        ("light", light()),
    ]
    .into_iter()
    .map(|(n, x)| (n, encode::<Defaults, f64>(&x).z))
    .collect();
    v.push((
        "hand-picked z, near the middle",
        Latent::new([0.3, -0.2, 0.5, -0.4, 0.1, 0.2, 0.4, -0.3]),
    ));
    v.push((
        "hand-picked z, toward the clamps",
        Latent::new([1.1, 0.8, -2.0, 1.5, -1.2, 2.2, -0.9, 1.3]),
    ));
    v
}

/// `(J_E(x)·W⁻¹)·(W·J_D(z))`, `x = D(z)`, as an 8 × 8 matrix.
fn product(enc: &[Phys<f64>; 8], dec: &[Phys<f64>; 8]) -> [[f64; 8]; 8] {
    std::array::from_fn(|i| std::array::from_fn(|j| (0..15).map(|k| enc[i][k] * dec[j][k]).sum()))
}

/// `(J_E·W⁻¹)·(W·J_D)` is `I₈` within `slack · ‖J_E·W⁻¹‖₂ · ‖W·J_D‖₂` in every entry, `dec` the decode side's
/// columns at `z` and the encode side read at `D(z)`.
fn check_identity(name: &str, z: &Latent<f64>, dec: [Phys<f64>; 8]) {
    let x = decode::decode::<Defaults, f64>(z).ic;
    let enc = weighted_encode_jacobian::<Defaults, f64>(&x);
    let p = product(&enc, &dec);
    let tol = slack() * sigma_max(enc) * sigma_max(dec);
    for (i, row) in p.iter().enumerate() {
        for (j, v) in row.iter().enumerate() {
            let want = if i == j { 1.0 } else { 0.0 };
            assert!(
                (v - want).abs() <= tol,
                "{name}: entry ({i}, {j}) of (J_E·W⁻¹)·(W·J_D) is {v:e}, not {want} within {tol:e}"
            );
        }
    }
}

#[test]
fn qa_encode_and_decode_jacobians_are_inverse_on_the_section() {
    for (name, z) in points() {
        check_identity(name, &z, weighted_jacobian::<Defaults, f64>(&z));
    }
}

negative_control!(
    qa_encode_and_decode_jacobians_are_inverse_on_the_section,
    "the decode's Jacobian at another point (z_q0 moved by 0.5) is not the encode's inverse at this one",
    expected = "of (J_E·W⁻¹)·(W·J_D) is",
    {
        let (name, z) = points().swap_remove(2);
        let mut moved = z.z;
        moved[2] += 0.5;
        check_identity(
            name,
            &z,
            weighted_jacobian::<Defaults, f64>(&Latent::new(moved)),
        );
    }
);

mod unweighted {
    use super::*;
    negative_control!(
        qa_encode_and_decode_jacobians_are_inverse_on_the_section,
        "the decode's Jacobian with its position rows weighted twice is not the encode's inverse",
        expected = "of (J_E·W⁻¹)·(W·J_D) is",
        {
            let (name, z) = points().swap_remove(1);
            let mut dec = weighted_jacobian::<Defaults, f64>(&z);
            for col in dec.iter_mut() {
                for v in &mut col[3..9] {
                    *v *= 0.5;
                }
            }
            check_identity(name, &z, dec);
        }
    );
}

/// `k ≥ 1/σ_min(W·J_D(z))·(1 − slack)`, the lower bound at `D(z)`.
fn check_lower_bound(name: &str, z: &Latent<f64>, k: f64) {
    let tangent = condition(weighted_jacobian::<Defaults, f64>(z));
    assert!(
        tangent.is_finite() && k >= tangent * (1.0 - slack()),
        "{name}: κ(D(z)) = {k:e} is below the section's tangent bound 1/σ_min(W·J_D) = {tangent:e}"
    );
}

#[test]
fn qa_kappa_is_at_least_the_tangent_bound() {
    for (name, z) in points() {
        let x = decode::decode::<Defaults, f64>(&z).ic;
        check_lower_bound(name, &z, kappa::<Defaults, f64>(&x));
    }
}

negative_control!(
    qa_kappa_is_at_least_the_tangent_bound,
    "nine tenths of the tangent bound is below it",
    expected = "is below the section's tangent bound",
    {
        let (name, z) = points().swap_remove(0);
        let tangent = condition(weighted_jacobian::<Defaults, f64>(&z));
        check_lower_bound(name, &z, 0.9 * tangent);
    }
);
