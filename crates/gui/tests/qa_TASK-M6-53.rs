//! QA tests for TASK-M6-53, written from REQ-GUI-412, R-685, R-689 and ADR-0001 § "Decision", not from the
//! implementation or its tests:
//!
//! - REQ-GUI-412: "On the mock engine, a value-noise lattice corner shared by adjacent cells must be bitwise equal from
//!   each cell, on every backend the regression test runs on".
//! - R-689: "an integer u32 hash on integer lattice coordinates", with ADR-0001's corner: the PCG hash of Jarzynski and
//!   Olano (2020), its top 24 bits times 2⁻²⁴; integer additions; `f` and `c` from one `floor`; no cast of `floor`
//!   straight to u32, which saturates the negative half-plane to 0.
//!
//! These run the shipped `noise_source()` in entry points of their own, drawn one pixel per input and read back as bits,
//! and go by the integer lattice itself: every quadrant, and both ends of `i32`, which no view reaches. They read
//! `value_noise` as well as `cell_corners`, since a probe that takes `floor` itself says nothing of the cell
//! `value_noise` takes. Their names hold no `lattice_`, so `gpu-metal`'s step reads none of this file and R-692's five
//! paths stay five; they run on lavapipe in the `ci` shards, and were run on Metal for the review.
//!
//! gui takes no dependency on validation (R-176, R-187), so each check runs its control through [`rejects`]: the shipped
//! source with one change, checked to compile outside `rejects`, so a control that fails to compile is no pass.
// The file name `qa_TASK-M6-53` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::collections::HashMap;

use engine::contract::canvas::Canvas;
use gui::layout::Placement;
use gui::mock::canvas::{noise_source, uniform_bytes, MockCanvas, CHART_BINDING, NO_INVALID};
use naga::valid::{Capabilities, ModuleInfo, ValidationFlags, Validator};
use naga::{Expression, Function, Handle, MathFunction, Module, ScalarKind};

/// Asserts that `check` panics: the check can fail (R-176's control, run in the test).
fn rejects(what: &str, check: impl FnOnce()) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check));
    assert!(
        outcome.is_err(),
        "the check passed on {what}, which it must reject"
    );
}

/// `pcg_hash`, as Jarzynski and Olano publish it ("Hash Functions for GPU Rendering", JCGT 9(3), 2020).
fn pcg_hash(input: u32) -> u32 {
    let state = input.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
    let word = ((state >> ((state >> 28) + 4)) ^ state).wrapping_mul(277_803_737);
    (word >> 22) ^ word
}

/// The hash of a lattice point, `pcg(y ^ pcg(x))` on the coordinates' two's-complement bits (ADR-0001 § "Decision").
fn point_hash([x, y]: [i32; 2]) -> u32 {
    pcg_hash(y as u32 ^ pcg_hash(x as u32))
}

/// The noise at a lattice point: the hash's top 24 bits over 2²⁴, which an f32 holds exactly.
fn corner_value(c: [i32; 2]) -> f32 {
    (point_hash(c) >> 8) as f32 / 16_777_216.0
}

/// A cell's corners in `cell_corners`' order, as lattice points.
fn corner_points([x, y]: [i32; 2]) -> [[i32; 2]; 4] {
    [[0, 0], [1, 0], [0, 1], [1, 1]].map(|[o, p]| [x.wrapping_add(o), y.wrapping_add(p)])
}

/// The cells: the lattice about the origin, all four quadrants, and the corners of `i32`'s range and of f32's integers.
fn cells() -> Vec<[i32; 2]> {
    const EDGES: [i32; 10] = [
        i32::MIN,
        i32::MIN + 1,
        -(1 << 24),
        -2,
        -1,
        0,
        (1 << 24) - 1,
        1 << 24,
        i32::MAX - 1,
        i32::MAX,
    ];
    let mut all = Vec::new();
    for x in -9..=9 {
        for y in -9..=9 {
            all.push([x, y]);
        }
    }
    for x in EDGES {
        for y in EDGES {
            all.push([x, y]);
        }
    }
    all.sort_unstable();
    all.dedup();
    all
}

