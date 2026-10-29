//! REQ-GUI-204 (R-457): the Manifold view's Navigate section holds centre `(u, v)`, the zoom, the depth readout, the
//! precision warning and the lock row, and no grid of `z₀` boxes; "Centre z₀" is "Slice centre", its eight sliders
//! each with its number box; "Slice & tilt" is "Tilt", `τ₁`, `τ₂` and `γ` each with its number box and no step slider;
//! the breadcrumb and the keyboard's sub-scopes use the new names. Each check is shown failing on a panel it must
//! reject (R-176, through `rejects`).

use eframe::egui::accesskit::Role;
use eframe::egui::{self, Key, Modifiers};
use engine::contract::sim_config::Plane;

use super::mock_manifold_view::{in_rect, panel_geometry, type_at};
use super::support::{focus, frame_texts, headless, mock_app, nodes, press, rejects, Node};
use crate::app::App;
use crate::explore::manifold_view::depth::{depth_text, NO_WARNING};
use crate::explore::manifold_view::tilt::Angles;
use crate::explore::manifold_view::{
    self as mv, basis_edit, in_plane, row, View, AXES, CHART, NAVIGATE, SECTION_TITLES,
    SLICE_CENTRE, TILT,
};
use crate::explore::MANIFOLD_VIEW;
use crate::headless::Headless;
use crate::mock::MOCK_Z0;
use crate::side::MockSide;

const NONE: Modifiers = Modifiers::NONE;

/// The nodes of `role` in row `i` of `section`, rows `rh` tall.
fn in_row(all: &[Node], section: egui::Rect, rh: f32, i: usize, role: Role) -> Vec<Node> {
    in_rect(all, row(section, rh, i), role)
}

/// Whether some node in `rect` reads `text`.
fn reads(all: &[Node], rect: egui::Rect, text: &str) -> bool {
    all.iter()
        .any(|n| n.text == text && rect.contains(n.rect.center()))
}

/// Asserts Navigate's widgets in `navigate`: its only number boxes are centre u and v, reading the centre's in-plane
/// coordinates `uv`; its only slider is the zoom; under them the depth at `zoom`, the precision row and the lock row.
fn check_navigate(all: &[Node], navigate: egui::Rect, rh: f32, uv: (f64, f64), zoom: f64) {
    let boxes: Vec<f64> = in_rect(all, navigate, Role::SpinButton)
        .iter()
        .filter_map(|n| n.value)
        .collect();
    assert_eq!(
        boxes.len(),
        2,
        "u and v, Navigate's only number boxes: {boxes:?}"
    );
    assert!(
        (boxes[0] - uv.0).abs() < 1e-9 && (boxes[1] - uv.1).abs() < 1e-9,
        "u and v read the centre in the plane, {uv:?}: {boxes:?}"
    );
    assert_eq!(
        in_row(all, navigate, rh, 1, Role::SpinButton).len(),
        2,
        "u and v in row 1"
    );
    assert_eq!(
        in_rect(all, navigate, Role::Slider).len(),
        1,
        "the zoom, its only slider"
    );
    assert_eq!(
        in_row(all, navigate, rh, 2, Role::Slider).len(),
        1,
        "the zoom in row 2"
    );
    for (i, text) in [
        (1, "centre u, v".to_owned()),
        (3, "depth".to_owned()),
        (3, depth_text(zoom)),
        (4, "precision".to_owned()),
        (4, NO_WARNING.to_owned()),
        (5, "unlock".to_owned()),
        (5, "open in Inspector".to_owned()),
        (6, mv::lock::FREE.to_owned()),
    ] {
        assert!(
            reads(all, row(navigate, rh, i), &text),
            "row {i} reads {text:?}"
        );
    }
}

