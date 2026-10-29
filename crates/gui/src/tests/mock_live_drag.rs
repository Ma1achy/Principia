//! Live drags (REQ-GUI-206, REQ-GUI-207; R-458; render_gui_spec §G2 "Every drag is live", § "The compass: look and
//! views (R-451, R-452)"; gui_state_contract §2 "A drag coalesces into one history entry"), by pointer scripts on the
//! mock:
//! - `mock_live_compass_drag` (REQ-GUI-206): a slice-mode and a tilt-mode drag on the compass's cube apply their edit
//!   every frame the pointer moves, so the snapshot, the Manifold view's slider and the compass's readout equal the
//!   drag's target at each frame; two pointer events in one frame are one edit; the drag is one undo entry, one Ctrl+Z
//!   restoring the plane before it, and one console line.
//! - `mock_live_slider_drag` (REQ-GUI-207): a drag on the Slice centre slider of the slice normal moves `z₀`, the
//!   compass's drawn offset and the figure's chart every frame, the compass in Slice from the first frame; one on `τ₁`
//!   tilts the drawn plane every frame, the compass in Tilt from the first frame; a press on `τ₂`'s thumb at 90° that
//!   changes nothing still switches to Tilt, and so does a click into `γ`'s number box; the Slice centre slider, `τ₁`,
//!   the scrubber and the figure's pan are each one undo entry and one console line.
//!
//! Each check also runs, in its test, on an input it must reject (R-176, through `rejects`; gui has no route to
//! `negative_control!`, `support`'s note): the trace a GUI with the fault would leave.

use eframe::egui::{self, accesskit::Role, Event, Modifiers, PointerButton, Pos2, Rect, Vec2};
use engine::contract::sim_config::{Latent, Plane};

use super::compass_support::{app_at, drawing, plane, square, view};
use super::support::{nodes, press, rejects, Node, SLICE_FRAME};
use crate::app::App;
use crate::explore::compass::{self, after, angle_text, drag_edit, signed, Drag};
use crate::explore::manifold_view::tilt::Angles;
use crate::explore::manifold_view::{Touch, View, AXES, GAMMA, SLICE_AXIS, SLIDERS, TAU1, TAU2};
use crate::explore::time::SCRUBBER;
use crate::headless::Headless;
use crate::layout::Layout;
use crate::side::{EngineSide, MockSide};

/// The frames each drag moves the pointer on.
const FRAMES: usize = 10;

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

/// The edits the mock has applied so far.
fn applied(app: &mut App<MockSide>) -> u64 {
    app.side().engine().edits_applied()
}

/// The console's `SetField` lines from row `from`, the hover's left out.
fn lines(app: &App<MockSide>, from: usize) -> Vec<String> {
    app.console()[from..]
        .iter()
        .map(|e| e.message.clone())
        .filter(|m| m.starts_with("SetField ") && !m.starts_with("SetField Hover.at "))
        .collect()
}

/// The value of the slider placed as control `id`, as the frame of `nodes` drew it.
fn slider_value(app: &App<MockSide>, nodes: &[Node], id: &str) -> f64 {
    let place = app
        .keyboard
        .place_of(id)
        .unwrap_or_else(|| panic!("`{id}` placed"))
        .rect;
    nodes
        .iter()
        .find(|n| n.role == Role::Slider && place.contains(n.rect.center()))
        .and_then(|n| n.value)
        .unwrap_or_else(|| panic!("no slider drawn in `{id}`"))
}

/// The rect of the slider placed as control `id`, as the next frame draws it.
fn slider_rect(h: &mut Headless, app: &mut App<MockSide>, id: &str) -> Rect {
    let all = nodes(h, app);
    let place = app.keyboard.place_of(id).expect("placed").rect;
    all.iter()
        .find(|n| n.role == Role::Slider && place.contains(n.rect.center()))
        .map(|n| n.rect)
        .unwrap_or_else(|| panic!("no slider drawn in `{id}`"))
}

