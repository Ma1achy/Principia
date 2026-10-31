//! `compass_physical_views` (REQ-GUI-199; render_gui_spec § "The compass: look and views (R-451, R-452)"; R-452, R-550):
//! the Jacobi and Shape grids are the normalised bivector of `J q̂₁ ∧ J q̂₂` over each set's eight coordinates, each over
//! its fixed range; the stretch the Gram factor; on the decoder a slice edit moves the grid and leaves the Plücker grid;
//! `J` at the hovered pixel over the figure; a degenerate pushforward said, not drawn. Each check also runs on an input
//! it must reject (R-176, through `rejects`).

use eframe::egui::{Event, Shape};
use engine::contract::sim_config::{Latent, Links, Plane, LATENT_DIM};

use super::compass_plucker::Rng;
use super::compass_support::{app_at, compass_shapes, plane, square, view};
use super::support::{click_scope, rejects};
use crate::app::App;
use crate::explore::compass::physical::{
    self, apply, gram, latent_jacobian, latent_point, pushforward, Decoder, Matrix, Pushforward,
    Set, DEGENERATE,
};
use crate::explore::compass::views::{
    self, orthonormal, plucker, sum_sq, upper, CompassView, Grid, GridPlace,
};
use crate::explore::compass::{pushed, Compass};
use crate::explore::manifold_view::z0_edit;
use crate::explore::{COMPASS_JACOBI, COMPASS_SHAPE};
use crate::headless::Headless;
use crate::layout::Layout;
use crate::side::MockSide;

/// A known, full-rank linear decoder: each row its own unit diagonal plus a small dyadic coupling.
pub fn linear_a() -> &'static Matrix {
    leaked(std::array::from_fn(|r| {
        std::array::from_fn(|c| {
            if r == c {
                1.0 + r as f64 / 8.0
            } else {
                ((r * 3 + c * 5) % 7) as f64 / 32.0 - 3.0 / 32.0
            }
        })
    }))
}

/// A fixture's matrix, for the life of the test run, as [`Decoder::Linear`] holds it.
pub fn leaked(a: Matrix) -> &'static Matrix {
    Box::leak(Box::new(a))
}

/// `a` with row `row` zero: its range misses `e_row`.
pub fn missing(a: &Matrix, row: usize) -> &'static Matrix {
    let mut out = *a;
    out[row] = [0.0; LATENT_DIM];
    leaked(out)
}

/// `a` with each row divided by `set`'s ranges at the defaults' `q_max`: the `J` of the linear fixture.
fn scaled(a: &Matrix, set: Set) -> Matrix {
    let r = set.ranges(Links::defaults().link_params().q_max);
    std::array::from_fn(|i| a[i].map(|x| x / r[i]))
}

/// The normalised bivector of `u₁ ∧ u₂` and its norm.
fn bivector(u1: &Latent, u2: &Latent) -> (Grid, f64) {
    let n = gram(u1, u2);
    let mut g = [[0.0; LATENT_DIM]; LATENT_DIM];
    for (i, j) in upper() {
        g[i][j] = (u1[i] * u2[j] - u1[j] * u2[i]) / n;
    }
    (g, n)
}

/// Asserts `push` is the bivector of `j` on `p`'s orthonormalised pair, within 1e-12.
fn check_grid(push: &Pushforward, j: &Matrix, p: &Plane) {
    let (e1, e2) = orthonormal(p).expect("a plane");
    let (want, n) = bivector(&apply(j, &e1), &apply(j, &e2));
    for (a, b) in upper() {
        assert!(
            (push.grid[a][b] - want[a][b]).abs() <= 1e-12,
            "cell ({a}, {b}): {} against {}",
            push.grid[a][b],
            want[a][b]
        );
    }
    assert!((push.stretch - n).abs() <= 1e-12 * n, "the Gram factor");
    assert!((sum_sq(&push.grid) - 1.0).abs() <= 1e-12, "Σ p² = 1");
    assert!(!push.degenerate);
}

/// A plane with `z₀ = 0`, where the fixture's difference is exact to rounding, and a generic basis.
fn plane_at_zero(rng: &mut Rng) -> Plane {
    Plane {
        z0: [0.0; LATENT_DIM],
        q1: rng.latent(),
        q2: rng.latent(),
    }
}