#[test]
fn mock_manifold_sections_navigate_has_no_z0_grid() {
    let mut app = mock_app();
    let mut h = headless();
    // Tilted, so q₁ leans on two axes and a pan along it moves two of z₀'s coordinates.
    let tilted = View {
        tau1: 30f64.to_radians(),
        ..View::read(&app.snapshot().sim)
    };
    app.set_field(basis_edit(tilted.basis()));
    let _ = h.frame(&mut app, Vec::new());
    let (_, _, rh, [_, navigate, ..]) = panel_geometry(&h);
    let all = nodes(&mut h, &mut app);
    let plane = app.snapshot().sim.plane.clone();
    let uv = in_plane(&plane, &plane.z0).expect("a plane");
    let zoom = View::read(&app.snapshot().sim).zoom();
    check_navigate(&all, navigate, rh, uv, zoom);
    // Typing into u pans within the plane, along q₁: no box there edits a single coordinate of z₀.
    let u = in_row(&all, navigate, rh, 1, Role::SpinButton)[0]
        .rect
        .center();
    type_at(&mut h, &mut app, u, &format!("{}", uv.0 + 0.25));
    let moved = app.snapshot().sim.plane.z0;
    let changed: Vec<usize> = (0..8).filter(|&i| moved[i] != plane.z0[i]).collect();
    assert_eq!(changed, [0, 5], "u moves z_α and z_q3, along q₁");
    let along = (moved[5] - plane.z0[5]) / (moved[0] - plane.z0[0]);
    assert!(
        (along - plane.q1[5] / plane.q1[0]).abs() < 1e-9,
        "along q₁: {along}"
    );
    // The control: the panel with the eight z₀ boxes under Navigate, two rows of four.
    let mut grid = all.clone();
    for (k, value) in MOCK_Z0.iter().enumerate() {
        let line = row(navigate, rh, 3 + k / 4);
        grid.push(Node {
            role: Role::SpinButton,
            rect: egui::Rect::from_min_size(
                egui::pos2(line.min.x + 60.0 * (k % 4) as f32, line.min.y),
                egui::vec2(40.0, rh - 2.0),
            ),
            value: Some(*value),
            text: AXES[k].to_owned(),
        });
    }
    rejects("the eight z₀ boxes under Navigate", || {
        check_navigate(&grid, navigate, rh, uv, zoom)
    });
}

/// Asserts Slice centre's rows: row `i + 1` holds `AXES[i]`, its slider and its number box, both reading `z0[i]`.
fn check_slice_centre(all: &[Node], centre: egui::Rect, rh: f32, z0: &[f64; 8]) {
    assert_eq!(in_rect(all, centre, Role::Slider).len(), 8, "eight sliders");
    for (i, want) in z0.iter().enumerate() {
        let (sliders, boxes) = (
            in_row(all, centre, rh, i + 1, Role::Slider),
            in_row(all, centre, rh, i + 1, Role::SpinButton),
        );
        assert_eq!(
            (sliders.len(), boxes.len()),
            (1, 1),
            "row {}: a slider and its box",
            i + 1
        );
        assert_eq!(sliders[0].value, Some(*want), "{}'s slider", AXES[i]);
        assert_eq!(boxes[0].value, Some(*want), "{}'s box", AXES[i]);
        assert!(
            reads(all, row(centre, rh, i + 1), AXES[i]),
            "row {} names {}",
            i + 1,
            AXES[i]
        );
    }
}

