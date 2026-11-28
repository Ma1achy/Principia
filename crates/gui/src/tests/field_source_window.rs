//! The field source in the window: the inspector's menu offers one "Field" source, not a choice per field; the
//! inspector draws its field picker, a click on which sets the node's field, one undoable edit, spelt as the stain
//! saves it, that changes the picture; and a stain saved with the old per-field sources opens with each shown as the
//! Field source, its own field picked. Each check also runs on an input it must reject (through `rejects`).

use std::sync::atomic::Ordering;

use eframe::egui::{Rect, Shape};
use engine::contract::render_state::{NodeId, NodeKind, Occupant, StainGraph};
use engine::contract::set_field::{Edit, RenderField, SetField};
use engine::contract::stain::describe::{describe_node, Control};
use engine::contract::view_ui::Mode;

use super::mock_stain_preview::{check_one_edit, depth, graph, on_stain};
use super::real_stain_drawn::{app as real_app, picture, Counting};
use super::support::{click, click_scope, frame_texts, mock_app, rejects, shapes};
use crate::app::App;
use crate::headless::Headless;
use crate::keyboard::scopes::ScopeId;
use crate::real::RealCanvas;
use crate::side::{EngineSide, RealSide};
use crate::stain::catalogue::{catalogue, Choice, FIELD};
use crate::stain::{choices, controls, CHOICE_IDS};

/// The source of the start graph, the mock's and the real one's: the field `state`.
const SOURCE: NodeId = NodeId(2);
/// The real start graph's colour, the outcome palette.
const PALETTE: NodeId = NodeId(3);

/// A stain saved before the field source was one node: its sources are `source/ctx.E_0` (node 2) and
/// `source/ctx.state` (node 3), each saved as its own generated text.
const BEFORE: &str = include_str!("../../../engine/tests/data/saved_stains/before_change.json");

/// Asserts that `offered` holds one source reading a field, named "Field", and no choice per field.
fn check_menu(offered: &[Choice]) {
    let sources: Vec<&Choice> = offered
        .iter()
        .filter(|c| c.slot == NodeKind::Source)
        .collect();
    let fields: Vec<&str> = sources
        .iter()
        .filter(|c| matches!(c.occupant, Occupant::Field(_)))
        .map(|c| c.name)
        .collect();
    assert_eq!(fields, [FIELD], "the field sources offered");
    assert!(
        !sources.iter().any(|c| c.name.starts_with("ctx.")),
        "a choice per field: {:?}",
        sources.iter().map(|c| c.name).collect::<Vec<_>>()
    );
    assert!(
        !sources
            .iter()
            .any(|c| c.occupant == Occupant::Builtin("field".into())),
        "the field source offered twice"
    );
}

#[test]
fn the_menu_offers_one_field_source() {
    let held = catalogue();
    assert!(held.problems.is_empty(), "{:?}", held.problems);
    let offered = &held.tables.choices;
    check_menu(offered);
    // The other sources are still offered.
    assert!(offered
        .iter()
        .any(|c| c.slot == NodeKind::Source && c.occupant == Occupant::Builtin("zero".into())));
    let mut per_field = offered.to_vec();
    let one = per_field
        .iter()
        .find(|c| matches!(c.occupant, Occupant::Field(_)))
        .cloned()
        .expect("the Field choice");
    per_field.push(Choice {
        occupant: Occupant::Field("ftle".into()),
        name: "ctx.ftle",
        ..one
    });
    rejects("a choice per field", || check_menu(&per_field));
}

/// The scopes of the picker's buttons in node `node`'s inspector, one a field, in [`controls::field_names`]'s order.
fn picker_scopes<S: EngineSide>(app: &mut App<S>, node: NodeId) -> Vec<ScopeId> {
    let c = app.side().stain_content().expect("the stain content");
    let d = &c.described[&node];
    controls::sections(d)
        .iter()
        .flat_map(|s| &s.controls)
        .find(|(k, _)| d.controls[*k].control == Control::FieldPicker)
        .map(|(_, p)| p.parts.iter().map(|q| q.scope.id).collect())
        .expect("a field picker")
}

