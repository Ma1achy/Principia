//! QA tests for TASK-M6-24 on the gui side, written from the requirements, not from the implementation:
//!
//! - REQ-GUI-165: the mock engine "serves plausible GUI-sized Snapshots, applies each SetField with undo and redo as
//!   R-69 gives them (an edit marked "no history" never entering the history), emits events through the contract's own
//!   channels, and runs a fake clock so Time plays"; verify: "the fake clock advances the playhead while playing and
//!   holds it while paused" (RQ-246: frozen at a fixed `t` in capture mode).
//! - REQ-GUI-166: the one suite runs the same case list against the mock, none skipped, and it passes.
//! - REQ-GUI-167: "While the app runs on the mock engine, the footer must carry a small "mock engine" tag, and on the
//!   real engine it must not."
//! - REQ-GUI-168: F3 hides and shows the egui layer over the figure; the top bar; the footer opening the console; the
//!   Explore page's regions, "with nothing drawn over the figure"; the figure identical underneath with F3 on and off;
//!   a raised warning and error change the footer's counts and nothing over the figure; the status line in the
//!   artboard's order, undo 2 after two edits and undo 1 / redo 1 after an undo.
//! - REQ-GUI-075: "egui's dark default theme with the Ubuntu / Ubuntu Mono fonts and no styling beyond egui's own";
//!   RQ-251: Ubuntu Mono Regular committed with its UFL 1.0 licence and a note giving its SHA-256.
//! - REQ-GUI-162: the headless capture mode renders a named screen offscreen and writes the PNG and the AccessKit names.
//! - REQ-GUI-176: fps is 1000 / the mean `frame_ms` (wall clock, telemetry §5) of the frames since the previous
//!   snapshot; the GUI draws "—" for an absent value.
//! - REQ-GUI-177: the snapshot carries the entries since the previous snapshot; the footer counts warnings and errors
//!   since the session began.
//!
//! gui takes no dependency on validation (R-187), so each test runs its own control (R-176) in the test, through
//! [`rejects`]: the same check on an input differing in the one respect the requirement turns on must panic.
// The file name `qa_TASK-M6-24` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::path::PathBuf;
use std::sync::Arc;

use eframe::egui::{self, Event, FontFamily, FontId, Key, Modifiers};
use engine::contract::conformance::{self, CASES};
use engine::contract::interface::EngineInterface;
use engine::contract::log::{Severity, Source};
use engine::contract::render_state::{Overlays, Palette, Playhead, RenderState, StainGraph};
use engine::contract::set_field::{Edit, RenderField, SetField};
use engine::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, KernelVariant, Links, Lock, Plane, Quality, SimConfig,
    Slice,
};
use engine::contract::snapshot::Snapshot;
use engine::contract::store::StateStore;
use gui::app::{window_title, App};
use gui::capture::{self, Shot, Step, PIXELS_PER_POINT, SIZE};
use gui::headless::{Headless, Name};
use gui::layout::Layout;
use gui::mock::canvas::MockCanvas;
use gui::mock::MockEngine;
use gui::side::{EngineSide, MockSide, RealSide};

const TAG: &str = "mock engine";
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// Asserts that `check` panics: the check can fail (R-176's control, run in the test).
fn rejects(what: &str, check: impl FnOnce()) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check));
    assert!(
        outcome.is_err(),
        "the check passed on {what}, which it must reject"
    );
}

fn edit(t: f64, no_history: bool) -> SetField {
    SetField {
        edit: Edit::Render(RenderField::Playhead(Playhead { t })),
        no_history,
    }
}

fn t_of(s: &Snapshot) -> f64 {
    s.render.playhead.t
}

fn depths(s: &Snapshot) -> (u32, u32) {
    (s.history.undo_depth, s.history.redo_depth)
}

fn state() -> (SimConfig, RenderState) {
    (
        SimConfig {
            chart: Chart {},
            plane: Plane {},
            slice: Slice {},
            lock: Lock {},
            links: Links {},
            integrator: Integrator {},
            kernel_variant: KernelVariant::Physics,
            horizon: Horizon {},
            collision: Collision {},
            quality: Quality {},
        },
        RenderState {
            stain_graph: StainGraph {},
            overlays: Overlays {},
            palette: Palette {},
            playhead: Playhead { t: 0.0 },
        },
    )
}

fn real_store() -> StateStore {
    let (sim, render) = state();
    StateStore::new(sim, render)
}

fn mock_app() -> App<MockSide> {
    App::new(MockSide::new(MockEngine::new(), None), FORMAT)
}

