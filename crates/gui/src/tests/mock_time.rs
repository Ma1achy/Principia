//! `mock_time` (REQ-GUI-173; render_gui_spec §G2, §G3; R-404, R-409 G1–G6): Time plays through no-history `SetField`s
//! on the playhead, at the transport's speed, and stops at the horizon `T`; its step buttons move by one `TimeStep`,
//! each an undoable `SetField`; a scrub is one `SetField`, sent when the drag ends; Space plays in Time, in either mode;
//! in interaction mode on the scrubber, ← → step the time by `TimeStep`, on speed by `Bounded`. Each check also runs on
//! an input it must reject (R-176, through `rejects`).

use eframe::egui::{self, Event, Key, Modifiers, PointerButton};
use engine::contract::interface::EngineInterface;
use engine::contract::set_field::{Edit, RenderField, SetField};
use engine::contract::view_ui::{KeyMode, Transport};

use super::support::{focus, headless, mock_app, playhead, press, rejects};
use crate::app::App;
use crate::clock::{Clock, Playback, TimeSource};
use crate::explore::time::{
    self, moved, speed_after, time_step, toggle_play, BACK, PLAY, SCRUBBER, SPEED, SPEED_DEFAULT,
    SPEED_RANGE, STEP, TIME,
};
use crate::headless::Headless;
use crate::mock::clock::{MockClock, MOCK_DT};
use crate::mock::{MockEngine, MOCK_T_MAX};
use crate::side::MockSide;

const NONE: Modifiers = Modifiers::NONE;

/// The app after one frame.
fn app() -> (App<MockSide>, Headless) {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    (app, h)
}

/// Runs `n` empty frames.
fn frames(h: &mut Headless, app: &mut App<MockSide>, n: usize) {
    for _ in 0..n {
        let _ = h.frame(app, Vec::new());
    }
}

/// The state after one more frame, which reads a snapshot (frames are further apart than the app's read interval): the
/// playhead and the undo depth.
fn now(h: &mut Headless, app: &mut App<MockSide>) -> (f64, u32) {
    let _ = h.frame(app, Vec::new());
    let s = app.snapshot();
    (s.render.playhead.t, s.history.undo_depth)
}

/// The playhead's `SetField` lines in the console from row `from`.
fn playhead_lines(app: &App<MockSide>, from: usize) -> Vec<String> {
    app.console()[from..]
        .iter()
        .map(|e| e.message.clone())
        .filter(|m| m.starts_with("SetField Playhead"))
        .collect()
}

/// Clicks Time's button `id` where it was last drawn.
fn press_button(h: &mut Headless, app: &mut App<MockSide>, id: &str) {
    let rect = app.keyboard.place_of(id).expect("the button drawn").rect;
    for events in crate::capture::click(rect.center()) {
        let _ = h.frame(app, events);
    }
}

/// Sets the focus to `path` in `mode`.
fn focus_on(app: &mut App<MockSide>, path: &[&str], mode: KeyMode) {
    app.view.focus.path = path.iter().map(|s| (*s).to_owned()).collect();
    app.view.focus.mode = mode;
}

// --- Play: the clock's no-history edits, at the speed, up to T -------------------------------------------------------

/// Asserts the playhead's log `lines` are all no-history moves, and at least `min` of them.
fn check_no_history(lines: &[String], min: usize) {
    assert!(
        lines.len() >= min,
        "{} playhead edits: {lines:?}",
        lines.len()
    );
    for line in lines {
        assert!(line.ends_with("(no history)"), "{line}");
    }
}

#[test]
fn mock_time_play_advances_through_no_history_setfields() {
    let (mut app, mut h) = app();
    let from = app.console().len();
    press_button(&mut h, &mut app, PLAY);
    assert!(app.view.transport.playing, "▶ plays");
    frames(&mut h, &mut app, 30);
    let (t, depth) = now(&mut h, &mut app);
    assert!(
        t > 0.3 && t < 0.7,
        "about half a second of t at speed 1: {t}"
    );
    assert_eq!(depth, 0, "playing enters nothing in the history");
    let lines = playhead_lines(&app, from);
    check_no_history(&lines, 20);
    rejects("a playhead edit with history", || {
        check_no_history(&["SetField Playhead.t 0 → 0.5".to_owned()], 1)
    });
    // ⏸ pauses: the playhead stays.
    press_button(&mut h, &mut app, PLAY);
    assert!(!app.view.transport.playing);
    let (paused, _) = now(&mut h, &mut app);
    frames(&mut h, &mut app, 10);
    assert_eq!(
        now(&mut h, &mut app).0,
        paused,
        "paused, the playhead stays"
    );
}

