//! `mock_keyboard` (REQ-GUI-169, REQ-GUI-098, REQ-GUI-146, REQ-GUI-158; render_gui_spec §G3): synthetic key events on
//! the mock move the focus and values as §G3 gives. Tab and Shift+Tab between big scopes, Enter in and Esc out one
//! level, arrows between siblings or adjusting a focused value, Shift ×10 and Alt ×0.1, held keys' delay then repeat,
//! and `?` over everything, closed by Esc. The top bar is scope 1, its controls reached with Enter. A screen joins the
//! tree through its registration. Each later track task extends this file with its own scopes (R-390). Each check
//! also runs on an input it must reject (R-176, through `rejects`).

use std::sync::Arc;

use eframe::egui::{self, Event, Key, Modifiers};
use engine::contract::view_ui::{Focus, Mode};

use super::support::{click, focus, frame_texts, headless, mock_app, press, rejects};
use crate::capture::{shoot, Shot, Step, PIXELS_PER_POINT, SIZE, STEPS};
use crate::explore::breadcrumb::{GLYPH, RING_WIDTH};
use crate::explore::top_bar::{CONTROLS, KEYS_ENTRY};
use crate::headless::{Headless, Name};
use crate::keyboard::keymap::{self, Command, Direction, SHORTCUTS};
use crate::keyboard::overlay::{CLOSE_HINT, TITLE};
use crate::keyboard::repeat::{self, Repeat, DELAY_MS, INTERVAL_MS};
use crate::keyboard::scopes::{Adjust, Base, Outcome, Scope, ScopeTree, StepKind};
use crate::layout::Layout;
use crate::mock::canvas::MockCanvas;

const NONE: Modifiers = Modifiers::NONE;
const SHIFT: Modifiers = Modifiers::SHIFT;
const ALT: Modifiers = Modifiers::ALT;

/// Presses each key of `keys` in turn.
fn keys<S: crate::side::EngineSide>(
    h: &mut Headless,
    app: &mut crate::app::App<S>,
    keys: &[(Key, Modifiers)],
) {
    for (key, modifiers) in keys {
        press(h, app, *key, *modifiers);
    }
}

// --- Tab, Enter, Esc and the arrows between siblings -----------------------------------------------------------------

/// The focus paths after each step of the walk Tab, Tab, Enter, ↓, ↓, ↓, ↓, ↑, ←, →, Esc, Esc, Esc.
fn sibling_walk() -> (Vec<Vec<String>>, Vec<String>) {
    let mut app = mock_app();
    let mut h = headless();
    let mut paths = Vec::new();
    let mut crumb = Vec::new();
    for (key, modifiers) in [
        (Key::Tab, NONE),
        (Key::Tab, NONE),
        (Key::Enter, NONE),
        (Key::ArrowDown, NONE),
        (Key::ArrowDown, NONE),
        (Key::ArrowDown, NONE),
        (Key::ArrowDown, NONE),
        (Key::ArrowUp, NONE),
        (Key::ArrowLeft, NONE),
        (Key::ArrowRight, NONE),
        (Key::Escape, NONE),
        (Key::Escape, NONE),
        (Key::Escape, NONE),
    ] {
        press(&mut h, &mut app, key, modifiers);
        paths.push(focus(&app).iter().map(|s| (*s).to_owned()).collect());
        // The breadcrumb is the name in the top bar that is not one of its controls or its status line.
        let bar = Layout::new(h.screen(), PIXELS_PER_POINT).top_bar;
        let bottom = f64::from(bar.max.y * PIXELS_PER_POINT);
        let names = super::support::names(&mut h, &mut app);
        crumb.push(
            names
                .iter()
                .filter(|n| n.rect.is_some_and(|r| r[3] <= bottom))
                .map(|n| n.name.clone())
                .find(|t| t.starts_with("Manifold view"))
                .unwrap_or_default(),
        );
    }
    (paths, crumb)
}

fn check_sibling_walk(paths: &[Vec<String>], expected: &[&[&str]]) {
    assert_eq!(paths.len(), expected.len());
    for (i, (path, want)) in paths.iter().zip(expected).enumerate() {
        assert_eq!(path, want, "step {i}");
    }
}

#[test]
fn mock_keyboard_enter_esc_and_arrows_between_siblings() {
    let (paths, crumbs) = sibling_walk();
    let mv = "manifold_view";
    let expected: [&[&str]; 13] = [
        &["top_bar"],
        &[mv],
        &[mv, "chart"],
        &[mv, "navigate"],
        &[mv, "centre_z0"],
        &[mv, "slice_tilt"],
        // The arrows stop at the last sibling.
        &[mv, "slice_tilt"],
        &[mv, "centre_z0"],
        &[mv, "navigate"],
        &[mv, "centre_z0"],
        &[mv],
        &[],
        // Esc with nothing focused does nothing.
        &[],
    ];
    check_sibling_walk(&paths, &expected);
    assert_eq!(crumbs[3], "Manifold view › Navigate");
    assert_eq!(crumbs[5], "Manifold view › Slice & tilt");
    assert_eq!(crumbs[10], "Manifold view");
    assert_eq!(crumbs[11], "", "a breadcrumb with nothing focused");
    let back_to_chart = [mv, "chart"];
    let mut wrapped = expected;
    wrapped[6] = &back_to_chart;
    rejects("arrows wrapping at the end", || {
        check_sibling_walk(&paths, &wrapped)
    });
}

/// From the first sibling, ↑ and ← stay; at the big-scope level the arrows move between the big scopes.
#[test]
fn mock_keyboard_arrows_stop_at_the_first_sibling() {
    let mut app = mock_app();
    let mut h = headless();
    keys(
        &mut h,
        &mut app,
        &[(Key::Tab, NONE), (Key::Tab, NONE), (Key::Enter, NONE)],
    );
    press(&mut h, &mut app, Key::ArrowUp, NONE);
    press(&mut h, &mut app, Key::ArrowLeft, NONE);
    assert_eq!(focus(&app), ["manifold_view", "chart"]);
    press(&mut h, &mut app, Key::Escape, NONE);
    press(&mut h, &mut app, Key::ArrowRight, NONE);
    assert_eq!(focus(&app), ["figure"]);
    press(&mut h, &mut app, Key::ArrowLeft, NONE);
    press(&mut h, &mut app, Key::ArrowLeft, NONE);
    press(&mut h, &mut app, Key::ArrowLeft, NONE);
    assert_eq!(focus(&app), ["top_bar"]);
    rejects("a focus that moved", || assert_eq!(focus(&app), ["legend"]));
}

// --- The top bar, scope 1 --------------------------------------------------------------------------------------------

/// Enter on the top bar reaches its controls in the order they are drawn, each with its breadcrumb.
fn top_bar_walk() -> Vec<(Vec<String>, bool)> {
    let mut app = mock_app();
    let mut h = headless();
    keys(&mut h, &mut app, &[(Key::Tab, NONE), (Key::Enter, NONE)]);
    let mut seen = Vec::new();
    for (i, (_, label)) in CONTROLS.iter().enumerate() {
        if i > 0 {
            press(&mut h, &mut app, Key::ArrowRight, NONE);
        }
        let path: Vec<String> = focus(&app).iter().map(|s| (*s).to_owned()).collect();
        let crumb = format!("Top bar › {label}");
        seen.push((path, frame_texts(&mut h, &mut app).contains(&crumb)));
    }
    seen
}

fn check_top_bar(seen: &[(Vec<String>, bool)], order: &[&str]) {
    assert_eq!(seen.len(), order.len());
    for ((path, crumb), id) in seen.iter().zip(order) {
        assert_eq!(path, &["top_bar".to_owned(), (*id).to_owned()]);
        assert!(crumb, "no breadcrumb for `{id}`");
    }
}

#[test]
fn mock_keyboard_top_bar_controls_in_order() {
    let seen = top_bar_walk();
    let order: Vec<&str> = CONTROLS.iter().map(|(id, _)| *id).collect();
    assert_eq!(
        order,
        [
            "file", "view", "windows", "overlays", "run", "profiler", "export", "help", "explore",
            "stain"
        ]
    );
    check_top_bar(&seen, &order);
    let mut reversed = order.clone();
    reversed.reverse();
    rejects("the controls reversed", || check_top_bar(&seen, &reversed));
}

