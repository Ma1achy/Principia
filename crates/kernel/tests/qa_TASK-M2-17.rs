//! QA tests for TASK-M2-17, written from the requirements, not the implementation:
//! - REQ-ENC-008: a state round-trips through each axis kind (kind 1 on an oblique latent plane, kind 2 on a mixed
//!   chart, kind 3 on both invariant charts, its coordinates checked against `L_z`, `K` computed here from first
//!   principles); a state off the chart lands in latent `z`, `E(x)`'s own.
//! - REQ-ENC-009: entering the `(L_z, E)` computed here from a clicked pixel's state gives the pixel's `z`, and the same
//!   state in physical units.
//! - REQ-ENC-010 / REQ-ENC-030: the feasibility test, fuzzed over the plane of `(L_z, K*)` with its edges and non-finite
//!   entries: a feasible pair is taken as is, an infeasible one never silently, never NaN; a projection lands on the
//!   parabola at the same energy and on `L_z`'s side, and a class change is rejected (the contract's § "Chart-aware
//!   validation"); the fibre lookup on a real chart decodes to the entered energy; a no-seed configuration is the pixel
//!   path's `DEGENERATE`.
//! - REQ-CHART-005: `m₀ = 0.4` under hold-`m₁:m₂` and hold-`m₁ = m₂`: the masses each convention states, two `z`, and
//!   each `z` decodes back to its own convention's masses only; encode of that state gives the convention's controls.
//!
//! Tolerances (applied per R-369, a test decision): "to precision" (REQ-ENC-009) is read as a small multiple of f64
//! eps of the quantity's scale; the landing tolerance is a physical-units norm (inverse_encode_contract Part 4) at
//! `1e-9`, `ε_phys` being uncalibrated (REQ-ENC-024), far above an on-chart residual and below an off-chart one (both
//! printed).
//!
//! Domain: encode alone, on states held fixed (pitfalls §10).

use kernel::chart::invariant::{Invariant, InvariantLE, InvariantLK};
use kernel::chart::mixed::{Alpha, PRhoX};
use kernel::chart::{ChartDecode, Constraint, Latent, Mixed};
use kernel::decode::config;
use kernel::decode::invariant::{construct_by_norms, Outcome};
use kernel::decode::mass::{self, from_masses};
use kernel::decode::{self as dec, Ic};
use kernel::encode::by_kind::{
    encode_into, feasibility, fibre, fibre_at, ByKind, Coord, Feasibility, HoldEqual, HoldRatio,
    InvariantAxis, Landing, Lookup, Residual, SeedRule,
};
use kernel::encode::encode;
use kernel::generated::links::Defaults;
use validation::decode_f32_eps::SplitMix;
use validation::gauge::{uniform, Gauge};
use validation::negative_control;

const TOL: f64 = 1e-9;

/// Max-abs physical-units distance between two states, NaN-propagating.
fn dist(a: &Ic<f64>, b: &Ic<f64>) -> f64 {
    let mut w = 0.0f64;
    for i in 0..3 {
        let mut d = vec![a.m[i] - b.m[i]];
        for k in 0..2 {
            d.push(a.r[i][k] - b.r[i][k]);
            d.push(a.p[i][k] - b.p[i][k]);
        }
        for x in d {
            if x.is_nan() {
                return f64::NAN;
            }
            w = w.max(x.abs());
        }
    }
    w
}

/// `L_z`, `K`, `U`, `E` from first principles, `G = 1`.
fn invariants_here(x: &Ic<f64>) -> (f64, f64, f64, f64) {
    let mut lz = 0.0;
    let mut k = 0.0;
    for i in 0..3 {
        lz += x.r[i][0] * x.p[i][1] - x.r[i][1] * x.p[i][0];
        k += (x.p[i][0] * x.p[i][0] + x.p[i][1] * x.p[i][1]) / (2.0 * x.m[i]);
    }
    let mut u = 0.0;
    for (i, j) in [(0, 1), (0, 2), (1, 2)] {
        let dx = x.r[i][0] - x.r[j][0];
        let dy = x.r[i][1] - x.r[j][1];
        u -= x.m[i] * x.m[j] / (dx * dx + dy * dy).sqrt();
    }
    (lz, k, u, k + u)
}

