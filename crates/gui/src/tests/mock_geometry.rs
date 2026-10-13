//! The mock's geometry off its initial unit basis, and the Trajectory panel's selected states and toggles: the mock's
//! out-of-plane direction, hover path, quads and figure points on a scaled, sheared and zoomed basis, where a unit basis
//! hides a wrong scale; the legend ramp over a domain that does not start at zero; and the panel's tabs, view buttons,
//! play and listen as drawn and clicked (REQ-GUI-172, REQ-GUI-173). Each check also runs on an input it must reject
//! (R-176, through `rejects`).

use eframe::egui::accesskit::Toggled;
use engine::contract::sim_config::{Latent, Plane};

use super::support::{headless, mock_app, names, rejects};
use crate::app::App;
use crate::capture::PIXELS_PER_POINT;
use crate::explore::manifold_view::dot;
use crate::explore::trajectory::{index_at, kept_row, playhead_width, wraps, Tab, KEPT_IDS};
use crate::headless::Headless;
use crate::layout::Layout;
use crate::mock::legend::{brightness, evaluate, Dimension, FakeStain, DARKEST, RAMP_SAMPLES};
use crate::mock::quads::{self, on_figure, overlaps};
use crate::mock::sample::{hover_path, out_of_plane, sample};
use crate::mock::{MOCK_Q1, MOCK_Q2, MOCK_T_MAX, MOCK_Z0};
use crate::side::MockSide;

/// `x` along axis `i`.
fn axis(i: usize, x: f64) -> Latent {
    std::array::from_fn(|j| if j == i { x } else { 0.0 })
}

/// A plane through `z0` with basis `q1`, `q2`.
fn plane(z0: Latent, q1: Latent, q2: Latent) -> Plane {
    Plane { z0, q1, q2 }
}

#[test]
fn mock_hover_geometry_out_of_plane_is_a_unit_normal() {
    // The mock's own basis, z_α and z_β: of the six axes off it, the last.
    let mock = plane(MOCK_Z0, MOCK_Q1, MOCK_Q2);
    assert_eq!(out_of_plane(&mock), Some(axis(7, 1.0)));
    // A scaled, sheared basis with a part on the last axis: the direction is unit and off both basis vectors.
    let q1 = [2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.5, 1.0];
    let q2 = [0.5, 3.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.25];
    let check = |n: Latent| {
        assert!((dot(&n, &n) - 1.0).abs() < 1e-12, "{n:?} is not unit");
        assert!(dot(&n, &q1).abs() < 1e-12, "{n:?} is not off q1");
        assert!(dot(&n, &q2).abs() < 1e-12, "{n:?} is not off q2");
    };
    let n = out_of_plane(&plane(MOCK_Z0, q1, q2)).expect("a direction");
    check(n);
    rejects("an axis taken as it is", || check(axis(7, 1.0)));
    rejects("a direction twice too long", || check(n.map(|x| 2.0 * x)));
    // A basis with a part on every axis, so no axis is off it whole: the residual is shorter than one, and normalised.
    let all = [1.0; 8];
    let alt = [1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0];
    let n = out_of_plane(&plane(MOCK_Z0, all, alt)).expect("a direction");
    assert!((dot(&n, &n) - 1.0).abs() < 1e-12, "{n:?} is not unit");
    assert!(dot(&n, &all).abs() < 1e-12 && dot(&n, &alt).abs() < 1e-12);
    // A basis with a part on every axis, so no axis is off it whole: the residual is shorter than one, and normalised.
    let all = [1.0; 8];
    let alt = [1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0];
    let n = out_of_plane(&plane(MOCK_Z0, all, alt)).expect("a direction");
    assert!((dot(&n, &n) - 1.0).abs() < 1e-12, "{n:?} is not unit");
    assert!(dot(&n, &all).abs() < 1e-12 && dot(&n, &alt).abs() < 1e-12);
    // A zero basis vector has no plane to leave.
    assert_eq!(out_of_plane(&plane(MOCK_Z0, [0.0; 8], q2)), None);
    assert_eq!(out_of_plane(&plane(MOCK_Z0, q1, [0.0; 8])), None);
}

