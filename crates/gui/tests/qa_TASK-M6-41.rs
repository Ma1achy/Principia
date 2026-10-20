//! QA tests for TASK-M6-41, written from REQ-GUI-210's statement and verify, render_gui_spec §G3's paragraph
//! "Keyboard navigation is on or off, apart from the focus (R-460, R-464)", R-460 and R-464, not from the
//! implementation:
//!
//! - The GUI starts with keyboard navigation off; any key press but a modifier alone turns it on and shows the ring
//!   where the focus is once the key has acted; with no focus set it draws nothing either way (R-464 M2). The flag is
//!   the GUI's own, never the contract's: turning it on changes no `ViewUI` field, no snapshot and no history
//!   (gui_state_contract §2; R-460 H4).
//! - A window opened with the mouse takes the focus on its landing with no ring and no breadcrumb path, whatever
//!   keyboard navigation was before ("whatever it was", §G3; R-460 H2); opened from the keyboard, the ring is on the
//!   landing.
//! - Esc closes a mouse-opened window at once, as it acts with keyboard navigation off (R-460 H2's "so Esc still
//!   closes it"; R-464 M2: Esc acts as always), the focus back on the opener and, Esc being a key press, the ring shown
//!   there.
//! - After a click, each key acts as always and shows the ring where the focus is: Tab, Shift+Tab and the arrows move
//!   on from the focus the click left, exactly as from that focus with the ring on; Esc, `?` and the letter shortcuts
//!   act; Enter in navigation mode only shows the ring, without acting (Copy copies nothing until the next Enter).
//! - Any click or drag hides the ring, a secondary click and a drag on a window's title bar included; a scroll and a
//!   hover leave it (R-464 M3).
//! - A click on the console's search field types with no ring; the first key then shows it: a letter goes in and the
//!   ring is blue; Esc or Enter stop typing, the text kept, the ring orange on the field; Tab leaves it, the ring
//!   orange where the focus lands (R-464 "The first key typed after a click on a text field").
//! - The console opening by itself on an error leaves the focus and keyboard navigation as they were (R-464, H1's
//!   paragraph).
//!
//! The ring is read from the shapes drawn (a rect stroked at the ring's width, in navigation's orange or the theme's
//! hyperlink blue), and the breadcrumb from the names drawn in the top bar. Each check runs its own control (R-176)
//! through [`rejects`].
// The file name `qa_TASK-M6-41` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use eframe::egui::{self, Color32, Event, Key, Modifiers, PointerButton, Pos2, Rect, Vec2};
use engine::contract::view_ui::{KeyMode, Window};
use gui::app::App;
use gui::capture::{FORMAT, PIXELS_PER_POINT, SIZE};
use gui::explore::breadcrumb::{self, NAVIGATION, RING_WIDTH};
use gui::explore::{footer, trajectory, FIGURE};
use gui::headless::Headless;
use gui::layout::Layout;
use gui::mock::MockEngine;
use gui::side::{EngineSide, MockSide};
use gui::windows::{self, console, REGISTRY};

const NONE: Modifiers = Modifiers::NONE;
const SHIFT: Modifiers = Modifiers::SHIFT;
const NAV: KeyMode = KeyMode::Navigation;
const INTERACT: KeyMode = KeyMode::Interaction;
/// A key no command binds and no widget reads.
const UNBOUND: Key = Key::F12;

/// Asserts that `check` panics: the check can fail (R-176's control, run in the test).
fn rejects(what: &str, check: impl FnOnce()) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check));
    assert!(
        outcome.is_err(),
        "the check passed on {what}, which it must reject"
    );
}

type Rig = (App<MockSide>, Headless);

fn fresh() -> Rig {
    let mut app = App::new(MockSide::new(MockEngine::frozen(), None), FORMAT);
    let mut h = Headless::new(SIZE, PIXELS_PER_POINT);
    for _ in 0..2 {
        let _ = h.frame(&mut app, Vec::new());
    }
    (app, h)
}

fn layout(h: &Headless) -> Layout {
    Layout::new(h.screen(), PIXELS_PER_POINT)
}

