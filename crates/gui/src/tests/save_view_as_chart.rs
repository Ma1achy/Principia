//! `save_view_as_chart` (render_gui_spec §G2): Save view as chart, the button after Chart builder… and View's entry after
//! Debug view, on the mock: one undo entry that makes a new saved chart active, the Saved group, the rename, the
//! provenance, the delete, and the settings that keep the list.
//!
//! **The criteria**, written before the first run:
//! - **One entry.** A click saves one chart, "Saved 1", active; the undo depth one greater; the state, its frame and
//!   `(z₀, q₁, q₂)` bit for bit, under both maps at any `γ` and targets, and the mock's integrated quads as they were; one Ctrl+Z gives the previous chart, the depth as before, the entry kept.
//! - **The active chart follows the history.** Undo then redo: active. Undo then a pan, which ends at the same depths:
//!   not. A zoom and a slice: still active. A steer to another plane: not; its undo: active again. A preset picked on
//!   the same plane: not.
//! - **The keyboard** reaches the button and View's entry: each saves one chart.
//! - **The rename** sticks across a relaunch on the same settings file, and the vectors come back bit for bit.
//! - **The provenance** names the parent, the map, the angles, the targets, the slice centre and the pair, and says a
//!   skewed pair is not orthogonal.
//! - **The delete** of an inactive chart takes it from the list with no history entry; of the active one leaves the
//!   view, the quads and the depth, the picker reading "unsaved chart" with its provenance.
//! - **The settings** read back every vector bit for bit, −0, the smallest double and unrounded sums among them.
//! - **The real side**: the same button on the real engine's store gives the same entry, undone the same way.
//!
//! **Controls** (through `rejects`): a save of two entries; an undo that takes the list's entry; a chart active at
//! the depths of the redo; a save that moves a frame angle; the keys one short of the button, and of the entry; a GUI that forgets its settings; a rigid
//! turn's provenance called skewed; a delete that adds an entry; a delete that switches the active chart; a vector
//! stored as `f32`; a real side not clicked.

use std::path::PathBuf;

use eframe::egui::{Event, Key, Modifiers};
use engine::contract::interface::EngineInterface;
use engine::contract::sim_config::{Latent, Plane, SimConfig, SliceFrame, TiltMap};

use super::support::{
    click, click_scope, frame_texts, headless, names, nodes, press, rejects, texts, FORMAT,
};
use crate::app::App;
use crate::explore::manifold_view::saved::{
    self, Charts, SavedChart, DELETE, DELETE_TEXT, RENAME, SAVE_TEXT, UNSAVED,
};
use crate::explore::manifold_view::tilt::Angles;
use crate::explore::manifold_view::{
    axis, basis_edit, frame_edit, z0_edit, View, AXES, BUILDER, PRESET,
};
use crate::headless::Headless;
use crate::mock::MockEngine;
use crate::settings::{Settings, Store, FILE};
use crate::side::{EngineSide, MockSide, RealSide};

const NONE: Modifiers = Modifiers::NONE;

/// A GUI on the mock, its settings kept in `store`, settled.
pub(super) fn gui(store: Store) -> (App<MockSide>, Headless) {
    let side = MockSide::new(MockEngine::new(), None);
    let mut app = App::with_settings(side, FORMAT, store);
    let mut h = headless();
    frames(&mut h, &mut app, 3);
    (app, h)
}

/// `n` frames with no input.
pub(super) fn frames<S: EngineSide>(h: &mut Headless, app: &mut App<S>, n: usize) {
    for _ in 0..n {
        let _ = h.frame(app, Vec::new());
    }
}

/// The bits of a plane.
pub(super) fn bits(p: &Plane) -> [[u64; 8]; 3] {
    [p.z0, p.q1, p.q2].map(|v| v.map(f64::to_bits))
}

/// The active saved chart's name.
pub(super) fn label<S: EngineSide>(app: &App<S>) -> Option<String> {
    let sim = &app.snapshot().sim;
    app.manifold.charts.label(sim).map(str::to_owned)
}

/// The list's names.
pub(super) fn listed<S: EngineSide>(app: &App<S>) -> Vec<String> {
    let charts = &app.manifold.charts;
    charts.list().map(|c| c.name.clone()).collect()
}