#[test]
fn mock_hover_geometry_path_on_a_scaled_sheared_basis_is_pinned() {
    let p = plane(
        MOCK_Z0,
        axis(0, 0.5),
        [0.1, 0.4, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    );
    let path = hover_path(&p, [0.4, 0.6]);
    assert_eq!(path.len(), 160);
    let print = |path: &[crate::mock::sample::PathPoint]| -> (f64, f64) {
        let at = path
            .iter()
            .enumerate()
            .map(|(i, q)| (i as f64 + 1.0) * (q.at[0] + 3.0 * q.at[1]))
            .sum();
        let alpha = path.iter().map(|q| f64::from(q.alpha)).sum();
        (at, alpha)
    };
    let (at, alpha) = print(&path);
    assert!((at - 27_410.660_121_150_48).abs() < 1e-6, "{at}");
    assert!((alpha - 123.363_011_002_540_59).abs() < 1e-4, "{alpha}");
    let unit = print(&hover_path(&plane(MOCK_Z0, MOCK_Q1, MOCK_Q2), [0.4, 0.6]));
    rejects("the path of the unit basis", || {
        assert!((unit.0 - 27_410.660_121_150_48).abs() < 1e-6, "{}", unit.0)
    });
    // The escaping body drifts out only from 70% of its end on: before, its radius is the bounded wobble's.
    let s = sample(
        &plane(MOCK_Z0, MOCK_Q1, MOCK_Q2),
        [0.0125, 0.37],
        MOCK_T_MAX,
        MOCK_T_MAX,
    );
    let end = s.t_end.expect("decided at the horizon");
    let n = s.bodies[0].len();
    let early = index_at(end * 0.5, s.path_t, n);
    let [x, y] = s.bodies[0][early];
    let r = x.hypot(y / 0.7);
    assert!(r <= 1.35 + 1e-9, "body 0 out at {r} before 70% of its end");
}

#[test]
fn mock_tile_bounds_geometry_off_the_unit_basis() {
    // A figure point on a scaled basis: (u, v) = (1, 0.25) on q₁ = 2 z_α, q₂ = 0.5 z_β is (s, t) = (0.75, 0.75).
    let p = plane([0.0; 8], axis(0, 2.0), axis(1, 0.5));
    let at = on_figure(&p, (1.0, 0.25)).expect("a point");
    assert!(
        (at[0] - 0.75).abs() < 1e-12 && (at[1] - 0.75).abs() < 1e-12,
        "{at:?}"
    );
    rejects("the unit basis's point", || {
        let u = on_figure(&plane([0.0; 8], MOCK_Q1, MOCK_Q2), (1.0, 0.25)).unwrap();
        assert!((u[0] - 0.75).abs() < 1e-12, "{u:?}")
    });
    assert_eq!(
        on_figure(&plane([0.0; 8], [0.0; 8], axis(1, 0.5)), (1.0, 0.25)),
        None
    );
    assert!(quads::quads(&plane([0.0; 8], axis(0, 2.0), [0.0; 8])).is_empty());
    // Quads that only touch do not overlap, on each side; quads that share area do.
    let a = [0.0, 0.0, 1.0, 1.0];
    for b in [
        [1.0, 0.0, 2.0, 1.0],
        [-1.0, 0.0, 0.0, 1.0],
        [0.0, 1.0, 1.0, 2.0],
        [0.0, -1.0, 1.0, 0.0],
    ] {
        assert!(!overlaps(&a, &b), "{a:?} and {b:?} only touch");
    }
    assert!(overlaps(&a, &[0.5, 0.5, 1.5, 1.5]));
    rejects("touching taken as overlapping", || {
        assert!(overlaps(&a, &[1.0, 0.0, 2.0, 1.0]))
    });
    // The tree, pinned on three views: zoomed out (MIN_DEPTH binds), a view four wide (the quads a quarter of the
    // figure stay whole away from the frontier), and zoomed in on the frontier (MAX_DEPTH binds).
    let pin = |p: &Plane| -> (usize, f64) {
        let leaves = quads::quads(p);
        let sum = leaves
            .iter()
            .map(|c| c[0] + 2.0 * c[1] + 3.0 * c[2] + 5.0 * c[3])
            .sum();
        (leaves.len(), sum)
    };
    // SPLIT_NEAR is met exactly: 4/2⁸ of 0.625 is 1/40.
    assert_eq!((4.0 / 256.0) / 0.625, crate::mock::quads::SPLIT_NEAR);
    let mut on_frontier = [0.0; 8];
    on_frontier[0] = 0.65;
    on_frontier[1] = 0.45;
    let views = [
        (plane([0.0; 8], axis(0, 8.0), axis(1, 8.0)), (97, 453.0)),
        (plane([0.0; 8], axis(0, 2.0), axis(1, 2.0)), (367, 1671.75)),
        (
            plane(on_frontier, axis(0, 0.01), axis(1, 0.01)),
            (12, 69.09375),
        ),
        // A view 0.625 wide on the frontier, where the quads of depth 8 are exactly SPLIT_NEAR of it and stay whole.
        (
            plane(on_frontier, axis(0, 0.3125), axis(1, 0.3125)),
            (468, 2572.609375),
        ),
    ];
    for (p, (n, sum)) in &views {
        let (got_n, got_sum) = pin(p);
        assert_eq!(got_n, *n, "{p:?}");
        assert!((got_sum - sum).abs() < 1e-9, "{p:?}: {got_sum}");
    }
    rejects("another view's tree", || {
        assert_eq!(pin(&views[0].0).0, views[1].1 .0)
    });
}

#[test]
fn mock_legend_keys_ramp_over_a_domain_off_zero() {
    // 30 of [10, 50] is halfway: 1 − ½(1 − DARKEST).
    let want = 1.0 - 0.5 * (1.0 - DARKEST);
    assert!((brightness(30.0, 10.0, 50.0) - want).abs() < 1e-12);
    rejects("the domain read from zero", || {
        assert!((brightness(30.0, 0.0, 50.0) - want).abs() < 1e-12)
    });
    let stain = FakeStain {
        out: vec![Dimension::Brightness {
            title: "t",
            lo: 10.0,
            hi: 50.0,
        }],
    };
    let legend = evaluate(&stain);
    let engine::contract::legend::LegendKey::Ramp { samples, .. } = &legend.keys[0] else {
        panic!("a bar key");
    };
    assert_eq!(samples.len(), RAMP_SAMPLES);
    let grey = |i: usize| {
        let b = 1.0 - (i as f64 / (RAMP_SAMPLES - 1) as f64) * (1.0 - DARKEST);
        let v = (b * 255.0).round() as u8;
        [v, v, v]
    };
    for (i, got) in samples.iter().enumerate() {
        assert_eq!(*got, grey(i), "sample {i}");
    }
    rejects("samples one step on", || assert_eq!(samples[1], grey(2)));
}

#[test]
fn mock_hover_geometry_trajectory_helpers() {
    // The seam: a step across θ's wrap, near 0 and near 2π, is not drawn; a step between two angles near π is.
    assert!(wraps([0.1, 1.0], [6.2, 1.0]));
    assert!(!wraps([3.0, 1.0], [3.5, 1.0]));
    rejects("a short step taken as a wrap", || {
        assert!(wraps([3.0, 1.0], [3.5, 1.0]))
    });
    // Halfway through five points is the third.
    assert_eq!(index_at(10.0, 20.0, 5), 2);
    assert_eq!(index_at(5.0, 20.0, 5), 1);
    rejects("the fourth", || assert_eq!(index_at(10.0, 20.0, 5), 3));
    // The playhead's width: the row less 70 points, never under 40.
    assert_eq!(playhead_width(200.0), 130.0);
    assert_eq!(playhead_width(110.0), 40.0);
    assert_eq!(playhead_width(111.0), 41.0);
    assert_eq!(playhead_width(20.0), 40.0);
    rejects("the whole row", || assert_eq!(playhead_width(200.0), 200.0));
}

/// Each toggle-reporting widget of one more frame: its label and whether it is drawn selected.
fn toggles(h: &mut Headless, app: &mut App<MockSide>) -> Vec<(String, bool)> {
    let output = h.frame(app, Vec::new());
    let Some(update) = &output.platform_output.accesskit_update else {
        return Vec::new();
    };
    update
        .nodes
        .iter()
        .filter_map(|(_, n)| {
            let on = match n.toggled()? {
                Toggled::True => true,
                Toggled::False => false,
                Toggled::Mixed => return None,
            };
            Some((n.label().unwrap_or_default().to_owned(), on))
        })
        .collect()
}

/// Whether the widget labelled `name` is drawn selected.
fn selected(h: &mut Headless, app: &mut App<MockSide>, name: &str) -> bool {
    let all = toggles(h, app);
    all.iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("no `{name}` in {all:?}"))
        .1
}

