//! QA tests for TASK-M6-44, from R-463 K2 ("Maximised, it covers the panels and the figure until toggled back") and
//! REQ-GUI-216's "open and close each within one frame": the figure and its lock's reticle give way to the console in
//! the very frame of the control's input, for the mouse's controls as for the keys. Each test has its control (R-176).

use eframe::egui::{Event, FullOutput, Key, Modifiers, PointerButton, Pos2, Rect, Shape};
use engine::contract::view_ui::Window;
use gui::app::App;
use gui::capture::{FORMAT, PIXELS_PER_POINT, SIZE};
use gui::cli::mock::canvas_flagging;
use gui::explore::lock::GOLD;
use gui::headless::Headless;
use gui::layout::Layout;
use gui::mock::MockEngine;
use gui::side::MockSide;
use gui::windows::console;

type Rig = (App<MockSide>, Headless);

fn rejects(what: &str, check: impl FnOnce()) {
    let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check)).is_err();
    assert!(failed, "the control passed: {what}");
}

fn rig() -> Rig {
    let canvas = canvas_flagging(None).expect("the mock's device");
    let mut app = App::new(MockSide::new(MockEngine::frozen(), Some(canvas)), FORMAT);
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

fn press(h: &mut Headless, app: &mut App<MockSide>, k: Key) -> FullOutput {
    let out = h.frame(app, vec![key(k, true, Modifiers::NONE)]);
    let _ = h.frame(app, vec![key(k, false, Modifiers::NONE)]);
    out
}

/// A click at `at`, returning the frame of its release: the frame in which it acts.
fn click(h: &mut Headless, app: &mut App<MockSide>, at: Pos2) -> FullOutput {
    let button = |pressed| Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    let _ = h.frame(app, vec![Event::PointerMoved(at)]);
    let _ = h.frame(app, vec![button(true)]);
    h.frame(app, vec![button(false)])
}

fn open(h: &mut Headless, app: &mut App<MockSide>) {
    let _ = h.frame(app, vec![key(Key::Tab, true, Modifiers::SHIFT)]);
    let _ = h.frame(app, vec![key(Key::Tab, false, Modifiers::SHIFT)]);
    let _ = press(h, app, Key::Enter);
    settle(h, app);
    assert!(app.is_open(Window::Console), "opened");
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

fn whole(h: &Headless, app: &App<MockSide>, clip: Rect) {
    let layout = Layout::with_splits(h.screen(), PIXELS_PER_POINT, &app.splits);
    assert_eq!(clip, layout.figure);
}

fn place(app: &App<MockSide>, id: &str) -> Rect {
    app.keyboard.place_of(id).expect("placed").rect
}

/// Whether `output` draws the lock's reticle ring, its gold circle.
fn reticle(output: &FullOutput) -> bool {
    fn find(shape: &Shape) -> bool {
        match shape {
            Shape::Vec(v) => v.iter().any(find),
            Shape::Circle(c) => c.stroke.color == GOLD,
            _ => false,
        }
    }
    output.shapes.iter().any(|s| find(&s.shape))
}

#[test]
fn qa_m6_44_a_click_on_maximise_moves_the_figures_clip_with_the_console_in_its_frame() {
    let (mut app, mut h) = rig();
    open(&mut h, &mut app);
    let at = place(&app, console::MAXIMISE).center();
    let out = click(&mut h, &mut app, at);
    assert!(app.windows.console.maximised, "the click maximised it");
    covered(figure_clip(&out));
    // What the same frame drew of the console agrees with the clip: no strip left uncovered by both.
    let layout = Layout::with_splits(h.screen(), PIXELS_PER_POINT, &app.splits);
    let top = place(&app, console::WINDOW).min.y;
    assert!(
        top <= layout.figure.min.y,
        "the clip gave way, the console at {top} did not reach {}",
        layout.figure.min.y
    );
    settle(&mut h, &mut app);
    let at = place(&app, console::MAXIMISE).center();
    let out = click(&mut h, &mut app, at);
    assert!(!app.windows.console.maximised, "the click restored it");
    whole(&h, &app, figure_clip(&out));
    rejects("a figure drawn whole under a maximised console", || {
        covered(figure_clip(&out))
    });
}

#[test]
fn qa_m6_44_a_click_on_the_close_gives_the_figure_back_in_its_frame() {
    let (mut app, mut h) = rig();
    open(&mut h, &mut app);
    let _ = press(&mut h, &mut app, Key::M);
    settle(&mut h, &mut app);
    assert!(app.windows.console.maximised, "M maximised it");
    covered(figure_clip(&h.frame(&mut app, Vec::new())));
    let at = place(&app, console::CLOSE).center();
    let out = click(&mut h, &mut app, at);
    assert!(!app.is_open(Window::Console), "the click closed it");
    whole(&h, &app, figure_clip(&out));
    rejects("the clip of the frame before", || {
        covered(figure_clip(&out))
    });
}

#[test]
fn qa_m6_44_the_reticle_goes_and_returns_in_the_frame_of_m() {
    let (mut app, mut h) = rig();
    let _ = press(&mut h, &mut app, Key::Tab);
    let _ = press(&mut h, &mut app, Key::Tab);
    let _ = press(&mut h, &mut app, Key::Tab);
    let figure = Layout::with_splits(h.screen(), PIXELS_PER_POINT, &app.splits).figure;
    let _ = h.frame(&mut app, vec![Event::PointerMoved(figure.center())]);
    settle(&mut h, &mut app);
    let _ = press(&mut h, &mut app, Key::K);
    settle(&mut h, &mut app);
    assert!(app.snapshot().sim.lock.locked, "K locks");
    let shown = h.frame(&mut app, Vec::new());
    assert!(reticle(&shown), "no reticle to cover");
    app.view.focus.path.clear();
    open(&mut h, &mut app);
    let maximised = press(&mut h, &mut app, Key::M);
    assert!(app.windows.console.maximised);
    assert!(!reticle(&maximised), "the reticle in M's frame");
    settle(&mut h, &mut app);
    let restored = press(&mut h, &mut app, Key::M);
    assert!(!app.windows.console.maximised);
    assert!(reticle(&restored), "no reticle in M's restoring frame");
    rejects("a reticle under the maximised console", || {
        assert!(reticle(&maximised))
    });
}
