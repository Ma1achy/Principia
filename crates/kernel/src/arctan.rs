//! The kernel's `atan2` at each `Real`, [`crate::Real::arctan2`] (RQ-628, decided by the orchestrator per R-369,
//! option (a); REQ-ENC-024, R-297, R-116): at f64, the CPU's alone, libm's `atan2`, unchanged bit for bit; at f32, the
//! GPU's precision, [`atan2_f32`], the kernel's own, in integer arithmetic.
//!
//! **Why the kernel has its own.** The fragment's ROUNDTRIP preset runs the encode as the backend compiles it (R-297),
//! and WGSL bounds `atan2` only to 4096 ULP: lavapipe's, Mesa's lowering, is off by about `2·10⁻⁵` relative to
//! `π/2 − α` near `π/2`. The decode rebuilds `‖ρ̃‖` from `cos α`, so an absolute error in the encode's `α` near `π/2`
//! returns as a relative error in `ρ̃` larger by `1/cos α`, past `ε_phys` for a light body (RQ-628). So the encode's
//! angles at f32 (`crate::encode::blocks`'s `α` and `β`, `crate::encode::by_kind`'s spherical and exponential
//! inverses) and the shape sphere decode's `β` (`crate::chart::shape_sphere::inverse`) take this module's `atan2`, never
//! the backend's. The callers' selects at the poles (R-437) are theirs, unchanged.
//!
//! **Integer arithmetic, so every backend gives the CPU's bits.** [`atan2_f32`] reads its operands' bits and computes
//! in 32-bit integers alone, two words where it needs 64 bits (WGSL has no u64): shifts, comparisons, selects and
//! wrapping sums and products, each exact. Nothing in it is a float operation a backend may round differently, fuse,
//! reassociate or fold under fast-math, and no float constant is read: its numbers are the generated fixed-point
//! fractions of `crate::generated::arctan`, summed by the generator in integer arithmetic. So the GPU's f32 `atan2`, as
//! Metal and lavapipe compile it, is the CPU's f32 `atan2`, bit for bit, and its accuracy is the CPU's.
//!
//! **Accuracy** (the bound RQ-628's decision sets: no worse than libm's `atan2f`, the arithmetic `ε_phys` was
//! calibrated on). The result is the f32 nearest a value within about `2⁻³⁷` relative of `atan2(y, x)`: its error
//! is at most about 0.50002 ULP of the f32 nearest the exact value, and it differs from that nearest f32 only where the
//! exact value lies within that `2⁻³⁷` of the midpoint between two f32s. The kernel's tests sweep it exhaustively against
//! f64 `atan2` and against libm's `atan2f` (`crates/kernel/tests/arctan.rs`).
//!
//! **The algorithm** ([`atan2_f32`]):
//! 1. The operands' magnitudes, by their bits: `n` the smaller and `d` the larger (compared as integers, which orders
//!    finite non-negative floats), `swap` where `|y| > |x|`; each as a 24-bit mantissa with its leading bit set and an
//!    exponent, a subnormal normalised ([`unpack`]). `t = n/d ∈ [0, 1]`.
//! 2. The breakpoint `k/8` nearest `t`, `k ∈ 0 … 8`, by eight exact comparisons of `n` with `(2j − 1)/16·d`, so
//!    `|t − k/8| ≤ 1/16`.
//! 3. The reduced argument `r = (t − k/8)/(1 + t·k/8) = (8n − kd)/(8d + kn)`, its numerator and denominator exact
//!    integers in a 32-bit word, divided bit by bit to a 64-bit quotient ([`divide`]); at `k = 0`, `r = t = n/d` with
//!    its exponent, so the quotient keeps 64 significant bits however small `t` is. `|r| ≤ 1/16`.
//! 4. `atan r = r·(1 − r²·G(r²))`, `G(z) = 1/3 − z/5 + z²/7 − z³/9 + z⁴/11`, in fixed point; the series' next term is
//!    below `2⁻⁴⁴` relative. `atan t = atan(k/8) + atan r`, `atan(k/8)` the generated `ATAN_EIGHTHS`.
//! 5. The octant: `π/2 − atan t` where `swap`, then `π −` that where `x`'s sign bit is set, in Q2.62 fixed point, from
//!    the generated `HALF_PI` and `PI`. Where neither applies and `k = 0` the result is `atan r` itself, kept with its
//!    exponent, so a small angle keeps its relative precision.
//! 6. One rounding to f32, to nearest with ties to even, subnormals included ([`round`]), then `y`'s sign bit.
//!
//! **Special values**, IEEE's: `atan2(±0, x)` is `±0` for `x` positive or `+0` and `±π` for `x` negative or `−0`;
//! `atan2(y, ±0)` is `±π/2` for `y ≠ 0`; an infinite operand gives the limit (`±π/2`, `±0` or `±π`, and `±π/4` or
//! `±3π/4` for two); a NaN operand gives NaN. The kernel never forms `±∞` (`crate::real`'s note), and its callers select
//! their poles (R-437); these hold so that the function is total.

