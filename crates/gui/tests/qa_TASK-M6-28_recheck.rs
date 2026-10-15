//! QA tests for TASK-M6-28's re-check, written from the spec, not from the implementation:
//!
//! - render_gui_spec §G5 "Export & share": "a share link (`principia://view?…`) … Snapshot JSON, share links and
//!   pxpack carry `SimConfig` and `RenderState` in their one canonical serialisation (`principia_gui_state_contract.md`
//!   §2, R-309)". The link Copy puts on the clipboard decodes (RFC 3986 percent-decoding of its query) to the very JSON
//!   Copy snapshot puts there, which is JCS-canonical and carries the state's horizon and playhead.
//! - render_gui_spec §G5 "Run": "Every field is a `SimConfig` field (gui_state_contract §2)"; gui_state_contract §2:
//!   the horizon's `SetField` path is `SimField::Horizon`, undoable, `t` in `[50, 200]`, "the Run window's
//!   `T_horizon` sends it". The window opens on the state's `T`; Recompute sends the `T` the keys set; undo restores it.
//! - render_gui_spec §G4: "The lock badge offers **unlock** and **open in Inspector** (§G8)": a lock badge's entry,
//!   so it acts on a lock; Enter on it while free opens nothing, and while locked opens the Inspector.
//! - render_gui_spec §G2: "**Open full viewer…** and **IC Inspector…** (both open §G8)": Open full viewer… opens the
//!   Inspector, by click and by Enter, and Esc returns the focus to it (the window rule).
//! - REQ-GUI-174 ("feel and behave"): a window the wheel scrolls stays where the wheel left it, the focus unmoved; the
//!   focus moving later scrolls its element back into view.
//!
//! The focus is read from `ViewUI` by the tree's labels; the drawn is read from AccessKit names; the clipboard from
//! the platform output. gui takes no dependency on validation (R-187), so each control (R-176) runs in the test,
//! through [`rejects`].
// The file name `qa_TASK-M6-28_recheck` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use eframe::egui::{self, Event, Key, Modifiers, OutputCommand, PointerButton, Pos2};
use engine::contract::canonical;
use engine::contract::render_state::Playhead;
use engine::contract::set_field::{Edit, RenderField, SetField, SimField};
use engine::contract::sim_config::{Horizon, Lock};
use engine::contract::view_ui::{KeyMode, Window};
use gui::app::App;
use gui::capture::{PIXELS_PER_POINT, SIZE};
use gui::headless::{Headless, Name};
use gui::layout::Layout;
use gui::mock::MockEngine;
use gui::side::MockSide;

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

/// Presses and releases `key`, each its own frame, then one idle frame; returns what was copied in them.
fn press(h: &mut Headless, app: &mut App<MockSide>, key: Key, modifiers: Modifiers) -> Vec<String> {
    let mut copied = Vec::new();
    for events in [
        vec![key_event(key, true, modifiers)],
        vec![key_event(key, false, modifiers)],
        Vec::new(),
    ] {
        let out = h.frame(app, events);
        for c in out.platform_output.commands {
            if let OutputCommand::CopyText(t) = c {
                copied.push(t);
            }
        }
    }
    copied
}

fn settle(h: &mut Headless, app: &mut App<MockSide>) {
    for _ in 0..4 {
        let _ = h.frame(app, Vec::new());
    }
}

fn labels(app: &App<MockSide>) -> Vec<String> {
    app.keyboard
        .tree(app.view.mode)
        .labels(&app.view.focus.path)
        .into_iter()
        .map(str::to_owned)
        .collect()
}

