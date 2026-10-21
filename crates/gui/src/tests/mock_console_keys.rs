//! REQ-GUI-189 (R-450; render_gui_spec §G3, §G12): the console's two scopes on the mock, by keyboard scripts. The
//! console opened from the footer or from Windows › Console lands in navigation mode on its header bar scope; from the
//! landing ↑ ↓, Tab, Shift+Tab, Enter, Esc and ← → reach every header control, × included, and the log; the search
//! field types after Enter and keeps its text on Esc and Enter; Enter on × and Esc at the top level close the console,
//! the focus back on its opener; the log scrolls to its first and last entry by Home, End, Page Up, Page Down and ↑ ↓,
//! Shift ×10, on a list longer than the rows drawn; the breadcrumb follows. Every script runs again after an auto-open
//! on error and with a filter that empties the list, giving the same order. A click on a console control moves the
//! keyboard's focus onto it (A3), which is the fix for the intermittent reachability this file first reproduces: a
//! click on the search field types at once, in interaction mode, and a click anywhere else lands in navigation mode,
//! ending any typing (R-456). Each check also runs on an input it must reject (R-176, through `rejects`).

use eframe::egui::{Event, Key, Modifiers};
use engine::contract::view_ui::{KeyMode, Window};

use super::support::{
    click_scope, focus, frame_texts, headless, mock_app, nodes, playhead, press, rejects,
};
use crate::app::App;
use crate::headless::Headless;
use crate::side::{EngineSide, MockSide};
use crate::windows::console::{self, Filter, CLEAR, CLOSE, COPY, HEADER, LOG, SEARCH, WINDOW};

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

fn check_at(got: &At, path: &[&str], mode: KeyMode, what: &str) {
    assert_eq!(got, &(owned(path), mode), "{what}");
}

fn settle(h: &mut Headless, app: &mut App<MockSide>) {
    for _ in 0..3 {
        let _ = h.frame(app, Vec::new());
    }
}

fn presses(h: &mut Headless, app: &mut App<MockSide>, key: Key, modifiers: Modifiers, n: usize) {
    for _ in 0..n {
        press(h, app, key, modifiers);
    }
}

/// The header bar's controls, in visual order (R-450).
const ROW: [&str; 8] = [
    "cs_all",
    "cs_warnings",
    "cs_errors",
    "cs_info",
    SEARCH,
    COPY,
    CLEAR,
    CLOSE,
];

/// The header bar scope, the landing.
const LANDING: [&str; 2] = [WINDOW, HEADER];
/// The log scope.
const ON_LOG: [&str; 2] = [WINDOW, LOG];

fn on_control(id: &str) -> [&str; 3] {
    [WINDOW, HEADER, id]
}

/// How the console was opened, and in what state.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Opened {
    /// Enter on the footer.
    Footer,
    /// Windows › Console, by the keys.
    Menu,
    /// An error opened it by itself, then Enter on the footer.
    AfterAutoOpen,
    /// Enter on the footer, then the Errors filter, which empties the list (only warnings logged).
    EmptyList,
}

const OPENINGS: [Opened; 4] = [
    Opened::Footer,
    Opened::Menu,
    Opened::AfterAutoOpen,
    Opened::EmptyList,
];

/// The app with `warnings` warnings logged.
fn with_warnings(warnings: usize) -> (App<MockSide>, Headless) {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    for _ in 0..warnings {
        app.side().engine().raise_warning();
        let _ = h.frame(&mut app, Vec::new());
    }
    (app, h)
}

/// Shift+Tab from no focus (the footer), then Enter.
fn by_footer(h: &mut Headless, app: &mut App<MockSide>) {
    app.view.focus.path.clear();
    press(h, app, Key::Tab, SHIFT);
    assert_eq!(focus(app), ["footer"]);
    press(h, app, Key::Enter, NONE);
    settle(h, app);
}

/// Tab to the top bar, Enter, → to Windows, Enter, ↓ to Console, Enter.
fn by_menu(h: &mut Headless, app: &mut App<MockSide>) {
    app.view.focus.path.clear();
    press(h, app, Key::Tab, NONE);
    press(h, app, Key::Enter, NONE);
    for _ in 0..12 {
        if focus(app).last() == Some(&"windows") {
            break;
        }
        press(h, app, Key::ArrowRight, NONE);
    }
    press(h, app, Key::Enter, NONE);
    for _ in 0..8 {
        if focus(app).last() == Some(&"windows_console") {
            break;
        }
        press(h, app, Key::ArrowDown, NONE);
    }
    assert_eq!(focus(app), ["top_bar", "windows", "windows_console"]);
    press(h, app, Key::Enter, NONE);
    settle(h, app);
}

/// Where the focus goes back to when the console opened as `opened` closes.
fn opener(opened: Opened) -> Vec<String> {
    match opened {
        Opened::Menu => owned(&["top_bar", "windows", "windows_console"]),
        _ => owned(&["footer"]),
    }
}

/// The console opened as `opened`, three warnings logged; the focus on the landing.
fn open(opened: Opened) -> (App<MockSide>, Headless) {
    let (mut app, mut h) = with_warnings(3);
    match opened {
        Opened::Footer => by_footer(&mut h, &mut app),
        Opened::Menu => by_menu(&mut h, &mut app),
        Opened::AfterAutoOpen => {
            press(&mut h, &mut app, Key::Tab, NONE);
            app.side().engine().raise_error();
            settle(&mut h, &mut app);
            assert!(app.is_open(Window::Console), "the error opened it");
            assert_eq!(focus(&app), ["top_bar"], "the auto-open took no focus");
            by_footer(&mut h, &mut app);
        }
        Opened::EmptyList => {
            by_footer(&mut h, &mut app);
            press(&mut h, &mut app, Key::Enter, NONE);
            presses(&mut h, &mut app, Key::ArrowRight, NONE, 2);
            press(&mut h, &mut app, Key::Enter, NONE);
            press(&mut h, &mut app, Key::Escape, NONE);
            assert_eq!(app.windows.console.filter, Filter::Errors);
            let rows = console::shown(app.console(), &app.windows.console).len();
            assert_eq!(rows, 0, "the Errors filter empties the list");
        }
    }
    assert!(app.is_open(Window::Console), "{opened:?}");
    (app, h)
}

