//! QA tests for TASK-M6-25, written from the requirements and render_gui_spec §G3, not from the implementation:
//!
//! - REQ-GUI-095: "The GUI must be a tree of keyboard scopes with Tab order 1 top bar · 2 Manifold view · 3 Figure ·
//!   4 Trajectory · 5 Compass · 6 Time · 7 Legend, with Manifold view's sub-scopes (Chart, Navigate, Centre z₀,
//!   Slice & tilt) reached with Enter"; verify: "Tab / Shift+Tab cycles the seven scopes in order; Enter on Manifold
//!   view enters Chart".
//! - REQ-GUI-169: on the mock, Tab and Shift+Tab between the big scopes, Enter into a scope and Esc out of it, arrows
//!   between siblings and on a focused value, Shift ×10 and Alt ×0.1 steps, the focus ring, the breadcrumb, `?`
//!   showing the shortcuts over everything (Esc closes it), and each later screen joining the scope tree.
//! - REQ-GUI-098: "Keyboard focus must be shown only as a focus ring on the current scope and a top-bar breadcrumb
//!   (e.g. 'Manifold view › Navigate › zoom'), with nothing else changing on screen"; verify: screenshots before and
//!   after moving focus differ only in the ring and the breadcrumb.
//! - REQ-GUI-146, REQ-GUI-158 (calibrations, proposed values pending the human at the M8 gate, R-71 / R-182): the
//!   tests check that the code honours the declared delay, rate and base steps — whatever they are — not the values.
//!
//! Scope labels are §G3's words; the breadcrumb is read from what the app draws (its AccessKit names), the focus from
//! `ViewUI` (gui_state_contract §2). gui takes no dependency on validation (R-187), so each check runs its own
//! control (R-176) in the test, through [`rejects`].
// The file name `qa_TASK-M6-25` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::sync::Arc;

use eframe::egui::{self, Event, Key, Modifiers};
use engine::contract::view_ui::Mode;
use gui::app::App;
use gui::capture::{self, Shot, Step, PIXELS_PER_POINT, SIZE};
use gui::headless::{Headless, Name};
use gui::keyboard::repeat::{DELAY_MS, INTERVAL_MS};
use gui::keyboard::scopes::{Base, Scope, StepKind};
use gui::layout::Layout;
use gui::mock::canvas::MockCanvas;
use gui::mock::MockEngine;
use gui::side::MockSide;

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const NONE: Modifiers = Modifiers::NONE;
const SHIFT: Modifiers = Modifiers::SHIFT;
const ALT: Modifiers = Modifiers::ALT;

/// §G3's big scopes, in Tab order.
const BIG: [&str; 7] = [
    "top bar",
    "Manifold view",
    "Figure",
    "Trajectory",
    "Compass",
    "Time",
    "Legend",
];
/// §G3's Manifold view sub-scopes, in order.
const MANIFOLD: [&str; 4] = ["Chart", "Navigate", "Centre z₀", "Slice & tilt"];
/// §G3's breadcrumb separator, from its example "Manifold view › Navigate › zoom".
const SEP: &str = " › ";

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

