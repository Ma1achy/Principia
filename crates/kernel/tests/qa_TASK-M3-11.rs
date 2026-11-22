//! QA tests for TASK-M3-11, written from the requirements it closes and their sources, not from the implementation:
//! - REQ-INT-042 (dd_integrator §3.7, §5 test 9; R-14): the live shape point `n = (u, v, w)/I` from mass-weighted
//!   Jacobi coordinates. A reference written here from §3.7's formula, checked against §3.7's own landmark table
//!   (`b̂₀₁ = (−1, 0, 0)`, `b̂₁₂ = (½, √3/2, 0)`, `b̂₂₀ = (½, −√3/2, 0)`, `L⁺` the anticlockwise equilateral at `w = +1`);
//!   `‖n‖ = 1`, equilateral `w = ±1`, collinear `w = 0`; the kernel's `shape` against the reference and against the IC
//!   Inspector's `shapePoint` on its two checked-in fixtures (5,000 + 500 ICs) after the mirror fold, to f64 round-off,
//!   the register's `shape_vec_js_tol` (`qa_m311_shape_*`).
//! - REQ-PAY-055: the shape helper and the readout's per-step entry take positions and masses, never momenta: a
//!   compile-time signature check (`qa_m311_shape_takes_no_momenta`).
//! - REQ-INT-087 (payload §2 `_reserved`; R-397): every code round-trips, decode lies in (−π, π), the encode's error is
//!   at most `π/65535 + 4·ulp_Real(2π)` over every code's midpoints and the edge longitudes and never yields 0xFFFF; a
//!   truncating encode fails the bound and an encode without the `N → 0` wrap yields the sentinel
//!   (`qa_m311_theta_ref_code_*`).
//! - REQ-INT-088 (dd_integrator §3.7; payload §2; R-389, R-392, R-397, R-547): the hold's lifecycle on hand-built
//!   points: fresh 0xFFFF, entry writes the code of the last longitude outside, inside holds, exit adds
//!   `wrap(exit − decode(code))` and writes 0xFFFF, exactly ±π adds +π, an IC inside adds nothing at its first exit;
//!   "inside" decided from the current state alone, agreeing with `√(n_u² + n_v²) < r_pole` written here and invariant
//!   under any power-of-two scale of the configuration (R-547's "no subnormal intermediate"); and on a real march through
//!   the disc, outside always 0xFFFF and descriptor bits 10–14 `I_max`'s level, never decreasing, bit 15 the departed bit's (R-545)
//!   (`qa_m311_theta_hold_*`).
//! - REQ-INT-089 (R-397): a march through the disc, and one from an IC inside it, at f32 and f64, as one call and as
//!   k ∈ {2, 3, 7} resumed calls split on the entry step, strictly inside and on the exit step, each resume from the
//!   stored `SimState` with `packed_a` put through the generated unpack and pack: `θ̃`'s bits, `_reserved` and the
//!   closure registers identical; a control holding the reference unquantised within a dispatch fails
//!   (`qa_m311_theta_resume_split_*`).
//! - REQ-PAY-043, REQ-PAY-056 (payload § "Why closure is here"; dd_validation_orbits §6; R-37, R-545): on the
//!   figure-eight (Chenciner & Montgomery's published IC and period, dd_integrator §5 test 2) the stored registers after
//!   every step equal the offline minimum of `|n̂(t) − n̂(0)|` over the stored shape path from the first sample that
//!   exceeds `δ_dep`, and the argmin step; the return is at the period the orbit's symmetry predicts (`T/2` from the
//!   Euler configuration the published IC starts in, `T` from a generic phase); synthetic paths whose closest approach
//!   is before departure ignore it, and a chord of exactly `δ_dep` does not depart (`qa_m311_closure_*`).
//! - REQ-INT-082 (dd_integrator §5 test 8; R-580): a rotating figure-eight; `θ̃` against a hand count of the signed
//!   crossings of the three binary-collision half-meridians, with no `atan2`; `orbit_count` against that count at a
//!   step where it is unambiguous; the mirror run's `θ̃` and `retrograde` bitwise equal and `L_z` bitwise opposite at every
//!   step, `|L_z| > 0.01`; controls: `θ̃` with its sense reversed fails the direction only, `retrograde` from `L_z`,
//!   `θ̃` from `w`, and a positions-only mirror each fail (`qa_m311_theta_unwrap_*`).
//!
//! Each test has its registered negative control (R-176).
// The file name `qa_TASK-M3-11` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::f64::consts::{PI, TAU};

use kernel::decode::Ic;
use kernel::driver::cadence::WrapperCadence;
use kernel::driver::closure::Closure;
use kernel::driver::shape::{
    decode, delta_dep, encode, r_pole, theta_hold, Readout, ReadoutReal, NO_REFERENCE,
};
use kernel::driver::substep::NsubThresholds;
use kernel::driver::wrapper::{integrate, integrate_resumed_with, WrapperReal, WrapperUniforms};
use kernel::generated::constants::{THETA_REF_NONE, THETA_REF_STEPS};
use kernel::generated::nsub_thresholds::NSUB_TABLE_ROWS;
use kernel::occupant::kdk::Kdk;
use kernel::payload::roundtrip::PackedA;
use kernel::payload::{
    orbit_count, pack_packed_a, retrograde, sd_departed, set_departed, tm_t_end_step, DminCounters,
    SimStateBaseOf,
};
use kernel::shape::{config_in_disc, longitude, shape};
use spirv_std::glam::UVec4;
use spirv_std::num_traits::NumCast;
use validation::{negative_control, SHAPE_VEC_JS_TOL};

// ── shared ───────────────────────────────────────────────────────────────────────────────────────────────────────

/// The precisions the readout runs at, each widened exactly to f64 for the checks.
trait Prec: WrapperReal + ReadoutReal + Into<f64> + std::fmt::Debug {}

impl<R: WrapperReal + ReadoutReal + Into<f64> + std::fmt::Debug> Prec for R {}

fn real<R: Prec>(x: f64) -> R {
    <R as NumCast>::from(x).expect("a finite f64")
}

fn wide<R: Prec>(x: R) -> f64 {
    x.into()
}

fn bits<R: Prec>(x: R) -> u64 {
    wide(x).to_bits()
}

fn eps<R: Prec>() -> f64 {
    wide(R::epsilon())
}

/// A sim key with `N_sub = 1` throughout (every threshold 0).
fn uniforms<R: Prec>(dt: f64, horizon: u32) -> WrapperUniforms<R> {
    WrapperUniforms {
        dt_macro: real(dt),
        horizon_steps: horizon,
        n_max: 1,
        thresholds: NsubThresholds {
            rows: [UVec4::splat(0f32.to_bits()); NSUB_TABLE_ROWS],
        },
    }
}

/// `s` as stored and read back: `packed_a` through the generated unpack and pack (payload §6), which must give the
/// same word.
fn stored<R: Prec>(s: SimStateBaseOf<R>) -> SimStateBaseOf<R> {
    let f = PackedA::unpack(s.packed_a);
    let w = pack_packed_a(
        f.state,
        f.detail,
        f.saturated,
        f.dmin_pair,
        f.last_symbol,
        f.i_max,
        f.departed,
        f.d_min,
        &DminCounters::new(),
    );
    assert_eq!(
        w, s.packed_a,
        "packed_a does not survive the generated unpack and pack"
    );
    SimStateBaseOf { packed_a: w, ..s }
}

/// `steps` more macro-steps of `ic`'s march from the stored `s`, under a horizon `horizon`.
fn resume<R: Prec>(
    ic: &Ic<R>,
    dt: f64,
    horizon: u32,
    s: &SimStateBaseOf<R>,
    steps: u32,
) -> SimStateBaseOf<R> {
    let s = stored(*s);
    let mut cadence = WrapperCadence::new(&ic.m, &s.r, &s.p);
    integrate_resumed_with::<Kdk, R, _>(ic, &uniforms(dt, horizon), &mut cadence, &s, steps)
}

/// The stored `SimState` after every macro-step `0 … steps` of `ic`, each a resumed call of one step.
fn path<R: Prec>(ic: &Ic<R>, dt: f64, steps: u32) -> Vec<SimStateBaseOf<R>> {
    let mut s = integrate::<Kdk, R>(ic, &uniforms(dt, 0));
    let mut out = vec![s];
    for k in 1..=steps {
        s = resume(ic, dt, steps, &s, 1);
        assert_eq!(
            tm_t_end_step(s.times),
            k,
            "a one-step resume did not end at step {k}"
        );
        out.push(s);
    }
    out
}

/// `|a − b|` on the circle, in `[0, π]`.
fn circle(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(TAU);
    d.min(TAU - d)
}

/// R-389's wrap written from its words: into (−π, π], exactly ±π giving +π.
fn wrap64(d: f64) -> f64 {
    let mut d = d;
    while d > PI {
        d -= TAU;
    }
    while d <= -PI {
        d += TAU;
    }
    d
}

/// `ulp_Real(2π)`: 2π is in [4, 8), so its ulp is 4ε.
fn ulp_tau<R: Prec>() -> f64 {
    4.0 * eps::<R>()
}

// ── the shape reference, from dd_integrator §3.7 ─────────────────────────────────────────────────────────────────

/// §3.7's map at f64: `ρ = r₁ − r₀`, `λ = r₂ − (m₀r₀ + m₁r₁)/(m₀ + m₁)`, mass-weighted by `√μ₁`, `√μ₂`, `μ₁ = m₀m₁/(m₀ +
/// m₁)`, `μ₂ = (m₀ + m₁)m₂/M`; `u = ‖ρ̃‖² − ‖λ̃‖²`, `v = 2ρ̃·λ̃`, `w = 2ρ̃∧λ̃`, over `I = ‖ρ̃‖² + ‖λ̃‖²`.
fn shape_ref(r: [[f64; 2]; 3], m: [f64; 3]) -> [f64; 3] {
    let m01 = m[0] + m[1];
    let mu1 = m[0] * m[1] / m01;
    let mu2 = m01 * m[2] / (m01 + m[2]);
    let c = [
        (m[0] * r[0][0] + m[1] * r[1][0]) / m01,
        (m[0] * r[0][1] + m[1] * r[1][1]) / m01,
    ];
    let rt = [
        mu1.sqrt() * (r[1][0] - r[0][0]),
        mu1.sqrt() * (r[1][1] - r[0][1]),
    ];
    let lt = [mu2.sqrt() * (r[2][0] - c[0]), mu2.sqrt() * (r[2][1] - c[1])];
    let rr = rt[0] * rt[0] + rt[1] * rt[1];
    let ll = lt[0] * lt[0] + lt[1] * lt[1];
    let i = rr + ll;
    [
        (rr - ll) / i,
        2.0 * (rt[0] * lt[0] + rt[1] * lt[1]) / i,
        2.0 * (rt[0] * lt[1] - rt[1] * lt[0]) / i,
    ]
}

