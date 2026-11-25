//! The circle's emitter: writes the kernel's `generated/circle.rs`, the numbers of the kernel's own f32 reduction of the
//! decode's `β` modulo `2π` (`kernel::circle`), which the kernel reads in integer arithmetic.
//!
//! - `INV_TWO_PI`: eight words, the bits of `1/2π` from weight `2³¹` down, word 0 zero (`1/2π < 1`) and words 1 to 7
//!   `⌊2²²⁴/2π⌋`'s, most significant first.
//! - `PI_HAT_BITS`: the bits of `π̂`, the f32 nearest `π`: `π·2²²` rounded to nearest, at the exponent of `[2, 4)`.
//! - `SIN_SERIES` and `COS_SERIES`: `1/n!` in Q0.64, rounded to nearest, for odd `n` from 3 to 13 and even `n` from 2 to
//!   14, the Taylor coefficients of the kernel's own f32 sine and cosine (`kernel::trig`), as two words each.
//!
//! **Exact, and the same on every host.** `π` is summed here in integer arithmetic, never read from a float: Machin's
//! `π = 16·atan(1/5) − 4·atan(1/239)`, each `atan(1/x) = Σₙ (−1)ⁿ/((2n+1)·x^(2n+1))` summed in units of
//! `2^-pi_bits()` with words of 32 bits ([`machin_pi`]). Each of the at most 250 truncated terms errs by under a unit
//! and the dropped tail is below one, so `π` is low by under a few hundred units of `2⁻³²⁰`. `⌊2²²⁴/2π⌋` is then an
//! exact long division ([`inv_two_pi`]); its floor could move only were `2²²⁴/2π` within `2⁻⁸⁰` of an integer.

use std::fmt::Write as _;
use std::path::PathBuf;

use crate::gen::Generated;
use crate::schema::{Entry, Word};

/// Where the circle's numbers are written, relative to the workspace root.
pub const CIRCLE_PATH: &str = "crates/kernel/src/generated/circle.rs";

/// The emitter [`crate::gen::EMITTERS`] runs: the one file.
pub fn emit(_words: &[Word], _entries: &[Entry]) -> Vec<Generated> {
    vec![Generated {
        path: PathBuf::from(CIRCLE_PATH),
        contents: circle_file(),
    }]
}

/// Interface data, not a register constant (REQ-SYS-082): the fraction bits `π` is summed in, past the 224 the
/// table keeps by a margin that leaves its floor exact.
pub fn pi_bits() -> u32 {
    320
}

/// Interface data (REQ-SYS-082): the words of `INV_TWO_PI`, so the kernel reads `1/2π` from weight `2³¹` to `2⁻²²⁴`.
pub fn table_words() -> usize {
    8
}

/// A non-negative integer as 32-bit words, least significant first, in [`Big::len`] words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Big(Vec<u32>);

impl Big {
    /// The words every number here has: 2⁵⁷⁶ covers `2^(224 + pi_bits())` and its partial remainders.
    fn len() -> usize {
        18
    }

    /// 0.
    pub fn zero() -> Big {
        Big(vec![0; Big::len()])
    }

    /// `2ᵏ`.
    pub fn pow2(k: u32) -> Big {
        let mut b = Big::zero();
        b.0[(k / 32) as usize] = 1 << (k % 32);
        b
    }

    /// Whether it is 0.
    pub fn is_zero(&self) -> bool {
        self.0.iter().all(|&w| w == 0)
    }

    /// `⌊self/d⌋`.
    pub fn div_small(&self, d: u32) -> Big {
        let mut out = Big::zero();
        let mut rem = 0u64;
        for i in (0..Big::len()).rev() {
            let cur = (rem << 32) | u64::from(self.0[i]);
            out.0[i] = (cur / u64::from(d)) as u32;
            rem = cur % u64::from(d);
        }
        out
    }

