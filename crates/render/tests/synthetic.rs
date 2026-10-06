//! The synthetic payload harness (TASK-M1-06; debug tooling plan step 0b): CPU-filled `SimState`, word,
//! `ICDescriptor` and `RenderQuad` buffers, bound to the fragment and rendered headless.
//! - REQ-RENDER-004: each sample rasterises to exactly its tile, and a tile's pixels all come from its one sample
//!   (`tile_rasterisation_*`);
//! - REQ-RENDER-008: the generated `RenderContext` declares `sample, ic, quad, uv, screen_uv, time`, and a debug view
//!   reading each compiles (`render_context_members_*`);
//! - REQ-COL-003, REQ-COL-056: the generated `ctx` declares every lane of colour_composition §3, `ctx.tile.uv`
//!   included, and each payload field has a validity accessor (`ctx_lanes_*`);
//! - REQ-TOOL-014: a CPU-filled SimState/ICDescriptor/RenderQuad set uploads and renders a debug view, through the
//!   generated context and through an assembled stain (`synthetic_upload_renders_*`).
//!
//! Each test registers its negative control (R-176).

use render::assemble::{assemble, Kind, Node, Occupant, Stain, Tier};
use render::bind::{self, Context, ViewOutput};
use render::headless::{self, Draw, Image, Target};
use render::raster::{Cell, Grid};
use validation::gpu::GpuHarness;
use validation::negative_control;
use validation::synthetic::Synthetic;

/// Three by two quads of 4 × 4 tiles, 3 px square, each sample with one ensemble copy: a 36 × 24 target.
fn grid() -> Grid {
    Grid::new([3, 2], 4, 1, 3).expect("the test grid")
}

fn context(grid: Grid) -> Context {
    Context {
        dt_macro: 0.01,
        delta_0: 1e-6,
        n_renorm: 16,
        horizon_steps: 1000,
        z: [0.5, -0.25, 0.125, 1.0, 2.0, -3.0, 0.75, 0.0625],
        grid,
        chart_id: 7,
        time: 2.5,
        ensemble_spread: 0.25,
        out_of_chart: false,
        valid_sample_count: 13,
    }
}

fn gpu() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

