//! QA tests for TASK-M6-55, from REQ-GUI-414's statement and verify, R-705 and R-462 J2: the app's window takes no size
//! under one at which the default layout meets every panel minimum; `MIN_COMPASS_W` and `MIN_BOTTOM_H` hold the widest
//! readout; and what each constant's doc says it measured is what a frame measures. Each check runs its control through
//! `rejects` (R-176): the window at 1280 by 720, the compass under its minimums, a panel past its edge, the doc's number
//! a point under.
// The file name gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::cell::RefCell;

use eframe::egui::{
    pos2, vec2, Event, FullOutput, PointerButton, Pos2, Rect, Shape, Vec2, ViewportBuilder,
};
use engine::contract::sim_config::{Latent, Plane};
use gui::app::App;
use gui::capture::{PIXELS_PER_POINT, SIZE};
use gui::cli::mock::{dispatch, MEASURED_LEAST_WINDOW, WINDOW_SIZE};
use gui::explore::compass::{self, physical, views, SLICE_BUTTON, TILT_BUTTON, TITLE};
use gui::explore::manifold_view::{basis_edit, z0_edit, Touch, View, PRESETS, SLICE_AXIS};
use gui::explore::{COMPASS_JACOBI, COMPASS_PLUCKER, COMPASS_SHAPE, COMPASS_SLICE, COMPASS_TILT};
use gui::headless::Headless;
use gui::layout::{
    Layout, Splits, Splitter, LEAST_WINDOW, MIN_BOTTOM_H, MIN_CANVAS_W, MIN_COMPASS_W,
    MIN_LEGEND_W, MIN_MANIFOLD_W, MIN_TIME_W, MIN_TOP_H, MIN_TRAJECTORY_W,
};
use gui::measure;
use gui::mock::MockEngine;
use gui::side::MockSide;

type Rig = (App<MockSide>, Headless);

/// The display scales a window opens on, pixels per point: 100% to 300%, by the twentieth.
fn scales() -> impl Iterator<Item = f32> {
    (20..=60).map(|k| k as f32 / 20.0)
}

/// The scales a platform offers, for the frames that cost more than arithmetic.
const OFFERED: [f32; 7] = [1.0, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0];

fn rejects(what: &str, check: impl FnOnce()) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check));
    assert!(
        outcome.is_err(),
        "the check passed on {what}, which it must reject"
    );
}

fn window(size: [f32; 2]) -> Rect {
    Rect::from_min_max(pos2(0.0, 0.0), size.into())
}

/// R-462 J2's minimums, read off `l`'s rects: every panel, the canvas apart from its axis strip, and both rows.
fn check_minimums(l: &Layout, what: &str) {
    for (panel, size, min) in [
        ("Manifold view", l.manifold_view.width(), MIN_MANIFOLD_W),
        ("canvas", l.figure.width(), MIN_CANVAS_W),
        ("Trajectory", l.trajectory.width(), MIN_TRAJECTORY_W),
        ("compass", l.compass.width(), MIN_COMPASS_W),
        ("Time", l.time.width(), MIN_TIME_W),
        ("Legend", l.legend.width(), MIN_LEGEND_W),
        ("top row", l.bottom_row.min.y - l.top_bar.max.y, MIN_TOP_H),
        ("bottom row", l.bottom_row.height(), MIN_BOTTOM_H),
    ] {
        assert!(size >= min, "{what}: {panel} {size} under {min}");
    }
}

/// The default layout of a window `[w, h]` pixels in size at `scale`.
fn default_at([w, h]: [u32; 2], scale: f32) -> Layout {
    Layout::new(window([w as f32 / scale, h as f32 / scale]), scale)
}

