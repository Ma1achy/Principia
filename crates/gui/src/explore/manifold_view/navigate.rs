//! Navigate (render_gui_spec §G2): the centre `(u, v)` and the zoom in log₂, editable by drag or by typing; under them
//! the depth readout, the precision warning and the lock (`depth`, `lock`), and nothing more: no grid of `z₀` values,
//! which repeated the Slice centre sliders (R-457). And the figure's own gestures, which are
//! navigation too: a drag pans, the wheel zooms, and with the Figure scope focused the arrows pan and + / − zoom (§G3).
//! Each is one `SetField`: a pan on `z₀` (`Δz₀ ∈ span(q₁, q₂)`), a zoom on the basis, scaled about the centre, and,
//! locked, about the pin ([`super::about_the_pin`]; chart_decoder_contract Part 4). A primary click with no drag, or
//! P with the pointer over the figure, asks for the sample there to be pinned in the Trajectory panel, which sends no
//! `SetField` (R-453 D1, D6).

use eframe::egui::{self, DragValue, Key, Rect, Response, RichText, Sense, Slider, Ui};
use engine::contract::sim_config::Plane;

use super::{
    add, basis_edit, header, in_plane, row, row_ui, z0_edit, Out, Panel, Scratch, View, CENTRE_U,
    CENTRE_V, SECTION_TITLES, ZOOM, ZOOM_RANGE,
};
use crate::keyboard::keymap::{multiplier, Direction};
use crate::keyboard::scopes::{Adjust, Base, StepKind};

/// The note beside Navigate's title, as 01_main.png's.
pub const NOTE: &str = "edits z₀ and the basis — no camera";
/// The zoom, in octaves, a point of wheel scroll gives (a look choice, R-390).
pub const OCTAVES_PER_POINT: f64 = 1.0 / 64.0;
/// The zoom, in octaves, a line of wheel scroll (one notch) gives (a look choice, R-390): an eighth.
pub const OCTAVES_PER_LINE: f64 = 1.0 / 8.0;

/// The zoom, in octaves, the wheel events among `events` ask for: up zooms in.
pub fn wheel_octaves(events: &[egui::Event]) -> f64 {
    events
        .iter()
        .map(|e| match e {
            egui::Event::MouseWheel { unit, delta, .. } => {
                let per = match unit {
                    egui::MouseWheelUnit::Point => OCTAVES_PER_POINT,
                    egui::MouseWheelUnit::Line => OCTAVES_PER_LINE,
                    egui::MouseWheelUnit::Page => 1.0,
                };
                f64::from(delta.y) * per
            }
            _ => 0.0,
        })
        .sum()
}

/// The magnification `2^zoom`, as the zoom row reads it: "× 1.00".
pub fn magnification(zoom: f64) -> String {
    format!("× {:.2}", zoom.exp2())
}

/// Draws Navigate's own rows in `section`, then the depth readout and the lock under them.
pub fn show(ui: &mut Ui, section: Rect, h: f32, clip: Rect, panel: &mut Panel<'_>) {
    header(ui, section, h, SECTION_TITLES[1], NOTE);
    let plane = panel.snapshot.sim.plane.clone();
    let view = View::read(&panel.snapshot.sim);
    let label_w = super::chart::LABEL_W;
    let mut centre = row_ui(ui, row(section, h, 1));
    centre.add_sized([label_w, h], egui::Label::new("centre u, v"));
    if let Some((u, v)) = in_plane(&plane, &plane.z0) {
        let speed = view.scale / 200.0;
        for (id, value, along) in [(CENTRE_U, u, plane.q1), (CENTRE_V, v, plane.q2)] {
            let mut edited = value;
            // As tall as egui's controls: the rows are never taller than a control and its 2-point gap.
            let field_h = centre.spacing().interact_size.y;
            let field = centre.add_sized(
                [72.0, field_h],
                DragValue::new(&mut edited).speed(speed).max_decimals(6),
            );
            panel.place(id, &field, clip);
            if field.changed() {
                // Along the unit direction: the other coordinate stays where it is.
                let unit = super::norm(&along);
                panel.out.edit(
                    z0_edit(add(&plane.z0, (edited - value) / unit, &along)),
                    None,
                    panel.scratch,
                );
            }
        }
    }
    let mut zoom_row = row_ui(ui, row(section, h, 2));
    zoom_row.add_sized([label_w, h], egui::Label::new("zoom (log₂)"));
    let mut zoom = view.zoom();
    zoom_row.spacing_mut().slider_width = (zoom_row.available_width() - 70.0).max(40.0);
    let slider =
        zoom_row.add(Slider::new(&mut zoom, ZOOM_RANGE.0..=ZOOM_RANGE.1).show_value(false));
    panel.place(ZOOM, &slider, clip);
    zoom_row.label(RichText::new(magnification(view.zoom())).monospace());
    if slider.changed() {
        panel.out.edit(
            basis_edit(view.with_zoom(zoom).basis()),
            None,
            panel.scratch,
        );
    }
    super::depth::show(ui, row(section, h, 3), row(section, h, 4), panel);
    super::lock::show(ui, row(section, h, 5), row(section, h, 6), clip, panel);
}

