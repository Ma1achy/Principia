//! QA tests for TASK-M2-08, written from the requirements and their sources, not from the implementation: the shape
//! map `n` (`kernel::shape::shape`, `shape_vec`), the shape-sphere chart's maps and closed-form inverse
//! (`kernel::chart::shape_sphere`) and the landmarks (`kernel::shape::landmarks`).
//!
//! Every expected value comes from an independent construction: [`reference_n`], chart_reference §3.1's map written
//! from the text on the mass-weighted Jacobi vectors of §0.2 at any total mass; the signed area for the sign of `w`
//! (`ρ ∧ λ` is twice the signed area of the triangle `r₀ r₁ r₂`); closed forms of the mass-weighted collision points
//! (a collinear configuration with the pair together, [`analytic_bc`]); §0.2's position reconstruction written here
//! ([`reconstruct`]); and the IC Inspector's own numbers in `fixtures/ic_inspector/shape_vec.json`.
//! - REQ-CHART-018 (`shape_vec_qa_l_plus_bc01`, `shape_vec_qa_reference`, `shape_vec_qa_inspector`): `L⁺` (bodies
//!   0 → 1 → 2 anticlockwise, equilateral, equal masses) is `w = +1` and `L⁻` is `w = −1` under any rotation, translation
//!   and scale; the collision of bodies 0 and 1 is `(−1, 0, 0)` at any masses; `n` agrees with the statement's map on
//!   random ICs and with the IC Inspector's JS on the fixture's random ICs.
//! - REQ-CHART-019 (`shape_sphere_chart_map_qa_spherical`, `shape_sphere_chart_map_qa_exponential`): `(s, 1)` is `L⁺`,
//!   `(s, 0)` is `L⁻`, the polar angle is `π(1 − t)` over the whole `[0, π]` (no cap excised next to either pole) and the
//!   azimuth `2πs`; the exponential map at `d = 0` is `n₀` exactly and lies at geodesic distance `‖d‖` along `d`.
//! - REQ-CHART-020, REQ-CHART-053 (`shape_sphere_roundtrip_qa_chart`, `shape_sphere_roundtrip_qa_closed_form`,
//!   `shape_sphere_roundtrip_qa_poles`): `Φ` alone (R-178), `shape(Φ(u, v)) = n(u, v)` within `shape_roundtrip_bound` on
//!   both hemispheres through the chart's own decode; the inverse is §3.2's closed form, `‖ρ̃‖² = I(1 + n₀)/2`,
//!   `‖λ̃‖² = I(1 − n₀)/2`, `ρ̃` at `φ_f`, `λ̃` at `φ_f + atan2(q, p)`; at `n₀ = ±1` with `±0` in `n₁`, `n₂` the `atan2` term
//!   is `+0` and `ψ = φ_f` (R-437). The gate's fixture holds the register's bound (`shape_sphere_roundtrip_qa_gate_json`).
//!   `Φ` is the decode's blocks (its `masses`, `config` and `momentum`), not its `ic`, which is `C`'s (below).
//! - REQ-CHART-020, R-141 (`shape_sphere_canonical_fold_qa`): `C(Φ(s, t))` is at `(θ, π − φ)` on the lower hemisphere,
//!   at `(θ, φ)` on the upper, and at the default gauge (`I = 1`, `φ_f = 0`) an upper-hemisphere `Φ` is on `C`'s section,
//!   so `C` leaves it.
//! - dd_decoder §2, §3.3, §3.5, §3.6; REQ-CHART-020, R-544 (`shape_sphere_decode_canonical_qa`,
//!   `shape_sphere_energy_norm_qa`): the decode's `ic` is canonical, `G = M = I = 1`, at any chart `I` and on both
//!   hemispheres, with `C`'s step 3, the mirror, not taken (R-544): the CoM at rest at the origin, `I = 1`, `ρ` on
//!   `+x`, `λ̃_y` keeping `n₂`'s sign, the masses `m_fixed`, the shape the signed `n(u, v)`, the energy `√I·E(Φ)` and
//!   `|L_z|` `I^{−1/4}·|L_z(Φ)|` (§3.5's similarity), the descriptor read from it; `η_E`'s entry decodes the chart's
//!   signed shape, with no target as the chart's own decode, with `E*` at the same positions and energy `E*`.
//! - REQ-CHART-036, REQ-INT-003 (`shape_landmarks_qa_equal_masses`, `shape_landmarks_mass_weighted_qa`,
//!   `shape_landmarks_qa_inspector`): equal masses, `BC₀₁ = (−1, 0, 0)` at 180°, `BC₁₂` at 60°, `BC₂₀` at 300°, 120°
//!   apart, `L± = (0, 0, ±1)`; random masses, `BC₀₁ = (−1, 0, 0)`, every collision on `w = 0` at its mass-weighted
//!   point; the collisions against the inspector's `collisionPhi`.
//! - REQ-VAL-023 (`shape_dot_on_collision_landmark_qa`): property, mass ratios up to `e⁸`, bodies `i` and `j` brought
//!   together along a straight path: the dot's distance to the landmark opposite `k` falls with the separation, by
//!   decades to coincidence, and its distance to the two other landmarks stays above half their separation.
//! - REQ-CHART-049 (`shape_vec_qa_fixture_provenance`): the fixture names the `.nvmrc` Node version and the inspector as
//!   its source (R-169, R-184).
//!
//! **Not asserted** (held open): `L±` off the poles at unequal masses (RQ-355); only `L⁺`'s orientation, `w > 0`, is
//! asserted there.
//!
//! **Tolerances.** The agreement of `n` computed by two routes (the inspector's, the closed-form collision points', the
//! statement's reference) is the register's `shape_vec_js_tol` (REQ-CHART-049); the round trip and the fold, which
//! recover a point of the sphere from `Φ`'s configuration, are within the register's `shape_roundtrip_bound`
//! (REQ-CHART-053, R-181). The chart map's own identities are a multiple of ε from the handful of rounded operations on
//! their path (applied per R-369, a test decision). Exact values (`(−1, 0, 0)` at `ρ̃ = 0`, `n₀` at `d = 0`, `+0` at the
//! poles) are asserted exactly.
//!
//! **Domain** (pitfalls §10): the map, the chart and `C` on states held fixed; no trajectory.
//!
//! Each test has a registered negative control (R-176).

use std::path::Path;

use kernel::canon::canonicalise;
use kernel::chart::shape_sphere::{exponential, inverse, spherical, ShapeMap, ShapeSphere};
use kernel::chart::{self, Chart};
use kernel::decode::energy_norm::{self, Outcome};
use kernel::decode::{Decoded, Ic};
use kernel::generated::links::Defaults;
use kernel::shape::{landmarks, shape, shape_vec, weighted_jacobi, Landmarks};
use proptest::prelude::*;
use validation::decode_f32_eps::SplitMix;
use validation::{negative_control, prop, SHAPE_ROUNDTRIP_BOUND, SHAPE_VEC_JS_TOL};

const PI: f64 = std::f64::consts::PI;
const EPS: f64 = f64::EPSILON;

/// `shape_vec_js_tol` (REQ-CHART-049), provisional until M2: the agreement of `n` by two routes.
const TWO_ROUTES: f64 = SHAPE_VEC_JS_TOL.number();

/// `shape_roundtrip_bound` (REQ-CHART-053, R-181), provisional until M2.
const ROUNDTRIP: f64 = SHAPE_ROUNDTRIP_BOUND.number();

/// The chart map's identities: a sine, a cosine and a product or two, each within an ulp, `π` at most in the angle.
const MAP_TOL: f64 = 8.0 * EPS;

type Pos = [[f64; 2]; 3];
type NFn = fn(Pos, [f64; 3]) -> [f64; 3];

// ── Independent constructions ───────────────────────────────────────────────────────────────────────────────────────

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

fn component(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|k| (a[k] - b[k]).abs()).fold(0.0, f64::max)
}

