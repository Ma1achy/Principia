//! `SetField`, the typed edit `setField(path, value)` (gui_state_contract §1, §2): the only way in. A GUI emits it
//! as plain data and never mutates engine state directly. Every `SimConfig` and `RenderState` edit is undoable
//! (R-69); `ViewUI` is never read by the engine, so no edit reaches it. The paths are added with the fields they
//! name (R-133): the first is the playhead's, `RenderField::Playhead` (RQ-243).

use crate::contract::render_state::Playhead;

/// One typed edit: a field path with its value (gui_state_contract §2).
#[derive(Clone, Debug, PartialEq)]
pub struct SetField {
    /// The field edited, and its new value.
    pub edit: Edit,
    /// Marked "no history": the edit does not enter undo, as the playback clock's playhead advance (R-101).
    pub no_history: bool,
}

/// The field path, by surface, carrying its value (gui_state_contract §2).
#[derive(Clone, Debug, PartialEq)]
pub enum Edit {
    /// A `SimConfig` field: re-integrates.
    Sim(SimField),
    /// A `RenderState` field: live.
    Render(RenderField),
}

/// A `SimConfig` field with its value; no path is named yet (R-133).
#[derive(Clone, Debug, PartialEq)]
pub enum SimField {}

/// A `RenderState` field with its value (gui_state_contract §2; R-133).
#[derive(Clone, Debug, PartialEq)]
pub enum RenderField {
    /// The playhead `t` (RQ-243): the GUI's no-history clock advance (R-101) and a scrub (R-96) both write it.
    Playhead(Playhead),
}
