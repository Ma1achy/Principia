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

/// Enter on a menu opens it; the menu then has the keys (Esc closes it, Tab moves nothing); a disabled control does
/// nothing.
#[test]
fn mock_keyboard_enter_opens_a_menu() {
    let mut app = mock_app();
    let mut h = headless();
    keys(&mut h, &mut app, &[(Key::Tab, NONE), (Key::Enter, NONE)]);
    assert!(!frame_texts(&mut h, &mut app).contains(&"Quit".to_owned()));
    press(&mut h, &mut app, Key::Enter, NONE);
    assert!(
        frame_texts(&mut h, &mut app).contains(&"Quit".to_owned()),
        "Enter on File opened no menu"
    );
    press(&mut h, &mut app, Key::Tab, NONE);
    assert_eq!(
        focus(&app),
        ["top_bar", "file"],
        "Tab moved the focus under a menu"
    );
    press(&mut h, &mut app, Key::Escape, NONE);
    assert!(!frame_texts(&mut h, &mut app).contains(&"Quit".to_owned()));
    assert_eq!(focus(&app), ["top_bar", "file"]);
    // Run… is disabled: Enter does nothing.
    for _ in 0..4 {
        press(&mut h, &mut app, Key::ArrowRight, NONE);
    }
    assert_eq!(focus(&app), ["top_bar", "run"]);
    press(&mut h, &mut app, Key::Enter, NONE);
    assert!(!egui::Popup::is_any_open(&h.ctx));
    press(&mut h, &mut app, Key::Escape, NONE);
    assert_eq!(focus(&app), ["top_bar"]);
    rejects("an open menu", || {
        assert!(frame_texts(&mut h, &mut app).contains(&"Quit".to_owned()))
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

/// Enter on a control that is not a menu opens no popup, though it is a widget with an id.
#[test]
fn mock_keyboard_activate_opens_only_menus() {
    let ctx = egui::Context::default();
    let widget = egui::Id::new("a control");
    let place = Some(crate::keyboard::Place {
        rect: egui::Rect::ZERO,
        widget: Some(widget),
    });
    let mut actions = crate::app::Actions::default();
    for id in ["run", "profiler", "export"] {
        crate::explore::top_bar::activate(&ctx, id, place, &mut actions);
        assert!(
            !egui::Popup::is_id_open(&ctx, widget.with("popup")),
            "{id} opened a popup"
        );
    }
    assert_eq!(actions, crate::app::Actions::default());
    crate::explore::top_bar::activate(&ctx, "help", place, &mut actions);
    assert!(egui::Popup::is_id_open(&ctx, widget.with("popup")));
    // A menu drawn as no widget opens nothing.
    let ctx = egui::Context::default();
    crate::explore::top_bar::activate(&ctx, "file", None, &mut actions);
    assert!(!egui::Popup::is_any_open(&ctx));
    rejects("a control that opened a popup", || {
        let ctx = egui::Context::default();
        crate::explore::top_bar::activate(&ctx, "file", place, &mut actions);
        assert!(!egui::Popup::is_id_open(&ctx, widget.with("popup")));
    });
}
