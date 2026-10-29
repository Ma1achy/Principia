//! `mock_layout_reframe` (REQ-GUI-214; R-462 J4; R-406's rule; render_gui_spec §G2's Layout): on the mock, a splitter
//! drag that widens the canvas sends no `SetField`, keeps the field point under a fixed screen pixel the same (to 1e-9
//! in chart coordinates) and leaves the mock's integrated quads in place, only quads in the newly exposed area
//! integrated; a drag that narrows it integrates nothing. Each check also runs on an outcome it must reject (R-176,
//! through `rejects`; gui takes no dependency on validation, R-187): a canvas resize that sends a `SetField`, one that
//! stretches the figure, and one that drops the integrated quads.

use eframe::egui::{pos2, Event, Pos2};
use engine::contract::log::Source;
use engine::contract::snapshot::Snapshot;

use super::mock_layout_splitters::{current, drag_splitter};
use super::support::{headless, mock_app, rejects};
use crate::app::App;
use crate::explore::manifold_view::navigate::{figure_coords, pan};
use crate::explore::manifold_view::point;
use crate::headless::Headless;
use crate::layout::{Layout, Splitter};
use crate::mock::quads::{self, Quad};
use crate::side::{EngineSide, MockSide};

/// What a re-frame must leave alone, read before and after it.
#[derive(Clone, Debug)]
struct Seen {
    snapshot: Snapshot,
    /// The console's `SetField` lines from the contract so far.
    set_fields: usize,
    quads: Vec<Quad>,
    layout: Layout,
}

fn seen(h: &mut Headless, app: &mut App<MockSide>) -> Seen {
    let _ = h.frame(app, Vec::new());
    Seen {
        snapshot: app.snapshot().clone(),
        set_fields: app
            .console()
            .iter()
            .filter(|e| e.source == Source::Contract && e.message.starts_with("SetField"))
            .count(),
        quads: app.side().engine().integrated().to_vec(),
        layout: current(h, app),
    }
}

/// An app on the mock, settled, the pointer off the figure, in the Legend.
fn settled() -> (App<MockSide>, Headless) {
    let mut app = mock_app();
    let mut h = headless();
    let away = crate::layout::Layout::new(h.screen(), h.pixels_per_point())
        .legend
        .center();
    let _ = h.frame(&mut app, vec![Event::PointerMoved(away)]);
    for _ in 0..3 {
        let _ = h.frame(&mut app, Vec::new());
    }
    (app, h)
}

/// No `SetField` between `a` and `b`: none logged, the state and its history as they were.
fn check_no_set_field(a: &Seen, b: &Seen) {
    assert_eq!(b.set_fields, a.set_fields, "a SetField was sent");
    assert_eq!(b.snapshot.sim, a.snapshot.sim);
    assert_eq!(b.snapshot.render, a.snapshot.render);
    assert_eq!(b.snapshot.history, a.snapshot.history);
}

/// The chart point under the screen pixel `p` in `layout`, read as the figure reads it, in its field.
fn chart_point(
    seen: &Seen,
    field: eframe::egui::Rect,
    p: Pos2,
) -> engine::contract::sim_config::Latent {
    let (s, t) = figure_coords(field, p);
    point(&seen.snapshot.sim.plane, s, t)
}

/// The same chart point under `p` before and after, to 1e-9 in each coordinate.
fn check_same_point(a: &Seen, b: &Seen, field_b: eframe::egui::Rect, p: Pos2) {
    let (za, zb) = (
        chart_point(a, a.layout.field, p),
        chart_point(b, field_b, p),
    );
    for (x, y) in za.iter().zip(zb) {
        assert!(
            (x - y).abs() <= 1e-9,
            "the chart point under {p:?} moved: {za:?} → {zb:?}"
        );
    }
}

/// `b`'s quads are `a`'s, in place, then quads over `b`'s span that `a`'s span did not reach: the newly exposed area's.
fn check_quads_kept(a: &Seen, b: &Seen) {
    let plane = &b.snapshot.sim.plane;
    assert!(b.quads.len() >= a.quads.len(), "quads dropped");
    assert_eq!(
        b.quads[..a.quads.len()],
        a.quads[..],
        "the integrated quads not kept in place"
    );
    let old = quads::view_of(plane, a.layout.span(true)).expect("the old span");
    let new = quads::view_of(plane, b.layout.span(true)).expect("the new span");
    for q in &b.quads[a.quads.len()..] {
        assert!(!quads::overlaps(q, &old), "{q:?} was already in view");
        assert!(quads::overlaps(q, &new), "{q:?} is not in view");
    }
}

