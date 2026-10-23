//! QA tests for TASK-M6-42, written from R-461 and its two requirements, not from the implementation:
//!
//! - REQ-GUI-211 (R-461 I1 to I3, I5; render_gui_spec §G5): on the mock, every window but the console (Run, Chart
//!   builder, Export & share, Display, Profiler, Inspector) is dragged by its title bar (anywhere on it off its ×, its
//!   title included) and resized from each edge and corner, down to a minimum size that keeps its title bar and its ×
//!   inside it; it stays at least partly on screen, its title bar's height and 64 points of its width inside the GUI,
//!   after a drag off any side and after the GUI's window shrinks, so it can always be dragged back by what is left
//!   of its title bar. The console is docked: a drag on its header bar does not move it.
//! - REQ-GUI-212 (R-461 I2, I4; gui_state_contract §2): each window's place and size, per window, survive a restart
//!   from the GUI's own settings; a remembered place off a smaller screen is clamped; with no settings each window
//!   opens where a GUI with no store opens it; an unreadable file (not JSON, the wrong shape, truncated, not UTF-8)
//!   is ignored with exactly one console line however long the session runs, and replaced on the next save, a GUI
//!   started from the replacement logging nothing; no drag reaches the contract's snapshot or its history: an undo
//!   after a drag undoes the edit before it and leaves the window where it was dragged.
//!
//! Every store here is a file under a scratch directory of the system's temporary directory, removed after: no test
//! reads or writes the user's config directory. Each check runs its own control (R-176) through [`rejects`]: a window
//! fixed in place, one not resizable, one wholly off screen, a GUI that forgets its places across a restart, a second
//! console line, and a snapshot an edit did reach.
// The file name `qa_TASK-M6-42` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::path::{Path, PathBuf};

use eframe::egui::{pos2, vec2, Event, Modifiers, PointerButton, Pos2, Rect, Vec2};
use engine::contract::log::Severity;
use engine::contract::render_state::Playhead;
use engine::contract::set_field::{Edit, RenderField, SetField};
use engine::contract::view_ui::Window;
use gui::app::App;
use gui::capture::{PIXELS_PER_POINT, SIZE};
use gui::headless::Headless;
use gui::mock::MockEngine;
use gui::settings::Store;
use gui::side::MockSide;
use gui::windows::entry;
use gui::windows::place::{min_size, TITLE_BAR_H};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// R-461 I1: the six windows that move; the console is docked.
const MOVABLE: [Window; 6] = [
    Window::Run,
    Window::ChartBuilder,
    Window::Export,
    Window::Display,
    Window::Profiler,
    Window::Inspector,
];

/// R-461 I2: the points of a window's title bar's width that stay inside the GUI.
const KEEP: f32 = 64.0;

/// egui rounds a window's edges to whole pixels: one point covers it at 1.5 pixels per point.
const TOL: f32 = 1.0;

/// The four edges and four corners, as (left, right, top, bottom).
const GRIPS: [(bool, bool, bool, bool); 8] = [
    (true, false, false, false),
    (false, true, false, false),
    (false, false, true, false),
    (false, false, false, true),
    (true, false, true, false),
    (false, true, true, false),
    (true, false, false, true),
    (false, true, false, true),
];

/// Asserts that `check` panics: the check can fail (R-176's control, run in the test).
fn rejects(what: &str, check: impl FnOnce()) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check));
    assert!(
        outcome.is_err(),
        "the check passed on {what}, which it must reject"
    );
}

/// A scratch directory of its own for `name`, empty.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("qa-m6-42-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

fn side() -> MockSide {
    MockSide::new(MockEngine::frozen(), None)
}

/// A GUI with no store: the tests' default, never the user's config directory.
fn plain() -> App<MockSide> {
    App::new(side(), FORMAT)
}

/// A GUI whose settings are kept in the file at `path`.
fn stored(path: &Path) -> App<MockSide> {
    App::with_settings(side(), FORMAT, Store::at(path))
}

fn headless_px(size: [u32; 2]) -> Headless {
    Headless::new(size, PIXELS_PER_POINT)
}

