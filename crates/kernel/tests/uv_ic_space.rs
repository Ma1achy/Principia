//! UV addressing and IC space (coordinate conventions note, "The three coordinate spaces"; chart_decoder_contract
//! Part 3; R-83; REQ-SYS-014; TASK-M2-22): the view's addresses are unsigned, `[0,1]²`, and the latent chart's
//! placement `z(s,t) = z₀ + (2s−1)·q₁ + (2t−1)·q₂` puts them into IC space signed and centred on `z₀`, the scale in `q`.
//!
//! - UV `(½, ½)` is `z₀`, whatever the basis;
//! - UV `(0, 0)` is `z₀ − q₁ − q₂` and UV `(1, 1)` is `z₀ + q₁ + q₂`;
//! - at any `z₀`, the address of every IC of the view is non-negative, and so is its quad index: the sign lives in the
//!   placement, never in the address.

use kernel::chart::{Chart, Latent};
use validation::negative_control;

type Z = [f64; 8];

/// Centres across IC space: the golden IC at the origin, far negative, far positive, mixed.
const CENTRES: [Z; 4] = [
    [0.0; 8],
    [-1e3; 8],
    [1e3; 8],
    [-5.0, 2.5, -0.75, 4.0, -3.0, 0.0, -1.25, 6.0],
];

/// An oblique basis, zoomed: the scale lives in `q` (R-83).
fn basis() -> (Z, Z) {
    (
        [0.3, -0.1, 0.2, 0.0, 0.05, -0.4, 0.1, 0.2],
        [-0.1, 0.25, 0.1, 0.3, -0.2, 0.0, 0.15, -0.05],
    )
}

fn chart(z0: Z) -> Latent<f64> {
    let (q1, q2) = basis();
    Latent { z0, q1, q2 }
}

/// The IC at the view address `(s, t)`.
fn ic(chart: &Latent<f64>, s: f64, t: f64) -> Z {
    chart.map::<f64>(s, t).z.z
}

#[test]
fn uv_ic_space_centre_is_z0() {
    for z0 in CENTRES {
        assert_eq!(ic(&chart(z0), 0.5, 0.5), z0, "UV (½, ½) is not z₀");
    }
}

negative_control!(
    uv_ic_space_centre_is_z0,
    "the corner is not the centre",
    expected = "is not z₀",
    {
        let z0 = CENTRES[1];
        assert_eq!(ic(&chart(z0), 0.0, 0.5), z0, "UV (½, ½) is not z₀");
    }
);

/// A placement of the view address `(s, t)` into IC space.
type Place = fn(&Latent<f64>, f64, f64) -> Z;

/// UV `(0, 0)` is `z₀ − q₁ − q₂` and UV `(1, 1)` is `z₀ + q₁ + q₂` on `c` under `place`.
fn check_corners(c: &Latent<f64>, place: Place) {
    let low: Z = std::array::from_fn(|i| c.z0[i] - c.q1[i] - c.q2[i]);
    let high: Z = std::array::from_fn(|i| c.z0[i] + c.q1[i] + c.q2[i]);
    assert_eq!(place(c, 0.0, 0.0), low, "UV (0, 0) is not z₀ − q₁ − q₂");
    assert_eq!(place(c, 1.0, 1.0), high, "UV (1, 1) is not z₀ + q₁ + q₂");
}

#[test]
fn uv_ic_space_corner_is_z0_minus_q1_minus_q2() {
    for z0 in CENTRES {
        check_corners(&chart(z0), ic);
        check_corners(&chart(z0).zoomed(0.125), ic);
    }
}

negative_control!(
    uv_ic_space_corner_is_z0_minus_q1_minus_q2,
    "an unsigned placement, z₀ + s·q₁ + t·q₂, puts UV (0, 0) at z₀",
    expected = "is not z₀ − q₁ − q₂",
    check_corners(&chart(CENTRES[3]), |c, s, t| std::array::from_fn(|i| c.z0
        [i]
        + s * c.q1[i]
        + t * c.q2[i]))
);

/// The view address of the IC `z` of chart `c`: `(s, t)` with `z − z₀ = (2s−1)·q₁ + (2t−1)·q₂`, solved by the 2×2
/// normal equations of the basis.
fn address(c: &Latent<f64>, z: &Z) -> (f64, f64) {
    let dot = |a: &Z, b: &Z| a.iter().zip(b).map(|(x, y)| x * y).sum::<f64>();
    let d: Z = std::array::from_fn(|i| z[i] - c.z0[i]);
    let (g11, g12, g22) = (dot(&c.q1, &c.q1), dot(&c.q1, &c.q2), dot(&c.q2, &c.q2));
    let (b1, b2) = (dot(&c.q1, &d), dot(&c.q2, &d));
    let det = g11 * g22 - g12 * g12;
    let a = (b1 * g22 - b2 * g12) / det;
    let b = (b2 * g11 - b1 * g12) / det;
    ((a + 1.0) / 2.0, (b + 1.0) / 2.0)
}

/// The address `(s, t)` and its quad index at depth `depth` are non-negative, to the rounding `tol` of recovering
/// the address from an IC.
fn check_non_negative(s: f64, t: f64, depth: u32) {
    let tol = 1e-9;
    let cells = f64::from(2u32.pow(depth));
    let index = [(s * cells + tol).floor(), (t * cells + tol).floor()];
    assert!(
        s >= -tol && t >= -tol && index.iter().all(|i| *i >= 0.0),
        "a negative address ({s}, {t}), quad {index:?}"
    );
}

#[test]
fn uv_ic_space_addresses_are_non_negative_at_any_z0() {
    let grid = [0.0, 0.125, 0.5, 0.875, 1.0];
    for z0 in CENTRES {
        let c = chart(z0);
        for s in grid {
            for t in grid {
                let (a, b) = address(&c, &ic(&c, s, t));
                assert!(
                    (a - s).abs() <= 1e-9 && (b - t).abs() <= 1e-9,
                    "the address ({a}, {b}) is not the view's ({s}, {t})"
                );
                check_non_negative(a, b, 6);
            }
        }
        // IC space is signed: a view centred far negative holds negative ICs at non-negative addresses.
        if z0[0] < 0.0 {
            assert!(ic(&c, 1.0, 1.0)[0] < 0.0, "IC space is not signed");
        }
    }
}

negative_control!(
    uv_ic_space_addresses_are_non_negative_at_any_z0,
    "an address read as the IC itself, not its offset from z₀, goes negative at a negative z₀",
    expected = "a negative address",
    {
        let c = chart(CENTRES[1]);
        let z = ic(&c, 0.5, 0.5);
        check_non_negative(z[0], z[1], 6);
    }
);
