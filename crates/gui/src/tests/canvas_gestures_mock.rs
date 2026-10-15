//! `canvas_gestures_mock` (REQ-GUI-131; render_gui_spec Part II §7): scripted pointer events for each gesture of the
//! Stain canvas produce the stated selection or position change — click-select, click on empty canvas deselecting,
//! marquee multi-select, dragging a node with its wires following and a selection moving together, space-drag and
//! middle-drag panning, scroll zooming about the pointer — first on the canvas's machine, then through the app on the
//! mock with egui's events. Each check also runs on an input it must reject (R-176, through `rejects`).

use std::collections::BTreeSet;

use eframe::egui::{
    self, pos2, vec2, Event, Key, Modifiers, MouseWheelUnit, PointerButton, Pos2, Rect,
};
use engine::contract::render_state::{NodeId, StainGraph};
use engine::contract::view_ui::Mode;

use super::support::{frame_texts, headless, mock_app, rejects};
use crate::app::App;
use crate::headless::Headless;
use crate::layout::Layout;
use crate::mock::stain::{default_graph, COMBINER, OUT};
use crate::side::MockSide;
use crate::stain::canvas::{
    Button, Canvas, Drawn, DRAG_THRESHOLD, LINE_POINTS, TITLE_H, ZOOM_MAX, ZOOM_MIN, ZOOM_PER_POINT,
};
use crate::stain::Regions;

/// The mock's nodes: its two sources, its colour and its brightness.
const OUTCOME: NodeId = NodeId(2);
const FATE: NodeId = NodeId(3);
const PALETTE: NodeId = NodeId(4);
const FADE: NodeId = NodeId(5);

/// Where the test's canvas starts on screen.
const ORIGIN: Pos2 = pos2(100.0, 50.0);

/// The canvas on the mock's graph, every node at its default place.
fn rig() -> (Canvas, StainGraph) {
    let g = default_graph();
    let mut c = Canvas::default();
    c.place(&g);
    (c, g)
}

fn drawn(c: &Canvas, g: &StainGraph) -> Vec<Drawn> {
    c.drawn(g, ORIGIN)
}

fn rect_of(c: &Canvas, g: &StainGraph, id: NodeId) -> Rect {
    drawn(c, g)
        .into_iter()
        .find(|d| d.id == id)
        .expect("the node is drawn")
        .rect
}

/// A point on node `id`'s title bar.
fn title_of(c: &Canvas, g: &StainGraph, id: NodeId) -> Pos2 {
    let r = rect_of(c, g, id);
    pos2(r.min.x + 30.0, r.min.y + TITLE_H / 2.0)
}

/// A point on empty canvas, above every node.
fn empty() -> Pos2 {
    ORIGIN + vec2(10.0, 10.0)
}

/// A primary press at `a`, moves to each of `path`, and a release at the last.
fn drag(c: &mut Canvas, g: &StainGraph, a: Pos2, path: &[Pos2], button: Button, space: bool) {
    let nodes = drawn(c, g);
    c.press(a, button, space, &nodes);
    for p in path {
        c.moved(*p);
    }
    let end = path.last().copied().unwrap_or(a);
    let nodes = drawn(c, g);
    c.release(end, &nodes);
}

fn selection(ids: &[NodeId]) -> BTreeSet<NodeId> {
    ids.iter().copied().collect()
}

// --- Select and deselect -----------------------------------------------------------------------------------------

fn check_selected(c: &Canvas, want: &[NodeId]) {
    assert_eq!(c.selection, selection(want), "the selection");
}