/// `module`'s entry `entry` drawn over `set`'s buffers and `ctx`'s uniforms into a target of `format`.
fn draw(
    h: &GpuHarness,
    set: &Synthetic,
    ctx: &Context,
    module: &str,
    entry: &str,
    format: wgpu::TextureFormat,
) -> Image {
    let bytes = set.bytes();
    let bound = bind::upload(h.device(), &bytes.payload(), ctx);
    let (width, height) = ctx.grid.target();
    headless::render(
        h.device(),
        h.queue(),
        &Draw {
            module,
            entry,
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

/// `view` (a `fn view(rc: RenderContext, l: Lanes) -> vec4<u32>`) drawn over `set` into an `Rgba32Uint` target.
fn draw_words(h: &GpuHarness, set: &Synthetic, ctx: &Context, view: &str) -> Image {
    let module = bind::module(view, ViewOutput::Words).unwrap_or_else(|e| panic!("{e}"));
    draw(
        h,
        set,
        ctx,
        &module,
        bind::ENTRY,
        wgpu::TextureFormat::Rgba32Uint,
    )
}

/// `image` with its rows in reverse order: the picture a wrong Y convention draws.
fn flipped(image: &Image) -> Image {
    let row = image.width as usize * image.texel_size();
    let bytes = image.bytes.chunks(row).rev().flatten().copied().collect();
    Image {
        bytes,
        ..image.clone()
    }
}

// ── REQ-RENDER-004: the sample → tile rasterisation ─────────────────────────────────────────────────────────────

/// The raster view: each pixel's base sample, quad, tile and the quad's depth.
const RASTER_VIEW: &str = "fn view(rc: RenderContext, l: Lanes) -> vec4<u32> {
    return vec4<u32>(l.tile.sample_index, l.quad.index, l.tile.tile_index, rc.quad.quad_depth);
}";

/// Every pixel of `image` reads the sample, quad and tile `grid` puts it in, and every tile's pixels, its whole
/// `tile_px` square, read its one sample and no other pixel does.
fn check_raster(grid: Grid, image: &Image, depth: u32) {
    let (width, height) = grid.target();
    for y in 0..height {
        for x in 0..width {
            let c = grid.cell(x, y);
            assert_eq!(
                image.words(x, y),
                [c.sample, c.quad, c.tile, depth],
                "pixel ({x}, {y}) does not read its tile's sample"
            );
        }
    }
    let t = grid.tile_px;
    for q in 0..grid.quad_count() {
        let quad_xy = [q % grid.quads[0], q / grid.quads[0]];
        for tile in 0..grid.n * grid.n {
            let tile_xy = [tile % grid.n, tile / grid.n];
            let sample = grid.sample_index(q, tile, 0);
            let (x0, y0, x1, y1) = grid.tile_pixels(quad_xy, tile_xy);
            assert_eq!(
                (x1 - x0, y1 - y0),
                (t, t),
                "tile {tile} of quad {q} is not {t} px square"
            );
            let mut covered = 0;
            for y in 0..height {
                for x in 0..width {
                    let inside = (x0..x1).contains(&x) && (y0..y1).contains(&y);
                    let reads = image.words(x, y)[0] == sample;
                    assert_eq!(
                        inside, reads,
                        "pixel ({x}, {y}) does not read its tile's sample: tile {tile} of quad {q} is {:?}",
                        (x0, y0, x1, y1)
                    );
                    covered += u32::from(reads);
                }
            }
            assert_eq!(covered, t * t, "sample {sample} covers {covered} pixels");
        }
    }
}

#[test]
fn tile_rasterisation_each_sample_covers_exactly_its_tile() {
    let g = grid();
    let set = Synthetic::flat(g, 2);
    let image = draw_words(&gpu(), &set, &context(g), RASTER_VIEW);
    check_raster(g, &image, 2);
}

negative_control!(
    tile_rasterisation_each_sample_covers_exactly_its_tile,
    "the picture with its rows reversed, a wrong Y convention, puts samples outside their tiles",
    expected = "does not read its tile's sample",
    {
        let g = grid();
        let set = Synthetic::flat(g, 2);
        let image = draw_words(&gpu(), &set, &context(g), RASTER_VIEW);
        check_raster(g, &flipped(&image), 2)
    }
);

/// `cell`, a rasterisation of `grid`'s target, partitions it: each pixel is in exactly one tile, the one whose pixel
/// square holds it, and rows count from the bottom.
fn check_partition(grid: Grid, cell: impl Fn(u32, u32) -> Cell) {
    let (width, height) = grid.target();
    for y in 0..height {
        for x in 0..width {
            let c = cell(x, y);
            let (x0, y0, x1, y1) = grid.tile_pixels(c.quad_xy, c.tile_xy);
            assert!(
                (x0..x1).contains(&x) && (y0..y1).contains(&y),
                "pixel ({x}, {y}) is outside the tile it rasterises to"
            );
        }
    }
    let bottom_left = cell(0, height - 1);
    assert_eq!(
        (bottom_left.quad, bottom_left.tile),
        (0, 0),
        "the bottom-left pixel is not in quad 0's tile 0"
    );
}

#[test]
fn tile_rasterisation_cpu_cells_partition_the_target() {
    for g in [grid(), Grid::new([1, 1], 1, 0, 1).expect("one pixel")] {
        check_partition(g, |x, y| g.cell(x, y));
    }
    assert!(
        Grid::new([0, 1], 4, 0, 3).is_err(),
        "an empty grid was accepted"
    );
}

negative_control!(
    tile_rasterisation_cpu_cells_partition_the_target,
    "a rasterisation counting rows from the top puts pixels outside their tiles",
    expected = "is outside the tile it rasterises to",
    {
        let g = grid();
        let (_, height) = g.target();
        check_partition(g, |x, y| g.cell(x, height - 1 - y))
    }
);

/// `grid`'s sample index is the flat grid's `(quad · N² + tile) · (E + 1) + copy` (render::raster's note, applied
/// per R-369): every `(quad, tile, copy)` has its own index, the indices fill `0..sample_count`, and a tile's E + 1
/// copies sit together after its base sample.
fn check_sample_index(grid: Grid, index: impl Fn(u32, u32, u32) -> u32) {
    let mut seen = vec![false; grid.sample_count() as usize];
    for quad in 0..grid.quad_count() {
        for tile in 0..grid.n * grid.n {
            for copy in 0..=grid.e {
                let i = index(quad, tile, copy);
                assert_eq!(
                    i,
                    (quad * grid.n * grid.n + tile) * (grid.e + 1) + copy,
                    "copy {copy} of quad {quad}'s tile {tile} is not at the flat grid's index"
                );
                assert!(
                    !std::mem::replace(&mut seen[i as usize], true),
                    "sample index {i} is given twice"
                );
            }
        }
    }
}

#[test]
fn tile_rasterisation_sample_index_is_the_flat_grids() {
    // Two quads of 2 × 2 tiles, each sample with two copies: the last copy of the last tile is index 23, the last
    // of the 2 · 4 · 3 = 24.
    let g = Grid::new([2, 1], 2, 2, 1).expect("the index grid");
    assert_eq!(g.sample_count(), 24, "the grid's sample count");
    assert_eq!(g.sample_index(1, 3, 2), 23, "the last sample's index");
    assert_eq!(g.sample_index(0, 1, 1), 4, "quad 0's tile 1's copy 1");
    check_sample_index(g, |q, t, c| g.sample_index(q, t, c));
}

negative_control!(
    tile_rasterisation_sample_index_is_the_flat_grids,
    "an index that ignores the copy gives each tile's copies one index",
    expected = "is not at the flat grid's index",
    {
        let g = Grid::new([2, 1], 2, 2, 1).expect("the index grid");
        check_sample_index(g, |q, t, _| g.sample_index(q, t, 0))
    }
);

/// Each sample's `E_0` set to its own value, distinct per sample.
fn distinct(grid: Grid) -> Synthetic {
    let mut set = Synthetic::flat(grid, 1);
    for i in 0..grid.sample_count() {
        set.sample(i).E_0(i as f32 + 0.5);
    }
    set
}

/// Every pixel of `image` holds, in its first word, the `E_0` bits of its tile's base sample in `set`: no pixel blends
/// two samples.
fn check_one_sample(grid: Grid, set: &Synthetic, image: &Image) {
    let (width, height) = grid.target();
    for y in 0..height {
        for x in 0..width {
            let want = set.simstate(grid.cell(x, y).sample).E_0.to_bits();
            assert_eq!(
                image.words(x, y)[0],
                want,
                "pixel ({x}, {y})'s value is not its tile's one sample's"
            );
        }
    }
}

const E0_VIEW: &str = "fn view(rc: RenderContext, l: Lanes) -> vec4<u32> {
    return vec4<u32>(bitcast<u32>(rc.sample.E_0), 0u, 0u, 0u);
}";

#[test]
fn tile_rasterisation_tile_pixels_come_from_one_sample() {
    let g = grid();
    let set = distinct(g);
    let image = draw_words(&gpu(), &set, &context(g), E0_VIEW);
    check_one_sample(g, &set, &image);
}

negative_control!(
    tile_rasterisation_tile_pixels_come_from_one_sample,
    "a pixel reading its neighbour tile's sample fails the check",
    expected = "is not its tile's one sample's",
    {
        let g = grid();
        let set = distinct(g);
        let mut image = draw_words(&gpu(), &set, &context(g), E0_VIEW);
        // Pixel (2, 23), the bottom-left tile's right column, given the next tile's value.
        let size = image.texel_size();
        let at = (23 * image.width as usize + 2) * size;
        let next = set.simstate(g.cell(3, 23).sample).E_0.to_bits();
        image.bytes[at..at + 4].copy_from_slice(&next.to_le_bytes());
        check_one_sample(g, &set, &image)
    }
);

// ── REQ-RENDER-008: RenderContext ─────────────────────────────────────────────────────────────────────────────────

/// The members of the WGSL struct `name` in `wgsl`, each `(name, type)`.
fn struct_members(wgsl: &str, name: &str) -> Vec<(String, String)> {
    wgsl.lines()
        .skip_while(|l| l.trim() != format!("struct {name} {{"))
        .skip(1)
        .take_while(|l| l.trim() != "}")
        .filter_map(|l| {
            let (n, t) = l.trim().trim_end_matches(',').split_once(": ")?;
            Some((n.to_owned(), t.to_owned()))
        })
        .collect()
}

/// Render contract Part 1's `RenderContext`.
const RENDER_CONTEXT: [(&str, &str); 6] = [
    ("sample", "SimState"),
    ("ic", "ICDescriptor"),
    ("quad", "RenderQuad"),
    ("uv", "vec2<f32>"),
    ("screen_uv", "vec2<f32>"),
    ("time", "f32"),
];

fn check_render_context(wgsl: &str) {
    let want: Vec<(String, String)> = RENDER_CONTEXT
        .iter()
        .map(|(n, t)| (n.to_string(), t.to_string()))
        .collect();
    assert_eq!(
        struct_members(wgsl, "RenderContext"),
        want,
        "the generated RenderContext is not render contract Part 1's"
    );
}

#[test]
fn render_context_members_are_render_contract_part_1() {
    check_render_context(&bind::context().unwrap_or_else(|e| panic!("{e}")));
}

negative_control!(
    render_context_members_are_render_contract_part_1,
    "a RenderContext without `time` is not Part 1's",
    expected = "the generated RenderContext is not render contract Part 1's",
    check_render_context(
        &bind::context()
            .unwrap_or_else(|e| panic!("{e}"))
            .replace("    time: f32,\n}", "}")
    )
);

/// A debug view reading `expr`, an f32.
fn reading(expr: &str) -> String {
    format!("fn view(rc: RenderContext, l: Lanes) -> vec4<f32> {{ return vec4<f32>({expr}, 0.0, 0.0, 1.0); }}")
}

/// The views reading each member of `RenderContext`.
fn member_views() -> Vec<(&'static str, String)> {
    vec![
        ("sample", reading("rc.sample.E_0")),
        ("ic", reading("rc.ic.K_0")),
        ("quad", reading("f32(rc.quad.quad_depth)")),
        ("uv", reading("rc.uv.x")),
        ("screen_uv", reading("rc.screen_uv.y")),
        ("time", reading("rc.time")),
    ]
}

/// Each view, the harness's module around it, parses and validates (naga, wgpu 30's front end).
fn check_views_compile(views: &[(&str, String)]) {
    for (member, view) in views {
        let module = bind::module(view, ViewOutput::Colour).unwrap_or_else(|e| panic!("{e}"));
        let parsed = naga::front::wgsl::parse_str(&module).unwrap_or_else(|e| {
            panic!(
                "a debug view reading `{member}` does not compile: {}",
                e.emit_to_string(&module)
            )
        });
        let mut capabilities = naga::valid::Capabilities::default();
        capabilities.insert(naga::valid::Capabilities::SHADER_FLOAT16_IN_FLOAT32);
        naga::valid::Validator::new(naga::valid::ValidationFlags::all(), capabilities)
            .validate(&parsed)
            .unwrap_or_else(|e| panic!("a debug view reading `{member}` does not compile: {e:?}"));
    }
}

#[test]
fn render_context_members_debug_view_reading_each_compiles() {
    check_views_compile(&member_views());
}

negative_control!(
    render_context_members_debug_view_reading_each_compiles,
    "a view reading `rc.playhead`, no member, does not compile",
    expected = "a debug view reading `playhead` does not compile",
    check_views_compile(&[("playhead", reading("rc.playhead"))])
);

/// The u32 module constant `name` in `m`.
fn wgsl_const(m: &naga::Module, name: &str) -> Option<u32> {
    let (_, c) = m
        .constants
        .iter()
        .find(|(_, c)| c.name.as_deref() == Some(name))?;
    match m.global_expressions[c.init] {
        naga::Expression::Literal(naga::Literal::U32(v)) => Some(v),
        _ => None,
    }
}

/// In `wgsl`, each of the four group-1 buffers is bound at its row of the ledger's one binding table
/// (`ledger::payload::bindings`, R-343), equal to the generated constants `<PREFIX>_GROUP` and `<PREFIX>_BINDING`.
fn check_buffer_bindings(wgsl: &str) {
    let m =
        naga::front::wgsl::parse_str(wgsl).unwrap_or_else(|e| panic!("{}", e.emit_to_string(wgsl)));
    let table = ledger::payload::bindings();
    assert_eq!(
        bind::buffers().map(|b| (b.buffer, b.group, b.binding)),
        table.map(|b| (b.buffer, b.group, b.binding)),
        "render::bind's buffers are not the ledger's table"
    );
    for b in table {
        let (_, g) = m
            .global_variables
            .iter()
            .find(|(_, g)| g.name.as_deref() == Some(b.buffer))
            .unwrap_or_else(|| panic!("the module binds no `{}`", b.buffer));
        let at = g.binding.as_ref().map(|r| (r.group, r.binding));
        assert_eq!(
            at,
            Some((b.group, b.binding)),
            "`{}` is not bound at the ledger table's numbers",
            b.buffer
        );
        for (suffix, want) in [("GROUP", b.group), ("BINDING", b.binding)] {
            let name = format!("{}_{suffix}", b.constant);
            assert_eq!(
                wgsl_const(&m, &name),
                Some(want),
                "the WGSL `{name}` is not the ledger table's number"
            );
        }
    }
}

#[test]
fn render_context_buffers_are_bound_from_the_ledger_table() {
    check_buffer_bindings(
        &bind::module(E0_VIEW, ViewOutput::Words).unwrap_or_else(|e| panic!("{e}")),
    );
}

negative_control!(
    render_context_buffers_are_bound_from_the_ledger_table,
    "an ICDescriptor buffer bound at 4, off the table's 2, fails",
    expected = "`ic_buffer` is not bound at the ledger table's numbers",
    check_buffer_bindings(
        &bind::module(E0_VIEW, ViewOutput::Words)
            .unwrap_or_else(|e| panic!("{e}"))
            .replace(
                "@group(1) @binding(2) var<storage, read> ic_buffer",
                "@group(1) @binding(4) var<storage, read> ic_buffer"
            )
    )
);

/// The functions `entry` reaches by calls in `m`, `entry` included: its body and each callee's, transitively.
fn reachable<'m>(m: &'m naga::Module, entry: &'m naga::Function) -> Vec<&'m naga::Function> {
    fn calls(block: &naga::Block, out: &mut Vec<naga::Handle<naga::Function>>) {
        for st in block.iter() {
            match st {
                naga::Statement::Call { function, .. } => out.push(*function),
                naga::Statement::Block(b) => calls(b, out),
                naga::Statement::If { accept, reject, .. } => {
                    calls(accept, out);
                    calls(reject, out);
                }
                naga::Statement::Switch { cases, .. } => {
                    cases.iter().for_each(|c| calls(&c.body, out))
                }
                naga::Statement::Loop {
                    body, continuing, ..
                } => {
                    calls(body, out);
                    calls(continuing, out);
                }
                _ => {}
            }
        }
    }
    let mut seen: Vec<naga::Handle<naga::Function>> = Vec::new();
    let mut todo = Vec::new();
    calls(&entry.body, &mut todo);
    while let Some(h) = todo.pop() {
        if !seen.contains(&h) {
            seen.push(h);
            calls(&m.functions[h].body, &mut todo);
        }
    }
    std::iter::once(entry)
        .chain(seen.into_iter().map(|h| &m.functions[h]))
        .collect()
}

/// What `functions` load from the buffer global `buffer`: each member loaded alone, by name, and whether any loads a
/// whole element.
fn buffer_loads(
    m: &naga::Module,
    functions: &[&naga::Function],
    buffer: &str,
) -> (Vec<String>, bool) {
    let (mut members, mut whole) = (Vec::new(), false);
    for f in functions {
        let is_buffer = |e: naga::Handle<naga::Expression>| match f.expressions[e] {
            naga::Expression::GlobalVariable(g) => {
                m.global_variables[g].name.as_deref() == Some(buffer)
            }
            _ => false,
        };
        let element = |e: naga::Handle<naga::Expression>| match f.expressions[e] {
            naga::Expression::Access { base, .. } | naga::Expression::AccessIndex { base, .. } => {
                is_buffer(base)
            }
            _ => false,
        };
        for (_, expr) in f.expressions.iter() {
            let naga::Expression::Load { pointer } = *expr else {
                continue;
            };
            if element(pointer) {
                whole = true;
            }
            if let naga::Expression::AccessIndex { base, index } = f.expressions[pointer] {
                if element(base) {
                    let (_, g) = m
                        .global_variables
                        .iter()
                        .find(|(_, g)| g.name.as_deref() == Some(buffer))
                        .expect("the buffer");
                    let naga::TypeInner::Array { base: ty, .. } = m.types[g.ty].inner else {
                        panic!("`{buffer}` is not an array");
                    };
                    let naga::TypeInner::Struct {
                        members: ref ms, ..
                    } = m.types[ty].inner
                    else {
                        panic!("`{buffer}`'s element is not a struct");
                    };
                    let name = ms[index as usize].name.clone().unwrap_or_default();
                    if !members.contains(&name) {
                        members.push(name);
                    }
                }
            }
        }
    }
    (members, whole)
}

fn parse(wgsl: &str) -> naga::Module {
    naga::front::wgsl::parse_str(wgsl).unwrap_or_else(|e| panic!("{}", e.emit_to_string(wgsl)))
}

/// No function of the harness module `wgsl` loads a whole `ICDescriptor` or `RenderQuad` element (R-378).
fn check_no_whole_loads(wgsl: &str) {
    let m = parse(wgsl);
    let functions: Vec<&naga::Function> = m
        .functions
        .iter()
        .map(|(_, f)| f)
        .chain(m.entry_points.iter().map(|e| &e.function))
        .collect();
    for buffer in ["ic_buffer", "quad_buffer"] {
        let (_, whole) = buffer_loads(&m, &functions, buffer);
        assert!(
            !whole,
            "a function loads a whole element of `{buffer}` (R-378)"
        );
    }
}

#[test]
fn render_context_buffers_are_read_one_member_per_load() {
    check_no_whole_loads(
        &bind::module(E0_VIEW, ViewOutput::Words).unwrap_or_else(|e| panic!("{e}")),
    );
    check_no_whole_loads(&e0_stain());
}

negative_control!(
    render_context_buffers_are_read_one_member_per_load,
    "a reader returning `ic_buffer[i]` whole fails",
    expected = "a function loads a whole element of `ic_buffer`",
    check_no_whole_loads(&format!(
        "{}\nfn whole(i: u32) -> ICDescriptor {{ return ic_buffer[i]; }}\n",
        bind::module(E0_VIEW, ViewOutput::Words).unwrap_or_else(|e| panic!("{e}"))
    ))
);

/// The stain entry point of `wgsl`, through every function it calls, loads only the masses `m0`, `m1` and `m2` of
/// `ICDescriptor`, and nothing of `RenderQuad` (R-378).
fn check_stain_loads_the_masses(wgsl: &str) {
    let m = parse(wgsl);
    let entry = m
        .entry_points
        .iter()
        .find(|e| e.name == bind::STAIN_ENTRY)
        .expect("the stain entry point");
    let functions = reachable(&m, &entry.function);
    let (mut ic, whole) = buffer_loads(&m, &functions, "ic_buffer");
    ic.sort();
    assert!(
        !whole && ic == ["m0", "m1", "m2"],
        "the stain loads {ic:?} of ICDescriptor (whole: {whole}), not only the masses (R-378)"
    );
    let (quad, whole) = buffer_loads(&m, &functions, "quad_buffer");
    assert!(
        !whole && quad.is_empty(),
        "the stain loads {quad:?} of RenderQuad (whole: {whole}) (R-378)"
    );
}

#[test]
fn synthetic_upload_stain_loads_only_the_masses() {
    check_stain_loads_the_masses(&e0_stain());
}

negative_control!(
    synthetic_upload_stain_loads_only_the_masses,
    "a stain reading its masses through the whole-ICDescriptor reader fails",
    expected = "not only the masses",
    check_stain_loads_the_masses(&e0_stain().replace(
        "vec3<f32>(ic_read_m0(r.sample), ic_read_m1(r.sample), ic_read_m2(r.sample))",
        "vec3<f32>(ic_read(r.sample).m0, ic_read(r.sample).m1, ic_read(r.sample).m2)"
    ))
);

// ── REQ-COL-003, REQ-COL-056: the ctx lanes ─────────────────────────────────────────────────────────────────────

fn colour_composition() -> String {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/design/principia_colour_composition.md"
    ))
    .expect("read colour_composition")
}

