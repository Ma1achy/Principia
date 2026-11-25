//! `Real`, the float type the kernel is generic over (canonical_spec §1 item 2; lowering Part 2): f32 for the GPU build,
//! f64 for the CPU build, and no other instantiation. The payload's widths are a function of it, never
//! hardcoded to f32 (philosophy §7.1, §7.7; dd_simstate_payload §1).
//!
//! **The poles' `±∞`** ([`IeeeBits`]; dd_decoder §3.6, "Each producer selects its pole value"). WGSL lets an
//! implementation assume there are no infinities (gpu_determinism_note § "The discipline", rule 4), so a producer never
//! forms `±∞` by `1/0` or by arithmetic on `∞`, and never by an infinite literal: it selects the integer pattern's bits,
//! then bitcasts them ([`IeeeBits::or_pos_inf`], [`IeeeBits::or_neg_inf`]: `0x7F80_0000` and `0xFF80_0000` at f32,
//! `0x7FF0_0000_0000_0000` and `0xFFF0_0000_0000_0000` at f64), as `d_min`'s packer writes its `+∞`. The select is on
//! the integer bits, so the value it discards is never read as a float, and no infinite float constant is left for a
//! shader compiler to fold. `self`'s bits are taken before the choice, so that both of its operands are computed and the
//! kernel's WGSL has a `select`, not a branch (gpu_determinism_note's discipline). A stored value is tested on its bits, an exponent field of all ones
//! ([`IeeeBits::non_finite_bits`]), as the kernel's NaN guard is, never by `isinf` or a comparison with the largest
//! finite value. The payload's float types have them ([`crate::payload::PayloadReal`]'s bound): f32 and f64.
//!
//! **What `Real` gives the physics layer** (integrator_contract Part 2c, "Rules the new kernel must hold by
//! construction"): the four operations and negation, [`RealOps::fma`], [`RealOps::square_root`] and the comparisons
//! of `PartialOrd`, and nothing else; its other functions, the transcendentals ([`Real::arctan2`], [`Real::sinc`],
//! [`Real::sine`] and the rest), ask for `num_traits::Float` too, so code bounded by `Real` alone cannot reach them. It
//! carries no numeric constant, no epsilon, no largest value, no infinity: rule 1
//! forbids a precision-dependent constant in a control-flow decision, so code bounded by `Real` alone, as
//! [`crate::physics`] is, has none to reach. A tolerance that must scale with precision is supplied where the value is
//! used, never where a branch is taken. `NAME` is the precision row's label for the ledger, not a number. The two
//! functions are named apart from `num_traits::Float`'s `mul_add` and `sqrt`, which the links' bound
//! ([`crate::generated::links::LinkReal`]) also carries, so that a call on a type bounded by both is not ambiguous; the
//! transcendentals are named apart from `Float`'s for the same reason (`sine` for `sin`, `logarithm` for `ln`).
//!
//! **The pinned maths library**. Every f64 transcendental the CPU chart evaluator reaches is a
//! method of [`Real`], and at f64 on the host each is the pinned pure-software libm's (`libm = "=0.2.16"` with
//! `force-soft-floats`, as `Cargo.toml` pins it for the `N_sub` table): `sin`, `cos`, `tan`, `asin`, `acos`,
//! `atan2`, `exp`, `expm1`, `log`, `log1p`, `pow`, `hypot`, `cbrt`, `sinh`, `cosh`, `tanh` and `atanh`, `sin_cos` its
//! `sin` and `cos`. No platform maths library, compiler intrinsic or architecture-specific instruction computes them,
//! so the f64 evaluation, and every record narrowed from it, has the same bits on every machine (the Mac, Linux,
//! Windows and the wasm build). `sqrt` is not among them: IEEE 754 requires it correctly rounded, so every platform gives
//! the same bits ([`RealOps::square_root`]). At f32, [`Real::sine`], [`Real::cosine`] and [`Real::sine_cosine`] are the
//! kernel's own on every target, host and GPU ([`crate::trig`]), as [`Real::arctan2`] and [`Real::sinc`] are; on the
//! host every other f32 transcendental is the pinned libm's f32 function (`expf`, `tanhf`, `logf`, …), and on the GPU
//! the backend's builtin, unchanged. `crates/kernel/tests/transcendental_pin.rs` fails on a `Float::` or inherent
//! transcendental anywhere in the evaluator's source outside this file's `Real` items, and on a platform maths call in
//! the production crates.
//!
//! Two features, off in every build but the measurement's (`cargo xtask measure libm-pin`), change the host
//! path: `measure-std-maths` puts it back on `num_traits::Float`, the platform's library (the
//! f32 sine and cosine stay the kernel's own), and `measure-perturb-sin` moves [`Real::sine`]'s f64 result one ulp up,
//! the measurement's control.

