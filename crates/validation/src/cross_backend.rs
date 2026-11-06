//! The kernel cross-backend check, the arm for now (TASK-M4-27; REQ-VAL-347; R-686, R-690, R-649; parity contract § 4):
//! decode's and canon's branch outcomes on identical inputs, on NVIDIA Vulkan, Metal, lavapipe and CPU f32, the
//! disagreement rate per backend pair, and R-690's driver condition tested per driver. `cargo xtask gate
//! cross-backend-branches` runs it.
//!
//! **A measurement** (R-649), on tier (a), the GPU explorer. It has no outside number, it blocks nothing, and no
//! tolerance or threshold enters it. Only the control records assert ([`CrossBackendGate`]). [`record`] asserts nothing,
//! and a record under [`compare`] has no `"expected"` and can't fail the gate at any rate.
//!
//! **Domain** (pitfalls §10): fixed inputs through two stages. The GPU runs no march yet, so a clean table says nothing
//! about collision or escape decisions along a trajectory, or about whether the missing `NoContraction` matters for the
//! real kernel; TASK-M4-28 does that. [`DOMAIN`] is the report's header.
//!
//! **The inputs** ([`build_inputs`]) are those `crates/engine/tests/canon_gpu.rs` and
//! `crates/engine/tests/decode_cross_backend.rs` build, seeds fixed, with #393's from
//! `crates/validation/tests/qa_TASK-M2-25_gpu.rs`: this file's own copy of those builders, since a binary can't call a
//! test file and those files stay as they are, the qa file above all (R-290). They are written once by `--write-inputs`,
//! no GPU, under `fixtures/gates/cross-backend-branches/inputs/`, committed, and every backend's `--record` reads them
//! from there, since the builders call the platform's `sin_cos` and `powf`, whose bits need not match on Windows. A
//! sweep built on a GPU's own decodes (`canonicaliser_noop_gpu`), or on a backend's own `V₀`, takes CPU f32's here, for
//! the same reason. The states are binary, the f32 words as they are: as text they would be some 30 MB (applied per
//! R-369).
//!
//! **A record** ([`Record`]) is one file per backend, keyed by the commit, the adapter line, the driver version, the
//! compute fast-math setting, the hardware label if any and the inputs' hash ([`Inputs::hash`]). It holds one row per
//! input and branch, grouped by branch and compiled entry ([`Group`]), which is where a row's entry, branch and kind
//! are named, its words packed as hex, canon's operands once per input: at some 1.7 million rows, a JSON object per row
//! would be gigabytes (applied per R-369). A row's outcome in the rate is the stage's where the stage emits it, otherwise
//! the read-back's ([`Row::outcome`]), and the row names which ([`Row::source`]). `stage_canon_canonicalise` emits only
//! the skip (`rescaled`), `ρ = 0` (`clamped`) and the mirror; the decode entries only the tag, and an earlier block's tag
//! masks a later one's ([`Listed::stage_outcome`]). `rescaled` is `¬(skip ∨ coincidence)`, so the row named `skip` reads
//! that word, and the coincidence is among its input words, so a row whose decision moved with the coincidence's is not
//! read as a comparison fork (applied per R-369).
//!
//! **The kinds** ([`LIST`]). `+ and ×` is floating `+`, `−` and `×`, with what rounds nothing and a rewrite can't move:
//! negation and `abs`, exact power-of-two scalings, bit reads and masks, integer operations, selects and comparisons.
//! `other operation` holds an operation with an ulp bound rather than correct rounding (division, `√`, `sin`, `cos`,
//! `atan2`, a rounding builtin) or `fma`. R-438's select is the encode's `β`, which these inputs do not run; the decode's
//! reading of `λ̃ = 0` is `rho_angle`'s, R-437's rows (applied per R-369). A kind is the expression's own, read at its
//! own operands, not at how they were reached: `rho_angle`'s `ρ̃₀λ̃₁ − ρ̃₁λ̃₀` and `ρ̃₀λ̃₀ + ρ̃₁λ̃₁` are `+`, `−` and `×` on the
//! emitted `ρ̃`, `λ̃`, the form R-690 exists for. Each `+ and ×` input names the kernel function
//! the arbiter evaluates, at f32, with no second copy ([`Listed::evaluate`]).
//!
//! **The levels and the standing** ([`compare`]): the parity contract's § 4 and the task's notes. Three levels, on a row
//! of kind `+ and ×`, within one named entry: level 1, operands differ, no evidence; level 2, operands identical and bits
//! differ, the driver fails, with or without a fork; level 3, operands and bits identical and the outcome differs, a
//! comparison fork, and it fails too. A stage outcome that differs where the read-back's operands, bits and outcome
//! don't is a **production-only difference**: it counts in the rate, takes no level and earns no word. The arbiter is
//! the written form on CPU f32, at its own run's operands, or, where two GPUs share operands that run didn't produce,
//! at those, which the host evaluates. A compare with no CPU f32 record has no run of its own, so the host evaluates
//! every shared operand (applied per R-369). A pair's outcome is PASS at 0 disagreements with neither driver recorded
//! unverified, not supported, and otherwise UNEXPLAINED until a measurement shows the mechanism: nothing in a record
//! shows one, so the word EXPLAINED DIVERGENCE is the reader's to earn, in the PR that shows it (applied per R-369).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use engine::contract::fast_math::{FastMath, StageMode};
use engine::decode_dispatch::{
    uniform_words, BranchReadCanon, BranchReadStage, CanonStage, DecodeStage, CANONICALISE_ENTRY,
};
use kernel::canon::{canonicalise, scale_step, Canonical};
use kernel::chart::invariant::Invariant;
use kernel::chart::mixed::{Alpha, ZMu1};
use kernel::chart::{
    Burrau, BurrauThetaDeltaM, BurrauThetaK, BurrauThetaLz, EuclidPlane, InvariantLE, InvariantLK,
    Latent, MassSimplex, Mixed, ShapeMap, ShapeSphere,
};
use kernel::decode::{decode, Ic};
use kernel::generated::constants::ChartConstants;
use kernel::generated::links::{Defaults, Link, LinkReal, StickBreaking};
use kernel::real::IeeeBits;
use kernel::stage::branch_read::{self as read, BranchVariant};
use kernel::stage::decode::{
    self as stage, BurrauDefaults, BurrauThetaDeltaMDefaults, BurrauThetaKDefaults,
    BurrauThetaLzDefaults, DecodeMode, DecodeUniforms, DecodeVariant, EuclidPlaneDefaults,
    InvariantLEDefaults, InvariantLKDefaults, LatentDefaults, LatentStickBreaking,
    MassSimplexDefaults, MixedMu1AlphaDefaults, ShapeSphereDefaults, UniformChart,
};
use serde::{Deserialize, Serialize};

use crate::decode_f32_eps::{golden, interior, kernel_wgsl, Fuzz, SplitMix};
use crate::decode_preset::uniforms as held_uniforms;
use crate::gate::report::Verdict;
use crate::gate::{MeasurementConfig, MeasurementControl, MeasurementGate};
use crate::gpu::{AdapterInfo, GpuHarness};

/// The gate's name, its directory under `fixtures/gates/`.
pub const NAME: &str = "cross-backend-branches";

/// The report's header: its domain, and that it does not answer the trajectory question (pitfalls §10).
pub const DOMAIN: &str = "**Domain.** Decode's and canon's branch outcomes on fixed inputs, read per backend, on tier (a), \
the GPU explorer: a measurement (R-649), blocking nothing, no tolerance or threshold in it. **It does not answer the \
trajectory question:** these are fixed inputs through two stages and the GPU runs no march yet, so a clean table says \
nothing about collision or escape decisions along a trajectory, or about whether the missing `NoContraction` matters \
for the real kernel (TASK-M4-28 does that). The claim is for identical inputs only (PIT-10).";

/// The backends the check runs on, in the report's order.
pub const BACKENDS: [&str; 4] = ["NVIDIA Vulkan", "Metal", "lavapipe", "CPU f32"];

/// CPU f32's backend name, the arbiter's.
pub const CPU: &str = "CPU f32";

/// The entry canon's rows name.
pub const CANON_ENTRY: &str = engine::decode_dispatch::BRANCH_READ_CANON_ENTRY;

/// A branch input's kind, read on what a driver can round (R-690; the module's note).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    /// `+ and ×`: floating `+`, `−`, `×` and what rounds nothing.
    #[serde(rename = "+ and ×")]
    AddMul,
    /// `other operation`: an operation with an ulp bound rather than correct rounding, or `fma`.
    #[serde(rename = "other operation")]
    Other,
}

impl Kind {
    /// The kind's words in a report.
    pub fn word(self) -> &'static str {
        match self {
            Kind::AddMul => "+ and ×",
            Kind::Other => "other operation",
        }
    }
}

/// What a stage emits of a branch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Emits {
    /// `Canonical::rescaled`.
    Rescaled,
    /// `Canonical::clamped`.
    Clamped,
    /// `Canonical::mirrored`.
    Mirrored,
    /// The decode stage's tag word, raised when it is this.
    Tag(u32),
    /// Nothing: the branch reaches a record only through the read-back.
    Nothing,
}

/// The kernel function the arbiter evaluates a `+ and ×` input through.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arbiter {
    /// `branch_read::write_canon`.
    Canon,
    /// `branch_read::write_m01`.
    M01,
    /// `branch_read::write_feasibility`.
    Feasibility,
    /// `branch_read::write_alpha_seed`.
    AlphaSeed,
    /// `branch_read::write_nu_seed`.
    NuSeed,
    /// `branch_read::write_angle`.
    Angle,
}

/// One branch: its name, its slot in the read-back's words, its kind and what emits it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Listed {
    /// The branch's name in a row.
    pub name: &'static str,
    /// Canon's, not a decode's.
    pub canon: bool,
    /// Its slot: canon's outcome word, a decode's `present` word.
    pub at: usize,
    /// Its own operand words; canon's rows share the sample's.
    pub operands: usize,
    /// Its input's bit words.
    pub bits: usize,
    /// Its kind.
    pub kind: Kind,
    /// The written form, for the report.
    pub form: &'static str,
    /// What the stage emits of it.
    pub stage: Emits,
    /// The kernel function the arbiter evaluates, for a `+ and ×` input.
    pub arbiter: Option<Arbiter>,
}

const fn canon_row(
    name: &'static str,
    at: usize,
    bits: usize,
    form: &'static str,
    stage: Emits,
) -> Listed {
    Listed {
        name,
        canon: true,
        at,
        operands: 0,
        bits,
        kind: Kind::AddMul,
        form,
        stage,
        arbiter: Some(Arbiter::Canon),
    }
}

const fn exact_row(
    name: &'static str,
    at: usize,
    shape: (usize, usize),
    form: &'static str,
    stage: Emits,
    arbiter: Arbiter,
) -> Listed {
    Listed {
        name,
        canon: false,
        at,
        operands: shape.0,
        bits: shape.1,
        kind: Kind::AddMul,
        form,
        stage,
        arbiter: Some(arbiter),
    }
}

const fn other_row(name: &'static str, at: usize, form: &'static str, stage: Emits) -> Listed {
    Listed {
        name,
        canon: false,
        at,
        operands: 0,
        bits: 0,
        kind: Kind::Other,
        form,
        stage,
        arbiter: None,
    }
}

