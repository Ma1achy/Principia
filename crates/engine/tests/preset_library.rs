//! The preset library as whole graphs (`render/presets/`): selecting one replaces the graph, the catalogue's default
//! composition is the file's graph, a debug preset loads locked and forks on its first edit, and a bivariate preset
//! loads with both combiner inputs occupied.
//!
//! Pass criteria, written before the runs:
//! - `preset_load`: each preset's graph equals its file's JSON read as a saved stain; the library's entry for the
//!   outcome palette holds that same graph; no node of a shipped preset reads back as pinned (by value);
//! - `preset_locked_fork`: an accepted edit to a loaded debug preset gives `Applied::Forked`, an unlocked, renamed copy
//!   that differs from the preset, and the library's preset is unchanged; a production preset gives `InPlace`; a refused
//!   edit forks nothing;
//! - `preset_bivariate`: n̂ × FTLE and n̂ × t_end have a wire into the combiner's colour port and into its brightness
//!   port, the combiner is `replace_l`, and the same graph with the brightness unwired assembles to other code.

use engine::contract::stain::assembled;
use engine::contract::stain::edit::{Refusal, StainEdit};
use engine::contract::stain::graph::{NodeId, NodeKind, Occupant, StainGraph};
use engine::contract::stain::preset::{self, Applied};
use render::presets::file::{self, Section};
use validation::negative_control;

const COMBINER: NodeId = NodeId(0);

fn check_load(extra_node: bool) {
    for f in file::files().expect("the library reads") {
        let mut want: StainGraph =
            serde_json::from_value(f.graph.clone()).expect("the file's graph");
        let p = preset::read(f.clone()).expect("the preset");
        if extra_node {
            want.add(NodeKind::Post, Occupant::None).expect("add");
        }
        assert_eq!(
            p.graph, want,
            "{}: the preset's graph is not its file's",
            f.id
        );
        assert_eq!(p.name, f.name, "{}: its name is not its file's", f.id);
    }
}

#[test]
fn preset_load() {
    check_load(false);
}

negative_control!(
    preset_load,
    "a graph that differs from its file's is refused",
    expected = "the preset's graph is not its file's",
    check_load(true)
);

/// The library's entry for the outcome palette holds the preset's graph: one data source.
fn catalogue_outcome_is_the_file(graph: &StainGraph) {
    let entry = assembled::library()
        .expect("the library")
        .into_iter()
        .find(|e| e.name == render::colour::outcome::name())
        .expect("the outcome palette's entry");
    assert_eq!(
        entry.graph.as_ref(),
        Some(graph),
        "the catalogue's default composition is not the preset's payload"
    );
}

#[test]
fn preset_load_catalogue_default_is_the_payload() {
    catalogue_outcome_is_the_file(&preset::graph(Section::Production, "outcome").expect("outcome"));
}

negative_control!(
    preset_load_catalogue_default_is_the_payload,
    "a payload that differs from the catalogue's default is refused",
    expected = "the catalogue's default composition is not the preset's payload",
    {
        let mut g = preset::graph(Section::Production, "outcome").expect("outcome");
        g.set_occupant(COMBINER, Occupant::Builtin("multiply".into()))
            .expect("occupant");
        catalogue_outcome_is_the_file(&g);
    }
);

fn no_pinned(g: &StainGraph, id: &str) {
    for n in g.nodes() {
        assert!(
            !matches!(n.occupant, Occupant::Pinned(_)),
            "{id}: node {} reads back pinned: its file's text differs from the shipped source it names",
            n.id.0
        );
    }
}

#[test]
fn preset_load_follows_shipped_sources() {
    for p in preset::library().expect("the library") {
        no_pinned(&p.graph, p.id);
    }
}

negative_control!(
    preset_load_follows_shipped_sources,
    "a node saved by value with one byte of a shipped source changed reads back pinned",
    expected = "reads back pinned",
    {
        let mut g = preset::graph(Section::Production, "outcome").expect("outcome");
        let text =
            assembled::node_source(NodeKind::Colour, &Occupant::Builtin("outcome_state".into()))
                .expect("source")
                .expect("a source");
        let form = engine::contract::stain::saved::write(
            NodeKind::Colour,
            &Occupant::Builtin("outcome_state".into()),
        )
        .expect("form");
        let mut json = serde_json::to_value(&form).expect("json");
        json["shader"]["source"] = serde_json::Value::String(format!("{text} "));
        json["shader"]["hash"] =
            serde_json::Value::String(engine::contract::stain::saved::hash(&format!("{text} ")));
        let edited = serde_json::from_value(json).expect("form");
        let occupant =
            engine::contract::stain::saved::read(NodeKind::Colour, edited).expect("read");
        g.set_occupant(NodeId(3), occupant).expect("occupant");
        no_pinned(&g, "outcome");
    }
);

