//! `mock_keyboard` on the Stain page (REQ-GUI-175, REQ-GUI-128; render_gui_spec §G3, Part II §1): Tab and Shift+Tab
//! walk the page's big scopes — the drawer, the canvas, the inspector, the code and Problems — with the top bar and the
//! footer; Enter goes into each on its first element, in navigation mode; the arrows walk its elements in the order
//! drawn, the canvas's nodes among them; Enter on a row, a node, a toggle, a choice or a button acts as its click; a
//! param takes the arrows in interaction mode by `Bounded`'s step, ×10 with Shift and ×0.1 with Alt, one `SetField`
//! each; Esc steps back out; a node the focus lands on is brought into view; and the ring is drawn once, apart from
//! the selection's outline. Each check also runs on an input it must reject (R-176, through `rejects`).

use eframe::egui::{self, vec2, Key, Modifiers};
use engine::contract::render_state::{NodeId, Occupant, StainGraph};
use engine::contract::view_ui::{KeyMode, Mode};

use super::support::{focus, frame_texts, headless, mock_app, press, rejects, shapes};
use crate::app::App;
use crate::explore::breadcrumb::{NAVIGATION, RING_WIDTH};
use crate::headless::Headless;
use crate::layout::Layout;
use crate::mock::stain::{COMBINER, FADE, RECOMPILED};
use crate::side::MockSide;
use crate::stain::{Regions, View};

const NONE: Modifiers = Modifiers::NONE;
const SHIFT: Modifiers = Modifiers::SHIFT;
const ALT: Modifiers = Modifiers::ALT;

/// The mock's colour and its brightness.
const PALETTE: NodeId = NodeId(4);
const LATE_FADE: NodeId = NodeId(5);

/// The mock app on the Stain page, one frame drawn.
fn stain_app() -> (App<MockSide>, Headless) {
    let mut app = mock_app();
    app.view.mode = Mode::Stain;
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    (app, h)
}

fn keys(h: &mut Headless, app: &mut App<MockSide>, keys: &[(Key, Modifiers)]) {
    for (key, modifiers) in keys {
        press(h, app, *key, *modifiers);
    }
}

/// Tab `tabs` times, Enter, then → `rights` times.
fn walk(h: &mut Headless, app: &mut App<MockSide>, tabs: usize, rights: usize) {
    let mut steps = vec![(Key::Tab, NONE); tabs];
    steps.push((Key::Enter, NONE));
    steps.extend(vec![(Key::ArrowRight, NONE); rights]);
    keys(h, app, &steps);
}

/// The focus, its path and mode.
fn at(app: &App<MockSide>) -> (Vec<&str>, KeyMode) {
    (focus(app), app.view.focus.mode)
}

fn graph(app: &App<MockSide>) -> &StainGraph {
    &app.snapshot().render.stain_graph
}

fn depth(app: &App<MockSide>) -> u32 {
    app.snapshot().history.undo_depth
}

// --- Tab, and Enter into each big scope ------------------------------------------------------------------------------

/// The focus paths seen, each joined by `/`, are `want`.
fn check_tabs(seen: &[String], want: &[&str]) {
    assert_eq!(seen, want);
}

#[test]
fn mock_keyboard_stain_tab_and_shift_tab_walk_the_big_scopes() {
    let (mut app, mut h) = stain_app();
    let order = [
        "top_bar",
        "stain_library",
        "stain_canvas",
        "stain_inspector",
        "stain_code",
        "footer",
        "top_bar",
    ];
    let mut seen = Vec::new();
    for _ in order {
        press(&mut h, &mut app, Key::Tab, NONE);
        seen.push(focus(&app).join("/"));
    }
    check_tabs(&seen, &order);
    // Shift+Tab walks them back.
    let mut back = Vec::new();
    for _ in 0..3 {
        press(&mut h, &mut app, Key::Tab, SHIFT);
        back.push(focus(&app).join("/"));
    }
    assert_eq!(back, ["footer", "stain_code", "stain_inspector"]);
    // The breadcrumb names the scope.
    let texts = frame_texts(&mut h, &mut app);
    assert!(
        texts.iter().any(|t| t == "Preview & node inspector"),
        "{texts:?}"
    );
    rejects("Explore's order", || {
        check_tabs(
            &seen,
            &[
                "top_bar",
                "manifold_view",
                "figure",
                "trajectory",
                "legend",
                "time",
                "footer",
            ],
        )
    });
}

