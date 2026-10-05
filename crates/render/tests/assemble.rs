//! The one assembler (`render::assemble`; render contract Part 2; lowering contract Part 2, Part 3a, Part 5), its
//! stains written as node lists (render never depends on engine, systems_architecture §7.1; the engine's stain-graph
//! type, its edits and its params are `crates/engine/tests/stain.rs`'s):
//! - REQ-RENDER-009: the backbone — a graph that feeds post back into colour, reorders the backbone or closes a cycle
//!   is refused, and the generated `shade()` walks sources → colour / brightness → combiner → (post)* → OUT
//!   (`backbone_*`);
//! - REQ-RENDER-010: a built-in occupant and a custom one of the same text assemble to the same source through the one
//!   entry point (`one_path_*`);
//! - REQ-RENDER-012: a custom occupant reading `ctx.sample.ftle`, `ctx.sample.ensemble_spread` and `ctx.sample.word`
//!   compiles and runs on the GPU at every tier variant, E = 0 and E ≥ 1 (`custom_reads_any_tier`);
//! - R-378: the stain's field set is its IR's reads; a stain whose set misses a field it reads is refused; and an
//!   assembled stain reading one field loads only that field's stored members and word components in naga's MSL,
//!   HLSL and SPIR-V (`assemble_field_set_*`, the method of `crates/ledger/tests/per_member_loads.rs`);
//! - REQ-RENDER-016: the slot signatures, colour → `vec3<f32>`, brightness → `f32` (`slot_signature_*`);
//! - REQ-RENDER-075: the canonical form — two wirings of one graph hash equal and assemble to one source, different
//!   graphs hash differently, and the text is the defined form (`canonical_hash_*`; the render key's params are the
//!   engine's);
//! - REQ-GEN-027: the declaration format, `// @uniform` and `// @input` (`declaration_*`).
//!
//! Each test registers its negative control (R-176). Run `assemble_field_set_an_assembled_stain_loads_only_its_fields_words`
//! with `--nocapture` for each backend's loads.

use std::collections::BTreeSet;

use ledger::gen::{prelude, read, rust, wgsl};
use naga::valid::{Capabilities, ModuleInfo, ValidationFlags, Validator};
use naga::Module;
use render::assemble::{
    self, AssembleError, Declaration, Kind, Node, Occupant, PortType, Stain, Tier, UniformType,
};
use validation::gpu::{BindingKind, GpuHarness};
use validation::negative_control;

