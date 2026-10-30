//! The stain graph model (render_gui_spec Part II §3 to §7, §15; gui_state_contract §5; colour_composition §9.4; R-64,
//! R-52, R-546): its node kinds' ports, its fixed singletons, its wire rules and its serialisation with `RenderState`,
//! and the custom-stop ramp. Each test registers its negative control (R-176).
//!
//! - `stain_node_kinds` (REQ-GUI-021): each kind's in-ports and out-ports are §3.1's row.
//! - `stain_backbone_singletons` (REQ-GUI-022): adding, deleting or duplicating `OUT` or the combiner is refused, and
//!   the graph always holds one of each.
//! - `stain_wires_acyclic` (REQ-GUI-023, a property): over random wire sequences the graph stays acyclic, every
//!   accepted wire type-checks, and a wire that would close a cycle is refused.
//! - `stain_wire_types` (REQ-GUI-025): `vec3 → f32` and `field → vec3` are refused and change nothing; matching drops
//!   attach; a categorical field drops onto a gradient colour node.
//! - `stain_port_arity` (REQ-GUI-026): a second wire on an in-port replaces the first; an out-port feeds five.
//! - `stain_renderstate_roundtrip` (REQ-GUI-016): a graph round-trips through `RenderState`; picking an occupant is one
//!   `SetField`, one undo entry.
//! - `stain_custom_stop_ramp` (REQ-COL-140): a ramp round-trips through `RenderState` and evaluates as §9.4 defines it,
//!   interpolated in OKLab, clamped, or wrapped where cyclic.
//! - `stain_ramp_wrap_is_finite` (REQ-COL-140, a property): a cyclic ramp's wrap holds at stops on or a hair from the
//!   ends, and every finite `t` on a valid cyclic ramp gives a finite colour within its stops' colours.
//!
//! The graphs, fields, ramps and `t` values are test inputs, not register constants (REQ-SYS-082).

use std::collections::BTreeSet;

use proptest::prelude::*;
use render::assemble::{self, Subtype as FieldSubtype};

use crate::contract::interface::EngineInterface;
use crate::contract::render_state::{Hover, Overlays, Palette, Playhead, RenderState};
use crate::contract::set_field::{Edit, RenderField, SetField};
use crate::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, KernelVariant, Links, Lock, Plane, Quality, SimConfig,
    Slice, SliceFrame, Window, Z_Q3,
};
use crate::contract::stain::edit::{apply, Action, Refusal, StainEdit};
use crate::contract::stain::graph::{NodeId, NodeKind, Occupant, StainGraph};
use crate::contract::stain::ports::{Direction, Port, PortType, Ports, Quotient, Subtype};
use crate::contract::stain::ramp::{oklab_to_linear, Ramp, Stop};
use crate::contract::store::{stain_graph, StateStore};

/// The ids [`StainGraph::new`] gives the combiner and `OUT`.
const COMBINER: NodeId = NodeId(0);
const OUT: NodeId = NodeId(1);

/// A colour occupant declaring three field inputs.
const COLOUR3: &str = "// @input a
// @input b
// @input c
fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.inputs[0].x, ctx.inputs[1].x, ctx.inputs[2].x); }
";

/// A post occupant declaring two field inputs after its colour.
const POST2: &str = "// @input m
// @input n
fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> { return rgb * ctx.inputs[0].x * ctx.inputs[1].x; }
";

/// A colour occupant showing its one field input as grey: not the identity, so it is live in the canonical form.
const SHOW: &str = "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.inputs[0].x); }";

/// A brightness occupant declaring one uniform, `gain`, in `[0, 4]`.
const GAIN: &str = "// @uniform gain: f32 = 1.0 [0.0, 4.0]
fn brightness(ctx: Ctx) -> f32 { return ctx.inputs[0].x * uniforms.gain; }
";

fn custom(text: &str) -> Occupant {
    Occupant::Custom(text.to_owned())
}

/// A source of the field `f`.
fn field(f: &str) -> Occupant {
    Occupant::Field(f.to_owned())
}

/// The first field a built-in source reads whose subtype is `want`.
fn field_of(want: FieldSubtype) -> String {
    assemble::source_fields()
        .expect("the ledger's fields")
        .into_iter()
        .find(|f| assemble::field_subtype(f) == Ok(want))
        .unwrap_or_else(|| panic!("no source field is {want:?}"))
}

/// `edit` applied to `g`, accepted: the graph its `SetField` writes.
fn accepted(g: &StainGraph, edit: StainEdit) -> StainGraph {
    let set = apply(g, &edit).unwrap_or_else(|r| panic!("{edit:?} was refused: {r}"));
    match set.edit {
        Edit::Render(RenderField::StainGraph(next)) => next,
        other => panic!("{edit:?} wrote {other:?}, not the stain graph"),
    }
}

/// `g` with a node of `kind` holding `occupant` added, and the new node's id.
fn add(g: &StainGraph, kind: NodeKind, occupant: Occupant) -> (StainGraph, NodeId) {
    let next = accepted(g, StainEdit::Add { kind, occupant });
    let id = next.nodes().map(|n| n.id).max().expect("a node");
    (next, id)
}

fn wire(g: &StainGraph, from: NodeId, to: NodeId, port: u32) -> StainGraph {
    accepted(g, StainEdit::Wire { from, to, port })
}

// ----- stain_node_kinds (REQ-GUI-021) -----

/// A port in `direction` of type `ty`.
fn port(direction: Direction, ty: PortType, subtype: Option<Subtype>) -> Port {
    Port {
        direction,
        ty,
        subtype,
    }
}

/// A row of §3.1: the in-port types and the out-port types.
type Row = (Vec<PortType>, Vec<PortType>);

/// Checks node `id`'s ports against `row`: each in-port an in, each out-port an out, the types in order.
fn check_row(g: &StainGraph, id: NodeId, row: &Row) {
    let Ports { inputs, outputs } = g.ports_of(id).expect("the node's ports");
    assert!(
        inputs.iter().all(|p| p.direction == Direction::In),
        "node {}: an in-port is not an in",
        id.0
    );
    assert!(
        outputs.iter().all(|p| p.direction == Direction::Out),
        "node {}: an out-port is not an out",
        id.0
    );
    let got: Row = (
        inputs.iter().map(|p| p.ty).collect(),
        outputs.iter().map(|p| p.ty).collect(),
    );
    assert_eq!(&got, row, "node {}'s ports are not §3.1's row", id.0);
}

