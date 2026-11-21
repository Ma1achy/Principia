//! The decode's angle on the circle at f32, the kernel's own reduction modulo `2π` ([`on_circle_f32`]), in integer
//! arithmetic, so the backend's `cos` and `sin` are only ever asked for an argument in `[−π̂, π̂]`, `π̂` the f32 nearest
//! `π`.
//!
//! **Why the kernel reduces.** The periodic `β` (dd_decoder §3.2, `periodic_beta`) is `z_β + π/2` with `z_β` any real
//! number, stored unreduced, and "read modulo `2π` by the decode's `cos β` and `sin β`". That reading assumes the
//! backend's `cos` and `sin` reduce their own argument. The CPU's do, exactly. WGSL bounds them only inside `[−π, π]`,
//! and outside it a GPU need not reduce at all: at `z_β = 10³⁰` lavapipe and Metal both decoded `I ≈ 5.7·10⁻¹⁵`,
//! `‖ρ̃‖²` alone (`qa_m206_gpu_saturated_controls`, sample 410), so `λ̃ = sin α·(cos β, sin β)` was `(0, 0)`: their
//! `cos β` and `sin β` both about 0, inferred from `I`, the two not read back separately.
//! While `β` came from a bounded link onto `[0, π]` this could not happen; the mirror step's removal let `β` run over
//! the whole line (pitfalls §11: when a fold or mirror is removed, re-run every cross-backend check it sat downstream
//! of).
//!
//! **Integer arithmetic, so every backend gives the CPU's bits**, as [`crate::arctan`]'s `atan2` does, and from its
//! helpers: the operand's bits, 32-bit words, shifts, comparisons, selects and exact products of 16-bit halves. No float
//! operation a backend may round differently, fuse or fold, and no float constant.
//!
//! **The algorithm** (Payne and Hanek's): `|β| = M·2ᵉ`, `M` a 24-bit integer. `|β|/2π = M·2ᵉ·C`, `C = 1/2π`; the bits of
//! `C` above `2⁻ᵉ` multiply `M` to an integer, which the circle ignores, so the fraction of a turn is the low 96 bits
//! of `M` times the 96 bits of `C` from `2^−(e+1)` on (`INV_TWO_PI`), exact but for the dropped tail of `C`, below
//! `M·2⁻⁹⁶ < 2⁻⁷²` of a turn. A fraction of a half turn or more is taken as its negative, `f − 1`, so the angle
//! `2π·f` lies in `[−π, π]`; times `π/4` (the generated `ATAN_EIGHTHS[8]`, Q0.64) in two words; one rounding to f32,
//! to nearest with ties to even ([`crate::arctan::round`]); then `β`'s sign. Measured on every f32 past `π̂`
//! (`crates/kernel/tests/circle.rs`, `circle_is_beta_mod_two_pi_exhaustive`): the closest any comes to a multiple of `2π`
//! is `2⁻²⁹·⁸⁶` of a turn (`β = 3.0916716·10²⁹`), far above the `2⁻⁷²` dropped, and every result is the f32 nearest the
//! exact `β mod 2π`. In the argument, a result could miss that nearest f32 only where the exact value lay within about
//! `2⁻⁴⁰` relative of a midpoint between two f32s; none does.
//!
//! **What passes unchanged.** `|β| ≤ π̂`, already on the circle's one period, is returned as it is, bit for bit, so
//! every `β` a bounded link gave and the golden IC's `π/2` keep their bits; NaN and `±∞` are returned as they are.

use crate::arctan::{add, mul, mul_hi, round, shl, sub, unpack, zeros, Words};
use crate::generated::arctan::ATAN_EIGHTHS;

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

/// The bits of `1/2π` from weight `2³¹` down (word 0 zero, words 1 to 7 `⌊2²²⁴/2π⌋`'s) and of `π̂`, the f32 nearest `π`:
/// the largest magnitude returned, and passed, unchanged. Both are generated (`crates/ledger/src/gen/circle.rs`), in
/// integer arithmetic from Machin's formula; `crates/kernel/tests/circle.rs` recomputes the table independently.
pub use crate::generated::circle::{INV_TWO_PI, PI_HAT_BITS};

/// `INV_TWO_PI[i]` for `i ∈ 0 … 7`, by selects, never an index a backend reads at run time.
#[inline]
fn table(i: u32) -> u32 {
    let mut w = INV_TWO_PI[0];
    w = select(i == 1, INV_TWO_PI[1], w);
    w = select(i == 2, INV_TWO_PI[2], w);
    w = select(i == 3, INV_TWO_PI[3], w);
    w = select(i == 4, INV_TWO_PI[4], w);
    w = select(i == 5, INV_TWO_PI[5], w);
    w = select(i == 6, INV_TWO_PI[6], w);
    select(i == 7, INV_TWO_PI[7], w)
}

