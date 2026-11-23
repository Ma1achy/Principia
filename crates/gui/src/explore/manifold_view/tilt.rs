//! Tilt (render_gui_spec §G2; named "Slice & tilt" before R-457): a "reset" beside its heading (`super::reset`; R-466),
//! "tilt `τ₁` into \[X\]" and "tilt `τ₂` into \[Y\]",
//! each a slider with its number box and, under it, the dropdown of the dimension it tilts into, then the rotation `γ`.
//! Each angle is one `SetField` on the basis, which turns about the centre, and, locked, about the pin: the app turns
//! every basis edit about it ([`super::about_the_pin`], §G4). The tilts span the map's range ([`tilt_range`]), the
//! rotation `[−180°, 180°]`. A dropdown offers the six dimensions not in the plane, by their
//! names; picking one is one `SetField` on the slice frame, carrying the basis re-tilted into it where its angle is not
//! 0 ([`super::target_edit`]; R-457 E3, E5). There is no step slider: the move along the slice normal is the Slice
//! centre slider of that dimension (R-457). Touching an angle or a target dropdown switches the compass to Tilt, its
//! buttons following (R-409), on the drag's first frame: the slider or its box pressed, the box clicked into to type,
//! the dropdown pressed, whether or not the value changes, so a press on a thumb at ±90° counts (R-458 F4). A drag
//! applies its value every frame it changes, the whole drag one history entry (R-458 F1). While `τ₁` or `τ₂` is held the
//! compass's third axis shows its target (R-459).
//!
//! The last row is the tilt map, "map: Tip as sheet / Lean each axis", two radio options: under Tip as sheet,
//! the default, the tilts span `[−180°, 180°]` and nothing clamps the tip; under Lean each axis `[−90°, 90°]`. The
//! angles are stored as set beside the map, and the sliders read them so ([`Angles::read`]).

use eframe::egui::{self, ComboBox, DragValue, Rect, Slider, Ui};
use engine::contract::set_field::SetField;
use engine::contract::sim_config::{Plane, SimConfig, SliceFrame, TiltMap};

use crate::explore::compass::Gesture;

use crate::dispatch::{self, Class};

use super::{
    close_under_console, frame_edit, reset::Section, row, row_ui, target_choices, target_edit,
    Panel, Target, Touch, View, ANGLE_READS_BACK, AXES, GAMMA, TAU1, TAU2, TILT_MAP_LEAN,
    TILT_MAP_SHEET, TILT_X, TILT_X_ENTRIES, TILT_Y, TILT_Y_ENTRIES,
};

/// The tilts' span under Tip as sheet, the default, in degrees: nothing clamps the tip, each angle running over
/// `[−180°, 180°]`.
pub const TILT_RANGE: (f64, f64) = (-180.0, 180.0);
/// The tilts' span under Lean each axis, in degrees: its ±90° clamp.
pub const LEAN_RANGE: (f64, f64) = (-90.0, 90.0);

/// The tilts' span under `map`.
pub fn tilt_range(map: TiltMap) -> (f64, f64) {
    match map {
        TiltMap::Sheet => TILT_RANGE,
        TiltMap::Lean => LEAN_RANGE,
    }
}
/// The rotation's span, in degrees.
pub const ROTATION_RANGE: (f64, f64) = (-180.0, 180.0);

/// The three angles, in degrees.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Angles {
    /// `τ₁`.
    pub tau1: f64,
    /// `τ₂`.
    pub tau2: f64,
    /// `γ`.
    pub gamma: f64,
}

impl Angles {
    /// The angles of `view`.
    pub fn of(view: &View) -> Self {
        Self {
            tau1: view.tau1.to_degrees(),
            tau2: view.tau2.to_degrees(),
            gamma: view.gamma.to_degrees(),
        }
    }

    /// The angles of `sim`'s chart: `τ₁` and `τ₂` as stored, bit for bit, where they stand ([`View::read`]), else
    /// as read off the basis; `γ` as read.
    pub fn read(sim: &SimConfig) -> Self {
        Self::read_framed(&sim.plane, &sim.slice.frame)
    }

