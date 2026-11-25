//! The Stain editor on the real engine: its content is the store's own graph through the real assembler and registry,
//! an edit made with the editor's controls is one undoable `SetField` that reaches the assembled code, a param edit
//! assembles nothing, and a graph that does not compile is a problem that leaves the last code in place. Each check also
//! runs on an input it must reject (through `rejects`).

use engine::contract::render_state::{NodeId, NodeKind, Occupant, StainGraph};
use engine::contract::set_field::{Edit, RenderField, SetField};
use engine::contract::stain::assembled::{assemble, library, node_source, source_fields};
use engine::presets;

use super::mock_stain_preview::{check_one_edit, depth, graph, logged, on_stain, select};
use super::support::{click, click_scope, frame_texts, keys_on, nodes, press, rejects, FORMAT};
use crate::app::App;
use crate::headless::Headless;
use crate::real::stain::{heading, TIER};
use crate::real::start;
use crate::side::{EngineSide, RealSide};
use crate::stain::{
    library as drawer, whole, Content, CHOICE_IDS, COMBINERS, GROUP_IDS, PARAM_IDS, ROW_IDS,
};

/// The start graph's nodes: the combiner, the source of `state` and the outcome palette.
const COMBINER: NodeId = NodeId(0);
const SOURCE: NodeId = NodeId(2);
const PALETTE: NodeId = NodeId(3);

/// The app on the real side at the start state, on the Stain page, with no canvas.
fn real_stain() -> (App<RealSide>, Headless) {
    let (sim, render) = start();
    on_stain(App::new(RealSide::new(sim, render), FORMAT))
}

/// The content, once the stain asked for is assembled.
fn content(app: &mut App<RealSide>) -> Content {
    app.side().wait_for_stain();
    app.side()
        .stain_content()
        .expect("the real side's stain content")
}

/// Clicks the library's row `name`, the drawer filtered to it.
fn load(app: &mut App<RealSide>, h: &mut Headless, name: &str) {
    let c = content(app);
    let k = (0..ROW_IDS.len())
        .find(|k| drawer::row(&c, *k) == Some(name))
        .unwrap_or_else(|| panic!("no library row `{name}`"));
    app.stain.library.filter = name.to_owned();
    let _ = h.frame(app, Vec::new());
    click_scope(h, app, ROW_IDS[k]);
    let _ = h.frame(app, Vec::new());
}

/// Asserts that `c` is what the real assembler and registry make of `g`.
fn check_content(c: Option<Content>, g: &StainGraph) {
    let c = c.expect("the stain content");
    let want = assemble(g, TIER).expect("the graph assembles");
    assert_eq!(c.pipeline, want.pipeline);
    assert_eq!(c.code, want.code);
    assert!(
        c.pipeline.ends_with(&c.code),
        "the graph's own part, the pipeline's tail"
    );
    assert!(c.code.contains("fn shade(ctx: Ctx)"), "{}", c.code);
    for n in g.nodes() {
        if let Some(text) = node_source(n.kind, &n.occupant).expect("the node's source") {
            assert_eq!(c.node_code[&n.id], text);
        }
    }
    assert!(c.problems.is_empty(), "{:?}", c.problems);
}

#[test]
fn the_content_is_the_assemblers_and_the_registrys() {
    let (mut app, _) = real_stain();
    let g = graph(&app).clone();
    assert_eq!(g, presets::outcome().expect("the outcome stain"));
    check_content(app.side().stain_content(), &g);
    rejects("a side with no content", || check_content(None, &g));
    rejects("the mock's placeholder content", || {
        check_content(Some(crate::mock::stain::content(&g)), &g)
    });
    let other = presets::view("fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(0.5); }\n")
        .expect("a graph");
    let frozen = app.side().stain_content();
    rejects("the code of another graph", || {
        check_content(frozen, &other)
    });
    // The fields and the occupants offered are the real ones.
    let c = content(&mut app);
    assert_eq!(c.fields, source_fields().expect("the fields"));
    let entries = library().expect("the library");
    let of = |dir: &str| -> Vec<String> {
        let mut ids: Vec<String> = entries
            .iter()
            .filter(|e| e.dir() == dir)
            .map(|e| e.builtin.clone().expect("a built-in"))
            .collect();
        ids.sort();
        ids
    };
    let mut offered: Vec<&str> = COMBINERS.iter().map(|(id, _)| *id).collect();
    offered.sort_unstable();
    assert_eq!(offered, of("combiner"));
    assert_eq!(
        c.colours.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
        of("colour")
    );
    assert_eq!(c.loaded, Some("outcome_state"));
}

