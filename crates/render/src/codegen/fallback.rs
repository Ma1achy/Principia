//! The invalid-graph fallback (render_gui_spec §13, "Always-renderable"): "A genuinely invalid graph (should be
//! unreachable given the fixed OUT+combiner and identity rules) shows a **defined fallback** (flat grey / error tint),
//! never a crash." The fallback is one flat colour, an error tint, and the generated `shade()` of a graph the
//! assembler refuses returns it at every pixel ([`crate::assemble::assemble_or_fallback`]): there is no node to walk,
//! so `shade()`'s body is the one statement [`statement`] gives, in the place the combiner's would take
//! ([`super::combine::statement`]), and the rest of the template is the assembler's own.
//!
//! **The colour is proposed (R-71; REQ-RENDER-081):** [`SRGB8`], `#00FF00`, which the human confirms at the M7 gate.
//! §13 leaves open which of "flat grey / error tint" it is, and its value. An error tint is proposed because a grey
//! sits beside the both-None mid-grey, `OKLab(0.6, 0, 0)` (sRGB `#808080`), a state that is not an error. It must be
//! distinguishable from that mid-grey, from the invalid-pixel hatch (`#9B00FF` and `#48FFFF`, R-132) and from the
//! presentation view's `#141418` (R-467; R-814), by a colour distance to each. `#00FF00`'s Euclidean distances in
//! OKLab are 0.397 to the mid-grey, 0.657 and 0.241 to the hatch's two colours and 0.738 to `#141418`; of the sRGB
//! lattice of step 15 it is the colour farthest from the nearest of every reserved colour (those four, the "not yet"
//! style's two greys, the NaN / Inf style's two colours, the fallback tint `#FF69FF`, the outcome palette's two sets,
//! the Okabe–Ito palette, the canvas background, black and white), 0.193 from the nearest, Okabe–Ito's yellow
//! `#F0E442`. `crates/render/tests/stain_graph.rs` measures the four required distances.

use crate::present;

/// The fallback's colour, 8-bit sRGB: `#00FF00` (proposed, R-71; REQ-RENDER-081).
pub const SRGB8: [u8; 3] = [0x00, 0xff, 0x00];

/// The fallback's colour as linear RGB, the colour slot's space (render contract Part 2): each channel the f32
/// nearest the sRGB decode of its 8-bit code, as `present.wgsl`'s `PRESENT_INVALID` is written.
pub fn linear() -> [f32; 3] {
    present::srgb8(SRGB8).map(|c| c as f32)
}

/// The statement `shade()` gives an invalid graph's output by: `var out = <the fallback's colour>;`, its channels
/// written as linear f32 literals, so the fragment returns the same bits on every backend, and a comment naming it.
pub fn statement() -> String {
    let [r, g, b] = linear();
    let [r8, g8, b8] = SRGB8;
    format!(
        "var out = vec3<f32>({r:?}, {g:?}, {b:?}); // an invalid graph: the defined fallback, the flat error tint \
         #{r8:02X}{g8:02X}{b8:02X} (render_gui_spec §13)"
    )
}