fn settle(h: &mut Headless, app: &mut App<MockSide>) {
    for _ in 0..4 {
        let _ = h.frame(app, Vec::new());
    }
}

fn open(h: &mut Headless, app: &mut App<MockSide>, window: Window) {
    let _ = h.frame(app, Vec::new());
    app.open_window(&h.ctx, window, None);
    settle(h, app);
}

/// Where `window` was drawn last frame.
fn drawn(app: &App<MockSide>, window: Window) -> Rect {
    app.keyboard
        .place_of(entry(window).scope)
        .unwrap_or_else(|| panic!("{window:?} is drawn"))
        .rect
}

/// Where `window`'s × was drawn last frame.
fn close_button(app: &App<MockSide>, window: Window) -> Rect {
    app.keyboard
        .place_of(entry(window).close)
        .unwrap_or_else(|| panic!("{window:?}'s × is drawn"))
        .rect
}

/// A pointer's primary drag from `from` to `to` in a few steps, released, then frames to settle.
fn drag(h: &mut Headless, app: &mut App<MockSide>, from: Pos2, to: Pos2) {
    let button = |pos, pressed| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    let mut frames = vec![vec![Event::PointerMoved(from)], vec![button(from, true)]];
    for k in 1..=4 {
        frames.push(vec![Event::PointerMoved(
            from + (to - from) * (k as f32 / 4.0),
        )]);
    }
    frames.push(vec![button(to, false)]);
    for events in frames {
        let _ = h.frame(app, events);
    }
    settle(h, app);
}

/// A point on the title bar of the window at `r`: `at` points from its left, in the middle of the title row.
fn on_title(r: Rect, at: f32) -> Pos2 {
    pos2(r.min.x + at, r.min.y + 14.0)
}

/// Moves `window` by a drag on its title bar, `at` points from its left, by `by`.
fn drag_title(h: &mut Headless, app: &mut App<MockSide>, window: Window, at: f32, by: Vec2) {
    let from = on_title(drawn(app, window), at);
    drag(h, app, from, from + by);
}

/// Moves `window` so its centre is at the screen's centre, by its title bar.
fn centre(h: &mut Headless, app: &mut App<MockSide>, window: Window) -> Rect {
    let r = drawn(app, window);
    drag_title(h, app, window, 30.0, h.screen().center() - r.center());
    drawn(app, window)
}

/// The point of the grip `(l, r, t, b)` of the window at `rect`: 2 points inside an edge's middle, or 3 inside a corner.
fn grip_point(rect: Rect, (l, r, t, b): (bool, bool, bool, bool)) -> Pos2 {
    let x = if l {
        rect.min.x + if t || b { 3.0 } else { 2.0 }
    } else if r {
        rect.max.x - if t || b { 3.0 } else { 2.0 }
    } else {
        rect.center().x
    };
    let y = if t {
        rect.min.y + if l || r { 3.0 } else { 2.0 }
    } else if b {
        rect.max.y - if l || r { 3.0 } else { 2.0 }
    } else {
        rect.center().y
    };
    pos2(x, y)
}

/// The outward direction of the grip.
fn outward((l, r, t, b): (bool, bool, bool, bool)) -> Vec2 {
    vec2(
        if l {
            -1.0
        } else if r {
            1.0
        } else {
            0.0
        },
        if t {
            -1.0
        } else if b {
            1.0
        } else {
            0.0
        },
    )
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() <= TOL
}

/// The check: the window moved from `before` to `after` by the drag `by`, its size kept.
fn assert_moved(before: Rect, after: Rect, by: Vec2, what: &str) {
    let want = before.translate(by);
    assert!(
        near(after.min.x, want.min.x)
            && near(after.min.y, want.min.y)
            && near(after.width(), want.width())
            && near(after.height(), want.height()),
        "{what}: {before:?} dragged by {by:?} is at {after:?}, want {want:?}"
    );
}

