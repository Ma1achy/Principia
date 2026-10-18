//! QA tests for TASK-M6-36, written from the requirements and R-453, not from the implementation:
//!
//! - REQ-GUI-201 (R-453 D1 to D3, D5, D6; render_gui_spec §G2, §G3): a primary click on the figure with no drag pins
//!   that sample in the Trajectory panel, whose first tab reads "pinned"; hovering other points, entering the panel and
//!   the pointer leaving the window keep it; a pan, a zoom and a slice edit keep it, the pin holding the IC, never a
//!   pixel; another click moves it; unpin returns the tab to "under cursor", which follows the next hover; unpinned, the
//!   panel keeps the last sample hovered when the pointer leaves the figure; a drag on the figure pans and does not
//!   pin; P pins under the pointer in navigation and in interaction mode on the Figure, and does nothing off the
//!   figure; pinned, the figure's path and the hover label follow the pointer (REQ-GUI-085); "+" clicked keeps the
//!   panel's sample; the pin is ViewUI state, no SetField on SimConfig and nothing in undo; a sim-key change
//!   re-integrates the pinned IC (D5).
//! - REQ-GUI-202 (R-453 D6 to D8; R-404's `Pan`, `Angle`, `ZoomLog2`; R-409): on real space and the unwrapped view a
//!   drag moves the drawn path by the drag, the wheel scales it about the pointer; on the sphere a drag orbits (yaw,
//!   pitch clamped at ±90°) and unticks "turn", which ticked again resumes from the view as it stands; the wheel scales
//!   the sphere; a double-click restores each default; the data's (θ, φ) and the unwrapped drawing are unchanged by
//!   an orbit; from the keyboard each view is an element of the Trajectory scope, Enter starting interaction, the
//!   arrows panning by 1/20 of the view's span or turning by 1°, + and − zooming by a quarter octave, Shift ×10 and Alt
//!   ×0.1, Esc returning to navigation mode, the unwrapped view in the sphere's place taking real space's keys; no
//!   interaction sends a SetField or enters undo.
//!
//! What is drawn is read from egui's shapes, by the clip rect of each view (the view's painter); the pin and the
//! views' state are read from the panel's `ViewUI` scratch, the snapshot's sample and SetFields from the contract's own
//! log (gui_state_contract §2). gui takes no dependency on validation (R-187), so each check runs its own control
//! (R-176) through [`rejects`].
// The file name `qa_TASK-M6-36` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use eframe::egui::{self, Event, Key, Modifiers, PointerButton, Pos2, Rect};
use engine::contract::log::Source;
use engine::contract::sample::Sample;
use engine::contract::set_field::{Edit, SetField, SimField};
use engine::contract::sim_config::Horizon;
use engine::contract::snapshot::Snapshot;
use engine::contract::view_ui::{KeyMode, Mode};
use gui::app::App;
use gui::capture::{FORMAT, PIXELS_PER_POINT, SIZE};
use gui::explore::trajectory::{self as traj, areas};
use gui::headless::{Headless, FRAME_S};
use gui::layout::Layout;
use gui::mock::MockEngine;
use gui::side::{EngineSide, MockSide};

const NONE: Modifiers = Modifiers::NONE;
const SHIFT: Modifiers = Modifiers::SHIFT;
const ALT: Modifiers = Modifiers::ALT;
const NAV: KeyMode = KeyMode::Navigation;
const INTERACT: KeyMode = KeyMode::Interaction;

/// Three figure points, `(s, t)`, apart from each other.
const A: [f32; 2] = [0.3, 0.7];
const B: [f32; 2] = [0.7, 0.25];
const C: [f32; 2] = [0.55, 0.5];

/// Asserts that `check` panics: the check can fail (R-176's control, run in the test).
fn rejects(what: &str, check: impl FnOnce()) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check));
    assert!(
        outcome.is_err(),
        "the check passed on {what}, which it must reject"
    );
}

// --- the rig --------------------------------------------------------------------------------------------------------

fn layout() -> Layout {
    Layout::new(
        Headless::new(SIZE, PIXELS_PER_POINT).screen(),
        PIXELS_PER_POINT,
    )
}

struct Rig {
    app: App<MockSide>,
    h: Headless,
}

impl Rig {
    /// The mock, its clock frozen (as capture mode runs it), with no canvas.
    fn new() -> Self {
        let mut rig = Self {
            app: App::new(MockSide::new(MockEngine::frozen(), None), FORMAT),
            h: Headless::new(SIZE, PIXELS_PER_POINT),
        };
        rig.idle(2);
        rig
    }

    /// The mock, its clock running one tick a frame.
    fn running() -> Self {
        let mut rig = Self {
            app: App::new(MockSide::new(MockEngine::new(), None), FORMAT),
            h: Headless::new(SIZE, PIXELS_PER_POINT),
        };
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

    fn snap(&self) -> Snapshot {
        self.app.snapshot().clone()
    }

    /// The `contract` log entries so far: one per SetField applied.
    fn contract(&self) -> Vec<String> {
        self.app
            .console()
            .iter()
            .filter(|e| e.source == Source::Contract)
            .map(|e| e.message.clone())
            .collect()
    }

    fn undo_depth(&self) -> u32 {
        self.app.snapshot().history.undo_depth
    }

    /// Puts the focus on the scope `id` in `mode`.
    fn focus(&mut self, id: &str, mode: KeyMode) {
        let path = self.app.keyboard.tree(Mode::Explore).path_to(id);
        assert!(!path.is_empty(), "no scope {id} in the tree");
        self.app.view.focus.path = path;
        self.app.view.focus.mode = mode;
    }

    fn focused(&self) -> Option<String> {
        self.app.view.focus.path.last().cloned()
    }

    /// The rect of the control `id`, where it was last drawn.
    fn place(&mut self, id: &str) -> Rect {
        self.idle(1);
        self.app
            .keyboard
            .place_of(id)
            .unwrap_or_else(|| panic!("the control {id} was not drawn"))
            .rect
    }

    fn press(&mut self, at: Pos2, pressed: bool) {
        let _ = self.frame(vec![Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed,
            modifiers: NONE,
        }]);
    }

