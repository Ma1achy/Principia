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
//!
//! **What `Real` gives the physics layer** (integrator_contract Part 2c, "Rules the new kernel must hold by
//! construction"): the four operations and negation, [`RealOps::fma`], [`RealOps::square_root`] and the comparisons
//! of `PartialOrd`, and nothing else; its one other function, [`Real::arctan2`], asks for `num_traits::Float` too, so
//! code bounded by `Real` alone cannot reach it. It carries no numeric constant, no epsilon, no largest value, no infinity: rule 1
//! forbids a precision-dependent constant in a control-flow decision, so code bounded by `Real` alone, as
//! [`crate::physics`] is, has none to reach. A tolerance that must scale with precision is supplied where the value is
//! used, never where a branch is taken. `NAME` is the precision row's label for the ledger, not a number. The two
//! functions are named apart from `num_traits::Float`'s `mul_add` and `sqrt`, which the links' bound
//! ([`crate::generated::links::LinkReal`]) also carries, so that a call on a type bounded by both is not ambiguous.

use core::fmt::Debug;
use core::ops::{Add, Div, Mul, Neg, Sub};

use spirv_std::num_traits::Float;

/// A float type the one kernel source is instantiated at: f32 on the GPU, f64 on the CPU (R-265).
pub trait Real:
    Copy
    + Default
    + Debug
    + PartialEq
    + PartialOrd
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
    + Neg<Output = Self>
    + RealOps
    + 'static
{
    /// The name the ledger's precision row gives this type (dd_simstate_payload §1).
    const NAME: &'static str;

    /// `atan2(self, x)`, the angle of the point `(x, self)`, in `[−π, π]`, for a type that is also a `Float`: libm's
    /// by default, so the f64 path is libm's bit for bit, and at f32, the GPU's precision, the kernel's own,
    /// [`crate::arctan::atan2_f32`], in integer arithmetic, so every backend gives the CPU's bits (RQ-628, decided by the
    /// orchestrator per R-369, option (a)). The encode's angles and the shape sphere decode's `β` call it, so the
    /// precision chooses it through `crate::generated::links::LinkReal`, whose bound holds `Real`.
    #[inline]
    fn arctan2(self, x: Self) -> Self
    where
        Self: Float,
    {
        Float::atan2(self, x)
    }
}

impl Real for f32 {
    const NAME: &'static str = "f32";

    #[inline]
    fn arctan2(self, x: f32) -> f32 {
        crate::arctan::atan2_f32(self, x)
    }
}

impl Real for f64 {
    const NAME: &'static str = "f64";
}

/// The two functions [`Real`] gives beyond its operators, taken from `num_traits::Float` for every float type, so that
/// `Float`'s constants (its epsilon, largest value, infinity) stay out of reach of code bounded by `Real` alone (the
/// module's note).
pub trait RealOps: Sized {
    /// `self·a + b`, the fused multiply-add, rounded once. R-34 requires it, written explicitly, at every branch input
    /// that consumes multi-op arithmetic (gpu_determinism_note § "The discipline", rules 2 and 6), as `d²` is built in
    /// [`crate::physics::pairs`]. Whether every GPU backend fuses it is RQ-298's open question.
    fn fma(self, a: Self, b: Self) -> Self;

    /// `√self`: correctly rounded on the CPU, not guaranteed so on a GPU backend, so no branch decides on it (Part 2c
    /// rule 2: compare `d²`, never `d`).
    fn square_root(self) -> Self;
}

impl<T: Float> RealOps for T {
    #[inline]
    fn fma(self, a: T, b: T) -> T {
        Float::mul_add(self, a, b)
    }

    #[inline]
    fn square_root(self) -> T {
        Float::sqrt(self)
    }
}

/// The IEEE 754 bit patterns a producer selects at a pole, and the stored value's bit test (R-436; the module's note).
/// A float type with them has the exact constructions' bit-level steps too ([`crate::chart::invariant::WarpReal`] and
/// its [`crate::decode::invariant::ExactSign`]), f32 and f64 alike, so that a chart's [`crate::chart::Chart::decode`],
/// at a payload float type, may run its construction (the Burrau chart's).
pub trait IeeeBits: Real + crate::chart::invariant::WarpReal {
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

    /// `|self|`'s significand, read from the bits: its fraction field under the exponent field of `1`, so in `[1, 2)`,
    /// with `|self| = significand · 2^exponent` for a normal `self` (exact, no arithmetic). The sign is dropped. Used
    /// where a product must be formed with no subnormal intermediate (R-547; `crate::word::crossing`).
    fn significand(self) -> Self;

    /// `self·b`, rounded the same on every backend, whether it contracts or fuses or not. At f32, the GPU's precision,
    /// each operand is split at its bits into a high part, its fraction's low 12 bits cleared, and the exact remainder,
    /// each of at most 12 significant bits, so the four partial products are exact (where normal) and a backend that
    /// fuses one into the following add rounds the same sum; they are added in one fixed order,
    /// `hh + ((hl + lh) + ll)`, within an ulp or so of `self·b`. At f64, the CPU's alone, where Rust never contracts,
    /// it is `self·b`.
    fn product(self, b: Self) -> Self;

    /// `1/φ = (√5 − 1)/2`, the golden section's ratio, correctly rounded (`0x3F1E_377A` at f32, `0x3FE3_C6EF_372F_E950`
    /// at f64): the register's `inv_phi`, one f64 whose rounding to f32, done as a constant, is the f32 nearest too. So
    /// every backend holds the same value and no runtime `sqrt` forms it (gpu_determinism_note § "The discipline"):
    /// `crate::encode::curve::nearest`'s step.
    const INV_PHI: Self;

    /// `τ_I`, step 0's skip tolerance (R-547; dd_encode §3.2; `crate::canon`): step 0 is skipped where
    /// `|I_in − 1| ≤ τ_I`. The register's `tau_i`, in eps of the precision, times the precision's eps, which the
    /// generator writes as each width's exact value (`TAU_I_F32`, `TAU_I_F64`). A constant, so every backend compares
    /// against the same bits.
    const TAU_I: Self;
}

impl IeeeBits for f32 {
    const INV_PHI: f32 = crate::generated::constants::INV_PHI as f32;
    const TAU_I: f32 = crate::generated::constants::TAU_I_F32;

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
    fn significand(self) -> f32 {
        f32::from_bits((self.to_bits() & 0x007F_FFFF) | 0x3F80_0000)
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
    const INV_PHI: f64 = crate::generated::constants::INV_PHI;
    const TAU_I: f64 = crate::generated::constants::TAU_I_F64;

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
    fn significand(self) -> f64 {
        f64::from_bits((self.to_bits() & 0x000F_FFFF_FFFF_FFFF) | 0x3FF0_0000_0000_0000)
    }

    #[inline]
    fn product(self, b: f64) -> f64 {
        self * b
    }
}
