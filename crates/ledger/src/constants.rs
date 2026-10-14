//! The constants register (dd_generation_root §3.8, "The constants register"): every settled constant is declared
//! once, here, with its value, its admissibility class (philosophy §4.2) and a citation of where it was measured or
//! derived (INDEX, "The evidence base"). A threshold on a quantity spanning decades also records its relative basis:
//! the observed distribution or the measured gap it was set from (pitfalls §3). [`gate`] refuses a register with an
//! entry missing any of these, naming the constant; `cargo xtask codegen` runs it before generating ([`crate::gen`]).
//! Code in the physics and engine crates reads its numbers from here, which `cargo xtask lint constants` checks.
//!
//! **Provisional entries** (R-427; REQ-SYS-081). A value the corpus leaves open may be entered with its proposed
//! number or threshold, marked `provisional` until a gate, a milestone's, and citing its calibration requirement
//! (R-71, R-182). It is read like any other entry ([`ConstantBuilder::number`]), never through a function standing in
//! for it (REQ-SYS-082). [`provisional_until`] lists the entries a gate's report gives, and an entry still marked
//! blocks its gate. The mark is not hashed into the schema version, as the citation is not ([`crate::version`]).

use crate::gen::GenError;

/// A constant's value (§3.8).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Value {
    /// A settled number.
    Exact(f64),
    /// A settled threshold: the entry must record its [`RelativeBasis`].
    Threshold(f64),
    /// Not settled: a calibration requirement, which the citation names, until the human confirms the value (R-71).
    Calibration,
}

/// Why a constant is admissible (philosophy §4.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Admissibility {
    /// Bounded by its own achievable maximum.
    AchievableMaximum,
    /// Fixed by a conservation law.
    ConservationLaw,
    /// Expressed in canonical units.
    CanonicalUnits,
}

/// Where a constant was measured or derived (INDEX, "The evidence base").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Citation {
    /// A section of the corpus: `file` from the repository root and the section's heading.
    Corpus {
        file: &'static str,
        section: &'static str,
    },
    /// A prin-rs `FINDINGS.md`, `README.md` or `results/` path at a prin-rs commit (R-159).
    PrinRs {
        commit: &'static str,
        path: &'static str,
    },
    /// The id of the calibration requirement that will settle the value (R-71).
    Calibration(&'static str),
}

/// A population in an observed distribution: its name, the closed range `[lo, hi]` of its values, and how many of
/// the distribution's observations it holds. A population is well formed when its ends are finite, `lo ≤ hi`, and
/// `count ≥ 1`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Population {
    pub name: &'static str,
    pub lo: f64,
    pub hi: f64,
    pub count: u64,
}

/// What a threshold was set from (pitfalls §3: "Set thresholds from the observed distribution, or from a measured
/// gap"). Either way the threshold sits between populations: at least one wholly below it, at least one wholly above
/// it, and none containing it (R-250).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RelativeBasis {
    /// The observed distribution at `source`: the populations it shows, and the percentile the threshold sits at.
    /// The percentile is derived from the populations' counts ([`percentile_below`]); the recorded one must equal it.
    Distribution {
        source: Citation,
        percentile: f64,
        populations: &'static [Population],
    },
    /// A gap measured at `source` between the population `below` and the population `above`.
    Gap {
        source: Citation,
        below: Population,
        above: Population,
    },
}

/// A complete register entry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Constant {
    pub name: &'static str,
    pub value: Value,
    pub class: Admissibility,
    pub citation: Citation,
    pub relative_basis: Option<RelativeBasis>,
    /// The gate until which the value is a proposal, if it is one (R-427).
    pub provisional: Option<&'static str>,
}

/// A register entry as written: each key is `None` until set. `relative_basis` is required of a threshold only;
/// `provisional` is set only on a proposed value (R-427): the milestone, `M0` … `M8`, whose gate confirms it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConstantBuilder {
    pub name: &'static str,
    pub value: Option<Value>,
    pub class: Option<Admissibility>,
    pub citation: Option<Citation>,
    pub relative_basis: Option<RelativeBasis>,
    pub provisional: Option<&'static str>,
}

