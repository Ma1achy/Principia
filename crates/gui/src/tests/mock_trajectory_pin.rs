//! `mock_trajectory_pin` (REQ-GUI-201; R-453 D1 to D3, D5, D6; render_gui_spec §G2's Trajectory panel, §G3's Figure
//! keys): a primary click on the figure with no drag pins that sample's trajectory in the Trajectory panel, its first
//! tab reading "pinned" with an unpin button; hover elsewhere, the pointer entering the panel and leaving the window
//! keep it; a pan, a zoom and a slice edit keep it, the pin holding the IC, not a pixel; a horizon edit re-integrates
//! it; another click moves it; unpin follows the cursor again; unpinned, the panel keeps the last sample hovered; a
//! drag pans and does not pin; P over the figure pins in either keyboard mode, and off it does nothing; the figure's
//! path and hover label follow the pointer, pinned or not; `+` with the mouse and Space keep the shown sample; and
//! nothing enters undo. Each check also runs on an input it must reject (R-176, through `rejects`).

use eframe::egui::{self, vec2, Event, Key, Modifiers, PointerButton, Pos2};
use engine::contract::sample::Sample;
use engine::contract::set_field::{Edit, SetField, SimField};
use engine::contract::sim_config::{Horizon, Plane};
use engine::contract::view_ui::{KeyMode, Mode};

use super::support::{click, frame_texts, headless, keys_on, mock_app, press, rejects, shapes};
use crate::app::App;
use crate::capture::PIXELS_PER_POINT;
use crate::explore::hover_label;
use crate::explore::manifold_view::{basis_edit, navigate::figure_coords, z0_edit};
use crate::explore::trajectory::{
    self, join, summary, Inputs, Scratch, Tab, FOLLOWING, KEEP, NO_SAMPLE, PINNED, TRAJECTORY,
    UNDER_CURSOR, UNPIN, UNPIN_LABEL,
};
use crate::explore::FIGURE;
use crate::headless::Headless;
use crate::keyboard::scopes::{Scope, ScopeTree};
use crate::layout::Layout;
use crate::mock::sample::{hover_path, integrate, sample};
use crate::mock::MOCK_T_MAX;
use crate::side::{EngineSide, MockSide};

const NONE: Modifiers = Modifiers::NONE;

/// The app on the mock, one frame run.
fn app() -> (App<MockSide>, Headless, Layout) {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    let layout = Layout::new(h.screen(), PIXELS_PER_POINT);
    (app, h, layout)
}

/// The point of `figure` at `(s, t)`, `t` up.
fn at(figure: egui::Rect, s: f32, t: f32) -> Pos2 {
    egui::pos2(
        figure.min.x + figure.width() * s,
        figure.max.y - figure.height() * t,
    )
}

/// The figure point `(s, t)` the app reads at `pos`.
fn coords(figure: egui::Rect, pos: Pos2) -> [f64; 2] {
    let (s, t) = figure_coords(figure, pos);
    [s, t]
}

/// The pointer moved to `pos`, then two frames, so the hover is sent and its sample read back.
fn point_at(h: &mut Headless, app: &mut App<MockSide>, pos: Pos2) {
    let _ = h.frame(app, vec![Event::PointerMoved(pos)]);
    for _ in 0..2 {
        let _ = h.frame(app, Vec::new());
    }
}

/// A primary click at `pos`, then two frames.
fn click_at(h: &mut Headless, app: &mut App<MockSide>, pos: Pos2) {
    for events in crate::capture::click(pos) {
        let _ = h.frame(app, events);
    }
    for _ in 0..2 {
        let _ = h.frame(app, Vec::new());
    }
}

/// The mock's sample at the figure point `at` on `plane`, at `t = 0` and the mock's horizon.
fn sample_at(plane: &Plane, at: [f64; 2]) -> Sample {
    sample(plane, at, 0.0, MOCK_T_MAX)
}

/// The pinned sample.
fn pinned(app: &App<MockSide>) -> Option<Sample> {
    app.trajectory.pin.as_ref().map(|p| p.sample.clone())
}

/// The SetField lines of the console other than the hover's.
fn edits(app: &App<MockSide>) -> Vec<String> {
    app.console()
        .iter()
        .map(|e| e.message.clone())
        .filter(|m| m.starts_with("SetField") && !m.starts_with("SetField Hover.at"))
        .collect()
}

