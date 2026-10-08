//! The console window's frame (render_gui_spec §G12; RQ-249): the footer, opened, over the bottom row as
//! 12_console.png places it, listing the log entries newest first as plain rows of severity, time, source and message.
//! Its layout, filters, copy and clear, and opening on an error, are TASK-M6-28's.

use eframe::egui::{self, Grid, Rect, RichText, ScrollArea};
use engine::contract::log::LogEntry;

use crate::explore::footer::severity_colour;

/// The console window's title.
pub const TITLE: &str = "Console";

/// `at`, milliseconds since the Unix epoch, as the time of day in UTC: "12:04:11.251" (12_console.png).
pub fn time_of_day(at: u64) -> String {
    let ms = at % 86_400_000;
    format!(
        "{:02}:{:02}:{:02}.{:03}",
        ms / 3_600_000,
        ms / 60_000 % 60,
        ms / 1000 % 60,
        ms % 1000
    )
}

/// Shows the console window in `rect` with `entries`; `open` turns false when it is closed.
pub fn show(ctx: &egui::Context, rect: Rect, entries: &[LogEntry], open: &mut bool) {
    egui::Window::new(TITLE)
        .fixed_rect(rect)
        .collapsible(false)
        .resizable(false)
        .open(open)
        .show(ctx, |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                Grid::new("console rows").striped(false).show(ui, |ui| {
                    for entry in entries.iter().rev() {
                        let colour = severity_colour(ui.visuals(), entry.severity);
                        ui.label(
                            RichText::new(entry.severity.name())
                                .monospace()
                                .color(colour),
                        );
                        ui.label(RichText::new(time_of_day(entry.at)).monospace().weak());
                        ui.label(RichText::new(entry.source.name()).monospace());
                        ui.label(RichText::new(&entry.message).monospace().color(colour));
                        ui.end_row();
                    }
                });
            });
        });
}