#[test]
fn qa_m6_55_every_minimum_holds_from_the_least_up() {
    for scale in scales() {
        // The pixels a platform gives a window of the least size in points: the nearest whole number.
        let [lw, lh] = LEAST_WINDOW.map(|points| (points * scale).round() as u32);
        for w in (lw..lw + 48).chain((lw + 48..4000).step_by(53)) {
            for h in (lh..lh + 48).chain((lh + 48..2600).step_by(59)) {
                check_minimums(&default_at([w, h], scale), &format!("{w}x{h} at {scale}"));
            }
        }
    }
    // The window RQ-662 measured: 157.3 against 160.
    let measured = Layout::new(window([1280.0, 720.0]), PIXELS_PER_POINT);
    assert!(measured.bottom_row.height() < MIN_BOTTOM_H);
    rejects("the window of 1280 by 720", || {
        check_minimums(&measured, "1280 by 720")
    });
    // Control: above the least, the row under its minimum, by a split and not by the window.
    let above = window([LEAST_WINDOW[0] + 200.0, LEAST_WINDOW[1] + 200.0]);
    let foot = Layout::new(above, 1.0).footer.min.y;
    let mut splits = Splits::default();
    splits.set(
        Splitter::TopBottom,
        (foot - MIN_BOTTOM_H + 2.0) / above.height(),
    );
    rejects("the bottom row under its minimum, above the least", || {
        check_minimums(&Layout::with_splits(above, 1.0, &splits), "the row")
    });
    // The least is no size picked well clear: well under it each way a minimum goes.
    let [w, h] = LEAST_WINDOW;
    rejects("a window narrower than the least", || {
        check_minimums(&Layout::new(window([w - 9.0, h]), 1.0), "narrow")
    });
    rejects("a window lower than the least", || {
        check_minimums(&Layout::new(window([w, h - 9.0]), 1.0), "low")
    });
}

/// The viewport the app's window opens with, no argument given.
fn opened() -> ViewportBuilder {
    let seen = RefCell::new(None);
    let outcome = dispatch(&[], |_, options, _| {
        *seen.borrow_mut() = Some(options.viewport);
        Ok(())
    });
    assert_eq!(outcome, Ok(()));
    seen.into_inner().expect("a window opened")
}

/// Asserts the viewport holds a window to the least size, and that it opens at or above it.
fn check_refuses(viewport: &ViewportBuilder) {
    let least = viewport.min_inner_size.expect("a least size");
    let [w, h] = LEAST_WINDOW;
    assert_eq!(least, vec2(w, h));
    assert!(w - 1.0 < least.x && h - 1.0 < least.y);
    let size = viewport.inner_size.expect("a size");
    assert!(size.x >= least.x && size.y >= least.y, "{size:?}");
}

#[test]
fn qa_m6_55_the_window_refuses_a_point_smaller() {
    check_refuses(&opened());
    const { assert!(WINDOW_SIZE[0] >= LEAST_WINDOW[0] && WINDOW_SIZE[1] >= LEAST_WINDOW[1]) };
    rejects("a window with no least size", || {
        check_refuses(&ViewportBuilder::default().with_inner_size(WINDOW_SIZE))
    });
    // The measured window leaves the app the same least, beside the panel's strip.
    let (app, _) = measure::split(window(MEASURED_LEAST_WINDOW));
    assert_eq!(app.size(), Vec2::from(LEAST_WINDOW));
}

/// A rig on the mock at the capture's window in points, at `scale`.
fn rig(scale: f32) -> Rig {
    let size = SIZE.map(|pixels| (pixels as f32 / PIXELS_PER_POINT * scale) as u32);
    let mut app = App::new(MockSide::new(MockEngine::new(), None), gui::capture::FORMAT);
    let mut h = Headless::new(size, scale);
    let _ = h.frame(&mut app, Vec::new());
    let _ = h.frame(&mut app, Vec::new());
    (app, h)
}

fn set_plane(h: &mut Headless, app: &mut App<MockSide>, p: &Plane) {
    app.set_field(z0_edit(p.z0));
    app.set_field(basis_edit((p.q1, p.q2)));
    let _ = h.frame(app, Vec::new());
    let _ = h.frame(app, Vec::new());
}

