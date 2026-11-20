//! `ctx.chart.slice_uv` is the sample's position in the slice plane (R-394; REQ-COL-063; colour_composition §3):
//! `c + h·(2·ctx.quad.uv − 1)` from its quad's centre and half-width, deep_zoom §1's `u = c + h·(2t − 1)`, stable under
//! in-plane pan and zoom (R-97), never a screen or view position. Drawn through the module harness over flat sets at
//! depth 5 from the slice's cell (7, 2), on a 3 × 2 grid, whose columns and rows differ and whose quads are not the
//! slice's tiling from 0: the one place the grid-tiling reading R-394 did not choose differs from the slice plane's.
//! - `chart_slice_uv_reads_the_quad_frame_on_an_offset_grid`: the quad in column `i`, row `j` reads
//!   `((7 + i + ½)/32, (2 + j + ½)/32)` at its centre, `c ± h` at its corners, Y-up, and `c + h·(2·quad.uv − 1)` at
//!   each of its pixels;
//! - `chart_slice_uv_is_stable_under_pan_and_zoom`: the same quad inside a panned grid and a zoomed-out grid reads the
//!   same `slice_uv`, bit for bit;
//! - `chart_slice_uv_child_and_parent_share_their_point`: the depth-6 child at cell (15, 5) at `quad.uv` (0, 0) and
//!   its depth-5 parent at cell (7, 2) at `quad.uv` (½, ½) read the same `slice_uv`, (15/64, 5/64), bit for bit.
//!
//! The lane is read at the pixel (`l.chart.slice_uv`) and, for the centre and corners, which no pixel centre reaches,
//! through the harness's own `lanes(rc, r)` on a raster whose quad is the pixel's and whose `quad_uv` is the probe:
//! the same fill expression, at another `quad.uv`. Every value is dyadic, so exact in f32.
//!
//! The negative controls (R-176) put the two lanes REQ-COL-063 names in its place: the grid tiling TASK-M1-06 filled,
//! `(quad_xy + quad.uv) / quads`, and the slice-plane formula with the grid's columns and rows swapped in the index of
//! its quad's frame.

use std::collections::HashMap;

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

/// Samples per quad axis, each a one-pixel tile: the pixels read `quad.uv` = (k + ½)/4, dyadic.
const N: u32 = 4;

/// The lane's fill drawn: the harness's own, or, under `controls`, one of the negative controls' two wrong ones.
#[derive(Clone, Copy, Debug)]
enum Lane {
    /// `l.chart.slice_uv`, and the harness's `lanes(rc, r)` at a probe `quad.uv`.
    Harness,
    /// TASK-M1-06's grid tiling, `(vec2<f32>(quad_xy) + quad_uv) / vec2<f32>(quads)`: the view reading.
    #[cfg(feature = "controls")]
    Tiling,
    /// `c + h·(2·quad_uv − 1)` from the frame of the quad whose index takes the grid's columns and rows swapped,
    /// `quad_frames[i · rows + j]`.
    #[cfg(feature = "controls")]
    Swapped,
}

