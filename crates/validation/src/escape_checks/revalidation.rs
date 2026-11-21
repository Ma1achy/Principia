//! `cargo xtask gate escape-revalidation` and `cargo xtask gate escape-precision` (TASK-M3-34): the escape rule's
//! precision and recall re-measured with R-29's `E_rel`, against check 2's independent ground truth and against the
//! legacy `t = 30` set kept as a comparison (pitfalls §2.2).
//!
//! **The legacy `t = 30` set** ([`legacy_truth`]): pitfalls §2.2's table's ground truth, "unbound and receding at
//! `t = 30`", on the config chart, transcribed from prin-rs's classifier (`docs/reference/prin-rs/src/outcome.rs`,
//! `escapes` with no distance gate): body `b` is unbound and receding where `½|Δv|² − M_pair/|Δr| > 0` (prin-rs's
//! `rel_two_body`, the pair's mass only, its distance floored at `10⁻¹²`) and `Δr·Δv > 0`, `Δr` and `Δv` relative to
//! the other two's centre of mass. It reads the state at the first sampling point at or after `t = 30` of the same
//! harness march ([`super::march::SNAPSHOT_T`]); a march a collision ended before it is censored. The harness
//! regenerates it on the two config slices (`config_stability` at its own `r_coll`, `config_basin` at `r_coll = 0`,
//! [`legacy_slices`]) and checks it in as `fixtures/escape/legacy_t30.json` ([`LEGACY_FIXTURE`]); the gate scores against
//! the checked-in set and reports how many labels a fresh regeneration differs in.
//!
//! **Scoring against it**: a firing by `t = 30` is a true positive where its escaper is one of the set's (any, for a
//! triple ejection), a false positive otherwise; an unbound-and-receding pixel not fired on by then is a false negative.
//!
//! **`escape-revalidation`** ([`EscapeRevalidationGate`]): an input names a slice or two and a truth (`check2` or
//! `legacy`) and a rule; it passes where the precision is at least the gate's threshold over at least
//! [`check2::MIN_DECIDED_FIRINGS`] decided firings and, for the detector, the replay at the register's rule is the
//! detector's own firing on every trajectory ([`replay::reproduces`]). Recall is recorded, not judged: late rather
//! than wrong.
//!
//! **`escape-precision`** ([`EscapePrecisionGate`]): on the re-validation set, the firings whose ground truth is bound
//! (or another body's escape) are counted; it passes where that count is at most the gate's threshold, `0`, the
//! measured precision (100.0%, as pre-R-29). Its controls are criteria that fire earlier: a larger `tau`, and the
//! pre-fix energy-and-receding rule. Each has false positives, so each fails.

use std::path::Path;
use std::sync::{Arc, OnceLock};

use serde::{Deserialize, Serialize};

use kernel::generated::links::Defaults;
use kernel::payload::DETAIL_ESCAPE_ALL_THREE_TRIPLE_EJECTION;

use crate::gate::{Gate, GateConfig, GateReport, RegionCount, Regions, Verdict};

use super::check2::{self, Tally};
use super::gate::{energy_receding, Marched};
use super::march::{self, PostEscape, Snapshot};
use super::replay::{self, Rule};
use super::tau_gap::{self, Slice};
use super::GroundTruthOccupant;

/// `escape-revalidation`'s name.
pub const REVALIDATION_NAME: &str = "escape-revalidation";
/// `escape-precision`'s name.
pub const PRECISION_NAME: &str = "escape-precision";

/// The checked-in legacy set, relative to the workspace root.
pub const LEGACY_FIXTURE: &str = "fixtures/escape/legacy_t30.json";

/// The legacy set's pixels per side.
pub const LEGACY_N: usize = 32;

/// The slices the legacy set is regenerated on, with their collision radii: `config_stability` at its own, and
/// `config_basin` at 0. The plan is unclear here: at its own `r_coll = 0.02` every `config_basin` march ends in a
/// collision by `t = 0.25` (measured: `d_min = 0.012` at the first close approach), which censors the whole slice; chose
/// `r_coll = 0`, no collision terminal, as check 3 chose for `deep interior`, because a censored slice scores nothing.
pub fn legacy_slices() -> [Slice; 2] {
    [
        Slice {
            slice: "config_stability".into(),
            n: LEGACY_N,
            r_coll: None,
        },
        Slice {
            slice: "config_basin".into(),
            n: LEGACY_N,
            r_coll: Some(0.0),
        },
    ]
}