    fn click(&mut self, at: Pos2) {
        let _ = self.frame(vec![Event::PointerMoved(at)]);
        self.press(at, true);
        self.press(at, false);
        self.idle(2);
    }

    /// A double-click at `at`: two clicks well within egui's double-click delay.
    fn double_click(&mut self, at: Pos2) {
        self.h.set_frame_step(0.02);
        let _ = self.frame(vec![Event::PointerMoved(at)]);
        for _ in 0..2 {
            self.press(at, true);
            self.press(at, false);
        }
        self.h.set_frame_step(FRAME_S);
        self.idle(2);
    }

    /// A primary drag from `from` to `to` in `steps` equal moves.
    fn drag(&mut self, from: Pos2, to: Pos2, steps: usize) {
        let _ = self.frame(vec![Event::PointerMoved(from)]);
        self.press(from, true);
        for i in 1..=steps {
            let at = from + (to - from) * (i as f32 / steps as f32);
            let _ = self.frame(vec![Event::PointerMoved(at)]);
        }
        self.press(to, false);
        self.idle(2);
    }

    fn wheel(&mut self, at: Pos2, lines: f32) {
        let _ = self.frame(vec![Event::PointerMoved(at)]);
        let _ = self.frame(vec![Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, lines),
            phase: egui::TouchPhase::Move,
            modifiers: NONE,
        }]);
        self.idle(2);
    }

    fn pointer_to(&mut self, at: Pos2) {
        let _ = self.frame(vec![Event::PointerMoved(at)]);
        self.idle(2);
    }

    fn pointer_gone(&mut self) {
        let _ = self.frame(vec![Event::PointerGone]);
        self.idle(2);
    }

    /// The shapes drawn this frame, flattened, each with its clip rect.
    fn shapes(&mut self) -> Vec<(Rect, egui::Shape)> {
        let out = self.frame(Vec::new());
        let mut flat = Vec::new();
        for clipped in &out.shapes {
            flatten(clipped.clip_rect, &clipped.shape, &mut flat);
        }
        flat
    }

    /// The texts drawn inside `rect`.
    fn texts_in(&mut self, rect: Rect) -> Vec<String> {
        self.shapes()
            .into_iter()
            .filter_map(|(_, s)| match &s {
                egui::Shape::Text(t)
                    if rect.expand(1.0).contains_rect(s.visual_bounding_rect()) =>
                {
                    Some(t.galley.text().to_owned())
                }
                _ => None,
            })
            .collect()
    }

    /// The mock's marks: the hover path it draws into its stand-in.
    fn marks(&mut self) -> Vec<([[f32; 2]; 2], [f32; 4])> {
        self.app
            .side()
            .engine()
            .marks()
            .into_iter()
            .map(|l| (l.ends, l.rgba))
            .collect()
    }

    fn pin_sample(&self) -> Option<Sample> {
        self.app.trajectory.pin.as_ref().map(|p| p.sample.clone())
    }

    fn first(&self) -> Option<Sample> {
        self.app.trajectory.first().cloned()
    }
}

fn flatten(clip: Rect, shape: &egui::Shape, out: &mut Vec<(Rect, egui::Shape)>) {
    match shape {
        egui::Shape::Vec(v) => v.iter().for_each(|s| flatten(clip, s, out)),
        other => out.push((clip, other.clone())),
    }
}

/// The point of the figure at the chart's `(s, t)`, `t` Y-up (chart_decoder_contract Part 3).
fn fig([s, t]: [f32; 2]) -> Pos2 {
    let f = layout().figure;
    egui::pos2(f.min.x + f.width() * s, f.max.y - f.height() * t)
}

/// One figure pixel, in chart units.
fn pixel() -> f64 {
    1.0 / f64::from(layout().figure.width() * PIXELS_PER_POINT)
}

fn near(at: [f64; 2], want: [f32; 2]) -> bool {
    (at[0] - f64::from(want[0])).abs() <= pixel() && (at[1] - f64::from(want[1])).abs() <= pixel()
}

fn same_rect(a: Rect, b: Rect) -> bool {
    (a.min - b.min).length() < 0.5 && (a.max - b.max).length() < 0.5
}

fn panel_text(rig: &mut Rig) -> String {
    rig.texts_in(layout().trajectory).join(" | ")
}

fn figure_texts(rig: &mut Rig) -> Vec<String> {
    let mut t = rig.texts_in(layout().figure);
    t.sort();
    t
}

// --- REQ-GUI-201: the pin -------------------------------------------------------------------------------------------

/// The pin holds the sample picked at `want`: the first tab reads "pinned" and the pin is that sample.
fn check_pinned(pin: Option<&Sample>, tab_texts: &[String], want: [f32; 2]) {
    let pin = pin.expect("no pin is held");
    assert!(
        near(pin.at, want),
        "the pin is at {:?}, not the sample picked at {want:?}",
        pin.at
    );
    assert!(
        tab_texts.iter().any(|t| t == traj::PINNED),
        "the first tab does not read \"pinned\": {tab_texts:?}"
    );
}

/// The panel shows `want`'s summary, and is not blank.
fn check_shows(panel: &str, want: &Sample) {
    assert!(
        !panel.contains(traj::NO_SAMPLE),
        "the panel went blank: {panel}"
    );
    assert!(
        panel.contains(&traj::summary(want)),
        "the panel does not show the sample at {:?}: {panel}",
        want.at
    );
}

/// The pin still holds the IC it was given (R-453 D1): not the IC now at its pixel.
fn check_holds_ic(before: &Sample, after: &Sample) {
    assert_eq!(
        after.ic, before.ic,
        "the pin's IC changed: it held a pixel, not the IC"
    );
}

fn title_texts(rig: &mut Rig) -> Vec<String> {
    rig.texts_in(areas(layout().trajectory).title)
}

