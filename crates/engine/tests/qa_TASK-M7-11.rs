//! QA's tests for TASK-M7-11, written from the requirements it closes and their sources, not from the code:
//!
//! - REQ-GUI-021 (render_gui_spec Part II §3, §3.1): each node kind's in-ports and out-port are §3.1's row, the
//!   ports directional and typed `field` / `vec3` / `f32`, a source's out-port subtype following its field.
//! - REQ-GUI-022 (§4, §7, §15): `OUT` and the combiner are never added, deleted or duplicated; a graph with two of
//!   either, or none, is refused on reading too; after any edit sequence there is exactly one of each.
//! - REQ-GUI-023 (§4, §15; a property): over random edit sequences the graph stays acyclic, each wire's out-port type is
//!   its in-port's, every wire that would close a cycle is refused, and the free interior takes any number of nodes.
//! - REQ-GUI-025 (§6): over every out-type × in-type pair a wire attaches only on equal types; a mismatch leaves the
//!   graph as it was; a categorical field onto a colour node in gradient mode attaches.
//! - REQ-GUI-026 (§6, §15): a second wire on an occupied in-port replaces the first; an out-port feeds five and more.
//! - REQ-GUI-016 (gui_state_contract §5): node ids, wires and per-node params round-trip through `RenderState`;
//!   picking an occupant is one `SetField`, one undo entry.
//! - REQ-COL-140 (colour_composition §9.4, "The custom-stop ramp"): the ramp's stops, positions, OKLab interpolation,
//!   clamped or cyclic ends, hard edge and invalid `t`; it round-trips through `RenderState`.
//!
//! Each test has its negative control (R-176). The OKLab conversion here is transcribed from Ottosson (2020), "A
//! perceptual color space for image processing", independently of the crate's (R-51).

use engine::contract::interface::EngineInterface;
use engine::contract::render_state::{Hover, Overlays, Palette, Playhead, RenderState};
use engine::contract::set_field::{Edit, RenderField, SetField};
use engine::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, KernelVariant, Links, Lock, Plane, Quality, SimConfig,
    Slice, SliceFrame, Window, Z_Q3,
};
use engine::contract::stain::edit::{apply, Refusal, StainEdit};
use engine::contract::stain::graph::{NodeId, NodeKind, Occupant, StainGraph};
use engine::contract::stain::ports::{Direction, PortType, Subtype};
use engine::contract::stain::ramp::{Ramp, Stop};
use engine::contract::store::StateStore;
use proptest::prelude::*;
use render::assemble::{self, Subtype as FieldSubtype};
use serde_json::{json, Value};
use validation::negative_control;

// ── Helpers ─────────────────────────────────────────────────────────────────────────────────────────────────────────

/// The graph a `SetField` writes, or `None` where it writes something else.
fn written(set: &SetField) -> Option<StainGraph> {
    match &set.edit {
        Edit::Render(RenderField::StainGraph(g)) => Some(g.clone()),
        _ => None,
    }
}

/// `edit` on `g`: the graph after it, or the refusal.
fn try_edit(g: &StainGraph, edit: StainEdit) -> Result<StainGraph, Refusal> {
    apply(g, &edit).map(|s| written(&s).expect("an accepted stain edit writes the stain graph"))
}

/// `edit` on `g`, which must be accepted.
fn must(g: &StainGraph, edit: StainEdit) -> StainGraph {
    try_edit(g, edit.clone()).unwrap_or_else(|r| panic!("{edit:?} was refused: {r}"))
}

/// The id of the node the graph has that `before` had not.
fn new_id(before: &StainGraph, after: &StainGraph) -> NodeId {
    let old: Vec<NodeId> = before.nodes().map(|n| n.id).collect();
    let fresh: Vec<NodeId> = after
        .nodes()
        .map(|n| n.id)
        .filter(|id| !old.contains(id))
        .collect();
    assert_eq!(fresh.len(), 1, "an add made {} new nodes", fresh.len());
    fresh[0]
}

fn add(g: &StainGraph, kind: NodeKind, occupant: Occupant) -> (StainGraph, NodeId) {
    let next = must(g, StainEdit::Add { kind, occupant });
    let id = new_id(g, &next);
    (next, id)
}

fn wire(g: &StainGraph, from: NodeId, to: NodeId, port: u32) -> StainGraph {
    must(g, StainEdit::Wire { from, to, port })
}

fn the(g: &StainGraph, kind: NodeKind) -> NodeId {
    let ids: Vec<NodeId> = g.nodes().filter(|n| n.kind == kind).map(|n| n.id).collect();
    assert_eq!(ids.len(), 1, "the graph holds {} {kind:?} nodes", ids.len());
    ids[0]
}

fn count(g: &StainGraph, kind: NodeKind) -> usize {
    g.nodes().filter(|n| n.kind == kind).count()
}

/// A source field of subtype `want`, from the scanned registry.
fn field_of(want: FieldSubtype) -> String {
    assemble::source_fields()
        .expect("the registry's fields")
        .into_iter()
        .find(|f| assemble::field_subtype(f) == Ok(want))
        .unwrap_or_else(|| panic!("no source field is {want:?}"))
}

/// §3.1's out-port type of a kind, read from the table, not from the code.
fn out_type(kind: NodeKind) -> Option<PortType> {
    match kind {
        NodeKind::Source => Some(PortType::Field),
        NodeKind::Colour | NodeKind::Combiner | NodeKind::Post => Some(PortType::Vec3),
        NodeKind::Brightness => Some(PortType::F32),
        NodeKind::Out => None,
    }
}

/// A post occupant with two field inputs after its colour.
const POST2: &str = "// @input m
// @input n
fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> { return rgb * ctx.inputs[0].x * ctx.inputs[1].x; }
";

/// A colour occupant with two field inputs.
const COLOUR2: &str = "// @input a
// @input b
fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.inputs[0].x, ctx.inputs[1].x, 0.0); }
";

/// A brightness occupant declaring two field inputs: §3.1 gives a brightness one.
const BRIGHT2: &str = "// @input a
// @input b
fn brightness(ctx: Ctx) -> f32 { return ctx.inputs[0].x * ctx.inputs[1].x; }
";

/// A brightness occupant with one uniform, `k`, in `[0, 2]`.
const BRIGHT_K: &str = "// @uniform k: f32 = 1.0 [0.0, 2.0]
fn brightness(ctx: Ctx) -> f32 { return ctx.inputs[0].x * uniforms.k; }
";

fn custom(text: &str) -> Occupant {
    Occupant::Custom(text.to_owned())
}

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

fn state(graph: StainGraph) -> RenderState {
    RenderState {
        stain_graph: graph,
        overlays: Overlays::default(),
        palette: Palette {},
        playhead: Playhead { t: 0.0 },
        hover: Hover::default(),
        debug_view: false,
    }
}

// ── REQ-GUI-021: node kinds and their ports ─────────────────────────────────────────────────────────────────────────

/// Node `id`'s ports as (in types, out types), each checked for its direction.
fn row(g: &StainGraph, id: NodeId) -> (Vec<PortType>, Vec<PortType>) {
    let p = g.ports_of(id).expect("ports");
    for i in &p.inputs {
        assert_eq!(
            i.direction,
            Direction::In,
            "node {} has an in-port marked out",
            id.0
        );
    }
    for o in &p.outputs {
        assert_eq!(
            o.direction,
            Direction::Out,
            "node {} has an out-port marked in",
            id.0
        );
    }
    (
        p.inputs.iter().map(|p| p.ty).collect(),
        p.outputs.iter().map(|p| p.ty).collect(),
    )
}

