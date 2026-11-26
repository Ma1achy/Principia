//! The preset library as whole graphs (render_gui_spec §11).
//!
//! - `preset_graph_as_before`: each library preset's graph is, node for node and wire for wire, the graph the in-code
//!   builder gave before the library was files (the reference below is that builder), and it assembles to the same
//!   WGSL and has the same keys, so no picture, golden or digest moves. Control: a reference wired to another view
//!   fails.
//! - `preset_graph_names_shipped_sources`: every node of every preset is a shipped source or a field, never pinned
//!   text, so a shipped file's edit reaches the preset and `updates()` is empty. Control: a graph with one node
//!   pinned to a one-byte-different copy of its source offers an update.
//! - `preset_locked_fork`: a debug preset loads locked and its first accepted edit makes a custom copy, editable, the
//!   library's preset unchanged; a refused edit forks nothing; a production preset loads editable and is edited in
//!   place. Control: an edit made on the library's own copy fails.

use engine::contract::stain::assembled::{assemble, library as entries};
use engine::contract::stain::edit::StainEdit;
use engine::contract::stain::preset::{library, preset, Loaded};
use engine::stain::{NodeId, NodeKind, Occupant, StainGraph};
use render::assemble::Tier;
use validation::negative_control;

/// The graph the library's builder made before presets were files: the source into the colour, the colour into the
/// pass-through combiner.
fn reference(source: Occupant, colour: Occupant) -> StainGraph {
    let mut g = StainGraph::new();
    let s = g.add(NodeKind::Source, source).expect("source");
    let c = g.add(NodeKind::Colour, colour).expect("colour");
    g.connect(s, c, 0).expect("wire");
    g.connect(c, NodeId(0), 0).expect("wire");
    g
}

fn reference_of(id: &str) -> StainGraph {
    if id == "colour/outcome_state" {
        reference(
            Occupant::Field("state".to_owned()),
            Occupant::Builtin(render::colour::outcome::name().to_owned()),
        )
    } else {
        reference(
            Occupant::Builtin(render::registry::ZERO.to_owned()),
            Occupant::Builtin(id.to_owned()),
        )
    }
}

/// `graph` is `reference`; with `deep`, also the same WGSL and keys, which equal graphs give and which is slow to
/// assemble for all of the library.
fn check_as_before(p_id: &str, graph: &StainGraph, reference: &StainGraph, deep: bool) {
    assert_eq!(graph, reference, "`{p_id}`: the graph is not the builder's");
    if !deep {
        return;
    }
    let (a, b) = (
        assemble(graph, Tier::FULL).expect("assembles"),
        assemble(reference, Tier::FULL).expect("assembles"),
    );
    assert_eq!(a.pipeline, b.pipeline, "`{p_id}`: not the same WGSL");
    assert_eq!(a.key, b.key, "`{p_id}`: not the same key");
    let (ca, cb) = (
        graph.canonical().expect("canonical"),
        reference.canonical().expect("canonical"),
    );
    assert_eq!(
        ca.render_key(),
        cb.render_key(),
        "`{p_id}`: not the same render key"
    );
}

#[test]
fn preset_graph_as_before() {
    let all = library().expect("the library");
    assert!(all.len() > 70, "the library holds {} presets", all.len());
    for p in &all {
        let deep = [
            "colour/outcome_state",
            "debug/s_depth",
            "debug/generated/state",
            "debug/word/hash",
        ]
        .contains(&p.id.as_str());
        check_as_before(&p.id, p.graph(), &reference_of(&p.id), deep);
    }
}

negative_control!(
    preset_graph_as_before,
    "a reference wired to another view",
    expected = "the graph is not the builder's",
    {
        let p = preset("debug/s_depth").expect("a preset");
        check_as_before(&p.id, p.graph(), &reference_of("debug/s_state"), false);
    }
);

/// Panics where `g` has a node the library's files would not give back: one held as pinned text.
fn check_no_pinned(id: &str, g: &StainGraph) {
    assert!(
        g.updates().is_empty(),
        "`{id}` has a node held as text of its own, not the shipped source"
    );
    for n in g.nodes() {
        assert!(
            !matches!(n.occupant, Occupant::Pinned(_)),
            "`{id}` has a node held as text of its own, not the shipped source"
        );
    }
}

