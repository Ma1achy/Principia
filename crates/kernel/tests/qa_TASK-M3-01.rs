//! QA's tests for TASK-M3-01, written from the requirements it closes, not from the implementation:
//! - REQ-INT-004 (R-265): the kernel's `Real` is implemented for f32 and f64 only, no double-double, and one generic
//!   golden-IC check (the M2 decoder's `z = 0`, and a second, asymmetric decode) runs at each.
//! - REQ-INT-036 (dd_integrator §3.1): `aᵢ = Σ_{j≠i} mⱼ(rⱼ − rᵢ)/‖rⱼ − rᵢ‖³`, `G = 1`, on a hand-computed 6-8-10
//!   right-triangle state (exact rationals), under every permutation of the bodies, and against a direct per-body sum
//!   over fuzzed states; `Σ mᵢaᵢ = 0`.
//! - R-22's pair map: pair `k` is the side opposite body `k`, pair 0 = (1, 2), pair 1 = (2, 0), pair 2 = (0, 1), so
//!   pair `k`'s distance is that of the two bodies other than `k`; `r_min` is the least of the three.
//! - REQ-PERF-001 (integrator_contract Part 1, "COM projection and invariant monitoring"): `E`'s potential and `r_min`
//!   read the force's pairwise distances. An instrumented `Real` counts square roots: building the geometry takes three,
//!   and the force, `E` (kinetic and potential) and `r_min` read from it take none.
//! - E and `L_z` (integrator_contract Part 1's formulas, `G = 1`) on the same hand state and against direct sums.
//!
//! **Tolerance, derived not picked.** "Matches to f64 rounding" (REQ-INT-036's verify) is bounded by a first-order
//! rounding count of the force's own evaluation at unit roundoff `u = eps/2`: a separation (1 rounding), its `d²` (2,
//! doubling the separation's), `‖·‖` (1, halving `d²`'s), `‖·‖³ = d²·‖·‖` (1), the quotient (1), the mass product (1)
//! and the pair sum (1), at most `12u = 6 eps` relative to the sum of the magnitudes of the terms. [`BOUND_EPS`] = 6
//! is used for every quantity; the energy's and `L_z`'s counts are smaller (≤ 8u and 4u). The fuzzed checks compare
//! against an f64 direct evaluation of the formula on the same (exactly widened) inputs, so at f32 the reference's own
//! error is `2⁻²⁹` of f32's `eps` and does not enter.

mod decode_support;

use std::cell::Cell;
use std::fs;
use std::ops::{Add, Div, Mul, Neg, Sub};
use std::path::{Path, PathBuf};

use decode_support::P;
use kernel::decode;
use kernel::generated::links::Defaults;
use kernel::physics::energy::{angular_momentum, energy, kinetic, potential};
use kernel::physics::force::force;
use kernel::physics::PairGeom;
use kernel::real::RealOps;
use kernel::Real;
use proptest::prelude::*;
use validation::{negative_control, prop};

/// The derived bound, in `eps` relative to the scale of the terms (the module's note).
const BOUND_EPS: f64 = 6.0;

type V3 = [[f64; 2]; 3];

/// `got` is within [`BOUND_EPS`] `eps` of `want`, relative to `scale`; of a zero scale (every term exactly zero, as
/// for a state at rest), exactly `want`.
fn close(what: &str, got: f64, want: f64, scale: f64, eps: f64) {
    if scale == 0.0 {
        assert_eq!(
            got, want,
            "{what}: every term is zero, so the value must be exactly {want:e}"
        );
        return;
    }
    let err = (got - want).abs() / (scale * eps);
    assert!(
        err <= BOUND_EPS,
        "{what}: got {got:e}, want {want:e}, {err:.3} eps of scale {scale:e} (bound {BOUND_EPS})"
    );
}

// ── The reference: the formula written directly, per body, in f64 ───────────────────────────────────────────────

