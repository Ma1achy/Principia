//! The coordinate convention (TASK-M1-07; principia_coordinate_conventions_note.md): the one framebuffer → UV flip, its
//! Rust twin, the image export, and the coordinate presets drawn through the synthetic harness.
//! - REQ-SYS-009: the WGSL flip, rendered at every row of a target, equals its Rust twin (`flip_twin_matches_wgsl_*`);
//! - REQ-SYS-080: the coordinate view's exported PNG has its top-left pixel at UV (0, 1) and its bottom-left at (0, 0),
//!   its rows written as the readback gives them (`export_orientation_*`);
//! - REQ-TOOL-019, REQ-TOOL-152: the coordinate view reconstructs each sampled quad's UV coordinate as
//!   c + h·(2t − 1), with c and h the synthetic harness's, and its adjacent deltas pass the banding criterion; the
//!   global form u_min + (u_max − u_min)·t, on a quad deep enough to band in f32, fails it
//!   (`uv_preset_reconstruction_*`);
//! - REQ-TOOL-153: the δ mode draws (δ/h + 1)/2 in R and G, B = 0 (`delta_mode_*`);
//! - colour_composition §6: `uv_screen` draws `ctx.screen.uv` and `uv_quad` `ctx.quad.uv` (`uv_presets_*`).
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

fn gpu() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

// ── REQ-SYS-009: the flip and its Rust twin ───────────────────────────────────────────────────────────────────────

/// The twin probe: `coords.wgsl` and an entry writing, at each pixel, the bits of `flip_y(frag_coord.y, H)` and of
/// `frag_uv(frag_coord.xy, (W, H))`.
fn twin_module(width: u32, height: u32) -> String {
    format!(
        "{}\n@fragment\nfn twin_fs(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<u32> {{\n    \
         let dims = vec2<f32>({width}.0, {height}.0);\n    \
         let uv = frag_uv(pos.xy, dims);\n    \
         return vec4<u32>(bitcast<u32>(flip_y(pos.y, dims.y)), bitcast<u32>(uv.x), bitcast<u32>(uv.y), 0u);\n}}\n",
        coords::WGSL
    )
}

fn draw_bare(h: &GpuHarness, module: &str, entry: &str, target: Target) -> Image {
    headless::render(
        h.device(),
        h.queue(),
        &Draw {
            module,
            entry,
            layouts: &[],
            groups: &[],
        },
        target,
    )
    .unwrap_or_else(|e| panic!("{e}"))
}

/// The probe drawn into a `width` × `height` target.
fn draw_twin(h: &GpuHarness, width: u32, height: u32) -> Image {
    draw_bare(
        h,
        &twin_module(width, height),
        "twin_fs",
        Target {
            width,
            height,
            format: wgpu::TextureFormat::Rgba32Uint,
        },
    )
}

/// The ulps a UV component may differ by: the fragment stage may compile with fast-math (R-297), and on Metal it does,
/// where a division is not correctly rounded (MSL's fast-math division is within 2.5 ulp), so `frag_uv`'s division by
/// a width or height that is not a power of two can land an ulp from the twin's. The flip itself, a subtraction of
/// pixel coordinates, is exact on both and compared bit for bit. One row of a target is `1/H` of v, millions of ulps.
const UV_ULPS: u32 = 3;

/// At every pixel of `image`, the WGSL flip equals the Rust twin's at the pixel `(x, y + shift)`'s centre bit for bit,
/// and its UV the twin's within [`UV_ULPS`].
fn check_twin(image: &Image, shift: u32) {
    let (w, hgt) = (image.width as f32, image.height as f32);
    for y in 0..image.height {
        for x in 0..image.width {
            let centre = [x as f32 + 0.5, (y + shift) as f32 + 0.5];
            let flip = coords::flip_y(centre[1], hgt);
            let uv = coords::frag_uv(centre, [w, hgt]);
            let got = image.words(x, y);
            let floats = [0, 1, 2].map(|k| f32::from_bits(got[k]));
            let close = |k: usize, want: f32| got[k].abs_diff(want.to_bits()) <= UV_ULPS;
            assert!(
                got[0] == flip.to_bits() && close(1, uv[0]) && close(2, uv[1]),
                "pixel ({x}, {y}) of {w} × {hgt}: the WGSL flip and UV {floats:?} are not the Rust twin's \
                 {flip}, {uv:?}"
            );
        }
    }
}

