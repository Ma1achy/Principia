//! QA's tests for TASK-M7-03, written from the requirements (colour_composition §1, §5, §8):
//! - REQ-GEN-022: the occupant tree compiles to deterministic, readable WGSL: one named function per node, its identity
//!   stable across recompiles; a comment carrying the node's name and parameter values; parameters bound as uniforms,
//!   so a slider edit rebinds, never recompiles; calls into the one shared WGSL library. Verify: the same graph twice
//!   gives byte-identical WGSL; a slider change triggers no recompile.
//! - REQ-COL-031: map parameters use the adopted ranges L ∈ [0.35, 0.90], C ∈ [0.05, 0.22], κ ∈ [0.5, 12],
//!   f ∈ [2, 14], N ∈ [12, 96], ks ∈ [1, 20], s ∈ [0, 1]. Verify: the schema clamps each to its range.
//!
//! The ranges below are transcribed from colour_composition §8, not read from the schema. Each test has its negative
//! control (R-176).

use render::assemble::{self, Kind, Node as StainNode, Occupant, Stain, Tier, UniformType};
use render::codegen::schema::MapParam;
use render::codegen::{generate, Arg, Call, Generated, Node, Param, Slot, ValueType};
use render::compositor::LAYER_FORMAT;
use render::frame_record::{Applied, RenderLoop};
use render::pipeline_cache::FrameInputs;
use render::present::{linear_to_srgb, ramp_grey};
use validation::gpu::GpuHarness;
use validation::negative_control;

// ----- REQ-COL-031: the adopted ranges, from colour_composition §8 -----

/// colour_composition §8's adopted ranges, in its order.
const ADOPTED: [(MapParam, f64, f64); 7] = [
    (MapParam::L, 0.35, 0.90),
    (MapParam::C, 0.05, 0.22),
    (MapParam::Kappa, 0.5, 12.0),
    (MapParam::F, 2.0, 14.0),
    (MapParam::N, 12.0, 96.0),
    (MapParam::Ks, 1.0, 20.0),
    (MapParam::S, 0.0, 1.0),
];

/// Every parameter of `table` clamps below, above, at and inside its range to the range: below to `lo`, above to
/// `hi`, the ends and the midpoint to themselves, ±∞ to the ends; NaN is no value.
fn check_clamps(table: &[(MapParam, f64, f64)]) {
    for &(p, lo, hi) in table {
        let mid = 0.5 * (lo + hi);
        let span = hi - lo;
        let cases = [
            (lo - span, lo),
            (lo - 1e-9, lo),
            (lo, lo),
            (mid, mid),
            (hi, hi),
            (hi + 1e-9, hi),
            (hi + span, hi),
            (-1e300, lo),
            (1e300, hi),
            (f64::NEG_INFINITY, lo),
            (f64::INFINITY, hi),
        ];
        for (raw, want) in cases {
            assert_eq!(
                p.clamp(raw),
                Some(want),
                "{p:?}: {raw} does not clamp to {want} in [{lo}, {hi}]"
            );
        }
        assert_eq!(p.clamp(f64::NAN), None, "{p:?}: NaN clamped to a number");
    }
}

#[test]
fn qa_param_schema_ranges_clamp_to_adopted() {
    check_clamps(&ADOPTED);
}

negative_control!(
    qa_param_schema_ranges_clamp_to_adopted,
    "a range off the adopted one (L's upper end 0.95) is caught",
    expected = "does not clamp to",
    check_clamps(&[(MapParam::L, 0.35, 0.95)])
);

/// The schema covers each of §8's seven parameters once, and each ranged uniform it declares carries its adopted
/// range and refuses a value outside it.
fn check_uniform_ranges(table: &[(MapParam, f64, f64)]) {
    assert_eq!(MapParam::ALL.len(), 7, "§8 adopts seven ranges");
    for &(p, lo, hi) in table {
        assert!(MapParam::ALL.contains(&p), "{p:?} is not in the schema");
        let u = p.uniform("x", hi + 1.0).expect("a number");
        assert_eq!(u.ty, UniformType::F32);
        assert_eq!(u.range, Some((lo, hi)), "{p:?}'s uniform range");
        assert_eq!(u.default, vec![hi], "{p:?}'s default was not clamped");
        assert!(u.admits(&[lo]) && u.admits(&[hi]), "{p:?}: an end refused");
        assert!(
            !u.admits(&[lo - 1e-6]) && !u.admits(&[hi + 1e-6]),
            "{p:?}'s uniform admits a value outside [{lo}, {hi}]"
        );
        assert!(p.uniform("x", f64::NAN).is_none(), "{p:?}: NaN declared");
    }
}

