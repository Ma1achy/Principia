//! `mock_compass_views` (REQ-GUI-194; render_gui_spec § "The compass: look and views (R-451, R-452)", §G2; R-452 C1):
//! the compass opens on Cube; each view button shows its view and leaves the mode; Tilt and Slice leave the view;
//! touching a slice slider in the Plücker view sets Slice and keeps Plücker; a drag in a grid view and a view switch
//! emit no `SetField` and add nothing to undo. Each check also runs on an input it must reject (R-176, through
//! `rejects`).

use eframe::egui::{self, Key, Modifiers, Shape};

use super::compass_support::{
    compass_rect, compass_shapes, drag, painted_edges, plane, setfields, settle, square, view,
};
use super::support::{click_scope, headless, mock_app, press, rejects};
use crate::app::App;
use crate::explore::compass::views::{self, CompassView, GridPlace, VIEWS, WAITS};
use crate::explore::compass::{self, SLICE_BUTTON, TILT_BUTTON};
use crate::explore::manifold_view::Touch;
use crate::headless::Headless;
use crate::side::MockSide;

const NONE: Modifiers = Modifiers::NONE;

/// The app on the mock with the plane tilted, at rest.
fn tilted_app() -> (App<MockSide>, Headless) {
    super::compass_support::app_at(&plane(&view(30.0, -20.0, 25.0, 1.0), [0.1, -0.2, 0.2]))
}

/// Whether each of the view buttons is drawn selected, from one more frame's AccessKit tree, in [`VIEWS`]' order.
fn selected_views(h: &mut Headless, app: &mut App<MockSide>) -> Vec<bool> {
    let output = h.frame(app, Vec::new());
    let update = output
        .platform_output
        .accesskit_update
        .expect("an AccessKit tree");
    VIEWS
        .iter()
        .map(|(_, _, label)| {
            let node = update
                .nodes
                .iter()
                .map(|(_, n)| n)
                .find(|n| n.label() == Some(label))
                .unwrap_or_else(|| panic!("no {label} button"));
            node.toggled() == Some(egui::accesskit::Toggled::True)
        })
        .collect()
}

/// What the compass draws in `shapes`, by view: the cube's twelve edges, the Plücker grid's 28 cells, or the note
/// that the view waits on physics review.
fn drawn_view(shapes: &[Shape], h: &Headless, app: &App<MockSide>) -> CompassView {
    let cube = compass::current(compass_rect(h), &app.snapshot().sim, &app.manifold.compass);
    if painted_edges(shapes, &cube).len() == 12 {
        return CompassView::Cube;
    }
    let place = GridPlace::fit(square(h));
    let cells = views::upper()
        .filter(|&(i, j)| {
            let at = place.cell_rect(i, j).shrink(0.5);
            shapes
                .iter()
                .any(|s| matches!(s, Shape::Rect(r) if r.rect == at))
        })
        .count();
    let waits = shapes
        .iter()
        .any(|s| matches!(s, Shape::Text(t) if t.galley.text() == WAITS));
    match (cells, waits) {
        (28, false) => CompassView::Plucker,
        (0, true) => {
            // The Jacobi and Shape views draw the same note: their selected button tells them apart.
            app.manifold.compass.view
        }
        other => panic!("the compass draws no view: {other:?}"),
    }
}

/// Asserts the compass shows `view` in `mode`: its state, its buttons and its drawing.
fn check_shows(
    h: &mut Headless,
    app: &mut App<MockSide>,
    view: CompassView,
    mode: Touch,
    what: &str,
) {
    assert_eq!(app.manifold.compass.view, view, "{what}: the view");
    assert_eq!(app.manifold.compass.mode, mode, "{what}: the mode");
    let want: Vec<bool> = VIEWS.iter().map(|(v, _, _)| *v == view).collect();
    assert_eq!(selected_views(h, app), want, "{what}: the view buttons");
    let shapes = compass_shapes(h, app);
    assert_eq!(drawn_view(&shapes, h, app), view, "{what}: what it draws");
}