#[test]
fn compass_physical_views_on_the_linear_fixture_the_grid_is_the_bivector() {
    let a = linear_a();
    let d = Decoder::Linear { a, shell: false };
    let links = Links::defaults();
    let mut rng = Rng::new(3);
    for set in [Set::Jacobi, Set::Shape] {
        let known = scaled(a, set);
        for _ in 0..30 {
            let p = plane_at_zero(&mut rng);
            let j = latent_jacobian(&d, set, &links, &p.z0);
            for r in 0..LATENT_DIM {
                for c in 0..LATENT_DIM {
                    assert!((j[r][c] - known[r][c]).abs() <= 1e-13, "J at ({r}, {c})");
                }
            }
            let push = pushforward(&j, &p);
            check_grid(&push, &known, &p);
            // The stretch does not move when the zoom doubles or the basis shears within the plane.
            let doubled = Plane {
                z0: p.z0,
                q1: p.q1.map(|x| 2.0 * x),
                q2: p.q2.map(|x| 2.0 * x),
            };
            let d2 = pushforward(&j, &doubled);
            assert!((d2.stretch - push.stretch).abs() <= 1e-12 * push.stretch);
            let sheared = Plane {
                z0: p.z0,
                q1: p.q1,
                q2: std::array::from_fn(|k| p.q2[k] + 0.4 * p.q1[k]),
            };
            let s2 = pushforward(&j, &sheared);
            assert!((s2.stretch - push.stretch).abs() <= 1e-12 * push.stretch);
        }
    }
    let p = plane_at_zero(&mut rng);
    let known = scaled(a, Set::Jacobi);
    let push = pushforward(&known, &p);
    // Controls: the reference's synthetic Jacobian (a fixed matrix unrelated to the decoder), and un-orthonormalised,
    // unnormalised u₁, u₂.
    let synthetic: Matrix =
        std::array::from_fn(|r| std::array::from_fn(|c| ((r + 1) as f64 * (c as f64 + 0.3)).sin()));
    rejects("the reference's synthetic Jacobian", || {
        check_grid(&pushforward(&synthetic, &p), &known, &p)
    });
    let (u1, u2) = (apply(&known, &p.q1), apply(&known, &p.q2));
    let mut raw = push;
    for (i, j) in upper() {
        raw.grid[i][j] = u1[i] * u2[j] - u1[j] * u2[i];
    }
    rejects("a grid from un-orthonormalised u₁, u₂", || {
        check_grid(&raw, &known, &p)
    });
    // Control: a grid with no range divided out, on rows of differing range.
    rejects("a grid not divided by its ranges", || {
        check_grid(&pushforward(a, &p), &known, &p)
    });
}

/// The app on the mock at the plane `p`, showing `id`'s view.
fn app_in(p: &Plane, id: &'static str) -> (App<MockSide>, Headless) {
    let (mut app, mut h) = app_at(p);
    click_scope(&mut h, &mut app, id);
    let _ = h.frame(&mut app, Vec::new());
    (app, h)
}

/// The push of `app`'s current view.
fn push_of(app: &App<MockSide>) -> Pushforward {
    let set = app
        .manifold
        .compass
        .view
        .physical()
        .expect("a physical view");
    pushed(&app.snapshot().sim, &app.manifold.compass, set)
}

/// A generic start plane on the decoder.
fn start() -> Plane {
    plane(&view(20.0, -15.0, 10.0, 0.8), [0.1, -0.3, 0.25])
}

/// Asserts the cells painted are `grid`'s colours and the names `set`'s.
fn check_painted(shapes: &[Shape], h: &Headless, grid: &Grid, set: Set) {
    let visuals = eframe::egui::Visuals::dark();
    let place = GridPlace::fit(square(h));
    for (i, j) in upper() {
        let at = place.cell_rect(i, j).shrink(0.5);
        let want = views::cell_colour(&visuals, grid[i][j]);
        assert!(
            shapes
                .iter()
                .any(|s| matches!(s, Shape::Rect(r) if r.rect == at && r.fill == want)),
            "cell ({i}, {j}) is not painted its colour"
        );
    }
    for name in set.names() {
        let n = shapes
            .iter()
            .filter(|s| matches!(s, Shape::Text(t) if t.galley.text() == name))
            .count();
        assert!(n >= 2, "{name} on both edges: {n}");
    }
}

