//! QA tests for TASK-M6-28, written from the requirements and render_gui_spec §G3, §G5, §G7, §G8, §G12 and §G13, not
//! from the implementation:
//!
//! - REQ-GUI-178 (R-405, R-409 F2, G4–G6): the footer is big scope 8, after Legend, Tab wrapping from it to the top bar
//!   and the first Shift+Tab from no focus landing on it; Enter on it (or a click) opens the console with the focus on
//!   its first filter, in navigation mode; the console's sections, in Tab order, are the filters, the text filter, copy
//!   and clear, and the entry list, Tab and Shift+Tab moving between them inside the console; the arrows move within a
//!   section; Enter acts on a filter, copy or clear as its click; the text filter types only after Enter, every key but
//!   Esc, Enter and Tab going to the field, `?` typing and Ctrl+Z undoing the text edit; the entry list scrolls with
//!   ↑ ↓ only in interaction mode; Tab from interaction mode moves to the next section, staying in the console; Esc in
//!   navigation mode closes it, the focus going back to whatever opened it, the footer or Windows › Console; an error
//!   opens it by itself without moving the focus.
//!   R-450 (A9) replaces the console's structure: two scopes, the header bar (All, Warnings, Errors, Info, the search
//!   field, Copy, Clear, ×) and the log; opening it lands on the header bar scope itself; Esc at its top level closes
//!   it. The tests below that asserted the four sections, the landing on the first filter or Esc from a section are
//!   changed to R-450's structure (TASK-M6-32's qa commit, R-290); REQ-GUI-189 is tested in `qa_TASK-M6-32`.
//! - REQ-GUI-126: the console is the footer opened, with severity, time, source and message columns, the filters (all,
//!   warnings, errors, info, text), copy and clear, and it opens automatically on an error.
//! - REQ-GUI-174: each window opens from its top-bar or menu entry; the Chart builder's two previews are square for a
//!   square viewport; the windows' content as §G5, §G7, §G8 and §G13 give it (the Run window under the contract's
//!   names, no "tolerance" field and no persistence count; Display's fixed order, "Legend" never "Fate"; the
//!   Profiler's tabs and its trace in profiler schema v1's shape; Export & share's image, state and present; the
//!   Inspector's two tabs, three panes and gauge buttons). The screenshots against the artboards are the gui
//!   reviewer's and `cargo xtask screenshot`'s.
//!
//! The focus is read from `ViewUI` (gui_state_contract §2), named by the tree's labels; what is drawn is read from the
//! app's AccessKit names. gui takes no dependency on validation (R-187), so each check runs its own control (R-176)
//! in the test, through [`rejects`] or by running the same check where it must come out the other way.
// The file name `qa_TASK-M6-28` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use eframe::egui::{self, Event, Key, Modifiers, OutputCommand, PointerButton, Pos2};
use engine::contract::profile;
use engine::contract::render_state::Playhead;
use engine::contract::set_field::{Edit, RenderField, SetField};
use engine::contract::view_ui::{KeyMode, Window};
use gui::app::App;
use gui::capture::{PIXELS_PER_POINT, SIZE};
use gui::headless::{Headless, Name};
use gui::layout::Layout;
use gui::mock::{MockEngine, MOCK_ERROR, MOCK_WARNING};
use gui::side::{EngineSide, MockSide};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const NONE: Modifiers = Modifiers::NONE;
const SHIFT: Modifiers = Modifiers::SHIFT;
const NAV: KeyMode = KeyMode::Navigation;
const INTERACT: KeyMode = KeyMode::Interaction;
/// §G3's breadcrumb separator, from its example "Manifold view › Navigate › zoom".
const SEP: &str = " › ";

/// §G3's big scopes, in Tab order, the footer last (R-405).
const BIG: [&str; 8] = [
    "top bar",
    "Manifold view",
    "Figure",
    "Trajectory",
    "Compass",
    "Time",
    "Legend",
    "Footer",
];

/// The seven windows the task builds, by their §G5, §G7, §G8 and §G12 names, as the Windows menu lists them.
const WINDOWS: [(Window, &str); 7] = [
    (Window::Run, "Run"),
    (Window::Profiler, "Profiler"),
    (Window::Export, "Export & share"),
    (Window::Display, "Display"),
    (Window::Console, "Console"),
    (Window::ChartBuilder, "Chart builder"),
    (Window::Inspector, "Inspector"),
];

/// Asserts that `check` panics: the check can fail (R-176's control, run in the test).
fn rejects(what: &str, check: impl FnOnce()) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check));
    assert!(
        outcome.is_err(),
        "the check passed on {what}, which it must reject"
    );
}

fn mock_app() -> App<MockSide> {
    App::new(MockSide::new(MockEngine::frozen(), None), FORMAT)
}

fn headless() -> Headless {
    Headless::new(SIZE, PIXELS_PER_POINT)
}

fn layout() -> Layout {
    Layout::new(headless().screen(), PIXELS_PER_POINT)
}