impl Lane {
    /// WGSL: `lane_at(rc, l, uv)`, the lane at `quad.uv = uv` in the pixel's quad, and `lane_here(rc, l)`, the lane at
    /// the pixel.
    fn wgsl(self) -> &'static str {
        match self {
            Lane::Harness => {
                "fn lane_at(rc: RenderContext, l: Lanes, uv: vec2<f32>) -> vec2<f32> {
    var r: Raster;
    r.quad = l.quad.index;
    r.quad_uv = uv;
    return lanes(rc, r).chart.slice_uv;
}
fn lane_here(rc: RenderContext, l: Lanes) -> vec2<f32> { return l.chart.slice_uv; }
"
            }
            #[cfg(feature = "controls")]
            Lane::Tiling => {
                "fn lane_at(rc: RenderContext, l: Lanes, uv: vec2<f32>) -> vec2<f32> {
    let q = ctx_uniforms.quads;
    let quad_xy = vec2<u32>(l.quad.index % q.x, l.quad.index / q.x);
    return (vec2<f32>(quad_xy) + uv) / vec2<f32>(q);
}
fn lane_here(rc: RenderContext, l: Lanes) -> vec2<f32> { return lane_at(rc, l, l.quad.uv); }
"
            }
            #[cfg(feature = "controls")]
            Lane::Swapped => {
                "fn lane_at(rc: RenderContext, l: Lanes, uv: vec2<f32>) -> vec2<f32> {
    let q = ctx_uniforms.quads;
    let quad_xy = vec2<u32>(l.quad.index % q.x, l.quad.index / q.x);
    let f = quad_frames[quad_xy.x * q.y + quad_xy.y];
    return f.xy + f.zw * (2.0 * uv - vec2<f32>(1.0));
}
fn lane_here(rc: RenderContext, l: Lanes) -> vec2<f32> { return lane_at(rc, l, l.quad.uv); }
"
            }
        }
    }
}

/// The readings, in order: at the pixel, then at the quad's centre and its four corners, bottom-left, bottom-right,
/// top-left, top-right (Y-up).
const PROBES: [Option<[f32; 2]>; 6] = [
    None,
    Some([0.5, 0.5]),
    Some([0.0, 0.0]),
    Some([1.0, 0.0]),
    Some([0.0, 1.0]),
    Some([1.0, 1.0]),
];

fn probe_wgsl(p: Option<[f32; 2]>) -> String {
    match p {
        None => "lane_here(rc, l)".into(),
        Some([u, v]) => format!("lane_at(rc, l, vec2<f32>({u:?}, {v:?}))"),
    }
}

/// A flat set at `depth` whose grid column `i`, row `j` is the slice's cell `origin + (i, j)`.
struct Set {
    synthetic: Synthetic,
    origin: [u64; 2],
}

fn set(quads: [u32; 2], depth: u32, origin: [u64; 2]) -> Set {
    let grid = Grid::new(quads, N, 0, 1).expect("the slice_uv grid");
    Set {
        synthetic: Synthetic::flat_at(grid, depth, origin),
        origin,
    }
}

/// The offset grid: 3 × 2 quads at depth 5 from the slice's cell (7, 2).
fn offset_grid() -> Set {
    set([3, 2], 5, [7, 2])
}

/// One pixel's readings: its quad's slice cell, its tile, its `quad.uv`, and the six readings of [`PROBES`].
struct Reading {
    pixel: (u32, u32),
    cell: [u64; 2],
    tile: [u32; 2],
    quad_uv: [f32; 2],
    lane: [[f32; 2]; 6],
}

/// `lane` drawn over `set` at every pixel, at the pixel and at each probe.
fn read(h: &GpuHarness, set: &Set, lane: Lane) -> Vec<Reading> {
    let grid = set.synthetic.grid();
    let ctx = context(grid);
    let bytes = set.synthetic.bytes();
    let bound = bind::upload(h.device(), &bytes.payload(), &ctx);
    let (width, height) = grid.target();
    let images: Vec<Image> = PROBES
        .chunks(2)
        .map(|pair| {
            let view = format!(
                "{}fn view(rc: RenderContext, l: Lanes) -> vec4<u32> {{\n    \
                 return vec4<u32>(bitcast<vec2<u32>>({}), bitcast<vec2<u32>>({}));\n}}\n",
                lane.wgsl(),
                probe_wgsl(pair[0]),
                probe_wgsl(pair[1]),
            );
            let module = bind::module(&view, ViewOutput::Words).unwrap_or_else(|e| panic!("{e}"));
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
        })
        .collect();
    let mut out = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let c = grid.cell(x, y);
            let mut lane = [[0.0; 2]; 6];
            for (k, image) in images.iter().enumerate() {
                let w = image.words(x, y);
                lane[2 * k] = [f32::from_bits(w[0]), f32::from_bits(w[1])];
                lane[2 * k + 1] = [f32::from_bits(w[2]), f32::from_bits(w[3])];
            }
            out.push(Reading {
                pixel: (x, y),
                cell: [0, 1].map(|k| set.origin[k] + u64::from(c.quad_xy[k])),
                tile: c.tile_xy,
                quad_uv: c.tile_xy.map(|t| (t as f32 + 0.5) / N as f32),
                lane,
            });
        }
    }
    out
}