/// The check: the grip's edges moved by `by`, the other edges stayed.
fn assert_resized(before: Rect, after: Rect, g: (bool, bool, bool, bool), by: Vec2, what: &str) {
    let (l, r, t, b) = g;
    let want = Rect::from_min_max(
        pos2(
            before.min.x + if l { by.x } else { 0.0 },
            before.min.y + if t { by.y } else { 0.0 },
        ),
        pos2(
            before.max.x + if r { by.x } else { 0.0 },
            before.max.y + if b { by.y } else { 0.0 },
        ),
    );
    assert!(
        near(after.min.x, want.min.x)
            && near(after.min.y, want.min.y)
            && near(after.max.x, want.max.x)
            && near(after.max.y, want.max.y),
        "{what}: {before:?} resized by {g:?} dragged {by:?} is {after:?}, want {want:?}"
    );
}

/// The check (R-461 I2): the window at `r` keeps its title bar's height and 64 points of its width inside `screen`.
fn assert_partly_on_screen(r: Rect, screen: Rect, what: &str) {
    let inside_w = r.max.x.min(screen.max.x) - r.min.x.max(screen.min.x);
    let bar_bottom = r.min.y + TITLE_BAR_H;
    let inside_h = bar_bottom.min(screen.max.y) - r.min.y.max(screen.min.y);
    assert!(
        inside_w >= KEEP - TOL,
        "{what}: {r:?} keeps {inside_w} points of its width on {screen:?}"
    );
    assert!(
        inside_h >= TITLE_BAR_H - TOL,
        "{what}: {r:?} keeps {inside_h} points of its title bar's height on {screen:?}"
    );
}

// --- REQ-GUI-211 ---------------------------------------------------------------------------------------------------

/// The title bar band the on-screen rule keeps is the band the title row is drawn in: the × lies inside it.
#[test]
fn qa_m6_42_the_kept_title_bar_holds_the_title_row() {
    let (mut app, mut h) = (plain(), headless_px(SIZE));
    for window in MOVABLE {
        open(&mut h, &mut app, window);
        let r = drawn(&app, window);
        let x = close_button(&app, window);
        assert!(
            x.min.y >= r.min.y - TOL && x.max.y <= r.min.y + TITLE_BAR_H + TOL,
            "{window:?}: its × {x:?} is outside its title bar's {TITLE_BAR_H} points at the top of {r:?}"
        );
        assert!(
            x.max.x <= r.max.x + TOL && x.min.x >= r.min.x,
            "{window:?}: its × {x:?} is outside {r:?}"
        );
        app.close_window(&h.ctx, window);
        settle(&mut h, &mut app);
    }
}

/// A drag anywhere on the title bar off the ×, its left end or its centred title, moves the window by the drag.
#[test]
fn qa_m6_42_a_title_bar_drag_moves_each_window_by_the_drag() {
    let (mut app, mut h) = (plain(), headless_px(SIZE));
    for window in MOVABLE {
        open(&mut h, &mut app, window);
        let start = centre(&mut h, &mut app, window);
        for (at, by) in [
            (20.0, vec2(-73.0, 41.0)),
            (start.width() / 2.0, vec2(55.0, -38.0)),
            (40.0, vec2(-5.0, 90.0)),
        ] {
            let before = drawn(&app, window);
            drag_title(&mut h, &mut app, window, at, by);
            let after = drawn(&app, window);
            assert_moved(before, after, by, &format!("{window:?} at {at}"));
            // Control: a window fixed in place, as before R-461.
            rejects("a window fixed in place", || {
                assert_moved(before, before, by, "fixed")
            });
        }
        app.close_window(&h.ctx, window);
        settle(&mut h, &mut app);
    }
}

/// A drag on the body of a window, off its title bar and edges, does not move it: only the title bar does.
#[test]
fn qa_m6_42_a_drag_on_a_windows_body_does_not_move_it() {
    let (mut app, mut h) = (plain(), headless_px(SIZE));
    let window = Window::Profiler;
    open(&mut h, &mut app, window);
    let before = centre(&mut h, &mut app, window);
    // Well below the title bar and away from every edge: the lower middle of the window, which has no slider there
    // on the frozen mock (a drag there could only scroll).
    let from = pos2(before.min.x + 20.0, before.max.y - 20.0);
    drag(&mut h, &mut app, from, from + vec2(60.0, 60.0));
    let after = drawn(&app, window);
    assert!(
        near(after.min.x, before.min.x) && near(after.min.y, before.min.y),
        "a drag on the body moved the window from {before:?} to {after:?}"
    );
}

