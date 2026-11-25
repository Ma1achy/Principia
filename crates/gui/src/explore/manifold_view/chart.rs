//! Chart (render_gui_spec §G2): the preset, named by its axes; the two basis vectors `q₁`, `q₂`, each with an edit
//! button, which opens its eight components for drag or typing; Chart builder…, which opens its window (TASK-M6-28);
//! and the chart's kind. A preset keeps the zoom; locked, it turns about the pin, as every basis edit does
//! ([`super::about_the_pin`], §G4). A preset, or a basis edit, whose plane takes the slice normal moves it off the
//! plane by the slice-axis rule in the same edit, a target following ([`super::plane_edit`]; R-459 G4). The shape
//! sphere's projection selector and hemisphere toggle are REQ-GUI-161's, TASK-M8-06's: the mock's charts are all
//! affine.
//!
//! After Chart builder…, Save view as chart ([`super::saved`]): the preset selector's Saved group lists the saved charts
//! under the presets, each with its provenance; while one is active the header shows its provenance, and the selector's
//! row renames and deletes it.

use eframe::egui::{
    self, Area, Button, ComboBox, DragValue, Frame, Grid, Id, Label, Order, Rect, RichText,
    TextEdit, Ui,
};
use engine::contract::set_field::SetField;
use engine::contract::sim_config::{Latent, SimConfig};

use super::saved::{self, Switch};
use super::{
    about_the_pin, agreeing, axis, close_under_console, plane_edit, row, row_ui, Out, Panel,
    Scratch, View, AXES, BUILDER, PRESET, PRESETS, Q1_COMPONENTS, Q1_EDIT, Q2_COMPONENTS, Q2_EDIT,
};

/// The chart's kind: every chart the mock serves is the affine slice (chart_decoder_contract Part 3).
pub const KIND: &str = "affine chart";
/// The width of a row's label, in points (a look choice, R-390).
pub const LABEL_W: f32 = 92.0;
/// The width of a basis editor's field, in points (a look choice, R-390): four to a row fit the panel.
pub const EDITOR_FIELD_W: f32 = 44.0;
/// The width of the selector's rename and delete buttons, in points (a look choice).
pub const BUTTONS_W: f32 = 96.0;
/// The height of a component's field in the basis editor, in points (a look choice, R-390).
pub const EDITOR_FIELD_H: f32 = 18.0;

/// The basis of preset `preset` at `view`'s scale: `q₁ = σ e_a`, `q₂ = σ e_b`.
pub fn preset_basis(view: View, preset: usize) -> (Latent, Latent) {
    let (a, b) = PRESETS[preset];
    (axis(a, view.scale), axis(b, view.scale))
}

/// The edit that switches `sim`'s chart, read as `view`, to preset `preset`, keeping its scale: the basis
/// [`preset_basis`], with the slice frame following the new plane in the same edit where the plane takes the normal or
/// a target ([`plane_edit`]; R-459 G4).
pub fn preset_edit(sim: &SimConfig, view: View, preset: usize) -> SetField {
    plane_edit(sim, preset_basis(view, preset))
}

/// The pick of preset `preset`: its edit, or, off a saved chart, the change of the active chart.
pub fn pick_preset(sim: &SimConfig, preset: usize, scratch: &mut Scratch, out: &mut Out) {
    let edit = preset_edit(sim, View::read(sim), preset);
    if scratch.charts.is_marked() {
        out.chart = Some(Switch::Leave(agreeing(about_the_pin(vec![edit], sim), sim)));
    } else {
        out.edit(edit, None, scratch);
    }
}

