//! qa for TASK-M3-72: (a1)'s precision floor, as dd_integrator §3.8 states it (R-974, R-72; REQ-VAL-038), written
//! from the definition, not from the code.
//! - `qa_m372_a1_floor` (§3.8 "(a1)'s precision floor"): at f32, where each pair's displacement `eₖ` lies below half
//!   the rounding step of its separation `dₖ` (`‖eₖ‖ ≲ ε‖dₖ‖/2`), `dₖ + eₖ` rounds to `dₖ` and (a1)'s difference is
//!   exactly 0, per pair and in the accelerations' change: the shadow drifts force-free. Checked at displacements of
//!   1e-9, 1e-12 and 1e-20 in many directions; above the floor (1e-5) the difference is not 0. Control: (a2) held to
//!   the floor, its difference formed from `eₖ` and not 0 there, fails.
//! - `qa_m372_a2_below_floor` (§3.8: "(a2), formed from `eₖ`, keeps about `ε` relative whatever `‖eₖ‖/‖dₖ‖`"): at
//!   f32, at the same displacements below (a1)'s floor, (a2)'s per-pair difference is within `256·ε₃₂` of the
//!   linearised difference `e/n³ − 3d(d·e)/n⁵` formed in f64 (its second-order remainder, `‖e‖/‖d‖ ≤ 1e-8` relative,
//!   far below the bound). Control: (a1), which reads 0 there, fails.
//!
//! **Domain** (pitfalls §10): a fixed triangle with every separation component at least 0.4 in magnitude, so the
//! floor is at least `ε₃₂·0.4/2 ≈ 2.4e-8` per component; a displacement of size `s` puts each `eₖ` component within
//! `2s`.

use kernel::physics::difference::{CancellationFree, ForceDifference, Plain};
use kernel::physics::PairGeom;
use validation::negative_control;

/// The triangle: separations (−0.5, 1.0), (−0.6, −0.6), (1.1, −0.4), every component at least 0.4 in magnitude.
const R: [[f32; 2]; 3] = [[-0.5, 0.1], [0.6, -0.3], [0.1, 0.7]];
const M: [f32; 3] = [0.5, 0.3, 0.2];

/// Displacement sizes below (a1)'s floor at the triangle's separations.
const BELOW: [f32; 3] = [1e-9, 1e-12, 1e-20];

/// A deterministic displacement of the three bodies, each component in `[−size, size]`, not all zero.
fn displacements(size: f32, seed: &mut u64) -> [[f32; 2]; 3] {
    let mut next = || {
        *seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        // In [−1, 1), never exactly 0 (an odd 23-bit numerator over 2^22, less 1).
        (((*seed >> 41) as i64 | 1) as f64 / (1u64 << 22) as f64 - 1.0) as f32
    };
    [
        [size * next(), size * next()],
        [size * next(), size * next()],
        [size * next(), size * next()],
    ]
}

/// Pair `k`'s displacement `eₖ = δrⱼ − δrᵢ`, R-22's `(i, j) = (1, 2), (2, 0), (0, 1)`.
fn pair_e(dr: &[[f32; 2]; 3]) -> [[f32; 2]; 3] {
    [
        [dr[2][0] - dr[1][0], dr[2][1] - dr[1][1]],
        [dr[0][0] - dr[2][0], dr[0][1] - dr[2][1]],
        [dr[1][0] - dr[0][0], dr[1][1] - dr[0][1]],
    ]
}

fn terms<D: ForceDifference>(g: &PairGeom<f32>, e: &[[f32; 2]; 3]) -> [[f32; 2]; 3] {
    let (sep, d2, norm, cube) = (g.sep(), g.d2(), g.norm(), g.cube());
    [0, 1, 2].map(|k| D::term(sep[k], d2[k], norm[k], cube[k], e[k]))
}

