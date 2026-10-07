//! Pointer picking and the harness's quad frames (TASK-M1-07; principia_coordinate_conventions_note.md path 2;
//! debug_tooling_plan §F's picking cross-check):
//! - REQ-GUI-001, REQ-TOOL-027: a click at each known corner of the canvas, through the function the GUI will call,
//!   reports that corner's UV (top-left → v ≈ 1, bottom-left → v ≈ 0), its quad and the screen's `ctx.chart.z`
//!   (`picking_corners*`); a pick at each pixel's centre lands in the quad the raster draws there, and one off the
//!   canvas is none (`picking_*`);
//! - REQ-TOOL-019 (RQ-214): the synthetic harness's per-quad centre and half-width from its own grid, deep_zoom §1's
//!   `c` and `h = 2^−(ℓ+1)`, and the f32 frames it uploads (`synthetic_quad_frame_*`).
//!
//! Each test registers its negative control (R-176).

use engine::picking::{pick, Pick, Screen};
use engine::synthetic::{QuadFrame, Synthetic};
use render::raster::Grid;
use validation::negative_control;

/// 4 × 2 quads of 3 × 3 tiles, 5 px square: a 60 × 30 target.
fn grid() -> Grid {
    Grid::new([4, 2], 3, 0, 5).expect("the picking grid")
}

/// The screen: a canvas of 120 × 60 event units (twice the target, as a scaled display), the grid, and a `z`.
fn screen() -> Screen {
    Screen {
        canvas: [120.0, 60.0],
        grid: grid(),
        z: [0.5, -0.25, 0.125, 1.0, 2.0, -3.0, 0.75, 0.0625],
    }
}