/// An app and its headless context, one frame run.
fn fresh() -> (App<MockSide>, Headless) {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
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

/// A few idle frames: egui lays a new window out in an invisible sizing pass first.
fn settle(h: &mut Headless, app: &mut App<MockSide>) {
    for _ in 0..4 {
        let _ = h.frame(app, Vec::new());
    }
}

/// Types `text` as a keyboard does: each character's key press with its text, then the release.
fn type_text(h: &mut Headless, app: &mut App<MockSide>, text: &str) {
    for c in text.chars() {
        let (key, modifiers) = match c {
            '?' => (Key::Questionmark, SHIFT),
            ' ' => (Key::Space, NONE),
            c => (
                Key::from_name(&c.to_uppercase().to_string()).expect("a letter key"),
                NONE,
            ),
        };
        let _ = h.frame(
            app,
            vec![key_event(key, true, modifiers), Event::Text(c.to_string())],
        );
        let _ = h.frame(app, vec![key_event(key, false, modifiers)]);
    }
    let _ = h.frame(app, Vec::new());
}

/// The labels of the focused path, as the tree names them.
fn labels(app: &App<MockSide>) -> Vec<String> {
    app.keyboard
        .tree(app.view.mode)
        .labels(&app.view.focus.path)
        .into_iter()
        .map(str::to_owned)
        .collect()
}

fn lower(s: &str) -> String {
    s.to_lowercase()
}

fn same(a: &str, b: &str) -> bool {
    lower(a) == lower(b)
}

fn check_path(got: &[String], want: &[&str], what: &str) {
    assert!(
        got.len() == want.len() && got.iter().zip(want).all(|(g, w)| same(g, w)),
        "{what}: the focus is {got:?}, the spec gives {want:?}"
    );
}

/// The window the focus is in, by its first label, or none.
fn in_console(path: &[String]) -> bool {
    path.first().is_some_and(|w| same(w, "Console"))
}

/// The console scope a focus path is in, as R-450 orders them (A6's names): 0 the header bar, 1 the log. Read from
/// the scope's label.
fn section_of(path: &[String]) -> Option<usize> {
    if !in_console(path) {
        return None;
    }
    let s = lower(path.get(1)?);
    if s == "header" {
        Some(0)
    } else if s == "log" {
        Some(1)
    } else {
        None
    }
}

fn check_console_at(
    app: &App<MockSide>,
    section: usize,
    last: Option<&str>,
    mode: KeyMode,
    what: &str,
) {
    let path = labels(app);
    assert_eq!(
        section_of(&path),
        Some(section),
        "{what}: the focus is {path:?}"
    );
    if let Some(last) = last {
        assert!(
            path.last().is_some_and(|l| same(l, last)),
            "{what}: the focus is {path:?}, not on `{last}`"
        );
    }
    assert_eq!(app.view.focus.mode, mode, "{what}: the mode");
}

fn names(h: &mut Headless, app: &mut App<MockSide>) -> Vec<Name> {
    let out = h.frame(app, Vec::new());
    h.names(&out)
}

/// The names drawn inside `rect` (points).
fn names_in(names: &[Name], rect: egui::Rect) -> Vec<Name> {
    let p = f64::from(PIXELS_PER_POINT);
    let [x0, y0, x1, y1] = [
        f64::from(rect.min.x) * p,
        f64::from(rect.min.y) * p,
        f64::from(rect.max.x) * p,
        f64::from(rect.max.y) * p,
    ];
    names
        .iter()
        .filter(|n| {
            n.rect.is_some_and(|r| {
                r[0] >= x0 - 0.5 && r[1] >= y0 - 0.5 && r[2] <= x1 + 0.5 && r[3] <= y1 + 0.5
            })
        })
        .cloned()
        .collect()
}

fn texts(names: &[Name]) -> Vec<String> {
    names.iter().map(|n| n.name.clone()).collect()
}

/// The centre of the first name that reads exactly `text`, in points.
fn centre_of(names: &[Name], text: &str) -> Option<Pos2> {
    let p = PIXELS_PER_POINT;
    names
        .iter()
        .filter(|n| n.name == text)
        .find_map(|n| n.rect)
        .map(|r| {
            egui::pos2(
                ((r[0] + r[2]) / 2.0) as f32 / p,
                ((r[1] + r[3]) / 2.0) as f32 / p,
            )
        })
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
    let _ = h.frame(app, Vec::new());
}

/// The rect window `window` was drawn in, in points.
fn window_rect(app: &App<MockSide>, window: Window) -> egui::Rect {
    app.keyboard
        .place_of(gui::windows::entry(window).scope)
        .expect("the window placed")
        .rect
}

/// A contract edit with history, which the contract logs as one `info` entry from `contract`.
fn playhead_edit(t: f64) -> SetField {
    SetField {
        edit: Edit::Render(RenderField::Playhead(Playhead { t })),
        no_history: false,
    }
}

/// The focus after Shift+Tab from no focus (the footer), then Enter: the console opened by the footer.
fn console_by_footer(h: &mut Headless, app: &mut App<MockSide>) {
    app.view.focus.path.clear();
    press(h, app, Key::Tab, SHIFT);
    check_path(&labels(app), &["Footer"], "Shift+Tab from no focus");
    press(h, app, Key::Enter, NONE);
    settle(h, app);
}

/// Walks to the top bar's `control` by the keys from no focus: Tab, Enter into the top bar, → along it.
fn to_top_bar_control(h: &mut Headless, app: &mut App<MockSide>, control: &str) {
    app.view.focus.path.clear();
    press(h, app, Key::Tab, NONE);
    press(h, app, Key::Enter, NONE);
    for _ in 0..20 {
        if labels(app).last().is_some_and(|l| same(l, control)) {
            return;
        }
        press(h, app, Key::ArrowRight, NONE);
    }
    panic!(
        "no `{control}` in the top bar: the focus is {:?}",
        labels(app)
    );
}

/// Opens Windows › `entry` by the keys: Enter on Windows, ↓ to the entry, Enter.
fn open_from_windows_menu(h: &mut Headless, app: &mut App<MockSide>, entry: &str) {
    to_top_bar_control(h, app, "Windows");
    press(h, app, Key::Enter, NONE);
    for _ in 0..10 {
        if labels(app).last().is_some_and(|l| same(l, entry)) {
            break;
        }
        press(h, app, Key::ArrowDown, NONE);
    }
    check_path(
        &labels(app),
        &["top bar", "Windows", entry],
        "Windows › entry",
    );
    press(h, app, Key::Enter, NONE);
    settle(h, app);
}

// --- REQ-GUI-178 / REQ-GUI-095: the footer is big scope 8 ------------------------------------------------------------

#[test]
fn qa_scope_tab_order_mock_footer_after_legend_then_the_top_bar() {
    let (mut app, mut h) = fresh();
    let mut seen = Vec::new();
    for _ in 0..9 {
        press(&mut h, &mut app, Key::Tab, NONE);
        seen.push(labels(&app));
    }
    for (i, path) in seen.iter().enumerate() {
        check_path(path, &[BIG[i % 8]], &format!("Tab {}", i + 1));
    }
    // Control: a tree without the footer scope, Tab from Legend wrapping to the top bar.
    rejects("Tab from Legend wrapping to the top bar", || {
        check_path(&seen[7], &["top bar"], "Tab from Legend")
    });
    // The first Shift+Tab from no focus goes to the footer; the next to Legend.
    let (mut app, mut h) = fresh();
    press(&mut h, &mut app, Key::Tab, SHIFT);
    let first = labels(&app);
    check_path(&first, &["Footer"], "Shift+Tab from no focus");
    rejects("Shift+Tab from no focus starting at Legend", || {
        check_path(&first, &["Legend"], "Shift+Tab")
    });
    press(&mut h, &mut app, Key::Tab, SHIFT);
    check_path(&labels(&app), &["Legend"], "Shift+Tab from the footer");
    // The footer's ring has the footer's rect to be drawn on: inside the bar at the bottom.
    press(&mut h, &mut app, Key::Tab, NONE);
    let _ = h.frame(&mut app, Vec::new());
    let place = app
        .keyboard
        .place_of(app.view.focus.path.last().unwrap())
        .expect("the footer placed");
    let footer = layout().footer;
    assert!(
        footer.expand(0.5).contains_rect(place.rect),
        "the footer scope's rect {:?} is not the footer's {footer:?}",
        place.rect
    );
}

// --- REQ-GUI-178: Enter on the footer opens the console, the focus on its header bar (R-450) --------------------------

#[test]
fn qa_mock_keyboard_enter_on_the_footer_opens_the_console_on_its_header_bar() {
    let (mut app, mut h) = fresh();
    assert!(!app.is_open(Window::Console), "the console opens closed");
    console_by_footer(&mut h, &mut app);
    assert!(app.is_open(Window::Console), "Enter on the footer");
    let path = labels(&app);
    check_path(&path, &["Console", "Header"], "Enter on the footer");
    check_console_at(&app, 0, Some("Header"), NAV, "Enter on the footer");
    rejects("a console that does not take the focus on Enter", || {
        check_path(&path, &["Footer"], "Enter on the footer")
    });
    rejects("a console landing on its first filter", || {
        check_path(&path, &["Console", "Header", "All"], "Enter on the footer")
    });
    // The breadcrumb names the console, drawn in the top bar.
    let n = names(&mut h, &mut app);
    let crumb = path.join(SEP);
    let top = names_in(&n, layout().top_bar);
    assert!(
        texts(&top).iter().any(|t| t.contains(&crumb)),
        "no breadcrumb `{crumb}` in the top bar"
    );
    // The ring has the header bar's rect to be drawn on, inside the console.
    let place = app
        .keyboard
        .place_of(app.view.focus.path.last().unwrap())
        .expect("the header bar placed");
    assert!(window_rect(&app, Window::Console).contains_rect(place.rect));
}

#[test]
fn qa_mock_keyboard_a_footer_click_opens_the_console_with_the_focus_and_esc_returns_it() {
    let (mut app, mut h) = fresh();
    click(
        &mut h,
        &mut app,
        gui::capture::footer_point(layout().footer),
    );
    settle(&mut h, &mut app);
    assert!(app.is_open(Window::Console), "a footer click");
    check_console_at(&app, 0, Some("Header"), NAV, "a footer click");
    // Esc in navigation mode, up to the console's top level and out: the console closes, the focus on the footer.
    let mut closed_after = None;
    for i in 1..=3 {
        press(&mut h, &mut app, Key::Escape, NONE);
        if !app.is_open(Window::Console) {
            closed_after = Some(i);
            break;
        }
        assert!(
            in_console(&labels(&app)),
            "Esc {i} left the console open without its focus"
        );
    }
    // The landing is the console's top level: the first Esc closes it (R-450 A2).
    assert_eq!(closed_after, Some(1), "Esc on the header bar scope");
    let path = labels(&app);
    check_path(
        &path,
        &["Footer"],
        "Esc out of the console opened by a click",
    );
    assert_eq!(app.view.focus.mode, NAV);
}

#[test]
fn qa_mock_keyboard_enter_on_the_footer_takes_the_focus_into_a_console_an_error_opened() {
    let (mut app, mut h) = fresh();
    app.side().engine().raise_error();
    settle(&mut h, &mut app);
    assert!(app.is_open(Window::Console), "an error opens the console");
    console_by_footer(&mut h, &mut app);
    assert!(app.is_open(Window::Console), "Enter left the console open");
    check_console_at(
        &app,
        0,
        Some("Header"),
        NAV,
        "Enter on the footer, the console open",
    );
}

// --- REQ-GUI-178: Tab and Shift+Tab between the console's scopes, inside it (R-450) ------------------------------------

#[test]
fn qa_mock_keyboard_console_sections_in_tab_order_stay_inside() {
    let (mut app, mut h) = fresh();
    console_by_footer(&mut h, &mut app);
    // Tab: the log, then back to the header bar, each landing on the scope itself in navigation mode.
    let mut forward = Vec::new();
    for _ in 0..5 {
        press(&mut h, &mut app, Key::Tab, NONE);
        assert!(app.is_open(Window::Console));
        assert_eq!(app.view.focus.mode, NAV, "a Tab lands in navigation mode");
        assert_eq!(labels(&app).len(), 2, "a Tab lands on the scope itself");
        forward.push(section_of(&labels(&app)));
    }
    let want: Vec<Option<usize>> = [1, 0, 1, 0, 1].into_iter().map(Some).collect();
    assert_eq!(forward, want, "Tab through the console's scopes");
    rejects("a Tab that leaves the console after the log", || {
        assert_eq!(forward[..1].to_vec(), want[..1].to_vec());
        assert_eq!(forward[1], None);
    });
    // Shift+Tab: back the other way, wrapping from the header bar to the log.
    let mut back = Vec::new();
    for _ in 0..5 {
        press(&mut h, &mut app, Key::Tab, SHIFT);
        assert_eq!(
            labels(&app).len(),
            2,
            "a Shift+Tab lands on the scope itself"
        );
        back.push(section_of(&labels(&app)));
    }
    let want_back: Vec<Option<usize>> = [0, 1, 0, 1, 0].into_iter().map(Some).collect();
    assert_eq!(back, want_back, "Shift+Tab through the console's scopes");
    rejects("a Shift+Tab that stays on the log", || {
        assert_eq!(back, vec![Some(1); 5])
    });
}

// --- REQ-GUI-178 / REQ-GUI-126: the arrows within a section; Enter on a filter, copy and clear as their clicks ---------

#[test]
fn qa_mock_keyboard_console_filters_copy_and_clear_by_the_keys() {
    let (mut app, mut h) = fresh();
    app.side().engine().raise_warning();
    let _ = h.frame(&mut app, Vec::new());
    app.side().engine().raise_error();
    settle(&mut h, &mut app);
    console_by_footer(&mut h, &mut app);
    // Enter on the header bar goes in, onto All (R-450).
    press(&mut h, &mut app, Key::Enter, NONE);
    check_console_at(&app, 0, Some("all"), NAV, "Enter on the header bar");
    // The arrows walk the filters in §G12's order: all, warnings, errors, info.
    let mut walked = vec![labels(&app).last().cloned().unwrap()];
    for _ in 0..3 {
        press(&mut h, &mut app, Key::ArrowRight, NONE);
        walked.push(labels(&app).last().cloned().unwrap());
    }
    check_path(
        &walked,
        &["all", "warnings", "errors", "info"],
        "→ through the filters",
    );
    check_console_at(&app, 0, None, NAV, "the arrows stay in navigation mode");
    // Enter on errors: only the error rows are drawn.
    press(&mut h, &mut app, Key::ArrowLeft, NONE);
    check_console_at(&app, 0, Some("errors"), NAV, "← to errors");
    let before = texts(&names_in(
        &names(&mut h, &mut app),
        window_rect(&app, Window::Console),
    ));
    assert!(before.iter().any(|t| t == MOCK_WARNING) && before.iter().any(|t| t == MOCK_ERROR));
    press(&mut h, &mut app, Key::Enter, NONE);
    check_console_at(
        &app,
        0,
        Some("errors"),
        NAV,
        "Enter on a filter is its click",
    );
    let shown = texts(&names_in(
        &names(&mut h, &mut app),
        window_rect(&app, Window::Console),
    ));
    assert!(
        shown.iter().any(|t| t == MOCK_ERROR),
        "the error row hidden by `errors`"
    );
    assert!(
        !shown.iter().any(|t| t == MOCK_WARNING),
        "a warning row shown under `errors`"
    );
    rejects("the rows before the filter", || {
        assert!(!before.iter().any(|t| t == MOCK_WARNING))
    });
    // Enter on all brings them back.
    presses(&mut h, &mut app, Key::ArrowLeft, NONE, 2);
    press(&mut h, &mut app, Key::Enter, NONE);
    let shown = texts(&names_in(
        &names(&mut h, &mut app),
        window_rect(&app, Window::Console),
    ));
    assert!(
        shown.iter().any(|t| t == MOCK_WARNING),
        "`all` hides the warning"
    );
    // Copy: the rows, to the clipboard, the error's message among them. → from all, past the search field (R-450).
    presses(&mut h, &mut app, Key::ArrowRight, NONE, 5);
    check_console_at(&app, 0, Some("copy"), NAV, "→ to copy");
    let mut copied = Vec::new();
    for events in [
        vec![key_event(Key::Enter, true, NONE)],
        vec![key_event(Key::Enter, false, NONE)],
        Vec::new(),
    ] {
        let out = h.frame(&mut app, events);
        for c in out.platform_output.commands {
            if let OutputCommand::CopyText(t) = c {
                copied.push(t);
            }
        }
    }
    assert!(
        copied
            .iter()
            .any(|t| t.contains(MOCK_ERROR) && t.contains(MOCK_WARNING)),
        "copy put {copied:?} on the clipboard"
    );
    // Clear: the console empties, the footer's counts reset, the console stays open with the focus on clear.
    press(&mut h, &mut app, Key::ArrowRight, NONE);
    check_console_at(&app, 0, Some("clear"), NAV, "→ to clear");
    let counts = app.counts();
    assert!(counts.warnings > 0 && counts.errors > 0 && !app.console().is_empty());
    press(&mut h, &mut app, Key::Enter, NONE);
    assert!(
        app.console().is_empty(),
        "clear left {} rows",
        app.console().len()
    );
    let counts = app.counts();
    assert_eq!(
        (counts.warnings, counts.errors),
        (0, 0),
        "clear resets the footer's counts"
    );
    assert!(app.is_open(Window::Console));
    check_console_at(&app, 0, Some("clear"), NAV, "the focus kept on clear");
    let shown = texts(&names_in(
        &names(&mut h, &mut app),
        window_rect(&app, Window::Console),
    ));
    assert!(
        !shown.iter().any(|t| t == MOCK_ERROR || t == MOCK_WARNING),
        "rows drawn after clear"
    );
}

// --- REQ-GUI-178: the text filter (R-409 G4, G5, G6) -------------------------------------------------------------------

/// The console opened by the footer, the focus on its search field: Enter into the header bar, → past the four
/// filters (R-450).
fn on_text_filter() -> (App<MockSide>, Headless) {
    let (mut app, mut h) = fresh();
    console_by_footer(&mut h, &mut app);
    press(&mut h, &mut app, Key::Enter, NONE);
    presses(&mut h, &mut app, Key::ArrowRight, NONE, 4);
    check_console_at(&app, 0, Some("search"), NAV, "→ to the search field");
    (app, h)
}

#[test]
fn qa_mock_keyboard_console_text_filter_types_only_after_enter() {
    let (mut app, mut h) = on_text_filter();
    let field = app.view.focus.path.clone();
    // Navigating, a letter types nothing.
    type_text(&mut h, &mut app, "z");
    assert_eq!(app.windows.console.text, "", "a letter typed on landing");
    // Enter: typing, in interaction mode; letters, Space and the arrows go to the field.
    press(&mut h, &mut app, Key::Enter, NONE);
    check_console_at(
        &app,
        0,
        Some("search"),
        INTERACT,
        "Enter on the text filter",
    );
    type_text(&mut h, &mut app, "ab");
    press(&mut h, &mut app, Key::ArrowLeft, NONE);
    type_text(&mut h, &mut app, "x ");
    // ← moved the field's cursor, not the focus: x and Space land between a and b.
    assert_eq!(
        app.windows.console.text, "ax b",
        "the keys typed into the field"
    );
    assert_eq!(
        app.view.focus.path, field,
        "the arrows moved the focus while typing"
    );
    press(&mut h, &mut app, Key::ArrowDown, NONE);
    assert_eq!(app.view.focus.path, field, "↓ moved the focus while typing");
    check_console_at(&app, 0, Some("search"), INTERACT, "still typing");
    // Esc: back to navigation mode on the field, the text kept, the console open.
    press(&mut h, &mut app, Key::Escape, NONE);
    assert!(
        app.is_open(Window::Console),
        "Esc while typing closed the console"
    );
    assert_eq!(app.view.focus.path, field);
    check_console_at(&app, 0, Some("search"), NAV, "Esc while typing");
    assert_eq!(
        app.windows.console.text, "ax b",
        "Esc while typing changed the text"
    );
    // Navigating again, a letter types nothing.
    type_text(&mut h, &mut app, "q");
    assert_eq!(app.windows.console.text, "ax b", "a letter typed after Esc");
    // Enter, then Enter: back to navigation mode, the text kept.
    press(&mut h, &mut app, Key::Enter, NONE);
    check_console_at(&app, 0, Some("search"), INTERACT, "Enter again");
    press(&mut h, &mut app, Key::Enter, NONE);
    check_console_at(&app, 0, Some("search"), NAV, "Enter while typing");
    assert_eq!(
        app.windows.console.text, "ax b",
        "Enter while typing changed the text"
    );
    // Tab while typing: the next scope, the log, navigating, in the console; Shift+Tab likewise backwards, wrapping
    // from the header bar to the log (R-450).
    press(&mut h, &mut app, Key::Enter, NONE);
    press(&mut h, &mut app, Key::Tab, NONE);
    check_console_at(&app, 1, Some("log"), NAV, "Tab while typing");
    press(&mut h, &mut app, Key::Tab, SHIFT);
    check_console_at(&app, 0, Some("header"), NAV, "Shift+Tab to the header bar");
    press(&mut h, &mut app, Key::Enter, NONE);
    presses(&mut h, &mut app, Key::ArrowRight, NONE, 4);
    press(&mut h, &mut app, Key::Enter, NONE);
    check_console_at(&app, 0, Some("search"), INTERACT, "typing again");
    press(&mut h, &mut app, Key::Tab, SHIFT);
    check_console_at(&app, 1, Some("log"), NAV, "Shift+Tab while typing");
    assert_eq!(app.windows.console.text, "ax b");
    // The typed filter filters: a text no row holds hides every row.
    let rows = texts(&names_in(
        &names(&mut h, &mut app),
        window_rect(&app, Window::Console),
    ));
    assert!(
        !rows.iter().any(|t| t.contains("SetField")),
        "a row matched `ax b`"
    );
}

#[test]
fn qa_mock_keyboard_console_question_mark_and_ctrl_z_while_typing_are_the_fields() {
    let (mut app, mut h) = on_text_filter();
    // A contract edit, so the contract's undo has something to undo.
    let t0 = app.snapshot().render.playhead.t;
    app.set_field(playhead_edit(t0 + 1.0));
    settle(&mut h, &mut app);
    let depth = app.snapshot().history.undo_depth;
    let t1 = app.snapshot().render.playhead.t;
    assert!(depth > 0 && t1 != t0);
    press(&mut h, &mut app, Key::Enter, NONE);
    type_text(&mut h, &mut app, "k?");
    assert_eq!(
        app.windows.console.text, "k?",
        "? while typing types a character"
    );
    assert!(
        !app.keyboard.shortcuts_open,
        "? while typing opened the overlay"
    );
    // Ctrl+Z while typing undoes the text edit, not the contract's history.
    let ctrl_z = Modifiers {
        ctrl: true,
        command: true,
        ..NONE
    };
    press(&mut h, &mut app, Key::Z, ctrl_z);
    settle(&mut h, &mut app);
    let text = app.windows.console.text.clone();
    assert!(
        text.len() < 2 && "k?".starts_with(&text),
        "Ctrl+Z while typing left `{text}`, not an earlier state of the edit"
    );
    assert_eq!(
        app.snapshot().history.undo_depth,
        depth,
        "Ctrl+Z reached the contract's undo"
    );
    assert_eq!(
        app.snapshot().render.playhead.t,
        t1,
        "Ctrl+Z undid the playhead edit"
    );
    check_console_at(
        &app,
        0,
        Some("search"),
        INTERACT,
        "still typing after Ctrl+Z",
    );
    // Control: the same `?` in navigation mode opens the overlay. (Ctrl+Z to the contract's undo is not on the mock
    // yet, REQ-GUI-096 being TASK-M8-13's, so the depth check above is shown able to move by the contract's own undo.)
    press(&mut h, &mut app, Key::Escape, NONE);
    check_console_at(&app, 0, Some("search"), NAV, "Esc");
    press(&mut h, &mut app, Key::Questionmark, SHIFT);
    assert!(
        app.keyboard.shortcuts_open,
        "? navigating opened no overlay"
    );
    press(&mut h, &mut app, Key::Escape, NONE);
    assert!(!app.keyboard.shortcuts_open);
    assert!(
        app.is_open(Window::Console),
        "Esc on the overlay closed the console"
    );
    let kept = app.windows.console.text.clone();
    press(&mut h, &mut app, Key::Z, ctrl_z);
    settle(&mut h, &mut app);
    assert_eq!(
        app.windows.console.text, kept,
        "Ctrl+Z navigating edited the text"
    );
    app.undo();
    settle(&mut h, &mut app);
    let undone = app.snapshot().history.undo_depth;
    rejects("a Ctrl+Z that reached the contract's undo", || {
        assert_eq!(undone, depth, "the depth after the contract's undo")
    });
}

// --- REQ-GUI-178: the entry list scrolls only in interaction mode (R-409 F2) -------------------------------------------

/// The y, in pixels, of the console's row that reads `message`, if drawn inside the console (the footer's latest
/// message reads the same, outside it).
fn row_y(h: &mut Headless, app: &mut App<MockSide>, message: &str) -> Option<f64> {
    let console = window_rect(app, Window::Console);
    names_in(&names(h, app), console)
        .iter()
        .filter(|n| n.name == message)
        .find_map(|n| n.rect)
        .map(|r| r[1])
}

#[test]
fn qa_mock_keyboard_console_entry_list_scrolls_only_after_enter() {
    let (mut app, mut h) = fresh();
    // Forty contract edits: forty info rows, more than the console shows at once.
    let t0 = app.snapshot().render.playhead.t;
    for i in 0..40 {
        app.set_field(playhead_edit(t0 + 0.25 * f64::from(i + 1)));
        let _ = h.frame(&mut app, Vec::new());
    }
    settle(&mut h, &mut app);
    let newest = app.console().last().expect("rows logged").message.clone();
    console_by_footer(&mut h, &mut app);
    // Shift+Tab from the header bar: the log, the last scope, navigating (R-450).
    press(&mut h, &mut app, Key::Tab, SHIFT);
    check_console_at(&app, 1, Some("log"), NAV, "Shift+Tab to the log");
    let list = app.view.focus.path.clone();
    let y0 = row_y(&mut h, &mut app, &newest).expect("the newest row drawn");
    presses(&mut h, &mut app, Key::ArrowDown, NONE, 3);
    let y_nav = row_y(&mut h, &mut app, &newest);
    assert_eq!(y_nav, Some(y0), "↓ scrolled the list in navigation mode");
    check_console_at(&app, 1, Some("log"), NAV, "↓ navigating");
    // Enter: interaction; ↓ scrolls towards the older rows, the newest moving up or out.
    press(&mut h, &mut app, Key::Enter, NONE);
    check_console_at(&app, 1, Some("log"), INTERACT, "Enter on the list");
    presses(&mut h, &mut app, Key::ArrowDown, NONE, 3);
    settle(&mut h, &mut app);
    let y_int = row_y(&mut h, &mut app, &newest);
    assert!(
        y_int.is_none_or(|y| y < y0 - 1.0),
        "↓ in interaction mode did not scroll the list: {y0} → {y_int:?}"
    );
    rejects("an entry list that scrolls in navigation mode", || {
        assert_eq!(y_int, Some(y0))
    });
    // ↑ scrolls back.
    presses(&mut h, &mut app, Key::ArrowUp, NONE, 3);
    settle(&mut h, &mut app);
    let y_back = row_y(&mut h, &mut app, &newest).expect("the newest row drawn again");
    assert!(
        (y_back - y0).abs() < 1.0,
        "↑ did not scroll back: {y0} → {y_back}"
    );
    // Esc: navigation mode on the list.
    press(&mut h, &mut app, Key::Escape, NONE);
    assert_eq!(app.view.focus.path, list);
    check_console_at(&app, 1, Some("log"), NAV, "Esc on the list");
    assert!(app.is_open(Window::Console));
    // Tab from interaction mode on the log: the console's next scope, the header bar, navigating, inside.
    press(&mut h, &mut app, Key::Enter, NONE);
    press(&mut h, &mut app, Key::Tab, NONE);
    assert!(app.is_open(Window::Console));
    let path = labels(&app);
    check_console_at(
        &app,
        0,
        Some("header"),
        NAV,
        "Tab from the list in interaction mode",
    );
    rejects("a Tab from the list that leaves the console", || {
        check_path(&path, &["top bar"], "Tab")
    });
}

// --- REQ-GUI-178: Esc returns the focus to whatever opened the console ------------------------------------------------

/// Esc, navigating, until the console closes (at most three); checks the focus stays in it until then.
fn esc_out(h: &mut Headless, app: &mut App<MockSide>) {
    for i in 1..=3 {
        assert_eq!(app.view.focus.mode, NAV);
        press(h, app, Key::Escape, NONE);
        if !app.is_open(Window::Console) {
            return;
        }
        assert!(
            in_console(&labels(app)),
            "Esc {i}: the focus left the open console"
        );
    }
    panic!("Esc did not close the console");
}

#[test]
fn qa_mock_keyboard_console_esc_returns_to_the_footer_or_windows_console() {
    // Opened by the footer: back on the footer.
    let (mut app, mut h) = fresh();
    console_by_footer(&mut h, &mut app);
    esc_out(&mut h, &mut app);
    let by_footer = labels(&app);
    check_path(
        &by_footer,
        &["Footer"],
        "Esc, the console opened by the footer",
    );
    // Opened from Windows › Console: the focus on its header bar, Tab between its scopes, then back on the entry (R-450).
    let (mut app, mut h) = fresh();
    open_from_windows_menu(&mut h, &mut app, "Console");
    assert!(app.is_open(Window::Console), "Windows › Console");
    check_console_at(&app, 0, Some("header"), NAV, "Windows › Console");
    press(&mut h, &mut app, Key::Tab, NONE);
    check_console_at(&app, 1, Some("log"), NAV, "Tab in the menu-opened console");
    press(&mut h, &mut app, Key::Tab, SHIFT);
    esc_out(&mut h, &mut app);
    let by_menu = labels(&app);
    check_path(
        &by_menu,
        &["top bar", "Windows", "Console"],
        "Esc, the console opened from the menu",
    );
    rejects(
        "an Esc that returns the menu-opened console's focus to the footer",
        || check_path(&by_menu, &["Footer"], "Esc"),
    );
    rejects("the footer-opened console's focus sent to the menu", || {
        check_path(&by_footer, &["top bar", "Windows", "Console"], "Esc")
    });
}

// --- REQ-GUI-178 / REQ-GUI-126: an error opens the console, taking no focus -----------------------------------------

#[test]
fn qa_mock_keyboard_a_raised_error_opens_the_console_without_moving_the_focus() {
    // The focus on the Figure.
    let (mut app, mut h) = fresh();
    presses(&mut h, &mut app, Key::Tab, NONE, 3);
    check_path(&labels(&app), &["Figure"], "three Tabs");
    let before = app.view.focus.clone();
    assert!(!app.is_open(Window::Console));
    app.side().engine().raise_error();
    settle(&mut h, &mut app);
    assert!(
        app.is_open(Window::Console),
        "a raised error did not open the console"
    );
    assert_eq!(app.view.focus, before, "the error moved the focus");
    // A key pressed next acts where the focus was: Tab goes on to Trajectory, not into the console.
    press(&mut h, &mut app, Key::Tab, NONE);
    check_path(&labels(&app), &["Trajectory"], "Tab after the error");
    // The focus inside another window: it stays there.
    let (mut app, mut h) = fresh();
    open_from_windows_menu(&mut h, &mut app, "Run");
    assert!(app.is_open(Window::Run));
    let in_run = app.view.focus.clone();
    assert!(labels(&app)
        .first()
        .is_some_and(|w| lower(w).starts_with("run")));
    app.side().engine().raise_error();
    settle(&mut h, &mut app);
    assert!(app.is_open(Window::Console));
    assert_eq!(
        app.view.focus, in_run,
        "the error took the focus from the Run window"
    );
    let got = labels(&app);
    rejects("a console that takes the focus on an error", || {
        assert!(in_console(&got) || got.is_empty())
    });
    assert!(!in_console(&got));
}

// --- REQ-GUI-126: the console's columns, filters, copy and clear --------------------------------------------------------

/// Whether `s` reads as a time of day, `hh:mm:ss` with or without a fraction (12_console.png's "12:04:11.251").
fn is_time(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() >= 8
        && [0, 1, 3, 4, 6, 7].iter().all(|&i| b[i].is_ascii_digit())
        && b[2] == b':'
        && b[5] == b':'
}

#[test]
fn qa_console_is_the_footer_opened_with_its_columns_filters_copy_and_clear() {
    let (mut app, mut h) = fresh();
    app.side().engine().raise_warning();
    let _ = h.frame(&mut app, Vec::new());
    assert!(!app.is_open(Window::Console), "a warning is not an error");
    let t0 = app.snapshot().render.playhead.t;
    app.set_field(playhead_edit(t0 + 1.0));
    let _ = h.frame(&mut app, Vec::new());
    assert!(!app.is_open(Window::Console));
    app.side().engine().raise_error();
    settle(&mut h, &mut app);
    // Errors open it automatically.
    assert!(
        app.is_open(Window::Console),
        "a raised error did not open the console"
    );
    let n = names(&mut h, &mut app);
    let console = window_rect(&app, Window::Console);
    // §G12: the footer, opened; the console sits over the bottom row, never over the figure.
    assert!(
        !console.intersects(layout().figure.shrink(1.0)),
        "the console covers the figure"
    );
    let drawn = texts(&names_in(&n, console));
    let has = |want: &str| drawn.iter().any(|t| same(t, want));
    let has_part = |want: &str| drawn.iter().any(|t| lower(t).contains(&lower(want)));
    // Severity, time, source and message, for each row.
    for sev in ["warn", "error", "info"] {
        assert!(has(sev), "no severity `{sev}` in {drawn:?}");
    }
    assert!(
        drawn.iter().any(|t| is_time(t)),
        "no time column in {drawn:?}"
    );
    for source in ["stain", "integrator", "contract"] {
        assert!(has(source), "no source `{source}` in {drawn:?}");
    }
    assert!(
        has(MOCK_WARNING) && has(MOCK_ERROR),
        "the messages in {drawn:?}"
    );
    assert!(
        has_part("SetField"),
        "the contract's SetField row in {drawn:?}"
    );
    // The filters, copy and clear.
    for control in ["all", "warnings", "errors", "info", "copy", "clear"] {
        assert!(has_part(control), "no `{control}` control in {drawn:?}");
    }
    // Controls: the time check rejects a message; the scan finds nothing outside the console's rect.
    assert!(!is_time(MOCK_ERROR) && is_time("12:04:11.251"));
    let figure = texts(&names_in(&n, layout().figure));
    rejects("the console's rows read inside the figure", || {
        assert!(figure.iter().any(|t| t == MOCK_ERROR))
    });
}

#[test]
fn qa_console_filters_and_clear_by_click() {
    let (mut app, mut h) = fresh();
    app.side().engine().raise_warning();
    let _ = h.frame(&mut app, Vec::new());
    app.side().engine().raise_error();
    settle(&mut h, &mut app);
    let console = window_rect(&app, Window::Console);
    let n = names_in(&names(&mut h, &mut app), console);
    // The warnings filter: its button reads "warnings" with the count.
    let warnings = n
        .iter()
        .find(|x| lower(&x.name).contains("warnings"))
        .map(|x| x.name.clone())
        .expect("a warnings filter");
    click(&mut h, &mut app, centre_of(&n, &warnings).unwrap());
    let shown = texts(&names_in(&names(&mut h, &mut app), console));
    assert!(
        shown.iter().any(|t| t == MOCK_WARNING),
        "`warnings` hid the warning"
    );
    assert!(
        !shown.iter().any(|t| t == MOCK_ERROR),
        "`warnings` showed the error"
    );
    // Clear by click: empties it and resets the footer's counts.
    let n = names_in(&names(&mut h, &mut app), console);
    click(
        &mut h,
        &mut app,
        centre_of(&n, "clear").expect("a clear button"),
    );
    assert!(app.console().is_empty());
    assert_eq!((app.counts().warnings, app.counts().errors), (0, 0));
}

// --- REQ-GUI-174: each window opens from its top-bar or menu entry ---------------------------------------------------

/// Opens Windows › `name` by the mouse: a click on Windows, then on the entry.
fn click_windows_entry(h: &mut Headless, app: &mut App<MockSide>, name: &str) {
    let top = Layout::new(h.screen(), PIXELS_PER_POINT).top_bar;
    let n = names_in(&names(h, app), top);
    click(h, app, centre_of(&n, "Windows").expect("a Windows menu"));
    let n = names(h, app);
    let at = centre_of(&n, name).unwrap_or_else(|| panic!("no `{name}` in the Windows menu"));
    click(h, app, at);
    settle(h, app);
}

#[test]
fn qa_mock_windows_open_each_window_from_the_windows_menu() {
    // The menu lists the seven by their spec names.
    let (mut app, mut h) = fresh();
    let top = layout().top_bar;
    let n = names_in(&names(&mut h, &mut app), top);
    click(
        &mut h,
        &mut app,
        centre_of(&n, "Windows").expect("a Windows menu"),
    );
    let listed = texts(&names(&mut h, &mut app));
    for (_, name) in WINDOWS {
        assert!(
            listed.iter().any(|t| t == name),
            "Windows lists no `{name}`: {listed:?}"
        );
    }
    let figure = layout().figure;
    for (window, name) in WINDOWS {
        let (mut app, mut h) = fresh();
        assert!(!app.is_open(window), "{name} opens closed");
        click_windows_entry(&mut h, &mut app, name);
        assert!(app.is_open(window), "Windows › {name} did not open it");
        // Open, it takes the focus (the window rule), and it never covers the figure (§G1).
        let path = labels(&app);
        assert_eq!(
            app.view.focus.path.first().map(String::as_str),
            Some(gui::windows::entry(window).scope),
            "Windows › {name}: the focus is {path:?}"
        );
        assert_eq!(app.view.focus.mode, NAV);
        let rect = window_rect(&app, window);
        assert!(
            !rect.intersects(figure.shrink(1.0)),
            "{name} at {rect:?} covers the figure {figure:?}"
        );
        // Every other window stays closed.
        for (other, other_name) in WINDOWS {
            if other != window {
                assert!(!app.is_open(other), "Windows › {name} opened {other_name}");
            }
        }
    }
}

#[test]
fn qa_mock_windows_open_from_the_top_bar_the_manifold_view_and_the_trajectory_panel() {
    // Run…, Profiler… and Export… in the top bar.
    for (button, window) in [
        ("Run…", Window::Run),
        ("Profiler…", Window::Profiler),
        ("Export…", Window::Export),
    ] {
        let (mut app, mut h) = fresh();
        let n = names_in(&names(&mut h, &mut app), layout().top_bar);
        click(
            &mut h,
            &mut app,
            centre_of(&n, button).unwrap_or_else(|| panic!("no `{button}`")),
        );
        settle(&mut h, &mut app);
        assert!(app.is_open(window), "{button} did not open {window:?}");
        // Esc out of it returns the focus to its opener.
        for _ in 0..4 {
            if !app.is_open(window) {
                break;
            }
            press(&mut h, &mut app, Key::Escape, NONE);
        }
        assert!(!app.is_open(window), "Esc did not close {window:?}");
        check_path(
            &labels(&app),
            &["top bar", button],
            "Esc back to the opener",
        );
    }
    // Chart builder… in the Manifold view's Chart.
    let (mut app, mut h) = fresh();
    let n = names_in(&names(&mut h, &mut app), layout().manifold_view);
    click(
        &mut h,
        &mut app,
        centre_of(&n, "Chart builder…").expect("a Chart builder…"),
    );
    settle(&mut h, &mut app);
    assert!(
        app.is_open(Window::ChartBuilder),
        "Chart builder… did not open it"
    );
    // IC Inspector… in the Trajectory panel.
    let (mut app, mut h) = fresh();
    let n = names_in(&names(&mut h, &mut app), layout().trajectory);
    click(
        &mut h,
        &mut app,
        centre_of(&n, "IC Inspector…").expect("an IC Inspector…"),
    );
    settle(&mut h, &mut app);
    assert!(
        app.is_open(Window::Inspector),
        "IC Inspector… did not open it"
    );
    // Control: a click on nothing opens nothing.
    let (mut app, mut h) = fresh();
    click(&mut h, &mut app, layout().legend.center());
    settle(&mut h, &mut app);
    for (window, name) in WINDOWS {
        assert!(!app.is_open(window), "a click on the Legend opened {name}");
    }
}

// --- REQ-GUI-174: the Chart builder's two previews are square for a square viewport ----------------------------------

#[test]
fn qa_mock_windows_chart_builder_previews_are_square() {
    // A square viewport: the capture's width, at the height that makes the figure square.
    let ppp = PIXELS_PER_POINT;
    let figure_of = |height: u32| {
        let screen = egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(SIZE[0] as f32 / ppp, height as f32 / ppp),
        );
        Layout::new(screen, ppp).figure
    };
    let height = (600..3000)
        .min_by(|a, b| {
            let d = |h: &u32| (figure_of(*h).width() - figure_of(*h).height()).abs();
            d(a).total_cmp(&d(b))
        })
        .unwrap();
    let mut h = Headless::new([SIZE[0], height], ppp);
    let figure = Layout::new(h.screen(), ppp).figure;
    assert!(
        (figure.width() - figure.height()).abs() < 1.0,
        "no square viewport found: {figure:?}"
    );
    let mut app = mock_app();
    let _ = h.frame(&mut app, Vec::new());
    click_windows_entry(&mut h, &mut app, "Chart builder");
    let _ = h.frame(&mut app, Vec::new());
    let previews = app.windows.chart.previews.expect("the previews drawn");
    let square = |r: egui::Rect| r.width() > 16.0 && (r.width() - r.height()).abs() <= 1.0;
    for (i, r) in previews.iter().enumerate() {
        assert!(square(*r), "preview {i} is {}×{}", r.width(), r.height());
    }
    // The two are side by side, distinct.
    assert!(
        !previews[0].intersects(previews[1].shrink(1.0)),
        "the previews overlap"
    );
    // Control: a 4:3 preview fails the check.
    let wide = egui::Rect::from_min_size(
        previews[0].min,
        egui::vec2(previews[0].width(), previews[0].width() * 0.75),
    );
    assert!(!square(wide));
}

