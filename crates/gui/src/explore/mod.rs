//! The Explore page (render_gui_spec §G2; 01_main.png): the top bar, the footer, and the regions — the left Manifold
//! view (`manifold_view`), the right Trajectory panel, and the bottom row's compass (`compass`), Time and Legend, the
//! regions later track tasks fill drawn as empty frames. The figure's rect is left to the figure: nothing is drawn
//! over it but the lock's reticle (`lock`, §G1), and its axis labels sit beside it (`axis_labels`). Each region is a
//! big scope of the keyboard's tree (§G3), and the Manifold view's four sub-scopes are reached with Enter.

pub mod axis_labels;
pub mod breadcrumb;
pub mod compass;
pub mod footer;
pub mod lock;
pub mod manifold_view;
pub mod top_bar;

use eframe::egui::{self, Rect, RichText, Ui, UiBuilder};

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

/// The Manifold view's scope.
pub const MANIFOLD_VIEW: ScopeId = "manifold_view";
/// The figure's scope: its arrows pan (§G3).
pub const FIGURE: ScopeId = "figure";
/// The compass's scope: its arrows tilt, Shift+arrows orbit (§G3).
pub const COMPASS: ScopeId = "compass";

/// Joins Explore's big scopes after the top bar, in §G3's Tab order: 2 Manifold view, with its sub-scopes, 3 Figure,
/// 4 Trajectory, 5 Compass, 6 Time, 7 Legend. The figure and the compass are values the arrows adjust: the figure's
/// pan, the compass's tilt. The Manifold view's controls join when it is first drawn ([`manifold_view::join`]).
pub fn register(tree: &mut ScopeTree) {
    use crate::keyboard::scopes::StepKind;
    tree.register(None, Scope::group(MANIFOLD_VIEW, "Manifold view"));
    for (id, label) in MANIFOLD_SECTIONS {
        tree.register(Some(MANIFOLD_VIEW), Scope::group(id, label));
    }
    tree.register(
        None,
        Scope::value(FIGURE, "Figure", StepKind::Pan).ring_outside(),
    );
    tree.register(None, Scope::group("trajectory", "Trajectory"));
    tree.register(None, Scope::value(COMPASS, "Compass", StepKind::Tilt));
    for (id, label) in [("time", "Time"), ("legend", "Legend")] {
        tree.register(None, Scope::group(id, label));
    }
}

/// The Manifold view's sections' rects in `manifold_view`, where its four sub-scopes are drawn and ringed
/// ([`manifold_view::sections`]).
pub fn manifold_sections(manifold_view: Rect) -> [Rect; 4] {
    manifold_view::sections(manifold_view)
}

/// Places the regions later track tasks fill, and the figure, for the keyboard; the Manifold view and the compass
/// place their own as they draw.
pub fn place(keyboard: &mut Keyboard, layout: &Layout) {
    for (id, rect) in [
        (FIGURE, layout.figure),
        ("trajectory", layout.trajectory),
        ("time", layout.time),
        ("legend", layout.legend),
    ] {
        keyboard.place(id, rect);
    }
}

/// The titles of the regions drawn as empty frames, as 01_main.png gives them, with the rect each is drawn in.
pub fn region_titles(layout: &Layout) -> [(&'static str, Rect); 3] {
    [
        ("Trajectory", layout.trajectory),
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

/// Draws the regions later track tasks fill as empty frames.
pub fn regions(ui: &mut Ui, layout: &Layout) {
    for (title, rect) in region_titles(layout) {
        region(ui, title, rect);
    }
}

/// Draws the Stain page as an empty frame, which TASK-M6-29 fills (RQ-249).
pub fn stain_page(ui: &mut Ui, rect: Rect) {
    region(ui, "Stain", rect);
}