/// `aᵢ = Σ_{j≠i} mⱼ(rⱼ − rᵢ)/‖rⱼ − rᵢ‖³` by a direct double sum, and the scale `Σ_{j≠i} mⱼ/‖rⱼ − rᵢ‖²` of each body's
/// terms.
fn reference_force(m: [f64; 3], r: V3) -> (V3, [f64; 3]) {
    let mut a = [[0.0; 2]; 3];
    let mut scale = [0.0; 3];
    for i in 0..3 {
        for j in 0..3 {
            if j == i {
                continue;
            }
            let d = [r[j][0] - r[i][0], r[j][1] - r[i][1]];
            let n = d[0].hypot(d[1]);
            a[i][0] += m[j] * d[0] / (n * n * n);
            a[i][1] += m[j] * d[1] / (n * n * n);
            scale[i] += m[j] / (n * n);
        }
    }
    (a, scale)
}

/// The distance between bodies `i` and `j`.
fn dist(r: V3, i: usize, j: usize) -> f64 {
    (r[j][0] - r[i][0]).hypot(r[j][1] - r[i][1])
}

/// `E = Σ‖pᵢ‖²/2mᵢ − Σ_{i<j} mᵢmⱼ/‖rᵢ − rⱼ‖`, `G = 1`, with the kinetic and potential parts and the scale `K + |V|`.
fn reference_energy(m: [f64; 3], r: V3, p: V3) -> (f64, f64, f64) {
    let k: f64 = (0..3)
        .map(|i| (p[i][0] * p[i][0] + p[i][1] * p[i][1]) / (2.0 * m[i]))
        .sum();
    let mut v = 0.0;
    for i in 0..3 {
        for j in i + 1..3 {
            v -= m[i] * m[j] / dist(r, i, j);
        }
    }
    (k, v, k + v.abs())
}

/// `L_z = Σ (xᵢ p_{y,i} − yᵢ p_{x,i})` and its scale.
fn reference_lz(r: V3, p: V3) -> (f64, f64) {
    let t: Vec<f64> = (0..3)
        .map(|i| r[i][0] * p[i][1] - r[i][1] * p[i][0])
        .collect();
    let s = (0..3)
        .map(|i| (r[i][0] * p[i][1]).abs() + (r[i][1] * p[i][0]).abs())
        .sum();
    (t.iter().sum(), s)
}

// ── REQ-INT-036: the hand-computed state ─────────────────────────────────────────────────────────────────────────

/// A 6-8-10 right triangle, the right angle at body 2, bodies in no symmetric arrangement, unequal masses.
const HM: [f64; 3] = [2.0, 3.0, 7.0];
const HR: V3 = [[0.0, 0.0], [6.0, 8.0], [6.0, 0.0]];
/// Momenta with `Σpᵢ = 0`.
const HP: V3 = [[1.0, 2.0], [-3.0, 0.5], [2.0, -2.5]];

/// The force on [`HR`] by hand: `|r₁ − r₀| = 10`, `|r₂ − r₀| = 6`, `|r₂ − r₁| = 8`;
/// `a₀ = 3(6, 8)/1000 + 7(6, 0)/216 = (0.018 + 7/36, 0.024)`,
/// `a₁ = 2(−6, −8)/1000 + 7(0, −8)/512 = (−0.012, −0.016 − 7/64)`,
/// `a₂ = 2(−6, 0)/216 + 3(0, 8)/512 = (−1/18, 3/64)`.
fn hand_force() -> V3 {
    [
        [0.018 + 7.0 / 36.0, 0.024],
        [-0.012, -0.016 - 7.0 / 64.0],
        [-1.0 / 18.0, 3.0 / 64.0],
    ]
}

/// `a` matches `want` body by body, component by component, to the derived bound at f64.
fn check_force_matches(a: V3, want: V3, scale: [f64; 3]) {
    for i in 0..3 {
        for c in 0..2 {
            close(
                &format!("force on body {i}, component {c}"),
                a[i][c],
                want[i][c],
                scale[i],
                f64::EPSILON,
            );
        }
    }
}

#[test]
fn qa_force_hand_state() {
    let a = force(&HM, &PairGeom::new(&HR));
    check_force_matches(a, hand_force(), reference_force(HM, HR).1);
}

negative_control!(
    qa_force_hand_state,
    "the force with the attraction reversed, mⱼ(rᵢ − rⱼ)/‖·‖³, must not match the hand values",
    expected = "force on body",
    check_force_matches(
        hand_force().map(|v| v.map(|x| -x)),
        hand_force(),
        reference_force(HM, HR).1
    )
);