#[test]
fn canvas_gestures_mock_click_selects_and_empty_deselects() {
    let (mut c, g) = rig();
    let at = title_of(&c, &g, PALETTE);
    drag(&mut c, &g, at, &[], Button::Primary, false);
    check_selected(&c, &[PALETTE]);
    rejects("another node selected", || check_selected(&c, &[OUTCOME]));
    // A click on another node selects it alone.
    let at = title_of(&c, &g, FATE);
    drag(&mut c, &g, at, &[], Button::Primary, false);
    check_selected(&c, &[FATE]);
    // A click on empty canvas deselects.
    drag(&mut c, &g, empty(), &[], Button::Primary, false);
    check_selected(&c, &[]);
    rejects("a selection kept", || check_selected(&c, &[FATE]));
}

/// A click on a node of a selection selects it alone; pressing it keeps the selection, so a drag takes it all.
#[test]
fn canvas_gestures_mock_click_in_a_selection_narrows_it() {
    let (mut c, g) = rig();
    c.selection = selection(&[OUTCOME, PALETTE]);
    let at = title_of(&c, &g, PALETTE);
    let nodes = drawn(&c, &g);
    c.press(at, Button::Primary, false, &nodes);
    check_selected(&c, &[OUTCOME, PALETTE]);
    c.release(at, &nodes);
    check_selected(&c, &[PALETTE]);
    // An unselected node pressed is selected at once.
    let at = title_of(&c, &g, FADE);
    c.press(at, Button::Primary, false, &nodes);
    check_selected(&c, &[FADE]);
    rejects("the old selection kept", || check_selected(&c, &[PALETTE]));
}

// --- Marquee -----------------------------------------------------------------------------------------------------

#[test]
fn canvas_gestures_mock_marquee_selects_what_it_touches() {
    let (mut c, g) = rig();
    let palette = rect_of(&c, &g, PALETTE);
    // From empty canvas left of the colour column, down and right to just inside the palette's top-left corner.
    let from = pos2(palette.min.x - 20.0, palette.min.y - 20.0);
    let to = palette.min + vec2(5.0, 5.0);
    drag(&mut c, &g, from, &[to], Button::Primary, false);
    check_selected(&c, &[PALETTE]);
    rejects("a marquee taking the source beside it", || {
        check_selected(&c, &[OUTCOME, PALETTE])
    });
    // A marquee over the whole first column takes both sources.
    let (a, b) = (rect_of(&c, &g, OUTCOME), rect_of(&c, &g, FATE));
    drag(
        &mut c,
        &g,
        a.min - vec2(10.0, 10.0),
        &[b.max + vec2(10.0, 10.0)],
        Button::Primary,
        false,
    );
    check_selected(&c, &[OUTCOME, FATE]);
    // While dragging, the marquee is drawn; after the release it is gone.
    let nodes = drawn(&c, &g);
    c.press(empty(), Button::Primary, false, &nodes);
    c.moved(empty() + vec2(50.0, 40.0));
    assert_eq!(
        c.marquee,
        Some(Rect::from_two_pos(empty(), empty() + vec2(50.0, 40.0)))
    );
    c.release(empty() + vec2(50.0, 40.0), &nodes);
    assert_eq!(c.marquee, None);
}

/// The marquee's edge: touching a node's edge takes it; stopping a point short does not.
#[test]
fn canvas_gestures_mock_marquee_edge() {
    let (c0, g) = rig();
    let palette = rect_of(&c0, &g, PALETTE);
    let from = pos2(palette.min.x - 30.0, palette.min.y + 10.0);
    let touch = |x: f32| {
        let mut c = c0.clone();
        drag(
            &mut c,
            &g,
            from,
            &[pos2(x, palette.min.y + 20.0)],
            Button::Primary,
            false,
        );
        c.selection
    };
    assert_eq!(touch(palette.min.x), selection(&[PALETTE]));
    assert_eq!(touch(palette.min.x - 1.0), selection(&[]));
    rejects("a marquee short of the edge read as touching", || {
        assert_eq!(touch(palette.min.x - 1.0), selection(&[PALETTE]))
    });
}

// --- Move ----------------------------------------------------------------------------------------------------------

