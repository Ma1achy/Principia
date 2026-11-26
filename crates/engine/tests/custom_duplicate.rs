//! Duplicate on any node copies its real source. Pass criteria, written before the run: for a shipped post node and a
//! field's source, the copy holds `Custom` text equal to the original's source, the original is untouched, and the
//! copy holds the original's params; a node holding no code (`OUT`) is refused. The control copies the node's
//! occupant as it is (a shipped name), which is no text to edit.

use engine::contract::stain::graph::{NodeKind, Occupant, StainGraph};
use render::assemble;
use validation::negative_control;

fn graph(
    kind: NodeKind,
    occupant: Occupant,
) -> (StainGraph, engine::contract::stain::graph::NodeId) {
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
    let mut g = StainGraph::new();
    assert!(
        g.duplicate_as_custom(engine::contract::stain::graph::NodeId(1))
            .is_err(),
        "OUT holds no code"
    );
}

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
