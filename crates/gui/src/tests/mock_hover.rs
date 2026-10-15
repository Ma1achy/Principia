//! `mock_hover` (REQ-GUI-157's Trajectory panel frame; render_gui_spec §G2, §G3; R-101): the cursor over the figure is
//! a no-history `SetField` on the hover, and the snapshot carries the mock's sample there; the label beside the cursor
//! reads it; the mock draws the hover path into its stand-in, solid near the slice and faint away from it; the
//! Trajectory panel's "under cursor" tab shows the same sample; moving off the figure clears them all. The Figure's
//! Space keeps the orbit and L toggles listen; the panel's sphere turns at its proposed rate by the time source's
//! seconds, the wall clock's, never by frames. Each check also runs on an input it must reject (R-176, through `rejects`).

use eframe::egui::{self, pos2, vec2, Event, Key, Modifiers, Rect};
use engine::contract::sample::Sample;
use engine::contract::view_ui::{KeptOrbits, KeyMode};

use super::support::{frame_texts, headless, mock_app, press, rejects, shapes};
use crate::app::App;
use crate::capture::{hover_point, PIXELS_PER_POINT};
use crate::explore::hover_label::{self, figure_point, placed, LISTEN_HINT, OFFSET};
use crate::explore::trajectory::{
    self, activate, adjust, fit, index_at, join_kept, keep, kept_row, on_sphere, remove, shown,
    summary, turned, unwrapped_point, view, wraps, Scratch, Tab, KEEP, KEPT, KEPT_IDS, LISTEN,
    MAX_KEPT, NO_SAMPLE, PLAYHEAD, TRAJECTORY, TURN_DEG_PER_S, UNDER_CURSOR,
};
use crate::explore::FIGURE;
use crate::headless::{Headless, FRAME_S};
use crate::keyboard::keymap::Direction;
use crate::keyboard::scopes::{Adjust, ScopeTree, StepKind};
use crate::layout::Layout;
use crate::mock::sample::{alpha, hover_path, sample, FADE, FAINT, NEAR};
use crate::mock::{MockEngine, MOCK_Q1, MOCK_Q2, MOCK_T_MAX, MOCK_Z0};
use crate::side::{EngineSide, MockSide};

const NONE: Modifiers = Modifiers::NONE;

/// The app with the pointer moved over the figure at the capture's hover point, after the frames that send the hover
/// and read the sample back.
fn hovering() -> (App<MockSide>, Headless, egui::Pos2) {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    let layout = Layout::new(h.screen(), PIXELS_PER_POINT);
    let at = hover_point(layout.figure);
    let _ = h.frame(&mut app, vec![Event::PointerMoved(at)]);
    for _ in 0..2 {
        let _ = h.frame(&mut app, Vec::new());
    }
    (app, h, at)
}

/// The hover's `SetField` lines of the console.
fn hover_lines(app: &App<MockSide>) -> Vec<String> {
    app.console()
        .iter()
        .map(|e| e.message.clone())
        .filter(|m| m.starts_with("SetField Hover.at"))
        .collect()
}

/// The text shapes of one frame.
fn painted_texts(h: &mut Headless, app: &mut App<MockSide>) -> Vec<String> {
    shapes(h, app)
        .into_iter()
        .filter_map(|s| match s {
            egui::Shape::Text(t) => Some(t.galley.text().to_owned()),
            _ => None,
        })
        .collect()
}

/// The mock's plane.
fn plane() -> engine::contract::sim_config::Plane {
    engine::contract::sim_config::Plane {
        z0: MOCK_Z0,
        q1: MOCK_Q1,
        q2: MOCK_Q2,
    }
}

// --- The hover: a no-history SetField, the sample in the snapshot -----------------------------------------------------