fn real_app() -> App<RealSide> {
    let (sim, render) = state();
    App::new(RealSide::new(sim, render), FORMAT)
}

fn headless() -> Headless {
    Headless::new(SIZE, PIXELS_PER_POINT)
}

fn names<S: EngineSide>(h: &mut Headless, app: &mut App<S>, events: Vec<Event>) -> Vec<Name> {
    let out = h.frame(app, events);
    h.names(&out)
}

fn texts(names: &[Name]) -> Vec<String> {
    names.iter().map(|n| n.name.clone()).collect()
}

fn key(k: Key, pressed: bool) -> Event {
    Event::Key {
        key: k,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}

/// A primary click at `pos` (points), over three frames.
fn click<S: EngineSide>(h: &mut Headless, app: &mut App<S>, pos: egui::Pos2) {
    let button = |pressed| Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    let _ = h.frame(app, vec![Event::PointerMoved(pos)]);
    let _ = h.frame(app, vec![button(true)]);
    let _ = h.frame(app, vec![button(false)]);
}

fn layout() -> Layout {
    Layout::new(headless().screen(), PIXELS_PER_POINT)
}

/// A rect in points as pixels `[x0, y0, x1, y1]`.
fn px(r: egui::Rect) -> [f64; 4] {
    let p = f64::from(PIXELS_PER_POINT);
    [r.min.x, r.min.y, r.max.x, r.max.y].map(|v| f64::from(v) * p)
}

// --- REQ-GUI-165: the mock's state semantics --------------------------------------------------------------------------

/// A SetField shows in the next snapshot; undo and redo restore and reapply it, through two levels and past the ends;
/// a new undoable edit clears redo; an edit marked "no history" changes the field and leaves both depths.
fn check_semantics(engine: &mut dyn EngineInterface) {
    let t0 = t_of(&engine.snapshot());
    engine.set_field(edit(1.0, false));
    assert_eq!(
        t_of(&engine.snapshot()),
        1.0,
        "a SetField did not show in the next snapshot"
    );
    engine.set_field(edit(2.0, false));
    engine.undo();
    let s = engine.snapshot();
    assert_eq!((t_of(&s), depths(&s)), (1.0, (1, 1)), "after one undo");
    engine.undo();
    engine.undo();
    let s = engine.snapshot();
    assert_eq!(
        (t_of(&s), depths(&s)),
        (t0, (0, 2)),
        "after undoing past the start"
    );
    engine.redo();
    let s = engine.snapshot();
    assert_eq!((t_of(&s), depths(&s)), (1.0, (1, 1)), "after a redo");
    engine.set_field(edit(7.0, true));
    let s = engine.snapshot();
    assert_eq!(
        (t_of(&s), depths(&s)),
        (7.0, (1, 1)),
        "a no-history edit changed the history"
    );
    engine.redo();
    assert_eq!(
        t_of(&engine.snapshot()),
        2.0,
        "redo past a no-history edit did not reapply the undone edit"
    );
    engine.undo();
    engine.set_field(edit(3.0, false));
    let s = engine.snapshot();
    assert_eq!(
        (t_of(&s), depths(&s)),
        (3.0, (2, 0)),
        "a new undoable edit did not clear redo"
    );
}

/// The mock, but every edit enters the history: R-69's "no history" broken.
struct HistoryForAll(MockEngine);

impl EngineInterface for HistoryForAll {
    fn set_field(&mut self, mut e: SetField) {
        e.no_history = false;
        self.0.set_field(e);
    }
    fn snapshot(&mut self) -> Snapshot {
        self.0.snapshot()
    }
    fn undo(&mut self) {
        self.0.undo();
    }
    fn redo(&mut self) {
        self.0.redo();
    }
}

#[test]
fn qa_mock_engine_set_field_undo_redo_and_no_history() {
    check_semantics(&mut MockEngine::new());
    check_semantics(&mut MockEngine::frozen());
    rejects("a mock whose history takes no-history edits", || {
        check_semantics(&mut HistoryForAll(MockEngine::new()))
    });
}

// --- REQ-GUI-166: the one suite on the mock --------------------------------------------------------------------------

#[test]
fn qa_mock_engine_conformance_same_case_list_as_the_real_engine() {
    let check = |mock: Result<Vec<&'static str>, String>| {
        let mock = mock.unwrap_or_else(|e| panic!("the mock fails the suite: {e}"));
        let real = conformance::run(real_store).expect("the real engine passes the suite");
        let listed: Vec<&str> = CASES.iter().map(|c| c.name).collect();
        assert_eq!(
            mock, listed,
            "the mock's run skipped a case of the one list"
        );
        assert_eq!(
            mock, real,
            "the mock and the real engine ran different case lists"
        );
    };
    check(conformance::run(MockEngine::new));
    check(conformance::run(MockEngine::frozen));
    rejects("a mock whose history takes no-history edits", || {
        check(conformance::run(|| HistoryForAll(MockEngine::new())))
    });
}

