//! The console (render_gui_spec §G12; GUI_DESIGN_NOTES § "12 Console"; 12_console.png; R-405, R-409): the footer,
//! opened over the bottom row: severity, time, source and message columns, newest first; the filters (all, warnings,
//! errors, info, and a text filter); copy and clear. It is the same stream as the profiler's telemetry: the log entries
//! the snapshots carry (gui_state_contract §2). An error opens it by itself, taking no focus (R-405); clear empties it
//! and resets the footer's warning and error counts, which count from the session's start until then.
//!
//! Its keys (§G3; R-405, R-409 F2, G4–G6): a window under the window rule, its sections in Tab order the filters, the
//! text filter, copy and clear, and the entry list. The arrows move between the filters, and between copy and clear,
//! in navigation mode; Enter on one acts as its click. The text filter lands in navigation mode, Enter starting typing,
//! in which every key but Esc, Enter and Tab goes to the field (Ctrl+Z undoing the text edit, `?` typing). The entry
//! list lands in navigation mode, where ↑ and ↓ do not scroll it; Enter starts interaction, in which they scroll it a
//! row a step, ten with Shift (applied per R-369: its own step of one row, under `Bounded`, no step kind added); Esc
//! returns to navigation mode on it.

use eframe::egui::{self, Button, Grid, RichText, ScrollArea, TextEdit, Ui};
use engine::contract::log::{LogEntry, Severity};

use crate::app::Counts;
use crate::explore::footer::severity_colour;
use crate::keyboard::scopes::{Adjust, Scope, ScopeId, ScopeTree, StepKind};
use crate::windows::{note, Keys};

/// The window's scope.
pub const WINDOW: ScopeId = "win_console";
/// The console window's title.
pub const TITLE: &str = "Console";

/// The filters' section.
pub const FILTERS: ScopeId = "cs_filters";
/// The text filter, a section of one text field.
pub const TEXT: ScopeId = "cs_text";
/// Copy and clear's section.
pub const COPY_CLEAR: ScopeId = "cs_copy_clear";
/// The entry list, a section of one element the arrows scroll in interaction mode.
pub const LIST: ScopeId = "cs_list";
/// Copy: the rows shown, as text, to the clipboard.
pub const COPY: ScopeId = "cs_copy";
/// Clear: empties the console and resets the footer's counts.
pub const CLEAR: ScopeId = "cs_clear";

/// The filters' scopes and labels, in order (§G12: all, warnings, errors, info).
pub const FILTER_IDS: [(ScopeId, Filter, &str); 4] = [
    ("cs_all", Filter::All, "all"),
    ("cs_warnings", Filter::Warnings, "warnings"),
    ("cs_errors", Filter::Errors, "errors"),
    ("cs_info", Filter::Info, "info"),
];

/// The text filter's hint (12_console.png).
pub const TEXT_HINT: &str = "filter…";
/// The line under the list (12_console.png).
pub const FOOT: &str =
    "the same stream the profiler's telemetry reads · errors also open it automatically";

/// A severity filter.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Filter {
    /// Every entry.
    #[default]
    All,
    /// `warn` entries.
    Warnings,
    /// `error` entries.
    Errors,
    /// `info` entries.
    Info,
}

impl Filter {
    /// Whether an entry of `severity` passes.
    pub fn passes(self, severity: Severity) -> bool {
        match self {
            Filter::All => true,
            Filter::Warnings => severity == Severity::Warn,
            Filter::Errors => severity == Severity::Error,
            Filter::Info => severity == Severity::Info,
        }
    }
}

/// The console's own state: the GUI's.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// The severity filter.
    pub filter: Filter,
    /// The text filter.
    pub text: String,
    /// The entry list's top row, among the rows shown, newest first.
    pub top_row: usize,
    /// Whether the list scrolls to [`State::top_row`] next frame.
    pub scroll_pending: bool,
    /// The text copy last put on the clipboard.
    pub copied: Option<String>,
}

/// What a console control asks of the app.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Act {
    /// Copy this text to the clipboard.
    Copy(String),
    /// Empty the console and reset the footer's counts.
    Clear,
}

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

/// Whether `entry` shows under `state`'s filters: its severity passes, and the text filter, if any, is in its message
/// or its source's name, letter case ignored.
pub fn shows(entry: &LogEntry, state: &State) -> bool {
    let text = state.text.trim().to_lowercase();
    state.filter.passes(entry.severity)
        && (text.is_empty()
            || entry.message.to_lowercase().contains(&text)
            || entry.source.name().contains(&text))
}

/// The rows shown, newest first.
pub fn shown<'a>(entries: &'a [LogEntry], state: &State) -> Vec<&'a LogEntry> {
    entries.iter().rev().filter(|e| shows(e, state)).collect()
}

/// One row as copy writes it: severity, time, source and message.
pub fn row_text(entry: &LogEntry) -> String {
    format!(
        "{} {} {} {}",
        entry.severity.name(),
        time_of_day(entry.at),
        entry.source.name(),
        entry.message
    )
}