/// The plane the snapshot holds.
fn held(app: &App<MockSide>) -> Plane {
    app.snapshot().sim.plane.clone()
}

/// Whether `a` and `b` agree within `tol`, component by component.
fn near(a: &Latent, b: &Latent, tol: f64) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() <= tol)
}

/// Whether two planes agree within `tol`.
fn same_plane(a: &Plane, b: &Plane, tol: f64) -> bool {
    near(&a.z0, &b.z0, tol) && near(&a.q1, &b.q1, tol) && near(&a.q2, &b.q2, tol)
}

// --- mock_live_compass_drag (REQ-GUI-206) ----------------------------------------------------------------------------

/// What one frame of a compass drag left: the plane the snapshot holds and the drag's target at that frame, the
/// Manifold view's slider as the next frame drew it and the value it should read, the compass's readout and the line it
/// should hold, the undo depth, and the edits the frame applied.
#[derive(Clone, Debug)]
struct Seen {
    plane: Plane,
    target: Plane,
    slider: f64,
    want_slider: f64,
    readout: Vec<String>,
    want_line: String,
    depth: u32,
    edits: u64,
}

/// A compass drag's trace: its frames, then the frame with two pointer events, the depth before it and at its end, its
/// console lines, and the plane one Ctrl+Z left.
#[derive(Clone, Debug)]
struct Trace {
    start: Plane,
    frames: Vec<Seen>,
    two_events: Seen,
    depth0: u32,
    end_depth: u32,
    lines: Vec<String>,
    undone: Plane,
}

/// The compass's drag in `mode` on the untilted plane through the cube's centre, by `step` points a frame for
/// [`FRAMES`] frames, then one frame with two pointer events, and the release; then one Ctrl+Z.
fn compass_drag(mode: Touch, step: Vec2) -> Trace {
    let start = plane(&view(0.0, 0.0, 0.0, 1.0), [0.0; 3]);
    let (mut app, mut h) = app_at(&start);
    app.manifold.compass.mode = mode;
    let sq = square(&h);
    let from = sq.center();
    let (depth0, rows) = (app.snapshot().history.undo_depth, app.console().len());
    let _ = h.frame(&mut app, vec![Event::PointerMoved(from)]);
    let _ = h.frame(&mut app, vec![button(from, true)]);
    let (slider_id, d) = match mode {
        Touch::Slice => (SLIDERS[SLICE_AXIS], SLICE_AXIS),
        Touch::Tilt => (TAU1, SLICE_AXIS),
    };
    // What the drag sets at `delta` from where it began, and what the slider and the readout should read then.
    let target = |delta: Vec2| -> (Plane, f64, String) {
        let mut drag = Drag::start(mode, &start, &SLICE_FRAME);
        drag.moved(delta, sq);
        let target = after(
            &start,
            &[drag_edit(&start, &SLICE_FRAME, &drag).expect("an edit")],
        );
        let a = Angles::of(&View::of(&target));
        match mode {
            Touch::Slice => (
                target.clone(),
                target.z0[d],
                format!("{} = {} of range", AXES[d], signed(target.z0[d], 2)),
            ),
            Touch::Tilt => (
                target,
                a.tau1,
                format!("τ₁ → {} {}°", AXES[d], angle_text(a.tau1)),
            ),
        }
    };
    let seen = |app: &mut App<MockSide>, h: &mut Headless, delta: Vec2, before: u64| {
        let edits = applied(app) - before;
        let (target, want_slider, want_line) = target(delta);
        let plane = held(app);
        let depth = app.snapshot().history.undo_depth;
        // A still frame, the pointer held: the controls drawn from the snapshot, and no edit.
        let still = applied(app);
        let drawn = nodes(h, app);
        assert_eq!(applied(app), still, "a frame with no move sent an edit");
        Seen {
            plane,
            target,
            slider: slider_value(app, &drawn, slider_id),
            want_slider,
            readout: drawing(h, app).readout,
            want_line,
            depth,
            edits,
        }
    };
    let mut frames = Vec::new();
    for k in 1..=FRAMES {
        let before = applied(&mut app);
        let delta = step * k as f32;
        let _ = h.frame(&mut app, vec![Event::PointerMoved(from + delta)]);
        frames.push(seen(&mut app, &mut h, delta, before));
    }
    // Two pointer events in one frame: one edit, at the second.
    let before = applied(&mut app);
    let (mid, last) = (step * (FRAMES as f32 + 0.5), step * (FRAMES as f32 + 1.0));
    let _ = h.frame(
        &mut app,
        vec![
            Event::PointerMoved(from + mid),
            Event::PointerMoved(from + last),
        ],
    );
    let two_events = seen(&mut app, &mut h, last, before);
    let _ = h.frame(&mut app, vec![button(from + last, false)]);
    let _ = h.frame(&mut app, Vec::new());
    let end_depth = app.snapshot().history.undo_depth;
    let lines = lines(&app, rows);
    press(&mut h, &mut app, egui::Key::Z, Modifiers::COMMAND);
    let _ = h.frame(&mut app, Vec::new());
    Trace {
        start,
        frames,
        two_events,
        depth0,
        end_depth,
        lines,
        undone: held(&app),
    }
}

