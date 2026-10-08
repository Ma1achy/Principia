//! The interface the GUI calls (gui_state_contract §1, §2; R-390; RQ-254): apply a `SetField`, read the latest
//! snapshot, and request undo and redo. The events are read from the snapshot, which carries the log entries
//! accumulated since the previous one (RQ-245). Everything crossing it is plain data: no handle into engine state
//! is ever given out. The real engine implements it with [`StateStore`](crate::contract::store::StateStore); the gui
//! crate's mock engine is a test double of it. Both pass [`conformance`](crate::contract::conformance).

use crate::contract::set_field::SetField;
use crate::contract::snapshot::Snapshot;

/// The GUI-facing interface: one way in, `set_field` and the undo / redo requests; one way out, the snapshot
/// (gui_state_contract §1).
pub trait EngineInterface {
    /// Applies one typed edit (gui_state_contract §2). Unless it is marked "no history", it enters the contract's one
    /// undo history and clears the redo history (R-52, R-69, R-101). Each applied edit logs one `info` entry from
    /// `contract` (render_gui_spec §G12).
    fn set_field(&mut self, edit: SetField);

    /// The latest snapshot. It carries the log entries accumulated since the previous call, which this call hands
    /// over, so each entry is in exactly one snapshot (RQ-245).
    fn snapshot(&mut self) -> Snapshot;

    /// Requests undo: steps back through the latest undoable edit, if there is one (R-52).
    fn undo(&mut self);

    /// Requests redo: reapplies the latest undone edit, if there is one (R-52).
    fn redo(&mut self);
}