/// The milestones whose gates exist, in order: `plan/MILESTONES.md`'s gate blocks, `<!-- gate:M0 -->` to
/// `<!-- gate:M8 -->`. xtask's `milestones_are_the_plans_gate_blocks` test holds this list to that file, so the plan
/// stays its one source.
pub const MILESTONES: &[&str] = &["M0", "M1", "M2", "M3", "M4", "M5", "M6", "M7", "M8"];

/// Whether `gate` names an existing milestone's gate: one of [`MILESTONES`], exactly (no leading zeros, no case
/// change, no milestone past the plan's last).
pub fn is_milestone(gate: &str) -> bool {
    MILESTONES.contains(&gate)
}

impl ConstantBuilder {
    /// The number of this entry, settled or provisional, for code to read at compile time; a build error if it has
    /// none.
    pub const fn number(&self) -> f64 {
        match self.value {
            Some(Value::Exact(v)) | Some(Value::Threshold(v)) => v,
            _ => panic!("the register entry has no settled number"),
        }
    }

    /// The complete entry, or a line naming the constant and what is missing or wrong.
    pub fn build(&self) -> Result<Constant, String> {
        let name = self.name;
        let missing = |key| format!("constant `{name}` has no `{key}` (dd_generation_root §3.8)");
        let value = self.value.ok_or_else(|| missing("value"))?;
        let class = self.class.ok_or_else(|| missing("class"))?;
        let citation = self.citation.ok_or_else(|| missing("citation"))?;
        let pending = matches!(value, Value::Calibration);
        let calibration = matches!(citation, Citation::Calibration(_));
        match self.provisional {
            Some(gate) => check_provisional(name, gate, pending, calibration)?,
            None if pending != calibration => {
                return Err(format!(
                    "constant `{name}`: a value is pending exactly when the citation is its calibration requirement \
                     (R-71)"
                ))
            }
            None => {}
        }
        if let Value::Exact(v) | Value::Threshold(v) = value {
            if !v.is_finite() {
                return Err(format!(
                    "constant `{name}` = {v} is not finite (dd_generation_root §3.8)"
                ));
            }
        }
        if let Value::Threshold(v) = value {
            let basis = self.relative_basis.ok_or_else(|| {
                format!(
                    "threshold `{name}` has no relative basis: set it from the observed distribution or a measured \
                     gap (pitfalls §3)"
                )
            })?;
            check_basis(name, v, &basis)?;
        }
        Ok(Constant {
            name,
            value,
            class,
            citation,
            relative_basis: self.relative_basis,
            provisional: self.provisional,
        })
    }
}

/// `Ok` if the entry `name`, marked provisional until `gate`, is a well-formed proposal (R-427): `gate` names a
/// milestone, the value is a number or threshold, not `calibration` (`pending`), and the citation is the value's
/// calibration requirement (`calibration`). Otherwise a line naming the constant and what is wrong.
fn check_provisional(
    name: &str,
    gate: &str,
    pending: bool,
    calibration: bool,
) -> Result<(), String> {
    if !is_milestone(gate) {
        return Err(format!(
            "provisional constant `{name}` names no gate: `{gate}` is not a milestone, `M0` … (R-427)"
        ));
    }
    if pending {
        return Err(format!(
            "provisional constant `{name}` has no proposed value: its value is `calibration` (R-427)"
        ));
    }
    if !calibration {
        return Err(format!(
            "provisional constant `{name}` does not cite its calibration requirement (R-427)"
        ));
    }
    Ok(())
}

/// The names of the entries of `register` marked provisional until `gate`, in register order: the entries that
/// gate's report lists, each of which blocks the gate while it is so marked (R-427; dd_generation_root §3.8).
pub fn provisional_until(register: &[ConstantBuilder], gate: &str) -> Vec<&'static str> {
    register
        .iter()
        .filter(|k| k.provisional == Some(gate))
        .map(|k| k.name)
        .collect()
}

/// The percentile of the distribution `populations` at the threshold `v`: the share of its observations, in percent,
/// held by the populations wholly below `v` (R-250). It is `100 · below / total`, computed as one division of the
/// integer counts, so a recorded percentile equal to that quotient compares equal.
pub fn percentile_below(v: f64, populations: &[Population]) -> f64 {
    let total: u64 = populations.iter().map(|p| p.count).sum();
    let below: u64 = populations
        .iter()
        .filter(|p| p.hi < v)
        .map(|p| p.count)
        .sum();
    (100 * below) as f64 / total as f64
}