/// Every frame the pointer moved, the snapshot, the slider and the readout changed and equal the drag's target at that
/// frame, one edit applied; two pointer events in one frame are one edit; the drag is one entry from its first frame,
/// one console line, and one undo restores the plane before it.
fn check_live(t: &Trace) {
    let mut last: Option<&Seen> = None;
    for (k, s) in t.frames.iter().chain([&t.two_events]).enumerate() {
        assert!(
            same_plane(&s.plane, &s.target, 1e-5),
            "frame {k}: the snapshot is not the drag's target: {:?} for {:?}",
            s.plane,
            s.target
        );
        assert!(
            (s.slider - s.want_slider).abs() < 1e-3 * s.want_slider.abs().max(1.0),
            "frame {k}: the slider reads {}, not the target's {}",
            s.slider,
            s.want_slider
        );
        assert!(
            s.readout.iter().any(|line| line.contains(&s.want_line)),
            "frame {k}: the readout {:?} has no {:?}",
            s.readout,
            s.want_line
        );
        assert_eq!(s.edits, 1, "frame {k}: one edit a frame the pointer moved");
        assert_eq!(s.depth, t.depth0 + 1, "frame {k}: the drag is one entry");
        if let Some(prev) = last {
            assert!(
                !same_plane(&s.plane, &prev.plane, 1e-9) && s.slider != prev.slider,
                "frame {k}: nothing changed"
            );
            assert_ne!(s.readout, prev.readout, "frame {k}: the readout unchanged");
        }
        last = Some(s);
    }
    assert_eq!(t.end_depth, t.depth0 + 1, "the drag is one undo entry");
    assert_eq!(
        t.lines.len(),
        1,
        "one console line for the drag: {:?}",
        t.lines
    );
    assert!(
        same_plane(&t.undone, &t.start, 1e-12),
        "one Ctrl+Z did not restore the plane before the drag: {:?}",
        t.undone
    );
}

/// The trace a GUI that sends the drag's edit only on its release would leave: during the drag, the snapshot, the
/// slider and the readout stay where the drag began, and no edit is applied.
fn on_release(t: &Trace) -> Trace {
    let mut c = t.clone();
    for s in c.frames.iter_mut().chain([&mut c.two_events]) {
        s.plane = t.start.clone();
        // Where the drag began, both sliders read 0: z_q3 and τ₁ of the untilted plane through the centre.
        s.slider = 0.0;
        s.readout = Vec::new();
        s.depth = t.depth0;
        s.edits = 0;
    }
    c
}

