//! `mock_tile_bounds` (REQ-GUI-172; render_gui_spec §G2; R-69): Overlays ▾ holds the overlay set's fourteen toggles in
//! their four groups, each with its Alt+digit; a toggle, by the menu, by Enter or by its shortcut, is one undoable
//! `SetField` on `RenderState`'s overlay set; with tile bounds on, the mock draws its fake quads' bounds into its
//! stand-in, and the GUI draws nothing over the figure; a pan moves the drawn bounds with the figure. Each check also
//! runs on an input it must reject (R-176, through `rejects`).

use std::sync::Arc;

use eframe::egui::{self, Event, Key, Modifiers};
use engine::contract::interface::EngineInterface;
use engine::contract::render_state::Overlays;
use engine::contract::sim_config::Plane;

use super::support::{click, frame_texts, headless, keys_on, mock_app, press, rejects, shapes};
use crate::app::{Actions, App};
use crate::capture::{render_frame, Shot, FORMAT};
use crate::explore::overlays_menu::{
    self, activate, count_on, flag, shortcut_label, shortcut_of, shortcut_presses, toggled,
    ALL_OFF, GROUPS, SAVE_DEFAULT, SHORTCUT_KEYS, TOGGLES,
};
use crate::headless::Headless;
use crate::layout::Layout;
use crate::mock::canvas::MockCanvas;
use crate::mock::quads::{self, near_frontier, on_figure, MAX_DEPTH, MIN_DEPTH, ROOT};
use crate::mock::{MockEngine, MOCK_Q1, MOCK_Q2, MOCK_Z0, TILE_BOUNDS_RGBA};
use crate::side::{EngineSide, MockSide};

const NONE: Modifiers = Modifiers::NONE;
const ALT: Modifiers = Modifiers::ALT;

/// The app after one frame.
fn fresh() -> (App<MockSide>, Headless) {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    (app, h)
}

/// The mock's plane.
fn plane() -> Plane {
    Plane {
        z0: MOCK_Z0,
        q1: MOCK_Q1,
        q2: MOCK_Q2,
    }
}

/// Only tile bounds on.
fn tile_bounds_only() -> Overlays {
    Overlays {
        tile_bounds: true,
        ..Overlays::default()
    }
}

/// The overlay `SetField` lines of the console from row `from`.
fn overlay_lines(app: &App<MockSide>, from: usize) -> Vec<String> {
    app.console()[from..]
        .iter()
        .map(|e| e.message.clone())
        .filter(|m| m.starts_with("SetField Overlays"))
        .collect()
}

// --- The toggle: one undoable SetField on the overlay set -------------------------------------------------------------

#[test]
fn mock_tile_bounds_toggle_is_one_setfield_on_the_overlay_set() {
    let (mut app, mut h) = fresh();
    let from = app.console().len();
    click(&mut h, &mut app, &format!("Overlays ▾ {}", 0));
    let texts = frame_texts(&mut h, &mut app);
    for (_, label) in TOGGLES {
        assert!(
            texts.iter().any(|t| t == label),
            "the menu shows {label}: {texts:?}"
        );
    }
    for heading in GROUPS.map(|(g, _)| g) {
        assert!(texts.iter().any(|t| t == heading), "the group {heading}");
    }
    for i in 0..TOGGLES.len() {
        let label = shortcut_label(i);
        assert!(texts.contains(&label), "the shortcut {label}");
    }
    click(&mut h, &mut app, "tile bounds");
    let texts = frame_texts(&mut h, &mut app);
    let snapshot = app.snapshot().clone();
    assert_eq!(snapshot.render.overlays, tile_bounds_only());
    assert_eq!(snapshot.history.undo_depth, 1, "one undoable SetField");
    let lines = overlay_lines(&app, from);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("[tile_bounds]"), "{}", lines[0]);
    assert!(
        texts.iter().any(|t| t == "Overlays ▾ 1"),
        "the count: {texts:?}"
    );
    rejects("a toggle that left the set as it was", || {
        assert_eq!(snapshot.render.overlays, Overlays::default())
    });
    app.undo();
    let _ = frame_texts(&mut h, &mut app);
    assert_eq!(
        app.snapshot().render.overlays,
        Overlays::default(),
        "undo turns it off"
    );
}

