//! `compass_plucker` (REQ-GUI-195; render_gui_spec § "The compass: look and views (R-451, R-452)"; R-452): the Plücker
//! view's grid is the slice plane's Plücker coordinates from the basis the decoder uses, orthonormalised, never from the
//! tilt angles; `Σ p² = 1` and the grid is unchanged by a rotation within the plane; an axis-aligned plane lights one
//! cell at ±1; the dominant cell is outlined above 0.02 and the readout names it; a cell's value shows on hover; the
//! upper triangle is drawn in two hues by sign, intensity `|p|`, the names on both edges. Each check also runs on an
//! input it must reject (R-176, through `rejects`).

use eframe::egui::{self, Shape};
use engine::contract::sim_config::{Latent, Plane, LATENT_DIM};

use super::compass_support::{app_at, compass_rect, plane, square, view};
use super::support::{click_scope, rejects, shapes};
use crate::app::App;
use crate::explore::compass::views::{
    self, cell_background, cell_colour, dominant, hover_text, plucker, plucker_drawing, readout,
    sum_sq, upper, CompassView, Grid, GridPlace, COLUMN_NAMES_H, NEGATIVE, OUTLINE_MIN, POSITIVE,
};
use crate::explore::manifold_view::{dot, norm, View, AXES};
use crate::explore::COMPASS_PLUCKER;
use crate::headless::Headless;
use crate::side::MockSide;

/// A small deterministic generator: splitmix64, its values in `[−1, 1)`.
pub struct Rng(u64);

impl Rng {
    /// A generator seeded with `seed`.
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// The next value in `[−1, 1)`.
    pub fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 11) as f64 / (1u64 << 52) as f64 - 1.0
    }

    /// A latent vector of values in `[−1, 1)`.
    pub fn latent(&mut self) -> Latent {
        std::array::from_fn(|_| self.next())
    }
}

/// A random plane: random `q₁`, `q₂`, neither unit nor orthogonal, scaled apart.
pub fn random_plane(rng: &mut Rng) -> Plane {
    let (a, b) = (0.2 + rng.next().abs() * 3.0, 0.05 + rng.next().abs());
    Plane {
        z0: rng.latent(),
        q1: rng.latent().map(|x| x * a),
        q2: rng.latent().map(|x| x * b),
    }
}

/// The plane spanned by `e_i` and `e_j`, `σ` long.
fn axis_plane(i: usize, j: usize, sigma: f64) -> Plane {
    let e = |k: usize| -> Latent { std::array::from_fn(|n| if n == k { sigma } else { 0.0 }) };
    Plane {
        z0: [0.1; LATENT_DIM],
        q1: e(i),
        q2: e(j),
    }
}

/// Asserts `Σ p² = 1` within 1e-12.
fn check_unit(grid: &Grid) {
    let s = sum_sq(grid);
    assert!((s - 1.0).abs() <= 1e-12, "Σ p² = {s}");
}

/// Asserts two grids equal within 1e-12, cell by cell.
fn check_same(a: &Grid, b: &Grid, what: &str) {
    for (i, j) in upper() {
        assert!(
            (a[i][j] - b[i][j]).abs() <= 1e-12,
            "{what}: cell ({i}, {j}) {} against {}",
            a[i][j],
            b[i][j]
        );
    }
}

/// The grid of `p_ij = q₁ᵢ q₂ⱼ − q₁ⱼ q₂ᵢ` of the basis as it stands, not orthonormalised: the control's.
fn raw_grid(plane: &Plane) -> Grid {
    let mut grid = [[0.0; LATENT_DIM]; LATENT_DIM];
    for (i, j) in upper() {
        grid[i][j] = plane.q1[i] * plane.q2[j] - plane.q1[j] * plane.q2[i];
    }
    grid
}

#[test]
fn compass_plucker_the_sum_of_squares_is_one() {
    let mut rng = Rng::new(0x5EED_0195);
    for _ in 0..500 {
        let p = random_plane(&mut rng);
        check_unit(&plucker(&p));
        // The orthonormalised basis is orthonormal.
        let (e1, e2) = views::orthonormal(&p).expect("a plane");
        assert!((norm(&e1) - 1.0).abs() < 1e-14 && (norm(&e2) - 1.0).abs() < 1e-14);
        assert!(dot(&e1, &e2).abs() < 1e-14);
    }
    // Tilted, rotated and scaled views of the mock preset's frame.
    for (t1, t2, g, s) in [
        (30.0, -20.0, 25.0, 0.5),
        (60.0, 45.0, -170.0, 3.0),
        (-89.0, 89.0, 90.0, 1e-3),
    ] {
        check_unit(&plucker(&plane(&view(t1, t2, g, s), [0.0; 3])));
    }
    let skew = random_plane(&mut Rng::new(7));
    rejects("a grid from the basis un-orthonormalised", || {
        check_unit(&raw_grid(&skew))
    });
}