#[test]
fn compass_physical_views_draw_on_the_decoder_and_a_slice_edit_moves_them() {
    for (id, view_is, set) in [
        (COMPASS_JACOBI, CompassView::Jacobi, Set::Jacobi),
        (COMPASS_SHAPE, CompassView::Shape, Set::Shape),
    ] {
        let p = start();
        let (mut app, mut h) = app_in(&p, id);
        assert_eq!(app.manifold.compass.view, view_is);
        let before = push_of(&app);
        check_painted(&compass_shapes(&mut h, &mut app), &h, &before.grid, set);
        let latent = plucker(&app.snapshot().sim.plane);
        // A slice edit: z₀ moves, the basis does not.
        let mut z0 = p.z0;
        z0[6] += 0.9;
        z0[1] -= 0.7;
        app.set_field(z0_edit(z0));
        let _ = h.frame(&mut app, Vec::new());
        let _ = h.frame(&mut app, Vec::new());
        let after = push_of(&app);
        check_painted(&compass_shapes(&mut h, &mut app), &h, &after.grid, set);
        let moved = upper()
            .map(|(a, b)| (before.grid[a][b] - after.grid[a][b]).abs())
            .fold(0.0, f64::max);
        assert!(moved > 1e-3, "{set:?}: the grid did not move: {moved}");
        assert_eq!(
            plucker(&app.snapshot().sim.plane),
            latent,
            "the Plücker grid is unchanged"
        );
        // Control: a painted check on the stale grid.
        let shapes = compass_shapes(&mut h, &mut app);
        rejects("a view that does not follow z₀", || {
            check_painted(&shapes, &h, &before.grid, set)
        });
    }
}

#[test]
fn compass_physical_views_read_after_eta_e_on_an_e_star_chart() {
    let p = start();
    let (mut app, mut h) = app_in(&p, COMPASS_JACOBI);
    let off = push_of(&app);
    app.manifold.compass.decoder = Decoder::Real { e_star: Some(0.0) };
    let _ = h.frame(&mut app, Vec::new());
    let on = push_of(&app);
    let links = Links::defaults();
    let j = latent_jacobian(
        &Decoder::Real { e_star: Some(0.0) },
        Set::Jacobi,
        &links,
        &app.snapshot().sim.plane.z0,
    );
    check_grid(&on, &j, &app.snapshot().sim.plane);
    // Control: the grid read before η_E on this E* chart.
    rejects("a grid read before η_E", || {
        check_grid(&off, &j, &app.snapshot().sim.plane)
    });
}

#[test]
fn compass_physical_views_take_j_at_the_hovered_pixel_over_the_figure() {
    let p = start();
    let (mut app, mut h) = app_in(&p, COMPASS_JACOBI);
    let at_z0 = push_of(&app);
    assert_eq!(app.manifold.compass.hover, None);
    let figure = Layout::new(h.screen(), crate::capture::PIXELS_PER_POINT).figure;
    let _ = h.frame(
        &mut app,
        vec![Event::PointerMoved(
            figure.center() + eframe::egui::vec2(90.0, -70.0),
        )],
    );
    let _ = h.frame(&mut app, Vec::new());
    let hover = app.manifold.compass.hover.expect("over the figure");
    let links = Links::defaults();
    let sim_plane = app.snapshot().sim.plane.clone();
    let z = latent_point(&sim_plane, Some(hover));
    let want = pushforward(
        &latent_jacobian(&Decoder::default(), Set::Jacobi, &links, &z),
        &sim_plane,
    );
    let got = push_of(&app);
    assert_eq!(got, want, "J at the hovered pixel's latent point");
    let moved = upper()
        .map(|(a, b)| (got.grid[a][b] - at_z0.grid[a][b]).abs())
        .fold(0.0, f64::max);
    assert!(moved > 1e-3, "the hovered pixel's J differs here: {moved}");
    // The pixel's latent point is the affine chart's: z₀ + (2s−1) q₁ + (2t−1) q₂.
    for (k, x) in z.iter().enumerate() {
        let want = sim_plane.z0[k]
            + (2.0 * hover[0] - 1.0) * sim_plane.q1[k]
            + (2.0 * hover[1] - 1.0) * sim_plane.q2[k];
        assert!((x - want).abs() <= 1e-15, "z at {k}");
    }
    // Leaving the figure returns to z₀.
    let _ = h.frame(&mut app, vec![Event::PointerGone]);
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(app.manifold.compass.hover, None);
    assert_eq!(push_of(&app), at_z0, "back at z₀");
    assert_eq!(latent_point(&sim_plane, None), sim_plane.z0);
    // Control: a view that stays at z₀ under the pointer.
    rejects("a view that ignores the hover", || assert_eq!(at_z0, want));
}

