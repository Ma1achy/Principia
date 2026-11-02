//! The kernel's own `atan2` (`kernel::arctan`; RQ-628, decided by the orchestrator per R-369, option (a)), on the CPU,
//! against its bound: no worse than libm's `atan2f`, the arithmetic `ε_phys` was calibrated on (`validation::arctan`'s
//! sweeps and measures).
//! - `arctan_f64_is_libm`: the f64 path is libm's `atan2`, bit for bit, on the plane sweep widened and on f64 draws.
//! - `arctan_special_values`: on every pair of signed zeros, subnormals, normals, the largest finite values, infinities
//!   and NaN, the f32 result is the f32 nearest f64 `atan2`, bit for bit, signed zeros and `±π` included, and NaN for
//!   NaN.
//! - `arctan_reduced`, `arctan_plane`, `arctan_round_trip`: CI's samples of the three sweeps, every 1021st `t` and `α`
//!   and 2²⁰ plane pairs. Against f64 `atan2`, no result is more than one f32 step from the f32 nearest the exact value,
//!   the largest error in ULP is at most libm's and the count of results not that nearest f32 at most libm's; on the
//!   round trip, the count of `α' ≠ α`, the most steps from `α`, and the largest relative error of the decode's `cos α'`
//!   and `sin α'` are each at most libm's.
//! - `arctan_entry_point`: the compute entry point `kernel::stage::encode::arctangent`, run natively, writes each pair's
//!   `atan2_f32` at its word and nothing for an invocation past the last pair.
//! - `arctan_reduced_exhaustive`, `arctan_plane_dense`, `arctan_round_trip_exhaustive`, ignored: the same checks on every
//!   f32 `t ∈ [0, 1]`, on 2²⁸ plane pairs and on every f32 `α ∈ [0, π/2]`, minutes in a release build (applied per R-369:
//!   too slow for CI; run with `cargo test --release -p kernel --test arctan -- --ignored`).
//!
//! **libm** (applied per R-369, a test decision): on macOS the platform's f32 `atan2`, `sin` and `cos`, Apple's libm,
//! on which `ε_phys` was calibrated; elsewhere the pinned pure-software libm's (`libm`, at the version `Cargo.lock`
//! holds), so a CI image's libm cannot move the bound.
//!
//! Each sweep prints Mesa's lowering of `atan2` for lavapipe, [`mesa_like`] (RQ-628), beside the two, and each control
//! runs a check on it, which the bound must refuse.

use std::ops::Range;
use std::thread;

use kernel::arctan::atan2_f32;
use kernel::stage::encode::arctangent;
use kernel::Real;
use spirv_std::glam::UVec3;
use spirv_std::num_traits::Float;
use validation::arctan::{plane, Accuracy, RoundTrip, HALF_PI_BELOW_BITS, ONE_BITS};
use validation::negative_control;

/// libm's f32 functions (the module's note).
#[cfg(target_os = "macos")]
mod libm32 {
    pub fn atan2(y: f32, x: f32) -> f32 {
        y.atan2(x)
    }
    pub fn sin(a: f32) -> f32 {
        a.sin()
    }
    pub fn cos(a: f32) -> f32 {
        a.cos()
    }
}

/// libm's f32 functions (the module's note).
#[cfg(not(target_os = "macos"))]
mod libm32 {
    pub fn atan2(y: f32, x: f32) -> f32 {
        libm::atan2f(y, x)
    }
    pub fn sin(a: f32) -> f32 {
        libm::sinf(a)
    }
    pub fn cos(a: f32) -> f32 {
        libm::cosf(a)
    }
}

/// An f32 `atan2`.
type Atan2 = fn(f32, f32) -> f32;