fn bits(v: [f32; 2]) -> [u32; 2] {
    v.map(f32::to_bits)
}

/// The slice-plane position of `quad_uv` in the depth-`depth` cell `cell`: `(cell + quad_uv) / 2^depth`, which is
/// `c + h·(2·quad_uv − 1)` with `c = (cell + ½)/2^depth` and `h = 1/2^(depth + 1)`, exact (dyadic) in f64 and f32.
fn slice_position(cell: [u64; 2], depth: u32, quad_uv: [f32; 2]) -> [f32; 2] {
    let side = (1u64 << depth) as f64;
    [0, 1].map(|k| ((cell[k] as f64 + f64::from(quad_uv[k])) / side) as f32)
}

// ── On the offset grid ──────────────────────────────────────────────────────────────────────────────────────────

/// Every pixel of the 3 × 2 grid at depth 5 from (7, 2): its quad, column `i` and row `j` (Y-up), reads
/// `((7 + i + ½)/32, (2 + j + ½)/32)` at its centre, `c ± h` (h = 1/64) at its corners and
/// `c + h·(2·quad.uv − 1)` at the pixel, bit for bit.
fn check_offset_grid(readings: &[Reading]) {
    assert_eq!(
        readings.len(),
        3 * 2 * (N * N) as usize,
        "the 3 × 2 grid's pixels"
    );
    for r in readings {
        let [i, j] = [0, 1].map(|k| r.cell[k] - [7, 2][k]);
        let centre = [(7.0 + i as f32 + 0.5) / 32.0, (2.0 + j as f32 + 0.5) / 32.0];
        let h = 1.0 / 64.0;
        let mut want = [[0.0; 2]; 6];
        want[0] = slice_position(r.cell, 5, r.quad_uv);
        for (k, p) in PROBES.iter().enumerate().skip(1) {
            let p = p.expect("a probe");
            want[k] = [0, 1].map(|a| centre[a] + h * (2.0 * p[a] - 1.0));
            assert_eq!(
                bits(want[k]),
                bits(slice_position(r.cell, 5, p)),
                "the test's centre ± h is not the cell's corner"
            );
        }
        for k in 0..6 {
            let at = PROBES[k].unwrap_or(r.quad_uv);
            assert_eq!(
                bits(r.lane[k]),
                bits(want[k]),
                "pixel {:?}, quad (column {i}, row {j}), slice cell {:?}: ctx.chart.slice_uv at quad.uv {at:?} \
                 reads {:?}, not the slice-plane position {:?}",
                r.pixel,
                r.cell,
                r.lane[k],
                want[k]
            );
        }
    }
}

#[test]
fn chart_slice_uv_reads_the_quad_frame_on_an_offset_grid() {
    check_offset_grid(&read(&gpu(), &offset_grid(), Lane::Harness));
}

negative_control!(
    chart_slice_uv_reads_the_quad_frame_on_an_offset_grid,
    "TASK-M1-06's grid tiling, (quad_xy + quad.uv) / quads, reads the view's position, not the slice plane's, on the \
     offset grid",
    expected = "not the slice-plane position",
    check_offset_grid(&read(&gpu(), &offset_grid(), Lane::Tiling))
);

// ── Under pan and zoom ──────────────────────────────────────────────────────────────────────────────────────────