/// A test entry: shades sample 0 at the pixel, with `ensemble_spread` 0.25, unit masses and fixed read params
/// (`dt_macro` 0.01, `delta_0` 1e-6, `n_renorm` 16, `horizon_steps` 1000), and returns the colour's bits.
const ENTRY: &str = r"
@fragment
fn t_stain(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<u32> {
    let rgb = shade_sample(0u, pos.xy, 0.25, vec3<f32>(1.0, 1.0, 1.0), ReadParams(0.01, 1e-6, 16u, 1000u));
    return vec4<u32>(bitcast<vec3<u32>>(rgb), 0u);
}
";

const NO_FTLE: Tier = Tier {
    has_ftle: false,
    has_word: true,
};
const NO_WORD: Tier = Tier {
    has_ftle: true,
    has_word: false,
};

/// The canonical quiet NaN's bits (lowering Part 3a).
const QNAN: u32 = 0x7fc0_0000;

/// A colour occupant that shows its first input.
const SHOW_INPUT: &str = "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.inputs[0].x); }";

/// A post occupant that passes its colour on.
const PASS_POST: &str = "fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> { return rgb; }";

fn custom(text: &str) -> Occupant {
    Occupant::Custom(text.to_owned())
}

fn pass_through() -> Occupant {
    Occupant::BuiltIn("pass_through".into())
}

/// A node of `kind` and `occupant`, its in-ports fed by `inputs`, exactly.
fn node(kind: Kind, occupant: Occupant, inputs: &[Option<usize>]) -> Node {
    Node {
        kind,
        occupant,
        inputs: inputs.to_vec(),
    }
}

/// A node of `kind` and `occupant`, one input per in-port its occupant declares: the first fed by `fed`, the rest
/// absent.
fn wired(kind: Kind, occupant: Occupant, fed: &[Option<usize>]) -> Node {
    let ports = assemble::declaration(kind, &occupant)
        .map_or(fed.len(), |d| assemble::in_ports(kind, &d).len());
    let mut inputs = fed.to_vec();
    inputs.resize(ports, None);
    node(kind, occupant, &inputs)
}

/// The backbone with `source` (node 0) into a colour of `colour` (1), into the combiner (2), into OUT (3).
fn graph(source: Occupant, colour: &str) -> Vec<Node> {
    vec![
        wired(Kind::Source, source, &[]),
        wired(Kind::Colour, custom(colour), &[Some(0)]),
        node(Kind::Combiner, pass_through(), &[Some(1), None]),
        node(Kind::Out, Occupant::None, &[Some(2)]),
    ]
}

/// A post of `occupant` added before OUT, which is last: it reads what OUT read, and OUT reads it. Its position.
fn add_post(g: &mut Vec<Node>, occupant: Occupant) -> usize {
    let out = g.pop().expect("OUT, last");
    let p = g.len();
    g.push(wired(Kind::Post, occupant, &[out.inputs[0]]));
    g.push(node(Kind::Out, Occupant::None, &[Some(p)]));
    p
}

/// Node `i` given `occupant`, its inputs kept as far as its new in-ports reach.
fn set_occupant(g: &mut [Node], i: usize, occupant: Occupant) {
    let inputs = g[i].inputs.clone();
    g[i] = wired(g[i].kind, occupant, &inputs);
}

fn stain(g: &[Node]) -> Stain {
    Stain::new(g.to_vec()).unwrap_or_else(|e| panic!("{e}"))
}

fn assembled(g: &[Node], tier: Tier) -> String {
    assemble::assemble(&stain(g), tier)
        .unwrap_or_else(|e| panic!("{e}"))
        .source
}

fn parse(source: &str) -> (Module, ModuleInfo) {
    let module = naga::front::wgsl::parse_str(source)
        .unwrap_or_else(|e| panic!("{}", e.emit_to_string(source)));
    let caps = Capabilities::default() | Capabilities::SHADER_FLOAT16_IN_FLOAT32;
    let info = Validator::new(ValidationFlags::all(), caps)
        .validate(&module)
        .unwrap_or_else(|e| panic!("{}", e.emit_to_string(source)));
    (module, info)
}

// ── The backbone (REQ-RENDER-009) ─────────────────────────────────────────────────────────────────────────────────

/// The assembler's own graph refuses a backward wire, the only way to write a cycle, and the backbone's breaks.
fn check_stain_refused(cases: &[(Vec<Node>, &str)]) {
    for (nodes, want) in cases {
        let got = Stain::new(nodes.clone());
        assert!(
            got.as_ref().is_err_and(|e| e.to_string().contains(want)),
            "a stain that should be refused ({want}) gave {got:?}"
        );
    }
}

fn chain(posts: usize) -> Vec<Node> {
    let mut v = vec![node(Kind::Combiner, pass_through(), &[None, None])];
    for k in 0..posts {
        v.push(node(
            Kind::Post,
            Occupant::Custom(PASS_POST.into()),
            &[Some(k)],
        ));
    }
    v.push(node(Kind::Out, Occupant::None, &[Some(posts)]));
    v
}

fn stain_cases() -> Vec<(Vec<Node>, &'static str)> {
    use Occupant as O;
    let post = O::Custom(PASS_POST.into());
    vec![
        (
            vec![
                node(Kind::Combiner, pass_through(), &[None, None]),
                node(Kind::Post, post.clone(), &[Some(2)]),
                node(Kind::Out, O::None, &[Some(1)]),
            ],
            "runs forward",
        ),
        (
            vec![
                node(Kind::Combiner, pass_through(), &[None, None]),
                node(Kind::Post, post.clone(), &[Some(1)]),
                node(Kind::Out, O::None, &[Some(1)]),
            ],
            "runs forward",
        ),
        (
            vec![
                node(Kind::Combiner, pass_through(), &[None, None]),
                node(Kind::Combiner, pass_through(), &[None, None]),
                node(Kind::Out, O::None, &[Some(1)]),
            ],
            "one combiner",
        ),
        (
            vec![node(Kind::Combiner, pass_through(), &[None, None])],
            "one out",
        ),
        (
            vec![
                node(Kind::Combiner, pass_through(), &[None, None]),
                node(Kind::Out, O::None, &[Some(0)]),
                node(Kind::Out, O::None, &[Some(0)]),
            ],
            "one out",
        ),
        (vec![node(Kind::Out, O::None, &[None])], "one combiner"),
        (
            vec![
                node(Kind::Combiner, pass_through(), &[None]),
                node(Kind::Out, O::None, &[Some(0)]),
            ],
            "1 input(s) given",
        ),
        (
            vec![
                node(Kind::Combiner, O::None, &[None, None]),
                node(Kind::Out, O::None, &[Some(0)]),
            ],
            "the combiner is required",
        ),
        (
            vec![
                node(Kind::Combiner, pass_through(), &[None, None]),
                node(Kind::Out, pass_through(), &[Some(0)]),
            ],
            "does not take an occupant",
        ),
        (chain(assemble::MAX_POSTS + 1), "at most 8"),
    ]
}

/// Graphs that break the backbone: post fed back into colour, colour past the combiner, a post or a brightness into
/// the combiner's colour, colour straight into OUT, and a cycle, which can only be written as a wire running backward.
fn backbone_cases() -> Vec<(Vec<Node>, &'static str)> {
    let post = || custom(PASS_POST);
    let colour = || custom(SHOW_INPUT);
    let combiner = || node(Kind::Combiner, pass_through(), &[None, None]);
    vec![
        (
            vec![
                combiner(),
                node(Kind::Post, post(), &[Some(0)]),
                node(Kind::Colour, colour(), &[Some(1)]),
                node(Kind::Out, Occupant::None, &[Some(1)]),
            ],
            "cannot feed",
        ),
        (
            vec![
                node(Kind::Source, Occupant::Field("d_min".into()), &[]),
                node(Kind::Colour, colour(), &[Some(0)]),
                node(Kind::Post, post(), &[Some(1)]),
                combiner(),
                node(Kind::Out, Occupant::None, &[Some(2)]),
            ],
            "backbone",
        ),
        (
            vec![
                node(Kind::Post, post(), &[None]),
                node(Kind::Combiner, pass_through(), &[Some(0), None]),
                node(Kind::Out, Occupant::None, &[Some(1)]),
            ],
            "backbone",
        ),
        (
            vec![
                node(Kind::Brightness, Occupant::None, &[None]),
                node(Kind::Combiner, pass_through(), &[Some(0), None]),
                node(Kind::Out, Occupant::None, &[Some(1)]),
            ],
            "cannot feed",
        ),
        (
            vec![
                node(Kind::Source, Occupant::Field("d_min".into()), &[]),
                node(Kind::Colour, colour(), &[Some(0)]),
                combiner(),
                node(Kind::Out, Occupant::None, &[Some(1)]),
            ],
            "backbone",
        ),
        (
            vec![
                combiner(),
                node(Kind::Post, post(), &[Some(2)]),
                node(Kind::Post, post(), &[Some(1)]),
                node(Kind::Out, Occupant::None, &[Some(2)]),
            ],
            "runs forward",
        ),
    ]
}

#[test]
fn backbone_a_graph_that_breaks_it_is_refused() {
    check_stain_refused(&backbone_cases());
}

negative_control!(
    backbone_a_graph_that_breaks_it_is_refused,
    "a post wired after the combiner is a graph, not refused",
    expected = "should be refused",
    check_stain_refused(&[(chain(1), "backbone")])
);

#[test]
fn backbone_the_assemblers_graph_is_checked_at_construction() {
    check_stain_refused(&stain_cases());
    // The bound itself is a stain.
    let s = Stain::new(chain(assemble::MAX_POSTS)).expect("eight posts");
    assert_eq!(s.nodes().len(), assemble::MAX_POSTS + 2);
}

negative_control!(
    backbone_the_assemblers_graph_is_checked_at_construction,
    "a combiner wired to OUT is a stain",
    expected = "should be refused",
    check_stain_refused(&[(chain(0), "backbone")])
);

/// The brightness of the full graph: its input.
const BRIGHTNESS: &str = "fn brightness(ctx: Ctx) -> f32 { return ctx.inputs[0].x; }";
/// The full graph's combiner, posts in order.
const COMBINE: &str = "fn combine(rgb: vec3<f32>, b: f32) -> vec3<f32> { return rgb * b; }";
const POST_1: &str = "fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> { return rgb * 0.5; }";
const POST_2: &str = "fn post(ctx: Ctx, rgb: vec3f) -> vec3f { return 1.0 - rgb; }";

/// A graph with every kind: a source (0) into a colour (1) and a brightness (2), the combiner (3) reading both, two
/// posts (4, 5) and OUT (6); the positions of the source, colour, brightness and posts.
fn full_graph() -> (Vec<Node>, [usize; 5]) {
    let g = vec![
        wired(Kind::Source, Occupant::Field("ftle".into()), &[]),
        wired(Kind::Colour, custom(SHOW_INPUT), &[Some(0)]),
        wired(Kind::Brightness, custom(BRIGHTNESS), &[Some(0)]),
        node(Kind::Combiner, custom(COMBINE), &[Some(1), Some(2)]),
        wired(Kind::Post, custom(POST_1), &[Some(3)]),
        wired(Kind::Post, custom(POST_2), &[Some(4)]),
        node(Kind::Out, Occupant::None, &[Some(5)]),
    ];
    (g, [0, 1, 2, 4, 5])
}

/// `shade()`'s calls, in the order they are made: each node function's name.
fn shade_calls(source: &str) -> Vec<String> {
    let body = source
        .split("fn shade(ctx: Ctx)")
        .nth(1)
        .and_then(|s| s.split("\n}\n").next())
        .expect("shade()");
    let mut out = Vec::new();
    for line in body.lines() {
        for slot in [
            "_source(",
            "_colour(",
            "_brightness(",
            "_combine(",
            "_post(",
        ] {
            if let Some(at) = line.find(slot) {
                let start = line[..at]
                    .rfind(|c: char| !(c.is_alphanumeric() || c == '_'))
                    .map_or(0, |k| k + 1);
                out.push(line[start..at + slot.len() - 1].to_owned());
            }
        }
    }
    out
}

/// `shade()` calls the nodes in backbone order.
fn check_shade_order(g: &[Node], want: &[&str]) {
    let got = shade_calls(&assembled(g, Tier::FULL));
    assert_eq!(got, want, "shade() is not in backbone order");
}

#[test]
fn backbone_shade_follows_the_backbone() {
    // Canonical positions: the source 0, the colour 1, the brightness 2, the combiner 3, the posts 4 and 5, OUT 6.
    check_shade_order(
        &full_graph().0,
        &[
            "n0_source",
            "n1_colour",
            "n2_brightness",
            "n3_combine",
            "n4_post",
            "n5_post",
        ],
    );
}

negative_control!(
    backbone_shade_follows_the_backbone,
    "the posts in the other order are not the graph's order",
    expected = "not in backbone order",
    check_shade_order(
        &full_graph().0,
        &[
            "n0_source",
            "n1_colour",
            "n2_brightness",
            "n3_combine",
            "n5_post",
            "n4_post"
        ],
    )
);

// ── Identity (render_gui_spec §13; colour_composition §4.1) ───────────────────────────────────────────────────────

/// Each graph's `shade()` contains its `want`.
fn check_identity(cases: &[(Vec<Node>, &str)]) {
    for (g, want) in cases {
        let source = assembled(g, Tier::FULL);
        let body = source.split("fn shade(ctx: Ctx)").nth(1).expect("shade()");
        assert!(
            body.contains(want),
            "shade() does not give `{want}`:\n{body}"
        );
    }
}

fn identity_cases() -> Vec<(Vec<Node>, &'static str)> {
    let (both, [s, c, b, p1, _]) = full_graph();
    let mut colour_none = both.clone();
    set_occupant(&mut colour_none, c, Occupant::None);
    let mut brightness_none = both.clone();
    brightness_none[b].inputs[0] = None;
    let mut neither = colour_none.clone();
    neither[b].inputs[0] = None;
    let mut source_none = both.clone();
    set_occupant(&mut source_none, s, Occupant::None);
    let mut post_none = both.clone();
    set_occupant(&mut post_none, p1, Occupant::None);
    let mut chain_cut = both.clone();
    chain_cut[p1].inputs[0] = None;
    vec![
        (both, "var out = n3_combine(rgb, b);"),
        (colour_none, "var out = n2_combine(vec3<f32>(1.0), b);"),
        (brightness_none, "var out = rgb;"),
        (neither, "var out = ramp_grey(0.6);"),
        (source_none, "var out = ramp_grey(0.6);"),
        (post_none, "out = n4_post(c4, out);\n    return out;"),
        (chain_cut, "var out = n3_combine(rgb, b);\n    var c4 = ctx;\n    out = n4_post(c4, out);\n    var c5"),
    ]
}

#[test]
fn backbone_absent_slots_are_the_identity() {
    check_identity(&identity_cases());
}

negative_control!(
    backbone_absent_slots_are_the_identity,
    "a graph with both slots is not the flat grey",
    expected = "does not give",
    check_identity(&[(full_graph().0, "var out = ramp_grey(0.6);")])
);

// ── One compile path (REQ-RENDER-010) ─────────────────────────────────────────────────────────────────────────────

/// `a` and `b` assemble to the same WGSL but for the node comments, which name the occupant.
fn check_same_source(a: &[Node], b: &[Node]) {
    let strip = |s: String| {
        s.lines()
            .filter(|l| !l.starts_with("// Node "))
            .collect::<Vec<_>>()
            .join("\n")
    };
    assert!(
        strip(assembled(a, Tier::FULL)) == strip(assembled(b, Tier::FULL)),
        "the built-in and the custom assemble differently"
    );
}

#[test]
fn one_path_a_builtin_is_assembled_as_a_custom_of_its_text() {
    let built_in = graph(Occupant::Field("d_min".into()), SHOW_INPUT);
    let mut as_custom = built_in.clone();
    let text = assemble::builtin(Kind::Combiner, "pass_through").expect("the built-in");
    as_custom[2].occupant = custom(text);
    check_same_source(&built_in, &as_custom);
    let source = assembled(&built_in, Tier::FULL);
    assert!(source.contains("// Node 2: combiner, built-in `pass_through`."));
    assert!(source.contains("// Node 1: colour, custom."));
    assert!(source.contains("// Node 0: source, the built-in source of `d_min`."));
}

negative_control!(
    one_path_a_builtin_is_assembled_as_a_custom_of_its_text,
    "a custom of other text assembles differently",
    expected = "assemble differently",
    {
        let built_in = graph(Occupant::Field("d_min".into()), SHOW_INPUT);
        let mut other = built_in.clone();
        other[2].occupant =
            custom("fn combine(rgb: vec3<f32>, b: f32) -> vec3<f32> { return rgb * 2.0; }");
        check_same_source(&built_in, &other);
    }
);

// ── The read side at every tier (REQ-RENDER-012) ──────────────────────────────────────────────────────────────────

/// The custom colour of REQ-RENDER-012: `ftle`, `ensemble_spread` and the whole word, by plain member access.
const READS_ANY_TIER: &str = r"
// Reads three fields at any tier, by plain member access (lowering Part 3a).
fn colour(ctx: Ctx) -> vec3<f32> {
    let w = ctx.sample.word;
    return vec3<f32>(ctx.sample.ftle, ctx.sample.ensemble_spread, bitcast<f32>(w.x ^ w.y ^ w.z ^ w.w));
}
";

/// The stored variant `tier` binds: each WGSL member and its byte range, by the ledger's layout.
fn layout(tier: Tier) -> Vec<(String, u32, u32)> {
    let name = if tier.has_ftle {
        "SimStateFTLE"
    } else {
        "SimStateBase"
    };
    let s = ledger::payload::structs()
        .into_iter()
        .find(|s| s.name == name)
        .expect("the stored variant");
    let (offsets, size) = rust::offsets(&s);
    let wgsl_names = wgsl::members(&s);
    let mut out = Vec::new();
    for (k, (m, &at)) in s.members.iter().zip(&offsets).enumerate() {
        let end = offsets.get(k + 1).copied().unwrap_or(size);
        let name = wgsl_names
            .iter()
            .find(|w| w.stores.contains(&m.name))
            .map_or_else(|| m.name.to_owned(), |w| w.name.clone());
        out.push((name, at, end));
    }
    out
}

/// One stored sample at `tier`: `S = 2`, `t_end_step = 100`, state escape, and a shadow `delta_0` from the state, so
/// `ftle = (S + ln(δ/δ₀))/(n·dt) = 2/(100 · 0.01)` (payload §5).
fn stored(tier: Tier) -> Vec<u32> {
    let l = layout(tier);
    let size = l.last().map_or(0, |m| m.2) as usize / 4;
    let mut words = vec![0u32; size];
    let at = |m: &str| {
        l.iter()
            .find(|x| x.0 == m)
            .map(|x| x.1 as usize / 4)
            .expect(m)
    };
    words[at("S")] = 2.0f32.to_bits();
    words[at("times")] = 100;
    if tier.has_ftle {
        words[at("r_sh")] = 1e-6f32.to_bits();
    }
    words
}

/// The word of sample 0.
const WORD: [u32; 4] = [1, 2, 4, 0x0800_0000];

/// What the custom colour reads at `tier` with `e` copies: `ftle` (2 within f32 rounding, or the canonical NaN without
/// the FTLE tier), `ensemble_spread` (0.25, or the NaN at E = 0) and the word's xor (`FGW_UNBOUND`'s without the word).
fn check_reads(tier: Tier, e: u32, got: [u32; 4]) {
    let ftle = f32::from_bits(got[0]);
    if tier.has_ftle {
        assert!(
            (ftle - 2.0).abs() < 1e-5,
            "{tier:?}, E = {e}: ftle reads {ftle}, not 2"
        );
    } else {
        assert_eq!(
            got[0], QNAN,
            "{tier:?}: a tier-absent ftle does not read the canonical NaN"
        );
    }
    let spread = if e >= 1 { 0.25f32.to_bits() } else { QNAN };
    assert_eq!(
        got[1], spread,
        "{tier:?}, E = {e}: ensemble_spread reads {:#x}",
        got[1]
    );
    let word = if tier.has_word {
        WORD
    } else {
        [0, 0, 0, 0xfe00_0000]
    };
    assert_eq!(
        got[2],
        word.iter().fold(0, |a, w| a ^ w),
        "{tier:?}: the word reads {:#x}",
        got[2]
    );
}

/// The custom colour `colour`, assembled and drawn at every tier with E = 0 and E = 1; each pixel checked.
fn check_any_tier(h: &GpuHarness, colour: &str) {
    let g = graph(Occupant::Field("ftle".into()), colour);
    for tier in Tier::ALL {
        let source = assembled(&g, tier) + ENTRY;
        let group1: &[BindingKind] = if tier.has_word {
            &[BindingKind::Storage, BindingKind::Storage]
        } else {
            &[BindingKind::Storage]
        };
        let kernel = h
            .fragment(&source, "t_stain", &[&[BindingKind::Uniform], group1], 1, 1)
            .unwrap_or_else(|e| panic!("{tier:?}: {e}"));
        let state = stored(tier);
        for e in [0, 1] {
            let uniforms = prelude::uniform_words(e);
            let g0: &[&[u32]] = &[&uniforms];
            let g1: Vec<&[u32]> = if tier.has_word {
                vec![&state, &WORD]
            } else {
                vec![&state]
            };
            let pixels = kernel
                .draw(&[g0, &g1])
                .unwrap_or_else(|err| panic!("{tier:?}: {err}"));
            check_reads(tier, e, pixels[0]);
        }
    }
}

#[test]
fn custom_reads_any_tier() {
    check_any_tier(&GpuHarness::new().expect("a GPU device"), READS_ANY_TIER);
}

negative_control!(
    custom_reads_any_tier,
    "a colour reading 0.25 for `ensemble_spread` reads it at E = 0, where the field is the absence NaN",
    expected = "ensemble_spread reads",
    check_any_tier(
        &GpuHarness::new().expect("a GPU device"),
        &READS_ANY_TIER.replace("ctx.sample.ensemble_spread", "0.25"),
    )
);

// ── The field set (R-378) ─────────────────────────────────────────────────────────────────────────────────────────

/// A source of the word's length, which reads `.w` alone.
fn length_source() -> Occupant {
    custom(
        "fn source(ctx: Ctx) -> Field { return Field(f32(fgw_length_raw(ctx.sample.word)), 0.0, 0.0, 0.0); }",
    )
}

/// A source passing the word to `fgw_length_raw`, which reads `.w`, in a block, both arms of an `if`, a loop and a
/// switch: each call is followed into, wherever it is.
const NESTED_LENGTHS: &str = r"
fn source(ctx: Ctx) -> Field {
    var n = 0.0;
    { n += f32(fgw_length_raw(ctx.sample.word)); }
    if n > 0.0 { n += f32(fgw_length_raw(ctx.sample.word)); } else { n -= f32(fgw_length_raw(ctx.sample.word)); }
    loop { n += f32(fgw_length_raw(ctx.sample.word)); break; }
    switch 0 { default { n += f32(fgw_length_raw(ctx.sample.word)); } }
    return Field(n, 0.0, 0.0, 0.0);
}
";

/// A source reading the word's `.x` and the whole word besides: the whole word.
const WORD_AND_WHOLE: &str = r"
fn source(ctx: Ctx) -> Field {
    let w = ctx.sample.word;
    return Field(f32(w.x), bitcast<vec4<f32>>(w).y, 0.0, 0.0);
}
";

/// A source reading `d_min` of a copy of the sample, through a pointer.
const SAMPLE_COPIED: &str = r"
fn source(ctx: Ctx) -> Field {
    var s = ctx.sample;
    return Field(s.d_min, 0.0, 0.0, 0.0);
}
";

/// Each stain's field set, by its IR, is `want`: the live nodes' reads only.
fn check_field_sets(cases: &[(Vec<Node>, &[&str])]) {
    for (g, want) in cases {
        let got = assemble::field_set(&stain(g), Tier::FULL).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(got, *want, "the stain's field set");
    }
}

fn field_set_cases() -> Vec<(Vec<Node>, &'static [&'static str])> {
    let d_min = graph(Occupant::Field("d_min".into()), SHOW_INPUT);
    let length = graph(length_source(), SHOW_INPUT);
    // A source nothing live reads, into a colour nothing reads.
    let mut dead = d_min.clone();
    let s = dead.len();
    dead.push(wired(Kind::Source, Occupant::Field("S".into()), &[]));
    dead.push(wired(Kind::Colour, custom(SHOW_INPUT), &[Some(s)]));
    let reads_three = graph(Occupant::Field("ftle".into()), READS_ANY_TIER);
    let source = |text: &str| graph(custom(text), SHOW_INPUT);
    vec![
        (d_min, &["d_min"]),
        (length, &["word.w"]),
        (dead, &["d_min"]),
        (reads_three, &["ensemble_spread", "ftle", "word"]),
        (source(NESTED_LENGTHS), &["word.w"]),
        (source(WORD_AND_WHOLE), &["word"]),
        (source(SAMPLE_COPIED), &["d_min"]),
    ]
}