/// The undo depth.
pub(super) fn depth<S: EngineSide>(app: &App<S>) -> u32 {
    app.snapshot().history.undo_depth
}

/// Clicks Save view as chart.
pub(super) fn save<S: EngineSide>(h: &mut Headless, app: &mut App<S>) {
    click(h, app, SAVE_TEXT);
    frames(h, app, 2);
}

/// The state `now` is `then`, the plane bit for bit.
fn unchanged(now: &SimConfig, then: &SimConfig) {
    assert_eq!(now, then);
    assert_eq!(bits(&now.plane), bits(&then.plane), "the view");
}

/// Ctrl+Z.
fn undo(h: &mut Headless, app: &mut App<MockSide>) {
    press(h, app, Key::Z, Modifiers::COMMAND);
    frames(h, app, 2);
}

/// Sets the view tilted to `(30°, 20°)` into `z_q3` under `map`.
pub(super) fn tilt_view(h: &mut Headless, app: &mut App<MockSide>, map: TiltMap) {
    let sim = &app.snapshot().sim;
    let angles = Angles {
        tau1: 30.0,
        tau2: 20.0,
        gamma: 0.0,
    };
    let view = View {
        map,
        ..View::read(sim)
    };
    let frame = SliceFrame {
        tilt_map: map,
        tau1: angles.tau1,
        tau2: angles.tau2,
        ..sim.slice.frame
    };
    let edit = frame_edit(frame, Some(angles.apply(view).basis()));
    app.set_field(edit);
    frames(h, app, 2);
}

/// A directory of its own for test `name`, empty.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "principia-gui-saved-charts-{}-{name}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

#[test]
fn save_view_as_chart_is_one_entry() {
    let (mut app, mut h) = gui(Store::default());
    let (before, quads) = (
        app.snapshot().clone(),
        app.side().engine().integrated().to_vec(),
    );
    save(&mut h, &mut app);
    let check = |app: &mut App<MockSide>| {
        assert_eq!(depth(app), before.history.undo_depth + 1, "one entry");
        assert_eq!(listed(app), ["Saved 1"]);
        assert_eq!(label(app).as_deref(), Some("Saved 1"));
        unchanged(&app.snapshot().sim, &before.sim);
        assert_eq!(
            app.side().engine().integrated(),
            quads,
            "nothing re-integrated"
        );
    };
    check(&mut app);
    undo(&mut h, &mut app);
    let undone = |app: &App<MockSide>| {
        assert_eq!(label(app), None);
        assert_eq!(depth(app), before.history.undo_depth);
        assert_eq!(listed(app), ["Saved 1"], "the list keeps it");
    };
    undone(&app);
    rejects("an undo that takes the list's entry", || {
        app.manifold.charts.set_list(Vec::new());
        undone(&app)
    });
    let (mut twice, mut h) = gui(Store::default());
    save(&mut h, &mut twice);
    let z0 = twice.snapshot().sim.plane.z0;
    twice.set_field(z0_edit(z0));
    frames(&mut h, &mut twice, 2);
    rejects("a save of two entries", || check(&mut twice));
}

#[test]
fn save_view_as_chart_leaves_every_view_as_it_is() {
    for map in [TiltMap::Sheet, TiltMap::Lean] {
        for (gamma, x, y) in [(0.0, 5, 5), (25.0, 5, 5), (0.0, 2, 7), (-40.0, 7, 3)] {
            let (mut app, mut h) = gui(Store::default());
            let sim = app.snapshot().sim.clone();
            let frame = SliceFrame {
                tilt_map: map,
                tilt_x: x,
                tilt_y: y,
                ..sim.slice.frame
            };
            app.set_field(frame_edit(frame, None));
            frames(&mut h, &mut app, 2);
            let to = Angles {
                tau1: 30.0,
                tau2: 20.0,
                gamma,
            };
            let sim = &app.snapshot().sim;
            let class = crate::dispatch::Class::Tilt {
                from: View::read(sim),
                to,
            };
            let edit = crate::dispatch::one(&sim.plane, class);
            app.set_field(edit);
            frames(&mut h, &mut app, 2);
            for _ in 0..2 {
                let then = app.snapshot().sim.clone();
                let read = View::read(&then);
                save(&mut h, &mut app);
                unchanged(&app.snapshot().sim, &then);
                assert_eq!(View::read(&app.snapshot().sim), read);
            }
            let then = app.snapshot().sim.clone();
            let mut frame = then.slice.frame;
            frame.tau1 = 29.0;
            app.set_field(frame_edit(frame, None));
            frames(&mut h, &mut app, 1);
            let now = app.snapshot().sim.clone();
            rejects("a save that moves a frame angle", || unchanged(&now, &then));
        }
    }
}