/// Enter on the mode switch switches the mode, keeping the focus; on the Stain page Tab reaches only the top bar.
#[test]
fn mock_keyboard_enter_switches_mode() {
    let mut app = mock_app();
    let mut h = headless();
    keys(&mut h, &mut app, &[(Key::Tab, NONE), (Key::Enter, NONE)]);
    for _ in 0..9 {
        press(&mut h, &mut app, Key::ArrowRight, NONE);
    }
    assert_eq!(focus(&app), ["top_bar", "stain"]);
    press(&mut h, &mut app, Key::Enter, NONE);
    assert_eq!(app.view.mode, Mode::Stain);
    assert_eq!(focus(&app), ["top_bar", "stain"]);
    press(&mut h, &mut app, Key::Tab, NONE);
    assert_eq!(focus(&app), ["top_bar"]);
    keys(&mut h, &mut app, &[(Key::Enter, NONE)]);
    for _ in 0..8 {
        press(&mut h, &mut app, Key::ArrowRight, NONE);
    }
    assert_eq!(focus(&app), ["top_bar", "explore"]);
    press(&mut h, &mut app, Key::Enter, NONE);
    assert_eq!(app.view.mode, Mode::Explore);
    rejects("a mode that did not switch", || {
        assert_eq!(app.view.mode, Mode::Stain)
    });
}

/// A focus left on an Explore scope when the mode switches is cut back to what the Stain page holds.
#[test]
fn mock_keyboard_mode_switch_drops_explore_focus() {
    let mut app = mock_app();
    let mut h = headless();
    keys(
        &mut h,
        &mut app,
        &[(Key::Tab, NONE), (Key::Tab, NONE), (Key::Enter, NONE)],
    );
    click(&mut h, &mut app, "Stain");
    press(&mut h, &mut app, Key::ArrowDown, NONE);
    assert!(focus(&app).is_empty(), "{:?}", focus(&app));
    let texts = frame_texts(&mut h, &mut app);
    assert!(!texts.iter().any(|t| t.starts_with("Manifold view ›")));
    press(&mut h, &mut app, Key::Tab, SHIFT);
    assert_eq!(focus(&app), ["top_bar"]);
    rejects("an Explore scope kept", || {
        assert_eq!(focus(&app), ["manifold_view", "chart"])
    });
}

/// Whether a name reading `name` is drawn in one more frame.
fn drawn<S: crate::side::EngineSide>(
    h: &mut Headless,
    app: &mut crate::app::App<S>,
    name: &str,
) -> bool {
    frame_texts(h, app).iter().any(|t| t == name)
}

