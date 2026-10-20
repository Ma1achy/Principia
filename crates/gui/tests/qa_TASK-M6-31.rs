//! QA tests for TASK-M6-31, written from the requirements and R-409, not from the implementation:
//!
//! - REQ-GUI-181: the compass is a fixed orthographic view at its default angle, with no orbit (no drag and no key
//!   turns the view); a Tilt and a Slice button beside it, Tilt the default, a click on one switching the mode and the
//!   buttons always showing the current mode; touching a slice slider switches it to Slice and touching a tilt to Tilt;
//!   in tilt mode a drag on it tilts the plane, one SetField on the basis; in slice mode a drag anywhere on it moves the
//!   slice plane along its normal, one SetField on z₀ per drag, the tilt and γ unchanged, and, locked, an excursion
//!   along that line through the anchor (R-409 points 1 and 2, A3).
//! - REQ-GUI-182: the keyboard's two modes: navigation (orange ring), where the arrows move between siblings, on a big
//!   scope between the big scopes; interaction (blue ring, `hyperlink_color`), where the element takes the arrows while
//!   Tab, Ctrl+Z, `?` and the shortcuts keep their meaning; every landing in navigation mode, a Tab on the big scope
//!   itself; Enter on an element starts interaction, on the Figure too, on the Legend does nothing, on a button acts as
//!   its click; Esc or Enter in interaction mode back to navigation mode on the same element; Esc in navigation mode up
//!   one scope; nothing else on screen changing with the mode but the ring's colour (and the breadcrumb's, A6).
//! - REQ-GUI-183: the Compass scope holds the compass, Tilt and Slice, in that order; Enter on a button sets the mode;
//!   Enter on the compass starts interaction, where the arrows tilt by R-404's `Tilt` step in tilt mode and move the
//!   slice along the normal by REQ-GUI-184's step in slice mode, Shift ×10 and Alt ×0.1 as everywhere (F3).
//! - REQ-GUI-184 (calibration, proposed, pending the human at the M8 gate, R-71 / R-182): the tests check that the code
//!   honours the declared step, whatever it is, not the value.
//!
//! The SetFields a gesture emits are counted from the contract's own log (every engine logs each SetField it applies as
//! one `info` entry from `contract`, gui_state_contract §2); what they edited is read from the snapshots. The plane's
//! in-plane directions are its basis `q₁`, `q₂` (chart_decoder_contract Part 3); its normal in the compass's frame is
//! the direction of that frame perpendicular to both. The compass's mode is read from what it draws: the button drawn
//! selected. gui takes no dependency on validation (R-187), so each check runs its own control (R-176) through
//! [`rejects`].
// The file name `qa_TASK-M6-31` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::sync::Arc;

use eframe::egui::{self, Color32, Event, Key, Modifiers, PointerButton, Pos2, Rect};
use engine::contract::log::Source;
use engine::contract::snapshot::Snapshot;
use engine::contract::view_ui::{KeyMode, Mode};
use gui::app::App;
use gui::capture::{self, Shot, Step, FORMAT, PIXELS_PER_POINT, SIZE};
use gui::headless::Headless;
use gui::keyboard::scopes::{Base, StepKind};
use gui::layout::Layout;
use gui::mock::canvas::MockCanvas;
use gui::mock::MockEngine;
use gui::side::MockSide;

type Z = [f64; 8];

const NONE: Modifiers = Modifiers::NONE;
const SHIFT: Modifiers = Modifiers::SHIFT;
const ALT: Modifiers = Modifiers::ALT;
/// A key no command binds: pressed only to turn keyboard navigation on (R-460, R-464 M2).
const UNBOUND: Key = Key::F12;
const NAV: KeyMode = KeyMode::Navigation;
const INTERACT: KeyMode = KeyMode::Interaction;
/// The arrows.
const ARROWS: [Key; 4] = [
    Key::ArrowLeft,
    Key::ArrowRight,
    Key::ArrowUp,
    Key::ArrowDown,
];

/// Asserts that `check` panics: the check can fail (R-176's control, run in the test).
fn rejects(what: &str, check: impl FnOnce()) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check));
    assert!(
        outcome.is_err(),
        "the check passed on {what}, which it must reject"
    );
}

// --- the rig --------------------------------------------------------------------------------------------------------

fn headless() -> Headless {
    Headless::new(SIZE, PIXELS_PER_POINT)
}

fn layout() -> Layout {
    Layout::new(headless().screen(), PIXELS_PER_POINT)
}

struct Rig {
    app: App<MockSide>,
    h: Headless,
}

fn mock() -> Rig {
    let mut rig = Rig {
        app: App::new(MockSide::new(MockEngine::frozen(), None), FORMAT),
        h: headless(),
    };
    rig.idle(2);
    rig
}

impl Rig {
    fn frame(&mut self, events: Vec<Event>) -> egui::FullOutput {
        self.h.frame(&mut self.app, events)
    }

    fn idle(&mut self, n: usize) {
        for _ in 0..n {
            let _ = self.frame(Vec::new());
        }
    }

    fn key(&mut self, key: Key, modifiers: Modifiers) {
        for pressed in [true, false] {
            let _ = self.frame(vec![Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers,
            }]);
        }
        self.idle(2);
    }

    fn keys(&mut self, keys: &[Key]) {
        for k in keys {
            self.key(*k, NONE);
        }
    }

    fn snap(&self) -> Snapshot {
        self.app.snapshot().clone()
    }

    /// The SetFields the engine has applied so far: its `contract` log entries, less the hover's. Since TASK-M6-27 a
    /// pointer over the figure writes the cursor, `Hover.at`, as a no-history SetField (gui_state_contract §2; R-101),
    /// which the rig's pointer moves add to any gesture; it is no edit of the compass, the plane or the lock.
    fn applied(&self) -> usize {
        self.app
            .console()
            .iter()
            .filter(|e| {
                e.source == Source::Contract && !e.message.starts_with("SetField Hover.at ")
            })
            .count()
    }

    /// The focus: its scopes' labels, outermost first, and its mode.
    fn at(&self) -> (Vec<String>, KeyMode) {
        let labels = self
            .app
            .keyboard
            .tree(Mode::Explore)
            .labels(&self.app.view.focus.path)
            .into_iter()
            .map(str::to_owned)
            .collect();
        (labels, self.app.view.focus.mode)
    }

    /// Puts the focus on the scope labelled `label` (the first, breadth first), in navigation mode, as a landing is.
    /// The focus is set by hand, as no user can, so a key no command binds is then pressed to turn keyboard
    /// navigation on, as a user arriving there by keys has it: the GUI starts with it off, and while it is off an
    /// Enter only shows the ring (R-460, R-464 M2).
    fn focus(&mut self, label: &str) {
        let tree = self.app.keyboard.tree(Mode::Explore);
        let mut all: Vec<&'static str> = tree.children(None);
        let mut i = 0;
        while i < all.len() {
            let kids = tree.children(Some(all[i]));
            all.extend(kids);
            i += 1;
        }
        let id = all
            .into_iter()
            .find(|id| tree.get(id).unwrap().label == label)
            .unwrap_or_else(|| panic!("no scope `{label}`"));
        self.app.view.focus.path = tree.path_to(id);
        self.app.view.focus.mode = NAV;
        self.key(UNBOUND, NONE);
        assert!(
            self.app.keyboard.navigating(),
            "a key press left keyboard navigation off"
        );
    }

    /// The focus on `label`, then Enter on it: interaction mode.
    fn interact(&mut self, label: &str) {
        self.focus(label);
        self.key(Key::Enter, NONE);
        assert_eq!(
            self.app.view.focus.mode, INTERACT,
            "Enter on {label} did not start interaction mode"
        );
    }

    /// The shapes drawn this frame, flattened.
    fn shapes(&mut self) -> Vec<egui::Shape> {
        let out = self.frame(Vec::new());
        let mut flat = Vec::new();
        for clipped in &out.shapes {
            flatten(&clipped.shape, &mut flat);
        }
        flat
    }

    fn pointer_away(&mut self) {
        let away = layout().figure.center();
        let _ = self.frame(vec![Event::PointerMoved(away)]);
        self.idle(1);
    }

    fn click(&mut self, at: Pos2) {
        let button = |pressed| Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed,
            modifiers: NONE,
        };
        let _ = self.frame(vec![Event::PointerMoved(at)]);
        let _ = self.frame(vec![button(true)]);
        let _ = self.frame(vec![button(false)]);
        self.idle(1);
        self.pointer_away();
    }

    fn drag(&mut self, from: Pos2, to: Pos2, steps: usize) {
        let button = |pos, pressed| Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: NONE,
        };
        let _ = self.frame(vec![Event::PointerMoved(from)]);
        let _ = self.frame(vec![button(from, true)]);
        for i in 1..=steps {
            let at = from + (to - from) * (i as f32 / steps as f32);
            let _ = self.frame(vec![Event::PointerMoved(at)]);
        }
        let _ = self.frame(vec![button(to, false)]);
        self.idle(2);
        self.pointer_away();
    }

    /// K with the pointer over the figure, off its centre.
    fn lock(&mut self) {
        let f = layout().figure;
        let at = f.min + egui::vec2(f.width() * 0.3, f.height() * 0.6);
        let _ = self.frame(vec![Event::PointerMoved(at)]);
        self.key(Key::K, NONE);
        assert!(self.snap().sim.lock.locked, "K did not lock");
    }

    /// The visuals the app draws with.
    fn visuals(&self) -> egui::Visuals {
        self.h.ctx.global_style().visuals.clone()
    }
}