/// Presses `key` and records where the focus is.
fn step(h: &mut Headless, app: &mut App<MockSide>, trace: &mut Vec<At>, key: Key, m: Modifiers) {
    press(h, app, key, m);
    trace.push(at(app));
}

/// The walk from the landing with only §G3's keys, every step's focus recorded: the scopes by ← → (nothing), ↑ ↓, Tab
/// and Shift+Tab; into the header bar, across every control and back, ↑ ↓ doing nothing there; Esc up; into the log
/// and out.
fn script(h: &mut Headless, app: &mut App<MockSide>) -> Vec<At> {
    let mut trace = vec![at(app)];
    for (key, m) in [
        (Key::ArrowRight, NONE),
        (Key::ArrowLeft, NONE),
        (Key::ArrowDown, NONE),
        (Key::ArrowDown, NONE),
        (Key::ArrowLeft, NONE),
        (Key::ArrowUp, NONE),
        (Key::ArrowUp, NONE),
        (Key::Tab, NONE),
        (Key::Tab, NONE),
        (Key::Tab, SHIFT),
        (Key::Tab, SHIFT),
        (Key::Enter, NONE),
    ] {
        step(h, app, &mut trace, key, m);
    }
    for _ in 0..ROW.len() {
        step(h, app, &mut trace, Key::ArrowRight, NONE);
    }
    step(h, app, &mut trace, Key::ArrowUp, NONE);
    step(h, app, &mut trace, Key::ArrowDown, NONE);
    for _ in 0..ROW.len() {
        step(h, app, &mut trace, Key::ArrowLeft, NONE);
    }
    for (key, m) in [
        (Key::Escape, NONE),
        (Key::ArrowDown, NONE),
        (Key::Enter, NONE),
        (Key::Escape, NONE),
        (Key::ArrowUp, NONE),
    ] {
        step(h, app, &mut trace, key, m);
    }
    trace
}

/// The trace [`script`] must give.
fn expected() -> Vec<At> {
    let nav = |p: &[&str]| (owned(p), NAV);
    let mut want = vec![nav(&LANDING)];
    for p in [
        LANDING, LANDING, ON_LOG, ON_LOG, ON_LOG, LANDING, LANDING, ON_LOG, LANDING, ON_LOG,
        LANDING,
    ] {
        want.push(nav(&p));
    }
    want.push(nav(&on_control(ROW[0])));
    for k in 1..=ROW.len() {
        want.push(nav(&on_control(ROW[k.min(ROW.len() - 1)])));
    }
    let close = nav(&on_control(CLOSE));
    want.push(close.clone());
    want.push(close);
    for k in (0..ROW.len()).rev() {
        want.push(nav(&on_control(ROW[k.saturating_sub(1)])));
    }
    want.push(nav(&LANDING));
    want.push(nav(&ON_LOG));
    want.push((owned(&ON_LOG), INTERACT));
    want.push(nav(&ON_LOG));
    want.push(nav(&LANDING));
    want
}

#[test]
fn mock_console_keys_land_on_the_header_bar_scope() {
    for opened in OPENINGS {
        let (app, _) = open(opened);
        let got = at(&app);
        check_at(&got, &LANDING, NAV, &format!("{opened:?}"));
    }
    let (app, _) = open(Opened::Footer);
    let got = at(&app);
    rejects("a landing on All", || {
        check_at(&got, &on_control("cs_all"), NAV, "landing")
    });
    rejects("a landing in interaction mode", || {
        check_at(&got, &LANDING, INTERACT, "landing")
    });
}

#[test]
fn mock_console_keys_reach_every_header_control_and_the_log_in_one_order() {
    let want = expected();
    let mut traces = Vec::new();
    for opened in OPENINGS {
        let (mut app, mut h) = open(opened);
        let trace = script(&mut h, &mut app);
        assert!(
            app.is_open(Window::Console),
            "{opened:?}: the walk closed it"
        );
        assert_eq!(trace, want, "{opened:?}");
        traces.push(trace);
    }
    // Every control and both scopes reached, × among them.
    for id in ROW.iter().chain([&HEADER, &LOG]) {
        assert!(
            want.iter()
                .any(|(p, _)| p.last().map(String::as_str) == Some(*id)),
            "{id} not reached"
        );
    }
    // Controls: × outside the scope tree; an order that skips the log when the list is empty.
    let without_close: Vec<At> = want
        .iter()
        .filter(|(p, _)| p.last().map(String::as_str) != Some(CLOSE))
        .cloned()
        .collect();
    rejects("a walk with × outside the scope tree", || {
        assert_eq!(without_close, want)
    });
    let skips_log: Vec<At> = traces[3]
        .iter()
        .map(|(p, m)| {
            let p = if p.last().map(String::as_str) == Some(LOG) {
                owned(&LANDING)
            } else {
                p.clone()
            };
            (p, *m)
        })
        .collect();
    rejects("an order that skips the log when the list is empty", || {
        assert_eq!(skips_log, traces[0])
    });
}

/// The breadcrumb drawn in the top bar.
fn breadcrumb(h: &mut Headless, app: &mut App<MockSide>) -> String {
    frame_texts(h, app)
        .into_iter()
        .find(|t| t.starts_with("Console ›"))
        .unwrap_or_default()
}