/// A drag on each edge and corner resizes the window by the drag, the other edges kept; a drag far inward stops at
/// a minimum size that is the same from every side, keeps its title bar and its ×, and a further drag does not pass it.
#[test]
fn qa_m6_42_each_edge_and_corner_resizes_and_stops_at_the_minimum() {
    let (mut app, mut h) = (plain(), headless_px(SIZE));
    for window in MOVABLE {
        open(&mut h, &mut app, window);
        for g in GRIPS {
            let before = centre(&mut h, &mut app, window);
            // Outward, then back, by the drag.
            let by = outward(g) * 30.0;
            drag(
                &mut h,
                &mut app,
                grip_point(before, g),
                grip_point(before, g) + by,
            );
            let grown = drawn(&app, window);
            assert_resized(before, grown, g, by, &format!("{window:?} {g:?} out"));
            rejects("a window not resizable", || {
                assert_resized(before, before, g, by, "not resizable")
            });
            drag(
                &mut h,
                &mut app,
                grip_point(grown, g),
                grip_point(grown, g) - by,
            );
            let back = drawn(&app, window);
            assert_resized(grown, back, g, -by, &format!("{window:?} {g:?} back"));

            // Far inward: past the opposite edge. The edges not held stay put.
            let far = -outward(g) * 2000.0;
            let to = grip_point(back, g) + far;
            let to = pos2(
                to.x.clamp(1.0, h.screen().max.x - 1.0),
                to.y.clamp(1.0, h.screen().max.y - 1.0),
            );
            drag(&mut h, &mut app, grip_point(back, g), to);
            let small = drawn(&app, window);
            let (l, r, t, b) = g;
            if !l {
                assert!(
                    near(small.min.x, back.min.x),
                    "{window:?} {g:?}: the left edge moved"
                );
            }
            if !r {
                assert!(
                    near(small.max.x, back.max.x),
                    "{window:?} {g:?}: the right edge moved"
                );
            }
            if !t {
                assert!(
                    near(small.min.y, back.min.y),
                    "{window:?} {g:?}: the top edge moved"
                );
            }
            if !b {
                assert!(
                    near(small.max.y, back.max.y),
                    "{window:?} {g:?}: the bottom edge moved"
                );
            }
            let min = min_size(window);
            assert!(
                min.x >= KEEP && min.y >= TITLE_BAR_H,
                "{window:?}: a minimum {min:?} smaller than its title bar"
            );
            if l || r {
                assert!(
                    near(small.width(), min.x),
                    "{window:?} {g:?}: width {} at its minimum {}",
                    small.width(),
                    min.x
                );
            } else {
                assert!(
                    near(small.width(), back.width()),
                    "{window:?} {g:?}: the width changed"
                );
            }
            if t || b {
                assert!(
                    near(small.height(), min.y),
                    "{window:?} {g:?}: height {} at its minimum {}",
                    small.height(),
                    min.y
                );
            } else {
                assert!(
                    near(small.height(), back.height()),
                    "{window:?} {g:?}: the height changed"
                );
            }
            // The title bar and its × stay usable at the minimum: the × is drawn inside the window's title bar.
            let x = close_button(&app, window);
            assert!(
                x.min.x >= small.min.x - TOL
                    && x.max.x <= small.max.x + TOL
                    && x.max.y <= small.min.y + TITLE_BAR_H + TOL,
                "{window:?} {g:?}: its × {x:?} is outside its title bar at {small:?}"
            );
            // A further drag inward does not pass the minimum.
            let p = grip_point(small, g);
            let to = p - outward(g) * 40.0;
            drag(&mut h, &mut app, p, to);
            let again = drawn(&app, window);
            assert!(
                near(again.width(), small.width()) && near(again.height(), small.height()),
                "{window:?} {g:?}: {small:?} shrank past its minimum to {again:?}"
            );
            // Restore a working size for the next grip: grow back out from the same grip.
            let p = grip_point(again, g);
            drag(&mut h, &mut app, p, p + outward(g) * 200.0);
        }
        app.close_window(&h.ctx, window);
        settle(&mut h, &mut app);
    }
}