/// The clock at `speed`, from `t`, over `ticks` ticks of a running mock clock: the playhead it last wrote.
fn played(speed: f64, t: f64, t_max: f64, ticks: usize) -> Option<f64> {
    let mut clock = Clock::new();
    let mut source = MockClock::running();
    let mut at = t;
    let mut last = None;
    for _ in 0..=ticks {
        let playback = Playback {
            playing: true,
            speed,
            t: at,
            t_max,
        };
        if let Some(SetField {
            edit: Edit::Render(RenderField::Playhead(p)),
            no_history,
        }) = clock.frame(&mut source, playback)
        {
            assert!(no_history);
            at = p.t;
            last = Some(p.t);
        }
    }
    last
}

#[test]
fn mock_time_speed_scales_play_and_stays_in_its_range() {
    let one = played(1.0, 0.0, MOCK_T_MAX, 60).unwrap();
    let two = played(2.0, 0.0, MOCK_T_MAX, 60).unwrap();
    assert!((one - 60.0 * MOCK_DT).abs() < 1e-9, "{one}");
    assert!(
        (two - 2.0 * one).abs() < 1e-9,
        "twice the speed, twice the t: {two}"
    );
    rejects("a speed that does not scale", || {
        assert!((one - two).abs() < 1e-9)
    });
    // The speed's steps: R-404's Bounded, 1/100 of its range, kept in it, at both ends.
    let (lo, hi) = SPEED_RANGE;
    let step = (hi - lo) / 100.0;
    assert!((speed_after(SPEED_DEFAULT, 0.01) - (SPEED_DEFAULT + step)).abs() < 1e-12);
    assert!((speed_after(SPEED_DEFAULT, -0.01) - (SPEED_DEFAULT - step)).abs() < 1e-12);
    assert_eq!(speed_after(hi, 0.01), hi);
    assert_eq!(speed_after(hi - step / 2.0, 0.01), hi);
    assert_eq!(speed_after(lo, -0.01), lo);
    assert_eq!(speed_after(lo + step / 2.0, -0.01), lo);
    assert_eq!(speed_after(hi, 0.0), hi);
    assert_eq!(speed_after(lo, 0.0), lo);
    rejects("a speed past its top", || {
        assert!(speed_after(hi, 0.01) > hi)
    });
    // The transport starts at the default speed.
    let (app, _) = app();
    assert_eq!(app.view.transport.speed, SPEED_DEFAULT);
}

#[test]
fn mock_time_play_stops_at_the_horizon() {
    // The clock: from just below T it writes T; from T, or past it, nothing.
    let below = MOCK_T_MAX - MOCK_DT / 2.0;
    assert_eq!(played(1.0, below, MOCK_T_MAX, 1), Some(MOCK_T_MAX));
    assert_eq!(played(1.0, MOCK_T_MAX, MOCK_T_MAX, 5), None);
    assert_eq!(played(1.0, MOCK_T_MAX + 1.0, MOCK_T_MAX, 5), None);
    assert_eq!(
        played(1.0, MOCK_T_MAX - 1.0, MOCK_T_MAX, 120),
        Some(MOCK_T_MAX)
    );
    rejects("a clock that plays past T", || {
        assert_eq!(
            played(1.0, MOCK_T_MAX - 1.0, MOCK_T_MAX, 120),
            Some(MOCK_T_MAX + 1.0)
        )
    });
    // A paused clock writes nothing.
    let mut clock = Clock::new();
    let mut source = MockClock::running();
    let paused = Playback {
        playing: false,
        speed: 1.0,
        t: 0.0,
        t_max: MOCK_T_MAX,
    };
    for _ in 0..3 {
        assert_eq!(clock.frame(&mut source, paused), None);
    }
    // The app: playing from just below T, the playhead reaches T and the transport stops.
    let (mut app, mut h) = app();
    app.set_field(playhead(MOCK_T_MAX - 0.1, false));
    frames(&mut h, &mut app, 2);
    press_button(&mut h, &mut app, PLAY);
    frames(&mut h, &mut app, 30);
    assert_eq!(now(&mut h, &mut app).0, MOCK_T_MAX);
    assert!(!app.view.transport.playing, "at T the transport stops");
    // ▶ at T plays again from 0: one entry in the history.
    let depth = now(&mut h, &mut app).1;
    press_button(&mut h, &mut app, PLAY);
    let (t, after) = now(&mut h, &mut app);
    assert!(t < 0.2, "from the start: {t}");
    assert_eq!(after, depth + 1);
}