fn a_real_edit(g: &StainGraph) -> StainEdit {
    // Any node but the fixed ones: swap the combiner's brightness-free occupant for another shipped one.
    let _ = g;
    StainEdit::Occupant {
        node: COMBINER,
        occupant: Occupant::Builtin("multiply".into()),
    }
}

#[test]
fn preset_locked_fork() {
    let pristine = preset::preset(Section::Debug, "uv_quad").expect("uv_quad");
    assert!(pristine.locked(), "a debug preset loads locked");
    let mut loaded = preset::select(&pristine);
    assert!(loaded.locked);

    // A refused edit forks nothing.
    let same = StainEdit::Occupant {
        node: COMBINER,
        occupant: Occupant::Builtin("pass_through".into()),
    };
    assert_eq!(loaded.apply(&same), Err(Refusal::NoChange));
    assert!(
        loaded.locked && loaded.graph == pristine.graph,
        "a refused edit changed or forked the preset"
    );

    // The first accepted edit forks.
    let applied = loaded.apply(&a_real_edit(&loaded.graph)).expect("accepted");
    assert_eq!(applied, Applied::Forked);
    assert!(!loaded.locked, "the copy is editable");
    assert_ne!(
        loaded.name, pristine.name,
        "the copy is named as a custom one"
    );
    assert_ne!(loaded.graph, pristine.graph, "the edit landed on the copy");
    // The library's preset, read again, is as it was; the copy is editable in place from here.
    assert_eq!(
        preset::preset(Section::Debug, "uv_quad")
            .expect("again")
            .graph,
        pristine.graph,
        "the library's preset moved"
    );
    let back = StainEdit::Occupant {
        node: COMBINER,
        occupant: Occupant::Builtin("pass_through".into()),
    };
    assert_eq!(loaded.apply(&back), Ok(Applied::InPlace));

    // A production preset loads editable and is edited in place.
    let production = preset::preset(Section::Production, "outcome").expect("outcome");
    assert!(!production.locked());
    let mut p = preset::select(&production);
    assert_eq!(p.apply(&a_real_edit(&p.graph)), Ok(Applied::InPlace));
    assert_eq!(p.name, production.name);
}

negative_control!(
    preset_locked_fork,
    "an edit to a locked graph that is applied in place instead of forking fails",
    expected = "a locked preset was edited in place",
    {
        let pristine = preset::preset(Section::Debug, "uv_quad").expect("uv_quad");
        let mut loaded = preset::select(&pristine);
        assert!(loaded.locked);
        // What a non-forking editor reports: the edit lands in place on the locked graph.
        assert_eq!(
            loaded.apply(&a_real_edit(&loaded.graph)),
            Ok(Applied::InPlace),
            "a locked preset was edited in place"
        );
    }
);

fn both_combiner_inputs(g: &StainGraph) -> bool {
    let w = g.wires();
    [0, 1]
        .iter()
        .all(|&p| w.iter().any(|w| w.to == COMBINER && w.port == p))
}

#[test]
fn preset_bivariate() {
    for id in ["outcome_ftle", "outcome_t_end"] {
        let g = preset::graph(Section::Production, id).expect(id);
        assert!(
            both_combiner_inputs(&g),
            "{id}: the combiner's colour and brightness are not both wired"
        );
        assert_eq!(
            g.node(COMBINER).map(|n| n.occupant.clone()),
            Some(Occupant::Builtin("replace_l".into())),
            "{id}: the combiner does not give L to the brightness"
        );
        assert!(
            g.nodes().any(|n| n.kind == NodeKind::Brightness),
            "{id}: no brightness node"
        );
        let mut unwired = g.clone();
        unwired.disconnect(COMBINER, 1).expect("disconnect");
        assert_ne!(
            g.canonical().expect("canonical").render_text(),
            unwired.canonical().expect("canonical").render_text(),
            "{id}: the bound brightness changes nothing in the assembled code"
        );
    }
}

negative_control!(
    preset_bivariate,
    "a bivariate preset with its brightness unwired fails",
    expected = "the combiner's colour and brightness are not both wired",
    {
        let mut g = preset::graph(Section::Production, "outcome_ftle").expect("outcome_ftle");
        g.disconnect(COMBINER, 1).expect("disconnect");
        assert!(
            both_combiner_inputs(&g),
            "outcome_ftle: the combiner's colour and brightness are not both wired"
        );
    }
);
