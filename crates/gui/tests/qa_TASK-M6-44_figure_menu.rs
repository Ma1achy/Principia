//! QA tests for TASK-M6-44, from R-463 K2 ("the panels under it cannot be reached") as the figure stays in use under a
//! risen console: its right-click menu goes when the console reaches the point it was opened on, whatever raised the
//! console, and not before. A console opening by itself shuts the menu of a point it covers and leaves one it does not,
//! an entry of a menu left hanging over a console risen after it was opened locks that point, and a console that opened
//! by itself and is then maximised covers the panels as any other. Each test runs both arms (R-176), so neither
//! "always" nor "never" passes.
// The file name gives a crate name that is not snake case.
#![allow(non_snake_case)]

use eframe::egui::{self, pos2, vec2, Event, Key, Modifiers, PointerButton, Pos2, Rect};
use engine::contract::view_ui::{KeyMode, Window};
use gui::app::App;
use gui::capture::{click, FORMAT, PIXELS_PER_POINT, SIZE};
use gui::explore::footer::FOOTER;
use gui::explore::manifold_view::navigate::figure_coords;
use gui::explore::manifold_view::point;
use gui::explore::FIGURE;
use gui::headless::Headless;
use gui::layout::Layout;
use gui::mock::MockEngine;
use gui::side::{EngineSide, MockSide};
use gui::windows::console::{HEADER, WINDOW};

const NONE: Modifiers = Modifiers::NONE;

type Rig = (App<MockSide>, Headless);

fn rig() -> Rig {
    let mut app = App::new(MockSide::new(MockEngine::new(), None), FORMAT);
    let mut h = Headless::new(SIZE, PIXELS_PER_POINT);
    settle(&mut h, &mut app);
    (app, h)
}

fn settle(h: &mut Headless, app: &mut App<MockSide>) {
    for _ in 0..6 {
        let _ = h.frame(app, Vec::new());
    }
}

fn press(h: &mut Headless, app: &mut App<MockSide>, key: Key, modifiers: Modifiers) {
    for pressed in [true, false] {
        let event = Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers,
        };
        let _ = h.frame(app, vec![event]);
    }
}

/// A GUI on the mock, its console opened by Enter on the footer.
fn opened() -> Rig {
    let (mut app, mut h) = rig();
    app.view.focus.path.clear();
    press(&mut h, &mut app, Key::Tab, Modifiers::SHIFT);
    assert_eq!(app.view.focus.path, [FOOTER]);
    press(&mut h, &mut app, Key::Enter, NONE);
    settle(&mut h, &mut app);
    assert!(app.is_open(Window::Console));
    (app, h)
}

fn console(app: &App<MockSide>) -> Rect {
    app.keyboard
        .place_of(WINDOW)
        .expect("the console drawn")
        .rect
}

fn figure(app: &App<MockSide>) -> Rect {
    app.keyboard.place_of(FIGURE).expect("drawn").rect
}

fn right_click(h: &mut Headless, app: &mut App<MockSide>, at: Pos2) {
    let button = |pressed| Event::PointerButton {
        pos: at,
        button: PointerButton::Secondary,
        pressed,
        modifiers: NONE,
    };
    for events in [
        vec![Event::PointerMoved(at)],
        vec![button(true)],
        vec![button(false)],
    ] {
        let _ = h.frame(app, events);
    }
    settle(h, app);
}

/// The console's top edge dragged to `y`, by the pointer.
fn top_to(h: &mut Headless, app: &mut App<MockSide>, y: f32) {
    let from = pos2(200.0, console(app).min.y);
    let to = pos2(200.0, y);
    let button = |pos, pressed| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: NONE,
    };
    for events in [
        vec![Event::PointerMoved(from)],
        vec![button(from, true)],
        vec![Event::PointerMoved(pos2(200.0, (from.y + y) / 2.0))],
        vec![Event::PointerMoved(to)],
        vec![button(to, false)],
    ] {
        let _ = h.frame(app, events);
    }
    settle(h, app);
}