/// The list: every branch a record holds, its kind read from its written form at `crates/kernel/src/canon.rs`,
/// `decode/mass.rs`, `decode/invariant.rs` and `chart/burrau.rs`, and, for the poles and `η_E`, at what their inputs are
/// reached through.
pub const LIST: [Listed; 19] = [
    canon_row(
        "skip",
        read::SKIP_AT,
        read::SKIP_BITS,
        "canon: Wide |I_in − 1| > τ_I, with the coincidence",
        Emits::Rescaled,
    ),
    canon_row(
        "coincidence",
        read::COINCIDENCE_AT,
        read::COINCIDENCE_BITS,
        "canon: Wide ρ = 0 and d02 = 0",
        Emits::Nothing,
    ),
    canon_row(
        "rho_zero",
        read::GUARD_AT,
        read::GUARD_BITS,
        "canon: Wide ρ = 0",
        Emits::Clamped,
    ),
    canon_row(
        "mirror",
        read::MIRROR_AT,
        read::MIRROR_BITS,
        "canon: Wide c < 0 and m₂c² > δ_λ²·M₀₁·‖ρ‖²·d²",
        Emits::Mirrored,
    ),
    canon_row(
        "rho_x_zero",
        read::RHO_X_AT,
        read::AXIS_BITS,
        "canon: scaled ρ_x = 0, an exact scaling of a Wide mantissa",
        Emits::Nothing,
    ),
    canon_row(
        "rho_y_zero",
        read::RHO_Y_AT,
        read::AXIS_BITS,
        "canon: scaled ρ_y = 0, an exact scaling of a Wide mantissa",
        Emits::Nothing,
    ),
    exact_row(
        "M01_TINY",
        read::M01_AT,
        read::M01_SHAPE,
        "decode/mass.rs: Wide ε > M₀₁ on the emitted masses",
        Emits::Tag(1),
        Arbiter::M01,
    ),
    exact_row(
        "INVARIANT_INFEASIBLE",
        read::FEASIBILITY_AT,
        read::FEASIBILITY_SHAPE,
        "decode/invariant.rs: exact 2·K* < L_z², bit reads and columns",
        Emits::Tag(4),
        Arbiter::Feasibility,
    ),
    exact_row(
        "seed_z_alpha",
        read::ALPHA_SEED_AT,
        read::ALPHA_SEED_SHAPE,
        "decode/invariant.rs: z_α ≤ 0 on its sign bits",
        Emits::Nothing,
        Arbiter::AlphaSeed,
    ),
    exact_row(
        "seed_nu",
        read::NU_SEED_AT,
        read::NU_SEED_SHAPE,
        "chart/burrau.rs: 3ν ≤ 1 by the two-sum",
        Emits::Nothing,
        Arbiter::NuSeed,
    ),
    other_row(
        "ENERGY_INFEASIBLE",
        read::ENERGY_INFEASIBLE_AT,
        "decode/energy_norm.rs: E* < V₀, V₀ through √ and division",
        Emits::Tag(2),
    ),
    other_row(
        "ENERGY_UNSCALABLE",
        read::ENERGY_UNSCALABLE_AT,
        "decode/energy_norm.rs: r_min = 0 or the rest start, through √ and division",
        Emits::Tag(3),
    ),
    other_row(
        "pole_rho_ratio",
        read::RATIO_POLE_AT,
        "decode/descriptor.rs: ‖λ̃‖ = 0 (R-436), through √",
        Emits::Nothing,
    ),
    other_row(
        "pole_v0",
        read::COLLISION_POLE_AT,
        "decode/descriptor.rs: r_min = 0 (R-436), through √",
        Emits::Nothing,
    ),
    other_row(
        "pole_virial",
        read::VIRIAL_POLE_AT,
        "decode/descriptor.rs: V₀ non-finite (R-436), through √ and division",
        Emits::Nothing,
    ),
    exact_row(
        "angle_cross_zero",
        read::CROSS_ZERO_AT,
        read::ANGLE_SHAPE,
        "decode/descriptor.rs: ρ̃₀λ̃₁ − ρ̃₁λ̃₀ = 0 (R-437), on the emitted ρ̃, λ̃",
        Emits::Nothing,
        Arbiter::Angle,
    ),
    exact_row(
        "angle_dot_zero",
        read::DOT_ZERO_AT,
        read::ANGLE_SHAPE,
        "decode/descriptor.rs: ρ̃₀λ̃₀ + ρ̃₁λ̃₁ = 0 (R-437), on the emitted ρ̃, λ̃",
        Emits::Nothing,
        Arbiter::Angle,
    ),
    exact_row(
        "angle_dot_negative",
        read::DOT_NEGATIVE_AT,
        read::ANGLE_SHAPE,
        "decode/descriptor.rs: ρ̃₀λ̃₀ + ρ̃₁λ̃₁ < 0, on the emitted ρ̃, λ̃",
        Emits::Nothing,
        Arbiter::Angle,
    ),
    other_row(
        "sphere_inverse_pole",
        read::SPHERE_POLE_AT,
        "chart/shape_sphere.rs: p = q = 0 (R-437, #393's select), reached through sin",
        Emits::Nothing,
    ),
];

impl Listed {
    /// The outcome the stage emits for this branch, on canon's `c`; `None` where it emits none.
    pub fn canon_stage_outcome(&self, c: &Canonical<f32>) -> Option<u32> {
        match self.stage {
            Emits::Rescaled => Some(u32::from(c.rescaled)),
            Emits::Clamped => Some(u32::from(c.clamped)),
            Emits::Mirrored => Some(u32::from(c.mirrored)),
            _ => None,
        }
    }

    /// The outcome the decode stage's tag word `tag` emits for this branch: the tag is the first raised of the mass
    /// block's, the construction's and `η_E`'s, so a tag masks the later blocks', which it says nothing of.
    pub fn stage_outcome(&self, tag: u32) -> Option<u32> {
        fn block(tag: u32) -> u32 {
            match tag {
                1 => 0,
                4 | 5 => 1,
                _ => 2,
            }
        }
        match self.stage {
            Emits::Tag(t) if block(tag) >= block(t) => Some(u32::from(tag == t)),
            _ => None,
        }
    }

    /// `(outcome, operands, bits)` of this decode branch in the sample `words`, `None` where it is not present. For a
    /// canon branch, `(outcome, [], bits)`.
    pub fn read<'a>(&self, words: &'a [u32]) -> Option<(u32, &'a [u32], &'a [u32])> {
        if self.canon {
            let bits = &words[self.at + 1..self.at + 1 + self.bits];
            return Some((words[self.at], &[], bits));
        }
        let present = words[self.at] == 1;
        let operands = self.at + 2;
        let inputs = operands + self.operands;
        present.then(|| {
            (
                words[self.at + 1],
                &words[operands..inputs],
                &words[inputs..inputs + self.bits],
            )
        })
    }

    /// The written form evaluated on the host at f32 at `operands`, through the kernel's own function: `(outcome, bits)`.
    /// `None` for a branch of kind `other operation`.
    pub fn evaluate(&self, operands: &[u32]) -> Option<(u32, Vec<u32>)> {
        let f = |k: usize| f32::from_bits(operands[k]);
        let mut words = vec![0; read::DECODE_WORDS.max(read::CANON_WORDS)];
        match self.arbiter? {
            Arbiter::Canon => {
                let r = [[f(3), f(4)], [f(5), f(6)], [f(7), f(8)]];
                read::write_canon(&mut words, 0, [f(0), f(1), f(2)], r);
            }
            Arbiter::M01 => read::write_m01(&mut words, self.at, f(0), f(1)),
            Arbiter::Feasibility => read::write_feasibility(&mut words, self.at, f(0), f(1)),
            Arbiter::AlphaSeed => read::write_alpha_seed(&mut words, self.at, f(0)),
            Arbiter::NuSeed => read::write_nu_seed(&mut words, self.at, f(0)),
            Arbiter::Angle => read::write_angle(&mut words, 0, [f(0), f(1)], [f(2), f(3)]),
        }
        let (outcome, _, bits) = self.read(&words)?;
        Some((outcome, bits.to_vec()))
    }
}

/// The list's entry named `branch`.
pub fn listed(branch: &str) -> Option<&'static Listed> {
    LIST.iter().find(|l| l.name == branch)
}

/// Words as lowercase hex, eight digits each.
mod hex {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(words: &[u32], s: S) -> Result<S::Ok, S::Error> {
        use std::fmt::Write;
        let mut text = String::with_capacity(8 * words.len());
        for w in words {
            write!(text, "{w:08x}").map_err(serde::ser::Error::custom)?;
        }
        s.serialize_str(&text)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u32>, D::Error> {
        let text = String::deserialize(d)?;
        if !text.len().is_multiple_of(8) {
            return Err(serde::de::Error::custom("hex words: not a multiple of 8"));
        }
        text.as_bytes()
            .chunks(8)
            .map(|c| {
                std::str::from_utf8(c)
                    .ok()
                    .and_then(|h| u32::from_str_radix(h, 16).ok())
                    .ok_or_else(|| serde::de::Error::custom("hex words: not hex"))
            })
            .collect()
    }
}

/// A row's head: its input, the stage's outcome ([`NOT_EMITTED`] where it emits none), the read-back's outcome.
pub const ROW_HEAD: usize = 3;

/// The stage word of a row whose stage emits no outcome.
pub const NOT_EMITTED: u32 = u32::MAX;

/// The rows of one branch through one compiled entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Group {
    /// The compiled entry each row's operands, bits and read-back outcome came from.
    pub entry: String,
    /// The branch.
    pub branch: String,
    /// The branch's kind.
    pub kind: Kind,
    /// The operand words in each row; 0 where the rows read [`Record::canon_operands`].
    pub operands: usize,
    /// The input's bit words in each row.
    pub bits: usize,
    /// The rows, [`ROW_HEAD`] words, the operands, then the bits, each.
    #[serde(with = "hex")]
    pub rows: Vec<u32>,
}

/// One row of a [`Group`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Row<'a> {
    /// The input.
    pub input: u32,
    /// The stage's outcome, where it emits one.
    pub stage: Option<u32>,
    /// The read-back's outcome.
    pub read_outcome: u32,
    /// The expression's own operands, as the entry emitted them.
    pub operands: &'a [u32],
    /// The input's bits.
    pub bits: &'a [u32],
}

impl Row<'_> {
    /// The row's outcome in the rate: the stage's where it emits one, otherwise the read-back's.
    pub fn outcome(&self) -> u32 {
        self.stage.unwrap_or(self.read_outcome)
    }

    /// Which outcome [`Row::outcome`] is.
    pub fn source(&self) -> &'static str {
        if self.stage.is_some() {
            "stage"
        } else {
            "read-back"
        }
    }
}

impl Group {
    fn stride(&self) -> usize {
        ROW_HEAD + self.operands + self.bits
    }

    /// Whether the rows read [`Record::canon_operands`]: canon's, whose operands are the sample's.
    fn shares_operands(&self) -> bool {
        self.operands == 0 && self.kind == Kind::AddMul
    }

    /// The number of rows.
    pub fn len(&self) -> usize {
        self.rows.len() / self.stride()
    }

    /// Whether the group has no rows.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Row `n`, reading a shared operand from `shared`.
    pub fn row<'a>(&'a self, n: usize, shared: &'a [u32]) -> Row<'a> {
        let at = n * self.stride();
        let w = &self.rows[at..at + self.stride()];
        let operands = if self.shares_operands() {
            let k = w[0] as usize * read::CANON_OPERANDS;
            &shared[k..k + read::CANON_OPERANDS]
        } else {
            &w[ROW_HEAD..ROW_HEAD + self.operands]
        };
        Row {
            input: w[0],
            stage: (w[1] != NOT_EMITTED).then_some(w[1]),
            read_outcome: w[2],
            operands,
            bits: &w[ROW_HEAD + self.operands..],
        }
    }

    /// The columns of a row as words, for a synthetic row.
    pub fn push(&mut self, input: u32, stage: Option<u32>, read_outcome: u32, words: &[u32]) {
        self.rows
            .extend([input, stage.unwrap_or(NOT_EMITTED), read_outcome]);
        self.rows.extend_from_slice(words);
    }
}

/// One backend's record (the module's note).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    /// The commit.
    pub commit: String,
    /// The backend: one of [`BACKENDS`], or the adapter's name for any other.
    pub backend: String,
    /// The adapter line.
    pub adapter: String,
    /// The driver version.
    pub driver: String,
    /// The compute fast-math setting, and the mode as compiled.
    pub fast_math: String,
    /// The hardware label, `--hardware-label`.
    pub hardware_label: Option<String>,
    /// The hash of the inputs.
    pub inputs_hash: String,
    /// The number of inputs.
    pub inputs: u32,
    /// Canon's operands, [`read::CANON_OPERANDS`] words per canon input.
    #[serde(with = "hex")]
    pub canon_operands: Vec<u32>,
    /// The rows.
    pub groups: Vec<Group>,
}

/// The backend a harness adapter is, by name (the module's note).
pub fn backend_of(info: &AdapterInfo) -> String {
    let name = info.name.to_lowercase();
    if info.backend == wgpu::Backend::Metal {
        "Metal".to_owned()
    } else if name.contains("llvmpipe") {
        "lavapipe".to_owned()
    } else if name.contains("nvidia") || name.contains("geforce") || name.contains("rtx") {
        "NVIDIA Vulkan".to_owned()
    } else {
        info.name.clone()
    }
}

// ── The inputs ───────────────────────────────────────────────────────────────────────────────────────────────────

/// A named run of canon states in [`Inputs::canon`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Part {
    /// The test the states come from.
    pub name: String,
    /// How many.
    pub count: usize,
}

/// One decode dispatch's inputs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Batch {
    /// The stage's entry: the variant.
    pub entry: String,
    /// The batch's kind in its test.
    pub label: String,
    /// The chart's uniform rows, 24 words.
    pub uniform: Vec<u32>,
    /// `E*`'s bits; `None` is `η_E` off.
    pub e_star: Option<u32>,
    /// The number of points.
    pub points: usize,
}

