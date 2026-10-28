//! QA tests for TASK-M6-29 on the engine side, written from the contract, not from the implementation:
//!
//! - gui_state_contract §2, "The stain graph": `RenderState`'s stain graph is §5's graph object, its nodes each
//!   `{ id, kind, occupant, params }` and its wires each `{ from, to, port }`, `port` a u32 (R-329); "It serialises
//!   with `RenderState` as `{ nodes, wires }`, in id order; the default is the backbone alone, the combiner wired to
//!   `OUT`". "Its `SetField` path is `RenderField::StainGraph`, the whole graph in one edit, undoable (R-69)", "logged
//!   as \"SetField StainGraph [before] → [after]\", each graph as its nodes in id order, `id kind occupant` (a custom
//!   occupant written `custom`) with its params, if any, as `{name=value, …}`, then its wires, each `from→to.port`".
//!   "A node's place on the editor's canvas, the canvas's pan and zoom, and which nodes are selected are the editor's
//!   own, not the graph's": the serialised graph carries none of them.
//! - gui_state_contract §5: node ids, wires and per-node params are the stain's data, and they serialise with
//!   `RenderState`.
//! - REQ-GUI-166 (re-run): the one conformance suite has a case for the new path (R-390, "Contract fields"), and a
//!   double that drops the stain graph's edits fails it, naming that case.
//!
//! Each check runs on the real engine's `StateStore`; its control (R-176) runs the same check on an input that differs
//! in the one respect the contract turns on.
// The file name `qa_TASK-M6-29` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use engine::contract::conformance;
use engine::contract::interface::EngineInterface;
use engine::contract::log::{Severity, Source};
use engine::contract::render_state::{
    Hover, NodeId, NodeKind, Occupant, Overlays, Palette, Playhead, RenderState, StainGraph, Wire,
};
use engine::contract::set_field::{Edit, RenderField, SetField};
use engine::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, KernelVariant, Latent, Links, Lock, Plane, Quality,
    SimConfig, Slice,
};
use engine::contract::snapshot::Snapshot;
use engine::contract::store::StateStore;
use serde_json::Value;
use validation::negative_control;

const ZERO: Latent = [0.0; 8];

fn sim() -> SimConfig {
    SimConfig {
        chart: Chart {
            window: engine::contract::sim_config::Window::SQUARE,
        },
        plane: Plane {
            z0: ZERO,
            q1: ZERO,
            q2: ZERO,
        },
        slice: Slice {},
        lock: Lock {
            locked: false,
            z_locked: ZERO,
        },
        links: Links::defaults(),
        integrator: Integrator {},
        kernel_variant: KernelVariant::Physics,
        horizon: Horizon { t: 0.0 },
        collision: Collision {},
        quality: Quality {},
    }
}

fn render(graph: StainGraph) -> RenderState {
    RenderState {
        stain_graph: graph,
        overlays: Overlays::default(),
        hover: Hover::default(),
        palette: Palette {},
        playhead: Playhead { t: 0.0 },
    }
}

fn store() -> StateStore {
    StateStore::new(sim(), render(StainGraph::new()))
}

/// A custom brightness occupant declaring one scalar uniform, `fade`, in `[0, 1]` (the lowering contract's
/// `@uniform` line).
const FADE_WGSL: &str = "// @input t [0, 50]
// @uniform fade: f32 = 0.25 [0, 1]
fn brightness(ctx: Ctx) -> f32 {
    return mix(1.0, fade, clamp(ctx.inputs[0].x / 50.0, 0.0, 1.0));
}
";

