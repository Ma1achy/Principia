//! The constants register (dd_generation_root §3.8, "The constants register"): every settled constant is declared
//! once, here, with its value, its admissibility class (philosophy §4.2) and a citation of where it was measured or
//! derived (INDEX, "The evidence base"). A threshold on a quantity spanning decades also records its relative basis:
//! the observed distribution or the measured gap it was set from (pitfalls §3). [`gate`] refuses a register with an
//! entry missing any of these, naming the constant; `cargo xtask codegen` runs it before generating ([`crate::gen`]).
//! Code in the physics and engine crates reads its numbers from here, which `cargo xtask lint constants` checks.

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
}

/// A register entry as written: each key is `None` until set. `relative_basis` is required of a threshold only.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConstantBuilder {
    pub name: &'static str,
    pub value: Option<Value>,
    pub class: Option<Admissibility>,
    pub citation: Option<Citation>,
    pub relative_basis: Option<RelativeBasis>,
}

impl ConstantBuilder {
    /// The settled number of this entry, for code to read at compile time; a build error if it has none.
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
        if pending != matches!(citation, Citation::Calibration(_)) {
            return Err(format!(
                "constant `{name}`: a value is pending exactly when the citation is its calibration requirement \
                 (R-71)"
            ));
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
        })
    }
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

/// The register: the constants the payload ledger uses (TASK-M0-08). No `diffusion` sentinel: an invalid fit reads
/// NaN by the predicate `n ≥ 2` (R-245).
pub const REGISTER: &[ConstantBuilder] = &[
    HORIZON_STEPS_MAX,
    FGW_CAPACITY,
    FGW_LENGTH_SENTINEL,
    F16_FINITE_MAX,
];
