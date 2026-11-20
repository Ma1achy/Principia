//! QA's tests for TASK-M1-04 on the engine's stain-graph type, written from the requirements:
//! - REQ-RENDER-009 (render contract Part 2; render_gui_spec Part II §4, §6; R-64): no edit and no loaded graph can
//!   reorder the backbone, feed post back into colour, close a cycle or add or delete the fixed singletons; a refused
//!   edit leaves the graph as it was (`qa_stain_graph_*`);
//! - REQ-RENDER-075 (lowering Part 5; render contract Part 3): two constructions of one graph, ids and order of
//!   construction different and dead nodes added, have one form and one fragment key and render key; params move the
//!   render key and never the fragment key; a param left at its default is the default; both keys are the 64-bit FNV-1a
//!   of their texts, and the render text is JCS (`qa_canonical_hash_*`);
//! - REQ-GEN-027 (gui_state_contract §3): a param outside its declared range, or of no declared uniform, is refused
//!   (`qa_declaration_*`).
//!
//! Every test has its negative control (R-176).

use engine::stain::{NodeId, NodeKind, Occupant, StainGraph};
use serde_json::Value;
use validation::negative_control;

const COMBINER: NodeId = NodeId(0);
const OUT: NodeId = NodeId(1);

fn custom(text: &str) -> Occupant {
    Occupant::Custom(text.to_owned())
}

fn src(v: f32) -> Occupant {
    custom(&format!(
        "fn source(ctx: Ctx) -> Field {{ return Field({v:?}, 0.0, 0.0, 0.0); }}"
    ))
}

const COLOUR: &str = "// @uniform gain: f32 = 0.5 [0.0, 2.0]\nfn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.inputs[0].x * uniforms.gain); }";
const BRIGHT: &str = "fn brightness(ctx: Ctx) -> f32 { return ctx.inputs[0].x; }";
const POST_A: &str =
    "fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> { return rgb + vec3<f32>(0.1); }";
const POST_B: &str = "fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> { return rgb * 2.0; }";
const COMBINE: &str = "fn combine(rgb: vec3<f32>, b: f32) -> vec3<f32> { return rgb * b; }";

fn ok<T>(r: Result<T, engine::stain::GraphError>) -> T {
    r.unwrap_or_else(|e| panic!("a legal edit refused: {e}"))
}

/// A full graph, built source-first: s(0.3) → colour, s(0.7) → brightness, multiply, posts A then B, OUT. Its ids.
struct Built {
    g: StainGraph,
    colour: NodeId,
    post_a: NodeId,
    post_b: NodeId,
}

fn forward() -> Built {
    let mut g = StainGraph::new();
    ok(g.set_occupant(COMBINER, custom(COMBINE)));
    let s0 = ok(g.add(NodeKind::Source, src(0.3)));
    let s1 = ok(g.add(NodeKind::Source, src(0.7)));
    let c = ok(g.add(NodeKind::Colour, custom(COLOUR)));
    let b = ok(g.add(NodeKind::Brightness, custom(BRIGHT)));
    ok(g.connect(s0, c, 0));
    ok(g.connect(s1, b, 0));
    ok(g.connect(c, COMBINER, 0));
    ok(g.connect(b, COMBINER, 1));
    let pa = ok(g.add(NodeKind::Post, custom(POST_A)));
    let pb = ok(g.add(NodeKind::Post, custom(POST_B)));
    ok(g.connect(COMBINER, pa, 0));
    ok(g.connect(pa, pb, 0));
    ok(g.connect(pb, OUT, 0));
    Built {
        g,
        colour: c,
        post_a: pa,
        post_b: pb,
    }
}

