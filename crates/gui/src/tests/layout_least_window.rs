//! `layout_least_window` (REQ-GUI-414; R-705, R-462 J2): at the window's least size the default splits give every panel
//! at least its minimum, at the display scales a window opens on, and so at every size above it; the window refuses
//! anything smaller; and the re-measured [`MIN_COMPASS_W`] and [`MIN_BOTTOM_H`] hold the compass's widest readouts, a
//! tilted Plücker line among them, whole in one view. Each check also runs on an outcome it must reject (R-176, through
//! `rejects`): the default row under its minimum at a size above the least, a window a point short of what a panel
//! needs, a window with no least size, and the compass under its minimums.

use std::cell::RefCell;

use eframe::egui::{self, pos2, FullOutput, Rect, Shape};
use engine::contract::sim_config::Plane;

use super::compass_support::{plane, settle, view};
use super::mock_layout_splitters::check_minimums;
use super::support::{click_scope, mock_app, rejects};
use crate::app::App;
use crate::capture::{PIXELS_PER_POINT, SIZE};
use crate::cli::mock::{dispatch, MEASURED_LEAST_WINDOW, WINDOW_SIZE};
use crate::explore::compass::{views, SLICE_BUTTON, TILT_BUTTON, TITLE};
use crate::explore::manifold_view::{basis_edit, z0_edit};
use crate::explore::{COMPASS_JACOBI, COMPASS_PLUCKER, COMPASS_SHAPE, COMPASS_SLICE};
use crate::headless::Headless;
use crate::layout::{
    Layout, Splits, Splitter, LEAST_HEIGHTS, LEAST_WIDTHS, LEAST_WINDOW, MIN_BOTTOM_H,
    MIN_CANVAS_W, MIN_COMPASS_W, MIN_LEGEND_W, MIN_MANIFOLD_W, MIN_TIME_W, MIN_TOP_H,
    MIN_TRAJECTORY_W,
};
use crate::measure;
use crate::side::MockSide;

/// The scales a window opens on, pixels per point.
const SCALES: [f32; 7] = [1.0, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0];

/// A window `size` points in size, its corner at the origin.
fn window(size: [f32; 2]) -> Rect {
    Rect::from_min_max(pos2(0.0, 0.0), size.into())
}

/// The default layout of a window `[w, h]` pixels in size at `scale`.
fn default_at([w, h]: [u32; 2], scale: f32) -> Layout {
    Layout::new(window([w as f32 / scale, h as f32 / scale]), scale)
}

#[test]
fn layout_least_window_the_default_splits_meet_every_minimum() {
    assert_eq!(LEAST_WINDOW, [1371.0, 817.0]);
    for scale in SCALES {
        // The window's size is a whole number of pixels, the nearest to the least in points.
        let least = LEAST_WINDOW.map(|points| (points * scale).round() as u32);
        check_minimums(&default_at(least, scale));
        for dw in (0..120).step_by(3) {
            for dh in 0..120 {
                check_minimums(&default_at([least[0] + dw, least[1] + dh], scale));
            }
        }
    }
    // The size the review measured: the row there is 157.3 high.
    let review = Layout::new(window([1280.0, 720.0]), PIXELS_PER_POINT);
    assert!((review.bottom_row.height() - 157.3).abs() < 0.1);
    rejects(
        "the review's window, under the bottom row's minimum",
        || check_minimums(&review),
    );
    // Control: the default row, under its minimum, at a size above the least.
    let above = window([LEAST_WINDOW[0] + 40.0, LEAST_WINDOW[1] + 40.0]);
    let footer = Layout::new(above, 1.0).footer.min.y;
    let mut splits = Splits::default();
    splits.set(
        Splitter::TopBottom,
        (footer - MIN_BOTTOM_H + 1.0) / above.height(),
    );
    let under = Layout::with_splits(above, 1.0, &splits);
    assert!(under.bottom_row.height() < MIN_BOTTOM_H);
    rejects("the bottom row under its minimum", || {
        check_minimums(&under)
    });
    // The panels that bind: a point short of each one's own least, the rounding apart, is under its minimum.
    let (wide, high) = (
        (MIN_COMPASS_W * 2160.0 / 531.0).ceil(),
        (MIN_TOP_H * 1350.0 / 977.0).ceil(),
    );
    rejects("a window a point too narrow for the compass", || {
        check_minimums(&Layout::new(window([wide - 1.0, LEAST_WINDOW[1]]), 3.0))
    });
    rejects("a window a point too low for the top row", || {
        check_minimums(&Layout::new(window([LEAST_WINDOW[0], high - 1.0]), 3.0))
    });
}

