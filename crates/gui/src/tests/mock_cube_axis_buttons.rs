//! `mock_cube_axis_buttons` (REQ-GUI-208; render_gui_spec § "The compass: look and views (R-451, R-452)", §G3; R-459 G1 to
//! G4, G9): the cube's H, V and D buttons sit on R-451 B1's edges in both modes, each a dropdown of the dimensions not on
//! the other two; a pick is one undoable entry, H and V in step with Chart's rows, D the slice frame's normal, a target
//! on it following and the plane re-tilting; a plane that takes D moves it by the slice-axis rule; the keys reach them.
//! Each check also runs on an input it must reject (R-176, through `rejects`).

use std::f64::consts::PI;

use eframe::egui::{vec2, Key, Modifiers, Rect};
use engine::contract::set_field::SetField;
use engine::contract::sim_config::{Latent, Lock, Plane, SliceFrame};
use engine::contract::view_ui::{KeyMode, Mode};

use super::compass_support::{app_at, check_painted, compass_shapes, drawing, plane, square, view};
use super::support::{click_scope, focus, frame_texts, press, rejects};
use crate::app::App;
use crate::explore::compass::views::{CompassView, GridPlace};
use crate::explore::compass::{
    self, choices, edge_label, pick_edit, Drawing, Edge, Iso, Shows, DEPTH_EDGE, HEIGHT_EDGE,
    WIDTH_EDGE,
};
use crate::explore::manifold_view::chart::basis_text;
use crate::explore::manifold_view::{
    adjust, axis, basis_edit, frame_edit, lock_edit, normal_edit, plane_edit, slice_axis, Out,
    Scratch, Touch, View, GAMMA, PRESETS, TAU1, TAU2,
};
use crate::explore::{
    COMPASS, COMPASS_BADGE, COMPASS_CUBE, COMPASS_D, COMPASS_D_ENTRIES, COMPASS_H,
    COMPASS_H_ENTRIES, COMPASS_JACOBI, COMPASS_PLUCKER, COMPASS_SHAPE, COMPASS_SLICE, COMPASS_TILT,
    COMPASS_V, COMPASS_VIEW, COMPASS_V_ENTRIES,
};
use crate::headless::Headless;
use crate::keyboard::keymap::Direction;
use crate::keyboard::scopes::{Adjust, StepKind};
use crate::side::MockSide;

const NONE: Modifiers = Modifiers::NONE;
const Z_ALPHA: usize = 0;
const Z_BETA: usize = 1;
const Z_Q0: usize = 2;
const Z_Q1: usize = 3;
const Z_Q2: usize = 4;
const Z_Q3: usize = 5;
const Z_MU1: usize = 6;

/// The slice frame `(normal, X, Y)`.
fn frame(normal: usize, x: usize, y: usize) -> SliceFrame {
    let index = |i: usize| u32::try_from(i).expect("a latent index");
    SliceFrame {
        normal: index(normal),
        tilt_x: index(x),
        tilt_y: index(y),
    }
}

/// The app at rest on the mock's preset at `(τ₁, τ₂, γ)` degrees, scale `σ`, a frame of the normal and targets given.
fn app_with(
    (tau1, tau2, gamma): (f64, f64, f64),
    sigma: f64,
    f: SliceFrame,
) -> (App<MockSide>, Headless) {
    let v = View {
        d: f.normal as usize,
        x: f.tilt_x as usize,
        y: f.tilt_y as usize,
        ..view(tau1, tau2, gamma, sigma)
    };
    let (mut app, mut h) = app_at(&plane(&v, [0.1, -0.2, 0.2]));
    app.set_field(frame_edit(f, None));
    settle(&mut h, &mut app);
    (app, h)
}

/// Frames for the snapshot and the compass to follow an edit.
fn settle(h: &mut Headless, app: &mut App<MockSide>) {
    let _ = h.frame(app, Vec::new());
    super::compass_support::settle(h, app);
}

/// Ctrl+Z, the contract's undo, then frames to follow it.
fn ctrl_z(h: &mut Headless, app: &mut App<MockSide>) {
    press(h, app, Key::Z, Modifiers::CTRL);
    settle(h, app);
}

/// The state a pick edits, and the undo depth.
fn state(app: &App<MockSide>) -> (u32, Plane, SliceFrame) {
    let s = app.snapshot();
    (s.history.undo_depth, s.sim.plane.clone(), s.sim.slice.frame)
}

/// Asserts the pick from `before` to `after` was one history entry, and `undone` the state before it.
fn check_one_entry(
    before: &(u32, Plane, SliceFrame),
    after: &(u32, Plane, SliceFrame),
    undone: &(u32, Plane, SliceFrame),
) {
    assert_eq!(after.0, before.0 + 1, "one history entry");
    assert_eq!(undone, before, "one Ctrl+Z undoes all of it");
}

/// The buttons' texts, H, third, V.
fn texts(h: &Headless, app: &App<MockSide>) -> [String; 3] {
    let d = drawing(h, app);
    assert_eq!(d.labels.len(), 3, "three labels: {:?}", d.labels);
    [
        d.labels[0].text.clone(),
        d.labels[1].text.clone(),
        d.labels[2].text.clone(),
    ]
}

/// Asserts the buttons read `want`, each centred on its edge's place, the scope placed there.
fn check_on_edges(h: &Headless, app: &App<MockSide>, want: [&str; 3]) {
    let iso = Iso::fit(square(h));
    assert_eq!(texts(h, app), want.map(str::to_owned));
    for (id, edge, text) in [
        (COMPASS_H, WIDTH_EDGE, want[0]),
        (COMPASS_D, DEPTH_EDGE, want[1]),
        (COMPASS_V, HEIGHT_EDGE, want[2]),
    ] {
        let at = edge_label(&iso, edge, text.to_owned(), false).at;
        let placed = app.keyboard.place_of(id).expect("placed").rect;
        assert!(
            (placed.center() - at).length() < 1e-3,
            "{id} at {:?}, its edge's label at {at:?}",
            placed.center()
        );
        assert!(
            placed.width() > 10.0 && placed.height() > 10.0,
            "{id} big enough to press: {placed:?}"
        );
    }
}

/// Asserts the button `id`, placed at `placed`, is its painted text's width `text_width` and the label size, each with
/// the padding round it.
fn check_padded(id: &str, placed: Rect, text_width: f32) {
    let pad = 2.0 * compass::AXIS_BUTTON_PAD;
    assert!(
        (placed.width() - (text_width + pad)).abs() < 1.0,
        "{id}'s width: {placed:?}"
    );
    assert!(
        (placed.height() - (compass::LABEL_SIZE + pad)).abs() < 0.5,
        "{id}'s height: {placed:?}"
    );
}

/// Asserts the drawing has corner numbers exactly where `shown` says.
fn check_numbers(d: &Drawing, shown: bool) {
    assert_eq!(!d.numbers.is_empty(), shown, "the numbers: {:?}", d.numbers);
}

#[test]
fn mock_cube_axis_buttons_sit_on_their_edges_in_both_modes() {
    let (mut app, mut h) = app_at(&plane(&view(0.0, 0.0, 0.0, 1.0), [0.0, 0.0, 0.0]));
    let want = ["H z_α", "D z_q3", "V z_β"];
    check_on_edges(&h, &app, want);
    let shapes = compass_shapes(&mut h, &mut app);
    check_painted(&shapes, &drawing(&h, &app));
    let tilt = drawing(&h, &app);
    check_numbers(&tilt, false);
    // Each button is its text and the padding round it, the text painted over it.
    let painted = super::compass_support::texts(&shapes);
    let mut sized = Vec::new();
    for (id, text) in [
        (COMPASS_H, want[0]),
        (COMPASS_D, want[1]),
        (COMPASS_V, want[2]),
    ] {
        let drawn = painted
            .iter()
            .find(|t| t.0 == text)
            .unwrap_or_else(|| panic!("{text} painted"));
        let placed = app.keyboard.place_of(id).expect("placed").rect;
        check_padded(id, placed, drawn.1.width());
        sized.push((id, placed, drawn.1.width()));
    }
    app.manifold.compass.mode = Touch::Slice;
    settle(&mut h, &mut app);
    check_on_edges(&h, &app, want);
    let slice = drawing(&h, &app);
    check_numbers(&slice, true);
    let shapes = compass_shapes(&mut h, &mut app);
    check_painted(&shapes, &drawing(&h, &app));
    rejects("H and V on each other's edges", || {
        check_on_edges(&h, &app, ["V z_β", "D z_q3", "H z_α"])
    });
    rejects("a button without its letter", || {
        check_on_edges(&h, &app, ["z_α", "D z_q3", "V z_β"])
    });
    for (id, placed, text_width) in sized {
        rejects("a button of its text alone, no padding", || {
            check_padded(
                id,
                Rect::from_center_size(placed.center(), vec2(text_width, compass::LABEL_SIZE)),
                text_width,
            )
        });
    }
    rejects("tilt's drawing, with no numbers, in slice mode", || {
        check_numbers(&tilt, true)
    });
    rejects("slice's drawing, with numbers, in tilt mode", || {
        check_numbers(&slice, false)
    });
}

/// Asserts each list holds the dimensions not on the other two, in order.
fn check_lists(v: &View, [h_list, v_list, d_list]: [&[usize]; 3]) {
    let all = |skip: [usize; 2]| -> Vec<usize> { (0..8).filter(|i| !skip.contains(i)).collect() };
    assert_eq!(h_list, all([v.b, v.d]), "H's list");
    assert_eq!(v_list, all([v.a, v.d]), "V's list");
    assert_eq!(d_list, all([v.a, v.b]), "D's list");
}

