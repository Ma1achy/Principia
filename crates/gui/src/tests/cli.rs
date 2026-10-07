//! The binary's commands on the mock (REQ-GUI-165; R-274): with no argument the window opens through eframe, titled
//! for the mock, egui-wgpu built from the mock's device and queue (RQ-247, RQ-248); `capture` writes the capture and
//! its names; anything else is refused, named. The window itself is opened by a test runner in place of eframe's.

use std::cell::RefCell;

use eframe::egui_wgpu::WgpuSetup;

use super::support::rejects;
use crate::cli::mock::{dispatch, WINDOW_SIZE};
use crate::cli::{run, NO_ENGINE};

/// What a runner was given: the app name, the window's title and size, and whether its wgpu setup is the engine
/// side's existing device.
type Seen = (String, Option<String>, Option<eframe::egui::Vec2>, bool);

/// Dispatches `args`, recording what the window's runner is given; the runner returns `result`.
fn dispatch_seen(args: &[&str], result: eframe::Result) -> (Result<(), String>, Option<Seen>) {
    let seen = RefCell::new(None);
    let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    let outcome = dispatch(&args, |name, options, _creator| {
        let existing = matches!(options.wgpu_options.wgpu_setup, WgpuSetup::Existing(_));
        assert_eq!(options.renderer, eframe::Renderer::Wgpu);
        *seen.borrow_mut() = Some((
            name.to_owned(),
            options.viewport.title.clone(),
            options.viewport.inner_size,
            existing,
        ));
        result
    });
    (outcome, seen.into_inner())
}

/// No argument opens the window, titled for the mock, at its size, on the mock's device.
fn check_window(args: &[&str]) {
    let (outcome, seen) = dispatch_seen(args, Ok(()));
    assert_eq!(outcome, Ok(()));
    let title = "principia · dev — mock engine";
    assert_eq!(
        seen,
        Some((
            title.to_owned(),
            Some(title.to_owned()),
            Some(WINDOW_SIZE.into()),
            true
        )),
        "the window was not opened as the mock's"
    );
}

#[test]
fn cli_opens_the_window_on_the_mock() {
    check_window(&[]);
    rejects("a command that opens no window", || {
        check_window(&["bogus"])
    });
    let (outcome, _) = dispatch_seen(&[], Err(eframe::Error::AppCreation("no display".into())));
    let err = outcome.expect_err("the runner's error was lost");
    assert!(
        err.starts_with("gui: ") && err.contains("no display"),
        "{err}"
    );
}

#[test]
fn cli_refuses_an_unknown_command() {
    let (outcome, seen) = dispatch_seen(&["bogus"], Ok(()));
    let err = outcome.expect_err("an unknown command ran");
    assert!(err.contains("unknown command `bogus`"), "{err}");
    assert!(seen.is_none(), "an unknown command opened the window");
    let (outcome, _) = dispatch_seen(&["capture", "--screen", "09_nope", "--out", "x"], Ok(()));
    assert!(outcome
        .expect_err("an unknown screen")
        .contains("no screen `09_nope`"));
}

#[test]
fn cli_capture_writes_the_capture_and_names() {
    let out = std::env::temp_dir().join(format!("gui-cli-capture-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    let out_arg = out.to_string_lossy().into_owned();
    let (outcome, seen) = dispatch_seen(
        &[
            "capture",
            "--screen",
            "01_main",
            "--steps",
            "raise_warning",
            "--out",
            &out_arg,
        ],
        Ok(()),
    );
    assert_eq!(outcome, Ok(()));
    assert!(seen.is_none(), "capture opened a window");
    let names = std::fs::read_to_string(out.join("names.json")).expect("names written");
    assert!(
        names.contains("⚠ 1 · × 0 · console ▴"),
        "the step was not played: {names}"
    );
    assert!(out.join("capture.png").is_file());
    std::fs::remove_dir_all(&out).expect("scratch removed");
}

/// Without the `mock` feature the binary says the real engine has no window yet; with it, `run` dispatches.
#[test]
fn cli_run_without_the_mock_feature_says_so() {
    let outcome = run(&["bogus".to_owned()]);
    if cfg!(feature = "mock") {
        assert!(outcome.expect_err("bogus ran").contains("unknown command"));
    } else {
        assert_eq!(outcome, Err(NO_ENGINE.to_owned()));
    }
}

/// One pass of `ctx` running `pass`: the number of shapes it painted.
fn shapes_painted(pass: impl FnMut(&mut eframe::egui::Ui)) -> usize {
    let ctx = eframe::egui::Context::default();
    let mut output = ctx.run_ui(Default::default(), pass);
    output.textures_delta.clear();
    output.shapes.len()
}

#[test]
fn cli_eframe_draws_the_app_through_its_ui() {
    let mut app = super::support::mock_app();
    let mut frame = eframe::Frame::_new_kittest();
    let painted = shapes_painted(|ui| eframe::App::ui(&mut app, ui, &mut frame));
    assert!(painted > 0, "eframe's call drew nothing");
    rejects("a pass that draws nothing", || {
        assert!(shapes_painted(|_| {}) > 0, "drew nothing")
    });
}