fn same(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

fn check_path(got: &[String], want: &[&str], what: &str) {
    assert!(
        got.len() == want.len() && got.iter().zip(want).all(|(g, w)| same(g, w)),
        "{what}: the focus is {got:?}, the spec gives {want:?}"
    );
}

fn last_is(app: &App<MockSide>, label: &str) -> bool {
    labels(app).last().is_some_and(|l| same(l, label))
}

/// From where the focus is, moves among siblings (↓, →, ↑, then ←) until the last label is `label`.
fn sibling(h: &mut Headless, app: &mut App<MockSide>, label: &str) {
    for key in [
        Key::ArrowDown,
        Key::ArrowRight,
        Key::ArrowUp,
        Key::ArrowLeft,
    ] {
        for _ in 0..40 {
            if last_is(app, label) {
                return;
            }
            press(h, app, key, NONE);
        }
    }
    panic!("no sibling `{label}`: the focus is {:?}", labels(app));
}

/// From no focus, walks the keys to the scope at `path` (labels): Tab to its big scope, then Enter into each level and
/// the arrows along it. Every landing in navigation mode.
fn walk_to(h: &mut Headless, app: &mut App<MockSide>, path: &[&str]) {
    app.view.focus.path.clear();
    for _ in 0..10 {
        press(h, app, Key::Tab, NONE);
        if labels(app).first().is_some_and(|l| same(l, path[0])) {
            break;
        }
    }
    check_path(&labels(app), &path[..1], "Tab to the big scope");
    for depth in 1..path.len() {
        press(h, app, Key::Enter, NONE);
        sibling(h, app, path[depth]);
        check_path(&labels(app), &path[..=depth], "the walk");
    }
    assert_eq!(
        app.view.focus.mode, NAV,
        "the walk lands in navigation mode"
    );
}

fn names(h: &mut Headless, app: &mut App<MockSide>) -> Vec<Name> {
    let out = h.frame(app, Vec::new());
    h.names(&out)
}

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

/// Opens Windows › `entry` by the keys: the top bar's Windows, Enter, ↓ to the entry, Enter.
fn open_from_windows_menu(h: &mut Headless, app: &mut App<MockSide>, entry: &str) {
    walk_to(h, app, &["top bar", "Windows"]);
    press(h, app, Key::Enter, NONE);
    sibling(h, app, entry);
    check_path(
        &labels(app),
        &["top bar", "Windows", entry],
        "Windows › entry",
    );
    press(h, app, Key::Enter, NONE);
    settle(h, app);
}

/// Tab (or Shift+Tab) inside the open window until the focus is in its section `section`.
fn to_section(h: &mut Headless, app: &mut App<MockSide>, section: &str, modifiers: Modifiers) {
    for _ in 0..10 {
        if labels(app).get(1).is_some_and(|l| same(l, section)) {
            return;
        }
        press(h, app, Key::Tab, modifiers);
    }
    panic!("no section `{section}`: the focus is {:?}", labels(app));
}

/// Into the section the focus is on, if on it, and along to its element `label`.
fn to_element(h: &mut Headless, app: &mut App<MockSide>, label: &str) {
    if labels(app).len() == 2 {
        press(h, app, Key::Enter, NONE);
    }
    sibling(h, app, label);
}

// --- §G5 Export & share: the share link carries the canonical snapshot ------------------------------------------------

/// The value of RFC 3986 percent-decoding `text` (§2.1): `%XX` to its byte, every other byte as it is.
fn percent_decode(text: &str) -> Option<String> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let hex = std::str::from_utf8(b.get(i + 1..i + 3)?).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// The state a `principia://view?…` link carries, by its query's parameters: each value percent-decoded, and the one
/// that is a JSON object returned. Checks the link is a URI: no character outside RFC 3986's unreserved, reserved and
/// `%` (§2), so it survives a paste.
fn link_state(link: &str) -> serde_json::Value {
    let query = link
        .strip_prefix("principia://view?")
        .unwrap_or_else(|| panic!("not a principia://view? link: {link}"));
    assert!(
        query
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-._~:/?#[]@!$&'()*+,;=%".contains(c)),
        "the link has a character a URI may not carry: {link}"
    );
    query
        .split('&')
        .filter_map(|kv| kv.split_once('='))
        .filter_map(|(_, v)| percent_decode(v))
        .filter_map(|v| serde_json::from_str::<serde_json::Value>(&v).ok())
        .find(serde_json::Value::is_object)
        .unwrap_or_else(|| panic!("the link carries no state: {link}"))
}

/// The value of the first key `key` anywhere in `v`, depth first.
fn find<'a>(v: &'a serde_json::Value, key: &str) -> Option<&'a serde_json::Value> {
    match v {
        serde_json::Value::Object(m) => {
            m.get(key).or_else(|| m.values().find_map(|x| find(x, key)))
        }
        serde_json::Value::Array(a) => a.iter().find_map(|x| find(x, key)),
        _ => None,
    }
}