/// The width of a probe's target, in pixels, each pixel one input.
const WIDTH: u32 = 256;
/// The target's format: bits, which nothing converts.
const BITS: wgpu::TextureFormat = wgpu::TextureFormat::Rgba32Uint;

/// The entry points appended to the shipped module: each takes its input as bits and gives its result back as bits.
const PROBES: &str = r#"
@group(0) @binding(20) var<storage, read> qa_in: array<vec2<u32>>;

@vertex
fn qa_vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let corner = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(corner * 2.0 - 1.0, 0.0, 1.0);
}

// The input of the pixel at `position`, the inputs in reading order, `QA_WIDTH` of them to a row.
fn qa_input(position: vec4<f32>) -> vec2<u32> {
    let pixel = vec2<u32>(position.xy);
    return qa_in[pixel.y * QA_WIDTH + pixel.x];
}

@fragment
fn qa_cell_corners(@builtin(position) position: vec4<f32>) -> @location(0) vec4<u32> {
    return bitcast<vec4<u32>>(cell_corners(bitcast<vec2<i32>>(qa_input(position))));
}

@fragment
fn qa_value_noise(@builtin(position) position: vec4<f32>) -> @location(0) vec4<u32> {
    return vec4<u32>(bitcast<u32>(value_noise(bitcast<vec2<f32>>(qa_input(position)))), 0u, 0u, 0u);
}

@fragment
fn qa_stand_in(@builtin(position) position: vec4<f32>) -> @location(0) vec4<u32> {
    let field = stand_in_at(bitcast<vec2<f32>>(qa_input(position)), vec2<f32>(0.0, 0.0));
    return bitcast<vec4<u32>>(vec4<f32>(field.n, field.warp, 0.0));
}
"#;

/// The shipped source with the one occurrence of `from` changed to `to`.
fn changed(from: &str, to: &str) -> String {
    let source = noise_source();
    assert_eq!(source.matches(from).count(), 1, "no one {from} to change");
    source.replace(from, to)
}

/// `entry` of `source` and [`PROBES`] over `input`, one pixel each, with the uniform `chart` if it reads one: each
/// result's bits.
fn probe(
    canvas: &MockCanvas,
    source: &str,
    entry: &str,
    input: &[[u32; 2]],
    chart: Option<&[u8]>,
) -> Vec<[u32; 4]> {
    let context = canvas.context();
    let device = &context.device;
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("qa lattice"),
        source: wgpu::ShaderSource::Wgsl(
            format!("{source}\nconst QA_WIDTH: u32 = {WIDTH}u;\n{PROBES}").into(),
        ),
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("qa lattice"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("qa_vertex"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some(entry),
            compilation_options: Default::default(),
            targets: &[Some(BITS.into())],
        }),
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    });
    let bytes: Vec<u8> = input
        .iter()
        .flatten()
        .flat_map(|word| word.to_ne_bytes())
        .collect();
    let source_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("qa lattice"),
        size: bytes.len() as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let rows = (input.len() as u32).div_ceil(WIDTH);
    let row = WIDTH * 16;
    let extent = wgpu::Extent3d {
        width: WIDTH,
        height: rows,
        depth_or_array_layers: 1,
    };
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("qa lattice"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: BITS,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("qa lattice readback"),
        size: u64::from(row) * u64::from(rows),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    context.queue.write_buffer(&source_buffer, 0, &bytes);
    let chart = chart.map(|bytes| {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("qa lattice chart"),
            size: bytes.len() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        context.queue.write_buffer(&buffer, 0, bytes);
        buffer
    });
    let mut entries = vec![wgpu::BindGroupEntry {
        binding: 20,
        resource: source_buffer.as_entire_binding(),
    }];
    entries.extend(chart.iter().map(|buffer| wgpu::BindGroupEntry {
        binding: CHART_BINDING,
        resource: buffer.as_entire_binding(),
    }));
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("qa lattice"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &entries,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(rows),
            },
        },
        extent,
    );
    context.queue.submit([encoder.finish()]);
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, |r| r.expect("qa lattice map"));
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("device poll failed");
    let mapped = readback
        .slice(..)
        .get_mapped_range()
        .expect("qa lattice range");
    let words: Vec<u32> = mapped
        .chunks_exact(4)
        .map(|word| u32::from_ne_bytes(word.try_into().unwrap()))
        .collect();
    words
        .chunks_exact(4)
        .take(input.len())
        .map(|w| [w[0], w[1], w[2], w[3]])
        .collect()
}