use core::fmt::Debug;
use core::ops::{Add, Div, Mul, Neg, Sub};

use spirv_std::num_traits::Float;

/// A float type the one kernel source is instantiated at: f32 on the GPU, f64 on the CPU.
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

    /// `atan2(self, x)`, the angle of the point `(x, self)`, in `[−π, π]`, for a type that is also a `Float`:
    /// `num_traits::Float`'s by default; at f64 on the host the pinned libm's `atan2` (the module's note); and at
    /// f32, the GPU's precision, the kernel's own, [`crate::arctan::atan2_f32`], in integer arithmetic, so every backend
    /// gives the CPU's bits. The encode's angles and the shape sphere decode's `β` call it, so the precision chooses it
    /// through `crate::generated::links::LinkReal`, whose bound holds `Real`.
    #[inline]
    fn arctan2(self, x: Self) -> Self
    where
        Self: Float,
    {
        Float::atan2(self, x)
    }

    /// `atan2(self, x)` where the f32 path keeps the backend's: `num_traits::Float`'s by default, so at f32 the GPU's own
    /// `atan2`, unchanged; at f64 on the host the pinned libm's `atan2`, as [`Real::arctan2`]'s (the module's note).
    /// Callers whose f32 bits may stay the backend's take it; those that need the CPU's bits on every backend take
    /// [`Real::arctan2`].
    #[inline]
    fn arctangent2(self, x: Self) -> Self
    where
        Self: Float,
    {
        Float::atan2(self, x)
    }

    /// `sin r/r` at `r = self`, nonzero, for a type that is also a `Float`, with `_square` the `r²` the caller formed
    /// before its square root: [`Real::sine`]`(r)/r` by default, so `r²` is not read, at f64 on the host the pinned
    /// libm's `sin` through it; and at f32, the GPU's precision, [`crate::chart::shape_sphere::sinc_f32`], which reads no
    /// backend `sin` near 0. The shape sphere's exponential map calls it, as [`Real::arctan2`] is gated.
    #[inline]
    fn sinc(self, _square: Self) -> Self
    where
        Self: Float,
    {
        self.sine() / self
    }

    /// `self` read on the circle, for an angle handed to [`Real::sine`] and [`Real::cosine`]: a value whose `sin` and `cos`
    /// are `self`'s, in `[−π̂, π̂]` (`π̂` the f32 nearest `π`) where the backend's own functions are bounded. At f32, the
    /// GPU's precision, [`crate::circle::on_circle_f32`], the kernel's own exact reduction modulo `2π` in integer
    /// arithmetic, since WGSL bounds `sin` and `cos` only inside `[−π, π]` and a GPU need not reduce past it. By default,
    /// at f64 included, `self` unchanged: f64 runs only on the host, where the pinned libm's `sin` and `cos` reduce
    /// exactly, so its bits stay as they were.
    #[inline]
    fn on_circle(self) -> Self
    where
        Self: Float,
    {
        self
    }

    /// `(sin(self), cos(self))`: [`Real::sine`]'s and [`Real::cosine`]'s values, so the pair is the two functions' own.
    #[inline]
    fn sine_cosine(self) -> (Self, Self)
    where
        Self: Float,
    {
        (self.sine(), self.cosine())
    }

    /// `sin(self)`. `Float::sin` by default; at f64 on the host libm's `sin`, pinned; at f32 the kernel's own (the
    /// module's note).
    #[inline]
    fn sine(self) -> Self
    where
        Self: Float,
    {
        Float::sin(self)
    }

    /// `cos(self)`. `Float::cos` by default; at f64 on the host libm's `cos`, pinned; at f32 the kernel's own (the
    /// module's note).
    #[inline]
    fn cosine(self) -> Self
    where
        Self: Float,
    {
        Float::cos(self)
    }

    /// `tan(self)`. `Float::tan` by default; at f64 on the host libm's `tan`, pinned (the module's note).
    #[inline]
    fn tangent(self) -> Self
    where
        Self: Float,
    {
        Float::tan(self)
    }

    /// `asin(self)`, in `[−π/2, π/2]`. `Float::asin` by default; at f64 on the host libm's `asin`, pinned (the module's
    /// note).
    #[inline]
    fn arcsine(self) -> Self
    where
        Self: Float,
    {
        Float::asin(self)
    }

    /// `acos(self)`, in `[0, π]`. `Float::acos` by default; at f64 on the host libm's `acos`, pinned (the module's
    /// note).
    #[inline]
    fn arccosine(self) -> Self
    where
        Self: Float,
    {
        Float::acos(self)
    }

    /// `eˢᵉˡᶠ`. `Float::exp` by default; at f64 on the host libm's `exp`, pinned (the module's note).
    #[inline]
    fn exponential(self) -> Self
    where
        Self: Float,
    {
        Float::exp(self)
    }

    /// `eˢᵉˡᶠ − 1`, accurate near 0 (libm's `expm1`). `Float::exp_m1` by default; at f64 on the host libm's `expm1`,
    /// pinned (the module's note).
    #[inline]
    fn exp_minus_one(self) -> Self
    where
        Self: Float,
    {
        Float::exp_m1(self)
    }

    /// `ln(self)`, the natural logarithm (libm's `log`). `Float::ln` by default; at f64 on the host libm's `log`,
    /// pinned (the module's note).
    #[inline]
    fn logarithm(self) -> Self
    where
        Self: Float,
    {
        Float::ln(self)
    }

    /// `ln(1 + self)`, accurate near 0 (libm's `log1p`). `Float::ln_1p` by default; at f64 on the host libm's `log1p`,
    /// pinned (the module's note).
    #[inline]
    fn ln_one_plus(self) -> Self
    where
        Self: Float,
    {
        Float::ln_1p(self)
    }

    /// `∛self`. `Float::cbrt` by default; at f64 on the host libm's `cbrt`, pinned (the module's note).
    #[inline]
    fn cube_root(self) -> Self
    where
        Self: Float,
    {
        Float::cbrt(self)
    }

    /// `sinh(self)`. `Float::sinh` by default; at f64 on the host libm's `sinh`, pinned (the module's note).
    #[inline]
    fn hyperbolic_sine(self) -> Self
    where
        Self: Float,
    {
        Float::sinh(self)
    }

    /// `cosh(self)`. `Float::cosh` by default; at f64 on the host libm's `cosh`, pinned (the module's note).
    #[inline]
    fn hyperbolic_cosine(self) -> Self
    where
        Self: Float,
    {
        Float::cosh(self)
    }

    /// `tanh(self)`. `Float::tanh` by default; at f64 on the host libm's `tanh`, pinned (the module's note).
    #[inline]
    fn hyperbolic_tangent(self) -> Self
    where
        Self: Float,
    {
        Float::tanh(self)
    }

    /// `artanh(self)`, the inverse of `tanh`. `Float::atanh` by default; at f64 on the host libm's `atanh`, pinned (the
    /// module's note).
    #[inline]
    fn arctanh(self) -> Self
    where
        Self: Float,
    {
        Float::atanh(self)
    }

    /// `selfˣ` (libm's `pow`). `Float::powf` by default; at f64 on the host libm's `pow`, pinned (the module's note).
    #[inline]
    fn power(self, x: Self) -> Self
    where
        Self: Float,
    {
        Float::powf(self, x)
    }

    /// `√(self² + x²)`, without undue overflow or underflow. `Float::hypot` by default; at f64 on the host libm's
    /// `hypot`, pinned (the module's note).
    #[inline]
    fn hypotenuse(self, x: Self) -> Self
    where
        Self: Float,
    {
        Float::hypot(self, x)
    }
}