#[test]
fn mock_cube_axis_buttons_each_dropdown_lists_the_dimensions_not_on_the_other_two() {
    let (app, _h) = app_with((0.0, 0.0, 0.0), 1.0, frame(Z_Q3, Z_Q3, Z_Q3));
    let read = View::read(&app.snapshot().sim);
    let (h_list, v_list, d_list) = (
        choices(&read, Edge::H),
        choices(&read, Edge::V),
        choices(&read, Edge::D),
    );
    check_lists(&read, [&h_list, &v_list, &d_list]);
    // The keyboard's menus hold the same entries.
    let tree = app.keyboard.tree(Mode::Explore);
    for (id, entries, list) in [
        (COMPASS_H, COMPASS_H_ENTRIES, &h_list),
        (COMPASS_V, COMPASS_V_ENTRIES, &v_list),
        (COMPASS_D, COMPASS_D_ENTRIES, &d_list),
    ] {
        let got = tree.children(Some(id));
        check_entries(id, &got, entries, list);
        rejects("a menu missing its last entry", || {
            check_entries(id, &got[..got.len() - 1], entries, list)
        });
    }
    // A dimension not offered is no pick; nor is the one it already is.
    let sim = &app.snapshot().sim;
    for (edge, to) in [
        (Edge::H, Z_BETA),
        (Edge::H, Z_Q3),
        (Edge::H, Z_ALPHA),
        (Edge::V, Z_ALPHA),
        (Edge::V, Z_Q3),
        (Edge::V, Z_BETA),
        (Edge::D, Z_BETA),
        (Edge::D, Z_Q3),
    ] {
        check_no_pick(pick_edit(sim, edge, to, Shows::Normal), edge, to);
    }
    let mut with_v = h_list.clone();
    with_v.insert(1, read.b);
    rejects("a dropdown that offers V's dimension in H's list", || {
        check_lists(&read, [&with_v, &v_list, &d_list])
    });
    rejects("an offered dimension's pick", || {
        check_no_pick(pick_edit(sim, Edge::H, Z_Q0, Shows::Normal), Edge::H, Z_Q0)
    });
}

/// Asserts the menu `id`'s entries `got` are `entries` of the dimensions in `list`, in order.
fn check_entries(id: &str, got: &[&str], entries: [&str; 8], list: &[usize]) {
    let want: Vec<&str> = list.iter().map(|&i| entries[i]).collect();
    assert_eq!(got, want, "{id}'s entries");
}

/// Asserts `edit`, the pick of `to` on `edge`, is no edit.
fn check_no_pick(edit: Option<SetField>, edge: Edge, to: usize) {
    assert_eq!(edit, None, "{edge:?} to {to}");
}

/// Asserts the H pick `to` from `before` to `after`: `q₁ = σ e_to`, `q₂ = σ e_b`, so the tilts and `γ` are 0, `z₀` kept.
fn check_h_picked(before: &Plane, after: &Plane, to: usize, b: usize, sigma: f64) {
    assert_eq!(after.q1, axis(to, sigma), "q₁ ∝ e_to");
    assert_eq!(after.q2, axis(b, sigma), "q₂ on V's");
    assert_eq!(after.z0, before.z0, "z₀ kept");
}

#[test]
fn mock_cube_axis_buttons_picking_h_sets_the_pair_chart_following_and_one_undo() {
    let (mut app, mut h) = app_with((20.0, 10.0, 5.0), 0.5, frame(Z_Q3, Z_Q3, Z_Q3));
    let before = state(&app);
    // The scale read back off the tilted basis, which is 0.5 only to the platform's sin and cos; the pick keeps this
    // value exactly.
    let kept = View::read(&app.snapshot().sim);
    assert!(
        (kept.scale - 0.5).abs() < 1e-12,
        "the scale read: {}",
        kept.scale
    );
    let unpicked = (texts(&h, &app), frame_texts(&mut h, &mut app));
    click_scope(&mut h, &mut app, COMPASS_H);
    let _ = h.frame(&mut app, Vec::new());
    click_scope(&mut h, &mut app, COMPASS_H_ENTRIES[Z_Q0]);
    settle(&mut h, &mut app);
    let after = state(&app);
    check_h_picked(&before.1, &after.1, Z_Q0, Z_BETA, kept.scale);
    let read = View::read(&app.snapshot().sim);
    check_read_as_picked(&read, (Z_Q0, Z_BETA), &kept);
    let picked = texts(&h, &app);
    check_text(&picked[0], "H z_q0");
    // Chart's horizontal q₁ reads it.
    let rows = frame_texts(&mut h, &mut app);
    check_chart_row(&after.1.q1, &rows, "0.50·z_q0");
    ctrl_z(&mut h, &mut app);
    let undone = state(&app);
    check_one_entry(&before, &after, &undone);
    rejects("a pick that keeps the tilt", || {
        check_h_picked(&before.1, &before.1, Z_Q0, Z_BETA, kept.scale)
    });
    rejects("the new pair read at the old tilt", || {
        check_read_as_picked(&View { a: Z_Q0, ..kept }, (Z_Q0, Z_BETA), &kept)
    });
    rejects("the pair at another zoom", || {
        check_read_as_picked(&read.with_zoom(read.zoom() + 1.0), (Z_Q0, Z_BETA), &kept)
    });
    rejects("H's button left on its old dimension", || {
        check_text(&unpicked.0[0], "H z_q0")
    });
    rejects("Chart's row left on the old q₁", || {
        check_chart_row(&before.1.q1, &unpicked.1, "0.50·z_q0")
    });
    rejects("an undo that leaves the pick", || {
        check_one_entry(&before, &after, &after)
    });
    rejects("a pick that made no entry", || {
        check_one_entry(&before, &before, &undone)
    });
}

/// Asserts `read` is on the axes `(a, b)` with every angle 0, at `kept`'s zoom.
fn check_read_as_picked(read: &View, (a, b): (usize, usize), kept: &View) {
    assert_eq!(
        (read.a, read.b, read.tau1, read.tau2, read.gamma),
        (a, b, 0.0, 0.0, 0.0),
        "the new pair, every angle 0"
    );
    assert_eq!(read.zoom(), kept.zoom(), "the zoom kept");
}

/// Asserts a button's text.
fn check_text(got: &str, want: &str) {
    assert_eq!(got, want, "the button's text");
}

/// Asserts Chart's horizontal row reads `q1` as `want`, the frame's texts `rows` showing it.
fn check_chart_row(q1: &Latent, rows: &[String], want: &str) {
    assert_eq!(basis_text(q1), want);
    assert!(rows.iter().any(|t| t == want), "Chart's row: {rows:?}");
}

#[test]
fn mock_cube_axis_buttons_picking_v_and_a_chart_edit_move_v() {
    let (mut app, mut h) = app_with((0.0, 0.0, 0.0), 1.0, frame(Z_Q3, Z_Q3, Z_Q3));
    let before = state(&app);
    let unpicked = texts(&h, &app);
    let edit = pick_edit(&app.snapshot().sim, Edge::V, Z_Q2, Shows::Normal).expect("offered");
    app.set_field(edit);
    settle(&mut h, &mut app);
    let after = state(&app);
    check_v_picked(&after.1, Z_Q2, Z_ALPHA);
    let picked = texts(&h, &app);
    check_text(&picked[2], "V z_q2");
    ctrl_z(&mut h, &mut app);
    let undone = state(&app);
    check_undone(&undone, &before);
    // Chart: editing q₂ to e_q1 moves V to z_q1, as H and V are the axes the view reads.
    let sim = app.snapshot().sim.clone();
    app.set_field(plane_edit(&sim, (sim.plane.q1, axis(Z_Q1, 1.0))));
    settle(&mut h, &mut app);
    let edited = texts(&h, &app);
    check_text(&edited[2], "V z_q1");
    check_text(&edited[0], "H z_α");
    rejects("V left behind by a Chart edit", || {
        assert_eq!(texts(&h, &app)[2], "V z_β")
    });
    rejects("a V pick that left q₂", || {
        check_v_picked(&before.1, Z_Q2, Z_ALPHA)
    });
    rejects("a V pick that moved H too", || {
        check_v_picked(
            &Plane {
                q1: axis(Z_Q0, 1.0),
                ..after.1.clone()
            },
            Z_Q2,
            Z_ALPHA,
        )
    });
    rejects("V's button left on its old dimension", || {
        check_text(&unpicked[2], "V z_q2")
    });
    rejects("an undo that leaves the pick", || {
        check_undone(&after, &before)
    });
    rejects("V's button left by a Chart edit", || {
        check_text(&unpicked[2], "V z_q1")
    });
    rejects("H moved onto the axis a Chart edit of q₂ took", || {
        check_text("H z_q1", "H z_α")
    });
}

/// Asserts the V pick `to` set `q₂ = e_to` and kept `q₁ = e_h`, at scale 1.
fn check_v_picked(after: &Plane, to: usize, h: usize) {
    assert_eq!(after.q2, axis(to, 1.0), "q₂ ∝ e_to");
    assert_eq!(after.q1, axis(h, 1.0), "q₁ kept");
}

/// Asserts `got`, after one Ctrl+Z, is the state `before` the edit.
fn check_undone(got: &(u32, Plane, SliceFrame), before: &(u32, Plane, SliceFrame)) {
    assert_eq!(got, before, "one Ctrl+Z undoes it");
}

/// Asserts `read` is on the axes `(a, b)` at `τ₁` and `γ`, in degrees.
fn check_reading(read: &View, (a, b): (usize, usize), (tau1, gamma): (f64, f64)) {
    assert_eq!(
        (read.a, read.b),
        (a, b),
        "the axes, τ₁ = {tau1}, γ = {gamma}"
    );
    assert!(
        (read.tau1.to_degrees() - tau1).abs() < 1e-9,
        "τ₁ {} against {tau1}",
        read.tau1.to_degrees()
    );
    assert!(
        (read.gamma.to_degrees() - gamma).abs() < 1e-9,
        "γ {} against {gamma}",
        read.gamma.to_degrees()
    );
}

