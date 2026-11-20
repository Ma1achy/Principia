//! QA tests for TASK-M6-35, written from R-550, R-633, REQ-DEC-049, REQ-GUI-199, REQ-GUI-200, REQ-GUI-347 and
//! REQ-GUI-410, not from the implementation:
//!
//! - `J` here is differenced apart from the code: the step of R-633, the coordinates read off `decoded::State`, the
//!   ranges written out as REQ-GUI-347 states them;
//! - the shell's normal comes from the Hamiltonian's own gradient in the scaled Jacobi coordinates, so the steer's
//!   refusal `√(1 − n_i² − n_j²) < 0.1` is predicted without `J⁺`;
//! - the preimage comes from a Gaussian solve, the grid from the raw bivector over its Gram area.
//!
//! Each check runs its control (R-176) through [`rejects`].
// The file name `qa_TASK-M6-35` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use eframe::egui::{self, vec2, Event, Pos2};
use engine::contract::decoded::{self, State};
use engine::contract::log::Source;
use engine::contract::set_field::{Edit, SetField, SimField};
use engine::contract::sim_config::{Links, Plane, SimConfig};
use gui::app::App;
use gui::capture::{self, FORMAT, PIXELS_PER_POINT, SIZE};
use gui::explore::compass::physical::{
    self, latent_jacobian, pushforward, steer, Decoder, Matrix, Refusal, Set,
};
use gui::explore::compass::views::{CompassView, GridPlace};
use gui::explore::compass::{self};
use gui::explore::manifold_view::{basis_edit, z0_edit, View};
use gui::headless::Headless;
use gui::layout::Layout;
use gui::mock::MockEngine;
use gui::side::MockSide;
use std::f64::consts::{FRAC_PI_2, PI, TAU};

type Z = [f64; 8];

fn rejects(what: &str, check: impl FnOnce()) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check));
    assert!(outcome.is_err(), "the check passed on {what}");
}

// --- arithmetic apart from the code ----------------------------------------------------------------------------------

fn dot(a: &Z, b: &Z) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn nrm(a: &Z) -> f64 {
    dot(a, a).sqrt()
}

fn apply(a: &Matrix, x: &Z) -> Z {
    a.map(|row| dot(&row, x))
}

fn upper() -> Vec<(usize, usize)> {
    (0..8)
        .flat_map(|i| (i + 1..8).map(move |j| (i, j)))
        .collect()
}

/// A fixed-seed splitmix64 on `[0, 1)`.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut x = self.0;
        x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        x ^= x >> 31;
        (x >> 11) as f64 / (1u64 << 53) as f64
    }
    fn z(&mut self, half: f64) -> Z {
        std::array::from_fn(|_| half * (2.0 * self.next() - 1.0))
    }
    fn matrix(&mut self) -> Matrix {
        std::array::from_fn(|_| self.z(1.0))
    }
}

/// The raw bivector over its Gram area, with the area: needs no orthonormal pair.
fn bivector(u1: &Z, u2: &Z) -> ([[f64; 8]; 8], f64) {
    let area = (dot(u1, u1) * dot(u2, u2) - dot(u1, u2).powi(2)).sqrt();
    let mut g = [[0.0; 8]; 8];
    for (i, j) in upper() {
        g[i][j] = (u1[i] * u2[j] - u1[j] * u2[i]) / area;
    }
    (g, area)
}

/// `q̂₁, q̂₂` by Gram–Schmidt.
fn orthonormal(q1: &Z, q2: &Z) -> (Z, Z) {
    let a = q1.map(|x| x / nrm(q1));
    let k = dot(q2, &a);
    let w: Z = std::array::from_fn(|i| q2[i] - k * a[i]);
    let n = nrm(&w);
    (a, w.map(|x| x / n))
}

/// `A x = b` by Gaussian elimination with partial pivoting.
fn solve(a: &Matrix, b: &Z) -> Z {
    let mut m: [[f64; 9]; 8] =
        std::array::from_fn(|r| std::array::from_fn(|c| if c < 8 { a[r][c] } else { b[r] }));
    for c in 0..8 {
        let p = (c..8)
            .max_by(|&x, &y| m[x][c].abs().total_cmp(&m[y][c].abs()))
            .unwrap();
        m.swap(c, p);
        let pivot = m[c];
        for row in m.iter_mut().skip(c + 1) {
            let f = row[c] / pivot[c];
            row.iter_mut().zip(&pivot).for_each(|(x, y)| *x -= f * y);
        }
    }
    let mut x = [0.0; 8];
    for r in (0..8).rev() {
        let s: f64 = (r + 1..8).map(|k| m[r][k] * x[k]).sum();
        x[r] = (m[r][8] - s) / m[r][r];
    }
    x
}

/// The last pivot over the first in Gaussian elimination with full pivoting: the numerical rank's gauge, no SVD in it.
fn last_pivot_ratio(a: &Matrix) -> f64 {
    let mut m = *a;
    let mut first = 0.0;
    let mut last = 0.0;
    for c in 0..8 {
        let (mut pr, mut pc, mut best) = (c, c, -1.0);
        for (r, row) in m.iter().enumerate().skip(c) {
            for (k, x) in row.iter().enumerate().skip(c) {
                if x.abs() > best {
                    (pr, pc, best) = (r, k, x.abs());
                }
            }
        }
        m.swap(c, pr);
        for row in &mut m {
            row.swap(c, pc);
        }
        if c == 0 {
            first = best;
        }
        last = best;
        if best == 0.0 {
            return 0.0;
        }
        let pivot = m[c];
        for row in m.iter_mut().skip(c + 1) {
            let f = row[c] / pivot[c];
            row.iter_mut().zip(&pivot).for_each(|(x, y)| *x -= f * y);
        }
    }
    last / first
}

