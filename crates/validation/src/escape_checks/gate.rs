//! `cargo xtask gate escape-checks` and `cargo xtask gate escape-rebind` (TASK-M3-33): pitfalls §2.4's checks on the
//! detector as it is, run in the harness ([`super::march`]).
//!
//! - **`escape-checks`** ([`EscapeChecksGate`]): check 2 on `config stability` and check 3 on `deep interior`, each
//!   input's report carrying check 1's record from prin-rs ([`super::CHECK1_PRIN_RS`]) and the harness's byte-identity
//!   arm (the production buffer's bytes before and after the harness, equal). `--propose check2` and `--propose check3`
//!   print each check's calibration proposal (REQ-VAL-123, REQ-VAL-124) with its result.
//! - **`escape-rebind`** ([`EscapeRebindGate`]): the re-bind test ([`super::rebind`]) on both slices. `--propose
//!   offsets` prints REQ-EVT-032's five offsets with the grid they are read from and its source.
//!
//! **An input** (`fixtures/gates/<gate>/*.json`): `{ "description", "slice", "n", "r_coll", "rule", "expected" }`.
//! `slice` is `config_stability` or `deep_interior`; `n` the pixels per side; `r_coll` the collision radius in the
//! slice's units, or null for the slice's own; `rule` is `detector`, the escape detector as it is, or
//! `energy_receding`, the control: the pre-fix rule (`E_rel > 0` and receding, no closure, no distance gate; pitfalls
//! §1.5), applied at the same sampling points of the same marches, whose firings must fail the gate.
//!
//! The plan is unclear here: a `gate.json` holds one threshold, and checks 2 and 3 each have a calibration requirement
//! of their own. Chose one gate with the precision both propose as its threshold (REQ-VAL-123's), each input's report
//! naming its own check's requirement, because the acceptance runs both under one gate name and they propose the same
//! value.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

use serde::Deserialize;

use crate::gate::{Gate, GateConfig, GateReport, Minimum, RegionCount, Regions, Verdict};

use super::check2::{self, Truth, PRECISION_PROPOSED};
use super::march::{self, Firing, PostEscape};
use super::rebind;
use super::{check3, config_stability, deep_interior, GroundTruthOccupant, Population};

/// `escape-checks`'s name.
pub const CHECKS_NAME: &str = "escape-checks";
/// `escape-rebind`'s name.
pub const REBIND_NAME: &str = "escape-rebind";

/// Check 2's production horizon on `config stability`: the slice's own, `horizon = 50` (`fixtures/slices.toml`).
pub const CONFIG_HORIZON: f64 = 50.0;

/// Check 2's pixels per side, proposed.
pub const CHECK2_N: usize = 32;

/// An input.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    description: String,
    slice: String,
    n: usize,
    #[serde(default)]
    r_coll: Option<f64>,
    rule: String,
}

/// Which firings an input scores.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    /// The escape detector as it is.
    Detector,
    /// The control: the pre-fix rule, `E_rel > 0` and receding (the module's note).
    EnergyReceding,
}

impl Rule {
    fn parse(word: &str) -> Option<Self> {
        match word {
            "detector" => Some(Self::Detector),
            "energy_receding" => Some(Self::EnergyReceding),
            _ => None,
        }
    }
}

/// A run's firing replaced by the pre-fix rule's: the first sampling point after `t = 0` at which the farthest body
/// has `E_rel > 0` and its distance has grown since the previous point. Its state is not kept (nothing compares it).
pub fn energy_receding(run: &PostEscape) -> PostEscape {
    let firing = run.points.windows(2).find_map(|w| {
        let k = (0..3)
            .max_by(|&i, &j| w[1].d[i].total_cmp(&w[1].d[j]))
            .expect("three bodies");
        (w[1].e_rel[k] > 0.0 && w[1].d[k] > w[0].d[k]).then_some(Firing {
            index: 0,
            t: w[1].t,
            escaper: k as u32,
            detail: k as u32,
            r: [[f64::NAN; 2]; 3],
            p: [[f64::NAN; 2]; 3],
        })
    });
    PostEscape {
        firing,
        ..run.clone()
    }
}

/// A slice's population and its harness marches, with whether the production buffer's bytes were the same after the
/// harness as before.
pub struct Marched {
    /// The population.
    pub pop: Population,
    /// The harness marches.
    pub runs: Vec<PostEscape>,
    /// Whether the buffer's bytes were unchanged by the harness.
    pub buffer_untouched: bool,
}