/// The cells' corners as `source` gives them.
fn cell_corners_of(canvas: &MockCanvas, source: &str, cells: &[[i32; 2]]) -> Vec<[u32; 4]> {
    let input: Vec<[u32; 2]> = cells.iter().map(|c| c.map(|v| v as u32)).collect();
    probe(canvas, source, "qa_cell_corners", &input, None)
}

/// `value_noise` at `points` as `source` gives it, its bits.
fn value_noise_of(canvas: &MockCanvas, source: &str, points: &[[f32; 2]]) -> Vec<u32> {
    let input: Vec<[u32; 2]> = points.iter().map(|p| p.map(f32::to_bits)).collect();
    probe(canvas, source, "qa_value_noise", &input, None)
        .into_iter()
        .map(|[bits, ..]| bits)
        .collect()
}

/// The twin itself pinned, with values worked by hand in Python from the published form, no code of the PR's run.
#[test]
fn qa_m653_the_published_hash_at_known_points() {
    assert_eq!(point_hash([0, 0]), 817_759_070);
    assert_eq!(point_hash([1, 0]), 3_911_864_714);
    assert_eq!(point_hash([0, 1]), 1_583_546_800);
    assert_eq!(point_hash([-1, -1]), 3_094_549_908);
    assert_eq!(point_hash([i32::MIN, i32::MAX]), 1_921_998_388);
    // The 24-bit top converts exactly, at the largest hash too.
    assert_eq!(((u32::MAX >> 8) as f32) as u32, u32::MAX >> 8);
    rejects("a cast that saturates the negative half-plane", || {
        assert_eq!(pcg_hash(pcg_hash((-1.0f32) as u32)), point_hash([-1, 0]));
    });
}

/// Asserts that each cell's four corners are the published noise at its integer lattice points.
fn assert_published(cells: &[[i32; 2]], corners: &[[u32; 4]]) {
    for (&cell, seen) in cells.iter().zip(corners) {
        let expected = corner_points(cell).map(|c| corner_value(c).to_bits());
        assert_eq!(*seen, expected, "the corners of cell {cell:?}");
    }
}

#[test]
fn qa_m653_corners_are_the_integer_hash_in_every_quadrant() {
    let canvas = MockCanvas::new().expect("a GPU adapter for the mock's canvas");
    let cells = cells();
    assert_published(&cells, &cell_corners_of(&canvas, &noise_source(), &cells));

    // A corner whose conversion rounds: 25 bits, which one multiply by a power of two still follows.
    let rounded = cell_corners_of(
        &canvas,
        &changed(">> 8u) * 0x1p-24f", ">> 7u) * 0x1p-25f"),
        &cells,
    );
    rejects("a 25-bit corner", || assert_published(&cells, &rounded));
    // The cast ADR-0001 forbids, through f32, which saturates the negative half-plane to 0.
    let saturated = cell_corners_of(
        &canvas,
        &changed("let u = vec2<u32>(c);", "let u = vec2<u32>(vec2<f32>(c));"),
        &cells,
    );
    rejects("the saturating cast", || {
        assert_published(&cells, &saturated)
    });
    let swapped = cell_corners_of(
        &canvas,
        &changed("return pcg(u.y ^ pcg(u.x));", "return pcg(u.x ^ pcg(u.y));"),
        &cells,
    );
    rejects("a hash with its axes swapped", || {
        assert_published(&cells, &swapped)
    });
}

