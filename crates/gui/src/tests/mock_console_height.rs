//! `mock_console_height` (REQ-GUI-215; R-463 K1 to K3; render_gui_spec §G12, §G5, §G3): on the mock, pointer and keyboard
//! scripts on the console's height. A drag on its top edge sets the height, held at both ends, and the height survives
//! a restart; a double-click, and View › reset layout, return it to its default; a click on maximise, and M with the
//! focus in the console, toggle the full height below the top bar and back to the dragged height; M while the search
//! field is typed in types 'm'; → from Clear reaches maximise, then ×; maximised, the console covers the panels and the
//! figure; the hover label stays on the strip of figure a dragged console leaves; a basis editor it covers is hidden and takes
//! no input, and a panel's popup it reaches closes; over the whole page, it leaves only the top bar and the footer to the
//! keys, and opening by itself it does not go over the page, as it takes no focus. Each check also runs on an outcome it must reject (R-176, through `rejects`; gui takes no dependency on
//! validation, R-187): a console of fixed height, an M that types nothing outside the field, and the others named.

use std::path::{Path, PathBuf};

use eframe::egui::{self, pos2, vec2, CursorIcon, Event, Key, Modifiers, Rect, Shape};
use engine::contract::view_ui::{KeyMode, Window};

use super::mock_layout_splitters::{current, drag};
use super::support::{
    click, click_scope, focus, headless, mock_app, press, rejects, shapes, FORMAT,
};
use crate::app::App;
use crate::explore::breadcrumb;
use crate::explore::footer::FOOTER;
use crate::explore::hover_label::{BACKING, LISTEN_HINT};
use crate::explore::manifold_view::{AXES, Q1_COMPONENTS, Q1_EDIT};
use crate::explore::top_bar::TOP_BAR;
use crate::explore::FIGURE;
use crate::headless::{Headless, FRAME_S};
use crate::layout::{console_handle, Layout, Splits, HANDLE, MIN_CONSOLE_H};
use crate::mock::MockEngine;
use crate::settings::{Settings, Store, CONSOLE_TOP, FILE};
use crate::side::{EngineSide, MockSide};
use crate::windows::console::{
    CLEAR, CLOSE, COPY, HEADER, LOG, MAXIMISE, MAXIMISE_TEXT, SEARCH, WINDOW,
};

const NONE: Modifiers = Modifiers::NONE;
const NAV: KeyMode = KeyMode::Navigation;
const INTERACT: KeyMode = KeyMode::Interaction;

/// A directory of its own for test `name`, empty.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "principia-gui-console-{}-{name}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

fn settle(h: &mut Headless, app: &mut App<MockSide>) {
    for _ in 0..3 {
        let _ = h.frame(app, Vec::new());
    }
}

/// `app` with its console opened by Enter on the footer, the focus on its header bar scope.
fn open_console(h: &mut Headless, app: &mut App<MockSide>) {
    settle(h, app);
    app.view.focus.path.clear();
    press(h, app, Key::Tab, Modifiers::SHIFT);
    assert_eq!(focus(app), [FOOTER]);
    press(h, app, Key::Enter, NONE);
    settle(h, app);
    assert!(app.is_open(Window::Console));
    assert_eq!(focus(app), [WINDOW, HEADER]);
}

/// A GUI on the mock with its console open.
fn opened() -> (App<MockSide>, Headless) {
    let mut app = mock_app();
    let mut h = headless();
    open_console(&mut h, &mut app);
    (app, h)
}

/// A GUI on the mock, its settings at `path`, with its console open.
fn opened_at(path: &Path) -> (App<MockSide>, Headless) {
    let mut app = App::with_settings(
        MockSide::new(MockEngine::new(), None),
        FORMAT,
        Store::at(path),
    );
    let mut h = headless();
    open_console(&mut h, &mut app);
    (app, h)
}

/// Where the console was drawn last frame.
fn drawn(app: &App<MockSide>) -> Rect {
    app.keyboard
        .place_of(WINDOW)
        .expect("the console drawn")
        .rect
}

/// A drag on the console's top edge by `by` points, down positive.
fn drag_edge(h: &mut Headless, app: &mut App<MockSide>, by: f32) {
    let from = pos2(200.0, drawn(app).min.y);
    drag(h, app, from, from + vec2(0.0, by));
}

/// The height below the top bar on `h`.
fn full(h: &Headless, app: &App<MockSide>) -> Rect {
    current(h, app).page
}

/// `a` and `b` the same rect, but for egui's rounding of a window's place.
fn same(a: Rect, b: Rect) {
    for (x, y) in [
        (a.min.x, b.min.x),
        (a.min.y, b.min.y),
        (a.max.x, b.max.x),
        (a.max.y, b.max.y),
    ] {
        assert!((x - y).abs() < 0.1, "{a:?}, want {b:?}");
    }
}

fn near(a: f32, b: f32, what: &str) {
    assert!((a - b).abs() <= 1.0, "{what}: {a}, want {b}");
}

/// Presses M with `modifiers`.
fn m(h: &mut Headless, app: &mut App<MockSide>, modifiers: Modifiers) {
    press(h, app, Key::M, modifiers);
    let _ = h.frame(app, Vec::new());
}

