//! QA tests for TASK-M6-32, written from R-450 and its requirements, not from the implementation:
//!
//! - REQ-GUI-189 (R-450, A1 to A6; render_gui_spec §G3, §G12): the console is a window of exactly two scopes, the
//!   header bar (All, Warnings, Errors, Info, the search field, Copy, Clear, ×, in that order) and the log. Opened from
//!   the footer or Windows › Console it lands in navigation mode on the header bar scope itself; an error opens it
//!   taking no focus. ↑ ↓, Tab and Shift+Tab move between the two scopes, inside the window; Enter on the header bar
//!   goes onto All, ← → cross every header control, × included, stopping at the row's ends, and Esc goes back up. The
//!   search field types after Enter and keeps its text on Esc and Enter. Enter (or a click) on × and Esc at the top
//!   level close the console, the focus back on its opener. Enter on the log starts interaction: ↑ ↓ an entry, Page Up
//!   and Page Down a page, Home and End to the first and last entry, Shift ×10, Alt the unmodified step, over the whole
//!   filtered list, clamped; Esc back to navigation on the log. A click on a console control moves the keyboard's
//!   focus onto it. The breadcrumb reads "Console › Header › Copy". Every script gives the same order after an
//!   auto-open on error and with a filter that empties the list.
//! - REQ-GUI-190 (R-450, A5, A7; §G3): every drawn control of the console, in each of its states, and of every other
//!   window, is reached by §G3's keys; the order does not depend on state.
//!
//! The focus is read from `ViewUI` (gui_state_contract §2), by the tree's labels (A6's names); what is drawn is read
//! from the frame's AccessKit tree. Each check runs its own control in the test (R-176), through [`rejects`].
// The file name `qa_TASK-M6-32` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use eframe::egui::accesskit::Role;
use eframe::egui::{self, Event, Key, Modifiers, PointerButton, Pos2, Rect};
use engine::contract::log::Severity;
use engine::contract::render_state::Playhead;
use engine::contract::set_field::{Edit, RenderField, SetField};
use engine::contract::view_ui::{Focus, KeyMode, Mode, Window};
use gui::app::App;
use gui::capture::{PIXELS_PER_POINT, SIZE};
use gui::headless::Headless;
use gui::keyboard::keymap;
use gui::keyboard::scopes::{Outcome, ScopeTree};
use gui::layout::Layout;
use gui::mock::MockEngine;
use gui::side::{EngineSide, MockSide};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const NONE: Modifiers = Modifiers::NONE;
const SHIFT: Modifiers = Modifiers::SHIFT;
const ALT: Modifiers = Modifiers::ALT;
const NAV: KeyMode = KeyMode::Navigation;
const INTERACT: KeyMode = KeyMode::Interaction;
/// §G3's breadcrumb separator, from R-450's example "Console › Header › Copy".
const SEP: &str = " › ";

/// The header bar's elements in R-450's visual order, by A6's breadcrumb names.
const HEADER_ROW: [&str; 8] = [
    "All", "Warnings", "Errors", "Info", "Search", "Copy", "Clear", "Close",
];

/// Asserts that `check` panics: the check can fail (R-176's control, run in the test).
fn rejects(what: &str, check: impl FnOnce()) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check));
    assert!(
        outcome.is_err(),
        "the check passed on {what}, which it must reject"
    );
}

fn fresh() -> (App<MockSide>, Headless) {
    let mut app = App::new(MockSide::new(MockEngine::frozen(), None), FORMAT);
    let mut h = Headless::new(SIZE, PIXELS_PER_POINT);
    let _ = h.frame(&mut app, Vec::new());
    (app, h)
}

fn layout() -> Layout {
    Layout::new(
        Headless::new(SIZE, PIXELS_PER_POINT).screen(),
        PIXELS_PER_POINT,
    )
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

/// Presses and releases `key`, each its own frame, then one idle frame.
fn press(h: &mut Headless, app: &mut App<MockSide>, key: Key, modifiers: Modifiers) {
    let _ = h.frame(app, vec![key_event(key, true, modifiers)]);
    let _ = h.frame(app, vec![key_event(key, false, modifiers)]);
    let _ = h.frame(app, Vec::new());
}

fn presses(h: &mut Headless, app: &mut App<MockSide>, key: Key, modifiers: Modifiers, n: usize) {
    for _ in 0..n {
        press(h, app, key, modifiers);
    }
}

fn settle(h: &mut Headless, app: &mut App<MockSide>) {
    for _ in 0..4 {
        let _ = h.frame(app, Vec::new());
    }
}

/// Types `text` as a keyboard does: each letter's key press with its text, then the release.
fn type_text(h: &mut Headless, app: &mut App<MockSide>, text: &str) {
    for c in text.chars() {
        let key = Key::from_name(&c.to_uppercase().to_string()).expect("a letter key");
        let _ = h.frame(
            app,
            vec![key_event(key, true, NONE), Event::Text(c.to_string())],
        );
        let _ = h.frame(app, vec![key_event(key, false, NONE)]);
    }
    let _ = h.frame(app, Vec::new());
}

fn click(h: &mut Headless, app: &mut App<MockSide>, pos: Pos2) {
    let button = |pressed| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: NONE,
    };
    let _ = h.frame(app, vec![Event::PointerMoved(pos)]);
    let _ = h.frame(app, vec![button(true)]);
    let _ = h.frame(app, vec![button(false)]);
    let _ = h.frame(app, vec![Event::PointerGone]);
    settle(h, app);
}

/// The focused path's labels and the mode.
type At = (Vec<String>, KeyMode);

