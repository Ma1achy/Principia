//! QA tests for TASK-M6-44, written from R-463 and its two requirements, not from the implementation:
//!
//! - REQ-GUI-215 (R-463 K1 to K3; render_gui_spec §G12, §G3): the console is docked along the bottom; its top edge is
//!   dragged between its header bar and three log rows and the full height below the top bar, and remembered with the
//!   layout, reset by a double-click and by View › reset layout (R-462 J3). Maximise, between Clear and ×, and the scope
//!   shortcut M, in either mode wherever the focus is in the console and never while the search field is typed in, toggle
//!   the full height and back to the dragged height. Maximised, it covers the panels and the figure: what is under it
//!   takes no pointer. ← → cross All, Warnings, Errors, Info, Search, Copy, Clear, Maximise, ×, the breadcrumb
//!   "Console › Header › Maximise" on it.
//! - REQ-GUI-216 (R-463 K4), what a test off the human's Mac can hold (the 16.7 ms itself is the benchmark's, R-186):
//!   each way of opening and closing completes in the frame of its input, whole, at the three log sizes; the rows laid
//!   out do not grow with the log, while the whole log stays reachable (R-450); and the harness reports each measured
//!   frame's breakdown at the three sizes and judges it against one frame at 60 fps.
//!
//! The focus is read from `ViewUI`, by the tree's labels; what is drawn from the frame's AccessKit tree and its painted
//! shapes, as an AccessKit tree can hold a discarded pass's. Each check runs its own control (R-176) through [`rejects`].
// The file name `qa_TASK-M6-44` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::path::PathBuf;

use eframe::egui::{
    self, pos2, Event, FullOutput, Key, Modifiers, PointerButton, Pos2, Rect, Shape,
};
use engine::contract::interface::EngineInterface;
use engine::contract::render_state::Playhead;
use engine::contract::set_field::{Edit, RenderField, SetField};
use engine::contract::view_ui::{Focus, KeyMode, Mode, Window};
use gui::app::{App, CONSOLE_ROWS};
use gui::bench::{self, FrameTime, Row, Table};
use gui::capture::{footer_point, FORMAT, PIXELS_PER_POINT, SIZE};
use gui::explore::{FIGURE, MANIFOLD_VIEW};
use gui::headless::Headless;
use gui::layout::{Layout, Splitter};
use gui::mock::MockEngine;
use gui::settings::Store;
use gui::side::{EngineSide, MockSide};
use gui::windows::console;

const NONE: Modifiers = Modifiers::NONE;
const NAV: KeyMode = KeyMode::Navigation;
const INTERACT: KeyMode = KeyMode::Interaction;
/// Edges sit on whole physical pixels, 1/1.5 point at the capture's scale: half of that each way, and a little more.
const TOL: f32 = 0.7;
/// R-463 K4's three log sizes.
const SIZES: [usize; 3] = [0, 1_000, 10_000];

type Rig = (App<MockSide>, Headless);

/// Asserts that `check` panics: the check can fail (R-176's control, run in the test).
fn rejects(what: &str, check: impl FnOnce()) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check));
    assert!(
        outcome.is_err(),
        "the check passed on {what}, which it must reject"
    );
}

fn side() -> MockSide {
    MockSide::new(MockEngine::frozen(), None)
}

fn settle<S: EngineSide>(h: &mut Headless, app: &mut App<S>) {
    for _ in 0..4 {
        let _ = h.frame(app, Vec::new());
    }
}

fn fresh() -> Rig {
    rig(App::new(side(), FORMAT))
}

fn stored(path: &PathBuf) -> Rig {
    rig(App::with_settings(side(), FORMAT, Store::at(path)))
}

fn rig(mut app: App<MockSide>) -> Rig {
    let mut h = Headless::new(SIZE, PIXELS_PER_POINT);
    settle(&mut h, &mut app);
    (app, h)
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("qa-m6-44-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir.join("gui_settings.json")
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

fn press(h: &mut Headless, app: &mut App<MockSide>, key: Key) {
    let _ = h.frame(app, vec![key_event(key, true, NONE)]);
    let _ = h.frame(app, vec![key_event(key, false, NONE)]);
    let _ = h.frame(app, Vec::new());
}

fn presses(h: &mut Headless, app: &mut App<MockSide>, key: Key, n: usize) {
    for _ in 0..n {
        press(h, app, key);
    }
}

/// M as a keyboard sends it: the key's press with its text, then the release.
fn m(h: &mut Headless, app: &mut App<MockSide>) {
    let events = vec![key_event(Key::M, true, NONE), Event::Text("m".to_owned())];
    let _ = h.frame(app, events);
    let _ = h.frame(app, vec![key_event(Key::M, false, NONE)]);
    settle(h, app);
}

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: NONE,
    }
}

/// A click at `pos`, returning the frame of its release: the frame in which a click acts.
fn click(h: &mut Headless, app: &mut App<MockSide>, pos: Pos2) -> FullOutput {
    let _ = h.frame(app, vec![Event::PointerMoved(pos)]);
    let _ = h.frame(app, vec![button(pos, true)]);
    h.frame(app, vec![button(pos, false)])
}

fn clicked(h: &mut Headless, app: &mut App<MockSide>, pos: Pos2) {
    let _ = click(h, app, pos);
    let _ = h.frame(app, vec![Event::PointerGone]);
    settle(h, app);
}

fn drag(h: &mut Headless, app: &mut App<MockSide>, from: Pos2, to: Pos2) {
    let mut frames = vec![vec![Event::PointerMoved(from)], vec![button(from, true)]];
    for k in 1..=4 {
        frames.push(vec![Event::PointerMoved(
            from + (to - from) * (k as f32 / 4.0),
        )]);
    }
    frames.push(vec![button(to, false)]);
    frames.push(vec![Event::PointerGone]);
    for events in frames {
        let _ = h.frame(app, events);
    }
    settle(h, app);
}

fn layout(h: &Headless, app: &App<MockSide>) -> Layout {
    Layout::with_splits(h.screen(), PIXELS_PER_POINT, &app.splits)
}

/// The rect scope `id` was placed on this frame.
fn place(app: &App<MockSide>, id: &str) -> Rect {
    app.keyboard
        .place_of(id)
        .unwrap_or_else(|| panic!("`{id}` not placed"))
        .rect
}

