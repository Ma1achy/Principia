//! QA tests for TASK-M6-32 under R-456 (17 Oct 2026, amending R-450 A3 and R-409's landing rule and G4 for text
//! inputs), written from the ruling and REQ-GUI-189's and REQ-GUI-190's verify, not from the implementation:
//!
//! - A click on a text input starts typing at once: the keyboard's focus on the field, interaction mode (the ring
//!   blue), a letter typed next landing in the field; §G3's typing rules then hold (Esc keeps the text and returns to
//!   navigation mode on the field; Tab moves on to the window's next section, R-409 G4, G6; R-456 B1).
//! - B6: a click that is not on a text input ends any typing and leaves the keyboard in navigation mode; it moves the
//!   focus only where a rule says a click does (R-450 A3's console controls); a click on the Figure leaves the focus
//!   where it was.
//! - B7: REQ-GUI-189's click-out script (search, type, then a click on Copy, and in a second run on the log), and the
//!   Figure-click script with its two controls; REQ-GUI-190's click on a text field outside the console (the Run
//!   window's, the stain library's filter), with a control in which the field needs Enter (B2).
//!
//! Each check is a function of what the app did; its control runs the same observation on a world that does what the
//! ruling forbids (a field reached without a click, which needs Enter; typing with no click; the focus on the Figure)
//! and asserts the check rejects it (R-176, through [`rejects`]).
// The file name `qa_TASK-M6-32_r456` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use eframe::egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect};
use engine::contract::view_ui::{KeyMode, Mode, Window};
use gui::app::App;
use gui::capture::{PIXELS_PER_POINT, SIZE};
use gui::headless::Headless;
use gui::layout::Layout;
use gui::mock::MockEngine;
use gui::side::{EngineSide, MockSide};
use gui::windows::run;

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const NONE: Modifiers = Modifiers::NONE;
const SHIFT: Modifiers = Modifiers::SHIFT;
const NAV: KeyMode = KeyMode::Navigation;
const INTERACT: KeyMode = KeyMode::Interaction;

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

fn key_event(key: Key, pressed: bool, modifiers: Modifiers) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers,
    }
}

fn press(h: &mut Headless, app: &mut App<MockSide>, key: Key, modifiers: Modifiers) {
    let _ = h.frame(app, vec![key_event(key, true, modifiers)]);
    let _ = h.frame(app, vec![key_event(key, false, modifiers)]);
    let _ = h.frame(app, Vec::new());
}

fn settle(h: &mut Headless, app: &mut App<MockSide>) {
    for _ in 0..4 {
        let _ = h.frame(app, Vec::new());
    }
}

/// Types `text` as a keyboard does: each letter's press with its text, then the release. No `l` (the Figure's lock).
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

fn labels_of(app: &App<MockSide>, path: &[String]) -> Vec<String> {
    app.keyboard
        .tree(app.view.mode)
        .labels(path)
        .into_iter()
        .map(str::to_owned)
        .collect()
}

fn at(app: &App<MockSide>) -> At {
    (labels_of(app, &app.view.focus.path), app.view.focus.mode)
}

fn want(path: &[&str], mode: KeyMode) -> At {
    (path.iter().map(|s| (*s).to_owned()).collect(), mode)
}

/// The labels of scope `id`'s path.
fn labels_to(app: &App<MockSide>, id: &str) -> Vec<String> {
    let path = app.keyboard.tree(app.view.mode).path_to(id);
    labels_of(app, &path)
}

/// The rect scope `id` was placed on this frame.
fn place(app: &App<MockSide>, id: &str) -> Rect {
    app.keyboard
        .place_of(id)
        .unwrap_or_else(|| panic!("`{id}` not placed"))
        .rect
}

/// Whether egui's text focus is on scope `id`'s widget.
fn egui_holds(h: &Headless, app: &App<MockSide>, id: &str) -> bool {
    let widget = app.keyboard.place_of(id).and_then(|p| p.widget);
    widget.is_some() && h.ctx.memory(|m| m.focused()) == widget
}