// --- REQ-GUI-165, REQ-GUI-176: plausible snapshots that follow the frame summary's definition -------------------------

/// Every frame-summary value is present and positive, and, the mock being steady (each snapshot carries the same
/// latest frame), fps is 1000 / its `frame_ms` (REQ-GUI-176; telemetry §5: `frame_ms` is wall clock), to within the
/// status line's half-unit of fps (it shows whole fps).
fn check_plausible(s: &Snapshot) {
    let f = &s.frame;
    let (ms, fps) = (
        f.frame_ms.expect("the mock serves frame_ms"),
        f.fps.expect("the mock serves fps"),
    );
    let quads = f.quad_count.expect("the mock serves quad_count");
    let mem = f.live_memory.expect("the mock serves live_memory");
    assert!(
        ms > 0.0 && fps > 0.0 && quads > 0,
        "an implausible frame summary: {f:?}"
    );
    assert!(
        mem.heap_bytes > 0 && mem.gpu_bytes > 0,
        "an implausible memory readout: {mem:?}"
    );
    assert!(
        (fps - 1000.0 / ms).abs() <= 0.5,
        "the mock's fps {fps} is not 1000 / its frame_ms {ms} = {:.1}, as REQ-GUI-176 defines it for a steady stream \
         of frames",
        1000.0 / ms
    );
}

#[test]
fn qa_mock_engine_snapshot_follows_the_frame_summary_definition() {
    let mut mock = MockEngine::new();
    check_plausible(&mock.snapshot());
    mock.set_field(edit(1.0, false));
    check_plausible(&mock.snapshot());
    rejects(
        "a frame summary at 60 fps with 4.1 ms wall-clock frames",
        || {
            let mut s = MockEngine::new().snapshot();
            s.frame.fps = Some(60.0);
            s.frame.frame_ms = Some(4.1);
            check_plausible(&s)
        },
    );
}

// --- REQ-GUI-165, REQ-GUI-177: events only in the snapshot, once each ------------------------------------------------

/// Each applied SetField, with history or not, logs one `info` entry from `contract`; a raised warning and error are
/// `warn` and `error` entries; all arrive in the next snapshot, oldest first, and in no later one.
fn check_events(engine: &mut MockEngine) {
    let _ = engine.snapshot();
    engine.set_field(edit(1.5, false));
    engine.raise_warning();
    engine.set_field(edit(2.5, true));
    engine.raise_error();
    let log = engine.snapshot().log;
    let got: Vec<(Severity, bool)> = log
        .iter()
        .map(|e| (e.severity, e.source == Source::Contract))
        .collect();
    assert_eq!(
        got,
        vec![
            (Severity::Info, true),
            (Severity::Warn, false),
            (Severity::Info, true),
            (Severity::Error, false)
        ],
        "the snapshot's entries, oldest first: {log:?}"
    );
    assert_eq!(
        log[0].message, "SetField Playhead.t 0 → 1.5",
        "the first SetField's entry"
    );
    assert_eq!(
        log[2].message, "SetField Playhead.t 1.5 → 2.5 (no history)",
        "the no-history SetField's entry"
    );
    assert!(
        engine.snapshot().log.is_empty(),
        "an entry rode in a second snapshot"
    );
}

#[test]
fn qa_mock_engine_events_once_in_the_snapshot() {
    check_events(&mut MockEngine::new());
    rejects(
        "a mock starting from t = 9, whose first entry reads 9 → 1.5",
        || {
            let mut m = MockEngine::new();
            m.set_field(edit(9.0, false));
            check_events(&mut m)
        },
    );
}

// --- REQ-GUI-165, RQ-246: the fake clock, through the app's clock ----------------------------------------------------

/// The playhead the app shows after `frames` frames with the transport `playing`, and the history's depths.
fn run_clock<S: EngineSide>(
    app: &mut App<S>,
    h: &mut Headless,
    playing: bool,
    frames: usize,
) -> (f64, (u32, u32)) {
    app.view.transport.playing = playing;
    for _ in 0..frames {
        let _ = h.frame(app, Vec::new());
    }
    let s = app.snapshot();
    (t_of(s), depths(s))
}