/// The trace a GUI whose drag pushes an undo entry a frame would leave.
fn entry_per_frame(t: &Trace) -> Trace {
    let mut c = t.clone();
    for (k, s) in c.frames.iter_mut().enumerate() {
        s.depth = t.depth0 + 1 + k as u32;
    }
    c.end_depth = t.depth0 + FRAMES as u32 + 1;
    c.lines = vec![t.lines[0].clone(); FRAMES + 1];
    c
}

/// The drawn normal's screen direction of the untilted plane, `ŝ = (−√3/2, ½)`, `k` points along it.
fn along_s(k: f32) -> Vec2 {
    egui::vec2(-(3f32.sqrt()) / 2.0 * k, 0.5 * k)
}

#[test]
fn mock_live_compass_drag_slice_mode() {
    let t = compass_drag(Touch::Slice, along_s(6.5));
    check_live(&t);
    rejects("a drag that sends its edit only on release", || {
        check_live(&on_release(&t))
    });
    rejects("a drag that pushes an undo entry per frame", || {
        check_live(&entry_per_frame(&t))
    });
}

#[test]
fn mock_live_compass_drag_tilt_mode() {
    // Right and up: `τ₁` and `τ₂` both move, the full width or height 180°, unclamped over the drag.
    let t = compass_drag(Touch::Tilt, egui::vec2(6.5, -2.0));
    check_live(&t);
    rejects("a drag that sends its edit only on release", || {
        check_live(&on_release(&t))
    });
    rejects("a drag that pushes an undo entry per frame", || {
        check_live(&entry_per_frame(&t))
    });
}

// --- mock_live_slider_drag (REQ-GUI-207) -----------------------------------------------------------------------------

/// What one frame of a slider drag left: the snapshot's plane, the mock's plane (the figure's chart), the compass's
/// drawn plane and its mode, and the undo depth.
#[derive(Clone, Debug)]
struct SliderSeen {
    plane: Plane,
    figure: Plane,
    drawn: Option<compass::Cut>,
    mode: Touch,
    depth: u32,
}

/// A slider drag's trace: the mode after the press, before any move; each frame's; the depth before and at the end; the
/// console lines.
#[derive(Clone, Debug)]
struct SliderTrace {
    pressed: Touch,
    frames: Vec<SliderSeen>,
    depth0: u32,
    end_depth: u32,
    lines: Vec<String>,
}

/// The plane the compass draws as the app last drew it: its eased cut.
fn drawn_cut(h: &Headless, app: &App<MockSide>) -> Option<compass::Cut> {
    drawing(h, app).slice.map(|s| s.cut)
}

/// A drag on control `id`'s slider, from its thumb at `value`'s place in `[lo, hi]`, by `step` points a frame for
/// [`FRAMES`] frames, the compass first in `mode`.
fn slider_drag(id: &str, (lo, hi): (f64, f64), value: f64, step: f32, mode: Touch) -> SliderTrace {
    let start = plane(&view(0.0, 0.0, 0.0, 1.0), [0.0; 3]);
    let (mut app, mut h) = app_at(&start);
    app.manifold.compass.mode = mode;
    let rail = slider_rect(&mut h, &mut app, id);
    drag_rail(&mut app, &mut h, rail, (lo, hi), value, step)
}