/// `Err` naming the threshold `name` and the population `p` if `p` is not well formed: finite ends, `lo ≤ hi`,
/// `count ≥ 1`.
fn check_population(name: &str, p: &Population) -> Result<(), String> {
    if !(p.lo.is_finite() && p.hi.is_finite() && p.lo <= p.hi && p.count >= 1) {
        return Err(format!(
            "threshold `{name}`: its population `{}` = [{}, {}] with count {} needs finite ends, lo ≤ hi and \
             count ≥ 1 (dd_generation_root §3.8)",
            p.name, p.lo, p.hi, p.count
        ));
    }
    Ok(())
}

/// `Ok` if the finite threshold `v` sits between populations of `basis` (R-250): each population is well formed,
/// none contains `v`, at least one lies wholly below and one wholly above it, a `gap` threshold lies inside its gap,
/// and a `distribution`'s recorded percentile equals the one its counts give. Otherwise a line naming the threshold
/// and what is wrong.
fn check_basis(name: &str, v: f64, basis: &RelativeBasis) -> Result<(), String> {
    let populations = match basis {
        RelativeBasis::Distribution { populations, .. } => populations.to_vec(),
        RelativeBasis::Gap { below, above, .. } => vec![*below, *above],
    };
    for p in &populations {
        check_population(name, p)?;
    }
    if let Some(p) = populations.iter().find(|p| p.lo <= v && v <= p.hi) {
        return Err(format!(
            "threshold `{name}` = {v} lies inside the range [{}, {}] of the population `{}`, at whatever percentile \
             (pitfalls §3; R-250)",
            p.lo, p.hi, p.name
        ));
    }
    let below = populations.iter().filter(|p| p.hi < v).count();
    let above = populations.iter().filter(|p| v < p.lo).count();
    if below == 0 || above == 0 {
        return Err(format!(
            "threshold `{name}` = {v} does not sit between populations: {below} of its basis's lie wholly below it \
             and {above} wholly above; it needs at least one of each (R-250)"
        ));
    }
    match basis {
        RelativeBasis::Gap { below, above, .. } if !(below.hi < v && v < above.lo) => Err(format!(
            "threshold `{name}` = {v} is not inside the measured gap ({}, {}) between `{}` and `{}` (pitfalls §3)",
            below.hi, above.lo, below.name, above.name
        )),
        RelativeBasis::Distribution {
            percentile,
            populations,
            ..
        } if *percentile != percentile_below(v, populations) => Err(format!(
            "threshold `{name}` records the {percentile}th percentile, but its populations' counts put it at the \
             {}th (R-250)",
            percentile_below(v, populations)
        )),
        _ => Ok(()),
    }
}

/// The generation gate: every entry of `register` built, or generation refused with a line naming each constant
/// that is incomplete or whose threshold has no admissible relative basis.
pub fn gate(register: &[ConstantBuilder]) -> Result<Vec<Constant>, GenError> {
    let (built, refused): (Vec<_>, Vec<_>) = register
        .iter()
        .map(ConstantBuilder::build)
        .partition(Result::is_ok);
    if refused.is_empty() {
        Ok(built.into_iter().map(Result::unwrap).collect())
    } else {
        Err(GenError::Constants(
            refused.into_iter().filter_map(Result::err).collect(),
        ))
    }
}

const PAYLOAD: &str = "docs/design/principia_dd_simstate_payload.md";

/// A settled number, admissible as bounded by its own achievable maximum, with its citation.
const fn maximum(name: &'static str, value: f64, citation: Citation) -> ConstantBuilder {
    ConstantBuilder {
        name,
        value: Some(Value::Exact(value)),
        class: Some(Admissibility::AchievableMaximum),
        citation: Some(citation),
        relative_basis: None,
        provisional: None,
    }
}

/// The dispatch limit on `horizon_steps = ⌈T/dt_macro⌉`: the greatest step count the exact u16 `times` holds; dispatch
/// refuses a configuration over it.
pub const HORIZON_STEPS_MAX: ConstantBuilder = maximum(
    "horizon_steps_max",
    65535.0,
    Citation::Corpus {
        file: "decisions.md",
        section: "R-86 — The payload doc governs the eight payload items *(closes RQ-37)*",
    },
);

