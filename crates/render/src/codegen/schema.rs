//! The map parameters' uniform schema (colour_composition §8, §1.1): the adopted ranges L ∈ [0.35, 0.90],
//! C ∈ [0.05, 0.22], κ ∈ [0.5, 12], f ∈ [2, 14], N ∈ [12, 96], ks ∈ [1, 20], s ∈ [0, 1], and the clamp into them.
//!
//! A ranged parameter is an `f32` uniform whose declaration carries its range, so the assembler's own check
//! ([`Uniform::admits`]) refuses a value outside it; a value reaches the declaration and a slider's write through
//! [`MapParam::clamp`] first.

use crate::assemble::{Uniform, UniformType};

/// A map parameter with an adopted range (colour_composition §8; κ and ks are §1.1's kernel temperatures, s §1.3's
/// per-blob strength).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MapParam {
    /// L ∈ [0.35, 0.90].
    L,
    /// C ∈ [0.05, 0.22].
    C,
    /// κ ∈ [0.5, 12], the vMF kernel's temperature (§1.1).
    Kappa,
    /// f ∈ [2, 14].
    F,
    /// N ∈ [12, 96].
    N,
    /// ks ∈ [1, 20], the top-k kernel's sharpness (§1.1).
    Ks,
    /// s ∈ [0, 1], the site overlay's per-blob strength (§1.3).
    S,
}

impl MapParam {
    /// Every map parameter, in §8's order.
    pub const ALL: [MapParam; 7] = [
        MapParam::L,
        MapParam::C,
        MapParam::Kappa,
        MapParam::F,
        MapParam::N,
        MapParam::Ks,
        MapParam::S,
    ];

    /// The parameter's symbol, as §8 writes it.
    pub const fn symbol(self) -> &'static str {
        match self {
            MapParam::L => "L",
            MapParam::C => "C",
            MapParam::Kappa => "κ",
            MapParam::F => "f",
            MapParam::N => "N",
            MapParam::Ks => "ks",
            MapParam::S => "s",
        }
    }

    /// The adopted range, `(lo, hi)`, both ends included (colour_composition §8).
    pub const fn range(self) -> (f64, f64) {
        match self {
            MapParam::L => (0.35, 0.90),
            MapParam::C => (0.05, 0.22),
            MapParam::Kappa => (0.5, 12.0),
            MapParam::F => (2.0, 14.0),
            MapParam::N => (12.0, 96.0),
            MapParam::Ks => (1.0, 20.0),
            MapParam::S => (0.0, 1.0),
        }
    }

    /// `v` clamped into the range; `None` for NaN, which has no place in it (±∞ clamp to the ends).
    pub fn clamp(self, v: f64) -> Option<f64> {
        let (lo, hi) = self.range();
        (!v.is_nan()).then(|| v.clamp(lo, hi))
    }

    /// The uniform `name` of this parameter: an `f32` whose range is the adopted one and whose default is `value`
    /// clamped into it; `None` for NaN.
    pub fn uniform(self, name: &str, value: f64) -> Option<Uniform> {
        Some(Uniform {
            name: name.to_owned(),
            ty: UniformType::F32,
            default: vec![self.clamp(value)?],
            range: Some(self.range()),
        })
    }
}