fn settle(h: &mut Headless, app: &mut App<MockSide>) {
    for _ in 0..8 {
        let _ = h.frame(app, Vec::new());
    }
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

/// Presses and releases `key`, returning whether any frame asked for a copy to the clipboard.
fn press(h: &mut Headless, app: &mut App<MockSide>, key: Key, modifiers: Modifiers) -> bool {
    let mut copied = false;
    for events in [
        vec![key_event(key, true, modifiers)],
        vec![key_event(key, false, modifiers)],
        Vec::new(),
    ] {
        let out = h.frame(app, events);
        copied |= out
            .platform_output
            .commands
            .iter()
            .any(|c| matches!(c, egui::OutputCommand::CopyText(_)));
    }
    copied
}

/// Types one letter as a keyboard does: its press with its text, then its release.
fn type_letter(h: &mut Headless, app: &mut App<MockSide>, c: char) {
    let key = Key::from_name(&c.to_uppercase().to_string()).expect("a letter key");
    let _ = h.frame(
        app,
        vec![key_event(key, true, NONE), Event::Text(c.to_string())],
    );
    let _ = h.frame(app, vec![key_event(key, false, NONE)]);
    let _ = h.frame(app, Vec::new());
}

fn button(pos: Pos2, which: PointerButton, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button: which,
        pressed,
        modifiers: NONE,
    }
}

/// A click of `which` at `pos`: move, press, release, then the pointer gone.
fn click_with(h: &mut Headless, app: &mut App<MockSide>, pos: Pos2, which: PointerButton) {
    for events in [
        vec![Event::PointerMoved(pos)],
        vec![button(pos, which, true)],
        vec![button(pos, which, false)],
        vec![Event::PointerGone],
    ] {
        let _ = h.frame(app, events);
    }
    settle(h, app);
}

fn click(h: &mut Headless, app: &mut App<MockSide>, pos: Pos2) {
    click_with(h, app, pos, PointerButton::Primary);
}

/// Where scope `id` was placed last frame.
fn place(app: &App<MockSide>, id: &str) -> Rect {
    app.keyboard
        .place_of(id)
        .unwrap_or_else(|| panic!("`{id}` not placed"))
        .rect
}

fn click_on(h: &mut Headless, app: &mut App<MockSide>, id: &str) {
    let at = place(app, id).center();
    click(h, app, at);
}

fn path_to(app: &App<MockSide>, id: &str) -> Vec<String> {
    app.keyboard.tree(app.view.mode).path_to(id)
}

fn owned(path: &[&str]) -> Vec<String> {
    path.iter().map(|s| (*s).to_owned()).collect()
}

/// What a frame shows of the keyboard layer.
#[derive(Clone, Debug)]
struct Seen {
    focus: Vec<String>,
    mode: KeyMode,
    /// The rings drawn, with their colours.
    rings: Vec<(Rect, Color32)>,
    /// Where the focused scope was placed, if it was.
    focus_rect: Option<Rect>,
    /// Whether the breadcrumb of the focus's path is drawn in the top bar.
    breadcrumb: bool,
}

fn flatten(shape: &egui::Shape, out: &mut Vec<egui::Shape>) {
    match shape {
        egui::Shape::Vec(v) => v.iter().for_each(|s| flatten(s, out)),
        other => out.push(other.clone()),
    }
}

fn blue(h: &Headless) -> Color32 {
    h.ctx.global_style().visuals.hyperlink_color
}