    /// [`Angles::read`] of `plane` read with the slice frame `frame`.
    pub fn read_framed(plane: &Plane, frame: &SliceFrame) -> Self {
        let view = View::framed(plane, Some(frame));
        let mut angles = Self::of(&view);
        let near = |rad: f64, deg: f64| (rad.to_degrees() - deg).abs() <= ANGLE_READS_BACK;
        if near(view.tau1, frame.tau1) && near(view.tau2, frame.tau2) {
            angles.tau1 = frame.tau1;
            angles.tau2 = frame.tau2;
        }
        angles
    }

    /// Turns the angle of scope `id` by `degrees`, within its span under `map`.
    pub fn turn(&mut self, id: &str, degrees: f64, map: TiltMap) {
        let (angle, (lo, hi)) = match id {
            TAU1 => (&mut self.tau1, tilt_range(map)),
            TAU2 => (&mut self.tau2, tilt_range(map)),
            _ => (&mut self.gamma, ROTATION_RANGE),
        };
        *angle = (*angle + degrees).clamp(lo, hi);
    }

    /// These angles with `τ₁` and `τ₂` within their span under `map`, each sign kept: Lean each axis's ±90°.
    pub fn within(self, map: TiltMap) -> Self {
        let (lo, hi) = tilt_range(map);
        Self {
            tau1: self.tau1.clamp(lo, hi),
            tau2: self.tau2.clamp(lo, hi),
            ..self
        }
    }

    /// `view` with these angles.
    pub fn apply(&self, view: View) -> View {
        View {
            tau1: self.tau1.to_radians(),
            tau2: self.tau2.to_radians(),
            gamma: self.gamma.to_radians(),
            ..view
        }
    }

    /// The compass's readout: "τ₁ 23.4° · τ₂ 0.0° · γ 30.3°".
    pub fn readout(&self) -> String {
        format!(
            "τ₁ {:.1}° · τ₂ {:.1}° · γ {:.1}°",
            self.tau1, self.tau2, self.gamma
        )
    }
}

/// The width of a target dropdown, in points (a look choice, R-390): the longest name, "z_μ1", and its arrow.
pub const TARGET_W: f32 = 64.0;

/// Draws Tilt in `section`: `τ₁` and its target, `τ₂` and its target, then `γ`, a row each.
pub fn show(ui: &mut Ui, section: Rect, h: f32, clip: Rect, panel: &mut Panel<'_>) {
    let view = View::read(&panel.snapshot.sim);
    super::header_with_reset(ui, section, h, clip, "", Section::Tilt, panel);
    let angles = Angles::read(&panel.snapshot.sim);
    let range = tilt_range(view.map);
    panel.scratch.compass.slider = None;
    for (i, (id, label, value, (lo, hi))) in [
        (TAU1, "τ₁", angles.tau1, range),
        (TAU2, "τ₂", angles.tau2, range),
        (GAMMA, "γ", angles.gamma, ROTATION_RANGE),
    ]
    .into_iter()
    .enumerate()
    {
        let rect = row(section, h, 1 + 2 * i);
        let mut line = row_ui(ui, rect);
        line.add_sized([40.0, rect.height()], egui::Label::new(label));
        let gap = line.spacing().item_spacing.x;
        line.spacing_mut().slider_width =
            (line.available_width() - super::centre::VALUE_W - gap).max(40.0);
        let mut moved = value;
        let slider = line.add(Slider::new(&mut moved, lo..=hi).show_value(false));
        let field = line.add_sized(
            [super::centre::VALUE_W, rect.height() - 2.0],
            DragValue::new(&mut moved)
                .speed(0.1)
                .range(lo..=hi)
                .suffix("°")
                .max_decimals(1),
        );
        let changed = slider.changed() || field.changed();
        if super::centre::touched(&slider, &field) {
            panel.scratch.compass.mode = Touch::Tilt;
        }
        if slider.is_pointer_button_down_on() || field.is_pointer_button_down_on() {
            panel.scratch.compass.slider = match id {
                TAU1 => Some(Gesture::Tau1),
                TAU2 => Some(Gesture::Tau2),
                _ => None,
            };
        }
        panel.place(id, &slider.union(field), clip);
        if changed {
            let mut to = angles;
            to.turn(id, moved - value, view.map);
            let plane = &panel.snapshot.sim.plane;
            let edit = dispatch::one(plane, Class::Tilt { from: view, to });
            panel.out.edit(edit, Some(Touch::Tilt), panel.scratch);
        }
    }
    for (i, (which, id, entries, label, now)) in [
        (Target::X, TILT_X, TILT_X_ENTRIES, "tilt τ₁ into", view.x),
        (Target::Y, TILT_Y, TILT_Y_ENTRIES, "tilt τ₂ into", view.y),
    ]
    .into_iter()
    .enumerate()
    {
        let rect = row(section, h, 2 + 2 * i);
        let mut line = row_ui(ui, rect);
        line.add_space(40.0 + line.spacing().item_spacing.x);
        line.label(label);
        let mut picked = now;
        let combo = ComboBox::from_id_salt(id)
            .selected_text(AXES[now])
            .width(TARGET_W)
            .show_ui(&mut line, |ui| {
                for to in target_choices(view.a, view.b) {
                    let entry = ui.selectable_value(&mut picked, to, AXES[to]);
                    panel.keyboard.place_widget(entries[to], &entry);
                }
            });
        close_under_console(&combo.response, panel.scratch.console_top);
        panel.place(id, &combo.response, clip);
        if combo.response.is_pointer_button_down_on() || combo.response.clicked() {
            panel.scratch.compass.mode = Touch::Tilt;
        }
        if let Some(edit) = target_edit(&view, which, picked) {
            panel.out.edit(edit, Some(Touch::Tilt), panel.scratch);
        }
    }
    let rect = row(section, h, 6);
    let mut line = row_ui(ui, rect);
    line.add_sized([40.0, rect.height()], egui::Label::new(MAP_LABEL));
    let mut picked = view.map;
    for (id, map) in [
        (TILT_MAP_SHEET, TiltMap::Sheet),
        (TILT_MAP_LEAN, TiltMap::Lean),
    ] {
        let radio = line
            .radio_value(&mut picked, map, map.name())
            .on_hover_text(map_tooltip(map));
        panel.place(id, &radio, clip);
    }
    if let Some(edit) = map_edit(&panel.snapshot.sim, picked) {
        panel.out.edit(edit, Some(Touch::Tilt), panel.scratch);
    }
}

