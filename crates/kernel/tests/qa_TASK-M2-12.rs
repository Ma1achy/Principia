//! QA tests for TASK-M2-12, written from the requirements (chart_reference §2, §2.1, §2.2, §5.2; REQ-CHART-017,
//! REQ-CHART-031), not from the implementation. Each check runs at the f64 and the f32 instantiation of the one source,
//! natively (R-265).
//! - `qa_m212_warp_follows_section_2_1` (f64), `qa_m212_warp_follows_section_2_1_f32`: the targets are §2.1's warp, `K(t) = K_max·t^γ_K`, `L_max(t) = √(2·I·K(t))`
//!   (`I = 1` on the canonical-frame decode, §2.2), `L_z(s, t) = (2s − 1)·L_max(t)`, and on `(L_z, E)` the axis is
//!   `E(t) = U + K(t)`, against an independent f64 transcription; over random `K_max > 0`, `γ_K ≥ 1` (non-integer
//!   included) and `(s, t) ∈ [0,1]²`.
//! - `qa_m212_warp_feasible_lz_e`, `qa_m212_warp_feasible_lz_k`: REQ-CHART-017's property (chart_reference §5.2): no `(u, v) ∈ [0,1]²` produces
//!   `K* < K_min`, on both charts, over random frozen configurations and masses, `K_max`, `γ_K`, with the square's
//!   edges and corners drawn on purpose. Checked twice: the construction's own outcome through the chart's decode is
//!   never `Infeasible`, and `2·K* ≥ L_z²` holds exactly, decided here without rounding (an f32 product is exact in
//!   f64; an f64 square is split error-free by `fma`). The chart fixes geometry and mass (§2): every pixel's masses
//!   and positions equal, bit for bit, those of the chart's centre pixel.
//! - `qa_m212_warp_reaches_the_parabola`: the warp maps the square *onto* the feasible interior (§2.1): the edge
//!   columns `s ∈ {0, 1}` lie on the parabola `|L_z| = √(2K)` to rounding, the centre column on `L_z = 0`, and the row
//!   `t = 0` at the apex `(L_z, K) = (0, 0)`, the rest start; just past `s = 1` the decode is infeasible (§2.1, R-407:
//!   `|2s − 1| > 1` gives `K* < K_min`), so the square's edge is the parabola, not inside it.
//! - `qa_m212_flags`: both invariant charts declare `forbids_energy_normalisation`, and the latent chart does not
//!   (§0.6, §5.1).
//!
//! **Tolerances**: the exact feasibility test has none (§2.2, "no rounding at the decision"). The warp's transcription
//! is compared within a few `eps` of the instantiation, the rounding of the few operations §2.1 writes (a `powf`, a
//! product, a square root), not a value picked to pass: `K` to `8·eps·K`, `L_max` to `8·eps·√(2K)`, `L_z` to
//! `8·eps·L_max`; the edge column's reach to `16·eps·√(2K)`.
//!
//! **Domain** (pitfalls §10): the stateless decode, its inputs held fixed; no trajectory.
//!
//! Each test has a registered negative control naming the panic it expects (R-176, R-212).

use std::fmt::Debug;

use kernel::chart::invariant::{self, Invariant, Targets, WarpReal};
use kernel::chart::{Chart, InvariantLE, InvariantLK, Latent, ValidationResult};
use kernel::decode::invariant::Outcome;
use kernel::decode::{self as dec, Decoded};
use kernel::generated::links::{Defaults, LinkReal};
use kernel::payload::PayloadReal;
use proptest::prelude::*;
use validation::negative_control;
use validation::prop;

// ── Precisions ──────────────────────────────────────────────────────────────────────────────────────────────────

/// An instantiation under test.
trait Q: WarpReal + LinkReal + PayloadReal + Copy + Debug + PartialOrd {
    const EPS: f64;
    const LABEL: &'static str;
    fn f(self) -> f64;
    fn of(x: f64) -> Self;
    /// Whether `2·k ≥ l²` holds exactly.
    fn exactly_feasible(k: Self, l: Self) -> bool;
}

