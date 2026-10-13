//! QA tests for TASK-M2-30 on the kernel side (`kernel::generated::links`), written from REQ-GEN-034 and R-417, not
//! from the implementation:
//! - the f32 images of π/2 and π the generated links use are `fl↓(π/2)` and `fl↓(π)`, the largest f32 not above
//!   each, bits `0x3FC9_0FDA` and `0x4049_0FDA`; the f64 images are the nearest f64 values, which lie below;
//! - every bounded link's output, at f32 and f64, saturated controls included, lies in the closed codomain at the
//!   exact bounds (α ∈ [0, π/2], β ∈ [0, π], q ∈ [−q_max, q_max], q_max = 2), compared against π itself, not against
//!   a float image of it;
//! - each lower end is reached where the link's output underflows, and `q` reaches `±q_max` at saturation (R-417, "the
//!   range is closed").
//!
//! "Below π" is decided exactly: a float is `m·2⁻ˢ` with integer `m`, and π is bracketed by its first 30 decimals,
//! `P·10⁻³⁰ ≤ π < (P + 1)·10⁻³⁰`; the comparison is done in integers. The f32 images are read from the links' own
//! outputs at saturation (`σ = 1` gives `width·1`, the literal itself), so the test needs no name of the generator's.

use std::cmp::Ordering;

#[cfg(feature = "controls")]
use kernel::generated::links::LinkReal;
use kernel::generated::links::{
    Link, SigmoidAlpha, SigmoidBeta, SigmoidQ, SoftsignAlpha, SoftsignBeta, SoftsignQ, TanhAlpha,
    TanhBeta, TanhQ,
};
use validation::negative_control;

// ── π, exactly enough ───────────────────────────────────────────────────────────────────────────────────────────

/// ⌊π·10³⁰⌋: π = 3.141592653589793238462643383279502884…
const PI_E30: u128 = 3_141_592_653_589_793_238_462_643_383_279;
/// ⌊π·10²⁰⌋, for f64 values, whose 53-bit mantissas leave no room for 10³⁰ in a u128.
const PI_E20: u128 = 314_159_265_358_979_323_846;

/// `m·2⁻ˢ` against `π/k` (k = 1 or 2), exactly: compares `m·k·10ᵈ` with `π·10ᵈ·2ˢ`, bracketed by the truncated
/// decimals `[P·2ˢ, (P + 1)·2ˢ)`. Panics if the bracket straddles the float (never for a float with < 60 bits).
fn cmp_pi(m: u128, s: u32, k: u128) -> Ordering {
    let (p, scale) = if m < 1 << 25 {
        (PI_E30, 10u128.pow(30))
    } else {
        (PI_E20, 10u128.pow(20))
    };
    let lhs = m * k * scale;
    let lo = p << s;
    let hi = (p + 1) << s;
    if lhs < lo {
        Ordering::Less
    } else if lhs >= hi {
        Ordering::Greater
    } else {
        panic!("π's decimals do not decide m = {m}, s = {s}")
    }
}

/// A positive normal f32 as `(m, s)`, its value `m·2⁻ˢ`.
fn parts32(v: f32) -> (u128, u32) {
    assert!(
        v.is_normal() && v > 0.0 && v < 4.0,
        "{v} is not in [2⁻¹²⁶, 4)"
    );
    let b = v.to_bits();
    let e = ((b >> 23) & 0xff) as i32 - 127;
    (u128::from((b & 0x7f_ffff) | 0x80_0000), (23 - e) as u32)
}

/// A positive normal f64 in [1, 4) as `(m, s)`.
fn parts64(v: f64) -> (u128, u32) {
    assert!(
        v.is_normal() && (1.0..4.0).contains(&v),
        "{v} is not in [1, 4)"
    );
    let b = v.to_bits();
    let e = ((b >> 52) & 0x7ff) as i32 - 1023;
    (
        u128::from((b & 0xf_ffff_ffff_ffff) | 0x10_0000_0000_0000),
        (52 - e) as u32,
    )
}

/// `v` is `fl↓(π/k)`, the largest f32 not above π/k: below it, and the next f32 up above it.
fn check_toward_zero(v: f32, k: u128, what: &str) {
    let (m, s) = parts32(v);
    let below = cmp_pi(m, s, k) == Ordering::Less;
    let next_above = cmp_pi(m + 1, s, k) == Ordering::Greater;
    assert!(
        below && next_above,
        "{what}: {v} (bits {:#010X}) is not fl↓(π/{k}), the largest f32 not above π/{k} (below: {below}; next \
         above: {next_above}) (R-417)",
        v.to_bits()
    );
}