#[test]
fn compass_plucker_an_axis_aligned_plane_lights_one_cell() {
    for (i, j) in upper() {
        for (sigma, swap) in [(1.0, false), (0.25, false), (2.0, true)] {
            let p = if swap {
                axis_plane(j, i, sigma)
            } else {
                axis_plane(i, j, sigma)
            };
            let grid = plucker(&p);
            let want = if swap { -1.0 } else { 1.0 };
            for (a, b) in upper() {
                let v = if (a, b) == (i, j) { want } else { 0.0 };
                assert!(
                    (grid[a][b] - v).abs() <= 1e-12,
                    "z_{i} × z_{j}: cell ({a}, {b}) {} for {v}",
                    grid[a][b]
                );
            }
            assert_eq!(dominant(&grid), Some((i, j)));
        }
    }
    let grid = plucker(&axis_plane(2, 5, 1.0));
    rejects("another cell lit for z_q0 × z_q3", || {
        assert!((grid[2][4] - 1.0).abs() <= 1e-12)
    });
}

/// `plane`'s basis turned by `θ` within the plane, `q₁` and `q₂` scaled by `a` and `b`.
fn turned(plane: &Plane, theta: f64, a: f64, b: f64) -> Plane {
    let (s, c) = theta.sin_cos();
    Plane {
        z0: plane.z0,
        q1: std::array::from_fn(|k| a * (c * plane.q1[k] + s * plane.q2[k])),
        q2: std::array::from_fn(|k| b * (-s * plane.q1[k] + c * plane.q2[k])),
    }
}

#[test]
fn compass_plucker_a_rotation_within_the_plane_leaves_every_cell() {
    let mut rng = Rng::new(42);
    for _ in 0..200 {
        let p = random_plane(&mut rng);
        let grid = plucker(&p);
        for theta in [0.3, -1.2, 2.9, std::f64::consts::PI / 2.0] {
            let q = turned(&p, theta, 0.7, 1.9);
            check_same(&grid, &plucker(&q), "turned within the plane");
        }
    }
    // A plane whose basis no preset's frame holds: its tilt angles, read back, are another plane.
    let p = random_plane(&mut Rng::new(3));
    let from_angles = {
        let (q1, q2) = View::of(&p).basis();
        plucker(&Plane { z0: p.z0, q1, q2 })
    };
    rejects("a grid from the tilt angles", || {
        check_same(&plucker(&p), &from_angles, "from the angles")
    });
}

#[test]
fn compass_plucker_the_dominant_cell_and_the_readout_name_the_largest() {
    let mut rng = Rng::new(195);
    for _ in 0..200 {
        let grid = plucker(&random_plane(&mut rng));
        let (bi, bj) = upper()
            .reduce(|best, c| {
                if grid[c.0][c.1].abs() > grid[best.0][best.1].abs() {
                    c
                } else {
                    best
                }
            })
            .expect("28 cells");
        assert_eq!(dominant(&grid), Some((bi, bj)));
        let text = readout(&grid);
        let want = format!(
            "strongest: {} × {} {} · Σ p² 1.000",
            AXES[bi],
            AXES[bj],
            views::value(grid[bi][bj])
        );
        assert_eq!(text, want);
    }
    // The threshold: outlined only above 0.02, the readout "—" at or below it.
    let mut grid: Grid = [[0.0; LATENT_DIM]; LATENT_DIM];
    grid[3][6] = -OUTLINE_MIN;
    assert_eq!(dominant(&grid), None, "at 0.02");
    assert_eq!(readout(&grid), "strongest: — · Σ p² 0.000");
    grid[3][6] = -(OUTLINE_MIN + 1e-9);
    assert_eq!(dominant(&grid), Some((3, 6)), "just above 0.02");
    assert_eq!(readout(&grid), "strongest: z_q1 × z_μ1 -0.020 · Σ p² 0.000");
    // Of equals, the first row by row; a larger later cell wins.
    let mut tie: Grid = [[0.0; LATENT_DIM]; LATENT_DIM];
    tie[1][4] = 0.5;
    tie[2][3] = -0.5;
    assert_eq!(dominant(&tie), Some((1, 4)));
    tie[2][3] = -0.5000001;
    assert_eq!(dominant(&tie), Some((2, 3)));
    tie[0][1] = 0.6;
    assert_eq!(dominant(&tie), Some((0, 1)));
    // A basis spanning no plane draws no cell.
    let flat = Plane {
        z0: [0.0; LATENT_DIM],
        q1: [1.0; LATENT_DIM],
        q2: [2.0; LATENT_DIM],
    };
    assert_eq!(plucker(&flat), [[0.0; LATENT_DIM]; LATENT_DIM]);
    let p = plucker(&random_plane(&mut Rng::new(9)));
    rejects("a readout naming another cell", || {
        let (i, j) = dominant(&p).expect("one");
        let other = if (i, j) == (0, 1) { (0, 2) } else { (0, 1) };
        assert_eq!(dominant(&p), Some(other));
    });
}

