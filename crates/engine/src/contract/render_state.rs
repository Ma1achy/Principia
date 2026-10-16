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
    /// The cursor over the figure, which the hover trace is drawn from (render_gui_spec §G2; trajectory_viewing §3):
    /// written through a `SetField` marked "no history", as the playhead's advance is (R-101).
    pub hover: Hover,
}

/// The stain graph: nodes, wires, per-node params (gui_state_contract §2, §5): the free, typed node graph of
/// render_gui_spec Part II (R-64), its edits checked ([`crate::stain`]). Its `SetField` path is
/// [`RenderField::StainGraph`](crate::contract::set_field::RenderField::StainGraph): one edit of the whole graph,
/// undoable (R-69). Added for the dev GUI's Stain editor (R-390, "Contract fields"); TASK-M7-11 keeps REQ-GUI-016.
pub use crate::stain::{
    GraphError, Node, NodeId, NodeKind, Occupant, PortType, StainGraph, Uniform, UniformType, Wire,
};

/// The overlay set (gui_state_contract §2): render_gui_spec §G2's fourteen toggles of Overlays ▾, each on or off, by
/// group. Its `SetField` path is [`RenderField::Overlays`](crate::contract::set_field::RenderField::Overlays): one
/// toggle, or "all off", is one edit of the whole set, undoable (R-69). Added for the dev GUI's Overlays menu (R-390,
/// "Contract fields"); each overlay's drawing is its own requirement's (Part II §12.1's tiers; TASK-M8-08).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Overlays {
    /// Quadtree: tile bounds.
    pub tile_bounds: bool,
    /// Quadtree: depth colouring.
    pub depth_colouring: bool,
    /// Quadtree: priority heatmap.
    pub priority_heatmap: bool,
    /// Quadtree: `refine_flagged`.
    pub refine_flagged: bool,
    /// Quadtree: split / merge activity.
    pub split_merge_activity: bool,
    /// Integration: substep density.
    pub substep_density: bool,
    /// Integration: substep-cap hits.
    pub substep_cap_hits: bool,
    /// Integration: NaN / invalid pixels.
    pub nan_invalid_pixels: bool,
    /// Integration: energy drift.
    pub energy_drift: bool,
    /// Chart: the forbidden region, the chart's domain function (R-26).
    pub forbidden_region: bool,
    /// Chart: the grid.
    pub grid: bool,
    /// Chart: the Burrau lattice (primitive triples).
    pub burrau_lattice: bool,
    /// Stain: class edges.
    pub class_edges: bool,
    /// Stain: `t_end` contours.
    pub t_end_contours: bool,
}

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

/// The cursor over the figure (gui_state_contract §2): where the pointer is on the figure, in the chart's `(s, t)`,
/// each in `[0, 1]` and `t` Y-up (chart_decoder_contract Part 3), or none when the pointer is off the figure. The engine
/// reads it to integrate the cursor's IC and draw its hover trace (trajectory_viewing §3), and the snapshot carries
/// that IC's sample ([`Sample`](crate::contract::sample::Sample)). Its `SetField` path is
/// [`RenderField::Hover`](crate::contract::set_field::RenderField::Hover), always marked "no history": a hover is
/// never undoable, as playback is not (R-101). Added for the dev GUI's hover label and Trajectory panel (R-390,
/// "Contract fields"); TASK-M8-10 keeps REQ-GUI-085 and REQ-GUI-086.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hover {
    /// The cursor's `(s, t)` on the figure; `None` off it.
    pub at: Option<[f64; 2]>,
}