/// The drag of [`slider_drag`] on the rail `rail`, the app as it stands.
fn drag_rail(
    app: &mut App<MockSide>,
    h: &mut Headless,
    rail: Rect,
    (lo, hi): (f64, f64),
    value: f64,
    step: f32,
) -> SliderTrace {
    // egui's slider keeps its handle inside the rail by half the rail's height at each end.
    let inset = rail.height() / 2.0;
    let x = rail.left() + inset + (rail.width() - 2.0 * inset) * ((value - lo) / (hi - lo)) as f32;
    let from = egui::pos2(x, rail.center().y);
    let (depth0, rows) = (app.snapshot().history.undo_depth, app.console().len());
    let _ = h.frame(app, vec![Event::PointerMoved(from)]);
    let _ = h.frame(app, vec![button(from, true)]);
    let pressed = app.manifold.compass.mode;
    let mut frames = Vec::new();
    for k in 1..=FRAMES {
        let at = from + egui::vec2(step * k as f32, 0.0);
        let _ = h.frame(app, vec![Event::PointerMoved(at)]);
        frames.push(SliderSeen {
            plane: held(app),
            figure: app.side().engine().plane().clone(),
            drawn: drawn_cut(h, app),
            mode: app.manifold.compass.mode,
            depth: app.snapshot().history.undo_depth,
        });
    }
    let end = from + egui::vec2(step * FRAMES as f32, 0.0);
    let _ = h.frame(app, vec![button(end, false)]);
    let _ = h.frame(app, Vec::new());
    SliderTrace {
        pressed,
        frames,
        depth0,
        end_depth: app.snapshot().history.undo_depth,
        lines: lines(app, rows),
    }
}

/// Every frame the snapshot's plane, the figure's chart and the compass's drawn plane (by `drawn`, its offset or its
/// normal) change, the figure's chart the snapshot's; the compass in `mode` from the press on; the drag one entry from
/// its first frame, and one console line.
fn check_slider(t: &SliderTrace, mode: Touch, drawn: fn(&compass::Cut) -> Vec<f64>) {
    assert_eq!(
        t.pressed, mode,
        "the compass's mode on the drag's first frame"
    );
    let mut last: Option<&SliderSeen> = None;
    for (k, s) in t.frames.iter().enumerate() {
        assert_eq!(
            s.figure, s.plane,
            "frame {k}: the figure's chart is not the snapshot's"
        );
        assert_eq!(s.mode, mode, "frame {k}: the compass's mode");
        assert_eq!(s.depth, t.depth0 + 1, "frame {k}: the drag is one entry");
        let cut = s.drawn.as_ref().expect("a drawn plane");
        if let Some(prev) = last {
            assert!(
                !same_plane(&s.plane, &prev.plane, 1e-12),
                "frame {k}: the plane did not move"
            );
            let prev_cut = prev.drawn.as_ref().expect("a drawn plane");
            assert_ne!(
                drawn(cut),
                drawn(prev_cut),
                "frame {k}: the compass's drawing did not move"
            );
        }
        last = Some(s);
    }
    assert_eq!(t.end_depth, t.depth0 + 1, "the drag is one undo entry");
    assert_eq!(
        t.lines.len(),
        1,
        "one console line for the drag: {:?}",
        t.lines
    );
}

/// The trace a GUI whose drag pushes an undo entry a frame would leave.
fn slider_entry_per_frame(t: &SliderTrace) -> SliderTrace {
    let mut c = t.clone();
    for (k, s) in c.frames.iter_mut().enumerate() {
        s.depth = t.depth0 + 1 + k as u32;
    }
    c.end_depth = t.depth0 + FRAMES as u32;
    c
}

/// The trace a GUI that switches the compass only on a changed value would leave, its thumb pressed and not yet moved.
fn switch_on_change(t: &SliderTrace, other: Touch) -> SliderTrace {
    SliderTrace {
        pressed: other,
        ..t.clone()
    }
}

#[test]
fn mock_live_slider_drag_slice_centre_of_the_normal() {
    // The compass starts in Tilt: the slider of the slice normal switches it to Slice from its first frame.
    let t = slider_drag(SLIDERS[SLICE_AXIS], (-1.0, 1.0), 0.0, 6.0, Touch::Tilt);
    let offset: fn(&compass::Cut) -> Vec<f64> = |c| vec![c.offset];
    check_slider(&t, Touch::Slice, offset);
    assert!(
        t.frames[0].plane.z0[SLICE_AXIS] > 0.0,
        "the drag right raised z_q3: {:?}",
        t.frames[0].plane.z0
    );
    rejects("a switch only on a changed value", || {
        check_slider(&switch_on_change(&t, Touch::Tilt), Touch::Slice, offset)
    });
    rejects("a slider that pushes an undo entry per frame", || {
        check_slider(&slider_entry_per_frame(&t), Touch::Slice, offset)
    });
}