#[test]
fn mock_tile_bounds_menu_entries_act() {
    // Each toggle flips its own flag and only it.
    for (i, (id, _)) in TOGGLES.iter().enumerate() {
        let mut actions = Actions::default();
        activate(id, &Overlays::default(), &mut actions);
        let set = actions.overlays.expect("a toggle sets the overlays");
        assert_eq!(count_on(&set), 1, "{id}");
        assert!(*flag(&mut set.clone(), i), "{id} flips flag {i}");
        assert_eq!(
            toggled(&set, i),
            Overlays::default(),
            "{id} again flips it back"
        );
    }
    let mut all = Overlays::default();
    for i in 0..TOGGLES.len() {
        all = toggled(&all, i);
    }
    assert_eq!(count_on(&all), 14);
    let mut actions = Actions::default();
    activate(ALL_OFF.0, &all, &mut actions);
    assert_eq!(actions.overlays, Some(Overlays::default()), "all off");
    assert!(!actions.save_overlays);
    let mut actions = Actions::default();
    activate(SAVE_DEFAULT.0, &all, &mut actions);
    assert_eq!(
        (actions.overlays.clone(), actions.save_overlays),
        (None, true)
    );
    let mut actions = Actions::default();
    activate("ov_nowhere", &all, &mut actions);
    assert_eq!(actions, Actions::default());
    rejects("a toggle that flips another flag", || {
        let mut actions = Actions::default();
        activate(TOGGLES[0].0, &Overlays::default(), &mut actions);
        assert!(actions.overlays.unwrap().depth_colouring)
    });
    // There is no fifteenth flag.
    let past = std::panic::catch_unwind(|| {
        let mut o = Overlays::default();
        *flag(&mut o, TOGGLES.len()) = true;
    });
    assert!(past.is_err(), "flag 14 exists");
    // The groups hold the fourteen, five, four, three and two.
    assert_eq!(GROUPS.iter().map(|(_, n)| n).sum::<usize>(), TOGGLES.len());
    // Save as default keeps the set, and all off through the menu clears it in one SetField.
    let (mut app, mut h) = fresh();
    app.set_field(engine::contract::set_field::SetField {
        edit: engine::contract::set_field::Edit::Render(
            engine::contract::set_field::RenderField::Overlays(tile_bounds_only()),
        ),
        no_history: false,
    });
    let _ = h.frame(&mut app, Vec::new());
    click(&mut h, &mut app, "Overlays ▾ 1");
    click(&mut h, &mut app, SAVE_DEFAULT.1);
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(app.overlay_default, tile_bounds_only());
    let depth = app.snapshot().history.undo_depth;
    click(&mut h, &mut app, ALL_OFF.1);
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(app.snapshot().render.overlays, Overlays::default());
    assert_eq!(app.snapshot().history.undo_depth, depth + 1);
}

#[test]
fn mock_tile_bounds_menu_is_reached_by_the_keys() {
    let (mut app, mut h) = fresh();
    app.view.focus.path = vec!["top_bar".to_owned(), "overlays".to_owned()];
    keys_on(&mut h, &mut app);
    press(&mut h, &mut app, Key::Enter, NONE);
    assert_eq!(
        app.view.focus.path,
        ["top_bar", "overlays", TOGGLES[0].0],
        "Enter opens the menu on its first toggle"
    );
    press(&mut h, &mut app, Key::Enter, NONE);
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(
        app.snapshot().render.overlays,
        tile_bounds_only(),
        "Enter acts as the click"
    );
    let tree = app.keyboard.tree(engine::contract::view_ui::Mode::Explore);
    let mut want: Vec<&str> = TOGGLES.iter().map(|(id, _)| *id).collect();
    want.extend([ALL_OFF.0, SAVE_DEFAULT.0]);
    assert_eq!(tree.children(Some(overlays_menu::MENU)), want);
    rejects("the menu's entries in another order", || {
        let mut other = want.clone();
        other.swap(0, 1);
        assert_eq!(tree.children(Some(overlays_menu::MENU)), other)
    });
}

