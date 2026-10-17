//! `mock_compass_cube` (REQ-GUI-193; R-451 B1, B3 to B6; render_gui_spec § "The compass: look and views (R-451,
//! R-452)"): the cube as the nav-cube reference draws it, in the dev GUI's dark colours: the isometric projection and
//! its fit, the edges' depth order, opacity and width, the Lambert shade, the grid, the labels, corner numbers and
//! readout per mode, the drags at the reference's rates, and the easing, in the drawing only, the `SetField` carrying
//! the target. The drawing is compared with the reference's own records under
//! `fixtures/screenshot/01_main/compass/reference/` (each record's `made` says how it was made). Each check also runs
//! on an input it must reject (R-176, through `rejects`).

use std::path::PathBuf;

use eframe::egui::{pos2, vec2, Pos2, Rect};
use engine::contract::set_field::{Edit, SimField};
use engine::contract::sim_config::{Lock, Plane};

use super::compass_support::{app_at, drag, drawing, on_slice, plane, setfields, settle, view};
use super::support::rejects;
use crate::capture::{self, CompassCase, Step};
use crate::explore::compass::{
    self, after, clip, drag_edit, ease, ease_factor, face_shade, fill_shade, grid, latent_normal,
    Cut, Drag, Drawing, Iso, Item, Shown, CORNERS, EASE_BASE, EASE_SNAP, EDGES, P3,
};
use crate::explore::manifold_view::slice_tilt::Angles;
use crate::explore::manifold_view::{Touch, View, Z0_HALF_RANGE};

/// The reference's square, 172 points, at the origin, as its records are.
fn square172() -> Rect {
    Rect::from_min_size(pos2(0.0, 0.0), vec2(172.0, 172.0))
}

fn free() -> Lock {
    Lock {
        locked: false,
        z_locked: [0.0; 8],
    }
}

/// The reference's record `name`.
fn record(name: &str) -> serde_json::Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/screenshot/01_main/compass/reference")
        .join(format!("{name}.json"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path:?}: {e}"));
    serde_json::from_str(&text).expect("a record")
}

fn f(v: &serde_json::Value) -> f64 {
    v.as_f64().expect("a number")
}

fn p2(v: &serde_json::Value) -> [f64; 2] {
    [f(&v[0]), f(&v[1])]
}

fn p3(v: &serde_json::Value) -> P3 {
    [f(&v[0]), f(&v[1]), f(&v[2])]
}

/// Within the goldens' 0.01 points.
fn near_point(a: Pos2, b: [f64; 2]) -> bool {
    (f64::from(a.x) - b[0]).abs() <= 0.01 && (f64::from(a.y) - b[1]).abs() <= 0.01
}

/// The labels `labels` of the reference's `want`, by text and place.
fn check_labels(labels: &[compass::Label], want: &serde_json::Value, what: &str) {
    let want = want.as_array().expect("labels");
    assert_eq!(labels.len(), want.len(), "{what}: {labels:?}");
    for (got, want) in labels.iter().zip(want) {
        assert_eq!(got.text, want["text"].as_str().expect("a text"), "{what}");
        assert!(
            near_point(got.at, p2(&want["at"])),
            "{what}: {} at {:?}, the reference's {:?}",
            got.text,
            got.at,
            want["at"]
        );
    }
}