/// Mesa's lowering of `atan2` (`nir_atan2`, `nir_atan`), which lavapipe runs: `t = min/max` of the magnitudes, an odd
/// degree-11 polynomial in `t`, `π/2 −` it where `|y| > |x|`, `π −` that where `x < 0`, and `y`'s sign (RQ-628).
fn mesa_like(y: f32, x: f32) -> f32 {
    let (ay, ax) = (y.abs(), x.abs());
    let t = ay.min(ax) / ay.max(ax);
    let z = t * t;
    let p = t
        * (0.999_979_3
            + z * (-0.332_675_65
                + z * (0.193_892_5
                    + z * (-0.117_350_32 + z * (0.053_681_38 + z * -0.012_132_321)))));
    let p = if ay > ax {
        core::f32::consts::FRAC_PI_2 - p
    } else {
        p
    };
    let p = if x.is_sign_negative() {
        core::f32::consts::PI - p
    } else {
        p
    };
    p.copysign(y)
}

/// The threads a sweep runs on.
fn threads() -> u64 {
    thread::available_parallelism().map_or(1, |n| n.get().min(4) as u64)
}

/// `f` over the indices `0 … n − 1`, in parts among [`threads`], the parts' results merged; the sweep's values are
/// formed from their indices, so no sweep is held in memory whole.
fn par<T: Send + Default>(n: u64, f: impl Fn(Range<u64>) -> T + Sync, merge: fn(T, T) -> T) -> T {
    let t = threads();
    let size = n.div_ceil(t).max(1);
    thread::scope(|s| {
        let f = &f;
        let parts: Vec<_> = (0..t)
            .map(|i| s.spawn(move || f((i * size).min(n)..((i + 1) * size).min(n))))
            .collect();
        parts
            .into_iter()
            .map(|p| p.join().expect("a sweep thread"))
            .fold(T::default(), merge)
    })
}

/// A sweep of `n` pairs `(y, x)`, the `i`-th formed by `pair(i)`.
struct Sweep<'a> {
    name: &'a str,
    n: u64,
    pair: &'a (dyn Fn(u64) -> [f32; 2] + Sync),
}

/// `own`'s and libm's accuracy on `sweep`.
fn measure(sweep: &Sweep, own: Atan2) -> (Accuracy, Accuracy) {
    par(
        sweep.n,
        |range| {
            let (mut o, mut l) = (Accuracy::default(), Accuracy::default());
            for i in range {
                let [y, x] = (sweep.pair)(i);
                o.add([y, x], own(y, x));
                l.add([y, x], libm32::atan2(y, x));
            }
            (o, l)
        },
        |(a, b), (c, d)| (a.merge(c), b.merge(d)),
    )
}

/// Checks `own` against libm on `sweep` (the module's list), printing both.
fn check_accuracy(sweep: &Sweep, own: Atan2) {
    let name = sweep.name;
    let ((o, l), (m, _)) = (measure(sweep, own), measure(sweep, mesa_like));
    println!("{name}: own {o:?}\n{name}: libm {l:?}\n{name}: Mesa's lowering {m:?}");
    assert_eq!(
        o.far, 0,
        "{name}: a result more than one f32 step from the nearest: {o:?}"
    );
    assert!(
        o.worst <= l.worst,
        "{name}: a larger error in ULP than libm's: {o:?} against {l:?}"
    );
    assert!(
        o.off <= l.off,
        "{name}: more results off the nearest f32 than libm's: {o:?} against {l:?}"
    );
}

/// Checks `own` on the reduced sweep at `stride`, 1 included.
fn check_reduced(name: &str, stride: u32, own: Atan2) {
    let last = u64::from(ONE_BITS / stride);
    let pair = move |i: u64| {
        let t = if i > last {
            1.0
        } else {
            f32::from_bits(i as u32 * stride)
        };
        [t, 1.0]
    };
    check_accuracy(
        &Sweep {
            name,
            n: last + 2,
            pair: &pair,
        },
        own,
    );
}

/// Checks `own` on `parts` plane sweeps of `n` pairs each, from consecutive seeds after `seed`.
fn check_plane(name: &str, n: usize, parts: u64, seed: u64, own: Atan2) {
    for part in 0..parts {
        let yx = plane(n, seed + part);
        let pair = |i: u64| yx[i as usize];
        check_accuracy(
            &Sweep {
                name,
                n: n as u64,
                pair: &pair,
            },
            own,
        );
    }
}