/// The text shapes of one frame.
fn painted(h: &mut Headless, app: &mut App<MockSide>) -> Vec<String> {
    shapes(h, app)
        .into_iter()
        .filter_map(|s| match s {
            egui::Shape::Text(t) => Some(t.galley.text().to_owned()),
            _ => None,
        })
        .collect()
}

/// Whether the panel's summary line is `sample`'s, and the first tab reads `tab`.
fn panel_shows(h: &mut Headless, app: &mut App<MockSide>, sample: &Sample, tab: &str) {
    let texts = frame_texts(h, app);
    assert!(
        texts.contains(&summary(sample)),
        "the panel does not show {}: {texts:?}",
        summary(sample)
    );
    assert!(texts.iter().any(|t| t == tab), "no tab {tab:?}: {texts:?}");
}

#[test]
fn mock_trajectory_pin_a_click_pins_and_hover_never_replaces_it() {
    let (mut app, mut h, layout) = app();
    let f = layout.figure;
    let (a, b) = (at(f, 0.57, 0.55), at(f, 0.3, 0.4));
    let plane = app.snapshot().sim.plane.clone();
    point_at(&mut h, &mut app, a);
    click_at(&mut h, &mut app, a);
    let want = sample_at(&plane, coords(f, a));
    assert_eq!(
        pinned(&app).as_ref(),
        Some(&want),
        "the click pinned its sample"
    );
    assert_eq!(app.trajectory.tab, Tab::UnderCursor);
    panel_shows(&mut h, &mut app, &want, PINNED);
    assert!(frame_texts(&mut h, &mut app)
        .iter()
        .any(|t| t == UNPIN_LABEL));
    // Hovering another point: the snapshot's sample follows the pointer, the panel keeps the pin.
    point_at(&mut h, &mut app, b);
    let other = app.snapshot().sample.clone().expect("the sample under b");
    assert_eq!(other, sample_at(&plane, coords(f, b)));
    assert_ne!(other, want);
    panel_shows(&mut h, &mut app, &want, PINNED);
    let texts = frame_texts(&mut h, &mut app);
    rejects("a hover that replaces the pin", || {
        assert!(texts.contains(&summary(&other)), "{texts:?}")
    });
    // While pinned the figure's path and the hover label follow the pointer, not the pin (R-453 D4 dropped).
    let hover = app.snapshot().render.hover.at.expect("the hover at b");
    assert_eq!(hover, coords(f, b));
    let marks = app.side().engine().marks();
    let path = hover_path(&plane, hover);
    assert_eq!(marks.len(), path.len() - 1, "the path drawn is b's");
    let start = marks[0].ends[0].map(f64::from);
    assert!((start[0] - hover[0]).abs() < 1e-6 && (start[1] - hover[1]).abs() < 1e-6);
    let label = hover_label::text(&other, 0.0, MOCK_T_MAX);
    let texts = painted(&mut h, &mut app);
    assert!(texts.contains(&label), "the label is b's: {texts:?}");
    rejects("a figure path that follows the pin", || {
        let from = marks[0].ends[0].map(f64::from);
        let pin_at = want.at;
        assert!((from[0] - pin_at[0]).abs() < 1e-6 && (from[1] - pin_at[1]).abs() < 1e-6)
    });
    // The pointer into the panel, then out of the window: the pin holds.
    point_at(&mut h, &mut app, layout.trajectory.center());
    panel_shows(&mut h, &mut app, &want, PINNED);
    let _ = h.frame(&mut app, vec![Event::PointerGone]);
    let _ = h.frame(&mut app, Vec::new());
    panel_shows(&mut h, &mut app, &want, PINNED);
    // Nothing entered undo, and no SetField but the hover's.
    assert_eq!(app.snapshot().history.undo_depth, 0);
    assert!(edits(&app).is_empty(), "{:?}", edits(&app));
}