#[test]
fn mock_console_keys_the_breadcrumb_names_the_path() {
    let (mut app, mut h) = open(Opened::Footer);
    assert_eq!(breadcrumb(&mut h, &mut app), "Console › Header");
    press(&mut h, &mut app, Key::Enter, NONE);
    let names = [
        "All", "Warnings", "Errors", "Info", "Search", "Copy", "Clear", "Close",
    ];
    for (k, name) in names.iter().enumerate() {
        if k > 0 {
            press(&mut h, &mut app, Key::ArrowRight, NONE);
        }
        let got = breadcrumb(&mut h, &mut app);
        assert_eq!(got, format!("Console › Header › {name}"));
    }
    presses(&mut h, &mut app, Key::ArrowLeft, NONE, 2);
    let on_copy = breadcrumb(&mut h, &mut app);
    assert_eq!(on_copy, "Console › Header › Copy");
    press(&mut h, &mut app, Key::Escape, NONE);
    press(&mut h, &mut app, Key::ArrowDown, NONE);
    assert_eq!(breadcrumb(&mut h, &mut app), "Console › Log");
    rejects("a breadcrumb of R-405's sections", || {
        assert_eq!("Console › copy and clear › copy", on_copy)
    });
}

#[test]
fn mock_console_keys_esc_on_a_header_control_goes_up_and_at_the_top_closes() {
    for opened in OPENINGS {
        let (mut app, mut h) = open(opened);
        press(&mut h, &mut app, Key::Enter, NONE);
        presses(&mut h, &mut app, Key::ArrowRight, NONE, 5);
        check_at(&at(&app), &on_control(COPY), NAV, "→ to Copy");
        press(&mut h, &mut app, Key::Escape, NONE);
        settle(&mut h, &mut app);
        let open_after_esc = app.is_open(Window::Console);
        assert!(open_after_esc, "{opened:?}: Esc on Copy closed the console");
        check_at(&at(&app), &LANDING, NAV, "Esc up to the header bar");
        if opened == Opened::Footer {
            rejects("an Esc on a header element that closes the console", || {
                assert!(!open_after_esc, "closed")
            });
        }
        // Esc at the top level, on the header bar, closes, the focus back on the opener.
        press(&mut h, &mut app, Key::Escape, NONE);
        settle(&mut h, &mut app);
        assert!(!app.is_open(Window::Console), "{opened:?}");
        assert_eq!(app.view.focus.path, opener(opened), "{opened:?}");
        // And from the log scope.
        let (mut app, mut h) = open(opened);
        press(&mut h, &mut app, Key::ArrowDown, NONE);
        press(&mut h, &mut app, Key::Escape, NONE);
        settle(&mut h, &mut app);
        assert!(!app.is_open(Window::Console), "{opened:?}: Esc on the log");
        assert_eq!(app.view.focus.path, opener(opened), "{opened:?}");
    }
}

#[test]
fn mock_console_keys_enter_on_close_closes_back_to_the_opener() {
    for opened in OPENINGS {
        let (mut app, mut h) = open(opened);
        press(&mut h, &mut app, Key::Enter, NONE);
        presses(&mut h, &mut app, Key::ArrowRight, NONE, ROW.len() - 1);
        check_at(&at(&app), &on_control(CLOSE), NAV, "→ to ×");
        press(&mut h, &mut app, Key::Enter, NONE);
        settle(&mut h, &mut app);
        assert!(!app.is_open(Window::Console), "{opened:?}: Enter on ×");
        let got = app.view.focus.path.clone();
        assert_eq!(got, opener(opened), "{opened:?}");
        rejects("a focus left nowhere", || {
            assert_eq!(got, Vec::<String>::new())
        });
    }
}

/// Types `text` as a keyboard would: each character's key press with its text, then the release.
fn type_text(h: &mut Headless, app: &mut App<MockSide>, text: &str) {
    for c in text.chars() {
        let key = Key::from_name(&c.to_string()).unwrap_or(Key::Space);
        let event = |pressed| Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: NONE,
        };
        let _ = h.frame(app, vec![event(true), Event::Text(c.to_string())]);
        let _ = h.frame(app, vec![event(false)]);
    }
}

#[test]
fn mock_console_keys_the_search_field_types_after_enter_and_keeps_its_text() {
    for opened in OPENINGS {
        let (mut app, mut h) = open(opened);
        let before = app.windows.console.text.clone();
        press(&mut h, &mut app, Key::Enter, NONE);
        presses(&mut h, &mut app, Key::ArrowRight, NONE, 4);
        check_at(&at(&app), &on_control(SEARCH), NAV, "→ to the search field");
        type_text(&mut h, &mut app, "w");
        assert_eq!(
            app.windows.console.text, before,
            "{opened:?}: typed on landing"
        );
        press(&mut h, &mut app, Key::Enter, NONE);
        check_at(
            &at(&app),
            &on_control(SEARCH),
            INTERACT,
            "Enter starts typing",
        );
        type_text(&mut h, &mut app, "mock");
        let typed = format!("{before}mock");
        assert_eq!(app.windows.console.text, typed, "{opened:?}");
        press(&mut h, &mut app, Key::Escape, NONE);
        check_at(&at(&app), &on_control(SEARCH), NAV, "Esc stops typing");
        assert!(app.is_open(Window::Console));
        assert_eq!(app.windows.console.text, typed, "Esc keeps the text");
        press(&mut h, &mut app, Key::Enter, NONE);
        type_text(&mut h, &mut app, "x");
        press(&mut h, &mut app, Key::Enter, NONE);
        check_at(&at(&app), &on_control(SEARCH), NAV, "Enter stops typing");
        let kept = app.windows.console.text.clone();
        assert_eq!(kept, format!("{typed}x"), "Enter keeps the text");
        rejects("an Esc that clears the text", || assert_eq!(kept, before));
        // Typing, Tab moves on to the log, inside the console.
        press(&mut h, &mut app, Key::Enter, NONE);
        press(&mut h, &mut app, Key::Tab, NONE);
        check_at(&at(&app), &ON_LOG, NAV, "Tab while typing");
    }
}

/// An app whose console holds `n` entries with distinct messages and more: one per playhead edit.
fn long_log(n: usize) -> (App<MockSide>, Headless) {
    let (mut app, mut h) = with_warnings(0);
    for i in 0..n {
        app.set_field(playhead(i as f64 + 1.0, false));
        let _ = h.frame(&mut app, Vec::new());
    }
    by_footer(&mut h, &mut app);
    (app, h)
}