#[test]
fn mock_live_slider_drag_tau1() {
    // The compass starts in Slice: τ₁ switches it to Tilt from its first frame, and the drawn plane tilts every frame.
    let t = slider_drag(TAU1, (-90.0, 90.0), 0.0, 6.0, Touch::Slice);
    let normal: fn(&compass::Cut) -> Vec<f64> = |c| c.normal.to_vec();
    check_slider(&t, Touch::Tilt, normal);
    rejects("a switch only on a changed value", || {
        check_slider(&switch_on_change(&t, Touch::Slice), Touch::Tilt, normal)
    });
    rejects("a slider that pushes an undo entry per frame", || {
        check_slider(&slider_entry_per_frame(&t), Touch::Tilt, normal)
    });
}

/// The mode after a press on `τ₂`'s thumb at 90° dragged on past the slider's end, the undo depth and the edits before
/// and after: no value changes, and the compass switches to Tilt all the same.
#[test]
fn mock_live_slider_drag_a_press_past_the_end_switches() {
    let at_end = plane(&view(0.0, 90.0, 0.0, 1.0), [0.0; 3]);
    let (mut app, mut h) = app_at(&at_end);
    app.manifold.compass.mode = Touch::Slice;
    let rail = slider_rect(&mut h, &mut app, TAU2);
    let (depth, edits) = (app.snapshot().history.undo_depth, applied(&mut app));
    // Pressed on the thumb's far side, past 90° on the rail, and dragged on to the right.
    let t = drag_rail(&mut app, &mut h, rail, (-90.0, 90.0), 95.0, 4.0);
    let check = |t: &SliderTrace| {
        assert_eq!(t.pressed, Touch::Tilt, "a press on τ₂'s thumb at its end");
        assert!(
            t.frames.iter().all(|s| s.mode == Touch::Tilt),
            "held past the end"
        );
    };
    check(&t);
    assert_eq!(
        (app.snapshot().history.undo_depth, applied(&mut app)),
        (depth, edits),
        "no value changed past the end"
    );
    let a = Angles::of(&View::read(&app.snapshot().sim));
    assert!((a.tau2 - 90.0).abs() < 1e-9, "τ₂ {}", a.tau2);
    rejects("a switch only on a changed value", || {
        check(&switch_on_change(&t, Touch::Slice))
    });
}

#[test]
fn mock_live_slider_drag_a_click_into_gamma_switches() {
    let start = plane(&view(0.0, 0.0, 0.0, 1.0), [0.0; 3]);
    let (mut app, mut h) = app_at(&start);
    app.manifold.compass.mode = Touch::Slice;
    let all = nodes(&mut h, &mut app);
    let place = app.keyboard.place_of(GAMMA).expect("γ placed").rect;
    let field = all
        .iter()
        .find(|n| n.role != Role::Slider && n.value.is_some() && place.contains(n.rect.center()))
        .map(|n| n.rect)
        .expect("γ's number box");
    let mut modes = Vec::new();
    for events in crate::capture::click(field.center())
        .into_iter()
        .chain([Vec::new()])
    {
        let _ = h.frame(&mut app, events);
        modes.push(app.manifold.compass.mode);
    }
    let check = |modes: &[Touch]| {
        assert_eq!(
            modes.last(),
            Some(&Touch::Tilt),
            "a click into γ's box: {modes:?}"
        );
    };
    check(&modes);
    assert_eq!(held(&app), start, "a click into the box changed nothing");
    rejects("a switch only on a changed value", || {
        check(&[Touch::Slice, Touch::Slice, Touch::Slice])
    });
}