#[test]
fn assemble_field_set_is_the_stains_reads() {
    check_field_sets(&field_set_cases());
}

negative_control!(
    assemble_field_set_is_the_stains_reads,
    "a stain reading `d_min` does not read `S`",
    expected = "the stain's field set",
    check_field_sets(&[(
        graph(Occupant::Field("d_min".into()), SHOW_INPUT),
        &["S", "d_min"]
    )])
);

/// `fields` for the stain reading `ftle` is refused, naming the field it misses, never assembled to read 0.
fn check_unfilled(fields: &[&str]) {
    let g = graph(Occupant::Field("ftle".into()), SHOW_INPUT);
    let got = assemble::assemble_reading(&stain(&g), Tier::FULL, fields);
    assert!(
        got.as_ref()
            .is_err_and(|e| *e == AssembleError::UnfilledField("ftle".into())),
        "not refused: {fields:?}"
    );
    assert!(got.is_err_and(|e| e.to_string().contains("would leave it reading 0")));
}

#[test]
fn assemble_field_set_a_stain_missing_a_field_it_reads_is_refused() {
    check_unfilled(&["d_min"]);
    check_unfilled(&[]);
    check_unfilled(&["word"]);
    check_unfilled(&["word.x", "word.y", "word.z", "word.w"]);
}

negative_control!(
    assemble_field_set_a_stain_missing_a_field_it_reads_is_refused,
    "the field set holding `ftle` is not refused",
    expected = "not refused",
    check_unfilled(&["ftle", "d_min"])
);

/// The word asked for by its four components covers the word, the word covers each component, and the fragment
/// records the fields it fills, each once.
#[test]
fn assemble_field_set_the_word_and_its_components_cover_each_other() {
    let whole = graph(Occupant::Field("ftle".into()), READS_ANY_TIER);
    let parts = [
        "word.w",
        "ftle",
        "ensemble_spread",
        "word.x",
        "word.y",
        "word.z",
        "ftle",
    ];
    let f = assemble::assemble_reading(&stain(&whole), Tier::FULL, &parts)
        .expect("four components are the word");
    assert_eq!(
        f.fields,
        [
            "ensemble_spread",
            "ftle",
            "word.w",
            "word.x",
            "word.y",
            "word.z"
        ],
        "the fields filled"
    );
    let length = graph(length_source(), SHOW_INPUT);
    assemble::assemble_reading(&stain(&length), Tier::FULL, &["word"])
        .expect("the word holds `.w`");
}

negative_control!(
    assemble_field_set_the_word_and_its_components_cover_each_other,
    "three components are not the word",
    expected = "UnfilledField",
    {
        let whole = graph(Occupant::Field("ftle".into()), READS_ANY_TIER);
        let parts = ["ftle", "ensemble_spread", "word.x", "word.y", "word.z"];
        assemble::assemble_reading(&stain(&whole), Tier::FULL, &parts).unwrap();
    }
);

// ── The loads, from the compiled output (R-378; per_member_loads.rs's method) ────────────────────────────────────

/// The stored words a compiled stain loads: each `simstate_buffer` member by its WGSL name (`(whole)` for a load of
/// the whole stored struct), and each `word_buffer` component.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Loads {
    state: BTreeSet<String>,
    word: BTreeSet<String>,
    excerpt: Vec<String>,
}

const WHOLE: &str = "(whole)";

/// The index just past the bracket that closes the one opening at `open` in `s`.
fn past_bracket(s: &str, open: usize) -> usize {
    let mut depth = 0;
    for (k, ch) in s[open..].char_indices() {
        match ch {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return open + k + 1;
                }
            }
            _ => {}
        }
    }
    panic!("unclosed bracket in {s}");
}

fn ident(s: &str) -> &str {
    let end = s
        .find(|c: char| !(c.is_alphanumeric() || c == '_'))
        .unwrap_or(s.len());
    &s[..end]
}

