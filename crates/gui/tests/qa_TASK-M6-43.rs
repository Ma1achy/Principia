//! QA tests for TASK-M6-43, written from R-462 and its two requirements, not from the implementation:
//!
//! - REQ-GUI-213 (R-462 J1 to J3, J6; render_gui_spec §G2's "Layout: every panel boundary is a splitter"): on the
//!   mock, the five boundaries (the Manifold view and the canvas, the canvas and the Trajectory panel, the top row and
//!   the bottom row, Compass and Time, Time and Legend) are dragged by the pointer, each moving by the drag and only it,
//!   the canvas taking what the panels leave; a drag past a minimum stops there, for every panel and the canvas; a
//!   double-click returns that splitter, and only it, to its default, 01_main.png's; the keyboard's ring follows a
//!   resized scope's rect; and no splitter, a double-click's included, takes a panel or the canvas below its minimum.
//! - REQ-GUI-214 (R-462 J3 to J5; R-406's rule; gui_state_contract §2): the layout is remembered across sessions in the
//!   GUI's own settings, never the contract; View › reset layout returns every splitter to its default, and it is
//!   saved; a splitter drag that resizes the canvas sends no `SetField`, keeps the field point under each fixed screen
//!   pixel (to 1e-9 in chart coordinates), shows more or less of the field at the same scale, keeps the mock's
//!   integrated quads and computes only quads over the newly exposed area.
//!
//! The panels are read where the app draws them: each big scope's rect as the keyboard places it, the scopes the ring
//! is drawn on (render_gui_spec §G3). The field point under a pixel is the contract's hover, which the app sends for
//! the pointer over the figure (gui_state_contract §2). Every `SetField` the app sends is recorded by a wrapper engine
//! side around the mock's. The minimum sizes are the task's own (R-462 J2: "the values are the task's, listed in its
//! PR"), read from `gui::layout`. Each check runs its own control (R-176) through [`rejects`]: a fixed layout, a
//! splitter that closes the canvas, a reset that moves another splitter, a GUI that forgets its layout, a resize that
//! sends a `SetField`, one that stretches the figure, and one that drops the integrated quads.
// The file name `qa_TASK-M6-43` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::path::PathBuf;
use std::sync::Arc;

use eframe::egui::{self, pos2, CursorIcon, Event, Key, Modifiers, PointerButton, Pos2, Rect};
use engine::contract::canvas::Canvas;
use engine::contract::interface::EngineInterface;
use engine::contract::profile::Trace;
use engine::contract::sample::Sample;
use engine::contract::set_field::SetField;
use engine::contract::sim_config::Latent;
use engine::contract::snapshot::Snapshot;
use gui::app::App;
use gui::capture::{FORMAT, PIXELS_PER_POINT, SIZE};
use gui::clock::TimeSource;
use gui::explore::breadcrumb::{NAVIGATION, RING_WIDTH};
use gui::explore::legend::LEGEND;
use gui::explore::manifold_view::point;
use gui::explore::time::TIME;
use gui::explore::trajectory::TRAJECTORY;
use gui::explore::{COMPASS, FIGURE, MANIFOLD_VIEW};
use gui::headless::Headless;
use gui::layout::{
    Placement, MIN_BOTTOM_H, MIN_CANVAS_W, MIN_COMPASS_W, MIN_LEGEND_W, MIN_MANIFOLD_W, MIN_TIME_W,
    MIN_TOP_H, MIN_TRAJECTORY_W,
};
use gui::mock::quads::{self, Quad};
use gui::mock::MockEngine;
use gui::settings::Store;
use gui::side::{EngineSide, MockSide};

const NONE: Modifiers = Modifiers::NONE;
/// Edges sit on whole physical pixels, 1/1.5 point at the capture's scale: half of that each way, and a little more.
const TOL: f32 = 0.7;
/// REQ-GUI-214's verify: the field point under a fixed pixel, in chart coordinates.
const CHART_TOL: f64 = 1e-9;

/// Asserts that `check` panics: the check can fail (R-176's control, run in the test).
fn rejects(what: &str, check: impl FnOnce()) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check));
    assert!(
        outcome.is_err(),
        "the check passed on {what}, which it must reject"
    );
}

// ---------------------------------------------------------------------------------------------------------------------
// A recording engine side: the mock's, every `SetField` the app sends kept.

struct Recording {
    inner: MockSide,
    sent: Vec<SetField>,
}

impl EngineInterface for Recording {
    fn set_field(&mut self, edit: SetField) {
        self.sent.push(edit.clone());
        self.inner.engine().set_field(edit);
    }
    fn snapshot(&mut self) -> Snapshot {
        self.inner.engine().snapshot()
    }
    fn undo(&mut self) {
        self.inner.engine().undo();
    }
    fn redo(&mut self) {
        self.inner.engine().redo();
    }
}

impl EngineSide for Recording {
    type Engine = Recording;
    fn engine(&mut self) -> &mut Recording {
        self
    }
    fn is_mock(&self) -> bool {
        self.inner.is_mock()
    }
    fn canvas(&self) -> Option<Arc<dyn Canvas>> {
        self.inner.canvas()
    }
    fn figure(&self, placement: Placement) -> Option<Arc<dyn Canvas>> {
        self.inner.figure(placement)
    }
    fn show_span(&mut self, span: [f64; 4]) {
        self.inner.show_span(span);
    }
    fn time_source(&mut self) -> Option<&mut dyn TimeSource> {
        self.inner.time_source()
    }
    fn trace(&self) -> Option<&Trace> {
        self.inner.trace()
    }
    fn stain_content(&self) -> Option<gui::stain::Content> {
        self.inner.stain_content()
    }
    fn preview(&self) -> Option<Arc<dyn Canvas>> {
        self.inner.preview()
    }
    fn recompile(&mut self) {
        self.inner.recompile();
    }
    fn compute_ic(&mut self, ic: &Latent, at: [f64; 2]) -> Option<Sample> {
        self.inner.compute_ic(ic, at)
    }
}