#[test]
fn mock_console_height_a_drag_on_the_top_edge_sets_it() {
    let (mut app, mut h) = opened();
    let before = drawn(&app);
    let default = current(&h, &app).bottom_row;
    same(before, default);
    assert_eq!(app.splits.console_top(), None);
    drag_edge(&mut h, &mut app, -100.0);
    let after = drawn(&app);
    near(after.min.y, before.min.y - 100.0, "the top edge");
    near(after.max.y, before.max.y, "docked above the footer");
    near(after.min.x, before.min.x, "its left");
    near(after.max.x, before.max.x, "its right");
    assert!(app.splits.console_top().is_some());
    // No panel moved: the console is over them, no splitter of theirs is its.
    assert_eq!(current(&h, &app).bottom_row, default);
    // Down too, and followed in the frame the pointer moves.
    let from = pos2(200.0, after.min.y);
    let button = |pressed| Event::PointerButton {
        pos: from,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: NONE,
    };
    let _ = h.frame(&mut app, vec![Event::PointerMoved(from)]);
    let _ = h.frame(&mut app, vec![button(true)]);
    let _ = h.frame(&mut app, vec![Event::PointerMoved(from + vec2(0.0, 40.0))]);
    near(drawn(&app).min.y, after.min.y + 40.0, "during the drag");
    // Back where it began: as it was, exactly.
    let top = app.splits.console_top();
    let _ = h.frame(&mut app, vec![Event::PointerMoved(from)]);
    let _ = h.frame(&mut app, vec![button(false)]);
    near(drawn(&app).min.y, after.min.y, "back");
    assert_ne!(app.splits.console_top(), top);
    // The control: a console of fixed height, which the drag leaves where it was.
    let check = |top: f32| near(top, before.min.y - 100.0, "the top edge");
    check(after.min.y);
    rejects("a console of fixed height", || check(before.min.y));
}

#[test]
fn mock_console_height_is_held_at_both_ends() {
    let (mut app, mut h) = opened();
    let page = full(&h, &app);
    // Up past the top bar: the full height below it, never over it.
    drag_edge(&mut h, &mut app, -2000.0);
    same(drawn(&app), page);
    // Down past the footer: the header bar and three log rows.
    drag_edge(&mut h, &mut app, 2000.0);
    let least = drawn(&app);
    near(least.height(), MIN_CONSOLE_H, "the least height");
    near(least.max.y, page.max.y, "docked");
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(app.windows.console.page, 3, "three log rows show");
    let log = app.keyboard.place_of(LOG).expect("the log").rect;
    assert!(least.contains_rect(log), "{log:?} in {least:?}");
    let foot = app.keyboard.place_of(HEADER).expect("the header").rect;
    assert!(least.contains_rect(foot), "{foot:?} in {least:?}");
    // No larger than it takes: a row less and a page of two.
    let row = app.windows.console.page as f32;
    let spare = log.height() - (row * 18.0 + (row - 1.0) * 3.0);
    assert!(spare < 21.0, "{spare} spare");
    // The controls: a GUI whose drags took no hold of the edge, starting off its handle, reached neither end.
    let (mut slack, mut h2) = opened();
    let off = pos2(200.0, drawn(&slack).min.y + HANDLE * 4.0);
    drag(&mut h2, &mut slack, off, off + vec2(0.0, -2000.0));
    let top = drawn(&slack);
    rejects("a drag that stopped short of the top bar", || {
        same(top, page)
    });
    drag(&mut h2, &mut slack, off, off + vec2(0.0, 2000.0));
    let height = drawn(&slack).height();
    rejects("a drag that stopped short of the least height", || {
        near(height, MIN_CONSOLE_H, "the least height")
    });
}

#[test]
fn mock_console_height_keeps_its_share_held_as_the_window_shrinks() {
    let (mut app, mut h) = opened();
    drag_edge(&mut h, &mut app, 2000.0);
    let fraction = app.splits.console_top().expect("dragged");
    h.set_size([2160, 900]);
    settle(&mut h, &mut app);
    let layout = current(&h, &app);
    assert_eq!(app.splits.console_top(), Some(fraction));
    near(drawn(&app).height(), MIN_CONSOLE_H, "held at its least");
    assert!(layout.console.min.y >= layout.page.min.y);
    // A window too short for the minimum: the top bar wins, the console never over it.
    let tiny = Layout::with_splits(
        Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 100.0)),
        1.0,
        &app.splits,
    );
    assert_eq!(tiny.console.min.y, tiny.footer.min.y - MIN_CONSOLE_H);
    // The control: a console that kept the height below the top bar, not its least.
    let whole = layout.page.height();
    rejects("a console not held as the window shrank", || {
        near(whole, MIN_CONSOLE_H, "held at its least")
    });
}

