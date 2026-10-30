//! The parts of the kernel's own `atan2` (`kernel::arctan`; RQ-628, decided by the orchestrator per R-369, option (a)),
//! each against a reference in native integer or f64 arithmetic, on edge values and on draws from every magnitude.
//! The sweeps (`tests/arctan.rs`) hold the whole function to its bound; these hold each part to its own contract, so
//! that an error too small or too rare for a sweep's sample still fails.
//! - `arctan_mul`: `mul` and `mul_hi` are the 64-bit product and its high word.
//! - `arctan_add_sub`: `add` and `sub` are 64-bit sums and differences modulo 2⁶⁴, the carry and the borrow at their
//!   edges.
//! - `arctan_shifts`: `shr` and `shl` are 64-bit shifts for every `s` to 80, 0 from 64 on.
//! - `arctan_zeros`: `zeros` counts the leading zeros of 64 bits, 64 for 0.
//! - `arctan_divide`: `divide(num, den)` is `⌊num·2⁶⁴/den⌋` for `num < den`, the remainder's carry out of the word
//!   (`den ≥ 2³¹`) included.
//! - `arctan_unpack`: `unpack` gives `m·2ᵉ`, the float's value, `m ∈ [2²³, 2²⁴)`, on every exponent field, subnormals
//!   and `+∞` included; 0 gives `m = 0`.
//! - `arctan_round`: `round` is f64's correctly rounded conversion to f32, ties to even, subnormals and the sticky bit
//!   in the low word included.
//! - `arctan_series`: `atan_reduced` is f64's `atan` within `2⁻³⁶` relative for `|r| ≤ 1/16`.

use kernel::arctan::{
    add, atan_reduced, divide, mul, mul_hi, round, shl, shr, sub, unpack, zeros, Words,
};
use validation::decode_f32_eps::SplitMix;
use validation::negative_control;

/// `w` as a u64.
fn wide(w: Words) -> u64 {
    (u64::from(w.hi) << 32) | u64::from(w.lo)
}

/// `v` as words.
fn words(v: u64) -> Words {
    Words {
        hi: (v >> 32) as u32,
        lo: v as u32,
    }
}

/// The draws: magnitudes of every bit length, and the edges of a 16-bit half, a word and its sign bit.
fn u32s() -> Vec<u32> {
    let mut v = vec![
        0,
        1,
        2,
        0xFFFF,
        0x1_0000,
        0x1_FFFF,
        0xFFFF_0000,
        0x7FFF_FFFF,
        0x8000_0000,
        0x8000_0001,
        0xFFFF_FFFE,
        0xFFFF_FFFF,
    ];
    let mut rng = SplitMix::new(0x0628_0101);
    for _ in 0..4096 {
        let r = rng.next_u64();
        v.push((r as u32) >> ((r >> 32) % 32));
    }
    v
}

/// The 64-bit draws: [`u32s`] in each word, and magnitudes of every bit length.
fn u64s() -> Vec<u64> {
    let small = u32s();
    let mut v: Vec<u64> = small
        .iter()
        .step_by(97)
        .flat_map(|&h| {
            small
                .iter()
                .step_by(89)
                .map(move |&l| (u64::from(h) << 32) | u64::from(l))
        })
        .collect();
    let mut rng = SplitMix::new(0x0628_0102);
    for _ in 0..4096 {
        let r = rng.next_u64();
        v.push(r >> (rng.next_u64() % 64));
    }
    v
}

/// Checks `mul` and `mul_hi` (taken as `full` and `high`) against u64 products.
fn check_mul(full: fn(u32, u32) -> Words, high: fn(u32, u32) -> u32) {
    let v = u32s();
    for &a in &v {
        for &b in v.iter().step_by(7) {
            let want = u64::from(a) * u64::from(b);
            assert_eq!(wide(full(a, b)), want, "mul({a:#x}, {b:#x})");
            assert_eq!(u64::from(high(a, b)), want >> 32, "mul_hi({a:#x}, {b:#x})");
        }
    }
}

#[test]
fn arctan_mul() {
    check_mul(mul, mul_hi);
}

negative_control!(
    arctan_mul,
    "a product whose low word's top half is the middle sum's carry, not its low half",
    expected = "mul(",
    check_mul(
        |a, b| {
            let w = mul(a, b);
            Words {
                hi: w.hi,
                lo: w.lo ^ 0x1_0000,
            }
        },
        mul_hi
    )
);

