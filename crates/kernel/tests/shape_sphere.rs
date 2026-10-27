//! The shape-sphere chart and the shape map's landmarks (`kernel::chart::shape_sphere`, `kernel::shape`; TASK-M2-08).
//! - `shape_vec_poles`: `L⁺` (bodies 0 → 1 → 2 anticlockwise, equilateral) is `w = +1`, `L⁻` is `w = −1`, and the
//!   collision of bodies 0 and 1 is exactly `(−1, 0, 0)`, under several masses, turned and scaled, at f64 and f32,
//!   through `shape` and `shape_vec` (REQ-CHART-018).
//! - `shape_vec_inspector`: `n` against the IC Inspector's on `fixtures/ic_inspector/shape_vec.json`'s random ICs,
//!   within `shape_vec_js_tol` (REQ-CHART-018, REQ-CHART-049), printing the measured worst.
//! - `shape_sphere_chart_map_spherical`: `(s, 1)` is `L⁺` exactly, `(s, 0)` is `L⁻`, and the polar angle runs over the
//!   whole `[0, π]`, `π·(1 − t)`, with the azimuth `2π·s`, at f64 and f32 (REQ-CHART-019).
//! - `shape_sphere_chart_map_exponential`: the exponential map at `d = 0` is `n₀` exactly, its points are unit, and
//!   each lies at the geodesic distance `r = ‖d‖` from `n₀` (REQ-CHART-019).
//! - `shape_sphere_inverse_poles`: at `n₀ = ±1` with `±0` in `n₁` and `n₂`, `β` is R-437's `+0` and `ψ = φ_f`; an `n`
//!   off the sphere by a rounding forms no root of a negative (REQ-CHART-020).
//! - `shape_sphere_roundtrip`: `Φ` alone (R-178), `shape(Φ(u, v)) = n(u, v)` within `shape_roundtrip_bound` on both
//!   hemispheres, the gate's measure in miniature (REQ-CHART-020, REQ-CHART-053).
//! - `shape_sphere_roundtrip_near_poles`: the same within polar radii `10⁻²` to `10⁻¹⁵` of `n₀ = ±1`, where `n₀` rounds
//!   to `±1` and the smaller of `a` and `b` must come from `n₁² + n₂²` (REQ-CHART-020).
//! - `shape_sphere_canonical_form`: `Φ` then `C` with its step 3 not taken keeps every point's signed shape, a
//!   lower-hemisphere point below the equator, with the held masses and momenta, and never mirrors (R-544, R-178;
//!   REQ-CHART-020, REQ-CHART-048).
//! - `shape_sphere_shared_decoder`: the chart's decode is the shared blocks, the configuration [`place`]'s, `C` with its
//!   step 3 not taken after, and its `validate` is `Valid` everywhere (REQ-CHART-015's shape, R-544).
//! - `shape_sphere_decode_canonical`: the decode's `ic` is `C(Φ)` with step 3 not taken, on `I = 1`, and its
//!   descriptor that state's, at `I ≠ 1` and on both hemispheres (dd_decoder §2, §3.6; REQ-CHART-015, R-544).
//! - `shape_sphere_energy_norm`: `η_E`'s entry, `energy_norm::decode_chart`, decodes the shape sphere through its own
//!   decode, the configuration `place`'s at `n(u, v)`, with and without a target `E*` (REQ-CHART-015).
//! - `shape_landmarks`: `BC₀₁ = (−1, 0, 0)`, `L⁺ = (0, 0, 1)`, every collision at `w = 0`, equal masses 120° apart, and
//!   the collisions against the inspector's (REQ-CHART-036).
//! - `shape_landmarks_mass_weighted`: dd_integrator test 9, `BC₀₁ = (−1, 0, 0)` for any masses, `L⁺ = (0, 0, 1)`,
//!   every collision at `w = 0`, equal masses at 180°, 60°, 300°, and each collision the `n` of any configuration with
//!   that pair together (REQ-INT-003).
//! - `shape_dot_on_collision_landmark`: bodies `i` and `j` brought together put the dot on the landmark opposite `k`,
//!   and keep it off the other two (REQ-VAL-023).
//!
//! **Tolerances.** The inspector's and the round trip's are the register's (R-427). The others are multiples of the
//! precision's epsilon, set from the worst case each test measures, with headroom (applied per R-369: a test decision,
//! as `canon.rs`'s).

use std::path::Path;

use kernel::canon::{self, canonicalise, canonicalise_signed};
use kernel::chart::shape_sphere::{
    exponential, held_momentum, inverse, place, spherical, Inverse, ShapeMap, ShapeSphere,
};
use kernel::chart::{self, Chart, ValidationResult};
use kernel::decode::energy_norm::{self, Outcome};
use kernel::decode::{descriptor, mass, momentum, Decoded, Ic};
use kernel::generated::links::Defaults;
use kernel::shape::{landmarks, shape, shape_vec, weighted_jacobi, Landmarks};
use kernel::stage::decode::StickBreakingLinks;
use proptest::prelude::*;
use validation::decode_f32_eps::SplitMix;
use validation::gauge::Gauge;
use validation::roundtrip::{eps_phys, phys, registry, DecodeAt, Precision};
use validation::{negative_control, prop, SHAPE_ROUNDTRIP_BOUND, SHAPE_VEC_JS_TOL};

const PI: f64 = std::f64::consts::PI;
const EPS: f64 = f64::EPSILON;

/// `shape_vec_js_tol` (REQ-CHART-049), provisional until M2.
const JS_TOL: f64 = SHAPE_VEC_JS_TOL.number();

/// `shape_roundtrip_bound` (REQ-CHART-053, R-181), provisional until M2.
const ROUNDTRIP: f64 = SHAPE_ROUNDTRIP_BOUND.number();

// ── Helpers ─────────────────────────────────────────────────────────────────────────────────────────────────────────

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

fn widen<R: Into<f64> + Copy>(n: [R; 3]) -> [f64; 3] {
    n.map(Into::into)
}