/// The app at `p`, at rest, switched to the Plücker view by its button.
pub fn plucker_app(p: &Plane) -> (App<MockSide>, Headless) {
    let (mut app, mut h) = app_at(p);
    click_scope(&mut h, &mut app, COMPASS_PLUCKER);
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(app.manifold.compass.view, CompassView::Plucker);
    (app, h)
}

/// The texts painted in a frame.
fn painted_texts(shapes: &[Shape]) -> Vec<String> {
    shapes
        .iter()
        .filter_map(|s| match s {
            Shape::Text(t) => Some(t.galley.text().to_owned()),
            _ => None,
        })
        .collect()
}

/// The texts of `frames` more frames with the pointer still at `at`.
fn hovered(
    h: &mut Headless,
    app: &mut App<MockSide>,
    at: egui::Pos2,
    frames: usize,
) -> Vec<String> {
    let _ = h.frame(app, vec![egui::Event::PointerMoved(at)]);
    let mut seen = Vec::new();
    for _ in 0..frames {
        seen = painted_texts(&shapes(h, app));
    }
    seen
}

#[test]
fn compass_plucker_hover_shows_a_cells_names_and_value() {
    let p = plane(&view(30.0, -20.0, 25.0, 1.0), [0.1, 0.2, 0.3]);
    let (mut app, mut h) = plucker_app(&p);
    let grid = plucker(&app.snapshot().sim.plane);
    let place = GridPlace::fit(square(&h));
    for (i, j) in [(0, 1), (0, 5), (1, 5), (3, 7)] {
        let seen = hovered(&mut h, &mut app, place.cell_rect(i, j).center(), 8);
        let want = hover_text(&grid, i, j);
        assert!(
            seen.contains(&want),
            "hovering ({i}, {j}): {want:?} not in {seen:?}"
        );
        assert_eq!(
            want,
            format!("{} × {}  {}", AXES[i], AXES[j], views::value(grid[i][j]))
        );
        let other = hover_text(&grid, 6, 7);
        rejects("another cell's value shown on hover", || {
            assert!(seen.contains(&other))
        });
    }
    // The tooltip sits beside the compass, never over the figure.
    let figure = crate::layout::Layout::new(h.screen(), crate::capture::PIXELS_PER_POINT).figure;
    let shown = shapes(&mut h, &mut app);
    let tip = shown
        .iter()
        .find(|s| matches!(s, Shape::Text(t) if t.galley.text() == hover_text(&grid, 3, 7)))
        .expect("the tooltip");
    assert!(!tip.visual_bounding_rect().intersects(figure));
}