/// Asserts `d` draws the reference's record `r`: the corners, the edges in its order with their opacity and width, the
/// slice at its place among them, the polygon, the labels, the numbers and the readout.
fn check_reference(d: &Drawing, r: &serde_json::Value) {
    for (corner, want) in CORNERS
        .iter()
        .zip(r["corners"].as_array().expect("corners"))
    {
        assert!(
            near_point(d.iso.at(*corner), p2(want)),
            "{corner:?} at {:?}, the reference's {want:?}",
            d.iso.at(*corner)
        );
    }
    let items = r["items"].as_array().expect("items");
    assert_eq!(d.items.len(), items.len(), "the items");
    for (got, want) in d.items.iter().zip(items) {
        match got {
            Item::Edge {
                from,
                to,
                opacity,
                width,
            } => {
                let (a, b) = (p3(&want["edge"][0]), p3(&want["edge"][1]));
                assert!(
                    (a, b) == (*from, *to) || (b, a) == (*from, *to),
                    "the order: {from:?}–{to:?} for {want}"
                );
                assert!((opacity - f(&want["opacity"])).abs() <= 0.003, "{want}");
                assert!((width - f(&want["width"])).abs() <= 1e-6, "{want}");
            }
            Item::Slice => assert!(want.get("slice").is_some(), "the slice for {want}"),
        }
    }
    let slice = d.slice.as_ref().expect("a slice");
    let polygon: Vec<P3> = r["polygon"]
        .as_array()
        .expect("a polygon")
        .iter()
        .map(p3)
        .collect();
    assert_eq!(slice.points, polygon, "the polygon");
    for (p, want) in slice
        .points
        .iter()
        .zip(r["polygon_screen"].as_array().expect("on screen"))
    {
        assert!(near_point(d.iso.at(*p), p2(want)), "{p:?} at {want}");
    }
    check_labels(&d.labels, &r["labels"], "the labels");
    check_labels(&d.numbers, &r["numbers"], "the numbers");
    let readout: Vec<&str> = r["readout"]
        .as_array()
        .expect("a readout")
        .iter()
        .map(|l| l.as_str().expect("a line"))
        .collect();
    assert_eq!(d.readout, readout, "the readout");
}

/// The untilted plane through depth `depth`, in `mode`, drawn in the reference's square.
fn untilted(depth: f64, mode: Touch) -> Drawing {
    let v = view(0.0, 0.0, 0.0, 1.0);
    compass::drawing(
        square172(),
        &plane(&v, [0.0, 0.0, depth]),
        &v,
        mode,
        &free(),
    )
}

#[test]
fn mock_compass_cube_draws_the_references_slice_mode() {
    for (name, depth) in [("slice_+0.25", 0.25), ("slice_0", 0.0)] {
        check_reference(&untilted(depth, Touch::Slice), &record(name));
    }
    let r = record("slice_+0.25");
    let v = view(0.0, 0.0, 0.0, 1.0);
    let off = compass::drawing(
        Rect::from_min_size(pos2(0.0, 0.0), vec2(171.0, 171.0)),
        &plane(&v, [0.0, 0.0, 0.25]),
        &v,
        Touch::Slice,
        &free(),
    );
    rejects("a square a point short", || check_reference(&off, &r));
    rejects("the wrong depth", || {
        check_reference(&untilted(0.0, Touch::Slice), &r)
    });
    rejects("tilt mode's labels", || {
        check_reference(&untilted(0.25, Touch::Tilt), &r)
    });
}

#[test]
fn mock_compass_cube_tilt_mode_against_the_reference() {
    let d = untilted(0.0, Touch::Tilt);
    let tilt = record("tilt_0");
    // The corners and the edges are the reference's tilt mode at 0; its polygon, the horizontal plane, is not the
    // decoder's, which is the rectangle the reference's slice mode draws at depth 0.
    let mut slice = record("slice_0");
    slice["labels"] = serde_json::json!([{
        "text": "q₁, q₂ → z_q3",
        "at": tilt["labels"][1]["at"],
    }]);
    slice["numbers"] = serde_json::json!([]);
    slice["readout"] = serde_json::json!(["mode tilt", tilt["readout"][1], "γ +0.0°"]);
    slice["items"] = tilt["items"].clone();
    check_reference(&d, &slice);
    assert!(d.numbers.is_empty(), "tilt mode numbers no corner");
    let mut horizontal = slice.clone();
    horizontal["polygon"] = tilt["polygon"].clone();
    rejects("the reference's horizontal tilt plane", || {
        check_reference(&d, &horizontal)
    });
    let mut wrong_edge = slice.clone();
    wrong_edge["labels"][0]["at"] = tilt["labels"][0]["at"].clone();
    rejects("the label on the bottom-left edge", || {
        check_reference(&d, &wrong_edge)
    });
}

