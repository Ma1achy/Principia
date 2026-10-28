//! `mock_layout_splitters` (REQ-GUI-213; R-462 J1, J2, J3, J6; render_gui_spec §G2's Layout): on the mock, pointer
//! scripts on the five splitters. A drag on each moves that boundary by the drag, the canvas taking the rest; a drag
//! past a minimum stops there, for every panel and the canvas; a double-click returns that splitter, and only it, to
//! its default; a splitter shows a resize cursor on hover and is no keyboard element; the ring follows a resized
//! scope's rect. Each check also runs on an outcome it must reject (R-176, through `rejects`; gui takes no dependency
//! on validation, R-187): a fixed layout, today's before R-462, and a splitter that closes the canvas.

use eframe::egui::{self, pos2, vec2, CursorIcon, Event, PointerButton, Pos2, Rect};

use super::support::{headless, keys_on, mock_app, rejects};
use crate::app::App;
use crate::explore::breadcrumb::RING_WIDTH;
use crate::headless::Headless;
use crate::layout::{
    Layout, Splits, Splitter, MIN_BOTTOM_H, MIN_CANVAS_W, MIN_COMPASS_W, MIN_LEGEND_W,
    MIN_MANIFOLD_W, MIN_TIME_W, MIN_TOP_H, MIN_TRAJECTORY_W,
};
use crate::side::MockSide;

const NONE: egui::Modifiers = egui::Modifiers::NONE;

/// The app's layout as it stands on `h`.
pub(super) fn current(h: &Headless, app: &App<MockSide>) -> Layout {
    Layout::with_splits(h.screen(), h.pixels_per_point(), &app.splits)
}

/// A point on `splitter`'s handle in `layout`: on its boundary, halfway along the handle.
pub(super) fn on_handle(layout: &Layout, splitter: Splitter) -> Pos2 {
    let c = layout.handle(splitter).center();
    if splitter.across() {
        pos2(layout.edge(splitter), c.y)
    } else {
        pos2(c.x, layout.edge(splitter))
    }
}

/// A primary drag from `from` to `to`, in two moves, then frames to settle.
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

/// A drag on `splitter` by `by` points along it.
pub(super) fn drag_splitter(
    h: &mut Headless,
    app: &mut App<MockSide>,
    splitter: Splitter,
    by: f32,
) {
    let from = on_handle(&current(h, app), splitter);
    let d = if splitter.across() {
        vec2(by, 0.0)
    } else {
        vec2(0.0, by)
    };
    drag(h, app, from, from + d);
}

/// A double-click at `at`: two clicks 0.05 s apart.
fn double_click(h: &mut Headless, app: &mut App<MockSide>, at: Pos2) {
    h.set_frame_step(0.05);
    for _ in 0..2 {
        for events in crate::capture::click(at) {
            let _ = h.frame(app, events);
        }
    }
    h.set_frame_step(crate::headless::FRAME_S);
    let _ = h.frame(app, Vec::new());
}

/// An app on the mock, settled.
fn settled() -> (App<MockSide>, Headless) {
    let mut app = mock_app();
    let mut h = headless();
    for _ in 0..3 {
        let _ = h.frame(&mut app, Vec::new());
    }
    (app, h)
}

/// Checks that `after` is `before` with `splitter` moved by `by` and nothing else: its boundary `by` points on, the
/// panels on its far sides unmoved, the canvas taking the rest: wider or taller by what the panels gave up.
fn check_moved(before: &Layout, after: &Layout, splitter: Splitter, by: f32) {
    let near = |a: f32, b: f32, what: &str| {
        assert!((a - b).abs() < 1e-3, "{splitter:?}: {what} {a}, want {b}")
    };
    near(
        after.edge(splitter),
        before.edge(splitter) + by,
        "the boundary",
    );
    for other in Splitter::ALL.into_iter().filter(|s| *s != splitter) {
        near(after.edge(other), before.edge(other), "another boundary");
    }
    let (w, h) = (
        after.figure.width() - before.figure.width(),
        after.figure.height() - before.figure.height(),
    );
    match splitter {
        Splitter::ManifoldFigure => {
            near(w, -by, "the canvas's width");
            assert_eq!(after.trajectory, before.trajectory);
        }
        Splitter::FigureTrajectory => {
            near(w, by, "the canvas's width");
            assert_eq!(after.manifold_view, before.manifold_view);
        }
        Splitter::TopBottom => {
            near(h, by, "the canvas's height");
            near(
                after.bottom_row.height(),
                before.bottom_row.height() - by,
                "the bottom row",
            );
        }
        Splitter::CompassTime | Splitter::TimeLegend => {
            assert_eq!(after.figure, before.figure, "the canvas");
        }
    }
}

