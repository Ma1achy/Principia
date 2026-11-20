//! The mock's invalid-pixel test hook, counted and warned of (R-467 D5, D7, D8; REQ-GUI-225, REQ-GUI-226). The hook
//! flags a region `[s₀, t₀, s₁, t₁)` of the field ([`super::canvas::MockCanvas::flag_invalid`]), past `[0,1]²` or inside
//! it, and the canvas draws its pixels as not part of the chart. Here the mock finds which of the figure's pixels the
//! region covers, as the canvas's shader decides it (`noise.wgsl`'s `forbidden`, in `f32`, at each pixel's centre):
//! their count is the debug view's "forbidden (chart)" count, and those inside `[0,1]²` are a decoder bug, which the
//! console is warned of once per view (D7), naming the first of them in row order from the bottom-left.
//!
//! A pixel is named by its column `x` from the viewport's left and its row `y` up from the viewport's bottom, so the
//! first pixel in row order from the bottom-left is the one with the least `y`, then the least `x` (applied per R-369:
//! D7 names the pixel and the order, not the frame its coordinates are in).

use crate::side::FigureView;

/// The flagged pixels inside `[0,1]²` of one view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Inside {
    /// How many there are.
    pub count: u64,
    /// The first in row order from the bottom-left: `(x, y)`, `y` up from the viewport's bottom.
    pub pixel: [u32; 2],
    /// That pixel's field point `(s, t)`, at its centre.
    pub st: [f32; 2],
}

/// What the hook flags in one view of the figure.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Flagged {
    /// The figure's pixels: those its draw is clipped to.
    pub figure: u64,
    /// The flagged pixels among them, drawn as not part of the chart.
    pub flagged: u64,
    /// The flagged pixels inside `[0,1]²`, if any.
    pub inside: Option<Inside>,
}

/// The pixels along one axis a region covers: each pixel's index, its field coordinate and whether it lies in `[0, 1]`.
type Covered = Vec<(u32, f32, bool)>;

/// The pixels along one axis that the region `[lo, hi)` covers, from `first` to `end` (exclusive), the axis `size`
/// pixels long and the shown rect at `centre ± half` in its normalized device coordinates; `flip` for the vertical
/// axis, whose rows run down while the coordinates run up. Each pixel is judged at its centre, in `f32`, as the shader
/// judges it: its field coordinate `xy = (ndc − centre) / half`, its `s = (xy + 1) / 2`.
fn covered(
    (first, end): (u32, u32),
    size: u32,
    (centre, half): (f32, f32),
    (lo, hi): (f32, f32),
    flip: bool,
) -> Covered {
    (first..end)
        .filter_map(|i| {
            let uv = (i as f32 + 0.5) / size as f32;
            let ndc = if flip { 1.0 - uv * 2.0 } else { uv * 2.0 - 1.0 };
            let xy = (ndc - centre) / half;
            let s = (xy + 1.0) * 0.5;
            (s >= lo && s < hi).then_some((i, s, xy.abs() <= 1.0))
        })
        .collect()
}

/// The columns and the rows of the figure `view` that `region` covers, each with its field coordinate and whether it
/// lies in `[0, 1]`, rows from the top: the flagged pixels are their product.
fn covered_axes(view: &FigureView, region: [f32; 4]) -> (Covered, Covered) {
    let [w, h] = view.size;
    let [x0, y0, x1, y1] = view.clip;
    let p = view.placement;
    let columns = covered(
        (x0, x1),
        w,
        (p.centre[0], p.half[0]),
        (region[0], region[2]),
        false,
    );
    let rows = covered(
        (y0, y1),
        h,
        (p.centre[1], p.half[1]),
        (region[1], region[3]),
        true,
    );
    (columns, rows)
}

/// The columns and the rows, from the top, of the figure `view` that `region` covers: the pixels the canvas draws as
/// not part of the chart are their product.
pub fn covered_pixels(view: &FigureView, region: [f32; 4]) -> (Vec<u32>, Vec<u32>) {
    let (columns, rows) = covered_axes(view, region);
    (
        columns.into_iter().map(|c| c.0).collect(),
        rows.into_iter().map(|r| r.0).collect(),
    )
}

/// What `region` flags in the figure `view`.
pub fn flagged(view: &FigureView, region: [f32; 4]) -> Flagged {
    let [x0, y0, x1, y1] = view.clip;
    let figure = u64::from(x1.saturating_sub(x0)) * u64::from(y1.saturating_sub(y0));
    let h = view.size[1];
    let (columns, rows) = covered_axes(view, region);
    let flagged = columns.len() as u64 * rows.len() as u64;
    let columns_in: Vec<_> = columns.iter().filter(|c| c.2).collect();
    let rows_in: Vec<_> = rows.iter().filter(|r| r.2).collect();
    let count = columns_in.len() as u64 * rows_in.len() as u64;
    // Row order from the bottom-left: the bottom row, the lowest on screen, is the last row down; then its leftmost.
    let inside = columns_in
        .first()
        .zip(rows_in.last())
        .map(|(&&(x, s, _), &&(row, t, _))| Inside {
            count,
            pixel: [x, h - 1 - row],
            st: [s, t],
        });
    Flagged {
        figure,
        flagged,
        inside,
    }
}

/// The console's warning of a decoder bug for `inside`, the chart named `chart` (R-467 D7; render_gui_spec §G12).
pub fn warning(chart: &str, inside: &Inside) -> String {
    let [x, y] = inside.pixel;
    let [s, t] = inside.st;
    format!(
        "decoder bug: chart {chart} fails validity inside [0,1]² at pixel ({x}, {y}), (s, t) = ({s:.4}, {t:.4}); {} \
         pixels in this view",
        inside.count
    )
}