/// §3's lane table in `doc`: each lane, as `Lanes` names it (`tile/sample` is `tile`), and the fields its row names in
/// backticks.
fn section_3_lanes(doc: &str) -> Vec<(String, Vec<String>)> {
    doc.lines()
        .skip_while(|l| !l.starts_with("## 3. The `ctx` contract"))
        .take_while(|l| !l.starts_with("## 4."))
        .filter_map(|l| {
            let cells: Vec<&str> = l.split('|').map(str::trim).collect();
            let lane = cells.get(1)?.strip_prefix("**")?.strip_suffix("**")?;
            let lane = lane.split('/').next()?.to_owned();
            let fields = cells
                .get(2)?
                .split('`')
                .skip(1)
                .step_by(2)
                .map(str::to_owned)
                .collect();
            Some((lane, fields))
        })
        .collect()
}

/// §3's names for fields the generated lanes hold under another: `t_end` is stored as `t_end_step` (payload §1), and
/// the diffusion predicate `n ≥ 2` is `diffusion`'s validity, `diffusion_valid` (R-245).
const ALIASES: [(&str, &str, &str); 2] = [
    ("payload", "t_end", "t_end_step"),
    ("validity", "n ≥ 2", "diffusion_valid"),
];

/// The generated lanes declare every lane of `doc`'s §3, in its order, and every field its row names.
fn check_lanes(doc: &str, wgsl: &str) {
    let want = section_3_lanes(doc);
    let names: Vec<String> = struct_members(wgsl, "Lanes")
        .into_iter()
        .map(|(n, _)| n)
        .collect();
    let lanes: Vec<String> = want.iter().map(|(l, _)| l.clone()).collect();
    assert_eq!(names, lanes, "the generated ctx's lanes are not §3's");
    let lanes = bind::lanes().unwrap_or_else(|e| panic!("{e}"));
    for (lane, fields) in &want {
        let ty = lanes
            .iter()
            .find(|l| l.name == lane)
            .map(|l| l.ty)
            .unwrap_or_else(|| panic!("the generated ctx declares no {lane} lane"));
        let declared: Vec<String> = struct_members(wgsl, ty)
            .into_iter()
            .map(|(n, _)| n)
            .collect();
        for field in fields {
            let name = ALIASES
                .iter()
                .find(|(l, f, _)| l == lane && f == field)
                .map_or(field.as_str(), |(_, _, to)| to);
            assert!(
                declared.iter().any(|d| d == name),
                "the generated ctx's {lane} lane does not declare §3's `{field}`"
            );
        }
    }
}

