//! Chart (render_gui_spec §G2): the preset, named by its axes; the two basis vectors `q₁`, `q₂`, each with an edit
//! button, which opens its eight components for drag or typing; Chart builder…, which opens its window (TASK-M6-28);
//! and the chart's kind. A preset keeps the zoom; locked, it turns about the pin, as every basis edit does
//! ([`super::about_the_pin`], §G4). A preset, or a basis edit, whose plane takes the slice normal moves it off the
//! plane by the slice-axis rule in the same edit, a target following ([`super::plane_edit`]; R-459 G4). The shape
//! sphere's projection selector and hemisphere toggle are REQ-GUI-161's, TASK-M8-06's: the mock's charts are all
//! affine.

use eframe::egui::{
    self, Area, Button, ComboBox, DragValue, Frame, Grid, Id, Order, Rect, RichText, Ui,
};
use engine::contract::set_field::SetField;
use engine::contract::sim_config::{Latent, SimConfig};

use super::{
    axis, close_under_console, header, plane_edit, row, row_ui, Panel, View, AXES, BUILDER, PRESET,
    PRESETS, Q1_COMPONENTS, Q1_EDIT, Q2_COMPONENTS, Q2_EDIT,
};

/// The chart's kind: every chart the mock serves is the affine slice (chart_decoder_contract Part 3).
pub const KIND: &str = "affine chart";
/// The width of a row's label, in points (a look choice, R-390).
pub const LABEL_W: f32 = 92.0;
/// The width of a basis editor's field, in points (a look choice, R-390): four to a row fit the panel.
pub const EDITOR_FIELD_W: f32 = 44.0;
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
    header(ui, section, h, super::SECTION_TITLES[0], "");
    let plane = panel.snapshot.sim.plane.clone();
    let view = View::read(&panel.snapshot.sim);
    let mut row1 = row_ui(ui, row(section, h, 1));
    row1.add_sized([LABEL_W, h], egui::Label::new("preset"));
    let mut chosen = view.preset;
    let combo = ComboBox::from_id_salt("chart preset")
        .selected_text(view.name())
        .width(row1.available_width() - 4.0)
        .show_ui(&mut row1, |ui| {
            for (i, &(a, b)) in PRESETS.iter().enumerate() {
                ui.selectable_value(&mut chosen, Some(i), format!("{} × {}", AXES[a], AXES[b]));
            }
        });
    close_under_console(&combo.response, panel.scratch.console_top);
    panel.place(PRESET, &combo.response, clip);
    if let Some(preset) = chosen.filter(|p| Some(*p) != view.preset) {
        panel.out.edit(
            preset_edit(&panel.snapshot.sim, view, preset),
            None,
            panel.scratch,
        );
    }
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
    last.add_space(8.0);
    last.label(RichText::new(KIND).monospace().weak());
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