const SLICE: [f64; 8] = [-0.25, 0.35, 0.4, -0.3, -0.2, 0.55, -0.15, 0.2];

fn frozen() -> Invariant<f64> {
    Invariant {
        z_cfg: [SLICE[0], SLICE[1]],
        z_mass: [SLICE[6], SLICE[7]],
        k_max: 2.0,
        gamma_k: 1.5,
    }
}

fn land<C: ByKind<Param = f64>>(chart: &C, x: &Ic<f64>) -> (Landing<f64>, Option<f64>) {
    let e = encode_into::<Defaults, f64, C, _>(chart, x, dist, TOL);
    (e.landing, e.residual)
}

/// Decodes `from` at interior points, applies a random gauge, encodes into `into`: must land at the same point.
fn round_trip<C: ByKind<Param = f64>>(from: &C, into: &C, seed: u64, name: &str) {
    let mut rng = SplitMix::new(seed);
    let mut worst = 0.0f64;
    for _ in 0..60 {
        let (u, v) = (uniform(&mut rng, 0.15, 0.85), uniform(&mut rng, 0.15, 0.85));
        let x = from.decode_at::<Defaults, f64>(u, v).ic;
        let g = Gauge::random(&mut rng, 2.0, 1.5).act(&x);
        let (landing, residual) = land(into, &g);
        match landing {
            Landing::Chart { u: lu, v: lv } => {
                assert!(
                    (lu - u).abs() < 1e-8 && (lv - v).abs() < 1e-8,
                    "{name}: lands at ({lu}, {lv}), not ({u}, {v})"
                );
            }
            Landing::Latent => {
                panic!("{name}: a state on the chart lands in latent z ({residual:?})")
            }
        }
        worst = worst.max(residual.unwrap());
    }
    eprintln!("qa {name}: worst on-chart residual {worst:e}");
}

// ── REQ-ENC-008 ────────────────────────────────────────────────────────────────────────────────────────────────────

fn oblique() -> Latent<f64> {
    Latent {
        z0: SLICE,
        q1: [0.1, -0.2, 0.3, 0.1, 0.0, -0.2, 0.25, 0.05],
        q2: [0.2, 0.1, -0.1, 0.3, 0.2, 0.1, 0.0, -0.3],
    }
}

fn mixed() -> Mixed<Alpha, PRhoX, f64> {
    Mixed::new(SLICE, [0.2, 1.1], [-0.7, 0.8])
}

#[test]
fn qa_encode_axis_kinds_round_trip() {
    round_trip(&oblique(), &oblique(), 1001, "kind 1, oblique latent");
    round_trip(&mixed(), &mixed(), 1002, "kind 2, (alpha, p_rho_x)");
    let le = InvariantLE(frozen());
    let lk = InvariantLK(frozen());
    round_trip(&le, &le, 1003, "kind 3, (L_z, E)");
    round_trip(&lk, &lk, 1004, "kind 3, (L_z, K)");
}

negative_control!(
    qa_encode_axis_kinds_round_trip,
    "decoding from a different latent slice must not land on the chart",
    expected = "lands in latent z",
    {
        let mut from = oblique();
        from.z0[3] += 0.25;
        round_trip(&from, &oblique(), 1001, "control");
    }
);

/// Kind 3 (a): the invariant chart's coordinates of a state are the warp's inverse of its `L_z` and `K`, both
/// evaluated directly (here, from first principles): `t = (K/K_max)^{1/γ}`, `s = (L_z/√(2K) + 1)/2` at `I = 1`.
fn check_invariant_coords(perturb: f64) {
    let lk = InvariantLK(frozen());
    let f = frozen();
    let mut rng = SplitMix::new(1010);
    for _ in 0..60 {
        let (u, v) = (uniform(&mut rng, 0.1, 0.9), uniform(&mut rng, 0.1, 0.9));
        let x = lk.decode_at::<Defaults, f64>(u, v).ic;
        let g = Gauge::random(&mut rng, 2.0, 1.0).act(&x);
        let e = encode::<Defaults, f64>(&g);
        let (lz, k, _, _) = invariants_here(&e.canonical.ic);
        let t = (k / f.k_max).powf(1.0 / f.gamma_k) + perturb;
        let s = (lz / (2.0 * k).sqrt() + 1.0) / 2.0;
        let [cu, cv] = lk.coords::<Defaults, f64>(&e);
        let (Coord::At(cu), Coord::At(cv)) = (cu, cv) else {
            panic!("an invariant axis did not place the state: {cu:?}, {cv:?}");
        };
        assert!(
            (cu - s).abs() < 1e-10 && (cv - t).abs() < 1e-10,
            "invariant coords ({cu}, {cv}) are not the direct evaluation ({s}, {t})"
        );
    }
}