#[test]
fn mock_hover_is_a_no_history_setfield_and_the_snapshot_carries_the_sample() {
    let (mut app, mut h, _) = hovering();
    let snapshot = app.snapshot().clone();
    let at = snapshot.render.hover.at.expect("the hover set");
    assert!(
        (at[0] - 0.57).abs() < 1e-3 && (at[1] - 0.55).abs() < 1e-3,
        "{at:?}"
    );
    assert_eq!(
        snapshot.history.undo_depth, 0,
        "the hover enters no history"
    );
    let sample = snapshot
        .sample
        .clone()
        .expect("the sample under the cursor");
    assert_eq!(sample.at, at);
    assert_eq!(
        sample,
        crate::mock::sample::sample(&snapshot.sim.plane, at, 0.0, MOCK_T_MAX)
    );
    let lines = hover_lines(&app);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].ends_with("(no history)"), "{}", lines[0]);
    // An unmoved pointer sends nothing more.
    for _ in 0..3 {
        let _ = h.frame(&mut app, Vec::new());
    }
    assert_eq!(hover_lines(&app).len(), 1);
    rejects("a hover with history", || {
        assert_eq!(snapshot.history.undo_depth, 1)
    });
    // Over a panel, not the figure: no hover.
    let layout = Layout::new(h.screen(), PIXELS_PER_POINT);
    let _ = h.frame(&mut app, vec![Event::PointerMoved(layout.time.center())]);
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(app.snapshot().render.hover.at, None);
    assert!(app.snapshot().sample.is_none());
}

#[test]
fn mock_hover_label_reads_the_sample() {
    let (mut app, mut h, _) = hovering();
    let sample = app.snapshot().sample.clone().unwrap();
    let want = hover_label::text(&sample, 0.0, MOCK_T_MAX);
    let painted = painted_texts(&mut h, &mut app);
    assert!(painted.contains(&want), "the label {want:?} in {painted:?}");
    rejects("a label of another time", || {
        assert!(painted.contains(&hover_label::text(&sample, 7.0, 40.0)))
    });
    // Its wordings: undecided, decided with its t_end, and a fate with no t_end.
    let mut s = sample.clone();
    s.fate = None;
    s.t_end = None;
    assert_eq!(
        hover_label::text(&s, 12.4, 50.0),
        format!("undecided · still interacting at t 12.4 of 50 · {LISTEN_HINT}")
    );
    s.fate = Some("body 1 escape".to_owned());
    s.t_end = Some(31.2);
    assert_eq!(
        hover_label::text(&s, 40.0, 50.0),
        format!("body 1 escape · t_end 31.2 · {LISTEN_HINT}")
    );
    s.t_end = None;
    assert_eq!(
        hover_label::text(&s, 40.0, 50.0),
        format!("body 1 escape · {LISTEN_HINT}")
    );
}

#[test]
fn mock_hover_sample_decides_at_its_t_end() {
    let p = plane();
    // Somewhere on the figure a sample decides late: undecided before its end, decided from it on.
    let late = (0..200)
        .map(|i| [0.005 * f64::from(i), 0.5])
        .map(|at| (at, sample(&p, at, MOCK_T_MAX, MOCK_T_MAX)))
        .find(|(_, s)| s.t_end.is_some_and(|e| e > 1.0 && e < MOCK_T_MAX))
        .expect("a sample with an inner t_end");
    let end = late.1.t_end.unwrap();
    let before = sample(&p, late.0, end - 1e-9, MOCK_T_MAX);
    let at_end = sample(&p, late.0, end, MOCK_T_MAX);
    assert_eq!((before.fate.is_none(), before.t_end), (true, None));
    assert_eq!(at_end.t_end, Some(end));
    assert!(at_end.fate.is_some());
    // The same IC gives the same sample; another IC another.
    assert_eq!(
        sample(&p, late.0, 3.0, MOCK_T_MAX),
        sample(&p, late.0, 3.0, MOCK_T_MAX)
    );
    rejects("a sample decided before its end", || {
        assert!(before.fate.is_some())
    });
    // Its paths: one shape point per body point, inside the sphere's ranges.
    assert_eq!(at_end.shape.len(), at_end.bodies[0].len());
    for [theta, phi] in &at_end.shape {
        assert!(
            (0.0..std::f64::consts::TAU).contains(theta)
                && (0.0..=std::f64::consts::PI).contains(phi)
        );
    }
}

// --- The hover path: drawn by the mock, solid near the slice, faint away ----------------------------------------------