#[test]
fn a_console_opening_by_itself_shuts_the_figure_menu_of_a_point_it_reaches_only() {
    for (offset, stays) in [(-30.0, true), (30.0, false)] {
        // Docked at its default the console stops under the figure, so it is dragged up on to it and shut.
        let (mut app, mut h) = opened();
        let shown = figure(&app);
        let docked = shown.center().y;
        top_to(&mut h, &mut app, docked);
        app.view.focus.path = app.keyboard.tree(app.view.mode).path_to(HEADER);
        app.view.focus.mode = KeyMode::Navigation;
        let _ = h.frame(&mut app, Vec::new());
        press(&mut h, &mut app, Key::Escape, NONE);
        settle(&mut h, &mut app);
        assert!(!app.is_open(Window::Console), "shut");
        let at = pos2(shown.center().x, docked + offset);
        right_click(&mut h, &mut app, at);
        assert!(egui::Popup::is_any_open(&h.ctx), "the menu opens");

        app.side().engine().raise_error();
        settle(&mut h, &mut app);
        assert!(app.is_open(Window::Console), "the error opened it");
        near(console(&app).min.y, docked);
        assert_eq!(egui::Popup::is_any_open(&h.ctx), stays, "at {offset}");
        for _ in 0..4 {
            let last = h.frame(&mut app, Vec::new());
            assert_eq!(
                last.platform_output.num_completed_passes, 1,
                "at {offset}: shut again frame after frame"
            );
        }
    }
}

fn near(a: f32, b: f32) {
    assert!((a - b).abs() < 1.0, "{a} against {b}");
}

#[test]
fn a_menu_left_over_a_console_risen_after_it_locks_its_point_and_one_reached_does_not() {
    for (rise, locks) in [(4.0, true), (-20.0, false)] {
        let (mut app, mut h) = opened();
        let layout = Layout::with_splits(h.screen(), h.pixels_per_point(), &app.splits);
        let at = figure(&app).center_top() + vec2(0.0, 60.0);
        right_click(&mut h, &mut app, at);
        assert!(egui::Popup::is_any_open(&h.ctx), "the menu opens");
        let plane = app.snapshot().sim.plane.clone();
        let (s, t) = figure_coords(layout.field, at);

        top_to(&mut h, &mut app, at.y + rise);
        near(console(&app).min.y, at.y + rise);
        assert_eq!(egui::Popup::is_any_open(&h.ctx), locks, "at {rise}");

        // The entry the menu's first row holds; reached, the same place is the console's.
        for events in click(at + vec2(10.0, 8.0)) {
            let _ = h.frame(&mut app, events);
        }
        settle(&mut h, &mut app);
        let lock = app.snapshot().sim.lock.clone();
        assert_eq!(lock.locked, locks, "at {rise}");
        if locks {
            assert_eq!(lock.z_locked, point(&plane, s, t), "that point");
        }
    }
}

#[test]
fn a_console_that_opened_by_itself_covers_the_panels_once_it_is_maximised() {
    // The control is the first arm: docked, the same console leaves a focus on the figure where it is.
    let (mut app, mut h) = rig();
    app.side().engine().raise_error();
    settle(&mut h, &mut app);
    assert!(app.is_open(Window::Console), "the error opened it");
    app.view.focus.path = vec![FIGURE.to_owned()];
    app.view.focus.mode = KeyMode::Navigation;
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(
        app.view.focus.path,
        [FIGURE],
        "not covered: the focus stays"
    );

    app.view.focus.path = app.keyboard.tree(app.view.mode).path_to(HEADER);
    let _ = h.frame(&mut app, Vec::new());
    press(&mut h, &mut app, Key::M, NONE);
    settle(&mut h, &mut app);
    assert!(app.windows.console.maximised, "M maximises it");
    app.view.focus.path = vec![FIGURE.to_owned()];
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(
        app.view.focus.path[0], WINDOW,
        "covered, the focus goes to the console"
    );
}