type Rig = (App<Recording>, Headless);

fn side() -> Recording {
    Recording {
        inner: MockSide::new(MockEngine::frozen(), None),
        sent: Vec::new(),
    }
}

fn settle(h: &mut Headless, app: &mut App<Recording>) {
    for _ in 0..3 {
        let _ = h.frame(app, Vec::new());
    }
}

/// A GUI with no store, settled, the pointer resting in the Legend, off the figure.
fn fresh() -> Rig {
    rig(App::new(side(), FORMAT))
}

/// A GUI whose settings are kept in the file at `path`, settled.
fn stored(path: &PathBuf) -> Rig {
    rig(App::with_settings(side(), FORMAT, Store::at(path)))
}

fn rig(mut app: App<Recording>) -> Rig {
    let mut h = Headless::new(SIZE, PIXELS_PER_POINT);
    settle(&mut h, &mut app);
    let away = drawn(&app).legend.center();
    let _ = h.frame(&mut app, vec![Event::PointerMoved(away)]);
    settle(&mut h, &mut app);
    (app, h)
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("qa-m6-43-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

// ---------------------------------------------------------------------------------------------------------------------
// The panels, as drawn.

/// The big scopes' rects as the app last placed them: where each panel is drawn and ringed.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Panels {
    manifold: Rect,
    figure: Rect,
    trajectory: Rect,
    compass: Rect,
    time: Rect,
    legend: Rect,
}

fn drawn(app: &App<Recording>) -> Panels {
    let at = |id: &str| {
        app.keyboard
            .place_of(id)
            .unwrap_or_else(|| panic!("{id} is placed"))
            .rect
    };
    Panels {
        manifold: at(MANIFOLD_VIEW),
        figure: at(FIGURE),
        trajectory: at(TRAJECTORY),
        compass: at(COMPASS),
        time: at(TIME),
        legend: at(LEGEND),
    }
}

/// The five boundaries R-462 J1 names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum B {
    ManifoldCanvas,
    CanvasTrajectory,
    TopBottom,
    CompassTime,
    TimeLegend,
}

const ALL: [B; 5] = [
    B::ManifoldCanvas,
    B::CanvasTrajectory,
    B::TopBottom,
    B::CompassTime,
    B::TimeLegend,
];

impl B {
    fn across(self) -> bool {
        self != B::TopBottom
    }
}

impl Panels {
    /// Where boundary `b` is, in points: an `x`, or the bottom row's top `y`.
    fn edge(&self, b: B) -> f32 {
        match b {
            B::ManifoldCanvas => self.manifold.max.x,
            B::CanvasTrajectory => self.trajectory.min.x,
            B::TopBottom => self.compass.min.y,
            B::CompassTime => self.compass.max.x,
            B::TimeLegend => self.legend.min.x,
        }
    }

    /// A point on boundary `b`, halfway along it.
    fn on(&self, b: B) -> Pos2 {
        let top_row = (self.manifold.min.y + self.manifold.max.y) / 2.0;
        let bottom_row = (self.compass.min.y + self.compass.max.y) / 2.0;
        match b {
            B::ManifoldCanvas | B::CanvasTrajectory => pos2(self.edge(b), top_row),
            B::TopBottom => pos2(self.figure.center().x, self.edge(b)),
            B::CompassTime | B::TimeLegend => pos2(self.edge(b), bottom_row),
        }
    }

    /// The top row's height: from the top bar's bottom to the bottom row's top.
    fn top_row_h(&self) -> f32 {
        self.compass.min.y - self.manifold.min.y
    }

    fn bottom_row_h(&self) -> f32 {
        self.compass.height()
    }
}

fn press(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: NONE,
    }
}

/// A pointer drag from `from` to `to`, in four moves, released, then frames to settle.
fn drag(h: &mut Headless, app: &mut App<Recording>, from: Pos2, to: Pos2) {
    let mut frames = vec![vec![Event::PointerMoved(from)], vec![press(from, true)]];
    for k in 1..=4 {
        frames.push(vec![Event::PointerMoved(
            from + (to - from) * (k as f32 / 4.0),
        )]);
    }
    frames.push(vec![press(to, false)]);
    for events in frames {
        let _ = h.frame(app, events);
    }
    settle(h, app);
}

/// A drag on boundary `b` by `by` points across it.
fn drag_b(h: &mut Headless, app: &mut App<Recording>, b: B, by: f32) {
    let from = drawn(app).on(b);
    let to = if b.across() {
        pos2(from.x + by, from.y)
    } else {
        pos2(from.x, from.y + by)
    };
    drag(h, app, from, to);
}

/// A drag on boundary `b` to the window's edge, `toward_max` its right (bottom) or else its left (top): past every
/// minimum.
fn drag_far(h: &mut Headless, app: &mut App<Recording>, b: B, toward_max: bool) {
    let screen = h.screen();
    let from = drawn(app).on(b);
    let to = match (b.across(), toward_max) {
        (true, true) => pos2(screen.max.x - 1.0, from.y),
        (true, false) => pos2(screen.min.x + 1.0, from.y),
        (false, true) => pos2(from.x, screen.max.y - 1.0),
        (false, false) => pos2(from.x, screen.min.y + 1.0),
    };
    drag(h, app, from, to);
}