/// The shared corners counted, by the neighbour's offset, asserting each equal bit for bit: the statement itself, from
/// the two cells' own reads, with no twin. An adjacent pair past `i32`'s end is adjacent: the lattice wraps.
fn assert_shared_equal(cells: &[[i32; 2]], corners: &[[u32; 4]]) -> [usize; 4] {
    const NEIGHBOURS: [[i32; 2]; 4] = [[1, 0], [0, 1], [1, 1], [1, -1]];
    let read: HashMap<[i32; 2], [u32; 4]> =
        cells.iter().copied().zip(corners.iter().copied()).collect();
    let mut shared = [0; 4];
    for (&cell, here) in cells.iter().zip(corners) {
        for (count, [dx, dy]) in shared.iter_mut().zip(NEIGHBOURS) {
            let next = [cell[0].wrapping_add(dx), cell[1].wrapping_add(dy)];
            let Some(there) = read.get(&next) else {
                continue;
            };
            for (i, a) in corner_points(cell).iter().enumerate() {
                for (j, b) in corner_points(next).iter().enumerate() {
                    if a == b {
                        assert_eq!(
                            here[i], there[j],
                            "corner {a:?} read from {cell:?} and from {next:?}"
                        );
                        *count += 1;
                    }
                }
            }
        }
    }
    shared
}

#[test]
fn qa_m653_a_shared_corner_is_one_value_from_each_cell() {
    let canvas = MockCanvas::new().expect("a GPU adapter for the mock's canvas");
    let cells = cells();
    let shipped = cell_corners_of(&canvas, &noise_source(), &cells);
    let [x, y, up, down] = assert_shared_equal(&cells, &shipped);
    // The 19 × 19 block's crossings at least: 18 × 19 edges of two corners each, 18 × 18 diagonals of one each.
    assert!(x >= 18 * 19 * 2, "{x} corners shared along x");
    assert!(y >= 18 * 19 * 2, "{y} corners shared along y");
    assert!(up >= 18 * 18, "{up} corners shared up a diagonal");
    assert!(down >= 18 * 18, "{down} corners shared down a diagonal");

    for (what, from, to) in [
        (
            "a corner of the wrong lattice point",
            "corner(c + vec2<i32>(1, 0))",
            "corner(c + vec2<i32>(2, 0))",
        ),
        (
            "a corner that is no lattice point's",
            "corner(c + vec2<i32>(0, 1))",
            "corner(c + vec2<i32>(1, 1)) * 0.5",
        ),
    ] {
        let control = cell_corners_of(&canvas, &changed(from, to), &cells);
        rejects(what, || {
            assert_shared_equal(&cells, &control);
        });
    }
}

/// The largest error of `value_noise` at a cell's centre, against the corners' mean: `u` is exactly 0.5 there, and each
/// of `mix`'s three blends rounds at most three times by half an ulp below 1, so 9 × 2⁻²⁵ at most, under this.
const CENTRE_ERROR: f64 = 1.0 / 2_097_152.0;

/// Asserts that `value_noise` reads, at a lattice point, that point's corner bit for bit, and at a cell's centre, the
/// mean of the four corners of the cell `floor` gives.
fn assert_value_noise(canvas: &MockCanvas, source: &str) {
    let mut points = Vec::new();
    for x in -9..=9 {
        for y in -9..=9 {
            points.push([x, y]);
        }
    }
    points.extend([[-(1 << 20), 1 << 20], [1 << 20, -(1 << 20)]]);
    let at: Vec<[f32; 2]> = points.iter().map(|c| c.map(|v| v as f32)).collect();
    for (c, seen) in points.iter().zip(value_noise_of(canvas, source, &at)) {
        assert_eq!(
            seen,
            corner_value(*c).to_bits(),
            "value_noise at the lattice point {c:?}"
        );
    }
    let at: Vec<[f32; 2]> = points.iter().map(|c| c.map(|v| v as f32 + 0.5)).collect();
    for (c, seen) in points.iter().zip(value_noise_of(canvas, source, &at)) {
        let mean = corner_points(*c)
            .iter()
            .map(|&point| f64::from(corner_value(point)))
            .sum::<f64>()
            / 4.0;
        let error = (f64::from(f32::from_bits(seen)) - mean).abs();
        assert!(
            error < CENTRE_ERROR,
            "value_noise at the centre of cell {c:?}: {error}"
        );
    }
}