#[test]
fn canvas_gestures_mock_drag_moves_a_node_and_its_wires_follow() {
    let (mut c, g) = rig();
    let before = drawn(&c, &g);
    let at = title_of(&c, &g, PALETTE);
    let path = [at + vec2(10.0, 5.0), at + vec2(40.0, 30.0)];
    drag(&mut c, &g, at, &path, Button::Primary, false);
    let after = drawn(&c, &g);
    let find = |v: &[Drawn], id| v.iter().find(|d| d.id == id).cloned().expect("drawn");
    let moved = |id| find(&after, id).rect.min - find(&before, id).rect.min;
    assert_eq!(moved(PALETTE), vec2(40.0, 30.0));
    assert_eq!(moved(OUTCOME), vec2(0.0, 0.0), "a node not dragged moved");
    // The wires follow: the palette's out-port and in-port move with it, the source's stay.
    let (b, a) = (find(&before, PALETTE), find(&after, PALETTE));
    assert_eq!(a.out_port(1.0) - b.out_port(1.0), vec2(40.0, 30.0));
    assert_eq!(a.in_port(0, 1.0) - b.in_port(0, 1.0), vec2(40.0, 30.0));
    assert_eq!(
        find(&after, OUTCOME).out_port(1.0),
        find(&before, OUTCOME).out_port(1.0)
    );
    check_selected(&c, &[PALETTE]);
    rejects("a node left in place", || {
        assert_eq!(moved(PALETTE), vec2(0.0, 0.0))
    });
}

/// The drag's threshold: a move of at most `DRAG_THRESHOLD` is still a click; past it, a drag.
#[test]
fn canvas_gestures_mock_drag_threshold() {
    let (c0, g) = rig();
    let at = title_of(&c0, &g, PALETTE);
    let start = c0.positions[&PALETTE];
    let moved_by = |dx: f32| {
        let mut c = c0.clone();
        c.selection = selection(&[OUTCOME, PALETTE]);
        drag(
            &mut c,
            &g,
            at,
            &[at + vec2(dx, 0.0)],
            Button::Primary,
            false,
        );
        (c.positions[&PALETTE] - start, c.selection)
    };
    // At the threshold: no move, and the click narrows the selection to the node.
    assert_eq!(
        moved_by(DRAG_THRESHOLD),
        (vec2(0.0, 0.0), selection(&[PALETTE]))
    );
    // Just past it: the whole move, and the selection kept.
    let (moved, kept) = moved_by(DRAG_THRESHOLD + 0.5);
    assert_eq!(moved, vec2(DRAG_THRESHOLD + 0.5, 0.0));
    assert_eq!(kept, selection(&[OUTCOME, PALETTE]));
    rejects("a move at the threshold read as a drag", || {
        assert_eq!(moved_by(DRAG_THRESHOLD).0, vec2(DRAG_THRESHOLD, 0.0))
    });
}

#[test]
fn canvas_gestures_mock_selection_moves_together() {
    let (mut c, g) = rig();
    c.selection = selection(&[OUTCOME, PALETTE]);
    let start = c.positions.clone();
    let at = title_of(&c, &g, OUTCOME);
    drag(
        &mut c,
        &g,
        at,
        &[at + vec2(-12.0, 25.0)],
        Button::Primary,
        false,
    );
    let moved = |id: NodeId| c.positions[&id] - start[&id];
    assert_eq!(moved(OUTCOME), vec2(-12.0, 25.0));
    assert_eq!(moved(PALETTE), vec2(-12.0, 25.0));
    assert_eq!(moved(FATE), vec2(0.0, 0.0));
    assert_eq!(moved(COMBINER), vec2(0.0, 0.0));
    rejects("a selected node left behind", || {
        assert_eq!(moved(PALETTE), vec2(0.0, 0.0))
    });
}