/// One node of each kind, two more with declared inputs, and §3.1's row for each.
fn kinds() -> (StainGraph, Vec<(NodeId, Row)>) {
    use PortType::{Field, Vec3, F32};
    let g = StainGraph::new();
    let (g, source) = add(&g, NodeKind::Source, field("ftle"));
    let (g, colour) = add(&g, NodeKind::Colour, Occupant::None);
    let (g, colour3) = add(&g, NodeKind::Colour, custom(COLOUR3));
    let (g, brightness) = add(&g, NodeKind::Brightness, Occupant::None);
    let (g, post) = add(&g, NodeKind::Post, Occupant::None);
    let (g, post2) = add(&g, NodeKind::Post, custom(POST2));
    let rows = vec![
        (source, (vec![], vec![Field])),
        (colour, (vec![Field], vec![Vec3])),
        (colour3, (vec![Field, Field, Field], vec![Vec3])),
        (brightness, (vec![Field], vec![F32])),
        (COMBINER, (vec![Vec3, F32], vec![Vec3])),
        (post, (vec![Vec3], vec![Vec3])),
        (post2, (vec![Vec3, Field, Field], vec![Vec3])),
        (OUT, (vec![Vec3], vec![])),
    ];
    (g, rows)
}

#[test]
fn stain_node_kinds() {
    let (g, rows) = kinds();
    for (id, row) in &rows {
        check_row(&g, *id, row);
    }
    // A source's out-port carries its field's subtype (§5): a scalar field, then a categorical one.
    let scalar = field_of(FieldSubtype::Scalar);
    let categorical = field_of(FieldSubtype::Categorical);
    for (f, want) in [
        (scalar, Subtype::Scalar),
        (categorical, Subtype::Categorical),
    ] {
        let (g, s) = add(&StainGraph::new(), NodeKind::Source, field(&f));
        let out = g.ports_of(s).expect("ports").outputs;
        assert_eq!(
            out,
            vec![port(Direction::Out, PortType::Field, Some(want))],
            "the source of `{f}`"
        );
    }
    // A custom source states no subtype; no other kind's out-port carries one.
    let (g, s) = add(
        &StainGraph::new(),
        NodeKind::Source,
        custom("fn source(ctx: Ctx) -> Field { return Field(0.5, 0.0, 0.0, 0.0); }"),
    );
    assert_eq!(g.ports_of(s).expect("ports").outputs[0].subtype, None);
    assert_eq!(
        g.ports_of(COMBINER).expect("ports").outputs[0].subtype,
        None
    );
    assert!(g.ports_of(NodeId(99)).is_err(), "a missing node has ports");
}

validation::negative_control!(
    stain_node_kinds,
    "the brightness node checked against the colour node's row (a vec3 out) must fail",
    expected = "ports are not §3.1's row",
    {
        let (g, rows) = kinds();
        let brightness = rows[3].0;
        check_row(&g, brightness, &rows[1].1)
    }
);

// ----- stain_backbone_singletons (REQ-GUI-022) -----

/// Checks that `edit` is refused as a fixed singleton's `action`.
fn check_singleton(g: &StainGraph, edit: StainEdit, action: Action) {
    match apply(g, &edit) {
        Err(Refusal::FixedSingleton { action: a, .. }) if a == action => {}
        other => panic!("{edit:?} was not refused as a fixed singleton: {other:?}"),
    }
}

/// The number of nodes of `kind` in `g`.
fn count(g: &StainGraph, kind: NodeKind) -> usize {
    g.nodes().filter(|n| n.kind == kind).count()
}

#[test]
fn stain_backbone_singletons() {
    let g = StainGraph::new();
    for kind in [NodeKind::Combiner, NodeKind::Out] {
        check_singleton(
            &g,
            StainEdit::Add {
                kind,
                occupant: Occupant::None,
            },
            Action::Add,
        );
    }
    for node in [COMBINER, OUT] {
        check_singleton(&g, StainEdit::Delete { node }, Action::Delete);
        check_singleton(&g, StainEdit::Duplicate { node }, Action::Duplicate);
    }
    // The graph's own methods refuse them too.
    let mut h = g.clone();
    assert!(h.add(NodeKind::Out, Occupant::None).is_err());
    assert!(h.remove(OUT).is_err() && h.remove(COMBINER).is_err());
    assert!(h.duplicate(OUT).is_err() && h.duplicate(COMBINER).is_err());
    assert_eq!(h, g, "a refused edit changed the graph");
    // The free nodes are added, duplicated and deleted, and through it all one combiner and one OUT stay.
    let (g, c) = add(&g, NodeKind::Colour, Occupant::None);
    let (g, p) = add(&g, NodeKind::Post, Occupant::None);
    let g = accepted(&g, StainEdit::Duplicate { node: c });
    let copy = g.nodes().map(|n| n.id).max().expect("a node");
    assert_eq!(g.node(copy).expect("the copy").kind, NodeKind::Colour);
    let g = accepted(&g, StainEdit::Delete { node: p });
    assert!(g.node(p).is_none(), "the post was not deleted");
    for kind in [NodeKind::Combiner, NodeKind::Out] {
        assert_eq!(
            count(&g, kind),
            1,
            "the graph holds other than one {kind:?}"
        );
    }
    // A missing node is refused, by name.
    assert_eq!(
        apply(&g, &StainEdit::Delete { node: NodeId(99) }),
        Err(Refusal::NoNode(NodeId(99)))
    );
    assert_eq!(
        apply(&g, &StainEdit::Duplicate { node: NodeId(99) }),
        Err(Refusal::NoNode(NodeId(99)))
    );
    // A serialised graph with a second OUT is refused when read.
    let mut v = serde_json::to_value(&g).expect("serialised");
    let second = serde_json::json!({"id": 50, "kind": "out", "occupant": "none"});
    v["nodes"].as_array_mut().expect("nodes").push(second);
    assert!(
        serde_json::from_value::<StainGraph>(v).is_err(),
        "a graph with two OUTs was read"
    );
}

validation::negative_control!(
    stain_backbone_singletons,
    "deleting a free post checked as a refused singleton must fail",
    expected = "was not refused as a fixed singleton",
    {
        let (g, p) = add(&StainGraph::new(), NodeKind::Post, Occupant::None);
        check_singleton(&g, StainEdit::Delete { node: p }, Action::Delete)
    }
);

// ----- stain_wires_acyclic (REQ-GUI-023) -----

/// Whether `wires` (from, to) hold a cycle: the test's own depth-first search.
fn cyclic(wires: &[(NodeId, NodeId)]) -> bool {
    let nodes: BTreeSet<NodeId> = wires.iter().flat_map(|&(a, b)| [a, b]).collect();
    // 0 unseen, 1 on the walk, 2 done.
    fn walk(n: NodeId, wires: &[(NodeId, NodeId)], state: &mut Vec<(NodeId, u8)>) -> bool {
        let at = |state: &Vec<(NodeId, u8)>| state.iter().find(|(m, _)| *m == n).map(|s| s.1);
        match at(state) {
            Some(1) => return true,
            Some(_) => return false,
            None => state.push((n, 1)),
        }
        for &(a, b) in wires {
            if a == n && walk(b, wires, state) {
                return true;
            }
        }
        state
            .iter_mut()
            .find(|(m, _)| *m == n)
            .expect("on the walk")
            .1 = 2;
        false
    }
    let mut state = Vec::new();
    nodes.iter().any(|&n| walk(n, wires, &mut state))
}

fn pairs(g: &StainGraph) -> Vec<(NodeId, NodeId)> {
    g.wires().iter().map(|w| (w.from, w.to)).collect()
}

