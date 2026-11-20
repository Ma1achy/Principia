//! `mock_keyboard`, extended with TASK-M6-28's scopes (REQ-GUI-169 re-checked for the windows, REQ-GUI-178; R-405,
//! R-409; render_gui_spec §G3, §G5, §G12): each window, opened from its Windows menu entry by the keys, takes the focus
//! on its first section's first element in navigation mode; Tab and Shift+Tab move between its sections and stay in
//! it; the arrows walk a section's elements, so Enter reaches each; Enter on a button acts as its click, on a value or
//! a text field starts interaction mode, and Esc comes back out; Esc in navigation mode goes up a scope, and from a
//! section closes the window, the focus back on its entry. The footer is big scope 8, Enter on it opens the console,
//! and the console keeps its keys as R-405 and R-409 F2, G4–G6 give them. Each check also runs on an input it must
//! reject (R-176, through `rejects`).

use eframe::egui::{Event, Key, Modifiers};
use engine::contract::view_ui::{KeyMode, Window};

use super::support::{focus, frame_texts, headless, mock_app, press, rejects};
use crate::app::App;
use crate::headless::Headless;
use crate::keyboard::scopes::{Arrows, ScopeTree};
use crate::side::{EngineSide, MockSide};
use crate::windows::{console, run, Entry, REGISTRY};

const NONE: Modifiers = Modifiers::NONE;
const SHIFT: Modifiers = Modifiers::SHIFT;
const ALT: Modifiers = Modifiers::ALT;
const NAV: KeyMode = KeyMode::Navigation;
const INTERACT: KeyMode = KeyMode::Interaction;

/// A focus path and its mode, owned.
type At = (Vec<String>, KeyMode);

fn owned(path: &[&str]) -> Vec<String> {
    path.iter().map(|s| (*s).to_owned()).collect()
}

fn at(app: &App<MockSide>) -> At {
    (owned(&focus(app)), app.view.focus.mode)
}

fn check_at(got: &At, path: &[String], mode: KeyMode, what: &str) {
    assert_eq!(got, &(path.to_vec(), mode), "{what}");
}

fn settle(h: &mut Headless, app: &mut App<MockSide>) {
    for _ in 0..3 {
        let _ = h.frame(app, Vec::new());
    }
}

fn fresh() -> (App<MockSide>, Headless) {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    (app, h)
}

fn tree(app: &App<MockSide>) -> &ScopeTree {
    app.keyboard.tree(app.view.mode)
}

/// Presses `key` `n` times.
fn presses(h: &mut Headless, app: &mut App<MockSide>, key: Key, modifiers: Modifiers, n: usize) {
    for _ in 0..n {
        press(h, app, key, modifiers);
    }
}

/// Opens `entry`'s window by the keys: Tab to the top bar, Enter, → to Windows, Enter, ↓ to the entry, Enter.
fn open_by_keys(h: &mut Headless, app: &mut App<MockSide>, entry: &Entry) {
    app.view.focus.path.clear();
    press(h, app, Key::Tab, NONE);
    press(h, app, Key::Enter, NONE);
    let controls = tree(app).children(Some("top_bar"));
    let windows = controls.iter().position(|c| *c == "windows").unwrap();
    let first = controls
        .iter()
        .position(|c| focus(app).last() == Some(c))
        .unwrap();
    presses(h, app, Key::ArrowRight, NONE, windows - first);
    press(h, app, Key::Enter, NONE);
    let entries = tree(app).children(Some("windows"));
    let k = entries.iter().position(|e| *e == entry.menu_entry).unwrap();
    presses(h, app, Key::ArrowDown, NONE, k);
    assert_eq!(focus(app), ["top_bar", "windows", entry.menu_entry]);
    press(h, app, Key::Enter, NONE);
    settle(h, app);
}

/// The breadcrumb drawn: the name in the frame that starts with `first`.
fn breadcrumb(h: &mut Headless, app: &mut App<MockSide>, first: &str) -> Option<String> {
    frame_texts(h, app)
        .into_iter()
        .find(|t| t.starts_with(&format!("{first} ›")))
}

/// The controls Enter must not press in the walk: they leave the page (present mode) or change the walk's scopes.
const SKIP_ENTER: [&str; 4] = [
    crate::windows::export::PRESENT,
    crate::windows::chart_builder::SAVE,
    crate::windows::chart_builder::DUPLICATE,
    crate::windows::chart_builder::DELETE,
];

