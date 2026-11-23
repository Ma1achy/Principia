//! Lock in the Manifold view (render_gui_spec §G2, §G4): the badge, "● locked at z_locked" with unlock, centre on pin
//! and open in Inspector (`explore::lock`), and the anchor's value beside the setting **Centre on pin when locking**.
//! Locking (K, or right-click › lock here, on the figure) does not move the view (R-866): the engine's lock leaf sets
//! the lock flag and the anchor `z_locked`, the pixel clicked, and pins that pixel where it is on screen; `z₀`, the basis
//! and the window are not edited. Centre on pin brings the pin to the centre, a `SetField` on `z₀`; with the setting on,
//! a lock and its centre on pin are one command, one history entry. Lock, unlock and centre on pin are undoable (R-69);
//! the setting is a preference, not undoable. Unlocking clears the flag and leaves `(z₀, q₁, q₂)` as it stands, the new
//! free start. Every route here is a thin adapter onto the engine's leaf (`crate::dispatch`), holding no formula.

use eframe::egui::{Checkbox, Rect, RichText, Ui};
use engine::contract::set_field::{Edit, SimField};
use engine::contract::sim_config::{Lock, Plane, SimConfig};
use engine::contract::view_ui::Window;
use engine::path::leaf::CentreOnPin;

use super::{
    lock_edit, row_ui, Out, Panel, Scratch, CENTRE_ON_PIN, CENTRE_ON_PIN_SETTING, OPEN_INSPECTOR,
    UNLOCK,
};
use crate::dispatch::{self, Class, Pinned};
use crate::explore::lock::{anchor_text, badge};

/// The figure's right-click entry.
pub const LOCK_HERE: &str = "lock here";
/// The note beside the badge, as 01_main.png's.
pub const NOTE: &str = "sliders re-based to the anchor · moving one is an excursion";
/// What the anchor line reads while free.
pub const FREE: &str = "free · K, or right-click › lock here, on the figure";
/// The badge's centre on pin, as the keyboard names it.
pub const CENTRE_ON_PIN_TEXT: &str = crate::explore::lock::CENTRE_ON_PIN;
/// The setting's name (render_gui_spec §G4): the keyboard's and the checkbox's hover text.
pub const SETTING_TEXT: &str = "Centre on pin when locking";
/// The setting's checkbox's label, beside centre on pin: "centre on pin ☐ when locking".
pub const SETTING_SHORT: &str = "when locking";

/// Locks at the figure's point `(s, t)` of `plane`, with `scratch`'s centre-on-pin setting: the engine's lock leaf
/// ([`Class::Lock`]), its edits one command, one history entry. The leaf's pin is kept for centre on pin.
pub fn lock_at(plane: &Plane, at: (f64, f64), scratch: &mut Scratch, out: &mut Out) {
    let setting = scratch.centre_on_pin;
    let Some(leaf) = dispatch::leaf(plane, Class::Lock { at, setting }) else {
        return;
    };
    if leaf.edits().is_empty() {
        return;
    }
    scratch.pinned = Pinned::of(&leaf);
    out.commands.push(leaf.edits().to_vec());
}

/// Centre on pin, while locked: the engine's leaf ([`Class::CentreOnPin`]) on `sim`'s plane, one command.
pub fn centre_on_pin(sim: &SimConfig, scratch: &Scratch, out: &mut Out) {
    if !sim.lock.locked {
        return;
    }
    let class = Class::CentreOnPin {
        z_locked: sim.lock.z_locked,
        pin: Pinned::on(scratch.pinned.as_ref(), sim),
    };
    let edits = dispatch::send(&sim.plane, class);
    // The pin at the centre already: `z₀ ← z₀` moves nothing, so it takes no history entry.
    let moves = edits.iter().any(|e| match &e.edit {
        Edit::Sim(SimField::Z0(z0)) => *z0 != sim.plane.z0,
        _ => true,
    });
    if moves {
        out.commands.push(edits);
    }
}

/// Turns the setting Centre on pin when locking on or off, and asks for the GUI's settings to be saved.
pub fn toggle_setting(scratch: &mut Scratch, out: &mut Out) {
    scratch.centre_on_pin = match scratch.centre_on_pin {
        CentreOnPin::Off => CentreOnPin::On,
        CentreOnPin::On => CentreOnPin::Off,
    };
    out.save_settings = true;
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

/// Draws the badge in row `badge_row`, and centre on pin, the setting and the anchor's value in row `anchor_row`.
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
    let centre = crate::explore::lock::centre_on_pin(&mut line, &lock);
    panel.place(CENTRE_ON_PIN, &centre, clip);
    if centre.clicked() {
        centre_on_pin(&panel.snapshot.sim, panel.scratch, panel.out);
    }
    let mut on = panel.scratch.centre_on_pin == CentreOnPin::On;
    let setting = line
        .add(Checkbox::new(&mut on, RichText::new(SETTING_SHORT).small()))
        .on_hover_text(SETTING_TEXT);
    panel.place(CENTRE_ON_PIN_SETTING, &setting, clip);
    if setting.changed() {
        toggle_setting(panel.scratch, panel.out);
    }
    let text = if lock.locked {
        format!("{} · {NOTE}", anchor_text(&lock.z_locked))
    } else {
        FREE.to_owned()
    };
    line.add(eframe::egui::Label::new(RichText::new(text).monospace().weak().small()).truncate());
}
