//! The `?` shortcuts overlay (render_gui_spec §G3, "? — shortcuts, over everything"): the global key table and the
//! rows the screens registered, in a frame centred on the window, above every other layer. While it is open it takes
//! every key and every click: Esc or `?` closes it, and so does a click outside its frame, which reaches nothing
//! beneath, as egui's menus do. Help's "Keys (?)" opens it too (RQ-250).

use eframe::egui::{self, Align2, Area, Frame, Grid, Id, Order, Rect, RichText};

use crate::keyboard::keymap::SHORTCUTS;
use crate::keyboard::scopes::Shortcut;

/// The overlay's title.
pub const TITLE: &str = "Keys";
/// The line that says how to close it.
pub const CLOSE_HINT: &str = "Esc, ? or a click outside closes";

/// Draws the overlay over everything: the global rows, then `screens'` rows. Returns its frame's rect.
pub fn show(ctx: &egui::Context, screens: &[Shortcut]) -> Rect {
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
                        for (key, action) in SHORTCUTS.iter().chain(screens) {
                            ui.label(RichText::new(*key).monospace().strong());
                            ui.label(*action);
                            ui.end_row();
                        }
                    });
                ui.add_space(6.0);
                ui.label(RichText::new(CLOSE_HINT).weak());
            });
        })
        .response
        .rect
}