/// The fixture: canon's states and the decode's batches. `points` holds every batch's `(u, v)` bits, in batch order.
#[derive(Clone, Debug, PartialEq)]
pub struct Inputs {
    /// The states.
    pub canon: Vec<Ic<f32>>,
    /// What the states are, in order.
    pub parts: Vec<Part>,
    /// The decode batches.
    pub batches: Vec<Batch>,
    /// Their points.
    pub points: Vec<[u32; 2]>,
}

/// The manifest beside the binary words.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    parts: Vec<Part>,
    batches: Vec<Batch>,
}

fn input_dir(root: &Path) -> PathBuf {
    root.join("fixtures/gates").join(NAME).join("inputs")
}

fn to_bytes(words: impl Iterator<Item = u32>) -> Vec<u8> {
    words.flat_map(u32::to_le_bytes).collect()
}

fn to_words(bytes: &[u8]) -> Vec<u32> {
    bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b))
        .collect()
}

fn ic_of(words: &[u32]) -> Ic<f32> {
    stage::read_ic(words, 0)
}

impl Inputs {
    /// The number of inputs.
    pub fn len(&self) -> usize {
        self.canon.len() + self.points.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn canon_words(&self) -> Vec<u32> {
        let n = stage::ic_words();
        let mut words = vec![0; n * self.canon.len()];
        for (i, ic) in self.canon.iter().enumerate() {
            stage::write_ic(&mut words, i, ic);
        }
        words
    }

    /// The hash of the inputs: FNV-1a, 64 bits, over the words, read, so a checkout's line endings don't enter.
    pub fn hash(&self) -> String {
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        let mut eat = |w: u32| {
            for b in w.to_le_bytes() {
                h = (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3);
            }
        };
        self.canon_words().into_iter().for_each(&mut eat);
        for b in &self.batches {
            b.entry.bytes().for_each(|c| eat(u32::from(c)));
            b.uniform.iter().for_each(|w| eat(*w));
            eat(b.e_star.unwrap_or(0));
            eat(u32::from(b.e_star.is_some()));
            eat(b.points as u32);
        }
        self.points.iter().flatten().for_each(|w| eat(*w));
        format!("{h:016x}")
    }

    /// Writes the inputs under `root`'s gate directory.
    pub fn write(&self, root: &Path) -> Result<(), String> {
        let dir = input_dir(root);
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let manifest = Manifest {
            parts: self.parts.clone(),
            batches: self.batches.clone(),
        };
        let text = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())? + "\n";
        for (name, bytes) in [
            ("manifest.json", text.into_bytes()),
            ("canon.bin", to_bytes(self.canon_words().into_iter())),
            (
                "points.bin",
                to_bytes(self.points.iter().flatten().copied()),
            ),
        ] {
            let path = dir.join(name);
            std::fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        }
        Ok(())
    }

    /// Reads the inputs under `root`'s gate directory.
    pub fn load(root: &Path) -> Result<Self, String> {
        let dir = input_dir(root);
        let read = |name: &str| {
            let path = dir.join(name);
            std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))
        };
        let manifest: Manifest = serde_json::from_slice(&read("manifest.json")?)
            .map_err(|e| format!("{}: manifest.json: {e}", dir.display()))?;
        let canon = to_words(&read("canon.bin")?);
        let points = to_words(&read("points.bin")?);
        Ok(Inputs {
            canon: canon.chunks_exact(stage::ic_words()).map(ic_of).collect(),
            parts: manifest.parts,
            batches: manifest.batches,
            points: points.chunks_exact(2).map(|p| [p[0], p[1]]).collect(),
        })
    }

    /// The input's name in a report: its test's part and place, or its batch and point.
    pub fn name(&self, input: u32) -> String {
        let mut at = input as usize;
        for p in &self.parts {
            if at < p.count {
                return format!("canon {} {at}", p.name);
            }
            at -= p.count;
        }
        for (n, b) in self.batches.iter().enumerate() {
            if at < b.points {
                return format!("{} {} batch {n} point {at}", b.entry, b.label);
            }
            at -= b.points;
        }
        format!("input {input}")
    }
}

fn narrow(ic: &Ic<f64>) -> Ic<f32> {
    Ic {
        m: ic.m.map(|x| x as f32),
        r: ic.r.map(|v| v.map(|x| x as f32)),
        p: ic.p.map(|v| v.map(|x| x as f32)),
    }
}

fn widen(ic: &Ic<f32>) -> Ic<f64> {
    Ic {
        m: ic.m.map(f64::from),
        r: ic.r.map(|v| v.map(f64::from)),
        p: ic.p.map(|v| v.map(f64::from)),
    }
}

const EPS32: f64 = f32::EPSILON as f64;

// canonicaliser_mirror_tie's states (`crates/engine/tests/canon_gpu.rs`).

fn tie<R: LinkReal + IeeeBits>(a: R) -> Ic<R> {
    let (q, h, z, one) = (
        R::from(0.25).unwrap(),
        R::from(0.5).unwrap(),
        R::zero(),
        R::one(),
    );
    Ic {
        m: [q, q, h],
        r: [[-one, a], [-one, a], [one, -a]],
        p: [[z; 2]; 3],
    }
}

fn axis_tie(a: f64) -> Ic<f64> {
    Ic {
        m: [0.25, 0.25, 0.5],
        r: [[-1.0, a], [1.0, a], [0.0, -a]],
        p: [[0.0; 2]; 3],
    }
}

fn tie_values<R: LinkReal>(up: fn(R) -> R, down: fn(R) -> R) -> Vec<R> {
    let d = R::DELTA_LAMBDA;
    let two = d + d;
    vec![
        d,
        up(d),
        down(d),
        two,
        -d,
        -up(d),
        -down(d),
        -two,
        R::zero(),
        -R::zero(),
    ]
}

fn turned(x: &Ic<f64>, theta: f64) -> Ic<f64> {
    let (s, c) = theta.sin_cos();
    let rot = |v: [f64; 2]| [c * v[0] - s * v[1], s * v[0] + c * v[1]];
    Ic {
        m: x.m,
        r: x.r.map(|v| {
            let v = rot(v);
            [v[0] + 0.75, v[1] - 1.25]
        }),
        p: [0, 1, 2].map(|i| {
            let v = rot(x.p[i]);
            [v[0] + 0.3 * x.m[i], v[1] - 0.2 * x.m[i]]
        }),
    }
}

fn mirror_ties() -> Vec<Ic<f32>> {
    tie_values::<f32>(f32::next_up, f32::next_down)
        .into_iter()
        .map(tie)
        .collect()
}

fn turned_ties() -> Vec<Ic<f32>> {
    tie_values::<f64>(f64::next_up, f64::next_down)
        .iter()
        .flat_map(|a| {
            [0.3, 1.9, -2.6]
                .into_iter()
                .flat_map(move |t| [turned(&tie(*a), t), turned(&axis_tie(*a / 2f64.sqrt()), t)])
        })
        .map(|x| narrow(&x))
        .collect()
}

// canonicaliser_seam_sweep's states.

const SWEEP: usize = 20_000;

fn seam_states() -> Vec<Ic<f32>> {
    let mut rng = SplitMix::new(41);
    (0..SWEEP)
        .map(|n| {
            let w = [0; 3].map(|_| f64::from(rng.uniform(0.2, 1.2)));
            let total: f64 = w.iter().sum();
            let theta = f64::from(rng.uniform(0.0, 1.0)) * std::f64::consts::TAU;
            let (sn, cs) = theta.sin_cos();
            let origin = [0; 2].map(|_| f64::from(rng.uniform(-0.5, 0.5)));
            let mut t = [0; 3].map(|_| f64::from(rng.uniform(-1.0, 1.0)));
            if n % 8 == 0 {
                t[1] = t[0];
            }
            Ic {
                m: w.map(|x| (x / total) as f32),
                r: t.map(|ti| [(origin[0] + ti * cs) as f32, (origin[1] + ti * sn) as f32]),
                p: [0; 3].map(|_| [rng.uniform(-0.5, 0.5), rng.uniform(-0.5, 0.5)]),
            }
        })
        .collect()
}

fn generic_seeded(n: usize, seed: u64) -> Vec<Ic<f64>> {
    let mut rng = SplitMix::new(seed);
    let mut u = |lo: f32, hi: f32| f64::from(rng.uniform(lo, hi));
    (0..n)
        .map(|_| {
            let w = [u(0.2, 1.2), u(0.2, 1.2), u(0.2, 1.2)];
            let total: f64 = w.iter().sum();
            Ic {
                m: w.map(|x| x / total),
                r: [0; 3].map(|_| [u(-1.0, 1.0), u(-1.0, 1.0)]),
                p: [0; 3].map(|_| [u(-0.5, 0.5), u(-0.5, 0.5)]),
            }
        })
        .collect()
}

fn at_inertia(target: f64, n: usize, seed: u64) -> Vec<Ic<f32>> {
    let mut out = Vec::new();
    for base in generic_seeded(4000, seed) {
        if out.len() == n {
            break;
        }
        let base = narrow(&base);
        let wide = widen(&base);
        let com = [0, 1].map(|j| (0..3).map(|i| wide.m[i] * wide.r[i][j]).sum::<f64>());
        let i0: f64 = (0..3)
            .map(|i| {
                wide.m[i] * ((wide.r[i][0] - com[0]).powi(2) + (wide.r[i][1] - com[1]).powi(2))
            })
            .sum();
        let mut k = (target / i0).sqrt() as f32;
        for _ in 0..64 {
            let x = Ic {
                r: base.r.map(|v| v.map(|q| q * k)),
                ..base
            };
            let i = f64::from(scale_step(&x).i_in);
            if i == target {
                out.push(x);
                break;
            }
            k = if i < target {
                k.next_up()
            } else {
                k.next_down()
            };
        }
    }
    out
}

fn tau_states() -> Vec<Ic<f32>> {
    let (e, tau) = (EPS32, f64::from(f32::TAU_I));
    [
        1.0 + tau - e,
        1.0 + tau,
        1.0 + tau + e,
        1.0 - tau + e / 2.0,
        1.0 - tau,
        1.0 - tau - e / 2.0,
    ]
    .into_iter()
    .enumerate()
    .flat_map(|(k, t)| at_inertia(t, 8, 90 + k as u64))
    .collect()
}

fn ulps(x: f32, k: u32) -> f32 {
    f32::from_bits(x.to_bits() + k)
}

fn subnormal_states() -> Vec<Ic<f32>> {
    let mut rng = SplitMix::new(53);
    (0..2000)
        .map(|n| {
            let w = [0; 3].map(|_| f64::from(rng.uniform(0.2, 1.2)));
            let total: f64 = w.iter().sum();
            let r0 = [0; 2].map(|_| rng.uniform(0.5, 2.0) * 1e-36);
            let mut k = || rng.uniform(1.0, 4000.0) as u32;
            let (kx, ky) = (k(), k());
            let r1 = match n % 5 {
                0 => [ulps(r0[0], kx), r0[1]],
                1 => [r0[0], ulps(r0[1], ky)],
                2 => [ulps(r0[0], kx), ulps(r0[1], ky)],
                3 => [r0[0] + 1e-37, ulps(r0[1], ky)],
                _ => r0,
            };
            let r2 = if n % 7 == 0 {
                [ulps(r0[0], k()), ulps(r0[1], k())]
            } else {
                [0; 2].map(|j| r0[j] + rng.uniform(0.2, 1.5) * 1e-36)
            };
            Ic {
                m: w.map(|x| (x / total) as f32),
                r: [r0, r1, r2],
                p: [0; 3].map(|_| [rng.uniform(-0.5, 0.5), rng.uniform(-0.5, 0.5)]),
            }
        })
        .collect()
}

fn range_states() -> Vec<Ic<f32>> {
    let p = [[0.1, 0.0], [-0.3, 0.2], [0.1, -0.1]];
    let m = [0.25, 0.25, 0.5];
    let far = [1e-37f32, 1e-30, 1e-25, 1e-20, 1e19, 1e20, 1e30, 1e37].map(|k| Ic {
        m,
        r: [[-k, 0.0], [k, 0.0], [0.0, k]],
        p,
    });
    let coincident = [[0.0f32, 0.0], [0.3, -1.7], [-2.5e-3, 7.0], [1e-36, 3e-36]].map(|point| Ic {
        m: [0.2, 0.3, 0.5],
        r: [point; 3],
        p,
    });
    far.into_iter().chain(coincident).collect()
}

// canonicaliser_scale_invariance's states.