#[test]
fn qa_encode_axis_kinds_invariant_direct() {
    check_invariant_coords(0.0);
}

negative_control!(
    qa_encode_axis_kinds_invariant_direct,
    "a shifted direct evaluation must disagree",
    expected = "are not the direct evaluation",
    check_invariant_coords(1e-6)
);

/// A generic state: random masses, positions and momenta.
fn generic(rng: &mut SplitMix) -> Ic<f64> {
    let a = uniform(rng, 0.2, 1.0);
    let b = uniform(rng, 0.2, 1.0);
    let c = uniform(rng, 0.2, 1.0);
    let s = a + b + c;
    let mut x = Ic {
        m: [a / s, b / s, c / s],
        r: [[0.0; 2]; 3],
        p: [[0.0; 2]; 3],
    };
    for i in 0..3 {
        for k in 0..2 {
            x.r[i][k] = uniform(rng, -1.5, 1.5);
            x.p[i][k] = uniform(rng, -0.8, 0.8);
        }
    }
    x
}

/// A state off the chart lands in latent `z`, and that `z` is `E(x)`'s.
fn check_unaligned<C: ByKind<Param = f64>>(chart: &C, x: &Ic<f64>, name: &str) {
    let e = encode_into::<Defaults, f64, C, _>(chart, x, dist, TOL);
    assert_eq!(
        e.landing,
        Landing::Latent,
        "{name}: a state off the chart lands on it (residual {:?})",
        e.residual
    );
    let z = encode::<Defaults, f64>(x).z.z;
    assert_eq!(
        e.encoded.z.z, z,
        "{name}: the latent landing is not E(x)'s z"
    );
    // The latent z represents the state: it decodes to C(x).
    let back = dec::decode::<Defaults, f64>(&dec::Latent::new(z)).ic;
    let d = dist(&back, &e.encoded.canonical.ic);
    assert!(d < 1e-9, "{name}: latent z decodes {d:e} from C(x)");
}

#[test]
fn qa_encode_axis_kinds_unaligned() {
    let mut rng = SplitMix::new(1020);
    let le = InvariantLE(frozen());
    let lk = InvariantLK(frozen());
    for _ in 0..40 {
        let x = generic(&mut rng);
        check_unaligned(&oblique(), &x, "kind 1");
        check_unaligned(&mixed(), &x, "kind 2");
        check_unaligned(&le, &x, "(L_z, E)");
        check_unaligned(&lk, &x, "(L_z, K)");
    }
    // A state decoded on the chart, then its momentum perturbed: off the slice, lands in latent z.
    for _ in 0..20 {
        let (u, v) = (uniform(&mut rng, 0.2, 0.8), uniform(&mut rng, 0.2, 0.8));
        let mut x = le.decode_at::<Defaults, f64>(u, v).ic;
        x.p[0][1] += 0.05;
        x.p[2][1] -= 0.05;
        check_unaligned(&le, &x, "(L_z, E) perturbed");
    }
}

negative_control!(
    qa_encode_axis_kinds_unaligned,
    "a state on the chart must fail the off-chart check",
    expected = "a state off the chart lands on it",
    {
        let x = mixed().decode_at::<Defaults, f64>(0.4, 0.6).ic;
        check_unaligned(&mixed(), &x, "control");
    }
);

// ── REQ-ENC-009 ────────────────────────────────────────────────────────────────────────────────────────────────────

