//! What the compass's tests share (TASK-M6-33): views and planes in the mock preset's frame, the app set to a plane
//! and left to come to rest, drags on the cube, the `SetField`s the mock logged, and the check that the compass's
//! paint follows its [`Drawing`].

use eframe::egui::{self, Modifiers, Pos2, Rect, Shape};
use engine::contract::sim_config::{Latent, Plane};

use super::support::{headless, mock_app, shapes};
use crate::app::App;
use crate::explore::compass::{
    self, edge_colour, label_colour, plane_colour, Drawing, Item, Label, CORNERS, EDGES,
    GRID_WIDTH, LABEL_SIZE, NUMBER_SIZE, OUTLINE_WIDTH, P3,
};
use crate::explore::manifold_view::{basis_edit, z0_edit, View, PRESETS, SLICE_AXIS};
use crate::headless::Headless;
use crate::mock::MOCK_Z0;
use crate::side::MockSide;

/// The mock preset's frame `(e_a, e_b, e_d)`.
pub const FRAME: (usize, usize, usize) = (PRESETS[0].0, PRESETS[0].1, SLICE_AXIS);

/// The view `(τ₁, τ₂, γ)` in degrees at scale `σ`, in the mock preset's frame.
pub fn view(tau1: f64, tau2: f64, gamma: f64, sigma: f64) -> View {
    View {
        preset: Some(0),
        a: FRAME.0,
        b: FRAME.1,
        d: FRAME.2,
        scale: sigma,
        tau1: tau1.to_radians(),
        tau2: tau2.to_radians(),
        gamma: gamma.to_radians(),
    }
}

/// The latent point with components `p` on the mock preset's frame, the mock's centre elsewhere.
pub fn z0_at(p: P3) -> Latent {
    let mut z0 = MOCK_Z0;
    z0[FRAME.0] = p[0];
    z0[FRAME.1] = p[1];
    z0[FRAME.2] = p[2];
    z0
}

/// `View::basis`'s plane of `view` through the point `p` of the frame.
pub fn plane(view: &View, p: P3) -> Plane {
    let (q1, q2) = view.basis();
    Plane {
        z0: z0_at(p),
        q1,
        q2,
    }
}

/// Runs frames until the compass is at rest, at most 400.
pub fn settle(h: &mut Headless, app: &mut App<MockSide>) {
    for _ in 0..400 {
        let _ = h.frame(app, Vec::new());
        if !app.manifold.compass.moving {
            return;
        }
    }
    panic!("the compass did not come to rest");
}

/// The app on the mock with its plane set to `p`, through the contract, at rest.
pub fn app_at(p: &Plane) -> (App<MockSide>, Headless) {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    app.set_field(z0_edit(p.z0));
    app.set_field(basis_edit((p.q1, p.q2)));
    let _ = h.frame(&mut app, Vec::new());
    settle(&mut h, &mut app);
    (app, h)
}

/// The compass's rect in the app on `h`.
pub fn compass_rect(h: &Headless) -> Rect {
    crate::layout::Layout::new(h.screen(), crate::capture::PIXELS_PER_POINT).compass
}

/// The cube's square in the app on `h`.
pub fn square(h: &Headless) -> Rect {
    compass::square(compass_rect(h))
}

/// The compass's drawing in the app as it stands.
pub fn drawing(h: &Headless, app: &App<MockSide>) -> Drawing {
    compass::current(compass_rect(h), &app.snapshot().sim, &app.manifold.compass)
}

fn button(pos: Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

/// A drag from `from` by `delta`, in four moves, then a frame.
pub fn drag(h: &mut Headless, app: &mut App<MockSide>, from: Pos2, delta: egui::Vec2) {
    let mut frames = vec![
        vec![egui::Event::PointerMoved(from)],
        vec![button(from, true)],
    ];
    for i in 1..=4 {
        frames.push(vec![egui::Event::PointerMoved(
            from + delta * (i as f32 / 4.0),
        )]);
    }
    frames.push(vec![button(from + delta, false)]);
    frames.push(Vec::new());
    for events in frames {
        let _ = h.frame(app, events);
    }
}

/// The `SetField`s the mock logged since row `from` of the console, by their paths.
pub fn setfields(app: &App<MockSide>, from: usize) -> Vec<&'static str> {
    app.console()[from..]
        .iter()
        .filter_map(|e| {
            let m = e.message.as_str();
            if m.starts_with("SetField Plane.z0 ") {
                Some("z0")
            } else if m.starts_with("SetField Plane.q1 Plane.q2 ") {
                Some("basis")
            } else if m.starts_with("SetField ") {
                Some("other")
            } else {
                None
            }
        })
        .collect()
}