/// The free interior the property wires: two sources, two colours, two brightnesses and four posts, unwired, beside the
/// backbone.
fn interior() -> (StainGraph, Vec<NodeId>) {
    let mut g = StainGraph::new();
    let mut ids = vec![COMBINER, OUT];
    for (kind, occupant) in [
        (NodeKind::Source, field("ftle")),
        (NodeKind::Source, field("ftle")),
        (NodeKind::Colour, Occupant::None),
        (NodeKind::Colour, custom(COLOUR3)),
        (NodeKind::Brightness, Occupant::None),
        (NodeKind::Brightness, Occupant::None),
        (NodeKind::Post, Occupant::None),
        (NodeKind::Post, Occupant::None),
        (NodeKind::Post, custom(POST2)),
        (NodeKind::Post, Occupant::None),
    ] {
        let (next, id) = add(&g, kind, occupant);
        g = next;
        ids.push(id);
    }
    (g, ids)
}

/// One step of the property: `outcome` is what wiring `from` → `to`.`port` into `before` gave, the graph after it or
/// the refusal. Accepted, the wire's types match and it closed no cycle, and the graph after it is acyclic and holds
/// the wire; refused where it would close a cycle, the refusal says so.
fn check_step(
    before: &StainGraph,
    (from, to, port): (NodeId, NodeId, u32),
    outcome: Result<&StainGraph, &Refusal>,
) {
    let mut with = pairs(before);
    with.push((from, to));
    let closes = cyclic(&with);
    match outcome {
        Ok(after) => {
            assert!(
                !closes,
                "a cycle-closing wire {from:?} → {to:?} was accepted"
            );
            let out = before.node(from).expect("from").kind.output();
            let ins = before.in_ports(to).expect("to's ports");
            assert_eq!(
                out,
                ins.get(port as usize).copied(),
                "an accepted wire does not type-check"
            );
            assert!(!cyclic(&pairs(after)), "the graph holds a cycle");
            assert!(
                after
                    .wires()
                    .iter()
                    .any(|w| (w.from, w.to, w.port) == (from, to, port)),
                "an accepted wire is not in the graph"
            );
        }
        Err(r) => {
            if closes && out_matches(before, from, to, port) {
                assert!(
                    matches!(r, Refusal::Cycle { .. }),
                    "a cycle-closing wire was refused as {r:?}"
                );
            }
        }
    }
}

/// Whether `from`'s out-port type is `to`'s in-port `port`'s.
fn out_matches(g: &StainGraph, from: NodeId, to: NodeId, port: u32) -> bool {
    let out = g.node(from).expect("from").kind.output();
    let ins = g.in_ports(to).expect("to's ports");
    ins.get(port as usize).is_some_and(|&t| Some(t) == out)
}

/// The property over a sequence of wire drops, each a node index, a node index and a port.
fn check_wires(drops: &[(usize, usize, u32)]) {
    let (mut g, ids) = interior();
    for &(a, b, port) in drops {
        let step = (ids[a], ids[b], port);
        let edit = StainEdit::Wire {
            from: step.0,
            to: step.1,
            port,
        };
        match apply(&g, &edit) {
            Ok(set) => {
                let Edit::Render(RenderField::StainGraph(next)) = set.edit else {
                    panic!("an accepted wire wrote another field");
                };
                check_step(&g, step, Ok(&next));
                g = next;
            }
            Err(r) => check_step(&g, step, Err(&r)),
        }
    }
}

#[test]
fn stain_wires_acyclic() {
    // A cycle-closing wire is refused: two posts wired one way, then back, and a post onto itself.
    let (g, ids) = interior();
    let (p1, p2) = (ids[8], ids[9]);
    let g = wire(&g, p1, p2, 0);
    for (from, to) in [(p2, p1), (p1, p1)] {
        assert_eq!(
            apply(&g, &StainEdit::Wire { from, to, port: 0 }),
            Err(Refusal::Cycle { from, to }),
            "the wire {from:?} → {to:?}"
        );
    }
    // Through a third post: p1 → p2 → p3, and p3 → p1 closes it.
    let p3 = ids[10];
    let g = wire(&g, p2, p3, 0);
    assert_eq!(
        apply(
            &g,
            &StainEdit::Wire {
                from: p3,
                to: p1,
                port: 0
            }
        ),
        Err(Refusal::Cycle { from: p3, to: p1 })
    );
    assert!(g.feeds(p1, p3) && !g.feeds(p3, p1) && !g.feeds(p1, p1));
    let n = interior().1.len();
    validation::prop::run(
        &prop::collection::vec((0..n, 0..n, 0u32..4), 1..40),
        |drops| {
            check_wires(&drops);
            Ok(())
        },
    );
}

validation::negative_control!(
    stain_wires_acyclic,
    "a cycle-closing wire reported as accepted must fail the property's step",
    expected = "a cycle-closing wire",
    {
        let (g, ids) = interior();
        let (p1, p2) = (ids[8], ids[9]);
        let g = wire(&g, p1, p2, 0);
        check_step(&g, (p2, p1, 0), Ok(&g))
    }
);

// ----- stain_wire_types (REQ-GUI-025) -----

/// Checks that wiring `from` → `to`.`port` is refused as a type mismatch and changes nothing, the graph's own
/// `connect` included.
fn check_mismatch(g: &StainGraph, from: NodeId, to: NodeId, port: u32) {
    let r = apply(g, &StainEdit::Wire { from, to, port });
    assert!(
        matches!(r, Err(Refusal::TypeMismatch { .. })),
        "the drop {from:?} → {to:?}.{port} was not refused as a type mismatch: {r:?}"
    );
    let mut h = g.clone();
    assert!(h.connect(from, to, port as usize).is_err());
    assert_eq!(&h, g, "a refused drop changed the graph");
}

/// A source, a colour and a brightness beside the backbone.
fn three() -> (StainGraph, NodeId, NodeId, NodeId) {
    let (g, s) = add(&StainGraph::new(), NodeKind::Source, field("ftle"));
    let (g, c) = add(&g, NodeKind::Colour, Occupant::None);
    let (g, b) = add(&g, NodeKind::Brightness, Occupant::None);
    (g, s, c, b)
}

/// A three-stop ramp, cyclic or not.
fn ramp(cyclic: bool) -> Ramp {
    Ramp::new(
        vec![
            Stop {
                position: 0.0,
                oklab: [0.3, 0.1, -0.1],
            },
            Stop {
                position: 0.25,
                oklab: [0.6, -0.12, 0.05],
            },
            Stop {
                position: 0.75,
                oklab: [0.9, 0.02, 0.15],
            },
        ],
        cyclic,
    )
    .expect("a ramp")
}