/// Joins the window's sections and elements to `tree`, in Tab order.
pub fn register(tree: &mut ScopeTree) {
    tree.register(Some(WINDOW), Scope::group(FILTERS, "filters"));
    for (id, _, label) in FILTER_IDS {
        tree.register(Some(FILTERS), Scope::control(id, label));
    }
    tree.register(Some(WINDOW), Scope::text(TEXT, "text filter"));
    tree.register(Some(WINDOW), Scope::group(COPY_CLEAR, "copy and clear"));
    tree.register(Some(COPY_CLEAR), Scope::control(COPY, "copy"));
    tree.register(Some(COPY_CLEAR), Scope::control(CLEAR, "clear"));
    tree.register(
        Some(WINDOW),
        Scope::value(LIST, "entries", StepKind::Bounded),
    );
}

/// Enter on control `id`, as its click, with the console's `entries`.
pub fn activate(id: &str, state: &mut State, entries: &[LogEntry]) -> Option<Act> {
    if let Some((_, filter, _)) = FILTER_IDS.iter().find(|(f, _, _)| *f == id) {
        state.filter = *filter;
        state.top_row = 0;
        state.scroll_pending = true;
        return None;
    }
    match id {
        COPY => {
            let text: Vec<String> = shown(entries, state).into_iter().map(row_text).collect();
            let text = text.join("\n");
            state.copied = Some(text.clone());
            Some(Act::Copy(text))
        }
        CLEAR => {
            state.top_row = 0;
            state.scroll_pending = true;
            Some(Act::Clear)
        }
        _ => None,
    }
}

/// The rows one arrow press scrolls: its sign, a row a step, ten with Shift (applied per R-369), at least one.
pub fn rows_of(times: f64) -> i64 {
    let rows = times.abs().round().max(1.0) as i64;
    if times < 0.0 {
        -rows
    } else {
        rows
    }
}

/// The arrows on the entry list, in interaction mode: ↓ scrolls down the list, to older rows, ↑ back up, kept on it.
pub fn adjust(adjust: &Adjust, state: &mut State, entries: &[LogEntry]) {
    if adjust.scope != LIST {
        return;
    }
    let last = shown(entries, state).len().saturating_sub(1) as i64;
    // ↑'s sign is +, and ↑ moves towards the top, the newest row.
    let to = (state.top_row as i64 - rows_of(adjust.times)).clamp(0, last);
    state.top_row = to as usize;
    state.scroll_pending = true;
}

/// What the console draws from.
pub struct View<'a> {
    /// The rows, oldest first.
    pub entries: &'a [LogEntry],
    /// The footer's counts.
    pub counts: Counts,
}

/// Draws the console's content; returns what its controls asked for.
pub fn show(ui: &mut Ui, keys: &mut Keys<'_>, state: &mut State, view: &View<'_>) -> Option<Act> {
    let mut act = None;
    ui.horizontal(|ui| {
        let filters = ui
            .scope(|ui| {
                for (id, filter, label) in FILTER_IDS {
                    let text = match filter {
                        Filter::Warnings => format!("⚠ {label} {}", view.counts.warnings),
                        Filter::Errors => format!("× {label} {}", view.counts.errors),
                        _ => label.to_owned(),
                    };
                    let button = ui.add(Button::selectable(state.filter == filter, text));
                    if button.clicked() {
                        act = activate(id, state, view.entries).or(act.take());
                    }
                    keys.place(id, &button);
                }
            })
            .response;
        keys.place_rect(ui, FILTERS, filters.rect);
        let text = ui.add(
            TextEdit::singleline(&mut state.text)
                .hint_text(TEXT_HINT)
                .desired_width(240.0),
        );
        keys.place(TEXT, &text);
        if text.changed() {
            state.top_row = 0;
        }
        let pair = ui
            .scope(|ui| {
                for (id, label) in [(COPY, "copy"), (CLEAR, "clear")] {
                    let button = ui.button(label);
                    if button.clicked() {
                        act = activate(id, state, view.entries).or(act.take());
                    }
                    keys.place(id, &button);
                }
            })
            .response;
        keys.place_rect(ui, COPY_CLEAR, pair.rect);
    });
    ui.separator();
    let rows = shown(view.entries, state);
    // A row is the grid's line, at least a control's height, as egui's grid makes it, and its spacing below.
    let line = ui
        .text_style_height(&egui::TextStyle::Monospace)
        .max(ui.spacing().interact_size.y);
    let row_h = line + ui.spacing().item_spacing.y;
    let foot_h = ui.text_style_height(&egui::TextStyle::Body) + 2.0 * ui.spacing().item_spacing.y;
    let mut area = ScrollArea::vertical()
        .id_salt("console rows")
        .auto_shrink(false)
        .max_height((ui.available_height() - foot_h).max(row_h));
    if std::mem::take(&mut state.scroll_pending) {
        area = area.vertical_scroll_offset(state.top_row as f32 * row_h);
    }
    let list = area.show(ui, |ui| {
        Grid::new("console rows")
            .striped(false)
            .min_row_height(line)
            .spacing(ui.spacing().item_spacing)
            .show(ui, |ui| {
                for entry in rows {
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
    keys.place_rect(ui, LIST, list.inner_rect);
    note(ui, FOOT);
    act
}
