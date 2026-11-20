//! The keyboard breadcrumb and the focus ring (render_gui_spec §G3: "a focus ring on the current scope, orange in
//! navigation mode and blue in interaction mode (R-409), and the breadcrumb in the top bar (e.g. "Manifold view ›
//! Navigate › zoom"). Nothing else changes on screen."). Both are filled from `ViewUI`'s focus scope
//! (gui_state_contract §2), and both are drawn in the mode's colour: in interaction mode 01_main.png's and
//! 07_keyboard.png's blue, egui's dark theme's hyperlink colour, confirmed by R-404; in navigation mode
//! [`NAVIGATION`]'s orange (R-409).

use eframe::egui::{self, Color32, Id, LayerId, Order, RichText, Stroke, StrokeKind, Ui};
use engine::contract::view_ui::{Focus, KeyMode};

use crate::keyboard::scopes::{Ring, ScopeTree};
use crate::keyboard::Place;

/// Between the breadcrumb's scopes.
pub const SEPARATOR: &str = " › ";
/// The glyph before the breadcrumb, as 01_main.png draws it.
pub const GLYPH: &str = "⌨";
/// The ring's width, in points: confirmed by R-404.
pub const RING_WIDTH: f32 = 2.0;
/// The ring's corner rounding, in points (a look choice, R-390): confirmed by R-404.
pub const RING_ROUNDING: f32 = 2.0;
/// The navigation mode's orange (R-409: "it should be orange when you're navigating"): a look choice, R-390, an
/// orange apart from the theme's warning amber and the lock's gold, as bright on the dark panels as the blue.
pub const NAVIGATION: Color32 = Color32::from_rgb(240, 120, 40);

/// The ring's colour in `mode`: [`NAVIGATION`]'s orange, or, interacting, the theme's hyperlink blue, confirmed by
/// R-404 and kept by R-409 for interaction mode.
pub fn colour(visuals: &egui::Visuals, mode: KeyMode) -> Color32 {
    match mode {
        KeyMode::Navigation => NAVIGATION,
        KeyMode::Interaction => visuals.hyperlink_color,
    }
}

/// The breadcrumb's colour in `mode`: the ring's, so the breadcrumb shows the mode too (R-409 A6: a look choice,
/// R-390).
pub fn breadcrumb_colour(visuals: &egui::Visuals, mode: KeyMode) -> Color32 {
    colour(visuals, mode)
}

/// The breadcrumb of `focus` in `tree`: its scopes' labels, outermost first; empty when nothing has focus.
pub fn text(tree: &ScopeTree, focus: &Focus) -> String {
    tree.labels(&focus.path).join(SEPARATOR)
}

/// Draws the breadcrumb `text` in `mode`'s colour, in a right-to-left row, after the status line: so the glyph goes
/// after the text.
pub fn show(ui: &mut Ui, text: &str, mode: KeyMode) {
    if text.is_empty() {
        return;
    }
    let colour = breadcrumb_colour(ui.visuals(), mode);
    ui.add_space(12.0);
    ui.label(RichText::new(text).monospace().color(colour));
    ui.label(RichText::new(GLYPH).color(colour));
}

/// Draws the focus ring on `place` in `mode`'s colour, above the figure's layer, inside the rect or outside it as
/// `ring` says: on the part of it that shows, so a ring on a widget a scroll area moves stays in the area, and none when
/// nothing shows.
pub fn ring(ctx: &egui::Context, place: Place, ring: Ring, mode: KeyMode) {
    let shown = place.shown();
    if !shown.is_positive() {
        return;
    }
    let colour = colour(&ctx.global_style().visuals, mode);
    // Outside, the rect grows by a point first, so the stroke's anti-aliasing stays off the rect too.
    let (rect, kind) = match ring {
        Ring::Inside => (shown, StrokeKind::Inside),
        Ring::Outside => (shown.expand(1.0), StrokeKind::Outside),
    };
    ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("focus ring")))
        .rect_stroke(rect, RING_ROUNDING, Stroke::new(RING_WIDTH, colour), kind);
}