/// One more frame, and what it shows.
fn seen(h: &mut Headless, app: &mut App<MockSide>) -> Seen {
    let out = h.frame(app, Vec::new());
    let blue = blue(h);
    let mut flat = Vec::new();
    for c in &out.shapes {
        flatten(&c.shape, &mut flat);
    }
    let rings = flat
        .into_iter()
        .filter_map(|s| match s {
            egui::Shape::Rect(r)
                if r.stroke.width == RING_WIDTH
                    && (r.stroke.color == NAVIGATION || r.stroke.color == blue) =>
            {
                Some((r.rect, r.stroke.color))
            }
            _ => None,
        })
        .collect();
    let crumb = breadcrumb::text(app.keyboard.tree(app.view.mode), &app.view.focus);
    let top = layout(h).top_bar;
    let ppp = f64::from(PIXELS_PER_POINT);
    let breadcrumb = !crumb.is_empty()
        && h.names(&out).iter().any(|n| {
            n.name.contains(&crumb)
                && n.rect.is_some_and(|r| {
                    // Its bounds, x0 y0 x1 y1 in pixels: the centre in the top bar.
                    let c = Pos2::new(
                        ((r[0] + r[2]) / 2.0 / ppp) as f32,
                        ((r[1] + r[3]) / 2.0 / ppp) as f32,
                    );
                    top.contains(c)
                })
        });
    let focus_rect = app
        .view
        .focus
        .path
        .last()
        .and_then(|id| app.keyboard.place_of(id))
        .map(|p| p.rect);
    Seen {
        focus: app.view.focus.path.clone(),
        mode: app.view.focus.mode,
        rings,
        focus_rect,
        breadcrumb,
    }
}

/// No ring and no breadcrumb path, the focus on `want`.
fn check_hidden(s: &Seen, want: &[String], what: &str) {
    assert_eq!(s.focus, want, "{what}: the focus");
    assert!(s.rings.is_empty(), "{what}: a ring drawn: {:?}", s.rings);
    assert!(!s.breadcrumb, "{what}: the breadcrumb's path drawn");
}

/// One ring, in `colour`, on the focused scope `want`, and the breadcrumb drawn.
fn check_shown(s: &Seen, want: &[String], colour: Color32, what: &str) {
    assert_eq!(s.focus, want, "{what}: the focus");
    let at = s
        .focus_rect
        .unwrap_or_else(|| panic!("{what}: the focus not placed"));
    assert_eq!(s.rings.len(), 1, "{what}: rings {:?}", s.rings);
    let (ring, c) = s.rings[0];
    assert_eq!(c, colour, "{what}: the ring's colour");
    assert!(
        at.expand(2.0).contains_rect(ring) && ring.expand(2.0).contains_rect(at),
        "{what}: the ring {ring:?} is not on the focus {at:?}"
    );
    assert!(s.breadcrumb, "{what}: no breadcrumb path drawn");
}

/// Tabs until the focus is the big scope `big`.
fn tab_to(h: &mut Headless, app: &mut App<MockSide>, big: &str) {
    for _ in 0..16 {
        if app.view.focus.path == [big] {
            return;
        }
        press(h, app, Key::Tab, NONE);
    }
    panic!("Tab never reached {big}");
}

/// Reaches scope `id` by keys: Tab to its big scope, then at each level Enter and the arrows.
fn reach(h: &mut Headless, app: &mut App<MockSide>, id: &str) {
    let target = path_to(app, id);
    assert!(!target.is_empty(), "no scope {id}");
    tab_to(h, app, &target[0]);
    for depth in 1..target.len() {
        press(h, app, Key::Enter, NONE);
        settle(h, app);
        for _ in 0..40 {
            if app.view.focus.path.get(depth) == Some(&target[depth]) {
                break;
            }
            let before = app.view.focus.path.clone();
            press(h, app, Key::ArrowRight, NONE);
            if app.view.focus.path == before {
                press(h, app, Key::ArrowDown, NONE);
            }
        }
        assert_eq!(
            app.view.focus.path[..=depth],
            target[..=depth],
            "the keys to {id}"
        );
    }
}

fn landing(app: &App<MockSide>, window: Window) -> Vec<String> {
    app.keyboard
        .tree(app.view.mode)
        .window_landing(windows::entry(window).scope)
}

