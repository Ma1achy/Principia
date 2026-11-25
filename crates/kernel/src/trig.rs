//! The kernel's own f32 sine and cosine, [`sin_cos_f32`], in integer arithmetic: [`crate::Real::sine`],
//! [`crate::Real::cosine`] and [`crate::Real::sine_cosine`] at f32 on every target, the host and the GPU alike.
//!
//! **Why the kernel has its own.** WGSL bounds `sin` and `cos` only absolutely (`2⁻¹¹` inside `[−π, π]`), and the
//! decode's `‖ρ̃‖ = cos α` needs a relative bound where `α → π/2`, which a backend's builtin `cos` need not hold
//! (`crates/engine/tests/cos_half_pi_gpu.rs`). On the host,
//! each platform's `sinf` and `cosf` round differently, so a record built from them differs between machines.
//!
//! Don't rewrite this in floats, e.g. `sin(π/2 − α)` from a two-part `π/2` and a polynomial: a backend under fast-math
//! may reassociate `(α − hi) − lo` into `α − (hi + lo)`, which loses every bit of `cos α` near `π/2`, and `fma` rounds
//! differently on different backends (pitfalls §11). Integer operations are exact everywhere, so every backend gives
//! the CPU's bits and the CPU's sweeps (`crates/kernel/tests/trig.rs`) hold on the GPU.
//!
//! **The algorithm.**
//! 1. `|x| < 2⁻¹²`: `sin x = x`, `cos x = 1`, each the f32 nearest (`x³/6` and `x²/2` are below half an ulp); `±0`
//!    keeps its sign. NaN and `±∞` give NaN.
//! 2. The fraction of a turn `f = |x|/2π mod 1` in 96 bits, as [`crate::circle`] reduces: the 24-bit mantissa times 96
//!    bits of `1/2π` read from the stream `[0, INV_TWO_PI]`, one word longer than the circle's so that the small
//!    exponents down to `2⁻¹²` read their leading zeros from it.
//! 3. The octant `o`, `f`'s top three bits; the remainder `r` below them, or `1/8 − r` for an odd octant, so the reduced
//!    angle `θ = 2π·r` lies in `[0, π/4]` and `|x| = qπ/2 ± θ` with `q = ⌈o/2⌉ mod 4`. `θ` is normalised as a 64-bit
//!    mantissa and an exponent, so a small `θ` (an `x` near a multiple of `π/2`) keeps its relative precision.
//! 4. `sin θ = θ·(1 − z·S(z))` and `cos θ = 1 − z·C(z)`, `z = θ²`, by Horner's rule in Q0.64 on the generated `1/n!`
//!    (`SIN_SERIES` to `1/13!`, `COS_SERIES` to `1/14!`); every step is positive, since each term is below a sixth of
//!    the one before. The dropped terms are below `2⁻⁴⁰` of the result, the truncations a few units of `2⁻⁶⁴`.
//! 5. The quadrant's swap and signs, `x`'s sign on the sine, and one rounding of each to f32, to nearest with ties to
//!    even ([`crate::arctan::round`]).
//!
//! So each result is the f32 nearest a value within about `2⁻⁴⁰` relative of the exact one: at most about 0.5 ulp off.

use crate::arctan::{add, mul, mul_halves, round, shl, sub, unpack, zeros, Words};
use crate::generated::arctan::ATAN_EIGHTHS;
use crate::generated::circle::{COS_SERIES, INV_TWO_PI, SIN_SERIES};

/// `t` where `c` holds, else `f`, by a mask. Don't write it as `if c { t } else { f }`: the GPU build lowers that to a
/// branch, and this function's hundreds of them in a row overflowed rust-gpu's stack.
#[inline]
fn pick(c: bool, t: u32, f: u32) -> u32 {
    let m = 0u32.wrapping_sub(u32::from(c));
    (t & m) | (f & !m)
}

/// [`pick`] on two words.
#[inline]
fn pick_words(c: bool, t: Words, f: Words) -> Words {
    Words {
        hi: pick(c, t.hi, f.hi),
        lo: pick(c, t.lo, f.lo),
    }
}

/// The generated `[hi, lo]` as words.
#[inline]
fn of(w: [u32; 2]) -> Words {
    Words { hi: w[0], lo: w[1] }
}