/// The same check, which a second control holds to the inverse-square law (`‖·‖³` under the separation).
#[test]
fn qa_force_hand_state_cube() {
    let a = force(&HM, &PairGeom::new(&HR));
    check_force_matches(a, hand_force(), reference_force(HM, HR).1);
}

negative_control!(
    qa_force_hand_state_cube,
    "a force with ‖·‖² in place of ‖·‖³ must not match the hand values",
    expected = "force on body 0",
    {
        let mut a = [[0.0; 2]; 3];
        for i in 0..3 {
            for j in 0..3 {
                if i != j {
                    let n = dist(HR, i, j);
                    a[i][0] += HM[j] * (HR[j][0] - HR[i][0]) / (n * n);
                    a[i][1] += HM[j] * (HR[j][1] - HR[i][1]) / (n * n);
                }
            }
        }
        check_force_matches(a, hand_force(), reference_force(HM, HR).1)
    }
);

/// `Σ mᵢaᵢ = 0`, relative to the scale `Σᵢ mᵢ Σ_{j≠i} mⱼ/‖rⱼ − rᵢ‖²`.
fn check_balance(m: [f64; 3], a: V3, scale: [f64; 3], eps: f64) {
    let s: f64 = (0..3).map(|i| m[i] * scale[i]).sum();
    let total = m.iter().zip(a).fold([0.0; 2], |t, (mi, ai)| {
        [t[0] + mi * ai[0], t[1] + mi * ai[1]]
    });
    for (c, t) in total.into_iter().enumerate() {
        close(&format!("Σ mᵢaᵢ, component {c}"), t, 0.0, s, eps);
    }
}

#[test]
fn qa_force_hand_state_total_zero() {
    let a = force(&HM, &PairGeom::new(&HR));
    check_balance(HM, a, reference_force(HM, HR).1, f64::EPSILON);
}

negative_control!(
    qa_force_hand_state_total_zero,
    "a force missing one pair's reaction on one body does not sum to zero",
    expected = "Σ mᵢaᵢ",
    {
        let mut a = hand_force();
        // Drop body 2's attraction to body 1, keeping body 1's to body 2.
        a[2] = [-1.0 / 18.0, 0.0];
        check_balance(HM, a, reference_force(HM, HR).1, f64::EPSILON)
    }
);

/// The six permutations of three bodies.
const PERMS: [[usize; 3]; 6] = [
    [0, 1, 2],
    [0, 2, 1],
    [1, 0, 2],
    [1, 2, 0],
    [2, 0, 1],
    [2, 1, 0],
];

/// Relabelling the bodies by `perm` (new body `k` is old body `perm[k]`) relabels the force the same way.
fn check_permuted(f: impl Fn(&[f64; 3], &V3) -> V3) {
    let want = hand_force();
    let scale = reference_force(HM, HR).1;
    for perm in PERMS {
        let m = perm.map(|k| HM[k]);
        let r = perm.map(|k| HR[k]);
        let a = f(&m, &r);
        check_force_matches(a, perm.map(|k| want[k]), perm.map(|k| scale[k]));
    }
}

#[test]
fn qa_force_hand_state_every_labelling() {
    check_permuted(|m, r| force(m, &PairGeom::new(r)));
}

negative_control!(
    qa_force_hand_state_every_labelling,
    "a force whose pair map pairs body k with body k + 1 only, missing a pair, fails some labelling",
    expected = "force on body",
    check_permuted(|m, r| {
        let mut a = [[0.0; 2]; 3];
        for i in 0..3 {
            let j = (i + 1) % 3;
            let n = dist(*r, i, j);
            a[i][0] += m[j] * (r[j][0] - r[i][0]) / (n * n * n);
            a[i][1] += m[j] * (r[j][1] - r[i][1]) / (n * n * n);
        }
        a
    })
);

// ── R-22's pair map and r_min ────────────────────────────────────────────────────────────────────────────────────

/// Pair `k`'s two bodies (R-22: the side opposite body `k`).
const R22: [(usize, usize); 3] = [(1, 2), (2, 0), (0, 1)];

