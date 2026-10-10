//! `SimConfig`, the sim key (gui_state_contract §2): a change to it re-integrates. Each field is one group §2
//! lists; each group's fields come from the contract that owns them, added by the requirements that need them
//! (R-133). At M0 a group is named and empty; the plane and the lock gained theirs with the dev GUI's Manifold view
//! (R-390, "Contract fields").
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

/// The latent dimension: the controls `z ∈ ℝ⁸` (chart_decoder_contract Part 2).
pub const LATENT_DIM: usize = 8;

/// A point or a direction of the latent space `ℝ⁸`, in chart_decoder_contract Part 2's block order: `z[0:2]`
/// configuration, `z[2:6]` momentum, `z[6:8]` mass.
pub type Latent = [f64; LATENT_DIM];

/// `z₀`, `q₁`, `q₂` (gui_state_contract §2): the affine slice `z(s,t) = z₀ + (2s−1) q₁ + (2t−1) q₂`
/// (chart_decoder_contract Part 3). The view state is this triple: navigation edits it and nothing else (Part 4).
/// Added for the dev GUI's Manifold view (R-390, "Contract fields"); TASK-M2-22 keeps its requirements.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plane {
    /// `z₀`, the slice centre: pan and slice edit it.
    pub z0: Latent,
    /// `q₁`, the basis vector along `s`: zoom, tilt and rotation edit it with `q₂`.
    pub q1: Latent,
    /// `q₂`, the basis vector along `t`.
    pub q2: Latent,
}

/// Slice values (gui_state_contract §2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Slice {}

/// Lock flag, `z_locked` anchor, `δ` excursion (gui_state_contract §2; R-69): the lock is CPU-side chart
/// construction (chart_decoder_contract Part 4, "The lock"). The flag and the anchor are added for the dev GUI's lock
/// (R-390, "Contract fields"); the excursion is read as `z₀ − z_locked`, the anchor stored, not re-derived, and
/// TASK-M2-23 keeps the lock's requirements.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lock {
    /// The lock flag.
    pub locked: bool,
    /// The anchor `z_locked`; meaningful while `locked`.
    pub z_locked: Latent,
}

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