/// Entering the pixel's `(L_z, E)` (computed here) gives the clicked pixel's `z` and state, to precision.
fn check_reuse(shift: f64) {
    let le = InvariantLE(frozen());
    let n = 16;
    let (mut worst_z, mut worst_x) = (0.0f64, 0.0f64);
    for i in 1..n {
        for j in 1..n {
            let (u, v) = (i as f64 / n as f64, j as f64 / n as f64);
            let pixel = le.decode_at::<Defaults, f64>(u, v).ic;
            let z_pixel = encode::<Defaults, f64>(&pixel).z.z;
            let (lz, _, _, e) = invariants_here(&pixel);
            let Lookup::Encoded { z, projected, .. } =
                fibre::<Defaults, f64, _>(&le, lz + shift, e)
            else {
                panic!("the pixel's own pair ({lz}, {e}) is not encoded");
            };
            assert_eq!(projected, None, "the pixel's own pair was projected");
            let entered = dec::decode::<Defaults, f64>(&dec::Latent::new(z)).ic;
            let dx = dist(&entered, &pixel);
            let dz = (0..8)
                .map(|k| (z[k] - z_pixel[k]).abs() / (1.0 + z_pixel[k].abs()))
                .fold(0.0, f64::max);
            worst_z = worst_z.max(dz);
            worst_x = worst_x.max(dx);
            assert!(
                dz <= 1e3 * f64::EPSILON && dx <= 1e3 * f64::EPSILON,
                "entered (L_z, E) at ({u}, {v}) differs from the pixel: dz {dz:e}, dx {dx:e}"
            );
        }
    }
    eprintln!("qa reuse: worst dz {worst_z:e}, worst dx {worst_x:e}");
}

#[test]
fn qa_encode_invariant_reuses_decode() {
    check_reuse(0.0);
}

negative_control!(
    qa_encode_invariant_reuses_decode,
    "a different entered L_z must give a different z",
    expected = "differs from the pixel",
    check_reuse(1e-6)
);

// ── REQ-ENC-010, REQ-ENC-030 ───────────────────────────────────────────────────────────────────────────────────────

/// `x = m·2^e`, `m` a non-negative integer below `2^53`.
fn split(x: f64) -> (u128, i32) {
    let bits = x.abs().to_bits();
    let exp = ((bits >> 52) & 0x7ff) as i32;
    let frac = (bits & ((1u64 << 52) - 1)) as u128;
    if exp == 0 {
        (frac, -1074)
    } else {
        (frac | (1u128 << 52), exp - 1075)
    }
}

/// `L_z² ≤ 2K*` in exact arithmetic, for finite inputs.
fn exactly_feasible(lz: f64, k: f64) -> bool {
    if k < 0.0 {
        return false;
    }
    let (ml, el) = split(lz);
    let (mk, ek) = split(k);
    if ml == 0 {
        return true;
    }
    if mk == 0 {
        return false;
    }
    // L_z² = ml²·2^(2el), 2K* = mk·2^(ek+1); compare ml² with mk·2^d.
    let sq = ml * ml;
    let d = ek + 1 - 2 * el;
    if d >= 0 {
        // mk·2^d at or above 2^128 exceeds sq (below 2^106).
        mk.leading_zeros() as i32 <= d || sq <= mk << d
    } else if sq >= 1u128 << 53 || -d >= 54 {
        false
    } else {
        sq << (-d) <= mk
    }
}

/// The contract's verdict on `(lz, k)`, `I = 1`, written from § "Chart-aware validation": valid inside the parabola;
/// non-finite rejected; outside, at `K* > 0` projected to `sign(L_z)√(2K*)` at the same `K*`; at `K* ≤ 0`, `L_z = 0`
/// projected to the apex, any other rejected.
fn check_verdict(lz: f64, k: f64, got: Feasibility<f64>, low: Constraint, wide: Constraint) {
    let ctx = format!("({lz}, {k}) -> {got:?}");
    if !(lz.is_finite() && k.is_finite()) {
        assert!(
            matches!(got, Feasibility::Rejected { .. }),
            "a non-finite entry not rejected: {ctx}"
        );
        return;
    }
    // Feasible: `L_z² ≤ 2K*`, decided exactly (the contract holds the projected root so that the projected pair passes
    // the same exact test).
    if exactly_feasible(lz, k) {
        assert_eq!(
            got,
            Feasibility::Valid,
            "a feasible pair not taken as is: {ctx}"
        );
        return;
    }
    let named = if k < 0.0 { low } else { wide };
    if k > 0.0 {
        let Feasibility::Projected {
            lz: lp,
            k: kp,
            failed,
        } = got
        else {
            panic!("an infeasible pair at K* > 0 not projected: {ctx}");
        };
        assert_eq!(failed, named, "the projection's flag: {ctx}");
        assert_eq!(kp, k, "the projection moved the energy: {ctx}");
        assert!(
            lp.is_finite() && exactly_feasible(lp, kp),
            "not on the feasible set: {ctx}"
        );
        assert_eq!(
            lp.signum(),
            lz.signum(),
            "the projection changed L_z's side: {ctx}"
        );
        let target = lz.signum() * (2.0 * k).sqrt();
        assert!(
            (lp - target).abs() <= 4.0 * f64::EPSILON * target.abs(),
            "not the nearest point: {ctx}"
        );
    } else if lz == 0.0 {
        let Feasibility::Projected {
            lz: lp,
            k: kp,
            failed,
        } = got
        else {
            panic!("L_z = 0 at K* < 0 not projected to the rest start: {ctx}");
        };
        assert_eq!((lp, kp), (0.0, 0.0), "not the apex: {ctx}");
        assert_eq!(failed, named, "the apex projection's flag: {ctx}");
    } else {
        assert_eq!(
            got,
            Feasibility::Rejected { failed: named },
            "a class change not rejected: {ctx}"
        );
    }
}