/// A drag's undo depth and lines: one entry and one console line.
fn check_one_entry(what: &str, depth0: u32, depth: u32, lines: &[String]) {
    assert_eq!(depth, depth0 + 1, "{what}: one undo entry");
    assert_eq!(lines.len(), 1, "{what}: one console line: {lines:?}");
}

#[test]
fn mock_live_slider_drag_scrubber_and_pan_are_one_entry() {
    // The scrubber: the playhead follows every frame, one entry and one line.
    let start = plane(&view(0.0, 0.0, 0.0, 1.0), [0.0; 3]);
    let (mut app, mut h) = app_at(&start);
    let rail = slider_rect(&mut h, &mut app, SCRUBBER);
    let t_max = app.snapshot().sim.horizon.t;
    let (depth0, rows) = (app.snapshot().history.undo_depth, app.console().len());
    let from = egui::pos2(rail.left() + rail.height(), rail.center().y);
    let _ = h.frame(&mut app, vec![Event::PointerMoved(from)]);
    let _ = h.frame(&mut app, vec![button(from, true)]);
    let mut times: Vec<f64> = Vec::new();
    for k in 1..=FRAMES {
        let at = from + egui::vec2(8.0 * k as f32, 0.0);
        let _ = h.frame(&mut app, vec![Event::PointerMoved(at)]);
        times.push(app.snapshot().render.playhead.t);
    }
    let _ = h.frame(&mut app, vec![button(from + egui::vec2(80.0, 0.0), false)]);
    let _ = h.frame(&mut app, Vec::new());
    let check_scrub = |times: &[f64]| {
        assert!(
            times.windows(2).all(|w| w[1] > w[0]) && times[0] > 0.0 && times[0] < t_max,
            "the playhead follows the scrub every frame: {times:?}"
        );
    };
    check_scrub(&times);
    rejects("a scrubber that applies only on release", || {
        check_scrub(&[0.0; FRAMES])
    });
    check_one_entry(
        "the scrubber",
        depth0,
        app.snapshot().history.undo_depth,
        &lines(&app, rows),
    );
    // The figure's pan: z₀ follows every frame, one entry and one line.
    let figure = Layout::new(h.screen(), crate::capture::PIXELS_PER_POINT).figure;
    let (depth0, rows) = (app.snapshot().history.undo_depth, app.console().len());
    let from = figure.center();
    let _ = h.frame(&mut app, vec![Event::PointerMoved(from)]);
    let _ = h.frame(&mut app, vec![button(from, true)]);
    let mut centres: Vec<Latent> = Vec::new();
    for k in 1..=FRAMES {
        let _ = h.frame(
            &mut app,
            vec![Event::PointerMoved(from + egui::vec2(8.0 * k as f32, 0.0))],
        );
        centres.push(held(&app).z0);
        assert_eq!(
            app.snapshot().history.undo_depth,
            depth0 + 1,
            "the pan's frame {k}: one entry"
        );
    }
    let _ = h.frame(&mut app, vec![button(from + egui::vec2(80.0, 0.0), false)]);
    let _ = h.frame(&mut app, Vec::new());
    assert!(
        centres.windows(2).all(|w| w[0] != w[1]),
        "the pan moves z₀ every frame"
    );
    let pan = (app.snapshot().history.undo_depth, lines(&app, rows));
    check_one_entry("the pan", depth0, pan.0, &pan.1);
    rejects("a pan of an entry per frame", || {
        check_one_entry("the pan", depth0, depth0 + FRAMES as u32, &pan.1)
    });
    // The Slice centre slider and τ₁: [`check_slider`] holds each to one entry and one line.
    for (id, range) in [(SLIDERS[SLICE_AXIS], (-1.0, 1.0)), (TAU1, (-90.0, 90.0))] {
        let t = slider_drag(id, range, 0.0, 6.0, Touch::Tilt);
        check_one_entry(id, t.depth0, t.end_depth, &t.lines);
    }
}