/// The re-validation set, check 2's populations: `config_stability` at its own `r_coll`, `config_basin` at 0, and
/// `deep interior` at check 3's.
pub fn revalidation_slices() -> [Slice; 3] {
    let [a, b] = legacy_slices();
    [
        a,
        b,
        Slice {
            slice: "deep_interior".into(),
            n: super::check3::N,
            r_coll: Some(super::check3::R_COLL),
        },
    ]
}

/// The legacy ground truth of one pixel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Legacy {
    /// The bodies unbound and receding at `t = 30`, a bit each; at least one.
    Escapes(u8),
    /// None is.
    Bound,
    /// The march ended before `t = 30`.
    Censored,
}

impl Legacy {
    /// Its word in the checked-in set: `B`, `C`, or `E` and the bodies.
    pub fn word(&self) -> String {
        match self {
            Legacy::Bound => "B".into(),
            Legacy::Censored => "C".into(),
            Legacy::Escapes(mask) => {
                let mut s = "E".to_owned();
                for b in 0..3 {
                    if mask & (1 << b) != 0 {
                        s += &b.to_string();
                    }
                }
                s
            }
        }
    }

    /// The truth a word names.
    pub fn parse(word: &str) -> Result<Self, String> {
        match word {
            "B" => Ok(Legacy::Bound),
            "C" => Ok(Legacy::Censored),
            w if w.starts_with('E') && w.len() > 1 => {
                let mut mask = 0u8;
                for c in w[1..].chars() {
                    let b = c.to_digit(10).filter(|&b| b < 3).ok_or(format!("`{w}`"))?;
                    mask |= 1 << b;
                }
                Ok(Legacy::Escapes(mask))
            }
            w => Err(format!("no legacy label `{w}`")),
        }
    }
}

/// prin-rs's `escapes(s, m, b, 0)` (the module's note): body `b` unbound from the other two by the pair's mass alone,
/// and receding from their centre of mass.
pub fn legacy_escapes(m: &[f64; 3], s: &Snapshot, b: usize) -> bool {
    let (i, j) = ((b + 1) % 3, (b + 2) % 3);
    let mp = m[i] + m[j];
    let v = |k: usize| [s.p[k][0] / m[k], s.p[k][1] / m[k]];
    let (vi, vj, vb) = (v(i), v(j), v(b));
    let dr: [f64; 2] =
        std::array::from_fn(|x| s.r[b][x] - (m[i] * s.r[i][x] + m[j] * s.r[j][x]) / mp);
    let dv: [f64; 2] = std::array::from_fn(|x| vb[x] - (m[i] * vi[x] + m[j] * vj[x]) / mp);
    let spec = 0.5 * (dv[0] * dv[0] + dv[1] * dv[1]) - mp / dr[0].hypot(dr[1]).max(1e-12);
    spec > 0.0 && dr[0] * dv[0] + dr[1] * dv[1] > 0.0
}

/// The legacy truth of `run` from masses `m` (the module's note).
pub fn legacy_truth(m: &[f64; 3], run: &PostEscape) -> Legacy {
    match run.snapshot {
        None => Legacy::Censored,
        Some(s) => {
            let mask = (0..3)
                .filter(|&b| legacy_escapes(m, &s, b))
                .fold(0u8, |a, b| a | 1 << b);
            if mask == 0 {
                Legacy::Bound
            } else {
                Legacy::Escapes(mask)
            }
        }
    }
}

/// The confusion of `runs` against the legacy labels `truth`: firings by the snapshot's time, `t = 30`.
pub fn legacy_tally(runs: &[PostEscape], truth: &[Legacy]) -> Tally {
    let mut t = Tally {
        n: runs.len(),
        ..Tally::default()
    };
    for (r, l) in runs.iter().zip(truth) {
        let fired = r.firing.filter(|f| f.t <= march::SNAPSHOT_T);
        match (fired, l) {
            (Some(f), Legacy::Escapes(mask))
                if f.detail == DETAIL_ESCAPE_ALL_THREE_TRIPLE_EJECTION
                    || mask & (1 << f.escaper) != 0 =>
            {
                t.tp += 1
            }
            (Some(_), Legacy::Censored) => t.fired_censored += 1,
            (Some(_), _) => t.fp += 1,
            (None, Legacy::Escapes(_)) => {
                t.fn_ += 1;
                t.fn_late += usize::from(r.firing.is_some());
            }
            (None, Legacy::Bound) => t.tn += 1,
            (None, Legacy::Censored) => t.censored += 1,
        }
    }
    t
}