/// Enter goes into each big scope on its first element, in navigation mode.
#[test]
fn mock_keyboard_stain_enter_lands_on_the_first_element() {
    for (tabs, want) in [
        (2, ["stain_library", "stain_lib_collapse"]),
        (3, ["stain_canvas", "stain_view_graph"]),
        (5, ["stain_code", "stain_auto_recompile"]),
    ] {
        let (mut app, mut h) = stain_app();
        walk(&mut h, &mut app, tabs, 0);
        assert_eq!(at(&app), (want.to_vec(), KeyMode::Navigation));
    }
    // The inspector with nothing selected has no element: Enter stays on the scope.
    let (mut app, mut h) = stain_app();
    walk(&mut h, &mut app, 4, 0);
    assert_eq!(focus(&app), ["stain_inspector"]);
    rejects("a landing in interaction mode", || {
        assert_eq!(app.view.focus.mode, KeyMode::Interaction)
    });
}

// --- The drawer ------------------------------------------------------------------------------------------------------

#[test]
fn mock_keyboard_stain_library_rows_and_groups() {
    let (mut app, mut h) = stain_app();
    // collapse, filter, the first group's heading, its first row.
    walk(&mut h, &mut app, 2, 3);
    assert_eq!(focus(&app), ["stain_library", "stain_lib_row_0"]);
    press(&mut h, &mut app, Key::Enter, NONE);
    assert_eq!(app.stain.library.selected, Some("Total energy E₀"));
    assert_eq!(app.view.focus.mode, KeyMode::Navigation);
    // Back to the heading: Enter closes the group, its rows leave the walk.
    press(&mut h, &mut app, Key::ArrowLeft, NONE);
    assert_eq!(focus(&app), ["stain_library", "stain_lib_group_0"]);
    press(&mut h, &mut app, Key::Enter, NONE);
    assert!(app.stain.library.closed.contains(&0));
    press(&mut h, &mut app, Key::ArrowRight, NONE);
    assert_eq!(focus(&app), ["stain_library", "stain_lib_group_1"]);
    // The drawer's disabled buttons are walked but do nothing.
    let rows = app.console().len();
    press(&mut h, &mut app, Key::ArrowDown, NONE);
    for _ in 0..40 {
        press(&mut h, &mut app, Key::ArrowRight, NONE);
    }
    assert_eq!(focus(&app), ["stain_library", "stain_lib_export"]);
    press(&mut h, &mut app, Key::Enter, NONE);
    assert_eq!(app.console().len(), rows);
    // The collapse: Enter folds the drawer to its strip, its one element.
    press(&mut h, &mut app, Key::Escape, NONE);
    walk(&mut h, &mut app, 0, 0);
    assert_eq!(focus(&app), ["stain_library", "stain_lib_collapse"]);
    press(&mut h, &mut app, Key::Enter, NONE);
    assert!(app.stain.library.collapsed);
    press(&mut h, &mut app, Key::ArrowRight, NONE);
    assert_eq!(focus(&app), ["stain_library", "stain_lib_collapse"]);
    rejects("a row selected by the heading", || {
        assert_eq!(app.stain.library.selected, Some("Shape n̂"))
    });
}

// --- The canvas ------------------------------------------------------------------------------------------------------

#[test]
fn mock_keyboard_stain_canvas_walks_toggles_then_nodes() {
    let (mut app, mut h) = stain_app();
    walk(&mut h, &mut app, 3, 0);
    let mut seen = vec![focus(&app)[1].to_owned()];
    for _ in 0..9 {
        press(&mut h, &mut app, Key::ArrowRight, NONE);
        seen.push(focus(&app)[1].to_owned());
    }
    let want = [
        "stain_view_graph",
        "stain_view_pipeline",
        "stain_view_node",
        "stain_node_2",
        "stain_node_3",
        "stain_node_4",
        "stain_node_5",
        "stain_node_0",
        "stain_node_1",
        "stain_node_1",
    ];
    assert_eq!(seen, want, "by column, then down it; stopping at the last");
    rejects("the nodes in id order", || {
        assert_eq!(&seen[3..5], ["stain_node_0", "stain_node_1"])
    });
}