#[test]
fn compass_physical_views_say_a_degenerate_pushforward() {
    // A plane along two saturated links: both columns of J are exactly 0, on the decoder.
    let mut z0 = [0.2; LATENT_DIM];
    z0[0] = 1000.0;
    z0[1] = 1000.0;
    let e = |k: usize| -> Latent { std::array::from_fn(|n| if n == k { 1.0 } else { 0.0 }) };
    let p = Plane {
        z0,
        q1: e(0),
        q2: e(1),
    };
    let links = Links::defaults();
    let j = latent_jacobian(&Decoder::default(), Set::Jacobi, &links, &p.z0);
    let push = pushforward(&j, &p);
    assert!(push.degenerate, "{push:?}");
    assert_eq!(push.grid, [[0.0; LATENT_DIM]; LATENT_DIM]);
    let (mut app, mut h) = app_in(&p, COMPASS_JACOBI);
    let shapes = compass_shapes(&mut h, &mut app);
    let says = |shapes: &[Shape]| {
        assert!(
            shapes
                .iter()
                .any(|s| matches!(s, Shape::Text(t) if t.galley.text() == DEGENERATE)),
            "the message"
        );
    };
    says(&shapes);
    let note = shapes
        .iter()
        .find_map(|s| match s {
            Shape::Text(t) if t.galley.text() == DEGENERATE => Some(t),
            _ => None,
        })
        .expect("the message");
    let laid = note.galley.rect.translate(note.pos.to_vec2());
    assert!(
        (laid.center() - square(&h).center()).length() < 1.0,
        "centred in the square"
    );
    let place = GridPlace::fit(square(&h));
    let cells = upper()
        .filter(|&(i, k)| {
            let at = place.cell_rect(i, k).shrink(0.5);
            shapes
                .iter()
                .any(|s| matches!(s, Shape::Rect(r) if r.rect == at))
        })
        .count();
    assert_eq!(cells, 0, "said, not drawn");
    // On the fixture: one vector in the null space, boundary of the cut.
    let a = missing(linear_a(), 0);
    let null = Plane {
        z0: [0.0; LATENT_DIM],
        q1: e(0),
        q2: e(1),
    };
    assert!(
        !pushforward(a, &null).degenerate,
        "row 0 only: still a plane"
    );
    let plane_of_rows = {
        let mut m = [[0.0; LATENT_DIM]; LATENT_DIM];
        m[0][0] = 1.0;
        m[1][0] = 1.0;
        m
    };
    assert!(
        pushforward(&plane_of_rows, &null).degenerate,
        "q₂ in the null space"
    );
    // A basis spanning no plane says so as well.
    let flat = Plane {
        z0: [0.0; LATENT_DIM],
        q1: e(3),
        q2: e(3),
    };
    assert!(pushforward(linear_a(), &flat).degenerate);
    // Control: the message on a view that draws.
    let (mut app, mut h) = app_in(&start(), COMPASS_JACOBI);
    let drawn = compass_shapes(&mut h, &mut app);
    rejects("a view with a plane", || says(&drawn));
}

