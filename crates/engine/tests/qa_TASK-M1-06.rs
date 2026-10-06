//! QA's tests for TASK-M1-06, written from the requirements and their sources, not from the implementation:
//! - REQ-RENDER-004 (canonical spec §8): QUAD → N×N SAMPLES → one TILE per sample → PIXELS, no interpolation. Checked
//!   on the rendered picture by geometry alone: every tile's pixel square is one stored value, every tile a different
//!   sample, every pixel bit-equal to a value the CPU wrote (no blend), and the lanes' `sample_index` and `quad.index`
//!   point at the sample and quad whose values the pixel shows.
//! - REQ-COL-056 (colour_composition §3): `ctx.tile.uv` and `ctx.quad.uv` in [0, 1]², origin bottom-left, Y-up, and
//!   `ctx.quad.uv = (vec2(i, j) + ctx.tile.uv) / N`; checked exactly on a dyadic grid.
//! - REQ-RENDER-008 (render contract Part 1): `RenderContext {sample: SimState, ic: ICDescriptor, quad: RenderQuad, uv,
//!   screen_uv, time}` read from naga's IR, and each member carrying the uploaded values to the fragment.
//! - REQ-COL-003 (colour_composition §3): every lane and every field §3 names, with naga's types; every payload field
//!   paired with a bool validity; the screen, chart, tile, quad and validity lanes carrying their sources' values.
//! - REQ-TOOL-014 (debug tooling plan, "Synthetic-first"): a bitwise-adversarial descriptor and hand-chosen values,
//!   CPU-filled, asserted on both surfaces: the Rust accessor returns it and the shader renders it.
//! - REQ-RENDER-007 (render contract Part 1): no compute pass and no writable buffer in the fragment modules, and no
//!   buffer of the unpacked logical struct.
//!
//! Each test registers its negative control (R-176).

use std::collections::{HashMap, HashSet};

use engine::synthetic::Synthetic;
use kernel::payload::{
    fgw_pack, pa_d_min, pb_dE_max, pb_dLz_max, sd_detail, sd_dmin_pair, sd_last_symbol,
    sd_saturated, sd_state, tm_t_dmin_step, tm_t_end_step,
};
use render::bind::{self, Context, ViewOutput};
use render::headless::{self, Draw, Image, Target};
use render::raster::Grid;
use validation::gpu::GpuHarness;
use validation::negative_control;

// ── Harness ─────────────────────────────────────────────────────────────────────────────────────────────────────

fn gpu() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

fn context(grid: Grid) -> Context {
    Context {
        dt_macro: 0.015625,
        delta_0: 1e-6,
        n_renorm: 8,
        horizon_steps: 4096,
        z: [1.5, -2.25, 0.125, 3.0, -0.0625, 7.5, 0.75, -9.0],
        grid,
        chart_id: 11,
        time: 1234.5,
        ensemble_spread: 0.375,
        out_of_chart: false,
        valid_sample_count: 3,
    }
}

/// `view` (`fn view(rc: RenderContext, l: Lanes) -> vec4<u32>`) over `set`'s buffers into an `Rgba32Uint` target the
/// size of `ctx`'s grid.
fn draw(h: &GpuHarness, set: &Synthetic, ctx: &Context, view: &str) -> Image {
    let module = bind::module(view, ViewOutput::Words).unwrap_or_else(|e| panic!("{e}"));
    let bytes = set.bytes();
    let bound = bind::upload(h.device(), &bytes.payload(), ctx);
    let (width, height) = ctx.grid.target();
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

fn view(body: &str) -> String {
    format!("fn view(rc: RenderContext, l: Lanes) -> vec4<u32> {{ return {body}; }}")
}

/// `image` with every row moved one pixel right (wrapping): a tile edge off by one.
#[cfg(feature = "controls")]
fn shifted(image: &Image) -> Image {
    let size = image.texel_size();
    let row = image.width as usize * size;
    let mut bytes = Vec::with_capacity(image.bytes.len());
    for r in image.bytes.chunks(row) {
        bytes.extend_from_slice(&r[row - size..]);
        bytes.extend_from_slice(&r[..row - size]);
    }
    Image {
        bytes,
        ..image.clone()
    }
}

/// `image` with its rows reversed: a Y-down picture.
#[cfg(feature = "controls")]
fn flipped(image: &Image) -> Image {
    let row = image.width as usize * image.texel_size();
    let bytes = image.bytes.chunks(row).rev().flatten().copied().collect();
    Image {
        bytes,
        ..image.clone()
    }
}

// ── REQ-RENDER-004: one sample per tile, no interpolation ───────────────────────────────────────────────────────

/// The grids the rasterisation is checked on: non-square quad counts, N from 1 to 3, E from 0 to 2, tiles of 1, 4
/// and 5 px.
fn raster_grids() -> Vec<Grid> {
    [
        ([2, 3], 3, 2, 5),
        ([1, 1], 1, 0, 4),
        ([4, 1], 2, 0, 1),
        ([3, 2], 2, 1, 4),
    ]
    .into_iter()
    .map(|(q, n, e, t)| Grid::new(q, n, e, t).expect("a qa grid"))
    .collect()
}

/// Every sample, copies included, with its own `E_0`, `i + 1`.
fn unique_e0(grid: Grid) -> Synthetic {
    let mut set = Synthetic::flat(grid, 0);
    for i in 0..grid.sample_count() {
        set.sample(i).E_0(i as f32 + 1.0);
    }
    set
}

/// By geometry alone (canonical spec §8): the target is the quads' N·tile_px squares; every pixel's first word is
/// bit-equal to an `E_0` the CPU wrote (no blend of two samples); every `tile_px` square is one value; and every tile
/// shows a different sample, so the quads × N² tiles show quads × N² samples.
fn check_tiles(grid: Grid, written: &HashSet<u32>, image: &Image) {
    let t = grid.tile_px;
    let (w, h) = (image.width, image.height);
    assert_eq!(
        (w, h),
        (grid.quads[0] * grid.n * t, grid.quads[1] * grid.n * t),
        "the target is not the quads' tiles"
    );
    let mut seen: HashMap<u32, (u32, u32)> = HashMap::new();
    for by in 0..h / t {
        for bx in 0..w / t {
            let v = image.words(bx * t, by * t)[0];
            for y in by * t..(by + 1) * t {
                for x in bx * t..(bx + 1) * t {
                    let p = image.words(x, y)[0];
                    assert!(
                        written.contains(&p),
                        "pixel ({x}, {y}) shows {p:#x}, no sample's value: an interpolated pixel"
                    );
                    assert_eq!(
                        p, v,
                        "tile square ({bx}, {by}) is not one sample: pixel ({x}, {y}) differs"
                    );
                }
            }
            if let Some(other) = seen.insert(v, (bx, by)) {
                panic!("tiles {other:?} and ({bx}, {by}) show the same sample");
            }
        }
    }
    assert_eq!(
        seen.len() as u32,
        grid.quad_count() * grid.n * grid.n,
        "the tiles do not show one sample each"
    );
}

const E0_VIEW: &str = "fn view(rc: RenderContext, l: Lanes) -> vec4<u32> {
    return vec4<u32>(bitcast<u32>(rc.sample.E_0), 0u, 0u, 0u);
}";