/// Walks `entry`'s window by the keys: the landing, each section by Tab, each element by ↓ with Enter (and Esc) on
/// it, Tab wrapping to the first section, Shift+Tab back, then Esc up and out. Returns the focus after each step that
/// moved it, for the controls.
fn walk(entry: &Entry) -> Vec<At> {
    let (mut app, mut h) = fresh();
    let h = &mut h;
    let app = &mut app;
    open_by_keys(h, app, entry);
    let mut seen = vec![at(app)];
    assert!(app.is_open(entry.window), "{:?} open", entry.window);
    let sections = tree(app).children(Some(entry.scope));
    assert!(!sections.is_empty(), "{:?} has sections", entry.window);
    for (k, section) in sections.iter().enumerate() {
        let landing = tree(app).section_landing(entry.scope, section);
        check_at(
            &at(app),
            &landing,
            NAV,
            &format!("{:?}: section {k}'s landing", entry.window),
        );
        let elements = tree(app).children(Some(section));
        for (i, element) in elements.iter().enumerate() {
            if i > 0 {
                press(h, app, Key::ArrowDown, NONE);
            }
            let here = owned(&[entry.scope, section, element]);
            check_at(&at(app), &here, NAV, &format!("↓ to {element}"));
            seen.push(at(app));
            // The ring is drawn on it, and the breadcrumb names its path.
            let _ = h.frame(app, Vec::new());
            assert!(app.keyboard.place_of(element).is_some(), "{element} drawn");
            // Scrolled into view: drawn inside the window.
            let _ = h.frame(app, Vec::new());
            let place = |id: &str| app.keyboard.place_of(id).map(|p| p.rect);
            let (drawn, window) = (place(element).unwrap(), place(entry.scope).unwrap());
            assert!(
                window.contains_rect(drawn),
                "{element}: {drawn:?} outside {window:?}"
            );
            let crumb = breadcrumb(h, app, entry.title).unwrap_or_default();
            assert!(
                crumb.ends_with(tree(app).get(element).unwrap().label),
                "{crumb}"
            );
            if SKIP_ENTER.contains(element) {
                continue;
            }
            let scope = *tree(app).get(element).unwrap();
            press(h, app, Key::Enter, NONE);
            let into = matches!(scope.arrows, Arrows::Adjust(_)) || scope.text;
            let mode = if into { INTERACT } else { NAV };
            check_at(&at(app), &here, mode, &format!("Enter on {element}"));
            if into {
                press(h, app, Key::Escape, NONE);
                check_at(&at(app), &here, NAV, &format!("Esc on {element}"));
            }
        }
        // Back to the first element, then Tab to the next section.
        presses(h, app, Key::ArrowUp, NONE, elements.len());
        press(h, app, Key::Tab, NONE);
    }
    // Tab wrapped to the first section; Shift+Tab goes back to the last, staying in the window.
    let first = tree(app).section_landing(entry.scope, sections[0]);
    check_at(&at(app), &first, NAV, "Tab wraps in the window");
    press(h, app, Key::Tab, SHIFT);
    let last = tree(app).section_landing(entry.scope, sections[sections.len() - 1]);
    check_at(&at(app), &last, NAV, "Shift+Tab wraps back");
    seen.push(at(app));
    // Esc goes up to the section, if the landing is an element, then closes the window, the focus back on its entry.
    if last.len() > 2 {
        press(h, app, Key::Escape, NONE);
        check_at(&at(app), &last[..2], NAV, "Esc up to the section");
        assert!(app.is_open(entry.window));
        // The ring has the section's rect, and the window's, to be drawn on.
        let _ = h.frame(app, Vec::new());
        let place = |id: &str| app.keyboard.place_of(id).map(|p| p.rect);
        let (section, window) = (
            place(&last[1]).expect("the section placed"),
            place(entry.scope).expect("the window placed"),
        );
        assert!(
            window.contains_rect(section),
            "{section:?} outside {window:?}"
        );
    }
    press(h, app, Key::Escape, NONE);
    settle(h, app);
    assert!(!app.is_open(entry.window), "Esc closed {:?}", entry.window);
    check_at(
        &at(app),
        &owned(&["top_bar", "windows", entry.menu_entry]),
        NAV,
        "back on the entry",
    );
    seen.push(at(app));
    seen
}