#[test]
fn qa_trajectory_pin_a_click_pins_and_hover_never_replaces_it() {
    let mut rig = Rig::new();
    let (u0, n0) = (rig.undo_depth(), rig.contract().len());
    rig.pointer_to(fig(A));
    let a = rig.snap().sample.expect("no sample under the cursor at A");
    assert!(rig.app.trajectory.pin.is_none(), "a hover pinned");
    rig.click(fig(A));
    let pinned_a = rig.pin_sample();
    let tabs = title_texts(&mut rig);
    check_pinned(pinned_a.as_ref(), &tabs, A);
    assert_eq!(pinned_a.as_ref().unwrap().ic, a.ic, "the pin is not A's IC");
    let shown_a = panel_text(&mut rig);
    check_shows(&shown_a, &a);
    // Hover elsewhere on the figure: the snapshot follows the pointer, the pin does not.
    rig.pointer_to(fig(B));
    let b = rig.snap().sample.expect("no sample under the cursor at B");
    assert!(near(b.at, B), "the hover did not move to B");
    assert_ne!(
        traj::summary(&b),
        traj::summary(&a),
        "A and B read alike, so the panel cannot tell them apart"
    );
    let tabs = title_texts(&mut rig);
    check_pinned(rig.pin_sample().as_ref(), &tabs, A);
    let panel = panel_text(&mut rig);
    check_shows(&panel, &a);
    // Into the panel, then out of the window.
    rig.pointer_to(layout().trajectory.center());
    let tabs = title_texts(&mut rig);
    check_pinned(rig.pin_sample().as_ref(), &tabs, A);
    check_shows(&panel_text(&mut rig), &a);
    rig.pointer_gone();
    let tabs = title_texts(&mut rig);
    check_pinned(rig.pin_sample().as_ref(), &tabs, A);
    check_shows(&panel_text(&mut rig), &a);
    // Another click moves it.
    rig.click(fig(C));
    let c = rig.snap().sample.expect("no sample under the cursor at C");
    let tabs = title_texts(&mut rig);
    check_pinned(rig.pin_sample().as_ref(), &tabs, C);
    check_shows(&panel_text(&mut rig), &c);
    // Nothing entered undo; every SetField was the cursor's, no history (R-453 D5).
    assert_eq!(rig.undo_depth(), u0, "pinning entered undo");
    for e in &rig.contract()[n0..] {
        assert!(
            e.starts_with("SetField Hover.at ") && e.ends_with(" (no history)"),
            "pinning applied `{e}`, not only the cursor's no-history SetField"
        );
    }
    // Controls: a hover that replaced the pin; a panel blanked; a first tab still reading "under cursor".
    let b_pin = Some(b.clone());
    rejects("a hover that replaced the pin", || {
        check_pinned(b_pin.as_ref(), &[traj::PINNED.to_owned()], A)
    });
    rejects("a panel blanked off the figure", || {
        check_shows(traj::NO_SAMPLE, &a)
    });
    rejects("a panel showing the hovered sample", || {
        check_shows(&traj::summary(&b), &a)
    });
    rejects("a first tab reading under cursor", || {
        check_pinned(pinned_a.as_ref(), &[traj::FOLLOWING.to_owned()], A)
    });
}

#[test]
fn qa_trajectory_pin_holds_the_ic_through_pan_zoom_and_slice() {
    let mut rig = Rig::new();
    rig.click(fig(A));
    let before = rig.pin_sample().expect("the click did not pin");
    let f = layout().figure;
    // A pan: one drag on the figure, which pans (a SetField on z₀) and does not move the pin.
    let z0 = rig.snap().sim.plane.z0;
    rig.drag(fig(B), fig(B) + egui::vec2(60.0, -40.0), 6);
    assert_ne!(rig.snap().sim.plane.z0, z0, "the drag did not pan");
    let after = rig.pin_sample().expect("the pan cleared the pin");
    check_holds_ic(&before, &after);
    // The control: the IC now at the pin's pixel, which the pan moved.
    let plane = rig.snap().sim.plane;
    let pixel_ic = gui::explore::manifold_view::point(&plane, before.at[0], before.at[1]);
    let at_pixel = rig
        .app
        .side()
        .compute_ic(&pixel_ic, before.at)
        .expect("the mock has computeIC");
    assert_ne!(at_pixel.ic, before.ic, "the pan moved nothing");
    rejects("a pin that held a pixel through a pan", || {
        check_holds_ic(&before, &at_pixel)
    });
    // A zoom: the wheel on the figure.
    let q1 = rig.snap().sim.plane.q1;
    rig.wheel(f.center(), 2.0);
    assert_ne!(rig.snap().sim.plane.q1, q1, "the wheel did not zoom");
    check_holds_ic(
        &before,
        &rig.pin_sample().expect("the zoom cleared the pin"),
    );
    // A slice edit: z₀ moved.
    let mut z0 = rig.snap().sim.plane.z0;
    z0[7] += 0.1;
    rig.app.set_field(SetField {
        edit: Edit::Sim(SimField::Z0(z0)),
        no_history: false,
    });
    rig.idle(3);
    assert_eq!(
        rig.snap().sim.plane.z0,
        z0,
        "the slice edit was not applied"
    );
    check_holds_ic(
        &before,
        &rig.pin_sample().expect("the slice edit cleared the pin"),
    );
    let tabs = title_texts(&mut rig);
    assert!(
        tabs.iter().any(|t| t == traj::PINNED),
        "after a pan, a zoom and a slice edit the tab reads {tabs:?}"
    );
}

#[test]
fn qa_trajectory_pin_a_drag_pans_and_does_not_pin() {
    let mut rig = Rig::new();
    let z0 = rig.snap().sim.plane.z0;
    rig.drag(fig(A), fig(A) + egui::vec2(50.0, 30.0), 5);
    assert_ne!(rig.snap().sim.plane.z0, z0, "the drag did not pan");
    assert!(rig.app.trajectory.pin.is_none(), "a drag pinned");
    let tabs = title_texts(&mut rig);
    assert!(
        tabs.iter().any(|t| t == traj::FOLLOWING),
        "after a drag the first tab reads {tabs:?}"
    );
    // A click does pin, the control on the same rig.
    rig.click(fig(A));
    assert!(rig.app.trajectory.pin.is_some(), "a click did not pin");
}