// --- R-550's coordinates and REQ-GUI-347's ranges, written out ---------------------------------------------------------

fn q_max() -> f64 {
    Links::defaults().link_params().q_max
}

fn jacobi_of(s: &State) -> Z {
    [
        s.alpha,
        s.beta,
        s.p_rho[0],
        s.p_rho[1],
        s.p_lambda[0],
        s.p_lambda[1],
        s.m[0],
        s.m[1],
    ]
}

fn jacobi_ranges(q: f64) -> Z {
    [FRAC_PI_2, PI, q, q, q, q, 1.0, 1.0]
}

/// R-14's `θ, φ`, the `ρ̃̇ = p_ρ/√μ_ρ`, `λ̃̇ = p_λ/√μ_λ` rates by differencing the free motion of `ρ̃, λ̃`, `L_z` and `Ṙ`.
fn shape_of(s: &State) -> Z {
    let m01 = s.m[0] + s.m[1];
    let (a, b) = ((s.m[0] * s.m[1] / m01).sqrt(), (s.m[2] * m01).sqrt());
    let angles = |t: f64| -> (f64, f64, f64) {
        let r: [f64; 2] = std::array::from_fn(|k| s.rho_t[k] + t * s.p_rho[k] / a);
        let l: [f64; 2] = std::array::from_fn(|k| s.lambda_t[k] + t * s.p_lambda[k] / b);
        let (aa, bb) = (r[0] * r[0] + r[1] * r[1], l[0] * l[0] + l[1] * l[1]);
        let i = aa + bb;
        let u = (aa - bb) / i;
        let v = 2.0 * (r[0] * l[0] + r[1] * l[1]) / i;
        let w = 2.0 * (r[0] * l[1] - r[1] * l[0]) / i;
        (v.atan2(u), u.hypot(v).atan2(w), i.sqrt())
    };
    let dt = 1e-5;
    let (o, x, y) = (angles(0.0), angles(-dt), angles(dt));
    let mut dtheta = y.0 - x.0;
    dtheta -= TAU * ((dtheta + PI) / TAU).floor();
    [
        o.0,
        o.1,
        dtheta / (2.0 * dt),
        (y.1 - x.1) / (2.0 * dt),
        s.l_z,
        (y.2 - x.2) / (2.0 * dt),
        s.m[0],
        s.m[1],
    ]
}

fn shape_ranges(q: f64) -> Z {
    let rate = 21.0_f64.sqrt() * q;
    [TAU, PI, 2.0 * rate, 2.0 * rate, rate, rate, 1.0, 1.0]
}

/// Column-by-column central difference at R-633's step of `f`, each row over `ranges`.
fn own_jacobian(f: impl Fn(&Z) -> Z, ranges: &Z, wrap_first: bool, z: &Z) -> Matrix {
    let mut j = [[0.0; 8]; 8];
    for k in 0..8 {
        let h = f64::EPSILON.powf(1.0 / 3.0) * z[k].abs().max(1.0);
        let (mut up, mut down) = (*z, *z);
        up[k] += h;
        down[k] -= h;
        let taken = up[k] - down[k];
        let (a, b) = (f(&down), f(&up));
        for r in 0..8 {
            let mut d = b[r] - a[r];
            if wrap_first && r == 0 {
                d -= TAU * ((d + PI) / TAU).floor();
            }
            j[r][k] = d / taken / ranges[r];
        }
    }
    j
}

fn real(e_star: Option<f64>) -> Decoder {
    Decoder::Real { e_star }
}

fn own_jacobi_j(e_star: Option<f64>, z: &Z) -> Matrix {
    let links = Links::defaults();
    own_jacobian(
        |x| jacobi_of(&decoded::state(x, &links, e_star)),
        &jacobi_ranges(q_max()),
        false,
        z,
    )
}

fn sin_phi(e_star: Option<f64>, z: &Z) -> f64 {
    decoded::Sphere::of(&decoded::state(z, &Links::defaults(), e_star)).sin_phi
}

// --- REQ-DEC-049 ------------------------------------------------------------------------------------------------------

#[test]
fn qa_compass_latent_jacobian_is_r_550s_on_the_decoder() {
    let links = Links::defaults();
    let mut rng = Rng(0x35);
    let close = |a: &Matrix, b: &Matrix| -> f64 {
        let mut worst = 0.0_f64;
        for r in 0..8 {
            for c in 0..8 {
                worst = worst.max((a[r][c] - b[r][c]).abs());
            }
        }
        worst
    };
    for _ in 0..40 {
        let z = rng.z(3.0);
        for e_star in [None, Some(0.0)] {
            let got = latent_jacobian(&real(e_star), Set::Jacobi, &links, &z);
            let want = own_jacobi_j(e_star, &z);
            // Two evaluations, at the step of R-633, over the ranges: the same to rounding in the order of division.
            assert!(close(&got, &want) <= 1e-13, "Jacobi at {z:?}");
            if sin_phi(e_star, &z) < 0.1 {
                continue;
            }
            let got = latent_jacobian(&real(e_star), Set::Shape, &links, &z);
            // The rates differenced in t carry that difference's own error, so this holds to a bound, not exactly.
            let want = own_jacobian(
                |x| shape_of(&decoded::state(x, &links, e_star)),
                &shape_ranges(q_max()),
                true,
                &z,
            );
            let peak = got.iter().flatten().fold(0.0_f64, |m, x| m.max(x.abs()));
            assert!(close(&got, &want) <= 3e-3 * peak.max(1.0), "Shape at {z:?}");
        }
    }
    // Control: the Jacobian before η_E is not the one after it on an E* chart.
    let z = rng.z(3.0);
    let after = latent_jacobian(&real(Some(0.0)), Set::Jacobi, &links, &z);
    rejects("J before η_E", || {
        assert_eq!(after, own_jacobi_j(None, &z))
    });
}

