//! QA tests for TASK-M6-29, the Stain node-graph editor on the mock, written from the requirements and their sources,
//! not from the implementation:
//!
//! - REQ-GUI-128 (render_gui_spec Part II §1; GUI_DESIGN_NOTES § "02 Stain"): Stain mode lays out the library drawer,
//!   collapsible, left; the graph canvas centre with its `Graph · Pipeline WGSL · Node WGSL` toggle; the preview and
//!   node inspector right; and the assembled code and a Problems pane (compile status, notes, auto-recompile) under
//!   the canvas. The screenshot against 02_stain.png is the gui reviewer's and `cargo xtask screenshot`'s.
//! - REQ-GUI-131 (Part II §7): click-select opens the node's inspector; a click on empty canvas deselects, the
//!   inspector showing the graph; a marquee multi-selects; dragging a node moves it with its wires following, and a
//!   multi-selection moves together; space-drag and middle-drag pan; scroll zooms. Driven by scripted egui pointer
//!   events through the app, read back from where the nodes and wires are drawn.
//! - REQ-GUI-175: an edit to the graph is a `SetField` (gui_state_contract §2: `RenderField::StainGraph`, undoable,
//!   logged "SetField StainGraph …"), and the mock's preview changes with it; the preview is square for a square
//!   viewport (§G1: "Every preview of the slice keeps the viewport's aspect ratio"). The canvas's places, pan, zoom and
//!   selection are the editor's own (§2), so no gesture is a `SetField`.
//! - REQ-GUI-169, re-checked for this screen (R-409; the task's acceptance line): in Stain mode Tab and Shift+Tab reach
//!   the top bar, the library drawer, the graph canvas, the preview and node inspector, and the code and Problems
//!   pane in that order (the footer last, R-405); Enter reaches a library row, a node, an inspector field and the
//!   toggle; every landing is navigation mode; Enter on a button acts as its click, on an element starts interaction;
//!   Esc returns to navigation on the same element, then goes up a scope; arrows adjust a focused value by its base
//!   step, 1/100 of its range (R-404's `Bounded`), ×10 with Shift and ×0.1 with Alt; the ring is orange navigating and
//!   `hyperlink_color` interacting, one ring; the breadcrumb names the path.
//!
//! gui takes no dependency on validation (R-187), so each check's control (R-176) runs in its test, through [`rejects`]
//! or by the same check where it must come out the other way. Nothing here allocates by a loop it does not bound: every
//! walk and scan has a fixed cap.
// The file name `qa_TASK-M6-29` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::collections::BTreeSet;
use std::sync::Arc;

use eframe::egui::{
    self, pos2, vec2, Color32, Event, Key, Modifiers, MouseWheelUnit, PointerButton, Pos2, Rect,
    Shape, TouchPhase, Vec2,
};
use engine::contract::render_state::{NodeId, NodeKind, Occupant, StainGraph};
use engine::contract::view_ui::{KeyMode, Mode};
use gui::app::App;
use gui::capture::{render_frame, Shot, PIXELS_PER_POINT, SIZE};
use gui::headless::{Headless, Name};
use gui::layout::Layout;
use gui::mock::canvas::MockCanvas;
use gui::mock::MockEngine;
use gui::side::{EngineSide, MockSide};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const NONE: Modifiers = Modifiers::NONE;
const SHIFT: Modifiers = Modifiers::SHIFT;
const ALT: Modifiers = Modifiers::ALT;
const NAV: KeyMode = KeyMode::Navigation;
const INTERACT: KeyMode = KeyMode::Interaction;

/// The Stain page's big scopes in Tab order, as the task file names them, the footer last (R-405).
const BIG: [&str; 6] = [
    "top bar",
    "Library drawer",
    "Graph canvas",
    "Preview & node inspector",
    "Code & Problems",
    "Footer",
];
/// The toggle's three views (Part II §1).
const TOGGLE: [&str; 3] = ["Graph", "Pipeline WGSL", "Node WGSL"];
/// A cap on every walk along siblings: more than the page holds.
const WALK: usize = 64;

/// Asserts that `check` panics: the check can fail (R-176's control, run in the test).
fn rejects(what: &str, check: impl FnOnce()) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check));
    assert!(
        outcome.is_err(),
        "the check passed on {what}, which it must reject"
    );
}