/// A uniform draw on `[lo, hi]` at f64.
fn on(rng: &mut SplitMix, lo: f64, hi: f64) -> f64 {
    lo + (hi - lo) * ((rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64)
}

/// Masses from a random logit pair, through the shared mass block.
fn masses(rng: &mut SplitMix) -> [f64; 3] {
    mass::decode::<Defaults, f64>([on(rng, -4.0, 4.0), on(rng, -4.0, 4.0)]).m
}

/// `r` turned by `a`, scaled by `k` and moved by `t`.
fn moved(r: [[f64; 2]; 3], a: f64, k: f64, t: [f64; 2]) -> [[f64; 2]; 3] {
    let (s, c) = a.sin_cos();
    r.map(|v| {
        [
            k * (c * v[0] - s * v[1]) + t[0],
            k * (s * v[0] + c * v[1]) + t[1],
        ]
    })
}

/// The masses the landmark and pole tests use: equal, a skewed set, extreme ones, and random draws.
fn mass_sets() -> Vec<[f64; 3]> {
    let mut rng = SplitMix::new(8);
    let mut sets = vec![
        [1.0 / 3.0; 3],
        [0.2, 0.3, 0.5],
        [0.6, 0.3, 0.1],
        [1e-4, 0.5, 0.4999],
        [0.4999, 1e-4, 0.5],
    ];
    sets.extend((0..32).map(|_| masses(&mut rng)));
    sets
}

/// The unit equilateral triangle, bodies 0 → 1 → 2 anticlockwise (`sign = 1`) or clockwise (`sign = −1`).
fn equilateral(sign: f64) -> [[f64; 2]; 3] {
    [[0.0, 0.0], [1.0, 0.0], [0.5, sign * 3f64.sqrt() / 2.0]]
}

type Map64 = fn([[f64; 2]; 3], [f64; 3]) -> [f64; 3];

/// `shape_vec` through the mass-weighted Jacobi vectors.
fn via_jacobi(r: [[f64; 2]; 3], m: [f64; 3]) -> [f64; 3] {
    let (rho_t, lambda_t) = weighted_jacobi(r, m);
    shape_vec(rho_t, lambda_t)
}

// ── shape_vec: the poles and the collision ─────────────────────────────────────────────────────────────────────────

/// `w` of the unit equilateral triangle, anticlockwise, under masses `m` (`Σmᵢ = 1`), in closed form from the pair
/// distances: `I = Σ_{i<j} mᵢmⱼ rᵢⱼ² = m₀m₁ + m₀m₂ + m₁m₂` and `ρ̃ ∧ λ̃ = √(m₀m₁m₂)·2·Area`, `2·Area = √3/2`, so
/// `w = √(3m₀m₁m₂)/(m₀m₁ + m₀m₂ + m₁m₂)`: 1 at equal masses, below 1 otherwise (RQ-355).
fn lagrange_w(m: [f64; 3]) -> f64 {
    (3.0 * m[0] * m[1] * m[2]).sqrt() / (m[0] * m[1] + m[0] * m[2] + m[1] * m[2])
}

/// `map` puts `L⁺` (anticlockwise) at `w = +1` and `L⁻` at `w = −1` within `16ε` at equal masses, and under every mass
/// set at `w = ±√(3m₀m₁m₂)/(m₀m₁ + m₀m₂ + m₁m₂)`, `L⁻` the mirror `(u, v, −w)` of `L⁺`; and the collision of 0 and 1
/// exactly at `(−1, 0, 0)`; each turned, scaled and moved. Returns the worst distance.
fn check_poles(map: Map64, lplus: [[f64; 2]; 3]) -> f64 {
    let mut rng = SplitMix::new(1);
    let mut worst: f64 = 0.0;
    for m in mass_sets() {
        for _ in 0..8 {
            let (a, k, t) = (
                on(&mut rng, -PI, PI),
                on(&mut rng, 0.01, 100.0),
                [on(&mut rng, -5.0, 5.0), on(&mut rng, -5.0, 5.0)],
            );
            let up = map(moved(lplus, a, k, t), m);
            let down = map(moved(equilateral(-1.0), a, k, t), m);
            let w = lagrange_w(m);
            let d = (up[2] - w)
                .abs()
                .max((down[2] + w).abs())
                .max(dist(up, [down[0], down[1], -down[2]]));
            assert!(
                d <= 16.0 * EPS,
                "L⁺ is not w = +{w}: L⁺ {up:?}, L⁻ {down:?} under masses {m:?}"
            );
            if m == [1.0 / 3.0; 3] {
                let pole = dist(up, [0.0, 0.0, 1.0]).max(dist(down, [0.0, 0.0, -1.0]));
                assert!(
                    pole <= 16.0 * EPS,
                    "L⁺ is not w = +1 at equal masses: L⁺ {up:?}, L⁻ {down:?}"
                );
            }
            worst = worst.max(d);
            let p = [on(&mut rng, -5.0, 5.0), on(&mut rng, -5.0, 5.0)];
            let collision = map(
                [p, p, [on(&mut rng, -5.0, 5.0), on(&mut rng, -5.0, 5.0)]],
                m,
            );
            assert_eq!(
                collision,
                [-1.0, 0.0, 0.0],
                "the collision of 0 and 1 is not (−1, 0, 0) under masses {m:?}"
            );
        }
    }
    worst
}

/// At f32: `L⁺` within `16ε_f32` of `(0, 0, 1)` and the collision of 0 and 1 exactly `(−1, 0, 0)`, through both maps.
fn check_poles_f32() {
    for m in mass_sets() {
        let m32 = m.map(|x| x as f32);
        let l = equilateral(1.0).map(|v| v.map(|x| x as f32));
        let c = [[0.25f32, -0.5], [0.25, -0.5], [1.5, 2.0]];
        let (rt, lt) = weighted_jacobi(l, m32);
        for n in [shape(l, m32), shape_vec(rt, lt)] {
            assert!(
                (f64::from(n[2]) - lagrange_w(m)).abs() <= 16.0 * f64::from(f32::EPSILON),
                "L⁺ is not w = +{} at f32: {n:?} under masses {m:?}",
                lagrange_w(m)
            );
        }
        let (rc, lc) = weighted_jacobi(c, m32);
        for n in [shape(c, m32), shape_vec(rc, lc)] {
            assert_eq!(n, [-1.0, 0.0, 0.0], "the f32 collision of 0 and 1 is {n:?}");
        }
    }
}

#[test]
fn shape_vec_poles() {
    let worst = check_poles(shape, equilateral(1.0)).max(check_poles(via_jacobi, equilateral(1.0)));
    check_poles_f32();
    eprintln!(
        "shape_vec_poles: worst pole distance {worst:.3e} ({:.1} ε)",
        worst / EPS
    );
}

negative_control!(
    shape_vec_poles,
    "the earlier negative-cross convention puts L⁺ at w = −1, so it must fail",
    expected = "L⁺ is not w = +1",
    {
        check_poles(
            |r, m| {
                let n = shape(r, m);
                [n[0], n[1], -n[2]]
            },
            equilateral(1.0),
        );
    }
);

// ── shape_vec: the IC Inspector's n ───────────────────────────────────────────────────────────────────────────────

/// One IC of the fixture's `raw` set.
struct RawIc {
    r: [[f64; 2]; 3],
    m: [f64; 3],
    n: [f64; 3],
    bc_azimuth: [f64; 3],
}

/// One IC of the fixture's `decoded` set.
struct DecodedIc {
    r: [[f64; 2]; 3],
    m: [f64; 3],
    n: [f64; 3],
}

fn bits(v: &serde_json::Value) -> f64 {
    let s = v.as_str().expect("a fixture number is a hex string");
    f64::from_bits(u64::from_str_radix(s, 16).expect("16 hex digits"))
}

fn three(v: &serde_json::Value) -> [f64; 3] {
    [bits(&v[0]), bits(&v[1]), bits(&v[2])]
}

fn positions(v: &serde_json::Value) -> [[f64; 2]; 3] {
    [0, 1, 2].map(|i| [bits(&v[i][0]), bits(&v[i][1])])
}

/// `fixtures/ic_inspector/shape_vec.json`, generated by its `gen_shape_vec.mjs` from the inspector's own script.
fn fixture() -> (Vec<RawIc>, Vec<DecodedIc>) {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/ic_inspector/shape_vec.json");
    let text = std::fs::read_to_string(&path).expect("read the inspector fixture");
    let json: serde_json::Value = serde_json::from_str(&text).expect("parse the inspector fixture");
    let raw = json["raw"]
        .as_array()
        .expect("raw")
        .iter()
        .map(|c| RawIc {
            r: positions(&c["r"]),
            m: three(&c["m"]),
            n: three(&c["n"]),
            bc_azimuth: three(&c["bc_azimuth"]),
        })
        .collect();
    let decoded = json["decoded"]
        .as_array()
        .expect("decoded")
        .iter()
        .map(|c| DecodedIc {
            r: positions(&c["r"]),
            m: three(&c["m"]),
            n: three(&c["n"]),
        })
        .collect();
    (raw, decoded)
}

/// The largest absolute component difference.
fn component(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| (a[i] - b[i]).abs()).fold(0.0, f64::max)
}

/// `map`'s `n` against the inspector's on every fixture IC, within `tol`: on the raw ICs against the canonical
/// representative, `w` compared in magnitude (the inspector mirrors `λ̃_y < −δ_λ`); on the decoded ones signed.
/// Returns the worst difference on each set.
fn check_inspector(map: Map64, tol: f64) -> (f64, f64) {
    let (raw, decoded) = fixture();
    assert_eq!(
        (raw.len(), decoded.len()),
        (500, 500),
        "the fixture's sizes"
    );
    let mut worst = (0.0f64, 0.0f64);
    for (k, c) in raw.iter().enumerate() {
        let n = map(c.r, c.m);
        let d = component([n[0], n[1], n[2].abs()], [c.n[0], c.n[1], c.n[2].abs()]);
        assert!(
            d <= tol,
            "raw IC {k}: n {n:?} differs from the IC Inspector's {:?} by {d:e}",
            c.n
        );
        worst.0 = worst.0.max(d);
    }
    for (k, c) in decoded.iter().enumerate() {
        let n = map(c.r, c.m);
        let d = component(n, c.n);
        assert!(
            d <= tol,
            "decoded IC {k}: n {n:?} differs from the IC Inspector's {:?} by {d:e}",
            c.n
        );
        worst.1 = worst.1.max(d);
    }
    worst
}

#[test]
fn shape_vec_inspector() {
    let a = check_inspector(shape, JS_TOL);
    let b = check_inspector(via_jacobi, JS_TOL);
    eprintln!(
        "shape_vec_inspector: worst |Δn| against the IC Inspector, raw / decoded: shape {:.3e} / {:.3e}, shape_vec \
         {:.3e} / {:.3e}; tolerance {JS_TOL:.3e} (shape_vec_js_tol, REQ-CHART-049, provisional until M2)",
        a.0, a.1, b.0, b.1
    );
}

negative_control!(
    shape_vec_inspector,
    "the negative-cross convention disagrees with the inspector's sign on its decoded ICs, so it must fail",
    expected = "differs from the IC Inspector's",
    {
        check_inspector(
            |r, m| {
                let n = shape(r, m);
                [n[0], n[1], -n[2]]
            },
            JS_TOL,
        );
    }
);

// ── The spherical map ───────────────────────────────────────────────────────────────────────────────────────────

type Spherical64 = fn(f64, f64) -> [f64; 3];

/// The azimuth of `n` on `[0, 2π)`.
fn azimuth(n: [f64; 3]) -> f64 {
    n[1].atan2(n[0]).rem_euclid(2.0 * PI)
}

/// The polar angle of `n` from `+w`.
fn polar(n: [f64; 3]) -> f64 {
    n[0].hypot(n[1]).atan2(n[2])
}

/// `map` puts `(s, 1)` exactly at `L⁺` and `(s, 0)` within `2ε` of `L⁻`; between, the polar angle is `π·(1 − t)` and
/// the azimuth `2π·s` within `8ε`, so φ runs over the whole `[0, π]` with no cap excised. Returns the worst.
fn check_spherical(map: Spherical64) -> f64 {
    let mut worst: f64 = 0.0;
    for k in 0..=64 {
        let s = f64::from(k) / 64.0;
        let top = map(s, 1.0);
        assert!(
            top == [0.0, 0.0, 1.0],
            "(s, t) = ({s}, 1) is not L⁺: {top:?}"
        );
        let bottom = map(s, 0.0);
        assert!(
            dist(bottom, [0.0, 0.0, -1.0]) <= 2.0 * EPS,
            "(s, t) = ({s}, 0) is not L⁻: {bottom:?}"
        );
        worst = worst.max(dist(bottom, [0.0, 0.0, -1.0]));
    }
    let mut rng = SplitMix::new(3);
    for _ in 0..4096 {
        let (s, t) = (on(&mut rng, 0.001, 0.999), on(&mut rng, 0.001, 0.999));
        let n = map(s, t);
        let dphi = (polar(n) - PI * (1.0 - t)).abs();
        let dtheta = (azimuth(n) - 2.0 * PI * s).abs();
        assert!(
            dphi <= 8.0 * EPS && dtheta <= 16.0 * EPS / polar(n).sin(),
            "(s, t) = ({s}, {t}): the polar angle {} is not π(1 − t) or the azimuth {} not 2πs",
            polar(n),
            azimuth(n)
        );
        assert!(
            (dist(n, [0.0; 3]) - 1.0).abs() <= 4.0 * EPS,
            "n {n:?} is not unit"
        );
        worst = worst.max(dphi);
    }
    worst
}