/// The point of the cube's square the drawn slice's polygon centres on, or its centre with no slice.
pub fn on_slice(drawing: &Drawing) -> Pos2 {
    let points = drawing
        .slice
        .as_ref()
        .map(|s| s.points.clone())
        .unwrap_or_default();
    if points.is_empty() {
        return drawing.iso.at([0.0; 3]);
    }
    let n = points.len() as f64;
    drawing.iso.at(std::array::from_fn(|k| {
        points.iter().map(|p| p[k]).sum::<f64>() / n
    }))
}

/// A point of `square` off the cube: near its top left corner.
pub fn off_cube(square: Rect) -> Pos2 {
    square.min + egui::vec2(4.0, 4.0)
}

/// The cube's edges painted in `shapes` in the order painted: each a segment between two corners' projections,
/// matched to [`EDGES`] by both ends (the corners `(1, 1, 1)` and `(−1, −1, −1)` share the centre), with its stroke.
pub fn painted_edges(shapes: &[Shape], drawing: &Drawing) -> Vec<([P3; 2], egui::Stroke)> {
    let close = |a: Pos2, b: Pos2| (a - b).length() < 1e-3;
    let outline = egui::Stroke::new(
        OUTLINE_WIDTH,
        plane_colour(&egui::Visuals::dark()).gamma_multiply(compass::OUTLINE_OPACITY),
    );
    shapes
        .iter()
        .filter_map(|s| match s {
            // A contact on an edge is drawn there as the plane's outline, not as the edge.
            Shape::LineSegment { points, stroke } if *stroke != outline => {
                EDGES.iter().find_map(|&(i, j)| {
                    let (a, b) = (CORNERS[i], CORNERS[j]);
                    (close(points[0], drawing.iso.at(a)) && close(points[1], drawing.iso.at(b)))
                        .then_some(([a, b], *stroke))
                })
            }
            _ => None,
        })
        .collect()
}

/// The filled polygons painted in `shapes`.
pub fn fills(shapes: &[Shape]) -> Vec<&egui::epaint::PathShape> {
    shapes
        .iter()
        .filter_map(|s| match s {
            Shape::Path(p) if p.closed && p.fill != egui::Color32::TRANSPARENT => Some(p),
            _ => None,
        })
        .collect()
}

/// The closed outlines painted in `shapes`: closed paths with no fill and the outline's width.
pub fn outlines(shapes: &[Shape]) -> Vec<&egui::epaint::PathShape> {
    shapes
        .iter()
        .filter_map(|s| match s {
            Shape::Path(p)
                if p.closed
                    && p.fill == egui::Color32::TRANSPARENT
                    && p.stroke.width == OUTLINE_WIDTH =>
            {
                Some(p)
            }
            _ => None,
        })
        .collect()
}

/// The texts painted in `shapes`, with their bounds.
pub fn texts(shapes: &[Shape]) -> Vec<(String, Rect, f32, egui::Color32)> {
    shapes
        .iter()
        .filter_map(|s| match s {
            // The galley's laid-out rect, which the painter's anchor places, not the glyphs' ink.
            Shape::Text(t) => Some((
                t.galley.text().to_owned(),
                t.galley.rect.translate(t.pos.to_vec2()),
                t.galley
                    .job
                    .sections
                    .first()
                    .map_or(0.0, |s| s.format.font_id.size),
                t.fallback_color,
            )),
            _ => None,
        })
        .collect()
}