fn on(rng: &mut SplitMix, lo: f64, hi: f64) -> f64 {
    lo + (hi - lo) * ((rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64)
}

/// Positive masses with `Σmᵢ = 1`, each ratio up to `e^(2·spread)`.
fn masses(rng: &mut SplitMix, spread: f64) -> [f64; 3] {
    let w = [0, 1, 2].map(|_| on(rng, -spread, spread).exp());
    let t = w[0] + w[1] + w[2];
    w.map(|x| x / t)
}

/// chart_reference §3.1 from the text, on §0.2's mass-weighted Jacobi vectors at any total mass `M`:
/// `μ_ρ = m₀m₁/M₀₁`, `μ_λ = m₂M₀₁/M`, `ρ = r₁ − r₀`, `λ = r₂ − (m₀r₀ + m₁r₁)/M₀₁`; `a = ‖ρ̃‖²`, `b = ‖λ̃‖²`,
/// `p = ρ̃·λ̃`, `q = ρ̃ₓλ̃_y − ρ̃_yλ̃ₓ` (standard cross, R-14), `n = ((a − b)/I, 2p/I, 2q/I)`.
fn reference_n(r: Pos, m: [f64; 3]) -> [f64; 3] {
    let total = m[0] + m[1] + m[2];
    let m01 = m[0] + m[1];
    let (srho, slam) = ((m[0] * m[1] / m01).sqrt(), (m[2] * m01 / total).sqrt());
    let rho = [r[1][0] - r[0][0], r[1][1] - r[0][1]];
    let c = [
        (m[0] * r[0][0] + m[1] * r[1][0]) / m01,
        (m[0] * r[0][1] + m[1] * r[1][1]) / m01,
    ];
    let lam = [r[2][0] - c[0], r[2][1] - c[1]];
    let (rt, lt) = (
        [srho * rho[0], srho * rho[1]],
        [slam * lam[0], slam * lam[1]],
    );
    let a = rt[0] * rt[0] + rt[1] * rt[1];
    let b = lt[0] * lt[0] + lt[1] * lt[1];
    let p = rt[0] * lt[0] + rt[1] * lt[1];
    let q = rt[0] * lt[1] - rt[1] * lt[0];
    let i = a + b;
    [(a - b) / i, 2.0 * p / i, 2.0 * q / i]
}

/// The superseded convention, `q` the negative of the cross (chart_reference §3.1's earlier version): a control.
fn negative_cross_n(r: Pos, m: [f64; 3]) -> [f64; 3] {
    let n = reference_n(r, m);
    [n[0], n[1], -n[2]]
}

/// Twice the signed area of the triangle `r₀ r₁ r₂`, positive when anticlockwise.
fn twice_area(r: Pos) -> f64 {
    (r[1][0] - r[0][0]) * (r[2][1] - r[0][1]) - (r[1][1] - r[0][1]) * (r[2][0] - r[0][0])
}

/// The unit equilateral triangle, bodies 0 → 1 → 2 anticlockwise for `sign = 1`, clockwise for `−1`.
fn equilateral(sign: f64) -> Pos {
    [[0.0, 0.0], [1.0, 0.0], [0.5, sign * 3f64.sqrt() / 2.0]]
}

/// `r` turned by `a`, scaled by `k` and moved by `t`: a configuration of the same shape.
fn moved(r: Pos, a: f64, k: f64, t: [f64; 2]) -> Pos {
    let (s, c) = a.sin_cos();
    r.map(|x| {
        [
            k * (c * x[0] - s * x[1]) + t[0],
            k * (s * x[0] + c * x[1]) + t[1],
        ]
    })
}

/// The mass-weighted collision points in closed form, `bc[k]` the collision of the side opposite body `k`: put the pair
/// at the origin and the third body at distance 1 on `+x`. With `μ_ρ = m₀m₁/M₀₁` and `μ_λ = m₂M₀₁/M`:
/// - `BC₁₂` (`r₁ = r₂`, `ρ = λ·M₀₁/m₀`): `a = μ_ρ`, `b = μ_λ(m₀/M₀₁)²`, `ρ̃ ∥ λ̃` along, so `p = +√(ab)`;
/// - `BC₂₀` (`r₀ = r₂`, `λ = −(m₁/M₀₁)ρ`): `a = μ_ρ`, `b = μ_λ(m₁/M₀₁)²`, opposite, `p = −√(ab)`;
/// - `BC₀₁` (`ρ = 0`): `(−1, 0, 0)`.
///
/// `n = ((a − b)/(a + b), 2p/(a + b), 0)`, `q = 0` for a collinear configuration.
fn analytic_bc(m: [f64; 3]) -> [[f64; 3]; 3] {
    let total = m[0] + m[1] + m[2];
    let m01 = m[0] + m[1];
    let mu_rho = m[0] * m[1] / m01;
    let mu_lam = m[2] * m01 / total;
    let point = |a: f64, b: f64, sign: f64| {
        [
            (a - b) / (a + b),
            sign * 2.0 * (a * b).sqrt() / (a + b),
            0.0,
        ]
    };
    [
        point(mu_rho, mu_lam * (m[0] / m01).powi(2), 1.0),
        point(mu_rho, mu_lam * (m[1] / m01).powi(2), -1.0),
        [-1.0, 0.0, 0.0],
    ]
}

/// chart_reference §0.2's reconstruction, from the text: `ρ = ρ̃/√μ_ρ`, `λ = λ̃/√μ_λ`, `r₀₁ = −m₂λ`, `r₂ = M₀₁λ`,
/// `r₀ = r₀₁ − (m₁/M₀₁)ρ`, `r₁ = r₀₁ + (m₀/M₀₁)ρ` (`M = 1`).
fn reconstruct(m: [f64; 3], rho_t: [f64; 2], lambda_t: [f64; 2]) -> Pos {
    let m01 = m[0] + m[1];
    let (srho, slam) = ((m[0] * m[1] / m01).sqrt(), (m[2] * m01).sqrt());
    let rho = rho_t.map(|x| x / srho);
    let lam = lambda_t.map(|x| x / slam);
    let r01 = lam.map(|x| -m[2] * x);
    [
        [r01[0] - m[1] / m01 * rho[0], r01[1] - m[1] / m01 * rho[1]],
        [r01[0] + m[0] / m01 * rho[0], r01[1] + m[0] / m01 * rho[1]],
        lam.map(|x| m01 * x),
    ]
}

/// The angle between `x` and `y` wrapped into `[0, π]`.
fn angle_gap(x: f64, y: f64) -> f64 {
    let d = (x - y).rem_euclid(2.0 * PI);
    d.min(2.0 * PI - d)
}

fn azimuth_deg(n: [f64; 3]) -> f64 {
    n[1].atan2(n[0]).to_degrees().rem_euclid(360.0)
}

// ── REQ-CHART-018: the forward map ──────────────────────────────────────────────────────────────────────────────────

/// Under the map `n_of` at equal masses: `L⁺` (anticlockwise) is `(0, 0, +1)`, `L⁻` `(0, 0, −1)`, under random
/// rotations, translations and scales; `w` has the sign of the signed area at any masses; the collision of 0 and 1 is
/// `(−1, 0, 0)` exactly at any masses. Returns the worst pole miss.
fn check_l_plus_bc01(n_of: NFn) -> f64 {
    let mut rng = SplitMix::new(0x2008);
    let equal = [1.0 / 3.0; 3];
    let mut worst: f64 = 0.0;
    for _ in 0..256 {
        let (a, k, t) = (
            on(&mut rng, -PI, PI),
            on(&mut rng, 1e-3, 1e3),
            [on(&mut rng, -5.0, 5.0), on(&mut rng, -5.0, 5.0)],
        );
        for (sign, pole) in [(1.0, [0.0, 0.0, 1.0]), (-1.0, [0.0, 0.0, -1.0])] {
            let n = n_of(moved(equilateral(sign), a, k, t), equal);
            let miss = dist(n, pole);
            assert!(
                miss <= TWO_ROUTES,
                "L{} at equal masses is {n:?}, not {pole:?} (miss {miss:e})",
                if sign > 0.0 { "⁺" } else { "⁻" }
            );
            worst = worst.max(miss);
        }
        let m = masses(&mut rng, 4.0);
        let r = [0, 1, 2].map(|_| [on(&mut rng, -2.0, 2.0), on(&mut rng, -2.0, 2.0)]);
        let area = twice_area(r);
        let n = n_of(r, m);
        if area.abs() > 1e-3 {
            assert!(
                n[2].signum() == area.signum(),
                "w = {} does not have the sign of the signed area {area} (masses {m:?})",
                n[2]
            );
        }
        let p = r[0];
        let n = n_of([p, p, r[2]], m);
        assert!(
            n[0] == -1.0 && n[1] == 0.0 && n[2] == 0.0,
            "the collision of bodies 0 and 1 is {n:?}, not (−1, 0, 0) (masses {m:?})"
        );
    }
    worst
}

fn kernel_shape(r: Pos, m: [f64; 3]) -> [f64; 3] {
    shape(r, m)
}

fn kernel_shape_vec(r: Pos, m: [f64; 3]) -> [f64; 3] {
    let (rt, lt) = weighted_jacobi(r, m);
    shape_vec(rt, lt)
}

#[test]
fn shape_vec_qa_l_plus_bc01() {
    let a = check_l_plus_bc01(kernel_shape);
    let b = check_l_plus_bc01(kernel_shape_vec);
    // chart_reference §3.1: a map built to the superseded negative cross has its third component negated, `L⁺` south.
    let old = negative_cross_n(equilateral(1.0), [1.0 / 3.0; 3]);
    assert!(
        old[2] < 0.0 && shape(equilateral(1.0), [1.0 / 3.0; 3])[2] > 0.0,
        "the two conventions agree on L⁺"
    );
    eprintln!("shape_vec_qa_l_plus_bc01: worst pole miss {:.3e}", a.max(b));
}

negative_control!(
    shape_vec_qa_l_plus_bc01,
    "the superseded negative-cross map puts L⁺ at the south pole, so it must fail",
    expected = "at equal masses is",
    {
        check_l_plus_bc01(negative_cross_n);
    }
);

/// `n_of` against the statement's map [`reference_n`] on random ICs, within the two-route tolerance; returns the
/// worst component difference.
fn check_reference(n_of: NFn) -> f64 {
    let mut rng = SplitMix::new(0x2018);
    let mut worst: f64 = 0.0;
    for _ in 0..4096 {
        let m = masses(&mut rng, 4.0);
        let r = [0, 1, 2].map(|_| [on(&mut rng, -3.0, 3.0), on(&mut rng, -3.0, 3.0)]);
        let (got, want) = (n_of(r, m), reference_n(r, m));
        let d = component(got, want);
        assert!(
            d <= TWO_ROUTES,
            "n {got:?} differs from the statement's map {want:?} by {d:e} (masses {m:?}, r {r:?})"
        );
        let norm = (got[0] * got[0] + got[1] * got[1] + got[2] * got[2]).sqrt();
        assert!((norm - 1.0).abs() <= TWO_ROUTES, "‖n‖ = {norm}, not 1");
        worst = worst.max(d);
    }
    worst
}

#[test]
fn shape_vec_qa_reference() {
    let w = check_reference(kernel_shape).max(check_reference(kernel_shape_vec));
    eprintln!("shape_vec_qa_reference: worst {w:.3e} ({:.1} ε)", w / EPS);
}

negative_control!(
    shape_vec_qa_reference,
    "unweighted Jacobi vectors (μ_ρ = μ_λ = 1) give another point off equal masses, so they must fail",
    expected = "differs from the statement's map",
    {
        check_reference(|r, _| {
            let rho = [r[1][0] - r[0][0], r[1][1] - r[0][1]];
            let c = [(r[0][0] + r[1][0]) / 2.0, (r[0][1] + r[1][1]) / 2.0];
            let lam = [r[2][0] - c[0], r[2][1] - c[1]];
            shape_vec(rho, lam)
        });
    }
);

// ── REQ-CHART-018, REQ-CHART-036, REQ-CHART-049: the IC Inspector's numbers ─────────────────────────────────────────

fn bits(v: &serde_json::Value) -> f64 {
    let s = v.as_str().expect("a hex string");
    f64::from_bits(u64::from_str_radix(s, 16).expect("hex bits"))
}

fn three(v: &serde_json::Value) -> [f64; 3] {
    [0, 1, 2].map(|k| bits(&v[k]))
}

fn positions(v: &serde_json::Value) -> Pos {
    [0, 1, 2].map(|k| [bits(&v[k][0]), bits(&v[k][1])])
}

fn fixture() -> serde_json::Value {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/ic_inspector/shape_vec.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("read the fixture")).expect("JSON")
}