#[test]
fn ctx_lanes_declare_every_lane_of_section_3() {
    let wgsl = bind::context().unwrap_or_else(|e| panic!("{e}"));
    check_lanes(&colour_composition(), &wgsl);
    // REQ-COL-056: §3's tile/sample lane lists `uv`, and the generated lane declares it.
    let doc = colour_composition();
    let tile = section_3_lanes(&doc)
        .into_iter()
        .find(|(l, _)| l == "tile")
        .expect("§3's tile/sample lane");
    assert!(
        tile.1.iter().any(|f| f == "uv"),
        "§3's tile/sample lane lists no `uv`"
    );
}

negative_control!(
    ctx_lanes_declare_every_lane_of_section_3,
    "a §3 whose quad lane names `sample_total`, which no lane declares, fails the check",
    expected = "does not declare §3's `sample_total`",
    check_lanes(
        &colour_composition().replace("`sample_count`", "`sample_total`"),
        &bind::context().unwrap_or_else(|e| panic!("{e}"))
    )
);

/// Every payload-lane field has a bool `<field>_valid` in the validity lane, and the payload lane holds every
/// `ICDescriptor` field and the read side's fields.
fn check_validity(lanes: &[bind::Lane]) {
    let lane = |name: &str| {
        lanes
            .iter()
            .find(|l| l.name == name)
            .unwrap_or_else(|| panic!("no {name} lane"))
    };
    let validity = lane("validity");
    for p in &lane("payload").members {
        let want = format!("{}_valid", p.name);
        assert!(
            validity
                .members
                .iter()
                .any(|v| v.name == want && v.ty == "bool"),
            "payload field `{}` has no validity accessor `{want}`",
            p.name
        );
    }
    for field in [
        "E_0",
        "ftle",
        "diffusion",
        "d_min",
        "word",
        "t_end_step",
        "m0",
        "K_0",
        "r_min_pair_0",
    ] {
        assert!(
            lane("payload").members.iter().any(|p| p.name == field),
            "the payload lane holds no `{field}`"
        );
    }
}