#[test]
fn mock_hover_path_is_drawn_by_the_mock_solid_and_faint() {
    let (mut app, _, _) = hovering();
    let marks = app.side().engine().marks();
    assert!(!marks.is_empty(), "the mock draws the path");
    let solid = marks.iter().filter(|m| m.rgba[3] >= 0.99).count();
    // Faded: past half way from solid to FAINT.
    let faint = marks
        .iter()
        .filter(|m| m.rgba[3] < (1.0 + FAINT) / 2.0)
        .count();
    assert!(solid > 5 && faint > 5, "solid {solid}, faint {faint}");
    rejects("a path all one opacity", || assert_eq!(faint, 0));
    // The fall-off at its edges.
    assert_eq!(alpha(0.0, 1.0), 1.0);
    assert_eq!(alpha(NEAR, 1.0), 1.0);
    assert!((alpha(NEAR + FADE, 1.0) - FAINT).abs() < 1e-6);
    assert!((alpha(NEAR + FADE / 2.0, 1.0) - (1.0 + FAINT) / 2.0).abs() < 1e-6);
    assert!(
        (alpha(10.0, 1.0) - FAINT).abs() < 1e-6,
        "no fainter than FAINT"
    );
    assert_eq!(alpha(2.0 * NEAR, 2.0), 1.0, "by the view's half-span");
    // A degenerate basis has no path.
    let mut flat = plane();
    flat.q2 = [0.0; 8];
    assert!(hover_path(&flat, [0.5, 0.5]).is_empty());
    let mut parallel = plane();
    parallel.q2 = parallel.q1;
    assert!(hover_path(&parallel, [0.5, 0.5]).is_empty());
    assert!(crate::mock::sample::out_of_plane(&parallel).is_none());
    // The path starts at the IC.
    let path = hover_path(&plane(), [0.3, 0.6]);
    assert!((path[0].at[0] - 0.3).abs() < 1e-9 && (path[0].at[1] - 0.6).abs() < 1e-9);
    // Off the figure the mock draws no path.
    let mut engine = MockEngine::new();
    assert!(engine.marks().is_empty());
    engine::contract::interface::EngineInterface::set_field(
        &mut engine,
        hover_label::hover_edit(Some([0.5, 0.5])),
    );
    assert!(!engine.marks().is_empty());
}

// --- The Trajectory panel: under cursor shows the same sample; moving off clears it all -------------------------------

#[test]
fn mock_hover_under_cursor_shows_the_same_sample_and_leaving_clears_it() {
    let (mut app, mut h, _) = hovering();
    let sample = app.snapshot().sample.clone().unwrap();
    let texts = frame_texts(&mut h, &mut app);
    assert!(
        texts.contains(&summary(&sample)),
        "the panel's summary: {texts:?}"
    );
    assert!(texts.contains(&sample.f2_word), "its F₂ word");
    assert!(!texts.iter().any(|t| t == NO_SAMPLE));
    // Leaving: the hover, the sample, the label, the path and the panel's orbit, all gone.
    let _ = h.frame(&mut app, vec![Event::PointerGone]);
    for _ in 0..2 {
        let _ = h.frame(&mut app, Vec::new());
    }
    let snapshot = app.snapshot().clone();
    assert_eq!(snapshot.render.hover.at, None);
    assert!(snapshot.sample.is_none());
    assert!(app.side().engine().marks().is_empty(), "no path");
    let texts = frame_texts(&mut h, &mut app);
    assert!(texts.iter().any(|t| t == NO_SAMPLE), "{texts:?}");
    let painted = painted_texts(&mut h, &mut app);
    assert!(
        !painted.iter().any(|t| t.ends_with(LISTEN_HINT)),
        "no label: {painted:?}"
    );
    assert_eq!(
        hover_lines(&app).len(),
        2,
        "on and off: two no-history SetFields"
    );
    rejects("a panel that keeps the orbit", || {
        assert!(texts.contains(&summary(&sample)))
    });
    // Hidden by F3, the hover goes too.
    let (mut app, mut h, _) = hovering();
    press(&mut h, &mut app, Key::F3, NONE);
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(app.snapshot().render.hover.at, None);
}

#[test]
fn mock_hover_figure_point_and_label_place() {
    let figure = Rect::from_min_size(pos2(100.0, 50.0), vec2(400.0, 400.0));
    assert_eq!(figure_point(figure, None, true), None);
    assert_eq!(
        figure_point(figure, Some(pos2(300.0, 250.0)), false),
        None,
        "another layer over it"
    );
    assert_eq!(
        figure_point(figure, Some(pos2(99.0, 250.0)), true),
        None,
        "left of it"
    );
    assert_eq!(
        figure_point(figure, Some(pos2(300.0, 250.0)), true),
        Some([0.5, 0.5])
    );
    assert_eq!(
        figure_point(figure, Some(pos2(100.0, 450.0)), true),
        Some([0.0, 0.0]),
        "its corner"
    );
    rejects("a point off the figure", || {
        assert!(figure_point(figure, Some(pos2(501.0, 250.0)), true).is_some())
    });
    // The label: OFFSET from the cursor, kept inside the figure at each edge.
    let size = vec2(120.0, 30.0);
    let mid = placed(figure, pos2(300.0, 250.0), size);
    assert_eq!(mid.min, pos2(300.0, 250.0) + OFFSET - vec2(0.0, 30.0));
    let right = placed(figure, pos2(495.0, 250.0), size);
    assert_eq!(right.max.x, figure.max.x);
    let top = placed(figure, pos2(300.0, 52.0), size);
    assert_eq!(top.min.y, figure.min.y);
    let huge = placed(figure, pos2(300.0, 250.0), vec2(500.0, 500.0));
    assert_eq!(
        huge.min, figure.min,
        "larger than the figure: at its corner"
    );
}

