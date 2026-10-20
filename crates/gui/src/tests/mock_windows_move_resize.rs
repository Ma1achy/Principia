//! REQ-GUI-211 (R-461 I1 to I3, I5; render_gui_spec §G5): on the mock, each of the six windows but the console, Run,
//! Chart builder, Export & share, Display, Profiler and the Inspector, driven by pointer scripts: a drag on its title
//! bar moves it by the drag; a drag on each edge and corner resizes it, stopped at its minimum size; a drag that would
//! take it off any side of the GUI leaves its title bar's height and 64 points of its width inside, and so does
//! shrinking the GUI's window. The console is docked and does not move. Each check also runs on an outcome it must
//! reject (R-176, through `rejects`): a window fixed in place, one not resizable, one dragged wholly off screen.

use eframe::egui::{self, pos2, vec2, CursorIcon, Event, PointerButton, Pos2, Rect, Vec2};
use engine::contract::view_ui::Window;

use super::support::{headless, mock_app, rejects};
use crate::app::App;
use crate::headless::Headless;
use crate::layout::Layout;
use crate::side::MockSide;
use crate::windows::place::{
    self, min_size, on_screen, resized, title_bar, Edges, Grip, Places, CORNER_GRAB, EDGE_GRAB,
    KEEP_W, TITLE_BAR_H,
};
use crate::windows::{self as wins, entry};

/// The six windows that move (R-461 I1).
pub(super) const MOVABLE: [Window; 6] = [
    Window::Run,
    Window::ChartBuilder,
    Window::Export,
    Window::Display,
    Window::Profiler,
    Window::Inspector,
];

const NONE: egui::Modifiers = egui::Modifiers::NONE;

/// A few frames with no input.
pub(super) fn settle(h: &mut Headless, app: &mut App<MockSide>) {
    for _ in 0..3 {
        let _ = h.frame(app, Vec::new());
    }
}

/// Opens `window` and lets it settle.
pub(super) fn open(h: &mut Headless, app: &mut App<MockSide>, window: Window) {
    let _ = h.frame(app, Vec::new());
    app.open_window(&h.ctx, window, None);
    settle(h, app);
}

/// Where `window` was drawn last frame: its scope's place.
pub(super) fn drawn(app: &App<MockSide>, window: Window) -> Rect {
    app.keyboard
        .place_of(entry(window).scope)
        .unwrap_or_else(|| panic!("{window:?} placed"))
        .rect
}

/// Where `window` is, by its place on `h`'s screen, after checking that it was drawn there, to a point (egui rounds a
/// window's edges to whole pixels, and lays a window partly off screen out a fraction of a point wider).
pub(super) fn placed(h: &Headless, app: &App<MockSide>, window: Window) -> Rect {
    let layout = Layout::new(h.screen(), h.pixels_per_point());
    let rect = app
        .places
        .shown(window, wins::rect(window, &layout), h.screen());
    let px = 1.0;
    let d = drawn(app, window);
    assert!(
        (d.min - rect.min).abs().max_elem() <= px && (d.max - rect.max).abs().max_elem() <= px,
        "{window:?} drawn at {d:?}, placed at {rect:?}"
    );
    rect
}

/// A primary drag from `from` to `to`, through the halfway point, then a frame to settle.
pub(super) fn drag(h: &mut Headless, app: &mut App<MockSide>, from: Pos2, to: Pos2) {
    let button = |pos, pressed| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: NONE,
    };
    for events in [
        vec![Event::PointerMoved(from)],
        vec![button(from, true)],
        vec![Event::PointerMoved(from + (to - from) / 2.0)],
        vec![Event::PointerMoved(to)],
        vec![button(to, false)],
        Vec::new(),
        Vec::new(),
    ] {
        let _ = h.frame(app, events);
    }
}

/// A point on `rect`'s title bar, off its ×: near its left end.
pub(super) fn on_title(rect: Rect) -> Pos2 {
    pos2(rect.min.x + 30.0, rect.min.y + TITLE_BAR_H / 2.0)
}