/// At f32: `(s, 1)` is `L⁺` exactly, `(s, 0)` within `2ε_f32` of `L⁻`.
fn check_spherical_f32() {
    let pi = std::f32::consts::PI;
    for k in 0..=64 {
        let s = k as f32 / 64.0;
        assert!(
            spherical(s, 1.0f32, pi) == [0.0, 0.0, 1.0],
            "f32 (s, 1) is not L⁺"
        );
        let bottom = widen(spherical(s, 0.0f32, pi));
        assert!(
            dist(bottom, [0.0, 0.0, -1.0]) <= 2.0 * f64::from(f32::EPSILON),
            "f32 (s, 0) is not L⁻: {bottom:?}"
        );
    }
}

#[test]
fn shape_sphere_chart_map_spherical() {
    let worst = check_spherical(|s, t| spherical(s, t, PI));
    check_spherical_f32();
    eprintln!("shape_sphere_chart_map_spherical: worst {worst:.3e}");
}

negative_control!(
    shape_sphere_chart_map_spherical,
    "R-12's rejected polar buffer, φ on [ε, π − ε], excises a cap at each pole, so it must fail",
    expected = "is not L⁺",
    {
        let buffered = |s: f64, t: f64| {
            let eps = 0.05;
            spherical(s, (eps + (1.0 - 2.0 * eps) * t) * 1.0, PI)
        };
        check_spherical(buffered);
    }
);

// ── The exponential map ─────────────────────────────────────────────────────────────────────────────────────────

type Exponential64 = fn([f64; 3], [f64; 3], [f64; 3], f64, f64, f64) -> [f64; 3];

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn unit(a: [f64; 3]) -> [f64; 3] {
    let k = dist(a, [0.0; 3]);
    a.map(|x| x / k)
}

/// A random orthonormal frame `(n₀, e₁, e₂)`.
fn frame(rng: &mut SplitMix) -> ([f64; 3], [f64; 3], [f64; 3]) {
    let n0 = unit([0, 1, 2].map(|_| on(rng, -1.0, 1.0)));
    let e1 = unit(cross(n0, [0, 1, 2].map(|_| on(rng, -1.0, 1.0))));
    (n0, e1, cross(n0, e1))
}

/// `map` at `(½, ½)` is `n₀` exactly; elsewhere its points are unit within `8ε` and at the geodesic distance `r` from
/// `n₀` within `64ε` (`r ≤ π`). Returns the worst.
fn check_exponential(map: Exponential64) -> f64 {
    let mut rng = SplitMix::new(4);
    let mut worst: f64 = 0.0;
    for _ in 0..256 {
        let (n0, e1, e2) = frame(&mut rng);
        let s = on(&mut rng, 0.01, 2.0);
        let centre = map(n0, e1, e2, s, 0.5, 0.5);
        assert!(
            centre == n0,
            "the exponential map at d = 0 is not n₀: {centre:?} for {n0:?}"
        );
        for _ in 0..16 {
            let (u, v) = (on(&mut rng, 0.0, 1.0), on(&mut rng, 0.0, 1.0));
            let n = map(n0, e1, e2, s, u, v);
            let r = s * (2.0 * u - 1.0).hypot(2.0 * v - 1.0);
            let geodesic = dist(n, n0).atan2(dist(n, n0.map(|x| -x))) * 2.0;
            assert!(
                (dist(n, [0.0; 3]) - 1.0).abs() <= 8.0 * EPS && (geodesic - r).abs() <= 64.0 * EPS,
                "the exponential map at ({u}, {v}) is {n:?}, at {geodesic} from n₀, not r = {r}"
            );
            worst = worst.max((geodesic - r).abs());
        }
    }
    worst
}

#[test]
fn shape_sphere_chart_map_exponential() {
    let worst = check_exponential(exponential);
    eprintln!("shape_sphere_chart_map_exponential: worst |geodesic − r| {worst:.3e}");
}

negative_control!(
    shape_sphere_chart_map_exponential,
    "sin(r)·d/‖d‖ as written divides 0 by 0 at d = 0, so the centre is NaN and must fail",
    expected = "is not n₀",
    {
        check_exponential(|n0, e1, e2, s, u, v| {
            let (a, b) = ((2.0 * u - 1.0) * s, (2.0 * v - 1.0) * s);
            let d = [0, 1, 2].map(|i| a * e1[i] + b * e2[i]);
            let r = dist(d, [0.0; 3]);
            [0, 1, 2].map(|i| r.cos() * n0[i] + r.sin() * d[i] / r)
        });
    }
);

// ── The inverse at its poles (R-437) ───────────────────────────────────────────────────────────────────────────────

type Inverse64 = fn([f64; 3], f64, f64) -> Inverse<f64>;

/// At `n₀ = ±1` with every sign of zero in `n₁` and `n₂`, `inv` gives `β = +0`, `ρ̃ = √a·(cos φ_f, sin φ_f)` and
/// `λ̃ = √b·(cos φ_f, sin φ_f)` bit for bit, so `ψ = φ_f`, and `shape_vec` of them is `(n₀, 0, 0)`. Off the pole, where
/// only one of `n₁` and `n₂` is zero, `β` is `atan2(n₂, n₁)`, the select taking the pole alone.
fn check_inverse_poles(inv: Inverse64) {
    let mut rng = SplitMix::new(5);
    for _ in 0..64 {
        let (i, phase) = (on(&mut rng, 0.25, 4.0), on(&mut rng, -PI, PI));
        for n in [
            [0.0, 0.0, 1.0],
            [0.0, 0.0, -1.0],
            [0.6, 0.0, 0.8],
            [-0.6, -0.0, -0.8],
            [0.6, 0.8, 0.0],
            [0.6, -0.8, -0.0],
            [0.0, -1.0, 0.0],
        ] {
            let got = inv(n, i, phase);
            assert_eq!(got.beta, n[2].atan2(n[1]), "β off the pole at {n:?}");
        }
        for n0 in [1.0, -1.0] {
            for n1 in [0.0, -0.0] {
                for n2 in [0.0, -0.0] {
                    let got = inv([n0, n1, n2], i, phase);
                    let (a, b) = (i * (1.0 + n0) / 2.0, i * (1.0 - n0) / 2.0);
                    let along = |x: f64| [x.sqrt() * phase.cos(), x.sqrt() * phase.sin()];
                    assert!(
                        got.beta.to_bits() == 0,
                        "β at the pole ({n0}, {n1:?}, {n2:?}) is {:?}, not R-437's +0",
                        got.beta
                    );
                    assert!(
                        got.rho_t.map(f64::to_bits) == along(a).map(f64::to_bits)
                            && got.lambda_t.map(f64::to_bits) == along(b).map(f64::to_bits),
                        "ψ at the pole ({n0}, {n1:?}, {n2:?}) is not φ_f: ρ̃ {:?}, λ̃ {:?}",
                        got.rho_t,
                        got.lambda_t
                    );
                    assert_eq!(
                        shape_vec(got.rho_t, got.lambda_t),
                        [n0, 0.0, 0.0],
                        "the pole's round trip"
                    );
                }
            }
        }
    }
}

#[test]
fn shape_sphere_inverse_poles() {
    check_inverse_poles(inverse);
    // An n off the unit sphere by one rounding, past either pole: the smaller of a and b is from n₁² + n₂², so no root
    // of a negative and no NaN.
    for n0 in [1.0 + EPS, -1.0 - EPS] {
        let got = inverse([n0, 0.0, 0.0], 1.0, 0.3);
        let all = [got.rho_t, got.lambda_t, [got.alpha, got.beta]];
        assert!(
            all.iter().flatten().all(|x| x.is_finite()),
            "n₀ = {n0} gives a non-finite inverse: {got:?}"
        );
        let zero = if n0 > 0.0 { got.lambda_t } else { got.rho_t };
        assert_eq!(zero, [0.0, 0.0], "n₀ = {n0}: the vanishing vector");
    }
}

negative_control!(
    shape_sphere_inverse_poles,
    "atan2(q, p) unselected gives atan2(±0, −0) = ±π at the pole, so β is not +0 and it must fail",
    expected = "not R-437's +0",
    {
        check_inverse_poles(|n, i, phase| {
            let beta = (i * n[2] / 2.0).atan2(i * n[1] / 2.0);
            let (a, b) = (i * (1.0 + n[0]) / 2.0, i * (1.0 - n[0]) / 2.0);
            let psi = phase + beta;
            Inverse {
                rho_t: [a.sqrt() * phase.cos(), a.sqrt() * phase.sin()],
                lambda_t: [b.sqrt() * psi.cos(), b.sqrt() * psi.sin()],
                alpha: b.sqrt().atan2(a.sqrt()),
                beta,
            }
        });
    }
);

// ── The round trip, Φ alone ─────────────────────────────────────────────────────────────────────────────────────────

type Place64 = fn([f64; 3], f64, f64, &mass::Masses<f64>) -> kernel::decode::config::Config<f64>;

/// `shape(Φ(n)) = n` and `shape_vec(ρ̃, λ̃) = n` within `bound` over random masses, spherical points on both
/// hemispheres, `I` and `φ_f`, `place` being `Φ`'s configuration. Returns the worst `‖Δn‖`.
fn check_roundtrip(place: Place64, bound: f64) -> f64 {
    let mut rng = SplitMix::new(6);
    let mut worst: f64 = 0.0;
    let mut lower = 0;
    for _ in 0..4096 {
        let m = mass::decode::<Defaults, f64>([on(&mut rng, -4.0, 4.0), on(&mut rng, -4.0, 4.0)]);
        let n = spherical(on(&mut rng, 0.0, 1.0), on(&mut rng, 0.0, 1.0), PI);
        let (i, phase) = (on(&mut rng, 0.25, 4.0), on(&mut rng, -PI, PI));
        let c = place(n, i, phase, &m);
        let d = dist(shape(c.r, m.m), n).max(dist(shape_vec(c.rho_t, c.lambda_t), n));
        assert!(
            d <= bound,
            "the round trip at n = {n:?} misses by {d:e} (masses {:?}, I {i}, φ_f {phase})",
            m.m
        );
        lower += usize::from(n[2] < 0.0);
        worst = worst.max(d);
    }
    assert!(
        lower > 1500 && lower < 2600,
        "{lower} of 4096 points on the lower hemisphere"
    );
    worst
}