#[test]
fn mock_time_toggle_play_restarts_only_at_the_horizon() {
    let (app, _) = app();
    let snapshot = app.snapshot().clone();
    let t_max = snapshot.sim.horizon.t;
    let paused = || Transport {
        playing: false,
        speed: 1.0,
    };
    let toggled = |t: f64, transport: &mut Transport| {
        let mut s = snapshot.clone();
        s.render.playhead.t = t;
        let mut edits = Vec::new();
        toggle_play(&s, transport, &mut edits);
        edits
    };
    let mut transport = paused();
    assert_eq!(toggled(t_max, &mut transport), [time::playhead_edit(0.0)]);
    assert!(transport.playing);
    let mut transport = paused();
    assert_eq!(
        toggled(t_max + 1.0, &mut transport),
        [time::playhead_edit(0.0)]
    );
    let mut transport = paused();
    assert!(toggled(t_max - 1e-9, &mut transport).is_empty());
    assert!(transport.playing);
    // Playing, at T, it pauses and moves nothing.
    let mut transport = Transport {
        playing: true,
        speed: 1.0,
    };
    assert!(toggled(t_max, &mut transport).is_empty());
    assert!(!transport.playing);
    rejects("a play from T that does not restart", || {
        let mut transport = paused();
        assert!(toggled(t_max, &mut transport).is_empty())
    });
}

// --- Step and back: one TimeStep, each an undoable SetField ------------------------------------------------------------

#[test]
fn mock_time_step_is_one_time_step() {
    assert_eq!(time_step(MOCK_T_MAX), MOCK_T_MAX / 100.0);
    assert_eq!(moved(10.0, 0.5, 50.0), 10.5);
    assert_eq!(moved(49.8, 0.5, 50.0), 50.0);
    assert_eq!(moved(50.0, 0.0, 50.0), 50.0);
    assert_eq!(moved(0.2, -0.5, 50.0), 0.0);
    assert_eq!(moved(0.0, 0.0, 50.0), 0.0);
    rejects("a step past T", || assert!(moved(49.8, 0.5, 50.0) > 50.0));
    let (mut app, mut h) = app();
    let from = app.console().len();
    press_button(&mut h, &mut app, STEP);
    let (t, depth) = now(&mut h, &mut app);
    assert_eq!(t, time_step(MOCK_T_MAX));
    assert_eq!(depth, 1, "one undoable SetField");
    let lines = playhead_lines(&app, from);
    assert_eq!(lines.len(), 1, "{lines:?}");
    rejects("a step that left no entry", || assert_eq!(depth, 0));
    press_button(&mut h, &mut app, BACK);
    let (t, depth) = now(&mut h, &mut app);
    assert_eq!((t, depth), (0.0, 2));
    // Back at 0 stays at 0, still one entry.
    press_button(&mut h, &mut app, BACK);
    assert_eq!(now(&mut h, &mut app), (0.0, 3));
    // Undo takes the steps back.
    app.undo();
    app.undo();
    assert_eq!(now(&mut h, &mut app).0, time_step(MOCK_T_MAX));
}

#[test]
fn mock_time_enter_on_a_button_is_its_click() {
    for (id, want) in [(STEP, time_step(MOCK_T_MAX)), (BACK, 0.0)] {
        let (mut app, mut h) = app();
        app.set_field(playhead(0.0, false));
        frames(&mut h, &mut app, 2);
        focus_on(&mut app, &[TIME, id], KeyMode::Navigation);
        press(&mut h, &mut app, Key::Enter, NONE);
        assert_eq!(now(&mut h, &mut app).0, want, "{id}");
        assert_eq!(
            app.view.focus.mode,
            KeyMode::Navigation,
            "a button starts no interaction"
        );
    }
    let (mut app, mut h) = app();
    focus_on(&mut app, &[TIME, PLAY], KeyMode::Navigation);
    press(&mut h, &mut app, Key::Enter, NONE);
    assert!(app.view.transport.playing, "Enter on ▶ plays");
    rejects("an Enter on ▶ that does nothing", || {
        assert!(!app.view.transport.playing)
    });
}

// --- The scrub: one SetField, sent on release -------------------------------------------------------------------------