/// The state of a menu walk: the focus, the menu the layer holds open, and whether egui shows Windows' "Display".
type MenuState = (Vec<String>, Option<&'static str>, bool);

fn menu_state(h: &mut Headless, app: &mut crate::app::App<crate::side::MockSide>) -> MenuState {
    let path = focus(app).iter().map(|s| (*s).to_owned()).collect();
    let display = drawn(h, app, "Display");
    (path, app.keyboard.open_menu(), display)
}

fn state(path: &[&str], open: Option<&'static str>, display: bool) -> MenuState {
    (
        path.iter().map(|s| (*s).to_owned()).collect(),
        open,
        display,
    )
}

/// Enter on a menu opens it on its first entry, with its breadcrumb and ring; the arrows move between its entries,
/// stopping at either end; Enter on a disabled entry does nothing; Esc closes the menu, back to its control; and the
/// layer keeps the keys after.
#[test]
fn mock_keyboard_menu_entries_are_scopes() {
    let mut app = mock_app();
    let mut h = headless();
    keys(
        &mut h,
        &mut app,
        &[
            (Key::Tab, NONE),
            (Key::Enter, NONE),
            (Key::ArrowRight, NONE),
            (Key::ArrowRight, NONE),
        ],
    );
    let mut seen = vec![menu_state(&mut h, &mut app)];
    press(&mut h, &mut app, Key::Enter, NONE);
    seen.push(menu_state(&mut h, &mut app));
    assert!(frame_texts(&mut h, &mut app).contains(&"Top bar › Windows › Run".to_owned()));
    assert!(
        app.keyboard.place_of("windows_run").is_some(),
        "no ring's place on the entry"
    );
    for (key, n) in [
        (Key::ArrowDown, 1),
        (Key::ArrowDown, 4),
        (Key::ArrowUp, 1),
        (Key::Enter, 1),
    ] {
        for _ in 0..n {
            press(&mut h, &mut app, key, NONE);
        }
        seen.push(menu_state(&mut h, &mut app));
    }
    press(&mut h, &mut app, Key::Escape, NONE);
    seen.push(menu_state(&mut h, &mut app));
    assert!(!egui::Popup::is_any_open(&h.ctx));
    press(&mut h, &mut app, Key::ArrowRight, NONE);
    seen.push(menu_state(&mut h, &mut app));
    let w = Some("windows");
    let want = [
        state(&["top_bar", "windows"], None, false),
        state(&["top_bar", "windows", "windows_run"], w, true),
        state(&["top_bar", "windows", "windows_profiler"], w, true),
        // The arrows stop at the last entry.
        state(&["top_bar", "windows", "windows_console"], w, true),
        state(&["top_bar", "windows", "windows_display"], w, true),
        // Display is disabled: Enter does nothing, the menu stays open.
        state(&["top_bar", "windows", "windows_display"], w, true),
        state(&["top_bar", "windows"], None, false),
        state(&["top_bar", "overlays"], None, false),
    ];
    assert_eq!(seen, want);
    let mut still_open = want.clone();
    still_open[6].2 = true;
    rejects("a menu Esc left open", || assert_eq!(seen, still_open));
}

/// Enter on an entry acts as its click and closes its menu, the focus back on the menu: Help's "Keys (?)" opens the
/// shortcuts, View's "F3 hide" hides the layer.
#[test]
fn mock_keyboard_menu_entry_acts_and_closes() {
    let mut app = mock_app();
    let mut h = headless();
    keys(&mut h, &mut app, &[(Key::Tab, NONE), (Key::Enter, NONE)]);
    for _ in 0..7 {
        press(&mut h, &mut app, Key::ArrowRight, NONE);
    }
    keys(&mut h, &mut app, &[(Key::Enter, NONE)]);
    assert_eq!(focus(&app), ["top_bar", "help", "help_keys"]);
    press(&mut h, &mut app, Key::Enter, NONE);
    assert!(app.keyboard.shortcuts_open);
    assert_eq!(focus(&app), ["top_bar", "help"]);
    assert_eq!(app.keyboard.open_menu(), None);
    assert!(!drawn(&mut h, &mut app, KEYS_ENTRY), "Help is still open");
    press(&mut h, &mut app, Key::Escape, NONE);
    assert!(!app.keyboard.shortcuts_open);
    for _ in 0..6 {
        press(&mut h, &mut app, Key::ArrowLeft, NONE);
    }
    let hidden = |shown: bool| assert!(!shown, "View › F3 hide did not hide the layer");
    let before = app.shown;
    keys(&mut h, &mut app, &[(Key::Enter, NONE), (Key::Enter, NONE)]);
    hidden(app.shown);
    assert_eq!(focus(&app), ["top_bar", "view"]);
    press(&mut h, &mut app, Key::F3, NONE);
    assert!(app.shown);
    assert!(!egui::Popup::is_any_open(&h.ctx));
    // The layer still has the keys: Enter opens View again.
    press(&mut h, &mut app, Key::Enter, NONE);
    assert_eq!(focus(&app), ["top_bar", "view", "view_hide"]);
    rejects("the layer shown, before the entry", || hidden(before));
}

/// Tab out of an open menu goes on to the next big scope and closes the menu.
#[test]
fn mock_keyboard_tab_leaves_a_menu() {
    let mut app = mock_app();
    let mut h = headless();
    keys(
        &mut h,
        &mut app,
        &[(Key::Tab, NONE), (Key::Enter, NONE), (Key::Enter, NONE)],
    );
    let closed = |quit_drawn: bool| assert!(!quit_drawn, "File stayed open after Tab");
    let before = drawn(&mut h, &mut app, "Quit");
    assert!(before);
    press(&mut h, &mut app, Key::Tab, NONE);
    assert_eq!(focus(&app), ["manifold_view"]);
    assert_eq!(app.keyboard.open_menu(), None);
    closed(drawn(&mut h, &mut app, "Quit"));
    rejects("File open, before the Tab", || closed(before));
}

/// Clicks the control named `name`, then runs the frame that sees what the click did.
fn mouse(h: &mut Headless, app: &mut crate::app::App<crate::side::MockSide>, name: &str) {
    click(h, app, name);
    let _ = h.frame(app, Vec::new());
}

/// A menu the mouse opens takes the focus, on its first entry, and follows §G3's keys; a click that closes it gives
/// the focus back to its control, and the layer keeps the keys; Esc closes an open menu with no entries.
#[test]
fn mock_keyboard_mouse_opened_menu_follows_the_keys() {
    let mut app = mock_app();
    let mut h = headless();
    mouse(&mut h, &mut app, "Windows");
    assert_eq!(focus(&app), ["top_bar", "windows", "windows_run"]);
    press(&mut h, &mut app, Key::ArrowDown, NONE);
    assert_eq!(focus(&app), ["top_bar", "windows", "windows_profiler"]);
    assert!(drawn(&mut h, &mut app, "Display"), "↓ closed the menu");
    press(&mut h, &mut app, Key::Escape, NONE);
    assert_eq!(focus(&app), ["top_bar", "windows"]);
    assert!(!egui::Popup::is_any_open(&h.ctx));
    press(&mut h, &mut app, Key::ArrowRight, NONE);
    assert_eq!(focus(&app), ["top_bar", "overlays"]);
    // A second click on File closes it: the focus goes back to File.
    mouse(&mut h, &mut app, "File");
    assert_eq!(focus(&app), ["top_bar", "file", "file_quit"]);
    mouse(&mut h, &mut app, "File");
    assert_eq!(focus(&app), ["top_bar", "file"]);
    assert_eq!(app.keyboard.open_menu(), None);
    press(&mut h, &mut app, Key::ArrowRight, NONE);
    assert_eq!(
        focus(&app),
        ["top_bar", "view"],
        "egui's focus kept the keys"
    );
    // Overlays ▾ has no entries: clicked open, the focus is on it, and Esc closes it there.
    let overlays = format!("Overlays ▾ {}", 0);
    mouse(&mut h, &mut app, &overlays);
    assert_eq!(focus(&app), ["top_bar", "overlays"]);
    assert_eq!(app.keyboard.open_menu(), Some("overlays"));
    press(&mut h, &mut app, Key::Escape, NONE);
    assert_eq!(focus(&app), ["top_bar", "overlays"]);
    assert_eq!(app.keyboard.open_menu(), None);
    assert!(!egui::Popup::is_any_open(&h.ctx));
    // Enter on it finds no entry to open.
    press(&mut h, &mut app, Key::Enter, NONE);
    assert!(!egui::Popup::is_any_open(&h.ctx));
    // Clicked open again, → leaves it, closing it.
    mouse(&mut h, &mut app, &overlays);
    press(&mut h, &mut app, Key::ArrowRight, NONE);
    assert_eq!(focus(&app), ["top_bar", "run"]);
    assert!(!egui::Popup::is_any_open(&h.ctx));
    rejects("a menu left open", || {
        assert!(egui::Popup::is_any_open(&h.ctx))
    });
}

// --- Values: arrows, Shift ×10, Alt ×0.1 -----------------------------------------------------------------------------

/// The app with a zoom value registered under Navigate, as TASK-M6-26 registers it, focused.
fn zoom_app() -> (crate::app::App<crate::side::MockSide>, Headless) {
    let mut app = mock_app();
    app.keyboard.tree_mut(Mode::Explore).register(
        Some("navigate"),
        Scope::value("zoom", "zoom", StepKind::ZoomLog2),
    );
    let mut h = headless();
    keys(
        &mut h,
        &mut app,
        &[
            (Key::Tab, NONE),
            (Key::Tab, NONE),
            (Key::Enter, NONE),
            (Key::ArrowDown, NONE),
            (Key::Enter, NONE),
        ],
    );
    (app, h)
}

/// The adjustment of one press of `key` with `modifiers`.
fn adjust_of(
    h: &mut Headless,
    app: &mut crate::app::App<crate::side::MockSide>,
    key: Key,
    modifiers: Modifiers,
) -> Vec<Adjust> {
    let [down, up] = crate::capture::key(key, modifiers);
    let _ = h.frame(app, down);
    let adjusted = app.keyboard.adjusted().to_vec();
    let _ = h.frame(app, up);
    adjusted
}

fn check_steps(got: &[Vec<Adjust>], want: &[Base]) {
    assert_eq!(got.len(), want.len());
    for (adjusts, base) in got.iter().zip(want) {
        assert_eq!(adjusts.len(), 1, "one press, one adjustment: {adjusts:?}");
        assert_eq!(adjusts[0].scope, "zoom");
        assert_eq!(adjusts[0].delta(), *base);
    }
}

#[test]
fn mock_keyboard_arrows_adjust_with_shift_and_alt() {
    let (mut app, mut h) = zoom_app();
    assert_eq!(focus(&app), ["manifold_view", "navigate", "zoom"]);
    assert!(frame_texts(&mut h, &mut app).contains(&"Manifold view › Navigate › zoom".to_owned()));
    let got: Vec<Vec<Adjust>> = [
        (Key::ArrowUp, NONE),
        (Key::ArrowRight, NONE),
        (Key::ArrowDown, NONE),
        (Key::ArrowLeft, NONE),
        (Key::ArrowUp, SHIFT),
        (Key::ArrowDown, ALT),
        (Key::ArrowRight, Modifiers::SHIFT | Modifiers::ALT),
    ]
    .into_iter()
    .map(|(k, m)| adjust_of(&mut h, &mut app, k, m))
    .collect();
    let want = [
        Base::Log2(0.25),
        Base::Log2(0.25),
        Base::Log2(-0.25),
        Base::Log2(-0.25),
        Base::Log2(2.5),
        Base::Log2(-0.025),
        Base::Log2(0.25),
    ];
    check_steps(&got, &want);
    // The arrows adjust; they do not move the focus.
    assert_eq!(focus(&app), ["manifold_view", "navigate", "zoom"]);
    let mut unscaled = want;
    unscaled[4] = Base::Log2(0.25);
    rejects("Shift not ×10", || check_steps(&got, &unscaled));
}

/// Each step kind's base step, the proposal (REQ-GUI-158), and the modifiers' multipliers.
#[test]
fn mock_keyboard_base_steps_and_multipliers() {
    let table = [
        (StepKind::Bounded, Base::OfRange(0.01)),
        (StepKind::Angle, Base::Degrees(1.0)),
        (StepKind::ZoomLog2, Base::Log2(0.25)),
        (StepKind::Pan, Base::OfView(0.05)),
        (StepKind::Tilt, Base::Degrees(1.0)),
        (StepKind::Orbit, Base::Degrees(5.0)),
        (StepKind::TimeStep, Base::OfRange(0.01)),
    ];
    for (kind, base) in table {
        assert_eq!(kind.base(), base, "{kind:?}");
        let adjust = Adjust {
            scope: "x",
            kind,
            times: -10.0,
        };
        assert_eq!(adjust.delta(), base.times(-10.0));
    }
    assert_eq!(Base::Degrees(2.0).times(0.5), Base::Degrees(1.0));
    assert_eq!(Base::Log2(2.0).times(0.5), Base::Log2(1.0));
    assert_eq!(Base::OfRange(2.0).times(0.5), Base::OfRange(1.0));
    assert_eq!(Base::OfView(2.0).times(0.5), Base::OfView(1.0));
    assert_eq!(keymap::multiplier(NONE), 1.0);
    assert_eq!(keymap::multiplier(SHIFT), 10.0);
    assert_eq!(keymap::multiplier(ALT), 0.1);
    assert_eq!(keymap::multiplier(SHIFT | ALT), 1.0);
    assert_eq!(keymap::multiplier(Modifiers::CTRL), 1.0);
    rejects("Alt as ×10", || assert_eq!(keymap::multiplier(ALT), 10.0));
}

// --- The key table ---------------------------------------------------------------------------------------------------

#[test]
fn mock_keyboard_key_table() {
    use Command::*;
    let cases = [
        (Key::Tab, NONE, Some(Next)),
        (Key::Tab, SHIFT, Some(Previous)),
        (Key::Tab, ALT, None),
        (Key::Tab, Modifiers::SHIFT | Modifiers::ALT, None),
        (Key::Tab, Modifiers::CTRL, None),
        (Key::Enter, NONE, Some(Enter)),
        (Key::Enter, SHIFT, None),
        (Key::Enter, ALT, None),
        (Key::Escape, NONE, Some(Back)),
        (Key::Escape, SHIFT, None),
        (Key::ArrowUp, NONE, Some(Arrow(Direction::Up))),
        (Key::ArrowDown, SHIFT, Some(Arrow(Direction::Down))),
        (Key::ArrowLeft, ALT, Some(Arrow(Direction::Left))),
        (Key::ArrowRight, NONE, Some(Arrow(Direction::Right))),
        (Key::ArrowRight, Modifiers::COMMAND, None),
        (Key::ArrowRight, Modifiers::MAC_CMD, None),
        (Key::ArrowUp, Modifiers::CTRL, None),
        (Key::Questionmark, SHIFT, Some(Shortcuts)),
        (Key::Questionmark, NONE, Some(Shortcuts)),
        (Key::Questionmark, ALT, None),
        (Key::Z, Modifiers::CTRL, None),
        (Key::A, NONE, None),
    ];
    for (key, modifiers, want) in cases {
        assert_eq!(
            keymap::command(key, modifiers),
            want,
            "{key:?} {modifiers:?}"
        );
    }
    assert_eq!(Direction::Up.sign(), 1.0);
    assert_eq!(Direction::Right.sign(), 1.0);
    assert_eq!(Direction::Down.sign(), -1.0);
    assert_eq!(Direction::Left.sign(), -1.0);
    for (command, repeats) in [
        (Next, true),
        (Previous, true),
        (Arrow(Direction::Up), true),
        (Enter, false),
        (Back, false),
        (Shortcuts, false),
    ] {
        assert_eq!(keymap::repeats(command), repeats, "{command:?}");
    }
    rejects("Alt+Tab as Next", || {
        assert_eq!(keymap::command(Key::Tab, ALT), Some(Next))
    });
}

// --- Held keys: delay, then repeat -----------------------------------------------------------------------------------

/// The repeats `due` gives, polled at each of `times`, after a press at 1000 ms.
fn repeats_at(times: &[u64]) -> Vec<u64> {
    let mut r = Repeat::default();
    r.press(Key::ArrowUp, NONE, 1000);
    times
        .iter()
        .map(|t| r.due(*t).map_or(0, |(_, _, n)| n))
        .collect()
}

#[test]
fn mock_keyboard_repeat_boundaries() {
    let t0 = 1000;
    let d = DELAY_MS;
    let i = INTERVAL_MS;
    let got = repeats_at(&[
        t0,
        t0 + d - 1,
        t0 + d,
        t0 + d + i - 1,
        t0 + d + i,
        t0 + d + 4 * i,
    ]);
    assert_eq!(got, [0, 0, 1, 0, 1, 3]);
    // A clock read before the press gives nothing.
    assert_eq!(repeats_at(&[0]), [0]);
    let mut r = Repeat::default();
    assert_eq!(r.due(5000), None);
    // Nothing new is no repeat at all, not a repeat of none.
    r.press(Key::ArrowUp, NONE, t0);
    assert_eq!(r.due(t0 + d - 1), None);
    assert_eq!(r.due(t0 + d), Some((Key::ArrowUp, NONE, 1)));
    assert_eq!(r.due(t0 + d), None);
    let mut r = Repeat::default();
    assert_eq!(r.next_due_ms(), None);
    r.press(Key::ArrowUp, SHIFT, t0);
    assert!(r.holds(Key::ArrowUp) && !r.holds(Key::ArrowDown));
    assert_eq!(r.next_due_ms(), Some(t0 + d));
    assert_eq!(r.due(t0 + d), Some((Key::ArrowUp, SHIFT, 1)));
    assert_eq!(r.next_due_ms(), Some(t0 + d + i));
    // Releasing another key keeps it; releasing it stops it.
    r.release(Key::ArrowDown);
    assert!(r.holds(Key::ArrowUp));
    r.release(Key::ArrowUp);
    assert_eq!(r.due(t0 + 10 * d), None);
    assert_eq!(repeat::millis(0.125), 125);
    assert_eq!(repeat::millis(0.5004), 500);
    assert_eq!(repeat::millis(0.5006), 501);
    // The proposal (REQ-GUI-146).
    assert_eq!((DELAY_MS, INTERVAL_MS), (500, 40));
    rejects("a repeat before the delay", || {
        assert_eq!(repeats_at(&[t0 + d - 1]), [1])
    });
}

/// A held ↑ on the zoom value: one step at once, the next at the delay, then one each interval; the system's own
/// repeats are ignored, and the release stops it. Held Enter acts once.
fn held_counts(step_ms: u64, frames: usize) -> Vec<usize> {
    let (mut app, mut h) = zoom_app();
    h.set_frame_step(step_ms as f64 / 1000.0);
    let down = |repeat| Event::Key {
        key: Key::ArrowUp,
        physical_key: None,
        pressed: true,
        repeat,
        modifiers: NONE,
    };
    let mut counts = Vec::new();
    let _ = h.frame(&mut app, vec![down(false)]);
    counts.push(app.keyboard.adjusted().len());
    for _ in 0..frames {
        let _ = h.frame(&mut app, vec![down(true)]);
        counts.push(app.keyboard.adjusted().len());
    }
    let _ = h.frame(
        &mut app,
        vec![Event::Key {
            key: Key::ArrowUp,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers: NONE,
        }],
    );
    counts.push(app.keyboard.adjusted().len());
    for _ in 0..30 {
        let _ = h.frame(&mut app, Vec::new());
        counts.push(app.keyboard.adjusted().len());
    }
    counts
}

#[test]
fn mock_keyboard_held_key_delays_then_repeats() {
    // Frames 20 ms apart: the press at 0, repeats at 500, 540, 580 … ms.
    let counts = held_counts(20, 30);
    let mut want = vec![0; counts.len()];
    want[0] = 1;
    for (frame, n) in want.iter_mut().enumerate().take(31).skip(1) {
        let t = frame as u64 * 20;
        if t >= DELAY_MS && (t - DELAY_MS).is_multiple_of(INTERVAL_MS) {
            *n = 1;
        }
    }
    assert_eq!(counts, want);
    rejects("the system's repeats counted", || assert_eq!(counts[1], 1));
    // Held Enter acts once: from Manifold view it enters Chart and goes no deeper.
    let mut app = mock_app();
    let mut h = headless();
    keys(&mut h, &mut app, &[(Key::Tab, NONE), (Key::Tab, NONE)]);
    h.set_frame_step(0.1);
    let enter = Event::Key {
        key: Key::Enter,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: NONE,
    };
    let _ = h.frame(&mut app, vec![enter]);
    for _ in 0..20 {
        let _ = h.frame(&mut app, Vec::new());
    }
    assert_eq!(focus(&app), ["manifold_view", "chart"]);
}

/// A held Tab repeats too, moving on through the big scopes.
#[test]
fn mock_keyboard_held_tab_repeats() {
    let mut app = mock_app();
    let mut h = headless();
    h.set_frame_step(DELAY_MS as f64 / 1000.0);
    let tab = Event::Key {
        key: Key::Tab,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: NONE,
    };
    let _ = h.frame(&mut app, vec![tab]);
    assert_eq!(focus(&app), ["top_bar"]);
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(focus(&app), ["manifold_view"]);
    rejects("a Tab that did not repeat", || {
        assert_eq!(focus(&app), ["top_bar"])
    });
}

// --- `?`: the shortcuts, over everything -----------------------------------------------------------------------------

fn check_overlay(texts: &[String]) {
    for name in [TITLE, CLOSE_HINT] {
        assert!(texts.iter().any(|t| t == name), "no `{name}` in {texts:?}");
    }
    for (key, action) in SHORTCUTS {
        assert!(texts.iter().any(|t| t == key), "no key `{key}`");
        assert!(texts.iter().any(|t| t == action), "no action `{action}`");
    }
}

#[test]
fn mock_keyboard_shortcuts_over_everything_closed_by_esc() {
    let mut app = mock_app();
    let mut h = headless();
    keys(&mut h, &mut app, &[(Key::Tab, NONE), (Key::Tab, NONE)]);
    press(&mut h, &mut app, Key::Questionmark, SHIFT);
    assert!(app.keyboard.shortcuts_open);
    let texts = frame_texts(&mut h, &mut app);
    check_overlay(&texts);
    // Over everything: the topmost layer at the window's centre is the overlay's.
    let centre = h.screen().center();
    let top = h.ctx.layer_id_at(centre).expect("a layer at the centre");
    assert_eq!(top.order, egui::Order::Tooltip);
    // While it is open it takes the keys: Tab moves nothing.
    press(&mut h, &mut app, Key::Tab, NONE);
    assert_eq!(focus(&app), ["manifold_view"]);
    press(&mut h, &mut app, Key::Escape, NONE);
    assert!(!app.keyboard.shortcuts_open);
    // Esc closed the overlay and left the focus where it was.
    assert_eq!(focus(&app), ["manifold_view"]);
    let texts = frame_texts(&mut h, &mut app);
    assert!(!texts.iter().any(|t| t == CLOSE_HINT));
    // `?` closes it as well.
    press(&mut h, &mut app, Key::Questionmark, SHIFT);
    press(&mut h, &mut app, Key::Questionmark, SHIFT);
    assert!(!app.keyboard.shortcuts_open);
    rejects("an overlay without its rows", || check_overlay(&texts));
}

/// A frame of a bare context showing a button and a text field, focusing `focus` (`"button"` or `"text"`) once.
fn bare_frame(ctx: &egui::Context, raw: egui::RawInput, focus: Option<&str>) {
    let mut text = String::new();
    let mut output = ctx.run_ui(raw, |ui| {
        let button = ui.button("button");
        let field = ui.text_edit_singleline(&mut text);
        match focus {
            Some("button") => button.request_focus(),
            Some("text") => field.request_focus(),
            _ => {}
        }
    });
    output.textures_delta.clear();
}

fn key_raw(key: Key, pressed: bool) -> egui::RawInput {
    egui::RawInput {
        events: vec![Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: NONE,
        }],
        ..Default::default()
    }
}

