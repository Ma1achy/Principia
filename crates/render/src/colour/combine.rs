//! The combiners in Rust (dd_colouring §3.5; colour_composition §4.1; R-77), the mirror of the built-in combiner
//! occupants `shaders/wgsl/frag/combiner/replace_l.wgsl` and `multiply.wgsl`, and of the `Option` handling the
//! generated `shade()` applies around them ([`crate::codegen::combine`]). The tests compare the WGSL with it.
//!
//! **Replace-L**, Principia's default: the base colour, linear sRGB, to OKLab; `L ← L_min + (L_max − L_min)·b`, with
//! defaults `L_min = 0`, `L_max = 1`, so the default is `L = b` (R-77); `(a, b_ab)` untouched; back to linear sRGB. A
//! bound brightness owns L, and the colour contributes hue and chroma only (render contract Part 4, L-ownership).
//! **Multiply**: `rgb·b` in linear space, which keeps the base's own L structure.
//!
//! **`None` is the identity of `combine`** (colour_composition §4.1's truth table; render_gui_spec §13): colour and
//! brightness → the combiner; colour alone → the colour as it is; brightness alone → the combiner on white, so
//! `OKLab(L = B, 0, 0)` under Replace-L and `white · B` under Multiply; neither → the flat mid-grey `OKLab(0.6, 0, 0)`.
//!
//! Every function is generic over [`Real`], so it runs in f32, the shaders' precision, and in f64.

use super::space::{linear_to_oklab, oklab_to_linear, Real};

/// The flat mid-grey's OKLab lightness, both slots None (colour_composition §4.1: `OKLab(0.6,0,0)`). The generated
/// `shade()` writes it from here ([`crate::codegen::combine`]).
pub const MID_GREY_L: f64 = 0.6;

/// White, linear sRGB: what a colour None gives the combiner (colour_composition §4.1: `white · B`).
pub const WHITE: [f64; 3] = [1.0, 1.0, 1.0];

/// Replace-L's lightness range, `L_min` and `L_max` (dd_colouring §3.5). Its default is `L_min = 0`, `L_max = 1`, so
/// `L = b` (R-77); the WGSL occupant's uniforms `l_min` and `l_max` default to the same.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LRange {
    pub l_min: f64,
    pub l_max: f64,
}

impl LRange {
    /// R-77's defaults: `L_min = 0`, `L_max = 1`.
    pub const DEFAULT: LRange = LRange {
        l_min: 0.0,
        l_max: 1.0,
    };

    /// The lightness brightness `b` gives: `L_min + (L_max − L_min)·b`, evaluated as `L_min·(1 − b) + L_max·b`,
    /// which is the same value and, in floating point, exact at each end: `b = 0` gives `L_min` and `b = 1` gives
    /// `L_max`, and at the defaults it is `b` itself. The WGSL evaluates the same form.
    pub fn lightness<T: Real>(self, b: T) -> T {
        T::of(self.l_min) * (T::of(1.0) - b) + T::of(self.l_max) * b
    }
}

impl Default for LRange {
    fn default() -> Self {
        LRange::DEFAULT
    }
}

/// A built-in combiner (render_gui_spec §2: the `combiner` slot is required, Replace-L or Multiply).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Combiner {
    /// Replace-L over its lightness range.
    ReplaceL(LRange),
    /// Multiply.
    Multiply,
}

impl Combiner {
    /// The combiner's built-in occupant id, its WGSL file's name in `shaders/wgsl/frag/combiner/`.
    pub const fn id(self) -> &'static str {
        match self {
            Combiner::ReplaceL(_) => "replace_l",
            Combiner::Multiply => "multiply",
        }
    }

    /// `combine(rgb, b)`, the slot function, both inputs present.
    pub fn apply<T: Real>(self, rgb: [T; 3], b: T) -> [T; 3] {
        match self {
            Combiner::ReplaceL(range) => replace_l(range, rgb, b),
            Combiner::Multiply => multiply(rgb, b),
        }
    }
}

/// Replace-L's step in OKLab: `L` replaced by `range`'s lightness for `b`, `(a, b_ab)` returned as given, bit for bit.
pub fn replace_l_lab<T: Real>(range: LRange, lab: [T; 3], b: T) -> [T; 3] {
    [range.lightness(b), lab[1], lab[2]]
}

/// The OKLab colour Replace-L gives: linear sRGB `rgb` to OKLab, then [`replace_l_lab`]. Its L is the brightness's,
/// its `(a, b_ab)` the colour's.
pub fn replace_l_oklab<T: Real>(range: LRange, rgb: [T; 3], b: T) -> [T; 3] {
    replace_l_lab(range, linear_to_oklab(rgb), b)
}

/// Replace-L (dd_colouring §3.5): [`replace_l_oklab`], back to linear sRGB. The result is not clamped: the gamut clamp
/// is the display stage's (render contract Part 4, composite order).
pub fn replace_l<T: Real>(range: LRange, rgb: [T; 3], b: T) -> [T; 3] {
    oklab_to_linear(replace_l_oklab(range, rgb, b))
}

/// Multiply (dd_colouring §3.5): `rgb·b`, each channel, in linear space.
pub fn multiply<T: Real>(rgb: [T; 3], b: T) -> [T; 3] {
    rgb.map(|c| c * b)
}

/// `combine` with `None` as its identity (colour_composition §4.1): both present → `combiner`; the colour alone → the
/// colour; the brightness alone → `combiner` on [`WHITE`]; neither → the mid-grey, `OKLab(`[`MID_GREY_L`]`, 0, 0)` as
/// linear sRGB, whatever the combiner.
pub fn combine<T: Real>(
    combiner: Combiner,
    colour: Option<[T; 3]>,
    brightness: Option<T>,
) -> [T; 3] {
    match (colour, brightness) {
        (Some(rgb), Some(b)) => combiner.apply(rgb, b),
        (Some(rgb), None) => rgb,
        (None, Some(b)) => combiner.apply(WHITE.map(T::of), b),
        (None, None) => oklab_to_linear([T::of(MID_GREY_L), T::of(0.0), T::of(0.0)]),
    }
}