#[test]
fn flip_twin_matches_wgsl_at_every_row() {
    let h = gpu();
    for (width, height) in [(3, 37), (2, 256), (1, 1), (5, 600)] {
        check_twin(&draw_twin(&h, width, height), 0);
    }
}

negative_control!(
    flip_twin_matches_wgsl_at_every_row,
    "a twin off by one row fails",
    expected = "are not the Rust twin's",
    check_twin(&draw_twin(&gpu(), 3, 37), 1)
);

/// The flip on integer edges and rows, as the CPU raster calls it: edge `y` maps to `H − y`, pixel row `y` to Y-up row
/// `flip_y(y + 1, H)`, and the flip is its own inverse.
fn check_integer_flip(flip: fn(u32, u32) -> u32) {
    for height in [1u32, 2, 7, 64] {
        for y in 0..=height {
            assert_eq!(flip(y, height), height - y, "edge {y} of {height}");
            assert_eq!(flip(flip(y, height), height), y, "edge {y} of {height}");
        }
        assert_eq!(flip(1, height), height - 1, "the top row is Y-up row H − 1");
        assert_eq!(flip(height, height), 0, "the bottom row is Y-up row 0");
    }
}

#[test]
fn flip_twin_matches_wgsl_on_integer_rows() {
    check_integer_flip(coords::flip_y);
}

negative_control!(
    flip_twin_matches_wgsl_on_integer_rows,
    "the identity is no flip",
    expected = "edge 0 of 1",
    check_integer_flip(|y, _| y)
);

// ── The harness and the presets ──────────────────────────────────────────────────────────────────────────────────

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

