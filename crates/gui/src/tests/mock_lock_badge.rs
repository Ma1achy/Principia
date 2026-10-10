//! `mock_lock_badge` (REQ-GUI-171; render_gui_spec §G2, §G4; chart_decoder_contract Part 4, "The lock"): K over a
//! point of the mock's figure locks there and recentres on it; the gold reticle sits at the figure's centre; the
//! compass carries the gold pin; the badge reads "● locked at z_locked" with the anchor's (fake) value; unlock emits the
//! `SetField` that clears the lock; open in Inspector requests the Inspector window. Each check also runs on an input
//! it must reject (R-176, through `rejects`).

use std::sync::Arc;

use eframe::egui::{self, Event, Key, Modifiers};
use engine::contract::set_field::{Edit, SimField};
use engine::contract::sim_config::Lock;
use engine::contract::view_ui::Window;

use super::support::{click, frame_texts, headless, rejects};
use crate::app::App;
use crate::capture::{lock_point, render_frame, Shot, FORMAT, LOCK_AT, PIXELS_PER_POINT};
use crate::explore::lock::{
    anchor_text, reticle_at, GOLD, LOCKED, OPEN_INSPECTOR, RETICLE_R, UNLOCK, UNLOCKED,
};
use crate::explore::manifold_view::point;
use crate::headless::Headless;
use crate::layout::Layout;
use crate::mock::canvas::MockCanvas;
use crate::mock::MockEngine;
use crate::side::MockSide;

struct Rig {
    canvas: Option<Arc<MockCanvas>>,
    app: App<MockSide>,
    headless: Headless,
    layout: Layout,
}

fn new_rig(with_canvas: bool) -> Rig {
    let canvas = with_canvas
        .then(|| Arc::new(MockCanvas::new().expect("a GPU adapter for the mock's canvas")));
    let app = App::new(MockSide::new(MockEngine::frozen(), canvas.clone()), FORMAT);
    let headless = headless();
    let layout = Layout::new(headless.screen(), PIXELS_PER_POINT);
    let mut rig = Rig {
        canvas,
        app,
        headless,
        layout,
    };
    for _ in 0..2 {
        let _ = rig.headless.frame(&mut rig.app, Vec::new());
    }
    rig
}

impl Rig {
    /// K with the pointer at `at`, as the `lock` step presses it.
    fn press_k_at(&mut self, at: egui::Pos2) {
        let _ = self
            .headless
            .frame(&mut self.app, vec![Event::PointerMoved(at)]);
        for events in crate::capture::key(Key::K, Modifiers::NONE) {
            let _ = self.headless.frame(&mut self.app, events);
        }
        let _ = self.headless.frame(&mut self.app, Vec::new());
    }

    fn lock(&mut self) {
        let at = lock_point(self.layout.figure);
        self.press_k_at(at);
    }

    fn shot(&mut self) -> Shot {
        let canvas = self.canvas.clone().expect("a rig with a canvas");
        render_frame(&canvas, &mut self.app, &mut self.headless)
    }
}

fn is_gold(p: [u8; 4]) -> bool {
    let g = GOLD.to_array();
    (0..3).all(|c| p[c].abs_diff(g[c]) <= 6)
}

/// The gold pixels of `shot` inside `rect` (in points).
fn gold_in(shot: &Shot, rect: egui::Rect) -> usize {
    let px = |v: f32| (v * PIXELS_PER_POINT).round() as usize;
    let mut n = 0;
    for y in px(rect.min.y)..px(rect.max.y) {
        for x in px(rect.min.x)..px(rect.max.x) {
            let i = (y * shot.size[0] as usize + x) * 4;
            if is_gold(shot.rgba[i..i + 4].try_into().unwrap()) {
                n += 1;
            }
        }
    }
    n
}

#[test]
fn mock_lock_badge_k_over_a_point_locks_there_and_recentres() {
    let mut rig = new_rig(false);
    let before = rig.app.snapshot().clone();
    let anchor = point(
        &before.sim.plane,
        f64::from(LOCK_AT.0),
        f64::from(LOCK_AT.1),
    );
    rig.lock();
    let after = rig.app.snapshot().clone();
    let close = |a: &[f64; 8], b: &[f64; 8]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-6);
    assert!(after.sim.lock.locked, "K locked");
    assert!(
        close(&after.sim.lock.z_locked, &anchor),
        "at the point under the pointer: {:?}",
        after.sim.lock.z_locked
    );
    assert!(
        close(&after.sim.plane.z0, &anchor),
        "and recentred on it: {:?}",
        after.sim.plane.z0
    );
    assert_eq!(
        (after.sim.plane.q1, after.sim.plane.q2),
        (before.sim.plane.q1, before.sim.plane.q2),
        "the basis kept"
    );
    assert_eq!(
        after.history.undo_depth,
        before.history.undo_depth + 2,
        "the recentre and the lock, each undoable"
    );
    // Control: K with the pointer off the figure locks nothing.
    let mut off = new_rig(false);
    let outside = off.layout.compass.center();
    off.press_k_at(outside);
    let free = off.app.snapshot().sim.lock.clone();
    rejects("K off the figure", || assert!(free.locked, "K locked"));
    // Control: the figure's centre is not the point under the pointer.
    let centre = point(&before.sim.plane, 0.5, 0.5);
    rejects("the figure's centre", || {
        assert!(close(&after.sim.lock.z_locked, &centre))
    });
}