/// Enter on the preset selector: the chart after the active one, the presets then the saved charts.
// The plan is unclear here: how the keyboard picks a saved chart; chose the selector's own Enter, on past the presets,
// because the picker's entries are no scopes.
pub fn pick_next(sim: &SimConfig, scratch: &mut Scratch, out: &mut Out) {
    let saved: Vec<u64> = scratch.charts.entries().iter().map(|(id, _)| *id).collect();
    let active = scratch.charts.active(sim).map(|(id, _)| id);
    let now = match active {
        Some(id) => saved
            .iter()
            .position(|s| *s == id)
            .map(|i| PRESETS.len() + i),
        None => View::read(sim).preset,
    };
    let next = now.map_or(0, |i| (i + 1) % (PRESETS.len() + saved.len()));
    match next.checked_sub(PRESETS.len()) {
        Some(i) => out.chart = Some(Switch::Open(saved[i])),
        None => pick_preset(sim, next, scratch, out),
    }
}

/// Starts the rename of the active saved chart.
pub fn begin_rename(sim: &SimConfig, scratch: &mut Scratch) {
    let name = scratch
        .charts
        .active(sim)
        .and_then(|(id, _)| scratch.charts.get(id))
        .map(|c| c.name.clone());
    scratch.charts.naming = name;
}

/// Deletes the active saved chart from the list, the view left as it is.
pub fn delete_active(sim: &SimConfig, scratch: &mut Scratch, out: &mut Out) {
    if let Some((id, _)) = scratch.charts.active(sim) {
        scratch.charts.delete(id);
        out.save_settings = true;
    }
}

/// `q` as the Chart rows read it: its non-zero components, each with its axis, "1.00·z_α".
pub fn basis_text(q: &Latent) -> String {
    let mut text = String::new();
    for (v, name) in q.iter().zip(AXES).filter(|(v, _)| **v != 0.0) {
        let sign = if v.is_sign_negative() { "−" } else { "+" };
        if text.is_empty() {
            let lead = if v.is_sign_negative() { "−" } else { "" };
            text = format!("{lead}{:.2}·{name}", v.abs());
        } else {
            text.push_str(&format!(" {sign} {:.2}·{name}", v.abs()));
        }
    }
    if text.is_empty() {
        "0".to_owned()
    } else {
        text
    }
}

/// Draws Chart in `section`, rows `h` tall.
pub fn show(ui: &mut Ui, section: Rect, h: f32, clip: Rect, panel: &mut Panel<'_>) {
    title(ui, section, h, panel);
    let plane = panel.snapshot.sim.plane.clone();
    let mut row1 = row_ui(ui, row(section, h, 1));
    row1.add_sized([LABEL_W, h], egui::Label::new("preset"));
    selector(&mut row1, clip, panel);
    for (i, (label, q, edit, components)) in [
        ("horizontal q₁", plane.q1, Q1_EDIT, Q1_COMPONENTS),
        ("vertical q₂", plane.q2, Q2_EDIT, Q2_COMPONENTS),
    ]
    .into_iter()
    .enumerate()
    {
        let mut line = row_ui(ui, row(section, h, 2 + i));
        line.add_sized([LABEL_W, h], egui::Label::new(label));
        let text_w = line.available_width() - 56.0;
        line.add_sized(
            [text_w, h],
            egui::Label::new(RichText::new(basis_text(&q)).monospace()).truncate(),
        );
        let open = panel.scratch.editing[i] || panel.focus_in(edit);
        let button = line.add(Button::selectable(open, "edit…"));
        if button.clicked() {
            panel.scratch.editing[i] = !panel.scratch.editing[i];
        }
        panel.place(edit, &button, clip);
        if open {
            let at = egui::pos2(section.min.x, button.rect.max.y);
            if editor_clear(ui, at, i, panel.scratch.console_top) {
                editor(ui, at, i, components, panel);
            } else if panel.focus.last().is_some_and(|f| f != edit) && panel.focus_in(edit) {
                // Its fields are not drawn, so the keys must not reach them.
                panel.out.focus = Some(edit);
            }
        }
    }
    let mut last = row_ui(ui, row(section, h, 4));
    let builder = last.add(Button::new("Chart builder…"));
    if builder.clicked() {
        panel
            .out
            .windows
            .push(engine::contract::view_ui::Window::ChartBuilder);
    }
    panel.place(BUILDER, &builder, clip);
    let save = last.add(Button::new(saved::SAVE_TEXT));
    if save.clicked() {
        panel.out.chart = Some(Switch::Save);
    }
    panel.place(saved::SAVE, &save, clip);
}