/// A graph past the backbone: a source feeding a built-in colour into the combiner's colour port, and a second
/// source feeding a custom brightness, its `fade` set, into the brightness port.
fn stained() -> (StainGraph, [NodeId; 4]) {
    let mut g = StainGraph::new();
    let combiner = NodeId(0);
    let s1 = g
        .add(NodeKind::Source, Occupant::Field("ftle".into()))
        .unwrap();
    let colour = g
        .add(NodeKind::Colour, Occupant::Builtin("outcome_state".into()))
        .unwrap();
    let s2 = g
        .add(NodeKind::Source, Occupant::Field("t_end_step".into()))
        .unwrap();
    let fade = g
        .add(NodeKind::Brightness, Occupant::Custom(FADE_WGSL.into()))
        .unwrap();
    g.connect(s1, colour, 0).unwrap();
    g.connect(colour, combiner, 0).unwrap();
    g.connect(s2, fade, 0).unwrap();
    g.connect(fade, combiner, 1).unwrap();
    g.set_param(fade, "fade", vec![0.5]).unwrap();
    (g, [s1, colour, s2, fade])
}

// --- §2: the graph's serialised shape -------------------------------------------------------------------------------

/// The graph's JSON, as it serialises inside `RenderState`.
fn graph_json(r: &RenderState) -> Value {
    let v = serde_json::to_value(r).expect("RenderState serialises");
    v.get("stain_graph")
        .cloned()
        .expect("RenderState carries `stain_graph`")
}

fn keys(v: &Value) -> Vec<String> {
    let mut k: Vec<String> = v.as_object().expect("an object").keys().cloned().collect();
    k.sort();
    k
}

/// §2's shape: `{ nodes, wires }`; each node exactly `{ id, kind, occupant, params }` (no place on the canvas, no
/// selection), in id order; each wire exactly `{ from, to, port }`, `port` a non-negative integer within u32.
fn check_shape(g: &Value) {
    assert_eq!(
        keys(g),
        ["nodes", "wires"],
        "the graph is not {{ nodes, wires }}: {g}"
    );
    let nodes = g["nodes"].as_array().expect("nodes is a list");
    let ids: Vec<u64> = nodes
        .iter()
        .map(|n| {
            assert_eq!(
                keys(n),
                ["id", "kind", "occupant", "params"],
                "a node is not {{ id, kind, occupant, params }}: {n}"
            );
            n["id"].as_u64().expect("a node id is a number")
        })
        .collect();
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    assert_eq!(ids, sorted, "the nodes are not in id order: {ids:?}");
    for w in g["wires"].as_array().expect("wires is a list") {
        assert_eq!(
            keys(w),
            ["from", "port", "to"],
            "a wire is not {{ from, to, port }}: {w}"
        );
        let port = w["port"]
            .as_u64()
            .expect("a wire's port is a non-negative integer");
        assert!(
            port <= u64::from(u32::MAX),
            "a wire's port is past u32: {w}"
        );
    }
}

