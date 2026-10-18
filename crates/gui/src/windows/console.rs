//! The console (render_gui_spec §G12; GUI_DESIGN_NOTES § "12 Console"; 12_console.png; R-405, R-409): the footer,
//! opened over the bottom row: severity, time, source and message columns, newest first; the filters (all, warnings,
//! errors, info, and a text filter); copy and clear. It is the same stream as the profiler's telemetry: the log entries
//! the snapshots carry (gui_state_contract §2). An error opens it by itself, taking no focus (R-405); clear empties it
//! and resets the footer's warning and error counts, which count from the session's start until then.
//!
//! Its keys (§G3, §G12; R-405, R-409 G4–G6, R-450): a window of exactly two scopes, top to bottom, the header bar and
//! the log. Opening it from the footer or Windows › Console lands in navigation mode on the header bar scope itself; ↑
//! ↓, Tab and Shift+Tab move between the two, inside the window (A1). Enter on the header bar goes in, onto All, and ←
//! → cross its controls in visual order, All, Warnings, Errors, Info, the search field, Copy, Clear and ×, stopping at
//! its ends; Esc goes back up to it (A2). The search field is §G3's text field: Enter starts typing, in which every key
//! but Esc, Enter and Tab goes to it; Esc or Enter stops and keeps the text. × is a button: Enter or a click closes the
//! console, the focus back on whatever opened it. Esc at the top level, on either scope, closes it too. Enter on the
//! log starts interaction: ↑ ↓ scroll it an entry, Page Up and Page Down a page (the entries its rect shows), Home and
//! End to its first and last entry, Shift ×10, over the whole filtered list, clamped (A4); an empty list leaves the log
//! where it is (A5). A click on a console control moves the keyboard's focus onto it (A3), and a click on the search
//! field starts typing in it (R-456).

use eframe::egui::{self, Align, Button, Grid, Layout, RichText, ScrollArea, TextEdit, Ui};
use engine::contract::log::{LogEntry, Severity};

use crate::app::Counts;
use crate::explore::footer::severity_colour;
use crate::keyboard::scopes::{Scope, ScopeId, ScopeTree, Scroll, ScrollBy};
use crate::windows::{note, Keys, CLOSE_LABEL, CLOSE_TEXT};

/// The window's scope.
pub const WINDOW: ScopeId = "win_console";
/// The console window's title.
pub const TITLE: &str = "Console";

/// The header bar, the first scope: the filters, the search field, Copy, Clear and × in a row (R-450).
pub const HEADER: ScopeId = "cs_header";
/// The log, the second scope: the entry list, scrolled by the keys in interaction mode (R-450 A4).
pub const LOG: ScopeId = "cs_log";
/// The search field: the text filter.
pub const SEARCH: ScopeId = "cs_search";
/// Copy: the rows shown, as text, to the clipboard.
pub const COPY: ScopeId = "cs_copy";
/// Clear: empties the console and resets the footer's counts.
pub const CLEAR: ScopeId = "cs_clear";
/// ×: closes the console, the focus back on its opener (R-450).
pub const CLOSE: ScopeId = "cs_close";