#[test]
fn mock_compass_views_the_compass_opens_on_cube() {
    assert_eq!(CompassView::default(), CompassView::Cube);
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    check_shows(&mut h, &mut app, CompassView::Cube, Touch::Tilt, "opened");
    assert_eq!(
        VIEWS.map(|(_, _, l)| l),
        ["Cube", "Plücker", "Jacobi", "Shape"],
        "the buttons, in order"
    );
    rejects("a compass opening on Plücker", || {
        check_shows(
            &mut h,
            &mut app,
            CompassView::Plucker,
            Touch::Tilt,
            "opened",
        )
    });
}

#[test]
fn mock_compass_views_each_view_button_shows_its_view_and_leaves_the_mode() {
    for mode in [Touch::Tilt, Touch::Slice] {
        let (mut app, mut h) = tilted_app();
        app.manifold.compass.mode = mode;
        for (view, id, label) in VIEWS.iter().rev().chain(VIEWS.iter()) {
            click_scope(&mut h, &mut app, id);
            let _ = h.frame(&mut app, Vec::new());
            check_shows(&mut h, &mut app, *view, mode, label);
        }
        // Control: a view switch that changes the mode.
        click_scope(&mut h, &mut app, crate::explore::COMPASS_PLUCKER);
        let other = match mode {
            Touch::Tilt => Touch::Slice,
            Touch::Slice => Touch::Tilt,
        };
        app.manifold.compass.mode = other;
        rejects("a view switch that changes the mode", || {
            check_shows(&mut h, &mut app, CompassView::Plucker, mode, "Plücker")
        });
    }
}

#[test]
fn mock_compass_views_tilt_and_slice_leave_the_view() {
    for (view, id, _) in VIEWS {
        let (mut app, mut h) = tilted_app();
        click_scope(&mut h, &mut app, id);
        for (button, mode) in [(SLICE_BUTTON, Touch::Slice), (TILT_BUTTON, Touch::Tilt)] {
            super::support::click(&mut h, &mut app, button);
            let _ = h.frame(&mut app, Vec::new());
            check_shows(&mut h, &mut app, view, mode, button);
        }
        // Control: a mode switch that changes the view.
        if view != CompassView::Cube {
            super::support::click(&mut h, &mut app, SLICE_BUTTON);
            app.manifold.compass.view = CompassView::Cube;
            rejects("a mode switch that changes the view", || {
                check_shows(&mut h, &mut app, view, Touch::Slice, "Slice")
            });
        }
    }
}

#[test]
fn mock_compass_views_a_slice_slider_in_plucker_sets_slice_and_keeps_plucker() {
    let (mut app, mut h) = tilted_app();
    click_scope(&mut h, &mut app, crate::explore::COMPASS_PLUCKER);
    assert_eq!(app.manifold.compass.mode, Touch::Tilt);
    let rows = app.console().len();
    // Tab to the Manifold view, Enter into Chart, ↓ ↓ to Centre z₀, Enter onto its first slider, Enter to interact,
    // → to raise it.
    for key in [
        Key::Tab,
        Key::Tab,
        Key::Enter,
        Key::ArrowDown,
        Key::ArrowDown,
        Key::Enter,
        Key::Enter,
        Key::ArrowRight,
    ] {
        press(&mut h, &mut app, key, NONE);
    }
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(setfields(&app, rows), ["z0"], "the slider moved z₀");
    settle(&mut h, &mut app);
    check_shows(
        &mut h,
        &mut app,
        CompassView::Plucker,
        Touch::Slice,
        "a slice slider",
    );
    rejects("a slice slider that switches the view", || {
        check_shows(
            &mut h,
            &mut app,
            CompassView::Cube,
            Touch::Slice,
            "a slice slider",
        )
    });
}

/// Asserts nothing was edited since `rows` and the undo depth `depth`.
fn check_no_edit(app: &App<MockSide>, rows: usize, depth: u32, what: &str) {
    assert!(setfields(app, rows).is_empty(), "{what}: a SetField");
    assert_eq!(app.snapshot().history.undo_depth, depth, "{what}: undo");
}

