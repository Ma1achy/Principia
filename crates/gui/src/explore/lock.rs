//! The lock's marks (render_gui_spec §G4; GUI_DESIGN_NOTES § "08 Lock"; 08_lock.png): the badge, "● locked at
//! z_locked", with unlock and open in Inspector, which requests the Inspector window until TASK-M6-28 builds its frame;
//! and the gold reticle on the figure at the anchor, which locking recentres the view on: the one mark the GUI draws
//! on the figure besides the hover label (§G1, "the hover label and the lock reticle are the figure's own marks").
//! Built here on the mock's lock (applied per R-369, review 5434766412 on PR #157); REQ-GUI-099 stays TASK-M8-07's.

use eframe::egui::{
    self, Button, Color32, Frame, Id, LayerId, Order, Pos2, Rect, Response, RichText, Stroke, Ui,
};
use engine::contract::sim_config::{Latent, Lock};

/// The lock's gold, of the badge, the reticle, the compass's pin and the sliders' anchor marks: 08_lock.png's gold
/// (a look choice, R-390).
pub const GOLD: Color32 = Color32::from_rgb(232, 196, 96);
/// The badge's fill: the gold, dimmed (a look choice, R-390).
pub const BADGE_FILL: Color32 = Color32::from_rgb(64, 52, 24);
/// The badge's text while locked.
pub const LOCKED: &str = "● locked at z_locked";
/// The badge's text while free.
pub const UNLOCKED: &str = "○ not locked";
/// The badge's unlock.
pub const UNLOCK: &str = "unlock";
/// The badge's open in Inspector.
pub const OPEN_INSPECTOR: &str = "open in Inspector";
/// The reticle's radius, in points (a look choice, R-390).
pub const RETICLE_R: f32 = 14.0;
/// The reticle's ticks' length beyond its circle, in points (a look choice, R-390).
pub const RETICLE_TICK: f32 = 8.0;

/// The anchor's value as the panel reads it: "z_locked (0.180, 0.410, …)".
pub fn anchor_text(z: &Latent) -> String {
    let parts: Vec<String> = z.iter().map(|v| format!("{v:.3}")).collect();
    format!("z_locked ({})", parts.join(", "))
}

/// Draws the badge in `ui`: its text, then unlock and open in Inspector, both disabled while free. Returns the two
/// buttons' responses.
pub fn badge(ui: &mut Ui, lock: &Lock) -> (Response, Response) {
    if lock.locked {
        Frame::new()
            .fill(BADGE_FILL)
            .corner_radius(8.0)
            .inner_margin(egui::Margin::symmetric(6, 1))
            .show(ui, |ui| {
                ui.label(RichText::new(LOCKED).monospace().color(GOLD))
            });
    } else {
        ui.label(RichText::new(UNLOCKED).monospace().weak());
    }
    let unlock = ui.add_enabled(lock.locked, Button::new(UNLOCK));
    let inspector = ui.add_enabled(lock.locked, Button::new(OPEN_INSPECTOR));
    (unlock, inspector)
}

/// Draws the gold reticle centred on `at`, over the figure `figure` and clipped to it.
pub fn reticle(ctx: &egui::Context, figure: Rect, at: Pos2) {
    let painter = ctx
        .layer_painter(LayerId::new(Order::Foreground, Id::new("lock reticle")))
        .with_clip_rect(figure);
    let stroke = Stroke::new(2.0, GOLD);
    painter.circle_stroke(at, RETICLE_R, stroke);
    painter.circle_filled(at, 2.5, GOLD);
    for d in [
        egui::vec2(1.0, 0.0),
        egui::vec2(-1.0, 0.0),
        egui::vec2(0.0, 1.0),
        egui::vec2(0.0, -1.0),
    ] {
        painter.line_segment(
            [
                at + d * (RETICLE_R - 4.0),
                at + d * (RETICLE_R + RETICLE_TICK),
            ],
            stroke,
        );
    }
}

/// Where the reticle goes on `figure`: the anchor's chart point, while locked and in view.
pub fn reticle_at(
    lock: &Lock,
    plane: &engine::contract::sim_config::Plane,
    figure: Rect,
) -> Option<Pos2> {
    if !lock.locked {
        return None;
    }
    let (s, t) = crate::explore::manifold_view::chart_coords(plane, &lock.z_locked)?;
    let at = egui::pos2(
        figure.min.x + figure.width() * s as f32,
        figure.max.y - figure.height() * t as f32,
    );
    figure.contains(at).then_some(at)
}
