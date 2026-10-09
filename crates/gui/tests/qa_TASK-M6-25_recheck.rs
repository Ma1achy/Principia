//! QA tests for TASK-M6-25, second round, written from the requirements and render_gui_spec §G3, not from the
//! implementation:
//!
//! - REQ-GUI-169: Enter into a scope, Esc back out one level and arrows between siblings in every scope the GUI has,
//!   the top bar's menus included (the task's Notes: the top bar's menus join the tree as scope 1; RQ-250: Help's
//!   "Keys (?)" opens the shortcuts); `?` shows the shortcuts *over everything*, so nothing beneath acts while they are
//!   open (applied per R-369 on PR #174: a click reaches nothing beneath; Esc and `?` close it); each later screen
//!   joins the tree, its keys in the shortcuts too; F3 (§G1, §G2's "F3 hide") is one of the shortcuts.
//! - REQ-GUI-146 / REQ-GUI-169: held keys delay, then repeat, while held: a key whose release can no longer arrive (the
//!   window lost the focus, or the layer was hidden) stops repeating.
//!
//! gui takes no dependency on validation (R-187), so each check runs its own control (R-176) in the test, through
//! [`rejects`].
// The file name `qa_TASK-M6-25_recheck` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use eframe::egui::{Event, Key, Modifiers, PointerButton, Pos2};
use engine::contract::view_ui::Mode;
use gui::app::App;
use gui::capture::{PIXELS_PER_POINT, SIZE};
use gui::headless::{Headless, Name};
use gui::keyboard::repeat::{DELAY_MS, INTERVAL_MS};
use gui::keyboard::scopes::{Scope, StepKind};
use gui::mock::MockEngine;
use gui::side::MockSide;

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const NONE: Modifiers = Modifiers::NONE;
const SHIFT: Modifiers = Modifiers::SHIFT;

