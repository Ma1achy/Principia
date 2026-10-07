//! QA tests for TASK-M1-07, written from the requirements it closes, not from its implementation:
//! - REQ-SYS-009: the one flip at the framebuffer → UV boundary, `v = 1 − frag_coord.y/H`, computed here from first
//!   principles at each pixel's centre and held against `coords.wgsl`'s `frag_uv`, the raster's `ctx.screen.uv` (the
//!   `uv_screen` preset) and the CPU raster's rows;
//! - REQ-SYS-080: the export writes the headless readback's rows unreversed (coordinate note, path 5), so the
//!   coordinate view's PNG has its top-left at UV (0, 1) and its bottom-left at (0, 0), green falling down every column;
//! - REQ-TOOL-019, REQ-TOOL-152: the coordinate view reconstructs u = c + h·(2t − 1) with c and h derived here from the
//!   quad's cell and depth (deep_zoom §1), its adjacent deltas pass the banding criterion, and the global form
//!   u_min + (u_max − u_min)·t, on quads deep enough, fails it; a depth sweep of the global form in f32 gives the
//!   evidence for the proposed bound;
//! - REQ-TOOL-153: the δ mode draws (δ/h + 1)/2, u's in R, v's in G, B = 0, on a quad deep enough that δ itself is
//!   ~1e−12, and agrees with `uv_quad` pixel for pixel;
//! - REQ-TOOL-027: the coordinate view's two modes: the UV mode's green rises monotonically up the whole target, the δ
//!   mode's restarts at each quad's bottom edge.
//!
//! Each test registers its negative control (R-176).

use render::assemble::{assemble, Tier};
use render::bind::{self, Context};
use render::coords::{self, banded, banding_departure, BANDING_BOUND};
use render::export;
use render::headless::{self, Draw, Image, Target};
use render::raster::Grid;
use validation::gpu::GpuHarness;
use validation::negative_control;
use validation::presets::{self, CoordinateMode};
use validation::stain::{NodeId, NodeKind, Occupant, StainGraph};
use validation::synthetic::Synthetic;

const F32: wgpu::TextureFormat = wgpu::TextureFormat::Rgba32Float;

/// Four f32 ulps at 1: f32's rounding of a UV in [0, 1], with the fragment stage's fast-math division (R-297; MSL's
/// fast-math division is within 2.5 ulp). One pixel row of the largest target here is 1/300 of v.
const UV_TOL: f64 = 4.0 * f32::EPSILON as f64;

fn gpu() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

/// The pixel `(x, y)`'s centre, `y` from the top, as UV by the convention: u = (x + ½)/W, v = 1 − (y + ½)/H.
fn uv_by_convention(x: u32, y: u32, w: u32, h: u32) -> [f64; 2] {
    [
        (f64::from(x) + 0.5) / f64::from(w),
        1.0 - (f64::from(y) + 0.5) / f64::from(h),
    ]
}