#[test]
fn compass_plucker_draws_the_upper_triangle_in_two_hues() {
    let visuals = egui::Visuals::dark();
    assert_eq!(cell_colour(&visuals, 0.0), cell_background(&visuals));
    assert_eq!(cell_colour(&visuals, -0.0), cell_background(&visuals));
    assert_eq!(cell_colour(&visuals, 1.0), POSITIVE);
    assert_eq!(cell_colour(&visuals, -1.0), NEGATIVE);
    assert_eq!(
        cell_colour(&visuals, 0.5),
        views::mix(cell_background(&visuals), POSITIVE, 0.5)
    );
    assert_eq!(
        cell_colour(&visuals, -0.25),
        views::mix(cell_background(&visuals), NEGATIVE, 0.25)
    );
    assert_ne!(POSITIVE, NEGATIVE);
    let p = plane(&view(30.0, -20.0, 25.0, 1.0), [0.1, 0.2, 0.3]);
    let (mut app, mut h) = plucker_app(&p);
    let drawing = plucker_drawing(square(&h), &app.snapshot().sim.plane, None);
    let shown = shapes(&mut h, &mut app);
    let rect = compass_rect(&h);
    let filled: Vec<(egui::Rect, egui::Color32)> = shown
        .iter()
        .filter_map(|s| match s {
            Shape::Rect(r) if rect.contains_rect(r.rect) => Some((r.rect, r.fill)),
            _ => None,
        })
        .collect();
    check_cells(&filled, &drawing);
    // No number in a cell: no text inside the grid.
    let grid_rect = drawing.place.rect();
    assert!(!shown.iter().any(
        |s| matches!(s, Shape::Text(t) if grid_rect.contains(t.visual_bounding_rect().center()))
    ));
    // The names on both edges: each axis left of the grid and above it.
    let names: Vec<(String, egui::Rect)> = shown
        .iter()
        .filter_map(|s| match s {
            Shape::Text(t) if rect.contains_rect(s.visual_bounding_rect()) => {
                Some((t.galley.text().to_owned(), s.visual_bounding_rect()))
            }
            _ => None,
        })
        .collect();
    for (k, name) in AXES.iter().enumerate() {
        let row = drawing.place.cell_rect(k, 0);
        let column = drawing.place.cell_rect(0, k);
        assert!(
            names.iter().any(|(t, r)| t == name
                && r.max.x <= grid_rect.min.x
                && (r.center().y - row.center().y).abs() < drawing.place.cell / 2.0),
            "{name} left of row {k}"
        );
        assert!(
            names.iter().any(|(t, r)| t == name
                && r.max.y <= grid_rect.min.y
                && (r.center().x - column.center().x).abs() < drawing.place.cell / 2.0),
            "{name} above column {k}"
        );
    }
    // The dominant cell outlined.
    let (i, j) = drawing.outline.expect("a dominant cell");
    let outline = drawing.place.cell_rect(i, j);
    assert!(shown.iter().any(|s| matches!(s, Shape::Rect(r)
        if r.rect == outline && r.stroke.width == views::OUTLINE_W
            && r.stroke.color == visuals.strong_text_color())));
    let mut flipped = drawing.clone();
    flipped.grid[i][j] = -flipped.grid[i][j];
    rejects("a cell drawn in the other sign's hue", || {
        check_cells(&filled, &flipped)
    });
}

/// Asserts that `filled` holds each upper cell of `drawing` in its colour, and no cell on or below the diagonal.
fn check_cells(filled: &[(egui::Rect, egui::Color32)], drawing: &views::PluckerDrawing) {
    let visuals = egui::Visuals::dark();
    for (i, j) in upper() {
        let at = drawing.place.cell_rect(i, j).shrink(0.5);
        assert!(
            filled
                .iter()
                .any(|(r, c)| *r == at && *c == cell_colour(&visuals, drawing.grid[i][j])),
            "cell ({i}, {j})"
        );
    }
    for i in 0..LATENT_DIM {
        for j in 0..=i {
            let at = drawing.place.cell_rect(i, j).shrink(0.5);
            assert!(
                !filled.iter().any(|(r, _)| *r == at),
                "({i}, {j}) on or below the diagonal is drawn"
            );
        }
    }
}

#[test]
fn compass_plucker_the_grid_fits_the_square() {
    use eframe::egui::{pos2, vec2, Rect};
    // In the app's square, 172 points: the cells as large as the height leaves under the names and the readout.
    let sq = Rect::from_min_size(pos2(12.0, 700.0), vec2(172.0, 172.0));
    let place = GridPlace::fit(sq);
    assert_eq!(place.cell, 11.0, "floor((172 − 2·22 − 6 − 30)/8)");
    assert_eq!(place.origin, pos2(12.0 + 26.0, 700.0 + 22.0));
    assert_eq!(
        place.cell_rect(2, 5),
        Rect::from_min_size(pos2(38.0 + 55.0, 722.0 + 22.0), vec2(11.0, 11.0))
    );
    assert_eq!(
        place.rect(),
        Rect::from_min_size(pos2(38.0, 722.0), vec2(88.0, 88.0))
    );
    assert_eq!(
        place.gauge_rect(),
        Rect::from_min_size(pos2(38.0, 722.0 + 88.0 + 6.0), vec2(88.0, 30.0))
    );
    let close =
        |a: Rect, b: Rect| (a.min - b.min).length() < 1e-4 && (a.max - b.max).length() < 1e-4;
    let bar = place.bar_rect(3, 0.5);
    assert!(
        close(
            bar,
            Rect::from_min_max(pos2(76.5 - 3.3, 823.5), pos2(76.5 + 3.3, 831.0))
        ),
        "{bar:?}"
    );
    let bar = place.bar_rect(0, -1.0);
    assert!(
        close(
            bar,
            Rect::from_min_max(pos2(43.5 - 3.3, 831.0), pos2(43.5 + 3.3, 846.0))
        ),
        "{bar:?}"
    );
    // Narrow, the width sets the cells; too small, none.
    assert_eq!(
        GridPlace::fit(Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 400.0))).cell,
        9.0
    );
    assert_eq!(
        GridPlace::fit(Rect::from_min_size(pos2(0.0, 0.0), vec2(20.0, 20.0))).cell,
        0.0
    );
    // Everything inside the square.
    assert!(sq.contains_rect(place.rect()) && sq.contains_rect(place.gauge_rect()));
    assert!(place.gauge_rect().max.y + COLUMN_NAMES_H <= sq.max.y + 1e-3);
    rejects("cells that overflow the square", || {
        assert!(sq.contains_rect(place.rect().expand(30.0)))
    });
}