/// `g`'s pair `k` is the side opposite body `k` of the positions `r`: its separation is `±(rⱼ − rᵢ)` of R-22's bodies,
/// its `d²`, `‖·‖` and `‖·‖³` theirs, and `r_min` the least distance of the three.
fn check_r22(g: &PairGeom<f64>, r: V3, map: [(usize, usize); 3]) {
    for (k, (i, j)) in map.into_iter().enumerate() {
        let d = [r[j][0] - r[i][0], r[j][1] - r[i][1]];
        let s = g.sep()[k];
        assert!(
            s == d || s == [-d[0], -d[1]],
            "pair {k}'s separation {s:?} is not that of bodies {i} and {j} (R-22)"
        );
        let n = dist(r, i, j);
        assert_eq!(
            g.norm()[k],
            n,
            "pair {k}'s distance is not bodies {i}, {j}'s (R-22)"
        );
        assert_eq!(
            g.d2()[k],
            n * n,
            "pair {k}'s d² is not bodies {i}, {j}'s (R-22)"
        );
        assert_eq!(
            g.cube()[k],
            n * n * n,
            "pair {k}'s ‖·‖³ is not bodies {i}, {j}'s (R-22)"
        );
    }
    let least = dist(r, 0, 1).min(dist(r, 1, 2)).min(dist(r, 2, 0));
    assert_eq!(g.r_min(), least, "r_min is not the least pairwise distance");
}

#[test]
fn qa_pair_map_r22() {
    // Exact distances 8, 6 and 10, all different, so a mis-indexed pair cannot pass.
    check_r22(&PairGeom::new(&HR), HR, R22);
    // Every relabelling, so the map holds whichever pair is shortest.
    for perm in PERMS {
        let r = perm.map(|k| HR[k]);
        check_r22(&PairGeom::new(&r), r, R22);
    }
}

negative_control!(
    qa_pair_map_r22,
    "pair k as the pair (k, k + 1), not the side opposite body k, is not R-22's map",
    expected = "(R-22)",
    check_r22(&PairGeom::new(&HR), HR, [(0, 1), (1, 2), (2, 0)])
);

/// The R-22 pair id of the closest pair is the body not in it.
fn check_closest_id(g: &PairGeom<f64>, outside: usize) {
    assert_eq!(
        g.closest(),
        outside,
        "the closest pair's id is not the body outside it (R-22)"
    );
}

#[test]
fn qa_pair_map_closest_id() {
    // Bodies 0 and 2 are closest (6), so the closest pair is pair 1, opposite body 1.
    check_closest_id(&PairGeom::new(&HR), 1);
    for perm in PERMS {
        let r = perm.map(|k| HR[k]);
        // The new labels of old bodies 0 and 2; the third is the one outside the closest pair.
        let outside = (0..3).find(|&k| perm[k] == 1).expect("old body 1");
        check_closest_id(&PairGeom::new(&r), outside);
    }
}

negative_control!(
    qa_pair_map_closest_id,
    "the closest pair named by the lower of its two bodies is not its R-22 id",
    expected = "the closest pair's id is not the body outside it",
    check_closest_id(&PairGeom::new(&HR), 0)
);

// ── Energy and L_z on the hand state ─────────────────────────────────────────────────────────────────────────────

/// By hand: `K = 5/4 + 9.25/6 + 10.25/14`; `V = −(2·3/10 + 2·7/6 + 3·7/8)`; `L_z = 0 + (6·0.5 + 8·3) + (−6·2.5) = 12`.
fn hand_energy() -> (f64, f64, f64) {
    let k = 5.0 / 4.0 + 9.25 / 6.0 + 10.25 / 14.0;
    let v = -(6.0 / 10.0 + 14.0 / 6.0 + 21.0 / 8.0);
    (k, v, 12.0)
}

fn check_energy(k: f64, v: f64, e: f64, lz: f64, want: (f64, f64, f64)) {
    let (wk, wv, wl) = want;
    let scale = wk + wv.abs();
    close("K", k, wk, wk, f64::EPSILON);
    close("V", v, wv, wv.abs(), f64::EPSILON);
    close("E", e, wk + wv, scale, f64::EPSILON);
    close("L_z", lz, wl, reference_lz(HR, HP).1, f64::EPSILON);
}

#[test]
fn qa_energy_hand_state() {
    let g = PairGeom::new(&HR);
    check_energy(
        kinetic(&HM, &HP),
        potential(&HM, &g),
        energy(&HM, &HP, &g),
        angular_momentum(&HR, &HP),
        hand_energy(),
    );
}