/// Drags the scrubber from fraction `from` of its rail to each of `to`, then releases: the frames' events.
fn drag(app: &App<MockSide>, from: f32, to: &[f32]) -> Vec<Vec<Event>> {
    let rail = app
        .keyboard
        .place_of(SCRUBBER)
        .expect("the scrubber drawn")
        .rect;
    let at = |f: f32| {
        egui::pos2(
            rail.left() + 8.0 + (rail.width() - 16.0) * f,
            rail.center().y,
        )
    };
    let button = |pos, pressed| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: NONE,
    };
    let mut frames = vec![
        vec![Event::PointerMoved(at(from))],
        vec![button(at(from), true)],
    ];
    for f in to {
        frames.push(vec![Event::PointerMoved(at(*f))]);
    }
    let end = at(*to.last().unwrap_or(&from));
    frames.push(vec![button(end, false)]);
    frames.push(Vec::new());
    frames
}

#[test]
fn mock_time_scrub_is_one_setfield_on_release() {
    let (mut app, mut h) = app();
    let from = app.console().len();
    let events = drag(&app, 0.1, &[0.3, 0.5, 0.7]);
    let n = events.len();
    let mut during = Vec::new();
    for (i, e) in events.into_iter().enumerate() {
        let _ = h.frame(&mut app, e);
        if i + 2 < n {
            during.push((app.snapshot().history.undo_depth, app.time.scrub));
        }
    }
    // While dragged, nothing is sent: the scrubber previews the time.
    assert!(during.iter().all(|(depth, _)| *depth == 0), "{during:?}");
    assert!(
        during.iter().any(|(_, s)| s.is_some()),
        "the drag previews: {during:?}"
    );
    let (t, depth) = now(&mut h, &mut app);
    assert_eq!(depth, 1, "one SetField for the scrub");
    assert_eq!(playhead_lines(&app, from).len(), 1);
    assert!(
        (t - 0.7 * MOCK_T_MAX).abs() < 0.05 * MOCK_T_MAX,
        "the release's time: {t}"
    );
    assert_eq!(app.time.scrub, None, "the preview ends with the drag");
    rejects("a scrub of three SetFields", || assert_eq!(depth, 3));
    // A click on the rail, no drag: one SetField too.
    for e in drag(&app, 0.2, &[]) {
        let _ = h.frame(&mut app, e);
    }
    let (t, depth) = now(&mut h, &mut app);
    assert_eq!(depth, 2);
    assert!((t - 0.2 * MOCK_T_MAX).abs() < 0.05 * MOCK_T_MAX, "{t}");
}

// --- The keys: Space plays in Time; the arrows on the scrubber and on speed in interaction mode ------------------------

#[test]
fn mock_time_space_plays_in_time_in_either_mode() {
    for (path, mode) in [
        (&[TIME][..], KeyMode::Navigation),
        (&[TIME, SCRUBBER][..], KeyMode::Navigation),
        (&[TIME, SCRUBBER][..], KeyMode::Interaction),
    ] {
        let (mut app, mut h) = app();
        focus_on(&mut app, path, mode);
        press(&mut h, &mut app, Key::Space, NONE);
        assert!(
            app.view.transport.playing,
            "Space plays at {path:?} in {mode:?}"
        );
        press(&mut h, &mut app, Key::Space, NONE);
        assert!(!app.view.transport.playing, "Space again pauses");
    }
    // Outside Time, Space plays nothing.
    let (mut app, mut h) = app();
    focus_on(&mut app, &[crate::explore::COMPASS], KeyMode::Navigation);
    press(&mut h, &mut app, Key::Space, NONE);
    let playing = app.view.transport.playing;
    assert!(!playing);
    rejects("Space in the Compass playing", || assert!(playing));
}

/// The playhead and speed after `keys` on Time's `element`, after Enter starts interaction mode.
fn after_keys(element: &str, keys: &[(Key, Modifiers)]) -> (f64, f64, u32) {
    let (mut app, mut h) = app();
    focus_on(&mut app, &[TIME, element], KeyMode::Navigation);
    press(&mut h, &mut app, Key::Enter, NONE);
    assert_eq!(
        app.view.focus.mode,
        KeyMode::Interaction,
        "Enter on {element}"
    );
    for (key, modifiers) in keys {
        press(&mut h, &mut app, *key, *modifiers);
    }
    let (t, depth) = now(&mut h, &mut app);
    (t, app.view.transport.speed, depth)
}