/// A drag on `window`'s title bar by `by`.
pub(super) fn drag_title(h: &mut Headless, app: &mut App<MockSide>, window: Window, by: Vec2) {
    let from = on_title(drawn(app, window));
    drag(h, app, from, from + by);
}

/// A point of `edges`' handle on `rect`, 2 points inside the window from the edge, or the corner.
fn on_handle(rect: Rect, edges: Edges) -> Pos2 {
    let c = edges.handle(rect).center();
    let inward = |v: f32, centre: f32| {
        if v < centre {
            2.0
        } else if v > centre {
            -2.0
        } else {
            0.0
        }
    };
    c + vec2(inward(c.x, rect.center().x), inward(c.y, rect.center().y))
}

/// A drag on `window`'s `edges` by `by`.
fn drag_edges(h: &mut Headless, app: &mut App<MockSide>, window: Window, edges: Edges, by: Vec2) {
    let from = on_handle(drawn(app, window), edges);
    drag(h, app, from, from + by);
}

fn close(a: Rect, b: Rect) -> bool {
    (a.min - b.min).length() < 0.01 && (a.max - b.max).length() < 0.01
}

/// The window moved from `before` to `after` by the drag `by`.
fn assert_moved(before: Rect, after: Rect, by: Vec2) {
    assert!(
        close(after, before.translate(by)),
        "{before:?} dragged by {by:?} is at {after:?}"
    );
}

/// The window resized from `before` to `after` by `edges` dragged by `by`: each of them moved by it, the others kept.
fn assert_resized(before: Rect, after: Rect, edges: Edges, by: Vec2) {
    let mut want = before;
    if edges.left {
        want.min.x += by.x;
    }
    if edges.right {
        want.max.x += by.x;
    }
    if edges.top {
        want.min.y += by.y;
    }
    if edges.bottom {
        want.max.y += by.y;
    }
    assert!(
        close(after, want),
        "{before:?}, {edges:?} dragged by {by:?}, is {after:?}, not {want:?}"
    );
}

/// The window at `rect` is at least partly on `screen` (R-461 I2): its title bar's height, and 64 points of its width,
/// inside.
pub(super) fn assert_on_screen(rect: Rect, screen: Rect) {
    let bar = title_bar(rect);
    let inside = bar.max.x.min(screen.max.x) - bar.min.x.max(screen.min.x);
    assert!(
        inside >= KEEP_W - 0.01,
        "{rect:?}: {inside} points of its title bar across {screen:?}"
    );
    assert!(
        bar.min.y >= screen.min.y - 0.01 && bar.max.y <= screen.max.y + 0.01,
        "{rect:?}: its title bar's height not inside {screen:?}"
    );
    assert!(
        (bar.height() - TITLE_BAR_H).abs() < 0.01,
        "{rect:?}: its title bar is not whole"
    );
}

#[test]
fn mock_windows_move_resize_a_title_bar_drag_moves_each_window_by_the_drag() {
    for window in MOVABLE {
        let mut app = mock_app();
        let mut h = headless();
        open(&mut h, &mut app, window);
        let before = placed(&h, &app, window);
        let by = vec2(30.0, 40.0);
        drag_title(&mut h, &mut app, window, by);
        let after = placed(&h, &app, window);
        assert_moved(before, after, by);
        // The keyboard's model is unchanged (I5): the window is still open, its scope placed where it is drawn.
        assert!(app.is_open(window));
        assert_eq!(app.places.get(window), Some(after), "{window:?}");
        rejects("a window fixed in place", || {
            assert_moved(before, before, by)
        });
    }
}

#[test]
fn mock_windows_move_resize_each_edge_and_corner_resizes_each_window() {
    for window in MOVABLE {
        let mut app = mock_app();
        let mut h = headless();
        open(&mut h, &mut app, window);
        for edges in Edges::ALL {
            let before = placed(&h, &app, window);
            // Toward the screen's middle across, and down: never meeting the window's minimum or the screen's sides.
            let toward = if before.center().x < h.screen().center().x {
                12.0
            } else {
                -12.0
            };
            let by = vec2(toward, 10.0);
            drag_edges(&mut h, &mut app, window, edges, by);
            let after = placed(&h, &app, window);
            assert_resized(before, after, edges, by);
            rejects("a window not resizable", || {
                assert_resized(before, before, edges, by)
            });
        }
    }
}

