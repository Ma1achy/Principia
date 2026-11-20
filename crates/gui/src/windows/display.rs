//! Display, the last stages (render_gui_spec §G5 "Display — the last stages"; GUI_DESIGN_NOTES § "04 Windows";
//! 04_windows.png; R-67, R-111, R-123, R-130): the fixed order SimState → stain → style → display scale → gamut clamp →
//! colour-vision simulation → screen, shown as a chain; the style (plain, watercolour & pencil, the seven print
//! presets), optional and on the figure only, scientific checks running with plain; the colour-vision simulation (off,
//! deuteranopia, protanopia, tritanopia, achromatopsia); and the display overlays: grid, class edges, `t_end`
//! contours, cursor crosshair.
//!
//! On the mock (applied per R-369): grid, class edges and `t_end` contours are `RenderState`'s overlay set, each toggle
//! one `SetField` of the whole set, as Overlays ▾'s (gui_state_contract §2); the style, the simulation and the crosshair
//! are the GUI's own choices here, which TASK-M8-23 and TASK-M7-25 place in the contract and draw.

use eframe::egui::{Button, Checkbox, RichText, Ui};
use engine::contract::render_state::Overlays;
use engine::contract::set_field::{Edit, RenderField, SetField};

use crate::keyboard::scopes::{Scope, ScopeId, ScopeTree};
use crate::windows::{note, section, Keys};

/// The window's scope.
pub const WINDOW: ScopeId = "win_display";
/// Its title, as 04_windows.png's.
pub const TITLE: &str = "Display · the last stages";

/// The fixed order (R-67, R-111), stage by stage.
pub const ORDER: [&str; 7] = [
    "SimState",
    "stain",
    "style",
    "display scale",
    "gamut clamp",
    "colour-vision simulation",
    "screen",
];
/// The order's caption (04_windows.png, with §G5's wording).
pub const ORDER_NOTE: &str =
    "order is fixed: the stain colours the data, the style draws it, the display scale and \
gamut clamp finish it, and colour-vision simulation shows how the finished picture is seen";

/// The Style section.
pub const STYLE: ScopeId = "dp_style";
/// The colour-vision simulation's section.
pub const CVD: ScopeId = "dp_cvd";
/// The overlays' section.
pub const OVERLAYS: ScopeId = "dp_overlays";

/// The styles, in order (R-130): plain, watercolour & pencil, and the seven print presets, as the poster names them.
pub const STYLES: [(ScopeId, &str); 9] = [
    ("dp_plain", "plain"),
    ("dp_watercolour", "watercolour & pencil"),
    ("dp_print_default", "print · Default"),
    ("dp_print_drops", "print · Drops"),
    ("dp_print_newspaper", "print · Newspaper"),
    ("dp_print_vintage", "print · Vintage"),
    ("dp_print_poster78", "print · Poster78"),
    ("dp_print_riso", "print · Riso"),
    ("dp_print_duotone", "print · Duotone"),
];
/// The paper grain, under watercolour & pencil (§G5).
pub const PAPER: ScopeId = "dp_paper";
/// The print's per-plate misregistration (§G5).
pub const MISREGISTRATION: ScopeId = "dp_misregistration";
/// The style's caption (§G5).
pub const STYLE_NOTE: &str =
    "style is optional and applies to the figure only — scientific checks run with plain";

/// The colour-vision simulations, in order (R-123).
pub const SIMULATIONS: [(ScopeId, &str); 5] = [
    ("dp_cvd_off", "off"),
    ("dp_deuteranopia", "deuteranopia"),
    ("dp_protanopia", "protanopia"),
    ("dp_tritanopia", "tritanopia"),
    ("dp_achromatopsia", "achromatopsia"),
];

/// The display overlays, in order (§G5).
pub const OVERLAY_IDS: [(ScopeId, &str); 4] = [
    ("dp_grid", "grid"),
    ("dp_class_edges", "class edges"),
    ("dp_t_end", "t_end contours"),
    ("dp_crosshair", "cursor crosshair"),
];

/// Display's own choices: the GUI's (applied per R-369, as the module says).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct State {
    /// The style, by its index in [`STYLES`]: plain by default.
    pub style: usize,
    /// The paper grain.
    pub paper: bool,
    /// The per-plate misregistration.
    pub misregistration: bool,
    /// The colour-vision simulation, by its index in [`SIMULATIONS`]: off by default.
    pub simulation: usize,
    /// The cursor crosshair.
    pub crosshair: bool,
}