fn console_rect(app: &App<MockSide>) -> Rect {
    place(app, console::WINDOW)
}

/// A drag on the console's top edge, at the window's middle, by `by` points.
fn drag_edge(h: &mut Headless, app: &mut App<MockSide>, by: f32) {
    let top = console_rect(app).min.y;
    let x = h.screen().center().x;
    drag(h, app, pos2(x, top), pos2(x, top + by));
}

fn near(a: f32, b: f32, what: &str) {
    assert!((a - b).abs() <= TOL, "{what}: {a} against {b}");
}

/// The console docked with its top at `top`: the window's width, down to the footer.
fn check_docked(h: &Headless, app: &App<MockSide>, top: f32, what: &str) {
    let rect = console_rect(app);
    let l = layout(h, app);
    near(rect.min.y, top, what);
    near(rect.max.y, l.footer.min.y, what);
    near(rect.min.x, h.screen().min.x, what);
    near(rect.max.x, h.screen().max.x, what);
}

/// The console at the full height below the top bar (R-463 K1, K2).
fn check_full(h: &Headless, app: &App<MockSide>, what: &str) {
    check_docked(h, app, layout(h, app).top_bar.max.y, what);
}

fn labels(app: &App<MockSide>) -> Vec<String> {
    app.keyboard
        .tree(app.view.mode)
        .labels(&app.view.focus.path)
        .into_iter()
        .map(str::to_owned)
        .collect()
}

fn check_at(app: &App<MockSide>, path: &[&str], mode: KeyMode, what: &str) {
    assert_eq!(labels(app), path, "{what}");
    assert_eq!(app.view.focus.mode, mode, "{what}");
}

/// Shift+Tab from no focus (the footer, R-405), then Enter: the console opened from the footer.
fn open(h: &mut Headless, app: &mut App<MockSide>) {
    app.view.focus.path.clear();
    let _ = h.frame(app, vec![key_event(Key::Tab, true, Modifiers::SHIFT)]);
    let _ = h.frame(app, vec![key_event(Key::Tab, false, Modifiers::SHIFT)]);
    press(h, app, Key::Enter);
    settle(h, app);
    check_at(app, &["Console", "Header"], NAV, "the landing");
}

fn opened() -> Rig {
    let (mut app, mut h) = fresh();
    open(&mut h, &mut app);
    (app, h)
}

/// A log of `lines` entries with distinct messages, the mock's line for each `SetField`, read into the console.
fn logged(lines: usize) -> Rig {
    let (mut app, mut h) = fresh();
    let t0 = app.snapshot().render.playhead.t;
    for i in 0..lines {
        app.side().engine().set_field(SetField {
            edit: Edit::Render(RenderField::Playhead(Playhead {
                t: t0 + 1e-3 * (i + 1) as f64,
            })),
            no_history: false,
        });
    }
    // Past the snapshot's interval, so the frame reads them; and back to 60 fps.
    h.set_frame_step(1.0);
    settle(&mut h, &mut app);
    h.set_frame_step(1.0 / 60.0);
    settle(&mut h, &mut app);
    assert!(
        app.console().len() >= lines.min(CONSOLE_ROWS),
        "{} entries in the log for {lines}",
        app.console().len()
    );
    (app, h)
}

/// One AccessKit node with text: its text and its rect in points.
fn named(h: &Headless, output: &FullOutput) -> Vec<(String, Rect)> {
    let ppp = f64::from(PIXELS_PER_POINT);
    h.names(output)
        .into_iter()
        .filter_map(|n| {
            let r = n.rect?;
            Some((
                n.name,
                Rect::from_min_max(
                    pos2((r[0] / ppp) as f32, (r[1] / ppp) as f32),
                    pos2((r[2] / ppp) as f32, (r[3] / ppp) as f32),
                ),
            ))
        })
        .collect()
}

/// The messages drawn in the log, top to bottom, whole inside its rect.
fn whole_rows(h: &mut Headless, app: &mut App<MockSide>) -> Vec<String> {
    let out = h.frame(app, Vec::new());
    let log = place(app, console::LOG);
    let messages: std::collections::HashSet<&str> =
        app.console().iter().map(|e| e.message.as_str()).collect();
    let mut rows: Vec<(String, Rect)> = named(h, &out)
        .into_iter()
        .filter(|(n, r)| messages.contains(n.as_str()) && log.expand(0.5).contains_rect(*r))
        .collect();
    rows.sort_by(|a, b| a.1.min.y.total_cmp(&b.1.min.y));
    rows.into_iter().map(|(n, _)| n).collect()
}

/// Clicks the control named `name` in the frame's AccessKit tree.
fn click_named(h: &mut Headless, app: &mut App<MockSide>, name: &str) {
    let out = h.frame(app, Vec::new());
    let at = named(h, &out)
        .into_iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("`{name}` not drawn"))
        .1
        .center();
    clicked(h, app, at);
}