/// A slice's population at `n` per side and `r_coll` (the slice's units; `None` for its own).
pub fn population(slice: &str, n: usize, r_coll: Option<f64>) -> Result<Population, String> {
    match slice {
        "config_stability" => match r_coll {
            None => Ok(config_stability(n, CONFIG_HORIZON)),
            Some(_) => Err("config_stability takes its own r_coll".into()),
        },
        "config_basin" => Ok(super::config_basin(
            n,
            CONFIG_HORIZON,
            r_coll.unwrap_or(super::CONFIG_BASIN_R_COLL),
        )),
        "deep_interior" => Ok(deep_interior(
            n,
            r_coll.unwrap_or(super::DEEP_INTERIOR_R_COLL),
        )),
        other => Err(format!(
            "no slice `{other}` (config_stability, config_basin, deep_interior)"
        )),
    }
}

/// The population of `slice` at `n` and `r_coll`, marched once per process and shared by the inputs that read it.
pub fn marched(slice: &str, n: usize, r_coll: Option<f64>) -> Result<Arc<Marched>, String> {
    type Cache = Mutex<HashMap<String, Arc<Marched>>>;
    static CACHE: OnceLock<Cache> = OnceLock::new();
    let key = format!("{slice}/{n}/{r_coll:?}");
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(m) = cache.lock().map_err(|e| e.to_string())?.get(&key) {
        return Ok(m.clone());
    }
    let pop = population(slice, n, r_coll)?;
    let buffer = march::production::<GroundTruthOccupant>(&pop.ics, &pop.keys);
    let before = march::buffer_bytes(&buffer);
    let runs =
        march::harness::<GroundTruthOccupant>(&buffer, &pop.ics, &pop.keys, &pop.harness_steps)?;
    let m = Arc::new(Marched {
        buffer_untouched: march::buffer_bytes(&buffer) == before,
        pop,
        runs,
    });
    cache
        .lock()
        .map_err(|e| e.to_string())?
        .insert(key, m.clone());
    Ok(m)
}

/// The marches an input scores under its rule.
fn under(rule: Rule, runs: &[PostEscape]) -> Vec<PostEscape> {
    match rule {
        Rule::Detector => runs.to_vec(),
        Rule::EnergyReceding => runs.iter().map(energy_receding).collect(),
    }
}

/// Reads an input: its slice marched and its rule.
fn read(name: &str, fixture: &serde_json::Value) -> Result<(Input, Rule, Arc<Marched>), String> {
    let input = Input::deserialize(fixture).map_err(|e| format!("refused: {name}: {e}"))?;
    let rule = Rule::parse(&input.rule).ok_or(format!(
        "refused: {name}: no rule `{}` (detector, energy_receding)",
        input.rule
    ))?;
    if input.n == 0 {
        return Err(format!("refused: {name}: no pixels"));
    }
    let m = marched(&input.slice, input.n, input.r_coll)
        .map_err(|e| format!("refused: {name}: {e}"))?;
    Ok((input, rule, m))
}

/// The check an input of `escape-checks` is, by its slice: check 2 on the config chart, check 3 on `deep interior`.
fn check_of(slice: &str) -> (&'static str, &'static str) {
    if slice == "deep_interior" {
        ("check 3", "REQ-VAL-124")
    } else {
        ("check 2", "REQ-VAL-123")
    }
}

/// `escape-checks` (the module's note).
pub struct EscapeChecksGate;

