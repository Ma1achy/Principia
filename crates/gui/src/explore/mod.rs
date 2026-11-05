//! The Explore page (render_gui_spec §G2; 01_main.png): the top bar with Overlays ▾ (`top_bar`, `overlays_menu`),
//! the footer, and the regions — the left Manifold view (`manifold_view`), the right Trajectory panel (`trajectory`),
//! and the bottom row's compass (`compass`), Time (`time`) and Legend (`legend`). The figure's rect is left to the
//! figure: nothing is drawn over it but its own marks, the lock's reticle (`lock`) and the hover label (`hover_label`)
//! (§G1), and its axis labels sit beside it (`axis_labels`). Each region is a big scope of the keyboard's tree (§G3);
//! the Manifold view's four sub-scopes, the compass with its Tilt and Slice buttons and its view buttons, Trajectory's
//! and Time's elements are reached with Enter, and the Legend is read-only.

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

use eframe::egui::Rect;

use crate::keyboard::scopes::{Scope, ScopeId, ScopeTree};
use crate::keyboard::Keyboard;
use crate::layout::Layout;

/// The Manifold view's sub-scopes, in order (§G3): Chart, Navigate, Slice centre, Tilt (named Centre z₀ and Slice &
/// tilt before R-457).
pub const MANIFOLD_SECTIONS: [(ScopeId, &str); 4] = [
    (manifold_view::CHART, manifold_view::SECTION_TITLES[0]),
    (manifold_view::NAVIGATE, manifold_view::SECTION_TITLES[1]),
    (
        manifold_view::SLICE_CENTRE,
        manifold_view::SECTION_TITLES[2],
    ),
    (manifold_view::TILT, manifold_view::SECTION_TITLES[3]),
];

/// The Manifold view's scope.
pub const MANIFOLD_VIEW: ScopeId = "manifold_view";
/// The figure's scope: in interaction mode its arrows pan (§G3).
pub const FIGURE: ScopeId = "figure";
/// The Compass's big scope: the compass, then its Tilt and Slice buttons, then its view buttons Cube, Plücker, Jacobi
/// and Shape, then, in the Cube view, the axis buttons H, V and D and, while it is shown, the badge (§G3; R-409 A2;
/// R-452; R-459 G9).
pub const COMPASS: ScopeId = "compass";
/// The compass itself, one element in navigation mode: in interaction mode its arrows tilt in tilt mode and move the
/// slice along the plane's normal in slice mode (§G3; R-409 F3).
pub const COMPASS_VIEW: ScopeId = "compass_view";
/// The compass's Tilt button.
pub const COMPASS_TILT: ScopeId = "compass_tilt";
/// The compass's Slice button.
pub const COMPASS_SLICE: ScopeId = "compass_slice";
/// The compass's Cube view button (R-452).
pub const COMPASS_CUBE: ScopeId = "compass_cube";
/// The compass's Plücker view button (R-452).
pub const COMPASS_PLUCKER: ScopeId = "compass_plucker";
/// The compass's Jacobi view button (R-452).
pub const COMPASS_JACOBI: ScopeId = "compass_jacobi";
/// The compass's Shape view button (R-452).
pub const COMPASS_SHAPE: ScopeId = "compass_shape";
/// The cube's H axis button, a menu of [`COMPASS_H_ENTRIES`] (R-459).
pub const COMPASS_H: ScopeId = "compass_h";
/// The cube's V axis button, a menu of [`COMPASS_V_ENTRIES`] (R-459).
pub const COMPASS_V: ScopeId = "compass_v";
/// The cube's third axis button, D, or X or Y while it shows a tilt target, a menu of [`COMPASS_D_ENTRIES`] (R-459).
pub const COMPASS_D: ScopeId = "compass_d";
/// The cube's "4D: see Plücker" badge, a sibling only while it is shown (R-459 G8, G9).
pub const COMPASS_BADGE: ScopeId = "compass_badge";
/// H's dropdown's entries, one per latent dimension; it holds those [`compass::choices`] offers.
pub const COMPASS_H_ENTRIES: [ScopeId; 8] = [
    "compass_h_0",
    "compass_h_1",
    "compass_h_2",
    "compass_h_3",
    "compass_h_4",
    "compass_h_5",
    "compass_h_6",
    "compass_h_7",
];
/// V's dropdown's entries.
pub const COMPASS_V_ENTRIES: [ScopeId; 8] = [
    "compass_v_0",
    "compass_v_1",
    "compass_v_2",
    "compass_v_3",
    "compass_v_4",
    "compass_v_5",
    "compass_v_6",
    "compass_v_7",
];
/// The third axis's dropdown's entries.
pub const COMPASS_D_ENTRIES: [ScopeId; 8] = [
    "compass_d_0",
    "compass_d_1",
    "compass_d_2",
    "compass_d_3",
    "compass_d_4",
    "compass_d_5",
    "compass_d_6",
    "compass_d_7",
];

/// Joins Explore's big scopes after the top bar, in §G3's Tab order: 2 Manifold view, with its sub-scopes, 3 Figure,
/// 4 Trajectory, with its elements, 5 Compass, with the compass, Tilt, Slice, Cube, Plücker, Jacobi and Shape inside
/// (R-452), H, V, D and the badge joining as the view shows them ([`compass::join`]; R-459), 6 Time, with its elements, 7 Legend, read-only. The figure and the compass are
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
    for (_, id, label) in compass::views::VIEWS {
        tree.register(Some(COMPASS), Scope::control(id, label));
    }
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