impl Q for f64 {
    const EPS: f64 = f64::EPSILON;
    const LABEL: &'static str = "f64";
    fn f(self) -> f64 {
        self
    }
    fn of(x: f64) -> Self {
        x
    }
    fn exactly_feasible(k: f64, l: f64) -> bool {
        // l² = hi + lo exactly (fma split), 2k exact. Far from hi the comparison is decided by hi alone (|lo| is at
        // most half an ulp of hi); within a factor 2 of it, 2k − hi is exact (Sterbenz) and is compared with lo.
        let hi = l * l;
        let lo = l.mul_add(l, -hi);
        let two_k = k + k;
        if two_k >= hi + hi {
            true
        } else if two_k + two_k <= hi {
            false
        } else {
            two_k - hi >= lo
        }
    }
}

impl Q for f32 {
    const EPS: f64 = f32::EPSILON as f64;
    const LABEL: &'static str = "f32";
    fn f(self) -> f64 {
        f64::from(self)
    }
    fn of(x: f64) -> Self {
        x as f32
    }
    fn exactly_feasible(k: f32, l: f32) -> bool {
        let l = f64::from(l);
        2.0 * f64::from(k) >= l * l
    }
}

// ── The charts ──────────────────────────────────────────────────────────────────────────────────────────────────

fn inv<R: Q>(z: [f64; 4], k_max: f64, gamma_k: f64) -> Invariant<R> {
    Invariant {
        z_cfg: [R::of(z[0]), R::of(z[1])],
        z_mass: [R::of(z[2]), R::of(z[3])],
        k_max: R::of(k_max),
        gamma_k: R::of(gamma_k),
    }
}

/// A chart's decode at `(u, v)` and the construction's outcome.
type Decode<R> = fn(&Invariant<R>, R, R) -> (Decoded<R>, Outcome);

fn decode_le<R: Q>(c: &Invariant<R>, u: R, v: R) -> (Decoded<R>, Outcome) {
    invariant::decode::<Defaults, R, _>(&InvariantLE(*c), u, v)
}

fn decode_lk<R: Q>(c: &Invariant<R>, u: R, v: R) -> (Decoded<R>, Outcome) {
    invariant::decode::<Defaults, R, _>(&InvariantLK(*c), u, v)
}

/// The targets at `(s, t)`, as the chart computes them.
type Warp<R> = fn(&Invariant<R>, R, R) -> Targets<R>;

fn chart_warp<R: Q>(c: &Invariant<R>, s: R, t: R) -> Targets<R> {
    c.targets(s, t)
}

// ── qa_m212_warp_follows_section_2_1 ────────────────────────────────────────────────────────────────────────────

/// §2.1 transcribed at f64 from the instantiation's own parameters: `(K, L_max, L_z)`.
fn section_2_1(k_max: f64, gamma_k: f64, s: f64, t: f64) -> [f64; 3] {
    let k = k_max * t.powf(gamma_k);
    let l_max = (2.0 * 1.0 * k).sqrt();
    [k, l_max, (2.0 * s - 1.0) * l_max]
}

fn check_section_2_1<R: Q>(warp: Warp<R>, k_max: f64, gamma_k: f64, s: f64, t: f64) {
    let c = inv::<R>([0.0; 4], k_max, gamma_k);
    let (s, t) = (R::of(s), R::of(t));
    let got = warp(&c, s, t);
    let [k, l_max, lz] = section_2_1(c.k_max.f(), c.gamma_k.f(), s.f(), t.f());
    let e = R::EPS;
    let ctx = format!(
        "{}: K_max = {k_max}, γ_K = {gamma_k}, (s, t) = ({:?}, {:?})",
        R::LABEL,
        s,
        t
    );
    assert!(
        (got.k.f() - k).abs() <= 8.0 * e * k,
        "K(t) is not K_max·t^γ_K: {:?} against {k}; {ctx}",
        got.k
    );
    assert!(
        (got.l_max.f() - l_max).abs() <= 8.0 * e * l_max,
        "L_max(t) is not √(2·I·K(t)): {:?} against {l_max}; {ctx}",
        got.l_max
    );
    assert!(
        (got.lz.f() - lz).abs() <= 8.0 * e * l_max,
        "L_z(s, t) is not (2s − 1)·L_max(t): {:?} against {lz}; {ctx}",
        got.lz
    );
    // The (L_z, E) chart's vertical axis is E(t) = U + K(t), U the frozen configuration's potential.
    for u_pot in [-0.192, -1.5, 0.0] {
        let want = u_pot + k;
        let got_e = InvariantLE(c).energy(R::of(u_pot), t).f();
        assert!(
            (got_e - want).abs() <= 8.0 * e * (u_pot.abs() + k).max(e),
            "E(t) is not U + K(t): {got_e} against {want} at U = {u_pot}; {ctx}"
        );
    }
}

