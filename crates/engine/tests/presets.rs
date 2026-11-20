//! The coordinate presets as stain graphs (TASK-M1-07; colour_composition §6; debug_tooling_plan §F): each lowers and
//! assembles, and drawn through the preset harness over the synthetic set it draws its coordinate as RG with B = 0:
//! `uv_screen` the pixel's post-flip UV, `uv_quad` the sample's t, the coordinate view c + h·(2t − 1) and its δ mode
//! (δ/h + 1)/2 (REQ-TOOL-019, REQ-TOOL-027, REQ-TOOL-153). The render crate's `coords` tests check the reconstruction,
//! the banding criterion and the export on these presets in full; these hold each preset to its picture.
//!
//! Each test registers its negative control (R-176).

use engine::presets::{coordinate_view, uv_quad, uv_screen, CoordinateMode};
use engine::stain::{GraphError, StainGraph};
use engine::synthetic::Synthetic;
use render::assemble::{assemble, Tier};
use render::bind::{self, Context};
use render::coords::frag_uv;
use render::headless::{self, Draw, Image, Target};
use render::raster::Grid;
use validation::gpu::GpuHarness;
use validation::negative_control;

/// 2 × 2 quads of 4 × 4 one-pixel tiles at depth 3, from the slice's cell (5, 2): an 8 × 8 target.
fn set() -> Synthetic {
    let grid = Grid::new([2, 2], 4, 0, 1).expect("the preset grid");
    Synthetic::flat_at(grid, 3, [5, 2])
}

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

/// `graph` drawn through `bind::preset_module` over `set` into an `Rgba32Float` target.
fn draw(h: &GpuHarness, set: &Synthetic, graph: Result<StainGraph, GraphError>) -> Image {
    let graph = graph.unwrap_or_else(|e| panic!("{e}"));
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

fn rgb(image: &Image, x: u32, y: u32) -> [f64; 3] {
    let t = image.texel(x, y);
    [0, 1, 2].map(|k| {
        f64::from(f32::from_le_bytes([
            t[4 * k],
            t[4 * k + 1],
            t[4 * k + 2],
            t[4 * k + 3],
        ]))
    })
}

/// What each preset draws at pixel `(x, y)` of `set`, by name: the RG it must hold.
fn expected(set: &Synthetic, name: &str, x: u32, y: u32) -> [f64; 2] {
    let g = set.grid();
    let (w, h) = g.target();
    let cell = g.cell(x, y);
    let t = cell.tile_xy.map(|i| (f64::from(i) + 0.5) / f64::from(g.n));
    let f = set.quad_frame(cell.quad);
    match name {
        "uv_screen" => frag_uv(
            [f64::from(x) + 0.5, f64::from(y) + 0.5],
            [f64::from(w), f64::from(h)],
        ),
        "uv_quad" | "delta" => t,
        _ => [0, 1].map(|k| f.c[k] + f.h[k] * (2.0 * t[k] - 1.0)),
    }
}

/// Every pixel of each preset's picture is its RG to f32's rounding, B = 0.
fn check(set: &Synthetic, pictures: &[(&str, Image)]) {
    let (w, h) = set.grid().target();
    for (name, image) in pictures {
        for y in 0..h {
            for x in 0..w {
                let got = rgb(image, x, y);
                let want = expected(set, name, x, y);
                let close = (0..2).all(|k| (got[k] - want[k]).abs() <= f64::from(f32::EPSILON));
                assert!(
                    close && got[2] == 0.0,
                    "{name} at ({x}, {y}) draws {got:?}, not {want:?} with B = 0"
                );
            }
        }
    }
}

fn pictures(h: &GpuHarness, set: &Synthetic) -> Vec<(&'static str, Image)> {
    vec![
        ("uv_screen", draw(h, set, uv_screen())),
        ("uv_quad", draw(h, set, uv_quad())),
        (
            "coordinate",
            draw(h, set, coordinate_view(CoordinateMode::Uv)),
        ),
        (
            "delta",
            draw(h, set, coordinate_view(CoordinateMode::Delta)),
        ),
    ]
}

#[test]
fn presets_draw_their_coordinates() {
    let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let set = set();
    check(&set, &pictures(&h, &set));
}

negative_control!(
    presets_draw_their_coordinates,
    "the coordinate view's picture checked as uv_screen's fails",
    expected = "uv_screen at",
    {
        let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
        let set = set();
        let mut p = pictures(&h, &set);
        p.swap(0, 2);
        p[0].0 = "uv_screen";
        check(&set, &p[..1]);
    }
);