/// The scope of the picker's button for `field` in node `node`'s inspector.
pub(super) fn picker<S: EngineSide>(app: &mut App<S>, node: NodeId, field: &str) -> ScopeId {
    let j = controls::field_names()
        .iter()
        .position(|f| f == field)
        .unwrap_or_else(|| panic!("no field `{field}`"));
    picker_scopes(app, node)[j]
}

/// Whether the widget placed at `id` is drawn selected in `drawn`.
fn lit<S: EngineSide>(
    app: &App<S>,
    drawn: &[Shape],
    fill: eframe::egui::Color32,
    id: &str,
) -> bool {
    let r: Rect = app.keyboard.place_of(id).expect("placed").rect;
    drawn
        .iter()
        .any(|s| matches!(s, Shape::Rect(x) if x.fill == fill && r.contains(x.rect.center())))
}

/// Asserts that node `node`, selected, shows as the Field source with `field` picked: the menu's "Field" lit, the
/// picker's every field drawn and placed for the keyboard, `field`'s alone lit.
fn check_picked<S: EngineSide>(app: &mut App<S>, h: &mut Headless, node: NodeId, field: &str) {
    app.stain.canvas.selection = [node].into();
    let texts = frame_texts(h, app);
    let names = controls::field_names();
    assert!(names.len() > 1, "{names:?}");
    for f in names {
        assert!(texts.iter().any(|t| t == f), "`{f}` not in the picker");
    }
    let fill = h.ctx.global_style().visuals.selection.bg_fill;
    let drawn = shapes(h, app);
    let c = app.side().stain_content().expect("the stain content");
    let n = graph(app).node(node).expect("the node").clone();
    let field_choice = choices(&n, &c)
        .iter()
        .position(|c| c.name == FIELD)
        .expect("the Field choice");
    assert!(
        lit(app, &drawn, fill, CHOICE_IDS[field_choice]),
        "the Field choice is not shown chosen"
    );
    let scopes = picker_scopes(app, node);
    assert_eq!(scopes.len(), names.len());
    let picked: Vec<&String> = names
        .iter()
        .zip(scopes)
        .filter(|(_, id)| lit(app, &drawn, fill, id))
        .map(|(f, _)| f)
        .collect();
    assert_eq!(picked, [field], "the fields shown picked");
}

#[test]
fn the_picker_shows_the_sources_field() {
    let (mut app, mut h) = on_stain(mock_app());
    check_picked(&mut app, &mut h, SOURCE, "state");
    rejects("another field", || {
        check_picked(&mut app, &mut h, SOURCE, "ftle")
    });
}

/// The field node `node` of `g` reads, as its description says: the one its picker holds.
pub(super) fn picked(g: &StainGraph, node: NodeId) -> Option<String> {
    let d = describe_node(g.node(node)?).ok()?;
    let c = d
        .controls
        .iter()
        .find(|c| c.control == Control::FieldPicker)?;
    let j = *c.value.first()? as usize;
    controls::field_names().get(j).cloned()
}

/// Asserts that `g` saved and opened again reads `field` at node `node`, spelt as the engine reads a field source.
fn check_reopens(g: &StainGraph, node: NodeId, field: &str) {
    let text = serde_json::to_string(g).expect("saved");
    let back: StainGraph = serde_json::from_str(&text).expect("opened");
    assert_eq!(
        picked(&back, node).as_deref(),
        Some(field),
        "the field it opens on"
    );
    let n = back.node(node).expect("the node");
    assert_eq!(n.occupant, Occupant::Field(field.into()));
}

/// The real start graph with the palette swapped for Margin, which draws the field wired into its first input, the zero
/// field into its second (the outcome palette reads the outcome itself, whatever its input).
fn margin(g: &StainGraph) -> StainGraph {
    let mut g = g.clone();
    g.set_occupant(PALETTE, Occupant::Builtin("margin".into()))
        .expect("margin");
    let zero = g
        .add(NodeKind::Source, Occupant::Builtin("zero".into()))
        .expect("the zero field");
    g.connect(zero, PALETTE, 1).expect("into its second input");
    g
}