#[test]
fn stain_wire_types() {
    let (g, s, c, b) = three();
    // vec3 → f32: the colour onto the combiner's brightness. field → vec3: the source onto its colour.
    check_mismatch(&g, c, COMBINER, 1);
    check_mismatch(&g, s, COMBINER, 0);
    // f32 → vec3, and OUT, which has no out-port.
    check_mismatch(&g, b, COMBINER, 0);
    assert_eq!(
        apply(
            &g,
            &StainEdit::Wire {
                from: OUT,
                to: c,
                port: 0
            }
        ),
        Err(Refusal::TypeMismatch {
            from: None,
            to: PortType::Field
        })
    );
    // Matching drops attach: field → field, vec3 → vec3, f32 → f32.
    let g = wire(&g, s, c, 0);
    let g = wire(&g, s, b, 0);
    let g = wire(&g, c, COMBINER, 0);
    let g = wire(&g, b, COMBINER, 1);
    assert_eq!(
        g.wires().len(),
        5,
        "the four drops and the combiner → OUT wire"
    );
    // A port past a node's in-ports is refused by name.
    assert_eq!(
        apply(
            &g,
            &StainEdit::Wire {
                from: s,
                to: c,
                port: 1
            }
        ),
        Err(Refusal::NoPort {
            node: c,
            port: 1,
            ports: 1
        })
    );
    // A type match that breaks the backbone is refused as such: a colour straight into OUT.
    assert!(matches!(
        apply(
            &g,
            &StainEdit::Wire {
                from: c,
                to: OUT,
                port: 0
            }
        ),
        Err(Refusal::Backbone(_))
    ));
    // The subtype is no gate: a categorical source dropped onto a colour node in gradient mode (wired from a scalar
    // source, with its ramp) attaches, and the in-port then carries the categorical subtype, keeping the ramp.
    let g = accepted(
        &g,
        StainEdit::Ramp {
            node: c,
            ramp: Some(ramp(false)),
        },
    );
    let (g, cat) = add(
        &g,
        NodeKind::Source,
        field(&field_of(FieldSubtype::Categorical)),
    );
    let inport = |g: &StainGraph| g.ports_of(c).expect("ports").inputs[0];
    assert_eq!(inport(&g).subtype, Some(Subtype::Scalar));
    let g = wire(&g, cat, c, 0);
    assert_eq!(inport(&g).subtype, Some(Subtype::Categorical));
    assert_eq!(g.node(c).expect("c").ramp, Some(ramp(false)));
    // Every subtype, point⟨q⟩ included, drops onto every field in-port; never a vec3 onto a field, nor in to out.
    let subtypes = [
        None,
        Some(Subtype::Scalar),
        Some(Subtype::Vector),
        Some(Subtype::Categorical),
        Some(Subtype::Point(Quotient::Sphere)),
        Some(Subtype::Point(Quotient::Simplex)),
    ];
    for a in subtypes {
        for b in subtypes {
            let out = port(Direction::Out, PortType::Field, a);
            let inp = port(Direction::In, PortType::Field, b);
            assert!(out.accepts(&inp), "{a:?} → {b:?}");
            assert!(!inp.accepts(&out) && !out.accepts(&out) && !inp.accepts(&inp));
        }
    }
    let vec3 = port(Direction::Out, PortType::Vec3, None);
    assert!(!vec3.accepts(&port(Direction::In, PortType::F32, None)));
    assert!(vec3.accepts(&port(Direction::In, PortType::Vec3, None)));
}

validation::negative_control!(
    stain_wire_types,
    "a matching drop (field → field) checked as a mismatch must fail",
    expected = "was not refused as a type mismatch",
    {
        let (g, s, c, _) = three();
        check_mismatch(&g, s, c, 0)
    }
);

// ----- stain_port_arity (REQ-GUI-026) -----

/// Checks that `g` has `want` wires into `to`.`port`, the first from `from`.
fn check_arity(g: &StainGraph, (to, port, from): (NodeId, u32, NodeId), want: usize) {
    let into: Vec<NodeId> = g
        .wires()
        .iter()
        .filter(|w| (w.to, w.port) == (to, port))
        .map(|w| w.from)
        .collect();
    assert_eq!(into.len(), want, "the in-port holds {} wire(s)", into.len());
    assert_eq!(
        into.first(),
        Some(&from),
        "the in-port's wire is not the last dropped"
    );
}

#[test]
fn stain_port_arity() {
    let (g, s1, c, _) = three();
    let (g, s2) = add(&g, NodeKind::Source, field("ftle"));
    let g = wire(&g, s1, c, 0);
    check_arity(&g, (c, 0, s1), 1);
    // The second drop replaces the first: last write wins.
    let g = wire(&g, s2, c, 0);
    check_arity(&g, (c, 0, s2), 1);
    assert!(
        !g.wires().iter().any(|w| w.from == s1),
        "the first wire stayed"
    );
    // Dropping the same wire again changes nothing, so it is no edit.
    assert_eq!(
        apply(
            &g,
            &StainEdit::Wire {
                from: s2,
                to: c,
                port: 0
            }
        ),
        Err(Refusal::NoChange)
    );
    // Unwiring it empties the port; unwiring an empty port is no edit.
    let h = accepted(&g, StainEdit::Unwire { to: c, port: 0 });
    assert!(!h.wires().iter().any(|w| w.to == c));
    assert_eq!(
        apply(&h, &StainEdit::Unwire { to: c, port: 0 }),
        Err(Refusal::NoChange)
    );
    assert_eq!(
        apply(
            &h,
            &StainEdit::Unwire {
                to: NodeId(99),
                port: 0
            }
        ),
        Err(Refusal::NoNode(NodeId(99)))
    );
    // One out-port feeds five in-ports: a colour's three, two brightnesses'.
    let (g, c3) = add(&g, NodeKind::Colour, custom(COLOUR3));
    let (g, b1) = add(&g, NodeKind::Brightness, Occupant::None);
    let (g, b2) = add(&g, NodeKind::Brightness, Occupant::None);
    let mut g = g;
    for (to, port) in [(c3, 0), (c3, 1), (c3, 2), (b1, 0), (b2, 0)] {
        g = wire(&g, s1, to, port);
    }
    let fan = g.wires().iter().filter(|w| w.from == s1).count();
    assert_eq!(fan, 5, "the out-port holds {fan} wire(s)");
}

validation::negative_control!(
    stain_port_arity,
    "an in-port checked for two wires after a second drop must fail",
    expected = "the in-port holds 1 wire(s)",
    {
        let (g, s1, c, _) = three();
        let (g, s2) = add(&g, NodeKind::Source, field("ftle"));
        let g = wire(&wire(&g, s1, c, 0), s2, c, 0);
        check_arity(&g, (c, 0, s2), 2)
    }
);

// ----- stain_renderstate_roundtrip (REQ-GUI-016) -----

fn sim() -> SimConfig {
    SimConfig {
        chart: Chart {
            window: Window::SQUARE,
        },
        plane: Plane {
            z0: [0.0; 8],
            q1: [0.0; 8],
            q2: [0.0; 8],
        },
        slice: Slice {
            frame: SliceFrame::along(Z_Q3),
        },
        lock: Lock {
            locked: false,
            z_locked: [0.0; 8],
        },
        links: Links::defaults(),
        integrator: Integrator {},
        kernel_variant: KernelVariant::Physics,
        horizon: Horizon { t: 100.0 },
        collision: Collision {},
        quality: Quality {},
    }
}

