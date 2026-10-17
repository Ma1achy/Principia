//! QA tests for TASK-M6-27, written from the requirements and their sources, not from the implementation:
//!
//! - REQ-GUI-172 (render_gui_spec §G2, §G3, §G6; GUI_DESIGN_NOTES "01 Explore", "06 Legend"; R-101, R-404, R-409):
//!   Time's play advances the playhead through `SetField`s marked "no history" and never enters the history; step moves
//!   one `TimeStep`, 1/100 of `[0, T]` (R-404); a scrub is one SetField, one undo entry (R-101); Space plays with the
//!   focus in Time in either mode; the arrows move between Time's elements in navigation mode and step no time, and
//!   step the time by `TimeStep` (Shift ×10, Alt ×0.1) in interaction mode on the scrubber (R-409 F1).
//!   The Legend draws exactly the keys the snapshot carries, in the order received, and none of its own: a stain with a
//!   dimension added or the brightness channel removed changes the keys drawn, and no keys draw an empty frame; it is
//!   called "Legend", never "Fate" (§G6). Its `running` swatch is the "not yet" style, the grey with a fine dot stipple
//!   from the pixel position, never flat (R-399; `debug_not_yet`'s CPU mirror, REQ-COL-064's proposal).
//!   The hover writes the cursor through a no-history SetField; the hover label names a fate only once it is decided,
//!   reading "undecided · still interacting at t … of …" until then (§G2); at the horizon every sample is decided
//!   (a terminal state or `bounded`, payload §2); the GUI draws nothing over the figure but the label; the mock draws
//!   the hover path, solid at the cursor's IC, which is on the slice, fainter where it leaves it; the Trajectory
//!   panel's "under cursor" tab shows the same sample; moving off clears both. Overlays ▾ holds §G2's fourteen
//!   toggles in its four groups, plus all off and save as default, with its count on the top bar.
//!   The keyboard's big scopes run Figure · Trajectory · Compass · Time · Legend in §G3's order (3 to 7), every
//!   landing in navigation mode; Enter on the Legend does nothing (read-only); the Trajectory panel's playhead, listen
//!   and kept orbits are reached with Enter. No test asserts Legend last or a count of big scopes (R-405).
//! - REQ-GUI-173 (§G2, §12.1 Tier 1): the tile-bounds toggle is one SetField on the overlay set; the GUI draws nothing
//!   over the figure for it; the mock draws the bounds of its fake quads into the stand-in, and they pan and zoom with
//!   it.
//! - REQ-GUI-157 (calibration, proposed, pending the human at the M8 gate, R-71): the sphere turns at a rate in degrees
//!   per second, frame-rate independent; clearing "turn" stops it. The tests check that the code honours the declared
//!   rate, whatever it is, not the value.
//!
//! Each SetField is counted from the contract's own log (every engine logs each SetField it applies as one `info`
//! entry from `contract`, gui_state_contract §2, "SetField <path> <before> → <after>", "(no history)" when so marked,
//! 12_console.png); what it edited is read from the snapshots. The chart is chart_decoder_contract Part 3's,
//! `z(s, t) = z₀ + (2s − 1) q₁ + (2t − 1) q₂`, `t` Y-up. gui takes no dependency on validation (R-187), so each check
//! runs its own control (R-176) through [`rejects`].
// The file name `qa_TASK-M6-27` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::sync::Arc;

use eframe::egui::{self, Event, Key, Modifiers, PointerButton, Pos2, Rect};
use engine::contract::legend::{not_yet_dot_at, NOT_YET_DOT_SRGB8, NOT_YET_SRGB8, SWATCHES};
use engine::contract::log::Source;
use engine::contract::render_state::{Overlays, Playhead};
use engine::contract::set_field::{Edit, RenderField, SetField};
use engine::contract::sim_config::Plane;
use engine::contract::snapshot::Snapshot;
use engine::contract::view_ui::{KeyMode, Mode};
use gui::app::App;
use gui::capture::{render_frame, Shot, FORMAT, PIXELS_PER_POINT, SIZE};
use gui::headless::Headless;
use gui::layout::Layout;
use gui::mock::canvas::MockCanvas;
use gui::mock::legend::{Class, Dimension, FakeStain};
use gui::mock::MockEngine;
use gui::side::{EngineSide, MockSide};

type Z = [f64; 8];

const NONE: Modifiers = Modifiers::NONE;
const SHIFT: Modifiers = Modifiers::SHIFT;
const ALT: Modifiers = Modifiers::ALT;
const NAV: KeyMode = KeyMode::Navigation;
const INTERACT: KeyMode = KeyMode::Interaction;

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

/// The mock, its clock frozen (as capture mode runs it), with no canvas.
fn frozen() -> Rig {
    Rig::new(MockSide::new(MockEngine::frozen(), None))
}

/// The mock, its clock running one tick a frame.
fn running() -> Rig {
    Rig::new(MockSide::new(MockEngine::new(), None))
}

