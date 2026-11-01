//! The measurement panel (TASK-M8-86; R-627, render_gui_spec G1): beside the figure, never over it; opening it sends no
//! `SetField` and adds no undo entry; a run's script; the flags; and the report's fields, each in both builds' one
//! implementation.

use std::sync::Arc;
use std::time::Duration;

use eframe::egui::{self, pos2, Rect, ViewportId};
use eframe::egui_wgpu;
use engine::contract::interface::EngineInterface;

use super::support::{mock_app, rejects};
use crate::app::App;
use crate::cli::mock::{MEASURED_WINDOW_SIZE, WINDOW_SIZE};
use crate::headless::Headless;
use crate::measure::stats::{Frame, WARM_UP_MS, WINDOW_MS};
use crate::measure::{
    frame_delay, pan_edit, pan_offset, split, Environment, Measured, Phase, Run, Session,
    PANEL_WIDTH,
};
use crate::mock::canvas::MockCanvas;
use crate::mock::MockEngine;
use crate::side::{EngineSide, MockSide};

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
const REPORT_KEYS: [&str; 19] = [
    "status",
    "run",
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
];
const ROW_LABELS: [&str; 15] = [
    "status",
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