/// Playing advances the playhead every few frames without touching the history; pausing holds it.
fn check_clock<S: EngineSide>(mut app: App<S>) {
    let mut h = headless();
    let (t0, d0) = run_clock(&mut app, &mut h, false, 3);
    let (t1, d1) = run_clock(&mut app, &mut h, true, 6);
    let (t2, d2) = run_clock(&mut app, &mut h, true, 6);
    assert!(
        t1 > t0 && t2 > t1,
        "the playhead did not advance while playing: {t0} → {t1} → {t2}"
    );
    assert_eq!(
        (d1, d2),
        (d0, d0),
        "the clock's advance entered the history"
    );
    let (t3, _) = run_clock(&mut app, &mut h, false, 2);
    let (t4, _) = run_clock(&mut app, &mut h, false, 6);
    assert_eq!(t4, t3, "the playhead moved while paused");
}

#[test]
fn qa_mock_engine_clock_plays_and_holds() {
    check_clock(mock_app());
    // Capture mode's clock is frozen (RQ-246), so a playing transport cannot move it.
    rejects("a frozen mock", || {
        check_clock(App::new(MockSide::new(MockEngine::frozen(), None), FORMAT))
    });
}

// --- REQ-GUI-167: the tag exactly on the mock ------------------------------------------------------------------------

/// The tag is among the frame's names exactly when `is_mock`, inside the footer's band; the window title says so too
/// (RQ-248).
fn check_tag(names: &[Name], is_mock: bool, title: &str) {
    let tags: Vec<&Name> = names.iter().filter(|n| n.name.contains(TAG)).collect();
    assert_eq!(
        !tags.is_empty(),
        is_mock,
        "the tag's presence on {}: {:?}",
        if is_mock {
            "the mock"
        } else {
            "the real engine"
        },
        texts(names)
    );
    assert_eq!(title.contains(TAG), is_mock, "the window title `{title}`");
    let footer = px(layout().footer);
    for tag in tags {
        let r = tag.rect.expect("the tag has a rect");
        let (cx, cy) = ((r[0] + r[2]) / 2.0, (r[1] + r[3]) / 2.0);
        assert!(
            (footer[0]..footer[2]).contains(&cx) && (footer[1]..footer[3]).contains(&cy),
            "the tag at {r:?} is not in the footer {footer:?}"
        );
    }
}

#[test]
fn qa_mock_tag_exactly_on_the_mock() {
    let mock = names(&mut headless(), &mut mock_app(), Vec::new());
    let real = names(&mut headless(), &mut real_app(), Vec::new());
    check_tag(&mock, true, window_title(true));
    check_tag(&real, false, window_title(false));
    rejects("the real engine's frame taken for the mock", || {
        check_tag(&real, true, window_title(true))
    });
    rejects("the mock's frame taken for the real engine", || {
        check_tag(&mock, false, window_title(false))
    });
}

// --- REQ-GUI-168: F3, the footer, the console, nothing over the figure -----------------------------------------------

/// With the layer shown, the top bar, the footer and the regions are named; after F3 nothing of the layer is; after
/// F3 again it is back.
fn check_f3(toggle: Key) {
    let (mut h, mut app) = (headless(), mock_app());
    let shown = texts(&names(&mut h, &mut app, Vec::new()));
    for want in [
        "File",
        "View",
        "Windows",
        "Help",
        TAG,
        "Manifold view",
        "Trajectory",
        "Time",
        "Legend",
    ] {
        assert!(
            shown.iter().any(|n| n.contains(want)),
            "`{want}` not shown with the layer on: {shown:?}"
        );
    }
    let _ = h.frame(&mut app, vec![key(toggle, true)]);
    let hidden = texts(&names(&mut h, &mut app, vec![key(toggle, false)]));
    assert!(
        hidden.is_empty(),
        "the egui layer still names {hidden:?} after F3"
    );
    let _ = h.frame(&mut app, vec![key(toggle, true)]);
    let back = texts(&names(&mut h, &mut app, vec![key(toggle, false)]));
    assert!(
        back.iter().any(|n| n.contains(TAG)),
        "F3 again did not bring the layer back: {back:?}"
    );
}

#[test]
fn qa_f3_hides_and_shows_the_layer() {
    check_f3(Key::F3);
    rejects("F4 in place of F3", || check_f3(Key::F4));
}

/// Whether two pixel rects overlap with positive area.
fn overlaps(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3]
}

/// No named node of the layer overlaps the figure's rect.
fn check_nothing_over(names: &[Name], figure: [f64; 4]) {
    for n in names {
        if let Some(r) = n.rect {
            assert!(
                !overlaps(r, figure),
                "`{}` at {r:?} is drawn over the figure {figure:?}",
                n.name
            );
        }
    }
}