/// A double-click on boundary `b`: two clicks 0.05 s apart.
fn double_click(h: &mut Headless, app: &mut App<Recording>, b: B) {
    let at = drawn(app).on(b);
    h.set_frame_step(0.05);
    for _ in 0..2 {
        for events in [
            vec![Event::PointerMoved(at)],
            vec![press(at, true)],
            vec![press(at, false)],
        ] {
            let _ = h.frame(app, events);
        }
    }
    h.set_frame_step(gui::headless::FRAME_S);
    settle(h, app);
}

// ---------------------------------------------------------------------------------------------------------------------
// The checks.

fn near(a: f32, b: f32, what: &str) {
    assert!((a - b).abs() <= TOL, "{what}: {a}, want {b}");
}

/// `after` is `before` with boundary `b` moved by `by` and no other boundary moved, the canvas taking the rest: what
/// a panel gives up, the canvas gains, and the panels across the window still meet edge to edge.
fn check_moved(before: &Panels, after: &Panels, b: B, by: f32) {
    near(
        after.edge(b),
        before.edge(b) + by,
        &format!("{b:?}: the boundary"),
    );
    for other in ALL.into_iter().filter(|o| *o != b) {
        near(
            after.edge(other),
            before.edge(other),
            &format!("{b:?} dragged: {other:?}"),
        );
    }
    let dw = after.figure.width() - before.figure.width();
    let dh = after.figure.height() - before.figure.height();
    match b {
        B::ManifoldCanvas => {
            near(dw, -by, "the canvas's width");
            near(dh, 0.0, "the canvas's height");
        }
        B::CanvasTrajectory => {
            near(dw, by, "the canvas's width");
            near(dh, 0.0, "the canvas's height");
        }
        B::TopBottom => {
            near(dh, by, "the canvas's height");
            near(dw, 0.0, "the canvas's width");
            near(
                after.bottom_row_h(),
                before.bottom_row_h() - by,
                "the bottom row's height",
            );
        }
        B::CompassTime | B::TimeLegend => {
            near(dw, 0.0, "the canvas's width");
            near(dh, 0.0, "the canvas's height");
        }
    }
    // The rows tile the window: no gap or overlap opened by the drag.
    for p in [before, after] {
        near(
            p.figure.max.x,
            p.trajectory.min.x,
            "the canvas meets the Trajectory panel",
        );
        near(p.compass.max.x, p.time.min.x, "Compass meets Time");
        near(p.time.max.x, p.legend.min.x, "Time meets Legend");
        near(
            p.manifold.max.y,
            p.compass.min.y,
            "the top row meets the bottom row",
        );
    }
}

