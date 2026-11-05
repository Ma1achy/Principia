//! QA tests for TASK-M8-86, written from the task's deliverable "the frame statistics" and its panel deliverable, not
//! from the implementation: the window is the 30 s after a 5 s warm-up; the mean fps is the frames presented in it over
//! its duration; the 5th-percentile fps is the nearest rank of `1000 / interval_ms` over those frames; the work time's
//! mean and nearest-rank 95th percentile; the refresh rate is the median interval of an idle loop; each figure is
//! "capped" on its own within 2% of it. The scripted pan is one figure width every 5 s, back and forth, where the figure's
//! width is `2 q₁` (chart_decoder_contract Part 3, `z = z₀ + (2s−1) q₁ + (2t−1) q₂`). The panel is beside the figure,
//! edits no state and no history, and reports the same fields in both builds. gui takes no dependency on validation
//! (R-187), so each check runs its own control (R-176) through [`rejects`].
// The file name `qa_TASK-M8-86` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use eframe::egui::{self, pos2, Rect, ViewportId};
use engine::contract::interface::EngineInterface;
use gui::app::App;
use gui::capture::FORMAT;
use gui::measure::stats::{capped, nearest_rank, refresh_hz, window_stats, Frame, WindowStats};
use gui::measure::{
    frame_delay, pan_edit, pan_offset, split, Environment, Measured, Phase, Run, Session,
    PANEL_WIDTH,
};
use gui::mock::MockEngine;
use gui::side::{EngineSide, MockSide};

fn rejects(what: &str, check: impl FnOnce()) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check));
    assert!(
        outcome.is_err(),
        "the check passed on {what}, which it must reject"
    );
}

fn close(got: f64, want: f64, what: &str) {
    assert!((got - want).abs() < 1e-9, "{what}: {got}, not {want}");
}

fn frame(at_ms: f64) -> Frame {
    Frame {
        at_ms,
        work_ms: 1.0,
        acquire_ms: 0.0,
    }
}

/// `count` frames from `from` ms, `step` ms apart.
fn frames(from: f64, step: f64, count: u32) -> impl Iterator<Item = Frame> {
    (0..count).map(move |k| frame(from + step * f64::from(k)))
}

type Stats = dyn Fn(&[Frame], Option<f64>) -> Option<WindowStats>;

/// 1500 frames at 10 ms, 500 at 30 ms, and frames past the window: 2000 frames in the 30 s, so 66.67 fps, where the mean
/// of the per-frame fps is 83.3; rank 100 of 2000 is a 30 ms frame's 33.3. Then a short window after a slow warm-up: 60
/// frames at 10 ms give rank 3, 100 fps, where the warm-up's five slow frames in it would give rank 4 of 65, 1 fps.
fn check_window(stats: &Stats) {
    let mut run: Vec<Frame> = [1000.0, 2000.0, 3000.0, 4000.0, 4990.0].map(frame).to_vec();
    run.extend(frames(5000.0, 10.0, 1500));
    run.extend(frames(20000.0, 30.0, 500));
    run.extend([35000.5, 36000.0, 37000.0].map(frame));
    let got = stats(&run, None).expect("frames in the window");
    assert_eq!(got.frames, 2000, "the frames presented in the window");
    close(got.mean_fps.fps, 2000.0 / 30.0, "the mean fps");
    close(got.p5_fps.fps, 1000.0 / 30.0, "the 5th-percentile fps");

    let mut short: Vec<Frame> = [0.0, 1000.0, 2000.0, 3000.0, 4000.0, 4990.0]
        .map(frame)
        .to_vec();
    short.extend(frames(5000.0, 10.0, 60));
    let got = stats(&short, None).expect("frames in the window");
    assert_eq!(got.frames, 60, "the warm-up's frames are not in the window");
    close(got.mean_fps.fps, 2.0, "60 frames in the 30 s");
    close(got.p5_fps.fps, 100.0, "the warm-up is in no figure");
}