/// `n_of` against the inspector's `n` on the fixture: on its decoded ICs (in its canonical frame) directly, on its raw
/// ICs as the canonical representative, `w` folded to `|w|` (the inspector canonicalises them, which mirrors), each
/// within `shape_vec_js_tol`. Returns the worst.
fn check_inspector(n_of: NFn) -> f64 {
    let f = fixture();
    let mut worst: f64 = 0.0;
    for (set, fold) in [("decoded", false), ("raw", true)] {
        let list = f[set].as_array().expect("a list");
        assert_eq!(list.len(), 500, "the fixture's {set} list");
        for (k, e) in list.iter().enumerate() {
            let (r, m, want) = (positions(&e["r"]), three(&e["m"]), three(&e["n"]));
            let mut got = n_of(r, m);
            if fold {
                got[2] = got[2].abs();
            }
            let d = component(got, want);
            assert!(
                d <= TWO_ROUTES,
                "the inspector's n on {set} IC {k} is {want:?}, ours {got:?}: {d:e} apart"
            );
            worst = worst.max(d);
        }
    }
    worst
}

#[test]
fn shape_vec_qa_inspector() {
    let w = check_inspector(kernel_shape).max(check_inspector(kernel_shape_vec));
    eprintln!(
        "shape_vec_qa_inspector: worst {w:.3e} ({:.1} ε) against shape_vec_js_tol {TWO_ROUTES:.3e}",
        w / EPS
    );
}

negative_control!(
    shape_vec_qa_inspector,
    "the superseded negative-cross map puts the inspector's decoded ICs on the other hemisphere, so it must fail",
    expected = "the inspector's n on decoded IC",
    {
        check_inspector(negative_cross_n);
    }
);

/// The fixture is the inspector's, run under the `.nvmrc` Node version (R-169, R-184), with the seed recorded.
fn check_provenance(nvmrc: &str) {
    let f = fixture();
    assert_eq!(
        f["source"], "docs/gui/reference/ic_inspector.html",
        "the fixture's source is not the IC Inspector"
    );
    assert_eq!(
        f["generator"], "fixtures/ic_inspector/gen_shape_vec.mjs",
        "the fixture's generator is not the pinned script"
    );
    let node = f["node"].as_str().expect("the Node version");
    assert_eq!(
        node.trim_start_matches('v'),
        nvmrc.trim().trim_start_matches('v'),
        "the fixture's Node version is not .nvmrc's"
    );
    assert!(f["seed"].is_u64(), "the fixture records no seed");
}

#[test]
fn shape_vec_qa_fixture_provenance() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for file in [
        "fixtures/ic_inspector/gen_shape_vec.mjs",
        "docs/gui/reference/ic_inspector.html",
    ] {
        assert!(root.join(file).is_file(), "{file} is missing");
    }
    check_provenance(&std::fs::read_to_string(root.join(".nvmrc")).expect("read .nvmrc"));
}

negative_control!(
    shape_vec_qa_fixture_provenance,
    "a fixture checked against another Node version must fail",
    expected = "is not .nvmrc's",
    {
        check_provenance("20.0.0");
    }
);

// ── REQ-CHART-019: the chart maps ───────────────────────────────────────────────────────────────────────────────────

type SphericalFn = fn(f64, f64) -> [f64; 3];

/// The spherical map `map`: `(s, 1)` is `L⁺` with `w = 1` exactly, `(s, 0)` is `L⁻`; on a grid that includes `t`
/// within `10⁻¹²` of each edge, the point is unit, its polar angle from `+w` is `π(1 − t)` and its azimuth `2πs`.
/// Returns the worst angle miss.
fn check_spherical(map: SphericalFn) -> f64 {
    let mut worst: f64 = 0.0;
    let ts = [
        0.0,
        1e-12,
        1e-6,
        0.01,
        0.25,
        0.5,
        0.75,
        0.99,
        1.0 - 1e-6,
        1.0 - 1e-12,
        1.0,
    ];
    let ss = [0.0, 0.1, 0.25, 0.4, 0.5, 0.6, 0.75, 0.9, 0.999];
    for &s in &ss {
        let top = map(s, 1.0);
        assert!(
            top[2] == 1.0 && top[0] == 0.0 && top[1] == 0.0,
            "(s, t) = ({s}, 1) is {top:?}, not L⁺ (0, 0, 1)"
        );
        let bottom = map(s, 0.0);
        assert!(
            dist(bottom, [0.0, 0.0, -1.0]) <= MAP_TOL,
            "(s, t) = ({s}, 0) is {bottom:?}, not L⁻ (0, 0, −1)"
        );
        for &t in &ts {
            let n = map(s, t);
            let norm = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            assert!((norm - 1.0).abs() <= MAP_TOL, "‖n({s}, {t})‖ = {norm}");
            let polar = n[0].hypot(n[1]).atan2(n[2]);
            let want = PI * (1.0 - t);
            let miss = (polar - want).abs();
            assert!(
                miss <= MAP_TOL * PI,
                "the polar angle at t = {t} is {polar}, not π(1 − t) = {want}"
            );
            worst = worst.max(miss);
            if t > 1e-3 && t < 1.0 - 1e-3 {
                let gap = angle_gap(n[1].atan2(n[0]), 2.0 * PI * s);
                assert!(
                    gap <= MAP_TOL * 2.0 * PI,
                    "the azimuth at s = {s} misses 2πs by {gap:e}"
                );
                worst = worst.max(gap);
            }
        }
    }
    worst
}

fn spherical_chart() -> ShapeSphere<f64> {
    ShapeSphere {
        map: ShapeMap::Spherical,
        n0: [0.0, 0.0, 1.0],
        e1: [1.0, 0.0, 0.0],
        e2: [0.0, 1.0, 0.0],
        scale: 1.0,
        inertia: 1.0,
        phase: 0.0,
        z_mass: [0.0, 0.0],
        z_mom: [0.0; 4],
    }
}

#[test]
fn shape_sphere_chart_map_qa_spherical() {
    let a = check_spherical(|s, t| spherical(s, t, PI));
    let b = check_spherical(|s, t| spherical_chart().point(s, t));
    // The f32 kernel: L⁺ at the top exactly, L⁻ at the bottom to its rounding.
    for s in [0.0f32, 0.3, 0.7] {
        let top = spherical(s, 1.0f32, std::f32::consts::PI);
        assert!(
            top[2] == 1.0 && top[0] == 0.0 && top[1] == 0.0,
            "f32 (s, 1) is {top:?}"
        );
        let bottom = spherical(s, 0.0f32, std::f32::consts::PI);
        assert!(
            (bottom[2] + 1.0).abs() <= 8.0 * f32::EPSILON,
            "f32 (s, 0) is {bottom:?}"
        );
    }
    eprintln!(
        "shape_sphere_chart_map_qa_spherical: worst {:.3e}",
        a.max(b)
    );
}