#[test]
fn qa_it_starts_off_and_a_key_turns_it_on_outside_the_contract() {
    let (mut app, mut h) = fresh();
    assert!(!app.keyboard.navigating(), "the GUI starts with it on");
    let start = seen(&mut h, &mut app);
    check_hidden(&start, &[], "at start");
    // Enter at start, no focus set: it turns keyboard navigation on and acts on nothing, so draws nothing.
    // ViewUI is the contract's UI state; its keyboard part is the focus, and its mode (gui_state_contract §2).
    let view0 = (app.view.focus.clone(), app.view.mode);
    let snap0 = app.snapshot().clone();
    let log0 = app.console().len();
    press(&mut h, &mut app, Key::Enter, NONE);
    settle(&mut h, &mut app);
    assert!(app.keyboard.navigating(), "Enter left it off");
    assert_eq!(
        (app.view.focus.clone(), app.view.mode),
        view0,
        "Enter at start changed ViewUI"
    );
    check_hidden(&seen(&mut h, &mut app), &[], "Enter at start");
    // The flag is the GUI's own: it changed, and ViewUI, the snapshot and the log did not (gui_state_contract §2).
    assert_eq!(app.snapshot(), &snap0, "turning it on changed the snapshot");
    assert_eq!(app.console().len(), log0, "turning it on logged");
    // Control: the next Tab lands on the top bar with the ring.
    press(&mut h, &mut app, Key::Tab, NONE);
    let tabbed = seen(&mut h, &mut app);
    check_shown(&tabbed, &owned(&["top_bar"]), NAVIGATION, "Tab");
    rejects("a GUI that starts with the ring", || {
        check_hidden(&tabbed, &owned(&["top_bar"]), "Tab")
    });
    // Control of the ViewUI check: the Tab, which moves the focus, is seen there.
    rejects("a key that changed ViewUI", || {
        assert_eq!(
            (app.view.focus.clone(), app.view.mode),
            view0,
            "the Tab changed ViewUI"
        )
    });
}

#[test]
fn qa_any_key_turns_it_on_where_the_focus_is() {
    // With the focus on the Figure and the ring hidden by a click, an unbound key shows it there, orange.
    let (mut app, mut h) = fresh();
    tab_to(&mut h, &mut app, FIGURE);
    let figure = layout(&h).figure;
    click(&mut h, &mut app, figure.center());
    let clicked = seen(&mut h, &mut app);
    check_hidden(&clicked, &owned(&[FIGURE]), "a click on the Figure");
    press(&mut h, &mut app, UNBOUND, NONE);
    let keyed = seen(&mut h, &mut app);
    check_shown(&keyed, &owned(&[FIGURE]), NAVIGATION, "an unbound key");
    rejects("no ring after the key", || {
        check_shown(&clicked, &owned(&[FIGURE]), NAVIGATION, "the key")
    });
    rejects("the ring kept through the click", || {
        check_hidden(&keyed, &owned(&[FIGURE]), "the click")
    });
}

/// The four kinds of entry point, each opened with keyboard navigation already on, then by the mouse.
fn mouse_open(h: &mut Headless, app: &mut App<MockSide>, opener: &str) {
    if opener == footer::FOOTER {
        let f = layout(h).footer;
        click(h, app, Pos2::new(f.min.x + 40.0, f.center().y));
        settle(h, app);
        return;
    }
    let tree = app.keyboard.tree(app.view.mode);
    if let Some(menu) = tree
        .parent(opener)
        .filter(|m| tree.get(m).is_some_and(|s| s.menu))
    {
        click_on(h, app, menu);
        settle(h, app);
    }
    click_on(h, app, opener);
    settle(h, app);
}