fn flatten(shape: &egui::Shape, out: &mut Vec<egui::Shape>) {
    match shape {
        egui::Shape::Vec(v) => v.iter().for_each(|s| flatten(s, out)),
        other => out.push(other.clone()),
    }
}

// --- vectors --------------------------------------------------------------------------------------------------------

fn add(a: &Z, k: f64, b: &Z) -> Z {
    std::array::from_fn(|i| a[i] + k * b[i])
}

fn dot(a: &Z, b: &Z) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn norm(a: &Z) -> f64 {
    dot(a, a).sqrt()
}

fn unit(a: &Z) -> Z {
    let n = norm(a);
    a.map(|x| x / n)
}

/// The part of `d` outside the span of `basis` (Gram–Schmidt on the basis).
fn residual(d: &Z, basis: &[Z]) -> Z {
    let mut ortho: Vec<Z> = Vec::new();
    for b in basis {
        let mut v = *b;
        for o in &ortho {
            v = add(&v, -dot(&v, o), o);
        }
        if norm(&v) > 1e-14 {
            ortho.push(unit(&v));
        }
    }
    let mut r = *d;
    for o in &ortho {
        r = add(&r, -dot(&r, o), o);
    }
    r
}

// --- the compass as drawn -------------------------------------------------------------------------------------------

/// The compass's mode, as it draws it: each of its Tilt and Slice buttons drawn selected, on a filled frame of its own
/// (egui frames a selectable button only when selected or hovered; the pointer is kept off the compass).
fn compass_mode(shapes: &[egui::Shape]) -> Vec<&'static str> {
    let compass = layout().compass;
    let mut shown = Vec::new();
    for label in ["Tilt", "Slice"] {
        let text = shapes.iter().find_map(|s| match s {
            egui::Shape::Text(t) if t.galley.text() == label => {
                Some(s.visual_bounding_rect()).filter(|r| compass.contains_rect(*r))
            }
            _ => None,
        });
        let text = text.unwrap_or_else(|| panic!("no {label} button drawn beside the compass"));
        let framed = shapes.iter().any(|s| match s {
            egui::Shape::Rect(r) => {
                r.fill.a() > 0
                    && r.rect.contains_rect(text)
                    && r.rect.width() < 3.0 * text.width() + 40.0
            }
            _ => false,
        });
        if framed {
            shown.push(label);
        }
    }
    shown
}

fn check_mode(shapes: &[egui::Shape], want: &str, what: &str) {
    let shown = compass_mode(shapes);
    assert!(
        shown == [want],
        "{what}: the buttons show {shown:?}, not {want}"
    );
}