#[test]
fn qa_compass_latent_jacobian_rank_is_8_unset_and_7_on_the_shell() {
    let links = Links::defaults();
    let mut rng = Rng(0x7);
    for _ in 0..100 {
        let z = rng.z(3.0);
        let unset = latent_jacobian(&real(None), Set::Jacobi, &links, &z);
        let shell = latent_jacobian(&real(Some(0.0)), Set::Jacobi, &links, &z);
        let (u, s) = (last_pivot_ratio(&unset), last_pivot_ratio(&shell));
        assert!(u > physical::CUT, "rank 8 with E* unset: {u:e} at {z:?}");
        assert!(s < physical::CUT, "rank 7 on the shell: {s:e} at {z:?}");
    }
    // Control: the gauge can tell them apart at all; the identity has rank 8, a repeated row does not.
    let mut one: Matrix = std::array::from_fn(|r| std::array::from_fn(|c| f64::from(r == c)));
    assert!(last_pivot_ratio(&one) > 0.5);
    one[7] = one[6];
    rejects("a repeated row at rank 8", || {
        assert!(last_pivot_ratio(&one) > physical::CUT)
    });
}

// --- REQ-GUI-199 on the linear fixture ---------------------------------------------------------------------------------

fn leak(a: Matrix) -> &'static Matrix {
    Box::leak(Box::new(a))
}

fn plane(z0: Z, q1: Z, q2: Z) -> Plane {
    Plane { z0, q1, q2 }
}

/// `A`, over the ranges of `set`, so `J = a` scaled back: the fixture states `J` in the view's scaled coordinates.
fn unscaled(j: &Matrix, set: Set) -> Matrix {
    let r = match set {
        Set::Jacobi => jacobi_ranges(q_max()),
        Set::Shape => shape_ranges(q_max()),
    };
    std::array::from_fn(|row| j[row].map(|x| x * r[row]))
}

#[test]
fn qa_compass_physical_views_the_grid_and_stretch_on_a_known_j() {
    let links = Links::defaults();
    let mut rng = Rng(0x199);
    for set in [Set::Jacobi, Set::Shape] {
        for _ in 0..50 {
            let want_j = rng.matrix();
            let d = Decoder::Linear {
                a: leak(unscaled(&want_j, set)),
                shell: false,
            };
            let (q1, q2) = (rng.z(1.0), rng.z(1.0));
            let p = plane([0.0; 8], q1, q2);
            let j = latent_jacobian(&d, set, &links, &p.z0);
            let got = pushforward(&j, &p);
            let (e1, e2) = orthonormal(&q1, &q2);
            let (grid, area) = bivector(&apply(&want_j, &e1), &apply(&want_j, &e2));
            assert!(!got.degenerate);
            for (a, b) in upper() {
                assert!(
                    (got.grid[a][b] - grid[a][b]).abs() <= 1e-12,
                    "({a}, {b}): {} against {}",
                    got.grid[a][b],
                    grid[a][b]
                );
            }
            assert!((got.stretch - area).abs() <= 1e-12 * area.max(1.0));
            // Unchanged when σ doubles, and when the pair shears within the plane.
            let doubled = plane(p.z0, q1.map(|x| 2.0 * x), q2.map(|x| 2.0 * x));
            let sheared: Z = std::array::from_fn(|i| q2[i] + 0.7 * q1[i]);
            for other in [doubled, plane(p.z0, q1, sheared)] {
                let o = pushforward(&j, &other);
                assert!((o.stretch - got.stretch).abs() <= 1e-12 * area.max(1.0));
                for (a, b) in upper() {
                    assert!((o.grid[a][b] - got.grid[a][b]).abs() <= 1e-12);
                }
            }
            // Control: the stretch on the un-orthonormalised q moves with σ, and differs from R-550's.
            let raw = bivector(&apply(&want_j, &q1), &apply(&want_j, &q2)).1;
            rejects("a stretch on the raw q", || {
                assert!((raw - got.stretch).abs() <= 1e-9)
            });
        }
    }
}

#[test]
fn qa_compass_physical_views_every_row_divides_by_its_range() {
    // The identity in scaled terms: A = diag(ranges), so J is I only if each row is divided by R-550's range.
    for (set, ranges) in [
        (Set::Jacobi, jacobi_ranges(q_max())),
        (Set::Shape, shape_ranges(q_max())),
    ] {
        let a: Matrix =
            std::array::from_fn(|r| std::array::from_fn(|c| if r == c { ranges[r] } else { 0.0 }));
        let d = Decoder::Linear {
            a: leak(a),
            shell: false,
        };
        let j = latent_jacobian(&d, set, &Links::defaults(), &[0.0; 8]);
        for (r, row) in j.iter().enumerate() {
            for (c, x) in row.iter().enumerate() {
                let want = if r == c { 1.0 } else { 0.0 };
                assert!((x - want).abs() <= 1e-10, "{set:?} ({r}, {c}): {x}");
            }
        }
        assert_eq!(set.ranges(q_max()), ranges);
        // Control: a range recomputed at another q_max moves the momenta's rows (REQ-GUI-347's fixed expression).
        rejects("a range at another q_max", || {
            assert_eq!(set.ranges(q_max() * 2.0), ranges)
        });
    }
    assert_eq!(
        Set::Jacobi.names(),
        ["α", "β", "p_ρx", "p_ρy", "p_λx", "p_λy", "m₀/M", "m₁/M"]
    );
    assert_eq!(
        Set::Shape.names(),
        ["θ", "φ", "dθ/dt", "dφ/dt", "L_z", "dR/dt", "m₀/M", "m₁/M"]
    );
}