/// The layer stands aside only for a text field: a focused button takes no key from it, and loses egui's focus.
#[test]
fn mock_keyboard_steps_aside_only_for_text() {
    let ctx = egui::Context::default();
    let mut keyboard = crate::keyboard::Keyboard::new();
    bare_frame(&ctx, egui::RawInput::default(), Some("button"));
    bare_frame(&ctx, egui::RawInput::default(), None);
    assert!(ctx.memory(|m| m.focused().is_some()) && !ctx.text_edit_focused());
    let mut raw = key_raw(Key::Tab, true);
    keyboard.take_keys(&ctx, &mut raw);
    let button_kept = raw.events.clone();
    assert!(button_kept.is_empty(), "a focused button had the Tab");
    assert_eq!(ctx.memory(|m| m.focused()), None);
    let mut raw = key_raw(Key::Tab, false);
    keyboard.take_keys(&ctx, &mut raw);
    bare_frame(&ctx, egui::RawInput::default(), Some("text"));
    bare_frame(&ctx, egui::RawInput::default(), None);
    assert!(ctx.text_edit_focused());
    let mut raw = key_raw(Key::Tab, true);
    keyboard.take_keys(&ctx, &mut raw);
    assert_eq!(raw.events.len(), 1, "the Tab was taken from a text field");
    rejects("a Tab left to a focused button", || {
        assert_eq!(button_kept.len(), 1)
    });
}

