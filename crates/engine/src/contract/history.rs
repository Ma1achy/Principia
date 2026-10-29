//! The contract's one undo/redo history (R-52; gui_state_contract §2), kept the same way by every engine: the real
//! engine's [`StateStore`](crate::contract::store::StateStore) and the gui crate's mock both hold one, as both apply an
//! edit through [`write`](crate::contract::store::write).
//!
//! An edit under no history key is one entry, logged when it is applied. A drag sends every frame's edit under its own
//! [`HistoryKey`], and a command of several edits sends them under one fresh key: the edits sent under one key, one
//! after another, are one entry, its before each field's value at the first of them and its after the last value sent,
//! and the entry is logged once, when it closes, one "SetField <path> <before> → <after>" line per field it set (R-96,
//! R-458 F1, R-466). The entry closes when its key ends (the GUI's end of the drag), at an edit under another key or
//! none, and at an undo or a redo. An edit marked "no history" enters no entry, is logged as it is applied, and leaves
//! the open entry open (R-101).

use crate::contract::set_field::{Edit, HistoryKey, SetField};
use crate::contract::store::{edit_path, edit_value, set_field_message};

/// One history entry: its edits in the order applied, each with the value its field held before it. Undo restores the
/// befores last first, so each field ends at its value before the entry's first edit of it; redo reapplies the edits in
/// order, so each ends at the last value sent.
#[derive(Clone, Debug)]
struct Entry {
    edits: Vec<(Edit, Edit)>,
}

impl Entry {
    /// The entry's log lines: one per field path it set, in the order first set, "SetField <path> <before> → <after>",
    /// the before the field's value before the entry's first edit of it and the after its last.
    fn lines(&self) -> Vec<String> {
        let mut fields: Vec<(&'static str, &Edit, &Edit)> = Vec::new();
        for (before, after) in &self.edits {
            let path = edit_path(after);
            match fields.iter_mut().find(|(p, _, _)| *p == path) {
                Some(field) => field.2 = after,
                None => fields.push((path, before, after)),
            }
        }
        fields
            .into_iter()
            .map(|(path, before, after)| {
                format!(
                    "SetField {path} {} → {}",
                    edit_value(before),
                    edit_value(after)
                )
            })
            .collect()
    }
}

/// The undo and redo histories, and the key of the entry still open to edits under it.
#[derive(Clone, Debug, Default)]
pub struct History {
    undo: Vec<Entry>,
    redo: Vec<Entry>,
    /// The key of the latest entry while it is open: the key of the latest undoable edit, until that entry closes.
    open: Option<HistoryKey>,
}

impl History {
    /// An empty history.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records `edit`, already applied, `before` the value it replaced, under `key`, and returns the lines to log now,
    /// in order. Marked "no history", it enters no entry and its own line is logged at once, the open entry left open.
    /// Under no key it closes the open entry, logging it, and is an entry of its own, logged at once. Under the open
    /// entry's key it joins that entry, logging nothing; under another key it closes the open entry, logging it, and
    /// opens one of its own, logged when that closes. An edit that enters the history clears redo.
    pub fn record(
        &mut self,
        before: Edit,
        edit: &SetField,
        key: Option<HistoryKey>,
    ) -> Vec<String> {
        if edit.no_history {
            return vec![set_field_message(&before, edit)];
        }
        self.redo.clear();
        let joins = key.is_some() && key == self.open;
        let mut lines = Vec::new();
        if !joins {
            lines = self.close_open();
        }
        if key.is_none() {
            lines.push(set_field_message(&before, edit));
        }
        let change = (before, edit.edit.clone());
        match self.undo.last_mut() {
            Some(entry) if joins => entry.edits.push(change),
            _ => self.undo.push(Entry {
                edits: vec![change],
            }),
        }
        self.open = key;
        lines
    }

    /// The drag or command under `key` has ended: its entry, if it is the open one, closes, and its lines are returned
    /// to log. Any other key returns none.
    pub fn end_key(&mut self, key: HistoryKey) -> Vec<String> {
        if self.open == Some(key) {
            self.close_open()
        } else {
            Vec::new()
        }
    }

    /// Closes the open entry, returning its lines to log; none when no entry is open.
    fn close_open(&mut self) -> Vec<String> {
        match self.open.take() {
            Some(_) => self.undo.last().map(Entry::lines).unwrap_or_default(),
            None => Vec::new(),
        }
    }

    /// Undo: closes the open entry, then steps back through the latest entry, if there is one, writing each of its
    /// befores with `write`, last first. Returns the closed entry's lines to log.
    pub fn undo(&mut self, mut write: impl FnMut(Edit)) -> Vec<String> {
        let lines = self.close_open();
        if let Some(entry) = self.undo.pop() {
            for (before, _) in entry.edits.iter().rev() {
                write(before.clone());
            }
            self.redo.push(entry);
        }
        lines
    }

    /// Redo: closes the open entry, then reapplies the latest undone entry, if there is one, writing each of its edits
    /// with `write`, in order. Returns the closed entry's lines to log.
    pub fn redo(&mut self, mut write: impl FnMut(Edit)) -> Vec<String> {
        let lines = self.close_open();
        if let Some(entry) = self.redo.pop() {
            for (_, after) in &entry.edits {
                write(after.clone());
            }
            self.undo.push(entry);
        }
        lines
    }

    /// The undo history's depth, as the snapshot's u32 (R-329), saturating.
    pub fn undo_depth(&self) -> u32 {
        u32::try_from(self.undo.len()).unwrap_or(u32::MAX)
    }

    /// The redo history's depth, as the snapshot's u32, saturating.
    pub fn redo_depth(&self) -> u32 {
        u32::try_from(self.redo.len()).unwrap_or(u32::MAX)
    }
}
