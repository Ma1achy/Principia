//! `SimConfig`, the sim key (gui_state_contract §2): a change to it re-integrates. Each field is one group §2
//! lists; each group's fields come from the contract that owns them, added by the requirements that need them
//! (R-133). At M0 a group is named and empty; the plane and the lock gained theirs with the dev GUI's Manifold view
//! (R-390, "Contract fields").
//!
//! `SimConfig` and its groups serialise in the one canonical serialisation, [`canonical`](super::canonical)
//! (gui_state_contract §2, R-309), JCS (R-318): an object keyed by its field names, an empty group as `{}`. That text is
//! the ViewState record ([`view_state`](super::view_state); R-483 H2).

use std::collections::BTreeMap;

use kernel::generated::links::{LinkParams, Selection};
use ledger::constants::{MU_MAX, Q_MAX, SLICE_AXIS_Z_Q3, WINDOW_SQUARE_CENTRE, WINDOW_SQUARE_HALF};
use ledger::links::{self as registry, Slot};
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
    /// Each block's link id and tunable parameter values (R-483 X1).
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

/// Chart id and params (gui_state_contract §2). The view's window on a nonlinear chart's own parameters is chart
/// construction among them, held here (R-69; chart_decoder_contract Part 4, "The lock's construction, defined";
/// REQ-SYS-011), so the ViewState record restores it and one undo of a lock restores it with `z₀` and the lock group.
/// The other chart params join with the requirements that need them (R-133).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Chart {
    /// The window on a nonlinear chart's parameters; the affine chart reads `(z₀, q₁, q₂)` and not this.
    pub window: Window,
}

/// The view's window on a nonlinear chart's own parameters (chart_decoder_contract Part 4, "The lock's construction,
/// defined"): the pixel `(s, t)` is the chart point `c + ((2s−1)·h_u, (2t−1)·h_v)`, Part 3's placement of the
/// addressing space applied to the chart's coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Window {
    /// The centre `c`, the chart point of the pixel `(½, ½)`.
    pub centre: [f64; 2],
    /// The half-widths `(h_u, h_v)`.
    pub half: [f64; 2],
}

impl Window {
    /// The window over the chart's whole square `[0,1]²`, where the chart point of the pixel `(s, t)` is `(s, t)`: `Φ`'s
    /// own domain (chart_decoder_contract Part 3), read from the register.
    pub const SQUARE: Window = Window {
        centre: [WINDOW_SQUARE_CENTRE.number(), WINDOW_SQUARE_CENTRE.number()],
        half: [WINDOW_SQUARE_HALF.number(), WINDOW_SQUARE_HALF.number()],
    };

    /// The chart point of the pixel `(s, t)`.
    pub fn at(&self, s: f64, t: f64) -> [f64; 2] {
        [
            self.centre[0] + (s + s - 1.0) * self.half[0],
            self.centre[1] + (t + t - 1.0) * self.half[1],
        ]
    }
}

/// A point or a direction of the latent space `ℝ⁸`, in chart_decoder_contract Part 2's block order: `z[0:2]`
/// configuration, `z[2:6]` momentum, `z[6:8]` mass. Its length is the type's own, as a struct's field widths are
/// (`[u32; 4]`, `ICDescriptor`'s twelve floats): a structural size the corpus defines, not a register constant
/// (dd_generation_root §3.8 registers measured or derived numbers).
pub type Latent = [f64; 8];

/// The latent dimension, `z ∈ ℝ⁸` (chart_decoder_contract Part 2): read from [`Latent`], never written again.
pub const LATENT_DIM: usize = std::mem::size_of::<Latent>() / std::mem::size_of::<f64>();

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

/// Slice values (gui_state_contract §2): the slice frame, added for the dev GUI's Tilt section (R-457).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Slice {
    /// The slice frame: the slice normal and the two tilt targets.
    pub frame: SliceFrame,
}

/// The slice frame (gui_state_contract §2; R-457 E3, R-459): three latent dimension indices, in
/// chart_decoder_contract Part 2's block order. `normal` is the slice normal, the compass cube's D axis, the dimension
/// the slice moves through; `tilt_x` and `tilt_y` are the dimensions `τ₁` and `τ₂` tilt into. Like the lock it is chart
/// construction, so it is undoable (R-69); no kernel reads it (the kernel reads `z₀`, `q₁`, `q₂`), so an edit of it
/// alone re-integrates nothing. Each index is a `u32`, as every serialised index is (R-329).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SliceFrame {
    /// The slice normal.
    pub normal: u32,
    /// The dimension `τ₁` tilts the horizontal axis `q₁` into.
    pub tilt_x: u32,
    /// The dimension `τ₂` tilts the vertical axis `q₂` into.
    pub tilt_y: u32,
}