/// Asserts that the library in `c` is the registry: each entry one row, its name no other's, in its directory's group;
/// and every row, group and choice has a scope, so none is dropped.
fn check_library(c: &Content) {
    let entries = library().expect("the library");
    let rows: Vec<&str> = c.library.iter().flat_map(|g| g.rows).copied().collect();
    assert_eq!(rows.len(), entries.len());
    assert_eq!(c.modes as usize, entries.len());
    assert_eq!(c.graphs.len(), entries.len());
    let mut unique = rows.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), rows.len(), "two rows share a name");
    let mut k = 0;
    for group in c.library {
        for row in group.rows {
            let e = &entries[k];
            assert!(row.starts_with(e.name()), "{row} is not {}", e.id);
            assert_eq!(group.title, heading(e.dir()), "{}", e.id);
            assert_eq!(c.graphs[k], e.graph);
            k += 1;
        }
    }
    let shown = drawer::visible(c, &drawer::State::default());
    assert_eq!(
        shown.len(),
        rows.len() + c.library.len(),
        "a row or a group with no scope"
    );
    assert!(c.library.len() <= GROUP_IDS.len());
    assert!(c.fields.len() <= CHOICE_IDS.len());
}

#[test]
fn the_library_is_the_registry_and_none_of_it_is_dropped() {
    let (mut app, _) = real_stain();
    let c = content(&mut app);
    check_library(&c);
    let mut short = c.clone();
    short.library = &c.library[1..];
    rejects("a library short of a group", || check_library(&short));
    rejects("the mock's library", || {
        check_library(&crate::mock::stain::content(graph(&app)))
    });
}

/// The outcome palette and every debug colour view load a graph, and no other row does; each assembles at the real
/// tier, so no row draws a problem.
#[test]
fn every_graph_the_library_loads_assembles() {
    for e in library().expect("the library") {
        let view = e.id == "colour/outcome_state" || (e.debug && e.slot == NodeKind::Colour);
        assert_eq!(e.graph.is_some(), view, "{}", e.id);
        if let Some(Err(why)) = e.graph.as_ref().map(|g| assemble(g, TIER)) {
            panic!("{}: {why}", e.id);
        }
    }
    rejects("a colour that reads no field there is", || {
        let g = presets::view("fn colour(ctx: Ctx) -> vec3<f32> { return ctx.sample.nothing; }\n")
            .expect("a graph");
        assemble(&g, TIER).expect("it assembles");
    });
}

#[test]
fn a_choice_is_one_undoable_set_field_that_reaches_the_code() {
    let (mut app, mut h) = real_stain();
    select(&mut app, &mut h, COMBINER);
    let (g0, d0, rows) = (graph(&app).clone(), depth(&app), app.console().len());
    let c0 = content(&mut app);
    click(&mut h, &mut app, "Replace-L");
    let _ = h.frame(&mut app, Vec::new());
    let replaced = |g: &StainGraph| {
        g.node(COMBINER).map(|n| &n.occupant) == Some(&Occupant::Builtin("replace_l".into()))
    };
    check_one_edit((&g0, d0, rows), &app, replaced);
    let g1 = graph(&app).clone();
    let c1 = content(&mut app);
    check_content(Some(c1.clone()), &g1);
    assert!(c1.code.contains("replace_l_lab("), "{}", c1.code);
    rejects("an edit that does not reach the code", || {
        check_content(Some(c0), &g1)
    });
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(
        logged(&app, rows)
            .iter()
            .filter(|m| m.starts_with("stain recompiled"))
            .count(),
        1,
        "{:?}",
        logged(&app, rows)
    );
    // Undo and redo move the code with the graph.
    app.undo();
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(graph(&app), &g0);
    let undone = content(&mut app);
    check_content(Some(undone), &g0);
    app.redo();
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(graph(&app), &g1);
    let redone = content(&mut app);
    check_content(Some(redone), &g1);
    rejects("the code left where it was by a redo", || {
        check_content(Some(c1), &g0)
    });
}