// --- Keep and listen: the Figure's Space and L ------------------------------------------------------------------------

#[test]
fn mock_hover_space_keeps_and_l_listens_in_the_figure() {
    let (mut app, mut h, _) = hovering();
    let sample = app.snapshot().sample.clone().unwrap();
    for mode in [KeyMode::Navigation, KeyMode::Interaction] {
        app.view.focus.path = vec![FIGURE.to_owned()];
        app.view.focus.mode = mode;
        press(&mut h, &mut app, Key::Space, NONE);
    }
    assert_eq!(
        app.view.kept_orbits.orbits,
        [sample.clone(), sample.clone()]
    );
    let texts = frame_texts(&mut h, &mut app);
    assert!(
        texts.iter().any(|t| t == "#1") && texts.iter().any(|t| t == "#2"),
        "the tabs: {texts:?}"
    );
    assert!(texts.contains(&kept_row(0, &sample)));
    let listen = app.trajectory.listen;
    press(&mut h, &mut app, Key::L, NONE);
    assert_eq!(app.trajectory.listen, !listen);
    assert!(
        !app.view.transport.playing,
        "Space in the Figure plays nothing"
    );
    // Outside the Figure, neither.
    app.view.focus.path = vec![TRAJECTORY.to_owned()];
    press(&mut h, &mut app, Key::Space, NONE);
    press(&mut h, &mut app, Key::L, NONE);
    assert_eq!(app.view.kept_orbits.orbits.len(), 2);
    assert_eq!(app.trajectory.listen, !listen);
    rejects("a Space that kept nothing", || {
        assert!(app.view.kept_orbits.orbits.is_empty())
    });
    // The kept orbits join the keys: each with its show and remove.
    let tree = app.keyboard.tree(engine::contract::view_ui::Mode::Explore);
    assert_eq!(tree.children(Some(KEPT)), [KEPT_IDS[0].0, KEPT_IDS[1].0]);
    assert_eq!(
        tree.children(Some(KEPT_IDS[1].0)),
        [KEPT_IDS[1].1, KEPT_IDS[1].2]
    );
}

// --- The panel's state: the tabs, the kept list, the turn -------------------------------------------------------------

/// A sample marked by `n`, for the kept list.
fn marked(n: f64) -> Sample {
    let mut s = sample(&plane(), [0.4, 0.4], 0.0, MOCK_T_MAX);
    s.ftle = n;
    s
}