/// A key's release goes to whoever saw its press: one egui saw (a text field had the keyboard) is egui's, though the
/// field has lost the focus; one the layer took is the layer's, though a field has the focus now.
#[test]
fn mock_keyboard_release_goes_to_the_press_owner() {
    let ctx = egui::Context::default();
    let mut keyboard = crate::keyboard::Keyboard::new();
    bare_frame(&ctx, egui::RawInput::default(), Some("text"));
    bare_frame(&ctx, egui::RawInput::default(), None);
    let mut raw = key_raw(Key::Tab, true);
    keyboard.take_keys(&ctx, &mut raw);
    assert_eq!(raw.events.len(), 1);
    bare_frame(&ctx, raw, None);
    assert!(ctx.input(|i| i.key_down(Key::Tab)));
    // The field loses the focus (egui's Tab may already have moved it on).
    if let Some(id) = ctx.memory(|m| m.focused()) {
        ctx.memory_mut(|m| m.surrender_focus(id));
    }
    assert!(!ctx.text_edit_focused());
    let mut raw = key_raw(Key::Tab, false);
    keyboard.take_keys(&ctx, &mut raw);
    let egui_release = raw.events.clone();
    bare_frame(&ctx, raw, None);
    assert_eq!(egui_release.len(), 1, "the layer took egui's release");
    assert!(
        !ctx.input(|i| i.key_down(Key::Tab)),
        "egui thinks Tab is still down"
    );
    // The layer's press, nothing focused, then a field takes the focus: the release is still the layer's.
    if let Some(id) = ctx.memory(|m| m.focused()) {
        ctx.memory_mut(|m| m.surrender_focus(id));
    }
    let mut raw = key_raw(Key::ArrowDown, true);
    keyboard.take_keys(&ctx, &mut raw);
    assert!(raw.events.is_empty());
    bare_frame(&ctx, egui::RawInput::default(), Some("text"));
    bare_frame(&ctx, egui::RawInput::default(), None);
    let mut raw = key_raw(Key::ArrowDown, false);
    keyboard.take_keys(&ctx, &mut raw);
    let layer_release = raw.events.clone();
    assert!(layer_release.is_empty(), "egui had the layer's release");
    // A system repeat goes with its press: egui's while the field has the keyboard.
    let mut raw = key_raw(Key::Tab, true);
    if let Event::Key { repeat, .. } = &mut raw.events[0] {
        *repeat = true;
    }
    keyboard.take_keys(&ctx, &mut raw);
    assert_eq!(raw.events.len(), 1);
    // Two of the layer's keys down, nothing focused: releasing one leaves the other's release the layer's.
    if let Some(id) = ctx.memory(|m| m.focused()) {
        ctx.memory_mut(|m| m.surrender_focus(id));
    }
    for key in [Key::ArrowUp, Key::Enter] {
        let mut raw = key_raw(key, true);
        keyboard.take_keys(&ctx, &mut raw);
    }
    let mut raw = key_raw(Key::ArrowUp, false);
    raw.events.extend(key_raw(Key::Enter, false).events);
    keyboard.take_keys(&ctx, &mut raw);
    assert!(
        raw.events.is_empty(),
        "a release of the layer's left to egui: {:?}",
        raw.events
    );
    rejects("the layer's release left to egui", || {
        assert_eq!(layer_release.len(), 1)
    });
}

/// The focus after a Tab held for two seconds, frames 0.1 s apart, with `mid` played after the press.
fn tab_held_through(mid: Vec<Vec<Event>>) -> Vec<String> {
    let mut app = mock_app();
    let mut h = headless();
    h.set_frame_step(0.1);
    let key = |key, pressed| Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: NONE,
    };
    let _ = h.frame(&mut app, vec![key(Key::Tab, true)]);
    for events in mid {
        let _ = h.frame(&mut app, events);
    }
    for _ in 0..20 {
        let _ = h.frame(&mut app, Vec::new());
    }
    focus(&app).iter().map(|s| (*s).to_owned()).collect()
}

fn key_event(key: Key, pressed: bool) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: NONE,
    }
}

/// A held key stops repeating when the layer is hidden (F3) and when the window loses the focus, whose release
/// never comes.
#[test]
fn mock_keyboard_held_key_dropped_when_hidden_or_unfocused() {
    let only_top_bar = |path: &[String]| assert_eq!(path, ["top_bar"]);
    // Hidden: F3, the release while hidden, F3 again.
    let hidden = tab_held_through(vec![
        vec![key_event(Key::F3, true)],
        vec![key_event(Key::F3, false)],
        vec![key_event(Key::Tab, false)],
        vec![key_event(Key::F3, true)],
        vec![key_event(Key::F3, false)],
    ]);
    only_top_bar(&hidden);
    // Hidden with the key still held, released only after F3 shows the layer again.
    let held = tab_held_through(vec![
        vec![key_event(Key::F3, true)],
        vec![key_event(Key::F3, false)],
        vec![key_event(Key::F3, true)],
        vec![key_event(Key::F3, false)],
    ]);
    only_top_bar(&held);
    // The window loses the focus: no release comes.
    let unfocused = tab_held_through(vec![vec![Event::WindowFocused(false)]]);
    only_top_bar(&unfocused);
    // The press and the loss in one frame.
    let mut app = mock_app();
    let mut h = headless();
    h.set_frame_step(0.1);
    let _ = h.frame(
        &mut app,
        vec![key_event(Key::Tab, true), Event::WindowFocused(false)],
    );
    for _ in 0..20 {
        let _ = h.frame(&mut app, Vec::new());
    }
    assert_eq!(focus(&app), ["top_bar"]);
    // Control: held through nothing, the Tab repeats on.
    let repeating = tab_held_through(Vec::new());
    rejects("a held Tab that repeats", || only_top_bar(&repeating));
}

