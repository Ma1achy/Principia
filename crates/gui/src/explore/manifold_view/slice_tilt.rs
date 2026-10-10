//! Slice & tilt (render_gui_spec §G2): the slice step, the tilts `τ₁`, `τ₂` and the rotation `γ`. The slice step is
//! `z₀`'s component along the hidden slice direction, so moving it slices, a `SetField` on `z₀` (re-based to the anchor
//! when locked, as Centre z₀'s sliders are); each angle is one `SetField` on the basis, which turns about the centre,
//! and, locked, about the pin: the app turns every basis edit about it ([`super::about_the_pin`], §G4). The tilts span
//! `[−90°, 90°]` (chart_decoder_contract Part 4, "Tilt"), the rotation `[−180°, 180°]`. Touching the step shows slicing
//! on the compass; touching an angle, tilting.

use eframe::egui::{self, DragValue, Rect, Slider, Ui};

use super::centre::slider_row;
use super::{
    basis_edit, header, row, row_ui, z0_edit, Panel, Touch, View, AXES, GAMMA, SLICE_STEP, TAU1,
    TAU2,
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

/// Draws Slice & tilt in `section`.
pub fn show(ui: &mut Ui, section: Rect, h: f32, clip: Rect, panel: &mut Panel<'_>) {
    let plane = panel.snapshot.sim.plane.clone();
    let lock = panel.snapshot.sim.lock.clone();
    let view = View::of(&plane);
    header(
        ui,
        section,
        h,
        "Slice & tilt",
        &format!("along {}", AXES[view.d]),
    );
    let anchor = lock.locked.then_some(lock.z_locked[view.d]);
    let (moved, response) = slider_row(ui, row(section, h, 1), "step", plane.z0[view.d], anchor);
    panel.place(SLICE_STEP, &response, clip);
    if let Some(value) = moved {
        let mut z0 = plane.z0;
        z0[view.d] = value;
        panel
            .out
            .edit(z0_edit(z0), Some(Touch::Slice), panel.scratch);
    }
    let angles = Angles::of(&view);
    for (i, (id, label, value, (lo, hi))) in [
        (TAU1, "τ₁", angles.tau1, TILT_RANGE),
        (TAU2, "τ₂", angles.tau2, TILT_RANGE),
        (GAMMA, "γ", angles.gamma, ROTATION_RANGE),
    ]
    .into_iter()
    .enumerate()
    {
        let rect = row(section, h, 2 + i);
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
}