#[test]
fn mock_hover_kept_orbits_and_their_tabs() {
    let mut kept = KeptOrbits::default();
    let mut scratch = Scratch::default();
    for n in 0..MAX_KEPT {
        keep(&marked(n as f64), &mut kept, &mut scratch);
    }
    assert_eq!(kept.orbits.len(), MAX_KEPT, "not yet full");
    assert_eq!(kept.orbits[0].ftle, 0.0, "none dropped yet");
    // Keeping one more drops the oldest, and the tab stays on the orbit it showed.
    scratch.tab = Tab::Kept(4);
    keep(&marked(MAX_KEPT as f64), &mut kept, &mut scratch);
    assert_eq!(kept.orbits.len(), MAX_KEPT, "the oldest dropped");
    assert_eq!(kept.orbits[0].ftle, 1.0);
    assert_eq!(scratch.tab, Tab::Kept(3));
    assert_eq!(shown(scratch.tab, None, &kept).map(|s| s.ftle), Some(4.0));
    rejects("a list past its most", || {
        assert_eq!(kept.orbits.len(), MAX_KEPT + 1)
    });
    rejects("a tab left where the orbits moved under it", || {
        assert_eq!(shown(Tab::Kept(4), None, &kept).map(|s| s.ftle), Some(4.0))
    });
    // A tab on the dropped orbit goes back to under cursor.
    scratch.tab = Tab::Kept(0);
    keep(&marked(99.0), &mut kept, &mut scratch);
    assert_eq!(scratch.tab, Tab::UnderCursor);
    assert_eq!(kept.orbits[0].ftle, 2.0);
    // Remove: the tab on it goes back to under cursor, one after it moves up one, one before it stays.
    scratch.tab = Tab::Kept(3);
    remove(3, &mut kept, &mut scratch);
    assert_eq!(scratch.tab, Tab::UnderCursor);
    scratch.tab = Tab::Kept(4);
    remove(2, &mut kept, &mut scratch);
    assert_eq!(scratch.tab, Tab::Kept(3));
    scratch.tab = Tab::Kept(1);
    remove(2, &mut kept, &mut scratch);
    assert_eq!(scratch.tab, Tab::Kept(1));
    let n = kept.orbits.len();
    remove(n, &mut kept, &mut scratch);
    assert_eq!(kept.orbits.len(), n, "past the end, nothing");
    // shown: the sample under the cursor, or the kept one.
    let s = marked(42.0);
    assert_eq!(shown(Tab::UnderCursor, Some(&s), &kept), Some(&s));
    assert_eq!(shown(Tab::Kept(0), Some(&s), &kept), kept.orbits.first());
    assert_eq!(shown(Tab::Kept(99), Some(&s), &kept), None);
    // activate: each control.
    let mut scratch = Scratch::default();
    let mut list = KeptOrbits::default();
    activate(KEEP, None, &mut list, &mut scratch);
    assert!(list.orbits.is_empty(), "+ with no sample keeps nothing");
    activate(KEEP, Some(&s), &mut list, &mut scratch);
    assert_eq!(list.orbits.len(), 1);
    activate(KEPT_IDS[0].1, None, &mut list, &mut scratch);
    assert_eq!(scratch.tab, Tab::Kept(0));
    activate(KEPT_IDS[1].1, None, &mut list, &mut scratch);
    assert_eq!(scratch.tab, Tab::Kept(0), "no second orbit to show");
    activate(UNDER_CURSOR, None, &mut list, &mut scratch);
    assert_eq!(scratch.tab, Tab::UnderCursor);
    for (id, get) in [
        (
            trajectory::UNWRAPPED,
            (|s: &Scratch| s.unwrapped) as fn(&Scratch) -> bool,
        ),
        (trajectory::TURN, |s| !s.turn),
        (trajectory::PLAY, |s| s.playing),
        (LISTEN, |s| s.listen),
        (trajectory::FOLLOW, |s| !s.follow),
    ] {
        let mut scratch = Scratch::default();
        activate(id, None, &mut list, &mut scratch);
        assert!(get(&scratch), "{id}");
    }
    let mut scratch = Scratch {
        unwrapped: true,
        ..Scratch::default()
    };
    activate(trajectory::SPHERE, None, &mut list, &mut scratch);
    assert!(!scratch.unwrapped);
    activate(KEPT_IDS[0].2, None, &mut list, &mut scratch);
    assert!(list.orbits.is_empty(), "× removes it");
    let before = scratch;
    activate("nowhere", None, &mut list, &mut scratch);
    assert_eq!(scratch, before);
    // The rows and the summary.
    let mut s = marked(0.5);
    s.fate = None;
    s.t_end = None;
    assert!(summary(&s).starts_with("undecided · FTLE 0.500"));
    assert_eq!(kept_row(0, &s), format!("#1 undecided · t {:.1}", s.path_t));
    s.fate = Some("bounded".into());
    s.t_end = Some(50.0);
    assert!(summary(&s).starts_with("bounded · t_end 50.0 · FTLE"));
    assert_eq!(kept_row(2, &s), "#3 bounded · t 50.0");
    s.t_end = None;
    assert!(summary(&s).starts_with("bounded · FTLE"));
}