use crate::generated::arctan::{ATAN_EIGHTHS, ATAN_SERIES, HALF_PI, PI};

/// `t` where `c` holds, else `f`: both operands are computed before the choice, so it is a select, not a branch.
#[inline]
fn select<T: Copy>(c: bool, t: T, f: T) -> T {
    if c {
        t
    } else {
        f
    }
}

/// 1 where `c` holds, else 0.
#[inline]
fn bit(c: bool) -> u32 {
    select(c, 1, 0)
}

/// A 64-bit number as two words, `hi·2³² + lo` (WGSL has no u64).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Words {
    /// The high word.
    pub hi: u32,
    /// The low word.
    pub lo: u32,
}

impl Words {
    /// The generated `[hi, lo]`.
    #[inline]
    const fn of(w: [u32; 2]) -> Words {
        Words { hi: w[0], lo: w[1] }
    }

    /// Whether it is 0.
    #[inline]
    fn is_zero(self) -> bool {
        (self.hi | self.lo) == 0
    }
}

/// `a·b`, all 64 bits, from the four products of the operands' 16-bit halves, each exact in a word.
#[inline]
pub fn mul(a: u32, b: u32) -> Words {
    let (a1, a0) = (a >> 16, a & 0xFFFF);
    let (b1, b0) = (b >> 16, b & 0xFFFF);
    let (p00, p01, p10, p11) = (a0 * b0, a0 * b1, a1 * b0, a1 * b1);
    let mid = (p00 >> 16) + (p01 & 0xFFFF) + (p10 & 0xFFFF);
    Words {
        hi: p11 + (p01 >> 16) + (p10 >> 16) + (mid >> 16),
        lo: (p00 & 0xFFFF) | (mid << 16),
    }
}

/// The high word of `a·b`: `⌊a·b/2³²⌋`.
#[inline]
pub fn mul_hi(a: u32, b: u32) -> u32 {
    mul(a, b).hi
}

/// `a + b`, modulo 2⁶⁴.
#[inline]
pub fn add(a: Words, b: Words) -> Words {
    let lo = a.lo.wrapping_add(b.lo);
    let carry = bit(lo < a.lo);
    Words {
        hi: a.hi.wrapping_add(b.hi).wrapping_add(carry),
        lo,
    }
}

/// `a − b`, modulo 2⁶⁴.
#[inline]
pub fn sub(a: Words, b: Words) -> Words {
    let borrow = bit(a.lo < b.lo);
    Words {
        hi: a.hi.wrapping_sub(b.hi).wrapping_sub(borrow),
        lo: a.lo.wrapping_sub(b.lo),
    }
}

/// `⌊a/2ˢ⌋` for any `s`, 0 from 64 on. No shift reaches 32 bits, which Rust and WGSL leave undefined or mask.
#[inline]
pub fn shr(a: Words, s: u32) -> Words {
    let k = s & 31;
    let spill = select(k == 0, 0, a.hi << ((32 - k) & 31));
    let near = Words {
        hi: a.hi >> k,
        lo: (a.lo >> k) | spill,
    };
    let far = Words {
        hi: 0,
        lo: a.hi >> k,
    };
    select(s >= 64, Words { hi: 0, lo: 0 }, select(s >= 32, far, near))
}

/// `a·2ˢ` modulo 2⁶⁴ for any `s`, 0 from 64 on.
#[inline]
pub fn shl(a: Words, s: u32) -> Words {
    let k = s & 31;
    let spill = select(k == 0, 0, a.lo >> ((32 - k) & 31));
    let near = Words {
        hi: (a.hi << k) | spill,
        lo: a.lo << k,
    };
    let far = Words {
        hi: a.lo << k,
        lo: 0,
    };
    select(s >= 64, Words { hi: 0, lo: 0 }, select(s >= 32, far, near))
}

/// The leading zero bits of a word, 32 for 0.
#[inline]
fn zeros32(w: u32) -> u32 {
    select(w == 0, 32, w.leading_zeros())
}

