//! Slice centre (render_gui_spec §G2; named "Centre z₀" before R-457): a "reset" beside its heading (`super::reset`;
//! R-466) and eight sliders, `z_α, z_β, z_q0…z_q3, z_μ1,
//! z_μ2`, each with its number box, dragged or typed, both editing the same component of `z₀`. The slider of the slice
//! normal's dimension moves the slice along its normal, the step slider that did it before R-457 removed. A free slider
//! spans `[−1, 1]`. Locked, the sliders are re-based to the anchor, not frozen (§G4;
//! chart_decoder_contract Part 4, "Sliders are re-based"): each spans `anchor ± 1`, marks the anchor in gold at its
//! centre, and reads anchor plus offset, `z_locked + δ`; moving one is an excursion, a `SetField` on `z₀`, the anchor
//! kept. Touching one switches the compass to Slice, its buttons following (R-409), on the drag's first frame: the
//! slider or its box pressed, or the box clicked into to type, whether or not the value changes (R-458 F4). A drag
//! applies its value every frame it changes, the whole drag one history entry (R-458 F1).

use eframe::egui::{self, DragValue, Rect, RichText, Slider, Stroke, Ui};

use super::{reset::Section, row, row_ui, z0_edit, Panel, Touch, AXES, SLIDERS, Z0_HALF_RANGE};
use crate::explore::lock::GOLD;

/// The width of the value field, in points (a look choice, R-390).
pub const VALUE_W: f32 = 58.0;
/// The width of the locked offset's readout, in points (a look choice, R-390).
pub const OFFSET_W: f32 = 64.0;
/// The fewest and the most decimals the value field shows where they fit (a look choice).
pub const DECIMALS: std::ops::RangeInclusive<usize> = 3..=6;

/// `value` as the value field shows it, `fits` telling whether a text fits the field: egui's own reading at the most
/// decimals, from [`DECIMALS`]' most down to none, that fits; past that the scientific form at the most digits that
/// fits, and its shortest, at most seven characters ("-1e-308"), where none does. So the text is bounded for any value:
/// a number box extends to its text, and a value of more digits than the field holds, which nothing rounds on its way
/// here (a preset's base point, a typed number, a point off the slider's span), would push the field past its row's end.
pub fn value_text(value: f64, fits: impl Fn(&str) -> bool) -> String {
    let (least, most) = (*DECIMALS.start(), *DECIMALS.end());
    for decimals in (0..=most).rev() {
        let text =
            egui::emath::format_with_decimals_in_range(value, least.min(decimals)..=decimals);
        if fits(&text) {
            return text;
        }
    }
    (1..=3)
        .rev()
        .map(|digits| format!("{value:.digits$e}"))
        .find(|text| fits(text))
        .unwrap_or_else(|| format!("{value:.0e}"))
}

/// What a number box's text reads as, as egui's own parser reads it: whitespace dropped, a true minus a hyphen.
fn parsed(text: &str) -> Option<f64> {
    let plain: String = text
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| if c == '−' { '-' } else { c })
        .collect();
    plain.parse().ok()
}

/// The value field of a slider row, [`VALUE_W`] wide whatever `value` is: its text is [`value_text`] measured in the
/// field's own font. A number box writes its text back when it loses the focus, so the text it showed reads as the
/// value it showed it for, digit for digit: a field clicked into and left, nothing typed, leaves the value as it was,
/// however few digits fitted.
fn value_field(ui: &mut Ui, height: f32, value: &mut f64) -> egui::Response {
    let font = ui.style().drag_value_text_style.resolve(ui.style());
    let width = VALUE_W - 2.0 * ui.spacing().button_padding.x;
    let ctx = ui.ctx().clone();
    let fits = move |text: &str| {
        let galley = ctx.fonts_mut(|fonts| {
            fonts.layout_no_wrap(text.to_owned(), font.clone(), egui::Color32::WHITE)
        });
        galley.size().x <= width
    };
    let (held, shown) = (*value, value_text(*value, &fits));
    ui.add_sized(
        [VALUE_W, height],
        DragValue::new(value)
            .speed(0.002)
            // The decimals a drag or a key's step rounds to; the text is the formatter's.
            .min_decimals(*DECIMALS.start())
            .max_decimals(*DECIMALS.end())
            .custom_formatter(move |v, _| value_text(v, &fits))
            .custom_parser(move |text| {
                if text == shown {
                    Some(held)
                } else {
                    parsed(text)
                }
            }),
    )
}

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

/// Whether a slider row with `slider` and its number box `field` is touched this frame (R-458 F4): either pressed, or
/// the box clicked into to type, so a press at a slider's end, or a click into the box, counts. A changed value counts
/// too: its edit carries the row's touch ([`Out::edit`](super::Out::edit)), which sets the mode.
pub fn touched(slider: &egui::Response, field: &egui::Response) -> bool {
    slider.is_pointer_button_down_on() || field.is_pointer_button_down_on() || field.gained_focus()
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
    let field = value_field(&mut line, rect.height() - 2.0, &mut moved);
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
    super::header_with_reset(ui, section, h, clip, note, Section::SliceCentre, panel);
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
