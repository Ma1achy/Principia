//! Pointer picking (principia_coordinate_conventions_note.md, "Code paths that must honour the single convention",
//! path 2; trajectory_viewing §1; REQ-GUI-001; TASK-M1-07): a canvas event, Y-down from the top-left, flipped by the
//! convention's one flip ([`render::coords::frag_uv`], the Rust twin of the render's) to the post-flip UV, Y-up, before
//! the picked quad or `z` is computed from it. Lock, hover and click-inspect all call [`pick`]; the GUI (M8) calls it
//! too, so it inherits the convention rather than re-deriving it.
//!
//! At M1 a pick reports the UV, the quad of the flat grid the screen is drawn over and the screen's single
//! `ctx.chart.z`; the per-pixel `z = z₀ + (2s − 1)·q₁ + (2t − 1)·q₂` arrives with `SimConfig.plane` in M2, through
//! this same function (RQ-216).

use render::coords::frag_uv;
use render::raster::Grid;

/// What a pointer event lands on: the canvas, in the event's units, Y-down from the top-left, and the screen drawn
/// over it, its grid of quads and the one `ctx.chart.z` it is drawn at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Screen {
    /// The canvas's `(width, height)`, in the units of the events' positions.
    pub canvas: [f64; 2],
    /// The flat grid of quads the screen is drawn over.
    pub grid: Grid,
    /// The screen's `ctx.chart.z`, the 8-D latent.
    pub z: [f32; 8],
}

/// A pick: the post-flip UV, the quad it falls in, by index and by `(column, row)` from the bottom-left, and the `z`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pick {
    pub uv: [f64; 2],
    pub quad: u32,
    pub quad_xy: [u32; 2],
    pub z: [f32; 8],
}

/// The pick at the canvas position `at` (Y-down, from the top-left, as a pointer event gives it), or `None` outside the
/// canvas, its edges included in it: the UV through the convention's flip (`v = 1 − y/H`), the quad of the grid that
/// UV falls in (half-open, the top and right edges in the last row and column, as the raster puts each pixel), and the
/// screen's `z`.
pub fn pick(screen: &Screen, at: [f64; 2]) -> Option<Pick> {
    let inside = (0..2).all(|k| (0.0..=screen.canvas[k]).contains(&at[k]));
    if !inside {
        return None;
    }
    let uv = frag_uv(at, screen.canvas);
    let quads = screen.grid.quads;
    let quad_xy = [0, 1].map(|k| ((uv[k] * f64::from(quads[k])) as u32).min(quads[k] - 1));
    Some(Pick {
        uv,
        quad: quad_xy[1] * quads[0] + quad_xy[0],
        quad_xy,
        z: screen.z,
    })
}