/// A corner: its name, canvas position (Y-down from the top-left), the UV it must report, and its quad's
/// `(column, row)` from the bottom-left.
type Corner = (&'static str, [f64; 2], [f64; 2], [u32; 2]);

/// The four corners: name, canvas position (Y-down from the top-left), the UV it must report, and its quad's
/// `(column, row)` from the bottom-left.
fn corners(s: &Screen) -> [Corner; 4] {
    let [w, h] = s.canvas;
    let [qx, qy] = s.grid.quads;
    [
        ("top-left", [0.0, 0.0], [0.0, 1.0], [0, qy - 1]),
        ("bottom-left", [0.0, h], [0.0, 0.0], [0, 0]),
        ("top-right", [w, 0.0], [1.0, 1.0], [qx - 1, qy - 1]),
        ("bottom-right", [w, h], [1.0, 0.0], [qx - 1, 0]),
    ]
}

/// Each corner click reports its UV, its quad (index `row · columns + column`) and the screen's `z`.
fn check_corners(s: &Screen, pick: impl Fn(&Screen, [f64; 2]) -> Option<Pick>) {
    for (name, at, uv, quad_xy) in corners(s) {
        let p = pick(s, at).unwrap_or_else(|| panic!("the {name} corner is on the canvas"));
        assert_eq!(p.uv, uv, "a {name} click reports UV {:?}, not {uv:?}", p.uv);
        assert_eq!(
            p.quad_xy, quad_xy,
            "a {name} click picks quad {:?}",
            p.quad_xy
        );
        assert_eq!(
            p.quad,
            quad_xy[1] * s.grid.quads[0] + quad_xy[0],
            "a {name} click's quad index"
        );
        assert_eq!(p.z, s.z, "a {name} click reports the screen's z");
    }
}

#[test]
fn picking_corners_report_their_uv_quad_and_z() {
    check_corners(&screen(), pick);
}

negative_control!(
    picking_corners_report_their_uv_quad_and_z,
    "a pick that skips the flip, reading the canvas Y-down, reports the top-left click at v = 0",
    expected = "a top-left click reports UV [0.0, 0.0]",
    check_corners(&screen(), |s, at| {
        pick(s, at).map(|p| Pick {
            uv: [at[0] / s.canvas[0], at[1] / s.canvas[1]],
            ..p
        })
    })
);

/// A pick at each pixel's centre (in canvas units) lands in the quad the raster puts that pixel in (`Grid::cell`), so
/// picking and the render agree on the flip.
fn check_agrees_with_raster(s: &Screen, pick: impl Fn(&Screen, [f64; 2]) -> Option<Pick>) {
    let (width, height) = s.grid.target();
    let scale = [
        s.canvas[0] / f64::from(width),
        s.canvas[1] / f64::from(height),
    ];
    for y in 0..height {
        for x in 0..width {
            let at = [
                (f64::from(x) + 0.5) * scale[0],
                (f64::from(y) + 0.5) * scale[1],
            ];
            let cell = s.grid.cell(x, y);
            let p = pick(s, at).expect("a pixel centre is on the canvas");
            assert_eq!(
                (p.quad, p.quad_xy),
                (cell.quad, cell.quad_xy),
                "pixel ({x}, {y}): picking lands in quad {:?}; the raster draws quad {:?} there",
                p.quad_xy,
                cell.quad_xy
            );
        }
    }
}

#[test]
fn picking_agrees_with_the_raster_at_every_pixel() {
    check_agrees_with_raster(&screen(), pick);
}

negative_control!(
    picking_agrees_with_the_raster_at_every_pixel,
    "a pick mirrored in Y picks the quad mirrored from the raster's",
    expected = "the raster draws quad",
    check_agrees_with_raster(&screen(), |s, at| pick(s, [at[0], s.canvas[1] - at[1]]))
);

/// The canvas is closed: its edges pick, and a position a little past any edge, or not a number, picks nothing. A
/// position on the boundary between two quads picks the upper or right one; the canvas's top and right edges pick
/// the last row and column.
fn check_edges(s: &Screen, pick: impl Fn(&Screen, [f64; 2]) -> Option<Pick>) {
    let [w, h] = s.canvas;
    let e = 1e-9;
    for at in [
        [-e, 0.0],
        [0.0, -e],
        [w + e, 0.0],
        [0.0, h + e],
        [f64::NAN, 1.0],
        [1.0, f64::NAN],
    ] {
        assert_eq!(pick(s, at), None, "{at:?} is off the canvas, yet picks");
    }
    for at in [[0.0, 0.0], [w, h], [w / 2.0, h / 2.0]] {
        assert!(
            pick(s, at).is_some(),
            "{at:?} is on the canvas, yet picks nothing"
        );
    }
    // Quad columns are 30 canvas units wide and rows 30 high: x = 30 is the boundary of columns 0 and 1, and y = 30
    // (v = ½) that of rows 0 and 1.
    let at = |x: f64, y: f64| pick(s, [x, y]).expect("on the canvas").quad_xy;
    assert_eq!(
        at(30.0, 45.0),
        [1, 0],
        "the column boundary picks the right-hand quad"
    );
    assert_eq!(
        at(30.0 - e, 45.0),
        [0, 0],
        "just left of the boundary picks the left"
    );
    assert_eq!(
        at(10.0, 30.0),
        [0, 1],
        "the row boundary picks the upper quad"
    );
    assert_eq!(
        at(10.0, 30.0 + e),
        [0, 0],
        "just below the boundary picks the lower"
    );
    assert_eq!(
        at(w - e, h - e),
        [3, 0],
        "just inside the bottom-right corner"
    );
}

#[test]
fn picking_edges_and_quad_boundaries() {
    check_edges(&screen(), pick);
}

negative_control!(
    picking_edges_and_quad_boundaries,
    "a pick that accepts any position picks off the canvas",
    expected = "is off the canvas, yet picks",
    check_edges(&screen(), |s, at| {
        pick(
            s,
            [at[0].clamp(0.0, s.canvas[0]), at[1].clamp(0.0, s.canvas[1])],
        )
        .or_else(|| pick(s, [0.0, 0.0]))
    })
);

// ── The harness's quad frames (REQ-TOOL-019, RQ-214) ────────────────────────────────────────────────────────────

/// Quad `q` of `set` (grid columns `columns`, depth `depth`, origin `origin`) has `c = (origin + (i, j) + ½)·2^−ℓ` and
/// `h = 2^−(ℓ+1)`, and the bytes it uploads hold each quad's `(c_u, c_v, h_u, h_v)` as f32, in quad order.
fn check_frames(
    set: &Synthetic,
    depth: u32,
    origin: [u64; 2],
    frame: impl Fn(&Synthetic, u32) -> QuadFrame,
) {
    let g = set.grid();
    let h = 0.5f64.powi(depth as i32 + 1);
    let bytes = set.bytes().quad_frame;
    assert_eq!(
        bytes.len(),
        16 * g.quad_count() as usize,
        "four f32 per quad"
    );
    for q in 0..g.quad_count() {
        let (i, j) = (q % g.quads[0], q / g.quads[0]);
        let want = QuadFrame {
            c: [
                (origin[0] as f64 + f64::from(i) + 0.5) * 2.0 * h,
                (origin[1] as f64 + f64::from(j) + 0.5) * 2.0 * h,
            ],
            h: [h, h],
        };
        let got = frame(set, q);
        assert_eq!(
            got, want,
            "quad {q} ({i}, {j}): frame {got:?}, not {want:?}"
        );
        let words: Vec<f32> = bytes[16 * q as usize..16 * (q as usize + 1)]
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .collect();
        let as_f32 = [want.c[0], want.c[1], want.h[0], want.h[1]].map(|x| x as f32);
        assert_eq!(words, as_f32, "quad {q}'s uploaded frame");
    }
}

#[test]
fn synthetic_quad_frame_from_the_grid() {
    let g = Grid::new([3, 2], 2, 1, 1).expect("a frame grid");
    check_frames(&Synthetic::flat(g, 4), 4, [0, 0], Synthetic::quad_frame);
    check_frames(
        &Synthetic::flat_at(g, 7, [5, 9]),
        7,
        [5, 9],
        Synthetic::quad_frame,
    );
    let deep = Grid::new([1, 1], 2, 0, 1).expect("a deep grid");
    let cell = 3 << 28;
    check_frames(
        &Synthetic::flat_at(deep, 30, [cell, cell + 1]),
        30,
        [cell, cell + 1],
        Synthetic::quad_frame,
    );
}

negative_control!(
    synthetic_quad_frame_from_the_grid,
    "a frame read Y-down, its row counted from the top, fails",
    expected = "frame",
    {
        let g = Grid::new([3, 2], 2, 1, 1).expect("a frame grid");
        check_frames(&Synthetic::flat(g, 4), 4, [0, 0], |set, q| {
            let rows = set.grid().quads[1];
            let flipped = (rows - 1 - q / 3) * 3 + q % 3;
            set.quad_frame(flipped)
        })
    }
);
