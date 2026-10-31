//! `mock_compass_view_keys` (REQ-GUI-198; render_gui_spec §G3, § "The compass: look and views (R-451, R-452)";
//! R-452 C6): the Compass scope's siblings, the seven and R-459 G9's H, V and D, in order both ways; Enter on each view button switches the view in
//! navigation mode; in the Plücker view Enter on the compass starts interaction on the dominant cell, the arrows move a
//! cell cursor over the upper triangle only, Space steers to it (undone by one Ctrl+Z) and Esc or Enter returns; in the
//! Cube view the arrows tilt as before; the breadcrumb names the view. Each check also runs on an input it must reject
//! (R-176, through `rejects`).

use eframe::egui::{Key, Modifiers};
use engine::contract::view_ui::{Focus, KeyMode, Mode};

use super::compass_support::{app_at, plane, setfields, view};
use super::support::{focus, rejects};
use crate::app::App;
use crate::explore::breadcrumb;
use crate::explore::compass;
use crate::explore::compass::views::{
    cursor_moved, cursor_start, dominant, plucker, upper, CompassView, VIEWS,
};
use crate::explore::manifold_view::tilt::Angles;
use crate::explore::manifold_view::View;
use crate::explore::{
    COMPASS, COMPASS_CUBE, COMPASS_D, COMPASS_H, COMPASS_JACOBI, COMPASS_PLUCKER, COMPASS_SHAPE,
    COMPASS_SLICE, COMPASS_TILT, COMPASS_V, COMPASS_VIEW,
};
use crate::headless::Headless;
use crate::keyboard::keymap::{Command, Direction};
use crate::keyboard::scopes::{Scope, ScopeTree, StepKind};
use crate::side::MockSide;

const NONE: Modifiers = Modifiers::NONE;
const SHIFT: Modifiers = Modifiers::SHIFT;
const ALT: Modifiers = Modifiers::ALT;

/// The siblings in the Cube view, in order (R-452 C6; R-459 G9).
const SIBLINGS: [&str; 10] = [
    COMPASS_VIEW,
    COMPASS_TILT,
    COMPASS_SLICE,
    COMPASS_CUBE,
    COMPASS_PLUCKER,
    COMPASS_JACOBI,
    COMPASS_SHAPE,
    COMPASS_H,
    COMPASS_V,
    COMPASS_D,
];

/// The app on a tilted plane at rest, the focus moved by Tab onto the compass, in navigation mode.
fn on_compass() -> (App<MockSide>, Headless) {
    let (mut app, mut h) = app_at(&plane(&view(30.0, -20.0, 25.0, 1.0), [0.1, -0.2, 0.2]));
    for key in [Key::Tab; 5] {
        press(&mut h, &mut app, key, NONE);
    }
    press(&mut h, &mut app, Key::Enter, NONE);
    assert_eq!(at(&app), (vec![COMPASS, COMPASS_VIEW], KeyMode::Navigation));
    (app, h)
}

fn press(h: &mut Headless, app: &mut App<MockSide>, key: Key, modifiers: Modifiers) {
    super::support::press(h, app, key, modifiers);
}

fn at(app: &App<MockSide>) -> (Vec<&str>, KeyMode) {
    (focus(app), app.view.focus.mode)
}

/// Walks the Compass scope of `tree` from the compass: → past the last and ← past the first; the id at each.
fn walk(tree: &ScopeTree) -> Vec<String> {
    let mut f = Focus {
        path: vec![COMPASS.to_owned()],
        mode: KeyMode::Navigation,
    };
    tree.navigate(&mut f, Command::Enter, 1.0);
    let mut seen = vec![f.path.last().cloned().unwrap_or_default()];
    for d in [Direction::Right; 10]
        .into_iter()
        .chain([Direction::Left; 10])
    {
        tree.navigate(&mut f, Command::Arrow(d), 1.0);
        seen.push(f.path.last().cloned().unwrap_or_default());
    }
    seen
}

fn check_walk(seen: &[String]) {
    let mut want: Vec<&str> = SIBLINGS.to_vec();
    want.push(COMPASS_D);
    want.extend(SIBLINGS.iter().rev().skip(1));
    want.push(COMPASS_VIEW);
    assert_eq!(seen, want, "the siblings, stopping at each end");
}