/// A frame the hidden layer does not run asks for no adjustment: the shown frame's, an arrow held on a value, is not
/// reported again once F3 hides the layer.
#[test]
fn mock_keyboard_hidden_layer_reports_no_adjustment() {
    let (mut app, mut h) = zoom_app();
    let _ = h.frame(&mut app, vec![key_event(Key::ArrowUp, true)]);
    let shown = app.keyboard.adjusted().len();
    assert_eq!(shown, 1);
    let check = |n: usize| assert_eq!(n, 0, "{n} adjustments reported with the layer hidden");
    let _ = h.frame(&mut app, vec![key_event(Key::F3, true)]);
    assert!(!app.shown);
    check(app.keyboard.adjusted().len());
    let _ = h.frame(&mut app, vec![key_event(Key::F3, false)]);
    check(app.keyboard.adjusted().len());
    rejects("the shown frame's adjustment", || check(shown));
}

// --- The `?` overlay takes the pointer --------------------------------------------------------------------------

/// While the overlay is open no click reaches beneath: a click outside its frame, on Stain or on the footer, closes
/// it and does nothing else; a click inside it changes nothing.
#[test]
fn mock_keyboard_overlay_blocks_clicks() {
    let mut app = mock_app();
    let mut h = headless();
    let footer = crate::capture::footer_point(Layout::new(h.screen(), PIXELS_PER_POINT).footer);
    let click_at = |h: &mut Headless, app: &mut crate::app::App<_>, pos| {
        for events in crate::capture::click(pos) {
            let _ = h.frame(app, events);
        }
    };
    press(&mut h, &mut app, Key::Questionmark, SHIFT);
    click(&mut h, &mut app, "Stain");
    let after_stain = (app.view.mode, app.keyboard.shortcuts_open);
    assert_eq!(after_stain, (Mode::Explore, false));
    press(&mut h, &mut app, Key::Questionmark, SHIFT);
    click_at(&mut h, &mut app, footer);
    assert_eq!(
        (app.console_open, app.keyboard.shortcuts_open),
        (false, false)
    );
    press(&mut h, &mut app, Key::Questionmark, SHIFT);
    let centre = h.screen().center();
    click_at(&mut h, &mut app, centre);
    assert!(
        app.keyboard.shortcuts_open,
        "a click on the overlay closed it"
    );
    // Esc still closes it, and then the clicks reach their controls again.
    press(&mut h, &mut app, Key::Escape, NONE);
    assert!(!app.keyboard.shortcuts_open);
    click_at(&mut h, &mut app, footer);
    assert!(
        app.console_open,
        "the footer took no click after the overlay closed"
    );
    click(&mut h, &mut app, "Stain");
    let unblocked = (app.view.mode, app.keyboard.shortcuts_open);
    rejects("a click that reached Stain", || {
        assert_eq!(unblocked, (Mode::Explore, false))
    });
}

/// A pointer press under the open overlay with its frame not yet drawn closes it; a release whose press the overlay
/// did not swallow is egui's.
#[test]
fn mock_keyboard_overlay_pointer_edges() {
    let ctx = egui::Context::default();
    let mut keyboard = crate::keyboard::Keyboard::new();
    let button = |pressed| Event::PointerButton {
        pos: egui::pos2(10.0, 10.0),
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: NONE,
    };
    // Closed: the pointer is egui's.
    let mut raw = egui::RawInput {
        events: vec![button(true), button(false)],
        ..Default::default()
    };
    keyboard.take_keys(&ctx, &mut raw);
    assert_eq!(raw.events.len(), 2);
    keyboard.shortcuts_open = true;
    // A release with no swallowed press is egui's; a press is swallowed, and with no frame drawn it is outside.
    let mut raw = egui::RawInput {
        events: vec![button(false), button(true)],
        ..Default::default()
    };
    keyboard.take_keys(&ctx, &mut raw);
    assert_eq!(raw.events, vec![button(false)]);
    assert!(!keyboard.shortcuts_open);
    // Its release follows it, though the overlay has closed.
    let mut raw = egui::RawInput {
        events: vec![button(false)],
        ..Default::default()
    };
    keyboard.take_keys(&ctx, &mut raw);
    let swallowed = raw.events.clone();
    assert!(swallowed.is_empty());
    // A press inside the drawn frame keeps it open.
    keyboard.shortcuts_open = true;
    keyboard.overlay_drawn(egui::Rect::from_min_max(
        egui::pos2(0.0, 0.0),
        egui::pos2(20.0, 20.0),
    ));
    let mut raw = egui::RawInput {
        events: vec![button(true)],
        ..Default::default()
    };
    keyboard.take_keys(&ctx, &mut raw);
    assert!(keyboard.shortcuts_open && raw.events.is_empty());
    rejects("a swallowed release passed on", || {
        assert_eq!(swallowed.len(), 1)
    });
}

/// A screen adds its own rows to the `?` overlay through its tree's registration; F3 is among the global rows.
#[test]
fn mock_keyboard_screen_adds_shortcut_rows() {
    let shown_rows = |register: bool| {
        let mut app = mock_app();
        if register {
            app.keyboard
                .tree_mut(Mode::Explore)
                .register_shortcut("+ / −", "zoom in / out");
        }
        let mut h = headless();
        press(&mut h, &mut app, Key::Questionmark, SHIFT);
        frame_texts(&mut h, &mut app)
    };
    let check = |texts: &[String]| {
        for name in ["+ / −", "zoom in / out", "F3", "hide / show the egui layer"] {
            assert!(texts.iter().any(|t| t == name), "no `{name}`");
        }
    };
    check(&shown_rows(true));
    let mut tree = ScopeTree::new();
    assert!(tree.shortcuts().is_empty());
    tree.register_shortcut("a", "b");
    assert_eq!(tree.shortcuts(), [("a", "b")]);
    let without = shown_rows(false);
    rejects("a row no screen registered", || check(&without));
}

/// Help's "Keys (?)" is enabled and opens the overlay (RQ-250).
#[test]
fn mock_keyboard_help_keys_opens_shortcuts() {
    let mut app = mock_app();
    let mut h = headless();
    click(&mut h, &mut app, "Help");
    click(&mut h, &mut app, KEYS_ENTRY);
    assert!(app.keyboard.shortcuts_open);
    check_overlay(&frame_texts(&mut h, &mut app));
    press(&mut h, &mut app, Key::Escape, NONE);
    assert!(!app.keyboard.shortcuts_open);
    rejects("a closed overlay", || assert!(app.keyboard.shortcuts_open));
}

/// With the layer hidden (F3), the keyboard takes no key.
#[test]
fn mock_keyboard_hidden_layer_takes_no_key() {
    let mut app = mock_app();
    let mut h = headless();
    press(&mut h, &mut app, Key::F3, NONE);
    assert!(!app.shown);
    press(&mut h, &mut app, Key::Tab, NONE);
    assert!(focus(&app).is_empty());
    press(&mut h, &mut app, Key::F3, NONE);
    press(&mut h, &mut app, Key::Tab, NONE);
    assert_eq!(focus(&app), ["top_bar"]);
    rejects("a key taken while hidden", || {
        assert!(focus(&app).is_empty())
    });
}

// --- The registration each later screen uses -------------------------------------------------------------------------