/// At zoom 2 a drag moves a node by half the pointer's move on the canvas, so it stays under the pointer.
#[test]
fn canvas_gestures_mock_drag_at_zoom() {
    let (mut c, g) = rig();
    c.view.zoom = 2.0;
    let start = c.positions[&FADE];
    let at = title_of(&c, &g, FADE);
    drag(
        &mut c,
        &g,
        at,
        &[at + vec2(20.0, -10.0)],
        Button::Primary,
        false,
    );
    assert_eq!(c.positions[&FADE] - start, vec2(10.0, -5.0));
    rejects("a drag unscaled by the zoom", || {
        assert_eq!(c.positions[&FADE] - start, vec2(20.0, -10.0))
    });
}

// --- Pan and zoom --------------------------------------------------------------------------------------------------

#[test]
fn canvas_gestures_mock_middle_and_space_drag_pan() {
    let (c0, g) = rig();
    let at = title_of(&c0, &g, PALETTE);
    let pan = |button: Button, space: bool| {
        let mut c = c0.clone();
        drag(
            &mut c,
            &g,
            at,
            &[at + vec2(15.0, 0.0), at + vec2(30.0, -20.0)],
            button,
            space,
        );
        c
    };
    for c in [
        pan(Button::Middle, false),
        pan(Button::Primary, true),
        pan(Button::Middle, true),
    ] {
        assert_eq!(c.view.pan, vec2(30.0, -20.0));
        assert_eq!(c.positions, c0.positions, "a pan moved a node");
        assert_eq!(c.selection, c0.selection, "a pan selected");
    }
    // The primary button alone drags the node instead.
    let plain = pan(Button::Primary, false);
    assert_eq!(plain.view.pan, vec2(0.0, 0.0));
    rejects("a plain drag read as a pan", || {
        assert_eq!(pan(Button::Primary, false).view.pan, vec2(30.0, -20.0))
    });
}

#[test]
fn canvas_gestures_mock_scroll_zooms_about_the_pointer() {
    let (mut c, g) = rig();
    let p = title_of(&c, &g, PALETTE);
    let canvas_point = |c: &Canvas| (p - ORIGIN - c.view.pan) / c.view.zoom;
    let before = canvas_point(&c);
    c.scroll(ORIGIN, p, 100.0);
    assert!((c.view.zoom - (100.0 * ZOOM_PER_POINT).exp()).abs() < 1e-6);
    assert!(
        (canvas_point(&c) - before).length() < 1e-3,
        "the point under the pointer moved"
    );
    c.scroll(ORIGIN, p, -100.0);
    assert!((c.view.zoom - 1.0).abs() < 1e-6, "zooming back");
    // From a panned view, at another point: that point stays too.
    c.view.pan = vec2(-35.0, 20.0);
    let q = ORIGIN + vec2(300.0, 140.0);
    let under = |c: &Canvas| (q - ORIGIN - c.view.pan) / c.view.zoom;
    let was = under(&c);
    c.scroll(ORIGIN, q, -150.0);
    assert!(
        (under(&c) - was).length() < 1e-3,
        "{:?} from {was:?}",
        under(&c)
    );
    assert!(c.view.pan != vec2(-35.0, 20.0));
    c.scroll(ORIGIN, q, 150.0);
    assert!((c.view.pan - vec2(-35.0, 20.0)).length() < 1e-3);
    // The zoom stays within its bounds.
    c.scroll(ORIGIN, p, 1e6);
    assert_eq!(c.view.zoom, ZOOM_MAX);
    c.scroll(ORIGIN, p, -1e6);
    assert_eq!(c.view.zoom, ZOOM_MIN);
    rejects("a zoom past its bound", || assert_eq!(c.view.zoom, 0.0));
}

/// A press during a gesture is ignored, and the gesture runs on.
#[test]
fn canvas_gestures_mock_a_press_during_a_gesture_is_ignored() {
    let (mut c, g) = rig();
    let nodes = drawn(&c, &g);
    c.press(empty(), Button::Middle, false, &nodes);
    assert!(c.busy());
    c.press(title_of(&c, &g, PALETTE), Button::Primary, false, &nodes);
    c.moved(empty() + vec2(5.0, 5.0));
    c.release(empty() + vec2(5.0, 5.0), &nodes);
    assert_eq!(c.view.pan, vec2(5.0, 5.0));
    check_selected(&c, &[]);
    assert!(!c.busy());
    rejects("the second press taken", || check_selected(&c, &[PALETTE]));
}