#[test]
fn preset_graph_names_shipped_sources() {
    for p in library().expect("the library") {
        check_no_pinned(&p.id, p.graph());
    }
    for e in entries().expect("the entries") {
        if let Some(g) = &e.graph {
            check_no_pinned(&e.id, g);
        }
    }
}

negative_control!(
    preset_graph_names_shipped_sources,
    "one node pinned to a one-byte-different copy of its source",
    expected = "held as text of its own",
    {
        let mut g = preset("debug/s_depth").expect("a preset").graph().clone();
        let colour = g
            .nodes()
            .find(|n| n.kind == NodeKind::Colour)
            .map(|n| n.id)
            .expect("a colour");
        let text = engine::contract::stain::assembled::node_source(
            NodeKind::Colour,
            &g.node(colour).expect("node").occupant,
        )
        .expect("source")
        .expect("text");
        let (id, version) = ("debug/s_depth".to_owned(), None);
        g.set_occupant(
            colour,
            Occupant::Pinned(engine::contract::stain::saved::Pinned {
                id,
                version,
                source: format!("{text} "),
                edited: None,
            }),
        )
        .expect("pinned");
        check_no_pinned("debug/s_depth", &g);
    }
);

/// An edit that changes a preset's graph: a param on the colour node's source would need a declared uniform, so it
/// adds a node, which every graph accepts.
fn an_edit() -> StainEdit {
    StainEdit::Add {
        kind: NodeKind::Post,
        occupant: Occupant::Builtin("edge_line".to_owned()),
    }
}

fn check_fork(library_before: &StainGraph, library_after: &StainGraph, loaded: &Loaded) {
    assert_eq!(
        library_before, library_after,
        "the edit changed the library's preset"
    );
    assert!(!loaded.locked(), "the copy is still locked");
    assert!(loaded.forked_from().is_some(), "no copy was made");
    assert_ne!(
        loaded.graph(),
        library_before,
        "the edit did not reach the copy"
    );
}

#[test]
fn preset_locked_fork() {
    let p = preset("debug/s_depth").expect("a debug preset");
    let before = p.graph().clone();
    let mut loaded = p.load();
    assert!(loaded.locked(), "a debug preset loads locked");
    assert_eq!(loaded.graph(), &before);

    // A refused edit forks nothing.
    let bad = StainEdit::Delete { node: NodeId(0) };
    loaded.apply(&bad).expect_err("the combiner is fixed");
    assert!(loaded.locked() && loaded.forked_from().is_none());
    assert_eq!(loaded.graph(), &before);

    loaded.apply(&an_edit()).expect("an edit");
    check_fork(&before, p.graph(), &loaded);
    assert_eq!(loaded.forked_from(), Some("debug/s_depth"));
    assert_ne!(loaded.name(), p.name, "the copy is named as a copy");

    // The copy is the user's: more edits stay on it, with no second copy.
    loaded.apply(&an_edit()).expect("a second edit");
    assert_eq!(loaded.forked_from(), Some("debug/s_depth"));

    // A production preset loads editable and is edited in place.
    let o = preset("colour/outcome_state").expect("the outcome preset");
    assert!(!o.locked);
    let mut edited = o.load();
    assert!(!edited.locked());
    edited.apply(&an_edit()).expect("an edit");
    assert!(
        edited.forked_from().is_none(),
        "a production preset forks nothing"
    );
    assert_ne!(edited.graph(), o.graph());

    // Every debug preset of the library loads locked.
    for p in library().expect("the library") {
        assert_eq!(p.load().locked(), p.id.starts_with("debug/"), "`{}`", p.id);
    }
}

negative_control!(
    preset_locked_fork,
    "an edit made on the library's own copy",
    expected = "the edit changed the library's preset",
    {
        let p = preset("debug/s_depth").expect("a debug preset");
        let before = p.graph().clone();
        let mut loaded = p.load();
        loaded.apply(&an_edit()).expect("an edit");
        // The library's copy edited in place, as a loader without a fork would.
        let mut in_place = p.graph().clone();
        in_place
            .add(NodeKind::Post, Occupant::Builtin("edge_line".to_owned()))
            .expect("add");
        check_fork(&before, &in_place, &loaded);
    }
);