#[test]
fn mock_keyboard_windows_each_window_is_a_scope_the_keys_walk() {
    // The console's two scopes are R-450's, walked by `mock_console_keys` and `mock_reachability`.
    for entry in REGISTRY.iter().filter(|e| e.window != Window::Console) {
        let seen = walk(entry);
        // Every step but the last stayed in the window.
        for s in &seen[..seen.len() - 1] {
            assert_eq!(s.0[0], entry.scope, "{:?} left the window", s);
        }
        rejects("a walk that left the window", || {
            let s = &seen[seen.len() - 1];
            assert_eq!(s.0[0], entry.scope)
        });
    }
}

/// Run's T_horizon in interaction mode: ↑ a hundredth of its range, ×10 with Shift, ×0.1 with Alt.
#[test]
fn mock_keyboard_windows_arrows_adjust_a_window_value_by_its_base_step() {
    let (mut app, mut h) = fresh();
    open_by_keys(&mut h, &mut app, crate::windows::entry(Window::Run));
    assert_eq!(focus(&app), ["win_run", run::INTEGRATION, run::T_HORIZON]);
    let (lo, hi) = run::T_RANGE;
    let step = 0.01 * (hi - lo);
    // Navigating, ↑ moves nothing.
    press(&mut h, &mut app, Key::ArrowUp, NONE);
    assert_eq!(app.windows.run.t_horizon, run::T_DEFAULT);
    press(&mut h, &mut app, Key::Enter, NONE);
    let mut want = run::T_DEFAULT;
    for (modifiers, times) in [(NONE, 1.0), (SHIFT, 10.0), (ALT, 0.1)] {
        press(&mut h, &mut app, Key::ArrowUp, modifiers);
        want += times * step;
        assert!(
            (app.windows.run.t_horizon - want).abs() < 1e-9,
            "{modifiers:?}"
        );
    }
    let got = app.windows.run.t_horizon;
    rejects("Shift read as one step", || {
        assert!((got - (run::T_DEFAULT + 2.1 * step)).abs() < 1e-9)
    });
}

// --- The footer and the console (R-405; R-450) -----------------------------------------------------------------------
// The console's own keys, its two scopes, are `mock_console_keys`'s (REQ-GUI-189).

/// The console's landing: its header bar scope itself (R-450).
fn header() -> Vec<String> {
    owned(&[console::WINDOW, console::HEADER])
}

/// An app with three warnings logged, the focus on the footer, then Enter.
fn console_by_footer() -> (App<MockSide>, Headless) {
    let (mut app, mut h) = fresh();
    for _ in 0..3 {
        app.side().engine().raise_warning();
        let _ = h.frame(&mut app, Vec::new());
    }
    press(&mut h, &mut app, Key::Tab, SHIFT);
    assert_eq!(
        focus(&app),
        ["footer"],
        "Shift+Tab from no focus starts at the footer"
    );
    press(&mut h, &mut app, Key::Enter, NONE);
    settle(&mut h, &mut app);
    (app, h)
}

#[test]
fn mock_keyboard_windows_enter_on_the_footer_opens_the_console_on_its_header_bar() {
    let (app, _) = console_by_footer();
    assert!(app.is_open(Window::Console));
    let got = at(&app);
    check_at(&got, &header(), NAV, "Enter on the footer");
    rejects("a console that does not take the focus on Enter", || {
        check_at(&(owned(&["footer"]), NAV), &header(), NAV, "Enter")
    });
    rejects("a console landing on All", || {
        check_at(
            &(owned(&[console::WINDOW, console::HEADER, "cs_all"]), NAV),
            &header(),
            NAV,
            "Enter",
        )
    });
    rejects("a console landing in interaction mode", || {
        check_at(&(header(), INTERACT), &header(), NAV, "Enter")
    });
}

#[test]
fn mock_keyboard_windows_esc_closes_the_console_back_to_the_footer() {
    let (mut app, mut h) = console_by_footer();
    press(&mut h, &mut app, Key::Escape, NONE);
    settle(&mut h, &mut app);
    assert!(!app.is_open(Window::Console));
    let got = at(&app);
    check_at(
        &got,
        &owned(&["footer"]),
        NAV,
        "Esc closes, back on the footer",
    );
    rejects(
        "an Esc that returns the focus to Windows › Console",
        || {
            check_at(
                &got,
                &owned(&["top_bar", "windows", "windows_console"]),
                NAV,
                "Esc",
            )
        },
    );
}

