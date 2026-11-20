//! QA tests for TASK-M6-44, from R-463 K1 ("Docked ... over the bottom row") and K2 ("Maximised, it covers the panels and
//! the figure until toggled back"), and §G1's rule that the hover label is the figure's own mark: the label, on a layer
//! above the console, shows on what the console leaves of the figure and never on the console, whatever the height the
//! console is dragged to, and in the very frame of the keys that open or maximise it. And the compass's H, V and D
//! buttons and its badge (R-459, merged beside this task) take no click through the console. Each test runs its control
//! (R-176).
// The file name gives a crate name that is not snake case.
#![allow(non_snake_case)]

use eframe::egui::{pos2, Event, FullOutput, Key, Modifiers, PointerButton, Pos2, Rect, Shape};
use engine::contract::view_ui::Window;
use gui::app::App;
use gui::capture::{CompassCase, FORMAT, PIXELS_PER_POINT, SIZE};
use gui::explore::hover_label::{BACKING, LISTEN_HINT};
use gui::explore::{
    COMPASS_BADGE, COMPASS_D, COMPASS_D_ENTRIES, COMPASS_H, COMPASS_H_ENTRIES, COMPASS_TILT,
    COMPASS_V, COMPASS_V_ENTRIES,
};
use gui::headless::Headless;
use gui::layout::{Layout, HANDLE};
use gui::mock::MockEngine;
use gui::side::MockSide;
use gui::windows::console;

const NONE: Modifiers = Modifiers::NONE;
/// Edges sit on whole physical pixels, 1/1.5 point at the capture's scale: half of that each way, and a little more.
const TOL: f32 = 0.7;

type Rig = (App<MockSide>, Headless);

fn rejects(what: &str, check: impl FnOnce()) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check));
    assert!(outcome.is_err(), "the control passed: {what}");
}

fn settle(h: &mut Headless, app: &mut App<MockSide>) {
    for _ in 0..4 {
        let _ = h.frame(app, Vec::new());
    }
}

fn rig() -> Rig {
    let mut app = App::new(MockSide::new(MockEngine::new(), None), FORMAT);
    let mut h = Headless::new(SIZE, PIXELS_PER_POINT);
    settle(&mut h, &mut app);
    (app, h)
}

fn key_event(key: Key, pressed: bool, modifiers: Modifiers) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers,
    }
}

/// The frame of `key`'s press, the frame in which a key acts, then its release.
fn press(h: &mut Headless, app: &mut App<MockSide>, key: Key) -> FullOutput {
    let out = h.frame(app, vec![key_event(key, true, NONE)]);
    let _ = h.frame(app, vec![key_event(key, false, NONE)]);
    out
}

fn to_footer(h: &mut Headless, app: &mut App<MockSide>) {
    app.view.focus.path.clear();
    let _ = h.frame(app, vec![key_event(Key::Tab, true, Modifiers::SHIFT)]);
    let _ = h.frame(app, vec![key_event(Key::Tab, false, Modifiers::SHIFT)]);
}

fn open(h: &mut Headless, app: &mut App<MockSide>) {
    to_footer(h, app);
    let _ = press(h, app, Key::Enter);
    settle(h, app);
    assert!(app.is_open(Window::Console), "opened");
}

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: NONE,
    }
}

fn click(h: &mut Headless, app: &mut App<MockSide>, pos: Pos2) {
    let _ = h.frame(app, vec![Event::PointerMoved(pos)]);
    let _ = h.frame(app, vec![button(pos, true)]);
    let _ = h.frame(app, vec![button(pos, false)]);
    settle(h, app);
}

fn drag(h: &mut Headless, app: &mut App<MockSide>, from: Pos2, to: Pos2) {
    let _ = h.frame(app, vec![Event::PointerMoved(from)]);
    let _ = h.frame(app, vec![button(from, true)]);
    for k in 1..=4 {
        let at = from + (to - from) * (k as f32 / 4.0);
        let _ = h.frame(app, vec![Event::PointerMoved(at)]);
    }
    let _ = h.frame(app, vec![button(to, false)]);
    let _ = h.frame(app, vec![Event::PointerGone]);
    settle(h, app);
}

fn place(app: &App<MockSide>, id: &str) -> Option<Rect> {
    app.keyboard.place_of(id).map(|p| p.rect)
}

fn console_top(app: &App<MockSide>) -> f32 {
    place(app, console::WINDOW).expect("the console").min.y
}

fn figure(h: &Headless, app: &App<MockSide>) -> Rect {
    Layout::with_splits(h.screen(), PIXELS_PER_POINT, &app.splits).figure
}

/// The console dragged so its top is at `top`.
fn drag_top_to(h: &mut Headless, app: &mut App<MockSide>, top: f32) {
    let from = console_top(app);
    let x = h.screen().center().x;
    drag(h, app, pos2(x, from), pos2(x, top));
}