const POW2_SCALES: [i32; 8] = [-60, -40, -30, -27, 27, 30, 40, 60];
const DECIMAL_SCALES: [f64; 6] = [1e-8, 1e-9, 1e-10, 1e-12, 1e16, 1e18];

fn scaled32(x: &Ic<f64>, k: f64) -> Ic<f32> {
    narrow(&Ic {
        m: x.m,
        r: x.r.map(|v| v.map(|q| q * k)),
        p: x.p,
    })
}

fn scale_states() -> Vec<Ic<f32>> {
    let states = generic_seeded(2000, 47);
    let scales = [1.0]
        .into_iter()
        .chain(POW2_SCALES.iter().map(|e| 2f64.powi(*e)))
        .chain(DECIMAL_SCALES);
    scales
        .flat_map(|k| states.iter().map(move |x| scaled32(x, k)))
        .collect()
}

// canonicaliser_hierarchy's states.

const RATIOS: [f64; 6] = [1e-3, 1e-4, 1e-5, 1e-6, 1e-7, 1e-8];

fn hierarchy(ratio: f64, axis: bool, edge: bool, seed: u64) -> Vec<Ic<f32>> {
    let mut rng = SplitMix::new(seed);
    let mut u = |lo: f32, hi: f32| f64::from(rng.uniform(lo, hi));
    (0..2000)
        .map(|_| {
            let w = [u(0.01, 1.0), u(0.01, 1.0), u(0.01, 1.0)];
            let total: f64 = w.iter().sum();
            let m = w.map(|x| f64::from((x / total) as f32));
            let m01 = m[0] + m[1];
            let theta = if axis {
                0.0
            } else {
                u(0.0, 1.0) * std::f64::consts::TAU
            };
            let (dir, across) = ([theta.cos(), theta.sin()], [-theta.sin(), theta.cos()]);
            let rho = dir.map(|q| ratio * q);
            let inner = rho.map(|q| m[1] * q / m01);
            let l = u(0.5, 2.0);
            let i = m[0] * m[1] / m01 * ratio * ratio + m[2] * m01 * l * l;
            let k = if edge { 1.0 + u(-1e-6, 1e-6) } else { 2.0 };
            let ly = -k * f64::DELTA_LAMBDA * i.sqrt() / (m[2] * m01).sqrt();
            let r2 = [0, 1].map(|j| inner[j] + l * dir[j] + ly * across[j]);
            Ic {
                m: m.map(|x| x as f32),
                r: [[0.0, 0.0], rho.map(|q| q as f32), r2.map(|q| q as f32)],
                p: [[0.1, 0.0], [0.0, -0.2], [0.05, 0.1]],
            }
        })
        .collect()
}

fn hierarchy_states() -> Vec<Ic<f32>> {
    let mut out = Vec::new();
    for (n, ratio) in RATIOS.into_iter().enumerate() {
        for axis in [true, false] {
            out.extend(hierarchy(ratio, axis, true, 61 + n as u64));
        }
        out.extend(hierarchy(ratio, true, false, 71 + n as u64));
    }
    out
}

// canonicaliser_hierarchy_edge's states.

const EDGE_RATIOS: [i32; 4] = [-12, -15, -20, -40];

fn edge_side(m: [f64; 3], r: [[f64; 2]; 3]) -> f64 {
    let diff = |a: [f64; 2], b: [f64; 2]| [b[0] - a[0], b[1] - a[1]];
    let (rho, d02, d12) = (diff(r[0], r[1]), diff(r[0], r[2]), diff(r[1], r[2]));
    let lam = [0, 1].map(|k| m[0] * d02[k] + m[1] * d12[k]);
    let c = rho[0] * lam[1] - rho[1] * lam[0];
    let sq = |v: [f64; 2]| v[0] * v[0] + v[1] * v[1];
    let d2 = m[0] * m[1] * sq(rho) + m[0] * m[2] * sq(d02) + m[1] * m[2] * sq(d12);
    let d = f64::DELTA_LAMBDA;
    m[2] * c * c - d * d * (m[0] + m[1]) * sq(rho) * d2
}

