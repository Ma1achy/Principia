//! The categorical palettes and the categorical filter (colour_composition §1.4; dd_colouring §3.7; R-77;
//! TASK-M7-08), the CPU twin of `shaders/wgsl/lib/palette.wgsl`, which the assembler places after the presentation
//! layer, so every node may call it.
//!
//! **Which palette a categorical field takes** ([`resolve`]). The outcome `state` takes colour_composition §1.4's
//! canonical nine-class palette, the occupant `outcome_state` ([`super::outcome`]), never a generated index palette
//! (R-77); so does `detail`, the union field the nine classes read beside `state` (dd_colouring §3.7: its palette
//! segment switches on `state`; applied per R-369). Every other categorical field of the ledger, a `categorical(n)`
//! scale, maps through the generated index palette of its `n` classes ([`generated`]): an Okabe–Ito cycle for
//! `n ≤ 8`, and for `n > 8` the golden-angle hue `θ_i = 2π·frac(i·φ_g)`, `φ_g = (√5 − 1)/2`, of every class, on the
//! prelude's `hue_wheel` (the register's OKLCH lightness and chroma; applied per R-369, as the debug views'
//! `dbg_cat` draws it).
//!
//! **The categorical filter** (colour_composition §1.4, "show class ∈ {…}, mute the rest"; REQ-COL-019) is a
//! parameter of every categorical mode: the class mask, a `u32` uniform ([`MASK_PARAM`]), bit `min(i, 31)` showing
//! class `i`, so bit 31 stands for class 31 and every class past it; every bit set ([`ALL`]), the default, shows
//! every class (applied per R-369). A class not shown takes the mute transform ([`mute`]; colour_composition §1.4,
//! REQ-COL-058): its OKLab lightness replaced by [`MUTE_L`] and its `a`, `b` scaled by [`MUTE_CHROMA`]. There is no
//! escaper field and no escaper render mode: the escaper view is the outcome palette filtered to the escape
//! classes.

use crate::colour::space;
use crate::present::{self, Rgb};
use ledger::schema::Scale;

/// The palettes' WGSL, placed after the presentation layer at assembly.
pub const WGSL: &str = include_str!("../../shaders/wgsl/lib/palette.wgsl");

/// The mute's OKLab lightness (colour_composition §1.4; REQ-COL-058), `CAT_MUTE_L`: proposed (R-71, REQ-COL-162), the
/// midpoint, to two places, of the presentation views' invalid colour `#141418` (L 0.193) and the "not yet" grey
/// `#4E4E4E` (L 0.424).
pub const MUTE_L: f64 = 0.31;

/// The mute's chroma factor (colour_composition §1.4; REQ-COL-058), `CAT_MUTE_CHROMA`: proposed (R-71, REQ-COL-162).
pub const MUTE_CHROMA: f64 = 0.25;

/// The minimum OKLab hue separation, in degrees, between consecutive golden-angle classes (dd_colouring §5 unit test
/// 9): proposed (R-71, REQ-COL-051) below the achieved separations of the 8-bit colours for `n` up to 96, the least
/// 136.68°, for the human to confirm at the M7 gate.
pub const MIN_HUE_SEPARATION_DEG: f64 = 135.0;

/// The class mask's param, a `u32` uniform of every categorical mode.
pub const MASK_PARAM: &str = "class_mask";

/// The class mask that shows every class, the default.
pub const ALL: u32 = u32::MAX;

/// The palette a categorical field takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Palette {
    /// colour_composition §1.4's canonical nine-class outcome palette, the occupant `outcome_state` (R-77).
    Outcome,
    /// The generated index palette of `classes` classes ([`generated`]).
    Generated { classes: u32 },
}

/// The fields the outcome palette reads, `state` and the `detail` union (colour_composition §1.4).
const OUTCOME_FIELDS: [&str; 2] = ["state", "detail"];

/// The palette of the ledger field `field`, or why it has none: not a ledger field, or not categorical.
pub fn resolve(field: &str) -> Result<Palette, String> {
    let l = ledger::payload::ledger();
    let entries = ledger::gen::validate(&l).map_err(|e| e.to_string())?;
    let e = entries
        .iter()
        .find(|e| e.name == field)
        .ok_or_else(|| format!("`{field}` is no ledger field"))?;
    match e.scale {
        Scale::Categorical(_) if OUTCOME_FIELDS.contains(&field) => Ok(Palette::Outcome),
        Scale::Categorical(classes) => Ok(Palette::Generated { classes }),
        _ => Err(format!("`{field}` is not categorical")),
    }
}

/// The golden-angle hue of class `i`, in turns: `frac(i · φ_g)` (dd_colouring §3.7).
pub fn golden_turns(i: u32) -> f64 {
    present::golden_turns(i)
}

/// `cat_palette(i, n)`: the generated palette's colour of class `i` of `n`, Okabe–Ito, cycling, for `n ≤ 8`; the
/// golden-angle hue of class `i` on `hue_wheel` for `n > 8`.
pub fn generated(i: u32, n: u32) -> Rgb {
    if n <= 8 {
        present::srgb8(present::OKABE_ITO[(i % 8) as usize])
    } else {
        present::hue_wheel(golden_turns(i))
    }
}

/// `cat_mute(c)`: the mute transform of the class colour `c`, linear RGB (colour_composition §1.4; REQ-COL-058).
pub fn mute(c: Rgb) -> Rgb {
    let [_, a, b] = space::linear_to_oklab(c);
    space::oklab_to_linear([MUTE_L, MUTE_CHROMA * a, MUTE_CHROMA * b]).map(|x| x.clamp(0.0, 1.0))
}

/// `cat_shown(class, mask)`: whether `mask` shows `class`, bit `min(class, 31)` set.
pub fn shown(class: u32, mask: u32) -> bool {
    (mask >> class.min(31)) & 1 == 1
}

/// `cat_filter(c, class, mask)`: `c` where `mask` shows `class`, else its mute.
pub fn filter(c: Rgb, class: u32, mask: u32) -> Rgb {
    if shown(class, mask) {
        c
    } else {
        mute(c)
    }
}

/// `cat_ramp(i, n, mask)`: class `i` of `n` on the generated palette, through the class mask.
pub fn ramp(i: u32, n: u32, mask: u32) -> Rgb {
    filter(generated(i, n), i, mask)
}

/// The class mask that shows exactly `classes`: bit `min(k, 31)` of each class `k`.
pub fn mask(classes: &[u32]) -> u32 {
    classes.iter().fold(0, |m, &k| m | 1 << k.min(31))
}