#[test]
fn mock_console_height_survives_a_restart() {
    let dir = scratch("restart");
    let path = dir.join(FILE);
    let (top, rect) = {
        let (mut app, mut h) = opened_at(&path);
        drag_edge(&mut h, &mut app, -120.0);
        (app.splits.console_top(), drawn(&app))
    };
    assert!(top.is_some());
    let text = std::fs::read_to_string(&path).expect("saved as the drag ended");
    assert!(text.contains(&format!("\"{CONSOLE_TOP}\"")), "{text}");
    let (app, h) = opened_at(&path);
    assert_eq!(app.splits.console_top(), top);
    same(drawn(&app), rect);
    same(current(&h, &app).console, rect);
    // The GUI that forgets: the same drag, no store.
    let (forgot, _) = opened();
    let got = drawn(&forgot);
    rejects("a GUI that forgets the height", || assert_eq!(got, rect));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn mock_console_height_the_file_reads_back() {
    let default = Settings::default();
    assert!(
        !default.to_json().contains(CONSOLE_TOP),
        "undragged, unnamed"
    );
    let mut layout = Splits::default();
    layout.set_console_top(Some(0.25));
    let settings = Settings {
        layout,
        ..Settings::default()
    };
    assert_eq!(Settings::from_json(&settings.to_json()), Ok(settings));
    for edge in ["0", "1"] {
        let text = format!(r#"{{"layout": {{"{CONSOLE_TOP}": {edge}}}}}"#);
        assert!(Settings::from_json(&text).is_ok(), "{text}");
    }
    for bad in ["-0.01", "1.01", "\"0.5\""] {
        let text = format!(r#"{{"layout": {{"{CONSOLE_TOP}": {bad}}}}}"#);
        assert!(Settings::from_json(&text).is_err(), "{text}");
    }
    rejects("an unreadable height read", || {
        Settings::from_json(&format!(r#"{{"layout": {{"{CONSOLE_TOP}": 2}}}}"#)).unwrap();
    });
}

#[test]
fn mock_console_height_a_double_click_resets_it() {
    let dir = scratch("double");
    let path = dir.join(FILE);
    let (mut app, mut h) = opened_at(&path);
    drag_edge(&mut h, &mut app, -100.0);
    let moved = drawn(&app);
    let default = current(&h, &app).bottom_row;
    assert_ne!(moved, default);
    let at = pos2(200.0, moved.min.y);
    h.set_frame_step(0.05);
    for _ in 0..2 {
        for events in crate::capture::click(at) {
            let _ = h.frame(&mut app, events);
        }
    }
    h.set_frame_step(FRAME_S);
    settle(&mut h, &mut app);
    same(drawn(&app), default);
    assert_eq!(app.splits.console_top(), None);
    assert_eq!(Store::at(&path).load().settings.layout.console_top(), None);
    rejects("a double-click that leaves it", || {
        assert_eq!(moved, default)
    });
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn mock_console_height_view_reset_layout_restores_it() {
    let dir = scratch("reset");
    let path = dir.join(FILE);
    let (mut app, mut h) = opened_at(&path);
    drag_edge(&mut h, &mut app, -100.0);
    let dragged = drawn(&app);
    click_scope(&mut h, &mut app, MAXIMISE);
    assert!(app.windows.console.maximised);
    click(&mut h, &mut app, "View");
    click(&mut h, &mut app, "reset layout");
    settle(&mut h, &mut app);
    assert_eq!(app.splits, Splits::default());
    assert!(!app.windows.console.maximised, "reset takes it back");
    same(drawn(&app), current(&h, &app).bottom_row);
    assert_eq!(Store::at(&path).load().settings.layout, Splits::default());
    let default = current(&h, &app).bottom_row;
    rejects("a reset that left the dragged height", || {
        same(dragged, default)
    });
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn mock_console_height_the_edge_shows_a_resize_cursor_and_is_no_keyboard_element() {
    let (mut app, mut h) = opened();
    let rect = drawn(&app);
    assert_eq!(console_handle(rect).height(), HANDLE);
    assert_eq!(console_handle(rect).center().y, rect.min.y);
    let cursor = |h: &mut Headless, app: &mut App<MockSide>, at| {
        let _ = h.frame(app, vec![Event::PointerMoved(at)]);
        h.frame(app, Vec::new()).platform_output.cursor_icon
    };
    // Each end of the strip is on the handle, and a point past it off, inside and above the console.
    let half = HANDLE / 2.0 - 0.5;
    for y in [rect.min.y - half, rect.min.y, rect.min.y + half] {
        let got = cursor(&mut h, &mut app, pos2(200.0, y));
        assert_eq!(got, CursorIcon::ResizeVertical, "at {y}");
    }
    let inside = cursor(&mut h, &mut app, pos2(rect.center().x, rect.center().y));
    assert_eq!(inside, CursorIcon::Default);
    rejects("an edge with no cursor", || {
        assert_eq!(inside, CursorIcon::ResizeVertical)
    });
    let tree = app.keyboard.tree(app.view.mode);
    assert_eq!(tree.children(Some(WINDOW)), [HEADER, LOG]);
}

#[test]
fn mock_console_height_on_the_stain_page_too() {
    let (mut app, mut h) = opened();
    app.view.mode = engine::contract::view_ui::Mode::Stain;
    settle(&mut h, &mut app);
    let before = drawn(&app);
    drag_edge(&mut h, &mut app, -60.0);
    near(drawn(&app).min.y, before.min.y - 60.0, "the top edge");
    m(&mut h, &mut app, NONE);
    assert!(app.windows.console.maximised, "M on the Stain page");
    let page = full(&h, &app);
    same(drawn(&app), page);
    rejects("a console that M left docked on the Stain page", || {
        same(before, page)
    });
}

#[test]
fn mock_console_height_a_click_on_maximise_toggles_it() {
    let (mut app, mut h) = opened();
    drag_edge(&mut h, &mut app, -50.0);
    let dragged = drawn(&app);
    click_scope(&mut h, &mut app, MAXIMISE);
    let _ = h.frame(&mut app, Vec::new());
    assert!(app.windows.console.maximised);
    same(drawn(&app), full(&h, &app));
    assert_eq!(
        focus(&app),
        [WINDOW, HEADER, MAXIMISE],
        "the click moved the focus onto it (R-450 A3)"
    );
    click_scope(&mut h, &mut app, MAXIMISE);
    let _ = h.frame(&mut app, Vec::new());
    assert!(!app.windows.console.maximised);
    same(drawn(&app), dragged);
    let got = drawn(&app);
    let page = full(&h, &app);
    rejects("a toggle that stays maximised", || assert_eq!(got, page));
}

#[test]
fn mock_console_height_m_toggles_it_with_the_focus_in_the_console() {
    let (mut app, mut h) = opened();
    let docked = drawn(&app);
    let page = full(&h, &app);
    // On the header bar scope, navigating.
    m(&mut h, &mut app, NONE);
    same(drawn(&app), page);
    m(&mut h, &mut app, NONE);
    same(drawn(&app), docked);
    // On a control of it.
    press(&mut h, &mut app, Key::Enter, NONE);
    m(&mut h, &mut app, NONE);
    same(drawn(&app), page);
    press(&mut h, &mut app, Key::Escape, NONE);
    // On the log, navigating and interacting: either mode.
    press(&mut h, &mut app, Key::ArrowDown, NONE);
    assert_eq!(focus(&app), [WINDOW, LOG]);
    m(&mut h, &mut app, NONE);
    same(drawn(&app), docked);
    press(&mut h, &mut app, Key::Enter, NONE);
    assert_eq!(app.view.focus.mode, INTERACT);
    m(&mut h, &mut app, NONE);
    same(drawn(&app), page);
    assert_eq!(app.view.focus.mode, INTERACT, "M moved no focus");
    // With a modifier it is another key: the system's Cmd+M among them.
    for modifiers in [Modifiers::COMMAND, Modifiers::SHIFT, Modifiers::ALT] {
        m(&mut h, &mut app, modifiers);
        same(drawn(&app), page);
    }
    // One toggle a press: the system's repeats are not presses.
    let held = |repeat| Event::Key {
        key: Key::M,
        physical_key: None,
        pressed: true,
        repeat,
        modifiers: NONE,
    };
    let _ = h.frame(&mut app, vec![held(false)]);
    same(drawn(&app), docked);
    let _ = h.frame(&mut app, vec![held(true)]);
    let _ = h.frame(&mut app, Vec::new());
    same(drawn(&app), docked);
    let released = Event::Key {
        key: Key::M,
        physical_key: None,
        pressed: false,
        repeat: false,
        modifiers: NONE,
    };
    let _ = h.frame(&mut app, vec![released]);
    m(&mut h, &mut app, NONE);
    same(drawn(&app), page);
    // The `?` overlay takes every key.
    press(&mut h, &mut app, Key::Questionmark, Modifiers::SHIFT);
    m(&mut h, &mut app, NONE);
    same(drawn(&app), page);
    press(&mut h, &mut app, Key::Escape, NONE);
    let rows = app.keyboard.tree(app.view.mode).shortcuts();
    assert!(rows.iter().any(|(key, _)| *key == "M"), "on the overlay");
    let got = drawn(&app);
    rejects("an M that did nothing", || same(got, docked));
}

#[test]
fn mock_console_height_m_with_the_focus_elsewhere_does_nothing() {
    // The console open by itself, on an error, the focus in the top bar.
    let mut app = mock_app();
    let mut h = headless();
    settle(&mut h, &mut app);
    press(&mut h, &mut app, Key::Tab, NONE);
    app.side().engine().raise_error();
    settle(&mut h, &mut app);
    assert!(app.is_open(Window::Console));
    assert_eq!(focus(&app), ["top_bar"]);
    let docked = drawn(&app);
    m(&mut h, &mut app, NONE);
    same(drawn(&app), docked);
    assert!(!app.windows.console.maximised);
    let page = full(&h, &app);
    rejects("M from outside it", || assert_eq!(docked, page));
}

/// M and its text, as a window sends them.
fn typed_m() -> Vec<Event> {
    vec![
        Event::Key {
            key: Key::M,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: NONE,
        },
        Event::Text("m".to_owned()),
    ]
}

/// M pressed, as a window sends it with its text, and released.
fn type_m(h: &mut Headless, app: &mut App<MockSide>) {
    let _ = h.frame(app, typed_m());
    let release = Event::Key {
        key: Key::M,
        physical_key: None,
        pressed: false,
        repeat: false,
        modifiers: NONE,
    };
    let _ = h.frame(app, vec![release]);
    let _ = h.frame(app, Vec::new());
}

#[test]
fn mock_console_height_m_while_typing_types_m() {
    let (mut app, mut h) = opened();
    press(&mut h, &mut app, Key::Enter, NONE);
    while focus(&app).last() != Some(&SEARCH) {
        press(&mut h, &mut app, Key::ArrowRight, NONE);
    }
    let check = |app: &App<MockSide>| {
        assert_eq!(app.windows.console.text, "m", "M typed");
        assert!(!app.windows.console.maximised, "M toggled nothing");
    };
    // Outside the field, on it but not typing: M toggles and types nothing.
    type_m(&mut h, &mut app);
    assert_eq!(app.windows.console.text, "", "typed with no field held");
    assert!(app.windows.console.maximised);
    rejects("an M that types nothing outside the field", || check(&app));
    m(&mut h, &mut app, NONE);
    // In it: Enter starts typing.
    press(&mut h, &mut app, Key::Enter, NONE);
    assert_eq!(app.view.focus.mode, INTERACT);
    type_m(&mut h, &mut app);
    check(&app);
    // A click on it types at once (R-456).
    press(&mut h, &mut app, Key::Escape, NONE);
    assert_eq!(app.view.focus.mode, NAV);
    click_scope(&mut h, &mut app, SEARCH);
    assert_eq!(app.view.focus.mode, INTERACT);
    type_m(&mut h, &mut app);
    assert_eq!(app.windows.console.text, "mm");
    assert!(!app.windows.console.maximised);
}

#[test]
fn mock_console_height_the_arrow_reaches_maximise_then_close() {
    let (mut app, mut h) = opened();
    press(&mut h, &mut app, Key::Enter, NONE);
    let crumb =
        |app: &App<MockSide>| breadcrumb::text(app.keyboard.tree(app.view.mode), &app.view.focus);
    while focus(&app).last() != Some(&CLEAR) {
        press(&mut h, &mut app, Key::ArrowRight, NONE);
    }
    press(&mut h, &mut app, Key::ArrowRight, NONE);
    assert_eq!(focus(&app), [WINDOW, HEADER, MAXIMISE], "→ from Clear");
    assert!(
        crumb(&app).ends_with("Console › Header › Maximise"),
        "{}",
        crumb(&app)
    );
    let got = focus(&app).last().map(|s| (*s).to_owned());
    rejects("× right after Clear", || {
        assert_eq!(got.as_deref(), Some(CLOSE))
    });
    press(&mut h, &mut app, Key::Enter, NONE);
    let _ = h.frame(&mut app, Vec::new());
    assert!(app.windows.console.maximised, "Enter acts as its click");
    same(drawn(&app), full(&h, &app));
    press(&mut h, &mut app, Key::ArrowRight, NONE);
    assert_eq!(focus(&app), [WINDOW, HEADER, CLOSE], "→ from maximise");
    press(&mut h, &mut app, Key::ArrowLeft, NONE);
    press(&mut h, &mut app, Key::ArrowLeft, NONE);
    assert_eq!(focus(&app), [WINDOW, HEADER, CLEAR]);
    // Drawn in that order, left to right.
    let x = |id| app.keyboard.place_of(id).expect("drawn").rect.center().x;
    assert!(x(COPY) < x(CLEAR));
    assert!(x(CLEAR) < x(MAXIMISE));
    assert!(x(MAXIMISE) < x(CLOSE));
}

#[test]
fn mock_console_height_maximised_it_covers_the_panels_and_the_figure() {
    let (mut app, mut h) = opened();
    m(&mut h, &mut app, NONE);
    let layout = current(&h, &app);
    let rect = drawn(&app);
    for (what, panel) in [
        ("the Manifold view", layout.manifold_view),
        ("the figure", layout.figure),
        ("the Trajectory panel", layout.trajectory),
        ("the bottom row", layout.bottom_row),
    ] {
        assert!(rect.expand(0.1).contains_rect(panel), "{what}");
    }
    assert!(
        !rect.intersects(layout.top_bar.shrink(1.0)),
        "not the top bar"
    );
    near(rect.max.y, layout.footer.min.y, "the footer stays");
    // On the figure the console takes the pointer: no hover goes to the engine.
    let over = layout.figure.center();
    let _ = h.frame(&mut app, vec![Event::PointerMoved(over)]);
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(
        app.snapshot().render.hover,
        Default::default(),
        "no hover under it"
    );
    // Restored, the same point hovers.
    m(&mut h, &mut app, NONE);
    let _ = h.frame(&mut app, vec![Event::PointerMoved(over)]);
    let _ = h.frame(&mut app, Vec::new());
    let _ = h.frame(&mut app, Vec::new());
    let hover = app.snapshot().render.hover.clone();
    assert_ne!(hover, Default::default(), "restored, it hovers");
    rejects("a hover under the console", || {
        assert_eq!(hover, Default::default())
    });
}

#[test]
fn mock_console_height_the_hover_label_stays_on_the_strip_it_leaves() {
    // A strip of figure shorter than the label, whose top `placed` holds at the figure's top, so its height overruns.
    const STRIP: f32 = 20.0;
    let (mut app, mut h) = opened();
    let figure = current(&h, &app).figure;
    let by = figure.min.y + STRIP - drawn(&app).min.y;
    drag_edge(&mut h, &mut app, by);
    let top = current(&h, &app).console.min.y;
    near(top, figure.min.y + STRIP, "the strip");
    // In the strip, clear of the handle on the console's edge.
    let cursor = pos2(figure.center().x, figure.min.y + 5.0);
    let _ = h.frame(&mut app, vec![Event::PointerMoved(cursor)]);
    let _ = h.frame(&mut app, Vec::new());
    let output = h.frame(&mut app, Vec::new());
    let hint = format!(" · {LISTEN_HINT}");
    let label: Vec<(Rect, Rect)> = output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Rect(r) if r.fill == BACKING => Some((r.rect, clipped.clip_rect)),
            Shape::Text(t) if t.galley.text().ends_with(&hint) => {
                Some((t.galley.rect.translate(t.pos.to_vec2()), clipped.clip_rect))
            }
            _ => None,
        })
        .collect();
    assert_eq!(label.len(), 2, "the backing and the text: {label:?}");
    let on_strip = |rect: Rect, clip: Rect| {
        let shows = rect.intersect(clip);
        assert!(
            shows.max.y <= top,
            "{rect:?} clipped to {clip:?} is past {top}"
        );
    };
    for (rect, clip) in label {
        assert!(rect.max.y > top, "{rect:?} is shorter than the strip");
        on_strip(rect, clip);
        rejects("the label clipped to the whole canvas", || {
            on_strip(rect, figure)
        });
    }
}

/// A GUI with its console open and `q₁`'s basis editor open, and where its first field is.
fn editing() -> (App<MockSide>, Headless, Rect) {
    let (mut app, mut h) = opened();
    click_scope(&mut h, &mut app, Q1_EDIT);
    settle(&mut h, &mut app);
    let field = app.keyboard.place_of(Q1_COMPONENTS[0]).expect("drawn").rect;
    (app, h, field)
}

/// That the basis editor is neither drawn nor over the console at `at`, a point the console covers: no field placed, no
/// field's label painted after the console's, and the pointer on the console's layer.
fn editor_gone(h: &mut Headless, app: &mut App<MockSide>, at: egui::Pos2) {
    let drawn = shapes(h, app);
    let text = |s: &Shape| match s {
        Shape::Text(t) => Some(t.galley.text().to_owned()),
        _ => None,
    };
    let console = drawn
        .iter()
        .position(|s| text(s).as_deref() == Some(MAXIMISE_TEXT))
        .expect("the console drawn");
    let after = drawn[console..].iter().filter_map(text);
    assert!(!after.into_iter().any(|t| AXES.contains(&t.as_str())));
    assert!(app.keyboard.place_of(Q1_COMPONENTS[0]).is_none());
    let layer = h.ctx.layer_id_at(at).expect("a layer");
    assert_eq!(layer.order, egui::Order::Middle, "{layer:?}");
}

/// The pointer's drag on `at` moved the basis.
fn dragging_edits(h: &mut Headless, app: &mut App<MockSide>, at: egui::Pos2) -> bool {
    let before = app.snapshot().sim.plane.q1;
    drag(h, app, at, at + vec2(40.0, 0.0));
    app.snapshot().sim.plane.q1 != before
}

#[test]
fn mock_console_height_maximised_it_hides_the_basis_editor() {
    let (mut app, mut h, field) = editing();
    let at = field.center();
    rejects("the editor at the default height", || {
        editor_gone(&mut h, &mut app, at)
    });
    click_scope(&mut h, &mut app, MAXIMISE);
    settle(&mut h, &mut app);
    editor_gone(&mut h, &mut app, at);
    assert!(app.manifold.editing[0], "it stays asked for");
    assert!(!dragging_edits(&mut h, &mut app, at), "no input through it");
    click_scope(&mut h, &mut app, MAXIMISE);
    settle(&mut h, &mut app);
    assert!(
        app.keyboard.place_of(Q1_COMPONENTS[0]).is_some(),
        "it comes back"
    );
    assert!(dragging_edits(&mut h, &mut app, at), "and takes the drag");
}

#[test]
fn mock_console_height_dragged_over_it_hides_the_basis_editor() {
    let (mut app, mut h, field) = editing();
    // The editor's bottom edge: its field's, with the frame's margin; a little under, the whole editor shows.
    let edge = drawn(&app).min.y;
    drag_edge(&mut h, &mut app, field.max.y + 30.0 - edge);
    settle(&mut h, &mut app);
    let clear = app.keyboard.place_of(Q1_COMPONENTS[0]);
    assert!(clear.is_some(), "the console under the editor keeps it");
    rejects("the editor clear of the console", || {
        editor_gone(&mut h, &mut app, field.center())
    });
    // Over the editor's lower part only: a part over the console is as bad as the whole.
    let edge = drawn(&app).min.y;
    drag_edge(&mut h, &mut app, field.max.y + 2.0 - edge);
    settle(&mut h, &mut app);
    let top = drawn(&app).min.y;
    let at = pos2(field.center().x, top + 20.0);
    editor_gone(&mut h, &mut app, at);
    assert!(app.manifold.editing[0], "it stays asked for");
    // Up past it.
    drag_edge(&mut h, &mut app, field.min.y - 30.0 - top);
    settle(&mut h, &mut app);
    editor_gone(&mut h, &mut app, field.center());
    assert!(!dragging_edits(&mut h, &mut app, field.center()));
}

#[test]
fn mock_console_height_the_focus_leaves_an_editor_it_hides() {
    let path = |app: &App<MockSide>, id| app.keyboard.tree(app.view.mode).path_to(id);
    let (mut app, mut h, field) = editing();
    let on_field = |h: &mut Headless, app: &mut App<MockSide>| {
        app.view.focus.path = path(app, Q1_COMPONENTS[0]);
        app.view.focus.mode = INTERACT;
        let _ = h.frame(app, Vec::new());
    };
    on_field(&mut h, &mut app);
    let kept = app.view.focus.clone();
    assert_eq!(kept.path, path(&app, Q1_COMPONENTS[0]), "the editor shows");
    let edge = drawn(&app).min.y;
    drag_edge(&mut h, &mut app, field.min.y - 30.0 - edge);
    settle(&mut h, &mut app);
    on_field(&mut h, &mut app);
    assert_eq!(app.view.focus.path, path(&app, Q1_EDIT));
    assert_eq!(app.view.focus.mode, NAV);
    let moved = app.view.focus.clone();
    rejects("a focus left on a hidden field", || {
        assert_eq!(moved.path, kept.path)
    });
}

/// `app` at `path`, one frame on, the keyboard on.
fn at_scope(h: &mut Headless, app: &mut App<MockSide>, id: &str) {
    app.view.focus.path = app.keyboard.tree(app.view.mode).path_to(id);
    app.view.focus.mode = NAV;
    let _ = h.frame(app, Vec::new());
}

/// That Tab, Shift+Tab and the arrows go only between the top bar and the footer.
fn keys_left_to_the_bars(h: &mut Headless, app: &mut App<MockSide>) {
    at_scope(h, app, TOP_BAR);
    press(h, app, Key::Tab, NONE);
    assert_eq!(focus(app), [FOOTER]);
    press(h, app, Key::Tab, NONE);
    assert_eq!(focus(app), [TOP_BAR]);
    press(h, app, Key::Tab, Modifiers::SHIFT);
    assert_eq!(focus(app), [FOOTER]);
    press(h, app, Key::ArrowLeft, NONE);
    assert_eq!(focus(app), [TOP_BAR]);
    press(h, app, Key::ArrowRight, NONE);
    assert_eq!(focus(app), [FOOTER]);
}

#[test]
fn mock_console_height_maximised_the_keys_reach_only_what_it_leaves() {
    let (mut app, mut h) = opened();
    rejects("the panels reached at the default height", || {
        keys_left_to_the_bars(&mut h, &mut app)
    });
    open_console_again(&mut h, &mut app);
    m(&mut h, &mut app, NONE);
    keys_left_to_the_bars(&mut h, &mut app);
    // Its own Tab stays inside it (R-450 A1).
    at_scope(&mut h, &mut app, HEADER);
    press(&mut h, &mut app, Key::Tab, NONE);
    assert_eq!(focus(&app), [WINDOW, LOG]);
}

/// The focus back in the console's header bar, after a check moved it out.
fn open_console_again(h: &mut Headless, app: &mut App<MockSide>) {
    at_scope(h, app, HEADER);
    assert_eq!(focus(app), [WINDOW, HEADER]);
}

#[test]
fn mock_console_height_maximised_on_the_stain_page_too() {
    let (mut app, mut h) = opened();
    app.view.mode = engine::contract::view_ui::Mode::Stain;
    settle(&mut h, &mut app);
    rejects("the panels reached at the default height", || {
        keys_left_to_the_bars(&mut h, &mut app)
    });
    open_console_again(&mut h, &mut app);
    m(&mut h, &mut app, NONE);
    keys_left_to_the_bars(&mut h, &mut app);
}

#[test]
fn mock_console_height_dragged_to_the_top_the_keys_reach_only_what_it_leaves() {
    let (mut app, mut h) = opened();
    let edge = drawn(&app).min.y;
    let top = full(&h, &app).min.y;
    drag_edge(&mut h, &mut app, top + 40.0 - edge);
    near(
        drawn(&app).min.y,
        full(&h, &app).min.y + 40.0,
        "short of it",
    );
    rejects("the panels reached short of the top", || {
        keys_left_to_the_bars(&mut h, &mut app)
    });
    drag_edge(&mut h, &mut app, -80.0);
    settle(&mut h, &mut app);
    same(drawn(&app), full(&h, &app));
    assert!(!app.windows.console.maximised);
    keys_left_to_the_bars(&mut h, &mut app);
}

#[test]
fn mock_console_height_a_focus_it_covers_moves_to_it() {
    let (mut app, mut h) = opened();
    at_scope(&mut h, &mut app, FIGURE);
    assert_eq!(focus(&app), [FIGURE], "under a docked console it stays");
    open_console_again(&mut h, &mut app);
    m(&mut h, &mut app, NONE);
    app.view.focus.mode = INTERACT;
    at_scope(&mut h, &mut app, FIGURE);
    assert_eq!(focus(&app), [WINDOW, HEADER]);
    assert_eq!(app.view.focus.mode, NAV);
    // Restored, it stays in the console, as on main; a figure under the console is then reached again.
    m(&mut h, &mut app, NONE);
    assert_eq!(focus(&app), [WINDOW, HEADER]);
    at_scope(&mut h, &mut app, FIGURE);
    assert_eq!(focus(&app), [FIGURE]);
    let restored = focus(&app);
    rejects("a focus moved at the restored height", || {
        assert_eq!(restored, [WINDOW, HEADER])
    });
}

#[test]
fn mock_console_height_closed_the_panels_are_reached_again() {
    let (mut app, mut h) = opened();
    m(&mut h, &mut app, NONE);
    press(&mut h, &mut app, Key::Escape, NONE);
    let _ = h.frame(&mut app, Vec::new());
    assert!(!app.is_open(Window::Console));
    assert_eq!(focus(&app), [FOOTER], "back on its opener");
    press(&mut h, &mut app, Key::Tab, Modifiers::SHIFT);
    let last = focus(&app);
    assert_ne!(last, [TOP_BAR], "the panels are in the order again");
    rejects("the order left at the bars", || assert_eq!(last, [TOP_BAR]));
}

/// `app` with the console closed from `covering`: maximised, or dragged to the top.
fn closed_from(maximised: bool) -> (App<MockSide>, Headless) {
    let (mut app, mut h) = opened();
    if maximised {
        m(&mut h, &mut app, NONE);
    } else {
        drag_edge(&mut h, &mut app, -5000.0);
        settle(&mut h, &mut app);
        at_scope(&mut h, &mut app, HEADER);
    }
    same(drawn(&app), full(&h, &app));
    press(&mut h, &mut app, Key::Escape, NONE);
    let _ = h.frame(&mut app, Vec::new());
    assert!(!app.is_open(Window::Console));
    (app, h)
}

#[test]
fn mock_console_height_opening_by_itself_it_takes_no_focus_at_any_height() {
    for maximised in [true, false] {
        let (mut app, mut h) = closed_from(maximised);
        at_scope(&mut h, &mut app, FIGURE);
        app.side().engine().raise_error();
        settle(&mut h, &mut app);
        assert!(app.is_open(Window::Console));
        assert_eq!(focus(&app), [FIGURE], "the focus not taken");
        let page = full(&h, &app);
        let got = drawn(&app);
        assert!(got.min.y > page.min.y, "not over the page: {got:?}");
        assert!(!app.windows.console.maximised);
        rejects("a console over the page", || same(got, page));
    }
    // The control: at the default height, the same.
    let mut app = mock_app();
    let mut h = headless();
    settle(&mut h, &mut app);
    at_scope(&mut h, &mut app, FIGURE);
    app.side().engine().raise_error();
    settle(&mut h, &mut app);
    assert_eq!(focus(&app), [FIGURE]);
}

#[test]
fn mock_console_height_only_an_auto_open_lowers_it() {
    // Opened by the keys, a console closed maximised opens maximised, and takes the focus.
    let (mut app, mut h) = closed_from(true);
    at_scope(&mut h, &mut app, FOOTER);
    press(&mut h, &mut app, Key::Enter, NONE);
    settle(&mut h, &mut app);
    assert!(app.is_open(Window::Console));
    assert!(app.windows.console.maximised, "still maximised");
    same(drawn(&app), full(&h, &app));
    assert_eq!(focus(&app), [WINDOW, HEADER]);
    // Already open, an error changes nothing.
    app.side().engine().raise_error();
    settle(&mut h, &mut app);
    assert!(app.windows.console.maximised, "an error while it is open");
    let (got, page) = (drawn(&app), full(&h, &app));
    same(got, page);
    rejects("a console lowered by an error", || {
        assert!(got.min.y > page.min.y)
    });
}

#[test]
fn mock_console_height_the_drag_is_held_in_any_window() {
    // A window off the screen's origin, so each edge's offset counts.
    let screen = Rect::from_min_size(pos2(0.0, 100.0), vec2(1000.0, 1000.0));
    let layout = Layout::new(screen, 1.0);
    let (top, footer) = (layout.page.min.y, layout.page.max.y);
    let edge = |fraction: f32| screen.min.y + fraction * screen.height();
    near(
        edge(layout.console_dragged(500.0, 5000.0)),
        footer - MIN_CONSOLE_H,
        "its least",
    );
    near(
        edge(layout.console_dragged(500.0, -5000.0)),
        top,
        "its most",
    );
    near(edge(layout.console_dragged(500.0, 10.0)), 510.0, "between");
    // The control: the fraction a drag held nowhere gives.
    let unheld = (500.0 + 5000.0 - screen.min.y) / screen.height();
    rejects("a drag held nowhere", || {
        near(edge(unheld), footer - MIN_CONSOLE_H, "its least")
    });
}