#[test]
fn mock_hover_sphere_turns_by_seconds_not_frames() {
    assert_eq!(turned(350.0, TURN_DEG_PER_S, 1.0), 2.0);
    assert_eq!(turned(0.0, TURN_DEG_PER_S, 0.0), 0.0);
    // Sixty frames of a sixtieth of a second turn as far as one frame of a second.
    let mut fine = Scratch::default();
    let mut coarse = Scratch::default();
    for i in 0..=60 {
        fine.advance(f64::from(i) / 60.0, 10.0);
    }
    coarse.advance(0.0, 10.0);
    coarse.advance(1.0, 10.0);
    assert!(
        (fine.angle - coarse.angle).abs() < 1e-9,
        "{} {}",
        fine.angle,
        coarse.angle
    );
    assert!((coarse.angle - TURN_DEG_PER_S).abs() < 1e-9);
    rejects("a turn by frames", || {
        assert!((fine.angle - 60.0 * coarse.angle).abs() < 1e-9)
    });
    // A step back in time turns nothing; nor does turn off, nor the unwrapped view.
    coarse.advance(0.5, 10.0);
    assert!((coarse.angle - TURN_DEG_PER_S).abs() < 1e-9);
    for scratch in [
        Scratch {
            turn: false,
            ..Scratch::default()
        },
        Scratch {
            unwrapped: true,
            ..Scratch::default()
        },
    ] {
        let mut s = scratch;
        s.advance(0.0, 10.0);
        s.advance(5.0, 10.0);
        assert_eq!(s.angle, 0.0);
        assert!(!s.moving(true));
    }
    // The orbit's playhead plays to its span and stops there.
    let mut s = Scratch {
        playing: true,
        ..Scratch::default()
    };
    s.advance(0.0, 2.0);
    s.advance(1.5, 2.0);
    assert_eq!((s.orbit_t, s.playing), (1.5, true));
    s.advance(2.0, 2.0);
    assert_eq!((s.orbit_t, s.playing), (2.0, false));
    // It asks for frames only with an orbit shown, turning or playing.
    let turning = Scratch::default();
    assert!(turning.moving(true));
    assert!(!turning.moving(false));
    let playing_flat = Scratch {
        unwrapped: true,
        playing: true,
        ..Scratch::default()
    };
    assert!(playing_flat.moving(true));
    // In the app: the turn reads the wall clock (the frame's time), so on the running mock it turns with an orbit shown
    // by the frames' seconds times the rate, never by the mock's ticks.
    let (mut app, mut h, _) = hovering();
    let a0 = app.trajectory.angle;
    for _ in 0..30 {
        let _ = h.frame(&mut app, Vec::new());
    }
    let a1 = app.trajectory.angle;
    assert!(
        (a1 - a0 - 30.0 * FRAME_S * TURN_DEG_PER_S).abs() < 1e-6,
        "{a0} → {a1}"
    );
    rejects("a turn by the mock's ticks", || {
        assert!((a1 - a0 - 30.0 / 60.0 * TURN_DEG_PER_S).abs() < 1e-6)
    });
}

/// The angle the sphere turns through on the running mock, the pointer over the figure, in one second of wall time
/// run as `frames` frames.
fn turn_in_one_second(frames: u32) -> f64 {
    let mut app = mock_app();
    let mut h = headless();
    h.set_frame_step(1.0 / f64::from(frames));
    let _ = h.frame(&mut app, Vec::new());
    let at = hover_point(Layout::new(h.screen(), PIXELS_PER_POINT).figure);
    let _ = h.frame(&mut app, vec![Event::PointerMoved(at)]);
    // Frames until the snapshot, read every snapshot interval, carries the sample under the cursor.
    for _ in 0..frames {
        let _ = h.frame(&mut app, Vec::new());
    }
    assert!(app.snapshot().sample.is_some(), "no orbit under the cursor");
    let a0 = app.trajectory.angle;
    for _ in 0..frames {
        let _ = h.frame(&mut app, Vec::new());
    }
    (app.trajectory.angle - a0).rem_euclid(360.0)
}

#[test]
fn mock_hover_sphere_turns_the_same_at_any_frame_rate() {
    // One second of wall time at 30 and at 120 frames a second turns the sphere by the rate, the same both ways.
    let slow = turn_in_one_second(30);
    let fast = turn_in_one_second(120);
    assert!((slow - TURN_DEG_PER_S).abs() < 1e-6, "30 fps: {slow}");
    assert!((fast - TURN_DEG_PER_S).abs() < 1e-6, "120 fps: {fast}");
    rejects("a turn that counts frames", || {
        assert!(
            (fast - 120.0 / 60.0 * TURN_DEG_PER_S).abs() < 1e-6,
            "{fast}"
        )
    });
    // Capture mode's frozen clock holds the sphere whatever the frames' time.
    let mut app = App::new(
        MockSide::new(MockEngine::frozen(), None),
        crate::capture::FORMAT,
    );
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    let at = hover_point(Layout::new(h.screen(), PIXELS_PER_POINT).figure);
    for _ in 0..10 {
        let _ = h.frame(&mut app, vec![Event::PointerMoved(at)]);
    }
    assert!(app.snapshot().sample.is_some(), "no orbit under the cursor");
    assert_eq!(app.trajectory.angle, 0.0);
}