fn same_label(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

fn check_path(got: &[String], want: &[&str], what: &str) {
    assert!(
        got.len() == want.len() && got.iter().zip(want).all(|(g, w)| same_label(g, w)),
        "{what}: the focus is {got:?}, §G3 gives {want:?}"
    );
}

fn layout() -> Layout {
    Layout::new(headless().screen(), PIXELS_PER_POINT)
}

fn px(r: egui::Rect) -> [f64; 4] {
    let p = f64::from(PIXELS_PER_POINT);
    [
        f64::from(r.min.x) * p,
        f64::from(r.min.y) * p,
        f64::from(r.max.x) * p,
        f64::from(r.max.y) * p,
    ]
}

fn names(h: &mut Headless, app: &mut App<MockSide>) -> Vec<Name> {
    let out = h.frame(app, Vec::new());
    h.names(&out)
}

/// The breadcrumb drawn in the top bar: a name within the top bar's rect that reads `want`.
fn check_breadcrumb(names: &[Name], top_bar: [f64; 4], want: &str) {
    let found = names.iter().any(|n| {
        n.name.contains(want)
            && n.rect.is_some_and(|r| {
                r[1] >= top_bar[1] - 0.5 && r[3] <= top_bar[3] + 0.5 && r[0] >= top_bar[0] - 0.5
            })
    });
    assert!(found, "no breadcrumb `{want}` drawn in the top bar");
}

// --- REQ-GUI-095: the seven big scopes, in order; Enter on Manifold view enters Chart ---------------------------------

/// The focus after each of `n` presses of `key` with `modifiers`, from no focus.
fn cycle(key: Key, modifiers: Modifiers, n: usize) -> Vec<Vec<String>> {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    (0..n)
        .map(|_| {
            press(&mut h, &mut app, key, modifiers);
            focus_labels(&app)
        })
        .collect()
}

fn check_cycle(seen: &[Vec<String>], order: &[&str], what: &str) {
    for (i, path) in seen.iter().enumerate() {
        check_path(
            path,
            &[order[i % order.len()]],
            &format!("{what}, press {}", i + 1),
        );
    }
}

#[test]
fn qa_scope_tab_order_mock_seven_big_scopes_cycle() {
    // Two full turns: Tab wraps from Legend to the top bar.
    let tabs = cycle(Key::Tab, NONE, 14);
    check_cycle(&tabs, &BIG, "Tab");
    // Shift+Tab: the same seven, backwards, wrapping too. From no focus it starts at Legend, the last.
    let mut back = BIG;
    back.reverse();
    let shift_tabs = cycle(Key::Tab, SHIFT, 14);
    check_cycle(&shift_tabs, &back, "Shift+Tab");
    // Controls: a swapped order, and Shift+Tab read as Tab.
    let mut swapped = BIG;
    swapped.swap(2, 3);
    rejects("Figure and Trajectory swapped", || {
        check_cycle(&tabs, &swapped, "Tab")
    });
    rejects("Shift+Tab against the forward order", || {
        check_cycle(&shift_tabs, &BIG, "Shift+Tab")
    });
}

#[test]
fn qa_scope_tab_order_mock_shift_tab_undoes_tab() {
    let mut app = mock_app();
    let mut h = headless();
    for _ in 0..4 {
        press(&mut h, &mut app, Key::Tab, NONE);
    }
    check_path(&focus_labels(&app), &["Trajectory"], "four Tabs");
    press(&mut h, &mut app, Key::Tab, SHIFT);
    check_path(&focus_labels(&app), &["Figure"], "then Shift+Tab");
    rejects("a focus left on Trajectory", || {
        check_path(&focus_labels(&app), &["Trajectory"], "then Shift+Tab")
    });
}

#[test]
fn qa_scope_tab_order_mock_enter_on_manifold_view_enters_chart() {
    let mut app = mock_app();
    let mut h = headless();
    press(&mut h, &mut app, Key::Tab, NONE);
    press(&mut h, &mut app, Key::Tab, NONE);
    check_path(&focus_labels(&app), &["Manifold view"], "two Tabs");
    press(&mut h, &mut app, Key::Enter, NONE);
    let path = focus_labels(&app);
    check_path(&path, &["Manifold view", "Chart"], "Enter on Manifold view");
    rejects("Enter landing on Navigate", || {
        check_path(&path, &["Manifold view", "Navigate"], "Enter")
    });
    // The four sub-scopes, in §G3's order, are the siblings the arrows walk.
    let mut seen = vec![path];
    for _ in 0..3 {
        press(&mut h, &mut app, Key::ArrowDown, NONE);
        seen.push(focus_labels(&app));
    }
    for (path, sub) in seen.iter().zip(MANIFOLD) {
        check_path(path, &["Manifold view", sub], "↓ through the sub-scopes");
    }
    // The tree holds exactly these four under Manifold view.
    let tree = app.keyboard.tree(Mode::Explore);
    let bigs = tree.children(None);
    let mv = bigs
        .iter()
        .find(|id| {
            tree.get(id)
                .is_some_and(|s| same_label(s.label, "Manifold view"))
        })
        .expect("a Manifold view scope");
    let subs: Vec<&str> = tree
        .children(Some(mv))
        .iter()
        .map(|id| tree.get(id).unwrap().label)
        .collect();
    assert_eq!(subs, MANIFOLD, "Manifold view's sub-scopes");
    let labels: Vec<&str> = bigs.iter().map(|id| tree.get(id).unwrap().label).collect();
    assert!(
        labels.len() == 7 && labels.iter().zip(BIG).all(|(a, b)| same_label(a, b)),
        "the big scopes are {labels:?}"
    );
}

// --- REQ-GUI-169: Enter, Esc, arrows between siblings, Tab out of a sub-scope --------------------------------------------

#[test]
fn qa_mock_keyboard_esc_backs_out_one_level() {
    let mut app = mock_app();
    let mut h = headless();
    for (k, m) in [
        (Key::Tab, NONE),
        (Key::Tab, NONE),
        (Key::Enter, NONE),
        (Key::ArrowDown, NONE),
    ] {
        press(&mut h, &mut app, k, m);
    }
    check_path(
        &focus_labels(&app),
        &["Manifold view", "Navigate"],
        "Enter, ↓",
    );
    press(&mut h, &mut app, Key::Escape, NONE);
    let one = focus_labels(&app);
    check_path(&one, &["Manifold view"], "Esc: one level out");
    rejects("Esc going all the way out", || {
        check_path(&[], &["Manifold view"], "Esc")
    });
    // ↑ from Navigate goes back to Chart, its previous sibling.
    press(&mut h, &mut app, Key::Enter, NONE);
    press(&mut h, &mut app, Key::ArrowDown, NONE);
    press(&mut h, &mut app, Key::ArrowUp, NONE);
    check_path(&focus_labels(&app), &["Manifold view", "Chart"], "↓ then ↑");
}

#[test]
fn qa_mock_keyboard_tab_from_a_sub_scope_goes_to_the_next_big_scope() {
    let mut app = mock_app();
    let mut h = headless();
    for (k, m) in [
        (Key::Tab, NONE),
        (Key::Tab, NONE),
        (Key::Enter, NONE),
        (Key::ArrowDown, NONE),
        (Key::Tab, NONE),
    ] {
        press(&mut h, &mut app, k, m);
    }
    let path = focus_labels(&app);
    check_path(&path, &["Figure"], "Tab from Manifold view › Navigate");
    rejects("a Tab that stays among the sub-scopes", || {
        check_path(&path, &["Manifold view", "Centre z₀"], "Tab")
    });
}

#[test]
fn qa_mock_keyboard_focus_is_view_ui_state() {
    // The scope lives in ViewUI: starting from a focus set there, the keys move on from it.
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    let tree = app.keyboard.tree(Mode::Explore);
    let bigs: Vec<String> = tree.children(None).iter().map(|s| s.to_string()).collect();
    app.view.focus.path = vec![bigs[4].clone()]; // Compass
    press(&mut h, &mut app, Key::Tab, NONE);
    check_path(
        &focus_labels(&app),
        &["Time"],
        "Tab after Compass set in ViewUI",
    );
}

// --- REQ-GUI-169 / REQ-GUI-158: a value joined to the tree; arrows adjust it, Shift ×10, Alt ×0.1 ------------------------

/// An app whose Explore tree has a value `zoom` joined under Navigate, as a later screen joins (§G3's example
/// "Manifold view › Navigate › zoom"), focused.
fn zoom_app(kind: StepKind) -> (App<MockSide>, Headless) {
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
    tree.register(Some(navigate), Scope::value("qa_zoom", "zoom", kind));
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
    (app, h)
}

/// The step one press of `key` with `modifiers` asks of the focused value, read in the press's own frame.
fn step_of(h: &mut Headless, app: &mut App<MockSide>, key: Key, modifiers: Modifiers) -> Vec<Base> {
    let _ = h.frame(app, vec![key_event(key, true, modifiers)]);
    let got = app.keyboard.adjusted().iter().map(|a| a.delta()).collect();
    let _ = h.frame(app, vec![key_event(key, false, modifiers)]);
    got
}

fn value(b: Base) -> (u8, f64) {
    match b {
        Base::Degrees(v) => (0, v),
        Base::Log2(v) => (1, v),
        Base::OfRange(v) => (2, v),
        Base::OfView(v) => (3, v),
    }
}

/// `got` is one step, `times` the declared base step of `kind`.
fn check_step(got: &[Base], kind: StepKind, times: f64, what: &str) {
    assert_eq!(got.len(), 1, "{what}: {got:?} is not one step");
    let (unit, v) = value(got[0]);
    let (want_unit, base) = value(kind.base());
    assert_eq!(unit, want_unit, "{what}: the step's unit");
    assert!(
        (v - base * times).abs() <= 1e-12 * base.abs().max(1.0),
        "{what}: the step is {v}, the base {base} × {times} is {}",
        base * times
    );
}

#[test]
fn qa_mock_keyboard_arrows_adjust_a_focused_value_shift_x10_alt_x0_1() {
    let kinds = [
        StepKind::Bounded,
        StepKind::Angle,
        StepKind::ZoomLog2,
        StepKind::Pan,
        StepKind::Tilt,
        StepKind::Orbit,
        StepKind::TimeStep,
    ];
    for kind in kinds {
        let (mut app, mut h) = zoom_app(kind);
        check_path(
            &focus_labels(&app),
            &["Manifold view", "Navigate", "zoom"],
            "Enter on Navigate",
        );
        let what = format!("{kind:?}");
        let up = step_of(&mut h, &mut app, Key::ArrowUp, NONE);
        check_step(&up, kind, 1.0, &format!("{what} ↑"));
        let right = step_of(&mut h, &mut app, Key::ArrowRight, NONE);
        check_step(&right, kind, 1.0, &format!("{what} →"));
        let down = step_of(&mut h, &mut app, Key::ArrowDown, NONE);
        check_step(&down, kind, -1.0, &format!("{what} ↓"));
        let shift = step_of(&mut h, &mut app, Key::ArrowUp, SHIFT);
        check_step(&shift, kind, 10.0, &format!("{what} Shift+↑"));
        let alt = step_of(&mut h, &mut app, Key::ArrowLeft, ALT);
        check_step(&alt, kind, -0.1, &format!("{what} Alt+←"));
        // Adjusting a value does not move the focus.
        check_path(
            &focus_labels(&app),
            &["Manifold view", "Navigate", "zoom"],
            "after the arrows",
        );
        // Controls: Shift read as ×1, Alt as ×10, a sign flipped.
        rejects("Shift's step taken as the base", || {
            check_step(&shift, kind, 1.0, "Shift")
        });
        rejects("Alt's step taken as ×10", || {
            check_step(&alt, kind, -10.0, "Alt")
        });
        rejects("↓ taken as raising", || check_step(&down, kind, 1.0, "↓"));
    }
}

#[test]
fn qa_mock_keyboard_breadcrumb_reads_the_spec_example() {
    let (mut app, mut h) = zoom_app(StepKind::ZoomLog2);
    let n = names(&mut h, &mut app);
    let top = px(layout().top_bar);
    let want = ["Manifold view", "Navigate", "zoom"].join(SEP);
    check_breadcrumb(&n, top, &want);
    rejects("a breadcrumb one level short", || {
        check_breadcrumb(&n, top, &format!("Manifold view{SEP}Chart{SEP}zoom"))
    });
    // Esc twice: the breadcrumb follows, one level each.
    press(&mut h, &mut app, Key::Escape, NONE);
    press(&mut h, &mut app, Key::Escape, NONE);
    let n = names(&mut h, &mut app);
    check_breadcrumb(&n, top, "Manifold view");
    let still_deep = n.iter().any(|x| x.name.contains(&want));
    assert!(
        !still_deep,
        "the breadcrumb still reads `{want}` after two Esc"
    );
}

// --- REQ-GUI-169: each later screen joins the tree -----------------------------------------------------------------

#[test]
fn qa_mock_keyboard_a_later_screen_joins_the_tab_order() {
    let mut app = mock_app();
    app.keyboard
        .tree_mut(Mode::Explore)
        .register(None, Scope::group("qa_later", "Later"));
    let mut h = headless();
    let mut seen = Vec::new();
    for _ in 0..9 {
        press(&mut h, &mut app, Key::Tab, NONE);
        seen.push(focus_labels(&app));
    }
    let mut order: Vec<&str> = BIG.to_vec();
    order.push("Later");
    check_cycle(&seen, &order, "Tab with a screen joined");
    rejects("the joined screen left out", || {
        check_cycle(&seen, &BIG, "Tab")
    });
}

// --- REQ-GUI-169 / REQ-GUI-146: held keys delay, then repeat ---------------------------------------------------------

/// Holds ↑ on a focused value for `hold_ms`, frames `step_ms` apart; returns the adjustments seen by each frame's time
/// since the press, cumulative, then the count after the release and a further second.
fn hold(step_ms: u64, hold_ms: u64) -> (Vec<(u64, usize)>, usize) {
    let (mut app, mut h) = zoom_app(StepKind::Bounded);
    h.set_frame_step(step_ms as f64 / 1000.0);
    let mut total = 0;
    let mut seen = Vec::new();
    let _ = h.frame(&mut app, vec![key_event(Key::ArrowUp, true, NONE)]);
    total += app.keyboard.adjusted().len();
    seen.push((0, total));
    let mut t = 0;
    while t < hold_ms {
        t += step_ms;
        let _ = h.frame(&mut app, Vec::new());
        total += app.keyboard.adjusted().len();
        seen.push((t, total));
    }
    let _ = h.frame(&mut app, vec![key_event(Key::ArrowUp, false, NONE)]);
    total += app.keyboard.adjusted().len();
    let at_release = total;
    for _ in 0..(1000 / step_ms) {
        let _ = h.frame(&mut app, Vec::new());
        total += app.keyboard.adjusted().len();
    }
    assert_eq!(total, at_release, "the key repeats after its release");
    (seen, at_release)
}

/// The count by `t` ms after the press, as the DAS / ARR model gives it with the declared delay and interval: the
/// press, then one at the delay and one every interval after.
fn das_arr(t: u64) -> usize {
    1 + if t >= DELAY_MS {
        ((t - DELAY_MS) / INTERVAL_MS + 1) as usize
    } else {
        0
    }
}

fn check_hold(seen: &[(u64, usize)], what: &str) {
    for &(t, n) in seen {
        assert_eq!(
            n,
            das_arr(t),
            "{what}: {n} adjustments by {t} ms after the press"
        );
    }
}

#[test]
fn qa_mock_keyboard_held_key_delays_then_repeats_at_the_declared_rate() {
    // Frames 5 ms apart, a divisor of the declared delay and interval (checked), so every boundary falls on a frame.
    const {
        assert!(
            DELAY_MS > 0
                && INTERVAL_MS > 0
                && DELAY_MS.is_multiple_of(5)
                && INTERVAL_MS.is_multiple_of(5)
        )
    };
    let step = 5;
    let hold_ms = DELAY_MS + 10 * INTERVAL_MS + 3 * step;
    let (seen, _) = hold(step, hold_ms);
    check_hold(&seen, "held ↑");
    // Before the delay there is only the press.
    assert!(
        seen.iter()
            .filter(|(t, _)| *t < DELAY_MS)
            .all(|(_, n)| *n == 1),
        "a repeat before the delay"
    );
    // Controls: no delay (repeating at the rate from the press), and twice the rate.
    rejects("repeats from the press, with no delay", || {
        let shifted: Vec<(u64, usize)> = seen.iter().map(|&(t, n)| (t + DELAY_MS, n)).collect();
        check_hold(&shifted, "no delay")
    });
    rejects("a repeat at twice the rate", || {
        let doubled: Vec<(u64, usize)> = seen
            .iter()
            .map(|&(t, n)| (t, if t >= DELAY_MS { 2 * n - 1 } else { n }))
            .collect();
        check_hold(&doubled, "twice the rate")
    });
}

#[test]
fn qa_mock_keyboard_held_key_rate_independent_of_the_frame_rate() {
    // Frames slower than the interval: the repeats due catch up, so the count follows time, not frames.
    let step = INTERVAL_MS * 3 + 1;
    let hold_ms = DELAY_MS + 20 * INTERVAL_MS;
    let (seen, _) = hold(step, hold_ms);
    check_hold(&seen, "held ↑, slow frames");
}

#[test]
fn qa_mock_keyboard_held_tab_repeats_too() {
    // Held keys repeat their command: a held Tab walks the big scopes at the declared delay and rate.
    let mut app = mock_app();
    let mut h = headless();
    let step = 5;
    h.set_frame_step(step as f64 / 1000.0);
    let _ = h.frame(&mut app, Vec::new());
    let _ = h.frame(&mut app, vec![key_event(Key::Tab, true, NONE)]);
    let mut t = 0;
    let end = DELAY_MS + 2 * INTERVAL_MS;
    while t < end {
        t += step;
        let _ = h.frame(&mut app, Vec::new());
    }
    let _ = h.frame(&mut app, vec![key_event(Key::Tab, false, NONE)]);
    let presses = das_arr(end);
    check_path(
        &focus_labels(&app),
        &[BIG[(presses - 1) % 7]],
        "Tab held past the delay and two intervals",
    );
}

// --- REQ-GUI-169: `?` shows the shortcuts over everything; Esc closes it --------------------------------------------

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

fn check_shortcuts_shown(n: &[Name]) {
    for key in G3_KEYS {
        assert!(
            n.iter().any(|x| x.name == key),
            "the shortcuts do not show §G3's `{key}`"
        );
    }
}

#[test]
fn qa_mock_keyboard_question_mark_opens_shortcuts_esc_closes() {
    let mut app = mock_app();
    let mut h = headless();
    press(&mut h, &mut app, Key::Tab, NONE);
    press(&mut h, &mut app, Key::Tab, NONE);
    let closed = names(&mut h, &mut app);
    rejects("the shortcuts before `?`", || {
        check_shortcuts_shown(&closed)
    });
    press(&mut h, &mut app, Key::Questionmark, SHIFT);
    let open = names(&mut h, &mut app);
    check_shortcuts_shown(&open);
    press(&mut h, &mut app, Key::Escape, NONE);
    let after = names(&mut h, &mut app);
    rejects("the shortcuts after Esc", || check_shortcuts_shown(&after));
}

// --- REQ-GUI-098 and the `?` overlay on the GPU: what changes on screen ------------------------------------------------

fn canvas() -> Arc<MockCanvas> {
    Arc::new(MockCanvas::new().expect("a GPU adapter for the mock's canvas"))
}

fn pixel(shot: &Shot, x: usize, y: usize) -> [u8; 4] {
    let i = (y * shot.size[0] as usize + x) * 4;
    shot.rgba[i..i + 4].try_into().unwrap()
}

/// The pixels where `a` and `b` differ.
fn diff(a: &Shot, b: &Shot) -> Vec<(usize, usize)> {
    let (w, h) = (SIZE[0] as usize, SIZE[1] as usize);
    (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .filter(|&(x, y)| pixel(a, x, y) != pixel(b, x, y))
        .collect()
}

/// Whether pixel `(x, y)` is on the edge band of `r` (points), `band` pixels either side of its border: where a ring
/// on `r` is drawn, inside or outside.
fn on_edge(r: egui::Rect, band: f64, x: usize, y: usize) -> bool {
    let [x0, y0, x1, y1] = px(r);
    let (x, y) = (x as f64 + 0.5, y as f64 + 0.5);
    let outer = x >= x0 - band && x <= x1 + band && y >= y0 - band && y <= y1 + band;
    let inner = x > x0 + band && x < x1 - band && y > y0 + band && y < y1 - band;
    outer && !inner
}

/// The breadcrumb's area in pixels: the names in the top bar that hold the separator, a scope label or the keyboard
/// glyph 01_main.png draws beside the breadcrumb, in either shot.
fn crumb_areas(shots: &[&Shot], top: [f64; 4], labels: &[&str]) -> Vec<[f64; 4]> {
    shots
        .iter()
        .flat_map(|s| s.names.iter())
        .filter(|n| {
            labels.iter().any(|l| n.name.contains(l))
                || n.name.contains(SEP.trim())
                || n.name == gui::explore::breadcrumb::GLYPH
        })
        .filter_map(|n| n.rect)
        .filter(|r| r[3] <= top[3] + 0.5)
        .collect()
}

/// Every pixel that differs is on a ring band of `rings` or in a breadcrumb area; returns the count of each.
fn check_only_ring_and_crumb(
    a: &Shot,
    b: &Shot,
    rings: &[egui::Rect],
    crumbs: &[[f64; 4]],
) -> (usize, usize) {
    // The ring's band: a few points either side of the rect's border, wide enough for a stroke and its
    // anti-aliasing, at any ring width up to 4 points. Not a tolerance on the content, a region.
    let band = 6.0 * f64::from(PIXELS_PER_POINT);
    let (mut ring, mut crumb) = (0, 0);
    for (x, y) in diff(a, b) {
        if rings.iter().any(|r| on_edge(*r, band, x, y)) {
            ring += 1;
        } else if crumbs.iter().any(|c| {
            let (xf, yf) = (x as f64 + 0.5, y as f64 + 0.5);
            xf >= c[0] - 2.0 && xf <= c[2] + 2.0 && yf >= c[1] - 2.0 && yf <= c[3] + 2.0
        }) {
            crumb += 1;
        } else {
            panic!("the screen changed at ({x}, {y}), on neither a ring nor the breadcrumb");
        }
    }
    (ring, crumb)
}

#[test]
fn qa_capture_focus_moves_only_the_ring_and_the_breadcrumb() {
    let canvas = canvas();
    let l = layout();
    let top = px(l.top_bar);
    let before = capture::shoot(canvas.clone(), &[Step::Tab, Step::Tab]);
    let after = capture::shoot(
        canvas.clone(),
        &[Step::Tab, Step::Tab, Step::Enter, Step::ArrowDown],
    );
    // The rings that may move: on Manifold view, and on its sub-scope Navigate. The sub-scopes' rects are not given by
    // §G3, so every pixel inside Manifold view's rect but off its title is a candidate ring for a sub-scope; the
    // check below confines them to edge bands of the four sections the panel is split into.
    let sections = gui::explore::manifold_sections(l.manifold_view);
    let mut rings = vec![l.manifold_view];
    rings.extend(sections);
    let crumbs = crumb_areas(&[&before, &after], top, &["Manifold view", "Navigate"]);
    assert!(!crumbs.is_empty(), "no breadcrumb drawn in the top bar");
    let (ring, crumb) = check_only_ring_and_crumb(&before, &after, &rings, &crumbs);
    assert!(ring > 0, "the ring did not move");
    assert!(crumb > 0, "the breadcrumb did not change");
    // Nothing changed on the figure, the right panel, the bottom row or the footer.
    for (what, r) in [
        ("the figure", l.figure),
        ("the trajectory", l.trajectory),
        ("the bottom row", l.bottom_row),
        ("the footer", l.footer),
    ] {
        let [x0, y0, x1, y1] = px(r).map(|v| v.round() as usize);
        let changed = diff(&before, &after)
            .into_iter()
            .any(|(x, y)| x >= x0 && x < x1 && y >= y0 && y < y1);
        assert!(!changed, "{what} changed when the focus moved");
    }
    // Controls: the same check rejects a change elsewhere (the shortcuts overlay), and a ring with no breadcrumb.
    let overlay = capture::shoot(canvas.clone(), &[Step::Tab, Step::Tab, Step::Shortcuts]);
    rejects("the shortcuts overlay as a focus move", || {
        check_only_ring_and_crumb(&before, &overlay, &rings, &crumbs);
    });
    rejects("the breadcrumb's change with no breadcrumb area", || {
        check_only_ring_and_crumb(&before, &after, &rings, &[]);
    });
}

#[test]
fn qa_capture_figure_focus_ring_leaves_the_figure_untouched() {
    // Focus on Figure shows a ring; nothing is drawn over the figure itself (§G1, §G3's "nothing else changes").
    let canvas = canvas();
    let l = layout();
    let none = capture::shoot(canvas.clone(), &[]);
    let figure = capture::shoot(canvas.clone(), &[Step::Tab, Step::Tab, Step::Tab]);
    let [x0, y0, x1, y1] = px(l.figure).map(|v| v.round() as usize);
    let changes = diff(&none, &figure);
    let over = changes
        .iter()
        .find(|&&(x, y)| x >= x0 && x < x1 && y >= y0 && y < y1);
    assert!(
        over.is_none(),
        "the figure's pixels changed under its ring at {over:?}"
    );
    let near = changes
        .iter()
        .any(|&(x, y)| on_edge(l.figure, 6.0 * f64::from(PIXELS_PER_POINT), x, y));
    assert!(near, "no ring around the focused figure");
    // Control: the check over the figure fires on a change there (the shortcuts overlay sits over the centre).
    let overlay = capture::shoot(canvas.clone(), &[Step::Shortcuts]);
    let covered = diff(&none, &overlay)
        .into_iter()
        .any(|(x, y)| x >= x0 && x < x1 && y >= y0 && y < y1);
    assert!(
        covered,
        "the shortcuts are not over the figure: not over everything"
    );
}

#[test]
fn qa_capture_shortcuts_over_everything_then_esc_restores() {
    let canvas = canvas();
    let before = capture::shoot(canvas.clone(), &[Step::Tab, Step::Tab]);
    let open = capture::shoot(canvas.clone(), &[Step::Tab, Step::Tab, Step::Shortcuts]);
    let closed = capture::shoot(
        canvas.clone(),
        &[Step::Tab, Step::Tab, Step::Shortcuts, Step::Escape],
    );
    check_shortcuts_shown(&open.names);
    // Over everything: the window's centre, on the figure, shows the overlay.
    let (cx, cy) = (SIZE[0] as usize / 2, SIZE[1] as usize / 2);
    assert_ne!(
        pixel(&before, cx, cy),
        pixel(&open, cx, cy),
        "the window's centre is not covered by the shortcuts"
    );
    // Esc closes it: the screen is as before `?`, focus included (the overlay took the Esc).
    let left = diff(&before, &closed);
    assert!(
        left.is_empty(),
        "{} pixels still differ after Esc closed the shortcuts, first {:?}",
        left.len(),
        left.first()
    );
    rejects("the open overlay as closed", || {
        assert!(diff(&before, &open).is_empty())
    });
}