fn check_follows<R: Q>(warp: Warp<R>) {
    let strategy = (0.01f64..10.0, 1.0f64..4.0, 0.0f64..=1.0, 0.0f64..=1.0);
    prop::run(&strategy, |(k_max, gamma_k, s, t)| {
        check_section_2_1::<R>(warp, k_max, gamma_k, s, t);
        Ok(())
    });
    for (k_max, gamma_k) in [(1.0, 1.0), (1.0, 2.0), (0.37, 2.5), (5.0, 3.0)] {
        for s in [0.0, 0.25, 0.5, 1.0] {
            for t in [0.0, 1e-3, 0.5, 1.0] {
                check_section_2_1::<R>(warp, k_max, gamma_k, s, t);
            }
        }
    }
}

#[test]
fn qa_m212_warp_follows_section_2_1() {
    check_follows::<f64>(chart_warp::<f64>);
}

#[test]
fn qa_m212_warp_follows_section_2_1_f32() {
    check_follows::<f32>(chart_warp::<f32>);
}

/// A warp with `K = K_max·t^(1/γ_K)`: the exponent inverted.
#[cfg(feature = "controls")]
fn inverted_exponent<R: Q>(c: &Invariant<R>, s: R, t: R) -> Targets<R> {
    let flipped = Invariant {
        gamma_k: R::of(1.0 / c.gamma_k.f()),
        ..*c
    };
    flipped.targets(s, t)
}

negative_control!(
    qa_m212_warp_follows_section_2_1,
    "A warp whose exponent is inverted is not §2.1's",
    expected = "K(t) is not K_max·t^γ_K",
    check_follows::<f64>(inverted_exponent::<f64>)
);

/// A warp whose `L_z` axis spans `(2s − 1)·L_max/2`: half the parabola's width.
#[cfg(feature = "controls")]
fn half_width<R: Q>(c: &Invariant<R>, s: R, t: R) -> Targets<R> {
    let w = c.targets(s, t);
    Targets {
        lz: R::of(w.lz.f() / 2.0),
        ..w
    }
}

negative_control!(
    qa_m212_warp_follows_section_2_1_f32,
    "A warp whose L_z spans half the parabola is not §2.1's",
    expected = "L_z(s, t) is not (2s − 1)·L_max(t)",
    check_follows::<f32>(half_width::<f32>)
);

// ── qa_m212_warp_feasible ───────────────────────────────────────────────────────────────────────────────────────

/// A coordinate in `[0, 1]`, the edges and their neighbours drawn often.
fn coord() -> impl Strategy<Value = f64> {
    prop_oneof![
        3 => 0.0f64..=1.0,
        1 => proptest::sample::select(vec![
            0.0,
            1.0,
            0.5,
            f64::MIN_POSITIVE,
            1e-30,
            1e-7,
            1.0 - f64::EPSILON / 2.0,
            1.0 - f32::EPSILON as f64 / 2.0,
            f32::EPSILON as f64,
            0.999_999,
        ]),
    ]
}