#[test]
fn shape_sphere_roundtrip() {
    let worst = check_roundtrip(place, ROUNDTRIP);
    eprintln!(
        "shape_sphere_roundtrip: worst ‖shape(Φ(n)) − n‖ {worst:.3e} over 4096 points; bound {ROUNDTRIP:e} \
         (shape_roundtrip_bound, REQ-CHART-053, provisional until M2)"
    );
}

negative_control!(
    shape_sphere_roundtrip,
    "the earlier inverse, ψ = φ_f + atan2(−q, p), misses every off-equator point, so it must fail",
    expected = "the round trip at",
    {
        check_roundtrip(
            |n, i, phase, m| place([n[0], n[1], -n[2]], i, phase, m),
            ROUNDTRIP,
        );
    }
);

/// `shape_vec(inverse(n)) = n` within `bound` about `n₀ = ±1` at polar radii `10⁻²` to `10⁻¹⁵`, where `n₀ = cos r`
/// rounds to `±1` from `r ≈ 10⁻⁸` on and `1 − |n₀|` loses `r`. Returns the worst.
fn check_near_poles(inv: Inverse64, bound: f64) -> f64 {
    let mut rng = SplitMix::new(11);
    let mut worst: f64 = 0.0;
    for sign in [1.0, -1.0] {
        for _ in 0..4096 {
            let r = 10f64.powf(-on(&mut rng, 2.0, 15.0));
            let chi = on(&mut rng, -PI, PI);
            let n = [sign * r.cos(), r.sin() * chi.cos(), r.sin() * chi.sin()];
            let got = inv(n, on(&mut rng, 0.25, 4.0), on(&mut rng, -PI, PI));
            let d = dist(shape_vec(got.rho_t, got.lambda_t), n);
            assert!(
                d <= bound,
                "near the pole, at polar radius {r:e}, the round trip misses by {d:e}"
            );
            worst = worst.max(d);
        }
    }
    worst
}

#[test]
fn shape_sphere_roundtrip_near_poles() {
    let worst = check_near_poles(inverse, ROUNDTRIP);
    eprintln!("shape_sphere_roundtrip_near_poles: worst {worst:.3e}");
}

negative_control!(
    shape_sphere_roundtrip_near_poles,
    "§3.2 evaluated as written, b = I(1 − n₀)/2, loses the polar radius once n₀ rounds to 1, so it must fail",
    expected = "near the pole",
    {
        check_near_poles(
            |n, i, phase| {
                let (a, b) = (i * (1.0 + n[0]) / 2.0, (i * (1.0 - n[0]) / 2.0).max(0.0));
                let (p, q) = (i * n[1] / 2.0, i * n[2] / 2.0);
                let beta = if p == 0.0 && q == 0.0 { 0.0 } else { q.atan2(p) };
                let psi = phase + beta;
                Inverse {
                    rho_t: [a.sqrt() * phase.cos(), a.sqrt() * phase.sin()],
                    lambda_t: [b.sqrt() * psi.cos(), b.sqrt() * psi.sin()],
                    alpha: b.sqrt().atan2(a.sqrt()),
                    beta,
                }
            },
            ROUNDTRIP,
        );
    }
);

// ── Φ then C, its step 3 not taken ──────────────────────────────────────────────────────────────────────────────────────────────

type Decode64 = fn(&ShapeSphere<f64>, f64, f64) -> Decoded<f64>;

/// A spherical chart with random held blocks, `I` and `φ_f`.
fn random_sphere(rng: &mut SplitMix) -> ShapeSphere<f64> {
    ShapeSphere {
        map: ShapeMap::Spherical,
        n0: [0.0, 0.0, 1.0],
        e1: [1.0, 0.0, 0.0],
        e2: [0.0, 1.0, 0.0],
        scale: 1.0,
        inertia: on(rng, 0.25, 4.0),
        phase: on(rng, -PI, PI),
        z_mass: [on(rng, -3.0, 3.0), on(rng, -3.0, 3.0)],
        z_mom: [0, 1, 2, 3].map(|_| on(rng, -2.0, 2.0)),
    }
}

/// The largest component difference of two sets of three planar vectors, relative to the larger's magnitude (or 1).
fn relative(a: [[f64; 2]; 3], b: [[f64; 2]; 3]) -> f64 {
    let flat = |x: [[f64; 2]; 3]| x.into_iter().flatten().collect::<Vec<f64>>();
    let (a, b) = (flat(a), flat(b));
    let scale = a.iter().chain(&b).fold(1.0f64, |k, x| k.max(x.abs()));
    a.iter()
        .zip(&b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max)
        / scale
}

/// `Φ` alone at `(s, t)`, without `C` (R-178): `D_mass`, [`place`] and [`held_momentum`] joined.
fn phi(chart: &ShapeSphere<f64>, s: f64, t: f64) -> Ic<f64> {
    let m = mass::decode::<Defaults, f64>(chart.z_mass);
    let c = place(chart.point(s, t), chart.inertia, chart.phase, &m);
    let p = held_momentum::<Defaults, f64>(chart.z_mom, &m, &c, chart.inertia);
    Ic {
        m: m.m,
        r: c.r,
        p: p.p,
    }
}

type Phi64 = fn(&ShapeSphere<f64>, f64, f64) -> Ic<f64>;

/// `C(Φ(chart, s, t))` with step 3 not taken, `Φ` given by `phi`: its shape is `n(s, t)` with its sign on both
/// hemispheres, a lower-hemisphere point at `n₂ < 0`, within `tol`; its masses are `m_fixed` exactly and its momenta
/// `p_fixed` within `tol` relative; nothing is mirrored. Returns the worst.
fn check_form(phi: Phi64, tol: f64) -> f64 {
    let mut rng = SplitMix::new(7);
    let mut worst: f64 = 0.0;
    let mut lower = 0;
    for _ in 0..512 {
        let chart = random_sphere(&mut rng);
        let fixed_m = mass::decode::<Defaults, f64>(chart.z_mass);
        let fixed_p = momentum::decode::<Defaults, f64>(chart.z_mom, &fixed_m).p;
        let (s, t) = (on(&mut rng, 0.0, 1.0), on(&mut rng, 0.02, 0.98));
        let n = chart.point(s, t);
        let c = canonicalise_signed(&phi(&chart, s, t));
        let got = shape(c.ic.r, c.ic.m);
        assert!(
            dist(got, n) <= tol,
            "C(Φ({s}, {t})) with step 3 not taken is at {got:?}, not n {n:?}"
        );
        assert_eq!(c.ic.m, fixed_m.m, "C(Φ)'s masses are not m_fixed");
        let dp = relative(c.ic.p, fixed_p);
        assert!(
            dp <= tol,
            "C(Φ({s}, {t}))'s momenta {:?} are not p_fixed {fixed_p:?}",
            c.ic.p
        );
        assert!(!c.mirrored, "C with step 3 not taken mirrored at n {n:?}");
        lower += usize::from(n[2] < 0.0);
        worst = worst.max(dist(got, n)).max(dp);
    }
    assert!(lower > 150, "{lower} of 512 points on the lower hemisphere");
    worst
}

/// The tolerance of `Φ` then `C`: `Φ`'s turn and scale and `C`'s undoing them, relative.
const FORM_TOL: f64 = 64.0 * EPS;

#[test]
fn shape_sphere_canonical_form() {
    let worst = check_form(phi, FORM_TOL);
    eprintln!(
        "shape_sphere_canonical_form: worst {worst:.3e} ({:.1} ε)",
        worst / EPS
    );
}

negative_control!(
    shape_sphere_canonical_form,
    "momenta held in Φ's frame unturned are turned by C's −φ_f, so C(Φ)'s momenta are not p_fixed and it must fail",
    expected = "are not p_fixed",
    {
        check_form(
            |c, s, t| {
                let ic = phi(c, s, t);
                let m = mass::decode::<Defaults, f64>(c.z_mass);
                Ic {
                    p: momentum::decode::<Defaults, f64>(c.z_mom, &m).p,
                    ..ic
                }
            },
            FORM_TOL,
        );
    }
);

// ── The shared decoder ──────────────────────────────────────────────────────────────────────────────────────────────