fn labels(app: &App<MockSide>) -> Vec<String> {
    app.keyboard
        .tree(app.view.mode)
        .labels(&app.view.focus.path)
        .into_iter()
        .map(str::to_owned)
        .collect()
}

fn at(app: &App<MockSide>) -> At {
    (labels(app), app.view.focus.mode)
}

fn want(path: &[&str], mode: KeyMode) -> At {
    (path.iter().map(|s| (*s).to_owned()).collect(), mode)
}

fn check_at(app: &App<MockSide>, path: &[&str], mode: KeyMode, what: &str) {
    assert_eq!(at(app), want(path, mode), "{what}");
}

/// The id of the focused scope.
fn focused_id(app: &App<MockSide>) -> String {
    app.view.focus.path.last().cloned().unwrap_or_default()
}

/// The id of the console's scope labelled `label` (A6's names), found in the tree under the console window.
fn console_id(app: &App<MockSide>, label: &str) -> &'static str {
    let tree = app.keyboard.tree(app.view.mode);
    let window = gui::windows::entry(Window::Console).scope;
    let mut stack = vec![window];
    while let Some(id) = stack.pop() {
        let path = tree.path_to(id);
        if tree.labels(&path).last() == Some(&label) {
            return id;
        }
        stack.extend(tree.children(Some(id)));
    }
    panic!("no console scope labelled `{label}`");
}

/// The rect scope `id` was placed on this frame.
fn place(app: &App<MockSide>, id: &str) -> Rect {
    app.keyboard
        .place_of(id)
        .unwrap_or_else(|| panic!("`{id}` not placed"))
        .rect
}

/// The window's rect.
fn window_rect(app: &App<MockSide>, window: Window) -> Rect {
    place(app, gui::windows::entry(window).scope)
}

/// One AccessKit node drawn this frame: its role, its text and its rect in points.
#[derive(Clone, Debug)]
struct Drawn {
    role: Role,
    text: String,
    rect: Rect,
}

fn drawn(h: &mut Headless, app: &mut App<MockSide>) -> Vec<Drawn> {
    let out = h.frame(app, Vec::new());
    let update = out
        .platform_output
        .accesskit_update
        .expect("an AccessKit tree");
    update
        .nodes
        .iter()
        .filter_map(|(_, n)| {
            let b = n.bounds()?;
            Some(Drawn {
                role: n.role(),
                text: n.label().or(n.value()).unwrap_or_default().to_owned(),
                rect: Rect::from_min_max(
                    egui::pos2(b.x0 as f32, b.y0 as f32),
                    egui::pos2(b.x1 as f32, b.y1 as f32),
                ),
            })
        })
        .collect()
}

/// Whether `role` is something the pointer acts on.
fn is_control(role: Role) -> bool {
    matches!(
        role,
        Role::Button
            | Role::DefaultButton
            | Role::CheckBox
            | Role::RadioButton
            | Role::Switch
            | Role::TextInput
            | Role::MultilineTextInput
            | Role::SearchInput
            | Role::SpinButton
            | Role::Slider
            | Role::ComboBox
            | Role::EditableComboBox
            | Role::Link
            | Role::MenuItem
            | Role::Tab
    )
}

/// The controls drawn with their centre inside `rect`.
fn controls_in(all: &[Drawn], rect: Rect) -> Vec<Drawn> {
    all.iter()
        .filter(|d| is_control(d.role) && rect.contains(d.rect.center()))
        .cloned()
        .collect()
}

// --- Openings ----------------------------------------------------------------------------------------------------------

/// Shift+Tab from no focus (the footer, R-405), then Enter: the console opened from the footer.
fn by_footer(h: &mut Headless, app: &mut App<MockSide>) {
    app.view.focus.path.clear();
    press(h, app, Key::Tab, SHIFT);
    check_at(app, &["Footer"], NAV, "Shift+Tab from no focus");
    press(h, app, Key::Enter, NONE);
    settle(h, app);
}

/// Tab to the top bar, Enter, → to `menu`, Enter, ↓ to `entry`; the focus left on the entry.
fn to_menu_entry(h: &mut Headless, app: &mut App<MockSide>, menu: &str, entry: &str) {
    app.view.focus.path.clear();
    press(h, app, Key::Tab, NONE);
    press(h, app, Key::Enter, NONE);
    for _ in 0..20 {
        if labels(app).last().is_some_and(|l| l == menu) {
            break;
        }
        press(h, app, Key::ArrowRight, NONE);
    }
    press(h, app, Key::Enter, NONE);
    for _ in 0..12 {
        if labels(app).last().is_some_and(|l| l == entry) {
            break;
        }
        press(h, app, Key::ArrowDown, NONE);
    }
    check_at(app, &["Top bar", menu, entry], NAV, "on the menu entry");
}

/// Windows › Console by the keys.
fn by_menu(h: &mut Headless, app: &mut App<MockSide>) {
    to_menu_entry(h, app, "Windows", "Console");
    press(h, app, Key::Enter, NONE);
    settle(h, app);
}

/// The ways and states the console is opened in for the scripts (R-450's "repeat all of this"): the footer, the menu,
/// an auto-open on error then Enter on the footer, and a filter that empties the list.
#[derive(Clone, Copy, Debug)]
enum Opened {
    Footer,
    Menu,
    AutoOpenThenFooter,
    EmptyFilter,
}

const OPENINGS: [Opened; 4] = [
    Opened::Footer,
    Opened::Menu,
    Opened::AutoOpenThenFooter,
    Opened::EmptyFilter,
];