/// The share link carries `snapshot` (the Copy snapshot JSON) exactly, in its one canonical serialisation, and the
/// state's horizon `t` and playhead `t`.
fn check_link_carries(link: &str, snapshot: &str, horizon_t: f64, playhead_t: f64) {
    let state = link_state(link);
    let parsed: serde_json::Value = serde_json::from_str(snapshot).expect("the snapshot is JSON");
    assert_eq!(state, parsed, "the link's state is not the snapshot's");
    let text = serde_json::to_string(&state).unwrap();
    let decoded = link
        .split('&')
        .filter_map(|kv| kv.split_once('='))
        .filter_map(|(_, v)| percent_decode(v))
        .find(|v| serde_json::from_str::<serde_json::Value>(v).is_ok_and(|x| x == state))
        .unwrap();
    assert_eq!(
        decoded, snapshot,
        "the link's JSON is not the snapshot's text byte for byte"
    );
    assert_eq!(
        canonical::json_to_string(&state).expect("canonicalises"),
        decoded,
        "the link's JSON is not in its canonical (JCS) form: {text}"
    );
    let t_of = |k: &str| {
        find(&state, k)
            .and_then(|x| find(x, "t"))
            .and_then(serde_json::Value::as_f64)
    };
    assert_eq!(t_of("horizon"), Some(horizon_t), "the link's horizon");
    assert_eq!(t_of("playhead"), Some(playhead_t), "the link's playhead");
}

/// Copy snapshot (JSON) and Copy share link, by the keys in the open Export & share window: their clipboard texts.
fn copy_both(h: &mut Headless, app: &mut App<MockSide>) -> (String, String) {
    to_section(h, app, "State", NONE);
    to_element(h, app, "Copy snapshot (JSON)");
    let snapshot = press(h, app, Key::Enter, NONE);
    to_element(h, app, "Copy share link");
    let link = press(h, app, Key::Enter, NONE);
    assert_eq!(
        (snapshot.len(), link.len()),
        (1, 1),
        "one copy each: {snapshot:?} {link:?}"
    );
    (snapshot[0].clone(), link[0].clone())
}

#[test]
fn qa_mock_windows_share_link_carries_the_canonical_snapshot() {
    let (mut app, mut h) = fresh();
    // A state off the mock's start, so the link carries what was set, both a SimConfig and a RenderState field.
    app.set_field(SetField {
        edit: Edit::Sim(SimField::Horizon(Horizon { t: 137.5 })),
        no_history: false,
    });
    app.set_field(SetField {
        edit: Edit::Render(RenderField::Playhead(Playhead { t: 12.25 })),
        no_history: false,
    });
    settle(&mut h, &mut app);
    open_from_windows_menu(&mut h, &mut app, "Export & share");
    assert!(app.is_open(Window::Export));
    let (snapshot, link) = copy_both(&mut h, &mut app);
    check_link_carries(&link, &snapshot, 137.5, 12.25);
    // Controls: a link carrying the view's z₀ only (an illustrative link), and the link of another state, each fail.
    rejects("a link carrying z₀ only", || {
        check_link_carries(
            "principia://view?v=1&z0=0.100,0.200,0.300",
            &snapshot,
            137.5,
            12.25,
        )
    });
    rejects("the link checked against another horizon", || {
        check_link_carries(&link, &snapshot, 50.0, 12.25)
    });
    let raw = link.replacen("%22", "\"", 1);
    rejects("a link with an unescaped quote", || {
        check_link_carries(&raw, &snapshot, 137.5, 12.25)
    });
    // The link follows the state: another playhead, another link, carrying it.
    app.set_field(SetField {
        edit: Edit::Render(RenderField::Playhead(Playhead { t: 30.0 })),
        no_history: false,
    });
    settle(&mut h, &mut app);
    let (snapshot2, link2) = copy_both(&mut h, &mut app);
    assert_ne!(link2, link, "the link did not follow the playhead");
    check_link_carries(&link2, &snapshot2, 137.5, 30.0);
}

// --- §G5 Run: T_horizon is SimConfig's horizon, sent on Recompute ----------------------------------------------------

/// The open Run window's Recompute, by the keys.
fn recompute(h: &mut Headless, app: &mut App<MockSide>) {
    to_section(h, app, "Recompute", NONE);
    to_element(h, app, "Recompute");
    let _ = press(h, app, Key::Enter, NONE);
    settle(h, app);
}

fn check_horizon(got: f64, want: f64, what: &str) {
    assert_eq!(got, want, "{what}: the snapshot's horizon");
}