#[test]
fn qa_footer_warning_error_and_console_nothing_over_the_figure() {
    let (mut h, mut app) = (headless(), mock_app());
    let figure = px(layout().figure);
    check_nothing_over(&names(&mut h, &mut app, Vec::new()), figure);
    app.side().engine().raise_warning();
    let _ = h.frame(&mut app, Vec::new());
    app.side().engine().raise_warning();
    app.side().engine().raise_error();
    app.set_field(edit(4.0, false));
    let named = names(&mut h, &mut app, Vec::new());
    check_nothing_over(&named, figure);
    // REQ-GUI-177: the footer counts the warnings and errors since the session began, across snapshots; info is not
    // counted.
    let counts = app.counts();
    assert_eq!(
        (counts.warnings, counts.errors),
        (2, 1),
        "the footer's counts since the session began"
    );
    // A footer click opens the console, which lists the entries: severity, source and message.
    let footer = layout().footer;
    click(&mut h, &mut app, footer.center());
    // egui lays a new window out in an invisible sizing pass first, at a default place; it settles on its rect after.
    for _ in 0..4 {
        let _ = h.frame(&mut app, Vec::new());
    }
    let named = names(&mut h, &mut app, Vec::new());
    let shown = texts(&named);
    assert!(
        app.console_open,
        "a footer click did not open the console: {shown:?}"
    );
    for want in [
        "Console",
        "warn",
        "error",
        "info",
        "contract",
        gui::mock::MOCK_WARNING,
        gui::mock::MOCK_ERROR,
    ] {
        assert!(
            shown.iter().any(|n| n == want),
            "the open console does not list `{want}`: {shown:?}"
        );
    }
    check_nothing_over(&named, figure);
    rejects("a name laid over the figure", || {
        check_nothing_over(
            &[Name {
                name: "over".into(),
                rect: Some([
                    figure[0] + 1.0,
                    figure[1] + 1.0,
                    figure[0] + 9.0,
                    figure[1] + 9.0,
                ]),
            }],
            figure,
        )
    });
}

// --- REQ-GUI-168, REQ-GUI-176: the status line and the footer's readout ----------------------------------------------

/// The one name holding the status line.
fn status(names: &[String]) -> String {
    let lines: Vec<&String> = names
        .iter()
        .filter(|n| n.contains("quads") && n.contains("undo"))
        .collect();
    assert_eq!(lines.len(), 1, "not one status line among {names:?}");
    lines[0].clone()
}

/// The status line holds, in the artboard's order ("t 12.40 · 60 fps · 4.1 ms · 1 842 quads · undo 2 / redo 0 · F3
/// hide"), each of `parts`.
fn check_status(line: &str, parts: &[&str]) {
    let mut from = 0;
    for part in parts {
        let at = line[from..]
            .find(part)
            .unwrap_or_else(|| panic!("the status line `{line}` lacks `{part}` after byte {from}"));
        from += at + part.len();
    }
}

fn number_parts(s: &Snapshot) -> (String, String, String) {
    let f = &s.frame;
    let q = f.quad_count.expect("quads").to_string();
    // Thousands grouped by spaces, as the artboard's "1 842 quads".
    let mut grouped = String::new();
    for (i, c) in q.chars().enumerate() {
        if i > 0 && (q.len() - i).is_multiple_of(3) {
            grouped.push(' ');
        }
        grouped.push(c);
    }
    (
        format!("{:.0} fps", f.fps.expect("fps")),
        format!("{:.1} ms", f.frame_ms.expect("ms")),
        format!("{grouped} quads"),
    )
}

#[test]
fn qa_status_line_on_the_mock() {
    let (mut h, mut app) = (headless(), mock_app());
    app.set_field(edit(5.0, false));
    app.set_field(edit(12.4, false));
    let shown = texts(&names(&mut h, &mut app, Vec::new()));
    let line = status(&shown);
    let (fps, ms, quads) = number_parts(app.snapshot());
    check_status(
        &line,
        &["t 12.40", &fps, &ms, &quads, "undo 2 / redo 0", "F3 hide"],
    );
    app.undo();
    let line = status(&texts(&names(&mut h, &mut app, Vec::new())));
    check_status(
        &line,
        &["t 5.00", &fps, &ms, &quads, "undo 1 / redo 1", "F3 hide"],
    );
    // The footer: the memory readout (GPU, heap) and the "? keys" hint.
    assert!(
        shown
            .iter()
            .any(|n| n.contains("GPU") && n.contains("heap")),
        "no memory readout: {shown:?}"
    );
    assert!(
        shown.iter().any(|n| n == "? keys"),
        "no `? keys` hint: {shown:?}"
    );
    rejects("the depths before the undo", || {
        check_status(&line, &["undo 2 / redo 0"])
    });
    rejects("the artboard's order reversed", || {
        check_status(&line, &["F3 hide", "t 5.00"])
    });
}