/// A drag that would take a window off any side leaves its title bar's height and 64 points of its width inside, and
/// what is left of the title bar drags it back.
#[test]
fn qa_m6_42_a_drag_off_any_side_keeps_it_partly_on_screen_and_draggable_back() {
    let (mut app, mut h) = (plain(), headless_px(SIZE));
    let screen = h.screen();
    for window in MOVABLE {
        open(&mut h, &mut app, window);
        for by in [
            vec2(-3000.0, 0.0),
            vec2(3000.0, 0.0),
            vec2(0.0, -3000.0),
            vec2(0.0, 3000.0),
            vec2(-3000.0, -3000.0),
            vec2(3000.0, 3000.0),
            vec2(-3000.0, 3000.0),
            vec2(3000.0, -3000.0),
        ] {
            centre(&mut h, &mut app, window);
            drag_title(&mut h, &mut app, window, 30.0, by);
            let off = drawn(&app, window);
            assert_partly_on_screen(off, screen, &format!("{window:?} dragged {by:?}"));
            // Control: the window wholly off screen.
            rejects("a window dragged wholly off screen", || {
                assert_partly_on_screen(off.translate(by), screen, "wholly off")
            });
            // What is left of its title bar drags it back.
            let left = off.min.x.max(screen.min.x) + 10.0;
            let from = pos2(left, off.min.y.max(screen.min.y) + 14.0);
            let back = screen.center() - from;
            drag(&mut h, &mut app, from, from + back);
            let after = drawn(&app, window);
            assert_moved(
                off,
                after,
                back,
                &format!("{window:?} dragged back from {by:?}"),
            );
        }
        app.close_window(&h.ctx, window);
        settle(&mut h, &mut app);
    }
}

/// A resize that drags the top edge off the top of the GUI, or a corner far off, keeps the window partly on screen.
#[test]
fn qa_m6_42_a_resize_off_screen_keeps_the_title_bar_on_screen() {
    let (mut app, mut h) = (plain(), headless_px(SIZE));
    let screen = h.screen();
    for window in MOVABLE {
        open(&mut h, &mut app, window);
        for g in [GRIPS[2], GRIPS[4], GRIPS[5]] {
            let r = centre(&mut h, &mut app, window);
            let p = grip_point(r, g);
            drag(&mut h, &mut app, p, p + outward(g) * 3000.0);
            assert_partly_on_screen(drawn(&app, window), screen, &format!("{window:?} {g:?}"));
        }
        app.close_window(&h.ctx, window);
        settle(&mut h, &mut app);
    }
}

/// Shrinking the GUI's window keeps every window partly on screen, wherever it was.
#[test]
fn qa_m6_42_shrinking_the_gui_keeps_each_window_partly_on_screen() {
    for corner in [vec2(3000.0, 3000.0), vec2(3000.0, 0.0), vec2(0.0, 3000.0)] {
        let (mut app, mut h) = (plain(), headless_px(SIZE));
        for window in MOVABLE {
            open(&mut h, &mut app, window);
            centre(&mut h, &mut app, window);
            drag_title(&mut h, &mut app, window, 30.0, corner);
            let small = [SIZE[0] / 2, SIZE[1] / 2];
            h.set_size(small);
            settle(&mut h, &mut app);
            let screen = h.screen();
            assert!(screen.max.x < 1000.0, "the GUI shrank");
            assert_partly_on_screen(
                drawn(&app, window),
                screen,
                &format!("{window:?} toward {corner:?}"),
            );
            h.set_size(SIZE);
            app.close_window(&h.ctx, window);
            settle(&mut h, &mut app);
        }
    }
}