// --- REQ-GUI-174: the windows' content, as §G5, §G8 and §G13 give it ------------------------------------------------

/// The texts drawn in `window`, opened from the Windows menu by its `name`.
/// The texts drawn once `window` is open that were not drawn before it opened: its own, wherever it scrolls them.
fn window_texts(window: Window, name: &str) -> Vec<String> {
    let (mut app, mut h) = fresh();
    let before = texts(&names(&mut h, &mut app));
    click_windows_entry(&mut h, &mut app, name);
    assert!(app.is_open(window));
    texts(&names(&mut h, &mut app))
        .into_iter()
        .filter(|t| !t.is_empty() && !before.contains(t))
        .collect()
}

fn check_has(drawn: &[String], want: &str, what: &str) {
    assert!(
        drawn.iter().any(|t| t.contains(want)),
        "{what}: no `{want}` drawn in {drawn:?}"
    );
}

fn check_lacks(drawn: &[String], word: &str, what: &str) {
    assert!(
        !drawn.iter().any(|t| lower(t).contains(word)),
        "{what}: `{word}` drawn: {drawn:?}"
    );
}

#[test]
fn qa_mock_windows_run_window_under_the_contract_names() {
    let drawn = window_texts(Window::Run, "Run");
    // §G5 Run: integrator_contract Part 3's parameters under their contract names (R-68), the escape criterion, the
    // quality selector, budget, Recompute and Cancel.
    for name in [
        "T_horizon",
        "dt_macro",
        "N_max",
        "r_sub",
        "gamma_sub",
        "r_coll",
        "r_close",
        "eps_E",
        "eps_L",
        "tau",
        "Recompute",
        "Cancel",
    ] {
        check_has(&drawn, name, "Run");
    }
    check_has(
        &drawn,
        "shape closure + energy sign",
        "Run's escape criterion",
    );
    // §G13: no "tolerance" field, no persistence count.
    check_lacks(&drawn, "tolerance", "Run");
    check_lacks(&drawn, "persistence", "Run");
    // Control: the scan reads the window, and would find a word that is there.
    rejects("a Run window scanned for a word it draws", || {
        check_lacks(&drawn, "t_horizon", "Run")
    });
}

