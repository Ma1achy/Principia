//! QA's tests for TASK-M2-22, REQ-SYS-014 (coordinate conventions note, "The three coordinate spaces"; chart_decoder
//! contract Part 3; R-83), written from the requirement: the view's addresses stay unsigned `[0,1]²`, and IC space is
//! signed and centred on `z₀` by the placement `z₀ + (2s−1)·q₁ + (2t−1)·q₂`.
//!
//! - UV `(½, ½)` is `z₀` and UV `(0, 0)` is `z₀ − q₁ − q₂`, at every centre and basis, at f64 and at f32;
//! - the address → IC-offset map does not depend on `z₀`, so a view at any centre, however negative, is addressed by
//!   the same non-negative `[0,1]²` addresses and unsigned quad indices, and recovering the address of any IC in the
//!   view gives a non-negative one.

use kernel::chart::{Chart as _, Latent};
use validation::negative_control;

type Z = [f64; 8];

/// A deterministic pseudo-random vector in `[−1, 1]⁸` (a 64-bit LCG).
fn draw(seed: &mut u64) -> Z {
    std::array::from_fn(|_| {
        *seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((*seed >> 11) as f64 / (1u64 << 53) as f64) * 2.0 - 1.0
    })
}

/// Charts at centres from far negative to far positive, with generic non-orthogonal bases of mixed scale.
fn charts() -> Vec<Latent<f64>> {
    let mut seed = 0x5_014_u64;
    let mut out = Vec::new();
    for shift in [-1e6, -37.5, -1.0, 0.0, 2.0, 1e6] {
        for zoom in [1.0, 0.03125] {
            let z0 = draw(&mut seed).map(|x| x + shift);
            let q1 = draw(&mut seed).map(|x| x * zoom);
            let q2 = draw(&mut seed).map(|x| x * zoom);
            out.push(Latent { z0, q1, q2 });
        }
    }
    out
}

fn place(c: &Latent<f64>, s: f64, t: f64) -> Z {
    c.map::<f64>(s, t).z.z
}

/// UV `(½, ½)` → `z₀` and UV `(0, 0)` → `z₀ − q₁ − q₂` under `place`.
fn check_centre_and_corner(c: &Latent<f64>, place: fn(&Latent<f64>, f64, f64) -> Z) {
    assert_eq!(place(c, 0.5, 0.5), c.z0, "UV (½, ½) does not land on z₀");
    let corner = place(c, 0.0, 0.0);
    for (k, got) in corner.iter().enumerate() {
        let want = c.z0[k] - c.q1[k] - c.q2[k];
        assert!(
            (got - want).abs() <= 1e-15 * want.abs().max(1.0),
            "UV (0, 0) is not z₀ − q₁ − q₂ at component {k}: {got} vs {want}"
        );
    }
}

#[test]
fn qa_uv_ic_space_centre_and_corner() {
    for c in charts() {
        check_centre_and_corner(&c, place);
        // The far corner and the signed IC space: UV (1, 1) is z₀ + q₁ + q₂.
        let far = place(&c, 1.0, 1.0);
        for (k, got) in far.iter().enumerate() {
            let want = c.z0[k] + c.q1[k] + c.q2[k];
            assert!((got - want).abs() <= 1e-15 * want.abs().max(1.0));
        }
    }
}

negative_control!(
    qa_uv_ic_space_centre_and_corner,
    "an unsigned placement z₀ + s·q₁ + t·q₂ puts UV (½, ½) off z₀",
    expected = "does not land on z₀",
    check_centre_and_corner(&charts()[0], |c, s, t| std::array::from_fn(|k| c.z0[k]
        + s * c.q1[k]
        + t * c.q2[k]))
);

/// Chart `c` narrowed to f32.
fn narrow(c: &Latent<f64>) -> Latent<f32> {
    Latent {
        z0: c.z0.map(|x| x as f32),
        q1: c.q1.map(|x| x as f32),
        q2: c.q2.map(|x| x as f32),
    }
}

fn place_f32(c: &Latent<f32>, s: f32, t: f32) -> [f32; 8] {
    c.map::<f32>(s, t).z.z
}