#[test]
fn mock_time_arrows_step_the_scrubber_in_interaction_mode() {
    let step = time_step(MOCK_T_MAX);
    let right = (Key::ArrowRight, NONE);
    let left = (Key::ArrowLeft, NONE);
    let (t, _, depth) = after_keys(SCRUBBER, &[right, right]);
    assert!((t - 2.0 * step).abs() < 1e-9, "→ → is two steps: {t}");
    assert_eq!(depth, 2, "each step an undoable SetField");
    let (t, _, _) = after_keys(SCRUBBER, &[right, right, left]);
    assert!((t - step).abs() < 1e-9, "{t}");
    let (t, _, _) = after_keys(SCRUBBER, &[(Key::ArrowRight, Modifiers::SHIFT)]);
    assert!((t - 10.0 * step).abs() < 1e-9, "Shift is ×10: {t}");
    let (t, _, _) = after_keys(SCRUBBER, &[left]);
    assert_eq!(t, 0.0, "← at 0 stays at 0");
    rejects("a step of a whole unit of t", || {
        let (t, _, _) = after_keys(SCRUBBER, &[right]);
        assert!((t - 1.0).abs() < 1e-9)
    });
    // On speed, the arrows move the speed by Bounded and leave the playhead.
    let (lo, hi) = SPEED_RANGE;
    let (t, speed, depth) = after_keys(SPEED, &[right]);
    assert!(
        (speed - (SPEED_DEFAULT + (hi - lo) / 100.0)).abs() < 1e-12,
        "{speed}"
    );
    assert_eq!(
        (t, depth),
        (0.0, 0),
        "the speed is the transport's, no SetField"
    );
    // In navigation mode the arrows move between Time's elements, not the time.
    let (mut app, mut h) = app();
    focus_on(&mut app, &[TIME, SPEED], KeyMode::Navigation);
    press(&mut h, &mut app, Key::ArrowRight, NONE);
    assert_eq!(focus(&app), [TIME, SCRUBBER]);
    assert_eq!(now(&mut h, &mut app).0, 0.0);
}

#[test]
fn mock_time_scopes_in_order_and_their_kinds() {
    let (app, _) = app();
    let tree = app.keyboard.tree(engine::contract::view_ui::Mode::Explore);
    assert_eq!(
        tree.children(Some(TIME)),
        [BACK, PLAY, STEP, SPEED, SCRUBBER]
    );
    use crate::keyboard::scopes::{Arrows, StepKind};
    assert_eq!(
        tree.get(SCRUBBER).map(|s| s.arrows),
        Some(Arrows::Adjust(StepKind::TimeStep))
    );
    assert_eq!(
        tree.get(SPEED).map(|s| s.arrows),
        Some(Arrows::Adjust(StepKind::Bounded))
    );
    for id in [BACK, PLAY, STEP] {
        assert!(tree.get(id).unwrap().activates, "{id}");
    }
    rejects("Time's elements in another order", || {
        assert_eq!(
            tree.children(Some(TIME)),
            [PLAY, BACK, STEP, SPEED, SCRUBBER]
        )
    });
}

#[test]
fn mock_time_readouts_and_ticks() {
    assert_eq!(time::readout(12.4), "t 12.40");
    assert_eq!(time::speed_readout(1.0), "× 1.00");
    assert_eq!(time::number(50.0), "50");
    assert_eq!(time::number(12.5), "12.5");
    let ticks: Vec<String> = time::ticks(50.0).into_iter().map(|(_, s)| s).collect();
    assert_eq!(ticks, ["0", "10", "20", "30", "40", "50"]);
    rejects("ticks of another horizon", || {
        let t: Vec<String> = time::ticks(40.0).into_iter().map(|(_, s)| s).collect();
        assert_eq!(t, ["0", "10", "20", "30", "40", "50"])
    });
    // The mock's horizon is 01_main.png's 50. Running, its clock holds no seconds, so what turns reads the wall clock;
    // frozen, it holds them at its start.
    let mut engine = MockEngine::new();
    assert_eq!(engine.snapshot().sim.horizon.t, MOCK_T_MAX);
    let clock = engine.clock();
    for _ in 0..30 {
        clock.tick();
    }
    assert_eq!(clock.held_seconds(), None);
    rejects("a running clock's ticks as seconds", || {
        assert_eq!(clock.held_seconds(), Some(0.5))
    });
    let mut frozen = MockClock::frozen();
    for _ in 0..30 {
        frozen.tick();
    }
    assert_eq!(frozen.held_seconds(), Some(0.0));
}

/// A time source that holds no seconds, by the trait's default.
struct Fixed(f64);

impl TimeSource for Fixed {
    fn tick(&mut self) {}
    fn now(&self) -> f64 {
        self.0
    }
    fn rate_hz(&self) -> f64 {
        60.0
    }
}

#[test]
fn mock_time_a_source_holds_no_seconds_by_default() {
    assert_eq!(Fixed(2.5).held_seconds(), None);
    rejects("its now held as its seconds", || {
        assert_eq!(Fixed(2.5).held_seconds(), Some(2.5))
    });
}