#[test]
fn mock_trajectory_pin_holds_the_ic_through_a_pan_a_zoom_and_a_slice_edit() {
    let (mut app, mut h, layout) = app();
    let f = layout.figure;
    let a = at(f, 0.57, 0.55);
    click_at(&mut h, &mut app, a);
    let want = pinned(&app).expect("pinned");
    let mut plane = app.snapshot().sim.plane.clone();
    // A pan (z₀ in the plane), a zoom (the basis) and a slice edit (z₀ out of it), each a SetField.
    let mut panned = plane.z0;
    panned[0] += 0.05;
    let mut sliced = panned;
    sliced[2] += 0.1;
    let half = (plane.q1.map(|x| x * 0.5), plane.q2.map(|x| x * 0.5));
    for edit in [z0_edit(panned), basis_edit(half), z0_edit(sliced)] {
        app.set_field(edit);
        for _ in 0..2 {
            let _ = h.frame(&mut app, Vec::new());
        }
        assert_eq!(pinned(&app).as_ref(), Some(&want), "an edit moved the pin");
    }
    panel_shows(&mut h, &mut app, &want, PINNED);
    plane = app.snapshot().sim.plane.clone();
    assert_eq!(plane.z0, sliced);
    // A pin that held the pixel would show another sample now.
    let pixel = sample_at(&plane, want.at);
    rejects("a pin that holds a pixel", || assert_eq!(pixel, want));
    // A horizon edit, a sim key's, re-integrates the pinned IC.
    app.set_field(SetField {
        edit: Edit::Sim(SimField::Horizon(Horizon { t: 120.0 })),
        no_history: false,
    });
    for _ in 0..2 {
        let _ = h.frame(&mut app, Vec::new());
    }
    let again = pinned(&app).expect("still pinned");
    assert_eq!(again, integrate(want.ic, want.at, 0.0, 120.0));
    assert_ne!(again, want, "the horizon changes the orbit");
    rejects("a pin left as it was by a horizon edit", || {
        assert_eq!(again, want)
    });
}

#[test]
fn mock_trajectory_pin_another_click_moves_it_and_unpin_follows_the_cursor() {
    let (mut app, mut h, layout) = app();
    let f = layout.figure;
    let plane = app.snapshot().sim.plane.clone();
    let (a, b, c) = (at(f, 0.57, 0.55), at(f, 0.3, 0.4), at(f, 0.7, 0.2));
    click_at(&mut h, &mut app, a);
    click_at(&mut h, &mut app, b);
    let want = sample_at(&plane, coords(f, b));
    assert_eq!(
        pinned(&app).as_ref(),
        Some(&want),
        "the second click moved it"
    );
    // A click while a kept orbit's tab shows: the pin moves and the first tab shows.
    app.trajectory.tab = Tab::Kept(0);
    click_at(&mut h, &mut app, a);
    assert_eq!(app.trajectory.tab, Tab::UnderCursor);
    // unpin, by the mouse: the first tab follows the cursor again.
    click(&mut h, &mut app, UNPIN_LABEL);
    assert!(app.trajectory.pin.is_none());
    point_at(&mut h, &mut app, c);
    let under = sample_at(&plane, coords(f, c));
    panel_shows(&mut h, &mut app, &under, FOLLOWING);
    assert!(!frame_texts(&mut h, &mut app)
        .iter()
        .any(|t| t == UNPIN_LABEL));
    rejects("an unpin that leaves the pin", || {
        assert!(app.trajectory.pin.is_some())
    });
    // unpin by Enter, as its click.
    click_at(&mut h, &mut app, a);
    assert!(app.trajectory.pin.is_some());
    app.view.focus.path = vec![TRAJECTORY.to_owned(), UNPIN.to_owned()];
    // The click hid the ring (R-464): a key shows it, and Enter then acts.
    keys_on(&mut h, &mut app);
    press(&mut h, &mut app, Key::Enter, NONE);
    assert!(app.trajectory.pin.is_none(), "Enter on unpin");
    assert_eq!(app.snapshot().history.undo_depth, 0);
}

#[test]
fn mock_trajectory_pin_unpinned_the_panel_keeps_the_last_sample() {
    let (mut app, mut h, layout) = app();
    let f = layout.figure;
    let plane = app.snapshot().sim.plane.clone();
    assert!(frame_texts(&mut h, &mut app).iter().any(|t| t == NO_SAMPLE));
    let a = at(f, 0.57, 0.55);
    point_at(&mut h, &mut app, a);
    let last = sample_at(&plane, coords(f, a));
    point_at(&mut h, &mut app, layout.trajectory.center());
    assert!(app.snapshot().sample.is_none(), "off the figure, no sample");
    panel_shows(&mut h, &mut app, &last, FOLLOWING);
    let texts = frame_texts(&mut h, &mut app);
    rejects("a panel that blanks off the figure", || {
        assert!(texts.iter().any(|t| t == NO_SAMPLE), "{texts:?}")
    });
    // Until the pointer hovers another point.
    let b = at(f, 0.3, 0.4);
    point_at(&mut h, &mut app, b);
    panel_shows(
        &mut h,
        &mut app,
        &sample_at(&plane, coords(f, b)),
        FOLLOWING,
    );
}