#[test]
fn mock_compass_cube_readout_per_mode() {
    let v = view(30.0, -12.5, 45.0, 1.0);
    let p = plane(&v, [0.0, 0.0, -0.4]);
    let tilt = compass::readout(&p.z0, &v, Touch::Tilt);
    assert_eq!(
        tilt,
        ["mode tilt", "q₁→z_q3 +30.0° · q₂→z_q3 -12.5°", "γ +45.0°"]
    );
    let slice = compass::readout(&p.z0, &v, Touch::Slice);
    assert_eq!(
        slice,
        [
            "mode slice · q₁→z_q3 +30.0° · q₂→z_q3 -12.5°",
            "z_q3 = -0.40 of range"
        ]
    );
    // Tilted by τ₂ alone, slice mode reads the tilt, not "plane ⟂".
    let v2 = view(0.0, 5.0, 0.0, 1.0);
    assert_eq!(
        compass::readout(&p.z0, &v2, Touch::Slice)[0],
        "mode slice · q₁→z_q3 +0.0° · q₂→z_q3 +5.0°"
    );
    assert_eq!(compass::signed(-0.0, 1), "+0.0", "no negative zero");
    assert_eq!(compass::signed(0.0, 2), "+0.00");
    assert_eq!(compass::signed(-0.004, 2), "-0.00");
    rejects("slice mode reading 'plane ⟂' while tilted", || {
        assert_eq!(slice[0], "mode slice · plane ⟂ z_q3")
    });
}

#[test]
fn mock_compass_cube_iso_and_its_fit() {
    let iso = Iso::fit(square172());
    assert_eq!((iso.cx, iso.cy), (86.0, 86.0));
    assert!((iso.s - 0.78 * 172.0 / 4.0).abs() < 1e-12, "{}", iso.s);
    let [x, y] = iso.at64([1.0, 0.0, 0.0]);
    assert!((x - (86.0 + iso.s * 3f64.sqrt() / 2.0)).abs() < 1e-12);
    assert!((y - (86.0 + iso.s / 2.0)).abs() < 1e-12);
    let [x, y] = iso.at64([0.0, 1.0, 0.0]);
    assert!(x == 86.0 && (y - (86.0 - iso.s)).abs() < 1e-12, "up is up");
    let [x, _] = iso.at64([0.0, 0.0, 1.0]);
    assert!((x - (86.0 - iso.s * 3f64.sqrt() / 2.0)).abs() < 1e-12);
    // The fit takes the smaller of the width's and the height's: wide, the height's; tall, the width's.
    let wide = Iso::fit(Rect::from_min_size(pos2(10.0, 20.0), vec2(400.0, 100.0)));
    assert!((wide.s - 0.78 * 100.0 / 4.0).abs() < 1e-12);
    assert_eq!((wide.cx, wide.cy), (210.0, 70.0));
    let tall = Iso::fit(Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 400.0)));
    assert!((tall.s - 0.78 * 100.0 / (2.0 * 3f64.sqrt())).abs() < 1e-12);
    rejects("the width's fit on a square", || {
        assert!((iso.s - 0.78 * 172.0 / (2.0 * 3f64.sqrt())).abs() < 1e-12)
    });
    // The depth order: the far edges first, the near last.
    let order: Vec<f64> = compass::items(None)
        .iter()
        .map(|i| match i {
            Item::Edge { from, to, .. } => compass::key(*from) + compass::key(*to),
            Item::Slice => unreachable!(),
        })
        .collect();
    assert!(order.windows(2).all(|w| w[0] <= w[1]), "{order:?}");
    assert_eq!(order.len(), EDGES.len());
}

#[test]
fn mock_compass_cube_shade() {
    let l = (0.35f64 * 0.35 + 1.0 + 0.55 * 0.55).sqrt();
    assert!((face_shade([0.0, 1.0, 0.0]) - (0.2 + 0.8 / l)).abs() < 1e-12);
    assert!((face_shade([0.0, 0.0, 1.0]) - (0.2 + 0.8 * 0.55 / l)).abs() < 1e-12);
    // Two-sided: a normal facing away is turned to face the view.
    assert_eq!(face_shade([0.0, -1.0, 0.0]), face_shade([0.0, 1.0, 0.0]));
    assert_eq!(face_shade([0.0, 0.0, -1.0]), face_shade([0.0, 0.0, 1.0]));
    // Facing the view and away from the light, the ambient alone.
    let away = [1.0, -1.0, 0.65];
    assert!(away[0] + away[1] + away[2] > 0.0);
    assert!((face_shade(away) - 0.2).abs() < 1e-12);
    rejects("a one-sided shade", || {
        assert!((face_shade([0.0, 0.0, -1.0]) - 0.2).abs() < 1e-12)
    });
    // The depth term, ±0.05 at the ends.
    let n = [0.0, 0.0, 1.0];
    assert!((fill_shade(n, 2.0) - face_shade(n) - 0.05).abs() < 1e-12);
    assert!((fill_shade(n, -2.0) - face_shade(n) + 0.05).abs() < 1e-12);
    assert_eq!(fill_shade(n, 0.0), face_shade(n));
}

