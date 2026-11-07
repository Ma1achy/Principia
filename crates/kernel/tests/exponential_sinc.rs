//! The shape sphere's exponential sinc, `kernel::Real::sinc` (R-696, RQ-687's option (b); REQ-DEC-053, REQ-DEC-054): at
//! f32 it never reads the backend's `sin` near 0, and at f64 its bits do not move.
//! - `exponential_sinc_f64_is_sin_over_r`: at f64 it is `sin(r)/r`, bit for bit, on fuzzed `r`; an ungated form moves a
//!   bit, its control.
//! - `exponential_sinc_f64_map_bits`: `exponential` at f64 is the line's old form, bit for bit, on fuzzed frames and at
//!   `r = 0`.
//! - `exponential_sinc_f32_within_the_bound`: at f32, on the CPU, within `sinc_bound` of the f64 value on fuzzed `r` and
//!   across the cut, printing the worst on each side. `exponential_sinc_f32_exhaustive`, ignored, does so on every f32
//!   below 4 (applied per R-369: too slow for CI; `cargo test --release -p kernel --test exponential_sinc -- --ignored`).
//! - `exponential_sinc_backend_sin_of_zero`: with the backend's `sin` emulated as 0 near 0, RQ-687's measured case, the
//!   old form gives `k = 0` and the new one its value.
//! - `exponential_sinc_cut_boundary`: the cut itself takes `sin(r)/r`, the next `r²` below it the series, `r² = 0` gives
//!   1 exactly, and the generated cut is the register's squared.
//! - `exponential_sinc_f32_map_reads_the_form`: `exponential` at f32 is `cos(r)·n₀ + k·d` with `k` the form on its `‖d‖²`.
//! - `exponential_sinc_fragment_regenerated`: `crates/render/wgsl/decode.wgsl` holds the form's cut and coefficients;
//!   `cargo xtask build-kernel` is what checks the whole file against the source.

use std::thread;

use kernel::chart::shape_sphere::{exponential, sinc_f32, sinc_of};
use kernel::generated::constants::{SINC_CUT2_F32, SINC_SERIES_F32};
use kernel::Real;
use validation::{negative_control, SINC_BOUND, SINC_CUT};

