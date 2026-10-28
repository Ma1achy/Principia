//! What TASK-M6-27's screens paint, shape by shape, against the geometry their own functions give: the Trajectory
//! panel's sphere, unwrapped view and real space, the hover label, the Legend's swatches, bar and ticks, and Time's
//! scrubber ticks (REQ-GUI-172, REQ-GUI-157's panel frame). Each check also runs on an input it must reject (R-176,
//! through `rejects`).

use eframe::egui::{self, pos2, vec2, Color32, Pos2, Rect, Shape};
use engine::contract::legend::{LegendEntry, LegendKey, SwatchStyle};
use engine::contract::sample::Sample;

use super::support::{headless, rejects, shapes};
use crate::app::App;
use crate::capture::{hover_point, FORMAT, PIXELS_PER_POINT};
use crate::explore::hover_label::{self, placed, BACKING, PADDING};
use crate::explore::legend::{
    colour, entry_rect, hatch_at, hatch_mesh, key_rects, nan_inf_at, nan_inf_mesh, ramp_mesh,
    MARGIN, ROW_H, SWATCH,
};
use crate::explore::time::{self, SCRUBBER};
use crate::explore::trajectory::{
    areas, fit, index_at, on_sphere, on_square, unwrapped_point, view, wraps, AXIS_COLOURS,
    BODY_COLOURS, FAR_SIDE,
};
use crate::headless::Headless;
use crate::layout::Layout;
use crate::mock::MockEngine;
use crate::side::MockSide;

/// A frozen mock with the pointer over the figure at the capture's hover point, the sphere at `angle`, not turning.
fn hovering(angle: f64) -> (App<MockSide>, Headless, Layout) {
    let mut app = App::new(MockSide::new(MockEngine::frozen(), None), FORMAT);
    let mut h = headless();
    let layout = Layout::new(h.screen(), PIXELS_PER_POINT);
    let _ = h.frame(&mut app, Vec::new());
    let _ = h.frame(
        &mut app,
        vec![egui::Event::PointerMoved(hover_point(layout.figure))],
    );
    for _ in 0..2 {
        let _ = h.frame(&mut app, Vec::new());
    }
    app.trajectory.turn = false;
    app.trajectory.angle = angle;
    (app, h, layout)
}

/// The line segments of `drawn`, with their colours.
fn segments(drawn: &[Shape]) -> Vec<([Pos2; 2], Color32)> {
    drawn
        .iter()
        .filter_map(|s| match s {
            Shape::LineSegment { points, stroke } => Some((*points, stroke.color)),
            _ => None,
        })
        .collect()
}

/// The filled circles of `drawn`: centre, radius, fill.
fn dots(drawn: &[Shape]) -> Vec<(Pos2, f32, Color32)> {
    drawn
        .iter()
        .filter_map(|s| match s {
            Shape::Circle(c) => Some((c.center, c.radius, c.fill)),
            _ => None,
        })
        .collect()
}

/// The texts of `drawn`, each with its galley's rect on the screen (a right-aligned galley's rect lies left of its pos).
fn texts(drawn: &[Shape]) -> Vec<(String, Rect)> {
    drawn
        .iter()
        .filter_map(|s| match s {
            Shape::Text(t) => Some((
                t.galley.text().to_owned(),
                t.galley.rect.translate(t.pos.to_vec2()),
            )),
            _ => None,
        })
        .collect()
}

fn near(a: Pos2, b: Pos2) -> bool {
    (a - b).length() < 1e-3
}

/// The sphere's screen point of `(θ, φ)` at `angle`, in `rect`, as the panel draws it.
fn sphere_point(rect: Rect, p: [f64; 2], angle: f64) -> (Pos2, f64) {
    let (xy, depth) = view(on_sphere(p), angle);
    (on_square(rect, [xy[0] * 0.86, xy[1] * 0.86]), depth)
}