#[test]
fn mock_trajectory_pin_a_drag_pans_and_does_not_pin() {
    let (mut app, mut h, layout) = app();
    let a = at(layout.figure, 0.57, 0.55);
    let z0 = app.snapshot().sim.plane.z0;
    let button = |pos, pressed| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: NONE,
    };
    for events in [
        vec![Event::PointerMoved(a)],
        vec![button(a, true)],
        vec![Event::PointerMoved(a + vec2(20.0, 10.0))],
        vec![Event::PointerMoved(a + vec2(40.0, 20.0))],
        vec![button(a + vec2(40.0, 20.0), false)],
        Vec::new(),
        Vec::new(),
    ] {
        let _ = h.frame(&mut app, events);
    }
    assert_ne!(app.snapshot().sim.plane.z0, z0, "the drag panned");
    assert!(app.trajectory.pin.is_none(), "the drag pinned");
    rejects("a drag that pins", || assert!(app.trajectory.pin.is_some()));
}

/// P with the focus on the Figure in `mode` and the pointer at `pos`: the pin after.
fn p_in(mode: KeyMode, over: bool) -> Option<Sample> {
    let (mut app, mut h, layout) = app();
    for _ in 0..3 {
        press(&mut h, &mut app, Key::Tab, NONE);
    }
    if mode == KeyMode::Interaction {
        press(&mut h, &mut app, Key::Enter, NONE);
    }
    assert_eq!(app.view.focus.path, [FIGURE]);
    assert_eq!(app.view.focus.mode, mode);
    let pos = if over {
        at(layout.figure, 0.57, 0.55)
    } else {
        layout.trajectory.center()
    };
    point_at(&mut h, &mut app, pos);
    press(&mut h, &mut app, Key::P, NONE);
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(app.view.focus.mode, mode, "P moved no focus");
    assert_eq!(app.snapshot().history.undo_depth, 0);
    pinned(&app)
}

#[test]
fn mock_trajectory_pin_p_pins_over_the_figure_in_either_mode() {
    let (app, _, layout) = app();
    let plane = app.snapshot().sim.plane.clone();
    let want = sample_at(&plane, coords(layout.figure, at(layout.figure, 0.57, 0.55)));
    let nav = p_in(KeyMode::Navigation, true);
    let interact = p_in(KeyMode::Interaction, true);
    let check = |nav: &Option<Sample>, interact: &Option<Sample>| {
        assert_eq!(nav.as_ref(), Some(&want), "P in navigation mode");
        assert_eq!(interact.as_ref(), Some(&want), "P in interaction mode");
    };
    check(&nav, &interact);
    rejects("a P that acts in one mode only", || check(&nav, &None));
    // Off the figure, as K, nothing.
    let off = |p: Option<Sample>| assert_eq!(p, None, "P off the figure pinned");
    off(p_in(KeyMode::Navigation, false));
    rejects("a P that pins off the figure", || off(nav.clone()));
    // Its row in the `?` overlay, with the Figure's keys.
    let app = mock_app();
    let rows = app.keyboard.tree(Mode::Explore).shortcuts();
    assert!(rows.iter().any(|r| r.0 == "Figure: P"), "{rows:?}");
}

#[test]
fn mock_trajectory_pin_plus_and_space_keep_the_shown_sample() {
    let (mut app, mut h, layout) = app();
    let f = layout.figure;
    let plane = app.snapshot().sim.plane.clone();
    // Unpinned: + with the mouse keeps the last sample hovered, the pointer having left the figure for it.
    let a = at(f, 0.57, 0.55);
    point_at(&mut h, &mut app, a);
    click(&mut h, &mut app, "+");
    let last = sample_at(&plane, coords(f, a));
    assert_eq!(app.view.kept_orbits.orbits, std::slice::from_ref(&last));
    rejects("a + that keeps nothing", || {
        assert!(app.view.kept_orbits.orbits.is_empty())
    });
    // Pinned: + and the Figure's Space keep the pinned sample, the pointer over another point.
    let b = at(f, 0.3, 0.4);
    click_at(&mut h, &mut app, b);
    let pin = pinned(&app).expect("pinned");
    click(&mut h, &mut app, "+");
    point_at(&mut h, &mut app, a);
    app.view.focus.path = vec![FIGURE.to_owned()];
    press(&mut h, &mut app, Key::Space, NONE);
    assert_eq!(app.view.kept_orbits.orbits, [last, pin.clone(), pin]);
    assert_eq!(
        app.snapshot().history.undo_depth,
        0,
        "keeping enters no undo"
    );
}