/// The leading zero bits of `a`, 64 for 0.
#[inline]
pub fn zeros(a: Words) -> u32 {
    select(a.hi == 0, 32 + zeros32(a.lo), zeros32(a.hi))
}

/// A non-negative float's bits `b` (the sign bit clear) as `(m, e)`, its value `m·2ᵉ`, `m ∈ [2²³, 2²⁴)`: a normal's
/// mantissa with its leading bit, a subnormal's shifted up to it. `+∞` reads as `2¹²⁸`; 0 gives `m = 0`.
#[inline]
pub fn unpack(b: u32) -> (u32, i32) {
    let field = (b >> 23) as i32;
    let fraction = b & 0x007F_FFFF;
    let normal = field != 0;
    let m = select(normal, fraction | 0x0080_0000, fraction);
    let e = select(normal, field - 150, -149);
    let up = select(m == 0, 0, zeros32(m) - 8);
    (m << up, e - up as i32)
}

/// `⌊num·2⁶⁴/den⌋` for `num < den`, by restoring division, one bit a step. The remainder stays below `den < 2³²`; its
/// doubling may carry out of the word, and then it is past `den`.
#[inline]
pub fn divide(num: u32, den: u32) -> Words {
    let mut rem = num;
    let mut q = Words { hi: 0, lo: 0 };
    for _ in 0..64 {
        let carry = rem >> 31 == 1;
        let doubled = rem << 1;
        let set = carry | (doubled >= den);
        rem = select(set, doubled.wrapping_sub(den), doubled);
        q = Words {
            hi: (q.hi << 1) | (q.lo >> 31),
            lo: (q.lo << 1) | bit(set),
        };
    }
    q
}

/// `atan r` for `r = (M/2⁶⁴)·2ᵉ`, `M` normalised (its top bit set) or 0, `|r| ≤ 1/16` (so `e ≤ −3`): `A` with
/// `atan r = (A/2⁶⁴)·2ᵉ`, `A = M − M·r²·G(r²)` (the module's step 4). `z = r²` is `(M_hi²/2⁶⁴)·2²ᵉ`, read in Q0.32 for
/// `G`'s Horner steps, each `1/(2j + 3) − z·(…)`, positive since `z ≤ 2⁻⁸`; `r²·G` keeps 31 significant bits, and the
/// correction `M·r²·G` is below `2⁻⁸·M`.
#[inline]
pub fn atan_reduced(m: Words, e: i32) -> Words {
    let z = mul_hi(m.hi, m.hi);
    let s = (-2 * e) as u32;
    let zq = select(s >= 32, 0, z >> (s & 31));
    let mut g = ATAN_SERIES[4];
    g = ATAN_SERIES[3] - mul_hi(zq, g);
    g = ATAN_SERIES[2] - mul_hi(zq, g);
    g = ATAN_SERIES[1] - mul_hi(zq, g);
    g = ATAN_SERIES[0] - mul_hi(zq, g);
    sub(m, shr(mul(m.hi, mul_hi(z, g)), s))
}

/// `(mant/2⁶⁴)·2^ex`, `mant` normalised, as f32 bits, rounded to nearest with ties to even: a normal's `24` bits are
/// the mantissa's top ones; below the least normal the mantissa is shifted right a further `1 − biased` bits, a
/// subnormal, 0 from 25 bits on. The bits below the kept ones give the rounding bit and the sticky bit.
#[inline]
pub fn round(mant: Words, ex: i32) -> u32 {
    let biased = ex + 126;
    let normal = biased >= 1;
    let extra = select(normal, 0, select(biased <= -24, 25, (1 - biased) as u32));
    let keep_at = 8 + extra;
    let kept = select(keep_at >= 32, 0, mant.hi >> (keep_at & 31));
    let round_at = 7 + extra;
    let round_bit = select(round_at >= 32, 0, (mant.hi >> (round_at & 31)) & 1);
    let below = select(
        round_at >= 32,
        u32::MAX,
        (1u32 << (round_at & 31)).wrapping_sub(1),
    );
    let sticky = ((mant.hi & below) | mant.lo) != 0;
    let up = (round_bit == 1) & (sticky | (kept & 1 == 1));
    let base = (select(normal, (biased - 1) as u32, 0) << 23) + kept;
    base + bit(up)
}

/// Whether `t ≥ (2j − 1)/16` (step 2), `n_al = n·2^(E+4)` against `d`'s mantissa `dm`, where `near`.
#[inline]
fn past(near: bool, n_al: u32, dm: u32, j: u32) -> bool {
    near & (n_al >= (2 * j - 1) * dm)
}

