//! The library drawer (render_gui_spec Part II §1; GUI_DESIGN_NOTES § "02 Stain"; 02_stain.png): collapsible, left;
//! the catalogue's groups, each collapsible, its rows filtered by the filter, the stain loaded highlighted, and New,
//! Rename, Import and Export; under the list, a line on your shaders' folder ([`yours`]). Its rows are the engine
//! side's content; selecting one highlights it. Loading a whole graph from a row (§11) is the content's, where it has
//! one for the row; the four buttons are drawn disabled, with nothing behind them yet. A locked row carries the lock
//! badge, and over the list stand the preset loaded, locked or not, and what it measures.

use std::collections::BTreeSet;
use std::f32::consts::FRAC_1_SQRT_2;

use eframe::egui::{
    self, pos2, vec2, Align, Button, Color32, Label, Layout as EguiLayout, Painter, Rect, Response,
    RichText, Sense, Shape, Stroke, Ui, UiBuilder, Vec2, WidgetInfo, WidgetType,
};
use engine::contract::stain::preset::Loaded;

use super::yours;
use super::{Content, Row, BUTTONS, COLLAPSE, FILTER, GROUP_IDS, ROW_IDS};
use crate::keyboard::scopes::ScopeId;
use crate::windows::Keys;

/// The filter's hint, as 02_stain.png's.
pub const FILTER_HINT: &str = "filter modes…";
/// The lock badge's accessible name, beside a locked row and the locked preset loaded.
pub const LOCK: &str = "locked";
/// The badge's body, in points: a filled rect under its shackle.
pub const BODY: Vec2 = vec2(8.0, 6.0);
/// What the badge says, hovered.
pub const LOCK_HINT: &str = "locked: the first edit makes an editable copy";

/// The drawer's own state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    /// Whether the drawer is collapsed to its strip.
    pub collapsed: bool,
    /// The filter's text.
    pub filter: String,
    /// The groups closed, by their place.
    pub closed: BTreeSet<usize>,
    /// The row selected, by its place in the whole library; the stain loaded until one is.
    pub selected: Option<usize>,
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
    for (g, group) in c.library.iter().enumerate().take(GROUP_IDS.len()) {
        let rows: Vec<Item> = group
            .rows
            .iter()
            .filter(|row| passes(row.name, &state.filter))
            .map(|row| Item::Row(row.place, row.name))
            .filter(|item| matches!(item, Item::Row(k, _) if *k < ROW_IDS.len()))
            .collect();
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
    at(c, k).map(|row| row.name)
}

fn at(c: &Content, k: usize) -> Option<&Row> {
    let mut rows = c.library.iter().flat_map(|g| &g.rows);
    rows.find(|row| row.place == k)
}

/// The row highlighted as the stain loaded: the selected row, else the content's, unless that row is locked and the
/// preset loaded is not, so a fork of it is never shown as the preset itself.
pub fn highlighted(c: &Content, state: &State, preset: Option<&Loaded>) -> Option<usize> {
    if state.selected.is_some() {
        return state.selected;
    }
    let k = c.loaded?;
    let forked = at(c, k).is_some_and(|row| row.locked) && preset.is_some_and(|p| !p.locked);
    (!forked).then_some(k)
}

/// What the drawer says over its list: the preset loaded, with the badge if it is locked, and what the row it is
/// measures.
pub fn heading(
    c: &Content,
    state: &State,
    preset: Option<&Loaded>,
) -> (String, bool, Option<String>) {
    let k = highlighted(c, state, preset).filter(|_| state.selected.is_none());
    let row = k.and_then(|k| at(c, k));
    let (name, locked) = match (preset, row) {
        (Some(p), _) => (p.name.clone(), p.locked),
        (None, Some(row)) => (row.name.to_owned(), row.locked),
        (None, None) => (String::new(), false),
    };
    (name, locked, row.and_then(|row| row.measure.clone()))
}

/// The lock badge in `rect`, painted, and its accessible name.
// Painted, not a glyph: no font of the theme has U+1F512, which draws as a stray mark.
fn badge(ui: &Ui, rect: Rect, response: Response) -> Response {
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, LOCK));
    #[cfg(test)]
    if UNPAINTED.with(std::cell::Cell::get) {
        return response.on_hover_text(LOCK_HINT);
    }
    paint_lock(ui.painter(), rect.center(), ui.visuals().text_color());
    response.on_hover_text(LOCK_HINT)
}