fn written_e0(set: &Synthetic) -> HashSet<u32> {
    (0..set.grid().sample_count())
        .map(|i| set.simstate(i).E_0.to_bits())
        .collect()
}

#[test]
fn tile_rasterisation_qa_every_tile_square_is_one_sample() {
    let h = gpu();
    for g in raster_grids() {
        let set = unique_e0(g);
        let image = draw(&h, &set, &context(g), E0_VIEW);
        check_tiles(g, &written_e0(&set), &image);
    }
}

negative_control!(
    tile_rasterisation_qa_every_tile_square_is_one_sample,
    "a picture whose tile edges are off by one pixel puts two samples in one tile square",
    expected = "is not one sample",
    {
        let g = Grid::new([2, 3], 3, 2, 5).expect("a qa grid");
        let set = unique_e0(g);
        let image = draw(&gpu(), &set, &context(g), E0_VIEW);
        check_tiles(g, &written_e0(&set), &shifted(&image))
    }
);

/// The lanes' pointers agree with the values drawn: the pixel's `E_0` is the one stored at `ctx.tile.sample_index`,
/// its `quad_depth` the one written to quad `ctx.quad.index`; `tile_index` is below N², and the N² tiles of one quad
/// square (N·tile_px px, aligned) all have its quad index and distinct tile indices.
fn check_pointers(grid: Grid, set: &Synthetic, depths: &[u32], image: &Image) {
    let side = grid.n * grid.tile_px;
    let mut tiles: HashMap<(u32, u32), HashSet<u32>> = HashMap::new();
    let mut quads: HashMap<(u32, u32), u32> = HashMap::new();
    for y in 0..image.height {
        for x in 0..image.width {
            let [sample, e0, quad, packed] = image.words(x, y)[..] else {
                unreachable!()
            };
            assert!(
                sample < grid.sample_count(),
                "pixel ({x}, {y}): sample_index {sample} is past the buffer"
            );
            assert_eq!(
                set.simstate(sample).E_0.to_bits(),
                e0,
                "pixel ({x}, {y}): the sample drawn is not ctx.tile.sample_index's"
            );
            let (depth, tile) = (packed >> 16, packed & 0xffff);
            assert_eq!(
                depths[quad as usize], depth,
                "pixel ({x}, {y}): the RenderQuad drawn is not ctx.quad.index's"
            );
            assert!(
                tile < grid.n * grid.n,
                "pixel ({x}, {y}): tile_index {tile} ≥ N²"
            );
            let square = (x / side, y / side);
            let q = *quads.entry(square).or_insert(quad);
            assert_eq!(q, quad, "pixel ({x}, {y}): one quad square holds two quads");
            tiles.entry(square).or_default().insert(tile);
        }
    }
    for (square, t) in &tiles {
        assert_eq!(
            t.len() as u32,
            grid.n * grid.n,
            "quad square {square:?} does not hold N² distinct tiles"
        );
    }
    let distinct: HashSet<u32> = quads.values().copied().collect();
    assert_eq!(
        distinct.len() as u32,
        grid.quad_count(),
        "the quad squares are not distinct quads"
    );
}

const POINTER_VIEW: &str = "fn view(rc: RenderContext, l: Lanes) -> vec4<u32> {
    return vec4<u32>(l.tile.sample_index, bitcast<u32>(rc.sample.E_0), l.quad.index,
        (rc.quad.quad_depth << 16u) | l.tile.tile_index);
}";

fn depth_set(grid: Grid) -> (Synthetic, Vec<u32>) {
    let mut set = unique_e0(grid);
    let depths: Vec<u32> = (0..grid.quad_count()).map(|q| 100 + 7 * q).collect();
    for (q, d) in depths.iter().enumerate() {
        set.quad(q as u32)
            .u32("quad_depth", *d)
            .unwrap_or_else(|e| panic!("{e}"));
    }
    (set, depths)
}

#[test]
fn tile_rasterisation_qa_lanes_point_at_what_is_drawn() {
    let h = gpu();
    for g in raster_grids() {
        let (set, depths) = depth_set(g);
        let image = draw(&h, &set, &context(g), POINTER_VIEW);
        check_pointers(g, &set, &depths, &image);
    }
}

