//! `SetField`, the typed edit `setField(path, value)` (gui_state_contract §1, §2): the only way in. A GUI emits it
//! as plain data and never mutates engine state directly. Every `SimConfig` and `RenderState` edit is undoable
//! (R-69); `ViewUI` is never read by the engine, so no edit reaches it. The paths are added with the fields they
//! name (R-133): the first is the playhead's, `RenderField::Playhead` (RQ-243); the navigation paths, `z₀`, the basis
//! and the lock, came with the dev GUI's Manifold view, the overlay set and the hover with its Overlays menu and
//! hover label, the horizon with its Run window, the stain graph with its Stain editor (R-390, "Contract fields"), and the
//! debug view with View › Debug view (R-467 D5).

use crate::contract::render_state::{Hover, Overlays, Playhead, StainGraph};
use crate::contract::sim_config::{Horizon, Latent, Lock, Window};

/// One typed edit: a field path with its value (gui_state_contract §2).
#[derive(Clone, Debug, PartialEq)]
pub struct SetField {
    /// The field edited, and its new value.
    pub edit: Edit,
    /// Marked "no history": the edit does not enter undo, as the playback clock's playhead advance (R-101).
    pub no_history: bool,
}

/// A history key (gui_state_contract §2, "A drag coalesces into one history entry" and "A command of several edits is
/// one history entry"; R-96, R-466): the edits sent under one key, one after another, are one history entry, its before
/// each field's value at the first of them and its after the last value sent; an edit under another key, or under
/// none, closes the entry. A drag sends every frame's edit under its own key; a command of several edits (a lock, a
/// section reset, Home) sends them in one frame under one fresh key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HistoryKey(pub u64);

/// The field path, by surface, carrying its value (gui_state_contract §2).
#[derive(Clone, Debug, PartialEq)]
pub enum Edit {
    /// A `SimConfig` field: re-integrates.
    Sim(SimField),
    /// A `RenderState` field: live.
    Render(RenderField),
}

/// A `SimConfig` field with its value (gui_state_contract §2; R-133). Navigation is chart construction: pan and
/// slice edit `z₀`, zoom, tilt and rotation the basis (chart_decoder_contract Part 4), each gesture one edit.
#[derive(Clone, Debug, PartialEq)]
pub enum SimField {
    /// `Plane.z0`: pan, slice, a slider's excursion, and the recentring of a lock (render_gui_spec §G4).
    Z0(Latent),
    /// `Plane.q1` and `Plane.q2` together, so a zoom or a rotation, which edits both, is one edit.
    Basis {
        /// The new `q₁`.
        q1: Latent,
        /// The new `q₂`.
        q2: Latent,
    },
    /// The lock group: locking and unlocking, undoable (R-69).
    Lock(Lock),
    /// `Chart.window`, a nonlinear chart's view window: chart construction, undoable (R-69); a lock on a nonlinear
    /// chart re-centres it in the lock's one command (chart_decoder_contract Part 4, "The lock's construction,
    /// defined"; R-466).
    Window(Window),
    /// The horizon `T`, a sim key, undoable (R-69): the Run window's `T_horizon`, in `[50, 200]` (render_gui_spec §G5,
    /// R-132; gui_state_contract §2).
    Horizon(Horizon),
}

/// A `RenderState` field with its value (gui_state_contract §2; R-133).
#[derive(Clone, Debug, PartialEq)]
pub enum RenderField {
    /// The playhead `t` (RQ-243): the GUI's no-history clock advance (R-101) and a scrub (R-96) both write it.
    Playhead(Playhead),
    /// The overlay set (gui_state_contract §2): a toggle of Overlays ▾, or "all off", is one edit of the whole set.
    Overlays(Overlays),
    /// The cursor over the figure (gui_state_contract §2), always sent marked "no history" (R-101).
    Hover(Hover),
    /// The stain graph (gui_state_contract §2, §5): an edit of the Stain editor (picking a node's occupant, a param)
    /// is one edit of the whole graph, undoable (R-69).
    StainGraph(StainGraph),
    /// View › Debug view, on or off (gui_state_contract §2; R-467 D5): one edit per toggle, undoable as an Overlays ▾
    /// toggle is (R-69).
    DebugView(bool),
}
