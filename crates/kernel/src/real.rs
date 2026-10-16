//! `Real`, the float type the kernel is generic over (canonical_spec §1 item 2; lowering Part 2): f32 for the GPU build,
//! f64 for the CPU build, and no other instantiation (R-265). The payload's widths are a function of it, never
//! hardcoded to f32 (philosophy §7.1, §7.7; dd_simstate_payload §1).
//!
//! **The poles' `±∞`** ([`IeeeBits`]; R-436; dd_decoder §3.6, "Each producer selects its pole value"). WGSL lets an
//! implementation assume there are no infinities (gpu_determinism_note § "The discipline", rule 4), so a producer never
//! forms `±∞` by `1/0` or by arithmetic on `∞`, and never by an infinite literal: it selects the integer pattern's bits,
//! then bitcasts them ([`IeeeBits::or_pos_inf`], [`IeeeBits::or_neg_inf`]: `0x7F80_0000` and `0xFF80_0000` at f32,
//! `0x7FF0_0000_0000_0000` and `0xFFF0_0000_0000_0000` at f64), as R-271's packer writes `d_min`'s `+∞`. The select is on
//! the integer bits, so the value it discards is never read as a float, and no infinite float constant is left for a
//! shader compiler to fold. `self`'s bits are taken before the choice, so that both of its operands are computed and the
//! kernel's WGSL has a `select`, not a branch (gpu_determinism_note's discipline). A stored value is tested on its bits, an exponent field of all ones
//! ([`IeeeBits::non_finite_bits`]), as R-114's NaN guard is, never by `isinf` or a comparison with the largest finite
//! value. The payload's float types have them ([`crate::payload::PayloadReal`]'s bound): f32 and f64.

use core::fmt::Debug;

/// A float type the one kernel source is instantiated at: f32 on the GPU, f64 on the CPU (R-265).
pub trait Real: Copy + Default + Debug + PartialEq + PartialOrd + 'static {
    /// The name the ledger's precision row gives this type (dd_simstate_payload §1).
    const NAME: &'static str;
}

impl Real for f32 {
    const NAME: &'static str = "f32";
}

impl Real for f64 {
    const NAME: &'static str = "f64";
}

/// The IEEE 754 bit patterns a producer selects at a pole, and the stored value's bit test (R-436; the module's note).
pub trait IeeeBits: Real {
    /// `+∞` where `c` holds, else `self`: `+∞`'s integer pattern or `self`'s bits, selected, then one bitcast.
    fn or_pos_inf(self, c: bool) -> Self;

    /// `−∞` where `c` holds, else `self`, selected as [`IeeeBits::or_pos_inf`] selects `+∞`.
    fn or_neg_inf(self, c: bool) -> Self;

    /// Whether `self`'s exponent field is all ones, so that it is `±∞` or NaN: a test on the bits, made before any
    /// arithmetic on `self`.
    fn non_finite_bits(self) -> bool;

    /// `|self|`'s unbiased exponent field, read from the bits: `e` with `2^e ≤ |self| < 2^(e+1)` for a normal `self`,
    /// the field's least value (`−127` at f32, `−1023` at f64) for `±0` and subnormals, its greatest (128, 1024) for
    /// `±∞` and NaN.
    fn exponent(self) -> i32;

    /// `2^e`, formed from its bits, for `e` a normal exponent (`−126..=127` at f32, `−1022..=1023` at f64).
    fn pow2(e: i32) -> Self;

    /// `self·b`, rounded the same on every backend, whether it contracts or fuses or not. At f32, the GPU's precision,
    /// each operand is split at its bits into a high part, its fraction's low 12 bits cleared, and the exact remainder,
    /// each of at most 12 significant bits, so the four partial products are exact (where normal) and a backend that
    /// fuses one into the following add rounds the same sum; they are added in one fixed order,
    /// `hh + ((hl + lh) + ll)`, within an ulp or so of `self·b`. At f64, the CPU's alone, where Rust never contracts,
    /// it is `self·b`.
    fn product(self, b: Self) -> Self;
}

impl IeeeBits for f32 {
    #[inline]
    fn or_pos_inf(self, c: bool) -> f32 {
        let bits = self.to_bits();
        f32::from_bits(if c { 0x7F80_0000 } else { bits })
    }

    #[inline]
    fn or_neg_inf(self, c: bool) -> f32 {
        let bits = self.to_bits();
        f32::from_bits(if c { 0xFF80_0000 } else { bits })
    }

    #[inline]
    fn non_finite_bits(self) -> bool {
        self.to_bits() & 0x7F80_0000 == 0x7F80_0000
    }

    #[inline]
    fn exponent(self) -> i32 {
        ((self.to_bits() >> 23) & 0xFF) as i32 - 127
    }

    #[inline]
    fn pow2(e: i32) -> f32 {
        f32::from_bits(((e + 127) as u32) << 23)
    }

    #[inline]
    fn product(self, b: f32) -> f32 {
        let ah = f32::from_bits(self.to_bits() & 0xFFFF_F000);
        let bh = f32::from_bits(b.to_bits() & 0xFFFF_F000);
        let (al, bl) = (self - ah, b - bh);
        ah * bh + ((ah * bl + al * bh) + al * bl)
    }
}

impl IeeeBits for f64 {
    #[inline]
    fn or_pos_inf(self, c: bool) -> f64 {
        let bits = self.to_bits();
        f64::from_bits(if c { 0x7FF0_0000_0000_0000 } else { bits })
    }

    #[inline]
    fn or_neg_inf(self, c: bool) -> f64 {
        let bits = self.to_bits();
        f64::from_bits(if c { 0xFFF0_0000_0000_0000 } else { bits })
    }

    #[inline]
    fn non_finite_bits(self) -> bool {
        self.to_bits() & 0x7FF0_0000_0000_0000 == 0x7FF0_0000_0000_0000
    }

    #[inline]
    fn exponent(self) -> i32 {
        ((self.to_bits() >> 52) & 0x7FF) as i32 - 1023
    }

    #[inline]
    fn pow2(e: i32) -> f64 {
        f64::from_bits(((e + 1023) as u64) << 52)
    }

    #[inline]
    fn product(self, b: f64) -> f64 {
        self * b
    }
}