/// The same graph built backward: posts first, then the brightness, a dead colour and a dead source, the live colour,
/// the sources last; with a None post wired into the chain.
fn backward() -> StainGraph {
    let mut g = StainGraph::new();
    let pb = ok(g.add(NodeKind::Post, custom(POST_B)));
    let none = ok(g.add(NodeKind::Post, Occupant::None));
    let pa = ok(g.add(NodeKind::Post, custom(POST_A)));
    ok(g.connect(pb, OUT, 0));
    ok(g.connect(none, pb, 0));
    ok(g.connect(pa, none, 0));
    ok(g.connect(COMBINER, pa, 0));
    let b = ok(g.add(NodeKind::Brightness, custom(BRIGHT)));
    let dead_c = ok(g.add(NodeKind::Colour, custom(COLOUR)));
    let dead_s = ok(g.add(NodeKind::Source, src(0.9)));
    ok(g.connect(dead_s, dead_c, 0));
    let c = ok(g.add(NodeKind::Colour, custom(COLOUR)));
    let s1 = ok(g.add(NodeKind::Source, src(0.7)));
    let s0 = ok(g.add(NodeKind::Source, src(0.3)));
    ok(g.connect(s1, b, 0));
    ok(g.connect(s0, c, 0));
    ok(g.connect(b, COMBINER, 1));
    ok(g.connect(c, COMBINER, 0));
    ok(g.set_occupant(COMBINER, custom(COMBINE)));
    g
}

// ── REQ-RENDER-009 on the editable graph ───────────────────────────────────────────────────────────────────────────

type Edit = Box<dyn Fn(&mut StainGraph, &Built) -> Result<(), engine::stain::GraphError>>;

fn breakers() -> Vec<(&'static str, Edit)> {
    vec![
        (
            "add a second combiner",
            Box::new(|g, _| g.add(NodeKind::Combiner, custom(COMBINE)).map(|_| ())),
        ),
        (
            "add a second OUT",
            Box::new(|g, _| g.add(NodeKind::Out, Occupant::None).map(|_| ())),
        ),
        ("delete the combiner", Box::new(|g, _| g.remove(COMBINER))),
        ("delete OUT", Box::new(|g, _| g.remove(OUT))),
        (
            "feed a post into the combiner's colour",
            Box::new(|g, b| g.connect(b.post_b, COMBINER, 0)),
        ),
        (
            "feed a post into a colour",
            Box::new(|g, b| g.connect(b.post_a, b.colour, 0)),
        ),
        (
            "feed the colour straight to OUT",
            Box::new(|g, b| g.connect(b.colour, OUT, 0)),
        ),
        (
            "feed the colour into a post",
            Box::new(|g, b| g.connect(b.colour, b.post_a, 0)),
        ),
        (
            "close the post chain on itself",
            Box::new(|g, b| g.connect(b.post_b, b.post_a, 0)),
        ),
        (
            "feed a post into itself",
            Box::new(|g, b| g.connect(b.post_a, b.post_a, 0)),
        ),
        (
            "wire into a port the node does not have",
            Box::new(|g, b| g.connect(COMBINER, b.post_a, 1)),
        ),
        (
            "make the combiner None",
            Box::new(|g, _| g.set_occupant(COMBINER, Occupant::None)),
        ),
        (
            "give OUT an occupant",
            Box::new(|g, _| g.set_occupant(OUT, custom(POST_A))),
        ),
        (
            "a ninth live post",
            Box::new(|g, b| {
                let mut prev = b.post_b;
                for _ in 0..7 {
                    let p = g.add(NodeKind::Post, custom(POST_A))?;
                    g.connect(prev, p, 0)?;
                    prev = p;
                }
                g.connect(prev, OUT, 0)
            }),
        ),
    ]
}