/// The focus's scroll: a node out of view is panned into it, the least; one in view stays.
#[test]
fn canvas_gestures_mock_reveal_pans_the_least() {
    let mut c = Canvas::default();
    let within = Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 100.0));
    c.reveal(
        Rect::from_min_size(pos2(120.0, 10.0), vec2(20.0, 20.0)),
        within,
    );
    assert_eq!(c.view.pan, vec2(-40.0, 0.0));
    c.reveal(
        Rect::from_min_size(pos2(-30.0, -5.0), vec2(20.0, 20.0)),
        within,
    );
    assert_eq!(c.view.pan, vec2(-10.0, 5.0));
    let kept = c.view.pan;
    c.reveal(
        Rect::from_min_size(pos2(10.0, 10.0), vec2(20.0, 20.0)),
        within,
    );
    assert_eq!(c.view.pan, kept);
    // Larger than the view: its top-left edge is shown.
    c.reveal(
        Rect::from_min_size(pos2(-10.0, 0.0), vec2(200.0, 20.0)),
        within,
    );
    assert_eq!(c.view.pan, kept + vec2(10.0, 0.0));
    rejects("a node in view moved", || assert_eq!(kept, vec2(0.0, 0.0)));
}

/// A node removed from the graph loses its place and its selection; a new one gets its column's next place.
#[test]
fn canvas_gestures_mock_places_follow_the_graph() {
    use engine::contract::render_state::{NodeKind, Occupant};
    let (mut c, mut g) = rig();
    c.selection = selection(&[FADE]);
    g.remove(FADE).expect("a brightness can go");
    let stale = c.clone();
    c.place(&g);
    assert!(!c.positions.contains_key(&FADE));
    check_selected(&c, &[]);
    let added = g
        .add(NodeKind::Source, Occupant::Field("ftle".into()))
        .expect("a source");
    c.place(&g);
    assert_eq!(
        c.positions[&added],
        crate::stain::canvas::default_position(NodeKind::Source, 2)
    );
    assert!(c.positions.contains_key(&OUT));
    rejects("a removed node's place kept", || {
        assert!(!stale.positions.contains_key(&FADE))
    });
}

// --- Through the app, with egui's events -----------------------------------------------------------------------------

/// The mock app on the Stain page, one frame drawn.
fn stain_app() -> (App<MockSide>, Headless) {
    let mut app = mock_app();
    app.view.mode = Mode::Stain;
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    (app, h)
}

fn canvas_rect(h: &Headless) -> Rect {
    let ppp = h.pixels_per_point();
    Regions::new(&Layout::new(h.screen(), ppp), ppp, false).canvas
}

fn app_title(app: &App<MockSide>, h: &Headless, id: NodeId) -> Pos2 {
    let g = &app.snapshot().render.stain_graph;
    let r = app
        .stain
        .canvas
        .drawn(g, canvas_rect(h).min)
        .into_iter()
        .find(|d| d.id == id)
        .expect("drawn")
        .rect;
    pos2(r.min.x + 30.0, r.min.y + TITLE_H / 2.0)
}