/// The f32 path: the kernel's own `atan2`, `sinc`, reduction, sine and cosine on every target; on the host each other
/// transcendental the pinned libm's f32 function, gated off the SPIR-V target, where the backend's builtin stays, and
/// off under `measure-std-maths` (the module's note).
impl Real for f32 {
    const NAME: &'static str = "f32";

    #[inline]
    fn arctan2(self, x: f32) -> f32 {
        crate::arctan::atan2_f32(self, x)
    }

    #[inline]
    fn sinc(self, square: f32) -> f32 {
        crate::chart::shape_sphere::sinc_f32(self, square)
    }

    #[inline]
    fn on_circle(self) -> f32 {
        crate::circle::on_circle_f32(self)
    }

    #[inline]
    fn sine(self) -> f32 {
        crate::trig::sin_cos_f32(self).0
    }

    #[inline]
    fn cosine(self) -> f32 {
        crate::trig::sin_cos_f32(self).1
    }

    #[inline]
    fn sine_cosine(self) -> (f32, f32) {
        crate::trig::sin_cos_f32(self)
    }

    #[cfg(not(any(target_arch = "spirv", feature = "measure-std-maths")))]
    #[inline]
    fn arctangent2(self, x: f32) -> f32 {
        libm::atan2f(self, x)
    }

