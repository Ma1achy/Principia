//! QA tests for TASK-M2-53, written from REQ-DEC-053 (the f32 exponential sinc reads no backend `sin` near 0, and at f64
//! the line's bits do not move), REQ-DEC-054 (the cut and bound) and R-696, not from the implementation. The kernel
//! decode stage's half, the same failure run through the built kernel's WGSL with the backend's `sin` emulated as 0, is
//! in `crates/validation/tests/qa_TASK-M2-25_gpu.rs` (`qa_m253_flat_sin_keeps_rho_angle_near_the_lambda_pole`).
//!
//! - `qa_exponential_sinc_f64_is_the_old_line`: at f64 `exponential` and `ShapeSphere::point` are the line before R-696,
//!   bit for bit, on fuzzed frames, at `r = 0`, with signed zeros, and at subnormal and large `r`. Control: the form with
//!   no gate on the precision moves a bit.
//! - `qa_exponential_sinc_f32_bound`: at f32, within `sinc_bound` of the f64 `sin r/r`, relative, on every f32 within
//!   about 20% of the cut, on fuzzed `r` through `(0, 32)` from the subnormals up, and on the neighbours of π, 2π and
//!   WGSL's bound, with the `r²` one ulp either side of `r·r`, since the map forms `r²` by another chain than `r`.
//!   Control: the same series with the cut at 1 leaves the bound.
//! - `qa_exponential_sinc_series_coefficients`: the coefficients are `−1/3!`, `1/5!`, `−1/7!`, each the nearest f32, and
//!   the cut's square is exact at f32. Control: a coefficient one digit off.
//! - `qa_exponential_sinc_reads_no_sin_below_the_cut`: below the cut no value of the backend's `sin`, NaN, an infinity or
//!   a wrong number, reaches `k`, on `r` from RQ-687's radius through WGSL's `2⁻¹¹` to the last f32 below the cut.
//!   Control: the line before R-696.
//! - `qa_exponential_sinc_on_a_conforming_backend`: with the worst `sin` WGSL's absolute `2⁻¹¹` allows, `k` is positive
//!   and within `2⁻¹¹/c` of `sin r/r` at every `r` up to 3, so `k` is never the 0 of RQ-687's card. Control: a cut below
//!   WGSL's bound.
//! - `qa_exponential_sinc_f32_point`: at f32 `ShapeSphere::point` on the exponential map is `cos(r)·n₀ + k·d` with `k`
//!   R-696's form, on `r` down to where `r²` underflows, and `n₀` exactly at `(½, ½)`. Control: the line before R-696.
//!
//! **Domain** (pitfalls §10): the sinc and the point it feeds, stateless, on the CPU; RQ-687's backend is emulated.

use kernel::chart::shape_sphere::{exponential, sinc_f32, sinc_of};
use kernel::chart::{ShapeMap, ShapeSphere};
use kernel::generated::constants::{SINC_CUT2_F32, SINC_SERIES_F32};
use validation::{negative_control, SINC_BOUND, SINC_CUT};

/// WGSL's absolute bound on `sin`, `2⁻¹¹` (R-696's "near 0"; the register's `sinc_cut` note).
const WGSL_SIN: f32 = 1.0 / 2048.0;

/// RQ-687's radius, the failing pixel's.
const CARD_R: f32 = 1.00005e-7;

/// A form of the f32 `sin r/r` from the backend's `sin r`: `(sin_r, r, r²)`.
type FromSin = fn(f32, f32, f32) -> f32;

/// A form of the f32 `sin r/r` from `(r, r²)`.
type Form = fn(f32, f32) -> f32;

/// The exponential map at f64, `(n₀, e₁, e₂, s, u, v)`.
type Map64 = fn([f64; 3], [f64; 3], [f64; 3], f64, f64, f64) -> [f64; 3];

/// SplitMix64: seeded, so the draws repeat from run to run.
struct Draws(u64);