/// The console opened from the footer (Shift+Tab from no focus, then Enter; R-405), a warning logged first so the log
/// has an entry.
fn console() -> (App<MockSide>, Headless) {
    let (mut app, mut h) = fresh();
    app.side().engine().raise_warning();
    settle(&mut h, &mut app);
    app.view.focus.path.clear();
    press(&mut h, &mut app, Key::Tab, SHIFT);
    press(&mut h, &mut app, Key::Enter, NONE);
    settle(&mut h, &mut app);
    assert!(app.is_open(Window::Console), "the console opened");
    assert_eq!(at(&app), want(&["Console", "Header"], NAV), "the landing");
    (app, h)
}

const SEARCH: &str = gui::windows::console::SEARCH;

fn search_text(app: &App<MockSide>) -> String {
    app.windows.console.text.clone()
}

/// Clicks the console's control `id`.
fn click_on(h: &mut Headless, app: &mut App<MockSide>, id: &str) {
    let c = place(app, id).center();
    click(h, app, c);
}

/// What follows reaching a text field: where the focus is, then whether a letter typed next lands in the field.
#[derive(Clone, Debug)]
struct Typed {
    at: At,
    before: String,
    after: String,
}

/// Types one letter and records what the field holds before and after it.
fn observe_letter(
    h: &mut Headless,
    app: &mut App<MockSide>,
    text: impl Fn(&App<MockSide>) -> String,
) -> Typed {
    let at = at(app);
    let before = text(app);
    type_text(h, app, "q");
    Typed {
        at,
        before,
        after: text(app),
    }
}

/// R-456: the field `field` (its path's labels) types at once: the focus on it in interaction mode (the ring blue), and
/// the next letter in its text.
fn check_types_at_once(seen: &Typed, field: &[String], what: &str) {
    assert_eq!(seen.at, (field.to_vec(), INTERACT), "{what}: the landing");
    assert_eq!(
        seen.after,
        format!("{}q", seen.before),
        "{what}: the next letter"
    );
}

/// B6/B7: the focus on `path` in navigation mode (the ring orange), and the next letter typed into nothing: the field's
/// text unchanged.
fn check_navigates(seen: &Typed, path: &[String], what: &str) {
    assert_eq!(seen.at, (path.to_vec(), NAV), "{what}: the landing");
    assert_eq!(seen.after, seen.before, "{what}: a letter typed something");
}

// --- REQ-GUI-189 (R-456): a click on the search field types at once ----------------------------------------------------

#[test]
fn qa_r456_a_click_on_the_search_field_types_at_once_then_g3s_rules_hold() {
    let (mut app, mut h) = console();
    let field = labels_to(&app, SEARCH);
    click_on(&mut h, &mut app, SEARCH);
    assert!(egui_holds(&h, &app, SEARCH), "egui's focus on the field");
    let seen = observe_letter(&mut h, &mut app, search_text);
    check_types_at_once(&seen, &field, "a click on the field");
    // Esc stops typing, keeps the text, navigation mode on the field (§G3, B1).
    press(&mut h, &mut app, Key::Escape, NONE);
    assert_eq!(at(&app), (field.clone(), NAV), "Esc while typing");
    assert_eq!(search_text(&app), "q", "Esc kept the text");
    let after_esc = observe_letter(&mut h, &mut app, search_text);
    check_navigates(&after_esc, &field, "after Esc");

    // Control: the field reached by the keys (Enter on the header, → to the field), a landing, which needs Enter
    // before it types (R-409 G4): the check rejects it.
    let (mut app, mut h) = console();
    press(&mut h, &mut app, Key::Enter, NONE);
    for _ in 0..8 {
        if app.view.focus.path.last().is_some_and(|l| l == SEARCH) {
            break;
        }
        press(&mut h, &mut app, Key::ArrowRight, NONE);
    }
    let landed = observe_letter(&mut h, &mut app, search_text);
    rejects("a search field that needs Enter before it types", || {
        check_types_at_once(&landed, &field, "control")
    });
    // ... and Enter is what that world needs.
    press(&mut h, &mut app, Key::Enter, NONE);
    let entered = observe_letter(&mut h, &mut app, search_text);
    check_types_at_once(&entered, &field, "the control's field after Enter");
}

#[test]
fn qa_r456_tab_while_typing_after_a_click_lands_on_the_log() {
    let (mut app, mut h) = console();
    click_on(&mut h, &mut app, SEARCH);
    type_text(&mut h, &mut app, "ab");
    assert_eq!(search_text(&app), "ab", "typed after the click");
    press(&mut h, &mut app, Key::Tab, NONE);
    assert_eq!(at(&app), want(&["Console", "Log"], NAV), "Tab while typing");
    assert_eq!(search_text(&app), "ab", "Tab kept the text");
    assert!(!egui_holds(&h, &app, SEARCH), "egui kept the field");
}