/// The console opened as `opened`, a warning logged first; the focus on the landing. Returns where the focus goes back
/// to when it closes.
fn open(opened: Opened) -> (App<MockSide>, Headless, At) {
    let (mut app, mut h) = fresh();
    app.side().engine().raise_warning();
    settle(&mut h, &mut app);
    let footer = want(&["Footer"], NAV);
    let opener = match opened {
        Opened::Footer => {
            by_footer(&mut h, &mut app);
            footer
        }
        Opened::Menu => {
            by_menu(&mut h, &mut app);
            want(&["Top bar", "Windows", "Console"], NAV)
        }
        Opened::AutoOpenThenFooter => {
            app.side().engine().raise_error();
            settle(&mut h, &mut app);
            assert!(app.is_open(Window::Console), "an error opens the console");
            by_footer(&mut h, &mut app);
            footer
        }
        Opened::EmptyFilter => {
            // Only a warning logged: the errors filter, set by the keys, empties the list; Esc up to the landing.
            by_footer(&mut h, &mut app);
            press(&mut h, &mut app, Key::Enter, NONE);
            presses(&mut h, &mut app, Key::ArrowRight, NONE, 2);
            check_at(&app, &["Console", "Header", "Errors"], NAV, "→ to Errors");
            press(&mut h, &mut app, Key::Enter, NONE);
            press(&mut h, &mut app, Key::Escape, NONE);
            assert!(
                app.console().iter().all(|e| e.severity != Severity::Error),
                "no error logged"
            );
            footer
        }
    };
    assert!(app.is_open(Window::Console), "{opened:?}: open");
    check_at(
        &app,
        &["Console", "Header"],
        NAV,
        &format!("{opened:?}: the landing"),
    );
    (app, h, opener)
}

// --- REQ-GUI-189: the landing -----------------------------------------------------------------------------------------

#[test]
fn qa_r450_the_console_lands_on_its_header_bar_scope_from_the_footer_and_the_menu() {
    for opened in [Opened::Footer, Opened::Menu] {
        let (mut app, mut h, _) = open(opened);
        // The whole scope highlighted: the ring's rect is the header bar's, which holds every header control.
        let header = place(&app, &focused_id(&app));
        let all = drawn(&mut h, &mut app);
        let header_controls = controls_in(&all, header);
        assert!(
            header_controls.len() >= 8,
            "{opened:?}: the ring's rect holds {} controls, the header bar has eight",
            header_controls.len()
        );
        assert!(window_rect(&app, Window::Console).contains_rect(header));
        let landing = at(&app);
        rejects("a console landing on All", || {
            assert_eq!(landing, want(&["Console", "Header", "All"], NAV))
        });
    }
    // An error opening it takes no focus.
    let (mut app, mut h) = fresh();
    presses(&mut h, &mut app, Key::Tab, NONE, 3);
    let before = app.view.focus.clone();
    app.side().engine().raise_error();
    settle(&mut h, &mut app);
    assert!(app.is_open(Window::Console));
    assert_eq!(app.view.focus, before, "the auto-open took the focus");
}

// --- REQ-GUI-189: one script reaches everything, in one order, in every state ------------------------------------------

/// The script from the landing, with only ↑ ↓ ← →, Tab, Shift+Tab, Enter and Esc, every step's focus recorded.
fn script(h: &mut Headless, app: &mut App<MockSide>) -> Vec<At> {
    let mut trace = vec![at(app)];
    let mut step = |h: &mut Headless, app: &mut App<MockSide>, key: Key, m: Modifiers| {
        press(h, app, key, m);
        assert!(app.is_open(Window::Console), "{key:?} closed the console");
        trace.push(at(app));
    };
    // The top level: ← → do nothing, ↑ ↓ between the two scopes stopping at the ends, Tab and Shift+Tab between them.
    for (key, m) in [
        (Key::ArrowLeft, NONE),
        (Key::ArrowRight, NONE),
        (Key::ArrowUp, NONE),
        (Key::ArrowDown, NONE),
        (Key::ArrowDown, NONE),
        (Key::ArrowUp, NONE),
        (Key::Tab, NONE),
        (Key::Tab, NONE),
        (Key::Tab, SHIFT),
        (Key::Tab, SHIFT),
    ] {
        step(h, app, key, m);
    }
    // Into the header bar, → across it to × and past its end, ↑ ↓ doing nothing, ← back past its start, Esc up.
    step(h, app, Key::Enter, NONE);
    for _ in 0..8 {
        step(h, app, Key::ArrowRight, NONE);
    }
    step(h, app, Key::ArrowUp, NONE);
    step(h, app, Key::ArrowDown, NONE);
    for _ in 0..8 {
        step(h, app, Key::ArrowLeft, NONE);
    }
    step(h, app, Key::Escape, NONE);
    // Into the log and out.
    step(h, app, Key::ArrowDown, NONE);
    step(h, app, Key::Enter, NONE);
    step(h, app, Key::Escape, NONE);
    step(h, app, Key::ArrowUp, NONE);
    trace
}

/// The trace R-450 gives for [`script`].
fn expected() -> Vec<At> {
    let header = want(&["Console", "Header"], NAV);
    let log = want(&["Console", "Log"], NAV);
    let on = |c: &str| want(&["Console", "Header", c], NAV);
    let mut t = vec![header.clone()];
    // ← → ↑ on the header bar: nothing; ↓ the log, ↓ nothing, ↑ the header bar; Tab, Tab, Shift+Tab, Shift+Tab.
    t.extend([
        header.clone(),
        header.clone(),
        header.clone(),
        log.clone(),
        log.clone(),
        header.clone(),
        log.clone(),
        header.clone(),
        log.clone(),
        header.clone(),
    ]);
    // Enter: All; → ×8: Warnings to Close, then Close kept; ↑ ↓ nothing; ← ×8: Clear to All, then All kept.
    t.push(on("All"));
    for c in &HEADER_ROW[1..] {
        t.push(on(c));
    }
    t.extend([on("Close"), on("Close"), on("Close")]);
    for c in HEADER_ROW[..7].iter().rev() {
        t.push(on(c));
    }
    t.push(on("All"));
    // Esc: the header bar; ↓ the log; Enter interaction; Esc navigation; ↑ the header bar.
    t.extend([
        header.clone(),
        log.clone(),
        want(&["Console", "Log"], INTERACT),
        log,
        header,
    ]);
    t
}

