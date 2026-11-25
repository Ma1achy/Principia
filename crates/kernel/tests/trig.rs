//! The kernel's own f32 sine and cosine (`kernel::trig::sin_cos_f32`), on the CPU. The function is integer arithmetic
//! alone, so these numbers are the GPU's too; `crates/engine/tests/cos_half_pi_gpu.rs` checks that identity on each
//! backend.
//!
//! The criteria, written before the first run:
//! - `trig_cos_near_half_pi`: every f32 `x` from `π/4` (rounded) to the decode's largest `α` (`LIT_6`, the f32 below
//!   `π/2`), `|cos(x) − cos_f64(x)| ≤ δ_max·|cos_f64(x)|`, `δ_max = ε_phys/4 = 16·2⁻²³`, the bound
//!   `gpu_cos_near_half_pi` holds the GPU to (its note gives the budget).
//! - `trig_nearest`: on every 997th f32 bit pattern (finite, 4.3 million of them) and every 7th f32 in `[π/4, π/2]`
//!   (`trig_nearest_exhaustive`, ignored, takes every f32), sine and
//!   cosine are each the f32 nearest the exact value, or the exact value lies within `2⁻³⁶` relative of the midpoint
//!   between the result and its neighbour (the module's accuracy claim, `2⁻⁴⁰`, with margin for the f64 reference,
//!   `2⁻⁵³`). The exact value is the pinned libm's f64 `sin`/`cos` of the f32 argument.
//! - `trig_mul_q64`: the GPU's 128-bit product's top half from 32-bit words, `mul_q64_words`, is the u128 product's,
//!   on edge values and 2²⁰ draws (the host takes the u128 product itself).
//! - `trig_entry_point`: the GPU entry `kernel::stage::decode::sine_cosine`, run natively, writes each argument's sine
//!   and cosine at words `2i` and `2i + 1`, and nothing past the last argument.
//! - `trig_special_values`: `sin(±0) = ±0`, `cos(±0) = 1`; NaN and `±∞` give NaN; `sin(−x) = −sin x` and
//!   `cos(−x) = cos x` bit for bit; `sin(π̂) < 0` (`π̂ > π`), `cos(LIT_6) > 0`; the signs the shape sphere's `atan2`
//!   selects depend on (pitfalls §11).

use kernel::generated::links::Literals;
use kernel::real::Real;
use kernel::trig::sin_cos_f32;
use validation::{negative_control, EPS_PHYS};

/// The decode's largest `α`: the f32 image of `π/2`, rounded toward zero.
const TOP: f32 = <f32 as Literals>::LIT_6;

/// The relative bound near `π/2`, `ε_phys/4`.
fn bound() -> f64 {
    EPS_PHYS.number() * f64::from(f32::EPSILON) / 4.0
}

/// One f32 function under test: `(sin x, cos x)`.
type SinCos = fn(f32) -> (f32, f32);

/// The worst relative error of `f`'s cosine on `[π/4, TOP]`, its argument, and how many pass the bound.
fn near_half_pi(f: SinCos) -> (f64, f32, usize) {
    let (mut worst, mut at, mut over) = (0.0f64, 0.0f32, 0);
    for bits in std::f32::consts::FRAC_PI_4.to_bits()..=TOP.to_bits() {
        let x = f32::from_bits(bits);
        let want = f64::from(x).cosine();
        let d = (f64::from(f(x).1) - want).abs() / want.abs();
        if d.is_nan() || d > bound() {
            over += 1;
        }
        if d.is_nan() || d > worst {
            (worst, at) = (d, x);
        }
    }
    (worst, at, over)
}

fn check_near_half_pi(f: SinCos) {
    let (worst, at, over) = near_half_pi(f);
    let eps = f64::from(f32::EPSILON);
    eprintln!(
        "cos on [π/4, LIT_6]: worst relative error {:.4} f32 eps at x = {at:e} ({:#010x}); bound {:.1} eps",
        worst / eps,
        at.to_bits(),
        bound() / eps
    );
    assert!(
        over == 0,
        "{over} arguments' cos exceeds the decode's relative bound; worst {:.1} eps at {at:e}",
        worst / eps
    );
}

#[test]
fn trig_cos_near_half_pi() {
    check_near_half_pi(sin_cos_f32);
}

/// The control's stand-in: the obvious float form, `cos α = −sin(α − π̂/2)` with the f32 `π/2` alone, whose low part
/// is dropped, so near `π/2` its error is the whole of `cos α`.
#[cfg(feature = "controls")]
fn one_part_reduction(x: f32) -> (f32, f32) {
    let half_pi = std::f32::consts::FRAC_PI_2;
    (sin_cos_f32(x).0, -sin_cos_f32(x - half_pi).0)
}