// --- The shortcuts: Alt+digit, the last four with Shift ----------------------------------------------------------------

fn key(key: Key, modifiers: Modifiers, repeat: bool) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat,
        modifiers,
    }
}

#[test]
fn mock_tile_bounds_shortcuts_toggle_in_order() {
    for (i, (k, shift)) in SHORTCUT_KEYS.iter().enumerate() {
        let modifiers = if *shift { ALT | Modifiers::SHIFT } else { ALT };
        assert_eq!(shortcut_of(*k, modifiers), Some(i), "{}", shortcut_label(i));
        let other = if *shift { ALT } else { ALT | Modifiers::SHIFT };
        assert_ne!(shortcut_of(*k, other), Some(i), "Shift chooses the group");
    }
    assert_eq!(shortcut_label(0), "Alt+1");
    assert_eq!(shortcut_label(9), "Alt+0");
    assert_eq!(shortcut_label(10), "Alt+Shift+1");
    assert_eq!(shortcut_label(13), "Alt+Shift+4");
    for modifiers in [
        NONE,
        Modifiers::CTRL | ALT,
        Modifiers::COMMAND | ALT,
        Modifiers::MAC_CMD | ALT,
    ] {
        assert_eq!(shortcut_of(Key::Num1, modifiers), None, "{modifiers:?}");
    }
    assert_eq!(
        shortcut_of(Key::Num5, ALT | Modifiers::SHIFT),
        None,
        "only four with Shift"
    );
    assert_eq!(shortcut_of(Key::A, ALT), None);
    // Presses only: a repeat or a release toggles nothing.
    let events = [
        key(Key::Num1, ALT, false),
        key(Key::Num2, ALT, true),
        Event::Key {
            key: Key::Num3,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers: ALT,
        },
        key(Key::Num4, ALT | Modifiers::SHIFT, false),
        Event::Text("¡".into()),
    ];
    assert_eq!(shortcut_presses(&events), [0, 13]);
    rejects("a repeat that toggles", || {
        assert_eq!(shortcut_presses(&events), [0, 1, 13])
    });
    // In the app: Alt+1 turns tile bounds on, one SetField; Alt+Shift+4 the t_end contours.
    let (mut app, mut h) = fresh();
    press(&mut h, &mut app, Key::Num1, ALT);
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(app.snapshot().render.overlays, tile_bounds_only());
    assert_eq!(app.snapshot().history.undo_depth, 1);
    press(&mut h, &mut app, Key::Num4, ALT | Modifiers::SHIFT);
    let _ = h.frame(&mut app, Vec::new());
    assert!(app.snapshot().render.overlays.t_end_contours);
    // With the `?` overlay open, they act not.
    let (mut app, mut h) = fresh();
    app.keyboard.shortcuts_open = true;
    press(&mut h, &mut app, Key::Num1, ALT);
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(app.snapshot().render.overlays, Overlays::default());
}

// --- The mock draws the bounds; the GUI draws nothing over the figure -------------------------------------------------

/// The shapes of one frame inside the figure, as text, the canvas's own callback aside.
fn figure_shapes(h: &mut Headless, app: &mut App<MockSide>) -> Vec<String> {
    let figure = Layout::new(h.screen(), crate::capture::PIXELS_PER_POINT)
        .figure
        .shrink(30.0);
    shapes(h, app)
        .into_iter()
        .filter(|s| figure.contains_rect(s.visual_bounding_rect()))
        .filter(|s| !matches!(s, egui::Shape::Callback(_)))
        .map(|s| format!("{s:?}"))
        .collect()
}