fn render(graph: StainGraph) -> RenderState {
    RenderState {
        stain_graph: graph,
        overlays: Overlays::default(),
        palette: Palette {},
        playhead: Playhead { t: 0.0 },
        hover: Hover::default(),
        debug_view: false,
    }
}

/// A wired graph with every kind, a param set and a ramp: the source feeds a colour, holding the ramp, and a
/// brightness into the combiner, two posts, OUT.
fn wired() -> StainGraph {
    let (g, s, c, b) = three();
    let (g, p1) = add(&g, NodeKind::Post, Occupant::None);
    let (g, p2) = add(&g, NodeKind::Post, custom(POST2));
    let g = accepted(
        &g,
        StainEdit::Occupant {
            node: c,
            occupant: custom(SHOW),
        },
    );
    let g = accepted(
        &g,
        StainEdit::Occupant {
            node: b,
            occupant: custom(GAIN),
        },
    );
    let g = accepted(
        &g,
        StainEdit::Param {
            node: b,
            name: "gain".into(),
            value: vec![2.5],
        },
    );
    let g = accepted(
        &g,
        StainEdit::Ramp {
            node: c,
            ramp: Some(ramp(true)),
        },
    );
    let mut g = g;
    for (from, to, port) in [
        (s, c, 0),
        (s, b, 0),
        (c, COMBINER, 0),
        (b, COMBINER, 1),
        (COMBINER, p1, 0),
        (p1, p2, 0),
        (s, p2, 1),
        (p2, OUT, 0),
    ] {
        g = wire(&g, from, to, port);
    }
    g
}

/// `state` written in the one canonical serialisation and read back.
fn round_trip(state: &RenderState) -> RenderState {
    let text = crate::contract::canonical::to_string(state).expect("JCS");
    serde_json::from_str(&text).expect("read back")
}

/// Checks that `back` is `state`, graph and all.
fn check_round_trip(state: &RenderState, back: &RenderState) {
    assert_eq!(back, state, "the RenderState did not round-trip");
}

#[test]
fn stain_renderstate_roundtrip() {
    let state = render(wired());
    let back = round_trip(&state);
    check_round_trip(&state, &back);
    // The ids, the wires and the params came back, not only an equal-looking graph.
    let (g, h) = (&state.stain_graph, &back.stain_graph);
    assert_eq!(h.wires(), g.wires());
    assert_eq!(
        h.nodes()
            .map(|n| (n.id, n.params.clone()))
            .collect::<Vec<_>>(),
        g.nodes()
            .map(|n| (n.id, n.params.clone()))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        h.canonical().expect("form").render_key(),
        g.canonical().expect("form").render_key()
    );
    // Picking a node's occupant is one SetField, undoable: one history entry, and undo restores the graph.
    let b = g
        .nodes()
        .find(|n| n.kind == NodeKind::Brightness)
        .expect("the brightness")
        .id;
    let set = apply(
        g,
        &StainEdit::Occupant {
            node: b,
            occupant: Occupant::None,
        },
    )
    .expect("picked");
    assert!(!set.no_history, "an occupant pick is outside the history");
    let Edit::Render(RenderField::StainGraph(next)) = &set.edit else {
        panic!("the pick wrote another field: {:?}", set.edit);
    };
    assert_eq!(next.node(b).expect("b").occupant, Occupant::None);
    let mut store = StateStore::new(sim(), state.clone());
    store.set_field(set.clone());
    let snap = store.snapshot();
    assert_eq!(
        snap.history.undo_depth, 1,
        "the pick is not one history entry"
    );
    assert_eq!(&snap.render.stain_graph, next);
    assert_eq!(snap.log.len(), 1, "the pick logged other than one SetField");
    store.undo();
    assert_eq!(
        &store.snapshot().render.stain_graph,
        g,
        "undo did not restore the graph"
    );
    // Every edit kind is one SetField of the whole graph, undoable.
    let edits = [
        StainEdit::Add {
            kind: NodeKind::Colour,
            occupant: Occupant::None,
        },
        StainEdit::Duplicate { node: b },
        StainEdit::Unwire { to: b, port: 0 },
        StainEdit::Param {
            node: b,
            name: "gain".into(),
            value: vec![1.0],
        },
    ];
    for edit in edits {
        let set: SetField = apply(g, &edit).expect("accepted");
        assert!(
            matches!(set.edit, Edit::Render(RenderField::StainGraph(_))) && !set.no_history,
            "{edit:?}"
        );
    }
}