negative_control!(
    tile_rasterisation_qa_lanes_point_at_what_is_drawn,
    "the depths checked against the quads in reverse order fail",
    expected = "is not ctx.quad.index's",
    {
        let g = Grid::new([3, 2], 2, 1, 4).expect("a qa grid");
        let (set, mut depths) = depth_set(g);
        let image = draw(&gpu(), &set, &context(g), POINTER_VIEW);
        depths.reverse();
        check_pointers(g, &set, &depths, &image)
    }
);

// ── REQ-COL-056 and the chart/quad lanes' geometry, exact on a dyadic grid ─────────────────────────────────────

/// Four by two quads of 2 × 2 tiles of 4 px: a 32 × 16 target, every coordinate below dyadic, so exact in f32.
fn dyadic() -> Grid {
    Grid::new([4, 2], 2, 1, 4).expect("the dyadic grid")
}

const UV_VIEWS: [&str; 3] = [
    "fn view(rc: RenderContext, l: Lanes) -> vec4<u32> {
    return vec4<u32>(bitcast<vec2<u32>>(l.tile.uv), bitcast<vec2<u32>>(l.quad.uv));
}",
    "fn view(rc: RenderContext, l: Lanes) -> vec4<u32> {
    return vec4<u32>(bitcast<vec2<u32>>(l.chart.slice_uv), bitcast<vec2<u32>>(rc.uv));
}",
    "fn view(rc: RenderContext, l: Lanes) -> vec4<u32> {
    return vec4<u32>(bitcast<vec2<u32>>(l.quad.tl), bitcast<vec2<u32>>(l.quad.centre));
}",
];

fn f(w: u32) -> f32 {
    f32::from_bits(w)
}

/// colour_composition §3 (R-72): `ctx.tile.uv` is the pixel centre's place in its tile, origin at the tile's
/// bottom-left, `v` upward; `ctx.quad.uv = (vec2(i, j) + ctx.tile.uv) / N` for the tile in column `i`, row `j` of its
/// quad counted from the quad's bottom-left; `RenderContext.uv` is the within-quad uv; on a flat grid tiling the
/// slice, `ctx.chart.slice_uv` is the centre's place on the slice, Y-up; the quad's `tl` and `centre` are its top-left
/// corner (largest `v`) and centre in slice coords. All exact: every quantity is dyadic.
fn check_geometry(grid: Grid, images: &[Image; 3]) {
    let t = grid.tile_px as f32;
    let n = grid.n as f32;
    let (w, h) = grid.target();
    for y in 0..h {
        for x in 0..w {
            let up = [x as f32 + 0.5, (h - y) as f32 - 0.5];
            let cell = [(up[0] / t).floor(), (up[1] / t).floor()];
            let tile_uv = [(up[0] - cell[0] * t) / t, (up[1] - cell[1] * t) / t];
            let ij = [cell[0] % n, cell[1] % n];
            let quad_uv = [(ij[0] + tile_uv[0]) / n, (ij[1] + tile_uv[1]) / n];
            let qxy = [(cell[0] / n).floor(), (cell[1] / n).floor()];
            let quads = [grid.quads[0] as f32, grid.quads[1] as f32];
            let slice = [up[0] / w as f32, up[1] / h as f32];
            let tl = [qxy[0] / quads[0], (qxy[1] + 1.0) / quads[1]];
            let centre = [(qxy[0] + 0.5) / quads[0], (qxy[1] + 0.5) / quads[1]];
            let got: Vec<[f32; 2]> = images
                .iter()
                .flat_map(|im| {
                    let v = im.words(x, y);
                    [[f(v[0]), f(v[1])], [f(v[2]), f(v[3])]]
                })
                .collect();
            let want = [tile_uv, quad_uv, slice, quad_uv, tl, centre];
            let names = [
                "ctx.tile.uv",
                "ctx.quad.uv",
                "ctx.chart.slice_uv",
                "RenderContext.uv",
                "ctx.quad.tl",
                "ctx.quad.centre",
            ];
            for k in 0..6 {
                assert_eq!(
                    got[k], want[k],
                    "pixel ({x}, {y}): {} is {:?}, not {:?}",
                    names[k], got[k], want[k]
                );
                assert!(
                    got[k].iter().all(|c| (0.0..=1.0).contains(c)),
                    "pixel ({x}, {y}): {} leaves [0, 1]²",
                    names[k]
                );
            }
        }
    }
}

fn draw_geometry(h: &GpuHarness) -> [Image; 3] {
    let g = dyadic();
    let set = Synthetic::flat(g, 0);
    UV_VIEWS.map(|v| draw(h, &set, &context(g), v))
}

#[test]
fn ctx_lanes_qa_tile_and_quad_uv_are_y_up_and_exact() {
    check_geometry(dyadic(), &draw_geometry(&gpu()));
}

negative_control!(
    ctx_lanes_qa_tile_and_quad_uv_are_y_up_and_exact,
    "the picture with its rows reversed, a Y-down v, fails",
    expected = "ctx.tile.uv is",
    {
        let images = draw_geometry(&gpu());
        check_geometry(dyadic(), &images.map(|i| flipped(&i)))
    }
);

// ── REQ-RENDER-008 and REQ-COL-003: the declarations, read from naga's IR ───────────────────────────────────────

fn parse(wgsl: &str) -> naga::Module {
    naga::front::wgsl::parse_str(wgsl).unwrap_or_else(|e| panic!("{}", e.emit_to_string(wgsl)))
}