/// `atan2`'s round trip over every `stride`-th angle of [`alphas`], `α' = atan2(sin α, cos α)` with libm's `sin` and
/// `cos`.
fn round_trip(stride: u32, atan2: Atan2) -> RoundTrip {
    par(
        u64::from(HALF_PI_BELOW_BITS / stride) + 1,
        |range| {
            let mut r = RoundTrip::default();
            for i in range {
                let a = f32::from_bits(i as u32 * stride);
                let (s, co) = (libm32::sin(a), libm32::cos(a));
                let back = atan2(s, co);
                r.add(a, [s, co], back, [libm32::sin(back), libm32::cos(back)]);
            }
            r
        },
        RoundTrip::merge,
    )
}

/// Checks `own`'s round trip against libm's over every `stride`-th angle (the module's list), printing both.
fn check_round_trip(stride: u32, own: Atan2) {
    let (o, l) = (round_trip(stride, own), round_trip(stride, libm32::atan2));
    let m = round_trip(stride, mesa_like);
    println!(
        "round trip: own {o:?}, share {:.6}%\nround trip: libm {l:?}, share {:.6}%\nround trip: Mesa's lowering {m:?}, \
         share {:.6}%",
        100.0 * o.share(),
        100.0 * l.share(),
        100.0 * m.share()
    );
    assert!(
        o.moved <= l.moved,
        "round trip: more α' ≠ α than libm's: {o:?} against {l:?}"
    );
    assert!(
        o.worst_steps <= l.worst_steps,
        "round trip: α' further from α than libm's: {o:?} against {l:?}"
    );
    assert!(
        o.worst_cos <= l.worst_cos,
        "round trip: a larger error in cos α' than libm's: {o:?} against {l:?}"
    );
    assert!(
        o.worst_sin <= l.worst_sin,
        "round trip: a larger error in sin α' than libm's: {o:?} against {l:?}"
    );
}

/// The values every pair of [`check_special`] is drawn from.
fn specials() -> Vec<f32> {
    let one: Vec<f32> = vec![
        0.0,
        f32::from_bits(1),
        f32::from_bits(0x0040_0000),
        f32::MIN_POSITIVE,
        1.0e-30,
        0.5,
        1.0,
        3.0,
        1.0e30,
        f32::MAX,
        f32::INFINITY,
    ];
    let mut all: Vec<f32> = one.iter().flat_map(|&v| [v, -v]).collect();
    all.push(f32::NAN);
    all
}

/// Checks `atan2` on every pair of [`specials`]: the f32 nearest f64 `atan2`, bit for bit, or NaN for NaN.
fn check_special(atan2: Atan2) {
    let s = specials();
    for &y in &s {
        for &x in &s {
            let got = atan2(y, x);
            let want = f64::from(y).atan2(f64::from(x)) as f32;
            if want.is_nan() {
                assert!(got.is_nan(), "atan2({y:e}, {x:e}) = {got:e}, not NaN");
            } else {
                assert_eq!(
                    got.to_bits(),
                    want.to_bits(),
                    "atan2({y:e}, {x:e}) = {got:e}, not {want:e}"
                );
            }
        }
    }
}

/// Checks that `atan2` at f64 is libm's, bit for bit, on the plane sweep widened and on `n` draws of f64 bits.
fn check_f64(atan2: fn(f64, f64) -> f64) {
    let widened = plane(1 << 16, 0x0628_0064)
        .into_iter()
        .map(|[y, x]| [f64::from(y), f64::from(x)]);
    let mut rng = validation::decode_f32_eps::SplitMix::new(0x0628_0065);
    let drawn = (0..1 << 16).map(|_| {
        [
            f64::from_bits(rng.next_u64()),
            f64::from_bits(rng.next_u64()),
        ]
    });
    for [y, x] in widened.chain(drawn) {
        let (got, want) = (atan2(y, x), Float::atan2(y, x));
        assert!(
            got.to_bits() == want.to_bits() || (got.is_nan() && want.is_nan()),
            "f64 atan2({y:e}, {x:e}) = {got:e}, not libm's {want:e}"
        );
    }
}

#[test]
fn arctan_f64_is_libm() {
    check_f64(Real::arctan2);
}

negative_control!(
    arctan_f64_is_libm,
    "the f64 path through the f32 function",
    expected = "not libm's",
    check_f64(|y, x| f64::from(atan2_f32(y as f32, x as f32)))
);