impl Gate for EscapeChecksGate {
    fn name(&self) -> &'static str {
        CHECKS_NAME
    }

    fn run(
        &self,
        config: &GateConfig,
        name: &str,
        fixture: &serde_json::Value,
    ) -> Result<GateReport, String> {
        let (input, rule, m) = read(name, fixture)?;
        let (check, requirement) = check_of(&input.slice);
        let runs = under(rule, &m.runs);
        let scores = check2::score(&m.pop, &runs);
        let tally = check2::tally(&scores, false);
        let verdict = check2::judge(&tally, config.threshold.value);
        let pass = verdict.is_ok() && m.buffer_untouched;
        let mut threshold = config.threshold.clone();
        threshold.requirement = requirement.to_owned();
        Ok(GateReport {
            gate: CHECKS_NAME.to_owned(),
            input: name.to_owned(),
            description: input.description,
            verdict: if pass { Verdict::Pass } else { Verdict::Fail },
            threshold,
            scatter: vec![
                format!("{check}, rule {:?}", rule),
                check2::report(m.pop.name, &m.pop, &runs, &scores),
                super::CHECK1_PRIN_RS.to_owned(),
            ],
            regions: Some(Regions {
                count: RegionCount::Recorded((tally.tp + tally.fp) as u32),
                minimum: Minimum::NotCalibrated {
                    requirement: requirement.to_owned(),
                },
            }),
            negative_results: vec![
                match verdict {
                    Ok(line) => format!("{check}: {line}: met"),
                    Err(line) => format!("{check}: {line}: not met"),
                },
                if m.buffer_untouched {
                    "the production buffer's bytes are the same after the harness march as before (REQ-VAL-054)".into()
                } else {
                    "the production buffer's bytes CHANGED across the harness march (REQ-VAL-054)"
                        .into()
                },
            ],
        })
    }

    fn flags(&self, _root: &Path, args: &[&str]) -> Result<(), String> {
        match args {
            ["--propose", "check2"] => {
                println!("{}", propose_check2()?);
                Ok(())
            }
            ["--propose", "check3"] => {
                println!("{}", propose_check3()?);
                Ok(())
            }
            _ => Err(format!(
                "escape-checks: unrecognised flags `{}`; use `--propose check2` or `--propose check3`",
                args.join(" ")
            )),
        }
    }
}

/// The result line of a check on the detector.
fn result(m: &Marched, check: &str) -> String {
    let scores = check2::score(&m.pop, &m.runs);
    let tally = check2::tally(&scores, false);
    let verdict = match check2::judge(&tally, PRECISION_PROPOSED) {
        Ok(line) => format!("PASS: {line}"),
        Err(line) => format!("FAIL: {line}"),
    };
    format!(
        "{check} on the detector as it is: {verdict}\n{}",
        check2::report(m.pop.name, &m.pop, &m.runs, &scores)
    )
}

/// The ground truth's definition, for a proposal.
fn truth_text() -> String {
    format!(
        "The ground truth (separation growing without bound; distances only, no energy): the trajectory marched past \
         escape to T_h = 3·t_max; over its sampling points in [T_h/2, T_h], body k escapes where it is the farthest from \
         the other two's centre of mass at every point, its distance grows strictly between consecutive points, and \
         d_k(T_h) − d_k(3T_h/4) ≥ ρ·(d_k(3T_h/4) − d_k(T_h/2)), ρ = 2^(2/3) − 1 = {:.6} (the slowest unbounded growth, \
         parabolic, from the segment's start). Otherwise bound; a march a collision, a failure or a spent budget ended \
         is censored. Judged: firings by T_h/2, precision ≥ {PRECISION_PROPOSED} over ≥ {} decided firings; recall \
         reported.",
        check2::rho(),
        check2::MIN_DECIDED_FIRINGS
    )
}

/// `--propose check2` (REQ-VAL-123).
pub fn propose_check2() -> Result<String, String> {
    let m = marched("config_stability", CHECK2_N, None)?;
    Ok(format!(
        "REQ-VAL-123, check 2, proposed:\n- {}\n- Horizon: the slice's own, t_max = {CONFIG_HORIZON}; T_h = 3·t_max = \
         {}.\n- Fixture set: config_stability (fixtures/slices.toml; the Config chart pitfalls §2.2 measured on), \
         {CHECK2_N} × {CHECK2_N} pixel centres, its r_coll = 0.005, Heggie × RK4 as the field backend marches it.\n\n{}",
        truth_text(),
        3.0 * CONFIG_HORIZON,
        result(&m, "check 2")
    ))
}

