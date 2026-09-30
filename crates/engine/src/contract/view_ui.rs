//! `ViewUI`, pure UI (gui_state_contract §2): the GUI's own scratch state, never read by the engine; it is the
//! firewall line, and a polished GUI may define its own. Its edits are not undoable (R-69). Each field is one group
//! §2 lists; at M0 a group is named and empty (R-133).

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
}

/// Backdrop ref (gui_state_contract §2).
pub struct Backdrop {}

/// Debug category visibility (gui_state_contract §2).
pub struct DebugCategories {}

/// Keyboard focus scope (gui_state_contract §2).
pub struct Focus {}

/// Selection (gui_state_contract §2).
pub struct Selection {}

/// Kept orbits (gui_state_contract §2).
pub struct KeptOrbits {}

/// Inspector `t_cursor` (gui_state_contract §2).
pub struct Inspector {}

/// Open windows (gui_state_contract §2).
pub struct Windows {}

/// Linked views for side by side (gui_state_contract §2; v2, R-106).
pub struct LinkedViews {}

/// Playback transport: play/pause/speed/loop (gui_state_contract §2; R-96, R-101).
pub struct Transport {}