#[test]
fn mock_cube_axis_buttons_the_pair_the_other_way_round_reads_as_picked() {
    // H and V swapped by two picks: no turn of the preset whose axes they are, so it reads as picked, every angle 0.
    let (mut app, mut h) = app_with((0.0, 0.0, 0.0), 1.0, frame(Z_Q3, Z_Q3, Z_Q3));
    let unswapped = texts(&h, &app);
    swap_h_and_v(&mut h, &mut app);
    let swapped = View::read(&app.snapshot().sim);
    check_reading(&swapped, (Z_BETA, Z_ALPHA), (0.0, 0.0));
    let swapped_texts = texts(&h, &app);
    check_h_and_v(&swapped_texts, "H z_β", "V z_α");
    rejects("the buttons left on the preset's pair", || {
        check_h_and_v(&unswapped, "H z_β", "V z_α")
    });
    // The preset turned by 90°, a real turn, still reads as the preset.
    let turned = Plane {
        z0: [0.0; 8],
        q1: axis(Z_BETA, 1.0),
        q2: axis(Z_ALPHA, -1.0),
    };
    let turned = View::framed(&turned, Some(&frame(Z_Q3, Z_Q3, Z_Q3)));
    check_reading(&turned, (Z_ALPHA, Z_BETA), (0.0, 90.0));
    check_preset(&turned, Some(0));
    rejects("the swapped pair read as a turn of the preset", || {
        check_preset(&swapped, Some(0))
    });
    // And tilted, the swapped pair keeps its axes, past 45° too.
    let tilted = |tau: f64| {
        let (s, c) = tau.to_radians().sin_cos();
        let mut q1 = axis(Z_BETA, c);
        q1[Z_Q3] = s;
        Plane {
            z0: [0.0; 8],
            q1,
            q2: axis(Z_ALPHA, 1.0),
        }
    };
    for tau in [20.0, 60.0, 89.0] {
        let read = View::framed(&tilted(tau), Some(&frame(Z_Q3, Z_Q3, Z_Q3)));
        check_reading(&read, (Z_BETA, Z_ALPHA), (tau, 0.0));
    }
    rejects("the swapped pair read as the preset", || {
        check_reading(&swapped, (Z_ALPHA, Z_BETA), (0.0, 0.0))
    });
    rejects("the preset's real turn read at no angle", || {
        check_reading(&turned, (Z_ALPHA, Z_BETA), (0.0, 0.0))
    });
    rejects("a swapped pair's tilt read as another", || {
        let read = View::framed(&tilted(60.0), Some(&frame(Z_Q3, Z_Q3, Z_Q3)));
        check_reading(&read, (Z_BETA, Z_ALPHA), (20.0, 0.0))
    });
}

/// Asserts the buttons' texts `got` read `h` on H and `v` on V.
fn check_h_and_v(got: &[String; 3], h: &str, v: &str) {
    assert_eq!((got[0].as_str(), got[2].as_str()), (h, v), "H and V");
}

/// Asserts `read` is the preset `want`, or none.
fn check_preset(read: &View, want: Option<usize>) {
    assert_eq!(read.preset, want, "the preset: {read:?}");
}

/// H and V swapped by the picks: `H z_β`, `V z_α`, every angle 0.
fn swap_h_and_v(h: &mut Headless, app: &mut App<MockSide>) {
    for (edge, to) in [(Edge::H, Z_Q0), (Edge::V, Z_ALPHA), (Edge::H, Z_BETA)] {
        let edit = pick_edit(&app.snapshot().sim, edge, to, Shows::Normal).expect("offered");
        app.set_field(edit);
        settle(h, app);
    }
}

/// The view at scale 1 on `(a, b)`, in `f`, at these angles in degrees.
fn view_on((a, b): (usize, usize), f: &SliceFrame, (t1, t2, g): (f64, f64, f64)) -> View {
    let index = |i: u32| usize::try_from(i).expect("a dimension");
    View {
        preset: None,
        a,
        b,
        d: index(f.normal),
        x: index(f.tilt_x),
        y: index(f.tilt_y),
        ..view(t1, t2, g, 1.0)
    }
}

/// Asserts `plane` is `want`'s basis, and reading it in `f` keeps `want`'s axes and writes that basis back: an edit made
/// from the reading does not mirror the chart.
fn check_stays(plane: &Plane, f: &SliceFrame, want: &View) {
    let read = View::framed(plane, Some(f));
    assert_eq!((read.a, read.b), (want.a, want.b), "the axes: {read:?}");
    let (w1, w2) = want.basis();
    let (r1, r2) = read.basis();
    for (got, wanted, what) in [
        (&plane.q1, &w1, "q₁ stored"),
        (&plane.q2, &w2, "q₂ stored"),
        (&r1, &w1, "q₁ read back"),
        (&r2, &w2, "q₂ read back"),
    ] {
        for i in 0..8 {
            assert!(
                (got[i] - wanted[i]).abs() < 1e-12,
                "{what}[{i}]: {} against {}, wanting {want:?}",
                got[i],
                wanted[i]
            );
        }
    }
}

/// The edit the Up arrow makes on the angle `id`, `degrees` of it, with the keys' own [`adjust`], read from `app`'s
/// snapshot.
fn turn_key(app: &App<MockSide>, id: &'static str, degrees: f64) -> Vec<SetField> {
    let mut out = Out::default();
    let step = Adjust {
        scope: id,
        kind: StepKind::Angle,
        times: degrees,
        direction: Direction::Up,
    };
    adjust(&step, app.snapshot(), &mut Scratch::default(), &mut out);
    out.edits
}

/// R-459 G3: a pair picked the other way round stays that pair through the whole of `γ`'s range, and the next edit from
/// any reading of it keeps the chart's handedness: at `γ = 60°` a reading by the dominant axes gives `(z_α, z_β)` at
/// `γ = −30°`, whose [`View::basis`] is the stored one with `z_β`'s components negated.
#[test]
fn mock_cube_axis_buttons_a_swapped_pair_stays_editable_through_the_full_rotation() {
    let default = frame(Z_Q3, Z_Q3, Z_Q3);
    let pair = (Z_BETA, Z_ALPHA);
    let (mut app, mut h) = app_with((0.0, 0.0, 0.0), 1.0, default);
    let unswapped = texts(&h, &app);
    swap_h_and_v(&mut h, &mut app);
    // With the keys: γ from 0 up to its end, one step at a time, each step made from the reading the last one left.
    for k in 1..=12 {
        let at = f64::from(k) * 15.0;
        for edit in turn_key(&app, GAMMA, 15.0) {
            app.set_field(edit);
        }
        settle(&mut h, &mut app);
        check_stays(
            &app.snapshot().sim.plane,
            &default,
            &view_on(pair, &default, (0.0, 0.0, at)),
        );
        check_h_and_v(&texts(&h, &app), "H z_β", "V z_α");
    }
    rejects("the buttons back on the preset's pair under a turn", || {
        check_h_and_v(&unswapped, "H z_β", "V z_α")
    });
    // Back down through 0 to the other end.
    for k in (-12..12).rev() {
        let at = f64::from(k) * 15.0;
        for edit in turn_key(&app, GAMMA, -15.0) {
            app.set_field(edit);
        }
        settle(&mut h, &mut app);
        check_stays(
            &app.snapshot().sim.plane,
            &default,
            &view_on(pair, &default, (0.0, 0.0, at)),
        );
    }
    // The finding's own place: γ = 60°, then a tilt and a turn, each from the last reading.
    app.set_field(basis_edit(
        view_on(pair, &default, (0.0, 0.0, 60.0)).basis(),
    ));
    settle(&mut h, &mut app);
    for (id, degrees, want) in [
        (TAU1, 10.0, (10.0, 0.0, 60.0)),
        (GAMMA, 5.0, (10.0, 0.0, 65.0)),
        (TAU2, -20.0, (10.0, -20.0, 65.0)),
    ] {
        for edit in turn_key(&app, id, degrees) {
            app.set_field(edit);
        }
        settle(&mut h, &mut app);
        check_stays(
            &app.snapshot().sim.plane,
            &default,
            &view_on(pair, &default, want),
        );
    }
    // Every rotation, at the tilts' ends too, in a frame of one target and of two.
    for f in [default, frame(Z_Q3, Z_Q3, Z_MU1)] {
        for (t1, t2) in [
            (0.0, 0.0),
            (20.0, -30.0),
            (90.0, 0.0),
            (0.0, -90.0),
            (90.0, 90.0),
        ] {
            for k in -36..=36 {
                let want = view_on(pair, &f, (t1, t2, f64::from(k) * 5.0));
                let (q1, q2) = want.basis();
                let plane = Plane {
                    z0: [0.0; 8],
                    q1,
                    q2,
                };
                check_stays(&plane, &f, &want);
            }
        }
    }
    // Every preset's pair swapped, in a frame holding neither axis: z_β's, or the default's where z_β is on the pair.
    for (a, b) in PRESETS {
        let pair = (b, a);
        let f = if a == Z_BETA || b == Z_BETA {
            default
        } else {
            frame(Z_BETA, Z_BETA, Z_BETA)
        };
        for k in -36..=36 {
            let want = view_on(pair, &f, (0.0, 0.0, f64::from(k) * 5.0));
            let (q1, q2) = want.basis();
            let plane = Plane {
                z0: [0.0; 8],
                q1,
                q2,
            };
            check_stays(&plane, &f, &want);
        }
    }
    // The chart mirrored: the swapped basis at 60° with z_β's components negated, the reading at −30° on (z_α, z_β).
    let mirrored = |want: &View| {
        let (q1, q2) = view_on((Z_ALPHA, Z_BETA), &default, (0.0, 0.0, -30.0)).basis();
        check_stays(
            &Plane {
                z0: [0.0; 8],
                q1,
                q2,
            },
            &default,
            want,
        );
    };
    let at_60 = view_on(pair, &default, (0.0, 0.0, 60.0));
    rejects("the chart mirrored at 60°", || mirrored(&at_60));
    let (q1, q2) = at_60.basis();
    let stored = Plane {
        z0: [0.0; 8],
        q1,
        q2,
    };
    rejects("the pair read the preset's way", || {
        check_stays(
            &stored,
            &default,
            &view_on((Z_ALPHA, Z_BETA), &default, (0.0, 0.0, -30.0)),
        )
    });
    rejects("the basis at another rotation", || {
        check_stays(
            &stored,
            &default,
            &view_on(pair, &default, (0.0, 0.0, 65.0)),
        )
    });
}