#[test]
fn ctx_lanes_every_payload_field_has_a_validity_accessor() {
    check_validity(&bind::lanes().unwrap_or_else(|e| panic!("{e}")));
}

negative_control!(
    ctx_lanes_every_payload_field_has_a_validity_accessor,
    "a validity lane without `d_min_valid` fails the check",
    expected = "payload field `d_min` has no validity accessor",
    {
        let mut lanes = bind::lanes().unwrap_or_else(|e| panic!("{e}"));
        for l in &mut lanes {
            l.members.retain(|m| m.name != "d_min_valid");
        }
        check_validity(&lanes)
    }
);

/// The validity view: `d_min_valid`, `dmin_pair_valid`, `ensemble_spread_valid` and `sd_is_failed`, each 0 or 1.
const VALIDITY_VIEW: &str = "fn view(rc: RenderContext, l: Lanes) -> vec4<u32> {
    return vec4<u32>(select(0u, 1u, l.validity.d_min_valid), select(0u, 1u, l.validity.dmin_pair_valid),
        select(0u, 1u, l.validity.ensemble_spread_valid), select(0u, 1u, l.validity.sd_is_failed));
}";

/// A set where sample `i` (base samples only) has `d_min` set when `i` is odd, `dmin_pair` 1 when `i % 3 == 0`, and is
/// `sim_failed` (code 4) when `i % 5 == 0`.
fn validity_set(grid: Grid) -> Synthetic {
    let mut set = Synthetic::flat(grid, 0);
    for i in 0..grid.sample_count() {
        let mut s = set.sample(i);
        if i % 2 == 1 {
            s.d_min(0.5);
        }
        if i.is_multiple_of(3) {
            s.dmin_pair(1);
        }
        if i.is_multiple_of(5) {
            s.state(4);
        }
    }
    set
}