/// The same pixel's centre read Y-down, the flip skipped: the wrong-count picture.
#[cfg(feature = "controls")]
fn uv_unflipped(x: u32, y: u32, w: u32, h: u32) -> [f64; 2] {
    [
        (f64::from(x) + 0.5) / f64::from(w),
        (f64::from(y) + 0.5) / f64::from(h),
    ]
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

// ── REQ-SYS-009: the one flip, from first principles ─────────────────────────────────────────────────────────────

/// A fragment writing `coords.wgsl`'s `frag_uv(frag_coord.xy, (W, H))` into R and G.
fn frag_uv_probe(h: &GpuHarness, width: u32, height: u32) -> Image {
    let module = format!(
        "{}\n@fragment\nfn qa_probe(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {{\n    \
         return vec4<f32>(frag_uv(pos.xy, vec2<f32>({width}.0, {height}.0)), 0.0, 1.0);\n}}\n",
        coords::WGSL
    );
    headless::render(
        h.device(),
        h.queue(),
        &Draw {
            module: &module,
            entry: "qa_probe",
            layouts: &[],
            groups: &[],
        },
        Target {
            width,
            height,
            format: F32,
        },
    )
    .unwrap_or_else(|e| panic!("{e}"))
}

/// Every pixel of `image` draws `want(x, y, W, H)` in R and G to f32's rounding.
fn check_uv_image(image: &Image, want: fn(u32, u32, u32, u32) -> [f64; 2], what: &str) {
    let (w, h) = (image.width, image.height);
    for y in 0..h {
        for x in 0..w {
            let got = rgb(image, x, y);
            let want = want(x, y, w, h);
            assert!(
                (got[0] - want[0]).abs() <= UV_TOL && (got[1] - want[1]).abs() <= UV_TOL,
                "{what}: pixel ({x}, {y}) of {w} × {h} draws UV {:?}, not v = 1 − frag_coord.y/H's {want:?}",
                [got[0], got[1]]
            );
        }
    }
}

fn check_wgsl_flip(want: fn(u32, u32, u32, u32) -> [f64; 2]) {
    let h = gpu();
    for (w, hgt) in [(7, 13), (1, 300), (64, 64), (5, 2)] {
        check_uv_image(&frag_uv_probe(&h, w, hgt), want, "coords.wgsl's frag_uv");
    }
}

#[test]
fn qa_wgsl_flip_is_one_minus_frag_y_over_h() {
    check_wgsl_flip(uv_by_convention);
}

negative_control!(
    qa_wgsl_flip_is_one_minus_frag_y_over_h,
    "the flip skipped, v read Y-down, is not the render's",
    expected = "not v = 1 − frag_coord.y/H's",
    check_wgsl_flip(uv_unflipped)
);

/// The Rust twin, on pixel centres and on events, is the same flip: `frag_uv` is (x/W, 1 − y/H), `flip_y` is its own
/// inverse, and the CPU raster puts framebuffer row `y` at Y-up row `H − 1 − y`, so the top row is in the top quad
/// row and the bottom row in quad row 0.
fn check_rust_twin(frag_uv: fn([f64; 2], [f64; 2]) -> [f64; 2]) {
    for (w, h) in [(7u32, 13u32), (1, 300), (64, 64)] {
        for y in 0..h {
            for x in 0..w {
                let got = frag_uv(
                    [f64::from(x) + 0.5, f64::from(y) + 0.5],
                    [f64::from(w), f64::from(h)],
                );
                let want = uv_by_convention(x, y, w, h);
                assert!(
                    (got[0] - want[0]).abs() <= 1e-15 && (got[1] - want[1]).abs() <= 1e-15,
                    "the Rust twin at pixel ({x}, {y}) of {w} × {h} gives {got:?}, not the convention's {want:?}"
                );
            }
        }
        for y in 0..=h {
            assert_eq!(coords::flip_y(coords::flip_y(y, h), h), y, "flip_y twice");
        }
    }
    let grid = Grid::new([3, 2], 2, 0, 3).expect("grid");
    let (w, h) = grid.target();
    for y in 0..h {
        for x in 0..w {
            let up = (h - 1 - y) / grid.tile_px;
            let cell = grid.cell(x, y);
            assert_eq!(
                [cell.quad_xy[1], cell.tile_xy[1]],
                [up / grid.n, up % grid.n],
                "the CPU raster puts framebuffer row {y} at the wrong Y-up row"
            );
        }
    }
}

#[test]
fn qa_rust_twin_is_the_same_flip() {
    check_rust_twin(coords::frag_uv);
}

negative_control!(
    qa_rust_twin_is_the_same_flip,
    "a twin that does not flip fails",
    expected = "not the convention's",
    check_rust_twin(|f, d| [f[0] / d[0], f[1] / d[1]])
);

// ── The harness ──────────────────────────────────────────────────────────────────────────────────────────────────

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

fn draw(h: &GpuHarness, set: &Synthetic, graph: &StainGraph, format: wgpu::TextureFormat) -> Image {
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
            format,
        },
    )
    .unwrap_or_else(|e| panic!("{e}"))
}

fn preset(g: Result<StainGraph, validation::stain::GraphError>) -> StainGraph {
    g.unwrap_or_else(|e| panic!("{e}"))
}

fn set(quads: [u32; 2], n: u32, tile_px: u32, depth: u32, origin: [u64; 2]) -> Synthetic {
    Synthetic::flat_at(
        Grid::new(quads, n, 0, tile_px).expect("grid"),
        depth,
        origin,
    )
}