fn click(h: &mut Headless, app: &mut App<MockSide>, id: &str) {
    let at = app.keyboard.place_of(id).expect("placed").rect.center();
    for events in gui::capture::click(at) {
        let _ = h.frame(app, events);
    }
    let _ = h.frame(app, vec![Event::PointerGone]);
}

/// A frame with the compass `[w, high]` points in size, the column's bar settled, and the panel.
fn panel_at(h: &mut Headless, app: &mut App<MockSide>, [w, high]: [f32; 2]) -> (FullOutput, Rect) {
    let screen = h.screen();
    let foot = Layout::new(screen, h.pixels_per_point()).footer.min.y;
    app.splits.set(Splitter::CompassTime, w / screen.width());
    app.splits
        .set(Splitter::TopBottom, (foot - high) / screen.height());
    for _ in 0..3 {
        let _ = h.frame(app, Vec::new());
    }
    let output = h.frame(app, Vec::new());
    let panel = Layout::with_splits(screen, h.pixels_per_point(), &app.splits).compass;
    let size = panel.size();
    assert!(
        (size.x - w).abs() <= 1.0 && (size.y - high).abs() <= 1.0,
        "{size:?} for {w} by {high}"
    );
    (output, panel)
}

/// The text in `panel` starting with `start`, laid out, with the clip it is drawn in.
fn text_in(output: &FullOutput, panel: Rect, start: &str) -> Option<(Rect, Rect)> {
    output.shapes.iter().find_map(|c| match &c.shape {
        Shape::Text(t) if t.galley.text().starts_with(start) && panel.contains(t.pos) => {
            Some((t.galley.rect.translate(t.pos.to_vec2()), c.clip_rect))
        }
        _ => None,
    })
}

/// Asserts that `text`, and the column above it, the title and the six buttons, show whole in `panel`, in one view.
fn check_whole(output: &FullOutput, panel: Rect, text: &[String], what: &str) {
    let above = [
        TITLE,
        TILT_BUTTON,
        SLICE_BUTTON,
        "Cube",
        "Plücker",
        "Jacobi",
        "Shape",
    ];
    let above = above.iter().map(|s| s.to_string());
    for start in above.chain(text.iter().cloned()) {
        let (laid, clip) = text_in(output, panel, &start)
            .unwrap_or_else(|| panic!("{what}: no `{start}` in {panel:?}"));
        assert!(
            clip.contains_rect(laid) && panel.contains_rect(clip),
            "{what}: `{start}` at {laid:?} in {clip:?}, in {panel:?}"
        );
    }
}

/// The view `(τ₁, τ₂, γ)` in degrees, in the mock preset's frame.
fn tilted(tau1: f64, tau2: f64, gamma: f64) -> View {
    let (a, b, d) = (PRESETS[0].0, PRESETS[0].1, SLICE_AXIS);
    View {
        preset: Some(0),
        a,
        b,
        d,
        x: d,
        y: d,
        scale: 0.8,
        tau1: tau1.to_radians(),
        tau2: tau2.to_radians(),
        gamma: gamma.to_radians(),
    }
}

const START: Latent = [0.1, -0.3, 0.25, 0.2, -0.1, 0.3, -0.2, 0.05];

fn plane_of(view: &View) -> Plane {
    let (q1, q2) = view.basis();
    Plane { z0: START, q1, q2 }
}

/// A basis with no zero component, from `seed`: its heading on no axis pair of any chart.
fn scattered(seed: u64) -> Plane {
    let mut x = seed;
    let mut next = || {
        x = x
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((x >> 33) as f64 / (1u64 << 31) as f64 - 0.5) * 1.6
    };
    let q1: Latent = std::array::from_fn(|_| next());
    let q2: Latent = std::array::from_fn(|_| next());
    let z0: Latent = std::array::from_fn(|_| next());
    Plane { z0, q1, q2 }
}

