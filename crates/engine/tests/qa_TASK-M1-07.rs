//! QA tests for TASK-M1-07, written from REQ-GUI-001 and REQ-TOOL-027 (coordinate note, path 2 and "The catch";
//! debug_tooling_plan §F's picking cross-check), not from the implementation:
//! - a click at each known corner, through `engine::picking::pick`, the function the GUI will call, reports that
//!   corner's UV (top-left → v ≈ 1, bottom-left → v ≈ 0), the quad there and the screen's `ctx.chart.z`, on a canvas
//!   whose size is not the target's;
//! - the cross-check against the render: a pick at every pixel's centre reports the UV the GPU's `uv_screen` preset
//!   draws at that pixel, and the quad whose deep_zoom §1 frame (the harness's c ± h) holds that UV, so picking and
//!   the render read the one flip.
//!
//! Each test registers its negative control (R-176).

use engine::picking::{pick, Pick, Screen};
use engine::presets::uv_screen;
use engine::synthetic::Synthetic;
use render::assemble::{assemble, Tier};
use render::bind::{self, Context};
use render::headless::{self, Draw, Image, Target};
use render::raster::Grid;
use validation::gpu::GpuHarness;
use validation::negative_control;

/// 4 × 4 quads of 2 × 2 tiles, 3 px square, at depth 2 from the slice's origin: they tile the slice, so quad
/// (i, j)'s frame is [i/4, (i + 1)/4] × [j/4, (j + 1)/4]. A 24 × 24 target.
fn the_set() -> Synthetic {
    Synthetic::flat_at(Grid::new([4, 4], 2, 0, 3).expect("grid"), 2, [0, 0])
}

/// The canvas is 37.5 × 37.5 event units, not the target's 24 px: a scaled display.
fn screen(grid: Grid) -> Screen {
    Screen {
        canvas: [37.5, 37.5],
        grid,
        z: [0.1, -0.2, 0.3, -0.4, 0.5, -0.6, 0.7, -0.8],
    }
}

type Picker = fn(&Screen, [f64; 2]) -> Option<Pick>;

/// The four corners of the canvas, Y-down event positions: each reports its corner's UV (to 1e-12), the quad in that
/// corner (column, Y-up row) and the screen's z.
fn check_corners(picker: Picker) {
    let s = screen(the_set().grid());
    let [w, h] = s.canvas;
    let [qx, qy] = s.grid.quads;
    for (name, at, uv, quad_xy) in [
        ("top-left", [0.0, 0.0], [0.0, 1.0], [0, qy - 1]),
        ("bottom-left", [0.0, h], [0.0, 0.0], [0, 0]),
        ("top-right", [w, 0.0], [1.0, 1.0], [qx - 1, qy - 1]),
        ("bottom-right", [w, h], [1.0, 0.0], [qx - 1, 0]),
    ] {
        let p = picker(&s, at).unwrap_or_else(|| panic!("the {name} corner picks nothing"));
        assert!(
            (p.uv[0] - uv[0]).abs() <= 1e-12 && (p.uv[1] - uv[1]).abs() <= 1e-12,
            "a {name} click reports UV {:?}, not {uv:?}",
            p.uv
        );
        assert_eq!(p.quad_xy, quad_xy, "a {name} click picks the wrong quad");
        assert_eq!(
            p.quad,
            quad_xy[1] * qx + quad_xy[0],
            "a {name} click's quad index"
        );
        assert_eq!(p.z, s.z, "a {name} click's z is not the screen's");
    }
}

#[test]
fn qa_picking_corners() {
    check_corners(pick);
}

negative_control!(
    qa_picking_corners,
    "a pick that flips the canvas twice, reading it Y-down, reports the top-left at v = 0",
    expected = "a top-left click reports UV",
    check_corners(|s, at| pick(s, [at[0], s.canvas[1] - at[1]]))
);

fn context(grid: Grid) -> Context {
    Context {
        dt_macro: 0.01,
        delta_0: 1e-6,
        n_renorm: 16,
        horizon_steps: 1000,
        z: [0.0; 8],
        grid,
        chart_id: 0,
        time: 0.0,
        ensemble_spread: 0.0,
        out_of_chart: false,
        valid_sample_count: grid.n * grid.n,
    }
}

/// The `uv_screen` preset over `set`, rendered on the GPU.
fn render_uv_screen(set: &Synthetic) -> Image {
    let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let graph = uv_screen().unwrap_or_else(|e| panic!("{e}"));
    let stain = graph.lower().unwrap_or_else(|e| panic!("{e}"));
    let fragment = assemble(&stain, Tier::FULL).unwrap_or_else(|e| panic!("{e}"));
    let module = bind::preset_module(&fragment.source);
    let ctx = context(set.grid());
    let bytes = set.bytes();
    let bound = bind::upload(h.device(), &bytes.payload(), &ctx);
    let (width, height) = ctx.grid.target();
    headless::render(
        h.device(),
        h.queue(),
        &Draw {
            module: &module,
            entry: bind::PRESET_ENTRY,
            layouts: &bound.layout_refs(),
            groups: &bound.group_refs(),
        },
        Target {
            width,
            height,
            format: wgpu::TextureFormat::Rgba32Float,
        },
    )
    .unwrap_or_else(|e| panic!("{e}"))
}

fn rg(image: &Image, x: u32, y: u32) -> [f64; 2] {
    let t = image.texel(x, y);
    [0, 1].map(|k| {
        f64::from(f32::from_le_bytes([
            t[4 * k],
            t[4 * k + 1],
            t[4 * k + 2],
            t[4 * k + 3],
        ]))
    })
}

/// Four f32 ulps at 1: the render's UV, rounded to f32 with fast-math division (R-297). A pixel is 1/24 of the UV.
const UV_TOL: f64 = 4.0 * f32::EPSILON as f64;

/// At every pixel's centre, in canvas units, the pick's UV is the one the GPU drew there, and its quad's frame
/// (c ± h, the harness's) holds that UV.
fn check_cross(picker: Picker) {
    let set = the_set();
    let image = render_uv_screen(&set);
    let s = screen(set.grid());
    let (w, h) = s.grid.target();
    let scale = [s.canvas[0] / f64::from(w), s.canvas[1] / f64::from(h)];
    for y in 0..h {
        for x in 0..w {
            let at = [
                (f64::from(x) + 0.5) * scale[0],
                (f64::from(y) + 0.5) * scale[1],
            ];
            let p = picker(&s, at).expect("a pixel centre is on the canvas");
            let drawn = rg(&image, x, y);
            assert!(
                (p.uv[0] - drawn[0]).abs() <= UV_TOL && (p.uv[1] - drawn[1]).abs() <= UV_TOL,
                "pixel ({x}, {y}): picking reports UV {:?}; the render drew {drawn:?} there",
                p.uv
            );
            let f = set.quad_frame(p.quad);
            for k in 0..2 {
                assert!(
                    (p.uv[k] - f.c[k]).abs() <= f.h[k],
                    "pixel ({x}, {y}): the picked quad {} (c {:?}, h {:?}) does not hold UV {:?}",
                    p.quad,
                    f.c,
                    f.h,
                    p.uv
                );
            }
        }
    }
}

#[test]
fn qa_picking_cross_check_against_the_render() {
    check_cross(pick);
}

negative_control!(
    qa_picking_cross_check_against_the_render,
    "a pick that skips the flip disagrees with the render",
    expected = "the render drew",
    check_cross(|s, at| {
        pick(s, at).map(|p| {
            let v = at[1] / s.canvas[1];
            Pick {
                uv: [p.uv[0], v],
                ..p
            }
        })
    })
);
