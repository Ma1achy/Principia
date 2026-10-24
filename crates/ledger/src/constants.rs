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

/// The most branch-cut crossings one accepted substep holds: one per cut, as a crossing is a sign change of a cut's
/// signed distance between the segment's two endpoints (dd_simstate_payload §3, the half-open sign convention), and the
/// word's generators `a` and `b` are two branch cuts (symbolic_dynamics_contract §1). The capacity of the march's
/// per-substep crossing list (`kernel::word::SubstepCrossings`), each slot reached by a constant index.
pub const MAX_CROSSINGS_PER_SUBSTEP: ConstantBuilder = maximum(
    "max_crossings_per_substep",
    2.0,
    Citation::Corpus {
        file: "docs/contracts/principia_symbolic_dynamics_contract.md",
        section: "1. Generator ↔ branch-cut convention",
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

/// `tanh_sat`: the bound dd_generation_root §3.9's `tanh` operator clamps its argument to, `tanh(clamp(x, ±tanh_sat))`,
/// a NaN kept (an R-72 definition, TASK-M2-06). REQ-DEC-047's proposal (R-71), provisional until the M2 gate (R-427):
/// `tanh` rounds to ±1 from `|x|` ≈ 9.01 at f32 and ≈ 19.06 at f64, so 20 changes no CPU value at either width, and it
/// lies below 43.671875, from which Metal's GPU `tanh` fails, 0 and then NaN.
pub const TANH_SAT: ConstantBuilder = proposed(
    "tanh_sat",
    20.0,
    Admissibility::CanonicalUnits,
    "REQ-DEC-047",
    "M2",
);

/// `K_max`, the invariant charts' warp scale (chart_reference §2.1; REQ-CHART-017): the kinetic energy at the top of
/// `(L_z, E)` and `(L_z, K)`, `K(t) = K_max·t^γ_K`, an energy in `G = M = I = 1`. REQ-CHART-044's proposal (R-71),
/// provisional until the M2 gate (R-427): 1, so the chart spans `E ∈ [U, U + 1]`, which holds the escape energy
/// `E = 0`, `K = |U|`, with room above it, for the bulk of the decoder's configurations: `|U|`'s 99th percentile is
/// 0.256 over `z` uniform on `[−1, 1]⁴` (none above 1) and 0.491 over `[−2, 2]⁴` (6 of 20 000 above 1), the golden
/// IC's 0.192. Its evidence, the renders and the measurement, is `fixtures/gates/invariant_warp/` (TASK-M2-12).
pub const K_MAX: ConstantBuilder = proposed(
    "k_max",
    1.0,
    Admissibility::CanonicalUnits,
    "REQ-CHART-044",
    "M2",
);

/// `γ_K`, the invariant charts' warp exponent (chart_reference §2.1; REQ-CHART-017), `γ_K ≥ 1`. REQ-CHART-044's
/// proposal (R-71), provisional until the M2 gate (R-427): 2, so `L_max(t) = √(2K_max)·t` is linear in `t` and the
/// rows crowd toward the apex `K → 0`, where `K_min = L_z²/2I` and `K` meet: 52 of 512 rows lie below `K = 0.01·K_max`
/// (6 at `γ_K = 1`), at the price of twice `γ_K = 1`'s row spacing at the top, `2K_max/511`. Its evidence is
/// `fixtures/gates/invariant_warp/` (TASK-M2-12).
pub const GAMMA_K: ConstantBuilder = proposed(
    "gamma_k",
    2.0,
    Admissibility::CanonicalUnits,
    "REQ-CHART-044",
    "M2",
);

/// The section the Euclid plane's defaults are given in (chart_reference §4.5).
const EUCLID_SECTION: Citation = Citation::Corpus {
    file: "docs/design/principia_chart_reference.md",
    section: "4.5 The Burrau-family chart maps",
};

/// `m_min`, the Euclid plane's default lower `m`: `m(u) = m_min·(m_max/m_min)^u` on `[1, 32]` (chart_reference §4.5).
pub const EUCLID_M_MIN: ConstantBuilder = chart("euclid_m_min", 1.0, EUCLID_SECTION);

/// `m_max`, the Euclid plane's default upper `m` (chart_reference §4.5).
pub const EUCLID_M_MAX: ConstantBuilder = chart("euclid_m_max", 32.0, EUCLID_SECTION);

/// `ν_min`, the Euclid plane's default lower `ν`: `ν(v)` linear on `[1/32, 31/32]` (chart_reference §4.5).
pub const EUCLID_NU_MIN: ConstantBuilder = chart("euclid_nu_min", 0.03125, EUCLID_SECTION);

/// `ν_max`, the Euclid plane's default upper `ν` (chart_reference §4.5).
pub const EUCLID_NU_MAX: ConstantBuilder = chart("euclid_nu_max", 0.96875, EUCLID_SECTION);

/// `ε_m`, the ternary mass plot's default interior buffer: the masses are shrunk toward the barycentre,
/// `m ← (1 − 3ε_m)·m + ε_m·(1, 1, 1)`, default `10⁻⁴` (chart_reference §4.5).
pub const MASS_SIMPLEX_EPS_M: ConstantBuilder = chart("mass_simplex_eps_m", 1e-4, EUCLID_SECTION);

/// The tolerance to which Burrau at `ν = 1/2` reproduces `(3, 4, 5)` and the classical configuration (chart_reference
/// §5.2): an absolute bound, in units of the precision's machine epsilon, on the largest difference of any mass,
/// position or momentum component of the canonical `(m, r, p)` (`G = M = I = 1`). The components are not all at most 1
/// in magnitude: `I = 1` bounds a position only by `|rᵢ| ≤ 1/√mᵢ`, and at `ν = 1/2` `r₂_y = 3/√5 ≈ 1.342`. REQ-VAL-118's
/// proposal (R-71), provisional until the M2 gate (R-427). Its evidence, `fixtures/gates/burrau_nu_half/` (TASK-M2-10):
/// at f64 and f32 the `ν = 1/2` decode equals the `(m, n) = (2, 1)` construction bit for bit, every intermediate an
/// exact operation or one rounding of the same rational; the classical configuration, normalised and historical,
/// brought to the canonical section by `C`, differs from it by at most 1 `eps` (4 leaves a margin for a platform's `sqrt`).
pub const BURRAU_NU_HALF_TOL: ConstantBuilder = proposed(
    "burrau_nu_half_tol",
    4.0,
    Admissibility::CanonicalUnits,
    "REQ-VAL-118",
    "M2",
);

/// `θ_min`, the acute-angle Burrau charts' default lower acute angle, in radians: `θ(u) = θ_min + (θ_max − θ_min)·u`
/// (chart_reference §4.5; REQ-CHART-024). REQ-CHART-045's proposal (R-71), provisional until the M2 gate (R-427):
/// `θ(31/32) = arctan(63/1984) ≈ 1.82°`, rounded to f32 (so the kernel's f32 uniform and the CPU's f64 chart hold the
/// same number), the acute angle of the Euclid plane's default `ν_max = 31/32`, so both Burrau shape charts reach the
/// same thinnest triangle; there the lightest mass is 1/64 of the total and `|U| ≈ 0.956`, and the strip's `E = 0` row
/// lies at `v ≈ 0.98` (at `θ_max`, `v ≈ 0.44`). Its evidence, the renders
/// and the measurement, is `fixtures/gates/burrau_acute/` (TASK-M2-11).
pub const BURRAU_THETA_MIN: ConstantBuilder = proposed(
    "burrau_theta_min",
    0.0317433662712574,
    Admissibility::CanonicalUnits,
    "REQ-CHART-045",
    "M2",
);

/// `θ_max`, the acute-angle Burrau charts' default upper acute angle, in radians (chart_reference §4.5). REQ-CHART-045's
/// proposal (R-71), provisional until the M2 gate (R-427): the fold's end `π/4`, the isosceles triangle, rounded down
/// to f32 (`π/4 − 3.8e-8`), so that both widths hold it inside `(0, π/4]`; f32's nearest to `π/4` lies above it. Its
/// evidence is `fixtures/gates/burrau_acute/` (TASK-M2-11).
pub const BURRAU_THETA_MAX: ConstantBuilder = proposed(
    "burrau_theta_max",
    0.7853981256484985,
    Admissibility::CanonicalUnits,
    "REQ-CHART-045",
    "M2",
);

/// `K`, `Φ_θ,L_z`'s fixed kinetic energy, in `G = M = I = 1` (chart_reference §4.5). REQ-CHART-046's proposal (R-71),
/// provisional until the M2 gate (R-427): 1/8, below `|U|` at every `θ` of the default range (`|U|` is least at the
/// fold's end, 0.195 at `π/4`), so every system of the chart is bound, `E < 0`, with `2K/|U|` from 0.26 at `θ_min` to
/// 1.28 at `π/4`, about the virial balance `2K = |U|`. Its evidence is `fixtures/gates/burrau_acute/` (TASK-M2-11).
pub const BURRAU_THETA_LZ_K: ConstantBuilder = proposed(
    "burrau_theta_lz_k",
    0.125,
    Admissibility::CanonicalUnits,
    "REQ-CHART-046",
    "M2",
);

/// `λ_L`, `Φ_θ,L_z`'s `L_z` range as a fraction of the feasible `√(2IK)`: `L_z ∈ [−λ_L·√(2IK), λ_L·√(2IK)]`
/// (chart_reference §4.5). REQ-CHART-046's proposal (R-71), provisional until the M2 gate (R-427): 1, the whole feasible
/// band, from the counter-rotating rigid rotation through `L_z = 0` to the co-rotating one, feasible at every pixel, the
/// edges included. Its evidence is `fixtures/gates/burrau_acute/` (TASK-M2-11).
pub const BURRAU_THETA_LZ_FRACTION: ConstantBuilder = proposed(
    "burrau_theta_lz_fraction",
    1.0,
    Admissibility::CanonicalUnits,
    "REQ-CHART-046",
    "M2",
);

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
/// confirmed it: a provisional entry whose mark has come off (R-427; dd_generation_root §3.8), or a calibration
/// entered once confirmed (R-433).
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

// ── The M1 gate's other calibrations (R-433; REQ-INT-086, REQ-VAL-122) ─────────────────────────────────────────────
//
// Two calibrations the human confirmed at the M1 gate (R-433) that had been held outside the register: `r_pole`, a
// parameter of `kernel::shape`, and the ‖n‖ = 1 test's tolerance, a definition in two test files. Each is a register
// entry, read from here.

/// The ruling that confirmed the M1 gate's other calibrations (dd_generation_root §3.8, `provisional`): `n_norm_tol`
/// cites it.
const R433: Citation = Citation::Corpus {
    file: "decisions.md",
    section: "R-433 — The M1 gate's other calibrations are confirmed: `r_pole` = 0.01, a constants-register entry provisional until M3; the running grey `#4E4E4E`; the triple outcomes' `#D6A1FF` and `#000097`; the banding criterion, 1/16 of a step; the ‖n‖ = 1 tolerance, 64 U = 2⁻¹⁸, one register entry *(closes RQ-279; amends R-389, R-395, R-398, R-399 and R-428 as they apply to these values)*",
};

/// `r_pole`, the pole radius of the unwrapped phase `θ̃` (dd_integrator §3.7; R-389): below it, in
/// `√(n_u² + n_v²)` on the unit shape sphere, the longitude is not trusted and `θ̃` holds with a frozen reference.
/// 0.01, TASK-M1-11's proposal (REQ-INT-086; PR #158, physics review 5436669140): the longitude's round-off from `n`'s
/// grows as `1/√(n_u² + n_v²)`, so outside 0.01 a step's longitude delta is trustworthy at f32 and f64, and the parity
/// contract's one-code allowance for the stored pole reference needs it above about 2e-3 (REQ-INT-087). Confirmed by
/// the human at the M1 gate and marked provisional until M3 (R-433, R-427's principle), where it is re-confirmed
/// against real orbits' near-pole arcs (TASK-M3-11, which reports them against `π·r_pole` and `(π/2)·r_pole`); it cites
/// REQ-INT-086 until then. `kernel::shape`'s `theta_step` and `in_disc` take it as a parameter; no product code passes
/// it yet, and the kernel's tests read it from here.
pub const R_POLE: ConstantBuilder = proposed(
    "r_pole",
    0.01,
    Admissibility::CanonicalUnits,
    "REQ-INT-086",
    "M3",
);

// ── The diverging fields' x₀ (REQ-COL-065, REQ-COL-066; R-71, R-411, R-427) ──────────────────────────────────────
//
// Each `diverging` field with no ledger `floor` has its own `x₀`, in that field's units, the magnitude where its
// symlog's compression sets in (dd_colouring §3.6): `b = ½ + ½·sign(x)·(1 − 1/(1 + ln(1 + |x|/x₀)))`, linear below it
// and logarithmic above. Each is a presentation constant, admissible as expressed in its field's canonical units
// (`G = M = I = 1`), and a power of two, so `|x|/x₀` is exact. Proposed, R-71, with its evidence on TASK-M7-05's PR,
// each provisional until the M7 gate (R-427), citing REQ-COL-066. The drifts take their floors, `eps_E` and `eps_L`,
// sim-key parameters, and are not here.

/// `E_0`'s `x₀`, an energy: 1, one unit of the normalised system's binding energy (the golden IC's `E_0 = −1/(3√3)`
/// ≈ −0.192). The fixture slices' `E_0` (−0.53 … −0.08) is drawn in the linear zone; the hypercube's (−0.13 … 78.8)
/// is compressed into the ramp's upper half, `b` ≈ 0.69 … 0.91.
pub const DIVERGING_X0_E_0: ConstantBuilder = proposed(
    "diverging_x0_E_0",
    1.0,
    Admissibility::CanonicalUnits,
    "REQ-COL-066",
    "M7",
);

/// `Lz_0`'s `x₀`, an angular momentum: 1, as `E_0`'s. The fixture slices' `Lz_0` (−0.29 … −0.03) is drawn in the
/// linear zone; the hypercube's (−6.6 … 5.5) spans `b` ≈ 0.22 … 0.80 from its 1st to its 99th percentile.
pub const DIVERGING_X0_LZ_0: ConstantBuilder = proposed(
    "diverging_x0_Lz_0",
    1.0,
    Admissibility::CanonicalUnits,
    "REQ-COL-066",
    "M7",
);

/// Each `diverging` field with no ledger `floor`, by its ledger name, with its `x₀`'s register entry: TASK-M7-05's
/// `diverging` compaction reads `x₀` from here. A diverging field neither here nor with a floor has no compaction, and
/// its ramp is refused (dd_colouring §3.6).
pub const DIVERGING_X0: [(&str, ConstantBuilder); 2] =
    [("E_0", DIVERGING_X0_E_0), ("Lz_0", DIVERGING_X0_LZ_0)];

/// The tolerance on `‖n‖ − 1` at f32 of the paired test asserting `‖n‖ = 1` for vector fields (REQ-VAL-122;
/// gui_state_contract §4): 64 U = 2⁻¹⁸ ≈ 3.81e-6, U = 2⁻²⁴ the f32 unit roundoff. `n = (a − b, 2p, 2q)/I` is exactly
/// unit for any `ρ̃`, `λ̃`, so only the rounding of `a`, `b`, `p`, `q`, `I`, the three divisions and the fragment's
/// `length` departs from it, about 30 U to first order, rounded up (TASK-M1-12, PR #179). Confirmed by the human at the
/// M1 gate (R-433), one entry in place of two test files' definitions.
pub const N_NORM_TOL: ConstantBuilder = confirmed("n_norm_tol", 1.0 / 262_144.0, R433);

// ── The shape-sphere chart's calibrations (REQ-CHART-049, REQ-CHART-053; R-71, R-181, R-427) ────────────────────
//
// Two tolerances TASK-M2-08 proposes with their evidence, each provisional until the M2 gate (R-427) and read from
// here by the tests and the `shape-roundtrip` gate: dimensionless differences of points on the unit shape sphere, so
// admissible as expressed in canonical units.

/// The agreement of `kernel::shape`'s `n` with the IC Inspector's (`docs/gui/reference/ic_inspector.html`) on
/// `fixtures/ic_inspector/shape_vec.json`'s 1000 random ICs, the largest absolute difference in any component
/// (REQ-CHART-049): 16ε = 2⁻⁴⁸ ≈ 3.55e-15, ε = 2⁻⁵² the f64 epsilon. TASK-M2-08's proposal (R-71): the inspector reaches
/// `n` by another route, canonicalising, then `α = atan2(‖λ̃‖, ‖ρ̃‖)`, `β = atan2(λ̃_y, λ̃ₓ)` and `(cos 2α, sin 2α cos β,
/// sin 2α sin β)`, so the two differ by rounding alone: measured, worst 6.94e-16 (3.1ε) over `shape` and `shape_vec`
/// on the raw and the decoded ICs, and 1.10e-15 (5.0ε) for the collision landmarks against its `collisionPhi`; 16ε
/// covers them by 5.1× and 3.2×, while the superseded negative-cross sign misses by O(1).
pub const SHAPE_VEC_JS_TOL: ConstantBuilder = proposed(
    "shape_vec_js_tol",
    1.0 / 281_474_976_710_656.0,
    Admissibility::CanonicalUnits,
    "REQ-CHART-049",
    "M2",
);

/// The shape-sphere round trip's bound, `max ‖shape(Φ(u, v)) − n(u, v)‖` with `Φ` alone (R-178), over the
/// `shape-roundtrip` gate's samples (REQ-CHART-053): 10⁻¹³ absolute, R-181's value. TASK-M2-08's proposal (R-71,
/// R-181): measured on `random.json`'s 208 704 samples (random masses, both hemispheres, both maps, polar radii down
/// to 10⁻¹⁵ about `n₀ = ±1`, and the eight poles), worst 1.06e-15, so 10⁻¹³ covers it by 95×.
pub const SHAPE_ROUNDTRIP_BOUND: ConstantBuilder = proposed(
    "shape_roundtrip_bound",
    1e-13,
    Admissibility::CanonicalUnits,
    "REQ-CHART-053",
    "M2",
);

/// `ε_phys`, the round trip's tolerance in physical units, `‖D(z) − D(E(D(z)))‖_phys ≤ ε_phys` (inverse-encode Part
/// 4; REQ-ENC-024), as `k` in `ε_phys = k·ε_R`, `ε_R` the precision's machine epsilon: 64, so 64·2⁻⁵² ≈ 1.42e-14 at
/// f64 and 64·2⁻²³ ≈ 7.63e-6 at f32. TASK-M2-16's proposal (R-71): the `encode-eps-phys` gate's `interior.json`, 65 536
/// points on each registered chart and 2²⁰ latent `z` on `[−6, 6]⁸`, the defaults links, at f64 and f32, measures a
/// worst interior residual of 21.90 `ε_R` (the ternary mass plot, f64); seeds 2 to 9 of the same draws, about 3.1·10⁷
/// interior samples in all, give at most 25.64 (the 8-D latent interior, f32), so 64 covers them by 2.5×.
pub const EPS_PHYS: ConstantBuilder = proposed(
    "eps_phys",
    64.0,
    Admissibility::CanonicalUnits,
    "REQ-ENC-024",
    "M2",
);

/// Lookup's layer-3 tolerance, decode sanity: the CoM at the origin and `Σpᵢ = 0` within it, as `k` in `k·ε_R` on the
/// residuals `‖Σmᵢrᵢ‖/R_x` and `‖Σpᵢ‖/P_x`, `R_x` and `P_x` Part 4's extents (inverse-encode § "Chart-aware
/// validation", layer 3; REQ-ENC-027): 16, so 16·2⁻⁵² ≈ 3.55e-15 at f64 and 16·2⁻²³ ≈ 1.91e-6 at f32. TASK-M2-19's
/// proposal (R-71): the `decode-sanity` gate's `fuzz.json`, 65 536 points on each registered chart and 2²⁰ latent `z`
/// on `[−16, 16]⁸`, past every control's saturation, the defaults links, at f64 and f32, measures a worst of 1.54
/// `ε_R` (`Σpᵢ`, the `latent` chart's row, f64; the 8-D latent `z`'s worst is 1.47), so 16 covers it by 10×; no
/// decoded mass is not positive.
pub const DECODE_SANITY: ConstantBuilder = proposed(
    "decode_sanity",
    16.0,
    Admissibility::CanonicalUnits,
    "REQ-ENC-027",
    "M2",
);

/// Lookup's layer-1 zero-class band, `k` in `|v| ≤ k·ε_R·S_v` for class 0, `S_v` the first-order bound on the
/// rounding of `L_z` or `E`, on the entered state's extents, or of a clamp's move or the value after it
/// (inverse-encode § "Chart-aware validation", "The zero class in floating point" and "The class a clamp moves";
/// REQ-ENC-036, REQ-ENC-037, REQ-ENC-038): 16, so `k·ε` ≈ 3.55e-15 at f64. The band absorbs rounding only; a clamp's
/// class is read from its move, so `k` has no bound below. TASK-M2-19's proposal (R-71):
/// `lookup_validation_ladder_zero_band_measure`, 2²⁰ states decoded from latent `z` on `[−8, 8]⁸`, 16 in every 18
/// with one control pushed to `±20` so that it clamps, each against a random gauge image of itself (offsets and
/// boosts on `[−8, 8)²`, scales `2^±12`, either parity), measures a worst difference of 1.18 `ε` of the bound for
/// `C`'s `L_z` and `E` and 1.95 `ε` for the moves and the values after them, so 16 covers both by 8×.
pub const ZERO_CLASS: ConstantBuilder = proposed(
    "zero_class",
    16.0,
    Admissibility::CanonicalUnits,
    "REQ-ENC-037",
    "M2",
);

// ── The steppers' coefficients (dd_integrator §3.2; REQ-INT-037, REQ-INT-056; R-501, R-515) ─────────────────────
//
// The free weights of Yoshida's two compositions, as dd_integrator §3.2 prints them; each consistency weight `w₀` and
// the kick sequences are derived from them in f64 by the steppers' emitter (`crate::gen::steppers`), which writes them
// into the kernel. Pure numbers, the same in any units, so admissible as expressed in canonical units (applied per
// R-369, as the chart constants are). Citation: Yoshida, H. (1990), "Construction of higher order symplectic
// integrators", Phys. Lett. A 150, 262–268 (R-501); the paper is not in the corpus, and the register cites the section
// that prints the digits.

/// The section that prints the steppers' coefficients.
const OCCUPANTS: Citation = Citation::Corpus {
    file: "docs/design/principia_dd_integrator.md",
    section: "3.2 Occupants",
};

/// A stepper coefficient of dd_integrator §3.2, with its citation.
const fn coefficient(name: &'static str, value: f64) -> ConstantBuilder {
    ConstantBuilder {
        name,
        value: Some(Value::Exact(value)),
        class: Some(Admissibility::CanonicalUnits),
        citation: Some(OCCUPANTS),
        relative_basis: None,
        provisional: None,
    }
}

/// Yoshida-4's `w₁ = 1 / (2 − 2^{1/3})`, correctly rounded to f64 (Yoshida 1990, the fourth-order symmetric triple
/// composition, in closed form; R-515).
pub const YOSHIDA4_W1: ConstantBuilder = coefficient("yoshida4_w1", 1.3512071919596575);

/// Yoshida-6's `w₁` (Yoshida 1990, Table 1, solution A).
pub const YOSHIDA6_W1: ConstantBuilder = coefficient("yoshida6_w1", -1.17767998417887);

/// Yoshida-6's `w₂` (Yoshida 1990, Table 1, solution A).
pub const YOSHIDA6_W2: ConstantBuilder = coefficient("yoshida6_w2", 0.235573213359357);

/// Yoshida-6's `w₃` (Yoshida 1990, Table 1, solution A).
pub const YOSHIDA6_W3: ConstantBuilder = coefficient("yoshida6_w3", 0.784513610477560);

/// dd_integrator §5 test 3's rounding-level criterion (REQ-VAL-191), as `k` in `k·ε_R`, `ε_R` the precision's machine
/// epsilon: the residual of `STEP(dt)`, momentum negation, `STEP(dt)`, negation, its Euclidean norm over the positions
/// and momenta divided by the state's, below which the state counts as returned. TASK-M3-02's proposal (R-71),
/// provisional until the M3 gate (R-427). The criterion holds only in its domain, [`STEP_REVERSAL_EDGE_F64`] (10⁻²)
/// `≤ dt/τ ≤` [`STEP_REVERSAL_EDGE_MAX`] (1) at f64 and [`STEP_REVERSAL_EDGE_F32`] (0.4) `≤ dt/τ ≤`
/// [`STEP_REVERSAL_EDGE_MAX`] at f32, where `τ` is the step's shortest two-body time: the least `√(r³ / (G m_pair))`,
/// `G = 1`, `m_pair = mᵢ + mⱼ`, over the three pairs at every position at which test 3 evaluates the force, every force
/// evaluation of `STEP(dt)` from the state and of `STEP(dt)` from its momentum-negated result, the start's positions
/// included (the f64 step's positions at either precision). The start's positions alone do not bound the step's
/// conditioning: a pair already approaching, or one a stage brings close (Burrau's start at rest), passes far closer
/// within one step than the start's `τ` says (qa reviews 5608308863 Q1 and Q2). In the domain dd's named controls, RK4
/// and Yoshida-4 with its stages made asymmetric `(w₁, w₁, w₀)`, read above the criterion at the same dt; outside it a
/// residual under the criterion says nothing, and no verdict is taken there (REQ-VAL-191). Its evidence,
/// `crates/kernel/tests/occupant.rs` (`step_reversibility`, `step_reversibility_upper_edge`,
/// `step_reversibility_domain`), on six fixtures, a generic state, a close pair, Burrau's (3, 4, 5) start at rest and
/// three approaching pairs (equal masses, speed and impact parameter 1 and 0.05, 5 and 0.05, 2 and 0.003), at the dt
/// ladder, at each edge and across the domain at 16 points per decade: KDK, Yoshida-4 and Yoshida-6 at most 1.3 `ε_R`
/// at f64 and 0.92 `ε_R` at f32 there, and at most 3.31 `ε_R` at f64 and 2.35 `ε_R` at f32 (Burrau, dt/τ 0.777 and
/// 0.764) on qa's placement, 1,993 cells over the domain (`crates/kernel/tests/qa_TASK-M3-02.rs`), so 16 covers them by
/// 4.8×; RK4 at least 68 `ε_R` at f64 and 98 `ε_R` at f32, both on the approach at the lower edge, 4.3× and 6.1× the
/// criterion; the asymmetric Yoshida-4 at least 5.2·10⁷ `ε_R` at f64 and 7.7·10⁴ `ε_R` at f32. Below the domain a
/// control's time-asymmetry, relative `O((dt/τ)⁴)` for the asymmetric Yoshida-4 (its stages satisfy `Σw = 1` and
/// `Σw³ = 0`, so the first term odd under reversal of their order is the `h⁴` commutator `[A₁, A₃]`) and `O((dt/τ)⁶)`
/// for RK4 (even order: the `h⁵` terms of `Φ_h` and `Φ_{−h}` cancel in `Φ_{−h}∘Φ_h`), falls under the precision's
/// rounding.
pub const STEP_REVERSAL_TOL: ConstantBuilder = proposed(
    "step_reversal_tol",
    16.0,
    Admissibility::CanonicalUnits,
    "REQ-VAL-191",
    "M3",
);

/// The f64 lower edge of [`STEP_REVERSAL_TOL`]'s domain (REQ-VAL-191): the smallest `dt/τ`, `τ` the step's shortest
/// two-body time ([`STEP_REVERSAL_TOL`]'s), at which test 3 discriminates at f64. TASK-M3-02's proposal (R-71),
/// provisional until the M3 gate (R-427). Its evidence, [`STEP_REVERSAL_TOL`]'s: at the edge RK4 reads 68 `ε_R` on the
/// approach, 4.3× the criterion, and 86 to 5.2·10³ `ε_R` on the other fixtures.
pub const STEP_REVERSAL_EDGE_F64: ConstantBuilder = proposed(
    "step_reversal_edge_f64",
    1e-2,
    Admissibility::CanonicalUnits,
    "REQ-VAL-191",
    "M3",
);

/// The f32 lower edge of [`STEP_REVERSAL_TOL`]'s domain (REQ-VAL-191): the smallest `dt/τ`, `τ` the step's, at which
/// test 3 discriminates at f32. TASK-M3-02's proposal (R-71), provisional until the M3 gate (R-427). Its evidence,
/// [`STEP_REVERSAL_TOL`]'s: at the edge RK4 reads 98 `ε_R` on the approach, 6.1× the criterion, and 120 to 1.8·10³
/// `ε_R` on the other fixtures; on the approach it reads 22 `ε_R` at dt/τ 0.30, 1.4×, and 3.6 `ε_R` at 0.21. *Was:*
/// 0.3, on the start's `τ` (fix rounds 2 to 4).
pub const STEP_REVERSAL_EDGE_F32: ConstantBuilder = proposed(
    "step_reversal_edge_f32",
    0.4,
    Admissibility::CanonicalUnits,
    "REQ-VAL-191",
    "M3",
);

/// The upper edge of [`STEP_REVERSAL_TOL`]'s domain (REQ-VAL-191), at f64 and f32: the largest `dt/τ`, `τ` the step's,
/// at which the criterion is applied. TASK-M3-02's proposal (R-71), provisional until the M3 gate (R-427). Its
/// evidence, [`STEP_REVERSAL_TOL`]'s, and a sweep of eleven fixtures (those six, a hierarchical triple, a rotating
/// Lagrange triangle and approaches at speed 0.75 and 2 with impact parameters 0.01 and 0.003) at 32 points per decade:
/// KDK, Yoshida-4 and Yoshida-6 at most 2.2 `ε_R` up to dt/τ 1 on that sweep and 3.31 `ε_R` on qa's placement (Burrau,
/// f64, dt/τ 0.777), 4.8× under the criterion, 2.0 from 1 to 1.2, 3.9 up to 2, and up to 22 `ε_R` at f64 and 44 `ε_R`
/// at f32 between 2 and 4, where the step's conditioning amplifies rounding past the criterion. *Was:* 0.5, on the
/// start's `τ` (fix round 4).
pub const STEP_REVERSAL_EDGE_MAX: ConstantBuilder = proposed(
    "step_reversal_edge_max",
    1.0,
    Admissibility::CanonicalUnits,
    "REQ-VAL-191",
    "M3",
);

// ── The curve projection's search (dd_encode §3.4; REQ-ENC-020; TASK-M2-18) ──────────────────────────────────────
//
// The minimiser `kernel::encode::curve::nearest` reads, as dd_encode §3.4's "As built" defines it (R-72, applied per
// R-369): a grid over the chart's `ν` span, then a fixed count of golden-section steps. None decides what the payload
// stores, so none is hashed (`crate::version::STORED_BITS`). The kernel reads each from its generated
// `generated/constants.rs` (`crate::gen::links`).

/// The section that defines the curve projection's minimiser.
const CURVE_PROJECTION_SECTION: Citation = Citation::Corpus {
    file: "docs/design/principia_dd_encode.md",
    section: "3.4 Curve projection (off-curve input) — metric pinned",
};

/// `log₂` of the search's grid, 6: `2⁶ = 64` cells, 65 points, the span's edges included (dd_encode §3.4, "The
/// minimiser"). Admissible as expressed in canonical units, the span's own: the spacing is the span times `2⁻⁶`, a
/// power of two, so it and every grid fraction `k·2⁻⁶` are exact at both widths and no division is formed (dd_encode
/// §3.4, "The branch inputs").
pub const CURVE_GRID_LOG2: ConstantBuilder = ConstantBuilder {
    name: "curve_grid_log2",
    value: Some(Value::Exact(6.0)),
    class: Some(Admissibility::CanonicalUnits),
    citation: Some(CURVE_PROJECTION_SECTION),
    relative_basis: None,
    provisional: None,
};

/// The golden-section steps inside the best grid point's two cells, 80, a fixed count (dd_encode §3.4, "The
/// minimiser"). Admissible as bounded by its own achievable maximum: each step shrinks the bracket by `1/φ`, so 80
/// take its width, `2/64` of the span, to `(2/64)·φ⁻⁸⁰ < 10⁻¹⁷`, past f64's resolution on `(0, 1)`; a further step
/// resolves nothing.
pub const CURVE_GOLDEN_STEPS: ConstantBuilder =
    maximum("curve_golden_steps", 80.0, CURVE_PROJECTION_SECTION);

/// `1/φ = (√5 − 1)/2`, the golden section's ratio (dd_encode §3.4, "The branch inputs": a constant, correctly rounded
/// at each width, never a runtime `sqrt`): 0.6180339887498949, the f64 nearest, bits `0x3FE3_C6EF_372F_E950`. Its
/// rounding to f32, `0x3F1E_377A`, is the f32 nearest to `1/φ` as well, so the one value serves both widths with no
/// double-rounding error (the kernel's `encode_curve_primitives` holds both). Admissible as expressed in canonical
/// units: a pure number.
pub const INV_PHI: ConstantBuilder = ConstantBuilder {
    name: "inv_phi",
    value: Some(Value::Exact(0.6180339887498949)),
    class: Some(Admissibility::CanonicalUnits),
    citation: Some(CURVE_PROJECTION_SECTION),
    relative_basis: None,
    provisional: None,
};

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

// ── The substep law (dd_integrator §3.3; integrator_contract Part 3, Part 4; R-502) ───────────────────────────────
//
// `N_sub = min(bucket(d²), N_max)`, the bucket a comparison of the position-quantised `d²` against the sim key's
// thresholds `THR[n] = round_f32((r_sub·n^{−1/γ_sub})²)`, `n = 1 … L_max`, generated on the host per sim key (R-502).
// The three defaults are the sim key's, the integrator contract's Part 3 values, user-adjustable; `cargo xtask codegen`
// evaluates the default table from them (`crate::gen::nsub`), which the host generator must reproduce bit for bit
// (REQ-INT-092). The table's maximum length is a calibration (REQ-INT-094). Pure numbers or lengths in canonical units.

/// The section that owns the substep law's parameters and their defaults.
const SUBSTEP_PARAMETERS: Citation = Citation::Corpus {
    file: "docs/contracts/principia_integrator_contract.md",
    section: "Part 3 — Parameter ownership (who owns what)",
};

/// `r_sub`'s default, 0.05, a length: the separation below which a macro-step takes more than one substep, the scale
/// of the substep law `N_sub = clamp(⌈(r_sub/r_min)^{γ_sub}⌉, 1, N_max)` (dd_integrator §3.3). A sim-key value.
pub const R_SUB: ConstantBuilder = ConstantBuilder {
    name: "r_sub",
    value: Some(Value::Exact(0.05)),
    class: Some(Admissibility::CanonicalUnits),
    citation: Some(SUBSTEP_PARAMETERS),
    relative_basis: None,
    provisional: None,
};

/// `gamma_sub`'s default, 1.5, the substep law's exponent (dd_integrator §3.3). A sim-key value.
pub const GAMMA_SUB: ConstantBuilder = ConstantBuilder {
    name: "gamma_sub",
    value: Some(Value::Exact(1.5)),
    class: Some(Admissibility::CanonicalUnits),
    citation: Some(SUBSTEP_PARAMETERS),
    relative_basis: None,
    provisional: None,
};

/// `N_max`'s default, 64, the substep cap per macro-step (R-68: the corpus's default wins). A sim-key value, at most
/// [`NSUB_TABLE_MAX`]; the host generator refuses one above it or below 1 (R-502).
pub const N_MAX: ConstantBuilder = ConstantBuilder {
    name: "n_max",
    value: Some(Value::Exact(64.0)),
    class: Some(Admissibility::CanonicalUnits),
    citation: Some(SUBSTEP_PARAMETERS),
    relative_basis: None,
    provisional: None,
};

/// `L_max`, the `N_sub` threshold table's maximum length (R-502; REQ-INT-094): the number of thresholds `SimUniforms`
/// holds, the leaves of the comparison tree less one, and the largest `N_max` the host generator accepts. 256,
/// TASK-M3-03's proposal (R-71), provisional until the M3 gate. Its evidence: the table takes `4·L_max` = 1 KiB of
/// `SimUniforms` on every backend, as 64 rows of four u32 (a uniform array's 16-byte stride, so no padding), within
/// WebGPU's default 64 KiB uniform binding and Vulkan's guaranteed 16 KiB; the generated tree is 256 comparisons and
/// 257 leaves, at most `⌈log₂ 257⌉` = 9 comparisons deep per lookup; and with `horizon_steps` at most 65535
/// ([`HORIZON_STEPS_MAX`]) the exact `total_substeps` u32 holds `65535 × 256` = 16,776,960 ≤ 2³² − 1 with room, the bound
/// allowing an `N_max` up to 65537. Four times the default `N_max`, so raising the cap to resolve close encounters
/// before the integration floor (integrator_contract Part 3) has two doublings of room.
pub const NSUB_TABLE_MAX: ConstantBuilder = proposed(
    "nsub_table_max",
    256.0,
    Admissibility::CanonicalUnits,
    "REQ-INT-094",
    "M3",
);

/// The least threshold the host generator accepts where it decides `N_sub` (`THR[n]`, `n < N_max`), `2⁻⁵⁵`, a `d²`.
/// Derived from the bucket's `d²` form (dd_integrator §3.3, the determinism pin; R-547): the square's floor sets an
/// axis square below `2⁻⁸⁰` to 0, lowering `d²` by less than `2⁻⁷⁹`, so at a threshold of at least `2⁻⁵⁵` it moves a
/// decision only within `2⁻²⁴` relative of the edge, one f32 unit roundoff beside the threshold's and `d²`'s own
/// roundings (`kernel::driver::nsub_table`). Below it the floor alone could decide `N_sub` against the specification
/// (REQ-INT-028), so the generator refuses such a sim key (R-502); `2⁻⁵⁵` is a closest approach of about `5·10⁻⁹`
/// canonical lengths. A settled consequence of the floor, not a calibration.
pub const NSUB_THRESHOLD_LEAST: ConstantBuilder = ConstantBuilder {
    name: "nsub_threshold_least",
    value: Some(Value::Exact(2.775_557_561_562_891_4e-17)),
    class: Some(Admissibility::CanonicalUnits),
    citation: Some(Citation::Corpus {
        file: "docs/design/principia_dd_integrator.md",
        section: "3.3 Substep law — and the determinism pin, made concrete",
    }),
    relative_basis: None,
    provisional: None,
};

/// The register: the constants the payload ledger uses (TASK-M0-08), the word's per-substep crossing capacity
/// (TASK-M3-15), the chart constants the link registry reads
/// (TASK-M2-01; dd_generation_root §3.9), the invariant charts' warp defaults (TASK-M2-12), the Burrau `ν = 1/2`
/// tolerance and the Euclid plane's defaults (TASK-M2-10), the acute-angle Burrau charts' defaults (TASK-M2-11), the
/// presentation constants the generated views and the prelude read: the log floors, the hatch and the hue wheel
/// (TASK-M1-18; R-427), `r_pole` and the ‖n‖ = 1 tolerance (R-433), the shape-sphere chart's two tolerances
/// (TASK-M2-08; REQ-CHART-049, REQ-CHART-053), the curve projection's search (TASK-M2-18; REQ-ENC-020), and the
/// diverging fields' `x₀` (TASK-M7-05; REQ-COL-066). No `diffusion` sentinel: an invalid fit reads NaN by the predicate
/// `n ≥ 2` (R-245). Then the steppers' coefficients and test 3's rounding-level criterion (TASK-M3-02; REQ-INT-037,
/// REQ-VAL-191), and the substep law's defaults and its threshold table's maximum length (TASK-M3-03; R-502,
/// REQ-INT-094).
pub const REGISTER: &[ConstantBuilder] = &[
    HORIZON_STEPS_MAX,
    FGW_CAPACITY,
    FGW_LENGTH_SENTINEL,
    MAX_CROSSINGS_PER_SUBSTEP,
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
    TANH_SAT,
    K_MAX,
    GAMMA_K,
    BURRAU_NU_HALF_TOL,
    EUCLID_M_MIN,
    EUCLID_M_MAX,
    EUCLID_NU_MIN,
    EUCLID_NU_MAX,
    MASS_SIMPLEX_EPS_M,
    BURRAU_THETA_MIN,
    BURRAU_THETA_MAX,
    BURRAU_THETA_LZ_K,
    BURRAU_THETA_LZ_FRACTION,
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
    R_POLE,
    N_NORM_TOL,
    SHAPE_VEC_JS_TOL,
    SHAPE_ROUNDTRIP_BOUND,
    EPS_PHYS,
    DECODE_SANITY,
    ZERO_CLASS,
    CURVE_GRID_LOG2,
    CURVE_GOLDEN_STEPS,
    INV_PHI,
    DIVERGING_X0_E_0,
    DIVERGING_X0_LZ_0,
    YOSHIDA4_W1,
    YOSHIDA6_W1,
    YOSHIDA6_W2,
    YOSHIDA6_W3,
    STEP_REVERSAL_TOL,
    STEP_REVERSAL_EDGE_F64,
    STEP_REVERSAL_EDGE_F32,
    STEP_REVERSAL_EDGE_MAX,
    R_SUB,
    GAMMA_SUB,
    N_MAX,
    NSUB_TABLE_MAX,
    NSUB_THRESHOLD_LEAST,
];
