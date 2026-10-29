//! The interface the GUI calls (gui_state_contract §1, §2; R-390; RQ-254): apply a `SetField`, read the latest
//! snapshot, and request undo and redo. The events are read from the snapshot, which carries the log entries
//! accumulated since the previous one (RQ-245). Everything crossing it is plain data: no handle into engine state
//! is ever given out. The real engine implements it with [`StateStore`](crate::contract::store::StateStore); the gui
//! crate's mock engine is a test double of it. Both pass [`conformance`](crate::contract::conformance).

use crate::contract::set_field::{HistoryKey, SetField};
use crate::contract::snapshot::Snapshot;

/// The GUI-facing interface: one way in, `set_field` and the undo / redo requests; one way out, the snapshot
/// (gui_state_contract §1).
pub trait EngineInterface {
    /// Applies one typed edit (gui_state_contract §2). Unless it is marked "no history", it enters the contract's one
    /// undo history and clears the redo history (R-52, R-69, R-101). Each applied edit logs one `info` entry from
    /// `contract` (render_gui_spec §G12).
    fn set_field(&mut self, edit: SetField);

    /// Applies `edit` under the history key `key` (gui_state_contract §2, "A drag coalesces into one history entry";
    /// R-96, R-458 F1, R-466): as [`EngineInterface::set_field`], but the edits sent under one key, one after another,
    /// are one history entry, its before each field's value at the first of them and its after the last value sent,
    /// logged once, when the entry closes ([`EngineInterface::end_key`], an edit under another key or none, an undo or
    /// a redo), one line per field it set. An edit marked "no history" enters no entry and leaves the open one open.
    ///
    /// The default applies the edit as `set_field` does, an entry of its own: a double that keeps no history keys. An
    /// engine overrides it, and the conformance suite's key cases check that it does.
    fn set_field_under(&mut self, edit: SetField, key: HistoryKey) {
        let _ = key;
        self.set_field(edit);
    }

    /// The drag or command sent under `key` has ended (gui_state_contract §2: the console logs a drag once, when it
    /// ends): its entry, if still open, closes and is logged. The default does nothing, as the default
    /// [`EngineInterface::set_field_under`] opens no entry.
    fn end_key(&mut self, key: HistoryKey) {
        let _ = key;
    }

    /// The latest snapshot. It carries the log entries accumulated since the previous call, which this call hands
    /// over, so each entry is in exactly one snapshot (RQ-245).
    fn snapshot(&mut self) -> Snapshot;

    /// Requests undo: steps back through the latest undoable edit, if there is one (R-52).
    fn undo(&mut self);

    /// Requests redo: reapplies the latest undone edit, if there is one (R-52).
    fn redo(&mut self);
}