fn check_floor<D: ForceDifference>() {
    let g = PairGeom::new(&R);
    // The fixture: every separation component is at least 0.4 in magnitude.
    for d in g.sep() {
        assert!(
            d.iter().all(|c| c.abs() >= 0.4),
            "fixture: a separation {d:?} has a component below 0.4"
        );
    }
    let mut seed = 72u64;
    for size in BELOW {
        for _ in 0..64 {
            let dr = displacements(size, &mut seed);
            let e = pair_e(&dr);
            // The fixture: each `eₖ` component is below half the rounding step of its separation's component.
            for (k, (d, ek)) in g.sep().iter().zip(&e).enumerate() {
                for (c, (dc, ec)) in d.iter().zip(ek).enumerate() {
                    // Half the rounding step of `dc` is at least `ε·|dc|/4`.
                    let half_step = f32::EPSILON * dc.abs() / 4.0;
                    assert!(
                        *ec != 0.0 && ec.abs() < half_step,
                        "fixture: e[{k}][{c}] = {ec:e} is not below the floor {half_step:e}"
                    );
                }
            }
            for (k, du) in terms::<D>(&g, &e).iter().enumerate() {
                assert!(
                    du[0] == 0.0 && du[1] == 0.0,
                    "pair {k}'s difference at ‖δr‖ ≈ {size:e} is {du:?}: it is not 0 below the floor"
                );
            }
            let da = D::accel(&M, &g, &dr);
            assert!(
                da.iter().flatten().all(|&v| v == 0.0),
                "the accelerations' change at ‖δr‖ ≈ {size:e} is {da:?}: it is not 0 below the floor"
            );
        }
    }
    // Above the floor the difference is not 0: the floor is a floor, not a difference that is always 0.
    let dr = displacements(1e-5, &mut seed);
    let du = terms::<D>(&g, &pair_e(&dr));
    assert!(
        du.iter().all(|t| t[0] != 0.0 || t[1] != 0.0),
        "above the floor (‖δr‖ ≈ 1e-5) a pair's difference is 0: {du:?}"
    );
}

#[test]
fn qa_m372_a1_floor() {
    check_floor::<Plain>();
}

negative_control!(
    qa_m372_a1_floor,
    "(a2), formed from e and not 0 below the floor, held to (a1)'s floor must fail",
    expected = "it is not 0 below the floor",
    check_floor::<CancellationFree>()
);

/// The linearised `u(d + e) − u(d)`, `e/n³ − 3d(d·e)/n⁵`, in f64.
fn linear(d: [f32; 2], e: [f32; 2]) -> [f64; 2] {
    let (d, e) = (d.map(f64::from), e.map(f64::from));
    let n2 = d[0] * d[0] + d[1] * d[1];
    let n = n2.sqrt();
    let (n3, n5) = (n2 * n, n2 * n2 * n);
    let de = d[0] * e[0] + d[1] * e[1];
    [0, 1].map(|c| e[c] / n3 - 3.0 * d[c] * de / n5)
}

const BOUND: f64 = 256.0 * f32::EPSILON as f64;

fn check_below_floor<D: ForceDifference>() {
    let g = PairGeom::new(&R);
    let mut seed = 271u64;
    for size in BELOW {
        for _ in 0..64 {
            let e = pair_e(&displacements(size, &mut seed));
            let got = terms::<D>(&g, &e);
            for (k, ((d, ek), du)) in g.sep().iter().zip(&e).zip(&got).enumerate() {
                let want = linear(*d, *ek);
                let err = (f64::from(du[0]) - want[0]).hypot(f64::from(du[1]) - want[1]);
                let rel = err / want[0].hypot(want[1]);
                assert!(
                    rel <= BOUND,
                    "pair {k} at ‖δr‖ ≈ {size:e}: relative error {rel:e} against the linearised difference, above \
                     {BOUND:e}"
                );
            }
        }
    }
}

#[test]
fn qa_m372_a2_below_floor() {
    check_below_floor::<CancellationFree>();
}

negative_control!(
    qa_m372_a2_below_floor,
    "(a1), which reads 0 below its floor, held to (a2)'s precision must fail",
    expected = "relative error",
    check_below_floor::<Plain>()
);