/// The WGSL spelling of `ty`: scalars, vectors, arrays and structs by name.
fn type_name(m: &naga::Module, ty: naga::Handle<naga::Type>) -> String {
    let t = &m.types[ty];
    let scalar = |s: naga::Scalar| match (s.kind, s.width) {
        (naga::ScalarKind::Float, 4) => "f32".to_owned(),
        (naga::ScalarKind::Uint, 4) => "u32".to_owned(),
        (naga::ScalarKind::Sint, 4) => "i32".to_owned(),
        (naga::ScalarKind::Bool, _) => "bool".to_owned(),
        other => format!("{other:?}"),
    };
    match &t.inner {
        naga::TypeInner::Scalar(s) => scalar(*s),
        naga::TypeInner::Vector { size, scalar: s } => {
            format!("vec{}<{}>", *size as u8, scalar(*s))
        }
        naga::TypeInner::Array { base, size, .. } => match size {
            naga::ArraySize::Constant(k) => format!("array<{}, {k}>", type_name(m, *base)),
            _ => format!("array<{}>", type_name(m, *base)),
        },
        naga::TypeInner::Struct { .. } => t.name.clone().unwrap_or_default(),
        other => format!("{other:?}"),
    }
}

/// The members of the struct `name` in `m`, each `(name, type)`.
fn members(m: &naga::Module, name: &str) -> Vec<(String, String)> {
    let (_, t) = m
        .types
        .iter()
        .find(|(_, t)| t.name.as_deref() == Some(name))
        .unwrap_or_else(|| panic!("the module declares no struct `{name}`"));
    match &t.inner {
        naga::TypeInner::Struct { members, .. } => members
            .iter()
            .map(|s| (s.name.clone().unwrap_or_default(), type_name(m, s.ty)))
            .collect(),
        _ => panic!("`{name}` is not a struct"),
    }
}

fn harness_module() -> String {
    bind::module(&view("vec4<u32>(0u)"), ViewOutput::Words).unwrap_or_else(|e| panic!("{e}"))
}

/// Render contract Part 1: `RenderContext`'s members, as naga's IR declares them, are exactly `{sample: SimState, ic:
/// ICDescriptor, quad: RenderQuad, uv, screen_uv, time}`; `uv` and `screen_uv` are 2-D coordinates and `time` a scalar.
fn check_render_context(got: &[(String, String)]) {
    let names: Vec<&str> = got.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(
        names,
        ["sample", "ic", "quad", "uv", "screen_uv", "time"],
        "RenderContext's members are not render contract Part 1's"
    );
    let ty: HashMap<&str, &str> = got.iter().map(|(n, t)| (n.as_str(), t.as_str())).collect();
    for (member, want) in [
        ("sample", "SimState"),
        ("ic", "ICDescriptor"),
        ("quad", "RenderQuad"),
        ("uv", "vec2<f32>"),
        ("screen_uv", "vec2<f32>"),
        ("time", "f32"),
    ] {
        assert_eq!(
            ty[member], want,
            "RenderContext.{member} is not Part 1's {want}"
        );
    }
}

fn render_context_members() -> Vec<(String, String)> {
    members(&parse(&harness_module()), "RenderContext")
}

#[test]
fn render_context_members_qa_declared_with_part_1_types() {
    check_render_context(&render_context_members());
}

negative_control!(
    render_context_members_qa_declared_with_part_1_types,
    "a RenderContext whose sample is the stored SimStateFTLE, not the read-side SimState, fails",
    expected = "RenderContext.sample is not Part 1's SimState",
    {
        let mut got = render_context_members();
        got[0].1 = "SimStateFTLE".into();
        check_render_context(&got)
    }
);

/// colour_composition §3's lanes and the fields each row names, transcribed by qa (`tile/sample` is `tile`), with the
/// types the row gives (`pixel` and `target_dims` ivec2, the uvs vec2); `None` where the row gives no type. The
/// payload row's `t_end` is stored as the exact macro-step `t_end_step` (render contract Part 1), and the validity
/// row's diffusion predicate is `diffusion`'s validity; each is checked under either name.
/// One §3 field: its name (alternatives split by `|`) and the type the row gives, if any.
type Field = (&'static str, Option<&'static str>);

const SECTION_3: [(&str, &[Field]); 6] = [
    (
        "screen",
        &[
            ("pixel", Some("vec2<i32>")),
            ("uv", Some("vec2<f32>")),
            ("target_dims", Some("vec2<i32>")),
        ],
    ),
    (
        "chart",
        &[
            ("slice_uv", Some("vec2<f32>")),
            ("z", None),
            ("chart_id", None),
        ],
    ),
    (
        "quad",
        &[
            ("index", None),
            ("depth", None),
            ("tl", Some("vec2<f32>")),
            ("centre", Some("vec2<f32>")),
            ("uv", Some("vec2<f32>")),
            ("state", None),
            ("impurity", None),
            ("spread", None),
            ("suspect_frac", None),
            ("priority", None),
            ("cache_age", None),
            ("sample_count", None),
        ],
    ),
    (
        "tile",
        &[
            ("tile_index", None),
            ("sample_index", None),
            ("N", None),
            ("E", None),
            ("uv", Some("vec2<f32>")),
        ],
    ),
    (
        "payload",
        &[
            ("state", None),
            ("ftle", None),
            ("energy_drift", None),
            ("Lz_drift", None),
            ("diffusion", None),
            ("d_min", None),
            ("word", None),
            ("t_end|t_end_step", None),
            ("m0", None),
            ("m1", None),
            ("m2", None),
        ],
    ),
    (
        "validity",
        &[
            ("ftle_valid", Some("bool")),
            ("diffusion_valid|diffusion_slope_valid", Some("bool")),
            ("sd_is_failed", Some("bool")),
            ("out_of_chart", Some("bool")),
            ("saturated", Some("bool")),
        ],
    ),
];

