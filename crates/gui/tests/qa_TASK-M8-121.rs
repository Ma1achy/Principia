//! QA tests for TASK-M8-121, written from REQ-PERF-119 and R-826, not from the implementation: a browser run started
//! from the measurement panel's button reads the refresh rate measured once at page load, so its report is marked as not
//! a measurement; a run started by `?run=` is unchanged (unmarked); a native run started from the panel's button reads
//! the monitor's rate as it starts, so it is unmarked too. The mark is the report's `not_a_measurement` field, `true`
//! on the marked run; the panel's row of the same name shows it. gui takes no dependency on validation (R-187), so each
//! check runs its own control (R-176) through [`rejects`].
// The file name `qa_TASK-M8-121` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use eframe::egui::{self, pos2, Rect, ViewportId};
use gui::app::App;
use gui::capture::FORMAT;
use gui::measure::stats::{WARM_UP_MS, WINDOW_MS};
use gui::measure::{Environment, Measured, Run, Session};
use gui::mock::MockEngine;
use gui::side::MockSide;
use serde_json::Value;

fn rejects(what: &str, check: impl FnOnce()) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check));
    assert!(
        outcome.is_err(),
        "the check passed on {what}, which it must reject"
    );
}

/// A browser session, as the loader's JSON gives it: an idle loop at 60 Hz measured at page load, and `?run=` when
/// `run` is given.
fn browser(run: Option<&str>) -> Session {
    let run = run.map_or(String::new(), |r| format!(r#","run":"{r}""#));
    let text = format!(
        r#"{{"crossOriginIsolated":true,"sharedArrayBuffer":true,"wasmThreads":"ok","layout":"worker","idleLoopMs":[0,16.667,33.333,50,66.667,83.333]{run}}}"#
    );
    Session::new(Environment::browser(&text).expect("the loader's JSON"))
}

/// A native session as `PRIN_MEASURE=1` opens it, `PRIN_MEASURE_RUN` being `run`.
fn native(run: Option<&str>) -> Session {
    Session::new(
        Environment::native(Some("1"), None, run, Some(60.0))
            .expect("the flags")
            .expect("open"),
    )
}

/// Where a session's run is read: as it starts, mid-run, and once it is done.
const TIMES: [f64; 3] = [0.0, WARM_UP_MS + 1.0, WARM_UP_MS + WINDOW_MS + 1.0];

/// Steps `s` through one whole run, the native frame reading the monitor (`Some(rate)`) or, the browser, nothing; returns
/// the report at each of [`TIMES`].
fn drive(s: &mut Session, monitor: Option<f64>) -> Vec<Value> {
    TIMES
        .iter()
        .map(|&t| {
            let step = s.step(t);
            if let Some(hz) = monitor {
                s.watch_display(step, None, &|| Some(hz));
            }
            s.report()
        })
        .collect()
}

/// The run reported, at every point, its status the one expected there, and the mark `marked` (JSON `true` for a run
/// that is not a measurement, `false` for one that is); the panel's row says the same at the end.
fn check_reports(reports: &[Value], s: &Session, run: &str, marked: bool) {
    assert_eq!(reports.len(), TIMES.len());
    for (report, status) in reports.iter().zip(["running", "running", "done"]) {
        assert_eq!(report["status"], status, "the run's phase");
        assert_eq!(report["run"], run, "the run reported");
        assert_eq!(
            report["not_a_measurement"],
            Value::Bool(marked),
            "{status}: the report's not-a-measurement mark"
        );
    }
    let row = s
        .rows()
        .into_iter()
        .find(|(label, _)| *label == "not a measurement")
        .expect("the panel's row for the mark")
        .1;
    assert_eq!(row, if marked { "yes" } else { "no" }, "the panel's row");
}

/// The arm: a browser run from the panel's button is marked, from its start to its end, for either run.
#[test]
fn qa_m8_121_browser_button_run_is_marked() {
    for (run, name) in [(Run::Idle, "idle"), (Run::Pan, "pan")] {
        let mut s = browser(None);
        assert_ne!(
            s.report()["not_a_measurement"],
            Value::Bool(true),
            "no run yet, nothing marked"
        );
        s.request(run);
        let reports = drive(&mut s, None);
        check_reports(&reports, &s, name, true);
        rejects("the button's run read as a measurement", || {
            check_reports(&reports, &s, name, false);
        });
        // The report the runner writes is the JSON text; the mark is in it.
        let text = reports[2].to_string();
        assert!(
            text.contains(r#""not_a_measurement":true"#),
            "the written report carries the mark: {text}"
        );
    }
}

/// The control: a `?run=` run in the browser is unchanged, unmarked, even when the panel's button is pressed while it
/// runs (a run under way ignores it).
#[test]
fn qa_m8_121_browser_flag_run_is_unmarked() {
    for name in ["idle", "pan"] {
        let mut s = browser(Some(name));
        let mut reports = Vec::new();
        for (k, &t) in TIMES.iter().enumerate() {
            if k == 1 {
                s.request(Run::Pan);
                s.request(Run::Idle);
            }
            s.step(t);
            reports.push(s.report());
        }
        check_reports(&reports, &s, name, false);
        rejects("a ?run= run marked", || {
            check_reports(&reports, &s, name, true)
        });
    }
}

/// The control: a native run from the panel's button reads the monitor's rate as it starts, so it is unmarked; so is a
/// native flag's run.
#[test]
fn qa_m8_121_native_runs_are_unmarked() {
    let mut button = native(None);
    button.request(Run::Idle);
    let reports = drive(&mut button, Some(60.0));
    check_reports(&reports, &button, "idle", false);
    rejects("a native button run marked", || {
        check_reports(&reports, &button, "idle", true)
    });

    let mut flag = native(Some("pan"));
    let reports = drive(&mut flag, Some(144.0));
    check_reports(&reports, &flag, "pan", false);
}

/// Each run is marked for how it was started, not for what ran before: in the browser, a `?run=` run, then a button's
/// run (marked), then another button's run (marked); natively, a button's run after a flag's (unmarked).
#[test]
fn qa_m8_121_mark_follows_each_run() {
    let mut s = browser(Some("idle"));
    let first = drive(&mut s, None);
    check_reports(&first, &s, "idle", false);
    let offset = TIMES[2] + 1.0;
    for run in [Run::Pan, Run::Idle] {
        s.request(run);
        let reports: Vec<Value> = TIMES
            .iter()
            .map(|&t| {
                s.step(offset + t);
                s.report()
            })
            .collect();
        check_reports(&reports, &s, run.name(), true);
    }
    // The browser's last run was the button's: its finished report still says so.
    assert_eq!(s.report()["not_a_measurement"], Value::Bool(true));

    let mut n = native(Some("idle"));
    drive(&mut n, Some(60.0));
    n.request(Run::Pan);
    let reports: Vec<Value> = TIMES
        .iter()
        .map(|&t| {
            let step = n.step(offset + t);
            n.watch_display(step, None, &|| Some(60.0));
            n.report()
        })
        .collect();
    check_reports(&reports, &n, "pan", false);
}

fn run_frame(ctx: &egui::Context, measured: &mut Measured<MockSide>, time: f64) {
    let mut raw = egui::RawInput {
        screen_rect: Some(Rect::from_min_max(pos2(0.0, 0.0), pos2(1740.0, 900.0))),
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

/// Through the app's frame, as the browser runs it (the panel with no monitor to read): the button's run is marked in
/// the session's report as it starts, and a `?run=` run, which starts as the panel opens, is not; the first report the
/// sink is told for the `?run=` run carries the field, unmarked.
#[test]
fn qa_m8_121_browser_frame_path() {
    let app = || App::new(MockSide::new(MockEngine::new(), None), FORMAT);
    let mark = |m: &Measured<MockSide>| {
        m.session.as_ref().expect("open").report()["not_a_measurement"].clone()
    };

    let ctx = egui::Context::default();
    let mut m = Measured::new(app(), Some(browser(None)), Box::new(|_| {}));
    run_frame(&ctx, &mut m, 0.0);
    assert_ne!(mark(&m), Value::Bool(true), "no run yet, nothing marked");
    m.session.as_mut().expect("open").request(Run::Idle);
    run_frame(&ctx, &mut m, 0.1);
    assert!(
        m.session.as_ref().expect("open").running(),
        "the button's run started"
    );
    assert_eq!(
        mark(&m),
        Value::Bool(true),
        "the button's browser run is marked"
    );

    let ctx = egui::Context::default();
    let reports = std::rc::Rc::new(std::cell::RefCell::new(Vec::<String>::new()));
    let sink = reports.clone();
    let mut f = Measured::new(
        app(),
        Some(browser(Some("idle"))),
        Box::new(move |json| sink.borrow_mut().push(json)),
    );
    run_frame(&ctx, &mut f, 0.0);
    assert!(
        f.session.as_ref().expect("open").running(),
        "the ?run= run started"
    );
    assert_eq!(mark(&f), Value::Bool(false), "a ?run= run is unmarked");
    let first: Value = serde_json::from_str(&reports.borrow()[0]).expect("the report's JSON");
    assert_eq!(first["status"], "running");
    assert_eq!(
        first["not_a_measurement"],
        Value::Bool(false),
        "the sink's report"
    );
    rejects("the ?run= run's report read as marked", || {
        assert_eq!(first["not_a_measurement"], Value::Bool(true))
    });
}