#[test]
fn qa_mock_windows_run_t_horizon_is_the_horizon_sent_on_recompute() {
    let (mut app, mut h) = fresh();
    // The state's T off the default 50, so a window that does not open on the state's T shows.
    app.set_field(SetField {
        edit: Edit::Sim(SimField::Horizon(Horizon { t: 120.0 })),
        no_history: false,
    });
    settle(&mut h, &mut app);
    open_from_windows_menu(&mut h, &mut app, "Run");
    assert!(app.is_open(Window::Run));
    let depth = app.snapshot().history.undo_depth;
    // Recompute with T untouched: T stays the state's.
    recompute(&mut h, &mut app);
    check_horizon(
        app.snapshot().sim.horizon.t,
        120.0,
        "Recompute, T untouched",
    );
    rejects("a window opened on the default T", || {
        check_horizon(50.0, 120.0, "Recompute, T untouched")
    });
    // → on T_horizon, in interaction mode, raises T; Recompute sends it, inside [50, 200].
    to_section(&mut h, &mut app, "Integration", SHIFT);
    to_element(&mut h, &mut app, "T_horizon");
    let _ = press(&mut h, &mut app, Key::Enter, NONE);
    assert_eq!(app.view.focus.mode, INTERACT, "Enter on T_horizon");
    let _ = press(&mut h, &mut app, Key::ArrowRight, NONE);
    let _ = press(&mut h, &mut app, Key::Escape, NONE);
    assert_eq!(app.view.focus.mode, NAV);
    recompute(&mut h, &mut app);
    let t = app.snapshot().sim.horizon.t;
    assert!(t > 120.0 && t <= 200.0, "→ then Recompute: T {t}");
    // One undoable edit, logged as the contract gives it.
    let s = app.snapshot().history.undo_depth;
    assert!(s > depth, "Recompute's horizon edit is not in the history");
    let line = format!("SetField Horizon.t 120 → {t}");
    assert!(
        app.console().iter().any(|e| e.message == line),
        "no `{line}` in the console: {:?}",
        app.console().iter().map(|e| &e.message).collect::<Vec<_>>()
    );
    // Undo restores T.
    app.undo();
    settle(&mut h, &mut app);
    check_horizon(app.snapshot().sim.horizon.t, 120.0, "undo");
    // T stays in [50, 200]: → held to the top stops at 200.
    to_section(&mut h, &mut app, "Integration", SHIFT);
    to_element(&mut h, &mut app, "T_horizon");
    let _ = press(&mut h, &mut app, Key::Enter, NONE);
    for _ in 0..200 {
        let _ = h.frame(&mut app, vec![key_event(Key::ArrowRight, true, SHIFT)]);
        let _ = h.frame(&mut app, vec![key_event(Key::ArrowRight, false, SHIFT)]);
    }
    let _ = press(&mut h, &mut app, Key::Escape, NONE);
    recompute(&mut h, &mut app);
    check_horizon(
        app.snapshot().sim.horizon.t,
        200.0,
        "→ held, then Recompute",
    );
}

// --- §G4: open in Inspector is the lock badge's ----------------------------------------------------------------------

#[test]
fn qa_mock_keyboard_open_in_inspector_acts_only_while_locked() {
    let path = ["Manifold view", "Navigate", "open in Inspector"];
    let (mut app, mut h) = fresh();
    assert!(!app.snapshot().sim.lock.locked, "the mock starts free");
    walk_to(&mut h, &mut app, &path);
    let _ = press(&mut h, &mut app, Key::Enter, NONE);
    settle(&mut h, &mut app);
    let check_closed = |open: bool| {
        assert!(
            !open,
            "Enter on open in Inspector while free opened the Inspector"
        )
    };
    check_closed(app.is_open(Window::Inspector));
    // Locked, the same Enter opens it (the control: the check fails on it).
    let z0 = app.snapshot().sim.plane.z0;
    app.set_field(SetField {
        edit: Edit::Sim(SimField::Lock(Lock {
            locked: true,
            z_locked: z0,
        })),
        no_history: false,
    });
    settle(&mut h, &mut app);
    walk_to(&mut h, &mut app, &path);
    let _ = press(&mut h, &mut app, Key::Enter, NONE);
    settle(&mut h, &mut app);
    assert!(
        app.is_open(Window::Inspector),
        "Enter on open in Inspector while locked"
    );
    rejects("the Inspector opened while locked", || {
        check_closed(app.is_open(Window::Inspector))
    });
}

// --- §G2: Open full viewer… opens the Inspector ------------------------------------------------------------------------