fn same(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

fn layout() -> Layout {
    Layout::new(
        Headless::new(SIZE, PIXELS_PER_POINT).screen(),
        PIXELS_PER_POINT,
    )
}

/// The app on the frozen mock, with its canvas or none, and its headless context.
struct Rig {
    app: App<MockSide>,
    h: Headless,
    canvas: Option<Arc<MockCanvas>>,
}

impl Rig {
    fn new(canvas: Option<Arc<MockCanvas>>) -> Self {
        Self::sized(canvas, SIZE)
    }

    /// The app in a window `size` pixels in size.
    fn sized(canvas: Option<Arc<MockCanvas>>, size: [u32; 2]) -> Self {
        let mut rig = Self {
            app: App::new(MockSide::new(MockEngine::frozen(), canvas.clone()), FORMAT),
            h: Headless::new(size, PIXELS_PER_POINT),
            canvas,
        };
        rig.idle(2);
        rig
    }

    /// On the Stain page, reached by clicking the top bar's "Stain", as a user does.
    fn stain(canvas: Option<Arc<MockCanvas>>) -> Self {
        Self::stain_sized(canvas, SIZE)
    }

    /// [`Rig::stain`] in a window `size` pixels in size.
    fn stain_sized(canvas: Option<Arc<MockCanvas>>, size: [u32; 2]) -> Self {
        let mut rig = Self::sized(canvas, size);
        let names = rig.names();
        let at = centre_of(&names, "Stain").expect("the top bar's Stain switch drawn");
        rig.click(at, PointerButton::Primary);
        assert_eq!(
            rig.app.view.mode,
            Mode::Stain,
            "the Stain switch did not open Stain mode"
        );
        rig.idle(2);
        rig
    }

    fn frame(&mut self, events: Vec<Event>) -> egui::FullOutput {
        self.h.frame(&mut self.app, events)
    }

    fn idle(&mut self, n: usize) {
        for _ in 0..n {
            let _ = self.frame(Vec::new());
        }
    }

    fn names(&mut self) -> Vec<Name> {
        let out = self.frame(Vec::new());
        self.h.names(&out)
    }

    /// The shapes of one idle frame, flattened.
    fn shapes(&mut self) -> Vec<Shape> {
        let out = self.frame(Vec::new());
        let mut all = Vec::new();
        for c in out.shapes {
            flatten(c.shape, &mut all);
        }
        all
    }

    fn key(&mut self, key: Key, modifiers: Modifiers) {
        for pressed in [true, false] {
            let _ = self.frame(vec![key_event(key, pressed, modifiers)]);
        }
        self.idle(1);
    }

    fn button(&self, pos: Pos2, button: PointerButton, pressed: bool) -> Event {
        Event::PointerButton {
            pos,
            button,
            pressed,
            modifiers: NONE,
        }
    }

    fn click(&mut self, pos: Pos2, button: PointerButton) {
        let _ = self.frame(vec![Event::PointerMoved(pos)]);
        let press = self.button(pos, button, true);
        let _ = self.frame(vec![press]);
        let release = self.button(pos, button, false);
        let _ = self.frame(vec![release]);
        self.idle(1);
    }

    /// A drag with `button` from `from` to `to`, in `steps` moves, each its own frame, then the release.
    fn drag(&mut self, from: Pos2, to: Pos2, button: PointerButton, steps: usize) {
        let _ = self.frame(vec![Event::PointerMoved(from)]);
        let press = self.button(from, button, true);
        let _ = self.frame(vec![press]);
        for k in 1..=steps {
            let p = from + (to - from) * (k as f32 / steps as f32);
            let _ = self.frame(vec![Event::PointerMoved(p)]);
        }
        let release = self.button(to, button, false);
        let _ = self.frame(vec![release]);
        self.idle(1);
    }

    fn scroll(&mut self, at: Pos2, unit: MouseWheelUnit, dy: f32) {
        let _ = self.frame(vec![Event::PointerMoved(at)]);
        let _ = self.frame(vec![Event::MouseWheel {
            unit,
            delta: vec2(0.0, dy),
            phase: TouchPhase::Move,
            modifiers: NONE,
        }]);
        self.idle(1);
    }

    fn graph(&self) -> StainGraph {
        self.app.snapshot().render.stain_graph.clone()
    }

    fn undo_depth(&self) -> u32 {
        self.app.snapshot().history.undo_depth
    }

    fn labels(&self) -> Vec<String> {
        self.app
            .keyboard
            .tree(self.app.view.mode)
            .labels(&self.app.view.focus.path)
            .into_iter()
            .map(str::to_owned)
            .collect()
    }

    fn last_label(&self) -> String {
        self.labels().last().cloned().unwrap_or_default()
    }

    /// The place the focused scope was drawn at, in points.
    fn focus_rect(&self) -> Rect {
        let id = self.app.view.focus.path.last().expect("a focus");
        self.app
            .keyboard
            .place_of(id)
            .expect("the focused scope drawn")
            .rect
    }

    /// Tabs from no focus to big scope `label`; its place.
    fn tab_to(&mut self, label: &str) -> Rect {
        self.app.view.focus.path.clear();
        for _ in 0..BIG.len() + 1 {
            self.key(Key::Tab, NONE);
            if same(&self.last_label(), label) {
                return self.focus_rect();
            }
        }
        panic!("Tab never reaches `{label}`: at {:?}", self.labels());
    }

    /// Enter into big scope `label`, then → along its elements, at most [`WALK`], collecting their labels until the
    /// walk comes back to its first.
    fn walk(&mut self, label: &str) -> Vec<String> {
        self.tab_to(label);
        self.key(Key::Enter, NONE);
        let first = self.app.view.focus.path.clone();
        let mut seen = vec![self.last_label()];
        for _ in 0..WALK {
            self.key(Key::ArrowRight, NONE);
            if self.app.view.focus.path == first {
                break;
            }
            let l = self.last_label();
            if seen.last() == Some(&l) {
                break;
            }
            seen.push(l);
        }
        seen
    }

    /// Enter into big scope `scope`, then → to the element `label` (case aside), within [`WALK`].
    fn focus_element(&mut self, scope: &str, pred: impl Fn(&str) -> bool) {
        self.tab_to(scope);
        self.key(Key::Enter, NONE);
        for _ in 0..WALK {
            if pred(&self.last_label()) {
                return;
            }
            self.key(Key::ArrowRight, NONE);
        }
        panic!("no such element in `{scope}`: at {:?}", self.labels());
    }

    fn render(&mut self) -> Shot {
        let canvas = self.canvas.clone().expect("a rig with a canvas");
        render_frame(&canvas, &mut self.app, &mut self.h)
    }
}

fn flatten(shape: Shape, out: &mut Vec<Shape>) {
    match shape {
        Shape::Vec(v) => {
            for s in v {
                flatten(s, out);
            }
        }
        s => out.push(s),
    }
}

fn key_event(key: Key, pressed: bool, modifiers: Modifiers) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers,
    }
}

fn rect_px(r: [f64; 4]) -> Rect {
    let p = f64::from(PIXELS_PER_POINT);
    Rect::from_min_max(
        pos2((r[0] / p) as f32, (r[1] / p) as f32),
        pos2((r[2] / p) as f32, (r[3] / p) as f32),
    )
}

/// The centre of the first name that reads exactly `text`, in points.
fn centre_of(names: &[Name], text: &str) -> Option<Pos2> {
    names
        .iter()
        .filter(|n| n.name == text)
        .find_map(|n| n.rect)
        .map(|r| rect_px(r).center())
}

/// The names drawn wholly inside `rect` (points).
fn names_in(names: &[Name], rect: Rect) -> Vec<String> {
    names
        .iter()
        .filter(|n| {
            n.rect
                .is_some_and(|r| rect.expand(0.5).contains_rect(rect_px(r)))
        })
        .map(|n| n.name.clone())
        .collect()
}

fn gpu() -> Arc<MockCanvas> {
    Arc::new(MockCanvas::new().expect("a GPU adapter for the mock's canvas"))
}

// --- REQ-GUI-128: the four surfaces --------------------------------------------------------------------------------

/// Where the big scopes are drawn, by Tab, in order after the top bar.
struct Surfaces {
    library: Rect,
    canvas: Rect,
    right: Rect,
    code: Rect,
}

fn surfaces(rig: &mut Rig) -> Surfaces {
    Surfaces {
        library: rig.tab_to("Library drawer"),
        canvas: rig.tab_to("Graph canvas"),
        right: rig.tab_to("Preview & node inspector"),
        code: rig.tab_to("Code & Problems"),
    }
}