#[test]
fn qa_status_line_absent_values_read_a_dash() {
    // The real engine's frame summary is absent until TASK-M8-05 (RQ-243): "—" for each absent value.
    let (mut h, mut app) = (headless(), real_app());
    let shown = texts(&names(&mut h, &mut app, Vec::new()));
    let line = status(&shown);
    check_status(
        &line,
        &[
            "t 0.00",
            "— fps",
            "— ms",
            "— quads",
            "undo 0 / redo 0",
            "F3 hide",
        ],
    );
    let memory: Vec<&String> = shown
        .iter()
        .filter(|n| n.contains("GPU") && n.contains("heap"))
        .collect();
    assert_eq!(memory.len(), 1, "not one memory readout: {shown:?}");
    assert_eq!(
        memory[0].matches('—').count(),
        2,
        "the absent memory readout `{}` lacks a dash each",
        memory[0]
    );
    rejects("the mock's line, whose values are present", || {
        let (mut h, mut app) = (headless(), mock_app());
        check_status(
            &status(&texts(&names(&mut h, &mut app, Vec::new()))),
            &["— fps"],
        )
    });
}

// --- REQ-GUI-075, RQ-251: the dark theme and the fonts ---------------------------------------------------------------

fn font_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/fonts")
}

/// A context with `fonts`, after one frame, so its fonts are loaded.
fn ctx_with(install: impl FnOnce(&egui::Context)) -> egui::Context {
    let ctx = egui::Context::default();
    install(&ctx);
    let mut out = ctx.run_ui(egui::RawInput::default(), |_| {});
    // epaint asserts, in debug builds, that a dropped delta was handled.
    out.textures_delta.clear();
    ctx
}

fn width(ctx: &egui::Context, family: FontFamily, text: &str) -> f32 {
    ctx.fonts_mut(|f| {
        text.chars()
            .map(|c| f.glyph_width(&FontId::new(20.0, family.clone()), c))
            .sum()
    })
}

#[test]
fn qa_theme_dark_default_ubuntu_and_ubuntu_mono() {
    let installed = ctx_with(gui::theme::install);
    // egui's dark theme, with no styling beyond egui's own.
    let style = installed.global_style();
    assert!(
        style.visuals == egui::Visuals::dark(),
        "the visuals are not egui's dark default"
    );
    assert!(
        style.spacing == egui::style::Spacing::default(),
        "the spacing is not egui's default"
    );
    // Ubuntu Mono, the committed file, is the monospace face: the same widths as a context given only that file.
    let ttf = std::fs::read(font_dir().join("UbuntuMono-R.ttf")).expect("the committed font");
    let reference = ctx_with(|ctx| {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "qa-ubuntu-mono".into(),
            Arc::new(egui::FontData::from_owned(ttf.clone())),
        );
        fonts
            .families
            .insert(FontFamily::Monospace, vec!["qa-ubuntu-mono".into()]);
        ctx.set_fonts(fonts);
    });
    let egui_own = ctx_with(|_| {});
    let sample = "0123456789 t 12.40 · fps";
    let check_mono = |ctx: &egui::Context| {
        assert_eq!(
            width(ctx, FontFamily::Monospace, sample),
            width(&reference, FontFamily::Monospace, sample),
            "the monospace face is not the committed Ubuntu Mono"
        )
    };
    check_mono(&installed);
    // Text keeps egui's own Ubuntu Light.
    let words = "Manifold view Trajectory";
    assert_eq!(
        width(&installed, FontFamily::Proportional, words),
        width(&egui_own, FontFamily::Proportional, words),
        "the proportional face is not egui's own"
    );
    rejects("egui's own monospace face, Hack", || check_mono(&egui_own));
}