/// Asserts the sphere of `orbit` at `angle` in `rect` is drawn in `drawn`: its outline, its three axes and its path,
/// near and far, and the playhead's dot at point `i`.
fn check_sphere(drawn: &[Shape], rect: Rect, orbit: &Sample, angle: f64, i: usize, ink: Color32) {
    let lines = segments(drawn);
    let circles = dots(drawn);
    let r = rect.width().min(rect.height()) / 2.0;
    assert!(
        drawn.iter().any(|s| matches!(s, Shape::Circle(c) if near(c.center, rect.center()) && (c.radius - r * 0.86).abs() < 1e-3)),
        "the sphere's outline"
    );
    for (axis, colour) in AXIS_COLOURS.iter().enumerate() {
        let tip: [f64; 3] = std::array::from_fn(|k| if k == axis { 1.12 } else { 0.0 });
        let (xy, _) = view(tip, angle);
        let end = on_square(rect, [xy[0] * 0.86, xy[1] * 0.86]);
        assert!(
            lines
                .iter()
                .any(|(p, c)| near(p[0], rect.center()) && near(p[1], end) && c == colour),
            "axis {axis} to {end:?}"
        );
    }
    let points: Vec<(Pos2, f64)> = orbit
        .shape
        .iter()
        .map(|p| sphere_point(rect, *p, angle))
        .collect();
    let (mut n_near, mut n_far) = (0, 0);
    for w in points.windows(2) {
        let front = w[0].1 >= 0.0 && w[1].1 >= 0.0;
        let colour = if front {
            ink
        } else {
            ink.gamma_multiply(FAR_SIDE)
        };
        assert!(
            lines
                .iter()
                .any(|(p, c)| near(p[0], w[0].0) && near(p[1], w[1].0) && *c == colour),
            "the path's segment from {:?}, {}",
            w[0].0,
            if front { "near" } else { "far" }
        );
        if front {
            n_near += 1;
        } else {
            n_far += 1;
        }
    }
    assert!(n_near > 0 && n_far > 0, "near {n_near}, far {n_far}");
    let at = points[i].0;
    assert!(
        circles
            .iter()
            .any(|(c, r, f)| near(*c, at) && (*r - 3.5).abs() < 1e-6 && *f == AXIS_COLOURS[2]),
        "the playhead's dot at {at:?}"
    );
}

#[test]
fn mock_hover_drawn_sphere_and_real_space() {
    let angle = 37.0;
    let (mut app, mut h, layout) = hovering(angle);
    let orbit = app
        .snapshot()
        .sample
        .clone()
        .expect("a sample under the cursor");
    app.trajectory.orbit_t = orbit.path_t * 0.4;
    let drawn = shapes(&mut h, &mut app);
    let a = areas(layout.trajectory);
    let i = index_at(app.trajectory.orbit_t, orbit.path_t, orbit.shape.len());
    assert!(i > 0 && i < orbit.shape.len() - 1);
    let ink = h.ctx.global_style().visuals.text_color();
    check_sphere(&drawn, a.sphere.shrink(8.0), &orbit, angle, i, ink);
    rejects("the sphere at another angle", || {
        check_sphere(&drawn, a.sphere.shrink(8.0), &orbit, angle + 30.0, i, ink)
    });
    rejects("the playhead at the start", || {
        check_sphere(&drawn, a.sphere.shrink(8.0), &orbit, angle, 0, ink)
    });
    // Real space: each body's path fitted to the square, with its dot at the playhead.
    let rect = a.real.shrink(8.0);
    let [x0, y0, x1, y1] = fit(&orbit.bodies);
    let to = |p: &[f64; 2]| {
        on_square(
            rect,
            [
                2.0 * (p[0] - x0) / (x1 - x0) - 1.0,
                2.0 * (p[1] - y0) / (y1 - y0) - 1.0,
            ],
        )
    };
    let paths: Vec<(Vec<Pos2>, Color32)> = drawn
        .iter()
        .filter_map(|s| match s {
            Shape::Path(p) => match p.stroke.color {
                egui::epaint::ColorMode::Solid(c) => Some((p.points.clone(), c)),
                _ => None,
            },
            _ => None,
        })
        .collect();
    let circles = dots(&drawn);
    for (k, body) in orbit.bodies.iter().enumerate() {
        let want: Vec<Pos2> = body.iter().map(to).collect();
        let colour = match &paths
            .iter()
            .find(|(p, _)| p.len() == want.len() && p.iter().zip(&want).all(|(a, b)| near(*a, *b)))
        {
            Some((_, c)) => *c,
            None => panic!("body {k}'s path"),
        };
        assert_eq!(
            colour,
            BODY_COLOURS[k].gamma_multiply(0.8),
            "body {k}'s colour"
        );
        let at = to(&body[i]);
        assert!(
            circles
                .iter()
                .any(|(c, r, f)| near(*c, at) && (*r - 3.5).abs() < 1e-6 && *f == BODY_COLOURS[k]),
            "body {k}'s dot"
        );
    }
    // The captions and the stats.
    let words = texts(&drawn);
    let caption = words
        .iter()
        .find(|(t, _)| t == "real space · CoM frame")
        .expect("the caption");
    assert!(
        near(
            caption.1.left_bottom(),
            a.real.left_bottom() + vec2(6.0, -6.0)
        ),
        "{:?}",
        caption.1
    );
}