#[test]
fn qa_stain_graph_serialises_as_nodes_and_wires_in_id_order() {
    let (g, _) = stained();
    let v = graph_json(&render(g.clone()));
    check_shape(&v);
    let kinds: Vec<&str> = v["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["kind"].as_str().expect("a kind is a name"))
        .collect();
    assert_eq!(
        kinds,
        [
            "combiner",
            "out",
            "source",
            "colour",
            "source",
            "brightness"
        ],
        "the nodes' kinds by id"
    );
    // The per-node params serialise with the graph (§5): the fade's value, one number per component.
    assert_eq!(v["nodes"][5]["params"]["fade"], serde_json::json!([0.5]));
    // It reads back as the same graph, with RenderState.
    let text = serde_json::to_string(&render(g.clone())).unwrap();
    let back: RenderState = serde_json::from_str(&text).expect("RenderState reads back");
    assert_eq!(back.stain_graph, g, "the graph did not round-trip");
    // R-329: the in-port's position is a u32 in the type itself.
    let w: Wire = g.wires()[0];
    let _: u32 = w.port;
}

negative_control!(
    qa_stain_graph_serialises_as_nodes_and_wires_in_id_order,
    "a node carrying its canvas place must fail the graph's shape",
    expected = "a node is not",
    {
        let mut v = graph_json(&render(StainGraph::new()));
        v["nodes"][0]["pos"] = serde_json::json!([24.0, 64.0]);
        check_shape(&v)
    }
);

/// The default graph is the backbone alone: the combiner wired into `OUT`, nothing else.
fn check_backbone(v: &Value) {
    check_shape(v);
    let nodes = v["nodes"].as_array().unwrap();
    let kinds: Vec<&str> = nodes.iter().map(|n| n["kind"].as_str().unwrap()).collect();
    assert_eq!(
        kinds,
        ["combiner", "out"],
        "the default graph is not the backbone alone"
    );
    let (combiner, out) = (&nodes[0]["id"], &nodes[1]["id"]);
    let wires = v["wires"].as_array().unwrap();
    assert_eq!(wires.len(), 1, "the backbone has one wire: {v}");
    assert_eq!(
        (&wires[0]["from"], &wires[0]["to"]),
        (combiner, out),
        "the backbone's wire is not combiner → OUT: {v}"
    );
}

#[test]
fn qa_stain_graph_default_is_the_backbone_alone() {
    check_backbone(&graph_json(&render(StainGraph::new())));
    let mut s = store();
    check_backbone(&graph_json(&s.snapshot().render));
}

negative_control!(
    qa_stain_graph_default_is_the_backbone_alone,
    "a graph past the backbone must not pass as the default",
    expected = "the default graph is not the backbone alone",
    check_backbone(&graph_json(&render(stained().0)))
);

// --- §2: the SetField path, undoable, and its log line ----------------------------------------------------------------

fn graph_edit(g: StainGraph, no_history: bool) -> SetField {
    SetField {
        edit: Edit::Render(RenderField::StainGraph(g)),
        no_history,
    }
}

/// A graph as §2 has the log write it, from its words: in brackets, its nodes in id order, `id kind occupant`, a
/// custom occupant `custom`, its params, if any, `{name=value, …}`; then `; ` and its wires, each `from→to.port`. The
/// graphs here carry only one-number params, whose writing §2 spells.
fn logged(g: &StainGraph) -> String {
    let nodes: Vec<String> = g
        .nodes()
        .map(|n| {
            let kind = serde_json::to_value(n.kind).unwrap();
            let occupant = match &n.occupant {
                Occupant::None => "none".to_owned(),
                Occupant::Field(f) => f.clone(),
                Occupant::Builtin(b) => b.clone(),
                Occupant::Custom(_) => "custom".to_owned(),
            };
            let mut s = format!("{} {} {occupant}", n.id.0, kind.as_str().unwrap());
            if !n.params.is_empty() {
                let ps: Vec<String> = n
                    .params
                    .iter()
                    .map(|(k, v)| format!("{k}={}", v[0]))
                    .collect();
                s += &format!(" {{{}}}", ps.join(", "));
            }
            s
        })
        .collect();
    let wires: Vec<String> = g
        .wires()
        .iter()
        .map(|w| format!("{}→{}.{}", w.from.0, w.to.0, w.port))
        .collect();
    format!("[{}; {}]", nodes.join(", "), wires.join(", "))
}

fn graph_of(s: &Snapshot) -> &StainGraph {
    &s.render.stain_graph
}

/// The edit shows in the next snapshot, is one undo step, undo restores the graph before and redo the edit; the
/// entry logged is one `info` from `contract` reading §2's line; nothing else of `RenderState` moves.
fn check_path(engine: &mut dyn EngineInterface) {
    let first = engine.snapshot();
    let before = graph_of(&first).clone();
    let depth = first.history.undo_depth;
    let (after, _) = stained();
    assert_ne!(before, after, "the test's edit must change the graph");
    engine.set_field(graph_edit(after.clone(), false));
    let s = engine.snapshot();
    assert_eq!(graph_of(&s), &after, "the graph after its SetField");
    assert_eq!(
        s.render.overlays, first.render.overlays,
        "the overlays moved"
    );
    assert_eq!(
        s.render.playhead, first.render.playhead,
        "the playhead moved"
    );
    assert_eq!(
        s.history.undo_depth,
        depth + 1,
        "the edit is not one undo step"
    );
    let line = format!(
        "SetField StainGraph {} → {}",
        logged(&before),
        logged(&after)
    );
    let entries: Vec<_> = s
        .log
        .iter()
        .filter(|e| e.message.starts_with("SetField StainGraph"))
        .collect();
    assert_eq!(entries.len(), 1, "one entry per edit: {:?}", s.log);
    assert_eq!(entries[0].message, line, "the log line is not §2's");
    assert_eq!(
        (entries[0].severity, entries[0].source),
        (Severity::Info, Source::Contract),
        "the entry is not an info from contract"
    );
    engine.undo();
    assert_eq!(
        graph_of(&engine.snapshot()),
        &before,
        "undo did not restore the graph"
    );
    engine.redo();
    assert_eq!(
        graph_of(&engine.snapshot()),
        &after,
        "redo did not reapply the graph"
    );
}

#[test]
fn qa_stain_graph_set_field_shows_undoes_and_logs() {
    check_path(&mut store());
    // After an earlier graph edit too.
    let mut s = store();
    s.set_field(graph_edit(StainGraph::new(), false));
    let _ = s.snapshot();
    check_path(&mut s);
}

/// The store, but each stain graph edit is dropped.
struct DropsGraph(StateStore);

impl EngineInterface for DropsGraph {
    fn set_field(&mut self, e: SetField) {
        if !matches!(e.edit, Edit::Render(RenderField::StainGraph(_))) {
            self.0.set_field(e);
        }
    }
    fn snapshot(&mut self) -> Snapshot {
        self.0.snapshot()
    }
    fn undo(&mut self) {
        self.0.undo()
    }
    fn redo(&mut self) {
        self.0.redo()
    }
}

negative_control!(
    qa_stain_graph_set_field_shows_undoes_and_logs,
    "a store dropping the graph's edits must fail the path's check",
    expected = "the graph after its SetField",
    check_path(&mut DropsGraph(store()))
);

/// The log writes a custom occupant `custom`, never its WGSL, and a param as `{name=value}`.
fn check_custom_written(message: &str) {
    assert!(
        message.contains("5 brightness custom {fade=0.5}"),
        "the custom brightness is not written `5 brightness custom {{fade=0.5}}`: {message}"
    );
    assert!(
        !message.contains("fn brightness"),
        "the log carries the custom occupant's WGSL: {message}"
    );
    assert!(
        message.contains("5→0.1") && message.contains("3→0.0"),
        "the wires into the combiner: {message}"
    );
}

#[test]
fn qa_stain_graph_log_writes_custom_and_params() {
    let mut s = store();
    s.set_field(graph_edit(stained().0, false));
    let snap = s.snapshot();
    let entry = snap
        .log
        .iter()
        .find(|e| e.message.starts_with("SetField StainGraph"))
        .expect("the edit is logged");
    check_custom_written(&entry.message);
}

negative_control!(
    qa_stain_graph_log_writes_custom_and_params,
    "a line carrying the WGSL must fail",
    expected = "the custom brightness is not written",
    check_custom_written(&format!("SetField StainGraph [5 brightness {FADE_WGSL}]"))
);

// --- REQ-GUI-166: the suite's case for the path ---------------------------------------------------------------------

fn check_suite_catches<E: EngineInterface>(make: impl FnMut() -> E) {
    let err = match conformance::run(make) {
        Ok(ran) => panic!("the suite passed a double that drops the stain graph's edits: {ran:?}"),
        Err(e) => e,
    };
    let named: Vec<&str> = conformance::CASES
        .iter()
        .map(|c| c.name)
        .filter(|n| err.contains(n))
        .collect();
    assert!(!named.is_empty(), "the failure names no case: {err}");
    assert!(
        named.iter().all(|n| n.contains("stain")),
        "dropping only the graph's edits fails cases not about the graph: {named:?}"
    );
}

#[test]
fn qa_conformance_has_a_stain_graph_case() {
    conformance::run(store).unwrap_or_else(|e| panic!("the real engine fails the suite: {e}"));
    check_suite_catches(|| DropsGraph(store()));
}

negative_control!(
    qa_conformance_has_a_stain_graph_case,
    "the conforming store, run through the same check, must be reported as passing",
    expected = "the suite passed a double that drops",
    check_suite_catches(store)
);