/// The checked-in legacy set.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LegacySet {
    /// What it is.
    pub description: String,
    /// The time it is read at, canonical.
    pub t: f64,
    /// Each slice's labels.
    pub slices: Vec<LegacySlice>,
}

/// One slice of the legacy set.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LegacySlice {
    /// The slice.
    pub slice: String,
    /// Pixels per side.
    pub n: usize,
    /// Its collision radius; null for its own.
    pub r_coll: Option<f64>,
    /// Each pixel's label ([`Legacy::word`]), row by row from the bottom left.
    pub labels: Vec<String>,
}

/// The legacy set regenerated by the harness on [`legacy_slices`].
pub fn regenerate() -> Result<LegacySet, String> {
    let mut slices = Vec::new();
    for s in legacy_slices() {
        let m = s.marched()?;
        slices.push(LegacySlice {
            slice: s.slice.clone(),
            n: s.n,
            r_coll: s.r_coll,
            labels: m
                .runs
                .iter()
                .zip(&m.pop.ics)
                .map(|(r, ic)| legacy_truth(&ic.m, r).word())
                .collect(),
        });
    }
    Ok(LegacySet {
        description: "The legacy t = 30 ground truth (pitfalls §2.2's table): per pixel of the config slices, the bodies \
                      unbound by the pair's mass alone and receding at the first sampling point at or after t = 30 \
                      (prin-rs outcome.rs `escapes`, no distance gate), B for none, C where the march ended before; \
                      regenerated by `cargo xtask gate escape-revalidation --write-legacy` under Heggie × RK4."
            .into(),
        t: march::SNAPSHOT_T,
        slices,
    })
}

/// The checked-in legacy set under the workspace `root`.
pub fn read_legacy(root: &Path) -> Result<LegacySet, String> {
    let path = root.join(LEGACY_FIXTURE);
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// The checked-in labels of `slice`, parsed, with how many a fresh regeneration on `m` differs in.
fn checked_in(set: &LegacySet, slice: &Slice, m: &Marched) -> Result<(Vec<Legacy>, usize), String> {
    let s = set
        .slices
        .iter()
        .find(|s| s.slice == slice.slice && s.n == slice.n && s.r_coll == slice.r_coll)
        .ok_or(format!(
            "the legacy set has no `{}` at n = {}, r_coll {:?}",
            slice.slice, slice.n, slice.r_coll
        ))?;
    if s.labels.len() != m.runs.len() {
        return Err(format!(
            "the legacy set's `{}` has {} labels for {} pixels",
            s.slice,
            s.labels.len(),
            m.runs.len()
        ));
    }
    let labels = s
        .labels
        .iter()
        .map(|w| Legacy::parse(w))
        .collect::<Result<Vec<_>, _>>()?;
    let differ = labels
        .iter()
        .zip(m.runs.iter().zip(&m.pop.ics))
        .filter(|(l, (r, ic))| **l != legacy_truth(&ic.m, r))
        .count();
    Ok((labels, differ))
}

/// Which firings an input scores.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Scored {
    /// The detector as it is.
    Detector,
    /// The rule replayed at a `tau` and window.
    Replay(Rule),
    /// The pre-fix rule, `E_rel > 0` and receding.
    EnergyReceding,
}

/// An input of either gate.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    description: String,
    populations: Vec<Slice>,
    /// `check2` or `legacy`; `escape-precision` reads `check2` only.
    #[serde(default)]
    truth: Option<String>,
    /// `detector`, `replay` or `energy_receding`.
    rule: String,
    /// For `replay`: the `tau`; null for the register's.
    #[serde(default)]
    tau: Option<f64>,
    /// For `replay`: the window; null for the register's.
    #[serde(default)]
    window: Option<f64>,
}

impl Input {
    fn scored(&self) -> Result<Scored, String> {
        match self.rule.as_str() {
            "detector" => Ok(Scored::Detector),
            "energy_receding" => Ok(Scored::EnergyReceding),
            "replay" => Ok(Scored::Replay(Rule {
                tau: self.tau.unwrap_or(Rule::register().tau),
                window: self.window.unwrap_or(Rule::register().window),
            })),
            other => Err(format!(
                "no rule `{other}` (detector, replay, energy_receding)"
            )),
        }
    }
}