#[test]
fn qa_mock_windows_display_window_fixed_order_and_legend_never_fate() {
    let drawn = window_texts(Window::Display, "Display");
    // §G5 Display: the colour-vision simulations and the overlays.
    for name in [
        "deuteranopia",
        "protanopia",
        "tritanopia",
        "achromatopsia",
        "plain",
    ] {
        check_has(&drawn, name, "Display");
    }
    // §G13: "Legend", never "Fate".
    check_lacks(&drawn, "fate", "Display");
    // The fixed order of the last stages (R-67, R-111), drawn in the window along one row, left to right.
    let order = [
        "SimState",
        "stain",
        "style",
        "display scale",
        "gamut clamp",
        "colour-vision simulation",
        "screen",
    ];
    let (mut app, mut h) = fresh();
    click_windows_entry(&mut h, &mut app, "Display");
    let inside = names_in(&names(&mut h, &mut app), window_rect(&app, Window::Display));
    let at: Vec<[f64; 4]> = order
        .iter()
        .map(|stage| {
            inside
                .iter()
                .filter(|n| same(&n.name, stage))
                .find_map(|n| n.rect)
                .unwrap_or_else(|| panic!("Display: no `{stage}` stage drawn"))
        })
        .collect();
    // Reading order, the row wrapping: B follows A when B starts on a later row, or on A's top row to its right; a
    // label wrapped over two rows starts at the end of its top row, so it is to the right of A when it ends there.
    let in_order = |at: &[[f64; 4]]| {
        at.windows(2).all(|w| {
            let (a, b) = (w[0], w[1]);
            let same_top = (a[1] - b[1]).abs() < 2.0;
            b[1] > a[1] + 2.0 || (same_top && (a[2] <= b[0] + 0.5 || a[2] <= b[2] - 0.5))
        })
    };
    assert!(in_order(&at), "Display: the stages out of order: {at:?}");
    let mut swapped = at.clone();
    swapped.swap(2, 3);
    rejects("style and the display scale swapped", || {
        assert!(in_order(&swapped))
    });
}