#[test]
fn qa_m8_86_window_mean_and_percentile() {
    check_window(&window_stats);
    rejects("a window with the warm-up in it", || {
        check_window(&|f, r| {
            let mut s = window_stats(f, r)?;
            let every: Vec<f64> = f
                .windows(2)
                .map(|w| 1000.0 / (w[1].at_ms - w[0].at_ms))
                .collect();
            s.p5_fps.fps = nearest_rank(&every, 5.0)?;
            Some(s)
        });
    });
    rejects("a mean taken as the mean of the per-frame fps", || {
        check_window(&|f, r| {
            let mut s = window_stats(f, r)?;
            let fps: Vec<f64> = f
                .windows(2)
                .filter(|w| (5000.0..35000.0).contains(&w[1].at_ms))
                .map(|w| 1000.0 / (w[1].at_ms - w[0].at_ms))
                .collect();
            s.mean_fps.fps = fps.iter().sum::<f64>() / fps.len() as f64;
            Some(s)
        });
    });
    rejects("a mean over the time the frames span", || {
        check_window(&|f, r| {
            let mut s = window_stats(f, r)?;
            s.mean_fps.fps = f64::from(s.frames as u32) / 29.0;
            Some(s)
        });
    });
}

/// The mean is over the window's duration even when the frames stop early: a hung run reads low.
#[test]
fn qa_m8_86_mean_is_over_the_window_not_the_frames_span() {
    let run: Vec<Frame> = frames(5000.0, 10.0, 600).collect();
    close(
        window_stats(&run, None).expect("frames").mean_fps.fps,
        20.0,
        "600 frames in the 30 s",
    );
}

/// Every frame in the window counts, its first by the interval from the warm-up's last: a stall across the boundary is a
/// frame presented in the window.
#[test]
fn qa_m8_86_the_windows_first_frame_has_its_interval() {
    let mut run = vec![frame(3000.0)];
    run.extend(frames(5000.0, 20.0, 19));
    close(
        window_stats(&run, None).expect("frames").p5_fps.fps,
        0.5,
        "rank 1 of 19: the 2000 ms stall",
    );
}

/// The nearest rank, where it differs from the interpolated value: of 25 intervals rank 2 of the fps; of 40 work times
/// rank 38.
fn check_ranks(stats: &Stats) {
    let mut at = 5000.0;
    let mut run = Vec::new();
    for k in 0..25 {
        at += f64::from(100 - k);
        run.push(frame(at));
    }
    run.insert(0, frame(5000.0 - 1.0));
    close(
        stats(&run, None).expect("frames").p5_fps.fps,
        1000.0 / 99.0,
        "rank 2 of the fps",
    );
    let works: Vec<Frame> = (0..40)
        .map(|k| Frame {
            work_ms: f64::from(k + 1),
            ..frame(5000.0 + 20.0 * f64::from(k))
        })
        .collect();
    let got = stats(&works, None).expect("frames");
    close(got.work_mean_ms, 20.5, "the work time's mean");
    close(got.work_p95_ms, 38.0, "the work time's rank 38 of 40");
}

#[test]
fn qa_m8_86_percentiles_are_by_the_nearest_rank() {
    check_ranks(&window_stats);
    rejects("an interpolated work percentile", || {
        check_ranks(&|f, r| {
            let mut s = window_stats(f, r)?;
            s.work_p95_ms += 0.05;
            Some(s)
        });
    });
    rejects("an fps percentile read from the work times", || {
        check_ranks(&|f, r| {
            let mut s = window_stats(f, r)?;
            s.p5_fps.fps = 1000.0 / f[0].work_ms;
            Some(s)
        });
    });
    // The rank itself, by hand, at the counts a run gives.
    assert_eq!(nearest_rank(&[3.0, 1.0, 2.0], 5.0), Some(1.0));
    assert_eq!(nearest_rank(&[3.0, 1.0, 2.0], 95.0), Some(3.0));
    assert_eq!(nearest_rank(&[], 5.0), None);
    for n in [20usize, 1800, 3600, 7200, 18000] {
        let sorted: Vec<f64> = (1..=n).map(|v| v as f64).collect();
        assert_eq!(nearest_rank(&sorted, 5.0), Some((n / 20) as f64), "n={n}");
        assert_eq!(
            nearest_rank(&sorted, 95.0),
            Some((n * 19 / 20) as f64),
            "n={n}"
        );
    }
}

/// A run whose mean is 48 fps and whose 5th percentile is 40 fps: 1200 frames at 20 ms, then 240 at 25 ms.
fn two_rates() -> Vec<Frame> {
    let mut run = vec![frame(4980.0)];
    run.extend(frames(5000.0, 20.0, 1200));
    run.extend(frames(29000.0, 25.0, 240));
    run
}