#[test]
fn mock_windows_move_resize_a_resize_stops_at_the_minimum() {
    for window in MOVABLE {
        let min = min_size(window);
        for edges in [
            Edges::of(false, true, false, true),
            Edges::of(true, false, true, false),
        ] {
            let mut app = mock_app();
            let mut h = headless();
            open(&mut h, &mut app, window);
            let before = placed(&h, &app, window);
            // Inward, far past the minimum.
            let by = vec2(
                if edges.left { 600.0 } else { -600.0 },
                if edges.top { 600.0 } else { -600.0 },
            );
            drag_edges(&mut h, &mut app, window, edges, by);
            let after = placed(&h, &app, window);
            assert!(
                (after.size() - min).length() < 0.01,
                "{window:?} {edges:?}: {:?}, its minimum {min:?}",
                after.size()
            );
            // The edges not held stay put.
            if edges.left {
                assert_eq!(after.max, before.max, "{window:?}");
            } else {
                assert_eq!(after.min, before.min, "{window:?}");
            }
            rejects("a resize past the minimum", || {
                let past = Rect::from_min_size(before.min, min - vec2(1.0, 1.0));
                assert!((past.size() - min).length() < 0.01)
            });
        }
    }
}

#[test]
fn mock_windows_move_resize_a_drag_off_any_side_leaves_the_title_bar_on_screen() {
    let sides = [
        vec2(-3000.0, 0.0),
        vec2(3000.0, 0.0),
        vec2(0.0, -3000.0),
        vec2(0.0, 3000.0),
    ];
    for window in MOVABLE {
        for by in sides {
            let mut app = mock_app();
            let mut h = headless();
            open(&mut h, &mut app, window);
            let screen = h.screen();
            let before = placed(&h, &app, window);
            drag_title(&mut h, &mut app, window, by);
            let after = placed(&h, &app, window);
            assert_on_screen(after, screen);
            assert!((after.size() - before.size()).length() < 0.01, "{window:?}");
            // As far as the clamp allows, on the side dragged to.
            let at = match (by.x < 0.0, by.x > 0.0, by.y < 0.0) {
                (true, _, _) => after.max.x - (screen.min.x + KEEP_W),
                (_, true, _) => (screen.max.x - KEEP_W) - after.min.x,
                (_, _, true) => after.min.y - screen.min.y,
                _ => (screen.max.y - TITLE_BAR_H) - after.min.y,
            };
            assert!(at.abs() < 0.01, "{window:?} by {by:?}: {after:?}");
            rejects("a window dragged wholly off screen", || {
                assert_on_screen(before.translate(by), screen)
            });
        }
    }
}

#[test]
fn mock_windows_move_resize_shrinking_the_gui_keeps_each_window_on_screen() {
    for window in MOVABLE {
        let mut app = mock_app();
        let mut h = headless();
        open(&mut h, &mut app, window);
        // Into the GUI's bottom-right corner, as far as it goes.
        drag_title(&mut h, &mut app, window, vec2(3000.0, 3000.0));
        let moved = placed(&h, &app, window);
        h.set_size([1200, 750]);
        settle(&mut h, &mut app);
        let screen = h.screen();
        let after = placed(&h, &app, window);
        assert_on_screen(after, screen);
        assert!((after.size() - moved.size()).length() < 0.01, "{window:?}");
        rejects("a window left where it was", || {
            assert_on_screen(moved, screen)
        });
    }
}

#[test]
fn mock_windows_move_resize_the_title_row_is_inside_the_title_bar() {
    for window in MOVABLE {
        let mut app = mock_app();
        let mut h = headless();
        open(&mut h, &mut app, window);
        let rect = drawn(&app, window);
        let x = app
            .keyboard
            .place_of(entry(window).close)
            .expect("the × placed")
            .rect;
        assert!(title_bar(rect).contains_rect(x), "{window:?}: × {x:?}");
        // A drag on the × moves nothing, and its click still closes the window.
        drag(&mut h, &mut app, x.center(), x.center() + vec2(-40.0, 30.0));
        assert_eq!(drawn(&app, window), rect, "{window:?}");
        super::support::click_scope(&mut h, &mut app, entry(window).close);
        assert!(!app.is_open(window), "{window:?}");
    }
}