/// The texts drawn inside the log's rect.
fn drawn_in_log(h: &mut Headless, app: &mut App<MockSide>) -> Vec<String> {
    let all = nodes(h, app);
    let log = app.keyboard.place_of(LOG).expect("the log placed").rect;
    all.into_iter()
        .filter(|n| log.contains(n.rect.center()))
        .map(|n| n.text)
        .collect()
}

fn check_shows(drawn: &[String], message: &str, what: &str) {
    assert!(
        drawn.iter().any(|t| t == message),
        "{what}: `{message}` not drawn in {drawn:?}"
    );
}

fn top(app: &App<MockSide>) -> usize {
    app.windows.console.top_row
}

#[test]
fn mock_console_keys_the_log_scrolls_by_entry_page_and_end_over_the_whole_list() {
    let (mut app, mut h) = long_log(60);
    let h = &mut h;
    let app = &mut app;
    let rows = console::shown(app.console(), &app.windows.console).len();
    let page = app.windows.console.page;
    assert!(
        page > 1 && rows > 3 * page + 12,
        "{rows} rows, a page of {page}"
    );
    let last_top = rows - page;
    let newest = app.console().last().unwrap().message.clone();
    let oldest = app.console().first().unwrap().message.clone();
    let first_view = drawn_in_log(h, app);
    check_shows(&first_view, &newest, "unscrolled");
    rejects("a scroll that stops at the last row drawn", || {
        check_shows(&first_view, &oldest, "the rows drawn unscrolled")
    });
    press(h, app, Key::ArrowDown, NONE);
    check_at(&at(app), &ON_LOG, NAV, "↓ to the log");
    // Navigating, the scroll keys move nothing.
    for key in [Key::ArrowDown, Key::PageDown, Key::End] {
        press(h, app, key, NONE);
        assert_eq!(top(app), 0, "{key:?} navigating");
    }
    check_at(&at(app), &ON_LOG, NAV, "still on the log");
    press(h, app, Key::Enter, NONE);
    check_at(&at(app), &ON_LOG, INTERACT, "Enter on the log");
    let mut want = 0;
    for (key, m, to) in [
        (Key::ArrowUp, NONE, 0),
        (Key::ArrowDown, NONE, 1),
        (Key::ArrowDown, SHIFT, 11),
        (Key::ArrowDown, ALT, 12),
        (Key::ArrowUp, NONE, 11),
        (Key::PageDown, NONE, 11 + page),
        (Key::PageUp, NONE, 11),
        (Key::PageDown, SHIFT, last_top),
        (Key::ArrowDown, NONE, last_top),
        (Key::PageUp, NONE, last_top - page),
        (Key::PageUp, SHIFT, 0),
        (Key::End, NONE, last_top),
        (Key::Home, NONE, 0),
        (Key::End, SHIFT, last_top),
        (Key::ArrowUp, SHIFT, last_top - 10),
        (Key::Home, SHIFT, 0),
    ] {
        press(h, app, key, m);
        settle(h, app);
        assert_eq!(top(app), to, "{key:?} {m:?} from {want}");
        want = to;
    }
    // End shows the last entry, the oldest; Home the first, the newest.
    press(h, app, Key::End, NONE);
    settle(h, app);
    let at_end = drawn_in_log(h, app);
    check_shows(&at_end, &oldest, "End");
    press(h, app, Key::Home, NONE);
    settle(h, app);
    check_shows(&drawn_in_log(h, app), &newest, "Home");
    rejects("an End that leaves the first entry in view", || {
        check_shows(&at_end, &newest, "End")
    });
    // Esc back to navigation mode on the log.
    press(h, app, Key::Escape, NONE);
    check_at(&at(app), &ON_LOG, NAV, "Esc");
    assert!(app.is_open(Window::Console));
}

#[test]
fn mock_console_keys_an_empty_list_keeps_the_log_where_it_is() {
    let (mut app, mut h) = open(Opened::EmptyList);
    press(&mut h, &mut app, Key::ArrowDown, NONE);
    check_at(&at(&app), &ON_LOG, NAV, "↓ to the empty log");
    press(&mut h, &mut app, Key::Enter, NONE);
    check_at(&at(&app), &ON_LOG, INTERACT, "Enter on the empty log");
    for key in [
        Key::ArrowDown,
        Key::PageDown,
        Key::End,
        Key::ArrowUp,
        Key::Home,
    ] {
        press(&mut h, &mut app, key, NONE);
        assert_eq!(top(&app), 0, "{key:?}");
    }
    check_at(&at(&app), &ON_LOG, INTERACT, "the scroll keys keep it");
    let got = at(&app);
    rejects("an empty list with no log scope", || {
        check_at(&got, &LANDING, INTERACT, "the empty log")
    });
}

// --- A3: a click moves the keyboard's focus; the intermittent reachability -------------------------------------------