#[test]
fn mock_layout_splitters_a_drag_moves_each_boundary_by_the_drag() {
    for splitter in Splitter::ALL {
        for by in [-16.0, 30.0] {
            let (mut app, mut h) = settled();
            let before = current(&h, &app);
            assert_eq!(before, Layout::new(h.screen(), h.pixels_per_point()));
            drag_splitter(&mut h, &mut app, splitter, by);
            let after = current(&h, &app);
            check_moved(&before, &after, splitter, by);
            // Drawn there: each panel's scope placed at its new rect.
            for (id, rect) in [
                (crate::explore::MANIFOLD_VIEW, after.manifold_view),
                (crate::explore::FIGURE, after.figure),
                (crate::explore::trajectory::TRAJECTORY, after.trajectory),
                (crate::explore::COMPASS, after.compass),
                (crate::explore::time::TIME, after.time),
                (crate::explore::legend::LEGEND, after.legend),
            ] {
                assert_eq!(
                    app.keyboard.place_of(id).map(|p| p.rect),
                    Some(rect),
                    "{id}"
                );
            }
            // A fixed layout, today's before R-462, fails.
            let fixed = Layout::new(h.screen(), h.pixels_per_point());
            rejects("a fixed layout", || {
                check_moved(&before, &fixed, splitter, by)
            });
        }
    }
}

/// The least size of each panel and of the canvas, in `layout`, against its minimum: none under it.
fn check_minimums(layout: &Layout) {
    let top_row = layout.bottom_row.min.y - layout.top_bar.max.y;
    for (what, size, min) in [
        (
            "Manifold view",
            layout.manifold_view.width(),
            MIN_MANIFOLD_W,
        ),
        ("canvas", layout.figure.width(), MIN_CANVAS_W),
        ("Trajectory", layout.trajectory.width(), MIN_TRAJECTORY_W),
        ("top row", top_row, MIN_TOP_H),
        ("bottom row", layout.bottom_row.height(), MIN_BOTTOM_H),
        ("compass", layout.compass.width(), MIN_COMPASS_W),
        ("Time", layout.time.width(), MIN_TIME_W),
        ("Legend", layout.legend.width(), MIN_LEGEND_W),
    ] {
        assert!(size >= min - 1e-3, "{what} {size} under its minimum {min}");
    }
}