#[test]
fn mock_keyboard_registration() {
    let mut tree = ScopeTree::new();
    let mut path = Vec::new();
    // An empty tree: nothing to move to.
    assert_eq!(tree.navigate(&mut path, Command::Next, 1.0), Outcome::None);
    assert_eq!(
        tree.navigate(&mut path, Command::Previous, 1.0),
        Outcome::None
    );
    assert_eq!(tree.navigate(&mut path, Command::Enter, 1.0), Outcome::None);
    assert_eq!(tree.navigate(&mut path, Command::Back, 1.0), Outcome::None);
    assert_eq!(
        tree.navigate(&mut path, Command::Arrow(Direction::Down), 1.0),
        Outcome::None
    );
    tree.register(None, Scope::group("a", "A"));
    tree.register(Some("a"), Scope::control("a1", "A1"));
    tree.register(None, Scope::group("b", "B").ring_outside());
    // A later screen's big scope joins the Tab order at the end.
    tree.register(None, Scope::group("later", "Later"));
    assert_eq!(tree.children(None), ["a", "b", "later"]);
    assert_eq!(tree.children(Some("a")), ["a1"]);
    assert_eq!(tree.children(Some("nowhere")), Vec::<&str>::new());
    assert_eq!(
        tree.get("b").map(|s| s.ring),
        Some(crate::keyboard::scopes::Ring::Outside)
    );
    let mut path = vec!["a".to_owned()];
    assert_eq!(
        tree.navigate(&mut path, Command::Enter, 1.0),
        Outcome::Moved
    );
    assert_eq!(
        tree.navigate(&mut path, Command::Enter, 1.0),
        Outcome::Activate("a1")
    );
    // Enter on a group with nothing under it does nothing.
    let mut path = vec!["b".to_owned()];
    assert_eq!(tree.navigate(&mut path, Command::Enter, 1.0), Outcome::None);
    assert_eq!(
        tree.navigate(&mut path, Command::Shortcuts, 1.0),
        Outcome::None
    );
    // A path the tree does not hold is cut back to its valid part.
    assert_eq!(tree.valid_prefix(&["a".into(), "a1".into()]), 2);
    assert_eq!(tree.valid_prefix(&["a".into(), "b".into()]), 1);
    assert_eq!(tree.valid_prefix(&["a1".into()]), 0);
    assert_eq!(tree.valid_prefix(&["x".into()]), 0);
    let mut stale = vec!["a".to_owned(), "zz".to_owned()];
    assert_eq!(
        tree.navigate(&mut stale, Command::Back, 1.0),
        Outcome::Moved
    );
    assert!(stale.is_empty());
    assert_eq!(tree.labels(&["a".into(), "a1".into()]), ["A", "A1"]);
    // Parents and paths; the menus.
    assert_eq!(tree.parent("a1"), Some("a"));
    assert_eq!(tree.parent("a"), None);
    assert_eq!(tree.parent("zz"), None);
    assert_eq!(tree.path_to("a1"), ["a", "a1"]);
    assert_eq!(tree.path_to("b"), ["b"]);
    assert_eq!(tree.path_to("zz"), Vec::<String>::new());
    assert!(tree.menus().is_empty());
    tree.register(Some("a"), Scope::menu("m", "M"));
    tree.register(Some("m"), Scope::disabled("m1", "M1"));
    assert_eq!(tree.menus(), ["m"]);
    let (menu, disabled) = (*tree.get("m").unwrap(), *tree.get("m1").unwrap());
    assert!(menu.menu && !menu.activates && !disabled.activates && !disabled.menu);
    assert!(!tree.get("a1").unwrap().menu);
    assert_eq!(tree.path_to("m1"), ["a", "m", "m1"]);
    // Enter on a big scope that is a control acts, the focus kept on it: no menu is open to close.
    let mut keyboard = crate::keyboard::Keyboard::new();
    keyboard
        .tree_mut(Mode::Explore)
        .register(None, Scope::control("big_control", "Big"));
    let mut big = Focus {
        path: vec!["big_control".to_owned()],
    };
    let ctx = egui::Context::default();
    let mut raw = egui::RawInput {
        events: crate::capture::key(Key::Enter, NONE)[0].clone(),
        ..Default::default()
    };
    keyboard.take_keys(&ctx, &mut raw);
    let activated = keyboard.run(&ctx, Mode::Explore, &mut big, 0.0);
    assert_eq!(activated, ["big_control"]);
    assert_eq!(big.path, ["big_control"]);
    let duplicate = std::panic::catch_unwind(|| {
        let mut t = ScopeTree::new();
        t.register(None, Scope::group("a", "A"));
        t.register(None, Scope::group("a", "A again"));
    });
    assert!(duplicate.is_err(), "a duplicate id was registered");
    let orphan = std::panic::catch_unwind(|| {
        let mut t = ScopeTree::new();
        t.register(Some("none"), Scope::group("a", "A"));
    });
    assert!(orphan.is_err(), "a scope registered under no parent");
    rejects("a later scope ahead of the others", || {
        assert_eq!(tree.children(None), ["later", "a", "b"])
    });
}

/// The focus lives in `ViewUI`: the app starts with none.
#[test]
fn mock_keyboard_focus_in_view_ui() {
    let app = mock_app();
    assert_eq!(app.view.focus, Focus { path: Vec::new() });
    assert_eq!(crate::app::initial_view().focus.path.len(), 0);
    rejects("a focus at start", || assert_eq!(focus(&app), ["top_bar"]));
}

// --- Before and after moving focus: only the ring and the breadcrumb change (REQ-GUI-098) ----------------------------

fn canvas() -> Arc<MockCanvas> {
    Arc::new(MockCanvas::new().expect("a GPU adapter for the mock's canvas"))
}

fn pixel(shot: &Shot, x: usize, y: usize) -> [u8; 4] {
    let i = (y * shot.size[0] as usize + x) * 4;
    shot.rgba[i..i + 4].try_into().unwrap()
}

/// A rect in points as whole pixels, grown by `grow` pixels: `[x0, y0, x1, y1)`.
fn px(r: egui::Rect, grow: f32) -> [f32; 4] {
    let p = PIXELS_PER_POINT;
    [
        r.min.x * p - grow,
        r.min.y * p - grow,
        r.max.x * p + grow,
        r.max.y * p + grow,
    ]
}

fn within(r: [f32; 4], x: usize, y: usize) -> bool {
    let (x, y) = (x as f32 + 0.5, y as f32 + 0.5);
    r[0] <= x && x < r[2] && r[1] <= y && y < r[3]
}

/// The ring's band on `rect`, drawn inside it: within the rect (and its anti-aliasing) and outside its inside less
/// the ring's width.
fn in_ring(rect: egui::Rect, x: usize, y: usize) -> bool {
    let outer = px(rect, 2.0);
    let inner = px(rect, -(RING_WIDTH * PIXELS_PER_POINT + 2.0));
    within(outer, x, y) && !within(inner, x, y)
}

/// The breadcrumb's names' rects, grown for anti-aliasing.
fn crumb_rects(names: &[Name]) -> Vec<[f32; 4]> {
    names
        .iter()
        .filter(|n| n.name == GLYPH || n.name.starts_with("Manifold view"))
        .filter_map(|n| n.rect)
        .map(|r| {
            [
                r[0] as f32 - 2.0,
                r[1] as f32 - 2.0,
                r[2] as f32 + 2.0,
                r[3] as f32 + 2.0,
            ]
        })
        .collect()
}

/// Every pixel that differs between `a` and `b` lies on one of `rings`' bands or on a breadcrumb; returns how many
/// differ on each ring.
fn check_only_ring_and_breadcrumb(a: &Shot, b: &Shot, rings: &[egui::Rect]) -> Vec<usize> {
    let mut crumbs = crumb_rects(&a.names);
    crumbs.extend(crumb_rects(&b.names));
    let mut on_ring = vec![0; rings.len()];
    let [w, h] = SIZE.map(|v| v as usize);
    for y in 0..h {
        for x in 0..w {
            if pixel(a, x, y) == pixel(b, x, y) {
                continue;
            }
            let mut ok = crumbs.iter().any(|r| within(*r, x, y));
            for (i, ring) in rings.iter().enumerate() {
                if in_ring(*ring, x, y) {
                    on_ring[i] += 1;
                    ok = true;
                }
            }
            assert!(ok, "({x}, {y}) differs off the ring and the breadcrumb");
        }
    }
    on_ring
}

