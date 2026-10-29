//! Slice centre (render_gui_spec §G2; named "Centre z₀" before R-457): eight sliders, `z_α, z_β, z_q0…z_q3, z_μ1,
//! z_μ2`, each with its number box, dragged or typed, both editing the same component of `z₀`. The slider of the slice
//! normal's dimension moves the slice along its normal, the step slider that did it before R-457 removed. A free slider
//! spans `[−1, 1]`. Locked, the sliders are re-based to the anchor, not frozen (§G4;
//! chart_decoder_contract Part 4, "Sliders are re-based"): each spans `anchor ± 1`, marks the anchor in gold at its
//! centre, and reads anchor plus offset, `z_locked + δ`; moving one is an excursion, a `SetField` on `z₀`, the anchor
//! kept. Touching one switches the compass to Slice, its buttons following (R-409), on the drag's first frame: the
//! slider or its box pressed, or the box clicked into to type, whether or not the value changes (R-458 F4). A drag
//! applies its value every frame it changes, the whole drag one history entry (R-458 F1).

use eframe::egui::{self, DragValue, Rect, RichText, Slider, Stroke, Ui};

use super::{
    header, row, row_ui, z0_edit, Panel, Touch, AXES, SECTION_TITLES, SLIDERS, Z0_HALF_RANGE,
};
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

/// Whether a slider row with `slider` and its number box `field` is touched this frame (R-458 F4): either pressed, the
/// box clicked into to type, or the value changed, so a press at a slider's end, or a click into the box, counts.
pub fn touched(slider: &egui::Response, field: &egui::Response) -> bool {
    slider.is_pointer_button_down_on()
        || field.is_pointer_button_down_on()
        || field.gained_focus()
        || slider.changed()
        || field.changed()
}

/// One slider row: the label, the slider over `span`, the value field, and while locked the anchor's mark and the
/// offset. Returns the new value if the user moved it, the slider's response, and whether the row is [`touched`].
pub fn slider_row(
    ui: &mut Ui,
    rect: Rect,
    label: &str,
    value: f64,
    anchor: Option<f64>,
) -> (Option<f64>, egui::Response, bool) {
    let mut line = row_ui(ui, rect);
    line.add_sized([40.0, rect.height()], egui::Label::new(label));
    // The slider takes what the field, the offset while locked, and the gaps before them leave.
    let gap = line.spacing().item_spacing.x;
    let after = if anchor.is_some() {
        VALUE_W + gap + OFFSET_W + gap
    } else {
        VALUE_W + gap
    };
    line.spacing_mut().slider_width = (line.available_width() - after).max(40.0);
    let (lo, hi) = span(anchor);
    let mut moved = value;
    let slider = line.add(
        Slider::new(&mut moved, lo..=hi)
            .show_value(false)
            .clamping(egui::SliderClamping::Never),
    );
    if anchor.is_some() {
        // The span is re-based about the anchor, so the anchor sits at the rail's centre.
        let rail = slider.rect;
        let x = rail.center().x;
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
    let touch = touched(&slider, &field);
    let response = slider.union(field);
    (changed.then_some(moved), response, touch)
}

/// Draws Slice centre in `section`.
pub fn show(ui: &mut Ui, section: Rect, h: f32, clip: Rect, panel: &mut Panel<'_>) {
    let lock = panel.snapshot.sim.lock.clone();
    let note = if lock.locked {
        "re-based to the anchor"
    } else {
        ""
    };
    header(ui, section, h, SECTION_TITLES[2], note);
    let z0 = panel.snapshot.sim.plane.z0;
    for (i, name) in AXES.iter().enumerate() {
        let anchor = lock.locked.then_some(lock.z_locked[i]);
        let (moved, response, touch) = slider_row(ui, row(section, h, 1 + i), name, z0[i], anchor);
        panel.place(SLIDERS[i], &response, clip);
        if touch {
            panel.scratch.compass.mode = Touch::Slice;
        }
        if let Some(value) = moved {
            let mut next = z0;
            next[i] = value;
            panel
                .out
                .edit(z0_edit(next), Some(Touch::Slice), panel.scratch);
        }
    }
}