#[test]
fn mock_keyboard_stain_enter_on_a_node_selects_it() {
    let (mut app, mut h) = stain_app();
    walk(&mut h, &mut app, 3, 5);
    assert_eq!(focus(&app), ["stain_canvas", "stain_node_4"]);
    press(&mut h, &mut app, Key::Enter, NONE);
    assert_eq!(app.stain.selected(), Some(PALETTE));
    assert_eq!(app.view.focus.mode, KeyMode::Navigation);
    let texts = frame_texts(&mut h, &mut app);
    assert!(
        texts
            .iter()
            .any(|t| t == "Graph canvas › colour · outcome_state"),
        "{texts:?}"
    );
    // The ring is drawn once on the node, its selection outline apart from it.
    let rings = ring_colours(&mut h, &mut app);
    assert_eq!(rings, [NAVIGATION]);
    // The toggle by Enter: Pipeline WGSL, and back to Graph.
    keys(
        &mut h,
        &mut app,
        &[
            (Key::ArrowLeft, NONE),
            (Key::ArrowLeft, NONE),
            (Key::ArrowLeft, NONE),
            (Key::ArrowLeft, NONE),
        ],
    );
    assert_eq!(focus(&app), ["stain_canvas", "stain_view_pipeline"]);
    press(&mut h, &mut app, Key::Enter, NONE);
    assert_eq!(app.stain.view, View::Pipeline);
    press(&mut h, &mut app, Key::ArrowLeft, NONE);
    press(&mut h, &mut app, Key::Enter, NONE);
    assert_eq!(app.stain.view, View::Graph);
    rejects("another node selected", || {
        assert_eq!(app.stain.selected(), Some(LATE_FADE))
    });
}

/// A node the focus lands on, panned out of view, is brought back into it.
#[test]
fn mock_keyboard_stain_focus_reveals_a_node() {
    let (mut app, mut h) = stain_app();
    app.stain.canvas.view.pan = vec2(-3000.0, 0.0);
    walk(&mut h, &mut app, 3, 3);
    assert_eq!(focus(&app), ["stain_canvas", "stain_node_2"]);
    let _ = h.frame(&mut app, Vec::new());
    let ppp = h.pixels_per_point();
    let canvas = Regions::new(&Layout::new(h.screen(), ppp), ppp, false).canvas;
    let r = app
        .stain
        .canvas
        .drawn(graph(&app), canvas.min)
        .into_iter()
        .find(|d| d.id == NodeId(2))
        .unwrap()
        .rect;
    assert!(canvas.contains_rect(r), "{r:?} in {canvas:?}");
    rejects("a node left out of view", || {
        assert!(canvas.contains_rect(r.translate(vec2(-3000.0, 0.0))))
    });
}

// --- The inspector ---------------------------------------------------------------------------------------------------

/// The fade's value, as the graph holds it.
fn fade(app: &App<MockSide>) -> f64 {
    let n = graph(app).node(LATE_FADE).unwrap();
    n.params.get(FADE).map_or(0.12, |v| v[0])
}

/// The late fade selected by the keyboard, the focus on its param in interaction mode.
fn on_the_fade() -> (App<MockSide>, Headless) {
    let (mut app, mut h) = stain_app();
    walk(&mut h, &mut app, 3, 6);
    press(&mut h, &mut app, Key::Enter, NONE);
    assert_eq!(app.stain.selected(), Some(LATE_FADE));
    keys(&mut h, &mut app, &[(Key::Tab, NONE), (Key::Enter, NONE)]);
    assert_eq!(
        at(&app),
        (
            vec!["stain_inspector", "stain_insp_param_0"],
            KeyMode::Navigation
        )
    );
    press(&mut h, &mut app, Key::Enter, NONE);
    assert_eq!(app.view.focus.mode, KeyMode::Interaction);
    (app, h)
}

/// Each arrow on a param: `Bounded`'s 1% of its range, ×10 with Shift, ×0.1 with Alt; one `SetField` each.
fn check_steps(steps: &[(Modifiers, f64)]) -> Vec<f64> {
    let (mut app, mut h) = on_the_fade();
    let mut got = Vec::new();
    for (i, (m, _)) in steps.iter().enumerate() {
        let (before, d) = (fade(&app), depth(&app));
        press(&mut h, &mut app, Key::ArrowRight, *m);
        let _ = h.frame(&mut app, Vec::new());
        assert_eq!(depth(&app), d + 1, "step {i}: one SetField");
        got.push(fade(&app) - before);
    }
    for (g, (_, want)) in got.iter().zip(steps) {
        assert!((g - want).abs() < 1e-9, "{got:?}");
    }
    got
}

#[test]
fn mock_keyboard_stain_param_arrows_step_by_range() {
    check_steps(&[(NONE, 0.01), (SHIFT, 0.1), (ALT, 0.001)]);
    rejects("Shift's step as the base", || {
        check_steps(&[(SHIFT, 0.01)]);
    });
}

