//! Lock in the Manifold view (render_gui_spec §G2, §G4): the badge, "● locked at z_locked" with unlock and open in
//! Inspector (`explore::lock`), and the anchor's value. Locking (K, or right-click › lock here, on the figure) sets
//! the lock flag and the anchor `z_locked = z₀ + (2s−1) q₁ + (2t−1) q₂`, computed directly on the affine chart
//! (chart_decoder_contract Part 4, "The lock"), and recentres on it, a `SetField` on `z₀`. Both are undoable (R-69).
//! Unlocking clears the flag and leaves `(z₀, q₁, q₂)` as it stands, the new free start.

use eframe::egui::{Rect, RichText, Ui};
use engine::contract::sim_config::{Lock, Plane};
use engine::contract::view_ui::Window;

use super::{lock_edit, point, row_ui, z0_edit, Out, Panel, Scratch, OPEN_INSPECTOR, UNLOCK};
use crate::explore::lock::{anchor_text, badge};

/// The figure's right-click entry.
pub const LOCK_HERE: &str = "lock here";
/// The note beside the badge, as 01_main.png's.
pub const NOTE: &str = "sliders re-based to the anchor · moving one is an excursion";
/// What the anchor line reads while free.
pub const FREE: &str = "free · K, or right-click › lock here, on the figure";

/// Locks at the figure's point `(s, t)` of `plane`: recentres on it, then sets the lock there.
pub fn lock_at(plane: &Plane, (s, t): (f64, f64), scratch: &mut Scratch, out: &mut Out) {
    let z_locked = point(plane, s, t);
    out.edit(z0_edit(z_locked), None, scratch);
    out.edit(
        lock_edit(Lock {
            locked: true,
            z_locked,
        }),
        None,
        scratch,
    );
}

/// Unlock: the `SetField` that clears the lock, while it is set.
pub fn unlock(lock: &Lock, out: &mut Out) {
    if lock.locked {
        out.edits.push(lock_edit(Lock {
            locked: false,
            z_locked: [0.0; 8],
        }));
    }
}

/// Open in Inspector: requests the Inspector window, while locked (its frame is TASK-M6-28's).
pub fn open_inspector(lock: &Lock, out: &mut Out) {
    if lock.locked {
        out.windows.push(Window::Inspector);
    }
}

/// Draws the badge in row `badge_row` and the anchor's value in row `anchor_row`.
pub fn show(ui: &mut Ui, badge_row: Rect, anchor_row: Rect, clip: Rect, panel: &mut Panel<'_>) {
    let lock = panel.snapshot.sim.lock.clone();
    let mut line = row_ui(ui, badge_row);
    let (unlock_button, inspector) = badge(&mut line, &lock);
    panel.place(UNLOCK, &unlock_button, clip);
    panel.place(OPEN_INSPECTOR, &inspector, clip);
    if unlock_button.clicked() {
        unlock(&lock, panel.out);
    }
    if inspector.clicked() {
        open_inspector(&lock, panel.out);
    }
    let mut line = row_ui(ui, anchor_row);
    let text = if lock.locked {
        format!("{} · {NOTE}", anchor_text(&lock.z_locked))
    } else {
        FREE.to_owned()
    };
    line.add(eframe::egui::Label::new(RichText::new(text).monospace().weak().small()).truncate());
}