/// Asserts that a panel measured by `size` meets `min` in a window of `layout(least)`, and falls under it four points
/// shorter on `min`'s scale: `least` is this panel's own, neither under nor far over what it needs.
fn check_own_least(
    what: &str,
    least: f32,
    min: f32,
    layout: impl Fn(f32) -> Layout,
    size: fn(&Layout) -> f32,
) {
    let at = size(&layout(least));
    assert!(at >= min, "{what}: {at} at {least}, under {min}");
    let short = size(&layout(least * (min - 4.0) / min));
    assert!(short < min, "{what}: {short} short of {least}, still {min}");
}

#[test]
fn layout_least_window_each_panel_is_considered() {
    type Panel = (&'static str, f32, fn(&Layout) -> f32);
    let widths: [Panel; 6] = [
        ("Manifold view", MIN_MANIFOLD_W, |l| l.manifold_view.width()),
        ("canvas", MIN_CANVAS_W, |l| l.figure.width()),
        ("Trajectory", MIN_TRAJECTORY_W, |l| l.trajectory.width()),
        ("compass", MIN_COMPASS_W, |l| l.compass.width()),
        ("Time", MIN_TIME_W, |l| l.time.width()),
        ("Legend", MIN_LEGEND_W, |l| l.legend.width()),
    ];
    let by_width = |w: f32| Layout::new(window([w, LEAST_WINDOW[1]]), 1.0);
    for ((what, min, size), least) in widths.into_iter().zip(LEAST_WIDTHS) {
        check_own_least(what, least, min, by_width, size);
    }
    let heights: [Panel; 2] = [
        ("top row", MIN_TOP_H, |l| {
            l.bottom_row.min.y - l.top_bar.max.y
        }),
        ("bottom row", MIN_BOTTOM_H, |l| l.bottom_row.height()),
    ];
    let by_height = |h: f32| Layout::new(window([LEAST_WINDOW[0], h]), 1.0);
    for ((what, min, size), least) in heights.into_iter().zip(LEAST_HEIGHTS) {
        check_own_least(what, least, min, by_height, size);
    }
    // The window's least is the most any of them needs, the bottom row alone about 792 high.
    assert_eq!(
        LEAST_WIDTHS.into_iter().fold(0.0, f32::max),
        LEAST_WINDOW[0]
    );
    assert_eq!(
        LEAST_HEIGHTS.into_iter().fold(0.0, f32::max),
        LEAST_WINDOW[1]
    );
    const { assert!(LEAST_HEIGHTS[1] < LEAST_HEIGHTS[0]) };
    // Control: a window a panel's minimum over what it takes.
    rejects("a least far over what the panel needs", || {
        let (what, min, size) = widths[0];
        check_own_least(what, LEAST_WINDOW[0] + 400.0, min, by_width, size)
    });
    rejects("a least under what the panel needs", || {
        let (what, min, size) = widths[0];
        check_own_least(what, LEAST_WIDTHS[0] - 40.0, min, by_width, size)
    });
}

#[test]
fn layout_least_window_is_under_the_window_the_app_opens_at() {
    let opens = |size: [f32; 2]| {
        assert!(size[0] >= LEAST_WINDOW[0] && size[1] >= LEAST_WINDOW[1]);
    };
    opens(WINDOW_SIZE);
    rejects("a window that opens under the least", || {
        opens([WINDOW_SIZE[0], LEAST_WINDOW[1] - 1.0])
    });
}

/// The window's least inner size, as it is opened with no argument.
fn window_least() -> Option<egui::Vec2> {
    let seen = RefCell::new(None);
    let outcome = dispatch(&[], |_name, options, _creator| {
        *seen.borrow_mut() = Some(options.viewport.min_inner_size);
        Ok(())
    });
    assert_eq!(outcome, Ok(()));
    seen.into_inner().expect("no window opened")
}

/// Whether a window whose least inner size is `least` refuses to be `size`.
fn refused(least: Option<egui::Vec2>, size: [f32; 2]) -> bool {
    least.is_some_and(|least| size[0] < least.x || size[1] < least.y)
}

#[test]
fn layout_least_window_a_point_smaller_is_refused() {
    let least = window_least();
    assert_eq!(least, Some(LEAST_WINDOW.into()));
    let [w, h] = LEAST_WINDOW;
    assert!(!refused(least, LEAST_WINDOW));
    assert!(refused(least, [w - 1.0, h]));
    assert!(refused(least, [w, h - 1.0]));
    // Control: a window with no least size takes it.
    rejects("a window with no least size", || {
        assert!(refused(None, [w - 1.0, h]))
    });
    // The measured window keeps the app at the least beside the panel's strip.
    let (app, _) = measure::split(window(MEASURED_LEAST_WINDOW));
    assert_eq!(app.size(), LEAST_WINDOW.into());
}

/// The readout's widest cases: what it is, the plane, the view's button, and how its last line starts.
type Case = (&'static str, Plane, Option<&'static str>, &'static str);

/// A plane with no large angle in the frame of `q₀` and `μ₂`: the Plücker readout's widest, a tilted line whose pair has
/// the longest names and whose value has a sign and three decimals.
fn tilted_plucker() -> Plane {
    let mut p = plane(&view(20.0, -15.0, 10.0, 0.8), [0.1, -0.3, 0.25]);
    p.q1 = [0.0, 0.0, -0.7, 0.1, 0.0, 0.0, 0.1, 0.0];
    p.q2 = [0.0, 0.0, 0.1, 0.0, 0.1, 0.0, 0.0, 0.7];
    p
}

/// Just off the Shape sphere's pole: the physical readout's widest, a pair with a stretch in scientific form.
fn near_pole() -> Plane {
    let mut p = plane(&view(20.0, -15.0, 10.0, 0.8), [0.1, -0.3, 0.25]);
    p.z0 = [-1e-5, 0.5e-5, 0.3, -0.2, 0.5, 0.1, 0.0, 0.0];
    p
}

/// The compass's widest readouts. The line where no plane is drawn is the widest of all, in both modes.
fn cases() -> [Case; 5] {
    let vertical = plane(&view(90.0, 90.0, 0.0, 0.8), [0.1, -0.3, 0.25]);
    let tilted = plane(&view(20.0, -15.0, 10.0, 0.8), [0.1, -0.3, 0.25]);
    [
        ("tilt mode, no plane", vertical.clone(), None, "no plane"),
        (
            "slice mode, no plane",
            vertical,
            Some(COMPASS_SLICE),
            "no plane",
        ),
        (
            "Plücker",
            tilted_plucker(),
            Some(COMPASS_PLUCKER),
            "strongest",
        ),
        ("Jacobi", tilted, Some(COMPASS_JACOBI), "strongest"),
        ("Shape", near_pole(), Some(COMPASS_SHAPE), "strongest"),
    ]
}

/// The app on the mock at `p`, in the view `button` sets, at the capture's window in points, at `scale`.
fn compass_at(p: &Plane, button: Option<&str>, scale: f32) -> (App<MockSide>, Headless) {
    let size = SIZE.map(|pixels| (pixels as f32 / PIXELS_PER_POINT * scale) as u32);
    let (mut app, mut h) = (mock_app(), Headless::new(size, scale));
    let _ = h.frame(&mut app, Vec::new());
    app.set_field(z0_edit(p.z0));
    app.set_field(basis_edit((p.q1, p.q2)));
    let _ = h.frame(&mut app, Vec::new());
    settle(&mut h, &mut app);
    if let Some(button) = button {
        click_scope(&mut h, &mut app, button);
    }
    (app, h)
}

/// A frame with the compass `[w, h]` points in size, the column's bar settled, and the panel.
fn panel_at(h: &mut Headless, app: &mut App<MockSide>, [w, high]: [f32; 2]) -> (FullOutput, Rect) {
    let screen = h.screen();
    let footer = Layout::new(screen, h.pixels_per_point()).footer.min.y;
    app.splits.set(Splitter::CompassTime, w / screen.width());
    app.splits
        .set(Splitter::TopBottom, (footer - high) / screen.height());
    let _ = h.frame(app, Vec::new());
    let _ = h.frame(app, Vec::new());
    let output = h.frame(app, Vec::new());
    let panel = Layout::with_splits(screen, h.pixels_per_point(), &app.splits).compass;
    (output, panel)
}

/// The text in `panel` that starts with `start`, with the clip it is drawn in.
fn text_in(output: &FullOutput, panel: Rect, start: &str) -> Option<(Rect, Rect)> {
    output.shapes.iter().find_map(|c| match &c.shape {
        Shape::Text(t) if t.galley.text().starts_with(start) && panel.contains(t.pos) => {
            Some((t.galley.rect.translate(t.pos.to_vec2()), c.clip_rect))
        }
        _ => None,
    })
}

/// Asserts that the compass's title, its mode buttons and the readout's line starting `last` are whole in `panel`, in
/// one view: drawn, and inside the clip, which is inside the panel.
fn check_whole(output: &FullOutput, panel: Rect, last: &str, what: &str) {
    for start in [TITLE, TILT_BUTTON, SLICE_BUTTON, last] {
        let (text, clip) = text_in(output, panel, start)
            .unwrap_or_else(|| panic!("{what}: no `{start}` in {panel:?}"));
        assert!(
            clip.contains_rect(text) && panel.contains_rect(clip),
            "{what}: `{start}` at {text:?} in {clip:?}, in {panel:?}"
        );
    }
}

/// The compass's sizes in points that decide the readout, from the least the minimums allow: the least width, the cube
/// growing with the height to half that width and narrowing the column, then the widest the capture's window opens at.
/// A wider compass at the same height only widens the column.
fn panels() -> impl Iterator<Item = [f32; 2]> {
    let tallest = 197.0;
    let heights = (0..).map(|i| MIN_BOTTOM_H + i as f32);
    heights
        .take_while(move |high| *high <= tallest)
        .map(|high| [MIN_COMPASS_W, high])
        .chain([[354.0, MIN_BOTTOM_H], [354.0, tallest]])
}

#[test]
fn layout_least_window_the_minimums_hold_the_widest_readout() {
    // The Plücker case is the line the requirement names: tilted, off every axis pair.
    let line = views::readout(&views::plucker(&tilted_plucker()));
    assert!(line.contains("z_q0 × z_μ2") && !line.contains("1.000 ·"));
    for (what, p, button, last) in cases() {
        for scale in [1.0, PIXELS_PER_POINT, 2.0] {
            let (mut app, mut h) = compass_at(&p, button, scale);
            for size in panels() {
                let (output, panel) = panel_at(&mut h, &mut app, size);
                let what = format!("{what} at {size:?}, scale {scale}");
                check_whole(&output, panel, last, &what);
            }
        }
    }
}

#[test]
fn layout_least_window_the_compass_under_its_minimums_loses_the_readout() {
    let (what, p, button, last) = cases().into_iter().next().expect("a case");
    for (scale, sizes) in [
        (
            PIXELS_PER_POINT,
            vec![
                [240.0, 160.0],
                [MIN_COMPASS_W - 12.0, MIN_BOTTOM_H],
                [MIN_COMPASS_W, MIN_BOTTOM_H - 4.0],
            ],
        ),
        // A point narrower than the least and taller: the cube takes the column's last point of width.
        (2.0, vec![[MIN_COMPASS_W - 2.0, 181.0]]),
    ] {
        let (mut app, mut h) = compass_at(&p, button, scale);
        for size in sizes {
            let (output, panel) = panel_at(&mut h, &mut app, size);
            rejects(&format!("the compass at {size:?}, scale {scale}"), || {
                check_whole(&output, panel, last, what)
            });
        }
    }
}

#[test]
fn layout_least_window_the_readout_is_whole_at_the_least_window() {
    for (what, p, button, last) in cases() {
        let (mut app, mut h) = compass_at(&p, button, PIXELS_PER_POINT);
        h.set_size(LEAST_WINDOW.map(|points| (points * PIXELS_PER_POINT) as u32));
        app.splits = Splits::default();
        let _ = h.frame(&mut app, Vec::new());
        let _ = h.frame(&mut app, Vec::new());
        let output = h.frame(&mut app, Vec::new());
        let panel = Layout::new(h.screen(), PIXELS_PER_POINT).compass;
        check_whole(&output, panel, last, what);
    }
}