fn sweep_feasibility(low: Constraint, wide: Constraint, f: impl Fn(f64, f64) -> Feasibility<f64>) {
    let mut rng = SplitMix::new(1030);
    let edges = [
        0.0,
        -0.0,
        1e-300,
        -1e-300,
        1e-12,
        -1e-12,
        0.5,
        -0.5,
        2.0,
        -2.0,
        1e300,
        -1e300,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ];
    for &lz in &edges {
        for &k in &edges {
            check_verdict(lz, k, f(lz, k), low, wide);
        }
    }
    for _ in 0..20_000 {
        let lz = uniform(&mut rng, -3.0, 3.0);
        let k = uniform(&mut rng, -2.0, 3.0);
        check_verdict(lz, k, f(lz, k), low, wide);
        // On the parabola itself, and a hair outside it.
        let kk = k.abs();
        let edge = (2.0 * kk).sqrt();
        check_verdict(edge, kk, f(edge, kk), low, wide);
        let out = edge * (1.0 + 1e-9) + 1e-300;
        check_verdict(-out, kk, f(-out, kk), low, wide);
    }
}

#[test]
fn qa_encode_infeasible_invariant_property() {
    // The exact oracle on known answers.
    let up = f64::from_bits(1.0f64.to_bits() + 1);
    for (lz, k, want) in [
        (1.0, 0.5, true),
        (-1.0, 0.5, true),
        (up, 0.5, false),
        (1.0, f64::from_bits(0.5f64.to_bits() - 1), false),
        (1e-300, 0.0, false),
        (0.0, 0.0, true),
        (3.0, 4.5, true),
        (1e200, 1e300, false),
        (1e100, 1e300, true),
        (1e-200, 1e-300, true),
        (f64::MIN_POSITIVE, 0.0, false),
        (4e-162, 5e-324, false),
    ] {
        assert_eq!(exactly_feasible(lz, k), want, "the oracle at ({lz}, {k})");
    }
    for (low, wide) in [
        (
            <InvariantLE<f64> as InvariantAxis>::LOW,
            <InvariantLE<f64> as InvariantAxis>::WIDE,
        ),
        (
            <InvariantLK<f64> as InvariantAxis>::LOW,
            <InvariantLK<f64> as InvariantAxis>::WIDE,
        ),
    ] {
        sweep_feasibility(low, wide, |lz, k| feasibility(lz, k, low, wide));
    }
    // The constraints the charts name (chart_decoder_contract layer 2): (L_z, E) the parabola; (L_z, K) K ≥ 0 and the
    // L_z bound.
    assert_eq!(
        <InvariantLE<f64> as InvariantAxis>::WIDE,
        Constraint::LzEParabola
    );
    assert_eq!(
        <InvariantLK<f64> as InvariantAxis>::LOW,
        Constraint::KNonNegative
    );
    assert_eq!(
        <InvariantLK<f64> as InvariantAxis>::WIDE,
        Constraint::KAboveLzBound
    );
}

negative_control!(
    qa_encode_infeasible_invariant_property,
    "a test that takes every pair as valid must be caught",
    expected = "not projected",
    sweep_feasibility(Constraint::LzEParabola, Constraint::LzEParabola, |_, _| {
        Feasibility::Valid
    })
);