fn check_refused(cases: Vec<(&'static str, Edit)>) {
    for (what, edit) in cases {
        let built = forward();
        let mut g = built.g.clone();
        let before = g.clone();
        assert!(
            edit(&mut g, &built).is_err(),
            "{what}: accepted, but the backbone forbids it"
        );
        // The last step of a multi-step edit is the refused one; the single-step edits leave the graph as it was.
        if what != "a ninth live post" {
            assert_eq!(g, before, "{what}: the refused edit changed the graph");
        }
    }
}

#[test]
fn qa_stain_graph_no_edit_breaks_the_backbone() {
    check_refused(breakers());
    // Eight live posts are legal.
    let built = forward();
    let mut g = built.g.clone();
    let mut prev = built.post_b;
    for _ in 0..6 {
        let p = ok(g.add(NodeKind::Post, custom(POST_A)));
        ok(g.connect(prev, p, 0));
        prev = p;
    }
    ok(g.connect(prev, OUT, 0));
}

negative_control!(
    qa_stain_graph_no_edit_breaks_the_backbone,
    "a legal edit, wiring the colour to the combiner again, offered as a breaker",
    expected = "accepted, but the backbone forbids it",
    check_refused(vec![(
        "rewire the colour",
        Box::new(|g: &mut StainGraph, b: &Built| g.connect(b.colour, COMBINER, 0)) as Edit,
    )])
);

/// A loaded graph is checked as an edit is: `wire` added to a legal graph's serialised form.
fn check_loaded_refused(wire: impl Fn(&Built) -> Value) {
    let built = forward();
    let mut v = serde_json::to_value(&built.g).expect("serialises");
    v["wires"]
        .as_array_mut()
        .expect("wires")
        .retain(|w| !(w["to"] == serde_json::json!(COMBINER.0) && w["port"] == 0));
    v["wires"].as_array_mut().expect("wires").push(wire(&built));
    let back: Result<StainGraph, _> = serde_json::from_value(v);
    assert!(
        back.is_err(),
        "a loaded graph wired post → combiner colour is accepted"
    );
}

#[test]
fn qa_stain_graph_a_loaded_graph_cannot_break_the_backbone() {
    // The legal graph round-trips.
    let built = forward();
    let text = serde_json::to_string(&built.g).expect("serialises");
    let back: StainGraph = serde_json::from_str(&text).expect("a legal graph loads");
    assert_eq!(back, built.g);
    check_loaded_refused(|b| serde_json::json!({"from": b.post_b.0, "to": COMBINER.0, "port": 0}));
}

negative_control!(
    qa_stain_graph_a_loaded_graph_cannot_break_the_backbone,
    "the colour, rewired to the combiner, loads",
    expected = "is accepted",
    check_loaded_refused(|b| serde_json::json!({"from": b.colour.0, "to": COMBINER.0, "port": 0}))
);

// ── REQ-RENDER-075: the canonical form's keys ──────────────────────────────────────────────────────────────────────

/// FNV-1a, 64-bit, from its definition (offset basis 0xcbf29ce484222325, prime 0x100000001b3).
fn fnv1a64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

fn check_same(a: &StainGraph, b: &StainGraph) {
    let (ca, cb) = (ok(a.canonical()), ok(b.canonical()));
    assert!(
        ca.text() == cb.text(),
        "two constructions of one graph differ:\n{}\n{}",
        ca.text(),
        cb.text()
    );
    assert_eq!(
        ca.fragment_key(),
        cb.fragment_key(),
        "two constructions of one graph: fragment keys differ"
    );
    assert_eq!(
        ca.render_key(),
        cb.render_key(),
        "two constructions of one graph: render keys differ"
    );
}

#[test]
fn qa_canonical_hash_construction_order_and_ids_are_not_in_it() {
    let f = forward().g;
    let b = backward();
    check_same(&f, &b);
    // A dead node added and removed again, so the ids shift, changes nothing.
    let mut g = forward().g;
    let dead = ok(g.add(NodeKind::Source, src(0.1)));
    ok(g.remove(dead));
    check_same(&f, &g);
}

negative_control!(
    qa_canonical_hash_construction_order_and_ids_are_not_in_it,
    "posts A and B swapped is another graph",
    expected = "two constructions of one graph differ",
    {
        let built = forward();
        let mut g = built.g.clone();
        ok(g.connect(COMBINER, built.post_b, 0));
        ok(g.connect(built.post_b, built.post_a, 0));
        ok(g.connect(built.post_a, OUT, 0));
        check_same(&built.g, &g)
    }
);

fn check_keys(gain: f64, render_moves: bool) {
    let built = forward();
    let base = ok(built.g.canonical());
    let mut g = built.g.clone();
    ok(g.set_param(built.colour, "gain", vec![gain]));
    let c = ok(g.canonical());
    assert_eq!(
        c.fragment_key(),
        base.fragment_key(),
        "a param edit moved the fragment key: params are uniforms (lowering Part 5)"
    );
    assert_eq!(
        c.render_key() != base.render_key(),
        render_moves,
        "gain {gain}: the render key {} with the param (render contract Part 3)",
        if render_moves {
            "does not move"
        } else {
            "moves"
        }
    );
    assert_eq!(
        c.fragment_key(),
        fnv1a64(c.text().as_bytes()),
        "the fragment key is not FNV-1a 64 of the text"
    );
    assert_eq!(
        c.render_key(),
        fnv1a64(c.render_text().as_bytes()),
        "the render key is not FNV-1a 64 of its text"
    );
}

#[test]
fn qa_canonical_hash_params_move_the_render_key_only() {
    check_keys(1.25, true);
    // The schema's default where none is set: setting the default is no change.
    check_keys(0.5, false);
}

negative_control!(
    qa_canonical_hash_params_move_the_render_key_only,
    "a changed gain claimed not to move the render key",
    expected = "the render key moves with the param",
    check_keys(1.25, false)
);

/// The render text is JCS (RFC 8785): it re-serialises to itself with keys sorted and no white space, each live node
/// carrying `params`, a number written in its shortest form (`1`, not `1.0`).
fn check_render_text(gain: f64, want: &str) {
    let built = forward();
    let mut g = built.g.clone();
    ok(g.set_param(built.colour, "gain", vec![gain]));
    let text = ok(g.canonical()).render_text();
    let v: Value = serde_json::from_str(&text).expect("the render text is JSON");
    let nodes = v["nodes"].as_array().expect("nodes");
    for node in nodes {
        let keys: Vec<&String> = node
            .as_object()
            .expect("a node is an object")
            .keys()
            .collect();
        assert_eq!(
            keys,
            ["inputs", "kind", "occupant", "params"],
            "a node's keys"
        );
    }
    let kinds: Vec<&str> = nodes.iter().map(|n| n["kind"].as_str().unwrap()).collect();
    assert_eq!(
        kinds,
        [
            "source",
            "source",
            "colour",
            "brightness",
            "combiner",
            "post",
            "post",
            "out"
        ],
        "lowering Part 5's order"
    );
    assert_eq!(
        serde_json::to_string(&v).expect("re-serialises"),
        text,
        "the render text is not in sorted-key, white-space-free form"
    );
    assert!(
        text.contains(want),
        "the render text does not hold `{want}`:\n{text}"
    );
}

#[test]
fn qa_canonical_hash_the_render_text_is_jcs() {
    check_render_text(1.0, r#""params":{"gain":[1]}"#);
}

negative_control!(
    qa_canonical_hash_the_render_text_is_jcs,
    "a number written `1.0` is not JCS",
    expected = "the render text does not hold",
    check_render_text(1.0, r#""params":{"gain":[1.0]}"#)
);

// ── REQ-GEN-027: params are values of the schema ──────────────────────────────────────────────────────────────────

fn check_params_refused(cases: &[(&str, Vec<f64>)]) {
    for (name, value) in cases {
        let built = forward();
        let mut g = built.g.clone();
        assert!(
            g.set_param(built.colour, name, value.clone()).is_err(),
            "`{name}` = {value:?} accepted, but gui_state_contract §3 refuses it"
        );
        assert_eq!(g, built.g, "a refused param changed the graph");
    }
}

#[test]
fn qa_declaration_a_param_outside_its_schema_is_refused() {
    check_params_refused(&[
        ("gain", vec![2.5]),
        ("gain", vec![-0.1]),
        ("gain", vec![0.5, 0.5]),
        ("gain", vec![f64::NAN]),
        ("gain", vec![]),
        ("other", vec![0.5]),
    ]);
    let built = forward();
    let mut g = built.g.clone();
    ok(g.set_param(built.colour, "gain", vec![2.0]));
    ok(g.set_param(built.colour, "gain", vec![0.0]));
}

negative_control!(
    qa_declaration_a_param_outside_its_schema_is_refused,
    "a gain inside its range offered as refused",
    expected = "accepted, but",
    check_params_refused(&[("gain", vec![1.0])])
);