negative_control!(
    trig_cos_near_half_pi,
    "cos from a one-part π/2 reduction",
    expected = "exceeds the decode's relative bound",
    check_near_half_pi(one_part_reduction)
);

/// Whether `got` is the f32 nearest `want`, or `want` lies within `2⁻³⁶` relative of the midpoint between `got` and
/// the neighbour on `want`'s side.
fn nearest(got: f32, want: f64) -> bool {
    if got.is_nan() || !want.is_finite() {
        return false;
    }
    let g = f64::from(got);
    if (want as f32).to_bits() == got.to_bits() {
        return true;
    }
    let next = if want > g {
        got.next_up()
    } else {
        got.next_down()
    };
    let mid = (g + f64::from(next)) / 2.0;
    (want - mid).abs() <= want.abs() * 2f64.powi(-36)
}

/// The arguments where `f` misses [`nearest`], with the first few; `xs` the arguments.
fn misses(f: SinCos, xs: impl Iterator<Item = f32>) -> (usize, usize, Vec<String>) {
    let (mut n, mut bad, mut first) = (0, 0, Vec::new());
    for x in xs {
        n += 1;
        let (s, c) = f(x);
        let (ws, wc) = (f64::from(x).sine(), f64::from(x).cosine());
        for (name, got, want) in [("sin", s, ws), ("cos", c, wc)] {
            if !nearest(got, want) {
                bad += 1;
                if first.len() < 5 {
                    first.push(format!(
                        "{name}({x:e} = {:#010x}) = {got:e}, exact {want:e}",
                        x.to_bits()
                    ));
                }
            }
        }
    }
    (n, bad, first)
}

fn check_nearest(f: SinCos) {
    let sweep = (0..=u32::MAX)
        .step_by(997)
        .map(f32::from_bits)
        .filter(|x| x.is_finite());
    let quarter = (std::f32::consts::FRAC_PI_4.to_bits()..=std::f32::consts::FRAC_PI_2.to_bits())
        .step_by(7)
        .map(f32::from_bits);
    let (n, bad, first) = misses(f, sweep.chain(quarter));
    eprintln!("sin and cos: {n} arguments, {bad} results not the nearest f32 (within 2⁻³⁶ of a midpoint allowed)");
    assert!(
        bad == 0,
        "not the nearest f32: {bad} results, first {}",
        first.join("; ")
    );
}

#[test]
fn trig_nearest() {
    check_nearest(sin_cos_f32);
}

/// The control's stand-in: the function's results one ulp up.
#[cfg(feature = "controls")]
fn one_ulp_up(x: f32) -> (f32, f32) {
    let (s, c) = sin_cos_f32(x);
    (
        f32::from_bits(s.to_bits() + 1),
        f32::from_bits(c.to_bits() + 1),
    )
}

negative_control!(
    trig_nearest,
    "sin and cos one ulp up",
    expected = "not the nearest f32",
    check_nearest(one_ulp_up)
);

fn check_special(f: SinCos) {
    let bits = |x: f32| x.to_bits();
    assert_eq!(bits(f(0.0).0), 0, "sin(+0) is +0");
    assert_eq!(bits(f(-0.0).0), 0x8000_0000, "sin(−0) is −0");
    assert_eq!(f(0.0).1, 1.0, "cos(+0) is 1");
    assert_eq!(f(-0.0).1, 1.0, "cos(−0) is 1");
    for x in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert!(
            f(x).0.is_nan() && f(x).1.is_nan(),
            "sin and cos of {x} are NaN"
        );
    }
    assert!(f(std::f32::consts::PI).0 < 0.0, "sin(π̂) is negative: π̂ > π");
    assert!(f(TOP).1 > 0.0, "cos(LIT_6) is positive");
    for b in (0..0x7F80_0000u32).step_by(997) {
        let x = f32::from_bits(b);
        assert_eq!(
            bits(f(-x).0),
            bits(f(x).0) ^ 0x8000_0000,
            "sin(−x) is −sin x at {x:e}"
        );
        assert_eq!(bits(f(-x).1), bits(f(x).1), "cos(−x) is cos x at {x:e}");
    }
}

#[test]
fn trig_special_values() {
    check_special(sin_cos_f32);
}

negative_control!(
    trig_special_values,
    "a sine that drops the sign of −0",
    expected = "sin(−0) is −0",
    check_special(|x| {
        let (s, c) = sin_cos_f32(x);
        (if s == 0.0 { 0.0 } else { s }, c)
    })
);

/// Every finite f32, too slow for CI: `cargo test -p kernel --release --test trig -- --ignored`.
#[test]
#[ignore = "every f32: minutes in release"]
fn trig_nearest_exhaustive() {
    check_exhaustive(sin_cos_f32, (0..=u32::MAX).map(f32::from_bits));
}