#[test]
fn a_library_row_loads_its_whole_graph_and_a_source_takes_a_field() {
    let (mut app, mut h) = real_stain();
    let (g0, d0, rows) = (graph(&app).clone(), depth(&app), app.console().len());
    load(&mut app, &mut h, "d_min");
    let loaded = library()
        .expect("the library")
        .into_iter()
        .find(|e| e.id == "debug/generated/d_min")
        .and_then(|e| e.graph)
        .expect("d_min's graph");
    check_one_edit((&g0, d0, rows), &app, |g| *g == loaded);
    let c = content(&mut app);
    check_content(Some(c.clone()), &loaded);
    assert!(c.code.contains("ctx.sample.d_min"), "{}", c.code);
    assert_eq!(c.loaded, Some("d_min"));
    rejects("the outcome stain left in place", || {
        check_one_edit((&g0, d0, rows), &app, |g| *g == g0)
    });
    app.undo();
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(graph(&app), &g0);
    assert_eq!(content(&mut app).loaded, Some("outcome_state"));
    // A row with no graph is only selected.
    let d = depth(&app);
    load(&mut app, &mut h, "multiply");
    assert_eq!((graph(&app), depth(&app)), (&g0, d));
    assert_eq!(app.stain.library.selected, Some("multiply"));
    // A source's field, past the eight choices the mock has.
    select(&mut app, &mut h, SOURCE);
    let k = source_fields()
        .expect("the fields")
        .iter()
        .position(|f| f == "energy_drift")
        .expect("energy_drift");
    click_scope(&mut h, &mut app, CHOICE_IDS[k]);
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(
        graph(&app).node(SOURCE).map(|n| &n.occupant),
        Some(&Occupant::Field("energy_drift".into()))
    );
    let c = content(&mut app);
    assert!(c.code.contains("ctx.sample.energy_drift"), "{}", c.code);
}

/// The stain's code stands until Recompile while auto-recompile is off, and the pane says so.
#[test]
fn with_auto_recompile_off_an_edit_waits_for_recompile() {
    let (mut app, mut h) = real_stain();
    let g0 = graph(&app).clone();
    click(&mut h, &mut app, "auto-recompile");
    select(&mut app, &mut h, COMBINER);
    click(&mut h, &mut app, "Replace-L");
    let _ = h.frame(&mut app, Vec::new());
    let g1 = graph(&app).clone();
    assert_ne!(g1, g0);
    let check = |c: Content, g: &StainGraph| {
        assert_eq!(c.code, assemble(g, TIER).expect("it assembles").code)
    };
    let stale = content(&mut app);
    check(stale.clone(), &g0);
    assert!(stale.notes.iter().any(|n| n.contains("edited since")));
    rejects("the edit drawn with no recompile", || {
        check(stale.clone(), &g1)
    });
    click(&mut h, &mut app, "Recompile");
    let _ = h.frame(&mut app, Vec::new());
    let fresh = content(&mut app);
    check(fresh.clone(), &g1);
    assert!(!fresh.notes.iter().any(|n| n.contains("edited since")));
}

/// Sends the graph `g` as an edit, and recompiles.
fn set(app: &mut App<RealSide>, h: &mut Headless, g: StainGraph) {
    app.set_field(SetField {
        edit: Edit::Render(RenderField::StainGraph(g)),
        no_history: false,
    });
    app.side().recompile();
    app.side().wait_for_stain();
    for _ in 0..2 {
        let _ = h.frame(app, Vec::new());
    }
}

/// Asserts the Problems pane of a graph that does not compile: one problem, the assembler's, the status saying so, and
/// the code still the graph `last`'s.
fn check_broken(c: &Content, last: &StainGraph) {
    assert_eq!(c.problems.len(), 1, "{:?}", c.problems);
    assert!(c.problems[0].contains("nothing"), "{:?}", c.problems);
    assert!(c.compiled.contains("does not compile"), "{}", c.compiled);
    assert_eq!(c.code, assemble(last, TIER).expect("it assembles").code);
}

#[test]
fn a_graph_that_does_not_compile_is_a_problem_and_the_last_code_stays() {
    let (mut app, mut h) = real_stain();
    let g0 = graph(&app).clone();
    let rows = app.console().len();
    let mut broken = g0.clone();
    broken
        .set_occupant(
            PALETTE,
            Occupant::Custom(
                "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(nothing); }\n".into(),
            ),
        )
        .expect("the graph takes it: the compile is the assembler's");
    set(&mut app, &mut h, broken.clone());
    let c = content(&mut app);
    check_broken(&c, &g0);
    assert!(c.node_code[&PALETTE].contains("nothing"), "the node's own");
    assert!(frame_texts(&mut h, &mut app)
        .iter()
        .any(|t| t == "Problems · 1"));
    assert!(logged(&app, rows)
        .iter()
        .any(|m| m.starts_with("stain does not compile")));
    // Back to the graph that compiles, nothing assembled again.
    let n = app.side().assemblies();
    app.undo();
    let _ = h.frame(&mut app, Vec::new());
    let back = content(&mut app);
    check_content(Some(back.clone()), &g0);
    assert_eq!(app.side().assemblies(), n);
    rejects("a graph that compiles", || check_broken(&back, &g0));
}