#[test]
fn qa_compass_physical_views_a_vanishing_wedge_is_degenerate_whatever_js_scale() {
    let mut rng = Rng(0x0DE6);
    for scale in [1e-6, 1.0, 1e6] {
        let mut j = rng.matrix();
        for row in &mut j {
            // Rank 1 on the plane z₀ × z₁: both columns the same.
            row[1] = row[0];
            *row = row.map(|x| x * scale);
        }
        let mut ax = [0.0; 8];
        ax[0] = 1.0;
        let mut ay = [0.0; 8];
        ay[1] = 1.0;
        let got = pushforward(&j, &plane([0.0; 8], ax, ay));
        assert!(got.degenerate, "at scale {scale}: {got:?}");
        assert_eq!(got.grid, [[0.0; 8]; 8], "said, not drawn");
        // A plane it does not flatten stays drawn at that scale: the message is not the scale's.
        let mut other = [0.0; 8];
        other[2] = 1.0;
        let kept = pushforward(&j, &plane([0.0; 8], ax, other));
        assert!(!kept.degenerate, "at scale {scale}");
        rejects("the flattened plane drawn", || assert!(!got.degenerate));
    }
}

#[test]
fn qa_compass_physical_views_are_finite_at_the_decoders_extremes() {
    // Saturated links, exact poles and overflowing latents: a grid is finite or said, and a steer is refused or finite.
    let links = Links::defaults();
    let mut cases: Vec<Z> = Vec::new();
    for x in [20.0, 40.0, 800.0, 1e6, 1e150, 1e300] {
        cases.push([x; 8]);
        cases.push([-x; 8]);
        cases.push(std::array::from_fn(|k| if k % 2 == 0 { x } else { -x }));
        for k in 0..8 {
            cases.push(std::array::from_fn(|i| if i == k { x } else { 0.1 }));
            cases.push(std::array::from_fn(|i| if i == k { -x } else { 0.1 }));
        }
    }
    cases.push([0.0; 8]);
    let mut e1 = [0.0; 8];
    e1[0] = 1.0;
    let mut e2 = [0.0; 8];
    e2[1] = 1.0;
    let mut sim = sim_of_mock();
    for z in cases {
        for e_star in [None, Some(0.0)] {
            for set in [Set::Jacobi, Set::Shape] {
                let d = real(e_star);
                let j = latent_jacobian(&d, set, &links, &z);
                let p = plane(z, e1, e2);
                let push = pushforward(&j, &p);
                assert!(
                    push.degenerate || push.grid.iter().flatten().all(|x| x.is_finite()),
                    "a NaN grid drawn at {z:?} ({e_star:?}, {set:?})"
                );
                assert!(!physical::readout(set, &push).contains("NaN"), "at {z:?}");
                sim.plane = p;
                for (a, b) in upper() {
                    if let Some(Ok(edit)) = steer(&d, set, &sim, a, b) {
                        let (q1, q2) = basis_of(&edit);
                        assert!(
                            q1.iter().chain(&q2).all(|x| x.is_finite()),
                            "a non-finite steer at {z:?} ({e_star:?}, {set:?}, {a}, {b})"
                        );
                    }
                }
            }
        }
    }
    rejects("a NaN steer", || {
        assert!([f64::NAN].iter().all(|x| x.is_finite()))
    });
}

// --- REQ-GUI-200 --------------------------------------------------------------------------------------------------------

fn sim_of_mock() -> SimConfig {
    App::new(MockSide::new(MockEngine::frozen(), None), FORMAT)
        .snapshot()
        .sim
        .clone()
}

fn basis_of(edit: &SetField) -> (Z, Z) {
    match &edit.edit {
        Edit::Sim(SimField::Basis { q1, q2 }) => (*q1, *q2),
        other => panic!("not one basis edit: {other:?}"),
    }
}

/// Whether `(q1, q2)` spans the same plane as `(a, b)`, by the largest projection residue.
fn same_plane(q1: &Z, q2: &Z, a: &Z, b: &Z) -> f64 {
    let (e1, e2) = orthonormal(a, b);
    let residue = |v: &Z| {
        let n = nrm(v);
        let (x, y) = (dot(v, &e1), dot(v, &e2));
        let w: Z = std::array::from_fn(|i| v[i] / n - (x * e1[i] + y * e2[i]) / n);
        nrm(&w)
    };
    residue(q1).max(residue(q2))
}

#[test]
fn qa_mock_compass_steer_physical_is_js_preimage_on_a_full_rank_fixture() {
    let mut rng = Rng(0x200);
    let mut sim = sim_of_mock();
    for set in [Set::Jacobi, Set::Shape] {
        for _ in 0..10 {
            let want_j = rng.matrix();
            let d = Decoder::Linear {
                a: leak(unscaled(&want_j, set)),
                shell: false,
            };
            sim.plane = plane([0.0; 8], rng.z(1.0), rng.z(1.0));
            let (s1, s2) = (nrm(&sim.plane.q1), nrm(&sim.plane.q2));
            for (a, b) in upper() {
                let edit = steer(&d, set, &sim, a, b)
                    .expect("a cell")
                    .expect("accepted");
                let (q1, q2) = basis_of(&edit);
                // Gram–Schmidt, each axis at the length it had.
                assert!(dot(&q1, &q2).abs() <= 1e-10 * s1 * s2, "not orthogonal");
                assert!((nrm(&q1) - s1).abs() <= 1e-10 && (nrm(&q2) - s2).abs() <= 1e-10);
                // The preimage, by a solve.
                let mut ei = [0.0; 8];
                ei[a] = 1.0;
                let mut ej = [0.0; 8];
                ej[b] = 1.0;
                let (x, y) = (solve(&want_j, &ei), solve(&want_j, &ej));
                assert!(same_plane(&q1, &q2, &x, &y) <= 1e-7, "not J⁻¹'s plane");
                // Lit at ±1, nothing else.
                let (g, _) = bivector(&apply(&want_j, &q1), &apply(&want_j, &q2));
                assert!(
                    (g[a][b].abs() - 1.0).abs() <= 1e-7,
                    "({a}, {b}): {}",
                    g[a][b]
                );
                // Control: without Gram–Schmidt the pair is not orthogonal.
                rejects("the raw preimages as the pair", || {
                    assert!(dot(&x, &y).abs() <= 1e-10 * nrm(&x) * nrm(&y))
                });
            }
        }
    }
}