fn hierarchy_edge(j: i32, theta: f64, seed: u64) -> Vec<Ic<f32>> {
    let mut rng = SplitMix::new(seed);
    let mut u = |lo: f32, hi: f32| f64::from(rng.uniform(lo, hi));
    let (cs, sn) = (theta.cos(), theta.sin());
    let turn = |v: [f64; 2]| [v[0] * cs - v[1] * sn, v[0] * sn + v[1] * cs];
    let mut out = Vec::new();
    for _ in 0..40 {
        let w = [u(0.2, 1.2), u(0.2, 1.2), u(0.2, 1.2)];
        let total: f64 = w.iter().sum();
        let m32 = w.map(|x| (x / total) as f32);
        let m = m32.map(f64::from);
        let half = f64::from((2f64.powi(j) * u(1.0, 2.0)) as f32);
        let x2 = f64::from(u(0.5, 1.5) as f32);
        let state = |a: f64| [[-half, a], [half, a], [x2, -a]].map(turn);
        let (mut lo, mut hi) = (0.0f64, 1e-6f64);
        for _ in 0..200 {
            let mid = 0.5 * (lo + hi);
            if edge_side(m, state(mid)) > 0.0 {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        let mut a = (0.5 * (lo + hi)) as f32;
        for _ in 0..30 {
            a = a.next_down();
        }
        for _ in 0..61 {
            out.push(Ic {
                m: m32,
                r: state(f64::from(a)).map(|v| v.map(|q| q as f32)),
                p: [[0.0; 2]; 3],
            });
            a = a.next_up();
        }
    }
    out
}

fn edge_states() -> Vec<Ic<f32>> {
    let mut out = Vec::new();
    for (k, j) in EDGE_RATIOS.into_iter().enumerate() {
        for theta in [0.0, 0.7] {
            out.extend(hierarchy_edge(j, theta, 81 + k as u64));
        }
    }
    out
}

// canonicaliser_canonicalise_order's states.

#[derive(Clone, Copy)]
struct Gauge {
    a: [f64; 2],
    u: [f64; 2],
    theta: f64,
    s: f64,
    mirror: bool,
}

fn apply(g: &Gauge, x: &Ic<f64>) -> Ic<f64> {
    let (sn, cs) = g.theta.sin_cos();
    let map = |v: [f64; 2], k: f64| {
        let y = if g.mirror { -v[1] } else { v[1] };
        [k * (cs * v[0] - sn * y), k * (sn * v[0] + cs * y)]
    };
    Ic {
        m: x.m,
        r: x.r.map(|v| {
            let w = map(v, g.s);
            [w[0] + g.a[0], w[1] + g.a[1]]
        }),
        p: [0, 1, 2].map(|i| {
            let w = map(x.p[i], g.s.powf(-0.5));
            [w[0] + x.m[i] * g.u[0], w[1] + x.m[i] * g.u[1]]
        }),
    }
}

fn gauges() -> Vec<Gauge> {
    let mut out = Vec::new();
    for (k, s) in [0.5, 3.0, 40.0].into_iter().enumerate() {
        for theta in [0.7, 2.9, -1.3] {
            for mirror in [false, true] {
                out.push(Gauge {
                    a: [3.0 * s, -2.0 * s - k as f64],
                    u: [0.4, -1.1],
                    theta,
                    s,
                    mirror,
                });
            }
        }
    }
    out
}

fn order_states() -> Vec<Ic<f32>> {
    let representatives: Vec<Ic<f64>> = [
        [0.4f64, -0.3, 0.5, -0.2, 0.3, 0.1, 0.6, -0.4],
        [-0.8, 1.1, -0.4, 0.7, -0.6, 0.2, -0.3, 0.9],
        [1.2, 0.2, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    ]
    .iter()
    .map(|z| decode::<Defaults, f64>(&kernel::decode::Latent::new(*z)).ic)
    .collect();
    representatives
        .iter()
        .flat_map(|x| gauges().into_iter().map(move |g| narrow(&apply(&g, x))))
        .collect()
}

// canonicaliser_coincident_inner_pair's states.

fn coincident() -> Vec<Ic<f64>> {
    let mut rng = SplitMix::new(17);
    let mut u = |lo: f32, hi: f32| f64::from(rng.uniform(lo, hi));
    (0..24)
        .map(|_| {
            let w = [u(0.1, 1.0), u(0.1, 1.0), u(0.1, 1.0)];
            let total: f64 = w.iter().sum();
            let pair = [u(-3.0, 3.0), u(-3.0, 3.0)];
            Ic {
                m: w.map(|x| x / total),
                r: [pair, pair, [u(-3.0, 3.0), u(-3.0, 3.0)]],
                p: [0; 3].map(|_| [u(-1.0, 1.0), u(-1.0, 1.0)]),
            }
        })
        .collect()
}

fn coincident_states() -> Vec<Ic<f32>> {
    let states = coincident();
    let mut out: Vec<Ic<f32>> = states.iter().map(narrow).collect();
    let m = [0.25, 0.375, 0.375];
    let p = [[0.1, -0.2], [0.3, 0.05], [-0.4, 0.15]];
    for (x, y) in [(0.0, 0.0), (-0.0, 0.0), (-0.0, -0.0), (0.0, -0.0)] {
        out.push(narrow(&Ic {
            m,
            r: [[0.0, 0.0], [x, y], [0.0, 0.0]],
            p,
        }));
    }
    for off in [[1.0, 0.0], [-1.0, 0.0], [0.0, 1.0], [0.0, -1.0]] {
        let step = 2f64.powi(-20);
        let mut x = states[0];
        x.r[1] = [x.r[0][0] + off[0] * step, x.r[0][1] + off[1] * step];
        out.push(narrow(&x));
    }
    out
}

// canonicaliser_phi_signed_zero's states.

fn signed_zero_states() -> Vec<Ic<f32>> {
    let m = [0.25, 0.375, 0.375];
    let p = [[0.1, -0.2], [0.3, 0.05], [-0.4, 0.15]];
    let tiny = 2f64.powi(-20);
    [
        ([0.0, -1.0], [-0.0, 1.0], [0.7, 0.2]),
        ([0.0, 1.0], [-0.0, -1.0], [-0.7, 0.2]),
        ([0.0, 0.0], [-0.0, tiny], [0.5, -0.3]),
        ([-0.0, 1.0], [0.0, -1.0], [0.4, 0.6]),
    ]
    .map(|(r0, r1, r2)| {
        narrow(&Ic {
            m,
            r: [r0, r1, r2],
            p,
        })
    })
    .to_vec()
}

// canonicaliser_noop_gpu's decodes, CPU f32's here (the module's note).

fn at(z: [f32; 8]) -> Latent<f32> {
    Latent {
        z0: z,
        q1: [0.0; 8],
        q2: [0.0; 8],
    }
}

fn noop_batches() -> Vec<crate::decode_f32_eps::Batch> {
    let mut b = interior(Fuzz {
        seed: 23,
        charts: 16,
        per_chart: 64,
    });
    let mut rng = SplitMix::new(29);
    let sat = [-1000.0f32, -30.0, 30.0, 1000.0];
    for k in 0..8 {
        for s in sat {
            let mut z: [f32; 8] = core::array::from_fn(|_| rng.uniform(-2.0, 2.0));
            z[k] = s;
            b.push((at(z), vec![[0.5, 0.5]]));
        }
    }
    for a in sat {
        for c in sat {
            let mut z: [f32; 8] = core::array::from_fn(|_| rng.uniform(-2.0, 2.0));
            z[0] = a;
            z[1] = c;
            b.push((at(z), vec![[0.5, 0.5]]));
        }
    }
    b
}

fn wide_batches() -> Vec<crate::decode_f32_eps::Batch> {
    let mut rng = SplitMix::new(99);
    (0..64)
        .map(|_| {
            let mut eight = |h: f32| core::array::from_fn(|_| rng.uniform(-h, h));
            let chart = Latent {
                z0: eight(3.0),
                q1: eight(2.0),
                q2: eight(2.0),
            };
            let points = (0..1024)
                .map(|_| [rng.uniform(0.0, 1.0), rng.uniform(0.0, 1.0)])
                .collect();
            (chart, points)
        })
        .collect()
}

fn cpu_decodes(batches: &[crate::decode_f32_eps::Batch]) -> Vec<Ic<f32>> {
    batches
        .iter()
        .flat_map(|(chart, uv)| {
            uv.iter()
                .map(|p| stage::sample::<Latent<f32>, Defaults, f32>(chart, p[0], p[1]).ic)
        })
        .collect()
}

// `crates/engine/tests/decode_cross_backend.rs`'s batches.

fn latent(rng: &mut SplitMix, spread: f32, rest: bool) -> Latent<f32> {
    let mut draw = |h: f32| -> [f32; 8] { core::array::from_fn(|_| rng.uniform(-h, h)) };
    let (mut z0, mut q1, mut q2) = (draw(1.0), draw(spread), draw(spread));
    if rest {
        for k in 2..6 {
            (z0[k], q1[k], q2[k]) = (0.0, 0.0, 0.0);
        }
    }
    Latent { z0, q1, q2 }
}

fn points(rng: &mut SplitMix, n: usize) -> Vec<[f32; 2]> {
    let mut out: Vec<[f32; 2]> = (0..n)
        .map(|_| [rng.uniform(0.0, 1.0), rng.uniform(0.0, 1.0)])
        .collect();
    for u in [0.0, 0.5, 1.0] {
        for v in [0.0, 0.5, 1.0] {
            out.push([u, v]);
        }
    }
    out
}

type Charted<C> = (C, Vec<[f32; 2]>);

fn latent_defaults() -> Vec<Charted<Latent<f32>>> {
    let mut b = golden();
    b.extend(interior(Fuzz {
        seed: 24,
        charts: 8,
        per_chart: 64,
    }));
    let mut rng = SplitMix::new(124);
    b.push((latent(&mut rng, 0.5, true), points(&mut rng, 64)));
    b
}

fn latent_stick(saturated: bool) -> Vec<Charted<Latent<f32>>> {
    let mut rng = SplitMix::new(224 + u64::from(saturated));
    let (spread, n) = if saturated { (20.0, 128) } else { (0.5, 64) };
    (0..4)
        .map(|_| (latent(&mut rng, spread, false), points(&mut rng, n)))
        .collect()
}

fn frozen(rng: &mut SplitMix) -> Invariant<f32> {
    Invariant {
        z_cfg: [rng.uniform(-2.0, 2.0), rng.uniform(-2.0, 2.0)],
        z_mass: [rng.uniform(-2.0, 2.0), rng.uniform(-2.0, 2.0)],
        k_max: crate::K_MAX.number() as f32,
        gamma_k: crate::GAMMA_K.number() as f32,
    }
}

fn edges(rng: &mut SplitMix) -> Vec<[f32; 2]> {
    let mut p = Vec::new();
    for s in [0.0, 1.0] {
        p.extend((0..=32).map(|j| [s, j as f32 / 32.0]));
        p.extend((0..16).map(|_| [s, rng.uniform(0.0, 1.0)]));
    }
    p
}

fn past(rng: &mut SplitMix) -> Vec<[f32; 2]> {
    let mut p = Vec::new();
    for s in [-0.5, -0.25, 1.25, 1.5] {
        p.extend((0..=16).map(|j| [s, j as f32 / 16.0]));
    }
    for k in 0..32 {
        let s = if k % 2 == 0 {
            rng.uniform(-0.75, -0.01)
        } else {
            rng.uniform(1.01, 1.75)
        };
        p.push([s, rng.uniform(0.0, 1.0)]);
    }
    p
}

fn with<C: Copy>(
    charts: &[C],
    rng: &mut SplitMix,
    draw: fn(&mut SplitMix) -> Vec<[f32; 2]>,
) -> Vec<Charted<C>> {
    charts.iter().map(|&c| (c, draw(rng))).collect()
}

fn inside(rng: &mut SplitMix, n: usize) -> Vec<[f32; 2]> {
    (0..n)
        .map(|_| [rng.uniform(0.01, 0.99), rng.uniform(0.0, 1.0)])
        .collect()
}

fn simplex() -> MassSimplex<f32> {
    MassSimplex {
        nu0: 0.5,
        eps_m: crate::MASS_SIMPLEX_EPS_M.number() as f32,
    }
}

fn burrau() -> Burrau<f32> {
    Burrau {
        nu_range: [0.05, 0.95],
        k_max: crate::K_MAX.number() as f32,
        gamma_k: crate::GAMMA_K.number() as f32,
    }
}

fn plane() -> EuclidPlane<f32> {
    EuclidPlane {
        m_range: [
            crate::EUCLID_M_MIN.number() as f32,
            crate::EUCLID_M_MAX.number() as f32,
        ],
        nu_range: [
            crate::EUCLID_NU_MIN.number() as f32,
            crate::EUCLID_NU_MAX.number() as f32,
        ],
    }
}

fn theta_range() -> [f32; 2] {
    [
        crate::BURRAU_THETA_MIN.number() as f32,
        crate::BURRAU_THETA_MAX.number() as f32,
    ]
}

fn strip_k() -> BurrauThetaK<f32> {
    BurrauThetaK {
        theta_range: theta_range(),
        k_max: crate::K_MAX.number() as f32,
        gamma_k: crate::GAMMA_K.number() as f32,
    }
}

fn strip_lz() -> BurrauThetaLz<f32> {
    BurrauThetaLz {
        theta_range: theta_range(),
        k: crate::BURRAU_THETA_LZ_K.number() as f32,
        lz_fraction: crate::BURRAU_THETA_LZ_FRACTION.number() as f32,
    }
}

fn strip_m() -> BurrauThetaDeltaM<f32> {
    BurrauThetaDeltaM {
        theta_range: theta_range(),
        m_target: [0.25, 0.375, 0.375],
    }
}

fn sphere(rng: &mut SplitMix, map: ShapeMap) -> ShapeSphere<f32> {
    ShapeSphere {
        map,
        n0: [0.0, 0.0, 1.0],
        e1: [1.0, 0.0, 0.0],
        e2: [0.0, 1.0, 0.0],
        scale: 1.2,
        inertia: 1.0,
        phase: 0.0,
        z_mass: [rng.uniform(-1.0, 1.0), rng.uniform(-1.0, 1.0)],
        z_mom: core::array::from_fn(|_| rng.uniform(-1.0, 1.0)),
    }
}

fn mixed(rng: &mut SplitMix) -> Mixed<ZMu1, Alpha, f32> {
    Mixed::new(
        core::array::from_fn(|_| rng.uniform(-2.0, 2.0)),
        [-2.0, 2.0],
        [0.1, 1.4],
    )
}

const POLES: [[f32; 2]; 3] = [[0.5, 0.5], [0.0, 0.5], [1.0, 0.5]];

const TARGETS: [f32; 4] = [0.0, -0.3, -40.0, 1.5];

/// `decode_parity_m01_crossings`' lines.
const LINES: u32 = 20_000;

fn crossings() -> Vec<[f32; 2]> {
    let eps = kernel::decode::mass::m01_epsilon::<f32>();
    let mut out = Vec::with_capacity(2 * LINES as usize);
    for k in 0..LINES {
        let z1 = (6.3 + 14.0 * f64::from(k) / f64::from(LINES)) as f32;
        let sum_tag = |z0: f32| {
            let m = <StickBreaking as Link<2, 3>>::forward([z0, z1]);
            m[0] + m[1] < eps
        };
        let (mut lo, mut hi) = (0.0f32, 30.0f32);
        loop {
            let mid = (lo + hi) * 0.5;
            if mid == lo || mid == hi {
                break;
            }
            if sum_tag(mid) {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        out.push([lo, z1]);
        out.push([hi, z1]);
    }
    out
}

fn crossing_batch() -> Charted<Latent<f32>> {
    let e = |k: usize| core::array::from_fn(|i| if i == k { 1.0 } else { 0.0 });
    let mut z0 = [0.0; 8];
    (z0[6], z0[7]) = (1.0, 1.0);
    let chart = Latent {
        z0,
        q1: e(6),
        q2: e(7),
    };
    let uv = crossings()
        .into_iter()
        .map(|z| [z[0] * 0.5, z[1] * 0.5])
        .collect();
    (chart, uv)
}

// #393's points (`crates/validation/tests/qa_TASK-M2-25_gpu.rs`).

const SCALE_393: f32 = 1e-4;

fn sphere_393(n0: [f32; 3], phase: f32) -> ShapeSphere<f32> {
    let held = ShapeSphere::<f32>::from_uniform(
        &held_uniforms(render::presets::DecodeChart::ShapeSphere).chart,
    );
    ShapeSphere {
        map: ShapeMap::Exponential,
        n0,
        e1: [0.0, 1.0, 0.0],
        e2: [0.0, 0.0, 1.0],
        scale: SCALE_393,
        phase,
        ..held
    }
}

fn points_393() -> Vec<[f32; 2]> {
    let mut out = vec![[0.5f32, 0.5]];
    let mut steps: Vec<f32> = (0..8).map(|k| 0.5 * 10f32.powi(-k)).collect();
    steps.push(0.5f32 - 0.5f32.next_down().max(0.0));
    for d in steps {
        for u in [0.5 + d, 0.5 - d] {
            out.push([u, 0.5]);
            out.push([0.5, u]);
        }
    }
    out
}

fn charts_393() -> Vec<ShapeSphere<f32>> {
    let mut out: Vec<_> = [[0.0f32, 0.0], [0.0, -0.0], [-0.0, 0.0], [-0.0, -0.0]]
        .into_iter()
        .map(|z| sphere_393([1.0, z[0], z[1]], 0.0))
        .collect();
    out.push(sphere_393([1.0, 0.0, 0.0], 0.3));
    out.push(sphere_393([1.0, 0.0, 0.0], 2.0));
    out
}

/// A batch under construction.
#[derive(Default)]
struct Builder {
    batches: Vec<Batch>,
    points: Vec<[u32; 2]>,
}

impl Builder {
    fn push<V: DecodeVariant>(
        &mut self,
        label: &str,
        chart: &V::Chart,
        e_star: Option<f32>,
        uv: &[[f32; 2]],
    ) {
        let u = DecodeUniforms {
            chart: chart.uniform(),
            ..Default::default()
        };
        self.batches.push(Batch {
            entry: V::ENTRY.to_owned(),
            label: label.to_owned(),
            uniform: uniform_words(&u)[..24].to_vec(),
            e_star: e_star.map(f32::to_bits),
            points: uv.len(),
        });
        self.points
            .extend(uv.iter().map(|p| [p[0].to_bits(), p[1].to_bits()]));
    }

    /// `run`'s batches in `decode_cross_backend.rs`: each chart `η_E` off, toward each of [`TARGETS`], and where `own`,
    /// every eighth point toward CPU f32's own `V₀`, where it is finite (the module's note).
    fn runs<V: DecodeVariant>(&mut self, label: &str, charted: &[Charted<V::Chart>], own: bool) {
        for (chart, uv) in charted {
            self.push::<V>(label, chart, None, uv);
            for e in TARGETS {
                self.push::<V>(label, chart, Some(e), uv);
            }
            if own {
                for p in uv.iter().step_by(8) {
                    let off = stage::sample::<V::Chart, V::Links, f32>(chart, p[0], p[1])
                        .descriptor
                        .V_0;
                    if off.is_finite() {
                        self.push::<V>(label, chart, Some(off), &[*p]);
                    }
                }
            }
        }
    }
}

/// The fixture's inputs, built from the tests' own seeds (the module's note). Run once, by `--write-inputs`.
pub fn build_inputs() -> Inputs {
    let mut canon = Vec::new();
    let mut parts = Vec::new();
    let noop_wide = cpu_decodes(&wide_batches());
    for (name, states) in [
        ("canonicaliser_mirror_tie", mirror_ties()),
        ("canonicaliser_mirror_tie turned", turned_ties()),
        ("canonicaliser_seam_sweep seam", seam_states()),
        ("canonicaliser_seam_sweep tau", tau_states()),
        ("canonicaliser_seam_sweep subnormal", subnormal_states()),
        ("canonicaliser_seam_sweep range", range_states()),
        ("canonicaliser_scale_invariance", scale_states()),
        ("canonicaliser_hierarchy", hierarchy_states()),
        ("canonicaliser_hierarchy_edge", edge_states()),
        ("canonicaliser_canonicalise_order", order_states()),
        ("canonicaliser_coincident_inner_pair", coincident_states()),
        ("canonicaliser_phi_signed_zero", signed_zero_states()),
        ("canonicaliser_noop_gpu", cpu_decodes(&noop_batches())),
        ("canonicaliser_noop_gpu wider", noop_wide),
    ] {
        parts.push(Part {
            name: name.to_owned(),
            count: states.len(),
        });
        canon.extend(states);
    }
    let mut b = Builder::default();
    b.runs::<LatentDefaults>("defaults", &latent_defaults(), true);
    b.runs::<LatentStickBreaking>("interior", &latent_stick(false), false);
    b.runs::<LatentStickBreaking>("saturated", &latent_stick(true), false);
    let mut rng = SplitMix::new(324);
    let le: Vec<_> = (0..3).map(|_| InvariantLE(frozen(&mut rng))).collect();
    let lk: Vec<_> = (0..3).map(|_| InvariantLK(frozen(&mut rng))).collect();
    b.runs::<InvariantLEDefaults>("fuzz", &with(&le, &mut rng, |g| inside(g, 64)), false);
    b.runs::<InvariantLKDefaults>("fuzz", &with(&lk, &mut rng, |g| inside(g, 64)), false);
    b.runs::<InvariantLEDefaults>("edge columns", &with(&le, &mut rng, edges), false);
    b.runs::<InvariantLKDefaults>("edge columns", &with(&lk, &mut rng, edges), false);
    b.runs::<InvariantLEDefaults>("past the square", &with(&le, &mut rng, past), false);
    b.runs::<InvariantLKDefaults>("past the square", &with(&lk, &mut rng, past), false);
    let one = |c, n| vec![(c, points(&mut SplitMix::new(n), 128))];
    b.runs::<MassSimplexDefaults>("fuzz", &one(simplex(), 1), true);
    b.runs::<BurrauDefaults>("fuzz", &[(burrau(), points(&mut rng, 128))], false);
    b.runs::<EuclidPlaneDefaults>("fuzz", &[(plane(), points(&mut rng, 128))], true);
    let mx: Vec<_> = (0..3)
        .map(|_| (mixed(&mut rng), points(&mut rng, 64)))
        .collect();
    b.runs::<MixedMu1AlphaDefaults>("fuzz", &mx, false);
    b.runs::<BurrauThetaKDefaults>("fuzz", &[(strip_k(), points(&mut rng, 128))], false);
    b.runs::<BurrauThetaLzDefaults>("fuzz", &[(strip_lz(), points(&mut rng, 128))], false);
    b.runs::<BurrauThetaDeltaMDefaults>("fuzz", &[(strip_m(), points(&mut rng, 128))], true);
    let spheres: Vec<_> = [ShapeMap::Spherical, ShapeMap::Exponential]
        .into_iter()
        .flat_map(|map| [0, 1].map(|_| map))
        .map(|map| {
            let uv = points(&mut rng, 64)
                .into_iter()
                .filter(|p| !POLES.contains(p));
            (sphere(&mut rng, map), uv.collect())
        })
        .collect();
    b.runs::<ShapeSphereDefaults>("fuzz", &spheres, true);
    let pole = [(sphere(&mut rng, ShapeMap::Spherical), POLES.to_vec())];
    b.runs::<ShapeSphereDefaults>("n₀ = ±1 poles", &pole, true);
    let (chart, uv) = crossing_batch();
    b.push::<LatentStickBreaking>("M₀₁ onset crossings", &chart, None, &uv);
    for chart in charts_393() {
        b.push::<ShapeSphereDefaults>("#393", &chart, None, &points_393());
    }
    Inputs {
        canon,
        parts,
        batches: b.batches,
        points: b.points,
    }
}

// ── The record ───────────────────────────────────────────────────────────────────────────────────────────────────

/// One batch's raw run: the stage's tag words and the read-back's words.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Raw {
    tags: Vec<u32>,
    words: Vec<u32>,
}

/// What [`on_each_variant`] calls.
trait Visit {
    fn visit<V: BranchVariant>(&mut self) -> Result<(), String>;
}

/// Calls `v` on each decode variant.
fn on_each_variant<T: Visit>(v: &mut T) -> Result<(), String> {
    v.visit::<LatentDefaults>()?;
    v.visit::<LatentStickBreaking>()?;
    v.visit::<InvariantLEDefaults>()?;
    v.visit::<InvariantLKDefaults>()?;
    v.visit::<MassSimplexDefaults>()?;
    v.visit::<BurrauDefaults>()?;
    v.visit::<EuclidPlaneDefaults>()?;
    v.visit::<MixedMu1AlphaDefaults>()?;
    v.visit::<BurrauThetaKDefaults>()?;
    v.visit::<BurrauThetaLzDefaults>()?;
    v.visit::<BurrauThetaDeltaMDefaults>()?;
    v.visit::<ShapeSphereDefaults>()
}

/// `batch`'s chart, from its uniform rows.
pub fn chart_of<C: UniformChart>(batch: &Batch) -> C {
    let mut u = DecodeUniforms::default();
    for (k, row) in u.chart.iter_mut().enumerate() {
        let w = |i: usize| f32::from_bits(batch.uniform[4 * k + i]);
        (row.x, row.y, row.z, row.w) = (w(0), w(1), w(2), w(3));
    }
    C::from_uniform(&u.chart)
}

/// Each batch with its points, in order.
pub fn with_points(inputs: &Inputs) -> Vec<(usize, &Batch, Vec<[f32; 2]>)> {
    let mut at = 0;
    let mut out = Vec::with_capacity(inputs.batches.len());
    for (n, b) in inputs.batches.iter().enumerate() {
        let uv = inputs.points[at..at + b.points]
            .iter()
            .map(|p| [f32::from_bits(p[0]), f32::from_bits(p[1])])
            .collect();
        at += b.points;
        out.push((n, b, uv));
    }
    out
}

/// The decode on the CPU at f32: the stage's source and the read-back's, natively.
struct CpuDecode<'a> {
    inputs: &'a Inputs,
    out: Vec<Raw>,
}

impl Visit for CpuDecode<'_> {
    fn visit<V: BranchVariant>(&mut self) -> Result<(), String> {
        for (n, b, uv) in with_points(self.inputs) {
            if b.entry != V::ENTRY {
                continue;
            }
            let chart = chart_of::<V::Chart>(b);
            let e = b.e_star.map(f32::from_bits);
            let mut raw = Raw {
                tags: Vec::with_capacity(uv.len()),
                words: vec![0; uv.len() * read::DECODE_WORDS],
            };
            for (i, p) in uv.iter().enumerate() {
                let out = stage::sample_with::<V::Chart, V::Links, f32>(&chart, p[0], p[1], e);
                raw.tags.push(stage::tag(out.degenerate));
                read::write_decode::<V>(
                    &mut raw.words,
                    i * read::DECODE_WORDS,
                    &chart,
                    (p[0], p[1]),
                    e,
                );
            }
            self.out[n] = raw;
        }
        Ok(())
    }
}

/// The decode on the harness's GPU: the production stage and the read-back, one pipeline each per variant.
struct GpuDecode<'a> {
    inputs: &'a Inputs,
    harness: &'a GpuHarness,
    wgsl: &'a str,
    /// Each entry's compute mode as compiled that was not off.
    modes: BTreeSet<String>,
    out: Vec<Raw>,
}

impl Visit for GpuDecode<'_> {
    fn visit<V: BranchVariant>(&mut self) -> Result<(), String> {
        if !self.inputs.batches.iter().any(|b| b.entry == V::ENTRY) {
            return Ok(());
        }
        let (device, queue) = (self.harness.device(), self.harness.queue());
        let setting = FastMath::default();
        let at = |e: engine::compute::ComputeError| format!("{}: {e}", V::ENTRY);
        let stage = DecodeStage::<V>::new(device, self.wgsl, setting).map_err(at)?;
        let reader = BranchReadStage::<V>::new(device, self.read_wgsl, setting).map_err(at)?;
        self.modes.push((V::ENTRY.to_owned(), stage.mode()));
        self.modes.push((V::READ_ENTRY.to_owned(), reader.mode()));
        for (n, b, uv) in with_points(self.inputs) {
            if b.entry != V::ENTRY {
                continue;
            }
            let chart = chart_of::<V::Chart>(b);
            let e = b.e_star.map(f32::from_bits);
            let outs = stage
                .run_with(device, queue, &chart, DecodeMode::Full, e, &uv)
                .map_err(at)?;
            self.out[n] = Raw {
                tags: outs.iter().map(|o| stage::tag(o.degenerate)).collect(),
                words: reader.dispatch(device, queue, &chart, e, &uv).map_err(at)?,
            };
        }
        Ok(())
    }
}

