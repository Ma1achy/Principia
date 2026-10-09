//! The `?` shortcuts overlay (render_gui_spec §G3, "? — shortcuts, over everything"): §G3's key table in a frame
//! centred on the window, above every other layer, taking the pointer as well as the keys; Esc or `?` closes it.
//! Help's "Keys (?)" opens it too (RQ-250).

use eframe::egui::{self, Align2, Area, Frame, Grid, Id, Order, RichText};

use crate::keyboard::keymap::SHORTCUTS;

/// The overlay's title.
pub const TITLE: &str = "Keys";
/// The line that says how to close it.
pub const CLOSE_HINT: &str = "Esc closes";

/// Draws the overlay over everything.
pub fn show(ctx: &egui::Context) {
    Area::new(Id::new("shortcuts overlay"))
        .order(Order::Tooltip)
        .anchor(Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            Frame::popup(ui.style()).show(ui, |ui| {
                ui.label(RichText::new(TITLE).heading());
                ui.add_space(6.0);
                Grid::new("shortcuts")
                    .num_columns(2)
                    .spacing([24.0, 6.0])
                    .show(ui, |ui| {
                        for (key, action) in SHORTCUTS {
                            ui.label(RichText::new(key).monospace().strong());
                            ui.label(action);
                            ui.end_row();
                        }
                    });
                ui.add_space(6.0);
                ui.label(RichText::new(CLOSE_HINT).weak());
            });
        });
}