/// The 8-D latent `z` has eight components; every field of `SECTION_3` is declared in its lane, with its type; and
/// every payload-lane field `f` has a bool `f_valid` in the validity lane (§3: "paired with every field").
fn check_lanes(wgsl: &str) {
    let m = parse(wgsl);
    let lanes = members(&m, "Lanes");
    let names: Vec<&str> = lanes.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(
        names,
        SECTION_3.map(|(l, _)| l),
        "the ctx lanes are not colour_composition §3's"
    );
    let lane = |l: &str| {
        let (_, ty) = lanes.iter().find(|(n, _)| n == l).expect("the lane");
        members(&m, ty)
    };
    for (l, fields) in SECTION_3 {
        let declared = lane(l);
        for (names, ty) in fields {
            let found = names
                .split('|')
                .find_map(|n| declared.iter().find(|(d, _)| d == n))
                .unwrap_or_else(|| panic!("the {l} lane does not declare §3's `{names}`"));
            if let Some(ty) = ty {
                assert_eq!(&found.1, ty, "ctx.{l}.{} is not §3's {ty}", found.0);
            }
        }
    }
    let z = lane("chart")
        .into_iter()
        .find(|(n, _)| n == "z")
        .map(|(_, t)| t);
    assert_eq!(
        z.as_deref(),
        Some("array<f32, 8>"),
        "ctx.chart.z is not the full 8-D latent"
    );
    let validity = lane("validity");
    for (field, _) in lane("payload") {
        let want = format!("{field}_valid");
        assert!(
            validity.iter().any(|(n, t)| *n == want && t == "bool"),
            "payload field `{field}` has no bool validity `{want}`"
        );
    }
    let ic: Vec<String> = members(&m, "ICDescriptor")
        .into_iter()
        .map(|(n, _)| n)
        .filter(|n| !n.starts_with('_'))
        .collect();
    let payload: Vec<String> = lane("payload").into_iter().map(|(n, _)| n).collect();
    for field in ic {
        assert!(
            payload.contains(&field),
            "the payload lane does not hold the decoded-IC quantity `{field}`"
        );
    }
}

#[test]
fn ctx_lanes_qa_every_section_3_field_declared_and_paired() {
    check_lanes(&harness_module());
}

negative_control!(
    ctx_lanes_qa_every_section_3_field_declared_and_paired,
    "a validity lane without `energy_drift_valid` leaves a payload field unpaired",
    expected = "payload field `energy_drift` has no bool validity",
    check_lanes(
        &harness_module()
            .replace("    energy_drift_valid: bool,\n", "")
            .replace(
                "    l.validity.energy_drift_valid = ",
                "    let unpaired_energy_drift = "
            )
    )
);

// ── REQ-RENDER-008, REQ-COL-003, REQ-TOOL-014: the values reach the fragment, both surfaces ────────────────────

/// One grid for the value tests: four by two quads of 2 × 2 tiles of 2 px, one copy each: a 16 × 8 target, so the
/// screen coordinates are dyadic and exact in f32.
fn value_grid() -> Grid {
    Grid::new([4, 2], 2, 1, 2).expect("the value grid")
}

/// Sample `i`'s `ICDescriptor` member `k` (render contract Part 1's order).
fn ic_value(i: u32, k: u32) -> f32 {
    (i * 16 + k) as f32 + 0.5
}

/// Quad `q`'s member `k` of dd_generation_root §3.7a's table, transcribed by qa, as its bits.
const SECTION_3_7A: [(&str, &str); 10] = [
    ("quad_depth", "u32"),
    ("quad_state", "u32"),
    ("coherence_score", "f32"),
    ("outcome_impurity", "f32"),
    ("ensemble_spread", "f32"),
    ("suspect_fraction", "f32"),
    ("priority_score", "f32"),
    ("ancestor_gap", "u32"),
    ("cache_age", "u32"),
    ("dominant_outcome", "u32"),
];

fn quad_value(q: u32, k: usize) -> u32 {
    match SECTION_3_7A[k].1 {
        "f32" => (q as f32 * 0.25 + k as f32 * 8.0 + 0.125).to_bits(),
        _ => q * 1000 + k as u32,
    }
}

/// A bitwise-adversarial `packed_a`: every low-16 bit pattern's state, detail, saturated, dmin_pair and last_symbol
/// varied with the reserved bits 10–15 set, and a finite f16 `d_min` in the high half.
fn adversarial_packed_a(i: u32) -> u32 {
    let low = (i.wrapping_mul(0x9e37) ^ 0xfc00) & 0xffff;
    let d_min_f16 = 0x3800 + (i % 0x0400);
    (d_min_f16 << 16) | low
}

/// A set with every sample's descriptor adversarial, its times, substeps, closure step, drift maxima and word at the
/// edges of their ranges, every `ICDescriptor` member and every `RenderQuad` member its own value.
fn adversarial(grid: Grid) -> Synthetic {
    let mut set = Synthetic::flat(grid, 0);
    for i in 0..grid.sample_count() {
        set.sample(i)
            .packed_a(adversarial_packed_a(i))
            .times(65535 - i, i)
            .total_substeps(u32::MAX - i)
            .closure_step(0xffff - i as u16)
            .drift_max(0.5 * (i % 100) as f32, 2f32.powi(-((i % 10) as i32)))
            .word([i * 3 + 1, 0, 0, 0], 2 + i % 5)
            .E_0(-(i as f32) - 0.25);
        let ic = set.ic(i);
        let m = [
            &mut ic.m0,
            &mut ic.m1,
            &mut ic.m2,
            &mut ic.q_mass,
            &mut ic.rho_mag,
            &mut ic.lambda_mag,
            &mut ic.rho_ratio,
            &mut ic.rho_angle,
            &mut ic.K_0,
            &mut ic.V_0,
            &mut ic.virial_ratio,
            &mut ic.r_min_pair_0,
        ];
        for (k, v) in m.into_iter().enumerate() {
            *v = ic_value(i, k as u32);
        }
    }
    for q in 0..grid.quad_count() {
        for (k, (name, ty)) in SECTION_3_7A.iter().enumerate() {
            let mut quad = set.quad(q);
            let bits = quad_value(q, k);
            match *ty {
                "f32" => quad.f32(name, f32::from_bits(bits)),
                _ => quad.u32(name, bits),
            }
            .map(|_| ())
            .unwrap_or_else(|e| panic!("{e}"));
        }
    }
    set
}