/// The axes the map tests turn into their fuzzed frames.
const FRAME: [[f64; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

/// `2⁻¹¹`, the absolute bound WGSL puts on `sin`: a conforming backend may return 0 up to here.
const BACKEND_BOUND: f32 = 1.0 / 2048.0;

/// RQ-687's radius: the failing test's pixel, where the card's `sin(1.00005e-7)` is read to have returned 0.
const MEASURED_R: f32 = 1.00005e-7;

/// A small seeded generator: the draws repeat from run to run.
struct Draws(u64);

impl Draws {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
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

/// `sin r/r` at f64.
fn truth(r: f32) -> f64 {
    let r = f64::from(r);
    r.sin() / r
}

/// The fuzzed radii: uniform and log-uniform through `(0, 4)`, and the 4096 f32s on each side of the cut.
fn radii() -> Vec<f32> {
    let mut draws = Draws(0x9E37_79B9_7F4A_7C15);
    let mut out = Vec::new();
    for _ in 0..1 << 20 {
        out.push((4.0 * draws.unit()) as f32);
        out.push(draws.log(-40.0, 2.0) as f32);
    }
    let cut = (SINC_CUT.number() as f32).to_bits();
    out.extend(((cut - 4096)..(cut + 4096)).map(f32::from_bits));
    out.retain(|r| *r > 0.0);
    out
}

/// The worst errors, in f32 eps relative to the f64 value, below the cut and from it up.
#[derive(Clone, Copy, Debug, Default)]
struct Worst {
    below: f64,
    above: f64,
}

/// `form(r, r²)` is within `sinc_bound` of the f64 `sin r/r` at each of `rs`; the worst on each side of the cut.
fn check_bound(form: fn(f32, f32) -> f32, rs: impl Iterator<Item = f32>) -> Worst {
    let bound = SINC_BOUND.number();
    let eps = f64::from(f32::EPSILON);
    let mut worst = Worst::default();
    for r in rs {
        let square = r * r;
        let t = truth(r);
        let err = ((f64::from(form(r, square)) - t) / t).abs() / eps;
        assert!(
            err <= bound,
            "at r = {r:e} the f32 sinc is {err} eps from the f64 sin r/r {t:e}, over REQ-DEC-054's {bound}"
        );
        let side = if square < SINC_CUT2_F32 {
            &mut worst.below
        } else {
            &mut worst.above
        };
        *side = side.max(err);
    }
    worst
}

/// The f32 form, through `Real`'s gate.
fn gated(r: f32, square: f32) -> f32 {
    Real::sinc(r, square)
}

/// A one-term series with the form's cut, too short to stay within the bound.
#[cfg(feature = "controls")]
fn short_series(r: f32, square: f32) -> f32 {
    if square < SINC_CUT2_F32 {
        1.0 - square / 6.0
    } else {
        r.sin() / r
    }
}

#[test]
fn exponential_sinc_f32_within_the_bound() {
    let worst = check_bound(gated, radii().into_iter());
    println!("{worst:?}");
    // Both sides are sampled, so the check above read both forms.
    assert!(worst.below > 0.0 && worst.above > 0.0, "{worst:?}");
}

negative_control!(
    exponential_sinc_f32_within_the_bound,
    "a series one term long, at the same cut, must leave REQ-DEC-054's bound",
    expected = "eps from the f64 sin r/r",
    {
        check_bound(short_series, radii().into_iter());
    }
);

/// Every f32 in `(0, 4)`, on as many threads as the machine has.
fn check_every_f32(form: fn(f32, f32) -> f32) -> Worst {
    let end = 4.0f32.to_bits();
    let threads = thread::available_parallelism().map_or(1, |n| n.get() as u32);
    let chunk = end / threads + 1;
    let per = thread::scope(|s| {
        let handles: Vec<_> = (0..threads)
            .map(|k| {
                s.spawn(move || {
                    let lo = (k * chunk).max(1);
                    let hi = ((k + 1) * chunk).min(end);
                    check_bound(form, (lo..hi).map(f32::from_bits))
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("a sweep thread panicked"))
            .collect::<Vec<_>>()
    });
    per.into_iter().fold(Worst::default(), |a, b| Worst {
        below: a.below.max(b.below),
        above: a.above.max(b.above),
    })
}

#[test]
#[ignore = "every f32 below 4: run with --release -- --ignored (the module's note)"]
fn exponential_sinc_f32_exhaustive() {
    println!("{:?}", check_every_f32(gated));
}

negative_control!(
    exponential_sinc_f32_exhaustive,
    "the exhaustive sweep's check, on a slice of it, must refuse the short series",
    expected = "eps from the f64 sin r/r",
    {
        check_bound(short_series, (0x3E7F_0000..0x3E80_0000).map(f32::from_bits));
    }
);

/// A form of `sin r/r` at f64, `square` the `r²`, is `sin(r)/r`, bit for bit, on fuzzed `r`.
fn check_f64(form: fn(f64, f64) -> f64) {
    let mut draws = Draws(0xD1B5_4A32_D192_ED03);
    for i in 0..1u32 << 20 {
        let r = if i % 2 == 0 {
            4.0 * draws.unit()
        } else {
            draws.log(-60.0, 2.0)
        };
        if r == 0.0 {
            continue;
        }
        let (got, old) = (form(r, r * r), r.sin() / r);
        assert_eq!(
            got.to_bits(),
            old.to_bits(),
            "at r = {r:e} the f64 sinc {got:e} is not sin(r)/r {old:e}, bit for bit"
        );
    }
}

/// The series and cut with no gate on the precision, at f64: what R-696's gate keeps out.
#[cfg(feature = "controls")]
fn ungated(r: f64, square: f64) -> f64 {
    if square < f64::from(SINC_CUT2_F32) {
        let [a, b, c] = SINC_SERIES_F32.map(f64::from);
        1.0 + square * (a + square * (b + square * c))
    } else {
        r.sin() / r
    }
}

#[test]
fn exponential_sinc_f64_is_sin_over_r() {
    check_f64(Real::sinc);
}

negative_control!(
    exponential_sinc_f64_is_sin_over_r,
    "the form with no gate on the precision must move an f64 bit",
    expected = "bit for bit",
    check_f64(ungated)
);

/// A frame `(n₀, e₁, e₂)` turned from the axes by `a`, `b`, `c`, so that every component of `d` is in play.
fn frame(a: f64, b: f64, c: f64) -> [[f64; 3]; 3] {
    let turn = |v: [f64; 3]| {
        let (sa, ca) = a.sin_cos();
        let (sb, cb) = b.sin_cos();
        let (sc, cc) = c.sin_cos();
        let v = [ca * v[0] - sa * v[1], sa * v[0] + ca * v[1], v[2]];
        let v = [cb * v[0] + sb * v[2], v[1], cb * v[2] - sb * v[0]];
        [v[0], cc * v[1] - sc * v[2], sc * v[1] + cc * v[2]]
    };
    FRAME.map(turn)
}

/// `exponential`'s line at f64 before R-696, its offset and radius as the map forms them.
fn old_map(n0: [f64; 3], e1: [f64; 3], e2: [f64; 3], s: f64, u: f64, v: f64) -> [f64; 3] {
    let (a, b) = ((u + u - 1.0) * s, (v + v - 1.0) * s);
    let d = [0, 1, 2].map(|i| a * e1[i] + b * e2[i]);
    let r = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    let c = r.cos();
    let k = if r == 0.0 { 1.0 } else { r.sin() / r };
    [0, 1, 2].map(|i| c * n0[i] + k * d[i])
}

/// The exponential map at f64, `(n₀, e₁, e₂, s, u, v)`.
type Map64 = fn([f64; 3], [f64; 3], [f64; 3], f64, f64, f64) -> [f64; 3];

/// `map` at f64 is the line's old form, bit for bit, on fuzzed frames, at `r` of every size and at `r = 0`.
fn check_map_f64(map: Map64) {
    let mut draws = Draws(0xA076_1D64_78BD_642F);
    for i in 0..1u32 << 16 {
        let [n0, e1, e2] = frame(draws.unit() * 6.0, draws.unit() * 6.0, draws.unit() * 6.0);
        let s = draws.log(-30.0, 1.0);
        let (u, v) = if i % 16 == 0 {
            (0.5, 0.5)
        } else {
            (draws.unit(), draws.unit())
        };
        let (got, old) = (map(n0, e1, e2, s, u, v), old_map(n0, e1, e2, s, u, v));
        assert_eq!(
            got.map(f64::to_bits),
            old.map(f64::to_bits),
            "at (u, v) = ({u}, {v}), s = {s:e}, the f64 map {got:?} is not the old form {old:?}, bit for bit"
        );
    }
}

#[test]
fn exponential_sinc_f64_map_bits() {
    check_map_f64(exponential);
    let n0 = [0.6, 0.0, 0.8];
    let at_centre = exponential(n0, [0.0, 1.0, 0.0], [-0.8, 0.0, 0.6], 1.0, 0.5, 0.5);
    assert_eq!(at_centre, n0, "r = 0 gives n₀ exactly");
}

negative_control!(
    exponential_sinc_f64_map_bits,
    "the old map with the ungated form in its k must move an f64 bit",
    expected = "bit for bit",
    check_map_f64(|n0, e1, e2, s, u, v| {
        let (a, b) = ((u + u - 1.0) * s, (v + v - 1.0) * s);
        let d = [0, 1, 2].map(|i| a * e1[i] + b * e2[i]);
        let z = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
        let r = z.sqrt();
        let k = if r == 0.0 { 1.0 } else { ungated(r, z) };
        [0, 1, 2].map(|i| r.cos() * n0[i] + k * d[i])
    })
);

/// The backend `sin` of RQ-687's measured case: 0 up to the absolute bound, where a conforming `sin` may be.
fn flat_sin(r: f32) -> f32 {
    if r <= BACKEND_BOUND {
        0.0
    } else {
        r.sin()
    }
}

/// The line before R-696, on the emulated `sin`.
fn old_form(sin_r: f32, r: f32, _square: f32) -> f32 {
    if r == 0.0 {
        1.0
    } else {
        sin_r / r
    }
}

/// `form` on the emulated `sin` is `sin r/r` near 0, within the bound, at RQ-687's radius and through to the bound.
fn check_flat_sin(form: fn(f32, f32, f32) -> f32) {
    let eps = f64::from(f32::EPSILON);
    let mut draws = Draws(0x8BB8_4B93_962E_ACC9);
    let rs = (0..1 << 16).map(|_| draws.log(-24.0, -11.0) as f32);
    for r in [MEASURED_R, BACKEND_BOUND].into_iter().chain(rs) {
        let k = form(flat_sin(r), r, r * r);
        assert!(k != 0.0, "at r = {r:e} the backend's sin of 0 gives k = 0");
        let err = ((f64::from(k) - truth(r)) / truth(r)).abs() / eps;
        assert!(
            err <= SINC_BOUND.number(),
            "at r = {r:e}, k = {k:e}, {err} eps"
        );
    }
}

#[test]
fn exponential_sinc_backend_sin_of_zero() {
    check_flat_sin(sinc_of);
    assert_eq!(
        old_form(flat_sin(MEASURED_R), MEASURED_R, 0.0),
        0.0,
        "the emulated sin is the card's: the old form's k is 0 at RQ-687's radius"
    );
}

negative_control!(
    exponential_sinc_backend_sin_of_zero,
    "the line before R-696, on the backend's sin of 0, must give k = 0",
    expected = "the backend's sin of 0 gives k = 0",
    check_flat_sin(old_form)
);

/// `form`'s boundary: the cut's `r²` itself reads `sin_r`, the next `r²` below it does not, and `r² = 0` gives 1 exactly.
fn check_boundary(form: fn(f32, f32, f32) -> f32) {
    let (r, sin_r) = (SINC_CUT.number() as f32, 0.125);
    let at_cut = form(sin_r, r, SINC_CUT2_F32);
    assert_eq!(
        at_cut,
        sin_r / r,
        "the cut's own r² reads the backend's sin"
    );
    let below = f32::from_bits(SINC_CUT2_F32.to_bits() - 1);
    let k = form(sin_r, r, below);
    assert_eq!(
        k,
        sinc_of(f32::NAN, r, below),
        "below the cut no sin is read"
    );
    assert!(!k.is_nan(), "below the cut no sin is read: {k}");
    assert_eq!(form(f32::NAN, 0.0, 0.0), 1.0, "r² = 0 gives 1 exactly");
}

/// The cut taken at `≤`.
#[cfg(feature = "controls")]
fn cut_inclusive(sin_r: f32, r: f32, square: f32) -> f32 {
    if square <= SINC_CUT2_F32 {
        sinc_of(f32::NAN, r, 0.0)
    } else {
        sin_r / r
    }
}

#[test]
fn exponential_sinc_cut_boundary() {
    check_boundary(sinc_of);
    let cut = SINC_CUT.number();
    assert_eq!(
        f64::from(SINC_CUT2_F32),
        cut * cut,
        "the register's cut, squared"
    );
    assert!(
        SINC_CUT.number() > f64::from(BACKEND_BOUND),
        "above the cut the backend's sin cannot conform and be 0"
    );
}

negative_control!(
    exponential_sinc_cut_boundary,
    "the cut taken at ≤ must be found",
    expected = "the cut's own r² reads the backend's sin",
    check_boundary(cut_inclusive)
);

/// `form(d, r²)`, the map's `k` for offset `d`, is `exponential`'s at f32 on fuzzed frames: the point's bits are
/// `cos(r)·n₀ + k·d` with `d` and `r` formed as the map forms them.
fn check_map_f32(k_of: fn(r: f32, d: [f32; 3]) -> f32) {
    let mut draws = Draws(0xE703_7ED1_A0B4_28DB);
    for _ in 0..1 << 16 {
        let [n0, e1, e2] = frame(draws.unit() * 6.0, draws.unit() * 6.0, draws.unit() * 6.0)
            .map(|v| v.map(|x| x as f32));
        let s = draws.log(-22.0, 1.0) as f32;
        let (u, v) = (draws.unit() as f32, draws.unit() as f32);
        let (a, b) = ((u + u - 1.0) * s, (v + v - 1.0) * s);
        let d = [0, 1, 2].map(|i| a * e1[i] + b * e2[i]);
        let r = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        let (c, k) = (r.cos(), k_of(r, d));
        let want = [0, 1, 2].map(|i| c * n0[i] + k * d[i]);
        let got = exponential(n0, e1, e2, s, u, v);
        assert_eq!(
            got.map(f32::to_bits),
            want.map(f32::to_bits),
            "at r = {r:e} the f32 map {got:?} is not cos(r)·n₀ + k·d {want:?} with the form's k"
        );
    }
}

/// `‖d‖²` as the map forms it for the cut, its `fma` chain.
fn square_of(d: [f32; 3]) -> f32 {
    d[2].mul_add(d[2], d[1].mul_add(d[1], d[0] * d[0]))
}

#[test]
fn exponential_sinc_f32_map_reads_the_form() {
    check_map_f32(|r, d| sinc_f32(r, square_of(d)));
}

negative_control!(
    exponential_sinc_f32_map_reads_the_form,
    "the f32 map compared with the line before R-696 must differ in a bit",
    expected = "with the form's k",
    check_map_f32(|r, _| r.sin() / r)
);

/// The fragment's WGSL holds the form: the cut and each coefficient, written as naga writes an f32.
fn check_fragment(wgsl: &str) {
    for value in [
        SINC_CUT2_F32,
        SINC_SERIES_F32[0],
        SINC_SERIES_F32[1],
        SINC_SERIES_F32[2],
    ] {
        let literal = format!("{value:?}f");
        assert!(
            wgsl.contains(&literal),
            "decode.wgsl holds no {literal}: regenerate it, `cargo xtask build-kernel --bless`"
        );
    }
}

#[test]
fn exponential_sinc_fragment_regenerated() {
    check_fragment(include_str!("../../render/wgsl/decode.wgsl"));
}

negative_control!(
    exponential_sinc_fragment_regenerated,
    "the line before R-696 as WGSL holds none of the form's numbers",
    expected = "regenerate it",
    check_fragment("let k = select((sin(r) / r), 1f, (r == 0f));")
);