#[test]
fn mock_lock_badge_the_reticle_sits_at_the_centre() {
    let mut rig = new_rig(true);
    let figure = rig.layout.figure;
    let unlocked = rig.shot();
    rig.lock();
    let snapshot = rig.app.snapshot().clone();
    let at =
        reticle_at(&snapshot.sim.lock, &snapshot.sim.plane, figure).expect("the reticle in view");
    assert!(
        at.distance(figure.center()) < 1e-3,
        "the reticle at {at:?}, the centre {:?}",
        figure.center()
    );
    // Its ring is drawn: gold about the centre, on the figure.
    let ring =
        egui::Rect::from_center_size(figure.center(), egui::Vec2::splat(2.0 * RETICLE_R + 4.0));
    let locked = rig.shot();
    assert!(gold_in(&locked, ring) > 40, "the gold ring at the centre");
    rejects("the figure before locking", || {
        assert!(gold_in(&unlocked, ring) > 40)
    });
    let corner = egui::Rect::from_min_size(figure.min + egui::vec2(8.0, 8.0), ring.size());
    rejects("a corner of the figure", || {
        assert!(gold_in(&locked, corner) > 40)
    });
}

#[test]
fn mock_lock_badge_the_compass_carries_the_pin() {
    let mut rig = new_rig(true);
    let compass = rig.layout.compass;
    let free = rig.shot();
    rig.lock();
    let locked = rig.shot();
    assert!(
        gold_in(&locked, compass) > 40,
        "the gold pin on the compass"
    );
    rejects("the compass before locking", || {
        assert!(gold_in(&free, compass) > 40)
    });
}

#[test]
fn mock_lock_badge_reads_locked_at_z_locked_with_its_value() {
    let mut rig = new_rig(false);
    let free = frame_texts(&mut rig.headless, &mut rig.app);
    rig.lock();
    let z_locked = rig.app.snapshot().sim.lock.z_locked;
    let texts = frame_texts(&mut rig.headless, &mut rig.app);
    assert!(
        texts.iter().any(|t| t == LOCKED),
        "the badge reads {LOCKED:?}: {texts:?}"
    );
    let value = anchor_text(&z_locked);
    assert!(
        texts.iter().any(|t| t.starts_with(&value)),
        "the anchor's value {value:?}: {texts:?}"
    );
    assert!(
        free.iter().any(|t| t == UNLOCKED),
        "free, the badge reads {UNLOCKED:?}"
    );
    rejects("the panel before locking", || {
        assert!(free.iter().any(|t| t == LOCKED))
    });
    let other = anchor_text(&[0.0; 8]);
    rejects("another value", || {
        assert!(texts.iter().any(|t| t.starts_with(&other)))
    });
}

#[test]
fn mock_lock_badge_unlock_emits_the_setfield_clearing_the_lock() {
    let mut rig = new_rig(false);
    rig.lock();
    let before = rig.app.snapshot().clone();
    click(&mut rig.headless, &mut rig.app, UNLOCK);
    let _ = rig.headless.frame(&mut rig.app, Vec::new());
    let after = rig.app.snapshot().clone();
    assert_eq!(
        after.sim.lock,
        Lock {
            locked: false,
            z_locked: [0.0; 8]
        },
        "the lock cleared"
    );
    assert_eq!(
        after.sim.plane, before.sim.plane,
        "(z₀, q₁, q₂) left as it stands"
    );
    assert_eq!(
        after.history.undo_depth,
        before.history.undo_depth + 1,
        "one undoable SetField"
    );
    // It is a SetField: undo restores the lock.
    rig.app.undo();
    let _ = rig.headless.frame(&mut rig.app, Vec::new());
    assert_eq!(
        rig.app.snapshot().sim.lock,
        before.sim.lock,
        "undo restores the lock"
    );
    rejects("the snapshot before unlocking", || {
        assert_eq!(
            before.sim.lock,
            Lock {
                locked: false,
                z_locked: [0.0; 8]
            }
        );
    });
    // Free, unlock emits nothing.
    let mut out = crate::explore::manifold_view::Out::default();
    crate::explore::manifold_view::lock::unlock(&after.sim.lock, &mut out);
    assert!(out.edits.is_empty(), "nothing to unlock");
    let mut out = crate::explore::manifold_view::Out::default();
    crate::explore::manifold_view::lock::unlock(&before.sim.lock, &mut out);
    let edit = out.edits.first().map(|e| e.edit.clone());
    assert_eq!(
        edit,
        Some(Edit::Sim(SimField::Lock(Lock {
            locked: false,
            z_locked: [0.0; 8]
        }))),
        "locked, unlock is the SetField on the lock"
    );
}

#[test]
fn mock_lock_badge_open_in_inspector_requests_the_inspector() {
    let mut rig = new_rig(false);
    rig.lock();
    assert!(rig.app.view.windows.open.is_empty(), "no window yet");
    click(&mut rig.headless, &mut rig.app, OPEN_INSPECTOR);
    assert_eq!(
        rig.app.view.windows.open,
        vec![Window::Inspector],
        "the Inspector requested"
    );
    // Again: it is open, so not requested twice.
    click(&mut rig.headless, &mut rig.app, OPEN_INSPECTOR);
    assert_eq!(
        rig.app.view.windows.open,
        vec![Window::Inspector],
        "requested once"
    );
    // Control: free, the button is disabled and requests nothing.
    let mut free = new_rig(false);
    click(&mut free.headless, &mut free.app, OPEN_INSPECTOR);
    let open = free.app.view.windows.open.clone();
    rejects("open in Inspector while free", || {
        assert_eq!(open, vec![Window::Inspector])
    });
}