// ── REQ-SYS-009 through the raster: uv_screen draws the post-flip UV ─────────────────────────────────────────────

fn check_uv_screen(want: fn(u32, u32, u32, u32) -> [f64; 2]) {
    let s = set([3, 2], 2, 3, 4, [1, 2]);
    let image = draw(&gpu(), &s, &preset(presets::uv_screen()), F32);
    check_uv_image(&image, want, "uv_screen");
    for y in 0..image.height {
        for x in 0..image.width {
            assert_eq!(rgb(&image, x, y)[2], 0.0, "uv_screen's B at ({x}, {y})");
        }
    }
}

#[test]
fn qa_uv_screen_draws_the_post_flip_uv() {
    check_uv_screen(uv_by_convention);
}

negative_control!(
    qa_uv_screen_draws_the_post_flip_uv,
    "the raster's UV read Y-down fails",
    expected = "not v = 1 − frag_coord.y/H's",
    check_uv_screen(uv_unflipped)
);

// ── REQ-TOOL-019: the reconstruction, c and h from first principles ──────────────────────────────────────────────

/// deep_zoom §1: the depth-ℓ cell `(column, row)` has centre `(cell + ½)·2^−ℓ` and half-width `2^−(ℓ+1)`.
fn frame(depth: u32, cell: [u64; 2]) -> ([f64; 2], f64) {
    let h = 0.5f64.powi(depth as i32 + 1);
    (cell.map(|k| (2.0 * k as f64 + 1.0) * h), h)
}

/// t = (i + ½)/N (deep_zoom §1).
fn t(i: u32, n: u32) -> f64 {
    (f64::from(i) + 0.5) / f64::from(n)
}

/// The pixel of sample `(i, j)` of grid quad `(qi, qj)`, one-pixel tiles: column `qi·N + i`, Y-up row `qj·N + j`,
/// framebuffer row `H − 1 −` that.
fn pixel(g: &Grid, q: [u32; 2], s: [u32; 2]) -> (u32, u32) {
    let (_, h) = g.target();
    (q[0] * g.n + s[0], h - 1 - (q[1] * g.n + s[1]))
}

/// The f32 ulp of `x`.
fn ulp(x: f64) -> f64 {
    let x = (x as f32).abs();
    f64::from(f32::from_bits(x.to_bits() + 1) - x)
}

/// Each sample of `image`, the coordinate view over `s` (depth `depth`, cells from `origin`), draws
/// c + h·(2t − 1) with `t_of` its t, to two f32 ulps (c's rounding and the sum's), and each row's u and each column's
/// v pass the banding criterion. Returns the largest departure seen.
fn check_reconstruction(
    s: &Synthetic,
    depth: u32,
    origin: [u64; 2],
    image: &Image,
    t_of: fn(u32, u32) -> f64,
) -> f64 {
    let g = s.grid();
    let n = g.n;
    let mut worst: f64 = 0.0;
    for qj in 0..g.quads[1] {
        for qi in 0..g.quads[0] {
            let (c, h) = frame(
                depth,
                [origin[0] + u64::from(qi), origin[1] + u64::from(qj)],
            );
            let f = s.quad_frame(qj * g.quads[0] + qi);
            assert_eq!(
                (f.c, f.h),
                (c, [h, h]),
                "the harness's frame of quad ({qi}, {qj}) is not deep_zoom §1's"
            );
            let at = |i: u32, j: u32| {
                let (x, y) = pixel(&g, [qi, qj], [i, j]);
                rgb(image, x, y)
            };
            for j in 0..n {
                for i in 0..n {
                    let want = [
                        c[0] + h * (2.0 * t_of(i, n) - 1.0),
                        c[1] + h * (2.0 * t_of(j, n) - 1.0),
                    ];
                    let got = at(i, j);
                    for k in 0..2 {
                        assert!(
                            (got[k] - want[k]).abs() <= 2.0 * ulp(want[k]),
                            "quad ({qi}, {qj}) sample ({i}, {j}): the view draws {:?}, not c + h·(2t − 1) = {want:?}",
                            [got[0], got[1]]
                        );
                    }
                    assert_eq!(got[2], 0.0, "B at quad ({qi}, {qj}) sample ({i}, {j})");
                }
            }
            let step = 2.0 * h / f64::from(n);
            for k in 0..n {
                let row: Vec<f64> = (0..n).map(|i| at(i, k)[0]).collect();
                let col: Vec<f64> = (0..n).map(|j| at(k, j)[1]).collect();
                for run in [row, col] {
                    let d = banding_departure(&run, step);
                    worst = worst.max(d);
                    assert!(
                        !banded(&run, step),
                        "quad ({qi}, {qj}): {run:?} departs from the step {step:e} by {d:e}: banded"
                    );
                }
            }
        }
    }
    worst
}