/// Asserts that an edit assembled `want` times.
fn check_assembled(before: u64, after: u64, want: u64) {
    assert_eq!(after - before, want);
}

#[test]
fn a_param_edit_assembles_nothing_and_an_occupant_swap_does() {
    let (mut app, mut h) = real_stain();
    select(&mut app, &mut h, PALETTE);
    let (g0, d0, rows) = (graph(&app).clone(), depth(&app), app.console().len());
    let n0 = app.side().assemblies();
    assert_eq!(n0, 1, "the start graph, once, however many frames drew it");
    // The palette's swatch set, a whole-number param: a click on its slider's right end sets 1.
    let slider = app
        .keyboard
        .place_of(PARAM_IDS[0])
        .expect("the swatch set's slider")
        .rect;
    let track = nodes(&mut h, &mut app)
        .into_iter()
        .find(|n| n.role == eframe::egui::accesskit::Role::Slider)
        .expect("a slider")
        .rect;
    assert!(slider.intersects(track));
    let at = eframe::egui::pos2(track.min.x + 0.8 * track.width(), track.center().y);
    for events in crate::capture::click(at) {
        let _ = h.frame(&mut app, events);
    }
    let _ = h.frame(&mut app, Vec::new());
    let set = |g: &StainGraph| {
        g.node(PALETTE).and_then(|n| n.params.get("swatch_set")) == Some(&vec![1.0])
    };
    check_one_edit((&g0, d0, rows), &app, set);
    let n1 = app.side().assemblies();
    check_assembled(n0, n1, 0);
    let c = content(&mut app);
    check_content(Some(c), &graph(&app).clone());
    assert!(
        logged(&app, rows)
            .iter()
            .any(|m| m.starts_with("stain recompiled · cached")),
        "{:?}",
        logged(&app, rows)
    );
    click(&mut h, &mut app, "margin");
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(
        graph(&app).node(PALETTE).map(|n| &n.occupant),
        Some(&Occupant::Builtin("margin".into()))
    );
    let n2 = app.side().assemblies();
    check_assembled(n1, n2, 1);
    rejects("an occupant swap counted as a param edit", || {
        check_assembled(n1, n2, 0)
    });
}

/// An arrow on a whole-number param steps it by one: a hundredth of its range is no value of it.
#[test]
fn an_arrow_steps_a_whole_number_param() {
    use eframe::egui::{Key, Modifiers};
    let (mut app, mut h) = real_stain();
    select(&mut app, &mut h, PALETTE);
    let g = graph(&app).clone();
    assert!(whole(&g, PALETTE, "swatch_set"));
    assert!(!whole(&g, PALETTE, "bounded"), "a vec3 is not whole");
    app.view.focus.path = vec!["stain_inspector".into(), PARAM_IDS[0].into()];
    app.view.focus.mode = engine::contract::view_ui::KeyMode::Interaction;
    keys_on(&mut h, &mut app);
    press(&mut h, &mut app, Key::ArrowRight, Modifiers::NONE);
    let _ = h.frame(&mut app, Vec::new());
    let value = |app: &App<RealSide>| {
        graph(app)
            .node(PALETTE)
            .and_then(|n| n.params.get("swatch_set").cloned())
    };
    assert_eq!(value(&app), Some(vec![1.0]));
    rejects("a refused step", || assert_eq!(value(&app), None));
}

/// The start state's stain is the outcome stain, its one colour the outcome palette.
#[test]
fn the_real_app_opens_on_the_outcome_stain() {
    let check = |g: &StainGraph| {
        let colours: Vec<&Occupant> = g
            .nodes()
            .filter(|n| n.kind == NodeKind::Colour)
            .map(|n| &n.occupant)
            .collect();
        assert_eq!(colours, [&Occupant::Builtin("outcome_state".into())]);
        assert!(!g.nodes().any(|n| n.kind == NodeKind::Brightness));
    };
    check(&start().1.stain_graph);
    rejects("the mock's stain", || {
        check(&crate::mock::state(0.0).1.stain_graph)
    });
}