/// `z_q3`'s index, the GUI's slice axis before R-459 and the slice frame's default for a plane that does not show it
/// (gui_state_contract §2; chart_decoder_contract Part 2: `z[2:6]` is the momentum block, `z_q3` its last), read from
/// the register.
pub const Z_Q3: u32 = SLICE_AXIS_Z_Q3.number() as u32;

impl SliceFrame {
    /// The frame with the normal and both targets along `axis`: gui_state_contract §2's defaults, all three the slice
    /// axis, so the targets give the one shared target the GUI tilted into before R-457.
    pub const fn along(axis: u32) -> Self {
        Self {
            normal: axis,
            tilt_x: axis,
            tilt_y: axis,
        }
    }
}

/// Lock flag, `z_locked` anchor, `δ` excursion (gui_state_contract §2; R-69): the lock is CPU-side chart
/// construction (chart_decoder_contract Part 4, "The lock"). The flag and the anchor are added for the dev GUI's lock
/// (R-390, "Contract fields"). The excursion is held by the stored pair, `δ = z₀ − z_locked` ([`Lock::excursion`]),
/// the anchor stored, not re-derived (chart_decoder_contract Part 4, "The lock's construction, defined"; R-72): a slice
/// step or a slider move edits `z₀` alone, snap-back sets `z₀` to the anchor bit for bit, and `δ` has no field of its
/// own that could disagree with `z₀`. TASK-M2-23 keeps the lock's requirements (`crate::lock`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lock {
    /// The lock flag.
    pub locked: bool,
    /// The anchor `z_locked`; meaningful while `locked`.
    pub z_locked: Latent,
}

impl Lock {
    /// The excursion `δ = z₀ − z_locked` of the centre `z0` from the anchor while locked; `None` while free.
    pub fn excursion(&self, z0: &Latent) -> Option<Latent> {
        self.locked
            .then(|| std::array::from_fn(|i| z0[i] - self.z_locked[i]))
    }
}

/// The links (gui_state_contract §2; R-483 X1, H1; R-106: the chart's per-block registry entries): one choice per row
/// of the Decoder window, the four momentum controls sharing the Momenta row's (R-479 E1). It is the one place the link
/// ids and their parameter values live; every destination reads them through the canonical serialisation. Every block
/// and every parameter is written even at its default: no field here has `skip_serializing_if` or a serde default
/// (R-483 X2). A text is read only if each block names a registry entry onto that block's codomain and lists exactly
/// that entry's tunable parameters ([`Links::check`]), so a record with a parameter stripped is refused, never read
/// under a default.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, try_from = "LinksText")]
pub struct Links {
    /// The mass block's link, `z[6:8]`.
    pub mass: LinkChoice,
    /// The configuration angle `α`'s link, `z[0]`.
    pub alpha: LinkChoice,
    /// The configuration angle `β`'s link, `z[1]`.
    pub beta: LinkChoice,
    /// The four free momentum controls' link, `z[2:6]`.
    pub momenta: LinkChoice,
}

/// One block's link (R-483 H1): `id` the registry entry's name, `params` each tunable parameter of that entry with its
/// value (`{"mu_max":5}`), a map, `{}` for an entry with none.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkChoice {
    /// The registry entry's name (`kernel::generated::links::NAMES`).
    pub id: String,
    /// Each tunable parameter's name and value.
    pub params: BTreeMap<String, f64>,
}

/// [`Links`] as a text holds it, before [`Links::check`].
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LinksText {
    mass: LinkChoice,
    alpha: LinkChoice,
    beta: LinkChoice,
    momenta: LinkChoice,
}

impl TryFrom<LinksText> for Links {
    type Error = String;

    fn try_from(text: LinksText) -> Result<Self, String> {
        let links = Links {
            mass: text.mass,
            alpha: text.alpha,
            beta: text.beta,
            momenta: text.momenta,
        };
        links.check().map(|()| links)
    }
}

/// The blocks, by their `Links` field's name, and the registry slot whose codomain each takes: the momenta the first
/// momentum control's, which the other three share.
const BLOCKS: [(&str, &str); 4] = [
    ("mass", "mass"),
    ("alpha", "alpha"),
    ("beta", "beta"),
    ("momenta", "q0"),
];