#[test]
fn mock_keyboard_windows_console_from_menu() {
    let (mut app, mut h) = fresh();
    let entry = crate::windows::entry(Window::Console);
    open_by_keys(&mut h, &mut app, entry);
    check_at(&at(&app), &header(), NAV, "Windows › Console");
    press(&mut h, &mut app, Key::Tab, NONE);
    check_at(
        &at(&app),
        &owned(&[console::WINDOW, console::LOG]),
        NAV,
        "Tab",
    );
    press(&mut h, &mut app, Key::Escape, NONE);
    settle(&mut h, &mut app);
    assert!(!app.is_open(Window::Console));
    let got = at(&app);
    check_at(
        &got,
        &owned(&["top_bar", "windows", "windows_console"]),
        NAV,
        "back on Windows › Console",
    );
    rejects("an Esc that returns the focus to the footer", || {
        check_at(&owned_at(&["footer"]), &got.0, NAV, "Esc")
    });
}

fn owned_at(path: &[&str]) -> At {
    (owned(path), NAV)
}

#[test]
fn mock_keyboard_windows_a_raised_error_opens_the_console_without_moving_the_focus() {
    let (mut app, mut h) = fresh();
    presses(&mut h, &mut app, Key::Tab, NONE, 4);
    let before = at(&app);
    app.side().engine().raise_error();
    settle(&mut h, &mut app);
    assert!(app.is_open(Window::Console));
    let after = at(&app);
    assert_eq!(after, before, "the focus moved");
    rejects("a console that took the focus", || {
        assert_eq!((header(), NAV), before)
    });
    // Tab goes on among the big scopes, not into the console.
    press(&mut h, &mut app, Key::Tab, NONE);
    assert_eq!(focus(&app).len(), 1);
    assert_ne!(focus(&app)[0], console::WINDOW);
}

// --- The window rule on the tree itself ------------------------------------------------------------------------------

/// A tree of a big scope `a`, a window `w` of sections `s1` (elements `e1`, `e2`) and `s2` (no element), and a window
/// `bare` with no section.
fn window_tree() -> ScopeTree {
    use crate::keyboard::scopes::Scope;
    let mut tree = ScopeTree::new();
    tree.register(None, Scope::group("a", "A"));
    tree.register(None, Scope::window("w", "W"));
    tree.register(Some("w"), Scope::group("s1", "S1"));
    tree.register(Some("s1"), Scope::control("e1", "E1"));
    tree.register(Some("s1"), Scope::control("e2", "E2"));
    tree.register(Some("w"), Scope::group("s2", "S2"));
    tree.register(None, Scope::window("bare", "Bare"));
    tree
}

fn nav(
    tree: &ScopeTree,
    path: &[&str],
    command: crate::keyboard::keymap::Command,
) -> (Vec<String>, crate::keyboard::scopes::Outcome) {
    let mut f = engine::contract::view_ui::Focus {
        path: owned(path),
        mode: NAV,
    };
    let outcome = tree.navigate(&mut f, command, 1.0);
    (f.path, outcome)
}

#[test]
fn mock_keyboard_windows_the_tree_keeps_tab_and_esc_in_a_window() {
    use crate::keyboard::keymap::{Command, Direction};
    use crate::keyboard::scopes::Outcome;
    let tree = window_tree();
    assert_eq!(tree.bigs(), ["a"], "the windows are not big scopes");
    assert_eq!(tree.window_of(&owned(&["w", "s1"])), Some("w"));
    assert_eq!(tree.window_of(&owned(&["a"])), None);
    assert_eq!(tree.window_of(&[]), None);
    assert_eq!(tree.window_landing("w"), ["w", "s1", "e1"]);
    assert_eq!(tree.window_landing("bare"), ["bare"]);
    assert_eq!(tree.section_landing("w", "s2"), ["w", "s2"]);
    // Tab and Shift+Tab: from the window itself, to its first or last section; between sections, wrapping.
    assert_eq!(nav(&tree, &["w"], Command::Next).0, ["w", "s1", "e1"]);
    assert_eq!(nav(&tree, &["w"], Command::Previous).0, ["w", "s2"]);
    assert_eq!(nav(&tree, &["w", "s1", "e2"], Command::Next).0, ["w", "s2"]);
    assert_eq!(nav(&tree, &["w", "s2"], Command::Next).0, ["w", "s1", "e1"]);
    assert_eq!(
        nav(&tree, &["w", "s1", "e1"], Command::Previous).0,
        ["w", "s2"]
    );
    assert_eq!(
        nav(&tree, &["w", "s2"], Command::Previous).0,
        ["w", "s1", "e1"]
    );
    // A window with no section: Tab does nothing.
    assert_eq!(
        nav(&tree, &["bare"], Command::Next),
        (owned(&["bare"]), Outcome::None)
    );
    // Esc: up from an element; from a section or the window, leave it.
    assert_eq!(
        nav(&tree, &["w", "s1", "e2"], Command::Back),
        (owned(&["w", "s1"]), Outcome::Moved)
    );
    assert_eq!(
        nav(&tree, &["w", "s1"], Command::Back).1,
        Outcome::Leave("w")
    );
    assert_eq!(nav(&tree, &["w"], Command::Back).1, Outcome::Leave("w"));
    assert_eq!(
        nav(&tree, &["a"], Command::Back),
        (Vec::new(), Outcome::Moved)
    );
    // The arrows: a window has no siblings; a big scope's are the big scopes alone.
    assert_eq!(
        nav(&tree, &["w"], Command::Arrow(Direction::Down)),
        (owned(&["w"]), Outcome::None)
    );
    assert_eq!(nav(&tree, &["a"], Command::Arrow(Direction::Down)).0, ["a"]);
    assert_eq!(
        nav(&tree, &["w", "s1"], Command::Arrow(Direction::Down)).0,
        ["w", "s2"]
    );
    rejects("Shift+Tab from the window to its first section", || {
        assert_eq!(nav(&tree, &["w"], Command::Previous).0, ["w", "s1", "e1"])
    });
}

