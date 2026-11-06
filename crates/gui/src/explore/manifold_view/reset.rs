//! The resets and Home (render_gui_spec §G2, R-466): each of Slice centre, Tilt and Navigate has a "reset" at the right
//! of its heading, the last element of its sub-scope, returning that section, and only it, to the active chart preset's
//! own values ([`PRESET_HOMES`], REQ-GUI-224), never literal zero; Home does all three in one command, from the house
//! over the figure's top-right corner ([`home_button`]), the Home key and View › Home.
//!
//! Each section's values are read in the preset's coordinates, its axes `e_a`, `e_b` and the six dimensions off them,
//! tilted or not (K2): Slice centre's are `z₀`'s six coordinates off the axes; Navigate's are `z₀`'s two on them and the
//! scale `σ`; Tilt's are `τ₁`, `τ₂`, `γ`. They are disjoint, so each reset leaves the other two sections as they were
//! and the three in any order compose to Home. With no active preset (K3) everything is dimmed and does nothing.
//!
//! A command is its edits under one fresh history key ([`Out::fresh`](super::Out::fresh); K10), each through the lock's
//! rule for its kind ([`about_the_pin`]; K5). Navigate moves `z₀` before it scales, so its zoom turns about the pin as
//! every zoom does; Home moves `z₀` last, so it lands on the home view exactly, locked or not. A reset that is dimmed
//! sends nothing, and none switches the compass's mode (K12): the compass reads the new state in the next frame.

use eframe::egui::{self, pos2, vec2, Area, Button, Color32, Id, Order, Rect, Shape, Stroke, Ui};
use engine::contract::set_field::SetField;
use engine::contract::sim_config::{SimConfig, LATENT_DIM};

use super::{
    about_the_pin, basis_edit, z0_edit, PresetHome, View, PRESET_HOMES, RESET_NAVIGATE,
    RESET_SLICE_CENTRE, RESET_TILT, SECTION_TITLES,
};
use crate::keyboard::scopes::ScopeId;

/// How near a value is to its home value and still at it (K11): radians for the angles, octaves for the zoom.
pub const HOME_TOLERANCE: f64 = 1e-12;
/// What a section's reset reads (K6).
pub const RESET: &str = "reset";
/// The tooltip of a reset or Home dimmed at home (K11).
pub const ALREADY_HOME: &str = "already at the chart preset's values";
/// The tooltip of a reset or Home with no active preset (K3).
pub const NO_PRESET: &str = "no chart preset";
/// The Home button's name and tooltip (K7).
pub const HOME_NAME: &str = "Home";
/// The Home button's tooltip (K7).
pub const HOME_TOOLTIP: &str = "Home: reset Slice centre, Tilt and Navigate (Home)";
/// The Home button's side, in points (K7).
pub const HOME_SIDE: f32 = 24.0;
/// The Home button's inset from the canvas's corner, in points (K7).
pub const HOME_INSET: f32 = 8.0;

/// A section with a reset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    /// `z₀`'s six coordinates off the preset's axes.
    SliceCentre,
    /// `τ₁`, `τ₂` and `γ`.
    Tilt,
    /// `z₀`'s two coordinates on the preset's axes, and the scale.
    Navigate,
}

/// The three sections, as Home does them.
pub const SECTIONS: [Section; 3] = [Section::SliceCentre, Section::Tilt, Section::Navigate];

impl Section {
    /// The section whose reset is scope `id`.
    pub fn of(id: &str) -> Option<Self> {
        SECTIONS.into_iter().find(|s| s.scope() == id)
    }

    /// Its reset's scope.
    pub fn scope(self) -> ScopeId {
        match self {
            Section::SliceCentre => RESET_SLICE_CENTRE,
            Section::Tilt => RESET_TILT,
            Section::Navigate => RESET_NAVIGATE,
        }
    }

    /// Its heading.
    pub fn title(self) -> &'static str {
        match self {
            Section::SliceCentre => SECTION_TITLES[2],
            Section::Tilt => SECTION_TITLES[3],
            Section::Navigate => SECTION_TITLES[1],
        }
    }

    /// Its reset's tooltip, the command's name and its values (K6, K9).
    pub fn tooltip(self) -> &'static str {
        match self {
            Section::SliceCentre => "Reset Slice centre to the chart preset's base point",
            Section::Tilt => "Reset Tilt: τ₁, τ₂ and γ to 0",
            Section::Navigate => "Reset Navigate to the chart preset's centre and window",
        }
    }
}

/// Whether `value` is at `home` (K11).
pub fn at(value: f64, home: f64) -> bool {
    (value - home).abs() <= HOME_TOLERANCE
}

/// The chart read in its active preset's coordinates.
struct Active {
    view: View,
    home: PresetHome,
}

impl Active {
    /// `sim`'s chart, if a preset holds it (K3).
    fn of(sim: &SimConfig) -> Option<Self> {
        let view = View::read(sim);
        let home = PRESET_HOMES[view.preset?];
        Some(Self { view, home })
    }

    /// `view` with its angles and scale replaced: the basis of a reset.
    fn basis(&self, scale: f64, angles: (f64, f64, f64)) -> (super::Latent, super::Latent) {
        let (tau1, tau2, gamma) = angles;
        View {
            scale,
            tau1,
            tau2,
            gamma,
            ..self.view
        }
        .basis()
    }