#[test]
fn qa_a_mouse_open_leaves_it_off_whatever_it_was() {
    let chart_builder = REGISTRY
        .iter()
        .find(|e| e.window == Window::ChartBuilder)
        .unwrap();
    for (name, opener, window) in [
        ("the footer", footer::FOOTER, Window::Console),
        ("the top bar's Profiler…", "profiler", Window::Profiler),
        (
            "Windows › Chart builder",
            chart_builder.menu_entry,
            Window::ChartBuilder,
        ),
        ("IC Inspector…", trajectory::INSPECTOR, Window::Inspector),
    ] {
        // Keyboard navigation on first, the ring on the top bar; then the mouse opens the window.
        let (mut app, mut h) = fresh();
        press(&mut h, &mut app, Key::Tab, NONE);
        check_shown(
            &seen(&mut h, &mut app),
            &owned(&["top_bar"]),
            NAVIGATION,
            "Tab",
        );
        mouse_open(&mut h, &mut app, opener);
        assert!(app.is_open(window), "{name}: not opened by the mouse");
        let want = landing(&app, window);
        let by_mouse = seen(&mut h, &mut app);
        check_hidden(&by_mouse, &want, name);
        // Control: opened from the keyboard, the ring on the landing.
        let (mut app, mut h) = fresh();
        if opener == footer::FOOTER {
            press(&mut h, &mut app, Key::Tab, SHIFT);
            assert_eq!(app.view.focus.path, [footer::FOOTER], "Shift+Tab");
        } else {
            reach(&mut h, &mut app, opener);
        }
        press(&mut h, &mut app, Key::Enter, NONE);
        settle(&mut h, &mut app);
        assert!(app.is_open(window), "{name}: not opened by keys");
        let by_keys = seen(&mut h, &mut app);
        check_shown(&by_keys, &want, NAVIGATION, &format!("{name} by keys"));
        rejects(&format!("{name}: the ring at a mouse open"), || {
            check_hidden(&by_keys, &want, name)
        });
        rejects(&format!("{name}: no ring at a keyboard open"), || {
            check_shown(&by_mouse, &want, NAVIGATION, name)
        });
    }
}

/// Presses Esc until `window` closes, at most four times: how many it took, and what the frame then shows.
fn escs_to_close(h: &mut Headless, app: &mut App<MockSide>, window: Window) -> (usize, Seen) {
    for n in 1..=4 {
        let _ = press(h, app, Key::Escape, NONE);
        settle(h, app);
        if !app.is_open(window) {
            return (n, seen(h, app));
        }
    }
    panic!("four Escs did not close {window:?}");
}

#[test]
fn qa_esc_closes_a_mouse_opened_window_as_from_the_keyboard_and_shows_the_ring_on_its_opener() {
    for (opener, window) in [
        (footer::FOOTER, Window::Console),
        ("profiler", Window::Profiler),
    ] {
        // Opened by keys: the Escs that close it from its landing.
        let (mut app, mut h) = fresh();
        if opener == footer::FOOTER {
            let _ = press(&mut h, &mut app, Key::Tab, SHIFT);
        } else {
            reach(&mut h, &mut app, opener);
        }
        let _ = press(&mut h, &mut app, Key::Enter, NONE);
        settle(&mut h, &mut app);
        assert!(app.is_open(window), "{opener}: not opened by keys");
        let (by_keys, _) = escs_to_close(&mut h, &mut app, window);
        // Opened by the mouse: Esc acts with keyboard navigation off, no press first needed to show the ring, so the
        // same Escs close it (R-460 H2's "so Esc still closes it"; R-464 M2).
        let (mut app, mut h) = fresh();
        mouse_open(&mut h, &mut app, opener);
        assert!(app.is_open(window), "{opener}: not opened");
        assert!(
            !app.keyboard.navigating(),
            "{opener}: a mouse open left it on"
        );
        let opened = seen(&mut h, &mut app);
        let (by_mouse, closed) = escs_to_close(&mut h, &mut app, window);
        assert_eq!(
            by_mouse, by_keys,
            "{opener}: Escs to close it opened by the mouse, and by keys"
        );
        if window == Window::Console {
            assert_eq!(by_mouse, 1, "the console's landing: one Esc closes it");
        }
        // Esc is a key press: the ring shows where the focus now is, the opener (R-464 M2).
        let back = path_to(&app, opener);
        check_shown(&closed, &back, NAVIGATION, "after Esc");
        rejects("the window still open", || {
            check_shown(&opened, &back, NAVIGATION, "after Esc")
        });
    }
}