/// The least angle, in degrees, between the screen directions of the sphere's x, y and z axes seen at `angle`.
fn least_axis_separation(angle: f64) -> f64 {
    let dirs: Vec<f64> = (0..3)
        .map(|k| {
            let tip = std::array::from_fn(|i| if i == k { 1.0 } else { 0.0 });
            let ([x, y], _) = view(tip, angle);
            y.atan2(x).to_degrees()
        })
        .collect();
    let mut least = f64::INFINITY;
    for (i, a) in dirs.iter().enumerate() {
        for b in &dirs[i + 1..] {
            let d = (a - b).rem_euclid(360.0);
            least = least.min(d.min(360.0 - d));
        }
    }
    least
}

#[test]
fn mock_hover_sphere_axes_show_apart_at_rest() {
    // At rest (the turn's start, and every capture) the three axes point well apart on screen, none hidden on another.
    assert_eq!(Scratch::default().angle, 0.0);
    let least = least_axis_separation(0.0);
    assert!(least > 45.0, "the axes {least}° apart at rest");
    // The resting azimuth itself: the look choice, oblique.
    assert_eq!(trajectory::REST_ANGLE_DEG, 35.0);
    rejects("a rest at azimuth 0, y on z's line", || {
        assert!(least_axis_separation(-trajectory::REST_ANGLE_DEG) > 45.0)
    });
}

#[test]
fn mock_hover_trajectory_geometry() {
    // The sphere: n = (sin φ cos θ, sin φ sin θ, cos φ).
    let close = |a: [f64; 3], b: [f64; 3]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-12);
    assert!(
        close(on_sphere([0.0, 0.0]), [0.0, 0.0, 1.0]),
        "φ 0 is the pole"
    );
    assert!(close(
        on_sphere([0.0, std::f64::consts::FRAC_PI_2]),
        [1.0, 0.0, 0.0]
    ));
    assert!(close(
        on_sphere([std::f64::consts::FRAC_PI_2, std::f64::consts::FRAC_PI_2]),
        [0.0, 1.0, 0.0]
    ));
    rejects("θ as the polar angle", || {
        assert!(close(
            on_sphere([0.0, std::f64::consts::FRAC_PI_2]),
            [0.0, 0.0, 1.0]
        ))
    });
    // Seen with the pole tilted toward the viewer: the pole up and near, and a turn moves x into y.
    let ([x, y], depth) = view([0.0, 0.0, 1.0], 0.0);
    assert!(x.abs() < 1e-12 && y > 0.9 && depth > 0.0);
    let (a, _) = view([1.0, 0.0, 0.0], 90.0);
    let (b, _) = view([0.0, 1.0, 0.0], 0.0);
    assert!((a[0] - b[0]).abs() < 1e-12 && (a[1] - b[1]).abs() < 1e-12);
    // Unwrapped: θ across, φ down.
    let r = Rect::from_min_size(pos2(0.0, 0.0), vec2(200.0, 100.0));
    assert_eq!(unwrapped_point(r, [0.0, 0.0]), pos2(0.0, 0.0));
    assert_eq!(
        unwrapped_point(r, [std::f64::consts::TAU, std::f64::consts::PI]),
        pos2(200.0, 100.0)
    );
    assert_eq!(unwrapped_point(r, [-1.0, 9.0]), pos2(0.0, 100.0), "clamped");
    let pi = std::f64::consts::PI;
    assert!(!wraps([0.0, 1.0], [pi, 1.0]), "exactly π does not wrap");
    assert!(wraps([0.0, 1.0], [pi + 1e-9, 1.0]));
    // The playhead's index.
    assert_eq!(index_at(1.0, 2.0, 0), 0);
    assert_eq!(index_at(1.0, 0.0, 10), 0);
    assert_eq!(index_at(0.0, 2.0, 11), 0);
    assert_eq!(index_at(1.0, 2.0, 11), 5);
    assert_eq!(index_at(2.0, 2.0, 11), 10);
    assert_eq!(index_at(9.0, 2.0, 11), 10);
    // Real space's fit: square, about the bounds, a little larger; none, the unit square.
    assert_eq!(fit(&[]), [-1.0, -1.0, 1.0, 1.0]);
    let f = fit(&[vec![[0.0, 0.0], [2.0, 1.0]]]);
    assert!((f[2] - f[0] - (f[3] - f[1])).abs() < 1e-12, "square");
    assert!((f[2] - f[0] - 2.2).abs() < 1e-12 && (f[0] + f[2] - 2.0).abs() < 1e-12);
}