#[test]
fn qa_trajectory_pin_unpin_follows_the_cursor_again() {
    let mut rig = Rig::new();
    rig.click(fig(A));
    assert!(rig.app.trajectory.pin.is_some(), "the click did not pin");
    let unpin = rig.place(traj::UNPIN);
    let n0 = rig.contract().len();
    let u0 = rig.undo_depth();
    rig.click(unpin.center());
    assert!(rig.app.trajectory.pin.is_none(), "unpin left the pin");
    let tabs = title_texts(&mut rig);
    assert!(
        tabs.iter().any(|t| t == traj::FOLLOWING) && !tabs.iter().any(|t| t == traj::PINNED),
        "after unpin the first tab reads {tabs:?}"
    );
    assert_eq!(rig.undo_depth(), u0, "unpin entered undo");
    assert!(
        rig.contract()[n0..]
            .iter()
            .all(|e| e.starts_with("SetField Hover.at ")),
        "unpin applied a SetField"
    );
    // It follows the next hover.
    rig.pointer_to(fig(B));
    let b = rig.snap().sample.expect("no sample at B");
    let first = rig.first().expect("the first tab shows nothing");
    assert!(
        near(first.at, B),
        "after unpin the tab did not follow the hover to B"
    );
    check_shows(&panel_text(&mut rig), &b);
    // Enter on unpin, as its click (R-409 A1).
    rig.click(fig(C));
    assert!(rig.app.trajectory.pin.is_some(), "the click did not pin");
    rig.idle(1);
    rig.focus(traj::UNPIN, NAV);
    rig.key(Key::Enter, NONE);
    assert!(
        rig.app.trajectory.pin.is_none(),
        "Enter on unpin left the pin"
    );
}

#[test]
fn qa_trajectory_pin_unpinned_the_panel_keeps_the_last_hover() {
    let mut rig = Rig::new();
    // Before any hover the panel says what to do.
    rig.pointer_gone();
    assert!(
        panel_text(&mut rig).contains(traj::NO_SAMPLE),
        "before any hover the panel does not say \"{}\"",
        traj::NO_SAMPLE
    );
    rig.pointer_to(fig(A));
    let a = rig.snap().sample.expect("no sample at A");
    check_shows(&panel_text(&mut rig), &a);
    // Onto the panel: the hover ends, the panel keeps A.
    rig.pointer_to(layout().trajectory.center());
    assert!(rig.snap().sample.is_none(), "the hover outlived the figure");
    check_shows(&panel_text(&mut rig), &a);
    // Elsewhere off the figure, and off the window.
    rig.pointer_to(layout().footer.center());
    check_shows(&panel_text(&mut rig), &a);
    rig.pointer_gone();
    check_shows(&panel_text(&mut rig), &a);
    // Until another point of the figure is hovered.
    rig.pointer_to(fig(B));
    let b = rig.snap().sample.expect("no sample at B");
    check_shows(&panel_text(&mut rig), &b);
    rig.pointer_to(layout().trajectory.center());
    check_shows(&panel_text(&mut rig), &b);
    assert!(rig.app.trajectory.pin.is_none(), "a hover pinned");
}

/// P pinned in both modes: `nav` and `interact`, each whether P pinned in that mode.
fn check_both_modes(nav: bool, interact: bool) {
    assert!(nav, "P did not pin in navigation mode");
    assert!(interact, "P did not pin in interaction mode");
}

#[test]
fn qa_trajectory_pin_p_pins_under_the_pointer_in_either_mode() {
    let mut rig = Rig::new();
    let u0 = rig.undo_depth();
    let mut pinned = Vec::new();
    for (mode, at) in [(NAV, A), (INTERACT, B)] {
        rig.app.trajectory.pin = None;
        rig.pointer_to(fig(at));
        rig.focus(gui::explore::FIGURE, mode);
        rig.key(Key::P, NONE);
        let ok = rig.pin_sample().is_some_and(|p| near(p.at, at));
        assert_eq!(
            rig.app.view.focus.mode, mode,
            "P changed the keyboard's mode"
        );
        pinned.push(ok);
    }
    check_both_modes(pinned[0], pinned[1]);
    rejects("a P that pins in navigation mode only", || {
        check_both_modes(true, false)
    });
    rejects("a P that pins in interaction mode only", || {
        check_both_modes(false, true)
    });
    // Off the figure P does nothing: no pin made, none moved.
    for mode in [NAV, INTERACT] {
        rig.app.trajectory.pin = None;
        rig.pointer_to(fig(A));
        rig.pointer_to(layout().trajectory.center());
        rig.focus(gui::explore::FIGURE, mode);
        rig.key(Key::P, NONE);
        assert!(
            rig.app.trajectory.pin.is_none(),
            "P off the figure pinned, in {mode:?}"
        );
        rig.click(fig(C));
        let held = rig.pin_sample().expect("the click did not pin");
        rig.pointer_to(layout().footer.center());
        rig.focus(gui::explore::FIGURE, mode);
        rig.key(Key::P, NONE);
        assert_eq!(
            rig.pin_sample().map(|p| p.ic),
            Some(held.ic),
            "P off the figure moved the pin, in {mode:?}"
        );
    }
    assert_eq!(rig.undo_depth(), u0, "P entered undo");
    // P is listed with the Figure's keys in the ? overlay (§G3).
    let rows = rig.app.keyboard.tree(Mode::Explore).shortcuts();
    assert!(
        rows.iter()
            .any(|(k, _)| k.contains("Figure") && k.contains('P')),
        "no ? overlay row for the Figure's P: {rows:?}"
    );
}

/// The mock's hover path, `(s, t)` ends and RGBA, and the texts over the figure.
type Drawn = (Vec<([[f32; 2]; 2], [f32; 4])>, Vec<String>);