/// On the `(L_z, E)` chart: fuzzed entries are never NaN, a projection or valid entry decodes to the entered energy,
/// and its `L_z` sits on the feasible set; the flag is set iff the entry was infeasible.
fn check_lookup(e_shift: f64) {
    let le = InvariantLE(frozen());
    let u_pot = {
        let x = le.decode_at::<Defaults, f64>(0.5, 0.5).ic;
        invariants_here(&Ic {
            p: [[0.0; 2]; 3],
            ..x
        })
        .2
    };
    let mut rng = SplitMix::new(1040);
    let (mut projected_n, mut rejected_n) = (0, 0);
    for _ in 0..400 {
        let lz = uniform(&mut rng, -2.0, 2.0);
        let e = u_pot + uniform(&mut rng, -0.5, 1.5);
        let k = e - u_pot;
        match fibre::<Defaults, f64, _>(&le, lz, e) {
            Lookup::Encoded { z, projected, .. } => {
                assert!(z.iter().all(|c| c.is_finite()), "NaN in z for ({lz}, {e})");
                let infeasible = lz * lz > 2.0 * k;
                assert_eq!(
                    projected.is_some(),
                    infeasible,
                    "the flag for ({lz}, {e}): {projected:?}"
                );
                projected_n += projected.is_some() as usize;
                let x = dec::decode::<Defaults, f64>(&dec::Latent::new(z)).ic;
                let (lz_x, k_x, _, e_x) = invariants_here(&x);
                let want_e = if k < 0.0 { u_pot } else { e } + e_shift;
                assert!(
                    (e_x - want_e).abs() < 1e-9,
                    "decodes to E {e_x}, not {want_e}, for ({lz}, {e})"
                );
                assert!(
                    lz_x * lz_x <= 2.0 * k_x + 1e-9,
                    "decodes off the feasible set for ({lz}, {e})"
                );
                if !infeasible {
                    assert!(
                        (lz_x - lz).abs() < 1e-9,
                        "a feasible entry's L_z moved: {lz_x} vs {lz}"
                    );
                } else {
                    assert!(
                        lz_x * lz >= 0.0,
                        "a projection changed L_z's side: {lz_x} vs {lz}"
                    );
                }
            }
            Lookup::Rejected { failed } => {
                rejected_n += 1;
                assert!(lz * lz > 2.0 * k, "a feasible pair ({lz}, {e}) rejected");
                assert!(
                    k <= 0.0 && lz != 0.0,
                    "a projectable pair ({lz}, {e}) rejected"
                );
                assert_eq!(failed, Constraint::LzEParabola);
            }
            Lookup::Degenerate => panic!("a decoded configuration degenerates at ({lz}, {e})"),
        }
    }
    assert!(
        projected_n > 20 && rejected_n > 20,
        "fuzz coverage: {projected_n} projected, {rejected_n} rejected"
    );
    for (lz, e) in [
        (f64::NAN, 0.0),
        (0.0, f64::NAN),
        (f64::INFINITY, u_pot + 1.0),
        (0.3, f64::NEG_INFINITY),
    ] {
        assert!(
            matches!(
                fibre::<Defaults, f64, _>(&le, lz, e),
                Lookup::Rejected { .. }
            ),
            "a non-finite entry ({lz}, {e}) is not rejected"
        );
    }
}

#[test]
fn qa_encode_infeasible_invariant_lookup() {
    check_lookup(0.0);
}

negative_control!(
    qa_encode_infeasible_invariant_lookup,
    "a wrong expected energy must be caught",
    expected = "decodes to E",
    check_lookup(1e-3)
);

/// At a configuration with no qualifying seed, every feasible pair gives the pixel path's `DEGENERATE`, and only there.
fn check_degenerate(cfg: &config::Config<f64>) {
    let masses = from_masses([0.25, 0.45, 0.3]);
    let wide = (Constraint::LzEParabola, Constraint::LzEParabola);
    for (lz, k) in [(0.0, 0.3), (0.0, 1.0), (0.0, 1e-3)] {
        let pixel = construct_by_norms(&masses, cfg, lz, k).outcome;
        let got = fibre_at::<Defaults, f64>(&masses, cfg, SeedRule::ByNorms, (lz, k), wide);
        assert_eq!(
            pixel == Outcome::Degenerate,
            got == Lookup::Degenerate,
            "{pixel:?} vs {got:?}"
        );
        assert_eq!(
            got,
            Lookup::Degenerate,
            "no DEGENERATE at ({lz}, {k}): {got:?}"
        );
    }
}

