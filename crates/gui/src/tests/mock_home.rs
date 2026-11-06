//! REQ-GUI-219 and REQ-GUI-220 (R-466): Home is the three resets in one command, from the house in the canvas's
//! top-right corner, the Home key in both keyboard modes and View › Home, and leaves everything else alone. Each check
//! is shown failing on what a GUI with the fault would leave (R-176, through `rejects`; `support`'s note).

use eframe::egui::{Event, Key, Modifiers};
use engine::contract::sim_config::Plane;
use engine::contract::view_ui::{KeyMode, Mode, Window};

use super::mock_layout_splitters::drag_splitter;
use super::mock_manifold_resets::{
    at_preset, check_home, move_all, move_navigate, step, Rig, Values, MU1, SCRIPT_PRESETS,
};
use super::mock_windows_move_resize::{drag_title, open};
use super::support::{click_scope, keys_on, names, press, rejects, texts};
use crate::explore::manifold_view::reset::{self, home_rect, HOME_INSET, HOME_NAME, HOME_SIDE};
use crate::explore::manifold_view::{self as mv, View, PRESET_HOMES};
use crate::explore::top_bar::{self, HOME};
use crate::explore::{time, FIGURE};
use crate::layout::{Layout, Splitter};
use crate::windows::console;

const NONE: Modifiers = Modifiers::NONE;

/// The canvas as the app lays it out.
fn canvas(rig: &Rig) -> eframe::egui::Rect {
    Layout::with_splits(
        rig.1.screen(),
        crate::capture::PIXELS_PER_POINT,
        &rig.0.splits,
    )
    .figure
}

/// A click on the Home button's place.
fn click_button(rig: &mut Rig) {
    let at = home_rect(canvas(rig)).center();
    for events in crate::capture::click(at) {
        let _ = rig.1.frame(&mut rig.0, events);
    }
}

/// The focus at `id`, keyboard navigation on.
fn focus_on(rig: &mut Rig, id: &str) {
    let (app, h) = (&mut rig.0, &mut rig.1);
    app.view.focus.path = app.keyboard.tree(app.view.mode).path_to(id);
    keys_on(h, app);
}

/// The Home key.
fn home_key(rig: &mut Rig, modifiers: Modifiers) {
    press(&mut rig.1, &mut rig.0, Key::Home, modifiers);
    let _ = rig.1.frame(&mut rig.0, Vec::new());
}

