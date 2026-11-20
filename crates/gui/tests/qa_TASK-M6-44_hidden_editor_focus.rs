//! QA test for TASK-M6-44, from R-463 K2 and `Out::focus`'s contract ("the scope the focus goes back to, off a basis
//! editor the console has hidden"): the console hiding an open basis editor moves a focus off the editor's hidden
//! fields, and only that. A focus anywhere else stays where it is, frame after frame: on the console, where the keys
//! are working while the editor stays asked for, on another control of the Chart section, or on the edit button itself.
//! The control (R-176): in the same state, a focus on a hidden field does go back to the button.
// The file name gives a crate name that is not snake case.
#![allow(non_snake_case)]

use eframe::egui::{pos2, Event, Key, Modifiers, PointerButton, Rect};
use engine::contract::view_ui::{KeyMode, Window};
use gui::app::App;
use gui::capture::{click, FORMAT, PIXELS_PER_POINT, SIZE};
use gui::explore::footer::FOOTER;
use gui::explore::manifold_view::{PRESET, Q1_COMPONENTS, Q1_EDIT};
use gui::headless::Headless;
use gui::mock::MockEngine;
use gui::side::MockSide;
use gui::windows::console::{HEADER, WINDOW};

type Rig = (App<MockSide>, Headless);

fn settle(h: &mut Headless, app: &mut App<MockSide>) {
    for _ in 0..6 {
        let _ = h.frame(app, Vec::new());
    }
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

fn path(app: &App<MockSide>, id: &str) -> Vec<String> {
    app.keyboard.tree(app.view.mode).path_to(id)
}

/// The focus on `id`, in navigation mode, a frame on.
fn at(h: &mut Headless, app: &mut App<MockSide>, id: &str) {
    app.view.focus.path = path(app, id);
    app.view.focus.mode = KeyMode::Navigation;
    let _ = h.frame(app, Vec::new());
}

fn console(app: &App<MockSide>) -> Rect {
    app.keyboard
        .place_of(WINDOW)
        .expect("the console drawn")
        .rect
}

/// The console's top edge dragged to `y`.
fn top_to(h: &mut Headless, app: &mut App<MockSide>, y: f32) {
    let from = pos2(200.0, console(app).min.y);
    let to = pos2(200.0, y);
    let button = |pos, pressed| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
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

/// A GUI on the mock, its console open, `q₁`'s editor asked for by a click, and the console over part of the editor,
/// which is so no longer drawn.
fn hidden() -> Rig {
    let mut app = App::new(MockSide::new(MockEngine::new(), None), FORMAT);
    let mut h = Headless::new(SIZE, PIXELS_PER_POINT);
    settle(&mut h, &mut app);
    app.view.focus.path.clear();
    press(&mut h, &mut app, Key::Tab, Modifiers::SHIFT);
    assert_eq!(app.view.focus.path, [FOOTER]);
    press(&mut h, &mut app, Key::Enter, Modifiers::NONE);
    settle(&mut h, &mut app);
    assert!(app.is_open(Window::Console));
    let button = app.keyboard.place_of(Q1_EDIT).expect("drawn").rect;
    for events in click(button.center()) {
        let _ = h.frame(&mut app, events);
    }
    settle(&mut h, &mut app);
    let field = app.keyboard.place_of(Q1_COMPONENTS[0]).expect("drawn").rect;
    top_to(&mut h, &mut app, field.max.y + 2.0);
    assert!(app.keyboard.place_of(Q1_COMPONENTS[0]).is_none(), "hidden");
    assert!(app.manifold.editing[0], "it stays asked for");
    (app, h)
}

#[test]
fn a_focus_on_the_hidden_field_goes_back_to_the_button() {
    let (mut app, mut h) = hidden();
    at(&mut h, &mut app, Q1_COMPONENTS[0]);
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(app.view.focus.path, path(&app, Q1_EDIT));
}

#[test]
fn a_focus_elsewhere_stays_where_it_is() {
    for id in [HEADER, PRESET, Q1_EDIT] {
        let (mut app, mut h) = hidden();
        at(&mut h, &mut app, id);
        settle(&mut h, &mut app);
        assert_eq!(app.view.focus.path, path(&app, id), "{id}");
    }
}