/// Clicks the Trajectory panel's control named `name` (Time has a ▶ of its own).
fn click(h: &mut Headless, app: &mut App<MockSide>, name: &str) {
    let panel = Layout::new(h.screen(), PIXELS_PER_POINT).trajectory;
    let scale = f64::from(h.pixels_per_point());
    let all = names(h, app);
    let centre = all
        .iter()
        .filter(|n| n.name == name)
        .filter_map(|n| n.rect)
        .map(|r| {
            eframe::egui::pos2(
                ((r[0] + r[2]) / 2.0 / scale) as f32,
                ((r[1] + r[3]) / 2.0 / scale) as f32,
            )
        })
        .find(|c| panel.contains(*c))
        .unwrap_or_else(|| panic!("no `{name}` in the panel"));
    for events in crate::capture::click(centre) {
        let _ = h.frame(app, events);
    }
}

#[test]
fn mock_hover_geometry_panel_selection_and_toggles() {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    // The tabs: under cursor at the start; a kept orbit's tab, and its row, once shown.
    let kept = sample(
        &plane(MOCK_Z0, MOCK_Q1, MOCK_Q2),
        [0.4, 0.4],
        0.0,
        MOCK_T_MAX,
    );
    let row = kept_row(0, &kept);
    app.view.kept_orbits.orbits.push(kept);
    assert!(selected(&mut h, &mut app, "under cursor"));
    assert!(!selected(&mut h, &mut app, KEPT_IDS[0].3));
    assert!(!selected(&mut h, &mut app, &row));
    app.trajectory.tab = Tab::Kept(0);
    assert!(!selected(&mut h, &mut app, "under cursor"));
    assert!(selected(&mut h, &mut app, KEPT_IDS[0].3));
    assert!(selected(&mut h, &mut app, &row));
    rejects("under cursor still selected", || {
        assert!(selected(&mut h, &mut app, "under cursor"))
    });
    // The view: the sphere selected, until unwrapped is clicked.
    assert!(selected(&mut h, &mut app, "sphere"));
    assert!(!selected(&mut h, &mut app, "unwrapped"));
    click(&mut h, &mut app, "unwrapped");
    assert!(app.trajectory.unwrapped);
    assert!(!selected(&mut h, &mut app, "sphere"));
    assert!(selected(&mut h, &mut app, "unwrapped"));
    // Play and listen: a click toggles each, on and off.
    assert!(!app.trajectory.playing);
    click(&mut h, &mut app, "▶");
    assert!(app.trajectory.playing);
    click(&mut h, &mut app, "⏸");
    assert!(!app.trajectory.playing);
    assert!(!app.trajectory.listen);
    click(&mut h, &mut app, "listen");
    assert!(app.trajectory.listen);
    click(&mut h, &mut app, "listen");
    assert!(!app.trajectory.listen);
    rejects("listen left on", || assert!(app.trajectory.listen));
}
