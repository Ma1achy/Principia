//! The addressing presets drawn on the GPU (colour_composition §9.7): TL = RB, centre = BG and the quad-index grey, each
//! checked at every pixel of a small synthetic set. The criteria are written above the checks.
//!
//! Each test registers its negative control.

use engine::contract::stain::preset;
use engine::stain::{GraphError, StainGraph};
use engine::synthetic::Synthetic;
use render::assemble::{assemble, Tier};
use render::bind::{self, Context};
use render::headless::{self, Draw, Image, Target};
use render::presets::file::Section;
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

/// The addressing presets' pictures on the GPU (colour_composition §9.7).
///
/// Pass criteria, written before the runs: at every pixel, TL = RB draws R = tl.x, G = 0, B = tl.y with
/// `tl = c + (−h, +h)`; centre = BG draws R = 0, G = c.y, B = c.x; the index grey draws `(quad & 255) / 255` on all three
/// channels. Each to f32's rounding of the f64 value.
fn pictures(h: &GpuHarness, set: &Synthetic) -> Vec<(&'static str, Image)> {
    ["tl_rb", "centre_bg", "index_grey"]
        .map(|id| (id, draw(h, set, preset::graph(Section::Debug, id))))
        .into()
}

fn want(set: &Synthetic, name: &str, x: u32, y: u32) -> [f64; 3] {
    let cell = set.grid().cell(x, y);
    let f = set.quad_frame(cell.quad);
    match name {
        "tl_rb" => [f.c[0] - f.h[0], 0.0, f.c[1] + f.h[1]],
        "centre_bg" => [0.0, f.c[1], f.c[0]],
        _ => {
            let g = f64::from(index_grey(cell.quad));
            [g, g, g]
        }
    }
}

/// The index grey's rule (colour_composition §9.7): the index's low byte over 255.
fn index_grey(index: u32) -> f32 {
    (index & 255) as f32 / 255.0
}

fn check(set: &Synthetic, pictures: &[(&str, Image)]) {
    let (w, h) = set.grid().target();
    for (name, image) in pictures {
        for y in 0..h {
            for x in 0..w {
                let got = rgb(image, x, y);
                let w3 = want(set, name, x, y);
                let close = (0..3).all(|k| (got[k] - w3[k]).abs() <= 1e-6 * (1.0 + w3[k].abs()));
                assert!(close, "{name} at ({x}, {y}) draws {got:?}, not {w3:?}");
            }
        }
    }
}

#[test]
fn addressing_grey_draws_its_channels() {
    let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let set = set();
    check(&set, &pictures(&h, &set));
}

negative_control!(
    addressing_grey_draws_its_channels,
    "the centre preset's picture checked as TL = RB fails",
    expected = "tl_rb at",
    {
        let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
        let set = set();
        let mut p = pictures(&h, &set);
        p[0].1 = std::mem::replace(
            &mut p[1].1,
            draw(&h, &set, preset::graph(Section::Debug, "tl_rb")),
        );
        check(&set, &p[..1]);
    }
);