#[test]
fn save_view_as_chart_follows_the_history() {
    let (mut app, mut h) = gui(Store::default());
    save(&mut h, &mut app);
    undo(&mut h, &mut app);
    let gone = |app: &App<MockSide>| assert_eq!(label(app), None);
    gone(&app);
    app.redo();
    frames(&mut h, &mut app, 2);
    assert_eq!(label(&app).as_deref(), Some("Saved 1"), "redone");
    let redone = app.snapshot().history;
    rejects("a chart active at the depths of the redo", || gone(&app));
    // A zoom and a slice are navigation on the chart.
    let sim = app.snapshot().sim.clone();
    app.set_field(basis_edit((
        sim.plane.q1.map(|v| v * 0.5),
        sim.plane.q2.map(|v| v * 0.5),
    )));
    app.set_field(z0_edit(sim.plane.z0.map(|v| v + 0.1)));
    frames(&mut h, &mut app, 2);
    assert_eq!(label(&app).as_deref(), Some("Saved 1"), "zoomed and sliced");
    // A steer to another plane leaves it; its undo comes back.
    app.set_field(basis_edit((axis(2, 1.0), axis(3, 1.0))));
    frames(&mut h, &mut app, 2);
    assert_eq!(label(&app), None, "steered away");
    undo(&mut h, &mut app);
    assert_eq!(label(&app).as_deref(), Some("Saved 1"), "the steer undone");
    for _ in 0..3 {
        undo(&mut h, &mut app);
    }
    assert_eq!(label(&app), None);
    // An undo then a pan in one drag: the same depths as the redo, not the same entry.
    let figure = crate::layout::Layout::new(h.screen(), h.pixels_per_point()).figure;
    super::mock_layout_splitters::drag(
        &mut h,
        &mut app,
        figure.center(),
        figure.center() + eframe::egui::vec2(40.0, 10.0),
    );
    frames(&mut h, &mut app, 2);
    let now = app.snapshot().history;
    assert_eq!(
        (now.undo_depth, now.redo_depth),
        (redone.undo_depth, redone.redo_depth),
        "the depths alone cannot tell the two"
    );
    gone(&app);
    // A preset picked on the chart's own plane leaves it, in one entry.
    save(&mut h, &mut app);
    assert!(label(&app).is_some());
    let at = depth(&app);
    click_scope(&mut h, &mut app, PRESET);
    frames(&mut h, &mut app, 2);
    let first = format!("{} × {}", AXES[0], AXES[1]);
    click_lowest(&mut h, &mut app, &first);
    assert_eq!(label(&app), None, "a preset picked");
    assert_eq!(depth(&app), at + 1);
}

/// Clicks the topmost node named `name` under the preset selector's row: an entry of its popup.
pub(super) fn click_lowest(h: &mut Headless, app: &mut App<MockSide>, name: &str) {
    let row = app
        .keyboard
        .place_of(PRESET)
        .expect("the selector")
        .rect
        .max
        .y;
    let mut at: Vec<_> = nodes(h, app)
        .into_iter()
        .filter(|n| n.text == name && n.rect.min.y >= row - 1.0)
        .map(|n| n.rect.center())
        .collect();
    at.sort_by(|a, b| a.y.total_cmp(&b.y));
    let at = *at.first().unwrap_or_else(|| {
        panic!("no `{name}` in the popup");
    });
    for events in crate::capture::click(at) {
        let _ = h.frame(app, events);
    }
    frames(h, app, 2);
}