/// `l`'s rows through `entry`, none yet.
fn empty_group(entry: &str, l: &Listed) -> Group {
    Group {
        entry: entry.to_owned(),
        branch: l.name.to_owned(),
        kind: l.kind,
        operands: l.operands,
        bits: l.bits,
        rows: Vec::new(),
    }
}

/// `inputs`' rows from one backend's runs: `canon` the stage's `C` per state and `canon_words` its read-back's words,
/// `decode` each batch's run.
fn rows(
    inputs: &Inputs,
    canon: &[Canonical<f32>],
    canon_words: &[u32],
    decode: &[Raw],
) -> (Vec<u32>, Vec<Group>) {
    let mut operands = Vec::with_capacity(inputs.canon.len() * read::CANON_OPERANDS);
    let mut canon_groups: Vec<(&Listed, Group)> = LIST
        .iter()
        .filter(|l| l.canon)
        .map(|l| (l, empty_group(CANON_ENTRY, l)))
        .collect();
    for (i, (c, words)) in canon
        .iter()
        .zip(canon_words.chunks_exact(read::CANON_WORDS))
        .enumerate()
    {
        operands.extend_from_slice(&words[..read::CANON_OPERANDS]);
        for (l, g) in &mut canon_groups {
            if let Some((outcome, _, bits)) = l.read(words) {
                g.push(i as u32, l.canon_stage_outcome(c), outcome, bits);
            }
        }
    }
    let mut groups: BTreeMap<(String, &str), Group> = canon_groups
        .into_iter()
        .map(|(l, g)| ((CANON_ENTRY.to_owned(), l.name), g))
        .collect();
    let mut input = inputs.canon.len() as u32;
    for (b, raw) in inputs.batches.iter().zip(decode) {
        let entry = entry_of(&b.entry);
        for (tag, words) in raw
            .tags
            .iter()
            .zip(raw.words.chunks_exact(read::DECODE_WORDS))
        {
            for l in LIST.iter().filter(|l| !l.canon) {
                if let Some((outcome, ops, bits)) = l.read(words) {
                    groups
                        .entry((entry.clone(), l.name))
                        .or_insert_with(|| empty_group(&entry, l))
                        .push(input, l.stage_outcome(*tag), outcome, &[ops, bits].concat());
                }
            }
            input += 1;
        }
    }
    (operands, groups.into_values().collect())
}

/// The read-back entry of the decode stage's `entry`.
fn entry_of(entry: &str) -> String {
    entry.replacen("stage_decode_", "stage_branch_read_", 1)
}

/// CPU f32's run on `inputs`, as a record: the same source, natively (rustc), at f32.
pub fn record_cpu(inputs: &Inputs, commit: &str) -> Result<Record, String> {
    let canon: Vec<Canonical<f32>> = inputs.canon.iter().map(canonicalise).collect();
    let mut words = vec![0; inputs.canon.len() * read::CANON_WORDS];
    for (i, ic) in inputs.canon.iter().enumerate() {
        read::write_canon(&mut words, i * read::CANON_WORDS, ic.m, ic.r);
    }
    let mut decode = CpuDecode {
        inputs,
        out: vec![Raw::default(); inputs.batches.len()],
    };
    on_each_variant(&mut decode)?;
    let (canon_operands, groups) = rows(inputs, &canon, &words, &decode.out);
    Ok(Record {
        commit: commit.to_owned(),
        backend: CPU.to_owned(),
        adapter: "CPU f32: the kernel source, native (rustc)".to_owned(),
        driver: "not a driver".to_owned(),
        fast_math: "setting off; no shader".to_owned(),
        hardware_label: None,
        inputs_hash: inputs.hash(),
        inputs: inputs.len() as u32,
        canon_operands,
        groups,
    })
}

/// The read-back's module, which `cargo xtask build-kernel` parts from the production `kernel.wgsl`.
pub fn read_back_wgsl(root: &Path) -> Result<String, String> {
    let path = root.join("target/spirv/branch_read.wgsl");
    std::fs::read_to_string(&path).map_err(|e| {
        format!(
            "refused: cannot read {} ({e}); run `cargo xtask build-kernel` first",
            path.display()
        )
    })
}

/// The states one dispatch takes, so that an output buffer stays within the default binding size.
const CANON_CHUNK: usize = 1 << 16;