/// `decode` on random spherical and exponential charts, under both link sets: its masses are `D_mass`'s on `z₀`'s
/// mass block, its configuration [`place`]'s at the chart's point, its momenta [`held_momentum`]'s, its `ic` `C` of
/// their join with step 3 not taken (R-544) and its descriptor that `ic`'s, `Σpᵢ = 0` and the CoM at the origin within `16ε`; `map`'s controls are the held blocks; `validate` is `Valid`
/// inside and past `[0,1]²`; the name is `ShapeSphere`.
fn check_shared(decode: Decode64) {
    let mut rng = SplitMix::new(9);
    for k in 0..256 {
        let mut chart = random_sphere(&mut rng);
        if k % 2 == 1 {
            let (n0, e1, e2) = frame(&mut rng);
            chart = ShapeSphere {
                map: ShapeMap::Exponential,
                n0,
                e1,
                e2,
                scale: on(&mut rng, 0.1, 1.5),
                ..chart
            };
        }
        let (u, v) = (on(&mut rng, 0.0, 1.0), on(&mut rng, 0.0, 1.0));
        let d = decode(&chart, u, v);
        let m = mass::decode::<Defaults, f64>(chart.z_mass);
        let c = place(chart.point(u, v), chart.inertia, chart.phase, &m);
        let p = held_momentum::<Defaults, f64>(chart.z_mom, &m, &c, chart.inertia);
        assert!(
            d.masses == m && d.config == c && d.momentum == p,
            "the chart's decode is not the shared blocks with the configuration place's at ({u}, {v})"
        );
        let joined = Ic {
            m: m.m,
            r: c.r,
            p: p.p,
        };
        assert_eq!(
            d.ic,
            canonicalise_signed(&joined).ic,
            "the ic is not C of the joined blocks with step 3 not taken"
        );
        assert_eq!(
            d.descriptor,
            descriptor::describe_decoded(&d.ic, &d.config),
            "the descriptor is not the canonical ic's, its rho_angle Φ's (R-628)"
        );
        let total = [0, 1].map(|j| d.ic.p.iter().map(|q| q[j]).sum::<f64>());
        let com = [0, 1].map(|j| (0..3).map(|i| d.ic.m[i] * d.ic.r[i][j]).sum::<f64>());
        assert!(
            total.iter().chain(&com).all(|x| x.abs() <= 16.0 * EPS),
            "Σp {total:?}, CoM {com:?}"
        );
        assert_eq!(
            chart.map(u, v).z.z,
            [
                0.0,
                0.0,
                chart.z_mom[0],
                chart.z_mom[1],
                chart.z_mom[2],
                chart.z_mom[3],
                chart.z_mass[0],
                chart.z_mass[1]
            ],
            "map's controls"
        );
        for (a, b) in [(u, v), (-0.5, 1.5), (2.0, -3.0)] {
            assert_eq!(
                chart.validate(a, b),
                ValidationResult::Valid,
                "validate at ({a}, {b})"
            );
        }
        assert_eq!(chart.name(), "ShapeSphere");
        assert!(
            !chart.forbids_energy_normalisation(),
            "the shape sphere forbids no E*"
        );
    }
    // The free entry dispatches to the chart's own decode, under each link set.
    let chart = random_sphere(&mut rng);
    assert_eq!(
        chart::decode::<StickBreakingLinks, f64, _>(&chart, 0.3, 0.7),
        chart.decode::<StickBreakingLinks, f64>(0.3, 0.7),
        "chart::decode on the shape sphere"
    );
    assert_ne!(
        chart.decode::<StickBreakingLinks, f64>(0.3, 0.7).masses,
        chart.decode::<Defaults, f64>(0.3, 0.7).masses,
        "the mass block reads the selection's links"
    );
}

#[test]
fn shape_sphere_shared_decoder() {
    check_shared(|c, u, v| c.decode::<Defaults, f64>(u, v));
    check_shared(chart::decode::<Defaults, f64, _>);
}

negative_control!(
    shape_sphere_shared_decoder,
    "decoding the chart's latent controls through D_cfg ignores the shape, so it must fail",
    expected = "is not the shared blocks",
    {
        check_shared(|c, u, v| kernel::decode::decode::<Defaults, f64>(&c.map(u, v).z));
    }
);

// ── The decode is canonical (dd_decoder §2, §3.6) ──────────────────────────────────────────────────────────────────

/// A random chart of either map, its `I` and `φ_f` drawn.
fn random_chart(rng: &mut SplitMix, exponential: bool) -> ShapeSphere<f64> {
    let chart = random_sphere(rng);
    if !exponential {
        return chart;
    }
    let (n0, e1, e2) = frame(rng);
    ShapeSphere {
        map: ShapeMap::Exponential,
        n0,
        e1,
        e2,
        scale: on(rng, 0.1, 1.5),
        ..chart
    }
}

/// `decode` on random charts of both maps, `I` in `[¼, 4]`, both hemispheres drawn: its `ic` is `C(Φ)` with step 3
/// not taken (R-544), at `I = 1` within `64ε`, and its descriptor is that state's, with `rho_mag` and `lambda_mag` in
/// `[0, 1]`.
fn check_canonical(decode: Decode64) {
    let mut rng = SplitMix::new(0x0206);
    let (mut lower, mut scaled) = (0, 0);
    for k in 0..1024 {
        let chart = random_chart(&mut rng, k % 2 == 1);
        let (u, v) = (on(&mut rng, 0.0, 1.0), on(&mut rng, 0.0, 1.0));
        let d = decode(&chart, u, v);
        let i = canon::inertia(d.ic.m, d.ic.r);
        assert!(
            (i - 1.0).abs() <= 64.0 * EPS,
            "the decode's ic at ({u}, {v}) is not on I = 1: I = {i} (chart I {})",
            chart.inertia
        );
        let want = canonicalise_signed(&phi(&chart, u, v)).ic;
        assert_eq!(
            d.ic, want,
            "the decode's ic is not C(Φ) with step 3 not taken at ({u}, {v})"
        );
        assert_eq!(
            d.descriptor,
            descriptor::describe_decoded(&want, &d.config),
            "the descriptor is not C(Φ)'s, its rho_angle Φ's (R-628)"
        );
        let q = &d.descriptor;
        assert!(
            (0.0..=1.0).contains(&q.rho_mag) && (0.0..=1.0).contains(&q.lambda_mag),
            "rho_mag {} or lambda_mag {} outside [0, 1]",
            q.rho_mag,
            q.lambda_mag
        );
        lower += usize::from(chart.point(u, v)[2] < 0.0);
        scaled += usize::from((chart.inertia - 1.0).abs() > 0.25);
    }
    assert!(
        lower > 200 && scaled > 500,
        "{lower} lower-hemisphere draws, {scaled} at I far from 1"
    );
}

#[test]
fn shape_sphere_decode_canonical() {
    check_canonical(|c, u, v| c.decode::<Defaults, f64>(u, v));
    check_canonical(chart::decode::<Defaults, f64, _>);
}

negative_control!(
    shape_sphere_decode_canonical,
    "Φ's state handed over without C, at I ≠ 1, is off the section, so it must fail",
    expected = "is not on I = 1",
    {
        check_canonical(|c, u, v| {
            let ic = phi(c, u, v);
            Decoded {
                ic,
                descriptor: descriptor::describe(&ic),
                ..c.decode::<Defaults, f64>(u, v)
            }
        });
    }
);

type DecodeChart64 = fn(&ShapeSphere<f64>, f64, f64, Option<f64>) -> (Decoded<f64>, Outcome);

/// `η_E`'s entry on the shape sphere: with no target, the decode is the chart's own, its configuration [`place`]'s at
/// `n(u, v)`, the outcome `Off`; with `E* = V₀ + ½`, the configuration is the same, the positions unchanged, the
/// outcome `Scaled` and the energy `E*` within `64ε` relative, the descriptor rewritten from the result.
fn check_energy_norm(decode: DecodeChart64) {
    let mut rng = SplitMix::new(0x0e5a);
    for k in 0..512 {
        let chart = random_chart(&mut rng, k % 2 == 1);
        let (u, v) = (on(&mut rng, 0.0, 1.0), on(&mut rng, 0.0, 1.0));
        let m = mass::decode::<Defaults, f64>(chart.z_mass);
        let at = place(chart.point(u, v), chart.inertia, chart.phase, &m);
        let own = chart.decode::<Defaults, f64>(u, v);
        let (off, outcome) = decode(&chart, u, v, None);
        assert!(
            off.config == at,
            "decode_chart's configuration at ({u}, {v}) is not place's at n(u, v)"
        );
        assert_eq!(
            (off, outcome),
            (own, Outcome::Off),
            "decode_chart with no target"
        );
        let e_star = own.descriptor.V_0 + 0.5;
        let (on_, outcome) = decode(&chart, u, v, Some(e_star));
        assert!(
            on_.config == at,
            "decode_chart's configuration under E* at ({u}, {v}) is not place's at n(u, v)"
        );
        assert_eq!(outcome, Outcome::Scaled, "the outcome under E* = V₀ + ½");
        assert_eq!(on_.ic.r, own.ic.r, "η_E moved the positions");
        let q = &on_.descriptor;
        assert_eq!(
            *q,
            descriptor::describe_decoded(&on_.ic, &on_.config),
            "the descriptor is not the result's, its rho_angle Φ's (R-628)"
        );
        let e = q.K_0 + q.V_0;
        assert!(
            (e - e_star).abs() <= 64.0 * EPS * e_star.abs().max(1.0),
            "the energy under E* = {e_star} is {e}"
        );
    }
}

#[test]
fn shape_sphere_energy_norm() {
    check_energy_norm(energy_norm::decode_chart::<Defaults, f64, _>);
}

negative_control!(
    shape_sphere_energy_norm,
    "η_E's entry decoding map's latent through D_cfg ignores the shape, so it must fail",
    expected = "is not place's at n(u, v)",
    {
        check_energy_norm(|c, u, v, e| {
            energy_norm::decode_with(
                &mut kernel::decode::Free::<Defaults>::new(),
                &c.map(u, v).z,
                e,
                false,
            )
        });
    }
);

// ── The signed decode (R-544; REQ-CHART-081) ──────────────────────────────────────────────────────────────────────

/// A decode of the chart at `(u, v)` to the state that crosses to the integrator: the chart's own, or a control's.
type DecodeIc = fn(&ShapeSphere<f64>, f64, f64) -> Ic<f64>;

/// The chart's own decode's `ic`.
fn decode_ic(chart: &ShapeSphere<f64>, u: f64, v: f64) -> Ic<f64> {
    chart.decode::<Defaults, f64>(u, v).ic
}

/// The Jacobi momenta `(p_ρ, p_λ)` of a state, §0.3's map inverted: `p_λ = p₂`, `p_ρ = p₁ + (m₁/M₀₁)p₂`.
fn jacobi_momenta(x: &Ic<f64>) -> [[f64; 2]; 2] {
    let w = x.m[1] / (x.m[0] + x.m[1]);
    let [_, p1, p2] = x.p;
    [[p1[0] + w * p2[0], p1[1] + w * p2[1]], p2]
}