/// The console is docked (R-461 I1, R-463): a drag on its header bar does not move it sideways or up.
#[test]
fn qa_m6_42_the_console_is_not_moved_by_a_drag_on_its_header() {
    let (mut app, mut h) = (plain(), headless_px(SIZE));
    open(&mut h, &mut app, Window::Console);
    let before = drawn(&app, Window::Console);
    let from = pos2(before.min.x + 8.0, before.min.y + 14.0);
    drag(&mut h, &mut app, from, from + vec2(200.0, -150.0));
    let after = drawn(&app, Window::Console);
    assert!(
        near(after.min.x, before.min.x)
            && near(after.max.x, before.max.x)
            && near(after.max.y, before.max.y),
        "the console moved from {before:?} to {after:?}"
    );
}

/// The keyboard's model of a window follows it (I5): after a move, its scope and its × are placed where it is drawn.
#[test]
fn qa_m6_42_the_keyboard_places_follow_a_moved_window() {
    let (mut app, mut h) = (plain(), headless_px(SIZE));
    for window in MOVABLE {
        open(&mut h, &mut app, window);
        let before = centre(&mut h, &mut app, window);
        let x0 = close_button(&app, window);
        drag_title(&mut h, &mut app, window, 30.0, vec2(-100.0, 60.0));
        let x1 = close_button(&app, window);
        assert_moved(x0, x1, vec2(-100.0, 60.0), &format!("{window:?}'s ×"));
        assert_moved(
            before,
            drawn(&app, window),
            vec2(-100.0, 60.0),
            &format!("{window:?}"),
        );
        app.close_window(&h.ctx, window);
        settle(&mut h, &mut app);
    }
}

/// The console's lines other than the applied `SetField`s' echoes (each applied `SetField` is an Info line, the
/// contract's log; a pointer moving over the figure during a drag sends hovers): what reports on the settings.
fn reports(app: &App<MockSide>) -> usize {
    app.console()
        .iter()
        .filter(|e| !(e.severity == Severity::Info && e.message.starts_with("SetField")))
        .count()
}

// --- REQ-GUI-212 ---------------------------------------------------------------------------------------------------

/// Moves `window` by its title bar and resizes it from its bottom-right corner; returns where it is.
fn move_and_resize(
    h: &mut Headless,
    app: &mut App<MockSide>,
    window: Window,
    by: Vec2,
    grow: Vec2,
) -> Rect {
    centre(h, app, window);
    drag_title(h, app, window, 30.0, by);
    let r = drawn(app, window);
    let p = grip_point(r, GRIPS[7]);
    drag(h, app, p, p + grow);
    drawn(app, window)
}

/// Where `window` opens in `app` on a GUI of `size` pixels.
fn opens_at(app: &mut App<MockSide>, size: [u32; 2], window: Window) -> Rect {
    let mut h = headless_px(size);
    open(&mut h, app, window);
    drawn(app, window)
}

fn same(a: Rect, b: Rect, what: &str) {
    assert!(
        near(a.min.x, b.min.x)
            && near(a.min.y, b.min.y)
            && near(a.max.x, b.max.x)
            && near(a.max.y, b.max.y),
        "{what}: {a:?} is not {b:?}"
    );
}