/// The param is kept in its range: at its top, → leaves it there and sends nothing.
#[test]
fn mock_keyboard_stain_param_clamps_at_its_bound() {
    let (mut app, mut h) = on_the_fade();
    for _ in 0..10 {
        press(&mut h, &mut app, Key::ArrowRight, SHIFT);
    }
    let _ = h.frame(&mut app, Vec::new());
    assert!((fade(&app) - 1.0).abs() < 1e-9, "{}", fade(&app));
    let d = depth(&app);
    press(&mut h, &mut app, Key::ArrowRight, NONE);
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(depth(&app), d, "no SetField past the bound");
    press(&mut h, &mut app, Key::ArrowLeft, NONE);
    let _ = h.frame(&mut app, Vec::new());
    assert!((fade(&app) - 0.99).abs() < 1e-9);
    rejects("a value past its bound", || assert!(fade(&app) > 1.0));
}

/// Esc ends the interaction on the same element, then goes up to the scope; the ring blue, then orange.
#[test]
fn mock_keyboard_stain_esc_and_the_ring() {
    let (mut app, mut h) = on_the_fade();
    let blue = egui::Visuals::dark().hyperlink_color;
    let interacting = ring_colours(&mut h, &mut app);
    assert_eq!(interacting, [blue]);
    press(&mut h, &mut app, Key::Escape, NONE);
    assert_eq!(
        at(&app),
        (
            vec!["stain_inspector", "stain_insp_param_0"],
            KeyMode::Navigation
        )
    );
    assert_eq!(ring_colours(&mut h, &mut app), [NAVIGATION]);
    press(&mut h, &mut app, Key::Escape, NONE);
    assert_eq!(focus(&app), ["stain_inspector"]);
    rejects("an orange ring interacting", || {
        assert_eq!(interacting, [NAVIGATION])
    });
}

/// Enter on a choice sets the occupant, one `SetField`; Enter on "Edit this node's WGSL" shows Node WGSL.
#[test]
fn mock_keyboard_stain_choice_and_edit_wgsl() {
    let (mut app, mut h) = stain_app();
    walk(&mut h, &mut app, 3, 7);
    press(&mut h, &mut app, Key::Enter, NONE);
    assert_eq!(app.stain.selected(), Some(COMBINER));
    walk(&mut h, &mut app, 1, 1);
    assert_eq!(focus(&app), ["stain_inspector", "stain_insp_choice_1"]);
    let d = depth(&app);
    press(&mut h, &mut app, Key::Enter, NONE);
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(depth(&app), d + 1);
    assert_eq!(
        graph(&app).node(COMBINER).unwrap().occupant,
        Occupant::Builtin("replace_l".into())
    );
    // Past the choices: Edit this node's WGSL, then Revert, disabled.
    keys(
        &mut h,
        &mut app,
        &[(Key::ArrowRight, NONE), (Key::ArrowRight, NONE)],
    );
    assert_eq!(focus(&app), ["stain_inspector", "stain_insp_edit_wgsl"]);
    press(&mut h, &mut app, Key::Enter, NONE);
    assert_eq!(app.stain.view, View::Node);
    press(&mut h, &mut app, Key::ArrowRight, NONE);
    assert_eq!(focus(&app), ["stain_inspector", "stain_insp_revert"]);
    let d = depth(&app);
    press(&mut h, &mut app, Key::Enter, NONE);
    assert_eq!(depth(&app), d, "Revert is disabled");
    rejects("the occupant unchanged", || {
        assert_eq!(
            graph(&app).node(COMBINER).unwrap().occupant,
            Occupant::Builtin("multiply".into())
        )
    });
}

// --- The code pane ---------------------------------------------------------------------------------------------------

#[test]
fn mock_keyboard_stain_code_pane_controls() {
    let (mut app, mut h) = stain_app();
    walk(&mut h, &mut app, 5, 0);
    assert!(app.stain.auto_recompile);
    press(&mut h, &mut app, Key::Enter, NONE);
    assert!(!app.stain.auto_recompile, "Enter toggles auto-recompile");
    press(&mut h, &mut app, Key::ArrowRight, NONE);
    assert_eq!(focus(&app), ["stain_code", "stain_recompile"]);
    let rows = app.console().len();
    press(&mut h, &mut app, Key::Enter, NONE);
    let _ = h.frame(&mut app, Vec::new());
    let recompiled = app.console()[rows..]
        .iter()
        .filter(|e| e.message == RECOMPILED)
        .count();
    assert_eq!(recompiled, 1);
    rejects("no recompile", || assert_eq!(recompiled, 0));
}

/// The ring's colours drawn in one more frame: each rect stroked at the ring's width.
fn ring_colours(h: &mut Headless, app: &mut App<MockSide>) -> Vec<egui::Color32> {
    shapes(h, app)
        .into_iter()
        .filter_map(|s| match s {
            egui::Shape::Rect(r) if r.stroke.width == RING_WIDTH => Some(r.stroke.color),
            _ => None,
        })
        .collect()
}