#[test]
fn qa_r456_a_click_on_any_other_console_control_lands_in_navigation_mode() {
    let (mut app, mut h) = console();
    for label in ["Copy", "Warnings", "Log"] {
        let id = console_id(&app, label);
        click_on(&mut h, &mut app, id);
        let path = labels_to(&app, id);
        let seen = observe_letter(&mut h, &mut app, search_text);
        check_navigates(&seen, &path, &format!("a click on {label}"));
    }
}

/// The id of the console's scope labelled `label` (R-450 A6's names).
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

// --- REQ-GUI-189 (R-456 B6, B7): typing, then a click elsewhere --------------------------------------------------------

/// The console with the search field clicked and "ab" typed in it.
fn typing_ab() -> (App<MockSide>, Headless) {
    let (mut app, mut h) = console();
    click_on(&mut h, &mut app, SEARCH);
    type_text(&mut h, &mut app, "ab");
    assert_eq!(search_text(&app), "ab", "typed after the click");
    assert_eq!(app.view.focus.mode, INTERACT, "typing is interaction mode");
    (app, h)
}

#[test]
fn qa_r456_typing_then_a_click_on_copy_or_the_log_lands_in_navigation_there() {
    for label in ["Copy", "Log"] {
        let (mut app, mut h) = typing_ab();
        let id = console_id(&app, label);
        let path = labels_to(&app, id);
        click_on(&mut h, &mut app, id);
        let seen = observe_letter(&mut h, &mut app, search_text);
        check_navigates(&seen, &path, &format!("typing, then a click on {label}"));
        assert!(
            !egui_holds(&h, &app, SEARCH),
            "{label}: egui kept the field"
        );
        assert_eq!(search_text(&app), "ab", "{label}: the text kept");
    }
    // Control: typing that survives (no click): the letter lands in the field, and the check rejects it, both on the
    // typing and on the focus.
    let (mut app, mut h) = typing_ab();
    let copy = labels_to(&app, console_id(&app, "Copy"));
    let survived = observe_letter(&mut h, &mut app, search_text);
    rejects("typing that survives the click", || {
        check_navigates(
            &Typed {
                at: (copy.clone(), NAV),
                ..survived.clone()
            },
            &copy,
            "control",
        )
    });
    rejects("a focus left on the field", || {
        check_navigates(&survived, &copy, "control")
    });
}

/// A point of the Figure that no open window covers.
fn figure_point(app: &App<MockSide>, h: &Headless) -> Pos2 {
    let figure = Layout::new(h.screen(), PIXELS_PER_POINT).figure;
    let console = place(app, gui::windows::entry(Window::Console).scope);
    let candidates = [
        figure.center(),
        Pos2::new(figure.center().x, figure.top() + 0.2 * figure.height()),
        Pos2::new(
            figure.left() + 0.2 * figure.width(),
            figure.top() + 0.2 * figure.height(),
        ),
    ];
    *candidates
        .iter()
        .find(|p| figure.contains(**p) && !console.expand(4.0).contains(**p))
        .expect("a Figure point outside the console")
}

#[test]
fn qa_r456_typing_then_a_figure_click_ends_typing_and_keeps_the_focus_on_the_field() {
    let (mut app, mut h) = typing_ab();
    let field = labels_to(&app, SEARCH);
    let p = figure_point(&app, &h);
    click(&mut h, &mut app, p);
    let seen = observe_letter(&mut h, &mut app, search_text);
    check_navigates(&seen, &field, "typing, then a Figure click");
    assert!(!egui_holds(&h, &app, SEARCH), "egui kept the field");
    assert!(app.keyboard.typing().is_none(), "still typing");
    assert!(
        app.is_open(Window::Console),
        "the Figure click closed the console"
    );

    // Control 1: a Figure click that moved the focus to the Figure scope.
    let figure = labels_to(&app, gui::explore::FIGURE);
    assert_eq!(figure, ["Figure"], "the Figure's path");
    rejects("a Figure click that moves the focus to the Figure", || {
        check_navigates(
            &Typed {
                at: (figure.clone(), NAV),
                ..seen.clone()
            },
            &field,
            "control",
        )
    });
    // The same control as a world: the focus moved onto the Figure; the check on the field rejects it.
    let (mut app, mut h) = typing_ab();
    let p = figure_point(&app, &h);
    click(&mut h, &mut app, p);
    app.view.focus.path = app
        .keyboard
        .tree(app.view.mode)
        .path_to(gui::explore::FIGURE);
    settle(&mut h, &mut app);
    let moved = observe_letter(&mut h, &mut app, search_text);
    rejects("the focus on the Figure after the click", || {
        check_navigates(&moved, &field, "control")
    });

    // Control 2: a Figure click that left the field typing (no click): the letter lands in the field.
    let (mut app, mut h) = typing_ab();
    let survived = observe_letter(&mut h, &mut app, search_text);
    rejects("a Figure click that leaves the field typing", || {
        check_navigates(&survived, &field, "control")
    });
}