#[test]
fn mock_hover_trajectory_scopes_and_the_playhead() {
    let mut tree = ScopeTree::new();
    trajectory::register(&mut tree);
    assert_eq!(
        tree.children(Some(TRAJECTORY)),
        [
            UNDER_CURSOR,
            KEEP,
            trajectory::SPHERE,
            trajectory::UNWRAPPED,
            trajectory::TURN,
            trajectory::PLAY,
            PLAYHEAD,
            LISTEN,
            trajectory::FOLLOW,
            trajectory::VIEWER,
            trajectory::INSPECTOR,
            KEPT
        ]
    );
    assert!(
        tree.get(trajectory::VIEWER).unwrap().activates,
        "Open full viewer… opens the Inspector (render_gui_spec §G2)"
    );
    assert!(
        tree.get(trajectory::INSPECTOR).unwrap().activates,
        "IC Inspector… opens the Inspector (TASK-M6-28)"
    );
    // The kept orbits come and go; a second join with the same count changes nothing.
    join_kept(&mut tree, 3);
    assert_eq!(tree.children(Some(KEPT)).len(), 3);
    join_kept(&mut tree, 3);
    assert_eq!(tree.children(Some(KEPT)).len(), 3);
    join_kept(&mut tree, 1);
    assert_eq!(tree.children(Some(KEPT)), [KEPT_IDS[0].0]);
    assert!(
        tree.get(KEPT_IDS[1].1).is_none(),
        "the removed orbit's show is gone"
    );
    assert_eq!(
        tree.children(Some(TRAJECTORY)).last(),
        Some(&KEPT),
        "the panel's other scopes stay"
    );
    join_kept(&mut tree, 0);
    assert!(tree.children(Some(KEPT)).is_empty());
    rejects("an orbit left behind", || {
        assert_eq!(tree.children(Some(KEPT)), [KEPT_IDS[0].0])
    });
    // The playhead: Bounded, 1/100 of the orbit's span, kept in it.
    let step = |times: f64, from: f64| {
        let mut s = Scratch {
            orbit_t: from,
            ..Scratch::default()
        };
        let a = Adjust {
            scope: PLAYHEAD,
            kind: StepKind::Bounded,
            times,
            direction: Direction::Right,
        };
        adjust(&a, 20.0, &mut s);
        s.orbit_t
    };
    assert!((step(1.0, 1.0) - 1.2).abs() < 1e-12);
    assert_eq!(step(1.0, 20.0), 20.0);
    assert_eq!(step(-1.0, 0.0), 0.0);
    let mut s = Scratch::default();
    let other = Adjust {
        scope: TRAJECTORY,
        kind: StepKind::Bounded,
        times: 1.0,
        direction: Direction::Right,
    };
    adjust(&other, 20.0, &mut s);
    assert_eq!(s.orbit_t, 0.0, "another scope's arrows move nothing");
    let pan = Adjust {
        scope: PLAYHEAD,
        kind: StepKind::Pan,
        times: 1.0,
        direction: Direction::Right,
    };
    adjust(&pan, 20.0, &mut s);
    assert_eq!(s.orbit_t, 0.0, "a step not of the range moves nothing");
}

#[test]
fn mock_hover_remove_children_keeps_the_rest() {
    let mut tree = ScopeTree::new();
    tree.register(None, crate::keyboard::scopes::Scope::group("a", "A"));
    tree.register(Some("a"), crate::keyboard::scopes::Scope::group("b", "B"));
    tree.register(Some("b"), crate::keyboard::scopes::Scope::control("c", "C"));
    tree.register(None, crate::keyboard::scopes::Scope::group("d", "D"));
    tree.register(Some("d"), crate::keyboard::scopes::Scope::control("e", "E"));
    tree.remove_children("a");
    assert!(
        tree.get("b").is_none() && tree.get("c").is_none(),
        "grandchildren too"
    );
    assert_eq!(tree.children(None), ["a", "d"]);
    assert_eq!(
        tree.children(Some("d")),
        ["e"],
        "the later scopes keep their parents"
    );
    tree.remove_children("nowhere");
    assert_eq!(tree.children(None), ["a", "d"]);
    rejects("a parent removed with its children", || {
        assert!(tree.get("a").is_none())
    });
}