/// Each pixel's validity words are its sample's, as [`validity_set`] wrote it, with `ensemble_spread` valid as E ≥ 1.
fn check_validity_drawn(grid: Grid, image: &Image) {
    let (width, height) = grid.target();
    for y in 0..height {
        for x in 0..width {
            let i = grid.cell(x, y).sample;
            let want = [
                i % 2,
                u32::from(i.is_multiple_of(3)),
                u32::from(grid.e >= 1),
                u32::from(i.is_multiple_of(5)),
            ];
            assert_eq!(
                image.words(x, y),
                want,
                "pixel ({x}, {y}): sample {i}'s validity lane is not its ledger predicates"
            );
        }
    }
}

#[test]
fn ctx_lanes_validity_reads_the_ledger_predicates() {
    let h = gpu();
    for e in [0, 1] {
        let g = Grid { e, ..grid() };
        let image = draw_words(&h, &validity_set(g), &context(g), VALIDITY_VIEW);
        check_validity_drawn(g, &image);
    }
}

negative_control!(
    ctx_lanes_validity_reads_the_ledger_predicates,
    "validity drawn at E = 0 checked as if E were 1 fails on ensemble_spread",
    expected = "validity lane is not its ledger predicates",
    {
        let g = Grid { e: 0, ..grid() };
        let image = draw_words(&gpu(), &validity_set(g), &context(g), VALIDITY_VIEW);
        check_validity_drawn(Grid { e: 1, ..g }, &image)
    }
);

/// The diffusion validity view: `diffusion_valid`, 0 or 1.
const DIFFUSION_VALIDITY_VIEW: &str = "fn view(rc: RenderContext, l: Lanes) -> vec4<u32> {
    return vec4<u32>(select(0u, 1u, l.validity.diffusion_valid), 0u, 0u, 0u);
}";

/// A set where sample `i` has `t_end_step`, the `n` of the diffusion slope, `i % 4`, or `n` for every sample.
fn steps_set(grid: Grid, n: Option<u32>) -> Synthetic {
    let mut set = Synthetic::flat(grid, 0);
    for i in 0..grid.sample_count() {
        set.sample(i).times(n.unwrap_or(i % 4), 0);
    }
    set
}

/// Each pixel's `diffusion_valid` is the predicate `n ≥ 2` over its sample's `n = i % 4` (R-245; payload §6's
/// `diffusion_slope_valid`; colour_composition §3).
fn check_diffusion_validity(grid: Grid, image: &Image) {
    let (width, height) = grid.target();
    for y in 0..height {
        for x in 0..width {
            let i = grid.cell(x, y).sample;
            assert_eq!(
                image.words(x, y)[0],
                u32::from(i % 4 >= 2),
                "pixel ({x}, {y}): sample {i}'s diffusion_valid is not n >= 2 for n = {}",
                i % 4
            );
        }
    }
}

#[test]
fn ctx_lanes_diffusion_validity_is_n_at_least_2() {
    let g = grid();
    let image = draw_words(
        &gpu(),
        &steps_set(g, None),
        &context(g),
        DIFFUSION_VALIDITY_VIEW,
    );
    check_diffusion_validity(g, &image);
}

