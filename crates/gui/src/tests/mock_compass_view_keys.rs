//! `mock_compass_view_keys` (REQ-GUI-198; render_gui_spec §G3, § "The compass: look and views (R-451, R-452)";
//! R-452 C6): the Compass scope's seven siblings in order both ways; Enter on each view button switches the view in
//! navigation mode; in the Plücker view Enter on the compass starts interaction on the dominant cell, the arrows move a
//! cell cursor over the upper triangle only, Space steers to it (undone by one Ctrl+Z) and Esc or Enter returns; in the
//! Cube view the arrows tilt as before; the breadcrumb names the view; the focus on the compass element brings the
//! column's text into view at the small panels; the ring stays in the panel under the wheel. Each check also runs on an input it must reject
//! (R-176, through `rejects`).

use eframe::egui::{
    pos2, vec2, Event, FullOutput, Key, Modifiers, MouseWheelUnit, Rect, Shape, TouchPhase,
};
use engine::contract::sim_config::Plane;
use engine::contract::view_ui::{Focus, KeyMode, Mode};

use super::compass_support::{app_at, plane, setfields, view};
use super::support::{focus, rejects};
use crate::app::App;
use crate::capture::PIXELS_PER_POINT;
use crate::explore::breadcrumb;
use crate::explore::compass;
use crate::explore::compass::physical;
use crate::explore::compass::views::{
    cursor_moved, cursor_start, dominant, plucker, plucker_drawing_with, upper, CompassView,
    PLUCKER_HELP, VIEWS,
};
use crate::explore::manifold_view::tilt::Angles;
use crate::explore::manifold_view::{z0_edit, View};
use crate::explore::{
    COMPASS, COMPASS_CUBE, COMPASS_JACOBI, COMPASS_PLUCKER, COMPASS_SHAPE, COMPASS_SLICE,
    COMPASS_TILT, COMPASS_VIEW,
};
use crate::headless::Headless;
use crate::keyboard::keymap::{Command, Direction};
use crate::keyboard::scopes::{Scope, ScopeTree, StepKind};
use crate::layout::{Layout, Splitter, MIN_BOTTOM_H, MIN_COMPASS_W};
use crate::side::MockSide;

const NONE: Modifiers = Modifiers::NONE;
const SHIFT: Modifiers = Modifiers::SHIFT;
const ALT: Modifiers = Modifiers::ALT;

/// The seven siblings, in order (R-452 C6).
const SEVEN: [&str; 7] = [
    COMPASS_VIEW,
    COMPASS_TILT,
    COMPASS_SLICE,
    COMPASS_CUBE,
    COMPASS_PLUCKER,
    COMPASS_JACOBI,
    COMPASS_SHAPE,
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
    for d in [Direction::Right; 7]
        .into_iter()
        .chain([Direction::Left; 7])
    {
        tree.navigate(&mut f, Command::Arrow(d), 1.0);
        seen.push(f.path.last().cloned().unwrap_or_default());
    }
    seen
}

fn check_walk(seen: &[String]) {
    let mut want: Vec<&str> = SEVEN.to_vec();
    want.push(COMPASS_SHAPE);
    want.extend(SEVEN.iter().rev().skip(1));
    want.push(COMPASS_VIEW);
    assert_eq!(seen, want, "the seven siblings, stopping at each end");
}