#[test]
fn mock_compass_cube_clip_and_grid() {
    let sq = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let mut back = sq;
    back.reverse();
    for poly in [sq, back] {
        assert_eq!(
            clip(&poly, [-1.0, 0.5], [2.0, 0.5]),
            Some(([0.0, 0.5], [1.0, 0.5]))
        );
        assert_eq!(
            clip(&poly, [0.25, 0.25], [0.75, 0.5]),
            Some(([0.25, 0.25], [0.75, 0.5]))
        );
        assert_eq!(clip(&poly, [-1.0, 2.0], [2.0, 2.0]), None, "misses");
        assert_eq!(
            clip(&poly, [-1.0, 0.5], [0.5, -1.0]),
            None,
            "misses a corner"
        );
        assert_eq!(clip(&poly, [2.0, 0.0], [2.0, 1.0]), None, "beside an edge");
        assert_eq!(
            clip(&poly, [-1.0, 1.0], [1.0, -1.0]),
            None,
            "touches a corner"
        );
    }
    assert_eq!(
        clip(
            &[[0.0, 0.0], [1.0, 0.0], [2.0, 0.0]],
            [0.0, 0.0],
            [1.0, 0.0]
        ),
        None,
        "no area"
    );
    rejects("a segment kept past the polygon", || {
        assert_eq!(
            clip(&sq, [-1.0, 0.5], [2.0, 0.5]),
            Some(([-1.0, 0.5], [2.0, 0.5]))
        )
    });
    // The grid of the untilted slice at 0.25: lines 0.5 apart from −1.9 about the foot, those inside the square.
    let c = Cut {
        normal: [0.0, 0.0, 1.0],
        offset: 0.25,
    };
    let poly = compass::cross_section(c);
    let lines = grid(c, &poly);
    assert_eq!(lines.len(), 8, "{lines:?}");
    for line in &lines {
        for p in line {
            assert!(
                (p[2] - 0.25).abs() < 1e-12
                    && p[0].abs() <= 1.0 + 1e-12
                    && p[1].abs() <= 1.0 + 1e-12
            );
        }
    }
    let mut at: Vec<f64> = lines
        .iter()
        .filter(|l| (l[0][0] - l[1][0]).abs() < 1e-12)
        .map(|l| l[0][0])
        .collect();
    at.sort_by(f64::total_cmp);
    let want: [f64; 4] = [-0.9, -0.4, 0.1, 0.6];
    assert!(
        at.len() == 4 && at.iter().zip(want).all(|(a, b)| (a - b).abs() < 1e-12),
        "{at:?}"
    );
    rejects("a grid from the cube's centre", || {
        assert!(at.iter().any(|a| a.abs() < 1e-12))
    });
}