negative_control!(
    ctx_lanes_diffusion_validity_is_n_at_least_2,
    "a set whose every sample has n = 2 is valid where n = i % 4 is not",
    expected = "diffusion_valid is not n >= 2",
    {
        let g = grid();
        let image = draw_words(
            &gpu(),
            &steps_set(g, Some(2)),
            &context(g),
            DIFFUSION_VALIDITY_VIEW,
        );
        check_diffusion_validity(g, &image)
    }
);

/// The uv view: `ctx.tile.uv` and `ctx.quad.uv`, as bits.
const UV_VIEW: &str = "fn view(rc: RenderContext, l: Lanes) -> vec4<u32> {
    return vec4<u32>(bitcast<vec2<u32>>(l.tile.uv), bitcast<vec2<u32>>(l.quad.uv));
}";

/// Each pixel's `ctx.tile.uv` is its centre's position in its tile, in [0, 1]², from the bottom-left, Y-up, and
/// `ctx.quad.uv = (vec2(i, j) + ctx.tile.uv) / N` (colour_composition §3; REQ-COL-056).
fn check_uv(grid: Grid, image: &Image) {
    let (width, height) = grid.target();
    let t = grid.tile_px as f32;
    for y in 0..height {
        for x in 0..width {
            let c = grid.cell(x, y);
            let w = image.words(x, y);
            let got = [f32::from_bits(w[0]), f32::from_bits(w[1])];
            let quad = [f32::from_bits(w[2]), f32::from_bits(w[3])];
            let up = [x as f32 + 0.5, height as f32 - (y as f32 + 0.5)];
            let (x0, _, _, y1) = grid.tile_pixels(c.quad_xy, c.tile_xy);
            let want = [(up[0] - x0 as f32) / t, (up[1] - (height - y1) as f32) / t];
            for k in 0..2 {
                assert!(
                    (got[k] - want[k]).abs() < 1e-6 && (0.0..=1.0).contains(&got[k]),
                    "pixel ({x}, {y}): ctx.tile.uv {got:?} is not {want:?}"
                );
                let q = (c.tile_xy[k] as f32 + got[k]) / grid.n as f32;
                assert!(
                    (quad[k] - q).abs() < 1e-6,
                    "pixel ({x}, {y}): ctx.quad.uv {quad:?} is not (tile + ctx.tile.uv) / N"
                );
            }
        }
    }
}

#[test]
fn ctx_lanes_tile_uv_is_within_tile_y_up() {
    let g = grid();
    let image = draw_words(&gpu(), &Synthetic::flat(g, 0), &context(g), UV_VIEW);
    check_uv(g, &image);
    // The picture is not symmetric under a Y flip, so the check sees a Y-down `v` (its control).
    assert_ne!(
        flipped(&image),
        image,
        "the uv picture is symmetric under a Y flip"
    );
}

negative_control!(
    ctx_lanes_tile_uv_is_within_tile_y_up,
    "the uv drawn with its rows reversed, a Y-down v, fails the check",
    expected = "is not",
    {
        let g = grid();
        let image = draw_words(&gpu(), &Synthetic::flat(g, 0), &context(g), UV_VIEW);
        check_uv(g, &flipped(&image))
    }
);

/// The sample-count view: `ctx.quad.sample_count`.
const SAMPLE_COUNT_VIEW: &str = "fn view(rc: RenderContext, l: Lanes) -> vec4<u32> {
    return vec4<u32>(l.quad.sample_count, 0u, 0u, 0u);
}";

/// Each pixel's `ctx.quad.sample_count` is `want`.
fn check_sample_count(grid: Grid, image: &Image, want: u32) {
    let (width, height) = grid.target();
    for y in 0..height {
        for x in 0..width {
            assert_eq!(
                image.words(x, y)[0],
                want,
                "pixel ({x}, {y}): ctx.quad.sample_count is not the quad's valid sample count"
            );
        }
    }
}

/// `ctx.quad.sample_count` is dd_generation_root §3.7's `valid_sample_count`, decoded of N², as the harness sets it
/// (13 of the grid's 16), not a geometric count such as N²(E + 1).
#[test]
fn ctx_lanes_quad_sample_count_is_the_valid_sample_count() {
    let g = grid();
    let ctx = context(g);
    assert!(ctx.valid_sample_count < g.n * g.n);
    let image = draw_words(&gpu(), &Synthetic::flat(g, 0), &ctx, SAMPLE_COUNT_VIEW);
    check_sample_count(g, &image, ctx.valid_sample_count);
}

negative_control!(
    ctx_lanes_quad_sample_count_is_the_valid_sample_count,
    "the lane checked against N²(E + 1), the geometric count of samples with copies, fails",
    expected = "is not the quad's valid sample count",
    {
        let g = grid();
        let image = draw_words(
            &gpu(),
            &Synthetic::flat(g, 0),
            &context(g),
            SAMPLE_COUNT_VIEW,
        );
        check_sample_count(g, &image, g.n * g.n * (g.e + 1))
    }
);

// ── REQ-TOOL-014: a CPU-filled set uploads and renders ─────────────────────────────────────────────────────────

/// A set with hand-chosen values per sample and per quad: `E_0`, `state`/`detail`, `t_end_step`, the word's `x` and
/// `ICDescriptor`'s `K_0` and `m1` from the sample's index; each quad's depth, impurity and state from its index.
fn filled(grid: Grid) -> Synthetic {
    let mut set = Synthetic::flat(grid, 0);
    for i in 0..grid.sample_count() {
        set.sample(i)
            .E_0(-0.5 - i as f32)
            .state(i % 6)
            .detail(i % 4)
            .times(i % 1000, 0)
            .word([i * 7 + 1, 0, 0, 0], 3);
        let ic = set.ic(i);
        ic.K_0 = 0.25 * i as f32;
        ic.m1 = 0.5;
    }
    for q in 0..grid.quad_count() {
        set.quad(q)
            .u32("quad_depth", q + 3)
            .and_then(|s| s.f32("outcome_impurity", 0.125 * q as f32))
            .and_then(|s| s.u32("quad_state", q % 5))
            .unwrap_or_else(|e| panic!("{e}"));
    }
    set
}