#[test]
fn mock_compass_view_keys_the_seven_siblings_in_order_both_ways() {
    let (mut app, mut h) = on_compass();
    check_walk(&walk(app.keyboard.tree(Mode::Explore)));
    // On the app: → six times reaches Shape, ← six times the compass.
    for id in SEVEN.iter().skip(1) {
        press(&mut h, &mut app, Key::ArrowRight, NONE);
        assert_eq!(at(&app), (vec![COMPASS, *id], KeyMode::Navigation));
    }
    for id in SEVEN.iter().rev().skip(1) {
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

/// The compass's whole text in `app`'s current view: each readout line and the hint.
fn column_text(app: &App<MockSide>, panel: Rect) -> Vec<String> {
    let sim = &app.snapshot().sim;
    let c = &app.manifold.compass;
    match c.view {
        CompassView::Cube => {
            let mut text = compass::current(panel, sim, c).readout;
            text.push(compass::HINT_TILT.to_owned());
            text
        }
        CompassView::Plucker => vec![
            plucker_drawing_with(
                compass::square(panel),
                &sim.plane,
                View::read(sim).d,
                c.cursor,
            )
            .readout,
            PLUCKER_HELP.to_owned(),
        ],
        CompassView::Jacobi | CompassView::Shape => {
            let set = c.view.physical().expect("a physical view");
            vec![
                physical::readout(set, &compass::pushed(sim, c, set)),
                physical::HELP.to_owned(),
            ]
        }
    }
}

/// The first of `wanted` not wholly in the panel's column in `output`, by the rect it is drawn in and its clip.
fn unread(output: &FullOutput, panel: Rect, wanted: &[String]) -> Option<String> {
    wanted
        .iter()
        .find(|text| {
            !output.shapes.iter().any(|c| match &c.shape {
                Shape::Text(t) if t.galley.text() == text.as_str() => {
                    let at = t.galley.rect.translate(t.pos.to_vec2());
                    panel.contains_rect(c.clip_rect) && c.clip_rect.contains_rect(at)
                }
                _ => false,
            })
        })
        .cloned()
}

/// Runs enough frames for the column's scroll to finish, then one whose output is returned.
fn settled_output(h: &mut Headless, app: &mut App<MockSide>) -> FullOutput {
    for _ in 0..60 {
        let _ = h.frame(app, Vec::new());
    }
    h.frame(app, Vec::new())
}

/// The app at `p` in a window of `window` points, at the least panel if `least`, the focus on the compass element by
/// Tab and Enter, in view `index` of [`SEVEN`] by the keys, back on the element: and the panel.
fn at_element(
    p: &Plane,
    window: [f32; 2],
    least: bool,
    index: usize,
) -> (App<MockSide>, Headless, Rect) {
    let (mut app, mut h) = app_at(p);
    h.set_size(window.map(|points| (points * PIXELS_PER_POINT) as u32));
    let screen = h.screen();
    if least {
        let footer = Layout::new(screen, PIXELS_PER_POINT).footer.min.y;
        app.splits
            .set(Splitter::CompassTime, MIN_COMPASS_W / screen.width());
        app.splits.set(
            Splitter::TopBottom,
            (footer - MIN_BOTTOM_H) / screen.height(),
        );
    }
    for key in [Key::Tab; 5] {
        press(&mut h, &mut app, key, NONE);
    }
    press(&mut h, &mut app, Key::Enter, NONE);
    for _ in 0..index {
        press(&mut h, &mut app, Key::ArrowRight, NONE);
    }
    press(&mut h, &mut app, Key::Enter, NONE);
    for _ in 0..index {
        press(&mut h, &mut app, Key::ArrowLeft, NONE);
    }
    assert_eq!(at(&app), (vec![COMPASS, COMPASS_VIEW], KeyMode::Navigation));
    let panel = Layout::with_splits(screen, PIXELS_PER_POINT, &app.splits).compass;
    (app, h, panel)
}

/// The sizes the column overflows at, the windows the gui review measured.
const SMALL: [(&str, [f32; 2], bool); 3] = [
    ("the least panel", [1440.0, 900.0], true),
    ("1024 by 768", [1024.0, 768.0], false),
    ("1280 by 720", [1280.0, 720.0], false),
];

/// The views by their place in [`SEVEN`].
const FOUR: [usize; 4] = [3, 4, 5, 6];

/// The tilted plane the tests start from.
fn tilted() -> Plane {
    plane(&view(30.0, -20.0, 25.0, 1.0), [0.1, -0.2, 0.2])
}

/// The tilted plane at the Shape sphere's `+w` pole, where the readout's stretch is at its widest.
fn at_the_pole() -> Plane {
    let mut p = tilted();
    p.z0 = POLE;
    p
}

/// The point of [`at_the_pole`].
const POLE: engine::contract::sim_config::Latent = [0.0, 0.0, 0.3, -0.2, 0.5, 0.1, 0.0, 0.0];

/// Presses `key` `times` times, then the frames for the column's scroll to finish: the last one's output.
fn after_keys(h: &mut Headless, app: &mut App<MockSide>, key: Key, times: usize) -> FullOutput {
    for _ in 0..times {
        press(h, app, key, NONE);
    }
    settled_output(h, app)
}

/// Wheels the column down by `points` (up if negative), the pointer over it, then the frames for it to finish.
fn wheel(h: &mut Headless, app: &mut App<MockSide>, panel: Rect, points: f32) -> FullOutput {
    let over = pos2(panel.max.x - 30.0, panel.max.y - 30.0);
    let _ = h.frame(app, vec![Event::PointerMoved(over)]);
    let _ = h.frame(
        app,
        vec![Event::MouseWheel {
            unit: MouseWheelUnit::Point,
            delta: vec2(0.0, -points),
            phase: TouchPhase::Move,
            modifiers: NONE,
        }],
    );
    settled_output(h, app)
}

/// The keys alone bring every readout line and the hint's last row into view, at the small panels in all four views:
/// the column's text is read with the focus on the compass element. The control is the wheel-only column, the same
/// text where the buttons leave it, which fails wherever the column overflows.
#[test]
fn mock_compass_view_keys_the_element_brings_the_column_text_into_view() {
    for (name, window, least) in SMALL {
        for index in FOUR {
            let (mut app, mut h, panel) = at_element(&tilted(), window, least, index);
            let output = settled_output(&mut h, &mut app);
            let text = column_text(&app, panel);
            assert_eq!(
                unread(&output, panel, &text),
                None,
                "{name}, {}: in {panel:?}",
                SEVEN[index]
            );
            let output = after_keys(&mut h, &mut app, Key::ArrowRight, 1);
            // At 1280 by 720 the Cube's and the Plücker view's column fits, so nothing needs the keys.
            let fits = window == [1280.0, 720.0] && index < 5;
            if fits {
                assert_eq!(unread(&output, panel, &text), None, "{name}, {index}");
            } else {
                rejects("the wheel-only column", || {
                    assert_eq!(unread(&output, panel, &text), None, "{name}, {index}")
                });
            }
        }
    }
}

/// The rect `wanted` is drawn in and its clip, in `output`, with its rows' height.
fn drawn(output: &FullOutput, wanted: &str) -> (Rect, Rect, f32) {
    output
        .shapes
        .iter()
        .find_map(|c| match &c.shape {
            Shape::Text(t) if t.galley.text() == wanted => {
                let at = t.galley.rect.translate(t.pos.to_vec2());
                Some((at, c.clip_rect, at.height() / t.galley.rows.len() as f32))
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("{wanted} drawn"))
}

/// The Shape view at its pole, the widest readout. At 1280 by 720 the element shows the whole column. At the least panel
/// and 1024 by 768 the column's text is taller than the view, so one stop cannot show it all: the element shows the hint
/// and the readout from its second row, and Tilt, a key from it, the readout's first row, at most a point short.
#[test]
fn mock_compass_view_keys_at_the_pole_the_keys_read_it_in_two_stops() {
    let shape = 6;
    let (name, window, least) = SMALL[2];
    let (mut app, mut h, panel) = at_element(&at_the_pole(), window, least, shape);
    let output = settled_output(&mut h, &mut app);
    let text = column_text(&app, panel);
    assert_eq!(unread(&output, panel, &text), None, "{name}");
    for (name, window, least) in &SMALL[..2] {
        let (mut app, mut h, panel) = at_element(&at_the_pole(), *window, *least, shape);
        let output = settled_output(&mut h, &mut app);
        let text = column_text(&app, panel);
        assert_eq!(unread(&output, panel, &text[1..]), None, "{name}: the hint");
        let (readout, clip, row) = drawn(&output, &text[0]);
        assert!(readout.max.y <= clip.max.y, "{name}: {readout:?} {clip:?}");
        assert!(
            readout.min.y + row >= clip.min.y,
            "{name}: from its second row: {readout:?} {clip:?}"
        );
        // Control: the whole column in one stop is what this view has no room for.
        rejects("the whole readout, in the view with the whole hint", || {
            assert_eq!(unread(&output, panel, &text[..1]), None, "{name}")
        });
        let output = after_keys(&mut h, &mut app, Key::ArrowRight, 1);
        assert_eq!(at(&app), (vec![COMPASS, COMPASS_TILT], KeyMode::Navigation));
        let (readout, clip, row) = drawn(&output, &text[0]);
        assert!(
            readout.min.y + row <= clip.max.y + 1.0,
            "{name}: its first row: {readout:?} {clip:?}"
        );
    }
}

/// The readout growing a row while the focus is on the element leaves the column's end in view; with the focus on a
/// button it does not move the column there.
#[test]
fn mock_compass_view_keys_the_element_keeps_the_end_as_the_readout_grows() {
    let shape = 6;
    let (name, window, least) = SMALL[0];
    let (mut app, mut h, panel) = at_element(&tilted(), window, least, shape);
    let _ = settled_output(&mut h, &mut app);
    app.set_field(z0_edit(POLE));
    let output = settled_output(&mut h, &mut app);
    let hint = [physical::HELP.to_owned()];
    assert!(
        column_text(&app, panel)[0].contains("169"),
        "the readout grew: {:?}",
        column_text(&app, panel)
    );
    assert_eq!(unread(&output, panel, &hint), None, "{name}");
    // The same edit with the focus on a button leaves the hint where it was, off the view.
    let (mut app, mut h, panel) = at_element(&tilted(), window, least, shape);
    let _ = after_keys(&mut h, &mut app, Key::ArrowRight, 1);
    app.set_field(z0_edit(POLE));
    let output = settled_output(&mut h, &mut app);
    rejects("the hint's end with the focus on a button", || {
        assert_eq!(unread(&output, panel, &hint), None, "{name}")
    });
}

/// With the focus on the element the wheel is still the user's: it takes the column back up, and stays there.
#[test]
fn mock_compass_view_keys_at_the_element_the_wheel_still_scrolls() {
    let (name, window, least) = SMALL[0];
    let (mut app, mut h, panel) = at_element(&tilted(), window, least, 3);
    let output = settled_output(&mut h, &mut app);
    let title = [compass::TITLE.to_owned()];
    rejects("the title at the column's end", || {
        assert_eq!(unread(&output, panel, &title), None, "{name}")
    });
    let output = wheel(&mut h, &mut app, panel, -2000.0);
    assert_eq!(unread(&output, panel, &title), None, "{name}");
    assert_eq!(at(&app), (vec![COMPASS, COMPASS_VIEW], KeyMode::Navigation));
}

/// The focus ring's rects in `output`.
fn rings_in(output: &FullOutput) -> Vec<Rect> {
    output
        .shapes
        .iter()
        .filter_map(|c| match &c.shape {
            Shape::Rect(r) if r.stroke.width == breadcrumb::RING_WIDTH => Some(r.rect),
            _ => None,
        })
        .collect()
}

/// The ring on a column's button stays in the panel when the wheel moves the button: on the part still shown, partly
/// scrolled; none at all, wholly scrolled off, where the ring on the button's whole rect would cross the panel's top.
#[test]
fn mock_compass_view_keys_the_ring_stays_in_the_panel_under_the_wheel() {
    let (name, window, least) = SMALL[0];
    let (mut app, mut h, panel) = at_element(&tilted(), window, least, 3);
    let output = after_keys(&mut h, &mut app, Key::ArrowRight, 1);
    assert_eq!(at(&app), (vec![COMPASS, COMPASS_TILT], KeyMode::Navigation));
    let at_rest = rings_in(&output);
    assert_eq!(at_rest.len(), 1, "{name}: one ring: {at_rest:?}");
    let whole = at_rest[0];
    assert!(panel.contains_rect(whole), "{whole:?} in {panel:?}");
    // A little: the button partly under the column's top edge, the ring on what shows.
    let output = wheel(
        &mut h,
        &mut app,
        panel,
        whole.max.y - panel.min.y - 12.0 - 4.0,
    );
    let part = rings_in(&output);
    assert_eq!(part.len(), 1, "{name}: the part shown: {part:?}");
    assert!(panel.contains_rect(part[0]), "{part:?} in {panel:?}");
    assert!(part[0].height() < whole.height(), "{part:?} of {whole:?}");
    // All the way: the button is above the panel's top, no ring.
    let output = wheel(&mut h, &mut app, panel, 2000.0);
    assert_eq!(
        rings_in(&output),
        Vec::<Rect>::new(),
        "{name}: nothing shown"
    );
    let place = app.keyboard.place_of(COMPASS_TILT).expect("placed");
    assert!(!place.shown().is_positive(), "{place:?}: nothing shown");
    // Control: the ring on the button's whole rect, as it was, crosses the panel's top.
    rejects("a ring on the whole rect of a button scrolled off", || {
        assert!(panel.contains_rect(place.rect.expand(3.0)), "{place:?}")
    });
}

/// A widget's place shows the part of it its `Ui` shows; wholly clipped, nothing.
#[test]
fn mock_compass_view_keys_a_widgets_place_shows_what_its_ui_shows() {
    let ctx = eframe::egui::Context::default();
    let clip = Rect::from_min_size(pos2(0.0, 100.0), vec2(100.0, 100.0));
    let mut keyboard = crate::keyboard::Keyboard::new();
    let mut rects = Vec::new();
    let mut output = ctx.run_ui(Default::default(), |ui| {
        ui.set_clip_rect(clip);
        for (id, y) in [("inside", 120.0), ("across", 90.0), ("above", 50.0)] {
            let rect = Rect::from_min_size(pos2(10.0, y), vec2(20.0, 20.0));
            let response = ui.interact(rect, ui.id().with(id), eframe::egui::Sense::click());
            keyboard.place_widget(id, &response);
            rects.push((rect, keyboard.place_of(id).expect("placed").shown()));
        }
    });
    output.textures_delta.clear();
    let [(inside, placed), (across, part), (_, none)] = rects[..] else {
        panic!("{rects:?}")
    };
    assert_eq!(placed, inside);
    assert_eq!(part, across.intersect(clip));
    assert!(!none.is_positive(), "{none:?}");
    rejects("the widget's whole rect", || {
        assert!(clip.contains_rect(across))
    });
}