/// The Shape sphere's pole, `r` off it, under a tilted plane.
fn near_pole(r: f64) -> Plane {
    let mut p = plane_of(&tilted(20.0, -15.0, 10.0));
    p.z0 = [-r, 0.5 * r, 0.3, -0.2, 0.5, 0.1, 0.0, 0.0];
    p
}

/// Planes spanning what the readout can say: tilts to the corners where no plane is drawn, each sign, the cut itself, a
/// turn of γ, headings off every axis pair, and the stretch of the sphere's pole at each decade.
fn planes() -> Vec<Plane> {
    let mut all: Vec<Plane> = [
        (90.0, 90.0, 0.0),
        (-90.0, -90.0, 0.0),
        (90.0, -90.0, 0.0),
        (89.97, 89.97, 0.0),
        (20.0, -15.0, 10.0),
        (-45.0, 45.0, 179.0),
        (0.0, 0.0, 0.0),
    ]
    .iter()
    .map(|&(t1, t2, g)| plane_of(&tilted(t1, t2, g)))
    .collect();
    all.extend((1..=3).map(scattered));
    all.extend([0.0, 1e-13, 1e-9, 1e-5, 1e-3, 8.889e-3].map(near_pole));
    all
}

/// The compass's views, and the Cube view's two modes.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Shown {
    CubeTilt,
    CubeSlice,
    Plucker,
    Jacobi,
    Shape,
}

const SHOWN: [Shown; 5] = [
    Shown::CubeTilt,
    Shown::CubeSlice,
    Shown::Plucker,
    Shown::Jacobi,
    Shown::Shape,
];

fn show(h: &mut Headless, app: &mut App<MockSide>, shown: Shown) {
    match shown {
        Shown::CubeTilt => click(h, app, COMPASS_TILT),
        Shown::CubeSlice => click(h, app, COMPASS_SLICE),
        Shown::Plucker => click(h, app, COMPASS_PLUCKER),
        Shown::Jacobi => click(h, app, COMPASS_JACOBI),
        Shown::Shape => click(h, app, COMPASS_SHAPE),
    }
}

/// The readout `app` should show, each of its lines, as the contract's plane reads.
fn readout(app: &App<MockSide>, shown: Shown) -> Vec<String> {
    let sim = &app.snapshot().sim;
    let view = View::read(sim);
    match shown {
        Shown::CubeTilt => compass::readout(&sim.plane.z0, &view, Touch::Tilt),
        Shown::CubeSlice => compass::readout(&sim.plane.z0, &view, Touch::Slice),
        Shown::Plucker => vec![views::readout(&views::plucker(&sim.plane))],
        Shown::Jacobi | Shown::Shape => vec!["strongest:".to_owned()],
    }
}

/// The readout of [`planes`] at each scale, in each view and mode, at the compass's minimums and where its cube is
/// tallest, a taller cube narrowing the column: all of it whole in one view.
#[test]
fn qa_m6_55_the_minimums_hold_every_readout_at_every_scale() {
    let planes = planes();
    let tall = 2.0 * compass::MARGIN + MIN_COMPASS_W / 2.0;
    for scale in OFFERED {
        for shown in SHOWN {
            let (mut app, mut h) = rig(scale);
            show(&mut h, &mut app, shown);
            for (k, p) in planes.iter().enumerate() {
                set_plane(&mut h, &mut app, p);
                let text = readout(&app, shown);
                for size in [
                    [MIN_COMPASS_W, MIN_BOTTOM_H],
                    [MIN_COMPASS_W, tall],
                    [MIN_COMPASS_W + 1.0, MIN_BOTTOM_H + 1.0],
                ] {
                    let (output, panel) = panel_at(&mut h, &mut app, size);
                    let what = format!("{shown:?}, plane {k}, at {size:?}, scale {scale}");
                    check_whole(&output, panel, &text, &what);
                }
            }
        }
    }
}