#[test]
fn mock_layout_reframe_a_wider_canvas_shows_more_of_the_field_at_the_same_scale() {
    // Each splitter that can widen the canvas. The Manifold view's 16 points and the bottom row's 30 expose strips
    // that the quads straddling the old edge already cover, so they integrate none; the Trajectory panel's 60 do.
    for (splitter, by, adds) in [
        (Splitter::FigureTrajectory, 60.0, true),
        (Splitter::ManifoldFigure, -16.0, false),
        (Splitter::TopBottom, 30.0, false),
    ] {
        let (mut app, mut h) = settled();
        let a = seen(&mut h, &mut app);
        assert_eq!(a.layout.span(true), [0.0, 0.0, 1.0, 1.0]);
        assert!(
            !a.quads.is_empty(),
            "the mock integrated the shown figure's quads"
        );
        drag_splitter(&mut h, &mut app, splitter, by);
        let b = seen(&mut h, &mut app);
        assert!(
            b.layout.figure.area() > a.layout.figure.area(),
            "{splitter:?} widened the canvas"
        );
        check_no_set_field(&a, &b);
        // The field stays where it was: its placement, and the chart point under fixed pixels, one inside the old
        // canvas and one the drag exposed.
        assert_eq!(b.layout.field, a.layout.field);
        assert_eq!(b.layout.placement(), a.layout.placement());
        let inside = pos2(a.layout.figure.min.x + 101.0, a.layout.figure.max.y - 77.0);
        check_same_point(&a, &b, b.layout.field, inside);
        let exposed = match splitter {
            Splitter::FigureTrajectory => {
                pos2(a.layout.figure.max.x + 30.0, a.layout.figure.center().y)
            }
            Splitter::ManifoldFigure => {
                pos2(a.layout.figure.min.x - 8.0, a.layout.figure.center().y)
            }
            _ => pos2(a.layout.figure.center().x, a.layout.figure.max.y + 15.0),
        };
        assert!(b.layout.figure.contains(exposed) && !a.layout.figure.contains(exposed));
        let (s, t) = figure_coords(b.layout.field, exposed);
        assert!(
            !(0.0..=1.0).contains(&s) || !(0.0..=1.0).contains(&t),
            "past [0, 1]²"
        );
        check_same_point(&a, &b, b.layout.field, exposed);
        check_quads_kept(&a, &b);
        assert_eq!(
            b.quads.len() > a.quads.len(),
            adds,
            "{splitter:?}: the exposed area's quads"
        );
        // Exactly the quads over the new span that the old one did not reach.
        let plane = &b.snapshot.sim.plane;
        let old = quads::view_of(plane, a.layout.span(true)).expect("the old span");
        let exposed_quads = quads::quads_over(plane, b.layout.span(true))
            .into_iter()
            .filter(|q| !quads::overlaps(q, &old))
            .count();
        assert_eq!(b.quads.len() - a.quads.len(), exposed_quads, "{splitter:?}");
        // The app's own hover reads the same field point under the pixel inside.
        let _ = h.frame(&mut app, vec![Event::PointerMoved(inside)]);
        let _ = h.frame(&mut app, Vec::new());
        let (s, t) = figure_coords(a.layout.field, inside);
        assert_eq!(app.snapshot().render.hover.at, Some([s, t]));
        // A canvas resize that stretches the figure, its field the new canvas, fails.
        rejects("a resize that stretches the figure", || {
            check_same_point(&a, &b, b.layout.figure, inside)
        });
    }
}

#[test]
fn mock_layout_reframe_a_narrower_canvas_integrates_nothing() {
    let (mut app, mut h) = settled();
    let a = seen(&mut h, &mut app);
    drag_splitter(&mut h, &mut app, Splitter::FigureTrajectory, -60.0);
    let b = seen(&mut h, &mut app);
    check_no_set_field(&a, &b);
    assert_eq!(b.quads, a.quads, "kept, and none added");
    let p = pos2(a.layout.figure.min.x + 40.0, a.layout.figure.center().y);
    check_same_point(&a, &b, b.layout.field, p);
    // Widened back, nothing is integrated again: the quads are still held.
    drag_splitter(&mut h, &mut app, Splitter::FigureTrajectory, 60.0);
    let c = seen(&mut h, &mut app);
    assert_eq!(c.quads, a.quads);
}

#[test]
fn mock_layout_reframe_controls_a_resize_that_sends_a_set_field_or_drops_the_quads() {
    let (mut app, mut h) = settled();
    let a = seen(&mut h, &mut app);
    drag_splitter(&mut h, &mut app, Splitter::FigureTrajectory, 60.0);
    // A canvas resize that also sent a SetField: a pan, as a resize that re-centred the view would.
    let edit = pan(
        &a.snapshot.sim.plane,
        eframe::egui::vec2(30.0, 0.0),
        a.layout.field.size(),
    );
    app.set_field(edit);
    let b = seen(&mut h, &mut app);
    rejects("a canvas resize that sends a SetField", || {
        check_no_set_field(&a, &b)
    });
    // The SetField moved the sim key: the mock integrated afresh, dropping the quads it held.
    rejects("a canvas resize that drops the integrated quads", || {
        check_quads_kept(&a, &b)
    });
}