negative_control!(
    shape_sphere_chart_map_qa_spherical,
    "a map with a polar buffer, φ = ε + (π − 2ε)(1 − t), excises a cap and misses the poles, so it must fail",
    expected = "not L⁺",
    {
        check_spherical(|s, t| {
            let e = 0.01;
            let phi = e + (PI - 2.0 * e) * (1.0 - t);
            let theta = 2.0 * PI * s;
            [phi.sin() * theta.cos(), phi.sin() * theta.sin(), phi.cos()]
        });
    }
);

mod spherical_cap {
    use super::*;
    negative_control!(
        shape_sphere_chart_map_qa_spherical,
        "a map whose φ stops at a cap next to each pole, though it reaches the poles at the edges, must fail",
        expected = "the polar angle at t",
        {
            check_spherical(|s, t| {
                let phi = PI * (1.0 - t);
                let phi = if t > 0.0 && t < 1.0 { phi.clamp(0.01, PI - 0.01) } else { phi };
                let theta = 2.0 * PI * s;
                [phi.sin() * theta.cos(), phi.sin() * theta.sin(), phi.cos()]
            });
        }
    );
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn unit(a: [f64; 3]) -> [f64; 3] {
    let k = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    a.map(|x| x / k)
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

type ExpFn = fn([f64; 3], [f64; 3], [f64; 3], f64, f64, f64) -> [f64; 3];

/// The exponential map `map` about random orthonormal frames: at `(½, ½)`, `d = 0`, it is `n₀` exactly; elsewhere the
/// point is unit, at geodesic distance `r = ‖d‖` from `n₀` (extent below `π/(2√2)`, so `r < π/2`), and its tangent
/// part is along `d = (2u − 1)·s·e₁ + (2v − 1)·s·e₂`. Returns the worst miss.
fn check_exponential(map: ExpFn) -> f64 {
    let mut rng = SplitMix::new(0x2019);
    let mut worst: f64 = 0.0;
    for _ in 0..1024 {
        let n0 = unit([0, 1, 2].map(|_| on(&mut rng, -1.0, 1.0)));
        let e1 = unit(cross(n0, [0, 1, 2].map(|_| on(&mut rng, -1.0, 1.0))));
        let e2 = cross(n0, e1);
        let s = on(&mut rng, 0.01, 1.1);
        let centre = map(n0, e1, e2, s, 0.5, 0.5);
        assert!(
            centre == n0,
            "the exponential map at d = 0 is {centre:?}, not n₀ {n0:?}"
        );
        let (u, v) = (on(&mut rng, 0.0, 1.0), on(&mut rng, 0.0, 1.0));
        let (x, y) = ((2.0 * u - 1.0) * s, (2.0 * v - 1.0) * s);
        let r = x.hypot(y);
        let n = map(n0, e1, e2, s, u, v);
        let along = dot(n, n0);
        let tangent = [dot(n, e1), dot(n, e2)];
        let geodesic = tangent[0].hypot(tangent[1]).atan2(along);
        let miss = (geodesic - r).abs();
        assert!(
            miss <= 16.0 * EPS,
            "the exponential map at ({u}, {v}) is {geodesic} from n₀, not r = ‖d‖ = {r}"
        );
        if r > 0.1 {
            let dir = (tangent[0] * y - tangent[1] * x).abs() / (tangent[0].hypot(tangent[1]) * r);
            assert!(
                dir <= 64.0 * EPS,
                "the exponential map's tangent part is not along d ({dir:e})"
            );
            worst = worst.max(dir);
        }
        worst = worst.max(miss);
    }
    worst
}

#[test]
fn shape_sphere_chart_map_qa_exponential() {
    let a = check_exponential(exponential);
    let b = check_exponential(|n0, e1, e2, s, u, v| {
        let chart = ShapeSphere {
            map: ShapeMap::Exponential,
            n0,
            e1,
            e2,
            scale: s,
            ..spherical_chart()
        };
        chart.point(u, v)
    });
    eprintln!(
        "shape_sphere_chart_map_qa_exponential: worst {:.3e}",
        a.max(b)
    );
}

negative_control!(
    shape_sphere_chart_map_qa_exponential,
    "the gnomonic map n = (n₀ + d)/‖n₀ + d‖ is at distance atan r, not r, so it must fail",
    expected = "the exponential map",
    {
        check_exponential(|n0, e1, e2, s, u, v| {
            let (x, y) = ((2.0 * u - 1.0) * s, (2.0 * v - 1.0) * s);
            unit([0, 1, 2].map(|k| n0[k] + x * e1[k] + y * e2[k]))
        });
    }
);

// ── REQ-CHART-020, REQ-CHART-053: the closed-form inverse and the round trip ─────────────────────────────────────────

/// A random spherical or exponential chart with random held blocks and gauge.
fn random_chart(rng: &mut SplitMix, spherical_map: bool) -> ShapeSphere<f64> {
    let n0 = unit([0, 1, 2].map(|_| on(rng, -1.0, 1.0)));
    let e1 = unit(cross(n0, [0, 1, 2].map(|_| on(rng, -1.0, 1.0))));
    ShapeSphere {
        map: if spherical_map {
            ShapeMap::Spherical
        } else {
            ShapeMap::Exponential
        },
        n0,
        e1,
        e2: cross(n0, e1),
        scale: on(rng, 0.1, PI),
        inertia: on(rng, 0.25, 4.0),
        phase: on(rng, -PI, PI),
        z_mass: [on(rng, -4.0, 4.0), on(rng, -4.0, 4.0)],
        z_mom: [0, 1, 2, 3].map(|_| on(rng, -2.0, 2.0)),
    }
}

type DecodeFn = fn(&ShapeSphere<f64>, f64, f64) -> ([f64; 3], Pos);

/// `Φ` alone (R-178) through `decode`: `shape(r, m)` of the decoded configuration is the chart's `n(u, v)` within
/// `shape_roundtrip_bound`, on random charts of both maps, both hemispheres drawn. Returns the worst.
fn check_chart_roundtrip(decode: DecodeFn) -> f64 {
    let mut rng = SplitMix::new(0x2020);
    let (mut worst, mut lower, mut upper): (f64, u32, u32) = (0.0, 0, 0);
    for k in 0..8192 {
        let chart = random_chart(&mut rng, k % 2 == 0);
        let (u, v) = (on(&mut rng, 0.0, 1.0), on(&mut rng, 0.0, 1.0));
        let n = chart.point(u, v);
        let (m, r) = decode(&chart, u, v);
        let got = shape(r, m);
        let miss = dist(got, n);
        assert!(
            miss <= ROUNDTRIP,
            "shape(Φ({u}, {v})) is {got:?}, not n {n:?}: {miss:e} beyond shape_roundtrip_bound"
        );
        if n[2] < 0.0 {
            lower += 1;
        } else {
            upper += 1;
        }
        worst = worst.max(miss);
    }
    assert!(
        lower > 1000 && upper > 1000,
        "the draws miss a hemisphere: {lower} lower, {upper} upper"
    );
    worst
}

/// `Φ` alone through the chart's decode: its blocks' masses and configuration, before `C` (R-178).
fn chart_decode(chart: &ShapeSphere<f64>, u: f64, v: f64) -> ([f64; 3], Pos) {
    let d = chart.decode::<Defaults, f64>(u, v);
    (d.masses.m, d.config.r)
}

/// `Φ`'s state through the chart's decode: its blocks joined, before `C` (R-178).
fn chart_phi(chart: &ShapeSphere<f64>, u: f64, v: f64) -> Ic<f64> {
    let d = chart.decode::<Defaults, f64>(u, v);
    Ic {
        m: d.masses.m,
        r: d.config.r,
        p: d.momentum.p,
    }
}

#[test]
fn shape_sphere_roundtrip_qa_chart() {
    let w = check_chart_roundtrip(chart_decode);
    eprintln!(
        "shape_sphere_roundtrip_qa_chart: worst {w:.3e} against shape_roundtrip_bound {ROUNDTRIP:.0e}"
    );
}

negative_control!(
    shape_sphere_roundtrip_qa_chart,
    "Φ built with ψ = φ_f + atan2(−q, p), the superseded sign, misses every point off the equator, so it must fail",
    expected = "beyond shape_roundtrip_bound",
    {
        check_chart_roundtrip(|chart, u, v| {
            let n = chart.point(u, v);
            let m = kernel::decode::mass::decode::<Defaults, f64>(chart.z_mass).m;
            let inv = inverse(n, chart.inertia, chart.phase);
            let b = inv.lambda_t[0].hypot(inv.lambda_t[1]);
            let psi = chart.phase - inv.beta;
            (m, reconstruct(m, inv.rho_t, [b * psi.cos(), b * psi.sin()]))
        });
    }
);

type InverseFn = fn([f64; 3], f64, f64) -> ([f64; 2], [f64; 2]);

/// The inverse `inv` is §3.2's closed form on random points of both hemispheres, near the poles included: `‖ρ̃‖² =
/// I(1 + n₀)/2` and `‖λ̃‖² = I(1 − n₀)/2` (to `shape_roundtrip_bound` of `I`), `ρ̃` at the angle `φ_f`, `λ̃` at
/// `φ_f + atan2(q, p)` where `λ̃` is not tiny; and through §0.2's reconstruction written here the round trip recovers
/// `n`. Returns the worst.
fn check_closed_form(inv: InverseFn) -> f64 {
    let mut rng = SplitMix::new(0x2021);
    let mut worst: f64 = 0.0;
    for k in 0..8192 {
        let n = if k % 4 == 3 {
            let r = 10f64.powf(-on(&mut rng, 1.0, 15.0));
            let chi = on(&mut rng, -PI, PI);
            let sign = if k % 8 == 3 { 1.0 } else { -1.0 };
            [sign * r.cos(), r.sin() * chi.cos(), r.sin() * chi.sin()]
        } else {
            unit([0, 1, 2].map(|_| on(&mut rng, -1.0, 1.0)))
        };
        let (i, phase) = (on(&mut rng, 0.25, 4.0), on(&mut rng, -PI, PI));
        let m = masses(&mut rng, 4.0);
        let (rt, lt) = inv(n, i, phase);
        let a = rt[0] * rt[0] + rt[1] * rt[1];
        let b = lt[0] * lt[0] + lt[1] * lt[1];
        let da = (a - i * (1.0 + n[0]) / 2.0)
            .abs()
            .max((b - i * (1.0 - n[0]) / 2.0).abs())
            / i;
        assert!(
            da <= ROUNDTRIP,
            "at n {n:?}, I {i}: ‖ρ̃‖² = {a}, ‖λ̃‖² = {b}, not I(1 ± n₀)/2 ({da:e})"
        );
        if a > 1e-6 * i {
            let gap = angle_gap(rt[1].atan2(rt[0]), phase);
            assert!(
                gap <= 64.0 * EPS,
                "ρ̃ is not at the angle φ_f = {phase} ({gap:e})"
            );
        }
        if b > 1e-6 * i {
            let want = phase + (n[2]).atan2(n[1]);
            let gap = angle_gap(lt[1].atan2(lt[0]), want);
            assert!(
                gap <= 64.0 * EPS,
                "λ̃ is not at the angle φ_f + atan2(q, p) at n {n:?} ({gap:e})"
            );
        }
        let got = shape(reconstruct(m, rt, lt), m);
        let miss = dist(got, n);
        assert!(
            miss <= ROUNDTRIP,
            "the round trip at n {n:?} gives {got:?} ({miss:e})"
        );
        worst = worst.max(miss).max(da);
    }
    worst
}

fn kernel_inverse(n: [f64; 3], i: f64, phase: f64) -> ([f64; 2], [f64; 2]) {
    let x = inverse(n, i, phase);
    (x.rho_t, x.lambda_t)
}

#[test]
fn shape_sphere_roundtrip_qa_closed_form() {
    let w = check_closed_form(kernel_inverse);
    eprintln!("shape_sphere_roundtrip_qa_closed_form: worst {w:.3e}");
}

negative_control!(
    shape_sphere_roundtrip_qa_closed_form,
    "the inverse as written, b = I(1 − n₀)/2 from n₀ alone, loses ‖λ̃‖ near n₀ = 1 and must fail",
    expected = "the round trip at n",
    {
        check_closed_form(|n, i, phase| {
            let x = inverse(n, i, phase);
            let (a, b) = (i * (1.0 + n[0]) / 2.0, i * (1.0 - n[0]) / 2.0);
            let psi = phase + x.beta;
            (
                [a.sqrt() * phase.cos(), a.sqrt() * phase.sin()],
                [b.sqrt() * psi.cos(), b.sqrt() * psi.sin()],
            )
        });
    }
);

mod closed_form_sign {
    use super::*;
    negative_control!(
        shape_sphere_roundtrip_qa_closed_form,
        "ψ = φ_f + atan2(−q, p) puts λ̃ on the mirror side, so it must fail",
        expected = "atan2(q, p)",
        {
            check_closed_form(|n, i, phase| {
                let x = inverse(n, i, phase);
                let b = x.lambda_t[0].hypot(x.lambda_t[1]);
                let psi = phase - x.beta;
                (x.rho_t, [b * psi.cos(), b * psi.sin()])
            });
        }
    );
}

/// At the eight poles `(±1, ±0, ±0)` (R-437): `atan2(q, p)`'s term, the inverse's `β`, is `+0` by its bits, so
/// `ψ = φ_f`: at `n₀ = −1` (`a = 0`) `λ̃ = √I·(cos φ_f, sin φ_f)` and `ρ̃ = 0`; at `n₀ = 1` (`b = 0`) `λ̃ = 0`; and the
/// chart's decode there is finite and recovers the pole.
fn check_poles(beta_of: fn([f64; 3], f64, f64) -> f64) {
    let mut rng = SplitMix::new(0x2437);
    for n0 in [1.0, -1.0] {
        for n1 in [0.0, -0.0] {
            for n2 in [0.0, -0.0] {
                for _ in 0..16 {
                    let n = [n0, n1, n2];
                    let (i, phase) = (on(&mut rng, 0.25, 4.0), on(&mut rng, -PI, PI));
                    let beta = beta_of(n, i, phase);
                    assert!(
                        beta.to_bits() == 0,
                        "at the pole {n:?} the atan2 term is {beta:e}, not R-437's +0"
                    );
                    let x = inverse(n, i, phase);
                    let want = if n0 < 0.0 {
                        [i.sqrt() * phase.cos(), i.sqrt() * phase.sin()]
                    } else {
                        [0.0, 0.0]
                    };
                    let d = (x.lambda_t[0] - want[0])
                        .abs()
                        .max((x.lambda_t[1] - want[1]).abs());
                    assert!(
                        d <= 4.0 * EPS * i.sqrt(),
                        "at the pole {n:?}, λ̃ is {:?}, not {want:?}",
                        x.lambda_t
                    );
                    let rho = x.rho_t[0].hypot(x.rho_t[1]);
                    let want_rho = if n0 < 0.0 { 0.0 } else { i.sqrt() };
                    assert!(
                        (rho - want_rho).abs() <= 4.0 * EPS * i.sqrt(),
                        "at the pole {n:?}, ‖ρ̃‖ = {rho}"
                    );
                    let m = masses(&mut rng, 4.0);
                    let got = shape(reconstruct(m, x.rho_t, x.lambda_t), m);
                    assert!(
                        dist(got, n) <= ROUNDTRIP,
                        "the round trip at the pole {n:?} gives {got:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn shape_sphere_roundtrip_qa_poles() {
    check_poles(|n, i, phase| inverse(n, i, phase).beta);
    // The chart at its poles, (s, 1) and (s, 0), is finite and lands on them.
    let mut rng = SplitMix::new(0x2438);
    for _ in 0..64 {
        let chart = ShapeSphere {
            map: ShapeMap::Spherical,
            ..random_chart(&mut rng, true)
        };
        let s = on(&mut rng, 0.0, 1.0);
        for (t, pole) in [(1.0, [0.0, 0.0, 1.0]), (0.0, [0.0, 0.0, -1.0])] {
            let (m, r) = chart_decode(&chart, s, t);
            assert!(
                r.iter().flatten().all(|x| x.is_finite()),
                "Φ({s}, {t}) is not finite: {r:?}"
            );
            assert!(
                dist(shape(r, m), pole) <= ROUNDTRIP,
                "Φ({s}, {t}) is not at {pole:?}"
            );
            // The decode keeps the signed shape (R-544): its canonical ic is at the pole it decodes, L⁻ at t = 0.
            let d = chart.decode::<Defaults, f64>(s, t);
            assert!(
                d.ic.r.iter().flatten().all(|x| x.is_finite())
                    && dist(shape(d.ic.r, d.ic.m), pole) <= ROUNDTRIP,
                "the decode at ({s}, {t}) is not finite at {pole:?}: {:?}",
                d.ic.r
            );
        }
    }
}

negative_control!(
    shape_sphere_roundtrip_qa_poles,
    "atan2(q, p) taken as it comes gives ±π at q = ±0, p = −0, not R-437's 0, so it must fail",
    expected = "not R-437's +0",
    {
        check_poles(|n, i, _| (i * n[2] / 2.0).atan2(i * n[1] / 2.0));
    }
);

/// The gate's fixture judges against the register's bound, provisional, citing REQ-CHART-053 and R-181; its inputs are
/// the round trip, expected to pass, and the negative-cross control, expected to fail.
fn check_gate_json(gate: &serde_json::Value, bound: f64) {
    let t = &gate["threshold"];
    assert_eq!(
        t["value"].as_f64(),
        Some(bound),
        "gate.json's threshold is not shape_roundtrip_bound"
    );
    assert_eq!(
        t["requirement"], "REQ-CHART-053",
        "gate.json's threshold does not cite REQ-CHART-053"
    );
    assert_eq!(
        t["status"], "provisional",
        "gate.json's threshold is not provisional"
    );
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/gates/shape-roundtrip");
    let mut expected = Vec::new();
    for name in gate["inputs"].as_array().expect("inputs") {
        let text = std::fs::read_to_string(dir.join(name.as_str().expect("a name")))
            .expect("read an input");
        let v: serde_json::Value = serde_json::from_str(&text).expect("JSON");
        expected.push((
            v["inverse"].as_str().unwrap_or("").to_owned(),
            v["expected"].as_str().unwrap_or("").to_owned(),
        ));
    }
    expected.sort();
    assert_eq!(
        expected,
        vec![
            ("closed_form".to_owned(), "pass".to_owned()),
            ("negative_cross".to_owned(), "fail".to_owned())
        ],
        "the gate's inputs are not the round trip and its negative-cross control"
    );
}

#[test]
fn shape_sphere_roundtrip_qa_gate_json() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/gates/shape-roundtrip/gate.json");
    let gate: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("read gate.json"))
            .expect("JSON");
    assert_eq!(
        ROUNDTRIP, 1e-13,
        "shape_roundtrip_bound is not R-181's 1e-13"
    );
    check_gate_json(&gate, ROUNDTRIP);
}

negative_control!(
    shape_sphere_roundtrip_qa_gate_json,
    "a gate judged at 1e-12, not the register's bound, must fail",
    expected = "is not shape_roundtrip_bound",
    {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/gates/shape-roundtrip/gate.json");
        let mut gate: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).expect("read gate.json"))
                .expect("JSON");
        gate["threshold"]["value"] = serde_json::json!(1e-12);
        check_gate_json(&gate, ROUNDTRIP);
    }
);

// ── REQ-CHART-020, R-141: Φ then C ─────────────────────────────────────────────────────────────────────────────────

type CanonFn = fn(&Ic<f64>) -> Ic<f64>;

/// `C(Φ(s, t))` on random spherical charts: on the lower hemisphere (`t < ½`) its shape is the map's point at
/// `(s, 1 − t)`, which is `(θ, π − φ)`; on the upper it is `(s, t)`'s own; within `shape_roundtrip_bound`. At the
/// default gauge `I = 1`, `φ_f = 0`, an upper-hemisphere `Φ` is on `C`'s section, so `C` leaves its positions. Returns
/// the worst.
fn check_fold(canon: CanonFn) -> f64 {
    let mut rng = SplitMix::new(0x2141);
    let (mut worst, mut lower): (f64, u32) = (0.0, 0);
    for k in 0..2048 {
        let mut chart = random_chart(&mut rng, true);
        if k % 2 == 0 {
            chart.inertia = 1.0;
            chart.phase = 0.0;
        }
        let (s, t) = (on(&mut rng, 0.0, 1.0), on(&mut rng, 0.01, 0.99));
        let phi = chart_phi(&chart, s, t);
        let c = canon(&phi);
        let got = shape(c.r, c.m);
        let want = if t < 0.5 {
            lower += 1;
            spherical(s, 1.0 - t, PI)
        } else {
            spherical(s, t, PI)
        };
        let miss = dist(got, want);
        assert!(
            miss <= ROUNDTRIP,
            "C(Φ({s}, {t})) is at {got:?}, not the fold's {want:?} ({miss:e})"
        );
        worst = worst.max(miss);
        if k % 2 == 0 && t > 0.5 {
            let size = phi.r.iter().flatten().fold(1.0f64, |k, x| k.max(x.abs()));
            let moved = (0..3)
                .flat_map(|b| (0..2).map(move |x| (b, x)))
                .map(|(b, x)| (c.r[b][x] - phi.r[b][x]).abs())
                .fold(0.0, f64::max)
                / size;
            assert!(
                moved <= ROUNDTRIP,
                "at the default gauge C moves the upper-hemisphere Φ({s}, {t}) by {moved:e}"
            );
        }
    }
    assert!(lower > 500, "only {lower} lower-hemisphere draws");
    worst
}

#[test]
fn shape_sphere_canonical_fold_qa() {
    let w = check_fold(|ic| canonicalise(ic).ic);
    eprintln!("shape_sphere_canonical_fold_qa: worst {w:.3e}");
}

negative_control!(
    shape_sphere_canonical_fold_qa,
    "without C the lower hemisphere stays put, off the fold's point, so it must fail",
    expected = "not the fold's",
    {
        check_fold(|ic| *ic);
    }
);

// ── dd_decoder §2, §3.5, R-544: the decode hands the integrator C(Φ), step 3 not taken, canonical ──────────────────

/// The canonical section's identities (`I = 1`, the CoM, `ρ_y`, `Σp`) and §3.5's scalings of `E` and `L_z`: `C`'s
/// rotation, mirror and scale, a few roundings each, against the state's own size (applied per R-369, a test decision).
const CANON_TOL: f64 = 64.0 * EPS;

/// `|n₂|` above which `λ̃_y`'s sign is asserted to be `n₂`'s: with `ρ` on `+x`, `n₂ = 2ρ̃ₓλ̃_y` at `I = 1` (§3.1), so the
/// two share a sign; at `n₂` within a few roundings of 0 the sign is the rounding's (applied per R-369, a test decision).
const SIGN_FLOOR: f64 = CANON_TOL;

/// `(com, I, E, L_z, size, L_z's scale)` of a state: the mass-weighted moment of inertia about the CoM, §3.6's
/// `K₀ + V₀` with `G = 1`, `Σ rᵢ × pᵢ`, the scale `Σ mᵢ‖rᵢ‖ + Σ‖pᵢ‖` the relative checks are taken against, and
/// `Σ ‖rᵢ‖‖pᵢ‖`, `L_z`'s own (a sum of products that can cancel).
fn state(ic: &Ic<f64>) -> ([f64; 2], f64, f64, f64, f64, f64) {
    let (m, r, p) = (ic.m, ic.r, ic.p);
    let total: f64 = m.iter().sum();
    let com = [0, 1].map(|j| (0..3).map(|i| m[i] * r[i][j]).sum::<f64>() / total);
    let inertia = (0..3)
        .map(|i| m[i] * ((r[i][0] - com[0]).powi(2) + (r[i][1] - com[1]).powi(2)))
        .sum::<f64>();
    let kinetic = (0..3)
        .map(|i| (p[i][0] * p[i][0] + p[i][1] * p[i][1]) / (2.0 * m[i]))
        .sum::<f64>();
    let potential = -[(0, 1), (1, 2), (0, 2)]
        .iter()
        .map(|&(i, j)| m[i] * m[j] / (r[i][0] - r[j][0]).hypot(r[i][1] - r[j][1]))
        .sum::<f64>();
    let lz = (0..3)
        .map(|i| r[i][0] * p[i][1] - r[i][1] * p[i][0])
        .sum::<f64>();
    let size = (0..3)
        .map(|i| m[i] * r[i][0].hypot(r[i][1]) + p[i][0].hypot(p[i][1]))
        .sum::<f64>();
    let lz_scale = (0..3)
        .map(|i| r[i][0].hypot(r[i][1]) * p[i][0].hypot(p[i][1]))
        .sum::<f64>();
    (com, inertia, kinetic + potential, lz, size, lz_scale)
}

type DecodedFn = fn(&ShapeSphere<f64>, f64, f64) -> Decoded<f64>;

/// The decode's `ic` on random charts of both maps, `I ∈ [¼, 4]`, both hemispheres drawn, is canonical in
/// `G = M = I = 1` (dd_decoder §2, §3.3, §3.5): the CoM at the origin and at rest, `I = 1`, `ρ = r₁ − r₀` on `+x`,
/// `λ̃_y` of `n₂`'s sign (`C`'s mirror not taken, R-544), the masses the blocks' `m_fixed`, its shape the signed
/// `n(u, v)` within `shape_roundtrip_bound`, and §3.5's similarity of `Φ`'s state, `ℓ = √I`: `E = ℓ·E(Φ)`,
/// `|L_z| = ℓ^{−½}·|L_z(Φ)|`. Its descriptor is read from that `ic` (§3.6): `rho_mag² + lambda_mag² = 1`,
/// `K₀ + V₀ = E`. Returns the worst relative miss of the identities.
fn check_decode_canonical(decode: DecodedFn) -> f64 {
    let mut rng = SplitMix::new(0x0236);
    let (mut worst, mut lower, mut far): (f64, u32, u32) = (0.0, 0, 0);
    for k in 0..4096 {
        let chart = random_chart(&mut rng, k % 2 == 0);
        let (u, v) = (on(&mut rng, 0.0, 1.0), on(&mut rng, 0.0, 1.0));
        let phi = chart_phi(&chart, u, v);
        let d = decode(&chart, u, v);
        let ic = d.ic;
        assert!(
            ic.r.iter()
                .chain(ic.p.iter())
                .flatten()
                .all(|x| x.is_finite()),
            "the decode at ({u}, {v}) is not finite"
        );
        let (com, inertia, e, lz, size, lz_scale) = state(&ic);
        let (_, inertia_phi, e_phi, lz_phi, _, lz_scale_phi) = state(&phi);
        let total = [0, 1].map(|j| ic.p.iter().map(|q| q[j]).sum::<f64>());
        let rho = [ic.r[1][0] - ic.r[0][0], ic.r[1][1] - ic.r[0][1]];
        let m01 = ic.m[0] + ic.m[1];
        let lambda_y = (ic.m[2] * m01).sqrt()
            * (ic.r[2][1] - (ic.m[0] * ic.r[0][1] + ic.m[1] * ic.r[1][1]) / m01);
        let ell = inertia_phi.sqrt();
        let misses = [
            (inertia - 1.0).abs(),
            com[0].abs().max(com[1].abs()) / size,
            total[0].abs().max(total[1].abs()) / size,
            rho[1].abs() / size,
            (e - ell * e_phi).abs() / (e.abs() + 1.0),
            (lz.abs() - lz_phi.abs() / ell.sqrt()).abs() / (lz_scale + lz_scale_phi / ell.sqrt()),
        ];
        let miss = misses.iter().fold(0.0f64, |a, &b| a.max(b));
        assert!(
            miss <= CANON_TOL,
            "the decode at ({u}, {v}), chart I {}, is not canonical: I − 1, CoM, Σp, ρ_y, E against ℓE(Φ), |L_z| \
             against ℓ^-½|L_z(Φ)|: {misses:?}",
            chart.inertia
        );
        worst = worst.max(miss);
        assert!(rho[0] > 0.0, "ρ at ({u}, {v}) is not on +x: {rho:?}");
        assert!(ic.m == d.masses.m, "the canonical masses are not m_fixed");
        let n = chart.point(u, v);
        assert!(
            n[2].abs() <= SIGN_FLOOR || (lambda_y < 0.0) == (n[2] < 0.0),
            "λ̃_y = {lambda_y:e} at ({u}, {v}), n₂ = {:e}: C's mirror step is taken",
            n[2]
        );
        let got = shape(ic.r, ic.m);
        assert!(
            dist(got, n) <= ROUNDTRIP,
            "the decode's shape at ({u}, {v}) is {got:?}, not the signed n {n:?}"
        );
        let q = &d.descriptor;
        assert!(
            (q.rho_mag.powi(2) + q.lambda_mag.powi(2) - 1.0).abs() <= CANON_TOL
                && (q.K_0 + q.V_0 - e).abs() <= CANON_TOL * (e.abs() + 1.0),
            "the descriptor at ({u}, {v}) is not the canonical ic's: rho_mag {}, lambda_mag {}, K₀ + V₀ {} against {e}",
            q.rho_mag,
            q.lambda_mag,
            q.K_0 + q.V_0
        );
        lower += u32::from(n[2] < 0.0);
        far += u32::from((chart.inertia - 1.0).abs() > 0.5);
    }
    assert!(
        lower > 1000 && far > 1000,
        "the draws miss a regime: {lower} lower-hemisphere, {far} at I far from 1"
    );
    worst
}

#[test]
fn shape_sphere_decode_canonical_qa() {
    let w = check_decode_canonical(|c, u, v| c.decode::<Defaults, f64>(u, v));
    let w2 = check_decode_canonical(chart::decode::<Defaults, f64, _>);
    eprintln!(
        "shape_sphere_decode_canonical_qa: worst {:.3e} ({:.1} ε)",
        w.max(w2),
        w.max(w2) / EPS
    );
}

negative_control!(
    shape_sphere_decode_canonical_qa,
    "Φ's state handed to the integrator without C is off I = 1 at a chart I ≠ 1, so it must fail",
    expected = "is not canonical",
    {
        check_decode_canonical(|c, u, v| Decoded {
            ic: chart_phi(c, u, v),
            ..c.decode::<Defaults, f64>(u, v)
        });
    }
);

mod decode_unfolded {
    use super::*;
    negative_control!(
        shape_sphere_decode_canonical_qa,
        "Φ scaled to I = 1 but not turned leaves ρ off +x, so it must fail",
        expected = "is not canonical",
        {
            check_decode_canonical(|c, u, v| {
                let phi = chart_phi(c, u, v);
                let ell = c.inertia.sqrt();
                Decoded {
                    ic: Ic {
                        r: phi.r.map(|x| x.map(|y| y / ell)),
                        p: phi.p.map(|x| x.map(|y| y * ell.sqrt())),
                        ..phi
                    },
                    ..c.decode::<Defaults, f64>(u, v)
                }
            });
        }
    );
}

mod decode_mirrored {
    use super::*;
    negative_control!(
        shape_sphere_decode_canonical_qa,
        "R-141's gauge left in, C's mirror step run on the decode, puts the lower hemisphere at λ̃_y > 0, so it must fail",
        expected = "C's mirror step is taken",
        {
            check_decode_canonical(|c, u, v| {
                let d = c.decode::<Defaults, f64>(u, v);
                Decoded {
                    ic: canonicalise(&d.ic).ic,
                    ..d
                }
            });
        }
    );
}

type DecodeChartFn = fn(&ShapeSphere<f64>, f64, f64, Option<f64>) -> (Decoded<f64>, Outcome);

/// `η_E`'s entry on the shape sphere (dd_decoder §3.7): with no target (`None`, "Off") the decode is the chart's own,
/// its shape the signed `n(u, v)` (R-544); with `E* = V₀ + ½` (feasible, `K₀ > 0`) the positions are the same, the outcome
/// `Scaled`, the energy of the result `E*` within `CANON_TOL` relative, and the descriptor's `K₀ + V₀` too.
fn check_energy_norm(decode: DecodeChartFn) {
    let mut rng = SplitMix::new(0x0237);
    for k in 0..1024 {
        let chart = random_chart(&mut rng, k % 2 == 0);
        let (u, v) = (on(&mut rng, 0.0, 1.0), on(&mut rng, 0.0, 1.0));
        let n = chart.point(u, v);
        let (off, outcome) = decode(&chart, u, v, None);
        let got = shape(off.ic.r, off.ic.m);
        assert!(
            dist(got, n) <= ROUNDTRIP,
            "η_E's entry at ({u}, {v}) decodes the shape {got:?}, not the signed n {n:?}"
        );
        assert_eq!(outcome, Outcome::Off, "the outcome with no target");
        assert!(
            off == chart.decode::<Defaults, f64>(u, v),
            "η_E's entry with no target is not the chart's own decode at ({u}, {v})"
        );
        let e_star = off.descriptor.V_0 + 0.5;
        let (on_, outcome) = decode(&chart, u, v, Some(e_star));
        assert_eq!(outcome, Outcome::Scaled, "the outcome under E* = V₀ + ½");
        assert!(
            on_.ic.r == off.ic.r,
            "η_E moved the positions at ({u}, {v})"
        );
        let (_, _, e, _, _, _) = state(&on_.ic);
        let q = &on_.descriptor;
        assert!(
            (e - e_star).abs() <= CANON_TOL * (e_star.abs() + 1.0)
                && (q.K_0 + q.V_0 - e_star).abs() <= CANON_TOL * (e_star.abs() + 1.0),
            "under E* = {e_star} the energy is {e}, the descriptor's {}",
            q.K_0 + q.V_0
        );
    }
}

#[test]
fn shape_sphere_energy_norm_qa() {
    check_energy_norm(energy_norm::decode_chart::<Defaults, f64, _>);
}

negative_control!(
    shape_sphere_energy_norm_qa,
    "η_E's entry decoding map's latent alone, through D_cfg, loses the chart's shape, so it must fail",
    expected = "not the signed n",
    {
        check_energy_norm(|c, u, v, e| {
            energy_norm::decode_with(
                &mut kernel::decode::Free::<Defaults>::new(),
                &c.map(u, v).z,
                e,
                c.forbids_energy_normalisation(),
            )
        });
    }
);

// ── REQ-CHART-036, REQ-INT-003: the landmarks ───────────────────────────────────────────────────────────────────────

type LandmarksFn = fn([f64; 3]) -> Landmarks<f64>;

/// An angle between landmarks, in degrees: the two-route tolerance on a point of the sphere as an angle, with `acos`'s
/// conditioning at 120° (`1/sin 120° < 2`) and the dot product's two roundings.
const DEG_TOL: f64 = 4.0 * TWO_ROUTES * 180.0 / PI;

/// Equal masses (dd_integrator test 9, R-14): `BC₀₁ = (−1, 0, 0)` exactly, at 180°; `BC₁₂` at 60°, `(½, √3/2, 0)`;
/// `BC₂₀` at 300°, `(½, −√3/2, 0)`; the three 120° apart on `w = 0`; `L± = (0, 0, ±1)`.
fn check_equal(of: LandmarksFn) {
    let l = of([1.0 / 3.0; 3]);
    assert!(
        l.bc[2] == [-1.0, 0.0, 0.0],
        "BC₀₁ at equal masses is {:?}, not (−1, 0, 0)",
        l.bc[2]
    );
    let h = 3f64.sqrt() / 2.0;
    for (k, want, deg) in [
        (0, [0.5, h, 0.0], 60.0),
        (1, [0.5, -h, 0.0], 300.0),
        (2, [-1.0, 0.0, 0.0], 180.0),
    ] {
        assert!(
            dist(l.bc[k], want) <= TWO_ROUTES,
            "the equal-mass collision bc[{k}] is {:?}, not {want:?}",
            l.bc[k]
        );
        let az = azimuth_deg(l.bc[k]);
        assert!(
            (az - deg).abs() <= DEG_TOL,
            "bc[{k}]'s azimuth is {az}°, not {deg}°"
        );
    }
    for (a, b) in [(0, 1), (1, 2), (2, 0)] {
        let gap = dot(l.bc[a], l.bc[b]).clamp(-1.0, 1.0).acos().to_degrees();
        assert!(
            (gap - 120.0).abs() <= DEG_TOL,
            "bc[{a}] and bc[{b}] are {gap}° apart, not 120°"
        );
    }
    assert!(
        dist(l.l_plus, [0.0, 0.0, 1.0]) <= TWO_ROUTES,
        "L⁺ at equal masses is {:?}",
        l.l_plus
    );
    assert!(
        dist(l.l_minus, [0.0, 0.0, -1.0]) <= TWO_ROUTES,
        "L⁻ at equal masses is {:?}",
        l.l_minus
    );
}

#[test]
fn shape_landmarks_qa_equal_masses() {
    check_equal(landmarks);
}

negative_control!(
    shape_landmarks_qa_equal_masses,
    "landmarks from the negative-cross map put L⁺ at the south pole, so they must fail",
    expected = "L⁺ at equal masses",
    {
        check_equal(|m| {
            let mut l = landmarks(m);
            l.l_plus[2] = -l.l_plus[2];
            l
        });
    }
);

/// Random masses, ratios up to `e⁸`: `BC₀₁ = (−1, 0, 0)` exactly; every collision on `w = 0`, unit, at the
/// mass-weighted point [`analytic_bc`]; and `L⁺`'s `w > 0` (its orientation, RQ-355 aside).
fn check_mass_weighted(of: LandmarksFn) -> f64 {
    let mut rng = SplitMix::new(0x2050);
    let mut worst: f64 = 0.0;
    for _ in 0..4096 {
        let m = masses(&mut rng, 4.0);
        let l = of(m);
        assert!(
            l.bc[2] == [-1.0, 0.0, 0.0],
            "BC₀₁ at masses {m:?} is {:?}",
            l.bc[2]
        );
        for (k, (got, want)) in l.bc.iter().zip(analytic_bc(m)).enumerate() {
            assert!(
                got[2].abs() <= TWO_ROUTES,
                "bc[{k}] at masses {m:?} is off the equator: {:?}",
                got
            );
            let d = component(*got, want);
            assert!(
                d <= TWO_ROUTES,
                "bc[{k}] at masses {m:?} is {:?}, not its mass-weighted point {:?} ({d:e})",
                got,
                want
            );
            worst = worst.max(d);
        }
        assert!(
            l.l_plus[2] > 0.0,
            "L⁺ at masses {m:?} has w ≤ 0: {:?}",
            l.l_plus
        );
        assert!(
            l.l_minus[2] < 0.0,
            "L⁻ at masses {m:?} has w ≥ 0: {:?}",
            l.l_minus
        );
    }
    worst
}

#[test]
fn shape_landmarks_mass_weighted_qa() {
    let w = check_mass_weighted(landmarks);
    eprintln!(
        "shape_landmarks_mass_weighted_qa: worst {w:.3e} ({:.1} ε)",
        w / EPS
    );
}

negative_control!(
    shape_landmarks_mass_weighted_qa,
    "landmarks hard-coded at the equal-mass 120° points miss the mass-weighted ones, so they must fail",
    expected = "not its mass-weighted point",
    {
        check_mass_weighted(|_| landmarks([1.0 / 3.0; 3]));
    }
);

/// The collisions against the inspector's `collisionPhi` azimuths on the fixture's raw ICs' masses (order `BC₀₁`,
/// `BC₁₂`, `BC₀₂`), each turned into its equator point, within `shape_vec_js_tol`. Returns the worst.
fn check_landmarks_inspector(of: LandmarksFn) -> f64 {
    let f = fixture();
    let mut worst: f64 = 0.0;
    for (k, e) in f["raw"].as_array().expect("a list").iter().enumerate() {
        let (m, phi) = (three(&e["m"]), three(&e["bc_azimuth"]));
        let l = of(m);
        for (ours, theirs) in [(2, 0), (0, 1), (1, 2)] {
            let want = [phi[theirs].cos(), phi[theirs].sin(), 0.0];
            let d = component(l.bc[ours], want);
            assert!(
                d <= TWO_ROUTES,
                "IC {k}: our bc[{ours}] {:?} is {d:e} from the inspector's collision at {}",
                l.bc[ours],
                phi[theirs]
            );
            worst = worst.max(d);
        }
    }
    worst
}

#[test]
fn shape_landmarks_qa_inspector() {
    let w = check_landmarks_inspector(landmarks);
    eprintln!(
        "shape_landmarks_qa_inspector: worst {w:.3e} ({:.1} ε)",
        w / EPS
    );
}

negative_control!(
    shape_landmarks_qa_inspector,
    "equal-mass landmarks against the inspector's mass-weighted collisions must fail",
    expected = "from the inspector's collision",
    {
        check_landmarks_inspector(|_| landmarks([1.0 / 3.0; 3]));
    }
);

// ── REQ-VAL-023: the dot on a collision ─────────────────────────────────────────────────────────────────────────────

/// Bodies `i` and `j` brought together along `r_j(ε) = r_i + ε(r_j − r_i)`: the dot's distance to the landmark of the
/// side opposite `k` (the third body) falls strictly as `ε` goes `10⁻³ → 10⁻⁵ → 10⁻⁷`, and two more decades at
/// `ε = 0`; its distance to the two other landmarks stays above half their distance from it.
fn check_dot(
    of: LandmarksFn,
    m: [f64; 3],
    r: Pos,
    i: usize,
    j: usize,
) -> Result<(), TestCaseError> {
    let k = 3 - i - j;
    let l = of(m);
    let at = |e: f64| {
        let mut x = r;
        x[j] = [
            r[i][0] + e * (r[j][0] - r[i][0]),
            r[i][1] + e * (r[j][1] - r[i][1]),
        ];
        shape(x, m)
    };
    let mut last = f64::INFINITY;
    for e in [1e-3, 1e-5, 1e-7] {
        let d = dist(at(e), l.bc[k]);
        prop_assert!(
            d < last,
            "bodies {i} and {j} at ε = {e}: the dot is {d:e} from landmark {k}, not nearer than {last:e}"
        );
        last = d;
    }
    // At coincidence the trend goes on: the dot is two more decades nearer than at ε = 10⁻⁷, as each step above is
    // (n is linear in ε at a collision). The rounding floor is not a fixed multiple of ε: `λ` is a difference of nearly
    // equal positions when the pair's masses differ widely.
    let d0 = dist(at(0.0), l.bc[k]);
    prop_assert!(
        d0 <= last / 100.0,
        "bodies {i} and {j} together: the dot is {d0:e} from landmark {k}, not two decades nearer than {last:e} \
         (masses {m:?})"
    );
    for o in (0..3).filter(|&o| o != k) {
        let sep = dist(l.bc[o], l.bc[k]);
        prop_assert!(sep > 0.0, "landmarks {o} and {k} coincide at masses {m:?}");
        for e in [1e-7, 0.0] {
            let d = dist(at(e), l.bc[o]);
            prop_assert!(
                d > sep / 2.0,
                "bodies {i} and {j} at ε = {e}: the dot is {d:e} from the other landmark {o} (separation {sep:e})"
            );
        }
    }
    Ok(())
}

fn collisions() -> impl Strategy<Value = ([f64; 3], Pos, usize, usize)> {
    (
        proptest::array::uniform3(-4.0..4.0f64),
        proptest::array::uniform3(proptest::array::uniform2(-2.0..2.0f64)),
        0usize..3,
        1usize..3,
    )
        .prop_map(|(w, r, i, step)| {
            let e = w.map(f64::exp);
            let t = e[0] + e[1] + e[2];
            (e.map(|x| x / t), r, i, (i + step) % 3)
        })
        .prop_filter("a triangle with its pairs apart", |(_, r, _, _)| {
            (0..3).all(|a| {
                let b = (a + 1) % 3;
                (r[a][0] - r[b][0]).hypot(r[a][1] - r[b][1]) > 0.1
            })
        })
}

#[test]
fn shape_dot_on_collision_landmark_qa() {
    prop::run(&collisions(), |(m, r, i, j)| {
        check_dot(landmarks, m, r, i, j)
    });
    eprintln!(
        "shape_dot_on_collision_landmark_qa: {}",
        prop::cases_status()
    );
}

negative_control!(
    shape_dot_on_collision_landmark_qa,
    "equal-mass landmarks at unequal masses are not where the dot lands, so it must fail",
    expected = "from landmark",
    {
        prop::run(&collisions(), |(m, r, i, j)| {
            check_dot(|_| landmarks([1.0 / 3.0; 3]), m, r, i, j)
        });
    }
);
