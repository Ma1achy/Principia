//! The snapshot, the only way out (gui_state_contract §1, §2): GUI-sized state only — view state, the current tier,
//! a few scalars — never engine-sized state (the payload, the quad tree, the reductions), which is only summarised
//! across. The engine posts it throttled to ~10 Hz (caching contract Part 6a, R-94).

use crate::contract::render_state::RenderState;
use crate::contract::sim_config::SimConfig;

/// A GUI-sized state snapshot (gui_state_contract §1, §2).
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
}

/// The current tier (gui_state_contract §1); its fields come from the contract that owns it (R-133).
pub struct Tier {}

/// The depth of the contract's one undo/redo history (gui_state_contract §2, R-52); its fields come with it (R-133).
pub struct History {}

/// The precision events, GUI-sized (gui_state_contract §2, R-54; deep_zoom §2; scheduler contract Part 4).
pub struct Precision {
    /// Whether `DECODE_SWITCHOVER` has fired on visible quads.
    pub decode_switchover: bool,
    /// Whether `AT_F32_FLOOR` has been hit.
    pub at_f32_floor: bool,
}