/// Part II §1's layout: the drawer left of the canvas, the preview and inspector right of it, the code and Problems
/// under it; the toggle's three views in the canvas, along its top, in order; the preview in the right column; the
/// Problems pane with its compile status and auto-recompile under the canvas.
fn check_layout(s: &Surfaces, names: &[Name]) {
    const EDGE: f32 = 2.0; // points: a border between two panes
    assert!(
        s.library.max.x <= s.canvas.min.x + EDGE && s.library.min.x < s.canvas.min.x,
        "the drawer {:?} is not left of the canvas {:?}",
        s.library,
        s.canvas
    );
    assert!(
        s.canvas.max.x <= s.right.min.x + EDGE,
        "the inspector {:?} is not right of the canvas {:?}",
        s.right,
        s.canvas
    );
    assert!(
        s.code.min.y >= s.canvas.max.y - EDGE
            && s.code.min.x >= s.canvas.min.x - EDGE
            && s.code.max.x <= s.canvas.max.x + EDGE,
        "the code and Problems {:?} are not under the canvas {:?}",
        s.code,
        s.canvas
    );
    // The toggle, in the canvas's top, in its order left to right.
    let xs: Vec<f32> = TOGGLE
        .iter()
        .map(|t| {
            let at = centre_of(names, t).unwrap_or_else(|| panic!("no `{t}` drawn"));
            assert!(
                s.canvas.contains(at) && at.y < s.canvas.min.y + s.canvas.height() * 0.25,
                "`{t}` at {at:?} is not on the canvas's top {:?}",
                s.canvas
            );
            at.x
        })
        .collect();
    assert!(
        xs.windows(2).all(|w| w[0] < w[1]),
        "the toggle is not Graph · Pipeline WGSL · Node WGSL left to right: {xs:?}"
    );
    let right = names_in(names, s.right);
    assert!(
        right.iter().any(|n| same(n, "Preview")),
        "no preview in the right column: {right:?}"
    );
    let under = names_in(names, s.code);
    for want in ["problems", "auto-recompile", "compiled"] {
        assert!(
            under.iter().any(|n| n.to_lowercase().contains(want)),
            "no `{want}` under the canvas: {under:?}"
        );
    }
}

#[test]
fn qa_mock_stain_four_surfaces_laid_out_as_part_ii_1() {
    let mut rig = Rig::stain(None);
    let s = surfaces(&mut rig);
    let names = rig.names();
    check_layout(&s, &names);
    // Control: the drawer and the inspector swapped must fail.
    rejects("the drawer right of the canvas", || {
        check_layout(
            &Surfaces {
                library: s.right,
                canvas: s.canvas,
                right: s.library,
                code: s.code,
            },
            &names,
        )
    });
    // Control: the Explore page has no toggle, no Problems.
    let mut explore = Rig::new(None);
    let explore_names = explore.names();
    rejects("the Explore page", || check_layout(&s, &explore_names));
}

#[test]
fn qa_mock_stain_library_drawer_collapses_and_the_canvas_takes_its_width() {
    let mut rig = Rig::stain(None);
    let open = surfaces(&mut rig);
    // The drawer's collapse, reached by the keys, and Enter as its click (R-409: Enter on a button is its click).
    rig.focus_element("Library drawer", |l| l.to_lowercase().contains("collapse"));
    rig.key(Key::Enter, NONE);
    rig.idle(2);
    let shut = surfaces(&mut rig);
    let check = |a: &Surfaces, b: &Surfaces| {
        assert!(
            b.library.width() < a.library.width() / 2.0,
            "the drawer did not collapse: {:?} → {:?}",
            a.library,
            b.library
        );
        assert!(
            b.canvas.width() > a.canvas.width() && (b.canvas.max.x - a.canvas.max.x).abs() < 1.0,
            "the canvas did not take the drawer's width: {:?} → {:?}",
            a.canvas,
            b.canvas
        );
    };
    check(&open, &shut);
    rejects("the drawer left open", || check(&open, &open));
    // And it opens again.
    rig.focus_element("Library drawer", |l| {
        l.to_lowercase().contains("collapse") || l.to_lowercase().contains("expand")
    });
    rig.key(Key::Enter, NONE);
    rig.idle(2);
    let again = surfaces(&mut rig);
    assert_eq!(
        (again.library, again.canvas),
        (open.library, open.canvas),
        "the drawer did not open back to its place"
    );
}

// --- REQ-GUI-131: the canvas's gestures ------------------------------------------------------------------------------

/// Each node's drawn rect, by its keyboard place (the canvas places each node it draws), in id order.
fn node_rects(rig: &mut Rig) -> Vec<(NodeId, Rect)> {
    rig.idle(1);
    let g = rig.graph();
    g.nodes()
        .filter_map(|n| {
            let scope = gui::stain::canvas::node_scope(n.id)?;
            Some((n.id, rig.app.keyboard.place_of(scope)?.rect))
        })
        .collect()
}

fn rect_of(rects: &[(NodeId, Rect)], id: NodeId) -> Rect {
    rects
        .iter()
        .find(|(i, _)| *i == id)
        .map(|(_, r)| *r)
        .expect("the node drawn")
}

fn find(g: &StainGraph, kind: NodeKind) -> Vec<NodeId> {
    g.nodes().filter(|n| n.kind == kind).map(|n| n.id).collect()
}

/// A point on node `r`'s title bar: its top, inside, clear of its ports.
fn title_point(r: Rect) -> Pos2 {
    pos2(r.center().x, r.min.y + 8.0)
}

/// A point of the canvas at least 24 points from every node, the toggle and the canvas's edges; scanned on a fixed
/// grid.
fn empty_point(canvas: Rect, rects: &[(NodeId, Rect)]) -> Pos2 {
    for j in 0..40 {
        for i in 0..40 {
            let p = pos2(
                canvas.min.x + canvas.width() * (i as f32 + 0.5) / 40.0,
                canvas.min.y + canvas.height() * (j as f32 + 0.5) / 40.0,
            );
            if canvas.shrink(48.0).contains(p)
                && rects.iter().all(|(_, r)| !r.expand(24.0).contains(p))
            {
                return p;
            }
        }
    }
    panic!("no empty canvas found");
}

fn selection(rig: &Rig) -> BTreeSet<NodeId> {
    rig.app.stain.canvas.selection.clone()
}

/// The names in the inspector's column below the preview.
fn inspector_names(rig: &mut Rig, right: Rect) -> Vec<String> {
    let names = rig.names();
    names_in(&names, right)
}

#[test]
fn qa_canvas_gestures_mock_click_selects_opening_the_inspector_and_empty_click_deselects() {
    let mut rig = Rig::stain(None);
    let canvas = rig.tab_to("Graph canvas");
    let right = rig.tab_to("Preview & node inspector");
    rig.app.view.focus.path.clear();
    let g = rig.graph();
    let rects = node_rects(&mut rig);
    let colour = find(&g, NodeKind::Colour)[0];
    let occupant = match &g.node(colour).unwrap().occupant {
        Occupant::Builtin(id) => id.clone(),
        o => panic!("the mock's colour node holds {o:?}"),
    };
    let graph_view = inspector_names(&mut rig, right);
    assert!(
        !graph_view.iter().any(|n| n.contains(&occupant)),
        "with nothing selected the inspector already shows the colour node: {graph_view:?}"
    );
    rig.click(title_point(rect_of(&rects, colour)), PointerButton::Primary);
    let check_selected = |sel: &BTreeSet<NodeId>, shown: &[String]| {
        assert_eq!(
            sel,
            &BTreeSet::from([colour]),
            "the click did not select the node alone"
        );
        assert!(
            shown.iter().any(|n| n.contains(&occupant)),
            "the inspector does not show the clicked node `{occupant}`: {shown:?}"
        );
    };
    let shown = inspector_names(&mut rig, right);
    check_selected(&selection(&rig), &shown);
    rejects("the graph's inspector", || {
        check_selected(&selection(&rig), &graph_view)
    });
    // A click on empty canvas deselects; the inspector shows the graph (Part II §7: "graph/global").
    rig.click(empty_point(canvas, &rects), PointerButton::Primary);
    let after = inspector_names(&mut rig, right);
    let check_deselected = |sel: &BTreeSet<NodeId>, shown: &[String]| {
        assert!(sel.is_empty(), "the empty click left {sel:?} selected");
        assert!(
            shown.iter().any(|n| n.to_lowercase().contains("graph"))
                && !shown.iter().any(|n| n.contains(&occupant)),
            "the inspector does not show the graph: {shown:?}"
        );
    };
    check_deselected(&selection(&rig), &after);
    rejects("the node's inspector", || {
        check_deselected(&BTreeSet::new(), &shown)
    });
    // A click is no graph edit (§2: the selection is the editor's own).
    assert_eq!(rig.graph(), g, "a click changed the graph");
    assert_eq!(rig.undo_depth(), 0, "a click entered the history");
}