/// The pixel `(u, v)` of `c` under `decode` and `warp`: the construction feasible and `2·K* ≥ L_z²` exactly, the
/// masses and positions those of the centre pixel.
fn check_pixel<R: Q>(decode: Decode<R>, warp: Warp<R>, c: &Invariant<R>, u: R, v: R, chart: &str) {
    let ctx = format!(
        "{} {chart}: z = ({:?}, {:?}), K_max = {:?}, γ_K = {:?}, (u, v) = ({:?}, {:?})",
        R::LABEL,
        c.z_cfg,
        c.z_mass,
        c.k_max,
        c.gamma_k,
        u,
        v
    );
    let t = warp(c, u, v);
    assert!(
        R::exactly_feasible(t.k, t.lz),
        "the warp produces K* < K_min: K* = {:?}, L_z = {:?}; {ctx}",
        t.k,
        t.lz
    );
    let (d, outcome) = decode(c, u, v);
    assert!(
        outcome != Outcome::Infeasible,
        "the decode finds the pixel infeasible; {ctx}"
    );
    let (centre, _) = decode(c, R::of(0.5), R::of(0.5));
    let bits = |ic: &dec::Ic<R>| {
        (
            ic.m.map(|m| m.f().to_bits()),
            ic.r.map(|r| r.map(|x| x.f().to_bits())),
        )
    };
    assert!(
        bits(&d.ic) == bits(&centre.ic),
        "the chart does not fix geometry and mass: m = {:?}, r = {:?} against the centre's {:?}, {:?}; {ctx}",
        d.ic.m,
        d.ic.r,
        centre.ic.m,
        centre.ic.r
    );
}

fn check_feasible<R: Q>(decode: Decode<R>, warp: Warp<R>, chart: &str) {
    let z = || -3.0f64..3.0;
    let strategy = (
        [z(), z(), z(), z()],
        prop_oneof![1 => Just(1.0f64), 3 => 1e-3f64..20.0],
        prop_oneof![1 => Just(1.0f64), 1 => Just(2.0f64), 3 => 1.0f64..5.0],
        coord(),
        coord(),
    );
    prop::run(&strategy, |(z, k_max, gamma_k, u, v)| {
        let c = inv::<R>(z, k_max, gamma_k);
        check_pixel::<R>(decode, warp, &c, R::of(u), R::of(v), chart);
        Ok(())
    });
    // The registered defaults (REQ-CHART-044's proposal), on a grid with every edge and corner.
    let k_max = validation::K_MAX.number();
    let gamma_k = validation::GAMMA_K.number();
    let n = 65;
    for z in [
        [0.0; 4],
        [0.3, -0.5, 0.7, -0.4],
        [1.8, 0.4, -1.0, 1.2],
        [-2.5, 2.0, 2.0, -2.0],
    ] {
        let c = inv::<R>(z, k_max, gamma_k);
        for j in 0..n {
            for i in 0..n {
                let (u, v) = (i as f64 / (n - 1) as f64, j as f64 / (n - 1) as f64);
                check_pixel::<R>(decode, warp, &c, R::of(u), R::of(v), chart);
            }
        }
    }
}

#[test]
fn qa_m212_warp_feasible_lz_e() {
    check_feasible::<f64>(decode_le::<f64>, chart_warp::<f64>, "(L_z, E)");
    check_feasible::<f32>(decode_le::<f32>, chart_warp::<f32>, "(L_z, E)");
}

#[test]
fn qa_m212_warp_feasible_lz_k() {
    check_feasible::<f64>(decode_lk::<f64>, chart_warp::<f64>, "(L_z, K)");
    check_feasible::<f32>(decode_lk::<f32>, chart_warp::<f32>, "(L_z, K)");
}

/// The warp with `L_max` a few ulps above the root, nothing held to `L_max² ≤ 2K`.
#[cfg(feature = "controls")]
fn rounded_root<R: Q>(c: &Invariant<R>, s: R, t: R) -> Targets<R> {
    let k = R::of(c.k_max.f() * t.f().powf(c.gamma_k.f()));
    let l_max = R::of((2.0 * k.f()).sqrt() * (1.0 + 4.0 * R::EPS));
    Targets {
        k,
        l_max,
        lz: R::of((2.0 * s.f() - 1.0) * l_max.f()),
    }
}

