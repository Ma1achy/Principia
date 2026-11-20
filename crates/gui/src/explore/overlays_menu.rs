//! Overlays ▾ (render_gui_spec §G2; 01_main.png): the top bar's menu of the overlay set's fourteen toggles, in their
//! four groups, each with its Alt+digit shortcut, then **all off** and **save as default**. A toggle, or all off, is one
//! `SetField` on `RenderState`'s overlay set, undoable (R-69); the engine draws what is on (the mock, tile bounds over
//! its stand-in), and the GUI draws nothing over the figure. The menu is a popup the user opens over the figure, not a
//! toast or label in §G1's sense (RQ-246, RQ-249, RQ-250, decided per R-369). The menu is the top bar's `overlays`
//! scope: Enter opens it on its first entry, the arrows move between the entries, Enter on one acts as its click.
//!
//! **The shortcuts** (applied per R-369, a look silence under R-390): §G2 gives each toggle an Alt+digit, 01_main.png
//! shows Alt+1 to Alt+9 on the first nine and none on the Chart and Stain groups, and ten digits cannot give fourteen
//! toggles one each. So the first nine take Alt+1 to Alt+9 as drawn, the tenth Alt+0, and the last four Alt+Shift+1 to
//! Alt+Shift+4 ([`SHORTCUT_KEYS`]). They act whenever the egui layer is shown and no text field holds the keyboard.
//! They are read from key events, never from text, since macOS's Option+digit types a character.

use eframe::egui::{self, Key, Modifiers, RichText, Ui};
use engine::contract::render_state::Overlays;

use crate::app::Actions;
use crate::keyboard::scopes::{Scope, ScopeId, ScopeTree};
use crate::keyboard::Keyboard;

/// The menu's scope, the top bar's control.
pub const MENU: ScopeId = "overlays";

/// The fourteen toggles' scopes and labels, in §G2's order.
pub const TOGGLES: [(ScopeId, &str); 14] = [
    ("ov_tile_bounds", "tile bounds"),
    ("ov_depth_colouring", "depth colouring"),
    ("ov_priority_heatmap", "priority heatmap"),
    ("ov_refine_flagged", "refine_flagged"),
    ("ov_split_merge", "split / merge activity"),
    ("ov_substep_density", "substep density"),
    ("ov_substep_cap", "substep cap hits"),
    ("ov_nan_invalid", "NaN / invalid pixels"),
    ("ov_energy_drift", "energy drift"),
    ("ov_forbidden_region", "forbidden region"),
    ("ov_grid", "grid"),
    ("ov_burrau_lattice", "Burrau lattice (primitive triples)"),
    ("ov_class_edges", "class edges"),
    ("ov_t_end_contours", "t_end contours"),
];

/// The groups, each a heading and how many toggles of [`TOGGLES`] it holds, in order (§G2: five, four, three, two).
pub const GROUPS: [(&str, usize); 4] = [
    ("QUADTREE", 5),
    ("INTEGRATION", 4),
    ("CHART", 3),
    ("STAIN", 2),
];

/// The menu's width, in points (a look choice, R-390).
pub const MENU_W: f32 = 320.0;

/// "all off".
pub const ALL_OFF: (ScopeId, &str) = ("ov_all_off", "all off");
/// "save as default".
pub const SAVE_DEFAULT: (ScopeId, &str) = ("ov_save_default", "save as default");

/// Each toggle's shortcut: its digit key and whether Shift is held with Alt.
pub const SHORTCUT_KEYS: [(Key, bool); 14] = [
    (Key::Num1, false),
    (Key::Num2, false),
    (Key::Num3, false),
    (Key::Num4, false),
    (Key::Num5, false),
    (Key::Num6, false),
    (Key::Num7, false),
    (Key::Num8, false),
    (Key::Num9, false),
    (Key::Num0, false),
    (Key::Num1, true),
    (Key::Num2, true),
    (Key::Num3, true),
    (Key::Num4, true),
];

/// Toggle `i`'s shortcut as the menu shows it: "Alt+1", or "Alt+Shift+1".
pub fn shortcut_label(i: usize) -> String {
    let (key, shift) = SHORTCUT_KEYS[i];
    let digit = key.name();
    if shift {
        format!("Alt+Shift+{digit}")
    } else {
        format!("Alt+{digit}")
    }
}