/// The console opened from the keyboard (Shift+Tab to the footer, Enter), a warning logged so the log has an entry.
fn console_by_keys() -> Rig {
    let (mut app, mut h) = fresh();
    app.side().engine().raise_warning();
    settle(&mut h, &mut app);
    press(&mut h, &mut app, Key::Tab, SHIFT);
    press(&mut h, &mut app, Key::Enter, NONE);
    settle(&mut h, &mut app);
    assert!(app.console_open, "the console opened");
    let want = landing(&app, Window::Console);
    check_shown(&seen(&mut h, &mut app), &want, NAVIGATION, "the console");
    (app, h)
}

#[test]
fn qa_enter_after_a_click_on_copy_shows_the_ring_and_copies_nothing() {
    let (mut app, mut h) = console_by_keys();
    click_on(&mut h, &mut app, console::COPY);
    let copy = path_to(&app, console::COPY);
    let clicked = seen(&mut h, &mut app);
    assert_eq!(clicked.mode, NAV, "the click on Copy");
    check_hidden(&clicked, &copy, "a click on Copy");
    let copied = press(&mut h, &mut app, Key::Enter, NONE);
    assert!(!copied, "the first Enter after the click copied");
    let shown = seen(&mut h, &mut app);
    check_shown(&shown, &copy, NAVIGATION, "Enter after the click");
    // Control: the next Enter acts, as Enter on Copy does.
    assert!(
        press(&mut h, &mut app, Key::Enter, NONE),
        "the second Enter did not copy"
    );
    rejects("a ring drawn at the click", || {
        check_hidden(&shown, &copy, "a click")
    });
}

/// After a click that leaves the focus where it was, `key` acts exactly as from that focus with the ring on, and shows
/// the ring where the focus lands.
fn after_a_click(
    key: Key,
    modifiers: Modifiers,
    setup: impl Fn(&mut Headless, &mut App<MockSide>),
) -> (Seen, Seen) {
    // The control: the same key with the ring on, no click.
    let (mut app, mut h) = fresh();
    setup(&mut h, &mut app);
    press(&mut h, &mut app, key, modifiers);
    settle(&mut h, &mut app);
    let keyed = seen(&mut h, &mut app);
    // The script: a click on the figure first, which moves no focus (R-456 B6).
    let (mut app, mut h) = fresh();
    setup(&mut h, &mut app);
    let before = app.view.focus.path.clone();
    let figure = layout(&h).figure;
    click(&mut h, &mut app, figure.center());
    let clicked = seen(&mut h, &mut app);
    check_hidden(&clicked, &before, "the click on the figure");
    press(&mut h, &mut app, key, modifiers);
    settle(&mut h, &mut app);
    (keyed, seen(&mut h, &mut app))
}

#[test]
fn qa_after_a_click_each_key_acts_as_always_and_shows_the_ring() {
    let on_top_bar = |h: &mut Headless, app: &mut App<MockSide>| tab_to(h, app, "top_bar");
    let in_top_bar = |h: &mut Headless, app: &mut App<MockSide>| {
        tab_to(h, app, "top_bar");
        press(h, app, Key::Enter, NONE);
    };
    for (what, key, modifiers, in_scope) in [
        ("Tab", Key::Tab, NONE, false),
        ("Shift+Tab", Key::Tab, SHIFT, false),
        ("→", Key::ArrowRight, NONE, true),
        ("↓", Key::ArrowDown, NONE, false),
    ] {
        let (keyed, after) = if in_scope {
            after_a_click(key, modifiers, in_top_bar)
        } else {
            after_a_click(key, modifiers, on_top_bar)
        };
        // The key moved on from the focus the click left, as it does with the ring on.
        check_shown(&keyed, &keyed.focus, NAVIGATION, what);
        check_shown(&after, &keyed.focus, NAVIGATION, what);
        rejects(&format!("{what}: a GUI with no ring after it"), || {
            check_hidden(&after, &keyed.focus, what)
        });
    }
    // `?` opens the shortcuts and turns it on.
    let (mut app, mut h) = fresh();
    tab_to(&mut h, &mut app, "top_bar");
    let figure = layout(&h).figure;
    click(&mut h, &mut app, figure.center());
    assert!(!app.keyboard.navigating(), "the click left it on");
    press(&mut h, &mut app, Key::Questionmark, SHIFT);
    assert!(app.keyboard.shortcuts_open, "? did not open the shortcuts");
    assert!(app.keyboard.navigating(), "? left it off");
    press(&mut h, &mut app, Key::Escape, NONE);
    assert!(!app.keyboard.shortcuts_open, "Esc did not close them");
}

