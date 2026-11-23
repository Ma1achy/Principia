//! `lock_moves_no_view` (TASK-M8-125; render_gui_spec §G4; chart_decoder_contract Part 4, "Setting the lock"): locking
//! through the GUI does not move the view; centre on pin brings the pin to the centre as one undoable edit; the setting
//! Centre on pin when locking, off by default, makes the lock and its centre on pin one command. The GUI's routes are
//! adapters onto the engine's lock leaf (`engine::path::leaf`), holding no formula.
//!
//! **The criteria**, written before the first run:
//! - **The lock moves no view.** Through `lock_at` on three planes (the mock's home, an oblique one, a skew one) at
//!   three off-centre pixels: `z₀`, the basis and the window bit for bit; `z_locked` the kernel's `Φ` at the pixel
//!   (`engine::lock::point`) bit for bit; the route's command the engine leaf's, `CentreOnPin::Off`, bit for bit (the
//!   engine's lock arms read through the GUI). Through K on the mock's figure: the same, one history entry, and the
//!   reticle within 10⁻³ points of the point pressed.
//! - **Centre on pin**, after an off-centre K: one command, one history entry; `z₀ = z_locked` bit for bit, the pin's
//!   chart point exactly `(½, ½)`, the basis kept; one undo restores the locked view bit for bit. With no lock leaf's pin
//!   held (the pin read from the snapshot), on the three planes: the pin's chart point within 10⁻¹² of `(½, ½)`. After
//!   an excursion `δ = 0.05` along `z_q0`, hidden from the mock's plane: `z₀` within 10⁻¹² of `z_locked + δ·e_q0`, the
//!   excursion kept (chart_decoder_contract Part 4: centre on pin clears the in-plane part only).
//! - **The setting** is off by default: the panel's scratch, a new app, the default settings and an empty settings file.
//!   On, K gives the state K then centre on pin gives, bit for bit, in one history entry where the two take two; one
//!   undo returns the state before the lock bit for bit. Saved, it reads back on.
//! - **Zoom about the pin.** The wheel after an off-centre K keeps the anchor's chart point within 10⁻¹².
//!
//! **Controls**: a lock that re-centres (the setting on) fails the first; a centre on pin holding the GUI's own formula,
//! `z₀ ← z_locked`, fails the excursion arm; the setting's two edits sent as two commands fail one history entry; the
//! wheel's zoom not turned about the pin fails the last.

use eframe::egui::{self, Event, Key, Modifiers, MouseWheelUnit};
use engine::contract::set_field::{Edit, SetField, SimField};
use engine::contract::sim_config::{Latent, Lock, Plane, SimConfig};
use engine::contract::snapshot::Snapshot;
use engine::path::leaf::{self, Affine, CentreOnPin};
use engine::path::stop;

use super::support::{click, headless, rejects};
use crate::app::App;
use crate::capture::{lock_point, FORMAT, LOCK_AT, PIXELS_PER_POINT};
use crate::dispatch::{self, Class};
use crate::explore::lock::{reticle_at, CENTRE_ON_PIN};
use crate::explore::manifold_view::{
    self as mv, chart_coords, lock as mv_lock, navigate, Out, Scratch,
};
use crate::headless::Headless;
use crate::layout::Layout;
use crate::mock::{MockEngine, MOCK_Q1, MOCK_Q2, MOCK_Z0};
use crate::settings::Settings;
use crate::side::MockSide;

/// The off-centre pixels.
const PINS: [(f64, f64); 3] = [(0.3, 0.8), (0.71, 0.13), (0.62, 0.21)];

/// The excursion along `z_q0`, hidden from the mock's plane.
const DELTA: f64 = 0.05;

/// `z_q0`'s index.
const Q0: usize = 2;