/// A way to Home: what it is, and the script.
type Way = (&'static str, fn(&mut Rig));

/// Each way to Home, on the rig it has moved.
fn ways() -> Vec<Way> {
    vec![
        ("the button", click_button),
        ("the key on the Figure", |rig| {
            focus_on(rig, FIGURE);
            home_key(rig, NONE);
        }),
        ("the key on a Chart element", |rig| {
            focus_on(rig, mv::PRESET);
            home_key(rig, NONE);
        }),
        ("the key interacting with a slider", |rig| {
            focus_on(rig, mv::SLIDERS[2]);
            press(&mut rig.1, &mut rig.0, Key::Enter, NONE);
            assert_eq!(rig.0.view.focus.mode, KeyMode::Interaction);
            home_key(rig, NONE);
        }),
        ("the key interacting with the compass", |rig| {
            focus_on(rig, crate::explore::COMPASS_VIEW);
            press(&mut rig.1, &mut rig.0, Key::Enter, NONE);
            assert_eq!(rig.0.view.focus.mode, KeyMode::Interaction);
            home_key(rig, NONE);
        }),
        ("the key with no focus", |rig| {
            rig.0.view.focus.path.clear();
            home_key(rig, NONE);
        }),
        ("Shift+Home", |rig| home_key(rig, Modifiers::SHIFT)),
        ("Alt+Home", |rig| home_key(rig, Modifiers::ALT)),
        ("View › Home, clicked", |rig| {
            click_scope(&mut rig.1, &mut rig.0, "view");
            let _ = rig.1.frame(&mut rig.0, Vec::new());
            click_scope(&mut rig.1, &mut rig.0, HOME.0);
        }),
        ("View › Home, Enter", |rig| {
            focus_on(rig, HOME.0);
            press(&mut rig.1, &mut rig.0, Key::Enter, NONE);
            let _ = rig.1.frame(&mut rig.0, Vec::new());
        }),
    ]
}

#[test]
fn mock_home_every_way_reaches_the_home_view() {
    for p in SCRIPT_PRESETS {
        for (what, way) in ways() {
            let mut rig = at_preset(p);
            move_all(&mut rig);
            let depth = rig.0.snapshot().history.undo_depth;
            way(&mut rig);
            check_home(&Values::read(&rig.0, p), p);
            assert_eq!(
                rig.0.snapshot().sim.plane.z0,
                PRESET_HOMES[p].base,
                "{what}"
            );
            assert_eq!(rig.0.snapshot().history.undo_depth, depth + 1, "{what}");
        }
    }
    // Controls: a Home of one section, or of two, is no home view.
    for section in [reset::Section::Tilt, reset::Section::Navigate] {
        let mut rig = at_preset(0);
        move_all(&mut rig);
        click_scope(&mut rig.1, &mut rig.0, section.scope());
        rejects("a Home of one section", || {
            check_home(&Values::read(&rig.0, 0), 0)
        });
    }
}

fn focus_of(rig: &Rig) -> Vec<String> {
    rig.0.view.focus.path.clone()
}

#[test]
fn mock_home_key_leaves_the_focus_and_the_mode() {
    let mut rig = at_preset(0);
    move_navigate(&mut rig);
    focus_on(&mut rig, mv::SLIDERS[2]);
    press(&mut rig.1, &mut rig.0, Key::Enter, NONE);
    let (path, mode) = (focus_of(&rig), rig.0.view.focus.mode);
    home_key(&mut rig, NONE);
    assert_eq!(focus_of(&rig), path);
    assert_eq!(rig.0.view.focus.mode, mode);
}

#[test]
fn mock_home_button_is_in_the_corner() {
    let rig = at_preset(0);
    let shown = canvas(&rig);
    let rect = home_rect(shown);
    assert_eq!(rect.width(), HOME_SIDE);
    assert_eq!(rect.height(), HOME_SIDE);
    assert!((shown.max.x - rect.max.x - HOME_INSET).abs() < 1e-3);
    assert!((rect.min.y - shown.min.y - HOME_INSET).abs() < 1e-3);
}

/// A house of strokes, no glyph: each stroke inside its button.
#[test]
fn mock_home_button_is_painted_in_strokes() {
    let mut rig = at_preset(0);
    let rect = home_rect(canvas(&rig));
    let _ = rig.1.frame(&mut rig.0, Vec::new());
    let colour = eframe::egui::Color32::WHITE;
    for shape in reset::house(rect, colour) {
        assert!(
            rect.contains_rect(shape.visual_bounding_rect()),
            "{shape:?}"
        );
    }
    let shapes = super::support::shapes(&mut rig.1, &mut rig.0);
    let lines = shapes
        .iter()
        .filter(|s| {
            !matches!(s, eframe::egui::Shape::Text(_))
                && rect.contains_rect(s.visual_bounding_rect())
        })
        .count();
    assert!(lines >= 2, "the two strokes: {lines}");
}

/// The state a click on the figure would change.
fn figure_state(rig: &Rig) -> (Plane, bool) {
    (
        rig.0.snapshot().sim.plane.clone(),
        rig.0.trajectory.pin.is_some(),
    )
}

#[test]
fn mock_home_button_click_pins_nothing_and_pans_nothing() {
    // Dimmed, at home, and live: neither is a click on the figure.
    for moved in [false, true] {
        let mut rig = at_preset(0);
        if moved {
            move_navigate(&mut rig);
        }
        click_button(&mut rig);
        let (plane, pinned) = figure_state(&rig);
        assert!(!pinned, "moved {moved}: no sample pinned");
        assert_eq!(plane.z0, PRESET_HOMES[0].base, "moved {moved}: no pan");
        // Control: the same click on the figure beside the button does pin.
        let beside = home_rect(canvas(&rig)).center() - eframe::egui::vec2(60.0, 0.0);
        for events in crate::capture::click(beside) {
            let _ = rig.1.frame(&mut rig.0, events);
        }
        rejects("a button click that pins", || {
            assert!(!figure_state(&rig).1)
        });
    }
    // A drag from it is no pan either.
    let mut rig = at_preset(0);
    let from = home_rect(canvas(&rig)).center();
    super::mock_layout_splitters::drag(
        &mut rig.1,
        &mut rig.0,
        from,
        from - eframe::egui::vec2(80.0, -40.0),
    );
    assert_eq!(rig.0.snapshot().sim.plane.z0, PRESET_HOMES[0].base);
}

/// Whether the Home button is in the frame's names.
fn button_drawn(rig: &mut Rig) -> bool {
    let (app, h) = (&mut rig.0, &mut rig.1);
    texts(&names(h, app)).iter().any(|t| t == HOME_NAME)
}

#[test]
fn mock_home_button_hides_with_the_layer() {
    let mut rig = at_preset(0);
    assert!(button_drawn(&mut rig), "shown");
    move_navigate(&mut rig);
    press(&mut rig.1, &mut rig.0, Key::F3, NONE);
    assert!(!rig.0.shown, "F3 hid the layer");
    assert!(!button_drawn(&mut rig), "F3 hides it");
    let before = rig.0.snapshot().sim.plane.clone();
    click_button(&mut rig);
    assert_eq!(
        rig.0.snapshot().sim.plane,
        before,
        "no Home on a hidden button"
    );
    // Control: the check sees it when it is there.
    press(&mut rig.1, &mut rig.0, Key::F3, NONE);
    rejects("a button that stays with the layer hidden", || {
        assert!(!button_drawn(&mut rig))
    });
    rig.0.present = true;
    assert!(!button_drawn(&mut rig), "present mode hides it");
}

#[test]
fn mock_home_button_goes_under_a_console_reaching_it() {
    let mut rig = at_preset(0);
    rig.0.console_open = true;
    assert!(button_drawn(&mut rig), "a docked console leaves the corner");
    rig.0.windows.console.maximised = true;
    let _ = rig.1.frame(&mut rig.0, Vec::new());
    assert!(!button_drawn(&mut rig), "a maximised one covers it");
}

#[test]
fn mock_home_key_on_the_stain_page() {
    let mut rig = at_preset(0);
    move_navigate(&mut rig);
    rig.0.view.mode = Mode::Stain;
    let _ = rig.1.frame(&mut rig.0, Vec::new());
    assert!(!button_drawn(&mut rig), "the button is the canvas's");
    home_key(&mut rig, NONE);
    check_home(&Values::read(&rig.0, 0), 0);
}

#[test]
fn mock_home_key_on_the_console_log_goes_to_the_first_entry() {
    let mut rig = at_preset(0);
    move_all(&mut rig);
    open(&mut rig.1, &mut rig.0, Window::Console);
    focus_on(&mut rig, console::LOG);
    press(&mut rig.1, &mut rig.0, Key::Enter, NONE);
    assert_eq!(rig.0.view.focus.mode, KeyMode::Interaction);
    press(&mut rig.1, &mut rig.0, Key::End, NONE);
    let before = Values::read(&rig.0, 0);
    home_key(&mut rig, NONE);
    assert_eq!(Values::read(&rig.0, 0), before, "nothing reset");
    // Control: a GUI where Home there resets the view leaves the home view; navigating on the log, it is Home.
    press(&mut rig.1, &mut rig.0, Key::Escape, NONE);
    assert_eq!(rig.0.view.focus.mode, KeyMode::Navigation);
    home_key(&mut rig, NONE);
    rejects("the log's Home left the view alone", || {
        assert_eq!(Values::read(&rig.0, 0), before)
    });
    check_home(&Values::read(&rig.0, 0), 0);
}

#[test]
fn mock_home_key_in_a_typed_field_moves_the_caret() {
    let mut rig = at_preset(0);
    move_navigate(&mut rig);
    open(&mut rig.1, &mut rig.0, Window::Console);
    focus_on(&mut rig, console::SEARCH);
    press(&mut rig.1, &mut rig.0, Key::Enter, NONE);
    assert_eq!(rig.0.keyboard.typing(), Some(console::SEARCH));
    let before = Values::read(&rig.0, 0);
    let _ = rig.1.frame(&mut rig.0, vec![Event::Text("ab".to_owned())]);
    home_key(&mut rig, NONE);
    let _ = rig.1.frame(&mut rig.0, vec![Event::Text("c".to_owned())]);
    let _ = rig.1.frame(&mut rig.0, Vec::new());
    assert_eq!(rig.0.windows.console.text, "cab", "the caret at the start");
    assert_eq!(Values::read(&rig.0, 0), before, "nothing reset");
}

#[test]
fn mock_home_ctrl_and_cmd_are_no_home() {
    for modifiers in [Modifiers::CTRL, Modifiers::COMMAND, Modifiers::MAC_CMD] {
        let mut rig = at_preset(0);
        move_navigate(&mut rig);
        let before = Values::read(&rig.0, 0);
        home_key(&mut rig, modifiers);
        assert_eq!(Values::read(&rig.0, 0), before, "{modifiers:?}");
    }
    // Control: unmodified, the same script resets.
    let mut rig = at_preset(0);
    move_navigate(&mut rig);
    let before = Values::read(&rig.0, 0);
    home_key(&mut rig, NONE);
    rejects("a Home that left the view", || {
        assert_eq!(Values::read(&rig.0, 0), before)
    });
}

#[test]
fn mock_home_key_does_not_repeat_and_keeps_the_overlay() {
    let mut rig = at_preset(0);
    move_navigate(&mut rig);
    let held = Event::Key {
        key: Key::Home,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: NONE,
    };
    let repeat = Event::Key {
        key: Key::Home,
        physical_key: None,
        pressed: true,
        repeat: true,
        modifiers: NONE,
    };
    let _ = rig.1.frame(&mut rig.0, vec![held]);
    let _ = rig.1.frame(&mut rig.0, Vec::new());
    move_slice_while_held(&mut rig);
    let moved = Values::read(&rig.0, 0);
    let _ = rig.1.frame(&mut rig.0, vec![repeat]);
    let _ = rig.1.frame(&mut rig.0, Vec::new());
    assert_eq!(Values::read(&rig.0, 0), moved, "a held key acts once");

    // Over the `?` overlay, which takes every key, it does nothing.
    let mut rig = at_preset(0);
    move_navigate(&mut rig);
    let before = Values::read(&rig.0, 0);
    rig.0.keyboard.shortcuts_open = true;
    let _ = rig.1.frame(&mut rig.0, Vec::new());
    home_key(&mut rig, NONE);
    assert_eq!(Values::read(&rig.0, 0), before, "the overlay takes it");
}

/// Moves `z₀` through the contract, the Home key still down.
fn move_slice_while_held(rig: &mut Rig) {
    let mut z0 = rig.0.snapshot().sim.plane.z0;
    z0[2] = 0.5;
    rig.0.set_field(mv::z0_edit(z0));
    let _ = rig.1.frame(&mut rig.0, Vec::new());
}

/// Home's locked landing: the home view exactly, the lock and its anchor kept (K5).
#[test]
fn mock_home_locked_lands_on_the_home_view() {
    let mut rig = at_preset(0);
    move_all(&mut rig);
    let sim = rig.0.snapshot().sim.clone();
    let lock = engine::contract::sim_config::Lock {
        locked: true,
        z_locked: mv::point(&sim.plane, 0.75, 0.25),
    };
    rig.0.set_field(mv::lock_edit(lock.clone()));
    let _ = rig.1.frame(&mut rig.0, Vec::new());
    let depth = rig.0.snapshot().history.undo_depth;
    click_button(&mut rig);
    check_home(&Values::read(&rig.0, 0), 0);
    assert_eq!(rig.0.snapshot().sim.plane.z0, PRESET_HOMES[0].base);
    assert_eq!(
        rig.0.snapshot().sim.lock,
        lock,
        "the lock and its anchor stay"
    );
    assert_eq!(rig.0.snapshot().history.undo_depth, depth + 1, "one entry");
}

/// What Home must leave alone.
#[derive(Debug, PartialEq)]
struct Left {
    preset: Option<usize>,
    axes: (usize, usize),
    slice: engine::contract::sim_config::Slice,
    playhead: engine::contract::render_state::Playhead,
    stain: engine::contract::render_state::StainGraph,
    overlays: engine::contract::render_state::Overlays,
    lock: engine::contract::sim_config::Lock,
    splits: crate::layout::Splits,
    places: crate::windows::place::Places,
    compass: (mv::Touch, crate::explore::compass::views::CompassView),
    kept: engine::contract::view_ui::KeptOrbits,
    open: Vec<Window>,
}

fn left(rig: &Rig) -> Left {
    let snapshot = rig.0.snapshot();
    let view = View::read(&snapshot.sim);
    Left {
        preset: view.preset,
        axes: (view.a, view.b),
        slice: snapshot.sim.slice.clone(),
        playhead: snapshot.render.playhead.clone(),
        stain: snapshot.render.stain_graph.clone(),
        overlays: snapshot.render.overlays.clone(),
        lock: snapshot.sim.lock.clone(),
        splits: rig.0.splits,
        places: rig.0.places.clone(),
        compass: (rig.0.manifold.compass.mode, rig.0.manifold.compass.view),
        kept: rig.0.view.kept_orbits.clone(),
        open: rig.0.view.windows.open.clone(),
    }
}

#[test]
fn mock_home_leaves_alone_everything_else() {
    let mut rig = at_preset(0);
    move_all(&mut rig);
    // The playhead off 0, the stain edited, the D axis and a target changed, a splitter dragged, a window moved,
    // Alt+1's overlay on, the compass in Slice.
    step(
        &mut rig,
        time::SCRUBBER,
        Key::ArrowRight,
        Modifiers::SHIFT,
        2,
    );
    let edit = crate::stain::edited(&rig.0.snapshot().render.stain_graph, |g| {
        g.set_occupant(
            engine::contract::render_state::NodeId(2),
            engine::contract::render_state::Occupant::Field("diffusion_slope_valid".into()),
        )
    })
    .expect("a source's field");
    rig.0.set_field(edit);
    let normal = mv::normal_edit(&View::read(&rig.0.snapshot().sim), 7).expect("a normal");
    rig.0.set_field(normal);
    drag_splitter(&mut rig.1, &mut rig.0, Splitter::ManifoldFigure, 20.0);
    open(&mut rig.1, &mut rig.0, Window::Run);
    drag_title(
        &mut rig.1,
        &mut rig.0,
        Window::Run,
        eframe::egui::vec2(40.0, 30.0),
    );
    press(&mut rig.1, &mut rig.0, Key::Num1, Modifiers::ALT);
    rig.0.manifold.compass.mode = mv::Touch::Slice;
    let _ = rig.1.frame(&mut rig.0, Vec::new());
    let before = left(&rig);
    assert_ne!(before.playhead.t, 0.0, "the playhead moved");
    assert_eq!(before.slice.frame.tilt_y as usize, MU1, "Y moved");
    click_button(&mut rig);
    let after = left(&rig);
    assert_eq!(after, before);
    check_home(&Values::read(&rig.0, 0), 0);
    // Controls: a Home touching the playhead, a target, D or the layout fails.
    let mut wrong = left(&rig);
    wrong.playhead.t = 0.0;
    rejects("a Home resetting the playhead", || {
        assert_eq!(wrong, before)
    });
    let mut wrong = left(&rig);
    wrong.slice.frame.tilt_y = wrong.slice.frame.normal;
    rejects("a Home resetting a target", || assert_eq!(wrong, before));
    let mut wrong = left(&rig);
    wrong.slice.frame.normal = 3;
    rejects("a Home resetting D", || assert_eq!(wrong, before));
    let mut wrong = left(&rig);
    wrong.splits = Default::default();
    rejects("a Home resetting the layout", || assert_eq!(wrong, before));
}

#[test]
fn mock_home_view_menu_is_first_with_its_shortcut() {
    let rig = at_preset(0);
    let tree = rig.0.keyboard.tree(rig.0.view.mode);
    assert_eq!(tree.children(Some("view")).first(), Some(&HOME.0));
    assert_eq!(top_bar::HOME.1, "Home");
    let row = crate::keyboard::keymap::SHORTCUTS
        .iter()
        .find(|(key, _)| *key == "Home");
    assert!(row.is_some_and(|(_, action)| action.contains("chart preset")));
}