#[test]
fn qa_mock_windows_profiler_tabs_and_trace_in_schema_v1() {
    let drawn = window_texts(Window::Profiler, "Profiler");
    // §G5 Profiler: the six tabs, the Arbiter among them (R-152); live / pause / Capture; passive logging.
    for tab in ["Timeline", "Flame", "GPU", "Memory", "Counters", "Arbiter"] {
        check_has(&drawn, tab, "Profiler's tabs");
    }
    for control in ["live", "pause", "Capture"] {
        assert!(
            drawn.iter().any(|t| lower(t).contains(&lower(control))),
            "Profiler: no `{control}` in {drawn:?}"
        );
    }
    // §G13: the top level is telemetry §2's five stages; the artboard's donut categories are not.
    for stage in ["integrate", "reduce", "colour", "upload", "present"] {
        assert!(
            drawn.iter().any(|t| lower(t).contains(stage)),
            "Profiler: stage `{stage}` not drawn: {drawn:?}"
        );
    }
    // The mock's trace is in profiler schema v1's shape (R-56): it writes and reads back as itself.
    let side = MockSide::new(MockEngine::frozen(), None);
    let trace = side.trace().expect("the mock has a trace");
    assert!(!trace.frames.is_empty(), "the mock's trace has no frames");
    let mut json = Vec::new();
    profile::write(trace, &mut json).expect("the trace writes as schema v1");
    let back = profile::read(json.as_slice()).expect("the written trace reads back as schema v1");
    assert_eq!(back.frames, trace.frames, "the trace's frames round trip");
    for f in &trace.frames {
        assert!(
            f.frame_ms.is_finite() && f.frame_ms > 0.0,
            "frame {} ms {}",
            f.frame,
            f.frame_ms
        );
    }
    // Control: a frame record with a sixth stage beside the five does not read as schema v1.
    let text = String::from_utf8(json).unwrap();
    assert!(text.contains("\"integrate\""));
    let sixth = text.replacen("\"integrate\"", "\"quadtree\"", 1);
    assert!(
        profile::read(sixth.as_bytes()).is_err(),
        "a stage renamed out of the five read as schema v1"
    );
}