/// Asserts that `trace` reached every header element and the log.
fn check_reaches_everything(trace: &[At], what: &str) {
    for c in HEADER_ROW {
        assert!(
            trace.iter().any(|(p, _)| p.last().is_some_and(|l| l == c)),
            "{what}: `{c}` never reached"
        );
    }
    assert!(
        trace
            .iter()
            .any(|(p, _)| p.last().is_some_and(|l| l == "Log")),
        "{what}: the log never reached"
    );
}

#[test]
fn qa_r450_one_script_reaches_every_header_element_the_log_and_close_in_every_state() {
    let expected = expected();
    let mut traces = Vec::new();
    for opened in OPENINGS {
        let (mut app, mut h, _) = open(opened);
        let trace = script(&mut h, &mut app);
        check_reaches_everything(&trace, &format!("{opened:?}"));
        assert_eq!(trace, expected, "{opened:?}: the script's trace");
        traces.push(trace);
    }
    // Controls: a trace that skips the log, one without ×, and one landing on All each fail.
    let t = traces[0].clone();
    let no_log: Vec<At> = t
        .iter()
        .filter(|(p, _)| p.last().is_none_or(|l| l != "Log"))
        .cloned()
        .collect();
    rejects("an order that skips the log", || {
        check_reaches_everything(&no_log, "no log")
    });
    let no_close: Vec<At> = t
        .iter()
        .filter(|(p, _)| p.last().is_none_or(|l| l != "Close"))
        .cloned()
        .collect();
    rejects("× outside the scope tree", || {
        check_reaches_everything(&no_close, "no ×")
    });
    let mut on_all = t;
    on_all[0] = want(&["Console", "Header", "All"], NAV);
    rejects("a landing on All", || assert_eq!(on_all, expected));
}

// --- REQ-GUI-189: closing, by × and by Esc, the focus back on the opener -----------------------------------------------

#[test]
fn qa_r450_enter_on_close_and_esc_at_the_top_level_close_back_to_the_opener() {
    for opened in OPENINGS {
        // Enter on ×.
        let (mut app, mut h, opener) = open(opened);
        press(&mut h, &mut app, Key::Enter, NONE);
        presses(&mut h, &mut app, Key::ArrowRight, NONE, 7);
        check_at(&app, &["Console", "Header", "Close"], NAV, "→ to ×");
        press(&mut h, &mut app, Key::Enter, NONE);
        settle(&mut h, &mut app);
        assert!(
            !app.is_open(Window::Console),
            "{opened:?}: Enter on × left it open"
        );
        assert_eq!(at(&app), opener, "{opened:?}: Enter on ×");
        // Esc on the header bar scope.
        let (mut app, mut h, opener) = open(opened);
        press(&mut h, &mut app, Key::Escape, NONE);
        assert!(
            !app.is_open(Window::Console),
            "{opened:?}: Esc on the header bar"
        );
        assert_eq!(at(&app), opener, "{opened:?}: Esc on the header bar");
        // Esc on the log scope.
        let (mut app, mut h, opener) = open(opened);
        press(&mut h, &mut app, Key::ArrowDown, NONE);
        check_at(&app, &["Console", "Log"], NAV, "↓ to the log");
        press(&mut h, &mut app, Key::Escape, NONE);
        assert!(!app.is_open(Window::Console), "{opened:?}: Esc on the log");
        assert_eq!(at(&app), opener, "{opened:?}: Esc on the log");
        // Esc on a header element goes up, not out; Esc in interaction on the log to navigation, not out.
        let (mut app, mut h, _) = open(opened);
        press(&mut h, &mut app, Key::Enter, NONE);
        presses(&mut h, &mut app, Key::ArrowRight, NONE, 5);
        check_at(&app, &["Console", "Header", "Copy"], NAV, "→ to Copy");
        press(&mut h, &mut app, Key::Escape, NONE);
        let open_after = app.is_open(Window::Console);
        assert!(open_after, "{opened:?}: Esc on Copy closed the console");
        check_at(&app, &["Console", "Header"], NAV, "Esc on Copy");
        rejects("an Esc on a header element that closes the console", || {
            assert!(!open_after)
        });
        press(&mut h, &mut app, Key::ArrowDown, NONE);
        press(&mut h, &mut app, Key::Enter, NONE);
        press(&mut h, &mut app, Key::Escape, NONE);
        assert!(app.is_open(Window::Console));
        check_at(
            &app,
            &["Console", "Log"],
            NAV,
            "Esc in interaction on the log",
        );
    }
}

// --- REQ-GUI-189: the search field (§G3's text field) -----------------------------------------------------------------