/// A rank-7 `J = (I − n nᵀ) B` with `n` the given unit normal.
fn with_normal(n: &Z, b: &Matrix) -> Matrix {
    let along: Z = std::array::from_fn(|c| (0..8).map(|r| n[r] * b[r][c]).sum());
    std::array::from_fn(|r| std::array::from_fn(|c| b[r][c] - n[r] * along[c]))
}

#[test]
fn qa_mock_compass_steer_physical_the_refusal_is_the_projector_and_scale_free() {
    let mut rng = Rng(0x346);
    let mut sim = sim_of_mock();
    sim.plane = plane([0.0; 8], rng.z(1.0), rng.z(1.0));
    let (mut refused, mut accepted) = (0, 0);
    for round in 0..40 {
        // Normals of every lean: spread out, and nearly in a few coordinates.
        let mut n = rng.z(1.0);
        if round % 2 == 0 {
            let (a, b) = (round % 8, (round / 2 + 1) % 8);
            n = n.map(|x| 0.02 * x);
            n[a] += 1.0;
            n[b] += 0.9;
        }
        let k = nrm(&n);
        let n = n.map(|x| x / k);
        let b = rng.matrix();
        for scale in [1e-5, 1.0, 1e5] {
            let j = with_normal(&n, &b).map(|row| row.map(|x| x * scale));
            let d = Decoder::Linear {
                a: leak(unscaled(&j, Set::Jacobi)),
                shell: true,
            };
            for (i, jj) in upper() {
                let want = (1.0 - n[i] * n[i] - n[jj] * n[jj]).max(0.0).sqrt();
                // Hold back from the tolerance itself, where rounding may fall either way.
                if (want - physical::TOLERANCE).abs() < 1e-6 {
                    continue;
                }
                let got = steer(&d, Set::Jacobi, &sim, i, jj).expect("a cell");
                if want < physical::TOLERANCE {
                    assert_eq!(got, Err(Refusal::EnergyShell), "({i}, {jj}) reads {want}");
                    refused += 1;
                } else {
                    let edit = got.unwrap_or_else(|r| panic!("({i}, {jj}) reads {want}: {r:?}"));
                    let (q1, q2) = basis_of(&edit);
                    let (e1, e2) = orthonormal(&q1, &q2);
                    let (g, _) = bivector(&apply(&j, &e1), &apply(&j, &e2));
                    // The cell reads the light the steer can reach (render_gui_spec, REQ-GUI-346's record).
                    assert!(
                        (g[i][jj].abs() - want).abs() <= 1e-6,
                        "({i}, {jj}): {} against {want}",
                        g[i][jj]
                    );
                    accepted += 1;
                }
            }
        }
    }
    assert!(
        refused > 10 && accepted > 100,
        "{refused} refused, {accepted} accepted"
    );
    // Control: the (a1) test on [J⁺e_i, J⁺e_j] scales with J, so its verdict flips with the scale.
    let inv = |scale: f64| {
        physical::Inverse::of(
            &std::array::from_fn(|r| std::array::from_fn(|c| if r == c { scale } else { 0.0 })),
            physical::CUT,
        )
    };
    let a1 = |s: f64| physical::smaller_singular(&inv(s).preimage(0), &inv(s).preimage(1));
    assert!((a1(1e5) - 1e-5).abs() < 1e-12 && (a1(1e-5) - 1e5).abs() < 1e-3);
    assert_eq!(
        inv(1e5).smaller(0, 1),
        inv(1e-5).smaller(0, 1),
        "(a2) is scale-free"
    );
    rejects("(a1)'s verdict", || {
        assert_eq!(physical::refuses(a1(1e5)), physical::refuses(a1(1e-5)))
    });
}

/// `H` of the Jacobi coordinates, `(α, β, p_ρ, p_λ, m₀, m₁)`, chart_reference §0.2 and §0.3.
fn hamiltonian(x: &Z) -> f64 {
    let (m0, m1) = (x[6], x[7]);
    let m2 = 1.0 - m0 - m1;
    let m01 = m0 + m1;
    let (mr, ml) = (m0 * m1 / m01, m2 * m01);
    let t = (x[2] * x[2] + x[3] * x[3]) / (2.0 * mr) + (x[4] * x[4] + x[5] * x[5]) / (2.0 * ml);
    let rho = [x[0].cos() / mr.sqrt(), 0.0];
    let l = [
        x[0].sin() * x[1].cos() / ml.sqrt(),
        x[0].sin() * x[1].sin() / ml.sqrt(),
    ];
    let (w0, w1) = (m0 / m01, m1 / m01);
    let r02 = (l[0] + w1 * rho[0]).hypot(l[1]);
    let r12 = (l[0] - w0 * rho[0]).hypot(l[1]);
    t - m0 * m1 / rho[0].abs() - m0 * m2 / r02 - m1 * m2 / r12
}

