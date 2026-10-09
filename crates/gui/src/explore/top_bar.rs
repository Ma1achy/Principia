//! The top bar (render_gui_spec §G2; 01_main.png; RQ-249, RQ-255): the "principia · dev" wordmark; the menus File,
//! View, Windows and Help; Overlays ▾ with its count; Run…, Profiler… and Export…, drawn disabled until their tasks;
//! the mode switch Explore / Stain; the keyboard breadcrumb (§G3); and the status line, in the artboard's format and
//! order, filled from the snapshot, in egui's `Monospace` (Ubuntu Mono, RQ-251). The top bar is the keyboard's big
//! scope 1, its controls the sub-scopes Enter reaches, in the order they are drawn; Enter on one acts as its click.

use eframe::egui::containers::menu::MenuButton;
use eframe::egui::{
    self, Align, Button, Layout as EguiLayout, Popup, Rect, Response, RichText, Ui, UiBuilder,
};
use engine::contract::render_state::Overlays;
use engine::contract::snapshot::Snapshot;
use engine::contract::view_ui::Mode;

use crate::app::Actions;
use crate::keyboard::scopes::{Scope, ScopeId, ScopeTree};
use crate::keyboard::{Keyboard, Place};

/// The top bar's controls as keyboard scopes, in the order they are drawn: the menus, Overlays ▾, Run…, Profiler…,
/// Export…, Help, and the mode switch's two sides.
pub const CONTROLS: [(ScopeId, &str); 10] = [
    ("file", "File"),
    ("view", "View"),
    ("windows", "Windows"),
    ("overlays", "Overlays ▾"),
    ("run", "Run…"),
    ("profiler", "Profiler…"),
    ("export", "Export…"),
    ("help", "Help"),
    ("explore", "Explore"),
    ("stain", "Stain"),
];

/// The controls that open a menu.
const MENUS: [ScopeId; 5] = ["file", "view", "windows", "overlays", "help"];

/// Joins the top bar to `tree` as big scope 1, its controls under it.
pub fn register(tree: &mut ScopeTree) {
    tree.register(None, Scope::group("top_bar", "Top bar"));
    for (id, label) in CONTROLS {
        tree.register(Some("top_bar"), Scope::control(id, label));
    }
}

/// Enter on control `id`, drawn at `place`: a menu opens; the mode switch switches; a disabled control does nothing.
pub fn activate(ctx: &egui::Context, id: &str, place: Option<Place>, actions: &mut Actions) {
    match id {
        "explore" => actions.mode = Some(Mode::Explore),
        "stain" => actions.mode = Some(Mode::Stain),
        menu if MENUS.contains(&menu) => {
            if let Some(widget) = place.and_then(|p| p.widget) {
                Popup::open_id(ctx, widget.with("popup"));
            }
        }
        _ => {}
    }
}

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

/// What the top bar shows besides the snapshot.
pub struct TopBar<'a> {
    /// The mode switch's mode.
    pub mode: Mode,
    /// The keyboard breadcrumb.
    pub breadcrumb: &'a str,
}

/// Draws the top bar in `rect`, recording what its controls ask for in `actions` and where they are in `keyboard`.
pub fn show(
    ui: &mut Ui,
    rect: Rect,
    snapshot: &Snapshot,
    bar_state: &TopBar<'_>,
    keyboard: &mut Keyboard,
    actions: &mut Actions,
) {
    let mode = bar_state.mode;
    keyboard.place("top_bar", rect);
    ui.painter().rect_filled(rect, 0.0, ui.visuals().panel_fill);
    let inner = rect.shrink2(egui::vec2(8.0, 0.0));
    let mut bar = ui.new_child(
        UiBuilder::new()
            .max_rect(inner)
            .layout(EguiLayout::left_to_right(Align::Center)),
    );
    let menu = |ui: &mut Ui, name: &str, content: &mut dyn FnMut(&mut Ui)| -> Response {
        MenuButton::from_button(Button::new(name).frame(false))
            .ui(ui, |ui| content(ui))
            .0
    };
    bar.label(RichText::new("principia · dev").strong());
    let file = menu(&mut bar, "File", &mut |ui| {
        if ui.button("Quit").clicked() {
            actions.quit = true;
        }
    });
    keyboard.place_widget("file", &file);
    let view = menu(&mut bar, "View", &mut |ui| {
        if ui.button("F3 hide").clicked() {
            actions.toggle_layer = true;
        }
    });
    keyboard.place_widget("view", &view);
    let windows = menu(&mut bar, "Windows", &mut |ui| {
        for name in WINDOWS {
            ui.add_enabled(false, Button::new(name));
        }
    });
    keyboard.place_widget("windows", &windows);
    let overlays = format!("Overlays ▾ {}", overlays_on(&snapshot.render.overlays));
    let (overlays, _) = MenuButton::from_button(Button::new(overlays)).ui(&mut bar, |_| {});
    keyboard.place_widget("overlays", &overlays);
    for (id, name) in [
        ("run", "Run…"),
        ("profiler", "Profiler…"),
        ("export", "Export…"),
    ] {
        let response = bar.add_enabled(false, Button::new(name));
        keyboard.place_widget(id, &response);
    }
    let help = menu(&mut bar, "Help", &mut |ui| {
        if ui.button(KEYS_ENTRY).clicked() {
            actions.shortcuts = true;
            ui.close();
        }
    });
    keyboard.place_widget("help", &help);
    bar.separator();
    for (id, this, name) in [
        ("explore", Mode::Explore, "Explore"),
        ("stain", Mode::Stain, "Stain"),
    ] {
        let response = bar.add(Button::selectable(mode == this, name));
        if response.clicked() {
            actions.mode = Some(this);
        }
        keyboard.place_widget(id, &response);
    }
    bar.with_layout(EguiLayout::right_to_left(Align::Center), |ui| {
        ui.label(RichText::new(status_line(snapshot)).monospace());
        crate::explore::breadcrumb::show(ui, bar_state.breadcrumb);
    });
}

/// Help's entry that opens the `?` shortcuts (RQ-250).
pub const KEYS_ENTRY: &str = "Keys (?)";
