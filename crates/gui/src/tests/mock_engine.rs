//! `mock_engine` (REQ-GUI-165; R-390, RQ-245, RQ-246): a SetField shows in the next snapshot; undo and redo restore and
//! reapply it; a no-history edit leaves the history unchanged; the fake clock, run through the app's clock on the
//! mock's tick, advances the playhead through no-history SetFields while playing and holds it while paused, and the
//! mock reads no `ViewUI`; events arrive only in the snapshot, as the entries since the previous snapshot, one
//! `contract` info entry per applied SetField.

use engine::contract::interface::EngineInterface;
use engine::contract::log::{Severity, Source};
use engine::contract::snapshot::History;

use super::support::{headless, mock_app, playhead, rejects};
use crate::clock::{Clock, TimeSource};
use crate::mock::clock::{MockClock, MOCK_DT, MOCK_TICK_HZ};
use crate::mock::{MockEngine, MOCK_ERROR, MOCK_FRAME, MOCK_START_MS, MOCK_WARNING};
use crate::side::EngineSide;

fn history(undo_depth: u32, redo_depth: u32) -> History {
    History {
        undo_depth,
        redo_depth,
    }
}

/// The mock applies, undoes and redoes as the contract says.
fn check_history(mock: &mut MockEngine) {
    mock.set_field(playhead(1.5, false));
    let s = mock.snapshot();
    assert_eq!(
        s.render.playhead.t, 1.5,
        "the SetField is not in the next snapshot"
    );
    assert_eq!(s.history, history(1, 0));
    mock.set_field(playhead(2.5, true));
    assert_eq!(
        mock.snapshot().history,
        history(1, 0),
        "a no-history edit entered the history"
    );
    mock.undo();
    let s = mock.snapshot();
    assert_eq!(
        s.render.playhead.t, 0.0,
        "undo did not restore the playhead"
    );
    assert_eq!(s.history, history(0, 1));
    mock.redo();
    let s = mock.snapshot();
    assert_eq!(s.render.playhead.t, 1.5, "redo did not reapply the edit");
    assert_eq!(s.history, history(1, 0));
    // Past the ends of the history, undo and redo do nothing; a new edit clears the redo history.
    mock.redo();
    assert_eq!(mock.snapshot().render.playhead.t, 1.5);
    mock.undo();
    mock.undo();
    assert_eq!(mock.snapshot().render.playhead.t, 0.0);
    mock.set_field(playhead(4.0, false));
    assert_eq!(
        mock.snapshot().history,
        history(1, 0),
        "a new edit must clear redo"
    );
}

#[test]
fn mock_engine_applies_undoes_and_redoes() {
    check_history(&mut MockEngine::new());
    // A mock already at 1.5 with one entry: the first check's "undo restores 0" fails.
    rejects("a mock already holding an edit", || {
        let mut mock = MockEngine::new();
        mock.set_field(playhead(9.0, false));
        mock.undo();
        mock.set_field(playhead(7.0, false));
        check_history(&mut mock);
    });
}

/// The mock's snapshot is plausible and GUI-sized: its frame summary is the mock's, its precision flags clear.
#[test]
fn mock_engine_serves_plausible_snapshots() {
    let s = MockEngine::new().snapshot();
    assert_eq!(s.frame, MOCK_FRAME);
    assert!(s.frame.fps.is_some() && s.frame.quad_count.is_some() && s.frame.live_memory.is_some());
    assert!(!s.precision.decode_switchover && !s.precision.at_f32_floor);
    assert!(s.log.is_empty(), "a fresh mock logged {:?}", s.log);
}

/// Events arrive only in the snapshot: each applied SetField's one `contract` info entry, and each raised warning and
/// error, in the next snapshot and no later one.
fn check_events(mock: &mut MockEngine) {
    mock.set_field(playhead(1.0, false));
    mock.set_field(playhead(2.0, true));
    mock.raise_warning();
    mock.raise_error();
    let log = mock.snapshot().log;
    let got: Vec<_> = log
        .iter()
        .map(|e| (e.severity, e.source, e.message.as_str()))
        .collect();
    assert_eq!(
        got,
        [
            (
                Severity::Info,
                Source::Contract,
                "SetField Playhead.t 0 → 1"
            ),
            (
                Severity::Info,
                Source::Contract,
                "SetField Playhead.t 1 → 2 (no history)"
            ),
            (Severity::Warn, Source::Stain, MOCK_WARNING),
            (Severity::Error, Source::Integrator, MOCK_ERROR),
        ]
    );
    assert!(log.iter().all(|e| e.at == MOCK_START_MS));
    assert!(
        mock.snapshot().log.is_empty(),
        "an entry was in two snapshots"
    );
}