/// The shell's unit normal in the scaled Jacobi coordinates: `H`'s gradient, each component times its range.
fn shell_normal(s: &State) -> Z {
    let x = jacobi_of(s);
    let ranges = jacobi_ranges(q_max());
    let g: Z = std::array::from_fn(|k| {
        let h = 1e-6 * x[k].abs().max(1e-3);
        let (mut up, mut down) = (x, x);
        up[k] += h;
        down[k] -= h;
        (hamiltonian(&up) - hamiltonian(&down)) / (up[k] - down[k]) * ranges[k]
    });
    let n = nrm(&g);
    g.map(|v| v / n)
}

#[test]
fn qa_mock_compass_steer_physical_on_the_shell_refuses_what_h_predicts() {
    let links = Links::defaults();
    let d = real(Some(0.0));
    let mut sim = sim_of_mock();
    let mut rng = Rng(0x345);
    let (mut refused, mut accepted) = (0, 0);
    for _ in 0..200 {
        let z = rng.z(3.0);
        let s = decoded::state(&z, &links, Some(0.0));
        assert!(
            hamiltonian(&jacobi_of(&s)).abs()
                <= 1e-9 * (1.0 + hamiltonian(&jacobi_of(&s)).abs()) + 1e-7,
            "the state is on H = 0: {}",
            hamiltonian(&jacobi_of(&s))
        );
        let n = shell_normal(&s);
        sim.plane = plane(z, rng.z(1.0), rng.z(1.0));
        let j = own_jacobi_j(Some(0.0), &z);
        for (a, b) in upper() {
            let want = (1.0 - n[a] * n[a] - n[b] * n[b]).max(0.0).sqrt();
            if (want - physical::TOLERANCE).abs() < 1e-3 {
                continue;
            }
            let got = steer(&d, Set::Jacobi, &sim, a, b).expect("a cell");
            if want < physical::TOLERANCE {
                assert_eq!(
                    got,
                    Err(Refusal::EnergyShell),
                    "({a}, {b}) reads {want} at {z:?}"
                );
                refused += 1;
            } else {
                let edit =
                    got.unwrap_or_else(|r| panic!("({a}, {b}) reads {want} at {z:?}: {r:?}"));
                let (q1, q2) = basis_of(&edit);
                let (e1, e2) = orthonormal(&q1, &q2);
                let (g, _) = bivector(&apply(&j, &e1), &apply(&j, &e2));
                assert!(
                    (g[a][b].abs() - want).abs() <= 1e-3,
                    "lit {} against {want}",
                    g[a][b]
                );
                accepted += 1;
            }
        }
    }
    assert!(
        refused >= 1 && accepted > 1000,
        "{refused} refused, {accepted} accepted"
    );
    // Control: a normal in another coordinate predicts other verdicts.
    let z = Rng(0x345).z(3.0);
    let n = shell_normal(&decoded::state(&z, &links, Some(0.0)));
    rejects("the normal unscaled to e₀", || {
        let mut other = [0.0; 8];
        other[0] = 1.0;
        assert!((0..8).all(|k| (n[k].abs() - other[k]).abs() <= 1e-3))
    });
}

#[test]
fn qa_mock_compass_steer_physical_on_the_decoder_lights_its_cell() {
    let links = Links::defaults();
    let mut sim = sim_of_mock();
    let mut rng = Rng(0x201);
    for _ in 0..30 {
        let z = rng.z(2.5);
        for set in [Set::Jacobi, Set::Shape] {
            if set == Set::Shape && sin_phi(None, &z) < 0.1 {
                continue;
            }
            sim.plane = plane(z, rng.z(1.0), rng.z(1.0));
            let j = latent_jacobian(&real(None), set, &links, &z);
            for (a, b) in upper() {
                let edit = steer(&real(None), set, &sim, a, b)
                    .expect("a cell")
                    .unwrap_or_else(|r| panic!("{set:?} ({a}, {b}) refused at {z:?}: {r:?}"));
                let (q1, q2) = basis_of(&edit);
                assert_eq!(q1.iter().chain(&q2).filter(|x| !x.is_finite()).count(), 0);
                let (e1, e2) = orthonormal(&q1, &q2);
                let (g, _) = bivector(&apply(&j, &e1), &apply(&j, &e2));
                // REQ-GUI-346's tolerance is the task's bound for this.
                assert!(
                    (g[a][b].abs() - 1.0).abs() <= physical::TOLERANCE,
                    "{}",
                    g[a][b]
                );
            }
        }
    }
    // Control: a steer to the wrong cell does not light this one.
    let z = rng.z(2.5);
    sim.plane = plane(z, rng.z(1.0), rng.z(1.0));
    let j = latent_jacobian(&real(None), Set::Jacobi, &links, &z);
    let (q1, q2) = basis_of(
        &steer(&real(None), Set::Jacobi, &sim, 0, 1)
            .unwrap()
            .unwrap(),
    );
    let (g, _) = bivector(&apply(&j, &q1), &apply(&j, &q2));
    rejects("another cell", || {
        assert!((g[2][3].abs() - 1.0).abs() <= physical::TOLERANCE)
    });
}

// --- the running app ----------------------------------------------------------------------------------------------------

struct Rig {
    app: App<MockSide>,
    h: Headless,
}

fn rig(p: &Plane, view: CompassView) -> Rig {
    let mut r = Rig {
        app: App::new(MockSide::new(MockEngine::frozen(), None), FORMAT),
        h: Headless::new(SIZE, PIXELS_PER_POINT),
    };
    r.idle(2);
    r.app.set_field(z0_edit(p.z0));
    r.app.set_field(basis_edit((p.q1, p.q2)));
    r.app.manifold.compass.view = view;
    r.idle(3);
    r
}