#[test]
fn qa_canvas_gestures_mock_marquee_selects_exactly_the_nodes_it_covers() {
    let mut rig = Rig::stain(None);
    let canvas = rig.tab_to("Graph canvas");
    rig.app.view.focus.path.clear();
    let g = rig.graph();
    let rects = node_rects(&mut rig);
    let sources = find(&g, NodeKind::Source);
    assert!(
        sources.len() >= 2,
        "the mock's graph has two sources to marquee"
    );
    let (a, b) = (rect_of(&rects, sources[0]), rect_of(&rects, sources[1]));
    let cover = a.union(b).expand(10.0);
    // The marquee covers both sources wholly and touches no other node, and starts on empty canvas.
    let others: Vec<NodeId> = rects
        .iter()
        .filter(|(_, r)| r.intersects(cover))
        .map(|(id, _)| *id)
        .collect();
    assert_eq!(
        BTreeSet::from_iter(others),
        BTreeSet::from([sources[0], sources[1]]),
        "the test's marquee must cover the two sources only"
    );
    assert!(
        canvas.contains(cover.min),
        "the marquee starts off the canvas"
    );
    rig.drag(cover.min, cover.max, PointerButton::Primary, 6);
    let check = |sel: &BTreeSet<NodeId>| {
        assert_eq!(
            sel,
            &BTreeSet::from([sources[0], sources[1]]),
            "the marquee did not select the two nodes it covers"
        );
    };
    check(&selection(&rig));
    rejects("one node selected", || check(&BTreeSet::from([sources[0]])));
    // A marquee moves no node.
    assert_eq!(node_rects(&mut rig), rects, "the marquee moved a node");
    // A marquee over nothing selects nothing.
    let empty = empty_point(canvas, &rects);
    rig.drag(empty, empty + vec2(12.0, 12.0), PointerButton::Primary, 3);
    assert!(
        selection(&rig).is_empty(),
        "a marquee over no node left a selection"
    );
}

/// The wires as drawn: each Bézier's two ends.
fn wire_ends(rig: &mut Rig) -> Vec<(Pos2, Pos2)> {
    rig.shapes()
        .into_iter()
        .filter_map(|s| match s {
            Shape::CubicBezier(b) => Some((b.points[0], b.points[3])),
            _ => None,
        })
        .collect()
}

fn near(a: Pos2, b: Pos2) -> bool {
    (a - b).length() < 0.5
}

/// The wires after a move of the nodes `moved` (their rects before, `before`) by `delta`: every end on a moved node's
/// edge moved by `delta`, every other end where it was, and as many wires as before.
fn check_wires_follow(
    before: &[(Pos2, Pos2)],
    after: &[(Pos2, Pos2)],
    moved: &[Rect],
    delta: Vec2,
) {
    assert_eq!(
        before.len(),
        after.len(),
        "the wires drawn changed in number"
    );
    let on = |p: Pos2| moved.iter().any(|r| r.expand(1.0).contains(p));
    let mut followed = 0;
    for &(a, b) in before {
        let want = (
            if on(a) { a + delta } else { a },
            if on(b) { b + delta } else { b },
        );
        followed += usize::from(on(a)) + usize::from(on(b));
        assert!(
            after
                .iter()
                .any(|&(x, y)| near(x, want.0) && near(y, want.1)),
            "the wire {a:?} → {b:?} did not follow: no wire {want:?} in {after:?}"
        );
    }
    assert!(followed > 0, "no wire ends on the moved nodes");
}

#[test]
fn qa_canvas_gestures_mock_dragging_a_node_moves_it_and_its_wires_follow() {
    let mut rig = Rig::stain(None);
    rig.idle(1);
    let g = rig.graph();
    let rects = node_rects(&mut rig);
    let wires = wire_ends(&mut rig);
    let colour = find(&g, NodeKind::Colour)[0];
    let r = rect_of(&rects, colour);
    let delta = vec2(60.0, 40.0);
    let from = title_point(r);
    rig.drag(from, from + delta, PointerButton::Primary, 8);
    let after = node_rects(&mut rig);
    let check_moved = |after: &[(NodeId, Rect)]| {
        for (id, before) in &rects {
            let now = rect_of(after, *id);
            let want = if *id == colour {
                before.translate(delta)
            } else {
                *before
            };
            assert!(
                (now.min - want.min).length() < 0.5 && (now.size() - want.size()).length() < 0.5,
                "node {id:?} is at {now:?}, not {want:?}"
            );
        }
    };
    check_moved(&after);
    rejects("the nodes unmoved", || check_moved(&rects));
    let wires_after = wire_ends(&mut rig);
    check_wires_follow(&wires, &wires_after, &[r], delta);
    rejects("the wires left behind", || {
        check_wires_follow(&wires, &wires, &[r], delta)
    });
    // A move is no graph edit (§2: a node's place is the editor's own).
    assert_eq!(rig.graph(), g, "a drag changed the graph");
    assert_eq!(rig.undo_depth(), 0, "a drag entered the history");
}

#[test]
fn qa_canvas_gestures_mock_a_multi_selection_moves_together() {
    let mut rig = Rig::stain(None);
    let canvas = rig.tab_to("Graph canvas");
    rig.app.view.focus.path.clear();
    let g = rig.graph();
    let rects = node_rects(&mut rig);
    let sources = find(&g, NodeKind::Source);
    let cover = rect_of(&rects, sources[0])
        .union(rect_of(&rects, sources[1]))
        .expand(10.0);
    assert!(canvas.contains(cover.min));
    rig.drag(cover.min, cover.max, PointerButton::Primary, 6);
    assert_eq!(
        selection(&rig).len(),
        2,
        "the marquee did not select both sources"
    );
    let wires = wire_ends(&mut rig);
    let delta = vec2(50.0, -30.0);
    let from = title_point(rect_of(&rects, sources[1]));
    rig.drag(from, from + delta, PointerButton::Primary, 8);
    let after = node_rects(&mut rig);
    let check = |after: &[(NodeId, Rect)]| {
        for (id, before) in &rects {
            let want = if sources[..2].contains(id) {
                before.translate(delta)
            } else {
                *before
            };
            let now = rect_of(after, *id);
            assert!(
                (now.min - want.min).length() < 0.5,
                "node {id:?} at {now:?}, not {want:?}"
            );
        }
    };
    check(&after);
    rejects("only the dragged node moved", || {
        let mut one = rects.clone();
        for (id, r) in &mut one {
            if *id == sources[1] {
                *r = r.translate(delta);
            }
        }
        check(&one)
    });
    let moved = [rect_of(&rects, sources[0]), rect_of(&rects, sources[1])];
    check_wires_follow(&wires, &wire_ends(&mut rig), &moved, delta);
    assert_eq!(
        selection(&rig).len(),
        2,
        "the drag dropped the multi-selection"
    );
}