#[test]
fn mock_cube_axis_buttons_a_picked_pair_keeps_its_axes_past_45_degrees() {
    let f = frame(Z_Q3, Z_Q3, Z_MU1);
    let at = |t1: f64, t2: f64| View {
        preset: None,
        a: Z_Q0,
        b: Z_Q2,
        d: Z_Q3,
        x: Z_Q3,
        y: Z_MU1,
        ..view(t1, t2, 0.0, 1.0)
    };
    let read_at = |t1: f64, t2: f64| {
        let (q1, q2) = at(t1, t2).basis();
        View::framed(
            &Plane {
                z0: [0.0; 8],
                q1,
                q2,
            },
            Some(&f),
        )
    };
    for (t1, t2) in [
        (0.0, 0.0),
        (44.0, 10.0),
        (60.0, 70.0),
        (89.0, 89.0),
        (-70.0, 50.0),
    ] {
        check_tilted_pair(&read_at(t1, t2), (Z_Q0, Z_Q2), (t1, t2));
    }
    rejects("past 45°, a reading of the dominant axes", || {
        check_tilted_pair(
            &View {
                a: Z_Q3,
                ..read_at(60.0, 70.0)
            },
            (Z_Q0, Z_Q2),
            (60.0, 70.0),
        )
    });
    rejects("the pair read at another tilt", || {
        check_tilted_pair(&read_at(44.0, 10.0), (Z_Q0, Z_Q2), (60.0, 70.0))
    });
    // A basis leaving the pair and the targets is read by its dominant axes, as before.
    let mut q1: Latent = axis(Z_Q1, 0.8);
    q1[Z_Q2] = 0.2;
    q1[Z_Q0] = 0.1;
    let read = View::framed(
        &Plane {
            z0: [0.0; 8],
            q1,
            q2: axis(Z_MU1, 1.0),
        },
        Some(&f),
    );
    check_axes(&read, (Z_Q1, Z_MU1), "the dominant axes");
    let leaving = read;
    rejects("a basis leaving the pair, read on the pair", || {
        check_axes(&read_at(20.0, 10.0), (Z_Q1, Z_MU1), "the dominant axes")
    });
    // A vector all on the targets has nothing off them: the dominant axes too.
    let read = View::framed(
        &Plane {
            z0: [0.0; 8],
            q1: {
                let mut q = axis(Z_Q3, 0.6);
                q[Z_MU1] = 0.8;
                q
            },
            q2: {
                let mut q = axis(Z_Q0, 0.6);
                q[Z_Q2] = 0.8;
                q
            },
        },
        Some(&f),
    );
    check_axes(&read, (Z_MU1, Z_Q2), "nothing off the targets");
    rejects("a vector on the targets read as one leaving them", || {
        check_axes(&leaving, (Z_MU1, Z_Q2), "nothing off the targets")
    });
    // And one whose q₂ is all on q₁'s axis and the targets.
    let read = View::framed(
        &Plane {
            z0: [0.0; 8],
            q1: {
                let mut q = axis(Z_Q0, 0.6);
                q[Z_Q3] = 0.8;
                q
            },
            q2: {
                let mut q = axis(Z_Q0, 0.8);
                q[Z_MU1] = 0.6;
                q
            },
        },
        Some(&f),
    );
    check_first_axis(&read, Z_Q0);
    rejects("q₁ read on a target", || {
        check_first_axis(&View { a: Z_Q3, ..read }, Z_Q0)
    });
}

/// Asserts `read`'s first axis is `a`.
fn check_first_axis(read: &View, a: usize) {
    assert_eq!(read.a, a, "q₂ has nothing else off the targets: {read:?}");
}

/// Asserts `read` is on the axes `(a, b)`.
fn check_axes(read: &View, (a, b): (usize, usize), what: &str) {
    assert_eq!((read.a, read.b), (a, b), "{what}: {read:?}");
}

/// Asserts `read` is on the axes `(a, b)` at the tilts `(τ₁, τ₂)`, in degrees.
fn check_tilted_pair(read: &View, (a, b): (usize, usize), (t1, t2): (f64, f64)) {
    check_axes(read, (a, b), &format!("({t1}, {t2})"));
    assert!(
        (read.tau1.to_degrees() - t1).abs() < 1e-9,
        "τ₁ ({t1}, {t2})"
    );
    assert!(
        (read.tau2.to_degrees() - t2).abs() < 1e-9,
        "τ₂ ({t1}, {t2})"
    );
}

/// Asserts the view on the axes `(a, b)` at these angles, in `f`, reads back on `(a, b)` with its own angles.
fn check_reads_back((a, b): (usize, usize), f: SliceFrame, (t1, t2, g): (f64, f64, f64)) {
    let v = View {
        preset: None,
        a,
        b,
        d: usize::try_from(f.normal).expect("a dimension"),
        x: usize::try_from(f.tilt_x).expect("a dimension"),
        y: usize::try_from(f.tilt_y).expect("a dimension"),
        ..view(t1, t2, g, 1.0)
    };
    let (q1, q2) = v.basis();
    let read = View::framed(
        &Plane {
            z0: [0.0; 8],
            q1,
            q2,
        },
        Some(&f),
    );
    let at = format!("({t1}, {t2}, {g})");
    assert_eq!((read.a, read.b), (a, b), "{at}");
    for (got, want, what) in [
        (read.tau1, t1, "τ₁"),
        (read.tau2, t2, "τ₂"),
        (read.gamma, g, "γ"),
    ] {
        assert!((got.to_degrees() - want).abs() < 1e-9, "{what} {at}");
    }
}

/// R-459 G3: H and V move only with a pick or an edit in Chart, so no angle in the range reads another pair. At ±90° the
/// kept axis is only `cos 90°`, and at a rotation past 45° it is not the largest off the targets.
#[test]
fn mock_cube_axis_buttons_a_pair_keeps_its_axes_at_the_ends_of_the_range_and_under_a_turn() {
    let f = frame(Z_Q3, Z_Q3, Z_MU1);
    let angles = [
        (90.0, 0.0, 0.0),
        (-90.0, 0.0, 0.0),
        (0.0, 90.0, 0.0),
        (90.0, 90.0, 0.0),
        (-90.0, 45.0, 0.0),
        (90.0, -90.0, 30.0),
        (90.0, 0.0, 30.0),
        (30.0, 20.0, 60.0),
        (0.0, 0.0, 60.0),
        (0.0, 0.0, -120.0),
    ];
    for at in angles {
        check_reads_back((Z_Q0, Z_BETA), f, at);
        // A preset's axes, the other way round, are no turn of that preset (R-459 G3): the determinant says so at the
        // ends too, where it is only `cos² 90°`.
        check_reads_back((Z_BETA, Z_ALPHA), f, at);
        // And a preset stays that preset.
        check_reads_back((Z_ALPHA, Z_BETA), f, at);
    }
    // At the smallest zoom's scale too.
    let small = View {
        preset: None,
        a: Z_Q0,
        b: Z_BETA,
        d: Z_Q3,
        x: Z_Q3,
        y: Z_MU1,
        ..view(90.0, 90.0, 30.0, (-48.0_f64).exp2())
    };
    let (q1, q2) = small.basis();
    let read = View::framed(
        &Plane {
            z0: [0.0; 8],
            q1,
            q2,
        },
        Some(&f),
    );
    check_axes(&read, (Z_Q0, Z_BETA), "zoomed in");
    rejects("zoomed in, a pair read off its tilt's target", || {
        check_axes(&View { a: Z_Q3, ..read }, (Z_Q0, Z_BETA), "zoomed in")
    });
    rejects("a pair read as another's axes", || {
        check_reads_back((Z_Q0, Z_BETA), frame(Z_Q3, Z_Q3, Z_Q0), (90.0, 0.0, 0.0))
    });
}

/// The basis with `q₁` and `q₂` of the components given, each `(axis, value)`, in the frame `(z_q3, z_q3, z_μ1)`.
fn read_of(q1: &[(usize, f64)], q2: &[(usize, f64)]) -> View {
    let vector = |parts: &[(usize, f64)]| -> Latent {
        let mut v = [0.0; 8];
        for &(i, x) in parts {
            v[i] = x;
        }
        v
    };
    View::framed(
        &Plane {
            z0: [0.0; 8],
            q1: vector(q1),
            q2: vector(q2),
        },
        Some(&frame(Z_Q3, Z_Q3, Z_MU1)),
    )
}

#[test]
fn mock_cube_axis_buttons_the_frame_reading_cuts_at_their_boundaries() {
    let (s, c) = 60.0_f64.to_radians().sin_cos();
    // No component is rounding: the smallest off the pair and the targets leaves them, to its dominant axes, the first
    // on the target. Its square would read 0, so the test is on the component.
    let least = f64::MIN_POSITIVE;
    let kept = read_of(&[(Z_Q0, c), (Z_Q3, s)], &[(Z_Q2, 1.0)]);
    check_axes(&kept, (Z_Q0, Z_Q2), "on the pair and the targets");
    let left_q1 = read_of(&[(Z_Q0, c), (Z_Q3, s), (Z_Q1, least)], &[(Z_Q2, 1.0)]);
    check_axes(&left_q1, (Z_Q3, Z_Q2), "off them, by the least");
    let left_q2 = read_of(&[(Z_Q0, c), (Z_Q3, s)], &[(Z_Q2, 1.0), (Z_Q1, least)]);
    check_axes(&left_q2, (Z_Q3, Z_Q2), "off them on q₂");
    rejects("the least off them, kept on the pair", || {
        check_axes(&left_q1, (Z_Q0, Z_Q2), "on the pair and the targets")
    });
    rejects("on them, read as left", || {
        check_axes(&kept, (Z_Q3, Z_Q2), "off them, by the least")
    });
    // A preset's exact determinant: 0 is the preset. The least below it is a typed component. This basis has X on q₁,
    // which the pair the other way round cannot hold, so it reads as at 0 and as the least above it, its axes and its
    // angles not jumping. That is this basis's, not every one's with X != Y: with Y on q₁ the swap keeps the tilt, and
    // it jumps at 0 (the swaps pinned in `..._a_mirrored_basis_is_swapped_where_that_keeps_it`).
    let on = read_of(&[(Z_Q3, 1.0)], &[(Z_BETA, 1.0)]);
    check_preset(&on, Some(0));
    check_tau1(&on, 90.0);
    for component in [-least, least] {
        let near = read_of(&[(Z_ALPHA, component), (Z_Q3, 1.0)], &[(Z_BETA, 1.0)]);
        check_same_reading(&near, &on, component);
    }
    rejects("a reading jumping to another tilt", || {
        check_same_reading(
            &View {
                tau1: on.tau1 - 1e-6,
                ..on
            },
            &on,
            least,
        )
    });
    rejects("a reading jumping to another pair", || {
        check_same_reading(&View { a: Z_Q0, ..on }, &on, least)
    });
    // A preset's frame, exactly: all of it holds, whatever is on the targets.
    let within = read_of(&[(Z_ALPHA, 1.0)], &[(Z_BETA, 1.0), (Z_MU1, 0.5)]);
    check_preset(&within, Some(0));
    rejects("the preset at no tilt, read at τ₁ = 90°", || {
        check_tau1(&within, 90.0)
    });
    // Where no exact reading holds, a preset allows noise. Its determinant: 1e-12 of the total of 2 below 0.
    let noise = 1e-12;
    let on = read_of(
        &[(Z_ALPHA, -1.5e-12), (Z_Q3, 1.0), (Z_Q1, noise)],
        &[(Z_BETA, 1.0)],
    );
    check_preset(&on, Some(0));
    let off = read_of(
        &[(Z_ALPHA, -2.5e-12), (Z_Q3, 1.0), (Z_Q1, noise)],
        &[(Z_BETA, 1.0)],
    );
    check_preset(&off, None);
    rejects("past the rounding, read as the preset", || {
        check_preset(&off, Some(0))
    });
    rejects("within the rounding, read as no preset", || {
        check_preset(&on, None)
    });
    // And what leaves it: 1.5e-18 of the total of 2 holds, on either vector, and 2.5e-18 does not.
    let leak = 1.5e-18_f64.sqrt();
    let on_q2 = read_of(&[(Z_ALPHA, 1.0)], &[(Z_BETA, 1.0), (Z_Q1, leak)]);
    check_preset(&on_q2, Some(0));
    let on_q1 = read_of(&[(Z_ALPHA, 1.0), (Z_Q1, leak)], &[(Z_BETA, 1.0)]);
    check_preset(&on_q1, Some(0));
    let leak = 2.5e-18_f64.sqrt();
    let off = read_of(&[(Z_ALPHA, 1.0)], &[(Z_BETA, 1.0), (Z_Q1, leak)]);
    check_preset(&off, None);
    rejects("past the preset's cut, read as the preset", || {
        check_preset(&off, Some(0))
    });
    rejects("within the preset's cut, read as no preset", || {
        check_preset(&on_q2, None)
    });
    // The determinant, not its terms' sum: 0.36 − 0.64 is mirrored.
    let mirrored = read_of(
        &[(Z_ALPHA, 0.6), (Z_BETA, 0.8)],
        &[(Z_ALPHA, 0.8), (Z_BETA, 0.6)],
    );
    check_preset(&mirrored, None);
    rejects("a mirrored basis read as a turn of the preset", || {
        check_preset(&mirrored, Some(0))
    });
}

