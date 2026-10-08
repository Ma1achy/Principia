//! The coordinate convention's one flip, on the CPU (principia_coordinate_conventions_note.md, "The one-line rule";
//! TASK-M1-07). There is one internal orientation, bottom-left origin and Y-up, and exactly one flip, at the
//! framebuffer → UV boundary: `v = 1 − frag_coord.y/H`. The framebuffer is Y-down, its row 0 at the top (frag_coord,
//! canvas and pointer events, the read-back image); everything after the flip is Y-up.
//!
//! [`flip_y`] is the Rust twin of the WGSL flip in `shaders/wgsl/lib/coords.wgsl` ([`WGSL`]): the same flip, named
//! the same, one per language (RQ-211). Picking (`engine::picking`) and the CPU raster ([`crate::raster::Grid`]) call
//! it, and the WGSL raster calls its WGSL twin, so every coordinate consumer reads the post-flip coordinate and they
//! agree by construction (the note's "Code paths that must honour the single convention"). `flip_twin_matches_wgsl`
//! renders the WGSL flip at every row of a target and holds the twin equal to it.
//!
//! The image export ([`crate::export`]) takes the headless readback, already display-oriented (rows from the top), and
//! reverses no rows: the note's path-5 flip fires only for a Y-up internal image, and the readback is not one
//! (REQ-SYS-080; RQ-212).

use std::ops::{Div, Sub};

/// The WGSL flip and its UV map, `flip_y(y, height)` and `frag_uv(frag, dims)`: the shared library's
/// `shaders/wgsl/lib/coords.wgsl`, which the raster prepends ([`crate::raster::WGSL`]) and the `m1-coords` golden case
/// prepends by path.
pub const WGSL: &str = include_str!("../shaders/wgsl/lib/coords.wgsl");

/// The WGSL file, relative to the workspace root.
pub const WGSL_PATH: &str = "crates/render/shaders/wgsl/lib/coords.wgsl";

/// **The convention flip** (the Rust twin of `coords.wgsl`'s `flip_y`): a framebuffer `y` (Y-down, measured from the
/// top edge) of a target `height` high, as the Y-up `y` measured from the bottom edge, `height − y`. On the framebuffer's
/// edges it maps edge `y` to edge `height − y`, so pixel row `y` (between edges `y` and `y + 1`) is Y-up row
/// `flip_y(y + 1, height)`; on a point it maps a pixel row's centre `y + 0.5` to its Y-up centre; and `v = 1 − y/H` is
/// `flip_y(y, H) / H` ([`frag_uv`]). It is generic so the raster can flip integer rows and picking an `f64` event
/// with the one function.
pub fn flip_y<T: Sub<Output = T>>(y: T, height: T) -> T {
    height - y
}

/// The post-flip UV of the framebuffer point `frag` (a pixel's centre, or a canvas event's position, in pixels from
/// the top-left) in a target or canvas of `dims` pixels: `u = x/W`, `v = flip_y(y, H)/H = 1 − y/H`, bottom-left origin,
/// Y-up (the Rust twin of `coords.wgsl`'s `frag_uv`).
pub fn frag_uv<T: Copy + Sub<Output = T> + Div<Output = T>>(frag: [T; 2], dims: [T; 2]) -> [T; 2] {
    [frag[0] / dims[0], flip_y(frag[1], dims[1]) / dims[1]]
}

/// The UV preset's banding criterion's bound (REQ-TOOL-152, an R-71 calibration): a run of reconstructed UV
/// coordinates is smooth, not step-quantised, when every adjacent-sample delta departs from the exact step `2h/N` by
/// at most this fraction of it ([`banding_departure`]). **Proposed, R-71**: confirmed by the human at the M1 gate and
/// recorded in decisions.md. Each reconstructed f32 coordinate is within half an ulp of its value, so a delta departs
/// by at most one ulp, and the criterion holds while the step spans at least `1 / BANDING_BOUND` = 16 ulps.
pub const BANDING_BOUND: f64 = 0.0625;

/// Where [`BANDING_BOUND`] stands.
pub const BANDING_BOUND_STATUS: &str =
    "proposed, R-71 (REQ-TOOL-152), awaiting the human's confirmation at the M1 gate";

/// The largest departure of `values`' adjacent deltas from the exact step `step`, as a fraction of it: the maximum over
/// adjacent pairs of `|(values[k + 1] − values[k]) − step| / |step|`; 0 for fewer than two values. A run is banded,
/// step-quantised, when this exceeds [`BANDING_BOUND`] ([`banded`]).
pub fn banding_departure(values: &[f64], step: f64) -> f64 {
    values
        .windows(2)
        .map(|w| ((w[1] - w[0]) - step).abs() / step.abs())
        .fold(0.0, f64::max)
}

/// Whether `values`, adjacent samples whose exact step is `step`, are banded: their departure ([`banding_departure`])
/// exceeds [`BANDING_BOUND`].
pub fn banded(values: &[f64], step: f64) -> bool {
    banding_departure(values, step) > BANDING_BOUND
}