#[test]
fn qa_r450_the_search_field_types_after_enter_and_keeps_its_text() {
    let (mut app, mut h, _) = open(Opened::Footer);
    press(&mut h, &mut app, Key::Enter, NONE);
    presses(&mut h, &mut app, Key::ArrowRight, NONE, 4);
    check_at(
        &app,
        &["Console", "Header", "Search"],
        NAV,
        "→ to the search field",
    );
    type_text(&mut h, &mut app, "q");
    assert_eq!(app.windows.console.text, "", "a letter typed navigating");
    press(&mut h, &mut app, Key::Enter, NONE);
    check_at(
        &app,
        &["Console", "Header", "Search"],
        INTERACT,
        "Enter on the field",
    );
    type_text(&mut h, &mut app, "ab");
    press(&mut h, &mut app, Key::Escape, NONE);
    check_at(
        &app,
        &["Console", "Header", "Search"],
        NAV,
        "Esc while typing",
    );
    assert!(app.is_open(Window::Console));
    assert_eq!(app.windows.console.text, "ab", "Esc kept the text");
    press(&mut h, &mut app, Key::Enter, NONE);
    type_text(&mut h, &mut app, "c");
    press(&mut h, &mut app, Key::Enter, NONE);
    check_at(
        &app,
        &["Console", "Header", "Search"],
        NAV,
        "Enter while typing",
    );
    assert_eq!(app.windows.console.text, "abc", "Enter kept the text");
    // Navigating again, → goes on to Copy.
    press(&mut h, &mut app, Key::ArrowRight, NONE);
    check_at(
        &app,
        &["Console", "Header", "Copy"],
        NAV,
        "→ from the field",
    );
    let text = app.windows.console.text.clone();
    rejects("a field that loses its text", || assert_eq!(text, ""));
}

// --- REQ-GUI-189: the breadcrumb ---------------------------------------------------------------------------------------

/// The texts drawn in the top bar.
fn top_bar_texts(h: &mut Headless, app: &mut App<MockSide>) -> Vec<String> {
    let bar = layout().top_bar;
    drawn(h, app)
        .into_iter()
        .filter(|d| bar.contains(d.rect.center()))
        .map(|d| d.text)
        .collect()
}

fn check_crumb(texts: &[String], crumb: &str, what: &str) {
    assert!(
        texts.iter().any(|t| t.contains(crumb)),
        "{what}: no breadcrumb `{crumb}` in the top bar's {texts:?}"
    );
}

#[test]
fn qa_r450_the_breadcrumb_follows_the_two_scopes() {
    let (mut app, mut h, _) = open(Opened::Footer);
    let header = top_bar_texts(&mut h, &mut app);
    check_crumb(&header, &["Console", "Header"].join(SEP), "the landing");
    press(&mut h, &mut app, Key::Enter, NONE);
    presses(&mut h, &mut app, Key::ArrowRight, NONE, 5);
    let copy = top_bar_texts(&mut h, &mut app);
    check_crumb(&copy, "Console › Header › Copy", "on Copy");
    rejects("R-405's breadcrumb on copy", || {
        check_crumb(&copy, "Console › copy and clear › copy", "on Copy")
    });
    presses(&mut h, &mut app, Key::ArrowRight, NONE, 2);
    check_crumb(
        &top_bar_texts(&mut h, &mut app),
        "Console › Header › Close",
        "on ×",
    );
    press(&mut h, &mut app, Key::Escape, NONE);
    press(&mut h, &mut app, Key::ArrowDown, NONE);
    let log = top_bar_texts(&mut h, &mut app);
    check_crumb(&log, "Console › Log", "on the log");
    rejects("the header's breadcrumb on the log", || {
        check_crumb(&log, "Console › Header", "on the log")
    });
}

// --- REQ-GUI-189: the log scrolls over the whole filtered list ---------------------------------------------------------

/// The indices, in the shown list `messages` (newest first), of the first and last row drawn whole inside the log.
fn rows_drawn(h: &mut Headless, app: &mut App<MockSide>, messages: &[String]) -> (usize, usize) {
    let log = place(app, console_id(app, "Log"));
    let idx: Vec<usize> = drawn(h, app)
        .into_iter()
        .filter(|d| log.expand(0.5).contains_rect(d.rect))
        .filter_map(|d| messages.iter().position(|m| *m == d.text))
        .collect();
    assert!(!idx.is_empty(), "no row drawn in the log");
    (
        *idx.iter().min().expect("a row"),
        *idx.iter().max().expect("a row"),
    )
}

/// An app whose console holds `n` info entries with distinct messages, a warning after each, the console opened from
/// the footer and filtered to Info by the keys, the focus in interaction on the log. Returns the shown messages, newest
/// first.
fn long_filtered_log(n: usize) -> (App<MockSide>, Headless, Vec<String>) {
    let (mut app, mut h) = fresh();
    let t0 = app.snapshot().render.playhead.t;
    for i in 0..n {
        app.set_field(SetField {
            edit: Edit::Render(RenderField::Playhead(Playhead {
                t: t0 + 0.5 * (i + 1) as f64,
            })),
            no_history: false,
        });
        let _ = h.frame(&mut app, Vec::new());
        app.side().engine().raise_warning();
        let _ = h.frame(&mut app, Vec::new());
    }
    settle(&mut h, &mut app);
    by_footer(&mut h, &mut app);
    // Info, by the keys: Enter, → three times, Enter; Esc up; ↓ to the log; Enter.
    press(&mut h, &mut app, Key::Enter, NONE);
    presses(&mut h, &mut app, Key::ArrowRight, NONE, 3);
    check_at(&app, &["Console", "Header", "Info"], NAV, "→ to Info");
    press(&mut h, &mut app, Key::Enter, NONE);
    press(&mut h, &mut app, Key::Escape, NONE);
    press(&mut h, &mut app, Key::ArrowDown, NONE);
    press(&mut h, &mut app, Key::Enter, NONE);
    check_at(&app, &["Console", "Log"], INTERACT, "Enter on the log");
    settle(&mut h, &mut app);
    let messages: Vec<String> = app
        .console()
        .iter()
        .rev()
        .filter(|e| e.severity == Severity::Info)
        .map(|e| e.message.clone())
        .collect();
    assert!(messages.len() >= n, "{} info rows", messages.len());
    let mut distinct = messages.clone();
    distinct.sort();
    distinct.dedup();
    assert_eq!(
        distinct.len(),
        messages.len(),
        "the info rows' messages differ"
    );
    (app, h, messages)
}