/// Asserts `read`'s `τ₁` is `degrees`.
fn check_tau1(read: &View, degrees: f64) {
    assert!(
        (read.tau1.to_degrees() - degrees).abs() < 1e-9,
        "τ₁ {} for {degrees}",
        read.tau1.to_degrees()
    );
}

/// Asserts `near`, the basis with `component` on a typed axis, reads as `on` does: its axes and its angles.
fn check_same_reading(near: &View, on: &View, component: f64) {
    assert_eq!((near.a, near.b), (on.a, on.b), "the axes at {component}");
    for (got, want, what) in [
        (near.tau1, on.tau1, "τ₁"),
        (near.tau2, on.tau2, "τ₂"),
        (near.gamma, on.gamma, "γ"),
    ] {
        assert!((got - want).abs() < 1e-9, "{what} at {component}");
    }
}

/// The largest difference, on any component, between `read`'s basis and `(q1, q2)`.
fn read_back_off(read: &View, q1: &Latent, q2: &Latent) -> f64 {
    let (p1, p2) = read.basis();
    (0..8)
        .map(|k| (p1[k] - q1[k]).abs().max((p2[k] - q2[k]).abs()))
        .fold(0.0, f64::max)
}

/// R-459 G3: a negative determinant is the pair the other way round where that order reads the basis back, or, reading
/// back in neither order, where it keeps more of the tilt; otherwise it stays as typed. A typed mirrored basis outside
/// the map's image keeps its tilt, and the next edit built from the view keeps the plane near it.
#[test]
fn mock_cube_axis_buttons_a_mirrored_basis_is_swapped_where_that_keeps_it() {
    let vector = |parts: &[(usize, f64)]| -> Latent {
        let mut v = [0.0; 8];
        for &(i, x) in parts {
            v[i] = x;
        }
        v
    };
    let in_frame = |f: SliceFrame, q1: &[(usize, f64)], q2: &[(usize, f64)]| {
        View::framed(
            &Plane {
                z0: [0.0; 8],
                q1: vector(q1),
                q2: vector(q2),
            },
            Some(&f),
        )
    };
    let (q1, q2) = ([(Z_ALPHA, -0.6), (Z_Q3, 0.8)], [(Z_BETA, 1.0)]);
    // X != Y: the swap would lose z_q3. The tilt is kept, on the axes as typed; only α's sign is lost.
    let tilt = 0.8_f64.atan2(0.6).to_degrees();
    let as_typed = in_frame(frame(Z_Q3, Z_Q3, Z_MU1), &q1, &q2);
    check_tilt_kept(&as_typed, tilt);
    // The next edit built from the view, a zoom, leaves the axes and the tilt where they were.
    let (p1, p2) = as_typed.with_zoom(1.0).basis();
    let zoomed = View::framed(
        &Plane {
            z0: [0.0; 8],
            q1: p1,
            q2: p2,
        },
        Some(&frame(Z_Q3, Z_Q3, Z_MU1)),
    );
    check_axes(&zoomed, (Z_ALPHA, Z_BETA), "zoomed");
    check_tau1(&zoomed, tilt);
    rejects("a swap that loses the tilt", || {
        assert_eq!((as_typed.a, as_typed.b), (Z_BETA, Z_ALPHA))
    });
    // X = Y: the swap holds it, exactly.
    let swapped = in_frame(frame(Z_Q3, Z_Q3, Z_Q3), &q1, &q2);
    check_swapped_tilt(&swapped, tilt);
    check_reads_basis(&swapped, &vector(&q1), &vector(&q2));
    rejects(
        "X != Y's reading, as typed, where the swap holds it",
        || check_swapped_tilt(&as_typed, tilt),
    );
    rejects("the reading as typed, which loses α's sign", || {
        check_reads_basis(&as_typed, &vector(&q1), &vector(&q2))
    });
    rejects("the swap's reading where X != Y loses z_q3", || {
        check_tilt_kept(&swapped, tilt)
    });
    rejects("a zoom that loses the tilt", || {
        check_tau1(
            &View {
                tau1: 0.0,
                ..zoomed
            },
            tilt,
        )
    });
    // X != Y with the tilt on the target the swap can hold: it reads back, so it is swapped.
    let (q1, q2) = ([(Z_ALPHA, -0.6), (Z_MU1, 0.8)], [(Z_BETA, 1.0)]);
    let read = in_frame(frame(Z_Q3, Z_Q3, Z_MU1), &q1, &q2);
    check_axes(&read, (Z_BETA, Z_ALPHA), "on the target the swap holds");
    check_reads_basis(&read, &vector(&q1), &vector(&q2));
    rejects("the same basis read in the order as typed", || {
        check_reads_basis(
            &View {
                a: Z_ALPHA,
                b: Z_BETA,
                ..read
            },
            &vector(&q1),
            &vector(&q2),
        )
    });
    // The read-back's cut, of a total of 2: 1.5e-18 of it left on z_q3 is the swap's noise, 2.5e-18 is not.
    let at = |left: f64| {
        in_frame(
            frame(Z_Q3, Z_Q3, Z_MU1),
            &[(Z_ALPHA, -1.0), (Z_Q3, left.sqrt())],
            &[(Z_BETA, 1.0)],
        )
    };
    check_axes(&at(1.5e-18), (Z_BETA, Z_ALPHA), "within");
    check_axes(&at(2.5e-18), (Z_ALPHA, Z_BETA), "past it");
    rejects("past the cut, read as the swap's noise", || {
        check_axes(&at(2.5e-18), (Z_BETA, Z_ALPHA), "within")
    });
    rejects("within the cut, read as past it", || {
        check_axes(&at(1.5e-18), (Z_ALPHA, Z_BETA), "past it")
    });
    // Neither order reads it back: the swap only if it keeps more of the tilt. Here it keeps both targets' parts, and
    // the order as typed neither.
    let more = in_frame(
        frame(Z_Q3, Z_Q3, Z_MU1),
        &[(Z_ALPHA, -0.6), (Z_MU1, 0.8)],
        &[(Z_BETA, 1.0), (Z_Q3, 0.3)],
    );
    check_axes(&more, (Z_BETA, Z_ALPHA), "keeps more of the tilt");
    // And a tie, no tilt to keep, stays as typed: the unequal lengths are no reading's, so no read-back.
    let tie = in_frame(
        frame(Z_Q3, Z_Q3, Z_MU1),
        &[(Z_ALPHA, -0.6), (Z_BETA, 0.1)],
        &[(Z_ALPHA, 0.1), (Z_BETA, 1.0)],
    );
    check_axes(&tie, (Z_ALPHA, Z_BETA), "a tie");
    rejects("a tie swapped", || {
        check_axes(&tie, (Z_BETA, Z_ALPHA), "keeps more of the tilt")
    });
    rejects(
        "the swap that keeps more of the tilt, left as typed",
        || check_axes(&more, (Z_ALPHA, Z_BETA), "a tie"),
    );
}

/// Asserts `read`, of `(−0.6 e_α + 0.8 e_q3, e_β)` with X != Y, keeps the tilt `tilt` degrees on the axes as typed, and
/// only α's sign is lost from its basis.
fn check_tilt_kept(read: &View, tilt: f64) {
    check_axes(read, (Z_ALPHA, Z_BETA), "as typed");
    assert!((read.tau1.to_degrees() - tilt).abs() < 1e-9, "{read:?}");
    assert!(
        read.tau2.abs() < 1e-9 && read.gamma.abs() < 1e-9,
        "{read:?}"
    );
    let (p1, p2) = read.basis();
    assert!((p1[Z_Q3] - 0.8).abs() < 1e-12, "z_q3 kept: {p1:?}");
    assert!((p1[Z_ALPHA] - 0.6).abs() < 1e-12, "the sign is what goes");
    assert!((p2[Z_BETA] - 1.0).abs() < 1e-12, "{p2:?}");
}

/// Asserts `read`, of the same basis with X = Y, is the swap: `τ₂ = −tilt`, `γ = −90°`.
fn check_swapped_tilt(read: &View, tilt: f64) {
    check_axes(read, (Z_BETA, Z_ALPHA), "swapped");
    assert!((read.tau2.to_degrees() + tilt).abs() < 1e-9, "{read:?}");
    assert!((read.gamma.to_degrees() + 90.0).abs() < 1e-9, "{read:?}");
}

/// Asserts `read`'s basis is `(q1, q2)`.
fn check_reads_basis(read: &View, q1: &Latent, q2: &Latent) {
    assert!(read_back_off(read, q1, q2) < 1e-12, "{read:?}");
}