/// The figure's path and label follow the pointer: they are those an unpinned hover at the same point draws.
fn check_follows(pinned: &Drawn, unpinned: &Drawn) {
    assert!(!unpinned.0.is_empty(), "the reference hover drew no path");
    assert_eq!(
        pinned.0, unpinned.0,
        "pinned, the figure's path is not the pointer's"
    );
    assert_eq!(
        pinned.1, unpinned.1,
        "pinned, the hover label is not the pointer's"
    );
}

#[test]
fn qa_trajectory_pin_the_figure_path_and_label_follow_the_pointer() {
    // The reference: an unpinned hover at B.
    let mut free = Rig::new();
    free.pointer_to(fig(B));
    let at_b = (free.marks(), figure_texts(&mut free));
    // Pinned at A, the pointer at A, then at B.
    let mut rig = Rig::new();
    rig.click(fig(A));
    assert!(rig.app.trajectory.pin.is_some(), "the click did not pin");
    rig.pointer_to(fig(A) + egui::vec2(0.0, 0.0));
    let at_a = (rig.marks(), figure_texts(&mut rig));
    rig.pointer_to(fig(B));
    let pinned_b = (rig.marks(), figure_texts(&mut rig));
    check_follows(&pinned_b, &at_b);
    // The control: a figure path (and label) that follow the pin, A, with the pointer at B.
    assert_ne!(at_a.0, at_b.0, "A and B draw the same path");
    rejects("a figure path that follows the pin", || {
        check_follows(&at_a, &at_b)
    });
}

/// `kept`'s last orbit is `want`.
fn check_kept(kept: &[Sample], want: &Sample) {
    let last = kept.last().expect("nothing was kept");
    assert_eq!(last.ic, want.ic, "\"+\" kept another orbit");
}

#[test]
fn qa_trajectory_pin_plus_with_the_mouse_keeps_the_shown_sample() {
    let mut rig = Rig::new();
    // Unpinned: hover A, then the pointer goes to "+" and clicks it.
    rig.pointer_to(fig(A));
    let a = rig.snap().sample.expect("no sample at A");
    let plus = rig.place(traj::KEEP);
    let u0 = rig.undo_depth();
    rig.click(plus.center());
    check_kept(&rig.app.view.kept_orbits.orbits, &a);
    // Pinned at B, hover C, then "+": B is kept.
    rig.click(fig(B));
    let b = rig.pin_sample().expect("the click did not pin");
    rig.pointer_to(fig(C));
    let plus = rig.place(traj::KEEP);
    rig.click(plus.center());
    let kept = rig.app.view.kept_orbits.orbits.clone();
    assert_eq!(
        kept.len(),
        2,
        "\"+\" did not keep a second orbit: {}",
        kept.len()
    );
    check_kept(&kept, &b);
    assert_eq!(rig.undo_depth(), u0, "keeping entered undo");
    rejects("a \"+\" that kept nothing", || check_kept(&[], &b));
    rejects("a \"+\" that kept the hovered orbit", || {
        check_kept(&kept[..1], &b)
    });
}

#[test]
fn qa_trajectory_pin_a_sim_key_change_reintegrates_the_ic() {
    let mut rig = Rig::new();
    rig.click(fig(A));
    let before = rig.pin_sample().expect("the click did not pin");
    let t = rig.snap().sim.horizon.t;
    let new_t = if (t - 120.0).abs() > 1.0 {
        120.0
    } else {
        150.0
    };
    rig.app.set_field(SetField {
        edit: Edit::Sim(SimField::Horizon(Horizon { t: new_t })),
        no_history: false,
    });
    rig.idle(3);
    assert_eq!(
        rig.snap().sim.horizon.t,
        new_t,
        "the horizon edit was not applied"
    );
    let after = rig
        .pin_sample()
        .expect("the sim key change cleared the pin");
    check_holds_ic(&before, &after);
    let want = rig
        .app
        .side()
        .compute_ic(&before.ic, before.at)
        .expect("the mock has computeIC");
    assert_eq!(
        after, want,
        "the pinned IC was not re-integrated under the new sim key"
    );
}

// --- REQ-GUI-202: the views -----------------------------------------------------------------------------------------

/// The app with A pinned and the pointer resting on the panel's title, the sphere's turn off.
fn shown() -> Rig {
    let mut rig = Rig::new();
    rig.click(fig(A));
    assert!(rig.app.trajectory.pin.is_some(), "the click did not pin");
    rig.pointer_to(areas(layout().trajectory).title.center());
    rig.app.trajectory.turn = false;
    rig.idle(1);
    rig
}

/// The inner rects the views draw in: real space, the sphere's place.
fn inner() -> (Rect, Rect) {
    let a = areas(layout().trajectory);
    (a.real.shrink(8.0), a.sphere.shrink(8.0))
}

/// The polyline points drawn clipped to `rect`.
fn path_points(rig: &mut Rig, rect: Rect) -> Vec<Pos2> {
    rig.shapes()
        .into_iter()
        .filter(|(clip, _)| same_rect(*clip, rect))
        .flat_map(|(_, s)| match s {
            egui::Shape::Path(p) => p.points,
            egui::Shape::LineSegment { points, .. } => points.to_vec(),
            _ => Vec::new(),
        })
        .collect()
}

/// The circles' radii drawn clipped to `rect`.
fn radii(rig: &mut Rig, rect: Rect) -> Vec<f32> {
    rig.shapes()
        .into_iter()
        .filter(|(clip, _)| same_rect(*clip, rect))
        .filter_map(|(_, s)| match s {
            egui::Shape::Circle(c) => Some(c.radius),
            _ => None,
        })
        .collect()
}

/// Each point of `after` is `before`'s moved by `by`, to `tol` points.
fn check_moved(before: &[Pos2], after: &[Pos2], by: egui::Vec2, tol: f32) {
    assert!(!before.is_empty(), "the view drew no path");
    assert_eq!(
        before.len(),
        after.len(),
        "the path's points changed in number"
    );
    for (p, q) in before.iter().zip(after) {
        assert!(
            ((*q - *p) - by).length() <= tol,
            "a point moved by {:?}, not {by:?}",
            *q - *p
        );
    }
}