/// Checks `add` and `sub` (taken as `plus` and `minus`) against wrapping u64 arithmetic.
fn check_add_sub(plus: fn(Words, Words) -> Words, minus: fn(Words, Words) -> Words) {
    let v = u64s();
    for &a in v.iter().step_by(3) {
        for &b in v.iter().step_by(11) {
            let (wa, wb) = (words(a), words(b));
            assert_eq!(wide(plus(wa, wb)), a.wrapping_add(b), "add({a:#x}, {b:#x})");
            assert_eq!(
                wide(minus(wa, wb)),
                a.wrapping_sub(b),
                "sub({a:#x}, {b:#x})"
            );
        }
    }
    for (a, b) in [
        (0xFFFF_FFFF, 1),
        (0x1_FFFF_FFFF, 0x1),
        (0x5, 0x1_0000_0000),
        (0x1_0000_0005, 0x5),
        (0x0, 0x1),
        (0x1_0000_0000, 0x1),
        (0x7, 0x7),
        (u64::MAX, u64::MAX),
    ] {
        let (wa, wb) = (words(a), words(b));
        assert_eq!(wide(plus(wa, wb)), a.wrapping_add(b), "add({a:#x}, {b:#x})");
        assert_eq!(wide(plus(wb, wa)), b.wrapping_add(a), "add({b:#x}, {a:#x})");
        assert_eq!(
            wide(minus(wa, wb)),
            a.wrapping_sub(b),
            "sub({a:#x}, {b:#x})"
        );
        assert_eq!(
            wide(minus(wb, wa)),
            b.wrapping_sub(a),
            "sub({b:#x}, {a:#x})"
        );
    }
}

#[test]
fn arctan_add_sub() {
    check_add_sub(add, sub);
}

negative_control!(
    arctan_add_sub,
    "a sum that drops the carry out of the low word",
    expected = "add(",
    check_add_sub(
        |a, b| Words {
            hi: a.hi.wrapping_add(b.hi),
            lo: a.lo.wrapping_add(b.lo)
        },
        sub
    )
);

/// Checks `shr` and `shl` (taken as `right` and `left`) against u64 shifts, 0 from 64 on.
fn check_shifts(right: fn(Words, u32) -> Words, left: fn(Words, u32) -> Words) {
    for &a in u64s().iter().step_by(5) {
        for s in 0..=80u32 {
            let (r, l) = if s >= 64 { (0, 0) } else { (a >> s, a << s) };
            assert_eq!(wide(right(words(a), s)), r, "shr({a:#x}, {s})");
            assert_eq!(wide(left(words(a), s)), l, "shl({a:#x}, {s})");
        }
    }
}

#[test]
fn arctan_shifts() {
    check_shifts(shr, shl);
}

negative_control!(
    arctan_shifts,
    "a right shift that loses the high word's bits spilled into the low",
    expected = "shr(",
    check_shifts(
        |a, s| {
            let w = shr(a, s);
            Words {
                hi: w.hi,
                lo: if s < 32 { a.lo >> s } else { w.lo },
            }
        },
        shl
    )
);

/// Checks `zeros` (taken as `count`) against u64's leading zeros.
fn check_zeros(count: fn(Words) -> u32) {
    for &a in &u64s() {
        assert_eq!(count(words(a)), a.leading_zeros(), "zeros({a:#x})");
    }
    for s in 0..64 {
        let a = 1u64 << s;
        assert_eq!(count(words(a)), a.leading_zeros(), "zeros({a:#x})");
    }
    assert_eq!(count(words(0)), 64, "zeros(0x0)");
}

#[test]
fn arctan_zeros() {
    check_zeros(zeros);
}

negative_control!(
    arctan_zeros,
    "a count that stops at the high word",
    expected = "zeros(",
    check_zeros(|a| zeros(Words { hi: a.hi, lo: 0 }).min(32))
);

/// Checks `divide` (taken as `div`) against u128 division, `num < den`.
fn check_divide(div: fn(u32, u32) -> Words) {
    let v = u32s();
    let mut rng = SplitMix::new(0x0628_0103);
    let mut pairs: Vec<(u32, u32)> = v
        .iter()
        .filter(|&&d| d > 0)
        .flat_map(|&d| [(0, d), (d - 1, d), (d / 2, d), (d / 3, d)])
        .collect();
    for &d in v.iter().filter(|&&d| d > 0) {
        pairs.push(((rng.next_u64() % u64::from(d)) as u32, d));
    }
    pairs.push((0x7FFF_FFFF, 0x8000_0000));
    pairs.push((0xFFFF_FFFE, 0xFFFF_FFFF));
    pairs.push((0x8000_0000, 0x8000_0001));
    for (num, den) in pairs {
        let want = ((u128::from(num) << 64) / u128::from(den)) as u64;
        assert_eq!(wide(div(num, den)), want, "divide({num:#x}, {den:#x})");
    }
}

#[test]
fn arctan_divide() {
    check_divide(divide);
}

negative_control!(
    arctan_divide,
    "a quotient short of its last bit",
    expected = "divide(",
    check_divide(|n, d| {
        let q = divide(n, d);
        Words {
            hi: q.hi,
            lo: q.lo & !1,
        }
    })
);