/// §3.7's equal-mass binary-collision directions `b̂₀₁, b̂₁₂, b̂₂₀`, from its table.
fn collisions() -> [[f64; 3]; 3] {
    let h = 3f64.sqrt() / 2.0;
    [[-1.0, 0.0, 0.0], [0.5, h, 0.0], [0.5, -h, 0.0]]
}

fn max_diff(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).fold(0.0f64, |w, i| w.max((a[i] - b[i]).abs()))
}

/// A seeded uniform stream (SplitMix64), so every draw is reproducible.
struct Draw(u64);

impl Draw {
    fn unit(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        ((z ^ (z >> 31)) >> 11) as f64 / (1u64 << 53) as f64
    }

    fn on(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.unit()
    }

    /// A random configuration and masses on the simplex (`M = 1`, the sim's masses, which the map's `μ_λ = m₂M₀₁`
    /// assumes), its bodies at least 0.05 apart (the identities' regime, away from the collisions where the map's
    /// conditioning, not its round-off, would set the error).
    fn config(&mut self) -> ([[f64; 2]; 3], [f64; 3]) {
        loop {
            let r = [0; 3].map(|_| [self.on(-1.0, 1.0), self.on(-1.0, 1.0)]);
            let m = [0; 3].map(|_| self.on(0.05, 1.0));
            let total = m[0] + m[1] + m[2];
            let m = m.map(|x| x / total);
            let apart = |i: usize, j: usize| (r[i][0] - r[j][0]).hypot(r[i][1] - r[j][1]) >= 0.05;
            if apart(0, 1) && apart(1, 2) && apart(2, 0) {
                return (r, m);
            }
        }
    }
}

/// `r` rotated by `a`, scaled by `k` and moved by `t`.
fn moved(r: [[f64; 2]; 3], a: f64, k: f64, t: [f64; 2]) -> [[f64; 2]; 3] {
    let (s, c) = a.sin_cos();
    r.map(|[x, y]| [k * (c * x - s * y) + t[0], k * (s * x + c * y) + t[1]])
}

// ── REQ-INT-042: the shape identities and the inspector ──────────────────────────────────────────────────────────

/// A shape map under test at f64: the kernel's, or a control's.
type Map = fn([[f64; 2]; 3], [f64; 3]) -> [f64; 3];

fn kernel_shape(r: [[f64; 2]; 3], m: [f64; 3]) -> [f64; 3] {
    shape(r, m)
}

/// The reference against §3.7's landmarks: each pair's collision at its `b̂` (with that pair coincident, the third body
/// anywhere, equal masses); the anticlockwise equilateral 0 → 1 → 2 at `L⁺ = (0, 0, 1)`, the clockwise one at `L⁻`.
fn check_landmarks(map: Map, tol: f64) {
    let m = [1.0 / 3.0; 3];
    let b = collisions();
    let pairs = [(0, 1, 2), (1, 2, 0), (2, 0, 1)];
    for (k, &(i, j, o)) in pairs.iter().enumerate() {
        let mut r = [[0.0; 2]; 3];
        r[i] = [0.3, -0.2];
        r[j] = [0.3, -0.2];
        r[o] = [-0.7, 0.9];
        for a in [0.0, 1.1, -2.5] {
            let n = map(moved(r, a, 1.7, [0.2, -0.4]), m);
            assert!(
                max_diff(n, b[k]) <= tol,
                "pair {i}{j}'s collision at {n:?}, not §3.7's {:?} (landmark)",
                b[k]
            );
        }
    }
    let tri = [90.0f64, 210.0, 330.0].map(|d| [d.to_radians().cos(), d.to_radians().sin()]);
    for a in [0.0, 0.4, 3.0] {
        let n = map(moved(tri, a, 0.6, [1.0, 2.0]), m);
        assert!(
            max_diff(n, [0.0, 0.0, 1.0]) <= tol,
            "the anticlockwise equilateral at {n:?}, not L⁺ (landmark)"
        );
        let cw = [tri[0], tri[2], tri[1]];
        let n = map(moved(cw, a, 0.6, [1.0, 2.0]), m);
        assert!(
            max_diff(n, [0.0, 0.0, -1.0]) <= tol,
            "the clockwise equilateral at {n:?}, not L⁻ (landmark)"
        );
    }
}

#[test]
fn qa_m311_shape_reference_has_section_3_7_landmarks() {
    check_landmarks(shape_ref, 8.0 * f64::EPSILON);
}

negative_control!(
    qa_m311_shape_reference_has_section_3_7_landmarks,
    "a map with w's sign flipped puts the anticlockwise equilateral at L⁻, so the landmark check must fail",
    expected = "(landmark)",
    check_landmarks(
        |r, m| {
            let n = shape_ref(r, m);
            [n[0], n[1], -n[2]]
        },
        8.0 * f64::EPSILON
    )
);

/// The identities at the precision `R`, through the kernel's map: on random ICs `‖n‖ = 1`, invariance under rotation,
/// translation and scale, the mirror `y → −y` keeping `u, v` and negating `w`, and agreement with the reference; on
/// collinear configurations `w = 0` (exactly, on the x axis); at equal masses the equilateral at `w = ±1`. The tolerance
/// is `k` ulps of the precision (round-off of the map's few operations).
fn check_identities<R: Prec>(k: f64) {
    let tol = k * eps::<R>();
    let at = |r: [[f64; 2]; 3], m: [f64; 3]| -> [f64; 3] {
        shape(r.map(|v| v.map(real::<R>)), m.map(real::<R>)).map(wide)
    };
    let mut d = Draw(0x5eed_0311);
    for _ in 0..2000 {
        let (r, m) = d.config();
        let n = at(r, m);
        let norm = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        assert!((norm - 1.0).abs() <= tol, "‖n‖ = {norm} at {r:?}, {m:?}");
        // The reference on the inputs as rounded to R, so only the map's own round-off is measured.
        let n_ref = shape_ref(
            r.map(|v| v.map(|x| wide(real::<R>(x)))),
            m.map(|x| wide(real::<R>(x))),
        );
        // The reference at f64 against the kernel at R: R's round-off of the inputs and the map.
        assert!(
            max_diff(n, n_ref) <= tol.max(1e3 * f64::EPSILON),
            "n {n:?} against the reference {n_ref:?} (reference)"
        );
        let mirror = at(r.map(|[x, y]| [x, -y]), m);
        assert!(
            (mirror[0] - n[0]).abs() <= tol
                && (mirror[1] - n[1]).abs() <= tol
                && (mirror[2] + n[2]).abs() <= tol,
            "the mirror's n {mirror:?} is not (u, v, −w) of {n:?}"
        );
    }
    // Rotation, translation and power-of-two scale move no shape (in exact arithmetic; in R by round-off).
    for _ in 0..200 {
        let (r, m) = d.config();
        let n = shape_ref(r, m);
        let a = d.on(-PI, PI);
        let shifted = at(moved(r, a, 4.0, [d.on(-2.0, 2.0), d.on(-2.0, 2.0)]), m);
        assert!(
            max_diff(shifted, n) <= tol.max(1e3 * f64::EPSILON) * 4.0,
            "a rigid motion and scale moved n from {n:?} to {shifted:?}"
        );
    }
    for _ in 0..200 {
        let x = [d.on(-1.0, 1.0), d.on(-1.0, 1.0), d.on(-1.0, 1.0)];
        let m = [d.on(0.05, 1.0), d.on(0.05, 1.0), d.on(0.05, 1.0)];
        let total = m[0] + m[1] + m[2];
        let m = m.map(|x| x / total);
        let n = at([[x[0], 0.0], [x[1], 0.0], [x[2], 0.0]], m);
        assert_eq!(
            n[2], 0.0,
            "a collinear configuration on the x axis has w = {}",
            n[2]
        );
        let n = at(
            moved(
                [[x[0], 0.0], [x[1], 0.0], [x[2], 0.0]],
                0.7,
                1.0,
                [0.1, 0.3],
            ),
            m,
        );
        assert!(
            n[2].abs() <= tol,
            "a rotated collinear configuration has w = {}",
            n[2]
        );
    }
    let tri = [90.0f64, 210.0, 330.0].map(|d| [d.to_radians().cos(), d.to_radians().sin()]);
    for a in [0.0, 1.0, 2.0, -0.3] {
        let n = at(moved(tri, a, 0.9, [0.3, 0.1]), [1.0 / 3.0; 3]);
        assert!(
            (n[2] - 1.0).abs() <= tol,
            "the anticlockwise equilateral has w = {}",
            n[2]
        );
        let n = at(
            moved([tri[0], tri[2], tri[1]], a, 0.9, [0.3, 0.1]),
            [1.0 / 3.0; 3],
        );
        assert!(
            (n[2] + 1.0).abs() <= tol,
            "the clockwise equilateral has w = {}",
            n[2]
        );
    }
}

#[test]
fn qa_m311_shape_identities_f64() {
    check_identities::<f64>(16.0);
    check_landmarks(kernel_shape, 16.0 * f64::EPSILON);
}

#[test]
fn qa_m311_shape_identities_f32() {
    check_identities::<f32>(16.0);
    check_landmarks(kernel_shape_f32, 16.0 * f32::EPSILON as f64);
}

/// The kernel's map at f32, its inputs rounded to f32 and its output widened.
fn kernel_shape_f32(r: [[f64; 2]; 3], m: [f64; 3]) -> [f64; 3] {
    shape(r.map(|v| v.map(|x| x as f32)), m.map(|x| x as f32)).map(|x| x as f64)
}

