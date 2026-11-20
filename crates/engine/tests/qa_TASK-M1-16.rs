//! QA's tests for TASK-M1-16, written from REQ-COL-063 (colour_composition §3; deep_zoom §1; R-97; R-394), not from
//! the implementation: `ctx.chart.slice_uv` is the sample's position in the slice plane's own frame,
//! `c + h·(2·ctx.quad.uv − 1)` with `c` and `h` its quad's centre and half-width as the CPU gives them, a vec2 in
//! [0, 1]², Y-up, unchanged by in-plane pan and zoom, and never a screen-relative position.
//!
//! `ctx.quad.uv` is taken at the pixel, as colour_composition §3 defines it, from the pixel's place in its quad, with
//! tiles of 1, 2 and 4 pixels so every value stays dyadic.
//!
//! - `qa_slice_uv_follows_each_quads_hand_filled_frame`: the per-quad frames are hand-filled (debug tooling plan,
//!   "Principle": synthetic buffers with known values), out of any grid order, with unequal `h_u` and `h_v` and one
//!   frame spanning all of [0, 1]²; every pixel reads `c + h·(2t − 1)` of its own quad's frame, bit for bit (every value
//!   dyadic), inside [0, 1]², and `v` rises up the framebuffer (Y-up) while `u` rises to the right.
//! - `qa_slice_uv_ignores_where_the_quad_is_drawn`: one frame given to every quad of two grids of different shape and
//!   pixel scale: every pixel of every quad reads the same `slice_uv` at its `quad.uv`, though its screen position differs.
//! - `qa_slice_uv_from_the_cpu_frames_at_the_slice_edges`: through the harness's own f64 frames, the depth-0 quad reads
//!   `slice_uv = quad.uv`, and a depth-3 grid at the slice's top-right corner reads `(cell + t)/8`, reaching up to 1.
//!
//! Negative controls (R-176): the screen position `ctx.screen.uv`, the grid tiling `(quad_xy + quad.uv) / quads`, a
//! Y-down reading of the frame, and another quad's frame each fail.

use engine::synthetic::Synthetic;
use render::bind::{self, Context, ViewOutput};
use render::headless::{self, Draw, Image, Target};
use render::raster::Grid;
use validation::gpu::GpuHarness;
use validation::negative_control;

fn gpu() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

fn context(grid: Grid) -> Context {
    Context {
        dt_macro: 0.015625,
        delta_0: 1e-6,
        n_renorm: 8,
        horizon_steps: 4096,
        z: [0.0; 8],
        grid,
        chart_id: 0,
        time: 0.0,
        ensemble_spread: 0.0,
        out_of_chart: false,
        valid_sample_count: grid.n * grid.n,
    }
}

/// The lane under test: the harness's `ctx.chart.slice_uv`.
const SLICE_UV: &str = "l.chart.slice_uv";

/// `expr` (a `vec2<f32>` over `l`) and `ctx.screen.uv`, as bits, at every pixel of `set`, its frames replaced by
/// `frames` (`(c_u, c_v, h_u, h_v)` per quad) where given.
fn draw(h: &GpuHarness, set: &Synthetic, frames: Option<&[[f32; 4]]>, expr: &str) -> Image {
    let grid = set.grid();
    let ctx = context(grid);
    let mut bytes = set.bytes();
    if let Some(frames) = frames {
        assert_eq!(
            frames.len(),
            grid.quad_count() as usize,
            "one frame per quad"
        );
        bytes.quad_frame = frames
            .iter()
            .flat_map(|f| f.iter().flat_map(|x| x.to_le_bytes()))
            .collect();
    }
    let bound = bind::upload(h.device(), &bytes.payload(), &ctx);
    let view = format!(
        "fn view(rc: RenderContext, l: Lanes) -> vec4<u32> {{\n    \
         return vec4<u32>(bitcast<vec2<u32>>({expr}), bitcast<vec2<u32>>(l.screen.uv));\n}}\n"
    );
    let module = bind::module(&view, ViewOutput::Words).unwrap_or_else(|e| panic!("{e}"));
    let (width, height) = grid.target();
    headless::render(
        h.device(),
        h.queue(),
        &Draw {
            module: &module,
            entry: bind::ENTRY,
            layouts: &bound.layout_refs(),
            groups: &bound.group_refs(),
        },
        Target {
            width,
            height,
            format: wgpu::TextureFormat::Rgba32Uint,
        },
    )
    .unwrap_or_else(|e| panic!("{e}"))
}

fn lane(image: &Image, x: u32, y: u32) -> [f32; 2] {
    let w = image.words(x, y);
    [f32::from_bits(w[0]), f32::from_bits(w[1])]
}

fn screen(image: &Image, x: u32, y: u32) -> [f32; 2] {
    let w = image.words(x, y);
    [f32::from_bits(w[2]), f32::from_bits(w[3])]
}

/// deep_zoom §1's `u = c + h·(2t − 1)` on each axis, in f64 from the frame, to f32 (exact for dyadic inputs).
fn position(frame: [f64; 4], t: [f64; 2]) -> [f32; 2] {
    [0, 1].map(|k| (frame[k] + frame[k + 2] * (2.0 * t[k] - 1.0)) as f32)
}

