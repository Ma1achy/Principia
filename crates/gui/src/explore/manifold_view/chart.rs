//! Chart (render_gui_spec §G2): the preset, named by its axes; the two basis vectors `q₁`, `q₂`, each with an edit
//! button, which opens its eight components for drag or typing; Chart builder…, drawn disabled until TASK-M6-28 builds
//! its window; and the chart's kind. A preset keeps the zoom; locked, it turns about the pin, as every basis edit does
//! ([`super::about_the_pin`], §G4). The shape sphere's projection selector and hemisphere toggle are REQ-GUI-161's,
//! TASK-M8-06's: the mock's charts are all affine.

use eframe::egui::{
    self, Area, Button, ComboBox, DragValue, Frame, Grid, Id, Order, Rect, RichText, Ui,
};
use engine::contract::set_field::SetField;
use engine::contract::sim_config::Latent;

use super::{
    axis, basis_edit, header, row, row_ui, Panel, View, AXES, BUILDER, PRESET, PRESETS,
    Q1_COMPONENTS, Q1_EDIT, Q2_COMPONENTS, Q2_EDIT,
};

/// The chart's kind: every chart the mock serves is the affine slice (chart_decoder_contract Part 3).
pub const KIND: &str = "affine chart";
/// The width of a row's label, in points (a look choice, R-390).
pub const LABEL_W: f32 = 92.0;
/// The width of a basis editor's field, in points (a look choice, R-390): four to a row fit the panel.
pub const EDITOR_FIELD_W: f32 = 44.0;

/// The basis edit that switches `view` to preset `preset`, keeping its scale: `q₁ = σ e_a`, `q₂ = σ e_b`.
pub fn preset_basis(view: View, preset: usize) -> SetField {
    let (a, b) = PRESETS[preset];
    basis_edit((axis(a, view.scale), axis(b, view.scale)))
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
    header(ui, section, h, "Chart", "");
    let plane = panel.snapshot.sim.plane.clone();
    let view = View::of(&plane);
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
    panel.place(PRESET, &combo.response, clip);
    if let Some(preset) = chosen.filter(|p| Some(*p) != view.preset) {
        panel
            .out
            .edit(preset_basis(view, preset), None, panel.scratch);
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
            editor(ui, at, i, components, panel);
        }
    }
    let mut last = row_ui(ui, row(section, h, 4));
    let builder = last.add_enabled(false, Button::new("Chart builder…"));
    panel.place(BUILDER, &builder, clip);
    last.add_space(8.0);
    last.label(RichText::new(KIND).monospace().weak());
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
    let area = Area::new(Id::new(("basis editor", which)))
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
                                [EDITOR_FIELD_W, 18.0],
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
        panel
            .out
            .edit(basis_edit((basis[0], basis[1])), None, panel.scratch);
    }
}