/// The reproduction of the intermittent reachability (R-450): an error opens the console by itself, which takes no
/// focus; the user clicks its search field, and the keys alone should then reach Copy. Before this task the layer's
/// focus never moved into the console on a click (a clicked field held the keyboard as egui's), and a window is outside
/// the big scopes' Tab cycle, so Tab and the arrows walked the big scopes and no key reached the search field, Copy or
/// Clear, until Enter on the footer or Windows › Console moved the focus in ("then later I can"). Now the click puts
/// the keyboard's focus on the field (A3), typing in it at once, the ring blue (R-456); Esc ends the typing and → moves
/// on.
#[test]
fn mock_console_keys_reproduce_the_intermittent_reachability() {
    let (mut app, mut h) = with_warnings(1);
    app.side().engine().raise_error();
    settle(&mut h, &mut app);
    assert!(app.is_open(Window::Console));
    assert!(focus(&app).is_empty(), "the auto-open took no focus");
    click_scope(&mut h, &mut app, SEARCH);
    let after_click = at(&app);
    check_at(
        &after_click,
        &on_control(SEARCH),
        INTERACT,
        "the click moved the focus and types",
    );
    rejects(
        "a click that leaves the keyboard's focus outside the console",
        || check_at(&(Vec::new(), NAV), &on_control(SEARCH), INTERACT, "click"),
    );
    rejects(
        "a clicked field in navigation mode, the ring orange",
        || check_at(&after_click, &on_control(SEARCH), NAV, "click"),
    );
    // Typing, as the ring shows: a letter types at once (R-456).
    type_text(&mut h, &mut app, "m");
    let text = app.windows.console.text.clone();
    assert_eq!(text, "m", "a clicked field types at once");
    rejects("a clicked field that types nothing", || {
        assert_eq!(text, "")
    });
    // Esc ends typing, the text kept (§G3, R-409 G4).
    press(&mut h, &mut app, Key::Escape, NONE);
    check_at(&at(&app), &on_control(SEARCH), NAV, "Esc ends typing");
    assert_eq!(app.windows.console.text, "m", "Esc kept the text");
    press(&mut h, &mut app, Key::ArrowRight, NONE);
    check_at(&at(&app), &on_control(COPY), NAV, "→ reaches Copy");
    press(&mut h, &mut app, Key::Enter, NONE);
    assert!(app.windows.console.copied.is_some(), "Enter on Copy copies");
    press(&mut h, &mut app, Key::ArrowRight, NONE);
    check_at(&at(&app), &on_control(CLEAR), NAV, "→ reaches Clear");
    // Esc up and out: the focus goes back to where it was before the click, nowhere.
    press(&mut h, &mut app, Key::Escape, NONE);
    press(&mut h, &mut app, Key::Escape, NONE);
    settle(&mut h, &mut app);
    assert!(!app.is_open(Window::Console));
    assert!(focus(&app).is_empty(), "{:?}", focus(&app));
}

/// A click on the search field types at once (R-456), and §G3's text-field rule holds while it does: Tab and Shift+Tab
/// leave the field for the window's next or previous scope, the log from the header (qa's F2), in navigation mode, the
/// text kept, a letter after it typing nothing; Enter stops typing on the field, the text kept (R-409 G4, G5).
#[test]
fn mock_console_keys_a_clicked_text_field_types_and_tab_leaves_it() {
    for (key, m) in [(Key::Tab, NONE), (Key::Tab, SHIFT), (Key::Enter, NONE)] {
        let (mut app, mut h) = open(Opened::Footer);
        click_scope(&mut h, &mut app, SEARCH);
        type_text(&mut h, &mut app, "ab");
        assert_eq!(app.windows.console.text, "ab", "typed after the click");
        press(&mut h, &mut app, key, m);
        let got = at(&app);
        if key == Key::Enter {
            check_at(&got, &on_control(SEARCH), NAV, "Enter while typing");
        } else {
            check_at(&got, &ON_LOG, NAV, &format!("{key:?} {m:?} while typing"));
        }
        rejects("a key the field kept, typing on", || {
            check_at(&got, &on_control(SEARCH), INTERACT, "after the key")
        });
        type_text(&mut h, &mut app, "c");
        let text = app.windows.console.text.clone();
        assert_eq!(
            text, "ab",
            "{key:?}: kept the text, and the next letter typed nothing"
        );
        rejects("a field still typing", || assert_eq!(text, "abc"));
    }
}