#[test]
fn mock_windows_move_resize_the_console_is_docked() {
    let mut app = mock_app();
    let mut h = headless();
    open(&mut h, &mut app, Window::Console);
    let before = drawn(&app, Window::Console);
    let from = pos2(before.min.x + 30.0, before.min.y + 10.0);
    drag(&mut h, &mut app, from, from + vec2(40.0, -60.0));
    drag(
        &mut h,
        &mut app,
        pos2(before.max.x - 2.0, before.max.y - 2.0),
        pos2(before.max.x - 80.0, before.max.y - 80.0),
    );
    assert_eq!(drawn(&app, Window::Console), before);
    assert_eq!(app.places.get(Window::Console), None);
    rejects("a console that moved", || {
        assert_eq!(before.translate(vec2(40.0, -60.0)), before)
    });
}

#[test]
fn mock_windows_move_resize_each_handle_shows_its_resize_cursor() {
    let mut app = mock_app();
    let mut h = headless();
    open(&mut h, &mut app, Window::Profiler);
    let rect = drawn(&app, Window::Profiler);
    let cursor = |h: &mut Headless, app: &mut App<MockSide>, at: Pos2| {
        let _ = h.frame(app, vec![Event::PointerMoved(at)]);
        h.frame(app, Vec::new()).platform_output.cursor_icon
    };
    let want = [
        CursorIcon::ResizeHorizontal,
        CursorIcon::ResizeHorizontal,
        CursorIcon::ResizeVertical,
        CursorIcon::ResizeVertical,
        CursorIcon::ResizeNwSe,
        CursorIcon::ResizeNeSw,
        CursorIcon::ResizeNeSw,
        CursorIcon::ResizeNwSe,
    ];
    for (edges, want) in Edges::ALL.into_iter().zip(want) {
        assert_eq!(edges.cursor(), want, "{edges:?}");
        assert_eq!(
            cursor(&mut h, &mut app, on_handle(rect, edges)),
            want,
            "{edges:?}"
        );
    }
    // Inside the window, off every handle, the cursor is the default.
    assert_eq!(cursor(&mut h, &mut app, rect.center()), CursorIcon::Default);
    rejects("a corner's cursor on an edge", || {
        assert_eq!(Edges::ALL[0].cursor(), CursorIcon::ResizeNwSe)
    });
}

// --- The geometry, at its boundaries ----------------------------------------------------------------------------------

const SCREEN: Rect = Rect {
    min: Pos2 { x: 0.0, y: 0.0 },
    max: Pos2 {
        x: 1000.0,
        y: 600.0,
    },
};

#[test]
fn mock_windows_move_resize_on_screen_at_each_boundary() {
    let r = |x: f32, y: f32| Rect::from_min_size(pos2(x, y), vec2(200.0, 100.0));
    // At each limit exactly, the window stays.
    for at in [
        r(KEEP_W - 200.0, 0.0),
        r(1000.0 - KEEP_W, 0.0),
        r(0.0, 600.0 - TITLE_BAR_H),
    ] {
        assert_eq!(on_screen(at, SCREEN), at);
    }
    // A point past each, it comes back to the limit.
    assert_eq!(
        on_screen(r(KEEP_W - 201.0, 5.0), SCREEN),
        r(KEEP_W - 200.0, 5.0)
    );
    assert_eq!(
        on_screen(r(1001.0 - KEEP_W, 5.0), SCREEN),
        r(1000.0 - KEEP_W, 5.0)
    );
    assert_eq!(on_screen(r(5.0, -1.0), SCREEN), r(5.0, 0.0));
    assert_eq!(
        on_screen(r(5.0, 601.0 - TITLE_BAR_H), SCREEN),
        r(5.0, 600.0 - TITLE_BAR_H)
    );
    // On a screen shorter than a title bar, the top stays on it.
    let short = Rect::from_min_size(Pos2::ZERO, vec2(1000.0, TITLE_BAR_H / 2.0));
    assert_eq!(on_screen(r(5.0, 20.0), short), r(5.0, 0.0));
    rejects("a window past the left limit", || {
        assert_on_screen(r(KEEP_W - 201.0, 5.0), SCREEN)
    });
}