/// The centre of the button labelled `label` beside the compass.
fn button_at(shapes: &[egui::Shape], label: &str) -> Pos2 {
    let compass = layout().compass;
    shapes
        .iter()
        .find_map(|s| match s {
            egui::Shape::Text(t) if t.galley.text() == label => {
                Some(s.visual_bounding_rect()).filter(|r| compass.contains_rect(*r))
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("no {label} button"))
        .center()
}

/// The cube's edges as drawn: the line segments inside the compass of an edge's width, `0.8 + 1.0·near` points with
/// `near = (x + z + 2)/4` at the edge's middle, one of 0.8, 1.05, 1.3, 1.55 and 1.8 (R-451 B6, which sets the edges'
/// opacity by depth, so their colour no longer tells them apart; the grid is 0.7 points, the outline 1.7, the pin 2).
fn cube_edges(rig: &mut Rig) -> Vec<[Pos2; 2]> {
    let compass = layout().compass;
    let edge_width = |w: f32| {
        [0.8_f32, 1.05, 1.3, 1.55, 1.8]
            .iter()
            .any(|e| (w - e).abs() < 1e-3)
    };
    rig.shapes()
        .into_iter()
        .filter_map(|s| match s {
            egui::Shape::LineSegment { points, stroke }
                if edge_width(stroke.width)
                    && compass.contains(points[0])
                    && compass.contains(points[1]) =>
            {
                Some(points)
            }
            _ => None,
        })
        .collect()
}

/// The twelve edges are a cube seen orthographically: three classes of four parallel edges of equal length (a
/// perspective view converges them).
fn check_orthographic_cube(edges: &[[Pos2; 2]]) {
    assert_eq!(edges.len(), 12, "the cube's twelve edges: {edges:?}");
    let dir = |[a, b]: [Pos2; 2]| {
        let d = b - a;
        if d.x < -1e-3 || (d.x.abs() <= 1e-3 && d.y < 0.0) {
            -d
        } else {
            d
        }
    };
    let mut classes: Vec<Vec<egui::Vec2>> = Vec::new();
    for e in edges {
        let d = dir(*e);
        match classes
            .iter_mut()
            .find(|c| (c[0] - d).length() < 0.05 * c[0].length().max(1.0))
        {
            Some(c) => c.push(d),
            None => classes.push(vec![d]),
        }
    }
    let sizes: Vec<usize> = classes.iter().map(Vec::len).collect();
    assert!(
        sizes == [4, 4, 4],
        "the cube's edges are not three classes of four parallel equal edges: {sizes:?}"
    );
}

/// The plane's polygon as drawn: the filled closed path in the compass, the true cross-section of 3 to 6 vertices
/// (R-451 B2; it was the fixed quad before).
fn plane_polygon(shapes: &[egui::Shape]) -> Vec<Pos2> {
    let compass = layout().compass;
    shapes
        .iter()
        .find_map(|s| match s {
            egui::Shape::Path(p)
                if p.closed
                    && (3..=6).contains(&p.points.len())
                    && p.fill.a() > 0
                    && p.points.iter().all(|q| compass.contains(*q)) =>
            {
                Some(p.points.clone())
            }
            _ => None,
        })
        .expect("the plane drawn in the compass")
}

fn inside(poly: &[Pos2], p: Pos2) -> bool {
    let n = poly.len();
    let side = |i: usize| {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        (b - a).x * (p - a).y - (b - a).y * (p - a).x
    };
    (0..n).all(|i| side(i) > 0.0) || (0..n).all(|i| side(i) < 0.0)
}

/// Two drag starts on the compass: on the plane (its centre) and off it (inside the cube's square, outside the plane).
fn starts(rig: &mut Rig) -> (Pos2, Pos2) {
    let shapes = rig.shapes();
    let poly = plane_polygon(&shapes);
    let centre = poly.iter().fold(egui::Vec2::ZERO, |a, p| a + p.to_vec2()) / poly.len() as f32;
    let on = centre.to_pos2();
    assert!(inside(&poly, on));
    let edges = cube_edges(rig);
    let bbox = edges
        .iter()
        .flatten()
        .fold(Rect::NOTHING, |r, p| r.union(Rect::from_min_max(*p, *p)));
    let off = [
        bbox.left_top(),
        bbox.right_top(),
        bbox.left_bottom(),
        bbox.right_bottom(),
    ]
    .into_iter()
    .map(|c| c + (bbox.center() - c) * 0.08)
    .find(|p| !inside(&poly, *p))
    .expect("a point of the cube off the plane");
    (on, off)
}

// --- what an edit did ----------------------------------------------------------------------------------------------

/// The gesture `act` applied exactly one SetField; returns the snapshots before and after.
fn one_set_field(rig: &mut Rig, what: &str, act: impl FnOnce(&mut Rig)) -> (Snapshot, Snapshot) {
    let (before, n0) = (rig.snap(), rig.applied());
    act(rig);
    let after = rig.snap();
    assert_eq!(
        rig.applied() - n0,
        1,
        "{what}: {} SetFields applied, the gesture is one",
        rig.applied() - n0
    );
    (before, after)
}

/// A tilt: the basis edited, `z₀` and the lock not.
fn check_tilt(before: &Snapshot, after: &Snapshot, what: &str) {
    let (a, b) = (&before.sim.plane, &after.sim.plane);
    assert!(
        (a.q1, a.q2) != (b.q1, b.q2),
        "{what}: the basis did not change"
    );
    assert_eq!(a.z0, b.z0, "{what}: z₀ changed");
    assert_eq!(before.sim.lock, after.sim.lock, "{what}: the lock changed");
}

/// A move along the plane's normal: `z₀` moved, the basis and the lock unchanged, and the move has no component along
/// the plane's in-plane directions `q₁`, `q₂` (within 1e-12, REQ-GUI-181) and lies in the compass's frame, the span of
/// the preset's axes and the hidden slice direction (`frame`). Returns the move.
fn check_along_normal(before: &Snapshot, after: &Snapshot, frame: &[Z], what: &str) -> Z {
    let (a, b) = (&before.sim.plane, &after.sim.plane);
    assert_eq!((a.q1, a.q2), (b.q1, b.q2), "{what}: the basis changed");
    assert_eq!(before.sim.lock, after.sim.lock, "{what}: the lock changed");
    let d = add(&b.z0, -1.0, &a.z0);
    assert!(norm(&d) > 0.0, "{what}: z₀ did not move");
    for (q, name) in [(&a.q1, "q₁"), (&a.q2, "q₂")] {
        let along = dot(&d, &unit(q));
        assert!(
            along.abs() <= 1e-12 * norm(&d).max(1.0),
            "{what}: the move {d:?} has {along} along {name}"
        );
    }
    let off = norm(&residual(&d, frame));
    assert!(
        off <= 1e-12 * norm(&d).max(1.0),
        "{what}: the move {d:?} leaves the compass's frame by {off}"
    );
    d
}

/// The hidden slice direction of the default chart, read from the slice step: ↑ on it moves `z₀` along that direction.
fn slice_direction() -> Z {
    let mut rig = mock();
    rig.interact("slice step");
    let before = rig.snap().sim.plane.z0;
    rig.key(Key::ArrowUp, NONE);
    unit(&add(&rig.snap().sim.plane.z0, -1.0, &before))
}

/// The compass's frame for the default chart: the preset's two axes (the free chart's `q₁`, `q₂`) and the hidden slice
/// direction.
fn compass_frame() -> Vec<Z> {
    let p = mock().snap().sim.plane;
    vec![unit(&p.q1), unit(&p.q2), slice_direction()]
}

/// A rig whose plane is tilted by both tilts and turned by `γ`, through the sliders.
fn tilted() -> Rig {
    let mut rig = mock();
    for (label, presses) in [("τ₁", 2), ("τ₂", 1), ("γ", 1)] {
        rig.interact(label);
        for _ in 0..presses {
            rig.key(Key::ArrowUp, SHIFT);
        }
    }
    let p = rig.snap().sim.plane;
    let frame = compass_frame();
    for q in [&p.q1, &p.q2] {
        assert!(
            norm(&residual(q, &frame)) < 1e-12,
            "the tilted basis left the compass's frame"
        );
    }
    let e_d = &frame[2];
    assert!(
        dot(&unit(&p.q1), e_d).abs() > 0.1,
        "the plane is not tilted: q₁ {:?}",
        p.q1
    );
    rig.focus("Figure");
    rig
}

// === REQ-GUI-181: the compass's view and modes ======================================================================

#[test]
fn qa_mock_compass_modes_open_in_tilt_and_follow_the_buttons() {
    let mut rig = mock();
    let shapes = rig.shapes();
    check_mode(&shapes, "Tilt", "at the start");
    let n0 = rig.applied();
    rig.click(button_at(&shapes, "Slice"));
    let slice = rig.shapes();
    check_mode(&slice, "Slice", "a click on Slice");
    rig.click(button_at(&slice, "Tilt"));
    let tilt = rig.shapes();
    check_mode(&tilt, "Tilt", "a click on Tilt");
    // The mode is the GUI's own state: the clicks edit nothing (R-409 A5).
    assert_eq!(
        rig.applied(),
        n0,
        "a click on a mode button applied a SetField"
    );
    rejects("Tilt read after a click on Slice", || {
        check_mode(&slice, "Tilt", "a click on Slice")
    });
    rejects("both buttons shown", || {
        assert!(compass_mode(&slice).len() == 1 && compass_mode(&tilt).len() == 2)
    });
}

#[test]
fn qa_mock_compass_modes_the_buttons_follow_the_slider_touched() {
    let mut rig = mock();
    let mut seen = Vec::new();
    for (label, want) in [
        ("z_q1", "Slice"),
        ("τ₁", "Tilt"),
        ("slice step", "Slice"),
        ("τ₂", "Tilt"),
        ("z_α", "Slice"),
        ("γ", "Tilt"),
    ] {
        rig.interact(label);
        rig.key(Key::ArrowUp, NONE);
        let shapes = rig.shapes();
        check_mode(&shapes, want, &format!("after touching {label}"));
        seen.push(shapes);
    }
    // After a click on a button, a slider still switches the mode: the button follows it.
    let shapes = rig.shapes();
    rig.click(button_at(&shapes, "Slice"));
    rig.interact("τ₂");
    rig.key(Key::ArrowDown, NONE);
    let after = rig.shapes();
    check_mode(&after, "Tilt", "touching τ₂ after a click on Slice");
    // Control: a button that does not follow a slider.
    rejects("Tilt left shown after a slice slider", || {
        check_mode(&seen[0], "Tilt", "after z_q1")
    });
}

#[test]
fn qa_mock_compass_modes_a_tilt_drag_on_and_off_the_plane_tilts_once() {
    for on_plane in [true, false] {
        let mut rig = mock();
        let (on, off) = starts(&mut rig);
        let from = if on_plane { on } else { off };
        let what = format!(
            "a tilt-mode drag {}",
            if on_plane { "on the plane" } else { "off it" }
        );
        let (before, after) = one_set_field(&mut rig, &what, |r| {
            r.drag(from, from + egui::vec2(14.0, -9.0), 5)
        });
        check_tilt(&before, &after, &what);
        check_mode(&rig.shapes(), "Tilt", &what);
    }
}

#[test]
fn qa_mock_compass_modes_a_slice_drag_anywhere_moves_along_the_normal() {
    let frame = compass_frame();
    let tilt_drag = {
        // A tilt-mode drag on the same tilted plane, for the control.
        let mut rig = tilted();
        let (on, _) = starts(&mut rig);
        one_set_field(&mut rig, "a tilt drag", |r| {
            r.drag(on, on + egui::vec2(0.0, -12.0), 4)
        })
    };
    for on_plane in [true, false] {
        let mut rig = tilted();
        let shapes = rig.shapes();
        rig.click(button_at(&shapes, "Slice"));
        check_mode(&rig.shapes(), "Slice", "a click on Slice");
        let (on, off) = starts(&mut rig);
        let from = if on_plane { on } else { off };
        let what = format!(
            "a slice-mode drag {}",
            if on_plane { "on the plane" } else { "off it" }
        );
        // R-451 B3: the horizontal component of a slice-mode drag moves the slice; the vertical does nothing.
        let (before, after) = one_set_field(&mut rig, &what, |r| {
            r.drag(from, from + egui::vec2(12.0, 0.0), 5)
        });
        let d = check_along_normal(&before, &after, &frame, &what);
        check_mode(&rig.shapes(), "Slice", &what);
        // Controls: a slice along the depth axis on this tilted plane; a slice-mode drag that tilts.
        let depth = frame[2].map(|x| x * norm(&d));
        let mut along_depth = after.clone();
        along_depth.sim.plane.z0 = add(&before.sim.plane.z0, 1.0, &depth);
        rejects("a slice along the depth axis on a tilted plane", || {
            check_along_normal(&before, &along_depth, &frame, "along the depth axis");
        });
        rejects("a slice-mode drag that tilts", || {
            check_along_normal(&tilt_drag.0, &tilt_drag.1, &frame, "a tilt");
        });
    }
    // Untilted, the normal is the hidden slice direction: the drag moves the slice step (R-409 A3).
    let mut rig = mock();
    let shapes = rig.shapes();
    rig.click(button_at(&shapes, "Slice"));
    let (on, _) = starts(&mut rig);
    let (before, after) = one_set_field(&mut rig, "an untilted slice drag", |r| {
        r.drag(on, on + egui::vec2(12.0, 0.0), 5)
    });
    let d = check_along_normal(&before, &after, &frame, "an untilted slice drag");
    let along = dot(&unit(&d), &frame[2]).abs();
    assert!(
        (along - 1.0).abs() < 1e-12,
        "untilted, the slice drag is not along the hidden slice direction: {d:?}"
    );
}

#[test]
fn qa_mock_compass_modes_locked_a_slice_drag_is_an_excursion_along_the_normal_through_the_anchor() {
    let frame = compass_frame();
    let mut rig = tilted();
    rig.lock();
    let anchor = rig.snap().sim.lock.z_locked;
    assert_eq!(
        rig.snap().sim.plane.z0,
        anchor,
        "the lock recentres on the anchor"
    );
    let shapes = rig.shapes();
    rig.click(button_at(&shapes, "Slice"));
    let mut snaps = vec![rig.snap()];
    // R-451 B3: the horizontal component of a slice-mode drag moves the slice.
    for (i, dx) in [-12.0, 20.0].into_iter().enumerate() {
        let (on, off) = starts(&mut rig);
        let from = if i == 0 { on } else { off };
        let (before, after) = one_set_field(&mut rig, "a locked slice drag", |r| {
            r.drag(from, from + egui::vec2(dx, 0.0), 4)
        });
        check_along_normal(&before, &after, &frame, "a locked slice drag");
        assert!(after.sim.lock.locked, "the drag unlocked");
        assert_eq!(after.sim.lock.z_locked, anchor, "the anchor moved");
        snaps.push(after);
    }
    // Both drags leave z₀ on the normal's line through the anchor.
    let first = snaps[0].clone();
    let last = snaps[2].clone();
    let mut from_anchor = first.clone();
    from_anchor.sim.plane.z0 = anchor;
    check_along_normal(
        &from_anchor,
        &last,
        &frame,
        "z₀ after two drags, from the anchor",
    );
    // Control: z₀ moved along the depth axis from the anchor is off that line.
    let mut off_line = last.clone();
    off_line.sim.plane.z0 = add(&anchor, 0.05, &frame[2]);
    rejects("z₀ along the depth axis from the anchor", || {
        check_along_normal(&from_anchor, &off_line, &frame, "off the line");
    });
}

#[test]
fn qa_mock_compass_modes_no_drag_and_no_key_turns_the_view() {
    let mut rig = mock();
    let start = cube_edges(&mut rig);
    check_orthographic_cube(&start);
    let check = |rig: &mut Rig, what: &str| {
        let now = cube_edges(rig);
        assert_eq!(now, start, "{what} turned the compass's view");
    };
    // Drags in both modes, on and off the plane.
    let (on, off) = starts(&mut rig);
    rig.drag(on, on + egui::vec2(25.0, 10.0), 5);
    check(&mut rig, "a tilt drag on the plane");
    rig.drag(off, off + egui::vec2(-20.0, 15.0), 5);
    check(&mut rig, "a tilt drag off the plane");
    let shapes = rig.shapes();
    rig.click(button_at(&shapes, "Slice"));
    check(&mut rig, "a click on Slice");
    let (on, off) = starts(&mut rig);
    rig.drag(on, on + egui::vec2(25.0, 10.0), 5);
    check(&mut rig, "a slice drag on the plane");
    rig.drag(off, off + egui::vec2(-20.0, 15.0), 5);
    check(&mut rig, "a slice drag off the plane");
    // Every key on the compass, in both modes of the compass and of the keyboard.
    for compass_mode in ["Slice", "Tilt"] {
        rig.focus(compass_mode);
        rig.key(Key::Enter, NONE);
        rig.focus("compass");
        for modifiers in [NONE, SHIFT, ALT] {
            for key in ARROWS {
                rig.key(key, modifiers);
                check(
                    &mut rig,
                    &format!("{key:?} ({modifiers:?}) in navigation mode"),
                );
            }
        }
        rig.focus("compass");
        rig.key(Key::Enter, NONE);
        assert_eq!(rig.app.view.focus.mode, INTERACT);
        for modifiers in [NONE, SHIFT, ALT] {
            for key in ARROWS {
                rig.key(key, modifiers);
                check(
                    &mut rig,
                    &format!("{key:?} ({modifiers:?}) in {compass_mode} mode"),
                );
            }
        }
        for key in [Key::Plus, Key::Minus, Key::Escape] {
            rig.key(key, NONE);
            check(&mut rig, &format!("{key:?}"));
        }
    }
    // Controls: a turned view, and a view with converging edges.
    let turned: Vec<[Pos2; 2]> = start
        .iter()
        .map(|[a, b]| {
            let c = layout().compass.center();
            let r = |p: Pos2| {
                let v = p - c;
                let (s, k) = 5f32.to_radians().sin_cos();
                c + egui::vec2(k * v.x - s * v.y, s * v.x + k * v.y)
            };
            [r(*a), r(*b)]
        })
        .collect();
    rejects("a view turned by 5°", || assert_eq!(turned, start));
    let mut converging = start.clone();
    converging[0][1] += egui::vec2(4.0, 3.0);
    rejects("a perspective cube", || {
        check_orthographic_cube(&converging)
    });
}

// === REQ-GUI-182: the two keyboard modes =============================================================================

fn path(labels: &[&str]) -> Vec<String> {
    labels.iter().map(|s| (*s).to_owned()).collect()
}

fn check_at(got: &(Vec<String>, KeyMode), want: &[&str], mode: KeyMode, what: &str) {
    let ok = got.0.len() == want.len()
        && got
            .0
            .iter()
            .zip(want)
            .all(|(g, w)| g.to_lowercase() == w.to_lowercase())
        && got.1 == mode;
    assert!(
        ok,
        "{what}: the focus is {got:?}, want {want:?} in {mode:?}"
    );
}

#[test]
fn qa_mock_keyboard_modes_a_tab_lands_on_the_big_scope_and_the_arrows_move_between_big_scopes() {
    let mut rig = mock();
    rig.keys(&[Key::Tab, Key::Tab]);
    let landed = rig.at();
    check_at(&landed, &["Manifold view"], NAV, "Tab, Tab");
    rejects("a Tab that lands on a big scope's first element", || {
        check_at(
            &(path(&["Manifold view", "Chart"]), NAV),
            &["Manifold view"],
            NAV,
            "Tab",
        )
    });
    let n0 = rig.applied();
    rig.key(Key::ArrowRight, NONE);
    let right = rig.at();
    check_at(&right, &["Figure"], NAV, "→ on Manifold view");
    rig.key(Key::ArrowLeft, NONE);
    check_at(&rig.at(), &["Manifold view"], NAV, "← on the Figure");
    rig.key(Key::ArrowDown, NONE);
    check_at(&rig.at(), &["Figure"], NAV, "↓ on Manifold view");
    rig.key(Key::ArrowUp, NONE);
    check_at(&rig.at(), &["Manifold view"], NAV, "↑ on the Figure");
    assert_eq!(rig.applied(), n0, "the arrows on a big scope edited");
    rejects("arrows on a big scope that do nothing", || {
        check_at(&landed, &["Figure"], NAV, "→")
    });
    rig.key(Key::Enter, NONE);
    let entered = rig.at();
    check_at(
        &entered,
        &["Manifold view", "Chart"],
        NAV,
        "Enter on Manifold view",
    );
    rejects(
        "an Enter into a scope that lands in interaction mode",
        || {
            check_at(
                &(entered.0.clone(), INTERACT),
                &["Manifold view", "Chart"],
                NAV,
                "Enter",
            )
        },
    );
}

/// The value of `z₀` along axis `i`.
fn z(rig: &Rig, i: usize) -> f64 {
    rig.snap().sim.plane.z0[i]
}

#[test]
fn qa_mock_keyboard_modes_a_slider_needs_enter_and_esc_comes_back_out() {
    let Base::OfRange(fraction) = StepKind::Bounded.base() else {
        panic!("a slider's base step is a fraction of its range")
    };
    let step = fraction * 2.0 * gui::explore::manifold_view::Z0_HALF_RANGE;
    let mut rig = mock();
    rig.focus("Centre z₀");
    rig.key(Key::Enter, NONE);
    check_at(
        &rig.at(),
        &["Manifold view", "Centre z₀", "z_α"],
        NAV,
        "Enter on Centre z₀",
    );
    let (start, n0) = (rig.snap().sim.plane.z0, rig.applied());
    rig.key(Key::ArrowDown, NONE);
    let after_down = rig.at();
    check_at(
        &after_down,
        &["Manifold view", "Centre z₀", "z_β"],
        NAV,
        "↓ on z_α",
    );
    assert_eq!(
        rig.snap().sim.plane.z0,
        start,
        "↓ in navigation mode changed a value"
    );
    assert_eq!(rig.applied(), n0, "↓ in navigation mode applied a SetField");
    // Control: the pre-R-409 `browsing` model, where the arrows adjust a value reached by Enter on its section.
    rejects(
        "arrows adjusting a value reached by Enter on its section",
        || {
            let mut browsing = start;
            browsing[0] -= step;
            assert_eq!(browsing, start, "↓ in navigation mode changed a value");
        },
    );
    rig.key(Key::Enter, NONE);
    check_at(
        &rig.at(),
        &["Manifold view", "Centre z₀", "z_β"],
        INTERACT,
        "Enter on z_β",
    );
    let b0 = z(&rig, 1);
    rig.key(Key::ArrowUp, NONE);
    assert!(
        (z(&rig, 1) - b0 - step).abs() < 1e-12,
        "↑ moved z_β by {}, the base step is {step}",
        z(&rig, 1) - b0
    );
    rig.key(Key::ArrowDown, NONE);
    rig.key(Key::ArrowDown, NONE);
    assert!(
        (z(&rig, 1) - b0 + step).abs() < 1e-12,
        "↓ ↓ in interaction mode did not adjust z_β"
    );
    check_at(
        &rig.at(),
        &["Manifold view", "Centre z₀", "z_β"],
        INTERACT,
        "↓ in interaction mode",
    );
    rig.key(Key::Escape, NONE);
    let esc = rig.at();
    check_at(
        &esc,
        &["Manifold view", "Centre z₀", "z_β"],
        NAV,
        "Esc in interaction mode",
    );
    rejects("an Esc in interaction mode that goes up a scope", || {
        check_at(
            &(path(&["Manifold view", "Centre z₀"]), NAV),
            &["Manifold view", "Centre z₀", "z_β"],
            NAV,
            "Esc",
        )
    });
    rig.key(Key::Escape, NONE);
    check_at(
        &rig.at(),
        &["Manifold view", "Centre z₀"],
        NAV,
        "Esc in navigation mode",
    );
    // Enter on the slider again, then Enter: back to navigation mode on it.
    rig.keys(&[Key::Enter, Key::ArrowDown, Key::Enter]);
    check_at(
        &rig.at(),
        &["Manifold view", "Centre z₀", "z_β"],
        INTERACT,
        "Enter on z_β again",
    );
    rig.key(Key::Enter, NONE);
    let enter = rig.at();
    check_at(
        &enter,
        &["Manifold view", "Centre z₀", "z_β"],
        NAV,
        "Enter in interaction mode",
    );
    rejects("an Enter in interaction mode that stays in it", || {
        check_at(
            &(enter.0.clone(), INTERACT),
            &["Manifold view", "Centre z₀", "z_β"],
            NAV,
            "Enter",
        )
    });
    // Back in navigation mode the arrows move again.
    rig.key(Key::ArrowDown, NONE);
    check_at(
        &rig.at(),
        &["Manifold view", "Centre z₀", "z_q0"],
        NAV,
        "↓ after Enter out",
    );
}

/// On the Centre z₀ slider z_α, in `mode`: what Ctrl+Z, `?` and Tab do. Returns the focus and the undo depth after
/// Ctrl+Z, whether `?` opened the shortcuts, the focus after closing them, and the focus after Tab.
type Global = (
    (Vec<String>, KeyMode),
    u32,
    bool,
    (Vec<String>, KeyMode),
    (Vec<String>, KeyMode),
);

fn global_keys(mode: KeyMode) -> Global {
    let mut rig = mock();
    rig.focus("Centre z₀");
    rig.key(Key::Enter, NONE);
    // An edit to undo.
    rig.key(Key::Enter, NONE);
    rig.key(Key::ArrowUp, NONE);
    if mode == NAV {
        rig.key(Key::Escape, NONE);
    }
    assert_eq!(rig.app.view.focus.mode, mode);
    let depth = rig.snap().history.undo_depth;
    rig.key(Key::Z, Modifiers::CTRL);
    let after_undo = rig.at();
    let undone = depth - rig.snap().history.undo_depth;
    rig.key(Key::Questionmark, SHIFT);
    let overlay = rig.app.keyboard.shortcuts_open;
    rig.key(Key::Escape, NONE);
    assert!(
        !rig.app.keyboard.shortcuts_open,
        "Esc did not close the shortcuts"
    );
    let after_overlay = rig.at();
    rig.key(Key::Tab, NONE);
    (after_undo, undone, overlay, after_overlay, rig.at())
}

#[test]
fn qa_mock_keyboard_modes_tab_ctrl_z_and_question_keep_their_meaning_in_interaction_mode() {
    let nav = global_keys(NAV);
    let interact = global_keys(INTERACT);
    let slider = ["Manifold view", "Centre z₀", "z_α"];
    check_at(&interact.0, &slider, INTERACT, "Ctrl+Z in interaction mode");
    check_at(
        &interact.3,
        &slider,
        INTERACT,
        "`?` and Esc in interaction mode",
    );
    assert_eq!(
        nav.1, interact.1,
        "Ctrl+Z undid differently in the two modes"
    );
    assert!(
        nav.2 && interact.2,
        "`?` did not open the shortcuts in each mode: {} {}",
        nav.2,
        interact.2
    );
    check_at(&interact.4, &["Figure"], NAV, "Tab in interaction mode");
    check_at(&nav.4, &["Figure"], NAV, "Tab in navigation mode");
    rejects("a Tab in interaction mode captured by the element", || {
        check_at(&interact.3, &["Figure"], NAV, "Tab")
    });
    rejects("a `?` the element took", || assert!(nav.2 && !interact.2));
}

/// What `+`, `−` and K do on the Figure in `mode`: whether each acted (zoom: the basis scaled; K: locked).
fn figure_shortcuts(mode: KeyMode) -> [bool; 3] {
    let run = |key: Key| -> (Snapshot, Snapshot) {
        let mut rig = mock();
        rig.keys(&[Key::Tab, Key::Tab, Key::Tab]);
        check_at(&rig.at(), &["Figure"], NAV, "three Tabs");
        if mode == INTERACT {
            rig.key(Key::Enter, NONE);
        }
        assert_eq!(rig.app.view.focus.mode, mode);
        let f = layout().figure;
        let _ = rig.frame(vec![Event::PointerMoved(
            f.min + egui::vec2(f.width() * 0.3, f.height() * 0.6),
        )]);
        let before = rig.snap();
        rig.key(key, NONE);
        (before, rig.snap())
    };
    let zoomed = |(a, b): (Snapshot, Snapshot), inwards: bool| {
        let k = norm(&b.sim.plane.q1) / norm(&a.sim.plane.q1);
        (k - 1.0).abs() > 1e-9 && (k < 1.0) == inwards && b.sim.plane.z0 == a.sim.plane.z0
    };
    let (a, b) = run(Key::K);
    [
        zoomed(run(Key::Plus), true),
        zoomed(run(Key::Minus), false),
        !a.sim.lock.locked && b.sim.lock.locked,
    ]
}

#[test]
fn qa_mock_keyboard_modes_the_figure_shortcuts_act_in_either_mode_and_its_arrows_need_enter() {
    let nav = figure_shortcuts(NAV);
    let interact = figure_shortcuts(INTERACT);
    assert_eq!(
        nav, [true; 3],
        "+, − and K in navigation mode on the Figure"
    );
    assert_eq!(
        interact, [true; 3],
        "+, − and K in interaction mode on the Figure"
    );
    rejects("a Figure shortcut that acts in one mode only", || {
        assert_eq!([true, true, false], interact)
    });
    // Navigation mode: the arrows move to the next big scope and edit nothing.
    let mut rig = mock();
    rig.keys(&[Key::Tab, Key::Tab, Key::Tab]);
    let n0 = rig.applied();
    rig.key(Key::ArrowRight, NONE);
    check_at(&rig.at(), &["Trajectory"], NAV, "→ on the Figure");
    assert_eq!(
        rig.applied(),
        n0,
        "→ on the Figure in navigation mode applied a SetField"
    );
    rig.key(Key::ArrowLeft, NONE);
    check_at(&rig.at(), &["Figure"], NAV, "← back");
    // Enter on the Figure: interaction mode, where the arrows pan (one SetField on z₀ each).
    rig.key(Key::Enter, NONE);
    let entered = rig.at();
    check_at(&entered, &["Figure"], INTERACT, "Enter on the Figure");
    rejects("Enter on the Figure landing in navigation mode", || {
        check_at(&(entered.0.clone(), NAV), &["Figure"], INTERACT, "Enter")
    });
    for key in ARROWS {
        let (before, after) = one_set_field(&mut rig, &format!("{key:?} on the Figure"), |r| {
            r.key(key, NONE)
        });
        assert!(
            after.sim.plane.z0 != before.sim.plane.z0
                && (after.sim.plane.q1, after.sim.plane.q2)
                    == (before.sim.plane.q1, before.sim.plane.q2),
            "{key:?} in interaction mode on the Figure is not a pan"
        );
        check_at(&rig.at(), &["Figure"], INTERACT, "a pan");
    }
}

#[test]
fn qa_mock_keyboard_modes_enter_on_the_legend_does_nothing() {
    let mut rig = mock();
    for _ in 0..7 {
        rig.key(Key::Tab, NONE);
    }
    let landed = rig.at();
    check_at(&landed, &["Legend"], NAV, "seven Tabs");
    let (snap, n0, mode, windows, shown) = (
        rig.snap(),
        rig.applied(),
        rig.app.view.mode,
        rig.app.view.windows.open.clone(),
        (
            rig.app.shown,
            rig.app.console_open,
            rig.app.keyboard.shortcuts_open,
        ),
    );
    rig.key(Key::Enter, NONE);
    let after = rig.at();
    check_at(&after, &["Legend"], NAV, "Enter on the Legend");
    assert_eq!(rig.snap().sim, snap.sim, "Enter on the Legend edited");
    assert_eq!(rig.applied(), n0, "Enter on the Legend applied a SetField");
    assert_eq!(
        rig.app.view.mode, mode,
        "Enter on the Legend switched the mode"
    );
    assert_eq!(
        rig.app.view.windows.open, windows,
        "Enter on the Legend opened a window"
    );
    assert_eq!(
        (
            rig.app.shown,
            rig.app.console_open,
            rig.app.keyboard.shortcuts_open
        ),
        shown,
        "Enter on the Legend toggled the layer, the console or the shortcuts"
    );
    rejects("Enter on the Legend starting interaction mode", || {
        check_at(&(after.0.clone(), INTERACT), &["Legend"], NAV, "Enter")
    });
}

#[test]
fn qa_mock_keyboard_modes_the_compass_scope_and_enter_on_a_button() {
    let mut rig = mock();
    for _ in 0..5 {
        rig.key(Key::Tab, NONE);
    }
    check_at(&rig.at(), &["Compass"], NAV, "five Tabs");
    rig.key(Key::Enter, NONE);
    check_at(
        &rig.at(),
        &["Compass", "compass"],
        NAV,
        "Enter on the Compass",
    );
    // Enter on Slice and on Tilt: their clicks, staying in navigation mode on the button.
    rig.keys(&[Key::ArrowRight, Key::ArrowRight, Key::Enter]);
    check_at(&rig.at(), &["Compass", "Slice"], NAV, "Enter on Slice");
    check_mode(&rig.shapes(), "Slice", "Enter on Slice");
    rig.keys(&[Key::ArrowLeft, Key::Enter]);
    check_at(&rig.at(), &["Compass", "Tilt"], NAV, "Enter on Tilt");
    check_mode(&rig.shapes(), "Tilt", "Enter on Tilt");
    // A menu entry: Help › Keys (?) opens the shortcuts, the focus staying on it in navigation mode.
    let mut rig = mock();
    rig.keys(&[Key::Tab, Key::Enter]);
    for _ in 0..12 {
        if rig.at().0.last().is_some_and(|l| l == "Help") {
            break;
        }
        rig.key(Key::ArrowRight, NONE);
    }
    rig.key(Key::Enter, NONE);
    let entry = rig.at();
    assert!(
        entry.0.len() == 3 && entry.0[2].contains("Keys") && entry.1 == NAV,
        "Enter on Help: {entry:?}"
    );
    rig.key(Key::Enter, NONE);
    assert!(
        rig.app.keyboard.shortcuts_open,
        "Enter on Keys (?) did not act as its click"
    );
    // As its click does, the entry's click closes its menu: the focus stays on the entry or goes back to its menu,
    // in navigation mode either way (REQ-GUI-182: "Enter on a button act as its click and stay in navigation mode").
    let after = rig.at();
    assert_eq!(after.1, NAV, "Enter on a menu entry left navigation mode");
    assert!(
        after.0 == entry.0 || after.0 == entry.0[..2],
        "Enter on a menu entry moved the focus to {after:?}"
    );
}

// --- the ring's colour by mode ----------------------------------------------------------------------------------------

/// The hue (degrees), saturation and value of `c`.
fn hsv(c: Color32) -> (f32, f32, f32) {
    let [r, g, b, _] = c.to_array().map(|v| f32::from(v) / 255.0);
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let d = max - min;
    let h = if d == 0.0 {
        0.0
    } else if max == r {
        60.0 * ((g - b) / d).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    (h, if max == 0.0 { 0.0 } else { d / max }, max)
}

fn check_orange(c: Color32, what: &str) {
    let (h, s, v) = hsv(c);
    assert!(
        (15.0..=45.0).contains(&h) && s >= 0.5 && v >= 0.5,
        "{what}: {c:?} (hue {h}°, saturation {s}, value {v}) is not an orange"
    );
}

/// The ring's colours: the strokes, `RING_WIDTH` wide, of the rect shapes around the focused scope's place.
fn ring_colours(rig: &mut Rig) -> Vec<Color32> {
    let id = rig.app.view.focus.path.last().expect("a focus").clone();
    let place = rig
        .app
        .keyboard
        .place_of(&id)
        .expect("the focused scope drawn")
        .rect;
    rig.shapes()
        .into_iter()
        .filter_map(|s| match s {
            egui::Shape::Rect(r)
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

/// The breadcrumb's colours: the colours of the top bar's texts holding the separator.
fn breadcrumb_colours(rig: &mut Rig) -> Vec<Color32> {
    let top = layout().top_bar;
    rig.shapes()
        .into_iter()
        .filter_map(|s| match &s {
            egui::Shape::Text(t)
                if t.galley.text().contains('›') && top.contains_rect(s.visual_bounding_rect()) =>
            {
                Some(
                    t.galley
                        .job
                        .sections
                        .iter()
                        .map(|x| x.format.color)
                        .collect::<Vec<_>>(),
                )
            }
            _ => None,
        })
        .flatten()
        .collect()
}

#[test]
fn qa_mock_keyboard_modes_the_ring_is_orange_navigating_and_hyperlink_blue_interacting() {
    let mut rig = mock();
    let blue = rig.visuals().hyperlink_color;
    rig.focus("z_q1");
    let nav = ring_colours(&mut rig);
    let nav_crumb = breadcrumb_colours(&mut rig);
    rig.key(Key::Enter, NONE);
    assert_eq!(rig.app.view.focus.mode, INTERACT);
    let interact = ring_colours(&mut rig);
    let interact_crumb = breadcrumb_colours(&mut rig);
    assert_eq!(nav.len(), 1, "one ring navigating: {nav:?}");
    assert_eq!(interact.len(), 1, "one ring interacting: {interact:?}");
    check_orange(nav[0], "the ring in navigation mode");
    assert_eq!(
        interact[0], blue,
        "the ring in interaction mode is not hyperlink_color"
    );
    assert_eq!(
        nav[0],
        gui::explore::breadcrumb::NAVIGATION,
        "the ring in navigation mode is not the declared orange"
    );
    // The breadcrumb (R-409 A6, a look choice): drawn in one colour per mode, either the ring's or the blue kept.
    assert!(
        !nav_crumb.is_empty() && !interact_crumb.is_empty(),
        "no breadcrumb drawn"
    );
    assert!(
        nav_crumb.iter().all(|c| *c == nav[0] || *c == blue)
            && interact_crumb.iter().all(|c| *c == blue),
        "the breadcrumb's colours {nav_crumb:?} / {interact_crumb:?}"
    );
    // Controls: a ring of one colour in both modes; the blue taken for the orange.
    rejects("a ring of one colour in both modes", || {
        check_orange(interact[0], "the blue ring")
    });
    rejects("a ring of one colour in both modes", || {
        assert_eq!(nav[0], blue)
    });
}

// --- on the GPU: the two modes differ only in the ring's colour and the breadcrumb's ----------------------------------

fn canvas() -> Arc<MockCanvas> {
    Arc::new(MockCanvas::new().expect("a GPU adapter for the mock's canvas"))
}

fn pixel(shot: &Shot, x: usize, y: usize) -> [u8; 4] {
    let i = (y * shot.size[0] as usize + x) * 4;
    shot.rgba[i..i + 4].try_into().unwrap()
}

fn diff(a: &Shot, b: &Shot) -> Vec<(usize, usize)> {
    let (w, h) = (SIZE[0] as usize, SIZE[1] as usize);
    (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .filter(|&(x, y)| pixel(a, x, y) != pixel(b, x, y))
        .collect()
}

/// Every pixel where `a` and `b` differ is either on the outline of the differing region below the top bar (a ring:
/// within `band` pixels of that region's bounding box's border) or in the top bar on a breadcrumb text (a name holding
/// `›` or the keyboard glyph). Returns the ring's bounding box in pixels and the counts.
fn check_ring_and_crumb_only(a: &Shot, b: &Shot) -> (Rect, usize, usize) {
    let p = PIXELS_PER_POINT;
    let top = layout().top_bar;
    let top_px = top.max.y * p;
    let crumbs: Vec<[f64; 4]> = [a, b]
        .iter()
        .flat_map(|s| s.names.iter())
        .filter(|n| n.name.contains('›') || n.name == gui::explore::breadcrumb::GLYPH)
        .filter_map(|n| n.rect)
        .filter(|r| r[3] <= f64::from(top_px) + 0.5)
        .collect();
    let d = diff(a, b);
    let (below, above): (Vec<_>, Vec<_>) = d.into_iter().partition(|&(_, y)| y as f32 >= top_px);
    let bbox = below.iter().fold(Rect::NOTHING, |r, &(x, y)| {
        r.union(Rect::from_min_max(
            egui::pos2(x as f32, y as f32),
            egui::pos2(x as f32 + 1.0, y as f32 + 1.0),
        ))
    });
    let band = 4.0 * p;
    for &(x, y) in &below {
        let (x, y) = (x as f32 + 0.5, y as f32 + 0.5);
        let inner = bbox.shrink(band);
        assert!(
            !inner.contains(egui::pos2(x, y)),
            "the screen changed at ({x}, {y}), inside the ring's box {bbox:?}: not a ring"
        );
    }
    for &(x, y) in &above {
        let (xf, yf) = (x as f64 + 0.5, y as f64 + 0.5);
        assert!(
            crumbs.iter().any(|c| xf >= c[0] - 2.0
                && xf <= c[2] + 2.0
                && yf >= c[1] - 2.0
                && yf <= c[3] + 2.0),
            "the top bar changed at ({x}, {y}), off the breadcrumb"
        );
    }
    (bbox, below.len(), above.len())
}

#[test]
fn qa_capture_navigation_and_interaction_differ_only_in_the_colours() {
    let canvas = canvas();
    // Centre z₀'s first slider, as the 07_keyboard cases reach it: navigation mode, then Enter.
    let steps = [
        Step::Tab,
        Step::Tab,
        Step::Enter,
        Step::ArrowDown,
        Step::ArrowDown,
        Step::Enter,
    ];
    let nav = capture::shoot(canvas.clone(), &steps);
    let mut more = steps.to_vec();
    more.push(Step::Enter);
    let interact = capture::shoot(canvas.clone(), &more);
    let (bbox, ring, _) = check_ring_and_crumb_only(&nav, &interact);
    assert!(ring > 0, "the ring did not change colour");
    // The ring is around a slider of Centre z₀, in the Manifold view panel.
    let mv = layout().manifold_view;
    let p = PIXELS_PER_POINT;
    let panel = Rect::from_min_max(mv.min * p, mv.max * p).expand(4.0 * p);
    assert!(
        panel.contains_rect(bbox),
        "the ring {bbox:?} is not in the Manifold view {panel:?}"
    );
    assert!(
        bbox.height() < 0.25 * panel.height(),
        "the ring {bbox:?} is not a slider's"
    );
    // The changed ring pixels are orange-ish in navigation mode and blue-ish in interaction mode.
    let (mut warm, mut cool) = (0, 0);
    for (x, y) in diff(&nav, &interact) {
        if (y as f32) < layout().top_bar.max.y * p {
            continue;
        }
        let [r, _, b, _] = pixel(&nav, x, y);
        let [r2, _, b2, _] = pixel(&interact, x, y);
        warm += usize::from(r > b);
        cool += usize::from(b2 > r2);
    }
    assert!(
        warm > 0 && cool > 0,
        "the ring's pixels: {warm} warm navigating, {cool} cool interacting"
    );
    // Control: the shortcuts overlay is not a change of colour alone.
    let mut overlay = steps.to_vec();
    overlay.push(Step::Shortcuts);
    let overlay = capture::shoot(canvas.clone(), &overlay);
    rejects("the shortcuts overlay as a change of mode", || {
        check_ring_and_crumb_only(&nav, &overlay);
    });
}

// === REQ-GUI-183: the compass from the keyboard ======================================================================

#[test]
fn qa_mock_compass_keys_the_compass_tilt_and_slice_are_siblings_in_that_order() {
    let mut rig = mock();
    for _ in 0..5 {
        rig.key(Key::Tab, NONE);
    }
    check_at(&rig.at(), &["Compass"], NAV, "Tab onto the Compass");
    rig.key(Key::Enter, NONE);
    let mut seen = vec![rig.at()];
    for key in [
        Key::ArrowRight,
        Key::ArrowRight,
        Key::ArrowLeft,
        Key::ArrowLeft,
    ] {
        rig.key(key, NONE);
        seen.push(rig.at());
    }
    let want = ["compass", "Tilt", "Slice", "Tilt", "compass"];
    for (got, w) in seen.iter().zip(want) {
        check_at(got, &["Compass", w], NAV, "the Compass's siblings");
    }
    rejects("→ from the compass reaching Slice", || {
        check_at(&seen[1], &["Compass", "Slice"], NAV, "→")
    });
    // Enter on Slice and on Tilt set the mode, staying in navigation mode; Enter on the compass starts interaction and
    // Esc returns to navigation mode on it.
    rig.keys(&[Key::ArrowRight, Key::ArrowRight, Key::Enter]);
    check_mode(&rig.shapes(), "Slice", "Enter on Slice");
    check_at(&rig.at(), &["Compass", "Slice"], NAV, "Enter on Slice");
    rig.keys(&[Key::ArrowLeft, Key::Enter]);
    check_mode(&rig.shapes(), "Tilt", "Enter on Tilt");
    check_at(&rig.at(), &["Compass", "Tilt"], NAV, "Enter on Tilt");
    rig.keys(&[Key::ArrowLeft, Key::Enter]);
    check_at(
        &rig.at(),
        &["Compass", "compass"],
        INTERACT,
        "Enter on the compass",
    );
    rig.key(Key::Escape, NONE);
    check_at(
        &rig.at(),
        &["Compass", "compass"],
        NAV,
        "Esc on the compass",
    );
    // Control: an arrow on the compass in navigation mode moves the focus and tilts nothing.
    let n0 = rig.applied();
    rig.key(Key::ArrowRight, NONE);
    check_at(
        &rig.at(),
        &["Compass", "Tilt"],
        NAV,
        "→ on the compass in navigation mode",
    );
    assert_eq!(
        rig.applied(),
        n0,
        "→ on the compass in navigation mode applied a SetField"
    );
}

/// The signed tilt of `q` out of the chart's plane span(`e_a`, `e_b`) toward `e_d`, in degrees.
fn signed_tilt(q: &Z, frame: &[Z]) -> f64 {
    let inside = dot(q, &frame[0]).hypot(dot(q, &frame[1]));
    dot(q, &frame[2]).atan2(inside).to_degrees()
}

/// One press of `key` with `modifiers` on the compass in interaction mode, in `mode` ("Tilt" or "Slice"), from the
/// free start; the snapshots before and after.
fn compass_press(mode: &str, key: Key, modifiers: Modifiers) -> (Snapshot, Snapshot) {
    let mut rig = mock();
    rig.focus(mode);
    rig.key(Key::Enter, NONE);
    rig.focus("compass");
    rig.key(Key::Enter, NONE);
    assert_eq!(rig.app.view.focus.mode, INTERACT);
    check_mode(&rig.shapes(), mode, "before the press");
    one_set_field(
        &mut rig,
        &format!("{key:?} on the compass in {mode}"),
        |r| r.key(key, modifiers),
    )
}

#[test]
fn qa_mock_compass_keys_tilt_mode_arrows_tilt_by_the_tilt_step() {
    // R-404's confirmed Tilt step: one degree.
    let Base::Degrees(base) = StepKind::Tilt.base() else {
        panic!("the Tilt step is in degrees")
    };
    assert_eq!(base, 1.0, "R-404 confirms the Tilt step at 1°");
    let frame = compass_frame();
    for (modifiers, times) in [(NONE, 1.0), (SHIFT, 10.0), (ALT, 0.1)] {
        let mut by_key = Vec::new();
        for key in ARROWS {
            let (a, b) = compass_press("Tilt", key, modifiers);
            check_tilt(&a, &b, &format!("{key:?}"));
            let d1 = signed_tilt(&b.sim.plane.q1, &frame) - signed_tilt(&a.sim.plane.q1, &frame);
            let d2 = signed_tilt(&b.sim.plane.q2, &frame) - signed_tilt(&a.sim.plane.q2, &frame);
            by_key.push((d1, d2));
        }
        let want = base * times;
        let what = format!("{modifiers:?}");
        let (l, r, u, d) = (by_key[0], by_key[1], by_key[2], by_key[3]);
        for ((t1, t2), name, on_tau1) in [
            (l, "←", true),
            (r, "→", true),
            (u, "↑", false),
            (d, "↓", false),
        ] {
            let (moved, still) = if on_tau1 { (t1, t2) } else { (t2, t1) };
            assert!(
                (moved.abs() - want).abs() < 1e-9 && still.abs() < 1e-9,
                "{what} {name}: τ₁ by {t1}°, τ₂ by {t2}°; want {want}° on {}",
                if on_tau1 { "τ₁" } else { "τ₂" }
            );
        }
        assert!(
            l.0 * r.0 < 0.0 && u.1 * d.1 < 0.0,
            "{what}: opposite arrows tilt the same way"
        );
    }
    // Controls: a Shift step smaller than the arrow's; a slice-mode arrow that tilts.
    let (a, b) = compass_press("Tilt", Key::ArrowRight, SHIFT);
    let shift = (signed_tilt(&b.sim.plane.q1, &frame) - signed_tilt(&a.sim.plane.q1, &frame)).abs();
    rejects(
        "Shift+arrows that give a step smaller than the arrow's",
        || assert!(shift < base),
    );
    rejects("a slice-mode arrow that tilts", || {
        check_along_normal(&a, &b, &frame, "a tilt");
    });
}

#[test]
fn qa_mock_compass_keys_slice_mode_arrows_move_along_the_normal_by_the_slice_step() {
    // REQ-GUI-184's step, as declared (a calibration pending the M8 gate): a fraction of the slice step's range.
    let Base::OfRange(fraction) = StepKind::SliceNormal.base() else {
        panic!("REQ-GUI-184's step is declared as a fraction of the slice step's range")
    };
    let range = 2.0 * gui::explore::manifold_view::Z0_HALF_RANGE;
    // The slice step's range, read back: its own ↑ moves z₀ by its base step of that range.
    let Base::OfRange(slider) = StepKind::Bounded.base() else {
        panic!()
    };
    {
        let mut rig = mock();
        rig.interact("slice step");
        let before = rig.snap().sim.plane.z0;
        rig.key(Key::ArrowUp, NONE);
        let moved = norm(&add(&rig.snap().sim.plane.z0, -1.0, &before));
        assert!(
            (moved - slider * range).abs() < 1e-12,
            "the slice step's range is not {range}"
        );
    }
    let frame = compass_frame();
    for (modifiers, times) in [(NONE, 1.0), (SHIFT, 10.0), (ALT, 0.1)] {
        let mut moves = Vec::new();
        for key in ARROWS {
            let (a, b) = compass_press("Slice", key, modifiers);
            let d =
                check_along_normal(&a, &b, &frame, &format!("{key:?} ({modifiers:?}) in Slice"));
            let want = fraction * range * times;
            assert!(
                (norm(&d) - want).abs() < 1e-12,
                "{key:?} ({modifiers:?}) moved the slice {}, REQ-GUI-184's step × {times} is {want}",
                norm(&d)
            );
            moves.push(d);
        }
        assert!(
            dot(&moves[0], &moves[1]) < 0.0 && dot(&moves[2], &moves[3]) < 0.0,
            "({modifiers:?}) opposite arrows move the slice the same way"
        );
    }
    // On a tilted plane the arrows follow its normal, not the depth axis.
    let mut rig = tilted();
    rig.focus("Slice");
    rig.key(Key::Enter, NONE);
    rig.focus("compass");
    rig.key(Key::Enter, NONE);
    let (a, b) = one_set_field(&mut rig, "↑ on the tilted compass in Slice", |r| {
        r.key(Key::ArrowUp, NONE)
    });
    let d = check_along_normal(&a, &b, &frame, "↑ on the tilted compass");
    assert!(
        dot(&unit(&d), &frame[2]).abs() < 1.0 - 1e-6,
        "the move is along the depth axis"
    );
    // Control: a slice-mode arrow that tilts.
    let (ta, tb) = compass_press("Tilt", Key::ArrowUp, NONE);
    rejects("a slice-mode arrow that tilts", || {
        check_along_normal(&ta, &tb, &frame, "a tilt");
    });
}