/// Where a click lands, after typing in the search field.
#[derive(Clone, Copy, Debug)]
enum Away {
    /// A console control: Copy, a filter or the log, the focus moved onto it (A3).
    Control(&'static str),
    /// The Figure, outside the console, which moves no focus (R-456 B6).
    Figure,
}

/// Clicks `away`, outside any console control for [`Away::Figure`].
fn click_away(h: &mut Headless, app: &mut App<MockSide>, away: Away) {
    match away {
        Away::Control(id) => click_scope(h, app, id),
        Away::Figure => {
            let console = app
                .keyboard
                .place_of(WINDOW)
                .expect("the console placed")
                .rect;
            let figure = app
                .keyboard
                .place_of(crate::explore::FIGURE)
                .expect("the figure")
                .rect;
            let outside = figure.left_top() + eframe::egui::vec2(8.0, 8.0);
            assert!(!console.contains(outside), "{outside:?} is in the console");
            for events in crate::capture::click(outside) {
                let _ = h.frame(app, events);
            }
        }
    }
    settle(h, app);
}

/// Ctrl+Z (Cmd+Z), which undoes the text edit while a field is typed in (§G3, R-409).
fn undo(h: &mut Headless, app: &mut App<MockSide>) {
    press(h, app, Key::Z, Modifiers::COMMAND);
}

/// A click anywhere but a text field lands in navigation mode (R-456): on a console control, Copy, a filter or the
/// log, the focus on it; on the Figure, outside the console, the focus where it was, on the field (B6). Either way the
/// typing ends: egui's text focus is released, and neither a letter nor Ctrl+Z reaches the field.
#[test]
fn mock_console_keys_a_click_elsewhere_lands_in_navigation() {
    for away in [
        Away::Control(COPY),
        Away::Control("cs_warnings"),
        Away::Control(LOG),
        Away::Figure,
    ] {
        let (mut app, mut h) = open(Opened::Footer);
        click_scope(&mut h, &mut app, SEARCH);
        check_at(
            &at(&app),
            &on_control(SEARCH),
            INTERACT,
            "the field clicked",
        );
        type_text(&mut h, &mut app, "y");
        assert_eq!(app.windows.console.text, "y", "{away:?}: typed");
        click_away(&mut h, &mut app, away);
        let got = at(&app);
        let want = match away {
            Away::Control(LOG) => ON_LOG.to_vec(),
            Away::Control(id) => on_control(id).to_vec(),
            Away::Figure => on_control(SEARCH).to_vec(),
        };
        check_at(&got, &want, NAV, &format!("a click on {away:?}"));
        rejects("typing that survives the click", || {
            check_at(&got, &on_control(SEARCH), INTERACT, "typing")
        });
        type_text(&mut h, &mut app, "z");
        undo(&mut h, &mut app);
        let text = app.windows.console.text.clone();
        assert_eq!(text, "y", "{away:?}: a letter or Ctrl+Z reached the field");
        rejects("a letter that reached the field", || {
            assert_eq!(format!("{text}z"), "y")
        });
        if let Away::Figure = away {
            // B6: the click moves no focus; the control has it moved onto the Figure scope.
            let figure = app
                .keyboard
                .tree(app.view.mode)
                .path_to(crate::explore::FIGURE);
            rejects("a Figure click that moved the focus to the Figure", || {
                check_at(&(figure, NAV), &want, NAV, "the Figure clicked")
            });
            press(&mut h, &mut app, Key::ArrowRight, NONE);
            check_at(
                &at(&app),
                &on_control(COPY),
                NAV,
                "→ after the click outside",
            );
        }
    }
    // The control for Ctrl+Z: typing, it undoes the text edit, so the check above can fail.
    let (mut app, mut h) = open(Opened::Footer);
    click_scope(&mut h, &mut app, SEARCH);
    type_text(&mut h, &mut app, "y");
    undo(&mut h, &mut app);
    assert_eq!(app.windows.console.text, "", "Ctrl+Z while typing");
}

/// Ctrl+Z while the search field is typed in undoes the text edit and not the contract's history; once a click
/// elsewhere ends the typing, Ctrl+Z undoes from the contract (§G3; R-409 G5).
#[test]
fn mock_console_keys_ctrl_z_while_typing_is_the_fields() {
    let (mut app, mut h) = open(Opened::Footer);
    let mut moved = app.snapshot().sim.plane.z0;
    moved[0] += 0.25;
    app.set_field(crate::explore::manifold_view::z0_edit(moved));
    let _ = h.frame(&mut app, Vec::new());
    let depth = app.snapshot().history.undo_depth;
    assert!(depth > 0, "the edit is undoable");
    click_scope(&mut h, &mut app, SEARCH);
    type_text(&mut h, &mut app, "y");
    undo(&mut h, &mut app);
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(app.windows.console.text, "", "Ctrl+Z undid the text edit");
    let typing = app.snapshot().history.undo_depth;
    assert_eq!(typing, depth, "Ctrl+Z while typing undid the contract");
    rejects("a Ctrl+Z while typing that undid the contract", || {
        assert_eq!(typing, depth - 1)
    });
    click_away(&mut h, &mut app, Away::Figure);
    undo(&mut h, &mut app);
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(
        app.snapshot().history.undo_depth,
        depth - 1,
        "Ctrl+Z after typing"
    );
}

/// Interaction mode on the log, then a click on the Figure, outside the console: navigation mode, the focus still on
/// the log (R-456: a click off a text input lands in navigation mode; B6: it moves no focus), and ↓ no longer scrolls.
#[test]
fn mock_console_keys_a_click_off_a_text_input_ends_interaction_on_the_log() {
    let (mut app, mut h) = long_log(60);
    press(&mut h, &mut app, Key::ArrowDown, NONE);
    press(&mut h, &mut app, Key::Enter, NONE);
    check_at(&at(&app), &ON_LOG, INTERACT, "Enter on the log");
    press(&mut h, &mut app, Key::ArrowDown, NONE);
    let scrolled = top(&app);
    assert!(scrolled > 0, "↓ scrolled the log in interaction mode");
    click_away(&mut h, &mut app, Away::Figure);
    let got = at(&app);
    check_at(&got, &ON_LOG, NAV, "a click on the Figure");
    rejects("interaction mode surviving the click", || {
        check_at(&got, &ON_LOG, INTERACT, "the Figure clicked")
    });
    press(&mut h, &mut app, Key::ArrowDown, NONE);
    assert_eq!(top(&app), scrolled, "↓ scrolled after the click");
}

#[test]
fn mock_console_keys_a_click_moves_the_keyboards_focus_onto_the_control() {
    let (mut app, mut h) = open(Opened::Footer);
    for id in [COPY, "cs_warnings", CLEAR, "cs_all"] {
        click_scope(&mut h, &mut app, id);
        check_at(&at(&app), &on_control(id), NAV, &format!("a click on {id}"));
    }
    assert_eq!(app.windows.console.filter, Filter::All, "the click acted");
    click_scope(&mut h, &mut app, LOG);
    check_at(&at(&app), &ON_LOG, NAV, "a click on the log");
    let got = at(&app);
    rejects("a click that moved egui's focus alone", || {
        check_at(&got, &LANDING, NAV, "a click on the log")
    });
    // A click on × closes, as Enter on it does: the focus back on the footer.
    click_scope(&mut h, &mut app, CLOSE);
    settle(&mut h, &mut app);
    assert!(!app.is_open(Window::Console));
    assert_eq!(focus(&app), ["footer"]);
    // After an auto-open, the focus elsewhere: a click on × leaves it where it was.
    let (mut app, mut h) = with_warnings(0);
    press(&mut h, &mut app, Key::Tab, NONE);
    app.side().engine().raise_error();
    settle(&mut h, &mut app);
    click_scope(&mut h, &mut app, CLOSE);
    settle(&mut h, &mut app);
    assert!(!app.is_open(Window::Console));
    assert_eq!(focus(&app), ["top_bar"]);
}

// --- The rules on the tree itself (R-450 A1, A4) ---------------------------------------------------------------------

#[test]
fn mock_console_keys_the_axes_move_along_their_arrows_only() {
    use crate::keyboard::keymap::Direction::{Down, Left, Right, Up};
    use crate::keyboard::scopes::Axis;
    for d in [Up, Down, Left, Right] {
        assert!(Axis::Both.moves(d), "{d:?}");
    }
    assert_eq!(
        [Up, Down, Left, Right].map(|d| Axis::Row.moves(d)),
        [false, false, true, true]
    );
    assert_eq!(
        [Up, Down, Left, Right].map(|d| Axis::Column.moves(d)),
        [true, true, false, false]
    );
    rejects("a row that ↓ moves along", || {
        assert!(Axis::Row.moves(Down))
    });
}

#[test]
fn mock_console_keys_a_scroll_counts_one_or_ten() {
    use crate::keyboard::keymap::multiplier;
    use crate::keyboard::scopes::scroll_count;
    let shift_alt = Modifiers {
        shift: true,
        alt: true,
        ..NONE
    };
    for (m, n) in [(NONE, 1), (SHIFT, 10), (ALT, 1), (shift_alt, 1)] {
        assert_eq!(scroll_count(1.0, multiplier(m)), n, "{m:?}");
        assert_eq!(scroll_count(-1.0, multiplier(m)), -n, "{m:?}");
    }
    assert_eq!(scroll_count(0.0, 1.0), 1, "no sign is down the list");
    rejects("Alt's tenth read as no step", || {
        assert_eq!(scroll_count(1.0, multiplier(ALT)), 0)
    });
}

#[test]
fn mock_console_keys_page_home_and_end_are_the_tables() {
    use crate::keyboard::keymap::{command, repeats, Command, ScrollKey};
    for (key, k, held) in [
        (Key::PageUp, ScrollKey::PageUp, true),
        (Key::PageDown, ScrollKey::PageDown, true),
        (Key::Home, ScrollKey::Home, false),
        (Key::End, ScrollKey::End, false),
    ] {
        for m in [NONE, SHIFT] {
            assert_eq!(command(key, m), Some(Command::Scroll(k)), "{key:?} {m:?}");
        }
        assert_eq!(repeats(Command::Scroll(k)), held, "{key:?}");
    }
    let ctrl = Modifiers {
        ctrl: true,
        command: true,
        ..NONE
    };
    assert_eq!(command(Key::End, ctrl), None, "Ctrl is the system's");
    rejects("Home repeating while held", || {
        assert!(repeats(Command::Scroll(ScrollKey::Home)))
    });
}

/// The console's own tree, from the layer.
fn console_tree() -> crate::keyboard::scopes::ScopeTree {
    let mut tree = crate::keyboard::scopes::ScopeTree::new();
    tree.register(
        None,
        crate::keyboard::scopes::Scope::scoped_window(WINDOW, "Console"),
    );
    console::register(&mut tree);
    tree
}

fn run(
    tree: &crate::keyboard::scopes::ScopeTree,
    path: &[&str],
    mode: KeyMode,
    command: crate::keyboard::keymap::Command,
    multiplier: f64,
) -> (At, crate::keyboard::scopes::Outcome) {
    let mut f = engine::contract::view_ui::Focus {
        path: owned(path),
        mode,
    };
    let outcome = tree.navigate(&mut f, command, multiplier);
    ((f.path, f.mode), outcome)
}

#[test]
fn mock_console_keys_the_tree_lands_moves_and_scrolls_as_r450_has_it() {
    use crate::keyboard::keymap::Command::{Arrow, Enter, Next, Previous, Scroll as Key};
    use crate::keyboard::keymap::Direction::{Down, Left, Right, Up};
    use crate::keyboard::keymap::ScrollKey::{End, Home, PageDown, PageUp};
    use crate::keyboard::scopes::{Outcome, Scroll, ScrollBy};
    let tree = console_tree();
    assert_eq!(tree.window_landing(WINDOW), LANDING);
    assert_eq!(tree.section_landing(WINDOW, LOG), ON_LOG);
    assert_eq!(run(&tree, &[WINDOW], NAV, Next, 1.0).0 .0, LANDING);
    assert_eq!(run(&tree, &[WINDOW], NAV, Previous, 1.0).0 .0, ON_LOG);
    assert_eq!(run(&tree, &on_control(COPY), NAV, Next, 1.0).0 .0, ON_LOG);
    // The arrows: ↑ ↓ between the scopes, ← → nothing; on the row ← → only.
    let moved = |path: &[&str], c| run(&tree, path, NAV, c, 1.0).0 .0;
    assert_eq!(moved(&LANDING, Arrow(Down)), ON_LOG);
    assert_eq!(moved(&ON_LOG, Arrow(Up)), LANDING);
    assert_eq!(moved(&LANDING, Arrow(Right)), LANDING);
    assert_eq!(moved(&ON_LOG, Arrow(Left)), ON_LOG);
    assert_eq!(moved(&on_control(COPY), Arrow(Right)), on_control(CLEAR));
    assert_eq!(moved(&on_control(COPY), Arrow(Left)), on_control(SEARCH));
    assert_eq!(moved(&on_control(COPY), Arrow(Down)), on_control(COPY));
    assert_eq!(moved(&on_control(COPY), Arrow(Up)), on_control(COPY));
    // Enter: into the header bar, on All; on the log, interaction.
    assert_eq!(
        run(&tree, &LANDING, NAV, Enter, 1.0).0 .0,
        on_control("cs_all")
    );
    assert_eq!(
        run(&tree, &ON_LOG, NAV, Enter, 1.0).0,
        (owned(&ON_LOG), INTERACT)
    );
    assert_eq!(
        run(&tree, &on_control(CLOSE), NAV, Enter, 1.0).1,
        Outcome::Activate(CLOSE)
    );
    // The log's keys, interacting.
    let scrolled = |c, m| run(&tree, &ON_LOG, INTERACT, c, m).1;
    let by = |by| Outcome::Scroll(Scroll { scope: LOG, by });
    assert_eq!(scrolled(Arrow(Down), 1.0), by(ScrollBy::Entries(1)));
    assert_eq!(scrolled(Arrow(Up), 1.0), by(ScrollBy::Entries(-1)));
    assert_eq!(scrolled(Arrow(Down), 10.0), by(ScrollBy::Entries(10)));
    assert_eq!(scrolled(Arrow(Left), 1.0), Outcome::None);
    assert_eq!(scrolled(Arrow(Right), 1.0), Outcome::None);
    assert_eq!(scrolled(Key(PageDown), 1.0), by(ScrollBy::Pages(1)));
    assert_eq!(scrolled(Key(PageUp), 1.0), by(ScrollBy::Pages(-1)));
    assert_eq!(scrolled(Key(PageUp), 10.0), by(ScrollBy::Pages(-10)));
    assert_eq!(scrolled(Key(Home), 10.0), by(ScrollBy::First));
    assert_eq!(scrolled(Key(End), 1.0), by(ScrollBy::Last));
    // Navigating, or on anything but a list, or on nothing, the scroll keys do nothing.
    assert_eq!(run(&tree, &ON_LOG, NAV, Key(End), 1.0).1, Outcome::None);
    let typing = run(&tree, &on_control(SEARCH), INTERACT, Key(End), 1.0).1;
    assert_eq!(typing, Outcome::None);
    assert_eq!(run(&tree, &[], NAV, Key(End), 1.0).1, Outcome::None);
    // A window of the old kind keeps its landing on a section's first element.
    let mut old = crate::keyboard::scopes::ScopeTree::new();
    old.register(None, crate::keyboard::scopes::Scope::window("w", "W"));
    old.register(Some("w"), crate::keyboard::scopes::Scope::group("s", "S"));
    old.register(Some("s"), crate::keyboard::scopes::Scope::control("e", "E"));
    assert_eq!(old.window_landing("w"), ["w", "s", "e"]);
    rejects("a log whose ↑ scrolls towards the last entry", || {
        assert_eq!(scrolled(Arrow(Up), 1.0), by(ScrollBy::Entries(1)))
    });
}

/// F3 hides the layer: a scroll asked for in the frame before is forgotten, as an adjustment is.
#[test]
fn mock_console_keys_hiding_the_layer_forgets_a_scroll() {
    let (mut app, mut h) = long_log(20);
    press(&mut h, &mut app, Key::ArrowDown, NONE);
    press(&mut h, &mut app, Key::Enter, NONE);
    let key = |key, pressed| Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: NONE,
    };
    let _ = h.frame(&mut app, vec![key(Key::End, true)]);
    let asked = app.keyboard.scrolled().len();
    assert_eq!(asked, 1, "End asked for a scroll");
    let _ = h.frame(&mut app, vec![key(Key::F3, true)]);
    assert!(!app.shown, "F3 hid the layer");
    let left = app.keyboard.scrolled().len();
    assert_eq!(left, 0, "the scroll forgotten");
    rejects("a scroll kept while hidden", || assert_eq!(asked, 0));
}

