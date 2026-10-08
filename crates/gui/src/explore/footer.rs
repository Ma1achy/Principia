//! The footer (render_gui_spec §G2; 01_main.png; RQ-255, RQ-249): the warning and error counts since the session
//! began, the latest message, and, in the right group, the "mock engine" tag on the mock (REQ-GUI-167), in egui's
//! weak text colour, before the memory readout (GPU, heap) and the "? keys" hint. No passive-logging indicator: logging
//! is off (R-129). A click on it opens the console (§G12).

use eframe::egui::{
    self, Align, Button, Color32, Layout as EguiLayout, Rect, RichText, Sense, Ui, UiBuilder,
};
use engine::contract::log::{LogEntry, Severity};
use engine::contract::snapshot::LiveMemory;

use crate::app::{Actions, Counts};
use crate::explore::top_bar::ABSENT;

/// The tag the footer carries on the mock (R-390; REQ-GUI-167).
pub const MOCK_TAG: &str = "mock engine";

/// The keyboard hint (render_gui_spec §G2).
pub const KEYS_HINT: &str = "? keys";

/// What the footer shows.
pub struct Footer<'a> {
    /// The counts since the session began.
    pub counts: Counts,
    /// The latest entry, if any.
    pub latest: Option<&'a LogEntry>,
    /// The snapshot's memory, if any.
    pub memory: Option<LiveMemory>,
    /// Whether the engine is the mock.
    pub is_mock: bool,
    /// Whether the console is open.
    pub console_open: bool,
}

/// `bytes` in megabytes, 10^6 bytes, rounded: "464 MB".
pub fn megabytes(bytes: u64) -> String {
    format!("{} MB", bytes.saturating_add(500_000) / 1_000_000)
}

/// The memory readout: "GPU 464 MB · heap 148 MB", "—" for each when absent (gui_state_contract §2).
pub fn memory_readout(memory: Option<LiveMemory>) -> String {
    let (gpu, heap) = memory.map_or((ABSENT.to_owned(), ABSENT.to_owned()), |m| {
        (megabytes(m.gpu_bytes), megabytes(m.heap_bytes))
    });
    format!("GPU {gpu} · heap {heap}")
}

/// The counts and the console's toggle: "⚠ 1 · × 0 · console ▴", "▾" while it is open.
pub fn counts_label(counts: Counts, console_open: bool) -> String {
    let arrow = if console_open { "▾" } else { "▴" };
    format!(
        "⚠ {} · × {} · console {arrow}",
        counts.warnings, counts.errors
    )
}

/// The colour an entry of `severity` is shown in.
pub fn severity_colour(visuals: &egui::Visuals, severity: Severity) -> Color32 {
    match severity {
        Severity::Error => visuals.error_fg_color,
        Severity::Warn => visuals.warn_fg_color,
        Severity::Info => visuals.weak_text_color(),
    }
}

/// Draws the footer in `rect`; a click on it toggles the console.
pub fn show(ui: &mut Ui, rect: Rect, footer: &Footer<'_>, actions: &mut Actions) {
    ui.painter().rect_filled(rect, 0.0, ui.visuals().panel_fill);
    let clicked = ui
        .interact(rect, ui.id().with("footer"), Sense::click())
        .clicked();
    let mut row = ui.new_child(
        UiBuilder::new()
            .max_rect(rect.shrink2(egui::vec2(8.0, 0.0)))
            .layout(EguiLayout::left_to_right(Align::Center)),
    );
    let label = RichText::new(counts_label(footer.counts, footer.console_open)).monospace();
    let toggle = row.add(Button::new(label).frame(false)).clicked();
    if let Some(entry) = footer.latest {
        let colour = severity_colour(row.visuals(), entry.severity);
        row.label(RichText::new(&entry.message).monospace().color(colour));
    }
    row.with_layout(EguiLayout::right_to_left(Align::Center), |ui| {
        ui.label(RichText::new(KEYS_HINT).monospace());
        ui.label(RichText::new(memory_readout(footer.memory)).monospace());
        if footer.is_mock {
            ui.label(RichText::new(MOCK_TAG).weak());
        }
    });
    actions.toggle_console |= clicked || toggle;
}
