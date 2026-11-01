//! The measurement panel (TASK-M8-86; R-627, render_gui_spec G1): beside the figure, never over it; opening it sends no
//! `SetField` and adds no undo entry; a run's script; the flags; and the report's fields, each in both builds' one
//! implementation.

use std::sync::Arc;
use std::time::Duration;

use eframe::egui::{self, pos2, Key, Modifiers, Rect, ViewportId};
use eframe::egui_wgpu;
use engine::contract::interface::EngineInterface;

use super::support::{mock_app, rejects};
use crate::app::App;
use crate::cli::mock::{MEASURED_WINDOW_SIZE, WINDOW_SIZE};
use crate::explore::breadcrumb::{self, RING_WIDTH};
use crate::explore::footer::FOOTER;
use crate::headless::Headless;
use crate::measure::stats::{Frame, WARM_UP_MS, WINDOW_MS};
use crate::measure::{
    build_name, frame_delay, pan_edit, pan_offset, split, Environment, Measured, Phase, Run,
    Session, PANEL_SCOPE, PANEL_WIDTH, RUN_IDLE, RUN_PAN,
};
use crate::mock::canvas::MockCanvas;
use crate::mock::MockEngine;
use crate::side::{EngineSide, MockSide};
use engine::contract::view_ui::{KeyMode, Mode};

fn session(run: Option<Run>) -> Session {
    Session::new(Environment {
        layout: "native".to_owned(),
        refresh_hz: Some(60.0),
        run,
        ..Environment::default()
    })
}

/// Runs a frame of `measured` at the window `window`.
fn frame(ctx: &egui::Context, measured: &mut Measured<MockSide>, window: Rect, time: f64) {
    let mut raw = egui::RawInput {
        screen_rect: Some(window),
        time: Some(time),
        ..Default::default()
    };
    raw.viewports
        .entry(ViewportId::ROOT)
        .or_default()
        .native_pixels_per_point = Some(1.0);
    measured.raw_input_hook(ctx, &mut raw);
    ctx.run_ui(raw, |ui| measured.ui(ui)).textures_delta.clear();
}

/// The panel's strip touches the app's part and lies inside the window, and never overlaps it.
fn check_split(split: impl Fn(Rect) -> (Rect, Rect)) {
    let window = Rect::from_min_max(pos2(0.0, 0.0), pos2(1000.0, 500.0));
    let (app, panel) = split(window);
    assert_eq!(panel.width(), PANEL_WIDTH, "the panel's width");
    assert_eq!(app.max.x, panel.min.x, "side by side");
    assert_eq!(app.min, window.min, "the app keeps the window's corner");
    assert_eq!(
        panel.max, window.max,
        "the panel ends at the window's corner"
    );
    assert_eq!(app.height(), window.height(), "the app's full height");
    // A window narrower than the panel leaves the app nothing, and the panel the window.
    let narrow = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 500.0));
    let (app, panel) = split(narrow);
    assert_eq!(app.width(), 0.0, "no room for the app");
    assert_eq!(panel, narrow, "the panel takes the window");
    // A window exactly the panel's width.
    let exact = Rect::from_min_max(pos2(0.0, 0.0), pos2(PANEL_WIDTH, 500.0));
    assert_eq!(split(exact).0.width(), 0.0, "exactly the panel's width");
}

#[test]
fn measure_panel_is_beside_the_figure() {
    check_split(split);
    rejects("a panel over the app's right edge", || {
        check_split(|w| {
            let (_, panel) = split(w);
            (w, panel)
        })
    });
}

/// The figure's rect the app draws lies in the app's part: laid out on a child, the layout reads its rect.
#[test]
fn measure_panel_opens_with_no_edit_and_no_undo_entry() {
    let ctx = egui::Context::default();
    let window = Rect::from_min_max(pos2(0.0, 0.0), pos2(1440.0, 900.0));
    let mut measured = Measured::new(mock_app(), Some(session(None)), Box::new(|_| {}));
    let before = measured.app.side().engine().snapshot();
    for i in 0..3 {
        frame(&ctx, &mut measured, window, f64::from(i));
    }
    let after = measured.app.side().engine().snapshot();
    assert_eq!(
        after.history.undo_depth, before.history.undo_depth,
        "no undo entry"
    );
    assert_eq!(
        after.sim.plane.z0, before.sim.plane.z0,
        "no edit of the view"
    );
    assert!(!after.render.debug_view, "it is not View › Debug view");
}

#[test]
fn measure_session_none_is_the_app_as_it_is() {
    let ctx = egui::Context::default();
    let window = Rect::from_min_max(pos2(0.0, 0.0), pos2(1440.0, 900.0));
    let reports = std::rc::Rc::new(std::cell::Cell::new(0));
    let count = reports.clone();
    let mut measured = Measured::new(
        mock_app(),
        None,
        Box::new(move |_| count.set(count.get() + 1)),
    );
    frame(&ctx, &mut measured, window, 0.0);
    assert_eq!(reports.get(), 0, "no session, no report");
}