/// The requirement's own case: a tilted Plücker line, off every axis pair, at the compass's minimums. Its line is the
/// long one: both names and a signed value. Control: the same line in the compass R-705 replaced loses the view.
#[test]
fn qa_m6_55_a_tilted_plucker_line_is_whole() {
    let (mut app, mut h) = rig(PIXELS_PER_POINT);
    show(&mut h, &mut app, Shown::Plucker);
    set_plane(&mut h, &mut app, &scattered(1));
    let text = readout(&app, Shown::Plucker);
    assert!(text[0].contains(" × "), "{text:?}");
    let (output, panel) = panel_at(&mut h, &mut app, [MIN_COMPASS_W, MIN_BOTTOM_H]);
    check_whole(&output, panel, &text, "at the minimums");
    let (mut app, mut h) = rig(PIXELS_PER_POINT);
    show(&mut h, &mut app, Shown::CubeTilt);
    set_plane(&mut h, &mut app, &plane_of(&tilted(90.0, 90.0, 0.0)));
    let text = readout(&app, Shown::CubeTilt);
    let (output, panel) = panel_at(&mut h, &mut app, [240.0, 160.0]);
    rejects("the widest readout in the compass R-705 replaced", || {
        check_whole(&output, panel, &text, "at the old minimums")
    });
}

/// The check that tilt mode's line where no plane is drawn, at the compass of `size` and `scale`, shows whole.
fn no_plane_shows(scale: f32, size: [f32; 2]) -> impl FnOnce() {
    let (mut app, mut h) = rig(scale);
    show(&mut h, &mut app, Shown::CubeTilt);
    set_plane(&mut h, &mut app, &plane_of(&tilted(90.0, 90.0, 0.0)));
    let text = readout(&app, Shown::CubeTilt);
    assert!(text.last().expect("a line").starts_with("no plane"));
    let (output, panel) = panel_at(&mut h, &mut app, size);
    move || check_whole(&output, panel, &text, &format!("{size:?} at {scale}"))
}

/// `MIN_BOTTOM_H`'s doc: the widest readout's last row shows at [`MIN_COMPASS_W`] from 169 up at 1 and 1.5 pixels per
/// point, and from 171 at 2. A point under, it doesn't.
#[test]
fn qa_m6_55_min_bottom_h_is_what_its_doc_measured() {
    for (scale, from) in [(1.0, 169.0), (1.5, 169.0), (2.0, 171.0)] {
        no_plane_shows(scale, [MIN_COMPASS_W, from])();
        rejects(
            &format!("a point under {from} at {scale}"),
            no_plane_shows(scale, [MIN_COMPASS_W, from - 1.0]),
        );
    }
    const { assert!(MIN_BOTTOM_H >= 171.0) };
}

/// `MIN_COMPASS_W`'s doc: at scales 1 to 3 the same line shows from 335 up at every height from `MIN_BOTTOM_H` to 197;
/// at 334 wide and 181 high, at 2, it leaves the view, and at 172 high it does from 325 down.
#[test]
fn qa_m6_55_min_compass_w_is_what_its_doc_measured() {
    for scale in OFFERED {
        let mut high = MIN_BOTTOM_H;
        while high <= 197.0 {
            no_plane_shows(scale, [335.0, high])();
            high += 1.0;
        }
    }
    rejects(
        "334 wide and 181 high, at 2",
        no_plane_shows(2.0, [334.0, 181.0]),
    );
    rejects(
        "325 wide at the least height",
        no_plane_shows(2.0, [325.0, 172.0]),
    );
    const { assert!(MIN_COMPASS_W >= 335.0) };
}

