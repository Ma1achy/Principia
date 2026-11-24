//! The shape-sphere chart and the shape map's landmarks (`kernel::chart::shape_sphere`, `kernel::shape`).
//! - `shape_vec_poles`: `L⁺` (bodies 0 → 1 → 2 anticlockwise, equilateral) is `w = +1`, `L⁻` is `w = −1`, and the
//!   collision of bodies 0 and 1 is exactly `(−1, 0, 0)`, under several masses, turned and scaled, at f64 and f32,
//!   through `shape` and `shape_vec`.
//! - `shape_vec_inspector`: `n` against the IC Inspector's on `fixtures/ic_inspector/shape_vec.json`'s random ICs,
//!   within `shape_vec_js_tol`, printing the measured worst.
//! - `shape_sphere_chart_map_spherical`: `(s, 1)` is `L⁺` exactly, `(s, 0)` is `L⁻`, and the polar angle runs over the
//!   whole `[0, π]`, `π·(1 − t)`, with the azimuth `2π·s`, at f64 and f32.
//! - `shape_sphere_chart_map_exponential`: the exponential map at `d = 0` is `n₀` exactly, its points are unit, and
//!   each lies at the geodesic distance `r = ‖d‖` from `n₀`.
//! - `shape_sphere_inverse_poles`: at `n₀ = ±1` with `±0` in `n₁` and `n₂`, `β` is `+0` and `ψ = φ_f`; an `n`
//!   off the sphere by a rounding forms no root of a negative.
//! - `shape_sphere_roundtrip`: `Φ` alone, `shape(Φ(u, v)) = n(u, v)` within `shape_roundtrip_bound` on both
//!   hemispheres, the gate's measure in miniature.
//! - `shape_sphere_roundtrip_near_poles`: the same within polar radii `10⁻²` to `10⁻¹⁵` of `n₀ = ±1`, where `n₀` rounds
//!   to `±1` and the smaller of `a` and `b` must come from `n₁² + n₂²`.
//! - `shape_sphere_canonical_form`: `Φ` then `C`, which has no mirror step, keeps every point's signed shape, a
//!   lower-hemisphere point below the equator, with the held masses and momenta, and never mirrors.
//! - `shape_sphere_shared_decoder`: the chart's decode is the shared blocks, the configuration [`place`]'s, `C` with its
//!   step 3 not taken after, and its `validate` is `Valid` everywhere.
//! - `shape_sphere_decode_canonical`: the decode's `ic` is `C(Φ)` (C has no mirror step), on `I = 1`, and its
//!   descriptor that state's, at `I ≠ 1` and on both hemispheres (dd_decoder §2, §3.6).
//! - `shape_sphere_energy_norm`: `η_E`'s entry, `energy_norm::decode_chart`, decodes the shape sphere through its own
//!   decode, the configuration `place`'s at `n(u, v)`, with and without a target `E*`.
//! - `shape_landmarks`: `BC₀₁ = (−1, 0, 0)`, `L⁺ = (0, 0, 1)`, every collision at `w = 0`, equal masses 120° apart, and
//!   the collisions against the inspector's.
//! - `shape_landmarks_mass_weighted`: dd_integrator test 9, `BC₀₁ = (−1, 0, 0)` for any masses, `L⁺ = (0, 0, 1)`,
//!   every collision at `w = 0`, equal masses at 180°, 60°, 300°, and each collision the `n` of any configuration with
//!   that pair together.
//! - `shape_dot_on_collision_landmark`: bodies `i` and `j` brought together put the dot on the landmark opposite `k`,
//!   and keep it off the other two.
//! - `shape_sphere_mirror_runs`: the run from `Π·x` is the run from `x` mirrored, bitwise, at every state the march
//!   left, under KDK, RK4 and Heggie × RK4 at f64 and f32, released from rest and with a held momentum of zero
//!   `y`-components: the fates and `t_end_step` equal, `L_z` and the frame's winding negated; "does not apply" with a
//!   held `p_λ,y ≠ 0` (chart_reference §3.3).
//! - `shape_sphere_mirror_runs_pairs`: the pixel pairs `(θ, φ)`, `(θ, π − φ)` on two named fixtures shown non-chaotic
//!   by a separation bound: fates equal, `L_z` and the winding opposite.
//! - `frame_winding`, `frame_winding_runs`: the frame's winding on its definition's cases (a rigid rotation, the
//!   negative `x`-axis, the `±π` tie, `ρ` on the axis and through zero) and on real runs, negated under `Π`.
//!
//! **Tolerances.** The inspector's and the round trip's are the register's. The others are multiples of the
//! precision's epsilon, set from the worst case each test measures, with headroom (this test's choice, as
//! `canon.rs`'s). `shape_landmarks_mass_weighted`'s is `8ε·κ`, scaled by `shape`'s condition number
//! `κ` at each draw: a collision configuration is an ill-conditioned input, `|ρ|` small against the positions, and
//! the error is `shape`'s rounding on it, not the landmark's; the worst error over 600,000 draws is `3.8 ε·κ`
//! (physics review 5897335433; this test's choice).

use std::path::Path;

use kernel::canon::{self, canonicalise};
use kernel::chart::shape_sphere::{
    exponential, held_momentum, inverse, place, spherical, Inverse, ShapeMap, ShapeSphere,
};
use kernel::chart::{self, Chart, ValidationResult};
use kernel::decode::energy_norm::{self, Outcome};
use kernel::decode::{descriptor, mass, momentum, Decoded, Ic};
use kernel::detect::collision::CollisionRadius;
use kernel::driver::advance::{Advance, Cadence};
use kernel::driver::cadence::WrapperCadence;
use kernel::driver::nsub_table::{thresholds, NsubConfig};
use kernel::driver::substep::Quantise;
use kernel::driver::wrapper::{
    integrate_detecting, integrate_with, monitored, WrapperReal, WrapperUniforms,
};
use kernel::generated::links::{Defaults, LinkReal};
use kernel::occupant::kdk::Kdk;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::{Geom, State};
use kernel::payload::{
    sd_detail, sd_state, tm_t_end_step, PayloadReal, SimStateBaseOf, STATE_BOUNDED,
};
use kernel::regularisation::az::AzReal;
use kernel::regularisation::heggie::{HgConfig, HgParams, DEFAULT_TIME};
use kernel::regularisation::pairs::DefaultOccupant;
use kernel::shape::{landmarks, shape, shape_vec, weighted_jacobi, Landmarks};
use kernel::stage::decode::StickBreakingLinks;
use kernel::winding::Winding;
use kernel::Real;
use proptest::prelude::*;
use validation::decode_f32_eps::SplitMix;
use validation::roundtrip::{eps_phys, phys};
use validation::{negative_control, prop, SHAPE_ROUNDTRIP_BOUND, SHAPE_VEC_JS_TOL};

const PI: f64 = std::f64::consts::PI;
const EPS: f64 = f64::EPSILON;

/// `shape_vec_js_tol`, provisional until M2.
const JS_TOL: f64 = SHAPE_VEC_JS_TOL.number();

/// `shape_roundtrip_bound`, provisional until M2.
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
/// `w = √(3m₀m₁m₂)/(m₀m₁ + m₀m₂ + m₁m₂)`: 1 at equal masses, below 1 otherwise.
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