/// The fixtures: M1's flat grid tiling the slice (4 × 4 quads at depth 2, 8 × 8 samples) and an offset 3 × 2 block at
/// depth 9.
fn reconstruction_fixtures() -> [([u32; 2], u32, u32, [u64; 2]); 2] {
    [([4, 4], 8, 2, [0, 0]), ([3, 2], 4, 9, [301, 77])]
}

fn run_reconstruction(t_of: fn(u32, u32) -> f64) {
    let h = gpu();
    let view = preset(presets::coordinate_view(CoordinateMode::Uv));
    for (quads, n, depth, origin) in reconstruction_fixtures() {
        let s = set(quads, n, 1, depth, origin);
        let image = draw(&h, &s, &view, F32);
        let worst = check_reconstruction(&s, depth, origin, &image, t_of);
        println!(
            "qa: {quads:?} quads, N = {n}, depth {depth}: largest adjacent-delta departure {worst:e} of 2h/N (bound {BANDING_BOUND})"
        );
    }
}

#[test]
fn qa_uv_preset_reconstruction_from_first_principles() {
    run_reconstruction(t);
}

negative_control!(
    qa_uv_preset_reconstruction_from_first_principles,
    "t taken Y-down within the quad, the mirrored decode, fails",
    expected = "not c + h·(2t − 1)",
    run_reconstruction(|i, n| 1.0 - t(i, n))
);

// ── REQ-TOOL-152: the negative fixture and the bound's evidence ──────────────────────────────────────────────────

/// The global form u_min + (u_max − u_min)·t of deep_zoom §1's "Problem", in f32 on the CPU (IEEE, correctly
/// rounded), along one row of a depth-`depth` quad whose centre is the cell nearest `at`.
fn global_form_row(depth: u32, at: f64, n: u32) -> (Vec<f64>, f64) {
    let cell = (at * (1u64 << depth) as f64) as u64;
    let (c, h) = frame(depth, [cell, cell]);
    let (c32, h32) = (c[0] as f32, h as f32);
    let (lo, hi) = (c32 - h32, c32 + h32);
    let row = (0..n)
        .map(|i| f64::from(lo + (hi - lo) * t(i, n) as f32))
        .collect();
    (row, 2.0 * h / f64::from(n))
}

/// Over depths 0 to 30, at a centre in [0.5, 1) (f32's ulp 2^−24 there): the global form is smooth by the criterion
/// while its step 2h/N ≥ 2^−(ℓ+3) spans at least 48 ulps (each value within 1.5 ulp, so a delta within 3, 3/48 = the
/// bound): ℓ ≤ 15; and banded once the step 2h/N ≤ 2^−(ℓ+2) is under half an ulp, when adjacent samples round to one
/// value (departure 1): ℓ ≥ 23. Both edges follow from f32's mantissa and the bound, not from the run.
///
/// N = 8 and the harness's dyadic frames make every value exact until the step falls under an ulp, so the departure
/// there is 0 or at least 1 and any bound in (0, 1) reads it alike; N = 6, whose t = (i + ½)/6 rounds, gives the
/// departures between, where the bound's value decides. Both run over the same edges.
fn check_sweep(banded: fn(&[f64], f64) -> bool) {
    for (depth, n) in (0..=30).flat_map(|d| [(d, 8), (d, 6)]) {
        let (row, step) = global_form_row(depth, 0.6, n);
        let d = banding_departure(&row, step);
        println!(
            "qa: global form in f32, depth {depth:2}, N = {n}, c ≈ 0.6: departure {d:e} of 2h/N"
        );
        if depth <= 15 {
            assert!(!banded(&row, step), "the global form at depth {depth} is banded ({d:e}), though its step spans ≥ 48 ulps");
        }
        if depth >= 23 {
            assert!(
                banded(&row, step),
                "the global form at depth {depth} departs by {d:e}: not banded"
            );
        }
    }
}