/// The free-group word's capacity in symbols: 1 + ⌊(121 − 2)/log₂3⌋, the most the 121-bit mixed-radix payload holds.
pub const FGW_CAPACITY: ConstantBuilder = maximum(
    "fgw_capacity",
    76.0,
    Citation::Corpus {
        file: PAYLOAD,
        section: "3. The word buffer — `free_group_word`",
    },
);

/// The word's truncation sentinel: `length_raw = 127`, the greatest value of the 7-bit length field.
pub const FGW_LENGTH_SENTINEL: ConstantBuilder = maximum(
    "fgw_length_sentinel",
    127.0,
    Citation::Corpus {
        file: PAYLOAD,
        section: "3. The word buffer — `free_group_word`",
    },
);

/// binary16's greatest finite value: the pack clamp, ±65504, applied before `pack2x16float`.
pub const F16_FINITE_MAX: ConstantBuilder = maximum(
    "f16_finite_max",
    65504.0,
    Citation::Corpus {
        file: PAYLOAD,
        section: "1. `SimState` — the hot struct",
    },
);

/// binary16's smallest positive subnormal, 2⁻²⁴: the floor a valid `d_min` below it is stored as (bits `0x0001`), so
/// 0.0 never appears (R-271). Admissible as the format's achievable extreme (R-278; dd_generation_root §3.8).
pub const F16_MIN_SUBNORMAL: ConstantBuilder = maximum(
    "f16_min_subnormal",
    1.0 / 16_777_216.0,
    Citation::Corpus {
        file: "decisions.md",
        section: "R-271 — `d_min`'s unset value is +inf; stored values never reach 0.0 *(closes RQ-163, amends payload §1)*",
    },
);

/// A settled number of the chart (dd_decoder §3), admissible as expressed in canonical units (`G = M = I = 1`;
/// philosophy §4.2), with its citation. The link registry reads it by name (dd_generation_root §3.9).
const fn chart(name: &'static str, value: f64, citation: Citation) -> ConstantBuilder {
    ConstantBuilder {
        name,
        value: Some(Value::Exact(value)),
        class: Some(Admissibility::CanonicalUnits),
        citation: Some(citation),
        relative_basis: None,
        provisional: None,
    }
}

const R10: Citation = Citation::Corpus {
    file: "decisions.md",
    section: "R-10 — μ_max and q_max are settled *(closes RQ-8, amends R-5)*",
};

const CLAMPS: Citation = Citation::Corpus {
    file: "docs/contracts/principia_chart_decoder_contract.md",
    section: "Three hard requirements on any registered link",
};

/// `μ_max`: the mass logits' saturation, `μₖ = μ_max·tanh(z_μk)` (dd_decoder §3.1).
pub const MU_MAX: ConstantBuilder = chart("mu_max", 5.0, R10);

/// `q_max`: the free momenta's cap, `qₖ = q_max·(2σ(z_qk) − 1)` (dd_decoder §3.4).
pub const Q_MAX: ConstantBuilder = chart("q_max", 2.0, R10);

/// `α_min`: the general `α` link's polar margin, 0 for full-sphere coverage (dd_decoder §3.2).
pub const ALPHA_MIN: ConstantBuilder = chart(
    "alpha_min",
    0.0,
    Citation::Corpus {
        file: "decisions.md",
        section: "R-21 — `α_min = 0` *(CD-1)*",
    },
);

/// `ε_μ`: the mass inverse's clamp, `|μₖ| ≤ (1 − ε_μ)·μ_max` (inverse_encode Part 3).
pub const EPS_MU: ConstantBuilder = chart("eps_mu", 1e-6, CLAMPS);

/// `ε_z`: the configuration inverse's clamp, `s ∈ [ε_z, 1 − ε_z]` (inverse_encode Part 3).
pub const EPS_Z: ConstantBuilder = chart("eps_z", 1e-6, CLAMPS);

/// `ε_q`: the free momentum inverse's clamp, `s ∈ [ε_q, 1 − ε_q]` (inverse_encode Part 3).
pub const EPS_Q: ConstantBuilder = chart("eps_q", 1e-6, CLAMPS);