#[test]
fn mock_windows_move_resize_resized_at_each_boundary() {
    let start = Rect::from_min_max(pos2(100.0, 100.0), pos2(500.0, 400.0));
    let min = vec2(300.0, 200.0);
    let edge = |l, r, t, b| Edges::of(l, r, t, b);
    // To the minimum exactly, and past it.
    assert_eq!(
        resized(
            start,
            edge(false, true, false, true),
            vec2(-100.0, -100.0),
            min,
            SCREEN
        ),
        Rect::from_min_size(start.min, min)
    );
    assert_eq!(
        resized(
            start,
            edge(false, true, false, true),
            vec2(-101.0, -101.0),
            min,
            SCREEN
        ),
        Rect::from_min_size(start.min, min)
    );
    assert_eq!(
        resized(
            start,
            edge(true, false, true, false),
            vec2(101.0, 101.0),
            min,
            SCREEN
        ),
        Rect::from_min_max(start.max - min, start.max)
    );
    // The left edge stops KEEP_W left of the screen's right; the right edge KEEP_W right of its left.
    let wide = Rect::from_min_max(pos2(600.0, 100.0), pos2(1500.0, 400.0));
    let r = resized(
        wide,
        edge(true, false, false, false),
        vec2(500.0, 0.0),
        vec2(10.0, 10.0),
        SCREEN,
    );
    assert_eq!(r.min.x, 1000.0 - KEEP_W);
    let left = Rect::from_min_max(pos2(-800.0, 100.0), pos2(400.0, 400.0));
    let r = resized(
        left,
        edge(false, true, false, false),
        vec2(-500.0, 0.0),
        vec2(10.0, 10.0),
        SCREEN,
    );
    assert_eq!(r.max.x, KEEP_W);
    // The top edge stops at the screen's top, and a title bar above its bottom.
    let r = resized(
        start,
        edge(false, false, true, false),
        vec2(0.0, -200.0),
        min,
        SCREEN,
    );
    assert_eq!(r.min.y, 0.0);
    let low = Rect::from_min_max(pos2(100.0, 500.0), pos2(500.0, 1200.0));
    let r = resized(
        low,
        edge(false, false, true, false),
        vec2(0.0, 300.0),
        vec2(10.0, 10.0),
        SCREEN,
    );
    assert_eq!(r.min.y, 600.0 - TITLE_BAR_H);
    // The bottom edge may leave the screen; the edges not held stay put.
    let r = resized(
        start,
        edge(false, false, false, true),
        vec2(7.0, 900.0),
        min,
        SCREEN,
    );
    assert_eq!(r, Rect::from_min_max(start.min, pos2(500.0, 1300.0)));
    rejects("a resize past the minimum", || {
        assert_eq!(
            Rect::from_min_size(start.min, min - vec2(1.0, 1.0)),
            Rect::from_min_size(start.min, min)
        )
    });
}