/// The stain graph `graph` drawn through the preset harness (`bind::preset_module`) over `set` into a target of
/// `format`.
fn draw_preset(
    h: &GpuHarness,
    set: &Synthetic,
    graph: &StainGraph,
    format: wgpu::TextureFormat,
) -> Image {
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

/// The RGB floats of an `Rgba32Float` image's pixel `(x, y)`, `y` from the top.
fn rgb(image: &Image, x: u32, y: u32) -> [f32; 3] {
    let t = image.texel(x, y);
    [0, 1, 2].map(|k| f32::from_le_bytes([t[4 * k], t[4 * k + 1], t[4 * k + 2], t[4 * k + 3]]))
}

/// A flat set of `quads` × `quads` quads of `n` × `n` one-pixel tiles at depth `depth` from `origin`.
fn flat(quads: u32, n: u32, depth: u32, origin: [u64; 2]) -> Synthetic {
    let grid = Grid::new([quads, quads], n, 0, 1).expect("the preset grid");
    Synthetic::flat_at(grid, depth, origin)
}

/// The flat grid of M1: 4 × 4 quads at depth 2 tiling the slice, 8 × 8 samples each.
fn flat_grid() -> Synthetic {
    flat(4, 8, 2, [0, 0])
}

/// The negative fixture's quad: one quad at depth 30 whose centre is near 0.6, deep enough that the global form
/// u_min + (u_max − u_min)·t bands in f32 (deep_zoom §1's "at depth 30 the quad is ~10⁻⁹ wide").
fn deep_quad() -> Synthetic {
    let depth = 30;
    let cell = (0.6 * (1u64 << depth) as f64) as u64;
    flat(1, 8, depth, [cell, cell])
}

/// Sample `(i, j)` of quad `q`'s pixel (one-pixel tiles): its column and row from the top.
fn sample_pixel(set: &Synthetic, q: u32, i: u32, j: u32) -> (u32, u32) {
    let g = set.grid();
    let quad_xy = [q % g.quads[0], q / g.quads[0]];
    let (x0, y0, _, _) = g.tile_pixels(quad_xy, [i, j]);
    (x0, y0)
}

/// t of sample index `i` in a quad of `n`: (i + 0.5)/N (deep_zoom §1).
fn t(i: u32, n: u32) -> f64 {
    (f64::from(i) + 0.5) / f64::from(n)
}

// ── REQ-TOOL-019, REQ-TOOL-152: the UV preset's reconstruction ────────────────────────────────────────────────────

/// Each sample of each quad of `image` reconstructs `c + h·(2t − 1)` from the harness's c and h, to f32's rounding
/// (one ulp at 1, `f32::EPSILON`), and each quad's rows and columns of samples pass the banding criterion; returns the
/// largest departure seen.
fn check_reconstruction(set: &Synthetic, image: &Image) -> f64 {
    let n = set.grid().n;
    let mut worst: f64 = 0.0;
    for q in 0..set.grid().quad_count() {
        let f = set.quad_frame(q);
        let got = |i: u32, j: u32| {
            let (x, y) = sample_pixel(set, q, i, j);
            let p = rgb(image, x, y);
            [f64::from(p[0]), f64::from(p[1])]
        };
        for j in 0..n {
            for i in 0..n {
                let want = [
                    f.c[0] + f.h[0] * (2.0 * t(i, n) - 1.0),
                    f.c[1] + f.h[1] * (2.0 * t(j, n) - 1.0),
                ];
                let g = got(i, j);
                for k in 0..2 {
                    assert!(
                        (g[k] - want[k]).abs() <= f64::from(f32::EPSILON),
                        "quad {q} sample ({i}, {j}): the reconstructed UV {g:?} is not c + h·(2t − 1) = {want:?}"
                    );
                }
            }
        }
        for k in 0..n {
            let row: Vec<f64> = (0..n).map(|i| got(i, k)[0]).collect();
            let column: Vec<f64> = (0..n).map(|j| got(k, j)[1]).collect();
            for (run, h) in [(row, f.h[0]), (column, f.h[1])] {
                let step = 2.0 * h / f64::from(n);
                worst = worst.max(banding_departure(&run, step));
                assert!(
                    !banded(&run, step),
                    "quad {q}: the adjacent deltas of {run:?} depart from the step {step} by {} > {BANDING_BOUND}: banded",
                    banding_departure(&run, step)
                );
            }
        }
    }
    worst
}

fn coordinate(mode: CoordinateMode) -> StainGraph {
    presets::coordinate_view(mode).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn uv_preset_reconstruction_matches_c_plus_h_on_the_flat_grid() {
    let set = flat_grid();
    let image = draw_preset(
        &gpu(),
        &set,
        &coordinate(CoordinateMode::Uv),
        wgpu::TextureFormat::Rgba32Float,
    );
    let worst = check_reconstruction(&set, &image);
    println!(
        "flat grid: largest adjacent-delta departure {worst:e} of the step (bound {BANDING_BOUND})"
    );
}

negative_control!(
    uv_preset_reconstruction_matches_c_plus_h_on_the_flat_grid,
    "the quad-local coordinate t drawn in place of the reconstruction fails",
    expected = "is not c + h·(2t − 1)",
    {
        let set = flat_grid();
        let graph = presets::uv_quad().unwrap_or_else(|e| panic!("{e}"));
        let image = draw_preset(&gpu(), &set, &graph, wgpu::TextureFormat::Rgba32Float);
        check_reconstruction(&set, &image);
    }
);

/// The global form, u_min + (u_max − u_min)·t with u_min = c − h and u_max = c + h in f32 (deep_zoom §1's
/// "Problem"), as a stain through the preset harness: the negative fixture's form, not a preset.
fn global_form() -> StainGraph {
    let mut g = StainGraph::new();
    let s = g
        .add(
            NodeKind::Source,
            Occupant::Custom(
                "fn source(ctx: Ctx) -> Field {\n    let lo = ctx.quad.c - ctx.quad.h;\n    \
                 let hi = ctx.quad.c + ctx.quad.h;\n    return Field(lo + (hi - lo) * ctx.quad.uv, 0.0, 0.0);\n}\n"
                    .into(),
            ),
        )
        .expect("the global form's source");
    let c = g
        .add(
            NodeKind::Colour,
            Occupant::Custom(
                "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.inputs[0].xy, 0.0); }\n"
                    .into(),
            ),
        )
        .expect("the RG colour");
    g.connect(s, c, 0).expect("source → colour");
    g.connect(c, NodeId(0), 0).expect("colour → combiner");
    g
}

/// Each row of samples of the one quad of `image`, its R as f64.
fn rows(set: &Synthetic, image: &Image) -> Vec<Vec<f64>> {
    let n = set.grid().n;
    (0..n)
        .map(|j| {
            (0..n)
                .map(|i| {
                    let (x, y) = sample_pixel(set, 0, i, j);
                    f64::from(rgb(image, x, y)[0])
                })
                .collect()
        })
        .collect()
}

/// The negative fixture: on the deep quad, the global form's rows are banded by the criterion; returns the
/// departure.
fn check_global_form_bands(set: &Synthetic, image: &Image) -> f64 {
    let f = set.quad_frame(0);
    let step = 2.0 * f.h[0] / f64::from(set.grid().n);
    let mut least = f64::INFINITY;
    for row in rows(set, image) {
        let d = banding_departure(&row, step);
        least = least.min(d);
        assert!(
            banded(&row, step),
            "the global form's row {row:?} departs from the step {step:e} by only {d}: not banded"
        );
    }
    least
}

#[test]
fn uv_preset_reconstruction_global_form_bands_on_a_deep_quad() {
    let set = deep_quad();
    let image = draw_preset(
        &gpu(),
        &set,
        &global_form(),
        wgpu::TextureFormat::Rgba32Float,
    );
    let d = check_global_form_bands(&set, &image);
    println!(
        "deep quad (depth 30, c ≈ 0.6): the global form's smallest row departure {d} of the step"
    );
    // Evidence for REQ-TOOL-152's proposal, not asserted: the coordinate view's own f32 sum c + h·(2t − 1) at the same
    // quad, which no f32 can resolve there (deep_zoom §1: the precision lives in the quad-local offset).
    let view = draw_preset(
        &gpu(),
        &set,
        &coordinate(CoordinateMode::Uv),
        wgpu::TextureFormat::Rgba32Float,
    );
    let f = set.quad_frame(0);
    let step = 2.0 * f.h[0] / f64::from(set.grid().n);
    let worst = rows(&set, &view)
        .iter()
        .map(|row| banding_departure(row, step))
        .fold(0.0, f64::max);
    println!("deep quad (depth 30, c ≈ 0.6): the coordinate view's f32 sum's largest row departure {worst} of the step");
}

negative_control!(
    uv_preset_reconstruction_global_form_bands_on_a_deep_quad,
    "the global form on the shallow flat grid does not band",
    expected = "not banded",
    {
        let set = flat(1, 8, 2, [1, 1]);
        let image = draw_preset(
            &gpu(),
            &set,
            &global_form(),
            wgpu::TextureFormat::Rgba32Float,
        );
        check_global_form_bands(&set, &image);
    }
);

/// On the deep quad, the quad-local offset the δ mode draws, δ = h·(2x − 1) for its R = x, added to the harness's c in
/// f64, reconstructs each sample's u, and its rows pass the criterion: deep_zoom §1's fix, the precision kept in the
/// quad-local coordinate. Returns the largest departure.
fn check_local_reconstruction(set: &Synthetic, image: &Image) -> f64 {
    let f = set.quad_frame(0);
    let n = set.grid().n;
    let step = 2.0 * f.h[0] / f64::from(n);
    let mut worst: f64 = 0.0;
    for row in rows(set, image) {
        let u: Vec<f64> = row
            .iter()
            .map(|x| f.c[0] + f.h[0] * (2.0 * x - 1.0))
            .collect();
        for (i, got) in u.iter().enumerate() {
            let want = f.c[0] + f.h[0] * (2.0 * t(i as u32, n) - 1.0);
            assert!(
                (got - want).abs() <= f.h[0] * f64::from(f32::EPSILON),
                "sample {i}: c + δ = {got} is not c + h·(2t − 1) = {want}"
            );
        }
        worst = worst.max(banding_departure(&u, step));
        assert!(!banded(&u, step), "c + δ along {u:?} is banded");
    }
    worst
}

#[test]
fn uv_preset_reconstruction_quad_local_offset_holds_on_a_deep_quad() {
    let set = deep_quad();
    let image = draw_preset(
        &gpu(),
        &set,
        &coordinate(CoordinateMode::Delta),
        wgpu::TextureFormat::Rgba32Float,
    );
    let worst = check_local_reconstruction(&set, &image);
    println!("deep quad (depth 30, c ≈ 0.6): c + δ's largest departure {worst:e} of the step");
}

negative_control!(
    uv_preset_reconstruction_quad_local_offset_holds_on_a_deep_quad,
    "the global form's values read as the offset fail",
    expected = "is not c + h·(2t − 1)",
    {
        let set = deep_quad();
        let image = draw_preset(
            &gpu(),
            &set,
            &global_form(),
            wgpu::TextureFormat::Rgba32Float,
        );
        check_local_reconstruction(&set, &image);
    }
);

// ── REQ-TOOL-153: the δ mode ──────────────────────────────────────────────────────────────────────────────────────

/// Each sample of each quad of `image` draws (δ_u/h_u + 1)/2 in R and (δ_v/h_v + 1)/2 in G, with δ = h·(2t − 1) from
/// the harness's h and `t_of` the sample's t, to f32's rounding, and B = 0.
fn check_delta(set: &Synthetic, image: &Image, t_of: fn(u32, u32) -> f64) {
    let n = set.grid().n;
    for q in 0..set.grid().quad_count() {
        let f = set.quad_frame(q);
        for j in 0..n {
            for i in 0..n {
                let delta = [
                    f.h[0] * (2.0 * t_of(i, n) - 1.0),
                    f.h[1] * (2.0 * t_of(j, n) - 1.0),
                ];
                let want = [0, 1].map(|k| (delta[k] / f.h[k] + 1.0) / 2.0);
                let (x, y) = sample_pixel(set, q, i, j);
                let got = rgb(image, x, y);
                for k in 0..2 {
                    assert!(
                        (f64::from(got[k]) - want[k]).abs() <= f64::from(f32::EPSILON),
                        "quad {q} sample ({i}, {j}): the δ mode draws {got:?}, not (δ/h + 1)/2 = {want:?}"
                    );
                }
                assert_eq!(
                    got[2], 0.0,
                    "quad {q} sample ({i}, {j}): B is {}, not 0",
                    got[2]
                );
            }
        }
    }
}

#[test]
fn delta_mode_draws_normalised_offset() {
    let h = gpu();
    for set in [flat_grid(), deep_quad()] {
        let image = draw_preset(
            &h,
            &set,
            &coordinate(CoordinateMode::Delta),
            wgpu::TextureFormat::Rgba32Float,
        );
        check_delta(&set, &image, t);
    }
}

negative_control!(
    delta_mode_draws_normalised_offset,
    "a δ taken Y-down, t mirrored within the quad, fails",
    expected = "not (δ/h + 1)/2",
    {
        let set = flat_grid();
        let image = draw_preset(
            &gpu(),
            &set,
            &coordinate(CoordinateMode::Delta),
            wgpu::TextureFormat::Rgba32Float,
        );
        check_delta(&set, &image, |i, n| 1.0 - t(i, n));
    }
);

// ── colour_composition §6: uv_screen and uv_quad ──────────────────────────────────────────────────────────────────

/// Every pixel of `screen` draws its post-flip UV, the Rust twin's `frag_uv` at its centre, and every pixel of `quad`
/// its sample's t, (tile + ½)/N (one-pixel tiles), each to f32's rounding, B = 0.
fn check_uv_presets(set: &Synthetic, screen: &Image, quad: &Image) {
    let g = set.grid();
    let (w, hgt) = g.target();
    for y in 0..hgt {
        for x in 0..w {
            let uv = coords::frag_uv([x as f32 + 0.5, y as f32 + 0.5], [w as f32, hgt as f32]);
            let cell = g.cell(x, y);
            let tq = [0, 1].map(|k| t(cell.tile_xy[k], g.n));
            for (image, want, what) in [
                (screen, [f64::from(uv[0]), f64::from(uv[1])], "uv_screen"),
                (quad, tq, "uv_quad"),
            ] {
                let got = rgb(image, x, y);
                for k in 0..2 {
                    assert!(
                        (f64::from(got[k]) - want[k]).abs() <= f64::from(f32::EPSILON),
                        "{what} at ({x}, {y}) draws {got:?}, not {want:?}"
                    );
                }
                assert_eq!(got[2], 0.0, "{what} at ({x}, {y}): B is not 0");
            }
        }
    }
}

fn draw_uv_presets(h: &GpuHarness, set: &Synthetic) -> (Image, Image) {
    let f = wgpu::TextureFormat::Rgba32Float;
    let screen = presets::uv_screen().unwrap_or_else(|e| panic!("{e}"));
    let quad = presets::uv_quad().unwrap_or_else(|e| panic!("{e}"));
    (
        draw_preset(h, set, &screen, f),
        draw_preset(h, set, &quad, f),
    )
}

#[test]
fn uv_presets_draw_screen_and_quad_uv() {
    let h = gpu();
    let set = flat(3, 4, 5, [7, 2]);
    let (screen, quad) = draw_uv_presets(&h, &set);
    check_uv_presets(&set, &screen, &quad);
}

negative_control!(
    uv_presets_draw_screen_and_quad_uv,
    "the two presets swapped fail",
    expected = "draws",
    {
        let h = gpu();
        let set = flat(3, 4, 5, [7, 2]);
        let (screen, quad) = draw_uv_presets(&h, &set);
        check_uv_presets(&set, &quad, &screen);
    }
);

// ── The criterion's boundary (REQ-TOOL-152) ───────────────────────────────────────────────────────────────────────

/// A run of three values whose second delta departs from the step `1` by `d`: banded exactly when `d` exceeds the
/// bound, and `banding_departure` is `d`, the largest of its deltas'.
fn check_criterion(bound: f64) {
    assert_eq!(
        banding_departure(&[0.0], 1.0),
        0.0,
        "one value has no delta"
    );
    assert_eq!(banding_departure(&[], 1.0), 0.0, "no value has no delta");
    assert_eq!(
        banding_departure(&[0.0, 1.0, 2.0], 1.0),
        0.0,
        "exact steps depart by 0"
    );
    assert_eq!(
        banding_departure(&[0.0, -1.0, -2.0], -1.0),
        0.0,
        "a negative step departs by 0"
    );
    assert_eq!(
        banding_departure(&[0.0, 1.5, 2.5], 1.0),
        0.5,
        "the first delta's departure is the largest"
    );
    assert_eq!(
        banding_departure(&[0.0, -1.5, -2.5], -1.0),
        0.5,
        "a departure is a fraction of the step's size"
    );
    assert_eq!(
        banding_departure(&[0.0, 0.0], 1.0),
        1.0,
        "a zero delta departs by the whole step"
    );
    let at = [0.0, 1.0, 2.0 + bound];
    assert_eq!(banding_departure(&at, 1.0), bound);
    assert!(
        !banded(&at, 1.0),
        "a departure equal to the bound is smooth"
    );
    let over = [0.0, 1.0, 2.0 + 2.0 * bound];
    assert!(banded(&over, 1.0), "a departure over the bound is banded");
}

#[test]
fn uv_preset_reconstruction_criterion_boundary() {
    check_criterion(BANDING_BOUND);
}

negative_control!(
    uv_preset_reconstruction_criterion_boundary,
    "a test bound twice the criterion's puts the equal case over it",
    expected = "equal to the bound is smooth",
    check_criterion(2.0 * BANDING_BOUND)
);

// ── REQ-SYS-080: the export's orientation ─────────────────────────────────────────────────────────────────────────

/// The coordinate view over the flat grid, into an 8-bit target.
fn coordinate_view_rgba8(h: &GpuHarness) -> (Synthetic, Image) {
    let set = flat_grid();
    let image = draw_preset(
        h,
        &set,
        &coordinate(CoordinateMode::Uv),
        wgpu::TextureFormat::Rgba8Unorm,
    );
    (set, image)
}

/// The PNG `bytes` decoded: width, height and RGBA rows, top first, as stored.
fn decode(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .expect("the export is a PNG");
    let mut buf = vec![0; reader.output_buffer_size().expect("a sized image")];
    let info = reader.next_frame(&mut buf).expect("one frame");
    assert_eq!(
        (info.color_type, info.bit_depth),
        (png::ColorType::Rgba, png::BitDepth::Eight),
        "the export is 8-bit RGBA"
    );
    buf.truncate(info.buffer_size());
    (info.width, info.height, buf)
}

/// The exported PNG's corners: the top-left pixel draws UV (0, 1) and the bottom-left (0, 0), the top-right (1, 1)
/// and the bottom-right (1, 0), each to within the half-sample its tile's sample sits from the corner, plus an 8-bit
/// step; and every pixel is the readback's, row for row.
fn check_export(png_bytes: &[u8], image: &Image, sample: f64) {
    let (w, hgt, rgba) = decode(png_bytes);
    assert_eq!((w, hgt), (image.width, image.height), "the export's size");
    let px = |x: u32, y: u32| {
        let at = 4 * (y as usize * w as usize + x as usize);
        [rgba[at], rgba[at + 1], rgba[at + 2]]
    };
    let near = sample / 2.0 + 1.0 / 255.0;
    for (name, x, y, want) in [
        ("top-left", 0, 0, [0.0, 1.0]),
        ("bottom-left", 0, hgt - 1, [0.0, 0.0]),
        ("top-right", w - 1, 0, [1.0, 1.0]),
        ("bottom-right", w - 1, hgt - 1, [1.0, 0.0]),
    ] {
        let p = px(x, y);
        let uv = [f64::from(p[0]) / 255.0, f64::from(p[1]) / 255.0];
        assert!(
            (uv[0] - want[0]).abs() <= near && (uv[1] - want[1]).abs() <= near && p[2] == 0,
            "the exported PNG's {name} pixel draws UV {uv:?}, not {want:?}"
        );
    }
    assert_eq!(
        rgba, image.bytes,
        "the export's rows are not the readback's, in order"
    );
}

#[test]
fn export_orientation_top_left_is_uv_0_1() {
    let (set, image) = coordinate_view_rgba8(&gpu());
    let mut out = Vec::new();
    export::write_png(&image, &mut out).unwrap_or_else(|e| panic!("{e}"));
    let g = set.grid();
    check_export(&out, &image, 1.0 / f64::from(g.quads[0] * g.n));
    assert_eq!(
        export::png_rows(&image).count(),
        image.height as usize,
        "the seam gives one row per readback row"
    );
}

negative_control!(
    export_orientation_top_left_is_uv_0_1,
    "a row-reversed export fails",
    expected = "top-left pixel draws UV",
    {
        let (set, image) = coordinate_view_rgba8(&gpu());
        let row = image.width as usize * 4;
        let reversed = Image {
            bytes: image.bytes.chunks(row).rev().flatten().copied().collect(),
            ..image.clone()
        };
        let mut out = Vec::new();
        export::write_png(&reversed, &mut out).unwrap_or_else(|e| panic!("{e}"));
        let g = set.grid();
        check_export(&out, &image, 1.0 / f64::from(g.quads[0] * g.n));
    }
);

/// The export refuses an image it cannot write as 8-bit RGBA: another format, no texel, or bytes that are not
/// `width × height` texels, one short or one over; and writes one of exactly that size, in either RGBA8 format.
fn check_export_refusals(write: fn(&Image) -> Result<Vec<u8>, String>) {
    let image = |format, width, height, len| Image {
        width,
        height,
        format,
        bytes: vec![7; len],
    };
    let rgba = wgpu::TextureFormat::Rgba8Unorm;
    for (what, bad) in [
        (
            "a float target",
            image(wgpu::TextureFormat::Rgba32Float, 2, 2, 64),
        ),
        (
            "a BGRA target",
            image(wgpu::TextureFormat::Bgra8Unorm, 2, 2, 16),
        ),
        ("no texel", image(rgba, 0, 0, 0)),
        ("a byte short", image(rgba, 2, 2, 15)),
        ("a byte over", image(rgba, 2, 2, 17)),
    ] {
        assert!(write(&bad).is_err(), "the export wrote {what}");
    }
    for format in [rgba, wgpu::TextureFormat::Rgba8UnormSrgb] {
        let good = image(format, 2, 3, 24);
        let png = write(&good).unwrap_or_else(|e| panic!("{format:?}: {e}"));
        assert_eq!(decode(&png).2, good.bytes, "{format:?} round trip");
    }
}

fn write_png(image: &Image) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    export::write_png(image, &mut out).map(|()| out)
}

#[test]
fn export_orientation_refuses_what_it_cannot_write() {
    check_export_refusals(write_png);
}

negative_control!(
    export_orientation_refuses_what_it_cannot_write,
    "an export that writes anything fails",
    expected = "the export wrote",
    check_export_refusals(|_| Ok(Vec::new()))
);