/// The pan of a drag by `delta` points over a figure `size` points in size: the picture follows the pointer, `t`
/// being Y-up.
pub fn pan(
    plane: &Plane,
    delta: egui::Vec2,
    size: egui::Vec2,
) -> engine::contract::set_field::SetField {
    let ds = -f64::from(delta.x) / f64::from(size.x);
    let dt = f64::from(delta.y) / f64::from(size.y);
    z0_edit(add(
        &add(&plane.z0, 2.0 * ds, &plane.q1),
        2.0 * dt,
        &plane.q2,
    ))
}

/// The zoom by `octaves` about the centre: the basis scaled by `2^−octaves`.
pub fn zoom_by(plane: &Plane, octaves: f64) -> engine::contract::set_field::SetField {
    let k = (-octaves).exp2();
    basis_edit((plane.q1.map(|x| x * k), plane.q2.map(|x| x * k)))
}

/// The arrows on the focused figure: ← → along `q₁`, ↑ ↓ along `q₂`, by `fraction` of the view's span.
pub fn pan_by_keys(
    adjust: &Adjust,
    plane: &Plane,
    fraction: f64,
    scratch: &mut Scratch,
    out: &mut Out,
) {
    let along = match adjust.direction {
        Direction::Left | Direction::Right => &plane.q1,
        Direction::Up | Direction::Down => &plane.q2,
    };
    out.edit(
        z0_edit(add(&plane.z0, 2.0 * fraction, along)),
        None,
        scratch,
    );
}

/// The figure's point under `pos`, `(s, t)`, `t` Y-up (the coordinate convention's one flip).
pub fn figure_coords(figure: Rect, pos: egui::Pos2) -> (f64, f64) {
    let s = f64::from((pos.x - figure.min.x) / figure.width());
    let t = f64::from((figure.max.y - pos.y) / figure.height());
    (s, t)
}

/// The figure's gestures in `figure`: drag to pan, the wheel to zoom, K over a point or right-click › lock here to
/// lock there, and, with the Figure scope focused, + / − to zoom. Takes no key while a text field holds the keyboard.
/// Returns the figure point `(s, t)` to pin in the Trajectory panel, if any: a primary click's with no drag past egui's
/// click threshold, or the pointer's when P is pressed over the figure, as K locks, in either keyboard mode (R-453 D1,
/// D6).
pub fn figure(
    ui: &mut Ui,
    figure: Rect,
    focused: bool,
    snapshot: &engine::contract::snapshot::Snapshot,
    scratch: &mut Scratch,
    out: &mut Out,
) -> Option<[f64; 2]> {
    let plane = &snapshot.sim.plane;
    let response: Response = ui.interact(
        figure,
        egui::Id::new("figure gestures"),
        Sense::click_and_drag(),
    );
    if response.dragged_by(egui::PointerButton::Primary) {
        let delta = response.drag_delta();
        if delta != egui::Vec2::ZERO {
            out.edit(pan(plane, delta, figure.size()), None, scratch);
        }
    }
    let typing = ui.ctx().text_edit_focused();
    let mut pin = None;
    if response.clicked_by(egui::PointerButton::Primary) {
        pin = response.interact_pointer_pos();
    }
    if response.hovered() {
        let octaves = ui.input(|i| wheel_octaves(&i.events));
        if octaves != 0.0 {
            out.edit(zoom_by(plane, octaves), None, scratch);
        }
        if !typing && ui.input(|i| i.key_pressed(Key::K)) {
            if let Some(pos) = response.hover_pos() {
                super::lock::lock_at(plane, figure_coords(figure, pos), scratch, out);
            }
        }
        if !typing && ui.input(|i| i.key_pressed(Key::P)) {
            pin = response.hover_pos().or(pin);
        }
    }
    if response.secondary_clicked() {
        scratch.menu_at = response
            .interact_pointer_pos()
            .map(|p| figure_coords(figure, p));
    }
    response.context_menu(|ui| {
        if ui.button(super::lock::LOCK_HERE).clicked() {
            if let Some(at) = scratch.menu_at.take() {
                super::lock::lock_at(plane, at, scratch, out);
            }
            ui.close();
        }
    });
    if focused && !typing {
        let presses: Vec<(f64, egui::Modifiers)> = ui.input(|i| {
            i.events
                .iter()
                .filter_map(|e| match e {
                    egui::Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } => match key {
                        Key::Plus | Key::Equals => Some((1.0, *modifiers)),
                        Key::Minus => Some((-1.0, *modifiers)),
                        _ => None,
                    },
                    _ => None,
                })
                .collect()
        });
        let Base::Log2(base) = StepKind::ZoomLog2.base() else {
            unreachable!("the zoom's base step is in octaves")
        };
        let mut plane = plane.clone();
        for (sign, modifiers) in presses {
            let edit = zoom_by(&plane, sign * base * multiplier(modifiers));
            if let engine::contract::set_field::Edit::Sim(
                engine::contract::set_field::SimField::Basis { q1, q2 },
            ) = &edit.edit
            {
                (plane.q1, plane.q2) = (*q1, *q2);
            }
            out.edit(edit, None, scratch);
        }
    }
    pin.map(|pos| {
        let (s, t) = figure_coords(figure, pos);
        [s, t]
    })
}
