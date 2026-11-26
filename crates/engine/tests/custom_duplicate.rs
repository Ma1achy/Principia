//! Duplicate on any node copies its real source. Pass criteria, written before the run: for a shipped post node and a
//! field's source, the copy holds `Custom` text equal to the original's source, the original is untouched, and the
//! copy holds the original's params; a custom occupant carrying `// @id` is copied with its text intact; `OUT`, which holds no code, is refused as that (not as a missing node), leaving the graph as it was. The control copies the node's
//! occupant as it is (a shipped name), which is no text to edit.

use engine::contract::stain::graph::{NodeId, NodeKind, Occupant, StainGraph};
use render::assemble;
use validation::negative_control;

fn graph(kind: NodeKind, occupant: Occupant) -> (StainGraph, NodeId) {
    let mut g = StainGraph::new();
    let id = g.add(kind, occupant).unwrap_or_else(|e| panic!("{e}"));
    (g, id)
}

fn check(as_custom: bool) {
    let cases = [
        (NodeKind::Post, Occupant::Builtin("edge_line".into())),
        (NodeKind::Source, Occupant::Field("state".into())),
    ];
    for (kind, occupant) in cases {
        let (mut g, id) = graph(kind, occupant.clone());
        let copy = if as_custom {
            g.duplicate_as_custom(id)
        } else {
            g.duplicate(id)
        }
        .unwrap_or_else(|e| panic!("{e}"));
        let original = assemble::occupant_source(kind.into(), &(&occupant).into())
            .unwrap_or_else(|e| panic!("{e}"))
            .expect("code");
        let held = &g.node(copy).expect("the copy").occupant;
        assert_eq!(
            held,
            &Occupant::Custom(original),
            "{kind:?}: the copy is the original's source as text to edit"
        );
        assert_eq!(
            g.node(id).expect("the original").occupant,
            occupant,
            "the original is untouched"
        );
        assert_eq!(
            g.node(copy).expect("the copy").params,
            g.node(id).expect("the original").params
        );
    }
    // A custom occupant carrying `// @id` is copied as it is, `@id` included.
    let text = "// @id mine/x\n// @name X\nfn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> { return rgb; }\n";
    let (mut g, id) = graph(NodeKind::Post, Occupant::Custom(text.into()));
    let copy = if as_custom {
        g.duplicate_as_custom(id)
    } else {
        g.duplicate(id)
    }
    .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        g.node(copy).expect("the copy").occupant,
        Occupant::Custom(text.into()),
        "a custom's text, @id included, is intact"
    );
}

/// OUT, a node of the graph that holds no code, is refused for that reason and the graph is left as it was.
fn check_out(id: NodeId) {
    let mut g = StainGraph::new();
    let before = g.clone();
    assert_eq!(
        g.node(NodeId(1)).map(|n| n.kind),
        Some(NodeKind::Out),
        "the backbone's node 1 is OUT"
    );
    let e = g.duplicate_as_custom(id).expect_err("OUT holds no code");
    assert!(
        e.to_string().contains("holds no code"),
        "OUT is refused as code-less: {e}"
    );
    assert_eq!(g, before, "the refusal changes nothing");
}

#[test]
fn custom_duplicate_refuses_out() {
    check_out(NodeId(1));
}

negative_control!(
    custom_duplicate_refuses_out,
    "a node that does not exist stands in for OUT",
    expected = "OUT is refused as code-less",
    check_out(NodeId(99)),
);

#[test]
fn custom_duplicate_copies_the_real_source() {
    check(true);
}

negative_control!(
    custom_duplicate_copies_the_real_source,
    "a duplicate that keeps the shipped name",
    expected = "the copy is the original's source as text to edit",
    check(false),
);