/// `δ_λ`: the mirror tie-break, mirror iff `λ̃_y < −δ_λ` (dd_decoder §3.3; R-82).
pub const DELTA_LAMBDA: ConstantBuilder = chart(
    "delta_lambda",
    1e-12,
    Citation::Corpus {
        file: "decisions.md",
        section: "R-82 — One mirror test, one seed rule *(closes RQ-33)*",
    },
);

/// `ε_w`: the seed-selection floor on the squared mass-weighted norm `‖w⁽²⁾‖²_m` (chart_reference §2.2; R-82).
pub const EPS_W: ConstantBuilder = chart(
    "eps_w",
    1e-10,
    Citation::Corpus {
        file: "docs/design/principia_chart_reference.md",
        section: "2.2 Deterministic momentum construction",
    },
);

/// The section `ε_M01` is proposed in (dd_decoder §3.1).
const M01_SECTION: Citation = Citation::Corpus {
    file: "docs/design/principia_dd_decoder.md",
    section: "3.1 Mass",
};

/// `ε_M01`: the mass block's `DEGENERATE(M01_TINY)` threshold, `M₀₁ < ε_M01` (dd_decoder §3.1). REQ-DEC-039's
/// proposal (R-71), provisional until the M2 gate (R-427): a threshold in the measured gap at `μ_max = 5` between the
/// `stick_breaking` corner's pile-up at `ε_μ − ε_μ²/4` and the ternary plot's floor `2ε_m` (TASK-M2-02,
/// `fixtures/gates/m01_tiny/measurement.toml`; pitfalls §3).
pub const EPS_M01: ConstantBuilder = ConstantBuilder {
    name: "eps_m01",
    value: Some(Value::Threshold(1e-5)),
    class: Some(Admissibility::CanonicalUnits),
    citation: Some(Citation::Calibration("REQ-DEC-039")),
    provisional: Some("M2"),
    relative_basis: Some(RelativeBasis::Gap {
        source: M01_SECTION,
        below: Population {
            name:
                "stick_breaking's saturated corner: M₀₁ at z_μ in {16, 40, 1e3, 1e38}², f64 and f32",
            lo: 9.9999975e-7,
            hi: 1e-6,
            count: 32,
        },
        above: Population {
            name: "the ternary plot's floor, the buffered vertex m₂ = 1: M₀₁ = 2ε_m",
            lo: 2e-4,
            hi: 2e-4,
            count: 1,
        },
    }),
};

/// A proposed number (R-71), admissible as `class`, citing its calibration requirement `requirement` and marked
/// provisional until the gate `gate` (R-427; dd_generation_root §3.8).
const fn proposed(
    name: &'static str,
    value: f64,
    class: Admissibility,
    requirement: &'static str,
    gate: &'static str,
) -> ConstantBuilder {
    ConstantBuilder {
        name,
        value: Some(Value::Exact(value)),
        class: Some(class),
        citation: Some(Citation::Calibration(requirement)),
        relative_basis: None,
        provisional: Some(gate),
    }
}

/// The ruling that confirmed the M1 gate's provisional entries as proposed (dd_generation_root §3.8, `provisional`):
/// a value confirmed at its gate cites the ruling that records the confirmation, in place of its calibration
/// requirement.
const R428: Citation = Citation::Corpus {
    file: "decisions.md",
    section: "R-428 — The M1 gate's nine provisional register entries are confirmed as proposed: the six log floors and the hatch's colours and stripe width; `d_min`'s floor at 2⁻¹⁴ is intended; `rho_ratio`'s and `r_min_pair_0`'s floors stay provisional until M2 *(amends R-401, R-419 and R-427 as they apply to these values)*",
};

/// A number the human confirmed at its gate (R-71), admissible as expressed in canonical units, citing the ruling that
/// confirmed it: a provisional entry whose mark has come off (R-427; dd_generation_root §3.8).
const fn confirmed(name: &'static str, value: f64, ruling: Citation) -> ConstantBuilder {
    ConstantBuilder {
        name,
        value: Some(Value::Exact(value)),
        class: Some(Admissibility::CanonicalUnits),
        citation: Some(ruling),
        relative_basis: None,
        provisional: None,
    }
}