#[test]
fn arctan_special_values() {
    check_special(atan2_f32);
    check_special(Real::arctan2);
}

negative_control!(
    arctan_special_values,
    "Mesa's lowering, which ignores the sign of a zero x",
    expected = "atan2(",
    check_special(mesa_like)
);

/// The entry point's invocations `0 … n` over `n` pairs of [`specials`], run natively: each of the first `n` words is
/// `want`'s bits for its pair, and the word past them, which invocation `n` would write, keeps its fill.
fn check_entry(want: Atan2) {
    let s = specials();
    let pairs: Vec<[f32; 2]> = s
        .iter()
        .flat_map(|&y| s.iter().map(move |&x| [y, x]))
        .collect();
    let words: Vec<u32> = pairs.as_flattened().iter().map(|v| v.to_bits()).collect();
    let fill = 0xDEAD_BEEF;
    let mut out = vec![fill; pairs.len() + 1];
    for i in 0..=pairs.len() as u32 {
        arctangent(UVec3::new(i, 0, 0), &words, &mut out);
    }
    for (i, &[y, x]) in pairs.iter().enumerate() {
        let w = want(y, x);
        assert!(
            out[i] == w.to_bits() || (w.is_nan() && f32::from_bits(out[i]).is_nan()),
            "the entry point's word {i}, atan2({y:e}, {x:e}), is {:#010x}, not {:#010x}",
            out[i],
            w.to_bits()
        );
    }
    assert_eq!(
        out[pairs.len()],
        fill,
        "an invocation past the last pair wrote"
    );
}

#[test]
fn arctan_entry_point() {
    check_entry(atan2_f32);
}

negative_control!(
    arctan_entry_point,
    "the entry point's words held to Mesa's lowering",
    expected = "the entry point's word",
    check_entry(mesa_like)
);

#[test]
fn arctan_reduced() {
    check_reduced("reduced", 1021, atan2_f32);
}

negative_control!(
    arctan_reduced,
    "Mesa's lowering",
    expected = "reduced: a result more than one f32 step",
    check_reduced("reduced", 1021, mesa_like)
);

#[test]
fn arctan_plane() {
    check_plane("plane", 1 << 20, 1, 0x0628_0001, atan2_f32);
}

negative_control!(
    arctan_plane,
    "Mesa's lowering",
    expected = "plane: a result more than one f32 step",
    check_plane("plane", 1 << 20, 1, 0x0628_0001, mesa_like)
);

#[test]
fn arctan_round_trip() {
    check_round_trip(1021, atan2_f32);
}

negative_control!(
    arctan_round_trip,
    "Mesa's lowering",
    expected = "round trip: more α' ≠ α than libm's",
    check_round_trip(1021, mesa_like)
);

#[test]
#[ignore = "every f32 t in [0, 1]: minutes in a release build (the module's note)"]
fn arctan_reduced_exhaustive() {
    check_reduced("reduced, every t", 1, atan2_f32);
}

negative_control!(
    arctan_reduced_exhaustive,
    "Mesa's lowering, on every 64th t",
    expected = "reduced, every 64th t: a result more than one f32 step",
    check_reduced("reduced, every 64th t", 64, mesa_like)
);

#[test]
#[ignore = "2²⁸ plane pairs: minutes in a release build (the module's note)"]
fn arctan_plane_dense() {
    check_plane("plane, dense", 1 << 22, 1 << 6, 0x0628_0100, atan2_f32);
}

negative_control!(
    arctan_plane_dense,
    "Mesa's lowering, on one part",
    expected = "plane, dense: a result more than one f32 step",
    check_plane("plane, dense", 1 << 20, 1, 0x0628_0100, mesa_like)
);

#[test]
#[ignore = "every f32 α in [0, π/2]: minutes in a release build (the module's note)"]
fn arctan_round_trip_exhaustive() {
    check_round_trip(1, atan2_f32);
}

negative_control!(
    arctan_round_trip_exhaustive,
    "Mesa's lowering, on every 64th α",
    expected = "round trip: more α' ≠ α than libm's",
    check_round_trip(64, mesa_like)
);