#[test]
fn mock_hover_drawn_unwrapped() {
    let (mut app, mut h, layout) = hovering(0.0);
    app.trajectory.unwrapped = true;
    let orbit = app.snapshot().sample.clone().unwrap();
    app.trajectory.orbit_t = orbit.path_t * 0.7;
    let drawn = shapes(&mut h, &mut app);
    let rect = areas(layout.trajectory).sphere.shrink(8.0);
    let ink = h.ctx.global_style().visuals.text_color();
    let lines = segments(&drawn);
    let mut skipped = 0;
    for w in orbit.shape.windows(2) {
        let (a, b) = (unwrapped_point(rect, w[0]), unwrapped_point(rect, w[1]));
        let found = lines
            .iter()
            .any(|(p, c)| near(p[0], a) && near(p[1], b) && *c == ink);
        assert_eq!(found, !wraps(w[0], w[1]), "the step from {:?}", w[0]);
        skipped += usize::from(!found);
    }
    assert!(skipped > 0, "the path wraps somewhere and is broken there");
    let i = index_at(app.trajectory.orbit_t, orbit.path_t, orbit.shape.len());
    let at = unwrapped_point(rect, orbit.shape[i]);
    assert!(
        dots(&drawn)
            .iter()
            .any(|(c, _, f)| near(*c, at) && *f == AXIS_COLOURS[2]),
        "the playhead's dot"
    );
    let words = texts(&drawn);
    let corner = |label: &str| {
        words
            .iter()
            .find(|(t, _)| t == label)
            .map(|(_, r)| *r)
            .unwrap()
    };
    assert!(near(corner("θ 0").left_bottom(), rect.left_bottom()));
    assert!(near(corner("2π").right_bottom(), rect.right_bottom()));
    assert!(near(corner("φ 0").left_top(), rect.left_top()));
    // No sphere axes in the unwrapped view.
    assert!(!lines.iter().any(|(_, c)| *c == AXIS_COLOURS[0]));
    rejects("a path drawn across the seam", || assert_eq!(skipped, 0));
}

#[test]
fn mock_hover_drawn_label() {
    let (mut app, mut h, layout) = hovering(0.0);
    let drawn = shapes(&mut h, &mut app);
    let sample = app.snapshot().sample.clone().unwrap();
    let want = hover_label::text(&sample, 0.0, app.snapshot().sim.horizon.t);
    let words = texts(&drawn);
    let (_, text_rect) = words.iter().find(|(t, _)| *t == want).expect("the label");
    let size = text_rect.size() + vec2(2.0 * PADDING, 2.0 * PADDING);
    let cursor = hover_point(layout.figure);
    let rect = placed(layout.figure, cursor, size);
    assert!(
        near(text_rect.min, rect.min + vec2(PADDING, PADDING)),
        "the text inside its padding"
    );
    let backing = drawn.iter().find_map(|s| match s {
        Shape::Rect(r) if r.fill == BACKING => Some(r.rect),
        _ => None,
    });
    let backing = backing.expect("the label's backing");
    assert!(
        near(backing.min, rect.min) && near(backing.max, rect.max),
        "{backing:?} against {rect:?}"
    );
    rejects("a backing elsewhere", || assert!(near(backing.min, cursor)));
}

#[test]
fn mock_hover_drawn_label_is_placed_by_its_offset() {
    let figure = Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 1000.0));
    let size = vec2(100.0, 40.0);
    let cursor = pos2(500.0, 500.0);
    let r = placed(figure, cursor, size);
    assert_eq!(r.min, cursor + hover_label::OFFSET - vec2(0.0, size.y));
    assert_eq!(r.size(), size);
    // Up and to the right of the cursor, clear of it.
    assert!(r.min.x > cursor.x && r.max.y < cursor.y, "{r:?}");
    rejects("a label over the cursor", || assert!(r.contains(cursor)));
    let left = placed(figure, pos2(-50.0, 500.0), size);
    assert_eq!(left.min.x, figure.min.x);
    let bottom = placed(figure, pos2(500.0, 2000.0), size);
    assert_eq!(bottom.max.y, figure.max.y);
}

// --- The Legend ------------------------------------------------------------------------------------------------------

/// Whether `p` is inside the triangle `a`, `b`, `c`, its edges included.
fn in_triangle(p: Pos2, a: Pos2, b: Pos2, c: Pos2) -> bool {
    let side = |u: Pos2, v: Pos2| (v.x - u.x) * (p.y - u.y) - (v.y - u.y) * (p.x - u.x);
    let (d1, d2, d3) = (side(a, b), side(b, c), side(c, a));
    let neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(neg && pos)
}

/// The colour `mesh` paints at `p`: that of its last triangle holding `p`, as the painter lays them in order; `None`
/// where none does. Each triangle is one colour, as every swatch mesh's rects are.
fn painted(mesh: &egui::Mesh, p: Pos2) -> Option<Color32> {
    mesh.indices.chunks(3).rev().find_map(|t| {
        let [a, b, c] = [t[0], t[1], t[2]].map(|k| mesh.vertices[k as usize]);
        in_triangle(p, a.pos, b.pos, c.pos).then_some(a.color)
    })
}