#[test]
fn mock_trajectory_pin_state_reintegrates_only_on_changed_inputs() {
    let plane = mock_app().snapshot().sim.plane.clone();
    let s = sample_at(&plane, [0.4, 0.6]);
    let snapshot = mock_app().snapshot().clone();
    let inputs = Inputs::of(&snapshot);
    let mut scratch = Scratch {
        tab: Tab::Kept(2),
        ..Scratch::default()
    };
    scratch.pin(s.clone(), inputs.clone());
    assert_eq!(scratch.tab, Tab::UnderCursor, "a pin shows the first tab");
    assert_eq!(scratch.first(), Some(&s));
    // The same inputs: no integration.
    let mut calls = 0;
    scratch.repin(inputs.clone(), |_, _| {
        calls += 1;
        None
    });
    assert_eq!(calls, 0);
    // Changed: one, its result kept; a side with none keeps the orbit, under the new inputs.
    let mut later = inputs.clone();
    later.playhead = 3.0;
    let fresh = integrate(s.ic, s.at, 3.0, MOCK_T_MAX);
    scratch.repin(later.clone(), |ic, at| {
        calls += 1;
        Some(integrate(*ic, at, 3.0, MOCK_T_MAX))
    });
    assert_eq!((calls, scratch.first()), (1, Some(&fresh)));
    let mut longer = later.clone();
    longer.horizon.t = 70.0;
    scratch.repin(longer.clone(), |_, _| None);
    assert_eq!(scratch.first(), Some(&fresh));
    assert_eq!(scratch.pin.as_ref().map(|p| &p.inputs), Some(&longer));
    rejects("a repin that integrates on equal inputs", || {
        let mut n = 0;
        scratch.clone().repin(longer.clone(), |_, _| {
            n += 1;
            None
        });
        assert_eq!(n, 1);
    });
    // No pin: nothing to integrate. Hovered keeps the last sample, and none keeps it too.
    let mut none = Scratch::default();
    none.repin(inputs, |_, _| panic!("no pin to integrate"));
    none.hovered(Some(&s));
    none.hovered(None);
    assert_eq!(none.first(), Some(&s));
    // The pin before the last hovered.
    none.pin(fresh.clone(), later);
    assert_eq!(none.first(), Some(&fresh));
    // Inputs read the snapshot's horizon and playhead, not its plane.
    let mut moved = snapshot.clone();
    moved.sim.plane.z0[0] += 1.0;
    assert_eq!(Inputs::of(&moved), Inputs::of(&snapshot));
    moved.render.playhead.t = 2.0;
    assert_ne!(Inputs::of(&moved), Inputs::of(&snapshot));
}

#[test]
fn mock_trajectory_pin_unpin_joins_the_keys_after_keep() {
    let mut tree = ScopeTree::new();
    trajectory::register(&mut tree);
    let mut scratch = Scratch::default();
    join(&mut tree, &scratch);
    assert!(tree.get(UNPIN).is_none());
    assert_eq!(tree.get(UNDER_CURSOR).unwrap().label, FOLLOWING);
    let plane = mock_app().snapshot().sim.plane.clone();
    let snapshot = mock_app().snapshot().clone();
    scratch.pin(sample_at(&plane, [0.5, 0.5]), Inputs::of(&snapshot));
    join(&mut tree, &scratch);
    join(&mut tree, &scratch);
    let children = tree.children(Some(TRAJECTORY));
    assert_eq!(&children[..3], [UNDER_CURSOR, KEEP, UNPIN]);
    assert!(tree.get(UNPIN).unwrap().activates);
    assert_eq!(tree.get(UNDER_CURSOR).unwrap().label, PINNED);
    scratch.pin = None;
    join(&mut tree, &scratch);
    assert!(tree.get(UNPIN).is_none());
    assert_eq!(tree.children(Some(TRAJECTORY)).len(), children.len() - 1);
    rejects("an unpin left after the pin went", || {
        assert!(tree.get(UNPIN).is_some())
    });
}

