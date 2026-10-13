//! `ViewUI`, pure UI (gui_state_contract §2): the GUI's own scratch state, never read by the engine; it is the
//! firewall line, and a polished GUI may define its own. Its edits are not undoable (R-69). Each field is one group
//! §2 lists; a group's fields come with the requirements that need them, and until then it is named and empty
//! (R-133).

/// Pure UI state (gui_state_contract §2) — never read by the engine.
pub struct ViewUI {
    /// The backdrop ref.
    pub backdrop: Backdrop,
    /// Debug category visibility.
    pub debug_categories: DebugCategories,
    /// The keyboard focus scope.
    pub focus: Focus,
    /// The selection.
    pub selection: Selection,
    /// The kept orbits.
    pub kept_orbits: KeptOrbits,
    /// The inspector's `t_cursor`.
    pub inspector: Inspector,
    /// The open windows.
    pub windows: Windows,
    /// Linked views for side by side (v2, R-106).
    pub linked_views: LinkedViews,
    /// The playback transport: play/pause/speed/loop; not undoable, not on the sim key (R-96).
    pub transport: Transport,
    /// The mode the mode switch shows, Explore or Stain (render_gui_spec §G2; RQ-249).
    pub mode: Mode,
}

/// Backdrop ref (gui_state_contract §2).
pub struct Backdrop {}

/// Debug category visibility (gui_state_contract §2).
pub struct DebugCategories {}

/// Keyboard focus scope (gui_state_contract §2): the path of the focused scope through the GUI's tree of scopes
/// (render_gui_spec §G3), each a scope id the GUI defines, from a big scope down; empty when no scope has focus. The
/// keyboard's mode is held with it (R-409 A5): never undoable, and never read by the engine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Focus {
    /// The scope ids, outermost first.
    pub path: Vec<String>,
    /// The keyboard's mode on the focused scope.
    pub mode: KeyMode,
}

/// The keyboard's two modes (render_gui_spec §G3, "Two modes: navigation and interaction"; R-409).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KeyMode {
    /// The arrows move between siblings; the ring is orange. Every landing is in it.
    #[default]
    Navigation,
    /// The focused element takes the arrows, or, a text field, every key but Esc, Enter and Tab; the ring is blue.
    Interaction,
}

/// Selection (gui_state_contract §2).
pub struct Selection {}

/// Kept orbits (gui_state_contract §2): the samples the user kept, in the order kept, each a tab of the Trajectory
/// panel (render_gui_spec §G2, `#1`, `#2`, …). A kept orbit is the snapshot's sample under the cursor when it was kept,
/// GUI-sized plain data the snapshot carried, so the engine, which never reads `ViewUI`, needs no list of them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct KeptOrbits {
    /// The kept samples, oldest first.
    pub orbits: Vec<crate::contract::sample::Sample>,
}

/// Inspector `t_cursor` (gui_state_contract §2).
pub struct Inspector {}

/// Open windows (gui_state_contract §2).
pub struct Windows {
    /// The windows open, in the order they were opened; the dev GUI's windows' frames are TASK-M6-28's.
    pub open: Vec<Window>,
}

/// A window of the dev GUI (render_gui_spec §G5, §G8), by name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Window {
    /// The Inspector: one IC, its trajectory, one timeline (§G8); the lock badge's open in Inspector requests it (§G4).
    Inspector,
}

/// Linked views for side by side (gui_state_contract §2; v2, R-106).
pub struct LinkedViews {}

/// Playback transport: play/pause/speed/loop (gui_state_contract §2; R-96, R-101). The GUI's clock reads it and
/// advances the playhead through a no-history `SetField` while playing (R-101; RQ-246).
pub struct Transport {
    /// Play (`true`) or pause (`false`).
    pub playing: bool,
    /// The playback speed: the playhead moves this many units of `t` for each unit its clock's time source moves.
    pub speed: f64,
}

/// The mode switch's mode (render_gui_spec §G2, "the mode switch **Explore / Stain**"; RQ-249).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// The everyday view (§G2).
    Explore,
    /// The stain node-graph editor (Part II).
    Stain,
}