#[test]
fn measure_reports_as_it_opens() {
    let ctx = egui::Context::default();
    let window = Rect::from_min_max(pos2(0.0, 0.0), pos2(1440.0, 900.0));
    let reports = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let sink = reports.clone();
    let mut measured = Measured::new(
        mock_app(),
        Some(session(None)),
        Box::new(move |json| sink.borrow_mut().push(json)),
    );
    frame(&ctx, &mut measured, window, 0.0);
    frame(&ctx, &mut measured, window, 0.1);
    assert_eq!(reports.borrow().len(), 1, "once, as the panel opens");
    assert!(reports.borrow()[0].contains("\"status\":\"ready\""));
}

#[test]
fn measure_pan_moves_the_view_with_no_undo_entry() {
    let ctx = egui::Context::default();
    let window = Rect::from_min_max(pos2(0.0, 0.0), pos2(1440.0, 900.0));
    let mut measured = Measured::new(mock_app(), Some(session(Some(Run::Pan))), Box::new(|_| {}));
    let before = measured.app.side().engine().snapshot();
    frame(&ctx, &mut measured, window, 0.0);
    std::thread::sleep(std::time::Duration::from_millis(50));
    frame(&ctx, &mut measured, window, 0.1);
    let after = measured.app.side().engine().snapshot();
    assert_ne!(
        after.sim.plane.z0, before.sim.plane.z0,
        "the pan moved the view"
    );
    assert_eq!(
        after.history.undo_depth, before.history.undo_depth,
        "no undo entry"
    );
    assert!(measured.session.as_ref().unwrap().running());
}

#[test]
fn measure_pan_edit_goes_one_figure_width() {
    let app: App<MockSide> = mock_app();
    let plane = app.snapshot().sim.plane.clone();
    let edit = pan_edit(&plane, 1.0);
    assert!(edit.no_history, "a scripted pan leaves no undo entries");
    let dragged = crate::explore::manifold_view::navigate::pan(
        &plane,
        eframe::egui::vec2(100.0, 0.0),
        eframe::egui::vec2(100.0, 50.0),
    );
    assert_eq!(edit.edit, dragged.edit, "a drag of one figure width");
}

/// The pan's offset: from 0 to 1 in 5 s, back to 0 in 5 s, at a constant rate.
fn check_offset(offset: impl Fn(f64) -> f64) {
    let near = |at: f64, want: f64| {
        assert!(
            (offset(at) - want).abs() < 1e-9,
            "at {at} ms: {}",
            offset(at)
        );
    };
    near(0.0, 0.0);
    near(2_500.0, 0.5);
    near(5_000.0, 1.0);
    near(7_500.0, 0.5);
    near(10_000.0, 0.0);
    near(12_500.0, 0.5);
    near(35_000.0, 1.0);
}

#[test]
fn measure_pan_offset_is_a_constant_rate_back_and_forth() {
    check_offset(pan_offset);
    rejects("a pan one way only", || {
        check_offset(|ms| (ms / 5_000.0).fract())
    });
    rejects("a pan at twice the rate", || {
        check_offset(|ms| pan_offset(ms * 2.0))
    });
}

#[test]
fn measure_run_steps_through_warm_up_and_window() {
    let mut s = session(None);
    assert_eq!(s.step(1.0), Default::default(), "no run asked for");
    s.request(Run::Idle);
    let first = s.step(100.0);
    assert!(first.started && !first.finished && first.pan.is_none());
    assert!(s.running());
    // Asked during a run, nothing changes.
    s.request(Run::Pan);
    let during = s.step(101.0);
    assert!(!during.started, "a run under way is not restarted");
    assert!(s.step(100.0 + WARM_UP_MS + WINDOW_MS - 1.0) == Default::default());
    let end = s.step(100.0 + WARM_UP_MS + WINDOW_MS);
    assert!(end.finished, "at the window's end");
    assert_eq!(
        s.phase,
        Phase::Done {
            run: Run::Idle,
            stats: None
        }
    );
    s.request(Run::Pan);
    let again = s.step(50_000.0);
    assert!(again.started, "another run after the first");
    assert_eq!(again.pan, Some(0.0), "a pan steps its offset");
    assert_eq!(s.step(52_500.0).pan, Some(0.5));
}

#[test]
fn measure_flag_starts_its_run() {
    let mut s = session(Some(Run::Pan));
    assert!(
        s.step(0.0).started,
        "the flag's run starts at the first step"
    );
    assert!(!s.step(1.0).started, "once");
}

/// `begin_frame` on a thread of its own: its delay spins on the session's clock, so a clock that does not reach the
/// delay fails this, in [`SPIN_LIMIT`], and never hangs the test.
fn begin_frame_bounded(session: Session, limit: Duration) -> Session {
    let (done, finished) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut session = session;
        session.begin_frame();
        let _ = done.send(session);
    });
    finished
        .recv_timeout(limit)
        .expect("the frame's delay did not end in time")
}

/// Far past any delay these tests ask for, and three tries of it inside the per-mutant timeout.
const SPIN_LIMIT: Duration = Duration::from_secs(10);

#[test]
fn measure_begin_frame_spends_the_delay() {
    let mut s = Session::new(Environment {
        frame_delay_ms: 20.0,
        ..Environment::default()
    });
    let start = s.now_ms();
    s = begin_frame_bounded(s, SPIN_LIMIT);
    assert!(s.now_ms() - start >= 20.0, "the delay is spent");
    rejects("a delay past the limit, which fails, not hangs", || {
        let slow = Session::new(Environment {
            frame_delay_ms: 2_000.0,
            ..Environment::default()
        });
        begin_frame_bounded(slow, Duration::from_millis(10));
    });
    let mut s = session(None);
    let start = s.now_ms();
    s.begin_frame();
    assert!(s.now_ms() - start < 20.0, "no delay unasked");
}