#[test]
fn qa_uv_preset_reconstruction_global_form_depth_sweep() {
    check_sweep(banded);
}

negative_control!(
    qa_uv_preset_reconstruction_global_form_depth_sweep,
    "a criterion that never bands misses the deep global form",
    expected = "not banded",
    check_sweep(|_, _| false)
);

/// The global form as a stain through the preset harness: lo + (hi − lo)·t with lo = c − h, hi = c + h, in R and G.
fn global_form() -> StainGraph {
    let mut g = StainGraph::new();
    let s = g
        .add(
            NodeKind::Source,
            Occupant::Custom(
                "fn source(ctx: Ctx) -> Field {\n    let lo = ctx.quad.centre - ctx.quad.half_width;\n    \
                 let span = (ctx.quad.centre + ctx.quad.half_width) - lo;\n    \
                 return Field(lo + span * ctx.quad.uv, 0.0, 0.0);\n}\n"
                    .into(),
            ),
        )
        .expect("source");
    let c = g
        .add(
            NodeKind::Colour,
            Occupant::Custom(
                "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.inputs[0].xy, 0.0); }\n"
                    .into(),
            ),
        )
        .expect("colour");
    g.connect(s, c, 0).expect("source → colour");
    g.connect(c, NodeId(0), 0).expect("colour → combiner");
    g
}

/// On the GPU, at a depth-26 quad near 0.71 (another fixture than the sweep's): every row of the global form, R, and
/// every column, G, is banded by the criterion.
fn check_gpu_global_form(depth: u32) {
    let origin = (0.71 * (1u64 << depth) as f64) as u64;
    let s = set([1, 1], 8, 1, depth, [origin, origin]);
    let image = draw(&gpu(), &s, &global_form(), F32);
    let g = s.grid();
    let (_, h) = frame(depth, [origin, origin]);
    let step = 2.0 * h / f64::from(g.n);
    for k in 0..g.n {
        let row: Vec<f64> = (0..g.n)
            .map(|i| {
                let (x, y) = pixel(&g, [0, 0], [i, k]);
                rgb(&image, x, y)[0]
            })
            .collect();
        let col: Vec<f64> = (0..g.n)
            .map(|j| {
                let (x, y) = pixel(&g, [0, 0], [k, j]);
                rgb(&image, x, y)[1]
            })
            .collect();
        for run in [row, col] {
            let d = banding_departure(&run, step);
            assert!(
                banded(&run, step),
                "depth {depth}: the global form's {run:?} departs by {d:e}: not banded"
            );
        }
    }
}

#[test]
fn qa_uv_preset_reconstruction_global_form_bands_on_the_gpu() {
    check_gpu_global_form(26);
}

negative_control!(
    qa_uv_preset_reconstruction_global_form_bands_on_the_gpu,
    "the same form on a shallow quad does not band",
    expected = "not banded",
    check_gpu_global_form(3)
);

// ── REQ-TOOL-153: the δ mode ─────────────────────────────────────────────────────────────────────────────────────