/// `m`'s marches under `scored`.
pub fn firings(m: &Marched, scored: Scored) -> Vec<PostEscape> {
    match scored {
        Scored::Detector => m.runs.clone(),
        Scored::Replay(rule) => replay::all_under(&m.runs, &m.pop.ics, rule),
        Scored::EnergyReceding => m.runs.iter().map(energy_receding).collect(),
    }
}

/// How many of `m`'s trajectories the replay at the register's rule does not reproduce, with the first.
pub fn unreproduced(m: &Marched) -> (usize, Option<String>) {
    let bad: Vec<String> = m
        .runs
        .iter()
        .zip(&m.pop.ics)
        .enumerate()
        .filter_map(|(i, (r, ic))| {
            replay::reproduces(r, ic)
                .err()
                .map(|e| format!("pixel {i}: {e}"))
        })
        .collect();
    (bad.len(), bad.into_iter().next())
}

/// A tally's line.
pub fn tally_line(t: &Tally) -> String {
    let fmt = |x: Option<f64>| x.map_or("none".into(), |x| format!("{x:.4}"));
    format!(
        "TP {} FP {} FN {} (of which fired later: {}) TN {}; censored {} fired, {} not; precision {}, recall {}",
        t.tp,
        t.fp,
        t.fn_,
        t.fn_late,
        t.tn,
        t.fired_censored,
        t.censored,
        fmt(t.precision()),
        fmt(t.recall())
    )
}

/// Sums tallies.
fn sum(ts: &[Tally]) -> Tally {
    ts.iter().fold(Tally::default(), |a, t| Tally {
        n: a.n + t.n,
        tp: a.tp + t.tp,
        fp: a.fp + t.fp,
        fired_censored: a.fired_censored + t.fired_censored,
        fn_: a.fn_ + t.fn_,
        fn_late: a.fn_late + t.fn_late,
        tn: a.tn + t.tn,
        censored: a.censored + t.censored,
    })
}

/// An input's populations marched, its truth and rule.
fn read(
    name: &str,
    fixture: &serde_json::Value,
) -> Result<(Input, Scored, Vec<Arc<Marched>>), String> {
    let input = Input::deserialize(fixture).map_err(|e| format!("refused: {name}: {e}"))?;
    let scored = input
        .scored()
        .map_err(|e| format!("refused: {name}: {e}"))?;
    if input.populations.is_empty() {
        return Err(format!("refused: {name}: no population"));
    }
    let pops = input
        .populations
        .iter()
        .map(Slice::marched)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("refused: {name}: {e}"))?;
    Ok((input, scored, pops))
}

/// `escape-revalidation` (the module's note).
pub struct EscapeRevalidationGate;

/// The workspace root the gate binary was given, for the checked-in legacy set; set by [`Gate::flags`] or found from
/// the crate's own location.
fn workspace_root() -> std::path::PathBuf {
    crate::import::root()
}