#[test]
fn qa_encode_infeasible_invariant_degenerate() {
    // Every body coincident: no seed has a norm.
    let cfg = config::Config {
        alpha: 0.0,
        beta: 0.0,
        rho_t: [0.0; 2],
        lambda_t: [0.0; 2],
        rho: [0.0; 2],
        lambda: [0.0; 2],
        r: [[0.0; 2]; 3],
    };
    check_degenerate(&cfg);
}

negative_control!(
    qa_encode_infeasible_invariant_degenerate,
    "a decoded, non-degenerate configuration must fail the DEGENERATE check",
    expected = "no DEGENERATE",
    {
        let masses = from_masses([0.25, 0.45, 0.3]);
        check_degenerate(&config::decode::<Defaults, f64>([0.1, 0.2], &masses));
    }
);

// ── REQ-CHART-005 ──────────────────────────────────────────────────────────────────────────────────────────────────

fn close(a: [f64; 3], b: [f64; 3]) -> bool {
    (0..3).all(|i| (a[i] - b[i]).abs() < 1e-12)
}

/// `m₀ = 0.4` under the two conventions, at the slice's masses `held`.
fn check_conventions(held: [f64; 3], swap: bool) {
    let m0 = 0.4;
    let rest = 1.0 - m0;
    // Written from the conventions' statements, not the code.
    let want_ratio = [
        m0,
        rest * held[1] / (held[1] + held[2]),
        rest * held[2] / (held[1] + held[2]),
    ];
    let want_equal = [m0, rest / 2.0, rest / 2.0];
    let (want_ratio, want_equal) = if swap {
        (want_equal, want_ratio)
    } else {
        (want_ratio, want_equal)
    };
    assert!(
        close(HoldRatio::masses(m0, held), want_ratio),
        "hold-ratio masses {:?}",
        HoldRatio::masses(m0, held)
    );
    assert!(
        close(HoldEqual::masses(m0, held), want_equal),
        "hold-equal masses {:?}",
        HoldEqual::masses(m0, held)
    );
    let zr = HoldRatio::controls::<Defaults, f64>(m0, held);
    let ze = HoldEqual::controls::<Defaults, f64>(m0, held);
    assert!(
        (zr[0] - ze[0]).abs() + (zr[1] - ze[1]).abs() > 1e-6,
        "the conventions give the same z: {zr:?}"
    );
    for (z, own, other, name) in [
        (zr, want_ratio, want_equal, "ratio"),
        (ze, want_equal, want_ratio, "equal"),
    ] {
        let back = mass::decode::<Defaults, f64>(z).m;
        assert!(
            close(back, own),
            "{name}'s z decodes to {back:?}, not its own convention's {own:?}"
        );
        assert!(
            !close(back, other),
            "{name}'s z round-trips under the other convention"
        );
        // Encode, on a state with those masses, uses the same convention: it gives the forward axis's controls.
        let mut x = generic(&mut SplitMix::new(1050));
        x.m = back;
        let ez = encode::<Defaults, f64>(&x).z.z;
        assert!(
            (ez[6] - z[0]).abs() < 1e-12 && (ez[7] - z[1]).abs() < 1e-12,
            "{name}: encode's mass controls {:?} are not the forward axis's {z:?}",
            &ez[6..8]
        );
    }
    assert_ne!(
        HoldRatio::STATEMENT,
        HoldEqual::STATEMENT,
        "the conventions' labels"
    );
    assert!(!HoldRatio::STATEMENT.is_empty() && !HoldEqual::STATEMENT.is_empty());
}

#[test]
fn qa_encode_residual_convention() {
    check_conventions([0.2, 0.5, 0.3], false);
    check_conventions([0.5, 0.1, 0.4], false);
}

negative_control!(
    qa_encode_residual_convention,
    "swapped conventions must be caught",
    expected = "hold-ratio masses",
    check_conventions([0.2, 0.5, 0.3], true)
);