fn check_exhaustive(f: SinCos, all: impl Iterator<Item = f32>) {
    let (n, bad, first) = misses(f, all.filter(|x| x.is_finite()));
    eprintln!("sin and cos, exhaustive: {n} arguments, {bad} not the nearest f32");
    assert!(
        bad == 0,
        "exhaustively, not the nearest f32: {bad} results, first {}",
        first.join("; ")
    );
}

// The control runs the same check over one binade, not every f32, so it takes a second.
negative_control!(
    trig_nearest_exhaustive,
    "the cosine one ulp up at a single argument",
    expected = "exhaustively, not the nearest f32",
    check_exhaustive(
        |x| {
            let (s, c) = sin_cos_f32(x);
            if x.to_bits() == 0x3F80_0000 {
                (s, f32::from_bits(c.to_bits() + 1))
            } else {
                (s, c)
            }
        },
        (0x3F80_0000u32..0x4000_0000).map(f32::from_bits)
    )
);

/// `w` as a u64.
fn wide(w: kernel::arctan::Words) -> u128 {
    (u128::from(w.hi) << 32) | u128::from(w.lo)
}

fn check_mul_q64(f: fn(kernel::arctan::Words, kernel::arctan::Words) -> kernel::arctan::Words) {
    let words = |v: u64| kernel::arctan::Words {
        hi: (v >> 32) as u32,
        lo: v as u32,
    };
    let edges = [
        0,
        1,
        u64::MAX,
        1 << 63,
        (1 << 32) - 1,
        1 << 32,
        0xFFFF_FFFF_0000_0001,
    ];
    let mut s = 0x2545_F491_4F6C_DD1Du64;
    let mut draw = || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        s >> (s % 64)
    };
    let pairs = edges
        .iter()
        .flat_map(|&a| edges.iter().map(move |&b| (a, b)))
        .chain((0..1 << 20).map(|_| (draw(), draw())));
    for (a, b) in pairs {
        let want = (u128::from(a) * u128::from(b)) >> 64;
        assert!(
            wide(f(words(a), words(b))) == want,
            "mul_q64({a:#x}, {b:#x}) is not the top of the u128 product"
        );
    }
}

#[test]
fn trig_mul_q64() {
    check_mul_q64(kernel::trig::mul_q64_words);
}

negative_control!(
    trig_mul_q64,
    "the low products' carry dropped",
    expected = "is not the top of the u128 product",
    check_mul_q64(|a, b| {
        let w = kernel::arctan::mul_halves(a.hi, b.hi);
        let hl = kernel::arctan::mul_halves(a.hi, b.lo);
        let lh = kernel::arctan::mul_halves(a.lo, b.hi);
        kernel::arctan::add(
            kernel::arctan::add(w, kernel::arctan::Words { hi: 0, lo: hl.hi }),
            kernel::arctan::Words { hi: 0, lo: lh.hi },
        )
    })
);

/// The entry point's invocations `0 … n` over `n` arguments, run natively: words `2i` and `2i + 1` are `f`'s sine and
/// cosine of argument `i`, and the two words past them, which invocation `n` would write, keep their fill.
fn check_entry(f: SinCos) {
    use spirv_std::glam::UVec3;
    let xs: Vec<f32> = [
        0.0f32,
        -0.0,
        1e-30,
        0.5,
        1.0,
        TOP,
        std::f32::consts::PI,
        1e30,
        f32::INFINITY,
        f32::NAN,
    ]
    .into_iter()
    .flat_map(|x| [x, -x])
    .collect();
    let words: Vec<u32> = xs.iter().map(|x| x.to_bits()).collect();
    let fill = 0xDEAD_BEEF;
    let mut out = vec![fill; 2 * xs.len() + 2];
    for i in 0..=xs.len() as u32 {
        kernel::stage::decode::sine_cosine(UVec3::new(i, 0, 0), &words, &mut out);
    }
    for (i, &x) in xs.iter().enumerate() {
        let (s, c) = f(x);
        for (k, w) in [(2 * i, s), (2 * i + 1, c)] {
            assert!(
                out[k] == w.to_bits() || (w.is_nan() && f32::from_bits(out[k]).is_nan()),
                "the entry point's word {k}, of {x:e}, is {:#010x}, not {:#010x}",
                out[k],
                w.to_bits()
            );
        }
    }
    assert_eq!(
        &out[2 * xs.len()..],
        [fill, fill],
        "an invocation past the last argument wrote"
    );
}

#[test]
fn trig_entry_point() {
    check_entry(sin_cos_f32);
}

negative_control!(
    trig_entry_point,
    "the entry's words held to a sine and cosine swapped",
    expected = "the entry point's word",
    check_entry(|x| {
        let (s, c) = sin_cos_f32(x);
        (c, s)
    })
);