#[test]
fn qa_mock_windows_export_and_share_image_state_and_present() {
    let drawn = window_texts(Window::Export, "Export & share");
    // §G5 Export & share: the image with pxpack and the stain's WGSL; copy, save and load the snapshot; the share
    // link; present.
    for want in ["pxpack", "WGSL", "PNG", "principia://view?"] {
        check_has(&drawn, want, "Export & share");
    }
    for want in ["copy snapshot", "save snapshot", "load", "present"] {
        assert!(
            drawn.iter().any(|t| lower(t).contains(want)),
            "Export & share: no `{want}` in {drawn:?}"
        );
    }
}

#[test]
fn qa_mock_windows_present_mode_hides_the_chrome_until_esc() {
    let (mut app, mut h) = fresh();
    click_windows_entry(&mut h, &mut app, "Export & share");
    let rect = window_rect(&app, Window::Export);
    let n = names_in(&names(&mut h, &mut app), rect);
    let present = n
        .iter()
        .find(|x| lower(&x.name).starts_with("present"))
        .map(|x| x.name.clone())
        .expect("a Present button");
    let chrome = texts(&names_in(&names(&mut h, &mut app), layout().top_bar));
    assert!(!chrome.is_empty(), "the top bar draws nothing");
    click(&mut h, &mut app, centre_of(&n, &present).unwrap());
    settle(&mut h, &mut app);
    // Present: all chrome hidden, the top bar, the footer and the window among it.
    let all = names(&mut h, &mut app);
    let shown = texts(&all);
    for gone in ["Windows", "Footer", "Export & share"] {
        assert!(
            !shown.iter().any(|t| t == gone),
            "`{gone}` drawn in present mode"
        );
    }
    assert!(
        names_in(&all, layout().top_bar).is_empty(),
        "the top bar drawn in present mode: {:?}",
        texts(&names_in(&all, layout().top_bar))
    );
    // Esc returns.
    press(&mut h, &mut app, Key::Escape, NONE);
    settle(&mut h, &mut app);
    let back = texts(&names_in(&names(&mut h, &mut app), layout().top_bar));
    assert!(
        back.iter().any(|t| t == "Windows"),
        "Esc did not return the chrome: {back:?}"
    );
}