/// Every panel, and the canvas, at least its minimum.
fn check_minimums(p: &Panels, what: &str) {
    for (size, min, name) in [
        (p.manifold.width(), MIN_MANIFOLD_W, "the Manifold view"),
        (p.figure.width(), MIN_CANVAS_W, "the canvas"),
        (
            p.trajectory.width(),
            MIN_TRAJECTORY_W,
            "the Trajectory panel",
        ),
        (p.top_row_h(), MIN_TOP_H, "the top row"),
        (p.bottom_row_h(), MIN_BOTTOM_H, "the bottom row"),
        (p.compass.width(), MIN_COMPASS_W, "Compass"),
        (p.time.width(), MIN_TIME_W, "Time"),
        (p.legend.width(), MIN_LEGEND_W, "Legend"),
    ] {
        assert!(
            size >= min - TOL,
            "{what}: {name} is {size} points, below its minimum {min}"
        );
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// REQ-GUI-213.

#[test]
fn qa_m6_43_the_minimums_are_sizes() {
    // R-462 J2: each panel has a minimum, the canvas one too, so no splitter can close it.
    for m in [
        MIN_MANIFOLD_W,
        MIN_CANVAS_W,
        MIN_TRAJECTORY_W,
        MIN_TOP_H,
        MIN_BOTTOM_H,
        MIN_COMPASS_W,
        MIN_TIME_W,
        MIN_LEGEND_W,
    ] {
        assert!(m.is_finite() && m > 0.0, "a minimum {m}");
    }
    // 01_main.png's layout at the capture's size meets every minimum, so a fresh GUI starts within them.
    let (app, _h) = fresh();
    check_minimums(&drawn(&app), "the default layout");
}

#[test]
fn qa_m6_43_a_drag_moves_each_boundary_by_the_drag_the_canvas_taking_the_rest() {
    for b in ALL {
        // Within the slack every default leaves above its minimums at 1440 × 900 points (the smallest, the Manifold
        // view's, is 20 points).
        for by in [-12.0_f32, 12.0] {
            let (mut app, mut h) = fresh();
            let before = drawn(&app);
            drag_b(&mut h, &mut app, b, by);
            let after = drawn(&app);
            check_moved(&before, &after, b, by);
            // Control: a fixed layout (today's before R-462): the drag moves nothing.
            rejects(&format!("a fixed layout ({b:?})"), || {
                check_moved(&before, &before, b, by)
            });
            // Control: the drag moving another boundary along.
            let other = ALL[(ALL.iter().position(|o| *o == b).unwrap() + 1) % 5];
            let mut wrong = after;
            match other {
                B::ManifoldCanvas => wrong.manifold.max.x += 5.0,
                B::CanvasTrajectory => wrong.trajectory.min.x += 5.0,
                B::TopBottom => wrong.compass.min.y += 5.0,
                B::CompassTime => wrong.compass.max.x += 5.0,
                B::TimeLegend => wrong.legend.min.x += 5.0,
            }
            rejects(&format!("{b:?} moving {other:?} too"), || {
                check_moved(&before, &wrong, b, by)
            });
        }
    }
}

#[test]
fn qa_m6_43_a_drag_past_a_minimum_stops_there() {
    // For each boundary and each way, the panel it squeezes: (boundary, toward max, its size, its minimum).
    type Size = fn(&Panels) -> f32;
    let cases: [(B, bool, Size, f32, &str); 10] = [
        (
            B::ManifoldCanvas,
            false,
            |p| p.manifold.width(),
            MIN_MANIFOLD_W,
            "the Manifold view",
        ),
        (
            B::ManifoldCanvas,
            true,
            |p| p.figure.width(),
            MIN_CANVAS_W,
            "the canvas",
        ),
        (
            B::CanvasTrajectory,
            false,
            |p| p.figure.width(),
            MIN_CANVAS_W,
            "the canvas",
        ),
        (
            B::CanvasTrajectory,
            true,
            |p| p.trajectory.width(),
            MIN_TRAJECTORY_W,
            "the Trajectory panel",
        ),
        (
            B::TopBottom,
            false,
            |p| p.top_row_h(),
            MIN_TOP_H,
            "the top row",
        ),
        (
            B::TopBottom,
            true,
            |p| p.bottom_row_h(),
            MIN_BOTTOM_H,
            "the bottom row",
        ),
        (
            B::CompassTime,
            false,
            |p| p.compass.width(),
            MIN_COMPASS_W,
            "Compass",
        ),
        (B::CompassTime, true, |p| p.time.width(), MIN_TIME_W, "Time"),
        (B::TimeLegend, false, |p| p.time.width(), MIN_TIME_W, "Time"),
        (
            B::TimeLegend,
            true,
            |p| p.legend.width(),
            MIN_LEGEND_W,
            "Legend",
        ),
    ];
    for (b, toward_max, size, min, name) in cases {
        let (mut app, mut h) = fresh();
        drag_far(&mut h, &mut app, b, toward_max);
        let after = drawn(&app);
        check_minimums(&after, &format!("{b:?} dragged to the window's edge"));
        // It stopped at the minimum, not short of it.
        near(size(&after), min, &format!("{b:?}: {name} at its minimum"));
    }
    // The canvas squeezed from both sides in turn keeps its minimum.
    let (mut app, mut h) = fresh();
    drag_far(&mut h, &mut app, B::CanvasTrajectory, false);
    drag_far(&mut h, &mut app, B::ManifoldCanvas, true);
    let squeezed = drawn(&app);
    check_minimums(&squeezed, "the canvas squeezed from both sides");
    near(
        squeezed.figure.width(),
        MIN_CANVAS_W,
        "the canvas at its minimum",
    );
    // Control: a splitter that closes the canvas.
    let mut closed = squeezed;
    closed.trajectory.min.x = closed.manifold.max.x;
    closed.figure.max.x = closed.figure.min.x;
    rejects("a splitter that closes the canvas", || {
        check_minimums(&closed, "closed")
    });
    // Control: a fixed layout passes the minimums but does not stop at one.
    let (fixed, _h) = fresh();
    rejects("a fixed layout (no drag stops at a minimum)", || {
        near(drawn(&fixed).manifold.width(), MIN_MANIFOLD_W, "fixed")
    });
}

#[test]
fn qa_m6_43_a_double_click_resets_that_splitter_only() {
    let (defaults, _h) = fresh();
    let defaults = drawn(&defaults);
    for b in ALL {
        let (mut app, mut h) = fresh();
        // Every boundary moved, each within its slack.
        for (o, by) in [
            (B::ManifoldCanvas, -12.0),
            (B::CanvasTrajectory, 30.0),
            (B::TopBottom, 20.0),
            (B::CompassTime, -40.0),
            (B::TimeLegend, 60.0),
        ] {
            drag_b(&mut h, &mut app, o, by);
        }
        let moved = drawn(&app);
        for o in ALL {
            assert!(
                (moved.edge(o) - defaults.edge(o)).abs() > 5.0,
                "{o:?} moved off its default"
            );
        }
        double_click(&mut h, &mut app, b);
        let after = drawn(&app);
        let check = |p: &Panels| {
            near(
                p.edge(b),
                defaults.edge(b),
                &format!("{b:?} back at its default"),
            );
            for o in ALL.into_iter().filter(|o| *o != b) {
                near(
                    p.edge(o),
                    moved.edge(o),
                    &format!("{b:?} reset: {o:?} left where it was"),
                );
            }
        };
        check(&after);
        // The double-click is the GUI's own: no SetField, no history (R-462 J3: reset is not undoable).
        // Controls: a splitter that does not reset, and a reset of every splitter.
        rejects(&format!("{b:?} not reset"), || check(&moved));
        rejects(
            &format!("{b:?}'s double-click resetting every splitter"),
            || check(&defaults),
        );
    }
}

#[test]
fn qa_m6_43_a_double_click_never_takes_a_panel_or_the_canvas_below_its_minimum() {
    // REQ-GUI-213: "each panel, and the canvas, must have a minimum size no splitter passes"; R-462 J2: "so no splitter
    // can close it". A neighbour dragged into the space the reset splitter's default needs: the reset may go no further
    // than the minimum lets it, toward its default, and moves no other splitter.
    let (defaults, _h) = fresh();
    let defaults = drawn(&defaults);
    let cases = [
        ResetCase {
            first: (B::CanvasTrajectory, true),
            second: (B::ManifoldCanvas, true),
            reset: B::CanvasTrajectory,
            size: |p| p.figure.width(),
            min: MIN_CANVAS_W,
            name: "the canvas",
        },
        ResetCase {
            first: (B::ManifoldCanvas, false),
            second: (B::CanvasTrajectory, false),
            reset: B::ManifoldCanvas,
            size: |p| p.figure.width(),
            min: MIN_CANVAS_W,
            name: "the canvas",
        },
        ResetCase {
            first: (B::TimeLegend, true),
            second: (B::CompassTime, true),
            reset: B::TimeLegend,
            size: |p| p.time.width(),
            min: MIN_TIME_W,
            name: "Time",
        },
        ResetCase {
            first: (B::CompassTime, false),
            second: (B::TimeLegend, false),
            reset: B::CompassTime,
            size: |p| p.time.width(),
            min: MIN_TIME_W,
            name: "Time",
        },
    ];
    // Every case is run and its outcome kept, so a failure names them all.
    let mut failed = Vec::new();
    for case in cases {
        let (mut app, mut h) = fresh();
        drag_far(&mut h, &mut app, case.first.0, case.first.1);
        drag_far(&mut h, &mut app, case.second.0, case.second.1);
        let moved = drawn(&app);
        check_minimums(&moved, "before the reset");
        double_click(&mut h, &mut app, case.reset);
        let after = drawn(&app);
        let what = format!(
            "{:?} reset beside {:?} at its far end",
            case.reset, case.second.0
        );
        let check = |after: &Panels| {
            check_minimums(after, &what);
            // As near its default as the minimum lets it: at the minimum.
            near(
                (case.size)(after),
                case.min,
                &format!("{what}: {}", case.name),
            );
            for o in ALL.into_iter().filter(|o| *o != case.reset) {
                near(
                    after.edge(o),
                    moved.edge(o),
                    &format!("{what}: {o:?} left where it was"),
                );
            }
        };
        // Control: the splitter put at its default whatever its neighbour, the panel between under its minimum.
        let mut at_default = moved;
        let d = defaults.edge(case.reset);
        match case.reset {
            B::ManifoldCanvas => {
                at_default.manifold.max.x = d;
                at_default.figure.min.x = d + 16.0;
            }
            B::CanvasTrajectory => {
                at_default.trajectory.min.x = d;
                at_default.figure.max.x = d;
            }
            B::CompassTime => {
                at_default.compass.max.x = d;
                at_default.time.min.x = d;
            }
            B::TimeLegend => {
                at_default.legend.min.x = d;
                at_default.time.max.x = d;
            }
            B::TopBottom => unreachable!(),
        }
        rejects(&format!("{what}: a reset that passes the minimum"), || {
            check(&at_default)
        });
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| check(&after)));
        if outcome.is_err() {
            failed.push(format!(
                "{what}: {} is {} points, minimum {}",
                case.name,
                (case.size)(&after),
                case.min
            ));
        }
    }
    assert!(
        failed.is_empty(),
        "a double-click passed a minimum: {failed:#?}"
    );
}