/// The fields every report and the panel carry, in every phase.
const REPORT_KEYS: [&str; 21] = [
    "status",
    "run",
    "build",
    "cross_origin_isolated",
    "shared_array_buffer",
    "wasm_threads",
    "layout",
    "fallback_reason",
    "refresh_hz",
    "frame_delay_ms",
    "app_width",
    "app_height",
    "frames",
    "mean_fps",
    "mean_fps_capped",
    "p5_fps",
    "p5_fps_capped",
    "work_mean_ms",
    "work_p95_ms",
    "acquire_mean_ms",
    "work_less_wait_p95_ms",
];
const ROW_LABELS: [&str; 17] = [
    "status",
    "build",
    "crossOriginIsolated",
    "SharedArrayBuffer",
    "wasm threads",
    "layout",
    "fallback reason",
    "refresh rate",
    "frame delay",
    "app size",
    "frames",
    "mean fps",
    "5th-percentile fps",
    "work mean",
    "work p95",
    "surface wait mean",
    "work less wait p95",
];

fn check_fields(report: &serde_json::Value, rows: &[(&str, String)]) {
    let object = report.as_object().expect("an object");
    for key in REPORT_KEYS {
        assert!(object.contains_key(key), "the report lacks {key}");
    }
    assert_eq!(object.len(), REPORT_KEYS.len(), "the report's fields");
    let labels: Vec<&str> = rows.iter().map(|r| r.0).collect();
    assert_eq!(labels, ROW_LABELS, "the panel's rows");
}

#[test]
fn measure_report_carries_every_field() {
    let mut s = session(None);
    check_fields(&s.report(), &s.rows());
    s.request(Run::Idle);
    s.step(0.0);
    check_fields(&s.report(), &s.rows());
    assert_eq!(s.report()["status"], "running");
    s.step(WARM_UP_MS + WINDOW_MS);
    check_fields(&s.report(), &s.rows());
    assert_eq!(s.report()["status"], "done");
    assert_eq!(s.report()["run"], "idle");
    rejects("a report lacking a field", || {
        let mut report = s.report();
        report.as_object_mut().unwrap().remove("p5_fps_capped");
        check_fields(&report, &s.rows());
    });
}

/// The report and the panel's row name the build these tests are in, whatever the phase.
fn check_build(report: &serde_json::Value, rows: &[(&str, String)], name: &str) {
    assert_eq!(report["build"], name, "the report's build");
    let row = rows.iter().find(|r| r.0 == "build").expect("the row");
    assert_eq!(row.1, name, "the panel's build");
}