#[test]
fn mock_compass_cube_drags_at_the_references_rates() {
    let sq = square172();
    let flat = plane(&view(0.0, 0.0, 0.0, 1.0), [0.0, 0.0, 0.1]);
    // Slice: half the width moves the plane H along its normal, one z₀ edit; the vertical does nothing.
    let mut d = Drag::start(Touch::Slice, &flat);
    d.moved(vec2(86.0, 0.0), sq);
    assert_eq!(d.amount, Z0_HALF_RANGE);
    let edit = drag_edit(&flat, &d).expect("an edit");
    assert!(matches!(edit.edit, Edit::Sim(SimField::Z0(_))));
    let moved = after(&flat, &[edit]);
    let n = latent_normal(&flat).expect("a normal");
    for ((m, z), n) in moved.z0.iter().zip(&flat.z0).zip(&n) {
        assert!((m - z - Z0_HALF_RANGE * n).abs() < 1e-12);
    }
    let mut up = Drag::start(Touch::Slice, &flat);
    up.moved(vec2(0.0, -80.0), sq);
    assert_eq!(drag_edit(&flat, &up), None, "a vertical slice drag");
    rejects("a vertical slice drag that moves", || {
        assert!(drag_edit(&flat, &up).is_some())
    });
    // At a corner point the compass draws no plane, and a slice drag sends nothing.
    let corner = plane(&view(90.0, 90.0, 0.0, 1.0), [0.0; 3]);
    let mut at_corner = Drag::start(Touch::Slice, &corner);
    at_corner.moved(vec2(86.0, 0.0), sq);
    assert_eq!(drag_edit(&corner, &at_corner), None);
    // The control: z₀ moved H along the rounding normal of q₁ × q₂, the plane x + y = c.
    let (q1, q2) = compass::frame_basis(&View::of(&corner));
    let rounding = [
        q1[1] * q2[2] - q1[2] * q2[1],
        q1[2] * q2[0] - q1[0] * q2[2],
        q1[0] * q2[1] - q1[1] * q2[0],
    ];
    let len = rounding.iter().map(|x| x * x).sum::<f64>().sqrt();
    let along = compass::lift(rounding.map(|x| x / len), &View::of(&corner));
    let rounded: [f64; 8] = std::array::from_fn(|i| corner.z0[i] + Z0_HALF_RANGE * along[i]);
    rejects("a corner slice drag along the rounding normal", || {
        let edit = drag_edit(&corner, &at_corner).expect("an edit");
        assert_eq!(after(&corner, &[edit]).z0, rounded)
    });
    // Tilt: the full width is 180°, clamped at 90°; a quarter of the height up is +45° to τ₂.
    let mut t = Drag::start(Touch::Tilt, &flat);
    t.moved(vec2(172.0, 0.0), sq);
    assert_eq!(t.tau, (90.0, 0.0));
    rejects("an unclamped tilt drag", || assert_eq!(t.tau.0, 180.0));
    t.moved(vec2(0.0, -43.0), sq);
    assert_eq!(t.tau, (90.0, 45.0));
    // Clamped as it moves: back half the width from the clamp is 0°.
    t.moved(vec2(-86.0, 43.0), sq);
    assert_eq!(t.tau, (0.0, 0.0));
    assert_eq!(drag_edit(&flat, &t), None, "back where it started");
    // γ kept through a tilt.
    let turned = plane(&view(0.0, 0.0, 30.0, 1.0), [0.0; 3]);
    let mut t = Drag::start(Touch::Tilt, &turned);
    t.moved(vec2(43.0, 0.0), sq);
    let edit = drag_edit(&turned, &t).expect("a tilt");
    let a = Angles::of(&View::of(&after(&turned, &[edit])));
    assert!(
        (a.tau1 - 45.0).abs() < 1e-9 && (a.gamma - 30.0).abs() < 1e-9,
        "{a:?}"
    );
}

#[test]
fn mock_compass_cube_easing() {
    assert!((ease_factor(1.0 / 60.0) - (1.0 - 0.0016f64.powf(1.0 / 60.0))).abs() < 1e-15);
    assert_eq!(ease_factor(0.125), ease_factor(0.05), "dt capped at 0.05 s");
    assert!(ease_factor(0.05) > ease_factor(0.049));
    assert_eq!(ease_factor(-1.0), 0.0);
    assert_eq!(ease(0.0, 1.0, 0.5), 0.5);
    assert_eq!(
        ease(0.0, 2.0 * EASE_SNAP, 0.5),
        2.0 * EASE_SNAP,
        "snapped at the cut"
    );
    assert_eq!(ease(0.0, 4.0 * EASE_SNAP, 0.5), 2.0 * EASE_SNAP);
    // γ the short way round: 170° to −170° passes 180°.
    let from = Shown::of(&plane(&view(0.0, 0.0, 170.0, 1.0), [0.0; 3]));
    let to = Shown::of(&plane(&view(0.0, 0.0, -170.0, 1.0), [0.0; 3]));
    let k = ease_factor(1.0 / 60.0);
    let next = Shown::eased(Some(from), to, 1.0 / 60.0);
    let want = 170f64.to_radians() + 20f64.to_radians() * k;
    assert!(
        (next.view.gamma - want).abs() < 1e-12,
        "{}",
        next.view.gamma.to_degrees()
    );
    rejects("γ the long way round", || {
        assert!((next.view.gamma - (170f64.to_radians() - 340f64.to_radians() * k)).abs() < 1e-12)
    });
    // The tilts and the point, at dt = 1/60 s and at the cap: a headless frame of 0.125 s eases as 0.05 s.
    let flat = Shown::of(&plane(&view(0.0, 0.0, 0.0, 1.0), [0.0, 0.0, 0.0]));
    let tilted = Shown::of(&plane(&view(30.0, -20.0, 0.0, 1.0), [0.2, 0.0, 0.5]));
    for (dt, k) in [(1.0 / 60.0, k), (0.125, ease_factor(0.05))] {
        let next = Shown::eased(Some(flat), tilted, dt);
        assert!(
            (next.view.tau1 - 30f64.to_radians() * k).abs() < 1e-12,
            "{dt}"
        );
        assert!(
            (next.view.tau2 + 20f64.to_radians() * k).abs() < 1e-12,
            "{dt}"
        );
        assert!((next.point[2] - tilted.point[2] * k).abs() < 1e-12, "{dt}");
    }
    let capped = Shown::eased(Some(flat), tilted, 0.125);
    rejects("dt uncapped", || {
        assert!(
            (capped.view.tau1 - 30f64.to_radians() * (1.0 - EASE_BASE.powf(0.125))).abs() < 1e-12
        )
    });
    assert_eq!(
        Shown::eased(None, to, 1.0 / 60.0),
        to,
        "from nothing, at once"
    );
    let mut other = to;
    other.view.d = other.view.a;
    assert_eq!(
        Shown::eased(Some(from), other, 1.0 / 60.0),
        other,
        "a new frame, at once"
    );
}