/// `atan2(y, x)` at f32, the kernel's own, in integer arithmetic (the module's note and its steps).
#[inline]
pub fn atan2_f32(y: f32, x: f32) -> f32 {
    let (yb, xb) = (y.to_bits(), x.to_bits());
    let (ay, ax) = (yb & 0x7FFF_FFFF, xb & 0x7FFF_FFFF);
    let nan = (ay > 0x7F80_0000) | (ax > 0x7F80_0000);
    // Step 1.
    let swap = ay > ax;
    let (nb, db) = (select(swap, ax, ay), select(swap, ay, ax));
    let ((nm, en), (dm, ed)) = (unpack(nb), unpack(db));
    let angle_zero = (nb == 0) | ((db == 0x7F80_0000) & (nb != 0x7F80_0000));
    let big_e = en - ed;
    // Step 2: `t ≥ (2j − 1)/16` for `j = 1 … 8`, read as `n·2^(E+4) ≥ (2j − 1)·d`, exact in a word since `t ≤ 1`;
    // below `E = −4`, `t < 1/16`.
    let near = (big_e >= -4) & !angle_zero;
    let n_al = nm << select(near, (big_e + 4) as u32, 0);
    let over = [
        past(near, n_al, dm, 1),
        past(near, n_al, dm, 2),
        past(near, n_al, dm, 3),
        past(near, n_al, dm, 4),
        past(near, n_al, dm, 5),
        past(near, n_al, dm, 6),
        past(near, n_al, dm, 7),
        past(near, n_al, dm, 8),
    ];
    let k = bit(over[0])
        + bit(over[1])
        + bit(over[2])
        + bit(over[3])
        + bit(over[4])
        + bit(over[5])
        + bit(over[6])
        + bit(over[7]);
    let mut eighth = Words::of(ATAN_EIGHTHS[0]);
    eighth = select(over[0], Words::of(ATAN_EIGHTHS[1]), eighth);
    eighth = select(over[1], Words::of(ATAN_EIGHTHS[2]), eighth);
    eighth = select(over[2], Words::of(ATAN_EIGHTHS[3]), eighth);
    eighth = select(over[3], Words::of(ATAN_EIGHTHS[4]), eighth);
    eighth = select(over[4], Words::of(ATAN_EIGHTHS[5]), eighth);
    eighth = select(over[5], Words::of(ATAN_EIGHTHS[6]), eighth);
    eighth = select(over[6], Words::of(ATAN_EIGHTHS[7]), eighth);
    eighth = select(over[7], Words::of(ATAN_EIGHTHS[8]), eighth);
    // Step 3. At `k = 0`, `r = n/d`, `d` doubled where `n ≥ d` so the quotient is below 1 and keeps its top bit set.
    let doubled = nm >= dm;
    let den_t = select(doubled, dm << 1, dm);
    let e_t = big_e + bit(doubled) as i32;
    let d_al = dm << 4;
    let diff = (n_al << 3).wrapping_sub(k * d_al);
    let neg = diff >> 31 == 1;
    let num_r = select(neg, 0u32.wrapping_sub(diff), diff);
    let den_r = (d_al << 3) + k * n_al;
    let at_zero = k == 0;
    let q = divide(select(at_zero, nm, num_r), select(at_zero, den_t, den_r));
    let lead = select(at_zero, 0, zeros(q));
    let e = select(at_zero, e_t, -(lead as i32));
    // Step 4.
    let a = atan_reduced(shl(q, lead), e);
    let a = select(angle_zero, Words { hi: 0, lo: 0 }, a);
    let r_q64 = shr(a, (-e) as u32);
    let t_q64 = select(neg, sub(eighth, r_q64), add(eighth, r_q64));
    // Step 5, in Q2.62.
    let t_q62 = select(at_zero, shr(a, (2 - e) as u32), shr(t_q64, 2));
    let turned = select(swap, sub(Words::of(HALF_PI), t_q62), t_q62);
    let x_neg = xb >> 31 == 1;
    let angle = select(x_neg, sub(Words::of(PI), turned), turned);
    let small = at_zero & !swap & !x_neg;
    let (mant, ex) = select(small, (a, e), (angle, 2));
    // Step 6.
    let lead = zeros(mant);
    let bits = select(mant.is_zero(), 0, round(shl(mant, lead), ex - lead as i32));
    f32::from_bits(select(nan, 0x7FC0_0000, bits | (yb & 0x8000_0000)))
}