/// `v` is the f64 nearest π/k: π/k lies within half an ulp of it, `(2m − 1)·2⁻⁽ˢ⁺¹⁾ < π/k < (2m + 1)·2⁻⁽ˢ⁺¹⁾`.
fn check_nearest(v: f64, k: u128, what: &str) {
    let (m, s) = parts64(v);
    let lower = cmp_pi(2 * m - 1, s + 1, k) == Ordering::Less;
    let upper = cmp_pi(2 * m + 1, s + 1, k) == Ordering::Greater;
    assert!(
        lower && upper,
        "{what}: {v} is not the f64 nearest π/{k} (R-417)"
    );
}

// ── The f32 images are fl↓ and the f64 images the nearest ───────────────────────────────────────────────────────

/// The width of `sigmoid_alpha` and `sigmoid_beta` at a saturated control: `a + (b − a)·σ(x)` with `a = 0`
/// (α_min = 0, REQ-DEC-009) and `σ = 1`, so the image of π/2 and of π the link carries at `R`.
fn images<R: kernel::generated::links::LinkReal>(big: R) -> (R, R) {
    (
        SigmoidAlpha::forward([big])[0],
        SigmoidBeta::forward([big])[0],
    )
}

fn check_f32_images(half_pi: f32, pi: f32) {
    check_toward_zero(half_pi, 2, "the f32 image of π/2");
    check_toward_zero(pi, 1, "the f32 image of π");
    assert_eq!(
        (half_pi.to_bits(), pi.to_bits()),
        (0x3FC9_0FDA, 0x4049_0FDA),
        "the f32 images' bits are not R-417's"
    );
}

#[test]
fn qa_f32_pi_images_toward_zero() {
    let (half_pi, pi) = images(f32::MAX);
    check_f32_images(half_pi, pi);
}

negative_control!(
    qa_f32_pi_images_toward_zero,
    "the round-to-nearest f32 images of π/2 and π (0x…0FDB) lie above them and must fail",
    expected = "is not fl↓(π/2)",
    check_f32_images(std::f32::consts::FRAC_PI_2, std::f32::consts::PI)
);

fn check_f64_images(half_pi: f64, pi: f64) {
    check_nearest(half_pi, 2, "the f64 image of π/2");
    check_nearest(pi, 1, "the f64 image of π");
    for (v, k) in [(half_pi, 2), (pi, 1)] {
        let (m, s) = parts64(v);
        assert_eq!(
            cmp_pi(m, s, k),
            Ordering::Less,
            "the f64 image of π/{k} is not below it"
        );
    }
}

#[test]
fn qa_f64_pi_images_nearest() {
    let (half_pi, pi) = images(f64::MAX);
    check_f64_images(half_pi, pi);
}

negative_control!(
    qa_f64_pi_images_nearest,
    "the f64 one ulp below π is not the nearest and must fail",
    expected = "is not the f64 nearest π/1",
    check_f64_images(
        std::f64::consts::FRAC_PI_2,
        f64::from_bits(std::f64::consts::PI.to_bits() - 1)
    )
);

// ── The closed range at the exact bounds ────────────────────────────────────────────────────────────────────────

/// A bounded codomain, from the docs (dd_decoder §3.2, §3.4; REQ-DEC-009: α_min = 0, q_max = 2).
#[derive(Clone, Copy, Debug, PartialEq)]
enum Range {
    /// `[0, π/k]`.
    ZeroToPiOver(u128),
    /// `[−2, 2]`.
    Q,
}

/// One bounded link, behind function pointers so a control can stand in.
struct Bounded {
    name: &'static str,
    range: Range,
    f32: fn(f32) -> f32,
    f64: fn(f64) -> f64,
}

fn bounded<L: Link<1, 1>>(range: Range) -> Bounded {
    Bounded {
        name: L::NAME,
        range,
        f32: |x| L::forward([x])[0],
        f64: |x| L::forward([x])[0],
    }
}

/// The nine bounded entries of dd_generation_root §3.9: σ, tanh and softsign onto α, β and q.
fn all_bounded() -> Vec<Bounded> {
    use Range::{ZeroToPiOver, Q};
    vec![
        bounded::<SigmoidAlpha>(ZeroToPiOver(2)),
        bounded::<SigmoidBeta>(ZeroToPiOver(1)),
        bounded::<SigmoidQ>(Q),
        bounded::<TanhAlpha>(ZeroToPiOver(2)),
        bounded::<TanhBeta>(ZeroToPiOver(1)),
        bounded::<TanhQ>(Q),
        bounded::<SoftsignAlpha>(ZeroToPiOver(2)),
        bounded::<SoftsignBeta>(ZeroToPiOver(1)),
        bounded::<SoftsignQ>(Q),
    ]
}

/// The controls: saturated (the float's ends, and the onset of saturation for σ and tanh at f32 and f64), and a
/// sweep of every magnitude from 2⁻²⁰ to 2¹²⁰ in steps of 2^(1/64), each with both signs.
fn controls(max: f64) -> Vec<f64> {
    let mut x = vec![0.0, max, -max];
    let mut v = 2f64.powi(-20);
    while v < 2f64.powi(120) {
        x.push(v);
        x.push(-v);
        v *= 2f64.powf(1.0 / 64.0);
    }
    for k in 0..=4000 {
        let v = f64::from(k) * 0.01;
        x.push(v);
        x.push(-v);
    }
    x
}