/// On 2 × 3 quads at depth 40 (δ ≈ 1e−12, so only the normalisation by h = 2^−41 brings it to [−1, 1]), each sample
/// draws (δ_u/h + 1)/2 in R and (δ_v/h + 1)/2 in G, δ = h·(2t − 1) with t = `t_of`, and B = 0; and the δ mode agrees with
/// `uv_quad` pixel for pixel (REQ-TOOL-153's note: in exact arithmetic it draws t).
fn check_delta(t_of: fn(u32, u32) -> f64) {
    let h = gpu();
    let depth = 40;
    let s = set([2, 3], 4, 1, depth, [12_345, 678]);
    let delta = draw(
        &h,
        &s,
        &preset(presets::coordinate_view(CoordinateMode::Delta)),
        F32,
    );
    let quad = draw(&h, &s, &preset(presets::uv_quad()), F32);
    let g = s.grid();
    let (_, hw) = frame(depth, [0, 0]);
    for qj in 0..g.quads[1] {
        for qi in 0..g.quads[0] {
            for j in 0..g.n {
                for i in 0..g.n {
                    let d = [
                        hw * (2.0 * t_of(i, g.n) - 1.0),
                        hw * (2.0 * t_of(j, g.n) - 1.0),
                    ];
                    let want = d.map(|x| (x / hw + 1.0) / 2.0);
                    let (x, y) = pixel(&g, [qi, qj], [i, j]);
                    let got = rgb(&delta, x, y);
                    assert!(
                        (got[0] - want[0]).abs() <= UV_TOL && (got[1] - want[1]).abs() <= UV_TOL && got[2] == 0.0,
                        "quad ({qi}, {qj}) sample ({i}, {j}): the δ mode draws {got:?}, not ((δ/h + 1)/2, B = 0) = {want:?}"
                    );
                    let q = rgb(&quad, x, y);
                    assert!(
                        (got[0] - q[0]).abs() <= UV_TOL && (got[1] - q[1]).abs() <= UV_TOL,
                        "quad ({qi}, {qj}) sample ({i}, {j}): the δ mode {got:?} and uv_quad {q:?} disagree"
                    );
                }
            }
        }
    }
}

#[test]
fn qa_delta_mode_draws_the_normalised_offset() {
    check_delta(t);
}

negative_control!(
    qa_delta_mode_draws_the_normalised_offset,
    "δ taken Y-down within the quad fails",
    expected = "not ((δ/h + 1)/2, B = 0)",
    check_delta(|i, n| 1.0 - t(i, n))
);

// ── REQ-TOOL-027: the coordinate view's two modes ────────────────────────────────────────────────────────────────

/// Over M1's flat grid tiling the slice (4 × 4 quads, N = 4, 2-px tiles): `mode`'s green, read up a column, either
/// rises at every sample row across the whole target (`continuous`, the UV mode: v is the slice's), or restarts at
/// each quad's bottom edge, its top sample above its bottom one (the δ mode: v is the quad's). Red likewise along a
/// row. Returns nothing; panics with the first disagreement.
fn check_modes(mode: CoordinateMode, continuous: bool) {
    let s = set([4, 4], 4, 2, 2, [0, 0]);
    let image = draw(&gpu(), &s, &preset(presets::coordinate_view(mode)), F32);
    let g = s.grid();
    let (w, h) = g.target();
    let t_px = g.tile_px;
    let x = w / 2 + 1;
    // Sample rows from the bottom: Y-up sample row r is framebuffer row H − 1 − r·tile_px.
    let rows = h / t_px;
    for r in 1..rows {
        let below = rgb(&image, x, h - 1 - (r - 1) * t_px)[1];
        let above = rgb(&image, x, h - 1 - r * t_px)[1];
        let quad_edge = r % g.n == 0;
        let rises = above > below;
        let want = continuous || !quad_edge;
        assert_eq!(
            rises,
            want,
            "{mode:?}: green from sample row {} ({below}) to {r} ({above}) {} a quad edge",
            r - 1,
            if quad_edge { "across" } else { "inside" }
        );
    }
    let y = h / 2 + 1;
    for c in 1..(w / t_px) {
        let left = rgb(&image, (c - 1) * t_px, y)[0];
        let right = rgb(&image, c * t_px, y)[0];
        assert_eq!(
            right > left,
            continuous || c % g.n != 0,
            "{mode:?}: red from column {} to {c}",
            c - 1
        );
    }
    let top_left = rgb(&image, 0, 0);
    assert!(
        top_left[1] > 0.9 && top_left[0] < 0.1,
        "{mode:?}: the top-left pixel draws {top_left:?}, not high green and low red"
    );
}

#[test]
fn qa_coordinate_view_two_modes() {
    check_modes(CoordinateMode::Uv, true);
    check_modes(CoordinateMode::Delta, false);
}

negative_control!(
    qa_coordinate_view_two_modes,
    "the δ mode held to the UV mode's continuous gradient fails at a quad edge",
    expected = "across a quad edge",
    check_modes(CoordinateMode::Delta, true)
);

// ── REQ-SYS-080: the export ──────────────────────────────────────────────────────────────────────────────────────

