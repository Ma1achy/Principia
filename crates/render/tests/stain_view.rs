//! The stain view (`render::stain_view`): a field's stored records drawn through a stain graph, on the GPU harness's
//! device, over the synthetic payload harness's records (`engine::synthetic`). The pass criteria, fixed before the
//! first run:
//! - With the canonical `outcome_state` stain over a synthetic field covering every `state` code and every `detail`,
//!   the picture equals `render::colour::outcome`'s CPU palette texel for texel: every 8-bit code exactly, on an
//!   sRGB-typed target and on the GUI's plain `Rgba8Unorm` one, a `decode_failed` sample the canvas background
//!   (`stain_view_outcome_equals_cpu_palette`). Equality is the outcome tests' (`outcome_state.rs`'s `Want::Srgb`): a
//!   swatch's pixels encode back to its 8-bit code exactly, no tolerance.
//! - Row order and placement agree with the GUI's real field (`gui/src/real/field.wgsl`), bottom row first: at a
//!   placement that is not the identity, the view's picture equals, byte for byte, `field.wgsl`'s drawing of the CPU
//!   palette's image of the same field (`stain_view_placement_agrees_with_gui_field`).
//! - Editing one parameter compiles nothing and makes no bind group (both counted): only that class's pixels change, to
//!   the edited colour (`stain_view_param_edit_compiles_nothing`); a graph edit back to a compiled stain hits the cache
//!   and compiles nothing (`stain_view_graph_edit_back_compiles_nothing`).
//! - A broken node leaves the last picture, byte for byte, and the error names the node, by its position and key
//!   (`stain_view_broken_node_keeps_last_picture`).
//!
//! Each test registers its negative control (R-176), which breaks its property and shows the check fails.

use render::colour::outcome::{self, Shown};
use render::headless::{read_back, Image, Target};
use render::pipeline_cache::{CompileError, FrameInputs, NodeKey, Requested};
use render::present;
use render::raster::Grid;
use render::stain_view::{Placement, Records, StainView};
use validation::gpu::GpuHarness;
use validation::negative_control;
use validation::stain::{NodeId, NodeKind, Occupant, StainGraph};
use validation::synthetic::{Bytes, Synthetic};

use ledger::gen::read::Tier;
use render::assemble::{self, Stain};

/// The field: 8 samples across, 4 up. Not square, and each row's details differ, so a row or axis swap shows.
const W: u32 = 8;
const H: u32 = 4;

/// Each sample's side in the picture, pixels, at the identity placement.
const PX: u32 = 3;

/// The read side's uniforms: E = 0, equal masses, fixed read params (the outcome palette reads none of them).
const INPUTS: FrameInputs = FrameInputs {
    ensemble: 0,
    ensemble_spread: 0.0,
    masses: [1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0],
    dt_macro: 0.01,
    delta_0: 1e-6,
    n_renorm: 16,
    horizon_steps: 1000,
};

/// Sample `i`'s `(state, detail)`: `state = i mod 8`, every code 0 to 7, the reserved 6 and 7 among them;
/// `detail = (i div 8 + i) mod 4`, so each state takes every detail once over the four rows.
fn state_detail(i: u32) -> (u32, u32) {
    (i % 8, (i / 8 + i) % 4)
}

/// The synthetic field's records, `W × H` samples in the view's order, bottom row first: sample `i` holds
/// [`state_detail`]`(i)`, stepped 40 macro-steps. With `top_first`, the control's, the rows are given top row first.
fn records(top_first: bool) -> Bytes {
    let n = W * H;
    let grid = Grid::new([n, 1], 1, 0, 1).unwrap_or_else(|e| panic!("{e}"));
    let mut set = Synthetic::flat(grid, 0);
    for i in 0..n {
        let source = if top_first {
            (H - 1 - i / W) * W + i % W
        } else {
            i
        };
        let (state, detail) = state_detail(source);
        set.sample(i).state(state).detail(detail).times(40, 0);
    }
    set.bytes()
}

/// The engine's stain graph of the outcome palette: a source into the built-in colour `outcome_state`, into the
/// backbone's pass-through combiner and OUT, and the colour node's id.
fn outcome_graph() -> (StainGraph, NodeId) {
    let mut g = StainGraph::new();
    let source = g
        .add(NodeKind::Source, Occupant::Field("ftle".into()))
        .unwrap_or_else(|e| panic!("{e}"));
    let colour = g
        .add(NodeKind::Colour, Occupant::Builtin(outcome::ID.into()))
        .unwrap_or_else(|e| panic!("{e}"));
    g.connect(source, colour, 0)
        .unwrap_or_else(|e| panic!("{e}"));
    g.connect(colour, NodeId(0), 0)
        .unwrap_or_else(|e| panic!("{e}"));
    (g, colour)
}