/// `MIN_BOTTOM_H`'s doc: the physical views' help ends in view from 191 up, 195 at 2, and a point under that it doesn't,
/// the cube narrowing the column as it grows. It is not so all the way down: at 172 high, at 1 and 1.5, the Jacobi view's
/// help is whole, its readout there a row, the Shape view's not.
#[test]
fn qa_m6_55_the_help_ends_in_view_where_the_doc_says() {
    let ends = |shown: Shown, scale: f32, high: f32| {
        let (mut app, mut h) = rig(scale);
        show(&mut h, &mut app, shown);
        set_plane(&mut h, &mut app, &plane_of(&tilted(20.0, -15.0, 10.0)));
        let (output, panel) = panel_at(&mut h, &mut app, [MIN_COMPASS_W, high]);
        let (laid, clip) = text_in(&output, panel, "changes as the slice moves").expect("the help");
        (laid, clip)
    };
    let (laid, clip) = ends(Shown::Jacobi, PIXELS_PER_POINT, MIN_BOTTOM_H);
    assert!(clip.contains_rect(laid), "{laid:?} in {clip:?}");
    let (laid, clip) = ends(Shown::Shape, PIXELS_PER_POINT, MIN_BOTTOM_H);
    rejects("the Shape view's help at the least", || {
        assert!(clip.contains_rect(laid))
    });
    for shown in [Shown::Jacobi, Shown::Shape] {
        for (scale, from) in [(1.0, 191.0), (1.5, 191.0), (2.0, 195.0)] {
            let (laid, clip) = ends(shown, scale, from);
            assert!(
                clip.contains_rect(laid),
                "{shown:?} at {from}, {scale}: {laid:?} in {clip:?}"
            );
            let (laid, clip) = ends(shown, scale, from - 1.0);
            rejects("a point under", || assert!(clip.contains_rect(laid)));
        }
    }
}

/// A flat plane at the sphere's pole: the longest thing a physical view says, the pole's line and the degenerate message
/// together, under the grid at the compass's minimums.
#[test]
fn qa_m6_55_the_lines_under_the_grid_stay_in_the_panel() {
    for scale in OFFERED {
        let (mut app, mut h) = rig(scale);
        show(&mut h, &mut app, Shown::Shape);
        let mut flat = near_pole(0.0);
        flat.q2 = flat.q1;
        set_plane(&mut h, &mut app, &flat);
        let (output, panel) = panel_at(&mut h, &mut app, [MIN_COMPASS_W, MIN_BOTTOM_H]);
        let under = |panel: Rect| {
            for line in [physical::POLE_LINE, physical::DEGENERATE] {
                let (laid, _) = text_in(&output, panel, line).expect("said under the grid");
                assert!(panel.contains_rect(laid), "{line}: {laid:?} in {panel:?}");
                assert!(
                    laid.max.x <= compass::square(panel).max.x,
                    "{line}: {laid:?} runs into the column"
                );
            }
        };
        under(panel);
        // Control: the panel a little shorter, its foot where the message ends less a row.
        let (laid, _) = text_in(&output, panel, physical::DEGENERATE).expect("said");
        rejects("a panel the message passes", || {
            under(panel.with_max_y(laid.max.y - 4.0))
        });
    }
}

/// Asserts that no text in `rect` shows past it, nor, if `whole`, is cut by its clip.
fn check_inside(output: &FullOutput, rect: Rect, whole: bool, what: &str) {
    let mut seen = 0;
    for c in &output.shapes {
        let Shape::Text(t) = &c.shape else { continue };
        if !rect.contains(t.pos) {
            continue;
        }
        seen += 1;
        let laid = t.galley.rect.translate(t.pos.to_vec2());
        let shows = laid.intersect(c.clip_rect);
        assert!(
            !shows.is_positive() || rect.expand(0.5).contains_rect(shows),
            "{what}: `{}` shows at {shows:?}, past {rect:?}",
            t.galley.text()
        );
        assert!(
            !whole || c.clip_rect.expand(0.5).contains_rect(laid),
            "{what}: `{}` at {laid:?} cut by {:?}",
            t.galley.text(),
            c.clip_rect
        );
    }
    assert!(seen > 0, "{what}: no text in {rect:?}");
}

/// [`check_inside`] for each panel of `l`; the compass's column and the Manifold view's rows scroll, so may be cut.
fn check_panels(output: &FullOutput, l: &Layout, what: &str) {
    for (panel, rect, whole) in [
        ("Manifold view", l.manifold_view, false),
        ("Trajectory", l.trajectory, true),
        ("compass", l.compass, false),
        ("Time", l.time, true),
        ("Legend", l.legend, true),
    ] {
        check_inside(output, rect, whole, &format!("{what}: {panel}"));
    }
}