    /// `self·m`, which must fit.
    pub fn mul_small(&self, m: u32) -> Big {
        let mut out = Big::zero();
        let mut carry = 0u64;
        for i in 0..Big::len() {
            let cur = u64::from(self.0[i]) * u64::from(m) + carry;
            out.0[i] = cur as u32;
            carry = cur >> 32;
        }
        assert_eq!(carry, 0, "the product overflows");
        out
    }

    /// `self + b`, which must fit.
    pub fn add(&self, b: &Big) -> Big {
        let mut out = Big::zero();
        let mut carry = 0u64;
        for i in 0..Big::len() {
            let cur = u64::from(self.0[i]) + u64::from(b.0[i]) + carry;
            out.0[i] = cur as u32;
            carry = cur >> 32;
        }
        assert_eq!(carry, 0, "the sum overflows");
        out
    }

    /// `self − b`, for `b ≤ self`.
    pub fn sub(&self, b: &Big) -> Big {
        let mut out = Big::zero();
        let mut borrow = 0i64;
        for i in 0..Big::len() {
            let cur = i64::from(self.0[i]) - i64::from(b.0[i]) - borrow;
            borrow = i64::from(cur < 0);
            out.0[i] = (cur + (borrow << 32)) as u32;
        }
        assert_eq!(borrow, 0, "the difference is negative");
        out
    }

    /// `self ≥ b`.
    pub fn at_least(&self, b: &Big) -> bool {
        for i in (0..Big::len()).rev() {
            if self.0[i] != b.0[i] {
                return self.0[i] > b.0[i];
            }
        }
        true
    }

    /// `2·self`, which must fit.
    pub fn double(&self) -> Big {
        self.add(self)
    }

    /// Word `i`, least significant first.
    pub fn word(&self, i: usize) -> u32 {
        self.0[i]
    }
}

/// `atan(1/x)` in units of `2^-pi_bits()`: the alternating series, each term `⌊2^pi_bits()/x^(2n+1)⌋/(2n+1)`, its
/// power truncated from the last, until the power truncates to 0.
fn atan_inv(x: u32) -> Big {
    let (mut pos, mut neg) = (Big::zero(), Big::zero());
    let mut power = Big::pow2(pi_bits()).div_small(x);
    let mut n = 0u32;
    while !power.is_zero() {
        let term = power.div_small(2 * n + 1);
        if n.is_multiple_of(2) {
            pos = pos.add(&term);
        } else {
            neg = neg.add(&term);
        }
        power = power.div_small(x * x);
        n += 1;
    }
    pos.sub(&neg)
}

/// `π` in units of `2^-pi_bits()`, by Machin's formula (the module's note).
pub fn machin_pi() -> Big {
    atan_inv(5).mul_small(16).sub(&atan_inv(239).mul_small(4))
}

/// `⌊2²²⁴/2π⌋`, by restoring division of `2^(224 + pi_bits())` by `2π` in units of `2^-pi_bits()`, one bit a step.
pub fn inv_two_pi() -> Big {
    let den = machin_pi().double();
    let steps = 224 + pi_bits();
    let mut rem = Big::pow2(0);
    let mut q = Big::zero();
    for _ in 0..steps {
        rem = rem.double();
        q = q.double();
        if rem.at_least(&den) {
            rem = rem.sub(&den);
            q = q.add(&Big::pow2(0));
        }
    }
    q
}

/// `INV_TWO_PI`: word 0 zero, then `⌊2²²⁴/2π⌋`'s seven words, most significant first.
pub fn inv_two_pi_words() -> Vec<u32> {
    let q = inv_two_pi();
    let low = table_words() - 1;
    for i in low..Big::len() {
        assert_eq!(q.word(i), 0, "⌊2²²⁴/2π⌋ is wider than 224 bits");
    }
    std::iter::once(0)
        .chain((0..low).rev().map(|i| q.word(i)))
        .collect()
}