#[test]
fn compass_plucker_mixes_toward_the_hue() {
    let (black, white) = (egui::Color32::BLACK, egui::Color32::WHITE);
    assert_eq!(views::mix(black, white, 0.0), black);
    assert_eq!(views::mix(black, white, 1.0), white);
    assert_eq!(
        views::mix(black, white, 0.5),
        egui::Color32::from_rgb(128, 128, 128)
    );
    assert_eq!(
        views::mix(black, white, 0.25),
        egui::Color32::from_rgb(64, 64, 64)
    );
    assert_eq!(
        views::mix(white, black, 0.25),
        egui::Color32::from_rgb(191, 191, 191)
    );
    assert_eq!(views::mix(black, white, 2.0), white, "clamped above");
    assert_eq!(views::mix(black, white, -1.0), black, "clamped below");
    let a = egui::Color32::from_rgb(10, 200, 40);
    let b = egui::Color32::from_rgb(110, 0, 240);
    assert_eq!(views::mix(a, b, 0.5), egui::Color32::from_rgb(60, 100, 140));
    rejects("a mix that does not move", || {
        assert_eq!(views::mix(black, white, 0.5), black)
    });
}

#[test]
fn compass_plucker_paints_the_cursor_the_readout_and_its_names() {
    let visuals = egui::Visuals::dark();
    let p = plane(&view(30.0, -20.0, 25.0, 1.0), [0.1, 0.2, 0.3]);
    let (mut app, mut h) = plucker_app(&p);
    let place = GridPlace::fit(square(&h));
    // A frame with no focus on the compass takes the cursor away: paint the drawing as the app does.
    let drawing = plucker_drawing(square(&h), &app.snapshot().sim.plane, Some((2, 6)));
    let ctx = egui::Context::default();
    let mut full = ctx.run_ui(Default::default(), |ui| {
        views::paint_plucker(ui.painter(), &visuals, &drawing);
    });
    // epaint asserts, in debug builds, that no texture delta is dropped unhandled.
    full.textures_delta.clear();
    let painter_shapes: Vec<Shape> = full.shapes.into_iter().map(|c| c.shape).collect();
    let ring = place.cell_rect(2, 6);
    assert!(
        painter_shapes.iter().any(|s| matches!(s, Shape::Rect(r)
            if r.rect == ring && r.stroke.width == views::CURSOR_W
                && r.stroke.color == visuals.hyperlink_color)),
        "the cell cursor's ring in interaction mode's blue"
    );
    let gauge = place.gauge_rect();
    assert!(
        painter_shapes
            .iter()
            .any(|s| matches!(s, Shape::LineSegment { points, .. }
            if points[0] == egui::pos2(gauge.min.x, gauge.center().y)
                && points[1] == egui::pos2(gauge.max.x, gauge.center().y))),
        "the readout's zero line"
    );
    // The readout's names under it, the slice axis's in the accent colour.
    for (k, name) in AXES.iter().enumerate() {
        let column = place.cell_rect(0, k);
        let want = if k == drawing.slice_axis {
            visuals.hyperlink_color
        } else {
            visuals.weak_text_color()
        };
        assert!(
            painter_shapes.iter().any(|s| match s {
                Shape::Text(t) => {
                    let r = s.visual_bounding_rect();
                    t.galley.text() == *name
                        && r.min.y >= gauge.max.y
                        && (r.center().x - column.center().x).abs() < place.cell / 2.0
                        && t.fallback_color == want
                }
                _ => false,
            }),
            "{name} under bar {k}"
        );
    }
    // The readout and the help text beside the grid, in the app.
    let shown = painted_texts(&shapes(&mut h, &mut app));
    assert!(shown.contains(&drawing.readout), "{shown:?}");
    assert!(
        shown.iter().any(|t| t.starts_with("the plane's heading")),
        "the help text"
    );
    rejects("no cursor drawn", || {
        let none = plucker_drawing(square(&h), &app.snapshot().sim.plane, None);
        assert!(none.cursor.is_some())
    });
}