#[test]
fn qa_r450_the_log_scrolls_by_entry_page_and_end_over_the_whole_filtered_list() {
    let (mut app, mut h, messages) = long_filtered_log(60);
    let (h, app) = (&mut h, &mut app);
    let rows = messages.len();
    let (top0, bottom0) = rows_drawn(h, app, &messages);
    assert_eq!(top0, 0, "the newest row first, unscrolled");
    // A page is the entries the log's rect shows (A4).
    let page = bottom0 - top0 + 1;
    assert!(
        page > 1 && rows > 3 * page + 12,
        "{rows} rows, {page} drawn"
    );
    let unscrolled = bottom0;
    rejects("a list no longer than the rows drawn", || {
        assert_eq!(unscrolled, rows - 1)
    });
    let last_top = rows - page;
    let mut from = 0;
    for (key, m, to) in [
        (Key::ArrowDown, NONE, 1),
        (Key::ArrowDown, SHIFT, 11),
        (Key::ArrowUp, NONE, 10),
        (Key::ArrowDown, ALT, 11),
        (Key::PageDown, NONE, 11 + page),
        (Key::PageUp, NONE, 11),
        (Key::Home, NONE, 0),
        (Key::ArrowUp, NONE, 0),
        (Key::End, NONE, last_top),
        (Key::ArrowDown, NONE, last_top),
        (Key::PageDown, NONE, last_top),
        (Key::PageUp, NONE, last_top - page),
        (Key::PageUp, SHIFT, 0),
        (Key::PageDown, SHIFT, last_top),
        (Key::ArrowUp, SHIFT, last_top - 10),
    ] {
        press(h, app, key, m);
        settle(h, app);
        let (top, bottom) = rows_drawn(h, app, &messages);
        assert_eq!(top, to, "{key:?} {m:?} from the top row {from}");
        assert_eq!(bottom - top + 1, page, "{key:?} {m:?}: the rows drawn");
        from = to;
    }
    // End: the last entry, the oldest info row, drawn.
    press(h, app, Key::End, NONE);
    settle(h, app);
    let (_, end_bottom) = rows_drawn(h, app, &messages);
    assert_eq!(end_bottom, rows - 1, "End leaves the last entry undrawn");
    rejects("a scroll that stops at the last row drawn", || {
        assert_eq!(end_bottom, unscrolled)
    });
    // ↓ alone reaches the last entry from the first: one entry a press, over the whole list.
    press(h, app, Key::Home, NONE);
    for _ in 0..last_top + 3 {
        press(h, app, Key::ArrowDown, NONE);
    }
    settle(h, app);
    assert_eq!(rows_drawn(h, app, &messages).1, rows - 1, "↓ to the end");
    // Page Down alone reaches it too.
    press(h, app, Key::Home, NONE);
    for _ in 0..rows / page + 2 {
        press(h, app, Key::PageDown, NONE);
    }
    settle(h, app);
    assert_eq!(
        rows_drawn(h, app, &messages).1,
        rows - 1,
        "Page Down to the end"
    );
    press(h, app, Key::Escape, NONE);
    check_at(app, &["Console", "Log"], NAV, "Esc on the log");
    // Navigating, the scroll keys move nothing.
    press(h, app, Key::Home, NONE);
    settle(h, app);
    assert_eq!(rows_drawn(h, app, &messages).1, rows - 1, "Home navigating");
}

#[test]
fn qa_r450_an_empty_list_keeps_the_log_in_place() {
    let (mut app, mut h, _) = open(Opened::EmptyFilter);
    press(&mut h, &mut app, Key::Tab, NONE);
    check_at(
        &app,
        &["Console", "Log"],
        NAV,
        "Tab to the log of an empty list",
    );
    let after_tab = at(&app);
    rejects("an order that skips the log when the list is empty", || {
        assert_eq!(after_tab, want(&["Console", "Header"], NAV))
    });
    press(&mut h, &mut app, Key::Enter, NONE);
    check_at(&app, &["Console", "Log"], INTERACT, "Enter on an empty log");
    for key in [
        Key::ArrowDown,
        Key::PageDown,
        Key::End,
        Key::Home,
        Key::PageUp,
        Key::ArrowUp,
    ] {
        press(&mut h, &mut app, key, NONE);
        check_at(
            &app,
            &["Console", "Log"],
            INTERACT,
            &format!("{key:?} on an empty log"),
        );
    }
    press(&mut h, &mut app, Key::Escape, NONE);
    check_at(&app, &["Console", "Log"], NAV, "Esc");
    press(&mut h, &mut app, Key::Tab, NONE);
    check_at(&app, &["Console", "Header"], NAV, "Tab back");
}

// --- REQ-GUI-189 A3 / REQ-GUI-190: a click moves the keyboard's focus; the intermittent fault ---------------------------

#[test]
fn qa_r450_a_click_on_a_console_control_moves_the_keyboards_focus_onto_it() {
    let (mut app, mut h, opener) = open(Opened::Footer);
    for c in ["Copy", "Warnings", "Search"] {
        let rect = place(&app, console_id(&app, c));
        click(&mut h, &mut app, rect.center());
        let (path, _) = at(&app);
        assert_eq!(path, ["Console", "Header", c], "a click on {c}");
        rejects("a click that moves egui's focus alone", || {
            assert_eq!(path, ["Console", "Header"])
        });
    }
    // The log too.
    let rect = place(&app, console_id(&app, "Log"));
    click(&mut h, &mut app, rect.center());
    check_at(&app, &["Console", "Log"], NAV, "a click on the log");
    // A click on × closes the console as Enter on it does, the focus back on its opener.
    let rect = place(&app, console_id(&app, "Close"));
    click(&mut h, &mut app, rect.center());
    assert!(!app.is_open(Window::Console), "a click on × left it open");
    assert_eq!(at(&app), opener, "a click on ×");
}

