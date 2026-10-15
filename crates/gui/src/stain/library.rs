//! The library drawer (render_gui_spec Part II §1; GUI_DESIGN_NOTES § "02 Stain"; 02_stain.png): collapsible, left;
//! the catalogue's groups, each collapsible, its rows filtered by the filter, the stain loaded highlighted, and New,
//! Rename, Import and Export. Its rows are the engine side's content; selecting one highlights it. Loading a whole
//! graph from a row (§11) and the four buttons are TASK-M8-19's, so the buttons are drawn disabled.

use std::collections::BTreeSet;

use eframe::egui::{
    self, pos2, vec2, Align, Button, Layout as EguiLayout, Rect, RichText, Ui, UiBuilder,
};

use super::{Content, BUTTONS, COLLAPSE, FILTER, GROUP_IDS, ROW_IDS};
use crate::keyboard::scopes::ScopeId;
use crate::windows::Keys;

/// The filter's hint, as 02_stain.png's.
pub const FILTER_HINT: &str = "filter modes…";

/// The drawer's own state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    /// Whether the drawer is collapsed to its strip.
    pub collapsed: bool,
    /// The filter's text.
    pub filter: String,
    /// The groups closed, by their place.
    pub closed: BTreeSet<usize>,
    /// The row selected; the stain loaded until one is.
    pub selected: Option<&'static str>,
}

/// One line of the drawer's list: a group's heading, by its place, or a row, by its place in the whole library.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    /// A heading.
    Group(usize, &'static str),
    /// A row.
    Row(usize, &'static str),
}

/// Whether `row` passes `filter`: it holds the filter's text, case aside; an empty filter passes every row.
pub fn passes(row: &str, filter: &str) -> bool {
    row.to_lowercase().contains(&filter.trim().to_lowercase())
}

/// The drawer's lines as they stand: each group with a row the filter passes, then, unless it is closed, those rows.
pub fn visible(c: &Content, state: &State) -> Vec<Item> {
    let mut items = Vec::new();
    let mut k = 0;
    for (g, group) in c.library.iter().enumerate().take(GROUP_IDS.len()) {
        let rows: Vec<Item> = group
            .rows
            .iter()
            .enumerate()
            .filter(|(_, row)| passes(row, &state.filter))
            .map(|(i, row)| Item::Row(k + i, row))
            .filter(|item| matches!(item, Item::Row(k, _) if *k < ROW_IDS.len()))
            .collect();
        k += group.rows.len();
        if rows.is_empty() {
            continue;
        }
        items.push(Item::Group(g, group.title));
        if !state.closed.contains(&g) {
            items.extend(rows);
        }
    }
    items
}

/// Row `k` of the whole library.
pub fn row(c: &Content, k: usize) -> Option<&'static str> {
    c.library.iter().flat_map(|g| g.rows.iter().copied()).nth(k)
}

/// The count beside the filter: the catalogue's modes, or, filtering, the rows that pass.
pub fn count(c: &Content, state: &State) -> u32 {
    if state.filter.trim().is_empty() {
        return c.modes;
    }
    let passing = c
        .library
        .iter()
        .flat_map(|g| g.rows.iter())
        .filter(|row| passes(row, &state.filter))
        .count();
    u32::try_from(passing).unwrap_or(u32::MAX)
}

/// Draws the drawer in `rect`: collapsed, its strip and expand; open, its heading, filter, groups and rows, and the
/// four buttons at its foot.
pub fn show(
    ui: &mut Ui,
    rect: Rect,
    content: Option<&Content>,
    state: &mut State,
    keys: &mut Keys<'_>,
    clicked: &mut Vec<ScopeId>,
) {
    let visuals = ui.visuals().clone();
    ui.painter().rect_filled(rect, 0.0, visuals.panel_fill);
    ui.painter().line_segment(
        [rect.right_top(), rect.right_bottom()],
        visuals.widgets.noninteractive.bg_stroke,
    );
    if state.collapsed {
        let mut strip = ui.new_child(UiBuilder::new().max_rect(rect.shrink2(vec2(2.0, 8.0))));
        let expand = strip.add(Button::new("▸").frame(false));
        if expand.clicked() {
            clicked.push(COLLAPSE);
        }
        keys.place(COLLAPSE, &expand);
        return;
    }
    let inner = rect.shrink2(vec2(12.0, 10.0));
    let foot_h = 30.0;
    let list = Rect::from_min_max(inner.min, pos2(inner.max.x, inner.max.y - foot_h));
    let mut panel = ui.new_child(UiBuilder::new().max_rect(list));
    panel.horizontal(|ui| {
        ui.label(RichText::new("Library").heading());
        ui.with_layout(EguiLayout::right_to_left(Align::Center), |ui| {
            let collapse = ui.add(Button::new("◂").frame(false));
            if collapse.clicked() {
                clicked.push(COLLAPSE);
            }
            keys.place(COLLAPSE, &collapse);
        });
    });
    panel.add_space(4.0);
    panel.horizontal(|ui| {
        let n = content.map_or(0, |c| count(c, state));
        let width = ui.available_width() - 40.0;
        let filter = ui.add(
            egui::TextEdit::singleline(&mut state.filter)
                .hint_text(FILTER_HINT)
                .desired_width(width),
        );
        keys.place(FILTER, &filter);
        ui.label(RichText::new(n.to_string()).monospace().weak());
    });
    panel.add_space(6.0);
    if let Some(c) = content {
        let loaded = state.selected.or(c.loaded);
        egui::ScrollArea::vertical()
            .id_salt("stain_library_list")
            .auto_shrink(false)
            .show(&mut panel, |ui| {
                // Justified, so each row spans the drawer and reads from its left edge, as 02_stain.png's.
                ui.with_layout(EguiLayout::top_down_justified(Align::LEFT), |ui| {
                    for item in visible(c, state) {
                        match item {
                            Item::Group(g, title) => {
                                ui.add_space(6.0);
                                let arrow = if state.closed.contains(&g) {
                                    "▸"
                                } else {
                                    "▾"
                                };
                                let heading = ui.add(
                                    Button::new(RichText::new(format!("{arrow} {title}")).strong())
                                        .frame(false),
                                );
                                if heading.clicked() {
                                    clicked.push(GROUP_IDS[g]);
                                }
                                keys.place(GROUP_IDS[g], &heading);
                            }
                            Item::Row(k, name) => {
                                let row = ui.add(Button::selectable(
                                    loaded == Some(name),
                                    format!("   {name}"),
                                ));
                                if row.clicked() {
                                    clicked.push(ROW_IDS[k]);
                                }
                                keys.place(ROW_IDS[k], &row);
                            }
                        }
                    }
                });
            });
    }
    let foot = Rect::from_min_max(pos2(inner.min.x, inner.max.y - foot_h), inner.max);
    let mut row = ui.new_child(
        UiBuilder::new()
            .max_rect(foot)
            .layout(EguiLayout::left_to_right(Align::Center)),
    );
    for (id, label) in BUTTONS {
        let button = row
            .add_enabled(false, Button::new(label))
            .on_disabled_hover_text("TASK-M8-19");
        keys.place(id, &button);
    }
}