#[test]
fn mock_windows_move_resize_the_handles_and_title_bar_geometry() {
    let rect = Rect::from_min_max(pos2(100.0, 100.0), pos2(500.0, 400.0));
    let h = |l, r, t, b| Edges::of(l, r, t, b).handle(rect);
    assert_eq!(
        h(true, false, false, false),
        Rect::from_min_max(
            pos2(100.0 - EDGE_GRAB, 100.0 + CORNER_GRAB),
            pos2(100.0 + EDGE_GRAB, 400.0 - CORNER_GRAB)
        )
    );
    assert_eq!(
        h(false, true, false, false),
        Rect::from_min_max(
            pos2(500.0 - EDGE_GRAB, 100.0 + CORNER_GRAB),
            pos2(500.0 + EDGE_GRAB, 400.0 - CORNER_GRAB)
        )
    );
    assert_eq!(
        h(false, false, true, false),
        Rect::from_min_max(
            pos2(100.0 + CORNER_GRAB, 100.0 - EDGE_GRAB),
            pos2(500.0 - CORNER_GRAB, 100.0 + EDGE_GRAB)
        )
    );
    assert_eq!(
        h(false, false, false, true),
        Rect::from_min_max(
            pos2(100.0 + CORNER_GRAB, 400.0 - EDGE_GRAB),
            pos2(500.0 - CORNER_GRAB, 400.0 + EDGE_GRAB)
        )
    );
    let corner = |x, y| Rect::from_center_size(pos2(x, y), Vec2::splat(2.0 * CORNER_GRAB));
    assert_eq!(h(true, false, true, false), corner(100.0, 100.0));
    assert_eq!(h(false, true, true, false), corner(500.0, 100.0));
    assert_eq!(h(true, false, false, true), corner(100.0, 400.0));
    assert_eq!(h(false, true, false, true), corner(500.0, 400.0));
    assert!(Edges::of(true, false, true, false).is_corner());
    assert!(!Edges::of(true, false, false, false).is_corner());
    assert!(!Edges::of(false, false, false, true).is_corner());
    // The title bar: TITLE_BAR_H along the top, or the whole window when it is shorter.
    assert_eq!(
        title_bar(rect),
        Rect::from_min_max(rect.min, pos2(500.0, 100.0 + TITLE_BAR_H))
    );
    let short = Rect::from_min_size(rect.min, vec2(400.0, TITLE_BAR_H / 2.0));
    assert_eq!(title_bar(short), short);
    // Only the console is docked.
    for window in MOVABLE {
        assert!(place::movable(window), "{window:?}");
        assert!(min_size(window).x >= KEEP_W && min_size(window).y >= TITLE_BAR_H);
    }
    assert!(!place::movable(Window::Console));
    rejects("an edge's handle across the corner", || {
        assert_eq!(h(true, false, false, false).min.y, rect.min.y - EDGE_GRAB)
    });
}

#[test]
fn mock_windows_move_resize_places_follow_the_drag_from_where_it_began() {
    let shown = Rect::from_min_size(pos2(100.0, 100.0), vec2(400.0, 300.0));
    let mut places = Places::default();
    assert_eq!(places.shown(Window::Run, shown, SCREEN), shown);
    places.drag(Window::Run, Grip::Title, shown, vec2(10.0, 0.0), SCREEN);
    // The next frame's drag is from where it began, not from where the window was drawn.
    let drawn = places.get(Window::Run).unwrap();
    places.drag(Window::Run, Grip::Title, drawn, vec2(20.0, 5.0), SCREEN);
    assert_eq!(
        places.get(Window::Run),
        Some(shown.translate(vec2(20.0, 5.0)))
    );
    // Another grip begins another drag, from where the window is.
    let moved = places.get(Window::Run).unwrap();
    let right = Grip::Edges(Edges::of(false, true, false, false));
    places.drag(Window::Run, right, moved, vec2(5.0, 0.0), SCREEN);
    assert_eq!(places.get(Window::Run).unwrap().max.x, moved.max.x + 5.0);
    // And another window, from where it is.
    places.drag(Window::Display, Grip::Title, shown, vec2(1.0, 1.0), SCREEN);
    assert_eq!(
        places.get(Window::Display),
        Some(shown.translate(vec2(1.0, 1.0)))
    );
    assert_eq!(places.rects().len(), 2);
    assert!(places.release());
    assert!(!places.release());
    // A remembered place is shown kept on screen.
    let off = Places::remembered([(Window::Run, shown.translate(vec2(5000.0, 0.0)))]);
    assert_eq!(off.shown(Window::Run, shown, SCREEN).min.x, 1000.0 - KEEP_W);
    rejects("a drag from where the window was drawn", || {
        assert_eq!(
            drawn.translate(vec2(20.0, 5.0)),
            shown.translate(vec2(20.0, 5.0))
        )
    });
}