/// The header: the title, the kind and, on a saved chart, its provenance, whole in the tooltip.
fn title(ui: &mut Ui, section: Rect, h: f32, panel: &Panel<'_>) {
    let mut line = row_ui(ui, row(section, h, 0));
    line.label(RichText::new(super::SECTION_TITLES[0]).strong());
    line.add_space(8.0);
    line.label(RichText::new(KIND).weak().small());
    if let Some((_, chart)) = panel.scratch.charts.active(&panel.snapshot.sim) {
        let lines = chart.provenance();
        line.add(Label::new(RichText::new(lines.join(" · ")).weak().small()).truncate())
            .on_hover_text(lines.join("\n"));
    }
}

/// The preset selector in `row`, its Saved group under the presets; on a saved chart its rename and delete beside it, or
/// the name field in its place while it is renamed.
fn selector(row: &mut Ui, clip: Rect, panel: &mut Panel<'_>) {
    let sim = &panel.snapshot.sim;
    let view = View::read(sim);
    let charts = &mut panel.scratch.charts;
    let active = charts.active(sim).map(|(id, _)| id);
    let listed = active.filter(|id| charts.get(*id).is_some());
    let naming = listed.filter(|_| charts.naming.is_some());
    let gap = row.spacing().item_spacing.x;
    let buttons = listed.map_or(0.0, |_| BUTTONS_W + 2.0 * gap);
    let width = row.available_width() - 4.0 - buttons;
    match naming {
        Some(id) => name_field(row, width, id, clip, panel),
        None => picker(row, width, view, active, clip, panel),
    }
    let Some(id) = listed else { return };
    for (scope, text) in [
        (saved::RENAME, saved::RENAME_TEXT),
        (saved::DELETE, saved::DELETE_TEXT),
    ] {
        let button = row.add_enabled(naming.is_none(), Button::new(text).small());
        panel.place(scope, &button, clip);
        if !button.clicked() {
            continue;
        }
        if scope == saved::RENAME {
            begin_rename(&panel.snapshot.sim, panel.scratch);
        } else {
            panel.scratch.charts.delete(id);
            panel.out.save_settings = true;
        }
    }
}

/// The picker: the presets, then the saved charts, each with its provenance and its delete.
fn picker(
    row: &mut Ui,
    width: f32,
    view: View,
    active: Option<u64>,
    clip: Rect,
    panel: &mut Panel<'_>,
) {
    let sim = &panel.snapshot.sim;
    let charts = &panel.scratch.charts;
    let preset = view.preset.filter(|_| active.is_none());
    let mut chosen = preset;
    let mut opened = None;
    let mut deleted = None;
    let name = charts.label(sim).map_or_else(|| view.name(), str::to_owned);
    let combo = ComboBox::from_id_salt("chart preset")
        .selected_text(name)
        .width(width)
        .show_ui(row, |ui| {
            for (i, &(a, b)) in PRESETS.iter().enumerate() {
                ui.selectable_value(&mut chosen, Some(i), format!("{} × {}", AXES[a], AXES[b]));
            }
            if charts.entries().is_empty() {
                return;
            }
            ui.separator();
            ui.label(RichText::new(saved::GROUP).weak());
            for (id, chart) in charts.entries() {
                let lines = chart.provenance();
                ui.horizontal(|ui| {
                    let entry = ui
                        .selectable_label(active == Some(*id), &chart.name)
                        .on_hover_text(lines.join("\n"));
                    if entry.clicked() {
                        opened = Some(*id);
                    }
                    ui.label(RichText::new(lines[..2].join(" · ")).weak().small());
                    if ui.add(Button::new(saved::DELETE_TEXT).small()).clicked() {
                        deleted = Some(*id);
                    }
                });
            }
        });
    close_under_console(&combo.response, panel.scratch.console_top);
    panel.place(PRESET, &combo.response, clip);
    if let Some(preset) = chosen.filter(|p| Some(*p) != preset) {
        pick_preset(&panel.snapshot.sim, preset, panel.scratch, panel.out);
    }
    if let Some(id) = opened {
        panel.out.chart = Some(Switch::Open(id));
    }
    if let Some(id) = deleted {
        panel.scratch.charts.delete(id);
        panel.out.save_settings = true;
    }
}