/// Each window, moved and resized differently, opens in a new GUI started from the saved settings at its own place
/// and size; control: a GUI that forgets the places across a restart opens them elsewhere.
#[test]
fn qa_m6_42_each_window_opens_where_it_was_left_in_a_new_gui() {
    let dir = scratch("restart");
    let path = dir.join("nested").join("settings.json");
    let mut left = Vec::new();
    {
        let mut app = stored(&path);
        let mut h = headless_px(SIZE);
        for (i, window) in MOVABLE.into_iter().enumerate() {
            open(&mut h, &mut app, window);
            let k = i as f32;
            let r = move_and_resize(
                &mut h,
                &mut app,
                window,
                vec2(-200.0 + 70.0 * k, -120.0 + 45.0 * k),
                vec2(25.0 + 7.0 * k, 15.0 + 9.0 * k),
            );
            left.push((window, r));
        }
        app.save_settings();
    }
    for (window, r) in &left {
        let mut again = stored(&path);
        same(
            opens_at(&mut again, SIZE, *window),
            *r,
            &format!("{window:?} after a restart"),
        );
        // Control: a GUI that forgets the place across a restart.
        let mut forgot = plain();
        let elsewhere = opens_at(&mut forgot, SIZE, *window);
        rejects("a GUI that forgets the place", || {
            same(elsewhere, *r, "forgot")
        });
    }
    // And all of them open at once in one new GUI, each at its own place.
    let mut again = stored(&path);
    let mut h = headless_px(SIZE);
    for (window, _) in &left {
        open(&mut h, &mut again, *window);
    }
    for (window, r) in &left {
        same(
            drawn(&again, *window),
            *r,
            &format!("{window:?} among the six"),
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// The places are saved without an explicit save, as the GUI is used: a drag that has ended is remembered by a GUI
/// started next (R-461 "remember position and size across sessions": the session can end at any time).
#[test]
fn qa_m6_42_a_finished_drag_is_remembered_without_an_explicit_save() {
    let dir = scratch("autosave");
    let path = dir.join("settings.json");
    let r = {
        let mut app = stored(&path);
        let mut h = headless_px(SIZE);
        open(&mut h, &mut app, Window::Run);
        move_and_resize(
            &mut h,
            &mut app,
            Window::Run,
            vec2(80.0, -60.0),
            vec2(40.0, 30.0),
        )
    };
    let mut again = stored(&path);
    same(
        opens_at(&mut again, SIZE, Window::Run),
        r,
        "Run after a restart",
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// A remembered place off a smaller screen is clamped on it as R-461 I2 says.
#[test]
fn qa_m6_42_a_remembered_place_off_a_smaller_screen_is_clamped() {
    let dir = scratch("smaller");
    let path = dir.join("settings.json");
    {
        let mut app = stored(&path);
        let mut h = headless_px(SIZE);
        for window in MOVABLE {
            open(&mut h, &mut app, window);
            centre(&mut h, &mut app, window);
            drag_title(&mut h, &mut app, window, 30.0, vec2(3000.0, 3000.0));
            let r = drawn(&app, window);
            assert!(
                r.min.x > 700.0 && r.min.y > 500.0,
                "{window:?} is near the bottom right: {r:?}"
            );
        }
        app.save_settings();
    }
    // Half the size: smaller GUIs trip an egui debug assertion in the Manifold view's chart, with no window open,
    // before this task.
    let small = [SIZE[0] / 2, SIZE[1] / 2];
    for window in MOVABLE {
        let mut again = stored(&path);
        let mut h = headless_px(small);
        open(&mut h, &mut again, window);
        let r = drawn(&again, window);
        assert_partly_on_screen(r, h.screen(), &format!("{window:?} on half of the screen"));
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// With no settings each window opens where a GUI with no store opens it, today's place: a missing file, and an empty
/// settings object, alike.
#[test]
fn qa_m6_42_with_no_settings_each_window_opens_at_todays_place() {
    let dir = scratch("none");
    let missing = dir.join("missing.json");
    let empty = dir.join("empty.json");
    std::fs::write(&empty, "{}").expect("write");
    for window in MOVABLE {
        let today = opens_at(&mut plain(), SIZE, window);
        let mut a = stored(&missing);
        same(
            opens_at(&mut a, SIZE, window),
            today,
            &format!("{window:?} with no file"),
        );
        let mut b = stored(&empty);
        same(
            opens_at(&mut b, SIZE, window),
            today,
            &format!("{window:?} with an empty file"),
        );
        assert_eq!(reports(&a), reports(&plain()), "no line for a missing file");
        assert_eq!(
            reports(&b),
            reports(&plain()),
            "no line for an empty object"
        );
    }
    assert!(
        !missing.exists(),
        "nothing is written before a window is moved"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// An unreadable settings file is ignored (each window at today's place) with exactly one console line however long
/// the session runs and however many drags are saved, and is replaced on the next save: a GUI started from the
/// replacement logs nothing and opens the window where it was left.
#[test]
fn qa_m6_42_an_unreadable_file_is_ignored_with_one_line_and_replaced() {
    let dir = scratch("corrupt");
    let bad: [(&str, &[u8]); 5] = [
        ("not json", b"this is not json"),
        ("truncated", b"{\"windows\": {\"Profiler\": {\"x\": 10"),
        ("wrong shape", b"{\"windows\": 5}"),
        ("not an object", b"[1, 2, 3]"),
        ("not utf-8", &[0xff, 0xfe, 0x00, 0x9f]),
    ];
    let window = Window::Profiler;
    let today = opens_at(&mut plain(), SIZE, window);
    for (what, bytes) in bad {
        let path = dir.join(format!("{}.json", what.replace(' ', "_")));
        std::fs::write(&path, bytes).expect("write");
        let baseline = {
            let mut a = plain();
            let mut h = headless_px(SIZE);
            open(&mut h, &mut a, window);
            reports(&a)
        };
        let mut app = stored(&path);
        let mut h = headless_px(SIZE);
        open(&mut h, &mut app, window);
        same(
            drawn(&app, window),
            today,
            &format!("{what}: the window at today's place"),
        );
        let one = |app: &App<MockSide>| {
            assert_eq!(
                reports(app),
                baseline + 1,
                "{what}: one console line for the unreadable file"
            )
        };
        one(&app);
        // Many frames and saved drags later, still one line.
        let r = move_and_resize(
            &mut h,
            &mut app,
            window,
            vec2(-90.0, 70.0),
            vec2(30.0, 20.0),
        );
        settle(&mut h, &mut app);
        app.save_settings();
        one(&app);
        // Replaced: a new GUI reads it without a line and opens the window where it was left.
        let mut again = stored(&path);
        let mut h2 = headless_px(SIZE);
        open(&mut h2, &mut again, window);
        assert_eq!(
            reports(&again),
            baseline,
            "{what}: the replaced file is read without a line"
        );
        // Control: the one-line check rejects a GUI that logged none.
        rejects("a GUI that logged no line", || one(&again));
        same(
            drawn(&again, window),
            r,
            &format!("{what}: after the file was replaced"),
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// The places never reach the contract (gui_state_contract §2): drags leave the snapshot's sim, render and history as
/// they were, and an undo after a drag undoes the edit before it, leaving the window where it was dragged.
#[test]
fn qa_m6_42_no_window_place_reaches_the_snapshot_or_the_history() {
    let dir = scratch("contract");
    let path = dir.join("settings.json");
    let mut app = stored(&path);
    let mut h = headless_px(SIZE);
    let window = Window::Display;
    open(&mut h, &mut app, window);
    let start = app.snapshot().clone();
    // One edit, in the history.
    app.set_field(SetField {
        edit: Edit::Render(RenderField::Playhead(Playhead { t: 1.25 })),
        no_history: false,
    });
    settle(&mut h, &mut app);
    let edited = app.snapshot().clone();
    let unchanged = |s: &engine::contract::snapshot::Snapshot| {
        assert_eq!(s.sim, edited.sim);
        assert_eq!(s.render, edited.render);
        assert_eq!(s.history, edited.history);
    };
    // Control: the edit did reach the contract.
    rejects("a snapshot an edit reached", || {
        assert_eq!(start.render, edited.render);
        assert_eq!(start.history, edited.history);
    });
    let moved = move_and_resize(
        &mut h,
        &mut app,
        window,
        vec2(120.0, -80.0),
        vec2(30.0, 25.0),
    );
    settle(&mut h, &mut app);
    unchanged(app.snapshot());
    // Undo steps back over the edit, not the drag.
    app.undo();
    settle(&mut h, &mut app);
    assert_eq!(app.snapshot().render, start.render, "undo undid the edit");
    same(drawn(&app, window), moved, "the window after an undo");
    app.redo();
    settle(&mut h, &mut app);
    assert_eq!(app.snapshot().render, edited.render, "redo redid the edit");
    same(drawn(&app, window), moved, "the window after a redo");
    let _ = std::fs::remove_dir_all(&dir);
}