fn button(pos: Pos2, button: PointerButton, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

/// Drags with `which` from `a` to `b`, a frame per event.
fn app_drag(app: &mut App<MockSide>, h: &mut Headless, a: Pos2, b: Pos2, which: PointerButton) {
    for events in [
        vec![Event::PointerMoved(a)],
        vec![button(a, which, true)],
        vec![Event::PointerMoved(a + (b - a) / 2.0)],
        vec![Event::PointerMoved(b)],
        vec![button(b, which, false)],
        vec![],
    ] {
        let _ = h.frame(app, events);
    }
}

#[test]
fn canvas_gestures_mock_through_the_app() {
    let (mut app, mut h) = stain_app();
    // Click a node: selected, its inspector open.
    let at = app_title(&app, &h, PALETTE);
    for events in crate::capture::click(at) {
        let _ = h.frame(&mut app, events);
    }
    check_selected(&app.stain.canvas, &[PALETTE]);
    let texts = frame_texts(&mut h, &mut app);
    assert!(
        texts.iter().any(|t| t == "Colour · outcome_state"),
        "{texts:?}"
    );
    // Drag it: it moves.
    let start = app.stain.canvas.positions[&PALETTE];
    app_drag(
        &mut app,
        &mut h,
        at,
        at + vec2(30.0, 20.0),
        PointerButton::Primary,
    );
    assert_eq!(
        app.stain.canvas.positions[&PALETTE] - start,
        vec2(30.0, 20.0)
    );
    // Middle-drag on empty canvas: a pan.
    // Right of the last column, below the toggle: empty canvas at any pan and zoom the test reaches.
    let blank = pos2(canvas_rect(&h).max.x - 40.0, canvas_rect(&h).min.y + 80.0);
    app_drag(
        &mut app,
        &mut h,
        blank,
        blank + vec2(-25.0, 10.0),
        PointerButton::Middle,
    );
    assert_eq!(app.stain.canvas.view.pan, vec2(-25.0, 10.0));
    // Space held, a primary drag: a pan too.
    let _ = h.frame(
        &mut app,
        crate::capture::key(Key::Space, Modifiers::NONE)[0].clone(),
    );
    app_drag(
        &mut app,
        &mut h,
        blank,
        blank + vec2(5.0, 5.0),
        PointerButton::Primary,
    );
    let _ = h.frame(
        &mut app,
        crate::capture::key(Key::Space, Modifiers::NONE)[1].clone(),
    );
    assert_eq!(app.stain.canvas.view.pan, vec2(-20.0, 15.0));
    // The wheel over the canvas: a zoom.
    let _ = h.frame(
        &mut app,
        vec![
            Event::PointerMoved(blank),
            Event::MouseWheel {
                unit: MouseWheelUnit::Point,
                delta: vec2(0.0, 50.0),
                phase: egui::TouchPhase::Move,
                modifiers: Modifiers::NONE,
            },
        ],
    );
    assert!((app.stain.canvas.view.zoom - (50.0 * ZOOM_PER_POINT).exp()).abs() < 1e-6);
    // A click on empty canvas deselects: the inspector shows the graph.
    for events in crate::capture::click(blank) {
        let _ = h.frame(&mut app, events);
    }
    check_selected(&app.stain.canvas, &[]);
    let texts = frame_texts(&mut h, &mut app);
    assert!(texts.iter().any(|t| t == "Graph"), "{texts:?}");
    rejects("a node still selected", || {
        check_selected(&app.stain.canvas, &[PALETTE])
    });
}

/// A press on the toggle, over the canvas, is the toggle's: no gesture starts.
#[test]
fn canvas_gestures_mock_the_toggle_takes_its_press() {
    let (mut app, mut h) = stain_app();
    super::support::click(&mut h, &mut app, "Pipeline WGSL");
    assert_eq!(app.stain.view, crate::stain::View::Pipeline);
    assert!(!app.stain.canvas.busy());
    check_selected(&app.stain.canvas, &[]);
    super::support::click(&mut h, &mut app, "Graph");
    assert_eq!(app.stain.view, crate::stain::View::Graph);
    rejects("the toggle unmoved", || {
        assert_eq!(app.stain.view, crate::stain::View::Pipeline)
    });
}

fn wheel(at: Pos2, unit: MouseWheelUnit, dy: f32) -> Vec<Event> {
    vec![
        Event::PointerMoved(at),
        Event::MouseWheel {
            unit,
            delta: vec2(0.0, dy),
            phase: egui::TouchPhase::Move,
            modifiers: Modifiers::NONE,
        },
    ]
}

/// The wheel by lines and by pages, as by points; off the canvas it zooms nothing.
#[test]
fn canvas_gestures_mock_wheel_units() {
    let (mut app, mut h) = stain_app();
    let rect = canvas_rect(&h);
    let blank = pos2(rect.max.x - 40.0, rect.min.y + 80.0);
    let zoom_after = |app: &mut App<MockSide>, h: &mut Headless, events: Vec<Event>| {
        app.stain.canvas.view = Default::default();
        let _ = h.frame(app, events);
        app.stain.canvas.view.zoom
    };
    let lines = zoom_after(&mut app, &mut h, wheel(blank, MouseWheelUnit::Line, 1.0));
    assert!(
        (lines - (LINE_POINTS * ZOOM_PER_POINT).exp()).abs() < 1e-6,
        "{lines}"
    );
    let pages = zoom_after(&mut app, &mut h, wheel(blank, MouseWheelUnit::Page, 0.1));
    let want = (0.1 * rect.height() * ZOOM_PER_POINT).exp();
    assert!((pages - want).abs() < 1e-5, "{pages} for {want}");
    // Over the drawer: no zoom.
    let drawer = pos2(rect.min.x - 100.0, rect.min.y + 300.0);
    let off = zoom_after(&mut app, &mut h, wheel(drawer, MouseWheelUnit::Point, 50.0));
    assert_eq!(off, 1.0);
    rejects("a zoom from the drawer", || assert!(off > 1.0));
}

/// A press off the canvas, or by another button, starts nothing: the selection stays.
#[test]
fn canvas_gestures_mock_only_its_presses() {
    let (mut app, mut h) = stain_app();
    app.stain.canvas.selection = [PALETTE].into();
    let rect = canvas_rect(&h);
    // On the drawer, below its rows.
    let drawer = pos2(rect.min.x - 30.0, rect.max.y - 20.0);
    for events in crate::capture::click(drawer) {
        let _ = h.frame(&mut app, events);
    }
    check_selected(&app.stain.canvas, &[PALETTE]);
    // The secondary button on another node.
    let at = app_title(&app, &h, OUTCOME);
    for pressed in [true, false] {
        let event = Event::PointerButton {
            pos: at,
            button: PointerButton::Secondary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        let _ = h.frame(&mut app, vec![Event::PointerMoved(at), event]);
    }
    check_selected(&app.stain.canvas, &[PALETTE]);
    assert!(!app.stain.canvas.busy());
    rejects("the secondary click taken", || {
        check_selected(&app.stain.canvas, &[OUTCOME])
    });
}

/// The inspector's heading: the graph's with nothing selected, the node's with one, the count with more.
#[test]
fn canvas_gestures_mock_the_inspector_follows_the_selection() {
    let (mut app, mut h) = stain_app();
    let heading = |app: &mut App<MockSide>, h: &mut Headless, ids: &[NodeId]| {
        app.stain.canvas.selection = selection(ids);
        frame_texts(h, app)
    };
    let none = heading(&mut app, &mut h, &[]);
    assert!(none.iter().any(|t| t == "6 nodes · 5 wires"), "{none:?}");
    let one = heading(&mut app, &mut h, &[FATE]);
    assert!(
        one.iter().any(|t| t == "Source · ctx.t_end_step"),
        "{one:?}"
    );
    assert!(one.iter().any(|t| t == "reads ctx.t_end_step → field"));
    let two = heading(&mut app, &mut h, &[FATE, PALETTE]);
    assert!(two.iter().any(|t| t == "2 nodes selected"), "{two:?}");
    assert!(!none.iter().any(|t| t.ends_with("nodes selected")));
    rejects("no selection read as a multi-selection", || {
        assert!(none.iter().any(|t| t.ends_with("nodes selected")))
    });
}