/// The planes: the mock's home, an oblique plane, a skew one.
fn planes() -> [Plane; 3] {
    let home = Plane {
        z0: MOCK_Z0,
        q1: MOCK_Q1,
        q2: MOCK_Q2,
    };
    let oblique = Plane {
        z0: [0.21, -0.37, 0.13, 0.04, -0.29, 0.08, 0.31, 0.27],
        q1: [0.3, 0.11, 0.05, 0.0, -0.07, 0.0, 0.0, 0.0],
        q2: [-0.02, 0.25, 0.0, 0.04, 0.0, 0.09, 0.0, 0.0],
    };
    let skew = Plane {
        z0: [-0.4, 0.9, 0.0, 0.33, 0.1, -0.2, 0.5, 0.15],
        q1: [0.07, 0.0, 0.31, -0.13, 0.0, 0.02, 0.0, 0.0],
        q2: [0.07, 0.01, 0.29, 0.0, 0.17, 0.0, 0.0, 0.0],
    };
    [home, oblique, skew]
}

/// The edits written exactly, `−0` and every bit of each f64 (`Debug` writes them so).
fn exact(edits: &[SetField]) -> String {
    format!("{edits:?}")
}

/// `sim` after `edits`, as the state store writes them.
fn applied(sim: &SimConfig, edits: &[SetField]) -> SimConfig {
    let mut out = sim.clone();
    for e in edits {
        match &e.edit {
            Edit::Sim(SimField::Z0(z0)) => out.plane.z0 = *z0,
            Edit::Sim(SimField::Basis { q1, q2 }) => (out.plane.q1, out.plane.q2) = (*q1, *q2),
            Edit::Sim(SimField::Lock(lock)) => out.lock = lock.clone(),
            Edit::Sim(SimField::Window(window)) => out.chart.window = *window,
            other => panic!("a lock edits no {other:?}"),
        }
    }
    out
}

/// `sim` with `plane`.
fn on(plane: &Plane) -> SimConfig {
    let mut sim = mock_app().snapshot().sim.clone();
    sim.plane = plane.clone();
    sim
}

/// The lock route's one command at `at` on `sim`, with the setting `setting`, and the scratch it leaves.
fn lock_command(sim: &SimConfig, at: (f64, f64), setting: CentreOnPin) -> (Vec<SetField>, Scratch) {
    let mut scratch = Scratch {
        centre_on_pin: setting,
        ..Scratch::default()
    };
    let mut out = Out::default();
    mv_lock::lock_at(&sim.plane, at, &mut scratch, &mut out);
    assert!(out.edits.is_empty(), "the lock's edits are one command");
    assert_eq!(out.commands.len(), 1, "one command");
    (out.commands.remove(0), scratch)
}

/// That the lock at `at` on `sim`, its command `command`, moved no view and is the engine leaf's.
fn check_no_view(sim: &SimConfig, at: (f64, f64), command: &[SetField]) {
    let after = applied(sim, command);
    let bits = |z: &Latent| z.map(f64::to_bits);
    assert_eq!(bits(&after.plane.z0), bits(&sim.plane.z0), "z₀ moved");
    assert_eq!(
        (bits(&after.plane.q1), bits(&after.plane.q2)),
        (bits(&sim.plane.q1), bits(&sim.plane.q2)),
        "the basis moved"
    );
    assert_eq!(after.chart.window, sim.chart.window, "the window moved");
    assert!(after.lock.locked, "locked");
    assert_eq!(
        bits(&after.lock.z_locked),
        bits(&engine::lock::point(&sim.plane, at.0, at.1)),
        "z_locked is not Φ at the pixel"
    );
}

#[test]
fn lock_moves_no_view_through_the_route() {
    let gate = stop::affine();
    for plane in planes() {
        let sim = on(&plane);
        for at in PINS {
            let (command, _) = lock_command(&sim, at, CentreOnPin::default());
            check_no_view(&sim, at, &command);
            let view = engine::path::View::latent(sim.clone());
            let leaf = leaf::lock(&view, &Affine, at, CentreOnPin::Off, &gate).expect("a lock");
            assert_eq!(exact(&command), exact(leaf.edits()), "not the leaf's");
            let (recentred, _) = lock_command(&sim, at, CentreOnPin::On);
            rejects("a lock that re-centres", || {
                check_no_view(&sim, at, &recentred)
            });
        }
    }
}

// ── Through the app ────────────────────────────────────────────────────────────────────────────────────────────────

fn mock_app() -> App<MockSide> {
    App::new(MockSide::new(MockEngine::frozen(), None), FORMAT)
}