#[test]
fn compass_physical_views_the_readout_and_hover_name_the_sets() {
    let mut grid = [[0.0; LATENT_DIM]; LATENT_DIM];
    grid[2][5] = -0.75;
    let push = Pushforward {
        grid,
        stretch: 1.5,
        degenerate: false,
    };
    assert_eq!(
        physical::readout(Set::Jacobi, &push),
        "strongest: p_ρx × p_λy -0.750 · stretch 1.500"
    );
    assert_eq!(
        physical::readout(Set::Shape, &push),
        "strongest: dθ/dt × dR/dt -0.750 · stretch 1.500"
    );
    assert_eq!(
        physical::hover_text(Set::Jacobi, &grid, 2, 5),
        "p_ρx × p_λy  -0.750"
    );
    let blank = Pushforward {
        grid: [[0.0; LATENT_DIM]; LATENT_DIM],
        stretch: 0.0,
        degenerate: true,
    };
    assert_eq!(
        physical::readout(Set::Jacobi, &blank),
        "strongest: — · stretch 0.000"
    );
    assert!(
        physical::HELP.contains("reference range"),
        "the metric named"
    );
    assert!(physical::HELP.contains("changes as the slice moves"));
    assert!(physical::HELP.contains("Plücker"));
    assert!(physical::HELP.contains("nonlinear"));
    // Which view's help: drawn beside the compass, the app's own text.
    let (mut app, mut h) = app_in(&start(), COMPASS_SHAPE);
    let texts = super::support::frame_texts(&mut h, &mut app);
    assert!(texts.contains(&physical::HELP.to_owned()), "the help drawn");
    assert!(
        texts
            .iter()
            .any(|t| t.starts_with("strongest: ") && t.contains("stretch")),
        "the readout drawn"
    );
    rejects("a readout of another set's names", || {
        assert_eq!(
            physical::readout(Set::Shape, &push),
            physical::readout(Set::Jacobi, &push)
        )
    });
}

#[test]
fn compass_physical_views_are_plain_grid_views_to_the_compass() {
    assert_eq!(CompassView::Jacobi.physical(), Some(Set::Jacobi));
    assert_eq!(CompassView::Shape.physical(), Some(Set::Shape));
    assert_eq!(CompassView::Plucker.physical(), None);
    assert_eq!(CompassView::Cube.physical(), None);
    let c = Compass::default();
    assert_eq!(
        c.decoder,
        Decoder::Real { e_star: None },
        "E* off by default"
    );
    assert!(!Decoder::default().on_shell());
    assert!(Decoder::Real { e_star: Some(0.0) }.on_shell());
    assert!(Decoder::Linear {
        a: linear_a(),
        shell: true
    }
    .on_shell());
    assert!(!Decoder::Linear {
        a: linear_a(),
        shell: false
    }
    .on_shell());
    // R-452's proposed set had a row always 0 (ρ̃ᵧ); R-550's has none anywhere on the sampled image.
    let links = Links::defaults();
    let mut rng = Rng::new(9);
    let check_rows = |j: &Matrix| {
        for row in j {
            assert!(row.iter().any(|x| *x != 0.0), "a row always 0");
        }
    };
    for _ in 0..20 {
        let z = rng.latent();
        for set in [Set::Jacobi, Set::Shape] {
            check_rows(&latent_jacobian(&Decoder::default(), set, &links, &z));
        }
    }
    let proposed = {
        let mut j = latent_jacobian(&Decoder::default(), Set::Jacobi, &links, &[0.1; 8]);
        j[1] = [0.0; LATENT_DIM];
        j
    };
    rejects("R-452's proposed set, a row always 0", || {
        check_rows(&proposed)
    });
}

#[test]
fn compass_physical_views_the_degenerate_cut_is_relative_to_sigma_1() {
    // J = diag(1000, t, 0, …), the plane e₀ × e₁: the pushforward vanishes where t < c·σ₁ = 1e-6, whatever J's scale.
    let at = |t: f64| {
        let mut j = [[0.0; LATENT_DIM]; LATENT_DIM];
        j[0][0] = 1000.0;
        j[1][1] = t;
        let e = |k: usize| -> Latent { std::array::from_fn(|n| f64::from(n == k)) };
        pushforward(
            &j,
            &Plane {
                z0: [0.0; LATENT_DIM],
                q1: e(0),
                q2: e(1),
            },
        )
        .degenerate
    };
    assert!(at(0.9e-6), "below the cut");
    assert!(!at(1.1e-6), "above it");
    rejects("a cut that does not scale", || assert!(at(1.1e-6)));
}