#[test]
fn mock_compass_views_a_drag_in_a_grid_view_emits_no_setfield() {
    for mode in [Touch::Tilt, Touch::Slice] {
        for view in [
            CompassView::Plucker,
            CompassView::Jacobi,
            CompassView::Shape,
        ] {
            let (mut app, mut h) = tilted_app();
            app.manifold.compass.mode = mode;
            app.manifold.compass.view = view;
            let _ = h.frame(&mut app, Vec::new());
            let (rows, depth) = (app.console().len(), app.snapshot().history.undo_depth);
            let sq = square(&h);
            let from = GridPlace::fit(sq).cell_rect(1, 4).center();
            drag(&mut h, &mut app, from, egui::vec2(60.0, -40.0));
            drag(&mut h, &mut app, sq.center(), egui::vec2(-50.0, 30.0));
            check_no_edit(&app, rows, depth, "a drag in a grid view");
            assert_eq!(app.manifold.compass.view, view);
            assert_eq!(app.manifold.compass.drag, None);
        }
        // Control: the same drag in the Cube view edits.
        let (mut app, mut h) = tilted_app();
        app.manifold.compass.mode = mode;
        let (rows, depth) = (app.console().len(), app.snapshot().history.undo_depth);
        let centre = square(&h).center();
        drag(&mut h, &mut app, centre, egui::vec2(60.0, -40.0));
        rejects("a drag on the Cube", || {
            check_no_edit(&app, rows, depth, "a drag on the Cube")
        });
    }
}

#[test]
fn mock_compass_views_a_view_switch_emits_no_setfield() {
    let (mut app, mut h) = tilted_app();
    let (rows, depth) = (app.console().len(), app.snapshot().history.undo_depth);
    for (_, id, _) in VIEWS.iter().chain(VIEWS.iter().rev()) {
        click_scope(&mut h, &mut app, id);
        let _ = h.frame(&mut app, Vec::new());
    }
    check_no_edit(&app, rows, depth, "the view switches");
    // By the keyboard too: Enter on each view button.
    for key in [Key::Tab; 5] {
        press(&mut h, &mut app, key, NONE);
    }
    press(&mut h, &mut app, Key::Enter, NONE);
    for _ in 0..3 {
        press(&mut h, &mut app, Key::ArrowRight, NONE);
    }
    for _ in 0..4 {
        press(&mut h, &mut app, Key::Enter, NONE);
        press(&mut h, &mut app, Key::ArrowRight, NONE);
    }
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(app.manifold.compass.view, CompassView::Shape);
    check_no_edit(&app, rows, depth, "the view switches by Enter");
    // Control: a slider touched does edit.
    app.set_field(crate::explore::manifold_view::z0_edit([0.3; 8]));
    let _ = h.frame(&mut app, Vec::new());
    rejects("an edit", || check_no_edit(&app, rows, depth, "an edit"));
}

#[test]
fn mock_compass_views_jacobi_and_shape_wait_on_physics_review() {
    for (view, id) in [
        (CompassView::Jacobi, crate::explore::COMPASS_JACOBI),
        (CompassView::Shape, crate::explore::COMPASS_SHAPE),
    ] {
        let (mut app, mut h) = tilted_app();
        click_scope(&mut h, &mut app, id);
        let shapes = compass_shapes(&mut h, &mut app);
        let note = shapes
            .iter()
            .find(|s| matches!(s, Shape::Text(t) if t.galley.text() == WAITS))
            .unwrap_or_else(|| panic!("{view:?}: the note"));
        let Shape::Text(t) = note else { unreachable!() };
        let laid = t.galley.rect.translate(t.pos.to_vec2());
        let sq = square(&h);
        assert!(
            (laid.center() - sq.center()).length() < 1.0,
            "{view:?}: centred in the square"
        );
        assert!(
            laid.width() <= sq.width() + 1e-3,
            "{view:?}: wrapped to the square"
        );
        assert_eq!(app.manifold.compass.view, view);
    }
    assert_eq!(WAITS, "waits on physics review (RQ-341 to RQ-346)");
}

#[test]
fn mock_compass_views_a_grid_view_drops_a_drag_in_progress() {
    let (mut app, mut h) = tilted_app();
    let p = app.snapshot().sim.plane.clone();
    app.manifold.compass.drag = Some(compass::Drag::start(Touch::Tilt, &p));
    app.manifold.compass.view = CompassView::Plucker;
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(app.manifold.compass.drag, None, "the drag dropped");
    app.manifold.compass.drag = Some(compass::Drag::start(Touch::Tilt, &p));
    app.manifold.compass.view = CompassView::Cube;
    let _ = h.frame(&mut app, Vec::new());
    let kept = app.manifold.compass.drag;
    rejects("a Cube view that drops a drag", || assert_eq!(kept, None));
}