/// The canonical form's stain of `g` (`StainGraph::canonical`), what the view draws.
fn canonical(g: &StainGraph) -> Stain {
    g.canonical()
        .unwrap_or_else(|e| panic!("{e}"))
        .stain()
        .clone()
}

/// Node keys for `stain`'s nodes: 100 + position, so a key is never its node's position.
fn keys(stain: &Stain) -> Vec<NodeKey> {
    (0..stain.nodes().len() as u64).map(|k| 100 + k).collect()
}

/// The position of the colour node in `stain`.
fn colour_position(stain: &Stain) -> usize {
    stain
        .nodes()
        .iter()
        .position(|n| n.kind == assemble::Kind::Colour)
        .expect("a colour node")
}

fn gpu() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

/// A view of the field `bytes` into `format` targets, the canonical outcome stain set and compiled.
fn view(h: &GpuHarness, format: wgpu::TextureFormat, bytes: &Bytes) -> (StainView, Stain) {
    let records = Records {
        simstate: &bytes.simstate,
        word: &bytes.word,
        ic: &bytes.ic,
    };
    let mut v = StainView::new(
        h.device(),
        h.queue(),
        format,
        [W, H],
        records,
        Tier::FULL,
        INPUTS,
    )
    .unwrap_or_else(|e| panic!("{e}"));
    let (g, _) = outcome_graph();
    let stain = canonical(&g);
    v.set_stain(&stain, &keys(&stain))
        .unwrap_or_else(|e| panic!("{e}"));
    v.wait();
    assert!(
        v.errors().is_empty(),
        "the outcome stain failed: {:?}",
        v.errors()
    );
    (v, stain)
}

/// What sample `i` shows: its swatch's 8-bit sRGB code, or the canvas background for a `decode_failed` sample, by the
/// CPU palette (`outcome::shown`; no sample is forbidden: the view's pass classifies none).
fn cpu_palette(i: u32, swatches: &[[u8; 3]]) -> [u8; 3] {
    let (state, detail) = state_detail(i);
    match outcome::shown(state, detail, false) {
        Shown::Swatch(k) => swatches[k],
        Shown::Forbidden | Shown::DecodeFailed => present::CANVAS_BACKGROUND_SRGB8,
    }
}

/// The sample pixel `(x, y)` of a `W·PX × H·PX` picture at the identity placement shows, `y` from the top: the field's
/// rows bottom first, so the picture's top row is the field's last.
fn sample_at(x: u32, y: u32) -> u32 {
    let row = (H * PX - 1 - y) / PX;
    row * W + x / PX
}

/// The texels of `image` that differ from `want(x, y)`, as `(x, y, got, want)`.
fn differing(
    image: &Image,
    want: impl Fn(u32, u32) -> [u8; 3],
) -> Vec<(u32, u32, Vec<u8>, [u8; 3])> {
    let mut out = Vec::new();
    for y in 0..image.height {
        for x in 0..image.width {
            let got = image.texel(x, y);
            let w = want(x, y);
            if got[..3] != w || got[3] != 255 {
                out.push((x, y, got.to_vec(), w));
            }
        }
    }
    out
}

// ── The outcome palette, texel for texel ─────────────────────────────────────────────────────────────────────────

/// The canonical outcome stain over the field, at the identity placement, on an sRGB-typed and a plain 8-bit target,
/// equals the CPU palette at every pixel. `top_first`, the control, gives the records top row first.
fn check_palette(top_first: bool) {
    let h = gpu();
    let bytes = records(top_first);
    let swatches = outcome::defaults();
    for format in [
        wgpu::TextureFormat::Rgba8UnormSrgb,
        wgpu::TextureFormat::Rgba8Unorm,
    ] {
        let (mut v, _) = view(&h, format, &bytes);
        let image = v.render(W * PX, H * PX).unwrap_or_else(|e| panic!("{e}"));
        let bad = differing(&image, |x, y| cpu_palette(sample_at(x, y), &swatches));
        println!(
            "{format:?}: {} of {} pixels differ from the CPU palette",
            bad.len(),
            image.width * image.height
        );
        assert!(
            bad.is_empty(),
            "{format:?}: the picture differs from the CPU palette at {} pixels, first {:?}",
            bad.len(),
            &bad[..bad.len().min(4)]
        );
    }
}