#[test]
fn qa_param_schema_ranges_reach_the_uniform() {
    check_uniform_ranges(&ADOPTED);
}

negative_control!(
    qa_param_schema_ranges_reach_the_uniform,
    "a range off the adopted one (κ from 1) is caught",
    expected = "uniform range",
    check_uniform_ranges(&[(MapParam::Kappa, 1.0, 12.0)])
);

// ----- REQ-GEN-022: the tests' trees, each node a call into the shared library -----

fn call(name: &str, function: &str, args: Vec<Arg>, params: Vec<Param>) -> Call {
    Call {
        name: name.into(),
        function: function.into(),
        args,
        params,
    }
}

/// An `f32` leaf holding the lightness `l` as the map parameter L.
fn light(id: u32, l: f64) -> Node {
    Node {
        id,
        output: ValueType::F32,
        call: call(
            "lightness",
            "f32",
            vec![Arg::Param(0)],
            vec![Param::ranged("L", MapParam::L, l)],
        ),
        inputs: vec![],
    }
}

/// `f32 → vec3`: the shared library's grey ramp.
fn grey(id: u32, t: Node) -> Node {
    Node {
        id,
        output: ValueType::Vec3,
        call: call("grey ramp", "ramp_grey", vec![Arg::Input(0)], vec![]),
        inputs: vec![t],
    }
}

/// A `vec3` leaf: the shared library's OKLCH at L, C and a hue.
fn swatch(id: u32, l: f64, c: f64, hue: f64) -> Node {
    Node {
        id,
        output: ValueType::Vec3,
        call: call(
            "swatch",
            "oklch_to_linear",
            vec![Arg::Param(0), Arg::Param(1), Arg::Param(2)],
            vec![
                Param::ranged("L", MapParam::L, l),
                Param::ranged("C", MapParam::C, c),
                Param::free("hue", UniformType::F32, vec![hue]),
            ],
        ),
        inputs: vec![],
    }
}

/// `vec3, vec3 → vec3`: a blend at strength s.
fn blend(id: u32, a: Node, b: Node, s: f64) -> Node {
    Node {
        id,
        output: ValueType::Vec3,
        call: call(
            "blend",
            "mix",
            vec![Arg::Input(0), Arg::Input(1), Arg::Param(0)],
            vec![Param::ranged("s", MapParam::S, s)],
        ),
        inputs: vec![a, b],
    }
}

/// Seven nodes, three deep: blend(blend(swatch 10, grey 11(light 12)), grey 20(light 21), …) at node 30.
struct Values {
    l_a: f64,
    l_b: f64,
    l_c: f64,
    c: f64,
    s_inner: f64,
    s_outer: f64,
}

const BASE: Values = Values {
    l_a: 0.62,
    l_b: 0.41,
    l_c: 0.77,
    c: 0.13,
    s_inner: 0.3,
    s_outer: 0.7,
};

fn big(v: &Values) -> Node {
    blend(
        30,
        blend(
            25,
            swatch(10, v.l_a, v.c, 0.125),
            grey(11, light(12, v.l_b)),
            v.s_inner,
        ),
        grey(20, light(21, v.l_c)),
        v.s_outer,
    )
}

const BIG_IDS: [u32; 7] = [10, 11, 12, 20, 21, 25, 30];

fn gen(root: &Node) -> Generated {
    generate(Slot::Colour, root).unwrap_or_else(|e| panic!("did not generate: {e}"))
}

/// The four-node stain: the `ftle` source, the colour occupant `colour`, the pass-through combiner and OUT.
fn stain(colour: &str) -> Stain {
    let n = |kind, occupant, inputs: &[Option<usize>]| StainNode {
        kind,
        occupant,
        inputs: inputs.to_vec(),
    };
    Stain::new(vec![
        n(Kind::Source, Occupant::Field("ftle".into()), &[]),
        n(Kind::Colour, Occupant::Custom(colour.into()), &[Some(0)]),
        n(
            Kind::Combiner,
            Occupant::BuiltIn("pass_through".into()),
            &[Some(1), None],
        ),
        n(Kind::Out, Occupant::None, &[Some(2)]),
    ])
    .unwrap_or_else(|e| panic!("{e}"))
}