/// Every node drawn `delta` from where it was, the same size.
/// Whether a node's place is its whole rect: a place is clipped to the canvas, so one touching its edges may be cut.
fn whole(canvas: Rect, r: Rect) -> bool {
    canvas.shrink(1.0).contains_rect(r)
}

/// The pairs of a node's rect before and after, for the nodes drawn whole in both.
fn whole_pairs(
    canvas: Rect,
    before: &[(NodeId, Rect)],
    after: &[(NodeId, Rect)],
) -> Vec<(NodeId, Rect, Rect)> {
    assert_eq!(
        before.len(),
        after.len(),
        "the nodes drawn changed in number"
    );
    let pairs: Vec<(NodeId, Rect, Rect)> = before
        .iter()
        .zip(after)
        .filter(|((_, b), (_, a))| whole(canvas, *b) && whole(canvas, *a))
        .map(|((id, b), (_, a))| (*id, *b, *a))
        .collect();
    assert!(
        pairs.len() >= 3,
        "fewer than three nodes stayed whole in view: {pairs:?}"
    );
    pairs
}

/// Every node drawn `delta` from where it was, the same size (among those drawn whole before and after).
fn check_panned(canvas: Rect, before: &[(NodeId, Rect)], after: &[(NodeId, Rect)], delta: Vec2) {
    for (id, b, a) in whole_pairs(canvas, before, after) {
        assert!(
            (a.min - (b.min + delta)).length() < 0.5 && (a.size() - b.size()).length() < 0.5,
            "node {id:?} at {a:?}, not panned {delta:?} from {b:?}"
        );
    }
}

#[test]
fn qa_canvas_gestures_mock_middle_drag_pans() {
    let mut rig = Rig::stain(None);
    let canvas = rig.tab_to("Graph canvas");
    rig.app.view.focus.path.clear();
    let g = rig.graph();
    let rects = node_rects(&mut rig);
    // Started on a node: a middle drag pans the whole canvas, it does not move the node or select it.
    let colour = find(&g, NodeKind::Colour)[0];
    let from = title_point(rect_of(&rects, colour));
    let delta = vec2(-70.0, 45.0);
    rig.drag(from, from + delta, PointerButton::Middle, 8);
    let after = node_rects(&mut rig);
    check_panned(canvas, &rects, &after, delta);
    rejects("no pan", || check_panned(canvas, &rects, &rects, delta));
    assert!(
        selection(&rig).is_empty(),
        "a middle drag selected {:?}",
        selection(&rig)
    );
    assert_eq!(
        (rig.graph(), rig.undo_depth()),
        (g, 0),
        "a pan is no graph edit"
    );
}

#[test]
fn qa_canvas_gestures_mock_space_drag_pans() {
    let mut rig = Rig::stain(None);
    let canvas = rig.tab_to("Graph canvas");
    rig.app.view.focus.path.clear();
    let g = rig.graph();
    let rects = node_rects(&mut rig);
    let colour = find(&g, NodeKind::Colour)[0];
    let from = title_point(rect_of(&rects, colour));
    let delta = vec2(40.0, 55.0);
    // Space held through a primary drag that starts on a node.
    let _ = rig.frame(vec![key_event(Key::Space, true, NONE)]);
    let _ = rig.frame(vec![Event::PointerMoved(from)]);
    let press = rig.button(from, PointerButton::Primary, true);
    let _ = rig.frame(vec![press]);
    for k in 1..=8 {
        let _ = rig.frame(vec![Event::PointerMoved(from + delta * (k as f32 / 8.0))]);
    }
    let release = rig.button(from + delta, PointerButton::Primary, false);
    let _ = rig.frame(vec![release]);
    let _ = rig.frame(vec![key_event(Key::Space, false, NONE)]);
    rig.idle(1);
    let after = node_rects(&mut rig);
    check_panned(canvas, &rects, &after, delta);
    assert_eq!(rig.graph(), g, "a pan is no graph edit");
    // Control: the same drag without Space moves that node alone.
    let mut plain = Rig::stain(None);
    plain.drag(from, from + delta, PointerButton::Primary, 8);
    let moved = node_rects(&mut plain);
    rejects("a plain drag", || {
        check_panned(canvas, &rects, &moved, delta)
    });
}

#[test]
fn qa_canvas_gestures_mock_scroll_zooms() {
    let mut rig = Rig::stain(None);
    let canvas = rig.tab_to("Graph canvas");
    rig.app.view.focus.path.clear();
    let rects = node_rects(&mut rig);
    let g = rig.graph();
    let at = empty_point(canvas, &rects);
    let scale = |before: &[(NodeId, Rect)], after: &[(NodeId, Rect)]| -> f32 {
        let ratios: Vec<f32> = whole_pairs(canvas, before, after)
            .into_iter()
            .map(|(_, b, a)| a.width() / b.width())
            .collect();
        assert!(
            ratios.windows(2).all(|w| (w[0] - w[1]).abs() < 1e-3),
            "the nodes did not scale together: {ratios:?}"
        );
        ratios[0]
    };
    rig.scroll(at, MouseWheelUnit::Point, 60.0);
    let up = node_rects(&mut rig);
    let s_up = scale(&rects, &up);
    rig.scroll(at, MouseWheelUnit::Point, -120.0);
    let down = node_rects(&mut rig);
    let s_down = scale(&up, &down);
    let check = |s_up: f32, s_down: f32| {
        assert!((s_up - 1.0).abs() > 1e-3, "a scroll did not zoom: {s_up}");
        assert!(
            (s_up > 1.0) != (s_down > 1.0) && (s_down - 1.0).abs() > 1e-3,
            "scrolling back did not zoom the other way: {s_up}, {s_down}"
        );
    };
    check(s_up, s_down);
    rejects("no zoom", || check(1.0, 1.0));
    // A line-unit wheel (a mouse's notches) zooms too.
    let before = node_rects(&mut rig);
    rig.scroll(at, MouseWheelUnit::Line, 1.0);
    let lined = node_rects(&mut rig);
    assert!(
        (scale(&before, &lined) - 1.0).abs() > 1e-3,
        "a line scroll did not zoom"
    );
    assert_eq!(
        (rig.graph(), rig.undo_depth()),
        (g, 0),
        "a zoom is no graph edit"
    );
}