#[test]
fn stain_view_outcome_equals_cpu_palette() {
    check_palette(false);
}

negative_control!(
    stain_view_outcome_equals_cpu_palette,
    "the records given top row first: the picture is the field upside down",
    expected = "the picture differs from the CPU palette",
    check_palette(true)
);

// ── Row order and placement, against the GUI's field ──────────────────────────────────────────────────────────────

/// The GUI's real field shader, `gui/src/real/field.wgsl`, as the GUI draws its outcome image.
const GUI_FIELD_WGSL: &str = include_str!("../../gui/src/real/field.wgsl");

/// The CPU palette's image of the field as the GUI uploads it (`engine::field::Field::rgba8`): rows from the top, each
/// sample its swatch, alpha 255, a `decode_failed` sample transparent black.
fn gui_image() -> Vec<u8> {
    let swatches = outcome::defaults();
    let mut out = Vec::new();
    for row in (0..H).rev() {
        for x in 0..W {
            let i = row * W + x;
            let (state, detail) = state_detail(i);
            match outcome::shown(state, detail, false) {
                Shown::Swatch(k) => out.extend(swatches[k].into_iter().chain([255])),
                _ => out.extend([0, 0, 0, 0]),
            }
        }
    }
    out
}

/// `field.wgsl`'s drawing of [`gui_image`] at `placement` into a `width × height` `Rgba8Unorm` target, with the GUI's
/// uniform (`gui::real::uniform_bytes`: the placement, then the background over 255, alpha 1).
fn gui_render(h: &GpuHarness, placement: Placement, width: u32, height: u32) -> Image {
    use wgpu::util::DeviceExt;
    let device = h.device();
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let extent = wgpu::Extent3d {
        width: W,
        height: H,
        depth_or_array_layers: 1,
    };
    let image = device.create_texture_with_data(
        h.queue(),
        &wgpu::TextureDescriptor {
            label: Some("gui field"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        },
        wgpu::util::TextureDataOrder::LayerMajor,
        &gui_image(),
    );
    let [r, g, b] = present::CANVAS_BACKGROUND_SRGB8.map(|c| f32::from(c) / 255.0);
    let [cx, cy] = placement.centre;
    let [hx, hy] = placement.half;
    let uniform: Vec<u8> = [cx, cy, hx, hy, r, g, b, 1.0]
        .into_iter()
        .flat_map(f32::to_ne_bytes)
        .collect();
    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("gui field"),
        contents: &uniform,
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("gui field"),
        source: wgpu::ShaderSource::Wgsl(GUI_FIELD_WGSL.into()),
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("gui field"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &[Some(format.into())],
        }),
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    });
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("gui field"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(
                    &image.create_view(&Default::default()),
                ),
            },
        ],
    });
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("gui target"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("gui field"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.draw(0..3, 0..1);
    }
    h.queue().submit([encoder.finish()]);
    read_back(
        device,
        h.queue(),
        &target,
        Target {
            width,
            height,
            format,
        },
    )
    .unwrap_or_else(|e| panic!("{e}"))
}

/// The placement both draw at: off centre, not square, part of the field past the viewport's right edge.
const PLACED: Placement = Placement {
    centre: [0.31, -0.17],
    half: [0.83, 0.61],
};

/// The view, on the GUI's `Rgba8Unorm` target, at [`PLACED`], equals `field.wgsl`'s drawing of the CPU palette's image
/// of the same field at [`PLACED`], every byte of a 53 × 37 target. `shift`, the control, moves the view's placement by
/// one target pixel across.
fn check_gui(shift: bool) {
    const TW: u32 = 53;
    const TH: u32 = 37;
    let h = gpu();
    let bytes = records(false);
    let (mut v, _) = view(&h, wgpu::TextureFormat::Rgba8Unorm, &bytes);
    let mut placement = PLACED;
    if shift {
        placement.centre[0] += 2.0 / TW as f32;
    }
    v.set_placement(placement);
    let ours = v.render(TW, TH).unwrap_or_else(|e| panic!("{e}"));
    let gui = gui_render(&h, PLACED, TW, TH);
    let bad = differing(&ours, |x, y| {
        let t = gui.texel(x, y);
        [t[0], t[1], t[2]]
    });
    let background = present::CANVAS_BACKGROUND_SRGB8;
    let field = (0..TH)
        .flat_map(|y| (0..TW).map(move |x| (x, y)))
        .filter(|&(x, y)| gui.texel(x, y)[..3] != background)
        .count();
    println!(
        "{} of {} pixels differ from the GUI's field; {field} pixels of the GUI's are not the background",
        bad.len(),
        TW * TH
    );
    assert!(field > 0, "the GUI's field drew nothing at the placement");
    assert!(
        bad.is_empty(),
        "the view differs from the GUI's field at {} pixels, first {:?}",
        bad.len(),
        &bad[..bad.len().min(4)]
    );
}