/// The function definitions in `source`: each `fn <name>(` it defines.
fn defined(source: &str) -> Vec<String> {
    source
        .lines()
        .filter_map(|l| l.trim_start().strip_prefix("fn "))
        .map(|rest| rest.split('(').next().unwrap_or("").trim().to_owned())
        .collect()
}

/// The source with every comment line dropped: the code the GPU compiles.
fn code(source: &str) -> String {
    source
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

// ----- byte-identical across recompiles; one named function per node -----

/// Generating the tree `make` builds, three times from fresh values, gives one text; it has one function per node,
/// each defined once, plus the slot's; and the text compiles through the one assembler, whose naga validation
/// resolves every call into the shared library.
fn check_deterministic(make: &dyn Fn(usize) -> Node) {
    let texts: Vec<String> = (0..3).map(|k| gen(&make(k)).source).collect();
    assert!(
        texts.iter().all(|t| *t == texts[0]),
        "the same graph did not give byte-identical WGSL"
    );
    let g = gen(&make(0));
    let mut names = defined(&g.source);
    assert_eq!(
        names.len(),
        BIG_IDS.len() + 1,
        "not one function per node: {names:?}"
    );
    assert_eq!(
        names.pop().as_deref(),
        Some("colour"),
        "the slot's function is not last"
    );
    names.sort();
    names.dedup();
    assert_eq!(names.len(), BIG_IDS.len(), "a function is defined twice");
    let mut ids: Vec<u32> = g.functions.iter().map(|f| f.id).collect();
    ids.sort_unstable();
    assert_eq!(ids, BIG_IDS, "the functions are not the tree's nodes");
    assemble::assemble(&stain(&g.source), Tier::FULL)
        .unwrap_or_else(|e| panic!("the generated occupant did not assemble: {e}"));
}

#[test]
fn qa_codegen_deterministic_byte_identical() {
    check_deterministic(&|_| big(&BASE));
}

negative_control!(
    qa_codegen_deterministic_byte_identical,
    "a graph that differs between builds gives another text",
    expected = "byte-identical",
    check_deterministic(&|k| big(&Values {
        s_outer: if k == 1 { 0.5 } else { 0.7 },
        ..BASE
    }))
);

// ----- stable identity: an edit to one node leaves every other function's name -----

/// After `edit` changes node 12's lightness, every node keeps its function's name; each node off the path from 12 to
/// the root keeps its function's text; node 12's code (comments aside) is unchanged too, its value a uniform.
fn check_stable(edit: &dyn Fn(Node) -> Node) {
    let before = gen(&big(&BASE));
    let after = gen(&edit(big(&BASE)));
    let name_of = |g: &Generated, id: u32| {
        g.functions
            .iter()
            .find(|f| f.id == id)
            .map(|f| f.name.clone())
    };
    for id in BIG_IDS {
        let (b, a) = (name_of(&before, id), name_of(&after, id));
        assert!(
            b.is_some() && b == a,
            "node {id}'s function was renamed by an edit to node 12: {b:?} → {a:?}"
        );
        assert!(
            before.source.contains(&format!("fn {}(", b.unwrap())),
            "node {id}'s function is not in the text"
        );
    }
    for id in [10, 20, 21] {
        let text = |g: &Generated| {
            g.functions
                .iter()
                .find(|f| f.id == id)
                .map(|f| f.text.clone())
        };
        assert_eq!(
            text(&before),
            text(&after),
            "node {id}'s function changed on an unrelated edit"
        );
    }
}

#[test]
fn qa_codegen_deterministic_identity_stable_across_edit() {
    check_stable(&|_| big(&Values { l_b: 0.5, ..BASE }));
}

negative_control!(
    qa_codegen_deterministic_identity_stable_across_edit,
    "renumbering node 20 is caught",
    expected = "was renamed",
    check_stable(&|mut t| {
        t.inputs[1].id = 40;
        t
    })
);

// ----- comments carry the node's name and its parameter values -----

/// A node's id, its name and its parameters' names and values, as its comments must carry them.
type Expected = (u32, &'static str, &'static [(&'static str, &'static str)]);

/// Each node's function text holds a comment naming the node and giving each parameter's value: `values` lists, per
/// node id, its name and its parameters' names and values as the value clamped into its range reads.
fn check_comments(l_a: f64, values: &[Expected]) {
    let g = gen(&big(&Values { l_a, ..BASE }));
    for &(id, name, params) in values {
        let f = g.functions.iter().find(|f| f.id == id).expect("a function");
        let comments: Vec<&str> = f
            .text
            .lines()
            .filter(|l| l.trim_start().starts_with("//"))
            .collect();
        assert!(
            comments.iter().any(|c| c.contains(name)),
            "node {id}'s comments do not name it `{name}`: {comments:?}"
        );
        // A declared range, `[lo, hi]`, is not a value: it is cut before looking for one. The value is a whole
        // number token, so 0.9 is not found in 0.95.
        let carries = |c: &str, p: &str, v: &str| {
            let c = c.split('[').next().unwrap_or("");
            let tokens: Vec<&str> = c
                .split(|ch: char| !(ch.is_alphanumeric() || ch == '.' || ch == '_' || ch == '-'))
                .collect();
            tokens
                .iter()
                .any(|t| *t == p || t.ends_with(&format!("_{p}")))
                && tokens.contains(&v)
        };
        for &(p, v) in params {
            assert!(
                comments.iter().any(|c| carries(c, p, v)),
                "node {id}'s comments do not carry {p} = {v}: {comments:?}"
            );
        }
        assert!(
            f.text.contains(&format!("fn {}(", f.name)),
            "node {id}'s text is not its function"
        );
    }
}

/// The values the comments must carry; node 10's L is 0.95 raw, clamped to L's upper end 0.9.
const COMMENTS: &[Expected] = &[
    (
        10,
        "swatch",
        &[("L", "0.9"), ("C", "0.13"), ("hue", "0.125")],
    ),
    (11, "grey ramp", &[]),
    (12, "lightness", &[("L", "0.41")]),
    (25, "blend", &[("s", "0.3")]),
    (30, "blend", &[("s", "0.7")]),
];

#[test]
fn qa_codegen_deterministic_comments_name_and_values() {
    check_comments(0.95, COMMENTS);
}

negative_control!(
    qa_codegen_deterministic_comments_name_and_values,
    "a value the tree does not hold is not in the comment",
    expected = "do not carry",
    check_comments(0.6, COMMENTS)
);

// ----- parameters are uniforms: a value change leaves the code, only the declarations' values change -----

/// Two trees differing only in their parameter values have the same code once comments are dropped: the values
/// are bound as uniforms, not baked into the code.
fn check_values_not_baked(other: &Values) {
    let (a, b) = (gen(&big(&BASE)), gen(&big(other)));
    assert_ne!(
        a.source, b.source,
        "the declarations do not carry the values"
    );
    assert_eq!(
        code(&a.source),
        code(&b.source),
        "a parameter's value is baked into the code"
    );
}

#[test]
fn qa_codegen_deterministic_params_are_uniforms() {
    check_values_not_baked(&Values {
        l_a: 0.4,
        l_b: 0.8,
        l_c: 0.36,
        c: 0.2,
        s_inner: 0.0,
        s_outer: 1.0,
    });
}

/// A node whose value is written into its code, as a parameter must not be: it calls a library function named for
/// the literal.
#[cfg(feature = "controls")]
fn baked(id: u32, l: f64) -> Node {
    Node {
        id,
        output: ValueType::F32,
        call: call(
            "lightness",
            &format!("f32_{}", (l * 100.0) as u32),
            vec![],
            vec![],
        ),
        inputs: vec![],
    }
}

negative_control!(
    qa_codegen_deterministic_params_are_uniforms,
    "a value written into the code is caught",
    expected = "baked into the code",
    {
        let (a, b) = (gen(&grey(1, baked(0, 0.4))), gen(&grey(1, baked(0, 0.8))));
        assert_eq!(
            code(&a.source),
            code(&b.source),
            "a parameter's value is baked into the code"
        );
    }
);

// ----- a slider edit rebinds, never recompiles, and the new value draws -----

/// The 4 × 2 layer's frame inputs.
const INPUTS: FrameInputs = FrameInputs {
    ensemble: 0,
    ensemble_spread: 0.25,
    masses: [1.0, 1.0, 1.0],
    dt_macro: 0.01,
    delta_0: 1e-6,
    n_renorm: 16,
    horizon_steps: 1000,
};

fn simstate_size() -> usize {
    let s = ledger::payload::structs()
        .into_iter()
        .find(|s| s.name == "SimStateFTLE")
        .expect("the FTLE stored variant");
    ledger::gen::rust::offsets(&s).1 as usize
}

/// The target's pixels, row by row.
fn read(h: &GpuHarness, texture: &wgpu::Texture) -> Vec<[u8; 4]> {
    let row = 256u32;
    let staging = h.device().create_buffer(&wgpu::BufferDescriptor {
        label: Some("qa readback"),
        size: u64::from(row * 2),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = h.device().create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &staging,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(2),
            },
        },
        wgpu::Extent3d {
            width: 4,
            height: 2,
            depth_or_array_layers: 1,
        },
    );
    h.queue().submit([encoder.finish()]);
    let slice = staging.slice(..);
    slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
    h.device()
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("poll");
    let bytes = slice.get_mapped_range().expect("range").to_vec();
    let mut out = Vec::new();
    for y in 0..2usize {
        for x in 0..4usize {
            let at = y * row as usize + x * 4;
            out.push([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
        }
    }
    out
}

/// Every pixel is the grey ramp at `t`, through the CPU twin of the shared library's `ramp_grey`, within one 8-bit
/// step.
fn assert_grey(pixels: &[[u8; 4]], t: f64, what: &str) {
    let rgb = ramp_grey(t);
    let want: Vec<u8> = rgb
        .iter()
        .map(|&v| (linear_to_srgb(v) * 255.0).round() as u8)
        .collect();
    for (i, p) in pixels.iter().enumerate() {
        assert!(
            p[..3].iter().zip(&want).all(|(&g, &w)| g.abs_diff(w) <= 1),
            "{what}: pixel {i} is {p:?}, not the grey ramp at {t} ({want:?})"
        );
    }
}

/// `grey(light(L))`, L = 0.5, compiled and drawn; then the slider moves L to 1.2 (clamped to 0.9) and, with `write`,
/// the edit is written as the node's uniform. The frame draws the new grey; nothing recompiled.
fn check_slider(write: bool) {
    use wgpu::util::DeviceExt;
    let h = GpuHarness::new().expect("a GPU device");
    let mut rl = RenderLoop::new(h.device(), h.queue(), LAYER_FORMAT, 4, 2)
        .unwrap_or_else(|e| panic!("{e}"));
    let target = h.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("qa target"),
        size: wgpu::Extent3d {
            width: 4,
            height: 2,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: LAYER_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let buffer = |label: &str, n: usize| {
        h.device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: &vec![0u8; n],
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            })
    };
    rl.set_sim(
        buffer("simstate", 8 * simstate_size()),
        Some(buffer("word", 8 * 16)),
    );

    let tree = grey(7, light(3, 0.5));
    let g = gen(&tree);
    let keys = [0, 1, 2, 3];
    rl.cache()
        .request(&stain(&g.source), &keys, Tier::FULL)
        .unwrap_or_else(|e| panic!("{e}"));
    rl.cache().wait();
    assert!(rl.cache().errors().is_empty(), "{:?}", rl.cache().errors());
    let key = rl.cache().current().expect("compiled").key();
    let compiles = rl.cache().compiles();
    rl.frame(&INPUTS, &view);
    assert_grey(&read(&h, &target), 0.5, "the generated default");

    // The uniform for node 3's L is the one its function reads.
    let uniform = g
        .functions
        .iter()
        .find(|f| f.id == 3)
        .and_then(|f| {
            f.text.split("uniforms.").nth(1).map(|r| {
                r.split(|c: char| !(c.is_alphanumeric() || c == '_'))
                    .next()
                    .unwrap()
                    .to_owned()
            })
        })
        .expect("node 3 reads a uniform");
    let value = MapParam::L.clamp(1.2).expect("a number");
    assert_eq!(value, 0.9, "the slider's 1.2 does not clamp to L's 0.9");
    if write {
        assert_eq!(
            rl.set_param(1, &uniform, &[value])
                .unwrap_or_else(|e| panic!("{e}")),
            Applied::Now
        );
    }
    rl.frame(&INPUTS, &view);
    assert_eq!(rl.cache().compiles(), compiles, "a slider edit recompiled");
    assert_eq!(rl.cache().current().expect("current").key(), key);
    assert_grey(&read(&h, &target), 0.9, "after the slider edit");
}

#[test]
fn qa_codegen_deterministic_slider_rebinds_and_draws() {
    check_slider(true);
}

negative_control!(
    qa_codegen_deterministic_slider_rebinds_and_draws,
    "without the uniform write, the old grey draws",
    expected = "after the slider edit",
    check_slider(false)
);
