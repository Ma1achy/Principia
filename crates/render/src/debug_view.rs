//! View › Debug view's CPU twin (R-467 clause 3, D5; R-470 G4; REQ-COL-109): what the debug variant's arm,
//! `shaders/wgsl/stain/debug_view.wgsl` ([`crate::assemble::DEBUG_ARM`]), draws over the active stain at one sample,
//! and the counts it keeps for the legend's "Debug" key (render_gui_spec §G6).
//!
//! **The order** (R-467 D5): the chart's classification first, so a pixel the chart classifies as not a system
//! (`ctx.validity.forbidden`, REQ-COL-102) draws the hatch, `debug_invalid`, whatever its payload; then
//! non-finiteness, so a non-finite sample draws the NaN / Inf style, `debug_nan_inf` (REQ-COL-110); then the failure, so
//! any other failure draws `debug_invalid`; every other sample, a system, exactly as the stain draws it.
//!
//! **Non-finite** (payload §2): a `sim_failed` of detail 0 (NaN in state), 1 (Inf or overflow in state) or 2 (a
//! non-finite derived quantity); a `decode_failed` of detail 0 (a non-finite decode output); and a sample whose stain
//! read a non-finite value (R-436), which the stain notes (`note_non_finite()`; a field ramp's invalid lane). **A
//! failure** is every state but escape, bounded, collision and running: `sim_failed` of detail 3, `decode_failed` of
//! detail 1, 2 or 3, and the reserved `state` codes. A `running` sample is drawn as the stain draws it.
//!
//! **The counts** are raw pixel counts, the figure's pixels drawn in each category, not R-408's area statistics.

use ledger::display::State;

use crate::present::{self, Rgb};

/// A category of the "Debug" key (render_gui_spec §G6), in the counts' order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    /// "forbidden (chart)": a pixel the chart classifies as not a system, in the hatch.
    Forbidden = 0,
    /// "decode or sim failure": any other failure, in `debug_invalid`'s style.
    Failure = 1,
    /// "NaN / Inf": a non-finite sample, in the NaN / Inf style.
    NanInf = 2,
}

impl Category {
    /// The three, in the counts' order.
    pub const ALL: [Category; 3] = [Category::Forbidden, Category::Failure, Category::NanInf];

    /// The key entry's label (R-467 D5; render_gui_spec §G6).
    pub fn label(self) -> &'static str {
        match self {
            Category::Forbidden => "forbidden (chart)",
            Category::Failure => "decode or sim failure",
            Category::NanInf => "NaN / Inf",
        }
    }

    /// The category's style at the pixel `frag_xy`: the hatch for the first two, the NaN / Inf style for the third.
    pub fn colour(self, frag_xy: [f64; 2]) -> Rgb {
        match self {
            Category::Forbidden | Category::Failure => present::debug_invalid(frag_xy),
            Category::NanInf => present::debug_nan_inf(frag_xy),
        }
    }
}

/// A sample as the debug view reads it: its chart classification, its `state` and `detail`, and whether the stain read
/// a non-finite value at it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sample {
    pub forbidden: bool,
    pub state: u32,
    pub detail: u32,
    pub stain_non_finite: bool,
}

/// Whether a sample with `state` and `detail` is non-finite by its payload (payload §2): a `sim_failed` of detail 0, 1
/// or 2, or a `decode_failed` of detail 0.
pub fn payload_non_finite(state: u32, detail: u32) -> bool {
    (state == State::SimFailed.code() && detail <= 2)
        || (state == State::DecodeFailed.code() && detail == 0)
}

/// Whether a sample with `state` is a failure: no physical outcome and not running.
pub fn payload_failed(state: u32) -> bool {
    ![
        State::Escape,
        State::Bounded,
        State::Collision,
        State::Running,
    ]
    .iter()
    .any(|s| s.code() == state)
}

/// The category the debug view draws `s` in, or `None` for a system pixel, drawn as the stain draws it: the chart
/// first, then non-finiteness, then the failure (R-467 D5).
pub fn category(s: Sample) -> Option<Category> {
    if s.forbidden {
        Some(Category::Forbidden)
    } else if s.stain_non_finite || payload_non_finite(s.state, s.detail) {
        Some(Category::NanInf)
    } else if payload_failed(s.state) {
        Some(Category::Failure)
    } else {
        None
    }
}

/// What the debug view draws at the pixel `frag_xy` for `s`, the stain drawing `stain` there.
pub fn colour(s: Sample, stain: Rgb, frag_xy: [f64; 2]) -> Rgb {
    category(s).map_or(stain, |c| c.colour(frag_xy))
}

/// The counts of the figure's pixels drawn in each category, in [`Category::ALL`]'s order, for pixels whose samples
/// are `pixels`.
pub fn counts(pixels: impl IntoIterator<Item = Sample>) -> [u32; 3] {
    let mut out = [0; 3];
    for c in pixels.into_iter().filter_map(category) {
        out[c as usize] += 1;
    }
    out
}