/// The decode of the targets of [`rounded_root`]: the construction given `L_z` a few ulps outside the parabola.
#[cfg(feature = "controls")]
fn decode_rounded_root<R: Q>(c: &Invariant<R>, u: R, v: R) -> (Decoded<R>, Outcome) {
    let t = rounded_root(c, u, v);
    let mut blocks = invariant::InvariantBlocks::<Defaults, R>::new(t.lz, t.k);
    let d = dec::decode_with(&mut blocks, &InvariantLE(*c).map(u, v).z);
    (d, blocks.outcome())
}

negative_control!(
    qa_m212_warp_feasible_lz_e,
    "A warp whose edge columns lie a few ulps outside the parabola produces K* < K_min",
    expected = "the warp produces K* < K_min",
    check_feasible::<f32>(decode_le::<f32>, rounded_root::<f32>, "(L_z, E)")
);

/// The same, with the exact check skipped: the decode's own outcome must catch it.
fn check_decode_only<R: Q>(decode: Decode<R>) {
    let c = inv::<R>([0.3, -0.5, 0.7, -0.4], 1.0, 2.0);
    for u in [0.0, 1.0] {
        let (_, outcome) = decode(&c, R::of(u), R::of(0.75));
        assert!(
            outcome != Outcome::Infeasible,
            "the decode finds the pixel infeasible at u = {u}"
        );
    }
}

#[test]
fn qa_m212_warp_feasible_edge_decode() {
    check_decode_only::<f32>(decode_le::<f32>);
    check_decode_only::<f64>(decode_lk::<f64>);
}

negative_control!(
    qa_m212_warp_feasible_edge_decode,
    "The construction finds a pixel just outside the parabola infeasible",
    expected = "the decode finds the pixel infeasible",
    check_decode_only::<f64>(decode_rounded_root::<f64>)
);

/// An `(L_z, K)` chart whose configuration control moves with the pixel: geometry not fixed.
#[cfg(feature = "controls")]
fn unfixed<R: Q>(c: &Invariant<R>, u: R, v: R) -> (Decoded<R>, Outcome) {
    let moved = Invariant {
        z_cfg: [R::of(c.z_cfg[0].f() + u.f()), c.z_cfg[1]],
        ..*c
    };
    decode_lk(&moved, u, v)
}

negative_control!(
    qa_m212_warp_feasible_lz_k,
    "A chart whose configuration moves with the pixel does not fix geometry",
    expected = "the chart does not fix geometry and mass",
    check_feasible::<f64>(unfixed::<f64>, chart_warp::<f64>, "(L_z, K), moving")
);

// ── qa_m212_warp_reaches_the_parabola ───────────────────────────────────────────────────────────────────────────