struct Rig {
    app: App<MockSide>,
    h: Headless,
    layout: Layout,
}

impl Rig {
    fn new(setting: CentreOnPin) -> Self {
        let mut rig = Rig {
            app: mock_app(),
            h: headless(),
            layout: Layout::new(headless().screen(), PIXELS_PER_POINT),
        };
        rig.app.manifold.centre_on_pin = setting;
        rig.frames(2);
        rig
    }

    fn frames(&mut self, n: usize) {
        for _ in 0..n {
            let _ = self.h.frame(&mut self.app, Vec::new());
        }
    }

    /// K with the pointer at the `lock` step's point, off the centre.
    fn press_k(&mut self) -> egui::Pos2 {
        let at = lock_point(self.layout.figure);
        let _ = self.h.frame(&mut self.app, vec![Event::PointerMoved(at)]);
        for events in crate::capture::key(Key::K, Modifiers::NONE) {
            let _ = self.h.frame(&mut self.app, events);
        }
        self.frames(1);
        at
    }

    fn snapshot(&self) -> Snapshot {
        self.app.snapshot().clone()
    }

    fn centre_on_pin(&mut self) {
        click(&mut self.h, &mut self.app, CENTRE_ON_PIN);
        self.frames(1);
    }

    fn undo(&mut self) {
        self.app.undo();
        self.frames(1);
    }
}

#[test]
fn lock_moves_no_view_through_k() {
    let mut rig = Rig::new(CentreOnPin::Off);
    let before = rig.snapshot();
    let pressed = rig.press_k();
    let after = rig.snapshot();
    let (s, t) = chart_coords(&before.sim.plane, &after.sim.lock.z_locked).expect("a chart");
    // The point K read: the field's `(s, t)` under the pointer.
    let at = navigate::figure_coords(rig.layout.field, pressed);
    assert!(
        (s - at.0).abs() < 1e-9 && (t - at.1).abs() < 1e-9,
        "the anchor at ({s}, {t}), the point pressed {at:?}"
    );
    // The app's state is the lock group alone applied to the state before it.
    let lock = [mv::lock_edit(after.sim.lock.clone())];
    check_no_view(&before.sim, at, &lock);
    assert_eq!(after.sim.plane, before.sim.plane, "the plane moved");
    assert_eq!(after.sim.chart, before.sim.chart, "the window moved");
    assert_eq!(after.history.undo_depth, before.history.undo_depth + 1);
    let reticle = reticle_at(&after.sim.lock, &after.sim.plane, rig.layout.field).expect("shown");
    assert!(
        reticle.distance(pressed) < 1e-3,
        "the reticle at {reticle:?}, pressed at {pressed:?}"
    );
    // Control: with the setting on, K re-centres.
    let mut snap = Rig::new(CentreOnPin::On);
    snap.press_k();
    let moved = snap.snapshot();
    rejects("a lock that re-centres", || {
        assert_eq!(moved.sim.plane, before.sim.plane)
    });
}

#[test]
fn lock_moves_no_view_centre_on_pin() {
    let mut rig = Rig::new(CentreOnPin::Off);
    rig.press_k();
    let locked = rig.snapshot();
    rig.centre_on_pin();
    let centred = rig.snapshot();
    let bits = |z: &Latent| z.map(f64::to_bits);
    assert_eq!(
        bits(&centred.sim.plane.z0),
        bits(&locked.sim.lock.z_locked),
        "z₀ ← z_locked"
    );
    assert_eq!(
        chart_coords(&centred.sim.plane, &centred.sim.lock.z_locked),
        Some((0.5, 0.5)),
        "the pin at the centre"
    );
    assert_eq!(
        (centred.sim.plane.q1, centred.sim.plane.q2),
        (locked.sim.plane.q1, locked.sim.plane.q2),
        "the basis kept"
    );
    assert_eq!(centred.sim.lock, locked.sim.lock, "the lock kept");
    assert_eq!(
        centred.history.undo_depth,
        locked.history.undo_depth + 1,
        "one undoable edit"
    );
    let reticle =
        reticle_at(&centred.sim.lock, &centred.sim.plane, rig.layout.field).expect("shown");
    assert!(
        reticle.distance(rig.layout.field.center()) < 1e-3,
        "{reticle:?}"
    );
    rig.undo();
    assert_eq!(
        rig.snapshot().sim,
        locked.sim,
        "undo restores the lock's view"
    );
    rejects("the view before centre on pin", || {
        assert_eq!(
            chart_coords(&locked.sim.plane, &locked.sim.lock.z_locked),
            Some((0.5, 0.5))
        )
    });
}

