//! The Explore page (render_gui_spec §G2; 01_main.png): the top bar, the footer, and the regions — the left Manifold
//! view, the right Trajectory panel, and the bottom row's compass, Time and Legend — as empty frames that later track
//! tasks fill. The figure's rect is left to the figure: nothing is drawn over it (§G1). Each region is a big scope of
//! the keyboard's tree (§G3), and the Manifold view's four sub-scopes are reached with Enter.

pub mod breadcrumb;
pub mod footer;
pub mod top_bar;

use eframe::egui::{self, vec2, Rect, RichText, Ui, UiBuilder};

use crate::keyboard::scopes::{Scope, ScopeId, ScopeTree};
use crate::keyboard::Keyboard;
use crate::layout::Layout;

/// The Manifold view's sub-scopes, in order (§G3): Chart, Navigate, Centre z₀, Slice & tilt.
pub const MANIFOLD_SECTIONS: [(ScopeId, &str); 4] = [
    ("chart", "Chart"),
    ("navigate", "Navigate"),
    ("centre_z0", "Centre z₀"),
    ("slice_tilt", "Slice & tilt"),
];

/// Joins Explore's big scopes after the top bar, in §G3's Tab order: 2 Manifold view, with its sub-scopes, 3 Figure,
/// 4 Trajectory, 5 Compass, 6 Time, 7 Legend.
pub fn register(tree: &mut ScopeTree) {
    tree.register(None, Scope::group("manifold_view", "Manifold view"));
    for (id, label) in MANIFOLD_SECTIONS {
        tree.register(Some("manifold_view"), Scope::group(id, label));
    }
    tree.register(None, Scope::group("figure", "Figure").ring_outside());
    for (id, label) in [
        ("trajectory", "Trajectory"),
        ("compass", "Compass"),
        ("time", "Time"),
        ("legend", "Legend"),
    ] {
        tree.register(None, Scope::group(id, label));
    }
}

/// The height the Manifold view's title takes above its sections, in points.
const TITLE_H: f32 = 40.0;
/// The gap between the sections, in points.
const SECTION_GAP: f32 = 8.0;

/// The Manifold view's sections' rects in `manifold_view`: the panel below its title, split evenly, top to bottom,
/// until TASK-M6-26 draws the sections themselves.
pub fn manifold_sections(manifold_view: Rect) -> [Rect; 4] {
    let body = manifold_view.shrink(12.0);
    let top = body.min.y + TITLE_H;
    let h = (body.max.y - top - 3.0 * SECTION_GAP) / 4.0;
    std::array::from_fn(|i| {
        let y = top + i as f32 * (h + SECTION_GAP);
        Rect::from_min_size(egui::pos2(body.min.x, y), vec2(body.width(), h))
    })
}

/// Places Explore's scopes for the keyboard: each region's rect, and the Manifold view's sections.
pub fn place(keyboard: &mut Keyboard, layout: &Layout) {
    keyboard.place("manifold_view", layout.manifold_view);
    for ((id, _), rect) in MANIFOLD_SECTIONS
        .iter()
        .zip(manifold_sections(layout.manifold_view))
    {
        keyboard.place(id, rect);
    }
    for (id, rect) in [
        ("figure", layout.figure),
        ("trajectory", layout.trajectory),
        ("compass", layout.compass),
        ("time", layout.time),
        ("legend", layout.legend),
    ] {
        keyboard.place(id, rect);
    }
}

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