/// The harness's GPU's run on `inputs`, as a record.
pub fn record_gpu(root: &Path, inputs: &Inputs, commit: &str) -> Result<Record, String> {
    let h = GpuHarness::new().map_err(|e| e.to_string())?;
    let wgsl = kernel_wgsl(root)?;
    let read_wgsl = read_back_wgsl(root)?;
    let setting = FastMath::default();
    let at = |e: engine::compute::ComputeError| e.to_string();
    let stage = CanonStage::new(h.device(), &wgsl, setting).map_err(at)?;
    let reader = BranchReadCanon::new(h.device(), &read_wgsl, setting).map_err(at)?;
    let (mut canon, mut words) = (Vec::new(), Vec::new());
    for chunk in inputs.canon.chunks(CANON_CHUNK) {
        canon.extend(stage.dispatch(h.device(), h.queue(), chunk).map_err(at)?);
        words.extend(reader.dispatch(h.device(), h.queue(), chunk).map_err(at)?);
    }
    let mut decode = GpuDecode {
        inputs,
        harness: &h,
        wgsl: &wgsl,
        read_wgsl: &read_wgsl,
        modes: vec![
            (CANONICALISE_ENTRY.to_owned(), stage.mode()),
            (CANON_ENTRY.to_owned(), reader.mode()),
        ],
        out: vec![Raw::default(); inputs.batches.len()],
    };
    on_each_variant(&mut decode)?;
    let (canon_operands, groups) = rows(inputs, &canon, &words, &decode.out);
    let info = h.adapter_info();
    let compiled = if decode.modes.is_empty() {
        "compute compiled off".to_owned()
    } else {
        format!(
            "compiled otherwise: {}",
            decode.modes.into_iter().collect::<Vec<_>>().join(", ")
        )
    };
    Ok(Record {
        commit: commit.to_owned(),
        backend: backend_of(info),
        adapter: info.to_string(),
        driver: format!("{} {}", info.driver, info.driver_info),
        fast_math: format!("setting off; {compiled}"),
        hardware_label: None,
        inputs_hash: inputs.hash(),
        inputs: inputs.len() as u32,
        canon_operands,
        groups,
    })
}

/// The checks `cross_backend_read_back` makes on CPU f32's `record`: each row's read-back outcome is what the stage gives
/// where it emits one, and each `+ and ×` input, re-evaluated from its emitted operands, gives its emitted bits and
/// outcome. `Err` names the first row that doesn't.
pub fn check_read_back(record: &Record) -> Result<usize, String> {
    let mut checked = 0;
    for g in &record.groups {
        let l = listed(&g.branch).ok_or_else(|| format!("{} is not on the list", g.branch))?;
        for n in 0..g.len() {
            let row = g.row(n, &record.canon_operands);
            let at = || format!("{} input {} ({})", g.branch, row.input, g.entry);
            if let Some(s) = row.stage {
                if s != row.read_outcome {
                    return Err(format!(
                        "{}: the read-back's outcome {} is not the stage's {s}",
                        at(),
                        row.read_outcome
                    ));
                }
            }
            if let Some((outcome, bits)) = l.evaluate(row.operands) {
                if bits != row.bits || outcome != row.read_outcome {
                    return Err(format!(
                        "{}: its input re-evaluated from its emitted operands is not its emitted bits",
                        at()
                    ));
                }
            }
            checked += 1;
        }
    }
    Ok(checked)
}

// ── The comparison ───────────────────────────────────────────────────────────────────────────────────────────────

/// Each pair's outcome under R-649.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PairOutcome {
    /// 0 disagreements, neither driver recorded unverified, not supported.
    Pass,
    /// Anything else, until a measurement shows the mechanism.
    Unexplained,
}

/// A pair of records compared.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pair {
    /// The two backends.
    pub a: String,
    /// The two backends.
    pub b: String,
    /// The inputs that disagreed.
    pub disagreed: Vec<u32>,
    /// The inputs compared.
    pub compared: u32,
    /// `+ and ×` rows whose operands differ.
    pub level_1: u64,
    /// `+ and ×` rows at identical operands whose bits differ.
    pub level_2: u64,
    /// `+ and ×` rows at identical operands and bits whose outcome differs.
    pub level_3: u64,
    /// Rows whose stage outcome differs where the read-back's operands, bits and outcome don't.
    pub production_only: u64,
    /// `other operation` rows whose outcome differs.
    pub other: u64,
    /// Rows one backend reached and the other didn't, an earlier branch having parted them: each counts its input as
    /// disagreeing and takes no level (applied per R-369).
    pub one_side: u64,
    /// The outcome.
    pub outcome: PairOutcome,
}

impl Pair {
    /// `d of N`.
    pub fn rate(&self) -> String {
        format!("{} of {}", self.disagreed.len(), self.compared)
    }
}

/// One branch input through one entry on one driver.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Count {
    /// Evaluations of the first kind with operands bit-identical to CPU f32's.
    pub n: u64,
    /// Level 1: operands matching neither CPU f32's nor another GPU's.
    pub level_1: u64,
    /// Rows of kind `other operation`.
    pub other: u64,
    /// Rows at operands two GPUs share and CPU f32's run didn't produce, the host's evaluation agreeing.
    pub arbitrated: u64,
}

/// A failure of R-690's condition.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Failure {
    /// The branch input.
    pub branch: String,
    /// The compiled entry.
    pub entry: String,
    /// 2 or 3.
    pub level: u8,
    /// The input.
    pub input: u32,
}

/// A driver's word (R-690).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Word {
    /// A failure: "unverified, not supported".
    Unverified,
    /// None, in N evaluations: "no failure in N".
    NoFailure(u64),
    /// CPU f32, the arbiter.
    Arbiter,
    /// No record: "not run".
    NotRun,
}

impl Word {
    /// The word as the report reads.
    pub fn text(self) -> String {
        match self {
            Word::Unverified => "unverified, not supported".to_owned(),
            Word::NoFailure(n) => format!("no failure in {n}"),
            Word::Arbiter => "the arbiter".to_owned(),
            Word::NotRun => "not run".to_owned(),
        }
    }
}

/// A driver's standing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Standing {
    /// The backend.
    pub driver: String,
    /// The word.
    pub word: Word,
    /// Where it holds: the compute stage, the setting, the adapter and driver, the entries read, any hardware label.
    pub domain: String,
    /// Per branch input and entry.
    pub counts: BTreeMap<(String, String), Count>,
    /// The failures.
    pub failures: Vec<Failure>,
}

/// The comparison of a set of records.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Comparison {
    /// One line per record: backend, commit, adapter, driver, fast math, hardware label.
    pub records: Vec<[String; 6]>,
    /// The inputs' hash.
    pub inputs_hash: String,
    /// The inputs compared.
    pub inputs: u32,
    /// Every pair of records compared.
    pub pairs: Vec<Pair>,
    /// Every driver.
    pub standings: Vec<Standing>,
}

/// What one row of a driver reads against the arbiter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Judged {
    /// Level 2 or 3.
    Fails(u8),
    /// Reached the test, at CPU f32's operands, and agrees.
    Clean,
    /// Reached the test, at operands two GPUs share, and agrees.
    Arbitrated,
    /// Level 1.
    Upstream,
    /// Kind `other operation`.
    Other,
}

/// `row` of a driver, of the branch `l`, against CPU f32's row `cpu` and the other GPUs' `others`.
fn judge(l: Option<&Listed>, kind: Kind, row: &Row, cpu: Option<&Row>, others: &[Row]) -> Judged {
    if kind == Kind::Other {
        return Judged::Other;
    }
    if let Some(c) = cpu.filter(|c| c.operands == row.operands) {
        return if c.bits != row.bits {
            Judged::Fails(2)
        } else if c.read_outcome != row.read_outcome {
            Judged::Fails(3)
        } else {
            Judged::Clean
        };
    }
    if !others.iter().any(|o| o.operands == row.operands) {
        return Judged::Upstream;
    }
    match l.and_then(|l| l.evaluate(row.operands)) {
        Some((_, bits)) if bits != row.bits => Judged::Fails(2),
        Some((outcome, _)) if outcome != row.read_outcome => Judged::Fails(3),
        Some(_) => Judged::Arbitrated,
        None => Judged::Upstream,
    }
}

/// A group's identity across records.
type Key<'a> = (&'a str, &'a str);

/// Refuses `records` that are not over the same inputs, or whose rows don't line up: a branch's kind and words differ, or
/// a record's own words are cut short. A row one record has and another doesn't is a measurement, not a refusal: a branch
/// an earlier one kept from running ([`Pair::one_side`]).
fn check_alike(records: &[Record]) -> Result<(), String> {
    let mut shapes: BTreeMap<Key, (Kind, usize, usize)> = BTreeMap::new();
    for r in records {
        for g in &r.groups {
            let cut = |what: &str| format!("{}: {} ({}): {what}", r.backend, g.branch, g.entry);
            if !g.rows.len().is_multiple_of(g.stride()) {
                return Err(cut("a row cut short, refused, not compared"));
            }
            let canon_reach = (g.len().checked_sub(1)).map(|n| g.rows[n * g.stride()]);
            if g.shares_operands()
                && canon_reach.is_some_and(|last| {
                    (last as usize + 1) * read::CANON_OPERANDS > r.canon_operands.len()
                })
            {
                return Err(cut("no operands for its input, refused, not compared"));
            }
            let shape = (g.kind, g.operands, g.bits);
            let seen = *shapes
                .entry((g.entry.as_str(), g.branch.as_str()))
                .or_insert(shape);
            if seen != shape {
                return Err(cut("different rows, refused, not compared"));
            }
        }
    }
    let first = records.first();
    if let Some((a, b)) =
        first.and_then(|a| Some(a).zip(records.iter().find(|b| a.inputs_hash != b.inputs_hash)))
    {
        return Err(format!(
            "{} and {}: different inputs ({} and {}), refused, not compared",
            a.backend, b.backend, a.inputs_hash, b.inputs_hash
        ));
    }
    Ok(())
}

/// A pair's tally, before its outcome.
#[derive(Default)]
struct Tally {
    disagreed: BTreeSet<u32>,
    level_1: u64,
    level_2: u64,
    level_3: u64,
    production_only: u64,
    other: u64,
    one_side: u64,
}

impl Tally {
    fn add(&mut self, kind: Kind, input: u32, a: Option<&Row>, b: Option<&Row>) {
        let (Some(a), Some(b)) = (a, b) else {
            self.disagreed.insert(input);
            self.one_side += 1;
            return;
        };
        let differs = a.outcome() != b.outcome();
        if differs {
            self.disagreed.insert(input);
        }
        if a.operands == b.operands && a.bits == b.bits && a.read_outcome == b.read_outcome {
            self.production_only += u64::from(differs);
        } else if kind == Kind::Other {
            self.other += 1;
        } else if a.operands != b.operands {
            self.level_1 += 1;
        } else if a.bits != b.bits {
            self.level_2 += 1;
        } else {
            self.level_3 += 1;
        }
    }
}

/// The domain a standing holds on, for `record`, the entries it read being `entries`.
fn domain(record: &Record, entries: &BTreeSet<&str>) -> String {
    let label = record
        .hardware_label
        .as_ref()
        .map_or(String::new(), |l| format!("; hardware label: {l}"));
    format!(
        "the compute stage, {}, {}, driver {}, read through {}{label}",
        record.fast_math,
        record.adapter,
        record.driver,
        entries.iter().copied().collect::<Vec<_>>().join(", ")
    )
}