/// Centre on pin with the pin read from the snapshot, no lock leaf's pin held, on `sim` after a lock at `at` and the
/// edits `then`: `z₀` after it.
fn centred_from_snapshot(sim: &SimConfig, at: (f64, f64), then: &[SetField]) -> SimConfig {
    let (command, _) = lock_command(sim, at, CentreOnPin::Off);
    let locked = applied(&applied(sim, &command), then);
    let mut out = Out::default();
    mv_lock::centre_on_pin(&locked, &Scratch::default(), &mut out);
    assert!(out.edits.is_empty());
    assert_eq!(out.commands.len(), 1, "one command");
    applied(&locked, &out.commands[0])
}

#[test]
fn lock_moves_no_view_centre_on_pin_read_from_the_snapshot() {
    for plane in planes() {
        let sim = on(&plane);
        for at in PINS {
            let centred = centred_from_snapshot(&sim, at, &[]);
            let (s, t) = chart_coords(&centred.plane, &centred.lock.z_locked).expect("a chart");
            assert!(
                (s - 0.5).abs() <= 1e-12 && (t - 0.5).abs() <= 1e-12,
                "the pin at ({s}, {t})"
            );
        }
    }
}

/// The GUI's own formula for centre on pin, the control: `z₀ ← z_locked`.
fn own_formula(sim: &SimConfig) -> SimConfig {
    let mut out = sim.clone();
    out.plane.z0 = sim.lock.z_locked;
    out
}

#[test]
fn lock_moves_no_view_centre_on_pin_keeps_the_excursion() {
    let sim = on(&planes()[0]);
    for at in PINS {
        let (command, _) = lock_command(&sim, at, CentreOnPin::Off);
        let locked = applied(&sim, &command);
        let value = locked.plane.z0[Q0] + DELTA;
        let slide = dispatch::send(&locked.plane, Class::Control { k: Q0, value });
        let slid = applied(&locked, &slide);
        let mut want = locked.lock.z_locked;
        want[Q0] += DELTA;
        let check = |got: &SimConfig| {
            let far = got
                .plane
                .z0
                .iter()
                .zip(&want)
                .map(|(g, w)| (g - w).abs())
                .fold(0.0, f64::max);
            assert!(far <= 1e-12, "z₀ {:?} is {far} from {want:?}", got.plane.z0);
        };
        check(&centred_from_snapshot(&sim, at, &slide));
        rejects("a centre on pin of the GUI's own formula", || {
            check(&own_formula(&slid))
        });
    }
}

#[test]
fn lock_moves_no_view_the_setting_is_off_by_default() {
    assert_eq!(Scratch::default().centre_on_pin, CentreOnPin::Off);
    assert_eq!(mock_app().manifold.centre_on_pin, CentreOnPin::Off);
    assert_eq!(Settings::default().centre_on_pin, CentreOnPin::Off);
    assert_eq!(
        Settings::from_json("{}").map(|s| s.centre_on_pin),
        Ok(CentreOnPin::Off)
    );
    let on = Settings {
        centre_on_pin: CentreOnPin::On,
        ..Settings::default()
    };
    assert_eq!(Settings::from_json(&on.to_json()), Ok(on.clone()));
    rejects("the setting lost on saving", || {
        assert_eq!(
            Settings::from_json(&Settings::default().to_json()),
            Ok(on.clone())
        )
    });
    let mut scratch = Scratch::default();
    let mut out = Out::default();
    mv::activate(
        mv::CENTRE_ON_PIN_SETTING,
        mock_app().snapshot(),
        &mut scratch,
        &mut out,
    );
    assert_eq!(scratch.centre_on_pin, CentreOnPin::On, "Enter turns it on");
    assert!(out.save_settings, "and saves it");
    assert!(
        out.edits.is_empty() && out.commands.is_empty(),
        "not undoable"
    );
}