// --- REQ-GUI-175: an edit is a SetField; the preview changes and is square ---------------------------------------------

/// The pixel box where `a` and `b` differ, in pixels, `[x0, y0, x1, y1)`, with the count of pixels differing.
fn diff_box(a: &Shot, b: &Shot, within: Option<[usize; 4]>) -> Option<([usize; 4], usize)> {
    let w = a.size[0] as usize;
    let [x0, y0, x1, y1] = within.unwrap_or([0, 0, w, a.size[1] as usize]);
    let mut bx = [usize::MAX, usize::MAX, 0, 0];
    let mut n = 0;
    for y in y0..y1 {
        for x in x0..x1 {
            let i = (y * w + x) * 4;
            if a.rgba[i..i + 4] != b.rgba[i..i + 4] {
                n += 1;
                bx = [
                    bx[0].min(x),
                    bx[1].min(y),
                    bx[2].max(x + 1),
                    bx[3].max(y + 1),
                ];
            }
        }
    }
    (n > 0).then_some((bx, n))
}

/// The pixels the preview is drawn in: where the app with the mock's canvas and the same app with none differ in the
/// right column (the preview is the only thing the canvas draws on the Stain page).
fn preview_box(canvas: &Arc<MockCanvas>) -> [usize; 4] {
    let mut with = Rig::stain(Some(canvas.clone()));
    let right = with.tab_to("Preview & node inspector");
    with.app.view.focus.path.clear();
    let mut without = Rig::stain(None);
    without.canvas = Some(canvas.clone());
    without.app.view.focus.path.clear();
    let (a, b) = (with.render(), without.render());
    let p = PIXELS_PER_POINT;
    let col = [
        (right.min.x * p) as usize,
        (right.min.y * p) as usize,
        ((right.max.x * p) as usize).min(SIZE[0] as usize),
        ((right.max.y * p) as usize).min(SIZE[1] as usize),
    ];
    diff_box(&a, &b, Some(col))
        .expect("no preview drawn in the right column")
        .0
}

/// The preview `w` × `h` keeps the viewport's aspect, to `slack` (its edges fall on whole pixels).
fn check_aspect(w: f32, h: f32, viewport: Vec2, slack: f32) {
    let want_h = w * viewport.y / viewport.x;
    assert!(
        (h - want_h).abs() <= slack,
        "the preview is {w} × {h}, not the viewport's aspect {viewport:?}"
    );
}

#[test]
fn qa_mock_stain_preview_keeps_the_viewports_aspect() {
    // On the GPU, at the capture's size: the pixels the preview is drawn in keep the figure's aspect, and they are
    // the rect the editor records for its preview (read below at other sizes).
    let canvas = gpu();
    let bx = preview_box(&canvas);
    let (w, h) = ((bx[2] - bx[0]) as f32, (bx[3] - bx[1]) as f32);
    let figure = layout().figure;
    assert!(w > 32.0, "the preview is {bx:?}: too small to read");
    check_aspect(w, h, figure.size(), 1.0);
    rejects("a 2:1 preview", || {
        check_aspect(200.0, 100.0, figure.size(), 1.0)
    });
    rejects("a stretched viewport", || {
        check_aspect(w, h, vec2(2.0, 1.0), 1.0)
    });
    let mut rig = Rig::stain(None);
    rig.idle(1);
    let recorded = rig.app.stain.preview.expect("the preview's rect recorded");
    let p = PIXELS_PER_POINT;
    let drawn = Rect::from_min_max(
        pos2(bx[0] as f32 / p, bx[1] as f32 / p),
        pos2(bx[2] as f32 / p, bx[3] as f32 / p),
    );
    assert!(
        (recorded.min - drawn.min).length() <= 1.0 && (recorded.max - drawn.max).length() <= 1.0,
        "the recorded preview {recorded:?} is not where it is drawn, {drawn:?}"
    );
}

/// A window size, in pixels, at the capture's width, whose figure (the viewport) is square; scanned over a fixed
/// range of heights.
fn square_window() -> Option<[u32; 2]> {
    (600..=3000u32).find_map(|hpx| {
        let size = [SIZE[0], hpx];
        // The window's rect in points, as the headless context gives it, built without a context.
        let screen = Rect::from_min_size(
            Pos2::ZERO,
            vec2(size[0] as f32, size[1] as f32) / PIXELS_PER_POINT,
        );
        let f = Layout::new(screen, PIXELS_PER_POINT).figure;
        ((f.width() - f.height()).abs() < 0.5 / PIXELS_PER_POINT).then_some(size)
    })
}

#[test]
fn qa_mock_stain_preview_is_square_for_a_square_viewport() {
    let size = square_window().expect("no window of the capture's width gives a square figure");
    let mut rig = Rig::stain_sized(None, size);
    rig.idle(1);
    let figure = Layout::new(rig.h.screen(), PIXELS_PER_POINT).figure;
    let preview = rig.app.stain.preview.expect("the preview's rect recorded");
    // One pixel: the rect's edges fall on whole pixels.
    let px = 1.0 / PIXELS_PER_POINT;
    let check = |r: Rect| {
        assert!(r.width() > 32.0, "the preview {r:?} is too small to read");
        assert!(
            (r.width() - r.height()).abs() <= px + 1e-3,
            "the preview {r:?} is not square for the square viewport {figure:?}"
        );
    };
    check(preview);
    rejects("a 2:1 preview", || {
        check(Rect::from_min_size(
            preview.min,
            vec2(preview.width(), preview.width() / 2.0),
        ))
    });
    // And at the capture's size, a viewport not square, it is not square either: it follows the viewport.
    let mut other = Rig::stain(None);
    other.idle(1);
    let f = layout().figure;
    let r = other
        .app
        .stain
        .preview
        .expect("the preview's rect recorded");
    if (f.width() - f.height()).abs() > 2.0 {
        rejects("a viewport not square", || check(r));
    }
}

/// Clicks the inspector's choice named `label` on the selected node.
fn click_choice(rig: &mut Rig, right: Rect, label: &str) {
    let names = rig.names();
    let at = names
        .iter()
        .filter(|n| n.name == label)
        .filter_map(|n| n.rect.map(rect_px))
        .find(|r| right.contains_rect(*r))
        .unwrap_or_else(|| {
            panic!(
                "no `{label}` in the inspector: {:?}",
                names_in(&names, right)
            )
        })
        .center();
    rig.click(at, PointerButton::Primary);
    rig.idle(2);
}

