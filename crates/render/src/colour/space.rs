//! The colour spaces of dd_colouring §3.1, in Rust: the sRGB transfer, linear sRGB ↔ OKLab through `M₁`, the cube
//! root and `M₂` and back through the closed `M₂⁻¹` and `M₁⁻¹`, and OKLCH, OKLab's polar form. Every function is
//! generic over [`Real`], so it runs in f32, as the shaders do, and in f64, as the references the tests compare them
//! with do; the CPU-side generators (swatches, palettes) call them too. This module is the one Rust source of the
//! transforms: `present`'s colour-space maps call it (render contract Part 5).
//!
//! The shaders' counterparts are WGSL, written once there: the transfer and OKLab → linear in the generated prelude
//! (`ledger::gen::prelude`), linear → OKLab and the rest in `shaders/wgsl/lib/colour_space.wgsl`, which follows the
//! prelude at assembly (render_gui_spec §10.1). `tests/oklab_transcription.rs` compares every coefficient of
//! [`OKLAB`], of dd_colouring §3.1 and of that WGSL with Ottosson's published values (R-51), and the WGSL with these
//! functions on the GPU.
//!
//! The names follow the corpus's usage (render contract Part 5): "sRGB" is the encoded value, "linear" linear sRGB,
//! the colour slot's space (render contract Part 2). So `srgb_to_oklab` takes an encoded colour and `linear_to_oklab`
//! the slot's linear one.

use std::ops::{Add, Div, Mul, Neg, Sub};

/// A row-major 3 × 3 matrix.
pub type Mat3 = [[f64; 3]; 3];

/// The decode's threshold: an encoded value at or below it is on the linear segment (dd_colouring §3.1).
pub const SRGB_DECODE_THRESHOLD: f64 = 0.04045;

/// The encode's threshold: a linear value at or below it is on the linear segment (dd_colouring §3.1).
pub const SRGB_ENCODE_THRESHOLD: f64 = 0.0031308;

/// The linear segment's slope, `12.92` (dd_colouring §3.1).
pub const SRGB_SLOPE: f64 = 12.92;

/// The power segment's offset, `0.055`, and its scale, `1.055` (dd_colouring §3.1).
pub const SRGB_OFFSET: f64 = 0.055;
pub const SRGB_SCALE: f64 = 1.055;

/// The power segment's exponent, `2.4` (dd_colouring §3.1).
pub const SRGB_GAMMA: f64 = 2.4;

/// The four matrices of dd_colouring §3.1: linear sRGB → OKLab is `M₂ · (M₁ · rgb)^{1/3}`, and back is
/// `M₁⁻¹ · (M₂⁻¹ · Lab)³`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Coefficients {
    /// `M₁`: linear sRGB → LMS.
    pub m1: Mat3,
    /// `M₂`: `lms^{1/3}` → OKLab.
    pub m2: Mat3,
    /// `M₂⁻¹`, closed: OKLab → `lms'`, `l' = L + 0.3963377774a + 0.2158037573b` and so on, so its first column is 1.
    pub m2_inv: Mat3,
    /// `M₁⁻¹`: LMS → linear sRGB.
    pub m1_inv: Mat3,
}

/// dd_colouring §3.1's coefficients, Ottosson's (R-51).
pub const OKLAB: Coefficients = Coefficients {
    m1: [
        [0.4122214708, 0.5363325363, 0.0514459929],
        [0.2119034982, 0.6806995451, 0.1073969566],
        [0.0883024619, 0.2817188376, 0.6299787005],
    ],
    m2: [
        [0.2104542553, 0.7936177850, -0.0040720468],
        [1.9779984951, -2.4285922050, 0.4505937099],
        [0.0259040371, 0.7827717662, -0.8086757660],
    ],
    m2_inv: [
        [1.0, 0.3963377774, 0.2158037573],
        [1.0, -0.1055613458, -0.0638541728],
        [1.0, -0.0894841775, -1.2914855480],
    ],
    m1_inv: [
        [4.0767416621, -3.3077115913, 0.2309699292],
        [-1.2684380046, 2.6097574011, -0.3413193965],
        [-0.0041960863, -0.7034186147, 1.7076147010],
    ],
};

/// The float types the transforms run in: f32, the shaders' precision, and f64.
pub trait Real:
    Copy
    + PartialOrd
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
    + Neg<Output = Self>
{
    /// `x`, rounded to this type.
    fn of(x: f64) -> Self;
    /// The real cube root, negative for a negative argument.
    fn cbrt(self) -> Self;
    /// `self` to the power `e`.
    fn powf(self, e: Self) -> Self;
    /// The square root.
    fn sqrt(self) -> Self;
    /// The angle of `(x, self)`, in radians in [−π, π].
    fn atan2(self, x: Self) -> Self;
    /// The cosine.
    fn cos(self) -> Self;
    /// The sine.
    fn sin(self) -> Self;
}