impl Gate for EscapeRevalidationGate {
    fn name(&self) -> &'static str {
        REVALIDATION_NAME
    }

    fn run(
        &self,
        config: &GateConfig,
        name: &str,
        fixture: &serde_json::Value,
    ) -> Result<GateReport, String> {
        let (input, scored, pops) = read(name, fixture)?;
        let legacy = match input.truth.as_deref() {
            Some("check2") | None => false,
            Some("legacy") => true,
            Some(other) => {
                return Err(format!(
                    "refused: {name}: no truth `{other}` (check2, legacy)"
                ))
            }
        };
        let set = if legacy {
            Some(read_legacy(&workspace_root()).map_err(|e| format!("refused: {name}: {e}"))?)
        } else {
            None
        };
        let mut scatter = vec![format!(
            "rule {scored:?}, truth {}",
            if legacy { "legacy t = 30" } else { "check 2" }
        )];
        let mut negative = Vec::new();
        let mut tallies = Vec::new();
        let mut reproduced = true;
        for (slice, m) in input.populations.iter().zip(&pops) {
            let runs = firings(m, scored);
            let t = if let Some(set) = &set {
                let (labels, differ) =
                    checked_in(set, slice, m).map_err(|e| format!("refused: {name}: {e}"))?;
                scatter.push(format!(
                    "{}: against the checked-in legacy set ({differ} of {} labels differ in a fresh regeneration, \
                     reported): {}",
                    m.pop.name,
                    labels.len(),
                    tally_line(&legacy_tally(&runs, &labels))
                ));
                legacy_tally(&runs, &labels)
            } else {
                let scores = check2::score(&m.pop, &runs);
                scatter.push(check2::report(m.pop.name, &m.pop, &runs, &scores));
                check2::tally(&scores, false)
            };
            if scored == Scored::Detector {
                let (bad, first) = unreproduced(m);
                reproduced &= bad == 0;
                negative.push(format!(
                    "{}: the replay at the register's rule is the detector's own firing on {} of {} trajectories{}",
                    m.pop.name,
                    m.runs.len() - bad,
                    m.runs.len(),
                    first.map_or(String::new(), |f| format!(" (first: {f})"))
                ));
            }
            tallies.push(t);
        }
        let all = sum(&tallies);
        let verdict = check2::judge(&all, config.threshold.value);
        scatter.push(format!("pooled: {}", tally_line(&all)));
        negative.push(match &verdict {
            Ok(line) => format!("{line}: met"),
            Err(line) => format!("{line}: not met"),
        });
        let pass = verdict.is_ok() && reproduced;
        Ok(GateReport {
            gate: REVALIDATION_NAME.to_owned(),
            input: name.to_owned(),
            description: input.description,
            verdict: if pass { Verdict::Pass } else { Verdict::Fail },
            threshold: config.threshold.clone(),
            scatter,
            regions: Some(Regions {
                count: RegionCount::Recorded((all.tp + all.fp) as u32),
                minimum: config.min_regions.clone(),
            }),
            negative_results: negative,
        })
    }

    fn flags(&self, root: &Path, args: &[&str]) -> Result<(), String> {
        match args {
            ["--write-legacy"] => {
                let set = regenerate()?;
                let path = root.join(LEGACY_FIXTURE);
                let text = serde_json::to_string_pretty(&set).map_err(|e| e.to_string())?;
                std::fs::write(&path, text + "\n")
                    .map_err(|e| format!("{}: {e}", path.display()))?;
                println!("wrote {}", path.display());
                Ok(())
            }
            _ => Err(format!(
                "escape-revalidation: unrecognised flags `{}`; use `--write-legacy`",
                args.join(" ")
            )),
        }
    }
}

/// `escape-precision` (the module's note).
pub struct EscapePrecisionGate;

impl Gate for EscapePrecisionGate {
    fn name(&self) -> &'static str {
        PRECISION_NAME
    }

    fn run(
        &self,
        config: &GateConfig,
        name: &str,
        fixture: &serde_json::Value,
    ) -> Result<GateReport, String> {
        let (input, scored, pops) = read(name, fixture)?;
        if input.truth.as_deref().is_some_and(|t| t != "check2") {
            return Err(format!(
                "refused: {name}: escape-precision reads check 2's truth only"
            ));
        }
        let mut scatter = vec![format!("rule {scored:?}, truth check 2")];
        let mut tallies = Vec::new();
        for m in &pops {
            let runs = firings(m, scored);
            let t = check2::tally(&check2::score(&m.pop, &runs), false);
            let fired: Vec<f64> = runs.iter().filter_map(|r| r.firing.map(|f| f.t)).collect();
            scatter.push(format!(
                "{}: {}; firing times {}",
                m.pop.name,
                tally_line(&t),
                super::spread(&fired)
            ));
            tallies.push(t);
        }
        let all = sum(&tallies);
        let pass = all.fp as f64 <= config.threshold.value && all.tp > 0;
        Ok(GateReport {
            gate: PRECISION_NAME.to_owned(),
            input: name.to_owned(),
            description: input.description,
            verdict: if pass { Verdict::Pass } else { Verdict::Fail },
            threshold: config.threshold.clone(),
            scatter,
            regions: Some(Regions {
                count: RegionCount::Recorded((all.tp + all.fp) as u32),
                minimum: config.min_regions.clone(),
            }),
            negative_results: vec![format!(
                "firings whose check-2 truth is bound or another body's escape: {} (criterion ≤ {}), over {} \
                 decided firings (precision {}): {}",
                all.fp,
                config.threshold.value,
                all.tp + all.fp,
                all.precision().map_or("none".into(), |p| format!("{p:.4}")),
                if pass { "met" } else { "not met" }
            )],
        })
    }
}

