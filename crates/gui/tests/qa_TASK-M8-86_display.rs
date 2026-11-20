//! QA tests for TASK-M8-86, written from the deliverable "natively, the monitor's rate as winit reports it" and the panel
//! deliverable's "reports the same fields": the native window has no rate until its first frame reads one, so the report
//! the panel sends as it opens must already carry the rate, with a flag's run or without. gui takes no dependency on
//! validation (R-187), so each check runs its own control (R-176) through [`rejects`].
// The file name `qa_TASK-M8-86_display` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::cell::RefCell;
use std::rc::Rc;

use eframe::egui::{self, pos2, Rect, ViewportId};
use gui::app::App;
use gui::capture::FORMAT;
use gui::measure::{Environment, Measured, RateRead, Session};
use gui::mock::MockEngine;
use gui::side::MockSide;

fn rejects(what: &str, check: impl FnOnce()) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check));
    assert!(
        outcome.is_err(),
        "the check passed on {what}, which it must reject"
    );
}

fn mock_app() -> App<MockSide> {
    App::new(MockSide::new(MockEngine::new(), None), FORMAT)
}

type Drive = dyn Fn(&mut Measured<MockSide>, &mut egui::Ui, &mut eframe::Frame);

/// The reports of a native panel's first frame through `drive`, opened as the binary opens it, with no rate yet.
fn first_reports(run: Option<&str>, rate: RateRead, drive: &Drive) -> Vec<serde_json::Value> {
    let env = Environment::native(Some("1"), None, run, None)
        .expect("flags")
        .expect("open");
    let reports = Rc::new(RefCell::new(Vec::new()));
    let sink = reports.clone();
    let mut measured = Measured::new(
        mock_app(),
        Some(Session::new(env)),
        Box::new(move |json: String| sink.borrow_mut().push(json)),
    )
    .reading_rate(rate);
    let ctx = egui::Context::default();
    let mut raw = egui::RawInput {
        screen_rect: Some(Rect::from_min_max(pos2(0.0, 0.0), pos2(1740.0, 900.0))),
        ..Default::default()
    };
    raw.viewports
        .entry(ViewportId::ROOT)
        .or_default()
        .native_pixels_per_point = Some(1.0);
    measured.raw_input_hook(&ctx, &mut raw);
    let mut frame = eframe::Frame::_new_kittest();
    ctx.run_ui(raw, |ui| drive(&mut measured, ui, &mut frame))
        .textures_delta
        .clear();
    let reports = reports.borrow();
    reports
        .iter()
        .map(|json| serde_json::from_str(json).expect("the report's JSON"))
        .collect()
}

fn check_opening_rate(drive: &Drive) {
    let opened = first_reports(None, Box::new(|_| Some(120.0)), drive);
    assert_eq!(opened.len(), 1, "one report as the panel opens");
    assert_eq!(opened[0]["refresh_hz"], 120.0, "the rate as it opens");
    assert_eq!(opened[0]["status"], "ready");
    assert!(opened[0]["display_changed"].is_null(), "no run yet");

    let run = first_reports(Some("idle"), Box::new(|_| Some(120.0)), drive);
    assert_eq!(run[0]["refresh_hz"], 120.0, "the rate a flag's run reads");
    assert_eq!(run[0]["status"], "running");
    assert_eq!(run[0]["display_changed"], false, "a run under watch");
}

#[test]
fn qa_m8_86_the_opening_report_carries_the_windows_rate() {
    check_opening_rate(&|measured, ui, frame| eframe::App::ui(measured, ui, frame));
    rejects("a frame that reads no rate", || {
        check_opening_rate(&|measured, ui, _| measured.ui(ui));
    });
}

/// A window whose monitor gives no rate says so: no invented figure, and nothing breaks.
#[test]
fn qa_m8_86_a_monitor_with_no_rate_reports_none() {
    let drive: &Drive = &|measured, ui, frame| eframe::App::ui(measured, ui, frame);
    let opened = first_reports(None, Box::new(|_| None), drive);
    assert!(opened[0]["refresh_hz"].is_null(), "{}", opened[0]);
    rejects("a monitor that gives one", || {
        let opened = first_reports(None, Box::new(|_| Some(60.0)), drive);
        assert!(opened[0]["refresh_hz"].is_null());
    });
}