/// Compares `records`, every pair, and gives each driver its standing (the module's note). A pair over different inputs
/// is refused. No threshold enters.
pub fn compare(records: &[Record]) -> Result<Comparison, String> {
    check_alike(records)?;
    let cpu = records.iter().position(|r| r.backend == CPU);
    let n = records.len();
    let mut tallies: BTreeMap<(usize, usize), Tally> = BTreeMap::new();
    let mut counts = vec![BTreeMap::new(); n];
    let mut failures: Vec<Vec<Failure>> = vec![Vec::new(); n];
    let mut keyed: BTreeMap<Key, Vec<Option<&Group>>> = BTreeMap::new();
    for (i, r) in records.iter().enumerate() {
        for g in &r.groups {
            keyed
                .entry((g.entry.as_str(), g.branch.as_str()))
                .or_insert_with(|| vec![None; n])[i] = Some(g);
        }
    }
    for ((entry, branch), groups) in keyed {
        let l = listed(branch);
        let Some(kind) = groups.iter().flatten().map(|g| g.kind).next() else {
            continue;
        };
        let mut next = vec![0; n];
        loop {
            let row_at = |i: usize, k: usize| {
                groups[i]
                    .filter(|g| k < g.len())
                    .map(|g| g.row(k, &records[i].canon_operands))
            };
            let Some(input) = (0..n)
                .filter_map(|i| row_at(i, next[i]).map(|r| r.input))
                .min()
            else {
                break;
            };
            let rows: Vec<Option<Row>> = (0..n)
                .map(|i| row_at(i, next[i]).filter(|r| r.input == input))
                .collect();
            for i in 0..n {
                next[i] += usize::from(rows[i].is_some());
                for j in i + 1..n {
                    tallies.entry((i, j)).or_default().add(
                        kind,
                        input,
                        rows[i].as_ref(),
                        rows[j].as_ref(),
                    );
                }
                let Some(row) = rows[i].as_ref().filter(|_| Some(i) != cpu) else {
                    continue;
                };
                let others: Vec<Row> = (0..n)
                    .filter(|&o| o != i && Some(o) != cpu)
                    .filter_map(|o| rows[o])
                    .collect();
                let on_cpu = cpu.and_then(|c| rows[c].as_ref());
                let count: &mut Count = counts[i]
                    .entry((branch.to_owned(), entry.to_owned()))
                    .or_default();
                match judge(l, kind, row, on_cpu, &others) {
                    Judged::Fails(level) => {
                        count.n += 1;
                        failures[i].push(Failure {
                            branch: branch.to_owned(),
                            entry: entry.to_owned(),
                            level,
                            input,
                        });
                    }
                    Judged::Clean => count.n += 1,
                    Judged::Arbitrated => count.arbitrated += 1,
                    Judged::Upstream => count.level_1 += 1,
                    Judged::Other => count.other += 1,
                }
            }
        }
    }
    let word = |i: usize| {
        if Some(i) == cpu {
            Word::Arbiter
        } else if !failures[i].is_empty() {
            Word::Unverified
        } else {
            let total: u64 = counts[i].values().map(|c: &Count| c.n).sum();
            Word::NoFailure(total)
        }
    };
    let pair_list = tallies
        .into_iter()
        .map(|((i, j), t)| {
            let clean = t.disagreed.is_empty()
                && word(i) != Word::Unverified
                && word(j) != Word::Unverified;
            Pair {
                a: records[i].backend.clone(),
                b: records[j].backend.clone(),
                compared: records[i].inputs,
                disagreed: t.disagreed.into_iter().collect(),
                level_1: t.level_1,
                level_2: t.level_2,
                level_3: t.level_3,
                production_only: t.production_only,
                other: t.other,
                one_side: t.one_side,
                outcome: if clean {
                    PairOutcome::Pass
                } else {
                    PairOutcome::Unexplained
                },
            }
        })
        .collect();
    let standings = records
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let entries: BTreeSet<&str> = r.groups.iter().map(|g| g.entry.as_str()).collect();
            Standing {
                driver: r.backend.clone(),
                word: word(i),
                domain: domain(r, &entries),
                counts: counts[i].clone(),
                failures: failures[i].clone(),
            }
        })
        .collect();
    Ok(Comparison {
        records: records
            .iter()
            .map(|r| {
                [
                    r.backend.clone(),
                    r.commit.clone(),
                    r.adapter.clone(),
                    r.driver.clone(),
                    r.fast_math.clone(),
                    r.hardware_label
                        .clone()
                        .unwrap_or_else(|| "none".to_owned()),
                ]
            })
            .collect(),
        inputs_hash: records
            .first()
            .map_or(String::new(), |r| r.inputs_hash.clone()),
        inputs: records.first().map_or(0, |r| r.inputs),
        pairs: pair_list,
        standings,
    })
}

impl Comparison {
    /// The pair of `a` and `b`, either way round.
    pub fn pair(&self, a: &str, b: &str) -> Option<&Pair> {
        self.pairs
            .iter()
            .find(|p| (p.a == a && p.b == b) || (p.a == b && p.b == a))
    }

    /// The standing of `driver`.
    pub fn standing(&self, driver: &str) -> Option<&Standing> {
        self.standings.iter().find(|s| s.driver == driver)
    }

    /// Every backend the report lists: [`BACKENDS`], then any other a record named.
    fn backends(&self) -> Vec<String> {
        let mut all: Vec<String> = BACKENDS.iter().map(|b| (*b).to_owned()).collect();
        for r in &self.records {
            if !all.contains(&r[0]) {
                all.push(r[0].clone());
            }
        }
        all
    }

    /// The report: its header, every pair's rate and every driver's standing, each backend with no record "not run".
    pub fn markdown(&self, name: &dyn Fn(u32) -> String) -> String {
        let mut t = format!(
            "# Gate `{NAME}`: the kernel cross-backend check (REQ-VAL-347; R-686, R-690)\n\n{DOMAIN}\n\n\
             Inputs: {}, hash `{}`.\n\n## Records\n\n\
             | backend | commit | adapter | driver | fast math | hardware label |\n|---|---|---|---|---|---|\n",
            self.inputs, self.inputs_hash
        );
        for r in &self.records {
            let cells = r.clone().map(|cell| cell.replace('|', "\\|"));
            t.push_str(&format!("| {} |\n", cells.join(" | ")));
        }
        t.push_str(
            "\n## Disagreement rate, per backend pair\n\n\
             The rate counts inputs: an input disagrees if any of its rows' outcomes does.\n\n\
             | pair | disagreements of inputs compared | rate | level 1 rows | level 2 rows | level 3 rows | \
             production-only differences | other operation rows differing | rows reached on one side | outcome |\n\
             |---|---|---|---|---|---|---|---|---|---|\n",
        );
        let backends = self.backends();
        for (i, a) in backends.iter().enumerate() {
            for b in &backends[i + 1..] {
                match self.pair(a, b) {
                    None => t.push_str(&format!(
                        "| {a} / {b} | not run | not run | | | | | | | not run |\n"
                    )),
                    Some(p) => t.push_str(&format!(
                        "| {a} / {b} | {} | {:.3e} | {} | {} | {} | {} | {} | {} | {} |\n",
                        p.rate(),
                        p.disagreed.len() as f64 / f64::from(p.compared.max(1)),
                        p.level_1,
                        p.level_2,
                        p.level_3,
                        p.production_only,
                        p.other,
                        p.one_side,
                        match p.outcome {
                            PairOutcome::Pass => "PASS",
                            PairOutcome::Unexplained => "UNEXPLAINED",
                        }
                    )),
                }
            }
        }
        t.push_str("\n## The inputs that disagreed\n");
        for p in &self.pairs {
            t.push_str(&format!("\n### {} / {}: {}\n\n", p.a, p.b, p.rate()));
            if p.disagreed.is_empty() {
                t.push_str("None.\n");
            }
            for i in &p.disagreed {
                t.push_str(&format!("- {}\n", name(*i)));
            }
        }
        t.push_str("\n## Each driver's standing under R-690's condition\n\n\
             N counts evaluations of kind `+ and ×` with operands bit-identical to CPU f32's; the rate's denominator is \
             inputs, a different count. Level 1 and `other operation` rows never reached the test.\n");
        for b in &backends {
            match self.standing(b) {
                None => t.push_str(&format!("\n### {b}: not run\n")),
                Some(s) if s.word == Word::Arbiter => {
                    t.push_str(&format!("\n### {b}: {}\n", s.word.text()));
                }
                Some(s) => {
                    t.push_str(&format!(
                        "\n### {b}: {}\n\nDomain: {}.\n\n\
                         | branch input | entry | kind | N | level 1 | other operation | shared operands, host agrees |\n\
                         |---|---|---|---|---|---|---|\n",
                        s.word.text(),
                        s.domain
                    ));
                    for ((branch, entry), c) in &s.counts {
                        let kind = listed(branch).map_or("", |l| l.kind.word());
                        t.push_str(&format!(
                            "| {branch} | {entry} | {kind} | {} | {} | {} | {} |\n",
                            c.n, c.level_1, c.other, c.arbitrated
                        ));
                    }
                    let mut tally: BTreeMap<(&str, &str, u8), (u64, u32)> = BTreeMap::new();
                    for f in &s.failures {
                        tally
                            .entry((&f.branch, &f.entry, f.level))
                            .and_modify(|c| c.0 += 1)
                            .or_insert((1, f.input));
                    }
                    for ((branch, entry, level), (count, first)) in tally {
                        t.push_str(&format!(
                            "\nFailure: level {level}, {branch} through {entry}: {count} rows, the first {}.\n",
                            name(first)
                        ));
                    }
                }
            }
        }
        t
    }
}

// ── The gate ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// A control input: records and what their comparison must read.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Control {
    /// What the input is.
    description: String,
    /// The records.
    records: Vec<Record>,
    /// Substrings the comparison must hold.
    #[serde(default)]
    reads: Vec<String>,
}

/// `cross-backend-branches`: its control records, with no GPU, and the flags.
pub struct CrossBackendGate;

/// The verdict on a comparison, for a control alone: pass where every pair reads PASS. A record under `--compare` never
/// reaches it.
pub fn control_verdict(c: &Comparison) -> Verdict {
    if c.pairs.iter().all(|p| p.outcome == PairOutcome::Pass) {
        Verdict::Pass
    } else {
        Verdict::Fail
    }
}

impl MeasurementGate for CrossBackendGate {
    fn name(&self) -> &'static str {
        NAME
    }

    fn control(
        &self,
        _config: &MeasurementConfig,
        name: &str,
        fixture: &serde_json::Value,
    ) -> Result<MeasurementControl, String> {
        let control = Control::deserialize(fixture).map_err(|e| format!("{name}: {e}"))?;
        let c = compare(&control.records)?;
        let text = c.markdown(&|i| format!("input {i}"));
        if let Some(missing) = control.reads.iter().find(|r| !text.contains(r.as_str())) {
            return Err(format!("{name}: the comparison does not read `{missing}`"));
        }
        Ok(MeasurementControl {
            description: control.description,
            verdict: control_verdict(&c),
            markdown: text,
        })
    }

    fn flags(&self, root: &Path, args: &[&str]) -> Result<(), String> {
        match args {
            ["--write-inputs"] => {
                let inputs = build_inputs();
                inputs.write(root)?;
                println!(
                    "gate {NAME}: {} inputs written, hash {}",
                    inputs.len(),
                    inputs.hash()
                );
                Ok(())
            }
            ["--record", file, rest @ ..] => run_record(root, file, rest),
            ["--compare", files @ ..] if !files.is_empty() => run_compare(root, files),
            _ => Err(format!(
                "gate {NAME}: unrecognised flags `{}`; usage: --write-inputs | --record <file> [--cpu-f32] \
                 [--hardware-label <label>] | --compare <records>",
                args.join(" ")
            )),
        }
    }
}

/// The commit of the workspace at `root`.
fn commit(root: &Path) -> Result<String, String> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|e| format!("git rev-parse: {e}"))?;
    if !out.status.success() {
        return Err("git rev-parse HEAD failed".to_owned());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

/// `--record <file> [--cpu-f32] [--hardware-label <label>]`: asserts nothing.
fn run_record(root: &Path, file: &str, flags: &[&str]) -> Result<(), String> {
    let (cpu, label) = match flags {
        [] => (false, None),
        ["--cpu-f32"] => (true, None),
        ["--hardware-label", l] => (false, Some(*l)),
        ["--cpu-f32", "--hardware-label", l] | ["--hardware-label", l, "--cpu-f32"] => {
            (true, Some(*l))
        }
        _ => {
            return Err(format!(
                "--record: unrecognised flags `{}`",
                flags.join(" ")
            ))
        }
    };
    let inputs = Inputs::load(root)?;
    let head = commit(root)?;
    let mut record = if cpu {
        record_cpu(&inputs, &head)?
    } else {
        record_gpu(root, &inputs, &head)?
    };
    record.hardware_label = label.map(str::to_owned);
    let text = serde_json::to_string(&record).map_err(|e| e.to_string())?;
    std::fs::write(file, text).map_err(|e| format!("{file}: {e}"))?;
    println!(
        "gate {NAME}: {} record of {} inputs written to {file}, commit {head}",
        record.backend,
        inputs.len()
    );
    Ok(())
}

/// `--compare <records>`: writes `comparison.md` in the gate's directory and prints it. It is no `Outcome`, and no rate
/// fails it.
fn run_compare(root: &Path, files: &[&str]) -> Result<(), String> {
    let records = files
        .iter()
        .map(|f| {
            let text = std::fs::read(f).map_err(|e| format!("{f}: {e}"))?;
            serde_json::from_slice(&text).map_err(|e| format!("{f}: {e}"))
        })
        .collect::<Result<Vec<Record>, String>>()?;
    let inputs = Inputs::load(root)?;
    if let Some(r) = records.iter().find(|r| r.inputs_hash != inputs.hash()) {
        return Err(format!(
            "{}: inputs {}, not the fixture's {}, refused, not compared",
            r.backend,
            r.inputs_hash,
            inputs.hash()
        ));
    }
    let text = compare(&records)?.markdown(&|i| inputs.name(i));
    let path = root.join("fixtures/gates").join(NAME).join("comparison.md");
    std::fs::write(&path, &text).map_err(|e| format!("{}: {e}", path.display()))?;
    print!("{text}");
    println!("written to {}", path.display());
    Ok(())
}