/// A reset scenario: `first` dragged to the window's edge (toward its max or not), then `second`, then `reset`
/// double-clicked; `size` reads the panel between, whose minimum is `min`.
struct ResetCase {
    first: (B, bool),
    second: (B, bool),
    reset: B,
    size: fn(&Panels) -> f32,
    min: f32,
    name: &'static str,
}

#[test]
fn qa_m6_43_a_splitter_shows_a_resize_cursor() {
    // R-462 J1: a splitter shows a resize cursor on hover.
    let (mut app, mut h) = fresh();
    for b in ALL {
        let at = drawn(&app).on(b);
        let _ = h.frame(&mut app, vec![Event::PointerMoved(at)]);
        let out = h.frame(&mut app, Vec::new());
        let want = if b.across() {
            [
                CursorIcon::ResizeHorizontal,
                CursorIcon::ResizeColumn,
                CursorIcon::ResizeEast,
                CursorIcon::ResizeWest,
            ]
        } else {
            [
                CursorIcon::ResizeVertical,
                CursorIcon::ResizeRow,
                CursorIcon::ResizeNorth,
                CursorIcon::ResizeSouth,
            ]
        };
        let got = out.platform_output.cursor_icon;
        assert!(want.contains(&got), "{b:?}: the cursor is {got:?}");
    }
    // Control: inside the canvas, away from every boundary, no resize cursor.
    let mid = drawn(&app).figure.center();
    let _ = h.frame(&mut app, vec![Event::PointerMoved(mid)]);
    let got = h.frame(&mut app, Vec::new()).platform_output.cursor_icon;
    rejects("the canvas's middle", || {
        assert!(
            matches!(
                got,
                CursorIcon::ResizeHorizontal | CursorIcon::ResizeVertical
            ),
            "{got:?}"
        )
    });
}

/// The rings drawn in one more frame after an unbound key, which shows the ring (R-464 M2) and acts on nothing.
fn rings(h: &mut Headless, app: &mut App<Recording>) -> Vec<Rect> {
    let key = |pressed| Event::Key {
        key: Key::F12,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: NONE,
    };
    let _ = h.frame(app, vec![key(true)]);
    let _ = h.frame(app, vec![key(false)]);
    let out = h.frame(app, Vec::new());
    let mut flat = Vec::new();
    fn flatten(shape: &egui::Shape, out: &mut Vec<egui::Shape>) {
        match shape {
            egui::Shape::Vec(v) => v.iter().for_each(|s| flatten(s, out)),
            other => out.push(other.clone()),
        }
    }
    for c in &out.shapes {
        flatten(&c.shape, &mut flat);
    }
    flat.into_iter()
        .filter_map(|s| match s {
            egui::Shape::Rect(r)
                if r.stroke.width == RING_WIDTH && r.stroke.color == NAVIGATION =>
            {
                Some(r.rect)
            }
            _ => None,
        })
        .collect()
}