/// A member's WGSL name from naga's MSL one: naga's namer appends `_` to a name that ends in a digit.
fn unsuffixed(name: &str) -> &str {
    match name.strip_suffix('_') {
        Some(base) if base.ends_with(|c: char| c.is_ascii_digit()) => base,
        _ => name,
    }
}

fn msl(module: &Module, info: &ModuleInfo) -> String {
    use naga::proc::{BoundsCheckPolicies, BoundsCheckPolicy};
    let options = naga::back::msl::Options {
        // MSL 3.2 and wgpu's checked bounds, as the compute entry point uses (`engine::compute`).
        lang_version: (3, 2),
        bounds_check_policies: BoundsCheckPolicies {
            index: BoundsCheckPolicy::Restrict,
            buffer: BoundsCheckPolicy::Restrict,
            image_load: BoundsCheckPolicy::Restrict,
            binding_array: BoundsCheckPolicy::Unchecked,
        },
        ..Default::default()
    };
    naga::back::msl::write_string(module, info, &options, &Default::default())
        .unwrap_or_else(|e| panic!("MSL: {e}"))
        .0
}

/// The loads in naga's MSL: `simstate_buffer[…].<member>`, or `simstate_buffer[…]` alone for the whole struct;
/// `word_buffer[…]` for the whole word, `.x` … `.w` after it for one component.
fn msl_loads(source: &str) -> Loads {
    let mut out = Loads::default();
    for line in source.lines() {
        let mut hit = false;
        for (buffer, word) in [("simstate_buffer[", false), ("word_buffer[", true)] {
            let mut from = 0;
            while let Some(at) = line[from..].find(buffer) {
                let past = past_bracket(line, from + at + buffer.len() - 1);
                let member = line[past..].strip_prefix('.').map(ident);
                hit = true;
                match (word, member) {
                    (false, Some(m)) => out.state.insert(unsuffixed(m).to_owned()),
                    (false, None) => out.state.insert(WHOLE.to_owned()),
                    (true, Some(c)) => out.word.insert(c.to_owned()),
                    (true, None) => {
                        out.word
                            .extend(read::WORD_COMPONENTS.iter().map(|c| (*c).to_owned()));
                        true
                    }
                };
                from = past;
            }
        }
        if hit {
            out.excerpt.push(line.trim().to_owned());
        }
    }
    out
}

fn hlsl(module: &Module, info: &ModuleInfo) -> String {
    let mut out = String::new();
    naga::back::hlsl::Writer::new(&mut out, &Default::default(), &Default::default())
        .write(module, info, None)
        .unwrap_or_else(|e| panic!("HLSL: {e}"));
    out
}

/// The loads in naga's HLSL: each `<buffer>.Load<n>(<offset>+…)` of a `ByteAddressBuffer`, the state's offset mapped
/// to the member whose bytes hold it, the word's to the component at `offset / 4`, `n` words long.
fn hlsl_loads(source: &str, layout: &[(String, u32, u32)]) -> Loads {
    let mut out = Loads::default();
    for line in source.lines() {
        let mut hit = false;
        for (buffer, word) in [("simstate_buffer.Load", false), ("word_buffer.Load", true)] {
            let mut from = 0;
            while let Some(at) = line[from..].find(buffer) {
                let rest = &line[from + at + buffer.len()..];
                let width: u32 = rest[..1].parse().unwrap_or(1);
                let args = rest.trim_start_matches(|c: char| c.is_ascii_digit());
                let args = args.strip_prefix('(').unwrap_or(args);
                let digits = ident(args);
                let offset: u32 = if args[digits.len()..].starts_with('+') {
                    digits.parse().unwrap_or(0)
                } else {
                    0
                };
                hit = true;
                if word {
                    for k in 0..width {
                        let c = ((offset / 4 + k) % 4) as usize;
                        out.word.insert(read::WORD_COMPONENTS[c].to_owned());
                    }
                } else {
                    let member = layout
                        .iter()
                        .find(|(_, a, b)| (*a..*b).contains(&offset))
                        .map_or_else(|| format!("(offset {offset})"), |(m, ..)| m.clone());
                    out.state.insert(member);
                }
                from += at + buffer.len();
            }
        }
        if hit {
            out.excerpt.push(line.trim().to_owned());
        }
    }
    out
}

fn spirv(module: &Module, info: &ModuleInfo) -> Vec<u32> {
    naga::back::spv::write_vec(module, info, &Default::default(), None)
        .unwrap_or_else(|e| panic!("SPIR-V: {e}"))
}

