//! The real engine's implementation of the GUI-facing interface, as far as the conformance suite reaches (R-390;
//! RQ-254): the state store holds a `SimConfig` and a `RenderState`, the contract's one undo/redo history (R-52) and
//! the log entries since the last snapshot (RQ-245). The frame summary is `None` until TASK-M8-05 wires the frame loop
//! (RQ-243). The surfaces carry no default, so the constructor takes the initial state explicitly.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::contract::interface::EngineInterface;
use crate::contract::log::{LogEntry, Severity, Source};
use crate::contract::render_state::RenderState;
use crate::contract::set_field::{Edit, RenderField, SetField};
use crate::contract::sim_config::SimConfig;
use crate::contract::snapshot::{FrameSummary, History, Precision, Snapshot, Tier};

/// One undoable change: the field's value before the edit and the edit itself (R-52).
#[derive(Clone, Debug)]
struct Change {
    before: Edit,
    after: Edit,
}

/// The real engine's state store (RQ-254).
#[derive(Debug)]
pub struct StateStore {
    sim: SimConfig,
    render: RenderState,
    undo: Vec<Change>,
    redo: Vec<Change>,
    log: Vec<LogEntry>,
}

impl StateStore {
    /// A store holding the initial state given; its history and log are empty.
    pub fn new(sim: SimConfig, render: RenderState) -> Self {
        Self {
            sim,
            render,
            undo: Vec::new(),
            redo: Vec::new(),
            log: Vec::new(),
        }
    }

    /// Writes `edit` into the state and returns the value it replaced, as an edit restoring it.
    fn write(&mut self, edit: Edit) -> Edit {
        match edit {
            Edit::Sim(field) => match field {},
            Edit::Render(RenderField::Playhead(playhead)) => Edit::Render(RenderField::Playhead(
                std::mem::replace(&mut self.render.playhead, playhead),
            )),
        }
    }
}

/// The wall-clock time now, in milliseconds since the Unix epoch (the log entry's `at`, gui_state_contract §2).
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// The `contract` log line of an applied edit: its path, the value before and after, and "(no history)" when so
/// marked (render_gui_spec §G12; 12_console.png's `SetField <path> <before> → <after>`).
pub fn set_field_message(before: &Edit, edit: &SetField) -> String {
    let value = |e: &Edit| match e {
        Edit::Sim(field) => match *field {},
        Edit::Render(RenderField::Playhead(p)) => p.t,
    };
    let path = match &edit.edit {
        Edit::Sim(field) => match *field {},
        Edit::Render(RenderField::Playhead(_)) => "Playhead.t",
    };
    let marker = if edit.no_history { " (no history)" } else { "" };
    format!(
        "SetField {path} {} → {}{marker}",
        value(before),
        value(&edit.edit)
    )
}

impl EngineInterface for StateStore {
    fn set_field(&mut self, edit: SetField) {
        let before = self.write(edit.edit.clone());
        self.log.push(LogEntry {
            severity: Severity::Info,
            at: now_ms(),
            source: Source::Contract,
            message: set_field_message(&before, &edit),
        });
        if !edit.no_history {
            self.undo.push(Change {
                before,
                after: edit.edit,
            });
            self.redo.clear();
        }
    }

    fn snapshot(&mut self) -> Snapshot {
        Snapshot {
            sim: self.sim.clone(),
            render: self.render.clone(),
            tier: Tier {},
            history: History {
                undo_depth: depth(&self.undo),
                redo_depth: depth(&self.redo),
            },
            precision: Precision {
                decode_switchover: false,
                at_f32_floor: false,
            },
            frame: FrameSummary {
                frame_ms: None,
                fps: None,
                quad_count: None,
                live_memory: None,
            },
            log: std::mem::take(&mut self.log),
        }
    }

    fn undo(&mut self) {
        if let Some(change) = self.undo.pop() {
            self.write(change.before.clone());
            self.redo.push(change);
        }
    }

    fn redo(&mut self) {
        if let Some(change) = self.redo.pop() {
            self.write(change.after.clone());
            self.undo.push(change);
        }
    }
}

/// A history's depth as the snapshot's u32 (R-329), saturating.
fn depth(changes: &[Change]) -> u32 {
    u32::try_from(changes.len()).unwrap_or(u32::MAX)
}