/// Asserts Tilt's rows: `τ₁`, `τ₂` and `γ` in rows 1, 3 and 5, each a slider and its number box reading `angles`,
/// the targets' dropdowns in rows 2 and 4, and no other slider.
fn check_tilt(all: &[Node], tilt: egui::Rect, rh: f32, angles: Angles) {
    assert_eq!(
        in_rect(all, tilt, Role::Slider).len(),
        3,
        "τ₁, τ₂ and γ, and no step slider"
    );
    assert_eq!(
        in_rect(all, tilt, Role::SpinButton).len(),
        3,
        "their three boxes"
    );
    for (i, label, want) in [
        (1, "τ₁", angles.tau1),
        (3, "τ₂", angles.tau2),
        (5, "γ", angles.gamma),
    ] {
        let boxes = in_row(all, tilt, rh, i, Role::SpinButton);
        assert_eq!(
            in_row(all, tilt, rh, i, Role::Slider).len(),
            1,
            "{label}'s slider"
        );
        assert!(
            boxes.len() == 1 && (boxes[0].value.expect("a value") - want).abs() < 1e-9,
            "{label}'s box reads {want}: {boxes:?}"
        );
        assert!(reads(all, row(tilt, rh, i), label), "row {i} names {label}");
    }
    for (i, label) in [(2, "tilt τ₁ into"), (4, "tilt τ₂ into")] {
        assert!(
            reads(all, row(tilt, rh, i), label),
            "row {i} reads {label:?}"
        );
        // The dropdown's row starts under its angle's slider, past the angle's name.
        let text = all
            .iter()
            .find(|n| n.text == label && row(tilt, rh, i).contains(n.rect.center()))
            .expect("the label");
        let slider = &in_row(all, tilt, rh, i - 1, Role::Slider)[0];
        assert!(
            (text.rect.min.x - slider.rect.min.x).abs() <= 1.0,
            "{label:?} at {:?}, under the slider at {:?}",
            text.rect,
            slider.rect
        );
    }
}

#[test]
fn mock_manifold_sections_slice_centre_and_tilt_rows() {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    let (_, _, rh, [_, _, centre, tilt]) = panel_geometry(&h);
    let all = nodes(&mut h, &mut app);
    check_slice_centre(&all, centre, rh, &MOCK_Z0);
    check_tilt(&all, tilt, rh, Angles::of(&View::read(&app.snapshot().sim)));
    // Each Slice centre box edits its own component of z₀ and no other.
    for i in 0..8 {
        let before = app.snapshot().sim.plane.z0;
        let all = nodes(&mut h, &mut app);
        let at = in_row(&all, centre, rh, i + 1, Role::SpinButton)[0]
            .rect
            .center();
        let typed = 0.05 * i as f64 - 0.2;
        type_at(&mut h, &mut app, at, &format!("{typed}"));
        let mut want = before;
        want[i] = typed;
        assert_eq!(
            app.snapshot().sim.plane.z0,
            want,
            "{}'s box edits z0[{i}]",
            AXES[i]
        );
    }
    // Tilt's boxes turn the basis and leave z₀ where it is.
    for (i, typed) in [(1, 20.0), (3, -15.0), (5, 40.0)] {
        let before: Plane = app.snapshot().sim.plane.clone();
        let all = nodes(&mut h, &mut app);
        let at = in_row(&all, tilt, rh, i, Role::SpinButton)[0].rect.center();
        type_at(&mut h, &mut app, at, &format!("{typed}"));
        let after = &app.snapshot().sim.plane;
        assert_eq!(after.z0, before.z0, "Tilt's row {i} leaves z₀");
        assert_ne!(
            (after.q1, after.q2),
            (before.q1, before.q2),
            "row {i} turns the basis"
        );
    }
    let all = nodes(&mut h, &mut app);
    let angles = Angles::of(&View::read(&app.snapshot().sim));
    assert!(
        (angles.tau1 - 20.0).abs() < 1e-9
            && (angles.tau2 + 15.0).abs() < 1e-9
            && (angles.gamma - 40.0).abs() < 1e-9,
        "{angles:?}"
    );
    check_tilt(&all, tilt, rh, angles);
    // The control: a Tilt section with a step slider, in the row of τ₁'s target as the old section had it.
    let mut stepped = all.clone();
    stepped.push(Node {
        role: Role::Slider,
        rect: row(tilt, rh, 2).shrink(2.0),
        value: Some(0.0),
        text: "step".to_owned(),
    });
    rejects("a Tilt section with a step slider", || {
        check_tilt(&stepped, tilt, rh, angles)
    });
    rejects("a target's row not under its angle's slider", || {
        let mut shifted = all.clone();
        for n in shifted.iter_mut().filter(|n| n.text == "tilt τ₁ into") {
            n.rect = n.rect.translate(egui::vec2(-16.0, 0.0));
        }
        check_tilt(&shifted, tilt, rh, angles)
    });
    rejects("a Slice centre box on the wrong component", || {
        let mut z0 = app.snapshot().sim.plane.z0;
        z0.swap(0, 1);
        check_slice_centre(&all, centre, rh, &z0)
    });
}