fn decode(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .expect("a PNG");
    let mut buf = vec![0; reader.output_buffer_size().expect("sized")];
    let info = reader.next_frame(&mut buf).expect("a frame");
    buf.truncate(info.buffer_size());
    (info.width, info.height, buf)
}

/// An export: the image to PNG bytes.
type Export = fn(&Image) -> Vec<u8>;

fn the_export(image: &Image) -> Vec<u8> {
    let mut out = Vec::new();
    export::write_png(image, &mut out).unwrap_or_else(|e| panic!("{e}"));
    out
}

/// A compensating flip at export: the readback's rows reversed, then written.
#[cfg(feature = "controls")]
fn a_reversing_export(image: &Image) -> Vec<u8> {
    let row = image.width as usize * 4;
    the_export(&Image {
        bytes: image.bytes.chunks(row).rev().flatten().copied().collect(),
        ..image.clone()
    })
}

/// A hand-filled readback, row k's every byte k (rows from the top): the PNG's row k is row k.
fn check_rows_unreversed(export: Export) {
    let (w, h) = (3u32, 9u32);
    let image = Image {
        width: w,
        height: h,
        format: wgpu::TextureFormat::Rgba8Unorm,
        bytes: (0..h).flat_map(|k| vec![k as u8; 4 * w as usize]).collect(),
    };
    let (pw, ph, rgba) = decode(&export(&image));
    assert_eq!((pw, ph), (w, h));
    for (k, row) in rgba.chunks(4 * w as usize).enumerate() {
        assert!(
            row.iter().all(|&b| b == k as u8),
            "the PNG's row {k} is {row:?}: rows reversed"
        );
    }
}

#[test]
fn qa_export_orientation_rows_unreversed() {
    check_rows_unreversed(the_export);
}

negative_control!(
    qa_export_orientation_rows_unreversed,
    "a compensating flip at export reverses the rows",
    expected = "rows reversed",
    check_rows_unreversed(a_reversing_export)
);

/// The coordinate view over the flat grid tiling the slice, exported: the PNG's top-left pixel is UV (0, 1) and its
/// bottom-left (0, 0) (to half a sample spacing, where the corner sample sits, plus half an 8-bit level); green never
/// rises down any column and red never falls along any row; B = 0.
fn check_exported_view(export: Export) {
    let s = set([4, 4], 4, 2, 2, [0, 0]);
    let image = draw(
        &gpu(),
        &s,
        &preset(presets::coordinate_view(CoordinateMode::Uv)),
        wgpu::TextureFormat::Rgba8Unorm,
    );
    let (w, h, rgba) = decode(&export(&image));
    let px = |x: u32, y: u32| {
        let at = 4 * (y * w + x) as usize;
        [rgba[at], rgba[at + 1], rgba[at + 2]]
    };
    let half_sample = 0.5 / f64::from(s.grid().quads[0] * s.grid().n);
    let near = half_sample + 0.5 / 255.0;
    for (name, x, y, want) in [
        ("top-left", 0, 0, [0.0, 1.0]),
        ("bottom-left", 0, h - 1, [0.0, 0.0]),
    ] {
        let p = px(x, y);
        let uv = [f64::from(p[0]) / 255.0, f64::from(p[1]) / 255.0];
        assert!(
            (uv[0] - want[0]).abs() <= near && (uv[1] - want[1]).abs() <= near && p[2] == 0,
            "the exported {name} pixel is UV {uv:?}, not {want:?}"
        );
    }
    for x in 0..w {
        for y in 1..h {
            assert!(
                px(x, y)[1] <= px(x, y - 1)[1],
                "green rises down column {x} at row {y} of the export"
            );
        }
    }
    for y in 0..h {
        for x in 1..w {
            assert!(
                px(x, y)[0] >= px(x - 1, y)[0],
                "red falls along row {y} at column {x} of the export"
            );
        }
    }
}

#[test]
fn qa_export_orientation_of_the_coordinate_view() {
    check_exported_view(the_export);
}

negative_control!(
    qa_export_orientation_of_the_coordinate_view,
    "a row-reversed export puts UV (0, 0) at the top-left",
    expected = "the exported top-left pixel is UV",
    check_exported_view(a_reversing_export)
);
