//! The top bar (render_gui_spec §G2; 01_main.png; RQ-249, RQ-255): the "principia · dev" wordmark; the menus File,
//! View, Windows and Help; Overlays ▾ with its count; Run…, Profiler… and Export…, drawn disabled until their tasks;
//! the mode switch Explore / Stain; the keyboard breadcrumb's slot, empty until TASK-M6-25; and the status line, in
//! the artboard's format and order, filled from the snapshot, in egui's `Monospace` (Ubuntu Mono, RQ-251).

use eframe::egui::containers::menu::MenuButton;
use eframe::egui::{self, Align, Button, Layout as EguiLayout, Rect, RichText, Ui, UiBuilder};
use engine::contract::render_state::Overlays;
use engine::contract::snapshot::Snapshot;
use engine::contract::view_ui::Mode;

use crate::app::Actions;

/// The Windows menu: render_gui_spec §G5's windows and the console, by their spec names, each disabled until its
/// task (RQ-249).
pub const WINDOWS: [&str; 5] = ["Run", "Profiler", "Export & share", "Display", "Console"];

/// What an absent value reads (RQ-243).
pub const ABSENT: &str = "—";

/// How many overlays are on: the overlay set has no toggle yet, so none is (the mock's count is 0, RQ-249).
pub fn overlays_on(overlays: &Overlays) -> u32 {
    let Overlays {} = overlays;
    0
}

/// `n` with its thousands grouped by spaces, as 01_main.png's "1 842 quads".
pub fn grouped(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

/// The status line: "t 12.40 · 60 fps · 4.1 ms · 1 842 quads · undo 2 / redo 0 · F3 hide", "—" for each absent value
/// (render_gui_spec §G2's list; RQ-255).
pub fn status_line(snapshot: &Snapshot) -> String {
    let frame = &snapshot.frame;
    let or_absent = |v: Option<String>| v.unwrap_or_else(|| ABSENT.to_owned());
    format!(
        "t {:.2} · {} fps · {} ms · {} quads · undo {} / redo {} · F3 hide",
        snapshot.render.playhead.t,
        or_absent(frame.fps.map(|f| format!("{f:.0}"))),
        or_absent(frame.frame_ms.map(|m| format!("{m:.1}"))),
        or_absent(frame.quad_count.map(grouped)),
        snapshot.history.undo_depth,
        snapshot.history.redo_depth,
    )
}

/// Draws the top bar in `rect`, recording what its controls ask for in `actions`.
pub fn show(ui: &mut Ui, rect: Rect, snapshot: &Snapshot, mode: Mode, actions: &mut Actions) {
    ui.painter().rect_filled(rect, 0.0, ui.visuals().panel_fill);
    let inner = rect.shrink2(egui::vec2(8.0, 0.0));
    let mut bar = ui.new_child(
        UiBuilder::new()
            .max_rect(inner)
            .layout(EguiLayout::left_to_right(Align::Center)),
    );
    let menu = |ui: &mut Ui, name: &str, content: &mut dyn FnMut(&mut Ui)| {
        MenuButton::from_button(Button::new(name).frame(false)).ui(ui, |ui| content(ui));
    };
    bar.label(RichText::new("principia · dev").strong());
    menu(&mut bar, "File", &mut |ui| {
        if ui.button("Quit").clicked() {
            actions.quit = true;
        }
    });
    menu(&mut bar, "View", &mut |ui| {
        if ui.button("F3 hide").clicked() {
            actions.toggle_layer = true;
        }
    });
    menu(&mut bar, "Windows", &mut |ui| {
        for name in WINDOWS {
            ui.add_enabled(false, Button::new(name));
        }
    });
    let overlays = format!("Overlays ▾ {}", overlays_on(&snapshot.render.overlays));
    MenuButton::from_button(Button::new(overlays)).ui(&mut bar, |_| {});
    for name in ["Run…", "Profiler…", "Export…"] {
        bar.add_enabled(false, Button::new(name));
    }
    menu(&mut bar, "Help", &mut |ui| {
        ui.add_enabled(false, Button::new("Keys (?)"));
    });
    bar.separator();
    for (this, name) in [(Mode::Explore, "Explore"), (Mode::Stain, "Stain")] {
        if bar.add(Button::selectable(mode == this, name)).clicked() {
            actions.mode = Some(this);
        }
    }
    // The keyboard breadcrumb's slot (§G3) stays empty until TASK-M6-25.
    bar.with_layout(EguiLayout::right_to_left(Align::Center), |ui| {
        ui.label(RichText::new(status_line(snapshot)).monospace());
    });
}