/// §G3's key column.
const G3_KEYS: [&str; 8] = [
    "Tab / Shift+Tab",
    "Enter",
    "Esc",
    "arrows",
    "Shift · Alt",
    "held keys",
    "Ctrl+Z",
    "?",
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

/// The labels of the focused path, as the tree names them.
fn focus_labels(app: &App<MockSide>) -> Vec<String> {
    app.keyboard
        .tree(app.view.mode)
        .labels(&app.view.focus.path)
        .into_iter()
        .map(str::to_owned)
        .collect()
}

fn check_path(got: &[String], want: &[&str], what: &str) {
    assert!(
        got.len() == want.len()
            && got
                .iter()
                .zip(want)
                .all(|(g, w)| g.to_lowercase() == w.to_lowercase()),
        "{what}: the focus is {got:?}, want {want:?}"
    );
}

fn names(h: &mut Headless, app: &mut App<MockSide>) -> Vec<Name> {
    let out = h.frame(app, Vec::new());
    h.names(&out)
}

fn drawn(n: &[Name], name: &str) -> bool {
    n.iter().any(|x| x.name == name)
}

fn check_shortcuts_shown(n: &[Name]) {
    for key in G3_KEYS {
        assert!(drawn(n, key), "the shortcuts do not show §G3's `{key}`");
    }
}

// --- REQ-GUI-169: the top bar's menus are scopes: Enter in, arrows between siblings, Esc out one level ---------------

#[test]
fn qa_mock_keyboard_menu_enter_goes_in_esc_backs_out_one_level() {
    let mut app = mock_app();
    let mut h = headless();
    press(&mut h, &mut app, Key::Tab, NONE);
    check_path(&focus_labels(&app), &["top bar"], "Tab from no focus");
    press(&mut h, &mut app, Key::Enter, NONE);
    check_path(
        &focus_labels(&app),
        &["top bar", "File"],
        "Enter on the top bar",
    );
    let closed = names(&mut h, &mut app);
    assert!(
        !drawn(&closed, "Quit"),
        "File's entries shown before Enter on File"
    );
    // Enter on File: into its entries, which are shown.
    press(&mut h, &mut app, Key::Enter, NONE);
    check_path(
        &focus_labels(&app),
        &["top bar", "File", "Quit"],
        "Enter on File",
    );
    let open = names(&mut h, &mut app);
    assert!(
        drawn(&open, "Quit"),
        "Enter on File does not show its entries"
    );
    rejects("the focus left on File", || {
        check_path(&focus_labels(&app), &["top bar", "File"], "Enter on File")
    });
    // Esc: one level out, onto File, its entries gone; again, onto the top bar.
    press(&mut h, &mut app, Key::Escape, NONE);
    check_path(&focus_labels(&app), &["top bar", "File"], "Esc in File");
    let after = names(&mut h, &mut app);
    assert!(
        !drawn(&after, "Quit"),
        "File's entries still shown after Esc"
    );
    press(&mut h, &mut app, Key::Escape, NONE);
    check_path(&focus_labels(&app), &["top bar"], "Esc on File");
}

#[test]
fn qa_mock_keyboard_arrows_move_between_the_top_bar_menus() {
    let mut app = mock_app();
    let mut h = headless();
    press(&mut h, &mut app, Key::Tab, NONE);
    press(&mut h, &mut app, Key::Enter, NONE);
    press(&mut h, &mut app, Key::ArrowRight, NONE);
    check_path(&focus_labels(&app), &["top bar", "View"], "→ from File");
    press(&mut h, &mut app, Key::ArrowLeft, NONE);
    check_path(&focus_labels(&app), &["top bar", "File"], "← from View");
    press(&mut h, &mut app, Key::ArrowRight, NONE);
    press(&mut h, &mut app, Key::Enter, NONE);
    let labels = focus_labels(&app);
    check_path(&labels, &["top bar", "View", "F3 hide"], "Enter on View");
    rejects("File's entry under View", || {
        check_path(&labels, &["top bar", "View", "Quit"], "Enter on View")
    });
    // Moving between a menu's entries and out: Esc gives back View, the menu closed, the layer still shown.
    press(&mut h, &mut app, Key::Escape, NONE);
    check_path(&focus_labels(&app), &["top bar", "View"], "Esc in View");
    assert!(app.shown, "Esc in View acted on its F3 hide entry");
}

#[test]
fn qa_mock_keyboard_help_keys_entry_opens_the_shortcuts() {
    // RQ-250 (decided per R-369): Help's "Keys (?)" opens the `?` shortcuts; by keyboard, Enter on it is its click.
    let mut app = mock_app();
    let mut h = headless();
    press(&mut h, &mut app, Key::Tab, NONE);
    press(&mut h, &mut app, Key::Enter, NONE);
    for _ in 0..12 {
        if focus_labels(&app).last().is_some_and(|l| l == "Help") {
            break;
        }
        press(&mut h, &mut app, Key::ArrowRight, NONE);
    }
    check_path(
        &focus_labels(&app),
        &["top bar", "Help"],
        "→ along the top bar",
    );
    let before = names(&mut h, &mut app);
    rejects("the shortcuts before Help's entry", || {
        check_shortcuts_shown(&before)
    });
    press(&mut h, &mut app, Key::Enter, NONE);
    let labels = focus_labels(&app);
    assert_eq!(labels.len(), 3, "Enter on Help: the focus is {labels:?}");
    assert!(
        labels[2].contains("Keys"),
        "Enter on Help: the focus is {labels:?}, not on its Keys entry"
    );
    press(&mut h, &mut app, Key::Enter, NONE);
    let open = names(&mut h, &mut app);
    check_shortcuts_shown(&open);
    press(&mut h, &mut app, Key::Escape, NONE);
    let after = names(&mut h, &mut app);
    rejects("the shortcuts after Esc", || check_shortcuts_shown(&after));
}

// --- REQ-GUI-169: the shortcuts are over everything: a click reaches nothing beneath --------------------------------

/// A click at `pos` (points): move, press, release, each its own frame, then one idle frame.
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

/// The centre, in points, of the drawn name `want`.
fn centre_of(n: &[Name], want: &str) -> Pos2 {
    let r = n
        .iter()
        .find(|x| x.name == want)
        .and_then(|x| x.rect)
        .unwrap_or_else(|| panic!("no `{want}` drawn"));
    let p = f64::from(PIXELS_PER_POINT);
    Pos2::new(
        ((r[0] + r[2]) / 2.0 / p) as f32,
        ((r[1] + r[3]) / 2.0 / p) as f32,
    )
}

#[test]
fn qa_mock_keyboard_shortcuts_take_the_pointer() {
    let mut app = mock_app();
    let mut h = headless();
    let n = names(&mut h, &mut app);
    let stain = centre_of(&n, "Stain");

    // Control: with the shortcuts closed, the click switches the mode.
    {
        let mut app = mock_app();
        let mut h = headless();
        let _ = names(&mut h, &mut app);
        click(&mut h, &mut app, stain);
        assert_eq!(
            app.view.mode,
            Mode::Stain,
            "the control: a click on Stain does not switch the mode"
        );
    }

    press(&mut h, &mut app, Key::Questionmark, SHIFT);
    check_shortcuts_shown(&names(&mut h, &mut app));
    // A click inside the shortcuts' frame (centred on the window) keeps them open.
    let middle = Pos2::new(
        SIZE[0] as f32 / PIXELS_PER_POINT / 2.0,
        SIZE[1] as f32 / PIXELS_PER_POINT / 2.0,
    );
    click(&mut h, &mut app, middle);
    assert!(
        app.keyboard.shortcuts_open,
        "a click on the shortcuts closed them"
    );
    // A click on Stain, beneath them: nothing beneath acts.
    click(&mut h, &mut app, stain);
    assert_eq!(
        app.view.mode,
        Mode::Explore,
        "a click reached the mode switch beneath the shortcuts"
    );
    assert!(
        !app.keyboard.shortcuts_open,
        "a click outside the shortcuts left them open"
    );
    // Closed, the next click acts.
    click(&mut h, &mut app, stain);
    assert_eq!(
        app.view.mode,
        Mode::Stain,
        "a click after the shortcuts closed did not act"
    );
}

#[test]
fn qa_mock_keyboard_question_mark_closes_the_shortcuts() {
    let mut app = mock_app();
    let mut h = headless();
    press(&mut h, &mut app, Key::Questionmark, SHIFT);
    assert!(
        app.keyboard.shortcuts_open,
        "`?` did not open the shortcuts"
    );
    // While open, keys reach nothing beneath: Tab moves no focus.
    let focus = app.view.focus.path.clone();
    press(&mut h, &mut app, Key::Tab, NONE);
    assert_eq!(
        app.view.focus.path, focus,
        "Tab moved the focus beneath the shortcuts"
    );
    press(&mut h, &mut app, Key::Questionmark, SHIFT);
    assert!(
        !app.keyboard.shortcuts_open,
        "`?` did not close the shortcuts"
    );
    let n = names(&mut h, &mut app);
    rejects("the shortcuts after the second `?`", || {
        check_shortcuts_shown(&n)
    });
}

// --- REQ-GUI-169: F3 and the later screens' keys are in the shortcuts -----------------------------------------------

#[test]
fn qa_mock_keyboard_shortcuts_list_f3_and_a_later_screens_keys() {
    let mut app = mock_app();
    app.keyboard
        .tree_mut(Mode::Explore)
        .register_shortcut("qa-key", "qa later screen action");
    let mut h = headless();
    let closed = names(&mut h, &mut app);
    let listed = |n: &[Name]| {
        assert!(drawn(n, "F3"), "the shortcuts do not list F3");
        assert!(
            drawn(n, "qa-key"),
            "the shortcuts do not list a later screen's key"
        );
        assert!(
            drawn(n, "qa later screen action"),
            "the shortcuts do not list a later screen's action"
        );
    };
    rejects("the shortcuts closed", || listed(&closed));
    press(&mut h, &mut app, Key::Questionmark, SHIFT);
    let open = names(&mut h, &mut app);
    check_shortcuts_shown(&open);
    listed(&open);
}

// --- REQ-GUI-146 / REQ-GUI-169: a held key stops when its release can no longer come --------------------------------

/// An app with a value `zoom` joined under Navigate, focused (§G3's "Manifold view › Navigate › zoom").
fn zoom_app() -> (App<MockSide>, Headless) {
    let mut app = mock_app();
    let tree = app.keyboard.tree_mut(Mode::Explore);
    let manifold = tree
        .children(None)
        .into_iter()
        .find(|id| tree.get(id).is_some_and(|s| s.label == "Manifold view"))
        .expect("a Manifold view scope");
    let navigate = tree
        .children(Some(manifold))
        .into_iter()
        .find(|id| tree.get(id).is_some_and(|s| s.label == "Navigate"))
        .expect("Navigate under Manifold view");
    tree.register(
        Some(navigate),
        Scope::value("qa_zoom", "zoom", StepKind::Bounded),
    );
    let mut h = headless();
    for (k, m) in [
        (Key::Tab, NONE),
        (Key::Tab, NONE),
        (Key::Enter, NONE),
        (Key::ArrowDown, NONE),
        (Key::Enter, NONE),
    ] {
        press(&mut h, &mut app, k, m);
    }
    check_path(
        &focus_labels(&app),
        &["Manifold view", "Navigate", "zoom"],
        "the zoom value",
    );
    h.set_frame_step(0.005);
    (app, h)
}

/// The adjustments over `ms` of idle frames, 5 ms apart.
fn run_for(h: &mut Headless, app: &mut App<MockSide>, ms: u64) -> usize {
    let mut n = 0;
    for _ in 0..ms / 5 {
        let _ = h.frame(app, Vec::new());
        n += app.keyboard.adjusted().len();
    }
    n
}

/// Holds ↑ past the delay, so it is repeating; returns the adjustments so far.
fn hold_until_repeating(h: &mut Headless, app: &mut App<MockSide>) -> usize {
    let _ = h.frame(app, vec![key_event(Key::ArrowUp, true, NONE)]);
    let mut n = app.keyboard.adjusted().len();
    n += run_for(h, app, DELAY_MS + 3 * INTERVAL_MS);
    assert!(n >= 3, "↑ held past the delay gave {n} adjustments");
    n
}

#[test]
fn qa_mock_keyboard_held_key_stops_when_the_window_loses_focus() {
    let window = DELAY_MS + 10 * INTERVAL_MS;
    // Control: still held, with the focus, it keeps repeating.
    {
        let (mut app, mut h) = zoom_app();
        hold_until_repeating(&mut h, &mut app);
        let more = run_for(&mut h, &mut app, window);
        assert!(more >= 5, "the control: held ↑ stopped repeating ({more})");
    }
    let (mut app, mut h) = zoom_app();
    hold_until_repeating(&mut h, &mut app);
    let _ = h.frame(&mut app, vec![Event::WindowFocused(false)]);
    let after = run_for(&mut h, &mut app, window);
    assert_eq!(
        after, 0,
        "↑ repeated {after} times after the window lost the focus"
    );
    // Back in focus, the release that never came does not hold the key: a new press is one step, then the delay.
    let _ = h.frame(&mut app, vec![Event::WindowFocused(true)]);
    let _ = h.frame(&mut app, vec![key_event(Key::ArrowUp, true, NONE)]);
    let first = app.keyboard.adjusted().len();
    let _ = h.frame(&mut app, vec![key_event(Key::ArrowUp, false, NONE)]);
    let rest = run_for(&mut h, &mut app, window);
    assert_eq!(
        (first, rest),
        (1, 0),
        "a fresh press after the focus came back"
    );
}

#[test]
fn qa_mock_keyboard_held_key_adjusts_nothing_while_the_layer_is_hidden() {
    // Frames 5 ms apart and the delay and interval multiples of 5 ms: the hold's last shown frame falls on a repeat.
    const { assert!(DELAY_MS.is_multiple_of(5) && INTERVAL_MS.is_multiple_of(5)) };
    let window = DELAY_MS + 10 * INTERVAL_MS;
    let (mut app, mut h) = zoom_app();
    hold_until_repeating(&mut h, &mut app);
    // F3 hides the layer while ↑ is held: the hidden layer takes no key, so no frame asks a value to move.
    let _ = h.frame(&mut app, vec![key_event(Key::F3, true, NONE)]);
    assert!(!app.shown, "F3 did not hide the layer");
    let mut hidden = app.keyboard.adjusted().len();
    let _ = h.frame(&mut app, vec![key_event(Key::F3, false, NONE)]);
    hidden += app.keyboard.adjusted().len();
    hidden += run_for(&mut h, &mut app, window);
    // Its release, egui's now, arrives while hidden.
    let _ = h.frame(&mut app, vec![key_event(Key::ArrowUp, false, NONE)]);
    hidden += app.keyboard.adjusted().len();
    assert_eq!(
        hidden, 0,
        "the keyboard reported {hidden} adjustments of the focused value with the layer hidden"
    );
}

#[test]
fn qa_mock_keyboard_held_key_does_not_resume_after_the_layer_is_shown_again() {
    let window = DELAY_MS + 10 * INTERVAL_MS;
    // Released while hidden.
    let (mut app, mut h) = zoom_app();
    hold_until_repeating(&mut h, &mut app);
    press(&mut h, &mut app, Key::F3, NONE);
    assert!(!app.shown, "F3 did not hide the layer");
    let _ = run_for(&mut h, &mut app, window);
    let _ = h.frame(&mut app, vec![key_event(Key::ArrowUp, false, NONE)]);
    press(&mut h, &mut app, Key::F3, NONE);
    assert!(app.shown, "F3 did not show the layer again");
    let shown = run_for(&mut h, &mut app, window);
    assert_eq!(
        shown, 0,
        "↑ repeated {shown} times once the layer was shown again"
    );

    // Hidden then shown with ↑ still held and never released: it does not resume repeating either.
    let (mut app, mut h) = zoom_app();
    hold_until_repeating(&mut h, &mut app);
    press(&mut h, &mut app, Key::F3, NONE);
    press(&mut h, &mut app, Key::F3, NONE);
    assert!(app.shown, "F3 twice left the layer hidden");
    let resumed = run_for(&mut h, &mut app, window);
    assert_eq!(
        resumed, 0,
        "↑ resumed repeating ({resumed}) after the layer was hidden and shown"
    );
}