/// The Burrau anchor's start and key as the label tests march it (`ground_truth::tiers::burrau`, the test's key at
/// [`crate::ground_truth::BENCHMARK_R_COLL`]), marched by the harness to 170 of Szebehely & Peters' units, with the time
/// scale; once per process.
pub fn burrau() -> &'static (kernel::decode::Ic<f64>, PostEscape, f64) {
    static ONCE: OnceLock<(kernel::decode::Ic<f64>, PostEscape, f64)> = OnceLock::new();
    ONCE.get_or_init(|| {
        let start = crate::ground_truth::tiers::burrau();
        let x = crate::import::unit_state(&start).expect("the Burrau start");
        let scale = crate::ground_truth::burrau_time_scale();
        let key = crate::ground_truth::march_key(
            crate::ground_truth::tiers::BURRAU_HORIZON * scale,
            crate::ground_truth::BENCHMARK_R_COLL,
        )
        .expect("the Burrau key");
        let ic = crate::ground_truth::path_a::check1::<Defaults>(&x).decoded;
        let steps = (BURRAU_MARCH * scale / key.loop_values.dt_macro).round() as u32;
        let run = march::march_past::<GroundTruthOccupant>(&ic, &key, steps);
        (ic, run, scale)
    })
}

/// How far the Burrau harness march runs, Szebehely & Peters' units: past the provisional `tau`'s firing near 158.
pub const BURRAU_MARCH: f64 = 170.0;

/// The Burrau escape label's firing under `rule`: its time in Szebehely & Peters' units and its escaper.
pub fn burrau_firing(rule: Rule) -> Option<(f64, u32)> {
    let (ic, run, scale) = burrau();
    replay::replay(run, ic, rule).map(|f| (f.t / scale, f.escaper))
}

/// `tau-gap --propose` (REQ-EVT-025): the confirmed `tau` the gap sets, with the gap, the outcomes across its middle,
/// precision and recall at it on both ground truths, and the Burrau anchor.
pub fn propose_tau() -> Result<String, String> {
    let slices = revalidation_slices();
    let pops = slices
        .iter()
        .map(Slice::marched)
        .collect::<Result<Vec<_>, _>>()?;
    let window = Rule::register().window;
    let (gap, each) = tau_gap::measure(&pops, window);
    let mut out =
        format!("REQ-EVT-025, the confirmed tau, proposed from the gap (window {window}):\n");
    for (_, r) in &each {
        out += &format!("{r}\n");
    }
    let chosen = gap.chosen();
    let [lo, hi] = gap.middle();
    out += &format!(
        "\nThe gap over the populations: L {:.4e}, U {:.4e}, U/L {:.3}; the middle half [{lo:.4e}, {hi:.4e}]; \
         √(L·U) = {:.4e}, to one figure {chosen:e} ({}).\n",
        gap.l,
        gap.u,
        gap.ratio(),
        (gap.l * gap.u).sqrt(),
        if gap.exists() && gap.in_middle(chosen) {
            "inside the middle half"
        } else {
            "NOT inside the middle half"
        }
    );
    for m in &pops {
        let x = tau_gap::across_middle(m, lo, hi, window);
        out += &format!(
            "- {} across the middle half: {} labels differ, {} of them escapes gained; false positives {} and {}\n",
            m.pop.name, x.changed, x.gained, x.fp_low, x.fp_high
        );
    }
    out += "\nPrecision and recall by tau (check 2, judged firings by T_h/2; legacy t = 30 on the config slices):\n";
    let set = read_legacy(&workspace_root()).ok();
    let mut taus = vec![
        1e-3,
        2e-3,
        3e-3,
        5e-3,
        chosen,
        1e-2,
        gap.u,
        2e-2,
        5e-2,
        1e-1,
        f64::INFINITY,
    ];
    taus.sort_by(f64::total_cmp);
    taus.dedup();
    for tau in taus {
        let rule = Rule { tau, window };
        let mut line = format!("- tau {tau:.4e}:");
        for (slice, m) in slices.iter().zip(&pops) {
            let runs = firings(m, Scored::Replay(rule));
            let t = check2::tally(&check2::score(&m.pop, &runs), false);
            line += &format!(" {} TP {} FP {} FN {}", m.pop.name, t.tp, t.fp, t.fn_);
            if let Some((labels, _)) = set.as_ref().and_then(|s| checked_in(s, slice, m).ok()) {
                let l = legacy_tally(&runs, &labels);
                line += &format!(" (legacy TP {} FP {} FN {})", l.tp, l.fp, l.fn_);
            }
            line += ";";
        }
        line += &format!(
            " Burrau fires at {:?} (S&P units, escaper)",
            burrau_firing(rule)
        );
        out += &format!("{line}\n");
    }
    Ok(out)
}