/// The largest component difference of two Jacobi momenta, relative to the larger's magnitude (or 1).
fn jacobi_gap(a: [[f64; 2]; 2], b: [[f64; 2]; 2]) -> f64 {
    let pad = |j: [[f64; 2]; 2]| [j[0], j[1], [0.0; 2]];
    relative(pad(a), pad(b))
}

/// The tolerance of the signed decode's identities: `Φ`'s turn and scale and `C`'s undoing them, relative, as
/// [`FORM_TOL`].
const SIGNED_TOL: f64 = 64.0 * EPS;

/// The pixels the signed decode visits on `chart`'s draws: uniform on `[0,1]²`, and every fourth on a pole (`t = 0`,
/// `t = 1`) or the equator (`t = ½`).
fn signed_pixel(rng: &mut SplitMix, k: usize) -> (f64, f64) {
    let s = on(rng, 0.0, 1.0);
    let t = on(rng, 0.0, 1.0);
    match k % 12 {
        0 => (s, 0.0),
        4 => (s, 0.5),
        8 => (s, 1.0),
        _ => (s, t),
    }
}

/// `decode` over random masses, held momenta, `I` and `φ_f`, at pixels of both hemispheres, the poles and the equator
/// included (R-544; REQ-CHART-081): its shape is `n(θ, φ)` with its sign, within `shape_roundtrip_bound`; it is
/// canonical in its CoM, scale, boost and rotation, `C` with its step 3 not taken leaving it unchanged within
/// [`SIGNED_TOL`] and `I = 1`; and its held Jacobi momenta are `p_fixed` at `(θ, φ)` and at `(θ, π − φ)`, within
/// [`SIGNED_TOL`] relative. Returns the worst shape distance.
fn check_signed_decode(decode: DecodeIc) -> f64 {
    let mut rng = SplitMix::new(0x0544);
    let (mut worst, mut lower, mut equator): (f64, u32, u32) = (0.0, 0, 0);
    for k in 0..2048 {
        let chart = random_sphere(&mut rng);
        let (s, t) = signed_pixel(&mut rng, k);
        let x = decode(&chart, s, t);
        let n = chart.point(s, t);
        let got = shape(x.r, x.m);
        assert!(
            dist(got, n) <= ROUNDTRIP,
            "the decoded state at ({s}, {t}) is at {got:?}, not the signed n {n:?}"
        );
        worst = worst.max(dist(got, n));
        let again = canonicalise_signed(&x);
        let off = relative(again.ic.r, x.r).max(relative(again.ic.p, x.p));
        let i = canon::inertia(x.m, x.r);
        assert!(
            off <= SIGNED_TOL && (i - 1.0).abs() <= SIGNED_TOL,
            "the decoded state at ({s}, {t}) is not canonical: C with step 3 not taken moves it by {off:e}, I − 1 = \
             {:e}",
            i - 1.0
        );
        let m = mass::decode::<Defaults, f64>(chart.z_mass);
        let held = momentum::decode::<Defaults, f64>(chart.z_mom, &m);
        let fixed = [held.p_rho, held.p_lambda];
        let twin = decode(&chart, s, 1.0 - t);
        let (here, there) = (jacobi_momenta(&x), jacobi_momenta(&twin));
        let gap = jacobi_gap(here, fixed).max(jacobi_gap(there, fixed));
        assert!(
            gap <= SIGNED_TOL,
            "the held Jacobi momenta at ({s}, {t}) and ({s}, {}) are {here:?} and {there:?}, not p_fixed {fixed:?}",
            1.0 - t
        );
        lower += u32::from(n[2] < 0.0);
        equator += u32::from(t == 0.5);
    }
    assert!(
        lower > 600 && equator > 100,
        "the draws miss a regime: {lower} lower-hemisphere, {equator} on the equator"
    );
    worst
}

/// The bits of a state's masses, positions and momenta, widened to f64 exactly.
type StateBits = ([u64; 3], [[u64; 2]; 3], [[u64; 2]; 3]);

fn state_bits<R: Precision>(c: &Ic<R>) -> StateBits {
    let b = |x: R| Into::<f64>::into(x).to_bits();
    let v = |a: [R; 2]| [b(a[0]), b(a[1])];
    (c.m.map(b), c.r.map(v), c.p.map(v))
}

/// `C` on the state `x` of the chart `key`: `canonicalise` is `C`'s steps with step 3 taken, bit for bit
/// [`canonicalise_signed`] then `Π` where R-82's decision ([`canon::mirror_decision`]) mirrors, every other field equal.
fn check_c_unchanged<R: Precision>(key: &str, x: &Ic<R>) {
    let full = canonicalise(x);
    let signed = canonicalise_signed(x);
    let flip = canon::mirror_decision(x.m, x.r);
    assert!(
        state_bits(&full.ic) == state_bits(&canon::reflect(&signed.ic, flip))
            && full.mirrored == flip
            && !signed.mirrored
            && (
                full.i_in,
                full.lambda,
                full.phi,
                full.lambda_t_y,
                full.clamped
            ) == (
                signed.i_in,
                signed.lambda,
                signed.phi,
                signed.lambda_t_y,
                signed.clamped
            ),
        "{key}: C on its decoded state is not its steps with step 3 taken, bit for bit"
    );
}

/// `C` on every other registered chart's decoded states, over a `9 × 9` grid at f64 and f32, each state with its
/// mirror and, at f64, a turned, scaled, moved and boosted copy ([`check_c_unchanged`]). These charts' decodes run no
/// `C`, so `C` with step 3 taken is the one piece of theirs this task's change reaches (R-544: "On every other chart
/// `C` is unchanged"). Returns the charts checked.
fn check_other_charts() -> usize {
    let gauge = Gauge {
        a: [0.3, -1.2],
        u: [0.4, 0.2],
        theta: 2.1,
        s: 3.0,
        mirror: false,
    };
    let mut checked = 0;
    for chart in registry() {
        if chart.type_name() == "ShapeSphere" {
            continue;
        }
        for i in 0..9 {
            for j in 0..9 {
                let (u, v) = (f64::from(i) / 8.0, f64::from(j) / 8.0);
                let x = <f64 as DecodeAt>::decode_at(chart.as_ref(), u, v).ic;
                for y in [x, canon::reflect(&x, true), gauge.act(&x)] {
                    check_c_unchanged(chart.key(), &y);
                }
                let x32 = <f32 as DecodeAt>::decode_at(chart.as_ref(), u, v).ic;
                check_c_unchanged(chart.key(), &x32);
                check_c_unchanged(chart.key(), &canon::reflect(&x32, true));
            }
        }
        checked += 1;
    }
    checked
}

#[test]
fn shape_sphere_signed_decode() {
    let worst = check_signed_decode(decode_ic);
    let charts = check_other_charts();
    eprintln!(
        "shape_sphere_signed_decode: worst ‖shape(decode) − n‖ {worst:.3e} over 2048 pixels of both hemispheres; C on \
         {charts} other registered charts' decodes is its steps with step 3 taken, bit for bit"
    );
}

negative_control!(
    shape_sphere_signed_decode,
    "R-141's gauge left in, C's mirror step run on Φ, lands every lower pixel at (θ, π − φ), so it must fail",
    expected = "not the signed n",
    {
        check_signed_decode(|c, u, v| canonicalise(&phi(c, u, v)).ic);
    }
);

/// `Φ` with `p_fixed` mirrored below the equator, R-141's held momentum, its `y`-controls negated where `n₂ < 0`, then
/// `C` with step 3 not taken: a control's decode.
#[cfg(feature = "controls")]
fn mirrored_hold(chart: &ShapeSphere<f64>, u: f64, v: f64) -> Ic<f64> {
    let below = chart.point(u, v)[2] < 0.0;
    let z = chart.z_mom;
    let held = ShapeSphere {
        z_mom: if below { [z[0], -z[1], z[2], -z[3]] } else { z },
        ..*chart
    };
    canonicalise_signed(&phi(&held, u, v)).ic
}

mod signed_decode_mirrored_hold {
    use super::*;
    negative_control!(
        shape_sphere_signed_decode,
        "a held momentum mirrored below the equator differs from the upper pixel's, so it must fail",
        expected = "held Jacobi momenta",
        {
            check_signed_decode(mirrored_hold);
        }
    );
}

mod signed_decode_skipping_c {
    use super::*;
    negative_control!(
        shape_sphere_signed_decode,
        "a decode skipping C hands over Φ's state at I ≠ 1, off the section, so it must fail",
        expected = "is not canonical",
        {
            check_signed_decode(phi);
        }
    );
}

// ── The mirror test (R-544 item 5; REQ-CHART-082) ─────────────────────────────────────────────────────────────────

/// Which case the mirror test is in: the coordinates the chart holds fixed are all their own mirror images, or not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MirrorCase {
    /// The masses, always, and `p_fixed` with its `y`-components zero (released from rest included): the pixels
    /// `(θ, φ)` and `(θ, π − φ)` are mirror twins.
    Equivariant,
    /// `p_fixed` has a `y`-component: the two hemispheres are different systems, and the test asserts no symmetry.
    DoesNotApply,
}

/// The case `chart` is in, from what it holds fixed (R-544 item 5): its masses are mirror-invariant, so the held
/// Jacobi momenta decide, mirror-equivariant exactly where `p_ρ,y = p_λ,y = 0`.
fn classify(chart: &ShapeSphere<f64>) -> MirrorCase {
    let m = mass::decode::<Defaults, f64>(chart.z_mass);
    let held = momentum::decode::<Defaults, f64>(chart.z_mom, &m);
    if held.p_rho[1] == 0.0 && held.p_lambda[1] == 0.0 {
        MirrorCase::Equivariant
    } else {
        MirrorCase::DoesNotApply
    }
}

/// `Π`, the mirror applied to a state: the kernel's, `y → −y` on every `rᵢ` and every `pᵢ`, or a control's.
type MirrorFn = fn(&Ic<f64>) -> Ic<f64>;

/// The kernel's `Π` on the full state.
fn full_mirror(x: &Ic<f64>) -> Ic<f64> {
    canon::reflect(x, true)
}