#[test]
fn qa_mock_stain_preview_an_edit_is_a_set_field_and_the_preview_changes() {
    let canvas = gpu();
    let bx = preview_box(&canvas);
    let mut rig = Rig::stain(Some(canvas.clone()));
    let right = rig.tab_to("Preview & node inspector");
    rig.app.view.focus.path.clear();
    let g0 = rig.graph();
    let d0 = rig.undo_depth();
    let combiner = find(&g0, NodeKind::Combiner)[0];
    let rects = node_rects(&mut rig);
    rig.click(
        title_point(rect_of(&rects, combiner)),
        PointerButton::Primary,
    );
    let before = rig.render();
    // Part II §3.1's combiners: Replace-L and Multiply. Pick the one the combiner does not hold.
    let holds_multiply =
        matches!(&g0.node(combiner).unwrap().occupant, Occupant::Builtin(id) if id == "multiply");
    click_choice(
        &mut rig,
        right,
        if holds_multiply {
            "Replace-L"
        } else {
            "Multiply"
        },
    );
    let g1 = rig.graph();
    assert_ne!(g1, g0, "the choice did not change the graph");
    assert_ne!(
        g1.node(combiner).unwrap().occupant,
        g0.node(combiner).unwrap().occupant,
        "the combiner's occupant did not change"
    );
    assert_eq!(
        rig.undo_depth(),
        d0 + 1,
        "the edit is not one undoable SetField"
    );
    let logged = rig
        .app
        .console()
        .iter()
        .filter(|e| e.message.starts_with("SetField StainGraph"))
        .count();
    assert_eq!(logged, 1, "the engine did not log one StainGraph SetField");
    // The mock's own graph is the edited one: the edit reached the engine, not only the GUI.
    assert_eq!(
        rig.app.side().engine().graph(),
        &g1,
        "the engine does not hold the edit"
    );
    let after = rig.render();
    let changed = diff_box(&before, &after, Some(bx));
    let check_changed = |c: Option<([usize; 4], usize)>| {
        let (_, n) = c.expect("the preview did not change with the edit");
        assert!(n > 0);
    };
    check_changed(changed);
    rejects("the same frame twice", || {
        check_changed(diff_box(&before, &before, Some(bx)))
    });
    // Undo, the app's request to the contract's history, restores the graph and the preview pixel for pixel.
    rig.app.undo();
    rig.idle(2);
    assert_eq!(rig.graph(), g0, "undo did not restore the graph");
    let undone = rig.render();
    assert!(
        diff_box(&before, &undone, Some(bx)).is_none(),
        "undo did not restore the preview"
    );
}

// --- REQ-GUI-169 re-checked on the Stain page --------------------------------------------------------------------------

fn tab_walk(rig: &mut Rig, modifiers: Modifiers) -> Vec<String> {
    rig.app.view.focus.path.clear();
    let mut seen = Vec::new();
    for _ in 0..BIG.len() {
        rig.key(Key::Tab, modifiers);
        seen.push(rig.last_label());
        assert_eq!(
            rig.app.view.focus.mode, NAV,
            "a Tab landing is not navigation mode"
        );
    }
    seen
}

fn check_order(got: &[String], want: &[&str]) {
    assert!(
        got.len() == want.len() && got.iter().zip(want).all(|(g, w)| same(g, w)),
        "the big scopes are {got:?}, the task gives {want:?}"
    );
}

#[test]
fn qa_mock_keyboard_stain_tab_and_shift_tab_reach_the_big_scopes_in_order() {
    let mut rig = Rig::stain(None);
    let forward = tab_walk(&mut rig, NONE);
    check_order(&forward, &BIG);
    // Shift+Tab from no focus lands on the last, the footer (R-405), and runs back.
    let back = tab_walk(&mut rig, SHIFT);
    let mut reversed = BIG;
    reversed.reverse();
    check_order(&back, &reversed);
    rejects("the reversed order forwards", || {
        check_order(&forward, &reversed)
    });
}

#[test]
fn qa_mock_keyboard_stain_enter_reaches_each_kind_of_control() {
    let mut rig = Rig::stain(None);
    // The library: a row of the catalogue (a drawn name in the drawer, not its buttons, filter or collapse).
    let library = rig.tab_to("Library drawer");
    let drawn = names_in(&rig.names(), library);
    let walk = rig.walk("Library drawer");
    let buttons = ["new", "rename", "import", "export"];
    let row = walk.iter().find(|l| {
        let l = l.to_lowercase();
        !buttons.contains(&l.as_str())
            && !l.contains("collapse")
            && !l.contains("filter")
            && drawn
                .iter()
                .any(|d| !d.trim().starts_with('▾') && same(d.trim(), &l))
    });
    assert!(
        row.is_some(),
        "Enter and → reach no library row: {walk:?} (drawn {drawn:?})"
    );
    // The canvas: the toggle's three buttons and every node of the graph.
    let walk = rig.walk("Graph canvas");
    for t in TOGGLE {
        assert!(
            walk.iter().any(|l| same(l, t)),
            "the walk misses the toggle's `{t}`: {walk:?}"
        );
    }
    let g = rig.graph();
    let nodes = g.nodes().count();
    let toggles = walk
        .iter()
        .filter(|l| TOGGLE.iter().any(|t| same(l, t)))
        .count();
    assert_eq!(
        walk.len() - toggles,
        nodes,
        "the walk reaches {walk:?}, not the graph's {nodes} nodes"
    );
    // The inspector: a node's field, its param, once a node holding one is selected.
    let brightness = find(&g, NodeKind::Brightness)[0];
    let param = g.uniforms(brightness).expect("the brightness's schema")[0]
        .name
        .clone();
    let rects = node_rects(&mut rig);
    rig.app.view.focus.path.clear();
    rig.click(
        title_point(rect_of(&rects, brightness)),
        PointerButton::Primary,
    );
    let walk = rig.walk("Preview & node inspector");
    assert!(
        walk.iter().any(|l| same(l, &param)),
        "the inspector's walk misses `{param}`: {walk:?}"
    );
    rejects("a walk without the param", || {
        assert!(TOGGLE.iter().any(|l| same(l, &param)))
    });
}

#[test]
fn qa_mock_keyboard_stain_enter_on_a_node_selects_it_in_navigation_mode() {
    let mut rig = Rig::stain(None);
    let g = rig.graph();
    let colour = find(&g, NodeKind::Colour)[0];
    let occupant = match &g.node(colour).unwrap().occupant {
        Occupant::Builtin(id) => id.clone(),
        o => panic!("{o:?}"),
    };
    rig.focus_element("Graph canvas", |l| l.contains(&occupant));
    assert_eq!(
        rig.app.view.focus.mode, NAV,
        "Enter into a scope lands in navigation mode"
    );
    rig.key(Key::Enter, NONE);
    assert_eq!(
        selection(&rig),
        BTreeSet::from([colour]),
        "Enter on the node is not its click"
    );
    assert_eq!(
        rig.app.view.focus.mode, NAV,
        "Enter on a node (a click) left navigation mode"
    );
    // The breadcrumb names the path: the canvas, then the node.
    let path = rig.labels();
    let crumb = rig
        .names()
        .into_iter()
        .filter(|n| n.name.contains('›'))
        .map(|n| n.name)
        .collect::<Vec<_>>();
    let want = path.join(" › ");
    assert!(
        crumb.iter().any(|c| c.contains(&want)),
        "the breadcrumb {crumb:?} does not name {want:?}"
    );
    assert!(same(&path[0], "Graph canvas"), "the path {path:?}");
}

