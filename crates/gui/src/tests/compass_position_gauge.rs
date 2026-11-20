//! `compass_position_gauge` (REQ-GUI-196; render_gui_spec § "The compass: look and views (R-451, R-452)"; R-452):
//! under the Plücker view, `z₀` as one signed bar per latent coordinate, each its component over the slider's
//! half-range clamped to ±1, the slice axis highlighted and following the preset; a slice edit moves its bar and leaves
//! the grid; the readout takes no input. Each check also runs on an input it must reject (R-176, through `rejects`).

use eframe::egui::{self, Shape};
use engine::contract::sim_config::{Plane, LATENT_DIM};

use super::compass_plucker::plucker_app;
use super::compass_support::{plane, setfields, square, view};
use super::support::{headless, mock_app, rejects, shapes};
use crate::explore::compass::views::{
    gauge, plucker, plucker_drawing, CompassView, GridPlace, GAUGE_H,
};
use crate::explore::manifold_view::{basis_edit, z0_edit, View, Z0_HALF_RANGE};

/// Asserts each bar is its `z₀` component over the half-range, clamped to ±1.
fn check_bars(z0: &[f64; LATENT_DIM], bars: &[f64; LATENT_DIM]) {
    for k in 0..LATENT_DIM {
        let want = (z0[k] / Z0_HALF_RANGE).clamp(-1.0, 1.0);
        assert_eq!(bars[k], want, "bar {k}");
    }
}

#[test]
fn compass_position_gauge_each_bar_is_its_component_over_the_half_range_clamped() {
    let z0 = [0.5, -0.25, 1.0, -1.0, 1.5, -2.0, 0.0, 1.0 + 1e-12];
    let bars = gauge(&z0);
    assert_eq!(bars, [0.5, -0.25, 1.0, -1.0, 1.0, -1.0, 0.0, 1.0]);
    check_bars(&z0, &bars);
    rejects("bars left unclamped", || check_bars(&z0, &z0));
    // Drawn: each bar from the zero line, up for a positive value, its length the value's share of the half-height.
    let place = GridPlace::fit(egui::Rect::from_min_size(
        egui::pos2(10.0, 20.0),
        egui::vec2(172.0, 172.0),
    ));
    let mid = place.gauge_rect().center().y;
    for (k, v) in bars.iter().enumerate() {
        let r = place.bar_rect(k, *v);
        assert!(
            (r.height() - v.abs() as f32 * GAUGE_H / 2.0).abs() < 1e-4,
            "bar {k}'s length"
        );
        if *v > 0.0 {
            assert_eq!(r.max.y, mid, "bar {k} rises from the zero line");
        } else {
            assert_eq!(r.min.y, mid, "bar {k} falls from the zero line");
        }
        let column = place.cell_rect(0, k);
        assert!(
            (r.center().x - column.center().x).abs() < 1e-4,
            "bar {k} under its column"
        );
        assert!(
            place.gauge_rect().contains_rect(r),
            "bar {k} in the readout"
        );
    }
    // The readout is under the grid.
    assert!(place.gauge_rect().min.y > place.rect().max.y);
}