negative_control!(
    qa_m311_shape_identities_f32,
    "the f32 map with w's sign flipped puts the anticlockwise equilateral at L⁻",
    expected = "(landmark)",
    check_landmarks(
        |r, m| {
            let n = kernel_shape_f32(r, m);
            [n[0], n[1], -n[2]]
        },
        16.0 * f32::EPSILON as f64
    )
);

negative_control!(
    qa_m311_shape_identities_f64,
    "a kernel map checked against a reference with u and v swapped must fail the reference comparison",
    expected = "(reference)",
    {
        let mut d = Draw(7);
        let (r, m) = d.config();
        let n = shape(r, m);
        let s = shape_ref(r, m);
        let swapped = [s[1], s[0], s[2]];
        assert!(
            max_diff(n, swapped) <= 1e-12,
            "n {n:?} against the reference {swapped:?} (reference)"
        );
    }
);

/// One IC of an inspector fixture: the raw positions, the masses, and the inspector's canonical `n` (`w ≥ 0`).
type JsIc = ([[f64; 2]; 3], [f64; 3], [f64; 3]);

fn hex(v: &serde_json::Value) -> f64 {
    f64::from_bits(u64::from_str_radix(v.as_str().expect("hex"), 16).expect("hex bits"))
}

fn inspector(file: &str) -> Vec<JsIc> {
    let path = format!(
        "{}/../../fixtures/ic_inspector/{file}",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let json: serde_json::Value = serde_json::from_str(&text).expect("the fixture's JSON");
    json["raw"]
        .as_array()
        .expect("raw")
        .iter()
        .map(|row| {
            let r = [0, 1, 2].map(|i| [hex(&row["r"][i][0]), hex(&row["r"][i][1])]);
            let m = [0, 1, 2].map(|i| hex(&row["m"][i]));
            let n = [0, 1, 2].map(|i| hex(&row["n"][i]));
            (r, m, n)
        })
        .collect()
}

/// `map`'s `n` against the inspector's on `file`'s ICs, `w` in magnitude (the inspector's canonical representative is
/// the upper hemisphere: the mirror fold), within the register's `shape_vec_js_tol` in every component. Returns the
/// count.
fn check_inspector(map: Map, file: &str) -> usize {
    let tol = SHAPE_VEC_JS_TOL.number();
    let ics = inspector(file);
    for (k, &(r, m, js)) in ics.iter().enumerate() {
        assert!(
            js[2] >= 0.0,
            "{file} IC {k}: the inspector's n is not folded"
        );
        let n = map(r, m);
        let folded = [n[0], n[1], n[2].abs()];
        assert!(
            max_diff(folded, js) <= tol,
            "{file} IC {k}: n {folded:?} against the inspector's {js:?} (inspector)"
        );
    }
    ics.len()
}

#[test]
fn qa_m311_shape_inspector_agreement() {
    assert_eq!(check_inspector(kernel_shape, "shape_live.json"), 5000);
    assert_eq!(check_inspector(kernel_shape, "shape_vec.json"), 500);
    // The reference too, so the identities above are checked against the inspector's convention.
    check_inspector(shape_ref, "shape_live.json");
}

negative_control!(
    qa_m311_shape_inspector_agreement,
    "a map with u and v swapped must disagree with the inspector",
    expected = "(inspector)",
    {
        check_inspector(
            |r, m| {
                let n = shape(r, m);
                [n[1], n[0], n[2]]
            },
            "shape_live.json",
        );
    }
);

// ── REQ-PAY-055: positions only ──────────────────────────────────────────────────────────────────────────────────

/// The readout's per-step entry: the readout, the positions, the masses and the step.
type AfterMacro = fn(&mut Readout<f64>, [[f64; 2]; 3], [f64; 3], u32);

/// The readout, fresh at the Euler configuration (longitude 0), stepped to the positions `given`: its `θ̃` is the
/// turning to the shape of `expected`, the positions alone deciding it (no momentum is an argument).
fn check_from_positions(given: [[f64; 2]; 3], expected: [[f64; 2]; 3]) {
    let mut a = Readout::<f64>::fresh(euler_point(), MASSES);
    a.after_macro(given, MASSES, 1);
    let want = wrap64(shape_ref(expected, MASSES)[1].atan2(shape_ref(expected, MASSES)[0]));
    assert!(
        (a.theta - want).abs() <= 1e-12,
        "θ̃ {} is not the turning to the positions' shape, {want} (positions)",
        a.theta
    );
}

/// A configuration well outside the pole disc and off the equator.
fn scalene() -> [[f64; 2]; 3] {
    [[-0.6, -0.1], [0.5, -0.3], [0.2, 0.7]]
}

#[test]
fn qa_m311_shape_takes_no_momenta() {
    // The shape helper and the readout's per-step entry are functions of positions and masses (and the step) alone:
    // these coercions compile only while neither takes a momentum.
    let _shape: fn([[f64; 2]; 3], [f64; 3]) -> [f64; 3] = kernel::driver::shape::shape::<f64>;
    let _after: AfterMacro = Readout::<f64>::after_macro;
    let _fresh: fn([[f64; 2]; 3], [f64; 3]) -> Readout<f64> = Readout::<f64>::fresh;
    check_from_positions(scalene(), scalene());
}

negative_control!(
    qa_m311_shape_takes_no_momenta,
    "a readout given other positions than the ones it is checked against fails",
    expected = "(positions)",
    check_from_positions(scalene(), equilateral_tilted())
);

/// A second configuration outside the disc, for the control.
#[cfg(feature = "controls")]
fn equilateral_tilted() -> [[f64; 2]; 3] {
    [[-0.5, 0.0], [0.5, 0.0], [0.1, 0.4]]
}

// ── REQ-INT-087: the code ────────────────────────────────────────────────────────────────────────────────────────

/// An encode under test at `R`.
type Encode<R> = fn(R) -> u32;

fn kernel_encode<R: Prec>(lon: R) -> u32 {
    encode(lon)
}

/// The longitudes swept at `R`: every code's midpoint and its neighbours one ulp either side, the codes themselves,
/// the edges `−π`, `−0`, `0`, `π` and their neighbours, small negatives (`λ' = λ + 2π` just below `2π`), and a
/// uniform draw.
fn sweep<R: Prec>() -> Vec<R> {
    let n = 65535.0;
    let mut out = Vec::new();
    let at = |x: f64| -> f64 {
        if x > PI {
            x - TAU
        } else {
            x
        }
    };
    for c in 0..65535u32 {
        for x in [at(c as f64 * TAU / n), at((c as f64 + 0.5) * TAU / n)] {
            let r: R = real(x);
            out.push(r);
            out.push(r.next_after_up());
            out.push(r.next_after_down());
        }
    }
    let pi = R::PI();
    for x in [-pi, -R::zero(), R::zero(), pi] {
        out.push(x);
    }
    out.push((-pi).next_after_up());
    out.push(pi.next_after_down());
    for x in [
        R::min_positive_value(),
        R::epsilon(),
        real(1e-6),
        real(TAU / n / 4.0),
    ] {
        out.push(-x);
    }
    let mut d = Draw(0xc0de);
    for _ in 0..100_000 {
        out.push(real(d.on(-PI, PI)));
    }
    let pi = wide(R::PI());
    out.retain(|x| wide(*x) >= -pi && wide(*x) <= pi);
    out
}

/// One ulp up and down for the float types, through their bits.
trait NextAfter {
    fn next_after_up(self) -> Self;
    fn next_after_down(self) -> Self;
}

impl<R: Prec> NextAfter for R {
    fn next_after_up(self) -> Self {
        step_ulp(self, 1)
    }
    fn next_after_down(self) -> Self {
        step_ulp(self, -1)
    }
}

fn step_ulp<R: Prec>(x: R, dir: i64) -> R {
    let w = wide(x);
    if eps::<R>() == f32::EPSILON as f64 {
        let f = w as f32;
        let b = f.to_bits() as i64;
        let next = if (f >= 0.0) == (dir > 0) {
            b + 1
        } else {
            b - 1
        };
        let f = if f == 0.0 {
            if dir > 0 {
                f32::from_bits(1)
            } else {
                -f32::from_bits(1)
            }
        } else {
            f32::from_bits(next as u32)
        };
        real(f as f64)
    } else {
        let b = w.to_bits() as i64;
        let next = if (w >= 0.0) == (dir > 0) {
            b + 1
        } else {
            b - 1
        };
        let f = if w == 0.0 {
            if dir > 0 {
                f64::from_bits(1)
            } else {
                -f64::from_bits(1)
            }
        } else {
            f64::from_bits(next as u64)
        };
        real(f)
    }
}

/// REQ-INT-087's code at `R` with `enc` as the encode: every code round-trips and decodes into (−π, π) within round-off
/// of `c·2π/65535` (less 2π above π); over the sweep the encode is never 0xFFFF, small negatives encode to 0, and the
/// decoded code is within `π/65535 + 4·ulp_Real(2π)` of the longitude, mod 2π. Returns the worst error.
fn check_code<R: Prec>(enc: Encode<R>) -> f64 {
    assert_eq!(THETA_REF_STEPS, 65535, "theta_ref_steps");
    assert_eq!(THETA_REF_NONE, 0xFFFF, "theta_ref_none");
    assert_eq!(NO_REFERENCE, 0xFFFF, "the readout's sentinel");
    for c in 0..65535u32 {
        let d: R = decode(c);
        let x = wide(d);
        assert!(x > -PI && x < PI, "decode({c}) = {x} is not in (−π, π)");
        let want = c as f64 * TAU / 65535.0;
        let want = if want > PI { want - TAU } else { want };
        assert!(
            (x - want).abs() <= 4.0 * ulp_tau::<R>(),
            "decode({c}) = {x}, not c·2π/65535 = {want}"
        );
        assert_eq!(enc(d), c, "encode(decode({c})) is not {c} (round trip)");
    }
    let bound = PI / 65535.0 + 4.0 * ulp_tau::<R>();
    let mut worst = 0.0f64;
    for lon in sweep::<R>() {
        let c = enc(lon);
        assert!(c != 0xFFFF, "encode({lon:?}) returned the sentinel 0xFFFF");
        assert!(c < 65535, "encode({lon:?}) = {c} is not a code");
        let err = circle(wide(decode::<R>(c)), wide(lon));
        worst = worst.max(err);
        assert!(
            err <= bound,
            "encode({lon:?}) = {c}: decoded {} is {err:e} from it, beyond π/65535 + 4·ulp(2π) = {bound:e} (bound)",
            wide(decode::<R>(c))
        );
    }
    for x in [R::min_positive_value(), R::epsilon()] {
        assert_eq!(
            enc(-x),
            0,
            "a longitude just below 0 (λ' just below 2π) encodes to 0"
        );
    }
    worst
}

#[test]
fn qa_m311_theta_ref_code() {
    let w32 = check_code::<f32>(kernel_encode);
    let w64 = check_code::<f64>(kernel_encode);
    println!(
        "worst encode error over π/65535: f32 {:.3e}, f64 {:.3e}",
        w32 - PI / 65535.0,
        w64 - PI / 65535.0
    );
}

/// The encode truncating (no `+ ½`): its error reaches a whole step, `2π/65535`.
#[cfg(feature = "controls")]
fn truncating<R: Prec>(lon: R) -> u32 {
    let l = wide(lon);
    let l = if l < 0.0 { l + TAU } else { l };
    let c = (l * 65535.0 / TAU).floor() as u32;
    if c >= 65535 {
        0
    } else {
        c
    }
}

/// The encode without the `N → 0` wrap: a longitude just below 0, `λ'` just below 2π, rounds to 65535, the sentinel.
#[cfg(feature = "controls")]
fn unwrapped<R: Prec>(lon: R) -> u32 {
    let l = wide(lon);
    let l = if l < 0.0 { l + TAU } else { l };
    (l * 65535.0 / TAU + 0.5).floor() as u32
}

negative_control!(
    qa_m311_theta_ref_code,
    "a truncating encode must exceed π/65535 + 4·ulp(2π)",
    expected = "(bound)",
    {
        // The round trip of a truncating encode holds on the codes themselves only where the decode is not below
        // the code's value, so the bound is checked first: on the midpoints alone.
        let bound = PI / 65535.0 + 4.0 * ulp_tau::<f64>();
        for c in 0..65535u32 {
            let lon = (c as f64 + 0.75) * TAU / 65535.0;
            let lon = if lon > PI { lon - TAU } else { lon };
            let e = truncating::<f64>(lon);
            let err = circle(decode::<f64>(e), lon);
            assert!(err <= bound, "truncating encode {err:e} (bound)");
        }
    }
);

#[test]
fn qa_m311_theta_ref_code_never_sentinel() {
    check_code::<f64>(kernel_encode);
}

negative_control!(
    qa_m311_theta_ref_code_never_sentinel,
    "an encode without the N → 0 wrap returns 0xFFFF for a longitude just below 0",
    expected = "returned the sentinel",
    {
        check_code::<f64>(unwrapped);
    }
);

// ── REQ-INT-088: the hold's lifecycle ────────────────────────────────────────────────────────────────────────────

/// Equal masses, the fixtures' (the landmarks of §3.7 are at equal masses).
const MASSES: [f64; 3] = [1.0 / 3.0; 3];

/// A unit point at longitude `lon` and latitude height `w`.
fn at_lon<R: Prec>(lon: f64, w: f64) -> [R; 3] {
    let c = (1.0 - w * w).sqrt();
    [real(c * lon.cos()), real(c * lon.sin()), real(w)]
}

/// The equal-mass equilateral (inside the disc: `n = (0, 0, 1)`), its circumradius 0.8.
fn equilateral() -> [[f64; 2]; 3] {
    [90.0f64, 210.0, 330.0].map(|d| [0.8 * d.to_radians().cos(), 0.8 * d.to_radians().sin()])
}

/// A collinear configuration, body 2 at the midpoint of 0 and 1: `n = (1, 0, 0)`, longitude 0, far outside.
fn euler_point() -> [[f64; 2]; 3] {
    [[-1.0, 0.0], [1.0, 0.0], [0.0, 0.0]]
}

fn readout<R: Prec>(r: [[f64; 2]; 3]) -> Readout<R> {
    Readout::fresh(r.map(|v| v.map(real)), MASSES.map(real))
}

/// The lifecycle on hand-built points at `R` (each assertion R-389's, R-392's or payload §2's words).
fn check_lifecycle<R: Prec>() {
    let tol = 8.0 * ulp_tau::<R>();
    // Fresh: 0xFFFF and θ̃ = 0, outside and inside alike.
    for r in [euler_point(), equilateral()] {
        let f = readout::<R>(r);
        assert_eq!(f.code, 0xFFFF, "a fresh sample's _reserved");
        assert_eq!(bits(f.theta), 0, "a fresh sample's θ̃");
    }
    // Outside → outside: θ̃ adds the wrapped longitude difference, _reserved stays 0xFFFF.
    let mut o = readout::<R>(euler_point());
    let mut theta = 0.0;
    let mut prev = 0.0;
    for (k, lon) in [0.5, 1.4, 2.9, -2.8, -1.0, 0.3].into_iter().enumerate() {
        o.point(at_lon(lon, 0.2), false, k as u32 + 1);
        theta += wrap64(lon - prev);
        prev = lon;
        assert_eq!(
            o.code,
            0xFFFF,
            "_reserved outside the disc after step {}",
            k + 1
        );
        assert!(
            (wide(o.theta) - theta).abs() <= tol,
            "θ̃ {:?} against {theta} outside",
            o.theta
        );
    }
    // Entry: the code of the last longitude outside; θ̃ holds.
    let last: [R; 3] = at_lon(prev, 0.2);
    let before = o.theta;
    o.point(at_lon(1.0, 0.99999), true, 7);
    assert_eq!(
        o.code,
        encode(longitude(last)),
        "the entry's code is not the last longitude outside's"
    );
    assert!(
        circle(wide(decode::<R>(o.code)), prev) <= PI / 65535.0 + 4.0 * ulp_tau::<R>(),
        "the entry's code decodes far from the last longitude outside"
    );
    assert_eq!(bits(o.theta), bits(before), "θ̃ moved on entry");
    // Inside → inside: both hold, whatever the longitude does.
    let code = o.code;
    for lon in [-2.0, 3.0, 0.1] {
        o.point(at_lon(lon, 0.99999), true, 8);
        assert_eq!(
            (bits(o.theta), o.code),
            (bits(before), code),
            "the hold moved inside"
        );
    }
    // Exit: θ̃ adds wrap(exit − decode(code)); _reserved back to 0xFFFF.
    let exit = -2.2;
    o.point(at_lon(exit, 0.1), false, 9);
    let want = wide(before) + wrap64(exit - wide(decode::<R>(code)));
    assert!(
        (wide(o.theta) - want).abs() <= tol,
        "the exit added {:?}, not {want}",
        o.theta
    );
    assert_eq!(o.code, 0xFFFF, "_reserved after the exit");
    // An IC inside: 0xFFFF throughout the hold and nothing added at its first exit; then it counts on.
    let mut i = readout::<R>(equilateral());
    i.point(at_lon(2.0, 0.99999), true, 1);
    assert_eq!((i.code, bits(i.theta)), (0xFFFF, 0), "an IC inside, inside");
    i.point(at_lon(-1.3, 0.3), false, 2);
    assert_eq!(
        (i.code, bits(i.theta)),
        (0xFFFF, 0),
        "an IC inside added at its first exit (R-392)"
    );
    i.point(at_lon(-1.0, 0.3), false, 3);
    assert!(
        (wide(i.theta) - 0.3).abs() <= tol,
        "after the first exit θ̃ counts from the exit longitude"
    );
}

#[test]
fn qa_m311_theta_hold_lifecycle() {
    check_lifecycle::<f32>();
    check_lifecycle::<f64>();
}

negative_control!(
    qa_m311_theta_hold_lifecycle,
    "a hold that writes its code on the step before entry leaves _reserved set outside the disc",
    expected = "_reserved outside the disc",
    {
        let mut o = readout::<f64>(euler_point());
        o.point(at_lon(0.5, 0.2), false, 1);
        o.code = encode(0.5f64);
        assert_eq!(o.code, 0xFFFF, "_reserved outside the disc after step 1");
    }
);

/// A hold step under test: `(θ̃, code, n_prev, prev_inside, n, inside) → (θ̃, code)`.
type Hold<R> = fn(R, u32, [R; 3], bool, [R; 3], bool) -> (R, u32);

/// The exact-π case at `R` (dd_integrator §3.7): reference longitude exactly 0 (code 0), exit at exactly `+π`
/// (`atan2(+0, −1)`) and at exactly `−π` (`atan2(−0, −1)`): both add `+π`, exactly. At f64 only: the host's f32
/// `atan2(+0, −1)` is `0x40490fda`, one ulp below f32's π `0x40490fdb` (measured), so at f32 no exit of this form
/// differs from the code 0 by exactly ±π; at f32 the case is found by search ([`check_exact_pi_search`]).
fn check_exact_pi<R: Prec>(hold: Hold<R>) {
    let one = R::one();
    let zero = R::zero();
    let start = [one, zero, zero];
    let pole = [zero, zero, one];
    let (theta, code) = hold(zero, 0xFFFF, start, false, pole, true);
    assert_eq!(code, 0, "the code of longitude 0");
    assert_eq!(bits(longitude([-one, zero, zero])), bits(R::PI()));
    assert_eq!(bits(longitude([-one, -zero, zero])), bits(-R::PI()));
    for exit in [[-one, zero, zero], [-one, -zero, zero]] {
        let (after, c) = hold(theta, code, pole, true, exit, false);
        assert_eq!(c, 0xFFFF);
        assert_eq!(
            bits(after),
            bits(R::PI()),
            "an exit exactly ±π from the reference added {after:?}, not +π (exact π)"
        );
    }
}

/// The exact-π case at `R` through the hold, on exits found by search: for codes across the circle, the exit point at
/// `decode(c) ± π` (rounded to `R`), kept where the hold's difference `longitude(n) − decode(c)` is exactly `+π` or
/// `−π` at `R`; each must add exactly `+π` and write 0xFFFF. Both signs must be found. Returns their counts.
fn check_exact_pi_search<R: Prec>(hold: Hold<R>) -> (usize, usize) {
    let pi = R::PI();
    let zero = R::zero();
    let pole = [zero, zero, R::one()];
    let (mut plus, mut minus) = (0, 0);
    for c in (0..65535u32).step_by(7) {
        let reference: R = decode(c);
        for sign in [1.0, -1.0] {
            let t = wrap64(wide(reference) + sign * wide(pi));
            let n: [R; 3] = [real(t.cos()), real(t.sin()), zero];
            let d = longitude(n) - reference;
            if d != pi && d != -pi {
                continue;
            }
            let (after, code) = hold(zero, c, pole, true, n, false);
            assert_eq!(code, 0xFFFF, "code {c}: the exit did not write 0xFFFF");
            assert_eq!(
                bits(after),
                bits(pi),
                "code {c}: an exit {d:?} from the reference added {after:?}, not +π (exact π)"
            );
            if d == pi {
                plus += 1;
            } else {
                minus += 1;
            }
        }
    }
    assert!(
        plus > 0 && minus > 0,
        "the search found {plus} exits at +π and {minus} at −π"
    );
    (plus, minus)
}

#[test]
fn qa_m311_theta_hold_exact_pi() {
    check_exact_pi::<f64>(theta_hold);
    let f32_found = check_exact_pi_search::<f32>(theta_hold);
    let f64_found = check_exact_pi_search::<f64>(theta_hold);
    println!("exits exactly (+π, −π) from a code: f32 {f32_found:?}, f64 {f64_found:?}");
    // f32: a difference of exactly ±π wraps to +π, and nothing else in (−π, π] moves.
    let pi = std::f32::consts::PI;
    for (d, want) in [
        (pi, pi),
        (-pi, pi),
        (0.5, 0.5),
        (-3.0, -3.0),
        (pi + 0.5, 0.5 - pi),
    ] {
        let got = kernel::shape::wrap(d);
        assert!(
            (got - want).abs() <= 4.0 * f32::EPSILON,
            "wrap({d}) = {got}, not {want} (exact π)"
        );
    }
    assert_eq!(
        kernel::shape::wrap(-pi).to_bits(),
        pi.to_bits(),
        "wrap(−π) at f32 (exact π)"
    );
}

negative_control!(
    qa_m311_theta_hold_exact_pi,
    "a hold adding the raw difference adds −π for the exit at −π",
    expected = "(exact π)",
    check_exact_pi::<f64>(|theta, code, n_prev, prev_inside, n, inside| {
        let (t, c) = theta_hold(theta, code, n_prev, prev_inside, n, inside);
        if prev_inside && !inside {
            (theta + (longitude(n) - decode::<f64>(code)), c)
        } else {
            (t, c)
        }
    })
);

/// `√(n_u² + n_v²)` of the configuration at f64, by the reference.
fn pole_radius(r: [[f64; 2]; 3], m: [f64; 3]) -> f64 {
    let n = shape_ref(r, m);
    n[0].hypot(n[1])
}

/// A near-equilateral configuration of masses `m` (a random triangle shrunk toward the equilateral by `s`).
fn near_pole(d: &mut Draw, s: f64) -> ([[f64; 2]; 3], [f64; 3]) {
    let tri = equilateral();
    let r = tri.map(|[x, y]| [x + s * d.on(-1.0, 1.0), y + s * d.on(-1.0, 1.0)]);
    let r = moved(
        r,
        d.on(-PI, PI),
        d.on(0.5, 2.0),
        [d.on(-1.0, 1.0), d.on(-1.0, 1.0)],
    );
    (r, MASSES)
}

/// A disc test under test at `R`.
type Disc<R> = fn([[R; 2]; 3], [R; 3], R) -> bool;

/// The march's disc test at `R` against `√(n_u² + n_v²) < r_pole` written here, on near-pole configurations whose
/// radius is at least 2% from `r_pole` (beyond `R`'s round-off of the radius); and invariant under every power-of-two
/// scale `2^k`, `k` over `scales`, which commutes exactly with every operation unless an intermediate underflows or
/// overflows (R-547: none does). Returns the counts inside and outside.
fn check_disc<R: Prec>(test: Disc<R>, scales: std::ops::RangeInclusive<i32>) -> (usize, usize) {
    let rp: R = r_pole();
    let rp64 = wide(rp);
    let mut d = Draw(0xd15c);
    let (mut inside, mut outside) = (0, 0);
    for k in 0..4000 {
        let s = [0.001, 0.003, 0.006, 0.012][k % 4];
        let (r, m) = near_pole(&mut d, s);
        let rad = pole_radius(r, m);
        if (rad / rp64 - 1.0).abs() < 0.02 {
            continue;
        }
        let rr = r.map(|v| v.map(real::<R>));
        let mm = m.map(real::<R>);
        let got = test(rr, mm, rp);
        assert_eq!(
            got,
            rad < rp64,
            "the disc test at radius {rad} (r_pole {rp64}) (oracle)"
        );
        if got {
            inside += 1;
        } else {
            outside += 1;
        }
        for e in scales.clone().step_by(7) {
            let f: R = real(2f64.powi(e));
            let scaled = rr.map(|v| v.map(|x| x * f));
            assert_eq!(
                test(scaled, mm, rp),
                got,
                "the disc test changed at scale 2^{e} (scale)"
            );
        }
    }
    assert!(
        inside > 100 && outside > 100,
        "the sweep has {inside} inside and {outside} outside"
    );
    (inside, outside)
}

#[test]
fn qa_m311_theta_hold_disc_test() {
    // Coordinates down to 2^-100 and up to 2^100 at f64, 2^-60 to 2^60 at f32: a naive form of I² (degree 8 in the
    // coordinates) underflows or overflows long before these.
    check_disc::<f64>(config_in_disc, -100..=100);
    check_disc::<f32>(config_in_disc, -60..=60);
}

/// The disc test in plain floats: `(n_u² + n_v²)·I² < r_pole²·I²` with the unnormalised `u, v, I` (degree 4 in the
/// coordinates, squared again): it underflows at small scales.
#[cfg(feature = "controls")]
fn naive_disc(r: [[f32; 2]; 3], m: [f32; 3], rp: f32) -> bool {
    let m01 = m[0] + m[1];
    let mu1 = m[0] * m[1] / m01;
    let mu2 = m01 * m[2] / (m01 + m[2]);
    let rho = [r[1][0] - r[0][0], r[1][1] - r[0][1]];
    let c = [
        (m[0] * r[0][0] + m[1] * r[1][0]) / m01,
        (m[0] * r[0][1] + m[1] * r[1][1]) / m01,
    ];
    let lam = [r[2][0] - c[0], r[2][1] - c[1]];
    let rr = mu1 * (rho[0] * rho[0] + rho[1] * rho[1]);
    let ll = mu2 * (lam[0] * lam[0] + lam[1] * lam[1]);
    let (a, b) = (mu1.sqrt(), mu2.sqrt());
    let u = rr - ll;
    let v = 2.0 * a * b * (rho[0] * lam[0] + rho[1] * lam[1]);
    let i = rr + ll;
    u * u + v * v < rp * rp * i * i
}

negative_control!(
    qa_m311_theta_hold_disc_test,
    "a plain-float disc test changes its answer at small scales",
    expected = "(scale)",
    {
        check_disc::<f32>(naive_disc, -60..=60);
    }
);

// ── the passage fixtures (REQ-INT-088 on a march, REQ-INT-089) ─────────────────────────────────────────────────

const PASS_DT: f64 = 0.002;
const PASS_STEPS: u32 = 200;

/// Equal masses on the equilateral of circumradius 0.8, body 0 moved off it by `(ξ, δ₀)` and bodies 1 and 2 by half
/// that back (the centre of mass fixed), the momenta deforming the triangle along body 0's radius at speed 0.1
/// (`Σp = 0`). `ξ = 0.002` keeps the path a few thousandths of the pole away; `δ₀ = −0.02` starts it outside the disc
/// so that it passes through the disc near step 100, and `δ₀ = 0` starts it inside.
fn passage_ic<R: Prec>(inside: bool) -> Ic<R> {
    let (xi, sigma) = (0.002, 0.1);
    let d0 = if inside { 0.0 } else { -sigma * 0.2 };
    let t = equilateral();
    let r = [
        [t[0][0] + xi, t[0][1] + d0],
        [t[1][0] - xi / 2.0, t[1][1] - d0 / 2.0],
        [t[2][0] - xi / 2.0, t[2][1] - d0 / 2.0],
    ];
    let m = 1.0 / 3.0;
    let p = [
        [0.0, m * sigma],
        [0.0, -m * sigma / 2.0],
        [0.0, -m * sigma / 2.0],
    ];
    Ic {
        m: MASSES.map(real),
        r: r.map(|v| v.map(real)),
        p: p.map(|v| v.map(real)),
    }
}

/// The march's disc side of each stored state.
fn sides<R: Prec>(ic: &Ic<R>, states: &[SimStateBaseOf<R>]) -> Vec<bool> {
    states
        .iter()
        .map(|s| config_in_disc(s.r, ic.m, r_pole()))
        .collect()
}

/// The first entry step (outside → inside) and the exit step after it, or `(0, exit)` for an IC inside.
fn passage_steps(side: &[bool]) -> (usize, usize) {
    let entry = if side[0] {
        0
    } else {
        (1..side.len())
            .find(|&k| side[k] && !side[k - 1])
            .expect("the march never enters the disc")
    };
    let exit = (entry + 1..side.len())
        .find(|&k| !side[k])
        .expect("the march never leaves the disc");
    (entry, exit)
}

/// The hold on a real march at `R` through the disc, and from an IC inside it: outside the disc `_reserved` is 0xFFFF
/// at every step; the entry step's code is the previous step's longitude's; inside, `θ̃` and `_reserved` hold; the exit
/// adds `wrap(exit − decode(code))` and writes 0xFFFF; descriptor bits 10–14 are `I_max`'s level, which never
/// decreases from one step to the next, and bit 15 is the departed bit, set exactly from the first step whose chord to
/// `n̂(0)` exceeds `δ_dep` (the hold writes none of them).
fn check_march_hold<R: Prec>(states: &[SimStateBaseOf<R>], ic: &Ic<R>) -> (usize, usize) {
    let side = sides(ic, states);
    let (entry, exit) = passage_steps(&side);
    let n0 = shape(ic.r, ic.m);
    let dd: R = delta_dep();
    let mut departed = false;
    let mut level = 0;
    for (k, s) in states.iter().enumerate() {
        assert!(
            (s.packed_a & 0x7c00) >> 10 >= level,
            "step {k}: descriptor bits 10–14 {:#x} below the last step's I_max level {level}",
            s.packed_a
        );
        level = (s.packed_a & 0x7c00) >> 10;
        let n = shape(s.r, ic.m);
        let c: f64 = (0..3)
            .map(|i| (wide(n[i]) - wide(n0[i])).powi(2))
            .sum::<f64>()
            .sqrt();
        departed |= k > 0 && c > wide(dd);
        assert_eq!(
            sd_departed(s.packed_a),
            departed,
            "step {k}: bit 15 is not the departed bit"
        );
        if !side[k] {
            assert_eq!(
                s._reserved, 0xFFFF,
                "step {k}: _reserved outside the disc (outside)"
            );
        }
    }
    if entry > 0 {
        let prev = shape(states[entry - 1].r, ic.m);
        assert_eq!(
            states[entry]._reserved as u32,
            encode(longitude(prev)),
            "the entry step's code is not the last longitude outside's"
        );
        assert_eq!(
            bits(states[entry].theta),
            bits(states[entry - 1].theta),
            "θ̃ moved on entry"
        );
    } else {
        assert_eq!(states[0]._reserved, 0xFFFF, "an IC inside starts at 0xFFFF");
    }
    for k in entry + 1..exit {
        assert_eq!(
            (bits(states[k].theta), states[k]._reserved),
            (bits(states[entry].theta), states[entry]._reserved),
            "step {k}: the hold moved inside the disc"
        );
    }
    let code = states[exit - 1]._reserved;
    let added = wide(states[exit].theta) - wide(states[exit - 1].theta);
    let want = if code == 0xFFFF {
        0.0
    } else {
        wrap64(wide(longitude(shape(states[exit].r, ic.m))) - wide(decode::<R>(code as u32)))
    };
    assert!(
        (added - want).abs() <= 8.0 * ulp_tau::<R>(),
        "the exit added {added}, not wrap(exit − decode(code)) = {want}"
    );
    assert_eq!(states[exit]._reserved, 0xFFFF, "_reserved after the exit");
    (entry, exit)
}

fn check_march_holds<R: Prec>() {
    for inside in [false, true] {
        let ic = passage_ic::<R>(inside);
        let states = path(&ic, PASS_DT, PASS_STEPS);
        let (entry, exit) = check_march_hold(&states, &ic);
        assert!(exit > entry + 2, "the passage is {entry} → {exit}");
        // R-547: whether a state is inside agrees with the radius written here, away from the edge.
        for (k, s) in states.iter().enumerate() {
            let r = s.r.map(|v| v.map(wide));
            let rad = pole_radius(r, MASSES);
            if (rad / 0.01 - 1.0).abs() > 0.02 {
                assert_eq!(
                    config_in_disc(s.r, ic.m, r_pole()),
                    rad < 0.01,
                    "step {k}: the disc test"
                );
            }
        }
    }
}

#[test]
fn qa_m311_theta_hold_march() {
    check_march_holds::<f32>();
    check_march_holds::<f64>();
}

negative_control!(
    qa_m311_theta_hold_march,
    "a march whose stored _reserved kept the entry's code after the exit fails the outside check",
    expected = "(outside)",
    {
        let ic = passage_ic::<f64>(false);
        let mut states = path(&ic, PASS_DT, PASS_STEPS);
        let side = sides(&ic, &states);
        let (entry, exit) = passage_steps(&side);
        let code = states[entry]._reserved;
        for s in states.iter_mut().skip(exit) {
            s._reserved = code;
        }
        check_march_hold(&states, &ic);
    }
);

// ── REQ-INT-089: the split ───────────────────────────────────────────────────────────────────────────────────────

/// A march under test: `ic`'s march to `horizon` as calls ending at `splits` then at the horizon; the stored state
/// at the end of each call.
type Split<R> = fn(&Ic<R>, u32, &[u32]) -> Vec<SimStateBaseOf<R>>;

fn kernel_split<R: Prec>(ic: &Ic<R>, horizon: u32, splits: &[u32]) -> Vec<SimStateBaseOf<R>> {
    if splits.is_empty() {
        return vec![integrate::<Kdk, R>(ic, &uniforms(PASS_DT, horizon))];
    }
    let mut s = integrate::<Kdk, R>(ic, &uniforms(PASS_DT, 0));
    let mut at = 0;
    let mut out = Vec::new();
    for &end in splits.iter().chain(std::iter::once(&horizon)) {
        s = resume(ic, PASS_DT, horizon, &s, end - at);
        assert_eq!(tm_t_end_step(s.times), end, "a call did not end at {end}");
        out.push(s);
        at = end;
    }
    out
}

/// `k − 1` split points, one at `focus`, the others spread over the march.
fn split_points(k: u32, focus: u32, horizon: u32) -> Vec<u32> {
    let mut out = vec![focus];
    let mut j = 1;
    while out.len() < (k - 1) as usize {
        let c = j * horizon / k + 1;
        if c != focus && c < horizon && !out.contains(&c) {
            out.push(c);
        }
        j += 1;
        if j > 4 * k {
            let c = (focus + j) % (horizon - 1) + 1;
            if !out.contains(&c) {
                out.push(c);
            }
        }
    }
    out.sort_unstable();
    out
}

/// `run` split at every `k ∈ {2, 3, 7}` with a split on the entry step, strictly inside the disc and on the exit
/// step, against `run`'s own one call to the same step (no split): `θ̃`'s bits, `_reserved` and the closure registers
/// equal at every split boundary and at the end. Both fixtures; returns the passage steps.
fn check_split<R: Prec>(run: Split<R>) -> Vec<(usize, usize)> {
    let mut seen = Vec::new();
    for inside in [false, true] {
        let ic = passage_ic::<R>(inside);
        let one = path(&ic, PASS_DT, PASS_STEPS);
        let whole = kernel_split(&ic, PASS_STEPS, &[])[0];
        let end = one[PASS_STEPS as usize];
        assert_eq!(
            (bits(whole.theta), whole._reserved, whole.r, whole.p),
            (bits(end.theta), end._reserved, end.r, end.p),
            "one call and the one-step path differ at the horizon"
        );
        let (entry, exit) = passage_steps(&sides(&ic, &one));
        seen.push((entry, exit));
        let foci = if inside {
            vec![
                (exit / 2).max(1) as u32,
                (exit as u32).saturating_sub(1).max(1),
            ]
        } else {
            vec![entry as u32, ((entry + exit) / 2) as u32, exit as u32]
        };
        for focus in foci {
            for k in [2u32, 3, 7] {
                let splits = split_points(k, focus, PASS_STEPS);
                let got = run(&ic, PASS_STEPS, &splits);
                for (s, &end) in got
                    .iter()
                    .zip(splits.iter().chain(std::iter::once(&PASS_STEPS)))
                {
                    let o = run(&ic, end, &[])[0];
                    assert_eq!(
                        (bits(s.theta), s._reserved),
                        (bits(o.theta), o._reserved),
                        "k = {k}, splits {splits:?}: θ̃ or _reserved differ at step {end} (split)"
                    );
                    assert_eq!(
                        (bits(s.closure_min), s.closure_step, sd_departed(s.packed_a)),
                        (bits(o.closure_min), o.closure_step, sd_departed(o.packed_a)),
                        "k = {k}, splits {splits:?}: the closure differs at step {end}"
                    );
                }
            }
        }
    }
    seen
}

#[test]
fn qa_m311_theta_resume_split() {
    let a = check_split::<f32>(kernel_split);
    let b = check_split::<f64>(kernel_split);
    println!("passages (entry, exit): f32 {a:?}, f64 {b:?}");
}

/// The control's march: the kernel's states, but `θ̃` replayed from the stored shape path with the reference held
/// unquantised within a call (only a resume quantises it), the R-397 fault.
#[cfg(feature = "controls")]
fn unquantised_split(ic: &Ic<f64>, horizon: u32, splits: &[u32]) -> Vec<SimStateBaseOf<f64>> {
    let states = path(ic, PASS_DT, horizon);
    let side = sides(ic, &states);
    let ends: Vec<u32> = splits
        .iter()
        .copied()
        .chain(std::iter::once(horizon))
        .collect();
    let mut theta = 0.0f64;
    let mut reference: Option<f64> = None;
    let mut out = Vec::new();
    for k in 1..=horizon as usize {
        let n = shape(states[k].r, ic.m);
        let prev = shape(states[k - 1].r, ic.m);
        match (side[k - 1], side[k]) {
            (false, false) => theta += kernel::shape::wrap(longitude(n) - longitude(prev)),
            (false, true) => reference = Some(longitude(prev)),
            (true, false) => {
                if let Some(r) = reference.take() {
                    theta += kernel::shape::wrap(longitude(n) - r);
                }
            }
            (true, true) => {}
        }
        if ends.contains(&(k as u32)) {
            // A resume reloads the reference from its code.
            reference = reference.map(|r| decode::<f64>(encode(r)));
            out.push(SimStateBaseOf { theta, ..states[k] });
        }
    }
    out
}

negative_control!(
    qa_m311_theta_resume_split,
    "a march holding its reference unquantised within a call depends on the split",
    expected = "(split)",
    {
        check_split::<f64>(unquantised_split);
    }
);

// ── REQ-PAY-043, REQ-PAY-056: the closure registers ──────────────────────────────────────────────────────────────

/// The figure-eight's published IC and period (Chenciner & Montgomery 2000, Simó's digits; dd_integrator §5 test 2),
/// in `G = 1`, `mᵢ = 1`: `x₁ = −x₂`, `x₃ = 0`, `v₁ = v₂ = −v₃/2`.
const EIGHT_X1: [f64; 2] = [0.970_004_36, -0.243_087_53];
const EIGHT_V3: [f64; 2] = [-0.932_407_37, -0.864_731_46];
const EIGHT_T0: f64 = 6.325_913_98;

/// The figure-eight at masses ⅓ (the sim's simplex): the same positions, so with `G = 1` the accelerations are a
/// third, the velocities `1/√3` and the times `√3` of the published ones: period `√3·T₀`. Rotated rigidly at `omega`
/// (`p += m ω ẑ × r`), which adds `L_z = ω·Σmr²`.
fn eight<R: Prec>(omega: f64) -> (Ic<R>, f64) {
    let s = 3f64.sqrt();
    let m = 1.0 / 3.0;
    let r = [EIGHT_X1, [-EIGHT_X1[0], -EIGHT_X1[1]], [0.0, 0.0]];
    let v = [
        [-EIGHT_V3[0] / 2.0, -EIGHT_V3[1] / 2.0],
        [-EIGHT_V3[0] / 2.0, -EIGHT_V3[1] / 2.0],
        EIGHT_V3,
    ];
    let p = [0, 1, 2].map(|i| {
        [
            m * v[i][0] / s - m * omega * r[i][1],
            m * v[i][1] / s + m * omega * r[i][0],
        ]
    });
    let ic = Ic {
        m: MASSES.map(real),
        r: r.map(|v| v.map(real)),
        p: p.map(|v| v.map(real)),
    };
    (ic, s * EIGHT_T0)
}

const EIGHT_DT: f64 = 0.01;

/// The registers' offline values over a stored path: the departure step (the first sample whose chord to `n̂(0)`
/// exceeds `δ_dep`), and for each step the least chord from departure to it and the first step it is taken at.
struct Offline {
    departure: Option<usize>,
    /// `(min, argmin)` after each step; `None` before departure.
    registers: Vec<Option<(f64, usize)>>,
    /// The chords, at f64 from the stored shapes.
    chords: Vec<f64>,
}

fn offline<R: Prec>(ic: &Ic<R>, states: &[SimStateBaseOf<R>]) -> Offline {
    let n0 = shape(ic.r, ic.m).map(wide);
    let chords: Vec<f64> = states
        .iter()
        .map(|s| {
            let n = shape(s.r, ic.m).map(wide);
            (0..3).map(|i| (n[i] - n0[i]).powi(2)).sum::<f64>().sqrt()
        })
        .collect();
    let dd = wide(delta_dep::<R>());
    let departure = (1..chords.len()).find(|&k| chords[k] > dd);
    let mut registers = Vec::new();
    let mut best: Option<(f64, usize)> = None;
    for (k, &c) in chords.iter().enumerate() {
        if departure.is_some_and(|d| k >= d) && best.is_none_or(|(b, _)| c < b) {
            best = Some((c, k));
        }
        registers.push(best);
    }
    Offline {
        departure,
        registers,
        chords,
    }
}

/// The stored registers after every step against the offline ones: before departure the departed bit clear and no
/// pre-departure chord stored as the minimum (it is ignored); from departure the bit set, `closure_step` the offline
/// argmin (or the runner-up where the two are within `R`'s round-off of the chord) and `closure_min` the offline
/// minimum to round-off. `final_` is the one-call march's state at the horizon, held to the same.
fn check_registers<R: Prec>(
    ic: &Ic<R>,
    states: &[SimStateBaseOf<R>],
    final_: &SimStateBaseOf<R>,
) -> Offline {
    let off = offline(ic, states);
    let dep = off.departure.expect("the figure-eight never departs");
    let round = 64.0 * eps::<R>();
    let dd = wide(delta_dep::<R>());
    let last = states.len() - 1;
    for (k, s) in states
        .iter()
        .enumerate()
        .chain(std::iter::once((last, final_)))
    {
        let min = wide(s.closure_min);
        if k < dep {
            assert!(
                !sd_departed(s.packed_a),
                "step {k}: departed before the chord exceeds δ_dep"
            );
            assert!(
                min > dd,
                "step {k}: a pre-departure chord {min} is stored as the minimum (ignored)"
            );
            continue;
        }
        assert!(
            sd_departed(s.packed_a),
            "step {k}: not departed after step {dep}"
        );
        let (want, at) = off.registers[k].expect("registers after departure");
        let step = s.closure_step as usize;
        let tie = step >= dep && step <= k && (off.chords[step] - want).abs() <= round;
        assert!(
            step == at || tie,
            "step {k}: closure_step {step}, not the offline argmin {at} (argmin)"
        );
        assert!(
            (min - want).abs() <= round,
            "step {k}: closure_min {min}, not the offline minimum {want}"
        );
    }
    off
}

/// The figure-eight at `R` from `phase` steps along its orbit, marched to `periods` of `T`: the registers against the
/// offline ones, and the return where the orbit's symmetry puts it (`expect` periods), to within two steps (one for
/// the sampling, one for the integration's phase error), its chord below the greatest step's chord near it (the
/// sampling's resolution) and below `δ_dep`. Returns `(closure_step, closure_min)`.
fn check_eight<R: Prec>(phase: u32, periods: f64, expect: f64) -> (u16, f64) {
    let (ic0, t) = eight::<R>(0.0);
    let ic = if phase == 0 {
        ic0
    } else {
        let s = integrate::<Kdk, R>(&ic0, &uniforms(EIGHT_DT, phase));
        Ic {
            r: s.r,
            p: s.p,
            ..ic0
        }
    };
    let horizon = (periods * t / EIGHT_DT).ceil() as u32;
    let states = path(&ic, EIGHT_DT, horizon);
    let final_ = integrate::<Kdk, R>(&ic, &uniforms(EIGHT_DT, horizon));
    let off = check_registers(&ic, &states, &final_);
    let step = final_.closure_step;
    let min = wide(final_.closure_min);
    let at = expect * t / EIGHT_DT;
    assert!(
        (step as f64 - at).abs() <= 2.0,
        "the closure is at step {step}, not the period's {at:.1} (period)"
    );
    let near = (step as usize).saturating_sub(3)..=(step as usize + 3).min(horizon as usize);
    let stride = near
        .clone()
        .skip(1)
        .map(|k| {
            let a = shape(states[k].r, ic.m).map(wide);
            let b = shape(states[k - 1].r, ic.m).map(wide);
            max_diff(a, b) * 3f64.sqrt()
        })
        .fold(0.0f64, f64::max);
    assert!(
        min <= stride,
        "closure_min {min} exceeds the step's chord {stride} near the return"
    );
    assert!(
        min < wide(delta_dep::<R>()),
        "closure_min {min} is not below δ_dep"
    );
    let _ = off;
    (step, min)
}

#[test]
fn qa_m311_closure_registers_eight() {
    // The published IC starts at the Euler configuration with body 2 in the middle (collinear, `λ = 0`, `n = (1, 0,
    // 0)`); half a period on, the orbit is its mirror image with the same labels, which has the same `u, v` and the
    // opposite `w` = 0: so the shape returns at `T/2`. From a generic phase (a twelfth of a period on, `w ≠ 0`) the
    // mirror is a different shape, and the return is at `T`.
    for (phase_fraction, periods, expect) in [(0.0, 0.75, 0.5), (1.0 / 12.0, 1.2, 1.0)] {
        let phase = (phase_fraction * 3f64.sqrt() * EIGHT_T0 / EIGHT_DT).round() as u32;
        let a = check_eight::<f32>(phase, periods, expect);
        let b = check_eight::<f64>(phase, periods, expect);
        println!("figure-eight from step {phase}: f32 {a:?}, f64 {b:?}");
    }
}

negative_control!(
    qa_m311_closure_registers_eight,
    "the generic phase's return expected at T/2 must fail the period check",
    expected = "(period)",
    {
        let phase = (3f64.sqrt() * EIGHT_T0 / EIGHT_DT / 12.0).round() as u32;
        check_eight::<f64>(phase, 1.2, 0.5);
    }
);

/// A closure sampler under test: the registers after a sample at `n` against `n_0` at a step with threshold `δ`.
type Sampler = fn(Closure<f64>, [f64; 3], [f64; 3], u32, f64) -> Closure<f64>;

fn kernel_sample(c: Closure<f64>, n: [f64; 3], n0: [f64; 3], step: u32, dd: f64) -> Closure<f64> {
    c.sample(n, n0, step, dd)
}

/// Synthetic paths (payload § "Why closure is here"; R-37): the chords, in order, `1e-9` and `1e-6` (the closest
/// approach, before departure), `0.01`, then `0.5` (departure), `0.2`, `3e-3` at step 6, `0.4`, `5e-3`: the registers
/// end at `3e-3`, step 6, departed; and departure stays latched when the chord falls back below `δ_dep`. A chord of
/// exactly `δ_dep` does not depart ("first exceeded"), one just above does.
fn check_synthetic(sample: Sampler) {
    let dd = delta_dep::<f64>();
    let n0 = [1.0, 0.0, 0.0];
    let at = |c: f64| [1.0, c, 0.0];
    let mut s = Closure::<f64>::fresh();
    assert!(!s.departed, "a fresh closure is departed");
    for (k, c) in [1e-9, 1e-6, 0.01, 0.5, 0.2, 3e-3, 0.4, 5e-3]
        .into_iter()
        .enumerate()
    {
        s = sample(s, at(c), n0, k as u32 + 1, dd);
        if k < 3 {
            assert!(!s.departed, "departed at chord {c} below δ_dep {dd}");
            assert!(
                s.min > dd,
                "the pre-departure chord {} is held as the minimum (ignored)",
                s.min
            );
        } else {
            assert!(s.departed, "not departed (latched) after chord {c}");
        }
    }
    assert_eq!(
        (s.min, s.step),
        (3e-3, 6),
        "the registers after departure (registers)"
    );
    let e = sample(Closure::fresh(), at(dd), n0, 1, dd);
    assert!(!e.departed, "a chord of exactly δ_dep departed");
    let up = f64::from_bits(dd.to_bits() + 1);
    let e = sample(e, at(up), n0, 2, dd);
    assert!(e.departed, "a chord just above δ_dep did not depart");
}

#[test]
fn qa_m311_closure_departure_synthetic() {
    check_synthetic(kernel_sample);
}

negative_control!(
    qa_m311_closure_departure_synthetic,
    "an ungated closure takes the pre-departure approach as its minimum",
    expected = "(ignored)",
    check_synthetic(|c, n, n0, step, dd| {
        let s = c.sample(n, n0, step, dd);
        let d = (0..3).map(|i| (n[i] - n0[i]).powi(2)).sum::<f64>().sqrt();
        if d < c.min {
            Closure { min: d, step, ..s }
        } else {
            s
        }
    })
);

#[test]
fn qa_m311_departed_is_bit_15() {
    // R-545: the departed bit is sample_descriptor bit 15, alone.
    assert_eq!(set_departed(0, true), 0x8000);
    assert_eq!(set_departed(u32::MAX, false), !0x8000u32);
    assert!(sd_departed(0x8000) && !sd_departed(0x7fff) && !sd_departed(!0x8000u32));
}

negative_control!(
    qa_m311_departed_is_bit_15,
    "bit 14 is not the departed bit",
    expected = "assertion",
    assert_eq!(set_departed(0, true), 0x4000, "assertion")
);

// ── REQ-INT-082: θ̃ on a real orbit ───────────────────────────────────────────────────────────────────────────────

/// The rotation added to the figure-eight: `L_z = ω·Σmr² = 0.03 · ⅔ = 0.02`.
const OMEGA: f64 = 0.03;

/// The fixture's `|L_z|` exceeds this at every sampled step, so "`L_z` opposite" is not vacuous.
const LZ_BOUND: f64 = 0.01;

/// The run's horizon in periods: the shape's longitude turns about twice a period, so the hand count reaches two
/// full turns.
const ORBIT_PERIODS: f64 = 1.3;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Mirror {
    Full,
    /// The positions-only mirror, the control's.
    #[cfg(feature = "controls")]
    PositionsOnly,
}

fn mirrored<R: Prec>(ic: Ic<R>, how: Mirror) -> Ic<R> {
    let flip = |v: [R; 2]| [v[0], -v[1]];
    match how {
        Mirror::Full => Ic {
            r: ic.r.map(flip),
            p: ic.p.map(flip),
            ..ic
        },
        #[cfg(feature = "controls")]
        Mirror::PositionsOnly => Ic {
            r: ic.r.map(flip),
            ..ic
        },
    }
}

/// `L_z = Σ (xᵢp_yᵢ − yᵢp_xᵢ)`, in a fixed order.
fn lz<R: Prec>(s: &SimStateBaseOf<R>) -> R {
    let t = |i: usize| s.r[i][0] * s.p[i][1] - s.r[i][1] * s.p[i][0];
    t(0) + t(1) + t(2)
}

/// `θ̃` along a run: the stored one, or a control's replay.
type Thetas<R> = fn(&Ic<R>, &[SimStateBaseOf<R>]) -> Vec<R>;

fn stored_theta<R: Prec>(_: &Ic<R>, states: &[SimStateBaseOf<R>]) -> Vec<R> {
    states.iter().map(|s| s.theta).collect()
}

/// `retrograde` of a sampled step: the read side's from `θ̃`, or a control's.
type Retro<R> = fn(R, R) -> bool;

fn read_retro<R: Prec>(theta: R, _lz: R) -> bool {
    retrograde(wide(theta) as f32)
}

/// The hand count along a path, with no `atan2`: for each binary-collision half-meridian `b̂ₖ` (§3.7's table), the
/// signed crossings, `+1` where `n·(ẑ × b̂ₖ)` goes from negative to non-negative with `n·b̂ₖ > 0` (toward increasing
/// longitude), `−1` the other way; summed over the three and cumulated per step.
fn hand_count(path: &[[f64; 3]]) -> Vec<i64> {
    let b = collisions();
    let mut total = 0i64;
    let mut out = vec![0];
    for k in 1..path.len() {
        for bk in b {
            let side = |n: [f64; 3]| -bk[1] * n[0] + bk[0] * n[1];
            let toward = |n: [f64; 3]| bk[0] * n[0] + bk[1] * n[1] > 0.0;
            let (a, c) = (side(path[k - 1]), side(path[k]));
            if toward(path[k]) && toward(path[k - 1]) {
                if a < 0.0 && c >= 0.0 {
                    total += 1;
                } else if a >= 0.0 && c < 0.0 {
                    total -= 1;
                }
            }
        }
        out.push(total);
    }
    out
}

/// R-580's test 8 at `R`. The run of the rotating figure-eight and its mirror `how`, sampled every step. Checked in
/// this order, so that a reversed sense fails the direction only:
/// 1. no 2π jumps: every step's change in `θ̃` is below π in magnitude;
/// 2. the mirror pair: `θ̃` and `retrograde` bitwise identical and `L_z` bitwise opposite at every step, `|L_z| >
///    LZ_BOUND`;
/// 3. `orbit_count` at the last step where the hand count `C` is not a multiple of 3 (and at least 3) is `⌊|C|/3⌋`:
///    between crossings of the three half-meridians, 120° apart, `|θ̃ − (2π/3)C| < 2π/3`, so `|θ̃|/2π` lies strictly
///    within `(⌊|C|/3⌋, ⌊|C|/3⌋ + 1)` there;
/// 4. the direction: `|θ̃ − (2π/3)C| < 2π/3` at every step, and `retrograde` is `C < 0` wherever `|C| ≥ 2`.
fn check_orbit<R: Prec>(thetas: Thetas<R>, retro: Retro<R>, how: Mirror) -> (i64, u32) {
    let (ic, t) = eight::<R>(OMEGA);
    let horizon = (ORBIT_PERIODS * t / EIGHT_DT).ceil() as u32;
    let a = path(&ic, EIGHT_DT, horizon);
    let mic = mirrored(ic, how);
    let b = path(&mic, EIGHT_DT, horizon);
    let ta = thetas(&ic, &a);
    let tb = thetas(&mic, &b);
    let shapes: Vec<[f64; 3]> = a.iter().map(|s| shape(s.r, ic.m).map(wide)).collect();
    let count = hand_count(&shapes);
    for k in 1..ta.len() {
        let d = wide(ta[k]) - wide(ta[k - 1]);
        assert!(d.abs() < PI, "step {k}: θ̃ jumped by {d}");
    }
    for k in 0..a.len() {
        assert_eq!(
            bits(ta[k]),
            bits(tb[k]),
            "step {k}: θ̃ differs across the mirror pair (mirror θ̃)"
        );
        assert_eq!(
            retro(ta[k], lz(&a[k])),
            retro(tb[k], lz(&b[k])),
            "step {k}: retrograde differs across the mirror pair (mirror retrograde)"
        );
        let (la, lb) = (lz(&a[k]), lz(&b[k]));
        assert_eq!(
            bits(la),
            bits(-lb),
            "step {k}: L_z {la:?} and {lb:?} are not opposite (mirror L_z)"
        );
        assert!(
            wide(la).abs() > LZ_BOUND,
            "step {k}: |L_z| = {la:?} is not above {LZ_BOUND}"
        );
    }
    let end = (0..ta.len())
        .rev()
        .find(|&k| count[k].abs() >= 3 && count[k] % 3 != 0)
        .expect("the hand count never passes a full turn");
    let q = (count[end].abs() / 3) as u32;
    let oc = orbit_count(wide(ta[end]) as f32);
    assert_eq!(
        oc, q,
        "step {end}: orbit_count {oc}, the hand count {} gives {q}",
        count[end]
    );
    let s = TAU / 3.0;
    for k in 0..ta.len() {
        let th = wide(ta[k]);
        assert!(
            (th - s * count[k] as f64).abs() < s,
            "step {k}: θ̃ {th} against the hand count {} (direction)",
            count[k]
        );
        if count[k].abs() >= 2 {
            assert_eq!(
                retro(ta[k], lz(&a[k])),
                count[k] < 0,
                "step {k}: retrograde against the hand count {} (direction)",
                count[k]
            );
        }
    }
    (count[end], oc)
}

#[test]
fn qa_m311_theta_unwrap_real_orbit() {
    let a = check_orbit::<f32>(stored_theta, read_retro, Mirror::Full);
    let b = check_orbit::<f64>(stored_theta, read_retro, Mirror::Full);
    println!("(hand count, orbit_count) at the check step: f32 {a:?}, f64 {b:?}");
}

/// `θ̃` replayed from the stored path through the kernel's hold with each point mapped by `f`.
#[cfg(feature = "controls")]
fn replay(ic: &Ic<f64>, states: &[SimStateBaseOf<f64>], f: fn([f64; 3]) -> [f64; 3]) -> Vec<f64> {
    let mut theta = 0.0;
    let mut code = NO_REFERENCE;
    let mut out = vec![theta];
    for k in 1..states.len() {
        let prev = f(shape(states[k - 1].r, ic.m));
        let n = f(shape(states[k].r, ic.m));
        let pi = config_in_disc(states[k - 1].r, ic.m, r_pole());
        let ni = config_in_disc(states[k].r, ic.m, r_pole());
        (theta, code) = theta_hold(theta, code, prev, pi, n, ni);
        out.push(theta);
    }
    out
}

negative_control!(
    qa_m311_theta_unwrap_real_orbit,
    "θ̃ with its sense reversed (v negated) passes orbit_count and the mirror, and fails the direction",
    expected = "(direction)",
    {
        check_orbit::<f64>(
            |ic, s| replay(ic, s, |n| [n[0], -n[1], n[2]]),
            read_retro,
            Mirror::Full,
        );
    }
);

// R-580's three further controls, each on its own test of the same check at f64 (one control per test, R-176).

#[test]
fn qa_m311_theta_unwrap_mirror_retrograde() {
    check_orbit::<f64>(stored_theta, read_retro, Mirror::Full);
}

negative_control!(
    qa_m311_theta_unwrap_mirror_retrograde,
    "retrograde computed from sign(L_z) differs across the mirror pair",
    expected = "(mirror retrograde)",
    {
        check_orbit::<f64>(stored_theta, |_, lz| lz < 0.0, Mirror::Full);
    }
);

#[test]
fn qa_m311_theta_unwrap_mirror_theta() {
    check_orbit::<f64>(stored_theta, read_retro, Mirror::Full);
}

negative_control!(
    qa_m311_theta_unwrap_mirror_theta,
    "θ̃ read from w (the longitude of (u, w)) is not identical across the mirror pair",
    expected = "(mirror θ̃)",
    {
        check_orbit::<f64>(
            |ic, s| replay(ic, s, |n| [n[0], n[2], n[1]]),
            read_retro,
            Mirror::Full,
        );
    }
);

#[test]
fn qa_m311_theta_unwrap_mirror_lz() {
    check_orbit::<f64>(stored_theta, read_retro, Mirror::Full);
}

negative_control!(
    qa_m311_theta_unwrap_mirror_lz,
    "a positions-only mirror of a state with nonzero y-momenta does not negate L_z",
    expected = "(mirror L_z)",
    {
        check_orbit::<f64>(stored_theta, read_retro, Mirror::PositionsOnly);
    }
);