// ── The log floors (R-401, R-419, R-427, R-428; REQ-TOOL-160, REQ-TOOL-163) ─────────────────────────────────────
//
// Each log-scaled field with no ledger `floor` has its own display floor `ε_f`, in that field's own units, the
// magnitude where its compression sets in: the numeric view places `raw = 1 − 1/(1 + ln(1 + |x|/ε_f))`, `dbg_log`'s
// place (`crate::gen::numeric`). Each is a presentation constant, not §3.8's `floor?` key, which R-263 gives a sim-key
// parameter (the drifts' `eps_E` and `eps_L`, unchanged). Each is a power of two, so the f32 WGSL literal and the f64
// twin hold the same value and `|x|/ε_f` is exact. Each is admissible as expressed in its field's canonical units.
// Proposed, R-71, each provisional until the M1 gate (R-427), where the human confirmed all six as proposed (R-428):
// four are confirmed entries citing R-428; `rho_ratio`'s and `r_min_pair_0`'s, confirmed there provisionally, are
// marked provisional until the M2 gate, where they are re-confirmed, and cite REQ-TOOL-163 (R-419, R-428). They replace
// TASK-M1-09's single ε = 2⁻²⁴.

/// `d_min`'s floor, a distance in `G = M = I = 1`: 2⁻¹⁴ ≈ 6.10e-5, binary16's smallest normal. The `d_min` latch is
/// f16 (payload §1): at and above 2⁻¹⁴ it holds a constant 11-bit relative precision, below it a linear subnormal grid
/// of step 2⁻²⁴, down to the stored floor 2⁻²⁴ (R-271), so the view is logarithmic where the storage is and linear
/// where the storage is. It sits 2.2 decades below `r_close` = 0.01 and 1.2 below the default `r_coll` = 10⁻³
/// (integrator contract Part 3, Part 7), so every close approach above the default collision radius is placed in
/// decades; the subnormal band below it holds a value chiefly when `r_coll` is set under 2⁻¹⁴, or when a saturated,
/// under-resolved substep undershoots the default `r_coll` that far. The view reads the band through
/// `unpack2x16float`, which a GPU may flush to 0 (payload §1; R-271): there it draws at raw 0 under any floor. Whether
/// the floor is intended was stated, with its evidence, for the M1 gate (R-427; TASK-M1-18's PR), and the human
/// confirmed it: intended, since 2⁻²⁰ would compress `d_min` ∈ [6e-5, 1] from half the ramp to an eighth of it (R-428).
pub const LOG_FLOOR_D_MIN: ConstantBuilder = confirmed("log_floor_d_min", 1.0 / 16_384.0, R428);

/// `dE_max`'s floor, an absolute energy in the stored field's normalised units (`dE_max = max_t |ΔE(t)|`, an absolute
/// maximum, payload §2; R-401's note, R-403): 2⁻²⁰ ≈ 9.54e-7, the power of two nearest `eps_E`'s default 10⁻⁶
/// (integrator contract Part 3), the floor the drift `energy_drift`'s view takes, so the latch and the drift compress
/// at the same magnitude. The f16 latch holds it as 16 subnormal steps of 2⁻²⁴: the floor and the values below
/// 2⁻¹⁴ are f16 subnormals, which a GPU that flushes them on unpack draws at raw 0 (payload §1; R-271).
pub const LOG_FLOOR_DE_MAX: ConstantBuilder =
    confirmed("log_floor_dE_max", 1.0 / 1_048_576.0, R428);

/// `dLz_max`'s floor, an absolute angular momentum in the stored field's normalised units
/// (`dLz_max = max_t |ΔL_z(t)|`, payload §2; R-401's note, R-403): 2⁻²⁰ ≈ 9.54e-7, the power of two nearest `eps_L`'s
/// default 10⁻⁶ (integrator contract Part 3), as [`LOG_FLOOR_DE_MAX`] is `eps_E`'s, and subnormal in the f16 latch
/// as that floor is.
pub const LOG_FLOOR_DLZ_MAX: ConstantBuilder =
    confirmed("log_floor_dLz_max", 1.0 / 1_048_576.0, R428);