/// `y` (a float's value, exactly, in f64) lies in `r` at its exact bounds.
fn in_range(y: f64, r: Range) -> Result<(), String> {
    if !y.is_finite() {
        return Err(format!("{y} is not finite"));
    }
    match r {
        Range::Q if (-2.0..=2.0).contains(&y) => Ok(()),
        Range::Q => Err(format!("{y} leaves [−q_max, q_max]")),
        Range::ZeroToPiOver(_) if y < 0.0 => Err(format!("{y} is below 0")),
        Range::ZeroToPiOver(_) if y < 1.0 => Ok(()),
        Range::ZeroToPiOver(k) => {
            let (m, s) = parts64(y);
            if cmp_pi(m, s, k) == Ordering::Greater {
                Err(format!("{y} passes π/{k}, the exact upper bound"))
            } else {
                Ok(())
            }
        }
    }
}

fn check_closed(u: &Bounded) {
    for x in controls(f64::from(f32::MAX)) {
        let y = (u.f32)(x as f32);
        if let Err(e) = in_range(f64::from(y), u.range) {
            panic!("{} at f32 x = {x}: {e} (closed range)", u.name);
        }
    }
    for x in controls(f64::MAX) {
        let y = (u.f64)(x);
        if let Err(e) = in_range(y, u.range) {
            panic!("{} at f64 x = {x}: {e} (closed range)", u.name);
        }
    }
}

#[test]
fn qa_bounded_links_closed_at_exact_bounds() {
    for u in all_bounded() {
        check_closed(&u);
    }
}

/// `v` at the float `R`, rounded to nearest.
#[cfg(feature = "controls")]
fn lit<R: LinkReal>(v: f64) -> R {
    <R as spirv_std::num_traits::NumCast>::from(v).expect("a float converts")
}

/// σ onto β with the round-to-nearest image of π, `fl(π)·σ(x)`, as the generator emitted before R-417.
#[cfg(feature = "controls")]
fn nearest_beta<R: LinkReal>(x: R) -> R {
    let one = R::one();
    lit::<R>(std::f64::consts::PI) * (one / (one + (-x).exp()))
}

negative_control!(
    qa_bounded_links_closed_at_exact_bounds,
    "sigmoid_beta with the round-to-nearest f32 image of π passes π at f32 saturation and must fail",
    expected = "passes π/1, the exact upper bound",
    check_closed(&Bounded {
        name: "nearest_beta",
        range: Range::ZeroToPiOver(1),
        f32: nearest_beta::<f32>,
        f64: nearest_beta::<f64>,
    })
);

// ── The ends reached: the lower poles at underflow, q at ±q_max ─────────────────────────────────────────────────

/// At f32 and f64, a control saturated downwards gives each lower end exactly (α = 0, β = 0, q = −q_max), and one
/// saturated upwards gives q = +q_max exactly (R-417; dd_generation_root §3.9, "The closed range").
fn check_ends(u: &Bounded) {
    let lo = match u.range {
        Range::Q => -2.0,
        Range::ZeroToPiOver(_) => 0.0,
    };
    for (x, y) in [
        (-f64::from(f32::MAX), f64::from((u.f32)(-f32::MAX))),
        (-f64::MAX, (u.f64)(-f64::MAX)),
    ] {
        assert!(
            y == lo,
            "{} at x = {x} gives {y}: it does not reach its lower end {lo} (ends)",
            u.name
        );
    }
    if u.range == Range::Q {
        for (x, y) in [
            (f64::from(f32::MAX), f64::from((u.f32)(f32::MAX))),
            (f64::MAX, (u.f64)(f64::MAX)),
        ] {
            assert!(
                y == 2.0,
                "{} at x = {x} gives {y}: it does not reach q_max (ends)",
                u.name
            );
        }
    }
}

#[test]
fn qa_bounded_links_reach_their_ends() {
    for u in all_bounded() {
        check_ends(&u);
    }
}

/// σ onto q floored away from the lower end: `−q_max + 10⁻³ + (2q_max − 10⁻³)·σ(x)`.
#[cfg(feature = "controls")]
fn floored_q<R: kernel::generated::links::LinkReal>(x: R) -> R {
    let one = R::one();
    let two = one + one;
    let gap = one / (two * two * two * (two + two + one) * (two + two + one) * (two + two + one));
    -(two) + gap + (two + two - gap) * (one / (one + (-x).exp()))
}

negative_control!(
    qa_bounded_links_reach_their_ends,
    "a link onto q that stops short of −q_max must fail",
    expected = "does not reach its lower end",
    check_ends(&Bounded {
        name: "floored_q",
        range: Range::Q,
        f32: floored_q::<f32>,
        f64: floored_q::<f64>,
    })
);