#[test]
fn mock_layout_reframe_the_mock_integrates_only_what_it_does_not_hold() {
    let mut engine = crate::mock::MockEngine::new();
    assert!(engine.integrated().is_empty());
    engine.show_span([0.0, 0.0, 1.0, 1.0]);
    let shown = engine.integrated().to_vec();
    assert_eq!(shown, quads::quads(engine.plane()));
    // The same span again adds nothing; a wider one adds what it exposes, after.
    engine.show_span([0.0, 0.0, 1.0, 1.0]);
    assert_eq!(engine.integrated(), &shown[..]);
    engine.show_span([0.0, 0.0, 1.5, 1.0]);
    let wider = engine.integrated().to_vec();
    assert_eq!(wider[..shown.len()], shown[..]);
    assert!(wider.len() > shown.len());
    let mut all = quads::quads_over(engine.plane(), [0.0, 0.0, 1.5, 1.0]);
    let mut got = wider.clone();
    let key = |q: &Quad| q.map(f64::to_bits);
    all.sort_by_key(key);
    got.sort_by_key(key);
    assert_eq!(got, all, "every quad over the span, once");
    // Back to the narrower span: nothing dropped.
    engine.show_span([0.0, 0.0, 1.0, 1.0]);
    assert_eq!(engine.integrated(), &wider[..]);
    // An edit of the sim key starts over.
    use engine::contract::interface::EngineInterface;
    let plane = engine.plane().clone();
    engine.set_field(pan(
        &plane,
        eframe::egui::vec2(50.0, 0.0),
        eframe::egui::vec2(400.0, 400.0),
    ));
    engine.show_span([0.0, 0.0, 1.0, 1.0]);
    assert_eq!(engine.integrated(), &quads::quads(engine.plane())[..]);
    rejects("the quads of the chart before the edit kept", || {
        assert_eq!(engine.integrated(), &shown[..])
    });
    // An edit not of the sim key keeps them.
    engine.set_field(super::support::playhead(2.0, false));
    let held = engine.integrated().to_vec();
    engine.show_span([0.0, 0.0, 1.0, 1.0]);
    assert_eq!(engine.integrated(), &held[..]);
}

#[test]
fn mock_layout_reframe_the_span_shown_follows_the_canvas_and_the_layer() {
    let l = Layout::new(headless().screen(), crate::capture::PIXELS_PER_POINT);
    assert_eq!(l.span(true), [0.0, 0.0, 1.0, 1.0]);
    assert_eq!(l.span(false), crate::layout::span_of(l.window, l.field));
    // The hidden layer's span is the window's span of the field at its placement, as the canvas draws it.
    for (a, b) in l.span(false).iter().zip(l.placement().span()) {
        assert!((a - b).abs() < 1e-5, "{a} against {b}");
    }
    let hidden = l.span(false);
    assert!(hidden[0] < 0.0 && hidden[1] < 0.0 && hidden[2] > 1.0 && hidden[3] > 1.0);
    // The real engine side takes the span and does nothing with it, sending nothing.
    use engine::contract::interface::EngineInterface;
    let mut real = super::support::real_app();
    let before = real.snapshot().clone();
    real.side().show_span(hidden);
    assert_eq!(real.side().engine().snapshot().history, before.history);
}

#[test]
fn mock_layout_reframe_the_axis_labels_read_the_canvas_ends() {
    use crate::explore::axis_labels::{axes, axes_over};
    let plane = crate::mock::MockEngine::new().plane().clone();
    // Over [0, 1]², the ends of the default figure, exactly.
    assert_eq!(axes_over(&plane, [0.0, 0.0, 1.0, 1.0]), axes(&plane));
    // A canvas half as wide again each way: each end read at the canvas's edge, `z₀ + q (2s − 1)`.
    let [h, v] = axes_over(&plane, [-0.25, -0.5, 1.25, 1.5]);
    let near = |a: f64, b: f64| assert!((a - b).abs() < 1e-12, "{a} against {b}");
    near(h.low, plane.z0[0] - 1.5 * plane.q1[0]);
    near(h.high, plane.z0[0] + 1.5 * plane.q1[0]);
    near(v.low, plane.z0[1] - 2.0 * plane.q2[1]);
    near(v.high, plane.z0[1] + 2.0 * plane.q2[1]);
    rejects("the default figure's ends on a wider canvas", || {
        near(h.low, axes(&plane)[0].low)
    });
}
