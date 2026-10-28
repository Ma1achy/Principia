//! Tilt (render_gui_spec §G2; named "Slice & tilt" before R-457): "tilt `τ₁` into [X]" and "tilt `τ₂` into [Y]",
//! each a slider with its number box and, under it, the dropdown of the dimension it tilts into, then the rotation `γ`.
//! Each angle is one `SetField` on the basis, which turns about the centre, and, locked, about the pin: the app turns
//! every basis edit about it ([`super::about_the_pin`], §G4). The tilts span `[−90°, 90°]` (chart_decoder_contract
//! Part 4, "Tilt"), the rotation `[−180°, 180°]`. A dropdown offers the six dimensions not in the plane, by their
//! names; picking one is one `SetField` on the slice frame, carrying the basis re-tilted into it where its angle is not
//! 0 ([`super::target_edit`]; R-457 E3, E5). There is no step slider: the move along the slice normal is the Slice
//! centre slider of that dimension (R-457). Touching an angle switches the compass to Tilt, its buttons following
//! (R-409).

use eframe::egui::{self, ComboBox, DragValue, Rect, Slider, Ui};

use super::{
    basis_edit, header, row, row_ui, target_choices, target_edit, Panel, Target, Touch, View, AXES,
    GAMMA, SECTION_TITLES, TAU1, TAU2, TILT_X, TILT_X_ENTRIES, TILT_Y, TILT_Y_ENTRIES,
};

/// The tilts' span, in degrees.
pub const TILT_RANGE: (f64, f64) = (-90.0, 90.0);
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

    /// Turns the angle of scope `id` by `degrees`, within its span.
    pub fn turn(&mut self, id: &str, degrees: f64) {
        let (angle, (lo, hi)) = match id {
            TAU1 => (&mut self.tau1, TILT_RANGE),
            TAU2 => (&mut self.tau2, TILT_RANGE),
            _ => (&mut self.gamma, ROTATION_RANGE),
        };
        *angle = (*angle + degrees).clamp(lo, hi);
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
    header(ui, section, h, SECTION_TITLES[3], "");
    let angles = Angles::of(&view);
    for (i, (id, label, value, (lo, hi))) in [
        (TAU1, "τ₁", angles.tau1, TILT_RANGE),
        (TAU2, "τ₂", angles.tau2, TILT_RANGE),
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
        panel.place(id, &slider.union(field), clip);
        if changed {
            let mut next = angles;
            next.turn(id, moved - value);
            panel.out.edit(
                basis_edit(next.apply(view).basis()),
                Some(Touch::Tilt),
                panel.scratch,
            );
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
        panel.place(id, &combo.response, clip);
        if let Some(edit) = target_edit(&view, which, picked) {
            panel.out.edit(edit, None, panel.scratch);
        }
    }
}