/// Asserts that the keyboard's mode `mode` and whether the focused text field takes letters agree: a field types only in
/// interaction mode (§G3, R-409 G4).
fn check_mode_agrees(mode: KeyMode, typed: bool, what: &str) {
    assert_eq!(
        mode == INTERACT,
        typed,
        "{what}: the mode is {mode:?}, and the field {} letters",
        if typed { "takes" } else { "does not take" }
    );
}

#[test]
fn qa_r450_after_an_auto_open_a_click_then_the_keys_reach_copy_clear_and_close() {
    // The focus on the Figure; an error opens the console without it.
    let (mut app, mut h) = fresh();
    presses(&mut h, &mut app, Key::Tab, NONE, 3);
    check_at(&app, &["Figure"], NAV, "three Tabs");
    let before = at(&app);
    app.side().engine().raise_error();
    settle(&mut h, &mut app);
    assert!(app.is_open(Window::Console));
    assert_eq!(at(&app), before, "the auto-open took the focus");
    // A click on the search field: the keyboard's focus there (A3), typing at once, which is interaction mode, the ring
    // blue (R-456; §G3, R-409 G4). The mode the ring shows and what the keys do agree.
    let rect = place(&app, console_id(&app, "Search"));
    click(&mut h, &mut app, rect.center());
    let (path, mode) = at(&app);
    assert_eq!(
        path,
        ["Console", "Header", "Search"],
        "a click on the field"
    );
    type_text(&mut h, &mut app, "m");
    let typed = app.windows.console.text == "m";
    check_mode_agrees(mode, typed, "after a click on the search field");
    rejects("a field typing while the ring shows navigation", || {
        check_mode_agrees(NAV, true, "control")
    });
    assert!(typed, "a clicked search field typed nothing (R-456)");
    press(&mut h, &mut app, Key::Escape, NONE);
    check_at(
        &app,
        &["Console", "Header", "Search"],
        NAV,
        "Esc ends the typing",
    );
    let mut reached = Vec::new();
    for _ in 0..3 {
        press(&mut h, &mut app, Key::ArrowRight, NONE);
        reached.push(labels(&app).last().cloned().unwrap_or_default());
    }
    assert_eq!(reached, ["Copy", "Clear", "Close"], "→ from the field");
    press(&mut h, &mut app, Key::Escape, NONE);
    press(&mut h, &mut app, Key::ArrowDown, NONE);
    check_at(&app, &["Console", "Log"], NAV, "↓ to the log");
    // Esc at the top level: closed, the focus back where it was before the auto-open (A2).
    press(&mut h, &mut app, Key::Escape, NONE);
    assert!(!app.is_open(Window::Console));
    assert_eq!(at(&app), before, "Esc after an auto-open");
}

/// A click on the search field starts typing (R-456), and §G3's typing rules hold (R-409 G4; R-456 B1): Tab leaves it
/// for the next scope, Enter stops typing and keeps the text, the arrows then moving on.
#[test]
fn qa_r450_a_clicked_search_field_follows_g3s_text_field() {
    for key in [Key::Enter, Key::Tab] {
        let (mut app, mut h, _) = open(Opened::Footer);
        let rect = place(&app, console_id(&app, "Search"));
        click(&mut h, &mut app, rect.center());
        check_at(
            &app,
            &["Console", "Header", "Search"],
            INTERACT,
            "a click on the field (R-456)",
        );
        type_text(&mut h, &mut app, "ab");
        assert_eq!(
            app.windows.console.text, "ab",
            "a clicked search field typed nothing (R-456)"
        );
        press(&mut h, &mut app, key, NONE);
        assert_eq!(app.windows.console.text, "ab", "{key:?} kept the text");
        assert!(app.is_open(Window::Console));
        if key == Key::Tab {
            check_at(
                &app,
                &["Console", "Log"],
                NAV,
                "Tab while typing after a click",
            );
        } else {
            check_at(
                &app,
                &["Console", "Header", "Search"],
                NAV,
                "Enter while typing after a click",
            );
            press(&mut h, &mut app, Key::ArrowRight, NONE);
            check_at(&app, &["Console", "Header", "Copy"], NAV, "→ after Enter");
        }
    }
}

// --- REQ-GUI-190: every drawn control reached, in every state; every other window --------------------------------------

/// The scopes the keys reach from `start`, staying in the window `within`: §G3's keys applied to the tree from every
/// focus reached, breadth first, Esc out of the window not followed. In the order first reached.
fn reach(tree: &ScopeTree, start: &Focus, within: &str) -> Vec<String> {
    let keys = [
        (Key::ArrowUp, NONE),
        (Key::ArrowDown, NONE),
        (Key::ArrowLeft, NONE),
        (Key::ArrowRight, NONE),
        (Key::Tab, NONE),
        (Key::Tab, SHIFT),
        (Key::Enter, NONE),
        (Key::Escape, NONE),
    ];
    let commands: Vec<_> = keys
        .iter()
        .map(|(k, m)| keymap::command(*k, *m).expect("a §G3 key"))
        .collect();
    let mut seen = vec![(start.path.clone(), start.mode)];
    let mut queue = std::collections::VecDeque::from([start.clone()]);
    let mut reached: Vec<String> = start.path.clone();
    while let Some(focus) = queue.pop_front() {
        for c in &commands {
            let mut next = focus.clone();
            if let Outcome::Leave(_) = tree.navigate(&mut next, *c, 1.0) {
                continue;
            }
            if next.path.first().map(String::as_str) != Some(within) {
                continue;
            }
            if seen.contains(&(next.path.clone(), next.mode)) {
                continue;
            }
            seen.push((next.path.clone(), next.mode));
            for id in &next.path {
                if !reached.contains(id) {
                    reached.push(id.clone());
                }
            }
            queue.push_back(next);
        }
    }
    reached
}