negative_control!(
    qa_energy_hand_state,
    "a potential with the pairs' mass products mismatched to their distances must not match",
    expected = "V:",
    {
        let g = PairGeom::new(&HR);
        let n = g.norm();
        // Each pair's mass product put on the wrong distance.
        let v = -(HM[1] * HM[2] / n[1] + HM[2] * HM[0] / n[2] + HM[0] * HM[1] / n[0]);
        check_energy(
            kinetic(&HM, &HP),
            v,
            kinetic(&HM, &HP) + v,
            angular_momentum(&HR, &HP),
            hand_energy(),
        )
    }
);

// ── Fuzzed: the force, Σmᵢaᵢ, E, L_z and r_min against the formulas ──────────────────────────────────────────────

/// Masses in `(0, 1]` and positions and momenta in `[-4, 4]`, kept where no two bodies are within `1/64` of the
/// configuration's size (the force is finite; near-collision accuracy is the driver's, not this formula's).
fn states() -> impl Strategy<Value = ([f64; 3], V3, V3)> {
    let c = -4.0f64..=4.0;
    (
        [1e-3f64..=1.0, 1e-3f64..=1.0, 1e-3f64..=1.0],
        [
            [c.clone(), c.clone()],
            [c.clone(), c.clone()],
            [c.clone(), c.clone()],
        ],
        [
            [c.clone(), c.clone()],
            [c.clone(), c.clone()],
            [c.clone(), c],
        ],
    )
        .prop_filter("bodies too close", |(_, r, _)| {
            let d = [dist(*r, 0, 1), dist(*r, 1, 2), dist(*r, 2, 0)];
            let big = d[0].max(d[1]).max(d[2]);
            d.iter().all(|&x| x > big / 64.0)
        })
}

/// The physics layer at `R`, on `(m, r, p)` rounded to `R`, against the f64 formulas on the same rounded inputs.
fn check_formulas_at<R: Real + P>(m: [f64; 3], r: V3, p: V3, flip: bool) {
    let (m, r, p) = (
        m.map(R::of),
        r.map(|v| v.map(R::of)),
        p.map(|v| v.map(R::of)),
    );
    let (mw, rw, pw) = (m.map(R::f), r.map(|v| v.map(R::f)), p.map(|v| v.map(R::f)));
    let g = PairGeom::new(&r);
    let a = force(&m, &g).map(|v| v.map(R::f));
    let (mut want, scale) = reference_force(mw, rw);
    if flip {
        want = want.map(|v| v.map(|x| -x));
    }
    for i in 0..3 {
        for c in 0..2 {
            close(
                &format!("{} force on body {i}", R::NAME),
                a[i][c],
                want[i][c],
                scale[i],
                R::eps(),
            );
        }
    }
    check_balance(mw, a, scale, R::eps());
    let (k, v, s) = reference_energy(mw, rw, pw);
    close(
        &format!("{} E", R::NAME),
        energy(&m, &p, &g).f(),
        k + v,
        s,
        R::eps(),
    );
    let (lz, ls) = reference_lz(rw, pw);
    close(
        &format!("{} L_z", R::NAME),
        angular_momentum(&r, &p).f(),
        lz,
        ls,
        R::eps(),
    );
    let least = dist(rw, 0, 1).min(dist(rw, 1, 2)).min(dist(rw, 2, 0));
    close(
        &format!("{} r_min", R::NAME),
        g.r_min().f(),
        least,
        least,
        R::eps(),
    );
}

fn check_formulas(flip: bool) {
    prop::run(&states(), |(m, r, p)| {
        check_formulas_at::<f64>(m, r, p, flip);
        check_formulas_at::<f32>(m, r, p, flip);
        Ok(())
    });
}

#[test]
fn qa_formulas_fuzzed() {
    check_formulas(false);
}

negative_control!(
    qa_formulas_fuzzed,
    "the reference's sign flipped must fail the fuzzed comparison",
    expected = "force on body",
    check_formulas(true)
);

// ── REQ-INT-004: f32 and f64 only, one golden-IC check at each ───────────────────────────────────────────────────

