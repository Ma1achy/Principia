//! A code edit whose values follow its declaration is one checked edit. Pass criteria, written before the run: a value
//! the new declaration no longer admits makes `set_occupant` refuse the edit (the control), while `set_occupant_with`
//! takes the edit with the values given, the declared uniforms holding them alone; a value the new declaration does not
//! admit is refused by `set_occupant_with` too, leaving the graph as it was.

use std::collections::BTreeMap;

use engine::contract::stain::graph::{NodeId, NodeKind, Occupant, StainGraph};
use validation::negative_control;

fn fade(range: &str) -> Occupant {
    Occupant::Custom(format!(
        "// @input t [0, 50]\n// @uniform fade: f32 = 0.25 [{range}]\n\
         fn brightness(ctx: Ctx) -> f32 {{\n    return mix(1.0, uniforms.fade, clamp(ctx.inputs[0].x / 50.0, 0.0, 1.0));\n}}\n"
    ))
}

fn graph() -> (StainGraph, NodeId) {
    let mut g = StainGraph::new();
    let id = g
        .add(NodeKind::Brightness, fade("0, 1"))
        .unwrap_or_else(|e| panic!("{e}"));
    g.set_param(id, "fade", vec![0.9])
        .unwrap_or_else(|e| panic!("{e}"));
    (g, id)
}

fn check_values_follow() {
    let (g, id) = graph();
    let mut plain = g.clone();
    assert!(
        plain.set_occupant(id, fade("0, 0.5")).is_err(),
        "set_occupant took a value outside the new range"
    );
    assert_eq!(plain, g);
    // The old 0.9 stays admitted here, so an edit that kept it would pass its check and only the values differ.
    let mut same_range = g.clone();
    let half = BTreeMap::from([("fade".to_owned(), vec![0.5])]);
    same_range
        .set_occupant_with(id, fade("0, 1"), &half)
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        same_range.node(id).expect("the node").params,
        half,
        "the edit holds the values given"
    );
    let mut with = g.clone();
    let values = BTreeMap::from([("fade".to_owned(), vec![0.5])]);
    with.set_occupant_with(id, fade("0, 0.5"), &values)
        .unwrap_or_else(|e| panic!("{e}"));
    let n = with.node(id).expect("the node");
    assert_eq!(n.occupant, fade("0, 0.5"));
    assert_eq!(n.params, values, "the edit holds the values given");
    let mut refused = g.clone();
    let outside = BTreeMap::from([("fade".to_owned(), vec![0.9])]);
    assert!(refused
        .set_occupant_with(id, fade("0, 0.5"), &outside)
        .is_err());
    assert_eq!(refused, g);
}

#[test]
fn values_follow_the_declaration_in_one_edit() {
    check_values_follow();
}

negative_control!(
    values_follow_the_declaration_in_one_edit,
    "a set_occupant_with that ignores `values` keeps the old 0.9",
    expected = "the edit holds the values given",
    {
        #[cfg(feature = "controls")]
        engine::contract::stain::folder::probe::lifted_in(
            engine::contract::stain::folder::probe::Lift::ValuesIgnored,
            check_values_follow,
        )
    },
);
