//! The stain graph (`engine::stain`; gui_state_contract §5; render_gui_spec Part II §3, §4, §6, §13): its edits, each
//! checked and atomic; its serialised form, checked when read; and its canonical form (lowering contract Part 5;
//! render contract Part 3; REQ-RENDER-075). The assembler's own tests (`crates/render/tests/assemble.rs`) cover the
//! lowering; these cover the graph type the engine defines. Each test registers its negative control (R-176).

use engine::stain::{Canonical, GraphError, NodeId, NodeKind, Occupant, StainGraph, Wire};
use validation::negative_control;

const SHOW_INPUT: &str = "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.inputs[0].x); }";
const BRIGHTNESS: &str = "fn brightness(ctx: Ctx) -> f32 { return ctx.inputs[0].x; }";
const POST: &str = "fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> { return rgb; }";
const COMBINE: &str = "fn combine(rgb: vec3<f32>, b: f32) -> vec3<f32> { return rgb * b; }";
const SOURCE: &str = "fn source(ctx: Ctx) -> Field { return Field(0.5, 0.0, 0.0, 0.0); }";

fn custom(text: &str) -> Occupant {
    Occupant::Custom(text.to_owned())
}

/// Every kind: a source (2) into a colour (3) and a brightness (4), the combiner (0) reading both, two posts (5, 6)
/// and OUT (1).
fn full() -> StainGraph {
    let mut g = StainGraph::new();
    let s = g
        .add(NodeKind::Source, Occupant::Field("ftle".into()))
        .expect("source");
    let c = g.add(NodeKind::Colour, custom(SHOW_INPUT)).expect("colour");
    let b = g
        .add(NodeKind::Brightness, custom(BRIGHTNESS))
        .expect("brightness");
    let p1 = g.add(NodeKind::Post, custom(POST)).expect("post");
    let p2 = g
        .add(NodeKind::Post, custom(&format!("{POST} // 2")))
        .expect("post");
    g.set_occupant(StainGraph::COMBINER, custom(COMBINE))
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
    g
}

/// The canonical form's text with each node's kind, occupant and inputs, short.
fn summary(c: &Canonical) -> Vec<String> {
    c.nodes
        .iter()
        .map(|n| {
            let occupant = match &n.occupant {
                Occupant::None => "none".to_owned(),
                Occupant::Field(f) => format!("field {f}"),
                Occupant::Builtin(id) => format!("builtin {id}"),
                Occupant::Custom(t) => format!("custom {}", t.len()),
            };
            format!("{:?} {occupant} {:?}", n.kind, n.inputs)
        })
        .collect()
}

// ── Construction ──────────────────────────────────────────────────────────────────────────────────────────────────

/// The backbone alone: the pass-through combiner wired to OUT; nodes take the next id.
fn check_new(g: &StainGraph, want: &[Wire]) {
    assert_eq!(g.wires(), want, "the backbone's wires");
    assert_eq!(
        g.node(StainGraph::COMBINER).map(|n| n.kind),
        Some(NodeKind::Combiner)
    );
    assert_eq!(g.node(StainGraph::OUT).map(|n| n.kind), Some(NodeKind::Out));
    assert_eq!(
        g.node(StainGraph::COMBINER).map(|n| n.occupant.clone()),
        Some(Occupant::Builtin("pass_through".into()))
    );
}

#[test]
fn stain_graph_new_is_the_backbone() {
    let g = StainGraph::new();
    check_new(
        &g,
        &[Wire {
            from: StainGraph::COMBINER,
            to: StainGraph::OUT,
            port: 0,
        }],
    );
    assert_eq!(StainGraph::default(), g);
    let mut g = g;
    assert_eq!(g.add(NodeKind::Source, Occupant::None), Ok(NodeId(2)));
    assert_eq!(g.add(NodeKind::Source, Occupant::None), Ok(NodeId(3)));
    g.remove(NodeId(2)).expect("removed");
    assert_eq!(g.add(NodeKind::Source, Occupant::None), Ok(NodeId(4)));
    assert!(g.node(NodeId(2)).is_none());
}

