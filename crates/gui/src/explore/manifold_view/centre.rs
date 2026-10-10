//! Centre z₀ (render_gui_spec §G2): eight sliders, `z_α, z_β, z_q0…z_q3, z_μ1, z_μ2`, each with its value, dragged or
//! typed. A free slider spans `[−1, 1]`. Locked, the sliders are re-based to the anchor, not frozen (§G4;
//! chart_decoder_contract Part 4, "Sliders are re-based"): each spans `anchor ± 1`, marks the anchor in gold at its
//! centre, and reads anchor plus offset, `z_locked + δ`; moving one is an excursion, a `SetField` on `z₀`, the anchor
//! kept. Touching one shows slicing on the compass.

use eframe::egui::{self, DragValue, Rect, RichText, Slider, Stroke, Ui};

use super::{header, row, row_ui, z0_edit, Panel, Touch, AXES, SLIDERS, Z0_HALF_RANGE};
use crate::explore::lock::GOLD;

/// The width of the value field, in points (a look choice, R-390).
pub const VALUE_W: f32 = 58.0;
/// The width of the locked offset's readout, in points (a look choice, R-390).
pub const OFFSET_W: f32 = 64.0;

/// The span of a slider whose anchor is `anchor` (`None` while free).
pub fn span(anchor: Option<f64>) -> (f64, f64) {
    let centre = anchor.unwrap_or(0.0);
    (centre - Z0_HALF_RANGE, centre + Z0_HALF_RANGE)
}

/// The offset `δ = z − anchor` as the readout shows it: "+ 0.050", a true minus for a negative.
pub fn offset_text(value: f64, anchor: f64) -> String {
    let delta = value - anchor;
    let sign = if delta < 0.0 { "−" } else { "+" };
    format!("{sign} {:.3}", delta.abs())
}

/// One slider row: the label, the slider over `span`, the value field, and while locked the anchor's mark and the
/// offset. Returns the new value if the user moved it, and the slider's response.
pub fn slider_row(
    ui: &mut Ui,
    rect: Rect,
    label: &str,
    value: f64,
    anchor: Option<f64>,
) -> (Option<f64>, egui::Response) {
    let mut line = row_ui(ui, rect);
    line.add_sized([40.0, rect.height()], egui::Label::new(label));
    let extra = if anchor.is_some() { OFFSET_W } else { 0.0 };
    line.spacing_mut().slider_width = (line.available_width() - VALUE_W - extra - 12.0).max(40.0);
    let (lo, hi) = span(anchor);
    let mut moved = value;
    let slider = line.add(
        Slider::new(&mut moved, lo..=hi)
            .show_value(false)
            .clamping(egui::SliderClamping::Never),
    );
    if let Some(anchor) = anchor {
        let rail = slider.rect;
        let x = rail.min.x + rail.width() * ((anchor - lo) / (hi - lo)) as f32;
        line.painter().line_segment(
            [
                egui::pos2(x, rail.min.y + 2.0),
                egui::pos2(x, rail.max.y - 2.0),
            ],
            Stroke::new(2.0, GOLD),
        );
    }
    let field = line.add_sized(
        [VALUE_W, rect.height() - 2.0],
        DragValue::new(&mut moved)
            .speed(0.002)
            .min_decimals(3)
            .max_decimals(6),
    );
    if let Some(anchor) = anchor {
        line.add_sized(
            [OFFSET_W, rect.height()],
            egui::Label::new(
                RichText::new(offset_text(moved, anchor))
                    .monospace()
                    .small()
                    .weak(),
            ),
        );
    }
    // Only a change the user made counts: a value the widget clamps for display is not an edit.
    let changed = slider.changed() || field.changed();
    let response = slider.union(field);
    (changed.then_some(moved), response)
}

/// Draws Centre z₀ in `section`.
pub fn show(ui: &mut Ui, section: Rect, h: f32, clip: Rect, panel: &mut Panel<'_>) {
    let lock = panel.snapshot.sim.lock.clone();
    let note = if lock.locked {
        "re-based to the anchor"
    } else {
        ""
    };
    header(ui, section, h, "Centre z₀", note);
    let z0 = panel.snapshot.sim.plane.z0;
    for (i, name) in AXES.iter().enumerate() {
        let anchor = lock.locked.then_some(lock.z_locked[i]);
        let (moved, response) = slider_row(ui, row(section, h, 1 + i), name, z0[i], anchor);
        panel.place(SLIDERS[i], &response, clip);
        if let Some(value) = moved {
            let mut next = z0;
            next[i] = value;
            panel
                .out
                .edit(z0_edit(next), Some(Touch::Slice), panel.scratch);
        }
    }
}