/// A click in the console while another window holds the focus keeps that window's opener: Run, opened from the top
/// bar, then the console opened by an error and clicked, Esc out of each in turn, back on Run….
#[test]
fn mock_console_keys_a_click_keeps_the_other_windows_openers() {
    let (mut app, mut h) = with_warnings(0);
    app.view.focus.path.clear();
    press(&mut h, &mut app, Key::Tab, NONE);
    press(&mut h, &mut app, Key::Enter, NONE);
    for _ in 0..12 {
        if focus(&app).last() == Some(&"run") {
            break;
        }
        press(&mut h, &mut app, Key::ArrowRight, NONE);
    }
    press(&mut h, &mut app, Key::Enter, NONE);
    settle(&mut h, &mut app);
    assert!(app.is_open(Window::Run));
    let in_run = app.view.focus.path.clone();
    assert_eq!(in_run[0], "win_run");
    app.side().engine().raise_error();
    settle(&mut h, &mut app);
    click_scope(&mut h, &mut app, COPY);
    check_at(&at(&app), &on_control(COPY), NAV, "the click");
    // Esc, Esc: out of the console, back where the focus was in Run.
    presses(&mut h, &mut app, Key::Escape, NONE, 2);
    settle(&mut h, &mut app);
    assert!(!app.is_open(Window::Console));
    assert_eq!(app.view.focus.path, in_run, "back in Run");
    // Esc up and out of Run: back on Run….
    for _ in 0..3 {
        if !app.is_open(Window::Run) {
            break;
        }
        press(&mut h, &mut app, Key::Escape, NONE);
    }
    settle(&mut h, &mut app);
    assert!(!app.is_open(Window::Run));
    let got = focus(&app)
        .iter()
        .map(|s| (*s).to_owned())
        .collect::<Vec<_>>();
    assert_eq!(got, ["top_bar", "run"], "back on Run…");
    rejects("Run's opener lost to the console's click", || {
        assert_eq!(got, Vec::<String>::new())
    });
}