#[test]
fn mock_tile_bounds_drawn_by_the_mock_not_the_gui() {
    let mut mock = MockEngine::new();
    assert!(mock.marks().is_empty(), "off, no hover: no marks");
    let off = {
        let (mut app, mut h) = fresh();
        figure_shapes(&mut h, &mut app)
    };
    let (mut app, mut h) = fresh();
    press(&mut h, &mut app, Key::Num1, ALT);
    let _ = h.frame(&mut app, Vec::new());
    assert!(app.snapshot().render.overlays.tile_bounds);
    let on = figure_shapes(&mut h, &mut app);
    assert_eq!(on, off, "the GUI draws nothing new over the figure");
    let marks = app.side().engine().marks();
    assert!(
        marks.len() > 100,
        "the mock draws the bounds: {}",
        marks.len()
    );
    assert!(marks.iter().all(|m| m.rgba == TILE_BOUNDS_RGBA));
    rejects("a mock that draws no bounds", || {
        assert!(mock.marks().len() > 100)
    });
    mock.set_field(engine::contract::set_field::SetField {
        edit: engine::contract::set_field::Edit::Render(
            engine::contract::set_field::RenderField::Overlays(tile_bounds_only()),
        ),
        no_history: false,
    });
    assert_eq!(mock.marks().len(), quads::bounds(mock.plane()).len());
}

/// The segments of `a` found in `b` once moved by `shift`, as a fraction of `a`'s.
fn found(a: &[[[f64; 2]; 2]], b: &[[[f64; 2]; 2]], shift: [f64; 2]) -> f64 {
    let close = |p: [f64; 2], q: [f64; 2]| (p[0] - q[0]).abs() < 1e-9 && (p[1] - q[1]).abs() < 1e-9;
    let moved = |p: [f64; 2]| [p[0] - shift[0], p[1] - shift[1]];
    let hits = a
        .iter()
        .filter(|[p, q]| {
            b.iter()
                .any(|[r, s]| close(moved(*p), *r) && close(moved(*q), *s))
        })
        .count();
    hits as f64 / a.len() as f64
}

#[test]
fn mock_tile_bounds_move_with_a_pan() {
    let before = plane();
    let mut after = before.clone();
    after.z0[0] += 0.05;
    let shift = {
        let a = on_figure(&before, (0.0, 0.0)).unwrap();
        let b = on_figure(&after, (0.0, 0.0)).unwrap();
        [b[0] - a[0], b[1] - a[1]]
    };
    assert!(
        shift[0] < 0.0 && shift[1].abs() < 1e-12,
        "a pan right moves the quads left: {shift:?}"
    );
    let (a, b) = (quads::bounds(&after), quads::bounds(&before));
    let moved = found(&a, &b, shift);
    assert!(moved > 0.6, "{moved} of the bounds moved with the figure");
    let unmoved = found(&a, &b, [0.0, 0.0]);
    rejects("bounds fixed on the screen", || assert!(unmoved > 0.6));
}

#[test]
fn mock_tile_bounds_pixels_change_in_the_figure_only() {
    let canvas = Arc::new(MockCanvas::new().expect("a GPU adapter for the mock's canvas"));
    let shoot = |overlays: Overlays| -> (Shot, Layout) {
        let mut app = App::new(
            MockSide::new(MockEngine::frozen(), Some(canvas.clone())),
            FORMAT,
        );
        let mut h = headless();
        app.set_field(engine::contract::set_field::SetField {
            edit: engine::contract::set_field::Edit::Render(
                engine::contract::set_field::RenderField::Overlays(overlays),
            ),
            no_history: false,
        });
        for _ in 0..2 {
            let _ = h.frame(&mut app, Vec::new());
        }
        let layout = Layout::new(h.screen(), crate::capture::PIXELS_PER_POINT);
        (render_frame(&canvas, &mut app, &mut h), layout)
    };
    let (off, layout) = shoot(Overlays::default());
    let (on, _) = shoot(tile_bounds_only());
    let px = |v: f32| (v * crate::capture::PIXELS_PER_POINT) as usize;
    let (f, bar, footer) = (layout.figure, layout.top_bar, layout.footer);
    // The pixels that differ, inside the figure and outside it, the top bar (whose count changes) and the footer (whose
    // latest entry is the SetField) aside.
    let changed = |a: &Shot, b: &Shot| {
        let (mut inside, mut outside) = (0, 0);
        for y in 0..a.size[1] as usize {
            for x in 0..a.size[0] as usize {
                let i = (y * a.size[0] as usize + x) * 4;
                if a.rgba[i..i + 4] == b.rgba[i..i + 4] {
                    continue;
                }
                let in_figure = (px(f.min.x)..px(f.max.x)).contains(&x)
                    && (px(f.min.y)..px(f.max.y)).contains(&y);
                if in_figure {
                    inside += 1;
                } else if y > px(bar.max.y) && y < px(footer.min.y) {
                    outside += 1;
                }
            }
        }
        (inside, outside)
    };
    let (inside, outside) = changed(&off, &on);
    assert!(
        inside > 1000,
        "the bounds change the figure: {inside} pixels"
    );
    assert_eq!(
        outside, 0,
        "and nothing outside it but the count and the log"
    );
    rejects("bounds that change nothing", || {
        assert!(changed(&off, &off).0 > 1000)
    });
    // Drawn as lines, one pixel wide, they change a small part of the figure (about 4% here); filled, as triangles
    // between their ends, they would change about a third of it.
    let figure = (px(f.max.x) - px(f.min.x)) * (px(f.max.y) - px(f.min.y));
    assert!(
        inside < figure / 10,
        "the bounds are lines: {inside} of {figure} pixels"
    );
    rejects("bounds filled as triangles", || {
        assert!(321_201 < figure / 10)
    });
}