/// The console's lines that are hover edits the engine applied.
fn hover_lines(app: &App<MockSide>) -> usize {
    app.console()
        .iter()
        .filter(|e| e.message.contains("SetField Hover"))
        .count()
}

/// A fast primary drag from `from` to `to` in one jump, held there a few frames with small moves, then released: each
/// frame checks that no hover point was sent.
fn fast_drag_sends_no_hover(h: &mut Headless, app: &mut App<MockSide>, from: Pos2, to: Pos2) {
    let button = |pos, pressed| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: NONE,
    };
    let start = hover_lines(app);
    let mut frames = vec![
        vec![Event::PointerMoved(from)],
        vec![button(from, true)],
        vec![Event::PointerMoved(from + vec2(8.0, 0.0))],
        vec![Event::PointerMoved(to)],
    ];
    for k in 1..4 {
        frames.push(vec![Event::PointerMoved(to + vec2(k as f32, k as f32))]);
    }
    frames.push(vec![button(to + vec2(3.0, 3.0), false)]);
    frames.push(Vec::new());
    for events in frames {
        let _ = h.frame(app, events);
        assert_eq!(
            app.snapshot().render.hover.at,
            None,
            "a hover point sent mid-drag"
        );
        assert_eq!(hover_lines(app), start, "a Hover edit logged mid-drag");
    }
}

#[test]
fn mock_windows_move_resize_a_drag_across_the_figure_sends_no_hover() {
    let figure = Layout::new(headless().screen(), headless().pixels_per_point()).figure;
    // By the title bar, and by the left edge, each from the Trajectory panel's side across the figure in one jump.
    for grip in [None, Some(Edges::of(true, false, false, false))] {
        let mut app = mock_app();
        let mut h = headless();
        open(&mut h, &mut app, Window::Run);
        let rect = drawn(&app, Window::Run);
        let from = match grip {
            None => on_title(rect),
            Some(edges) => on_handle(rect, edges),
        };
        fast_drag_sends_no_hover(&mut h, &mut app, from, figure.center());
        assert_ne!(
            drawn(&app, Window::Run),
            rect,
            "{grip:?}: the window followed the drag"
        );
    }
    // The control: the pointer crossing the figure with no drag does hover it.
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    rejects("a pointer that hovers the figure", || {
        for events in [
            vec![Event::PointerMoved(figure.center())],
            vec![Event::PointerMoved(figure.center() + vec2(5.0, 5.0))],
            Vec::new(),
        ] {
            let _ = h.frame(&mut app, events);
            assert_eq!(hover_lines(&app), 0);
        }
    });
}

#[test]
fn mock_windows_move_resize_the_window_is_drawn_under_the_pointer_in_the_frame_it_moves() {
    for grip in [None, Some(Edges::of(true, false, false, false))] {
        let mut app = mock_app();
        let mut h = headless();
        open(&mut h, &mut app, Window::Run);
        let rect = drawn(&app, Window::Run);
        let from = match grip {
            None => on_title(rect),
            Some(edges) => on_handle(rect, edges),
        };
        let button = |pos, pressed| Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: NONE,
        };
        let _ = h.frame(&mut app, vec![Event::PointerMoved(from)]);
        let _ = h.frame(&mut app, vec![button(from, true)]);
        let _ = h.frame(&mut app, vec![Event::PointerMoved(from + vec2(-10.0, 0.0))]);
        let by = vec2(-200.0, 0.0);
        let _ = h.frame(&mut app, vec![Event::PointerMoved(from + by)]);
        // This frame's drag is where the window was drawn this frame, not the frame before's.
        let now = drawn(&app, Window::Run);
        let want = rect.min.x + by.x;
        assert!(
            (now.min.x - want).abs() <= 1.0,
            "{grip:?}: drawn at {now:?}, not at x {want}"
        );
        let _ = h.frame(&mut app, vec![button(from + by, false)]);
        rejects("a window a frame behind the pointer", || {
            assert!((rect.min.x + by.x / 2.0 - want).abs() <= 1.0)
        });
    }
}
