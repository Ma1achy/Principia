//! QA tests for TASK-M1-07 at the merged `Ctx` (`{sample, ic, frag_xy, screen, quad, inputs}`), written from the
//! requirements, not from the implementation:
//! - REQ-SYS-009 / REQ-TOOL-019 / REQ-TOOL-153: a stain that reads its screen and quad lanes (`ctx.screen.uv`,
//!   `ctx.quad.uv`, `ctx.quad.centre`, `ctx.quad.half_width`) gets the post-flip UV, t = (i + ½)/N and deep_zoom §1's
//!   c and h, while the same node also reads `ctx.ic` (RQ-227), which gets each sample's own stored descriptor: one
//!   assembled module, every lane correct at once.
//!
//! The test registers its negative control (R-176).

use render::assemble::{assemble, Tier};
use render::bind::{self, Context};
use render::headless::{self, Draw, Image, Target};
use render::raster::Grid;
use validation::gpu::GpuHarness;
use validation::negative_control;
use validation::stain::{NodeId, NodeKind, Occupant, StainGraph};
use validation::synthetic::Synthetic;

const F32: wgpu::TextureFormat = wgpu::TextureFormat::Rgba32Float;

/// Four f32 ulps at 1, as the TASK-M1-07 qa tests allow a UV.
const UV_TOL: f64 = 4.0 * f32::EPSILON as f64;

const DEPTH: u32 = 9;
const ORIGIN: [u64; 2] = [301, 77];

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

fn graph(colour: &str) -> StainGraph {
    let mut g = StainGraph::new();
    let s = g
        .add(
            NodeKind::Source,
            Occupant::Custom(
                "fn source(ctx: Ctx) -> Field { return Field(0.0, 0.0, 0.0, 0.0); }\n".into(),
            ),
        )
        .expect("source");
    let c = g
        .add(NodeKind::Colour, Occupant::Custom(colour.into()))
        .expect("colour");
    g.connect(s, c, 0).expect("source → colour");
    g.connect(c, NodeId(0), 0).expect("colour → combiner");
    g
}

fn draw(h: &GpuHarness, set: &Synthetic, colour: &str) -> Image {
    let stain = graph(colour).lower().unwrap_or_else(|e| panic!("{e}"));
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
            format: F32,
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

fn rho_angle(s: u32) -> f32 {
    0.125 * (s + 1) as f32
}
fn k0(s: u32) -> f32 {
    -((s + 1) as f32)
}
fn v0(s: u32) -> f32 {
    0.5 * (s + 1) as f32 + 0.03125
}

/// deep_zoom §1: the depth-ℓ cell `(column, row)` has centre `(cell + ½)·2^−ℓ` and half-width `2^−(ℓ+1)`.
fn frame(cell: [u64; 2]) -> ([f64; 2], f64) {
    let h = 0.5f64.powi(DEPTH as i32 + 1);
    (cell.map(|k| (2.0 * k as f64 + 1.0) * h), h)
}

fn ulp(x: f64) -> f64 {
    let x = (x as f32).abs();
    f64::from(f32::from_bits(x.to_bits() + 1) - x)
}

fn assert_near(got: f64, want: f64, tol: f64, what: &str) {
    assert!((got - want).abs() <= tol, "{what}: drawn {got}, not {want}");
}

/// On 2 × 2 quads of 4 × 4 one-pixel samples, each sample's descriptor stored at index `stored_at(s)`: three stains,
/// each reading a member of `ctx.ic` beside the screen or quad lane, draw the sample's own descriptor member and the
/// lanes' values.
fn check_ic_with_lanes(stored_at: fn(u32) -> u32) {
    let grid = Grid::new([2, 2], 4, 0, 1).expect("grid");
    let mut set = Synthetic::flat_at(grid, DEPTH, ORIGIN);
    for s in 0..grid.sample_count() {
        let d = set.ic(stored_at(s));
        d.rho_angle = rho_angle(s);
        d.K_0 = k0(s);
        d.V_0 = v0(s);
    }
    let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let quad_uv = draw(
        &h,
        &set,
        "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.ic.rho_angle, ctx.quad.uv); }\n",
    );
    let quad_frame = draw(
        &h,
        &set,
        "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.ic.K_0, ctx.quad.centre.x, ctx.quad.half_width.y); }\n",
    );
    let screen = draw(
        &h,
        &set,
        "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.ic.V_0, ctx.screen.uv); }\n",
    );
    let (w, ht) = grid.target();
    for y in 0..ht {
        for x in 0..w {
            let cell = grid.cell(x, y);
            let s = cell.sample;
            let at = format!("pixel ({x}, {y}), sample {s}");

            let a = rgb(&quad_uv, x, y);
            assert_eq!(
                a[0] as f32,
                rho_angle(s),
                "{at}: ctx.ic.rho_angle is not the sample's own stored descriptor (RQ-227)"
            );
            for k in 0..2 {
                let t = (f64::from(cell.tile_xy[k]) + 0.5) / f64::from(grid.n);
                assert_near(
                    a[1 + k],
                    t,
                    UV_TOL,
                    &format!("{at}: ctx.quad.uv[{k}] beside ctx.ic"),
                );
            }

            let b = rgb(&quad_frame, x, y);
            assert_eq!(
                b[0] as f32,
                k0(s),
                "{at}: ctx.ic.K_0 is not the sample's own stored descriptor (RQ-227)"
            );
            let (c, hw) = frame([
                ORIGIN[0] + u64::from(cell.quad_xy[0]),
                ORIGIN[1] + u64::from(cell.quad_xy[1]),
            ]);
            assert_near(
                b[1],
                c[0],
                ulp(c[0]),
                &format!("{at}: ctx.quad.centre.x beside ctx.ic"),
            );
            assert_near(
                b[2],
                hw,
                ulp(hw),
                &format!("{at}: ctx.quad.half_width.y beside ctx.ic"),
            );

            let c3 = rgb(&screen, x, y);
            assert_eq!(
                c3[0] as f32,
                v0(s),
                "{at}: ctx.ic.V_0 is not the sample's own stored descriptor (RQ-227)"
            );
            let uv = [
                (f64::from(x) + 0.5) / f64::from(w),
                1.0 - (f64::from(y) + 0.5) / f64::from(ht),
            ];
            for k in 0..2 {
                assert_near(
                    c3[1 + k],
                    uv[k],
                    UV_TOL,
                    &format!("{at}: ctx.screen.uv[{k}] beside ctx.ic"),
                );
            }
        }
    }
}

#[test]
fn qa_ctx_ic_and_screen_quad_lanes_in_one_module() {
    check_ic_with_lanes(|s| s);
}

negative_control!(
    qa_ctx_ic_and_screen_quad_lanes_in_one_module,
    "descriptors stored in another sample order are not each sample's own",
    expected = "is not the sample's own stored descriptor",
    check_ic_with_lanes(|s| s ^ 1)
);