#[test]
fn stain_view_placement_agrees_with_gui_field() {
    check_gui(false);
}

negative_control!(
    stain_view_placement_agrees_with_gui_field,
    "the view's placement moved one target pixel across",
    expected = "the view differs from the GUI's field",
    check_gui(true)
);

// ── A param edit compiles nothing ─────────────────────────────────────────────────────────────────────────────────

/// The swatch edited, and its edit, 8-bit sRGB: escape body 2's, to olive.
const EDIT: (&str, [u8; 3]) = ("escape_body_1", [0x80, 0x80, 0x20]);

/// Edits the swatch [`EDIT`] names: through its param (`set_param`, the property), or, `via_graph`, the control, as a
/// graph edit, the colour node's occupant replaced by `outcome_state`'s WGSL as custom text with that swatch's default
/// edited. The compile and bind counts must not move, and only that class's pixels change, to the edit.
fn check_param_edit(via_graph: bool) {
    let h = gpu();
    let bytes = records(false);
    let (mut v, stain) = view(&h, wgpu::TextureFormat::Rgba8UnormSrgb, &bytes);
    let before = v.render(W * PX, H * PX).unwrap_or_else(|e| panic!("{e}"));
    let (compiles, binds) = (v.compiles(), v.binds());
    let k = colour_position(&stain);
    let key = keys(&stain)[k];
    let linear = present::srgb8(EDIT.1).to_vec();
    if via_graph {
        let mut nodes = stain.nodes().to_vec();
        nodes[k].occupant =
            assemble::Occupant::Custom(outcome::WGSL.replace("escape_body_1", "escape_body_1_x"));
        let edited = Stain::new(nodes).unwrap_or_else(|e| panic!("{e}"));
        v.set_stain(&edited, &keys(&stain))
            .unwrap_or_else(|e| panic!("{e}"));
        v.wait();
    } else {
        v.set_param(key, EDIT.0, &linear)
            .unwrap_or_else(|e| panic!("{e}"));
    }
    let after = v.render(W * PX, H * PX).unwrap_or_else(|e| panic!("{e}"));
    println!(
        "compiles {compiles} → {}, binds {binds} → {}",
        v.compiles(),
        v.binds()
    );
    assert_eq!(v.compiles(), compiles, "a param edit compiled");
    assert_eq!(v.binds(), binds, "a param edit made bind groups");
    let mut swatches = outcome::defaults();
    swatches[outcome::index(EDIT.0).expect("the swatch")] = EDIT.1;
    let bad = differing(&after, |x, y| cpu_palette(sample_at(x, y), &swatches));
    assert!(bad.is_empty(), "the edited picture differs at {bad:?}");
    let changed = (0..after.height)
        .flat_map(|y| (0..after.width).map(move |x| (x, y)))
        .filter(|&(x, y)| after.texel(x, y) != before.texel(x, y))
        .count();
    let class = (0..W * H)
        .filter(|&i| {
            let (s, d) = state_detail(i);
            outcome::shown(s, d, false) == Shown::Swatch(outcome::index(EDIT.0).unwrap())
        })
        .count() as u32;
    println!(
        "{changed} pixels changed; the edited class covers {} pixels",
        class * PX * PX
    );
    assert_eq!(
        changed as u32,
        class * PX * PX,
        "pixels outside the class changed"
    );
}

#[test]
fn stain_view_param_edit_compiles_nothing() {
    check_param_edit(false);
}

negative_control!(
    stain_view_param_edit_compiles_nothing,
    "the edit made as a graph edit, the occupant's text changed: a new fragment key, compiled",
    expected = "a param edit compiled",
    check_param_edit(true)
);