impl Draws {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let z = (self.0 ^ (self.0 >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        let z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// In `[0, 1)`.
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Log-uniform in `[2^lo, 2^hi)`.
    fn log(&mut self, lo: f64, hi: f64) -> f64 {
        (lo + (hi - lo) * self.unit()).exp2()
    }
}

/// `sin r/r` at f64, the requirement's reference.
fn truth(r: f32) -> f64 {
    let r = f64::from(r);
    r.sin() / r
}

/// `1 − z/3! + z²/5! − z³/7!` at f32, written from R-696's form, with the literals of the factorials.
fn series(z: f32) -> f32 {
    let (c1, c2, c3) = (-1.0 / 6.0f32, 1.0 / 120.0f32, -1.0 / 5040.0f32);
    z.mul_add(z.mul_add(z.mul_add(c3, c2), c1), 1.0)
}

/// `form` is within `sinc_bound` of the f64 `sin r/r` at `r`, `z` its `r²`.
fn within_bound(form: Form, r: f32, z: f32) {
    let t = truth(r);
    let err = ((f64::from(form(r, z)) - t) / t).abs() / f64::from(f32::EPSILON);
    assert!(
        err <= SINC_BOUND.number(),
        "at r = {r:e}, r² = {z:e}, the f32 sinc is {err} eps from the f64 sin r/r {t:e}"
    );
}

/// The neighbours of `x`, `n` f32s each side.
fn around(x: f32, n: i32) -> impl Iterator<Item = f32> {
    (-n..=n).map(move |k| f32::from_bits(x.to_bits().wrapping_add_signed(k)))
}

/// The bound's radii: fuzzed through `(0, 32)` and from the subnormals up, and the neighbours of the places that matter.
fn radii() -> Vec<f32> {
    use std::f32::consts::PI;
    let mut draws = Draws(0x0123_4567_89AB_CDEF);
    let mut out: Vec<f32> = (0..1 << 18)
        .flat_map(|_| [(32.0 * draws.unit()) as f32, draws.log(-149.0, 5.0) as f32])
        .collect();
    for x in [PI, 2.0 * PI, WGSL_SIN, CARD_R, SINC_CUT.number() as f32] {
        out.extend(around(x, 64));
    }
    out.retain(|r| *r > 0.0);
    out
}

/// The bound on `form` at every radius, and on every f32 within about 20% of the cut.
fn check_bound(form: Form) {
    for r in radii() {
        let z = r * r;
        within_bound(form, r, z);
        if z > 0.0 {
            within_bound(form, r, z.next_up());
            within_bound(form, r, z.next_down());
        }
    }
    let cut = SINC_CUT.number() as f32;
    for bits in (0.8 * cut).to_bits()..(1.2 * cut).to_bits() {
        let r = f32::from_bits(bits);
        within_bound(form, r, r * r);
    }
}

/// R-696's series with its cut moved up to `r = 1`, past where four terms hold.
#[cfg(feature = "controls")]
fn wide_cut(r: f32, z: f32) -> f32 {
    if z < 1.0 {
        series(z)
    } else {
        r.sin() / r
    }
}

#[test]
fn qa_exponential_sinc_f32_bound() {
    check_bound(sinc_f32);
}

negative_control!(
    qa_exponential_sinc_f32_bound,
    "four terms with the cut at 1 must leave REQ-DEC-054's bound",
    expected = "eps from the f64 sin r/r",
    check_bound(wide_cut)
);

/// The coefficients are the nearest f32s of the factorials' reciprocals, and the cut's square is exact.
fn check_coefficients(got: [f32; 3]) {
    let want = [-1.0 / 6.0f64, 1.0 / 120.0, -1.0 / 5040.0].map(|c| c as f32);
    assert_eq!(
        got.map(f32::to_bits),
        want.map(f32::to_bits),
        "the series' coefficients {got:?} are not the nearest f32s {want:?}"
    );
    let cut = SINC_CUT.number();
    assert_eq!(f64::from(SINC_CUT2_F32), cut * cut, "the cut's square");
}

#[test]
fn qa_exponential_sinc_series_coefficients() {
    check_coefficients(SINC_SERIES_F32);
}

negative_control!(
    qa_exponential_sinc_series_coefficients,
    "a coefficient one digit off must be found",
    expected = "are not the nearest f32s",
    check_coefficients([SINC_SERIES_F32[0], 0.008_333, SINC_SERIES_F32[2]])
);

/// `form` at `r < cut`, on any value of the backend's `sin`, gives the same bits.
fn check_no_read(form: FromSin) {
    let cut = SINC_CUT.number() as f32;
    let mut draws = Draws(0xFEDC_BA98_7654_3210);
    let fuzzed = (0..1 << 16).map(|_| draws.log(-30.0, -2.0) as f32);
    let edges = around(WGSL_SIN, 8)
        .chain(around(CARD_R, 8))
        .chain([cut.next_down()]);
    for r in fuzzed.chain(edges).filter(|r| r * r < SINC_CUT2_F32) {
        let z = r * r;
        let base = form(0.0, r, z);
        assert!(
            base != 0.0,
            "at r = {r:e} the backend's sin of 0 gives k = 0"
        );
        for sin_r in [f32::NAN, f32::INFINITY, -f32::INFINITY, -1.0, 1.0, 1e30, r] {
            assert_eq!(
                form(sin_r, r, z).to_bits(),
                base.to_bits(),
                "at r = {r:e} the backend's sin, {sin_r:e}, reached k"
            );
        }
    }
}

/// The line before R-696.
#[cfg(feature = "controls")]
fn old_line(sin_r: f32, r: f32, _z: f32) -> f32 {
    sin_r / r
}

/// R-696's form with its cut below WGSL's bound on `sin`.
#[cfg(feature = "controls")]
fn low_cut(sin_r: f32, r: f32, z: f32) -> f32 {
    if r < WGSL_SIN / 2.0 {
        series(z)
    } else {
        sin_r / r
    }
}

#[test]
fn qa_exponential_sinc_reads_no_sin_below_the_cut() {
    check_no_read(sinc_of);
}

negative_control!(
    qa_exponential_sinc_reads_no_sin_below_the_cut,
    "the line before R-696 reads the backend's sin at RQ-687's radius",
    expected = "the backend's sin of 0 gives k = 0",
    check_no_read(old_line)
);

/// `form` on the worst `sin` a conforming backend may return, `sin r − 2⁻¹¹` kept at 0 or above, is positive and within
/// `2⁻¹¹/c` of `sin r/r`, on `r` from below RQ-687's radius to 3, short of π, where `sin r` itself reaches `2⁻¹¹`.
fn check_conforming(form: FromSin) {
    let c = SINC_CUT.number();
    let allowance = f64::from(WGSL_SIN) / c;
    let mut draws = Draws(0x0F0F_F0F0_3C3C_C3C3);
    for _ in 0..1 << 18 {
        let r = if draws.next() & 1 == 0 {
            draws.log(-25.0, 1.5) as f32
        } else {
            (3.0 * draws.unit()) as f32
        };
        if r == 0.0 {
            continue;
        }
        let worst = (r.sin() - WGSL_SIN).max(0.0);
        let k = form(worst, r, r * r);
        assert!(
            k > 0.0,
            "at r = {r:e} the worst conforming sin gives k = {k:e}"
        );
        let err = (f64::from(k) - truth(r)).abs();
        assert!(
            err <= allowance,
            "at r = {r:e} the worst conforming sin moves k by {err:e}, over 2⁻¹¹/c = {allowance:e}"
        );
    }
}

#[test]
fn qa_exponential_sinc_on_a_conforming_backend() {
    check_conforming(sinc_of);
}

negative_control!(
    qa_exponential_sinc_on_a_conforming_backend,
    "a cut below WGSL's bound must let the worst conforming sin set k to 0",
    expected = "the worst conforming sin",
    check_conforming(low_cut)
);

/// The line before R-696 at f64, with `d` and `r` as it formed them, the sinc and all.
fn line_f64(n0: [f64; 3], e1: [f64; 3], e2: [f64; 3], s: f64, u: f64, v: f64) -> [f64; 3] {
    let (a, b) = ((u + u - 1.0) * s, (v + v - 1.0) * s);
    let d = [0, 1, 2].map(|i| a * e1[i] + b * e2[i]);
    let r = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    // the pinned libm's `cos` and `sin`, the f64 path's since R-991 (REQ-CHART-212)
    let c = libm::cos(r);
    let k = if r == 0.0 { 1.0 } else { libm::sin(r) / r };
    [0, 1, 2].map(|i| c * n0[i] + k * d[i])
}

/// The same with R-696's series and cut in its `k` and no gate on the precision.
#[cfg(feature = "controls")]
fn ungated_f64(n0: [f64; 3], e1: [f64; 3], e2: [f64; 3], s: f64, u: f64, v: f64) -> [f64; 3] {
    let (a, b) = ((u + u - 1.0) * s, (v + v - 1.0) * s);
    let d = [0, 1, 2].map(|i| a * e1[i] + b * e2[i]);
    let z = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
    let r = z.sqrt();
    let c = libm::cos(r);
    let [c1, c2, c3] = SINC_SERIES_F32.map(f64::from);
    let k = if r == 0.0 {
        1.0
    } else if z < f64::from(SINC_CUT2_F32) {
        1.0 + z * (c1 + z * (c2 + z * c3))
    } else {
        libm::sin(r) / r
    };
    [0, 1, 2].map(|i| c * n0[i] + k * d[i])
}

/// The f64 map through the chart.
fn through_point(n0: [f64; 3], e1: [f64; 3], e2: [f64; 3], s: f64, u: f64, v: f64) -> [f64; 3] {
    sphere(n0, e1, e2, s).point(u, v)
}

/// The chart on the exponential map, the blocks the map does not read at their defaults.
fn sphere<P: From<f32> + Copy>(n0: [P; 3], e1: [P; 3], e2: [P; 3], scale: P) -> ShapeSphere<P> {
    let zero = P::from(0.0);
    ShapeSphere {
        map: ShapeMap::Exponential,
        n0,
        e1,
        e2,
        scale,
        inertia: P::from(1.0),
        phase: zero,
        z_mass: [zero; 2],
        z_mom: [zero; 4],
    }
}

/// `map` at f64 is the old line, bit for bit, on fuzzed turned frames and at the edges.
fn check_f64(map: Map64) {
    let same = |n0, e1, e2, s, u, v| {
        let (got, old) = (map(n0, e1, e2, s, u, v), line_f64(n0, e1, e2, s, u, v));
        assert_eq!(
            got.map(f64::to_bits),
            old.map(f64::to_bits),
            "at s = {s:e}, (u, v) = ({u}, {v}): the f64 map {got:?} is not the old line {old:?}, bit for bit"
        );
    };
    let mut draws = Draws(0x1357_9BDF_0246_8ACE);
    for i in 0..1u32 << 17 {
        let (ca, sa) = (draws.unit() * 7.0).sin_cos();
        let (cb, sb) = (draws.unit() * 7.0).sin_cos();
        let n0 = [ca * cb, sa * cb, sb];
        let e1 = [-sa, ca, 0.0];
        let e2 = [-ca * sb, -sa * sb, cb];
        let s = match i % 4 {
            0 => draws.log(-1074.0, 1.0),
            1 => draws.log(-3.0, 20.0),
            _ => draws.log(-30.0, 1.0),
        };
        same(n0, e1, e2, s, draws.unit(), draws.unit());
    }
    // r = 0, with every sign of the zeros, at the centre and at a zero extent.
    for z in [0.0, -0.0] {
        for sign in [1.0, -1.0] {
            let n0 = [sign, z, -z];
            let (e1, e2) = ([z, 1.0, 0.0], [0.0, z, sign]);
            same(n0, e1, e2, 1.0, 0.5, 0.5);
            same(n0, e1, e2, 0.0, 0.25, 0.75);
            same(n0, e1, e2, z, 0.75, 0.25);
        }
    }
}

#[test]
fn qa_exponential_sinc_f64_is_the_old_line() {
    check_f64(exponential);
    check_f64(through_point);
}

negative_control!(
    qa_exponential_sinc_f64_is_the_old_line,
    "the form with no gate on the precision must move an f64 bit",
    expected = "bit for bit",
    check_f64(ungated_f64)
);

/// R-696's `k`, written from the ruling: 1 at `r = 0`, the series below the cut on `z`, `r²` by its `fma` chain, and the
/// backend's `sin r/r` from it up.
fn form_k(r: f32, z: f32) -> f32 {
    if r == 0.0 {
        1.0
    } else if z < SINC_CUT2_F32 {
        series(z)
    } else {
        r.sin() / r
    }
}

/// The f32 point is `cos(r)·n₀ + k·d`, `k = k_of(r, r²)`, bit for bit, on fuzzed extents down to where `r²` underflows.
fn check_point(k_of: Form) {
    let mut draws = Draws(0x2468_ACE0_1357_9BDF);
    for _ in 0..1 << 17 {
        let (sa, ca) = (draws.unit() as f32 * 7.0).sin_cos();
        let n0 = [ca, sa, 0.0];
        let (e1, e2) = ([-sa, ca, 0.0], [0.0, 0.0, 1.0]);
        let s = draws.log(-110.0, 1.0) as f32;
        let (u, v) = (draws.unit() as f32, draws.unit() as f32);
        let (a, b) = ((u + u - 1.0) * s, (v + v - 1.0) * s);
        let d = [0, 1, 2].map(|i| a * e1[i] + b * e2[i]);
        let r = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        let z = d[2].mul_add(d[2], d[1].mul_add(d[1], d[0] * d[0]));
        let (c, k) = (r.cos(), k_of(r, z));
        let want = [0, 1, 2].map(|i| c * n0[i] + k * d[i]);
        let got = sphere(n0, e1, e2, s).point(u, v);
        assert_eq!(
            got.map(f32::to_bits),
            want.map(f32::to_bits),
            "at r = {r:e} the f32 point {got:?} is not cos(r)·n₀ + k·d {want:?}"
        );
    }
    let n0 = [0.6, 0.0, 0.8];
    let centre = sphere(n0, [0.0, 1.0, 0.0], [-0.8, 0.0, 0.6], 1.0).point(0.5, 0.5);
    assert_eq!(centre, n0, "r = 0 gives n₀ exactly");
}

#[test]
fn qa_exponential_sinc_f32_point() {
    check_point(form_k);
}

negative_control!(
    qa_exponential_sinc_f32_point,
    "the line before R-696 must differ from the f32 point in a bit",
    expected = "is not cos(r)·n₀ + k·d",
    check_point(|r, _| if r == 0.0 { 1.0 } else { r.sin() / r })
);