/// The views of the value tests, each four words, and what each word is.
fn value_views() -> Vec<(String, [&'static str; 4])> {
    let mut v = vec![
        (
            view("vec4<u32>(rc.sample.state, rc.sample.detail, select(0u, 1u, rc.sample.saturated), rc.sample.dmin_pair)"),
            ["state", "detail", "saturated", "dmin_pair"],
        ),
        (
            view("vec4<u32>(rc.sample.last_symbol, bitcast<u32>(rc.sample.d_min), rc.sample.t_end_step, rc.sample.t_dmin_step)"),
            ["last_symbol", "d_min", "t_end_step", "t_dmin_step"],
        ),
        (
            view("vec4<u32>(rc.sample.total_substeps, rc.sample.closure_step, bitcast<u32>(rc.sample.dE_max), bitcast<u32>(rc.sample.dLz_max))"),
            ["total_substeps", "closure_step", "dE_max", "dLz_max"],
        ),
        (
            view("vec4<u32>(rc.sample.word.x, l.payload.word.x, bitcast<u32>(rc.sample.E_0), bitcast<u32>(rc.time))"),
            ["word.x", "payload.word.x", "E_0", "time"],
        ),
        (
            view("vec4<u32>(select(0u, 1u, l.validity.sd_is_failed), select(0u, 1u, l.validity.saturated), select(0u, 1u, l.validity.out_of_chart), l.tile.N | (l.tile.E << 8u))"),
            ["sd_is_failed", "validity.saturated", "out_of_chart", "N|E"],
        ),
        (
            view("vec4<u32>(l.chart.chart_id, bitcast<u32>(l.chart.z[0]), bitcast<u32>(l.chart.z[7]), l.quad.sample_count)"),
            ["chart_id", "z[0]", "z[7]", "sample_count"],
        ),
        (
            view("vec4<u32>(bitcast<vec2<u32>>(rc.screen_uv), bitcast<vec2<u32>>(vec2<f32>(l.screen.target_dims)))"),
            ["screen_uv.x", "screen_uv.y", "target_dims.x", "target_dims.y"],
        ),
        (
            view("vec4<u32>(l.quad.depth, l.quad.state, bitcast<u32>(l.quad.impurity), bitcast<u32>(l.quad.spread))"),
            ["quad.depth", "quad.state", "quad.impurity", "quad.spread"],
        ),
        (
            view("vec4<u32>(bitcast<u32>(l.quad.suspect_frac), bitcast<u32>(l.quad.priority), l.quad.cache_age, bitcast<vec2<u32>>(vec2<f32>(l.screen.pixel)).x)"),
            ["quad.suspect_frac", "quad.priority", "quad.cache_age", "pixel.x"],
        ),
    ];
    for chunk in SECTION_3_7A.chunks(4) {
        let mut words = Vec::new();
        let mut names = ["", "", "", ""];
        for (k, (name, ty)) in chunk.iter().enumerate() {
            words.push(match *ty {
                "f32" => format!("bitcast<u32>(rc.quad.{name})"),
                _ => format!("rc.quad.{name}"),
            });
            names[k] = name;
        }
        while words.len() < 4 {
            words.push("0u".into());
        }
        v.push((view(&format!("vec4<u32>({})", words.join(", "))), names));
    }
    let ic = [
        "m0",
        "m1",
        "m2",
        "q_mass",
        "rho_mag",
        "lambda_mag",
        "rho_ratio",
        "rho_angle",
        "K_0",
        "V_0",
        "virial_ratio",
        "r_min_pair_0",
    ];
    for (c, chunk) in ic.chunks(4).enumerate() {
        let words: Vec<String> = chunk
            .iter()
            .map(|m| format!("bitcast<u32>(rc.ic.{m})"))
            .collect();
        let names = [
            ["ic0", "ic1", "ic2", "ic3"],
            ["ic4", "ic5", "ic6", "ic7"],
            ["ic8", "ic9", "ic10", "ic11"],
        ][c];
        v.push((view(&format!("vec4<u32>({})", words.join(", "))), names));
    }
    v.push((
        view("vec4<u32>(bitcast<u32>(l.payload.m0), bitcast<u32>(l.payload.m1), bitcast<u32>(l.payload.m2), bitcast<u32>(l.payload.r_min_pair_0))"),
        ["ic0", "ic1", "ic2", "ic11"],
    ));
    v
}

/// What the word `what` must be at a pixel whose sample is `i` (copy 0 of its tile, by `ctx.tile.sample_index`) and
/// quad `q`, at `(x, y)` of a `w × h` target: the hand-chosen value, and, for a packed field, the Rust accessor's
/// reading of the stored word too (both surfaces).
fn expected(set: &Synthetic, ctx: &Context, i: u32, q: u32, x: u32, what: &str) -> u32 {
    let s = set.simstate(i);
    let a = adversarial_packed_a(i);
    let both = |accessor: u32, chosen: u32| {
        assert_eq!(
            accessor, chosen,
            "sample {i}: the Rust accessor does not return the hand-chosen `{what}`"
        );
        chosen
    };
    let (w, _) = ctx.grid.target();
    match what {
        "state" => both(sd_state(s.packed_a), a & 0x7),
        "detail" => both(sd_detail(s.packed_a), (a >> 3) & 0x3),
        "saturated" | "validity.saturated" => {
            both(u32::from(sd_saturated(s.packed_a)), (a >> 5) & 1)
        }
        "dmin_pair" => both(sd_dmin_pair(s.packed_a), (a >> 6) & 0x3),
        "last_symbol" => both(sd_last_symbol(s.packed_a), (a >> 8) & 0x3),
        "d_min" => pa_d_min(s.packed_a).to_bits(),
        "t_end_step" => both(tm_t_end_step(s.times), 65535 - i),
        "t_dmin_step" => both(tm_t_dmin_step(s.times), i),
        "total_substeps" => both(s.total_substeps, u32::MAX - i),
        "closure_step" => both(u32::from(s.closure_step), 0xffff - i),
        "dE_max" => both(
            pb_dE_max(s.packed_b).to_bits(),
            (0.5 * (i % 100) as f32).to_bits(),
        ),
        "dLz_max" => both(
            pb_dLz_max(s.packed_b).to_bits(),
            2f32.powi(-((i % 10) as i32)).to_bits(),
        ),
        "word.x" | "payload.word.x" => {
            both(set.word(i)[0], fgw_pack([i * 3 + 1, 0, 0, 0], 2 + i % 5)[0])
        }
        "E_0" => (-(i as f32) - 0.25).to_bits(),
        "time" => ctx.time.to_bits(),
        "sd_is_failed" => u32::from(a & 0x7 >= 4),
        "out_of_chart" => u32::from(ctx.out_of_chart),
        "N|E" => ctx.grid.n | (ctx.grid.e << 8),
        "chart_id" => ctx.chart_id,
        "z[0]" => ctx.z[0].to_bits(),
        "z[7]" => ctx.z[7].to_bits(),
        "sample_count" => ctx.valid_sample_count,
        "screen_uv.x" => ((x as f32 + 0.5) / w as f32).to_bits(),
        "screen_uv.y" => u32::MAX, // either orientation; checked in `check_values`
        "target_dims.x" => (w as f32).to_bits(),
        "target_dims.y" => (ctx.grid.target().1 as f32).to_bits(),
        "pixel.x" => (x as f32).to_bits(),
        "quad.depth" => quad_value(q, 0),
        "quad.state" => quad_value(q, 1),
        "quad.impurity" => quad_value(q, 3),
        "quad.spread" => quad_value(q, 4),
        "quad.suspect_frac" => quad_value(q, 5),
        "quad.priority" => quad_value(q, 6),
        "quad.cache_age" => quad_value(q, 8),
        "" => 0,
        ic if ic.starts_with("ic") => {
            let k: u32 = ic[2..].parse().expect("an ic index");
            ic_value(i, k).to_bits()
        }
        quad => {
            let k = SECTION_3_7A
                .iter()
                .position(|(n, _)| *n == quad)
                .unwrap_or_else(|| panic!("no expectation for `{quad}`"));
            quad_value(q, k)
        }
    }
}

/// Each pixel of each view shows its sample's and quad's hand-chosen values, read through `RenderContext` and the
/// lanes, with the sample and quad the lanes name (`ctx.tile.sample_index`, `ctx.quad.index`, `pointers`).
/// `screen_uv.y` is `(y + 0.5) / h` from the top or the bottom, one orientation over the whole image (the corpus
/// names none for the screen).
fn check_values(set: &Synthetic, ctx: &Context, pointers: &Image, images: &[(Image, [&str; 4])]) {
    let (w, h) = ctx.grid.target();
    let mut orientation = None;
    for y in 0..h {
        for x in 0..w {
            let p = pointers.words(x, y);
            let (i, q) = (p[0], p[1]);
            for (image, names) in images {
                let got = image.words(x, y);
                for (k, what) in names.iter().enumerate() {
                    if *what == "screen_uv.y" {
                        let down = ((y as f32 + 0.5) / h as f32).to_bits();
                        let up = ((h - y) as f32 - 0.5) / h as f32;
                        let o = if got[k] == down {
                            false
                        } else if got[k] == up.to_bits() {
                            true
                        } else {
                            panic!(
                                "pixel ({x}, {y}): screen_uv.y {} is neither orientation",
                                f(got[k])
                            )
                        };
                        assert_eq!(
                            *orientation.get_or_insert(o),
                            o,
                            "pixel ({x}, {y}): screen_uv changes orientation"
                        );
                        continue;
                    }
                    let want = expected(set, ctx, i, q, x, what);
                    assert_eq!(
                        got[k], want,
                        "pixel ({x}, {y}), sample {i}, quad {q}: `{what}` rendered {:#x}, the CPU wrote {want:#x}",
                        got[k]
                    );
                }
            }
        }
    }
}

const POINTERS: &str = "fn view(rc: RenderContext, l: Lanes) -> vec4<u32> {
    return vec4<u32>(l.tile.sample_index, l.quad.index, 0u, 0u);
}";