/// `tilted_axes`'s determinant is the difference of the products, not of a quotient: the basis below has `1 · 0.5 −
/// 0.5 · 2 = −0.5`, so it is mirrored, while `1 / 0.5 − 0.5 · 2` and `1 · 0.5 − 0.5 / 2` are both positive. Reading back
/// in neither order, it swaps because that keeps more of the tilt, and the swap's reading is worked by hand from
/// `View::in_frame`: on `(z_β, z_α)`, `γ = atan2(1, 2)`, so `sin γ = 1/√5`, `cos γ = 2/√5`; `σ cos τ₁ = √0.5`,
/// `σ sin τ₁ = −0.3/√5`; `tan τ₂ = (0.8/√5) / √5 = 0.16`.
#[test]
fn mock_cube_axis_buttons_a_mirrored_basis_is_told_by_its_determinant() {
    let read = read_of(
        &[(Z_ALPHA, 1.0), (Z_BETA, 0.5), (Z_MU1, 0.8)],
        &[(Z_ALPHA, 2.0), (Z_BETA, 0.5), (Z_Q3, 0.3)],
    );
    let check = |read: &View| {
        let root5 = 5.0_f64.sqrt();
        let (cos1, sin1) = (0.5_f64.sqrt(), -0.3 / root5);
        assert_eq!((read.a, read.b), (Z_BETA, Z_ALPHA), "{read:?}");
        assert_eq!(read.preset, None, "{read:?}");
        for (got, want, what) in [
            (read.gamma, 1.0_f64.atan2(2.0), "γ"),
            (read.scale, cos1.hypot(sin1), "σ"),
            (read.tau1, sin1.atan2(cos1), "τ₁"),
            (read.tau2, 0.8_f64.atan2(5.0), "τ₂"),
        ] {
            assert!((got - want).abs() < 1e-12, "{what}: {got} for {want}");
        }
    };
    check(&read);
    // A mirrored basis that does read in the order as typed: the swap would lose its z_q3, so only the pair's assertion
    // is reached.
    let as_typed = read_of(&[(Z_ALPHA, -0.6), (Z_Q3, 0.8)], &[(Z_BETA, 1.0)]);
    assert_eq!((as_typed.a, as_typed.b), (Z_ALPHA, Z_BETA), "{as_typed:?}");
    rejects("the mirrored basis read in the order as typed", || {
        check(&as_typed)
    });
    // The same pair, with the tilts on each other's targets: the pair's assertion holds, and the angles' do not.
    let other_tilt = read_of(
        &[(Z_ALPHA, 1.0), (Z_BETA, 0.5), (Z_Q3, 0.8)],
        &[(Z_ALPHA, 2.0), (Z_BETA, 0.5), (Z_MU1, 0.3)],
    );
    assert_eq!(
        (other_tilt.a, other_tilt.b),
        (Z_BETA, Z_ALPHA),
        "{other_tilt:?}"
    );
    rejects("the same pair with the tilts exchanged", || {
        check(&other_tilt)
    });
    rejects("the same axes at another turn", || {
        check(&View {
            gamma: read.gamma + 1e-6,
            ..read
        })
    });
}

/// The read-back's cut counts what the swap loses on `q₂` as it does what it loses on `q₁`, the other vector from
/// `..._a_mirrored_basis_is_swapped_where_that_keeps_it`'s cut, of a total of 2: what is left is on `q₂`'s `z_μ1`, which
/// the swap's `q₂ = u₁` cannot hold. A difference of the two vectors' losses would read this one back at any size.
#[test]
fn mock_cube_axis_buttons_the_read_back_counts_what_q2_loses() {
    let at = |left: f64| read_of(&[(Z_ALPHA, -1.0)], &[(Z_BETA, 1.0), (Z_MU1, left.sqrt())]);
    let pair = |read: View| (read.a, read.b);
    let check = |left: f64, want: (usize, usize)| {
        assert_eq!(pair(at(left)), want, "{left} left on q₂");
    };
    check(1.5e-18, (Z_BETA, Z_ALPHA));
    check(2.5e-18, (Z_ALPHA, Z_BETA));
    // Far past it, the order as typed holds the tilt: τ₂ is what is on z_μ1, turned by γ = −180° as q₁ = −e_α says.
    check(0.25, (Z_ALPHA, Z_BETA));
    let check_far = |read: &View| {
        assert!(
            (read.tau2 - (-0.5_f64).atan2(1.0)).abs() < 1e-12,
            "{read:?}"
        );
        assert!((read.gamma.abs() - PI).abs() < 1e-12, "{read:?}");
    };
    check_far(&at(0.25));
    rejects("within the cut, the swap's reading", || {
        check_far(&at(1.5e-18))
    });
    rejects("far past it, τ₂ read off z_μ1 unturned", || {
        check_far(&View {
            tau2: -(-0.5_f64).atan2(1.0),
            ..at(0.25)
        })
    });
    rejects("far past it, γ not turned", || {
        check_far(&View {
            gamma: 0.0,
            ..at(0.25)
        })
    });
    rejects("past the cut, read back as a swap", || {
        check(2.5e-18, (Z_BETA, Z_ALPHA))
    });
    rejects("within the cut, left as typed", || {
        check(1.5e-18, (Z_ALPHA, Z_BETA))
    });
}

#[test]
fn mock_cube_axis_buttons_picking_d_sets_the_normal_and_re_tilts_in_one_entry() {
    let (mut app, mut h) = app_with((20.0, 0.0, 0.0), 1.0, frame(Z_Q3, Z_Q3, Z_Q2));
    assert_eq!(
        texts(&h, &app)[1],
        "X z_q3",
        "τ₁ set: the third axis shows X"
    );
    // The button shows X while τ₁ is set: at rest in Slice mode it is D, the normal's.
    app.manifold.compass.mode = Touch::Slice;
    settle(&mut h, &mut app);
    let unpicked = texts(&h, &app);
    check_text(&unpicked[1], "D z_q3");
    let before = state(&app);
    click_scope(&mut h, &mut app, COMPASS_D);
    let _ = h.frame(&mut app, Vec::new());
    click_scope(&mut h, &mut app, COMPASS_D_ENTRIES[Z_MU1]);
    settle(&mut h, &mut app);
    let after = state(&app);
    let (s, c) = 20.0_f64.to_radians().sin_cos();
    let mut q1 = axis(Z_ALPHA, c);
    q1[Z_MU1] = s;
    check_d_picked(&before, &after, &q1);
    let picked = texts(&h, &app);
    check_text(&picked[1], "D z_μ1");
    ctrl_z(&mut h, &mut app);
    let undone = state(&app);
    check_one_entry(&before, &after, &undone);
    check_text(&texts(&h, &app)[1], "D z_q3");
    rejects("a D pick that moved nothing", || {
        check_d_picked(&before, &before, &q1)
    });
    rejects(
        "a D pick that moved the frame and left the plane in the old one",
        || check_d_picked(&before, &(after.0, before.1.clone(), after.2), &q1),
    );
    rejects("a D pick that moved Y with X", || {
        check_d_picked(
            &before,
            &(after.0, after.1.clone(), frame(Z_MU1, Z_MU1, Z_MU1)),
            &q1,
        )
    });
    rejects("D's button left on the old normal", || {
        check_text(&unpicked[1], "D z_μ1")
    });
    rejects("an undo that leaves D's button on the pick", || {
        check_text(&picked[1], "D z_q3")
    });
    // The control: the same pick as two edits, the frame then the basis, is two entries.
    let d0 = state(&app);
    app.set_field(frame_edit(frame(Z_MU1, Z_MU1, Z_Q2), None));
    app.set_field(frame_edit(
        frame(Z_MU1, Z_MU1, Z_Q2),
        Some((q1, after.1.q2)),
    ));
    settle(&mut h, &mut app);
    let d1 = state(&app);
    ctrl_z(&mut h, &mut app);
    let d2 = state(&app);
    rejects("a D pick that makes two entries", || {
        check_one_entry(&d0, &d1, &d2)
    });
}

/// Asserts the D pick of `z_μ1` from `before` to `after`, from `(z_q3, z_q3, z_q2)` at `τ₁ = 20°`: the normal and X
/// on `z_μ1`, Y kept, `q₁` re-tilted into it as `q1`, `q₂` and `z₀` kept.
fn check_d_picked(
    before: &(u32, Plane, SliceFrame),
    after: &(u32, Plane, SliceFrame),
    q1: &Latent,
) {
    assert_eq!(after.2, frame(Z_MU1, Z_MU1, Z_Q2), "X followed; Y kept");
    let off = q1
        .iter()
        .zip(&after.1.q1)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f64::max);
    assert!(off < 1e-15, "q₁ re-tilted into e_μ1: {:?}", after.1.q1);
    assert_eq!(after.1.q2, before.1.q2, "τ₂ is 0");
    assert_eq!(after.1.z0, before.1.z0, "z₀ kept");
}

/// The normal edit to `to` from the view at `(t1, t2)` on frame `(x, y)`: its normal and whether it carries a basis.
fn normal_edit_of(t1: f64, t2: f64, x: usize, y: usize, to: usize) -> Option<(usize, bool)> {
    use engine::contract::set_field::{Edit, SimField};
    let v = View {
        x,
        y,
        ..view(t1, t2, 0.0, 1.0)
    };
    normal_edit(&v, to).map(|e| match e.edit {
        Edit::Sim(SimField::SliceFrame { basis, frame }) => {
            (frame.normal as usize, basis.is_some())
        }
        other => panic!("not a frame edit: {other:?}"),
    })
}

/// No edit where `want` is `None`; otherwise an edit to normal `to` carrying a basis exactly where `want` says.
fn check_carries(got: Option<(usize, bool)>, to: usize, want: Option<bool>) {
    match (got, want) {
        (None, None) => {}
        (Some((normal, basis)), Some(want)) => {
            assert_eq!(normal, to, "the edit sets the picked normal");
            assert_eq!(
                basis, want,
                "a basis carried only where a follower is tilted"
            );
        }
        (got, want) => panic!("edit {got:?}, wanted basis presence {want:?}"),
    }
}