/// FIPS 180-4 SHA-256, for the font note's digest.
fn sha256(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut msg = data.to_vec();
    let bits = (data.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bits.to_be_bytes());
    for block in msg.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes(block[i * 4..i * 4 + 4].try_into().unwrap());
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut v = h;
        for i in 0..64 {
            let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
            let ch = (v[4] & v[5]) ^ (!v[4] & v[6]);
            let t1 = v[7]
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
            let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
            let t2 = s0.wrapping_add(maj);
            v = [
                t1.wrapping_add(t2),
                v[0],
                v[1],
                v[2],
                v[3].wrapping_add(t1),
                v[4],
                v[5],
                v[6],
            ];
        }
        for (a, b) in h.iter_mut().zip(v) {
            *a = a.wrapping_add(b);
        }
    }
    h.iter().map(|x| format!("{x:08x}")).collect()
}

/// The note beside the font gives the committed file's SHA-256 and an upstream URL; the licence is UFL 1.0.
fn check_font_note(ttf: &[u8], note: &str, licence: &str) {
    let digest = sha256(ttf);
    assert!(
        note.contains(&digest),
        "the font note does not give the committed file's SHA-256 {digest}"
    );
    assert!(
        note.contains("https://"),
        "the font note gives no source URL"
    );
    assert!(
        licence.contains("UBUNTU FONT LICENCE Version 1.0"),
        "the licence is not the UFL 1.0"
    );
}