/// The ring hugs `rect`: within a few points of it on every side, inside or out.
fn check_ring_on(rings: &[Rect], rect: Rect, what: &str) {
    assert_eq!(rings.len(), 1, "{what}: rings {rings:?}");
    let ring = rings[0];
    let slack = RING_WIDTH + 3.0;
    assert!(
        rect.expand(slack).contains_rect(ring) && ring.expand(slack).contains_rect(rect),
        "{what}: the ring {ring:?} is not on {rect:?}"
    );
}

#[test]
fn qa_m6_43_the_ring_follows_a_resized_scope() {
    // R-462 J6: the scope tree is unchanged, the ring following each scope's new rect. Tab from the start to each
    // resized scope, drag the boundary, and the ring is on the scope's new rect.
    for (scope, b, by) in [
        (FIGURE, B::CanvasTrajectory, 40.0_f32),
        (MANIFOLD_VIEW, B::ManifoldCanvas, -15.0),
        (LEGEND, B::TimeLegend, -80.0),
    ] {
        let (mut app, mut h) = fresh();
        let tab = |pressed| Event::Key {
            key: Key::Tab,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: NONE,
        };
        let mut reached = false;
        for _ in 0..30 {
            let _ = h.frame(&mut app, vec![tab(true)]);
            let _ = h.frame(&mut app, vec![tab(false)]);
            if app.view.focus.path.last().is_some_and(|id| id == scope) {
                reached = true;
                break;
            }
        }
        assert!(reached, "Tab reaches {scope}: {:?}", app.view.focus.path);
        let before = app.keyboard.place_of(scope).expect("placed").rect;
        check_ring_on(&rings(&mut h, &mut app), before, &format!("{scope} before"));
        drag_b(&mut h, &mut app, b, by);
        let after = app.keyboard.place_of(scope).expect("placed").rect;
        assert!(
            (after.width() - before.width()).abs() > 10.0,
            "{scope} resized"
        );
        let seen = rings(&mut h, &mut app);
        check_ring_on(&seen, after, &format!("{scope} after {b:?}"));
        // Control: a ring left on the scope's old rect.
        rejects(&format!("a ring left on {scope}'s old rect"), || {
            check_ring_on(&seen, before, "old")
        });
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// REQ-GUI-214: remembered and reset.

/// Moves every boundary, each within its slack.
fn change(h: &mut Headless, app: &mut App<Recording>) {
    for (b, by) in [
        (B::ManifoldCanvas, -14.0),
        (B::CanvasTrajectory, 50.0),
        (B::TopBottom, 25.0),
        (B::CompassTime, -70.0),
        (B::TimeLegend, 100.0),
    ] {
        drag_b(h, app, b, by);
    }
}

fn check_same_layout(a: &Panels, b: &Panels, what: &str) {
    for x in ALL {
        near(a.edge(x), b.edge(x), &format!("{what}: {x:?}"));
    }
    for (r, s, name) in [
        (a.figure, b.figure, "the canvas"),
        (a.trajectory, b.trajectory, "Trajectory"),
        (a.time, b.time, "Time"),
    ] {
        near(r.width(), s.width(), &format!("{what}: {name}'s width"));
        near(r.height(), s.height(), &format!("{what}: {name}'s height"));
    }
}

/// What the contract holds that a layout change must leave alone (gui_state_contract §2: the layout is never the
/// contract's).
fn contract(app: &App<Recording>) -> (String, String, String) {
    let s = app.snapshot();
    (
        format!("{:?}", s.sim),
        format!("{:?}", s.render),
        format!("{:?}", s.history),
    )
}

#[test]
fn qa_m6_43_a_layout_saved_comes_back_in_a_new_gui() {
    let dir = scratch("remembered");
    let path = dir.join("gui_settings.json");
    let (left, sent) = {
        let (mut app, mut h) = stored(&path);
        let held = contract(&app);
        let sent_before = app.side().sent.len();
        change(&mut h, &mut app);
        // Never the contract: no SetField, the state and its history as they were.
        assert_eq!(
            app.side().sent.len(),
            sent_before,
            "a drag sent {:?}",
            &app.side().sent[sent_before..]
        );
        assert_eq!(contract(&app), held, "a drag reached the contract");
        (drawn(&app), app.side().sent.len())
    };
    let _ = sent;
    let (defaults, _h) = fresh();
    let defaults = drawn(&defaults);
    for b in ALL {
        assert!((left.edge(b) - defaults.edge(b)).abs() > 5.0, "{b:?} moved");
    }
    let (again, _h) = stored(&path);
    check_same_layout(&drawn(&again), &left, "a new GUI on the same settings");
    // Control: a GUI that forgets the layout: one started with no store.
    rejects("a GUI that forgets the layout", || {
        check_same_layout(&defaults, &left, "forgot")
    });
    // And a second restart, with no drag between, still has it.
    drop(again);
    let (third, _h) = stored(&path);
    check_same_layout(&drawn(&third), &left, "a third GUI");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Clicks the drawn element named exactly `name`, read from one more frame's AccessKit names.
fn click_named(h: &mut Headless, app: &mut App<Recording>, name: &str) {
    let out = h.frame(app, Vec::new());
    let names = h.names(&out);
    let ppp = f64::from(PIXELS_PER_POINT);
    let r = names
        .iter()
        .filter(|n| n.name == name)
        .find_map(|n| n.rect)
        .unwrap_or_else(|| {
            panic!(
                "`{name}` drawn: {:?}",
                names.iter().map(|n| &n.name).collect::<Vec<_>>()
            )
        });
    let at = pos2(
        ((r[0] + r[2]) / 2.0 / ppp) as f32,
        ((r[1] + r[3]) / 2.0 / ppp) as f32,
    );
    for events in [
        vec![Event::PointerMoved(at)],
        vec![press(at, true)],
        vec![press(at, false)],
    ] {
        let _ = h.frame(app, events);
    }
    settle(h, app);
}

#[test]
fn qa_m6_43_view_reset_layout_restores_every_default_and_is_saved() {
    let dir = scratch("reset");
    let path = dir.join("gui_settings.json");
    let (defaults, _h) = fresh();
    let defaults = drawn(&defaults);
    {
        let (mut app, mut h) = stored(&path);
        change(&mut h, &mut app);
        let moved = drawn(&app);
        rejects("a layout left moved", || {
            check_same_layout(&moved, &defaults, "moved")
        });
        let held = contract(&app);
        let sent_before = app.side().sent.len();
        click_named(&mut h, &mut app, "View");
        click_named(&mut h, &mut app, "reset layout");
        check_same_layout(&drawn(&app), &defaults, "View › reset layout");
        // The GUI's own: no SetField, no history entry, so no undo brings the layout back (R-462 J3).
        assert_eq!(
            app.side().sent.len(),
            sent_before,
            "reset layout sent {:?}",
            &app.side().sent[sent_before..]
        );
        assert_eq!(contract(&app), held, "reset layout reached the contract");
    }
    // Saved: a new GUI on the same settings starts at the defaults.
    let (again, _h) = stored(&path);
    check_same_layout(&drawn(&again), &defaults, "a new GUI after reset layout");
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------------------------------------------------
// REQ-GUI-214: the re-frame.

/// The figure point `(s, t)` the app reports under pixel `p`: the contract's hover with the pointer resting there.
fn hover_at(h: &mut Headless, app: &mut App<Recording>, p: Pos2) -> [f64; 2] {
    let _ = h.frame(app, vec![Event::PointerMoved(p)]);
    settle(h, app);
    app.snapshot()
        .render
        .hover
        .at
        .unwrap_or_else(|| panic!("a hover at {p:?}"))
}

/// The chart point of the figure point `st` on the app's plane.
fn chart(app: &App<Recording>, st: [f64; 2]) -> Latent {
    point(&app.snapshot().sim.plane, st[0], st[1])
}

fn check_same_chart(a: &Latent, b: &Latent, what: &str) {
    for (x, y) in a.iter().zip(b) {
        assert!((x - y).abs() <= CHART_TOL, "{what}: {a:?} → {b:?}");
    }
}

/// The figure's map from screen points to `(s, t)`, `s = s0 + x·ds`, `t = t0 + y·dt`, read from the hover at two
/// pixels of the canvas `c`.
fn map_of(h: &mut Headless, app: &mut App<Recording>, c: Rect) -> [f64; 4] {
    let (p, q) = (
        pos2(c.min.x + 0.25 * c.width(), c.min.y + 0.25 * c.height()),
        pos2(c.min.x + 0.75 * c.width(), c.min.y + 0.75 * c.height()),
    );
    let (a, b) = (hover_at(h, app, p), hover_at(h, app, q));
    let ds = (b[0] - a[0]) / f64::from(q.x - p.x);
    let dt = (b[1] - a[1]) / f64::from(q.y - p.y);
    [
        a[0] - ds * f64::from(p.x),
        ds,
        a[1] - dt * f64::from(p.y),
        dt,
    ]
}

/// The span `[s0, t0, s1, t1]` of canvas `c` under `map`.
fn span_of(map: [f64; 4], c: Rect) -> [f64; 4] {
    let s = |x: f32| map[0] + map[1] * f64::from(x);
    let t = |y: f32| map[2] + map[3] * f64::from(y);
    [s(c.min.x), t(c.max.y), s(c.max.x), t(c.min.y)]
}

/// The integrated quads `b` keep `a`'s, every one; each added quad lies over `new` and not over `old`, the newly
/// exposed area's; and `b`'s quads cover the whole of `new` (the view shows no hole).
fn check_quads(a: &[Quad], b: &[Quad], old: Quad, new: Quad) {
    for q in a {
        assert!(b.contains(q), "the integrated quad {q:?} was dropped");
    }
    let added: Vec<&Quad> = b.iter().filter(|q| !a.contains(q)).collect();
    for q in &added {
        assert!(quads::overlaps(q, &new), "{q:?} is not over the canvas");
        assert!(
            !quads::overlaps(q, &old),
            "{q:?} was over the old canvas: re-integrated"
        );
    }
    for i in 0..=20 {
        for j in 0..=20 {
            let u = new[0] + (new[2] - new[0]) * (0.001 + 0.998 * f64::from(i) / 20.0);
            let v = new[1] + (new[3] - new[1]) * (0.001 + 0.998 * f64::from(j) / 20.0);
            assert!(
                b.iter()
                    .any(|q| q[0] <= u && u <= q[2] && q[1] <= v && v <= q[3]),
                "({u}, {v}) of the canvas is covered by no quad"
            );
        }
    }
}

#[test]
fn qa_m6_43_a_wider_canvas_is_reframed_never_reintegrated() {
    // Each boundary that widens or heightens the canvas, by a drag within its slack.
    for (b, by) in [
        (B::CanvasTrajectory, 80.0_f32),
        (B::ManifoldCanvas, -16.0),
        (B::TopBottom, 40.0),
    ] {
        let (mut app, mut h) = fresh();
        let c0 = drawn(&app).figure;
        // Pixels well inside the canvas, which stay inside it.
        let pixels: Vec<Pos2> = [(0.3, 0.3), (0.5, 0.5), (0.7, 0.2), (0.2, 0.8), (0.9, 0.6)]
            .iter()
            .map(|(x, y)| pos2(c0.min.x + x * c0.width(), c0.min.y + y * c0.height()))
            .collect();
        let before: Vec<Latent> = pixels
            .iter()
            .map(|p| {
                let st = hover_at(&mut h, &mut app, *p);
                chart(&app, st)
            })
            .collect();
        let map0 = map_of(&mut h, &mut app, c0);
        let away = drawn(&app).legend.center();
        let _ = h.frame(&mut app, vec![Event::PointerMoved(away)]);
        settle(&mut h, &mut app);
        let quads0 = app.side().inner.engine().integrated().to_vec();
        assert!(!quads0.is_empty(), "the mock integrated the canvas's quads");
        let sent0 = app.side().sent.len();
        let held = contract(&app);

        drag_b(&mut h, &mut app, b, by);

        // No SetField: nothing sent during the drag, the state and its history as they were.
        let sent: Vec<SetField> = app.side().sent[sent0..].to_vec();
        let no_set_field =
            |sent: &[SetField]| assert!(sent.is_empty(), "{b:?}: the resize sent {sent:?}");
        no_set_field(&sent);
        assert_eq!(
            contract(&app),
            held,
            "{b:?}: the resize reached the contract"
        );
        let quads1 = app.side().inner.engine().integrated().to_vec();
        let c1 = drawn(&app).figure;
        assert!(
            c1.width() * c1.height() > c0.width() * c0.height() + 100.0,
            "{b:?}: the canvas grew"
        );

        // The same chart point under each fixed pixel, to 1e-9.
        let after: Vec<Latent> = pixels
            .iter()
            .map(|p| {
                let st = hover_at(&mut h, &mut app, *p);
                chart(&app, st)
            })
            .collect();
        for ((p, x), y) in pixels.iter().zip(&before).zip(&after) {
            check_same_chart(x, y, &format!("{b:?}: the point under {p:?}"));
        }
        // At the same scale everywhere on the canvas: the same map, the newly exposed strip read on it.
        let map1 = map_of(&mut h, &mut app, c1);
        let same_map = |m: [f64; 4]| {
            for k in 0..4 {
                assert!(
                    (m[k] - map0[k]).abs() <= 1e-6 * map0[k].abs().max(1e-3),
                    "{b:?}: the map {m:?}, was {map0:?}"
                );
            }
        };
        same_map(map1);
        // The newly exposed strip is drawn on the same map: the engine side covers it with quads (below). The hover
        // there is R-537's (M8): the GUI reads no figure point past the default figure rect yet, so it is not read here.
        // The quads: kept, and only the newly exposed area's added.
        let plane = app.snapshot().sim.plane.clone();
        let old = quads::view_of(&plane, span_of(map0, c0)).expect("the old view");
        let new = quads::view_of(&plane, span_of(map0, c1)).expect("the new view");
        check_quads(&quads0, &quads1, old, new);

        // Controls (R-176): a resize that sends a SetField; one that stretches the figure (the old span over the new
        // canvas); one that drops the integrated quads; one that re-integrates the view from scratch.
        let one = vec![gui::explore::hover_label::hover_edit(None)];
        rejects(&format!("{b:?}: a resize that sends a SetField"), || {
            no_set_field(&one)
        });
        let stretched = {
            let (sx, sy) = (
                f64::from(c0.width() / c1.width()),
                f64::from(c0.height() / c1.height()),
            );
            [map0[0], map0[1] * sx, map0[2], map0[3] * sy]
        };
        rejects(
            &format!("{b:?}: a resize that stretches the figure"),
            || same_map(stretched),
        );
        rejects(&format!("{b:?}: a resize that drops the quads"), || {
            check_quads(&quads0, &quads1[..quads1.len() / 2], old, new)
        });
        let redone: Vec<Quad> = quads0
            .iter()
            .map(|q| [q[0], q[1], q[2], q[3] + 1e-9])
            .chain(quads1.iter().copied())
            .collect();
        rejects(
            &format!("{b:?}: a resize that re-integrates the old view"),
            || check_quads(&quads0, &redone, old, new),
        );
    }
}

#[test]
fn qa_m6_43_a_narrower_canvas_then_widened_back_integrates_nothing() {
    // A canvas narrowed shows less of the field at the same scale; widened back over what it showed, it needs nothing
    // new: the quads kept are redrawn, none computed again.
    let (mut app, mut h) = fresh();
    let c0 = drawn(&app).figure;
    let p = pos2(c0.min.x + 0.3 * c0.width(), c0.center().y);
    let st0 = hover_at(&mut h, &mut app, p);
    let away = drawn(&app).legend.center();
    let _ = h.frame(&mut app, vec![Event::PointerMoved(away)]);
    settle(&mut h, &mut app);
    let quads0 = app.side().inner.engine().integrated().to_vec();
    let sent0 = app.side().sent.len();
    drag_b(&mut h, &mut app, B::CanvasTrajectory, -200.0);
    let narrow = drawn(&app).figure;
    assert!(narrow.width() < c0.width() - 150.0, "narrowed");
    let quads_narrow = app.side().inner.engine().integrated().to_vec();
    assert_eq!(
        quads_narrow, quads0,
        "a narrower canvas integrated or dropped quads"
    );
    drag_b(&mut h, &mut app, B::CanvasTrajectory, 200.0);
    near(drawn(&app).figure.width(), c0.width(), "widened back");
    let quads1 = app.side().inner.engine().integrated().to_vec();
    assert_eq!(
        quads1, quads0,
        "a canvas widened back over what it showed integrated again"
    );
    assert_eq!(
        app.side().sent.len(),
        sent0,
        "the resizes sent {:?}",
        &app.side().sent[sent0..]
    );
    let st1 = hover_at(&mut h, &mut app, p);
    check_same_chart(
        &chart(&app, st0),
        &chart(&app, st1),
        "the point under the pixel",
    );
}