#[test]
fn mock_engine_events_arrive_once_in_the_snapshot() {
    check_events(&mut MockEngine::new());
    rejects("a mock whose earlier entries are still undelivered", || {
        let mut mock = MockEngine::new();
        mock.raise_warning();
        check_events(&mut mock);
    });
}

/// The entries' `at` moves with the mock's clock: 60 ticks are one second at the mock's rate.
#[test]
fn mock_engine_entry_time_follows_its_clock() {
    let mut mock = MockEngine::new();
    for _ in 0..MOCK_TICK_HZ as u32 {
        mock.clock().tick();
    }
    assert_eq!(mock.clock().elapsed_ms(), 1000);
    mock.raise_warning();
    assert_eq!(mock.snapshot().log[0].at, MOCK_START_MS + 1000);
}

/// The mock reads no `ViewUI`: its own clock ticking never moves the playhead, whatever the transport; only the app's
/// clock's SetFields do.
#[test]
fn mock_engine_reads_no_view_ui() {
    let mut mock = MockEngine::new();
    for _ in 0..30 {
        mock.clock().tick();
    }
    assert_eq!(mock.clock().now(), 30.0 * MOCK_DT);
    let s = mock.snapshot();
    assert_eq!(s.render.playhead.t, 0.0, "the mock moved its own playhead");
    assert!(s.log.is_empty());
}

/// The frozen clock, as capture mode runs it, never moves.
#[test]
fn mock_engine_frozen_clock_holds() {
    let mut clock = MockClock::frozen();
    clock.tick();
    assert_eq!((clock.now(), clock.elapsed_ms()), (0.0, 0));
    let mut running = MockClock::running();
    running.tick();
    assert_eq!(running.now(), MOCK_DT);
    assert_eq!(running.rate_hz(), MOCK_TICK_HZ);
}

/// The app's clock on the mock's tick: while playing, each frame advances the playhead by the mock's `dt` through a
/// no-history SetField, the history unchanged; while paused, it holds.
fn check_clock(frames_playing: u32) {
    let mut app = mock_app();
    let mut headless = headless();
    app.set_field(playhead(1.0, false));
    let _ = headless.frame(&mut app, Vec::new());
    app.view.transport.playing = true;
    for _ in 0..frames_playing {
        let _ = headless.frame(&mut app, Vec::new());
    }
    // The first playing frame reads the tick the paused frame left, so every playing frame moves the playhead.
    let s = app.snapshot().clone();
    let want = 1.0 + f64::from(frames_playing) * MOCK_DT;
    assert!(
        (s.render.playhead.t - want).abs() < 1e-9,
        "the playhead is {} after {frames_playing} playing frames, expected {want}",
        s.render.playhead.t
    );
    assert_eq!(s.history, history(1, 0), "playback entered the history");
    let entries = app
        .console()
        .iter()
        .filter(|e| e.message.ends_with("(no history)"))
        .count();
    assert_eq!(
        entries, frames_playing as usize,
        "one no-history SetField per playing frame"
    );
    app.view.transport.playing = false;
    for _ in 0..5 {
        let _ = headless.frame(&mut app, Vec::new());
    }
    assert_eq!(
        app.snapshot().render.playhead.t,
        s.render.playhead.t,
        "the playhead moved while paused"
    );
}

#[test]
fn mock_engine_app_clock_plays_and_holds() {
    check_clock(10);
    rejects("no playing frame at all", || {
        let mut app = mock_app();
        let mut headless = headless();
        let _ = headless.frame(&mut app, Vec::new());
        assert_ne!(
            app.snapshot().render.playhead.t,
            0.0,
            "the playhead never moved"
        );
    });
}