/// Joins the menu's entries to `tree`, under the top bar's Overlays ▾: the toggles, then all off and save as default.
pub fn register(tree: &mut ScopeTree) {
    for (id, label) in TOGGLES {
        tree.register(Some(MENU), Scope::control(id, label));
    }
    tree.register(Some(MENU), Scope::control(ALL_OFF.0, ALL_OFF.1));
    tree.register(Some(MENU), Scope::control(SAVE_DEFAULT.0, SAVE_DEFAULT.1));
    tree.register_shortcut(
        "Alt+1 … Alt+0, Alt+Shift+1 … 4",
        "toggle an overlay, in Overlays ▾'s order",
    );
}

/// Toggle `i` of `overlays`, in [`TOGGLES`]' order.
pub fn flag(overlays: &mut Overlays, i: usize) -> &mut bool {
    match i {
        0 => &mut overlays.tile_bounds,
        1 => &mut overlays.depth_colouring,
        2 => &mut overlays.priority_heatmap,
        3 => &mut overlays.refine_flagged,
        4 => &mut overlays.split_merge_activity,
        5 => &mut overlays.substep_density,
        6 => &mut overlays.substep_cap_hits,
        7 => &mut overlays.nan_invalid_pixels,
        8 => &mut overlays.energy_drift,
        9 => &mut overlays.forbidden_region,
        10 => &mut overlays.grid,
        11 => &mut overlays.burrau_lattice,
        12 => &mut overlays.class_edges,
        13 => &mut overlays.t_end_contours,
        _ => panic!("no overlay {i}: there are {}", TOGGLES.len()),
    }
}

/// How many overlays are on: Overlays ▾'s count (§G2).
pub fn count_on(overlays: &Overlays) -> u32 {
    let mut o = overlays.clone();
    (0..TOGGLES.len()).filter(|i| *flag(&mut o, *i)).count() as u32
}

/// `overlays` with toggle `i` flipped.
pub fn toggled(overlays: &Overlays, i: usize) -> Overlays {
    let mut o = overlays.clone();
    let f = flag(&mut o, i);
    *f = !*f;
    o
}

/// The toggles the key events in `events` ask for, in order: each a press of a [`SHORTCUT_KEYS`] digit with Alt, and
/// with Shift exactly when the shortcut has it, and no Ctrl or Cmd.
pub fn shortcut_presses(events: &[egui::Event]) -> Vec<usize> {
    events
        .iter()
        .filter_map(|e| match e {
            egui::Event::Key {
                key,
                pressed: true,
                repeat: false,
                modifiers,
                ..
            } => shortcut_of(*key, *modifiers),
            _ => None,
        })
        .collect()
}

/// The toggle `key` with `modifiers` is the shortcut of, if any.
pub fn shortcut_of(key: Key, modifiers: Modifiers) -> Option<usize> {
    if !modifiers.alt || modifiers.ctrl || modifiers.command || modifiers.mac_cmd {
        return None;
    }
    SHORTCUT_KEYS
        .iter()
        .position(|(k, shift)| *k == key && *shift == modifiers.shift)
}

/// Enter on the menu's entry `id`, as its click: a toggle flips its overlay, all off clears them, save as default
/// keeps the set as the default.
pub fn activate(id: &str, overlays: &Overlays, actions: &mut Actions) {
    if let Some(i) = TOGGLES.iter().position(|(t, _)| *t == id) {
        actions.overlays = Some(toggled(overlays, i));
    } else if id == ALL_OFF.0 {
        actions.overlays = Some(Overlays::default());
    } else if id == SAVE_DEFAULT.0 {
        actions.save_overlays = true;
    }
}

/// The menu's content, in its popup `ui`: the groups' headings and toggles, each with its shortcut, then all off and
/// save as default.
pub fn show(ui: &mut Ui, overlays: &Overlays, keyboard: &mut Keyboard, actions: &mut Actions) {
    ui.set_min_width(MENU_W);
    ui.set_max_width(MENU_W);
    let mut i = 0;
    for (heading, n) in GROUPS {
        ui.label(RichText::new(heading).weak().small());
        for _ in 0..n {
            let (id, label) = TOGGLES[i];
            let mut on = *flag(&mut overlays.clone(), i);
            let row = ui.horizontal(|row| {
                let check = row.checkbox(&mut on, label);
                row.with_layout(egui::Layout::right_to_left(egui::Align::Center), |r| {
                    r.label(RichText::new(shortcut_label(i)).monospace().weak());
                });
                check
            });
            keyboard.place_widget(id, &row.inner);
            if row.inner.changed() {
                actions.overlays = Some(toggled(overlays, i));
            }
            i += 1;
        }
    }
    ui.separator();
    ui.horizontal(|row| {
        for (id, label) in [ALL_OFF, SAVE_DEFAULT] {
            let button = row.button(label);
            keyboard.place_widget(id, &button);
            if button.clicked() {
                activate(id, overlays, actions);
            }
        }
    });
}
