//! The decoder badge (render_gui_spec §G2's Figure, §G7a): in the figure's header strip, the chrome above the plot and
//! outside it, left-aligned. It names the chart by its axes and, while any block is off its default, each such block
//! after a "·": "z_α × z_β · stick-breaking masses", "z_α × z_β · μ_max 4 masses". At the defaults, the chart alone.

use eframe::egui::{self, Align, Layout as EguiLayout, Rect, RichText, Ui, UiBuilder};
use engine::contract::links::{badge_text, off_default};
use engine::contract::sim_config::Links;

/// The badge's text for the chart named `chart` under `links`.
pub fn text(chart: &str, links: &Links) -> String {
    let mut parts = vec![chart.to_owned()];
    parts.extend(
        off_default(links)
            .into_iter()
            .map(|b| badge_text(b, b.of(links))),
    );
    parts.join(" · ")
}

/// The badge's tooltip: off the default, that the figure was decoded with the default links all the same.
pub fn tooltip(links: &Links) -> Option<String> {
    (!off_default(links).is_empty()).then(|| crate::windows::decoder::NOT_DECODED.to_owned())
}

/// Draws the badge `text` in the header strip `strip`, amber while the decoder is off its default, with `tooltip`.
pub fn show(ui: &mut Ui, strip: Rect, text: &str, off: bool, tooltip: Option<String>) {
    let mut line = ui.new_child(
        UiBuilder::new()
            .max_rect(strip.shrink2(egui::vec2(6.0, 0.0)))
            .layout(EguiLayout::left_to_right(Align::Center)),
    );
    let mut rich = RichText::new(text).small();
    if off {
        rich = rich.color(line.visuals().warn_fg_color);
    }
    let badge = line.add(egui::Label::new(rich).truncate());
    if let Some(t) = tooltip {
        badge.on_hover_text(t);
    }
}