#[test]
fn mock_compass_cube_eases_the_drawing_and_sends_the_target() {
    let flat = plane(&view(0.0, 0.0, 0.0, 1.0), [0.0; 3]);
    let (mut app, mut h) = app_at(&flat);
    let start = app.console().len();
    let sq = super::compass_support::square(&h);
    let from = on_slice(&drawing(&h, &app));
    drag(&mut h, &mut app, from, vec2(sq.width() / 4.0, 0.0));
    assert_eq!(setfields(&app, start), ["basis"], "one edit");
    let sent: Plane = app.snapshot().sim.plane.clone();
    let target = Angles::of(&View::of(&sent));
    assert!(
        (target.tau1 - 45.0).abs() < 1e-9,
        "the target sent: {target:?}"
    );
    let compass = app.manifold.compass;
    assert!(compass.moving, "still easing");
    let shown = compass.shown.expect("shown").view.tau1.to_degrees();
    assert!(shown > 0.0 && shown < 45.0, "the drawing eases: {shown}");
    rejects("an eased value sent through the contract", || {
        assert!((target.tau1 - shown).abs() < 1e-9)
    });
    settle(&mut h, &mut app);
    let rest = app.manifold.compass.shown.expect("shown").view;
    assert!(
        (rest.tau1.to_degrees() - 45.0).abs() < 1e-9,
        "at rest, the target"
    );
    assert_eq!(
        setfields(&app, start),
        ["basis"],
        "the easing sends nothing"
    );
}

// --- the capture's cases and record (R-451 B8) ------------------------------------------------------------------------

/// The case `case` captured on `01_main`, its record, and the record of the drawing at rest on the case's plane.
fn captured(case: CompassCase) -> (serde_json::Value, serde_json::Value) {
    let (_, rec) = capture::shoot_case(
        super::mock_keyboard::canvas(),
        "01_main",
        &[Step::Compass(case)],
    );
    let rec = rec.expect("a record on 01_main");
    let ((t1, t2, g), depth, mode) = case.parameters();
    let v = view(t1, t2, g, 1.0);
    let layout = crate::layout::Layout::new(
        super::support::headless().screen(),
        capture::PIXELS_PER_POINT,
    );
    let sq = compass::square(layout.compass);
    let at_rest = compass::drawing(sq, &plane(&v, [0.0, 0.0, depth]), &v, mode, &free());
    let mut want = compass::record(&at_rest, sq);
    want["pixels_per_point"] = serde_json::json!(capture::PIXELS_PER_POINT);
    (rec, want)
}

/// Asserts the record's polygon is the cycle `want`, within 1e-12.
fn check_polygon(rec: &serde_json::Value, want: &[P3]) {
    let got: Vec<P3> = rec["polygon"]
        .as_array()
        .expect("a polygon")
        .iter()
        .map(p3)
        .collect();
    let n = want.len();
    let near = |a: P3, b: P3| (0..3).all(|i| (a[i] - b[i]).abs() < 1e-12);
    assert!(
        got.len() == n
            && (0..n).any(|s| {
                (0..n).all(|i| near(got[i], want[(s + i) % n]))
                    || (0..n).all(|i| near(got[i], want[(s + n - i) % n]))
            }),
        "{got:?} for {want:?}"
    );
}

