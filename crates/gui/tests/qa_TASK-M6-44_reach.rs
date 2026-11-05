//! QA tests for TASK-M6-44, from REQ-GUI-215 ("its height dragged ... and remembered with the layout"; maximise toggles
//! "between that height and the full height"), R-463 K2 and §G12: what the keys do at the console's reach. Enter on a
//! panel's tilt menu the console has reached leaves no popup over the console and the focus on the menu, without a pass
//! discarded frame after frame; an error opening a closed console keeps a dragged height, and a console closed maximised
//! opens lower with the toggle still between its height and the full one; Enter on the footer, the editor open under a
//! maximised console, never paints the editor over it. Each test runs its control (R-176): the same input where it acts.
// The file name gives a crate name that is not snake case.
#![allow(non_snake_case)]

use eframe::egui::{self, pos2, Event, FullOutput, Key, Modifiers, PointerButton, Rect, Shape};
use engine::contract::view_ui::{KeyMode, Mode, Window};
use gui::app::App;
use gui::capture::{click, FORMAT, PIXELS_PER_POINT, SIZE};
use gui::explore::footer::FOOTER;
use gui::explore::manifold_view::{AXES, Q1_COMPONENTS, Q1_EDIT, TILT_X, TILT_Y};
use gui::headless::Headless;
use gui::mock::MockEngine;
use gui::side::{EngineSide, MockSide};
use gui::windows::console::{HEADER, MAXIMISE_TEXT, WINDOW};

const NONE: Modifiers = Modifiers::NONE;

type Rig = (App<MockSide>, Headless);

fn settle(h: &mut Headless, app: &mut App<MockSide>) {
    for _ in 0..6 {
        let _ = h.frame(app, Vec::new());
    }
}

/// The two frames of a press and release of `key`.
fn press(
    h: &mut Headless,
    app: &mut App<MockSide>,
    key: Key,
    modifiers: Modifiers,
) -> Vec<FullOutput> {
    [true, false]
        .map(|pressed| {
            h.frame(
                app,
                vec![Event::Key {
                    key,
                    physical_key: None,
                    pressed,
                    repeat: false,
                    modifiers,
                }],
            )
        })
        .into()
}

/// The focus on `id`, in navigation mode, a frame on.
fn at(h: &mut Headless, app: &mut App<MockSide>, id: &str) {
    app.view.focus.path = app.keyboard.tree(app.view.mode).path_to(id);
    app.view.focus.mode = KeyMode::Navigation;
    let _ = h.frame(app, Vec::new());
}

/// The Enter that, after the pointer's drag turned keyboard navigation off, only turns it on (R-460 H1).
fn reveal(h: &mut Headless, app: &mut App<MockSide>) {
    assert!(!app.keyboard.navigating(), "the drag turned it off");
    let _ = press(h, app, Key::Enter, NONE);
    assert!(app.keyboard.navigating());
}

fn console(app: &App<MockSide>) -> Rect {
    app.keyboard
        .place_of(WINDOW)
        .expect("the console drawn")
        .rect
}

/// A GUI on the mock, its console opened by Enter on the footer.
fn opened() -> Rig {
    let mut app = App::new(MockSide::new(MockEngine::new(), None), FORMAT);
    let mut h = Headless::new(SIZE, PIXELS_PER_POINT);
    settle(&mut h, &mut app);
    app.view.focus.path.clear();
    let _ = press(&mut h, &mut app, Key::Tab, Modifiers::SHIFT);
    assert_eq!(app.view.focus.path, [FOOTER]);
    let _ = press(&mut h, &mut app, Key::Enter, NONE);
    settle(&mut h, &mut app);
    assert!(app.is_open(Window::Console));
    (app, h)
}

/// The console's top edge dragged to `y`.
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

fn close(h: &mut Headless, app: &mut App<MockSide>) {
    at(h, app, HEADER);
    let _ = press(h, app, Key::Escape, NONE);
    settle(h, app);
    assert!(!app.is_open(Window::Console));
}

fn near(a: f32, b: f32, what: &str) {
    assert!((a - b).abs() < 1.0, "{what}: {a} against {b}");
}

/// Whether a field's label is painted after the console's maximise button, if the console painted.
fn editor_over_it(output: &FullOutput) -> Option<bool> {
    fn flatten(shape: &Shape, out: &mut Vec<String>) {
        match shape {
            Shape::Vec(v) => v.iter().for_each(|s| flatten(s, out)),
            Shape::Text(t) => out.push(t.galley.text().to_owned()),
            _ => {}
        }
    }
    let mut text = Vec::new();
    output
        .shapes
        .iter()
        .for_each(|c| flatten(&c.shape, &mut text));
    let console = text.iter().position(|t| t == MAXIMISE_TEXT)?;
    Some(text[console..].iter().any(|t| AXES.contains(&t.as_str())))
}