#[test]
fn compass_position_gauge_highlights_the_slice_axis_and_follows_the_preset() {
    let visuals = egui::Visuals::dark();
    let mut seen = Vec::new();
    for (q1, q2, axis) in [(0, 1, 5), (4, 5, 0), (6, 7, 5)] {
        let mut p = plane(&view(0.0, 0.0, 0.0, 1.0), [0.2, -0.4, 0.3]);
        p.q1 = std::array::from_fn(|k| if k == q1 { 1.0 } else { 0.0 });
        p.q2 = std::array::from_fn(|k| if k == q2 { 1.0 } else { 0.0 });
        let (mut app, mut h) = plucker_app(&p);
        let drawing = plucker_drawing(square(&h), &app.snapshot().sim.plane, None);
        assert_eq!(
            drawing.slice_axis, axis,
            "the slice axis of z_{q1} × z_{q2}"
        );
        assert_eq!(drawing.slice_axis, View::of(&app.snapshot().sim.plane).d);
        let shown = shapes(&mut h, &mut app);
        let bar_colour = |k: usize| {
            let at = drawing.place.bar_rect(k, drawing.bars[k]);
            shown
                .iter()
                .find_map(|s| match s {
                    Shape::Rect(r) if r.rect == at => Some(r.fill),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("bar {k} drawn"))
        };
        for k in 0..LATENT_DIM {
            let want = if k == axis {
                visuals.hyperlink_color
            } else {
                visuals.weak_text_color()
            };
            assert_eq!(bar_colour(k), want, "bar {k} of z_{q1} × z_{q2}");
        }
        seen.push((drawing.slice_axis, axis));
    }
    rejects(
        "the slice axis highlighted where it was, the preset changed",
        || {
            for (drawn, _) in &seen {
                assert_eq!(*drawn, 5);
            }
        },
    );
}

#[test]
fn compass_position_gauge_a_slice_edit_moves_its_bar_and_leaves_the_grid() {
    let p = plane(&view(25.0, -10.0, 15.0, 1.0), [0.1, 0.2, 0.3]);
    let (mut app, mut h) = plucker_app(&p);
    let before = plucker_drawing(square(&h), &app.snapshot().sim.plane, None);
    let d = before.slice_axis;
    let mut z0 = app.snapshot().sim.plane.z0;
    z0[d] += 0.3;
    app.set_field(z0_edit(z0));
    let _ = h.frame(&mut app, Vec::new());
    let after = plucker_drawing(square(&h), &app.snapshot().sim.plane, None);
    for k in 0..LATENT_DIM {
        let moved = after.bars[k] - before.bars[k];
        let want = if k == d { 0.3 } else { 0.0 };
        assert!((moved - want).abs() < 1e-12, "bar {k} moved {moved}");
    }
    assert_eq!(
        after.grid, before.grid,
        "the grid unchanged by a slice edit"
    );
    // A tilt is no slice edit: it changes the grid.
    let mut q = app.snapshot().sim.plane.clone();
    q.q2[d] += 0.4;
    app.set_field(basis_edit((q.q1, q.q2)));
    let _ = h.frame(&mut app, Vec::new());
    let tilted = plucker(&app.snapshot().sim.plane);
    rejects("a grid that a tilt leaves", || {
        assert_eq!(tilted, after.grid)
    });
}

#[test]
fn compass_position_gauge_takes_no_input() {
    let p = plane(&view(25.0, -10.0, 15.0, 1.0), [0.1, 0.2, 0.3]);
    let (mut app, mut h) = plucker_app(&p);
    let place = GridPlace::fit(square(&h));
    let gauge_rect = place.gauge_rect();
    let check_nothing =
        |app: &crate::app::App<crate::side::MockSide>, rows: usize, depth: u32, plane: &Plane| {
            assert!(
                setfields(app, rows).is_empty(),
                "a SetField from the readout"
            );
            assert_eq!(app.snapshot().history.undo_depth, depth);
            assert_eq!(&app.snapshot().sim.plane, plane);
            assert_eq!(app.manifold.compass.view, CompassView::Plucker);
        };
    for at in [
        gauge_rect.center(),
        place.bar_rect(2, 1.0).center(),
        gauge_rect.left_bottom() + egui::vec2(2.0, -2.0),
    ] {
        let (rows, depth) = (app.console().len(), app.snapshot().history.undo_depth);
        let plane = app.snapshot().sim.plane.clone();
        for events in crate::capture::click(at) {
            let _ = h.frame(&mut app, events);
        }
        let _ = h.frame(&mut app, Vec::new());
        check_nothing(&app, rows, depth, &plane);
    }
    // A click on a cell is input: the same check rejects it.
    let (rows, depth) = (app.console().len(), app.snapshot().history.undo_depth);
    let plane = app.snapshot().sim.plane.clone();
    for events in crate::capture::click(place.cell_rect(0, 3).center()) {
        let _ = h.frame(&mut app, events);
    }
    let _ = h.frame(&mut app, Vec::new());
    rejects("a click on a cell", || {
        check_nothing(&app, rows, depth, &plane)
    });
    // A fresh app draws no gauge in the Cube view: it is the Plücker view's.
    let mut fresh = mock_app();
    let mut h2 = headless();
    let _ = h2.frame(&mut fresh, Vec::new());
    assert_eq!(fresh.manifold.compass.view, CompassView::Cube);
}