#[test]
fn mock_cube_axis_buttons_normal_edit_carries_the_basis_only_where_a_follower_is_tilted() {
    let carries = |t1: f64, t2: f64, x: usize, y: usize, to: usize, want: Option<bool>| {
        check_carries(normal_edit_of(t1, t2, x, y, to), to, want)
    };
    // None for the normal it is and for a dimension in the plane.
    carries(0.0, 0.0, Z_Q3, Z_Q3, Z_Q3, None);
    carries(0.0, 0.0, Z_Q3, Z_Q3, Z_ALPHA, None);
    carries(0.0, 0.0, Z_Q3, Z_Q3, Z_BETA, None);
    // Zero angles: the frame alone.
    carries(0.0, 0.0, Z_Q3, Z_Q3, Z_MU1, Some(false));
    // X follows, τ₁ set; Y follows, τ₂ set; a target that does not follow keeps the plane.
    carries(20.0, 0.0, Z_Q3, Z_Q2, Z_MU1, Some(true));
    carries(0.0, 15.0, Z_Q2, Z_Q3, Z_MU1, Some(true));
    carries(20.0, 0.0, Z_Q2, Z_Q3, Z_MU1, Some(false));
    carries(0.0, 15.0, Z_Q3, Z_Q2, Z_MU1, Some(false));
    carries(20.0, 15.0, Z_Q2, Z_Q0, Z_MU1, Some(false));
    // The controls: a wrong basis presence, a wrong normal, and an edit where there must be none.
    let tilted = normal_edit_of(20.0, 0.0, Z_Q3, Z_Q2, Z_MU1);
    rejects("a tilted follower's edit without its basis", || {
        check_carries(tilted.map(|(n, _)| (n, false)), Z_MU1, Some(true))
    });
    rejects("an untilted edit carrying a basis", || {
        check_carries(Some((Z_MU1, true)), Z_MU1, Some(false))
    });
    rejects("the edit to another normal", || {
        check_carries(tilted.map(|(_, b)| (Z_Q2, b)), Z_MU1, Some(true))
    });
    rejects("an edit for a dimension in the plane", || {
        check_carries(Some((Z_ALPHA, false)), Z_ALPHA, None)
    });
}

#[test]
fn mock_cube_axis_buttons_the_third_axis_dropdown_picks_the_target_it_shows() {
    use engine::contract::set_field::{Edit, SimField};
    let (app, _h) = app_with((20.0, 15.0, 0.0), 1.0, frame(Z_Q3, Z_Q3, Z_MU1));
    let sim = &app.snapshot().sim;
    let framed =
        |shows: Shows, to: usize| match pick_edit(sim, Edge::D, to, shows).expect("offered").edit {
            Edit::Sim(SimField::SliceFrame { frame, .. }) => frame,
            other => panic!("not a frame edit: {other:?}"),
        };
    check_routed(&framed);
    rejects("X's and Y's picks routed to each other's target", || {
        check_routed(&|shows: Shows, to: usize| {
            let swapped = match shows {
                Shows::X => Shows::Y,
                Shows::Y => Shows::X,
                Shows::Normal => Shows::Normal,
            };
            framed(swapped, to)
        })
    });
    rejects("D's pick routed to X", || {
        check_routed(&|shows: Shows, to: usize| {
            framed(
                if shows == Shows::Normal {
                    Shows::X
                } else {
                    shows
                },
                to,
            )
        })
    });
    rejects("X's pick routed to the normal", || {
        check_routed(&|shows: Shows, to: usize| {
            framed(
                if shows == Shows::X {
                    Shows::Normal
                } else {
                    shows
                },
                to,
            )
        })
    });
}

/// Asserts the third axis's pick of `z_q2` from the frame `(z_q3, z_q3, z_μ1)`, by `framed` the frame each pick sets,
/// moves the target or the normal the button shows, and only it.
fn check_routed(framed: &dyn Fn(Shows, usize) -> SliceFrame) {
    assert_eq!(framed(Shows::X, Z_Q2), frame(Z_Q3, Z_Q2, Z_MU1), "X's");
    assert_eq!(framed(Shows::Y, Z_Q2), frame(Z_Q3, Z_Q3, Z_Q2), "Y's");
    assert_eq!(framed(Shows::Normal, Z_Q2), frame(Z_Q2, Z_Q2, Z_MU1), "D's");
}

#[test]
fn mock_cube_axis_buttons_a_plucker_steer_taking_d_moves_it_by_the_slice_axis_rule() {
    let (mut app, mut h) = app_with((0.0, 0.0, 0.0), 1.0, frame(Z_Q3, Z_Q3, Z_Q3));
    let before = state(&app);
    let unsteered = texts(&h, &app);
    app.manifold.compass.view = CompassView::Plucker;
    settle(&mut h, &mut app);
    let at = GridPlace::fit(square(&h)).cell_rect(Z_Q2, Z_Q3).center();
    for events in crate::capture::click(at) {
        let _ = h.frame(&mut app, events);
    }
    settle(&mut h, &mut app);
    let after = state(&app);
    let rule = slice_axis(Z_Q2, Z_Q3);
    assert_eq!(rule, Z_ALPHA, "the first axis the plane does not show");
    check_steered(&after, rule);
    app.manifold.compass.view = CompassView::Cube;
    settle(&mut h, &mut app);
    let steered = texts(&h, &app);
    check_steered_texts(&steered);
    ctrl_z(&mut h, &mut app);
    check_one_entry(&before, &after, &state(&app));
    rejects("a steer that leaves D in the plane", || {
        assert_eq!(after.2, frame(Z_Q3, Z_Q3, Z_Q3))
    });
    rejects("a steer that moved nothing", || {
        check_steered(&before, rule)
    });
    rejects("a steer that moved D and left the plane", || {
        check_steered(&(after.0, before.1.clone(), after.2), rule)
    });
    rejects("D moved, its target left in the plane", || {
        check_steered(&(after.0, after.1.clone(), frame(rule, Z_Q3, Z_Q3)), rule)
    });
    rejects("the buttons as before the steer", || {
        check_steered_texts(&unsteered)
    });
    // From the keyboard, Space on the cell cursor, the same rule.
    let sim = app.snapshot().sim.clone();
    check_no_cell(
        compass::views::steer_on(&sim, Z_Q3, Z_Q2),
        "the lower triangle is empty",
    );
    check_no_cell(
        compass::views::steer_on(&sim, Z_Q3, Z_Q3),
        "the diagonal is empty",
    );
    check_no_cell(compass::views::steer_on(&sim, 7, 8), "past the grid");
    let cell = compass::views::steer_on(&sim, 6, 7);
    assert!(cell.is_some());
    rejects("a cell of the upper triangle read as empty", || {
        check_no_cell(cell, "the upper triangle")
    });
}

/// Asserts the steer onto `(z_q2, z_q3)` from `(z_q3, z_q3, z_q3)` left `after`: the normal and both targets on `rule`,
/// the plane on the cell's axes.
fn check_steered(after: &(u32, Plane, SliceFrame), rule: usize) {
    assert_eq!(
        after.2,
        frame(rule, rule, rule),
        "D moved, a target with it"
    );
    assert_eq!(after.1.q1, axis(Z_Q2, 1.0));
    assert_eq!(after.1.q2, axis(Z_Q3, 1.0));
}

/// Asserts the buttons after that steer: H and V on the cell's axes, D on the rule's.
fn check_steered_texts(got: &[String; 3]) {
    assert_eq!(got, &["H z_q2", "D z_α", "V z_q3"]);
}

/// Asserts no cell steers: `got` is `None`.
fn check_no_cell<T: std::fmt::Debug>(got: Option<T>, what: &str) {
    assert!(got.is_none(), "{what}: {got:?}");
}

#[test]
fn mock_cube_axis_buttons_locked_a_pick_turns_about_the_pin() {
    let (mut app, mut h) = app_with((0.0, 0.0, 0.0), 0.5, frame(Z_Q3, Z_Q3, Z_Q3));
    let sim = app.snapshot().sim.clone();
    let anchor = crate::explore::manifold_view::point(&sim.plane, 0.75, 0.5);
    app.set_field(lock_edit(Lock {
        locked: true,
        z_locked: anchor,
    }));
    settle(&mut h, &mut app);
    click_scope(&mut h, &mut app, COMPASS_H);
    let _ = h.frame(&mut app, Vec::new());
    click_scope(&mut h, &mut app, COMPASS_H_ENTRIES[Z_Q0]);
    settle(&mut h, &mut app);
    let after = app.snapshot().sim.plane.clone();
    check_turned_about_the_pin(&sim.plane, &after, &anchor);
    rejects("a pick turned about the centre, the pin displaced", || {
        check_turned_about_the_pin(
            &sim.plane,
            &Plane {
                z0: sim.plane.z0,
                ..after.clone()
            },
            &anchor,
        )
    });
    rejects("a pick that turned nothing", || {
        check_turned_about_the_pin(&sim.plane, &sim.plane, &anchor)
    });
    rejects("the pin's chart point moved along the new q₁", || {
        let mut slid = after.clone();
        for (z, q) in slid.z0.iter_mut().zip(after.q1) {
            *z += 0.1 * q;
        }
        check_turned_about_the_pin(&sim.plane, &slid, &anchor)
    });
}

/// Asserts the locked H pick of `z_q0` turned `before` to `after` about the pin `anchor`, at chart point `(0.75, 0.5)`:
/// `q₁ = 0.5 e_q0`, the anchor at the same chart point, the centre moved.
fn check_turned_about_the_pin(before: &Plane, after: &Plane, anchor: &Latent) {
    assert_eq!(after.q1, axis(Z_Q0, 0.5));
    let (s, t) = crate::explore::manifold_view::chart_coords(after, anchor).expect("a plane");
    assert!(
        (s - 0.75).abs() < 1e-12 && (t - 0.5).abs() < 1e-12,
        "the anchor's chart point kept: ({s}, {t})"
    );
    assert_ne!(after.z0, before.z0, "the centre moved about the pin");
}

/// Tab onto the Compass, Enter in, onto the compass.
fn into_compass(h: &mut Headless, app: &mut App<MockSide>) {
    for key in [Key::Tab; 5] {
        press(h, app, key, NONE);
    }
    press(h, app, Key::Enter, NONE);
    assert_eq!(focus(app), [COMPASS, COMPASS_VIEW]);
}