    #[cfg(not(any(target_arch = "spirv", feature = "measure-std-maths")))]
    #[inline]
    fn tangent(self) -> f32 {
        libm::tanf(self)
    }

    #[cfg(not(any(target_arch = "spirv", feature = "measure-std-maths")))]
    #[inline]
    fn arcsine(self) -> f32 {
        libm::asinf(self)
    }

    #[cfg(not(any(target_arch = "spirv", feature = "measure-std-maths")))]
    #[inline]
    fn arccosine(self) -> f32 {
        libm::acosf(self)
    }

    #[cfg(not(any(target_arch = "spirv", feature = "measure-std-maths")))]
    #[inline]
    fn exponential(self) -> f32 {
        libm::expf(self)
    }

    #[cfg(not(any(target_arch = "spirv", feature = "measure-std-maths")))]
    #[inline]
    fn exp_minus_one(self) -> f32 {
        libm::expm1f(self)
    }

    #[cfg(not(any(target_arch = "spirv", feature = "measure-std-maths")))]
    #[inline]
    fn logarithm(self) -> f32 {
        libm::logf(self)
    }

    #[cfg(not(any(target_arch = "spirv", feature = "measure-std-maths")))]
    #[inline]
    fn ln_one_plus(self) -> f32 {
        libm::log1pf(self)
    }

    #[cfg(not(any(target_arch = "spirv", feature = "measure-std-maths")))]
    #[inline]
    fn cube_root(self) -> f32 {
        libm::cbrtf(self)
    }

    #[cfg(not(any(target_arch = "spirv", feature = "measure-std-maths")))]
    #[inline]
    fn hyperbolic_sine(self) -> f32 {
        libm::sinhf(self)
    }

    #[cfg(not(any(target_arch = "spirv", feature = "measure-std-maths")))]
    #[inline]
    fn hyperbolic_cosine(self) -> f32 {
        libm::coshf(self)
    }

    #[cfg(not(any(target_arch = "spirv", feature = "measure-std-maths")))]
    #[inline]
    fn hyperbolic_tangent(self) -> f32 {
        libm::tanhf(self)
    }

    #[cfg(not(any(target_arch = "spirv", feature = "measure-std-maths")))]
    #[inline]
    fn arctanh(self) -> f32 {
        libm::atanhf(self)
    }

    #[cfg(not(any(target_arch = "spirv", feature = "measure-std-maths")))]
    #[inline]
    fn power(self, x: f32) -> f32 {
        libm::powf(self, x)
    }

    #[cfg(not(any(target_arch = "spirv", feature = "measure-std-maths")))]
    #[inline]
    fn hypotenuse(self, x: f32) -> f32 {
        libm::hypotf(self, x)
    }
}

/// The f64 path: on the host each transcendental the pinned libm's (the module's note), [`Real::sinc`] and
/// [`Real::sine_cosine`] the defaults, through [`Real::sine`] and [`Real::cosine`]. Each override is gated off the SPIR-V
/// target, where the pinned libm is not built and the kernel instantiates f32 alone, and off under
/// `measure-std-maths`, the measurement's path before the pin; there every method is the default, `num_traits::Float`'s,
/// on the host the platform's maths library. Under `measure-perturb-sin`, the measurement's control, [`Real::sine`]'s
/// result is moved one ulp up, its bit pattern plus one.
impl Real for f64 {
    const NAME: &'static str = "f64";

    #[cfg(not(any(target_arch = "spirv", feature = "measure-std-maths")))]
    #[inline]
    fn arctan2(self, x: f64) -> f64 {
        libm::atan2(self, x)
    }

    #[cfg(not(any(target_arch = "spirv", feature = "measure-std-maths")))]
    #[inline]
    fn arctangent2(self, x: f64) -> f64 {
        libm::atan2(self, x)
    }

    #[cfg(not(any(target_arch = "spirv", feature = "measure-std-maths")))]
    #[inline]
    fn sine(self) -> f64 {
        let s = libm::sin(self);
        #[cfg(feature = "measure-perturb-sin")]
        let s = f64::from_bits(s.to_bits().wrapping_add(1));
        s
    }

    #[cfg(not(any(target_arch = "spirv", feature = "measure-std-maths")))]
    #[inline]
    fn cosine(self) -> f64 {
        libm::cos(self)
    }

    #[cfg(not(any(target_arch = "spirv", feature = "measure-std-maths")))]
    #[inline]
    fn tangent(self) -> f64 {
        libm::tan(self)
    }