/// The controls of `controls` that no reached leaf scope's placed rect holds (a scope with children holds none: each of
/// its children must be reached itself).
fn unreached(app: &App<MockSide>, reached: &[String], controls: &[Drawn]) -> Vec<String> {
    let tree = app.keyboard.tree(app.view.mode);
    let rects: Vec<Rect> = reached
        .iter()
        .filter(|id| tree.children(Some(id)).is_empty())
        .filter_map(|id| app.keyboard.place_of(id).map(|p| p.rect))
        .collect();
    controls
        .iter()
        .filter(|c| {
            !rects
                .iter()
                .any(|r| r.expand(0.5).contains(c.rect.center()))
        })
        .map(|c| format!("{:?} `{}`", c.role, c.text))
        .collect()
}

#[test]
fn qa_r450_every_drawn_console_control_is_reached_in_every_state_in_one_order() {
    let mut orders = Vec::new();
    for opened in OPENINGS {
        let (mut app, mut h, _) = open(opened);
        let all = drawn(&mut h, &mut app);
        let controls = controls_in(&all, window_rect(&app, Window::Console));
        assert!(
            controls.len() >= 8,
            "{opened:?}: {} controls drawn",
            controls.len()
        );
        let tree = app.keyboard.tree(app.view.mode);
        let window = gui::windows::entry(Window::Console).scope;
        let reached = reach(tree, &app.view.focus, window);
        let missed = unreached(&app, &reached, &controls);
        assert!(missed.is_empty(), "{opened:?}: unreached {missed:?}");
        // Control: × left out of the reached scopes is named.
        let close = console_id(&app, "Close").to_owned();
        let without: Vec<String> = reached.iter().filter(|r| **r != close).cloned().collect();
        let missed_close = unreached(&app, &without, &controls);
        rejects("× outside the scope tree", || {
            assert!(missed_close.is_empty())
        });
        orders.push(reached);
    }
    for (o, opened) in orders.iter().zip(OPENINGS) {
        assert_eq!(o, &orders[0], "{opened:?}: the order depends on state");
    }
}

#[test]
fn qa_r450_every_other_window_reaches_every_drawn_control_and_its_close() {
    let windows = [
        (Window::Run, "Run"),
        (Window::Profiler, "Profiler"),
        (Window::Export, "Export & share"),
        (Window::Display, "Display"),
        (Window::ChartBuilder, "Chart builder"),
        (Window::Inspector, "Inspector"),
    ];
    for mode in [Mode::Explore, Mode::Stain] {
        for (window, name) in windows {
            let (mut app, mut h) = fresh();
            app.view.mode = mode;
            settle(&mut h, &mut app);
            let before = drawn(&mut h, &mut app);
            to_menu_entry(&mut h, &mut app, "Windows", name);
            let opener = at(&app);
            press(&mut h, &mut app, Key::Enter, NONE);
            settle(&mut h, &mut app);
            assert!(app.is_open(window), "{name} on {mode:?}: opened");
            let after = drawn(&mut h, &mut app);
            let rect = window_rect(&app, window);
            // What the window drew: the controls inside its rect it had not drawn before it opened.
            let controls: Vec<Drawn> = controls_in(&after, rect)
                .into_iter()
                .filter(|c| {
                    !before
                        .iter()
                        .any(|b| b.role == c.role && b.rect == c.rect && b.text == c.text)
                })
                .collect();
            assert!(!controls.is_empty(), "{name}: no control drawn");
            let x = controls
                .iter()
                .find(|c| c.text == "×")
                .unwrap_or_else(|| panic!("{name} on {mode:?}: no × drawn"))
                .rect;
            let tree = app.keyboard.tree(app.view.mode);
            let scope = gui::windows::entry(window).scope;
            let reached = reach(tree, &app.view.focus, scope);
            let missed = unreached(&app, &reached, &controls);
            assert!(
                missed.is_empty(),
                "{name} on {mode:?}: unreached {missed:?}"
            );
            // Control: the scope placed on the × left out, the × is named.
            let without: Vec<String> = reached
                .iter()
                .filter(|id| {
                    app.keyboard
                        .place_of(id)
                        .is_none_or(|p| !p.rect.expand(0.5).contains(x.center()))
                })
                .cloned()
                .collect();
            let missed_x = unreached(&app, &without, &controls);
            rejects(&format!("{name}'s × outside the tree"), || {
                assert!(missed_x.is_empty())
            });
            // The ×, by the keys: Tab until it, Enter; the window closes, the focus back on its menu entry.
            let mut found = false;
            for _ in 0..40 {
                if labels(&app).last().is_some_and(|l| l == "Close") {
                    found = true;
                    break;
                }
                press(&mut h, &mut app, Key::Tab, NONE);
                assert!(app.is_open(window), "{name}: Tab left the window");
            }
            assert!(found, "{name} on {mode:?}: Tab never reached ×");
            press(&mut h, &mut app, Key::Enter, NONE);
            settle(&mut h, &mut app);
            assert!(!app.is_open(window), "{name}: Enter on × left it open");
            assert_eq!(at(&app), opener, "{name}: Enter on ×");
        }
    }
}
