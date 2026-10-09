//! The keyboard breadcrumb and the focus ring (render_gui_spec §G3: "a focus ring on the current scope and the
//! breadcrumb in the top bar (e.g. "Manifold view › Navigate › zoom"). Nothing else changes on screen."). Both are
//! filled from `ViewUI`'s focus scope (gui_state_contract §2), and both are drawn in one colour, 01_main.png's and
//! 07_keyboard.png's blue, egui's dark theme's hyperlink colour (a look choice, flagged for the human, R-390).

use eframe::egui::{self, Color32, Id, LayerId, Order, RichText, Stroke, StrokeKind, Ui};
use engine::contract::view_ui::Focus;

use crate::keyboard::scopes::{Ring, ScopeTree};
use crate::keyboard::Place;

/// Between the breadcrumb's scopes.
pub const SEPARATOR: &str = " › ";
/// The glyph before the breadcrumb, as 01_main.png draws it.
pub const GLYPH: &str = "⌨";
/// The ring's width, in points.
pub const RING_WIDTH: f32 = 2.0;
/// The ring's corner rounding, in points (a look choice, flagged for the human, R-390).
pub const RING_ROUNDING: f32 = 2.0;

/// The breadcrumb's and the ring's colour.
pub fn colour(visuals: &egui::Visuals) -> Color32 {
    visuals.hyperlink_color
}

/// The breadcrumb of `focus` in `tree`: its scopes' labels, outermost first; empty when nothing has focus.
pub fn text(tree: &ScopeTree, focus: &Focus) -> String {
    tree.labels(&focus.path).join(SEPARATOR)
}

/// Draws the breadcrumb `text`, in a right-to-left row, after the status line: so the glyph goes after the text.
pub fn show(ui: &mut Ui, text: &str) {
    if text.is_empty() {
        return;
    }
    let colour = colour(ui.visuals());
    ui.add_space(12.0);
    ui.label(RichText::new(text).monospace().color(colour));
    ui.label(RichText::new(GLYPH).color(colour));
}

/// Draws the focus ring on `place`, above the figure's layer, inside the rect or outside it as `ring` says.
pub fn ring(ctx: &egui::Context, place: Place, ring: Ring) {
    let colour = colour(&ctx.global_style().visuals);
    // Outside, the rect grows by a point first, so the stroke's anti-aliasing stays off the rect too.
    let (rect, kind) = match ring {
        Ring::Inside => (place.rect, StrokeKind::Inside),
        Ring::Outside => (place.rect.expand(1.0), StrokeKind::Outside),
    };
    ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("focus ring")))
        .rect_stroke(rect, RING_ROUNDING, Stroke::new(RING_WIDTH, colour), kind);
}