/// `L_z = Σ rᵢ × pᵢ`.
fn lz(x: &Ic<f64>) -> f64 {
    (0..3)
        .map(|i| x.r[i][0] * x.p[i][1] - x.r[i][1] * x.p[i][0])
        .sum()
}

/// The pixels' grid: `GRID × GRID` points `(i, j)/(GRID − 1)`. Odd, so `(½, ½)`, `BC₀₁`'s pixel, its own pair, is on it.
const GRID: u32 = 41;

/// The state comparison's distance from each binary-collision landmark, chordal on the sphere. Near a landmark `λ̃`'s
/// direction is conditioned as `ulp/‖ρ̃‖` (chart_reference §3.2), so a rounding-level difference in `n̂` turns the state
/// by up to about `2·ulp/δ` (the 2 near `BC₀₁`; physics review 5640193616 of PR #288); at `δ = 1/16` that is
/// `32 ulp ≤ ε_phys = 64 ulp` (REQ-ENC-024), the reason for `δ`.
const DELTA: f64 = 1.0 / 16.0;

/// The mirror test on `chart` (R-544 item 5; REQ-CHART-082), its decode `decode` and its `Π` `pi`, the state comparison
/// kept `delta` from each landmark: it classifies the chart's held coordinates and says which case it is in. On every
/// decoded state of the grid `Π·x` has every `rᵢ` and `pᵢ` reflected through the `x`-axis, bit for bit, its `L_z`
/// `−L_z(x)` and its `shape` `x`'s with `w` negated, bitwise where nonzero and by value at zero: a property of `Π` alone,
/// checked in both cases. In the equivariant case, or with `assert_anyway`, each pixel pair's shapes are mirror images,
/// `shape(Π·decode(θ, φ))` against `shape(decode(θ, π − φ))` within `shape_roundtrip_bound`, at every pixel, and its
/// states within `ε_phys` in `‖·‖_phys` on the pixels farther than `delta` from each landmark; in the other case it
/// asserts no symmetry. Returns the case and the worst state residual in `ε`.
fn mirror_test(
    chart: &ShapeSphere<f64>,
    decode: DecodeIc,
    pi: MirrorFn,
    delta: f64,
    assert_anyway: bool,
) -> (MirrorCase, f64) {
    let case = classify(chart);
    let last = f64::from(GRID - 1);
    let m = mass::decode::<Defaults, f64>(chart.z_mass).m;
    let bc = landmarks(m).bc;
    let eps = eps_phys::<f64>();
    let symmetric = case == MirrorCase::Equivariant || assert_anyway;
    let mut worst: f64 = 0.0;
    for i in 0..GRID {
        for j in 0..GRID {
            let (s, t) = (f64::from(i) / last, f64::from(j) / last);
            let x = decode(chart, s, t);
            let y = pi(&x);
            let flipped = |a: [f64; 2], b: [f64; 2]| {
                a[0].to_bits() == b[0].to_bits() && a[1].to_bits() == (-b[1]).to_bits()
            };
            let (nx, ny) = (shape(x.r, x.m), shape(y.r, y.m));
            let w_negated = if nx[2] == 0.0 {
                ny[2] == 0.0
            } else {
                ny[2].to_bits() == (-nx[2]).to_bits()
            };
            assert!(
                y.m.map(f64::to_bits) == x.m.map(f64::to_bits)
                    && (0..3).all(|k| flipped(y.r[k], x.r[k]) && flipped(y.p[k], x.p[k]))
                    && lz(&y) == -lz(&x)
                    && ny[0].to_bits() == nx[0].to_bits()
                    && ny[1].to_bits() == nx[1].to_bits()
                    && w_negated,
                "Π·x is not x's mirror at ({s}, {t}): x {x:?}, Π·x {y:?}"
            );
            if !symmetric {
                continue;
            }
            let twin = decode(chart, s, 1.0 - t);
            let d = dist(shape(y.r, y.m), shape(twin.r, twin.m));
            assert!(
                d <= ROUNDTRIP,
                "the pair ({s}, {t}) and ({s}, {})'s shapes are not mirror images: {d:e} apart",
                1.0 - t
            );
            let n = chart.point(s, t);
            if bc.iter().all(|&b| dist(n, b) > delta) {
                let r = phys(&twin, &y);
                assert!(
                    r <= eps,
                    "the pair ({s}, {t}) and ({s}, {})'s states differ by {r:e} in ‖·‖_phys, beyond ε_phys {eps:e}",
                    1.0 - t
                );
                worst = worst.max(r / EPS);
            }
        }
    }
    (case, worst)
}

/// A spherical chart at random masses, `I` and `φ_f`, holding the momentum controls `z_mom`.
fn mirror_chart(rng: &mut SplitMix, z_mom: [f64; 4]) -> ShapeSphere<f64> {
    ShapeSphere {
        z_mom,
        ..random_sphere(rng)
    }
}

/// Released from rest.
const REST: [f64; 4] = [0.0; 4];
/// A held momentum with zero `y`-components, `p_ρ,y = p_λ,y = 0`.
const X_ONLY: [f64; 4] = [0.4, 0.0, -0.3, 0.0];
/// A held `p_λ,y ≠ 0`.
const LAMBDA_Y: [f64; 4] = [0.1, 0.0, 0.2, 0.5];

#[test]
fn shape_sphere_mirror_test() {
    let mut rng = SplitMix::new(0x0582);
    let mut worst: f64 = 0.0;
    for _ in 0..4 {
        for (z, want) in [
            (REST, MirrorCase::Equivariant),
            (X_ONLY, MirrorCase::Equivariant),
            (LAMBDA_Y, MirrorCase::DoesNotApply),
        ] {
            let chart = mirror_chart(&mut rng, z);
            let (case, w) = mirror_test(&chart, decode_ic, full_mirror, DELTA, false);
            assert_eq!(case, want, "the case of z_mom {z:?}");
            let masses = mass::decode::<Defaults, f64>(chart.z_mass).m;
            eprintln!("shape_sphere_mirror_test: z_mom {z:?}, masses {masses:?}: {case:?}");
            worst = worst.max(w);
        }
    }
    eprintln!(
        "shape_sphere_mirror_test: worst pair state residual {worst:.2} ε (ε_phys {} ε), away from the landmarks by \
         δ = {DELTA}",
        eps_phys::<f64>() / EPS
    );
}

negative_control!(
    shape_sphere_mirror_test,
    "R-141's gauge left in folds each pair onto one shape, equal, not mirror images, so it must fail",
    expected = "shapes are not mirror images",
    {
        let chart = mirror_chart(&mut SplitMix::new(1), X_ONLY);
        mirror_test(
            &chart,
            |c, u, v| canonicalise(&phi(c, u, v)).ic,
            full_mirror,
            DELTA,
            false,
        );
    }
);

mod mirror_turned_px {
    use super::*;
    negative_control!(
        shape_sphere_mirror_test,
        "a momentum rule turning p_x below the equator makes the pair's states not mirror images, so it must fail",
        expected = "states differ",
        {
            let chart = mirror_chart(&mut SplitMix::new(2), X_ONLY);
            mirror_test(
                &chart,
                |c, u, v| {
                    let z = c.z_mom;
                    let held = ShapeSphere {
                        z_mom: if c.point(u, v)[2] < 0.0 {
                            [-z[0], z[1], -z[2], z[3]]
                        } else {
                            z
                        },
                        ..*c
                    };
                    decode_ic(&held, u, v)
                },
                full_mirror,
                DELTA,
                false,
            );
        }
    );
}

mod mirror_does_not_apply {
    use super::*;
    negative_control!(
        shape_sphere_mirror_test,
        "asserting the symmetry where a held p_λ,y ≠ 0 makes the hemispheres different systems fails on the pairs",
        expected = "states differ",
        {
            let chart = mirror_chart(&mut SplitMix::new(3), LAMBDA_Y);
            mirror_test(&chart, decode_ic, full_mirror, DELTA, true);
        }
    );
}

mod mirror_at_bc01 {
    use super::*;
    negative_control!(
        shape_sphere_mirror_test,
        "comparing states at BC₀₁'s pixel (½, ½), where λ̃'s direction is set by rounding, must fail",
        expected = "states differ",
        {
            let chart = mirror_chart(&mut SplitMix::new(4), REST);
            mirror_test(&chart, decode_ic, full_mirror, 0.0, false);
        }
    );
}

mod mirror_configuration_only {
    use super::*;
    negative_control!(
        shape_sphere_mirror_test,
        "a configuration-only Π, positions reflected and momenta kept, on a state with p_y ≠ 0 must fail Π's check",
        expected = "is not x's mirror",
        {
            let chart = mirror_chart(&mut SplitMix::new(5), LAMBDA_Y);
            mirror_test(
                &chart,
                decode_ic,
                |x| Ic {
                    p: x.p,
                    ..canon::reflect(x, true)
                },
                DELTA,
                false,
            );
        }
    );
}

// ── The landmarks ─────────────────────────────────────────────────────────────────────────────────────────────────

type LandmarksFn = fn([f64; 3]) -> Landmarks<f64>;

/// The azimuth of a point on `[0°, 360°)`.
fn degrees(n: [f64; 3]) -> f64 {
    azimuth(n).to_degrees()
}