    /// Whether `section`'s values are at their home values.
    fn at_home(&self, sim: &SimConfig, section: Section) -> bool {
        let View { a, b, .. } = self.view;
        let z0 = &sim.plane.z0;
        match section {
            Section::SliceCentre => (0..LATENT_DIM)
                .filter(|i| *i != a && *i != b)
                .all(|i| at(z0[i], self.home.base[i])),
            Section::Tilt => [self.view.tau1, self.view.tau2, self.view.gamma]
                .into_iter()
                .all(|angle| at(angle, 0.0)),
            Section::Navigate => {
                let (u, v) = self.home.centre(a, b);
                at(z0[a], u) && at(z0[b], v) && at(self.view.zoom_row(), 0.0)
            }
        }
    }

    /// The edits of `section`'s reset, before the lock's rule.
    fn edits(&self, sim: &SimConfig, section: Section) -> Vec<SetField> {
        let View { a, b, .. } = self.view;
        let mut z0 = sim.plane.z0;
        match section {
            Section::SliceCentre => {
                for i in (0..LATENT_DIM).filter(|i| *i != a && *i != b) {
                    z0[i] = self.home.base[i];
                }
                vec![z0_edit(z0)]
            }
            Section::Tilt => vec![basis_edit(self.basis(self.view.scale, (0.0, 0.0, 0.0)))],
            Section::Navigate => {
                (z0[a], z0[b]) = self.home.centre(a, b);
                let angles = (self.view.tau1, self.view.tau2, self.view.gamma);
                vec![
                    z0_edit(z0),
                    basis_edit(self.basis(self.home.window, angles)),
                ]
            }
        }
    }
}

/// Whether `section`'s reset is live: a preset is active and the section is off its home values.
pub fn enabled(sim: &SimConfig, section: Section) -> bool {
    Active::of(sim).is_some_and(|active| !active.at_home(sim, section))
}

/// Whether Home is live: any section's reset is.
pub fn home_enabled(sim: &SimConfig) -> bool {
    SECTIONS.into_iter().any(|section| enabled(sim, section))
}

/// Why a reset or Home on `sim` is dimmed, for its tooltip.
pub fn dimmed_because(sim: &SimConfig) -> &'static str {
    if View::read(sim).preset.is_some() {
        ALREADY_HOME
    } else {
        NO_PRESET
    }
}

/// `section`'s reset on `sim`, through the lock's rule: nothing while it is dimmed.
pub fn command(sim: &SimConfig, section: Section) -> Vec<SetField> {
    match Active::of(sim) {
        Some(active) if !active.at_home(sim, section) => {
            about_the_pin(active.edits(sim, section), sim)
        }
        _ => Vec::new(),
    }
}

/// Home on `sim`, through the lock's rule, its edit of `z₀` last, so it lands on the home view exactly (K5): nothing
/// while it is dimmed.
pub fn home(sim: &SimConfig) -> Vec<SetField> {
    match Active::of(sim).filter(|_| home_enabled(sim)) {
        Some(active) => {
            let basis = active.basis(active.home.window, (0.0, 0.0, 0.0));
            about_the_pin(vec![basis_edit(basis), z0_edit(active.home.base)], sim)
        }
        None => Vec::new(),
    }
}

/// Where the Home button goes on `shown`, the canvas as it shows: its top-right corner, inset (K7; §G2).
pub fn home_rect(shown: Rect) -> Rect {
    Rect::from_min_size(
        pos2(
            shown.max.x - HOME_INSET - HOME_SIDE,
            shown.min.y + HOME_INSET,
        ),
        vec2(HOME_SIDE, HOME_SIDE),
    )
}

/// The house in `rect`: a roof over a square, in egui's strokes, no font glyph (K7).
pub fn house(rect: Rect, colour: Color32) -> [Shape; 2] {
    let c = rect.center();
    let stroke = Stroke::new(1.5, colour);
    let at = |x: f32, y: f32| c + vec2(x, y);
    [
        Shape::line(vec![at(-7.0, 0.0), at(0.0, -7.0), at(7.0, 0.0)], stroke),
        Shape::line(
            vec![at(-5.0, -1.0), at(-5.0, 6.0), at(5.0, 6.0), at(5.0, -1.0)],
            stroke,
        ),
    ]
}

/// Draws the Home button over the figure, on `shown`, what shows of the canvas: on a layer above the figure's, which
/// takes the pointer, so a click on it, dimmed or not, is none on the figure (K7). Where the console reaches it, it is
/// not drawn: its layer is above the console's (R-463 K2; applied per R-369). Returns whether it was clicked.
pub fn home_button(ctx: &egui::Context, shown: Rect, sim: &SimConfig) -> bool {
    let rect = home_rect(shown);
    if !shown.contains_rect(rect) {
        return false;
    }
    let live = home_enabled(sim);
    Area::new(Id::new("home button"))
        .order(Order::Foreground)
        .fixed_pos(rect.min)
        .constrain(false)
        .fade_in(false)
        .show(ctx, |ui: &mut Ui| {
            // Takes the press a dimmed button leaves, so it never reaches the figure under it.
            let _ = ui.interact(rect, Id::new("home button cover"), egui::Sense::click());
            ui.add_enabled_ui(live, |ui| {
                ui.spacing_mut().button_padding = vec2(0.0, 0.0);
                let button = ui
                    .add(Button::new("").min_size(rect.size()))
                    .on_hover_text(HOME_TOOLTIP)
                    .on_disabled_hover_text(dimmed_because(sim));
                button.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, live, HOME_NAME)
                });
                let colour = ui.style().interact(&button).text_color();
                ui.painter().extend(house(button.rect, colour));
                button.clicked()
            })
            .inner
        })
        .inner
}
