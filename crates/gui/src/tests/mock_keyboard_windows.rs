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
    for entry in &REGISTRY {
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

// --- The footer and the console (R-405; R-409 F2, G4–G6) -------------------------------------------------------------

/// The console's landing: its first filter, `all`.
fn first_filter() -> Vec<String> {
    owned(&[console::WINDOW, console::FILTERS, "cs_all"])
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
fn mock_keyboard_windows_enter_on_the_footer_opens_the_console_on_its_first_filter() {
    let (app, _) = console_by_footer();
    assert!(app.is_open(Window::Console));
    let got = at(&app);
    check_at(&got, &first_filter(), NAV, "Enter on the footer");
    rejects("a console that does not take the focus on Enter", || {
        check_at(&(owned(&["footer"]), NAV), &first_filter(), NAV, "Enter")
    });
    rejects("a console landing in interaction mode", || {
        check_at(&(first_filter(), INTERACT), &first_filter(), NAV, "Enter")
    });
}

/// The console's sections, by their landings, in Tab order.
fn sections() -> [Vec<String>; 4] {
    [
        first_filter(),
        owned(&[console::WINDOW, console::TEXT]),
        owned(&[console::WINDOW, console::COPY_CLEAR, console::COPY]),
        owned(&[console::WINDOW, console::LIST]),
    ]
}

#[test]
fn mock_keyboard_windows_tab_moves_between_the_consoles_sections_inside_it() {
    let (mut app, mut h) = console_by_footer();
    let s = sections();
    let mut seen = Vec::new();
    for _ in 0..s.len() {
        press(&mut h, &mut app, Key::Tab, NONE);
        seen.push(at(&app));
    }
    for (i, got) in seen.iter().enumerate() {
        check_at(got, &s[(i + 1) % s.len()], NAV, &format!("Tab {}", i + 1));
    }
    for i in (0..s.len()).rev() {
        press(&mut h, &mut app, Key::Tab, SHIFT);
        check_at(&at(&app), &s[i], NAV, "Shift+Tab");
    }
    rejects("a Tab that left the console", || {
        check_at(&(owned(&["top_bar"]), NAV), &s[1], NAV, "Tab")
    });
    // The arrows move within a section: the filters, then copy and clear.
    let (mut app, mut h) = console_by_footer();
    press(&mut h, &mut app, Key::ArrowRight, NONE);
    check_at(
        &at(&app),
        &owned(&[console::WINDOW, console::FILTERS, "cs_warnings"]),
        NAV,
        "→",
    );
    press(&mut h, &mut app, Key::Enter, NONE);
    assert_eq!(
        app.windows.console.filter,
        console::Filter::Warnings,
        "Enter acts as the click"
    );
    check_at(
        &at(&app),
        &owned(&[console::WINDOW, console::FILTERS, "cs_warnings"]),
        NAV,
        "Enter on a filter",
    );
    presses(&mut h, &mut app, Key::ArrowRight, NONE, 5);
    check_at(
        &at(&app),
        &owned(&[console::WINDOW, console::FILTERS, "cs_info"]),
        NAV,
        "→ stops at info",
    );
}

#[test]
fn mock_keyboard_windows_enter_on_clear_clears_the_console_and_the_counts() {
    let (mut app, mut h) = console_by_footer();
    assert!(!app.console().is_empty());
    assert_eq!(app.counts().warnings, 3);
    presses(&mut h, &mut app, Key::Tab, NONE, 2);
    press(&mut h, &mut app, Key::ArrowRight, NONE);
    let clear = owned(&[console::WINDOW, console::COPY_CLEAR, console::CLEAR]);
    check_at(&at(&app), &clear, NAV, "→ to clear");
    press(&mut h, &mut app, Key::Enter, NONE);
    assert!(app.console().is_empty(), "cleared");
    assert_eq!(
        app.counts(),
        crate::app::Counts::default(),
        "the counts reset"
    );
    check_at(&at(&app), &clear, NAV, "the focus kept on clear");
    rejects("a console left full", || assert_eq!(app.console().len(), 3));
}

/// Types `text` as a keyboard would: each character's key press with its text, then the release.
fn type_text(h: &mut Headless, app: &mut App<MockSide>, text: &str) {
    for c in text.chars() {
        let key = Key::from_name(&c.to_string()).unwrap_or(Key::Space);
        let modifiers = if c == '?' { SHIFT } else { NONE };
        let down = Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        };
        let _ = h.frame(app, vec![down, Event::Text(c.to_string())]);
        let up = Event::Key {
            key,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers,
        };
        let _ = h.frame(app, vec![up]);
    }
}

/// Nothing was typed.
fn check_untyped(text: &str) {
    assert_eq!(text, "", "a letter typed on landing");
}

/// The text typed was kept.
fn check_kept(text: &str, typed: &str) {
    assert_eq!(text, typed, "the text not kept");
}

/// The contract's history kept its depth.
fn check_no_undo(after: u32, before: u32) {
    assert_eq!(after, before, "Ctrl+Z reached the contract's undo");
}

/// The console with the focus on its text filter, navigating.
fn on_text() -> (App<MockSide>, Headless) {
    let (mut app, mut h) = console_by_footer();
    press(&mut h, &mut app, Key::Tab, NONE);
    check_at(&at(&app), &sections()[1], NAV, "Tab to the text filter");
    (app, h)
}

#[test]
fn mock_keyboard_windows_the_text_filter_types_only_after_enter() {
    let (mut app, mut h) = on_text();
    type_text(&mut h, &mut app, "a");
    check_untyped(&app.windows.console.text);
    rejects("a text filter that types on landing", || check_untyped("a"));
    press(&mut h, &mut app, Key::Enter, NONE);
    check_at(&at(&app), &sections()[1], INTERACT, "Enter starts typing");
    type_text(&mut h, &mut app, "ab c");
    press(&mut h, &mut app, Key::ArrowLeft, NONE);
    press(&mut h, &mut app, Key::ArrowDown, NONE);
    assert_eq!(
        app.windows.console.text, "ab c",
        "letters and Space went to the field"
    );
    check_at(
        &at(&app),
        &sections()[1],
        INTERACT,
        "the arrows are the field's",
    );
    press(&mut h, &mut app, Key::Escape, NONE);
    check_at(&at(&app), &sections()[1], NAV, "Esc back to navigation");
    assert!(
        app.is_open(Window::Console),
        "Esc while typing closed the console"
    );
    check_kept(&app.windows.console.text, "ab c");
    press(&mut h, &mut app, Key::Enter, NONE);
    press(&mut h, &mut app, Key::Enter, NONE);
    check_at(&at(&app), &sections()[1], NAV, "Enter back to navigation");
    assert_eq!(
        app.windows.console.text, "ab c",
        "the text kept after Enter"
    );
    rejects("an Esc while typing that clears the text", || {
        check_kept("", "ab c")
    });
    // Typing, Tab moves to the next section, navigating, still in the console.
    press(&mut h, &mut app, Key::Enter, NONE);
    press(&mut h, &mut app, Key::Tab, NONE);
    check_at(&at(&app), &sections()[2], NAV, "Tab while typing");
    press(&mut h, &mut app, Key::Tab, SHIFT);
    press(&mut h, &mut app, Key::Enter, NONE);
    press(&mut h, &mut app, Key::Tab, SHIFT);
    check_at(&at(&app), &sections()[0], NAV, "Shift+Tab while typing");
}

#[test]
fn mock_keyboard_windows_question_mark_and_ctrl_z_are_the_fields_while_typing() {
    let (mut app, mut h) = on_text();
    // A contract edit, so the contract's undo has something to undo.
    let t = app.snapshot().render.playhead.t;
    app.set_field(super::support::playhead(t + 1.0, false));
    let _ = h.frame(&mut app, Vec::new());
    let depth = app.snapshot().history.undo_depth;
    assert!(depth > 0);
    press(&mut h, &mut app, Key::Enter, NONE);
    type_text(&mut h, &mut app, "x?");
    assert_eq!(app.windows.console.text, "x?", "? typed");
    assert!(!app.keyboard.shortcuts_open, "? opened the overlay");
    let typed = app.windows.console.text.clone();
    rejects("a ? while typing that opens the overlay", || {
        assert!(typed.is_empty() || !typed.contains('?'))
    });
    // Ctrl+Z (the command key) undoes the text edit, not the contract's history.
    let command = Modifiers {
        ctrl: true,
        command: true,
        ..Modifiers::NONE
    };
    press(&mut h, &mut app, Key::Z, command);
    let _ = h.frame(&mut app, Vec::new());
    assert_ne!(app.windows.console.text, "x?", "Ctrl+Z left the text");
    check_no_undo(app.snapshot().history.undo_depth, depth);
    check_at(&at(&app), &sections()[1], INTERACT, "still typing");
    rejects("a field whose Ctrl+Z goes to the contract's undo", || {
        check_no_undo(depth - 1, depth)
    });
}

#[test]
fn mock_keyboard_windows_the_entry_list_scrolls_only_after_enter() {
    let (mut app, mut h) = console_by_footer();
    press(&mut h, &mut app, Key::Tab, SHIFT);
    check_at(&at(&app), &sections()[3], NAV, "Shift+Tab to the list");
    press(&mut h, &mut app, Key::ArrowDown, NONE);
    press(&mut h, &mut app, Key::ArrowDown, NONE);
    assert_eq!(
        app.windows.console.top_row, 0,
        "↓ scrolled in navigation mode"
    );
    check_at(&at(&app), &sections()[3], NAV, "↓ navigating");
    let top = app.windows.console.top_row;
    rejects("an entry list that scrolls in navigation mode", || {
        assert_eq!(top, 1)
    });
    press(&mut h, &mut app, Key::Enter, NONE);
    check_at(&at(&app), &sections()[3], INTERACT, "Enter on the list");
    press(&mut h, &mut app, Key::ArrowDown, NONE);
    assert_eq!(app.windows.console.top_row, 1, "↓ scrolls");
    press(&mut h, &mut app, Key::ArrowUp, NONE);
    assert_eq!(app.windows.console.top_row, 0, "↑ scrolls back");
    press(&mut h, &mut app, Key::Escape, NONE);
    check_at(
        &at(&app),
        &sections()[3],
        NAV,
        "Esc back to navigation on the list",
    );
    // From interaction mode on the list, Tab moves to the console's next section, navigating, inside it.
    press(&mut h, &mut app, Key::Enter, NONE);
    press(&mut h, &mut app, Key::Tab, NONE);
    let got = at(&app);
    check_at(&got, &sections()[0], NAV, "Tab from the list");
    rejects(
        "a Tab from the list in interaction that leaves the console",
        || check_at(&got, &owned(&["top_bar"]), NAV, "Tab"),
    );
}

#[test]
fn mock_keyboard_windows_esc_closes_the_console_back_to_the_footer() {
    let (mut app, mut h) = console_by_footer();
    press(&mut h, &mut app, Key::Escape, NONE);
    check_at(
        &at(&app),
        &first_filter()[..2],
        NAV,
        "Esc up to the filters",
    );
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
    check_at(&at(&app), &first_filter(), NAV, "Windows › Console");
    press(&mut h, &mut app, Key::Tab, NONE);
    check_at(&at(&app), &sections()[1], NAV, "Tab");
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
        assert_eq!((first_filter(), NAV), before)
    });
    // Tab goes on among the big scopes, not into the console.
    press(&mut h, &mut app, Key::Tab, NONE);
    assert_eq!(focus(&app).len(), 1);
    assert_ne!(focus(&app)[0], console::WINDOW);
}