/// Asserts that the compass's `shapes`, painted in the dark theme, follow `drawing`: the edges in its order with their
/// strokes, the slice at its place among them as a polygon filled, its grid and its outline, or an edge or a corner
/// contact as its outline alone, the labels and numbers centred where it puts them, and the pin.
pub fn check_painted(shapes: &[Shape], drawing: &Drawing) {
    let visuals = egui::Visuals::dark();
    let close = |a: Pos2, b: Pos2| (a - b).length() < 1e-3;
    let painted = painted_edges(shapes, drawing);
    let want: Vec<(P3, P3, f64, f64)> = drawing
        .items
        .iter()
        .filter_map(|i| match i {
            Item::Edge {
                from,
                to,
                opacity,
                width,
            } => Some((*from, *to, *opacity, *width)),
            Item::Slice => None,
        })
        .collect();
    assert_eq!(painted.len(), 12, "twelve edges painted: {painted:?}");
    for ((edge, stroke), (from, to, opacity, width)) in painted.iter().zip(&want) {
        assert_eq!(*edge, [*from, *to], "the edges' order");
        assert_eq!(stroke.width, *width as f32, "{edge:?}'s width");
        assert_eq!(
            stroke.color,
            edge_colour(&visuals).gamma_multiply(*opacity as f32),
            "{edge:?}'s colour"
        );
    }
    let fills = fills(shapes);
    let outlines = outlines(shapes);
    match &drawing.slice {
        None => {
            assert!(fills.is_empty(), "no slice, no fill: {fills:?}");
            assert!(outlines.is_empty(), "no slice, no outline");
            assert!(
                !drawing.items.contains(&Item::Slice),
                "no slice in the order"
            );
        }
        Some(slice) => {
            let screen: Vec<Pos2> = slice.points.iter().map(|p| drawing.iso.at(*p)).collect();
            let grid_colour = plane_colour(&visuals).gamma_multiply(compass::GRID_OPACITY);
            let grid = shapes
                .iter()
                .filter(|s| {
                    matches!(s, Shape::LineSegment { stroke, .. }
                    if stroke.width == GRID_WIDTH && stroke.color == grid_colour)
                })
                .count();
            if screen.len() >= 3 {
                assert_eq!(fills.len(), 1, "one fill");
                assert!(
                    fills[0].points.len() == screen.len()
                        && fills[0]
                            .points
                            .iter()
                            .zip(&screen)
                            .all(|(a, b)| close(*a, *b)),
                    "the fill at {screen:?}: {:?}",
                    fills[0].points
                );
                assert_eq!(outlines.len(), 1, "one outline");
                assert_eq!(grid, slice.grid.len(), "the grid's lines");
                // The fill is painted at the slice's place among the edges.
                let at = drawing
                    .items
                    .iter()
                    .position(|i| *i == Item::Slice)
                    .expect("the slice in the order");
                let fill_index = shapes
                    .iter()
                    .position(|s| matches!(s, Shape::Path(p) if p.closed && p.fill != egui::Color32::TRANSPARENT))
                    .expect("a fill");
                let before = painted_edges(&shapes[..fill_index], drawing).len();
                assert_eq!(before, at, "the slice drawn after {at} edges");
            } else {
                assert!(fills.is_empty(), "a contact has no fill");
                assert_eq!(grid, 0, "a contact has no grid");
                let outline_colour =
                    plane_colour(&visuals).gamma_multiply(compass::OUTLINE_OPACITY);
                let drawn = shapes.iter().any(|s| match s {
                    Shape::LineSegment { points, stroke } => {
                        screen.len() == 2
                            && stroke.width == OUTLINE_WIDTH
                            && stroke.color == outline_colour
                            && close(points[0], screen[0])
                            && close(points[1], screen[1])
                    }
                    Shape::Circle(c) => {
                        screen.len() == 1 && close(c.center, screen[0]) && c.fill == outline_colour
                    }
                    _ => false,
                });
                assert!(drawn, "the contact {:?} outlined", slice.points);
            }
        }
    }
    let painted_texts = texts(shapes);
    for (labels, size) in [
        (&drawing.labels, LABEL_SIZE),
        (&drawing.numbers, NUMBER_SIZE),
    ] {
        for Label { text, at, accent } in labels {
            let colour = if size == LABEL_SIZE {
                label_colour(&visuals, *accent)
            } else {
                visuals.weak_text_color()
            };
            assert!(
                painted_texts.iter().any(|(t, r, s, c)| t == text
                    && (r.center() - *at).length() < 1.0
                    && *s == size
                    && *c == colour),
                "{text:?} at {at:?}: {painted_texts:?}"
            );
        }
    }
    let gold = shapes.iter().any(|s| {
        matches!(s, Shape::Circle(c) if c.fill == crate::explore::lock::GOLD && c.radius == 3.0
            && drawing.pin.is_some_and(|p| close(c.center, drawing.iso.at(p))))
    });
    assert_eq!(gold, drawing.pin.is_some(), "the pin while locked");
}

/// The shapes of one more frame of `app` in the compass's rect.
pub fn compass_shapes(h: &mut Headless, app: &mut App<MockSide>) -> Vec<Shape> {
    let rect = compass_rect(h);
    shapes(h, app)
        .into_iter()
        .filter(|s| rect.expand(1.0).contains_rect(s.visual_bounding_rect()))
        .collect()
}