#[test]
fn mock_compass_cube_capture_records_each_case_at_rest() {
    let t = 30f64.to_radians().tan();
    let cases: [(CompassCase, Vec<P3>, &str); 4] = [
        (
            CompassCase::Slice,
            vec![
                [-1.0, -1.0, 0.25],
                [1.0, -1.0, 0.25],
                [1.0, 1.0, 0.25],
                [-1.0, 1.0, 0.25],
            ],
            "mode slice · plane ⟂ z_q3",
        ),
        (
            CompassCase::Tilt0,
            vec![
                [-1.0, -1.0, 0.0],
                [1.0, -1.0, 0.0],
                [1.0, 1.0, 0.0],
                [-1.0, 1.0, 0.0],
            ],
            "mode tilt",
        ),
        (
            CompassCase::TiltRect,
            vec![
                [1.0, 1.0, t],
                [1.0, -1.0, t],
                [-1.0, -1.0, -t],
                [-1.0, 1.0, -t],
            ],
            "mode tilt",
        ),
        (
            CompassCase::TiltHex,
            vec![
                [1.0, -1.0, 0.0],
                [1.0, 0.0, 1.0],
                [0.0, 1.0, 1.0],
                [-1.0, 1.0, 0.0],
                [-1.0, 0.0, -1.0],
                [0.0, -1.0, -1.0],
            ],
            "mode tilt",
        ),
    ];
    for (case, polygon, first) in cases {
        let (rec, want) = captured(case);
        assert_eq!(rec["readout"][0], first, "{case:?}");
        assert_eq!(rec["square"], serde_json::json!([172.0, 172.0]));
        check_polygon(&rec, &polygon);
        assert_eq!(rec, want, "{case:?} recorded at rest");
    }
    // The control: the hexagon case recorded while the compass still eases, a few frames from flat.
    let (rec, _) = captured(CompassCase::TiltHex);
    let flat = view(0.0, 0.0, 0.0, 1.0);
    let mut shown = Shown::of(&plane(&flat, [0.0; 3]));
    let target = Shown::of(&plane(&view(45.0, 45.0, 0.0, 1.0), [0.0; 3]));
    for _ in 0..3 {
        shown = Shown::eased(Some(shown), target, 0.125);
    }
    let layout = crate::layout::Layout::new(
        super::support::headless().screen(),
        capture::PIXELS_PER_POINT,
    );
    let sq = compass::square(layout.compass);
    let easing = compass::drawing(
        sq,
        &shown.plane(&plane(&flat, [0.0; 3])),
        &shown.view,
        Touch::Tilt,
        &free(),
    );
    let mut early = compass::record(&easing, sq);
    early["pixels_per_point"] = serde_json::json!(capture::PIXELS_PER_POINT);
    rejects("a record taken before the compass is at rest", || {
        assert_eq!(rec, early)
    });
    let (_, none) = capture::shoot_case(super::mock_keyboard::canvas(), "02_stain", &[]);
    assert_eq!(none, None, "no compass on the Stain page");
}

#[test]
fn mock_compass_cube_case_edits_set_the_frame_and_keep_the_rest() {
    let v = view(10.0, -20.0, 30.0, 0.5);
    let mut start = plane(&v, [0.4, -0.3, 0.6]);
    start.z0[0] = 0.123;
    for case in [
        CompassCase::Slice,
        CompassCase::Tilt0,
        CompassCase::TiltRect,
        CompassCase::TiltHex,
    ] {
        let ((t1, t2, g), depth, _) = case.parameters();
        let set = after(&start, &case.edits(&start));
        let sv = View::of(&set);
        for (i, (got, was)) in set.z0.iter().zip(&start.z0).enumerate() {
            let want = if i == sv.a || i == sv.b {
                0.0
            } else if i == sv.d {
                depth * Z0_HALF_RANGE
            } else {
                *was
            };
            assert_eq!(*got, want, "{case:?}: z₀[{i}]");
        }
        let a = Angles::of(&sv);
        assert!(
            (a.tau1 - t1).abs() < 1e-9 && (a.tau2 - t2).abs() < 1e-9 && (a.gamma - g).abs() < 1e-9,
            "{case:?}: {a:?}"
        );
        assert!((sv.scale - 0.5).abs() < 1e-12, "the scale kept");
        rejects("the scale reset", || {
            assert!((sv.scale - 1.0).abs() < 1e-12)
        });
    }
}