/// What `output` paints of the hover label: its backing and its text, each its rect and its clip.
fn label(output: &FullOutput) -> Vec<(Rect, Rect)> {
    let hint = format!(" · {LISTEN_HINT}");
    output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Rect(r) if r.fill == BACKING => Some((r.rect, clipped.clip_rect)),
            Shape::Text(t) if t.galley.text().ends_with(&hint) => {
                Some((t.galley.rect.translate(t.pos.to_vec2()), clipped.clip_rect))
            }
            _ => None,
        })
        .collect()
}

/// Every part of the label that shows on the screen is on `shows`.
fn check_on(parts: &[(Rect, Rect)], shows: Rect) {
    for (rect, clip) in parts {
        let seen = rect.intersect(*clip);
        if seen.width() <= 0.0 || seen.height() <= 0.0 {
            continue;
        }
        assert!(
            seen.min.y >= shows.min.y - TOL && seen.max.y <= shows.max.y + TOL,
            "{rect:?} clipped to {clip:?} shows outside {shows:?}"
        );
    }
}

/// `parts` with what shows of them taken as whole, a label no clip holds.
fn unclipped(parts: &[(Rect, Rect)]) -> Vec<(Rect, Rect)> {
    parts.iter().map(|(rect, _)| (*rect, *rect)).collect()
}

fn check_shows(parts: &[(Rect, Rect)], what: &str) {
    assert_eq!(parts.len(), 2, "{what}: the backing and the text");
    for (rect, clip) in parts {
        let seen = rect.intersect(*clip);
        assert!(
            seen.width() > 0.0 && seen.height() > 0.0,
            "{what}: {rect:?} does not show through {clip:?}"
        );
    }
}

#[test]
fn qa_m6_44_on_a_strip_of_any_height_the_hover_label_shows_and_only_on_the_strip() {
    // Some strips shorter than the label, whose height is the mock's; the taller ones it fits in.
    for strip in [14.0, 20.0, 40.0, 120.0] {
        let (mut app, mut h) = rig();
        open(&mut h, &mut app);
        let fig = figure(&h, &app);
        drag_top_to(&mut h, &mut app, fig.min.y + strip);
        let top = console_top(&app);
        assert!(
            (top - fig.min.y - strip).abs() < 1.0,
            "the strip of {strip}"
        );
        let shows = Rect::from_min_max(fig.min, pos2(fig.max.x, top));
        // The cursor at the strip's top and just clear of the console's handle, at the figure's left, middle and right, clear
        // of the panels' splitters.
        let ys = [fig.min.y + 2.0, top - HANDLE / 2.0 - 2.0];
        let xs = [fig.min.x + HANDLE, fig.center().x, fig.max.x - HANDLE];
        for (x, y) in xs.iter().flat_map(|x| ys.iter().map(move |y| (*x, *y))) {
            let _ = h.frame(&mut app, vec![Event::PointerMoved(pos2(x, y))]);
            settle(&mut h, &mut app);
            let out = h.frame(&mut app, Vec::new());
            let parts = label(&out);
            check_shows(&parts, "a label on the strip");
            check_on(&parts, shows);
            // Control: for a strip shorter than the label, a label no clip holds is past the strip.
            let (rect, _) = parts[0];
            if rect.height() > strip {
                rejects("a label past the strip", || {
                    check_on(&unclipped(&parts), shows)
                });
            }
        }
    }
}

#[test]
fn qa_m6_44_the_keys_that_open_and_maximise_the_console_leave_no_label_on_it_in_their_frame() {
    let (mut app, mut h) = rig();
    let fig = figure(&h, &app);
    // A console dragged tall enough to cover part of the figure, closed by its ×: the resting pointer goes under it.
    open(&mut h, &mut app);
    drag_top_to(&mut h, &mut app, fig.center().y);
    let top = console_top(&app);
    let close = place(&app, console::CLOSE).expect("×").center();
    click(&mut h, &mut app, close);
    assert!(!app.is_open(Window::Console), "closed");
    let under = pos2(fig.center().x, (top + fig.max.y) / 2.0);
    assert!(under.y > top && fig.contains(under), "under the console");
    let _ = h.frame(&mut app, vec![Event::PointerMoved(under)]);
    settle(&mut h, &mut app);
    let before = label(&h.frame(&mut app, Vec::new()));
    check_shows(&before, "the label before the console opens");

    // Enter on the footer: in its own frame the label is off the console.
    to_footer(&mut h, &mut app);
    let opening = press(&mut h, &mut app, Key::Enter);
    assert!(app.is_open(Window::Console), "opened");
    let top = console_top(&app);
    let shows = Rect::from_min_max(fig.min, pos2(fig.max.x, top));
    check_on(&label(&opening), shows);
    rejects("the label on the console", || check_on(&before, shows));

    // A pointer on the figure above the console: M covers it in M's own frame.
    settle(&mut h, &mut app);
    let above = pos2(fig.center().x, (fig.min.y + top) / 2.0);
    let _ = h.frame(&mut app, vec![Event::PointerMoved(above)]);
    settle(&mut h, &mut app);
    let docked = label(&h.frame(&mut app, Vec::new()));
    check_shows(&docked, "the label above the docked console");
    let events = vec![key_event(Key::M, true, NONE), Event::Text("m".to_owned())];
    let maximised = h.frame(&mut app, events);
    assert!(app.windows.console.maximised, "M maximised it");
    let none = |parts: &[(Rect, Rect)]| {
        let seen = parts
            .iter()
            .map(|(rect, clip)| rect.intersect(*clip))
            .any(|r| r.width() > 0.0 && r.height() > 0.0);
        assert!(!seen, "the label shows on the maximised console: {parts:?}");
    };
    none(&label(&maximised));
    rejects("the label of the frame before", || none(&docked));
}