validation::negative_control!(
    stain_renderstate_roundtrip,
    "a RenderState read back with its params dropped must fail to round-trip",
    expected = "the RenderState did not round-trip",
    {
        let state = render(wired());
        let text = crate::contract::canonical::to_string(&state).expect("JCS");
        let dropped = text.replace(r#""params":{"gain":[2.5]}"#, r#""params":{}"#);
        assert_ne!(dropped, text, "the control dropped nothing");
        let back: RenderState = serde_json::from_str(&dropped).expect("read back");
        check_round_trip(&state, &back)
    }
);

// ----- the post chain's reorder (render_gui_spec Part II §4) -----

/// Checks that `g`'s live post chain is `want`, nearest the combiner first.
fn check_chain(g: &StainGraph, want: &[NodeId]) {
    let chain = g.chain();
    assert_eq!(chain, want, "the live chain is {chain:?}, not {want:?}");
}

#[test]
fn stain_reorder_moves_a_post_in_the_live_chain() {
    let (g, ids) = interior();
    let (c, b, p1, p2, p3) = (ids[4], ids[6], ids[8], ids[9], ids[11]);
    let s = ids[2];
    let mut g = g;
    for (from, to, port) in [
        (s, c, 0),
        (c, COMBINER, 0),
        (s, b, 0),
        (b, COMBINER, 1),
        (COMBINER, p1, 0),
        (p1, p2, 0),
        (p2, p3, 0),
        (p3, OUT, 0),
    ] {
        g = wire(&g, from, to, port);
    }
    check_chain(&g, &[p1, p2, p3]);
    let moved = accepted(
        &g,
        StainEdit::Reorder {
            post: p3,
            position: 0,
        },
    );
    check_chain(&moved, &[p3, p1, p2]);
    let fed = |g: &StainGraph, to: NodeId| {
        g.wires()
            .iter()
            .find(|w| w.to == to && w.port == 0)
            .map(|w| w.from)
    };
    assert_eq!(fed(&moved, p3), Some(COMBINER));
    assert_eq!(fed(&moved, OUT), Some(p2));
    let moved = accepted(
        &g,
        StainEdit::Reorder {
            post: p1,
            position: 2,
        },
    );
    check_chain(&moved, &[p2, p3, p1]);
    let moved = accepted(
        &g,
        StainEdit::Reorder {
            post: p2,
            position: 0,
        },
    );
    check_chain(&moved, &[p2, p1, p3]);
    // To its own place, no edit; past the end, or a post off the chain, refused by name.
    assert_eq!(
        apply(
            &g,
            &StainEdit::Reorder {
                post: p2,
                position: 1
            }
        ),
        Err(Refusal::NoChange)
    );
    assert_eq!(
        apply(
            &g,
            &StainEdit::Reorder {
                post: p2,
                position: 3
            }
        ),
        Err(Refusal::NoPosition {
            position: 3,
            chain: 3
        })
    );
    let off = ids[10];
    assert_eq!(
        apply(
            &g,
            &StainEdit::Reorder {
                post: off,
                position: 0
            }
        ),
        Err(Refusal::NotInChain(off))
    );
    assert!(g.clone().reorder(off, 0).is_err() && g.clone().reorder(p1, 3).is_err());
    // A chain whose first post nothing feeds keeps its head empty.
    let headless = accepted(&g, StainEdit::Unwire { to: p1, port: 0 });
    let moved = accepted(
        &headless,
        StainEdit::Reorder {
            post: p3,
            position: 0,
        },
    );
    check_chain(&moved, &[p3, p1, p2]);
    assert_eq!(fed(&moved, p3), None);
    assert_eq!(fed(&moved, p1), Some(p3));
    // No chain: OUT fed by the combiner straight.
    assert!(StainGraph::new().chain().is_empty());
}

validation::negative_control!(
    stain_reorder_moves_a_post_in_the_live_chain,
    "a chain checked against the order before the move must fail",
    expected = "the live chain is",
    {
        let (g, ids) = interior();
        let (p1, p2) = (ids[8], ids[9]);
        let g = wire(&wire(&wire(&g, COMBINER, p1, 0), p1, p2, 0), p2, OUT, 0);
        let moved = accepted(
            &g,
            StainEdit::Reorder {
                post: p2,
                position: 0,
            },
        );
        check_chain(&moved, &[p1, p2])
    }
);

// ----- stain_custom_stop_ramp (REQ-COL-140) -----

/// `a + u (b − a)`, per component.
fn mix(a: [f64; 3], b: [f64; 3], u: f64) -> [f64; 3] {
    [0, 1, 2].map(|i| a[i] + u * (b[i] - a[i]))
}

/// A point between two stops: the stops' OKLab colours and how far from the first to the second, in `[0, 1]`.
type Segment = ([f64; 3], [f64; 3], f64);

/// Where the interpolation runs.
#[derive(Clone, Copy, Debug)]
enum Space {
    Oklab,
    LinearSrgb,
}

/// The colour, linear sRGB, between stops `a` and `b` at `u`, interpolated in `space`.
fn between(space: Space, a: [f64; 3], b: [f64; 3], u: f64) -> [f64; 3] {
    match space {
        Space::Oklab => oklab_to_linear(mix(a, b, u)),
        Space::LinearSrgb => mix(oklab_to_linear(a), oklab_to_linear(b), u),
    }
}

fn close(a: [f64; 3], b: [f64; 3]) -> bool {
    (0..3).all(|i| (a[i] - b[i]).abs() <= 1e-12)
}

/// Checks `r` at `t` against stops `a` and `b` at `u`, interpolated in `space`.
fn check_at(r: &Ramp, t: f64, (a, b, u): Segment, space: Space) {
    let got = r.linear_at(t).expect("a finite t");
    let want = between(space, a, b, u);
    assert!(
        close(got, want),
        "at t = {t} the ramp is {got:?}, not {want:?} interpolated in {space:?}"
    );
}

/// The ramp's evaluations §9.4 fixes, each `(t, (from, to, u))`, cyclic or not.
fn cases(cyclic: bool) -> Vec<(f64, Segment)> {
    let s = ramp(cyclic).stops().to_vec();
    let (c0, c1, c2) = (s[0].oklab, s[1].oklab, s[2].oklab);
    let mut cases = vec![
        // At the stops, their colours; between, linear in t: a quarter and a half of the way.
        (0.0, (c0, c1, 0.0)),
        (0.25, (c1, c2, 0.0)),
        (0.0625, (c0, c1, 0.25)),
        (0.5, (c1, c2, 0.5)),
    ];
    if cyclic {
        // Past the last stop (0.75) the ramp runs on to the first, reached at 0 + 1: 0.875 is half way, and t
        // wraps modulo 1.
        cases.extend([
            (0.75, (c2, c0, 0.0)),
            (0.875, (c2, c0, 0.5)),
            (1.875, (c2, c0, 0.5)),
            (-0.125, (c2, c0, 0.5)),
            (1.5, (c1, c2, 0.5)),
            (1.0, (c0, c1, 0.0)),
        ]);
    } else {
        // Clamped: at and past the last stop, the last colour; below 0 the first.
        cases.extend([
            (0.75, (c2, c2, 0.0)),
            (0.875, (c2, c2, 0.0)),
            (1.875, (c2, c2, 0.0)),
            (-0.125, (c0, c0, 0.0)),
            (1.0, (c2, c2, 0.0)),
        ]);
    }
    cases
}

#[test]
fn stain_custom_stop_ramp() {
    for cyclic in [false, true] {
        let r = ramp(cyclic);
        for (t, at) in cases(cyclic) {
            check_at(&r, t, at, Space::Oklab);
        }
        assert_eq!(r.oklab_at(f64::NAN), None);
        assert_eq!(r.linear_at(f64::INFINITY), None);
        assert_eq!(r.cyclic(), cyclic);
    }
    // The two spaces disagree mid-way between two stops, so the control, which checks the other space, can fail.
    let s = ramp(false).stops().to_vec();
    let (oklab, rgb) = (
        between(Space::Oklab, s[1].oklab, s[2].oklab, 0.5),
        between(Space::LinearSrgb, s[1].oklab, s[2].oklab, 0.5),
    );
    assert!(!close(oklab, rgb), "the spaces agree: {oklab:?}");
    // A t a hair below an integer wraps to the integer's colour, not past the end.
    let r = ramp(true);
    assert_eq!(r.oklab_at(-1e-20), r.oklab_at(0.0));
    // A cyclic ramp whose stops do not reach 0 wraps below the first stop too: stops at 0.25 and 0.75, t = 0 half way
    // from the last back to the first.
    let (a, b) = ([0.2, 0.0, 0.0], [0.8, 0.0, 0.0]);
    let two = |cyclic| {
        Ramp::new(
            vec![
                Stop {
                    position: 0.25,
                    oklab: a,
                },
                Stop {
                    position: 0.75,
                    oklab: b,
                },
            ],
            cyclic,
        )
        .expect("a ramp")
    };
    check_at(&two(true), 0.0, (b, a, 0.5), Space::Oklab);
    check_at(&two(true), 0.125, (b, a, 0.75), Space::Oklab);
    check_at(&two(false), 0.125, (a, a, 0.0), Space::Oklab);
    // Two stops at one position draw a hard edge; at it, the later stop's colour.
    let edge = Ramp::new(
        vec![
            Stop {
                position: 0.0,
                oklab: a,
            },
            Stop {
                position: 0.5,
                oklab: a,
            },
            Stop {
                position: 0.5,
                oklab: b,
            },
            Stop {
                position: 1.0,
                oklab: b,
            },
        ],
        false,
    )
    .expect("a hard edge");
    assert_eq!(edge.oklab_at(0.5 - 1e-9), Some(a));
    assert_eq!(edge.oklab_at(0.5), Some(b));
    // At a shared first position too; and a cyclic ramp from 0 to 1 takes a t a hair below an integer, which rounds to
    // the integer, to the integer's colour.
    let shared = |cyclic| {
        let stop = |position, oklab| Stop { position, oklab };
        Ramp::new(vec![stop(0.0, a), stop(0.0, b), stop(1.0, a)], cyclic).expect("a ramp")
    };
    assert_eq!(shared(false).oklab_at(0.0), Some(b));
    // Every stop at one position: there, the last stop's colour, cyclic or not.
    for cyclic in [false, true] {
        let one = Ramp::new(
            vec![
                Stop {
                    position: 0.5,
                    oklab: a,
                },
                Stop {
                    position: 0.5,
                    oklab: b,
                },
            ],
            cyclic,
        )
        .expect("a ramp");
        assert_eq!(one.oklab_at(0.5), Some(b), "cyclic: {cyclic}");
    }
    assert_eq!(shared(true).oklab_at(-1e-20), Some(b));
    assert_eq!(shared(true).oklab_at(1.0), Some(b));
    // The conversion to linear sRGB is OKLab's: its published primaries (Ottosson, "A perceptual color space for
    // image processing", 2020) go to the unit vectors, and white to white.
    for (lab, rgb) in [
        ([0.627_955, 0.224_863, 0.125_846], [1.0, 0.0, 0.0]),
        ([0.866_440, -0.233_888, 0.179_498], [0.0, 1.0, 0.0]),
        ([0.452_014, -0.032_457, -0.311_528], [0.0, 0.0, 1.0]),
        ([1.0, 0.0, 0.0], [1.0, 1.0, 1.0]),
    ] {
        let got = oklab_to_linear(lab);
        assert!(
            (0..3).all(|i| (got[i] - rgb[i]).abs() < 1e-4),
            "OKLab {lab:?} is linear sRGB {got:?}, not {rgb:?}"
        );
    }
    // The stops are checked: two or more, positions in [0, 1] and never decreasing, L in [0, 1], all finite.
    let stop = |position, l| Stop {
        position,
        oklab: [l, 0.0, 0.0],
    };
    for (stops, ok) in [
        (vec![stop(0.0, 0.0), stop(1.0, 1.0)], true),
        (vec![stop(0.5, 0.5)], false),
        (vec![stop(-1e-9, 0.5), stop(1.0, 0.5)], false),
        (vec![stop(0.0, 0.5), stop(1.0 + 1e-9, 0.5)], false),
        (vec![stop(0.6, 0.5), stop(0.5, 0.5)], false),
        (vec![stop(0.0, -1e-9), stop(1.0, 0.5)], false),
        (vec![stop(0.0, 0.5), stop(1.0, 1.0 + 1e-9)], false),
        (vec![stop(f64::NAN, 0.5), stop(1.0, 0.5)], false),
        (
            vec![
                stop(0.0, 0.5),
                Stop {
                    position: 1.0,
                    oklab: [0.5, f64::NAN, 0.0],
                },
            ],
            false,
        ),
        (
            vec![
                stop(0.0, 0.5),
                Stop {
                    position: 1.0,
                    oklab: [0.5, 0.0, f64::INFINITY],
                },
            ],
            false,
        ),
    ] {
        assert_eq!(Ramp::new(stops.clone(), false).is_ok(), ok, "{stops:?}");
    }
    let why = |stops: Vec<Stop>| Ramp::new(stops, false).expect_err("refused").to_string();
    assert!(why(vec![stop(0.5, 0.5)]).contains("two or more stops"));
    assert!(why(vec![stop(0.0, 0.5), stop(2.0, 0.5)]).contains("not in [0, 1]"));
    assert!(why(vec![stop(0.6, 0.5), stop(0.5, 0.5)]).contains("below the stop before it"));
    assert!(why(vec![stop(0.0, 2.0), stop(1.0, 0.5)]).contains("not an OKLab colour"));
    // A ramp round-trips through RenderState, on its colour node, and a ramp read back is checked.
    let state = render(wired());
    let back = round_trip(&state);
    let ramp_of = |s: &RenderState| {
        s.stain_graph
            .nodes()
            .find_map(|n| n.ramp.clone())
            .expect("the colour node's ramp")
    };
    assert_eq!(ramp_of(&back), ramp(true));
    let text = crate::contract::canonical::to_string(&state).expect("JCS");
    let bad = text.replace(r#""position":0.25"#, r#""position":1.5"#);
    assert_ne!(bad, text);
    assert!(
        serde_json::from_str::<RenderState>(&bad).is_err(),
        "a bad ramp was read"
    );
    // A node without a ramp serialises no `ramp`; only a colour node takes one.
    let plain = serde_json::to_value(StainGraph::new()).expect("serialised");
    assert!(plain["nodes"][0].get("ramp").is_none());
    let (g, s, _, b) = three();
    for node in [s, b, COMBINER, OUT] {
        assert!(
            matches!(
                apply(
                    &g,
                    &StainEdit::Ramp {
                        node,
                        ramp: Some(ramp(false))
                    }
                ),
                Err(Refusal::Graph(_))
            ),
            "a ramp on node {node:?}"
        );
    }
    // The ramp is in the render key, not the fragment key, and in the log line.
    let g = wired();
    let c = g.nodes().find(|n| n.ramp.is_some()).expect("c").id;
    let flat = accepted(
        &g,
        StainEdit::Ramp {
            node: c,
            ramp: Some(ramp(false)),
        },
    );
    let (k, kf) = (
        g.canonical().expect("form"),
        flat.canonical().expect("form"),
    );
    assert_ne!(k.render_key(), kf.render_key());
    assert_eq!(k.fragment_key(), kf.fragment_key());
    assert!(stain_graph(&g).contains(
        " ramp[0=(0.3, 0.1, -0.1), 0.25=(0.6, -0.12, 0.05), 0.75=(0.9, 0.02, 0.15)] cyclic"
    ));
    assert!(stain_graph(&flat).contains(", 0.75=(0.9, 0.02, 0.15)]"));
    assert!(!stain_graph(&flat).contains("cyclic"));
    let none = accepted(
        &g,
        StainEdit::Ramp {
            node: c,
            ramp: None,
        },
    );
    assert!(!stain_graph(&none).contains("ramp"));
}

validation::negative_control!(
    stain_custom_stop_ramp,
    "the ramp checked against an interpolation in linear sRGB, the other space, must fail",
    expected = "interpolated in LinearSrgb",
    {
        let r = ramp(true);
        for (t, at) in cases(true) {
            check_at(&r, t, at, Space::LinearSrgb);
        }
    }
);

// ----- stain_ramp_wrap_is_finite (REQ-COL-140) -----

/// Checks that `got`, the ramp of `stops` at `t`, is a colour, each component finite and within the stops' own range.
fn check_within(stops: &[Stop], t: f64, got: Option<[f64; 3]>) {
    let got = got.expect("a finite t");
    for i in 0..3 {
        let (lo, hi) = stops
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), s| {
                (lo.min(s.oklab[i]), hi.max(s.oklab[i]))
            });
        assert!(
            got[i].is_finite() && got[i] >= lo - 1e-12 && got[i] <= hi + 1e-12,
            "at t = {t} the ramp {stops:?} is {got:?}, not a colour within its stops'"
        );
    }
}

