//! The snapshot, the only way out (gui_state_contract §1, §2): GUI-sized state only — view state, the current tier,
//! a few scalars — never engine-sized state (the payload, the quad tree, the reductions), which is only summarised
//! across. The engine posts it throttled to ~10 Hz (caching contract Part 6a, R-94).

use crate::contract::log::LogEntry;
use crate::contract::render_state::RenderState;
use crate::contract::sim_config::SimConfig;

/// A GUI-sized state snapshot (gui_state_contract §1, §2).
#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    /// The sim key the GUI edits.
    pub sim: SimConfig,
    /// The render key the GUI edits.
    pub render: RenderState,
    /// The current tier.
    pub tier: Tier,
    /// The undo/redo history's depth, which a GUI shows (R-52).
    pub history: History,
    /// The precision events the GUI reports (R-54).
    pub precision: Precision,
    /// The GUI-sized frame summary (RQ-243; its definitions, gui_state_contract §2, R-72).
    pub frame: FrameSummary,
    /// The log entries accumulated since the previous snapshot, oldest first (gui_state_contract §2, R-72; RQ-245).
    pub log: Vec<LogEntry>,
}

/// The current tier (gui_state_contract §1); its fields come from the contract that owns it (R-133).
#[derive(Clone, Debug, PartialEq)]
pub struct Tier {}

/// The depth of the contract's one undo/redo history (gui_state_contract §2, R-52; RQ-243). Counts are u32 (R-329).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct History {
    /// The number of edits undo can step back through.
    pub undo_depth: u32,
    /// The number of undone edits redo can reapply.
    pub redo_depth: u32,
}

/// The precision events, GUI-sized (gui_state_contract §2, R-54; deep_zoom §2; scheduler contract Part 4).
#[derive(Clone, Debug, PartialEq)]
pub struct Precision {
    /// Whether `DECODE_SWITCHOVER` has fired on visible quads.
    pub decode_switchover: bool,
    /// Whether `AT_F32_FLOOR` has been hit.
    pub at_f32_floor: bool,
}

/// The GUI-sized frame summary, named from the frame record (gui_state_contract §2; telemetry §5; RQ-243). Each value
/// is `None` until a frame loop fills it; the real engine's is TASK-M8-05's, so its values are `None` until then, and
/// the GUI draws "—" for `None`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameSummary {
    /// The frame record's `frame_ms` at the latest frame: its wall-clock milliseconds.
    pub frame_ms: Option<f64>,
    /// 1000 / the mean `frame_ms` of the frames since the previous snapshot.
    pub fps: Option<f64>,
    /// The frame record's `leaf_count` at the latest frame.
    pub quad_count: Option<u64>,
    /// The frame record's `live_memory` at the latest frame, the tile cache not added.
    pub live_memory: Option<LiveMemory>,
}

/// The footer's memory readout: the `heap` and `gpu` pools' live bytes of the frame record's `live_memory`, the
/// `tile_cache` pool not added (gui_state_contract §2; telemetry §5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LiveMemory {
    /// `live_memory.heap.bytes`.
    pub heap_bytes: u64,
    /// `live_memory.gpu.bytes`.
    pub gpu_bytes: u64,
}