/// The mesh `drawn` holds over the swatch square `square`, from its corner: a patterned key's, the hatch's or the NaN /
/// Inf style's, as a mesh, not a flat rect.
fn hatch_drawn(drawn: &[Shape], square: Rect) -> egui::Mesh {
    drawn
        .iter()
        .find_map(|s| match s {
            Shape::Mesh(m)
                if (m.vertices[0].pos - square.min).length() < 1.0 / PIXELS_PER_POINT =>
            {
                let m: &egui::Mesh = m;
                Some(m.clone())
            }
            _ => None,
        })
        .expect("a patterned swatch, a mesh")
}

/// Checks that `mesh` draws the pattern `at` gives over `square` as the style (render_gui_spec §G6, "compared as the
/// style"; R-136), the hatch's ([`hatch_at`], `debug_invalid`'s) or the NaN / Inf style's ([`nan_inf_at`],
/// `debug_nan_inf`'s): each physical pixel of the square at [`PIXELS_PER_POINT`] painted, at its centre, the colour `at`
/// gives at its position `[x + 0.5, y + 0.5]`, and so the pattern's two colours, never one.
fn check_pattern(mesh: &egui::Mesh, square: Rect, at: fn([f64; 2]) -> [u8; 3]) {
    let px = |v: f32| (v * PIXELS_PER_POINT).round() as i64;
    let mut seen = std::collections::BTreeSet::new();
    for y in px(square.min.y)..px(square.max.y) {
        for x in px(square.min.x)..px(square.max.x) {
            let frag = [x as f64 + 0.5, y as f64 + 0.5];
            let centre = pos2(frag[0] as f32, frag[1] as f32) / PIXELS_PER_POINT;
            let want = colour(at(frag));
            assert_eq!(
                painted(mesh, centre),
                Some(want),
                "the pattern's pixel ({x}, {y})"
            );
            seen.insert(at(frag));
        }
    }
    assert_eq!(seen.len(), 2, "the pattern's two colours");
}

/// A pattern's colour at a pixel centre: the hatch's or the NaN / Inf style's.
type Pattern = fn([f64; 2]) -> [u8; 3];

/// Checks that `drawn`, whose texts are `words`, paints the entries `entries` of a categorical key in `area` as
/// `legend::show` lays them out: each swatch in its style (a flat colour, the stipple, the hatch or the NaN / Inf
/// style, each pattern as the style, pixel by pixel), its label beside it and its value, the share or the count and the
/// share, right, before the cell's end.
fn check_entries(drawn: &[Shape], words: &[(String, Rect)], area: Rect, entries: &[LegendEntry]) {
    for (i, entry) in entries.iter().enumerate() {
        let cell = entry_rect(area.width(), area.min + vec2(0.0, ROW_H), i);
        let square = Rect::from_center_size(
            pos2(cell.min.x + SWATCH / 2.0, cell.center().y),
            vec2(SWATCH, SWATCH),
        );
        match entry.swatch {
            SwatchStyle::Flat(rgb) => assert!(
                drawn.iter().any(|s| matches!(s, Shape::Rect(r) if near(r.rect.min, square.min) && near(r.rect.max, square.max) && r.fill == colour(rgb))),
                "{}'s swatch",
                entry.label
            ),
            SwatchStyle::NotYet => assert!(
                drawn.iter().any(|s| matches!(s, Shape::Mesh(m) if m.vertices.len() > 4 && (m.vertices[0].pos - square.min).length() < 1.0 / PIXELS_PER_POINT)),
                "running's stipple"
            ),
            SwatchStyle::Hatch | SwatchStyle::NanInf => {
                let (at, want): (Pattern, _) = match entry.swatch {
                    SwatchStyle::Hatch => (hatch_at, hatch_mesh(square, PIXELS_PER_POINT)),
                    _ => (nan_inf_at, nan_inf_mesh(square, PIXELS_PER_POINT)),
                };
                let mesh = hatch_drawn(drawn, square);
                assert!(
                    mesh.vertices.len() == want.vertices.len()
                        && mesh
                            .vertices
                            .iter()
                            .zip(&want.vertices)
                            .all(|(a, b)| near(a.pos, b.pos) && a.color == b.color),
                    "{}'s swatch is not its style's mesh",
                    entry.label
                );
                check_pattern(&mesh, square, at);
                let flat = colour(at([0.5, 0.5]));
                rejects("a patterned key drawn as a flat rect", || {
                    hatch_drawn(&[Shape::rect_filled(square, 0.0, flat)], square);
                });
                rejects("a patterned key drawn as a mesh of one colour", || {
                    let mut one = egui::Mesh::default();
                    one.add_colored_rect(square, flat);
                    check_pattern(&one, square, at);
                });
            }
        }
        // The label left in its cell, beside the swatch; the share right, before the cell's end.
        let label = words
            .iter()
            .find(|(t, r)| *t == entry.label && cell.contains(r.center()))
            .unwrap_or_else(|| panic!("{}'s label", entry.label));
        assert!(
            (label.1.min.x - (square.max.x + 6.0)).abs() < 0.5,
            "{} at {:?}",
            entry.label,
            label.1
        );
        assert!((label.1.center().y - cell.center().y).abs() < 1.5);
        let share = crate::explore::legend::value_text(entry);
        let value = words
            .iter()
            .find(|(t, r)| *t == share && cell.contains(r.center()))
            .unwrap_or_else(|| panic!("{}'s share", entry.label));
        assert!(
            (value.1.max.x - (cell.max.x - 8.0)).abs() < 0.5,
            "{share} at {:?}",
            value.1
        );
    }
}