#[test]
fn qa_mock_windows_open_full_viewer_opens_the_inspector() {
    // By Enter: the Inspector takes the focus, and Esc from its top level returns it to Open full viewer….
    let (mut app, mut h) = fresh();
    walk_to(&mut h, &mut app, &["Trajectory", "Open full viewer…"]);
    assert!(!app.is_open(Window::Inspector));
    let _ = press(&mut h, &mut app, Key::Enter, NONE);
    settle(&mut h, &mut app);
    assert!(app.is_open(Window::Inspector), "Enter on Open full viewer…");
    assert_eq!(
        app.view.focus.path.first().map(String::as_str),
        Some(gui::windows::entry(Window::Inspector).scope),
        "the Inspector takes the focus: {:?}",
        labels(&app)
    );
    for _ in 0..4 {
        if !app.is_open(Window::Inspector) {
            break;
        }
        let _ = press(&mut h, &mut app, Key::Escape, NONE);
    }
    assert!(!app.is_open(Window::Inspector), "Esc closes it");
    check_path(
        &labels(&app),
        &["Trajectory", "Open full viewer…"],
        "Esc back to the opener",
    );
    // By click.
    let (mut app, mut h) = fresh();
    let n = names_in(&names(&mut h, &mut app), layout().trajectory);
    let at = centre_of(&n, "Open full viewer…").expect("an Open full viewer…");
    click(&mut h, &mut app, at);
    settle(&mut h, &mut app);
    assert!(
        app.is_open(Window::Inspector),
        "a click on Open full viewer…"
    );
}

// --- REQ-GUI-174: the wheel scrolls a window, and the focus moving scrolls it back ------------------------------------

/// Whether the focused element is drawn inside its window's rect.
fn focus_in_view(app: &App<MockSide>, window: Window) -> bool {
    let id = app.view.focus.path.last().expect("a focus").clone();
    let element = app.keyboard.place_of(&id).expect("the focus placed").rect;
    let frame = app
        .keyboard
        .place_of(gui::windows::entry(window).scope)
        .expect("the window placed")
        .rect;
    frame.expand(0.5).contains_rect(element)
}

#[test]
fn qa_mock_windows_the_wheel_scroll_stays_and_a_focus_move_scrolls_back() {
    let window = Window::Run;
    let (mut app, mut h) = fresh();
    open_from_windows_menu(&mut h, &mut app, "Run");
    settle(&mut h, &mut app);
    let landing = app.view.focus.path.clone();
    let id = landing.last().cloned().expect("a landing");
    let landing_label = labels(&app).last().cloned().unwrap();
    assert!(focus_in_view(&app, window), "the landing is in view");
    let top = |app: &App<MockSide>| app.keyboard.place_of(&id).expect("placed").rect.min.y;
    let before = top(&app);
    let frame = app
        .keyboard
        .place_of(gui::windows::entry(window).scope)
        .unwrap()
        .rect;
    let _ = h.frame(&mut app, vec![Event::PointerMoved(frame.center())]);
    for _ in 0..6 {
        let _ = h.frame(
            &mut app,
            vec![Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: egui::vec2(0.0, -8.0),
                phase: egui::TouchPhase::Move,
                modifiers: NONE,
            }],
        );
    }
    settle(&mut h, &mut app);
    let scrolled = top(&app);
    assert!(
        scrolled < before - 50.0,
        "the wheel did not scroll the Run window: {before} → {scrolled}"
    );
    // It stays where the wheel left it, the focus unmoved, for many frames.
    for _ in 0..20 {
        let _ = h.frame(&mut app, Vec::new());
    }
    assert_eq!(app.view.focus.path, landing, "the focus moved");
    let later = top(&app);
    assert!(
        (later - scrolled).abs() < 1.0,
        "the window snapped back to its focus: {scrolled} → {later}"
    );
    assert!(
        !focus_in_view(&app, window),
        "the wheel left the landing in view"
    );
    // The focus moving scrolls its element into view: Tab to the next section and Shift+Tab back to the landing's.
    let _ = press(&mut h, &mut app, Key::Tab, NONE);
    let _ = press(&mut h, &mut app, Key::Tab, SHIFT);
    to_element(&mut h, &mut app, &landing_label);
    settle(&mut h, &mut app);
    assert_eq!(app.view.focus.path, landing, "back on the landing");
    assert!(
        focus_in_view(&app, window),
        "the focus moved back to the landing, out of view: {:?}",
        labels(&app)
    );
}

// --- §G8: the Inspector's readouts, on an orbit ------------------------------------------------------------------------

