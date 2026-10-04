//! The one assembler (`render::assemble`; render contract Part 2; lowering contract Part 2, Part 3a, Part 5), with
//! stains built through the engine's stain-graph type (gui_state_contract §5):
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
//! - REQ-RENDER-075: the canonical form — two constructions of one graph hash equal, different graphs differently,
//!   params only in the render key (`canonical_hash_*`);
//! - REQ-GEN-027: the declaration format, `// @uniform` and `// @input` (`declaration_*`).
//!
//! Each test registers its negative control (R-176). Run `assemble_field_set_an_assembled_stain_loads_only_its_fields_words`
//! with `--nocapture` for each backend's loads.

use std::collections::BTreeSet;

use engine::stain::{GraphError, NodeId, NodeKind, Occupant, StainGraph};
use ledger::gen::{prelude, read, rust, wgsl};
use naga::valid::{Capabilities, ModuleInfo, ValidationFlags, Validator};
use naga::Module;
use render::assemble::{
    self, AssembleError, Declaration, Kind, PortType, Stain, Tier, UniformType,
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

/// The backbone with `source` wired into a colour node of `colour`, wired into the combiner: ids 2 and 3.
fn graph(source: Occupant, colour: &str) -> (StainGraph, NodeId, NodeId) {
    let mut g = StainGraph::new();
    let s = g.add(NodeKind::Source, source).expect("a source");
    let c = g.add(NodeKind::Colour, custom(colour)).expect("a colour");
    g.connect(s, c, 0).expect("source → colour");
    g.connect(c, StainGraph::COMBINER, 0)
        .expect("colour → combiner");
    (g, s, c)
}

fn stain(g: &StainGraph) -> Stain {
    g.lower().unwrap_or_else(|e| panic!("{e}"))
}

fn assembled(g: &StainGraph, tier: Tier) -> String {
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

fn post(g: &mut StainGraph) -> NodeId {
    g.add(NodeKind::Post, custom(PASS_POST)).expect("a post")
}

fn colour_node(g: &mut StainGraph) -> NodeId {
    g.add(NodeKind::Colour, custom(SHOW_INPUT))
        .expect("a colour")
}

// ── The backbone (REQ-RENDER-009) ─────────────────────────────────────────────────────────────────────────────────

type Edit = dyn Fn(&mut StainGraph) -> Result<(), GraphError>;

/// Each edit's last step is refused with a message containing its `want`.
fn check_refused(edits: &[(&str, &Edit)]) {
    for (want, edit) in edits {
        let (mut g, ..) = graph(Occupant::Field("d_min".into()), SHOW_INPUT);
        let got = edit(&mut g);
        assert!(
            got.as_ref().is_err_and(|e| e.0.contains(want)),
            "an edit that should be refused ({want}) gave {got:?}"
        );
    }
}

fn backbone_edits() -> Vec<(&'static str, Box<Edit>)> {
    vec![
        // Post fed back into colour: a vec3 out-port into a field in-port.
        (
            "cannot feed",
            Box::new(|g: &mut StainGraph| {
                let p = post(g);
                let c = colour_node(g);
                g.connect(p, c, 0)
            }),
        ),
        // Colour straight into a post, skipping the combiner.
        (
            "backbone",
            Box::new(|g: &mut StainGraph| {
                let p = post(g);
                let c = colour_node(g);
                g.connect(c, p, 0)
            }),
        ),
        // A post into the combiner's colour: post before the combiner.
        (
            "backbone",
            Box::new(|g: &mut StainGraph| {
                let p = post(g);
                g.connect(p, StainGraph::COMBINER, 0)
            }),
        ),
        // Colour straight into OUT.
        (
            "backbone",
            Box::new(|g: &mut StainGraph| {
                let c = colour_node(g);
                g.connect(c, StainGraph::OUT, 0)
            }),
        ),
        // Two posts into each other: a cycle.
        (
            "acyclic",
            Box::new(|g: &mut StainGraph| {
                let a = post(g);
                let b = post(g);
                g.connect(a, b, 0)?;
                g.connect(b, a, 0)
            }),
        ),
        // A second combiner or OUT, or OUT deleted.
        (
            "fixed singleton",
            Box::new(|g: &mut StainGraph| {
                g.add(NodeKind::Combiner, Occupant::Builtin("pass_through".into()))
                    .map(|_| ())
            }),
        ),
        (
            "fixed singleton",
            Box::new(|g: &mut StainGraph| g.add(NodeKind::Out, Occupant::None).map(|_| ())),
        ),
        (
            "fixed singleton",
            Box::new(|g: &mut StainGraph| g.remove(StainGraph::OUT)),
        ),
        (
            "fixed singleton",
            Box::new(|g: &mut StainGraph| g.remove(StainGraph::COMBINER)),
        ),
        // A brightness into the combiner's colour: f32 into vec3.
        (
            "cannot feed",
            Box::new(|g: &mut StainGraph| {
                let b = g.add(NodeKind::Brightness, Occupant::None)?;
                g.connect(b, StainGraph::COMBINER, 0)
            }),
        ),
        // A port the node does not have, and a node that does not exist.
        (
            "no port 1",
            Box::new(|g: &mut StainGraph| g.connect(NodeId(2), NodeId(3), 1)),
        ),
        (
            "names no node",
            Box::new(|g: &mut StainGraph| g.connect(NodeId(9), NodeId(3), 0)),
        ),
        (
            "no node 9",
            Box::new(|g: &mut StainGraph| g.remove(NodeId(9))),
        ),
        (
            "no node 9",
            Box::new(|g: &mut StainGraph| g.set_occupant(NodeId(9), Occupant::None)),
        ),
        (
            "no node 9",
            Box::new(|g: &mut StainGraph| g.set_param(NodeId(9), "x", vec![1.0])),
        ),
        // The combiner's occupant None.
        (
            "the combiner is required",
            Box::new(|g: &mut StainGraph| g.set_occupant(StainGraph::COMBINER, Occupant::None)),
        ),
        // Nine posts in the live chain.
        (
            "at most 8",
            Box::new(|g: &mut StainGraph| {
                let mut previous = StainGraph::COMBINER;
                for _ in 0..=assemble::MAX_POSTS {
                    let p = post(g);
                    g.connect(previous, p, 0)?;
                    previous = p;
                }
                g.connect(previous, StainGraph::OUT, 0)
            }),
        ),
    ]
}

#[test]
fn backbone_a_graph_that_breaks_it_is_refused() {
    let edits = backbone_edits();
    let refs: Vec<(&str, &Edit)> = edits.iter().map(|(w, e)| (*w, e.as_ref())).collect();
    check_refused(&refs);
}

negative_control!(
    backbone_a_graph_that_breaks_it_is_refused,
    "a post wired after the combiner is a graph, not refused",
    expected = "should be refused",
    check_refused(&[("backbone", &|g: &mut StainGraph| {
        let p = post(g);
        g.connect(StainGraph::COMBINER, p, 0)?;
        g.connect(p, StainGraph::OUT, 0)
    })])
);

/// Edits that keep the graph a graph: removing a node drops its wires, a new wire replaces the old on its in-port,
/// and a new occupant with fewer in-ports drops the wires to those it lost.
#[test]
fn backbone_edits_keep_the_graph() {
    let (mut g, s, c) = graph(Occupant::Field("d_min".into()), SHOW_INPUT);
    let t = g
        .add(NodeKind::Source, Occupant::Field("S".into()))
        .expect("a second source");
    g.connect(t, c, 0).expect("last write wins");
    assert!(g
        .wires()
        .iter()
        .any(|w| (w.from, w.to, w.port) == (t, c, 0)));
    assert!(
        !g.wires().iter().any(|w| w.from == s),
        "the old wire stayed"
    );
    g.set_occupant(
        c,
        custom(&format!("// @input a\n// @input b\n{SHOW_INPUT}")),
    )
    .expect("two inputs");
    g.connect(s, c, 1).expect("into the second");
    g.set_occupant(c, custom(SHOW_INPUT))
        .expect("one input again");
    assert!(
        !g.wires().iter().any(|w| w.to == c && w.port == 1),
        "a wire to a lost port stayed"
    );
    g.remove(t).expect("removed");
    assert!(g.node(t).is_none());
    assert!(
        !g.wires().iter().any(|w| w.from == t || w.to == t),
        "a removed node's wire stayed"
    );
    g.disconnect(c, 0).expect("disconnected");
    assert!(!g.wires().iter().any(|w| w.to == c), "the wire stayed");
    assert_eq!(StainGraph::default(), StainGraph::new());
    // A refused edit leaves the graph as it was.
    let before = g.clone();
    assert!(
        g.connect(c, StainGraph::OUT, 0).is_err(),
        "colour → OUT was taken"
    );
    assert_eq!(g, before, "a refused edit changed the graph");
}

negative_control!(
    backbone_edits_keep_the_graph,
    "a wire onto an occupied in-port does not add a second",
    expected = "the old wire stayed",
    {
        let (mut g, s, c) = graph(Occupant::Field("d_min".into()), SHOW_INPUT);
        g.connect(s, c, 0).expect("the same wire again");
        assert!(
            !g.wires().iter().any(|w| w.from == s),
            "the old wire stayed"
        );
    }
);

/// The assembler's own graph refuses a backward wire, the only way to write a cycle, and the backbone's breaks.
fn check_stain_refused(cases: &[(Vec<assemble::Node>, &str)]) {
    for (nodes, want) in cases {
        let got = Stain::new(nodes.clone());
        assert!(
            got.as_ref().is_err_and(|e| e.to_string().contains(want)),
            "a stain that should be refused ({want}) gave {got:?}"
        );
    }
}

fn node(kind: Kind, occupant: assemble::Occupant, inputs: &[Option<usize>]) -> assemble::Node {
    assemble::Node {
        kind,
        occupant,
        inputs: inputs.to_vec(),
    }
}

fn pass_through() -> assemble::Occupant {
    assemble::Occupant::BuiltIn("pass_through".into())
}

fn chain(posts: usize) -> Vec<assemble::Node> {
    let mut v = vec![node(Kind::Combiner, pass_through(), &[None, None])];
    for k in 0..posts {
        v.push(node(
            Kind::Post,
            assemble::Occupant::Custom(PASS_POST.into()),
            &[Some(k)],
        ));
    }
    v.push(node(Kind::Out, assemble::Occupant::None, &[Some(posts)]));
    v
}

fn stain_cases() -> Vec<(Vec<assemble::Node>, &'static str)> {
    use assemble::Occupant as O;
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
                node(Kind::Source, O::Field("d_min".into()), &[]),
                node(Kind::Colour, O::Custom(SHOW_INPUT.into()), &[Some(0)]),
                node(Kind::Combiner, pass_through(), &[None, None]),
                node(Kind::Out, O::None, &[Some(1)]),
            ],
            "backbone",
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

/// A graph with every kind: a source into a colour and a brightness, a combiner reading both, and two posts.
fn full_graph() -> (StainGraph, [NodeId; 5]) {
    let mut g = StainGraph::new();
    let s = g
        .add(NodeKind::Source, Occupant::Field("ftle".into()))
        .expect("source");
    let c = colour_node(&mut g);
    let b = g
        .add(
            NodeKind::Brightness,
            custom("fn brightness(ctx: Ctx) -> f32 { return ctx.inputs[0].x; }"),
        )
        .expect("brightness");
    let p1 = g
        .add(
            NodeKind::Post,
            custom("fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> { return rgb * 0.5; }"),
        )
        .expect("post");
    let p2 = g
        .add(
            NodeKind::Post,
            custom("fn post(ctx: Ctx, rgb: vec3f) -> vec3f { return 1.0 - rgb; }"),
        )
        .expect("post");
    g.set_occupant(
        StainGraph::COMBINER,
        custom("fn combine(rgb: vec3<f32>, b: f32) -> vec3<f32> { return rgb * b; }"),
    )
    .expect("combiner");
    for (from, to, port) in [
        (s, c, 0),
        (s, b, 0),
        (c, StainGraph::COMBINER, 0),
        (b, StainGraph::COMBINER, 1),
        (StainGraph::COMBINER, p1, 0),
        (p1, p2, 0),
        (p2, StainGraph::OUT, 0),
    ] {
        g.connect(from, to, port).expect("a backbone wire");
    }
    (g, [s, c, b, p1, p2])
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
fn check_shade_order(g: &StainGraph, want: &[&str]) {
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
fn check_identity(cases: &[(StainGraph, &str)]) {
    for (g, want) in cases {
        let source = assembled(g, Tier::FULL);
        let body = source.split("fn shade(ctx: Ctx)").nth(1).expect("shade()");
        assert!(
            body.contains(want),
            "shade() does not give `{want}`:\n{body}"
        );
    }
}

fn identity_cases() -> Vec<(StainGraph, &'static str)> {
    let (both, [s, c, b, p1, _]) = full_graph();
    let mut colour_none = both.clone();
    colour_none
        .set_occupant(c, Occupant::None)
        .expect("colour None");
    let mut brightness_none = both.clone();
    brightness_none
        .disconnect(b, 0)
        .expect("brightness dangling");
    let mut neither = colour_none.clone();
    neither.disconnect(b, 0).expect("brightness dangling");
    let mut source_none = both.clone();
    source_none
        .set_occupant(s, Occupant::None)
        .expect("source None");
    let mut post_none = both.clone();
    post_none
        .set_occupant(p1, Occupant::None)
        .expect("post None");
    let mut chain_cut = both.clone();
    chain_cut
        .disconnect(p1, 0)
        .expect("the chain cut before the first post");
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
fn check_same_source(a: &StainGraph, b: &StainGraph) {
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
    let (built_in, ..) = graph(Occupant::Field("d_min".into()), SHOW_INPUT);
    let mut as_custom = built_in.clone();
    let text = assemble::builtin(Kind::Combiner, "pass_through").expect("the built-in");
    as_custom
        .set_occupant(StainGraph::COMBINER, custom(text))
        .expect("the same text, custom");
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
        let (built_in, ..) = graph(Occupant::Field("d_min".into()), SHOW_INPUT);
        let mut other = built_in.clone();
        other
            .set_occupant(
                StainGraph::COMBINER,
                custom("fn combine(rgb: vec3<f32>, b: f32) -> vec3<f32> { return rgb * 2.0; }"),
            )
            .expect("custom");
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
    let (g, ..) = graph(Occupant::Field("ftle".into()), colour);
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

/// Each stain's field set, by its IR, is `want`: the live nodes' reads only.
fn check_field_sets(cases: &[(StainGraph, &[&str])]) {
    for (g, want) in cases {
        let got = assemble::field_set(&stain(g), Tier::FULL).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(got, *want, "the stain's field set");
    }
}

fn field_set_cases() -> Vec<(StainGraph, &'static [&'static str])> {
    let d_min = graph(Occupant::Field("d_min".into()), SHOW_INPUT).0;
    let length = graph(length_source(), SHOW_INPUT).0;
    let mut dead = d_min.clone();
    let s = dead
        .add(NodeKind::Source, Occupant::Field("S".into()))
        .expect("a source nothing reads");
    let c = colour_node(&mut dead);
    dead.connect(s, c, 0).expect("into a colour nothing reads");
    let (reads_three, ..) = graph(Occupant::Field("ftle".into()), READS_ANY_TIER);
    vec![
        (d_min, &["d_min"]),
        (length, &["word.w"]),
        (dead, &["d_min"]),
        (reads_three, &["ensemble_spread", "ftle", "word"]),
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
        graph(Occupant::Field("d_min".into()), SHOW_INPUT).0,
        &["S", "d_min"]
    )])
);

/// `fields` for the stain reading `ftle` is refused, naming the field it misses, never assembled to read 0.
fn check_unfilled(fields: &[&str]) {
    let (g, ..) = graph(Occupant::Field("ftle".into()), SHOW_INPUT);
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
    let (whole, ..) = graph(Occupant::Field("ftle".into()), READS_ANY_TIER);
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
    let (length, ..) = graph(length_source(), SHOW_INPUT);
    assemble::assemble_reading(&stain(&length), Tier::FULL, &["word"])
        .expect("the word holds `.w`");
}

negative_control!(
    assemble_field_set_the_word_and_its_components_cover_each_other,
    "three components are not the word",
    expected = "UnfilledField",
    {
        let (whole, ..) = graph(Occupant::Field("ftle".into()), READS_ANY_TIER);
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
        let (g, ..) = graph(c.source.clone(), SHOW_INPUT);
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
fn check_slots(cases: &[(NodeKind, &str, &str)]) {
    for &(kind, text, want) in cases {
        let (mut g, s, c) = graph(Occupant::Field("d_min".into()), SHOW_INPUT);
        let edited = match kind {
            NodeKind::Brightness => g.add(kind, custom(text)).and_then(|b| {
                g.connect(s, b, 0)?;
                g.connect(b, StainGraph::COMBINER, 1)
            }),
            _ => g.set_occupant(c, custom(text)),
        };
        let got = edited
            .map_err(|e| e.0)
            .and_then(|()| assemble::assemble(&stain(&g), Tier::FULL).map_err(|e| e.to_string()));
        assert!(
            got.as_ref().is_err_and(|e| e.contains(want)),
            "a {kind:?} `{text}` was not refused with `{want}`: {got:?}"
        );
    }
}

fn slot_cases() -> Vec<(NodeKind, &'static str, &'static str)> {
    vec![
        (
            NodeKind::Colour,
            "fn colour(ctx: Ctx) -> f32 { return 1.0; }",
            "a colour slot is `fn(Ctx) -> vec3<f32>`",
        ),
        (
            NodeKind::Colour,
            "fn colour(ctx: Ctx) -> vec4<f32> { return vec4<f32>(1.0); }",
            "a colour slot is `fn(Ctx) -> vec3<f32>`",
        ),
        (
            NodeKind::Colour,
            "fn colour(ctx: Ctx) { }",
            "is `fn(Ctx) -> `; a colour slot",
        ),
        (
            NodeKind::Brightness,
            "fn brightness(ctx: Ctx) -> vec3<f32> { return vec3<f32>(1.0); }",
            "a brightness slot is `fn(Ctx) -> f32`",
        ),
        (
            NodeKind::Brightness,
            "fn brightness(c: vec3<f32>) -> f32 { return c.x; }",
            "is `fn(vec3<f32>) -> f32`; a brightness slot is `fn(Ctx) -> f32`",
        ),
        (
            NodeKind::Brightness,
            "fn brightness(ctx: Ctx, extra: f32) -> f32 { return extra; }",
            "a brightness slot is `fn(Ctx) -> f32`",
        ),
        (
            NodeKind::Colour,
            "fn paint(ctx: Ctx) -> vec3<f32> { return vec3<f32>(1.0); }",
            "defines `fn colour`",
        ),
        (NodeKind::Colour, "fn colour", "is `fn() -> `"),
    ]
}

#[test]
fn slot_signature_colour_is_linear_rgb_and_brightness_a_scalar() {
    check_slots(&slot_cases());
    // The slot signatures assemble, `vec3f` and `vec4f` read as the types they name.
    assemble::assemble(&stain(&full_graph().0), Tier::FULL).expect("the slot signatures");
    let (g, ..) = graph(
        custom("fn source(ctx: Ctx) -> vec4f { return vec4f(1.0); }"),
        "fn colour(ctx: Ctx) -> vec3f { return vec3f(ctx.inputs[0].x); }",
    );
    assemble::assemble(&stain(&g), Tier::FULL).expect("the predeclared aliases");
}

negative_control!(
    slot_signature_colour_is_linear_rgb_and_brightness_a_scalar,
    "a colour returning vec3<f32> is the slot's signature",
    expected = "was not refused",
    check_slots(&[(NodeKind::Colour, SHOW_INPUT, "a colour slot")])
);

/// What a node may not write: a module-scope binding or `var`, an entry point, or a name of the stored buffers.
fn check_occupant_refused(cases: &[(&str, &str)]) {
    for &(text, want) in cases {
        let (g, ..) = graph(
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

/// A node's own names are prefixed — its functions, constants, structs and aliases — and its struct members,
/// swizzles, comments and numbers are not, so two nodes of one text coexist.
fn check_own_names(text: &str, want: &[&str]) {
    let (mut g, ..) = graph(Occupant::Field("d_min".into()), text);
    let p = g
        .add(
            NodeKind::Post,
            custom(&format!(
                "{text}\nfn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> {{ return rgb * helper(Pair(1.0, K)); }}"
            )),
        )
        .expect("a post of the same helpers");
    g.connect(StainGraph::COMBINER, p, 0)
        .expect("combiner → post");
    g.connect(p, StainGraph::OUT, 0).expect("post → OUT");
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

// ── The canonical form (REQ-RENDER-075) ───────────────────────────────────────────────────────────────────────────

/// The full graph built a second way: other ids (a node added and removed first), nodes added and wires made in
/// another order, a dead source, and an identity colour it feeds.
fn full_graph_rebuilt() -> StainGraph {
    let mut g = StainGraph::new();
    let junk = g.add(NodeKind::Post, Occupant::None).expect("junk");
    g.remove(junk).expect("removed");
    let p2 = g
        .add(
            NodeKind::Post,
            custom("fn post(ctx: Ctx, rgb: vec3f) -> vec3f { return 1.0 - rgb; }"),
        )
        .expect("post");
    let b = g
        .add(
            NodeKind::Brightness,
            custom("fn brightness(ctx: Ctx) -> f32 { return ctx.inputs[0].x; }"),
        )
        .expect("brightness");
    let p1 = g
        .add(
            NodeKind::Post,
            custom("fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> { return rgb * 0.5; }"),
        )
        .expect("post");
    let c = colour_node(&mut g);
    let s = g
        .add(NodeKind::Source, Occupant::Field("ftle".into()))
        .expect("source");
    let dead = g
        .add(NodeKind::Source, Occupant::Field("S".into()))
        .expect("dead source");
    let none = g
        .add(NodeKind::Colour, Occupant::None)
        .expect("an identity colour");
    g.connect(dead, none, 0).expect("into nothing live");
    g.set_occupant(
        StainGraph::COMBINER,
        custom("fn combine(rgb: vec3<f32>, b: f32) -> vec3<f32> { return rgb * b; }"),
    )
    .expect("combiner");
    for (from, to, port) in [
        (p2, StainGraph::OUT, 0),
        (p1, p2, 0),
        (StainGraph::COMBINER, p1, 0),
        (b, StainGraph::COMBINER, 1),
        (c, StainGraph::COMBINER, 0),
        (s, b, 0),
        (s, c, 0),
    ] {
        g.connect(from, to, port).expect("a backbone wire");
    }
    g
}

/// Each pair hashes equal, with equal canonical text.
fn check_equal(pairs: &[(StainGraph, StainGraph)]) {
    for (a, b) in pairs {
        let (x, y) = (a.canonical(), b.canonical());
        assert!(
            x.fragment_key() == y.fragment_key() && x.text() == y.text(),
            "one graph hashes differently:\n{}\n{}",
            x.text(),
            y.text()
        );
    }
}

/// The full graph and its variants, each a different graph.
fn variants() -> Vec<StainGraph> {
    let (base, [s, c, b, p1, p2]) = full_graph();
    let mut out = vec![base.clone()];
    let mut edit = |f: &dyn Fn(&mut StainGraph)| {
        let mut g = base.clone();
        f(&mut g);
        out.push(g);
    };
    edit(&|g| {
        g.set_occupant(s, Occupant::Field("d_min".into()))
            .expect("another field")
    });
    edit(&|g| {
        g.set_occupant(c, custom(&format!("{SHOW_INPUT} // edited")))
            .expect("other custom text")
    });
    edit(&|g| {
        g.disconnect(StainGraph::COMBINER, 1)
            .expect("no brightness")
    });
    edit(&|g| {
        // The posts swapped.
        g.connect(StainGraph::COMBINER, p2, 0)
            .expect("combiner → p2");
        g.connect(p2, p1, 0).expect("p2 → p1");
        g.connect(p1, StainGraph::OUT, 0).expect("p1 → OUT");
    });
    edit(&|g| g.connect(p1, StainGraph::OUT, 0).expect("one post"));
    edit(&|g| {
        // The brightness from a second source of the same field.
        let t = g
            .add(NodeKind::Source, Occupant::Field("ftle".into()))
            .expect("second source");
        g.connect(t, b, 0).expect("t → brightness");
    });
    edit(&|g| {
        g.set_occupant(
            StainGraph::COMBINER,
            Occupant::Builtin("pass_through".into()),
        )
        .expect("built-in combiner")
    });
    edit(&|g| g.set_occupant(c, Occupant::None).expect("colour None"));
    out
}

/// Every two graphs hash differently.
fn check_distinct(graphs: &[StainGraph]) {
    let keys: Vec<u64> = graphs
        .iter()
        .map(|g| g.canonical().fragment_key())
        .collect();
    for i in 0..keys.len() {
        for j in 0..i {
            assert!(
                keys[i] != keys[j],
                "graphs {j} and {i} hash the same:\n{}\n{}",
                graphs[j].canonical().text(),
                graphs[i].canonical().text()
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

/// The canonical text of a source into a custom colour, as lowering Part 5 defines it, and its hash.
fn check_text(g: &StainGraph, want: &str) {
    let c = g.canonical();
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
        &graph(Occupant::Field("d_min".into()), SHOW_INPUT).0,
        &defined_text(),
    );
}

negative_control!(
    canonical_hash_the_text_is_the_defined_form,
    "the brightness input absent is `null`, not 0",
    expected = "the canonical text",
    check_text(
        &graph(Occupant::Field("d_min".into()), SHOW_INPUT).0,
        &defined_text().replace("[1,null]", "[1,0]")
    )
);

/// A param edit changes the render key, never the fragment key; the render text carries the value, the schema's
/// default where none is set.
fn check_params(value: f64, same_render_key: bool) {
    let colour = "// @uniform gain: f32 = 1.0 [0.0, 4.0]\nfn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.inputs[0].x * uniforms.gain); }";
    let (a, _, c) = graph(Occupant::Field("d_min".into()), colour);
    let mut b = a.clone();
    b.set_param(c, "gain", vec![value])
        .expect("a value in range");
    let (x, y) = (a.canonical(), b.canonical());
    assert_eq!(
        x.fragment_key(),
        y.fragment_key(),
        "a param edit changed the fragment key"
    );
    assert_eq!(
        x.render_key() == y.render_key(),
        same_render_key,
        "the render key does not follow the param"
    );
    assert!(
        x.render_text().contains(r#""params":{"gain":[1.0]}"#)
            || x.render_text().contains(r#""params":{"gain":[1]}"#)
    );
    assert!(
        !x.text().contains("params"),
        "the params are in the fragment key's text"
    );
    assert_eq!(
        y.render_key(),
        ledger::version::fnv1a64(y.render_text().as_bytes()),
        "the render key is not the render text's hash"
    );
}

#[test]
fn canonical_hash_params_are_in_the_render_key_only() {
    check_params(2.0, false);
    check_params(1.0, true);
}

negative_control!(
    canonical_hash_params_are_in_the_render_key_only,
    "a param edit to another value changes the render key",
    expected = "does not follow the param",
    check_params(3.0, true)
);

/// The graph serialises as its nodes and wires and reads back equal; a serialised graph edited by `text_edit` is
/// refused when read.
fn check_serde(text_edit: (&str, &str)) {
    let (g, ..) = full_graph();
    let json = serde_json::to_string(&g).expect("serialises");
    let back: StainGraph = serde_json::from_str(&json).expect("reads back");
    assert_eq!(back, g, "the graph does not read back");
    let broken = json.replacen(text_edit.0, text_edit.1, 1);
    assert!(
        broken != json,
        "the edit's target is not in the text: {json}"
    );
    let got = serde_json::from_str::<StainGraph>(&broken);
    assert!(got.is_err(), "a broken graph was read: {broken}");
}

#[test]
fn canonical_hash_a_serialised_graph_is_checked_when_read() {
    // The combiner's colour fed from a post: post back into the backbone, and a cycle.
    check_serde((
        r#"{"from":3,"to":0,"port":0}"#,
        r#"{"from":6,"to":0,"port":0}"#,
    ));
    // A second OUT.
    check_serde((r#""kind":"post""#, r#""kind":"out""#));
    // A node id used twice.
    check_serde((r#""id":6"#, r#""id":5"#));
    // Two wires into one in-port.
    check_serde((
        r#"{"from":3,"to":0,"port":0}"#,
        r#"{"from":3,"to":0,"port":0},{"from":3,"to":0,"port":0}"#,
    ));
    // A param no uniform declares.
    check_serde((r#""params":{}"#, r#""params":{"x":[1.0]}"#));
}

negative_control!(
    canonical_hash_a_serialised_graph_is_checked_when_read,
    "the same graph with white space added is read",
    expected = "a broken graph was read",
    check_serde((
        r#"{"from":2,"to":3,"port":0}"#,
        r#"{"from":2,"to":3,"port":0} "#
    ))
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
    ]);
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
        let got = assemble::declaration(*kind, &assemble::Occupant::Custom((*text).into()))
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
        let d = assemble::declaration(kind, &assemble::Occupant::None).expect("None");
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

/// A node with a schema gets its uniform block, at the binding after the prelude's, read as `uniforms.<name>`; a
/// param is set when it is a value of the schema.
fn check_uniforms(value: Vec<f64>) {
    let colour = "// @uniform gain: f32 = 1.0 [0.0, 4.0]\n// @uniform tint: vec3<f32> = (1.0, 0.5, 0.25)\nfn colour(ctx: Ctx) -> vec3<f32> { return uniforms.tint * ctx.inputs[0].x * uniforms.gain; }";
    let mut g = StainGraph::new();
    let s = g
        .add(NodeKind::Source, Occupant::Field("d_min".into()))
        .expect("source");
    let b = g
        .add(
            NodeKind::Brightness,
            custom("// @uniform lift: f32 = 0.0\nfn brightness(ctx: Ctx) -> f32 { return ctx.inputs[0].x + uniforms.lift; }"),
        )
        .expect("brightness");
    let c = g.add(NodeKind::Colour, custom(colour)).expect("colour");
    for (from, to, port) in [
        (s, c, 0),
        (s, b, 0),
        (c, StainGraph::COMBINER, 0),
        (b, StainGraph::COMBINER, 1),
    ] {
        g.connect(from, to, port).expect("wired");
    }
    let f = assemble::assemble(&stain(&g), Tier::FULL).unwrap_or_else(|e| panic!("{e}"));
    let first = prelude::uniforms_binding();
    let blocks: Vec<(usize, u32, u32, usize)> = f
        .uniforms
        .iter()
        .map(|u| (u.node, u.group, u.binding, u.uniforms.len()))
        .collect();
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
    g.set_param(c, "gain", value)
        .expect("a value of the schema");
}

#[test]
fn declaration_a_schema_is_the_nodes_uniforms() {
    check_uniforms(vec![4.0]);
    let (mut g, _, c) = graph(
        Occupant::Field("d_min".into()),
        "// @uniform gain: f32 = 1.0 [0.0, 4.0]\n// @uniform n: u32 = 1\nfn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(uniforms.gain); }",
    );
    for (name, value) in [
        ("gain", vec![5.0]),
        ("gain", vec![-0.5]),
        ("gain", vec![1.0, 2.0]),
        ("gain", vec![]),
        ("bias", vec![1.0]),
        ("gain", vec![f64::NAN]),
        ("n", vec![1.5]),
    ] {
        let before = g.clone();
        assert!(
            g.set_param(c, name, value.clone()).is_err(),
            "`{name}` = {value:?} was taken"
        );
        assert_eq!(g, before);
    }
    g.set_param(c, "n", vec![7.0]).expect("an integer");
    // A new occupant drops the params its schema no longer declares.
    g.set_occupant(c, custom("// @uniform n: u32 = 1\nfn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(f32(uniforms.n)); }"))
        .expect("another schema");
    assert_eq!(
        g.node(c).map(|n| n.params.len()),
        Some(1),
        "a param of the old schema stayed"
    );
    // An occupant that declares `uniforms` itself collides with the assembler's.
    let (g, ..) = graph(
        Occupant::Field("d_min".into()),
        "// @uniform gain: f32 = 1.0\nconst uniforms: f32 = 1.0;\nfn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(uniforms); }",
    );
    let got = assemble::assemble(&stain(&g), Tier::FULL);
    assert!(got.is_err_and(|e| e.to_string().contains("is the assembler's")));
}

negative_control!(
    declaration_a_schema_is_the_nodes_uniforms,
    "a value outside the schema's range is refused",
    expected = "a value of the schema",
    check_uniforms(vec![4.5])
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
    let mut g = StainGraph::new();
    for (kind, occupant) in [
        (NodeKind::Source, Occupant::Field("word".into())),
        (NodeKind::Colour, Occupant::Field("ftle".into())),
        (NodeKind::Colour, Occupant::Builtin("nope".into())),
        (NodeKind::Source, Occupant::Builtin("pass_through".into())),
    ] {
        assert!(
            g.add(kind, occupant.clone()).is_err(),
            "a {kind:?} took {occupant:?}"
        );
    }
    // A bool field reads as 0 or 1, the others as themselves.
    let (g, ..) = graph(Occupant::Field("is_failed".into()), SHOW_INPUT);
    assert!(assembled(&g, Tier::FULL)
        .contains("Field(select(0.0, 1.0, ctx.sample.is_failed), 0.0, 0.0, 0.0)"));
    let (g, ..) = graph(Occupant::Field("t_end_step".into()), SHOW_INPUT);
    assert!(assembled(&g, Tier::FULL).contains("Field(f32(ctx.sample.t_end_step), 0.0, 0.0, 0.0)"));
}

negative_control!(
    declaration_a_source_reads_a_scalar_field,
    "a bool field is categorical",
    expected = "the subtype of",
    check_sources(&[("is_failed", Some(assemble::Subtype::Scalar))])
);