/// One check, instantiated at each precision: the decode of `z` at `R`, run through the physics layer at `R`, against
/// the f64 formulas on the same decoded state; at `z = 0` the decoded state is the golden IC, equal masses `1/3` at the
/// corners of an equilateral triangle of side `√3` about the CoM, at rest, so also each force is attractive (toward
/// the CoM) and `K = L_z = 0`.
fn golden_check<R: Real + P>(z: [f64; 8], flip: bool) {
    let ic = decode::decode::<Defaults, R>(&decode_support::latent::<R>(z)).ic;
    let (m, r, p) = (
        ic.m.map(R::f),
        ic.r.map(|v| v.map(R::f)),
        ic.p.map(|v| v.map(R::f)),
    );
    check_formulas_at::<R>(m, r, p, flip);
    if z == [0.0; 8] {
        let g = PairGeom::new(&ic.r);
        let a = force(&ic.m, &g).map(|v| v.map(R::f));
        for i in 0..3 {
            assert!(
                a[i][0] * r[i][0] + a[i][1] * r[i][1] < 0.0,
                "{} golden force on body {i} is not attractive",
                R::NAME
            );
        }
        assert_eq!(kinetic(&ic.m, &ic.p).f(), 0.0, "{} golden K", R::NAME);
        assert_eq!(
            angular_momentum(&ic.r, &ic.p).f(),
            0.0,
            "{} golden L_z",
            R::NAME
        );
    }
}

#[test]
fn qa_golden_ic_at_each_precision() {
    for z in [[0.0; 8], [0.4, -0.7, 0.2, 0.9, -0.3, 0.5, -0.6, 0.1]] {
        golden_check::<f32>(z, false);
        golden_check::<f64>(z, false);
    }
}

negative_control!(
    qa_golden_ic_at_each_precision,
    "the golden IC's force against a reversed reference must fail at f32",
    expected = "f32 force on body",
    golden_check::<f32>([0.0; 8], true)
);

/// The type each `impl … Real for <type>` in the `.rs` files under `dir` names, from its source text.
fn real_impls(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in fs::read_dir(&d).expect("kernel source readable") {
            let path: PathBuf = entry.expect("entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|x| x == "rs") {
                out.extend(impls_in(&fs::read_to_string(&path).expect("source")));
            }
        }
    }
    out.sort();
    out
}

/// The type after each `Real for` that follows `impl`, `Real` a whole word (so `PayloadReal for` is not one).
fn impls_in(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in source.lines() {
        let line = line.trim_start();
        if !line.starts_with("impl") {
            continue;
        }
        for (at, _) in line.match_indices("Real for ") {
            let before = line[..at].chars().next_back();
            if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
                continue;
            }
            let ty: String = line[at + "Real for ".len()..]
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            out.push(ty);
        }
    }
    out
}

fn check_instantiations(found: &[String]) {
    assert_eq!(
        found,
        ["f32", "f64"],
        "the kernel's Real is not implemented for f32 and f64 only (R-265)"
    );
}

#[test]
fn qa_real_f32_f64_only() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let found = real_impls(&src);
    check_instantiations(&found);
    assert_eq!(<f32 as Real>::NAME, "f32");
    assert_eq!(<f64 as Real>::NAME, "f64");
}

negative_control!(
    qa_real_f32_f64_only,
    "a double-double impl of Real beside f32 and f64 must fail",
    expected = "not implemented for f32 and f64 only",
    {
        let mut found = impls_in(
            "impl Real for f32 {}\nimpl crate::Real for f64 {}\nimpl PayloadReal for f32 {}\nimpl Real for DoubleF64 {}\n",
        );
        found.sort();
        check_instantiations(&found)
    }
);

// ── REQ-PERF-001: one distance computation, read by the force, E and r_min ───────────────────────────────────────

thread_local! {
    static ROOTS: Cell<usize> = const { Cell::new(0) };
}

/// f64, counting square roots: a distance needs one, and nothing else in the force, E or r_min does.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
struct Sq(f64);

impl Add for Sq {
    type Output = Sq;
    fn add(self, b: Sq) -> Sq {
        Sq(self.0 + b.0)
    }
}
impl Sub for Sq {
    type Output = Sq;
    fn sub(self, b: Sq) -> Sq {
        Sq(self.0 - b.0)
    }
}
impl Mul for Sq {
    type Output = Sq;
    fn mul(self, b: Sq) -> Sq {
        Sq(self.0 * b.0)
    }
}
impl Div for Sq {
    type Output = Sq;
    fn div(self, b: Sq) -> Sq {
        Sq(self.0 / b.0)
    }
}
impl Neg for Sq {
    type Output = Sq;
    fn neg(self) -> Sq {
        Sq(-self.0)
    }
}
impl RealOps for Sq {
    fn fma(self, a: Sq, b: Sq) -> Sq {
        Sq(self.0.mul_add(a.0, b.0))
    }
    fn square_root(self) -> Sq {
        ROOTS.with(|c| c.set(c.get() + 1));
        Sq(self.0.sqrt())
    }
}
impl Real for Sq {
    const NAME: &'static str = "qa-counted";
}