#[test]
fn mock_trajectory_pin_tree_insert_and_remove_keep_the_rest() {
    let mut tree = ScopeTree::new();
    tree.register(None, Scope::group("a", "A"));
    tree.register(Some("a"), Scope::control("b", "B"));
    tree.register(Some("a"), Scope::control("c", "C"));
    tree.register(None, Scope::group("d", "D"));
    tree.register(Some("d"), Scope::group("e", "E"));
    tree.register(Some("e"), Scope::control("f", "F"));
    // After the first child; after the last node of the tree.
    tree.insert_after("b", Scope::control("x", "X"));
    assert_eq!(tree.children(Some("a")), ["b", "x", "c"]);
    assert_eq!(tree.children(None), ["a", "d"]);
    assert_eq!(tree.children(Some("d")), ["e"]);
    assert_eq!(
        tree.children(Some("e")),
        ["f"],
        "later parents moved with the insert"
    );
    tree.insert_after("f", Scope::control("y", "Y"));
    assert_eq!(tree.children(Some("e")), ["f", "y"]);
    tree.insert_after("nowhere", Scope::control("z", "Z"));
    assert!(tree.get("z").is_none());
    // Remove: a leaf; a scope with its children; the rest keep their parents.
    tree.remove("x");
    assert_eq!(tree.children(Some("a")), ["b", "c"]);
    assert_eq!(tree.children(Some("e")), ["f", "y"]);
    tree.remove("e");
    assert!(tree.get("f").is_none() && tree.get("y").is_none());
    assert!(tree.children(Some("d")).is_empty());
    tree.remove("nowhere");
    assert_eq!(tree.children(None), ["a", "d"]);
    tree.remove("a");
    assert_eq!(tree.children(None), ["d"]);
    rejects("a removed scope's sibling gone too", || {
        assert!(tree.get("d").is_none())
    });
    tree.get_mut("d").unwrap().label = "D2";
    assert_eq!(tree.get("d").unwrap().label, "D2");
    assert!(tree.get_mut("nowhere").is_none());
}

#[test]
fn mock_trajectory_pin_kept_tabs_join_the_keys_after_the_first() {
    use crate::explore::trajectory::{join_kept, KEPT_TAB_IDS};
    let mut tree = ScopeTree::new();
    trajectory::register(&mut tree);
    join_kept(&mut tree, 2);
    let children = tree.children(Some(TRAJECTORY));
    assert_eq!(
        &children[..4],
        [UNDER_CURSOR, KEPT_TAB_IDS[0], KEPT_TAB_IDS[1], KEEP]
    );
    assert!(tree.get(KEPT_TAB_IDS[0]).unwrap().activates);
    assert_eq!(tree.get(KEPT_TAB_IDS[1]).unwrap().label, "#2");
    join_kept(&mut tree, 1);
    assert_eq!(
        &tree.children(Some(TRAJECTORY))[..3],
        [UNDER_CURSOR, KEPT_TAB_IDS[0], KEEP]
    );
    assert!(tree.get(KEPT_TAB_IDS[1]).is_none());
    join_kept(&mut tree, 0);
    rejects("a tab left after its orbit went", || {
        assert!(tree.get(KEPT_TAB_IDS[0]).is_some())
    });
    // In the app: Enter on #1 shows it, as its click; the tab's widget is placed.
    let (mut app, mut h, layout) = app();
    point_at(&mut h, &mut app, at(layout.figure, 0.57, 0.55));
    app.view.focus.path = vec![FIGURE.to_owned()];
    press(&mut h, &mut app, Key::Space, NONE);
    let _ = h.frame(&mut app, Vec::new());
    assert!(
        app.keyboard.place_of(KEPT_TAB_IDS[0]).is_some(),
        "#1 placed"
    );
    app.view.focus.path = vec![TRAJECTORY.to_owned(), KEPT_TAB_IDS[0].to_owned()];
    press(&mut h, &mut app, Key::Enter, NONE);
    assert_eq!(app.trajectory.tab, Tab::Kept(0));
    app.trajectory.tab = Tab::UnderCursor;
    trajectory::activate(
        KEPT_TAB_IDS[3],
        &mut app.view.kept_orbits,
        &mut app.trajectory,
    );
    assert_eq!(app.trajectory.tab, Tab::UnderCursor, "no fourth orbit");
}