macro_rules! real {
    ($t:ty) => {
        impl Real for $t {
            fn of(x: f64) -> Self {
                x as $t
            }
            fn cbrt(self) -> Self {
                <$t>::cbrt(self)
            }
            fn powf(self, e: Self) -> Self {
                <$t>::powf(self, e)
            }
            fn sqrt(self) -> Self {
                <$t>::sqrt(self)
            }
            fn atan2(self, x: Self) -> Self {
                <$t>::atan2(self, x)
            }
            fn cos(self) -> Self {
                <$t>::cos(self)
            }
            fn sin(self) -> Self {
                <$t>::sin(self)
            }
        }
    };
}

real!(f32);
real!(f64);

/// `m · v`, each coefficient rounded to `T` first.
fn mul<T: Real>(m: &Mat3, v: [T; 3]) -> [T; 3] {
    m.map(|row| T::of(row[0]) * v[0] + T::of(row[1]) * v[1] + T::of(row[2]) * v[2])
}

/// The sRGB transfer, encoded to linear (dd_colouring §3.1): `c/12.92` if `c ≤ 0.04045`, else
/// `((c + 0.055)/1.055)^2.4`.
pub fn srgb_to_linear<T: Real>(c: T) -> T {
    if c <= T::of(SRGB_DECODE_THRESHOLD) {
        c / T::of(SRGB_SLOPE)
    } else {
        ((c + T::of(SRGB_OFFSET)) / T::of(SRGB_SCALE)).powf(T::of(SRGB_GAMMA))
    }
}

/// The sRGB transfer's inverse, linear to encoded (dd_colouring §3.1): `12.92c` if `c ≤ 0.0031308`, else
/// `1.055 c^{1/2.4} − 0.055`.
pub fn linear_to_srgb<T: Real>(c: T) -> T {
    if c <= T::of(SRGB_ENCODE_THRESHOLD) {
        T::of(SRGB_SLOPE) * c
    } else {
        T::of(SRGB_SCALE) * c.powf(T::of(1.0 / SRGB_GAMMA)) - T::of(SRGB_OFFSET)
    }
}

/// Linear sRGB to OKLab through `k`'s matrices: `Lab = M₂ · (M₁ · rgb)^{1/3}`, the real cube root of each.
pub fn linear_to_oklab_with<T: Real>(k: &Coefficients, rgb: [T; 3]) -> [T; 3] {
    mul(&k.m2, mul(&k.m1, rgb).map(T::cbrt))
}

/// OKLab to linear sRGB through `k`'s matrices: `rgb = M₁⁻¹ · (M₂⁻¹ · Lab)³`.
pub fn oklab_to_linear_with<T: Real>(k: &Coefficients, lab: [T; 3]) -> [T; 3] {
    mul(&k.m1_inv, mul(&k.m2_inv, lab).map(|x| x * x * x))
}

/// Linear sRGB to OKLab (dd_colouring §3.1).
pub fn linear_to_oklab<T: Real>(rgb: [T; 3]) -> [T; 3] {
    linear_to_oklab_with(&OKLAB, rgb)
}

/// OKLab to linear sRGB (dd_colouring §3.1).
pub fn oklab_to_linear<T: Real>(lab: [T; 3]) -> [T; 3] {
    oklab_to_linear_with(&OKLAB, lab)
}

/// Encoded sRGB to OKLab: decoded by [`srgb_to_linear`], then [`linear_to_oklab`].
pub fn srgb_to_oklab<T: Real>(srgb: [T; 3]) -> [T; 3] {
    linear_to_oklab(srgb.map(srgb_to_linear))
}

/// OKLab to encoded sRGB: [`oklab_to_linear`], then encoded by [`linear_to_srgb`].
pub fn oklab_to_srgb<T: Real>(lab: [T; 3]) -> [T; 3] {
    oklab_to_linear(lab).map(linear_to_srgb)
}

/// OKLab to OKLCH, its polar form (dd_colouring §3.1): `(L, C, h)`, `C = √(a² + b²)`, `h = atan2(b, a)` in radians,
/// in [−π, π]. An achromatic colour, `C = 0`, has `h = 0`, atan2's value at `(+0, +0)`: WGSL leaves atan2 undefined
/// there, and the shader returns 0 too.
pub fn oklab_to_oklch<T: Real>(lab: [T; 3]) -> [T; 3] {
    let [l, a, b] = lab;
    let c = (a * a + b * b).sqrt();
    let zero = T::of(0.0);
    let h = if c > zero { b.atan2(a) } else { zero };
    [l, c, h]
}

/// OKLCH to OKLab (dd_colouring §3.1): `(L, C cos h, C sin h)`, `h` in radians.
pub fn oklch_to_oklab<T: Real>(lch: [T; 3]) -> [T; 3] {
    let [l, c, h] = lch;
    [l, c * h.cos(), c * h.sin()]
}