/// `ctx.quad.uv` at pixel `(x, y)` from the pixel geometry alone (colour_composition §3, R-72: `quad.uv = (vec2(i, j) +
/// tile.uv) / N`, `tile.uv` the pixel centre's position in its tile, Y-up from the bottom-left): the pixel centre's
/// position in its quad's `N · tile_px` pixel square, the framebuffer's rows counted from the top.
fn t_of(grid: Grid, x: u32, y: u32) -> [f64; 2] {
    let (_, height) = grid.target();
    let side = f64::from(grid.n * grid.tile_px);
    let up = height - 1 - y;
    [x, up].map(|p| (f64::from(p % (grid.n * grid.tile_px)) + 0.5) / side)
}

// ── Hand-filled frames ──────────────────────────────────────────────────────────────────────────────────────────

/// Six frames for a 3 × 2 grid, in no grid order: unequal half-widths on most, one spanning [0, 1]², one at each of
/// the slice's corners; all dyadic.
const FRAMES: [[f32; 4]; 6] = [
    [0.125, 0.875, 0.125, 0.125],
    [0.75, 0.25, 0.25, 0.0625],
    [0.5, 0.5, 0.5, 0.5],
    [0.9375, 0.0625, 0.0625, 0.0625],
    [0.3125, 0.625, 0.03125, 0.25],
    [0.0078125, 0.5, 0.0078125, 0.001953125],
];

fn hand_filled_set() -> Synthetic {
    // Two pixels per tile, so `quad.uv` moves inside a tile.
    let grid = Grid::new([3, 2], 4, 0, 2).expect("the hand-filled grid");
    Synthetic::flat_at(grid, 5, [11, 3])
}

fn check_hand_filled(image: &Image, set: &Synthetic) {
    let grid = set.grid();
    let (width, height) = grid.target();
    for y in 0..height {
        for x in 0..width {
            let c = grid.cell(x, y);
            let f = FRAMES[c.quad as usize].map(f64::from);
            let want = position(f, t_of(grid, x, y));
            let got = lane(image, x, y);
            assert_eq!(
                got.map(f32::to_bits),
                want.map(f32::to_bits),
                "pixel ({x}, {y}), quad {} at {:?}, tile {:?}: slice_uv {got:?} is not c + h·(2t − 1) = {want:?} of \
                 its own frame",
                c.quad,
                c.quad_xy,
                c.tile_xy
            );
            assert!(
                got.iter().all(|v| (0.0..=1.0).contains(v)),
                "pixel ({x}, {y}): slice_uv {got:?} is outside [0, 1]²"
            );
            // Y-up, by the framebuffer alone: within one quad, the pixel above (smaller y) has the larger v, the pixel
            // to the right the larger u.
            if y > 0 {
                let above = grid.cell(x, y - 1);
                if above.quad == c.quad {
                    let v_above = lane(image, x, y - 1)[1];
                    assert!(
                        v_above > got[1],
                        "pixel ({x}, {y}) to the one above it in quad {}: v {} then {v_above}; slice_uv is not Y-up",
                        c.quad,
                        got[1]
                    );
                }
            }
            if x + 1 < width {
                let right = grid.cell(x + 1, y);
                if right.quad == c.quad {
                    let u_right = lane(image, x + 1, y)[0];
                    assert!(
                        u_right > got[0],
                        "pixel ({x}, {y}) to the one right of it in quad {}: u {} then {u_right}",
                        c.quad,
                        got[0]
                    );
                }
            }
        }
    }
}

fn hand_filled(expr: &str) {
    let set = hand_filled_set();
    check_hand_filled(&draw(&gpu(), &set, Some(&FRAMES), expr), &set);
}

#[test]
fn qa_slice_uv_follows_each_quads_hand_filled_frame() {
    hand_filled(SLICE_UV);
}

negative_control!(
    qa_slice_uv_follows_each_quads_hand_filled_frame,
    "the screen position ctx.screen.uv in slice_uv's place is not the frame's position",
    expected = "is not c + h·(2t − 1)",
    hand_filled("l.screen.uv")
);

/// The grid tiling, `(quad_xy + quad.uv) / quads`, the view reading R-394 did not choose.
#[cfg(feature = "controls")]
const TILING: &str = "(vec2<f32>(vec2<u32>(l.quad.index % ctx_uniforms.quads.x, l.quad.index / \
                      ctx_uniforms.quads.x)) + l.quad.uv) / vec2<f32>(ctx_uniforms.quads)";

mod tiling {
    use super::*;
    negative_control!(
        qa_slice_uv_follows_each_quads_hand_filled_frame,
        "the grid tiling (quad_xy + quad.uv) / quads reads the grid, not the quad's frame",
        expected = "is not c + h·(2t − 1)",
        hand_filled(TILING)
    );
}