/// Checks each node's row against §3.1, given which ports the occupant declares.
fn check_kind_rows(g: &StainGraph, nodes: &[(NodeId, NodeKind, usize)]) {
    use PortType::{Field, Vec3, F32};
    for &(id, kind, declared) in nodes {
        let (ins, outs) = row(g, id);
        let want_out: Vec<PortType> = out_type(kind).into_iter().collect();
        assert_eq!(outs, want_out, "{kind:?}'s out-port is not §3.1's");
        let want_in: Vec<PortType> = match kind {
            NodeKind::Source => vec![],
            NodeKind::Colour => vec![Field; declared.max(1)],
            NodeKind::Brightness => vec![Field],
            NodeKind::Combiner => vec![Vec3, F32],
            NodeKind::Post => [vec![Vec3], vec![Field; declared]].concat(),
            NodeKind::Out => vec![Vec3],
        };
        assert_eq!(ins, want_in, "{kind:?}'s in-ports are not §3.1's row");
    }
}

fn every_kind() -> (StainGraph, Vec<(NodeId, NodeKind, usize)>) {
    let g = StainGraph::new();
    let scalar = field_of(FieldSubtype::Scalar);
    let (g, s) = add(&g, NodeKind::Source, Occupant::Field(scalar));
    let (g, c) = add(&g, NodeKind::Colour, Occupant::None);
    let (g, c2) = add(&g, NodeKind::Colour, custom(COLOUR2));
    let (g, b) = add(&g, NodeKind::Brightness, Occupant::None);
    let (g, p) = add(&g, NodeKind::Post, Occupant::None);
    let (g, p2) = add(&g, NodeKind::Post, custom(POST2));
    let combiner = the(&g, NodeKind::Combiner);
    let out = the(&g, NodeKind::Out);
    let nodes = vec![
        (s, NodeKind::Source, 0),
        (c, NodeKind::Colour, 1),
        (c2, NodeKind::Colour, 2),
        (b, NodeKind::Brightness, 1),
        (p, NodeKind::Post, 0),
        (p2, NodeKind::Post, 2),
        (combiner, NodeKind::Combiner, 0),
        (out, NodeKind::Out, 0),
    ];
    (g, nodes)
}

#[test]
fn qa_stain_node_kinds_rows() {
    let (g, nodes) = every_kind();
    // Every one of §3's six kinds is present.
    for k in [
        NodeKind::Source,
        NodeKind::Colour,
        NodeKind::Brightness,
        NodeKind::Combiner,
        NodeKind::Post,
        NodeKind::Out,
    ] {
        assert!(count(&g, k) >= 1, "no {k:?} node");
    }
    check_kind_rows(&g, &nodes);
}

negative_control!(
    qa_stain_node_kinds_rows,
    "a brightness row read as vec3 out must fail",
    expected = "out-port is not §3.1's",
    {
        let (g, nodes) = every_kind();
        // Claim the colour node is a brightness: §3.1's brightness row has an f32 out.
        let swapped: Vec<_> = nodes
            .into_iter()
            .map(|(id, k, d)| {
                if k == NodeKind::Colour {
                    (id, NodeKind::Brightness, d)
                } else {
                    (id, k, d)
                }
            })
            .collect();
        check_kind_rows(&g, &swapped)
    }
);

/// A source's out-port subtype follows the field it reads (§3.1 notes; §3: scalar / vector / categorical / point⟨q⟩),
/// and a colour node's in-port reports the subtype wired into it, which picks its inspector (§9).
fn check_subtypes(scalar: &str, categorical: &str) {
    let g = StainGraph::new();
    let (g, s) = add(&g, NodeKind::Source, Occupant::Field(scalar.to_owned()));
    let (g, k) = add(
        &g,
        NodeKind::Source,
        Occupant::Field(categorical.to_owned()),
    );
    let (g, c) = add(&g, NodeKind::Colour, Occupant::None);
    let out = |g: &StainGraph, id| g.ports_of(id).unwrap().outputs[0].subtype;
    assert_eq!(
        out(&g, s),
        Some(Subtype::Scalar),
        "a scalar field's source is not scalar"
    );
    assert_eq!(
        out(&g, k),
        Some(Subtype::Categorical),
        "a categorical field's source is not categorical"
    );
    let g = wire(&g, s, c, 0);
    assert_eq!(
        g.ports_of(c).unwrap().inputs[0].subtype,
        Some(Subtype::Scalar)
    );
    let g = wire(&g, k, c, 0);
    assert_eq!(
        g.ports_of(c).unwrap().inputs[0].subtype,
        Some(Subtype::Categorical),
        "the colour node's in-port did not take the categorical subtype"
    );
}

#[test]
fn qa_stain_source_subtype_follows_field() {
    check_subtypes(
        &field_of(FieldSubtype::Scalar),
        &field_of(FieldSubtype::Categorical),
    );
}

negative_control!(
    qa_stain_source_subtype_follows_field,
    "a scalar field passed as the categorical one must fail",
    expected = "categorical field's source is not categorical",
    {
        let s = field_of(FieldSubtype::Scalar);
        check_subtypes(&s, &s)
    }
);

/// §3.1: a brightness takes one `field`; an occupant declaring two is refused, and the graph is unchanged.
fn check_brightness_one(text: &str) {
    let g = StainGraph::new();
    let r = try_edit(
        &g,
        StainEdit::Add {
            kind: NodeKind::Brightness,
            occupant: custom(text),
        },
    );
    assert!(
        r.is_err(),
        "a brightness with two field inputs was accepted"
    );
}

#[test]
fn qa_stain_brightness_takes_one_field() {
    check_brightness_one(BRIGHT2);
}

negative_control!(
    qa_stain_brightness_takes_one_field,
    "a one-input brightness is accepted, so the check fails",
    expected = "with two field inputs was accepted",
    check_brightness_one(BRIGHT_K)
);

// ── REQ-GUI-022: fixed singletons ───────────────────────────────────────────────────────────────────────────────────

/// Every add, delete and duplicate of the combiner and `OUT` is refused, leaving exactly one of each.
fn check_singletons(g: &StainGraph) {
    let combiner = the(g, NodeKind::Combiner);
    let out = the(g, NodeKind::Out);
    let edits = [
        StainEdit::Add {
            kind: NodeKind::Combiner,
            occupant: Occupant::Builtin("multiply".into()),
        },
        StainEdit::Add {
            kind: NodeKind::Combiner,
            occupant: Occupant::Builtin("pass_through".into()),
        },
        StainEdit::Add {
            kind: NodeKind::Out,
            occupant: Occupant::None,
        },
        StainEdit::Delete { node: combiner },
        StainEdit::Delete { node: out },
        StainEdit::Duplicate { node: combiner },
        StainEdit::Duplicate { node: out },
    ];
    for e in edits {
        if let Ok(next) = try_edit(g, e.clone()) {
            panic!(
                "{e:?} was accepted: {} combiner(s), {} OUT(s)",
                count(&next, NodeKind::Combiner),
                count(&next, NodeKind::Out)
            );
        }
    }
}

#[test]
fn qa_stain_singletons_refused() {
    check_singletons(&StainGraph::new());
    let (g, nodes) = every_kind();
    let _ = nodes;
    check_singletons(&g);
    // A free node is deleted and duplicated freely: the refusal is the singletons' alone.
    let (g, c) = add(&g, NodeKind::Colour, Occupant::None);
    let dup = must(&g, StainEdit::Duplicate { node: c });
    assert_eq!(
        count(&dup, NodeKind::Colour),
        count(&g, NodeKind::Colour) + 1
    );
    let del = must(&g, StainEdit::Delete { node: c });
    assert_eq!(
        count(&del, NodeKind::Colour),
        count(&g, NodeKind::Colour) - 1
    );
}