/// The offset grid's quads inside a panned grid, 3 × 2 from (6, 1), sharing cells (7, 2) and (8, 2), and a zoomed-out
/// grid, 5 × 4 from (5, 0), holding all six.
fn moved_grids() -> [(&'static str, Set, usize); 2] {
    [
        ("panned", set([3, 2], 5, [6, 1]), 2),
        ("zoomed", set([5, 4], 5, [5, 0]), 6),
    ]
}

/// Each slice cell and tile the offset grid and a moved grid both draw reads the same six readings, bit for bit; the
/// moved grid shares `cells` of the offset grid's cells.
fn check_pan_and_zoom(base: &[Reading], moved: &[(&str, Vec<Reading>, usize)]) {
    let at: HashMap<([u64; 2], [u32; 2]), &Reading> =
        base.iter().map(|r| ((r.cell, r.tile), r)).collect();
    for (name, readings, cells) in moved {
        let mut shared = 0;
        for r in readings {
            let Some(b) = at.get(&(r.cell, r.tile)) else {
                continue;
            };
            shared += 1;
            for k in 0..6 {
                assert_eq!(
                    bits(r.lane[k]),
                    bits(b.lane[k]),
                    "slice cell {:?}, tile {:?}, reading {k}: the {name} grid's ctx.chart.slice_uv {:?} is not the \
                     offset grid's {:?}",
                    r.cell,
                    r.tile,
                    r.lane[k],
                    b.lane[k]
                );
            }
        }
        assert_eq!(
            shared,
            cells * (N * N) as usize,
            "the {name} grid shares {shared} pixels with the offset grid, not {cells} quads'"
        );
    }
}

fn pan_and_zoom(lane: Lane) {
    let h = gpu();
    let base = read(&h, &offset_grid(), lane);
    let moved: Vec<(&str, Vec<Reading>, usize)> = moved_grids()
        .into_iter()
        .map(|(name, s, cells)| (name, read(&h, &s, lane), cells))
        .collect();
    check_pan_and_zoom(&base, &moved);
}

#[test]
fn chart_slice_uv_is_stable_under_pan_and_zoom() {
    pan_and_zoom(Lane::Harness);
}

negative_control!(
    chart_slice_uv_is_stable_under_pan_and_zoom,
    "a lane indexing its quad's frame with the grid's columns and rows swapped reads another cell's frame in the \
     panned grid",
    expected = "is not the offset grid's",
    pan_and_zoom(Lane::Swapped)
);

// ── Across depths ───────────────────────────────────────────────────────────────────────────────────────────────

/// The depth-6 child at cell (15, 5) read at `quad.uv` (0, 0), its bottom-left corner, and its depth-5 parent at
/// (7, 2) read at (½, ½), its centre, are the one point (15/64, 5/64): both read it, bit for bit.
fn check_child_and_parent(child: &[Reading], parent: &[Reading]) {
    let want = [15.0 / 64.0, 5.0 / 64.0];
    for (name, readings, k, cell) in [
        ("depth-6 child", child, 2, [15, 5]),
        ("depth-5 parent", parent, 1, [7, 2]),
    ] {
        assert!(!readings.is_empty(), "the {name} draws no pixel");
        for r in readings {
            assert_eq!(r.cell, cell, "the {name}'s cell");
            assert_eq!(
                bits(r.lane[k]),
                bits(want),
                "pixel {:?}: the {name} at cell {cell:?} reads ctx.chart.slice_uv {:?} at quad.uv {:?}, not the \
                 shared point {want:?}",
                r.pixel,
                r.lane[k],
                PROBES[k]
            );
        }
    }
}

fn child_and_parent(lane: Lane) {
    let h = gpu();
    let child = read(&h, &set([1, 1], 6, [15, 5]), lane);
    let parent = read(&h, &set([1, 1], 5, [7, 2]), lane);
    check_child_and_parent(&child, &parent);
}

#[test]
fn chart_slice_uv_child_and_parent_share_their_point() {
    child_and_parent(Lane::Harness);
}

negative_control!(
    chart_slice_uv_child_and_parent_share_their_point,
    "TASK-M1-06's grid tiling reads each one-quad grid's own [0, 1]², so child and parent read 0 and ½",
    expected = "not the shared point",
    child_and_parent(Lane::Tiling)
);