/// `value_noise` is what the figure reads, so the cell it takes is what's checked, not one a probe worked out.
#[test]
fn qa_m653_value_noise_reads_the_cell_floor_gives() {
    let canvas = MockCanvas::new().expect("a GPU adapter for the mock's canvas");
    assert_value_noise(&canvas, &noise_source());

    // Truncation toward 0 is floor only in the positive half-plane.
    let truncated = changed("let c = vec2<i32>(whole);", "let c = vec2<i32>(p);");
    parsed(&truncated);
    rejects("a cell truncated toward 0", || {
        assert_value_noise(&canvas, &truncated)
    });
    let crossed = changed("mix(k.x, k.y, u.x)", "mix(k.y, k.x, u.x)");
    parsed(&crossed);
    rejects("a blend reading its corners crossed", || {
        assert_value_noise(&canvas, &crossed)
    });
}

/// `source` parsed and validated.
fn parsed(source: &str) -> (Module, ModuleInfo) {
    let module = naga::front::wgsl::parse_str(source).expect("the source parses");
    let info = Validator::new(ValidationFlags::all(), Capabilities::all())
        .validate(&module)
        .expect("the source validates");
    (module, info)
}

/// The function called `name`.
fn named(module: &Module, name: &str) -> Handle<Function> {
    module
        .functions
        .iter()
        .find(|(_, f)| f.name.as_deref() == Some(name))
        .unwrap_or_else(|| panic!("no fn {name}"))
        .0
}

/// The names of what `handle` calls.
fn callees(module: &Module, handle: Handle<Function>) -> Vec<String> {
    module.functions[handle]
        .expressions
        .iter()
        .filter_map(|(_, e)| match *e {
            Expression::CallResult(f) => module.functions[f].name.clone(),
            _ => None,
        })
        .collect()
}

/// Whether `cell_corners`' own body, its offsets, moves any float: no arithmetic and no conversion between its input
/// and its calls to `corner`, whose exactness `lattice_source_is_integer` reads.
fn holds_float_step(module: &Module, info: &ModuleInfo, handle: Handle<Function>) -> bool {
    let function = &module.functions[handle];
    let of = &info[handle];
    let float = |e: Handle<Expression>| {
        of[e].ty.inner_with(&module.types).scalar_kind() == Some(ScalarKind::Float)
    };
    function.expressions.iter().any(|(_, e)| match *e {
        Expression::Binary { left, right, .. } => float(left) || float(right),
        Expression::Unary { expr, .. } | Expression::Math { arg: expr, .. } => float(expr),
        Expression::As { expr, kind, .. } => float(expr) || kind == ScalarKind::Float,
        _ => false,
    })
}

/// Asserts `cell_corners`' offsets are integer additions, on `source`.
fn assert_integer_offsets(source: &str) {
    let (module, info) = parsed(source);
    let handle = named(&module, "cell_corners");
    assert!(
        !holds_float_step(&module, &info, handle),
        "a float step in cell_corners"
    );
    assert_eq!(callees(&module, handle), ["corner"; 4], "cell_corners");
}

/// Asserts `value_noise` takes its corners from `cell_corners` alone and its `f` with no `fract`, on `source`.
fn assert_one_floor(source: &str) {
    let (module, _) = parsed(source);
    let handle = named(&module, "value_noise");
    assert_eq!(callees(&module, handle), ["cell_corners"], "value_noise");
    let fract = module.functions[handle].expressions.iter().any(|(_, e)| {
        matches!(
            e,
            Expression::Math {
                fun: MathFunction::Fract,
                ..
            }
        )
    });
    assert!(!fract, "value_noise's f comes from a fract");
}