impl Rig {
    fn idle(&mut self, n: usize) {
        for _ in 0..n {
            let _ = self.h.frame(&mut self.app, Vec::new());
        }
    }
    fn layout(&self) -> Layout {
        Layout::new(self.h.screen(), PIXELS_PER_POINT)
    }
    fn cell(&self, i: usize, j: usize) -> Pos2 {
        GridPlace::fit(compass::square(self.layout().compass))
            .cell_rect(i, j)
            .center()
    }
    fn click(&mut self, at: Pos2) {
        for events in capture::click(at) {
            let _ = self.h.frame(&mut self.app, events);
        }
        self.idle(2);
    }
    fn plane(&self) -> Plane {
        self.app.snapshot().sim.plane.clone()
    }
    fn undo(&mut self) {
        for pressed in [true, false] {
            let _ = self.h.frame(
                &mut self.app,
                vec![Event::Key {
                    key: egui::Key::Z,
                    physical_key: None,
                    pressed,
                    repeat: false,
                    modifiers: egui::Modifiers::COMMAND,
                }],
            );
        }
        self.idle(2);
    }
    /// The SetFields since `from`, less the hover's no-history ones.
    fn edits(&self, from: usize) -> Vec<String> {
        self.app.console()[from..]
            .iter()
            .filter(|e| {
                e.source == Source::Contract
                    && e.message.starts_with("SetField")
                    && !e.message.starts_with("SetField Hover.at ")
            })
            .map(|e| e.message.clone())
            .collect()
    }
    fn texts(&mut self) -> Vec<String> {
        let out = self.h.frame(&mut self.app, Vec::new());
        self.h.names(&out).iter().map(|n| n.name.clone()).collect()
    }
}

fn start() -> Plane {
    plane(
        [0.3, -0.2, 0.5, 0.1, -0.4, 0.2, 0.3, -0.1],
        [0.6, 0.1, -0.2, 0.1, 0.05, 0.0, 0.1, -0.1],
        [0.0, 0.5, 0.1, -0.2, 0.1, 0.1, -0.05, 0.1],
    )
}

#[test]
fn qa_mock_compass_steer_physical_a_click_is_one_setfield_undone_by_one_ctrl_z() {
    let p = start();
    for (view, set) in [
        (CompassView::Jacobi, Set::Jacobi),
        (CompassView::Shape, Set::Shape),
    ] {
        for (a, b) in [(0, 1), (2, 5), (4, 7)] {
            let mut r = rig(&p, view);
            let before = r.plane();
            let frame_before = r.app.snapshot().sim.slice.frame;
            let depth = r.app.snapshot().history.undo_depth;
            let from = r.app.console().len();
            r.click(r.cell(a, b));
            assert_eq!(r.edits(from).len(), 1, "{:?}", r.edits(from));
            // One SetField carrying the basis: on the basis alone, or, where the plane took the slice normal or a target,
            // on the slice frame too, in the same edit (REQ-GUI-208, R-459 G4).
            let want_edit = if r.app.snapshot().sim.slice.frame == frame_before {
                "SetField Plane.q1 Plane.q2"
            } else {
                "SetField Slice.frame Plane.q1 Plane.q2"
            };
            assert!(
                r.edits(from)[0].starts_with(want_edit),
                "{:?}",
                r.edits(from)
            );
            assert_eq!(r.app.snapshot().history.undo_depth, depth + 1);
            let after = r.plane();
            assert_eq!(after.z0, before.z0, "z₀ kept");
            let want = steer(&Decoder::default(), set, &r.app.snapshot().sim, a, b)
                .unwrap()
                .unwrap();
            let (wq1, wq2) = basis_of(&want);
            // Steered from the steered plane the result is the same plane, J being taken at the same z₀.
            assert!(same_plane(&after.q1, &after.q2, &wq1, &wq2) <= 1e-9);
            // The Manifold view follows, as after Apply: the same one basis edit, and on a cell whose preimage lies along
            // two axes (J's α and β rows are z_α's and z_β's alone) its sliders name that pair, untilted.
            if set == Set::Jacobi && (a, b) == (0, 1) {
                let v = View::read(&r.app.snapshot().sim);
                assert_eq!((v.a, v.b), (0, 1), "{v:?}");
                assert!(v.tau1.abs() <= 1e-9 && v.tau2.abs() <= 1e-9, "{v:?}");
                let (vq1, vq2) = v.basis();
                assert!(
                    same_plane(&vq1, &vq2, &after.q1, &after.q2) <= 1e-9,
                    "{v:?}"
                );
            }
            r.undo();
            assert_eq!(r.plane(), before, "one Ctrl+Z restores it");
            assert_eq!(
                r.app.snapshot().sim.slice.frame,
                frame_before,
                "the frame too"
            );
            assert_eq!(r.app.snapshot().history.undo_depth, depth);
            // Control: the steer did move the plane, so the restore can fail.
            rejects("a steer that moved nothing", || assert_eq!(after, before));
        }
    }
}