#[test]
fn measure_report_names_the_build() {
    assert_eq!(build_name(true), "debug");
    assert_eq!(build_name(false), "release");
    let here = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let native = Session::new(
        Environment::native(Some("1"), None, None, None)
            .expect("flags")
            .expect("open"),
    );
    let browser = Session::new(Environment::browser(r#"{"layout":"worker"}"#).expect("the JSON"));
    for s in [&native, &browser] {
        check_build(&s.report(), &s.rows(), here);
    }
    let mut running = session(None);
    running.request(Run::Idle);
    running.step(0.0);
    check_build(&running.report(), &running.rows(), here);
    running.step(WARM_UP_MS + WINDOW_MS);
    check_build(&running.report(), &running.rows(), here);

    let other = build_name(!cfg!(debug_assertions));
    rejects("a report naming the other build", || {
        let mut report = native.report();
        report["build"] = other.into();
        check_build(&report, &native.rows(), here);
    });
    rejects("a row naming the other build", || {
        let rows = vec![("build", other.to_owned())];
        check_build(&native.report(), &rows, here);
    });
    rejects("a report with no build", || {
        let mut report = native.report();
        report.as_object_mut().unwrap().remove("build");
        check_build(&report, &native.rows(), here);
    });
}

#[test]
fn measure_rows_mark_a_capped_figure() {
    let mut s = session(None);
    s.request(Run::Idle);
    s.step(0.0);
    let row = |s: &Session, label: &str| {
        s.rows()
            .iter()
            .find(|r| r.0 == label)
            .expect("a row")
            .1
            .clone()
    };
    assert_eq!(row(&s, "mean fps"), "–", "no figure yet");
    assert_eq!(row(&s, "crossOriginIsolated"), "n/a", "natively");
    assert_eq!(row(&s, "wasm threads"), "n/a", "natively");
    assert_eq!(row(&s, "refresh rate"), "60.0 Hz");
    let mut at = WARM_UP_MS;
    let mut stamps = Vec::new();
    while at < WARM_UP_MS + WINDOW_MS {
        stamps.push(Frame {
            at_ms: at,
            work_ms: 2.0,
            acquire_ms: 0.5,
        });
        at += 1000.0 / 60.0;
    }
    let stats = crate::measure::stats::window_stats(&stamps, Some(60.0));
    s.phase = Phase::Done {
        run: Run::Idle,
        stats,
    };
    assert!(
        row(&s, "mean fps").ends_with(" capped"),
        "{}",
        row(&s, "mean fps")
    );
    assert_eq!(row(&s, "work mean"), "2.00 ms");
    assert_eq!(row(&s, "surface wait mean"), "0.50 ms");
    assert_eq!(row(&s, "work p95"), "2.00 ms");
    assert_eq!(row(&s, "work less wait p95"), "1.50 ms");
    assert_eq!(s.report()["work_p95_ms"], 2.0);
    assert_eq!(s.report()["work_less_wait_p95_ms"], 1.5);
    let mut low = session(None);
    let stats = crate::measure::stats::window_stats(&stamps, Some(120.0));
    low.phase = Phase::Done {
        run: Run::Idle,
        stats,
    };
    assert!(
        !row(&low, "mean fps").contains("capped"),
        "uncapped, below the rate"
    );
}

#[test]
fn measure_native_environment_reads_its_flags() {
    assert_eq!(
        Environment::native(None, None, None, None),
        Ok(None),
        "unset"
    );
    assert_eq!(
        Environment::native(Some("0"), None, None, None),
        Ok(None),
        "0"
    );
    assert_eq!(
        Environment::native(Some("true"), None, None, None),
        Ok(None),
        "only 1 opens it"
    );
    let env = Environment::native(Some("1"), Some("20"), Some("pan"), Some(120.0))
        .unwrap()
        .unwrap();
    assert_eq!(env.layout, "native");
    assert_eq!(env.frame_delay_ms, 20.0);
    assert_eq!(env.run, Some(Run::Pan));
    assert_eq!(env.refresh_hz, Some(120.0));
    assert_eq!(env.cross_origin_isolated, None, "a browser's item");
    assert!(
        Environment::native(Some("1"), Some("x"), None, None).is_err(),
        "a bad delay"
    );
    assert!(
        Environment::native(Some("1"), None, Some("x"), None).is_err(),
        "a bad run"
    );
}

#[test]
fn measure_frame_delay_refuses_what_is_no_delay() {
    assert_eq!(frame_delay(None), Ok(0.0));
    assert_eq!(frame_delay(Some("0")), Ok(0.0), "0 is the boundary");
    assert_eq!(frame_delay(Some(" 16.5 ")), Ok(16.5));
    assert!(frame_delay(Some("-0.5")).is_err(), "below 0");
    assert!(frame_delay(Some("inf")).is_err(), "not finite");
    assert!(frame_delay(Some("NaN")).is_err(), "not a number");
    assert!(frame_delay(Some("soon")).is_err(), "not a number");
}

#[test]
fn measure_browser_environment_reads_the_loader() {
    let env = Environment::browser(
        r#"{"crossOriginIsolated":true,"sharedArrayBuffer":false,"wasmThreads":"ok","layout":"main",
            "fallbackReason":"no adapter","idleLoopMs":[0,10,20],"frameDelayMs":"25","run":"idle"}"#,
    )
    .unwrap();
    assert_eq!(env.cross_origin_isolated, Some(true));
    assert_eq!(env.shared_array_buffer, Some(false));
    assert_eq!(env.wasm_threads.as_deref(), Some("ok"));
    assert_eq!(env.layout, "main");
    assert_eq!(env.fallback_reason.as_deref(), Some("no adapter"));
    assert_eq!(env.refresh_hz, Some(100.0), "the idle loop's median");
    assert_eq!(env.frame_delay_ms, 25.0);
    assert_eq!(env.run, Some(Run::Idle));
    let bare = Environment::browser(r#"{"layout":"worker"}"#).unwrap();
    assert_eq!(
        (bare.refresh_hz, bare.run, bare.frame_delay_ms),
        (None, None, 0.0)
    );
    assert!(Environment::browser("{").is_err(), "not JSON");
    assert!(Environment::browser("{}").is_err(), "no layout");
    assert!(
        Environment::browser(r#"{"layout":"main","run":"x"}"#).is_err(),
        "a bad run"
    );
    assert!(
        Environment::browser(r#"{"layout":"main","frameDelayMs":"x"}"#).is_err(),
        "a bad delay"
    );
}

#[test]
fn measure_run_names_round_trip() {
    for run in [Run::Pan, Run::Idle] {
        assert_eq!(Run::parse(run.name()), Ok(run));
    }
    assert!(Run::parse("").is_err());
}

#[test]
fn measure_record_reads_a_frame_from_its_stamps() {
    let mut s = session(None);
    s.record(1.0, 2.0, 3.0, 4.0);
    assert!(s.frames().is_empty(), "no run, nothing recorded");
    s.request(Run::Idle);
    s.step(100.0);
    s.record(110.0, 113.0, 117.0, 130.0);
    let expected = Frame {
        at_ms: 30.0,
        work_ms: 20.0,
        acquire_ms: 4.0,
    };
    assert_eq!(s.frames(), [expected]);
    rejects("a frame stamped from the run's end, not its start", || {
        let wrong = Frame {
            at_ms: 130.0,
            ..expected
        };
        assert_eq!(s.frames(), [wrong]);
    });
}

#[test]
fn measure_clock_reads_milliseconds() {
    let mut s = Session::new(Environment {
        frame_delay_ms: 25.0,
        ..Environment::default()
    });
    let wall = std::time::Instant::now();
    let start = s.now_ms();
    s = begin_frame_bounded(s, SPIN_LIMIT);
    let spent = wall.elapsed();
    assert!(spent.as_millis() >= 25, "the delay is real time");
    assert!(spent.as_millis() < 5_000, "and not a thousand times it");
    assert!(s.now_ms() - start >= 25.0, "the session's clock agrees");
    rejects(
        "a clock in seconds, whose delay ends a thousand times late",
        || {
            assert!(spent.as_millis() < 1, "no real time at all");
        },
    );
}

/// The panel's rows and its buttons are drawn, and a button starts its run only while none is under way.
#[test]
fn measure_panel_draws_its_rows_and_its_buttons_start_a_run() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let scale = Headless::new([1440, 900], 1.0);
    let window = Rect::from_min_max(pos2(0.0, 0.0), pos2(1440.0, 900.0));
    let mut measured = Measured::new(mock_app(), Some(session(None)), Box::new(|_| {}));
    let step = |measured: &mut Measured<MockSide>, events: Vec<egui::Event>| {
        let mut raw = egui::RawInput {
            screen_rect: Some(window),
            events,
            ..Default::default()
        };
        measured.raw_input_hook(&ctx, &mut raw);
        let mut out = ctx.run_ui(raw, |ui| measured.ui(ui));
        out.textures_delta.clear();
        scale.names(&out)
    };
    let names = step(&mut measured, Vec::new());
    for label in ["crossOriginIsolated", "work p95", "Run the scripted pan"] {
        assert!(names.iter().any(|n| n.name == label), "no `{label}` drawn");
    }
    let idle = names
        .iter()
        .find(|n| n.name == "Run idle")
        .expect("the button");
    let rect = idle.rect.expect("its rect");
    assert!(
        rect[0] >= 1440.0 - f64::from(PANEL_WIDTH),
        "in the panel's strip"
    );
    let at = pos2(
        ((rect[0] + rect[2]) / 2.0) as f32,
        ((rect[1] + rect[3]) / 2.0) as f32,
    );
    for events in crate::capture::click(at) {
        step(&mut measured, events);
    }
    step(&mut measured, Vec::new());
    let s = measured.session.as_ref().expect("the session");
    assert!(
        matches!(s.phase, Phase::Running { run: Run::Idle, .. }),
        "the click started the run: {:?}",
        s.phase
    );
    rejects("the other button's run", || {
        assert!(matches!(s.phase, Phase::Running { run: Run::Pan, .. }));
    });
}

/// The window's clear colour is the app's, through eframe's trait.
#[test]
fn measure_clear_colour_is_the_apps() {
    let visuals = egui::Visuals::dark();
    let measured = Measured::new(mock_app(), None, Box::new(|_| {}));
    let check = |got: [f32; 4]| {
        assert_eq!(
            got,
            crate::app::clear_colour(&visuals).to_normalized_gamma_f32()
        );
    };
    check(eframe::App::clear_color(&measured, &visuals));
    rejects("another colour", || check([0.0; 4]));
}

/// A real frame through egui-wgpu, as the drivers paint one, through eframe's trait as they call it: the two marks stamp
/// it, and the next frame's start records it. Bounded: three frames.
#[test]
fn measure_marks_stamp_a_painted_frame() {
    let canvas = Arc::new(MockCanvas::new().expect("a GPU adapter for the mock's canvas"));
    let context = engine::contract::canvas::Canvas::context(&*canvas).clone();
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut renderer = egui_wgpu::Renderer::new(
        &context.device,
        format,
        egui_wgpu::RendererOptions::default(),
    );
    let target = context.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("measure target"),
        size: wgpu::Extent3d {
            width: 1440,
            height: 900,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let ctx = egui::Context::default();
    let window = Rect::from_min_max(pos2(0.0, 0.0), pos2(1440.0, 900.0));
    let app = App::new(MockSide::new(MockEngine::new(), None), format);
    let mut measured = Measured::new(app, Some(session(Some(Run::Idle))), Box::new(|_| {}));
    let mut eframe_frame = eframe::Frame::_new_kittest();
    for _ in 0..3 {
        let mut raw = egui::RawInput {
            screen_rect: Some(window),
            ..Default::default()
        };
        eframe::App::raw_input_hook(&mut measured, &ctx, &mut raw);
        let mut out = ctx.run_ui(raw, |ui| {
            eframe::App::ui(&mut measured, ui, &mut eframe_frame);
        });
        let primitives = ctx.tessellate(std::mem::take(&mut out.shapes), out.pixels_per_point);
        for (id, deltas) in &out.textures_delta.set {
            for delta in deltas {
                renderer.update_texture(&context.device, &context.queue, *id, delta);
            }
        }
        let descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [1440, 900],
            pixels_per_point: out.pixels_per_point,
        };
        let mut encoder = context.device.create_command_encoder(&Default::default());
        let commands = renderer.update_buffers(
            &context.device,
            &context.queue,
            &mut encoder,
            &primitives,
            &descriptor,
        );
        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        renderer.render(&mut pass.forget_lifetime(), &primitives, &descriptor);
        context
            .queue
            .submit(commands.into_iter().chain([encoder.finish()]));
        out.textures_delta.clear();
    }
    // The run started in the first frame's step, after that frame's start: the first frame's paint is in it; the third
    // is still in flight.
    let s = measured.session.as_ref().expect("the session");
    let frames = s.frames();
    assert_eq!(frames.len(), 2, "two frames finished and recorded");
    assert!(frames[1].at_ms > frames[0].at_ms, "in order: {frames:?}");
    assert!(
        frames[0].work_ms > 0.0,
        "work from the frame's start: {frames:?}"
    );
    assert!(
        frames[0].acquire_ms >= 0.0,
        "the marks in order: {frames:?}"
    );
    rejects("a frame that took no work", || {
        assert!(frames[0].work_ms <= 0.0);
    });
}

/// The measured native window: the panel's strip added to the window's size, so the app keeps it.
fn measured_window() -> Rect {
    Rect::from_min_max(pos2(0.0, 0.0), MEASURED_WINDOW_SIZE.into())
}

/// One frame of `measured` with `events`, its focus rings' rects: the rects `RING_WIDTH` wide.
fn step_rings(
    ctx: &egui::Context,
    measured: &mut Measured<MockSide>,
    events: Vec<egui::Event>,
) -> Vec<Rect> {
    let mut raw = egui::RawInput {
        screen_rect: Some(measured_window()),
        events,
        ..Default::default()
    };
    measured.raw_input_hook(ctx, &mut raw);
    let mut out = ctx.run_ui(raw, |ui| measured.ui(ui));
    out.textures_delta.clear();
    fn collect(shape: &egui::Shape, rings: &mut Vec<Rect>) {
        match shape {
            egui::Shape::Vec(v) => v.iter().for_each(|s| collect(s, rings)),
            egui::Shape::Rect(r) if r.stroke.width == RING_WIDTH => rings.push(r.rect),
            _ => {}
        }
    }
    let mut rings = Vec::new();
    out.shapes
        .iter()
        .for_each(|c| collect(&c.shape, &mut rings));
    rings
}

/// Presses and releases `key`, with `modifiers`.
fn press_on(
    ctx: &egui::Context,
    measured: &mut Measured<MockSide>,
    key: Key,
    modifiers: Modifiers,
) {
    for events in crate::capture::key(key, modifiers) {
        step_rings(ctx, measured, events);
    }
}

fn measured_panel() -> (egui::Context, Measured<MockSide>) {
    let ctx = egui::Context::default();
    let measured = Measured::new(mock_app(), Some(session(None)), Box::new(|_| {}));
    (ctx, measured)
}

fn focus_of(measured: &Measured<MockSide>) -> Vec<&str> {
    let path = &measured.app.view.focus.path;
    path.iter().map(String::as_str).collect()
}

fn phase_of(measured: &Measured<MockSide>) -> Phase {
    measured.session.as_ref().expect("the session").phase
}

/// Tab and Enter reach the panel's buttons in navigation mode, the ring and the breadcrumb following; the arrows move
/// between them and start nothing; Enter on one acts as its click, stays in navigation mode and adds no undo entry.
#[test]
fn measure_panel_keyboard_tab_and_enter_reach_its_buttons() {
    let (ctx, mut measured) = measured_panel();
    step_rings(&ctx, &mut measured, Vec::new());
    let before = measured.app.side().engine().snapshot();
    let none = Modifiers::NONE;
    press_on(&ctx, &mut measured, Key::Tab, Modifiers::SHIFT);
    assert_eq!(focus_of(&measured), [PANEL_SCOPE], "last in the Tab order");
    press_on(&ctx, &mut measured, Key::Tab, Modifiers::SHIFT);
    assert_eq!(focus_of(&measured), [FOOTER], "the footer comes before it");
    press_on(&ctx, &mut measured, Key::Tab, none);
    assert_eq!(focus_of(&measured), [PANEL_SCOPE], "Tab from the footer");
    let tree = measured.app.keyboard.tree(Mode::Explore);
    let first = tree.bigs()[0];
    let label = |id: &str| tree.get(id).expect("the scope").label;
    assert_eq!(label(PANEL_SCOPE), "Measurement");
    let breadcrumb_at = |m: &Measured<MockSide>| {
        let tree = m.app.keyboard.tree(Mode::Explore);
        breadcrumb::text(tree, &m.app.view.focus)
    };
    assert!(breadcrumb_at(&measured).contains("Measurement"));
    let rings = step_rings(&ctx, &mut measured, Vec::new());
    let strip = split(measured_window()).1;
    assert_eq!(rings, [strip], "the ring on the panel");

    press_on(&ctx, &mut measured, Key::Enter, none);
    assert_eq!(focus_of(&measured), [PANEL_SCOPE, RUN_PAN], "Enter goes in");
    assert_eq!(measured.app.view.focus.mode, KeyMode::Navigation);
    assert!(breadcrumb_at(&measured).ends_with("Run the scripted pan"));
    let check_ring_on = |measured: &mut Measured<MockSide>, id: &str| {
        let rings = step_rings(&ctx, measured, Vec::new());
        let place = measured.app.keyboard.place_of(id).expect("its place");
        assert!(place.widget.is_some(), "the button's widget");
        assert!(strip.contains_rect(place.rect), "in the panel's strip");
        assert_eq!(rings, [place.rect], "one ring, on the button");
    };
    check_ring_on(&mut measured, RUN_PAN);

    press_on(&ctx, &mut measured, Key::ArrowDown, none);
    assert_eq!(focus_of(&measured), [PANEL_SCOPE, RUN_IDLE], "↓ moves on");
    assert!(breadcrumb_at(&measured).ends_with("Run idle"));
    check_ring_on(&mut measured, RUN_IDLE);
    assert_eq!(phase_of(&measured), Phase::Ready, "an arrow starts nothing");

    press_on(&ctx, &mut measured, Key::Enter, none);
    step_rings(&ctx, &mut measured, Vec::new());
    assert!(
        matches!(phase_of(&measured), Phase::Running { run: Run::Idle, .. }),
        "Enter on Run idle: {:?}",
        phase_of(&measured)
    );
    assert_eq!(measured.app.view.focus.mode, KeyMode::Navigation);
    assert_eq!(focus_of(&measured), [PANEL_SCOPE, RUN_IDLE], "it stays");
    let after = measured.app.side().engine().snapshot();
    assert_eq!(after.history.undo_depth, before.history.undo_depth);

    press_on(&ctx, &mut measured, Key::Escape, none);
    assert_eq!(focus_of(&measured), [PANEL_SCOPE], "Esc goes up");
    press_on(&ctx, &mut measured, Key::Tab, none);
    assert_eq!(focus_of(&measured), [first], "Tab wraps to the first scope");
    rejects("a focus left on the footer", || {
        assert_eq!(focus_of(&measured), [FOOTER]);
    });
}

/// Enter on a button during a run neither restarts it nor changes it, as a click on the disabled button does not.
#[test]
fn measure_panel_keyboard_enter_during_a_run_starts_nothing() {
    let (ctx, mut measured) = measured_panel();
    for key in [Key::Tab, Key::Enter, Key::Enter] {
        let modifiers = if key == Key::Tab {
            Modifiers::SHIFT
        } else {
            Modifiers::NONE
        };
        press_on(&ctx, &mut measured, key, modifiers);
    }
    step_rings(&ctx, &mut measured, Vec::new());
    let running = phase_of(&measured);
    assert!(
        matches!(running, Phase::Running { run: Run::Pan, .. }),
        "Enter on Run the scripted pan: {running:?}"
    );
    press_on(&ctx, &mut measured, Key::ArrowDown, Modifiers::NONE);
    press_on(&ctx, &mut measured, Key::Enter, Modifiers::NONE);
    step_rings(&ctx, &mut measured, Vec::new());
    assert_eq!(phase_of(&measured), running, "the run goes on, unchanged");
    rejects("a run the other button replaced", || {
        assert!(matches!(
            phase_of(&measured),
            Phase::Running { run: Run::Idle, .. }
        ));
    });
}

/// The panel joins both pages' trees, last in the Tab order, only while it is open.
#[test]
fn measure_panel_keyboard_joins_both_trees_only_while_open() {
    let (_, measured) = measured_panel();
    let check = |measured: &Measured<MockSide>| {
        for mode in [Mode::Explore, Mode::Stain] {
            let tree = measured.app.keyboard.tree(mode);
            assert_eq!(tree.bigs().last(), Some(&PANEL_SCOPE), "{mode:?}");
            assert_eq!(tree.children(Some(PANEL_SCOPE)), [RUN_PAN, RUN_IDLE]);
        }
    };
    check(&measured);
    let plain = Measured::new(mock_app(), None, Box::new(|_| {}));
    for mode in [Mode::Explore, Mode::Stain] {
        let tree = plain.app.keyboard.tree(mode);
        assert_eq!(tree.get(PANEL_SCOPE), None, "no session, no scope");
    }
    rejects("a tree with no panel", || check(&plain));
}

/// The ring is drawn once wherever the focus is, and the panel places no scope and draws no ring while the layer is
/// hidden or presenting: nothing would clear the places.
#[test]
fn measure_panel_keyboard_ring_follows_the_layer() {
    let (ctx, mut measured) = measured_panel();
    step_rings(&ctx, &mut measured, Vec::new());
    press_on(&ctx, &mut measured, Key::Tab, Modifiers::SHIFT);
    press_on(&ctx, &mut measured, Key::Tab, Modifiers::SHIFT);
    assert_eq!(focus_of(&measured), [FOOTER]);
    let rings = step_rings(&ctx, &mut measured, Vec::new());
    assert_eq!(rings.len(), 1, "the app's ring, not drawn twice: {rings:?}");
    press_on(&ctx, &mut measured, Key::Tab, Modifiers::NONE);
    assert_eq!(focus_of(&measured), [PANEL_SCOPE]);
    assert_eq!(step_rings(&ctx, &mut measured, Vec::new()).len(), 1);

    measured.app.shown = false;
    assert_eq!(step_rings(&ctx, &mut measured, Vec::new()), [], "hidden");
    measured.app.shown = true;
    assert_eq!(step_rings(&ctx, &mut measured, Vec::new()).len(), 1);
    measured.app.present = true;
    assert_eq!(
        step_rings(&ctx, &mut measured, Vec::new()),
        [],
        "presenting"
    );
    rejects("a ring drawn while presenting", || {
        assert_eq!(step_rings(&ctx, &mut measured, Vec::new()).len(), 1);
    });
}

/// An Enter pressed and released within one frame still starts the run: nothing but the panel asks for the next frame,
/// the layer having taken the keys out of egui's input.
#[test]
fn measure_panel_keyboard_enter_asks_for_the_frame_that_starts_the_run() {
    let (ctx, mut measured) = measured_panel();
    press_on(&ctx, &mut measured, Key::Tab, Modifiers::SHIFT);
    press_on(&ctx, &mut measured, Key::Enter, Modifiers::NONE);
    let delay_after = |measured: &mut Measured<MockSide>, key: Key| {
        let [down, up] = crate::capture::key(key, Modifiers::NONE);
        let mut raw = egui::RawInput {
            screen_rect: Some(measured_window()),
            events: [down, up].concat(),
            ..Default::default()
        };
        measured.raw_input_hook(&ctx, &mut raw);
        let mut out = ctx.run_ui(raw, |ui| measured.ui(ui));
        out.textures_delta.clear();
        out.viewport_output[&ViewportId::ROOT].repaint_delay
    };
    // Settled, so the frames before these ask for nothing of their own.
    for _ in 0..5 {
        step_rings(&ctx, &mut measured, Vec::new());
    }
    let unrelated = delay_after(&mut measured, Key::ArrowDown);
    assert_eq!(focus_of(&measured), [PANEL_SCOPE, RUN_IDLE]);
    assert_eq!(phase_of(&measured), Phase::Ready);
    assert_ne!(unrelated, Duration::ZERO, "an arrow asks for no frame");
    let enter = delay_after(&mut measured, Key::Enter);
    assert_eq!(
        enter,
        Duration::ZERO,
        "Enter asks for the next frame at once"
    );
    rejects("a frame that is not asked for at once", || {
        assert_eq!(unrelated, Duration::ZERO);
    });
}

/// A frame the layer does not run leaves no activation of an earlier frame behind.
#[test]
fn measure_panel_keyboard_activation_does_not_outlive_its_frame() {
    let (ctx, mut measured) = measured_panel();
    press_on(&ctx, &mut measured, Key::Tab, Modifiers::SHIFT);
    press_on(&ctx, &mut measured, Key::Enter, Modifiers::NONE);
    let [enter, release] = crate::capture::key(Key::Enter, Modifiers::NONE);
    step_rings(&ctx, &mut measured, enter);
    assert_eq!(measured.app.keyboard.activated(), [RUN_PAN], "its frame");
    measured.app.shown = false;
    step_rings(&ctx, &mut measured, release);
    assert_eq!(
        measured.app.keyboard.activated(),
        [] as [&str; 0],
        "stood down"
    );
    rejects("a stale activation", || {
        assert_eq!(measured.app.keyboard.activated(), [RUN_PAN]);
    });
}

/// The measured window leaves the app the plain window's size, the panel's strip added, as the Chrome runner asks.
#[test]
fn measure_window_leaves_the_app_its_size() {
    let check = |window: [f32; 2]| {
        let window = Rect::from_min_max(pos2(0.0, 0.0), window.into());
        let want: egui::Vec2 = WINDOW_SIZE.into();
        assert_eq!(split(window).0.size(), want);
    };
    check(MEASURED_WINDOW_SIZE);
    rejects("the plain window, with the panel inside it", || {
        check(WINDOW_SIZE)
    });
    let chrome = include_str!("../../../../web/skeleton/runner/chrome.mjs");
    let [width, height] = MEASURED_WINDOW_SIZE;
    let asked = format!("--window-size={width},{height}");
    assert!(chrome.contains(&asked), "chrome.mjs asks for {asked}");
}

/// The panel's fields carry the app's size, from the panel's first report on.
#[test]
fn measure_reports_the_apps_size() {
    let ctx = egui::Context::default();
    let reports = Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = reports.clone();
    let mut measured = Measured::new(
        mock_app(),
        Some(session(None)),
        Box::new(move |r| sink.lock().unwrap().push(r)),
    );
    let s = measured.session.as_ref().expect("the session");
    assert_eq!(
        s.report()["app_width"],
        serde_json::Value::Null,
        "no frame yet"
    );
    assert_eq!(s.report()["app_height"], serde_json::Value::Null);
    let row = |s: &Session| {
        s.rows()
            .into_iter()
            .find(|r| r.0 == "app size")
            .expect("the row")
            .1
    };
    assert_eq!(row(s), "–");
    // Heights apart from the widths, so swapped fields fail.
    let window = Rect::from_min_max(pos2(0.0, 0.0), pos2(1500.0, 700.0));
    frame(&ctx, &mut measured, window, 0.0);
    let first: serde_json::Value =
        serde_json::from_str(&reports.lock().unwrap()[0]).expect("the JSON");
    assert_eq!(
        first["app_width"], 1200.0,
        "the width less the panel's strip"
    );
    assert_eq!(first["app_height"], 700.0);
    let s = measured.session.as_ref().expect("the session");
    assert_eq!(row(s), "1200 × 700 pt");
    rejects("the window's width", || {
        assert_eq!(first["app_width"], 1500.0);
    });
}