/// The filters' scopes, their buttons' text (12_console.png) and their breadcrumb names (R-450 A6), in order (§G12:
/// all, warnings, errors, info).
pub const FILTER_IDS: [(ScopeId, Filter, &str, &str); 4] = [
    ("cs_all", Filter::All, "all", "All"),
    ("cs_warnings", Filter::Warnings, "warnings", "Warnings"),
    ("cs_errors", Filter::Errors, "errors", "Errors"),
    ("cs_info", Filter::Info, "info", "Info"),
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
    /// The log's top row, among the rows shown, newest first.
    pub top_row: usize,
    /// The rows the log's rect showed when last drawn: a page (R-450 A4).
    pub page: usize,
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
    /// Close the console, the focus back on its opener (×, R-450).
    Close,
    /// A click on this control: the keyboard's focus moves onto it (R-450 A3).
    Focus(ScopeId),
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

/// Joins the window's two scopes and the header bar's controls to `tree`, in order (R-450), and the log's keys to the
/// `?` overlay.
pub fn register(tree: &mut ScopeTree) {
    tree.register(Some(WINDOW), Scope::group(HEADER, "Header").row());
    for (id, _, _, label) in FILTER_IDS {
        tree.register(Some(HEADER), Scope::control(id, label));
    }
    tree.register(Some(HEADER), Scope::text(SEARCH, "Search"));
    tree.register(Some(HEADER), Scope::control(COPY, "Copy"));
    tree.register(Some(HEADER), Scope::control(CLEAR, "Clear"));
    tree.register(Some(HEADER), Scope::control(CLOSE, CLOSE_LABEL));
    tree.register(Some(WINDOW), Scope::scroll(LOG, "Log"));
    tree.register_shortcut(
        "Page Up / Down · Home / End",
        "the console's log, interacting: a page · its first / last entry",
    );
}

/// Enter on control `id`, as its click, with the console's `entries`.
pub fn activate(id: &str, state: &mut State, entries: &[LogEntry]) -> Option<Act> {
    if let Some((_, filter, _, _)) = FILTER_IDS.iter().find(|(f, _, _, _)| *f == id) {
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

/// The log's top row once `by` has scrolled it from `top`, over `rows` rows of which `page` show: clamped so the
/// first row is the top one at least and the last row shows at most (R-450 A4).
pub fn scrolled_top(top: usize, by: ScrollBy, rows: usize, page: usize) -> usize {
    let page = page.max(1);
    let last_top = rows.saturating_sub(page) as i64;
    let to = match by {
        ScrollBy::Entries(n) => top as i64 + n,
        ScrollBy::Pages(n) => top as i64 + n * page as i64,
        ScrollBy::First => 0,
        ScrollBy::Last => last_top,
    };
    to.clamp(0, last_top) as usize
}

/// The keys' scroll of the log, in interaction mode, over the whole filtered list.
pub fn scroll(scroll: &Scroll, state: &mut State, entries: &[LogEntry]) {
    if scroll.scope != LOG {
        return;
    }
    let rows = shown(entries, state).len();
    state.top_row = scrolled_top(state.top_row, scroll.by, rows, state.page);
    state.scroll_pending = true;
}

/// The rows a log of `height` points shows whole, each `line` tall and `row_h` from the next: at least one.
pub fn page_of(height: f32, line: f32, row_h: f32) -> usize {
    if row_h <= 0.0 {
        return 1;
    }
    ((height - line) / row_h).floor().max(0.0) as usize + 1
}

/// The first row drawn whole in a log scrolled `offset` points down, its rows `row_h` apart: a row scrolled past by
/// less than a thousandth of a row counts as whole, so an offset of exact rows gives that row.
pub fn top_of(offset: f32, row_h: f32) -> usize {
    if row_h <= 0.0 {
        return 0;
    }
    (offset / row_h - 1e-3).ceil().max(0.0) as usize
}

/// What the console draws from.
pub struct View<'a> {
    /// The rows, oldest first.
    pub entries: &'a [LogEntry],
    /// The footer's counts.
    pub counts: Counts,
}

/// Draws one header button, as `id`: its click acts and moves the keyboard's focus onto it (A3).
fn header_button(
    ui: &mut Ui,
    keys: &mut Keys<'_>,
    id: ScopeId,
    button: Button<'_>,
    state: &mut State,
    entries: &[LogEntry],
    acts: &mut Vec<Act>,
) {
    let response = ui.add(button);
    if response.clicked() {
        acts.push(Act::Focus(id));
        // Enter on × closes through the window rule (`windows::closed_by`); its click, as here.
        let act = if id == CLOSE {
            Some(Act::Close)
        } else {
            activate(id, state, entries)
        };
        acts.extend(act);
    }
    keys.place(id, &response);
}

/// Draws the console's content, its header bar first (its title, then its controls in order, Copy, Clear and × at
/// the right, as 12_console.png's row has copy and clear); returns what its controls asked for, in order.
pub fn show(ui: &mut Ui, keys: &mut Keys<'_>, state: &mut State, view: &View<'_>) -> Vec<Act> {
    let mut acts = Vec::new();
    let header = ui
        .horizontal(|ui| {
            ui.label(RichText::new(TITLE).strong());
            for (id, filter, label, _) in FILTER_IDS {
                let text = match filter {
                    Filter::Warnings => format!("⚠ {label} {}", view.counts.warnings),
                    Filter::Errors => format!("× {label} {}", view.counts.errors),
                    _ => label.to_owned(),
                };
                let button = Button::selectable(state.filter == filter, text);
                header_button(ui, keys, id, button, state, view.entries, &mut acts);
            }
            let text = ui.add(
                TextEdit::singleline(&mut state.text)
                    .hint_text(TEXT_HINT)
                    .desired_width(240.0),
            );
            // A click on the field starts typing through `Keyboard::follow_text_clicks`, as on every text field
            // (R-456).
            keys.place(SEARCH, &text);
            if text.changed() {
                state.top_row = 0;
                state.scroll_pending = true;
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                for (id, label) in [(CLOSE, CLOSE_TEXT), (CLEAR, "clear"), (COPY, "copy")] {
                    let button = Button::new(label);
                    header_button(ui, keys, id, button, state, view.entries, &mut acts);
                }
            });
        })
        .response;
    keys.place_rect(ui, HEADER, header.rect);
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
    let pending = std::mem::take(&mut state.scroll_pending);
    if pending {
        area = area.vertical_scroll_offset(state.top_row as f32 * row_h);
    }
    let list = area.show(ui, |ui| {
        Grid::new("console rows")
            .striped(false)
            .min_row_height(line)
            .spacing(ui.spacing().item_spacing)
            .show(ui, |ui| {
                for entry in &rows {
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
    state.page = page_of(list.inner_rect.height(), line, row_h);
    if !pending {
        // The wheel's scroll is where the keys go on from: its first row drawn whole.
        state.top_row =
            top_of(list.state.offset.y, row_h).min(rows.len().saturating_sub(state.page));
    }
    if ui.rect_contains_pointer(list.inner_rect) && ui.input(|i| i.pointer.primary_clicked()) {
        acts.push(Act::Focus(LOG));
    }
    keys.place_rect(ui, LOG, list.inner_rect);
    note(ui, FOOT);
    acts
}
