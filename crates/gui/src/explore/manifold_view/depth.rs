//! The depth readout and its precision warning (render_gui_spec §G2): the depth `2^-k`, from the zoom, and the quad
//! level; the warning is raised by the snapshot's events, `DECODE_SWITCHOVER` on visible quads and `AT_F32_FLOOR`
//! (R-54; gui_state_contract §2), never by a fixed depth. The readout's form is the artboard's, "f64 · 7 digits left ·
//! linearise decode"; the event-driven warning on the real engine is TASK-M6-21's.

use eframe::egui::{self, Rect, RichText, Ui};
use engine::contract::snapshot::Precision;

use super::{row_ui, Panel, View};

/// What the quad level reads: the snapshot carries no quad level yet, so it is absent, "—" (RQ-243's form).
pub const QUAD_LEVEL: &str = "—";
/// The precision row with no event raised.
pub const NO_WARNING: &str = "no warning";
/// The decimal digits of an f64's 53-bit significand, `53 log₁₀ 2`.
const F64_DIGITS: f64 = 53.0 * std::f64::consts::LOG10_2;

/// `n` in superscript digits, with a superscript minus.
pub fn superscript(n: i64) -> String {
    const DIGITS: [char; 10] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];
    let mut out = String::new();
    if n < 0 {
        out.push('⁻');
    }
    for c in n.unsigned_abs().to_string().chars() {
        out.push(DIGITS[c.to_digit(10).expect("a decimal digit") as usize]);
    }
    out
}

/// The depth, `2^-k`, for the zoom `zoom`: `k` is the zoom rounded down to whole octaves.
pub fn depth_text(zoom: f64) -> String {
    let k = zoom.floor() as i64;
    format!("2{} · quad level {QUAD_LEVEL}", superscript(-k))
}

/// The decimal digits an f64 has left at zoom `zoom`: those of its significand less the zoom's, never below zero.
pub fn digits_left(zoom: f64) -> u32 {
    (F64_DIGITS - zoom.max(0.0) * std::f64::consts::LOG10_2)
        .floor()
        .max(0.0) as u32
}

/// The warning the snapshot's events raise at zoom `zoom`; `None` with no event.
pub fn warning(precision: &Precision, zoom: f64) -> Option<String> {
    let mut parts = Vec::new();
    if precision.decode_switchover {
        parts.push(format!(
            "f64 · {} digits left · linearise decode",
            digits_left(zoom)
        ));
    }
    if precision.at_f32_floor {
        parts.push("at the f32 floor".to_owned());
    }
    (!parts.is_empty()).then(|| parts.join(" · "))
}

/// Draws the depth in row `depth` and the precision warning in row `precision`.
pub fn show(ui: &mut Ui, depth: Rect, precision: Rect, panel: &mut Panel<'_>) {
    let zoom = View::of(&panel.snapshot.sim.plane).zoom();
    let label_w = super::chart::LABEL_W;
    let mut line = row_ui(ui, depth);
    line.add_sized([label_w, depth.height()], egui::Label::new("depth"));
    line.label(RichText::new(depth_text(zoom)).monospace());
    let mut line = row_ui(ui, precision);
    line.add_sized([label_w, precision.height()], egui::Label::new("precision"));
    match warning(&panel.snapshot.precision, zoom) {
        Some(text) => {
            let colour = line.visuals().warn_fg_color;
            line.add(egui::Label::new(RichText::new(text).monospace().color(colour)).truncate());
        }
        None => {
            line.label(RichText::new(NO_WARNING).monospace().weak());
        }
    }
}