/// `closure_min`'s floor, a dimensionless chord `|n̂(t) − n̂(0)|` on the unit shape sphere: 2⁻²³ ≈ 1.19e-7, f32's
/// spacing on `[1, 2)`, the floor of a difference of two O(1) f32 states, where closure bottoms out on the f32 GPU
/// path ("~1e-7", payload §1). On that path a value below it is saturation, not an orbit's closure, so the view is
/// linear there. It is the f32 path's floor: the f64 CPU path resolves closures to ~1e-16 (payload §1), and under
/// 2⁻²³ those below about 5e-10 draw within one viridis step (1/255) of 0.
pub const LOG_FLOOR_CLOSURE_MIN: ConstantBuilder =
    confirmed("log_floor_closure_min", 1.0 / 8_388_608.0, R428);

/// `rho_ratio`'s floor, a dimensionless ratio, provisional (R-419): 2⁻²³ ≈ 1.19e-7. It assumes the mass-weighted
/// `‖ρ̃‖/‖λ̃‖`, which is `cot α` in the canonical-frame decode (`‖ρ̃‖ = cos α`, `‖λ̃‖ = sin α`, decoder §3.2): near
/// `α = π/2` the ratio is `≈ π/2 − α`, and f32 holds `α ∈ [1, π/2)` in steps of 2⁻²³, so the f32 decode's ratios
/// below 2⁻²³ are its first grid step alone. That is the f32 decode's floor: an f64 decode, stored to f32, holds
/// `cot α` far below it. TASK-M2-03 defines the ratio (REQ-PAY-088) and may choose otherwise; the
/// floor is re-confirmed at M2 either way (REQ-TOOL-163): confirmed provisionally at the M1 gate (R-428), it is marked
/// provisional until M2, citing REQ-TOOL-163.
pub const LOG_FLOOR_RHO_RATIO: ConstantBuilder = proposed(
    "log_floor_rho_ratio",
    1.0 / 8_388_608.0,
    Admissibility::CanonicalUnits,
    "REQ-TOOL-163",
    "M2",
);

/// `r_min_pair_0`'s floor, a distance in `G = M = I = 1` (decoder §2, §3.6): 2⁻¹⁴ ≈ 6.10e-5, [`LOG_FLOOR_D_MIN`]'s, the
/// other distance view's, so the two place a shared distance alike, 2.2 decades below `r_close` = 0.01. Its units are
/// assumed by nothing; its evidence, M1's showcase values, is provisional, re-given on decoded values at M2 (R-419,
/// REQ-TOOL-163): confirmed provisionally at the M1 gate (R-428), it is marked provisional until M2, citing
/// REQ-TOOL-163.
pub const LOG_FLOOR_R_MIN_PAIR_0: ConstantBuilder = proposed(
    "log_floor_r_min_pair_0",
    1.0 / 16_384.0,
    Admissibility::CanonicalUnits,
    "REQ-TOOL-163",
    "M2",
);

/// Each log-scaled field with no ledger `floor`, by its ledger name, with its floor's register entry: the numeric view
/// template reads `ε_f` from here (`crate::gen::numeric`), as TASK-M7-05's `log` compaction will ("its R-401 display
/// floor"). A log field not listed has no floor of its own, and its view is refused (R-401: no default).
pub const LOG_FLOORS: [(&str, ConstantBuilder); 6] = [
    ("d_min", LOG_FLOOR_D_MIN),
    ("dE_max", LOG_FLOOR_DE_MAX),
    ("dLz_max", LOG_FLOOR_DLZ_MAX),
    ("closure_min", LOG_FLOOR_CLOSURE_MIN),
    ("rho_ratio", LOG_FLOOR_RHO_RATIO),
    ("r_min_pair_0", LOG_FLOOR_R_MIN_PAIR_0),
];