/// The map selector's label.
pub const MAP_LABEL: &str = "map";

/// What each map does, the selector's tooltip.
pub fn map_tooltip(map: TiltMap) -> &'static str {
    match map {
        TiltMap::Sheet => {
            "Tip as sheet: the plane turns rigidly by √(τ₁² + τ₂²) about the in-plane axis across (τ₁, τ₂); nothing clamps it"
        }
        TiltMap::Lean => {
            "Lean each axis: each screen axis a mix of its own coordinate and its target; each angle within ±90°"
        }
    }
}

/// The choice of `map` on `sim`'s chart: one edit of the slice frame, carrying the basis where the maps
/// differ on it (one target, both angles set) or an angle past ±90° is clamped to Lean each axis's range, sign kept, in
/// the same history entry, so the views that re-integrate are exactly those (R10, C10); the frame alone elsewhere.
/// `None` for the map it already is.
pub fn map_edit(sim: &SimConfig, map: TiltMap) -> Option<SetField> {
    let frame = sim.slice.frame;
    if frame.tilt_map == map {
        return None;
    }
    let view = View::read(sim);
    let at = Angles::read(sim);
    let to = at.within(map);
    let next = View {
        map,
        tau1: to.tau1.to_radians(),
        tau2: to.tau2.to_radians(),
        ..view
    };
    let frame = SliceFrame {
        tilt_map: map,
        tau1: to.tau1,
        tau2: to.tau2,
        ..frame
    };
    if !(view.sheet() || next.sheet() || to != at) {
        return Some(frame_edit(frame, None));
    }
    let edit = crate::dispatch::one(
        &sim.plane,
        crate::dispatch::Class::Tilt {
            from: View { map, ..view },
            to,
        },
    );
    match edit.edit {
        engine::contract::set_field::Edit::Sim(
            engine::contract::set_field::SimField::SliceFrame {
                basis: Some(basis), ..
            },
        ) => Some(frame_edit(frame, Some(basis))),
        // The leaf refused it (an input not finite): the map alone, the basis as it is.
        _ => Some(frame_edit(frame, None)),
    }
}