#[test]
fn mock_legend_keys_drawn_swatches_bar_and_ticks() {
    let mut app = App::new(MockSide::new(MockEngine::frozen(), None), FORMAT);
    let mut h = headless();
    let layout = Layout::new(h.screen(), PIXELS_PER_POINT);
    let _ = h.frame(&mut app, Vec::new());
    let drawn = shapes(&mut h, &mut app);
    let legend = app.snapshot().legend.clone();
    let body = layout.legend.shrink(MARGIN);
    let rects = key_rects(&legend, body);
    let LegendKey::Categorical { entries, .. } = &legend.keys[0] else {
        panic!()
    };
    let words = texts(&drawn);
    check_entries(&drawn, &words, rects[0], entries);
    // The bar: its mesh across the key, and its ends and middle marked under it.
    let LegendKey::Ramp {
        samples, lo, hi, ..
    } = &legend.keys[1]
    else {
        panic!()
    };
    let area = rects[1];
    let bar = Rect::from_min_size(
        area.min + vec2(0.0, ROW_H),
        vec2(area.width(), crate::explore::legend::BAR_H),
    );
    let want = ramp_mesh(bar, samples);
    assert!(
        drawn.iter().any(|s| matches!(s, Shape::Mesh(m) if m.vertices.len() == want.vertices.len() && m.vertices.iter().zip(&want.vertices).all(|(a, b)| near(a.pos, b.pos) && a.color == b.color))),
        "the bar"
    );
    let y = bar.max.y + ROW_H / 2.0;
    let tick = |label: String| {
        words
            .iter()
            .find(|(t, _)| *t == label)
            .map(|(_, r)| *r)
            .unwrap_or_else(|| panic!("{label}"))
    };
    let low = tick(format!("{} · full", time::number(*lo)));
    assert!(near(low.left_center(), pos2(bar.min.x, y)), "{low:?}");
    let mid = tick(time::number((lo + hi) / 2.0));
    assert!(near(mid.center(), pos2(bar.center().x, y)), "{mid:?}");
    let high = tick(format!("{} · darkest", time::number(*hi)));
    assert!(near(high.right_center(), pos2(bar.max.x, y)), "{high:?}");
    rejects("the middle at the bar's end", || {
        assert!(near(mid.center(), pos2(bar.max.x, y)))
    });
}

// --- Time's ticks ----------------------------------------------------------------------------------------------------

#[test]
fn mock_time_drawn_ticks_under_the_scrubber() {
    let mut app = App::new(MockSide::new(MockEngine::frozen(), None), FORMAT);
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    let drawn = shapes(&mut h, &mut app);
    let rail = app.keyboard.place_of(SCRUBBER).unwrap().rect;
    // The rail spans the panel's body but for 70 points left for its readout.
    let body = Layout::new(h.screen(), PIXELS_PER_POINT)
        .time
        .shrink(time::MARGIN);
    assert!(
        (rail.width() - (body.width() - 70.0)).abs() < 0.5,
        "{rail:?} in {body:?}"
    );
    assert!((rail.left() - body.left()).abs() < 0.5);
    let t_max = app.snapshot().sim.horizon.t;
    let words = texts(&drawn);
    let y = rail.bottom() + 7.0;
    for (t, label) in time::ticks(t_max) {
        let x = rail.left() + rail.width() * (t / t_max) as f32;
        assert!(
            words
                .iter()
                .any(|(w, r)| *w == label && near(r.center(), pos2(x, y))),
            "the tick {label} at ({x}, {y})"
        );
    }
    rejects("ticks spaced by another horizon", || {
        let x = rail.left() + rail.width() * 0.5;
        assert!(words
            .iter()
            .any(|(w, r)| w == "50" && near(r.center(), pos2(x, y))))
    });
}

// --- The geometry's own numbers ---------------------------------------------------------------------------------------