/// Word `i ∈ 0 … 8` of the stream `[0, INV_TWO_PI]`, by selects, never an index a backend reads at run time.
#[inline]
fn table(i: u32) -> u32 {
    let mut w = 0;
    w = pick(i == 1, INV_TWO_PI[0], w);
    w = pick(i == 2, INV_TWO_PI[1], w);
    w = pick(i == 3, INV_TWO_PI[2], w);
    w = pick(i == 4, INV_TWO_PI[3], w);
    w = pick(i == 5, INV_TWO_PI[4], w);
    w = pick(i == 6, INV_TWO_PI[5], w);
    w = pick(i == 7, INV_TWO_PI[6], w);
    pick(i == 8, INV_TWO_PI[7], w)
}

/// The 32 bits of the stream from bit `t` on, `t ≤ 232`, counted from word 0's most significant bit: bit `t` has
/// weight `2^(63 − t)`.
#[inline]
fn window_word(t: u32) -> u32 {
    let (q, r) = (t >> 5, t & 31);
    let (a, b) = (table(q), table(q + 1));
    pick(r == 0, a, (a << r) | (b >> ((32 - r) & 31)))
}

/// `⌊a·b/2⁶⁴⌋` for `a`, `b` in two words each: the top 64 bits of the 128-bit product, exact. On the host one u128
/// product, the same bits as the GPU's [`mul_q64_words`].
#[cfg(not(target_arch = "spirv"))]
#[inline]
fn mul_q64(a: Words, b: Words) -> Words {
    let wide = |w: Words| (u128::from(w.hi) << 32) | u128::from(w.lo);
    let p = (wide(a) * wide(b)) >> 64;
    Words {
        hi: (p >> 32) as u32,
        lo: p as u32,
    }
}

/// `⌊a·b/2⁶⁴⌋` on the GPU: [`mul_q64_words`].
#[cfg(target_arch = "spirv")]
#[inline]
fn mul_q64(a: Words, b: Words) -> Words {
    mul_q64_words(a, b)
}

/// `⌊a·b/2⁶⁴⌋` from 32-bit words alone (WGSL has no u64): the GPU's `mul_q64`, built on the host too so
/// `crates/kernel/tests/trig.rs` holds it to the u128 product.
#[inline]
pub fn mul_q64_words(a: Words, b: Words) -> Words {
    let (hl, lh) = (mul_halves(a.hi, b.lo), mul_halves(a.lo, b.hi));
    let low = add(
        add(Words { hi: 0, lo: hl.lo }, Words { hi: 0, lo: lh.lo }),
        Words {
            hi: 0,
            lo: mul_halves(a.lo, b.lo).hi,
        },
    );
    let high = add(mul_halves(a.hi, b.hi), Words { hi: 0, lo: hl.hi });
    add(
        add(high, Words { hi: 0, lo: lh.hi }),
        Words { hi: 0, lo: low.hi },
    )
}

/// `c − z·g` in Q0.64, one Horner step.
#[inline]
fn step(c: [u32; 2], z: Words, g: Words) -> Words {
    sub(of(c), mul_q64(z, g))
}