/// The 32 bits of [`INV_TWO_PI`]'s bit stream from bit `t` on, `t < 224`, counted from word 0's most significant bit.
#[inline]
fn window_word(t: u32) -> u32 {
    let (q, r) = (t >> 5, t & 31);
    let (a, b) = (table(q), table(q + 1));
    select(r == 0, a, (a << r) | (b >> ((32 - r) & 31)))
}

/// `a + b` modulo 2⁶⁴ and its carry out.
#[inline]
fn add_carry(a: Words, b: Words) -> (Words, u32) {
    let s = add(a, b);
    (s, bit((s.hi < a.hi) | ((s.hi == a.hi) & (s.lo < a.lo))))
}

/// `β mod 2π` in `[−π̂, π̂]` at f32, the kernel's own, in integer arithmetic (the module's note); `β` itself where
/// `|β| ≤ π̂`, NaN or infinite.
#[inline]
pub fn on_circle_f32(beta: f32) -> f32 {
    let b = beta.to_bits();
    let a = b & 0x7FFF_FFFF;
    let keep = (a <= PI_HAT_BITS) | (a >= 0x7F80_0000);
    // `|β| = m·2ᵉ`, `m ∈ [2²³, 2²⁴)`, `e ∈ [−22, 104]` past `π̂`. Where `keep`, any normal stands in, its result unread.
    let (m, e) = unpack(select(keep, 0x4080_0000, a));
    // The 96 bits of `1/2π` from weight `2^−(e+1)` on: the stream's bit `e + 32`, at most 136, so word 7 the last read.
    let t = (e + 32) as u32;
    let (w0, w1, w2) = (window_word(t), window_word(t + 32), window_word(t + 64));
    // `f = m·w mod 2⁹⁶`, the fraction of a turn in units of `2⁻⁹⁶`, as three words `f0·2⁶⁴ + f1·2³² + f2`.
    let (p2, p1) = (mul(m, w2), mul(m, w1));
    let f2 = p2.lo;
    let f1 = p2.hi.wrapping_add(p1.lo);
    let f0 = p1
        .hi
        .wrapping_add(mul(m, w0).lo)
        .wrapping_add(bit(f1 < p2.hi));
    // A half turn or more is `f − 1`: its magnitude, `2⁹⁶ − f`, the three words' two's complement.
    let negative = f0 >> 31 == 1;
    // `0 − f`: the low word `0 − f2`, its borrow out of the top two words.
    let n2 = 0u32.wrapping_sub(f2);
    let n01 = sub(
        sub(Words { hi: 0, lo: 0 }, Words { hi: f0, lo: f1 }),
        Words {
            hi: 0,
            lo: bit(f2 != 0),
        },
    );
    let (n0, n1) = (n01.hi, n01.lo);
    let (g0, g1, g2) = (
        select(negative, n0, f0),
        select(negative, n1, f1),
        select(negative, n2, f2),
    );
    // The magnitude's top 64 bits, normalised: `g = (top/2⁶⁴)·2^(96 − lead)` to the 32 bits dropped.
    let high = Words { hi: g0, lo: g1 };
    let lead_high = zeros(high);
    // Below 64 leading zeros in the top two words, theirs; else 64 and `g2`'s (`zeros` of `g2` above 32 zeros gives
    // 32 or more only for `g2 = 0`, where `g = 0` and the result is 0 whatever `lead` is).
    let lead = select(
        lead_high < 64,
        lead_high,
        64 + (zeros(Words { hi: g2, lo: 0 }) & 31),
    );
    let spill = select(
        lead >= 32,
        shl(Words { hi: 0, lo: g2 }, lead.wrapping_sub(32) & 63),
        Words {
            hi: 0,
            lo: select(lead == 0, 0, g2 >> (32u32.wrapping_sub(lead) & 31)),
        },
    );
    let top = shl(high, lead);
    let top = Words {
        hi: top.hi | spill.hi,
        lo: top.lo | spill.lo,
    };
    // The angle `2π·g/2⁹⁶ = (π/4)·(top/2⁶⁴)·2^(3 − lead)`: the two words' product's top 64 bits, exact.
    let quarter = Words {
        hi: ATAN_EIGHTHS[8][0],
        lo: ATAN_EIGHTHS[8][1],
    };
    let (s1, k1) = add_carry(mul(top.hi, quarter.lo), mul(top.lo, quarter.hi));
    let (s2, k2) = add_carry(
        s1,
        Words {
            hi: 0,
            lo: mul_hi(top.lo, quarter.lo),
        },
    );
    let product = add(
        mul(top.hi, quarter.hi),
        Words {
            hi: k1 + k2,
            lo: s2.hi,
        },
    );
    let shift = zeros(product);
    let zero = (g0 | g1 | g2) == 0;
    let bits = select(
        zero,
        0,
        round(shl(product, shift), 3 - lead as i32 - shift as i32),
    );
    let sign = (b & 0x8000_0000) ^ select(negative, 0x8000_0000, 0);
    f32::from_bits(select(keep, b, bits | sign))
}