#[test]
fn mock_compass_view_keys_the_siblings_in_order_both_ways() {
    let (mut app, mut h) = on_compass();
    check_walk(&walk(app.keyboard.tree(Mode::Explore)));
    // On the app: → nine times reaches D, ← nine times the compass.
    for id in SIBLINGS.iter().skip(1) {
        press(&mut h, &mut app, Key::ArrowRight, NONE);
        assert_eq!(at(&app), (vec![COMPASS, *id], KeyMode::Navigation));
    }
    for id in SIBLINGS.iter().rev().skip(1) {
        press(&mut h, &mut app, Key::ArrowLeft, NONE);
        assert_eq!(at(&app), (vec![COMPASS, *id], KeyMode::Navigation));
    }
    let mut wrong = ScopeTree::new();
    wrong.register(None, Scope::group(COMPASS, "Compass"));
    wrong.register(
        Some(COMPASS),
        Scope::value(COMPASS_VIEW, "compass", StepKind::Tilt),
    );
    for id in [
        COMPASS_TILT,
        COMPASS_SLICE,
        COMPASS_PLUCKER,
        COMPASS_CUBE,
        COMPASS_JACOBI,
        COMPASS_SHAPE,
        COMPASS_H,
        COMPASS_V,
        COMPASS_D,
    ] {
        wrong.register(Some(COMPASS), Scope::control(id, "x"));
    }
    rejects("another sibling order", || check_walk(&walk(&wrong)));
}

#[test]
fn mock_compass_view_keys_enter_on_each_view_button_switches_the_view() {
    let (mut app, mut h) = on_compass();
    for _ in 0..2 {
        press(&mut h, &mut app, Key::ArrowRight, NONE);
    }
    let mode = app.manifold.compass.mode;
    for (view, id, label) in VIEWS {
        press(&mut h, &mut app, Key::ArrowRight, NONE);
        press(&mut h, &mut app, Key::Enter, NONE);
        assert_eq!(app.manifold.compass.view, view, "Enter on {label}");
        assert_eq!(
            at(&app),
            (vec![COMPASS, id], KeyMode::Navigation),
            "{label}: navigating"
        );
        assert_eq!(app.manifold.compass.mode, mode, "{label}: the mode kept");
    }
    rejects("Enter on a view button that leaves the view", || {
        assert_eq!(app.manifold.compass.view, CompassView::Cube)
    });
}

/// The app on the compass in the Plücker view, switched by Enter on its button and back, in interaction mode.
fn interacting_in_plucker() -> (App<MockSide>, Headless) {
    let (mut app, mut h) = on_compass();
    for _ in 0..4 {
        press(&mut h, &mut app, Key::ArrowRight, NONE);
    }
    press(&mut h, &mut app, Key::Enter, NONE);
    assert_eq!(app.manifold.compass.view, CompassView::Plucker);
    for _ in 0..4 {
        press(&mut h, &mut app, Key::ArrowLeft, NONE);
    }
    assert_eq!(
        app.manifold.compass.cursor, None,
        "no cursor while navigating"
    );
    press(&mut h, &mut app, Key::Enter, NONE);
    assert_eq!(
        at(&app),
        (vec![COMPASS, COMPASS_VIEW], KeyMode::Interaction)
    );
    (app, h)
}

/// Asserts `cell` is an upper-triangle cell.
fn check_upper(cell: (usize, usize)) {
    assert!(
        upper().any(|c| c == cell),
        "{cell:?} off the upper triangle"
    );
}

#[test]
fn mock_compass_view_keys_the_cell_cursor_starts_on_the_dominant_cell_and_stays_upper() {
    let (mut app, mut h) = interacting_in_plucker();
    let grid = plucker(&app.snapshot().sim.plane);
    assert_eq!(app.manifold.compass.cursor, Some(cursor_start(&grid)));
    assert_eq!(
        app.manifold.compass.cursor,
        dominant(&grid),
        "on the dominant cell"
    );
    // A walk of every arrow, with and without Shift and Alt, stays upper, each step as `cursor_moved` has it.
    let keys = [
        (Key::ArrowRight, NONE, Direction::Right),
        (Key::ArrowUp, NONE, Direction::Up),
        (Key::ArrowDown, SHIFT, Direction::Down),
        (Key::ArrowLeft, ALT, Direction::Left),
        (Key::ArrowLeft, SHIFT, Direction::Left),
        (Key::ArrowDown, NONE, Direction::Down),
        (Key::ArrowRight, SHIFT, Direction::Right),
        (Key::ArrowUp, SHIFT, Direction::Up),
    ];
    let depth = app.snapshot().history.undo_depth;
    for round in 0..3 {
        for (key, modifiers, direction) in keys {
            let was = app.manifold.compass.cursor.expect("a cursor");
            press(&mut h, &mut app, key, modifiers);
            let now = app.manifold.compass.cursor.expect("a cursor");
            check_upper(now);
            let times = crate::keyboard::keymap::multiplier(modifiers) * direction.sign();
            assert_eq!(
                now,
                cursor_moved(was, direction, times),
                "round {round}: {key:?}"
            );
        }
    }
    assert_eq!(
        app.snapshot().history.undo_depth,
        depth,
        "the cursor edits nothing"
    );
    rejects("a cursor on the diagonal", || check_upper((3, 3)));
    rejects("a cursor in the lower triangle", || check_upper((5, 2)));
}