#[test]
fn mock_cube_axis_buttons_keyboard_tab_enter_right_to_h_enter_down_enter_picks() {
    let (mut app, mut h) = app_with((0.0, 0.0, 0.0), 1.0, frame(Z_Q3, Z_Q3, Z_Q3));
    into_compass(&mut h, &mut app);
    let on_view = path(&app);
    for id in [
        COMPASS_TILT,
        COMPASS_SLICE,
        COMPASS_CUBE,
        COMPASS_PLUCKER,
        COMPASS_JACOBI,
        COMPASS_SHAPE,
        COMPASS_H,
    ] {
        press(&mut h, &mut app, Key::ArrowRight, NONE);
        check_path(&path(&app), &[COMPASS, id], "→ along the siblings");
    }
    rejects("→ that stays on the view", || {
        check_path(&on_view, &[COMPASS, COMPASS_TILT], "→ along the siblings")
    });
    let before = state(&app);
    let on_h = path(&app);
    press(&mut h, &mut app, Key::Enter, NONE);
    let opened = path(&app);
    check_path(
        &opened,
        &[COMPASS, COMPASS_H, COMPASS_H_ENTRIES[Z_ALPHA]],
        "Enter opens it, on its first entry",
    );
    // Esc closes it unchanged.
    press(&mut h, &mut app, Key::Escape, NONE);
    let closed = (path(&app), state(&app));
    check_path(&closed.0, &[COMPASS, COMPASS_H], "Esc closes it");
    check_undone(&closed.1, &before);
    press(&mut h, &mut app, Key::Enter, NONE);
    press(&mut h, &mut app, Key::ArrowDown, NONE);
    let down = path(&app);
    check_path(&down, &[COMPASS, COMPASS_H, COMPASS_H_ENTRIES[Z_Q0]], "↓");
    press(&mut h, &mut app, Key::Enter, NONE);
    let back = path(&app);
    check_path(&back, &[COMPASS, COMPASS_H], "back on the button");
    assert_eq!(app.view.focus.mode, KeyMode::Navigation);
    settle(&mut h, &mut app);
    let after = state(&app);
    let picked = texts(&h, &app);
    check_key_pick(&before, &after, &picked);
    ctrl_z(&mut h, &mut app);
    check_undone(&state(&app), &before);
    rejects("Enter that leaves the menu closed", || {
        check_path(
            &on_h,
            &[COMPASS, COMPASS_H, COMPASS_H_ENTRIES[Z_ALPHA]],
            "Enter opens it, on its first entry",
        )
    });
    rejects("Esc that leaves the menu open", || {
        check_path(&opened, &[COMPASS, COMPASS_H], "Esc closes it")
    });
    rejects("↓ that stays on the first entry", || {
        check_path(&opened, &[COMPASS, COMPASS_H, COMPASS_H_ENTRIES[Z_Q0]], "↓")
    });
    rejects(
        "Enter on an entry that leaves the focus in the menu",
        || check_path(&down, &[COMPASS, COMPASS_H], "back on the button"),
    );
    rejects("Enter on an entry that picks nothing", || {
        check_key_pick(&before, &closed.1, &picked)
    });
    // V and D, then nothing more: → stops on the last sibling.
    press(&mut h, &mut app, Key::ArrowRight, NONE);
    let on_v = path(&app);
    check_path(&on_v, &[COMPASS, COMPASS_V], "→ onto V");
    press(&mut h, &mut app, Key::ArrowRight, NONE);
    check_path(&path(&app), &[COMPASS, COMPASS_D], "→ onto D");
    press(&mut h, &mut app, Key::ArrowRight, NONE);
    check_path(
        &path(&app),
        &[COMPASS, COMPASS_D],
        "D is last with no badge",
    );
    rejects("→ that stays on V", || {
        check_path(&on_v, &[COMPASS, COMPASS_D], "→ onto D")
    });
    rejects("→ past the last sibling", || {
        check_path(
            &[COMPASS.to_owned(), COMPASS_BADGE.to_owned()],
            &[COMPASS, COMPASS_D],
            "D is last with no badge",
        )
    });
    // A D pick from the keyboard, in Tilt with both angles 0: the normal.
    let unpicked = app.snapshot().sim.slice.frame;
    press(&mut h, &mut app, Key::Enter, NONE);
    for _ in 0..4 {
        press(&mut h, &mut app, Key::ArrowDown, NONE);
    }
    check_path(
        &path(&app),
        &[COMPASS, COMPASS_D, COMPASS_D_ENTRIES[Z_MU1]],
        "four ↓ in D's menu",
    );
    press(&mut h, &mut app, Key::Enter, NONE);
    settle(&mut h, &mut app);
    let d_picked = app.snapshot().sim.slice.frame;
    check_frame(d_picked, frame(Z_MU1, Z_MU1, Z_MU1));
    rejects("a D pick from the keys that left the frame", || {
        check_frame(unpicked, frame(Z_MU1, Z_MU1, Z_MU1))
    });
    // A V pick too.
    press(&mut h, &mut app, Key::ArrowLeft, NONE);
    press(&mut h, &mut app, Key::Enter, NONE);
    press(&mut h, &mut app, Key::Enter, NONE);
    settle(&mut h, &mut app);
    assert_eq!(app.snapshot().sim.plane.q2, axis(Z_BETA, 1.0), "z_β kept");
    rejects("the pick by the wrong entry", || {
        assert_eq!(app.snapshot().sim.plane.q1, axis(Z_Q1, 1.0))
    });
}

/// `ViewUI`'s focus path, owned.
fn path(app: &App<MockSide>) -> Vec<String> {
    focus(app).into_iter().map(str::to_owned).collect()
}

/// Asserts the focus path `got` is `want`.
fn check_path(got: &[String], want: &[&str], what: &str) {
    assert_eq!(got, want, "{what}");
}

/// Asserts the slice frame `got` is `want`.
fn check_frame(got: SliceFrame, want: SliceFrame) {
    assert_eq!(got, want, "the slice frame");
}

/// Asserts the keys' H pick of `z_q0` from `before` to `after` at scale 1: `q₁ = e_q0`, H's button on it, one entry.
fn check_key_pick(
    before: &(u32, Plane, SliceFrame),
    after: &(u32, Plane, SliceFrame),
    texts: &[String; 3],
) {
    assert_eq!(after.1.q1, axis(Z_Q0, 1.0), "picked");
    check_text(&texts[0], "H z_q0");
    assert_eq!(after.0, before.0 + 1);
}

#[test]
fn mock_cube_axis_buttons_are_the_cube_views_siblings_and_leave_with_it() {
    let (mut app, mut h) = app_with((20.0, 15.0, 0.0), 1.0, frame(Z_Q3, Z_Q3, Z_MU1));
    let siblings = |app: &App<MockSide>| -> Vec<String> {
        app.keyboard
            .tree(Mode::Explore)
            .children(Some(COMPASS))
            .iter()
            .map(|s| (*s).to_owned())
            .collect()
    };
    let cube = [
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
        COMPASS_BADGE,
    ];
    let shown = siblings(&app);
    check_siblings(&shown, &cube, "the badge last while it is shown");
    // The third axis's breadcrumb name follows what it shows.
    let label = |app: &App<MockSide>| {
        app.keyboard
            .tree(Mode::Explore)
            .get(COMPASS_D)
            .map(|s| s.label)
    };
    let tilted = label(&app);
    check_label(tilted, "X");
    // A focus on H when the mouse picks a grid view moves to that view's button.
    app.view.focus.path = vec![COMPASS.to_owned(), COMPASS_H.to_owned()];
    let on_h = path(&app);
    app.manifold.compass.view = CompassView::Jacobi;
    settle(&mut h, &mut app);
    let in_grid = siblings(&app);
    check_siblings(&in_grid, &cube[..7], "none of them in a grid view");
    check_path(
        &path(&app),
        &[COMPASS, COMPASS_JACOBI],
        "onto the grid view's button",
    );
    rejects("H left focused in a grid view", || {
        check_path(
            &on_h,
            &[COMPASS, COMPASS_JACOBI],
            "onto the grid view's button",
        )
    });
    rejects("the axis buttons kept in a grid view", || {
        check_siblings(&shown, &cube[..7], "none of them in a grid view")
    });
    // Back on the Cube, they come back in order.
    app.manifold.compass.view = CompassView::Cube;
    settle(&mut h, &mut app);
    check_siblings(&siblings(&app), &cube, "back on the Cube");
    rejects("the axis buttons not back on the Cube", || {
        check_siblings(&in_grid, &cube, "back on the Cube")
    });
    // The badge goes when an angle does; a focus on it moves to D.
    app.view.focus.path = vec![COMPASS.to_owned(), COMPASS_BADGE.to_owned()];
    app.view.focus.mode = KeyMode::Interaction;
    let on_badge = (path(&app), app.view.focus.mode);
    let v = View {
        y: Z_MU1,
        ..view(20.0, 0.0, 0.0, 1.0)
    };
    app.set_field(crate::explore::manifold_view::basis_edit(v.basis()));
    settle(&mut h, &mut app);
    check_siblings(&siblings(&app), &cube[..10], "no badge");
    let moved = (path(&app), app.view.focus.mode);
    check_moved_to_d(&moved);
    rejects("a focus left on the gone badge", || {
        check_moved_to_d(&on_badge)
    });
    // A focus elsewhere, and a whole path in the compass, stay where they are.
    app.view.focus.path = vec![COMPASS.to_owned(), COMPASS_V.to_owned()];
    let _ = h.frame(&mut app, Vec::new());
    check_path(&path(&app), &[COMPASS, COMPASS_V], "V stays");
    rejects("V's focus moved to D", || {
        check_path(&moved.0, &[COMPASS, COMPASS_V], "V stays")
    });
    app.view.focus.path = vec!["figure".to_owned(), "gone".to_owned()];
    let _ = h.frame(&mut app, Vec::new());
    let elsewhere = path(&app);
    check_outside(&elsewhere);
    rejects("a focus outside moved into the compass", || {
        check_outside(&moved.0)
    });
    rejects("the third axis's breadcrumb as D while it shows X", || {
        check_label(Some("D"), "X")
    });
    rejects("no breadcrumb name", || check_label(None, "X"));
    rejects("the badge kept as a sibling with no badge", || {
        assert_eq!(siblings(&app), cube)
    });
}

/// Asserts the compass's siblings `got` are `want`.
fn check_siblings(got: &[String], want: &[&str], what: &str) {
    assert_eq!(got, want, "{what}");
}

/// Asserts the third axis's breadcrumb name.
fn check_label(got: Option<&str>, want: &str) {
    assert_eq!(got, Some(want), "the third axis's breadcrumb");
}

/// Asserts a focus on the gone badge moved to D, in navigation mode.
fn check_moved_to_d((at, mode): &(Vec<String>, KeyMode)) {
    check_path(at, &[COMPASS, COMPASS_D], "onto D");
    assert_eq!(*mode, KeyMode::Navigation);
}

/// Asserts a focus outside the compass stayed outside it.
fn check_outside(at: &[String]) {
    assert_eq!(at[0], "figure", "not the compass's to move");
}