#[test]
fn mock_hover_drawn_geometry_in_numbers() {
    use crate::explore::trajectory::{
        MARGIN, REST_ANGLE_DEG, STATS_H, SUMMARY_H, TITLE_H, VIEW_FRACTION, VIEW_GAP,
    };
    // The panel's regions, for a 400 × 1000 panel at (10, 20): the body 376 × 976 inside the margin, the two views side
    // by side, each the body's half less the gap, 182, under a quarter of its height, 244.
    let a = areas(Rect::from_min_size(pos2(10.0, 20.0), vec2(400.0, 1000.0)));
    let body = Rect::from_min_size(pos2(10.0 + MARGIN, 20.0 + MARGIN), vec2(376.0, 976.0));
    let side = (376.0 - VIEW_GAP) / 2.0;
    assert_eq!(side, 182.0);
    assert!(side < 976.0 * VIEW_FRACTION);
    assert_eq!(a.title, Rect::from_min_size(body.min, vec2(376.0, TITLE_H)));
    assert_eq!(
        a.summary,
        Rect::from_min_size(pos2(22.0, 32.0 + TITLE_H), vec2(376.0, SUMMARY_H))
    );
    // Real space and the sphere side by side (GUI_DESIGN_NOTES, render_gui_spec §G2), at the same height.
    assert_eq!(
        a.real,
        Rect::from_min_size(pos2(22.0, 32.0 + 46.0 + 4.0), vec2(side, side))
    );
    assert_eq!(
        a.sphere,
        Rect::from_min_size(pos2(22.0 + side + VIEW_GAP, 82.0), vec2(side, side))
    );
    assert_eq!(a.sphere.max.x, body.max.x, "the two fill the body's width");
    assert_eq!(
        a.stats,
        Rect::from_min_size(pos2(22.0, 82.0 + side + 6.0), vec2(376.0, STATS_H))
    );
    assert_eq!(
        a.rows,
        Rect::from_min_max(pos2(22.0, a.stats.max.y + 6.0), body.max)
    );
    // The stats' cells: the F₂ word two fifths of the row, the other three a fifth each, edge to edge.
    let cells = crate::explore::trajectory::stat_cells(a.stats);
    let fifth = 376.0 / 5.0;
    assert_eq!(
        cells[0],
        Rect::from_min_size(a.stats.min, vec2(2.0 * fifth, STATS_H))
    );
    for (k, cell) in cells.iter().enumerate().skip(1) {
        assert_eq!(
            *cell,
            Rect::from_min_size(
                a.stats.min + vec2((k as f32 + 1.0) * fifth, 0.0),
                vec2(fifth, STATS_H)
            )
        );
    }
    assert_eq!(cells[3].max.x, a.stats.max.x);
    rejects("four equal cells", || {
        assert_eq!(cells[0].width(), cells[1].width())
    });
    rejects("the views stacked", || {
        assert!(a.sphere.min.y >= a.real.max.y)
    });
    // A short panel: the views a quarter of its height, 194 of a 776 body, side by side still.
    let short = areas(Rect::from_min_size(pos2(10.0, 20.0), vec2(500.0, 800.0)));
    assert_eq!(short.real.height(), 776.0 * VIEW_FRACTION);
    assert_eq!(short.sphere.width(), 194.0);
    assert_eq!(
        short.sphere.min,
        short.real.right_top() + vec2(VIEW_GAP, 0.0)
    );
    rejects("views a third of the height", || {
        assert_eq!(short.real.height(), 776.0 / 3.0)
    });
    // The sphere seen from its tilt: the axes' screen points and depths, turned from its resting azimuth.
    let (st, ct) = 20f64.to_radians().sin_cos();
    let close = |(p, d): ([f64; 2], f64), (q, e): ([f64; 2], f64)| {
        (p[0] - q[0]).abs() < 1e-12 && (p[1] - q[1]).abs() < 1e-12 && (d - e).abs() < 1e-12
    };
    let r0 = -REST_ANGLE_DEG;
    assert!(close(view([1.0, 0.0, 0.0], r0), ([1.0, 0.0], 0.0)));
    assert!(close(view([0.0, 1.0, 0.0], r0), ([0.0, st], -ct)));
    assert!(close(view([0.0, 0.0, 1.0], r0), ([0.0, ct], st)));
    assert!(close(view([1.0, 0.0, 0.0], r0 + 90.0), ([0.0, st], -ct)));
    assert!(close(view([0.0, 1.0, 0.0], r0 + 90.0), ([-1.0, 0.0], 0.0)));
    rejects("an untilted view", || {
        assert!(close(view([0.0, 0.0, 1.0], r0), ([0.0, 1.0], 0.0)))
    });
    // At rest the sphere sits at its resting azimuth: x at (cos 35°, sin 35° sin 20°).
    let (sr, cr) = 35f64.to_radians().sin_cos();
    assert!(close(view([1.0, 0.0, 0.0], 0.0), ([cr, sr * st], -sr * ct)));
    rejects("a rest at azimuth 0", || {
        assert!(close(view([1.0, 0.0, 0.0], 0.0), ([1.0, 0.0], 0.0)))
    });
    // The square's points: the unit square on the rect's largest centred square, y up.
    let r = Rect::from_min_size(pos2(0.0, 0.0), vec2(200.0, 100.0));
    assert_eq!(on_square(r, [0.0, 0.0]), pos2(100.0, 50.0));
    assert_eq!(on_square(r, [1.0, 1.0]), pos2(150.0, 0.0));
    assert_eq!(on_square(r, [-1.0, -0.5]), pos2(50.0, 75.0));
    let tall = Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 300.0));
    assert_eq!(on_square(tall, [1.0, 0.0]), pos2(100.0, 150.0));
    // The unwrapped view's middle.
    assert_eq!(
        unwrapped_point(r, [std::f64::consts::PI, std::f64::consts::FRAC_PI_2]),
        pos2(100.0, 50.0)
    );
    // Fit: a tall set fits its height; the square is 1.1 of the larger side.
    let f = fit(&[vec![[0.0, 0.0], [1.0, 4.0]], vec![[0.5, 2.0]]]);
    assert!(
        (f[0] - (0.5 - 2.2)).abs() < 1e-12 && (f[1] - (2.0 - 2.2)).abs() < 1e-12,
        "{f:?}"
    );
    assert!(
        (f[2] - (0.5 + 2.2)).abs() < 1e-12 && (f[3] - (2.0 + 2.2)).abs() < 1e-12,
        "{f:?}"
    );
    assert_eq!(
        fit(&[vec![[3.0, 3.0]]]),
        [2.0, 2.0, 4.0, 4.0],
        "a single point gets the unit square about it"
    );
}