/// `--propose check3` (REQ-VAL-124).
pub fn propose_check3() -> Result<String, String> {
    let m = marched("deep_interior", check3::N, Some(check3::R_COLL))?;
    let own = marched("deep_interior", 16, None)?;
    let censored = own
        .runs
        .iter()
        .filter(|r| check2::truth(r) == Truth::Censored)
        .count();
    Ok(format!(
        "REQ-VAL-124, check 3, proposed:\n- {}\n- The deep-interior region: fixtures/slices.toml [regions.deep_interior], \
         body 0 over the box of half-width 0.05 about (0, 0), the others at Burrau's positions, at rest; t_max = 13 \
         Burrau units, each pixel's key in its own canonical units; T_h = 3·t_max; {} × {} pixel centres; r_coll = {} \
         (no collision terminal; the slice's own 0.001 censors the region: {censored} of {} marches on a 16 × 16 grid).\n\n{}",
        truth_text(),
        check3::N,
        check3::N,
        check3::R_COLL,
        own.runs.len(),
        result(&m, "check 3")
    ))
}

/// `escape-rebind` (the module's note).
pub struct EscapeRebindGate;

impl Gate for EscapeRebindGate {
    fn name(&self) -> &'static str {
        REBIND_NAME
    }

    fn run(
        &self,
        config: &GateConfig,
        name: &str,
        fixture: &serde_json::Value,
    ) -> Result<GateReport, String> {
        let (input, rule, m) = read(name, fixture)?;
        let runs = under(rule, &m.runs);
        let readings = rebind::rebinds(&m.pop, &runs);
        let rebound = readings
            .iter()
            .filter(|r| {
                r.readings
                    .iter()
                    .any(|x| matches!(x, rebind::Reading::Rebound(_)))
            })
            .count();
        let verdict = rebind::judge(&readings);
        let pass = verdict.is_ok() && rebound as f64 <= config.threshold.value;
        Ok(GateReport {
            gate: REBIND_NAME.to_owned(),
            input: name.to_owned(),
            description: input.description,
            verdict: if pass { Verdict::Pass } else { Verdict::Fail },
            threshold: config.threshold.clone(),
            scatter: vec![
                format!("rule {:?}", rule),
                rebind::report(m.pop.name, &readings),
            ],
            regions: Some(Regions {
                count: RegionCount::Recorded(readings.len() as u32),
                minimum: config.min_regions.clone(),
            }),
            negative_results: vec![match verdict {
                Ok(line) => format!("{line}: met"),
                Err(line) => format!("{line}: not met"),
            }],
        })
    }

    fn flags(&self, _root: &Path, args: &[&str]) -> Result<(), String> {
        match args {
            ["--propose", "offsets"] => {
                println!("{}", propose_offsets()?);
                Ok(())
            }
            _ => Err(format!(
                "escape-rebind: unrecognised flags `{}`; use `--propose offsets`",
                args.join(" ")
            )),
        }
    }
}

/// `--propose offsets` (REQ-EVT-032).
pub fn propose_offsets() -> Result<String, String> {
    let centre = deep_interior(1, check3::R_COLL);
    let mut out = format!(
        "REQ-EVT-032, the re-bind test's five offsets past firing, proposed:\n- The grid: prin-rs's measurement of the \
         895 transients, deep interior at t_max = {} and n_sync = {} in Burrau's units, one boundary t_max/n_sync = {} \
         (docs/reference/prin-rs/src/integrate/az/driver.rs, `escape_confirm`'s note; the integrator contract Part 2a's \
         \"production grid (t_max / n_sync = 13/32)\").\n- The offsets, +{:?} boundaries: {:?} Burrau units; at the \
         deep interior's centre pixel, {:?} canonical (×{:.6}); each pixel converts at its own scale and reads at the \
         first sync boundary (Δ_sync = 1/8) at or after t_fire + offset.\n",
        rebind::PRIN_RS_T_MAX,
        rebind::PRIN_RS_N_SYNC,
        rebind::PRIN_RS_T_MAX / rebind::PRIN_RS_N_SYNC,
        rebind::BOUNDARIES,
        rebind::offsets_burrau(),
        rebind::offsets(centre.scale[0]),
        centre.scale[0]
    );
    for (slice, n, r_coll) in [
        ("deep_interior", check3::N, Some(check3::R_COLL)),
        ("config_stability", CHECK2_N, None),
    ] {
        let m = marched(slice, n, r_coll)?;
        let readings = rebind::rebinds(&m.pop, &m.runs);
        let verdict = match rebind::judge(&readings) {
            Ok(line) => format!("PASS: {line}"),
            Err(line) => format!("FAIL: {line}"),
        };
        out += &format!("\n{verdict}\n{}", rebind::report(m.pop.name, &readings));
    }
    Ok(out)
}