/// Joins the window's sections and elements to `tree`, in order.
pub fn register(tree: &mut ScopeTree) {
    tree.register(Some(WINDOW), Scope::group(STYLE, "Style"));
    for (id, label) in STYLES {
        tree.register(Some(STYLE), Scope::control(id, label));
    }
    tree.register(Some(STYLE), Scope::control(PAPER, "paper grain"));
    tree.register(
        Some(STYLE),
        Scope::control(MISREGISTRATION, "press misregistration"),
    );
    tree.register(Some(WINDOW), Scope::group(CVD, "Colour-vision simulation"));
    for (id, label) in SIMULATIONS {
        tree.register(Some(CVD), Scope::control(id, label));
    }
    tree.register(Some(WINDOW), Scope::group(OVERLAYS, "Overlays"));
    for (id, label) in OVERLAY_IDS {
        tree.register(Some(OVERLAYS), Scope::control(id, label));
    }
}

/// `overlays` with display overlay `id` toggled: grid, class edges or `t_end` contours; `None` for any other id.
pub fn toggled(id: &str, overlays: &Overlays) -> Option<Overlays> {
    let mut next = overlays.clone();
    match id {
        "dp_grid" => next.grid = !next.grid,
        "dp_class_edges" => next.class_edges = !next.class_edges,
        "dp_t_end" => next.t_end_contours = !next.t_end_contours,
        _ => return None,
    }
    Some(next)
}

/// Enter on control `id`, as its click: a style or a simulation is chosen, a switch toggles; a contract overlay's
/// toggle is one `SetField` of the whole set, pushed to `edits`.
pub fn activate(id: &str, state: &mut State, overlays: &Overlays, edits: &mut Vec<SetField>) {
    if let Some(i) = STYLES.iter().position(|(s, _)| *s == id) {
        state.style = i;
    } else if let Some(i) = SIMULATIONS.iter().position(|(s, _)| *s == id) {
        state.simulation = i;
    } else if let Some(next) = toggled(id, overlays) {
        edits.push(SetField {
            edit: Edit::Render(RenderField::Overlays(next)),
            no_history: false,
        });
    } else {
        match id {
            PAPER => state.paper = !state.paper,
            MISREGISTRATION => state.misregistration = !state.misregistration,
            "dp_crosshair" => state.crosshair = !state.crosshair,
            _ => {}
        }
    }
}

/// Draws the window's content from the snapshot's `overlays`, its clicks acting as Enter does.
pub fn show(
    ui: &mut Ui,
    keys: &mut Keys<'_>,
    state: &mut State,
    overlays: &Overlays,
    edits: &mut Vec<SetField>,
) {
    note(ui, ORDER_NOTE);
    ui.horizontal_wrapped(|ui| {
        for (i, stage) in ORDER.iter().enumerate() {
            if i > 0 {
                ui.label("→");
            }
            ui.label(RichText::new(*stage).monospace());
        }
    });
    ui.add_space(8.0);
    let mut clicked = Vec::new();
    section(ui, keys, STYLE, "Style", |ui, keys| {
        ui.horizontal_wrapped(|ui| {
            for (i, (id, label)) in STYLES.iter().enumerate() {
                let button = ui.add(Button::selectable(state.style == i, *label));
                if button.clicked() {
                    clicked.push(*id);
                }
                keys.place(id, &button);
            }
        });
        ui.horizontal(|ui| {
            let mut paper = state.paper;
            let grain = ui.add(Checkbox::new(&mut paper, "paper grain"));
            let mut slip = state.misregistration;
            let press = ui.add(Checkbox::new(&mut slip, "press misregistration"));
            for (id, response) in [(PAPER, &grain), (MISREGISTRATION, &press)] {
                if response.clicked() {
                    clicked.push(id);
                }
                keys.place(id, response);
            }
        });
        note(ui, STYLE_NOTE);
    });
    section(ui, keys, CVD, "Colour-vision simulation", |ui, keys| {
        ui.horizontal_wrapped(|ui| {
            for (i, (id, label)) in SIMULATIONS.iter().enumerate() {
                let button = ui.add(Button::selectable(state.simulation == i, *label));
                if button.clicked() {
                    clicked.push(*id);
                }
                keys.place(id, &button);
            }
        });
    });
    section(ui, keys, OVERLAYS, "Overlays", |ui, keys| {
        ui.horizontal_wrapped(|ui| {
            let on = [
                overlays.grid,
                overlays.class_edges,
                overlays.t_end_contours,
                state.crosshair,
            ];
            for ((id, label), mut on) in OVERLAY_IDS.into_iter().zip(on) {
                let check = ui.add(Checkbox::new(&mut on, label));
                if check.clicked() {
                    clicked.push(id);
                }
                keys.place(id, &check);
            }
        });
    });
    for id in clicked {
        activate(id, state, overlays, edits);
    }
}