/// Each figure is marked on its own, within 2% of the refresh rate, on either side of it.
fn check_marks(stats: &Stats) {
    let marks = |hz: Option<f64>| {
        let got = stats(&two_rates(), hz).expect("frames");
        close(got.mean_fps.fps, 48.0, "the mean");
        close(got.p5_fps.fps, 40.0, "the percentile");
        (got.mean_fps.capped, got.p5_fps.capped)
    };
    assert_eq!(marks(Some(48.5)), (true, false), "the mean 1% below");
    assert_eq!(marks(Some(47.5)), (true, false), "the mean 1% above");
    assert_eq!(marks(Some(40.5)), (false, true), "the percentile 1% below");
    assert_eq!(marks(Some(50.0)), (false, false), "the mean 4% below");
    assert_eq!(marks(Some(60.0)), (false, false), "both far below");
    assert_eq!(marks(None), (false, false), "no rate caps nothing");
}

#[test]
fn qa_m8_86_each_figure_is_capped_on_its_own_at_2_percent() {
    check_marks(&window_stats);
    rejects("a capped mark at 10%", || {
        check_marks(&|f, r| {
            let mut s = window_stats(f, r)?;
            let wide = |fps: f64| r.is_some_and(|hz| (fps - hz).abs() <= 0.10 * hz);
            s.mean_fps.capped = wide(s.mean_fps.fps);
            s.p5_fps.capped = wide(s.p5_fps.fps);
            Some(s)
        });
    });
    rejects("one mark for both figures, from the mean", || {
        check_marks(&|f, r| {
            let mut s = window_stats(f, r)?;
            s.p5_fps.capped = s.mean_fps.capped;
            Some(s)
        });
    });
    assert!(capped(59.0, Some(60.0)));
    assert!(!capped(58.0, Some(60.0)));
}

/// The median interval of the idle loop, which an outlier does not move; a loop with no interval to read gives none.
fn check_refresh(refresh: impl Fn(&[f64]) -> Option<f64>) {
    let hz = refresh(&[0.0, 10.0, 20.0, 30.0, 130.0, 140.0]).expect("intervals");
    close(hz, 100.0, "a stalled frame is not the display's rate");
    let hz = refresh(&[0.0, 10.0, 30.0]).expect("intervals");
    close(hz, 1000.0 / 15.0, "the middle two of an even count");
    assert_eq!(refresh(&[]), None, "no stamp");
    assert_eq!(refresh(&[5.0]), None, "one stamp");
    assert_eq!(refresh(&[3.0, 3.0, 3.0]), None, "no positive interval");
}

#[test]
fn qa_m8_86_refresh_is_the_median_of_the_idle_loop() {
    check_refresh(refresh_hz);
    rejects("the mean interval", || {
        check_refresh(|stamps| {
            let n = stamps.len().checked_sub(1).filter(|n| *n > 0)? as f64;
            let ms = (stamps.last()? - stamps.first()?) / n;
            (ms > 0.0).then(|| 1000.0 / ms)
        });
    });
}

/// The browser's refresh rate comes from the idle loop alone, whatever delay the frames carry; natively, from the
/// monitor's, also whatever the delay.
#[test]
fn qa_m8_86_the_delay_does_not_move_the_refresh_rate() {
    let browser = |delay: &str| {
        Environment::browser(&format!(
            r#"{{"layout":"worker","idleLoopMs":[0,10,20,30],"frameDelayMs":"{delay}"}}"#
        ))
        .expect("the loader's JSON")
    };
    let (none, slow) = (browser("0"), browser("50"));
    assert_eq!(slow.frame_delay_ms, 50.0, "the delay is read");
    assert_eq!(none.refresh_hz, Some(100.0));
    assert_eq!(slow.refresh_hz, none.refresh_hz, "it fell with the delay");
    let native = Environment::native(Some("1"), Some("50"), None, Some(120.0))
        .expect("good flags")
        .expect("open");
    assert_eq!(
        (native.frame_delay_ms, native.refresh_hz),
        (50.0, Some(120.0))
    );
}