/// The wheel scrolls the log; the keys go on from where it left it, so the mouse and the keyboard agree (A3, A4).
#[test]
fn mock_console_keys_the_keys_scroll_on_from_the_wheel() {
    let (mut app, mut h) = long_log(60);
    let log = app.keyboard.place_of(LOG).expect("the log placed").rect;
    let _ = h.frame(&mut app, vec![Event::PointerMoved(log.center())]);
    for _ in 0..4 {
        let wheel = Event::MouseWheel {
            unit: eframe::egui::MouseWheelUnit::Line,
            delta: eframe::egui::vec2(0.0, -3.0),
            phase: eframe::egui::TouchPhase::Move,
            modifiers: NONE,
        };
        let _ = h.frame(&mut app, vec![wheel]);
    }
    for _ in 0..4 {
        settle(&mut h, &mut app);
    }
    let wheeled = top(&app);
    assert!(wheeled > 2, "the wheel scrolled the log to row {wheeled}");
    press(&mut h, &mut app, Key::ArrowDown, NONE);
    press(&mut h, &mut app, Key::Enter, NONE);
    press(&mut h, &mut app, Key::ArrowDown, NONE);
    settle(&mut h, &mut app);
    let after = top(&app);
    assert_eq!(after, wheeled + 1, "↓ goes on from the wheel's row");
    rejects("a ↓ that jumps back to the keys' last row", || {
        assert_eq!(1, wheeled + 1)
    });
}