#[test]
fn qa_mock_compass_steer_physical_a_refused_click_logs_and_edits_nothing() {
    // A shell chart whose normal is e₀ alone: every cell in row 0 needs it.
    let n: Z = std::array::from_fn(|k| if k == 0 { 1.0 } else { 0.0 });
    let b: Matrix = std::array::from_fn(|r| std::array::from_fn(|c| f64::from(r == c)));
    let d = Decoder::Linear {
        a: leak(unscaled(&with_normal(&n, &b), Set::Jacobi)),
        shell: true,
    };
    let mut r = rig(&start(), CompassView::Jacobi);
    r.app.manifold.compass.decoder = d;
    r.idle(1);
    let before = r.plane();
    let depth = r.app.snapshot().history.undo_depth;
    let warnings = r.app.counts().warnings;
    let from = r.app.console().len();
    r.click(r.cell(0, 3));
    assert!(r.edits(from).is_empty(), "no SetField: {:?}", r.edits(from));
    assert_eq!(r.plane(), before);
    assert_eq!(r.app.snapshot().history.undo_depth, depth);
    assert_eq!(r.app.counts().warnings, warnings + 1);
    assert!(
        r.app.console()[from..]
            .iter()
            .any(|e| e.message.contains("normal to the energy shell")),
        "logged"
    );
    // A cell clear of the normal is steered on the same chart: the refusal is the cell's, not the chart's.
    let from = r.app.console().len();
    r.click(r.cell(2, 3));
    assert_eq!(r.edits(from).len(), 1);
    rejects("a cell refused and steered at once", || {
        assert_eq!(r.edits(from).len(), 0)
    });
}

#[test]
fn qa_compass_physical_views_the_pointer_leaving_the_figure_for_the_compass_returns_to_z0() {
    let p = start();
    let mut r = rig(&p, CompassView::Jacobi);
    let at_z0 = compass::pushed(&r.app.snapshot().sim, &r.app.manifold.compass, Set::Jacobi);
    let figure = r.layout().figure;
    let _ = r.h.frame(
        &mut r.app,
        vec![Event::PointerMoved(figure.center() + vec2(120.0, -60.0))],
    );
    r.idle(2);
    let hover = r.app.manifold.compass.hover.expect("over the figure");
    let on_figure = compass::pushed(&r.app.snapshot().sim, &r.app.manifold.compass, Set::Jacobi);
    let z = physical::latent_point(&p, Some(hover));
    // The pixel's latent point, R-550 J2's formula written out.
    for (k, x) in z.iter().enumerate() {
        let want = p.z0[k] + (2.0 * hover[0] - 1.0) * p.q1[k] + (2.0 * hover[1] - 1.0) * p.q2[k];
        assert!((x - want).abs() <= 1e-14);
    }
    assert_ne!(on_figure, at_z0, "the hovered J differs here");
    // Straight onto a compass cell, with no PointerGone between: J is z₀'s again, and the steer is from z₀.
    let at = r.cell(1, 4);
    let _ = r.h.frame(&mut r.app, vec![Event::PointerMoved(at)]);
    r.idle(2);
    assert_eq!(r.app.manifold.compass.hover, None, "off the figure");
    let back = compass::pushed(&r.app.snapshot().sim, &r.app.manifold.compass, Set::Jacobi);
    assert_eq!(back, at_z0);
    let want = steer(
        &Decoder::default(),
        Set::Jacobi,
        &r.app.snapshot().sim,
        1,
        4,
    )
    .unwrap()
    .unwrap();
    let (wq1, wq2) = basis_of(&want);
    r.click(at);
    let after = r.plane();
    assert!(
        same_plane(&after.q1, &after.q2, &wq1, &wq2) <= 1e-12,
        "the steer's J is z₀'s"
    );
    // Control: the figure's J is a different picture from z₀'s.
    rejects("z₀'s grid under the pointer", || {
        assert_eq!(on_figure, at_z0)
    });
}

#[test]
fn qa_compass_physical_views_the_readout_and_help_are_said() {
    let mut r = rig(&start(), CompassView::Shape);
    let push = compass::pushed(&r.app.snapshot().sim, &r.app.manifold.compass, Set::Shape);
    let texts = r.texts().join(" | ");
    // The dominant cell by this file's own search.
    let mut best = (0, 1);
    for (a, b) in upper() {
        if push.grid[a][b].abs() > push.grid[best.0][best.1].abs() {
            best = (a, b);
        }
    }
    let names = Set::Shape.names();
    assert!(
        texts.contains(&format!("{} × {}", names[best.0], names[best.1])),
        "the dominant pair: {texts}"
    );
    assert!(
        texts.contains(&format!("stretch {:.3}", push.stretch)),
        "{texts}"
    );
    // The help names the metric, and why the view changes where Plücker does not (REQ-GUI-199).
    for needed in [
        "reference range",
        "moves",
        "Plücker",
        "z₀",
        "nonlinear chart",
    ] {
        assert!(physical::HELP.contains(needed), "{needed} in the help");
    }
    assert!(
        texts.contains("reference range"),
        "the help is on screen: {texts}"
    );
    rejects("a word the help lacks", || {
        assert!(physical::HELP.contains("Euclidean"))
    });
}

#[test]
fn qa_compass_physical_views_a_slice_edit_moves_j_and_not_plucker() {
    let p = start();
    let mut r = rig(&p, CompassView::Jacobi);
    let before = compass::pushed(&r.app.snapshot().sim, &r.app.manifold.compass, Set::Jacobi);
    let plucker = gui::explore::compass::views::plucker(&r.app.snapshot().sim.plane);
    let mut z = p.z0;
    z[0] += 0.8;
    z[3] -= 0.6;
    r.app.set_field(z0_edit(z));
    r.idle(2);
    let after = compass::pushed(&r.app.snapshot().sim, &r.app.manifold.compass, Set::Jacobi);
    assert_eq!(
        gui::explore::compass::views::plucker(&r.app.snapshot().sim.plane),
        plucker
    );
    let moved = upper()
        .into_iter()
        .map(|(a, b)| (after.grid[a][b] - before.grid[a][b]).abs())
        .fold(0.0, f64::max);
    assert!(moved > 1e-3, "J depends on z₀: {moved}");
    rejects("a grid that did not move", || assert_eq!(after, before));
}