// --- REQ-GUI-190 (R-456 B2): a click on a text field outside the console ----------------------------------------------

/// The Run window open, each of its text fields that is drawn inside the window, with its text's index.
fn run_open() -> (App<MockSide>, Headless) {
    let (mut app, mut h) = fresh();
    let ctx = h.ctx.clone();
    app.open_window(&ctx, Window::Run, None);
    settle(&mut h, &mut app);
    assert!(app.is_open(Window::Run));
    (app, h)
}

fn run_drawn(app: &App<MockSide>) -> Vec<(usize, &'static str)> {
    let window = place(app, gui::windows::entry(Window::Run).scope);
    run::FIELDS
        .iter()
        .enumerate()
        .filter(|(_, f)| {
            app.keyboard
                .place_of(f.id)
                .is_some_and(|p| window.contains_rect(p.rect) && p.rect.height() > 4.0)
        })
        .map(|(i, f)| (i, f.id))
        .collect()
}

#[test]
fn qa_r456_a_click_on_a_run_window_text_field_types_at_once() {
    let (app, _) = run_open();
    let drawn = run_drawn(&app);
    assert!(!drawn.is_empty(), "no Run text field drawn in the window");
    for (i, id) in drawn {
        let (mut app, mut h) = run_open();
        let field = labels_to(&app, id);
        let text = move |a: &App<MockSide>| a.windows.run.text[i].clone();
        click_on(&mut h, &mut app, id);
        let seen = observe_letter(&mut h, &mut app, text);
        check_types_at_once(&seen, &field, &format!("a click on {id}"));

        // Control: the field reached by a landing (the keys' focus on it, navigation mode): it needs Enter.
        let (mut app, mut h) = run_open();
        app.view.focus.path = app.keyboard.tree(app.view.mode).path_to(id);
        settle(&mut h, &mut app);
        let landed = observe_letter(&mut h, &mut app, text);
        rejects(&format!("{id} needing Enter before it types"), || {
            check_types_at_once(&landed, &field, "control")
        });
        press(&mut h, &mut app, Key::Enter, NONE);
        let entered = observe_letter(&mut h, &mut app, text);
        check_types_at_once(&entered, &field, &format!("{id} after Enter"));
    }
}

fn stain_page() -> (App<MockSide>, Headless) {
    let (mut app, mut h) = fresh();
    app.view.mode = Mode::Stain;
    settle(&mut h, &mut app);
    (app, h)
}

#[test]
fn qa_r456_a_click_on_the_stain_librarys_filter_types_at_once() {
    const FILTER: &str = gui::stain::FILTER;
    let filter_text = |a: &App<MockSide>| a.stain.library.filter.clone();
    let (mut app, mut h) = stain_page();
    let field = labels_to(&app, FILTER);
    click_on(&mut h, &mut app, FILTER);
    let seen = observe_letter(&mut h, &mut app, filter_text);
    check_types_at_once(&seen, &field, "a click on the filter");
    // §G3 holds after the click: Esc keeps the text, navigation mode on the field.
    press(&mut h, &mut app, Key::Escape, NONE);
    assert_eq!(at(&app), (field.clone(), NAV), "Esc while typing");
    assert_eq!(filter_text(&app), "q");

    // Control: the filter reached by a landing needs Enter.
    let (mut app, mut h) = stain_page();
    app.view.focus.path = app.keyboard.tree(Mode::Stain).path_to(FILTER);
    settle(&mut h, &mut app);
    let landed = observe_letter(&mut h, &mut app, filter_text);
    rejects("a filter needing Enter before it types", || {
        check_types_at_once(&landed, &field, "control")
    });
    press(&mut h, &mut app, Key::Enter, NONE);
    let entered = observe_letter(&mut h, &mut app, filter_text);
    check_types_at_once(&entered, &field, "the filter after Enter");
}