#[test]
fn mock_compass_cube_record_and_write() {
    let sq = Rect::from_min_size(pos2(12.0, 30.0), vec2(172.0, 172.0));
    let v = view(30.0, 0.0, 0.0, 1.0);
    let lock = Lock {
        locked: true,
        z_locked: plane(&v, [0.1, 0.2, 0.05]).z0,
    };
    let d = compass::drawing(sq, &plane(&v, [0.1, 0.2, 0.05]), &v, Touch::Slice, &lock);
    let rec = compass::record(&d, sq);
    assert_eq!(rec["origin"], serde_json::json!([12.0, 30.0]));
    assert_eq!(rec["centre"], serde_json::json!([86.0, 86.0]));
    assert_eq!(f(&rec["s"]), d.iso.s);
    let corner = p2(&rec["corners"][1]);
    let [x, y] = d.iso.at64(CORNERS[1]);
    assert_eq!(corner, [x - 12.0, y - 30.0]);
    let slice = d.slice.as_ref().expect("a slice");
    assert_eq!(rec["fill"], true);
    assert_eq!(rec["grid"], slice.grid.len());
    assert_eq!(
        rec["polygon"].as_array().expect("points").len(),
        slice.points.len()
    );
    let screen = p2(&rec["polygon_screen"][0]);
    let [x, y] = d.iso.at64(slice.points[0]);
    assert_eq!(screen, [x - 12.0, y - 30.0]);
    let at = d
        .items
        .iter()
        .position(|i| *i == Item::Slice)
        .expect("in the order");
    assert_eq!(f(&rec["items"][at]["slice"]), slice.key);
    let edge = d
        .items
        .iter()
        .position(|i| *i != Item::Slice)
        .expect("an edge");
    let Item::Edge {
        from,
        opacity,
        width,
        ..
    } = &d.items[edge]
    else {
        unreachable!()
    };
    assert_eq!(p3(&rec["items"][edge]["edge"][0]), *from);
    assert_eq!(f(&rec["items"][edge]["opacity"]), *opacity);
    assert_eq!(f(&rec["items"][edge]["width"]), *width);
    let [x, y] = d.iso.at64(*from);
    assert_eq!(p2(&rec["items"][edge]["screen"][0]), [x - 12.0, y - 30.0]);
    assert_eq!(rec["labels"][0]["text"], d.labels[0].text);
    assert_eq!(
        p2(&rec["labels"][0]["at"]),
        [
            f64::from(d.labels[0].at.x) - 12.0,
            f64::from(d.labels[0].at.y) - 30.0
        ]
    );
    assert_eq!(
        rec["numbers"].as_array().expect("numbers").len(),
        d.numbers.len()
    );
    assert_eq!(rec["readout"], serde_json::json!(d.readout));
    let pin = d.pin.expect("locked");
    let [x, y] = d.iso.at64(pin);
    assert_eq!(p2(&rec["pin"]), [x - 12.0, y - 30.0]);
    // An edge contact: outlined, not filled.
    let n = compass::drawn_normal(&v).expect("a plane");
    let past = plane(&v, [n[0] * 5.0, n[1] * 5.0, n[2] * 5.0]);
    let contact = compass::record(&compass::drawing(sq, &past, &v, Touch::Slice, &free()), sq);
    assert_eq!(contact["fill"], false);
    assert_eq!(contact["grid"], 0);
    assert_eq!(contact["pin"], serde_json::Value::Null);
    // Written beside the capture, and refused where the directory cannot be made.
    let out = std::env::temp_dir().join(format!("gui-compass-record-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    capture::write_compass(&rec, &out).expect("written");
    let back: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(out.join(capture::COMPASS)).expect("read"))
            .expect("json");
    assert_eq!(back, rec);
    let blocked = out.join(capture::COMPASS).join("below");
    let err = capture::write_compass(&rec, &blocked).expect_err("a file in the way");
    assert!(err.starts_with("cannot create"), "{err}");
    std::fs::remove_dir_all(&out).expect("scratch removed");
    rejects("a record at the square's corner, not its centre", || {
        assert_eq!(rec["centre"], serde_json::json!([98.0, 116.0]))
    });
}