/// R-689's cause, removed "by construction": the offset never reaches float arithmetic, which ADR-0001 says of the
/// `+1`. `lattice_source_is_integer` reads `lattice_hash` and `corner`; this reads the two functions on either side.
#[test]
fn qa_m653_the_offset_and_the_cell_stay_integer() {
    let source = noise_source();
    assert_integer_offsets(&source);
    assert_one_floor(&source);

    // ADR-0001's option A1, the `+1` still in float.
    let float_offset = changed(
        "corner(c + vec2<i32>(1, 0))",
        "corner(vec2<i32>(vec2<f32>(c) + vec2<f32>(1.0, 0.0)))",
    );
    parsed(&float_offset);
    rejects("the offset in float", || {
        assert_integer_offsets(&float_offset)
    });
    let fract = changed("let f = p - whole;", "let f = fract(p);");
    parsed(&fract);
    rejects("an f from a second route", || assert_one_floor(&fract));
    let second_read = changed("let k = cell_corners(c);", "let k = vec4<f32>(corner(c));");
    parsed(&second_read);
    rejects("a corner read outside cell_corners", || {
        assert_one_floor(&second_read)
    });
}

/// The stand-in's `n`, `warp.x` and `warp.y` at centre `[cx, cy]` and half-span `half`, tilts 0, on a square grid of the
/// field's `[-1, 1]²`.
fn stand_in_of(canvas: &MockCanvas, source: &str, [cx, cy]: [f32; 2], half: f32) -> Vec<[u32; 4]> {
    const GRID: u16 = 200;
    let chart = [
        [cx, cy, 0.0, 0.0],
        [half, 0.0, 0.0, 0.0],
        [0.0, half, 0.0, 0.0],
    ];
    let bytes = uniform_bytes(chart, Placement::IDENTITY, NO_INVALID, [0; 3]);
    let along = |i: u16| 2.0 * f32::from(i) / f32::from(GRID - 1) - 1.0;
    let input: Vec<[u32; 2]> = (0..GRID)
        .flat_map(|y| (0..GRID).map(move |x| [x, y]))
        .map(|[x, y]| [along(x).to_bits(), along(y).to_bits()])
        .collect();
    probe(canvas, source, "qa_stand_in", &input, Some(&bytes))
}

/// Asserts each of the field's three values is a number in `[0, 1]`: a blend of corners in `[0, 1)` by weights summing
/// to 1 or less can be nothing else. The jump-line check can't say this: a NaN compares false, so a field of NaN holds no
/// line and passes, the hole `raw_difference`'s `non_finite` closed in the report (REQ-GUI-418's detector).
fn assert_numbers(fields: &[[u32; 4]]) {
    for [n, x, y, _] in fields {
        for bits in [n, x, y] {
            let value = f32::from_bits(*bits);
            assert!((0.0..=1.0).contains(&value), "a field value of {value}");
        }
    }
}

#[test]
fn qa_m653_the_field_the_jump_check_reads_is_numbers() {
    let canvas = MockCanvas::new().expect("a GPU adapter for the mock's canvas");
    // REQ-GUI-412's four views.
    let views = [
        ([0.0, 0.0], 1.0 / 0.77),
        ([0.18, 0.41], 1.0),
        ([-40.0, -30.0], 1.0),
        ([4.0e4, 4.0e4], 1.0),
    ];
    for (centre, half) in views {
        assert_numbers(&stand_in_of(&canvas, &noise_source(), centre, half));
    }

    // All the exponent's bits set, with no float operation for a compiler to fold: an infinity or a NaN, whichever `n`
    // holds in its mantissa.
    let broken = changed(
        "return StandIn(n, fract(n * 4.0), warp);",
        "return StandIn(bitcast<f32>(bitcast<u32>(n) | 0x7f800000u), fract(n * 4.0), warp);",
    );
    parsed(&broken);
    let (centre, half) = views[0];
    let control = stand_in_of(&canvas, &broken, centre, half);
    rejects("a field of no numbers", || assert_numbers(&control));
}
