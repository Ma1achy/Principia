//! QA tests for TASK-M6-44, from R-463 K2 ("Maximised, it covers the panels and the figure until toggled back") read
//! with R-468's fill modes (render_gui_spec §G1: egui is a layer F3 hides, and View › F3 fills by: Extend / Fit). The
//! console is part of the egui layer, so F3 takes it away with the layer: hidden in Fit, the figure is drawn whole in
//! Fit's square, never clipped by a console no longer drawn; F3 again brings the layer back with the console still
//! maximised, covering the figure, since only its own toggle restores it. Each check has its control (R-176).

use eframe::egui::{Event, FullOutput, Key, Modifiers, Rect, Shape};
use engine::contract::view_ui::{F3Fill, Window};
use gui::app::App;
use gui::capture::{FORMAT, PIXELS_PER_POINT, SIZE};
use gui::cli::mock::canvas_flagging;
use gui::headless::Headless;
use gui::layout::Layout;
use gui::mock::MockEngine;
use gui::side::MockSide;

type Rig = (App<MockSide>, Headless);

fn rejects(what: &str, check: impl FnOnce()) {
    let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check)).is_err();
    assert!(failed, "the control passed: {what}");
}

fn rig(fill: F3Fill) -> Rig {
    let canvas = canvas_flagging(None).expect("the mock's device");
    let mut app = App::new(MockSide::new(MockEngine::frozen(), Some(canvas)), FORMAT);
    app.view.f3_fill = fill;
    let mut h = Headless::new(SIZE, PIXELS_PER_POINT);
    settle(&mut h, &mut app);
    (app, h)
}

fn settle(h: &mut Headless, app: &mut App<MockSide>) {
    for _ in 0..4 {
        let _ = h.frame(app, Vec::new());
    }
}

fn key(key: Key, pressed: bool, modifiers: Modifiers) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers,
    }
}

/// `k` pressed and released, returning the frame of the press: the frame in which it acts.
fn press(h: &mut Headless, app: &mut App<MockSide>, k: Key) -> FullOutput {
    let out = h.frame(app, vec![key(k, true, Modifiers::NONE)]);
    let _ = h.frame(app, vec![key(k, false, Modifiers::NONE)]);
    out
}

/// Opens the console from the keyboard (Shift+Tab to the bottom row's console entry, Enter) and maximises it with M.
fn open_maximised(h: &mut Headless, app: &mut App<MockSide>) {
    let _ = h.frame(app, vec![key(Key::Tab, true, Modifiers::SHIFT)]);
    let _ = h.frame(app, vec![key(Key::Tab, false, Modifiers::SHIFT)]);
    let _ = press(h, app, Key::Enter);
    settle(h, app);
    assert!(app.is_open(Window::Console), "opened");
    let _ = press(h, app, Key::M);
    settle(h, app);
    assert!(app.windows.console.maximised, "M maximised it");
}

/// The rect the figure's paint callback is clipped to in `output`.
fn figure_clip(output: &FullOutput) -> Rect {
    output
        .shapes
        .iter()
        .find(|s| matches!(s.shape, Shape::Callback(_)))
        .expect("the figure painted")
        .clip_rect
}

fn covered(clip: Rect) {
    assert!(clip.height() <= 0.0, "the figure shows: {clip:?}");
}

fn layout(h: &Headless, app: &App<MockSide>) -> Layout {
    Layout::with_splits(h.screen(), PIXELS_PER_POINT, &app.splits)
}

#[test]
fn qa_m6_44_f3_in_fit_draws_the_figure_whole_past_a_maximised_console_and_returns_it_covered() {
    let (mut app, mut h) = rig(F3Fill::Fit);
    open_maximised(&mut h, &mut app);
    covered(figure_clip(&h.frame(&mut app, Vec::new())));

    // F3 hides the layer, the console with it: the figure is Fit's square, whole, in the frame of the key.
    let hidden = figure_clip(&press(&mut h, &mut app, Key::F3));
    let square = layout(&h, &app).fit_rect(PIXELS_PER_POINT);
    assert_eq!(
        hidden, square,
        "hidden in Fit, the figure is Fit's square, unclipped"
    );
    rejects("a figure still clipped by the hidden console", || {
        covered(hidden)
    });
    settle(&mut h, &mut app);
    assert_eq!(
        figure_clip(&h.frame(&mut app, Vec::new())),
        square,
        "it stays whole while hidden"
    );

    // F3 again: the layer returns with the console still maximised; F3 is not its toggle.
    let back = figure_clip(&press(&mut h, &mut app, Key::F3));
    assert!(app.is_open(Window::Console), "still open");
    assert!(app.windows.console.maximised, "F3 left it maximised");
    covered(back);
    rejects("the shown figure in Fit's square", || {
        assert_eq!(back, square)
    });
}

#[test]
fn qa_m6_44_f3_in_extend_draws_the_window_past_a_maximised_console() {
    let (mut app, mut h) = rig(F3Fill::Extend);
    open_maximised(&mut h, &mut app);
    let hidden = figure_clip(&press(&mut h, &mut app, Key::F3));
    let window = layout(&h, &app).figure_rect(false);
    assert_eq!(
        hidden, window,
        "hidden in Extend, the figure fills the window"
    );
    rejects("a figure clipped to Fit's square in Extend", || {
        assert_eq!(hidden, layout(&h, &app).fit_rect(PIXELS_PER_POINT))
    });
    settle(&mut h, &mut app);
    let back = figure_clip(&press(&mut h, &mut app, Key::F3));
    assert!(app.windows.console.maximised, "F3 left it maximised");
    covered(back);
    rejects("the shown figure in the whole window", || {
        assert_eq!(back, window)
    });
}