/// The loads in naga's SPIR-V: each `OpLoad` whose pointer is an `OpAccessChain` (or a chain of them) rooted at the
/// variable `OpName`d `simstate_buffer` or `word_buffer`; after the wrapper's `0` and the sample index, the member
/// (state) or component (word), none for the whole element.
fn spirv_loads(words: &[u32], members: &[String]) -> Loads {
    use std::collections::HashMap;
    const OP_NAME: u32 = 5;
    const OP_CONSTANT: u32 = 43;
    const OP_LOAD: u32 = 61;
    const OP_ACCESS_CHAIN: u32 = 65;
    const OP_IN_BOUNDS_ACCESS_CHAIN: u32 = 66;
    let mut names = HashMap::new();
    let mut constants = HashMap::new();
    let mut chains: HashMap<u32, (u32, Vec<u32>)> = HashMap::new();
    let mut out = Loads::default();
    let mut at = 5;
    while at < words.len() {
        let (count, op) = ((words[at] >> 16) as usize, words[at] & 0xffff);
        let ins = &words[at..at + count.max(1)];
        match op {
            OP_NAME => {
                let bytes: Vec<u8> = ins[2..].iter().flat_map(|w| w.to_le_bytes()).collect();
                let name: String = bytes
                    .iter()
                    .take_while(|&&b| b != 0)
                    .map(|&b| b as char)
                    .collect();
                names.insert(ins[1], name);
            }
            OP_CONSTANT => {
                constants.insert(ins[2], ins[3]);
            }
            OP_ACCESS_CHAIN | OP_IN_BOUNDS_ACCESS_CHAIN => {
                let (root, mut idx) = chains.get(&ins[3]).cloned().unwrap_or((ins[3], Vec::new()));
                idx.extend_from_slice(&ins[4..]);
                chains.insert(ins[2], (root, idx));
            }
            OP_LOAD => {
                if let Some((root, idx)) = chains.get(&ins[3]) {
                    let element = idx
                        .get(2)
                        .and_then(|i| constants.get(i))
                        .map(|&v| v as usize);
                    match names.get(root).map_or("", String::as_str) {
                        "simstate_buffer" => {
                            let m = element.map_or(WHOLE.to_owned(), |k| members[k].clone());
                            out.excerpt
                                .push(format!("OpLoad (OpAccessChain %simstate_buffer 0 %i {m})"));
                            out.state.insert(m);
                        }
                        "word_buffer" => {
                            match element {
                                Some(c) => {
                                    out.word.insert(read::WORD_COMPONENTS[c].to_owned());
                                }
                                None => out
                                    .word
                                    .extend(read::WORD_COMPONENTS.iter().map(|c| (*c).to_owned())),
                            }
                            let c = element.map_or(WHOLE, |c| read::WORD_COMPONENTS[c]);
                            out.excerpt
                                .push(format!("OpLoad (OpAccessChain %word_buffer 0 %i {c})"));
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        at += count.max(1);
    }
    out
}

/// One case: a stain reading one field through its source, the tier, and the state members and word components it
/// must load, no more.
struct LoadCase {
    source: Occupant,
    tier: Tier,
    state: &'static [&'static str],
    word: &'static [&'static str],
}

fn load_cases() -> Vec<LoadCase> {
    vec![
        LoadCase {
            source: Occupant::Field("d_min".into()),
            tier: Tier::FULL,
            state: &["packed_a"],
            word: &[],
        },
        LoadCase {
            source: Occupant::Field("diffusion".into()),
            tier: NO_FTLE,
            state: &["C_ty", "times"],
            word: &[],
        },
        LoadCase {
            source: Occupant::Field("ftle".into()),
            tier: Tier::FULL,
            state: &["r", "p", "r_sh", "p_sh", "S", "packed_a", "times"],
            word: &[],
        },
        LoadCase {
            source: Occupant::Field("ftle".into()),
            tier: NO_FTLE,
            state: &[],
            word: &[],
        },
        LoadCase {
            source: length_source(),
            tier: Tier::FULL,
            state: &[],
            word: &["w"],
        },
        LoadCase {
            source: length_source(),
            tier: NO_WORD,
            state: &[],
            word: &[],
        },
    ]
}

/// Each case's assembled stain loads exactly its words on every backend; with `every`, its read side fills every
/// field instead, as the checked-in read side does, for the control.
fn check_loads(cases: &[LoadCase], every: bool) {
    let l = ledger::payload::ledger();
    let entries = ledger::gen::validate(&l).expect("the ledger validates");
    let all: Vec<String> = read::members(&l.words, &entries)
        .into_iter()
        .map(|m| m.name)
        .collect();
    let all: Vec<&str> = all.iter().map(String::as_str).collect();
    for c in cases {
        let g = graph(c.source.clone(), SHOW_INPUT);
        let s = stain(&g);
        let fragment = if every {
            assemble::assemble_reading(&s, c.tier, &all)
        } else {
            assemble::assemble(&s, c.tier)
        }
        .unwrap_or_else(|e| panic!("{e}"));
        let (module, info) = parse(&(fragment.source + ENTRY));
        let layout = layout(c.tier);
        let members: Vec<String> = layout.iter().map(|(m, ..)| m.clone()).collect();
        let want_state: BTreeSet<String> = c.state.iter().map(|s| (*s).to_owned()).collect();
        let want_word: BTreeSet<String> = c.word.iter().map(|s| (*s).to_owned()).collect();
        for (backend, loads) in [
            ("MSL", msl_loads(&msl(&module, &info))),
            ("HLSL", hlsl_loads(&hlsl(&module, &info), &layout)),
            ("SPIR-V", spirv_loads(&spirv(&module, &info), &members)),
        ] {
            println!("--- {:?}, {:?}, {backend}:", c.tier, c.source);
            for line in &loads.excerpt {
                println!("    {line}");
            }
            assert_eq!(
                (&loads.state, &loads.word),
                (&want_state, &want_word),
                "{backend} at {:?}: the stain reading {:?} loads other stored words than its field needs",
                c.tier,
                c.source
            );
        }
    }
}

#[test]
fn assemble_field_set_an_assembled_stain_loads_only_its_fields_words() {
    check_loads(&load_cases(), false);
}

negative_control!(
    assemble_field_set_an_assembled_stain_loads_only_its_fields_words,
    "the stain reading `d_min` through a read side filling every field loads every stored member",
    expected = "loads other stored words than its field needs",
    check_loads(&load_cases()[..1], true)
);

// ── The slot signatures (REQ-RENDER-016) ──────────────────────────────────────────────────────────────────────────

/// Each colour or brightness occupant, in a stain, is refused with `want` in the message.
fn check_slots(cases: &[(Kind, &str, &str)]) {
    for &(kind, text, want) in cases {
        let g = match kind {
            Kind::Brightness => vec![
                wired(Kind::Source, Occupant::Field("d_min".into()), &[]),
                wired(Kind::Colour, custom(SHOW_INPUT), &[Some(0)]),
                wired(kind, custom(text), &[Some(0)]),
                node(Kind::Combiner, pass_through(), &[Some(1), Some(2)]),
                node(Kind::Out, Occupant::None, &[Some(3)]),
            ],
            _ => graph(Occupant::Field("d_min".into()), text),
        };
        let got = Stain::new(g)
            .and_then(|s| assemble::assemble(&s, Tier::FULL))
            .map_err(|e| e.to_string());
        assert!(
            got.as_ref().is_err_and(|e| e.contains(want)),
            "a {kind:?} `{text}` was not refused with `{want}`: {got:?}"
        );
    }
}

fn slot_cases() -> Vec<(Kind, &'static str, &'static str)> {
    vec![
        (
            Kind::Colour,
            "fn colour(ctx: Ctx) -> f32 { return 1.0; }",
            "a colour slot is `fn(Ctx) -> vec3<f32>`",
        ),
        (
            Kind::Colour,
            "fn colour(ctx: Ctx) -> vec4<f32> { return vec4<f32>(1.0); }",
            "a colour slot is `fn(Ctx) -> vec3<f32>`",
        ),
        (
            Kind::Colour,
            "fn colour(ctx: Ctx) { }",
            "is `fn(Ctx) -> `; a colour slot",
        ),
        (
            Kind::Brightness,
            "fn brightness(ctx: Ctx) -> vec3<f32> { return vec3<f32>(1.0); }",
            "a brightness slot is `fn(Ctx) -> f32`",
        ),
        (
            Kind::Brightness,
            "fn brightness(c: vec3<f32>) -> f32 { return c.x; }",
            "is `fn(vec3<f32>) -> f32`; a brightness slot is `fn(Ctx) -> f32`",
        ),
        (
            Kind::Brightness,
            "fn brightness(ctx: Ctx, extra: f32) -> f32 { return extra; }",
            "a brightness slot is `fn(Ctx) -> f32`",
        ),
        (
            Kind::Colour,
            "fn paint(ctx: Ctx) -> vec3<f32> { return vec3<f32>(1.0); }",
            "defines `fn colour`",
        ),
        (Kind::Colour, "fn colour", "is `fn() -> `"),
        (
            Kind::Colour,
            "fn colour() -> vec3<f32> { return vec3<f32>(1.0); }",
            "is `fn() -> vec3<f32>`",
        ),
    ]
}

#[test]
fn slot_signature_colour_is_linear_rgb_and_brightness_a_scalar() {
    check_slots(&slot_cases());
    // The slot signatures assemble, `vec3f` and `vec4f` read as the types they name.
    assemble::assemble(&stain(&full_graph().0), Tier::FULL).expect("the slot signatures");
    let g = graph(
        custom("fn source(ctx: Ctx) -> vec4f { return vec4f(1.0); }"),
        "fn colour(ctx: Ctx) -> vec3f { return vec3f(ctx.inputs[0].x); }",
    );
    assemble::assemble(&stain(&g), Tier::FULL).expect("the predeclared aliases");
}

negative_control!(
    slot_signature_colour_is_linear_rgb_and_brightness_a_scalar,
    "a colour returning vec3<f32> is the slot's signature",
    expected = "was not refused",
    check_slots(&[(Kind::Colour, SHOW_INPUT, "a colour slot")])
);

/// What a node may not write: a module-scope binding or `var`, an entry point, or a name of the stored buffers.
fn check_occupant_refused(cases: &[(&str, &str)]) {
    for &(text, want) in cases {
        let g = graph(
            Occupant::Field("d_min".into()),
            &format!("{SHOW_INPUT}\n{text}"),
        );
        let got = assemble::assemble(&stain(&g), Tier::FULL);
        assert!(
            got.as_ref().is_err_and(|e| e.to_string().contains(want)),
            "`{text}` was not refused with `{want}`: {got:?}"
        );
    }
}

fn occupant_cases() -> Vec<(&'static str, &'static str)> {
    vec![
        ("var<private> x: f32;", "module-scope `var`"),
        ("override k: f32 = 1.0;", "module-scope `override`"),
        (
            "@fragment fn e() -> @location(0) vec4<f32> { return vec4<f32>(); }",
            "`@fragment`",
        ),
        ("@vertex fn e() -> @builtin(position) vec4<f32> { return vec4<f32>(); }", "`@vertex`"),
        ("@compute @workgroup_size(1) fn e() { }", "`@compute`"),
        (
            "@group(3) @binding(0) var<storage, read> b: array<u32>;",
            "`@group`",
        ),
        (
            "fn peek() -> u32 { return simstate_buffer[0].packed_a; }",
            "does not name `simstate_buffer`",
        ),
        (
            "fn peek() -> vec4<u32> { return word_buffer[0]; }",
            "does not name `word_buffer`",
        ),
        (
            "fn peek() -> f32 { return sample_read(0u, 0.0, false, vec3<f32>(), ReadParams()).d_min; }",
            "does not name `sample_read`",
        ),
        ("/* never closed", "unclosed"),
        ("alias A = f32", "expected"),
        ("const N = 1", "expected"),
        ("fn", "with no name"),
        (
            "fn colour2(ctx: Ctx) -> vec3<f32> { return vec3<f32>(oops); }",
            "oops",
        ),
    ]
}

#[test]
fn slot_signature_a_node_writes_no_binding_entry_point_or_buffer_read() {
    check_occupant_refused(&occupant_cases());
}

negative_control!(
    slot_signature_a_node_writes_no_binding_entry_point_or_buffer_read,
    "a helper function is allowed",
    expected = "was not refused",
    check_occupant_refused(&[("fn helper() -> f32 { return 1.0; }", "refused")])
);

/// Each colour, its comments holding characters of more than one byte, assembles, never panics, with `want` in its
/// source as written (render_gui_spec §13: a defined fallback, never a crash).
fn check_unicode_comments(cases: &[(&str, &str)]) {
    for &(text, want) in cases {
        let g = graph(Occupant::Field("d_min".into()), text);
        let got = assemble::assemble(&stain(&g), Tier::FULL).map(|f| f.source);
        assert!(
            got.as_ref().is_ok_and(|s| s.contains(want)),
            "`{want}` is not in the source of `{text}`: {got:?}"
        );
    }
}

fn unicode_comment_cases() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "/* é */ fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.inputs[0].x); }",
            "/* é */ fn n1_colour(",
        ),
        (
            "/* ü /* 色 */ ∂ */ fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.inputs[0].x); }",
            "/* ü /* 色 */ ∂ */ fn n1_colour(",
        ),
        (
            "/*é*//*∂*/fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.inputs[0].x); }",
            "/*é*//*∂*/fn n1_colour(",
        ),
        (
            "// é, 色\nfn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.inputs[0].x); } // ∂",
            "// é, 色\nfn n1_colour(",
        ),
    ]
}

#[test]
fn slot_signature_a_comment_of_any_characters_assembles() {
    check_unicode_comments(&unicode_comment_cases());
    // An unclosed comment holding them is refused, not a panic.
    let g = graph(
        Occupant::Field("d_min".into()),
        &format!("{SHOW_INPUT}\n/* é /* 色 */"),
    );
    let got = assemble::assemble(&stain(&g), Tier::FULL);
    assert!(
        got.as_ref()
            .is_err_and(|e| e.to_string().contains("unclosed")),
        "an unclosed comment: {got:?}"
    );
}

negative_control!(
    slot_signature_a_comment_of_any_characters_assembles,
    "a comment is kept as written, its characters unchanged",
    expected = "is not in the source",
    check_unicode_comments(&[(unicode_comment_cases()[0].0, "/* e */")])
);

/// A node's own names are prefixed — its functions, constants, structs and aliases — and its struct members,
/// swizzles, comments and numbers are not, so two nodes of one text coexist.
fn check_own_names(text: &str, want: &[&str]) {
    let mut g = graph(Occupant::Field("d_min".into()), text);
    add_post(
        &mut g,
        custom(&format!(
            "{text}\nfn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> {{ return rgb * helper(Pair(1.0, K)); }}"
        )),
    );
    let source = assembled(&g, Tier::FULL);
    for name in want {
        assert!(source.contains(name), "`{name}` is not in the source");
    }
}

const OWN_NAMES: &str = r"
// colour's helper: `helper`, `K`, `Pair` and `Scalar` are this node's. /* a /* nested */ comment */
const K: f32 = 0.25;
alias Scalar = f32;
struct Pair { colour: Scalar, k: f32 }
fn helper(p: Pair) -> f32 { return p.colour + p.k * K; }
fn colour(ctx: Ctx) -> vec3<f32> {
    let colour = Pair(ctx.inputs[0].x, 1e-2);
    return vec3<f32>(helper(colour), 0x1p-3, 1.5e+1).xyz;
}
";

#[test]
fn slot_signature_a_nodes_names_are_its_own() {
    check_own_names(
        OWN_NAMES,
        &[
            "n1_helper",
            "n1_K",
            "n1_Pair",
            "n1_Scalar",
            "n3_helper",
            "n3_K",
            "n3_Pair",
            "struct n1_Pair { colour: n1_Scalar, k: f32 }",
            "p.colour + p.k * n1_K",
            "// colour's helper: `helper`, `K`, `Pair` and `Scalar` are this node's. /* a /* nested */ comment */",
            "0x1p-3, 1.5e+1).xyz",
        ],
    );
}

negative_control!(
    slot_signature_a_nodes_names_are_its_own,
    "a name the text never declares is not prefixed",
    expected = "is not in the source",
    check_own_names(OWN_NAMES, &["n1_ctx"])
);