#[test]
fn save_view_as_chart_by_the_keyboard() {
    let play = |keys: &[Key]| {
        let (mut app, mut h) = gui(Store::default());
        for key in keys {
            press(&mut h, &mut app, *key, NONE);
        }
        frames(&mut h, &mut app, 2);
        app
    };
    let check = |app: &App<MockSide>| {
        assert_eq!(listed(app), ["Saved 1"]);
        assert_eq!(depth(app), 1);
    };
    use Key::{ArrowDown as Down, ArrowRight as Right, Enter, Tab};
    // Down from the buttons' row reaches the Links row's Decoder…, so the button is Right of Chart builder….
    let button = [Tab, Tab, Enter, Enter, Down, Down, Down, Right, Enter];
    check(&play(&button));
    rejects("the keys one short of the button", || {
        check(&play(&[Tab, Tab, Enter, Enter, Down, Down, Down, Enter]))
    });
    let entry = [Tab, Enter, Right, Enter, Down, Down, Down, Down, Enter];
    check(&play(&entry));
    rejects("the keys one short of View's entry", || {
        check(&play(&[Tab, Enter, Right, Enter, Down, Down, Down, Enter]))
    });
    // View's entry is after Debug view and before reset layout, the button after Chart builder….
    let (app, _) = gui(Store::default());
    let tree = app.keyboard.tree(app.view.mode);
    let view = tree.children(Some("view"));
    let at = |id| view.iter().position(|v| *v == id).expect("an entry");
    use crate::explore::top_bar as bar;
    assert_eq!(at(bar::SAVE_VIEW.0), at(bar::DEBUG_VIEW.0) + 1);
    assert_eq!(at(bar::RESET_LAYOUT.0), at(bar::SAVE_VIEW.0) + 1);
    let chart = tree.children(Some(crate::explore::manifold_view::CHART));
    let decoder = crate::explore::manifold_view::DECODER;
    assert_eq!(chart[chart.len() - 3..], [BUILDER, saved::SAVE, decoder]);
}