/// The registry slot named `name`.
fn slot(name: &str) -> Slot {
    registry::slots()
        .into_iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("the link registry has no slot `{name}`"))
}

/// `id`'s choice at the defaults of its tunable parameters (dd_generation_root §3.9's table).
fn at_defaults(id: &str) -> LinkChoice {
    LinkChoice {
        id: id.to_owned(),
        params: registry::tunables()
            .iter()
            .filter(|t| t.entry == id)
            .map(|t| (t.param.to_owned(), t.default))
            .collect(),
    }
}

impl Links {
    /// The default links, each block's slot default (dd_decoder §3), at its parameters' defaults (R-481 A2).
    pub fn defaults() -> Links {
        let [mass, alpha, beta, momenta] = BLOCKS.map(|(_, s)| at_defaults(slot(s).default));
        Links {
            mass,
            alpha,
            beta,
            momenta,
        }
    }

    /// The links of the selection `S`, at its parameters' defaults; `Err` where the four momentum controls' links
    /// differ, which one Momenta row cannot hold (R-479 E1).
    pub fn of<S: Selection>() -> Result<Links, String> {
        let [mass, alpha, beta, q0, q1, q2, q3] = S::LINK_IDS;
        if [q1, q2, q3].iter().any(|q| *q != q0) {
            return Err(format!(
                "the momentum controls' links differ ({q0}, {q1}, {q2}, {q3}): one Momenta row holds one (R-479 E1)"
            ));
        }
        Ok(Links {
            mass: at_defaults(mass),
            alpha: at_defaults(alpha),
            beta: at_defaults(beta),
            momenta: at_defaults(q0),
        })
    }

    /// Each block with its `Links` field's name, in `BLOCKS`' order.
    pub fn blocks(&self) -> [(&'static str, &LinkChoice); 4] {
        [
            (BLOCKS[0].0, &self.mass),
            (BLOCKS[1].0, &self.alpha),
            (BLOCKS[2].0, &self.beta),
            (BLOCKS[3].0, &self.momenta),
        ]
    }

    /// `Ok` if each block names a registry entry onto its slot's codomain (REQ-GEN-013) and lists exactly that entry's
    /// tunable parameters, none missing and none extra (R-483 X2); otherwise each problem, naming its block.
    pub fn check(&self) -> Result<(), String> {
        let mut bad = Vec::new();
        for ((block, choice), (_, s)) in self.blocks().into_iter().zip(BLOCKS) {
            if let Err(e) = registry::select(&slot(s), &choice.id) {
                bad.push(format!("links.{block}: {e}"));
                continue;
            }
            let want: Vec<String> = at_defaults(&choice.id).params.into_keys().collect();
            let got: Vec<String> = choice.params.keys().cloned().collect();
            if got != want {
                bad.push(format!(
                    "links.{block}: `{}` takes the parameters {want:?}, and the record gives {got:?}: none may be \
                     missing, none extra (R-483 X2)",
                    choice.id
                ));
            }
        }
        match bad.is_empty() {
            true => Ok(()),
            false => Err(bad.join("; ")),
        }
    }

    /// The tunable parameters the kernel's `_at` functions read (R-481 G7): `mu_max` from the mass block's choice and
    /// `q_max` from the momenta's. A parameter the chosen entry does not take is read by no link of that block, and
    /// is left at its default.
    pub fn link_params(&self) -> LinkParams<f64> {
        let mut p = LinkParams::<f64>::DEFAULT;
        if let Some(&v) = self.mass.params.get(MU_MAX.name) {
            p.mu_max = v;
        }
        if let Some(&v) = self.momenta.params.get(Q_MAX.name) {
            p.q_max = v;
        }
        p
    }
}

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

/// `T`/`dt`/thresholds/`eps` (gui_state_contract §2). The horizon `T` was added for the dev GUI's Time, whose scrubber
/// spans `[0, T]` (R-404's `TimeStep`), and its hover label, "at t of t_max" (render_gui_spec §G2) (R-390, "Contract
/// fields"); `dt`, the thresholds and `eps` join with the requirements that need them (R-133).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Horizon {
    /// The single global horizon `T` (canonical_spec; dd_simstate_payload §2): reaching it without escape or collision
    /// is `bounded`. The GUI's `t_max`.
    pub t: f64,
}

/// Collision radius `r_coll` (gui_state_contract §2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Collision {}

/// Quality settings (gui_state_contract §2, §6).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Quality {}