/// A colour whose names are its own, written to reach each case of the renaming: a nested block comment holding the
/// words a node may not write; names beginning `_` and named as attributes are; a local `var` and `const`; a local
/// shadowing the node's constant; a struct member of a node's name; hex, exponent and suffixed numbers.
const RENAMED: &str = r"/* a /* nested */ var x: f32; @group(0) fn K */
const _h: f32 = 0.5;
const group: f32 = 1.0;
const K: f32 = 0.25;
struct Pair { colour: f32, K: f32 }
fn colour(ctx: Ctx) -> vec3<f32> {
    var acc = 0x1e-K;
    const q: f32 = 2.0;
    let K: f32 = 1e5f;
    let p = Pair(_h, group);
    return vec3<f32>(acc + q * K + p.K, 1.5e-3, 0.0);
}
";

/// [`RENAMED`] as node 1 of a stain assembles to exactly this, one blank line before the next node's comment.
const RENAMED_AS_NODE_1: &str = r"/* a /* nested */ var x: f32; @group(0) fn K */
const n1__h: f32 = 0.5;
const n1_group: f32 = 1.0;
const n1_K: f32 = 0.25;
struct n1_Pair { colour: f32, K: f32 }
fn n1_colour(ctx: Ctx) -> vec3<f32> {
    var acc = 0x1e-n1_K;
    const q: f32 = 2.0;
    let n1_K: f32 = 1e5f;
    let p = n1_Pair(n1__h, n1_group);
    return vec3<f32>(acc + q * n1_K + p.K, 1.5e-3, 0.0);
}

// Node 2:";

/// The colour `text`, as node 1, assembles to `want`.
fn check_renamed(text: &str, want: &str) {
    let g = graph(Occupant::Field("d_min".into()), text);
    let source = assembled(&g, Tier::FULL);
    assert!(
        source.contains(want),
        "the node does not assemble to\n{want}\nin\n{source}"
    );
}

#[test]
fn slot_signature_a_nodes_text_is_renamed_exactly() {
    check_renamed(RENAMED, RENAMED_AS_NODE_1);
    // A text with no closing newline is given one.
    check_renamed(SHOW_INPUT, "vec3<f32>(ctx.inputs[0].x); }\n\n// Node 2:");
}

negative_control!(
    slot_signature_a_nodes_text_is_renamed_exactly,
    "the local constant `q` is not the node's",
    expected = "does not assemble to",
    check_renamed(RENAMED, &RENAMED_AS_NODE_1.replace("const q", "const n1_q"))
);

// ── Stain construction, read back ─────────────────────────────────────────────────────────────────────────────────

/// Each node of `g`'s stain holds its occupant's declarations.
fn check_stain_declarations(g: &[Node]) {
    let s = stain(g);
    for (i, n) in s.nodes().iter().enumerate() {
        let want = assemble::declaration(n.kind, &n.occupant).expect("a declaration");
        assert_eq!(s.declaration(i), &want, "node {i}'s declarations");
    }
}

/// A colour with a uniform and a declared input.
const DECLARING: &str = "// @uniform gain: f32 = 1.0 [0.0, 2.0]\n// @input a [0.0, 1.0]\nfn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(uniforms.gain); }";

#[test]
fn declaration_a_stain_holds_each_nodes_declarations() {
    check_stain_declarations(&graph(Occupant::Field("d_min".into()), DECLARING));
}

negative_control!(
    declaration_a_stain_holds_each_nodes_declarations,
    "a colour's uniform is in its declarations",
    expected = "node 1's declarations",
    {
        let g = graph(Occupant::Field("d_min".into()), DECLARING);
        let s = stain(&g);
        assert_eq!(
            s.declaration(1),
            &Declaration::default(),
            "node 1's declarations"
        );
    }
);

/// A stain of the combiner, one post of `occupant` and `inputs`, and OUT: `shade()` calls `want`.
fn check_post(occupant: Occupant, inputs: &[Option<usize>], want: &[&str]) {
    let s = Stain::new(vec![
        node(Kind::Combiner, pass_through(), &[None, None]),
        node(Kind::Post, occupant, inputs),
        node(Kind::Out, Occupant::None, &[Some(1)]),
    ])
    .expect("a stain");
    let source = assemble::assemble(&s, Tier::FULL)
        .expect("assembled")
        .source;
    assert_eq!(shade_calls(&source), want, "the post's calls");
}

/// A post whose colour input is absent reads the combiner (here the flat grey, neither slot filled) and is applied: an
/// absent colour input is no identity. A post of None, wired, is the identity: passed over.
#[test]
fn backbone_a_post_with_no_colour_input_reads_the_combiner() {
    let post = || Occupant::Custom(PASS_POST.into());
    check_post(post(), &[None], &["n1_post"]);
    check_post(Occupant::None, &[Some(0)], &[]);
}

negative_control!(
    backbone_a_post_with_no_colour_input_reads_the_combiner,
    "a post of None is not applied",
    expected = "the post's calls",
    check_post(Occupant::None, &[Some(0)], &["n1_post"])
);

// ── The canonical form (REQ-RENDER-075) ───────────────────────────────────────────────────────────────────────────

/// The full graph given a second way: its nodes in another order, a dead source, and an identity colour it feeds.
fn full_graph_rebuilt() -> Vec<Node> {
    vec![
        wired(Kind::Source, Occupant::Field("S".into()), &[]),
        wired(Kind::Source, Occupant::Field("ftle".into()), &[]),
        wired(Kind::Colour, Occupant::None, &[Some(0)]),
        wired(Kind::Brightness, custom(BRIGHTNESS), &[Some(1)]),
        wired(Kind::Colour, custom(SHOW_INPUT), &[Some(1)]),
        node(Kind::Combiner, custom(COMBINE), &[Some(4), Some(3)]),
        wired(Kind::Post, custom(POST_1), &[Some(5)]),
        wired(Kind::Post, custom(POST_2), &[Some(6)]),
        node(Kind::Out, Occupant::None, &[Some(7)]),
    ]
}

/// Each pair hashes equal, with equal canonical text, and assembles to one source.
fn check_equal(pairs: &[(Vec<Node>, Vec<Node>)]) {
    for (a, b) in pairs {
        let (x, y) = (stain(a).canonical(), stain(b).canonical());
        assert!(
            x.fragment_key() == y.fragment_key() && x.text() == y.text(),
            "one graph hashes differently:\n{}\n{}",
            x.text(),
            y.text()
        );
        assert!(
            assembled(a, Tier::FULL) == assembled(b, Tier::FULL),
            "one graph assembles differently"
        );
    }
}

/// The full graph with a second source of `ftle` (1) feeding the brightness.
fn full_graph_two_sources() -> Vec<Node> {
    vec![
        wired(Kind::Source, Occupant::Field("ftle".into()), &[]),
        wired(Kind::Source, Occupant::Field("ftle".into()), &[]),
        wired(Kind::Colour, custom(SHOW_INPUT), &[Some(0)]),
        wired(Kind::Brightness, custom(BRIGHTNESS), &[Some(1)]),
        node(Kind::Combiner, custom(COMBINE), &[Some(2), Some(3)]),
        wired(Kind::Post, custom(POST_1), &[Some(4)]),
        wired(Kind::Post, custom(POST_2), &[Some(5)]),
        node(Kind::Out, Occupant::None, &[Some(6)]),
    ]
}

/// The full graph and its variants, each a different graph.
fn variants() -> Vec<Vec<Node>> {
    let (base, [s, c, _, p1, p2]) = full_graph();
    let combiner = 3;
    let out = 6;
    let mut v = vec![base.clone()];
    let mut edit = |f: &dyn Fn(&mut Vec<Node>)| {
        let mut g = base.clone();
        f(&mut g);
        v.push(g);
    };
    edit(&|g| set_occupant(g, s, Occupant::Field("d_min".into())));
    edit(&|g| set_occupant(g, c, custom(&format!("{SHOW_INPUT} // edited"))));
    edit(&|g| g[combiner].inputs[1] = None);
    // The posts swapped.
    edit(&|g| {
        g[p1].occupant = custom(POST_2);
        g[p2].occupant = custom(POST_1);
    });
    edit(&|g| g[out].inputs[0] = Some(p1));
    edit(&|g| *g = full_graph_two_sources());
    edit(&|g| g[combiner].occupant = pass_through());
    edit(&|g| set_occupant(g, c, Occupant::None));
    v
}

/// Every two graphs hash differently.
fn check_distinct(graphs: &[Vec<Node>]) {
    let texts: Vec<String> = graphs.iter().map(|g| stain(g).canonical().text()).collect();
    let keys: Vec<u64> = graphs
        .iter()
        .map(|g| stain(g).canonical().fragment_key())
        .collect();
    for i in 0..keys.len() {
        for j in 0..i {
            assert!(
                keys[i] != keys[j],
                "graphs {j} and {i} hash the same:\n{}\n{}",
                texts[j],
                texts[i]
            );
        }
    }
}

#[test]
fn canonical_hash_two_wirings_of_one_graph_hash_equal() {
    check_equal(&[(full_graph().0, full_graph_rebuilt())]);
}

negative_control!(
    canonical_hash_two_wirings_of_one_graph_hash_equal,
    "the graph with its posts swapped is another graph",
    expected = "hashes differently",
    check_equal(&[(full_graph().0, variants()[4].clone())])
);

#[test]
fn canonical_hash_different_graphs_hash_differently() {
    check_distinct(&variants());
}

negative_control!(
    canonical_hash_different_graphs_hash_differently,
    "a rebuilt graph is the same graph",
    expected = "hash the same",
    check_distinct(&[full_graph().0, full_graph_rebuilt()])
);

/// The canonical text of a source into a custom colour, as lowering Part 5 defines it, and its hash; the form keeps
/// each node's place in the graph it was taken from.
fn check_text(g: &[Node], want: &str) {
    let c = stain(g).canonical();
    assert_eq!(c.text(), want, "the canonical text");
    assert_eq!(
        c.fragment_key(),
        ledger::version::fnv1a64(want.as_bytes()),
        "the fragment key"
    );
}

fn defined_text() -> String {
    format!(
        r#"{{"nodes":[{{"inputs":[],"kind":"source","occupant":{{"field":"d_min"}}}},{{"inputs":[0],"kind":"colour","occupant":{{"custom":"{SHOW_INPUT}"}}}},{{"inputs":[1,null],"kind":"combiner","occupant":{{"builtin":"pass_through"}}}},{{"inputs":[2],"kind":"out","occupant":null}}]}}"#
    )
}

