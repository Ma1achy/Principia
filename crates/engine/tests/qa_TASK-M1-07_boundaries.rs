//! QA tests for TASK-M1-07, REQ-GUI-001 at quad boundaries: picking flips canvas coordinates the same way as the render
//! before computing the picked quad. The raster puts each pixel in the quad whose half-open Y-up interval
//! [j/q, (j + 1)/q) holds it, so a click exactly on a quad edge, in canvas units, is in the quad above it (or right of
//! it), and the canvas's top and right edges in the last row and column. Checked over many canvas sizes, quad counts
//! and every interior edge, with the expected quad derived in exact integers, not from the implementation.
//!
//! Each test registers its negative control (R-176).

use engine::picking::{pick, Pick, Screen};
use render::raster::Grid;
use validation::negative_control;

type Picker = fn(&Screen, [f64; 2]) -> Option<Pick>;

/// For canvases `size` × `size` with `size` from 1 to 512 and `q` × `q` quads, `q` from 1 to 64, whenever `q` divides
/// `size`: a click at the edge `k·size/q` from the top (and from the left) is in Y-up row `q − k` (column `k`), each
/// clamped to `q − 1`.
fn check_boundaries(picker: Picker) {
    for q in 1u32..=64 {
        let grid = Grid::new([q, q], 2, 0, 1).expect("grid");
        for size in (1u32..=512).filter(|s| s % q == 0) {
            let s = Screen {
                canvas: [f64::from(size), f64::from(size)],
                grid,
                z: [0.0; 8],
            };
            for k in 0..=q {
                let edge = f64::from(size / q * k);
                let p = picker(&s, [edge, edge]).expect("an edge is on the canvas");
                let want = [k.min(q - 1), (q - k).min(q - 1)];
                assert_eq!(
                    p.quad_xy, want,
                    "canvas {size}, {q} quads: a click on edge {k} (at {edge}, {edge}) picks quad {:?}, not {want:?}",
                    p.quad_xy
                );
            }
        }
    }
}

#[test]
fn qa_picking_quad_boundaries() {
    check_boundaries(pick);
}

negative_control!(
    qa_picking_quad_boundaries,
    "a quad taken from the UV after dividing, floor(uv·q), rounds some edges to the quad below",
    expected = "a click on edge",
    check_boundaries(|s, at| {
        pick(s, at).map(|p| {
            let quads = s.grid.quads;
            let quad_xy =
                [0, 1].map(|k| ((p.uv[k] * f64::from(quads[k])) as u32).min(quads[k] - 1));
            Pick {
                quad_xy,
                quad: quad_xy[1] * quads[0] + quad_xy[0],
                ..p
            }
        })
    })
);