/// Each point of `after` is `before`'s scaled by `k` about `about`, to `tol` points.
fn check_scaled(before: &[Pos2], after: &[Pos2], about: Pos2, k: f32, tol: f32) {
    assert!(!before.is_empty(), "the view drew no path");
    assert_eq!(
        before.len(),
        after.len(),
        "the path's points changed in number"
    );
    for (p, q) in before.iter().zip(after) {
        let want = about + (*p - about) * k;
        assert!(
            (*q - want).length() <= tol,
            "a point went to {q:?}, not {want:?}: not scaled by {k} about {about:?}"
        );
    }
}

/// The scale `after` is of `before` about `about`, by least squares.
fn fitted_scale(before: &[Pos2], after: &[Pos2], about: Pos2) -> f32 {
    let (mut num, mut den) = (0.0f32, 0.0f32);
    for (p, q) in before.iter().zip(after) {
        let (u, v) = (*p - about, *q - about);
        num += u.x * v.x + u.y * v.y;
        den += u.x * u.x + u.y * u.y;
    }
    num / den
}

#[test]
fn qa_trajectory_views_real_space_drags_and_zooms_about_the_pointer() {
    let mut rig = shown();
    let (real, _) = inner();
    let (n0, u0) = (rig.contract().len(), rig.undo_depth());
    let p0 = path_points(&mut rig, real);
    // A drag of (36, −24) points across real space, in six moves.
    let by = egui::vec2(36.0, -24.0);
    let from = real.center() + egui::vec2(-20.0, 10.0);
    rig.drag(from, from + by, 6);
    let p1 = path_points(&mut rig, real);
    check_moved(&p0, &p1, by, 0.05);
    // The control: a view that only paints.
    rejects("a view that did not follow the drag", || {
        check_moved(&p0, &p0, by, 0.05)
    });
    // The wheel about a point off the centre: everything scales about the pointer.
    let at = real.center() + egui::vec2(15.0, -12.0);
    rig.wheel(at, 1.0);
    let p2 = path_points(&mut rig, real);
    let k = fitted_scale(&p1, &p2, at);
    assert!(
        (k - 1.0).abs() > 0.01,
        "the wheel did not zoom real space: {k}"
    );
    check_scaled(&p1, &p2, at, k, 0.05);
    rejects("a zoom about the view's centre", || {
        let about_centre: Vec<Pos2> = p1
            .iter()
            .map(|p| real.center() + (*p - real.center()) * k)
            .collect();
        check_scaled(&p1, &about_centre, at, k, 0.05)
    });
    // A double-click restores the fitted view.
    rig.double_click(real.center());
    let p3 = path_points(&mut rig, real);
    check_moved(&p0, &p3, egui::Vec2::ZERO, 0.05);
    // No SetField, no undo.
    assert_eq!(rig.contract().len(), n0, "the view sent a SetField");
    assert_eq!(rig.undo_depth(), u0, "the view entered undo");
}

fn sphere_view(rig: &Rig) -> traj::Orbit {
    rig.app.trajectory.views_of(rig.app.trajectory.tab).sphere
}

#[test]
fn qa_trajectory_views_the_sphere_orbits_clamps_and_zooms() {
    let mut rig = shown();
    let (_, sphere) = inner();
    let (n0, u0) = (rig.contract().len(), rig.undo_depth());
    rig.app.trajectory.turn = true;
    rig.idle(1);
    let o0 = sphere_view(&rig);
    let p0 = path_points(&mut rig, sphere);
    // A sideways drag turns the yaw, and unticks "turn".
    let c = sphere.center();
    rig.drag(c, c + egui::vec2(40.0, 0.0), 4);
    let o1 = sphere_view(&rig);
    assert!(!rig.app.trajectory.turn, "a drag left \"turn\" ticked");
    assert!(o1.yaw != o0.yaw, "a sideways drag did not turn the yaw");
    assert_eq!(o1.pitch, o0.pitch, "a sideways drag changed the pitch");
    assert_ne!(
        path_points(&mut rig, sphere),
        p0,
        "the orbit drew nothing new"
    );
    // An up-down drag turns the pitch; a long one holds at ±90°.
    rig.drag(c, c + egui::vec2(0.0, 30.0), 3);
    let o2 = sphere_view(&rig);
    assert!(
        o2.pitch != o1.pitch,
        "an up-down drag did not turn the pitch"
    );
    for dy in [600.0f32, -1200.0] {
        rig.drag(c, c + egui::vec2(0.0, dy), 12);
        let p = sphere_view(&rig).pitch;
        assert_eq!(
            p.abs(),
            90.0,
            "a long drag left the pitch at {p}, not clamped at ±90°"
        );
    }
    // The wheel scales the sphere: its outline circle grows.
    let r0 = radii(&mut rig, sphere);
    rig.wheel(c, 1.0);
    let r1 = radii(&mut rig, sphere);
    let big = |r: &[f32]| r.iter().copied().fold(0.0f32, f32::max);
    assert!(
        big(&r1) > big(&r0) * 1.01,
        "the wheel did not scale the sphere: {r0:?} → {r1:?}"
    );
    // A double-click restores the default view and size.
    rig.double_click(c);
    let d = sphere_view(&rig);
    let def = traj::Orbit::default();
    assert_eq!(
        (d.yaw, d.pitch, d.zoom),
        (def.yaw, def.pitch, def.zoom),
        "a double-click did not restore the sphere's default"
    );
    assert!(
        (big(&radii(&mut rig, sphere)) - big(&r0)).abs() < 0.01,
        "a double-click did not restore the sphere's size"
    );
    assert_eq!(rig.contract().len(), n0, "the sphere sent a SetField");
    assert_eq!(rig.undo_depth(), u0, "the sphere entered undo");
}