/// A focused section scrolls into its window's view: the Chart builder, taller than its window, focused on each section
/// from the far end, places each on a part it shows.
#[test]
fn mock_keyboard_windows_a_focused_section_scrolls_into_view() {
    let (mut app, mut h) = fresh();
    let entry = crate::windows::entry(Window::ChartBuilder);
    let ctx = h.ctx.clone();
    app.open_window(&ctx, Window::ChartBuilder, None);
    settle(&mut h, &mut app);
    let sections: Vec<String> = app
        .keyboard
        .tree(app.view.mode)
        .children(Some(entry.scope))
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    assert!(sections.len() > 2, "{sections:?}");
    let shown = |r: eframe::egui::Rect| r.is_positive();
    let mut order = sections.clone();
    order.reverse();
    order.extend(sections.iter().cloned());
    for section in &order {
        app.view.focus.path = vec![entry.scope.to_owned(), section.clone()];
        // egui animates the scroll; the far end takes some frames to reach.
        for _ in 0..6 {
            settle(&mut h, &mut app);
        }
        let placed = app.keyboard.place_of(section).expect("placed").rect;
        assert!(shown(placed), "{section}: {placed:?} out of view");
    }
    rejects("a section scrolled out of view", || {
        assert!(shown(eframe::egui::Rect::from_min_max(
            eframe::egui::pos2(0.0, 30.0),
            eframe::egui::pos2(100.0, 10.0)
        )))
    });
}

/// The focus scrolls its window only as it moves there: the wheel then scrolls the window away from the focused element,
/// and the window stays where the wheel left it (code review 5529823383, finding 1).
#[test]
fn mock_keyboard_windows_the_wheel_scrolls_a_window_away_from_its_focus() {
    let (mut app, mut h) = fresh();
    let entry = crate::windows::entry(Window::ChartBuilder);
    let ctx = h.ctx.clone();
    app.open_window(&ctx, Window::ChartBuilder, None);
    settle(&mut h, &mut app);
    let landing = app.view.focus.path.clone();
    let element = landing.last().cloned().expect("a landing");
    let top = |app: &App<MockSide>| app.keyboard.place_of(&element).expect("placed").rect.min.y;
    let before = top(&app);
    let window = app.keyboard.place_of(entry.scope).expect("placed").rect;
    let _ = h.frame(&mut app, vec![Event::PointerMoved(window.center())]);
    for _ in 0..3 {
        let _ = h.frame(
            &mut app,
            vec![Event::MouseWheel {
                unit: eframe::egui::MouseWheelUnit::Line,
                delta: eframe::egui::vec2(0.0, -5.0),
                phase: eframe::egui::TouchPhase::Move,
                modifiers: Modifiers::NONE,
            }],
        );
    }
    for _ in 0..4 {
        settle(&mut h, &mut app);
    }
    assert_eq!(app.view.focus.path, landing, "the focus stays");
    let after = top(&app);
    let scrolled = |before: f32, after: f32| assert!(after < before - 50.0, "{before} → {after}");
    scrolled(before, after);
    rejects("a window snapped back to its focus", || {
        scrolled(before, before)
    });
}
