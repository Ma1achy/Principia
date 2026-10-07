//! `RenderState`, the render key (gui_state_contract §2): a change to it recolours, live. Each field is one group §2
//! lists; each group's fields come from the contract that owns them, added by the requirements that need them
//! (R-133). At M0 a group is named and empty.
//!
//! `RenderState` and its groups serialise in the one canonical serialisation, [`canonical`](super::canonical)
//! (gui_state_contract §2, R-309), JCS (R-318): an object keyed by its field names, an empty group as `{}`.

use serde::{Deserialize, Serialize};

/// The render key (gui_state_contract §2): every render-side knob, as a typed field.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenderState {
    /// The stain graph: nodes, wires and per-node params (§5).
    pub stain_graph: StainGraph,
    /// The overlay set.
    pub overlays: Overlays,
    /// The palette and compaction params.
    pub palette: Palette,
    /// The playhead `t`; the GUI's clock advances it each frame through a `SetField` marked "no history" (R-101).
    pub playhead: Playhead,
}

/// The stain graph: nodes, wires, per-node params (gui_state_contract §2, §5).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StainGraph {}

/// The overlay set (gui_state_contract §2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Overlays {}

/// Palette and compaction params (gui_state_contract §2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Palette {}

/// The playhead `t` (gui_state_contract §2): its `SetField` path is
/// [`RenderField::Playhead`](crate::contract::set_field::RenderField::Playhead) (RQ-243).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Playhead {
    /// The playhead's simulation time `t`: f64, as the f64 inspector and the playhead's fixed-`dt` sum need (RQ-243).
    pub t: f64,
}