#[test]
fn mock_legend_keys_drawn_geometry_in_numbers() {
    use crate::explore::legend::{key_height, not_yet_mesh, BAR_H, KEY_GAP};
    let legend = crate::mock::legend::evaluate(&crate::mock::legend::default_stain());
    assert_eq!(key_height(&legend.keys[1]), ROW_H * 2.0 + BAR_H);
    let body = Rect::from_min_size(pos2(5.0, 10.0), vec2(300.0, 400.0));
    let rects = key_rects(&legend, body);
    assert_eq!(rects[0].min, pos2(5.0, 10.0 + ROW_H * 1.6));
    assert_eq!(rects[0].width(), 300.0);
    assert_eq!(rects[1].min.y, rects[0].max.y + KEY_GAP);
    rejects("keys with no gap", || {
        assert_eq!(rects[1].min.y, rects[0].max.y)
    });
    // The stipple's dots, each one physical pixel where the mirror dots.
    let ppp = 2.0;
    let rect = Rect::from_min_size(pos2(3.0, 4.0), vec2(6.0, 6.0));
    let mesh = not_yet_mesh(rect, ppp);
    assert_eq!(mesh.vertices[0].pos, pos2(3.0, 4.0));
    assert_eq!(mesh.vertices[3].pos, pos2(9.0, 10.0));
    let mut want = Vec::new();
    for y in 8..20 {
        for x in 6..18 {
            if engine::contract::legend::not_yet_dot_at([f64::from(x) + 0.5, f64::from(y) + 0.5]) {
                want.push(pos2(x as f32 / ppp, y as f32 / ppp));
            }
        }
    }
    let got: Vec<Pos2> = mesh.vertices[4..].chunks(4).map(|q| q[0].pos).collect();
    assert_eq!(got, want);
    let quad = &mesh.vertices[4..8];
    assert_eq!(
        quad[3].pos - quad[0].pos,
        vec2(1.0 / ppp, 1.0 / ppp),
        "one pixel"
    );
    // The bar's mesh: two vertices a sample, top and bottom, two triangles between each pair.
    let bar = Rect::from_min_size(pos2(0.0, 10.0), vec2(90.0, 6.0));
    let m = ramp_mesh(bar, &[[0; 3], [100; 3], [200; 3], [255; 3]]);
    let xs: Vec<f32> = m.vertices.iter().step_by(2).map(|v| v.pos.x).collect();
    assert_eq!(xs, [0.0, 30.0, 60.0, 90.0]);
    assert!(m.vertices.iter().step_by(2).all(|v| v.pos.y == 10.0));
    assert!(m
        .vertices
        .iter()
        .skip(1)
        .step_by(2)
        .all(|v| v.pos.y == 16.0));
    assert_eq!(&m.indices[..6], [0, 1, 2, 1, 3, 2]);
    assert_eq!(m.vertices[3].color, Color32::from_rgb(100, 100, 100));
    let one = ramp_mesh(bar, &[[7; 3]]);
    assert_eq!(one.vertices[0].pos, pos2(0.0, 10.0));
    assert!(one.indices.is_empty());
}

