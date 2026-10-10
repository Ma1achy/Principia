//! The figure's axis labels (render_gui_spec §G2, "Axis labels carry the short axis name and the range at each end";
//! GUI_DESIGN_NOTES § "01 Explore"): each axis's short name and its value at each end of the figure, in the strips
//! beside the figure, never over it (§G1). On the mock they are read from its chart: the axis `q₁` (`q₂`) leans on
//! most, and that coordinate's value at `s = 0` and `s = 1` (`t = 0` and `t = 1`) through the figure's centre, so a
//! pan updates them (applied per R-369, review 5434766412 on PR #157); REQ-GUI-084 stays TASK-M8-06's, on the real
//! engine's chart ranges.

use std::f32::consts::FRAC_PI_2;

use eframe::egui::{self, Align, Align2, Layout as EguiLayout, Rect, RichText, Ui, UiBuilder};
use engine::contract::sim_config::Plane;

use crate::explore::manifold_view::{dominant, AXES};

/// One axis's label: its short name and its value at each end.
#[derive(Clone, Debug, PartialEq)]
pub struct Axis {
    /// The short axis name, e.g. "z_α".
    pub name: &'static str,
    /// The value at the low end: the left, or the bottom.
    pub low: f64,
    /// The value at the high end: the right, or the top.
    pub high: f64,
}

/// The horizontal and the vertical axis of `plane`.
pub fn axes(plane: &Plane) -> [Axis; 2] {
    [(&plane.q1), (&plane.q2)].map(|q| {
        let i = dominant(q);
        Axis {
            name: AXES[i],
            low: plane.z0[i] - q[i],
            high: plane.z0[i] + q[i],
        }
    })
}

/// An end's value with its sign, a true minus, and enough decimals to tell the two ends apart: two at least, more as
/// the span narrows, at most twelve.
pub fn end_text(value: f64, span: f64) -> String {
    let decimals = (2.0 - span.abs().max(1e-300).log10())
        .ceil()
        .clamp(2.0, 12.0) as usize;
    let sign = if value < 0.0 { "−" } else { "+" };
    format!("{sign}{:.*}", decimals, value.abs())
}

/// The three texts of an axis: low end, name with its arrow, high end.
pub fn texts(axis: &Axis, arrow: &str) -> [String; 3] {
    let span = axis.high - axis.low;
    [
        end_text(axis.low, span),
        format!("{} {arrow}", axis.name),
        end_text(axis.high, span),
    ]
}

/// Draws the labels of `plane`: the horizontal axis in `strip_x`, under the figure, the low end at its left, the name
/// centred, the high end at its right; the vertical axis in `strip_y`, left of the figure, turned to read upwards.
pub fn show(ui: &mut Ui, strip_x: Rect, strip_y: Rect, figure: Rect, plane: &Plane) {
    let [horizontal, vertical] = axes(plane);
    let [low, name, high] = texts(&horizontal, "→");
    let strip = Rect::from_min_max(egui::pos2(figure.min.x, strip_x.min.y), strip_x.max)
        .shrink2(egui::vec2(4.0, 0.0));
    for (text, layout) in [
        (low, EguiLayout::left_to_right(Align::Center)),
        (
            name,
            EguiLayout::centered_and_justified(egui::Direction::LeftToRight),
        ),
        (high, EguiLayout::right_to_left(Align::Center)),
    ] {
        let mut child = ui.new_child(UiBuilder::new().max_rect(strip).layout(layout));
        child.label(RichText::new(text).monospace().small());
    }
    let [low, name, high] = texts(&vertical, "→");
    // Turned to read upwards, the arrow points up the axis.
    let painter = ui.painter_at(strip_y);
    let colour = ui.visuals().text_color();
    let font = egui::TextStyle::Small.resolve(ui.style());
    let font = egui::FontId::monospace(font.size);
    let x = strip_y.center().x;
    for (text, y, anchor) in [
        (low, strip_y.max.y - 4.0, Align2::LEFT_CENTER),
        (name, strip_y.center().y, Align2::CENTER_CENTER),
        (high, strip_y.min.y + 4.0, Align2::RIGHT_CENTER),
    ] {
        let galley = painter.layout_no_wrap(text, font.clone(), colour);
        // The anchor's point of the unturned text lands on `(x, y)`, and the text turns about it.
        let a0 = anchor.pos_in_rect(&galley.rect).to_vec2();
        let shape = egui::epaint::TextShape::new(egui::pos2(x, y) - a0, galley, colour)
            .with_angle_and_anchor(-FRAC_PI_2, anchor);
        painter.add(shape);
    }
}