/// `1` opens the panel; nothing else does, and a value that is no delay or no run is refused.
#[test]
fn qa_m8_86_flags() {
    for off in [None, Some("0"), Some(""), Some("true"), Some("11")] {
        assert_eq!(
            Environment::native(off, None, None, None),
            Ok(None),
            "{off:?}"
        );
    }
    let open = Environment::native(Some("1"), None, None, Some(60.0))
        .expect("good flags")
        .expect("open");
    assert_eq!(open.layout, "native");
    assert_eq!(open.cross_origin_isolated, None, "browser-only");
    assert_eq!(frame_delay(None), Ok(0.0));
    assert_eq!(frame_delay(Some("16.5")), Ok(16.5));
    for bad in ["-1", "NaN", "inf", "-inf", "1e999", "", "abc", "5ms"] {
        assert!(frame_delay(Some(bad)).is_err(), "the delay {bad:?}");
    }
    for bad in ["PAN", "", "both"] {
        assert!(Run::parse(bad).is_err(), "the run {bad:?}");
    }
    assert!(Environment::native(Some("1"), Some("x"), None, None).is_err());
    assert!(Environment::native(Some("1"), None, Some("x"), None).is_err());
    assert!(
        Environment::browser(r#"{"crossOriginIsolated":true}"#).is_err(),
        "no layout"
    );
    assert!(Environment::browser(r#"{"layout":"main","frameDelayMs":"x"}"#).is_err());
    assert!(Environment::browser(r#"{"layout":"main","run":"x"}"#).is_err());
    assert!(Environment::browser("not json").is_err());
    let env = Environment::browser(
        r#"{"crossOriginIsolated":false,"sharedArrayBuffer":true,"wasmThreads":"ok",
            "layout":"main","fallbackReason":"OffscreenCanvas is missing","run":"pan"}"#,
    )
    .expect("the loader's JSON");
    assert_eq!(env.cross_origin_isolated, Some(false));
    assert_eq!(env.shared_array_buffer, Some(true));
    assert_eq!(env.wasm_threads.as_deref(), Some("ok"));
    assert_eq!(env.layout, "main");
    assert_eq!(
        env.fallback_reason.as_deref(),
        Some("OffscreenCanvas is missing"),
        "the worker's reason"
    );
    assert_eq!(env.run, Some(Run::Pan));
}

/// One figure width in 5 s, there and back, at a constant rate.
fn check_pan_rate(offset: impl Fn(f64) -> f64) {
    for (ms, want) in [
        (0.0, 0.0),
        (1250.0, 0.25),
        (2500.0, 0.5),
        (5000.0, 1.0),
        (6250.0, 0.75),
        (7500.0, 0.5),
        (10000.0, 0.0),
        (12500.0, 0.5),
        (35000.0, 1.0),
    ] {
        close(offset(ms), want, &format!("at {ms} ms"));
    }
}

#[test]
fn qa_m8_86_pan_is_one_width_every_5_s_back_and_forth() {
    check_pan_rate(pan_offset);
    rejects("a pan that does not come back", || {
        check_pan_rate(|ms| ms / 5000.0);
    });
    rejects("a pan at half the rate", || {
        check_pan_rate(|ms| pan_offset(ms / 2.0));
    });
    rejects("a pan eased, not constant", || {
        check_pan_rate(|ms| pan_offset(ms).powi(2));
    });
}

fn mock_app() -> App<MockSide> {
    App::new(MockSide::new(MockEngine::new(), None), FORMAT)
}

/// `pan_edit` moves the view `offset × 2 q₁`, one figure width at 1, along the view's own axis, from the plane it is
/// given whatever the view is now, with no undo entry.
#[test]
fn qa_m8_86_pan_edit_goes_its_offset_in_figure_widths() {
    for offset in [0.25, 0.5, 1.0] {
        let mut app = mock_app();
        let from = app.side().engine().snapshot();
        let plane = &from.sim.plane;
        let width: f64 = plane.q1.iter().map(|x| 4.0 * x * x).sum::<f64>().sqrt();
        assert!(width > 0.0, "the mock's view has a width");
        app.set_field(pan_edit(plane, 0.75));
        app.set_field(pan_edit(plane, offset));
        let to = app.side().engine().snapshot();
        let moved: Vec<f64> = (0..8).map(|i| to.sim.plane.z0[i] - plane.z0[i]).collect();
        let length = moved.iter().map(|x| x * x).sum::<f64>().sqrt();
        assert!(
            (length - offset * width).abs() < 1e-5 * width,
            "{offset}: moved {length}, a width is {width}"
        );
        let along: f64 = (0..8).map(|i| moved[i] * plane.q1[i]).sum();
        let q1: f64 = plane.q1.iter().map(|x| x * x).sum::<f64>().sqrt();
        assert!(
            (along.abs() - length * q1).abs() < 1e-5 * width * q1,
            "{offset}: not along q₁"
        );
        assert_eq!(
            to.history.undo_depth, from.history.undo_depth,
            "a scripted pan leaves no undo entry"
        );
        assert_eq!(to.sim.plane.q1, plane.q1, "the scale is the view's");
    }
}

/// The run's script on the session's clock: the warm-up and the window, then the figures, once.
#[test]
fn qa_m8_86_a_run_ends_after_the_warm_up_and_the_window() {
    let env = Environment {
        layout: "native".to_owned(),
        run: Some(Run::Pan),
        ..Environment::default()
    };
    let mut session = Session::new(env);
    assert!(!session.running(), "a run starts at the first step");
    let first = session.step(1000.0);
    assert!(first.started && !first.finished);
    close(first.pan.expect("the pan's offset"), 0.0, "from the start");
    assert!(!session.step(1000.0 + 34_999.0).finished, "in the window");
    close(
        session.step(1000.0 + 7500.0).pan.expect("pan"),
        0.5,
        "the clock runs from the run's start",
    );
    let last = session.step(1000.0 + 35_000.0);
    assert!(last.finished && last.pan.is_none(), "35 s in");
    assert!(!session.running());
    assert_eq!(
        session.phase,
        Phase::Done {
            run: Run::Pan,
            stats: None
        },
        "no frame in its window"
    );
    assert!(!session.step(50_000.0).finished, "it ends once");

    session.request(Run::Idle);
    let idle = session.step(60_000.0);
    assert!(idle.started && idle.pan.is_none(), "idle has no pan");
    session.request(Run::Pan);
    let during = session.step(61_000.0);
    assert!(!during.started, "a request during a run starts nothing");
    assert!(during.pan.is_none(), "it stays idle");
}

fn report_keys(report: &serde_json::Value) -> Vec<String> {
    let mut keys: Vec<String> = report
        .as_object()
        .expect("the report is an object")
        .keys()
        .cloned()
        .collect();
    keys.sort();
    keys
}

/// The panel's items in the rows and the report, in each phase, on one set of keys in both builds; a figure's value and
/// its mark as computed.
#[test]
fn qa_m8_86_both_builds_report_the_same_fields() {
    let native = Session::new(
        Environment::native(Some("1"), None, None, Some(48.5))
            .expect("flags")
            .expect("open"),
    );
    let mut browser = Session::new(
        Environment::browser(
            r#"{"crossOriginIsolated":true,"sharedArrayBuffer":true,"wasmThreads":"ok","layout":"worker"}"#,
        )
        .expect("the loader's JSON"),
    );
    assert_eq!(
        report_keys(&native.report()),
        report_keys(&browser.report()),
        "the two builds' fields"
    );
    let ready = report_keys(&browser.report());
    for item in [
        "cross_origin_isolated",
        "shared_array_buffer",
        "wasm_threads",
        "layout",
        "fallback_reason",
        "refresh_hz",
        "mean_fps",
        "mean_fps_capped",
        "p5_fps",
        "p5_fps_capped",
        "work_mean_ms",
        "work_p95_ms",
    ] {
        assert!(ready.iter().any(|k| k == item), "the report lacks {item}");
    }
    browser.step(0.0);
    browser.request(Run::Idle);
    browser.step(0.0);
    assert_eq!(report_keys(&browser.report()), ready, "while running");

    let stats = window_stats(&two_rates(), Some(48.5));
    browser.phase = Phase::Done {
        run: Run::Idle,
        stats,
    };
    let report = browser.report();
    assert_eq!(report_keys(&report), ready, "when done");
    assert_eq!(report["status"], "done");
    assert_eq!(report["run"], "idle");
    assert_eq!(report["layout"], "worker");
    assert_eq!(report["mean_fps"], 48.0);
    assert_eq!(report["mean_fps_capped"], true);
    assert_eq!(report["p5_fps"], 40.0);
    assert_eq!(report["p5_fps_capped"], false);

    let rows = browser.rows();
    let row = |label: &str| {
        rows.iter()
            .find(|(l, _)| *l == label)
            .unwrap_or_else(|| panic!("no row {label}: {rows:?}"))
            .1
            .clone()
    };
    assert_eq!(row("crossOriginIsolated"), "yes");
    assert_eq!(row("SharedArrayBuffer"), "yes");
    assert_eq!(row("wasm threads"), "ok");
    assert_eq!(row("layout"), "worker");
    assert!(row("mean fps").contains("capped"), "{}", row("mean fps"));
    assert!(
        !row("5th-percentile fps").contains("capped"),
        "{}",
        row("5th-percentile fps")
    );
    for label in ["refresh rate", "work mean", "work p95"] {
        row(label);
    }
}

/// Beside the figure and never over it, in a window of any width.
fn check_beside(split: impl Fn(Rect) -> (Rect, Rect)) {
    for width in [1.0_f32, 299.0, 300.0, 301.0, 1440.0, 1740.0, 5000.0] {
        let window = Rect::from_min_max(pos2(10.0, 20.0), pos2(10.0 + width, 920.0));
        let (app, panel) = split(window);
        assert!(app.max.x <= panel.min.x, "{width}: they overlap");
        assert_eq!(app.min, window.min, "{width}: the app's corner");
        assert_eq!(panel.max, window.max, "{width}: the panel's corner");
        assert_eq!(app.height(), 900.0, "{width}: the app's height");
        assert_eq!(panel.height(), 900.0, "{width}: the panel's height");
        assert_eq!(
            panel.width(),
            PANEL_WIDTH.min(width),
            "{width}: the panel's width"
        );
    }
}

#[test]
fn qa_m8_86_panel_is_beside_the_figure() {
    check_beside(split);
    rejects("a panel laid over the app", || {
        check_beside(|w| {
            let (_, panel) = split(w);
            (w, panel)
        });
    });
}

fn run_frame(ctx: &egui::Context, measured: &mut Measured<MockSide>, window: Rect, time: f64) {
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

/// What opening the panel may not touch: the sim, the render state and the history, compared whole.
fn state(measured: &mut Measured<MockSide>) -> String {
    let snap = measured.app.side().engine().snapshot();
    format!("{:?} {:?} {:?}", snap.sim, snap.render, snap.history)
}

fn window() -> Rect {
    Rect::from_min_max(pos2(0.0, 0.0), pos2(1740.0, 900.0))
}

fn open_session() -> Session {
    Session::new(
        Environment::native(Some("1"), None, None, Some(60.0))
            .expect("flags")
            .expect("open"),
    )
}

/// Opening the panel leaves the engine's whole state as the app without it has it, and the app its 1440 × 900.
#[test]
fn qa_m8_86_opening_the_panel_edits_nothing() {
    let ctx = egui::Context::default();
    let mut with = Measured::new(mock_app(), Some(open_session()), Box::new(|_| {}));
    let mut without = Measured::new(mock_app(), None, Box::new(|_| {}));
    let plain = egui::Context::default();
    for i in 0..4 {
        run_frame(&ctx, &mut with, window(), f64::from(i) * 0.1);
        run_frame(&plain, &mut without, window(), f64::from(i) * 0.1);
    }
    assert_eq!(state(&mut with), state(&mut without), "the panel edited");
    let snap = with.app.side().engine().snapshot();
    assert!(!snap.render.debug_view, "it is not View › Debug view");
    let size = with.session.as_ref().expect("open").app_size;
    assert_eq!(size, Some(egui::vec2(1440.0, 900.0)), "the app's part");

    // The comparison can fail: a scripted pan does edit the view.
    rejects("a panel that pans the view", || {
        let plane = with.app.side().engine().snapshot().sim.plane.clone();
        with.app.set_field(pan_edit(&plane, 0.5));
        assert_eq!(state(&mut with), state(&mut without), "the panel edited");
    });
}

/// A flag's run starts the frame the panel opens; it reports at once, the run named, and a run under way asks for frames.
#[test]
fn qa_m8_86_a_flags_run_starts_as_the_panel_opens() {
    let ctx = egui::Context::default();
    let reports = std::rc::Rc::new(std::cell::RefCell::new(Vec::<String>::new()));
    let sink = reports.clone();
    let env = Environment::native(Some("1"), None, Some("idle"), Some(60.0))
        .expect("flags")
        .expect("open");
    let mut measured = Measured::new(
        mock_app(),
        Some(Session::new(env)),
        Box::new(move |json| sink.borrow_mut().push(json)),
    );
    run_frame(&ctx, &mut measured, window(), 0.0);
    assert!(measured.session.as_ref().expect("open").running());
    assert!(ctx.has_requested_repaint(), "a run repaints every frame");
    let first: serde_json::Value =
        serde_json::from_str(&reports.borrow()[0]).expect("the report's JSON");
    assert_eq!(first["status"], "running");
    assert_eq!(first["run"], "idle");
}