/// The texts drawn in the Inspector, opened by IC Inspector… with the orbit under the pointer shown.
fn inspector_on_an_orbit() -> (App<MockSide>, Headless, Vec<String>) {
    let (mut app, mut h) = fresh();
    let at = gui::capture::hover_point(layout().figure);
    for _ in 0..4 {
        let _ = h.frame(&mut app, vec![Event::PointerMoved(at)]);
    }
    walk_to(&mut h, &mut app, &["Trajectory", "IC Inspector…"]);
    let _ = press(&mut h, &mut app, Key::Enter, NONE);
    settle(&mut h, &mut app);
    assert!(app.is_open(Window::Inspector), "IC Inspector…");
    let rect = app
        .keyboard
        .place_of(gui::windows::entry(Window::Inspector).scope)
        .expect("placed")
        .rect;
    let drawn = names_in(&names(&mut h, &mut app), rect)
        .into_iter()
        .map(|n| n.name)
        .collect();
    (app, h, drawn)
}

fn check_readouts(drawn: &[String]) {
    // §G8 "Readouts: latent `z`, `α, β`, masses, `E · L_z`, outcome, F₂ word, round-trip error, gauge error".
    for want in [
        "latent z", "α", "β", "masses", "E", "L_z", "outcome", "F₂", "round", "gauge",
    ] {
        assert!(
            drawn.iter().any(|t| t.contains(want)),
            "the Inspector draws no `{want}` readout: {drawn:?}"
        );
    }
}

#[test]
fn qa_mock_windows_inspector_draws_the_g8_readouts() {
    let (app, _, drawn) = inspector_on_an_orbit();
    assert!(
        app.windows.inspector.sample.is_some(),
        "the Inspector opened on no orbit"
    );
    check_readouts(&drawn);
    // Control: the Inspector opened on no orbit draws no readouts.
    let (mut app, mut h) = fresh();
    let ctx = h.ctx.clone();
    app.open_window(&ctx, Window::Inspector, None);
    settle(&mut h, &mut app);
    let rect = app
        .keyboard
        .place_of(gui::windows::entry(Window::Inspector).scope)
        .expect("placed")
        .rect;
    let empty: Vec<String> = names_in(&names(&mut h, &mut app), rect)
        .into_iter()
        .map(|n| n.name)
        .collect();
    rejects("the Inspector with no IC", || check_readouts(&empty));
}

/// A sample with the three bodies' paths `bodies` over `path_t`, its masses equal (`z[6] = z[7] = 0`, so the mass block
/// sits at the simplex's centre).
fn sample_with(bodies: [Vec<[f64; 2]>; 3], path_t: f64) -> engine::contract::sample::Sample {
    engine::contract::sample::Sample {
        at: [0.5, 0.5],
        ic: [0.0; 8],
        fate: None,
        t_end: None,
        ftle: 0.0,
        e0: -1.0,
        f2_word: String::new(),
        substeps: 0,
        min_separation: 1.0,
        energy_drift: 1e-6,
        path_t,
        shape: vec![[0.0, 0.0]; 3],
        bodies,
    }
}

#[test]
fn qa_inspector_l_z_and_masses_on_a_known_answer() {
    use gui::windows::inspector::{l_z, masses};
    // Equal masses from the mass block's centre, on the simplex.
    let free = |v0: f64| {
        sample_with(
            [
                vec![[0.0, 1.0], [v0, 1.0], [2.0 * v0, 1.0]],
                vec![[0.0, -1.0], [-1.0, -1.0], [-2.0, -1.0]],
                vec![[0.0, 0.0]; 3],
            ],
            2.0,
        )
    };
    let m = masses(&free(1.0));
    assert!(
        m.iter().all(|x| (x - 1.0 / 3.0).abs() < 1e-12),
        "equal masses at the mass block's centre: {m:?}"
    );
    // L_z = Σ m (x v_y − y v_x): body 0 at (0, 1) moving +x at 1, body 1 at (0, −1) moving −x at 1, body 2 at rest;
    // each contributes −m, so L_z = −2/3. Free bodies keep their L_z, so any step of the path gives it.
    let check = |got: f64| assert!((got + 2.0 / 3.0).abs() < 1e-12, "L_z {got}, expected −2/3");
    check(l_z(&free(1.0)));
    // Control: body 0 moving −x cancels body 1, so L_z = 0, which the check rejects.
    let reversed = l_z(&free(-1.0));
    assert!(reversed.abs() < 1e-12, "the reversed L_z {reversed}");
    rejects("L_z of the reversed orbit", || check(reversed));
}