fn check_reaches<R: Q>(warp: Warp<R>, decode: Decode<R>) {
    for (k_max, gamma_k) in [(1.0, 2.0), (1.0, 1.0), (3.0, 2.7), (0.05, 1.5)] {
        let c = inv::<R>([0.3, -0.5, 0.7, -0.4], k_max, gamma_k);
        let ctx = |t: f64| format!("{}: K_max = {k_max}, γ_K = {gamma_k}, t = {t}", R::LABEL);
        // The apex row: (L_z, K) = (0, 0), the rest start.
        for s in [0.0, 0.5, 1.0] {
            let w = warp(&c, R::of(s), R::of(0.0));
            assert!(
                w.k.f() == 0.0 && w.lz.f() == 0.0,
                "the row t = 0 is not the apex: K = {:?}, L_z = {:?} at s = {s}; {}",
                w.k,
                w.lz,
                ctx(0.0)
            );
        }
        for t in [1e-3, 0.1, 0.5, 0.9, 1.0] {
            let centre = warp(&c, R::of(0.5), R::of(t));
            assert!(
                centre.lz.f() == 0.0,
                "the centre column is not L_z = 0: {:?}; {}",
                centre.lz,
                ctx(t)
            );
            for s in [0.0, 1.0] {
                let w = warp(&c, R::of(s), R::of(t));
                let edge = (2.0 * w.k.f()).sqrt();
                assert!(
                    w.lz.f().abs() >= edge * (1.0 - 16.0 * R::EPS),
                    "the edge column s = {s} does not reach the parabola: |L_z| = {:?} against √(2K) = {edge}; {}",
                    w.lz,
                    ctx(t)
                );
                assert!(
                    (w.lz.f() > 0.0) == (s == 1.0),
                    "L_z's sign at s = {s} is not 2s − 1's: {:?}; {}",
                    w.lz,
                    ctx(t)
                );
            }
            // Just past s = 1 the decode is infeasible (§2.1, R-407).
            let (_, outcome) = decode(&c, R::of(1.01), R::of(t));
            assert!(
                outcome == Outcome::Infeasible,
                "s = 1.01 is not infeasible: {outcome:?}; {}",
                ctx(t)
            );
        }
        // Validation: inside the square every point is valid (layer 2 holds by the warp); past s it is not.
        let chart = InvariantLK(c);
        for (u, v) in [(0.0, 0.0), (1.0, 1.0), (0.0, 1.0), (1.0, 0.0), (0.5, 0.3)] {
            assert_eq!(
                chart.validate(u, v),
                ValidationResult::Valid,
                "({u}, {v}) inside the square is not valid"
            );
        }
        for (u, v) in [(1.01, 0.5), (-0.01, 0.5), (0.5, -0.01)] {
            assert_ne!(
                chart.validate(u, v),
                ValidationResult::Valid,
                "({u}, {v}) past the square is valid"
            );
        }
    }
}

#[test]
fn qa_m212_warp_reaches_the_parabola() {
    check_reaches::<f64>(chart_warp::<f64>, decode_le::<f64>);
    check_reaches::<f32>(chart_warp::<f32>, decode_lk::<f32>);
}

/// A warp whose `L_z` spans 90% of the parabola: inside it, not onto it.
#[cfg(feature = "controls")]
fn shrunk<R: Q>(c: &Invariant<R>, s: R, t: R) -> Targets<R> {
    let w = c.targets(s, t);
    Targets {
        lz: R::of(w.lz.f() * 0.9),
        ..w
    }
}

negative_control!(
    qa_m212_warp_reaches_the_parabola,
    "A warp spanning 90% of the parabola does not map onto the feasible interior",
    expected = "does not reach the parabola",
    check_reaches::<f64>(shrunk::<f64>, decode_le::<f64>)
);

// ── qa_m212_flags ───────────────────────────────────────────────────────────────────────────────────────────────

fn check_flags(charts: &[(&str, bool)]) {
    for (name, forbids) in charts {
        let want = name.starts_with("Invariant");
        assert_eq!(
            *forbids, want,
            "{name}'s forbids_energy_normalisation is {forbids}, §0.6 says {want}"
        );
    }
}

fn flags() -> Vec<(&'static str, bool)> {
    let c = inv::<f64>([0.0; 4], 1.0, 2.0);
    let latent = Latent::<f64> {
        z0: [0.0; 8],
        q1: [0.0; 8],
        q2: [0.0; 8],
    };
    vec![
        ("InvariantLE", InvariantLE(c).forbids_energy_normalisation()),
        ("InvariantLK", InvariantLK(c).forbids_energy_normalisation()),
        ("Latent", latent.forbids_energy_normalisation()),
    ]
}

#[test]
fn qa_m212_flags() {
    let f = flags();
    assert_eq!(f[0].0, InvariantLE(inv::<f64>([0.0; 4], 1.0, 2.0)).name());
    assert_eq!(f[1].0, InvariantLK(inv::<f64>([0.0; 4], 1.0, 2.0)).name());
    check_flags(&f);
}

negative_control!(
    qa_m212_flags,
    "An invariant chart that allows η_E breaks §0.6",
    expected = "forbids_energy_normalisation is false",
    check_flags(&[("InvariantLK", false)])
);
