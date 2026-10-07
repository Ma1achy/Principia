//! `SimConfig`, the sim key (gui_state_contract §2): a change to it re-integrates. Each field is one group §2
//! lists; each group's fields come from the contract that owns them, added by the requirements that need them
//! (R-133). At M0 a group is named and empty.
//!
//! `SimConfig` and its groups serialise in the one canonical serialisation, [`canonical`](super::canonical)
//! (gui_state_contract §2, R-309), JCS (R-318): an object keyed by its field names, an empty group as `{}`.

use serde::{Deserialize, Serialize};

/// The sim key (gui_state_contract §2): every sim-side knob, as a typed field.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SimConfig {
    /// Chart id and params.
    pub chart: Chart,
    /// `z₀`, `q₁`, `q₂`: the slice anchor and the in-plane basis.
    pub plane: Plane,
    /// The slice values.
    pub slice: Slice,
    /// The lock flag, the `z_locked` anchor and the `δ` excursion; lock is chart construction (R-69).
    pub lock: Lock,
    /// The link ids.
    pub links: Links,
    /// The integrator occupant.
    pub integrator: Integrator,
    /// The kernel variant: physics, or colour_composition Appendix A's bring-up mode, "the only sim-key item" of
    /// colour_composition §0 (RQ-221). A change re-integrates, as for any sim-key field (caching contract Part 2).
    pub kernel_variant: KernelVariant,
    /// `T`, `dt`, the thresholds and `eps`.
    pub horizon: Horizon,
    /// The collision radius `r_coll`.
    pub collision: Collision,
    /// The quality settings (§6). Not every quality field is sim-key: `render_scale`, `lock_to_native`,
    /// `MAX_REL_DEPTH` and `checkerboard_mode` invalidate nothing.
    pub quality: Quality,
}

/// Chart id and params (gui_state_contract §2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Chart {}

/// `z₀`, `q₁`, `q₂` (gui_state_contract §2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plane {}

/// Slice values (gui_state_contract §2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Slice {}

/// Lock flag, `z_locked` anchor, `δ` excursion (gui_state_contract §2; R-69).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lock {}

/// Link ids (gui_state_contract §2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Links {}

/// Integrator occupant (gui_state_contract §2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Integrator {}

/// The kernel variant, each a baked kernel (lowering Part 3, "Compute side"; R-41): the physics kernel, or the one
/// debug mode, colour_composition Appendix A's bring-up mode (R-75; `kernel::bringup`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KernelVariant {
    /// The physics kernel.
    Physics,
    /// The bring-up mode: the kernel writes a known pattern instead of physics.
    BringUp,
}

/// `T`/`dt`/thresholds/`eps` (gui_state_contract §2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Horizon {}

/// Collision radius `r_coll` (gui_state_contract §2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Collision {}

/// Quality settings (gui_state_contract §2, §6).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Quality {}
