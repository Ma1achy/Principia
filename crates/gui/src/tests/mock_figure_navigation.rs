//! `mock_figure_navigation` (REQ-GUI-171; render_gui_spec §G2, §G3; chart_decoder_contract Parts 3–4): a pan and a
//! zoom on the mock's figure are each one `SetField` and no other edit, a pan on `z₀` and a zoom on the basis, and the
//! mock's next frame is the previous one shifted or scaled, within one pixel: navigation is chart construction, the
//! stand-in redrawn for the chart. Each check also runs on an input it must reject (R-176, through `rejects`).

use std::sync::Arc;

use eframe::egui::{self, Event, Modifiers, PointerButton};
use engine::contract::log::Source;
use engine::contract::snapshot::Snapshot;

use super::support::{headless, rejects};
use crate::app::App;
use crate::capture::{render_frame, Shot, FORMAT, PIXELS_PER_POINT};
use crate::headless::Headless;
use crate::layout::Layout;
use crate::mock::canvas::MockCanvas;
use crate::mock::MockEngine;
use crate::side::MockSide;

/// The drag, in points: right and down.
const DRAG: egui::Vec2 = egui::vec2(20.0, 10.0);
/// The drag in whole pixels.
const DRAG_PX: (i64, i64) = (30, 15);
/// The pixels left out at the figure's edges, where the shifted picture has no source.
const MARGIN_PX: i64 = 48;
/// How far, in any channel, a pixel may stray outside the span of the pixels within one pixel of its source: the
/// stand-in's colour ramp is not monotone in each channel, so a point between pixel centres can read a little outside
/// their span.
const TOLERANCE: u8 = 8;
/// The share of pixels that may match nothing: those on the stand-in's sharp band edges, where the colour jumps
/// between neighbouring pixels and a point between their centres can read either side.
const STRAYS: f64 = 0.01;

struct Rig {
    canvas: Arc<MockCanvas>,
    app: App<MockSide>,
    headless: Headless,
    figure: egui::Rect,
}

fn rig() -> Rig {
    let canvas = Arc::new(MockCanvas::new().expect("a GPU adapter for the mock's canvas"));
    let app = App::new(
        MockSide::new(MockEngine::frozen(), Some(canvas.clone())),
        FORMAT,
    );
    let headless = headless();
    let figure = Layout::new(headless.screen(), PIXELS_PER_POINT).figure;
    let mut rig = Rig {
        canvas,
        app,
        headless,
        figure,
    };
    for _ in 0..2 {
        rig.frame(Vec::new());
    }
    rig
}

impl Rig {
    fn frame(&mut self, events: Vec<Event>) {
        let _ = self.headless.frame(&mut self.app, events);
    }

    fn shot(&mut self) -> Shot {
        render_frame(&self.canvas, &mut self.app, &mut self.headless)
    }

    fn snapshot(&self) -> Snapshot {
        self.app.snapshot().clone()
    }

    fn contract_entries(&self) -> usize {
        self.app
            .console()
            .iter()
            .filter(|e| e.source == Source::Contract)
            .count()
    }

    /// A primary drag by `delta` from the figure's centre: move, press, move, release, one frame each.
    fn drag(&mut self, delta: egui::Vec2) {
        let from = self.figure.center();
        let to = from + delta;
        let button = |pos, pressed| Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        for events in [
            vec![Event::PointerMoved(from)],
            vec![button(from, true)],
            vec![Event::PointerMoved(to)],
            vec![button(to, false)],
            Vec::new(),
        ] {
            self.frame(events);
        }
    }

    /// `lines` notches of the wheel up, over the figure's centre.
    fn wheel(&mut self, lines: f32) {
        let at = self.figure.center();
        self.frame(vec![Event::PointerMoved(at)]);
        self.frame(vec![Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, lines),
            phase: egui::TouchPhase::Move,
            modifiers: Modifiers::NONE,
        }]);
        self.frame(Vec::new());
    }

    /// The figure in whole pixels: `[x0, y0, x1, y1)`.
    fn figure_px(&self) -> [i64; 4] {
        let f = self.figure;
        [f.min.x, f.min.y, f.max.x, f.max.y].map(|v| (v * PIXELS_PER_POINT).round() as i64)
    }
}

fn pixel(shot: &Shot, x: i64, y: i64) -> [u8; 4] {
    let i = (y as usize * shot.size[0] as usize + x as usize) * 4;
    shot.rgba[i..i + 4].try_into().unwrap()
}

/// Asserts that every 7th pixel of `after`'s figure, inside the margin, but for at most [`STRAYS`] of them, is what
/// `before` shows within one pixel of `source(x, y)`: each channel within the span of the 3 × 3 pixels about it, give or
/// take [`TOLERANCE`].
fn matches_within_one_pixel(
    before: &Shot,
    after: &Shot,
    figure: [i64; 4],
    source: impl Fn(f64, f64) -> (f64, f64),
) {
    let [x0, y0, x1, y1] = figure;
    let (mut checked, mut strays) = (0_usize, Vec::new());
    for y in (y0 + MARGIN_PX..y1 - MARGIN_PX).step_by(7) {
        for x in (x0 + MARGIN_PX..x1 - MARGIN_PX).step_by(7) {
            let (sx, sy) = source(x as f64 + 0.5, y as f64 + 0.5);
            let (sx, sy) = ((sx - 0.5).round() as i64, (sy - 0.5).round() as i64);
            let want = pixel(after, x, y);
            let around: Vec<[u8; 4]> = (-1..=1)
                .flat_map(|dy| (-1..=1).map(move |dx| (dx, dy)))
                .map(|(dx, dy)| pixel(before, sx + dx, sy + dy))
                .collect();
            let fits = (0..4).all(|c| {
                let lo = around
                    .iter()
                    .map(|p| p[c])
                    .min()
                    .unwrap_or(0)
                    .saturating_sub(TOLERANCE);
                let hi = around
                    .iter()
                    .map(|p| p[c])
                    .max()
                    .unwrap_or(0)
                    .saturating_add(TOLERANCE);
                (lo..=hi).contains(&want[c])
            });
            if !fits {
                strays.push((x, y));
            }
            checked += 1;
        }
    }
    assert!(checked > 1000, "only {checked} pixels were compared");
    assert!(
        strays.len() as f64 <= STRAYS * checked as f64,
        "{} of {checked} pixels match nothing within one pixel of their source, the first at {:?}",
        strays.len(),
        strays.first()
    );
}