// ── The inverse at its poles ───────────────────────────────────────────────────────────────────────────────────────

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
            assert_eq!(got.beta, n[2].arctan2(n[1]), "β off the pole at {n:?}");
        }
        for n0 in [1.0, -1.0] {
            for n1 in [0.0, -0.0] {
                for n2 in [0.0, -0.0] {
                    let got = inv([n0, n1, n2], i, phase);
                    let (a, b) = (i * (1.0 + n0) / 2.0, i * (1.0 - n0) / 2.0);
                    let along = |x: f64| [x.sqrt() * phase.cosine(), x.sqrt() * phase.sine()];
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

// ── Φ then C, with no mirror step ──────────────────────────────────────────────────────────────────────────────────────────────

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

/// `Φ` alone at `(s, t)`, without `C`: `D_mass`, [`place`] and [`held_momentum`] joined.
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

/// `C(Φ(chart, s, t))` (C has no mirror step), `Φ` given by `phi`: its shape is `n(s, t)` with its sign on both
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
        let c = canonicalise(&phi(&chart, s, t));
        let got = shape(c.ic.r, c.ic.m);
        assert!(
            dist(got, n) <= tol,
            "C(Φ({s}, {t})) (C has no mirror step, R-865) is at {got:?}, not n {n:?}"
        );
        assert_eq!(c.ic.m, fixed_m.m, "C(Φ)'s masses are not m_fixed");
        let dp = relative(c.ic.p, fixed_p);
        assert!(
            dp <= tol,
            "C(Φ({s}, {t}))'s momenta {:?} are not p_fixed {fixed_p:?}",
            c.ic.p
        );
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
/// their join (C has no mirror step) and its descriptor that `ic`'s, `Σpᵢ = 0` and the CoM at the origin within
/// `16ε`; `map`'s controls are the held blocks; `validate` is `Valid` inside and past `[0,1]²`; the name is
/// `ShapeSphere`.
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
            canonicalise(&joined).ic,
            "the ic is not C of the joined blocks (C has no mirror step, R-865)"
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

/// `decode` on random charts of both maps, `I` in `[¼, 4]`, both hemispheres drawn: its `ic` is `C(Φ)` (no mirror step,
/// on the signed sphere), at `I = 1` within `64ε`, and its descriptor is that state's, with `rho_mag` and
/// `lambda_mag` in `[0, 1]`.
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
        let want = canonicalise(&phi(&chart, u, v)).ic;
        assert_eq!(
            d.ic, want,
            "the decode's ic is not C(Φ) (C has no mirror step, R-865) at ({u}, {v})"
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

// ── The signed decode ─────────────────────────────────────────────────────────────────────────────────────────────

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
/// included: its shape is `n(θ, φ)` with its sign, within `shape_roundtrip_bound`; it is
/// canonical in its CoM, scale, boost and rotation, `C`, which has no mirror step, leaving it unchanged within
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
        let again = canonicalise(&x);
        let off = relative(again.ic.r, x.r).max(relative(again.ic.p, x.p));
        let i = canon::inertia(x.m, x.r);
        assert!(
            off <= SIGNED_TOL && (i - 1.0).abs() <= SIGNED_TOL,
            "the decoded state at ({s}, {t}) is not canonical: C moves it by {off:e}, I − 1 = \
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

#[test]
fn shape_sphere_signed_decode() {
    let worst = check_signed_decode(decode_ic);
    eprintln!(
        "shape_sphere_signed_decode: worst ‖shape(decode) − n‖ {worst:.3e} over 2048 pixels of both hemispheres"
    );
}

negative_control!(
    shape_sphere_signed_decode,
    "R-141's gauge left in, C's mirror step run on Φ, lands every lower pixel at (θ, π − φ), so it must fail",
    expected = "not the signed n",
    {
        check_signed_decode(|c, u, v| {
            let out = canonicalise(&phi(c, u, v));
            canon::reflect(&out.ic, out.lambda_t_y < -1e-12)
        });
    }
);

/// `Φ` with `p_fixed` mirrored below the equator, the retired 2-to-1 gauge's held momentum, its `y`-controls negated
/// where `n₂ < 0`, then `C` (C has no mirror step): a control's decode.
#[cfg(feature = "controls")]
fn mirrored_hold(chart: &ShapeSphere<f64>, u: f64, v: f64) -> Ic<f64> {
    let below = chart.point(u, v)[2] < 0.0;
    let z = chart.z_mom;
    let held = ShapeSphere {
        z_mom: if below { [z[0], -z[1], z[2], -z[3]] } else { z },
        ..*chart
    };
    canonicalise(&phi(&held, u, v)).ic
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

// ── The mirror test ───────────────────────────────────────────────────────────────────────────────────────────────

/// Which case the mirror test is in: the coordinates the chart holds fixed are all their own mirror images, or not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MirrorCase {
    /// The masses, always, and `p_fixed` with its `y`-components zero (released from rest included): the pixels
    /// `(θ, φ)` and `(θ, π − φ)` are mirror twins.
    Equivariant,
    /// `p_fixed` has a `y`-component: the two hemispheres are different systems, and the test asserts no symmetry.
    DoesNotApply,
}

/// The case `chart` is in, from what it holds fixed: its masses are mirror-invariant, so the held
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
/// `32 ulp ≤ ε_phys = 64 ulp`, the reason for `δ`.
const DELTA: f64 = 1.0 / 16.0;

/// The mirror test on `chart`, its decode `decode` and its `Π` `pi`, the state comparison
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
    "R-141's gauge left in, C's retired mirror step, folds each pair onto one shape, equal, not mirror images, so it \
     must fail",
    expected = "shapes are not mirror images",
    {
        let chart = mirror_chart(&mut SplitMix::new(1), X_ONLY);
        mirror_test(
            &chart,
            |c, u, v| {
                let out = canonicalise(&phi(c, u, v));
                kernel::canon::reflect(&out.ic, out.lambda_t_y < -1e-12)
            },
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

/// The condition number of `shape` at a configuration `r` with the pair opposite body `k` together (`k` = 0 or 1):
/// `κ = 1 + (max|rᵢ| / |ρ|) · (M₀₁ / m_k)`, `ρ = r₁ − r₀`, `M₀₁ = m₀ + m₁`: `shape`'s rounding error on the
/// configuration grows with it (physics review 5897335433).
fn condition(r: [[f64; 2]; 3], m: [f64; 3], k: usize) -> f64 {
    let norm = |v: [f64; 2]| v[0].hypot(v[1]);
    let big = r.iter().map(|&v| norm(v)).fold(0.0, f64::max);
    let rho = norm([r[1][0] - r[0][0], r[1][1] - r[0][1]]);
    1.0 + (big / rho) * ((m[0] + m[1]) / m[k])
}

/// dd_integrator test 9, mass-weighted landmarks: the landmarks of [`check_landmarks`], equal masses at azimuths
/// 180°, 60° and 300° for `BC₀₁`, `BC₁₂` and `BC₂₀` within `1e-12°`; and under every mass set each collision is
/// the `n` of random configurations with that pair together, turned, scaled and moved: `BC₀₁` exactly, the others
/// within `8ε·κ` ([`condition`]); mass-weighted, so unequal masses move them.
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
            let r = moved(
                r,
                on(&mut rng, -PI, PI),
                on(&mut rng, 0.1, 10.0),
                [0.3, -0.7],
            );
            let n = shape(r, m);
            let d = dist(n, l.bc[k]);
            if k == 2 {
                // Bodies 0 and 1 together: ρ = r₁ − r₀ = 0, and the collision is exact.
                assert!(
                    n == l.bc[k],
                    "the collision opposite body {k} is {:?}, not {n:?}, a configuration with that pair together, \
                     under masses {m:?}",
                    l.bc[k]
                );
            } else {
                let kappa = condition(r, m, k);
                assert!(
                    d <= 8.0 * EPS * kappa,
                    "the collision opposite body {k} is {:?}, not {n:?}, a configuration with that pair together, \
                     under masses {m:?}: {:.2} ε against 8 ε · κ, κ = {kappa:.2}",
                    l.bc[k],
                    d / EPS
                );
            }
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

// ── The mirror test's runs and the frame's winding ─────────────────────────────────────────────────────────────────

/// A precision the runs are made at, f64 the CPU's and f32 the GPU's, each widened exactly to f64 for the checks.
trait Prec: WrapperReal + LinkReal + Real + Into<f64> + std::fmt::Debug {
    /// `x` at this precision.
    fn of(x: f64) -> Self;
    /// `chart`'s own decode at `(s, t)` at this precision: the f64 chart's values rounded once at f32.
    fn decode_at(chart: &ShapeSphere<f64>, s: f64, t: f64) -> Ic<Self>;
}

impl Prec for f64 {
    fn of(x: f64) -> f64 {
        x
    }

    fn decode_at(chart: &ShapeSphere<f64>, s: f64, t: f64) -> Ic<f64> {
        decode_ic(chart, s, t)
    }
}

impl Prec for f32 {
    fn of(x: f64) -> f32 {
        x as f32
    }

    fn decode_at(chart: &ShapeSphere<f64>, s: f64, t: f64) -> Ic<f32> {
        let n = |v: [f64; 3]| v.map(|x| x as f32);
        let narrow = ShapeSphere::<f32> {
            map: chart.map,
            n0: n(chart.n0),
            e1: n(chart.e1),
            e2: n(chart.e2),
            scale: chart.scale as f32,
            inertia: chart.inertia as f32,
            phase: chart.phase as f32,
            z_mass: chart.z_mass.map(|x| x as f32),
            z_mom: chart.z_mom.map(|x| x as f32),
        };
        narrow.decode::<Defaults, f32>(s as f32, t as f32).ic
    }
}

/// `x`'s bits, widened exactly to f64, so an f32's bits are distinct where the f32's are.
fn bits_of<R: Prec>(x: R) -> u64 {
    Into::<f64>::into(x).to_bits()
}

/// `b` is `a`'s negation: bit for bit where `a` is nonzero, and by value where it is zero, since an exactly zero result
/// takes the sign the rounding gives, `+0` in both runs (the module's comparison).
fn negation<R: Prec>(a: R, b: R) -> bool {
    let (a, b): (f64, f64) = (a.into(), b.into());
    if a == 0.0 {
        b == 0.0
    } else {
        b.to_bits() == (-a).to_bits()
    }
}

/// `b` is `a`: bit for bit where `a` is nonzero, by value where it is zero.
fn equal<R: Prec>(a: R, b: R) -> bool {
    let (a, b): (f64, f64) = (a.into(), b.into());
    if a == 0.0 {
        b == 0.0
    } else {
        b.to_bits() == a.to_bits()
    }
}

/// Heggie's step-control values for the runs, as `tests/heggie_seam.rs` takes them: `η = 0.01`, `Δ_sync = 1/8`, the
/// reference's `TINY`, `f = 0.02`, a budget that does not bind, the landing in force, Eq. 22. Test inputs, not
/// calibrations.
struct HgRuns;

impl HgConfig for HgRuns {
    fn params<R: AzReal>() -> HgParams<R> {
        HgParams {
            eta: R::at_width(0.01, 0.01),
            delta_sync: R::at_width(0.125, 0.125),
            tiny: R::at_width(1e-37, 1e-300),
            predictive_f: R::at_width(0.02, 0.02),
            step_budget_rate: R::at_width(1e7, 1e7),
            n_min: 1_000_000,
            clamp: true,
            secant_tries: 4,
            bisect_tries: 64,
            time: DEFAULT_TIME,
        }
    }
}

/// The default occupant, Heggie × RK4, under [`HgRuns`].
type HeggieRk4 = DefaultOccupant<HgRuns>;

/// The runs' macro-step, `10⁻³`.
const DT_MACRO: f64 = 1e-3;

/// The runs' collision threshold, the default `r_coll = 10⁻³` (integrator_contract Part 3's table).
const R_COLL: f64 = 1e-3;

/// The sim key's loop at [`DT_MACRO`] for `horizon` macro-steps, the default substep law.
fn run_uniforms<R: Prec>(horizon: u32) -> WrapperUniforms<R> {
    let law = NsubConfig::DEFAULT;
    WrapperUniforms {
        dt_macro: R::of(DT_MACRO),
        horizon_steps: horizon,
        n_max: law.n_max,
        thresholds: thresholds(&law).expect("the default substep law generates"),
    }
}

/// A state of a run: its positions and momenta.
type Phase<R> = ([[R; 2]; 3], [[R; 2]; 3]);

/// The wrapper's own cadence, passed through, with every state it sees recorded: each internal step's after the
/// cadence's projection, and each sampling point's with its index.
struct Recorder<R: WrapperReal> {
    inner: WrapperCadence<R>,
    steps: Vec<Phase<R>>,
    samples: Vec<(u32, Phase<R>)>,
}

impl<R: WrapperReal> Cadence<R> for Recorder<R> {
    fn after_step(&mut self, state: &mut State<R>, geom: &mut Geom<R>) {
        self.inner.after_step(state, geom);
        self.steps.push((state.r, state.p));
    }

    fn done(&self) -> bool {
        self.inner.done()
    }

    fn sample(&mut self, state: &State<R>, index: u32, quantum: R) {
        self.inner.sample(state, index, quantum);
        self.samples.push((index, (state.r, state.p)));
    }

    fn signals(&mut self, floored: bool, cap_hit: bool) {
        self.inner.signals(floored, cap_hit);
    }

    fn regularised_drift(&mut self, drift: R) {
        self.inner.regularised_drift(drift);
    }

    fn bucket_d2(&self, r: &[[R; 2]; 3]) -> f32
    where
        R: Quantise,
    {
        self.inner.bucket_d2(r)
    }

    fn clock(&mut self, completed: u32) {
        self.inner.clock(completed);
    }

    fn resume(&mut self, stored: &SimStateBaseOf<R>)
    where
        R: PayloadReal,
    {
        self.inner.resume(stored);
    }

    fn latched(&mut self) {
        self.inner.latched();
    }

    fn finish(&self, out: SimStateBaseOf<R>) -> SimStateBaseOf<R>
    where
        R: PayloadReal,
    {
        self.inner.finish(out)
    }
}

/// A run: its IC, the `SimState` the wrapper wrote, and every state it passed through.
struct Run<R: WrapperReal> {
    ic: Ic<R>,
    out: SimStateBaseOf<R>,
    steps: Vec<Phase<R>>,
    samples: Vec<(u32, Phase<R>)>,
}

/// Every field of a `SimState` (its declared tail padding aside), as bits.
fn sim_bits<R: Prec>(s: &SimStateBaseOf<R>) -> Vec<u64> {
    let mut v: Vec<u64> =
        s.r.iter()
            .chain(&s.p)
            .flatten()
            .map(|&x| bits_of(x))
            .collect();
    v.extend(
        [
            s.S,
            s.theta,
            s.mean_y,
            s.C_ty,
            s.E_0,
            s.Lz_0,
            s.closure_min,
            s._nhat_lag_0,
            s._nhat_lag_1,
            s._nhat_lag_2,
        ]
        .map(bits_of),
    );
    v.extend(
        [
            s.packed_a,
            s.packed_b,
            s.times,
            s.total_substeps,
            s.coll_floor_bits,
        ]
        .map(u64::from),
    );
    v.extend([s.closure_step, s._reserved].map(u64::from));
    v
}

/// The run of `ic` by the occupant `A` over `horizon` macro-steps under [`R_COLL`]: the wrapper's own entry,
/// `integrate_with` and then `monitored`, as `integrate_detecting` runs them, its cadence recorded. Its `SimState` must
/// be `integrate_detecting`'s, field for field, so the outcome and `t_end` the test reads are the wrapper's and the
/// classifier's.
fn run<A: Advance, R: Prec>(ic: &Ic<R>, horizon: u32) -> Run<R> {
    let uniforms = run_uniforms::<R>(horizon);
    let radius = CollisionRadius::from_r_coll(R_COLL);
    let mut rec = Recorder {
        inner: WrapperCadence::with_radius(&ic.m, &ic.r, &ic.p, radius),
        steps: Vec::new(),
        samples: Vec::new(),
    };
    let out = monitored(
        integrate_with::<A, R, _>(ic, &uniforms, &mut rec),
        &rec.inner,
    );
    let direct = integrate_detecting::<A, R>(ic, &uniforms, radius);
    assert!(
        sim_bits(&out) == sim_bits(&direct),
        "the recorded run's SimState is not the wrapper's: {out:?} against {direct:?}"
    );
    Run {
        ic: *ic,
        out,
        steps: rec.steps,
        samples: rec.samples,
    }
}

/// A run's fate: its outcome class `(state, detail)` and `t_end_step`, as the wrapper and the classifier wrote them.
fn fate<R: Prec>(s: &SimStateBaseOf<R>) -> (u32, u32, u32) {
    (
        sd_state(s.packed_a),
        sd_detail(s.packed_a),
        tm_t_end_step(s.times),
    )
}

/// `L_z = Σ rᵢ × pᵢ` at the run's precision, in body order.
fn lz_at<R: Prec>(r: &[[R; 2]; 3], p: &[[R; 2]; 3]) -> R {
    let term = |i: usize| r[i][0] * p[i][1] - r[i][1] * p[i][0];
    term(0) + term(1) + term(2)
}

/// The frame's winding of a run, or a control's stand-in.
type WindingOf<R> = fn(&Run<R>) -> R;

/// The kernel's frame's winding of a run (chart_reference §3.3): from the IC, every state the cadence saw, then the
/// state the `SimState` stores.
fn frame_of<R: Prec>(run: &Run<R>) -> R {
    let mut w = Winding::new(&run.ic.r);
    for s in &run.steps {
        w.step(&s.0);
    }
    w.step(&run.out.r);
    w.total
}

/// Every state of a run after its IC: each one the cadence saw, then the one the `SimState` stores, which the occupant
/// hands back and can differ from the last the cadence saw by a rounding.
fn states<R: Prec>(run: &Run<R>) -> Vec<Phase<R>> {
    run.steps
        .iter()
        .copied()
        .chain([(run.out.r, run.out.p)])
        .collect()
}

/// The mirror applied to a sampled state for the comparison: `Π`, or a control's.
type StateMirror<R> = fn(&Phase<R>) -> Phase<R>;

/// `Π` on a sampled state: `y → −y` on every `rᵢ` and `pᵢ`.
fn reflected<R: Prec>(s: &Phase<R>) -> Phase<R> {
    let f = |v: [R; 2]| [v[0], -v[1]];
    (s.0.map(f), s.1.map(f))
}

/// The largest per-step turn the winding can take and still be the net turn of `ρ`: the sum telescopes only while no
/// step turns `ρ` by `π` or more, and `π/2` leaves a margin of `π/2` (chart_reference §3.3's cadence).
const TURN_MAX: f64 = PI / 2.0;

/// The largest `|turn|` of a run's winding, over every step from the IC.
fn largest_turn<R: Prec>(run: &Run<R>) -> f64 {
    let mut prev = kernel::winding::rho(&run.ic.r);
    let mut most = 0.0f64;
    for r in run.steps.iter().map(|s| &s.0).chain([&run.out.r]) {
        let next = kernel::winding::rho(r);
        most = most.max(Into::<f64>::into(kernel::winding::turn(prev, next)).abs());
        prev = next;
    }
    most
}

/// What the exact arm measured, for its report and its non-vacuity checks.
#[derive(Default, Debug)]
struct Exact {
    runs: usize,
    states: usize,
    lz_min: f64,
    winding_min: f64,
    theta_min: f64,
    turn_max: f64,
    stored_apart: usize,
    p_y_states: usize,
    fates: Vec<(u32, u32, u32)>,
}

/// The mirror test's exactly mirrored run (chart_reference §3.3's whole-sphere paragraph): on `chart`'s pixels, the
/// run from `pi(x)` against the run from `x`, `x` the chart's decode, by the occupant `A` at `R` over `horizon`. In the
/// case "does not apply" it says so and asserts nothing (`None`). In the equivariant case, per pixel, in this order:
/// `L_z` at the IC and the `SimState`'s `Lz_0` negated; the frame's winding (`winding`) negated; `L_z` negated at every
/// state; the outcome, its detail and `t_end_step` equal; the same number of states; and every state the march left
/// `mirror` of `x`'s, every `rᵢ` and `pᵢ`, compared on bits where nonzero and by value where zero.
fn exact_arm<A: Advance, R: Prec>(
    chart: &ShapeSphere<f64>,
    pixels: &[(f64, f64)],
    horizon: u32,
    pi: fn(&Ic<R>) -> Ic<R>,
    mirror: StateMirror<R>,
    winding: WindingOf<R>,
) -> Option<Exact> {
    if classify(chart) == MirrorCase::DoesNotApply {
        return None;
    }
    let mut e = Exact {
        lz_min: f64::INFINITY,
        winding_min: f64::INFINITY,
        theta_min: f64::INFINITY,
        ..Exact::default()
    };
    for &(s, t) in pixels {
        let x = R::decode_at(chart, s, t);
        let y = pi(&x);
        let (rx, ry) = (run::<A, R>(&x, horizon), run::<A, R>(&y, horizon));
        let (lx, ly) = (lz_at(&x.r, &x.p), lz_at(&y.r, &y.p));
        assert!(
            negation(lx, ly) && negation(rx.out.Lz_0, ry.out.Lz_0),
            "({s}, {t}): L_z not negated at the IC: x's {lx:?} ({:#x}), Π·x's {ly:?} ({:#x}); Lz_0 {:?} and {:?}",
            bits_of(lx),
            bits_of(ly),
            rx.out.Lz_0,
            ry.out.Lz_0
        );
        let (wx, wy) = (winding(&rx), winding(&ry));
        assert!(
            negation(wx, wy),
            "({s}, {t}): the frame's winding not negated: x's {wx:?} ({:#x}), Π·x's {wy:?} ({:#x})",
            bits_of(wx),
            bits_of(wy)
        );
        for (k, (a, b)) in states(&rx).iter().zip(&states(&ry)).enumerate() {
            let (la, lb) = (lz_at(&a.0, &a.1), lz_at(&b.0, &b.1));
            assert!(
                negation(la, lb),
                "({s}, {t}): L_z not negated at state {k}: {la:?} and {lb:?}"
            );
        }
        let (fx, fy) = (fate(&rx.out), fate(&ry.out));
        assert!(
            fx == fy,
            "({s}, {t}): the fates differ: x's (state, detail, t_end_step) {fx:?}, Π·x's {fy:?}"
        );
        assert!(
            rx.steps.len() == ry.steps.len(),
            "({s}, {t}): the runs left {} and {} states",
            rx.steps.len(),
            ry.steps.len()
        );
        for (k, (a, b)) in states(&rx).iter().zip(&states(&ry)).enumerate() {
            let want = mirror(a);
            for i in 0..3 {
                for (name, wv, got, xv) in [
                    ("r", want.0[i], b.0[i], a.0[i]),
                    ("p", want.1[i], b.1[i], a.1[i]),
                ] {
                    for (c, axis) in ["x", "y"].iter().enumerate() {
                        assert!(
                            equal(wv[c], got[c]),
                            "({s}, {t}): sampled state {k} of {} not reflected: body {i}'s {name}_{axis}: x's {:?} \
                             ({:#x}), Π·x's {:?} ({:#x})",
                            rx.steps.len() + 1,
                            xv[c],
                            bits_of(xv[c]),
                            got[c],
                            bits_of(got[c])
                        );
                    }
                }
            }
            if a.1.iter().any(|v| Into::<f64>::into(v[1]) != 0.0) {
                e.p_y_states += 1;
            }
        }
        e.runs += 1;
        e.states += rx.steps.len() + 1;
        e.lz_min = e.lz_min.min(Into::<f64>::into(lx).abs());
        e.winding_min = e.winding_min.min(Into::<f64>::into(wx).abs());
        e.theta_min = e.theta_min.min(Into::<f64>::into(rx.out.theta).abs());
        e.turn_max = e.turn_max.max(largest_turn(&rx));
        let last = states(&rx);
        let flat = |x: &Phase<R>| -> Vec<u64> {
            x.0.iter()
                .chain(&x.1)
                .flatten()
                .map(|&v| bits_of(v))
                .collect()
        };
        if last.len() > 1 && flat(&last[last.len() - 2]) != flat(&last[last.len() - 1]) {
            e.stored_apart += 1;
        }
        e.fates.push(fx);
    }
    Some(e)
}

/// The largest component difference between two states, over every `rᵢ` and `pᵢ`.
fn separation(a: &Phase<f64>, b: &Phase<f64>) -> f64 {
    let flat = |s: &Phase<f64>| -> Vec<f64> { s.0.iter().chain(&s.1).flatten().copied().collect() };
    flat(a)
        .iter()
        .zip(flat(b))
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max)
}

/// The largest component magnitude of a state.
fn magnitude(a: &Phase<f64>) -> f64 {
    a.0.iter()
        .chain(&a.1)
        .flatten()
        .fold(0.0f64, |m, x| m.max(x.abs()))
}

/// A named fixture of the pixel-pair arm: its chart (masses, `I`, `φ_f` and the held momentum), the upper pixels
/// `(s, t)` of its pairs, each paired with `(s, 1 − t)`, its horizon in macro-steps, and the bound its pairs' state
/// separation stays below over that horizon, which shows it non-chaotic there.
struct PairFixture {
    name: &'static str,
    chart: ShapeSphere<f64>,
    pairs: &'static [(f64, f64)],
    horizon: u32,
    bound: f64,
}

/// What the pixel-pair arm measured.
#[derive(Default, Debug)]
struct Pairs {
    pairs: usize,
    separated_at_ic: usize,
    sep0_max: f64,
    sep_max: f64,
    terminal: usize,
    lz_min: f64,
    lz_gap: f64,
    winding_min: f64,
    winding_gap: f64,
}

/// The tolerance on `|L_z(a) + L_z(b)|` across a pair whose states differ by at most `sep` in each component, each
/// component at most `scale`: `L_z = Σ (xᵢp_{y,i} − yᵢp_{x,i})` moves by at most `2·sep·scale` per product, six
/// products, so `12·sep·scale`, with its rounding, `16ε·scale²`.
fn lz_tolerance(sep: f64, scale: f64) -> f64 {
    12.0 * sep * scale + 16.0 * EPS * scale * scale
}

/// The tolerance on `|W(a) + W(b)|` across a pair, where `ρ` never falls below `rho_min` and the two runs' states differ
/// by at most `sep` in each component. The winding is the net turn, `angle(ρ_end) − angle(ρ_0)` unwrapped, whatever the
/// cadence; a component difference `sep` in each body's position moves `ρ` by at most `2√2·sep`, its angle by at most
/// `2√2·sep/rho_min` at each end, so `4√2·sep/rho_min`, under `8·sep/rho_min`; and each run's sum of `n` turns rounds
/// by at most `n·ε·(π + |W|)` (a turn's own few roundings within it), so `2n·ε·(π + |W|)` for the two.
fn winding_tolerance(sep: f64, rho_min: f64, n: usize, w: f64) -> f64 {
    8.0 * sep / rho_min + 2.0 * (n as f64) * EPS * (PI + w.abs())
}

/// The pixel-pair arm on the fixture `fx` by the occupant `A` at f64: each pair `(s, t)` and `(s, 1 − t)`, `twin` the
/// second pixel's decode (the chart's own, or a control's). In this order: the outcome, its detail and `t_end_step`
/// equal; the pair's state separation, `Π` of the first run's against the second's at each sampling point both reached
/// and at the states the runs ended in, below `fx.bound`, else the fixture is refused as chaotic; `L_z` opposite at the
/// IC and at the end within [`lz_tolerance`] of the bound; and the frame's winding opposite within
/// [`winding_tolerance`] of the bound.
fn pair_arm<A: Advance>(fx: &PairFixture, twin: DecodeIc) -> Pairs {
    assert_eq!(
        classify(&fx.chart),
        MirrorCase::Equivariant,
        "{}: the fixture is not mirror-equivariant",
        fx.name
    );
    let mut out = Pairs {
        lz_min: f64::INFINITY,
        winding_min: f64::INFINITY,
        ..Pairs::default()
    };
    for &(s, t) in fx.pairs {
        let (x1, x2) = (decode_ic(&fx.chart, s, t), twin(&fx.chart, s, 1.0 - t));
        let (r1, r2) = (
            run::<A, f64>(&x1, fx.horizon),
            run::<A, f64>(&x2, fx.horizon),
        );
        let (f1, f2) = (fate(&r1.out), fate(&r2.out));
        assert!(
            f1 == f2,
            "{}: the pair ({s}, {t}), ({s}, {}): the fates differ: (state, detail, t_end_step) {f1:?} and {f2:?}",
            fx.name,
            1.0 - t
        );
        let sep0 = separation(&reflected(&(x1.r, x1.p)), &(x2.r, x2.p));
        let ends = (reflected(&(r1.out.r, r1.out.p)), (r2.out.r, r2.out.p));
        let mut sep = sep0.max(separation(&ends.0, &ends.1));
        let mut scale = magnitude(&(x1.r, x1.p)).max(magnitude(&ends.1));
        let mut later = r2.samples.iter().peekable();
        for (k, a) in &r1.samples {
            while later.peek().is_some_and(|(j, _)| j < k) {
                later.next();
            }
            if let Some((_, b)) = later.peek().filter(|(j, _)| j == k) {
                let d = separation(&reflected(a), b);
                assert!(
                    d <= fx.bound,
                    "{}: the pair ({s}, {t}), ({s}, {}) is chaotic over the horizon: its state separation {d:e} at \
                     sampling point {k} passes the bound {:e} (at the IC, {sep0:e})",
                    fx.name,
                    1.0 - t,
                    fx.bound
                );
                sep = sep.max(d);
                scale = scale.max(magnitude(a));
            }
        }
        assert!(
            sep <= fx.bound,
            "{}: the pair ({s}, {t}), ({s}, {}) is chaotic over the horizon: its state separation {sep:e} passes the \
             bound {:e}",
            fx.name,
            1.0 - t,
            fx.bound
        );
        let lz = |a: &Phase<f64>| lz_at(&a.0, &a.1);
        let lz_gap = (lz(&(x1.r, x1.p)) + lz(&(x2.r, x2.p)))
            .abs()
            .max((lz(&(r1.out.r, r1.out.p)) + lz(&(r2.out.r, r2.out.p))).abs());
        let lz_tol = lz_tolerance(fx.bound, scale);
        assert!(
            lz_gap <= lz_tol,
            "{}: the pair ({s}, {t}), ({s}, {}): L_z not opposite: {:e} and {:e}, |sum| {lz_gap:e} beyond {lz_tol:e}",
            fx.name,
            1.0 - t,
            lz(&(x1.r, x1.p)),
            lz(&(x2.r, x2.p))
        );
        let rho_min = r1
            .steps
            .iter()
            .chain(&r2.steps)
            .map(|st| (st.0[1][0] - st.0[0][0]).hypot(st.0[1][1] - st.0[0][1]))
            .fold(f64::INFINITY, f64::min);
        let (w1, w2) = (frame_of(&r1), frame_of(&r2));
        let n = r1.steps.len().max(r2.steps.len());
        let w_tol = winding_tolerance(fx.bound, rho_min, n, w1);
        assert!(
            w1.abs() > w_tol,
            "{}: the pair ({s}, {t}): |W| {:e} is within its tolerance {w_tol:e}, so \"opposite\" would be vacuous",
            fx.name,
            w1.abs()
        );
        assert!(
            (w1 + w2).abs() <= w_tol,
            "{}: the pair ({s}, {t}), ({s}, {}): the frame's windings not opposite: {w1:e} and {w2:e}, |sum| {:e} \
             beyond {w_tol:e}",
            fx.name,
            1.0 - t,
            (w1 + w2).abs()
        );
        out.pairs += 1;
        out.separated_at_ic += usize::from(sep0 > 0.0);
        out.sep0_max = out.sep0_max.max(sep0);
        out.sep_max = out.sep_max.max(sep);
        out.terminal += usize::from(f1.0 != STATE_BOUNDED);
        out.lz_min = out.lz_min.min(lz(&(x1.r, x1.p)).abs());
        out.lz_gap = out.lz_gap.max(lz_gap);
        out.winding_min = out.winding_min.min(w1.abs());
        out.winding_gap = out.winding_gap.max((w1 + w2).abs());
    }
    out
}

/// The runs' chart: masses `z_mass = (0.2, −0.3)`, `m ≈ (0.255, 0.685, 0.060)`, `I = 1`, `φ_f = 0.3`, the spherical
/// map, holding `z_mom`.
fn runs_chart(z_mom: [f64; 4]) -> ShapeSphere<f64> {
    ShapeSphere {
        map: ShapeMap::Spherical,
        n0: [0.0, 0.0, 1.0],
        e1: [1.0, 0.0, 0.0],
        e2: [0.0, 1.0, 0.0],
        scale: 1.0,
        inertia: 1.0,
        phase: 0.3,
        z_mass: [0.2, -0.3],
        z_mom,
    }
}

/// The exact arm's pixels, of both hemispheres, off the equator and the poles.
const RUN_PIXELS: [(f64, f64); 5] = [
    (0.15, 0.65),
    (0.25, 0.75),
    (0.75, 0.35),
    (0.85, 0.45),
    (0.95, 0.25),
];

/// The runs' horizon, `T = 20` at [`DT_MACRO`].
const HORIZON: u32 = 20_000;

/// The held-momentum arm's `|L_z|` at every IC is above this, so "`L_z` negated" is not the vacuous negation of 0.
const LZ_MIN: f64 = 0.01;

/// The arm released from rest has `|W|` above this at every run, in radians, so "the winding negated" is not the
/// vacuous negation of 0, and a winding equal across the pair is not negated.
const WINDING_MIN: f64 = 0.01;

/// The kernel's `Π` on a state, `canon::reflect`.
fn kernel_pi<R: Prec>(x: &Ic<R>) -> Ic<R> {
    canon::reflect(x, true)
}

/// `Π`, then `C` with the retired 2-to-1 gauge left in, the retired mirror step: `Π·x` folded back onto `λ̃_y ≥ 0`,
/// which is `x`'s side for an upper-hemisphere `x` (a control's `Π`).
#[cfg(feature = "controls")]
fn gauged_pi(x: &Ic<f64>) -> Ic<f64> {
    let c = canonicalise(&canon::reflect(x, true));
    canon::reflect(&c.ic, c.lambda_t_y < 0.0)
}

/// The default occupant's name in the reports.
const HEGGIE: &str = "Heggie × RK4";

/// The exact arm on `runs_chart(z_mom)`'s [`RUN_PIXELS`] at `R`, under the stepper KDK, RK4 and the default occupant
/// Heggie × RK4, each with its name.
fn exact_occupants<R: Prec>(
    z_mom: [f64; 4],
    pi: fn(&Ic<R>) -> Ic<R>,
    mirror: StateMirror<R>,
    winding: WindingOf<R>,
) -> Vec<(&'static str, Option<Exact>)> {
    let chart = runs_chart(z_mom);
    vec![
        (
            "KDK",
            exact_arm::<Kdk, R>(&chart, &RUN_PIXELS, HORIZON, pi, mirror, winding),
        ),
        (
            "RK4",
            exact_arm::<Rk4, R>(&chart, &RUN_PIXELS, HORIZON, pi, mirror, winding),
        ),
        (
            HEGGIE,
            exact_arm::<HeggieRk4, R>(&chart, &RUN_PIXELS, HORIZON, pi, mirror, winding),
        ),
    ]
}

/// The exact arm's checks on what it measured: the case equivariant; some sampled state with `p_y ≠ 0`, so the
/// configuration-only mirror is seen; under the default occupant, [`HEGGIE`], no step turning `ρ` by more than
/// [`TURN_MAX`], so the winding is the net turn (the unregularised steppers, KDK and RK4, are measured and reported, not
/// held to it: released from rest, a close approach of bodies 0 and 1 turns `ρ` by up to `~2.9` rad in one of their
/// internal steps, which the mirror's negation does not need and the winding's count of turns does); on the held-momentum arm `|L_z| > LZ_MIN` at every IC; released from rest,
/// `|W| > WINDING_MIN` at every run. Returns the report line.
fn exact_report(arm: &str, prec: &str, occupant: &str, e: Option<Exact>, held: bool) -> String {
    let e = e.unwrap_or_else(|| panic!("{arm}: classified as \"does not apply\""));
    assert!(
        e.p_y_states > 0,
        "{arm}, {prec}, {occupant}: no sampled state has p_y ≠ 0"
    );
    if occupant == HEGGIE {
        assert!(
            e.turn_max <= TURN_MAX,
            "{arm}, {prec}, {occupant}: a step turned ρ by {:e}, beyond {TURN_MAX:e}: the winding may alias",
            e.turn_max
        );
    }
    if held {
        assert!(
            e.lz_min > LZ_MIN,
            "{arm}, {prec}, {occupant}: |L_z| {:e} at an IC, not above {LZ_MIN}",
            e.lz_min
        );
    } else {
        assert!(
            e.winding_min > WINDING_MIN,
            "{arm}, {prec}, {occupant}: |W| {:e} at a run, not above {WINDING_MIN}",
            e.winding_min
        );
    }
    format!(
        "{arm}, {prec}, {occupant}: {} runs, {} states each compared, {} with p_y ≠ 0; min |L_z| {:.3e}, min |W| {:.3e}, \
         min |θ̃| {:.3e}, largest per-step turn {:.3e}; {} runs store a state apart from the last the cadence saw; \
         fates (state, detail, t_end_step) {:?}",
        e.runs, e.states, e.p_y_states, e.lz_min, e.winding_min, e.theta_min, e.turn_max, e.stored_apart, e.fates
    )
}

/// The run from `Π·x` is the run from `x` mirrored, bitwise (chart_reference §3.3's whole sphere): released from rest
/// and with a held momentum of zero `y`-components, at f64 and f32, under KDK, RK4 and Heggie × RK4, every state the
/// march left reflected, the outcome and `t_end_step` equal, `L_z` and the frame's winding negated; with a held
/// `p_λ,y ≠ 0` it reports "does not apply" and asserts nothing.
#[test]
fn shape_sphere_mirror_runs() {
    for (z, arm, held) in [
        (REST, "released from rest", false),
        (X_ONLY, "held p, zero y-components", true),
    ] {
        for (occupant, e) in exact_occupants::<f64>(z, kernel_pi, reflected, frame_of) {
            eprintln!(
                "shape_sphere_mirror_runs: {}",
                exact_report(arm, "f64", occupant, e, held)
            );
        }
        for (occupant, e) in exact_occupants::<f32>(z, kernel_pi, reflected, frame_of) {
            eprintln!(
                "shape_sphere_mirror_runs: {}",
                exact_report(arm, "f32", occupant, e, held)
            );
        }
    }
    for (occupant, e) in exact_occupants::<f64>(LAMBDA_Y, kernel_pi, reflected, frame_of) {
        assert!(
            e.is_none(),
            "a held p_λ,y ≠ 0 under {occupant} was not classified \"does not apply\""
        );
        eprintln!("shape_sphere_mirror_runs: held p_λ,y ≠ 0, {occupant}: does not apply, no symmetry asserted");
    }
}

negative_control!(
    shape_sphere_mirror_runs,
    "R-141's gauge left in folds Π·x back onto x's side: on the held-momentum arm L_z is equal, not negated",
    expected = "L_z not negated at the IC",
    {
        exact_arm::<HeggieRk4, f64>(&runs_chart(X_ONLY), &RUN_PIXELS, HORIZON, gauged_pi, reflected, frame_of);
    }
);

mod mirror_runs_gauge_rest {
    use super::*;
    negative_control!(
        shape_sphere_mirror_runs,
        "R-141's gauge left in, released from rest, where L_z ≡ 0 at the IC: the winding is equal, not negated",
        expected = "the frame's winding not negated",
        {
            exact_arm::<HeggieRk4, f64>(&runs_chart(REST), &RUN_PIXELS, HORIZON, gauged_pi, reflected, frame_of);
        }
    );
}

mod mirror_runs_configuration_only {
    use super::*;
    negative_control!(
        shape_sphere_mirror_runs,
        "a configuration-only mirror in the every-state comparison, where the run's momenta have y-components, leaves \
         p_y unreflected",
        expected = "not reflected",
        {
            exact_arm::<Rk4, f64>(
                &runs_chart(X_ONLY),
                &RUN_PIXELS,
                HORIZON,
                kernel_pi,
                |s| (reflected(s).0, s.1),
                frame_of,
            );
        }
    );
}

/// The pixel pairs' separation bound: a pair whose state separation, at rounding level at the IC (the decode's
/// `~10⁻¹⁵`), stays below `10⁻⁹` over the horizon has amplified it by at most `~10⁶`, so its fates are the system's,
/// not the rounding's.
const SEP_BOUND: f64 = 1e-9;

/// The named fixture released from rest: [`runs_chart`] at `z_mom = 0`, its pairs ending in collisions (pairs 2 and
/// 1) and bounded at the horizon, `T = 20`.
fn rest_fixture() -> PairFixture {
    PairFixture {
        name: "the rest fixture",
        chart: runs_chart(REST),
        pairs: &[
            (0.25, 0.75),
            (0.75, 0.65),
            (0.15, 0.65),
            (0.85, 0.55),
            (0.05, 0.85),
        ],
        horizon: HORIZON,
        bound: SEP_BOUND,
    }
}

/// The named fixture with a held momentum: [`runs_chart`] at [`X_ONLY`], its pairs ending in triple ejections
/// (escape, detail 3) and bounded at the horizon, `T = 20`.
fn held_fixture() -> PairFixture {
    PairFixture {
        name: "the held fixture",
        chart: runs_chart(X_ONLY),
        pairs: &[(0.65, 0.55), (0.75, 0.75), (0.25, 0.65), (0.55, 0.65)],
        horizon: HORIZON,
        bound: SEP_BOUND,
    }
}

/// The pixel pairs `(θ, φ)` and `(θ, π − φ)` on the named fixtures, by Heggie × RK4 at f64: shown non-chaotic over the
/// horizon by the separation bound, their outcome and `t_end_step` equal, `L_z` and the frame's winding opposite to
/// the bound's tolerance. Each fixture has a pair separated at the IC (else the arm repeats the exact one), a pair
/// ending in a terminal event, and the non-vacuity bounds: `|L_z| > LZ_MIN` with the held momentum, `|W| > WINDING_MIN`
/// released from rest.
#[test]
fn shape_sphere_mirror_runs_pairs() {
    for (fx, held) in [(rest_fixture(), false), (held_fixture(), true)] {
        let p = pair_arm::<HeggieRk4>(&fx, decode_ic);
        assert!(
            p.separated_at_ic > 0,
            "{}: every pair is bitwise mirrored at the IC",
            fx.name
        );
        assert!(
            p.terminal > 0,
            "{}: no pair ends in a terminal event",
            fx.name
        );
        if held {
            assert!(
                p.lz_min > LZ_MIN,
                "{}: |L_z| {:e}, not above {LZ_MIN}",
                fx.name,
                p.lz_min
            );
        } else {
            assert!(
                p.winding_min > WINDING_MIN,
                "{}: |W| {:e}, not above {WINDING_MIN}",
                fx.name,
                p.winding_min
            );
        }
        eprintln!(
            "shape_sphere_mirror_runs_pairs: {}: {} pairs ({} separated at the IC, {} terminal); state separation at \
             the IC ≤ {:.2e}, over the horizon ≤ {:.2e} against the bound {:e}; |L_z(a) + L_z(b)| ≤ {:.2e} (min |L_z| \
             {:.3e}); |W(a) + W(b)| ≤ {:.2e} (min |W| {:.3e})",
            fx.name,
            p.pairs,
            p.separated_at_ic,
            p.terminal,
            p.sep0_max,
            p.sep_max,
            fx.bound,
            p.lz_gap,
            p.lz_min,
            p.winding_gap,
            p.winding_min
        );
    }
}

negative_control!(
    shape_sphere_mirror_runs_pairs,
    "a fixture whose pair separation passes its bound over the horizon, the rest chart's (0.05, 0.75) at T = 65.535, \
     is refused as chaotic",
    expected = "is chaotic over the horizon",
    {
        let fx = PairFixture {
            name: "the long fixture",
            chart: runs_chart(REST),
            pairs: &[(0.05, 0.75)],
            horizon: 65_535,
            bound: SEP_BOUND,
        };
        pair_arm::<HeggieRk4>(&fx, decode_ic);
    }
);

mod mirror_runs_pairs_px_turned {
    use super::*;
    negative_control!(
        shape_sphere_mirror_runs_pairs,
        "a momentum rule turning p_x below the equator gives the lower pixel another system: its fate differs",
        expected = "the fates differ",
        {
            pair_arm::<HeggieRk4>(&held_fixture(), |c, u, v| {
                let z = c.z_mom;
                let held = ShapeSphere {
                    z_mom: if c.point(u, v)[2] < 0.0 { [-z[0], z[1], -z[2], z[3]] } else { z },
                    ..*c
                };
                decode_ic(&held, u, v)
            });
        }
    );
}

// ── The frame's winding (chart_reference §3.3) ─────────────────────────────────────────────────────────────────────

/// `θ̃`, the wrapper's unwrapped shape longitude, in the winding's place (a control's).
#[cfg(feature = "controls")]
fn theta_of<R: Prec>(run: &Run<R>) -> R {
    run.out.theta
}

/// The run from `Π·x` winds the negation of the run from `x`'s winding, bitwise where nonzero and by value where zero,
/// released from rest and with the held momentum, under KDK and Heggie × RK4 at f64 and f32; `θ̃` is nonzero at every
/// run, so its control can fail.
#[test]
fn frame_winding_runs() {
    for (z, arm, held) in [
        (REST, "released from rest", false),
        (X_ONLY, "held p, zero y-components", true),
    ] {
        let chart = runs_chart(z);
        let pixels = &RUN_PIXELS[..3];
        for (prec, occupant, e) in [
            (
                "f64",
                "KDK",
                exact_arm::<Kdk, f64>(&chart, pixels, HORIZON, kernel_pi, reflected, frame_of),
            ),
            (
                "f64",
                HEGGIE,
                exact_arm::<HeggieRk4, f64>(
                    &chart, pixels, HORIZON, kernel_pi, reflected, frame_of,
                ),
            ),
            (
                "f32",
                HEGGIE,
                exact_arm::<HeggieRk4, f32>(
                    &chart, pixels, HORIZON, kernel_pi, reflected, frame_of,
                ),
            ),
        ] {
            let e = e.expect("equivariant");
            assert!(
                e.theta_min > 0.0,
                "{arm}, {prec}, {occupant}: a run with θ̃ = 0"
            );
            eprintln!(
                "frame_winding_runs: {}",
                exact_report(arm, prec, occupant, Some(e), held)
            );
        }
    }
}

negative_control!(
    frame_winding_runs,
    "θ̃, the shape sphere's longitude, in the winding's place is mirror-invariant: equal, not negated",
    expected = "the frame's winding not negated",
    {
        exact_arm::<HeggieRk4, f64>(&runs_chart(REST), &RUN_PIXELS[..3], HORIZON, kernel_pi, reflected, theta_of);
    }
);

mod frame_winding_runs_aliasing {
    use super::*;
    negative_control!(
        frame_winding_runs,
        "the per-step turn guard held to KDK, released from rest, where a close approach of bodies 0 and 1 turns ρ by \
         ~2.9 rad in one internal step, fires",
        expected = "the winding may alias",
        {
            let e = exact_arm::<Kdk, f64>(&runs_chart(REST), &RUN_PIXELS, HORIZON, kernel_pi, reflected, frame_of);
            exact_report("released from rest", "f64", HEGGIE, e, false);
        }
    );
}

/// A winding over a sequence of `ρ`: the kernel's, or a control's.
type WindRho<R> = fn(&[[R; 2]]) -> R;

/// The kernel's winding over `ρ`'s sequence, through [`Winding`] on positions with body 0 at the origin and body 1 at
/// `ρ`, so `r₁ − r₀` is `ρ` exactly (`x − 0 = x`, `±0 − (+0)` keeps the sign).
fn kernel_wind<R: Prec>(rho: &[[R; 2]]) -> R {
    let at = |v: [R; 2]| [[R::of(0.0); 2], v, [R::of(0.0); 2]];
    let mut w = Winding::new(&at(rho[0]));
    for &v in &rho[1..] {
        w.step(&at(v));
    }
    w.total
}

/// The tie left at `atan2(+0, d < 0) = +π`, the wrap dd_integrator §3.7 closes at `+π` (a control's winding).
#[cfg(feature = "controls")]
fn tie_at_pi<R: Prec>(rho: &[[R; 2]]) -> R {
    rho.windows(2).fold(R::of(0.0), |w, p| {
        let c = p[0][0] * p[1][1] - p[0][1] * p[1][0];
        let d = p[0][0] * p[1][0] + p[0][1] * p[1][1];
        w + Real::arctan2(c, d)
    })
}

/// A principal-value difference of angles, `wrap(atan2(ρ_y, ρ_x) − atan2(ρ_prev,y, ρ_prev,x))`, with no rule for the
/// negative `x`-axis (a control's winding).
#[cfg(feature = "controls")]
fn principal<R: Prec>(rho: &[[R; 2]]) -> R {
    rho.windows(2).fold(R::of(0.0), |w, p| {
        w + kernel::shape::wrap(Real::arctan2(p[1][1], p[1][0]) - Real::arctan2(p[0][1], p[0][0]))
    })
}

/// `ρ`'s sequence mirrored, `y → −y`, a zero `y` kept as `+0`, as a run computes an exact zero in both runs.
fn mirror_rho<R: Prec>(rho: &[[R; 2]]) -> Vec<[R; 2]> {
    rho.iter()
        .map(|&[x, y]| {
            [
                x,
                if Into::<f64>::into(y) == 0.0 {
                    R::of(0.0)
                } else {
                    -y
                },
            ]
        })
        .collect()
}

fn at<R: Prec>(v: &[[f64; 2]]) -> Vec<[R; 2]> {
    v.iter().map(|&[x, y]| [R::of(x), R::of(y)]).collect()
}

/// The synthetic sequences at `R` under `wind`, each against its mirror: a sequence starting on the negative `x`-axis
/// (`ρ_y = +0` in it and its mirror), chosen where a principal-value difference of angles is not negated; the exactly
/// antiparallel step `(1, 1) → (−1, −1)`, its mirror `(1, −1) → (−1, 1)`, the tie, continued one step so the total is
/// nonzero; `ρ` held on the `x`-axis, mirrored with `−0` and with `+0`, a total of `+0` by value; `ρ` through zero, the
/// vanishing vector, which adds `+0`; and random walks of turns below `π/2`. Each mirror winds the negation, bits where
/// nonzero and by value where zero. Returns the count of sequences checked.
fn check_synthetic<R: Prec>(wind: WindRho<R>) -> usize {
    let check = |name: &str, rho: &[[R; 2]], mirror: &[[R; 2]]| {
        let (w, m) = (wind(rho), wind(mirror));
        assert!(
            negation(w, m),
            "the {name} sequence's mirror does not wind its negation: {w:?} ({:#x}) and {m:?} ({:#x})",
            bits_of(w),
            bits_of(m)
        );
        w
    };
    let negative_x = at::<R>(&[[-1.688, 0.0], [-0.269, 1.049], [-1.992, -0.218]]);
    check("negative x-axis", &negative_x, &mirror_rho(&negative_x));
    let tie = at::<R>(&[[1.0, 1.0], [-1.0, -1.0], [-1.0, -2.0]]);
    let w = check("tie", &tie, &mirror_rho(&tie));
    assert!(Into::<f64>::into(w) != 0.0, "the tie sequence winds 0");
    let axis = at::<R>(&[[1.0, 0.0], [2.0, 0.0], [3.0, 0.0], [0.5, 0.0]]);
    let negated: Vec<[R; 2]> = axis.iter().map(|&[x, y]| [x, -y]).collect();
    let w = check("x-axis", &axis, &mirror_rho(&axis));
    check("x-axis (−0)", &axis, &negated);
    assert!(
        Into::<f64>::into(w) == 0.0,
        "ρ held on the x-axis winds {w:?}"
    );
    let vanishing = at::<R>(&[[1.0, 1.0], [0.0, 0.0], [-1.0, 2.0]]);
    let w = check("vanishing", &vanishing, &mirror_rho(&vanishing));
    assert!(Into::<f64>::into(w) == 0.0, "ρ through zero winds {w:?}");
    let mut rng = SplitMix::new(0x3803);
    for k in 0..200 {
        let mut v = vec![[on(&mut rng, -2.0, 2.0), on(&mut rng, -2.0, 2.0)]];
        for _ in 0..50 {
            let [x, y] = v[v.len() - 1];
            let (a, s) = (on(&mut rng, -1.5, 1.5), on(&mut rng, 0.5, 1.5));
            v.push([
                s * (x * a.cos() - y * a.sin()),
                s * (x * a.sin() + y * a.cos()),
            ]);
        }
        let rho = at::<R>(&v);
        check(&format!("random walk {k}"), &rho, &mirror_rho(&rho));
    }
    204
}

/// `n` steps of a rigid rotation through `2πk` of a scalene configuration, at `R`: positions at each step, rotated in
/// f64 and rounded once.
fn rotation<R: Prec>(k: i32, n: u32) -> Vec<[[R; 2]; 3]> {
    let base = [[-0.7, 0.2], [0.5, -0.1], [0.3, 0.6]];
    (0..=n)
        .map(|j| {
            let a = 2.0 * PI * f64::from(k) * f64::from(j) / f64::from(n);
            base.map(|[x, y]| {
                [
                    R::of(x * a.cos() - y * a.sin()),
                    R::of(x * a.sin() + y * a.cos()),
                ]
            })
        })
        .collect()
}

/// A winding over positions: the kernel's frame's winding, or a control's.
type WindPositions<R> = fn(&[[[R; 2]; 3]]) -> R;

fn kernel_wind_positions<R: Prec>(r: &[[[R; 2]; 3]]) -> R {
    let mut w = Winding::new(&r[0]);
    for x in &r[1..] {
        w.step(x);
    }
    w.total
}

/// A rigid rotation through `2πk` winds `k` turns: `|W − 2πk| ≤ 2n·ε·(8 + 2π|k|)` over `n` steps, each turn within a
/// few roundings of the rounded positions' (`8ε`) and each sum within `ε·|W|`, doubled. For `k ∈ {1, 2, 3, −1, −4}`,
/// at 3, 7, 61 and 64 steps per turn (64's turn, `π/32`, is dyadic, so its f64 sums can be exact; 61's
/// is not). Returns the worst error in units of `ε`.
fn check_rotation<R: Prec>(wind: WindPositions<R>) -> f64 {
    let eps: f64 = Into::<f64>::into(R::epsilon());
    let mut worst = 0.0f64;
    for k in [1i32, 2, 3, -1, -4] {
        for per_turn in [3u32, 7, 61, 64] {
            let n = per_turn * k.unsigned_abs();
            let w: f64 = wind(&rotation::<R>(k, n)).into();
            let want = 2.0 * PI * f64::from(k);
            let tol = 2.0 * f64::from(n) * eps * (8.0 + 2.0 * PI * f64::from(k.abs()));
            assert!(
                (w - want).abs() <= tol,
                "a rigid rotation through 2π·{k} in {n} steps winds {w}, not {k} turns ({want}), beyond {tol:e}"
            );
            worst = worst.max((w - want).abs() / eps);
        }
    }
    worst
}

/// The frame's winding on its definition's cases (chart_reference §3.3), at f64 and f32: the synthetic sequences
/// against their mirrors ([`check_synthetic`]) and a rigid rotation through `2πk` winding `k` turns ([`check_rotation`]).
#[test]
fn frame_winding() {
    let n = check_synthetic::<f64>(kernel_wind) + check_synthetic::<f32>(kernel_wind);
    let (a, b) = (
        check_rotation::<f64>(kernel_wind_positions),
        check_rotation::<f32>(kernel_wind_positions),
    );
    eprintln!("frame_winding: {n} sequences each wind the negation under the mirror; rotations wind k turns within \
               {a:.1} ε (f64) and {b:.1} ε (f32)");
}

negative_control!(
    frame_winding,
    "the tie left at atan2(+0, d < 0) = +π, the wrap closed at +π: the tie sequence and its mirror both wind +π",
    expected = "the tie sequence's mirror does not wind its negation",
    {
        check_synthetic::<f64>(tie_at_pi);
    }
);

mod frame_winding_principal {
    use super::*;
    negative_control!(
        frame_winding,
        "a principal-value difference of angles, no rule on the negative x-axis: not negated there",
        expected = "the negative x-axis sequence's mirror does not wind its negation",
        {
            check_synthetic::<f64>(principal);
        }
    );
}

mod frame_winding_theta {
    use super::*;
    negative_control!(
        frame_winding,
        "θ̃, the shape's longitude, in the winding's place: a rigid rotation leaves the shape and θ̃ unchanged",
        expected = "winds",
        {
            check_rotation::<f64>(|r| {
                let m = [0.2, 0.5, 0.3];
                r.windows(2).fold(0.0, |w, p| {
                    w + kernel::shape::wrap(
                        kernel::shape::longitude(shape(p[1], m)) - kernel::shape::longitude(shape(p[0], m)),
                    )
                })
            });
        }
    );
}