/// The active saved chart's name, typed: kept as it is typed, a blank one not, until the field loses the keyboard.
fn name_field(row: &mut Ui, width: f32, id: u64, clip: Rect, panel: &mut Panel<'_>) {
    let Some(mut name) = panel.scratch.charts.naming.take() else {
        return;
    };
    let field = row.add(TextEdit::singleline(&mut name).desired_width(width - 8.0));
    panel.place(saved::NAME, &field, clip);
    if field.changed() {
        panel.scratch.charts.rename(id, &name);
    }
    if field.lost_focus() {
        panel.out.save_settings = true;
        if panel.focus_in(saved::NAME) {
            panel.out.focus = Some(saved::RENAME);
        }
        return;
    }
    if !field.has_focus() {
        // The keyboard layer follows egui's focus into a text scope and starts typing in it.
        field.request_focus();
    }
    panel.scratch.charts.naming = Some(name);
}

/// The id of the area of basis vector `which`'s editor.
fn editor_id(which: usize) -> Id {
    Id::new(("basis editor", which))
}

/// Whether basis vector `which`'s editor, at `at`, ends above the console's top edge `console_top`: its layer is above
/// the console's, so an editor the console would cover, in part or whole, is not drawn and takes no input, and comes
/// back when uncovered (R-463 K2).
fn editor_clear(ui: &Ui, at: egui::Pos2, which: usize, console_top: Option<f32>) -> bool {
    console_top.is_none_or(|top| {
        let drawn = ui.ctx().memory(|m| m.area_rect(editor_id(which)));
        editor_ends_above(at.y, drawn.map(|r| r.height()), top)
    })
}

/// Whether an editor at `y`, `drawn` tall when last drawn, ends at or above `top`; never drawn, it takes its two rows.
pub fn editor_ends_above(y: f32, drawn: Option<f32>, top: f32) -> bool {
    y + drawn.unwrap_or(2.0 * EDITOR_FIELD_H) <= top
}

/// The editor of basis vector `which` (0 for `q₁`), at `at`, under its row, over the panel and never the figure: eight
/// fields in two rows, each a `SetField` on the basis.
fn editor(
    ui: &mut Ui,
    at: egui::Pos2,
    which: usize,
    ids: [&'static str; 8],
    panel: &mut Panel<'_>,
) {
    let plane = panel.snapshot.sim.plane.clone();
    let mut basis = [plane.q1, plane.q2];
    let mut changed = false;
    let area = Area::new(editor_id(which))
        .order(Order::Foreground)
        .fixed_pos(at)
        .show(ui.ctx(), |ui| {
            Frame::popup(ui.style()).show(ui, |ui| {
                Grid::new(("basis editor grid", which))
                    .num_columns(8)
                    .show(ui, |ui| {
                        for (i, name) in AXES.iter().enumerate() {
                            ui.label(*name);
                            let field = ui.add_sized(
                                [EDITOR_FIELD_W, EDITOR_FIELD_H],
                                DragValue::new(&mut basis[which][i])
                                    .speed(0.005)
                                    .max_decimals(3),
                            );
                            changed |= field.changed();
                            let clip = field.rect;
                            panel.place(ids[i], &field, clip);
                            if i == 3 {
                                ui.end_row();
                            }
                        }
                    });
            });
        });
    let _ = area;
    if changed {
        panel.out.edit(
            plane_edit(&panel.snapshot.sim, (basis[0], basis[1])),
            None,
            panel.scratch,
        );
    }
}