/// The fade's value in the engine's graph.
fn param_value(rig: &Rig, id: NodeId, name: &str) -> f64 {
    let g = rig.graph();
    let n = g.node(id).unwrap();
    match n.params.get(name) {
        Some(v) => v[0],
        None => {
            g.uniforms(id)
                .unwrap()
                .into_iter()
                .find(|u| u.name == name)
                .unwrap()
                .default[0]
        }
    }
}

#[test]
fn qa_mock_keyboard_stain_a_param_steps_by_its_base_step_in_interaction_mode() {
    let mut rig = Rig::stain(None);
    let g = rig.graph();
    let brightness = find(&g, NodeKind::Brightness)[0];
    let u = g
        .uniforms(brightness)
        .unwrap()
        .into_iter()
        .next()
        .expect("a param");
    let (lo, hi) = u.range.expect("a scalar param has a range");
    // R-404's `Bounded`: 1/100 of the field's range; Shift ×10, Alt ×0.1.
    let base = (hi - lo) / 100.0;
    let rects = node_rects(&mut rig);
    rig.click(
        title_point(rect_of(&rects, brightness)),
        PointerButton::Primary,
    );
    rig.focus_element("Preview & node inspector", |l| same(l, &u.name));
    assert_eq!(rig.app.view.focus.mode, NAV);
    let element = rig.app.view.focus.path.clone();
    // In navigation mode an arrow moves between siblings; it never steps the value.
    let v0 = param_value(&rig, brightness, &u.name);
    let d0 = rig.undo_depth();
    rig.key(Key::ArrowRight, NONE);
    assert_ne!(
        rig.app.view.focus.path, element,
        "→ in navigation mode did not move to a sibling"
    );
    rig.key(Key::ArrowLeft, NONE);
    assert_eq!(
        rig.app.view.focus.path, element,
        "← did not come back to the param"
    );
    assert_eq!(
        (param_value(&rig, brightness, &u.name), rig.undo_depth()),
        (v0, d0),
        "an arrow in navigation mode stepped the value"
    );
    rig.key(Key::Enter, NONE);
    assert_eq!(
        rig.app.view.focus.mode, INTERACT,
        "Enter on a param did not start interaction"
    );
    let mut v = v0;
    for (k, (key, m, step)) in [
        (Key::ArrowLeft, NONE, -base),
        (Key::ArrowLeft, SHIFT, -10.0 * base),
        (Key::ArrowRight, ALT, 0.1 * base),
        (Key::ArrowRight, NONE, base),
    ]
    .into_iter()
    .enumerate()
    {
        rig.key(key, m);
        let now = param_value(&rig, brightness, &u.name);
        let want = (v + step).clamp(lo, hi);
        let check = |now: f64| {
            assert!(
                (now - want).abs() < 1e-9 * (hi - lo),
                "{key:?} with {m:?} stepped {v} to {now}, not {want}"
            );
        };
        check(now);
        rejects("no step", || check(v));
        assert_eq!(
            rig.undo_depth(),
            d0 + k as u32 + 1,
            "a step is not one SetField"
        );
        v = now;
        assert_eq!(rig.app.view.focus.path, element, "the step moved the focus");
    }
    // Esc: back to navigation mode on the same element; again: up one scope.
    rig.key(Key::Escape, NONE);
    assert_eq!(
        (rig.app.view.focus.mode, &rig.app.view.focus.path),
        (NAV, &element),
        "Esc in interaction mode did not return to navigation on the param"
    );
    rig.key(Key::Escape, NONE);
    assert_eq!(
        rig.app.view.focus.path.as_slice(),
        &element[..element.len() - 1],
        "Esc in navigation mode did not go up one scope"
    );
    // Undo, the app's request to the contract's history, walks the steps back.
    for _ in 0..4 {
        rig.app.undo();
        rig.idle(1);
    }
    assert!(
        (param_value(&rig, brightness, &u.name) - v0).abs() < 1e-12,
        "undo did not restore the param"
    );
}

/// The ring strokes drawn, `RING_WIDTH` wide, around the focused scope.
fn rings(rig: &mut Rig) -> Vec<Color32> {
    let place = rig.focus_rect();
    rig.shapes()
        .into_iter()
        .filter_map(|s| match s {
            Shape::Rect(r)
                if r.stroke.width == gui::explore::breadcrumb::RING_WIDTH
                    && (r.rect.center() - place.center()).length() < 3.0
                    && (r.rect.size() - place.size()).length() < 8.0 =>
            {
                Some(r.stroke.color)
            }
            _ => None,
        })
        .collect()
}

fn hue(c: Color32) -> f32 {
    let [r, g, b] = [c.r(), c.g(), c.b()].map(|v| f32::from(v) / 255.0);
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let d = max - min;
    if d == 0.0 {
        0.0
    } else if max == r {
        60.0 * ((g - b) / d).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    }
}

fn check_orange(c: Color32) {
    assert!((15.0..=45.0).contains(&hue(c)), "{c:?} is not an orange");
}

#[test]
fn qa_mock_keyboard_stain_one_ring_orange_navigating_blue_interacting() {
    let mut rig = Rig::stain(None);
    let blue = rig.h.ctx.global_style().visuals.hyperlink_color;
    // On a node, selected by Enter: one ring, orange; the selection's outline is not a second ring.
    let g = rig.graph();
    let colour = find(&g, NodeKind::Colour)[0];
    let occupant = match &g.node(colour).unwrap().occupant {
        Occupant::Builtin(id) => id.clone(),
        o => panic!("{o:?}"),
    };
    rig.focus_element("Graph canvas", |l| l.contains(&occupant));
    rig.key(Key::Enter, NONE);
    assert!(selection(&rig).contains(&colour));
    let on_node = rings(&mut rig);
    assert_eq!(
        on_node.len(),
        1,
        "one ring on the selected node: {on_node:?}"
    );
    check_orange(on_node[0]);
    // On a param in interaction mode: one ring, blue.
    let brightness = find(&g, NodeKind::Brightness)[0];
    let name = g.uniforms(brightness).unwrap()[0].name.clone();
    let rects = node_rects(&mut rig);
    rig.click(
        title_point(rect_of(&rects, brightness)),
        PointerButton::Primary,
    );
    rig.focus_element("Preview & node inspector", |l| same(l, &name));
    let nav = rings(&mut rig);
    rig.key(Key::Enter, NONE);
    let interact = rings(&mut rig);
    assert_eq!(
        (nav.len(), interact.len()),
        (1, 1),
        "one ring each: {nav:?} {interact:?}"
    );
    check_orange(nav[0]);
    assert_eq!(
        interact[0], blue,
        "the ring interacting is not hyperlink_color"
    );
    rejects("the blue ring", || check_orange(interact[0]));
}