/// `BC₀₁ = (−1, 0, 0)` exactly; `L±` at `w = ±√(3m₀m₁m₂)/(m₀m₁ + m₀m₂ + m₁m₂)` within `16ε`, the poles at equal
/// masses; every collision exactly at `w = 0` and unit; equal
/// masses 120° apart; under every mass set. Returns the worst pole distance.
fn check_landmarks(of: LandmarksFn) -> f64 {
    let mut worst: f64 = 0.0;
    for m in mass_sets() {
        let l = of(m);
        assert_eq!(l.bc[2], [-1.0, 0.0, 0.0], "BC₀₁ under masses {m:?}");
        let w = lagrange_w(m);
        let d = (l.l_plus[2] - w)
            .abs()
            .max(dist(l.l_minus, [l.l_plus[0], l.l_plus[1], -l.l_plus[2]]));
        assert!(
            d <= 16.0 * EPS,
            "L± are {:?}, {:?}, not at w = ±{w}",
            l.l_plus,
            l.l_minus
        );
        worst = worst.max(d);
        for (k, b) in l.bc.iter().enumerate() {
            assert!(
                b[2] == 0.0 && (dist(*b, [0.0; 3]) - 1.0).abs() <= 4.0 * EPS,
                "the collision opposite body {k} is {b:?}, off the equator, under masses {m:?}"
            );
        }
    }
    let l = of([1.0 / 3.0; 3]);
    let pole = dist(l.l_plus, [0.0, 0.0, 1.0]).max(dist(l.l_minus, [0.0, 0.0, -1.0]));
    assert!(
        pole <= 16.0 * EPS,
        "equal masses: L± are {:?}, {:?}, not the poles",
        l.l_plus,
        l.l_minus
    );
    for (a, b) in [(0, 1), (1, 2), (2, 0)] {
        let dot: f64 = (0..3).map(|i| l.bc[a][i] * l.bc[b][i]).sum();
        assert!(
            (dot + 0.5).abs() <= 8.0 * EPS,
            "equal masses: the collisions {a} and {b} are not 120° apart (cos {dot})"
        );
    }
    worst
}

/// The collisions against the inspector's `collisionPhi` on the fixture's masses, as points on the equator, within
/// the inspector's tolerance.
fn check_landmarks_inspector(of: LandmarksFn) -> f64 {
    let (raw, _) = fixture();
    let mut worst: f64 = 0.0;
    for (k, c) in raw.iter().enumerate() {
        let l = of(c.m);
        // collisionPhi's pairs (0, 1), (1, 2), (0, 2): the sides opposite bodies 2, 0, 1.
        for (az, ours) in c.bc_azimuth.iter().zip([l.bc[2], l.bc[0], l.bc[1]]) {
            let theirs = [az.cos(), az.sin(), 0.0];
            let d = component(ours, theirs);
            assert!(
                d <= JS_TOL,
                "raw IC {k}: the collision {ours:?} differs from the IC Inspector's {theirs:?} by {d:e}"
            );
            worst = worst.max(d);
        }
    }
    worst
}

#[test]
fn shape_landmarks() {
    let worst = check_landmarks(landmarks);
    let js = check_landmarks_inspector(landmarks);
    eprintln!(
        "shape_landmarks: worst pole distance {worst:.3e}; worst against the inspector {js:.3e}"
    );
}

negative_control!(
    shape_landmarks,
    "the equal-mass collisions hard-coded are not the inspector's at its unequal masses, so it must fail",
    expected = "differs from the IC Inspector's",
    {
        check_landmarks_inspector(|_| landmarks([1.0 / 3.0; 3]));
    }
);

/// dd_integrator test 9 and R-50: the landmarks of [`check_landmarks`], equal masses at azimuths 180°, 60° and 300°
/// for `BC₀₁`, `BC₁₂` and `BC₂₀` within `1e-12°`; and under every mass set each collision is, within `64ε`, the `n` of
/// random configurations with that pair together, turned, scaled and moved: mass-weighted, so unequal masses move them.
fn check_mass_weighted(of: LandmarksFn) {
    check_landmarks(of);
    let l = of([1.0 / 3.0; 3]);
    for (k, want) in [(2, 180.0), (0, 60.0), (1, 300.0)] {
        assert!(
            (degrees(l.bc[k]) - want).abs() <= 1e-12,
            "equal masses: the collision opposite body {k} is at {}°, not {want}°",
            degrees(l.bc[k])
        );
    }
    let mut rng = SplitMix::new(10);
    for m in mass_sets() {
        let l = of(m);
        for k in 0..3 {
            let mut r = [0, 1, 2].map(|_| [on(&mut rng, -2.0, 2.0), on(&mut rng, -2.0, 2.0)]);
            r[(k + 2) % 3] = r[(k + 1) % 3];
            let n = shape(
                moved(
                    r,
                    on(&mut rng, -PI, PI),
                    on(&mut rng, 0.1, 10.0),
                    [0.3, -0.7],
                ),
                m,
            );
            assert!(
                dist(n, l.bc[k]) <= 64.0 * EPS,
                "the collision opposite body {k} is {:?}, not {n:?}, a configuration with that pair together, under \
                 masses {m:?}",
                l.bc[k]
            );
        }
    }
    let skewed = of([0.6, 0.3, 0.1]);
    assert!(
        (degrees(skewed.bc[0]) - 60.0).abs() > 1.0,
        "unequal masses leave BC₁₂ at 60°: not mass-weighted"
    );
}

#[test]
fn shape_landmarks_mass_weighted() {
    check_mass_weighted(landmarks);
    let l = landmarks([1.0 / 3.0; 3]);
    eprintln!(
        "shape_landmarks_mass_weighted: equal masses' collisions at {:.12}°, {:.12}°, {:.12}° (BC₀₁, BC₁₂, BC₂₀)",
        degrees(l.bc[2]),
        degrees(l.bc[0]),
        degrees(l.bc[1])
    );
}

negative_control!(
    shape_landmarks_mass_weighted,
    "fixed 120° landmarks (B18's rejected option) are not the collisions at unequal masses, so it must fail",
    expected = "a configuration with that pair together",
    {
        check_mass_weighted(|m| Landmarks {
            l_plus: landmarks(m).l_plus,
            l_minus: landmarks(m).l_minus,
            ..landmarks([1.0 / 3.0; 3])
        });
    }
);

// ── The dot on the collision landmark ──────────────────────────────────────────────────────────────────────────

/// Which landmark a collision of `i` and `j` lands on: the side opposite the third body.
type Opposite = fn(usize, usize) -> usize;

/// Bodies `i` and `j` of `r` under masses `m` brought together, `r_j ← r_i + ε(r_j − r_i)`, for `ε` down to `1e-9`:
/// the dot's distance to the landmark `opposite(i, j)` falls to within `1e-6` at `ε = 1e-9` and `1e-12` at `ε = 0`, and its
/// distance to the other two stays above 0, and from `ε = 1e-3` on above half their least distance from it.
fn check_collision_dot(
    opposite: Opposite,
    m: [f64; 3],
    r: [[f64; 2]; 3],
    i: usize,
    j: usize,
) -> Result<(), TestCaseError> {
    let l = landmarks(m);
    let k = opposite(i, j);
    let others: Vec<usize> = (0..3).filter(|&x| x != k).collect();
    let gap = others
        .iter()
        .map(|&x| dist(l.bc[x], l.bc[k]))
        .fold(f64::INFINITY, f64::min);
    // The approach is relative (pitfalls §3: a quantity spanning decades): `n` is linear in `ε` near a collision, with a
    // slope set by the masses and the triangle, so no absolute bound at a fixed `ε` holds for every mass ratio. From
    // `ε = 10⁻³` each two decades of `ε` bring the dot at least one decade nearer, and at coincidence it is two decades
    // nearer than at `ε = 10⁻⁷`, the rounding floor not a fixed multiple of ε (`λ` is a difference of nearly equal
    // positions when the pair's masses differ widely).
    let mut last = f64::INFINITY;
    for e in [1e-1, 1e-3, 1e-5, 1e-7, 0.0] {
        let mut rr = r;
        rr[j] = [
            r[i][0] + e * (r[j][0] - r[i][0]),
            r[i][1] + e * (r[j][1] - r[i][1]),
        ];
        let n = shape(rr, m);
        let d = dist(n, l.bc[k]);
        for &x in &others {
            let away = dist(n, l.bc[x]);
            prop_assert!(
                away > 0.0 && (e > 1e-3 || away > gap / 2.0),
                "bodies {i} and {j} at ε = {e}: the dot {n:?} nears the wrong landmark {x} ({away:e})"
            );
        }
        let factor = if e == 0.0 { 100.0 } else { 10.0 };
        prop_assert!(
            e > 1e-5 || d <= last / factor,
            "bodies {i} and {j} at ε = {e}: the dot does not approach landmark {k}: {d:e}, against {last:e} at the \
             step before (masses {m:?})"
        );
        last = d;
    }
    Ok(())
}

fn the_opposite(i: usize, j: usize) -> usize {
    3 - i - j
}

/// Random masses, a random triangle, and an ordered pair of distinct bodies.
fn collisions() -> impl Strategy<Value = ([f64; 3], [[f64; 2]; 3], usize, usize)> {
    (
        (-4.0..4.0f64, -4.0..4.0f64),
        proptest::array::uniform3(proptest::array::uniform2(-2.0..2.0f64)),
        0usize..3,
        1usize..3,
    )
        .prop_map(|((z1, z2), r, i, step)| {
            (
                mass::decode::<Defaults, f64>([z1, z2]).m,
                r,
                i,
                (i + step) % 3,
            )
        })
        .prop_filter("a triangle with no pair already near", |(_, r, _, _)| {
            (0..3).all(|a| {
                let b = (a + 1) % 3;
                (r[a][0] - r[b][0]).hypot(r[a][1] - r[b][1]) > 0.05
            })
        })
}

#[test]
fn shape_dot_on_collision_landmark() {
    prop::run(&collisions(), |(m, r, i, j)| {
        check_collision_dot(the_opposite, m, r, i, j)
    });
    eprintln!("shape_dot_on_collision_landmark: {}", prop::cases_status());
}

negative_control!(
    shape_dot_on_collision_landmark,
    "the landmark indexed by a body of the pair, not the third, is the wrong collision, so it must fail",
    expected = "landmark",
    {
        prop::run(&collisions(), |(m, r, i, j)| {
            check_collision_dot(|i, _| i, m, r, i, j)
        });
    }
);