/// After an undo while playing, the clock advances from the undone playhead, not its own.
#[test]
fn mock_engine_app_clock_resyncs_after_undo() {
    let mut app = mock_app();
    let mut headless = headless();
    app.view.transport.playing = true;
    for _ in 0..3 {
        let _ = headless.frame(&mut app, Vec::new());
    }
    app.set_field(playhead(10.0, false));
    let _ = headless.frame(&mut app, Vec::new());
    app.undo();
    let _ = headless.frame(&mut app, Vec::new());
    let t = app.snapshot().render.playhead.t;
    assert!(t < 10.0, "the clock carried on from the undone edit: t {t}");
    app.redo();
    let _ = headless.frame(&mut app, Vec::new());
    assert!(app.snapshot().render.playhead.t >= 10.0);
}

/// A time source that moves only when told to.
struct Manual(f64);

impl TimeSource for Manual {
    fn tick(&mut self) {}
    fn now(&self) -> f64 {
        self.0
    }
    fn rate_hz(&self) -> f64 {
        1.0
    }
}

/// The clock writes nothing on its first tick, nothing when the source has not moved, and nothing while paused; it
/// starts from the snapshot's playhead, then from its own, until resynced.
#[test]
fn mock_engine_clock_frames() {
    let mut clock = Clock::new();
    let mut source = Manual(5.0);
    assert_eq!(
        clock.frame(&mut source, true, 0.0),
        None,
        "no tick to measure from"
    );
    assert_eq!(
        clock.frame(&mut source, true, 0.0),
        None,
        "the source did not move"
    );
    source.0 = 5.5;
    assert_eq!(
        clock.frame(&mut source, true, 2.0),
        Some(playhead(2.5, true))
    );
    source.0 = 6.0;
    assert_eq!(
        clock.frame(&mut source, true, 2.0),
        Some(playhead(3.0, true)),
        "from its own playhead"
    );
    clock.resync();
    source.0 = 6.25;
    assert_eq!(
        clock.frame(&mut source, true, 8.0),
        Some(playhead(8.25, true)),
        "from the snapshot's"
    );
    source.0 = 7.0;
    assert_eq!(clock.frame(&mut source, false, 8.0), None, "paused");
    source.0 = 7.5;
    assert_eq!(
        clock.frame(&mut source, true, 1.0),
        Some(playhead(1.5, true)),
        "a pause resyncs"
    );
}

/// The mock side has the tick; the real side has none, so its clock holds.
#[test]
fn mock_engine_only_the_mock_has_a_time_source() {
    let mut app = mock_app();
    assert!(app.side().time_source().is_some());
    let mut real = super::support::real_app();
    assert!(real.side().time_source().is_none());
    real.view.transport.playing = true;
    let mut headless = headless();
    for _ in 0..3 {
        let _ = headless.frame(&mut real, Vec::new());
    }
    assert_eq!(real.snapshot().render.playhead.t, 0.0);
}

/// The repaint delay egui was asked for in `output`.
fn repaint_delay(output: &eframe::egui::FullOutput) -> std::time::Duration {
    output.viewport_output[&eframe::egui::ViewportId::ROOT].repaint_delay
}

/// The app reads a snapshot at most every [`SNAPSHOT_INTERVAL_S`] (~10 Hz, never per frame), one interval after the
/// last at the earliest, and asks for the frame that reads the next one; while playing it asks for a frame at the
/// tick's rate.
#[test]
fn mock_engine_snapshots_throttled_to_the_interval() {
    use crate::app::SNAPSHOT_INTERVAL_S;
    use std::time::Duration;
    let mut app = mock_app();
    let mut headless = headless();
    headless.set_frame_step(SNAPSHOT_INTERVAL_S);
    let _ = headless.frame(&mut app, Vec::new());
    headless.set_frame_step(SNAPSHOT_INTERVAL_S / 2.0);
    app.side().engine().raise_warning();
    let _ = headless.frame(&mut app, Vec::new());
    assert_eq!(
        app.counts().warnings,
        1,
        "no snapshot read exactly one interval after the last"
    );
    app.side().engine().raise_warning();
    let early = headless.frame(&mut app, Vec::new());
    assert_eq!(
        app.counts().warnings,
        1,
        "read a snapshot half an interval after the last"
    );
    assert!(
        repaint_delay(&early) <= Duration::from_secs_f64(SNAPSHOT_INTERVAL_S),
        "no frame asked for"
    );
    app.view.transport.playing = true;
    let playing = headless.frame(&mut app, Vec::new());
    assert!(repaint_delay(&playing) <= Duration::from_secs_f64(1.0 / MOCK_TICK_HZ));
}