negative_control!(
    qa_stain_singletons_refused,
    "a free colour's duplicate is accepted, so treating it as a singleton fails",
    expected = "was accepted",
    {
        let g = StainGraph::new();
        let (g, c) = add(&g, NodeKind::Colour, Occupant::None);
        if try_edit(&g, StainEdit::Duplicate { node: c }).is_ok() {
            panic!("Duplicate {{ node: {c:?} }} was accepted");
        }
    }
);

/// The graph as `RenderState` JSON, with its `stain_graph` replaced by `graph`.
fn state_json_with(graph: Value) -> String {
    let mut v = serde_json::to_value(state(StainGraph::new())).expect("json");
    v["stain_graph"] = graph;
    v.to_string()
}

/// Reads a `RenderState` whose graph is `graph`; must be refused.
fn check_read_refused(graph: Value, what: &str) {
    let text = state_json_with(graph);
    if serde_json::from_str::<RenderState>(&text).is_ok() {
        panic!("a RenderState whose graph {what} was read");
    }
}

fn backbone_json() -> Value {
    json!({
        "nodes": [
            {"id": 0, "kind": "combiner", "occupant": {"builtin": "pass_through"}},
            {"id": 1, "kind": "out", "occupant": "none"}
        ],
        "wires": [{"from": 0, "to": 1, "port": 0}]
    })
}