/// A drag to the window's edge: the splitter, the way (−1 or 1), the panel squeezed, its size and its minimum.
type Stop = (Splitter, f32, &'static str, fn(&Layout) -> f32, f32);

#[test]
fn mock_layout_splitters_a_drag_past_a_minimum_stops_there() {
    // Each splitter dragged to the window's edge each way, and the panel or the canvas it squeezes at its minimum.
    let cases: [Stop; 10] = [
        (
            Splitter::ManifoldFigure,
            -1.0,
            "Manifold view",
            |l| l.manifold_view.width(),
            MIN_MANIFOLD_W,
        ),
        (
            Splitter::ManifoldFigure,
            1.0,
            "canvas",
            |l| l.figure.width(),
            MIN_CANVAS_W,
        ),
        (
            Splitter::FigureTrajectory,
            -1.0,
            "canvas",
            |l| l.figure.width(),
            MIN_CANVAS_W,
        ),
        (
            Splitter::FigureTrajectory,
            1.0,
            "Trajectory",
            |l| l.trajectory.width(),
            MIN_TRAJECTORY_W,
        ),
        (
            Splitter::TopBottom,
            -1.0,
            "top row",
            |l| l.bottom_row.min.y - l.top_bar.max.y,
            MIN_TOP_H,
        ),
        (
            Splitter::TopBottom,
            1.0,
            "bottom row",
            |l| l.bottom_row.height(),
            MIN_BOTTOM_H,
        ),
        (
            Splitter::CompassTime,
            -1.0,
            "compass",
            |l| l.compass.width(),
            MIN_COMPASS_W,
        ),
        (
            Splitter::CompassTime,
            1.0,
            "Time",
            |l| l.time.width(),
            MIN_TIME_W,
        ),
        (
            Splitter::TimeLegend,
            -1.0,
            "Time",
            |l| l.time.width(),
            MIN_TIME_W,
        ),
        (
            Splitter::TimeLegend,
            1.0,
            "Legend",
            |l| l.legend.width(),
            MIN_LEGEND_W,
        ),
    ];
    for (splitter, sign, what, size, min) in cases {
        let (mut app, mut h) = settled();
        let before = current(&h, &app);
        let from = on_handle(&before, splitter);
        let screen = h.screen();
        let to = if splitter.across() {
            pos2(if sign < 0.0 { 1.0 } else { screen.max.x - 1.0 }, from.y)
        } else {
            pos2(
                from.x,
                if sign < 0.0 {
                    screen.min.y + 1.0
                } else {
                    screen.max.y - 1.0
                },
            )
        };
        drag(&mut h, &mut app, from, to);
        let after = current(&h, &app);
        assert!(
            (size(&after) - min).abs() < 1e-3,
            "{splitter:?} toward {sign}: the {what} is {}, want its minimum {min}",
            size(&after)
        );
        check_minimums(&after);
        // A drag that stops short of the minimum moves by the drag: the stop is the minimum's, not a fixed travel.
        assert!(size(&before) > min + 1.0, "{what} starts above its minimum");
    }
    // A splitter that closes the canvas fails.
    let (_, h) = settled();
    let mut closed = Splits::default();
    closed.set(
        Splitter::ManifoldFigure,
        closed.get(Splitter::FigureTrajectory),
    );
    let closing = Layout::with_splits(h.screen(), h.pixels_per_point(), &closed);
    rejects("a splitter that closes the canvas", || {
        check_minimums(&closing)
    });
}

#[test]
fn mock_layout_splitters_the_range_holds_each_neighbour_at_its_minimum() {
    let l = Layout::new(headless().screen(), crate::capture::PIXELS_PER_POINT);
    let w = l.window;
    let figure_w = MIN_CANVAS_W + l.axis_y.width();
    assert_eq!(
        l.range(Splitter::ManifoldFigure),
        (w.min.x + MIN_MANIFOLD_W, l.trajectory.min.x - figure_w)
    );
    assert_eq!(
        l.range(Splitter::FigureTrajectory),
        (l.manifold_view.max.x + figure_w, w.max.x - MIN_TRAJECTORY_W)
    );
    assert_eq!(
        l.range(Splitter::TopBottom),
        (l.top_bar.max.y + MIN_TOP_H, l.footer.min.y - MIN_BOTTOM_H)
    );
    assert_eq!(
        l.range(Splitter::CompassTime),
        (w.min.x + MIN_COMPASS_W, l.legend.min.x - MIN_TIME_W)
    );
    assert_eq!(
        l.range(Splitter::TimeLegend),
        (l.compass.max.x + MIN_TIME_W, w.max.x - MIN_LEGEND_W)
    );
    // The fraction is of the window from its own origin: a window not at the screen's.
    let offset = Rect::from_min_size(pos2(100.0, 50.0), l.window.size());
    let o = Layout::with_splits(offset, crate::capture::PIXELS_PER_POINT, &Splits::default());
    for s in Splitter::ALL {
        let start = o.edge(s);
        let (min, size) = if s.across() {
            (offset.min.x, offset.width())
        } else {
            (offset.min.y, offset.height())
        };
        let f = o.dragged(s, start, 4.0);
        assert!((f - (start + 4.0 - min) / size).abs() < 1e-6, "{s:?}: {f}");
        assert!(
            (f - l.dragged(s, l.edge(s), 4.0)).abs() < 1e-6,
            "{s:?}: as at the origin"
        );
    }
    // Inside the range a drag lands where it is sent; at each end it stops; the fraction is of the window's width, or
    // its height down.
    for s in Splitter::ALL {
        let (lo, hi) = l.range(s);
        let size = if s.across() { w.width() } else { w.height() };
        let start = l.edge(s);
        let at = |delta: f32| l.dragged(s, start, delta) * size;
        assert!(
            (at(lo - start + 2.0) - (lo + 2.0)).abs() < 1e-3,
            "{s:?} inside"
        );
        assert!(
            (at(hi - start - 2.0) - (hi - 2.0)).abs() < 1e-3,
            "{s:?} inside"
        );
        assert!((at(lo - start) - lo).abs() < 1e-3, "{s:?} at its low end");
        assert!((at(hi - start) - hi).abs() < 1e-3, "{s:?} at its high end");
        assert!(
            (at(lo - start - 50.0) - lo).abs() < 1e-3,
            "{s:?} past its low end"
        );
        assert!(
            (at(hi - start + 50.0) - hi).abs() < 1e-3,
            "{s:?} past its high end"
        );
        rejects("a drag held at the far end", || {
            assert!((at(lo - start - 50.0) - hi).abs() < 1e-3)
        });
    }
}

#[test]
fn mock_layout_splitters_a_double_click_resets_that_splitter_only() {
    let (mut app, mut h) = settled();
    drag_splitter(&mut h, &mut app, Splitter::FigureTrajectory, 40.0);
    drag_splitter(&mut h, &mut app, Splitter::TimeLegend, -45.0);
    drag_splitter(&mut h, &mut app, Splitter::TopBottom, 20.0);
    let moved = app.splits;
    let defaults = Splits::default();
    for s in [
        Splitter::FigureTrajectory,
        Splitter::TimeLegend,
        Splitter::TopBottom,
    ] {
        assert_ne!(moved.get(s), defaults.get(s), "{s:?} moved");
    }
    let at = on_handle(&current(&h, &app), Splitter::TimeLegend);
    double_click(&mut h, &mut app, at);
    let reset = app.splits;
    assert_eq!(
        reset.get(Splitter::TimeLegend),
        defaults.get(Splitter::TimeLegend)
    );
    for s in Splitter::ALL
        .into_iter()
        .filter(|s| *s != Splitter::TimeLegend)
    {
        assert_eq!(reset.get(s), moved.get(s), "{s:?} left as it was");
    }
    let l = current(&h, &app);
    let default = Layout::new(h.screen(), h.pixels_per_point());
    assert_eq!(l.legend.min.x, default.legend.min.x);
    assert_ne!(l.trajectory.min.x, default.trajectory.min.x);
    // A double-click that reset every splitter, or none, fails.
    rejects("a reset of every splitter", || {
        assert_eq!(
            Splits::default().get(Splitter::FigureTrajectory),
            moved.get(Splitter::FigureTrajectory)
        )
    });
    rejects("no reset", || {
        assert_eq!(
            moved.get(Splitter::TimeLegend),
            defaults.get(Splitter::TimeLegend)
        )
    });
    // A single click moves nothing.
    let at = on_handle(&current(&h, &app), Splitter::FigureTrajectory);
    for events in crate::capture::click(at) {
        let _ = h.frame(&mut app, events);
    }
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(app.splits, reset);
}

#[test]
fn mock_layout_splitters_show_a_resize_cursor_and_are_no_keyboard_element() {
    for splitter in Splitter::ALL {
        let (mut app, mut h) = settled();
        let at = on_handle(&current(&h, &app), splitter);
        let _ = h.frame(&mut app, vec![Event::PointerMoved(at)]);
        let out = h.frame(&mut app, Vec::new());
        let want = if splitter.across() {
            CursorIcon::ResizeHorizontal
        } else {
            CursorIcon::ResizeVertical
        };
        assert_eq!(out.platform_output.cursor_icon, want, "{splitter:?}");
        // Off it, no resize cursor: the figure's centre.
        let centre = current(&h, &app).figure.center();
        let _ = h.frame(&mut app, vec![Event::PointerMoved(centre)]);
        let off = h.frame(&mut app, Vec::new()).platform_output.cursor_icon;
        assert_ne!(off, want);
        rejects("the cursor off the splitter", || assert_eq!(off, want));
        // No accessible node, no scope: a pointer control only (J6).
        assert!(Splitter::of_id(splitter.id()) == Some(splitter));
        assert!(app
            .keyboard
            .tree(app.view.mode)
            .get(splitter.name())
            .is_none());
    }
    // The handle lies across its boundary, along it.
    let l = Layout::new(headless().screen(), crate::capture::PIXELS_PER_POINT);
    for s in Splitter::ALL {
        let r = l.handle(s);
        let e = l.edge(s);
        let (lo, hi) = if s.across() {
            (r.min.x, r.max.x)
        } else {
            (r.min.y, r.max.y)
        };
        assert!(lo < e && e < hi, "{s:?}: {r:?} across {e}");
        assert!(
            (hi - lo - crate::layout::HANDLE).abs() < 1e-3,
            "{s:?}: {r:?} is the handle's width"
        );
        assert!(
            ((lo + hi) / 2.0 - e).abs() < 1e-3,
            "{s:?}: {r:?} centred on {e}"
        );
        assert_eq!(l.splitter_at(on_handle(&l, s)), Some(s));
    }
    // Along its boundary, each meeting the top row's bottom where that handle begins, never overlapping it.
    let across = l.handle(Splitter::TopBottom);
    assert_eq!(
        (across.min.x, across.max.x),
        (l.window.min.x, l.window.max.x)
    );
    for s in [Splitter::ManifoldFigure, Splitter::FigureTrajectory] {
        assert_eq!(l.handle(s).min.y, l.top_bar.max.y, "{s:?}");
        assert_eq!(l.handle(s).max.y, across.min.y, "{s:?}");
    }
    for s in [Splitter::CompassTime, Splitter::TimeLegend] {
        assert_eq!(l.handle(s).min.y, across.max.y, "{s:?}");
        assert_eq!(l.handle(s).max.y, l.footer.min.y, "{s:?}");
    }
    assert_eq!(l.splitter_at(l.figure.center()), None);
    assert_eq!(l.splitter_at(l.manifold_view.center()), None);
}

/// The focus ring's rects drawn by one more frame.
fn rings(h: &mut Headless, app: &mut App<MockSide>) -> Vec<Rect> {
    super::support::shapes(h, app)
        .into_iter()
        .filter_map(|s| match s {
            egui::Shape::Rect(r) if r.stroke.width == RING_WIDTH => Some(r.rect),
            _ => None,
        })
        .collect()
}

#[test]
fn mock_layout_splitters_the_ring_follows_a_resized_scope() {
    use crate::explore::trajectory::TRAJECTORY;
    let (mut app, mut h) = settled();
    app.view.focus.path = vec![TRAJECTORY.to_owned()];
    keys_on(&mut h, &mut app);
    let before = current(&h, &app).trajectory;
    assert_eq!(rings(&mut h, &mut app), vec![before]);
    drag_splitter(&mut h, &mut app, Splitter::FigureTrajectory, -50.0);
    // The drag hides the ring (R-464); the next key shows it on the scope's new rect.
    keys_on(&mut h, &mut app);
    let after = current(&h, &app).trajectory;
    assert_eq!(after.min.x, before.min.x - 50.0);
    let drawn = rings(&mut h, &mut app);
    assert_eq!(drawn, vec![after]);
    assert_eq!(
        app.view.focus.path,
        vec![TRAJECTORY.to_owned()],
        "the focus kept"
    );
    rejects("a ring left at the old rect", || {
        assert_eq!(drawn, vec![before])
    });
}

#[test]
fn mock_layout_splitters_a_pointer_on_or_dragging_a_splitter_hovers_no_figure_point() {
    let (mut app, mut h) = settled();
    let hover = |h: &mut Headless, app: &mut App<MockSide>, at: Pos2| {
        let _ = h.frame(app, vec![Event::PointerMoved(at)]);
        let _ = h.frame(app, Vec::new());
        app.snapshot().render.hover.at
    };
    let l = current(&h, &app);
    // On the figure | Trajectory handle, its half over the canvas: no hover; a little further in, the figure's.
    let on = on_handle(&l, Splitter::FigureTrajectory) - vec2(2.0, 0.0);
    assert!(l.figure.contains(on) && l.splitter_at(on).is_some());
    let on_it = hover(&mut h, &mut app, on);
    assert_eq!(on_it, None);
    let off = on - vec2(10.0, 0.0);
    let figure = hover(&mut h, &mut app, off);
    assert!(figure.is_some());
    rejects("the figure's hover", || assert_eq!(figure, None));
    // Dragging it past the canvas's minimum, the pointer over the canvas: still no hover, no SetField sent.
    let _ = hover(&mut h, &mut app, l.legend.center());
    let lines = app.console().len();
    let from = on_handle(&l, Splitter::FigureTrajectory);
    let into = pos2(l.figure.min.x + 100.0, l.figure.center().y);
    let press = |pos, pressed| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: NONE,
    };
    for events in [
        vec![Event::PointerMoved(from)],
        vec![press(from, true)],
        vec![Event::PointerMoved(into)],
        Vec::new(),
        Vec::new(),
    ] {
        let _ = h.frame(&mut app, events);
    }
    let held = current(&h, &app);
    assert!(
        !held.handle(Splitter::FigureTrajectory).contains(into),
        "the pointer off the held handle"
    );
    assert!(held.figure.contains(into));
    assert_eq!(app.snapshot().render.hover.at, None);
    assert_eq!(app.console().len(), lines, "no SetField while held");
    let _ = h.frame(&mut app, vec![press(into, false)]);
}