/// `(sin x, cos x)` at f32, the kernel's own, in integer arithmetic (the module's note and its steps). Out of line on the
/// GPU, so each entry point holds its body once, not once per `sin` and `cos`.
#[cfg_attr(not(target_arch = "spirv"), inline)]
#[cfg_attr(target_arch = "spirv", inline(never))]
pub fn sin_cos_f32(x: f32) -> (f32, f32) {
    let b = x.to_bits();
    let a = b & 0x7FFF_FFFF;
    // Step 1.
    let special = a >= 0x7F80_0000;
    let tiny = a < 0x3980_0000;
    // Step 2. `|x| = m·2ᵉ`, `e ∈ [−35, 104]` past the tiny; where not, any normal stands in, its result unread.
    let (m, e) = unpack(pick(special | tiny, 0x3F80_0000, a));
    let t = (e + 64) as u32;
    let (w0, w1, w2) = (window_word(t), window_word(t + 32), window_word(t + 64));
    let (p2, p1) = (mul(m, w2), mul(m, w1));
    let f2 = p2.lo;
    let f1 = p2.hi.wrapping_add(p1.lo);
    let f0 = p1
        .hi
        .wrapping_add(mul(m, w0).lo)
        .wrapping_add(u32::from(f1 < p2.hi));
    // Step 3. An odd octant's remainder is `1/8 − r`: `0 − r` modulo 2⁹⁶, read in its low 93 bits.
    let octant = f0 >> 29;
    let odd = octant & 1 == 1;
    let n2 = 0u32.wrapping_sub(f2);
    let n01 = sub(
        sub(Words { hi: 0, lo: 0 }, Words { hi: f0, lo: f1 }),
        Words {
            hi: 0,
            lo: u32::from(f2 != 0),
        },
    );
    let g0 = pick(odd, n01.hi, f0) & 0x1FFF_FFFF;
    let g1 = pick(odd, n01.lo, f1);
    let g2 = pick(odd, n2, f2);
    let flat = (g0 | g1 | g2) == 0;
    // The remainder's top 64 bits, normalised, as `crate::circle` takes them: `g·2^lead/2³²` to the bits dropped.
    let high = Words { hi: g0, lo: g1 };
    let lead_high = zeros(high);
    let lead = pick(
        lead_high < 64,
        lead_high,
        64 + (zeros(Words { hi: g2, lo: 0 }) & 31),
    );
    let spill = pick_words(
        lead >= 32,
        shl(Words { hi: 0, lo: g2 }, lead.wrapping_sub(32) & 63),
        Words {
            hi: 0,
            lo: pick(lead == 0, 0, g2 >> (32u32.wrapping_sub(lead) & 31)),
        },
    );
    let top = shl(high, lead);
    let top = Words {
        hi: top.hi | spill.hi,
        lo: top.lo | spill.lo,
    };
    // `θ = 2π·g/2⁹⁶ = (π/4)·(top/2⁶⁴)·2^(3 − lead)`, then normalised: `θ = (p/2⁶⁴)·2^ex`, `ex ≤ 0`.
    let product = mul_q64(top, of(ATAN_EIGHTHS[8]));
    let shift = zeros(product);
    let p = shl(product, shift);
    let ex = 3 - lead as i32 - shift as i32;
    // Step 4. `θ` in Q0.64, and `z = θ²`.
    let theta = crate::arctan::shr(p, (-ex) as u32);
    let z = mul_q64(theta, theta);
    let mut s = of(SIN_SERIES[5]);
    s = step(SIN_SERIES[4], z, s);
    s = step(SIN_SERIES[3], z, s);
    s = step(SIN_SERIES[2], z, s);
    s = step(SIN_SERIES[1], z, s);
    s = step(SIN_SERIES[0], z, s);
    let sine = sub(p, mul_q64(p, mul_q64(z, s)));
    let up = zeros(sine);
    let sin_theta = pick(flat, 0, round(shl(sine, up), ex - up as i32));
    let mut c = of(COS_SERIES[6]);
    c = step(COS_SERIES[5], z, c);
    c = step(COS_SERIES[4], z, c);
    c = step(COS_SERIES[3], z, c);
    c = step(COS_SERIES[2], z, c);
    c = step(COS_SERIES[1], z, c);
    c = step(COS_SERIES[0], z, c);
    let u = mul_q64(z, c);
    let one = (u.hi | u.lo) == 0;
    let cos_theta = pick(
        one | flat,
        0x3F80_0000,
        round(sub(Words { hi: 0, lo: 0 }, u), 0),
    );
    // Step 5. `|x| = qπ/2 + φ`, `φ = ±θ`, negative in an odd octant.
    let q = ((octant + 1) >> 1) & 3;
    let swap = q & 1 == 1;
    let sin_mag = pick(swap, cos_theta, sin_theta);
    let cos_mag = pick(swap, sin_theta, cos_theta);
    let sin_neg = (swap & (q == 3)) | (!swap & ((q == 2) != odd));
    let cos_neg = (swap & ((q == 1) != odd)) | (!swap & (q == 2));
    let sin_bits = sin_mag | ((u32::from(sin_neg) << 31) ^ (b & 0x8000_0000));
    let cos_bits = cos_mag | (u32::from(cos_neg) << 31);
    let sin_bits = pick(tiny, b, pick(special, 0x7FC0_0000, sin_bits));
    let cos_bits = pick(tiny, 0x3F80_0000, pick(special, 0x7FC0_0000, cos_bits));
    (f32::from_bits(sin_bits), f32::from_bits(cos_bits))
}