#[test]
fn qa_trajectory_views_ticking_turn_resumes_from_the_view_as_it_stands() {
    // The live mock: the turn reads the wall clock, which the frozen one holds (REQ-GUI-157).
    let mut rig = Rig::running();
    rig.click(fig(A));
    assert!(rig.app.trajectory.pin.is_some(), "the click did not pin");
    rig.pointer_to(areas(layout().trajectory).title.center());
    let (_, sphere) = inner();
    let c = sphere.center();
    assert!(
        rig.app.trajectory.turn,
        "the sphere does not turn by default"
    );
    rig.drag(c, c + egui::vec2(40.0, 25.0), 4);
    assert!(!rig.app.trajectory.turn, "a drag left \"turn\" ticked");
    // Held off, the view holds.
    let (orbit, angle) = (sphere_view(&rig), rig.app.trajectory.angle);
    assert_ne!(orbit, traj::Orbit::default(), "the drag did not orbit");
    rig.idle(4);
    assert_eq!(
        rig.app.trajectory.angle, angle,
        "with turn off the sphere turned"
    );
    // Ticked again, it turns on from the view as it stands: the orbit kept, the angle going on from where it stood.
    let turn = rig.place(traj::TURN);
    rig.click(turn.center());
    assert!(rig.app.trajectory.turn, "the turn box did not tick");
    rig.idle(4);
    let after = sphere_view(&rig);
    assert_eq!(after, orbit, "ticking turn reset the orbit");
    let moved = (rig.app.trajectory.angle - angle).rem_euclid(360.0);
    // At most the frames since the tick, under twelve at FRAME_S, at the declared rate.
    assert!(
        moved > 0.0 && moved <= traj::TURN_DEG_PER_S * 12.0 * FRAME_S + 1e-9,
        "ticked again, the turn went {moved}° from where it stood"
    );
}

/// The data's (θ, φ) are as they were.
fn check_data(before: &[[f64; 2]], after: &[[f64; 2]]) {
    assert_eq!(before, after, "the orbit rotated the data");
}

#[test]
fn qa_trajectory_views_the_orbit_turns_the_view_never_the_data() {
    let mut rig = shown();
    let (_, sphere) = inner();
    let data0 = rig.first().expect("nothing shown").shape;
    // The unwrapped drawing before any orbit.
    let unwrapped = rig.place(traj::UNWRAPPED);
    rig.click(unwrapped.center());
    assert!(
        rig.app.trajectory.unwrapped,
        "the unwrapped view is not shown"
    );
    rig.pointer_to(areas(layout().trajectory).title.center());
    let u0 = path_points(&mut rig, sphere);
    // Back to the sphere, orbit it hard both ways.
    let sph = rig.place(traj::SPHERE);
    rig.click(sph.center());
    assert!(!rig.app.trajectory.unwrapped, "the sphere is not shown");
    let c = sphere.center();
    rig.drag(c, c + egui::vec2(70.0, -45.0), 7);
    assert_ne!(
        sphere_view(&rig),
        traj::Orbit::default(),
        "the drag did not orbit"
    );
    let data1 = rig.first().expect("nothing shown").shape;
    check_data(&data0, &data1);
    // The unwrap agrees: drawn as before.
    let unwrapped = rig.place(traj::UNWRAPPED);
    rig.click(unwrapped.center());
    rig.pointer_to(areas(layout().trajectory).title.center());
    let u1 = path_points(&mut rig, sphere);
    assert!(!u0.is_empty(), "the unwrapped view drew no path");
    assert_eq!(u0, u1, "an orbit of the sphere changed its unwrap");
    // The control: data rotated by the orbit.
    let rotated: Vec<[f64; 2]> = data0
        .iter()
        .map(|[t, p]| [(t + 0.3).rem_euclid(std::f64::consts::TAU), *p])
        .collect();
    rejects("an orbit that rotated the data", || {
        check_data(&data0, &rotated)
    });
}

#[test]
fn qa_trajectory_views_the_unwrapped_view_pans_and_zooms() {
    let mut rig = shown();
    let (_, place) = inner();
    let unwrapped = rig.place(traj::UNWRAPPED);
    rig.click(unwrapped.center());
    assert!(
        rig.app.trajectory.unwrapped,
        "the unwrapped view is not shown"
    );
    rig.pointer_to(areas(layout().trajectory).title.center());
    let (n0, u0) = (rig.contract().len(), rig.undo_depth());
    let p0 = path_points(&mut rig, place);
    let by = egui::vec2(-30.0, 20.0);
    let from = place.center() + egui::vec2(10.0, -10.0);
    rig.drag(from, from + by, 5);
    let p1 = path_points(&mut rig, place);
    check_moved(&p0, &p1, by, 0.05);
    let at = place.center() + egui::vec2(-18.0, 9.0);
    rig.wheel(at, -1.0);
    let p2 = path_points(&mut rig, place);
    let k = fitted_scale(&p1, &p2, at);
    assert!(
        (k - 1.0).abs() > 0.01,
        "the wheel did not zoom the unwrapped view"
    );
    check_scaled(&p1, &p2, at, k, 0.05);
    rig.double_click(place.center());
    check_moved(&p0, &path_points(&mut rig, place), egui::Vec2::ZERO, 0.05);
    assert_eq!(
        rig.contract().len(),
        n0,
        "the unwrapped view sent a SetField"
    );
    assert_eq!(rig.undo_depth(), u0, "the unwrapped view entered undo");
}

/// A step `got` is the named `want`, to `rel` of it: 1e-9 for a state's step, 1e-3 for one measured in drawn points,
/// which egui holds in f32.
fn check_step(got: f64, want: f64, rel: f64) {
    assert!(
        (got - want).abs() <= rel * want.abs().max(1e-3),
        "the step was {got}, not the named {want}"
    );
}