#[test]
fn qa_stain_singletons_on_read() {
    // The hand-written backbone reads: the check below refuses on the singletons, not on the form.
    let ok: RenderState =
        serde_json::from_str(&state_json_with(backbone_json())).expect("the backbone reads");
    assert_eq!(count(&ok.stain_graph, NodeKind::Combiner), 1);
    assert_eq!(count(&ok.stain_graph, NodeKind::Out), 1);

    let mut two_combiners = backbone_json();
    two_combiners["nodes"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id": 2, "kind": "combiner", "occupant": {"builtin": "multiply"}}));
    check_read_refused(two_combiners, "has two combiners");

    let mut two_outs = backbone_json();
    two_outs["nodes"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id": 2, "kind": "out", "occupant": "none"}));
    check_read_refused(two_outs, "has two OUTs");

    check_read_refused(
        json!({"nodes": [{"id": 1, "kind": "out", "occupant": "none"}], "wires": []}),
        "has no combiner",
    );
    check_read_refused(
        json!({"nodes": [{"id": 0, "kind": "combiner", "occupant": {"builtin": "pass_through"}}], "wires": []}),
        "has no OUT",
    );
}

negative_control!(
    qa_stain_singletons_on_read,
    "the valid backbone is read, so asserting it is refused fails",
    expected = "was read",
    check_read_refused(backbone_json(), "is the plain backbone")
);

// ── REQ-GUI-023: acyclic, type-compatible, any number of free nodes (a property) ────────────────────────────────────

/// One random editor action, by indices into the graph's nodes at the time.
#[derive(Clone, Debug)]
enum Op {
    Add(u8),
    Wire(u8, u8, u8),
    Unwire(u8, u8),
    Delete(u8),
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        2 => (0u8..5).prop_map(Op::Add),
        6 => (any::<u8>(), any::<u8>(), 0u8..3).prop_map(|(a, b, p)| Op::Wire(a, b, p)),
        1 => (any::<u8>(), 0u8..3).prop_map(|(a, p)| Op::Unwire(a, p)),
        1 => any::<u8>().prop_map(Op::Delete),
    ]
}

/// Whether `to` reaches `from` along the wires (each wire `a → b`), so that a wire `from → to` closes a cycle; also
/// true for `from == to`.
fn reaches(wires: &[(NodeId, NodeId)], to: NodeId, from: NodeId) -> bool {
    let mut stack = vec![to];
    let mut seen = vec![];
    while let Some(n) = stack.pop() {
        if n == from {
            return true;
        }
        if seen.contains(&n) {
            continue;
        }
        seen.push(n);
        for &(a, b) in wires {
            if a == n {
                stack.push(b);
            }
        }
    }
    false
}

/// Whether the wires hold a cycle: some wire `a → b` with `b` reaching `a`.
fn has_cycle(wires: &[(NodeId, NodeId)]) -> bool {
    wires.iter().any(|&(a, b)| reaches(wires, b, a))
}

fn pairs(g: &StainGraph) -> Vec<(NodeId, NodeId)> {
    g.wires().iter().map(|w| (w.from, w.to)).collect()
}

/// The checks of one graph: acyclic, each wire's out-type its in-port's (§3.1's out types, the port's own in type),
/// one combiner, one OUT, and at most one wire per in-port.
fn check_graph(g: &StainGraph) -> Result<(), String> {
    let w = pairs(g);
    if has_cycle(&w) {
        return Err(format!("the graph holds a cycle: {w:?}"));
    }
    if count(g, NodeKind::Combiner) != 1 || count(g, NodeKind::Out) != 1 {
        return Err("the graph does not hold one combiner and one OUT".into());
    }
    let mut seen = vec![];
    for wire in g.wires() {
        let from = g.node(wire.from).ok_or("a wire from no node")?;
        let ins = g.ports_of(wire.to).map_err(|e| e.to_string())?.inputs;
        let ty = ins.get(wire.port as usize).ok_or("a wire into no port")?.ty;
        if out_type(from.kind) != Some(ty) {
            return Err(format!(
                "a wire {:?} → {:?} port {} joins {:?} to {:?}",
                wire.from,
                wire.to,
                wire.port,
                out_type(from.kind),
                ty
            ));
        }
        if seen.contains(&(wire.to, wire.port)) {
            return Err(format!(
                "in-port {:?}.{} holds two wires",
                wire.to, wire.port
            ));
        }
        seen.push((wire.to, wire.port));
    }
    Ok(())
}

/// Runs `ops` from the backbone, checking each graph, and that every wire closing a cycle (by [`reaches`]) is refused
/// and every refused edit leaves the graph as it was. Returns how many cycle-closing wires were tried.
fn run_ops(ops: &[Op], cycle_ok: bool) -> Result<usize, String> {
    let kinds = [
        (
            NodeKind::Source,
            Occupant::Field(field_of(FieldSubtype::Scalar)),
        ),
        (NodeKind::Colour, Occupant::None),
        (NodeKind::Brightness, Occupant::None),
        (NodeKind::Post, Occupant::None),
        (NodeKind::Post, custom(POST2)),
    ];
    let mut g = StainGraph::new();
    // Start with three posts on hand so post → post cycles are reachable early.
    for _ in 0..3 {
        g = add(&g, NodeKind::Post, Occupant::None).0;
    }
    let mut tried = 0;
    for o in ops {
        let ids: Vec<NodeId> = g.nodes().map(|n| n.id).collect();
        let pick = |i: u8| ids[i as usize % ids.len()];
        let edit = match o {
            Op::Add(k) => {
                let (kind, occ) = kinds[*k as usize % kinds.len()].clone();
                StainEdit::Add {
                    kind,
                    occupant: occ,
                }
            }
            Op::Wire(a, b, p) => StainEdit::Wire {
                from: pick(*a),
                to: pick(*b),
                port: u32::from(*p),
            },
            Op::Unwire(a, p) => StainEdit::Unwire {
                to: pick(*a),
                port: u32::from(*p),
            },
            Op::Delete(a) => StainEdit::Delete { node: pick(*a) },
        };
        let closes = match &edit {
            StainEdit::Wire { from, to, .. } => reaches(&pairs(&g), *to, *from),
            _ => false,
        };
        let types_match = match &edit {
            StainEdit::Wire { from, to, port } => {
                let ins = g.ports_of(*to).unwrap().inputs;
                ins.get(*port as usize).map(|p| p.ty) == out_type(g.node(*from).unwrap().kind)
            }
            _ => true,
        };
        if closes && types_match {
            tried += 1;
        }
        match apply(&g, &edit) {
            Ok(set) => {
                if closes && !cycle_ok {
                    return Err(format!("{edit:?} closes a cycle and was accepted"));
                }
                if set.no_history {
                    return Err(format!("{edit:?} is not undoable"));
                }
                g = written(&set).ok_or("an edit wrote no stain graph")?;
                check_graph(&g)?;
            }
            // §4: a wire is refused only for its types, a cycle, or the backbone's tail `combiner → (post)* → OUT`
            // with its live chain of at most 8 posts (colour_composition §9.2, §4.2); dropping a wire where it
            // already is changes nothing.
            Err(r) => {
                if let StainEdit::Wire { .. } = edit {
                    let allowed = matches!(
                        r,
                        Refusal::TypeMismatch { .. }
                            | Refusal::NoPort { .. }
                            | Refusal::Cycle { .. }
                            | Refusal::Backbone(_)
                            | Refusal::NoChange
                    ) || matches!(&r, Refusal::Graph(e) if e.0.contains("post chain holds"));
                    if types_match && !closes && !allowed {
                        return Err(format!("{edit:?} was refused for another reason: {r}"));
                    }
                }
            }
        }
    }
    Ok(tried)
}

#[test]
fn qa_stain_wires_acyclic_property() {
    let strategy = prop::collection::vec(op(), 1..60);
    validation::prop::run(&strategy, |ops| {
        run_ops(&ops, false).map_err(TestCaseError::fail)?;
        Ok(())
    });
}

/// A deterministic sequence that does try cycle-closing wires between posts, so the property's refusal arm is
/// reached, not only possible: p1 → p2 → p3, then p3 → p1 and p2 → p2.
fn post_cycle(cycle_ok: bool) -> Result<usize, String> {
    // Nodes after the backbone and three posts: ids 0 (combiner), 1 (OUT), 2, 3, 4 (posts), in id order.
    run_ops(
        &[
            Op::Wire(2, 3, 0),
            Op::Wire(3, 4, 0),
            Op::Wire(4, 2, 0),
            Op::Wire(3, 3, 0),
        ],
        cycle_ok,
    )
}

#[test]
fn qa_stain_cycle_closing_wire_refused() {
    let tried = post_cycle(false).expect("the sequence holds");
    assert_eq!(
        tried, 2,
        "the sequence did not try its two cycle-closing wires"
    );
    // The direct check too: p3 → p1 after p1 → p2 → p3 is refused as a cycle, and the graph unchanged.
    let g = StainGraph::new();
    let (g, p1) = add(&g, NodeKind::Post, Occupant::None);
    let (g, p2) = add(&g, NodeKind::Post, Occupant::None);
    let (g, p3) = add(&g, NodeKind::Post, Occupant::None);
    let g = wire(&wire(&g, p1, p2, 0), p2, p3, 0);
    match apply(
        &g,
        &StainEdit::Wire {
            from: p3,
            to: p1,
            port: 0,
        },
    ) {
        Err(Refusal::Cycle { .. }) => {}
        other => panic!("a cycle-closing wire was not refused as a cycle: {other:?}"),
    }
    assert!(matches!(
        apply(
            &g,
            &StainEdit::Wire {
                from: p2,
                to: p2,
                port: 0
            }
        ),
        Err(Refusal::Cycle { .. })
    ));
}

negative_control!(
    qa_stain_wires_acyclic_property,
    "the property's own cycle detector must catch an accepted cycle: a cycle-closing wire counted as accepted fails",
    expected = "holds a cycle",
    {
        let p = |n| NodeId(n);
        let w = [(p(2), p(3)), (p(3), p(4)), (p(4), p(2))];
        assert!(!has_cycle(&w), "the graph holds a cycle: {w:?}");
    }
);

negative_control!(
    qa_stain_cycle_closing_wire_refused,
    "a sequence that tries no cycle-closing wire must fail the count",
    expected = "did not try its two cycle-closing wires",
    {
        let tried = run_ops(&[Op::Wire(2, 3, 0)], false).expect("holds");
        assert_eq!(
            tried, 2,
            "the sequence did not try its two cycle-closing wires"
        );
    }
);

/// §4: the free interior takes any number of source, colour, brightness and post nodes.
fn check_many(n: usize) {
    let mut g = StainGraph::new();
    let kinds = [
        (
            NodeKind::Source,
            Occupant::Field(field_of(FieldSubtype::Scalar)),
        ),
        (NodeKind::Colour, Occupant::None),
        (NodeKind::Brightness, Occupant::None),
        (NodeKind::Post, Occupant::None),
    ];
    for (kind, occ) in &kinds {
        for _ in 0..n {
            g = add(&g, *kind, occ.clone()).0;
        }
    }
    for (kind, _) in &kinds {
        assert_eq!(
            count(&g, *kind),
            n,
            "the graph does not hold {n} {kind:?} nodes"
        );
    }
    // One source fans out to every colour and brightness, all accepted.
    let src = g.nodes().find(|x| x.kind == NodeKind::Source).unwrap().id;
    let targets: Vec<NodeId> = g
        .nodes()
        .filter(|x| matches!(x.kind, NodeKind::Colour | NodeKind::Brightness))
        .map(|x| x.id)
        .collect();
    for t in targets {
        g = wire(&g, src, t, 0);
    }
    check_graph(&g).expect("the graph holds");
}

#[test]
fn qa_stain_free_interior_any_number() {
    check_many(20);
}

negative_control!(
    qa_stain_free_interior_any_number,
    "the count check fails when the graph holds fewer than claimed",
    expected = "does not hold",
    {
        let g = StainGraph::new();
        let g = add(&g, NodeKind::Colour, Occupant::None).0;
        assert_eq!(
            count(&g, NodeKind::Colour),
            2,
            "the graph does not hold 2 Colour nodes"
        );
    }
);

// ── REQ-GUI-025: a wire attaches only on matching types ─────────────────────────────────────────────────────────────

/// A graph with a representative of each out-type and in-type: (graph, outs by type, ins by type).
#[allow(clippy::type_complexity)]
fn typed() -> (
    StainGraph,
    Vec<(PortType, NodeId)>,
    Vec<(PortType, NodeId, u32)>,
) {
    let g = StainGraph::new();
    let combiner = the(&g, NodeKind::Combiner);
    let out = the(&g, NodeKind::Out);
    let (g, s) = add(
        &g,
        NodeKind::Source,
        Occupant::Field(field_of(FieldSubtype::Scalar)),
    );
    let (g, c) = add(&g, NodeKind::Colour, Occupant::None);
    let (g, b) = add(&g, NodeKind::Brightness, Occupant::None);
    let (g, p) = add(&g, NodeKind::Post, custom(POST2));
    let outs = vec![
        (PortType::Field, s),
        (PortType::Vec3, c),
        (PortType::Vec3, combiner),
        (PortType::Vec3, p),
        (PortType::F32, b),
    ];
    let ins = vec![
        (PortType::Field, c, 0),
        (PortType::Field, b, 0),
        (PortType::Field, p, 1),
        (PortType::Vec3, combiner, 0),
        (PortType::Vec3, p, 0),
        (PortType::Vec3, out, 0),
        (PortType::F32, combiner, 1),
    ];
    (g, outs, ins)
}

/// Every mismatched pair is refused with the graph unchanged; returns the number tried.
fn check_mismatches(
    g: &StainGraph,
    outs: &[(PortType, NodeId)],
    ins: &[(PortType, NodeId, u32)],
) -> usize {
    let mut n = 0;
    for &(ot, from) in outs {
        for &(it, to, port) in ins {
            if ot == it || from == to {
                continue;
            }
            n += 1;
            let before = g.clone();
            match apply(g, &StainEdit::Wire { from, to, port }) {
                Ok(_) => panic!("a {ot:?} → {it:?} wire ({from:?} → {to:?}.{port}) attached"),
                Err(Refusal::TypeMismatch { .. }) => {}
                Err(other) => panic!(
                    "a {ot:?} → {it:?} wire was refused, but not as a type mismatch: {other}"
                ),
            }
            assert_eq!(*g, before, "a refused wire changed the graph");
        }
    }
    n
}

#[test]
fn qa_stain_wire_types_matrix() {
    let (g, outs, ins) = typed();
    let n = check_mismatches(&g, &outs, &ins);
    // 5 outs × 7 ins less the matching pairs and the 4 self-wires (a node into itself): 18, every one tried.
    assert_eq!(n, 18, "{n} mismatched pairs tried, not the matrix's 18");
    // The two the verify detail names.
    let (c, b) = (outs[1].1, outs[4].1);
    let combiner = the(&g, NodeKind::Combiner);
    let s = outs[0].1;
    assert!(
        apply(
            &g,
            &StainEdit::Wire {
                from: c,
                to: combiner,
                port: 1
            }
        )
        .is_err(),
        "vec3 → f32 attached"
    );
    assert!(
        apply(
            &g,
            &StainEdit::Wire {
                from: s,
                to: combiner,
                port: 0
            }
        )
        .is_err(),
        "field → vec3 attached"
    );
    // Matching drops attach: field → field (three ports), vec3 → vec3 on the backbone, f32 → f32.
    let mut h = g.clone();
    for (from, to, port) in [
        (s, c, 0),
        (s, b, 0),
        (s, outs[3].1, 1),
        (c, combiner, 0),
        (b, combiner, 1),
        (combiner, outs[3].1, 0),
    ] {
        h = wire(&h, from, to, port);
        assert!(
            h.wires()
                .iter()
                .any(|w| w.from == from && w.to == to && w.port == port),
            "a matching wire did not attach"
        );
    }
}

negative_control!(
    qa_stain_wire_types_matrix,
    "a matching pair fed to the mismatch check attaches, so the check fails",
    expected = "attached",
    {
        let (g, outs, ins) = typed();
        // Pretend the source's out-port is a vec3: the source → colour field wire is then a "mismatch".
        let lying = [(PortType::Vec3, outs[0].1)];
        let lying_in = [(PortType::Field, ins[0].1, ins[0].2)];
        check_mismatches(&g, &lying, &lying_in);
    }
);

/// §6: subtype is not a gate. A colour node in gradient mode (a scalar wired, its ramp set) takes a categorical field.
fn check_categorical_onto_gradient(categorical: &str) {
    let g = StainGraph::new();
    let (g, s) = add(
        &g,
        NodeKind::Source,
        Occupant::Field(field_of(FieldSubtype::Scalar)),
    );
    let (g, k) = add(
        &g,
        NodeKind::Source,
        Occupant::Field(categorical.to_owned()),
    );
    let (g, c) = add(&g, NodeKind::Colour, Occupant::None);
    let g = wire(&g, s, c, 0);
    let g = must(
        &g,
        StainEdit::Ramp {
            node: c,
            ramp: Some(two_stop(false)),
        },
    );
    let g = match try_edit(
        &g,
        StainEdit::Wire {
            from: k,
            to: c,
            port: 0,
        },
    ) {
        Ok(g) => g,
        Err(r) => panic!("a categorical field onto a gradient colour node was refused: {r}"),
    };
    assert_eq!(
        g.ports_of(c).unwrap().inputs[0].subtype,
        Some(Subtype::Categorical),
        "the colour node did not see the categorical subtype"
    );
}

#[test]
fn qa_stain_subtype_not_a_gate() {
    check_categorical_onto_gradient(&field_of(FieldSubtype::Categorical));
}

negative_control!(
    qa_stain_subtype_not_a_gate,
    "a scalar field is not categorical, so the subtype check fails",
    expected = "did not see the categorical subtype",
    check_categorical_onto_gradient(&field_of(FieldSubtype::Scalar))
);

// ── REQ-GUI-026: in-port arity one, out-port unbounded ──────────────────────────────────────────────────────────────

/// The wires into `to`.`port`.
fn into(g: &StainGraph, to: NodeId, port: u32) -> Vec<NodeId> {
    g.wires()
        .iter()
        .filter(|w| w.to == to && w.port == port)
        .map(|w| w.from)
        .collect()
}

/// Drops `first` then `second` on `c`'s in-port 0; returns who holds it.
fn last_write(swap: bool) -> Vec<NodeId> {
    let g = StainGraph::new();
    let scalar = field_of(FieldSubtype::Scalar);
    let (g, s1) = add(&g, NodeKind::Source, Occupant::Field(scalar.clone()));
    let (g, s2) = add(&g, NodeKind::Source, Occupant::Field(scalar));
    let (g, c) = add(&g, NodeKind::Colour, Occupant::None);
    let (first, second) = if swap { (s2, s1) } else { (s1, s2) };
    let g = wire(&wire(&g, first, c, 0), second, c, 0);
    let held = into(&g, c, 0);
    assert_eq!(
        held,
        vec![s2],
        "the in-port holds {held:?}, not the last wire alone"
    );
    held
}

#[test]
fn qa_stain_in_port_last_write_wins() {
    last_write(false);
    // On the combiner's colour in-port too: a second colour node replaces the first.
    let g = StainGraph::new();
    let combiner = the(&g, NodeKind::Combiner);
    let (g, c1) = add(&g, NodeKind::Colour, Occupant::None);
    let (g, c2) = add(&g, NodeKind::Colour, Occupant::None);
    let g = wire(&wire(&g, c1, combiner, 0), c2, combiner, 0);
    assert_eq!(into(&g, combiner, 0), vec![c2]);
    // And OUT's: a post dropped on OUT replaces the combiner's wire.
    let out = the(&g, NodeKind::Out);
    let (g, p) = add(&g, NodeKind::Post, Occupant::None);
    let g = wire(&g, p, out, 0);
    assert_eq!(into(&g, out, 0), vec![p]);
}

negative_control!(
    qa_stain_in_port_last_write_wins,
    "the first wire dropped is not the one held",
    expected = "not the last wire alone",
    {
        last_write(true);
    }
);

/// One source's out-port feeding `n` colour nodes.
fn fan_out(n: usize) -> usize {
    let g = StainGraph::new();
    let (mut g, s) = add(
        &g,
        NodeKind::Source,
        Occupant::Field(field_of(FieldSubtype::Scalar)),
    );
    for _ in 0..n {
        let (h, c) = add(&g, NodeKind::Colour, Occupant::None);
        g = wire(&h, s, c, 0);
    }
    g.wires().iter().filter(|w| w.from == s).count()
}

#[test]
fn qa_stain_out_port_fan_out() {
    assert_eq!(fan_out(5), 5, "an out-port does not hold five wires");
    assert_eq!(fan_out(40), 40, "an out-port does not hold forty wires");
}

negative_control!(
    qa_stain_out_port_fan_out,
    "four wires are not five",
    expected = "does not hold five",
    assert_eq!(fan_out(4), 5, "an out-port does not hold five wires")
);

// ── REQ-GUI-016: RenderState round-trip; picking an occupant is one SetField ────────────────────────────────────────

/// A wired graph with ids, wires, a param and a ramp.
fn full() -> StainGraph {
    let g = StainGraph::new();
    let combiner = the(&g, NodeKind::Combiner);
    let out = the(&g, NodeKind::Out);
    let (g, s) = add(
        &g,
        NodeKind::Source,
        Occupant::Field(field_of(FieldSubtype::Scalar)),
    );
    let (g, c) = add(&g, NodeKind::Colour, Occupant::None);
    let (g, b) = add(&g, NodeKind::Brightness, custom(BRIGHT_K));
    let (g, p) = add(&g, NodeKind::Post, Occupant::None);
    let g = must(
        &g,
        StainEdit::Param {
            node: b,
            name: "k".into(),
            value: vec![1.25],
        },
    );
    let g = must(
        &g,
        StainEdit::Ramp {
            node: c,
            ramp: Some(two_stop(true)),
        },
    );
    let mut g = g;
    for (f, t, port) in [
        (s, c, 0),
        (s, b, 0),
        (c, combiner, 0),
        (b, combiner, 1),
        (combiner, p, 0),
        (p, out, 0),
    ] {
        g = wire(&g, f, t, port);
    }
    g
}

fn check_round_trip(g: StainGraph, corrupt: bool) {
    let st = state(g.clone());
    let mut text = serde_json::to_string(&st).expect("serialise");
    if corrupt {
        text = text.replacen("1.25", "1.5", 1);
    }
    let back: RenderState = serde_json::from_str(&text).expect("read back");
    assert_eq!(back, st, "the RenderState did not round-trip");
    // Ids, wires and params survive by value, not only by equality of the whole.
    let ids = |g: &StainGraph| {
        g.nodes()
            .map(|n| (n.id, n.kind, n.occupant.clone(), n.params.clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(&back.stain_graph), ids(&g));
    assert_eq!(back.stain_graph.wires(), g.wires());
}

#[test]
fn qa_stain_renderstate_roundtrip() {
    let g = full();
    // The graph is not the default: it carries ids past the backbone, wires and a param.
    assert!(g.nodes().count() >= 6 && g.wires().len() >= 6);
    assert!(g.nodes().any(|n| n.params.get("k") == Some(&vec![1.25])));
    check_round_trip(g, false);
}

negative_control!(
    qa_stain_renderstate_roundtrip,
    "a param changed in the text fails the round trip",
    expected = "did not round-trip",
    check_round_trip(full(), true)
);

/// Picking `occupant` for node `node`: one `SetField`, undoable, which one undo reverses whole. Returns the number of
/// undo entries the pick made.
fn check_pick(node_kind: NodeKind, occupant: Occupant, picks: usize) -> u32 {
    let g = full();
    let node = g.nodes().find(|n| n.kind == node_kind).unwrap().id;
    let mut store = StateStore::new(sim(), state(g.clone()));
    let before = store.snapshot().history;
    let set = apply(
        &g,
        &StainEdit::Occupant {
            node,
            occupant: occupant.clone(),
        },
    )
    .expect("the pick is accepted");
    assert!(!set.no_history, "the pick is not undoable");
    let next = written(&set).expect("the pick writes the stain graph");
    assert_eq!(next.node(node).unwrap().occupant, occupant);
    // Every other node, and the node's id, unchanged.
    for n in g.nodes().filter(|n| n.id != node) {
        assert_eq!(
            next.node(n.id),
            Some(n),
            "picking an occupant changed another node"
        );
    }
    for _ in 0..picks {
        store.set_field(set.clone());
    }
    assert_eq!(store.snapshot().render.stain_graph, next);
    let after = store.snapshot().history;
    let entries = after.undo_depth - before.undo_depth;
    store.undo();
    assert_eq!(
        entries, 1,
        "picking an occupant made {entries} undo entries"
    );
    assert_eq!(
        store.snapshot().render.stain_graph,
        g,
        "one undo did not restore the graph"
    );
    entries
}

#[test]
fn qa_stain_pick_occupant_one_setfield() {
    check_pick(NodeKind::Combiner, Occupant::Builtin("multiply".into()), 1);
    let other = assemble::source_fields()
        .unwrap()
        .into_iter()
        .find(|f| {
            *f != field_of(FieldSubtype::Scalar)
                && assemble::field_subtype(f) == Ok(FieldSubtype::Scalar)
        })
        .expect("a second scalar field");
    check_pick(NodeKind::Source, Occupant::Field(other), 1);
}

negative_control!(
    qa_stain_pick_occupant_one_setfield,
    "two picks sent make two entries, so the one-entry check fails",
    expected = "made 2 undo entries",
    {
        check_pick(NodeKind::Combiner, Occupant::Builtin("multiply".into()), 2);
    }
);

// ── REQ-COL-140: the custom-stop ramp (colour_composition §9.4) ─────────────────────────────────────────────────────

/// Ottosson (2020): linear sRGB → OKLab.
fn ottosson_forward([r, g, b]: [f64; 3]) -> [f64; 3] {
    let l = 0.412_221_470_8 * r + 0.536_332_536_3 * g + 0.051_445_992_9 * b;
    let m = 0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b;
    let s = 0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b;
    let (l, m, s) = (l.cbrt(), m.cbrt(), s.cbrt());
    [
        0.210_454_255_3 * l + 0.793_617_785_0 * m - 0.004_072_046_8 * s,
        1.977_998_495_1 * l - 2.428_592_205_0 * m + 0.450_593_709_9 * s,
        0.025_904_037_1 * l + 0.782_771_766_2 * m - 0.808_675_766_0 * s,
    ]
}

/// Ottosson (2020): OKLab → linear sRGB.
fn ottosson_inverse([l, a, b]: [f64; 3]) -> [f64; 3] {
    let l_ = l + 0.396_337_777_4 * a + 0.215_803_757_3 * b;
    let m_ = l - 0.105_561_345_8 * a - 0.063_854_172_8 * b;
    let s_ = l - 0.089_484_177_5 * a - 1.291_485_548_0 * b;
    let (l, m, s) = (l_.powi(3), m_.powi(3), s_.powi(3));
    [
        4.076_741_662_1 * l - 3.307_711_591_3 * m + 0.230_969_929_2 * s,
        -1.268_438_004_6 * l + 2.609_757_401_1 * m - 0.341_319_396_5 * s,
        -0.004_196_086_3 * l - 0.703_418_614_7 * m + 1.707_614_701_0 * s,
    ]
}

fn mix(a: [f64; 3], b: [f64; 3], u: f64) -> [f64; 3] {
    [0, 1, 2].map(|i| a[i] + u * (b[i] - a[i]))
}

/// Agreement of two f64 evaluations of the same formula: only rounding separates them, a few ulps of values of order
/// 1, so 1e-12; the rgb-space control differs by order 1e-2.
fn near(a: [f64; 3], b: [f64; 3]) -> bool {
    (0..3).all(|i| (a[i] - b[i]).abs() <= 1e-12)
}

const RED: [f64; 3] = [0.627_955, 0.224_863, 0.125_846];
const BLUE: [f64; 3] = [0.452_014, -0.032_457, -0.311_528];
const GREEN: [f64; 3] = [0.866_440, -0.233_888, 0.179_498];

fn stop(position: f64, oklab: [f64; 3]) -> Stop {
    Stop { position, oklab }
}

fn two_stop(cyclic: bool) -> Ramp {
    Ramp::new(vec![stop(0.2, RED), stop(0.8, BLUE)], cyclic).expect("a ramp")
}

#[test]
fn qa_ottosson_transcription() {
    // Our forward transform reproduces the published OKLab of the sRGB primaries (Ottosson's reference values, as
    // tabulated to 6 decimals: the bound is half the last digit), and the crate's inverse inverts it to the precision
    // of the 10-digit published coefficients.
    for (rgb, lab) in [
        ([1.0, 0.0, 0.0], RED),
        ([0.0, 1.0, 0.0], GREEN),
        ([0.0, 0.0, 1.0], BLUE),
    ] {
        let f = ottosson_forward(rgb);
        for i in 0..3 {
            assert!(
                (f[i] - lab[i]).abs() <= 5e-6,
                "OKLab of {rgb:?} is {f:?}, not {lab:?}"
            );
        }
    }
    // The crate's OKLab → linear sRGB is Ottosson's inverse, transcribed here independently: only f64 rounding
    // separates the two, over the OKLab of the primaries, white, greys and mixed colours.
    for c in [
        [1.0, 0.0, 0.0],
        [0.2, 0.7, 0.1],
        [0.05, 0.4, 0.9],
        [1.0, 1.0, 1.0],
        [0.18, 0.18, 0.18],
    ] {
        let lab = ottosson_forward(c);
        let crate_ = engine::contract::stain::ramp::oklab_to_linear(lab);
        assert!(
            near(crate_, ottosson_inverse(lab)),
            "the crate's OKLab → linear of {lab:?} is {crate_:?}, not Ottosson's"
        );
        // And the round trip lands back on the colour to the precision Ottosson's 10-digit coefficients carry.
        for i in 0..3 {
            assert!(
                (crate_[i] - c[i]).abs() <= 1e-6,
                "{lab:?} does not invert to {c:?}"
            );
        }
    }
}

negative_control!(
    qa_ottosson_transcription,
    "a digit changed in a primary's reference value fails",
    expected = "OKLab of",
    {
        let f = ottosson_forward([1.0, 0.0, 0.0]);
        let lab = [0.627_955, 0.224_863, 0.125_946];
        for i in 0..3 {
            assert!(
                (f[i] - lab[i]).abs() <= 5e-6,
                "OKLab of red is {f:?}, not {lab:?}"
            );
        }
    }
);

/// How a ramp is interpolated between neighbouring stops; `LinearRgb`, the other space, is the controls' alone.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(feature = "controls"), allow(dead_code))] // Built only by the negative control, under `controls`.
enum Space {
    Oklab,
    LinearRgb,
}

/// §9.4's colour between stops `a` and `b` at fraction `u`, in linear sRGB.
fn expect(space: Space, a: [f64; 3], b: [f64; 3], u: f64) -> [f64; 3] {
    match space {
        Space::Oklab => ottosson_inverse(mix(a, b, u)),
        Space::LinearRgb => mix(ottosson_inverse(a), ottosson_inverse(b), u),
    }
}

/// The ramp's colour at each `t` against §9.4, worked by hand: (t, a, b, u).
fn check_ramp(r: &Ramp, cases: &[(f64, [f64; 3], [f64; 3], f64)], space: Space) {
    for &(t, a, b, u) in cases {
        let got = r
            .linear_at(t)
            .unwrap_or_else(|| panic!("no colour at t = {t}"));
        let want = expect(space, a, b, u);
        assert!(
            near(got, want),
            "at t = {t} the ramp is {got:?}, not §9.4's {want:?}"
        );
    }
}

/// The two-stop ramp, stops at 0.2 (red) and 0.8 (blue), not cyclic.
fn clamped_cases() -> Vec<(f64, [f64; 3], [f64; 3], f64)> {
    vec![
        (0.2, RED, BLUE, 0.0),
        (0.5, RED, BLUE, 0.5),
        (0.35, RED, BLUE, 0.25),
        (0.8, RED, BLUE, 1.0),
        // Below the first stop, the first's colour; at or past the last, the last's; `t` clamped to [0, 1].
        (0.0, RED, RED, 0.0),
        (0.1, RED, RED, 0.0),
        (-3.0, RED, RED, 0.0),
        (0.9, BLUE, BLUE, 0.0),
        (1.0, BLUE, BLUE, 0.0),
        (7.5, BLUE, BLUE, 0.0),
    ]
}

/// The same stops, cyclic: `t` mod 1, wrapping from blue at 0.8 to red at 1.2 (length 0.4).
fn cyclic_cases() -> Vec<(f64, [f64; 3], [f64; 3], f64)> {
    vec![
        (0.2, RED, BLUE, 0.0),
        (0.5, RED, BLUE, 0.5),
        (1.5, RED, BLUE, 0.5),
        (-0.5, RED, BLUE, 0.5),
        (0.8, BLUE, RED, 0.0),
        (0.9, BLUE, RED, 0.25),
        (1.0, BLUE, RED, 0.5),
        (0.0, BLUE, RED, 0.5),
        (0.1, BLUE, RED, 0.75),
        (2.1, BLUE, RED, 0.75),
        (-0.9, BLUE, RED, 0.75),
    ]
}

#[test]
fn qa_ramp_evaluates_as_9_4() {
    check_ramp(&two_stop(false), &clamped_cases(), Space::Oklab);
    check_ramp(&two_stop(true), &cyclic_cases(), Space::Oklab);
    // OKLab is the ramp's own output too: oklab_at is the linear mix of the stops.
    let r = two_stop(false);
    let lab = r.oklab_at(0.5).expect("a colour");
    assert!(near(lab, mix(RED, BLUE, 0.5)));
}

negative_control!(
    qa_ramp_evaluates_as_9_4,
    "a ramp interpolated in linear rgb, the other space, fails §9.4",
    expected = "not §9.4's",
    check_ramp(&two_stop(false), &clamped_cases(), Space::LinearRgb)
);

/// A hard edge: two stops at one position, `t` there taking the later stop's colour.
fn check_hard_edge(later: [f64; 3]) {
    let r = Ramp::new(
        vec![
            stop(0.0, RED),
            stop(0.5, GREEN),
            stop(0.5, BLUE),
            stop(1.0, RED),
        ],
        false,
    )
    .expect("stops may share a position");
    let at = r.linear_at(0.5).unwrap();
    assert!(
        near(at, ottosson_inverse(later)),
        "at the shared position the ramp is {at:?}"
    );
    // Either side of the edge interpolates toward its own stop.
    check_ramp(
        &r,
        &[(0.25, RED, GREEN, 0.5), (0.75, BLUE, RED, 0.5)],
        Space::Oklab,
    );
    // Just below the edge is near green, not blue.
    let below = r.oklab_at(0.5 - 1e-9).unwrap();
    assert!(
        (below[1] - GREEN[1]).abs() < 1e-6,
        "just below the edge the ramp is not green"
    );
    // Cyclic, the same.
    let rc = Ramp::new(r.stops().to_vec(), true).unwrap();
    assert!(near(rc.linear_at(0.5).unwrap(), ottosson_inverse(later)));
}

#[test]
fn qa_ramp_hard_edge_takes_later_stop() {
    check_hard_edge(BLUE);
}

negative_control!(
    qa_ramp_hard_edge_takes_later_stop,
    "the earlier stop's colour at the edge fails",
    expected = "at the shared position",
    check_hard_edge(GREEN)
);

/// A cyclic ramp whose first stop is at 0 and last at 1, one colour: seamless, its colour at 0 and at 1 agree.
fn check_seamless(last: [f64; 3]) {
    let r = Ramp::new(vec![stop(0.0, RED), stop(0.5, BLUE), stop(1.0, last)], true).unwrap();
    // `t` mod 1 makes the colour at 1 the colour at 0 by construction; seamless is the colour just below 1 meeting
    // it. A step of 1e-9 moves a ramp whose stops differ by order 1 over a span of 0.5 by order 1e-9, so 1e-7 bounds
    // the continuous case and a seam of order 0.1 fails it.
    let (a, b) = (r.linear_at(0.0).unwrap(), r.linear_at(1.0 - 1e-9).unwrap());
    assert!(
        (0..3).all(|i| (a[i] - b[i]).abs() <= 1e-7),
        "the cyclic ramp's colour at 0 {a:?} and just below 1 {b:?} differ"
    );
    assert!(
        near(a, r.linear_at(1.0).unwrap()),
        "t = 1 is not t = 0 on a cyclic ramp"
    );
    // t and t + 1 are one colour across the range.
    for k in 0..20 {
        let t = f64::from(k) / 20.0 + 0.013;
        assert!(near(r.linear_at(t).unwrap(), r.linear_at(t + 1.0).unwrap()));
        assert!(near(r.linear_at(t).unwrap(), r.linear_at(t - 2.0).unwrap()));
    }
}

#[test]
fn qa_ramp_cyclic_seamless() {
    check_seamless(RED);
}

negative_control!(
    qa_ramp_cyclic_seamless,
    "a last stop of another colour is not seamless",
    expected = "differ",
    check_seamless(GREEN)
);

/// A non-finite `t` gives no colour of the ramp (the invalid-pixel treatment).
fn check_invalid(ts: &[f64]) {
    for cyclic in [false, true] {
        let r = two_stop(cyclic);
        for &t in ts {
            assert!(
                r.linear_at(t).is_none(),
                "t = {t} took a colour of the ramp"
            );
            assert!(r.oklab_at(t).is_none(), "t = {t} took a colour of the ramp");
        }
    }
}

#[test]
fn qa_ramp_invalid_t() {
    check_invalid(&[f64::NAN, f64::INFINITY, f64::NEG_INFINITY]);
}

negative_control!(
    qa_ramp_invalid_t,
    "a finite t takes a colour",
    expected = "took a colour of the ramp",
    check_invalid(&[0.5])
);

/// §9.4's stops: two or more; positions in [0, 1], never decreasing; L in [0, 1]; every number finite.
fn bad_ramps() -> Vec<(&'static str, Vec<Stop>)> {
    vec![
        ("one stop", vec![stop(0.5, RED)]),
        ("no stops", vec![]),
        ("a position below 0", vec![stop(-0.1, RED), stop(1.0, BLUE)]),
        ("a position above 1", vec![stop(0.0, RED), stop(1.1, BLUE)]),
        ("a NaN position", vec![stop(f64::NAN, RED), stop(1.0, BLUE)]),
        (
            "decreasing positions",
            vec![stop(0.6, RED), stop(0.4, BLUE)],
        ),
        (
            "L above 1",
            vec![stop(0.0, [1.2, 0.0, 0.0]), stop(1.0, BLUE)],
        ),
        (
            "L below 0",
            vec![stop(0.0, [-0.1, 0.0, 0.0]), stop(1.0, BLUE)],
        ),
        (
            "a NaN a",
            vec![stop(0.0, [0.5, f64::NAN, 0.0]), stop(1.0, BLUE)],
        ),
        (
            "an infinite b",
            vec![stop(0.0, [0.5, 0.0, f64::INFINITY]), stop(1.0, BLUE)],
        ),
    ]
}

fn check_bad_ramps(cases: Vec<(&'static str, Vec<Stop>)>) {
    for (what, stops) in cases {
        for cyclic in [false, true] {
            assert!(
                Ramp::new(stops.clone(), cyclic).is_err(),
                "a ramp with {what} was accepted"
            );
        }
    }
}

#[test]
fn qa_ramp_stops_checked() {
    check_bad_ramps(bad_ramps());
    // The edges are in: positions exactly 0 and 1, L exactly 0 and 1, equal positions.
    Ramp::new(
        vec![stop(0.0, [0.0, 0.0, 0.0]), stop(1.0, [1.0, 0.0, 0.0])],
        false,
    )
    .expect("the edges are stops");
    Ramp::new(vec![stop(0.3, RED), stop(0.3, BLUE)], true).expect("two stops at one position");
}

negative_control!(
    qa_ramp_stops_checked,
    "a valid ramp is accepted, so listing it as bad fails",
    expected = "was accepted",
    check_bad_ramps(vec![(
        "nothing wrong",
        vec![stop(0.0, RED), stop(1.0, BLUE)]
    )])
);

/// The ramp is a colour node's param: it round-trips through `RenderState` and is refused on any other node and on
/// reading an invalid one.
fn check_ramp_round_trip(cyclic_in_text: bool) {
    let g = full();
    let st = state(g);
    let mut text = serde_json::to_string(&st).unwrap();
    if cyclic_in_text {
        text = text.replacen("\"cyclic\":true", "\"cyclic\":false", 1);
    }
    let back: RenderState = serde_json::from_str(&text).unwrap();
    let ramp = |s: &RenderState| {
        s.stain_graph
            .nodes()
            .find_map(|n| n.ramp.clone())
            .expect("a ramp")
    };
    assert_eq!(ramp(&back), ramp(&st), "the ramp did not round-trip");
    assert!(ramp(&back).cyclic());
    // The round-tripped ramp evaluates as before.
    for k in 0..=10 {
        let t = f64::from(k) / 10.0;
        assert_eq!(ramp(&back).linear_at(t), ramp(&st).linear_at(t));
    }
}

#[test]
fn qa_ramp_renderstate_roundtrip() {
    check_ramp_round_trip(false);
    // An invalid ramp in the text is refused on reading: positions decreasing.
    let st = state(full());
    let mut v = serde_json::to_value(&st).unwrap();
    let nodes = v["stain_graph"]["nodes"].as_array_mut().unwrap();
    let node = nodes
        .iter_mut()
        .find(|n| n.get("ramp").is_some())
        .expect("the ramp node");
    node["ramp"]["stops"][0]["position"] = json!(0.9);
    assert!(
        serde_json::from_value::<RenderState>(v).is_err(),
        "a ramp with decreasing stops was read"
    );
    // A ramp on a non-colour node is refused, as an edit.
    let g = full();
    let b = g
        .nodes()
        .find(|n| n.kind == NodeKind::Brightness)
        .unwrap()
        .id;
    assert!(
        apply(
            &g,
            &StainEdit::Ramp {
                node: b,
                ramp: Some(two_stop(false))
            }
        )
        .is_err(),
        "a ramp on a brightness node was accepted"
    );
}

negative_control!(
    qa_ramp_renderstate_roundtrip,
    "a cyclic flag flipped in the text fails the round trip",
    expected = "did not round-trip",
    check_ramp_round_trip(true)
);