/// Every text painted in `output`, with the point it is painted at.
fn painted(output: &FullOutput) -> Vec<(String, Pos2)> {
    fn walk(shape: &Shape, out: &mut Vec<(String, Pos2)>) {
        match shape {
            Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
            Shape::Text(t) => out.push((t.galley.text().to_owned(), t.pos)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    output.shapes.iter().for_each(|s| walk(&s.shape, &mut out));
    out
}

// --- REQ-GUI-215: the height dragged ----------------------------------------------------------------------------------

#[test]
fn qa_m6_44_a_drag_on_the_top_edge_sets_the_height_and_moves_no_panel() {
    let (mut app, mut h) = opened();
    let before = console_rect(&app);
    // Its first place is the bottom row's (§G12, K1).
    near(
        before.min.y,
        layout(&h, &app).bottom_row.min.y,
        "the default",
    );
    let (figure, manifold) = (place(&app, FIGURE), place(&app, MANIFOLD_VIEW));
    drag_edge(&mut h, &mut app, -150.0);
    check_docked(&h, &app, before.min.y - 150.0, "dragged up");
    assert_eq!(place(&app, FIGURE), figure, "the figure moved");
    assert_eq!(place(&app, MANIFOLD_VIEW), manifold, "a panel moved");
    drag_edge(&mut h, &mut app, 60.0);
    check_docked(&h, &app, before.min.y - 90.0, "dragged down");
    // Control: a drag that starts off the edge is a console of fixed height to this check, and fails it.
    let inside = console_rect(&app).center();
    let held = console_rect(&app).min.y;
    drag(&mut h, &mut app, inside, inside - egui::vec2(0.0, 40.0));
    near(
        console_rect(&app).min.y,
        held,
        "a drag off the edge moved it",
    );
    rejects("a console of fixed height", || {
        check_docked(&h, &app, held - 40.0, "control")
    });
}

#[test]
fn qa_m6_44_the_drag_is_clamped_at_the_header_and_three_rows_and_at_the_top_bar() {
    let (mut app, mut h) = logged(40);
    open(&mut h, &mut app);
    drag_edge(&mut h, &mut app, 5000.0);
    let l = layout(&h, &app);
    let least = console_rect(&app);
    near(least.max.y, l.footer.min.y, "docked at its least");
    // Its least is its header bar and three log rows, whole: three, not two, and no slack for a fourth.
    let rows = whole_rows(&mut h, &mut app);
    assert_eq!(rows.len(), 3, "the log's whole rows at the least: {rows:?}");
    let header = place(&app, console::HEADER);
    assert!(
        least.contains_rect(header),
        "the header bar is cut: {header:?}"
    );
    let log = place(&app, console::LOG);
    assert!(
        least.contains_rect(log),
        "the log is cut: {log:?} in {least:?}"
    );
    rejects("a log of two rows", || assert_eq!(rows.len(), 2));
    // A further drag down stays there.
    drag_edge(&mut h, &mut app, 300.0);
    near(console_rect(&app).min.y, least.min.y, "past the least");
    drag_edge(&mut h, &mut app, -5000.0);
    check_full(&h, &app, "dragged past the top bar");
    let top = console_rect(&app);
    assert!(top.min.y >= l.top_bar.max.y - TOL, "over the top bar");
    rejects("a console that climbs over the top bar", || {
        check_docked(&h, &app, l.top_bar.min.y, "control")
    });
    // The log takes the height: dragged tall, more rows show.
    let tall = whole_rows(&mut h, &mut app);
    assert!(
        tall.len() > 3 * 3,
        "only {} rows at the full height",
        tall.len()
    );
}

#[test]
fn qa_m6_44_the_height_is_its_own_not_the_top_and_bottom_rows_splitter() {
    // The console's default top edge is the rows' boundary: with it open, a drag there moves the console, not the rows.
    let (mut app, mut h) = opened();
    let rows = app.splits.get(Splitter::TopBottom);
    drag_edge(&mut h, &mut app, -100.0);
    assert_eq!(app.splits.get(Splitter::TopBottom), rows, "the rows moved");
    let dragged = console_rect(&app);
    // Closed, that boundary is the rows' again; the console, reopened, is at the height it was dragged to.
    press(&mut h, &mut app, Key::Escape);
    assert!(!app.is_open(Window::Console), "Esc closes it");
    let bottom = layout(&h, &app).bottom_row.min.y;
    let x = h.screen().center().x;
    drag(&mut h, &mut app, pos2(x, bottom), pos2(x, bottom - 50.0));
    assert_ne!(app.splits.get(Splitter::TopBottom), rows, "the splitter");
    open(&mut h, &mut app);
    near(
        console_rect(&app).min.y,
        dragged.min.y,
        "the rows' splitter moved it",
    );
    // A double-click on its edge resets it, and only it.
    let moved = app.splits.get(Splitter::TopBottom);
    let at = pos2(x, console_rect(&app).min.y);
    let _ = h.frame(&mut app, vec![Event::PointerMoved(at)]);
    for _ in 0..2 {
        let _ = h.frame(&mut app, vec![button(at, true)]);
        let _ = h.frame(&mut app, vec![button(at, false)]);
    }
    let _ = h.frame(&mut app, vec![Event::PointerGone]);
    settle(&mut h, &mut app);
    near(
        console_rect(&app).min.y,
        layout(&h, &app).bottom_row.min.y,
        "reset to the bottom row",
    );
    assert_eq!(
        app.splits.get(Splitter::TopBottom),
        moved,
        "another splitter reset"
    );
}

#[test]
fn qa_m6_44_the_height_survives_a_restart_the_maximised_state_leaving_it() {
    let path = scratch("restart");
    let top = {
        let (mut app, mut h) = stored(&path);
        open(&mut h, &mut app);
        drag_edge(&mut h, &mut app, -120.0);
        let top = console_rect(&app).min.y;
        // Toggled to the full height and left there: what is remembered is still the dragged height (K2).
        m(&mut h, &mut app);
        check_full(&h, &app, "M");
        top
    };
    let (mut app, mut h) = stored(&path);
    open(&mut h, &mut app);
    if (console_rect(&app).min.y - layout(&h, &app).top_bar.max.y).abs() <= TOL {
        m(&mut h, &mut app);
    }
    check_docked(&h, &app, top, "a new GUI");
    // Control: a GUI that forgets it, the same script with no file, is at the default.
    let (mut other, mut oh) = fresh();
    open(&mut oh, &mut other);
    rejects("a GUI that forgets the height", || {
        check_docked(&oh, &other, top, "control")
    });
}

#[test]
fn qa_m6_44_a_remembered_height_out_of_its_range_is_held_or_refused() {
    // A window smaller since, or a hand-edited file: the file's edge is still held to the console's range.
    for (fraction, full) in [(0.0, true), (1.0, false)] {
        let path = scratch(&format!("range{fraction}"));
        std::fs::write(
            &path,
            format!("{{\"layout\":{{\"console_top\":{fraction}}}}}"),
        )
        .unwrap();
        let (mut app, mut h) = stored(&path);
        open(&mut h, &mut app);
        if full {
            check_full(&h, &app, "console_top 0");
        } else {
            let rect = console_rect(&app);
            assert!(rect.height() > 60.0, "console_top 1 closes it: {rect:?}");
            check_docked(&h, &app, rect.min.y, "console_top 1");
        }
    }
    // Out of [0, 1]: not a height; the file is ignored, the console at its default.
    let path = scratch("range-bad");
    std::fs::write(&path, "{\"layout\":{\"console_top\":1.5}}").unwrap();
    let (mut app, mut h) = stored(&path);
    open(&mut h, &mut app);
    near(
        console_rect(&app).min.y,
        layout(&h, &app).bottom_row.min.y,
        "console_top 1.5",
    );
}

#[test]
fn qa_m6_44_view_reset_layout_restores_the_height_from_under_a_maximised_console() {
    let path = scratch("reset");
    {
        let (mut app, mut h) = stored(&path);
        open(&mut h, &mut app);
        drag_edge(&mut h, &mut app, -100.0);
        m(&mut h, &mut app);
        check_full(&h, &app, "maximised");
        // The top bar stays usable above it: K1's full height is below the top bar.
        click_named(&mut h, &mut app, "View");
        click_named(&mut h, &mut app, "reset layout");
        let default = layout(&h, &app).bottom_row.min.y;
        check_docked(&h, &app, default, "reset layout");
        // No drag is left to restore: M, the focus back in the console, goes to the full height and back to the default.
        let copy = place(&app, console::COPY).center();
        clicked(&mut h, &mut app, copy);
        m(&mut h, &mut app);
        check_full(&h, &app, "M after the reset");
        m(&mut h, &mut app);
        check_docked(&h, &app, default, "back after the reset");
    }
    let (mut app, mut h) = stored(&path);
    open(&mut h, &mut app);
    let default = layout(&h, &app).bottom_row.min.y;
    check_docked(&h, &app, default, "a new GUI after reset layout");
}

// --- REQ-GUI-215: the toggle -------------------------------------------------------------------------------------------

#[test]
fn qa_m6_44_a_click_on_maximise_and_enter_on_it_toggle_the_full_height_and_back_to_the_dragged_one()
{
    let (mut app, mut h) = opened();
    drag_edge(&mut h, &mut app, -70.0);
    let dragged = console_rect(&app).min.y;
    let at = place(&app, console::MAXIMISE).center();
    clicked(&mut h, &mut app, at);
    check_full(&h, &app, "a click on maximise");
    // Its place follows the header bar's move: it is clicked again where it now is.
    let at = place(&app, console::MAXIMISE).center();
    assert!(console_rect(&app).contains(at), "maximise off the console");
    clicked(&mut h, &mut app, at);
    check_docked(&h, &app, dragged, "a second click");
    rejects("a toggle that returns to the default height", || {
        check_docked(&h, &app, layout(&h, &app).bottom_row.min.y, "control")
    });
    // Enter on it, by the keys, in the same way; the focus stays on it.
    press(&mut h, &mut app, Key::Escape);
    press(&mut h, &mut app, Key::Enter);
    for _ in 0..12 {
        if labels(&app).last().is_some_and(|l| l == "Maximise") {
            break;
        }
        press(&mut h, &mut app, Key::ArrowRight);
    }
    check_at(
        &app,
        &["Console", "Header", "Maximise"],
        NAV,
        "→ to maximise",
    );
    press(&mut h, &mut app, Key::Enter);
    settle(&mut h, &mut app);
    check_full(&h, &app, "Enter on maximise");
    check_at(
        &app,
        &["Console", "Header", "Maximise"],
        NAV,
        "the focus kept",
    );
    press(&mut h, &mut app, Key::Enter);
    settle(&mut h, &mut app);
    check_docked(&h, &app, dragged, "Enter again");
}

#[test]
fn qa_m6_44_m_toggles_wherever_the_focus_is_in_the_console_in_either_mode_typing_nothing() {
    // Each place in the console's two scopes, by the keys, from the landing.
    type Go = fn(&mut Headless, &mut App<MockSide>);
    let places: [(&str, Go, &[&str], KeyMode); 7] = [
        ("the header scope", |_, _| {}, &["Console", "Header"], NAV),
        (
            "All",
            |h, a| press(h, a, Key::Enter),
            &["Console", "Header", "All"],
            NAV,
        ),
        (
            "the search field, not typing",
            |h, a| {
                press(h, a, Key::Enter);
                presses(h, a, Key::ArrowRight, 4);
            },
            &["Console", "Header", "Search"],
            NAV,
        ),
        (
            "Clear",
            |h, a| {
                press(h, a, Key::Enter);
                presses(h, a, Key::ArrowRight, 6);
            },
            &["Console", "Header", "Clear"],
            NAV,
        ),
        (
            "×",
            |h, a| {
                press(h, a, Key::Enter);
                presses(h, a, Key::ArrowRight, 8);
            },
            &["Console", "Header", "Close"],
            NAV,
        ),
        (
            "the log, navigating",
            |h, a| press(h, a, Key::ArrowDown),
            &["Console", "Log"],
            NAV,
        ),
        (
            "the log, interacting",
            |h, a| {
                press(h, a, Key::ArrowDown);
                press(h, a, Key::Enter);
            },
            &["Console", "Log"],
            INTERACT,
        ),
    ];
    for (what, go, path, mode) in places {
        let (mut app, mut h) = opened();
        let docked = console_rect(&app).min.y;
        go(&mut h, &mut app);
        check_at(&app, path, mode, what);
        m(&mut h, &mut app);
        check_full(&h, &app, what);
        check_at(&app, path, mode, what);
        m(&mut h, &mut app);
        check_docked(&h, &app, docked, what);
        check_at(&app, path, mode, what);
        assert_eq!(app.windows.console.text, "", "{what}: M typed");
    }
}

/// Whether the search field took the `m`s.
fn check_typed(text: &str, want: &str, what: &str) {
    assert_eq!(text, want, "{what}");
}

#[test]
fn qa_m6_44_m_while_typing_in_the_search_field_types_m_and_leaves_the_height() {
    let (mut app, mut h) = opened();
    let docked = console_rect(&app).min.y;
    press(&mut h, &mut app, Key::Enter);
    presses(&mut h, &mut app, Key::ArrowRight, 4);
    // On the field, not typing: M is the shortcut, and the field takes no 'm'.
    m(&mut h, &mut app);
    check_full(&h, &app, "M on the field, not typing");
    let untyped = app.windows.console.text.clone();
    check_typed(&untyped, "", "M on the field, not typing");
    rejects("an M that types outside the field's typing", || {
        check_typed("m", &untyped, "control")
    });
    m(&mut h, &mut app);
    check_docked(&h, &app, docked, "back");
    // Typing, by Enter: every M is a letter.
    press(&mut h, &mut app, Key::Enter);
    check_at(&app, &["Console", "Header", "Search"], INTERACT, "typing");
    m(&mut h, &mut app);
    m(&mut h, &mut app);
    check_typed(&app.windows.console.text, "mm", "typed by Enter");
    check_docked(&h, &app, docked, "M while typing moved it");
    rejects("a console that toggled on the letter", || {
        check_full(&h, &app, "control")
    });
    // Esc keeps the text and stops the typing: M is the shortcut again.
    press(&mut h, &mut app, Key::Escape);
    m(&mut h, &mut app);
    check_full(&h, &app, "M after Esc");
    check_typed(&app.windows.console.text, "mm", "M after Esc");
    m(&mut h, &mut app);
    // Typing, by a click on the field (R-456), at the full height as well: maximised, the letter is still a letter.
    m(&mut h, &mut app);
    check_full(&h, &app, "maximised");
    let at = place(&app, console::SEARCH).center();
    clicked(&mut h, &mut app, at);
    m(&mut h, &mut app);
    check_typed(&app.windows.console.text, "mmm", "typed by a click");
    check_full(&h, &app, "M typed at the full height");
}

#[test]
fn qa_m6_44_m_outside_the_console_neither_toggles_nor_types() {
    let (mut app, mut h) = fresh();
    presses(&mut h, &mut app, Key::Tab, 3);
    assert_eq!(labels(&app), ["Figure"], "three Tabs");
    // An error opens the console, taking no focus.
    app.side().engine().raise_error();
    settle(&mut h, &mut app);
    assert!(app.is_open(Window::Console));
    let docked = console_rect(&app).min.y;
    m(&mut h, &mut app);
    check_docked(&h, &app, docked, "M on the Figure");
    assert_eq!(app.windows.console.text, "", "M on the Figure typed");
    assert_eq!(labels(&app), ["Figure"]);
    // Control: the same M with the focus in the console does toggle.
    let copy = place(&app, console::COPY).center();
    clicked(&mut h, &mut app, copy);
    assert_eq!(labels(&app).first().map(String::as_str), Some("Console"));
    m(&mut h, &mut app);
    check_full(&h, &app, "M with the focus moved in");
    rejects("an M outside the console that toggles", || {
        check_docked(&h, &app, docked, "control")
    });
}

#[test]
fn qa_m6_44_m_is_on_the_shortcuts_sheet() {
    // §G3: a scope's letters are its shortcuts, on the `?` sheet.
    let (app, _h) = opened();
    let rows = app.keyboard.tree(app.view.mode).shortcuts();
    assert!(
        rows.iter().any(|(key, _)| *key == "M"),
        "M not on the sheet: {rows:?}"
    );
}

// --- REQ-GUI-215: the header bar ---------------------------------------------------------------------------------------

#[test]
fn qa_m6_44_the_arrows_cross_the_header_bar_in_k3s_order_with_the_breadcrumb() {
    const ROW: [&str; 9] = [
        "All", "Warnings", "Errors", "Info", "Search", "Copy", "Clear", "Maximise", "Close",
    ];
    let check_row = |seen: &[String], what: &str| assert_eq!(seen, ROW, "{what}");
    let (mut app, mut h) = opened();
    press(&mut h, &mut app, Key::Enter);
    let mut seen = vec![labels(&app).last().cloned().unwrap_or_default()];
    for _ in 1..ROW.len() {
        press(&mut h, &mut app, Key::ArrowRight);
        seen.push(labels(&app).last().cloned().unwrap_or_default());
    }
    check_row(&seen, "→ across the header bar");
    // R-450's old order, × right after Clear, is what this rejects.
    let mut old = seen.clone();
    old.retain(|s| s != "Maximise");
    rejects("R-450's header bar", || check_row(&old, "control"));
    // × is its end; ← from it is maximise, then Clear.
    press(&mut h, &mut app, Key::ArrowRight);
    check_at(&app, &["Console", "Header", "Close"], NAV, "past the end");
    press(&mut h, &mut app, Key::ArrowLeft);
    check_at(&app, &["Console", "Header", "Maximise"], NAV, "← from ×");
    // The breadcrumb, as the top bar paints it.
    let out = h.frame(&mut app, Vec::new());
    let top = layout(&h, &app).top_bar;
    let crumb = "Console › Header › Maximise";
    assert!(
        painted(&out)
            .iter()
            .any(|(t, p)| t == crumb && top.contains(*p)),
        "no breadcrumb `{crumb}` in the top bar"
    );
    press(&mut h, &mut app, Key::ArrowLeft);
    check_at(
        &app,
        &["Console", "Header", "Clear"],
        NAV,
        "← from maximise",
    );
}

#[test]
fn qa_m6_44_the_controls_are_drawn_in_that_order_at_each_height() {
    let check_order = |app: &App<MockSide>, ids: &[&str], what: &str| {
        let xs: Vec<f32> = ids.iter().map(|id| place(app, id).center().x).collect();
        assert!(
            xs.windows(2).all(|w| w[0] < w[1]),
            "{what}: {ids:?} at {xs:?}"
        );
    };
    let ids = [
        console::SEARCH,
        console::COPY,
        console::CLEAR,
        console::MAXIMISE,
        console::CLOSE,
    ];
    let (mut app, mut h) = opened();
    check_order(&app, &ids, "docked");
    m(&mut h, &mut app);
    check_order(&app, &ids, "maximised");
    let header = place(&app, console::HEADER);
    for id in ids {
        assert!(
            header.contains_rect(place(&app, id)),
            "{id} out of the header"
        );
    }
    rejects("× before maximise", || {
        check_order(&app, &[console::CLOSE, console::MAXIMISE], "control")
    });
}

// --- REQ-GUI-215: maximised, it covers what is under it ----------------------------------------------------------------

#[test]
fn qa_m6_44_maximised_nothing_under_the_console_takes_the_pointer() {
    let (mut app, mut h) = fresh();
    let figure = place(&app, FIGURE).center();
    let boundary = {
        let l = layout(&h, &app);
        pos2(l.figure.max.x + 1.0, l.figure.center().y)
    };
    let hover = |h: &mut Headless, app: &mut App<MockSide>| {
        let _ = h.frame(app, vec![Event::PointerMoved(figure)]);
        settle(h, app);
        app.snapshot().render.hover.at
    };
    // Control: with the console closed, the pointer hovers the figure and the boundary drags.
    assert!(
        hover(&mut h, &mut app).is_some(),
        "the figure takes no hover"
    );
    let before = app.splits;
    drag(
        &mut h,
        &mut app,
        boundary,
        boundary + egui::vec2(-30.0, 0.0),
    );
    assert_ne!(app.splits, before, "the boundary is not a splitter");
    app.splits = before;
    settle(&mut h, &mut app);
    open(&mut h, &mut app);
    m(&mut h, &mut app);
    check_full(&h, &app, "M");
    let _ = h.frame(&mut app, vec![Event::PointerGone]);
    settle(&mut h, &mut app);
    assert_eq!(
        hover(&mut h, &mut app),
        None,
        "the figure hovered through it"
    );
    drag(
        &mut h,
        &mut app,
        boundary,
        boundary + egui::vec2(-30.0, 0.0),
    );
    assert_eq!(app.splits, before, "a splitter dragged through it");
    assert_eq!(app.windows.console.top_row, 0, "the drag scrolled nothing");
}

#[test]
fn qa_m6_44_dragged_tall_the_part_of_the_figure_it_covers_takes_no_pointer_and_the_rest_does() {
    let (mut app, mut h) = opened();
    drag_edge(&mut h, &mut app, -300.0);
    let rect = console_rect(&app);
    let figure = place(&app, FIGURE);
    assert!(
        rect.min.y > figure.min.y && rect.min.y < figure.max.y,
        "over part of it"
    );
    let mut hover = |p: Pos2| {
        let _ = h.frame(&mut app, vec![Event::PointerMoved(p)]);
        settle(&mut h, &mut app);
        app.snapshot().render.hover.at
    };
    let x = figure.center().x;
    assert_eq!(
        hover(pos2(x, rect.min.y + 20.0)),
        None,
        "hovered through it"
    );
    assert!(
        hover(pos2(x, rect.min.y - 20.0)).is_some(),
        "the figure above it"
    );
}

/// Whether `output` paints the lock's gold reticle over `figure`: its ring, the gold circle of its radius.
fn paints_reticle(output: &FullOutput, figure: Rect) -> bool {
    fn find(shape: &Shape, figure: Rect) -> bool {
        match shape {
            Shape::Vec(v) => v.iter().any(|s| find(s, figure)),
            Shape::Circle(c) => {
                c.stroke.color == gui::explore::lock::GOLD && figure.contains(c.center)
            }
            _ => false,
        }
    }
    output.shapes.iter().any(|s| find(&s.shape, figure))
}

#[test]
fn qa_m6_44_maximised_the_figures_own_marks_do_not_show_through_and_a_resting_pointer_loses_its_hover(
) {
    // §G1's figure marks, the reticle and the hover label, are drawn on a layer above the windows': K2's "covers" holds
    // for them too.
    let (mut app, mut h) = fresh();
    presses(&mut h, &mut app, Key::Tab, 3);
    assert_eq!(labels(&app), ["Figure"], "three Tabs");
    let figure = place(&app, FIGURE);
    let _ = h.frame(&mut app, vec![Event::PointerMoved(figure.center())]);
    settle(&mut h, &mut app);
    press(&mut h, &mut app, Key::K);
    settle(&mut h, &mut app);
    assert!(app.snapshot().sim.lock.locked, "K locks");
    // Control: unmaximised and closed, the marks show and the pointer hovers.
    let out = h.frame(&mut app, Vec::new());
    assert!(paints_reticle(&out, figure), "no reticle to cover");
    assert!(app.snapshot().render.hover.at.is_some(), "no hover to lose");
    // The console opened and maximised, by the keys, the pointer resting where it was.
    open(&mut h, &mut app);
    m(&mut h, &mut app);
    check_full(&h, &app, "M");
    let out = h.frame(&mut app, Vec::new());
    assert!(!paints_reticle(&out, figure), "the reticle shows through");
    assert_eq!(
        app.snapshot().render.hover.at,
        None,
        "the hover stays through it"
    );
    m(&mut h, &mut app);
    settle(&mut h, &mut app);
    let out = h.frame(&mut app, Vec::new());
    assert!(
        console_rect(&app).min.y > figure.center().y,
        "docked below the mark"
    );
    assert!(
        paints_reticle(&out, figure),
        "the reticle did not come back"
    );
}

#[test]
fn qa_m6_44_maximised_the_keys_page_by_the_taller_log() {
    let (mut app, mut h) = logged(200);
    open(&mut h, &mut app);
    press(&mut h, &mut app, Key::ArrowDown);
    press(&mut h, &mut app, Key::Enter);
    let page_step = |h: &mut Headless, app: &mut App<MockSide>| {
        press(h, app, Key::Home);
        settle(h, app);
        let at = app.windows.console.top_row;
        press(h, app, Key::PageDown);
        settle(h, app);
        app.windows.console.top_row - at
    };
    let docked = page_step(&mut h, &mut app);
    m(&mut h, &mut app);
    let maximised = page_step(&mut h, &mut app);
    check_at(
        &app,
        &["Console", "Log"],
        INTERACT,
        "M kept the interaction",
    );
    assert!(docked >= 1, "a page moved {docked}");
    assert!(
        maximised > 2 * docked,
        "a page moved {maximised}, docked {docked}"
    );
    // What was the top is on screen after the toggle: the top row is what is drawn first.
    let rows = whole_rows(&mut h, &mut app);
    assert!(
        rows.len() >= maximised,
        "{} rows for a page of {maximised}",
        rows.len()
    );
}

// --- REQ-GUI-216: within one frame, at every log size -------------------------------------------------------------------

/// The console whole in `output`: its button painted, at full strength; with a log, a row of it above the footer.
fn check_drawn(h: &Headless, app: &App<MockSide>, output: &FullOutput, settled: u8, what: &str) {
    assert!(app.is_open(Window::Console), "{what}: not open");
    let text = painted(output);
    let rect = console_rect(app);
    assert!(
        text.iter()
            .any(|(t, p)| t == console::MAXIMISE_TEXT && rect.contains(*p)),
        "{what}: no header bar painted in the frame of its input"
    );
    let footer = layout(h, app).footer;
    if let Some(last) = app.console().last() {
        assert!(
            text.iter()
                .any(|(t, p)| t.contains(&last.message) && p.y < footer.min.y),
            "{what}: no row painted in the frame of its input"
        );
    }
    assert_eq!(fill_alpha(output, rect), Some(settled), "{what}: faded");
}

/// The console gone from `output`.
fn check_gone(app: &App<MockSide>, output: &FullOutput, what: &str) {
    assert!(!app.is_open(Window::Console), "{what}: still open");
    assert!(
        !painted(output)
            .iter()
            .any(|(t, _)| t == console::MAXIMISE_TEXT),
        "{what}: painted in the frame that closes it"
    );
}

/// The alpha of the console's frame: the wide rect painted with its top on the console's.
fn fill_alpha(output: &FullOutput, at: Rect) -> Option<u8> {
    fn find(shape: &Shape, at: Rect) -> Option<u8> {
        match shape {
            Shape::Vec(v) => v.iter().find_map(|s| find(s, at)),
            Shape::Rect(r)
                if (r.rect.min.y - at.min.y).abs() < 2.0 && r.rect.width() > at.width() * 0.9 =>
            {
                Some(r.fill.a())
            }
            _ => None,
        }
    }
    output.shapes.iter().find_map(|s| find(&s.shape, at))
}

/// The console's frame's alpha, settled, for a rig of the same shape.
fn settled_alpha() -> u8 {
    let (mut app, mut h) = opened();
    for _ in 0..40 {
        let _ = h.frame(&mut app, Vec::new());
    }
    let out = h.frame(&mut app, Vec::new());
    fill_alpha(&out, console_rect(&app)).expect("its frame is painted")
}

/// The frame of a press of `key`.
fn press_frame(h: &mut Headless, app: &mut App<MockSide>, key: Key) -> FullOutput {
    let out = h.frame(app, vec![key_event(key, true, NONE)]);
    let _ = h.frame(app, vec![key_event(key, false, NONE)]);
    out
}

#[test]
fn qa_m6_44_every_opening_and_closing_completes_in_the_frame_of_its_input_at_every_log_size() {
    let settled = settled_alpha();
    assert!(settled > 200, "the console's frame is {settled}");
    for size in SIZES {
        let (mut app, mut h) = logged(size);
        let what = |how: &str| format!("{how}, {size} entries");
        // A first opening, in a new GUI, by a click on the footer.
        let at = footer_point(layout(&h, &app).footer);
        let out = click(&mut h, &mut app, at);
        check_drawn(
            &h,
            &app,
            &out,
            settled,
            &what("a first click on the footer"),
        );
        let _ = h.frame(&mut app, vec![Event::PointerGone]);
        settle(&mut h, &mut app);
        // Closed by a click on ×.
        let at = place(&app, console::CLOSE).center();
        let out = click(&mut h, &mut app, at);
        check_gone(&app, &out, &what("a click on ×"));
        let _ = h.frame(&mut app, vec![Event::PointerGone]);
        settle(&mut h, &mut app);
        // Opened by Enter on the footer; closed by Esc.
        app.view.focus.path.clear();
        let _ = h.frame(&mut app, vec![key_event(Key::Tab, true, Modifiers::SHIFT)]);
        let _ = h.frame(&mut app, vec![key_event(Key::Tab, false, Modifiers::SHIFT)]);
        let out = press_frame(&mut h, &mut app, Key::Enter);
        check_drawn(&h, &app, &out, settled, &what("Enter on the footer"));
        settle(&mut h, &mut app);
        let out = press_frame(&mut h, &mut app, Key::Escape);
        check_gone(&app, &out, &what("Esc"));
        settle(&mut h, &mut app);
        // Opened again by a click; closed by Enter on ×.
        let at = footer_point(layout(&h, &app).footer);
        let out = click(&mut h, &mut app, at);
        check_drawn(
            &h,
            &app,
            &out,
            settled,
            &what("a later click on the footer"),
        );
        let _ = h.frame(&mut app, vec![Event::PointerGone]);
        settle(&mut h, &mut app);
        let at = place(&app, console::COPY).center();
        clicked(&mut h, &mut app, at);
        presses(&mut h, &mut app, Key::ArrowRight, 3);
        check_at(&app, &["Console", "Header", "Close"], NAV, "→ from Copy");
        let out = press_frame(&mut h, &mut app, Key::Enter);
        check_gone(&app, &out, &what("Enter on ×"));
    }
    // Controls: a frame with no console fails the first check, one with it the second.
    let (mut app, mut h) = fresh();
    let closed = h.frame(&mut app, Vec::new());
    open(&mut h, &mut app);
    let shown = h.frame(&mut app, Vec::new());
    rejects("a frame that has not drawn it", || {
        check_drawn(&h, &app, &closed, settled, "control")
    });
    rejects("a frame at half strength", || {
        check_drawn(&h, &app, &shown, settled / 2, "control")
    });
    press(&mut h, &mut app, Key::Escape);
    rejects("a frame that still paints it", || {
        check_gone(&app, &shown, "control")
    });
}

#[test]
fn qa_m6_44_a_first_opening_by_the_windows_menu_completes_in_its_frame() {
    let settled = settled_alpha();
    let (mut app, mut h) = fresh();
    click_named(&mut h, &mut app, "Windows");
    let out = h.frame(&mut app, Vec::new());
    let top = layout(&h, &app).top_bar;
    let entry = named(&h, &out)
        .into_iter()
        .find(|(n, r)| n == "Console" && r.min.y >= top.max.y - 1.0)
        .expect("Windows › Console drawn")
        .1
        .center();
    let out = click(&mut h, &mut app, entry);
    check_drawn(&h, &app, &out, settled, "Windows › Console");
}

// --- REQ-GUI-216: what the fix must hold ------------------------------------------------------------------------------

#[test]
fn qa_m6_44_the_rows_laid_out_do_not_grow_with_the_log() {
    // At each height, the rows drawn are what the log's rect shows, whatever the log's length. An AccessKit node per row
    // laid out: the whole grid of R-463's pointer would show here as thousands.
    let mut counts = Vec::new();
    for lines in [100, CONSOLE_ROWS] {
        let (mut app, mut h) = logged(lines);
        open(&mut h, &mut app);
        for maximised in [false, true] {
            if maximised {
                m(&mut h, &mut app);
            }
            let out = h.frame(&mut app, Vec::new());
            let messages: std::collections::HashSet<&str> =
                app.console().iter().map(|e| e.message.as_str()).collect();
            let footer = layout(&h, &app).footer.min.y;
            let laid = named(&h, &out)
                .iter()
                .filter(|(n, r)| messages.contains(n.as_str()) && r.min.y < footer)
                .count();
            counts.push((lines, maximised, laid));
        }
    }
    let at = |lines, maximised| {
        counts
            .iter()
            .find(|c| c.0 == lines && c.1 == maximised)
            .expect("measured")
            .2
    };
    for maximised in [false, true] {
        let (few, many) = (at(100, maximised), at(CONSOLE_ROWS, maximised));
        assert!(few > 0, "no row drawn");
        assert_eq!(
            few, many,
            "rows laid out, maximised {maximised}: {counts:?}"
        );
    }
    assert!(at(CONSOLE_ROWS, true) < 200, "{counts:?}");
}

#[test]
fn qa_m6_44_the_whole_log_stays_reachable_at_console_rows_docked_and_maximised() {
    let (mut app, mut h) = logged(CONSOLE_ROWS);
    assert_eq!(app.console().len(), CONSOLE_ROWS, "the log's length");
    open(&mut h, &mut app);
    press(&mut h, &mut app, Key::ArrowDown);
    press(&mut h, &mut app, Key::Enter);
    let shown: Vec<String> = {
        let rows: Vec<String> = app.console().iter().map(|e| e.message.clone()).collect();
        // The list's ends are the log's, whichever way it runs.
        vec![rows[0].clone(), rows[rows.len() - 1].clone()]
    };
    assert_ne!(shown[0], shown[1], "the log's ends differ");
    for maximised in [false, true] {
        if maximised {
            m(&mut h, &mut app);
        }
        press(&mut h, &mut app, Key::Home);
        settle(&mut h, &mut app);
        let first = whole_rows(&mut h, &mut app);
        let top = first.first().cloned().expect("a row at Home");
        assert!(
            shown.contains(&top),
            "maximised {maximised}: Home shows {top}"
        );
        press(&mut h, &mut app, Key::End);
        settle(&mut h, &mut app);
        let last = whole_rows(&mut h, &mut app);
        let bottom = last.last().cloned().expect("a row at End");
        assert!(
            shown.contains(&bottom),
            "maximised {maximised}: End shows {bottom}"
        );
        assert_ne!(top, bottom, "maximised {maximised}: End did not move");
        // A row in the middle: an entry's step lands on a row drawn at the log's top.
        presses(&mut h, &mut app, Key::ArrowUp, 3);
        settle(&mut h, &mut app);
        let up = whole_rows(&mut h, &mut app);
        assert_ne!(
            up.last(),
            last.last(),
            "maximised {maximised}: ↑ did not scroll"
        );
    }
}

// --- REQ-GUI-216: the harness ---------------------------------------------------------------------------------------

fn row_of(frame: &'static str, layout_ms: f64) -> Row {
    Row {
        frame,
        times: vec![FrameTime {
            layout_ms,
            ..FrameTime::default()
        }],
    }
}

#[test]
fn qa_m6_44_the_harness_measures_the_three_sizes_and_judges_against_one_frame() {
    assert_eq!(bench::LOG_SIZES, SIZES, "R-463 K4's sizes");
    assert_eq!(CONSOLE_ROWS, SIZES[2], "CONSOLE_ROWS");
    assert!(
        (bench::TARGET_MS - 1000.0 / 60.0).abs() < 1e-9,
        "one frame at 60 fps"
    );
    // Its verdict can fail, at any frame; and the text says so.
    for (ms, met) in [(16.0, true), (16.7, false), (20.0, false)] {
        let table = Table {
            log: 0,
            rows: vec![row_of("open: click", 1.0), row_of("close: click", ms)],
        };
        assert_eq!(table.meets_target(), met, "{ms} ms");
        assert_eq!(
            bench::render_table(&table).contains("MISSED"),
            !met,
            "{ms} ms"
        );
    }
    // Run off the GPU, a cycle at each size: every frame of the open and the close, its time broken down, and the
    // console's filter and rows timed only while it is drawn.
    let tables = bench::measure(&SIZES, 1, None);
    assert_eq!(tables.iter().map(|t| t.log).collect::<Vec<_>>(), SIZES);
    for table in &tables {
        let frames: Vec<&str> = table.rows.iter().map(|r| r.frame).collect();
        for needed in ["open: click", "close: click"] {
            assert!(frames.contains(&needed), "{needed} in {frames:?}");
        }
        let time = |frame: &str| {
            table
                .rows
                .iter()
                .find(|r| r.frame == frame)
                .expect("measured")
                .slowest()
        };
        let (open, closed) = (time("open: click"), time("closed, before"));
        assert!(open.total_ms() > 0.0, "log {}: nothing timed", table.log);
        assert!(
            open.filter_ms > 0.0,
            "log {}: the open frame's filter",
            table.log
        );
        assert!(
            open.rows_ms > 0.0,
            "log {}: the open frame's rows",
            table.log
        );
        assert_eq!(
            closed.filter_ms, 0.0,
            "log {}: filtered while closed",
            table.log
        );
        assert_eq!(closed.rows_ms, 0.0, "log {}: rows while closed", table.log);
        assert!(
            closed.layout_ms > 0.0,
            "log {}: the rest of egui",
            table.log
        );
        assert_eq!(
            time("close: +1").rows_ms,
            0.0,
            "log {}: rows after the close",
            table.log
        );
        rejects("a console drawn while closed", || {
            assert_eq!(open.rows_ms, 0.0)
        });
    }
}

// --- The contract ----------------------------------------------------------------------------------------------------

#[test]
fn qa_m6_44_the_height_is_never_the_contracts() {
    // gui_state_contract §2, R-462 J5: the GUI's own settings. Dragging and maximising leave the contract's `ViewUI`
    // focus and mode, its page and its open windows as they were.
    let (mut app, mut h) = opened();
    let keep = |app: &App<MockSide>| -> (Focus, Mode, Vec<Window>) {
        (
            app.view.focus.clone(),
            app.view.mode,
            app.view.windows.open.clone(),
        )
    };
    let before = keep(&app);
    let snapshot = app.snapshot().clone();
    drag_edge(&mut h, &mut app, -100.0);
    m(&mut h, &mut app);
    assert_eq!(keep(&app), before, "the height reached ViewUI");
    assert_eq!(
        app.snapshot().sim,
        snapshot.sim,
        "the height reached the snapshot"
    );
}