#[test]
fn qa_a_letter_after_a_click_on_the_figure_acts_and_shows_the_ring() {
    // L on the Figure scope toggles listening (§G3's letter shortcuts act in either mode).
    let (mut app, mut h) = fresh();
    tab_to(&mut h, &mut app, FIGURE);
    let figure = layout(&h).figure;
    click(&mut h, &mut app, figure.center());
    let clicked = seen(&mut h, &mut app);
    check_hidden(&clicked, &owned(&[FIGURE]), "a click on the Figure");
    let listen = app.trajectory.listen;
    press(&mut h, &mut app, Key::L, NONE);
    assert_ne!(app.trajectory.listen, listen, "L did not act");
    let shown = seen(&mut h, &mut app);
    check_shown(&shown, &owned(&[FIGURE]), NAVIGATION, "L after the click");
    rejects("no ring after the letter", || {
        check_shown(&clicked, &owned(&[FIGURE]), NAVIGATION, "L")
    });
}

#[test]
fn qa_a_secondary_click_and_a_title_bar_drag_hide_it_a_scroll_and_a_hover_do_not() {
    // A secondary click on the legend.
    let (mut app, mut h) = fresh();
    tab_to(&mut h, &mut app, "top_bar");
    let legend = layout(&h).legend;
    click_with(&mut h, &mut app, legend.center(), PointerButton::Secondary);
    assert!(
        !app.keyboard.navigating(),
        "a secondary click left keyboard navigation on"
    );
    let right = seen(&mut h, &mut app);
    check_hidden(&right, &owned(&["top_bar"]), "a secondary click");
    // A drag on the Profiler's title bar, the window opened by keys.
    let (mut app, mut h) = fresh();
    reach(&mut h, &mut app, "profiler");
    press(&mut h, &mut app, Key::Enter, NONE);
    settle(&mut h, &mut app);
    let scope = windows::entry(Window::Profiler).scope;
    let want = landing(&app, Window::Profiler);
    let before = seen(&mut h, &mut app);
    check_shown(&before, &want, NAVIGATION, "the Profiler by keys");
    let win = place(&app, scope);
    let from = Pos2::new(win.center().x, win.min.y + 8.0);
    for events in [
        vec![Event::PointerMoved(from)],
        vec![button(from, PointerButton::Primary, true)],
        vec![Event::PointerMoved(from + Vec2::new(-20.0, 10.0))],
        vec![Event::PointerMoved(from + Vec2::new(-40.0, 20.0))],
        vec![button(
            from + Vec2::new(-40.0, 20.0),
            PointerButton::Primary,
            false,
        )],
        vec![Event::PointerGone],
    ] {
        let _ = h.frame(&mut app, events);
    }
    settle(&mut h, &mut app);
    let dragged = seen(&mut h, &mut app);
    assert!(
        app.is_open(Window::Profiler),
        "the drag closed the Profiler"
    );
    check_hidden(&dragged, &want, "a drag on the title bar");
    rejects("the ring kept through the drag", || {
        check_hidden(&before, &want, "a drag")
    });
    // A hover and a scroll over the figure leave the ring on.
    let (mut app, mut h) = fresh();
    tab_to(&mut h, &mut app, "top_bar");
    let figure = layout(&h).figure;
    for events in [
        vec![Event::PointerMoved(figure.center())],
        vec![Event::PointerMoved(figure.center() + Vec2::new(30.0, 10.0))],
        vec![Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: Vec2::new(0.0, 40.0),
            phase: egui::TouchPhase::Move,
            modifiers: NONE,
        }],
        vec![Event::PointerGone],
    ] {
        let _ = h.frame(&mut app, events);
    }
    settle(&mut h, &mut app);
    let scrolled = seen(&mut h, &mut app);
    check_shown(
        &scrolled,
        &owned(&["top_bar"]),
        NAVIGATION,
        "a hover and a scroll",
    );
    // Control: a click there hides it.
    click(&mut h, &mut app, figure.center());
    let clicked = seen(&mut h, &mut app);
    rejects("a click that keeps the ring", || {
        check_shown(&clicked, &owned(&["top_bar"]), NAVIGATION, "a click")
    });
}