/// REQ-GUI-414's window at its least: at the default splits every panel holds its text, in each of the compass's views.
/// Control: Time under the width its own doc gives.
#[test]
fn qa_m6_55_at_the_least_window_every_panel_holds_its_text() {
    for scale in OFFERED {
        for shown in SHOWN {
            let (mut app, mut h) = rig(scale);
            show(&mut h, &mut app, shown);
            set_plane(&mut h, &mut app, &plane_of(&tilted(90.0, 90.0, 0.0)));
            h.set_size(LEAST_WINDOW.map(|points| (points * scale).round() as u32));
            for _ in 0..3 {
                let _ = h.frame(&mut app, Vec::new());
            }
            let output = h.frame(&mut app, Vec::new());
            let l = Layout::new(h.screen(), scale);
            check_minimums(&l, "the least window");
            check_panels(&output, &l, &format!("{shown:?} at {scale}"));
        }
    }
    let (mut app, mut h) = rig(PIXELS_PER_POINT);
    let screen = h.screen();
    let l = Layout::new(screen, PIXELS_PER_POINT);
    app.splits.set(
        Splitter::TimeLegend,
        (l.time.min.x + MIN_TIME_W - 60.0) / screen.width(),
    );
    let _ = h.frame(&mut app, Vec::new());
    let output = h.frame(&mut app, Vec::new());
    let narrow = Layout::with_splits(screen, PIXELS_PER_POINT, &app.splits);
    rejects("Time under its minimum", || {
        check_inside(&output, narrow.time, true, "Time")
    });
}

fn press_at(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    }
}

/// `MIN_*` still hold a drag at the least window, R-705 leaving `with_splits` alone: each handle, both ways, to the
/// window's edge, leaves every minimum met.
#[test]
fn qa_m6_55_at_the_least_window_a_drag_keeps_every_minimum() {
    for splitter in Splitter::ALL {
        let mut moved = false;
        for corner in [pos2(1.0, 1.0), pos2(1000.0, 1000.0)] {
            let (mut app, mut h) = rig(1.0);
            h.set_size(LEAST_WINDOW.map(|points| points as u32));
            let _ = h.frame(&mut app, Vec::new());
            let _ = h.frame(&mut app, Vec::new());
            let screen = h.screen();
            let before = Layout::with_splits(screen, 1.0, &app.splits);
            let from = before.handle(splitter).center();
            let to = pos2(
                corner.x.min(screen.max.x - 1.0),
                corner.y.min(screen.max.y - 1.0),
            );
            let _ = h.frame(&mut app, vec![Event::PointerMoved(from)]);
            let _ = h.frame(&mut app, vec![press_at(from, true)]);
            for k in 1..=6 {
                let at = from + (to - from) * (k as f32 / 6.0);
                let _ = h.frame(&mut app, vec![Event::PointerMoved(at)]);
            }
            let _ = h.frame(&mut app, vec![press_at(to, false)]);
            let _ = h.frame(&mut app, vec![Event::PointerGone]);
            let _ = h.frame(&mut app, Vec::new());
            let after = Layout::with_splits(screen, 1.0, &app.splits);
            check_minimums(&after, &format!("{splitter:?} dragged to {to:?}"));
            moved |= after.handle(splitter) != before.handle(splitter);
        }
        assert!(moved, "{splitter:?}: no drag moved it");
    }
    // Control: a split past what the drag allows, as a drag with no minimum would leave it.
    let screen = window(LEAST_WINDOW);
    let mut splits = Splits::default();
    splits.set(Splitter::CompassTime, 0.1);
    rejects("the compass dragged under its minimum", || {
        check_minimums(&Layout::with_splits(screen, 1.0, &splits), "dragged")
    });
}