#[test]
fn lock_moves_no_view_the_setting_is_lock_then_centre_on_pin_in_one_entry() {
    let mut snap = Rig::new(CentreOnPin::On);
    let before = snap.snapshot();
    snap.press_k();
    let snapped = snap.snapshot();
    let mut off = Rig::new(CentreOnPin::Off);
    off.press_k();
    off.centre_on_pin();
    let two = off.snapshot();
    assert_eq!(
        format!("{:?}", snapped.sim),
        format!("{:?}", two.sim),
        "the setting's state is lock then centre on pin's, bit for bit"
    );
    let entries = |s: &Snapshot| s.history.undo_depth - before.history.undo_depth;
    assert_eq!(entries(&snapped), 1, "one history entry");
    assert_eq!(entries(&two), 2, "lock then centre on pin, two");
    snap.undo();
    assert_eq!(snap.snapshot().sim, before.sim, "one undo before the lock");
    // Control: the setting's two edits sent as two commands are two entries.
    let mut split = Rig::new(CentreOnPin::Off);
    let (command, _) = lock_command(&split.snapshot().sim, LOCK_AT_ST, CentreOnPin::On);
    assert_eq!(command.len(), 2, "z₀ and the lock");
    let start = split.snapshot().history.undo_depth;
    for edit in command {
        split.app.set_field(edit);
    }
    split.frames(1);
    let depth = split.snapshot().history.undo_depth - start;
    rejects("the setting's edits as two commands", || {
        assert_eq!(depth, 1)
    });
}

/// The `lock` step's point, `(s, t)`.
const LOCK_AT_ST: (f64, f64) = (LOCK_AT.0 as f64, LOCK_AT.1 as f64);

#[test]
fn lock_moves_no_view_zoom_about_the_pin() {
    let mut rig = Rig::new(CentreOnPin::Off);
    let pressed = rig.press_k();
    let locked = rig.snapshot();
    let pin = chart_coords(&locked.sim.plane, &locked.sim.lock.z_locked).expect("a chart");
    let wheel = Event::MouseWheel {
        unit: MouseWheelUnit::Point,
        delta: egui::vec2(0.0, 120.0),
        phase: egui::TouchPhase::Move,
        modifiers: Modifiers::NONE,
    };
    let _ = rig
        .h
        .frame(&mut rig.app, vec![Event::PointerMoved(pressed), wheel]);
    rig.frames(1);
    let zoomed = rig.snapshot();
    assert_ne!(zoomed.sim.plane.q1, locked.sim.plane.q1, "the wheel zoomed");
    let kept = |sim: &SimConfig| {
        let (s, t) = chart_coords(&sim.plane, &sim.lock.z_locked).expect("a chart");
        assert!(
            (s - pin.0).abs() <= 1e-12 && (t - pin.1).abs() <= 1e-12,
            "the pin from {pin:?} to ({s}, {t})"
        );
    };
    kept(&zoomed.sim);
    // Control: the same zoom not turned about the pin.
    let mut bare = locked.sim.clone();
    let Edit::Sim(SimField::Basis { q1, q2 }) = navigate::zoom_by(&locked.sim.plane, 0.5).edit
    else {
        panic!("a zoom edits the basis")
    };
    (bare.plane.q1, bare.plane.q2) = (q1, q2);
    rejects("a zoom about the centre", || kept(&bare));
}

#[test]
fn lock_moves_no_view_the_lock_group_is_set_whole() {
    // The lock route sends a `SetField` on the lock group whole: the flag and the anchor, nothing on the view.
    let sim = on(&planes()[1]);
    let (command, _) = lock_command(&sim, PINS[0], CentreOnPin::Off);
    assert!(
        matches!(
            command.as_slice(),
            [SetField {
                edit: Edit::Sim(SimField::Lock(Lock { locked: true, .. })),
                ..
            }]
        ),
        "{command:?}"
    );
}