mod y_down {
    use super::*;
    negative_control!(
        qa_slice_uv_follows_each_quads_hand_filled_frame,
        "the frame read Y-down, t_v → 1 − t_v, puts v the wrong way up",
        expected = "is not c + h·(2t − 1)",
        hand_filled(
            "l.quad.centre + l.quad.half_width * (2.0 * vec2<f32>(l.quad.uv.x, 1.0 - l.quad.uv.y) - vec2<f32>(1.0))"
        )
    );
}

mod other_frame {
    use super::*;
    negative_control!(
        qa_slice_uv_follows_each_quads_hand_filled_frame,
        "the next quad's frame in the quad's place reads another quad's position",
        expected = "is not c + h·(2t − 1)",
        hand_filled(
            "quad_frames[(l.quad.index + 1u) % 6u].xy + quad_frames[(l.quad.index + 1u) % 6u].zw * \
             (2.0 * l.quad.uv - vec2<f32>(1.0))"
        )
    );
}

// ── Pan and zoom of the view ────────────────────────────────────────────────────────────────────────────────────

/// One frame, given to every quad.
const ONE: [f32; 4] = [0.40625, 0.71875, 0.03125, 0.0078125];

/// Two grids of different shape and pixel scale, every quad holding [`ONE`]: every pixel reads `ONE`'s position at its
/// `quad.uv`, bit for bit, wherever its quad is drawn, while the screen positions of its copies differ.
fn placement(expr: &str) {
    let h = gpu();
    for (quads, tile_px) in [([3, 2], 1), ([2, 3], 2)] {
        let grid = Grid::new(quads, 4, 0, tile_px).expect("the placement grid");
        let set = Synthetic::flat_at(grid, 4, [0, 0]);
        let frames = vec![ONE; grid.quad_count() as usize];
        let image = draw(&h, &set, Some(&frames), expr);
        let (width, height) = grid.target();
        let mut screens = std::collections::HashSet::new();
        for y in 0..height {
            for x in 0..width {
                let c = grid.cell(x, y);
                let want = position(ONE.map(f64::from), t_of(grid, x, y));
                let got = lane(&image, x, y);
                assert_eq!(
                    got.map(f32::to_bits),
                    want.map(f32::to_bits),
                    "{quads:?} quads of {tile_px} px tiles, pixel ({x}, {y}), quad {:?}, tile {:?}: slice_uv {got:?} \
                     moved with the view; the frame's position is {want:?}",
                    c.quad_xy,
                    c.tile_xy
                );
                if (x % (grid.n * tile_px), y % (grid.n * tile_px)) == (0, 0) {
                    screens.insert(screen(&image, x, y).map(f32::to_bits));
                }
            }
        }
        assert!(
            screens.len() > 1,
            "the copies of one quad-local pixel read one screen position: the test could not see a screen-relative lane"
        );
    }
}

#[test]
fn qa_slice_uv_ignores_where_the_quad_is_drawn() {
    placement(SLICE_UV);
}

negative_control!(
    qa_slice_uv_ignores_where_the_quad_is_drawn,
    "the screen position moves with the view",
    expected = "moved with the view",
    placement("l.screen.uv")
);

// ── Through the CPU's f64 frames ────────────────────────────────────────────────────────────────────────────────

/// The depth-0 quad (the whole slice, `c = (½, ½)`, `h = ½`) reads `slice_uv = quad.uv`; a 2 × 1 grid at depth 3 from
/// the slice's cell (6, 7) reads `(cell + t)/8`, its top-right sample nearest (1, 1).
fn slice_edges(expr: &str) {
    let h = gpu();
    for (quads, depth, origin) in [([1, 1], 0, [0, 0]), ([2, 1], 3, [6, 7])] {
        let grid = Grid::new(quads, 8, 0, 1).expect("the edge grid");
        let set = Synthetic::flat_at(grid, depth, origin);
        let image = draw(&h, &set, None, expr);
        let (width, height) = grid.target();
        let side = f64::from(1u32 << depth);
        for y in 0..height {
            for x in 0..width {
                let c = grid.cell(x, y);
                let t = t_of(grid, x, y);
                let want = [0, 1]
                    .map(|k| ((origin[k] + u64::from(c.quad_xy[k])) as f64 + t[k]) / side)
                    .map(|v| v as f32);
                let got = lane(&image, x, y);
                assert_eq!(
                    got.map(f32::to_bits),
                    want.map(f32::to_bits),
                    "depth {depth} from {origin:?}, pixel ({x}, {y}), quad {:?}: slice_uv {got:?}, not the slice-plane \
                     position {want:?}",
                    c.quad_xy
                );
            }
        }
    }
}

#[test]
fn qa_slice_uv_from_the_cpu_frames_at_the_slice_edges() {
    slice_edges(SLICE_UV);
}

mod edges_tiling {
    use super::*;
    negative_control!(
        qa_slice_uv_from_the_cpu_frames_at_the_slice_edges,
        "the grid tiling reads the 2 × 1 grid's own [0, 1]², not the slice's corner",
        expected = "not the slice-plane",
        slice_edges(TILING)
    );
}