negative_control!(
    stain_graph_new_is_the_backbone,
    "the backbone has a wire",
    expected = "the backbone's wires",
    check_new(&StainGraph::new(), &[])
);

/// Each edit on [`full`] is refused with `want` in its message and leaves the graph as it was.
fn check_refused(cases: &[(&str, &Edit)]) {
    for (want, edit) in cases {
        let mut g = full();
        let before = g.clone();
        let got = edit(&mut g);
        assert!(
            got.as_ref().is_err_and(|e| e.to_string().contains(want)),
            "the edit ({want}) gave {got:?}"
        );
        assert_eq!(g, before, "a refused edit ({want}) changed the graph");
    }
}

type Edit = dyn Fn(&mut StainGraph) -> Result<(), GraphError>;

fn refusals() -> Vec<(&'static str, Box<Edit>)> {
    vec![
        (
            "fixed singleton",
            Box::new(|g: &mut StainGraph| g.add(NodeKind::Combiner, custom(COMBINE)).map(|_| ())),
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
        (
            "names no node",
            Box::new(|g: &mut StainGraph| g.connect(NodeId(9), NodeId(3), 0)),
        ),
        (
            "names no node",
            Box::new(|g: &mut StainGraph| g.connect(NodeId(2), NodeId(9), 0)),
        ),
        (
            "no port 1",
            Box::new(|g: &mut StainGraph| g.connect(NodeId(2), NodeId(3), 1)),
        ),
        (
            "cannot feed",
            Box::new(|g: &mut StainGraph| g.connect(NodeId(5), NodeId(3), 0)),
        ),
        (
            "backbone",
            Box::new(|g: &mut StainGraph| g.connect(NodeId(3), NodeId(5), 0)),
        ),
        (
            "backbone",
            Box::new(|g: &mut StainGraph| g.connect(NodeId(6), StainGraph::COMBINER, 0)),
        ),
        (
            "acyclic",
            Box::new(|g: &mut StainGraph| g.connect(NodeId(6), NodeId(5), 0)),
        ),
        (
            "acyclic",
            Box::new(|g: &mut StainGraph| g.connect(NodeId(5), NodeId(5), 0)),
        ),
        (
            "the combiner is required",
            Box::new(|g: &mut StainGraph| g.set_occupant(StainGraph::COMBINER, Occupant::None)),
        ),
        (
            "does not take",
            Box::new(|g: &mut StainGraph| {
                g.set_occupant(NodeId(3), Occupant::Field("ftle".into()))
            }),
        ),
        (
            "is no value",
            Box::new(|g: &mut StainGraph| g.set_param(NodeId(3), "gain", vec![1.0])),
        ),
    ]
}

#[test]
fn stain_graph_a_bad_edit_is_refused_and_changes_nothing() {
    let cases = refusals();
    let refs: Vec<(&str, &Edit)> = cases.iter().map(|(w, e)| (*w, e.as_ref())).collect();
    check_refused(&refs);
}

/// [`full`] with its chain grown to `posts` posts, all wired, and one more post wired from the last but not to OUT.
fn chain_of(posts: usize) -> (StainGraph, NodeId) {
    let mut g = full();
    let mut previous = NodeId(6);
    for _ in 2..posts {
        let p = g.add(NodeKind::Post, custom(POST)).expect("a post");
        g.connect(previous, p, 0).expect("chained");
        g.connect(p, StainGraph::OUT, 0).expect("to OUT");
        previous = p;
    }
    let p = g.add(NodeKind::Post, custom(POST)).expect("one more");
    g.connect(previous, p, 0).expect("chained, not yet live");
    (g, p)
}

/// The chain of `posts` posts grown by one is refused at the bound and leaves the graph as it was.
fn check_bound(posts: usize) {
    let (mut g, p) = chain_of(posts);
    let before = g.clone();
    let got = g.connect(p, StainGraph::OUT, 0);
    assert!(
        got.as_ref().is_err_and(|e| e.0.contains("at most 8")),
        "{posts} + 1 posts gave {got:?}"
    );
    assert_eq!(g, before);
}

#[test]
fn stain_graph_the_live_chain_holds_eight_posts() {
    check_bound(8);
    let (mut g, p) = chain_of(7);
    g.connect(p, StainGraph::OUT, 0).expect("eight posts");
}

negative_control!(
    stain_graph_the_live_chain_holds_eight_posts,
    "seven posts grown by one are eight",
    expected = "posts gave Ok",
    check_bound(7)
);

negative_control!(
    stain_graph_a_bad_edit_is_refused_and_changes_nothing,
    "a wire from the source into the brightness is taken",
    expected = "gave Ok",
    check_refused(&[("cannot feed", &|g: &mut StainGraph| g.connect(
        NodeId(2),
        NodeId(4),
        0
    ))])
);

/// Edits that keep a graph: last write wins on an in-port; a new occupant drops the wires to ports and the params it
/// lost; removing a node drops its wires; disconnecting empties an in-port.
#[test]
fn stain_graph_edits_keep_the_graph() {
    let mut g = full();
    let t = g
        .add(NodeKind::Source, custom(SOURCE))
        .expect("second source");
    g.connect(t, NodeId(3), 0).expect("last write wins");
    let into = |g: &StainGraph, to: NodeId| -> Vec<(NodeId, usize)> {
        g.wires()
            .iter()
            .filter(|w| w.to == to)
            .map(|w| (w.from, w.port))
            .collect()
    };
    assert_eq!(into(&g, NodeId(3)), [(t, 0)]);
    let two = format!("// @input a\n// @input b\n// @uniform k: f32 = 1.0\n{SHOW_INPUT}");
    g.set_occupant(NodeId(3), custom(&two)).expect("two inputs");
    g.connect(NodeId(2), NodeId(3), 1)
        .expect("the second input");
    g.set_param(NodeId(3), "k", vec![2.0]).expect("a param");
    assert_eq!(into(&g, NodeId(3)), [(t, 0), (NodeId(2), 1)]);
    g.set_occupant(NodeId(3), custom(SHOW_INPUT))
        .expect("one input");
    assert_eq!(
        into(&g, NodeId(3)),
        [(t, 0)],
        "a wire to a lost port stayed"
    );
    assert!(
        g.node(NodeId(3)).is_some_and(|n| n.params.is_empty()),
        "a lost param stayed"
    );
    g.remove(t).expect("removed");
    assert!(
        into(&g, NodeId(3)).is_empty(),
        "a removed node's wire stayed"
    );
    g.connect(NodeId(2), NodeId(3), 0).expect("rewired");
    g.disconnect(NodeId(3), 0).expect("disconnected");
    assert!(into(&g, NodeId(3)).is_empty(), "the wire stayed");
    g.disconnect(NodeId(3), 0)
        .expect("an empty in-port stays empty");
    // Removing a node drops the wires out of it too.
    g.connect(NodeId(2), NodeId(3), 0).expect("rewired");
    g.remove(NodeId(5)).expect("a post removed");
    assert!(
        into(&g, NodeId(6)).is_empty(),
        "a wire from a removed node stayed"
    );
    assert!(g
        .wires()
        .iter()
        .all(|w| w.from != NodeId(5) && w.to != NodeId(5)));
}

negative_control!(
    stain_graph_edits_keep_the_graph,
    "the source's wire into the colour is the colour's only input",
    expected = "left: [",
    {
        let g = full();
        let into: Vec<NodeId> = g
            .wires()
            .iter()
            .filter(|w| w.to == NodeId(3))
            .map(|w| w.from)
            .collect();
        assert_eq!(into, [NodeId(4)]);
    }
);

// ── The serialised form ───────────────────────────────────────────────────────────────────────────────────────────

/// The graph reads back equal; each edit of its JSON text is refused when read.
fn check_serde(edits: &[(&str, &str)]) {
    let g = full();
    let json = serde_json::to_string(&g).expect("serialises");
    assert!(
        json.starts_with(r#"{"nodes":[{"id":0,"kind":"combiner""#),
        "{json}"
    );
    let back: StainGraph = serde_json::from_str(&json).expect("reads back");
    assert_eq!(back, g, "the graph does not read back");
    for (from, to) in edits {
        let broken = json.replacen(from, to, 1);
        assert!(broken != json, "`{from}` is not in {json}");
        let got = serde_json::from_str::<StainGraph>(&broken);
        assert!(got.is_err(), "a broken graph was read: {broken}");
    }
}

#[test]
fn stain_graph_a_serialised_graph_is_checked_when_read() {
    check_serde(&[
        (
            r#"{"from":3,"to":0,"port":0}"#,
            r#"{"from":6,"to":0,"port":0}"#,
        ),
        (r#""kind":"post""#, r#""kind":"out""#),
        (r#""id":6"#, r#""id":5"#),
        (
            r#"{"from":3,"to":0,"port":0}"#,
            r#"{"from":3,"to":0,"port":0},{"from":3,"to":0,"port":0}"#,
        ),
        (r#""params":{}"#, r#""params":{"x":[1.0]}"#),
        (r#""wires":["#, r#""wires":[{"from":1,"to":0,"port":0},"#),
        (r#""params":{}"#, r#""params":{},"extra":1"#),
    ]);
    let text = r#"{"nodes":[{"id":0,"kind":"combiner","occupant":{"builtin":"pass_through"}},{"id":1,"kind":"out","occupant":"none"}],"wires":[{"from":0,"to":1,"port":0}]}"#;
    let g: StainGraph = serde_json::from_str(text).expect("params default to none");
    assert_eq!(g, StainGraph::new());
}

negative_control!(
    stain_graph_a_serialised_graph_is_checked_when_read,
    "white space is no edit of the graph",
    expected = "a broken graph was read",
    check_serde(&[(
        r#"{"from":2,"to":3,"port":0}"#,
        r#"{"from":2,"to":3,"port":0} "#
    )])
);

#[test]
fn stain_graph_errors_name_the_problem() {
    let e = serde_json::from_str::<StainGraph>(
        r#"{"nodes":[{"id":0,"kind":"combiner","occupant":{"builtin":"pass_through"}},{"id":0,"kind":"out","occupant":"none"}],"wires":[]}"#,
    )
    .expect_err("an id twice");
    assert!(e.to_string().contains("node id 0 is used twice"), "{e}");
    let e = serde_json::from_str::<StainGraph>(
        r#"{"nodes":[{"id":0,"kind":"combiner","occupant":{"builtin":"pass_through"}},{"id":1,"kind":"out","occupant":"none"}],"wires":[{"from":0,"to":1,"port":0},{"from":0,"to":1,"port":0}]}"#,
    )
    .expect_err("a port wired twice");
    assert!(
        e.to_string().contains("node 1's in-port 0 has two wires"),
        "{e}"
    );
    let e = serde_json::from_str::<StainGraph>(
        r#"{"nodes":[{"id":0,"kind":"combiner","occupant":{"builtin":"pass_through"}},{"id":2,"kind":"out","occupant":"none"}],"wires":[]}"#,
    )
    .expect_err("OUT is not node 1");
    assert!(e.to_string().contains("fixed singleton, node 1"), "{e}");
    assert_eq!(GraphError("x".into()).to_string(), "x");
}

negative_control!(
    stain_graph_errors_name_the_problem,
    "an error's text is its message",
    expected = "assertion",
    assert_eq!(GraphError("x".into()).to_string(), "y")
);

// ── The canonical form ────────────────────────────────────────────────────────────────────────────────────────────

/// `g`'s canonical form, by kind, occupant and inputs.
fn check_canonical(g: &StainGraph, want: &[&str]) {
    assert_eq!(summary(&g.canonical()), want, "the canonical form");
}

#[test]
fn stain_graph_the_canonical_form_is_the_live_graph_in_order() {
    let g = full();
    check_canonical(
        &g,
        &[
            "Source field ftle []",
            &format!("Colour custom {} [Some(0)]", SHOW_INPUT.len()),
            &format!("Brightness custom {} [Some(0)]", BRIGHTNESS.len()),
            &format!("Combiner custom {} [Some(1), Some(2)]", COMBINE.len()),
            &format!("Post custom {} [Some(3)]", POST.len()),
            &format!("Post custom {} [Some(4)]", POST.len() + 5),
            "Out none [Some(5)]",
        ],
    );
    // The identity drops out: a None colour, a brightness whose source is None, a None post; a post's absent colour
    // reads the combiner.
    let mut h = g.clone();
    h.set_occupant(NodeId(3), Occupant::None)
        .expect("colour None");
    h.set_occupant(NodeId(5), Occupant::None)
        .expect("post None");
    let t = h
        .add(NodeKind::Source, Occupant::None)
        .expect("a None source");
    h.connect(t, NodeId(4), 0).expect("into the brightness");
    check_canonical(
        &h,
        &[
            &format!("Combiner custom {} [None, None]", COMBINE.len()),
            &format!("Post custom {} [Some(0)]", POST.len() + 5),
            "Out none [Some(1)]",
        ],
    );
    let mut k = g.clone();
    k.disconnect(NodeId(6), 0).expect("the chain cut");
    k.add(NodeKind::Brightness, custom(BRIGHTNESS))
        .expect("a dead brightness");
    check_canonical(
        &k,
        &[
            "Source field ftle []",
            &format!("Colour custom {} [Some(0)]", SHOW_INPUT.len()),
            &format!("Brightness custom {} [Some(0)]", BRIGHTNESS.len()),
            &format!("Combiner custom {} [Some(1), Some(2)]", COMBINE.len()),
            &format!("Post custom {} [Some(3)]", POST.len() + 5),
            "Out none [Some(4)]",
        ],
    );
    // Two sources, in order of first use: the colour's, then the brightness's, then a post's field input.
    let mut m = g.clone();
    let u = m
        .add(NodeKind::Source, custom(SOURCE))
        .expect("a second source");
    m.connect(u, NodeId(4), 0).expect("into the brightness");
    m.set_occupant(NodeId(5), custom(&format!("// @input mask\n{POST}")))
        .expect("a post with a field");
    m.connect(NodeId(2), NodeId(5), 1)
        .expect("the first source into the post");
    let w = m
        .add(NodeKind::Source, Occupant::Field("S".into()))
        .expect("a third source");
    m.set_occupant(NodeId(6), custom(&format!("// @input mask\n{POST} // 2")))
        .expect("a second post with a field");
    m.connect(NodeId(6), StainGraph::OUT, 0).expect("unchanged");
    m.connect(w, NodeId(6), 1)
        .expect("the third source into the second post");
    check_canonical(
        &m,
        &[
            "Source field ftle []",
            &format!("Source custom {} []", SOURCE.len()),
            "Source field S []",
            &format!("Colour custom {} [Some(0)]", SHOW_INPUT.len()),
            &format!("Brightness custom {} [Some(1)]", BRIGHTNESS.len()),
            &format!("Combiner custom {} [Some(3), Some(4)]", COMBINE.len()),
            &format!("Post custom {} [Some(5), Some(0)]", POST.len() + 15),
            &format!("Post custom {} [Some(6), Some(2)]", POST.len() + 20),
            "Out none [Some(7)]",
        ],
    );
}

negative_control!(
    stain_graph_the_canonical_form_is_the_live_graph_in_order,
    "the empty backbone is not the full graph",
    expected = "the canonical form",
    check_canonical(&StainGraph::new(), &["Source field ftle []"])
);

/// The canonical text, its keys and the assembler's stain agree with the form.
fn check_keys(g: &StainGraph) {
    let c = g.canonical();
    let text = c.text();
    assert!(
        text.starts_with(r#"{"nodes":[{"inputs":[],"kind":"source","occupant":{"field":"ftle"}}"#),
        "{text}"
    );
    for kind in ["colour", "brightness", "combiner", "post", "out"] {
        assert!(
            text.contains(&format!(r#""kind":"{kind}""#)),
            "no {kind} in {text}"
        );
    }
    assert!(text.contains(r#""occupant":{"custom":"fn colour"#));
    assert!(text.contains(r#""occupant":null"#));
    assert_eq!(
        c.fragment_key(),
        ledger::version::fnv1a64(text.as_bytes()),
        "the fragment key"
    );
    assert_eq!(
        c.render_key(),
        ledger::version::fnv1a64(c.render_text().as_bytes()),
        "the render key"
    );
    assert!(
        c.render_text().contains(r#""params":{}"#),
        "{}",
        c.render_text()
    );
    assert_ne!(c.render_text(), text);
    let stain = g.lower().expect("lowers");
    assert_eq!(stain.nodes().len(), c.nodes.len(), "the stain's nodes");
    assert_eq!(stain.nodes()[1].kind, render::assemble::Kind::Colour);
    assert_eq!(stain.nodes()[6].kind, render::assemble::Kind::Out);
    assert_eq!(
        stain.nodes()[0].occupant,
        render::assemble::Occupant::Field("ftle".into())
    );
    assert_eq!(stain.nodes()[3].inputs, [Some(1), Some(2)]);
    let mut builtin = g.clone();
    builtin
        .set_occupant(
            StainGraph::COMBINER,
            Occupant::Builtin("pass_through".into()),
        )
        .expect("built-in");
    assert!(builtin
        .canonical()
        .text()
        .contains(r#""occupant":{"builtin":"pass_through"}"#));
    assert_eq!(
        builtin.lower().expect("lowers").nodes()[3].occupant,
        render::assemble::Occupant::BuiltIn("pass_through".into())
    );
    let mut none = g.clone();
    none.set_occupant(NodeId(4), Occupant::None)
        .expect("brightness None");
    assert!(none.canonical().text().contains(r#""inputs":[1,null]"#));
    assert_eq!(
        none.lower().expect("lowers").nodes()[1].occupant,
        render::assemble::Occupant::Custom(SHOW_INPUT.into())
    );
}

#[test]
fn stain_graph_the_text_and_keys_are_the_forms() {
    check_keys(&full());
}

negative_control!(
    stain_graph_the_text_and_keys_are_the_forms,
    "the backbone alone has no source",
    expected = "\"nodes\"",
    check_keys(&StainGraph::new())
);

/// Params are the render key's, with the schema's default where unset; never the fragment key's.
fn check_params(set: f64, want_same: bool) {
    let mut g = full();
    g.set_occupant(
        NodeId(3),
        custom(&format!(
            "// @uniform gain: f32 = 1.0 [0.0, 4.0]\n{SHOW_INPUT}"
        )),
    )
    .expect("a schema");
    let before = g.canonical();
    g.set_param(NodeId(3), "gain", vec![set]).expect("set");
    let after = g.canonical();
    assert_eq!(
        before.fragment_key(),
        after.fragment_key(),
        "the fragment key moved"
    );
    assert_eq!(
        before.render_key() == after.render_key(),
        want_same,
        "the render key"
    );
    assert!(
        before.render_text().contains(r#""params":{"gain":[1]}"#),
        "{}",
        before.render_text()
    );
    assert_eq!(after.nodes[1].params.get("gain"), Some(&vec![set]));
}

#[test]
fn stain_graph_params_are_the_render_keys() {
    check_params(2.5, false);
    check_params(1.0, true);
}

negative_control!(
    stain_graph_params_are_the_render_keys,
    "another value moves the render key",
    expected = "the render key",
    check_params(2.5, true)
);