/// Checks `unpack` (taken as `parts`) on every exponent field, subnormals and `+∞` included.
fn check_unpack(parts: fn(u32) -> (u32, i32)) {
    let mut rng = SplitMix::new(0x0628_0104);
    let mut bits = vec![
        0,
        1,
        2,
        0x0040_0000,
        0x007F_FFFF,
        0x0080_0000,
        0x3F80_0000,
        0x7F7F_FFFF,
        0x7F80_0000,
    ];
    for field in 0..=254u32 {
        for _ in 0..16 {
            bits.push((field << 23) | (rng.next_u64() as u32 & 0x007F_FFFF));
        }
        bits.push(field << 23 | 1);
    }
    for b in bits {
        let (m, e) = parts(b);
        if b == 0 {
            assert_eq!(m, 0, "unpack({b:#010x}): 0");
            continue;
        }
        assert!(
            (1 << 23..1 << 24).contains(&m),
            "unpack({b:#010x}): m = {m:#x} is not normalised"
        );
        let want = if b == 0x7F80_0000 {
            2f64.powi(128)
        } else {
            f64::from(f32::from_bits(b))
        };
        assert_eq!(
            f64::from(m) * 2f64.powi(e),
            want,
            "unpack({b:#010x}) = ({m:#x}, {e})"
        );
    }
}

#[test]
fn arctan_unpack() {
    check_unpack(unpack);
}

negative_control!(
    arctan_unpack,
    "an exponent one too large",
    expected = "unpack(",
    check_unpack(|b| {
        let (m, e) = unpack(b);
        (m, e + 1)
    })
);

/// `(mant/2⁶⁴)·2^ex` rounded to f32 by f64: the bits below the round bit of a normal (bit 39) folded into one sticky
/// bit, so that the value is exact in f64 and its conversion rounds once.
fn rounded(mant: u64, ex: i32) -> u32 {
    let low = (1u64 << 39) - 1;
    let folded = (mant & !low) | if mant & low != 0 { 1 << 38 } else { 0 };
    (((folded as f64) * 2f64.powi(ex - 64)) as f32).to_bits()
}

/// Checks `round` (taken as `to_f32`) against [`rounded`] on normalised mantissas and `ex` from −200 to 2.
fn check_round(to_f32: fn(Words, i32) -> u32) {
    let mut rng = SplitMix::new(0x0628_0105);
    let mut mants = vec![
        0x8000_0000_0000_0000u64,
        0xFFFF_FFFF_FFFF_FFFF,
        0x8000_0080_0000_0000,
        0x8000_0180_0000_0000,
        0x8000_0080_0000_0001,
        0x8000_0180_0000_0001,
        0x8000_0000_0000_0001,
        0xFFFF_FF80_0000_0000,
        0xFFFF_FF7F_FFFF_FFFF,
    ];
    for _ in 0..2048 {
        let r = rng.next_u64() | 1 << 63;
        let cut = 64 - (rng.next_u64() % 40) as u32;
        let tie = r & !((1u64 << (64 - cut)) - 1);
        mants.extend([r, tie, tie | 1, tie | (1 << 31), tie | (1 << 32)]);
    }
    for &m in &mants {
        for ex in -200..=2 {
            let want = rounded(m, ex);
            let got = to_f32(words(m), ex);
            assert_eq!(
                got, want,
                "round({m:#018x}, {ex}) = {got:#010x}, not {want:#010x}"
            );
        }
    }
}

#[test]
fn arctan_round() {
    check_round(round);
}

negative_control!(
    arctan_round,
    "a rounding blind to the low word",
    expected = "round(",
    check_round(|m, ex| round(Words { hi: m.hi, lo: 0 }, ex))
);

/// Checks `atan_reduced` (taken as `atan`) against f64's `atan` within `2⁻³⁶` relative, for `|r| ≤ 1/16`.
fn check_series(atan: fn(Words, i32) -> Words) {
    assert_eq!(wide(atan(words(0), -10)), 0, "atan_reduced(0, -10)");
    let mut rng = SplitMix::new(0x0628_0106);
    let mut cases = vec![
        (1u64 << 63, -3),
        (u64::MAX, -4),
        (1 << 63, -4),
        (u64::MAX, -60),
    ];
    for e in -60..=-4 {
        for _ in 0..256 {
            cases.push((rng.next_u64() | 1 << 63, e));
        }
    }
    for (m, e) in cases {
        let r = (m as f64) * 2f64.powi(e - 64);
        let want = r.atan();
        let got = wide(atan(words(m), e)) as f64 * 2f64.powi(e - 64);
        assert!(
            (got - want).abs() <= 2f64.powi(-36) * want,
            "atan_reduced({m:#018x}, {e}) = {got:e}, not atan({r:e}) = {want:e}"
        );
    }
}

#[test]
fn arctan_series() {
    check_series(atan_reduced);
}

negative_control!(
    arctan_series,
    "the series' first correction only, atan r ≈ r − r³/3, its next term dropped",
    expected = "atan_reduced(",
    check_series(|m, e| {
        let third = mul_hi(m.hi, m.hi);
        let s = (-2 * e) as u32;
        let z = if s >= 32 { 0 } else { third >> s };
        sub(m, shr(mul(m.hi, mul_hi(z, 0x5555_5555)), s))
    })
);