const FILLED_VIEWS: [&str; 2] = [
    "fn view(rc: RenderContext, l: Lanes) -> vec4<u32> {
    return vec4<u32>(bitcast<u32>(rc.sample.E_0), rc.sample.state | (rc.sample.detail << 8u), bitcast<u32>(rc.ic.K_0),
        rc.quad.quad_depth);
}",
    "fn view(rc: RenderContext, l: Lanes) -> vec4<u32> {
    return vec4<u32>(bitcast<u32>(l.quad.impurity), l.payload.t_end_step, l.payload.word.x,
        bitcast<u32>(l.payload.m1) ^ l.quad.state);
}",
];

/// Each pixel shows its sample's and its quad's values as `filled(grid)` wrote them, through `RenderContext` (view 0)
/// and through the lanes (view 1).
fn check_filled(grid: Grid, images: &[Image; 2], shift: u32) {
    let (width, height) = grid.target();
    for y in 0..height {
        for x in 0..width {
            let c = grid.cell(x, y);
            let (i, q) = (c.sample + shift, c.quad);
            let want = [
                [
                    (-0.5 - i as f32).to_bits(),
                    (i % 6) | ((i % 4) << 8),
                    (0.25 * i as f32).to_bits(),
                    q + 3,
                ],
                [
                    (0.125 * q as f32).to_bits(),
                    i % 1000,
                    i * 7 + 1,
                    0.5f32.to_bits() ^ (q % 5),
                ],
            ];
            for (v, image) in images.iter().enumerate() {
                assert_eq!(
                    image.words(x, y),
                    want[v],
                    "pixel ({x}, {y}), view {v}: the rendered values are not the CPU-filled set's"
                );
            }
        }
    }
}

fn draw_filled(h: &GpuHarness, g: Grid) -> [Image; 2] {
    let set = filled(g);
    FILLED_VIEWS.map(|v| draw_words(h, &set, &context(g), v))
}

#[test]
fn synthetic_upload_renders_the_render_context() {
    let g = grid();
    check_filled(g, &draw_filled(&gpu(), g), 0);
}

negative_control!(
    synthetic_upload_renders_the_render_context,
    "the set checked one sample along fails",
    expected = "the rendered values are not the CPU-filled set's",
    {
        let g = grid();
        check_filled(g, &draw_filled(&gpu(), g), 1)
    }
);

/// The stain: the built-in source of `E_0` into a colour that writes `-E_0 / 16` to red, the pass-through combiner and
/// OUT, assembled through the one entry point (render contract Part 2).
fn e0_stain() -> String {
    let node = |kind, occupant, inputs: &[Option<usize>]| Node {
        kind,
        occupant,
        inputs: inputs.to_vec(),
    };
    let colour =
        "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(-ctx.inputs[0].x / 16.0, 0.0, 0.0); }";
    let stain = Stain::new(vec![
        node(Kind::Source, Occupant::Field("E_0".into()), &[]),
        node(Kind::Colour, Occupant::Custom(colour.into()), &[Some(0)]),
        node(
            Kind::Combiner,
            Occupant::BuiltIn("pass_through".into()),
            &[Some(1), None],
        ),
        node(Kind::Out, Occupant::None, &[Some(2)]),
    ])
    .unwrap_or_else(|e| panic!("{e}"));
    let fragment = assemble(&stain, Tier::FULL).unwrap_or_else(|e| panic!("{e}"));
    bind::stain_module(&fragment.source)
}

/// A set whose sample `i` has `E_0 = -(i mod 16)`, so red is `(i mod 16) / 16`.
fn ramp(grid: Grid) -> Synthetic {
    let mut set = Synthetic::flat(grid, 0);
    for i in 0..grid.sample_count() {
        set.sample(i).E_0(-((i % 16) as f32));
    }
    set
}

/// Each pixel's red is its tile's sample's `(i mod 16) / 16` in 8-bit unorm, `shift` samples along for a control.
fn check_ramp(grid: Grid, image: &Image, shift: u32) {
    let (width, height) = grid.target();
    for y in 0..height {
        for x in 0..width {
            let i = grid.cell(x, y).sample + shift;
            let want = ((i % 16) as f32 / 16.0 * 255.0).round() as i32;
            let texel = image.texel(x, y);
            assert!(
                (i32::from(texel[0]) - want).abs() <= 1 && texel[1] == 0 && texel[3] == 255,
                "pixel ({x}, {y}): the screen does not colour the hand-filled buffer: {texel:?}, red {want}"
            );
        }
    }
}

#[test]
fn synthetic_upload_renders_an_assembled_stain() {
    let g = grid();
    let image = draw(
        &gpu(),
        &ramp(g),
        &context(g),
        &e0_stain(),
        bind::STAIN_ENTRY,
        wgpu::TextureFormat::Rgba8Unorm,
    );
    check_ramp(g, &image, 0);
}

negative_control!(
    synthetic_upload_renders_an_assembled_stain,
    "the ramp checked one sample along fails",
    expected = "the screen does not colour the hand-filled buffer",
    {
        let g = grid();
        let image = draw(
            &gpu(),
            &ramp(g),
            &context(g),
            &e0_stain(),
            bind::STAIN_ENTRY,
            wgpu::TextureFormat::Rgba8Unorm,
        );
        check_ramp(g, &image, 1)
    }
);