fn draw_values(
    h: &GpuHarness,
    set: &Synthetic,
    ctx: &Context,
) -> (Image, Vec<(Image, [&'static str; 4])>) {
    let pointers = draw(h, set, ctx, POINTERS);
    let images = value_views()
        .into_iter()
        .map(|(v, names)| (draw(h, set, ctx, &v), names))
        .collect();
    (pointers, images)
}

#[test]
fn synthetic_upload_renders_qa_adversarial_set_on_both_surfaces() {
    let h = gpu();
    let g = value_grid();
    let set = adversarial(g);
    for (time, out_of_chart) in [(1234.5, false), (0.0, true)] {
        let ctx = Context {
            time,
            out_of_chart,
            ..context(g)
        };
        let (pointers, images) = draw_values(&h, &set, &ctx);
        check_values(&set, &ctx, &pointers, &images);
    }
}

negative_control!(
    synthetic_upload_renders_qa_adversarial_set_on_both_surfaces,
    "the picture drawn at time 1234.5 checked as if the playhead were 0 fails on RenderContext.time",
    expected = "`time` rendered",
    {
        let h = gpu();
        let g = value_grid();
        let set = adversarial(g);
        let ctx = context(g);
        let (pointers, images) = draw_values(&h, &set, &ctx);
        check_values(&set, &Context { time: 0.0, ..ctx }, &pointers, &images)
    }
);

/// The accessor surface alone: for every sample, the generated Rust accessors read back each hand-chosen packed value
/// from the stored words (debug tooling plan, "Synthetic-first": "asserting both the accessor returns it and the
/// shader renders it").
const PACKED: [&str; 11] = [
    "state",
    "detail",
    "saturated",
    "dmin_pair",
    "last_symbol",
    "t_end_step",
    "t_dmin_step",
    "total_substeps",
    "closure_step",
    "dE_max",
    "word.x",
];

fn check_accessors(set: &Synthetic) {
    let ctx = context(set.grid());
    for i in 0..set.grid().sample_count() {
        for what in PACKED {
            expected(set, &ctx, i, 0, 0, what);
        }
    }
}

#[test]
fn synthetic_upload_qa_rust_accessors_return_the_chosen_values() {
    check_accessors(&adversarial(value_grid()));
}

negative_control!(
    synthetic_upload_qa_rust_accessors_return_the_chosen_values,
    "a set whose descriptors are stored one bit off disagrees with the hand-chosen values",
    expected = "the Rust accessor does not return the hand-chosen",
    {
        let g = value_grid();
        let mut set = adversarial(g);
        for i in 0..g.sample_count() {
            set.sample(i).packed_a(adversarial_packed_a(i) >> 1);
        }
        check_accessors(&set)
    }
);

// ── REQ-RENDER-007: accessors over the physical structs, never an unpacked copy ────────────────────────────────

/// In `wgsl`, every entry point is a fragment entry: no compute pass.
fn check_no_compute(wgsl: &str) {
    let m = parse(wgsl);
    for e in &m.entry_points {
        assert_eq!(
            e.stage,
            naga::ShaderStage::Fragment,
            "entry `{}` is not a fragment entry: a compute pass in the fragment module",
            e.name
        );
    }
}

/// In `wgsl`, no storage buffer is writable.
fn check_read_only(wgsl: &str) {
    let m = parse(wgsl);
    for (_, g) in m.global_variables.iter() {
        if let naga::AddressSpace::Storage { access } = g.space {
            assert!(
                !access.contains(naga::StorageAccess::STORE),
                "storage buffer `{}` is writable: the fragment may write a payload copy",
                g.name.clone().unwrap_or_default()
            );
        }
    }
}

/// In `wgsl`, no global holds the logical, unpacked structs (`SimState`, `RenderContext`, a lane): the buffers hold
/// only the physical ones.
fn check_no_logical_buffer(wgsl: &str) {
    let m = parse(wgsl);
    let logical = [
        "SimState",
        "RenderContext",
        "Lanes",
        "PayloadLane",
        "ValidityLane",
    ];
    for (_, g) in m.global_variables.iter() {
        let ty = type_name(&m, g.ty);
        assert!(
            !logical
                .iter()
                .any(|l| ty == *l || ty.starts_with(&format!("array<{l}>"))),
            "global `{}` holds the unpacked logical `{ty}`",
            g.name.clone().unwrap_or_default()
        );
    }
}

/// The harness module and an assembled stain over the harness's buffers.
fn modules() -> Vec<String> {
    use render::assemble::{assemble, Kind, Node, Occupant, Stain, Tier};
    let node = |kind, occupant, inputs: &[Option<usize>]| Node {
        kind,
        occupant,
        inputs: inputs.to_vec(),
    };
    let stain = Stain::new(vec![
        node(Kind::Source, Occupant::Field("d_min".into()), &[]),
        node(
            Kind::Colour,
            Occupant::Custom(
                "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.inputs[0].x, 0.0, 0.0); }"
                    .into(),
            ),
            &[Some(0)],
        ),
        node(
            Kind::Combiner,
            Occupant::BuiltIn("pass_through".into()),
            &[Some(1), None],
        ),
        node(Kind::Out, Occupant::None, &[Some(2)]),
    ])
    .unwrap_or_else(|e| panic!("{e}"));
    let fragment = assemble(&stain, Tier::FULL).unwrap_or_else(|e| panic!("{e}"));
    vec![harness_module(), bind::stain_module(&fragment.source)]
}

#[test]
fn render_sample_qa_no_compute_pass() {
    for m in modules() {
        check_no_compute(&m);
    }
}

negative_control!(
    render_sample_qa_no_compute_pass,
    "a module with a compute entry point fails",
    expected = "is not a fragment entry",
    check_no_compute(&format!(
        "{}\n@compute @workgroup_size(1)\nfn unpack_pass() {{}}\n",
        harness_module()
    ))
);

#[test]
fn render_sample_qa_no_writable_buffer() {
    for m in modules() {
        check_read_only(&m);
    }
}

negative_control!(
    render_sample_qa_no_writable_buffer,
    "a module binding a read_write buffer fails",
    expected = "is writable",
    check_read_only(&format!(
        "{}\n@group(3) @binding(0) var<storage, read_write> unpacked: array<u32>;\n",
        harness_module()
    ))
);

#[test]
fn render_sample_qa_no_buffer_of_the_logical_struct() {
    for m in modules() {
        check_no_logical_buffer(&m);
    }
}

negative_control!(
    render_sample_qa_no_buffer_of_the_logical_struct,
    "a module binding a buffer of the logical SimState fails",
    expected = "holds the unpacked logical `array<SimState>`",
    check_no_logical_buffer(&format!(
        "{}\n@group(3) @binding(0) var<storage, read> unpacked: array<SimState>;\n",
        harness_module()
    ))
);