#[test]
fn canonical_hash_the_text_is_the_defined_form() {
    check_text(
        &graph(Occupant::Field("d_min".into()), SHOW_INPUT),
        &defined_text(),
    );
    // A post's field input, a quote, a backslash and a control character in an occupant's text, escaped (JCS).
    let mut g = graph(Occupant::Field("d_min".into()), SHOW_INPUT);
    let p = add_post(
        &mut g,
        custom(&format!("// @input mask\n{PASS_POST} // \"\\\t\u{1}")),
    );
    g[p].inputs[1] = Some(0);
    let c = stain(&g).canonical();
    assert!(
        c.text().ends_with(
            r#"{"inputs":[2,0],"kind":"post","occupant":{"custom":"// @input mask\nfn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> { return rgb; } // \"\\\t\u0001"}},{"inputs":[3],"kind":"out","occupant":null}]}"#
        ),
        "{}",
        c.text()
    );
    // The chain wired by position: a post's absent colour input reads the combiner, and OUT, fed by a None post,
    // reads the post before it.
    let (mut g, [_, _, _, p1, p2]) = full_graph();
    g[p1].inputs[0] = None;
    set_occupant(&mut g, p2, Occupant::None);
    let chain: Vec<Vec<Option<usize>>> = stain(&g).canonical().stain().nodes()[3..]
        .iter()
        .map(|n| n.inputs.clone())
        .collect();
    assert_eq!(
        chain,
        [vec![Some(1), Some(2)], vec![Some(3)], vec![Some(4)]],
        "the chain's inputs"
    );
    // The full graph given in another order: each canonical node's place in it.
    assert_eq!(
        stain(&full_graph_rebuilt()).canonical().order(),
        [1, 4, 3, 5, 6, 7, 8],
        "the canonical order"
    );
}

negative_control!(
    canonical_hash_the_text_is_the_defined_form,
    "the brightness input absent is `null`, not 0",
    expected = "the canonical text",
    check_text(
        &graph(Occupant::Field("d_min".into()), SHOW_INPUT),
        &defined_text().replace("[1,null]", "[1,0]")
    )
);

// ── The declaration format (REQ-GEN-027) ──────────────────────────────────────────────────────────────────────────

/// Each text parses to `want`'s uniforms and inputs, written `name:type=default range` and `name=domain`.
fn check_declarations(cases: &[(&str, &str)]) {
    for &(text, want) in cases {
        let d = Declaration::parse(text).unwrap_or_else(|e| panic!("{e}"));
        let mut got: Vec<String> = d
            .uniforms
            .iter()
            .map(|u| format!("{}:{}={:?}{:?}", u.name, u.ty.wgsl(), u.default, u.range))
            .collect();
        got.extend(
            d.inputs
                .iter()
                .map(|i| format!("{}={:?}", i.name, i.domain)),
        );
        assert_eq!(got.join(" "), want, "the declarations of `{text}`");
    }
}

const DECLARATIONS: [(&str, &str); 4] = [
    (
        "// @uniform kappa: f32 = 8.0 [0.0, 64.0]\n  // @uniform tint: vec3<f32> = (0.5, 0.25, 1)\nfn colour() {}",
        "kappa:f32=[8.0]Some((0.0, 64.0)) tint:vec3<f32>=[0.5, 0.25, 1.0]None",
    ),
    (
        "// @input t [0.0, 1.0]\n// @input n\n// @uniform steps: u32 = 3 [1, 9]\n// @uniform bias: i32 = -2",
        "steps:u32=[3.0]Some((1.0, 9.0)) bias:i32=[-2.0]None t=Some((0.0, 1.0)) n=None",
    ),
    (
        "// @uniform a: vec2<f32> = (1, 2)\n// @uniform b: vec4<f32> = (1, 2, 3, 4)\n// @uniform c: f32 = 2.5E+2",
        "a:vec2<f32>=[1.0, 2.0]None b:vec4<f32>=[1.0, 2.0, 3.0, 4.0]None c:f32=[250.0]None",
    ),
    (
        "// a comment, not a declaration: @uniform\nfn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(1e-3); }",
        "",
    ),
];

#[test]
fn declaration_the_format_reads_uniforms_and_inputs() {
    check_declarations(&DECLARATIONS);
}

negative_control!(
    declaration_the_format_reads_uniforms_and_inputs,
    "the second uniform has no range",
    expected = "the declarations of",
    check_declarations(&[(
        DECLARATIONS[0].0,
        "kappa:f32=[8.0]Some((0.0, 64.0)) tint:vec3<f32>=[0.5, 0.25, 1.0]Some((0.0, 1.0))"
    )])
);

/// Each text is refused with `want` in the message.
fn check_declaration_refused(cases: &[(&str, &str)]) {
    for &(text, want) in cases {
        let got = Declaration::parse(text);
        assert!(
            got.as_ref().is_err_and(|e| e.to_string().contains(want)),
            "`{text}` was not refused with `{want}`: {got:?}"
        );
    }
}

const BAD_DECLARATIONS: [(&str, &str); 22] = [
    ("// @ uniform x: f32 = 1.0", "a declaration is"),
    ("// @uniforms x: f32 = 1.0", "a declaration is"),
    ("// @uniform", "no `:`"),
    ("fn a() {}\n// @uniform x f32 = 1.0", "line 2"),
    ("// @uniform x: f32 1.0", "no `=`"),
    ("// @uniform x: f64 = 1.0", "the type"),
    ("// @uniform 2x: f32 = 1.0", "is not a name"),
    ("// @uniform x-y: f32 = 1.0", "is not a name"),
    ("// @uniform : f32 = 1.0", "is not a name"),
    ("// @uniform x: f32 = 1.0f", "is not a number"),
    ("// @uniform x: f32 = inf", "is not a number"),
    ("// @uniform x: f32 = ", "is not a number"),
    ("// @uniform x: f32 = 1e999", "is not a number"),
    ("// @uniform x: f32 = 5.0 [0.0, 4.0]", "the default"),
    ("// @uniform x: f32 = 1.0 [4.0, 0.0]", "lo < hi"),
    ("// @uniform x: f32 = 1.0 [1.0, 1.0]", "lo < hi"),
    ("// @uniform x: u32 = -1", "the default"),
    ("// @uniform x: u32 = 4294967296", "the default"),
    ("// @uniform x: i32 = 1.5", "the default"),
    ("// @uniform x: i32 = 2147483648", "the default"),
    ("// @uniform x: vec2<f32> = (1.0, 2.0, 3.0)", "the default"),
    (
        "// @uniform x: vec2<f32> = (1.0, 2.0) [0.0, 1.0]",
        "for a scalar",
    ),
];

#[test]
fn declaration_a_malformed_declaration_is_refused() {
    check_declaration_refused(&BAD_DECLARATIONS);
    check_declaration_refused(&[
        (
            "// @uniform x: f32 = 1.0\n// @uniform x: f32 = 2.0",
            "declared twice",
        ),
        ("// @uniform x: vec2<f32> = (1.0, 2.0", "unclosed"),
        ("// @input t\n// @input t", "declared twice"),
        ("// @input t [0.0]", "two numbers"),
        ("// @input t [0.0, 1.0] extra", "nothing after it"),
        ("// @input t [0.0, 1.0", "nothing after it"),
        // An f32 component beyond f32::MAX is no f32, finite as an f64 though it is.
        ("// @uniform x: f32 = 1e39", "the default"),
        ("// @uniform x: f32 = -3.5e38", "the default"),
        ("// @uniform x: vec3<f32> = (0, 1e39, 0)", "the default"),
        // A name is an ASCII WGSL identifier: not `_` alone, not beginning `__`, no non-ASCII letter.
        ("// @uniform _: f32 = 1.0", "is not a name"),
        ("// @uniform __x: f32 = 1.0", "is not a name"),
        ("// @input __", "is not a name"),
        ("// @input é", "is not a name"),
        ("// @uniform größe: f32 = 1.0", "is not a name"),
    ]);
    // A name may begin with one `_`.
    Declaration::parse("// @uniform _x: f32 = 1.0\n// @input _t\n// @input a_b2")
        .expect("names beginning with one `_`");
    // f32::MAX and its negative are values of an f32, written as Rust writes them, which rounds to them.
    Declaration::parse(&format!(
        "// @uniform a: f32 = {}\n// @uniform b: vec2<f32> = (0, {})",
        f32::MAX,
        -f32::MAX
    ))
    .expect("the f32 extremes");
    // A uniform and an input may share a name: they are read apart, `uniforms.t` and `ctx.inputs[0]`.
    Declaration::parse("// @uniform t: f32 = 1.0\n// @input t")
        .expect("a uniform and an input of one name");
    // The extremes of the integer types are values of them.
    Declaration::parse(
        "// @uniform a: u32 = 4294967295\n// @uniform b: i32 = -2147483648\n// @uniform c: u32 = 0",
    )
    .expect("the integer extremes");
}

negative_control!(
    declaration_a_malformed_declaration_is_refused,
    "a well-formed declaration is read",
    expected = "was not refused",
    check_declaration_refused(&[("// @uniform x: f32 = 1.0 [0.0, 4.0]", "refused")])
);

/// A port case: the kind, the occupant's text, and its ports or a word of its refusal.
type PortCase<'a> = (Kind, &'a str, Result<Vec<PortType>, &'a str>);

/// Each occupant's declared inputs are its node's ports, within the slot's bounds.
fn check_ports(cases: &[PortCase]) {
    for (kind, text, want) in cases {
        let got = assemble::declaration(*kind, &Occupant::Custom((*text).into()))
            .map(|d| assemble::in_ports(*kind, &d))
            .map_err(|e| e.to_string());
        match want {
            Ok(ports) => assert_eq!(got.as_ref(), Ok(ports), "the ports of `{text}`"),
            Err(w) => assert!(
                got.as_ref().is_err_and(|e| e.contains(w)),
                "the ports of `{text}`: {got:?}"
            ),
        }
    }
}

fn port_cases() -> Vec<PortCase<'static>> {
    use PortType::{Field, Vec3, F32};
    vec![
        (Kind::Colour, SHOW_INPUT, Ok(vec![Field])),
        (
            Kind::Colour,
            "// @input a\n// @input b",
            Ok(vec![Field, Field]),
        ),
        (
            Kind::Colour,
            "// @input a\n// @input b\n// @input c\n// @input d",
            Ok(vec![Field; 4]),
        ),
        (Kind::Brightness, "", Ok(vec![Field])),
        (Kind::Brightness, "// @input a [0.0, 2.0]", Ok(vec![Field])),
        (
            Kind::Brightness,
            "// @input a\n// @input b",
            Err("at most 1"),
        ),
        (
            Kind::Colour,
            "// @input a\n// @input b\n// @input c\n// @input d\n// @input e",
            Err("at most 4"),
        ),
        (Kind::Post, "", Ok(vec![Vec3])),
        (Kind::Post, "// @input mask", Ok(vec![Vec3, Field])),
        (Kind::Combiner, "", Ok(vec![Vec3, F32])),
        (Kind::Combiner, "// @input a", Err("at most 0")),
        (Kind::Source, "// @input a", Err("at most 0")),
        (Kind::Source, "", Ok(vec![])),
        (Kind::Out, "", Err("does not take an occupant")),
    ]
}

