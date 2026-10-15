//! The Problems pane (render_gui_spec Part II §1, §10; GUI_DESIGN_NOTES § "02 Stain"; 02_stain.png), under the canvas
//! beside the code: "Problems · n", the compile status, the problems and the notes, auto-recompile and Recompile. The
//! status, the problems and the notes are the engine side's; Recompile asks it to recompile. The compile itself, its
//! errors and the last-valid fallback are TASK-M7-24's.

use eframe::egui::{self, vec2, Color32, Rect, RichText, Ui, UiBuilder};

use super::{Content, AUTO_RECOMPILE, RECOMPILE};
use crate::keyboard::scopes::ScopeId;
use crate::windows::Keys;

/// The pane's heading: "Problems · n".
pub fn heading(content: Option<&Content>) -> String {
    format!("Problems · {}", content.map_or(0, |c| c.problems.len()))
}

/// The compile status's colour: green, as 02_stain.png's (a look choice, R-390).
pub const COMPILED: Color32 = Color32::from_rgb(120, 200, 120);

/// Draws the pane in `rect`.
pub fn show(
    ui: &mut Ui,
    rect: Rect,
    content: Option<&Content>,
    auto_recompile: bool,
    keys: &mut Keys<'_>,
    clicked: &mut Vec<ScopeId>,
) {
    let visuals = ui.visuals().clone();
    ui.painter().rect_filled(rect, 0.0, visuals.panel_fill);
    let stroke = visuals.widgets.noninteractive.bg_stroke;
    ui.painter()
        .line_segment([rect.left_top(), rect.right_top()], stroke);
    ui.painter()
        .line_segment([rect.left_top(), rect.left_bottom()], stroke);
    let mut pane = ui.new_child(UiBuilder::new().max_rect(rect.shrink2(vec2(12.0, 10.0))));
    pane.label(RichText::new(heading(content)).heading());
    pane.add_space(6.0);
    if let Some(c) = content {
        pane.label(RichText::new(&c.compiled).monospace().color(COMPILED));
        for problem in &c.problems {
            pane.label(
                RichText::new(problem)
                    .monospace()
                    .color(visuals.error_fg_color),
            );
        }
        for note in &c.notes {
            pane.label(RichText::new(note).monospace().weak());
        }
    }
    pane.add_space(10.0);
    pane.horizontal(|ui| {
        let mut on = auto_recompile;
        let auto = ui.checkbox(&mut on, "auto-recompile");
        if auto.clicked() {
            clicked.push(AUTO_RECOMPILE);
        }
        keys.place(AUTO_RECOMPILE, &auto);
        let recompile = ui.add(egui::Button::new("Recompile"));
        if recompile.clicked() {
            clicked.push(RECOMPILE);
        }
        keys.place(RECOMPILE, &recompile);
    });
}