#[test]
fn qa_a_clicked_search_field_shows_its_ring_on_the_first_key() {
    let search = |app: &App<MockSide>| path_to(app, console::SEARCH);
    // A letter: it goes in and shows the ring blue on the field.
    let (mut app, mut h) = console_by_keys();
    click_on(&mut h, &mut app, console::SEARCH);
    let clicked = seen(&mut h, &mut app);
    assert_eq!(clicked.mode, INTERACT, "a click on the field does not type");
    check_hidden(&clicked, &search(&app), "a click on the search field");
    type_letter(&mut h, &mut app, 'q');
    assert_eq!(app.windows.console.text, "q", "the letter not in the field");
    let typed = seen(&mut h, &mut app);
    let b = blue(&h);
    check_shown(&typed, &search(&app), b, "a letter");
    rejects("a ring drawn at the click", || {
        check_shown(&clicked, &search(&app), b, "the click")
    });
    rejects("an orange ring while typing", || {
        check_shown(&typed, &search(&app), NAVIGATION, "typing")
    });
    // Esc or Enter first: typing stops, the text kept, the ring orange on the field.
    for k in [Key::Escape, Key::Enter] {
        let (mut app, mut h) = console_by_keys();
        click_on(&mut h, &mut app, console::SEARCH);
        type_letter(&mut h, &mut app, 'q');
        // A second click on the field hides the ring again, still typing.
        click_on(&mut h, &mut app, console::SEARCH);
        check_hidden(&seen(&mut h, &mut app), &search(&app), "the second click");
        let _ = press(&mut h, &mut app, k, NONE);
        let after = seen(&mut h, &mut app);
        assert_eq!(after.mode, NAV, "{k:?} did not stop typing");
        assert_eq!(app.windows.console.text, "q", "{k:?} lost the text");
        check_shown(&after, &search(&app), NAVIGATION, &format!("{k:?}"));
    }
    // Tab first: it leaves the field, the ring orange where the focus lands.
    let (mut app, mut h) = console_by_keys();
    click_on(&mut h, &mut app, console::SEARCH);
    let _ = press(&mut h, &mut app, Key::Tab, NONE);
    let after = seen(&mut h, &mut app);
    assert_ne!(after.focus, search(&app), "Tab stayed on the field");
    check_shown(
        &after,
        &after.focus,
        NAVIGATION,
        "Tab from the clicked field",
    );
}

#[test]
fn qa_the_console_opening_on_an_error_leaves_it_as_it_was() {
    // Off: the console opens by itself, no ring.
    let (mut app, mut h) = fresh();
    app.side().engine().raise_error();
    settle(&mut h, &mut app);
    assert!(app.console_open, "an error did not open the console");
    assert!(!app.keyboard.navigating(), "an error turned it on");
    check_hidden(&seen(&mut h, &mut app), &[], "the console on an error");
    // On: the ring stays where it was.
    let (mut app, mut h) = fresh();
    tab_to(&mut h, &mut app, FIGURE);
    app.side().engine().raise_error();
    settle(&mut h, &mut app);
    assert!(app.console_open, "an error did not open the console");
    let on = seen(&mut h, &mut app);
    check_shown(
        &on,
        &owned(&[FIGURE]),
        NAVIGATION,
        "the console on an error",
    );
    rejects("a GUI that hides it on an error", || {
        check_hidden(&on, &owned(&[FIGURE]), "an error")
    });
}