// ── The invalid hatch (REQ-COL-055; R-71, R-132, R-136, R-427, R-428) ───────────────────────────────────────────
//
// `debug_invalid(frag_xy)`'s diagonal stripes (render contract, presentation layer): `half_period` pixels wide across
// `x + y`, alternating violet and cyan. Proposed, R-71, provisional until the M1 gate (R-427), where the human
// confirmed the pattern and its colours as proposed (R-428): each entry cites R-428. Each colour is a packed 8-bit
// sRGB triple, `0xRRGGBB`, and with the stripe width is admissible as expressed in the display's canonical units,
// 8-bit sRGB codes and device pixels. Each colour is farther
// in OKLab from every palette entry (the outcome palette, the `dbg_*` palettes, every LUT of colour_composition §7.1
// as its ramp draws it, the grey ramp and the OKLCH hue circle at L 0.75, C 0.12) than the flat magenta R-16 kept is
// from `#E034C6`, 0.101: violet is 0.142 from plasma near its stop 71, and cyan 0.112 from Turbo between its stops 83
// and 84 (`render/tests/prelude.rs`). `crate::gen::prelude::hatch` reads the three.

/// The hatch's first colour, violet `#9B00FF`, at `((p.x + p.y) >> 2) & 1 = 0`.
pub const HATCH_VIOLET: ConstantBuilder = confirmed("hatch_violet", 0x9b00ff as f64, R428);

/// The hatch's second colour, cyan `#48FFFF`, at `((p.x + p.y) >> 2) & 1 = 1`.
pub const HATCH_CYAN: ConstantBuilder = confirmed("hatch_cyan", 0x48ffff as f64, R428);

/// The hatch's stripe width in pixels, 4, a power of two: the pattern shifts `x + y` right by its log2.
pub const HATCH_HALF_PERIOD: ConstantBuilder = confirmed("hatch_half_period", 4.0, R428);

/// The section that defines `hue_wheel` (R-72).
const PRELUDE_SECTION: Citation = Citation::Corpus {
    file: "docs/gui/principia_render_gui_spec.md",
    section: "10.1 The shared prelude library",
};

/// `hue_wheel(t)`'s OKLCH lightness, 0.75: the lightness at which the sRGB gamut holds the largest chroma at every hue
/// (render_gui_spec §10.1). Admissible as the gamut's achievable extreme.
pub const HUE_WHEEL_L: ConstantBuilder = ConstantBuilder {
    name: "hue_wheel_l",
    value: Some(Value::Exact(0.75)),
    class: Some(Admissibility::AchievableMaximum),
    citation: Some(PRELUDE_SECTION),
    relative_basis: None,
    provisional: None,
};

/// `hue_wheel(t)`'s OKLCH chroma, 0.12: the largest chroma the sRGB gamut holds at every hue at L 0.75, 0.1275, less a
/// margin (render_gui_spec §10.1). Admissible as bounded by its own achievable maximum.
pub const HUE_WHEEL_C: ConstantBuilder = ConstantBuilder {
    name: "hue_wheel_c",
    value: Some(Value::Exact(0.12)),
    class: Some(Admissibility::AchievableMaximum),
    citation: Some(PRELUDE_SECTION),
    relative_basis: None,
    provisional: None,
};

/// The register: the constants the payload ledger uses (TASK-M0-08), the chart constants the link registry reads
/// (TASK-M2-01; dd_generation_root §3.9), and the presentation constants the generated views and the prelude read: the
/// log floors, the hatch and the hue wheel (TASK-M1-18; R-427). No `diffusion` sentinel: an invalid fit reads NaN by
/// the predicate `n ≥ 2` (R-245).
pub const REGISTER: &[ConstantBuilder] = &[
    HORIZON_STEPS_MAX,
    FGW_CAPACITY,
    FGW_LENGTH_SENTINEL,
    F16_FINITE_MAX,
    F16_MIN_SUBNORMAL,
    MU_MAX,
    Q_MAX,
    ALPHA_MIN,
    EPS_MU,
    EPS_Z,
    EPS_Q,
    DELTA_LAMBDA,
    EPS_W,
    EPS_M01,
    LOG_FLOOR_D_MIN,
    LOG_FLOOR_DE_MAX,
    LOG_FLOOR_DLZ_MAX,
    LOG_FLOOR_CLOSURE_MIN,
    LOG_FLOOR_RHO_RATIO,
    LOG_FLOOR_R_MIN_PAIR_0,
    HATCH_VIOLET,
    HATCH_CYAN,
    HATCH_HALF_PERIOD,
    HUE_WHEEL_L,
    HUE_WHEEL_C,
];