/// The texts `legend::show` paints for `legend` in `rect`, on a bare context, in the third frame, a scroll area having
/// settled.
fn legend_texts(legend: &engine::contract::legend::Legend, rect: Rect) -> Vec<(String, Rect)> {
    let ctx = egui::Context::default();
    let mut keyboard = crate::keyboard::Keyboard::new();
    let raw = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(2000.0, 2000.0))),
        ..Default::default()
    };
    let mut run = || {
        let mut out = ctx.run_ui(raw.clone(), |ui| {
            crate::explore::legend::show(ui, rect, legend, &mut keyboard);
        });
        out.textures_delta.clear();
        out
    };
    let _ = run();
    let _ = run();
    let out = run();
    let mut flat = Vec::new();
    fn flatten(shape: &Shape, out: &mut Vec<Shape>) {
        match shape {
            Shape::Vec(v) => v.iter().for_each(|s| flatten(s, out)),
            other => out.push(other.clone()),
        }
    }
    for clipped in &out.shapes {
        flatten(&clipped.shape, &mut flat);
    }
    texts(&flat)
}

#[test]
fn mock_legend_keys_drawn_long_labels_are_cut_before_the_share() {
    use engine::contract::legend::Legend;
    let long = "a class whose label runs on far past its cell".to_owned();
    let legend = Legend {
        keys: vec![LegendKey::Categorical {
            title: "colour · something".into(),
            entries: vec![LegendEntry {
                label: long.clone(),
                swatch: SwatchStyle::Flat([10, 20, 30]),
                share: 12.3,
                count: None,
            }],
        }],
    };
    let rect = Rect::from_min_size(pos2(100.0, 100.0), vec2(324.0, 200.0));
    let words = legend_texts(&legend, rect);
    let area = key_rects(&legend, rect.shrink(MARGIN))[0];
    let cell = entry_rect(area.width(), area.min + vec2(0.0, ROW_H), 0);
    let label = words
        .iter()
        .find(|(t, _)| t.starts_with("a class"))
        .expect("the label");
    assert!(
        label.1.max.x <= cell.max.x - 44.0 + 0.5,
        "{:?} before the share's room",
        label.1
    );
    assert!((label.1.min.x - (cell.min.x + SWATCH + 6.0)).abs() < 0.5);
    let share = words.iter().find(|(t, _)| t == "12.3").expect("the share");
    assert!(
        (share.1.max.x - (cell.max.x - 8.0)).abs() < 0.5,
        "{:?}",
        share.1
    );
    rejects("a label running into the share", || {
        assert!(label.1.max.x <= cell.max.x - 50.0, "{:?}", label.1)
    });
    assert_eq!(
        label.0, long,
        "the galley holds the whole text, elided where it is cut"
    );
}

/// Keys taller than the panel scroll, held at the bottom: the last key's last entry is drawn inside the body, and the
/// last key above where `key_rects` puts it unscrolled.
fn check_scrolled_to_bottom(legend: &engine::contract::legend::Legend, rect: Rect) {
    use crate::explore::legend::overflows;
    let body = rect.shrink(MARGIN);
    assert!(overflows(legend, body), "the keys fit");
    let words = legend_texts(legend, rect);
    let last = words
        .iter()
        .find(|(t, _)| t == "NaN / Inf")
        .unwrap_or_else(|| panic!("the last entry drawn: {words:?}"));
    assert!(
        last.1.max.y <= body.max.y + 0.5,
        "{:?} below {body:?}",
        last.1
    );
    // The "Debug" key's title is drawn above where the keys, unscrolled, would put it.
    let title = words
        .iter()
        .find(|(t, _)| t == "Debug")
        .expect("the Debug key's title");
    let unscrolled = *key_rects(legend, body).last().unwrap();
    assert!(
        title.1.min.y < unscrolled.min.y - 1.0,
        "{:?} not scrolled from {unscrolled:?}",
        title.1
    );
}

#[test]
fn mock_legend_keys_drawn_scroll_past_the_panel() {
    use crate::explore::legend::{key_rects as rects, overflows};
    use crate::mock::invalid::Flagged;
    use crate::mock::legend::{debug_key, default_stain, evaluate};
    let mut legend = evaluate(&default_stain());
    legend.keys.push(debug_key(&Flagged::default()));
    let panel = Layout::new(
        Rect::from_min_size(pos2(0.0, 0.0), vec2(1440.0, 900.0)),
        PIXELS_PER_POINT,
    )
    .legend;
    check_scrolled_to_bottom(&legend, panel);
    // At the edge: a body whose bottom is the last key's fits, one a point shorter overflows.
    let body = panel.shrink(MARGIN);
    let bottom = rects(&legend, body).last().unwrap().max.y;
    let exact = Rect::from_min_max(body.min, pos2(body.max.x, bottom));
    assert!(!overflows(&legend, exact));
    assert!(overflows(
        &legend,
        Rect::from_min_max(body.min, pos2(body.max.x, bottom - 1.0))
    ));
    assert!(!overflows(
        &engine::contract::legend::Legend::default(),
        body
    ));
    // Control: a panel tall enough for every key draws them unscrolled, and fails the check.
    let tall = Rect::from_min_size(panel.min, vec2(panel.width(), 600.0));
    rejects("keys that fit, drawn unscrolled", || {
        check_scrolled_to_bottom(&legend, tall)
    });
}