#[test]
fn mock_keyboard_focus_moves_only_ring_and_breadcrumb() {
    let canvas = canvas();
    let layout = Layout::new(
        Headless::new(SIZE, PIXELS_PER_POINT).screen(),
        PIXELS_PER_POINT,
    );
    let navigate = crate::explore::manifold_sections(layout.manifold_view)[1];
    let before = shoot(canvas.clone(), &[Step::Tab, Step::Tab]);
    let after = shoot(
        canvas.clone(),
        &[Step::Tab, Step::Tab, Step::Enter, Step::ArrowDown],
    );
    let names = |s: &Shot| s.names.iter().map(|n| n.name.clone()).collect::<Vec<_>>();
    assert!(names(&before).contains(&"Manifold view".to_owned()));
    assert!(names(&after).contains(&"Manifold view › Navigate".to_owned()));
    let on_ring =
        check_only_ring_and_breadcrumb(&before, &after, &[layout.manifold_view, navigate]);
    assert!(
        on_ring.iter().all(|n| *n > 0),
        "a ring did not change: {on_ring:?}"
    );
    // No focus at all, as the shell's captures are, against the figure focused: the figure's pixels never change, its
    // ring drawn outside it.
    let none = shoot(canvas.clone(), &[]);
    let figure = shoot(canvas.clone(), &[Step::Tab, Step::Tab, Step::Tab]);
    let f = px(layout.figure, 0.0);
    for y in f[1] as usize..f[3] as usize {
        for x in f[0] as usize..f[2] as usize {
            assert_eq!(
                pixel(&none, x, y),
                pixel(&figure, x, y),
                "the ring covers the figure at ({x}, {y})"
            );
        }
    }
    let shortcuts = shoot(canvas, &[Step::Tab, Step::Tab, Step::Shortcuts]);
    rejects("the shortcuts overlay, which changes more", || {
        check_only_ring_and_breadcrumb(&before, &shortcuts, &[layout.manifold_view]);
    });
}

/// The capture mode's steps, the keyboard's among them, read from their names; an unknown name is refused, naming
/// the list.
fn check_steps_named(names: &[(&str, Step)]) {
    for (name, step) in names {
        assert_eq!(Step::parse(name), Ok(*step), "the step `{name}`");
    }
}

#[test]
fn mock_keyboard_capture_steps() {
    check_steps_named(&STEPS);
    let names: Vec<&str> = STEPS.iter().map(|(n, _)| *n).collect();
    assert_eq!(
        names,
        [
            "f3",
            "raise_warning",
            "raise_error",
            "click_footer",
            "tab",
            "shift_tab",
            "enter",
            "escape",
            "arrow_up",
            "arrow_down",
            "arrow_left",
            "arrow_right",
            "shortcuts"
        ]
    );
    let err = Step::parse("zoom").expect_err("an unknown step");
    assert!(
        err.contains("no step `zoom`") && err.contains("shift_tab, enter"),
        "{err}"
    );
    assert_eq!(
        Step::parse_list("tab,tab,enter"),
        Ok(vec![Step::Tab, Step::Tab, Step::Enter])
    );
    rejects("a step under another's name", || {
        check_steps_named(&[("tab", Step::ShiftTab)])
    });
}

// --- What the frame loop and the layout rely on ----------------------------------------------------------------------

/// The repaint the app asks for while a key is held: the held key's next repeat, `delay` seconds after the frame.
fn check_repaint(delays: &[f64], want: &[f64]) {
    assert_eq!(delays.len(), want.len());
    for (got, want) in delays.iter().zip(want) {
        assert!(
            (got - want).abs() < 1e-6,
            "repaint after {got} s, not {want} s"
        );
    }
}

#[test]
fn mock_keyboard_held_key_asks_for_its_repeat() {
    let mut app = mock_app();
    let mut h = headless();
    // Ten frames first, so the press falls at 1.25 s of input time.
    for _ in 0..10 {
        let _ = h.frame(&mut app, Vec::new());
    }
    let down = Event::Key {
        key: Key::ArrowDown,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: NONE,
    };
    let delay = |output: &egui::FullOutput| {
        output.viewport_output[&egui::ViewportId::ROOT]
            .repaint_delay
            .as_secs_f64()
    };
    let pressed = h.frame(&mut app, vec![down]);
    let held = h.frame(&mut app, Vec::new());
    let delays = [delay(&pressed), delay(&held)];
    // The repeat falls due at 1.75 s: 0.5 s after the press, 0.375 s after the next frame, less the 1/60 s egui takes
    // off each repaint delay for the frame's own time.
    let frame = 1.0 / 60.0;
    check_repaint(&delays, &[0.5 - frame, 0.375 - frame]);
    assert_eq!(app.keyboard.next_repeat_s(), Some(1.75));
    rejects("a repaint at the press's time", || {
        check_repaint(&delays, &[0.0, 0.0])
    });
}

/// eframe's hook before each pass hands the raw input to the keyboard, which takes its keys out of it.
#[test]
fn mock_keyboard_eframe_hook_takes_the_keys() {
    let mut app = mock_app();
    let h = headless();
    let tab = Event::Key {
        key: Key::Tab,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: NONE,
    };
    let mut raw = egui::RawInput {
        events: vec![tab.clone(), Event::Text("a".into())],
        ..Default::default()
    };
    <crate::app::App<_> as eframe::App>::raw_input_hook(&mut app, &h.ctx, &mut raw);
    assert_eq!(raw.events, vec![Event::Text("a".into())]);
    rejects("the Tab left to egui", || {
        assert_eq!(raw.events, vec![tab.clone(), Event::Text("a".into())])
    });
}

/// The Manifold view's four sections: the panel inside a 12-point margin, below a 40-point title, split evenly with
/// 8-point gaps.
#[test]
fn mock_keyboard_manifold_sections() {
    let panel = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(360.0, 640.0));
    let got = crate::explore::manifold_sections(panel);
    let want = [52.0, 198.0, 344.0, 490.0]
        .map(|y| egui::Rect::from_min_size(egui::pos2(12.0, y), egui::vec2(336.0, 138.0)));
    assert_eq!(got, want);
    let moved = egui::Rect::from_min_max(egui::pos2(100.0, 50.0), egui::pos2(460.0, 690.0));
    assert_eq!(
        crate::explore::manifold_sections(moved),
        want.map(|r| r.translate(egui::vec2(100.0, 50.0)))
    );
    rejects("sections without their gaps", || {
        assert_eq!(got[1].min.y, 190.0)
    });
}

/// Each frame's places are its own: after the switch to the Stain page, Explore's scopes are placed nowhere.
#[test]
fn mock_keyboard_places_are_the_frames() {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    assert!(app.keyboard.place_of("figure").is_some());
    click(&mut h, &mut app, "Stain");
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(app.keyboard.place_of("figure"), None);
    assert!(app.keyboard.place_of("stain").is_some());
    rejects("a stale place", || {
        assert!(app.keyboard.place_of("figure").is_some())
    });
}

/// Enter on a control acts as its click: the mode switch and the menus' entries act; a menu, a disabled control or
/// entry, or an unknown id does nothing.
#[test]
fn mock_keyboard_activate_acts_as_the_click() {
    use crate::app::Actions;
    use crate::explore::top_bar::{activate, HIDE, KEYS, QUIT};
    let acts = |id: &str| {
        let mut actions = Actions::default();
        activate(id, &mut actions);
        actions
    };
    let cases = [
        (
            "explore",
            Actions {
                mode: Some(Mode::Explore),
                ..Default::default()
            },
        ),
        (
            "stain",
            Actions {
                mode: Some(Mode::Stain),
                ..Default::default()
            },
        ),
        (
            QUIT.0,
            Actions {
                quit: true,
                ..Default::default()
            },
        ),
        (
            HIDE.0,
            Actions {
                toggle_layer: true,
                ..Default::default()
            },
        ),
        (
            KEYS.0,
            Actions {
                shortcuts: true,
                ..Default::default()
            },
        ),
    ];
    for (id, want) in cases {
        assert_eq!(acts(id), want, "{id}");
    }
    for id in ["file", "help", "run", "windows_run", "nowhere"] {
        assert_eq!(acts(id), Actions::default(), "{id}");
    }
    // The top bar's scopes: the menus, the disabled controls and entries that Enter does not activate.
    let mut tree = ScopeTree::new();
    crate::explore::top_bar::register(&mut tree);
    assert_eq!(
        tree.menus(),
        ["file", "view", "windows", "overlays", "help"]
    );
    for id in [
        "run",
        "profiler",
        "export",
        "windows_run",
        "windows_console",
    ] {
        assert!(!tree.get(id).unwrap().activates, "{id} activates");
    }
    for id in ["explore", "stain", QUIT.0, HIDE.0, KEYS.0] {
        assert!(tree.get(id).unwrap().activates, "{id} does not activate");
    }
    assert_eq!(tree.children(Some("overlays")), Vec::<&str>::new());
    let quit = acts(QUIT.0);
    rejects("Quit that did nothing", || {
        assert_eq!(quit, Actions::default())
    });
}