#[test]
fn qa_mock_windows_inspector_frame_tabs_panes_and_one_timeline() {
    let drawn = window_texts(Window::Inspector, "Inspector");
    // §G8: Inspect and Create & locate; the gauge buttons; listen.
    for want in [
        "Inspect",
        "Create & locate",
        "Reset",
        "Randomise",
        "Burrau",
        "Rotate 15°",
        "Scale × 1.25",
        "Boost",
    ] {
        check_has(&drawn, want, "Inspector");
    }
    // Opened from the Trajectory panel with an orbit shown: its IC, the one timeline and listen.
    let (mut app, mut h) = fresh();
    let figure = layout().figure;
    let at = gui::capture::hover_point(figure);
    for _ in 0..4 {
        let _ = h.frame(&mut app, vec![Event::PointerMoved(at)]);
    }
    // The pointer stays on the figure, so the Trajectory panel shows the orbit under it; the keys reach IC Inspector….
    let shown = texts(&names_in(&names(&mut h, &mut app), layout().trajectory));
    assert!(
        !shown
            .iter()
            .any(|t| t.contains("move the pointer over the figure")),
        "no orbit under the pointer: {shown:?}"
    );
    presses(&mut h, &mut app, Key::Tab, NONE, 4);
    check_path(&labels(&app), &["Trajectory"], "four Tabs");
    press(&mut h, &mut app, Key::Enter, NONE);
    for _ in 0..20 {
        if labels(&app)
            .last()
            .is_some_and(|l| same(l, "IC Inspector…"))
        {
            break;
        }
        press(&mut h, &mut app, Key::ArrowDown, NONE);
    }
    check_path(
        &labels(&app),
        &["Trajectory", "IC Inspector…"],
        "↓ to IC Inspector…",
    );
    press(&mut h, &mut app, Key::Enter, NONE);
    settle(&mut h, &mut app);
    assert!(
        app.is_open(Window::Inspector),
        "IC Inspector… with an orbit shown"
    );
    let inside = texts(&names_in(
        &names(&mut h, &mut app),
        window_rect(&app, Window::Inspector),
    ));
    let lowered: Vec<String> = inside.iter().map(|t| lower(t)).collect();
    // (The panes' captions are painted, not named: their look is the screenshot's, 05_inspectors/mock_inspector.)
    for want in ["listen", "one timeline", "latent z"] {
        assert!(
            lowered.iter().any(|t| t.contains(want)),
            "Inspector with an orbit: no `{want}` in {inside:?}"
        );
    }
    assert!(
        !inside.iter().any(|t| t.starts_with("no IC")),
        "the Inspector opened on an orbit shows no IC: {inside:?}"
    );
}