impl Rig {
    fn new(side: MockSide) -> Self {
        let mut rig = Self {
            app: App::new(side, FORMAT),
            h: headless(),
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

    /// The `contract` log entries so far, oldest first: one per SetField applied.
    fn contract(&self) -> Vec<String> {
        self.app
            .console()
            .iter()
            .filter(|e| e.source == Source::Contract)
            .map(|e| e.message.clone())
            .collect()
    }

    /// The `contract` entries since the first `n`.
    fn since(&self, n: usize) -> Vec<String> {
        self.contract().split_off(n)
    }

    /// The focus's labels, outermost first, and its mode.
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

    /// Puts the focus on the scope `id`, in navigation mode, as a landing is (R-409 point 4).
    fn focus(&mut self, id: &str) {
        let path = self.app.keyboard.tree(Mode::Explore).path_to(id);
        assert!(!path.is_empty(), "no scope {id} in the tree");
        self.app.view.focus.path = path;
        self.app.view.focus.mode = NAV;
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
        self.idle(2);
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

    /// The shapes drawn this frame, flattened.
    fn shapes(&mut self) -> Vec<egui::Shape> {
        let out = self.frame(Vec::new());
        let mut flat = Vec::new();
        for clipped in &out.shapes {
            flatten(&clipped.shape, &mut flat);
        }
        flat
    }

    /// The texts drawn this frame, with their rects in points.
    fn texts(&mut self) -> Vec<(String, Rect)> {
        self.shapes()
            .into_iter()
            .filter_map(|s| match &s {
                egui::Shape::Text(t) => {
                    Some((t.galley.text().to_owned(), s.visual_bounding_rect()))
                }
                _ => None,
            })
            .collect()
    }

    /// The texts drawn inside `rect`.
    fn texts_in(&mut self, rect: Rect) -> Vec<(String, Rect)> {
        self.texts()
            .into_iter()
            .filter(|(_, r)| rect.expand(1.0).contains_rect(*r))
            .collect()
    }

    /// The mock's marks: the lines it draws into its stand-in, `(s, t)` ends and RGBA.
    fn marks(&mut self) -> Vec<([[f32; 2]; 2], [f32; 4])> {
        self.app
            .side()
            .engine()
            .marks()
            .into_iter()
            .map(|l| (l.ends, l.rgba))
            .collect()
    }

    fn set_stain(&mut self, stain: FakeStain) {
        self.app.side().engine().set_stain(stain);
        self.idle(2);
    }
}

fn flatten(shape: &egui::Shape, out: &mut Vec<egui::Shape>) {
    match shape {
        egui::Shape::Vec(v) => v.iter().for_each(|s| flatten(s, out)),
        other => out.push(other.clone()),
    }
}

/// The point of `figure` at the chart's `(s, t)`, `t` Y-up (chart_decoder_contract Part 3).
fn figure_pos(figure: Rect, s: f32, t: f32) -> Pos2 {
    egui::pos2(
        figure.min.x + figure.width() * s,
        figure.max.y - figure.height() * t,
    )
}

/// Whether `message` is a SetField log line on `path`, marked "no history" or not.
fn is_set_field(message: &str, path: &str, no_history: bool) -> bool {
    message.starts_with(&format!("SetField {path} "))
        && message.ends_with(" (no history)") == no_history
}

// --- REQ-GUI-172: Time ----------------------------------------------------------------------------------------------

/// Play's run: the playhead moved forward from `t0` to `t1`, through at least one SetField, every one on the playhead
/// and marked "no history", and the history is as it was (R-101).
fn check_play(entries: &[String], t0: f64, t1: f64, depths0: (u32, u32), depths1: (u32, u32)) {
    assert!(t1 > t0, "play did not advance the playhead: {t0} → {t1}");
    assert!(
        !entries.is_empty(),
        "play advanced the playhead through no SetField"
    );
    for e in entries {
        assert!(
            is_set_field(e, "Playhead.t", true),
            "play applied `{e}`, not a no-history SetField on the playhead"
        );
    }
    assert_eq!(depths1, depths0, "play entered the history (R-101)");
}

#[test]
fn qa_time_play_advances_through_no_history_setfields_and_pause_stops_it() {
    let mut rig = running();
    let s0 = rig.snap();
    let depths = |s: &Snapshot| (s.history.undo_depth, s.history.redo_depth);
    // Space plays with the focus on Time (§G3).
    rig.focus("time");
    let n0 = rig.contract().len();
    rig.key(Key::Space, NONE);
    assert!(rig.app.view.transport.playing, "Space on Time did not play");
    rig.idle(30);
    let s1 = rig.snap();
    let played = rig.since(n0);
    check_play(
        &played,
        s0.render.playhead.t,
        s1.render.playhead.t,
        depths(&s0),
        depths(&s1),
    );
    // Space again pauses: the playhead holds and nothing more is applied.
    rig.key(Key::Space, NONE);
    assert!(
        !rig.app.view.transport.playing,
        "Space on Time did not pause"
    );
    rig.idle(2);
    let (t_paused, n_paused) = (rig.snap().render.playhead.t, rig.contract().len());
    rig.idle(20);
    assert_eq!(
        rig.snap().render.playhead.t,
        t_paused,
        "paused, the playhead moved"
    );
    assert_eq!(
        rig.contract().len(),
        n_paused,
        "paused, a SetField was applied"
    );
    // Controls: a played edit that enters the history; a run that moved nothing.
    rejects("an undoable playhead edit", || {
        let mut bad = played.clone();
        bad.push("SetField Playhead.t 0.1 → 0.2".to_owned());
        check_play(&bad, 0.0, 1.0, (0, 0), (0, 0))
    });
    rejects("an undo depth grown by play", || {
        check_play(&played, 0.0, 1.0, (0, 0), (1, 0))
    });
    rejects("a playhead that stood still", || {
        check_play(&played, 1.0, 1.0, (0, 0), (0, 0))
    });
}

/// One button press: exactly one SetField on the playhead, with history, one undo entry, the playhead now `want`.
fn check_one_step(entries: &[String], undo0: u32, undo1: u32, t: f64, want: f64) {
    assert_eq!(
        entries.len(),
        1,
        "the step applied {entries:?}, not one SetField"
    );
    assert!(
        is_set_field(&entries[0], "Playhead.t", false),
        "the step applied `{}`, not one undoable SetField on the playhead",
        entries[0]
    );
    assert_eq!(undo1, undo0 + 1, "the step is not one undo entry");
    assert!(
        (t - want).abs() <= 1e-12 * want.abs().max(1.0),
        "the step moved the playhead to {t}, not {want}"
    );
}

#[test]
fn qa_time_step_moves_one_time_step() {
    let mut rig = frozen();
    let s0 = rig.snap();
    let t_max = s0.sim.horizon.t;
    assert!(t_max > 0.0, "the mock's horizon T is {t_max}");
    // R-404: `TimeStep` is 1/100 of the scrubber's range [0, t_max].
    let step = t_max / 100.0;
    let at = rig.place("time_step").center();
    let n0 = rig.contract().len();
    rig.click(at);
    let s1 = rig.snap();
    check_one_step(
        &rig.since(n0),
        s0.history.undo_depth,
        s1.history.undo_depth,
        s1.render.playhead.t,
        s0.render.playhead.t + step,
    );
    // Step back returns it.
    let back = rig.place("time_back").center();
    let n1 = rig.contract().len();
    rig.click(back);
    let s2 = rig.snap();
    check_one_step(
        &rig.since(n1),
        s1.history.undo_depth,
        s2.history.undo_depth,
        s2.render.playhead.t,
        s0.render.playhead.t,
    );
    rejects("a step of 1/10 of the range", || {
        check_one_step(
            &["SetField Playhead.t 0 → 5".to_owned()],
            0,
            1,
            t_max / 10.0,
            step,
        )
    });
    rejects("a step that is not an undo entry", || {
        check_one_step(
            &["SetField Playhead.t 0 → 0.5 (no history)".to_owned()],
            0,
            0,
            step,
            step,
        )
    });
}

#[test]
fn qa_time_a_scrub_is_one_setfield() {
    let mut rig = frozen();
    let s0 = rig.snap();
    let t_max = s0.sim.horizon.t;
    let rail = rig.place("time_scrubber");
    let from = egui::pos2(rail.min.x + 2.0, rail.center().y);
    let to = egui::pos2(rail.min.x + rail.width() * 0.6, rail.center().y);
    let n0 = rig.contract().len();
    rig.drag(from, to, 12);
    let s1 = rig.snap();
    let entries = rig.since(n0);
    check_one_scrub(&entries, s0.history.undo_depth, s1.history.undo_depth);
    // Where the scrub let go: 60 % of the rail, within the slider's resolution, a few points of the rail.
    let resolution = t_max * 4.0 / f64::from(rail.width());
    assert!(
        (s1.render.playhead.t - 0.6 * t_max).abs() <= resolution,
        "the scrub left the playhead at {}, not near {}",
        s1.render.playhead.t,
        0.6 * t_max
    );
    // Undo takes the whole scrub back in one step.
    rig.app.undo();
    rig.idle(2);
    assert_eq!(
        rig.snap().render.playhead.t,
        s0.render.playhead.t,
        "one undo did not take the scrub back"
    );
    rejects("a scrub of two entries", || {
        check_one_scrub(&[entries[0].clone(), entries[0].clone()], 0, 2)
    });
    rejects("a scrub through no-history edits", || {
        check_one_scrub(
            &["SetField Playhead.t 0 → 30 (no history)".to_owned()],
            0,
            0,
        )
    });
}

/// A scrub: one SetField on the playhead, with history, one undo entry (R-96, R-101).
fn check_one_scrub(entries: &[String], undo0: u32, undo1: u32) {
    assert_eq!(
        entries.len(),
        1,
        "a scrub applied {} SetFields, not one: {entries:?}",
        entries.len()
    );
    assert!(
        is_set_field(&entries[0], "Playhead.t", false),
        "the scrub applied `{}`, not one undoable SetField on the playhead",
        entries[0]
    );
    assert_eq!(undo1, undo0 + 1, "a scrub is not one undo entry (R-101)");
}

/// The arrows' step on the scrubber in interaction mode: each press moves the playhead by `want`.
fn check_moved(t0: f64, t1: f64, want: f64, what: &str) {
    assert!(
        (t1 - t0 - want).abs() <= 1e-9 * want.abs().max(1.0),
        "{what}: the playhead moved {}, not {want}",
        t1 - t0
    );
}

#[test]
fn qa_time_keys_in_both_modes() {
    let mut rig = frozen();
    let t_max = rig.snap().sim.horizon.t;
    let step = t_max / 100.0;
    // Enter on Time goes in, onto its first element, in navigation mode; the arrows there move between its elements and
    // step no time (R-409 F1).
    rig.focus("time");
    rig.key(Key::Enter, NONE);
    let (labels, mode) = rig.at();
    assert_eq!(mode, NAV, "Enter into Time did not land in navigation mode");
    assert_eq!(
        labels.len(),
        2,
        "Enter on Time did not go onto an element: {labels:?}"
    );
    assert_eq!(labels[0], "Time", "Enter on Time left it: {labels:?}");
    let (t0, n0) = (rig.snap().render.playhead.t, rig.contract().len());
    let mut seen = vec![labels[1].clone()];
    for k in [
        Key::ArrowRight,
        Key::ArrowRight,
        Key::ArrowRight,
        Key::ArrowRight,
        Key::ArrowLeft,
    ] {
        rig.key(k, NONE);
        let (l, m) = rig.at();
        assert_eq!(m, NAV, "an arrow left navigation mode");
        assert_eq!(l[0], "Time", "an arrow in navigation mode left Time: {l:?}");
        seen.push(l[1].clone());
    }
    assert!(
        seen.windows(2).any(|w| w[0] != w[1]),
        "the arrows did not move between Time's elements: {seen:?}"
    );
    assert_eq!(
        rig.snap().render.playhead.t,
        t0,
        "an arrow in navigation mode stepped the time"
    );
    assert_eq!(
        rig.contract().len(),
        n0,
        "an arrow in navigation mode applied a SetField"
    );
    // Enter on the scrubber: interaction mode; → and ← step the time by `TimeStep`, Shift ×10, Alt ×0.1 (R-404, R-409).
    rig.focus("time_scrubber");
    rig.key(Key::Enter, NONE);
    assert_eq!(
        rig.at().1,
        INTERACT,
        "Enter on the scrubber did not start interaction mode"
    );
    for (key, modifiers, want, what) in [
        (Key::ArrowRight, NONE, step, "→"),
        (Key::ArrowRight, SHIFT, 10.0 * step, "Shift+→"),
        (Key::ArrowRight, ALT, 0.1 * step, "Alt+→"),
        (Key::ArrowLeft, NONE, -step, "←"),
    ] {
        let t0 = rig.snap().render.playhead.t;
        rig.key(key, modifiers);
        check_moved(t0, rig.snap().render.playhead.t, want, what);
    }
    // Space plays with the focus on an element of Time, in interaction mode too.
    assert!(!rig.app.view.transport.playing);
    rig.key(Key::Space, NONE);
    assert!(
        rig.app.view.transport.playing,
        "Space in interaction mode on the scrubber did not play"
    );
    assert_eq!(rig.at().1, INTERACT, "Space ended interaction mode");
    rig.key(Key::Space, NONE);
    // And in navigation mode on an element.
    rig.focus("time_speed");
    rig.key(Key::Space, NONE);
    assert!(
        rig.app.view.transport.playing,
        "Space in navigation mode on speed did not play"
    );
    rejects("a step of 1/10", || {
        check_moved(0.0, t_max / 10.0, step, "→")
    });
}

// --- REQ-GUI-173: tile bounds ---------------------------------------------------------------------------------------

/// The overlay set after a toggle: `before` with tile bounds flipped and nothing else.
fn check_toggled(before: &Overlays, after: &Overlays) {
    let want = Overlays {
        tile_bounds: !before.tile_bounds,
        ..before.clone()
    };
    assert_eq!(after, &want, "the toggle did not flip tile bounds alone");
}

/// One SetField, on the overlay set, undoable.
fn check_one_on_overlays(entries: &[String], undo0: u32, undo1: u32) {
    assert_eq!(
        entries.len(),
        1,
        "the toggle applied {entries:?}, not one SetField"
    );
    assert!(
        is_set_field(&entries[0], "Overlays", false),
        "the toggle applied `{}`, not one undoable SetField on the overlay set",
        entries[0]
    );
    assert_eq!(undo1, undo0 + 1, "the toggle is not one undo entry (R-69)");
}

/// The text of the top bar's Overlays ▾, with its count.
fn overlays_button(rig: &mut Rig) -> String {
    let top = layout().top_bar;
    rig.texts_in(top)
        .into_iter()
        .map(|(t, _)| t)
        .find(|t| t.starts_with("Overlays ▾"))
        .expect("no Overlays ▾ on the top bar")
}

#[test]
fn qa_tile_bounds_toggle_is_one_set_field_on_the_overlay_set() {
    let mut rig = frozen();
    rig.pointer_gone();
    let s0 = rig.snap();
    assert_eq!(
        s0.render.overlays,
        Overlays::default(),
        "an overlay is on at start"
    );
    assert!(
        overlays_button(&mut rig).ends_with('0'),
        "the count is not 0"
    );
    // Alt+1: the first toggle, tile bounds (§G2's order; 01_main.png's Alt+1).
    let n0 = rig.contract().len();
    rig.key(Key::Num1, ALT);
    let s1 = rig.snap();
    check_one_on_overlays(&rig.since(n0), s0.history.undo_depth, s1.history.undo_depth);
    check_toggled(&s0.render.overlays, &s1.render.overlays);
    assert_eq!(s1.sim, s0.sim, "the toggle edited the sim config");
    assert_eq!(
        (s1.render.playhead.t, &s1.render.hover),
        (s0.render.playhead.t, &s0.render.hover),
        "the toggle edited the render state beyond the overlay set"
    );
    assert!(
        overlays_button(&mut rig).ends_with('1'),
        "the count does not read 1"
    );
    // Undo turns it off again.
    rig.app.undo();
    rig.idle(2);
    assert_eq!(
        rig.snap().render.overlays,
        s0.render.overlays,
        "undo did not undo the toggle"
    );
    rejects("a toggle that flips two overlays", || {
        check_toggled(
            &Overlays::default(),
            &Overlays {
                tile_bounds: true,
                grid: true,
                ..Overlays::default()
            },
        )
    });
    rejects("a toggle of two SetFields", || {
        check_one_on_overlays(
            &[
                "SetField Overlays [] → [tile_bounds]".to_owned(),
                "SetField Overlays [] → [tile_bounds]".to_owned(),
            ],
            0,
            2,
        )
    });
    rejects("a toggle with no history", || {
        check_one_on_overlays(
            &["SetField Overlays [] → [tile_bounds] (no history)".to_owned()],
            0,
            0,
        )
    });
}

/// The shapes drawn over the figure: each kind and rect.
fn over_figure(shapes: &[egui::Shape], figure: Rect) -> Vec<String> {
    let mut v: Vec<String> = shapes
        .iter()
        .filter(|s| s.visual_bounding_rect().intersects(figure.shrink(1.0)))
        .map(|s| {
            let kind = match s {
                egui::Shape::Text(t) => format!("text `{}`", t.galley.text()),
                egui::Shape::Callback(_) => "callback".to_owned(),
                egui::Shape::Mesh(_) => "mesh".to_owned(),
                egui::Shape::Rect(_) => "rect".to_owned(),
                egui::Shape::LineSegment { .. } => "line".to_owned(),
                egui::Shape::Path(_) => "path".to_owned(),
                other => format!("{other:?}").chars().take(16).collect(),
            };
            format!("{kind} {:?}", s.visual_bounding_rect())
        })
        .collect();
    v.sort();
    v
}

fn check_same_over_figure(off: &[String], on: &[String]) {
    assert_eq!(on, off, "the GUI drew over the figure for the overlay");
}

#[test]
fn qa_tile_bounds_drawn_by_the_mock_not_the_gui() {
    let figure = layout().figure;
    let mut rig = frozen();
    rig.pointer_gone();
    let off_shapes = rig.shapes();
    let off = over_figure(&off_shapes, figure);
    assert!(
        rig.marks().is_empty(),
        "the mock draws marks with no overlay and no hover"
    );
    rig.key(Key::Num1, ALT);
    assert!(rig.snap().render.overlays.tile_bounds);
    let on_shapes = rig.shapes();
    let on = over_figure(&on_shapes, figure);
    check_same_over_figure(&off, &on);
    let marks = rig.marks();
    assert!(
        !marks.is_empty(),
        "with tile bounds on, the mock draws no bounds into its stand-in"
    );
    // Every bound it draws is a line on the figure's chart, some of them inside the figure.
    assert!(
        marks
            .iter()
            .any(|(e, _)| e.iter().all(|p| p.iter().all(|v| (0.0..=1.0).contains(v)))),
        "no bound lies on the figure"
    );
    let mut extra = on.clone();
    extra.push("line [[1 1] - [2 2]]".to_owned());
    rejects("a line drawn by the GUI over the figure", || {
        check_same_over_figure(&off, &extra)
    });
}

// --- the chart (chart_decoder_contract Part 3), written here ------------------------------------------------------

fn add(a: &Z, k: f64, b: &Z) -> Z {
    std::array::from_fn(|i| a[i] + k * b[i])
}

fn dot(a: &Z, b: &Z) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// `z(s, t) = z₀ + (2s − 1) q₁ + (2t − 1) q₂`.
fn point(p: &Plane, s: f64, t: f64) -> Z {
    add(&add(&p.z0, 2.0 * s - 1.0, &p.q1), 2.0 * t - 1.0, &p.q2)
}

/// The `(s, t)` on chart `to` of the point at `(s, t)` on chart `from`, both charts of one plane.
fn rechart(from: &Plane, to: &Plane, [s, t]: [f64; 2]) -> [f64; 2] {
    let r = add(&point(from, s, t), -1.0, &to.z0);
    [
        (dot(&r, &to.q1) / dot(&to.q1, &to.q1) + 1.0) / 2.0,
        (dot(&r, &to.q2) / dot(&to.q2, &to.q2) + 1.0) / 2.0,
    ]
}

type Seg = [[f64; 2]; 2];

fn segs(marks: &[([[f32; 2]; 2], [f32; 4])]) -> Vec<Seg> {
    marks
        .iter()
        .map(|(e, _)| e.map(|p| p.map(f64::from)))
        .collect()
}

fn inside(p: [f64; 2], margin: f64) -> bool {
    p.iter().all(|v| (margin..=1.0 - margin).contains(v))
}

/// Every bound drawn `after` the gesture, well inside both figures, is a bound drawn `before` it moved to where the
/// gesture moved the chart (a pan), within `tol` of the figure. Returns how many were checked.
fn check_panned(before: &[Seg], b: &Plane, after: &[Seg], a: &Plane, tol: f64) -> usize {
    let mut checked = 0;
    for seg in after {
        let back = seg.map(|p| rechart(a, b, p));
        if !(seg.iter().all(|p| inside(*p, 0.05)) && back.iter().all(|p| inside(*p, 0.05))) {
            continue;
        }
        checked += 1;
        let close = |p: [f64; 2], q: [f64; 2]| (p[0] - q[0]).hypot(p[1] - q[1]) <= tol;
        let found = before.iter().any(|o| {
            (close(o[0], back[0]) && close(o[1], back[1]))
                || (close(o[0], back[1]) && close(o[1], back[0]))
        });
        assert!(
            found,
            "the bound {seg:?} has no bound before the gesture at {back:?}"
        );
    }
    assert!(checked > 0, "no bound inside both figures to check");
    checked
}

/// After a zoom in, every corner of a bound drawn `before`, inside the figure `after`, is still a corner drawn there:
/// a finer quadtree keeps its parents' corners.
fn check_zoomed(before: &[Seg], b: &Plane, after: &[Seg], a: &Plane, tol: f64) -> usize {
    let corners: Vec<[f64; 2]> = after.iter().flat_map(|s| *s).collect();
    let mut checked = 0;
    for p in before.iter().flat_map(|s| *s) {
        let q = rechart(b, a, p);
        if !inside(q, 0.05) {
            continue;
        }
        checked += 1;
        assert!(
            corners
                .iter()
                .any(|c| (c[0] - q[0]).hypot(c[1] - q[1]) <= tol),
            "the corner {p:?} is not drawn after the zoom, at {q:?}"
        );
    }
    assert!(checked > 0, "no corner inside the zoomed figure to check");
    checked
}

#[test]
fn qa_tile_bounds_pan_and_zoom_with_the_stand_in() {
    let figure = layout().figure;
    let mut rig = frozen();
    rig.key(Key::Num1, ALT);
    rig.pointer_gone();
    let (b0, p0) = (segs(&rig.marks()), rig.snap().sim.plane);
    // A pan: a drag on the figure.
    let from = figure.center();
    rig.drag(from, from + egui::vec2(-60.0, 35.0), 6);
    rig.pointer_gone();
    let (b1, p1) = (segs(&rig.marks()), rig.snap().sim.plane);
    assert_ne!(p0, p1, "the drag panned nothing");
    // f32 ends: a tolerance of a few f32 ulps of the figure's unit square.
    let tol = 1e-5;
    check_panned(&b0, &p0, &b1, &p1, tol);
    // A zoom in about the centre.
    rig.wheel(figure.center(), 2.0);
    rig.pointer_gone();
    let (b2, p2) = (segs(&rig.marks()), rig.snap().sim.plane);
    assert_ne!(p1, p2, "the wheel zoomed nothing");
    check_zoomed(&b1, &p1, &b2, &p2, tol);
    // Controls: the bounds read as if the chart had not moved; the zoom read as unzoomed.
    rejects("bounds that did not move with the pan", || {
        check_panned(&b0, &p0, &b1, &p0, tol);
    });
    rejects("bounds that did not scale with the zoom", || {
        check_zoomed(&b1, &p1, &b2, &p1, tol);
    });
}

/// GPU: with tile bounds on, pixels change inside the figure only.
#[test]
fn qa_tile_bounds_pixels_change_in_the_figure_only() {
    let lay = layout();
    let canvas = Arc::new(MockCanvas::new().expect("a GPU adapter for the mock's canvas"));
    let mut rig = Rig::new(MockSide::new(MockEngine::frozen(), Some(canvas.clone())));
    rig.pointer_gone();
    let off = render_frame(&canvas, &mut rig.app, &mut rig.h);
    rig.key(Key::Num1, ALT);
    rig.pointer_gone();
    let on = render_frame(&canvas, &mut rig.app, &mut rig.h);
    let skip = [lay.top_bar, lay.footer];
    check_figure_only(&off, &on, lay.figure, &skip);
    // Control: the same frames, read as if the figure were elsewhere, show the change outside it.
    let elsewhere = lay.figure.translate(egui::vec2(0.0, lay.figure.height()));
    rejects("bounds drawn outside the figure", || {
        check_figure_only(&off, &on, elsewhere, &skip)
    });
}

/// The overlay changed pixels of the figure and none outside it.
fn check_figure_only(off: &Shot, on: &Shot, figure: Rect, skip: &[Rect]) {
    let (inside, outside) = changed_pixels(off, on, figure, skip);
    assert!(inside > 0, "tile bounds changed no pixel of the figure");
    assert_eq!(
        outside, 0,
        "tile bounds changed {outside} pixels outside the figure"
    );
}

/// Pixels that differ between two shots: inside `figure`, and outside it but for the `skip` rects (the top bar, whose
/// count and undo depth change, and the footer, whose latest message is the toggle's SetField).
fn changed_pixels(a: &Shot, b: &Shot, figure: Rect, skip: &[Rect]) -> (usize, usize) {
    let p = PIXELS_PER_POINT;
    let (mut inside, mut outside) = (0, 0);
    for y in 0..a.size[1] as usize {
        for x in 0..a.size[0] as usize {
            let i = (y * a.size[0] as usize + x) * 4;
            if a.rgba[i..i + 4] == b.rgba[i..i + 4] {
                continue;
            }
            let pt = egui::pos2((x as f32 + 0.5) / p, (y as f32 + 0.5) / p);
            if figure.contains(pt) {
                inside += 1;
            } else if !skip.iter().any(|r| r.expand(2.0).contains(pt)) {
                outside += 1;
            }
        }
    }
    (inside, outside)
}

// --- REQ-GUI-172: Overlays ▾ --------------------------------------------------------------------------------------

/// §G2's toggles, by group, as words each entry must carry.
const G2_GROUPS: [&[&str]; 4] = [
    &[
        "tile bounds",
        "depth colouring",
        "priority heatmap",
        "refine_flagged",
        "split / merge",
    ],
    &[
        "substep density",
        "substep-cap hits",
        "NaN / invalid",
        "energy drift",
    ],
    &["forbidden region", "grid", "Burrau lattice"],
    &["class edges", "t_end contours"],
];

/// `a` and `b` name the same toggle, hyphen or space alike.
fn same_words(a: &str, b: &str) -> bool {
    let norm = |s: &str| s.replace('-', " ").to_lowercase();
    norm(a).contains(&norm(b))
}

/// The menu's texts, top to bottom, hold §G2's fourteen toggles in order and in their four groups (a heading before
/// each group), then all off and save as default.
fn check_menu(texts: &[String]) {
    let mut pos = 0;
    let mut last = 0;
    for (g, group) in G2_GROUPS.iter().enumerate() {
        let first = texts[pos..]
            .iter()
            .position(|t| same_words(t, group[0]))
            .unwrap_or_else(|| panic!("the menu has no `{}`: {texts:?}", group[0]))
            + pos;
        if g > 0 {
            assert!(
                texts[last + 1..first]
                    .iter()
                    .any(|t| !t.starts_with("Alt+")),
                "no group heading between `{}` and `{}`: {texts:?}",
                texts[last],
                texts[first]
            );
        }
        let mut at = first;
        for name in group.iter() {
            let i = texts[at..]
                .iter()
                .position(|t| same_words(t, name))
                .unwrap_or_else(|| {
                    panic!("the menu has no `{name}` after `{}`: {texts:?}", texts[at])
                })
                + at;
            at = i;
        }
        last = at;
        pos = at + 1;
    }
    for end in ["all off", "save as default"] {
        assert!(
            texts[pos..].iter().any(|t| t.eq_ignore_ascii_case(end)),
            "the menu has no `{end}` after the toggles: {texts:?}"
        );
    }
}

#[test]
fn qa_overlays_menu_holds_g2_toggles_grouped_with_all_off_and_save() {
    let mut rig = frozen();
    let top = layout().top_bar;
    let button = rig
        .texts_in(top)
        .into_iter()
        .find(|(t, _)| t.starts_with("Overlays ▾"))
        .expect("no Overlays ▾")
        .1;
    let before: Vec<String> = rig.texts().into_iter().map(|(t, _)| t).collect();
    rig.click(button.center());
    // The popup's texts: those the click added, and the toggles whose words the frame already shows elsewhere ("grid";
    // "forbidden region", the outcome legend's key since R-446, amended by qa for TASK-M2-32).
    let mut menu: Vec<(String, Rect)> = rig
        .texts()
        .into_iter()
        .filter(|(t, _)| !before.contains(t) || t == "grid" || t == "forbidden region")
        .collect();
    // The popup's own texts, top to bottom, then left to right.
    menu.sort_by(|a, b| {
        (a.1.min.y.round() as i64, a.1.min.x as i64)
            .cmp(&(b.1.min.y.round() as i64, b.1.min.x as i64))
    });
    let texts: Vec<String> = menu.into_iter().map(|(t, _)| t).collect();
    check_menu(&texts);
    // A click on tile bounds is one SetField; all off is one SetField turning every overlay off.
    let row = rig
        .texts()
        .into_iter()
        .find(|(t, _)| t == "tile bounds")
        .expect("no tile bounds entry")
        .1;
    let (s0, n0) = (rig.snap(), rig.contract().len());
    rig.click(row.center());
    let s1 = rig.snap();
    check_one_on_overlays(&rig.since(n0), s0.history.undo_depth, s1.history.undo_depth);
    check_toggled(&s0.render.overlays, &s1.render.overlays);
    let all_off = rig
        .texts()
        .into_iter()
        .find(|(t, _)| t == "all off")
        .expect("the menu closed after a toggle")
        .1;
    let n1 = rig.contract().len();
    rig.click(all_off.center());
    let s2 = rig.snap();
    check_one_on_overlays(&rig.since(n1), s1.history.undo_depth, s2.history.undo_depth);
    assert_eq!(
        s2.render.overlays,
        Overlays::default(),
        "all off left an overlay on"
    );
    let mut shuffled = texts.clone();
    if let Some(i) = shuffled.iter().position(|t| same_words(t, "grid")) {
        shuffled.remove(i);
    }
    rejects("a menu missing the grid", || check_menu(&shuffled));
}

// --- REQ-GUI-172: the Legend ----------------------------------------------------------------------------------------

/// A stain of one categorical dimension and, if `brightness`, a brightness dimension after it.
fn stain(
    title: &'static str,
    classes: Vec<(Class, f64)>,
    brightness: Option<&'static str>,
) -> FakeStain {
    let mut out = vec![Dimension::Categorical { title, classes }];
    if let Some(title) = brightness {
        out.push(Dimension::Brightness {
            title,
            lo: 0.0,
            hi: 10.0,
        });
    }
    FakeStain { out }
}

/// The legend's texts, top to bottom then left to right.
fn legend_texts(rig: &mut Rig) -> Vec<String> {
    let mut t = rig.texts_in(layout().legend);
    t.sort_by(|a, b| {
        (a.1.min.y.round() as i64, a.1.min.x as i64)
            .cmp(&(b.1.min.y.round() as i64, b.1.min.x as i64))
    });
    t.into_iter().map(|(t, _)| t).collect()
}

/// The legend draws `titles` as keys, in this order, and no other key title of `all`.
fn check_keys(texts: &[String], titles: &[&str], all: &[&str]) {
    let mut at = 0;
    for title in titles {
        let i = texts[at..]
            .iter()
            .position(|t| t.contains(title))
            .unwrap_or_else(|| panic!("the legend does not draw `{title}` in order: {texts:?}"))
            + at;
        at = i + 1;
    }
    for other in all.iter().filter(|t| !titles.contains(t)) {
        assert!(
            !texts.iter().any(|t| t.contains(other)),
            "the legend draws `{other}`, which the snapshot does not carry: {texts:?}"
        );
    }
}

fn swatch_label(param: &str) -> &'static str {
    SWATCHES
        .iter()
        .find(|s| s.param == param)
        .unwrap_or_else(|| panic!("no swatch {param}"))
        .label
}

#[test]
fn qa_legend_draws_exactly_the_keys_received() {
    const A: &str = "qa colour · alpha";
    const B: &str = "qa brightness · beta";
    const C: &str = "qa colour · gamma";
    let all = [A, B, C];
    let mut rig = frozen();
    // One categorical key with two classes, weights 3 : 1, then a brightness bar.
    rig.set_stain(stain(
        A,
        vec![(Class::Swatch("bounded"), 3.0), (Class::Running, 1.0)],
        Some(B),
    ));
    let texts = legend_texts(&mut rig);
    check_keys(&texts, &[A, B], &all);
    assert!(
        texts.iter().any(|t| t == "Legend"),
        "the frame is not called Legend: {texts:?}"
    );
    assert!(
        !texts.iter().any(|t| t.contains("Fate")),
        "the legend says Fate: {texts:?}"
    );
    // Its classes, and their shares of the view: 75 and 25 per cent.
    let bounded = swatch_label("bounded");
    for want in [bounded, "running", "75.0", "25.0"] {
        assert!(
            texts.iter().any(|t| t.contains(want)),
            "the legend does not draw `{want}`: {texts:?}"
        );
    }
    // No class of its own: no outcome label the stain does not carry.
    for s in SWATCHES.iter().filter(|s| s.param != "bounded") {
        assert!(
            !texts.iter().any(|t| t == s.label),
            "the legend draws `{}`, a class the stain does not carry: {texts:?}",
            s.label
        );
    }
    // A dimension added: a second categorical key between them, in the order received.
    rig.set_stain(FakeStain {
        out: vec![
            Dimension::Categorical {
                title: A,
                classes: vec![(Class::Swatch("bounded"), 1.0)],
            },
            Dimension::Categorical {
                title: C,
                // R-444 removed "degenerate": another class of the nine (amended by qa for TASK-M2-32).
                classes: vec![(Class::Swatch("triple_ejection"), 1.0)],
            },
            Dimension::Brightness {
                title: B,
                lo: 0.0,
                hi: 10.0,
            },
        ],
    });
    let texts = legend_texts(&mut rig);
    check_keys(&texts, &[A, C, B], &all);
    // The brightness channel removed: its bar goes.
    rig.set_stain(stain(A, vec![(Class::Swatch("bounded"), 1.0)], None));
    let texts = legend_texts(&mut rig);
    check_keys(&texts, &[A], &all);
    // No keys: an empty frame, still called Legend, with no key title and no class.
    rig.set_stain(FakeStain { out: Vec::new() });
    let texts = legend_texts(&mut rig);
    check_keys(&texts, &[], &all);
    assert!(
        texts.iter().any(|t| t == "Legend"),
        "the empty frame lost its title: {texts:?}"
    );
    for s in SWATCHES.iter() {
        assert!(
            !texts.iter().any(|t| t == s.label),
            "the empty legend draws `{}`: {texts:?}",
            s.label
        );
    }
    rejects("a missing key", || check_keys(&texts, &[A], &all));
    rejects("keys out of order", || {
        check_keys(
            &[
                "qa colour · gamma".to_owned(),
                "qa colour · alpha".to_owned(),
            ],
            &[A, C],
            &all,
        )
    });
}

fn px(shot: &Shot, x: usize, y: usize) -> [u8; 3] {
    let i = (y * shot.size[0] as usize + x) * 4;
    [shot.rgba[i], shot.rgba[i + 1], shot.rgba[i + 2]]
}

fn near(a: [u8; 3], b: [u8; 3]) -> bool {
    a.iter().zip(b).all(|(x, y)| x.abs_diff(y) <= 1)
}

/// The pixels `[x0, x1) × [y0, y1)` are R-399's "not yet" style: the grey, with a dot wherever `debug_not_yet`'s CPU
/// mirror puts one by the pixel's position (`dot_at`), each within 8-bit rounding; both colours present, so not flat.
fn check_not_yet(shot: &Shot, [x0, y0, x1, y1]: [usize; 4], dot_at: impl Fn(usize, usize) -> bool) {
    assert!(
        x1 >= x0 + 4 && y1 >= y0 + 4,
        "the swatch is {}×{} pixels",
        x1 - x0,
        y1 - y0
    );
    let (mut grey, mut dots) = (0, 0);
    for y in y0..y1 {
        for x in x0..x1 {
            let (got, dot) = (px(shot, x, y), dot_at(x, y));
            let want = if dot {
                NOT_YET_DOT_SRGB8
            } else {
                NOT_YET_SRGB8
            };
            assert!(
                near(got, want),
                "the pixel ({x}, {y}) is {got:?}, not the not-yet style's {want:?}"
            );
            if dot {
                dots += 1;
            } else {
                grey += 1;
            }
        }
    }
    assert!(
        dots > 0 && grey > 0,
        "the swatch is flat: {dots} dots, {grey} grey"
    );
}

#[test]
fn qa_legend_running_swatch_is_the_not_yet_style() {
    let canvas = Arc::new(MockCanvas::new().expect("a GPU adapter for the mock's canvas"));
    let mut rig = Rig::new(MockSide::new(MockEngine::frozen(), Some(canvas.clone())));
    rig.pointer_gone();
    // R-446 takes `running` out of the outcome key; the "not yet" style stays a class a key of another field shows
    // (R-399): a stain whose field holds no value for its running samples (amended by qa for TASK-M2-32).
    rig.set_stain(stain(
        "qa colour · d_min",
        vec![(Class::Swatch("bounded"), 3.0), (Class::Running, 1.0)],
        None,
    ));
    let legend = layout().legend;
    let label = rig
        .texts_in(legend)
        .into_iter()
        .find(|(t, _)| t == "running")
        .expect("the stain's legend has no running class")
        .1;
    let shot = render_frame(&canvas, &mut rig.app, &mut rig.h);
    let p = PIXELS_PER_POINT;
    // The swatch: the not-yet pixels on the label's row, left of it.
    // The label's row, half a label height above and below, so the whole swatch beside it is in it.
    let half = label.height() / 2.0;
    let (ya, yb) = (
        ((label.min.y - half) * p) as usize,
        ((label.max.y + half) * p).ceil() as usize,
    );
    // Searched for within four label heights left of the label: the swatch sits beside its label, and the other
    // column's texts, whose anti-aliased edges pass through these greys, lie farther left.
    let xb = (label.min.x * p) as usize;
    let xa = ((legend.min.x * p) as usize).max(xb - (4.0 * label.height() * p) as usize);
    let (mut x0, mut y0, mut x1, mut y1) = (usize::MAX, usize::MAX, 0, 0);
    for y in ya..yb {
        for x in xa..xb {
            let c = px(&shot, x, y);
            if near(c, NOT_YET_SRGB8) || near(c, NOT_YET_DOT_SRGB8) {
                (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1));
            }
        }
    }
    assert!(x1 > x0, "no not-yet pixel left of `running`");
    // Inside its outline: two pixels in from each edge.
    let interior = [x0 + 2, y0 + 2, x1 - 2, y1 - 2];
    let dot_at = |x: usize, y: usize| not_yet_dot_at([x as f64 + 0.5, y as f64 + 0.5]);
    check_not_yet(&shot, interior, dot_at);
    // Controls: the stipple read one pixel out of phase; a flat grey.
    rejects("the stipple one pixel off its position", || {
        check_not_yet(&shot, interior, |x, y| dot_at(x + 1, y))
    });
    let flat = Shot {
        size: shot.size,
        rgba: shot
            .rgba
            .chunks(4)
            .flat_map(|_| [NOT_YET_SRGB8[0], NOT_YET_SRGB8[1], NOT_YET_SRGB8[2], 255])
            .collect(),
        names: Vec::new(),
    };
    rejects("a flat grey swatch", || {
        check_not_yet(&flat, interior, dot_at)
    });
}

// --- REQ-GUI-172: the hover label, the hover path, the Trajectory panel ----------------------------------------------

/// The hover label's text names a fate only once it is decided: before, "undecided · still interacting at t … of
/// t_max" and no fate; after, the fate and no "undecided".
fn check_label(label: &str, fate: Option<&str>, t_max: f64) {
    match fate {
        None => {
            assert!(
                label.contains("undecided")
                    && label.contains("still interacting at t")
                    && label.contains(&format!("of {t_max}")),
                "an undecided sample's label reads `{label}`"
            );
            for s in SWATCHES.iter() {
                assert!(
                    !label.contains(s.label),
                    "an undecided sample's label names the fate `{}`: `{label}`",
                    s.label
                );
            }
        }
        Some(f) => {
            assert!(
                label.contains(f),
                "the label `{label}` does not name the fate `{f}`"
            );
            assert!(
                !label.contains("undecided"),
                "a decided sample's label reads `{label}`"
            );
        }
    }
}

/// The texts over the figure.
fn figure_texts(rig: &mut Rig) -> Vec<(String, Rect)> {
    rig.texts_in(layout().figure)
}

/// The texts drawn in the Trajectory panel, joined.
fn trajectory_text(rig: &mut Rig) -> String {
    rig.texts_in(layout().trajectory)
        .into_iter()
        .map(|(t, _)| t)
        .collect::<Vec<_>>()
        .join(" | ")
}

#[test]
fn qa_hover_label_path_and_under_cursor_follow_the_sample() {
    let figure = layout().figure;
    let mut rig = frozen();
    rig.pointer_gone();
    let base_texts: Vec<String> = figure_texts(&mut rig).into_iter().map(|(t, _)| t).collect();
    let base_shapes = over_figure(&rig.shapes(), figure);
    let s0 = rig.snap();
    let t_max = s0.sim.horizon.t;
    assert!(
        s0.sample.is_none() && s0.render.hover.at.is_none(),
        "a sample with no hover"
    );
    // The pointer over the figure at (s, t) = (0.3, 0.7).
    let n0 = rig.contract().len();
    rig.pointer_to(figure_pos(figure, 0.3, 0.7));
    let s1 = rig.snap();
    let entries = rig.since(n0);
    assert_eq!(
        entries.len(),
        1,
        "the hover applied {entries:?}, not one SetField"
    );
    assert!(
        is_set_field(&entries[0], "Hover.at", true),
        "the hover applied `{}`, not a no-history SetField on the cursor",
        entries[0]
    );
    assert_eq!(
        (s1.history.undo_depth, s1.history.redo_depth),
        (s0.history.undo_depth, s0.history.redo_depth),
        "the hover entered the history (R-101)"
    );
    let at = s1.render.hover.at.expect("the hover wrote no cursor");
    // Within a pixel of (0.3, 0.7), t Y-up.
    let pixel = 1.0 / f64::from(figure.width() * PIXELS_PER_POINT);
    assert!(
        (at[0] - 0.3).abs() <= pixel && (at[1] - 0.7).abs() <= pixel,
        "the cursor is at {at:?}, not (0.3, 0.7)"
    );
    let sample = s1
        .sample
        .clone()
        .expect("the snapshot carries no sample under the cursor");
    assert_eq!(sample.at, at, "the sample is not the cursor's IC");
    // The label: the one text the hover adds over the figure.
    let label_of = |rig: &mut Rig| -> Vec<(String, Rect)> {
        figure_texts(rig)
            .into_iter()
            .filter(|(t, _)| !base_texts.contains(t))
            .collect()
    };
    let label = label_of(&mut rig);
    assert_eq!(
        label.len(),
        1,
        "the hover drew {label:?} over the figure, not one label"
    );
    check_label(&label[0].0, sample.fate.as_deref(), t_max);
    // The GUI draws nothing over the figure but the label: every shape the hover adds lies within the label's backing.
    let hovered = rig.shapes();
    let added: Vec<Rect> = hovered
        .iter()
        .filter(|s| s.visual_bounding_rect().intersects(figure.shrink(1.0)))
        .filter(|s| {
            let r = s.visual_bounding_rect();
            !base_shapes.iter().any(|b| b.ends_with(&format!("{r:?}")))
        })
        .map(|s| s.visual_bounding_rect())
        .collect();
    let backing = added
        .iter()
        .copied()
        .reduce(|a, b| a.union(b))
        .expect("the hover added nothing over the figure");
    assert!(
        backing.contains_rect(label[0].1) && backing.width() <= label[0].1.width() + 40.0,
        "the GUI drew over the figure beyond the label: {added:?}, the label at {:?}",
        label[0].1
    );
    // The mock draws the hover path: solid at the cursor's IC, which is on the slice, fainter where it leaves it.
    let marks = rig.marks();
    assert!(marks.len() > 2, "the mock draws no hover path");
    let solid = marks.iter().map(|(_, c)| c[3]).fold(0.0f32, f32::max);
    let faint = marks.iter().map(|(_, c)| c[3]).fold(1.0f32, f32::min);
    assert!(
        faint < solid,
        "the hover path is one opacity throughout: {solid}"
    );
    let nearest = marks
        .iter()
        .min_by(|a, b| {
            let d = |m: &([[f32; 2]; 2], [f32; 4])| {
                m.0.iter()
                    .map(|p| (f64::from(p[0]) - at[0]).hypot(f64::from(p[1]) - at[1]))
                    .fold(f64::MAX, f64::min)
            };
            d(a).total_cmp(&d(b))
        })
        .unwrap();
    assert_eq!(
        nearest.1[3], solid,
        "the path is not solid at the cursor's IC, on the slice"
    );
    // The Trajectory panel's "under cursor" tab shows the same sample.
    let panel = trajectory_text(&mut rig);
    match &sample.fate {
        None => assert!(
            panel.contains("undecided"),
            "the panel does not show the undecided sample: {panel}"
        ),
        Some(f) => assert!(
            panel.contains(f.as_str()),
            "the panel does not show `{f}`: {panel}"
        ),
    }
    // At the horizon every sample is decided (a terminal state, or bounded at the horizon): the label names its fate,
    // and the panel shows it.
    rig.app.set_field(SetField {
        edit: Edit::Render(RenderField::Playhead(Playhead { t: t_max })),
        no_history: false,
    });
    rig.idle(3);
    let decided = rig.snap().sample.expect("the sample went at the horizon");
    let fate = decided
        .fate
        .clone()
        .expect("at the horizon the sample is still undecided");
    let label = label_of(&mut rig);
    assert_eq!(label.len(), 1, "at the horizon the hover drew {label:?}");
    check_label(&label[0].0, Some(&fate), t_max);
    let panel = trajectory_text(&mut rig);
    assert!(
        panel.contains(fate.as_str()),
        "the panel does not show the fate `{fate}`: {panel}"
    );
    // Moving off the figure clears both.
    let n1 = rig.contract().len();
    rig.pointer_to(layout().footer.center());
    let s2 = rig.snap();
    let entries = rig.since(n1);
    assert!(
        entries.len() == 1 && is_set_field(&entries[0], "Hover.at", true),
        "moving off applied {entries:?}, not one no-history SetField on the cursor"
    );
    assert!(
        s2.render.hover.at.is_none(),
        "moving off left the cursor at {:?}",
        s2.render.hover.at
    );
    assert!(s2.sample.is_none(), "moving off left a sample");
    assert!(label_of(&mut rig).is_empty(), "moving off left the label");
    assert!(rig.marks().is_empty(), "moving off left the hover path");
    let panel = trajectory_text(&mut rig);
    assert!(
        !panel.contains(fate.as_str()),
        "moving off left the sample in the panel: {panel}"
    );
    // Controls.
    rejects("an undecided label naming a fate", || {
        check_label(
            &format!(
                "undecided · still interacting at t 1.0 of {t_max} · {}",
                swatch_label("bounded")
            ),
            None,
            t_max,
        )
    });
    rejects("a decided label reading undecided", || {
        check_label("undecided", Some(&fate), t_max)
    });
}

// --- REQ-GUI-172: the keyboard's scopes (§G3) -----------------------------------------------------------------------

/// The big scopes a Tab lands on, in order, from `labels`, holds §G3's 3 to 7 consecutively.
fn check_order(landed: &[String]) {
    let want = ["Figure", "Trajectory", "Compass", "Time", "Legend"];
    let i = landed
        .iter()
        .position(|l| l == "Figure")
        .unwrap_or_else(|| panic!("Tab never lands on the Figure: {landed:?}"));
    assert!(
        landed.len() >= i + want.len() && landed[i..i + want.len()] == want,
        "the big scopes after the Figure are {:?}, not §G3's {want:?}",
        &landed[i..landed.len().min(i + want.len())]
    );
    assert!(
        i >= 1 && landed[i - 1] == "Manifold view",
        "the Figure does not follow Manifold view: {landed:?}"
    );
}

#[test]
fn qa_keyboard_scopes_in_g3_order_and_the_legend_is_read_only() {
    let mut rig = frozen();
    rig.focus("top_bar");
    let mut landed = vec![rig.at().0[0].clone()];
    for _ in 0..8 {
        rig.key(Key::Tab, NONE);
        let (labels, mode) = rig.at();
        assert_eq!(
            labels.len(),
            1,
            "a Tab did not land on a big scope itself: {labels:?}"
        );
        assert_eq!(mode, NAV, "a Tab landed on {labels:?} in interaction mode");
        landed.push(labels[0].clone());
    }
    check_order(&landed);
    // Shift+Tab walks it back.
    rig.focus("legend");
    let mut back = vec!["Legend".to_owned()];
    for _ in 0..4 {
        rig.key(Key::Tab, SHIFT);
        back.push(rig.at().0[0].clone());
    }
    back.reverse();
    assert_eq!(
        back,
        ["Figure", "Trajectory", "Compass", "Time", "Legend"],
        "Shift+Tab's order"
    );
    // The Legend is read-only: Enter on it changes nothing.
    rig.focus("legend");
    let (before, n0) = (rig.at(), rig.contract().len());
    rig.key(Key::Enter, NONE);
    assert_eq!(
        rig.at(),
        before,
        "Enter on the Legend moved the focus or the mode"
    );
    assert_eq!(
        rig.contract().len(),
        n0,
        "Enter on the Legend applied a SetField"
    );
    // The Trajectory panel: Enter goes in; its elements include the playhead, listen and the kept orbits.
    rig.focus("trajectory");
    rig.key(Key::Enter, NONE);
    let (labels, mode) = rig.at();
    assert_eq!(
        (labels.len(), mode),
        (2, NAV),
        "Enter on Trajectory: {labels:?} {mode:?}"
    );
    let tree = rig.app.keyboard.tree(Mode::Explore);
    let kids: Vec<String> = tree
        .children(Some("trajectory"))
        .into_iter()
        .map(|id| tree.get(id).unwrap().label.to_lowercase())
        .collect();
    for want in ["playhead", "listen", "kept orbits"] {
        assert!(
            kids.iter().any(|k| k.contains(want)),
            "Enter on Trajectory does not reach `{want}`: {kids:?}"
        );
    }
    let mut swapped = landed.clone();
    if let Some(i) = swapped.iter().position(|l| l == "Time") {
        swapped.swap(i, i - 1);
    }
    rejects("Compass and Time swapped", || check_order(&swapped));
}

// --- REQ-GUI-157: the sphere's turn ---------------------------------------------------------------------------------

/// The angle turned over `seconds` at `rate` degrees per second, in [0, 360).
fn turn_after(rate: f64, seconds: f64) -> f64 {
    (rate * seconds).rem_euclid(360.0)
}

fn check_turned(a0: f64, a1: f64, rate: f64, seconds: f64) {
    let want = turn_after(rate, seconds);
    let got = (a1 - a0).rem_euclid(360.0);
    let d = (got - want).abs();
    assert!(
        d.min(360.0 - d) <= 1e-6,
        "the sphere turned {got}° in {seconds} s, not {want}° at {rate}°/s"
    );
}

#[test]
fn qa_sphere_turns_in_degrees_per_second_frame_rate_independent() {
    use gui::explore::trajectory::{Scratch, TURN_DEG_PER_S};
    let rate = TURN_DEG_PER_S;
    assert!(rate > 0.0 && rate < 360.0, "the rate is {rate}°/s");
    // Frame-rate independent: one second in 1, 7 uneven, 30 and 144 frames turns the same.
    let run = |frames: &[f64]| {
        let mut s = Scratch::default();
        s.advance(0.0, 1.0);
        for &t in frames {
            s.advance(t, 1.0);
        }
        s.angle
    };
    let one = run(&[1.0]);
    let uneven = run(&[0.01, 0.2, 0.21, 0.5, 0.77, 0.9, 1.0]);
    let thirty: Vec<f64> = (1..=30).map(|i| f64::from(i) / 30.0).collect();
    let fast: Vec<f64> = (1..=144).map(|i| f64::from(i) / 144.0).collect();
    for (what, a) in [
        ("1 frame", one),
        ("7 uneven", uneven),
        ("30 fps", run(&thirty)),
        ("144 fps", run(&fast)),
    ] {
        assert!(
            (a - turn_after(rate, 1.0)).abs() <= 1e-9,
            "{what}: the sphere turned {a}° in one second, not {rate}°"
        );
    }
    // In the app, on the live mock: the turn reads the wall clock (egui's frame time, the headless frames' `step`), so
    // the same wall span turns the sphere the same at two frame steps, by the span times the rate (REQ-GUI-157).
    let span = 0.75;
    let (slow, slow_frames) = wall_turn(0.125, span);
    let (fast, fast_frames) = wall_turn(1.0 / 60.0, span);
    assert_ne!(
        slow_frames, fast_frames,
        "the two runs are at one frame rate"
    );
    for (a0, a1) in [slow, fast] {
        check_turned(a0, a1, rate, span);
    }
    // Clearing "turn" stops it, while wall time runs on.
    let mut rig = hovered(1.0 / 60.0);
    rig.focus(gui::explore::trajectory::TURN);
    rig.key(Key::Enter, NONE);
    assert!(!rig.app.trajectory.turn, "Enter on turn did not clear it");
    let a2 = rig.app.trajectory.angle;
    rig.idle(30);
    assert_eq!(
        rig.app.trajectory.angle, a2,
        "with turn cleared, the sphere turned"
    );
    // Capture mode's frozen clock holds it, so a capture is the same every run.
    let mut rig = frozen();
    rig.pointer_to(figure_pos(layout().figure, 0.4, 0.4));
    assert!(rig.snap().sample.is_some(), "no orbit under the cursor");
    let a3 = rig.app.trajectory.angle;
    rig.idle(30);
    assert_eq!(
        rig.app.trajectory.angle, a3,
        "on the frozen clock, the sphere turned"
    );
    // Controls: the turn read per frame (the mock's 60 Hz tick), and a span that is not the one run.
    rejects("a turn counted per tick at 1/8 s frames", || {
        check_turned(
            slow.0,
            slow.1,
            rate,
            slow_frames as f64 / gui::mock::clock::MOCK_TICK_HZ,
        )
    });
    rejects("a turn over half the span", || {
        check_turned(fast.0, fast.1, rate, span / 2.0)
    });
}

/// The live mock with an orbit under the cursor, its headless frames `step` seconds apart.
fn hovered(step: f64) -> Rig {
    let mut rig = running();
    rig.h.set_frame_step(step);
    let _ = rig.frame(vec![Event::PointerMoved(figure_pos(
        layout().figure,
        0.4,
        0.4,
    ))]);
    // The snapshot is read every `SNAPSHOT_INTERVAL_S` of frame time: run frames until it carries the sample.
    let mut n = 0;
    while rig.snap().sample.is_none() {
        n += 1;
        assert!(n < 1000, "no orbit under the cursor");
        rig.idle(1);
    }
    rig.idle(2);
    rig
}

/// The sphere's angle before and after `span` seconds of frames `step` apart, with how many frames that was.
fn wall_turn(step: f64, span: f64) -> ((f64, f64), usize) {
    let mut rig = hovered(step);
    let frames = (span / step).round() as usize;
    assert!(
        (frames as f64 * step - span).abs() <= 1e-12,
        "{span} s is not a whole number of {step} s frames"
    );
    let a0 = rig.app.trajectory.angle;
    rig.idle(frames);
    ((a0, rig.app.trajectory.angle), frames)
}

// --- REQ-GUI-172: the kept orbits' tabs -----------------------------------------------------------------------------

/// The orbit the Trajectory panel shows: the sample under the cursor or a kept one, by its tab.
fn shown(rig: &Rig) -> Option<engine::contract::sample::Sample> {
    gui::explore::trajectory::shown(
        rig.app.trajectory.tab,
        rig.app.snapshot().sample.as_ref(),
        &rig.app.view.kept_orbits,
    )
    .cloned()
}

/// A tab on a kept orbit still shows that orbit after another is kept (§G2: the panel is "for the IC under the cursor
/// or a kept orbit (tabs)"), or goes back to "under cursor" if its orbit was the one let go.
fn check_tab_kept(
    before: &engine::contract::sample::Sample,
    kept: &[engine::contract::sample::Sample],
    after: Option<&engine::contract::sample::Sample>,
    under: Option<&engine::contract::sample::Sample>,
) {
    if kept.contains(before) {
        assert_eq!(
            after,
            Some(before),
            "keeping another orbit moved the tab off its orbit"
        );
    } else {
        assert_eq!(
            after, under,
            "the tab's orbit went, and the tab is not back on \"under cursor\""
        );
    }
}

#[test]
fn qa_kept_orbit_tabs_stay_on_their_orbit_when_another_is_kept() {
    use gui::explore::trajectory::{KEEP, KEPT_IDS, MAX_KEPT};
    let figure = layout().figure;
    let mut rig = frozen();
    // Keep more orbits than the list holds, each from its own point, the panel's tab on one of them meanwhile.
    for i in 0..MAX_KEPT + 3 {
        let at = 0.1 + 0.8 * i as f32 / (MAX_KEPT + 3) as f32;
        rig.pointer_to(figure_pos(figure, at, 1.0 - at));
        assert!(
            rig.snap().sample.is_some(),
            "no orbit under the cursor at {at}"
        );
        let n = rig.app.view.kept_orbits.orbits.len();
        if n >= 3 {
            rig.focus(KEPT_IDS[2].1);
            rig.key(Key::Enter, NONE);
        }
        let before = shown(&rig);
        rig.focus(KEEP);
        rig.key(Key::Enter, NONE);
        let kept = rig.app.view.kept_orbits.orbits.clone();
        assert_eq!(
            kept.len(),
            (n + 1).min(MAX_KEPT),
            "keeping did not keep the orbit"
        );
        if let (Some(before), true) = (before, n >= 3) {
            check_tab_kept(
                &before,
                &kept,
                shown(&rig).as_ref(),
                rig.app.snapshot().sample.as_ref(),
            );
            rejects("a tab moved to the next orbit", || {
                check_tab_kept(&before, &kept, kept.get(3), None)
            });
        }
    }
}