/// The preview's pixels once the stain the graph asks for has compiled.
fn shown(app: &mut App<RealSide>, h: &mut Headless, canvas: &RealCanvas) -> Vec<[u8; 3]> {
    let _ = h.frame(app, Vec::new());
    app.side().wait_for_stain();
    picture(app, h, canvas, Mode::Stain)
}

/// Sets the graph `g` as a stain opened from outside the editor is, and recompiles it, as an edit outside the editor
/// waits for Recompile.
fn open_graph(app: &mut App<RealSide>, h: &mut Headless, g: StainGraph) {
    app.set_field(SetField {
        edit: Edit::Render(RenderField::StainGraph(g)),
        no_history: false,
    });
    let _ = h.frame(app, Vec::new());
    click(h, app, "Recompile");
}

#[test]
fn a_picked_field_is_one_edit_that_changes_the_picture() {
    let backend = Counting::default();
    let (mut app, mut h, canvas) = real_app(&backend);
    let opened = backend.0.load(Ordering::SeqCst);
    let start = margin(graph(&app));
    open_graph(&mut app, &mut h, start.clone());
    let before = shown(&mut app, &mut h, &canvas);
    assert_eq!(graph(&app), &start);
    let problems = app.side().stain_content().expect("the content").problems;
    assert!(problems.is_empty(), "{problems:?}");
    let (g0, d0, rows) = (graph(&app).clone(), depth(&app), app.console().len());
    assert_eq!(picked(&g0, SOURCE).as_deref(), Some("state"));
    app.stain.canvas.selection = [SOURCE].into();
    let _ = h.frame(&mut app, Vec::new());
    let detail = picker(&mut app, SOURCE, "detail");
    click_scope(&mut h, &mut app, detail);
    let _ = h.frame(&mut app, Vec::new());
    let reads =
        |field: &'static str| move |g: &StainGraph| picked(g, SOURCE).as_deref() == Some(field);
    check_one_edit((&g0, d0, rows), &app, reads("detail"));
    rejects("the field unchanged", || {
        check_one_edit((&g0, d0, rows), &app, reads("state"))
    });
    app.side().wait_for_stain();
    let code = app.side().stain_content().expect("the content").code;
    assert!(code.contains("ctx.sample.detail"), "{code}");
    let after = shown(&mut app, &mut h, &canvas);
    assert_eq!(before.len(), after.len());
    assert_ne!(before, after, "the picked field did not change the picture");
    assert_eq!(
        backend.0.load(Ordering::SeqCst),
        opened,
        "the field computed again"
    );
    let edited = graph(&app).clone();
    app.undo();
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(graph(&app), &g0);
    assert_eq!(shown(&mut app, &mut h, &canvas), before, "the undo");
    // Saved and opened, it is the field source of the field picked, and draws as it did.
    check_reopens(&edited, SOURCE, "detail");
    rejects("the stain before the pick", || {
        check_reopens(&g0, SOURCE, "detail")
    });
    let back: StainGraph =
        serde_json::from_str(&serde_json::to_string(&edited).expect("saved")).expect("opened");
    open_graph(&mut app, &mut h, back);
    assert_eq!(
        shown(&mut app, &mut h, &canvas),
        after,
        "the stain opened again"
    );
}

#[test]
fn old_per_field_sources_open_as_the_field_source() {
    let g: StainGraph = serde_json::from_str(BEFORE).expect("the old stain opens");
    let (mut app, mut h) = on_stain(mock_app());
    app.set_field(SetField {
        edit: Edit::Render(RenderField::StainGraph(g)),
        no_history: false,
    });
    let _ = h.frame(&mut app, Vec::new());
    check_picked(&mut app, &mut h, NodeId(2), "E_0");
    check_picked(&mut app, &mut h, NodeId(3), "state");
    rejects("another field", || {
        check_picked(&mut app, &mut h, NodeId(2), "state")
    });
}