/// `PI_HAT_BITS`: `π̂`, the f32 nearest `π`, as bits: `π ∈ [2, 4)`, so the biased exponent is 128 and the 24-bit
/// mantissa `π·2²²` rounded to nearest (no tie: `π` is irrational), its leading bit dropped.
pub fn pi_hat_bits() -> u32 {
    let pi = machin_pi();
    let shift = pi_bits() - 22;
    let scaled = |k: u32| -> u64 {
        let mut v = 0u64;
        for b in (k..k + 26).rev() {
            v = (v << 1) | u64::from((pi.word((b / 32) as usize) >> (b % 32)) & 1);
        }
        v
    };
    let top = scaled(shift - 1);
    let mantissa = ((top + 1) >> 1) as u32;
    (128 << 23) | (mantissa & 0x007F_FFFF)
}

/// Interface data, not a register constant: the factorials the sine's and cosine's series read, `n` in `3, 5, …, 13` and
/// `2, 4, …, 14`, so the first dropped term is below `2⁻⁴⁰` of the result on `[0, π/4]`.
pub fn series_orders() -> ([u32; 6], [u32; 7]) {
    ([3, 5, 7, 9, 11, 13], [2, 4, 6, 8, 10, 12, 14])
}

/// `1/n!` in Q0.64, rounded to nearest, as `[hi, lo]`: `⌊(2⁶⁵/n! + 1)/2⌋`, in integer arithmetic.
pub fn inverse_factorial(n: u32) -> [u32; 2] {
    let f: u128 = (1..=u128::from(n)).product();
    let q = ((1u128 << 65) / f).div_ceil(2);
    assert!(q < 1 << 64, "1/{n}! does not fit Q0.64");
    [(q >> 32) as u32, q as u32]
}

/// `w` as a hex literal in groups of four digits.
fn hex(w: u32) -> String {
    format!("0x{:04X}_{:04X}", w >> 16, w & 0xFFFF)
}

/// The circle's file.
pub fn circle_file() -> String {
    let mut s = String::from(
        "//! Generated by `cargo xtask codegen` from `crates/ledger/src/gen/circle.rs`; do not edit.\n\
         //!\n\
         //! The numbers of the kernel's own f32 reduction of the decode's `β` modulo `2π` (`crate::circle`), each\n\
         //! computed by the generator in integer arithmetic, never by a float.\n\n\
         /// The bits of `1/2π` from weight `2³¹` down: word `k` holds weights `2^−(32k − 31)` to `2^−32k`, most\n\
         /// significant first; word 0 is 0 (`1/2π < 1`), words 1 to 7 are `⌊2²²⁴/2π⌋`'s.\n",
    );
    let _ = writeln!(s, "pub const INV_TWO_PI: [u32; {}] = [", table_words());
    for w in inv_two_pi_words() {
        let _ = writeln!(s, "    {},", hex(w));
    }
    s.push_str("];\n\n/// The bits of `π̂`, the f32 nearest `π`.\n");
    let _ = writeln!(s, "pub const PI_HAT_BITS: u32 = {};", hex(pi_hat_bits()));
    let (sines, cosines) = series_orders();
    for (name, orders, what) in [
        ("SIN_SERIES", &sines[..], "odd `n` from 3 to 13"),
        ("COS_SERIES", &cosines[..], "even `n` from 2 to 14"),
    ] {
        let _ = write!(
            s,
            "\n/// `1/n!` in Q0.64, rounded to nearest, `[hi, lo]`, for {what}: the kernel's own f32 sine's and cosine's\n\
             /// series (`crate::trig`).\npub const {name}: [[u32; 2]; {}] = [\n",
            orders.len()
        );
        for &n in orders {
            let [hi, lo] = inverse_factorial(n);
            let _ = writeln!(s, "    [{}, {}],", hex(hi), hex(lo));
        }
        s.push_str("];\n");
    }
    s
}