/// A position in `[0, 1]`, drawn often at or a hair from the ends.
fn position() -> impl Strategy<Value = f64> {
    prop_oneof![
        0.0..=1.0f64,
        Just(0.0),
        Just(1.0),
        0.0..1e-15f64,
        (0.0..1e-15f64).prop_map(|e| 1.0 - e),
        Just(f64::MIN_POSITIVE),
        Just(5e-324),
    ]
}

#[test]
fn stain_ramp_wrap_is_finite() {
    let (a, b) = ([0.2, 0.1, -0.1], [0.8, -0.1, 0.1]);
    let pair = |p: f64, q: f64| {
        Ramp::new(
            vec![
                Stop {
                    position: p,
                    oklab: a,
                },
                Stop {
                    position: q,
                    oklab: b,
                },
            ],
            true,
        )
        .expect("a ramp")
    };
    // First stop a hair past 0, the last at 1: the wrap's span is the hair, and at 0, the wrap's start, the last
    // stop's colour, as at 1; at the first stop, its own.
    let r = pair(1e-17, 1.0);
    assert_eq!(r.oklab_at(0.0), Some(b));
    assert_eq!(r.oklab_at(1.0), Some(b));
    assert_eq!(r.oklab_at(-1e-20), Some(b));
    assert_eq!(r.oklab_at(1e-17), Some(a));
    // At 0 and 1: the two ends are one point, so there is no wrap; 0 takes the first stop's colour.
    let r = pair(0.0, 1.0);
    assert_eq!(r.oklab_at(0.0), Some(a));
    assert_eq!(r.oklab_at(1.0), Some(a));
    assert!(close(r.oklab_at(0.5).expect("a colour"), mix(a, b, 0.5)));
    // The last stop a hair below 1: at it, its colour; past 1, back to the first.
    let last = 1.0 - 1e-16;
    let r = pair(0.0, last);
    assert_eq!(r.oklab_at(last), Some(b));
    assert_eq!(r.oklab_at(0.0), Some(a));
    let r = pair(0.0, 1.0 - 2e-16);
    assert!(close(
        r.oklab_at(1.0 - 1e-16).expect("a colour"),
        mix(b, a, 0.5)
    ));
    // Every finite t on a valid cyclic ramp, its stops often at or a hair from the ends, is a colour within its stops'.
    let colour = (0.0..=1.0f64, -0.4..0.4f64, -0.4..0.4f64).prop_map(|(l, a, b)| [l, a, b]);
    let t = prop_oneof![
        -3.0..3.0f64,
        position(),
        position().prop_map(|p| p - 1.0),
        prop::num::f64::POSITIVE
            | prop::num::f64::NEGATIVE
            | prop::num::f64::NORMAL
            | prop::num::f64::SUBNORMAL
            | prop::num::f64::ZERO,
    ];
    validation::prop::run(
        &(prop::collection::vec((position(), colour), 2..6), t),
        |(mut stops, t)| {
            stops.sort_by(|p, q| p.0.total_cmp(&q.0));
            let stops: Vec<Stop> = stops
                .into_iter()
                .map(|(position, oklab)| Stop { position, oklab })
                .collect();
            let r = Ramp::new(stops.clone(), true).expect("a valid ramp");
            check_within(&stops, t, r.oklab_at(t));
            Ok(())
        },
    );
}