#[test]
fn mock_compass_view_keys_the_cursor_stops_at_the_triangles_edges() {
    use Direction::{Down, Left, Right, Up};
    // One cell at a time, and the ends.
    assert_eq!(cursor_moved((2, 5), Right, 1.0), (2, 6));
    assert_eq!(cursor_moved((2, 5), Left, -1.0), (2, 4));
    assert_eq!(cursor_moved((2, 5), Up, 1.0), (1, 5));
    assert_eq!(cursor_moved((2, 5), Down, -1.0), (3, 5));
    // At the edges: the last column, the cell after the diagonal, the first row, the cell above the diagonal.
    assert_eq!(cursor_moved((2, 7), Right, 1.0), (2, 7));
    assert_eq!(cursor_moved((2, 3), Left, -1.0), (2, 3));
    assert_eq!(cursor_moved((0, 4), Up, 1.0), (0, 4));
    assert_eq!(cursor_moved((3, 4), Down, -1.0), (3, 4));
    // Shift, ×10, to the row's or column's end; Shift and Alt together, ×1, and Alt, a cell.
    assert_eq!(cursor_moved((2, 4), Right, 10.0), (2, 7));
    assert_eq!(cursor_moved((2, 6), Left, -10.0), (2, 3));
    assert_eq!(cursor_moved((5, 6), Up, 10.0), (0, 6));
    assert_eq!(cursor_moved((1, 6), Down, -10.0), (5, 6));
    assert_eq!(cursor_moved((1, 4), Right, 1.0 + 1e-9), (1, 7));
    assert_eq!(cursor_moved((1, 4), Right, 0.1), (1, 5));
    assert_eq!(cursor_moved((1, 4), Left, -0.1), (1, 3));
    // The first upper cell, where none is dominant.
    assert_eq!(cursor_start(&[[0.0; 8]; 8]), (0, 1));
    // Else the dominant cell, wherever it is.
    let mut grid = [[0.0; 8]; 8];
    grid[3][6] = -0.8;
    grid[1][2] = 0.5;
    let check_start = |want: (usize, usize)| assert_eq!(cursor_start(&grid), want);
    check_start((3, 6));
    rejects(
        "a cursor started on the first cell, another dominant",
        || check_start((0, 1)),
    );
    rejects("a cursor that passes the diagonal", || {
        check_upper(cursor_moved((3, 4), Left, -1.0));
        check_upper((3, 3));
    });
}