    #[cfg(not(any(target_arch = "spirv", feature = "measure-std-maths")))]
    #[inline]
    fn arcsine(self) -> f64 {
        libm::asin(self)
    }

    #[cfg(not(any(target_arch = "spirv", feature = "measure-std-maths")))]
    #[inline]
    fn arccosine(self) -> f64 {
        libm::acos(self)
    }

    #[cfg(not(any(target_arch = "spirv", feature = "measure-std-maths")))]
    #[inline]
    fn exponential(self) -> f64 {
        libm::exp(self)
    }

    #[cfg(not(any(target_arch = "spirv", feature = "measure-std-maths")))]
    #[inline]
    fn exp_minus_one(self) -> f64 {
        libm::expm1(self)
    }

    #[cfg(not(any(target_arch = "spirv", feature = "measure-std-maths")))]
    #[inline]
    fn logarithm(self) -> f64 {
        libm::log(self)
    }

    #[cfg(not(any(target_arch = "spirv", feature = "measure-std-maths")))]
    #[inline]
    fn ln_one_plus(self) -> f64 {
        libm::log1p(self)
    }

    #[cfg(not(any(target_arch = "spirv", feature = "measure-std-maths")))]
    #[inline]
    fn cube_root(self) -> f64 {
        libm::cbrt(self)
    }

    #[cfg(not(any(target_arch = "spirv", feature = "measure-std-maths")))]
    #[inline]
    fn hyperbolic_sine(self) -> f64 {
        libm::sinh(self)
    }

    #[cfg(not(any(target_arch = "spirv", feature = "measure-std-maths")))]
    #[inline]
    fn hyperbolic_cosine(self) -> f64 {
        libm::cosh(self)
    }

    #[cfg(not(any(target_arch = "spirv", feature = "measure-std-maths")))]
    #[inline]
    fn hyperbolic_tangent(self) -> f64 {
        libm::tanh(self)
    }

    #[cfg(not(any(target_arch = "spirv", feature = "measure-std-maths")))]
    #[inline]
    fn arctanh(self) -> f64 {
        libm::atanh(self)
    }

    #[cfg(not(any(target_arch = "spirv", feature = "measure-std-maths")))]
    #[inline]
    fn power(self, x: f64) -> f64 {
        libm::pow(self, x)
    }

    #[cfg(not(any(target_arch = "spirv", feature = "measure-std-maths")))]
    #[inline]
    fn hypotenuse(self, x: f64) -> f64 {
        libm::hypot(self, x)
    }
}

/// The two functions [`Real`] gives beyond its operators, taken from `num_traits::Float` for every float type, so that
/// `Float`'s constants (its epsilon, largest value, infinity) stay out of reach of code bounded by `Real` alone (the
/// module's note).
pub trait RealOps: Sized {
    /// `self·a + b`, the fused multiply-add, rounded once where the backend fuses it. Not every backend does (CI's
    /// lavapipe rounds the product, then the add), so it never builds a branch input that must agree across backends
    /// (gpu_determinism_note § "The discipline", rule 6).
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

/// The IEEE 754 bit patterns a producer selects at a pole, and the stored value's bit test (the module's note).
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

    /// Whether `self` is NaN, not `±∞`: its exponent field all ones and its fraction field non-zero, a test on the bits,
    /// made before any arithmetic on `self`.
    fn nan_bits(self) -> bool;

    /// `|self|`'s unbiased exponent field, read from the bits: `e` with `2^e ≤ |self| < 2^(e+1)` for a normal `self`,
    /// the field's least value (`−127` at f32, `−1023` at f64) for `±0` and subnormals, its greatest (128, 1024) for
    /// `±∞` and NaN.
    fn exponent(self) -> i32;

    /// `2^e`, formed from its bits, for `e` a normal exponent (`−126..=127` at f32, `−1022..=1023` at f64).
    fn pow2(e: i32) -> Self;

    /// `|self|`'s significand, read from the bits: its fraction field under the exponent field of `1`, so in `[1, 2)`,
    /// with `|self| = significand · 2^exponent` for a normal `self` (exact, no arithmetic). The sign is dropped. Used
    /// where a product must be formed with no subnormal intermediate (`crate::word::crossing`).
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

    /// `τ_I`, step 0's skip tolerance (dd_encode §3.2; `crate::canon`): step 0 is skipped where
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
    fn nan_bits(self) -> bool {
        self.to_bits() & 0x7FFF_FFFF > 0x7F80_0000
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
    fn nan_bits(self) -> bool {
        self.to_bits() & 0x7FFF_FFFF_FFFF_FFFF > 0x7FF0_0000_0000_0000
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
