//! The Explore page (render_gui_spec §G2; 01_main.png): the top bar with Overlays ▾ (`top_bar`, `overlays_menu`),
//! the footer, and the regions — the left Manifold view (`manifold_view`), the right Trajectory panel (`trajectory`),
//! and the bottom row's compass (`compass`), Time (`time`) and Legend (`legend`). The figure's rect is left to the
//! figure: nothing is drawn over it but its own marks, the lock's reticle (`lock`) and the hover label (`hover_label`)
//! (§G1), and its axis labels sit beside it (`axis_labels`). Each region is a big scope of the keyboard's tree (§G3);
//! the Manifold view's four sub-scopes, the compass with its Tilt and Slice buttons, Trajectory's and Time's elements
//! are reached with Enter, and the Legend is read-only.

pub mod axis_labels;
pub mod breadcrumb;
pub mod compass;
pub mod footer;
pub mod hover_label;
pub mod legend;
pub mod lock;
pub mod manifold_view;
pub mod overlays_menu;
pub mod time;
pub mod top_bar;
pub mod trajectory;

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
/// The figure's scope: in interaction mode its arrows pan (§G3).
pub const FIGURE: ScopeId = "figure";
/// The Compass's big scope: the compass, then its Tilt and Slice buttons (§G3; R-409 A2).
pub const COMPASS: ScopeId = "compass";
/// The compass itself, one element in navigation mode: in interaction mode its arrows tilt in tilt mode and move the
/// slice along the plane's normal in slice mode (§G3; R-409 F3).
pub const COMPASS_VIEW: ScopeId = "compass_view";
/// The compass's Tilt button.
pub const COMPASS_TILT: ScopeId = "compass_tilt";
/// The compass's Slice button.
pub const COMPASS_SLICE: ScopeId = "compass_slice";

/// Joins Explore's big scopes after the top bar, in §G3's Tab order: 2 Manifold view, with its sub-scopes, 3 Figure,
/// 4 Trajectory, with its elements, 5 Compass, with the compass, Tilt and Slice inside, 6 Time, with its elements,
/// 7 Legend, read-only. The figure and the compass are
/// values the arrows adjust in interaction mode: the figure's pan, the compass's tilt or slice (registered as the
/// tilt; [`compass::adjust`] steps by the mode). The Manifold view's controls join when it is first drawn
/// ([`manifold_view::join`]).
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
    trajectory::register(tree);
    tree.register(None, Scope::group(COMPASS, "Compass"));
    tree.register(
        Some(COMPASS),
        Scope::value(COMPASS_VIEW, "compass", StepKind::Tilt),
    );
    tree.register(
        Some(COMPASS),
        Scope::control(COMPASS_TILT, compass::TILT_BUTTON),
    );
    tree.register(
        Some(COMPASS),
        Scope::control(COMPASS_SLICE, compass::SLICE_BUTTON),
    );
    time::register(tree);
    legend::register(tree);
}

/// The Manifold view's sections' rects in `manifold_view`, where its four sub-scopes are drawn and ringed
/// ([`manifold_view::sections`]).
pub fn manifold_sections(manifold_view: Rect) -> [Rect; 4] {
    manifold_view::sections(manifold_view)
}

/// Places the figure for the keyboard; every region places its own as it draws.
pub fn place(keyboard: &mut Keyboard, layout: &Layout) {
    keyboard.place(FIGURE, layout.figure);
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

/// Draws the Stain page as an empty frame, which TASK-M6-29 fills (RQ-249).
pub fn stain_page(ui: &mut Ui, rect: Rect) {
    region(ui, "Stain", rect);
}