#[test]
fn mock_compass_view_keys_space_steers_to_the_cursor_and_esc_returns() {
    let (mut app, mut h) = interacting_in_plucker();
    press(&mut h, &mut app, Key::ArrowUp, SHIFT);
    press(&mut h, &mut app, Key::ArrowRight, SHIFT);
    let cell = app.manifold.compass.cursor.expect("a cursor");
    assert_eq!(cell.1, 7, "Shift+→ to the row's end");
    assert_eq!(cell.0, 0, "Shift+↑ to the column's top");
    let before = app.snapshot().sim.plane.clone();
    let (rows, depth) = (app.console().len(), app.snapshot().history.undo_depth);
    press(&mut h, &mut app, Key::Space, NONE);
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(setfields(&app, rows), ["basis"], "one SetField");
    assert_eq!(app.snapshot().history.undo_depth, depth + 1);
    let v = View::of(&app.snapshot().sim.plane);
    assert_eq!((v.a, v.b), cell, "steered to the cursor's plane");
    let a = Angles::of(&v);
    assert_eq!((a.tau1, a.tau2, a.gamma), (0.0, 0.0, 0.0));
    assert_eq!(
        at(&app),
        (vec![COMPASS, COMPASS_VIEW], KeyMode::Interaction)
    );
    press(&mut h, &mut app, Key::Z, Modifiers::CTRL);
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(app.snapshot().sim.plane, before, "undone by one Ctrl+Z");
    assert_eq!(app.snapshot().history.undo_depth, depth);
    // Esc returns to navigation mode on the compass, the cursor gone; there Space steers nothing.
    press(&mut h, &mut app, Key::Escape, NONE);
    assert_eq!(at(&app), (vec![COMPASS, COMPASS_VIEW], KeyMode::Navigation));
    assert_eq!(app.manifold.compass.cursor, None);
    let rows = app.console().len();
    press(&mut h, &mut app, Key::Space, NONE);
    let _ = h.frame(&mut app, Vec::new());
    let steered_in_navigation = setfields(&app, rows);
    assert!(steered_in_navigation.is_empty(), "Space in navigation mode");
    // Enter returns too, on a fresh cursor at the dominant cell.
    press(&mut h, &mut app, Key::Enter, NONE);
    let grid = plucker(&app.snapshot().sim.plane);
    assert_eq!(app.manifold.compass.cursor, dominant(&grid));
    press(&mut h, &mut app, Key::Enter, NONE);
    assert_eq!(at(&app), (vec![COMPASS, COMPASS_VIEW], KeyMode::Navigation));
    // Control: the navigation-mode check on a Space that steered.
    let mut scratch = crate::explore::manifold_view::Scratch::default();
    scratch.compass.view = CompassView::Plucker;
    scratch.compass.cursor = Some((0, 7));
    let navigating = Focus {
        path: vec![COMPASS.to_owned(), COMPASS_VIEW.to_owned()],
        mode: KeyMode::Navigation,
    };
    let sim = app.snapshot().sim.clone();
    let mut out = crate::explore::manifold_view::Out::default();
    compass::space(&navigating, &sim, &mut scratch, &mut out);
    assert!(out.edits.is_empty(), "space() in navigation mode");
    let interacting = Focus {
        mode: KeyMode::Interaction,
        ..navigating
    };
    compass::space(&interacting, &sim, &mut scratch, &mut out);
    let steered = out.edits.len();
    rejects("a Space that steers in navigation mode", || {
        assert_eq!(steered, 0)
    });
}

#[test]
fn mock_compass_view_keys_in_the_cube_view_the_arrows_tilt_as_before() {
    let (mut app, mut h) = on_compass();
    press(&mut h, &mut app, Key::Enter, NONE);
    assert_eq!(
        at(&app),
        (vec![COMPASS, COMPASS_VIEW], KeyMode::Interaction)
    );
    assert_eq!(
        app.manifold.compass.cursor, None,
        "no cell cursor on the Cube"
    );
    let before = View::of(&app.snapshot().sim.plane);
    let rows = app.console().len();
    press(&mut h, &mut app, Key::ArrowRight, NONE);
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(setfields(&app, rows), ["basis"]);
    let after = View::of(&app.snapshot().sim.plane);
    let d1 = Angles::of(&after).tau1 - Angles::of(&before).tau1;
    assert!((d1 - 1.0).abs() < 1e-9, "→ tilts τ₁ by 1°: {d1}");
    // In the Jacobi and Shape views, which have no grid yet, the arrows do nothing.
    for view in [CompassView::Jacobi, CompassView::Shape] {
        app.manifold.compass.view = view;
        let rows = app.console().len();
        press(&mut h, &mut app, Key::ArrowRight, NONE);
        press(&mut h, &mut app, Key::Space, NONE);
        let _ = h.frame(&mut app, Vec::new());
        assert!(setfields(&app, rows).is_empty(), "{view:?}");
        assert_eq!(app.manifold.compass.cursor, None);
    }
    rejects("a Cube-view arrow that edits nothing", || {
        assert!(setfields(&app, 0).is_empty())
    });
}

#[test]
fn mock_compass_view_keys_the_breadcrumb_names_the_view() {
    let (mut app, mut h) = on_compass();
    let crumb =
        |app: &App<MockSide>| breadcrumb::text(app.keyboard.tree(Mode::Explore), &app.view.focus);
    assert_eq!(crumb(&app), "Compass › compass");
    for (view, id, label) in VIEWS {
        app.manifold.compass.view = view;
        let _ = h.frame(&mut app, Vec::new());
        let want = if view == CompassView::Cube {
            "Compass › compass".to_owned()
        } else {
            format!("Compass › {label}")
        };
        assert_eq!(crumb(&app), want, "{id}");
    }
    app.manifold.compass.view = CompassView::Plucker;
    let _ = h.frame(&mut app, Vec::new());
    let shown = crumb(&app);
    rejects("a breadcrumb that does not name the view", || {
        assert_eq!(shown, "Compass › compass")
    });
}