/// A rig with the console over the compass, `maximised` or dragged tall, or closed.
fn covered(cover: Option<bool>) -> Rig {
    let (mut app, mut h) = rig();
    let edits = CompassCase::Badge.edits(&app.snapshot().sim);
    for edit in edits {
        app.set_field(edit);
    }
    settle(&mut h, &mut app);
    let tilt = place(&app, COMPASS_TILT).expect("Tilt").center();
    click(&mut h, &mut app, tilt);
    let _ = h.frame(&mut app, vec![Event::PointerGone]);
    settle(&mut h, &mut app);
    if let Some(maximised) = cover {
        open(&mut h, &mut app);
        let page = Layout::with_splits(h.screen(), PIXELS_PER_POINT, &app.splits).page;
        if maximised {
            let at = place(&app, console::MAXIMISE).expect("maximise").center();
            click(&mut h, &mut app, at);
            let _ = h.frame(&mut app, vec![Event::PointerGone]);
            settle(&mut h, &mut app);
            assert!(app.windows.console.maximised, "maximised");
        } else {
            drag_top_to(&mut h, &mut app, page.min.y + 90.0);
        }
    }
    (app, h)
}

fn axis_click_opens_its_menu(cover: Option<bool>, id: &str, entries: &[&str; 8]) -> bool {
    let (mut app, mut h) = covered(cover);
    let at = place(&app, id).expect("the axis button").center();
    if cover.is_some() {
        let console = place(&app, console::WINDOW).expect("the console");
        assert!(console.contains(at), "{id} at {at:?} not under {console:?}");
    }
    click(&mut h, &mut app, at);
    let open = entries.iter().any(|e| place(&app, e).is_some());
    let _ = h.frame(&mut app, vec![Event::PointerGone]);
    open
}

#[test]
fn qa_m6_44_the_cubes_axis_buttons_take_no_click_through_the_console() {
    for (id, entries) in [
        (COMPASS_H, &COMPASS_H_ENTRIES),
        (COMPASS_V, &COMPASS_V_ENTRIES),
        (COMPASS_D, &COMPASS_D_ENTRIES),
    ] {
        assert!(
            axis_click_opens_its_menu(None, id, entries),
            "control: with the console closed {id} opens its menu"
        );
        assert!(
            !axis_click_opens_its_menu(Some(false), id, entries),
            "{id} opened through the docked console"
        );
        assert!(
            !axis_click_opens_its_menu(Some(true), id, entries),
            "{id} opened through the maximised console"
        );
    }
}

fn badge_click_leaves_the_cube(cover: Option<bool>) -> bool {
    let (mut app, mut h) = covered(cover);
    let at = place(&app, COMPASS_BADGE).expect("the badge").center();
    if cover.is_some() {
        let console = place(&app, console::WINDOW).expect("the console");
        assert!(console.contains(at), "the badge at {at:?} is not under it");
    }
    click(&mut h, &mut app, at);
    // Off the Cube view its buttons are not drawn.
    place(&app, COMPASS_H).is_none()
}

#[test]
fn qa_m6_44_the_badge_takes_no_click_through_the_console() {
    assert!(
        badge_click_leaves_the_cube(None),
        "control: with the console closed the badge leaves the cube"
    );
    assert!(
        !badge_click_leaves_the_cube(Some(false)),
        "the badge clicked through the docked console"
    );
    assert!(
        !badge_click_leaves_the_cube(Some(true)),
        "the badge clicked through the maximised console"
    );
}

#[test]
fn qa_m6_44_maximised_a_panels_open_menu_does_not_show_over_the_console() {
    // H's menu opened by the pointer, the console then opened and maximised by the keys, no click closing the menu.
    let (mut app, mut h) = covered(None);
    let at = place(&app, COMPASS_H).expect("H").center();
    click(&mut h, &mut app, at);
    let menu = |app: &App<MockSide>| COMPASS_H_ENTRIES.iter().any(|e| place(app, e).is_some());
    assert!(menu(&app), "control: the menu is open before the console");
    open(&mut h, &mut app);
    let _ = press(&mut h, &mut app, Key::M);
    settle(&mut h, &mut app);
    assert!(app.windows.console.maximised, "M maximised it");
    assert!(!menu(&app), "H's menu shows over the maximised console");
}