/// The shackle's half circle, `(cos t, sin t)` for t = 0, π/8, …, π: a table, since production code makes no platform
/// maths call.
const ARC: [(f32, f32); 9] = [
    (1.0, 0.0),
    (0.923_88, 0.382_68),
    (FRAC_1_SQRT_2, FRAC_1_SQRT_2),
    (0.382_68, 0.923_88),
    (0.0, 1.0),
    (-0.382_68, 0.923_88),
    (-FRAC_1_SQRT_2, FRAC_1_SQRT_2),
    (-0.923_88, 0.382_68),
    (-1.0, 0.0),
];

/// A padlock centred at `c`: the [`BODY`] filled, a shackle arched over it.
fn paint_lock(painter: &Painter, c: egui::Pos2, colour: Color32) {
    let body = Rect::from_center_size(c + vec2(0.0, 2.0), BODY);
    painter.rect_filled(body, 1.0, colour);
    let (r, top) = (2.5, body.min.y - 2.0);
    let mut points = vec![pos2(c.x - r, body.min.y)];
    points.extend(ARC.iter().map(|(x, y)| pos2(c.x - r * x, top - r * y)));
    points.push(pos2(c.x + r, body.min.y));
    painter.add(Shape::line(points, Stroke::new(1.5, colour)));
}

#[cfg(test)]
thread_local! {
    /// The badge left unpainted: the fault the badge's test must catch.
    pub(crate) static UNPAINTED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
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
        .filter(|row| passes(row.name, &state.filter))
        .count();
    u32::try_from(passing).unwrap_or(u32::MAX)
}

/// The height under the list kept for the preset loaded and its measure: fixed, so loading a row never moves the rows.
pub const NOW_H: f32 = 64.0;

/// Draws, in `rect`, the preset loaded, the badge if it is locked, and what it measures.
fn loaded_box(ui: &mut Ui, rect: Rect, c: &Content, state: &State, preset: Option<&Loaded>) {
    let stroke = ui.visuals().widgets.noninteractive.bg_stroke;
    ui.painter()
        .line_segment([rect.left_top(), rect.right_top()], stroke);
    let mut ui = ui.new_child(UiBuilder::new().max_rect(rect.shrink2(vec2(0.0, 4.0))));
    ui.set_clip_rect(rect);
    let (name, locked, measure) = heading(c, state, preset);
    ui.horizontal(|ui| {
        ui.label(RichText::new(name).strong());
        if locked {
            let (rect, response) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::hover());
            badge(ui, rect, response);
        }
    });
    if let Some(measure) = measure {
        let text = Label::new(RichText::new(&measure).small().weak()).wrap();
        ui.add(text).on_hover_text(measure);
    }
}

/// The open drawer's list and its foot, the four buttons', in `rect`: inset 12 points across and 10 down, the foot
/// the last 30.
pub fn areas(rect: Rect) -> (Rect, Rect) {
    let inner = rect.shrink2(vec2(12.0, 10.0));
    let top = inner.max.y - 30.0;
    (
        Rect::from_min_max(inner.min, pos2(inner.max.x, top)),
        Rect::from_min_max(pos2(inner.min.x, top), inner.max),
    )
}

/// Draws the drawer in `rect`: collapsed, its strip and expand; open, its heading, filter, groups and rows, and the
/// four buttons at its foot.
pub fn show(
    ui: &mut Ui,
    rect: Rect,
    content: Option<&Content>,
    state: &mut State,
    preset: Option<&Loaded>,
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
    let (list, foot) = areas(rect);
    let (list, line) = list.split_top_bottom_at_y(list.max.y - yours::SHELF_H);
    let (list, now) = list.split_top_bottom_at_y(list.max.y - NOW_H);
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
        let loaded = highlighted(c, state, preset);
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
                                let row = ui
                                    .add(Button::selectable(
                                        loaded == Some(k),
                                        format!("   {name}"),
                                    ))
                                    .on_hover_ui(|ui| {
                                        ui.label(at(c, k).map_or("", |row| &row.from));
                                    });
                                if row.clicked() {
                                    clicked.push(ROW_IDS[k]);
                                }
                                keys.place(ROW_IDS[k], &row);
                                // Apart from the row's text, which is its name as the keyboard and the search read it.
                                if at(c, k).is_some_and(|row| row.locked) {
                                    let r = row.rect;
                                    let badge_rect =
                                        Rect::from_min_max(pos2(r.max.x - 24.0, r.min.y), r.max);
                                    let id = ui.id().with(("lock_badge", k));
                                    let response = ui.interact(badge_rect, id, Sense::hover());
                                    badge(ui, badge_rect, response);
                                }
                            }
                        }
                    }
                });
            });
        loaded_box(ui, now, c, state, preset);
        let mut strip = ui.new_child(UiBuilder::new().max_rect(line));
        yours::line(&mut strip, &c.yours, keys, clicked);
    }
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