#[test]
fn declaration_inputs_are_the_nodes_ports() {
    check_ports(&port_cases());
    // The identity's ports: a colour or a brightness None keeps its one input; OUT takes the colour.
    for (kind, want) in [
        (Kind::Colour, vec![PortType::Field]),
        (Kind::Brightness, vec![PortType::Field]),
        (Kind::Post, vec![PortType::Vec3]),
        (Kind::Out, vec![PortType::Vec3]),
        (Kind::Source, vec![]),
    ] {
        let d = assemble::declaration(kind, &Occupant::None).expect("None");
        assert_eq!(
            assemble::in_ports(kind, &d),
            want,
            "the ports of a None {kind:?}"
        );
    }
}

negative_control!(
    declaration_inputs_are_the_nodes_ports,
    "two declared inputs are two ports",
    expected = "the ports of",
    check_ports(&[(
        Kind::Colour,
        "// @input a\n// @input b",
        Ok(vec![PortType::Field])
    )])
);

/// A colour with two uniforms, a range on the first.
const GAIN_TINT: &str = "// @uniform gain: f32 = 1.0 [0.0, 4.0]\n// @uniform tint: vec3<f32> = (1.0, 0.5, 0.25)\nfn colour(ctx: Ctx) -> vec3<f32> { return uniforms.tint * ctx.inputs[0].x * uniforms.gain; }";

/// A node with a schema gets its uniform block, at the binding after the prelude's, read as `uniforms.<name>`; a
/// value is the schema's when it is `value`.
fn check_uniforms(value: Vec<f64>) {
    let lift = "// @uniform lift: f32 = 0.0\nfn brightness(ctx: Ctx) -> f32 { return ctx.inputs[0].x + uniforms.lift; }";
    let g = vec![
        wired(Kind::Source, Occupant::Field("d_min".into()), &[]),
        wired(Kind::Brightness, custom(lift), &[Some(0)]),
        wired(Kind::Colour, custom(GAIN_TINT), &[Some(0)]),
        node(Kind::Combiner, pass_through(), &[Some(2), Some(1)]),
        node(Kind::Out, Occupant::None, &[Some(3)]),
    ];
    let f = assemble::assemble(&stain(&g), Tier::FULL).unwrap_or_else(|e| panic!("{e}"));
    let first = prelude::uniforms_binding();
    let blocks: Vec<(usize, u32, u32, usize)> = f
        .uniforms
        .iter()
        .map(|u| (u.node, u.group, u.binding, u.uniforms.len()))
        .collect();
    // Canonical positions: the colour 1, the brightness 2.
    assert_eq!(
        blocks,
        [
            (1, first.group, first.binding + 1, 2),
            (2, first.group, first.binding + 2, 1)
        ],
        "the uniform blocks"
    );
    assert_eq!(f.uniforms[0].uniforms[1].ty, UniformType::Vec3);
    assert!(f
        .source
        .contains("@group(0) @binding(1) var<uniform> n1_uniforms: n1_Uniforms;"));
    assert!(f
        .source
        .contains("struct n1_Uniforms {\n    gain: f32,\n    tint: vec3<f32>,\n}"));
    assert!(f
        .source
        .contains("n1_uniforms.tint * ctx.inputs[0].x * n1_uniforms.gain"));
    let d = Declaration::parse(GAIN_TINT).expect("the schema");
    assert!(
        d.uniforms[0].admits(&value),
        "{value:?} is not a value of the schema"
    );
}

#[test]
fn declaration_a_schema_is_the_nodes_uniforms() {
    check_uniforms(vec![4.0]);
    let d = Declaration::parse("// @uniform gain: f32 = 1.0 [0.0, 4.0]\n// @uniform n: u32 = 1")
        .expect("the schema");
    let (gain, n) = (&d.uniforms[0], &d.uniforms[1]);
    for value in [
        vec![5.0],
        vec![-0.5],
        vec![1.0, 2.0],
        vec![],
        vec![f64::NAN],
    ] {
        assert!(!gain.admits(&value), "gain = {value:?} was taken");
    }
    // An unranged f32 takes any f32, and nothing beyond f32::MAX.
    let free = Declaration::parse("// @uniform k: f32 = 0.0").expect("the schema");
    let k = &free.uniforms[0];
    assert!(k.admits(&[f64::from(f32::MAX)]), "k = f32::MAX was refused");
    assert!(!k.admits(&[1e39]), "k = 1e39 was taken");
    assert!(!k.admits(&[-1e39]), "k = -1e39 was taken");
    assert!(!k.admits(&[f64::INFINITY]), "k = inf was taken");
    assert!(!n.admits(&[1.5]), "n = 1.5 was taken");
    assert!(n.admits(&[7.0]), "an integer");
    // An occupant that declares `uniforms` itself collides with the assembler's.
    let g = graph(
        Occupant::Field("d_min".into()),
        "// @uniform gain: f32 = 1.0\nconst uniforms: f32 = 1.0;\nfn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(uniforms); }",
    );
    let got = assemble::assemble(&stain(&g), Tier::FULL);
    assert!(got.is_err_and(|e| e.to_string().contains("is the assembler's")));
}

negative_control!(
    declaration_a_schema_is_the_nodes_uniforms,
    "a value outside the schema's range is refused",
    expected = "is not a value of the schema",
    check_uniforms(vec![4.5])
);

/// The worked example of gui_state_contract §3, read from the doc: the ```wgsl block after its heading.
fn worked_example() -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/contracts/principia_gui_state_contract.md");
    let doc = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let (_, after) = doc
        .split_once("**Worked example of the declaration format.**")
        .expect("the worked example's heading");
    let (_, block) = after.split_once("```wgsl\n").expect("its ```wgsl block");
    let (text, _) = block.split_once("```").expect("the block's close");
    text.to_owned()
}

/// `text` declares the example's schema and ports, as the doc reads them, and assembles as a colour fed two fields.
fn check_worked_example(text: &str) {
    let d = Declaration::parse(text).unwrap_or_else(|e| panic!("{e}"));
    let got: Vec<String> = d
        .uniforms
        .iter()
        .map(|u| format!("{}:{}={:?}{:?}", u.name, u.ty.wgsl(), u.default, u.range))
        .chain(
            d.inputs
                .iter()
                .map(|i| format!("{}={:?}", i.name, i.domain)),
        )
        .collect();
    assert_eq!(
        got,
        [
            "gain:f32=[1.0]Some((0.0, 4.0))",
            "tint:vec3<f32>=[1.0, 0.5, 0.25]None",
            "t=Some((0.0, 1.0))",
            "mask=None"
        ],
        "the example's declarations"
    );
    let g = vec![
        wired(Kind::Source, Occupant::Field("d_min".into()), &[]),
        wired(Kind::Source, Occupant::Field("ftle".into()), &[]),
        node(Kind::Colour, custom(text), &[Some(0), Some(1)]),
        node(Kind::Combiner, pass_through(), &[Some(2), None]),
        node(Kind::Out, Occupant::None, &[Some(3)]),
    ];
    let f = assemble::assemble(&stain(&g), Tier::FULL).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(f.uniforms.len(), 1, "the example's uniform blocks");
    assert!(
        f.source
            .contains("struct n2_Uniforms {\n    gain: f32,\n    tint: vec3<f32>,\n}"),
        "the example's block"
    );
    assert!(
        f.source.contains(
            "n2_uniforms.tint * clamp(ctx.inputs[0].x * n2_uniforms.gain, 0.0, 1.0) * ctx.inputs[1].x"
        ),
        "the example's reads"
    );
}

#[test]
fn declaration_the_worked_example_is_read_and_assembled() {
    check_worked_example(&worked_example());
}

negative_control!(
    declaration_the_worked_example_is_read_and_assembled,
    "the example's second input is a declaration",
    expected = "the example's declarations",
    check_worked_example(&worked_example().replace("// @input mask\n", ""))
);

/// A built-in source reads one scalar field, and its out-port carries the field's subtype.
fn check_sources(cases: &[(&str, Option<assemble::Subtype>)]) {
    let fields = assemble::source_fields().expect("the fields");
    for &(field, want) in cases {
        let got = assemble::field_subtype(field).ok();
        assert_eq!(got, want, "the subtype of `{field}`");
        assert_eq!(
            fields.iter().any(|f| f == field),
            want.is_some(),
            "whether `{field}` is a source's field"
        );
    }
}

#[test]
fn declaration_a_source_reads_a_scalar_field() {
    use assemble::Subtype::{Categorical, Scalar};
    check_sources(&[
        ("ftle", Some(Scalar)),
        ("state", Some(Categorical)),
        ("is_failed", Some(Categorical)),
        ("saturated", Some(Categorical)),
        ("t_end_step", Some(Scalar)),
        ("word", None),
        ("r", None),
        ("speed", None),
    ]);
    for (kind, occupant) in [
        (Kind::Source, Occupant::Field("word".into())),
        (Kind::Colour, Occupant::Field("ftle".into())),
        (Kind::Colour, Occupant::BuiltIn("nope".into())),
        (Kind::Source, Occupant::BuiltIn("pass_through".into())),
    ] {
        assert!(
            assemble::declaration(kind, &occupant).is_err(),
            "a {kind:?} took {occupant:?}"
        );
    }
    // A bool field reads as 0 or 1, the others as themselves.
    let g = graph(Occupant::Field("is_failed".into()), SHOW_INPUT);
    assert!(assembled(&g, Tier::FULL)
        .contains("Field(select(0.0, 1.0, ctx.sample.is_failed), 0.0, 0.0, 0.0)"));
    let g = graph(Occupant::Field("t_end_step".into()), SHOW_INPUT);
    assert!(assembled(&g, Tier::FULL).contains("Field(f32(ctx.sample.t_end_step), 0.0, 0.0, 0.0)"));
}

negative_control!(
    declaration_a_source_reads_a_scalar_field,
    "a bool field is categorical",
    expected = "the subtype of",
    check_sources(&[("is_failed", Some(assemble::Subtype::Scalar))])
);
