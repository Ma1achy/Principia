//! QA tests for TASK-M6-44, from R-463 K1 ("docked ... over the bottom row") and K2 ("Maximised, it covers the panels and
//! the figure until toggled back"): the compass under the console, whose cube takes a drag and whose column now scrolls by
//! the wheel, takes neither through it. Each test runs its control (R-176): the same input with the console closed acts.
// The file name gives a crate name that is not snake case.
#![allow(non_snake_case)]

use eframe::egui::{
    pos2, vec2, Event, FullOutput, Key, Modifiers, MouseWheelUnit, PointerButton, Pos2, Rect,
    TouchPhase,
};
use engine::contract::sim_config::Plane;
use gui::app::App;
use gui::capture::{PIXELS_PER_POINT, SIZE};
use gui::explore::{compass, COMPASS_CUBE};
use gui::headless::Headless;
use gui::layout::{Layout, Splitter};
use gui::mock::MockEngine;
use gui::side::MockSide;
use gui::windows::console;

const NONE: Modifiers = Modifiers::NONE;

type Rig = (App<MockSide>, Headless);

fn settled(h: &mut Headless, app: &mut App<MockSide>) -> FullOutput {
    for _ in 0..60 {
        let _ = h.frame(app, Vec::new());
    }
    h.frame(app, Vec::new())
}

fn rig() -> Rig {
    let mut app = App::new(
        MockSide::new(MockEngine::frozen(), None),
        gui::capture::FORMAT,
    );
    let mut h = Headless::new(SIZE, PIXELS_PER_POINT);
    let _ = settled(&mut h, &mut app);
    (app, h)
}

fn press(h: &mut Headless, app: &mut App<MockSide>, key: Key, modifiers: Modifiers) {
    for pressed in [true, false] {
        let _ = h.frame(
            app,
            vec![Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers,
            }],
        );
    }
}

/// Opens the console from the keys, landing on its header bar; `maximised`, M then fills the height.
fn open(h: &mut Headless, app: &mut App<MockSide>, maximised: bool) {
    app.view.focus.path.clear();
    press(h, app, Key::Tab, Modifiers::SHIFT);
    press(h, app, Key::Enter, NONE);
    if maximised {
        press(h, app, Key::M, NONE);
    }
    let _ = settled(h, app);
    assert!(app.keyboard.place_of(console::WINDOW).is_some(), "not open");
    assert_eq!(app.windows.console.maximised, maximised);
}

fn console_rect(app: &App<MockSide>) -> Rect {
    app.keyboard
        .place_of(console::WINDOW)
        .expect("the console")
        .rect
}

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: NONE,
    }
}

fn drag(h: &mut Headless, app: &mut App<MockSide>, from: Pos2, by: eframe::egui::Vec2) {
    let _ = h.frame(app, vec![Event::PointerMoved(from)]);
    let _ = h.frame(app, vec![button(from, true)]);
    for k in 1..=4 {
        let _ = h.frame(app, vec![Event::PointerMoved(from + by * (k as f32 / 4.0))]);
    }
    let _ = h.frame(app, vec![button(from + by, false)]);
    let _ = h.frame(app, vec![Event::PointerGone]);
    let _ = settled(h, app);
}

fn plane(app: &App<MockSide>) -> Plane {
    app.snapshot().sim.plane.clone()
}

fn cube_drag_moves_the_plane(cover: Option<bool>) -> bool {
    let (mut app, mut h) = rig();
    let panel = Layout::with_splits(h.screen(), PIXELS_PER_POINT, &app.splits).compass;
    let cube = compass::square(panel).center();
    if let Some(maximised) = cover {
        open(&mut h, &mut app, maximised);
        assert!(console_rect(&app).contains(cube), "not over the cube");
    }
    let before = plane(&app);
    drag(&mut h, &mut app, cube, vec2(30.0, 12.0));
    plane(&app) != before
}

#[test]
fn qa_m6_44_the_compass_cube_takes_no_drag_through_the_console() {
    assert!(
        cube_drag_moves_the_plane(None),
        "control: with the console closed the drag tilts"
    );
    assert!(
        !cube_drag_moves_the_plane(Some(false)),
        "tilted through the docked console"
    );
    assert!(
        !cube_drag_moves_the_plane(Some(true)),
        "tilted through the maximised console"
    );
}

/// Where the column's Cube button is: a widget in the column, so it moves with its scroll.
fn column_y(app: &App<MockSide>) -> f32 {
    app.keyboard
        .place_of(COMPASS_CUBE)
        .expect("the column's Cube button")
        .rect
        .min
        .y
}

/// How far a wheel over the compass, the panels at their least so its column overflows, moves the column.
fn wheel_moves_the_column(cover: Option<bool>) -> f32 {
    let (mut app, mut h) = rig();
    // The panels at their least: the handles dragged past their ends.
    let l = Layout::with_splits(h.screen(), PIXELS_PER_POINT, &app.splits);
    drag(
        &mut h,
        &mut app,
        l.handle(Splitter::CompassTime).center(),
        vec2(-400.0, 0.0),
    );
    let l = Layout::with_splits(h.screen(), PIXELS_PER_POINT, &app.splits);
    drag(
        &mut h,
        &mut app,
        l.handle(Splitter::TopBottom).center(),
        vec2(0.0, 400.0),
    );
    let panel = Layout::with_splits(h.screen(), PIXELS_PER_POINT, &app.splits).compass;
    let over = pos2(panel.max.x - 30.0, panel.max.y - 30.0);
    if let Some(maximised) = cover {
        open(&mut h, &mut app, maximised);
        assert!(console_rect(&app).contains(over), "not over the column");
    }
    let _ = settled(&mut h, &mut app);
    let before = column_y(&app);
    let _ = h.frame(&mut app, vec![Event::PointerMoved(over)]);
    let _ = h.frame(
        &mut app,
        vec![Event::MouseWheel {
            unit: MouseWheelUnit::Point,
            delta: vec2(0.0, -40.0),
            phase: TouchPhase::Move,
            modifiers: NONE,
        }],
    );
    let _ = settled(&mut h, &mut app);
    (column_y(&app) - before).abs()
}

#[test]
fn qa_m6_44_the_compass_column_takes_no_wheel_through_the_console() {
    assert!(
        wheel_moves_the_column(None) > 0.0,
        "control: with the console closed the wheel scrolls the column"
    );
    assert_eq!(
        wheel_moves_the_column(Some(false)),
        0.0,
        "scrolled through the docked console"
    );
    assert_eq!(
        wheel_moves_the_column(Some(true)),
        0.0,
        "scrolled through the maximised console"
    );
}