validation::negative_control!(
    stain_ramp_wrap_is_finite,
    "the wrap at 0 of stops at 1e-17 and 1 shifted by one, (0 + 1 - 1) / (1e-17 + 1 - 1), must fail the check",
    expected = "not a colour within its stops'",
    {
        let stops = [
            Stop {
                position: 1e-17,
                oklab: [0.2, 0.1, -0.1],
            },
            Stop {
                position: 1.0,
                oklab: [0.8, -0.1, 0.1],
            },
        ];
        let (first, last) = (stops[0], stops[1]);
        let u = (0.0 + 1.0 - last.position) / (first.position + 1.0 - last.position);
        check_within(&stops, 0.0, Some(mix(last.oklab, first.oklab, u)))
    }
);

// ----- the refusals name their reasons -----

/// Checks that `r` reads `want`.
fn check_reason(r: &Refusal, want: &str) {
    assert_eq!(r.to_string(), want, "the refusal {r:?} reads otherwise");
}

/// Each refusal and what it reads.
fn reasons() -> Vec<(Refusal, &'static str)> {
    vec![
        (
            Refusal::FixedSingleton {
                kind: NodeKind::Out,
                action: Action::Delete,
            },
            "the OUT is a fixed singleton: it is never deleted",
        ),
        (
            Refusal::FixedSingleton {
                kind: NodeKind::Combiner,
                action: Action::Add,
            },
            "the combiner is a fixed singleton: it is never added",
        ),
        (
            Refusal::FixedSingleton {
                kind: NodeKind::Combiner,
                action: Action::Duplicate,
            },
            "the combiner is a fixed singleton: it is never duplicated",
        ),
        (Refusal::NoNode(NodeId(7)), "no node 7"),
        (
            Refusal::TypeMismatch {
                from: None,
                to: PortType::Field,
            },
            "a node with no out-port cannot feed an in-port of type field: the types differ",
        ),
        (
            Refusal::NoPort {
                node: NodeId(3),
                port: 2,
                ports: 1,
            },
            "node 3 has 1 in-port(s); no port 2",
        ),
        (
            Refusal::TypeMismatch {
                from: Some(PortType::Vec3),
                to: PortType::F32,
            },
            "a vec3 out-port cannot feed an in-port of type f32: the types differ",
        ),
        (
            Refusal::Cycle {
                from: NodeId(4),
                to: NodeId(5),
            },
            "node 5 already feeds node 4: the wire would close a cycle",
        ),
        (Refusal::Backbone("the backbone".into()), "the backbone"),
        (
            Refusal::NotInChain(NodeId(6)),
            "node 6 is not in the live post chain",
        ),
        (
            Refusal::NoPosition {
                position: 3,
                chain: 2,
            },
            "the live post chain holds 2 post(s); no position 3",
        ),
        (Refusal::NoChange, "the edit leaves the graph as it is"),
        (
            Refusal::Graph(crate::contract::stain::graph::GraphError("why".into())),
            "why",
        ),
    ]
}

#[test]
fn stain_refusals_name_their_reasons() {
    for (r, want) in reasons() {
        check_reason(&r, want);
    }
}

validation::negative_control!(
    stain_refusals_name_their_reasons,
    "a refusal checked against another's reason must fail",
    expected = "reads otherwise",
    check_reason(&reasons()[0].0, reasons()[1].1)
);