/// A graph edit to another stain and back: the way back is a cache hit, the picture the first one's, and the compile
/// count moves by the one new stain only. `trailing`, the control, comes back with a comment appended to the colour's
/// text: a new fragment key, compiled.
fn check_graph_back(trailing: bool) {
    let h = gpu();
    let bytes = records(false);
    let (mut v, stain) = view(&h, wgpu::TextureFormat::Rgba8UnormSrgb, &bytes);
    let first = v.render(W * PX, H * PX).unwrap_or_else(|e| panic!("{e}"));
    let k = colour_position(&stain);
    let keys = keys(&stain);
    let with = |occupant: assemble::Occupant| {
        let mut nodes = stain.nodes().to_vec();
        nodes[k].occupant = occupant;
        Stain::new(nodes).unwrap_or_else(|e| panic!("{e}"))
    };
    let grey = with(assemble::Occupant::Custom(
        "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(0.25); }".into(),
    ));
    v.set_stain(&grey, &keys).unwrap_or_else(|e| panic!("{e}"));
    v.wait();
    let compiles = v.compiles();
    let back = if trailing {
        with(assemble::Occupant::Custom(format!(
            "{}\n// back\n",
            outcome::WGSL
        )))
    } else {
        stain.clone()
    };
    let requested = v.set_stain(&back, &keys).unwrap_or_else(|e| panic!("{e}"));
    v.wait();
    let again = v.render(W * PX, H * PX).unwrap_or_else(|e| panic!("{e}"));
    println!(
        "back: {requested:?}, compiles {compiles} → {}",
        v.compiles()
    );
    assert_eq!(v.compiles(), compiles, "the way back compiled");
    assert_eq!(requested, Requested::Hit);
    assert_eq!(again, first, "the way back's picture is not the first's");
}

#[test]
fn stain_view_graph_edit_back_compiles_nothing() {
    check_graph_back(false);
}

negative_control!(
    stain_view_graph_edit_back_compiles_nothing,
    "the way back with a comment appended to the colour's text: a new fragment key",
    expected = "the way back compiled",
    check_graph_back(true)
);

// ── A broken node keeps the last picture ──────────────────────────────────────────────────────────────────────────

/// A colour occupant with a syntax error.
const BROKEN: &str = "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(0.5 0.5, 0.5); }";

/// The outcome stain drawn, then its colour node, under the same key, given [`BROKEN`]: the picture stays the last
/// one, byte for byte, and the error is a node's, naming the colour node's position and key. `valid`, the control,
/// gives it a valid flat colour instead, which changes the picture.
fn check_broken(valid: bool) {
    let h = gpu();
    let bytes = records(false);
    let (mut v, stain) = view(&h, wgpu::TextureFormat::Rgba8UnormSrgb, &bytes);
    let last = v.render(W * PX, H * PX).unwrap_or_else(|e| panic!("{e}"));
    let k = colour_position(&stain);
    let keys = keys(&stain);
    let mut nodes = stain.nodes().to_vec();
    nodes[k].occupant = assemble::Occupant::Custom(if valid {
        "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(0.5, 0.5, 0.5); }".into()
    } else {
        BROKEN.into()
    });
    let broken = Stain::new(nodes).unwrap_or_else(|e| panic!("{e}"));
    v.set_stain(&broken, &keys)
        .unwrap_or_else(|e| panic!("{e}"));
    v.wait();
    let now = v.render(W * PX, H * PX).unwrap_or_else(|e| panic!("{e}"));
    let changed = (0..now.height)
        .flat_map(|y| (0..now.width).map(move |x| (x, y)))
        .filter(|&(x, y)| now.texel(x, y) != last.texel(x, y))
        .count();
    let errors = v.errors();
    println!("{changed} pixels changed; errors: {errors:?}");
    assert_eq!(changed, 0, "the picture changed");
    match errors.as_slice() {
        [CompileError::Node { node, key, .. }] => {
            assert_eq!((*node, *key), (k, keys[k]), "the error names another node");
            let text = errors[0].to_string();
            assert!(
                text.contains(&format!("node {k}")) && text.contains(&keys[k].to_string()),
                "the message `{text}` does not name the node"
            );
        }
        other => panic!("not one node's error: {other:?}"),
    }
}

#[test]
fn stain_view_broken_node_keeps_last_picture() {
    check_broken(false);
}

negative_control!(
    stain_view_broken_node_keeps_last_picture,
    "a valid flat colour in the node instead: the picture changes",
    expected = "the picture changed",
    check_broken(true)
);