/// Asserts that the only difference between `before` and `after` is `plane`'s `z₀` (`pan`) or its basis.
fn only_the_plane_changed(before: &Snapshot, after: &Snapshot, pan: bool) {
    let (mut a, mut b) = (before.sim.clone(), after.sim.clone());
    if pan {
        assert_ne!(a.plane.z0, b.plane.z0, "the pan moved z₀");
        assert_eq!(
            (a.plane.q1, a.plane.q2),
            (b.plane.q1, b.plane.q2),
            "the pan left the basis"
        );
        b.plane.z0 = a.plane.z0;
    } else {
        assert_ne!(
            (a.plane.q1, a.plane.q2),
            (b.plane.q1, b.plane.q2),
            "the zoom changed the basis"
        );
        assert_eq!(a.plane.z0, b.plane.z0, "the zoom left z₀");
        (b.plane.q1, b.plane.q2) = (a.plane.q1, a.plane.q2);
    }
    a.plane = b.plane.clone();
    assert_eq!(a, b, "nothing else in the simulation config changed");
    assert_eq!(
        before.render, after.render,
        "nothing in the render state changed"
    );
}

#[test]
fn mock_figure_navigation_a_drag_is_one_setfield_on_z0() {
    let mut rig = rig();
    let before = rig.snapshot();
    let entries = rig.contract_entries();
    rig.drag(DRAG);
    let after = rig.snapshot();
    assert_eq!(
        after.history.undo_depth,
        before.history.undo_depth + 1,
        "one undoable SetField"
    );
    assert_eq!(rig.contract_entries(), entries + 1, "one SetField logged");
    only_the_plane_changed(&before, &after, true);
    // Control: no drag, no SetField.
    let mut still = super::support::mock_app();
    let mut h = headless();
    let _ = h.frame(&mut still, Vec::new());
    let depth = still.snapshot().history.undo_depth;
    rejects("a frame with no drag", || {
        assert_eq!(depth, before.history.undo_depth + 1)
    });
    // Control: a playhead edit is not a pan.
    let mut moved = after.clone();
    moved.render.playhead.t += 1.0;
    rejects("an edit of the playhead too", || {
        only_the_plane_changed(&before, &moved, true)
    });
}

#[test]
fn mock_figure_navigation_the_wheel_is_one_setfield_on_the_basis() {
    let mut rig = rig();
    let before = rig.snapshot();
    let entries = rig.contract_entries();
    rig.wheel(8.0);
    let after = rig.snapshot();
    assert_eq!(
        after.history.undo_depth,
        before.history.undo_depth + 1,
        "one undoable SetField"
    );
    assert_eq!(rig.contract_entries(), entries + 1, "one SetField logged");
    only_the_plane_changed(&before, &after, false);
    // Eight lines are one octave: the basis halves.
    let half = before.sim.plane.q1.map(|v| v / 2.0);
    assert_eq!(after.sim.plane.q1, half, "eight notches zoom in one octave");
    rejects("a pan, which is not a zoom", || {
        only_the_plane_changed(&before, &after, true)
    });
}

#[test]
fn mock_figure_navigation_a_pan_shifts_the_picture() {
    let mut rig = rig();
    let figure = rig.figure_px();
    let before = rig.shot();
    rig.drag(DRAG);
    let after = rig.shot();
    // The picture follows the pointer.
    let (dx, dy) = (DRAG_PX.0 as f64, DRAG_PX.1 as f64);
    matches_within_one_pixel(&before, &after, figure, |x, y| (x - dx, y - dy));
    rejects("the picture unshifted", || {
        matches_within_one_pixel(&before, &after, figure, |x, y| (x, y));
    });
    rejects("the picture shifted the other way", || {
        matches_within_one_pixel(&before, &after, figure, |x, y| (x + dx, y + dy));
    });
}

#[test]
fn mock_figure_navigation_a_zoom_scales_the_picture_about_the_centre() {
    let mut rig = rig();
    let figure = rig.figure_px();
    let centre = rig.figure.center();
    let (cx, cy) = (
        f64::from(centre.x * PIXELS_PER_POINT),
        f64::from(centre.y * PIXELS_PER_POINT),
    );
    let before = rig.shot();
    rig.wheel(8.0);
    let after = rig.shot();
    // One octave in: each pixel shows what was half as far from the centre.
    matches_within_one_pixel(&before, &after, figure, |x, y| {
        (cx + (x - cx) / 2.0, cy + (y - cy) / 2.0)
    });
    rejects("the picture unscaled", || {
        matches_within_one_pixel(&before, &after, figure, |x, y| (x, y));
    });
    rejects("the picture scaled about another point", || {
        matches_within_one_pixel(&before, &after, figure, |x, y| {
            (cx + 40.0 + (x - cx) / 2.0, cy + (y - cy) / 2.0)
        });
    });
}