#[test]
fn qa_r456_typing_in_the_filter_then_a_click_off_it_ends_typing() {
    // B6 on a page with no console: a click that is not on a text input (a point of the library drawer just above the
    // filter) ends typing and leaves the keyboard in navigation mode.
    const FILTER: &str = gui::stain::FILTER;
    let filter_text = |a: &App<MockSide>| a.stain.library.filter.clone();
    let (mut app, mut h) = stain_page();
    click_on(&mut h, &mut app, FILTER);
    type_text(&mut h, &mut app, "ab");
    assert_eq!(filter_text(&app), "ab");
    let r = place(&app, FILTER);
    // Just above the filter: the library drawer, no text input.
    let p = Pos2::new(r.center().x, r.top() - 12.0);
    click(&mut h, &mut app, p);
    let seen = observe_letter(&mut h, &mut app, filter_text);
    check_ended(&seen, "a click off the filter");
    assert!(app.keyboard.typing().is_none(), "still typing");
    assert!(!egui_holds(&h, &app, FILTER), "egui kept the filter");
    // Control: no click, the filter still typing: the check rejects it.
    let (mut app, mut h) = stain_page();
    click_on(&mut h, &mut app, FILTER);
    type_text(&mut h, &mut app, "ab");
    let survived = observe_letter(&mut h, &mut app, filter_text);
    rejects("typing that survives", || check_ended(&survived, "control"));
}

/// B6: typing ended, navigation mode, the next letter typed into nothing.
fn check_ended(seen: &Typed, what: &str) {
    assert_eq!(seen.at.1, NAV, "{what}: the mode");
    assert_eq!(seen.after, seen.before, "{what}: a letter typed something");
}

// --- REQ-GUI-189 (R-456 B3): a drag on the search field types at once, as a click does --------------------------------

/// A press at `from`, the pointer moved to `to` with the button held, then the release: a drag.
fn drag(h: &mut Headless, app: &mut App<MockSide>, from: Pos2, to: Pos2) {
    let button = |pos, pressed| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: NONE,
    };
    let _ = h.frame(app, vec![Event::PointerMoved(from)]);
    let _ = h.frame(app, vec![button(from, true)]);
    let mid = from + (to - from) * 0.5;
    let _ = h.frame(app, vec![Event::PointerMoved(mid)]);
    let _ = h.frame(app, vec![Event::PointerMoved(to)]);
    let _ = h.frame(app, vec![button(to, false)]);
    let _ = h.frame(app, vec![Event::PointerGone]);
    settle(h, app);
}

#[test]
fn qa_r456_a_drag_across_the_search_field_types_at_once() {
    // A press on the field and a drag along it (a selection, B3's caret) is a press on a text input: it starts typing,
    // as a click does.
    let (mut app, mut h) = console();
    let field = labels_to(&app, SEARCH);
    let rect = place(&app, SEARCH);
    let (from, to) = (
        Pos2::new(rect.left() + 0.25 * rect.width(), rect.center().y),
        Pos2::new(rect.left() + 0.75 * rect.width(), rect.center().y),
    );
    drag(&mut h, &mut app, from, to);
    assert!(egui_holds(&h, &app, SEARCH), "egui's focus on the field");
    let seen = observe_letter(&mut h, &mut app, search_text);
    check_types_at_once(&seen, &field, "a drag on the field");

    // Control: a drag that starts on Copy and ends on the field presses no text input; it lands on Copy (R-450 A3) or
    // nowhere new, in navigation mode, and the check rejects it.
    let (mut app, mut h) = console();
    let (copy, onto) = (
        place(&app, console_id(&app, "Copy")).center(),
        place(&app, SEARCH).center(),
    );
    drag(&mut h, &mut app, copy, onto);
    let off = observe_letter(&mut h, &mut app, search_text);
    rejects("a drag that never pressed the field", || {
        check_types_at_once(&off, &field, "control")
    });
}
