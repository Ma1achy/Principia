//! The Explore page (render_gui_spec §G2; 01_main.png): the top bar, the footer, and the regions — the left Manifold
//! view, the right Trajectory panel, and the bottom row's compass, Time and Legend — as empty frames that later track
//! tasks fill. The figure's rect is left to the figure: nothing is drawn over it (§G1).

pub mod footer;
pub mod top_bar;

use eframe::egui::{self, Rect, RichText, Ui, UiBuilder};

use crate::layout::Layout;

/// The regions' titles, as 01_main.png gives them, with the rect each is drawn in.
pub fn region_titles(layout: &Layout) -> [(&'static str, Rect); 5] {
    [
        ("Manifold view", layout.manifold_view),
        ("Trajectory", layout.trajectory),
        ("Compass", layout.compass),
        ("Time", layout.time),
        ("Legend", layout.legend),
    ]
}

/// Draws one empty region: its frame and its title.
fn region(ui: &mut Ui, title: &str, rect: Rect) {
    let visuals = ui.visuals();
    let (fill, stroke) = (visuals.panel_fill, visuals.widgets.noninteractive.bg_stroke);
    ui.painter().rect_filled(rect, 0.0, fill);
    ui.painter()
        .rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Inside);
    let mut inner = ui.new_child(UiBuilder::new().max_rect(rect.shrink(12.0)));
    inner.label(RichText::new(title).heading());
}

/// Draws the Explore page's regions as empty frames.
pub fn regions(ui: &mut Ui, layout: &Layout) {
    for (title, rect) in region_titles(layout) {
        region(ui, title, rect);
    }
}

/// Draws the Stain page as an empty frame, which TASK-M6-29 fills (RQ-249).
pub fn stain_page(ui: &mut Ui, rect: Rect) {
    region(ui, "Stain", rect);
}