/// Reaches view `id` by the keyboard from the Trajectory scope (Enter, then the arrows) and starts its interaction.
fn interact_on(rig: &mut Rig, id: &str) {
    rig.focus(traj::TRAJECTORY, NAV);
    rig.key(Key::Enter, NONE);
    let mut reached = false;
    for _ in 0..40 {
        if rig.focused().as_deref() == Some(id) {
            reached = true;
            break;
        }
        rig.key(Key::ArrowDown, NONE);
    }
    assert!(
        reached,
        "the arrows never reach the view {id} in the Trajectory scope"
    );
    let path = rig.app.view.focus.path.clone();
    assert_eq!(
        path.first().map(String::as_str),
        Some(traj::TRAJECTORY),
        "the view {id} is not in the Trajectory scope: {path:?}"
    );
    assert_eq!(
        rig.app.view.focus.mode, NAV,
        "landing on a view is not navigation mode"
    );
    rig.key(Key::Enter, NONE);
    assert_eq!(
        rig.app.view.focus.mode, INTERACT,
        "Enter on {id} did not start interaction"
    );
}

#[test]
fn qa_trajectory_views_the_keys_pan_by_pan_and_zoom_by_zoomlog2() {
    let mut rig = shown();
    let (real, place) = inner();
    let (n0, u0) = (rig.contract().len(), rig.undo_depth());
    // Real space: the view's span is its square's side; R-404's Pan is 1/20 of it.
    for (id, rect, unwrapped) in [
        (traj::REAL_VIEW, real, false),
        (traj::SPHERE_VIEW, place, true),
    ] {
        if unwrapped {
            let u = rig.place(traj::UNWRAPPED);
            rig.click(u.center());
            assert!(
                rig.app.trajectory.unwrapped,
                "the unwrapped view is not shown"
            );
            rig.pointer_to(areas(layout().trajectory).title.center());
        }
        interact_on(&mut rig, id);
        let span = f64::from(rect.width().min(rect.height()));
        for (modifiers, times) in [(NONE, 1.0), (SHIFT, 10.0), (ALT, 0.1)] {
            for (key, horizontal) in [(Key::ArrowRight, true), (Key::ArrowUp, false)] {
                let p0 = path_points(&mut rig, rect);
                rig.key(key, modifiers);
                let p1 = path_points(&mut rig, rect);
                assert!(!p0.is_empty(), "{id} drew no path");
                let d = p1[0] - p0[0];
                let (along, across) = if horizontal { (d.x, d.y) } else { (d.y, d.x) };
                assert!(
                    across.abs() < 1e-3,
                    "{key:?} on {id} moved across its axis: {d:?}"
                );
                check_step(f64::from(along.abs()), span / 20.0 * times, 1e-3);
                check_moved(&p0, &p1, d, 0.05);
            }
        }
        // + and −: a quarter octave about the view's centre, Shift ×10, Alt ×0.1.
        for (key, modifiers, octaves) in [
            (Key::Plus, NONE, 0.25),
            (Key::Minus, NONE, -0.25),
            (Key::Plus, ALT, 0.025),
            (Key::Minus, SHIFT, -2.5),
            (Key::Plus, SHIFT, 2.5),
        ] {
            let p0 = path_points(&mut rig, rect);
            rig.key(key, modifiers);
            let p1 = path_points(&mut rig, rect);
            let k = fitted_scale(&p0, &p1, rect.center());
            check_step(f64::from(k).log2(), octaves, 1e-3);
            check_scaled(&p0, &p1, rect.center(), k, 0.05);
        }
        rig.key(Key::Escape, NONE);
        assert_eq!(
            rig.app.view.focus.mode, NAV,
            "Esc on {id} did not return to navigation"
        );
        assert_eq!(rig.focused().as_deref(), Some(id), "Esc left the view {id}");
    }
    assert_eq!(rig.contract().len(), n0, "a view's key sent a SetField");
    assert_eq!(rig.undo_depth(), u0, "a view's key entered undo");
    rejects("a pan step of 1/10 of the span", || {
        check_step(0.1, 0.05, 1e-3)
    });
    rejects("a zoom step of half an octave", || {
        check_step(0.5, 0.25, 1e-3)
    });
}

#[test]
fn qa_trajectory_views_the_keys_turn_the_sphere_by_angle_and_zoom_it() {
    let mut rig = shown();
    let (n0, u0) = (rig.contract().len(), rig.undo_depth());
    interact_on(&mut rig, traj::SPHERE_VIEW);
    assert!(!rig.app.trajectory.unwrapped, "the sphere is not shown");
    // Start mid-range so no step meets the clamp.
    for (modifiers, times) in [(NONE, 1.0), (SHIFT, 10.0), (ALT, 0.1)] {
        let o0 = sphere_view(&rig);
        rig.key(Key::ArrowRight, modifiers);
        let o1 = sphere_view(&rig);
        let dyaw = (o1.yaw - o0.yaw + 180.0).rem_euclid(360.0) - 180.0;
        check_step(dyaw.abs(), times, 1e-9);
        assert_eq!(o1.pitch, o0.pitch, "→ changed the pitch");
        rig.key(Key::ArrowDown, modifiers);
        let o2 = sphere_view(&rig);
        check_step((o2.pitch - o1.pitch).abs(), times, 1e-9);
        assert_eq!(o2.yaw, o1.yaw, "↓ changed the yaw");
        // Back up, so the pitch stays mid-range.
        rig.key(Key::ArrowUp, modifiers);
        check_step(sphere_view(&rig).pitch, o1.pitch, 1e-9);
    }
    for (key, modifiers, octaves) in [
        (Key::Plus, NONE, 0.25),
        (Key::Minus, NONE, -0.25),
        (Key::Plus, ALT, 0.025),
        (Key::Plus, SHIFT, 2.5),
        (Key::Minus, SHIFT, -2.5),
    ] {
        let z0 = sphere_view(&rig).zoom;
        rig.key(key, modifiers);
        check_step(sphere_view(&rig).zoom - z0, octaves, 1e-9);
    }
    rig.key(Key::Escape, NONE);
    assert_eq!(
        rig.app.view.focus.mode, NAV,
        "Esc did not return to navigation"
    );
    assert_eq!(
        rig.contract().len(),
        n0,
        "the sphere's keys sent a SetField"
    );
    assert_eq!(rig.undo_depth(), u0, "the sphere's keys entered undo");
    rejects("a turn of 2° for a step", || check_step(2.0, 1.0, 1e-9));
}