#[test]
fn qa_theme_font_committed_with_licence_and_digest() {
    // The SHA-256 itself, against FIPS 180-4's "abc" example.
    assert_eq!(
        sha256(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    let ttf = std::fs::read(font_dir().join("UbuntuMono-R.ttf")).expect("the font");
    let note = std::fs::read_to_string(font_dir().join("README.md")).expect("the note");
    let licence = std::fs::read_to_string(font_dir().join("UFL.txt")).expect("the licence");
    check_font_note(&ttf, &note, &licence);
    let mut changed = ttf.clone();
    changed[1000] ^= 1;
    rejects("a font file one bit off the noted digest", || {
        check_font_note(&changed, &note, &licence)
    });
}

// --- REQ-GUI-162, REQ-GUI-168: the capture mode, on the GPU ----------------------------------------------------------

fn canvas() -> Arc<MockCanvas> {
    Arc::new(MockCanvas::new().expect("a GPU adapter for the mock's canvas"))
}

fn shot_px(shot: &Shot, x: usize, y: usize) -> [u8; 4] {
    let i = (y * shot.size[0] as usize + x) * 4;
    shot.rgba[i..i + 4].try_into().unwrap()
}

/// A pixel rect, in whole pixels, `[x0, y0, x1, y1)`.
fn whole(r: egui::Rect) -> [usize; 4] {
    px(r).map(|v| v.round() as usize)
}

fn inside(r: [usize; 4], x: usize, y: usize) -> bool {
    (r[0]..r[2]).contains(&x) && (r[1]..r[3]).contains(&y)
}

/// `a` and `b` are pixel-identical over `r`.
fn check_identical_over(a: &Shot, b: &Shot, r: [usize; 4], what: &str) {
    for y in r[1]..r[3] {
        for x in r[0]..r[2] {
            assert_eq!(
                shot_px(a, x, y),
                shot_px(b, x, y),
                "{what}: the figure differs at ({x}, {y})"
            );
        }
    }
}

/// `a` and `b` differ somewhere in `r`.
fn check_differs_over(a: &Shot, b: &Shot, r: [usize; 4], what: &str) {
    let differs = (r[1]..r[3]).any(|y| (r[0]..r[2]).any(|x| shot_px(a, x, y) != shot_px(b, x, y)));
    assert!(differs, "{what}: nothing changed in {r:?}");
}

#[test]
fn qa_capture_figure_identical_under_f3_warning_and_console() {
    let canvas = canvas();
    let base = capture::shoot(canvas.clone(), &[]);
    let f3 = capture::shoot(canvas.clone(), &[Step::F3]);
    let raised = capture::shoot(
        canvas.clone(),
        &[Step::RaiseWarning, Step::RaiseError, Step::ClickFooter],
    );
    assert_eq!(
        base.size,
        [2160, 1350],
        "the capture is not the artboard's 2160 × 1350"
    );
    let l = layout();
    let figure = whole(l.figure);
    check_identical_over(&base, &f3, figure, "F3 off");
    check_identical_over(
        &base,
        &raised,
        figure,
        "a warning, an error and the console",
    );
    // The figure is the stand-in, not a flat fill: it has more than one colour.
    let first = shot_px(&base, figure[0], figure[1]);
    let varied = (figure[1]..figure[3])
        .any(|y| (figure[0]..figure[2]).any(|x| shot_px(&base, x, y) != first));
    assert!(
        varied,
        "the figure is one flat colour, not the noise stand-in"
    );
    // With F3 off, everything but the figure is one colour, the clear colour.
    let (w, h) = (SIZE[0] as usize, SIZE[1] as usize);
    let clear = shot_px(&f3, 0, 0);
    for y in 0..h {
        for x in 0..w {
            if !inside(figure, x, y) {
                assert_eq!(
                    shot_px(&f3, x, y),
                    clear,
                    "drawn outside the figure with F3 off at ({x}, {y})"
                );
            }
        }
    }
    // The raised warning and error, and the console, change the footer and the bottom row.
    check_differs_over(
        &base,
        &raised,
        whole(l.footer),
        "the footer after a warning and an error",
    );
    check_differs_over(
        &base,
        &raised,
        whole(l.bottom_row),
        "the bottom row with the console open",
    );
    rejects("the footer compared with itself", || {
        check_differs_over(&base, &base, whole(l.footer), "the footer")
    });
    rejects("the shown layer against F3 off over the footer", || {
        check_identical_over(&base, &f3, whole(l.footer), "the footer")
    });
}

/// The PNG `path` holds is `size`: its IHDR, read by its offsets (PNG spec §11.2.2).
fn png_size(bytes: &[u8]) -> [u32; 2] {
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n", "not a PNG");
    assert_eq!(&bytes[12..16], b"IHDR", "no IHDR first");
    [
        u32::from_be_bytes(bytes[16..20].try_into().unwrap()),
        u32::from_be_bytes(bytes[20..24].try_into().unwrap()),
    ]
}

/// The capture command writes the PNG at the artboard's size and the names file, with the tag named and its rect.
fn check_capture_files(out: &std::path::Path) {
    let png = std::fs::read(out.join("capture.png")).expect("capture.png written");
    assert_eq!(png_size(&png), [2160, 1350], "the PNG's size");
    let names: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(out.join("names.json")).expect("names.json written"),
    )
    .expect("names.json is JSON");
    let list = names["names"].as_array().expect("a names list");
    let tag = list
        .iter()
        .find(|n| n["name"] == TAG)
        .unwrap_or_else(|| panic!("no tag among {list:?}"));
    assert_eq!(
        tag["rect"].as_array().map(Vec::len),
        Some(4),
        "the tag's rect: {tag}"
    );
}

fn run_capture(steps: &str, out: &std::path::Path) -> Result<(), String> {
    let _ = std::fs::remove_dir_all(out);
    let args: Vec<String> = ["capture", "--screen", "01_main", "--steps", steps, "--out"]
        .into_iter()
        .map(String::from)
        .chain([out.display().to_string()])
        .collect();
    gui::cli::mock::dispatch(&args, |_, _, _| panic!("the capture opened a window"))
}

#[test]
fn qa_capture_command_writes_png_and_names() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("qa_m6_24_capture");
    let out = root.join("shell");
    run_capture("", &out).expect("the capture command ran");
    check_capture_files(&out);
    // A step outside the closed list, and a screen with no artboard, are refused.
    assert!(
        run_capture("f3,bogus", &root.join("bogus")).is_err(),
        "an unknown step was accepted"
    );
    let refused = gui::cli::mock::dispatch(
        &["capture", "--screen", "99_nowhere", "--steps", "", "--out"]
            .into_iter()
            .map(String::from)
            .chain([root.join("nowhere").display().to_string()])
            .collect::<Vec<_>>(),
        |_, _, _| panic!("the capture opened a window"),
    );
    assert!(refused.is_err(), "an unknown screen was accepted");
    rejects("an output directory the capture never wrote", || {
        check_capture_files(&root.join("never"))
    });
}

/// `gui` with no command opens the window on the mock: the title carries the tag (RQ-248), and egui-wgpu is built from
/// the device and queue the mock's canvas provides (RQ-247), not a context of eframe's own.
fn check_window(title: &str, existing: bool) {
    assert!(
        title.contains(TAG),
        "the window's title `{title}` does not say it runs on the mock"
    );
    assert!(
        existing,
        "egui-wgpu is not built from the engine side's device"
    );
}

#[test]
fn qa_window_opens_on_the_mock() {
    let mut seen = None;
    gui::cli::mock::dispatch(&[], |title, options, _creator| {
        let existing = matches!(
            options.wgpu_options.wgpu_setup,
            eframe::egui_wgpu::WgpuSetup::Existing(_)
        );
        seen = Some((title.to_owned(), existing));
        Ok(())
    })
    .expect("the window command ran");
    let (title, existing) = seen.expect("the window command opened no window");
    check_window(&title, existing);
    rejects("the real engine's title", || {
        check_window(window_title(false), existing)
    });
}