fn roots() -> usize {
    ROOTS.with(Cell::get)
}

/// One force evaluation and its monitoring, as the wrapper does it: the geometry from `geom`, then the force, E and
/// r_min. Returns the roots taken building and reading, and E and r_min.
fn evaluation(geom: impl Fn(&[[Sq; 2]; 3]) -> PairGeom<Sq>) -> (usize, usize, f64, f64) {
    let m = HM.map(Sq);
    let r = HR.map(|v| v.map(Sq));
    let p = HP.map(|v| v.map(Sq));
    let start = roots();
    let g = geom(&r);
    let built = roots() - start;
    let _ = force(&m, &g);
    let e = energy(&m, &p, &g);
    let rm = g.r_min();
    (built, roots() - start - built, e.0, rm.0)
}

fn check_one_computation(counts: (usize, usize, f64, f64)) {
    let (built, read, e, rm) = counts;
    assert_eq!(
        built + read,
        3,
        "not one pairwise-distance computation (three roots) per force evaluation and its E and r_min"
    );
    assert_eq!(read, 0, "the force, E or r_min forms a distance of its own");
    let (k, v, _) = hand_energy();
    close(
        "E from the shared geometry",
        e,
        k + v,
        k + v.abs(),
        f64::EPSILON,
    );
    assert_eq!(rm, 6.0, "r_min from the shared geometry");
}

#[test]
fn qa_one_distance_computation() {
    check_one_computation(evaluation(PairGeom::new));
}

negative_control!(
    qa_one_distance_computation,
    "a second geometry formed for the energy is a second distance computation",
    expected = "not one pairwise-distance computation",
    check_one_computation(evaluation(|r| {
        let _ = PairGeom::new(r);
        PairGeom::new(r)
    }))
);

/// E's potential and r_min come from the geometry they are handed: with the hand state's masses and momenta and the
/// geometry of other positions, they are those positions' values.
fn check_reads_geometry(r_geom: V3) {
    let g = PairGeom::new(&r_geom);
    let (k, v, s) = reference_energy(HM, r_geom, HP);
    close(
        "E with another geometry",
        energy(&HM, &HP, &g),
        k + v,
        s,
        f64::EPSILON,
    );
    let least = dist(r_geom, 0, 1)
        .min(dist(r_geom, 1, 2))
        .min(dist(r_geom, 2, 0));
    assert_eq!(g.r_min(), least, "r_min with another geometry");
}

#[test]
fn qa_energy_reads_the_geometry() {
    check_reads_geometry([[1.0, -2.0], [3.5, 0.25], [-0.75, 1.5]]);
}

negative_control!(
    qa_energy_reads_the_geometry,
    "the hand state's energy is not that of other positions",
    expected = "E with another geometry",
    {
        let g = PairGeom::new(&HR);
        let other: V3 = [[1.0, -2.0], [3.5, 0.25], [-0.75, 1.5]];
        let (k, v, s) = reference_energy(HM, other, HP);
        close(
            "E with another geometry",
            energy(&HM, &HP, &g),
            k + v,
            s,
            f64::EPSILON,
        )
    }
);

// ── REQ-PERF-002: the crate the physics layer lives in is no_std ─────────────────────────────────────────────────

fn check_no_std(lib: &str) {
    assert!(
        lib.lines().any(|l| l.trim() == "#![no_std]"),
        "the kernel crate, and so its physics layer, is not #![no_std]"
    );
}

#[test]
fn qa_kernel_no_std() {
    let lib = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"))
        .expect("lib.rs");
    check_no_std(&lib);
}

negative_control!(
    qa_kernel_no_std,
    "a crate root without #![no_std] must fail",
    expected = "is not #![no_std]",
    check_no_std("//! #![no_std] in a comment only\npub mod physics;\n")
);