#[test]
fn save_view_as_chart_rename_sticks_across_a_relaunch() {
    let dir = scratch("rename");
    let path = dir.join(FILE);
    let (mut app, mut h) = gui(Store::at(&path));
    tilt_view(&mut h, &mut app, TiltMap::Lean);
    save(&mut h, &mut app);
    click_scope(&mut h, &mut app, RENAME);
    frames(&mut h, &mut app, 3);
    let _ = h.frame(&mut app, vec![Event::Text(" b".to_owned())]);
    press(&mut h, &mut app, Key::Enter, NONE);
    frames(&mut h, &mut app, 2);
    assert_eq!(listed(&app), ["Saved 1 b"]);
    assert_eq!(label(&app).as_deref(), Some("Saved 1 b"));
    assert!(app.manifold.charts.naming.is_none(), "Enter ends it");
    assert_eq!(depth(&app), 2, "a rename is no entry");
    let saved = bits(&app.snapshot().sim.plane);
    let check = |app: &App<MockSide>| {
        assert_eq!(listed(app), ["Saved 1 b"]);
        let chart = app.manifold.charts.list().next().expect("one");
        assert_eq!(bits(&chart.plane), saved);
    };
    check(&gui(Store::at(&path)).0);
    rejects("a GUI that forgets its settings", || {
        check(&gui(Store::default()).0)
    });
    // The relaunched GUI opens no saved chart by itself, and reopens it bit for bit.
    let (mut again, mut h) = gui(Store::at(&path));
    assert_eq!(label(&again), None);
    click_scope(&mut h, &mut again, PRESET);
    frames(&mut h, &mut again, 2);
    click_lowest(&mut h, &mut again, "Saved 1 b");
    assert_eq!(label(&again).as_deref(), Some("Saved 1 b"));
    assert_eq!(bits(&again.snapshot().sim.plane), saved);
    assert_eq!(depth(&again), 1, "picked in one entry");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn save_view_as_chart_shows_its_provenance() {
    let shown = |map| {
        let (mut app, mut h) = gui(Store::default());
        tilt_view(&mut h, &mut app, map);
        save(&mut h, &mut app);
        frame_texts(&mut h, &mut app)
            .into_iter()
            .find(|t| t.contains("from z_α × z_β"))
            .expect("the provenance")
    };
    let check = |text: &str, map: &str| {
        for part in [
            &format!("{map}: τ₁ 30.0° into z_q3, τ₂ 20.0° into z_q3, γ 0.0°"),
            "z₀ (0.180, 0.318, ",
            "q₁ 0.87·z_α",
            "q₂ ",
        ] {
            assert!(text.contains(part), "{part} in {text}");
        }
    };
    let lean = shown(TiltMap::Lean);
    check(&lean, "Lean each axis");
    let skewed = |text: &str| assert!(text.contains("not orthogonal (cos 0.171)"), "{text}");
    skewed(&lean);
    let sheet = shown(TiltMap::Sheet);
    assert!(sheet.contains("Tip as sheet: τ₁ 30.0° into z_q3"));
    rejects("a rigid turn's pair called skewed", || skewed(&sheet));
    rejects("another map's provenance", || {
        check(&sheet, "Lean each axis")
    });
}

#[test]
fn save_view_as_chart_delete() {
    let (mut app, mut h) = gui(Store::default());
    save(&mut h, &mut app);
    save(&mut h, &mut app);
    assert_eq!(listed(&app), ["Saved 1", "Saved 2"]);
    let sim = app.snapshot().sim.clone();
    let quads = app.side().engine().integrated().to_vec();
    let unchanged = |app: &mut App<MockSide>| {
        assert_eq!(depth(app), 2, "no entry");
        assert_eq!(app.snapshot().sim, sim, "the state");
        assert_eq!(app.side().engine().integrated(), quads, "the quads");
    };
    // The inactive one, from the picker's first entry.
    click_scope(&mut h, &mut app, PRESET);
    frames(&mut h, &mut app, 2);
    click_lowest(&mut h, &mut app, DELETE_TEXT);
    assert_eq!(listed(&app), ["Saved 2"]);
    let still = |app: &App<MockSide>| assert_eq!(label(app).as_deref(), Some("Saved 2"));
    still(&app);
    unchanged(&mut app);
    // The active one.
    press(&mut h, &mut app, Key::Escape, NONE);
    click_scope(&mut h, &mut app, DELETE);
    frames(&mut h, &mut app, 2);
    assert!(listed(&app).is_empty());
    let gone = |app: &App<MockSide>| assert_eq!(label(app).as_deref(), Some(UNSAVED));
    gone(&app);
    unchanged(&mut app);
    let all = texts(&names(&mut h, &mut app));
    assert!(all.iter().any(|t| t == UNSAVED), "the picker: {all:?}");
    assert!(all.iter().any(|t| t.contains("from Saved 1")), "{all:?}");
    // Saved again, it is in the list again.
    save(&mut h, &mut app);
    assert_eq!(listed(&app), ["Saved 1"]);
    assert_eq!(label(&app).as_deref(), Some("Saved 1"));
    rejects("a delete that adds an entry", || unchanged(&mut app));
    // The history's first entry names a deleted chart too; before it, no chart.
    undo(&mut h, &mut app);
    undo(&mut h, &mut app);
    gone(&app);
    undo(&mut h, &mut app);
    rejects("a delete that switches the active chart", || gone(&app));
}

/// A chart of `plane`.
fn chart_of(plane: Plane) -> SavedChart {
    SavedChart {
        name: "a \"name\"".to_owned(),
        plane,
        frame: super::support::SLICE_FRAME,
        parent: "z_α × z_β".to_owned(),
        angles: Angles {
            tau1: 0.1 + 0.2,
            tau2: -0.0,
            gamma: 1.0 / 3.0,
        },
        pin: None,
    }
}

#[test]
fn save_view_as_chart_settings_read_back_bit_for_bit() {
    let hard: Latent = [
        -0.0,
        5e-324,
        0.1 + 0.2,
        1.0 / 3.0,
        -1.7976931348623157e308,
        2f64.sqrt() * 1e-9,
        std::f64::consts::PI,
        9007199254740993.0,
    ];
    let mut rng = 0x9e37_79b9_7f4a_7c15u64;
    let mut random = || -> Latent {
        std::array::from_fn(|_| {
            rng = rng.wrapping_mul(0xd134_2543_de82_ef95).wrapping_add(1);
            (rng >> 11) as f64 / (1u64 << 53) as f64 - 0.5
        })
    };
    let check = |settings: &Settings, plane: &Plane| {
        let back = Settings::from_json(&settings.to_json()).expect("read");
        assert_eq!(&back, settings);
        assert_eq!(bits(&back.saved_charts[0].plane), bits(plane));
        let angles = back.saved_charts[0].angles;
        assert_eq!(angles.tau2.to_bits(), (-0.0f64).to_bits());
    };
    let settings = |plane: &Plane| Settings {
        saved_charts: vec![chart_of(plane.clone())],
        ..Settings::default()
    };
    for i in 0..200 {
        let plane = Plane {
            z0: if i == 0 { hard } else { random() },
            q1: random(),
            q2: random(),
        };
        check(&settings(&plane), &plane);
        let short = Plane {
            z0: plane.z0.map(|v| f64::from(v as f32)),
            ..plane.clone()
        };
        rejects("a vector stored as f32", || {
            check(&settings(&short), &plane)
        });
    }
    assert!(!Settings::default().to_json().contains("saved_charts"));
    for bad in [
        r#"{"saved_charts": {}}"#,
        r#"{"saved_charts": [{"name": "x"}]}"#,
        r#"{"saved_charts": [1]}"#,
    ] {
        assert!(Settings::from_json(bad).is_err(), "{bad}");
    }
    // A target that is no latent dimension is no chart.
    let mut value = chart_of(Plane {
        z0: hard,
        q1: hard,
        q2: hard,
    })
    .to_json();
    assert!(SavedChart::from_json(&value).is_some());
    value["frame"]["tilt_x"] = 8.into();
    assert!(SavedChart::from_json(&value).is_none());
}

#[test]
fn save_view_as_chart_on_the_real_side() {
    let (sim, render) = crate::real::start();
    let mut app = App::new(RealSide::new(sim, render), FORMAT);
    let mut h = headless();
    frames(&mut h, &mut app, 2);
    let before = app.snapshot().sim.clone();
    let check = |app: &App<RealSide>| {
        assert_eq!(listed(app), ["Saved 1"]);
        assert_eq!(label(app).as_deref(), Some("Saved 1"));
        assert_eq!(depth(app), 1);
        unchanged(&app.snapshot().sim, &before);
    };
    rejects("a real side not clicked", || check(&app));
    save(&mut h, &mut app);
    check(&app);
    app.undo();
    frames(&mut h, &mut app, 2);
    assert_eq!(label(&app), None);
    assert_eq!(listed(&app), ["Saved 1"]);
    assert_eq!(depth(&app), 0);
}

/// What the app sends is followed by the engine's own rule: an edit under the open key joins its entry, an undo closes it.
#[test]
fn save_view_as_chart_follows_keys_as_the_history_does() {
    use engine::contract::set_field::HistoryKey;
    let chart = Some((
        1,
        chart_of(Plane {
            z0: [0.0; 8],
            q1: axis(0, 1.0),
            q2: axis(1, 1.0),
        }),
    ));
    let key = HistoryKey(7);
    let mut charts = Charts::default();
    charts.edited(false, Some(key));
    charts.mark(chart);
    charts.edited(false, Some(key));
    charts.edited(true, None);
    charts.edited(false, Some(key));
    charts.undone();
    assert!(!charts.is_marked(), "the key's edits are one entry");
    charts.redone();
    assert!(charts.is_marked());
    charts.ended(key);
    charts.edited(false, Some(key));
    charts.undone();
    assert!(charts.is_marked(), "an ended key's next edit is another");
    charts.undone();
    charts.edited(false, None);
    assert!(!charts.is_marked(), "an edit after the undo drops the redo");
    rejects(
        "a mark kept past a new edit",
        || assert!(charts.is_marked()),
    );
    let mut idle = MockEngine::new();
    idle.undo();
    charts.undone();
    charts.undone();
    charts.edited(false, None);
    assert!(!charts.is_marked(), "an undo of nothing is nothing");
}