/// UV `(½, ½)` → `z₀` exactly at f32 under `place`.
fn check_centre_f32(c: &Latent<f32>, place: fn(&Latent<f32>, f32, f32) -> [f32; 8]) {
    assert_eq!(place(c, 0.5, 0.5), c.z0, "UV (½, ½) is not z₀ at f32");
}

#[test]
fn qa_uv_ic_space_centre_is_z0_at_f32() {
    for c in charts() {
        check_centre_f32(&narrow(&c), place_f32);
    }
}

negative_control!(
    qa_uv_ic_space_centre_is_z0_at_f32,
    "an unsigned f32 placement z₀ + s·q₁ + t·q₂ puts UV (½, ½) off z₀",
    expected = "is not z₀ at f32",
    check_centre_f32(&narrow(&charts()[0]), |c, s, t| std::array::from_fn(|k| c
        .z0[k]
        + s * c.q1[k]
        + t * c.q2[k]))
);

/// The address `(s, t)` of the IC `z` of chart `c`, solved from `z − z₀ = (2s−1)·q₁ + (2t−1)·q₂` by the normal
/// equations.
fn address(c: &Latent<f64>, z: &Z) -> (f64, f64) {
    let dot = |a: &Z, b: &Z| a.iter().zip(b).map(|(x, y)| x * y).sum::<f64>();
    let d: Z = std::array::from_fn(|k| z[k] - c.z0[k]);
    let (g11, g12, g22) = (dot(&c.q1, &c.q1), dot(&c.q1, &c.q2), dot(&c.q2, &c.q2));
    let (b1, b2) = (dot(&c.q1, &d), dot(&c.q2, &d));
    let det = g11 * g22 - g12 * g12;
    (
        ((b1 * g22 - b2 * g12) / det + 1.0) / 2.0,
        ((b2 * g11 - b1 * g12) / det + 1.0) / 2.0,
    )
}

/// Every quad at depth `depth`, indexed by unsigned `(i, j)`, holds the view's ICs at non-negative addresses, and its
/// IC offsets from `z₀` are the same at every centre.
fn check_addresses(c: &Latent<f64>, origin: &Latent<f64>, depth: u32) {
    let n: u32 = 1 << depth;
    for i in 0..n {
        for j in 0..n {
            let (s, t) = (
                (f64::from(i) + 0.5) / f64::from(n),
                (f64::from(j) + 0.5) / f64::from(n),
            );
            let z = place(c, s, t);
            let (a, b) = address(c, &z);
            let tol = 1e-9 * c.z0.iter().fold(1.0_f64, |m, x| m.max(x.abs()));
            assert!(
                a >= 0.0 && b >= 0.0 && a <= 1.0 && b <= 1.0,
                "quad ({i}, {j}) holds an IC at the address ({a}, {b}), outside unsigned [0,1]²"
            );
            assert!((a - s).abs() <= tol && (b - t).abs() <= tol);
            let here: Z = std::array::from_fn(|k| z[k] - c.z0[k]);
            let there: Z = std::array::from_fn(|k| place(origin, s, t)[k] - origin.z0[k]);
            for k in 0..8 {
                assert!(
                    (here[k] - there[k]).abs() <= tol,
                    "the offset of address ({s}, {t}) depends on z₀"
                );
            }
        }
    }
}

#[test]
fn qa_uv_ic_space_addresses_unsigned_at_any_z0() {
    for c in charts() {
        let origin = Latent { z0: [0.0; 8], ..c };
        check_addresses(&c, &origin, 3);
        // IC space is signed: at a far-negative centre every IC in the view is negative, its address is not.
        if c.z0.iter().all(|x| *x < -1e5) {
            assert!(place(&c, 1.0, 1.0).iter().all(|x| *x < 0.0));
        }
    }
}

negative_control!(
    qa_uv_ic_space_addresses_unsigned_at_any_z0,
    "addressing the view by IC value, not offset from z₀, needs negative addresses at a negative centre",
    expected = "outside unsigned [0,1]²",
    {
        let c = charts()[0];
        // Treat the IC itself as the address: z₀ = −1e6 gives a negative one.
        let z = place(&c, 0.5, 0.5);
        let (a, b) = (z[0], z[1]);
        assert!(
            a >= 0.0 && b >= 0.0 && a <= 1.0 && b <= 1.0,
            "quad (0, 0) holds an IC at the address ({a}, {b}), outside unsigned [0,1]²"
        );
    }
);