// --- The fake quadtree ------------------------------------------------------------------------------------------------

#[test]
fn mock_tile_bounds_quads_refine_toward_the_frontier() {
    let leaves = quads::quads(&plane());
    let width = |q: &[f64; 4]| q[2] - q[0];
    let widest = leaves.iter().map(width).fold(0.0, f64::max);
    let narrowest = leaves.iter().map(width).fold(f64::INFINITY, f64::min);
    assert!(
        widest <= 2.0 * ROOT / f64::from(1u32 << MIN_DEPTH),
        "split at least MIN_DEPTH: {widest}"
    );
    assert!(
        narrowest >= 2.0 * ROOT / f64::from(1u32 << MAX_DEPTH) - 1e-12,
        "no deeper than MAX_DEPTH"
    );
    let (near, far): (Vec<_>, Vec<_>) = leaves.iter().partition(|q| near_frontier(q));
    let mean = |v: &[&[f64; 4]]| v.iter().map(|q| width(q)).sum::<f64>() / v.len() as f64;
    assert!(!near.is_empty() && !far.is_empty());
    assert!(mean(&near) < mean(&far) / 2.0, "finer near the frontier");
    rejects("a uniform tree", || {
        assert!(mean(&near) < mean(&far) / 100.0)
    });
    // The frontier test at its edges: a quad touching the circle is near; one a width beyond is not.
    let ([cu, cv], r) = quads::FRONTIER;
    let w = 0.1;
    assert!(near_frontier(&[cu + r, cv, cu + r + w, cv + w]));
    assert!(near_frontier(&[cu + r + w, cv, cu + r + 2.0 * w, cv + w]));
    assert!(!near_frontier(&[
        cu + r + 1.01 * w,
        cv,
        cu + r + 2.01 * w,
        cv + w
    ]));
    // A quad well inside the circle, more than its width from it, is not near.
    let inner = 0.02;
    assert!(!near_frontier(&[
        cu - inner / 2.0,
        cv - inner / 2.0,
        cu + inner / 2.0,
        cv + inner / 2.0
    ]));
    // Zoomed in, the leaves in view are smaller.
    let mut zoomed = plane();
    zoomed.q1 = zoomed.q1.map(|x| x / 4.0);
    zoomed.q2 = zoomed.q2.map(|x| x / 4.0);
    let zoomed_widest = quads::quads(&zoomed).iter().map(width).fold(0.0, f64::max);
    assert!(zoomed_widest < widest, "{zoomed_widest} < {widest}");
    // A degenerate basis has none.
    let mut flat = plane();
    flat.q2 = [0.0; 8];
    assert!(quads::quads(&flat).is_empty());
    assert!(quads::bounds(&flat).is_empty());
    assert_eq!(on_figure(&flat, (0.0, 0.0)), None);
    // Every bound is a quad's edge on the figure: four per quad.
    assert_eq!(quads::bounds(&plane()).len(), 4 * leaves.len());
}