#[test]
fn enter_on_a_menu_the_console_reached_leaves_it_shut_and_the_focus_on_it() {
    for id in [TILT_X, TILT_Y] {
        let (mut app, mut h) = opened();
        let control = app.keyboard.place_of(id).expect("drawn").rect;
        // The control: short of the combo, Enter opens it and Esc shuts it.
        top_to(&mut h, &mut app, control.max.y + 4.0);
        at(&mut h, &mut app, id);
        reveal(&mut h, &mut app);
        let _ = press(&mut h, &mut app, Key::Enter, NONE);
        settle(&mut h, &mut app);
        assert!(egui::Popup::is_any_open(&h.ctx), "{id}: Enter opens it");
        let _ = press(&mut h, &mut app, Key::Escape, NONE);
        settle(&mut h, &mut app);
        assert!(!egui::Popup::is_any_open(&h.ctx), "{id}: Esc shuts it");

        // Reached, and not over the whole page, so the keys still reach the combo.
        top_to(&mut h, &mut app, control.min.y - 4.0);
        at(&mut h, &mut app, id);
        let path = app.keyboard.tree(app.view.mode).path_to(id);
        assert_eq!(app.view.focus.path, path, "{id}: the keys reach it");
        reveal(&mut h, &mut app);
        let _ = press(&mut h, &mut app, Key::Enter, NONE);
        let _ = h.frame(&mut app, Vec::new());
        assert!(!egui::Popup::is_any_open(&h.ctx), "{id}: shut");
        assert_eq!(app.view.focus.path, path, "{id}: the focus on the menu");
        assert_eq!(app.view.focus.mode, KeyMode::Navigation);
        for _ in 0..4 {
            let last = h.frame(&mut app, Vec::new());
            assert_eq!(
                last.platform_output.num_completed_passes, 1,
                "{id}: opened and shut again, frame after frame"
            );
        }
    }
}

#[test]
fn opening_by_itself_it_keeps_a_dragged_height() {
    let (mut app, mut h) = opened();
    let docked = console(&app);
    top_to(&mut h, &mut app, docked.min.y - 80.0);
    let dragged = console(&app);
    near(dragged.min.y, docked.min.y - 80.0, "the control: dragged");
    let remembered = app.splits.console_top();
    assert!(remembered.is_some(), "the control: remembered");
    close(&mut h, &mut app);
    app.side().engine().raise_error();
    settle(&mut h, &mut app);
    assert!(app.is_open(Window::Console), "the error opened it");
    near(console(&app).min.y, dragged.min.y, "its dragged height");
    assert_eq!(app.splits.console_top(), remembered);
}

#[test]
fn opening_by_itself_closed_maximised_the_toggle_is_kept_on_either_page() {
    for mode in [Mode::Explore, Mode::Stain] {
        let (mut app, mut h) = opened();
        let docked = console(&app);
        top_to(&mut h, &mut app, docked.min.y - 80.0);
        let dragged = console(&app);
        at(&mut h, &mut app, HEADER);
        let _ = press(&mut h, &mut app, Key::M, NONE);
        settle(&mut h, &mut app);
        assert!(app.windows.console.maximised, "the control: M maximises");
        let whole = console(&app);
        assert!(whole.min.y + 40.0 < dragged.min.y, "the control: taller");
        close(&mut h, &mut app);
        app.view.mode = mode;
        settle(&mut h, &mut app);
        app.side().engine().raise_error();
        settle(&mut h, &mut app);
        assert!(
            app.is_open(Window::Console),
            "{mode:?}: the error opened it"
        );
        assert!(!app.windows.console.maximised, "{mode:?}: lowered");
        near(console(&app).min.y, dragged.min.y, "the height under it");
        // The toggle is as it was, between that height and the full one.
        at(&mut h, &mut app, HEADER);
        let _ = press(&mut h, &mut app, Key::M, NONE);
        settle(&mut h, &mut app);
        near(console(&app).min.y, whole.min.y, "M to the full height");
        let _ = press(&mut h, &mut app, Key::M, NONE);
        settle(&mut h, &mut app);
        near(console(&app).min.y, dragged.min.y, "M back");
    }
}

#[test]
fn enter_on_the_footer_never_paints_the_editor_over_a_maximised_console() {
    let (mut app, mut h) = opened();
    let button = app.keyboard.place_of(Q1_EDIT).expect("drawn").rect;
    for events in click(button.center()) {
        let _ = h.frame(&mut app, events);
    }
    settle(&mut h, &mut app);
    // The control: the docked console leaves the editor, whose layer is above its own.
    assert!(app.keyboard.place_of(Q1_COMPONENTS[0]).is_some(), "drawn");
    let docked = h.frame(&mut app, Vec::new());
    assert_eq!(editor_over_it(&docked), Some(true), "the check can fire");

    at(&mut h, &mut app, HEADER);
    let _ = press(&mut h, &mut app, Key::M, NONE);
    settle(&mut h, &mut app);
    close(&mut h, &mut app);
    assert!(app.keyboard.place_of(Q1_COMPONENTS[0]).is_some(), "back");

    at(&mut h, &mut app, FOOTER);
    let mut frames = press(&mut h, &mut app, Key::Enter, NONE);
    frames.extend((0..3).map(|_| h.frame(&mut app, Vec::new())));
    assert!(app.is_open(Window::Console), "Enter opened it");
    assert!(app.windows.console.maximised, "as it closed");
    let painted: Vec<_> = frames.iter().filter_map(editor_over_it).collect();
    assert!(!painted.is_empty(), "the console painted");
    assert!(!painted.contains(&true), "the frames: {painted:?}");
}