/// Asserts the section headings read the new names, and no text reads an old one.
fn check_names(texts: &[String]) {
    for name in SECTION_TITLES {
        assert!(texts.iter().any(|t| t == name), "no {name:?}");
    }
    assert_eq!(
        SECTION_TITLES,
        ["Chart", "Navigate", "Slice centre", "Tilt"]
    );
    for old in ["Centre z₀", "Slice & tilt"] {
        assert!(
            !texts.iter().any(|t| t.contains(old)),
            "the old name {old:?}"
        );
    }
}

/// Presses `keys` with no modifier and returns the focus path after each.
fn walk(h: &mut Headless, app: &mut App<MockSide>, keys: &[Key]) -> Vec<Vec<String>> {
    keys.iter()
        .map(|key| {
            press(h, app, *key, NONE);
            focus(app).iter().map(|s| (*s).to_owned()).collect()
        })
        .collect()
}

/// Asserts the sections the walk ↓ reached, in order.
fn check_order(paths: &[Vec<String>], want: &[&str]) {
    let reached: Vec<&str> = paths
        .iter()
        .map(|p| p.last().map_or("", String::as_str))
        .collect();
    assert_eq!(reached, want);
}

#[test]
fn mock_manifold_sections_names_order_and_breadcrumb() {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    check_names(&frame_texts(&mut h, &mut app));
    // Tab to the Manifold view, Enter, then ↓: Chart, Navigate, Slice centre, Tilt.
    let _ = walk(&mut h, &mut app, &[Key::Tab, Key::Tab]);
    assert_eq!(focus(&app), [MANIFOLD_VIEW]);
    let paths = walk(
        &mut h,
        &mut app,
        &[Key::Enter, Key::ArrowDown, Key::ArrowDown, Key::ArrowDown],
    );
    check_order(&paths, &[CHART, NAVIGATE, SLICE_CENTRE, TILT]);
    let tree = app.keyboard.tree(engine::contract::view_ui::Mode::Explore);
    assert_eq!(
        tree.children(Some(MANIFOLD_VIEW)),
        [CHART, NAVIGATE, SLICE_CENTRE, TILT]
    );
    // Enter on Tilt: τ₁, its breadcrumb in the new names.
    let _ = walk(&mut h, &mut app, &[Key::Enter]);
    assert_eq!(focus(&app), [MANIFOLD_VIEW, TILT, mv::TAU1]);
    let texts = frame_texts(&mut h, &mut app);
    assert!(
        texts.contains(&"Manifold view › Tilt › τ₁".to_owned()),
        "the breadcrumb: {texts:?}"
    );
    // ↑ ↑ out of Tilt: Slice centre's breadcrumb on its first slider.
    let _ = walk(&mut h, &mut app, &[Key::Escape, Key::ArrowUp, Key::Enter]);
    assert!(
        frame_texts(&mut h, &mut app).contains(&"Manifold view › Slice centre › z_α".to_owned())
    );
    // The controls: the old names, and the old sections' order.
    rejects("the old names", || {
        check_names(&["Chart", "Navigate", "Centre z₀", "Slice & tilt"].map(str::to_owned))
    });
    rejects("the old sections' ids", || {
        check_order(&paths, &[CHART, NAVIGATE, "centre_z0", "slice_tilt"])
    });
    rejects("the old breadcrumb", || {
        assert!(texts.contains(&"Manifold view › Slice & tilt › τ₁".to_owned()))
    });
}
